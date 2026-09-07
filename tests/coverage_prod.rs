//! Drives `scripts/coverage-prod.py` — the production-slice coverage floor — against
//! checked-in fixture reports under `tests/fixtures/coverage/`, and against synthetic
//! reports built at test time over this crate's own real `src/*.rs` files.
//!
//! See `openspec/changes/gate-integrity/design.md` -> Decision 1/1a and the
//! `quality-gates` spec's "The coverage floor is measured against production code" for the
//! contract this file exercises. RED at task 1.1 time: `scripts/coverage-prod.py` does not
//! exist yet.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn script_path() -> PathBuf {
    manifest_dir().join("scripts/coverage-prod.py")
}

fn fixtures_dir() -> PathBuf {
    manifest_dir().join("tests/fixtures/coverage")
}

/// Runs the checker against `report`, with an optional `PROD_MIN` override. Returns
/// whether it exited 0 and the combined stdout+stderr text.
///
/// Passes no second (covers-map) argument at all — gate-integrity task 6.4's addition, the
/// `covers`-range check, runs only when a map path is named on the command line, and every
/// test above this comment drives the checker with a small, ad hoc report that names none of
/// the real `tests/degraded-coverage.toml`'s `src/*.rs` files. Without omitting the argument,
/// every one of them would fail on the covers check rather than on whatever the test itself
/// means to exercise. `run_checker_with_covers_toml` below is the one function that DOES pass
/// a map, for exactly the tests that mean to exercise that check. The real `make coverage`
/// invocation always names `tests/degraded-coverage.toml` as this argument, so the covers
/// check is unconditional there — the spec's own requirement.
fn run_checker(report: &Path, prod_min: Option<&str>) -> (bool, String) {
    let mut cmd = Command::new("python3");
    cmd.arg(script_path())
        .arg(report)
        .current_dir(manifest_dir());
    if let Some(min) = prod_min {
        cmd.env("PROD_MIN", min);
    }
    let output = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn python3 {}: {e}", script_path().display()));
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.success(), combined)
}

/// Runs the checker against `report`, pointed at `toml_path` via the checker's second
/// positional argument (task 6.4's map path, on `run_checker`'s own terms) rather than the
/// real `tests/degraded-coverage.toml`, and with the production floor disabled (`PROD_MIN=0`)
/// so only the covers-range check can fail these tests. Used by the covers-range scenarios
/// below, never by the production-floor tests above.
fn run_checker_with_covers_toml(report: &Path, toml_path: &Path) -> (bool, String) {
    let mut cmd = Command::new("python3");
    cmd.arg(script_path())
        .arg(report)
        .arg(toml_path)
        .env("PROD_MIN", "0")
        .current_dir(manifest_dir());
    let output = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn python3 {}: {e}", script_path().display()));
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.success(), combined)
}

fn fixture(name: &str) -> PathBuf {
    fixtures_dir().join(name)
}

#[test]
fn healthy_report_passes() {
    let (ok, out) = run_checker(&fixture("healthy.json"), None);
    assert!(ok, "expected the healthy fixture to pass:\n{out}");
    assert!(
        out.contains("COVERAGE-PROD OK"),
        "expected an OK line in output:\n{out}"
    );
}

#[test]
fn one_module_zeroed_fails() {
    let (ok, out) = run_checker(&fixture("one_module_zeroed.json"), None);
    assert!(
        !ok,
        "expected the fixture with production lines zeroed to fail:\n{out}"
    );
}

#[test]
fn empty_report_fails() {
    let (ok, out) = run_checker(&fixture("empty.json"), None);
    assert!(
        !ok,
        "expected an empty report (no files at all) to fail rather than report 100% of nothing:\n{out}"
    );
}

#[test]
fn src_less_report_fails() {
    let (ok, out) = run_checker(&fixture("no_src.json"), None);
    assert!(
        !ok,
        "expected a report naming no file under src/ to fail:\n{out}"
    );
}

#[test]
fn malformed_report_fails() {
    let (ok, out) = run_checker(&fixture("malformed.json"), None);
    assert!(!ok, "expected an unparseable report to fail:\n{out}");
}

