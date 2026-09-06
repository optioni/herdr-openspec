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
    NotStarted {
        reason: String,
    },
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

/// The pure decision behind the `npm prefix -g` probe: given the run's exit
/// success and its stdout bytes, decide the prefix. Taking only these two
/// parameters is the *structural* proof that stderr cannot influence the
/// result — there is no parameter to read it from. Decodes lossily (as
/// [`run_and_map`] does), then trims the **whole** output rather than
/// splitting into lines — splitting would be parsing, which this module
/// does none of. Returns nothing on failure, on empty output, or on
/// whitespace-only output; a non-zero exit beats even plausible-looking
/// output, so a failing run's stdout is never trusted.
fn npm_prefix_from(success: bool, stdout: &[u8]) -> Option<PathBuf> {
    if !success {
        return None;
    }
    let decoded = String::from_utf8_lossy(stdout);
    let trimmed = decoded.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(PathBuf::from(trimmed))
}

/// The spawning probe: start `program` with exactly the arguments `prefix`
/// and `-g`, and hand the outcome to [`npm_prefix_from`]. Parameterizing the
/// program (rather than hardcoding `npm`) is what lets the end-to-end
/// scenario in group 7 drive a real spawn on a machine with no `npm`,
/// without touching `PATH` — `std::env::set_var` is `unsafe` in edition
/// 2024 and races parallel tests, which `AGENTS.md` forbids outright.
pub fn npm_prefix_via(program: &Path) -> Option<PathBuf> {
    match spawn(program, &["prefix", "-g"]) {
        RunOutcome::Completed {
            success, stdout, ..
        } => npm_prefix_from(success, &stdout),
        RunOutcome::NotStarted { .. } => None,
    }
}

/// The one binding to the real `npm` program, following
/// `config::env_lookup`'s shape: a one-line binding to the real world, with
/// nothing left to assert beyond delegation. This is the production
/// binding `resolve::openspec_bin_from_env` passes as its fourth-step hook.
pub fn npm_prefix() -> Option<PathBuf> {
    npm_prefix_via(Path::new("npm"))
}

/// The bare program name `herdr`, resolved by the operating system through `PATH` — the
/// one place that literal is written as a program name. This crate builds no resolution
/// chain for `herdr`, deliberately: failing to start it is already the documented
/// "Herdr socket unreachable" degraded state.
pub const HERDR_PROGRAM: &str = "herdr";

/// Construct the real `HerdrCli` over `program`, following `npm_prefix_via`'s shape
/// precisely: a parameterised spawner, so a scenario can drive a real spawn against a
/// scratch `#!/bin/sh` program without touching `PATH`. `ui::run` is the only caller
/// that passes [`HERDR_PROGRAM`]; every other caller — the poller's own tests, the
/// outer-loop acceptance test — passes a scratch path instead.
///
/// Group 1 stub: the program path is fixed rather than read from `program`, so group
/// 5's tests are red on behaviour rather than on a stub that already answers correctly.
pub fn agent_cli_via(_program: &Path) -> std::sync::Arc<dyn HerdrCli> {
    std::sync::Arc::new(RealHerdrCli::new("herdr"))
}

/// Turn a `resolve::BinResolution` into the real `OpenspecCli` the
/// `live-refresh` worker should use — `None` when no usable binary was
/// found, which is exactly `refresh::start`'s no-binary case: no thread, no
/// process, the inert `Refresher`. Pure and testable, unlike its `_from_env`
/// sibling below.
pub fn worker_cli(
    resolution: crate::resolve::BinResolution,
) -> Option<std::sync::Arc<dyn OpenspecCli>> {
    resolution.found.map(|found| {
        std::sync::Arc::new(RealOpenspecCli::new(found.path)) as std::sync::Arc<dyn OpenspecCli>
    })
}

