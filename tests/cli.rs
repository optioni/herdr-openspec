// Binary-integration tier: spawns the crate's own binary via
// `env!("CARGO_BIN_EXE_herdr-openspec")`. See design.md -> Test Strategy and
// -> Decisions ("A `tests/cli.rs` tier that spawns the crate's own binary") for
// why this does not breach the "nothing spawns a process outside `cli`" rule:
// that rule is about the `openspec` and `herdr` binaries in production code,
// and this spawns only this crate's own binary.

use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_herdr-openspec"))
}

/// Every `HERDR_*` variable a plugin process could inherit, per design.md -> Test
/// Boundaries and -> Risks ("`cargo test` may run inside a Herdr pane"). Named once here
/// so `scrubbed()` and `stub_herdr()`'s synthetic-context callers agree on the full set.
const HERDR_VARS: &[&str] = &[
    "HERDR_ENV",
    "HERDR_BIN_PATH",
    "HERDR_SOCKET_PATH",
    "HERDR_PANE_ID",
    "HERDR_TAB_ID",
    "HERDR_WORKSPACE_ID",
    "HERDR_PLUGIN_ID",
    "HERDR_PLUGIN_ROOT",
    "HERDR_PLUGIN_CONFIG_DIR",
    "HERDR_PLUGIN_STATE_DIR",
    "HERDR_PLUGIN_ACTION_ID",
    "HERDR_PLUGIN_CONTEXT_JSON",
];

/// A `Command` for the crate's own binary with every `HERDR_*` variable removed from the
/// child's environment, so a suite running inside a Herdr pane cannot inherit a live
/// context and reach the real socket (design.md -> Risks).
fn scrubbed() -> Command {
    let mut cmd = bin();
    for var in HERDR_VARS {
        cmd.env_remove(var);
    }
    cmd
}

/// A scratch directory under `std::env::temp_dir()`, removed recursively on drop —
/// `testutil::ScratchDir`'s shape reimplemented here because `tests/cli.rs` is a separate
/// integration-test crate and cannot reach the library's `pub(crate)` items.
struct ScratchDir {
    path: std::path::PathBuf,
}

impl ScratchDir {
    fn new() -> Self {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let counter = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "herdr-openspec-cli-test-{}-{counter}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("create scratch dir");
        Self { path }
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Write a `#!/bin/sh` program named `herdr` into a fresh [`ScratchDir`] that appends its
/// argument vector (one line, space-joined) to `argv.log` inside the scratch dir and
/// answers `pane list` with an empty `{"result":{"panes":[]}}` — everything else exits 0
/// with empty stdout, since `open::run` never depends on any other call succeeding.
/// Returns the scratch dir (kept alive for its `Drop`) and the directory holding the
/// `herdr` program, suitable for prepending to `PATH`.
fn stub_herdr() -> (ScratchDir, std::path::PathBuf) {
    let scratch = ScratchDir::new();
    let bin_dir = scratch.path().join("bin");
    std::fs::create_dir_all(&bin_dir).expect("create stub bin dir");
    let herdr = bin_dir.join("herdr");
    let script = r#"#!/bin/sh
echo "$@" >> "$(dirname "$0")/../argv.log"
case "$*" in
  "pane list") printf '{"result":{"panes":[]}}' ;;
  *) printf '' ;;
esac
exit 0
"#;
    std::fs::write(&herdr, script).expect("write stub herdr script");
    let mut perms = std::fs::metadata(&herdr)
        .expect("stat stub herdr script")
        .permissions();
    use std::os::unix::fs::PermissionsExt;
    perms.set_mode(0o755);
    std::fs::set_permissions(&herdr, perms).expect("chmod stub herdr script");
    (scratch, bin_dir)
}

#[test]
fn ui_without_a_terminal_exits_three() {
    // stdin's write end is deliberately left open (not Stdio::null()) so a
    // wrongly-blocking implementation hangs rather than reaching EOF and
    // exiting for an unrelated reason.
    let mut child = bin()
        .arg("ui")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn binary");

    let deadline = Instant::now() + Duration::from_secs(10);
    let exited = loop {
        if child.try_wait().expect("try_wait failed").is_some() {
            break true;
        }
        if Instant::now() >= deadline {
            break false;
        }
        thread::sleep(Duration::from_millis(10));
    };
    assert!(
        exited,
        "process blocked on stdin instead of exiting immediately"
    );

    let output = child.wait_with_output().expect("failed to wait on child");

    assert_eq!(output.status.code(), Some(3), "status: {:?}", output.status);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("herdr-openspec"), "stderr: {stderr}");
    assert!(stderr.contains("not a terminal"), "stderr: {stderr}");
}

