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
        Invocation::Reject(token) => {
            eprint!("{}", rejection_text(token.as_deref()));
            exit(2);
        }
    }
}
