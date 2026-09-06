//! Guards that every capability spec under `openspec/specs/` carries a written Purpose,
//! and that the guard itself cannot pass vacuously. `openspec validate --specs --strict` is
//! the authority on the rule, but needs `node` and the `openspec` binary — optional for
//! this plugin and absent from both CI runners — so this test is what actually runs on
//! every `cargo test`. See `openspec/changes/spec-purposes/design.md` -> Decisions -> 5.

use std::fs;
use std::path::{Path, PathBuf};

/// The exact placeholder `openspec archive` writes into a freshly created capability.
const PLACEHOLDER: &str = "TBD - created by archiving change";

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn specs_dir() -> PathBuf {
    manifest_dir().join("openspec/specs")
}

/// Extracts the body of the `## Purpose` section from a capability's `spec.md` source:
/// every line strictly between the `## Purpose` heading and the next `## `-level heading
/// (or EOF). A spec with no `## Purpose` heading at all yields an empty body, which the
/// caller treats the same as an empty Purpose.
fn purpose_body(spec_md: &str) -> String {
    let mut body = String::new();
    let mut in_purpose = false;
    for line in spec_md.lines() {
        if line.starts_with("## ") {
            in_purpose = line == "## Purpose";
            continue;
        }
        if in_purpose {
            body.push_str(line);
            body.push('\n');
        }
    }
    body
}

/// Walks every capability directory directly under `dir` (sorted, so a failure is
/// reproducible), reads its `spec.md`, and checks the `## Purpose` section. Returns the
/// number of capabilities checked on success, or `Err` naming the first capability whose
/// Purpose is empty or still holds the archiver's placeholder — or naming the count found
/// (with the minimum required) when `dir` holds no capability directory at all, so the
/// check cannot pass by finding nothing to check.
fn check_all_purposes(dir: &Path) -> Result<usize, String> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|e| format!("failed to read {}: {e}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    entries.sort();

    if entries.is_empty() {
        return Err(format!(
            "found 0 capability directories under {} (minimum 1)",
            dir.display()
        ));
    }

    for capability_dir in &entries {
        let name = capability_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unknown>")
            .to_string();
        let spec_path = capability_dir.join("spec.md");
        let content = fs::read_to_string(&spec_path)
            .map_err(|e| format!("{name}: failed to read {}: {e}", spec_path.display()))?;
        let body = purpose_body(&content);
        let trimmed = body.trim();
        if trimmed.is_empty() {
            return Err(format!(
                "{name}: `## Purpose` body is empty — the Purpose must be written in the \
                 main spec"
            ));
        }
        if trimmed.contains(PLACEHOLDER) {
            return Err(format!(
                "{name}: `## Purpose` still holds the archiver's placeholder ({PLACEHOLDER}) \
                 — the Purpose must be written in the main spec, because a `## Purpose` in a \
                 delta is read only when the capability is created"
            ));
        }
    }

    Ok(entries.len())
}

/// A scratch directory under `std::env::temp_dir()`, removed recursively on drop.
/// Reimplemented here rather than shared, because `tests/spec_purposes.rs` is a separate
/// integration-test crate and cannot reach the library's `pub(crate)` items — the same
/// reason `tests/cli.rs` carries its own copy.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new() -> Self {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let counter = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "herdr-openspec-spec-purposes-test-{}-{counter}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create scratch dir");
        Self { path }
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn every_capability_has_a_written_purpose() {
    match check_all_purposes(&specs_dir()) {
        Ok(n) => assert!(n >= 1, "expected at least one capability, found 0"),
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn an_archived_placeholder_fails_the_test() {
    let scratch = ScratchDir::new();
    let cap = scratch.path.join("some-capability");
    fs::create_dir_all(&cap).unwrap();
    fs::write(
        cap.join("spec.md"),
        "# some-capability Specification\n\n## Purpose\nTBD - created by archiving change some-change\n\n## Requirements\n",
    )
    .unwrap();

    let result = check_all_purposes(&scratch.path);
    let err = result.expect_err("a fresh archiver placeholder must fail the check");
    assert!(
        err.contains("some-capability"),
        "failure must name the capability by directory name: {err}"
    );
    assert!(
        err.contains("main spec"),
        "failure must state the Purpose belongs in the main spec, not a delta: {err}"
    );
}

#[test]
fn the_test_cannot_pass_vacuously() {
    // `scratch.path` exists but holds no capability subdirectory.
    let scratch = ScratchDir::new();

    let result = check_all_purposes(&scratch.path);
    let err = result.expect_err("an empty directory must not report success");
    assert!(
        err.contains("found 0") && err.contains("minimum 1"),
        "failure must name the count found and the minimum required: {err}"
    );
}
