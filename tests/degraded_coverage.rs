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
const MIN_ROWS: usize = 46;

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
    /// `path:first-last` line ranges naming the production code the row's `proof`
    /// exercises — the coverage spec's sixth key. Parsed leniently here (an absent
    /// `covers` key parses as an empty `Vec`) so `parse_coverage_toml` keeps reading
    /// every row of the checked-in map before task 6.3's backfill lands; the "must be
    /// non-empty and every entry must resolve" rule itself lives in
    /// [`validate_covers`], called from `check_coverage`.
    covers: Vec<(String, usize, usize)>,
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
        let mut covers = Vec::new();
        if let Some(covers_value) = table.get("covers") {
            let covers_array = covers_value.as_array().ok_or_else(|| {
                format!("row {index} ({condition:?}): \"covers\" is not an array")
            })?;
            for c in covers_array {
                let raw = c.as_str().ok_or_else(|| {
                    format!("row {index} ({condition:?}): a \"covers\" entry is not a string")
                })?;
                let (path, range) = raw.rsplit_once(':').ok_or_else(|| {
                    format!(
                        "row {index} ({condition:?}): covers entry {raw:?} is not \
                         \"path:first-last\""
                    )
                })?;
                let (first_s, last_s) = range.split_once('-').ok_or_else(|| {
                    format!(
                        "row {index} ({condition:?}): covers entry {raw:?} has no \
                         \"first-last\" range"
                    )
                })?;
                let first: usize = first_s.parse().map_err(|_| {
                    format!(
                        "row {index} ({condition:?}): covers entry {raw:?} has a \
                         non-numeric first line"
                    )
                })?;
                let last: usize = last_s.parse().map_err(|_| {
                    format!(
                        "row {index} ({condition:?}): covers entry {raw:?} has a \
                         non-numeric last line"
                    )
                })?;
                covers.push((path.to_string(), first, last));
            }
        }
        rows.push(Row {
            condition,
            tier,
            proof,
            verdict,
            why,
            covers,
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

/// Condition 4b: is the occurrence of a bare `fn <name>(` at line index `fn_line` of
/// `lines` directly marked `#[test]` and not `#[ignore]`d? Walks upward through the
/// contiguous block of attribute lines immediately above the `fn` line — an ordinary
/// Rust test's `#[test]` may sit above or below a sibling attribute such as
/// `#[should_panic(...)]` (this crate writes it `#[test]` first, `src/changes.rs`'s
/// `an_empty_name_panics` among others), so this looks for `#[test]` anywhere in that
/// block rather than requiring it to be the single line immediately above — the spec's
/// own "`#[test]` attribute on the line above" phrasing covers the ordinary one-attribute
/// case; a stacked `#[should_panic]` is the one shape here that needs the wider walk.
/// `#[ignore` anywhere in the same block always wins: a test nothing runs proves nothing,
/// on exactly the same terms whether it once was `#[test]` alone or `#[test]` stacked with
/// another attribute.
fn is_directly_marked_test(lines: &[&str], fn_line: usize) -> bool {
    let mut has_test = false;
    let mut has_ignore = false;
    let mut i = fn_line;
    while i > 0 {
        let prev = lines[i - 1].trim();
        if !prev.starts_with("#[") {
            break;
        }
        if prev.starts_with("#[test]") {
            has_test = true;
        }
        if prev.starts_with("#[ignore") {
            has_ignore = true;
        }
        i -= 1;
    }
    has_test && !has_ignore
}

/// Every directly-`#[test]`-marked function's own body, across `files` — computed once per
/// `check_coverage` call, since condition 4b's "or... be named in the body of at least one
/// function that does [carry `#[test]`]" clause (for a shared helper several tests call) is
/// independent of any one `proof` name. Matches only a bare `fn name(` — never a `pub fn` —
/// on `function_bodies`' own established terms: no `#[test]` function in this crate is
/// declared `pub`.
fn all_directly_tested_bodies(files: &[PathBuf]) -> Vec<String> {
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
            if trimmed_start.starts_with("fn ") && is_directly_marked_test(&lines, i) {
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

/// Condition 4b, the whole rule: `name` is a proof precisely when at least one of its own
/// `fn <name>(` occurrences is directly `#[test]`-marked (and not `#[ignore]`d), or `name`
/// is called (`name(`) in the body of at least one function that is. `all_tested_bodies` is
/// `all_directly_tested_bodies`'s output, built once per `check_coverage` call and shared
/// across every `proof` name it checks.
fn is_proof_a_test(files: &[PathBuf], name: &str, all_tested_bodies: &[String]) -> bool {
    let needle = format!("fn {name}(");
    for path in files {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.trim_start().starts_with(&needle) && is_directly_marked_test(&lines, i) {
                return true;
            }
        }
    }
    let call_needle = format!("{name}(");
    all_tested_bodies
        .iter()
        .any(|body| body.contains(&call_needle))
}

/// Condition 4c and the six-key rule's `covers` half: `covers` SHALL be non-empty, and
/// every `path:first-last` entry SHALL resolve — `path` a file under `src/`, `first <=
/// last`, both within the file's own line count, and the range holding at least one line
/// of code (a line, once trimmed, that is non-empty and does not open with `//`). This is
/// a stated limit on the same terms `coverage-prod.py`'s own comment/string masking is:
/// it does not parse Rust, so a block comment (`/* ... */`) spanning into the range from
/// outside it is not detected — recorded here rather than silently assumed complete.
fn validate_covers(condition: &str, covers: &[(String, usize, usize)]) -> Result<(), String> {
    if covers.is_empty() {
        return Err(format!(
            "row {condition:?}: missing or empty \"covers\" — every row must name the \
             production code its proof covers"
        ));
    }
    for (path, first, last) in covers {
        if !path.starts_with("src/") {
            return Err(format!(
                "row {condition:?}: covers entry {path:?} does not name a path under src/"
            ));
        }
        let full = manifest_dir().join(path);
        let Ok(text) = fs::read_to_string(&full) else {
            return Err(format!(
                "row {condition:?}: covers entry {path}:{first}-{last} names a path that \
                 does not exist under src/"
            ));
        };
        if first > last {
            return Err(format!(
                "row {condition:?}: covers entry {path}:{first}-{last} is a reversed range \
                 (first > last)"
            ));
        }
        let lines: Vec<&str> = text.lines().collect();
        if *first == 0 || *last > lines.len() {
            return Err(format!(
                "row {condition:?}: covers entry {path}:{first}-{last} is out of range \
                 ({path} has {} lines)",
                lines.len()
            ));
        }
        let holds_code = lines[(first - 1)..*last].iter().any(|line| {
            let t = line.trim();
            !t.is_empty() && !t.starts_with("//")
        });
        if !holds_code {
            return Err(format!(
                "row {condition:?}: covers entry {path}:{first}-{last} holds no line of \
                 code (blank or comment-only)"
            ));
        }
    }
    Ok(())
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
/// (`the_mode_follows_the_current_frame_not_the_startup_size`, `resizing_changes_the_slice_on_the_next_frame`,
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

    // Built once, shared across every row's condition 4b check below.
    let all_tested_bodies = all_directly_tested_bodies(files);

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

        // Condition 4, 4b, and 5.
        for name in &row.proof {
            let bodies = function_bodies(files, name);
            if bodies.is_empty() {
                return Err(format!(
                    "row {:?}: proof {name:?} is not defined as \"fn {name}(\" anywhere under \
                     src/ or tests/",
                    row.condition
                ));
            }
            if !is_proof_a_test(files, name, &all_tested_bodies) {
                return Err(format!(
                    "row {:?}: proof {name:?} is not a #[test] (and is not named in the \
                     body of one that is) — a test nothing runs proves nothing",
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

        // Condition 4c and the six-key rule's `covers` half.
        validate_covers(&row.condition, &row.covers)?;
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
        out.push_str(&format!("why = {:?}\n", row.why));
        // An empty `covers` renders no key at all, rather than `covers = []` — the two
        // read identically to `parse_coverage_toml` (both leave `Row::covers` empty), and
        // omitting the key is what lets a test exercise the "no `covers` key at all"
        // failure case directly, rather than only the "present but empty" one.
        if !row.covers.is_empty() {
            let entries: Vec<String> = row
                .covers
                .iter()
                .map(|(path, first, last)| format!("{path}:{first}-{last}"))
                .collect();
            out.push_str(&format!(
                "covers = [{}]\n",
                entries
                    .iter()
                    .map(|e| format!("{e:?}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        out.push('\n');
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

// --- gate-integrity, group 6: conditions 4b and 4c ---------------------------------------

/// This file's own RED/GREEN fixture for `a_proof_naming_an_ignored_test_fails` below: a
/// real test, resolvable by condition 4's "fn <name>(" search, `#[ignore]`d so nothing ever
/// runs it. Its own `#[ignore]` is not a defect to fix — it exists so a `proof` entry can be
/// repointed at it.
///
/// gate-integrity Change Review (task 9.3), CRITICAL 4: this attribute SHALL stay on ONE
/// line. `is_directly_marked_test`'s upward walk stops at the first line above `fn` that
/// does not start with `#[`; a line-wrapped `#[ignore = "..."]` puts a bare string-
/// continuation line directly above `fn`, which breaks the walk before it ever reaches
/// `#[test]` — `has_test` comes back `false` for the wrong reason, and `has_ignore` (and the
/// `&& !has_ignore` it feeds) never gets exercised at all. Measured: deleting `has_ignore`
/// and `&& !has_ignore` entirely left this fixture's own proving test green.
#[test]
#[ignore = "intentionally ignored: gate-integrity's own fixture for the coverage binding's \"a proof naming an #[ignore]d test fails\" case (task 6.1) - not a test anyone should un-ignore"]
fn ignored_fixture_for_coverage_binding_tests() {}

/// The index of the first `unit`-tier row — used by the two tests immediately below so
/// condition 5's own "a view/outer proof must render" rule (which also names the offending
/// proof in its own failure message) cannot mask condition 4b's, on a tier where condition
/// 5 does not even apply.
fn first_unit_row_index(rows: &[Row]) -> usize {
    rows.iter()
        .position(|r| r.tier == "unit")
        .expect("the coverage map names at least one unit-tier row")
}

/// Task 6.1 RED / 6.2 GREEN, condition 4b: a `proof` naming a real, non-test production
/// function must fail as "not a test" — not as "not defined" (condition 4 already resolves
/// it) and not silently pass. `fn start(` does not work as the plant: every `start` in the
/// crate is `pub fn start(` (`src/watch.rs:260`, `src/agents.rs:464`, `src/refresh.rs:78`,
/// `src/launch.rs:379`), and condition 4 matches only a bare, non-`pub` `fn <name>(`, so that
/// plant fails as "not defined" — passing for the wrong reason. `is_usable_binary` in
/// `src/resolve.rs` is the plant `specs/degraded-coverage/spec.md`'s own scenario names:
/// declared `fn` without `pub`, resolved by condition 4, carrying no `#[test]` anywhere it is
/// defined, and never named in the body of a test either.
#[test]
fn a_proof_naming_a_non_test_function_fails() {
    let mut mutated = parse_coverage_toml(&coverage_toml()).expect("parse the coverage map");
    let idx = first_unit_row_index(&mutated);
    mutated[idx].proof = vec!["is_usable_binary".to_string()];
    let mutated_toml = render_rows_as_toml(&mutated);
    let files = searchable_files();
    let err = check_coverage(&spec_md(), &mutated_toml, &files)
        .expect_err("a proof naming a real, non-test function must fail");
    assert!(
        err.contains("is_usable_binary"),
        "the error must name the offending proof: {err:?}"
    );
    assert!(
        !err.contains("renders nothing"),
        "this must fail on the \"not a test\" rule, not condition 5's rendering rule: {err:?}"
    );
}

/// The same rule catches a real test whose `#[test]` attribute was replaced with
/// `#[ignore]` — a test nothing runs proves nothing.
///
/// gate-integrity Change Review (task 9.3), CRITICAL 4: asserts the DISTINGUISHING failure
/// message ("is not a #[test]"), not merely that the proof name appears somewhere in the
/// error — condition 4's own "not defined" message also names the proof, and (before the
/// fixture's `#[ignore]` was put on one line, see the fixture's own comment) this test
/// could not tell the two apart: it passed even with `has_ignore` deleted from
/// `is_directly_marked_test` entirely, because `has_test` alone already came back `false`.
#[test]
fn a_proof_naming_an_ignored_test_fails() {
    let mut mutated = parse_coverage_toml(&coverage_toml()).expect("parse the coverage map");
    let idx = first_unit_row_index(&mutated);
    mutated[idx].proof = vec!["ignored_fixture_for_coverage_binding_tests".to_string()];
    let mutated_toml = render_rows_as_toml(&mutated);
    let files = searchable_files();
    let err = check_coverage(&spec_md(), &mutated_toml, &files)
        .expect_err("a proof naming an #[ignore]d test must fail");
    assert!(
        err.contains("ignored_fixture_for_coverage_binding_tests"),
        "{err:?}"
    );
    assert!(
        err.contains("is not a #[test]"),
        "must fail on the \"not a #[test]\" rule (condition 4b), naming the #[ignore]d \
         test as not directly marked, not on condition 4's \"not defined\" rule: {err:?}"
    );
    assert!(
        !err.contains("is not defined as"),
        "the function IS defined - failing as \"not defined\" would mean condition 4's own \
         check masked condition 4b's, which is not the rule this test exercises: {err:?}"
    );
}

/// Task 6.1 RED / 6.2 GREEN, condition 4c: a `covers` range past the end of its file fails.
#[test]
fn a_covers_range_past_the_end_of_the_file_fails() {
    let mut mutated = parse_coverage_toml(&coverage_toml()).expect("parse the coverage map");
    mutated[0].covers = vec![("src/watch.rs".to_string(), 9000, 9001)];
    let mutated_toml = render_rows_as_toml(&mutated);
    let files = searchable_files();
    let err = check_coverage(&spec_md(), &mutated_toml, &files)
        .expect_err("a covers range past the end of the file must fail");
    assert!(err.contains("src/watch.rs"), "{err:?}");
}

/// Condition 4c: a reversed (`last` < `first`) `covers` range fails.
#[test]
fn a_reversed_covers_range_fails() {
    let mut mutated = parse_coverage_toml(&coverage_toml()).expect("parse the coverage map");
    mutated[0].covers = vec![("src/watch.rs".to_string(), 280, 270)];
    let mutated_toml = render_rows_as_toml(&mutated);
    let files = searchable_files();
    let err = check_coverage(&spec_md(), &mutated_toml, &files)
        .expect_err("a reversed covers range must fail");
    assert!(err.contains("src/watch.rs"), "{err:?}");
}

/// Condition 4c: a `covers` entry naming a path that does not exist under `src/` fails.
#[test]
fn a_covers_range_naming_a_missing_path_fails() {
    let mut mutated = parse_coverage_toml(&coverage_toml()).expect("parse the coverage map");
    mutated[0].covers = vec![("src/does-not-exist.rs".to_string(), 1, 2)];
    let mutated_toml = render_rows_as_toml(&mutated);
    let files = searchable_files();
    let err = check_coverage(&spec_md(), &mutated_toml, &files)
        .expect_err("a covers entry naming a missing path must fail");
    assert!(err.contains("src/does-not-exist.rs"), "{err:?}");
}

/// Condition 4c: a `covers` range holding only the module doc comment — no code at all —
/// fails. `src/watch.rs:1-9` is entirely `//!` lines at HEAD.
#[test]
fn a_comment_only_covers_range_fails() {
    let mut mutated = parse_coverage_toml(&coverage_toml()).expect("parse the coverage map");
    mutated[0].covers = vec![("src/watch.rs".to_string(), 1, 9)];
    let mutated_toml = render_rows_as_toml(&mutated);
    let files = searchable_files();
    let err = check_coverage(&spec_md(), &mutated_toml, &files)
        .expect_err("a comment-only covers range must fail");
    assert!(err.contains("src/watch.rs"), "{err:?}");
}

/// The six-key rule: an entry with no `covers` key at all fails, so the twenty `unproven`
/// rows cannot keep their verdict without saying where the behaviour lives.
#[test]
fn a_row_with_no_covers_entries_fails_the_six_key_rule() {
    let mut mutated = parse_coverage_toml(&coverage_toml()).expect("parse the coverage map");
    mutated[0].covers = Vec::new();
    let mutated_toml = render_rows_as_toml(&mutated);
    let files = searchable_files();
    let err = check_coverage(&spec_md(), &mutated_toml, &files)
        .expect_err("a row with no covers key at all must fail the six-key rule");
    assert!(err.contains(&mutated[0].condition), "{err:?}");
}
