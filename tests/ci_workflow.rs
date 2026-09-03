//! Parity guard between `.github/workflows/ci.yml` and the `Makefile`.
//!
//! Std-only: the crate may take no dependency, so this reads the workflow and the
//! `Makefile` as plain text and matches lines and substrings rather than parsing YAML.
//! See `openspec/changes/ci-pipeline/design.md` for the scope and the reasoning behind
//! each assertion.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workflow_path() -> PathBuf {
    manifest_dir().join(".github/workflows/ci.yml")
}

fn workflows_dir_path() -> PathBuf {
    manifest_dir().join(".github/workflows")
}

fn makefile_path() -> PathBuf {
    manifest_dir().join("Makefile")
}

fn read_workflow() -> String {
    let path = workflow_path();
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn read_makefile() -> String {
    let path = makefile_path();
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// A line with no leading whitespace and not a comment — a candidate top-level YAML key.
fn is_column0_line(line: &str) -> bool {
    !line.is_empty() && !line.starts_with(' ') && !line.starts_with('\t') && !line.starts_with('#')
}

/// Whether `line` is a job-key line under `jobs:` — exactly two leading spaces, then a
/// GitHub-legal job id, then `:` and nothing else. The character class must accept `_`
/// and uppercase, because GitHub job ids allow both and a narrower pattern would make
/// the aggregate-check guard green for the case it exists to catch.
fn job_key_name(line: &str) -> Option<String> {
    if line.len() < 2 || &line[0..2] != "  " {
        return None;
    }
    if line.as_bytes().get(2) == Some(&b' ') {
        return None;
    }
    let rest = line[2..].trim_end();
    let key = rest.strip_suffix(':')?;
    if key.is_empty() {
        return None;
    }
    let mut chars = key.chars();
    let first = chars.next()?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return None;
    }
    Some(key.to_string())
}

struct JobSection {
    name: String,
    start: usize,
    end: usize,
    text: String,
}

/// Splits the workflow into job sections. A section runs from its job-key line to the
/// next job-key line, the next column-0 line, or EOF.
fn job_sections(content: &str) -> Vec<JobSection> {
    let lines: Vec<&str> = content.lines().collect();
    let jobs_idx = lines
        .iter()
        .position(|l| *l == "jobs:")
        .expect("workflow must contain a column-0 `jobs:` line");

    let mut starts: Vec<(String, usize)> = Vec::new();
    let mut i = jobs_idx + 1;
    while i < lines.len() {
        if let Some(name) = job_key_name(lines[i]) {
            starts.push((name, i));
        } else if is_column0_line(lines[i]) {
            break;
        }
        i += 1;
    }

    let mut sections = Vec::new();
    for (idx, (name, start)) in starts.iter().enumerate() {
        let end = if idx + 1 < starts.len() {
            starts[idx + 1].1
        } else {
            let mut e = lines.len();
            for (j, line) in lines.iter().enumerate().skip(start + 1) {
                if is_column0_line(line) {
                    e = j;
                    break;
                }
            }
            e
        };
        sections.push(JobSection {
            name: name.clone(),
            start: *start,
            end,
            text: lines[*start..end].join("\n"),
        });
    }
    sections
}

fn find_job<'a>(sections: &'a [JobSection], name: &str) -> &'a JobSection {
    sections
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("workflow must declare a `{name}` job"))
}

fn contains_trimmed_key(content: &str, key: &str) -> bool {
    let target = format!("{key}:");
    content.lines().any(|l| l.trim() == target)
}

fn phony_targets(makefile: &str) -> Vec<String> {
    makefile
        .lines()
        .find(|l| l.trim_start().starts_with(".PHONY"))
        .expect(".PHONY line not found in Makefile")
        .split_whitespace()
        .skip(1)
        .map(|s| s.trim_end_matches(':').to_string())
        .collect()
}

fn parse_needs_list(line: &str) -> BTreeSet<String> {
    let start = line
        .find('[')
        .expect("`needs:` must use inline list syntax `[a, b]`");
    let end = line.find(']').expect("`needs:` must be closed with `]`");
    line[start + 1..end]
        .split(',')
        .map(|s| s.trim().to_string())
        .collect()
}

