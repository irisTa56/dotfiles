//! `reply-language-stop`: the Stop hook that sends a final reply in English back once to
//! be rewritten in Japanese, run with a temporary directory of the test's own, where it
//! keeps what one stop leaves for the next.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

/// What the hook asks Claude for when it sends a reply back.
const REWRITE: &str = "直前の返答は英語で書かれている。同じ内容を日本語で書き直して、もう一度返答する。ユーザーが英語で求めた文やコードはそのまま残し、その周りの説明を日本語で書く。";

/// A reply of a couple of sentences, too short to be sent back.
const SHORT_ENGLISH: &str =
    "I updated the hook and ran the tests; all of them pass, and the type check is clean too.";

/// A summary long enough to judge.
fn english() -> String {
    format!(
        "{SHORT_ENGLISH} The Stop hook now sends an English final reply back once, and the rewrite is let through. The README line describes the change, and the commit is on the branch, ready for review."
    )
}

/// What a run left: its exit status, its stdout and its stderr.
#[derive(Debug, PartialEq)]
struct Run {
    code: Option<i32>,
    out: String,
    err: String,
}

impl Run {
    /// What the hook leaves when it sends the reply back.
    fn blocked() -> Self {
        Run {
            code: Some(0),
            out: json!({ "decision": "block", "reason": REWRITE }).to_string(),
            err: String::new(),
        }
    }

    /// What the hook leaves when it lets the stop go ahead.
    fn passed() -> Self {
        Run {
            code: Some(0),
            out: String::new(),
            err: String::new(),
        }
    }
}

/// A temporary directory of the test's own, given to the hook as TMPDIR.
struct Sandbox {
    tmp: PathBuf,
}

impl Sandbox {
    fn new(test: &str) -> Self {
        let tmp = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("reply-language-stop")
            .join(test);
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).expect("the sandbox is created");
        Sandbox { tmp }
    }

    /// Runs the hook on a stop of the session `session` with the fields `fields`.
    fn stop(&self, session: &str, fields: &Value) -> Run {
        let mut input = json!({ "session_id": session, "hook_event_name": "Stop" });
        let (Value::Object(input_fields), Value::Object(extra)) = (&mut input, fields) else {
            panic!("the fields are an object");
        };
        input_fields.extend(extra.clone());
        self.run_with_input(input.to_string().as_bytes())
    }

    /// The state file named `name`, where the hook keeps a session's state.
    fn state_file(&self, name: &str) -> PathBuf {
        self.tmp.join("claude-reply-language").join(name)
    }

    /// Runs the hook with `input` as its stdin and the sandbox as TMPDIR.
    fn run_with_input(&self, input: &[u8]) -> Run {
        let mut command = self.command();
        command.env("TMPDIR", &self.tmp);
        Self::run(command, input)
    }

    /// The hook run from the sandbox, with none of the variables naming a temporary
    /// directory set.
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_dotfiles-local"));
        command
            .arg("reply-language-stop")
            .current_dir(&self.tmp)
            .env_remove("TMPDIR")
            .env_remove("TMP")
            .env_remove("TEMP");
        command
    }

    fn run(mut command: Command, input: &[u8]) -> Run {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("dotfiles-local starts");
        let _ = child.stdin.take().expect("stdin is piped").write_all(input);
        let output = child.wait_with_output().expect("dotfiles-local ends");
        Run {
            code: output.status.code(),
            out: String::from_utf8_lossy(&output.stdout).into_owned(),
            err: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }
}

#[test]
fn sends_an_english_final_reply_back_to_be_rewritten_in_japanese() {
    let sandbox = Sandbox::new("english");
    assert_eq!(
        sandbox.stop(
            "s",
            &json!({ "stop_hook_active": false, "last_assistant_message": english() })
        ),
        Run::blocked()
    );
}

/// What the hook leaves on the first stop of a fresh session, ending in `message`.
fn first_stop(test: &str, message: &str) -> Run {
    Sandbox::new(test).stop(
        "s",
        &json!({ "stop_hook_active": false, "last_assistant_message": message }),
    )
}

const JAPANESE: &str = "フックを直し、テストと型チェックがすべて通った。";

#[test]
fn leaves_a_japanese_final_reply_alone() {
    assert_eq!(first_stop("japanese", JAPANESE), Run::passed());
}

