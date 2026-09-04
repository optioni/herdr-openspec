//! The subprocess seam: `cli` is the only module in this crate permitted to
//! spawn a process. See `SPEC.md` -> Architecture ("The subprocess seam") for
//! the contract this module implements, and
//! `openspec/changes/subprocess-seam/design.md` for the full design.
//!
//! Not to be confused with `tests/cli.rs`, which is a different file with the
//! same base name and an unrelated meaning: it exercises this crate's own
//! binary's command-line interface (`herdr-openspec ui`, `herdr-openspec
//! wat`, and so on) by spawning `env!("CARGO_BIN_EXE_herdr-openspec")`. This
//! module is the seam through which the crate calls the external `openspec`
//! and `herdr` programs.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Everything that can go wrong running a program through this seam. Both
/// variants carry the **argument vector** alongside the program name:
/// `changes-from-cli` drives four distinct invocations through one
/// `RealOpenspecCli` and `agent-launch` four more through one
/// `RealHerdrCli`, and without the arguments all eight failures would render
/// identically, leaving a caller unable to say which call failed.
///
/// There is deliberately no `Utf8` variant. `SPEC.md` -> Degraded states
/// records that the OpenSpec CLI itself decodes a tasks file lossily and
/// still reports a count — a precedent for the choice rather than a rule
/// that governs this case. The substantive reason stands on its own: a
/// third variant would force every caller to handle a case that, for JSON
/// output, means the payload was already unusable and the parse will fail
/// anyway. Invalid UTF-8 on either stream is therefore decoded lossily
/// rather than becoming an error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    /// The program could not be started at all: absent, not executable, or
    /// an OS error.
    NotStarted {
        program: String,
        args: Vec<String>,
        reason: String,
    },
    /// It started, ran, and exited non-zero.
    Failed {
        program: String,
        args: Vec<String>,
        code: Option<i32>,
        stderr: String,
    },
}

/// The outcome of actually starting a program: either it ran to completion
/// (carrying its exit success, stdout, and stderr, all as raw bytes — this
/// helper decides nothing about them), or it could not be started at all
/// (carrying the operating system's error text).
enum RunOutcome {
    Completed {
        success: bool,
        code: Option<i32>,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    },
    NotStarted { reason: String },
}

/// The crate's single spawn site. Nothing else in the crate may call
/// `Command::new` after this — see `NOSPAWN-GREP` in
/// `openspec/changes/subprocess-seam/design.md` -> Test Strategy, which
/// checks exactly that. Attaches an empty stdin, so a program that reads
/// stdin fails fast instead of blocking a pane that never types. Adds no
/// argument of its own, never sets `current_dir`, never mutates the
/// environment: everything a caller can observe comes from the program and
/// the arguments it was given.
fn spawn(program: &Path, args: &[&str]) -> RunOutcome {
    match Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
    {
        Ok(output) => RunOutcome::Completed {
            success: output.status.success(),
            code: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
        },
        Err(err) => RunOutcome::NotStarted {
            reason: err.to_string(),
        },
    }
}

/// The shared mapping from a completed run to the traits' `Result`, so
/// `OpenspecCli` and `HerdrCli`'s real implementations need not duplicate
/// it. Success maps to `Ok` of stdout decoded lossily and returned
/// **verbatim** — not trimmed, not split, not parsed; trimming is a
/// caller's decision and this module makes none. A non-zero exit maps to
/// `Failed`, carrying the argument vector, the exit code, and stderr
/// decoded lossily and **also verbatim** — trimming stderr would likewise
/// be a decision. An unstartable program maps to `NotStarted`, carrying the
/// argument vector and the operating system's error text; it never panics,
/// because an absent `openspec` and an unreachable `herdr` are both
/// documented degraded states.
fn run_and_map(program: &Path, args: &[&str]) -> Result<String, CliError> {
    let program_name = program.display().to_string();
    let arg_vec: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    match spawn(program, args) {
        RunOutcome::Completed {
            success: true,
            stdout,
            ..
        } => Ok(String::from_utf8_lossy(&stdout).into_owned()),
        RunOutcome::Completed {
            success: false,
            code,
            stderr,
            ..
        } => Err(CliError::Failed {
            program: program_name,
            args: arg_vec,
            code,
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
        }),
        RunOutcome::NotStarted { reason } => Err(CliError::NotStarted {
            program: program_name,
            args: arg_vec,
            reason,
        }),
    }
}