#[test]
fn ci_yml_is_the_only_workflow_file() {
    let dir = workflows_dir_path();
    let mut entries: Vec<String> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", dir.display()))
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    assert_eq!(
        entries,
        vec!["ci.yml".to_string()],
        "expected exactly one workflow file, ci.yml"
    );
}

#[test]
fn pull_requests_and_main_pushes_both_trigger_the_workflow() {
    let content = read_workflow();
    assert!(
        contains_trimmed_key(&content, "push"),
        "missing `push:` trigger"
    );
    assert!(
        content.contains("branches: [main]"),
        "push trigger must restrict to branches: [main]"
    );
    assert!(
        contains_trimmed_key(&content, "pull_request"),
        "missing `pull_request:` trigger"
    );
    assert!(
        contains_trimmed_key(&content, "workflow_dispatch"),
        "missing `workflow_dispatch:` trigger"
    );
    for absent in [
        "schedule:",
        "release:",
        "workflow_call:",
        "paths:",
        "paths-ignore:",
    ] {
        assert!(
            !content.contains(absent),
            "workflow must not declare `{absent}`"
        );
    }
}

#[test]
fn both_declared_platforms_run_the_gates() {
    let content = read_workflow();
    let sections = job_sections(&content);
    let check = find_job(&sections, "check");
    assert!(
        check.text.contains("os: [ubuntu-latest, macos-latest]"),
        "matrix must name exactly ubuntu-latest and macos-latest"
    );
    assert!(
        check.text.contains("fail-fast: false"),
        "check job must set fail-fast: false"
    );
    assert!(
        check.text.contains("timeout-minutes"),
        "check job must declare timeout-minutes"
    );
    assert!(
        !content.contains("windows"),
        "workflow must not reference windows"
    );
}

#[test]
fn every_gate_the_makefile_composes_runs_in_ci() {
    let content = read_workflow();
    let sections = job_sections(&content);
    let check = find_job(&sections, "check");
    let fmt_pos = check
        .text
        .find("make fmt-check")
        .expect("check job must run make fmt-check");
    let lint_pos = check
        .text
        .find("make lint")
        .expect("check job must run make lint");
    let test_pos = check
        .text
        .find("make test")
        .expect("check job must run make test");
    assert!(
        fmt_pos < lint_pos && lint_pos < test_pos,
        "gates must run in order fmt-check, lint, test"
    );

    assert!(
        !content.contains("make check"),
        "workflow must not invoke the composite make check target"
    );
    for target in ["make fmt-check", "make lint", "make test", "make coverage"] {
        assert!(
            content.contains(target),
            "workflow must invoke `{target}` at least once"
        );
    }
}

#[test]
fn no_gate_step_can_be_skipped_or_ignored() {
    let content = read_workflow();
    assert!(
        !content.contains("continue-on-error"),
        "no step may set continue-on-error"
    );

    let sections = job_sections(&content);
    let ci = find_job(&sections, "ci");
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("if:") || trimmed.starts_with("- if:") {
            assert!(
                i >= ci.start && i < ci.end,
                "line {i} sets `if:` outside the `ci` job section: `{line}`"
            );
        }
    }
}

#[test]
fn gate_commands_are_defined_only_in_the_makefile() {
    let content = read_workflow();
    let makefile = read_makefile();
    let phony = phony_targets(&makefile);

    for line in content.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("run: ") {
            let rest = rest.trim();
            assert!(
                !rest.contains("cargo"),
                "run: step must not contain `cargo`: `{trimmed}`"
            );
            if rest == "exit 1" {
                continue;
            }
            let target = rest.strip_prefix("make ").unwrap_or_else(|| {
                panic!("run: step must be `make <target>` or `exit 1`, found: `{trimmed}`")
            });
            assert!(
                phony.iter().any(|t| t == target),
                "run: step invokes undeclared Makefile target `{target}`"
            );
        }
        assert_ne!(
            trimmed, "env:",
            "workflow must declare no `env:` mapping: line `{line}`"
        );
    }
}

#[test]
fn coverage_threshold_is_not_restated_in_ci() {
    let content = read_workflow();
    assert!(
        !content.contains("--fail-under-lines"),
        "workflow must not restate the coverage threshold"
    );
    let makefile = read_makefile();
    assert!(
        makefile.contains("--fail-under-lines 80"),
        "Makefile must still declare the coverage floor"
    );
}