#[test]
fn absent_report_fails() {
    let (ok, out) = run_checker(&fixture("does-not-exist.json"), None);
    assert!(!ok, "expected an absent report path to fail:\n{out}");
}

#[test]
fn zero_production_lines_fails() {
    let (ok, out) = run_checker(&fixture("zero_prod_lines.json"), None);
    assert!(
        !ok,
        "expected a report whose production-line count is zero to fail:\n{out}"
    );
}

/// Extracts an integer following `key=` on a line naming `filename`, from the checker's
/// `COVERAGE-PROD FILE <filename> extents=N prod_instrumented=N ...` output. Panics with
/// the full output if the file's line, or the key on it, cannot be found — a missing line
/// is a harness problem, not a passing/failing result to fall back on silently.
fn extract_metric(out: &str, filename: &str, key: &str) -> u64 {
    let prefix = format!("{key}=");
    for line in out.lines() {
        if !line.contains(filename) {
            continue;
        }
        for token in line.split_whitespace() {
            if let Some(value) = token.strip_prefix(&prefix) {
                return value
                    .parse()
                    .unwrap_or_else(|e| panic!("bad {key} value {value:?} in line {line:?}: {e}"));
            }
        }
    }
    panic!("no {key:?} found for {filename:?} in output:\n{out}");
}

/// A scratch temp file removed on drop, used for synthetic reports generated at test time
/// over this crate's own real `src/*.rs` files — those are too large to check in as fixed
/// fixtures, so this file builds them on the fly instead.
struct ScratchFile {
    path: PathBuf,
}

