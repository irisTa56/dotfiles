//! `mise-multi-upgrade`: mise tools upgraded directory by directory, asking for each
//! first.

mod command;
mod group;
mod mise;
mod options;
mod parallel;
mod place;
mod question;
mod release_dates;
mod report;
mod row;
mod survey;
mod table;
mod temp_dirs;
mod tracked;
mod upgrades;

pub(crate) use command::run;
