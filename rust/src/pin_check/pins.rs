//! Reading the pins from the tracked files, and moving the differing ones in the working
//! tree.

use std::collections::HashMap;
use std::{fs, io};

use super::report::fail;
use super::{Differing, Marker, Move, Moves, Pin, Pins, Status, Text};

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
            let text = String::from_utf8_lossy(&fs::read(file)?).into_owned();
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

    /// Outside GitHub Actions the move lands in the working tree, for whoever ran it to
    /// commit.
    pub(super) fn move_in_working_tree(&self) -> io::Result<Status> {
        let tips: HashMap<String, &str> = self
            .0
            .iter()
            .map(|Move { pin, tip }| (pin.marker().to_string(), tip.as_str()))
            .collect();
        let mut moved = Moves::default();
        for file in self.files() {
            let text = String::from_utf8_lossy(&fs::read(file)?).into_owned();
            let (text, file_moved) = Text(&text)
                .rewrite(|found| tips.get(&found.to_string()).map(|tip| (*tip).to_owned()));
            fs::write(file, text)?;
            moved.0.extend(file_moved.0);
        }
        for line in moved.describe() {
            fail(&format!("moved the pin of {line}"));
        }
        Ok(Status::Differs)
    }
}