#[test]
fn failing_statuses_are_distinct() {
    // Every run pipes stdout and starts stdin at EOF (Stdio::null()), so
    // none of the four can block: `ui` refuses immediately for lack of a
    // terminal, and the other three are argument-classification rejections
    // that never reach `ui::run` at all.
    let run = |args: &[&str]| {
        bin()
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("failed to run binary")
    };

    let ui = run(&["ui"]);
    let wat = run(&["wat"]);
    let ui_tab = run(&["ui", "--tab"]);
    let none = run(&[]);

    assert_eq!(ui.status.code(), Some(3), "ui status: {:?}", ui.status);
    assert_eq!(wat.status.code(), Some(2), "wat status: {:?}", wat.status);
    assert_eq!(
        ui_tab.status.code(),
        Some(2),
        "ui --tab status: {:?}",
        ui_tab.status
    );
    assert_eq!(
        none.status.code(),
        Some(2),
        "no-args status: {:?}",
        none.status
    );

    for (name, output) in [("wat", &wat), ("ui --tab", &ui_tab), ("no-args", &none)] {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("ui"), "{name} stderr: {stderr}");
        // The Commands: block now names three whole tokens — see
        // specs/pane-open/spec.md -> "Usage names all three commands".
        assert!(stderr.contains("open-tab"), "{name} stderr: {stderr}");
        assert!(
            stderr.contains("usage: herdr-openspec <ui|open|open-tab>"),
            "{name} stderr: {stderr}"
        );
    }
    let wat_stderr = String::from_utf8_lossy(&wat.stderr);
    assert!(wat_stderr.contains("wat"), "wat stderr: {wat_stderr}");
    let ui_tab_stderr = String::from_utf8_lossy(&ui_tab.stderr);
    assert!(
        ui_tab_stderr.contains("--tab"),
        "ui --tab stderr: {ui_tab_stderr}"
    );

    for (name, output) in [
        ("ui", &ui),
        ("wat", &wat),
        ("ui --tab", &ui_tab),
        ("no-args", &none),
    ] {
        assert!(
            output.stdout.is_empty(),
            "{name} stdout: {:?}",
            output.stdout
        );
    }
}

#[test]
fn unknown_subcommand() {
    let output = bin()
        .arg("wat")
        .stdin(Stdio::null())
        .output()
        .expect("failed to run binary");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ui"), "stderr: {stderr}");
    assert!(stderr.contains("wat"), "stderr: {stderr}");
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
}

#[test]
fn extra_arguments_after_ui() {
    // Leave stdin's write end open (not Stdio::null()) so a wrongly-blocking
    // implementation would hang here instead of trivially reaching EOF.
    let mut child = bin()
        .arg("ui")
        .arg("--tab")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn binary");

    // Poll to a deadline rather than sleeping a fixed interval. Process startup
    // on a cold or loaded machine can exceed any single sleep, which made this
    // test flaky; a wrongly-blocking implementation still fails, because the
    // deadline expires with the child alive.
    let deadline = Instant::now() + Duration::from_secs(10);
    let exited = loop {
        if child.try_wait().expect("try_wait failed").is_some() {
            break true;
        }
        if Instant::now() >= deadline {
            break false;
        }
        thread::sleep(Duration::from_millis(10));
    };
    assert!(
        exited,
        "process blocked on stdin instead of exiting immediately"
    );

    let output = child.wait_with_output().expect("failed to wait on child");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--tab"), "stderr: {stderr}");
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
}

/// `open` with the environment scrubbed exits 1 within a ten-second deadline, names
/// `HERDR_WORKSPACE_ID` on stderr, and prints nothing to stdout. See
/// `specs/pane-open/spec.md` -> "`open` outside Herdr fails promptly rather than hanging
/// or rendering".
#[test]
fn open_outside_herdr_exits_one() {
    let mut child = scrubbed()
        .arg("open")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn binary");

    let deadline = Instant::now() + Duration::from_secs(10);
    let exited = loop {
        if child.try_wait().expect("try_wait failed").is_some() {
            break true;
        }
        if Instant::now() >= deadline {
            break false;
        }
        thread::sleep(Duration::from_millis(10));
    };
    assert!(exited, "process blocked instead of exiting promptly");

    let output = child.wait_with_output().expect("failed to wait on child");
    assert_eq!(output.status.code(), Some(1), "status: {:?}", output.status);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("HERDR_WORKSPACE_ID"), "stderr: {stderr}");
}

