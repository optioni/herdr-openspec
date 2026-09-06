use std::process::exit;

use herdr_openspec::open::{self, Placement};
use herdr_openspec::ui::{self, StartError};
use herdr_openspec::{Invocation, parse, rejection_text};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

    let invocation = parse(&arg_refs);

    match &invocation {
        Invocation::Ui => match ui::run() {
            Ok(()) => exit(0),
            Err(StartError::NotATerminal) => {
                eprintln!("herdr-openspec: not a terminal");
                exit(3);
            }
            Err(other) => {
                eprintln!("{other}");
                exit(1);
            }
        },
        Invocation::Open | Invocation::OpenTab => {
            // Which placement is `open::placement_for`'s decision, never a literal
            // mapping written here (design.md -> Decision 10).
            let placement = open::placement_for(&invocation)
                .expect("Open and OpenTab always map to a placement");
            run_open(placement);
        }
        Invocation::Reject(token) => {
            eprint!("{}", rejection_text(token.as_deref()));
            exit(2);
        }
    }
}

/// `open`/`open-tab`'s whole dispatch: `open::run_from_env`, then `open::report_output`'s
/// lines to stderr and its exit status. No branch of `main`'s own — both decisions are
/// pure functions in `src/open.rs` (design.md -> Decision 10).
fn run_open(placement: Placement) -> ! {
    let report = open::run_from_env(placement);
    let (lines, status) = open::report_output(&report);
    for line in lines {
        eprintln!("{line}");
    }
    exit(status);
}