#[test]
fn leaves_short_english_alone() {
    assert_eq!(first_stop("short-english", SHORT_ENGLISH), Run::passed());
    assert_eq!(first_stop("too-short", "Done."), Run::passed());
}

// About two English sentences, 200 Latin letters, is what a reply needs to be judged; a
// heading over a list of paths, or a title the user asked for in English, has fewer.
#[test]
fn judges_a_reply_from_200_latin_letters() {
    let letters = |count: usize| "abcd ".repeat(count / 4) + &"z".repeat(count % 4);
    assert_eq!(first_stop("199-letters", &letters(199)), Run::passed());
    assert_eq!(first_stop("200-letters", &letters(200)), Run::blocked());
}

// Only the letters A to Z count, not digits or letters with a diacritic.
#[test]
fn counts_only_ascii_letters() {
    let short = "abcd ".repeat(49) + "zzz";
    assert_eq!(first_stop("digits", &format!("{short} 1")), Run::passed());
    assert_eq!(first_stop("accented", &format!("{short} é")), Run::passed());
    assert_eq!(first_stop("upper", &format!("{short} Z")), Run::blocked());
}

// A Japanese sentence is written with hiragana and punctuated with 。 and 、, any one
// of which makes the reply Japanese.
#[test]
fn takes_hiragana_and_japanese_punctuation_for_a_japanese_sentence() {
    for (test, mark) in [("hiragana", "の"), ("maru", "。"), ("ten", "、")] {
        assert_eq!(
            (test, first_stop(test, &format!("{} {mark}", english()))),
            (test, Run::passed())
        );
    }
}

// A term or a file name an English reply names is kanji or katakana, which carries no
// Japanese sentence.
#[test]
fn sends_back_english_naming_japanese_terms_outside_quotes() {
    for (test, terms) in [
        ("kanji", "See 設計.md and 概要.md."),
        ("katakana", "See the カタカナ heading."),
    ] {
        assert_eq!(
            (test, first_stop(test, &format!("{} {terms}", english()))),
            (test, Run::blocked())
        );
    }
}

#[test]
fn leaves_kanji_katakana_and_ten_around_long_english_paths_alone() {
    let message = [
        "変更対象、計五件",
        "- .claude/skills/global-hooks/hooks/reply-language.ts",
        "- .claude/skills/global-hooks/hooks/reply-language.test.ts",
        "- .claude/skills/global-hooks/context/reply-language.md",
        "- .claude/skills/global-hooks/hooks/index.ts",
        "- .claude/skills/global-hooks/hooks/subagent-model.ts",
        "- .claude/skills/global-hooks/hooks/scheduled-chrome.ts",
        "- README.md",
    ]
    .join("\n");
    assert_eq!(first_stop("paths", &message), Run::passed());
}

/// The four kinds of quotation marks a reply quotes in, opening and closing.
const QUOTES: [(&str, &str, &str); 4] = [
    ("kagi", "「", "」"),
    ("nijukagi", "『", "』"),
    ("ascii", "\"", "\""),
    ("curly", "“", "”"),
];

// A quotation is in the language of what it quotes, so a Japanese sentence quoted in an
// English reply leaves it English.
#[test]
fn sends_back_english_quoting_a_japanese_sentence() {
    for (name, open, close) in QUOTES {
        let message = format!(
            "{} The rule says {open}ユーザーへの返答は日本語で書く。{close} and I followed it.",
            english()
        );
        assert_eq!(
            (name, first_stop(&format!("quote-{name}"), &message)),
            (name, Run::blocked())
        );
    }
}

// A quotation closes on the line it opens on, so a mark left open does not hide the
// Japanese on the lines after it.
#[test]
fn ends_a_quotation_at_the_end_of_its_line() {
    for (name, open, close) in QUOTES {
        let message = format!(
            "{} {open}a\nユーザーへの返答は日本語で書く。{close}",
            english()
        );
        assert_eq!(
            (name, first_stop(&format!("open-quote-{name}"), &message)),
            (name, Run::passed())
        );
    }
}

#[test]
fn leaves_japanese_quoting_an_english_sentence_alone() {
    let message = "README には \"A final reply in English is sent back once to be rewritten in Japanese, and the rewrite is let through.\" とある。";
    assert_eq!(first_stop("quoting-english", message), Run::passed());
}

