//! `skills:size-check`: whether the body of each skill git tracks, its frontmatter left
//! out, fits in what Claude Code re-attaches of an invoked skill after a compaction.
//!
//! After a compaction, Claude Code re-attaches each skill the session invoked, cut
//! short: its docs say to the first 5,000 tokens, and session transcripts show the cut
//! at 20,000 characters. A rule past the cut is gone from the re-attached copy. Length
//! is counted in UTF-16 code units, as JavaScript counts a string's, which is never
//! fewer than its characters. The re-attached copy opens with a line naming the skill's
//! directory, whose length depends on where the skill was loaded from, so the limit
//! leaves room for it.

use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::process::ExitCode;

use crate::git::Git;

/// The longest body that passes, in UTF-16 code units.
const LIMIT: usize = 20_000 - 200;

/// The skills checked where no pathspec names others; `glob` keeps `*` to one directory
/// level.
const SKILLS: &str = ":(glob).claude/skills/*/SKILL.md";

/// The status of a run that could not check the skills, as on arguments it does not
/// read.
const UNCHECKED: u8 = 2;

/// A run's arguments: `--verbose` first, and then a git pathspec.
#[derive(Debug)]
pub(crate) struct Request<'a> {
    verbosity: Verbosity,
    args: &'a [String],
}

/// Whether a passing skill is reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verbosity {
    /// Only what fails, so a commit's output stays quiet.
    Failures,
    /// Each passing skill's length too, for sizing a skill against the limit.
    Lengths,
}

/// A tracked `SKILL.md`, as the working tree holds it.
#[derive(Debug)]
struct Skill {
    contents: Vec<u8>,
}

/// Why a skill's body could not be measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unmeasured {
    /// Line 1 opens a frontmatter that no later line closes, which taken as all
    /// frontmatter would pass a skill of any length.
    Unclosed,
    /// The file is not UTF-8, so it has no length in UTF-16 code units.
    NotUtf8,
}

impl<'a> Request<'a> {
    pub(crate) fn new(args: &'a [String]) -> Self {
        match args.split_first() {
            Some((first, rest)) if first == "--verbose" => Request {
                verbosity: Verbosity::Lengths,
                args: rest,
            },
            _ => Request {
                verbosity: Verbosity::Failures,
                args,
            },
        }
    }

    /// Runs the check, and returns the code to exit with.
    pub(crate) fn run(&self) -> ExitCode {
        match self.check() {
            Ok(code) | Err(code) => code,
        }
    }

    /// Each stage hands the next what it needs, or ends the run with a code it has
    /// already reported.
    fn check(&self) -> Result<ExitCode, ExitCode> {
        let pathspec = self.pathspec()?;
        let listed = Self::list(pathspec)?;
        let mut failed = false;
        for path in listed
            .split(|&byte| byte == 0)
            .filter(|path| !path.is_empty())
        {
            failed |= !self.measure(Path::new(OsStr::from_bytes(path)));
        }
        Ok(ExitCode::from(u8::from(failed)))
    }

    /// The pathspec argument, or the default one. An argument past it, such as a
    /// `--verbose` written last, would otherwise be dropped and leave a silent pass.
    fn pathspec(&self) -> Result<&'a str, ExitCode> {
        match self.args {
            [] => Ok(SKILLS),
            [pathspec] if pathspec.is_empty() => Ok(SKILLS),
            [pathspec] => Ok(pathspec),
            [_, unread @ ..] => {
                eprintln!(
                    "size-check does not read {}; its arguments are [--verbose] [pathspec], in that order.",
                    unread.join(" ")
                );
                Err(ExitCode::from(UNCHECKED))
            }
        }
    }

    /// The tracked files `pathspec` names, NUL-separated: asked of git, so an ignored or
    /// vendored skill is left out and one staged for the commit being made is checked.
    fn list(pathspec: &str) -> Result<Vec<u8>, ExitCode> {
        let listed = Git::inheriting().run(&["ls-files", "-z", "--", pathspec]);
        if !listed.success() {
            eprint!("{}", listed.stderr());
            // git's own status, such as 128 outside a repository; one that never ran, or
            // that a signal stopped, has none, and says no more about the skills.
            return Err(ExitCode::from(
                listed
                    .status()
                    .and_then(|status| u8::try_from(status).ok())
                    .unwrap_or(UNCHECKED),
            ));
        }
        if listed.stdout_bytes().is_empty() {
            eprintln!("git tracks no skill at {pathspec} to check.");
            return Err(ExitCode::FAILURE);
        }
        Ok(listed.stdout_bytes().to_vec())
    }

    /// Reports the skill at `path` where it fails, or where the run is verbose, and
    /// returns whether it passed.
    fn measure(&self, path: &Path) -> bool {
        let shown = path.display();
        let skill = match Skill::read(path) {
            Ok(Some(skill)) => skill,
            Ok(None) => return true,
            Err(error) => {
                eprintln!("{shown}: could not be read: {error}");
                return false;
            }
        };
        match skill.length() {
            Ok(length) if length > LIMIT => {
                eprintln!(
                    "{shown}: {length} UTF-16 code units, past the {LIMIT} a compaction re-attaches; shorten it, or move into references/ only what it reads at one moment, keeping in the body what a step needs before the body points there"
                );
                false
            }
            Ok(length) => {
                if self.verbosity == Verbosity::Lengths {
                    println!("{shown}: {length} of {LIMIT} UTF-16 code units");
                }
                true
            }
            Err(unmeasured) => {
                eprintln!("{shown}: {unmeasured}");
                false
            }
        }
    }
}

impl Skill {
    /// The skill at `path`, or `None` where the working tree no longer holds a file
    /// there, as where it was deleted and the deletion is not yet staged.
    fn read(path: &Path) -> io::Result<Option<Self>> {
        if !path.is_file() {
            return Ok(None);
        }
        fs::read(path).map(|contents| Some(Skill { contents }))
    }

    /// The body's length in UTF-16 code units, as JavaScript counts a string's.
    fn length(&self) -> Result<usize, Unmeasured> {
        let body = std::str::from_utf8(self.body()?).map_err(|_| Unmeasured::NotUtf8)?;
        Ok(body.encode_utf16().count())
    }

    /// What follows the frontmatter: the lines after the first `---` that closes one
    /// opened by `---` on line 1, or the whole file where line 1 opens none.
    fn body(&self) -> Result<&[u8], Unmeasured> {
        let mut lines = self.contents.split_inclusive(|&byte| byte == b'\n');
        let Some(first) = lines.next().filter(|line| is_fence(line)) else {
            return Ok(&self.contents);
        };
        let mut end = first.len();
        for line in lines {
            end += line.len();
            if is_fence(line) {
                return Ok(&self.contents[end..]);
            }
        }
        Err(Unmeasured::Unclosed)
    }
}

impl fmt::Display for Unmeasured {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Unmeasured::Unclosed => "its frontmatter has no closing ---",
            Unmeasured::NotUtf8 => "it is not UTF-8, so its length cannot be counted",
        })
    }
}

/// Whether `line`, its line end included, is a frontmatter's `---`, a CRLF one included.
fn is_fence(line: &[u8]) -> bool {
    matches!(line, b"---" | b"---\n" | b"---\r" | b"---\r\n")
}
