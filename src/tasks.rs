//! Markdown task-file parsing: which lines are tasks, whether each is
//! checked, and how they group under headings.
//!
//! See `openspec/changes/task-parsing/design.md` for the full contract.

/// A checkbox count: how many task lines a document holds and how many are
/// checked. The shape both sources of a change's task progress must produce
/// — `changes-from-files` obtains it by counting checkboxes, `changes-from-cli`
/// by reading `completedTasks`/`totalTasks` out of `openspec list --json` — so
/// the two can be compared with `==` and summed with no conversion and no
/// third state. See `openspec/changes/task-parsing/design.md` -> Contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub completed: usize,
    pub total: usize,
}

impl Progress {
    /// True only when there is at least one task and every task is checked.
    /// A change with no tasks is deliberately not "complete" — matching the
    /// CLI's own three-way "No tasks" / "n/m tasks" / "✓ Complete" split.
    pub fn is_complete(&self) -> bool {
        self.total > 0 && self.completed == self.total
    }
}

impl std::ops::Add for Progress {
    type Output = Progress;

    fn add(self, rhs: Progress) -> Progress {
        Progress {
            completed: self.completed + rhs.completed,
            total: self.total + rhs.total,
        }
    }
}

impl std::ops::AddAssign for Progress {
    fn add_assign(&mut self, rhs: Progress) {
        self.completed += rhs.completed;
        self.total += rhs.total;
    }
}

/// The flat checkbox count for `text`. Deliberately identical to the
/// OpenSpec CLI's own `countTasksFromContent` (`dist/utils/task-progress.js`,
/// `@fission-ai/openspec` 1.11.0): a `-` or `*` bullet at any indent carrying
/// a one-character `[ ]` / `[x]` / `[X]` box, with no code-fence, comment, or
/// block-quote exemption. See design.md -> Decisions 1 for why the rule is
/// copied rather than invented.
pub fn count(text: &str) -> Progress {
    let mut progress = Progress {
        completed: 0,
        total: 0,
    };
    for raw_line in text.split('\n') {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if let Some(checked) = task_line_checked(line) {
            progress.total += 1;
            if checked {
                progress.completed += 1;
            }
        }
    }
    progress
}

/// If `line` is a task line, whether it is checked. `None` when the line is
/// not a task line at all. A single left-to-right scan: skip leading
/// whitespace, require exactly one `-` or `*`, skip whitespace, require `[`,
/// take exactly one character, require `]`. No fence, comment, or
/// block-quote state — see design.md -> Decisions 6.
///
/// Whitespace here is `char::is_whitespace`, the standard-library predicate;
/// it satisfies every scenario this group owns. Group 2 narrows it to the
/// CLI's own alphabet.
fn task_line_checked(line: &str) -> Option<bool> {
    let mut chars = line.chars();
    skip_whitespace(&mut chars);

    match chars.next() {
        Some('-') | Some('*') => {}
        _ => return None,
    }

    skip_whitespace(&mut chars);

    if chars.next() != Some('[') {
        return None;
    }

    let box_char = chars.next()?;
    if !(box_char.is_whitespace() || box_char == 'x' || box_char == 'X') {
        return None;
    }

    if chars.next() != Some(']') {
        return None;
    }

    Some(box_char == 'x' || box_char == 'X')
}

/// Advance `chars` past any run of whitespace characters at its front.
fn skip_whitespace(chars: &mut std::str::Chars<'_>) {
    let mut lookahead = chars.clone();
    while let Some(c) = lookahead.next() {
        if c.is_whitespace() {
            *chars = lookahead.clone();
        } else {
            break;
        }
    }
}

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