// Code is English by rule, so Japanese inside it says nothing about the reply's own
// language: in a fence of backticks or tildes, indented by spaces or a tab, or inline.
#[test]
fn sends_back_english_with_japanese_in_code() {
    for (name, code) in [
        ("backticks", "```ts\n// 日本語のコメントがある。\n```"),
        ("tildes", "~~~ts\n// 日本語のコメントがある。\n~~~"),
        (
            "backticks-tab-spaces",
            "\t```ts\n// 日本語のコメントがある。\n  ```",
        ),
        (
            "backticks-spaces-tab",
            "  ```ts\n// 日本語のコメントがある。\n\t```",
        ),
        (
            "tildes-tab-spaces",
            "\t~~~ts\n// 日本語のコメントがある。\n  ~~~",
        ),
        (
            "tildes-spaces-tab",
            "  ~~~ts\n// 日本語のコメントがある。\n\t~~~",
        ),
        ("inline", "`label: 日本語で書く。`"),
    ] {
        let message = format!("{}\n{code}", english());
        assert_eq!(
            (name, first_stop(&format!("code-{name}"), &message)),
            (name, Run::blocked())
        );
    }
}

// Where what looks like code is none, the Japanese around it is the reply's own: a fence
// opens only at a line start, ends at the first closing line, closes only
// with the characters it opened with, and inline code ends with its line.
#[test]
fn reads_japanese_outside_what_is_no_code() {
    let japanese = "ユーザーへの返答は日本語で書く。";
    for (name, text) in [
        (
            "opener-mid-line",
            format!("I wrote ``` inline.\n{japanese}\n```"),
        ),
        (
            "tilde-opener-mid-line",
            format!("I wrote ~~~ inline.\n{japanese}\n~~~"),
        ),
        (
            "between-fences",
            format!("```\nx\n```\n{japanese}\n```\ny\n```"),
        ),
        (
            "between-tilde-fences",
            format!("~~~\nx\n~~~\n{japanese}\n~~~\ny\n~~~"),
        ),
        (
            "backticks-closed-by-tildes",
            format!("```\n{japanese}\n~~~"),
        ),
        (
            "tildes-closed-by-backticks",
            format!("~~~\n{japanese}\n```"),
        ),
        ("inline-across-lines", format!("`a\n{japanese}`")),
    ] {
        let message = format!("{}\n{text}", english());
        assert_eq!(
            (name, first_stop(&format!("no-code-{name}"), &message)),
            (name, Run::passed())
        );
    }
}

// A fence closes only at a line start, so its closing characters mid-line inside it do
// not end it early.
#[test]
fn sends_back_english_with_a_fence_holding_its_closing_characters_mid_line() {
    for (name, fence) in [("backticks", "```"), ("tildes", "~~~")] {
        let message = format!(
            "{}\n{fence}\nx {fence} 日本語のコメントがある。\n{fence}",
            english()
        );
        assert_eq!(
            (
                name,
                first_stop(&format!("closer-mid-line-{name}"), &message)
            ),
            (name, Run::blocked())
        );
    }
}

// A triple backtick named in prose opens no fence, so it hides none of the reply.
#[test]
fn sends_back_english_naming_an_unclosed_triple_backtick() {
    let message = format!(
        "{} I fixed the parser, which now treats ``` as an opener and keeps going.",
        english()
    );
    assert_eq!(first_stop("unclosed-fence", &message), Run::blocked());
}

#[test]
fn leaves_japanese_around_english_code_alone() {
    let message = [
        &format!("{JAPANESE}次のコマンドで確かめられる。"),
        "```bash",
        "mise run pre-commit && claude plugin test .claude/skills/global-hooks --verbose",
        "```",
        "`git log --oneline --decorate --graph` also shows the commit.",
    ]
    .join("\n");
    assert_eq!(first_stop("around-code", &message), Run::passed());
}

// The prose, not the URLs it lists, is what is too short to judge, whether over https
// or http.
#[test]
fn leaves_short_english_listing_long_urls_alone() {
    for scheme in ["https", "http"] {
        let message = [
            "See these:",
            "://code.claude.com/docs/en/hooks#stop-and-subagentstop-decision-control",
            "://code.claude.com/docs/en/plugins/create#scaffold-a-plugin-that-loads-every-session",
            "://code.claude.com/docs/en/permission-modes#how-the-classifier-evaluates-actions",
            "://code.claude.com/docs/en/settings#available-settings-and-their-defaults",
        ]
        .join(&format!("\n{scheme}"));
        assert_eq!(
            (scheme, first_stop(&format!("urls-{scheme}"), &message)),
            (scheme, Run::passed())
        );
    }
}

