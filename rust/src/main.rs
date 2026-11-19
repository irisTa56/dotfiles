//! The shared tasks that other repositories run through mise, one subcommand each; the
//! wrappers under tasks/ pin the release that runs them.

mod git;
mod link_check_staged;
mod secrets_push_scan;
mod skills_size_check;
mod stream;
mod upstream_pin_check;

use std::process::ExitCode;

use stream::Stream;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.split_first() {
        Some((task, _)) if task == "link-check-staged" => link_check_staged::Lychee::check_staged(),
        Some((task, rest)) if task == "secrets-push-scan" => secrets_push_scan::Push::scan(rest),
        Some((task, rest)) if task == "upstream-pin-check" => {
            ExitCode::from(upstream_pin_check::Request::new(rest).run() as u8)
        }
        Some((task, rest)) if task == "skills-size-check" => {
            skills_size_check::Request::new(rest).run()
        }
        _ => {
            Stream::Stderr.line(format_args!(
                "[fail] usage: dotfiles-tasks <task> [args...]; tasks: link-check-staged, secrets-push-scan, skills-size-check, upstream-pin-check"
            ));
            ExitCode::from(upstream_pin_check::Status::Unknown as u8)
        }
    }
}