/// The crate's trait for the `openspec` program. `Send + Sync` because
/// `live-refresh` runs CLI calls on a worker thread and `agent-polling`
/// polls on another; a trait that could not cross a thread boundary would
/// have to be redesigned by the first change that uses it.
pub trait OpenspecCli: Send + Sync {
    fn run(&self, args: &[&str]) -> Result<String, CliError>;
}

/// The crate's trait for the `herdr` program. Same contract as
/// [`OpenspecCli`]; kept as a separate trait rather than a type parameter so
/// a caller holding both is unambiguous about which program it is calling.
pub trait HerdrCli: Send + Sync {
    fn run(&self, args: &[&str]) -> Result<String, CliError>;
}

/// The one real implementation of [`OpenspecCli`]. Constructed with the
/// program path — the value `resolve::openspec_bin`'s result carries,
/// never its canonicalized target, since that is the stable,
/// upgrade-surviving name a caller should keep spawning. Does nothing but
/// delegate to the crate's one spawn helper: no added argument, no
/// `current_dir`, no environment mutation, no retry, no timeout, no
/// caching, no inspection of the output.
pub struct RealOpenspecCli {
    program: PathBuf,
}

impl RealOpenspecCli {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
        }
    }

    pub fn program(&self) -> &Path {
        &self.program
    }
}

impl OpenspecCli for RealOpenspecCli {
    fn run(&self, args: &[&str]) -> Result<String, CliError> {
        run_and_map(&self.program, args)
    }
}

/// The one real implementation of [`HerdrCli`]. Unlike `openspec`, this
/// crate builds no resolution chain for `herdr`: failing to start it is
/// already the documented "Herdr socket unreachable" degraded state, so
/// [`Default`] simply names the bare program `herdr`, resolved by the
/// operating system through `PATH`.
pub struct RealHerdrCli {
    program: PathBuf,
}

impl RealHerdrCli {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
        }
    }

    pub fn program(&self) -> &Path {
        &self.program
    }
}

impl Default for RealHerdrCli {
    fn default() -> Self {
        Self::new("herdr")
    }
}

impl HerdrCli for RealHerdrCli {
    fn run(&self, args: &[&str]) -> Result<String, CliError> {
        run_and_map(&self.program, args)
    }
}

#[cfg(test)]
mod tests {
    use crate::testutil::{ScratchDir, write_with_mode};

    fn script(dir: &crate::testutil::ScratchDir, name: &str, body: &str) -> std::path::PathBuf {
        let path = dir.path().join(name);
        write_with_mode(&path, format!("#!/bin/sh\n{body}").as_bytes(), 0o755);
        path
    }

    #[test]
    fn stdout_is_returned_verbatim_on_success() {
        let scratch = ScratchDir::new();
        let prog = script(&scratch, "prog", "printf 'line one\\nline two\\n'\n");
        let cli = super::RealOpenspecCli::new(prog);
        let result = super::OpenspecCli::run(&cli, &[]);
        assert_eq!(result, Ok("line one\nline two\n".to_string()));
    }

    #[test]
    fn stderr_never_reaches_the_success_value() {
        let scratch = ScratchDir::new();
        let prog = script(
            &scratch,
            "prog",
            "printf 'payload'; printf 'warning: unrelated shell-plugin noise' >&2\n",
        );
        let cli = super::RealOpenspecCli::new(prog);
        let result = super::OpenspecCli::run(&cli, &[]).expect("should succeed");
        assert_eq!(result, "payload");
        assert!(!result.contains("warning"));
        assert!(!result.contains("noise"));
    }

