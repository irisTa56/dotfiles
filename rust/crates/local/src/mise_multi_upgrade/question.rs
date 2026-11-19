//! The question asked of each place before anything is upgraded.

use std::io::{self, BufRead, Write};

/// What a question was answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Answer {
    Yes,
    No,
    /// Input ended, which answers no to every question left.
    End,
}

/// A question, put on stdout and answered by a line of stdin.
pub(super) struct Question {
    prompt: String,
}

impl Question {
    pub(super) fn new(prompt: String) -> Self {
        Question { prompt }
    }

    /// Asks until an answer reads as yes or no, an empty one as no, or input ends.
    pub(super) fn ask(&self) -> io::Result<Answer> {
        let mut stdout = io::stdout();
        let mut stdin = io::stdin().lock();
        loop {
            write!(stdout, "{}", self.prompt)?;
            stdout.flush()?;
            let mut line = Vec::new();
            if stdin.read_until(b'\n', &mut line)? == 0 {
                return Ok(Answer::End);
            }
            match String::from_utf8_lossy(&line)
                .trim()
                .to_lowercase()
                .as_str()
            {
                "y" | "yes" => return Ok(Answer::Yes),
                "" | "n" | "no" => return Ok(Answer::No),
                _ => {}
            }
        }
    }
}
