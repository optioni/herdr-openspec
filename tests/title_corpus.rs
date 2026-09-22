//! `title-heading-preamble`'s corpus guard: every committed `tasks.md` under
//! `openspec/changes/` (active and `archive/` alike) that has at least one
//! task item is checked against a **regression**, not against an absolute
//! rule — the title rule (`Dashboard::sync_detail`'s private `title_heading`
//! helper) must not change whether the file *splits* relative to what it did
//! before this change existed.
//!
//! This drives the *real* derivation end to end rather than re-deriving the
//! rule: it builds a one-artifact `Dashboard` with `tracks_tasks: true` and
//! calls the public `Dashboard::sync_detail` with a closure that returns the
//! file's own bytes, then compares `detail.sections.len() > 1` against the
//! same file's **pre-change** split decision. `title_heading` itself is
//! private, so a guard that re-parsed headings and recomputed its three
//! clauses (`design.md` -> Boundaries) would be a second implementation of
//! the rule under guard — green even if `title_heading` were never written,
//! or were written wrong. Driving `sync_detail` is the one way to avoid that.
//!
//! **Selection rule** (this is the "which files" choice `tasks.md`'s task
//! 2.2 asks to be stated and justified): every `tasks.md` with at least one
//! task item, i.e. `tasks::count(text).total > 0`. That is exactly the split
//! gate's own precondition for a tracked-tasks artifact
//! (`sync_detail`: `splits = (tracks_tasks && tasks::count(&text).total > 0
//! || has_requirement_heading(..)) && (base > 0 || contributions > 1)`) —
//! not a restatement of what the title rule demotes. A file with no task
//! items would never split regardless of title demotion, so including it
//! would tell the guard nothing about the title rule; restricting to files
//! that clear that same precondition is the narrowest selection that still
//! drives the real code path for every file the title rule could possibly
//! affect, without re-deriving `title_heading`'s own three clauses (title
//! index found, per `design.md`'s "sole heading at the shallowest level, no
//! task items in its own body" test, and non-empty trimmed body) to decide
//! membership up front.
//!
//! **Why a regression, not `sections.len() > 1` outright**: every committed
//! `tasks.md` with task items happening to split is stronger than the rule
//! this guard exists to protect. An untitled, one-group file such as
//! `## 1. Only\n\n- [ ] 1.1 x\n` never split before this change either
//! (`a_single_heading_task_file_is_not_split_and_keeps_its_heading` asserts
//! exactly that, unaffected by title demotion since it carries no title at
//! all) — flagging it would blame this change for a pre-existing, correct
//! non-split, and would turn `make check` red for any future change that
//! happens to draft such a file mid-flight. What the title rule can actually
//! break is a file whose split decision **changes** once a title is demoted
//! out of the heading count, so that is what this guard compares:
//! `pre_change_contributions` re-derives the split gate's contribution count
//! exactly as `sync_detail` computed it **before** this change — every
//! heading `split_headings` returns, plus one when the text before the first
//! heading is non-empty (the OLD, byte-emptiness preamble predicate, not the
//! trimmed one `title_heading`'s own body check uses) — a file always has a
//! single path here, so `base` is always `0` and `contributions > 1` is
//! exactly the old `splits` value. This re-derives the *old* formula, not
//! `title_heading`'s three clauses, so it stays a regression check rather
//! than a second implementation of the rule under guard.
//!
//! Lives in `tests/` rather than beside `src/ui/app.rs`'s own unit tests
//! because `scripts/gates/noio-view.sh`'s `PURE` sweep covers `src/ui/app.rs`
//! and greps the whole file, `#[cfg(test)]` modules included — a walk of
//! `openspec/changes/` with `std::fs` living there would trip it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use herdr_openspec::agents::AgentSnapshot;
use herdr_openspec::changes::{ArtifactRef, Change, ChangeSet, Origin};
use herdr_openspec::state::Mapping;
use herdr_openspec::tasks::Progress;
use herdr_openspec::ui::app::{
    Dashboard, Detail, Filter, Launch, Overlay, Refresh, Route, Sections,
};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Recursively collect every `tasks.md` under `dir`, depth-first, in no
/// particular order — the assertion below is per-file and order-independent.
fn find_tasks_md(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            find_tasks_md(&path, out);
        } else if path.file_name().and_then(|n| n.to_str()) == Some("tasks.md") {
            out.push(path);
        }
    }
}

