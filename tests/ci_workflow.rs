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

/// `AGENTS.md` is the real file; `CLAUDE.md` is a symlink to it (verified on disk, not
/// assumed) — reading this path is reading the same bytes either name would resolve to.
fn agents_md_path() -> PathBuf {
    manifest_dir().join("AGENTS.md")
}

fn spec_md_path() -> PathBuf {
    manifest_dir().join("SPEC.md")
}

fn read_workflow() -> String {
    let path = workflow_path();
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn read_makefile() -> String {
    let path = makefile_path();
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn read_agents_md() -> String {
    let path = agents_md_path();
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn read_spec_md() -> String {
    let path = spec_md_path();
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// The body of a Markdown heading line — every line strictly between a line that equals
/// `heading` (after trimming trailing whitespace) and the next heading line of any level,
/// or EOF. Shared by the `AGENTS.md` and `SPEC.md` document tests below: both read a
/// scoped section rather than the whole file, so a coincidental substring match elsewhere
/// in the document cannot pass either test vacuously.
fn markdown_section(markdown: &str, heading: &str) -> String {
    let mut body = String::new();
    let mut in_section = false;
    for line in markdown.lines() {
        let trimmed = line.trim_end();
        if trimmed.starts_with('#') {
            in_section = trimmed == heading;
            continue;
        }
        if in_section {
            body.push_str(line);
            body.push('\n');
        }
    }
    body
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

/// Parses `check:`'s own prerequisite list out of the `Makefile` — the targets after the
/// colon on the line `check: <prereq> <prereq> ...` — in declaration order. This is what
/// makes `every_gate_the_makefile_composes_runs_in_ci` force a real guarantee rather than
/// compare against a hardcoded literal: a fifth gate joining `check` with no matching CI
/// step is caught because the parser sees it, not because someone remembered to update a
/// list here too.
fn parse_check_prereqs(makefile: &str) -> Vec<String> {
    let line = makefile
        .lines()
        .find(|l| l.trim_start().starts_with("check:") || l.trim_start().starts_with("check :"))
        .expect("no `check:` rule found in Makefile");
    let rest = line.split_once(':').expect("`check:` line has no colon").1;
    rest.split_whitespace().map(|s| s.to_string()).collect()
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

/// `check:`'s own prerequisites run individually in CI, so the Makefile-composed guarantee
/// is real for this gate and for the next one that joins `check` — a fifth gate added to
/// `check` with no matching CI step fails this test, because the prerequisite list is
/// parsed out of the Makefile rather than compared against a hardcoded literal. `coverage`
/// is excluded from the per-step, in-order check below: it runs once, in its own job, on
/// Linux only, and `coverage_gate_runs_exactly_once_and_only_on_linux` is what covers it.
#[test]
fn every_gate_the_makefile_composes_runs_in_ci() {
    let content = read_workflow();
    let makefile = read_makefile();
    let sections = job_sections(&content);
    let check = find_job(&sections, "check");

    let prereqs = parse_check_prereqs(&makefile);
    assert!(
        !prereqs.is_empty(),
        "check: rule names no prerequisites — the Makefile is unparseable"
    );
    assert!(
        prereqs.contains(&"gates".to_string()),
        "check: must compose gates — parsed prerequisites were {prereqs:?}"
    );

    let per_step_targets: Vec<&String> = prereqs
        .iter()
        .filter(|t| t.as_str() != "coverage")
        .collect();
    let mut positions = Vec::new();
    for target in &per_step_targets {
        let needle = format!("make {target}");
        let pos = check.text.find(&needle).unwrap_or_else(|| {
            panic!(
                "check job must run `{needle}` — check: composes {target}, parsed from the Makefile"
            )
        });
        positions.push((target.as_str(), pos));
    }
    for pair in positions.windows(2) {
        assert!(
            pair[0].1 < pair[1].1,
            "gates must run in the order check: composes them: {prereqs:?} (found {} after {})",
            pair[1].0,
            pair[0].0
        );
    }

    assert!(
        !content.contains("make check"),
        "workflow must not invoke the composite make check target"
    );
    // `coverage` runs once, in its own job — checked in full by
    // coverage_gate_runs_exactly_once_and_only_on_linux — but its presence somewhere in the
    // workflow is asserted here too, since it is one of check:'s own prerequisites.
    assert!(
        content.contains("make coverage"),
        "workflow must invoke `make coverage` at least once — coverage is one of check:'s prerequisites"
    );
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
        // A `run:` or `env:` key can open either a mapping entry (`  run: ...`) or a
        // YAML sequence item (`  - run: ...`, `- uses: ...` already occurs in this
        // workflow) — strip a leading `- ` before matching either key so the guard
        // cannot be defeated by writing the second, equally legal form.
        let trimmed = line.trim_start();
        let trimmed = trimmed.strip_prefix("- ").unwrap_or(trimmed);
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
        // Matches both the block form (`env:` alone) and a flow-style mapping on the
        // same line (`env: {RUSTFLAGS: "-A warnings"}`) — either neuters a gate
        // without touching any `run:` body.
        assert!(
            !trimmed.starts_with("env:"),
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
        4,
        "expected exactly four job sections (check, coverage, gates-full, ci), found {}",
        sections.len()
    );

    for line in &lines {
        let trimmed = line.trim_start();
        let trimmed = trimmed.strip_prefix("- ").unwrap_or(trimmed);
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

    let has_step_if_key = ci.text.lines().any(|l| {
        let t = l.trim();
        (t.starts_with("if:") || t.starts_with("- if:")) && t != "if: always()"
    });
    assert!(
        has_step_if_key,
        "`ci` job must have a step-level `if:` distinct from the job-level `if: always()`"
    );
    assert!(
        ci.text.contains("needs.*.result")
            && ci.text.contains("failure")
            && ci.text.contains("cancelled")
            && ci.text.contains("skipped"),
        "`ci` job's failing step must check failure, cancelled, and skipped across needs.*.result"
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

#[test]
fn gates_target_exists_and_names_both_scripts() {
    let makefile = read_makefile();
    let gates_line = makefile
        .lines()
        .find(|l| l.trim_start().starts_with("gates:") || l.trim_start().starts_with("gates :"))
        .expect("Makefile must declare a `gates:` target");
    // The recipe is the indented lines following the target line, up to the next
    // column-0 (unindented) line.
    let start = makefile.find(gates_line).unwrap() + gates_line.len();
    let recipe: String = makefile[start..]
        .lines()
        .skip(1)
        .take_while(|l| l.starts_with('\t') || l.starts_with(' ') || l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        recipe.contains("scripts/gates/deps.sh"),
        "gates: recipe must invoke scripts/gates/deps.sh, found: {recipe}"
    );
    assert!(
        recipe.contains("scripts/gates/build-graph.sh"),
        "gates: recipe must invoke scripts/gates/build-graph.sh, found: {recipe}"
    );

    let phony = phony_targets(&makefile);
    assert!(
        phony.iter().any(|t| t == "gates"),
        "gates must be declared .PHONY"
    );
    assert!(
        phony.iter().any(|t| t == "gates-full"),
        "gates-full must be declared .PHONY"
    );

    assert!(
        manifest_dir().join("scripts/gates/deps.sh").is_file(),
        "scripts/gates/deps.sh must exist and be a regular file"
    );
    assert!(
        manifest_dir()
            .join("scripts/gates/build-graph.sh")
            .is_file(),
        "scripts/gates/build-graph.sh must exist and be a regular file"
    );
}

/// The `gates:` recipe's own body — the indented lines following the `gates:` target line,
/// up to the next column-0 line. Shared by `gates_target_exists_and_names_both_scripts` and
/// `degraded_states`'s own correspondence test below (not refactored into one call site,
/// since the existing test's own inline copy is left byte-for-byte unchanged).
fn gates_recipe(makefile: &str) -> String {
    let gates_line = makefile
        .lines()
        .find(|l| l.trim_start().starts_with("gates:") || l.trim_start().starts_with("gates :"))
        .expect("Makefile must declare a `gates:` target");
    let start = makefile.find(gates_line).unwrap() + gates_line.len();
    makefile[start..]
        .lines()
        .skip(1)
        .take_while(|l| l.starts_with('\t') || l.starts_with(' ') || l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// `degraded-states` :: every script under `scripts/gates/` is named at least once in the
/// `gates:` recipe, and every `scripts/gates/<name>` the recipe names exists on disk — both
/// directions, so a script added without a recipe line and a recipe line naming a deleted
/// script are both caught. `EXTENDED` and `TESTCOUNT` are named nowhere in either direction
/// (design.md -> Decision 9): a per-change ratchet and a shell function respectively, neither
/// composed into `make gates`.
#[test]
fn every_gate_script_the_recipe_names_exists_and_every_script_is_named() {
    let makefile = read_makefile();
    let recipe = gates_recipe(&makefile);

    let dir = manifest_dir().join("scripts/gates");
    let on_disk: BTreeSet<String> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        on_disk.len() >= 25,
        "expected at least 25 files under scripts/gates/, found {}",
        on_disk.len()
    );

    // Every file on disk must be named in the recipe (direction 1).
    for name in &on_disk {
        assert!(
            recipe.contains(&format!("scripts/gates/{name}")),
            "scripts/gates/{name} exists but is not named anywhere in the gates: recipe"
        );
    }

    // Every `scripts/gates/<name>` the recipe mentions must exist on disk (direction 2).
    let mut named_in_recipe: BTreeSet<String> = BTreeSet::new();
    for token in recipe.split(|c: char| c.is_whitespace()) {
        if let Some(rest) = token.strip_prefix("scripts/gates/") {
            named_in_recipe.insert(rest.to_string());
            assert!(
                dir.join(rest).is_file(),
                "gates: recipe names scripts/gates/{rest}, which does not exist"
            );
        }
    }
    assert_eq!(
        on_disk, named_in_recipe,
        "the set of files under scripts/gates/ and the set the recipe names must match exactly"
    );

    // No script exists, and none is named, for the two deliberately-excluded gates.
    for excluded in ["extended.sh", "testcount.sh", "EXTENDED.sh", "TESTCOUNT.sh"] {
        assert!(
            !dir.join(excluded).exists(),
            "scripts/gates/{excluded} must not exist — EXTENDED and TESTCOUNT are deliberately \
             not extracted (design.md -> Decision 9)"
        );
    }
    assert!(
        !recipe.to_uppercase().contains("EXTENDED") && !recipe.to_uppercase().contains("TESTCOUNT"),
        "the gates: recipe must name neither EXTENDED nor TESTCOUNT: {recipe}"
    );
}

#[test]
fn check_composes_gates_third() {
    let makefile = read_makefile();
    let prereqs = parse_check_prereqs(&makefile);
    assert_eq!(
        prereqs,
        vec!["fmt-check", "lint", "gates", "test", "coverage"],
        "check: must compose fmt-check, lint, gates, test, coverage in that order"
    );
}

#[test]
fn gates_full_is_not_composed_into_check() {
    let makefile = read_makefile();
    let prereqs = parse_check_prereqs(&makefile);
    assert!(
        !prereqs.iter().any(|t| t == "gates-full"),
        "check: must not compose gates-full — it rebuilds the crate several times and gets \
         its own CI job instead, per design.md -> Decisions -> 3"
    );

    let gates_full_line = makefile
        .lines()
        .find(|l| {
            l.trim_start().starts_with("gates-full:") || l.trim_start().starts_with("gates-full :")
        })
        .expect("Makefile must declare a `gates-full:` target");
    // The recipe is the indented lines following the target line, up to the next
    // column-0 (unindented) line — the target line itself never carries the recipe body.
    let start = makefile.find(gates_full_line).unwrap() + gates_full_line.len();
    let recipe: String = makefile[start..]
        .lines()
        .skip(1)
        .take_while(|l| l.starts_with('\t') || l.starts_with(' ') || l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        recipe.contains("DEPS_FULL=1"),
        "gates-full: recipe must set DEPS_FULL=1, found: {recipe}"
    );
}

#[test]
fn both_runners_run_the_gates_step() {
    let content = read_workflow();
    let sections = job_sections(&content);
    let check = find_job(&sections, "check");

    let lint_pos = check
        .text
        .find("make lint")
        .expect("check job must run make lint");
    let gates_pos = check
        .text
        .find("make gates")
        .expect("check job must run make gates");
    let test_pos = check
        .text
        .find("make test")
        .expect("check job must run make test");
    assert!(
        lint_pos < gates_pos && gates_pos < test_pos,
        "the Gates step must sit between Lint and Test"
    );
    assert!(
        !check.text.contains("make gates-full"),
        "the check job must not invoke make gates-full"
    );
}

#[test]
fn gates_full_has_its_own_unconditional_job() {
    let content = read_workflow();
    let sections = job_sections(&content);
    let gates_full = find_job(&sections, "gates-full");

    assert!(
        gates_full.text.contains("make gates-full"),
        "gates-full job must invoke make gates-full"
    );
    assert!(
        gates_full.text.contains("runs-on: ubuntu-latest"),
        "gates-full job must run on ubuntu-latest"
    );
    assert!(
        gates_full.text.contains("timeout-minutes"),
        "gates-full job must declare timeout-minutes"
    );
    assert!(
        !gates_full.text.contains("strategy:"),
        "gates-full job must declare no strategy/matrix — it runs once"
    );
    let has_if = gates_full.text.lines().any(|l| {
        let t = l.trim();
        t.starts_with("if:") || t.starts_with("- if:")
    });
    assert!(!has_if, "gates-full job must carry no `if:` condition");
    assert!(
        !gates_full.text.contains("continue-on-error"),
        "gates-full job must not set continue-on-error"
    );
}

#[test]
fn gates_full_is_in_the_aggregate_needs() {
    let content = read_workflow();
    let needs_line = content
        .lines()
        .find(|l| l.trim_start().starts_with("needs:"))
        .expect("`ci` job must declare `needs:`");
    let needs_set = parse_needs_list(needs_line);
    assert!(
        needs_set.contains("gates-full"),
        "ci job's needs must list gates-full, found {needs_set:?}"
    );
}

/// `gate-integrity` :: D3. `AGENTS.md`'s Quality gates section names `EXTENDED` and
/// `TESTCOUNT` as deliberately not extracted, and `OPENSPEC-UNTOUCHED` as split, one line
/// of reason each. This is the prose clause the D3 scenario requires beside the
/// file-absence clause `every_gate_script_the_recipe_names_exists_and_every_script_is_named`
/// already checks: at the time this test was added, all three names returned zero hits
/// across `AGENTS.md`, `README.md`, and `SPEC.md` while that other test reported green,
/// because it only ever checked the directory and the recipe, never the document a reader
/// actually meets the exclusion in.
#[test]
fn agents_md_names_the_two_excluded_gates_and_the_split_one() {
    let agents = read_agents_md();
    let section = markdown_section(&agents, "## Quality gates");
    assert!(
        !section.trim().is_empty(),
        "AGENTS.md must have a `## Quality gates` section"
    );
    for name in ["EXTENDED", "TESTCOUNT", "OPENSPEC-UNTOUCHED"] {
        assert!(
            section.contains(name),
            "AGENTS.md's Quality gates section must name {name}"
        );
    }

    let dir = manifest_dir().join("scripts/gates");
    for excluded in ["extended.sh", "testcount.sh", "EXTENDED.sh", "TESTCOUNT.sh"] {
        assert!(
            !dir.join(excluded).exists(),
            "scripts/gates/{excluded} must not exist — EXTENDED and TESTCOUNT are \
             deliberately not extracted"
        );
    }
    assert!(
        dir.join("openspec-untouched.sh").is_file(),
        "scripts/gates/openspec-untouched.sh must exist — OPENSPEC-UNTOUCHED is split, not \
         wholly excluded, and the document must not describe it as excluded outright"
    );
}

/// `gate-integrity` :: D1. `SPEC.md` -> Gates names each of `check`'s prerequisites, in the
/// `Makefile`, **by name** — not merely as many rows, which five unrelated rows would
/// satisfy, and not the stale "all four" this test replaces, which a fifth prerequisite
/// joining `check` left uncorrected.
#[test]
fn spec_md_gates_section_names_every_check_prerequisite_by_name() {
    let spec = read_spec_md();
    let makefile = read_makefile();
    let prereqs = parse_check_prereqs(&makefile);
    assert!(
        !prereqs.is_empty(),
        "check: rule names no prerequisites — the Makefile is unparseable"
    );

    let section = markdown_section(&spec, "### Gates");
    assert!(!section.trim().is_empty(), "SPEC.md must have a `### Gates` section");

    for target in &prereqs {
        assert!(
            section.contains(target.as_str()),
            "SPEC.md's Gates section must name check's prerequisite `{target}` by name — a \
             count comparison is not sufficient, since renaming a row would leave it green: \
             section was {section:?}"
        );
    }

    assert!(
        !section.contains("all four"),
        "SPEC.md's Gates section must not describe the tier as \"all four\" — `check` \
         composes {} gates",
        prereqs.len()
    );
    assert!(
        section.contains("gates-full"),
        "SPEC.md's Gates section must name `gates-full` and that it runs in its own CI job \
         rather than inside `check`"
    );
}

/// `gate-integrity` :: ci-workflow, "The production floor reaches CI without a workflow
/// edit". The production-slice floor and the JSON report path it reads both live in the
/// `Makefile` alone, so either can change with no workflow edit — asserted here rather than
/// left to `coverage_threshold_is_not_restated_in_ci`, which only ever checked the total
/// floor's own flag.
#[test]
fn ci_yml_names_no_production_floor_or_report_path() {
    let content = read_workflow();
    for forbidden in [
        "PROD_MIN",
        "coverage-prod.py",
        "llvm-cov.json",
        "--output-path",
    ] {
        assert!(
            !content.contains(forbidden),
            "workflow must not name `{forbidden}` — the production floor and its JSON \
             report path live in the Makefile alone, so a floor change there changes CI \
             with no workflow edit"
        );
    }
}
