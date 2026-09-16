// Corpus tier: sweeps every real `tasks.md` under `openspec/changes/` —
// archived changes included — rather than a `&str` fixture. Two claims are
// about the archive itself and cannot be made against a hand-written literal:
// that retention moved no count, and that retention is total, every non-blank
// source line landing in exactly one of an item's body, a group's block, a
// heading, or an item. Both live here rather than in `src/tasks.rs` so that no
// file under `src/` gains a run-time filesystem read (design.md -> Test
// Boundaries).
//
// Every sweep prints the number of files it visited and fails on a zero sweep:
// a glob under a mistyped path finds nothing and exits clean, which is exactly
// the failure mode these guards exist for.

use herdr_openspec::tasks::{self, Progress, Tasks};

/// The crate root, resolved at compile time. `std::env::current_dir()` is not
/// used: `cargo test` sets the working directory to the package root today,
/// but `CARGO_MANIFEST_DIR` says so rather than assuming it.
fn crate_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every `tasks.md` under `openspec/changes/` and `openspec/changes/archive/`,
/// as paths relative to the crate root, sorted. The same set
/// `scripts/measure-tasks-corpus.py` walks, so its measured figures and these
/// tests are about the same files.
fn corpus() -> Vec<String> {
    let root = crate_root();
    let mut found = Vec::new();
    for dir in ["openspec/changes", "openspec/changes/archive"] {
        let entries = match std::fs::read_dir(root.join(dir)) {
            Ok(entries) => entries,
            Err(e) => panic!("{dir} is not readable: {e}"),
        };
        for entry in entries {
            let entry = entry.expect("read a directory entry");
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == "archive" {
                continue;
            }
            let relative = format!("{dir}/{name}/tasks.md");
            if root.join(&relative).is_file() {
                found.push(relative);
            }
        }
    }
    found.sort();
    found
}

/// Read one corpus file, or fail naming it.
fn read(relative: &str) -> String {
    let path = crate_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} unreadable: {e}", path.display()))
}

/// `tests/fixtures/task-counts.txt`: one `path<TAB>completed<TAB>total` row per
/// corpus file, produced by `count` **before** retention existed. The
/// independent side of "Retention leaves every count in the archive unmoved" —
/// regenerated after the parse edit it would agree with a wrong implementation
/// by construction.
fn recorded_counts() -> Vec<(String, Progress)> {
    let text = read("tests/fixtures/task-counts.txt");
    let mut rows = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields.len(), 3, "malformed fixture row {line:?}");
        rows.push((
            fields[0].to_string(),
            Progress {
                completed: fields[1].parse().expect("completed is a number"),
                total: fields[2].parse().expect("total is a number"),
            },
        ));
    }
    rows
}

/// True when `line` is an ATX heading by `task-groups`' rule: one to six `#`
/// characters at column zero followed by a space or the end of the line.
/// Restated here rather than reached for, because `tasks::heading_line` is
/// private and this file is a separate crate — the point of the partition test
/// is that an independent classifier and the parser agree.
fn is_heading(line: &str) -> bool {
    let hashes = line.chars().take_while(|&c| c == '#').count();
    (1..=6).contains(&hashes) && matches!(line[hashes..].chars().next(), None | Some(' '))
}

/// True when `line` is a task line — asked of `count`, the rule's one
/// authority, which this change does not touch.
fn is_task(line: &str) -> bool {
    tasks::count(line).total == 1
}

/// Every non-blank line of `text` that is neither a heading nor a task line,
/// trimmed, in document order: what retention must place in a body or a block.
fn source_other_lines(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in text.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if line.trim().is_empty() || is_heading(line) || is_task(line) {
            continue;
        }
        out.push(line.trim().to_string());
    }
    out
}

/// Every non-blank line a parse **retained**, trimmed, in document order:
/// within each group, the blocks recorded before item `k` come first, then item
/// `k`'s own body, so the sequence is the document's own order. Trimmed on both
/// sides because a body is dedented to its own shallowest line and a block is
/// not, and the claim under test is about lines, not columns.
fn retained_other_lines(parsed: &Tasks) -> Vec<String> {
    let mut out = Vec::new();
    for group in &parsed.groups {
        for k in 0..=group.items.len() {
            for b in group.blocks.iter().filter(|b| b.after == k) {
                out.extend(b.text.lines().map(str::to_string));
            }
            if let Some(item) = group.items.get(k) {
                out.extend(item.body.lines().map(str::to_string));
            }
        }
    }
    out.into_iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_string())
        .collect()
}

