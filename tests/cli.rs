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
fn ui_prints_placeholder_banner() {
    let output = bin()
        .arg("ui")
        .stdin(Stdio::null())
        .output()
        .expect("failed to run binary");

    assert!(output.status.success(), "status: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("herdr-openspec"), "stdout: {stdout}");
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "stdout: {stdout}"
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn ui_holds_open_until_stdin_closes() {
    let mut child = bin()
        .arg("ui")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn binary");

    // Give the process time to print the banner and start blocking on stdin.
    thread::sleep(Duration::from_millis(200));
    assert!(
        child.try_wait().expect("try_wait failed").is_none(),
        "process exited before stdin closed"
    );

    // Dropping the stdin handle closes the write end, so the child's stdin
    // reaches EOF and it should exit.
    drop(child.stdin.take());

    let status = child.wait().expect("failed to wait on child");
    assert!(status.success(), "status: {status:?}");
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
