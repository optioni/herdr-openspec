//! The detail region's grammar: plain data, with no I/O API and no
//! dependency on how the view styles it — the same shape `ui::list` uses
//! for the row grammar and `ui::markdown` for the document. Every public
//! function here takes an interior width, which is what makes
//! `DETAILWIDTHS` possible with no exemption list. See
//! `openspec/changes/detail-view/design.md` -> Boundaries and Contracts.

/// The change header's fixed-field grammar:
/// `[name field][space][schema cell][space][progress cell]`, exactly
/// `width` characters. The progress cell is `ui::list::progress_cell` — one
/// implementation, so the header and a list row can never disagree about a
/// change's progress. A cell too wide for the row is dropped **whole**: the
/// schema cell first, then the progress cell, leaving the name field alone;
/// the name is truncated only after both cells have been dropped.
pub fn header_row(
    name: &str,
    schema: &str,
    progress: &crate::tasks::Progress,
    width: u16,
) -> String {
    if width == 0 {
        return String::new();
    }
    let w = i64::from(width);
    let progress_cell = crate::ui::list::progress_cell(progress);
    let progress_len = progress_cell.chars().count() as i64;
    let schema_cell = format!("({schema})");
    let schema_len = schema_cell.chars().count() as i64;

    // Full form: name + space + schema cell + space + progress cell.
    let name_field_full = w - 2 - schema_len - progress_len;
    if name_field_full >= 1 {
        let name_field = crate::ui::list::pad_or_truncate_right(name, name_field_full as usize);
        return format!("{name_field} {schema_cell} {progress_cell}");
    }

    // Drop the schema cell and its separating space: name + space + progress.
    let name_field_no_schema = w - 1 - progress_len;
    if name_field_no_schema >= 1 {
        let name_field =
            crate::ui::list::pad_or_truncate_right(name, name_field_no_schema as usize);
        return format!("{name_field} {progress_cell}");
    }

    // Drop the progress cell too: the name field alone, the whole width.
    crate::ui::list::pad_or_truncate_right(name, w as usize)
}

#[cfg(test)]
mod tests {
    use super::header_row;
    use crate::tasks::Progress;

    #[test]
    fn the_full_header_grammar_at_both_mandated_interior_widths() {
        let progress = Progress {
            completed: 4,
            total: 42,
        };
        for (width, name_field_width) in [(78, 65usize), (58, 45)] {
            let got = header_row("detail-view", "tdd", &progress, width);
            assert_eq!(got.chars().count(), width as usize, "width {width}");
            let expected_name_field =
                format!("{:<width$}", "detail-view", width = name_field_width);
            let expected = format!("{expected_name_field} (tdd) [4/42]");
            assert_eq!(got, expected, "width {width}");
            assert!(got.ends_with("[4/42]"), "width {width}");
        }
    }

    #[test]
    fn a_change_with_no_tasks_still_ends_its_row_in_the_same_column() {
        let progress = Progress {
            completed: 0,
            total: 0,
        };
        for width in [78, 58] {
            let got = header_row("migrate-ai-sdk-v7", "tdd", &progress, width);
            assert_eq!(got.chars().count(), width as usize, "width {width}");
            assert!(got.ends_with("[-]"), "width {width}: {got:?}");
        }
    }

    #[test]
    fn a_long_name_is_truncated_with_an_ellipsis_never_overflowing_the_row() {
        let name = "a".repeat(200);
        let progress = Progress {
            completed: 4,
            total: 42,
        };
        for width in [78, 58] {
            let got = header_row(&name, "tdd", &progress, width);
            assert_eq!(got.chars().count(), width as usize, "width {width}");
            assert!(got.contains("(tdd)"), "width {width}: {got:?}");
            assert!(got.ends_with("[4/42]"), "width {width}: {got:?}");
            let name_field_end = got.find(" (tdd)").expect("schema cell present");
            let name_field = &got[..name_field_end];
            assert!(
                name_field.ends_with('…'),
                "width {width}: name field does not end in an ellipsis: {name_field:?}"
            );
        }
    }

    #[test]
    fn the_cells_are_dropped_whole_in_order_as_the_row_narrows() {
        let progress = Progress {
            completed: 4,
            total: 9,
        };
        for w in [78, 58, 13, 12, 7, 6, 5, 1, 0] {
            let got = header_row("add-token-refresh", "tdd", &progress, w);
            assert_eq!(got.chars().count(), w as usize, "width {w}");
            if w >= 13 {
                assert!(got.contains("(tdd)"), "width {w}: {got:?}");
                assert!(got.contains("[4/9]"), "width {w}: {got:?}");
            } else if (7..=12).contains(&w) {
                assert!(!got.contains("(tdd"), "width {w}: {got:?}");
                assert!(got.contains("[4/9]"), "width {w}: {got:?}");
            } else if w > 0 {
                assert!(!got.contains("(tdd"), "width {w}: {got:?}");
                assert!(!got.contains("[4/"), "width {w}: {got:?}");
            } else {
                assert_eq!(got, "", "width {w}");
            }
            // No cell was ever cut short rather than dropped.
            if got.contains("(tdd") {
                assert!(got.contains("(tdd)"), "width {w}: partial schema cell");
            }
            if got.contains("[4/") {
                assert!(got.contains("[4/9]"), "width {w}: partial progress cell");
            }
        }
    }

    #[test]
    fn an_empty_schema_name_is_a_cell_of_two_characters_not_an_absent_one() {
        let progress = Progress {
            completed: 1,
            total: 2,
        };
        for width in [78, 58] {
            let got = header_row("alpha", "", &progress, width);
            assert_eq!(got.chars().count(), width as usize, "width {width}");
            assert!(got.contains("() [1/2]"), "width {width}: {got:?}");
        }
    }
}