// A URL ends at what JavaScript's `\s` takes for a space: U+FEFF ends it and U+0085
// does not, where Unicode's White_Space has them the other way round.
#[test]
fn ends_a_url_where_javascript_sees_a_space() {
    for (separator, outcome) in [
        (' ', Run::passed()),
        ('\t', Run::passed()),
        ('\n', Run::passed()),
        ('\u{a0}', Run::passed()),
        ('\u{2028}', Run::passed()),
        ('\u{3000}', Run::passed()),
        ('\u{feff}', Run::passed()),
        ('\u{85}', Run::blocked()),
        ('\u{200b}', Run::blocked()),
    ] {
        let message = format!(
            "{} https://example.com{separator}ユーザーへの返答は日本語で書く。",
            english()
        );
        let test = format!("url-end-{:x}", u32::from(separator));
        assert_eq!(
            (separator, first_stop(&test, &message)),
            (separator, outcome)
        );
    }
}

// A line starts after any line terminator JavaScript knows, so a fence opens and closes
// after each; the backtick in it keeps inline code from hiding the line instead.
#[test]
fn starts_a_line_after_every_javascript_line_terminator() {
    for (name, newline) in [
        ("lf", "\n"),
        ("crlf", "\r\n"),
        ("cr", "\r"),
        ("ls", "\u{2028}"),
        ("ps", "\u{2029}"),
    ] {
        let message = [english().as_str(), "```", "日本語の`コメント。", "```"].join(newline);
        assert_eq!(
            (name, first_stop(&format!("newline-{name}"), &message)),
            (name, Run::blocked())
        );
    }
}

/// A stop of the session `s` ending in an English reply, `continued` where a Stop hook
/// blocked the stop before.
fn english_stop(sandbox: &Sandbox, continued: bool) -> Run {
    sandbox.stop(
        "s",
        &json!({ "stop_hook_active": continued, "last_assistant_message": english() }),
    )
}

// The reply after the hook's own block is already the rewrite; another block would loop.
#[test]
fn lets_the_rewrite_through_on_the_stop_after_its_own_block() {
    let sandbox = Sandbox::new("rewrite");
    assert_eq!(english_stop(&sandbox, false), Run::blocked());
    assert_eq!(english_stop(&sandbox, true), Run::passed());
}

// A subagent's replies go to its parent agent, not the user.
#[test]
fn leaves_a_subagent_alone() {
    let sandbox = Sandbox::new("subagent");
    let fields = json!({
        "stop_hook_active": false,
        "last_assistant_message": english(),
        "agent_id": "sub-1",
    });
    assert_eq!(sandbox.stop("s", &fields), Run::passed());
}

// A subagent's stop leaves what the main conversation's last stop left, so the rewrite
// after the hook's own block is still let through.
#[test]
fn keeps_the_main_conversation_s_state_over_a_subagent_s_stop() {
    let sandbox = Sandbox::new("subagent-between");
    assert_eq!(english_stop(&sandbox, false), Run::blocked());
    let subagent = json!({ "stop_hook_active": false, "agent_id": "sub-1" });
    assert_eq!(sandbox.stop("s", &subagent), Run::passed());
    assert_eq!(english_stop(&sandbox, true), Run::passed());
}

// Another Stop hook, such as /goal's, continued the turn instead, so the reply is judged
// like any other.
#[test]
fn sends_back_english_after_another_hook_s_block() {
    let sandbox = Sandbox::new("other-hook");
    assert_eq!(english_stop(&sandbox, true), Run::blocked());
}

// A turn's first stop never follows a block, so the flag from an earlier turn is
// overwritten there.
#[test]
fn sends_back_english_in_the_next_turn_after_its_own_block() {
    let sandbox = Sandbox::new("next-turn");
    assert_eq!(english_stop(&sandbox, false), Run::blocked());
    assert_eq!(english_stop(&sandbox, false), Run::blocked());
}

