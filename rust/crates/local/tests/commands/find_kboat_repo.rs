//! `find-kboat-repo-export-catalog` and `find-kboat-repo-lookup`: export the K-Boat repo
//! catalogue through `kboat-note` into chunks for judging, and look up the records of
//! the titles judged to fit. A fake mise, which records how it was called and runs what
//! follows `--`, and a fake kboat-note, which prints a fixture, are all that is on PATH.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::support::{DOTFILES_LOCAL, Sandbox};

/// A sandbox holding the fakes, the k-boat checkout they are pointed at, and the
/// temporary directory the export lands in.
struct Fixture {
    sandbox: Sandbox,
}

impl Fixture {
    /// A fresh fixture named after `test`, where kboat-note prints `catalog` and exits 0.
    fn new(test: &str) -> Self {
        let sandbox = Sandbox::new("find-kboat-repo", test, &["k-boat", "tmp"]);
        sandbox.fake(
            "mise",
            "printf '%s\\n' \"$@\" > \"$SANDBOX/mise-args\"\n\
             while [ \"$1\" != -- ]; do shift; done\n\
             shift\n\
             exec \"$@\"",
        );
        let fixture = Fixture { sandbox };
        fixture.kboat_note("{\"anomalies\": [], \"notes\": []}", 0);
        fixture
    }

    fn root(&self) -> &Path {
        self.sandbox.root()
    }

    /// Makes kboat-note print `output` and exit with `code`.
    fn kboat_note(&self, output: &str, code: u8) {
        fs::write(self.root().join("kboat-note-output"), output).expect("the fixture is written");
        self.sandbox.fake(
            "kboat-note",
            &format!("/bin/cat \"$SANDBOX/kboat-note-output\"\nexit {code}"),
        );
    }

    /// The arguments mise was called with, one a line, or `None` where it was not called.
    fn mise_args(&self) -> Option<String> {
        fs::read_to_string(self.root().join("mise-args")).ok()
    }

    /// `dotfiles-local` with `args`, given the sandbox's PATH alone and its temporary
    /// directory, and the k-boat checkout as `KBOAT_REPO`.
    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(DOTFILES_LOCAL);
        command
            .args(args)
            .env_clear()
            .env("PATH", self.sandbox.bin())
            .env("SANDBOX", self.root())
            .env("TMPDIR", self.root().join("tmp"))
            .env("KBOAT_REPO", self.root().join("k-boat"));
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().expect("dotfiles-local starts")
    }
}

// Without KBOAT_REPO there is no checkout to read the catalogue through, so the run
// says where it is set and exits 2 before running anything.
#[test]
fn export_fails_without_kboat_repo() {
    let sandbox = Fixture::new("no-kboat-repo");
    for value in [None, Some("")] {
        let mut command = sandbox.command(&["find-kboat-repo-export-catalog"]);
        match value {
            None => command.env_remove("KBOAT_REPO"),
            Some(value) => command.env("KBOAT_REPO", value),
        };
        let run = command.output().expect("dotfiles-local starts");

        assert_eq!(run.status.code(), Some(2));
        assert_eq!(
            String::from_utf8_lossy(&run.stderr),
            "KBOAT_REPO is not set: it names this machine's k-boat checkout (dotfiles README, Setting up K-Boat).\n"
        );
        assert_eq!(run.stdout, [] as [u8; 0]);
        assert_eq!(sandbox.mise_args(), None);
    }
}

/// A kboat-note report holding a note whose frontmatter gives `fields`, a JSON object's
/// members, for each of `notes`, with `anomalies`.
fn report(notes: &[&str], anomalies: &str) -> String {
    let notes: Vec<String> = notes
        .iter()
        .enumerate()
        .map(|(index, fields)| {
            format!(
                "{{\"slug\": \"s{index}\", \"path\": \"Repos/s{index}.md\", \"frontmatter\": {{{fields}}}}}"
            )
        })
        .collect();
    format!(
        "{{\"anomalies\": {anomalies}, \"counts\": {{\"notes\": {}}}, \"notes\": [{}]}}\n",
        notes.len(),
        notes.join(", ")
    )
}

