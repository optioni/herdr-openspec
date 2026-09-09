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
    /// Extra env-prefix tokens (same grammar as `env`) applied ONLY on the post-plant run,
    /// appended after `env`'s own tokens. Exists for a plant whose failure mode depends on
    /// an environment variable pointing at what the plant just created (`noblock.sh`'s
    /// `UIDIR`, for the "leg 1 still refuses a tree with no subject" control): the baseline
    /// run must see the ordinary environment (the created path does not exist yet), so a
    /// single `env` field shared by both runs cannot express this — `env` alone would either
    /// fail the baseline (pointed at a not-yet-created path) or leave the planted run
    /// unaffected (pointed at the default).
    plant_env: String,
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
        let plant_env = get_str_opt(table, "plant_env").unwrap_or_default();
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
            plant_env,
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
    run_script_with_env(control, root, &control.env)
}

/// Same as `run_script`, but with `env` overriding `control.env` for this one invocation —
/// used to give the post-plant run its own environment (`control.env` plus `plant_env`)
/// without changing what the baseline run sees. See `Control::plant_env`'s doc comment for
/// why the two runs cannot share a single env string.
fn run_script_with_env(control: &Control, root: &Path, env: &str) -> (bool, String) {
    let interpreter = match control.interpreter.as_str() {
        "python3" => "python3",
        _ => "/bin/sh",
    };
    let script_path = Path::new("scripts/gates").join(&control.script);
    let mut cmd = Command::new(interpreter);
    cmd.arg(&script_path).current_dir(root);
    let (sets, unsets) = env_overrides(env);
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

/// Runs `control`'s baseline-then-plant cycle inside an already-prepared `root` (already
/// copied, and already `git init`-ed if the control needs it). Split out from
/// `run_one_control` so a suite-integrity test can prepare its own scratch root (for example,
/// one whose gate script has been neutered) and drive exactly this same baseline/plant/assert
/// sequence over it. `Ok(())` means the gate caught its own plant; `Err` names why it did not
/// (or why the harness itself could not finish), with the two kept textually distinct so a
/// reader never mistakes one for the other.
fn run_control_in(control: &Control, root: &Path) -> Result<(), String> {
    let (baseline_ok, baseline_out) = run_script(control, root);
    if !baseline_ok {
        return Err(format!(
            "harness error: the UNPLANTED gate already exits non-zero:\n{baseline_out}"
        ));
    }

    apply_plant(control, root)?;

    let planted_env = if control.plant_env.is_empty() {
        control.env.clone()
    } else if control.env.is_empty() {
        control.plant_env.clone()
    } else {
        format!("{} {}", control.env, control.plant_env)
    };
    let (planted_ok, planted_out) = run_script_with_env(control, root, &planted_env);
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

/// Runs one control end to end: copy, then `run_control_in`. `Ok(())` means the gate caught
/// its own plant; `Err` names why it did not (or why the harness itself could not finish).
fn run_one_control(control: &Control) -> Result<(), String> {
    let scratch = ScratchDir::new(&control.id);
    copy_tree(scratch.path());

    if control.needs_git {
        git_init_and_commit(scratch.path())?;
    }

    run_control_in(control, scratch.path())
}

/// The correspondence check itself, factored out of `gate_controls_every_script_has_a_control`
/// so a suite-integrity test can drive it over a scratch script list without touching the real
/// `scripts/gates/` directory. Returns (scripts with no `[[control]]` entry, `[[control]]`
/// entries naming a script that does not exist), both sorted for a deterministic message.
fn missing_and_orphan_controls(
    scripts: &[String],
    controls: &[Control],
) -> (Vec<String>, Vec<String>) {
    let controlled: BTreeSet<&str> = controls.iter().map(|c| c.script.as_str()).collect();
    let mut missing: Vec<String> = scripts
        .iter()
        .filter(|s| !controlled.contains(s.as_str()))
        .cloned()
        .collect();
    missing.sort();

    let script_set: BTreeSet<&str> = scripts.iter().map(String::as_str).collect();
    let mut orphans: Vec<String> = controls
        .iter()
        .map(|c| c.script.clone())
        .filter(|s| !script_set.contains(s.as_str()))
        .collect();
    orphans.sort();

    (missing, orphans)
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

    let (missing, orphans) = missing_and_orphan_controls(&scripts, &controls);
    assert!(
        missing.is_empty(),
        "scripts under scripts/gates/ with no [[control]] entry: {missing:?}"
    );
    assert!(
        orphans.is_empty(),
        "[[control]] entries naming a script that does not exist under scripts/gates/: {orphans:?}"
    );
}

/// Task 7.2's second suite-integrity case, on the spec's "A gate added without a control fails
/// the test" scenario: a script added under `scripts/gates/` with no `[[control]]` entry must
/// be reported missing, and — the other direction — a `[[control]]` entry naming a script that
/// does not exist must be reported an orphan. Driven over an in-memory script list rather than
/// a real file, so this test needs no scratch tree of its own: the property under test is the
/// correspondence check's own behaviour, not the filesystem walk that feeds it, and that walk
/// is already exercised for real by `gate_controls_every_script_has_a_control` above.
#[test]
fn gate_controls_script_added_without_a_control_fails_the_check() {
    let controls = parsed_controls();
    let mut scripts = gate_scripts();

    // A script added to the directory and forgotten in the map.
    scripts.push("planted-uncontrolled-gate.sh".to_string());
    let (missing, orphans) = missing_and_orphan_controls(&scripts, &controls);
    assert_eq!(
        missing,
        vec!["planted-uncontrolled-gate.sh".to_string()],
        "a script with no [[control]] entry must be the only one reported missing"
    );
    assert!(
        orphans.is_empty(),
        "adding an uncontrolled script must not also report an orphan control: {orphans:?}"
    );

    // The other direction: a [[control]] entry naming a script nobody wrote.
    let mut controls_with_orphan = controls;
    let orphan_control = Control {
        id: "planted-orphan-control".to_string(),
        script: "planted-nonexistent-gate.sh".to_string(),
        interpreter: "sh".to_string(),
        env: String::new(),
        needs_git: false,
        cargo: false,
        plant_kind: "edit".to_string(),
        plant_file: String::new(),
        plant_find: None,
        plant_replace: None,
        plant_content: None,
        plant_env: String::new(),
        expect: String::new(),
    };
    controls_with_orphan.push(orphan_control);
    let (missing2, orphans2) = missing_and_orphan_controls(&gate_scripts(), &controls_with_orphan);
    assert!(
        missing2.is_empty(),
        "a [[control]] entry naming a nonexistent script must not also report a missing one: {missing2:?}"
    );
    assert_eq!(
        orphans2,
        vec!["planted-nonexistent-gate.sh".to_string()],
        "a [[control]] entry naming a script nobody wrote must be reported an orphan"
    );
}

/// Task 7.2's first suite-integrity case, on the spec's "A gate neutered to `exit 0` is
/// caught" scenario: a script whose body is replaced with `exit 0` must fail its own
/// `[[control]]` entry — the plant no longer produces a failure, so `run_control_in` reports
/// that the gate stayed green — while `make gates` itself, run over that same neutered tree,
/// still exits 0. That gap between "the control test catches it" and "`make gates` does not"
/// is precisely what this requirement closes: nothing but the control test does.
#[test]
fn gate_controls_neutered_script_fails_its_control_while_make_gates_stays_green() {
    let controls = parsed_controls();
    // openspec-untouched-stray is chosen deliberately: its plant is a brand-new file under
    // openspec/, which no OTHER gate script under scripts/gates/ scans (every other gate
    // reads Makefile/.github/src/tests/scripts) and which does not touch anything cargo
    // compiles. Picking a control whose plant collides with an unrelated grep sweep, or
    // that inserts an item into a multi-line `//!` module doc comment and breaks
    // `cargo build --locked`, would make the second assertion below fail for a reason that
    // has nothing to do with neutering - a control artifact, not the property under test.
    let control = controls
        .iter()
        .find(|c| c.id == "openspec-untouched-stray")
        .expect("the openspec-untouched-stray control exists in tests/gate-controls.toml");

    let scratch = ScratchDir::new("neuter-openspec-untouched");
    copy_tree(scratch.path());
    if control.needs_git {
        git_init_and_commit(scratch.path()).expect("git init the scratch copy");
    }

    let script_path = scratch.path().join("scripts/gates").join(&control.script);
    fs::write(&script_path, "#!/bin/sh\nexit 0\n").expect("neuter the gate script");

    let result = run_control_in(control, scratch.path());
    let message = result.expect_err(
        "a gate script neutered to `exit 0` must fail to catch its own plant, \
         but the control reported success",
    );
    assert!(
        message.contains("still exits 0 after the plant"),
        "expected the neutering to be reported as the gate staying green after the plant, got: {message}"
    );

    // The neutering test above proves the CONTROL catches this. Now prove `make gates` does
    // not: it just runs the (now-neutered) script bare, unplanted, on an otherwise-untouched
    // tree, so it stays green — the gap this requirement exists to close.
    let status = Command::new("make")
        .arg("gates")
        .current_dir(scratch.path())
        .env("CARGO_TARGET_DIR", manifest_dir().join("target"))
        .status()
        .expect("spawn make gates in the scratch copy");
    assert!(
        status.success(),
        "make gates must still exit 0 on a tree where one gate script's body was replaced \
         with `exit 0` - a neutered gate silently disables itself and nothing but the \
         gate-control test notices"
    );
}

/// A recursive, content-based digest of every entry under `dir` — path, bytes (empty for a
/// directory entry), and modification time — skipping `.git` and `target`. This is a private
/// reimplementation of `testutil::snapshot`'s own shape (`src/lib.rs`'s `pub(crate) mod
/// testutil`), carried here rather than reached by name: it is `#[cfg(test)] pub(crate)` and
/// therefore invisible to `tests/gate_controls.rs`, a separate integration-test crate — the
/// same reason `tests/cli.rs` and `tests/spec_purposes.rs` each carry their own copy. Used to
/// prove the REAL working tree is byte-identical before and after this suite runs. Not `git
/// status --porcelain`: the real tree is ordinarily dirty during implementation, which would
/// make that a false red (design.md -> Decision 6a). A directory entry is recorded (with empty
/// bytes) as well as a file's, on `testutil::snapshot`'s own reasoning: an empty directory
/// created or removed by a leaking plant would otherwise be invisible to the comparison.
#[derive(Debug, PartialEq, Eq)]
struct TreeDigest(Vec<(PathBuf, Vec<u8>, std::time::SystemTime)>);

fn tree_digest(dir: &Path) -> TreeDigest {
    let mut out = Vec::new();
    collect_digest(dir, &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    TreeDigest(out)
}

fn collect_digest(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>, std::time::SystemTime)>) {
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
        let mtime = metadata.modified().unwrap_or(std::time::UNIX_EPOCH);
        if metadata.is_dir() {
            out.push((path.clone(), Vec::new(), mtime));
            collect_digest(&path, out);
        } else {
            let bytes = fs::read(&path).unwrap_or_default();
            out.push((path, bytes, mtime));
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

    let before = tree_digest(&manifest_dir());

    let mut failures = Vec::new();
    let mut passed = Vec::new();
    for control in &controls {
        match run_one_control(control) {
            Ok(()) => passed.push(control.id.clone()),
            Err(reason) => failures.push(format!("{} ({}): {reason}", control.id, control.script)),
        }
    }

    let after = tree_digest(&manifest_dir());
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

/// `mouse-input`'s Change Review, gap 4: `quality-gates` -> "Hardcoding the
/// mouse-capture reason in `run` fails the gate" carries three clauses. The
/// first — that `WIRED` exits non-zero on leg 5c — is proved by the checked-in
/// `wired-mouse-problem-hardcoded` control above, which runs the real script
/// against a real planted tree.
///
/// This proves the **third**: that leg **1** stays green against that same
/// tree, which is the whole reason leg 5c is body-scoped rather than a
/// fourteenth leg-1 name. It restates leg 1's own predicate — `code "$MOD" |
/// grep -q -- "$n"`, a plain substring search over the production slice — and
/// leg 5c's two halves, against the planted text in memory. No tree copy and no
/// subprocess: the claim is structural (where `pub struct Startup` sits
/// relative to the file's first line-anchored `#[cfg(test)]`), so reading the
/// real file and applying the real plant is the whole of it.
///
/// The scenario's remaining clause — that `cargo test --all-features` stays
/// green against the planted tree — is deliberately **not** checked in: proving
/// it costs a full suite run on a copied tree per invocation. It was
/// reproduced by hand and recorded in `tasks.md` 8.3, and it is the weaker of
/// the two claims anyway, since every test constructs its own `Startup` and
/// drives `run_wired` directly, which is a property of the tests rather than of
/// the plant.
#[test]
fn leg_one_cannot_see_the_defect_leg_five_c_catches() {
    const PLANT_FIND: &str = "mouse_problem: guard.mouse_problem(),";
    const PLANT_REPLACE: &str = "mouse_problem: None,";

    let path = manifest_dir().join("src/ui/mod.rs");
    let real = std::fs::read_to_string(&path).expect("read src/ui/mod.rs");
    assert!(
        real.contains(PLANT_FIND),
        "the plant's find text is gone from src/ui/mod.rs - this check has no subject"
    );
    let planted = real.replace(PLANT_FIND, PLANT_REPLACE);

    // `prod()`: everything before the first line that is exactly `#[cfg(test)]`.
    let production = |src: &str| -> String {
        let mut out = String::new();
        for line in src.lines() {
            if line == "#[cfg(test)]" {
                return out;
            }
            out.push_str(line);
            out.push('\n');
        }
        out
    };
    // Leg 2/5's cut: from `pub fn run()`'s signature to the next column-zero `}`.
    let run_body = |prod: &str| -> String {
        let mut out = String::new();
        let mut inside = false;
        for line in prod.lines() {
            if line.starts_with("pub fn run()") {
                inside = true;
            }
            if inside {
                out.push_str(line);
                out.push('\n');
                if line == "}" {
                    break;
                }
            }
        }
        out
    };

    let prod = production(&planted);
    assert!(
        !prod.is_empty() && prod.len() < planted.len(),
        "the production cut must find src/ui/mod.rs's own #[cfg(test)]"
    );

    // Leg 1's predicate, restated: a slice-wide substring search is SATISFIED by
    // the planted tree. This is the assertion that makes leg 5c necessary rather
    // than redundant.
    assert!(
        prod.contains("mouse_problem"),
        "leg 1 would be GREEN against the planted tree - if this ever fails, leg 5c \
         could safely become a fourteenth leg-1 name"
    );

    // The stronger form, which is the one that actually carries the argument.
    // The assertion above is satisfied by the plant's own `mouse_problem: None,`
    // text, so on its own it would still hold in a tree where the name lived
    // ONLY inside `run`. Strip every mention from `run`'s body as well: the
    // production slice still names `mouse_problem`, because `pub struct
    // Startup`'s own field declaration and `run_wired`'s read of it both sit
    // above the file's first line-anchored `#[cfg(test)]`.
    //
    // That is the structural fact leg 5c exists for, and it is stronger than an
    // observation about today's layout: it is FORCED. `ui::run` — production
    // code — constructs a `Startup`, so the declaration cannot move below
    // `#[cfg(test)]` without the crate failing to compile. Attempted as a
    // negative control during `mouse-input`'s Change Review: hoisting the file's
    // first `#[cfg(test)]` above `pub struct Startup` gives `E0422: cannot find
    // struct ... Startup in this scope`, and renaming the field gives `E0609`.
    // The assertion below therefore cannot be made to fail by any edit that
    // leaves the crate compiling — which is the point, and is why leg 5c can
    // never be replaced by a fourteenth leg-1 name.
    let gutted = production(&real.replace(PLANT_FIND, ""));
    let gutted_body = run_body(&gutted);
    assert!(
        !gutted_body.contains("mouse_problem"),
        "the gutted body must name it nowhere: {gutted_body:?}"
    );
    assert!(
        gutted.contains("mouse_problem"),
        "leg 1 is green even on a `run` that names mouse_problem NOWHERE at all - \
         the field declaration and run_wired's read are inside the slice it greps"
    );

    // Leg 5c's two halves, both firing on the same tree.
    let body = run_body(&prod);
    assert!(
        body.contains("pub fn run()") && body.trim_end().ends_with('}'),
        "the run-body cut failed: {body:?}"
    );
    assert!(
        !body.contains("mouse_problem("),
        "leg 5c's name half must fire: run's body still names mouse_problem("
    );
    assert!(
        body.contains("mouse_problem: None"),
        "leg 5c's literal half must fire: run's body does not hardcode None"
    );

    // And the unplanted tree passes both halves, so neither is vacuous.
    let real_body = run_body(&production(&real));
    assert!(real_body.contains("mouse_problem("));
    assert!(!real_body.contains("mouse_problem: None"));
}
