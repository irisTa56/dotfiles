//! The shared tasks that other repositories run through mise, one subcommand each; the
//! wrappers under tasks/ pin the release that runs them.

mod git;
mod pin_check;
mod size_check;
mod stream;

use std::process::ExitCode;

use stream::Stream;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.split_first() {
        Some((task, rest)) if task == "pin-check" => {
            ExitCode::from(pin_check::Request::new(rest).run() as u8)
        }
        Some((task, rest)) if task == "size-check" => size_check::Request::new(rest).run(),
        _ => {
            Stream::Stderr.line(format_args!(
                "[fail] usage: dotfiles-tasks <task> [args...]; tasks: pin-check, size-check"
            ));
            ExitCode::from(pin_check::Status::Unknown as u8)
        }
    }
}
