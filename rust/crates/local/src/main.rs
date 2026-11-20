//! The commands for this machine alone, one subcommand each, which `mise run
//! local:install` installs into `~/.local/bin` from the main checkout.

mod ccusage_popup;
mod failure;
mod find_kboat_repo;
mod git;
mod git_sweep_branches;
mod git_sync_base;
mod mise_multi_upgrade;
mod pre_tool_use;
mod reply_language_stop;
mod skills_unmanaged;
mod skills_update;
mod with_new_terminal;

use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.split_first() {
        Some((command, _)) if command == "ask-email-in-request" => {
            pre_tool_use::ask_email_in_request::run()
        }
        Some((command, _)) if command == "block-bsd-sed-inplace" => {
            pre_tool_use::block_bsd_sed_inplace::run()
        }
        Some((command, _)) if command == "block-bsd-stat-format" => {
            pre_tool_use::block_bsd_stat_format::run()
        }
        Some((command, _)) if command == "block-piped-git-push" => {
            pre_tool_use::block_piped_git_push::run()
        }
        Some((command, rest)) if command == "ccusage-popup" => ccusage_popup::run(rest),
        Some((command, rest)) if command == "find-kboat-repo-export-catalog" => {
            find_kboat_repo::export_catalog::run(rest)
        }
        Some((command, rest)) if command == "find-kboat-repo-lookup" => {
            find_kboat_repo::lookup::run(rest)
        }
        Some((command, rest)) if command == "git-sweep-branches" => git_sweep_branches::run(rest),
        Some((command, rest)) if command == "git-sync-base" => git_sync_base::run(rest),
        Some((command, rest)) if command == "mise-multi-upgrade" => mise_multi_upgrade::run(rest),
        Some((command, _)) if command == "reply-language-stop" => reply_language_stop::run(),
        Some((command, rest)) if command == "skills-unmanaged" => skills_unmanaged::run(rest),
        Some((command, rest)) if command == "skills-update" => skills_update::run(rest),
        Some((command, rest)) if command == "with-new-terminal" => with_new_terminal::run(rest),
        _ => {
            // A name with no command ends as dotfiles-tasks ends on one, unable to tell.
            let _ = writeln!(
                io::stderr(),
                "[fail] usage: dotfiles-local <command> [args...]; commands: ask-email-in-request, block-bsd-sed-inplace, block-bsd-stat-format, block-piped-git-push, ccusage-popup, find-kboat-repo-export-catalog, find-kboat-repo-lookup, git-sweep-branches, git-sync-base, mise-multi-upgrade, reply-language-stop, skills-unmanaged, skills-update, with-new-terminal"
            );
            ExitCode::from(2)
        }
    }
}
