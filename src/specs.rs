//! Recognising the structural vocabulary OpenSpec itself writes into a delta
//! spec: the three delta-operation headings (`## ADDED Requirements` and its
//! two siblings), and the keyword that opens a scenario clause (`- **WHEN**`,
//! `- **THEN**`, `- **AND**`). Two pure total functions over borrowed text,
//! plus the [`DeltaOp`] vocabulary. `ui::app` owns the attribution walk that
//! turns the first into a per-section operation and the header row that
//! draws it, `ui::markdown` faces a clause keyword from the second, and
//! `ui::palette` decides what each looks like. This module classifies one
//! heading and one run; it never walks a section list. See
//! `openspec/changes/spec-emphasis/specs/spec-delta-badges/spec.md`.
//!
//! It lives outside `src/ui/` for exactly the reason `crate::tasks` does:
//! recognising a heading is a fact about a spec's text and not about how a
//! frame is painted, and a new pure-view file would move a count that
//! `view-palette` and `responsive-layout` both bind and that three gate
//! scripts carry as a `PURE` list — five sites for a function that needs
//! none of them. This module classifies and never styles, never reads a
//! file, and never consults any workflow definition — see `AGENTS.md` ->
//! Architecture rules.

use crate::tasks::LabelRole;

/// A requirement's delta operation, as OpenSpec's own `##` heading names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaOp {
    Added,
    Modified,
    Removed,
}

/// The delta operation the heading at `level` with label `label` names, or
/// `None` when it names none. `label` is the heading's remainder with
/// surrounding whitespace trimmed, exactly as `HeadingSection.label` carries
/// it.
///
/// `Some` only when **both** hold:
///
/// 1. `level` is exactly `2` — the operation headings OpenSpec writes are
///    `##` headings, and the check lives inside this function so it is not a
///    caller's to remember.
/// 2. `label` splits on ASCII whitespace into exactly two tokens, the second
///    of which is exactly `Requirements`, and the first of which is exactly
///    `ADDED`, `MODIFIED`, or `REMOVED`, matched case-sensitively.
///
/// Splitting on whitespace rather than comparing the whole string is the
/// only tolerance offered: `##  ADDED   Requirements` classifies, while
/// `## Added Requirements`, `## ADDED Requirement`, and
/// `## ADDED Requirements (2)` do not. `RENAMED Requirements` — a fourth
/// operation OpenSpec's own instructions name, unseen across this
/// repository's archive — classifies to `None` like any other heading this
/// crate has never rendered: it loses the badge and keeps every other thing
/// the detail region already draws.
///
/// Total: never panics, for any `u8` and any `&str`.
pub fn operation_of_heading(level: u8, label: &str) -> Option<DeltaOp> {
    if level != 2 {
        return None;
    }
    let mut tokens = label.split_ascii_whitespace();
    let first = tokens.next()?;
    let second = tokens.next()?;
    if second != "Requirements" || tokens.next().is_some() {
        return None;
    }
    match first {
        "ADDED" => Some(DeltaOp::Added),
        "MODIFIED" => Some(DeltaOp::Modified),
        "REMOVED" => Some(DeltaOp::Removed),
        _ => None,
    }
}

/// What a bold run at the head of a list item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clause {
    /// A keyword naming its own lifecycle position.
    Opens(LabelRole),
    /// A continuation, carrying the position of the clause above it.
    Continues,
}

/// `run`'s clause classification, or `None` when it is not a clause keyword.
/// Classified against the **whole** run, with no trimming, no case folding,
/// and no prefix matching:
///
/// 1. `AND` is `Some(Clause::Continues)`.
/// 2. Otherwise `run` is looked up in the `task-labels` token table through
///    [`crate::tasks::role_of`], and a hit is `Some(Clause::Opens(role))` —
///    so `WHEN` reports `Opens(Change)` and `THEN` reports `Opens(Confirm)`,
///    and `GIVEN`, `ARRANGE`, `ACT`, and `ASSERT` classify identically to the
///    conventions they belong to.
/// 3. Every other run is `None`.
///
/// The table is reached and never copied: `crate::tasks::role_of` is the
/// crate's one lifecycle-token table, and a spec's `WHEN` and a task's
/// `WHEN` are one fact, not two implementations of it that could drift
/// (design.md -> Decision 3).
///
/// Total: never panics, for any `&str`.
pub fn clause_of(run: &str) -> Option<Clause> {
    if run == "AND" {
        return Some(Clause::Continues);
    }
    crate::tasks::role_of(run).map(Clause::Opens)
}

#[cfg(test)]
mod tests {
    use super::{Clause, DeltaOp, clause_of, operation_of_heading};
    use crate::tasks::LabelRole;

