//! The commands for this machine alone, one subcommand each, which `mise run
//! local:install` installs into `~/.local/bin` from the main checkout.

mod apm_lock;
mod ccusage;
mod dotfiles_checkout;
mod dotfiles_setup;
mod failure;
mod find_kboat_repo;
mod fnox_add_secret;
mod git;
mod git_sweep_branches;
mod git_sync_base;
mod mise_multi_upgrade;
mod pre_tool_use;
mod reply_language_stop;
mod skills_push;
mod skills_unmanaged;
mod skills_update;
mod with_new_terminal;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// The commands for this machine alone, installed from the main checkout.
#[derive(Debug, Parser)]
#[command(name = "dotfiles-local", disable_help_subcommand = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// One subcommand per command, each described by the doc comment of its request.
#[derive(Debug, Subcommand)]
enum Command {
    AskEmailInRequest(pre_tool_use::ask_email_in_request::Request),
    Ccusage(ccusage::Request),
    DotfilesSetup(dotfiles_setup::Request),
    FindKboatRepoExportCatalog(find_kboat_repo::export_catalog::Request),
    FindKboatRepoLookup(find_kboat_repo::lookup::Request),
    FnoxAddSecret(fnox_add_secret::Request),
    GitSweepBranches(git_sweep_branches::Request),
    GitSyncBase(git_sync_base::Request),
    MiseMultiUpgrade(mise_multi_upgrade::Request),
    ReplyLanguageStop(reply_language_stop::Request),
    SkillsPush(skills_push::Request),
    SkillsUnmanaged(skills_unmanaged::Request),
    SkillsUpdate(skills_update::Request),
    WithNewTerminal(with_new_terminal::Request),
}

fn main() -> ExitCode {
    // A usage error exits 2 on stderr, and --help 0 on stdout, as clap's `Error::exit`
    // does; a write it cannot make there, as to a closed pipe, is dropped.
    let outcome = match Cli::parse().command {
        Command::AskEmailInRequest(request) => request.run(),
        Command::Ccusage(request) => request.run(),
        Command::DotfilesSetup(request) => request.run(),
        Command::FindKboatRepoExportCatalog(request) => request.run(),
        Command::FindKboatRepoLookup(request) => request.run(),
        Command::FnoxAddSecret(request) => request.run(),
        Command::GitSweepBranches(request) => request.run(),
        Command::GitSyncBase(request) => request.run(),
        Command::MiseMultiUpgrade(request) => request.run(),
        Command::ReplyLanguageStop(request) => request.run(),
        Command::SkillsPush(request) => request.run(),
        Command::SkillsUnmanaged(request) => request.run(),
        Command::SkillsUpdate(request) => request.run(),
        Command::WithNewTerminal(request) => request.run(),
    };
    // Each command's failure is reported here, once, whatever stage it came from.
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => failure.report(),
    }
}