/// The split gate's contribution count **before** `title-heading-preamble`:
/// every heading `split_headings` (public, and this guard's one call into
/// the real derivation beyond `Dashboard::sync_detail` itself) returns, plus
/// one when the text before the first heading line is non-empty. That
/// preamble offset is derived from `sections` the same way `sync_detail`'s
/// own (private) `preamble_len` does — walking backwards from the end of
/// `text`, subtracting each section's own `body` length in reverse and
/// stepping back over the heading line above it — because the two must
/// agree on where a heading line starts for this to be the same preamble
/// the old code measured. This is deliberately the OLD, byte-emptiness
/// predicate (`!preamble.is_empty()`), not the trimmed-whitespace one
/// `title_heading`'s own body check uses, because this function's whole job
/// is to reproduce what `sync_detail` computed **before** this change, not
/// to restate the new rule.
fn pre_change_contributions(text: &str) -> usize {
    let headings = herdr_openspec::ui::app::split_headings(text);
    let mut pos = text.len();
    for section in headings.iter().rev() {
        pos = pos.saturating_sub(section.body.len());
        let Some(head) = text.get(..pos) else {
            pos = text.len();
            break;
        };
        let line = head.strip_suffix('\n').unwrap_or(head);
        pos = line.rfind('\n').map_or(0, |i| i + 1);
    }
    let has_preamble = !text.get(..pos).unwrap_or_default().is_empty();
    headings.len() + usize::from(has_preamble)
}

/// A one-artifact, one-active-change `Dashboard` selecting that change, with
/// its sole artifact marked `tracks_tasks: true` and pointed at `path`.
/// `selected: 1` addresses `targets()`'s second entry — `Target::Section(Active)`
/// is first since the active tier is non-empty and open by default
/// (`sections.collapsed` is empty), then `Target::Change(0)` — the same
/// indexing `tests/doc_contract.rs`'s `sweep_dashboard` fixture relies on.
fn one_artifact_dashboard(path: &Path) -> Dashboard {
    let change = Change {
        name: "corpus-probe".to_string(),
        dir: path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("/corpus-probe")),
        origin: Origin::Active,
        schema: "tdd".to_string(),
        artifacts: vec![ArtifactRef {
            id: "tasks".to_string(),
            paths: vec![path.to_path_buf()],
            tracks_tasks: true,
        }],
        progress: Progress {
            completed: 0,
            total: 0,
        },
        problems: Vec::new(),
    };
    Dashboard {
        settings: herdr_openspec::settings::PanelState {
            rows: Vec::new(),
            cursor: 0,
        },
        selection: None,
        repo: Some(PathBuf::from("/repo")),
        searched_from: PathBuf::from("/repo"),
        changes: ChangeSet {
            active: vec![change],
            archived: Vec::new(),
            problems: Vec::new(),
            archived_total: 0,
        },
        route: Route::Detail,
        quit: false,
        selected: 1,
        filter: Filter {
            query: String::new(),
            active: false,
        },
        detail: Detail {
            sections: Vec::new(),
            scroll: 0,
            tab: 0,
            problems: Vec::new(),
            loaded: None,
            expanded: BTreeSet::new(),
            drawn_width: None,
        },
        refresh: Refresh {
            requested: false,
            reload: false,
            startup: Vec::new(),
            problems: Vec::new(),
        },
        agents: AgentSnapshot {
            agents: Vec::new(),
            reachable: true,
            stalled: false,
            problem: None,
        },
        agent_names: Mapping {
            names: BTreeMap::new(),
            problems: Vec::new(),
        },
        launch: Launch {
            pending: None,
            problems: Vec::new(),
            in_flight: false,
        },
        sections: Sections {
            collapsed: BTreeSet::new(),
        },
        file_mode: false,
        overlay: Overlay {
            panel: None,
            scroll: 0,
            edit: None,
        },
    }
}

/// Flags a file only when its split decision **changed** under title
/// demotion — see the module doc comment's "Why a regression, not
/// `sections.len() > 1` outright" — never a file that never split before
/// this change either.
#[test]
fn no_committed_task_file_changes_its_split_decision_under_title_demotion() {
    let root = manifest_dir().join("openspec").join("changes");
    let mut files = Vec::new();
    find_tasks_md(&root, &mut files);

    let mut scanned = 0usize;
    let mut offenders = Vec::new();

    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        // The split gate's own precondition for a tracked-tasks artifact:
        // a file with no task items never splits regardless of title
        // demotion, so it is not part of this guard's corpus.
        if herdr_openspec::tasks::count(&text).total == 0 {
            continue;
        }
        scanned += 1;

        let pre_split = pre_change_contributions(&text) > 1;

        let mut dashboard = one_artifact_dashboard(path);
        let bytes = text.clone();
        let read = |_: &Path| -> Result<String, String> { Ok(bytes.clone()) };
        dashboard.sync_detail(&read);
        let post_split = dashboard.detail.sections.len() > 1;

        if post_split != pre_split {
            offenders.push(format!(
                "{} (pre-change split: {pre_split}, post-change split: {post_split}, sections: {})",
                path.display(),
                dashboard.detail.sections.len()
            ));
        }
    }

    println!("title_corpus: scanned {scanned} tasks.md file(s) with at least one task item");

    assert!(
        scanned > 0,
        "found zero tasks.md files with task items under {}; the walk is broken",
        root.display()
    );
    assert!(
        offenders.is_empty(),
        "title demotion changes the split decision for {} file(s):\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}
