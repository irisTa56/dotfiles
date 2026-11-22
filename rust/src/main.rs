//! The shared tasks that other repositories run through mise, one subcommand each; the
//! wrappers under tasks/ pin the release that runs them.

mod git;
mod link_check_staged;
mod secrets_push_scan;
mod skills_size_check;
mod stream;
mod upstream_pin_check;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// The shared tasks that other repositories run through mise; what each does and how it
/// exits is in <https://github.com/irisTa56/dotfiles/blob/main/tasks/README.md>.
#[derive(Debug, Parser)]
#[command(name = "dotfiles-tasks", disable_help_subcommand = true)]
struct Cli {
    #[command(subcommand)]
    task: Task,
}

/// One subcommand per task, named as the task with its `:` turned into `-`.
#[derive(Debug, Subcommand)]
enum Task {
    /// Check the links of the files the commit stages with lychee.
    // The task is being renamed `links:commit-check`, and its wrapper under that name
    // runs this subcommand by the matching name from a release published before it.
    #[command(alias = "links-commit-check")]
    LinkCheckStaged,
    /// Scan the commits a push is about to send for secrets with trufflehog, as a
    /// pre-push hook.
    SecretsPushScan(secrets_push_scan::Hook),
    /// Check that the body of each skill git tracks fits in what a compaction
    /// re-attaches.
    SkillsSizeCheck(skills_size_check::Request),
    /// Check whether the upstream paths this repository pins have changed on their
    /// default branches.
    UpstreamPinCheck(upstream_pin_check::Request),
}

fn main() -> ExitCode {
    // A usage error exits 2 on stderr, and --help 0 on stdout, as clap's `Error::exit`
    // does; a write it cannot make there, as to a closed pipe, is dropped.
    match Cli::parse().task {
        Task::LinkCheckStaged => link_check_staged::Lychee::check_staged(),
        Task::SecretsPushScan(hook) => secrets_push_scan::Push::scan(&hook),
        Task::SkillsSizeCheck(request) => request.run(),
        Task::UpstreamPinCheck(request) => ExitCode::from(request.run() as u8),
    }
}