    #[test]
    fn a_non_zero_exit_is_a_failure_carrying_the_code_and_stderr() {
        let scratch = ScratchDir::new();
        let prog = script(
            &scratch,
            "prog",
            "printf 'partial'; printf 'boom\\n' >&2; exit 3\n",
        );
        let cli = super::RealOpenspecCli::new(prog);
        let result = super::OpenspecCli::run(&cli, &["list", "--json"]);
        match result {
            Err(super::CliError::Failed {
                code,
                stderr,
                args,
                ..
            }) => {
                assert_eq!(code, Some(3));
                assert_eq!(stderr, "boom\n");
                assert_eq!(args, vec!["list".to_string(), "--json".to_string()]);
                assert!(!stderr.contains("partial"));
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn a_program_that_cannot_be_started_is_a_failure_not_a_panic() {
        let scratch = ScratchDir::new();
        let missing = scratch.path().join("does-not-exist");
        assert!(!missing.exists());
        let cli = super::RealOpenspecCli::new(missing.clone());
        let result = super::OpenspecCli::run(&cli, &["list", "--json"]);
        match result {
            Err(super::CliError::NotStarted { program, args, .. }) => {
                assert_eq!(program, missing.display().to_string());
                assert_eq!(args, vec!["list".to_string(), "--json".to_string()]);
            }
            other => panic!("expected NotStarted, got {other:?}"),
        }
        assert!(!missing.exists());
    }

    #[test]
    fn invalid_utf8_on_stdout_is_decoded_lossily_rather_than_failing() {
        let scratch = ScratchDir::new();
        let prog = script(&scratch, "prog", "printf '\\141\\377\\142'\n");
        let cli = super::RealOpenspecCli::new(prog);
        let result = super::OpenspecCli::run(&cli, &[]);
        assert_eq!(result, Ok("a\u{FFFD}b".to_string()));
    }

    #[test]
    fn empty_stdout_with_a_zero_exit_is_success_not_a_failure() {
        let scratch = ScratchDir::new();
        let prog = script(&scratch, "prog", "exit 0\n");
        let cli = super::RealOpenspecCli::new(prog);
        let result = super::OpenspecCli::run(&cli, &[]);
        assert_eq!(result, Ok(String::new()));
    }

    #[test]
    fn arguments_reach_the_program_in_order_and_unaltered() {
        let scratch = ScratchDir::new();
        let prog = script(&scratch, "prog", "for a in \"$@\"; do printf '%s\\n' \"$a\"; done\n");
        let cli = super::RealOpenspecCli::new(prog);
        let result = super::OpenspecCli::run(&cli, &["list", "--json", "a b", "--", "-x"])
            .expect("should succeed");
        assert_eq!(result, "list\n--json\na b\n--\n-x\n");
    }

    #[test]
    fn a_trait_object_crosses_a_thread_boundary() {
        let scratch = ScratchDir::new();
        let prog = script(&scratch, "prog", "printf 'ok'\n");
        let openspec: std::sync::Arc<dyn super::OpenspecCli> =
            std::sync::Arc::new(super::RealOpenspecCli::new(prog.clone()));
        let inline = openspec.run(&[]);
        let os = openspec.clone();
        let handle = std::thread::spawn(move || os.run(&[]));
        let threaded = handle.join().expect("thread panicked");
        assert_eq!(inline, threaded);
        assert_eq!(inline, Ok("ok".to_string()));

        let herdr: std::sync::Arc<dyn super::HerdrCli> =
            std::sync::Arc::new(super::RealHerdrCli::new(prog));
        let inline = herdr.run(&[]);
        let h = herdr.clone();
        let handle = std::thread::spawn(move || h.run(&[]));
        let threaded = handle.join().expect("thread panicked");
        assert_eq!(inline, threaded);
    }
}
