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
        for run in ["and", "When", "WHENEVER", "OR", "BUT", "IF", "Requirement", ""] {
            assert_eq!(clause_of(run), None, "{run:?}");
        }
        // `and` and `When` return `None` while `AND` and `WHEN` do not, so
        // matching is case-sensitive and whole-run.
        assert!(clause_of("AND").is_some());
        assert!(clause_of("WHEN").is_some());
    }
}