// The rewrite was let through, and a hook beneath, such as /goal's, then continued the
// turn.
#[test]
fn sends_back_english_after_its_rewrite_and_another_hook_s_block() {
    let sandbox = Sandbox::new("rewrite-then-other-hook");
    assert_eq!(english_stop(&sandbox, false), Run::blocked());
    assert_eq!(english_stop(&sandbox, true), Run::passed());
    assert_eq!(english_stop(&sandbox, true), Run::blocked());
}

// Sessions run side by side, so one session's block does not let another's reply
// through.
#[test]
fn keeps_one_session_s_block_from_letting_another_s_reply_through() {
    let sandbox = Sandbox::new("sessions");
    let english =
        |continued| json!({ "stop_hook_active": continued, "last_assistant_message": english() });
    assert_eq!(sandbox.stop("a", &english(false)), Run::blocked());
    assert_eq!(sandbox.stop("b", &english(true)), Run::blocked());
    assert_eq!(sandbox.stop("a", &english(true)), Run::passed());
}

#[test]
fn leaves_a_stop_with_no_text_alone() {
    let sandbox = Sandbox::new("no-text");
    assert_eq!(
        sandbox.stop("s", &json!({ "stop_hook_active": false })),
        Run::passed()
    );
}

// A Japanese reply listing English identifiers, commit subjects, file names or paths
// still has a Japanese sentence, or too few Latin letters to judge.
#[test]
fn leaves_japanese_around_english_names_alone() {
    for (name, message) in [
        (
            "identifiers",
            "PostToolBatch hook の additionalContext と Stop hook の last_assistant_message を確かめた。".to_owned(),
        ),
        (
            "commit-subjects",
            [
                "コミットは次の通り。",
                "- feat(global-hooks): add the reply-language context from the mod",
                "- fix(global-hooks): leave quotations out when judging a final reply's language",
                "- refactor(global-hooks): stop repeating the reply-language context after Skill and Agent batches",
            ]
            .join("\n"),
        ),
        (
            "table",
            [
                "結果:",
                "",
                "| file | status |",
                "| --- | --- |",
                "| reply-language.ts | modified |",
                "| reply-language.test.ts | modified |",
                "| reply-language.md | unchanged |",
                "",
                "以上。",
            ]
            .join("\n"),
        ),
        (
            "heading-over-paths",
            [
                "ファイル一覧",
                "- src/index.ts",
                "- src/reply-language.ts",
                "- src/reply-language.test.ts",
                "- README.md",
            ]
            .join("\n"),
        ),
    ] {
        assert_eq!(
            (name, first_stop(&format!("names-{name}"), &message)),
            (name, Run::passed())
        );
    }
}

// The state is "1" after the hook's own block and "0" after any other stop, in a file per
// session under `$TMPDIR/claude-reply-language`. Anything else in the file counts as no
// block.
#[test]
fn keeps_the_state_in_a_file_per_session_under_the_temporary_directory() {
    let sandbox = Sandbox::new("state-file");
    english_stop(&sandbox, false);
    assert_eq!(
        fs::read_to_string(sandbox.state_file("s")).ok().as_deref(),
        Some("1")
    );
    english_stop(&sandbox, true);
    assert_eq!(
        fs::read_to_string(sandbox.state_file("s")).ok().as_deref(),
        Some("0")
    );
    fs::write(sandbox.state_file("s"), "1").expect("the state is written");
    assert_eq!(english_stop(&sandbox, true), Run::passed());
    for text in ["", "true", "1\n"] {
        fs::write(sandbox.state_file("s"), text).expect("the state is written");
        assert_eq!((text, english_stop(&sandbox, true)), (text, Run::blocked()));
    }
}

// A session id is made one file name by replacing each UTF-16 code unit other than an
// ASCII letter, digit, `_` or `-` with `_`.
#[test]
fn names_the_state_file_after_the_session() {
    for (session, name) in [
        ("../a/b.c", "___a_b_c"),
        ("x-y_Z9", "x-y_Z9"),
        ("é😀", "___"),
    ] {
        let sandbox = Sandbox::new(&format!("session-name-{name}"));
        let fields = json!({ "stop_hook_active": false, "last_assistant_message": english() });
        sandbox.stop(session, &fields);
        assert_eq!(
            (session, fs::read_to_string(sandbox.state_file(name)).ok()),
            (session, Some("1".to_owned()))
        );
    }
}