impl ScratchFile {
    fn new(label: &str, contents: &str) -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "herdr-openspec-coverage-prod-{}-{n}-{label}.json",
            std::process::id()
        ));
        fs::write(&path, contents).expect("write scratch report");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Builds a synthetic llvm-cov JSON export naming a single real file (`rel_path`, relative
/// to the repository root) with every one of its lines marked instrumented and covered —
/// enough to exercise the classifier's per-file extent tracking without needing a real
/// `cargo llvm-cov` run.
fn synthetic_all_covered_report(rel_path: &str) -> String {
    let text = fs::read_to_string(manifest_dir().join(rel_path))
        .unwrap_or_else(|e| panic!("read {rel_path} to build synthetic report: {e}"));
    let n = text.lines().count();
    let mut segments = Vec::with_capacity(n);
    for line in 1..=n {
        segments.push(format!("[{line},1,1,true,true,false]"));
    }
    format!(
        r#"{{"data":[{{"files":[{{"filename":"{rel_path}","segments":[{}]}}]}}]}}"#,
        segments.join(",")
    )
}

/// Task 1.2b: the naive "first line-anchored `#[cfg(test)]`" cut would classify
/// `src/changes.rs` as having only 76 production lines (everything from line 77 on is
/// `mod conformance`, a `#[cfg(test)]` item, and the naive cut treats the rest of the file
/// as test too). The correct brace-extent classifier must instead exclude only the actual
/// `#[cfg(test)]` items' extents and count everything else — thousands of lines — as
/// production.
#[test]
fn changes_rs_contributes_its_real_production_body_not_76_lines() {
    let report = synthetic_all_covered_report("src/changes.rs");
    let scratch = ScratchFile::new("changes-rs", &report);
    let (ok, out) = run_checker(scratch.path(), Some("0"));
    assert!(ok, "synthetic all-covered report should pass:\n{out}");

    let extents = extract_metric(&out, "src/changes.rs", "extents");
    let prod_instr = extract_metric(&out, "src/changes.rs", "prod_instrumented");

    assert!(
        extents >= 3,
        "expected at least 3 #[cfg(test)] extents in src/changes.rs, found {extents}:\n{out}"
    );
    assert!(
        prod_instr > 1000,
        "expected src/changes.rs's production line count to be its real (large) body, \
         not the naive first-occurrence cut's 76 lines; got {prod_instr}:\n{out}"
    );
}

#[test]
fn cli_rs_contributes_more_than_the_naive_cut() {
    let report = synthetic_all_covered_report("src/cli.rs");
    let scratch = ScratchFile::new("cli-rs", &report);
    let (ok, out) = run_checker(scratch.path(), Some("0"));
    assert!(ok, "synthetic all-covered report should pass:\n{out}");

    let extents = extract_metric(&out, "src/cli.rs", "extents");
    let prod_instr = extract_metric(&out, "src/cli.rs", "prod_instrumented");

    assert!(
        extents >= 10,
        "expected at least 10 #[cfg(test)] extents in src/cli.rs, found {extents}:\n{out}"
    );
    // The naive cut takes everything before line 317 as production (~300 lines) and
    // discards the rest; the real production body extends past the test modules too.
    assert!(
        prod_instr > 300,
        "expected src/cli.rs's production line count to exceed the naive cut's ~300 lines; \
         got {prod_instr}:\n{out}"
    );
}

#[test]
fn lib_rs_contributes_more_than_the_naive_cut() {
    let report = synthetic_all_covered_report("src/lib.rs");
    let scratch = ScratchFile::new("lib-rs", &report);
    let (ok, out) = run_checker(scratch.path(), Some("0"));
    assert!(ok, "synthetic all-covered report should pass:\n{out}");

    let extents = extract_metric(&out, "src/lib.rs", "extents");
    let prod_instr = extract_metric(&out, "src/lib.rs", "prod_instrumented");

    assert!(
        extents >= 2,
        "expected at least 2 #[cfg(test)] extents in src/lib.rs, found {extents}:\n{out}"
    );
    // The naive cut takes everything before line 37 (`mod testutil`) as production; the
    // real body includes every production module declared after it (`pub mod changes`,
    // `pub mod cli`, ... at lines 9-20, all before line 37, plus `pid()` at line ~30).
    // The point of this test is that the classifier does not stop at the naive cut's
    // ~36 lines once a later, larger `#[cfg(test)]` module (line 987) is also present.
    assert!(
        prod_instr > 36,
        "expected src/lib.rs's production line count to exceed the naive cut's ~36 lines; \
         got {prod_instr}:\n{out}"
    );
}

/// Task 1.2b's negative control: move a production function into a `#[cfg(test)]` module
/// (here, `tests/fixtures/coverage/src/sample_moved.rs` — `sample.rs` with `#[cfg(test)]`
/// added directly above `pub fn sub`) and require the reported production line count to
/// fall, using checked-in fixture reports that mark the same two candidate lines in each
/// file (shifted by the one inserted attribute line).
#[test]
fn moving_a_production_fn_into_cfg_test_lowers_the_count() {
    let (before_ok, before_out) = run_checker(&fixture("move_control_before.json"), Some("0"));
    assert!(before_ok, "before-move report should pass:\n{before_out}");
    let before = extract_metric(
        &before_out,
        "tests/fixtures/coverage/src/sample.rs",
        "prod_instrumented",
    );

    let (after_ok, after_out) = run_checker(&fixture("move_control_after.json"), Some("0"));
    assert!(after_ok, "after-move report should pass:\n{after_out}");
    let after = extract_metric(
        &after_out,
        "tests/fixtures/coverage/src/sample_moved.rs",
        "prod_instrumented",
    );

    assert_eq!(
        before, 2,
        "sanity: both candidate lines start out production:\n{before_out}"
    );
    assert!(
        after < before,
        "moving a production fn into #[cfg(test)] must lower the production line count: \
         before={before} after={after}\n{before_out}\n{after_out}"
    );
}

/// The floor is exceedable: running with `PROD_MIN` far above the healthy fixture's
/// figure must fail, proving the threshold is enforced rather than merely printed.
#[test]
fn raising_the_floor_above_measurement_fails() {
    let (ok, out) = run_checker(&fixture("healthy.json"), Some("100.5"));
    assert!(
        !ok,
        "expected a floor above the measured figure to fail:\n{out}"
    );
}

// --- gate-integrity Change Review, task 9.3 (CRITICAL 2): DEFAULT_PROD_MIN itself --------

/// Reads `DEFAULT_PROD_MIN` straight out of `scripts/coverage-prod.py`'s own source, rather
/// than transcribing a copy of the number here - a copy would drift from the real constant
/// exactly the way `specs/quality-gates/spec.md`'s own transcribed figures drifted from the
/// checker's real output (Change Review W-C).
fn default_prod_min() -> u32 {
    let text = fs::read_to_string(script_path()).expect("read scripts/coverage-prod.py");
    for line in text.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("DEFAULT_PROD_MIN = ") {
            return rest
                .trim()
                .parse()
                .unwrap_or_else(|e| panic!("bad DEFAULT_PROD_MIN {rest:?}: {e}"));
        }
    }
    panic!(
        "DEFAULT_PROD_MIN not found in {}",
        script_path().display()
    );
}

