//! `reply-language-stop`: the Stop hook that sends a final reply of the main conversation
//! in English back once to be rewritten in Japanese.
//!
//! Replies in the middle of a turn may drift into English, and the final reply has too,
//! after the turn filled with English tool output. It is a command hook rather than a
//! `classic.Stop` hook in the `global-hooks` plugin's module, since a managed plugin may
//! continue that event past the user tier, where the module's hook never runs.
//!
//! Each stop is a process of its own, so what one stop leaves for the next is kept in a
//! file per session.

use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::LazyLock;

use clap::Args;
use regex::{Captures, Regex};
use serde_json::{Value, json};

use crate::failure;
use dotfiles_shared::Failure;

/// What Claude is asked for when its reply is sent back.
const REWRITE: &str = "直前の返答は英語で書かれている。同じ内容を日本語で書き直して、もう一度返答する。ユーザーが英語で求めた文やコードはそのまま残し、その周りの説明を日本語で書く。";

/// Code is English by rule, so it says nothing about the reply's own language: a fence
/// of backticks or tildes, inline code, or a URL.
///
/// A fence opens and closes at a line start, so a backtick run named in prose is not
/// one. A line starts after `\r`, U+2028 and U+2029 as well as `\n`, which `(?m)` does
/// not know, so an alternative takes the terminator in. A fence closes with the
/// characters it opened with, an alternative for each, since the crate has no
/// back-references. A URL ends at what JavaScript's `\s` takes for a space, which
/// differs from Unicode's `White_Space` only in U+0085 and U+FEFF.
static CODE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?mx)
        (?:^|[\r\x{2028}\x{2029}]) [\ \t]* ``` [\s\S]*? (?:^|[\r\x{2028}\x{2029}]) [\ \t]* ```
        | (?:^|[\r\x{2028}\x{2029}]) [\ \t]* ~~~ [\s\S]*? (?:^|[\r\x{2028}\x{2029}]) [\ \t]* ~~~
        | `[^`\n]*`
        | https?://[\S\x{85}--\x{FEFF}]+
        ",
    )
    .expect("the pattern compiles")
});

/// A quotation is in the language of what it quotes, so it says nothing about the
/// reply's own.
static QUOTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"「[^」\n]*」|『[^』\n]*』|"[^"\n]*"|“[^”\n]*”"#).expect("the pattern compiles")
});

/// Japanese grammar, its particles and endings, is written in hiragana, and a Japanese
/// sentence is punctuated with 。 and 、; a term or a file name an English reply names is
/// kanji or katakana, so it carries neither.
static JAPANESE_SENTENCE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\p{Script=Hiragana}。、]").expect("the pattern compiles"));

/// An escape in JSON text: a surrogate pair, a lone surrogate (captured), or any other
/// escape, so that an escaped backslash is never taken for the start of one.
static LONE_SURROGATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\\u(?i:d[89ab][0-9a-f]{2})\\u(?i:d[c-f][0-9a-f]{2})|(\\u(?i:d[89a-f][0-9a-f]{2}))|\\[\s\S]")
        .expect("the pattern compiles")
});

/// A reply of fewer Latin letters than this, about two English sentences, is accepted in
/// any language.
const MIN_LATIN: usize = 200;

/// A stop, as the hook's input on stdin gives it.
struct Stop {
    session: String,
    /// Whether a Stop hook blocked the stop before this one, this hook's or another's.
    continued: bool,
    /// The reply the stop ends in, kept as the input has it until it is judged, where
    /// one that is no string fails the hook.
    last_message: Option<Value>,
    /// Whether a subagent stops rather than the main conversation, which a Stop hook is
    /// only guarded against: a subagent stops through `SubagentStop`.
    of_subagent: bool,
}

/// What the hook decides about a stop, and leaves for the next one.
struct Verdict {
    block: bool,
    /// Whether this stop ended in this hook's own request for a rewrite.
    rewrite_asked: bool,
}

impl Verdict {
    /// Prints the verdict as Claude Code reads it: a block decision with the request for
    /// a rewrite, or nothing.
    fn report(&self) -> io::Result<()> {
        if !self.block {
            return Ok(());
        }
        let output = json!({ "decision": "block", "reason": REWRITE });
        // Flushed here, since stdout flushes what it holds at exit without reporting a
        // failure.
        let mut stdout = io::stdout().lock();
        write!(stdout, "{output}")?;
        stdout.flush()
    }
}

impl Stop {
    /// The stop on stdin.
    fn read() -> Result<Self, Failure> {
        let mut raw = Vec::new();
        io::stdin()
            .read_to_end(&mut raw)
            .map_err(|error| Failure::new(format_args!("cannot read stdin: {error}"), 1))?;
        let input: Value = serde_json::from_str(&json_text(&raw))
            .map_err(|error| Failure::new(format_args!("stdin is no JSON: {error}"), 1))?;
        let Some(session) = input.get("session_id").and_then(Value::as_str) else {
            return Err(Failure::new(format_args!("the input has no session_id"), 1));
        };
        Ok(Stop {
            session: session.to_owned(),
            continued: input.get("stop_hook_active").is_some_and(truthy),
            last_message: input.get("last_assistant_message").cloned(),
            of_subagent: input.get("agent_id").is_some(),
        })
    }

    /// Judges the stop, given whether the previous stop of the session ended in this
    /// hook's own request for a rewrite. The reply written after that request is let
    /// through, so the rewrite is asked for once.
    fn judge(&self, rewrite_asked: bool) -> Result<Verdict, Failure> {
        if self.of_subagent {
            return Ok(Verdict {
                block: false,
                rewrite_asked,
            });
        }
        if rewrite_asked && self.continued {
            return Ok(Verdict {
                block: false,
                rewrite_asked: false,
            });
        }
        let block = match &self.last_message {
            None => false,
            Some(Value::String(message)) => is_english(message),
            Some(other) => {
                return Err(Failure::new(
                    format_args!("last_assistant_message is no string: {other}"),
                    1,
                ));
            }
        };
        Ok(Verdict {
            block,
            rewrite_asked: block,
        })
    }
}

/// The JSON text in `raw`, whatever text a string of Claude Code's holds: bytes that are
/// no UTF-8 are read with each invalid sequence replaced, and a lone UTF-16 surrogate
/// escape, which `serde_json` refuses, stands for U+FFFD, which the judgment and the
/// state file's name take as they would the surrogate.
fn json_text(raw: &[u8]) -> String {
    let text = String::from_utf8_lossy(raw);
    LONE_SURROGATE
        .replace_all(&text, |escape: &Captures<'_>| {
            if escape.get(1).is_some() {
                r"\ufffd".to_owned()
            } else {
                escape[0].to_owned()
            }
        })
        .into_owned()
}

/// Whether JavaScript takes `value` for true, which is how `stop_hook_active` is read.
fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(number) => number.as_f64().is_some_and(|number| number != 0.0),
        Value::String(text) => !text.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// Whether `message` is English: long enough to judge, with no Japanese sentence outside
/// code and quotations.
fn is_english(message: &str) -> bool {
    let prose = CODE.replace_all(message, " ");
    let prose = QUOTE.replace_all(&prose, " ");
    prose.chars().filter(char::is_ascii_alphabetic).count() >= MIN_LATIN
        && !JAPANESE_SENTENCE.is_match(&prose)
}

/// The file in which the hook keeps, for a session, whether its last stop ended in its
/// own request for a rewrite. Each stop is a process of its own.
struct SessionFile {
    path: PathBuf,
}

impl SessionFile {
    /// The file of `session`, named by replacing each UTF-16 code unit of the id other
    /// than an ASCII letter, digit, `_` or `-` with `_`.
    fn of(session: &str) -> Self {
        let name: String = session
            .chars()
            .flat_map(|c| {
                let kept = c.is_ascii_alphanumeric() || c == '_' || c == '-';
                let units = if kept { 1 } else { c.len_utf16() };
                std::iter::repeat_n(if kept { c } else { '_' }, units)
            })
            .collect();
        SessionFile {
            path: temp_dir().join("claude-reply-language").join(name),
        }
    }

    fn rewrite_asked(&self) -> bool {
        fs::read(&self.path).is_ok_and(|text| text == b"1")
    }

    fn record(&self, rewrite_asked: bool) -> io::Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(&self.path, if rewrite_asked { "1" } else { "0" })
    }
}

/// The temporary directory: the first of TMPDIR, TMP and TEMP that is set and not empty,
/// or /tmp. `std::env::temp_dir` reads TMPDIR alone, and takes an empty one for the
/// working directory.
fn temp_dir() -> PathBuf {
    ["TMPDIR", "TMP", "TEMP"]
        .into_iter()
        .filter_map(std::env::var_os)
        .find(|dir| !dir.is_empty())
        .map_or_else(|| PathBuf::from("/tmp"), PathBuf::from)
}

/// Send a final reply in English back once to be rewritten in Japanese, as a Stop hook
/// reading the stop on stdin.
#[derive(Debug, Args)]
pub(crate) struct Request {}

impl Request {
    /// Judges the stop on stdin. A failure of the hook's own ends with exit 1 and no
    /// verdict, which Claude Code shows as a hook error and lets the stop go ahead: exit
    /// 2 would keep the session from stopping at all.
    ///
    /// It records what the stop leaves for the next before printing the verdict, so that
    /// a state that cannot be written fails open rather than asking for the rewrite again
    /// and again.
    #[expect(
        clippy::unused_self,
        reason = "the command takes no arguments, and every command's entry has one shape"
    )]
    pub(crate) fn run(self) -> Result<(), Failure> {
        let stop = Stop::read()?;
        let file = SessionFile::of(&stop.session);
        let verdict = stop.judge(file.rewrite_asked())?;
        file.record(verdict.rewrite_asked).map_err(|error| {
            Failure::new(
                format_args!("cannot write {}: {error}", file.path.display()),
                1,
            )
        })?;
        verdict.report().map_err(|error| failure::writing(&error))
    }
}
