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

/// One drawn tab cell: its label, its column offset from the interior's
/// first column, its position in the artifact list (`None` for the
/// zero-artifact placeholder), and whether it is the selected tab. Carries
/// no styling: the view is what emphasises the selected cell, exactly as
/// it does for `ui::list::Row`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    pub text: String,
    pub x: u16,
    pub index: Option<usize>,
    pub selected: bool,
}

/// `text`, unpadded, when it already fits in `width`; truncated to `width`
/// characters with a trailing `…` — by the same `ui::list::pad_or_truncate_right`
/// the header and the row grammar use — when it does not. Unlike
/// `pad_or_truncate_right` itself, this never pads: a tab cell's width is
/// its own label's length, not the whole interior's.
fn cell_text(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        text.to_string()
    } else {
        crate::ui::list::pad_or_truncate_right(text, width)
    }
}

/// The joined width of cells `start..=end`: their own widths plus two
/// separating columns between every pair.
fn joined_width(cell_lens: &[usize], start: usize, end: usize) -> usize {
    let cells_width: usize = cell_lens[start..=end].iter().sum();
    let seps = (end - start) * 2;
    cells_width + seps
}

/// The tab bar: `Change::artifacts` in the schema's declared order,
/// addressed by **position**, windowed to a contiguous run of whole cells
/// that always contains the selected tab. `width == 0` returns an empty
/// vector before every other branch; an empty `artifacts` returns a single
/// `no artifacts` placeholder cell; a `selected` past the end of the list
/// is treated as `0` for windowing purposes and marks no cell selected. See
/// `openspec/changes/detail-view/design.md` -> Contracts and
/// `specs/artifact-tabs/spec.md`.
pub fn tab_bar(artifacts: &[crate::changes::ArtifactRef], selected: usize, width: u16) -> Vec<Tab> {
    if width == 0 {
        return Vec::new();
    }
    let w = width as usize;

    if artifacts.is_empty() {
        return vec![Tab {
            text: cell_text("no artifacts", w),
            x: 0,
            index: None,
            selected: false,
        }];
    }

    let n = artifacts.len();
    let cells: Vec<String> = artifacts
        .iter()
        .enumerate()
        .map(|(i, a)| {
            if i < 9 {
                format!("{} {}", i + 1, a.id)
            } else {
                a.id.clone()
            }
        })
        .collect();
    let cell_lens: Vec<usize> = cells.iter().map(|c| c.chars().count()).collect();

    let selected_valid = selected < n;
    let anchor = if selected_valid { selected } else { 0 };

    // The one exception: the anchor cell alone is wider than the bar, so no
    // whole-cell window exists at all.
    if cell_lens[anchor] > w {
        return vec![Tab {
            text: cell_text(&cells[anchor], w),
            x: 0,
            index: Some(anchor),
            selected: selected_valid,
        }];
    }

    // `start`: the smallest index not greater than `anchor` for which
    // `start..=anchor` fits. `end`: the largest index not less than
    // `anchor` for which `start..=end` fits, given that fixed `start`. When
    // every cell fits, this naturally degenerates to `start == 0` and
    // `end == n - 1` — no separate "everything fits" branch is needed.
    let mut start = anchor;
    while start > 0 && joined_width(&cell_lens, start - 1, anchor) <= w {
        start -= 1;
    }
    let mut end = anchor;
    while end + 1 < n && joined_width(&cell_lens, start, end + 1) <= w {
        end += 1;
    }

    let mut out = Vec::new();
    let mut x: u16 = 0;
    for i in start..=end {
        out.push(Tab {
            text: cells[i].clone(),
            x,
            index: Some(i),
            selected: selected_valid && i == selected,
        });
        x += cell_lens[i] as u16 + 2;
    }
    out
}