    /// `spec-delta-badges` :: "Each of the three operation headings
    /// classifies to its own variant".
    #[test]
    fn each_of_the_three_operation_headings_classifies_to_its_own_variant() {
        assert_eq!(
            operation_of_heading(2, "ADDED Requirements"),
            Some(DeltaOp::Added)
        );
        assert_eq!(
            operation_of_heading(2, "MODIFIED Requirements"),
            Some(DeltaOp::Modified)
        );
        assert_eq!(
            operation_of_heading(2, "REMOVED Requirements"),
            Some(DeltaOp::Removed)
        );
        // The assertion discriminates: a function collapsing two variants
        // could not pass this.
        assert_ne!(
            operation_of_heading(2, "ADDED Requirements"),
            Some(DeltaOp::Modified)
        );
    }

    /// `spec-delta-badges` :: "Internal whitespace is tolerated and nothing
    /// else is".
    #[test]
    fn internal_whitespace_is_tolerated_and_nothing_else_is() {
        assert_eq!(
            operation_of_heading(2, "ADDED   Requirements"),
            Some(DeltaOp::Added)
        );
        assert_eq!(
            operation_of_heading(2, "MODIFIED\tRequirements"),
            Some(DeltaOp::Modified)
        );
        for label in [
            "Added Requirements",
            "ADDED Requirement",
            "ADDED Requirements (2)",
            "ADDEDRequirements",
            "Requirements ADDED",
            "ADDED",
        ] {
            assert_eq!(operation_of_heading(2, label), None, "{label:?}");
        }
    }

    /// `spec-delta-badges` :: "Only a level-2 heading carries an operation".
    #[test]
    fn only_a_level_2_heading_carries_an_operation() {
        for level in [0u8, 1, 3, 4, 5, 6, 255] {
            assert_eq!(
                operation_of_heading(level, "ADDED Requirements"),
                None,
                "level {level}"
            );
        }
        // The same label at level 2 returns `Some`, so the level is what
        // discriminates and not the label.
        assert_eq!(
            operation_of_heading(2, "ADDED Requirements"),
            Some(DeltaOp::Added)
        );
    }

    /// `spec-delta-badges` :: "A renamed operation and a main spec's heading
    /// both decline".
    #[test]
    fn a_renamed_operation_and_a_main_specs_heading_both_decline() {
        for label in ["RENAMED Requirements", "Requirements", "Purpose"] {
            assert_eq!(operation_of_heading(2, label), None, "{label:?}");
        }
    }

    /// `spec-delta-badges` :: "The recognition is total over degenerate
    /// input".
    #[test]
    fn the_recognition_is_total_over_degenerate_input() {
        let long_run = "A".repeat(10_000);
        for label in [
            "",
            "   ",
            "\t\n",
            long_run.as_str(),
            "日本語 Requirements",
            "ADDED Requirements 日本語",
        ] {
            // No call panics, and every call returns `None`.
            assert_eq!(operation_of_heading(2, label), None, "{label:?}");
        }
    }

    /// `spec-delta-badges` :: "The three measured keywords classify as
    /// specified".
    #[test]
    fn the_three_measured_keywords_classify_as_specified() {
        assert_eq!(clause_of("WHEN"), Some(Clause::Opens(LabelRole::Change)));
        assert_eq!(clause_of("THEN"), Some(Clause::Opens(LabelRole::Confirm)));
        assert_eq!(clause_of("AND"), Some(Clause::Continues));
        // `WHEN` and `THEN` report different roles, so a function
        // collapsing them could not pass.
        assert_ne!(clause_of("WHEN"), clause_of("THEN"));
    }

    /// `spec-delta-badges` :: "The wider testing vocabulary classifies
    /// through the same table".
    #[test]
    fn the_wider_testing_vocabulary_classifies_through_the_same_table() {
        for (run, role) in [
            ("GIVEN", LabelRole::Evidence),
            ("ARRANGE", LabelRole::Evidence),
            ("ACT", LabelRole::Change),
            ("ASSERT", LabelRole::Confirm),
            ("RED", LabelRole::Evidence),
        ] {
            assert_eq!(clause_of(run), Some(Clause::Opens(role)), "{run:?}");
            // Each agrees with `crate::tasks::role_of` called on the same
            // run: the two cannot disagree without failing.
            assert_eq!(crate::tasks::role_of(run), Some(role), "{run:?}");
        }
    }

    /// `spec-delta-badges` :: "A run outside the table is not a clause".
    #[test]
    fn a_run_outside_the_table_is_not_a_clause() {
        for run in [
            "and",
            "When",
            "WHENEVER",
            "OR",
            "BUT",
            "IF",
            "Requirement",
            "",
        ] {
            assert_eq!(clause_of(run), None, "{run:?}");
        }
        // `and` and `When` return `None` while `AND` and `WHEN` do not, so
        // matching is case-sensitive and whole-run.
        assert!(clause_of("AND").is_some());
        assert!(clause_of("WHEN").is_some());
    }
}