// The temporary directory is the first of TMPDIR, TMP and TEMP that is set and not empty,
// or /tmp.
#[test]
fn finds_the_temporary_directory_in_tmpdir_tmp_or_temp() {
    let input = json!({
        "session_id": format!("dotfiles-local-test-{}", std::process::id()),
        "stop_hook_active": false,
        "last_assistant_message": english(),
    })
    .to_string();
    let sandbox = Sandbox::new("tmpdir");
    let other = sandbox.tmp.join("other");
    let state = |dir: &Path| {
        fs::read_to_string(
            dir.join("claude-reply-language")
                .join(format!("dotfiles-local-test-{}", std::process::id())),
        )
        .ok()
    };
    for (name, env) in [
        (
            "tmp-after-empty-tmpdir",
            vec![("TMPDIR", ""), ("TMP", "{dir}"), ("TEMP", "{other}")],
        ),
        ("temp", vec![("TEMP", "{dir}")]),
        (
            "tmpdir-before-tmp",
            vec![("TMPDIR", "{dir}/"), ("TMP", "{other}")],
        ),
    ] {
        let dir = sandbox.tmp.join(name);
        let mut command = sandbox.command();
        for (variable, value) in env {
            let value = value
                .replace("{dir}", &dir.to_string_lossy())
                .replace("{other}", &other.to_string_lossy());
            command.env(variable, value);
        }
        assert_eq!(
            (name, Sandbox::run(command, input.as_bytes()), state(&dir)),
            (name, Run::blocked(), Some("1".to_owned()))
        );
    }
    let tmp = Path::new("/tmp");
    let run = Sandbox::run(sandbox.command(), input.as_bytes());
    let written = state(tmp);
    let _ = fs::remove_file(
        tmp.join("claude-reply-language")
            .join(format!("dotfiles-local-test-{}", std::process::id())),
    );
    assert_eq!((run, written), (Run::blocked(), Some("1".to_owned())));
}

/// What the hook leaves where it fails: exit 1, no verdict, and a message on stderr.
fn assert_failed(run: &Run) {
    assert_eq!((run.code, run.out.as_str()), (Some(1), ""), "{run:?}");
    assert!(!run.err.is_empty(), "{run:?}");
}

// A failure of the hook's own ends with exit 1 and no verdict: exit 2 would keep every
// session from stopping, and 1 puts a hook error in front of the user and lets the stop
// go ahead.
#[test]
fn fails_with_exit_1_on_input_it_cannot_read() {
    let sandbox = Sandbox::new("bad-input");
    for input in [
        "not json",
        "null",
        "[]",
        r#"{"stop_hook_active": false}"#,
        r#"{"session_id": 5, "stop_hook_active": false}"#,
        r#"{"session_id": "a\UD83Db", "stop_hook_active": false}"#,
    ] {
        let run = sandbox.run_with_input(input.as_bytes());
        assert_failed(&run);
    }
}

// The state is written before the verdict is printed, so a state that cannot be written
// fails the hook before it sends a reply back, rather than sending it back again and
// again.
#[test]
fn fails_with_exit_1_before_the_verdict_where_the_state_cannot_be_written() {
    let sandbox = Sandbox::new("unwritable");
    fs::write(sandbox.tmp.join("claude-reply-language"), "").expect("a file blocks the directory");
    assert_failed(&english_stop(&sandbox, false));
}

// A verdict that cannot be written fails the hook too.
#[test]
fn fails_with_exit_1_where_the_verdict_cannot_be_written() {
    let sandbox = Sandbox::new("closed-stdout");
    let input = json!({
        "session_id": "s",
        "stop_hook_active": false,
        "last_assistant_message": english(),
    })
    .to_string();
    let mut child = sandbox
        .command()
        .env("TMPDIR", &sandbox.tmp)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("dotfiles-local starts");
    drop(child.stdout.take());
    let _ = child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(input.as_bytes());
    let output = child.wait_with_output().expect("dotfiles-local ends");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
}