/// The production binding: resolve the binary from the environment via
/// `resolve::openspec_bin_from_env`, then [`worker_cli`]. `ui::run` calls
/// this rather than constructing a `RealOpenspecCli` itself — the type
/// appears only in THIS function's signature, never in `ui::run`'s own
/// source text, which is the mechanical reason `NOCLI-SHELL` ("no file
/// under `src/ui/` names `OpenspecCli`") stays satisfied while the live
/// tier's worker is still wired to a real binary. See
/// `openspec/changes/live-refresh/design.md` -> Decisions 1.
pub fn worker_cli_from_env(
    config: &crate::config::Config,
) -> Option<std::sync::Arc<dyn OpenspecCli>> {
    worker_cli(crate::resolve::openspec_bin_from_env(config))
}

/// Which program a fake invocation addressed. Recorded and keyed alongside
/// the argument vector: one type implements both traits below, so a vector
/// alone would let a caller that reached for the wrong handle be answered
/// out of the other program's registration — precisely the "a caller's test
/// passes while the caller spawned the wrong command" failure this seam
/// exists to prevent.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Program {
    Openspec,
    Herdr,
}

#[cfg(test)]
impl Program {
    fn label(self) -> &'static str {
        match self {
            Program::Openspec => "openspec",
            Program::Herdr => "herdr",
        }
    }
}

/// The recording fake every later change's tests use instead of a real
/// `openspec` or `herdr` binary. `#[cfg(test)] pub(crate)`, following
/// `testutil`'s existing placement in `src/lib.rs`: it contributes nothing
/// to the release binary and nothing to the coverage denominator.
///
/// Every registration and every recorded call is keyed on the **pair** of
/// which program was addressed and the exact argument vector — not the
/// vector alone, for the reason on [`Program`]. An invocation with no
/// registered response panics naming both, rather than returning an empty
/// `Ok`: every consumer of this seam is required to degrade rather than
/// fail on a real error, so a silent empty answer would let a caller's test
/// pass while the caller spawned the wrong command. Registering a pair more
/// than once queues the responses in registration order, and the last
/// registered response for a pair repeats for every further invocation of
/// it, so a poll calling the same command many times needs one
/// registration and a refresh returning changed data needs two.
///
/// Interior state sits behind a `std::sync::Mutex`, never a `RefCell` —
/// `RefCell` is not `Sync`, so it could not satisfy `OpenspecCli: Send +
/// Sync` / `HerdrCli: Send + Sync`, and `live-refresh`/`agent-polling` both
/// need a fake usable from a worker thread.
#[cfg(test)]
pub(crate) struct FakeCli {
    state: std::sync::Mutex<FakeCliState>,
}

#[cfg(test)]
type FakeCliKey = (Program, Vec<String>);

#[cfg(test)]
type FakeCliResponses =
    std::collections::HashMap<FakeCliKey, std::collections::VecDeque<Result<String, CliError>>>;

#[cfg(test)]
#[derive(Default)]
struct FakeCliState {
    responses: FakeCliResponses,
    calls: Vec<FakeCliKey>,
}

#[cfg(test)]
impl FakeCli {
    pub(crate) fn new() -> Self {
        Self {
            state: std::sync::Mutex::new(FakeCliState::default()),
        }
    }

    fn register(&self, program: Program, args: &[&str], response: Result<String, CliError>) {
        let key = (program, args.iter().map(|a| a.to_string()).collect());
        let mut state = self.state.lock().expect("fake cli mutex poisoned");
        state.responses.entry(key).or_default().push_back(response);
    }

    /// Register a response for `args` on the `OpenspecCli` side.
    pub(crate) fn register_openspec(&self, args: &[&str], response: Result<String, CliError>) {
        self.register(Program::Openspec, args, response);
    }

    /// Register a response for `args` on the `HerdrCli` side.
    pub(crate) fn register_herdr(&self, args: &[&str], response: Result<String, CliError>) {
        self.register(Program::Herdr, args, response);
    }

    /// The pairs recorded so far, in call order.
    pub(crate) fn calls(&self) -> Vec<(Program, Vec<String>)> {
        self.state
            .lock()
            .expect("fake cli mutex poisoned")
            .calls
            .clone()
    }

