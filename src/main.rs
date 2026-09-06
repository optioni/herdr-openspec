use std::process::exit;

use herdr_openspec::ui::{self, StartError};
use herdr_openspec::{Invocation, parse, rejection_text};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

    match parse(&arg_refs) {
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
        // Stub arms: group 2 lands both variants so the crate compiles again; group 7
        // replaces these with the real open::run_from_env dispatch (design.md ->
        // Decision 10). Until then both take the same rejection path Invocation::Reject
        // already takes, so no test that runs before group 7 can observe them.
        Invocation::Open | Invocation::OpenTab => {
            eprint!("{}", rejection_text(None));
            exit(2);
        }
        Invocation::Reject(token) => {
            eprint!("{}", rejection_text(token.as_deref()));
            exit(2);
        }
    }
}
