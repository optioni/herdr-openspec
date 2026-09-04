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