    fn respond(&self, program: Program, args: &[&str]) -> Result<String, CliError> {
        let key: (Program, Vec<String>) = (program, args.iter().map(|a| a.to_string()).collect());
        let mut state = self.state.lock().expect("fake cli mutex poisoned");
        state.calls.push(key.clone());
        let Some(queue) = state.responses.get_mut(&key) else {
            drop(state);
            panic!(
                "FakeCli: no response registered for {} call with arguments {:?}",
                program.label(),
                key.1
            );
        };
        // The last registered response repeats: pop while more than one
        // remains queued, otherwise clone the sole remaining one in place.
        if queue.len() > 1 {
            queue.pop_front().expect("checked non-empty above")
        } else {
            queue.front().expect("checked non-empty above").clone()
        }
    }
}

#[cfg(test)]
impl OpenspecCli for FakeCli {
    fn run(&self, args: &[&str]) -> Result<String, CliError> {
        self.respond(Program::Openspec, args)
    }
}

#[cfg(test)]
impl HerdrCli for FakeCli {
    fn run(&self, args: &[&str]) -> Result<String, CliError> {
        self.respond(Program::Herdr, args)
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
        let cli = super::RealOpenspecCli::new(prog.clone());
        let result = super::OpenspecCli::run(&cli, &["list", "--json"]);
        match result {
            Err(super::CliError::Failed {
                program,
                code,
                stderr,
                args,
            }) => {
                assert_eq!(program, prog.display().to_string());
                assert_eq!(code, Some(3));
                assert_eq!(stderr, "boom\n");
                assert_eq!(args, vec!["list".to_string(), "--json".to_string()]);
                // The stdout payload ("partial") must be nowhere in the
                // error at all — checked against the whole error's Debug
                // rendering, not just the already-asserted-equal `stderr`
                // field, which cannot discriminate this on its own.
                let rendered = format!(
                    "{:?}",
                    super::CliError::Failed {
                        program: program.clone(),
                        code,
                        stderr: stderr.clone(),
                        args: args.clone(),
                    }
                );
                assert!(!rendered.contains("partial"), "rendered: {rendered}");
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    // `live-refresh`: `worker_cli` is the one place a `resolve::BinResolution`
    // becomes the real `OpenspecCli` the refresh worker spawns.

    #[test]
    fn worker_cli_is_none_when_no_binary_was_found() {
        let resolution = crate::resolve::BinResolution {
            found: None,
            problems: Vec::new(),
        };
        assert!(super::worker_cli(resolution).is_none());
    }

    #[test]
    fn worker_cli_spawns_the_resolved_path() {
        let scratch = ScratchDir::new();
        let prog = script(&scratch, "prog", "printf 'ok'\n");
        let resolution = crate::resolve::BinResolution {
            found: Some(crate::resolve::FoundBin {
                path: prog.clone(),
                source: crate::resolve::BinSource::Path,
            }),
            problems: Vec::new(),
        };
        let cli = super::worker_cli(resolution).expect("a binary was found");
        assert_eq!(cli.run(&[]), Ok("ok".to_string()));
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
        let prog = script(
            &scratch,
            "prog",
            "for a in \"$@\"; do printf '%s\\n' \"$a\"; done\n",
        );
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
        assert_eq!(inline, Ok("ok".to_string()));
    }

    // --- group 3: the real implementations ----------------------------------

    #[test]
    fn the_constructed_path_is_the_program_that_runs() {
        let scratch = ScratchDir::new();
        let a = script(&scratch, "a", "printf 'A'\n");
        let b = script(&scratch, "b", "printf 'B'\n");

        let cli_a = super::RealOpenspecCli::new(a);
        assert_eq!(super::OpenspecCli::run(&cli_a, &[]), Ok("A".to_string()));

        let cli_b = super::RealOpenspecCli::new(b);
        assert_eq!(super::OpenspecCli::run(&cli_b, &[]), Ok("B".to_string()));
    }

    #[test]
    fn no_argument_is_added_and_the_working_directory_is_inherited() {
        let scratch = ScratchDir::new();
        let prog = script(&scratch, "prog", "printf '%s\\n' \"$#\"; pwd\n");
        let cli = super::RealOpenspecCli::new(prog);
        let result = super::OpenspecCli::run(&cli, &[]).expect("should succeed");
        let mut lines = result.lines();
        assert_eq!(lines.next(), Some("0"));
        let printed_cwd = lines.next().expect("second line");
        let expected_cwd = std::env::current_dir().expect("current dir");
        assert_eq!(
            std::fs::canonicalize(printed_cwd).expect("canonicalize printed cwd"),
            std::fs::canonicalize(&expected_cwd).expect("canonicalize expected cwd")
        );
    }

    #[test]
    fn the_default_herdr_program_name_is_herdr() {
        let cli = super::RealHerdrCli::default();
        assert_eq!(cli.program(), std::path::Path::new("herdr"));
    }

    #[test]
    fn openspec_clis_program_accessor_reports_the_constructed_path() {
        let cli = super::RealOpenspecCli::new("/some/openspec/path");
        assert_eq!(cli.program(), std::path::Path::new("/some/openspec/path"));
    }

    #[test]
    fn a_program_that_reads_stdin_returns_rather_than_blocking() {
        use std::sync::mpsc;
        use std::time::{Duration, Instant};

        let scratch = ScratchDir::new();
        let prog = script(&scratch, "prog", "cat\n");
        let cli = super::RealOpenspecCli::new(prog);

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = super::OpenspecCli::run(&cli, &[]);
            let _ = tx.send(result);
        });

        let deadline = Instant::now() + Duration::from_secs(30);
        let mut received = None;
        while Instant::now() < deadline {
            if let Ok(result) = rx.try_recv() {
                received = Some(result);
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(received, Some(Ok(String::new())));
    }

    // --- group 4: the recording fake -----------------------------------------

    #[test]
    fn invocations_are_recorded_in_call_order() {
        let fake = super::FakeCli::new();
        fake.register_openspec(&["list", "--json"], Ok("a".to_string()));
        fake.register_openspec(&["status", "--json"], Ok("b".to_string()));

        let _ = super::OpenspecCli::run(&fake, &["status", "--json"]);
        let _ = super::OpenspecCli::run(&fake, &["list", "--json"]);
        let _ = super::OpenspecCli::run(&fake, &["status", "--json"]);

        assert_eq!(
            fake.calls(),
            vec![
                (
                    super::Program::Openspec,
                    vec!["status".to_string(), "--json".to_string()]
                ),
                (
                    super::Program::Openspec,
                    vec!["list".to_string(), "--json".to_string()]
                ),
                (
                    super::Program::Openspec,
                    vec!["status".to_string(), "--json".to_string()]
                ),
            ]
        );
    }

    #[test]
    fn a_response_is_matched_by_the_exact_argument_vector() {
        let fake = super::FakeCli::new();
        fake.register_openspec(&["list", "--json"], Ok("A".to_string()));
        fake.register_openspec(&["list"], Ok("B".to_string()));

        let result = super::OpenspecCli::run(&fake, &["list", "--json"]);
        assert_eq!(result, Ok("A".to_string()));
    }

    /// Extract a panic's `String` or `&str` payload as an owned `String`.
    /// Shared by both panic-inspecting tests below, so the naming assertion
    /// need not be duplicated verbatim.
    fn panic_message(err: &(dyn std::any::Any + Send)) -> String {
        err.downcast_ref::<String>()
            .cloned()
            .or_else(|| err.downcast_ref::<&str>().map(|s| s.to_string()))
            .expect("panic payload should be a string")
    }

    #[test]
    fn an_openspec_call_is_not_answered_from_a_herdr_registration() {
        let fake = super::FakeCli::new();
        fake.register_openspec(&["list", "--json"], Ok("openspec answer".to_string()));

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            super::HerdrCli::run(&fake, &["list", "--json"])
        }));
        let err = result.expect_err(
            "should have panicked rather than answering from the openspec registration",
        );
        let message = panic_message(err.as_ref());
        assert!(message.contains("list"), "message: {message}");
        assert!(
            message.to_lowercase().contains("herdr"),
            "message should name the HerdrCli program: {message}"
        );
    }

