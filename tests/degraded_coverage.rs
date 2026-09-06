//! Binds `SPEC.md`'s "## Degraded states" table to `tests/degraded-coverage.toml`, an
//! executable coverage map: every row names the test that proves it, the tier that test
//! runs at, and the audit's own verdict. See
//! `openspec/changes/degraded-states/specs/degraded-coverage/spec.md` for the full
//! contract this file implements.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The floor `SPEC.md`'s table and the coverage map must each meet — a lower bound, not an
/// equality, so a later change may add a degraded state without touching this number.
const MIN_ROWS: usize = 44;

/// Parse the table between `## Degraded states` and `### No terminal is not a degraded
/// state`, skipping the header and separator rows, taking each remaining line's first
/// pipe-delimited cell trimmed. The same parse task 1.1 ran by hand; this is its executable
/// form. `Err` names why no table could be found at all (never a panic).
fn parse_spec_conditions(spec_md: &str) -> Result<Vec<String>, String> {
    let start = spec_md
        .find("## Degraded states")
        .ok_or_else(|| "SPEC.md has no \"## Degraded states\" heading".to_string())?;
    let after_start = &spec_md[start..];
    let end = after_start
        .find("### No terminal is not a degraded state")
        .map(|i| start + i)
        .unwrap_or(spec_md.len());
    let section = &spec_md[start..end];

    let mut conditions = Vec::new();
    for line in section.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        // The separator row (`|---|---|`, no space after a pipe anywhere) and the header
        // row (`| Condition | Behaviour |`) are both skipped.
        let is_separator = trimmed
            .trim_matches('|')
            .split('|')
            .all(|cell| !cell.is_empty() && cell.chars().all(|c| c == '-' || c == ':'));
        if is_separator {
            continue;
        }
        let first_cell = trimmed
            .trim_start_matches('|')
            .split('|')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        if first_cell == "Condition" {
            continue;
        }
        conditions.push(first_cell);
    }
    Ok(conditions)
}

/// One `[[row]]` entry, exactly as `tests/degraded-coverage.toml` carries it.
#[derive(Debug, Clone)]
struct Row {
    condition: String,
    tier: String,
    proof: Vec<String>,
    verdict: String,
    why: String,
}

/// Parse `tests/degraded-coverage.toml`'s `[[row]]` array via the `toml` crate's own
/// `Table`/`Value` API — no `serde` derive dependency is added; `src/config.rs` already
/// parses TOML this way. `Err` names the first structural problem found.
fn parse_coverage_toml(text: &str) -> Result<Vec<Row>, String> {
    let doc: toml::Table = text
        .parse()
        .map_err(|e| format!("tests/degraded-coverage.toml is not valid TOML: {e}"))?;
    let rows_value = doc
        .get("row")
        .ok_or_else(|| "tests/degraded-coverage.toml has no [[row]] entries".to_string())?;
    let array = rows_value
        .as_array()
        .ok_or_else(|| "tests/degraded-coverage.toml's \"row\" key is not an array".to_string())?;

    let mut rows = Vec::with_capacity(array.len());
    for (index, entry) in array.iter().enumerate() {
        let table = entry
            .as_table()
            .ok_or_else(|| format!("row {index}: not a table"))?;
        let condition = table
            .get("condition")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("row {index}: missing or non-string \"condition\""))?
            .to_string();
        let tier = table
            .get("tier")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("row {index} ({condition:?}): missing or non-string \"tier\""))?
            .to_string();
        let proof_value = table
            .get("proof")
            .ok_or_else(|| format!("row {index} ({condition:?}): missing \"proof\""))?;
        let proof_array = proof_value
            .as_array()
            .ok_or_else(|| format!("row {index} ({condition:?}): \"proof\" is not an array"))?;
        if proof_array.is_empty() {
            return Err(format!("row {index} ({condition:?}): \"proof\" is empty"));
        }
        let mut proof = Vec::with_capacity(proof_array.len());
        for p in proof_array {
            let name = p.as_str().ok_or_else(|| {
                format!("row {index} ({condition:?}): a \"proof\" entry is not a string")
            })?;
            proof.push(name.to_string());
        }
        let verdict = table
            .get("verdict")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                format!("row {index} ({condition:?}): missing or non-string \"verdict\"")
            })?
            .to_string();
        let why = table
            .get("why")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("row {index} ({condition:?}): missing or non-string \"why\""))?
            .to_string();
        rows.push(Row {
            condition,
            tier,
            proof,
            verdict,
            why,
        });
    }
    Ok(rows)
}

