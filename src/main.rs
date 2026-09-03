use std::io::{Read, Write};
use std::process::exit;

use herdr_openspec::{Invocation, banner, parse, rejection_text};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

    match parse(&arg_refs) {
        Invocation::Ui => {
            print!("{}", banner());
            std::io::stdout().flush().ok();
            // Hold the pane open: block until stdin reaches EOF.
            let mut discard = Vec::new();
            let _ = std::io::stdin().read_to_end(&mut discard);
            exit(0);
        }
        Invocation::Reject(token) => {
            eprint!("{}", rejection_text(token.as_deref()));
            exit(2);
        }
    }
}
