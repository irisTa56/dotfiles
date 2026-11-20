//! The `dotfiles-local` subcommands, run as a built binary.
//!
//! The tests are one crate, so that a helper no test uses any longer is reported as
//! dead code.

mod ask_email_in_request;
mod block_bsd_sed_inplace;
mod block_bsd_stat_format;
mod block_piped_git_push;
mod ccusage_popup;
mod dispatch;
mod find_kboat_repo;
mod fnox_add_secret;
mod git_sweep_branches;
mod git_sync_base;
mod mise_multi_upgrade;
mod pre_tool_use;
mod reply_language_stop;
mod skills_unmanaged;
mod skills_update;
mod with_new_terminal;