/// Every `.rs` file under `dir`, recursively — no new dependency (`walkdir` et al.): a hand
/// rolled recursive walk over `std::fs::read_dir`, matching this repository's other
/// integration tests' own style.
fn rust_files_under(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files_under(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Every source file this crate's proofs may live under: `src/` and `tests/`, per the
/// coverage spec's own condition 4.
fn searchable_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    rust_files_under(&manifest_dir().join("src"), &mut out);
    rust_files_under(&manifest_dir().join("tests"), &mut out);
    out
}

/// The text of every function named `fn <name>(` at column zero (module- or impl-level,
/// never nested), searched across `files` — each occurrence's whole body, from that line to
/// the closing brace at the SAME indentation as the `fn` line itself. More than one
/// occurrence is legal (two files may name a test the same); condition 4 needs only that at
/// least one exists, and condition 5 (for `view`/`outer` tiers) is satisfied if at least one
/// occurrence's body carries a rendering proof.
fn function_bodies(files: &[PathBuf], name: &str) -> Vec<String> {
    let needle_owned = format!("fn {name}(");
    let mut bodies = Vec::new();
    for path in files {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        let lines: Vec<&str> = text.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];
            let trimmed_start = line.trim_start();
            if trimmed_start.starts_with(&needle_owned) {
                let indent = line.len() - trimmed_start.len();
                let close = format!("{}}}", " ".repeat(indent));
                let mut body = String::new();
                let mut j = i;
                while j < lines.len() {
                    body.push_str(lines[j]);
                    body.push('\n');
                    if j > i && lines[j] == close {
                        break;
                    }
                    j += 1;
                }
                bodies.push(body);
            }
            i += 1;
        }
    }
    bodies
}

/// Condition 5's own rendering-proof rule, applied to one already-found function body:
/// literally naming `TestBackend`, or calling one of this codebase's established
/// TestBackend-driving helpers (`render_at(`, `run_wired(`, `run_wired_at(`,
/// `run_wired_staged(`, `run_wired_probed(`) — every one of which constructs a
/// `ratatui::backend::TestBackend` in ITS OWN body (`src/lib.rs`'s `render_at`;
/// `src/ui/mod.rs`'s `run_wired_at`/`run_wired_staged`/`run_wired_probed`, all three
/// wrapping the real `run_wired`). A file-level or helper-name-only check would prove
/// nothing new; requiring the literal string in every calling test's own body would fail
/// nearly every view test in this crate, which reaches a `TestBackend` through exactly
/// these helpers rather than constructing one inline — inlining is the exception
/// (`resizing_the_backend_changes_the_next_frame`, `resizing_changes_the_slice_on_the_next_frame`,
/// and the two watcher sub-cases in `a_watch_failure_keeps_the_loop_drawing`), not the rule.
fn renders_through_a_backend(body: &str) -> bool {
    const MARKERS: &[&str] = &[
        "TestBackend",
        "render_at(",
        "run_wired(",
        "run_wired_at(",
        "run_wired_staged(",
        "run_wired_probed(",
    ];
    MARKERS.iter().any(|m| body.contains(m))
}

const LEGAL_VERDICTS: &[&str] = &[
    "confirmed",
    "unproven",
    "spec-corrected",
    "repaired",
    "implemented",
];
const LEGAL_TIERS: &[&str] = &["view", "outer", "unit", "integration"];

/// The whole check, task 11.2's GREEN: every condition 1-6 `specs/degraded-coverage/spec.md`
/// names. Returns the number of table rows matched on success.
fn check_coverage(spec_md: &str, toml_text: &str, files: &[PathBuf]) -> Result<usize, String> {
    let conditions = parse_spec_conditions(spec_md)?;
    if conditions.len() < MIN_ROWS {
        return Err(format!(
            "SPEC.md's degraded-states table holds {} rows, expected at least {MIN_ROWS}",
            conditions.len()
        ));
    }

    let rows = parse_coverage_toml(toml_text)?;
    if rows.len() < MIN_ROWS {
        return Err(format!(
            "tests/degraded-coverage.toml holds {} [[row]] entries, expected at least {MIN_ROWS}",
            rows.len()
        ));
    }

    // Condition 3: no two entries share a condition.
    let mut seen_conditions: BTreeSet<&str> = BTreeSet::new();
    for row in &rows {
        if !seen_conditions.insert(row.condition.as_str()) {
            return Err(format!(
                "tests/degraded-coverage.toml has two [[row]] entries with the same condition: {:?}",
                row.condition
            ));
        }
    }

    // Condition 1: every table row has a matching entry.
    let toml_conditions: BTreeSet<&str> = rows.iter().map(|r| r.condition.as_str()).collect();
    for condition in &conditions {
        if !toml_conditions.contains(condition.as_str()) {
            return Err(format!(
                "SPEC.md row {condition:?} has no tests/degraded-coverage.toml [[row]] entry"
            ));
        }
    }

    // Condition 2: no orphan entry (a condition matching no table row).
    let spec_conditions: BTreeSet<&str> = conditions.iter().map(String::as_str).collect();
    for row in &rows {
        if !spec_conditions.contains(row.condition.as_str()) {
            return Err(format!(
                "tests/degraded-coverage.toml's [[row]] {:?} matches no SPEC.md row (an orphan \
                 left by a reworded or removed row)",
                row.condition
            ));
        }
    }

    for row in &rows {
        // Verdict check (both the general legality and the `repaired`-names-a-change rule).
        if !LEGAL_VERDICTS.contains(&row.verdict.as_str()) {
            return Err(format!(
                "row {:?}: verdict {:?} is not one of {LEGAL_VERDICTS:?}",
                row.condition, row.verdict
            ));
        }
        if row.verdict == "repaired" {
            let archive_dir = manifest_dir().join("openspec/changes/archive");
            let names_a_change = fs::read_dir(&archive_dir)
                .map(|entries| {
                    entries
                        .flatten()
                        .filter_map(|e| e.file_name().into_string().ok())
                        .any(|name| row.why.contains(&name))
                })
                .unwrap_or(false);
            if !names_a_change {
                return Err(format!(
                    "row {:?}: verdict is \"repaired\" but \"why\" ({:?}) names no directory \
                     under openspec/changes/archive/",
                    row.condition, row.why
                ));
            }
        }

        if !LEGAL_TIERS.contains(&row.tier.as_str()) {
            return Err(format!(
                "row {:?}: tier {:?} is not one of {LEGAL_TIERS:?}",
                row.condition, row.tier
            ));
        }

        // Condition 4 and 5.
        for name in &row.proof {
            let bodies = function_bodies(files, name);
            if bodies.is_empty() {
                return Err(format!(
                    "row {:?}: proof {name:?} is not defined as \"fn {name}(\" anywhere under \
                     src/ or tests/",
                    row.condition
                ));
            }
            if (row.tier == "view" || row.tier == "outer")
                && !bodies.iter().any(|b| renders_through_a_backend(b))
            {
                return Err(format!(
                    "row {:?}: tier {:?} names {name:?}, whose body renders nothing (no \
                     TestBackend, directly or through render_at/run_wired)",
                    row.condition, row.tier
                ));
            }
        }
    }

    Ok(conditions.len())
}