/// Every real `.rs` file under `src/`, found by walking the directory rather than shelling
/// out to `find` - this crate's own convention elsewhere in this file already reads real
/// `src/*.rs` files directly (`synthetic_all_covered_report`), just one file at a time.
fn all_src_rs_files() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(&manifest_dir().join("src"), &mut out);
    out.sort();
    out
}

/// The whole-tree extension of `synthetic_all_covered_report`: every real `.rs` file under
/// `src/`, in one report, every line marked instrumented and covered. Run through the
/// checker, this yields the HIGHEST production percentage any report can ever show under
/// the checker's own classifier - not a hardcoded transcription of it, so a change to
/// `classify_file`'s rule keeps this ceiling honest without anyone updating a copied number.
fn synthetic_all_covered_report_for_tree() -> String {
    let root = manifest_dir();
    let mut files_json = Vec::new();
    for path in all_src_rs_files() {
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let n = text.lines().count();
        let mut segments = Vec::with_capacity(n);
        for line in 1..=n {
            segments.push(format!("[{line},1,1,true,true,false]"));
        }
        files_json.push(format!(
            r#"{{"filename":"{rel}","segments":[{}]}}"#,
            segments.join(",")
        ));
    }
    format!(r#"{{"data":[{{"files":[{}]}}]}}"#, files_json.join(","))
}

/// Extracts the percentage following "COVERAGE-PROD OK: production " from the checker's own
/// output line, e.g. "COVERAGE-PROD OK: production 100.00% (...)".
fn extract_prod_pct(out: &str) -> f64 {
    for line in out.lines() {
        if let Some(rest) = line.strip_prefix("COVERAGE-PROD OK: production ") {
            if let Some(pct_str) = rest.split('%').next() {
                return pct_str.trim().parse().unwrap_or_else(|e| {
                    panic!("bad production percentage {pct_str:?} in line {line:?}: {e}")
                });
            }
        }
    }
    panic!("no \"COVERAGE-PROD OK: production\" line found in output:\n{out}");
}

/// CRITICAL 2: `DEFAULT_PROD_MIN` was bound to nothing - a typo or a deliberate edit could
/// gut it to near-zero with every gate staying green (the covering `NOWAIVER` leg above
/// only guards the Makefile's own override, not this constant). A degenerate low floor
/// (well below anything this checker has ever measured) must fail this sanity test.
#[test]
fn default_prod_min_is_not_a_degenerate_floor() {
    let min = default_prod_min();
    assert!(
        min >= 50,
        "DEFAULT_PROD_MIN={min} is below a sane sentinel - a floor this low would let \
         production coverage collapse by more than half before make check ever noticed"
    );
}

/// CRITICAL 2's other half: `DEFAULT_PROD_MIN` must not exceed the checker's OWN
/// measurement of what a report can ever show - not a hardcoded ceiling, but the real
/// output of running `scripts/coverage-prod.py` against a synthetic report marking every
/// real `src/*.rs` line covered. No real (execution-driven) report can ever score higher
/// than this idealised one, so a `DEFAULT_PROD_MIN` above it could never be satisfied by
/// any real run - exactly `specs/quality-gates/spec.md`'s "rounded down ... and SHALL NOT
/// be lower" contract read from the other direction.
#[test]
fn default_prod_min_does_not_exceed_the_checkers_own_measurement() {
    let report = synthetic_all_covered_report_for_tree();
    let scratch = ScratchFile::new("whole-tree-all-covered", &report);
    let (ok, out) = run_checker(scratch.path(), Some("0"));
    assert!(ok, "synthetic all-covered whole-tree report should pass:\n{out}");

    let ceiling = extract_prod_pct(&out);
    let min = default_prod_min();
    assert!(
        f64::from(min) <= ceiling + 0.01,
        "DEFAULT_PROD_MIN={min} exceeds {ceiling:.2}%, the checker's own maximum possible \
         measurement (every real src/*.rs line marked covered) - no real report could ever \
         satisfy a floor set above this"
    );
}

// --- gate-integrity task 6.3b/6.4: the `covers`-range check -------------------------------

/// Two rows' worth of `covers` ranges, both healthy in `healthy.json`, must pass.
#[test]
fn covers_ranges_all_covered_pass() {
    let (ok, out) = run_checker_with_covers_toml(
        &fixture("healthy.json"),
        &fixture("degraded-coverage-two-rows.toml"),
    );
    assert!(ok, "expected both healthy covers ranges to pass:\n{out}");
}

/// The scenario the requirement is written to force: a fixture report with one row's
/// `covers` range zeroed (here, `sub`'s lines 5-6 — `add`'s lines 1-2 stay covered) must
/// fail, naming that row's own `condition` in the failure message.
#[test]
fn a_zeroed_covers_range_fails_naming_the_row_condition() {
    let (ok, out) = run_checker_with_covers_toml(
        &fixture("degraded_covers_one_zeroed.json"),
        &fixture("degraded-coverage-two-rows.toml"),
    );
    assert!(
        !ok,
        "expected the zeroed range to fail the covers check:\n{out}"
    );
    assert!(
        out.contains("Fixture: sub is exercised"),
        "the failure must name the row whose range is cold:\n{out}"
    );
    assert!(
        !out.contains("Fixture: add is exercised"),
        "the still-covered row must not be named as a failure:\n{out}"
    );
}

/// A row whose `covers` array is empty must fail rather than silently reporting the map's
/// one remaining range as full coverage — the same reduction "fewer ranges than rows"
/// below states as a floor.
#[test]
fn an_empty_covers_array_fails_vacuously() {
    let (ok, out) = run_checker_with_covers_toml(
        &fixture("healthy.json"),
        &fixture("degraded-coverage-empty-covers.toml"),
    );
    assert!(
        !ok,
        "expected an empty covers array to fail rather than pass vacuously:\n{out}"
    );
}

/// A report naming none of the map's `covers` paths at all must fail every row, not
/// report 100% of nothing.
#[test]
fn a_report_naming_none_of_the_covers_paths_fails() {
    let (ok, out) = run_checker_with_covers_toml(
        &fixture("no_src.json"),
        &fixture("degraded-coverage-two-rows.toml"),
    );
    assert!(
        !ok,
        "expected a report naming none of the ranges' paths to fail:\n{out}"
    );
    assert!(
        out.contains("Fixture: add is exercised") || out.contains("no such file"),
        "the failure must come from the covers check itself, not merely the unrelated \
         \"no file under src/\" floor check that also fires on this report: {out:?}"
    );
}

/// The map itself holding fewer `covers` ranges than `[[row]]` entries — dropping a range —
/// is exactly what `degraded-coverage-empty-covers.toml` does (two rows, one range): this
/// asserts the failure fires on the count mismatch, independent of the empty-array wording
/// above.
#[test]
fn fewer_ranges_than_rows_fails() {
    let (ok, out) = run_checker_with_covers_toml(
        &fixture("healthy.json"),
        &fixture("degraded-coverage-empty-covers.toml"),
    );
    assert!(
        !ok,
        "expected fewer covers ranges than rows to fail:\n{out}"
    );
}
