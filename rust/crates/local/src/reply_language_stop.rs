//! `reply-language-stop`: the Stop hook that sends a final reply of the main conversation
//! in English back once to be rewritten in Japanese.
//!
//! Replies in the middle of a turn may drift into English, and the final reply has too,
//! after the turn filled with English tool output. The `global-hooks` plugin registers
//! this as a command hook in `hooks.json` rather than a module's `classic.Stop` hook,
//! since a managed plugin may continue that event past the user tier, where the module's
//! hook never runs.
//!
//! Each stop is a process of its own, so what one stop leaves for the next is kept in a
//! file per session.

use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Value, json};

use crate::failure::Failure;

/// What Claude is asked for when its reply is sent back.
const REWRITE: &str = "直前の返答は英語で書かれている。同じ内容を日本語で書き直して、もう一度返答する。ユーザーが英語で求めた文やコードはそのまま残し、その周りの説明を日本語で書く。";

/// Code is English by rule, so it says nothing about the reply's own language: a fence
/// of backticks or tildes, inline code, or a URL.
///
/// The judgment keeps the TypeScript hook's, whose JavaScript pattern this restates. A
/// fence opens and closes at a line start, so a backtick run named in prose is not one,
/// and a line starts after `\r`, U+2028 and U+2029 as well as `\n`, which `(?m)` does
/// not know, so an alternative takes the terminator in. A fence closes with the
/// characters it opened with, which the original says with a back-reference and this
/// with an alternative for each. A URL ends at what JavaScript's `\s` takes for a
/// space, which differs from Unicode's `White_Space` only in U+0085 and U+FEFF.
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

/// A reply of fewer Latin letters than this, about two English sentences, is accepted in
/// any language.
const MIN_LATIN: usize = 200;

/// A stop, as the hook's input on stdin gives it.
struct Stop {
    session: String,
    /// Whether a Stop hook blocked the stop before this one, this hook's or another's.
    continued: bool,
    /// The reply the stop ends in, kept as the input has it until it is judged, where
    /// one that is no string fails the hook, as it made the TypeScript hook throw.
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
    /// The stop on stdin, whose bytes are read as UTF-8 with any invalid sequence
    /// replaced, as node read them for the TypeScript hook.
    fn read() -> Result<Self, Failure> {
        let mut raw = Vec::new();
        io::stdin()
            .read_to_end(&mut raw)
            .map_err(|error| Failure::new(format_args!("cannot read stdin: {error}"), 1))?;
        let input: Value = serde_json::from_str(&String::from_utf8_lossy(&raw))
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

/// Whether JavaScript takes `value` for true, as the TypeScript hook read
/// `stop_hook_active`.
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
    /// than an ASCII letter, digit, `_` or `-` with `_`, as the TypeScript hook's
    /// `/[^\w-]/g` did, so that a session open across the switch keeps its state.
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

/// The temporary directory as Node's `os.tmpdir()` finds it, which kept the TypeScript
/// hook's state: the first of TMPDIR, TMP and TEMP that is set and not empty, or /tmp.
/// `std::env::temp_dir` reads TMPDIR alone, takes an empty one for the working
/// directory, and on macOS asks the system where none is set.
fn temp_dir() -> PathBuf {
    ["TMPDIR", "TMP", "TEMP"]
        .into_iter()
        .filter_map(std::env::var_os)
        .find(|dir| !dir.is_empty())
        .map_or_else(|| PathBuf::from("/tmp"), PathBuf::from)
}

/// Judges the stop on stdin. A failure of the hook's own ends with exit 1 and no verdict,
/// as the TypeScript hook's thrown error did: exit 2 would keep the session from
/// stopping at all.
pub(crate) fn run() -> ExitCode {
    match handle() {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => failure.report(),
    }
}

/// Judges the stop and records what it leaves for the next before printing the verdict,
/// so that a state that cannot be written fails open rather than asking for the rewrite
/// again and again.
fn handle() -> Result<(), Failure> {
    let stop = Stop::read()?;
    let file = SessionFile::of(&stop.session);
    let verdict = stop.judge(file.rewrite_asked())?;
    file.record(verdict.rewrite_asked).map_err(|error| {
        Failure::new(
            format_args!("cannot write {}: {error}", file.path.display()),
            1,
        )
    })?;
    verdict.report().map_err(|error| Failure::writing(&error))
}
