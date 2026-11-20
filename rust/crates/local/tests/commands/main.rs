//! The `dotfiles-local` subcommands, run as a built binary.
//!
//! The tests are one crate, so that a helper no test uses any longer is reported as
//! dead code.

mod block_bsd_sed_inplace;
mod block_bsd_stat_format;
mod block_piped_git_push;
mod ccusage_popup;
mod dispatch;
mod mise_multi_upgrade;
mod pre_tool_use;
mod skills_update;
mod with_new_terminal;