/// `main` really routes `open` to the split placement and `open-tab` to the tab
/// placement, driven end to end through the real built binary against a scratch `herdr`
/// stub on `PATH` — see design.md -> Decision 12 and `specs/pane-open/spec.md` -> "`main`
/// really routes each subcommand to its own placement". A status-only assertion cannot
/// distinguish the two subcommands, because with `HERDR_*` scrubbed both die identically
/// inside `open::context`.
#[test]
fn main_routes_each_subcommand_to_its_own_placement() {
    let (scratch, bin_dir) = stub_herdr();
    let path_var = format!(
        "{}:{}",
        bin_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let context_json = r#"{"workspace_id":"w8","workspace_cwd":"/repo","focused_pane_id":"w8:p1"}"#;

    let run = |sub: &str| {
        scrubbed()
            .arg(sub)
            .env("PATH", &path_var)
            .env("HERDR_WORKSPACE_ID", "w8")
            .env("HERDR_PLUGIN_CONTEXT_JSON", context_json)
            .stdin(Stdio::null())
            .output()
            .expect("failed to run binary")
    };

    let open_out = run("open");
    let open_tab_out = run("open-tab");

    assert_eq!(
        open_out.status.code(),
        Some(0),
        "open stderr: {}",
        String::from_utf8_lossy(&open_out.stderr)
    );
    assert_eq!(
        open_tab_out.status.code(),
        Some(0),
        "open-tab stderr: {}",
        String::from_utf8_lossy(&open_tab_out.stderr)
    );

    // Each subcommand issues `pane list` before its own `plugin pane open` call (design.md
    // -> "An already-open dashboard is focused, never duplicated"), so four lines total —
    // filter down to the two `plugin pane open` calls to read the placement each produced.
    let argv_log = std::fs::read_to_string(scratch.path().join("argv.log")).expect("read argv.log");
    let lines: Vec<&str> = argv_log.lines().collect();
    assert_eq!(lines.len(), 4, "argv.log:\n{argv_log}");
    let opens: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|l| l.contains("plugin pane open"))
        .collect();
    assert_eq!(opens.len(), 2, "argv.log:\n{argv_log}");
    assert!(
        opens[0].contains("--placement split --direction right"),
        "open call 0: {}",
        opens[0]
    );
    assert!(
        opens[1].contains("--placement tab --workspace"),
        "open call 1: {}",
        opens[1]
    );
    // No --cwd is ever passed to `plugin pane open` — corrected in group 10 after a live
    // check found Herdr 0.8.2 resolves the manifest's *relative* command against --cwd
    // too, breaking the spawn entirely for any workspace directory that holds no
    // target/release/ binary of its own (design.md -> Decision 6, corrected).
    assert!(!opens[0].contains("--cwd"), "open call 0: {}", opens[0]);
    assert!(!opens[1].contains("--cwd"), "open call 1: {}", opens[1]);

    let tab_flag_out = scrubbed()
        .arg("open")
        .arg("--tab")
        .env("PATH", &path_var)
        .stdin(Stdio::null())
        .output()
        .expect("failed to run binary");
    assert_eq!(tab_flag_out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&tab_flag_out.stderr);
    assert!(stderr.contains("usage"), "stderr: {stderr}");
}

/// The open family never reaches status 3 — that stays `ui`'s alone, since a plugin
/// action's stdout is measured not to be a terminal. See `specs/plugin-build/spec.md` ->
/// "The open family never reaches status 3".
#[test]
fn open_never_reaches_status_three() {
    let run = |args: &[&str]| {
        scrubbed()
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("failed to run binary")
    };

    let open = run(&["open"]);
    let open_tab = run(&["open-tab"]);
    let open_tab_flag = run(&["open", "--tab"]);

    assert_eq!(
        open.status.code(),
        Some(1),
        "open stderr: {}",
        String::from_utf8_lossy(&open.stderr)
    );
    assert_eq!(
        open_tab.status.code(),
        Some(1),
        "open-tab stderr: {}",
        String::from_utf8_lossy(&open_tab.stderr)
    );
    assert_eq!(
        open_tab_flag.status.code(),
        Some(2),
        "open --tab stderr: {}",
        String::from_utf8_lossy(&open_tab_flag.stderr)
    );

    for (name, output) in [
        ("open", &open),
        ("open-tab", &open_tab),
        ("open --tab", &open_tab_flag),
    ] {
        assert!(
            output.stdout.is_empty(),
            "{name} stdout: {:?}",
            output.stdout
        );
    }
    let usage_stderr = String::from_utf8_lossy(&open_tab_flag.stderr);
    assert!(usage_stderr.contains("usage"), "{usage_stderr}");
}

