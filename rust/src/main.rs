//! The shared tasks that other repositories run through mise, one subcommand each; the
//! wrappers under tasks/ pin the release that runs them.

mod git;
mod pin_check;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let status = match args.split_first() {
        Some((task, rest)) if task == "pin-check" => pin_check::Request::new(rest).run(),
        _ => {
            eprintln!("[fail] usage: dotfiles-tasks <task> [args...]; tasks: pin-check");
            pin_check::Status::Unknown
        }
    };
    ExitCode::from(status as u8)
}