fn spec_md() -> String {
    fs::read_to_string(manifest_dir().join("SPEC.md")).expect("read SPEC.md")
}

fn coverage_toml() -> String {
    fs::read_to_string(manifest_dir().join("tests/degraded-coverage.toml"))
        .expect("read tests/degraded-coverage.toml")
}

#[test]
fn every_table_row_has_a_proof() {
    let files = searchable_files();
    match check_coverage(&spec_md(), &coverage_toml(), &files) {
        Ok(n) => assert!(n >= MIN_ROWS, "expected at least {MIN_ROWS} rows, got {n}"),
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn every_row_carries_a_verdict() {
    let rows = parse_coverage_toml(&coverage_toml()).expect("parse the coverage map");
    assert!(rows.len() >= MIN_ROWS);
    for row in &rows {
        assert!(
            LEGAL_VERDICTS.contains(&row.verdict.as_str()),
            "{:?}: {:?}",
            row.condition,
            row.verdict
        );
    }

    // A `verdict` of `probably-fine` must fail — the same rule `check_coverage` runs,
    // proved directly against the parser rather than by editing the checked-in file.
    let mut mutated = rows;
    mutated[0].verdict = "probably-fine".to_string();
    let mutated_toml = render_rows_as_toml(&mutated);
    let files = searchable_files();
    let result = check_coverage(&spec_md(), &mutated_toml, &files);
    assert!(
        result.is_err(),
        "a \"probably-fine\" verdict must fail the check"
    );
}

/// Render `rows` back to `[[row]]` TOML text, for `every_row_carries_a_verdict`'s
/// self-contained mutation proof — never written back to the checked-in file.
fn render_rows_as_toml(rows: &[Row]) -> String {
    let mut out = String::new();
    for row in rows {
        out.push_str("[[row]]\n");
        out.push_str(&format!("condition = {:?}\n", row.condition));
        out.push_str(&format!("tier = {:?}\n", row.tier));
        out.push_str("proof = [");
        out.push_str(
            &row.proof
                .iter()
                .map(|p| format!("{p:?}"))
                .collect::<Vec<_>>()
                .join(", "),
        );
        out.push_str("]\n");
        out.push_str(&format!("verdict = {:?}\n", row.verdict));
        out.push_str(&format!("why = {:?}\n\n", row.why));
    }
    out
}

#[test]
fn an_empty_table_fails_the_floor() {
    let empty_section = "## Degraded states\n\n| Condition | Behaviour |\n|---|---|\n\n### No terminal is not a degraded state\n";
    let err = parse_spec_conditions(empty_section)
        .map(|conditions| {
            if conditions.len() < MIN_ROWS {
                Err(format!(
                    "SPEC.md's degraded-states table holds {} rows, expected at least {MIN_ROWS}",
                    conditions.len()
                ))
            } else {
                Ok(())
            }
        })
        .and_then(|r| r);
    assert!(
        err.is_err(),
        "an empty table (header and separator only) must fail the floor, not report full \
         coverage of zero rows"
    );

    // The same holds when the heading is absent entirely.
    let no_heading = "Nothing here names a degraded state.\n";
    assert!(parse_spec_conditions(no_heading).is_err());
}
