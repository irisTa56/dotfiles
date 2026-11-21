//! apm.lock.yaml, the lockfile APM keeps beside apm.yml.

use std::fmt;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::failure::Failure;

/// The lockfile's file name, relative to the checkout's root.
const FILE: &str = "apm.lock.yaml";

/// What apm.lock.yaml holds.
pub(crate) struct ApmLock {
    /// The whole document, for a message to show what was read.
    document: Value,
    /// The fields the commands read, each as the text the lockfile writes it in.
    fields: Fields,
}

/// The part of the lockfile the commands read.
///
/// It is read into text fields rather than through the untyped `document`, which takes
/// a plain scalar for a number where YAML 1.2 says it is one: a commit such as
/// `1234e567…`, which APM leaves unquoted, would come back as a float.
#[derive(Deserialize)]
struct Fields {
    dependencies: Option<Vec<LockedDependency>>,
}

/// One dependency the lockfile lists, with the fields the commands read.
#[derive(Deserialize, Serialize)]
pub(crate) struct LockedDependency {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    virtual_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolved_commit: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    deployed_files: Vec<String>,
}

impl ApmLock {
    /// Reads the apm.lock.yaml in `dir`.
    pub(crate) fn read(dir: &Path) -> Result<Self, Failure> {
        let text = fs::read_to_string(dir.join(FILE))
            .map_err(|error| Failure::new(format_args!("cannot read {FILE}: {error}"), 1))?;
        let parsing = |error: serde_saphyr::Error| {
            Failure::new(format_args!("cannot parse {FILE}: {error}"), 1)
        };
        let document = serde_saphyr::from_str(&text).map_err(parsing)?;
        let fields = serde_saphyr::from_str(&text).map_err(parsing)?;
        Ok(ApmLock { document, fields })
    }

    /// The dependencies the lockfile lists, or `None` where it has no such list.
    pub(crate) fn dependencies(&self) -> Option<&[LockedDependency]> {
        self.fields.dependencies.as_deref()
    }

    /// Every path a dependency lists among its deployed files, a dependency listing none
    /// and a lockfile with no dependencies list adding nothing.
    pub(crate) fn deployed_files(&self) -> impl Iterator<Item = &str> {
        self.dependencies()
            .unwrap_or_default()
            .iter()
            .flat_map(|dependency| &dependency.deployed_files)
            .map(String::as_str)
    }

    /// The gist dependency named `name`, or `None` where there is none.
    pub(crate) fn gist(&self, name: &str) -> Option<&LockedDependency> {
        self.dependencies()?.iter().find(|dependency| {
            dependency.name() == Some(name) && dependency.host() == Some("gist.github.com")
        })
    }
}

/// The lockfile as compact JSON, as a message naming what was read shows it.
impl fmt::Display for ApmLock {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.document.fmt(formatter)
    }
}

impl LockedDependency {
    pub(crate) fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub(crate) fn host(&self) -> Option<&str> {
        self.host.as_deref()
    }

    /// The repository it comes from, `<owner>/<repo>`, or `<owner>/<gist_id>` for a gist.
    pub(crate) fn repo_url(&self) -> Option<&str> {
        self.repo_url.as_deref()
    }

    /// The path to it within its repository, absent for a whole repository.
    pub(crate) fn virtual_path(&self) -> Option<&str> {
        self.virtual_path.as_deref()
    }

    /// The commit the lockfile pins it at.
    pub(crate) fn resolved_commit(&self) -> Option<&str> {
        self.resolved_commit.as_deref()
    }
}

/// The fields read, as compact JSON, as a message naming what was read shows them.
impl fmt::Display for LockedDependency {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        formatter.write_str(&json)
    }
}