/// The directory a run's printed summary names.
fn exported_dir(stdout: &[u8]) -> PathBuf {
    let summary: serde_json::Value = serde_json::from_slice(stdout).expect("stdout is JSON");
    PathBuf::from(summary["dir"].as_str().expect("dir is a string"))
}

// The catalogue is read through `mise -C $KBOAT_REPO x -- kboat-note list` with the
// fields judging and lookup read, its report kept whole as catalog.json in a fresh
// directory under TMPDIR, and the run prints that directory, the note count, the
// anomalies with their keys in the report's order and the chunk files, indented by two
// spaces.
#[test]
fn export_reads_the_catalogue_through_kboat_note() {
    let sandbox = Fixture::new("export");
    let output = report(
        &[
            "\"title\": \"a/one\", \"role\": \"library\"",
            "\"title\": \"b/two\", \"role\": \"tool\"",
        ],
        "[{\"path\": \"Repos/bad.md\", \"error\": \"no frontmatter\"}]",
    );
    sandbox.kboat_note(&output, 0);

    let run = sandbox.run(&["find-kboat-repo-export-catalog"]);

    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let k_boat = sandbox.root().join("k-boat");
    assert_eq!(
        sandbox.mise_args().as_deref(),
        Some(
            format!(
                "-C\n{}\nx\n--\nkboat-note\nlist\n--type\nrepo\n\
                 --field\ntitle\n--field\nurl\n--field\ndescription\n--field\ntopics\n\
                 --field\nsummary\n--field\ndomain\n--field\nrole\n--field\narchived\n\
                 --field\nlast_commit\n--field\nstatus\n",
                k_boat.display()
            )
            .as_str()
        )
    );
    let dir = exported_dir(&run.stdout);
    assert_eq!(dir.parent(), Some(sandbox.root().join("tmp").as_path()));
    assert!(
        dir.file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("tmp.")),
        "{}",
        dir.display()
    );
    assert_eq!(
        fs::read_to_string(dir.join("catalog.json")).expect("catalog.json is written"),
        output
    );
    let dir = dir.display();
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        format!(
            "{{\n  \"dir\": \"{dir}\",\n  \"notes\": 2,\n  \"anomalies\": [\n    {{\n      \"path\": \"Repos/bad.md\",\n      \"error\": \"no frontmatter\"\n    }}\n  ],\n  \"chunks\": [\n    \"{dir}/chunk-aa\"\n  ]\n}}\n"
        )
    );
}

// Each note becomes one line of what judging reads, in a fixed order, with escaped
// text written out as UTF-8. A list kboat-note gives in flow style as a string,
// such as `[robotics, ml]`, becomes an array, as an empty or missing one does; an array
// stays as it is.
#[test]
fn export_compacts_each_note_for_judging() {
    let sandbox = Fixture::new("compact");
    sandbox.kboat_note(
        &report(
            &[
                "\"summary\": \"\\u30ed\\u30dc\\u30c3\\u30c8\", \"title\": \"a/one\", \
                 \"url\": \"https://github.com/a/one\", \"domain\": \"[robotics,  ml ,]\", \
                 \"topics\": [\"x\", \"y\"], \"description\": \"An arm\", \"role\": \"library\"",
                "\"title\": \"b/two\", \"domain\": \"\", \"topics\": null",
                "\"title\": \"c/three\", \"domain\": \"[]\", \"topics\": []",
                "\"title\": \"d/four\"",
            ],
            "[]",
        ),
        0,
    );

    let run = sandbox.run(&["find-kboat-repo-export-catalog"]);

    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let dir = exported_dir(&run.stdout);
    let compact = fs::read_to_string(dir.join("compact.jsonl")).expect("compact.jsonl is written");
    assert_eq!(
        compact,
        "{\"title\":\"a/one\",\"role\":\"library\",\"domain\":[\"robotics\",\"ml\"],\"topics\":[\"x\",\"y\"],\"description\":\"An arm\",\"summary\":\"ロボット\"}\n\
         {\"title\":\"b/two\",\"role\":null,\"domain\":[],\"topics\":[],\"description\":null,\"summary\":null}\n\
         {\"title\":\"c/three\",\"role\":null,\"domain\":[],\"topics\":[],\"description\":null,\"summary\":null}\n\
         {\"title\":\"d/four\",\"role\":null,\"domain\":[],\"topics\":[],\"description\":null,\"summary\":null}\n"
    );
    assert_eq!(
        fs::read_to_string(dir.join("chunk-aa")).expect("chunk-aa is written"),
        compact
    );
}