/// The single line list both `ui::view::render` and `Dashboard::normalise_scroll`
/// derive the detail content from, so the drawn slice and the scroll clamp
/// can never disagree about how many lines there are: one `"! <problem>"`
/// line per entry of the selected change's own `problems` (`degraded-states`'
/// addition — `SPEC.md` rows naming a reason on `Change::problems` that
/// nothing rendered before this change), then one such line per
/// `detail.problems` entry, then the selected tab's **body**, and — only
/// when all three are empty — exactly one line reading `No content yet`.
/// When either problem source is non-empty and `source` is empty, the
/// problem lines alone are returned: the reason is known, and adding `No
/// content yet` would say two contradictory things about the same tab.
///
/// The body is `ui::tasks::lines(&detail.source, &change.progress, width)`
/// — `tasks-checklist`'s grammar and `tasks-progress-bar`'s leading line —
/// when `change` is `Some` and the `ArtifactRef` at `detail.tab` carries
/// `tracks_tasks == true`, and `ui::markdown::lines(&detail.source, width)`
/// in every other case, including a `None` change, a `detail.tab` past the
/// end of the artifact list, and a change carrying no artifacts at all.
/// The decision is made exactly here, once, so `ui::view::render` and
/// `Dashboard::normalise_scroll` — both of which pass
/// `Dashboard::selected_change()` — can never disagree about which grammar
/// the tab holds.
pub fn content_lines(
    detail: &crate::ui::app::Detail,
    change: Option<&crate::changes::Change>,
    width: u16,
) -> Vec<crate::ui::markdown::Line> {
    fn problem_line(p: &str, width: u16) -> crate::ui::markdown::Line {
        crate::ui::markdown::Line {
            segments: vec![crate::ui::markdown::Segment {
                text: crate::ui::list::pad_or_truncate_right(&format!("! {p}"), width as usize),
                face: crate::ui::markdown::Face::plain(),
            }],
        }
    }

    // `degraded-states`: the selected change's own problems lead, above `detail.problems` —
    // "change_problem_precedes_tab_problem" — using the SAME `pad_or_truncate_right` call and
    // plain face `detail.problems` already renders with, on the existing problem-row
    // mechanism rather than a new one.
    let mut out: Vec<crate::ui::markdown::Line> = change
        .map(|c| c.problems.as_slice())
        .unwrap_or(&[])
        .iter()
        .map(|p| problem_line(p, width))
        .collect();
    out.extend(detail.problems.iter().map(|p| problem_line(p, width)));

    let tracked_tasks_progress = change.and_then(|c| {
        c.artifacts
            .get(detail.tab)
            .filter(|a| a.tracks_tasks)
            .map(|_| &c.progress)
    });
    let body = match tracked_tasks_progress {
        Some(progress) => crate::ui::tasks::lines(&detail.source, progress, width),
        None => crate::ui::markdown::lines(&detail.source, width),
    };
    out.extend(body);

    if out.is_empty() {
        out.push(crate::ui::markdown::Line {
            segments: vec![crate::ui::markdown::Segment {
                text: "No content yet".to_string(),
                face: crate::ui::markdown::Face::plain(),
            }],
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Tab, content_lines, header_row, tab_bar};
    use crate::changes::fixture;
    use crate::tasks::Progress;
    use crate::ui::app::Detail;

    fn detail(source: &str, problems: Vec<String>) -> Detail {
        Detail {
            source: source.to_string(),
            scroll: 0,
            tab: 0,
            problems,
            loaded: None,
        }
    }

    /// Artifacts through `changes::fixture::with_artifacts`, never an
    /// `ArtifactRef {}` literal of this module's own — `changes::fixture`
    /// stays the one place a `Change`'s (and its artifacts') shape is
    /// spelled out.
    fn artifacts(ids: &[&str]) -> Vec<crate::changes::ArtifactRef> {
        let pairs: Vec<(&str, &[&str])> = ids.iter().map(|id| (*id, &[][..])).collect();
        fixture::with_artifacts(fixture::active("x", 0, 0), &pairs).artifacts
    }

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

    #[test]
    fn the_five_tdd_artifacts_become_five_numbered_tabs_at_both_mandated_widths() {
        let a = artifacts(&["proposal", "specs", "design", "tasks", "planning-review"]);
        for width in [78, 58] {
            let tabs = tab_bar(&a, 0, width);
            assert_eq!(tabs.len(), 5, "width {width}");
            let texts: Vec<&str> = tabs.iter().map(|t| t.text.as_str()).collect();
            assert_eq!(
                texts,
                vec![
                    "1 proposal",
                    "2 specs",
                    "3 design",
                    "4 tasks",
                    "5 planning-review"
                ],
                "width {width}"
            );
            let xs: Vec<u16> = tabs.iter().map(|t| t.x).collect();
            assert_eq!(xs, vec![0, 12, 21, 31, 40], "width {width}");
            let last = tabs.last().unwrap();
            let last_end = last.x as usize + last.text.chars().count() - 1;
            assert_eq!(last_end, 56, "width {width}");
            let indices: Vec<Option<usize>> = tabs.iter().map(|t| t.index).collect();
            assert_eq!(
                indices,
                vec![Some(0), Some(1), Some(2), Some(3), Some(4)],
                "width {width}"
            );
            assert!(tabs[0].selected, "width {width}");
            assert!(tabs[1..].iter().all(|t| !t.selected), "width {width}");
        }
    }

    #[test]
    fn duplicate_artifact_ids_remain_two_separately_addressable_tabs() {
        let a = artifacts(&["spec", "spec", "notes"]);
        for width in [78, 58] {
            let tabs = tab_bar(&a, 1, width);
            let texts: Vec<&str> = tabs.iter().map(|t| t.text.as_str()).collect();
            assert_eq!(texts, vec!["1 spec", "2 spec", "3 notes"], "width {width}");
            assert_eq!(tabs[1].index, Some(1), "width {width}");
            assert!(tabs[1].selected, "width {width}");
            assert_eq!(tabs[0].index, Some(0), "width {width}");
            assert!(!tabs[0].selected, "width {width}");
        }
    }

    #[test]
    fn a_tenth_artifact_is_labelled_without_a_digit() {
        let ids: Vec<String> = (1..=12).map(|i| format!("a{i:02}")).collect();
        let ids_ref: Vec<&str> = ids.iter().map(String::as_str).collect();
        let a = artifacts(&ids_ref);
        for width in [78, 58] {
            let tabs = tab_bar(&a, 0, width);
            for (i, tab) in tabs.iter().enumerate() {
                if i < 9 {
                    assert_eq!(
                        tab.text,
                        format!("{} a{:02}", i + 1, i + 1),
                        "width {width}"
                    );
                } else {
                    assert_eq!(tab.text, format!("a{:02}", i + 1), "width {width}");
                }
            }
        }
    }

    #[test]
    fn no_artifacts_is_a_single_placeholder_cell_not_an_empty_bar() {
        let a = artifacts(&[]);
        for width in [78, 58] {
            let tabs = tab_bar(&a, 0, width);
            assert_eq!(tabs.len(), 1, "width {width}");
            assert_eq!(tabs[0].text, "no artifacts", "width {width}");
            assert_eq!(tabs[0].x, 0, "width {width}");
            assert_eq!(tabs[0].index, None, "width {width}");
            assert!(!tabs[0].selected, "width {width}");
        }
        let narrow = tab_bar(&artifacts(&[]), 0, 8);
        assert_eq!(narrow.len(), 1);
        assert_eq!(narrow[0].text.chars().count(), 8);
        assert!(narrow[0].text.ends_with('…'));
    }

    #[test]
    fn a_zero_width_bar_is_empty_and_does_not_panic() {
        let with_five = artifacts(&["a", "b", "c", "d", "e"]);
        assert_eq!(tab_bar(&with_five, 0, 0), Vec::new());
        assert_eq!(tab_bar(&artifacts(&[]), 0, 0), Vec::new());
        // Discriminating companion, naming both mandated widths: the same
        // five-artifact list is *not* empty at either real width, so the
        // width-0 case above is a property of the width, not a constant.
        for width in [78, 58] {
            assert!(!tab_bar(&with_five, 0, width).is_empty(), "width {width}");
        }
    }

    #[test]
    fn a_twelve_artifact_bar_windows_to_keep_the_selected_tab_visible() {
        let ids: Vec<String> = (1..=12).map(|i| format!("artifact-{i:02}")).collect();
        let ids_ref: Vec<&str> = ids.iter().map(String::as_str).collect();
        let a = artifacts(&ids_ref);
        let mut window_sizes_by_width: Vec<(u16, Vec<usize>)> = Vec::new();
        for width in [78, 58] {
            let mut window_sizes = Vec::new();
            for selected in [0usize, 3, 11] {
                let tabs = tab_bar(&a, selected, width);
                assert!(!tabs.is_empty(), "width {width} selected {selected}");
                let indices: Vec<usize> = tabs.iter().map(|t| t.index.unwrap()).collect();
                for w in indices.windows(2) {
                    assert_eq!(
                        w[1],
                        w[0] + 1,
                        "width {width} selected {selected}: not contiguous"
                    );
                }
                assert_eq!(tabs[0].x, 0, "width {width} selected {selected}");
                let last = tabs.last().unwrap();
                let last_end = last.x as usize + last.text.chars().count();
                assert!(
                    last_end <= width as usize,
                    "width {width} selected {selected}: last cell's final column {last_end} \
                     runs past width"
                );
                let selected_tabs: Vec<&Tab> = tabs.iter().filter(|t| t.selected).collect();
                assert_eq!(selected_tabs.len(), 1, "width {width} selected {selected}");
                assert_eq!(
                    selected_tabs[0].index,
                    Some(selected),
                    "width {width} selected {selected}"
                );
                assert!(
                    tabs.iter().all(|t| !t.text.ends_with('…')),
                    "width {width} selected {selected}: a shown cell was truncated"
                );
                if selected == 0 {
                    assert_eq!(indices[0], 0, "width {width}");
                }
                if selected == 11 {
                    assert_eq!(*indices.last().unwrap(), 11, "width {width}");
                }
                window_sizes.push(tabs.len());
            }
            window_sizes_by_width.push((width, window_sizes));
        }
        // The 78-column window holds strictly more cells than the
        // 58-column one, for the same selection — the width is genuinely
        // load-bearing.
        let wide = &window_sizes_by_width[0].1;
        let narrow = &window_sizes_by_width[1].1;
        for (wide_n, narrow_n) in wide.iter().zip(narrow.iter()) {
            assert!(
                wide_n > narrow_n,
                "78-column window ({wide_n}) must hold strictly more cells than \
                 58-column ({narrow_n})"
            );
        }
    }

    #[test]
    fn the_window_slides_back_when_the_selection_moves_left_again() {
        let ids: Vec<String> = (1..=12).map(|i| format!("artifact-{i:02}")).collect();
        let ids_ref: Vec<&str> = ids.iter().map(String::as_str).collect();
        let a = artifacts(&ids_ref);
        for width in [78, 58] {
            let _ = tab_bar(&a, 11, width);
            let tabs = tab_bar(&a, 0, width);
            assert_eq!(tabs[0].index, Some(0), "width {width}");
        }
    }

    #[test]
    fn a_selected_cell_wider_than_the_whole_bar_is_truncated_rather_than_dropped() {
        let id = "x".repeat(200);
        let a = artifacts(&[&id]);
        for width in [78, 58] {
            let tabs = tab_bar(&a, 0, width);
            assert_eq!(tabs.len(), 1, "width {width}");
            assert_eq!(tabs[0].x, 0, "width {width}");
            assert!(tabs[0].selected, "width {width}");
            assert_eq!(tabs[0].index, Some(0), "width {width}");
            assert_eq!(
                tabs[0].text.chars().count(),
                width as usize,
                "width {width}"
            );
            assert!(tabs[0].text.ends_with('…'), "width {width}");
        }
    }

    #[test]
    fn a_selected_index_past_the_end_of_the_list_does_not_panic() {
        let a = artifacts(&["a", "b", "c"]);
        for width in [78, 58] {
            let tabs = tab_bar(&a, 7, width);
            assert!(tabs.iter().all(|t| t.index.unwrap() < 3), "width {width}");
            assert!(!tabs.iter().any(|t| t.selected), "width {width}");
            assert_eq!(tabs.len(), 3, "width {width}");
            assert_eq!(tabs[0].index, Some(0), "width {width}");
        }
    }

    #[test]
    fn an_empty_detail_returns_exactly_one_no_content_yet_line() {
        for width in [78, 58] {
            let lines = content_lines(&detail("", Vec::new()), None, width);
            assert_eq!(lines.len(), 1, "width {width}");
            assert_eq!(lines[0].text(), "No content yet", "width {width}");
        }
    }

    #[test]
    fn problems_only_are_returned_with_no_no_content_yet_line() {
        for width in [78, 58] {
            let d = detail("", vec!["/repo/a.md: boom".to_string()]);
            let lines = content_lines(&d, None, width);
            assert_eq!(lines.len(), 1, "width {width}");
            assert!(
                lines[0].text().starts_with("! /repo/a.md: boom"),
                "width {width}: {:?}",
                lines[0].text()
            );
            assert!(
                !lines.iter().any(|l| l.text().contains("No content yet")),
                "width {width}"
            );
        }
    }

    #[test]
    fn a_source_only_renders_as_markdown_with_no_problem_line() {
        for width in [78, 58] {
            let d = detail("# heading\n", Vec::new());
            let lines = content_lines(&d, None, width);
            assert!(!lines.is_empty(), "width {width}");
            assert!(
                !lines.iter().any(|l| l.text().starts_with('!')),
                "width {width}"
            );
            assert!(
                !lines.iter().any(|l| l.text() == "No content yet"),
                "width {width}"
            );
        }
    }

    #[test]
    fn both_problems_and_source_are_returned_with_problems_first() {
        for width in [78, 58] {
            let d = detail("# heading\n", vec!["/repo/a.md: boom".to_string()]);
            let lines = content_lines(&d, None, width);
            assert!(
                lines[0].text().starts_with("! /repo/a.md: boom"),
                "width {width}: {:?}",
                lines[0].text()
            );
            assert!(
                lines[1..].iter().any(|l| l.text().contains("heading")),
                "width {width}"
            );
            assert!(
                !lines.iter().any(|l| l.text() == "No content yet"),
                "width {width}"
            );
        }
    }

    // --- degraded-states: the selected change's own problems (task group 5) --------------

    /// `artifact-content` :: "A change whose schema will not parse names the reason in the
    /// detail region" — the change's own problems render above the tab content, using the
    /// same `! `-prefixed, plain-face, `pad_or_truncate_right` grammar `detail.problems`
    /// already uses.
    #[test]
    fn change_problems_render_above_the_content() {
        for width in [78, 58] {
            let change = fixture::with_problems(
                fixture::active("x", 0, 0),
                vec!["schema is not vendored".to_string()],
            );
            let d = detail("# heading\n", Vec::new());
            let lines = content_lines(&d, Some(&change), width);
            assert!(
                lines[0].text().starts_with("! schema is not vendored"),
                "width {width}: {:?}",
                lines[0].text()
            );
            assert!(
                lines[1..].iter().any(|l| l.text().contains("heading")),
                "width {width}"
            );
        }
    }

    /// `artifact-content` :: "A change problem and a tab problem are both shown, change
    /// first".
    #[test]
    fn change_problem_precedes_tab_problem() {
        for width in [78, 58] {
            let change = fixture::with_problems(
                fixture::active("x", 0, 0),
                vec!["schema is not vendored".to_string()],
            );
            let d = detail("", vec!["/repo/a.md: boom".to_string()]);
            let lines = content_lines(&d, Some(&change), width);
            assert_eq!(lines.len(), 2, "width {width}: {:?}", lines_text(&lines));
            assert!(
                lines[0].text().starts_with("! schema is not vendored"),
                "width {width}: {:?}",
                lines[0].text()
            );
            assert!(
                lines[1].text().starts_with("! /repo/a.md: boom"),
                "width {width}: {:?}",
                lines[1].text()
            );
        }
    }

    /// `artifact-content` :: "No selected change contributes no lines" — a `None` change
    /// must render identically to a change carrying an empty `problems` vector, both at the
    /// unit level and against a blank content area.
    #[test]
    fn no_selected_change_contributes_no_lines() {
        for width in [78, 58] {
            let d = detail("# heading\n", Vec::new());
            let with_none = content_lines(&d, None, width);
            let empty_change = fixture::active("x", 0, 0);
            let with_empty = content_lines(&d, Some(&empty_change), width);
            assert_eq!(with_none, with_empty, "width {width}");
            assert!(
                !with_none.iter().any(|l| l.text().starts_with('!')),
                "width {width}"
            );
        }
    }

    /// Renders `lines` as plain text, for an assertion failure message only.
    fn lines_text(lines: &[crate::ui::markdown::Line]) -> Vec<String> {
        lines.iter().map(|l| l.text().to_string()).collect()
    }

    // --- degraded-states: group 7 proofs — the not-vendored/unparseable rows -------------

    /// `degraded-coverage` :: "A schema that is not vendored and one that will not parse
    /// both empty the tab bar" — SPEC.md rows 4 and 5, driven through the REAL
    /// `changes::from_files` against two real scratch repositories, rather than a
    /// hand-built fixture that already assumes an empty artifact list.
    #[test]
    fn an_unusable_schema_renders_no_artifacts() {
        // Sub-case 1: not vendored at all.
        let not_vendored = crate::testutil::ScratchDir::new();
        let root = not_vendored.path();
        crate::testutil::write_with_mode(
            &root.join("openspec/changes/x/.openspec.yaml"),
            b"schema: nowhere\n",
            0o644,
        );
        crate::testutil::write_with_mode(
            &root.join("openspec/changes/x/proposal.md"),
            b"# x\n",
            0o644,
        );
        let files = crate::changes::from_files(root, 5);
        let change = files
            .active
            .iter()
            .find(|c| c.name == "x")
            .expect("change x is read");
        assert!(change.artifacts.is_empty());
        assert!(!change.problems.is_empty());

        // Sub-case 2: vendored but unparseable (not a YAML mapping at all).
        let unparseable = crate::testutil::ScratchDir::new();
        let root2 = unparseable.path();
        crate::testutil::write_with_mode(
            &root2.join("openspec/changes/y/.openspec.yaml"),
            b"schema: broken\n",
            0o644,
        );
        crate::testutil::write_with_mode(
            &root2.join("openspec/schemas/broken/schema.yaml"),
            b"just a bare scalar, not a mapping\n",
            0o644,
        );
        crate::testutil::write_with_mode(
            &root2.join("openspec/changes/y/proposal.md"),
            b"# y\n",
            0o644,
        );
        let files2 = crate::changes::from_files(root2, 5);
        let change2 = files2
            .active
            .iter()
            .find(|c| c.name == "y")
            .expect("change y is read");
        assert!(change2.artifacts.is_empty());
        assert!(!change2.problems.is_empty());

        for width in [78, 58] {
            for change in [change, change2] {
                let tabs = tab_bar(&change.artifacts, 0, width as u16);
                assert_eq!(tabs.len(), 1, "width {width}: {tabs:?}");
                assert_eq!(tabs[0].text, "no artifacts", "width {width}");
                assert_eq!(tabs[0].index, None, "width {width}");
            }
        }
    }

    /// `degraded-coverage` :: "A schema with no tasks artifact renders every tab as
    /// markdown and still counts" — SPEC.md row 6. A real schema with three artifacts, no
    /// `apply.tracks` and no artifact whose id is `tasks`, so `tracks_tasks` is `false`
    /// everywhere; every tab must dispatch to the markdown grammar, never the checklist one,
    /// and the change's own progress must still fall back to counting `tasks.md` directly.
    #[test]
    fn no_tasks_artifact_renders_every_tab_as_markdown() {
        let scratch = crate::testutil::ScratchDir::new();
        let root = scratch.path();
        crate::testutil::write_with_mode(
            &root.join("openspec/changes/x/.openspec.yaml"),
            b"schema: notrack\n",
            0o644,
        );
        crate::testutil::write_with_mode(
            &root.join("openspec/schemas/notrack/schema.yaml"),
            b"name: notrack\nartifacts:\n  - id: proposal\n    generates: proposal.md\n  - id: specs\n    generates: specs.md\n  - id: design\n    generates: design.md\n",
            0o644,
        );
        crate::testutil::write_with_mode(
            &root.join("openspec/changes/x/proposal.md"),
            b"# proposal\n",
            0o644,
        );
        crate::testutil::write_with_mode(
            &root.join("openspec/changes/x/specs.md"),
            b"# specs\n",
            0o644,
        );
        crate::testutil::write_with_mode(
            &root.join("openspec/changes/x/design.md"),
            b"# design\n",
            0o644,
        );
        crate::testutil::write_with_mode(
            &root.join("openspec/changes/x/tasks.md"),
            b"- [x] a\n- [ ] b\n- [ ] c\n",
            0o644,
        );

        let files = crate::changes::from_files(root, 5);
        let change = files
            .active
            .iter()
            .find(|c| c.name == "x")
            .expect("change x is read");
        assert_eq!(change.artifacts.len(), 3);
        assert!(
            change.artifacts.iter().all(|a| !a.tracks_tasks),
            "no artifact may be marked: {:?}",
            change.artifacts
        );
        // The task count is not deferred: it falls back to counting tasks.md directly.
        assert_eq!(
            change.progress,
            Progress {
                completed: 1,
                total: 3
            }
        );

        for width in [78, 58] {
            for (tab, source_heading) in [(0, "proposal"), (1, "specs"), (2, "design")] {
                let d = Detail {
                    source: crate::ui::read_artifact(&change.artifacts[tab].paths[0])
                        .expect("read the artifact's own file"),
                    scroll: 0,
                    tab,
                    problems: Vec::new(),
                    loaded: None,
                };
                let lines = content_lines(&d, Some(change), width);
                // Markdown, not the checklist grammar: a `# heading` renders as one line
                // carrying the heading text, never a `[ ]`/`[x]` glyph or a progress bar.
                assert!(
                    lines.iter().any(|l| l.text().contains(source_heading)),
                    "width {width}, tab {tab}: {:?}",
                    lines_text(&lines)
                );
                assert!(
                    !lines.iter().any(|l| l.text().contains('[')),
                    "width {width}, tab {tab}: rendered the checklist grammar instead of \
                     markdown: {:?}",
                    lines_text(&lines)
                );
            }
        }
    }

    /// `degraded-coverage` :: "An unsupported `generates` glob empties one artifact and
    /// names why" — SPEC.md row 16. A real schema names one artifact with an unsupported
    /// glob pattern (a metacharacter inside a directory segment that is not the trailing
    /// `**`) alongside one ordinary artifact, over a real scratch repository — the bad
    /// artifact's own list is empty and the reason names its id; the good one is untouched.
    #[test]
    fn an_unsupported_glob_names_its_reason_and_spares_the_others() {
        let scratch = crate::testutil::ScratchDir::new();
        let root = scratch.path();
        crate::testutil::write_with_mode(
            &root.join("openspec/changes/x/.openspec.yaml"),
            b"schema: globby\n",
            0o644,
        );
        crate::testutil::write_with_mode(
            &root.join("openspec/schemas/globby/schema.yaml"),
            b"name: globby\nartifacts:\n  - id: proposal\n    generates: proposal.md\n  - id: bad\n    generates: a*/x.md\n  - id: tasks\n    generates: tasks.md\n",
            0o644,
        );
        crate::testutil::write_with_mode(
            &root.join("openspec/changes/x/proposal.md"),
            b"# proposal\n",
            0o644,
        );
        crate::testutil::write_with_mode(
            &root.join("openspec/changes/x/tasks.md"),
            b"- [x] a\n",
            0o644,
        );

        let files = crate::changes::from_files(root, 5);
        let change = files
            .active
            .iter()
            .find(|c| c.name == "x")
            .expect("change x is read");

        let good = change
            .artifacts
            .iter()
            .find(|a| a.id == "proposal")
            .expect("the good artifact still resolves");
        assert_eq!(good.paths.len(), 1);

        let bad = change
            .artifacts
            .iter()
            .find(|a| a.id == "bad")
            .expect("the bad artifact is still listed, just empty");
        assert!(bad.paths.is_empty());

        assert_eq!(change.problems.len(), 1, "{:?}", change.problems);
        assert!(change.problems[0].contains("bad"), "{:?}", change.problems);
        assert!(
            !change.problems[0].contains("proposal"),
            "the reason must name only the affected artifact: {:?}",
            change.problems
        );

        for width in [78, 58] {
            let d = Detail {
                source: String::new(),
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
            };
            let lines = content_lines(&d, Some(change), width);
            assert!(
                lines[0].text().starts_with('!') && lines[0].text().contains("bad"),
                "width {width}: {:?}",
                lines_text(&lines)
            );
        }
    }

    /// `degraded-coverage` :: "A duplicate artifact id is accepted by this crate and stays
    /// file-mode" — row 32, driven from real YAML bytes through `changes::from_files` (and
    /// so through `schema::parse`) rather than from a hand-built `Schema` or `ArtifactRef`
    /// list, over a real scratch repository.
    #[test]
    fn a_duplicate_artifact_id_parses_and_renders() {
        let scratch = crate::testutil::ScratchDir::new();
        let root = scratch.path();
        crate::testutil::write_with_mode(
            &root.join("openspec/changes/x/.openspec.yaml"),
            b"schema: dupe\n",
            0o644,
        );
        crate::testutil::write_with_mode(
            &root.join("openspec/schemas/dupe/schema.yaml"),
            b"name: dupe\nartifacts:\n  - id: spec\n    generates: a.md\n  - id: spec\n    generates: b.md\n  - id: tasks\n    generates: tasks.md\n",
            0o644,
        );
        crate::testutil::write_with_mode(&root.join("openspec/changes/x/a.md"), b"# a\n", 0o644);
        crate::testutil::write_with_mode(&root.join("openspec/changes/x/b.md"), b"# b\n", 0o644);
        crate::testutil::write_with_mode(
            &root.join("openspec/changes/x/tasks.md"),
            b"- [x] a\n",
            0o644,
        );

        let files = crate::changes::from_files(root, 5);
        let change = files
            .active
            .iter()
            .find(|c| c.name == "x")
            .expect("change x is read");

        // The CLI would reject this schema outright ("Duplicate artifact ID"); this crate's
        // own parser accepts it, keeping both positions rather than de-duplicating —
        // exactly the row's "usable is strictly wider than the CLI's" claim.
        let spec_positions: Vec<usize> = change
            .artifacts
            .iter()
            .enumerate()
            .filter(|(_, a)| a.id == "spec")
            .map(|(i, _)| i)
            .collect();
        assert_eq!(spec_positions, vec![0, 1], "{:?}", change.artifacts);

        for width in [78, 58] {
            let tabs = tab_bar(&change.artifacts, 1, width);
            let texts: Vec<&str> = tabs.iter().map(|t| t.text.as_str()).collect();
            assert!(
                texts.iter().filter(|t| t.ends_with("spec")).count() == 2,
                "width {width}: {texts:?}"
            );
            assert_eq!(tabs[1].index, Some(1), "width {width}");
            assert!(tabs[1].selected, "width {width}");
        }
    }

    /// `degraded-coverage` :: "A tasks file that cannot be read is zero tasks with a named
    /// reason" — SPEC.md row 17, both fixtures: a directory where a file was expected, and
    /// invalid UTF-8. Real scratch repositories, real `changes::from_files`.
    #[test]
    fn an_unreadable_tasks_file_is_zero_with_a_named_reason() {
        // Fixture 1: tasks.md is a directory.
        let scratch = crate::testutil::ScratchDir::new();
        let root = scratch.path();
        crate::testutil::write_with_mode(
            &root.join("openspec/changes/x/.openspec.yaml"),
            b"schema: tdd\n",
            0o644,
        );
        crate::testutil::write_with_mode(
            &root.join("openspec/schemas/tdd/schema.yaml"),
            b"name: tdd\nartifacts:\n  - id: proposal\n    generates: proposal.md\n  - id: tasks\n    generates: tasks.md\n",
            0o644,
        );
        crate::testutil::write_with_mode(
            &root.join("openspec/changes/x/proposal.md"),
            b"# x\n",
            0o644,
        );
        // Writing a file *inside* `tasks.md` makes `tasks.md` itself a directory, without
        // this file naming an I/O API directly — `NOIO-VIEW` forbids that under
        // `src/ui/detail.rs`, tests included.
        crate::testutil::write_with_mode(
            &root.join("openspec/changes/x/tasks.md/marker"),
            b"x",
            0o644,
        );

        let files = crate::changes::from_files(root, 5);
        let change = files
            .active
            .iter()
            .find(|c| c.name == "x")
            .expect("change x is read");
        assert_eq!(
            change.progress,
            Progress {
                completed: 0,
                total: 0
            }
        );
        assert!(
            change.problems.iter().any(|p| p.contains("tasks.md")),
            "{:?}",
            change.problems
        );

        // Fixture 2: tasks.md holds invalid UTF-8.
        let scratch2 = crate::testutil::ScratchDir::new();
        let root2 = scratch2.path();
        crate::testutil::write_with_mode(
            &root2.join("openspec/changes/y/.openspec.yaml"),
            b"schema: tdd\n",
            0o644,
        );
        crate::testutil::write_with_mode(
            &root2.join("openspec/schemas/tdd/schema.yaml"),
            b"name: tdd\nartifacts:\n  - id: proposal\n    generates: proposal.md\n  - id: tasks\n    generates: tasks.md\n",
            0o644,
        );
        crate::testutil::write_with_mode(
            &root2.join("openspec/changes/y/proposal.md"),
            b"# y\n",
            0o644,
        );
        crate::testutil::write_with_mode(
            &root2.join("openspec/changes/y/tasks.md"),
            &[0x2d, 0x20, 0xff, 0xfe, b'\n'],
            0o644,
        );

        let files2 = crate::changes::from_files(root2, 5);
        let change2 = files2
            .active
            .iter()
            .find(|c| c.name == "y")
            .expect("change y is read");
        assert_eq!(
            change2.progress,
            Progress {
                completed: 0,
                total: 0
            }
        );
        assert!(
            change2.problems.iter().any(|p| p.contains("tasks.md")),
            "{:?}",
            change2.problems
        );

        for width in [78, 58] {
            for change in [change, change2] {
                let d = Detail {
                    source: String::new(),
                    scroll: 0,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
                };
                let lines = content_lines(&d, Some(change), width);
                assert!(
                    lines.iter().any(|l| l.text().starts_with('!')),
                    "width {width}: {:?}",
                    lines_text(&lines)
                );
            }
        }
    }

    #[test]
    fn a_wrapped_paragraph_produces_strictly_more_lines_at_58_than_at_78() {
        let paragraph = format!("{}\n", "word ".repeat(40).trim());
        assert!(paragraph.chars().count() > 100);
        let d = detail(&paragraph, Vec::new());
        let lines78 = content_lines(&d, None, 78);
        let lines58 = content_lines(&d, None, 58);
        assert!(
            lines58.len() > lines78.len(),
            "58: {}, 78: {}",
            lines58.len(),
            lines78.len()
        );
        for line in &lines78 {
            assert!(line.text().chars().count() <= 78);
        }
        for line in &lines58 {
            assert!(line.text().chars().count() <= 58);
        }
    }

    // --- group 6: the tracked-tasks dispatch -------------------------

    /// `ids`, each with no paths — the `(&str, &[&str])` shape
    /// `changes::fixture::with_marked_artifacts` (in turn
    /// `changes::fixture::with_artifacts`) needs. Returns pairs, not a
    /// change value, so this helper's own return type never names that
    /// type at all — `NOLIT-CHANGE`'s pattern would catch a return type
    /// exactly as it would a literal.
    fn paths_free<'a>(ids: &'a [&'a str]) -> Vec<(&'a str, &'a [&'a str])> {
        ids.iter().map(|id| (*id, &[][..])).collect()
    }

    #[test]
    fn marked_tab_returns_the_checklist_body() {
        let progress = Progress {
            completed: 1,
            total: 3,
        };
        let change =
            fixture::with_marked_artifacts(&paths_free(&["proposal", "tasks"]), Some(1), progress);
        let source = "## 1. Setup\n\n- [x] a\n- [ ] b\n- [ ] c\n";
        for width in [78, 58] {
            let d = Detail {
                source: source.to_string(),
                scroll: 0,
                tab: 1,
                problems: Vec::new(),
                loaded: None,
            };
            let lines = content_lines(&d, Some(&change), width);
            // Discriminating: the marked body's first line is the bar,
            // which the markdown rendering of the same source never
            // produces.
            assert_eq!(
                lines[0].text(),
                crate::ui::tasks::progress_bar(&progress, width),
                "width {width}"
            );
            let want = crate::ui::tasks::lines(source, &progress, width);
            assert_eq!(lines, want, "width {width}");
        }
    }

    #[test]
    fn unmarked_tab_returns_the_markdown_body() {
        let progress = Progress {
            completed: 1,
            total: 3,
        };
        let change =
            fixture::with_marked_artifacts(&paths_free(&["proposal", "tasks"]), Some(1), progress);
        let source = "## 1. Setup\n\n- [x] a\n- [ ] b\n- [ ] c\n";
        for width in [78, 58] {
            // tab 0 ("proposal") is not the marked position.
            let d = Detail {
                source: source.to_string(),
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
            };
            let lines = content_lines(&d, Some(&change), width);
            let want = crate::ui::markdown::lines(source, width);
            assert_eq!(lines, want, "width {width}");
            assert_ne!(
                lines[0].text(),
                crate::ui::tasks::progress_bar(&progress, width),
                "width {width}: markdown source starts with its own text, not the bar"
            );
        }
    }

    #[test]
    fn tab_past_the_end() {
        let progress = Progress {
            completed: 1,
            total: 1,
        };
        let change =
            fixture::with_marked_artifacts(&paths_free(&["proposal", "tasks"]), Some(1), progress);
        let source = "- [x] a\n";
        for width in [78, 58] {
            let d = Detail {
                source: source.to_string(),
                scroll: 0,
                tab: 7,
                problems: Vec::new(),
                loaded: None,
            };
            let lines = content_lines(&d, Some(&change), width);
            assert_eq!(
                lines,
                crate::ui::markdown::lines(source, width),
                "width {width}"
            );

            // The same holds for a change carrying no artifacts at all.
            let empty_artifacts = fixture::with_marked_artifacts(&paths_free(&[]), None, progress);
            let lines_empty = content_lines(&d, Some(&empty_artifacts), width);
            assert_eq!(
                lines_empty,
                crate::ui::markdown::lines(source, width),
                "width {width}"
            );
        }
    }

    #[test]
    fn content_lines_total() {
        let marked = fixture::with_marked_artifacts(
            &paths_free(&["a", "b"]),
            Some(0),
            Progress {
                completed: 1,
                total: 2,
            },
        );
        let unmarked = fixture::with_marked_artifacts(
            &paths_free(&["a", "b"]),
            None,
            Progress {
                completed: 1,
                total: 2,
            },
        );
        let no_artifacts = fixture::with_marked_artifacts(
            &paths_free(&[]),
            None,
            Progress {
                completed: 0,
                total: 0,
            },
        );
        let paragraph = format!("{}\n", "word ".repeat(40).trim());
        let details = [
            detail("", Vec::new()),
            detail("", vec!["/repo/a.md: boom".to_string()]),
            detail("- [ ] only\n", Vec::new()),
            detail("- [ ] only\n", vec!["/repo/a.md: boom".to_string()]),
            {
                let mut d = detail(&paragraph, Vec::new());
                d.tab = 0;
                d
            },
            {
                let mut d = detail("- [ ] a\n", Vec::new());
                d.tab = 9;
                d
            },
        ];
        for width in [78, 58] {
            for change in [None, Some(&marked), Some(&unmarked), Some(&no_artifacts)] {
                for d in &details {
                    let lines = content_lines(d, change, width);
                    for line in &lines {
                        assert!(
                            line.text().chars().count() <= width as usize,
                            "width {width}, source {:?}: {:?} exceeds its width",
                            d.source,
                            line.text()
                        );
                    }
                }
            }
        }
        // The wrapped paragraph (details[4], tab 0 — unmarked under every
        // one of the four `change` shapes above) produces strictly more
        // lines at 58 than at 78, so the width genuinely reaches the wrap.
        let at_78 = content_lines(&details[4], None, 78).len();
        let at_58 = content_lines(&details[4], None, 58).len();
        assert!(
            at_58 > at_78,
            "58: {at_58}, 78: {at_78} — the width must genuinely reach the wrap"
        );
    }

    /// `artifact-content` :: total function, no panic — `change: None`, an empty `Detail`,
    /// and width `0`, per task 5.3. Also driven at 78 and 58, both of which behave
    /// identically to width `0` here (an empty source and no problems is always "No content
    /// yet", regardless of width).
    #[test]
    fn content_lines_never_panics_with_no_change_an_empty_detail_and_zero_width() {
        let d = detail("", Vec::new());
        for width in [0, 78, 58] {
            let lines = content_lines(&d, None, width);
            assert_eq!(lines.len(), 1, "width {width}");
            assert_eq!(lines[0].text(), "No content yet", "width {width}");
        }
    }
}