#[test]
fn parser_preconditions_hold() {
    let makefile = read_makefile();
    assert!(
        makefile
            .lines()
            .any(|l| l.trim_start().starts_with(".PHONY")),
        ".PHONY line missing from Makefile"
    );

    let workflow = read_workflow();
    let lines: Vec<&str> = workflow.lines().collect();
    let jobs_idx = lines
        .iter()
        .position(|l| *l == "jobs:")
        .expect("column-0 `jobs:` line not found in workflow");

    let last_column0_idx = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| is_column0_line(l))
        .map(|(i, _)| i)
        .next_back()
        .expect("no column-0 line found");
    assert_eq!(
        jobs_idx, last_column0_idx,
        "`jobs:` must be the last column-0 key in the file"
    );

    let sections = job_sections(&workflow);
    assert_eq!(
        sections.len(),
        3,
        "expected exactly three job sections, found {}",
        sections.len()
    );

    for line in &lines {
        let trimmed = line.trim_start();
        assert!(
            !trimmed.starts_with("run: |") && !trimmed.starts_with("run: >"),
            "block-scalar `run:` found: {line}"
        );
    }
}

#[test]
fn coverage_gate_runs_exactly_once_and_only_on_linux() {
    let content = read_workflow();
    let occurrences = content.matches("make coverage").count();
    assert_eq!(
        occurrences, 1,
        "`make coverage` must occur exactly once in the workflow"
    );

    let sections = job_sections(&content);
    let coverage = find_job(&sections, "coverage");
    assert!(
        coverage.text.contains("make coverage"),
        "`coverage` job must run `make coverage`"
    );
    assert!(
        coverage.text.contains("runs-on: ubuntu-latest"),
        "`coverage` job must run on ubuntu-latest"
    );
    assert!(
        !coverage.text.contains("strategy:"),
        "`coverage` job must declare no strategy/matrix"
    );

    let check = find_job(&sections, "check");
    assert!(
        !check.text.contains("make coverage"),
        "`check` job must not run `make coverage`"
    );
}

#[test]
fn every_job_is_behind_the_aggregate_status_check() {
    let content = read_workflow();
    let sections = job_sections(&content);
    let ci = find_job(&sections, "ci");

    assert!(
        ci.text.contains("if: always()"),
        "`ci` job must declare `if: always()`"
    );

    let has_failing_step = content.lines().any(|l| {
        let t = l.trim();
        t.starts_with("if:")
            && t.contains("needs.*.result")
            && t.contains("failure")
            && t.contains("cancelled")
            && t.contains("skipped")
    });
    assert!(
        has_failing_step,
        "`ci` job must have a step whose `if:` checks failure, cancelled, and skipped across needs.*.result"
    );
    assert!(
        ci.text.contains("exit 1"),
        "`ci` job's failing step must run `exit 1`"
    );

    let job_names: BTreeSet<String> = sections
        .iter()
        .map(|s| s.name.clone())
        .filter(|n| n != "ci")
        .collect();
    let needs_line = content
        .lines()
        .find(|l| l.trim_start().starts_with("needs:"))
        .expect("`ci` job must declare `needs:`");
    let needs_set = parse_needs_list(needs_line);
    assert_eq!(
        job_names, needs_set,
        "every job other than `ci` must appear in `ci`'s `needs`, and vice versa"
    );
}

#[test]
fn workflow_grants_no_write_and_reads_no_secret() {
    let content = read_workflow();
    let lines: Vec<&str> = content.lines().collect();
    let perm_idx = lines
        .iter()
        .position(|l| *l == "permissions:")
        .expect("workflow must declare a column-0 `permissions:` block");
    let mut end = lines.len();
    for (j, line) in lines.iter().enumerate().skip(perm_idx + 1) {
        if is_column0_line(line) {
            end = j;
            break;
        }
    }
    let perm_block = lines[perm_idx..end].join("\n");
    assert!(
        perm_block.contains("contents: read"),
        "permissions must grant contents: read"
    );
    let entries: Vec<&&str> = lines[perm_idx + 1..end]
        .iter()
        .filter(|l| !l.trim().is_empty())
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "top-level `permissions:` must grant nothing beyond `contents: read`"
    );

    assert!(
        !content.contains("secrets."),
        "workflow must reference no secret"
    );
    assert!(
        !content.contains(": write"),
        "workflow must grant no write permission"
    );
}