    #[test]
    fn each_handle_gets_its_own_registration() {
        let fake = super::FakeCli::new();
        fake.register_openspec(&["same"], Ok("openspec side".to_string()));
        fake.register_herdr(&["same"], Ok("herdr side".to_string()));

        // Call the HerdrCli handle FIRST, deliberately out of registration
        // order: a fake keyed on the argument vector alone (rather than the
        // pair of program-addressed and vector) would queue both
        // registrations under one shared key and hand back "openspec side"
        // here regardless — the queue's registration order, not the
        // program actually addressed — so this ordering is what makes the
        // assertion discriminate rather than pass by queue-order accident.
        assert_eq!(
            super::HerdrCli::run(&fake, &["same"]),
            Ok("herdr side".to_string())
        );
        assert_eq!(
            super::OpenspecCli::run(&fake, &["same"]),
            Ok("openspec side".to_string())
        );
        assert_eq!(
            fake.calls(),
            vec![
                (super::Program::Herdr, vec!["same".to_string()]),
                (super::Program::Openspec, vec!["same".to_string()]),
            ]
        );
    }

    #[test]
    #[should_panic(expected = "lst")]
    fn an_unregistered_invocation_panics_naming_the_program_and_the_vector() {
        let fake = super::FakeCli::new();
        fake.register_openspec(&["list", "--json"], Ok("a".to_string()));
        let _ = super::OpenspecCli::run(&fake, &["lst", "--json"]);
    }

