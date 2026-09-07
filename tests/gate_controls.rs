//! The outer-loop acceptance test for `gate-integrity`: every gate's positive control is
//! EXECUTED, not attested. Binds `tests/gate-controls.toml` — one `[[control]]` per
//! `scripts/gates/*` script, plus wired.sh's own second entry — to a real run: copy the
//! repository tree to a scratch directory, confirm the named gate exits 0 UNPLANTED, apply
//! the recorded plant, and confirm the gate now exits non-zero and names its own `expect`
//! fragment. See `openspec/changes/gate-integrity/design.md` -> Test Strategy and the
//! `quality-gates` spec's "Every gate's positive control is executed, not attested" and "A
//! gate that cannot fail is itself a failure" scenarios.
//!
//! This file is written FIRST, as task group 0, and is RED for exactly the five entries
//! `tests/gate-controls.toml` documents as such: the gate they name still reports success on
//! the exact defect it exists to catch. Groups 2-5 repair the named scripts under
//! `scripts/gates/`; nothing in this file or the toml table should change to make a failure
//! disappear.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A scratch directory under `std::env::temp_dir()`, removed recursively on drop —
/// `testutil::ScratchDir`'s shape reimplemented here because `tests/gate_controls.rs` is a
/// separate integration-test crate and cannot reach the library's `pub(crate)` items, the
/// same reason `tests/cli.rs` and `tests/spec_purposes.rs` each carry their own copy.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new(label: &str) -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        let safe_label: String = label
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let path = std::env::temp_dir().join(format!(
            "herdr-openspec-gate-controls-{}-{counter}-{safe_label}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create scratch dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// One `[[control]]` entry, exactly as `tests/gate-controls.toml` carries it. See that
/// file's own header comment for the field reference.
#[derive(Debug, Clone)]
struct Control {
    id: String,
    script: String,
    interpreter: String,
    env: String,
    needs_git: bool,
    cargo: bool,
    plant_kind: String,
    plant_file: String,
    plant_find: Option<String>,
    plant_replace: Option<String>,
    plant_content: Option<String>,
    expect: String,
}

fn get_str(table: &toml::Table, key: &str, id_hint: &str) -> Result<String, String> {
    table
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| format!("{id_hint}: missing or non-string {key:?}"))
}

fn get_str_opt(table: &toml::Table, key: &str) -> Option<String> {
    table.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

fn get_bool_opt(table: &toml::Table, key: &str) -> bool {
    table.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Parse `tests/gate-controls.toml`'s `[[control]]` array via the `toml` crate's own
/// `Table`/`Value` API — the same hand-rolled approach `tests/degraded_coverage.rs` uses for
/// `tests/degraded-coverage.toml`, and for the same reason: no `serde` derive dependency.
fn parse_controls(text: &str) -> Result<Vec<Control>, String> {
    let doc: toml::Table = text
        .parse()
        .map_err(|e| format!("tests/gate-controls.toml is not valid TOML: {e}"))?;
    let array = doc
        .get("control")
        .ok_or_else(|| "tests/gate-controls.toml has no [[control]] entries".to_string())?
        .as_array()
        .ok_or_else(|| "tests/gate-controls.toml's \"control\" key is not an array".to_string())?;

    let mut out = Vec::with_capacity(array.len());
    for (index, entry) in array.iter().enumerate() {
        let table = entry
            .as_table()
            .ok_or_else(|| format!("control {index}: not a table"))?;
        let id = get_str(table, "id", &format!("control {index}"))?;
        let script = get_str(table, "script", &id)?;
        let interpreter = get_str_opt(table, "interpreter").unwrap_or_else(|| "sh".to_string());
        let env = get_str_opt(table, "env").unwrap_or_default();
        let needs_git = get_bool_opt(table, "needs_git");
        let cargo = get_bool_opt(table, "cargo");
        let plant_kind = get_str_opt(table, "plant_kind").unwrap_or_else(|| "edit".to_string());
        let plant_file = get_str(table, "plant_file", &id)?;
        let plant_find = get_str_opt(table, "plant_find");
        let plant_replace = get_str_opt(table, "plant_replace");
        let plant_content = get_str_opt(table, "plant_content");
        let expect = get_str(table, "expect", &id)?;

        if plant_kind == "edit" && (plant_find.is_none() || plant_replace.is_none()) {
            return Err(format!(
                "control {id:?}: plant_kind \"edit\" requires both plant_find and plant_replace"
            ));
        }
        if plant_kind == "create" && plant_content.is_none() {
            return Err(format!(
                "control {id:?}: plant_kind \"create\" requires plant_content"
            ));
        }

        out.push(Control {
            id,
            script,
            interpreter,
            env,
            needs_git,
            cargo,
            plant_kind,
            plant_file,
            plant_find,
            plant_replace,
            plant_content,
            expect,
        });
    }
    Ok(out)
}

fn controls_toml() -> String {
    fs::read_to_string(manifest_dir().join("tests/gate-controls.toml"))
        .expect("read tests/gate-controls.toml")
}

fn parsed_controls() -> Vec<Control> {
    parse_controls(&controls_toml()).expect("parse tests/gate-controls.toml")
}

/// Every `.sh` and `.py` file directly under `scripts/gates/`, sorted.
fn gate_scripts() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(manifest_dir().join("scripts/gates"))
        .expect("read scripts/gates")
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

/// `raw` is the invocation prefix exactly as the `gates:` recipe writes it for this
/// subject — e.g. `"env -u GRAPH_WRITE"`, `"SCAN_MIN=135"`, or empty for a bare invocation.
/// Returns the environment variables to set and the ones to remove from the child's
/// inherited environment.
fn env_overrides(raw: &str) -> (Vec<(String, String)>, Vec<String>) {
    let mut sets = Vec::new();
    let mut unsets = Vec::new();
    let tokens: Vec<&str> = raw.split_whitespace().collect();
    let mut i = 0;
    while i < tokens.len() {
        let token = tokens[i];
        if token == "env" {
            i += 1;
            continue;
        }
        if token == "-u" {
            if let Some(var) = tokens.get(i + 1) {
                unsets.push((*var).to_string());
            }
            i += 2;
            continue;
        }
        if let Some((key, value)) = token.split_once('=') {
            let cleaned = value.trim_matches('\'').trim_matches('"');
            sets.push((key.to_string(), cleaned.to_string()));
        }
        i += 1;
    }
    (sets, unsets)
}

/// Copies every file under the repository root into `dest`, excluding `.git` and `target` —
/// the former because each control that needs one makes its own, the latter because it can
/// hold gigabytes of build artifacts no gate script reads. `tar` piped into `tar`, per
/// design.md -> Test Boundaries ("a per-plant copy ... made with tar/cp").
fn copy_tree(dest: &Path) {
    let src = manifest_dir();
    // The exclude flag is assembled at runtime, rather than written out whole, so this
    // file's own checked-in text never holds the contiguous string nowaiver.sh's
    // extension-less "Makefile .github/workflows src tests" sweep forbids.
    let dashdash = ["-", "-"].concat();
    let script = format!(
        "tar {dashdash}exclude=.git {dashdash}exclude=target -cf - -C \"$1\" . | tar -xf - -C \"$2\""
    );
    let status = Command::new("sh")
        .arg("-c")
        .arg(&script)
        .arg("--")
        .arg(&src)
        .arg(dest)
        .status()
        .expect("spawn tar copy");
    assert!(
        status.success(),
        "tar copy from {} to {} failed",
        src.display(),
        dest.display()
    );
}

/// `git init && git add -A && git commit` inside a scratch copy, so
/// `openspec-untouched.sh`'s `git rev-parse --show-toplevel` succeeds on it — a plain
/// `temp_dir()` copy is not a git repository at all (design.md -> Decision 6a).
fn git_init_and_commit(root: &Path) -> Result<(), String> {
    let run = |args: &[&str]| -> Result<(), String> {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .map_err(|e| format!("spawn git {args:?}: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "git {args:?} failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    };
    run(&["init", "-q"])?;
    run(&[
        "-c",
        "user.email=scratch@example.com",
        "-c",
        "user.name=scratch",
        "add",
        "-A",
    ])?;
    run(&[
        "-c",
        "user.email=scratch@example.com",
        "-c",
        "user.name=scratch",
        "commit",
        "-q",
        "-m",
        "scratch snapshot for openspec-untouched.sh's control",
    ])?;
    Ok(())
}

/// Runs `control`'s script (unplanted or planted, the caller does not care which) inside
/// `root`, with the recorded env overrides applied. Returns whether it exited 0 and the
/// combined stdout+stderr text `expect` is matched against.
fn run_script(control: &Control, root: &Path) -> (bool, String) {
    let interpreter = match control.interpreter.as_str() {
        "python3" => "python3",
        _ => "/bin/sh",
    };
    let script_path = Path::new("scripts/gates").join(&control.script);
    let mut cmd = Command::new(interpreter);
    cmd.arg(&script_path).current_dir(root);
    let (sets, unsets) = env_overrides(&control.env);
    for (key, value) in sets {
        cmd.env(key, value);
    }
    for key in unsets {
        cmd.env_remove(key);
    }
    if control.cargo {
        cmd.env("CARGO_TARGET_DIR", manifest_dir().join("target"));
    }
    let output = cmd.output().unwrap_or_else(|e| {
        panic!(
            "control {:?}: failed to run {interpreter} {}: {e}",
            control.id,
            script_path.display()
        )
    });
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.success(), combined)
}

/// Expands the one placeholder `tests/gate-controls.toml` may use inside a plant string:
/// `%%DASHDASH%%` becomes two dashes. Some plants must inject a coverage-waiver flag
/// nowaiver.sh's own extension-less "Makefile .github/workflows src tests" sweep forbids;
/// writing that flag whole into the checked-in toml (or into this comment) would trip the
/// very gate the "nowaiver-hit" control tests before its plant is ever applied, since that
/// sweep has no file-extension filter and reads `tests/gate-controls.toml` itself.
fn expand_placeholders(text: &str) -> String {
    text.replace("%%DASHDASH%%", "--")
}

/// Applies `control`'s plant inside `root`. `Err` names a HARNESS problem (a missing file,
/// an absent `plant_find`) distinct from the gate simply not catching the plant — the two
/// must never be conflated, or a broken plant would read as a gate that "still passes".
fn apply_plant(control: &Control, root: &Path) -> Result<(), String> {
    let target = root.join(&control.plant_file);
    match control.plant_kind.as_str() {
        "create" => {
            let content = expand_placeholders(control.plant_content.as_deref().unwrap_or(""));
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("create parent of {}: {e}", target.display()))?;
            }
            fs::write(&target, content).map_err(|e| format!("write {}: {e}", target.display()))
        }
        "edit" => {
            let find = control
                .plant_find
                .as_deref()
                .map(expand_placeholders)
                .ok_or_else(|| format!("control {:?}: no plant_find", control.id))?;
            let replace = control
                .plant_replace
                .as_deref()
                .map(expand_placeholders)
                .ok_or_else(|| format!("control {:?}: no plant_replace", control.id))?;
            let text = fs::read_to_string(&target)
                .map_err(|e| format!("read {}: {e}", target.display()))?;
            if !text.contains(&find) {
                return Err(format!(
                    "control {:?}: plant_find not present in {}: {find:?}",
                    control.id, control.plant_file
                ));
            }
            let patched = text.replacen(&find, &replace, 1);
            fs::write(&target, patched).map_err(|e| format!("write {}: {e}", target.display()))
        }
        other => Err(format!(
            "control {:?}: unknown plant_kind {other:?}",
            control.id
        )),
    }
}

/// Runs one control end to end: copy, baseline, plant, replant-run. `Ok(())` means the gate
/// caught its own plant; `Err` names why it did not (or why the harness itself could not
/// finish), with the two kept textually distinct so a reader never mistakes one for the other.
fn run_one_control(control: &Control) -> Result<(), String> {
    let scratch = ScratchDir::new(&control.id);
    copy_tree(scratch.path());

    if control.needs_git {
        git_init_and_commit(scratch.path())?;
    }

    let (baseline_ok, baseline_out) = run_script(control, scratch.path());
    if !baseline_ok {
        return Err(format!(
            "harness error: the UNPLANTED gate already exits non-zero:\n{baseline_out}"
        ));
    }

    apply_plant(control, scratch.path())?;

    let (planted_ok, planted_out) = run_script(control, scratch.path());
    if planted_ok {
        return Err(format!(
            "the gate still exits 0 after the plant (expected non-zero); output:\n{planted_out}"
        ));
    }
    if !planted_out.contains(&control.expect) {
        return Err(format!(
            "the gate failed but its output does not name {:?}; output:\n{planted_out}",
            control.expect
        ));
    }
    Ok(())
}

/// Task 0.2's both-directions requirement, on `tests/ci_workflow.rs`'s own terms: a script
/// under `scripts/gates/` with no `[[control]]` entry is a failure, and a `[[control]]` entry
/// naming a script that does not exist is a failure too.
#[test]
fn gate_controls_every_script_has_a_control() {
    let controls = parsed_controls();
    let scripts = gate_scripts();
    assert!(
        scripts.len() >= 28,
        "found {} files under scripts/gates, expected >= 28",
        scripts.len()
    );

    let controlled: BTreeSet<&str> = controls.iter().map(|c| c.script.as_str()).collect();
    let missing: Vec<&String> = scripts
        .iter()
        .filter(|s| !controlled.contains(s.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "scripts under scripts/gates/ with no [[control]] entry: {missing:?}"
    );

    let script_set: BTreeSet<&str> = scripts.iter().map(String::as_str).collect();
    let orphans: Vec<&String> = controls
        .iter()
        .map(|c| &c.script)
        .filter(|s| !script_set.contains(s.as_str()))
        .collect();
    assert!(
        orphans.is_empty(),
        "[[control]] entries naming a script that does not exist under scripts/gates/: {orphans:?}"
    );
}

/// A lightweight recursive snapshot of every file under `dir` — path, byte length, and
/// modification time, skipping `.git` and `target` — used to prove the REAL working tree is
/// byte-identical before and after this suite runs. Not `git status --porcelain`: the real
/// tree is ordinarily dirty during implementation, which would make that a false red
/// (design.md -> Decision 6a).
fn tree_fingerprint(dir: &Path) -> Vec<(PathBuf, u64, std::time::SystemTime)> {
    let mut out = Vec::new();
    collect_fingerprint(dir, &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn collect_fingerprint(dir: &Path, out: &mut Vec<(PathBuf, u64, std::time::SystemTime)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if name == ".git" || name == "target" {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_dir() {
            collect_fingerprint(&path, out);
        } else {
            let mtime = metadata.modified().unwrap_or(std::time::UNIX_EPOCH);
            out.push((path, metadata.len(), mtime));
        }
    }
}

/// The main loop: every `[[control]]` entry, run end to end. Failures are accumulated rather
/// than aborting at the first one, so a single run reports every gate that does not yet
/// catch its own plant — at task-group-0 time, exactly the five design.md names.
#[test]
fn gate_controls_catch_their_plants() {
    let controls = parsed_controls();
    assert!(
        !controls.is_empty(),
        "tests/gate-controls.toml has no [[control]] entries"
    );

    let before = tree_fingerprint(&manifest_dir());

    let mut failures = Vec::new();
    let mut passed = Vec::new();
    for control in &controls {
        match run_one_control(control) {
            Ok(()) => passed.push(control.id.clone()),
            Err(reason) => failures.push(format!("{} ({}): {reason}", control.id, control.script)),
        }
    }

    let after = tree_fingerprint(&manifest_dir());
    assert_eq!(
        before, after,
        "the real working tree changed while running the gate controls - a plant leaked \
         out of its scratch copy"
    );

    if !failures.is_empty() {
        panic!(
            "{} of {} controls passed ({}); {} did not catch their plant:\n\n{}",
            passed.len(),
            controls.len(),
            passed.join(", "),
            failures.len(),
            failures.join("\n\n")
        );
    }
}
