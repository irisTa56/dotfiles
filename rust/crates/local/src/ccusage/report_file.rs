//! The report as a file Quick Look shows.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::program::Program;
use dotfiles_shared::Failure;

/// A report written as HTML in a directory of its own under the temporary one, both
/// removed when this is dropped.
pub(super) struct ReportFile {
    dir: PathBuf,
}

impl ReportFile {
    pub(super) fn create(report: &str) -> Result<Self, Failure> {
        Self::write(report)
            .map_err(|error| Failure::new(format_args!("cannot write the report: {error}"), 1))
    }

    fn write(report: &str) -> io::Result<Self> {
        let dir = Self::new_dir()?;
        let report_file = ReportFile { dir };
        let body = report
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        fs::write(
            report_file.html(),
            format!(
                "<meta charset=\"utf-8\"><body style=\"margin:0;background:#1e1e1e\">\n\
                 <pre style=\"font:13px ui-monospace,Menlo,monospace;color:#eee;padding:16px\">\n\
                 {body}\n\
                 </pre>\n"
            ),
        )?;
        Ok(report_file)
    }

    /// A directory no other run has, since `create_dir` fails on a name already taken
    /// rather than sharing it.
    fn new_dir() -> io::Result<PathBuf> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.subsec_nanos());
        let base = format!("ccusage.{}.{nanos}", std::process::id());
        let mut attempt = 0;
        loop {
            let dir = std::env::temp_dir().join(format!("{base}.{attempt}"));
            match fs::create_dir(&dir) {
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => attempt += 1,
                result => return result.map(|()| dir),
            }
        }
    }

    /// Shows the report in a Quick Look panel, until it is dismissed.
    pub(super) fn show(&self) -> Result<(), Failure> {
        // Quick Look renders HTML via WebKit, guaranteeing a monospace font and exact
        // column alignment that a proportional AppleScript dialog would mangle.
        // qlmanage -p blocks until the preview panel is dismissed. It is noisy on stderr
        // even on success, so its output is dropped, but an outright render failure still
        // surfaces instead of doing nothing.
        let status = Program::new("qlmanage")
            .arg("-p")
            .arg(self.html())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(Failure::new(format_args!("Quick Look preview failed"), 1))
        }
    }

    fn html(&self) -> PathBuf {
        self.dir.join("report.html")
    }
}

impl Drop for ReportFile {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}