// The compact lines are split 100 to a chunk, named as `split -l 100` names its files,
// and the summary lists every chunk in order.
#[test]
fn export_splits_the_notes_into_chunks_of_100() {
    let sandbox = Fixture::new("chunks");
    let notes: Vec<String> = (0..250)
        .map(|index| format!("\"title\": \"o/r{index}\""))
        .collect();
    let notes: Vec<&str> = notes.iter().map(String::as_str).collect();
    sandbox.kboat_note(&report(&notes, "[]"), 0);

    let run = sandbox.run(&["find-kboat-repo-export-catalog"]);

    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let dir = exported_dir(&run.stdout);
    let summary: serde_json::Value = serde_json::from_slice(&run.stdout).expect("stdout is JSON");
    assert_eq!(summary["notes"], 250);
    let chunks: Vec<PathBuf> = ["aa", "ab", "ac"]
        .iter()
        .map(|suffix| dir.join(format!("chunk-{suffix}")))
        .collect();
    assert_eq!(
        summary["chunks"],
        serde_json::Value::from(
            chunks
                .iter()
                .map(|path| path.to_string_lossy())
                .collect::<Vec<_>>()
        )
    );
    let titles = |path: &Path| -> Vec<String> {
        fs::read_to_string(path)
            .expect("the chunk is written")
            .lines()
            .map(|line| {
                let note: serde_json::Value = serde_json::from_str(line).expect("a line is JSON");
                note["title"]
                    .as_str()
                    .expect("title is a string")
                    .to_owned()
            })
            .collect()
    };
    for (index, chunk) in chunks.iter().enumerate() {
        let expected: Vec<String> = (index * 100..(index * 100 + 100).min(250))
            .map(|n| format!("o/r{n}"))
            .collect();
        assert_eq!(titles(chunk), expected);
    }
    assert!(!dir.join("chunk-ad").exists());
}

// Where kboat-note fails, the run exits with its code and prints the directory, that
// code and the anomalies its report gives, which say why Repos/ could not be read, or
// null where it left no report.
#[test]
fn export_reports_a_failing_kboat_note() {
    let sandbox = Fixture::new("failing");
    for (output, code, anomalies) in [
        (
            "{\"anomalies\": [{\"path\": \"Repos\", \"error\": \"missing\"}], \"notes\": []}",
            1,
            "[\n    {\n      \"path\": \"Repos\",\n      \"error\": \"missing\"\n    }\n  ]",
        ),
        ("Traceback (most recent call last):", 3, "null"),
        ("", 4, "null"),
    ] {
        sandbox.kboat_note(output, code);

        let run = sandbox.run(&["find-kboat-repo-export-catalog"]);

        assert_eq!(run.status.code(), Some(i32::from(code)));
        let dir = exported_dir(&run.stdout);
        assert_eq!(
            String::from_utf8_lossy(&run.stdout),
            format!(
                "{{\n  \"dir\": \"{}\",\n  \"exit\": {code},\n  \"anomalies\": {anomalies}\n}}\n",
                dir.display()
            )
        );
        assert!(!dir.join("compact.jsonl").exists());
    }
}