/// `task-groups` :: "Retention leaves every count in the archive unmoved".
#[test]
fn retention_leaves_every_count_in_the_archive_unmoved() {
    let files = corpus();
    println!(
        "retention_leaves_every_count_in_the_archive_unmoved: swept {} files",
        files.len()
    );
    assert!(
        !files.is_empty(),
        "the corpus sweep selected zero files: a mistyped path finds nothing and passes vacuously"
    );

    for relative in &files {
        let text = read(relative);
        assert_eq!(
            tasks::parse(&text).progress(),
            tasks::count(&text),
            "{relative}: parse and count disagree"
        );
    }

    let recorded = recorded_counts();
    assert!(!recorded.is_empty(), "the count fixture holds no rows");
    let mut matched = 0;
    for (relative, expected) in &recorded {
        let text = read(relative);
        assert_eq!(
            tasks::count(&text),
            *expected,
            "{relative}: count moved against the pre-retention fixture"
        );
        assert_eq!(
            tasks::parse(&text).progress(),
            *expected,
            "{relative}: parse moved against the pre-retention fixture"
        );
        matched += 1;
    }
    println!("  checked {matched} recorded pairs");
    assert_eq!(
        matched,
        recorded.len(),
        "not every recorded row was checked"
    );
}

/// `task-groups` :: "Every retained line appears exactly once".
#[test]
fn every_retained_line_appears_exactly_once() {
    let files = corpus();
    println!(
        "every_retained_line_appears_exactly_once: swept {} files",
        files.len()
    );
    assert!(
        !files.is_empty(),
        "the corpus sweep selected zero files: a mistyped path finds nothing and passes vacuously"
    );

    let mut lines_compared = 0;
    for relative in &files {
        let text = read(relative);
        let parsed = tasks::parse(&text);

        // The partition, asserted in both directions at once: a sequence
        // equality fails both when a source line is missing from the parse and
        // when the parse holds a line the source does not, and pins the order.
        let expected = source_other_lines(&text);
        let retained = retained_other_lines(&parsed);
        assert_eq!(
            retained, expected,
            "{relative}: retention is not a partition"
        );
        lines_compared += expected.len();

        // The other two thirds of the partition: no heading and no item line
        // was turned into a block or a body along the way.
        let source_headings = text
            .split('\n')
            .map(|l| l.strip_suffix('\r').unwrap_or(l))
            .filter(|l| is_heading(l))
            .count();
        assert_eq!(
            parsed.groups.iter().filter(|g| g.heading.is_some()).count(),
            source_headings,
            "{relative}: heading count moved"
        );
        let source_items = text
            .split('\n')
            .map(|l| l.strip_suffix('\r').unwrap_or(l))
            .filter(|l| is_task(l))
            .count();
        assert_eq!(
            parsed.groups.iter().map(|g| g.items.len()).sum::<usize>(),
            source_items,
            "{relative}: item count moved"
        );
    }

    println!("  compared {lines_compared} retained lines");
    assert!(
        lines_compared > 0,
        "no non-task line was compared: the partition assertion proved nothing"
    );
}

/// `task-labels` :: "The exposed skip agrees with the one `label_of` performs".
///
/// The binding that makes `task_number_len` a second *reader* of the skip rule
/// rather than a second *copy* of it: asserted over every item text in the
/// archive, so a divergent copy that only differed on an unusual number —
/// a three-segment number, a lettered one, a bare `.` — could not pass.
#[test]
fn the_exposed_skip_agrees_with_the_one_label_of_performs() {
    let files = corpus();
    println!(
        "the_exposed_skip_agrees_with_the_one_label_of_performs: swept {} files",
        files.len()
    );
    assert!(
        !files.is_empty(),
        "the corpus sweep selected zero files: a mistyped path finds nothing and passes vacuously"
    );

    let mut items_seen = 0;
    let mut labelled = 0;
    for relative in &files {
        let text = read(relative);
        for group in tasks::parse(&text).groups {
            for item in group.items {
                items_seen += 1;
                let n = tasks::task_number_len(&item.text);

                // Totality, over real text rather than literals: never past the
                // end, and always on a character boundary, so `split_at` stands
                // in for the boundary assertion by panicking otherwise.
                assert!(
                    n <= item.text.len(),
                    "{relative}: {:?} skipped {n} of {} bytes",
                    item.text,
                    item.text.len()
                );
                let _ = item.text.split_at(n);

                if let Some(label) = tasks::label_of(&item.text) {
                    assert_eq!(
                        label.start, n,
                        "{relative}: label_of and task_number_len disagree on {:?}",
                        item.text
                    );
                    labelled += 1;
                }
            }
        }
    }

    println!("  compared {labelled} labelled items of {items_seen} item texts");
    assert!(
        labelled > 0,
        "no labelled item was compared: the agreement assertion proved nothing"
    );
}
