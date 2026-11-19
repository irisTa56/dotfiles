//! The shared tasks that other repositories run through mise, one subcommand each; the
//! wrappers under tasks/ pin the release that runs them.

mod git;
mod link_check_staged;
mod skills_size_check;
mod stream;
mod upstream_pin_check;

use std::process::ExitCode;

use stream::Stream;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.split_first() {
        Some((task, _)) if task == "link-check-staged" => link_check_staged::Lychee::check_staged(),
        // A wrapper runs the release it pins, and the wrappers pinning 0.4.0 called these
        // checks as pin-check and size-check until they moved to the new names; no wrapper
        // calls those names now, so a later release can drop them.
        Some((task, rest)) if task == "upstream-pin-check" || task == "pin-check" => {
            ExitCode::from(upstream_pin_check::Request::new(rest).run() as u8)
        }
        Some((task, rest)) if task == "skills-size-check" || task == "size-check" => {
            skills_size_check::Request::new(rest).run()
        }
        _ => {
            Stream::Stderr.line(format_args!(
                "[fail] usage: dotfiles-tasks <task> [args...]; tasks: link-check-staged, skills-size-check, upstream-pin-check"
            ));
            ExitCode::from(upstream_pin_check::Status::Unknown as u8)
        }
    }
}