// Where kboat-note exits 0 but prints what is not a report, the run fails with nothing on
// stdout, and its stderr gives the path of the catalog.json that holds the output, so
// that it can be inspected.
#[test]
fn export_names_the_stored_output_when_it_is_not_a_report() {
    let sandbox = Fixture::new("unparsable");
    sandbox.kboat_note("not json", 0);

    let run = sandbox.run(&["find-kboat-repo-export-catalog"]);

    assert_ne!(run.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&run.stdout), "");
    let dirs: Vec<_> = fs::read_dir(sandbox.root().join("tmp"))
        .expect("the temporary directory is readable")
        .map(|entry| entry.expect("an entry is read").path())
        .collect();
    assert_eq!(dirs.len(), 1, "the export creates one directory");
    let catalog = dirs[0].join("catalog.json");
    assert_eq!(
        fs::read_to_string(&catalog).expect("the output is stored"),
        "not json"
    );
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains(&catalog.display().to_string()),
        "stderr does not name {}: {stderr}",
        catalog.display()
    );
}

// `--help` prints how the command is called; any other argument is a usage error before
// kboat-note runs.
#[test]
fn export_takes_no_arguments() {
    let sandbox = Fixture::new("export-args");

    let help = sandbox.run(&["find-kboat-repo-export-catalog", "--help"]);
    let other = sandbox.run(&["find-kboat-repo-export-catalog", "extra"]);

    let help_out = String::from_utf8_lossy(&help.stdout);
    assert_eq!(help.status.code(), Some(0));
    assert!(
        help_out.contains("Usage: dotfiles-local find-kboat-repo-export-catalog\n"),
        "{help_out}"
    );
    let other_err = String::from_utf8_lossy(&other.stderr);
    assert_eq!(other.status.code(), Some(2));
    assert!(
        other_err.starts_with("error: unexpected argument 'extra'")
            && other_err.contains("Usage: dotfiles-local find-kboat-repo-export-catalog\n"),
        "{other_err}"
    );
    assert_eq!(other.stdout, [] as [u8; 0]);
    assert_eq!(sandbox.mise_args(), None);
}

// Lookup prints, one compact line each and in the catalogue's order, the record of each
// note whose title it is given, from the catalog.json an export left; a title the
// catalogue does not hold prints nothing, and a title matches only one equal to it, not
// one it begins or one differing in case.
#[test]
fn lookup_prints_the_record_of_each_title() {
    let sandbox = Fixture::new("lookup");
    let dir = sandbox.root().join("export");
    fs::create_dir_all(&dir).expect("the export directory is created");
    fs::write(
        dir.join("catalog.json"),
        report(
            &[
                "\"title\": \"a/one\", \"url\": \"https://github.com/a/one\", \
                 \"summary\": \"\\u30ed\\u30dc\", \"status\": \"dormant\", \
                 \"last_commit\": \"2024-01-02\", \"archived\": true, \"role\": \"library\"",
                "\"title\": \"b/two\"",
                "\"title\": \"c/three\", \"archived\": false",
                "\"title\": \"a/one-extra\"",
                "\"title\": \"A/One\"",
            ],
            "[]",
        ),
    )
    .expect("catalog.json is written");
    let dir = dir.to_string_lossy();

    let run = sandbox.run(&[
        "find-kboat-repo-lookup",
        &dir,
        "c/three",
        "nobody/none",
        "a/one",
        "a/one",
    ]);

    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "{\"title\":\"a/one\",\"url\":\"https://github.com/a/one\",\"summary\":\"ロボ\",\"status\":\"dormant\",\"last_commit\":\"2024-01-02\",\"archived\":true}\n\
         {\"title\":\"c/three\",\"url\":null,\"summary\":null,\"status\":null,\"last_commit\":null,\"archived\":false}\n"
    );
    assert_eq!(sandbox.mise_args(), None);
}