/// Every place this file spawns the crate's own binary sets stdout to a pipe or to null
/// (never inheriting the test process's own terminal), and every such site whose
/// arguments name `open` or `open-tab` goes through `scrubbed()` rather than the bare
/// `bin()`. An **executing** inspection of this file's own source through `file!()` —
/// the landed scenario of this name was satisfied by manual inspection, not a test. See
/// `specs/plugin-build/spec.md` -> "Every binary-integration run pipes stdout".
#[test]
fn every_run_pipes_and_scrubs_herdr() {
    let source = std::fs::read_to_string(file!()).expect("read own source");

    // Comment lines (including doc comments, which mention `bin()`/`scrubbed()` by name
    // in prose) are blanked before scanning, so a doc comment cannot be mistaken for a
    // call site — every line's length and position is otherwise preserved.
    let mut code = String::with_capacity(source.len());
    for line in source.lines() {
        if line.trim_start().starts_with("//") {
            code.push('\n');
        } else {
            code.push_str(line);
            code.push('\n');
        }
    }

    // A "spawn site" is a call chain starting at a `bin()` or `scrubbed()` **call** (not
    // their own `fn` definitions) and ending at the next `.output()` or `.spawn(` in the
    // source — the two ways this file starts the crate's own binary. Measured against
    // this file's own landed shape: 9. The floor is the realized count, not a round
    // number below it — a rewritten harness that hid a site would drop below it.
    const MIN_SPAWN_SITES: usize = 9;

    let mut starts: Vec<(usize, usize, bool)> = Vec::new();
    for (i, m) in code.match_indices("bin()") {
        starts.push((i, m.len(), false));
    }
    for (i, m) in code.match_indices("scrubbed()") {
        starts.push((i, m.len(), true));
    }
    starts.sort_by_key(|(i, ..)| *i);

    let mut spawn_sites = 0usize;
    for (start, matched_len, is_scrubbed) in starts {
        // Skip the two functions' own definitions (`fn bin() -> Command {` / `fn
        // scrubbed() -> Command {`), which are not call sites.
        let before = &code[start.saturating_sub(4)..start];
        if before.contains("fn ") {
            continue;
        }
        // Skip a bare, non-chained call (`let mut cmd = bin();` inside `scrubbed()`'s own
        // body) — a real spawn site always continues the chain with `.arg(`/`.args(`.
        let after = code[start + matched_len..].trim_start();
        if !after.starts_with('.') {
            continue;
        }

        let rel_end = code[start..]
            .find(".output()")
            .into_iter()
            .chain(code[start..].find(".spawn("))
            .min();
        let Some(rel_end) = rel_end else {
            continue;
        };
        let end = start + rel_end;
        let chain = &code[start..end];
        assert!(
            !chain.contains("\nfn ") && !chain.contains("    fn "),
            "spawn-site scan at byte {start} ran past a function boundary: {chain:?}"
        );
        spawn_sites += 1;

        let uses_output = code[end..].starts_with(".output()");
        let stdout_piped_or_null =
            chain.contains(".stdout(Stdio::piped())") || chain.contains(".stdout(Stdio::null())");
        assert!(
            uses_output || stdout_piped_or_null,
            "spawn site at byte {start} neither uses .output() (which always pipes \
             stdout) nor sets .stdout(Stdio::piped()/null()) before .spawn(): {chain}"
        );

        let names_open = chain.contains("\"open\"") || chain.contains("\"open-tab\"");
        if names_open {
            assert!(
                is_scrubbed,
                "spawn site at byte {start} names open/open-tab but was built from bin() \
                 rather than scrubbed(), so it does not remove HERDR_* from the child's \
                 environment: {chain}"
            );
        }
    }

    assert!(
        spawn_sites >= MIN_SPAWN_SITES,
        "found only {spawn_sites} spawn sites (expected >= {MIN_SPAWN_SITES}) - the scan \
         is vacuous"
    );
}

#[test]
fn no_subcommand() {
    let output = bin()
        .stdin(Stdio::null())
        .output()
        .expect("failed to run binary");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ui"), "stderr: {stderr}");
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
}