// Bytes that are no UTF-8 are read with each invalid sequence replaced, rather than
// failing the hook.
#[test]
fn reads_input_that_is_no_utf_8() {
    let sandbox = Sandbox::new("no-utf-8");
    let input = json!({
        "session_id": "s",
        "stop_hook_active": false,
        "last_assistant_message": format!("{} INVALID", english()),
    })
    .to_string();
    let (before, after) = input.split_once("INVALID").expect("the marker is there");
    let bytes = [before.as_bytes(), &[0xff], after.as_bytes()].concat();
    assert_eq!(sandbox.run_with_input(&bytes), Run::blocked());
}

// Fields of a type Claude Code never sends: a reply that is no string fails the hook
// where it comes to be judged, and `stop_hook_active` is true as JavaScript takes a value
// for true.
#[test]
fn reads_fields_of_other_types() {
    let sandbox = Sandbox::new("other-types");
    for message in [json!(null), json!(5)] {
        let run = sandbox.stop(
            "judged",
            &json!({ "stop_hook_active": false, "last_assistant_message": message }),
        );
        assert_failed(&run);
        let subagent = json!({ "last_assistant_message": message, "agent_id": "sub-1" });
        assert_eq!(sandbox.stop("subagent", &subagent), Run::passed());
    }
    for (continued, outcome) in [
        (json!("yes"), Run::passed()),
        (json!(1), Run::passed()),
        (json!({}), Run::passed()),
        (json!(0), Run::blocked()),
        (json!(""), Run::blocked()),
        (json!(null), Run::blocked()),
    ] {
        let fields = json!({ "stop_hook_active": continued, "last_assistant_message": english() });
        fs::create_dir_all(sandbox.state_file("")).expect("the state directory is created");
        fs::write(sandbox.state_file("truthy"), "1").expect("the state is written");
        assert_eq!(
            (&continued, sandbox.stop("truthy", &fields)),
            (&continued, outcome)
        );
    }
}

// A lone UTF-16 surrogate escape in a string, which `JSON.parse` accepted, leaves the
// input readable: the reply is judged like any other, so English holding one is sent back
// and Japanese holding one is left alone. A paired escape and an escaped backslash before
// `ud83d` (a backslash and letters, no escape) read as they always did.
#[test]
fn judges_a_reply_holding_a_lone_surrogate_escape() {
    let japanese =
        "実装を更新して、テストをすべて実行した。結果はすべて成功で、型の検査にも問題はない。"
            .repeat(4);
    let judge = |reply: &str| {
        let input = format!(
            r#"{{"session_id":"s","stop_hook_active":false,"last_assistant_message":"{reply}"}}"#
        );
        Sandbox::new("lone-surrogate-reply").run_with_input(input.as_bytes())
    };
    for (name, escape) in [
        ("high", r"\ud83d"),
        ("low", r"\ude00"),
        ("high before a letter", r"\ud83dx"),
        ("high before another high", r"\ud83d\ud83d"),
        ("low before a high", r"\ude00\ud83d"),
        ("pair", r"😀"),
        ("escaped backslash", r"\\ud83d"),
    ] {
        let english = format!("{} {escape}", english());
        let japanese = format!("{japanese}{escape}");
        assert_eq!((name, judge(&english)), (name, Run::blocked()));
        assert_eq!((name, judge(&japanese)), (name, Run::passed()));
    }
    // A pair of escapes spells the character itself, here U+1F200, whose script is
    // Hiragana.
    let hiragana = format!("{} \\ud83c\\ude00", english());
    assert_eq!(judge(&hiragana), Run::passed());
}

// A lone surrogate escape in the session id is one UTF-16 code unit, so it is one `_` in
// the state file's name.
#[test]
fn names_the_state_file_with_one_underscore_for_a_lone_surrogate_escape() {
    for (escape, name) in [
        (r"a\ud83db", "a_b"),
        (r"a\ude00b", "a_b"),
        (r"a\ud83d", "a_"),
        (r"a\ud83d\ud83db", "a__b"),
        (r"a😀b", "a__b"),
        (r"a\\ud83db", "a_ud83db"),
    ] {
        let sandbox = Sandbox::new("lone-surrogate-session");
        let input = format!(
            r#"{{"session_id":"{escape}","stop_hook_active":false,"last_assistant_message":"{}"}}"#,
            english()
        );
        let run = sandbox.run_with_input(input.as_bytes());
        assert_eq!(
            (
                escape,
                run,
                fs::read_to_string(sandbox.state_file(name)).ok()
            ),
            (escape, Run::blocked(), Some("1".to_owned()))
        );
    }
}