// `--help` prints how lookup is called, and a run with no directory is a usage error; a
// directory without the export's catalog.json fails with exit 1.
#[test]
fn lookup_needs_an_export() {
    let sandbox = Fixture::new("lookup-args");
    let missing = sandbox.root().join("missing");

    let help = sandbox.run(&["find-kboat-repo-lookup", "--help"]);
    let none = sandbox.run(&["find-kboat-repo-lookup"]);
    let unread = sandbox.run(&[
        "find-kboat-repo-lookup",
        &missing.to_string_lossy(),
        "a/one",
    ]);

    let help_out = String::from_utf8_lossy(&help.stdout);
    assert_eq!(help.status.code(), Some(0));
    let usage = "Usage: dotfiles-local find-kboat-repo-lookup <DIR> [TITLES]...\n";
    assert!(help_out.contains(usage), "{help_out}");
    let none_err = String::from_utf8_lossy(&none.stderr);
    assert_eq!(none.status.code(), Some(2));
    assert!(
        none_err.starts_with("error: the following required arguments were not provided:")
            && none_err.contains(usage),
        "{none_err}"
    );
    assert_eq!(unread.status.code(), Some(1));
    let unread_err = String::from_utf8_lossy(&unread.stderr);
    assert!(
        unread_err.starts_with(&format!(
            "cannot read {}: ",
            missing.join("catalog.json").display()
        )),
        "{unread_err}"
    );
    assert_eq!(unread.stdout, [] as [u8; 0]);
}

// An empty catalogue reads as one with no chunks to judge.
#[test]
fn export_lists_no_chunks_for_an_empty_catalogue() {
    let sandbox = Fixture::new("empty");

    let run = sandbox.run(&["find-kboat-repo-export-catalog"]);

    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let summary: serde_json::Value = serde_json::from_slice(&run.stdout).expect("stdout is JSON");
    assert_eq!(summary["notes"], 0);
    assert_eq!(summary["chunks"], serde_json::json!([]));
    let dir = exported_dir(&run.stdout);
    assert_eq!(
        fs::read_to_string(dir.join("compact.jsonl")).expect("compact.jsonl is written"),
        ""
    );
    assert!(!dir.join("chunk-aa").exists());
}

// A note whose list field is neither a list nor a string fails the run before any
// summary is printed, and its stderr names that note by its path and the catalog.json
// that holds the report, so the note can be found among many.
#[test]
fn export_names_the_note_and_the_stored_output_when_a_note_cannot_be_read() {
    let sandbox = Fixture::new("list-type");
    sandbox.kboat_note(
        &report(
            &[
                "\"title\": \"a/zero\"",
                "\"title\": \"a/one\", \"domain\": 5",
            ],
            "[]",
        ),
        0,
    );

    let run = sandbox.run(&["find-kboat-repo-export-catalog"]);

    assert_ne!(run.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&run.stdout), "");
    let dirs: Vec<_> = fs::read_dir(sandbox.root().join("tmp"))
        .expect("the temporary directory is readable")
        .map(|entry| entry.expect("an entry is read").path())
        .collect();
    assert_eq!(dirs.len(), 1, "the export creates one directory");
    let catalog = dirs[0].join("catalog.json");
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("Repos/s1.md"),
        "stderr does not name the failing note: {stderr}"
    );
    assert!(
        stderr.contains(&catalog.display().to_string()),
        "stderr does not name {}: {stderr}",
        catalog.display()
    );
}

// Without mise on PATH there is no way to run kboat-note, so the run says so and exits 1
// without a summary.
#[test]
fn export_fails_without_mise() {
    let sandbox = Fixture::new("no-mise");
    fs::remove_file(sandbox.root().join("bin").join("mise")).expect("the fake mise is removed");

    let run = sandbox.run(&["find-kboat-repo-export-catalog"]);

    assert_eq!(run.status.code(), Some(1));
    assert_eq!(run.stdout, [] as [u8; 0]);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(stderr.starts_with("cannot run mise: "), "{stderr}");
}
