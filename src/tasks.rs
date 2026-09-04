//! Markdown task-file parsing: which lines are tasks, whether each is
//! checked, and how they group under headings.
//!
//! See `openspec/changes/task-parsing/design.md` for the full contract.

#[cfg(test)]
mod tests {
    // Group 1: `Progress` and the flat line rule. Every test here drives
    // `count` only — `parse` does not exist until group 3, which picks up
    // the `parse` half of the three scenarios that carry both clauses (see
    // tasks.md group 1 preamble and 3.2).

    #[test]
    fn the_four_canonical_shapes_are_task_lines() {
        let text = "- [ ] a\n- [x] b\n* [ ] c\n* [x] d";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 2,
                total: 4
            }
        );
    }

    #[test]
    fn whitespace_around_the_bullet_and_the_box_is_optional() {
        let text = "-[x]done\n   *   [ ]   todo   ";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 1,
                total: 2
            }
        );
    }

    #[test]
    fn a_plus_bullet_and_an_ordered_marker_are_not_task_lines() {
        let text = "+ [x] plus\n1. [x] ordered\n1) [x] paren";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 0,
                total: 0
            }
        );
    }

    #[test]
    fn a_malformed_box_is_not_a_task_line() {
        // Each line paired with a genuine task line so an implementation
        // matching nothing at all cannot pass by returning zero.
        let text = "- [] empty\n- [x] real1\n\
                     - [-] dash\n- [x] real2\n\
                     - [xx] two\n- [x] real3\n\
                     - [x extra\n- [x] real4\n\
                     - ( ) round\n- [x] real5";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 5,
                total: 5
            }
        );
    }

    #[test]
    fn a_bullet_with_no_checkbox_and_prose_that_mentions_one_are_not_task_lines() {
        let text = "- an ordinary list item\n- [x] real1\n\
                     text - [x] mid-line\n- [x] real2\n\
                     -- [x] two dashes\n- [x] real3";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 3,
                total: 3
            }
        );
    }

    #[test]
    fn both_letter_cases_count_as_done() {
        let text = "- [x] lower\n- [X] upper";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 2,
                total: 2
            }
        );
    }

    #[test]
    fn a_space_a_tab_and_a_non_breaking_space_are_all_empty_boxes() {
        let text = "- [\u{20}] space\n- [\u{9}] tab\n- [\u{a0}] nbsp";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 0,
                total: 3
            }
        );
    }

    #[test]
    fn a_checkbox_inside_a_fenced_code_block_counts() {
        let text = "```text\n- [x] example\n```\n- [ ] real";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 1,
                total: 2
            }
        );
    }

    #[test]
    fn a_checkbox_inside_an_html_comment_counts() {
        let text = "<!--\n- [x] commented out\n-->\n- [ ] real";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 1,
                total: 2
            }
        );
    }

    #[test]
    fn a_block_quoted_checkbox_does_not_count() {
        // `>` is not in the bullet alphabet: no block-quote rule exists to
        // go wrong, this is a guard pinning a non-feature, not a driver of
        // new behaviour.
        let text = "> - [x] quoted";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 0,
                total: 0
            }
        );
    }

    #[test]
    fn the_last_line_counts_without_a_trailing_newline() {
        let text = "- [x] only";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 1,
                total: 1
            }
        );
    }

    #[test]
    fn a_document_whose_only_checkbox_is_on_the_last_line_after_a_trailing_newline() {
        let text = "prose\n\n- [x] last\n";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 1,
                total: 1
            }
        );
    }

    #[test]
    fn an_empty_string_has_no_tasks() {
        let progress = super::count("");
        assert_eq!(
            progress,
            super::Progress {
                completed: 0,
                total: 0
            }
        );
    }

    #[test]
    fn a_document_of_blank_lines_and_prose_has_no_tasks() {
        let text = "\n\n   \nSome prose that mentions nothing checkbox-shaped.\n\n";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 0,
                total: 0
            }
        );
    }

    #[test]
    fn is_complete_is_true_at_three_of_three() {
        let progress = super::count("- [x] a\n- [x] b\n- [x] c");
        assert!(progress.is_complete());
    }

    #[test]
    fn is_complete_is_false_at_zero_of_zero() {
        let progress = super::count("no tasks here");
        assert!(!progress.is_complete());
    }

    #[test]
    fn progress_values_from_several_files_sum() {
        let a = super::Progress {
            completed: 1,
            total: 3,
        };
        let b = super::Progress {
            completed: 2,
            total: 2,
        };
        assert_eq!(
            a + b,
            super::Progress {
                completed: 3,
                total: 5
            }
        );

        let mut acc = super::Progress {
            completed: 0,
            total: 0,
        };
        acc += a;
        acc += b;
        assert_eq!(
            acc,
            super::Progress {
                completed: 3,
                total: 5
            }
        );
    }

    #[test]
    fn a_file_derived_count_and_a_cli_shaped_count_compare_equal() {
        let text = "- [x] a\n- [ ] b\n- [ ] c\n- [ ] d";
        let from_file = super::count(text);
        let from_cli = super::Progress {
            completed: 1,
            total: 4,
        };
        assert_eq!(from_file, from_cli);
    }
}