    #[test]
    fn an_unregistered_invocation_panic_message_names_the_program() {
        let fake = super::FakeCli::new();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            super::OpenspecCli::run(&fake, &["lst"])
        }));
        let err = result.expect_err("should have panicked");
        let message = panic_message(err.as_ref());
        assert!(message.contains("lst"), "message: {message}");
        assert!(
            message.to_lowercase().contains("openspec"),
            "message should name the program addressed: {message}"
        );
    }

    #[test]
    fn queued_responses_are_returned_in_order_and_the_last_one_repeats() {
        let fake = super::FakeCli::new();
        fake.register_openspec(&["poll"], Ok("first".to_string()));
        fake.register_openspec(&["poll"], Ok("second".to_string()));

        let results: Vec<_> = (0..4)
            .map(|_| super::OpenspecCli::run(&fake, &["poll"]))
            .collect();
        assert_eq!(
            results,
            vec![
                Ok("first".to_string()),
                Ok("second".to_string()),
                Ok("second".to_string()),
                Ok("second".to_string()),
            ]
        );
    }

    #[test]
    fn a_failure_can_be_registered() {
        let fake = super::FakeCli::new();
        let err = super::CliError::Failed {
            program: "openspec".to_string(),
            args: vec!["list".to_string()],
            code: Some(1),
            stderr: "boom".to_string(),
        };
        fake.register_openspec(&["list"], Err(err.clone()));

        let result = super::OpenspecCli::run(&fake, &["list"]);
        assert_eq!(result, Err(err));
    }

    #[test]
    fn the_fake_is_usable_from_another_thread() {
        let fake = std::sync::Arc::new(super::FakeCli::new());
        fake.register_openspec(&["a"], Ok("A".to_string()));
        fake.register_openspec(&["b"], Ok("B".to_string()));

        let inline = super::OpenspecCli::run(fake.as_ref(), &["a"]);
        let f = fake.clone();
        let handle = std::thread::spawn(move || super::OpenspecCli::run(f.as_ref(), &["b"]));
        let threaded = handle.join().expect("thread panicked");

        assert_eq!(inline, Ok("A".to_string()));
        assert_eq!(threaded, Ok("B".to_string()));
        assert_eq!(fake.calls().len(), 2);
    }

    #[test]
    fn two_fakes_are_independent() {
        let fake_a = super::FakeCli::new();
        let fake_b = super::FakeCli::new();
        fake_a.register_openspec(&["list"], Ok("A".to_string()));
        fake_b.register_openspec(&["list"], Ok("B".to_string()));

        assert_eq!(
            super::OpenspecCli::run(&fake_a, &["list"]),
            Ok("A".to_string())
        );
        assert_eq!(
            super::OpenspecCli::run(&fake_b, &["list"]),
            Ok("B".to_string())
        );
        assert_eq!(fake_a.calls().len(), 1);
        assert_eq!(fake_b.calls().len(), 1);
    }

    // --- group 5: the npm-prefix decision and the spawning probe -----------

    #[test]
    fn a_trailing_newline_is_trimmed_off_the_prefix() {
        assert_eq!(
            super::npm_prefix_from(true, b"/opt/homebrew\n"),
            Some(std::path::PathBuf::from("/opt/homebrew"))
        );
    }

    #[test]
    fn surrounding_whitespace_is_trimmed() {
        assert_eq!(
            super::npm_prefix_from(true, b"  /usr/local  \n"),
            Some(std::path::PathBuf::from("/usr/local"))
        );
    }

    #[test]
    fn empty_or_whitespace_only_output_is_no_prefix() {
        for stdout in [b"".as_slice(), b"\n".as_slice(), b"   \t \n".as_slice()] {
            assert_eq!(
                super::npm_prefix_from(true, stdout),
                None,
                "stdout {stdout:?} should yield no prefix"
            );
        }
    }

    #[test]
    fn a_non_zero_exit_is_no_prefix_even_with_output() {
        assert_eq!(super::npm_prefix_from(false, b"/opt/homebrew\n"), None);
    }

    #[test]
    fn invalid_utf8_on_stdout_is_decoded_lossily_and_then_trimmed() {
        assert_eq!(
            super::npm_prefix_from(true, &[0x2F, 0x61, 0xFF, 0x0A]),
            Some(std::path::PathBuf::from("/a\u{FFFD}"))
        );
    }

    #[test]
    fn stderr_noise_does_not_reach_the_prefix() {
        let scratch = ScratchDir::new();
        let prog = script(
            &scratch,
            "npm",
            "printf '/scratch/prefix\\n'; printf 'zsh: plugin warning\\n/wrong/prefix\\n' >&2\n",
        );
        let result = super::npm_prefix_via(&prog);
        assert_eq!(result, Some(std::path::PathBuf::from("/scratch/prefix")));
        assert!(!result.unwrap().to_string_lossy().contains("wrong"));
    }

    #[test]
    fn a_program_that_cannot_be_started_is_no_prefix() {
        let scratch = ScratchDir::new();
        let missing = scratch.path().join("does-not-exist");
        assert!(!missing.exists());
        let result = super::npm_prefix_via(&missing);
        assert_eq!(result, None);
    }

    // --- group 6: the hand-over's two successor tests -----------------------

    #[test]
    fn the_binding_delegates_to_the_probe_rather_than_answering_for_itself() {
        // Machine-independent, and — unlike an assertion on the value
        // alone — red for a hardcoded `None` body wherever a working `npm`
        // exists: `npm_prefix()`'s own body is exactly `npm_prefix_via(Path::new("npm"))`,
        // so the two calls must agree regardless of what `npm` (if any) is
        // actually on this machine's `PATH`.
        assert_eq!(
            super::npm_prefix(),
            super::npm_prefix_via(std::path::Path::new("npm"))
        );
    }

    #[test]
    fn the_binding_yields_either_nothing_or_an_absolute_path() {
        // Names no machine-specific value, so it passes with npm installed,
        // with npm broken, and on the no-tools PATH task 8.4 runs the suite
        // on.
        match super::npm_prefix() {
            None => {}
            Some(path) => assert!(
                path.is_absolute(),
                "expected an absolute path or nothing, got {path:?}"
            ),
        }
    }

    // --- group 7: the end-to-end join, and proof that nothing is written ---

    /// Build an environment lookup closure over a fixture map — mirrors
    /// `resolve::tests::env`, kept local rather than shared across modules
    /// since it is a two-line test helper, not part of either module's
    /// public contract.
    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        let map: std::collections::BTreeMap<&str, &str> = pairs.iter().copied().collect();
        move |name| map.get(name).map(|s| s.to_string())
    }

    #[test]
    fn a_real_spawn_resolves_the_npm_prefix_binary() {
        let scratch = ScratchDir::new();
        let path_only = scratch.path().join("path-only");
        std::fs::create_dir_all(&path_only).expect("create path-only dir");

        let prefix_dir = scratch.path().join("N");
        let openspec_bin = prefix_dir.join("bin").join("openspec");
        write_with_mode(&openspec_bin, b"#!/bin/sh\n", 0o755);

        let npm = script(
            &scratch,
            "npm",
            &format!(
                "printf '%s\\n' '{}'; printf 'unrelated noise\\n' >&2\n",
                prefix_dir.display()
            ),
        );

        let path_only_str = path_only.display().to_string();
        let pairs = [("PATH", path_only_str.as_str())];
        let lookup = env(&pairs);
        let hook = || super::npm_prefix_via(&npm);

        let result = crate::resolve::openspec_bin(None, &lookup, &hook);
        assert_eq!(
            result.found,
            Some(crate::resolve::FoundBin {
                path: openspec_bin.clone(),
                source: crate::resolve::BinSource::NpmPrefix,
            })
        );
    }

    #[test]
    fn a_failing_real_spawn_resolves_nothing() {
        let scratch = ScratchDir::new();
        let path_only = scratch.path().join("path-only");
        std::fs::create_dir_all(&path_only).expect("create path-only dir");

        let prefix_dir = scratch.path().join("N");
        let openspec_bin = prefix_dir.join("bin").join("openspec");
        write_with_mode(&openspec_bin, b"#!/bin/sh\n", 0o755);

        let npm = script(
            &scratch,
            "npm",
            &format!("printf '%s\\n' '{}'; exit 1\n", prefix_dir.display()),
        );

        let path_only_str = path_only.display().to_string();
        let pairs = [("PATH", path_only_str.as_str())];
        let lookup = env(&pairs);
        let hook = || super::npm_prefix_via(&npm);

        let result = crate::resolve::openspec_bin(None, &lookup, &hook);
        assert_eq!(result.found, None);
        assert!(result.problems.is_empty());
    }

    #[test]
    fn a_run_and_a_probe_leave_the_scratch_tree_byte_identical() {
        use crate::testutil::{shallow_snapshot, snapshot};

        let scratch = ScratchDir::new();
        let root = scratch.path();

        let ok_prog = script(&scratch, "ok_prog", "printf 'ok'\n");
        let fail_prog = script(
            &scratch,
            "fail_prog",
            "printf 'partial'; printf 'boom' >&2; exit 3\n",
        );
        let missing = root.join("does-not-exist");
        assert!(!missing.exists());

        let prefix_dir = root.join("prefix");
        write_with_mode(
            &prefix_dir.join("bin").join("openspec"),
            b"#!/bin/sh\n",
            0o755,
        );

        let npm = script(&scratch, "npm", "printf '/scratch/prefix\\n'\n");

        // An empty directory too, since it is the entry a listing-only
        // comparison could miss but the extended snapshot cannot.
        let empty = root.join("empty");
        std::fs::create_dir_all(&empty).expect("create empty dir");

        // Snapshots taken around the scratch tree AND around the test
        // process's own working directory — the requirement's "not in the
        // working directory" clause is covered by evidence, not implied by
        // the scratch tree's result alone. The scratch tree is snapshotted
        // in full (`snapshot`, recursive, byte-comparing); the cwd is
        // snapshotted only at its top level (`shallow_snapshot`) — under
        // `cargo test`, the real cwd is this crate's own repository root,
        // whose `target/` directory alone holds tens of thousands of
        // build-artifact files, so a recursive byte-comparing snapshot of
        // it was measured to cost multiple seconds and hundreds of
        // megabytes of reads per test run, for evidence no more meaningful
        // than the shallow form: nothing in this seam computes a path
        // relative to the current directory, so a stray write would land
        // at the top level, which the shallow snapshot catches directly.
        let cwd = std::env::current_dir().expect("current dir");

        let before_scratch = snapshot(root);
        let before_cwd = shallow_snapshot(&cwd);

        let ok_cli = super::RealOpenspecCli::new(ok_prog);
        let _ = super::OpenspecCli::run(&ok_cli, &[]);

        let fail_cli = super::RealOpenspecCli::new(fail_prog);
        let _ = super::OpenspecCli::run(&fail_cli, &[]);

        let missing_cli = super::RealOpenspecCli::new(missing);
        let _ = super::OpenspecCli::run(&missing_cli, &[]);

        let _ = super::npm_prefix_via(&npm);

        let after_scratch = snapshot(root);
        let after_cwd = shallow_snapshot(&cwd);

        assert_eq!(before_scratch, after_scratch);
        assert_eq!(before_cwd, after_cwd);
    }
}
