//! Reading the pins from the tracked files, checking each, and moving the differing ones.

use std::collections::HashMap;
use std::{env, fs, io};

use dotfiles_shared::Failure;

use super::{
    Checked, Differing, FailLine, Marker, Move, Moves, Pin, Pins, Proposal, Scratch, Status, Text,
};

impl Pin {
    fn marker(&self) -> Marker<'_> {
        Marker {
            url: &self.url,
            path: &self.path,
            commit: &self.commit,
        }
    }
}

impl Pins {
    /// The pins `files` spell, read from the working tree.
    pub(super) fn read(files: &[&str]) -> io::Result<Self> {
        let mut pins: Vec<Pin> = Vec::new();
        let mut seen: HashMap<(String, String, String), usize> = HashMap::new();
        for &file in files {
            let text = fs::read(file)?;
            for (_, found) in Text(&text).markers() {
                let key = (
                    found.url.to_owned(),
                    found.path.to_owned(),
                    found.commit.to_owned(),
                );
                let index = *seen.entry(key).or_insert_with_key(|(url, path, commit)| {
                    pins.push(Pin {
                        url: url.clone(),
                        path: path.clone(),
                        commit: commit.clone(),
                        files: Vec::new(),
                    });
                    pins.len() - 1
                });
                let pin = &mut pins[index];
                if !pin.files.iter().any(|known| known == file) {
                    pin.files.push(file.to_owned());
                }
            }
        }
        Ok(Pins(pins))
    }

    /// Checks every pin in `scratch`, and returns the worst outcome with the pins that
    /// differ, each with the tip it would move to.
    pub(super) fn check<'p>(&'p self, scratch: &Scratch) -> (Status, Differing<'p>) {
        let mut status = Status::Ok;
        let mut differing = Differing(Vec::new());
        for pin in &self.0 {
            let checked = scratch.check(pin);
            status = status.max(checked.status());
            if let Checked::Differs(tip) = checked {
                differing.0.push(Move { pin, tip });
            }
        }
        (status, differing)
    }
}

impl Differing<'_> {
    /// Each file that spells one of the pins, once, in the order first met.
    pub(super) fn files(&self) -> Vec<&str> {
        let mut files: Vec<&str> = Vec::new();
        for file in self.0.iter().flat_map(|m| &m.pin.files) {
            if !files.contains(&file.as_str()) {
                files.push(file);
            }
        }
        files
    }

    /// Moves the pins: under GitHub Actions on origin, proposing the move as `proposal`
    /// says, and otherwise in the working tree.
    pub(super) fn move_pins(
        &self,
        scratch: &Scratch,
        proposal: Proposal,
    ) -> Result<Status, Failure> {
        if env::var("GITHUB_ACTIONS").is_ok_and(|v| v == "true") {
            scratch.move_on_origin(self, proposal)
        } else {
            self.move_in_working_tree()
                .map_err(|error| Status::stopped_on(&error))
        }
    }

    /// Outside GitHub Actions the move lands in the working tree, for whoever ran it to
    /// commit.
    fn move_in_working_tree(&self) -> io::Result<Status> {
        let tips: HashMap<String, &str> = self
            .0
            .iter()
            .map(|Move { pin, tip }| (pin.marker().to_string(), tip.as_str()))
            .collect();
        let mut moved = Moves::default();
        for file in self.files() {
            let text = fs::read(file)?;
            let (text, file_moved) = Text(&text)
                .rewrite(|found| tips.get(&found.to_string()).map(|tip| (*tip).to_owned()));
            fs::write(file, text)?;
            moved.0.extend(file_moved.0);
        }
        for line in moved.describe() {
            FailLine(&format!("moved the pin of {line}")).report();
        }
        Ok(Status::Differs)
    }
}
