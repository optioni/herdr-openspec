//! The detail region's grammar: plain data, with no I/O API and no
//! dependency on how the view styles it — the same shape `ui::list` uses
//! for the row grammar and `ui::markdown` for the document. Every public
//! function here takes an interior width, which is what makes
//! `DETAILWIDTHS` possible with no exemption list. See
//! `openspec/changes/detail-view/design.md` -> Boundaries and Contracts.
//!
//! Every measurement and truncation here reaches the crate's one display-
//! width measure, `crate::ui::layout::columns`, and nowhere counts
//! `char`s. See
//! `openspec/changes/view-fidelity/specs/responsive-layout/spec.md` ->
//! "Display width is measured in terminal columns by one pair of
//! primitives".

use crate::ui::layout::columns;

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
    let progress_len = columns(&progress_cell) as i64;
    let schema_cell = format!("({schema})");
    let schema_len = columns(&schema_cell) as i64;

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
    if columns(text) <= width {
        text.to_string()
    } else {
        crate::ui::list::pad_or_truncate_right(text, width)
    }
}

/// The joined width of cells `start..=end`: their own widths plus **one**
/// separating column between every pair. One column is the minimum that
/// still shows an edge between two painted chips; two read as two chips
/// with a gap rather than as a segmented control (`color-palette` ->
/// design.md -> Decision 6).
fn joined_width(cell_lens: &[usize], start: usize, end: usize) -> usize {
    let cells_width: usize = cell_lens[start..=end].iter().sum();
    let seps = end - start;
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
///
/// Every cell is a **chip**: the bare `<id>` with one space on each side, so
/// `ui::view` has a `columns(id) + 2` span to paint and the reported cell and
/// the painted cell are one object (`color-palette` -> design.md -> Decision
/// 5). No label carries a leading digit — `1`-`9` still select a tab, but the
/// bar no longer advertises them.
pub fn tab_bar(artifacts: &[crate::changes::ArtifactRef], selected: usize, width: u16) -> Vec<Tab> {
    if width == 0 {
        return Vec::new();
    }
    let w = width as usize;

    if artifacts.is_empty() {
        return vec![Tab {
            text: cell_text(" no artifacts ", w),
            x: 0,
            index: None,
            selected: false,
        }];
    }

    let n = artifacts.len();
    let cells: Vec<String> = artifacts.iter().map(|a| format!(" {} ", a.id)).collect();
    let cell_lens: Vec<usize> = cells.iter().map(|c| columns(c)).collect();

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
        x += cell_lens[i] as u16 + 1;
    }
    out
}

/// The artifact position of the tab cell [`tab_bar`] places over `column` — an
/// offset from the bar's own first column — and `None` when that column holds
/// no addressable cell.
///
/// A column holds a cell exactly when it falls within
/// `[cell.x, cell.x + columns(cell.text))` for a [`Tab`] whose `index` is
/// `Some`. The one separating column between two chips belongs to neither and
/// returns `None`; so does a column past the last drawn cell, a column covered
/// only by the `no artifacts` placeholder, and every column when `width` is `0`.
///
/// Built from `tab_bar`'s own output rather than from a second placement
/// calculation, so the cell a click lands on and the cell painted there are the
/// same cell — the same reason [`Tab`] reports its `x` rather than letting the
/// view derive one. A cell's width is measured through
/// [`crate::ui::layout::columns`], never a `char` count, so a bar whose artifact
/// ids carry a CJK or emoji character targets the cell the reader sees.
///
/// Pure and total: no I/O, no clock, and no panic for any artifact slice
/// including an empty one, any `selected` including one past the end, any
/// `width`, and any `column`.
pub fn tab_at(
    artifacts: &[crate::changes::ArtifactRef],
    selected: usize,
    width: u16,
    column: u16,
) -> Option<usize> {
    tab_bar(artifacts, selected, width)
        .into_iter()
        .find_map(|cell| {
            let index = cell.index?;
            let end = cell.x as usize + columns(&cell.text);
            ((cell.x..end as u16).contains(&column)).then_some(index)
        })
}

/// One row `content_lines` returns: a rendered `markdown::Line` plus what
/// kind of row it is. Plain data, naming no styling type at all — the same
/// shape `ui::list::Row`/`RowKind` already give the list region, so
/// `ui::view` alone decides what a row looks like
/// (`openspec/changes/foldable-spec-sections/design.md` -> Decision 12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentRow {
    pub line: crate::ui::markdown::Line,
    pub kind: ContentKind,
}

impl ContentRow {
    /// `self.line.text()` — named here so most of this module's tests read
    /// exactly as they did while `content_lines` returned a bare line list.
    pub fn text(&self) -> String {
        self.line.text()
    }
}

/// What a [`ContentRow`] represents: a problem line (either problem source
/// `artifact-content` stacks, change first), an ordinary body line, or a
/// foldable artifact's own section-header row, naming its section's index
/// and whether the cursor is on or in that section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentKind {
    Problem,
    Body,
    SectionHeader { section: usize, selected: bool },
}

/// A foldable tab's own section-header row: `<glyph> <label>`, where the
/// glyph is `ui::list::fold_glyph`'s own — the crate's one site for that
/// pair, so one fold reads the same glyph in both regions
/// (design.md -> Decision 9). Passed through the crate's one right-
/// truncation grammar, `ui::list::pad_or_truncate_right`, so the glyph and
/// its separating space — emitted first — survive any truncation the label
/// needs and the row still measures at most `width` display columns.
fn header(label: &str, expanded: bool, width: u16) -> String {
    let glyph = crate::ui::list::fold_glyph(!expanded);
    crate::ui::list::pad_or_truncate_right(&format!("{glyph} {label}"), width as usize)
}

/// The `No content yet` row, padded to `width` on the same terms every
/// other row here is — `view-fidelity`'s repair, carried forward unchanged.
fn no_content_yet_row(width: u16) -> ContentRow {
    ContentRow {
        line: crate::ui::markdown::Line {
            segments: vec![crate::ui::markdown::Segment {
                text: crate::ui::list::pad_or_truncate_right("No content yet", width as usize),
                face: crate::ui::markdown::Face::plain(),
            }],
        },
        kind: ContentKind::Body,
    }
}

/// The `"! <problem>"` row both problem sources share.
fn problem_row(p: &str, width: u16) -> ContentRow {
    ContentRow {
        line: crate::ui::markdown::Line {
            segments: vec![crate::ui::markdown::Segment {
                text: crate::ui::list::pad_or_truncate_right(&format!("! {p}"), width as usize),
                face: crate::ui::markdown::Face::plain(),
            }],
        },
        kind: ContentKind::Problem,
    }
}

fn body_row(line: crate::ui::markdown::Line) -> ContentRow {
    ContentRow {
        line,
        kind: ContentKind::Body,
    }
}

/// The single row list both `ui::view::render` and
/// `Dashboard::normalise_scroll` derive the detail content from, so the
/// drawn slice and the scroll clamp can never disagree about how many rows
/// there are: one `Problem` row per entry of the selected change's own
/// `problems` (`degraded-states`' addition — `SPEC.md` rows naming a reason
/// on `Change::problems` that nothing rendered before this change), then
/// one such row per `detail.problems` entry, then the selected tab's
/// **body**, and — only when all three are empty — exactly one `No content
/// yet` row. When either problem source is non-empty and `sections` is
/// empty, the problem rows alone are returned: the reason is known, and
/// adding `No content yet` would say two contradictory things about the
/// same tab.
///
/// The body is `ui::tasks::lines(&text, &change.progress, width)` —
/// `tasks-checklist`'s grammar and `tasks-progress-bar`'s leading line —
/// over the concatenation of every section's text, when `change` is `Some`
/// and the `ArtifactRef` at `detail.tab` carries `tracks_tasks == true`
/// (`artifact-folds` -> Decision 8: this tab is never foldable, at any
/// section count); `artifact-folds`' own header rows and per-section
/// bodies when the artifact is **foldable** (`detail.sections.len() > 1`,
/// derived rather than stored — Decision 3); and
/// `ui::markdown::lines(&text, width)` over the same concatenation in every
/// other case — a single section, no section at all, a `None` change, a
/// `detail.tab` past the end of the artifact list, and a change carrying no
/// artifacts at all. Each header row carries `ContentKind::SectionHeader {
/// section, selected }`, where `selected` is true for exactly the header
/// whose section the cursor — `detail.scroll`, an index into this same row
/// list — is on or in, and false on every header when the cursor addresses
/// a problem row or when there are no sections.
///
/// The decision is made exactly here, once, so `ui::view::render` and
/// `Dashboard::normalise_scroll` — both of which pass
/// `Dashboard::selected_change()` — can never disagree about which grammar
/// the tab holds.
pub fn content_lines(
    detail: &crate::ui::app::Detail,
    change: Option<&crate::changes::Change>,
    width: u16,
) -> Vec<ContentRow> {
    // `degraded-states`: the selected change's own problems lead, above `detail.problems` —
    // "change_problem_precedes_tab_problem" — using the SAME `pad_or_truncate_right` call and
    // plain face `detail.problems` already renders with, on the existing problem-row
    // mechanism rather than a new one.
    let mut out: Vec<ContentRow> = change
        .map(|c| c.problems.as_slice())
        .unwrap_or(&[])
        .iter()
        .map(|p| problem_row(p, width))
        .collect();
    out.extend(detail.problems.iter().map(|p| problem_row(p, width)));
    let problem_count = out.len();

    let tracked_tasks_progress = change.and_then(|c| {
        c.artifacts
            .get(detail.tab)
            .filter(|a| a.tracks_tasks)
            .map(|_| &c.progress)
    });

    match tracked_tasks_progress {
        Some(progress) => {
            // `artifact-folds` -> Decision 8: every section's text
            // concatenated in order, with no separator inserted, never
            // folded — exactly the grammar this tab had before
            // `artifact-folds` existed.
            let text: String = detail.sections.iter().map(|s| s.text.as_str()).collect();
            out.extend(
                crate::ui::tasks::lines(&text, progress, width)
                    .into_iter()
                    .map(body_row),
            );
        }
        None if detail.sections.len() > 1 => {
            // `artifact-folds`: a header row per section, in order, each
            // followed by that section's own rendered markdown exactly
            // when it is open.
            for (index, section) in detail.sections.iter().enumerate() {
                let expanded = detail.expanded.contains(&index);
                out.push(ContentRow {
                    line: crate::ui::markdown::Line {
                        segments: vec![crate::ui::markdown::Segment {
                            text: header(&section.label, expanded, width),
                            face: crate::ui::markdown::Face::plain(),
                        }],
                    },
                    kind: ContentKind::SectionHeader {
                        section: index,
                        selected: false,
                    },
                });
                if expanded {
                    out.extend(
                        crate::ui::markdown::lines(&section.text, width)
                            .into_iter()
                            .map(body_row),
                    );
                }
            }
        }
        None => {
            let text: String = detail.sections.iter().map(|s| s.text.as_str()).collect();
            out.extend(
                crate::ui::markdown::lines(&text, width)
                    .into_iter()
                    .map(body_row),
            );
        }
    }

    // The header whose section the cursor is on or in: the greatest
    // section-header row index at or before `detail.scroll`. None at all
    // when the cursor addresses a problem row (every problem row precedes
    // every section) or there are no section headers to begin with — every
    // `selected` already defaults to `false` above, so there is nothing
    // further to do on either of those paths.
    if detail.scroll >= problem_count {
        let cursor_section = out
            .iter()
            .enumerate()
            .filter_map(|(index, row)| match row.kind {
                ContentKind::SectionHeader { section, .. } => Some((index, section)),
                _ => None,
            })
            .take_while(|(index, _)| *index <= detail.scroll)
            .map(|(_, section)| section)
            .last();
        if let Some(cursor_section) = cursor_section {
            for row in &mut out {
                if let ContentKind::SectionHeader { section, selected } = &mut row.kind {
                    *selected = *section == cursor_section;
                }
            }
        }
    }

    if out.is_empty() {
        out.push(no_content_yet_row(width));
    }
    out
}

/// The section index of the [`ContentKind::SectionHeader`] drawn `row` rows
/// below `rows[offset]` — a **lookup**, not a second derivation: it indexes
/// `rows` at `offset + row` (saturating, so it can never panic or wrap) and
/// reads that row's own kind. `None` for every row that is not a section
/// header, including one past the end of `rows`. See design.md ->
/// Decision 12 and `specs/artifact-folds/spec.md` -> "A section header row
/// resolves to its own section index". Takes no drawn geometry of its own:
/// the caller that has a rectangle has already used it to decide there is a
/// row here at all — the drawn row list and offset arrive as arguments
/// instead, so a click and the pixels it landed on can never disagree.
pub fn section_at(rows: &[ContentRow], offset: usize, row: u16) -> Option<usize> {
    let index = offset.saturating_add(row as usize);
    match rows.get(index)?.kind {
        ContentKind::SectionHeader { section, .. } => Some(section),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ContentKind, ContentRow, Tab, content_lines, header, header_row, section_at, tab_bar,
    };
    use crate::changes::fixture;
    use crate::tasks::Progress;
    use crate::testutil::{cell, render_at, row_text};
    use crate::ui::app::{ArtifactSection, Dashboard, Detail, Filter, Route};
    use crate::ui::layout::columns;

    /// Three sections with short, single-line bodies, so a scenario about
    /// header rows and folding does not also have to reason about
    /// `ui::markdown`'s own wrapping. Labels match the ones
    /// `ui::app::tests::the_three_spec_files_of_a_change_become_three_labelled_sections`
    /// already fixes for the same three-file shape, so a reader who has seen
    /// that test recognises this one.
    fn three_spec_detail(expanded: std::collections::BTreeSet<usize>) -> Detail {
        Detail {
            sections: vec![
                ArtifactSection {
                    label: "degraded-coverage".to_string(),
                    text: "one\n".to_string(),
                },
                ArtifactSection {
                    label: "markdown-render".to_string(),
                    text: "two\n".to_string(),
                },
                ArtifactSection {
                    label: "tasks-checklist".to_string(),
                    text: "three\n".to_string(),
                },
            ],
            scroll: 0,
            tab: 0,
            problems: Vec::new(),
            loaded: None,
            expanded,
        }
    }

    fn detail(source: &str, problems: Vec<String>) -> Detail {
        Detail {
            sections: if source.is_empty() {
                Vec::new()
            } else {
                vec![ArtifactSection {
                    label: String::new(),
                    text: source.to_string(),
                }]
            },
            scroll: 0,
            tab: 0,
            problems,
            loaded: None,
            expanded: std::collections::BTreeSet::new(),
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

    fn empty_filter() -> Filter {
        Filter {
            query: String::new(),
            active: false,
        }
    }

    /// `mouse-input`: a column of the tab bar resolves to the tab drawn there.
    /// Every expected span is read off `tab_bar`'s own output, never computed a
    /// second time — the cell a click lands on and the cell painted there must
    /// be the same cell.
    mod tab_at {
        use crate::ui::detail::{tab_at, tab_bar};
        use crate::ui::layout::columns;

        use super::artifacts;

        /// The columns `tab_bar` reported for cell `index`, as an inclusive-start,
        /// exclusive-end range over the bar's own first column.
        fn span(
            a: &[crate::changes::ArtifactRef],
            selected: usize,
            width: u16,
            index: usize,
        ) -> std::ops::Range<u16> {
            let cell = tab_bar(a, selected, width)
                .into_iter()
                .find(|t| t.index == Some(index))
                .expect("the cell is drawn");
            cell.x..cell.x + columns(&cell.text) as u16
        }

        #[test]
        fn each_cell_answers_for_its_own_columns() {
            let a = artifacts(&["proposal", "specs", "design", "tasks", "planning-review"]);
            let widths: [u16; 2] = [78, 58];
            for width in widths {
                let bar = tab_bar(&a, 0, width);
                assert_eq!(bar.len(), 5, "width {width}");
                for index in 0..5 {
                    let expected = span(&a, 0, width, index);
                    let answered: Vec<u16> = (0..width)
                        .filter(|c| tab_at(&a, 0, width, *c) == Some(index))
                        .collect();
                    assert_eq!(
                        answered,
                        expected.clone().collect::<Vec<_>>(),
                        "width {width}, cell {index}: the columns returning Some(i) are \
                         exactly the span tab_bar reported"
                    );
                }
                // The one separating column between two cells belongs to neither.
                for pair in bar.windows(2) {
                    let separator = pair[0].x + columns(&pair[0].text) as u16;
                    assert_eq!(separator, pair[1].x - 1);
                    assert_eq!(tab_at(&a, 0, width, separator), None, "width {width}");
                }
                // Every column past the last drawn cell.
                let last = bar.last().expect("cells are drawn");
                let end = last.x + columns(&last.text) as u16;
                for column in end..width {
                    assert_eq!(tab_at(&a, 0, width, column), None, "width {width}");
                }
            }
        }

        #[test]
        fn a_windowed_bar_answers_for_drawn_cells() {
            let ids: Vec<String> = (0..12).map(|i| format!("artifact-{i:02}")).collect();
            let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
            let a = artifacts(&refs);
            // The spec names width 78; the check is run at 58 too, which windows
            // harder and is what `DETAILWIDTHS` requires of every test here.
            let widths: [u16; 2] = [78, 58];
            for width in widths {
                let bar = tab_bar(&a, 9, width);
                let drawn: Vec<usize> = bar.iter().filter_map(|t| t.index).collect();
                assert!(
                    bar.len() < 12,
                    "width {width}: the fixture must actually window, {} cells drawn",
                    bar.len()
                );
                let first = *drawn.first().expect("a cell is drawn");
                assert_ne!(first, 0, "width {width}: the window does not start at 0");

                for column in 0..width {
                    if let Some(index) = tab_at(&a, 9, width, column) {
                        assert!(
                            drawn.contains(&index),
                            "width {width} column {column} answered {index}, outside the \
                             drawn window {drawn:?}"
                        );
                    }
                }
                for column in span(&a, 9, width, first) {
                    assert_eq!(tab_at(&a, 9, width, column), Some(first));
                }
            }
        }

        #[test]
        fn the_placeholder_addresses_nothing() {
            let five = artifacts(&["proposal", "specs", "design", "tasks", "planning-review"]);
            let widths: [u16; 2] = [78, 58];
            for width in widths {
                // The `no artifacts` placeholder carries `index: None`.
                for column in 0..width {
                    assert_eq!(tab_at(&[], 0, width, column), None, "column {column}");
                }
                // `column` past every drawn cell.
                assert_eq!(tab_at(&five, 0, width, 65535), None);

                // A `selected` past the end still addresses the cells the bar
                // drew, since windowing treats it as `0`.
                for cell in tab_bar(&five, 99, width) {
                    let index = cell.index.expect("a real cell, not the placeholder");
                    for column in span(&five, 99, width, index) {
                        assert_eq!(tab_at(&five, 99, width, column), Some(index));
                    }
                }
            }
            // Width 0 draws nothing at all.
            for column in [0u16, 1, 65535] {
                assert_eq!(tab_at(&five, 0, 0, column), None);
            }
        }

        #[test]
        fn a_wide_id_is_addressed_by_columns() {
            // `日` is two terminal columns wide.
            let a = artifacts(&["日x", "next"]);
            let widths: [u16; 2] = [78, 58];
            for width in widths {
                let first = span(&a, 0, width, 0);
                assert_eq!(
                    first.clone().count(),
                    columns(" 日x "),
                    "width {width}: the cell's span is measured in display columns, not chars"
                );
                // Both columns of the two-column character answer for the cell.
                for column in first.clone() {
                    assert_eq!(tab_at(&a, 0, width, column), Some(0), "column {column}");
                }
                let second = span(&a, 0, width, 1);
                assert_eq!(tab_at(&a, 0, width, second.start), Some(1));
                assert_eq!(tab_at(&a, 0, width, first.end), None, "the separator");
            }
        }
    }

    /// `artifact-folds` :: "A narrow pane truncates the label and keeps the
    /// glyph" — the header-row grammar itself, called directly rather than
    /// through `content_lines`, since the scenario is about `header`'s own
    /// truncation rather than the tab's body.
    mod header {
        use super::header;
        use crate::ui::layout::columns;
        use crate::ui::list::fold_glyph;

        /// The content areas a 20- and a 15-column narrow frame produce
        /// below the 100-column breakpoint are 18 and 13 columns
        /// (`layout::interior`'s own two gutter columns) — `artifact-folds`
        /// states the frame widths; this test states the content widths
        /// `header` itself is called with.
        #[test]
        fn a_narrow_pane_truncates_the_label_and_keeps_the_glyph() {
            let label = "degraded-coverage";
            let glyph = fold_glyph(true);

            let at_18 = header(label, false, 18);
            assert_eq!(columns(&at_18), 18);
            assert_eq!(at_18, format!("{glyph} degraded-covera…"));

            let at_13 = header(label, false, 13);
            assert_eq!(columns(&at_13), 13);
            assert_eq!(at_13, format!("{glyph} degraded-c…"));

            // The glyph and its separating space survive the truncation at
            // both widths: the row still opens with them, unchanged.
            assert!(at_18.starts_with(&format!("{glyph} ")));
            assert!(at_13.starts_with(&format!("{glyph} ")));

            // The mandated pair, named explicitly per DETAILWIDTHS: the
            // label fits whole at both, so the row is padded rather than
            // truncated, and still opens with the glyph and its space.
            for width in [78, 58] {
                let got = header(label, false, width);
                assert_eq!(columns(&got), width as usize, "width {width}");
                assert!(got.trim_end().ends_with(label), "width {width}: {got:?}");
                assert!(
                    got.starts_with(&format!("{glyph} ")),
                    "width {width}: {got:?}"
                );
            }
        }

        /// A CJK label is truncated in display columns, not in `char`s, so
        /// its header still measures at most the content width even though
        /// its `chars().count()` would be smaller than that.
        #[test]
        fn a_cjk_label_is_truncated_in_columns() {
            let label = "日本語のラベルです見出しの続き";
            for width in [18u16, 13, 78, 58] {
                let got = header(label, false, width);
                assert!(
                    columns(&got) <= width as usize,
                    "width {width}: {got:?} exceeds its width"
                );
            }
        }

        /// Total and never panicking from `0` through `20`, plus the
        /// mandated pair — `header` is one of the functions `DETAILWIDTHS`
        /// requires to name both.
        #[test]
        fn header_is_total_from_zero_through_twenty_columns() {
            let label = "degraded-coverage";
            for expanded in [false, true] {
                for width in 0u16..=20 {
                    let got = header(label, expanded, width);
                    assert!(
                        columns(&got) <= width as usize,
                        "expanded {expanded} width {width}: {got:?}"
                    );
                }
                for width in [78, 58] {
                    let got = header(label, expanded, width);
                    assert_eq!(columns(&got), width as usize, "width {width}");
                }
            }
        }
    }

    /// `artifact-folds` :: "Each drawn header row resolves to its own
    /// index".
    #[test]
    fn each_drawn_header_row_resolves_to_its_own_index() {
        for width in [78, 58] {
            let collapsed = three_spec_detail(std::collections::BTreeSet::new());
            let rows = content_lines(&collapsed, None, width);
            assert_eq!(rows.len(), 3, "width {width}");
            assert_eq!(section_at(&rows, 0, 0), Some(0), "width {width}");
            assert_eq!(section_at(&rows, 0, 1), Some(1), "width {width}");
            assert_eq!(section_at(&rows, 0, 2), Some(2), "width {width}");
            assert_eq!(section_at(&rows, 0, 3), None, "width {width}");

            let mut expanded = std::collections::BTreeSet::new();
            expanded.insert(0);
            let opened = three_spec_detail(expanded);
            let rows2 = content_lines(&opened, None, width);
            // Row 0 is section 0's header; row 1 is the first (and only)
            // line of its body, which resolves to no section at all; the
            // header that follows resolves to section 1.
            assert_eq!(section_at(&rows2, 0, 0), Some(0), "width {width}");
            assert_eq!(section_at(&rows2, 0, 1), None, "width {width}");
            let header1 = rows2
                .iter()
                .position(|r| matches!(r.kind, ContentKind::SectionHeader { section: 1, .. }))
                .expect("section 1's header is drawn");
            assert_eq!(
                section_at(&rows2, 0, header1 as u16),
                Some(1),
                "width {width}"
            );
        }
    }

    /// `artifact-folds` :: "Resolution is total and inert where it should
    /// be".
    #[test]
    fn resolution_is_total_and_inert_where_it_should_be() {
        for width in [78, 58] {
            // A non-foldable dashboard's row list: every row is `Body`, so
            // `section_at` answers `None` everywhere, including past the
            // end.
            let single = detail("# heading\n", Vec::new());
            let rows = content_lines(&single, None, width);
            for row in 0..(rows.len() as u16 + 5) {
                assert_eq!(section_at(&rows, 0, row), None, "width {width} row {row}");
            }

            // An empty row list.
            let empty: Vec<ContentRow> = Vec::new();
            assert_eq!(section_at(&empty, 0, 0), None, "width {width}");

            // `offset` past the end, at `usize::MAX`, and `row` at
            // `u16::MAX` — every combination saturates rather than
            // panicking or wrapping.
            assert_eq!(section_at(&rows, rows.len() + 10, 0), None, "width {width}");
            assert_eq!(section_at(&rows, usize::MAX, 0), None, "width {width}");
            assert_eq!(section_at(&rows, 0, u16::MAX), None, "width {width}");
            assert_eq!(
                section_at(&rows, usize::MAX, u16::MAX),
                None,
                "width {width}"
            );
        }
    }

    /// A `Dashboard` at `Route::Detail` with exactly one active change
    /// selected — the shape every full-frame render test in this module's
    /// "measuring in columns" section needs. Mirrors `ui::view`'s own
    /// private `dashboard_with_detail` test helper field for field;
    /// duplicated rather than shared because that helper is private to
    /// `ui::view`'s own test module and this module builds no `Dashboard`
    /// anywhere else.
    fn dashboard_at_detail(change: crate::changes::Change, detail: Detail) -> Dashboard {
        Dashboard {
            repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
            searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
            changes: fixture::set(vec![change], Vec::new(), Vec::new()),
            route: Route::Detail,
            quit: false,
            // `list-sections`: 1, not 0 — target 0 is the active header.
            selected: 1,
            filter: empty_filter(),
            detail,
            refresh: crate::ui::app::Refresh {
                requested: false,
                reload: false,
                startup: Vec::new(),
                problems: Vec::new(),
            },
            agents: crate::agents::AgentSnapshot {
                agents: Vec::new(),
                reachable: false,
                stalled: false,
                problem: None,
            },
            agent_names: crate::state::Mapping::default(),
            launch: crate::ui::app::Launch {
                pending: None,
                in_flight: false,
                problems: Vec::new(),
            },
            sections: crate::ui::app::Sections {
                collapsed: std::collections::BTreeSet::new(),
            },
            file_mode: false,
        }
    }

    /// Columns `range` of `text`, by character index — never by byte
    /// offset. Mirrors `ui::view`'s own private test-only helper of the
    /// same name and the same reason: a box-drawing border or a header's
    /// `…` is multi-byte.
    fn cols(text: &str, range: std::ops::Range<usize>) -> String {
        text.chars().skip(range.start).take(range.len()).collect()
    }

    #[test]
    fn the_full_header_grammar_at_both_mandated_interior_widths() {
        let progress = Progress {
            completed: 4,
            total: 42,
        };
        for (width, name_field_width) in [(78, 65usize), (58, 45)] {
            let got = header_row("detail-view", "tdd", &progress, width);
            assert_eq!(columns(&got), width as usize, "width {width}");
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
            assert_eq!(columns(&got), width as usize, "width {width}");
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
            assert_eq!(columns(&got), width as usize, "width {width}");
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
            assert_eq!(columns(&got), w as usize, "width {w}");
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
            assert_eq!(columns(&got), width as usize, "width {width}");
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
                    " proposal ",
                    " specs ",
                    " design ",
                    " tasks ",
                    " planning-review "
                ],
                "width {width}"
            );
            // Every chip begins and ends in a space: the padding is part of
            // `text`, so the reported cell and the painted cell are one object.
            assert!(
                texts.iter().all(|t| t.starts_with(' ') && t.ends_with(' ')),
                "width {width}: {texts:?}"
            );
            let xs: Vec<u16> = tabs.iter().map(|t| t.x).collect();
            assert_eq!(xs, vec![0, 11, 19, 28, 36], "width {width}");
            // Each chip starts exactly one column after the previous chip's
            // last column — one separating column, never two.
            for pair in tabs.windows(2) {
                assert_eq!(
                    pair[1].x as usize,
                    pair[0].x as usize + columns(&pair[0].text) + 1,
                    "width {width}"
                );
            }
            let last = tabs.last().unwrap();
            let last_end = last.x as usize + columns(&last.text) - 1;
            assert_eq!(last_end, 52, "width {width}");
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
            assert_eq!(texts, vec![" spec ", " spec ", " notes "], "width {width}");
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
            // Nine five-column chips and eight separators are 53 columns; a
            // tenth would be 59. So the wide bar holds all twelve and the
            // narrow one holds nine — the two widths differ, and the narrower
            // is not silently vacuous.
            let expected_len = if width == 78 { 12 } else { 9 };
            assert_eq!(tabs.len(), expected_len, "width {width}");
            for (i, tab) in tabs.iter().enumerate() {
                assert_eq!(tab.text, format!(" a{:02} ", i + 1), "width {width}");
                assert_eq!(columns(&tab.text), 5, "width {width}");
                assert!(
                    !tab.text
                        .trim_start()
                        .starts_with(|c: char| c.is_ascii_digit()),
                    "width {width}: {:?} carries a leading digit",
                    tab.text
                );
                assert!(!tab.text.starts_with("1 "), "width {width}");
            }
        }
    }

    #[test]
    fn no_artifacts_is_a_single_placeholder_cell_not_an_empty_bar() {
        let a = artifacts(&[]);
        for width in [78, 58] {
            let tabs = tab_bar(&a, 0, width);
            assert_eq!(tabs.len(), 1, "width {width}");
            // Drawn on the bar's own terms: a chip, padding included, so the
            // placeholder occupies fourteen columns and nothing else.
            assert_eq!(tabs[0].text, " no artifacts ", "width {width}");
            assert_eq!(columns(&tabs[0].text), 14, "width {width}");
            assert_eq!(tabs[0].x, 0, "width {width}");
            assert_eq!(tabs[0].index, None, "width {width}");
            assert!(!tabs[0].selected, "width {width}");
        }
        let narrow = tab_bar(&artifacts(&[]), 0, 8);
        assert_eq!(narrow.len(), 1);
        assert_eq!(columns(&narrow[0].text), 8);
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
                // A 13-column chip each: four fit in 58 columns (52 + 3
                // separators = 55) and five in 78 (65 + 4 = 69).
                assert_eq!(
                    tabs.len(),
                    if width == 78 { 5 } else { 4 },
                    "width {width} selected {selected}"
                );
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
                let last_end = last.x as usize + columns(&last.text);
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
            assert_eq!(columns(&tabs[0].text), width as usize, "width {width}");
            assert!(tabs[0].text.ends_with('…'), "width {width}");
            // What is truncated is the padded chip, not a bare id: the
            // padding is what the view paints, so it is what is measured.
            assert!(tabs[0].text.starts_with(' '), "width {width}");
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
            // The window is the one `selected: 0` would have produced: all
            // three three-column chips from column 0, one column apart.
            let xs: Vec<u16> = tabs.iter().map(|t| t.x).collect();
            assert_eq!(xs, vec![0, 4, 8], "width {width}");
        }
    }

    #[test]
    fn an_empty_detail_returns_exactly_one_no_content_yet_line() {
        for width in [78, 58] {
            let lines = content_lines(&detail("", Vec::new()), None, width);
            assert_eq!(lines.len(), 1, "width {width}");
            // Padded to the full width, on the same terms every neighbouring line already
            // is (`view-fidelity`'s repair) — see the narrow-frame and total-width tests
            // below for the range in which the literal itself is longer than the region.
            let want = format!("No content yet{}", " ".repeat(width as usize - 14));
            assert_eq!(lines[0].text(), want, "width {width}");
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

    /// Renders `rows` as plain text, for an assertion failure message only.
    fn lines_text(rows: &[ContentRow]) -> Vec<String> {
        rows.iter().map(|r| r.text()).collect()
    }

    /// Asserts every row in `got` carries a `line` equal to the
    /// corresponding entry of `want`, in order, and that `got` has exactly
    /// `want`'s length — the comparison `content_lines`' tracked-tasks and
    /// non-foldable branches must always satisfy, since each wraps its
    /// underlying grammar's own lines one `ContentRow` per `Line` with no
    /// row added or dropped.
    fn assert_rows_equal_lines(got: &[ContentRow], want: &[crate::ui::markdown::Line]) {
        assert_eq!(got.len(), want.len(), "{:?}", lines_text(got));
        for (row, line) in got.iter().zip(want.iter()) {
            assert_eq!(&row.line, line, "{:?}", lines_text(got));
        }
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
        let files = crate::changes::from_files(root, crate::changes::ArchivedScope::Full);
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
        let files2 = crate::changes::from_files(root2, crate::changes::ArchivedScope::Full);
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
                assert_eq!(tabs[0].text, " no artifacts ", "width {width}");
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

        let files = crate::changes::from_files(root, crate::changes::ArchivedScope::Full);
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

        // The exact bytes already written to each artifact file above, spelled out directly
        // rather than read back — this file names no file-reading API at all (`READSEAM`
        // and `NOIO-VIEW` both forbid it here), on exactly the terms `content_lines` itself
        // is proven against: `detail.sections` is filled by the injected reader before this
        // function ever runs.
        let sources = ["# proposal\n", "# specs\n", "# design\n"];
        for width in [78, 58] {
            for (tab, source_heading) in [(0, "proposal"), (1, "specs"), (2, "design")] {
                let d = Detail {
                    sections: vec![ArtifactSection {
                        label: String::new(),
                        text: sources[tab].to_string(),
                    }],
                    scroll: 0,
                    tab,
                    problems: Vec::new(),
                    loaded: None,
                    expanded: std::collections::BTreeSet::new(),
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

        let files = crate::changes::from_files(root, crate::changes::ArchivedScope::Full);
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
                sections: Vec::new(),
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
                expanded: std::collections::BTreeSet::new(),
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

        let files = crate::changes::from_files(root, crate::changes::ArchivedScope::Full);
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
                texts.iter().filter(|t| t.trim() == "spec").count() == 2,
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

        let files = crate::changes::from_files(root, crate::changes::ArchivedScope::Full);
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

        let files2 = crate::changes::from_files(root2, crate::changes::ArchivedScope::Full);
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
                    sections: Vec::new(),
                    scroll: 0,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
                    expanded: std::collections::BTreeSet::new(),
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
        assert!(columns(&paragraph) > 100);
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
            assert!(columns(&line.text()) <= 78);
        }
        for line in &lines58 {
            assert!(columns(&line.text()) <= 58);
        }
    }

    // --- artifact-folds: header rows, per-section bodies, and folding ----

    /// `artifact-content` :: "A foldable tab's body is headers, and an open
    /// section's markdown beneath its own".
    #[test]
    fn a_foldable_tabs_body_is_headers_and_an_open_sections_markdown_beneath_its_own() {
        for width in [78, 58] {
            let mut expanded = std::collections::BTreeSet::new();
            expanded.insert(1);
            let d = three_spec_detail(expanded);
            let rows = content_lines(&d, None, width);

            // `selected` is not this scenario's concern (`view-palette`
            // covers it) — only which section each header names.
            assert!(
                matches!(rows[0].kind, ContentKind::SectionHeader { section: 0, .. }),
                "width {width}: {:?}",
                rows[0].kind
            );
            assert!(
                matches!(rows[1].kind, ContentKind::SectionHeader { section: 1, .. }),
                "width {width}: {:?}",
                rows[1].kind
            );

            let body = crate::ui::markdown::lines(&d.sections[1].text, width);
            let body_rows = &rows[2..2 + body.len()];
            for (row, line) in body_rows.iter().zip(body.iter()) {
                assert_eq!(&row.line, line, "width {width}");
                assert_eq!(row.kind, ContentKind::Body, "width {width}");
            }

            let after = &rows[2 + body.len()];
            assert!(
                matches!(after.kind, ContentKind::SectionHeader { section: 2, .. }),
                "width {width}: {:?}",
                after.kind
            );

            // With `expanded` empty the returned list is exactly three
            // header rows.
            let collapsed = three_spec_detail(std::collections::BTreeSet::new());
            let rows2 = content_lines(&collapsed, None, width);
            assert_eq!(rows2.len(), 3, "width {width}");
            assert!(
                rows2
                    .iter()
                    .all(|r| matches!(r.kind, ContentKind::SectionHeader { .. })),
                "width {width}: {:?}",
                rows2.iter().map(|r| r.kind).collect::<Vec<_>>()
            );

            // No returned line exceeds `width`.
            for row in &rows {
                assert!(columns(&row.text()) <= width as usize, "width {width}");
            }
        }
    }

    /// `artifact-content` :: "A non-foldable tab is byte-identical to
    /// today".
    #[test]
    fn a_non_foldable_tab_is_byte_identical_to_today() {
        let source: String = (0..20).map(|i| format!("- line-{i:02}\n")).collect();
        for width in [78, 58] {
            let d = detail(&source, Vec::new());
            let rows = content_lines(&d, None, width);
            let want = crate::ui::markdown::lines(&d.sections[0].text, width);
            assert_eq!(rows.len(), want.len(), "width {width}");
            for (row, line) in rows.iter().zip(want.iter()) {
                assert_eq!(&row.line, line, "width {width}");
                assert_eq!(row.kind, ContentKind::Body, "width {width}");
            }
            assert!(
                !rows
                    .iter()
                    .any(|r| matches!(r.kind, ContentKind::SectionHeader { .. })),
                "width {width}: a header row was prepended"
            );
        }
    }

    /// `artifact-content` :: "The tracked-tasks tab concatenates rather
    /// than folding".
    #[test]
    fn the_tracked_tasks_tab_concatenates_rather_than_folding() {
        let progress = Progress {
            completed: 1,
            total: 2,
        };
        let change =
            fixture::with_marked_artifacts(&paths_free(&["proposal", "tasks"]), Some(1), progress);
        for width in [78, 58] {
            let d = Detail {
                sections: vec![
                    ArtifactSection {
                        label: String::new(),
                        text: "## 1. Setup\n- [x] a\n".to_string(),
                    },
                    ArtifactSection {
                        label: String::new(),
                        text: "## 2. Build\n- [ ] b\n".to_string(),
                    },
                ],
                scroll: 0,
                tab: 1,
                problems: Vec::new(),
                loaded: None,
                expanded: std::collections::BTreeSet::new(),
            };
            let rows = content_lines(&d, Some(&change), width);
            let want = crate::ui::tasks::lines(
                "## 1. Setup\n- [x] a\n## 2. Build\n- [ ] b\n",
                &progress,
                width,
            );
            assert_eq!(rows.len(), want.len(), "width {width}");
            for (row, line) in rows.iter().zip(want.iter()) {
                assert_eq!(&row.line, line, "width {width}");
            }
            assert!(
                !rows
                    .iter()
                    .any(|r| matches!(r.kind, ContentKind::SectionHeader { .. })),
                "width {width}: a header row must never appear on the tracked-tasks tab, \
                 even with two sections"
            );
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
                sections: vec![ArtifactSection {
                    label: String::new(),
                    text: source.to_string(),
                }],
                scroll: 0,
                tab: 1,
                problems: Vec::new(),
                loaded: None,
                expanded: std::collections::BTreeSet::new(),
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
            assert_rows_equal_lines(&lines, &want);
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
                sections: vec![ArtifactSection {
                    label: String::new(),
                    text: source.to_string(),
                }],
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
                expanded: std::collections::BTreeSet::new(),
            };
            let lines = content_lines(&d, Some(&change), width);
            let want = crate::ui::markdown::lines(source, width);
            assert_rows_equal_lines(&lines, &want);
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
                sections: vec![ArtifactSection {
                    label: String::new(),
                    text: source.to_string(),
                }],
                scroll: 0,
                tab: 7,
                problems: Vec::new(),
                loaded: None,
                expanded: std::collections::BTreeSet::new(),
            };
            let lines = content_lines(&d, Some(&change), width);
            assert_rows_equal_lines(&lines, &crate::ui::markdown::lines(source, width));

            // The same holds for a change carrying no artifacts at all.
            let empty_artifacts = fixture::with_marked_artifacts(&paths_free(&[]), None, progress);
            let lines_empty = content_lines(&d, Some(&empty_artifacts), width);
            assert_rows_equal_lines(&lines_empty, &crate::ui::markdown::lines(source, width));
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
        // A 200-column CJK paragraph — `artifact-content`'s own seventh `Detail` value for
        // this scenario, distinct from the 200-character ASCII one above: `columns` of this
        // source is roughly twice its `chars().count()`, which is exactly the gap a
        // `chars()`-based wrap would get wrong.
        let cjk_paragraph = format!("{}\n", "日本語".repeat(34));
        assert!(columns(&cjk_paragraph) >= 200);
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
                let mut d = detail(&cjk_paragraph, Vec::new());
                d.tab = 0;
                d
            },
            {
                let mut d = detail("- [ ] a\n", Vec::new());
                d.tab = 9;
                d
            },
            // `artifact-folds`' own eighth and ninth `Detail` values: three
            // sections, every one collapsed, and three sections with
            // `expanded` holding `0`, `1`, `2`, and `7` — the last one past
            // the end, which `content_lines` must fold shut rather than
            // panic on.
            three_spec_detail(std::collections::BTreeSet::new()),
            three_spec_detail(std::collections::BTreeSet::from([0, 1, 2, 7])),
        ];
        for width in [78, 58] {
            for change in [None, Some(&marked), Some(&unmarked), Some(&no_artifacts)] {
                for d in &details {
                    let lines = content_lines(d, change, width);
                    for line in &lines {
                        assert!(
                            columns(&line.text()) <= width as usize,
                            "width {width}, source {:?}: {:?} exceeds its width",
                            d.sections,
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

        // The CJK paragraph (details[5]) wraps by columns too, so it also produces
        // strictly more lines at 58 than at 78 — a `chars()`-based wrap would pack
        // roughly twice as many CJK characters per line as the region can hold and
        // would not necessarily show this gap the same way.
        let cjk_at_78 = content_lines(&details[5], None, 78).len();
        let cjk_at_58 = content_lines(&details[5], None, 58).len();
        assert!(
            cjk_at_58 > cjk_at_78,
            "CJK: 58: {cjk_at_58}, 78: {cjk_at_78} — the width must genuinely reach the wrap"
        );
    }

    /// `artifact-content` :: total function, no panic — `change: None`, an empty `Detail`,
    /// and width `0`, per task 5.3. Also driven at 78 and 58 — `No content yet` is now
    /// padded like every neighbouring line (`view-fidelity`'s repair): at width `0` there is
    /// no room even for the literal, so the line is empty; at 78 and 58 it is padded with
    /// trailing spaces out to the full width.
    #[test]
    fn content_lines_never_panics_with_no_change_an_empty_detail_and_zero_width() {
        let d = detail("", Vec::new());
        let lines = content_lines(&d, None, 0);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text(), "");
        for width in [78, 58] {
            let lines = content_lines(&d, None, width);
            assert_eq!(lines.len(), 1, "width {width}");
            let want = format!("No content yet{}", " ".repeat(width as usize - 14));
            assert_eq!(lines[0].text(), want, "width {width}");
        }
    }

    // --- view-fidelity group 5: the header and content measure in columns ---------------

    /// `detail-header` :: "A CJK change name keeps the header inside its region at both
    /// mandated widths" — a ten-character, twenty-display-column name.
    #[test]
    fn a_cjk_change_name_keeps_the_header_inside_its_region_at_both_mandated_widths() {
        let name = "日本語の変更名前です";
        let progress = Progress {
            completed: 4,
            total: 9,
        };
        for width in [78, 58] {
            let got = header_row(name, "tdd", &progress, width);
            assert_eq!(columns(&got), width as usize, "width {width}: {got:?}");
            assert!(got.ends_with("[4/9]"), "width {width}: {got:?}");
            assert!(got.contains("(tdd)"), "width {width}: {got:?}");
            // Discriminating: proves the padding was computed in columns rather than in
            // characters — a `chars().count()`-based budget would have produced a header
            // whose `chars().count()` also equalled `width`, dropping the schema and
            // progress cells off the row instead.
            assert!(
                got.chars().count() < columns(&got),
                "width {width}: {got:?} was not measured in columns"
            );
        }
    }

    /// `detail-header` :: "The header reaches the buffer without crossing the region
    /// border". The tail `" (tdd) [4/9]"` is checked by exact column-indexed slicing (it is
    /// pure ASCII, one buffer cell per character); the CJK name field itself is not
    /// reconstructed by slicing `row_text`'s per-column output, because a two-column
    /// grapheme cluster's own trailing cell is reset to a single blank space, which would
    /// otherwise be misread as a character the name never had.
    #[test]
    fn the_header_reaches_the_buffer_without_crossing_the_region_border() {
        let name = "日本語の変更名前です";
        let tail = " (tdd) [4/9]";
        let change = fixture::active(name, 4, 9);
        let d = dashboard_at_detail(change, detail("", Vec::new()));

        // (frame width, the detail region's own interior width, the interior's first
        // column) — the two mandated pairs, named as bare literals for `DETAILWIDTHS`.
        // The header is now drawn into the region's own heading row (buffer row 0,
        // `pane-chrome` -> "The header row is drawn into the detail region's first
        // interior row"), not an interior row two rows below it.
        let cases: [(u16, usize, usize); 2] = [(120, 78, 42), (60, 58, 1)];
        for (frame, interior, first_col) in cases {
            let buf = render_at(frame, 20, &d);
            let tail_start = first_col + interior - tail.len();
            assert_eq!(
                cols(&row_text(&buf, 0), tail_start..tail_start + tail.len()),
                tail,
                "frame {frame}"
            );
            let last_col = (first_col + interior - 1) as u16;
            assert_eq!(cell(&buf, last_col, 0).symbol(), "]", "frame {frame}");
        }

        // At 60 columns the narrow region takes `Gutters::Both`: the frame's own last
        // column is the region's right gutter, always a space.
        let buf60 = render_at(60, 20, &d);
        assert_eq!(
            cell(&buf60, 59, 0).symbol(),
            " ",
            "the narrow region's right gutter must stay blank"
        );

        // At 120 columns, unchanged from the same render with an ASCII name: columns
        // 39 and 41 are the blank columns either side of the divider, and column 40
        // holds it — the wide detail region has no right gutter of its own, so its
        // content reaches the frame's own last column instead.
        let buf120 = render_at(120, 20, &d);
        assert_eq!(
            cell(&buf120, 39, 0).symbol(),
            " ",
            "list region's right gutter"
        );
        assert_eq!(cell(&buf120, 40, 0).symbol(), "│", "the divider");
        assert_eq!(
            cell(&buf120, 41, 0).symbol(),
            " ",
            "detail region's left gutter"
        );
    }

    /// `detail-header` :: "The header is total over adversarial names at every width".
    #[test]
    fn header_row_is_total_over_adversarial_names_at_every_width() {
        let family_emoji = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}";
        let names = [
            "日".repeat(200),
            family_emoji.to_string(),
            "e\u{0301}\u{0301}\u{0301}\u{0301}\u{0301}".to_string(),
            "a\u{0}b".to_string(),
            String::new(),
        ];
        let progress = Progress {
            completed: 4,
            total: 9,
        };
        for name in &names {
            for width in 0u16..=130 {
                let got = header_row(name, "tdd", &progress, width);
                if width == 0 {
                    assert_eq!(got, "", "name {name:?} width {width}");
                } else {
                    assert_eq!(
                        columns(&got),
                        width as usize,
                        "name {name:?} width {width}: {got:?}"
                    );
                }
                match width {
                    78 | 58 | 13 => {
                        assert!(
                            got.contains("(tdd)"),
                            "name {name:?} width {width}: {got:?}"
                        );
                        assert!(
                            got.contains("[4/9]"),
                            "name {name:?} width {width}: {got:?}"
                        );
                    }
                    12 | 7 => {
                        assert!(
                            !got.contains("(tdd"),
                            "name {name:?} width {width}: {got:?}"
                        );
                        assert!(
                            got.contains("[4/9]"),
                            "name {name:?} width {width}: {got:?}"
                        );
                    }
                    6 | 1 => {
                        assert!(
                            !got.contains("(tdd"),
                            "name {name:?} width {width}: {got:?}"
                        );
                        assert!(!got.contains("[4/"), "name {name:?} width {width}: {got:?}");
                    }
                    _ => {}
                }
            }
            // The mandated pair, asserted explicitly by this sweep too.
            for width in [78, 58] {
                let got = header_row(name, "tdd", &progress, width);
                assert_eq!(columns(&got), width as usize, "name {name:?} width {width}");
            }
        }
    }

    /// `artifact-content` :: "`No content yet` does not eat the border at a narrow frame".
    #[test]
    fn no_content_yet_does_not_eat_the_border_at_a_narrow_frame() {
        let change = fixture::with_artifacts(fixture::active("x", 0, 0), &[("proposal", &[])]);
        let d = dashboard_at_detail(change, detail("", Vec::new()));

        let cases: [(u16, usize, &str); 3] = [
            (15, 13, "No content y…"),
            (14, 12, "No content …"),
            (13, 11, "No content…"),
        ];
        // The content area's first row is buffer row 5 now, not 4: the region's
        // heading row (`pane-chrome`'s change header) and its blank padding row sit
        // two rows above the interior, and the tab bar, the rule, and the interior's
        // own padding row take three more before the content itself starts.
        for (frame, area_width, want) in cases {
            let buf = render_at(frame, 20, &d);
            let row = cols(&row_text(&buf, 5), 1..1 + area_width);
            assert_eq!(row, want, "frame {frame}");
            assert_eq!(columns(&row), area_width, "frame {frame}");
            assert_eq!(
                cell(&buf, frame - 1, 5).symbol(),
                " ",
                "frame {frame}: the region's right gutter was overwritten"
            );
        }

        // The mandated pair: the literal fits whole and is padded to the full interior.
        let mandated: [(u16, usize); 2] = [(120, 78), (60, 58)];
        for (frame, width) in mandated {
            let buf = render_at(frame, 20, &d);
            let content_x = if frame == 120 { 42 } else { 1 };
            let row = cols(&row_text(&buf, 5), content_x..content_x + width);
            let want = format!("No content yet{}", " ".repeat(width - 14));
            assert_eq!(row, want, "width {width}");
        }

        // At 2x20 and 1x20 the content area collapses to zero columns: no panic, and the
        // literal — which cannot fit — is drawn nowhere.
        for frame in [2u16, 1] {
            let buf = render_at(frame, 20, &d);
            assert!(
                !row_text(&buf, 5).contains("No content"),
                "frame {frame}: a zero-width content area must draw nothing"
            );
        }
    }

    /// `artifact-content` :: "A wide-character document stays inside the detail region".
    #[test]
    fn a_wide_character_document_stays_inside_the_detail_region() {
        let family_emoji = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}";
        let paragraph = "日本語".repeat(40);
        let source = format!("{paragraph}\n\n# 🎉 見出し\n\n- {family_emoji} family\n");
        let change = fixture::with_artifacts(fixture::active("x", 0, 0), &[("proposal", &[])]);
        let d = dashboard_at_detail(change, detail(&source, Vec::new()));

        // There is no border any more: at the narrow layout the region's two gutter
        // columns stay blank at every body row, unchanged from an ASCII document.
        let buf60 = render_at(60, 20, &d);
        for y in 0..=18u16 {
            assert_eq!(cell(&buf60, 0, y).symbol(), " ", "y={y}");
            assert_eq!(cell(&buf60, 59, y).symbol(), " ", "y={y}");
        }

        // At the wide layout columns 39 and 41 — the blank columns either side of the
        // divider — stay spaces and column 40 holds the divider `│`, at every body row.
        let buf120 = render_at(120, 20, &d);
        for y in 0..=18u16 {
            assert_eq!(cell(&buf120, 39, y).symbol(), " ", "y={y}");
            assert_eq!(cell(&buf120, 40, y).symbol(), "│", "y={y}");
            assert_eq!(cell(&buf120, 41, y).symbol(), " ", "y={y}");
        }

        let mandated: [u16; 2] = [78, 58];
        for width in mandated {
            let lines = content_lines(&d.detail, d.selected_change(), width);
            for line in &lines {
                assert!(
                    columns(&line.text()) <= width as usize,
                    "width {width}: {:?} exceeds its width",
                    line.text()
                );
            }
        }

        // `foldable-spec-sections`: the same holds for a foldable artifact of three
        // sections whose labels are CJK, with one section open, so a header row is
        // measured in columns like every other row.
        let cjk_change = fixture::with_artifacts(fixture::active("x", 0, 0), &[("proposal", &[])]);
        let cjk_detail = Detail {
            sections: vec![
                ArtifactSection {
                    label: "日本語ラベル".to_string(),
                    text: "one\n".to_string(),
                },
                ArtifactSection {
                    label: "見出し二番目".to_string(),
                    text: paragraph.clone(),
                },
                ArtifactSection {
                    label: "タスク一覧".to_string(),
                    text: "three\n".to_string(),
                },
            ],
            scroll: 0,
            tab: 0,
            problems: Vec::new(),
            loaded: None,
            expanded: std::collections::BTreeSet::from([1]),
        };
        let cjk_dashboard = dashboard_at_detail(cjk_change, cjk_detail);
        for width in mandated {
            let lines = content_lines(
                &cjk_dashboard.detail,
                cjk_dashboard.selected_change(),
                width,
            );
            for line in &lines {
                assert!(
                    columns(&line.text()) <= width as usize,
                    "width {width}: {:?} exceeds its width",
                    line.text()
                );
            }
        }
    }

    /// `artifact-content` :: "No `content_lines` line exceeds its width at any width" —
    /// a sweep of `0..=130` over seven `Detail` values (the six `content_lines_total`
    /// already carries, plus a 200-column CJK paragraph) crossed with the four `change`
    /// shapes, and the `No content yet` case for `width` `0` through `13`.
    #[test]
    fn no_content_lines_line_exceeds_its_width_at_any_width() {
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
        let cjk_paragraph = format!("{}\n", "日本語".repeat(70));
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
                let mut d = detail(&cjk_paragraph, Vec::new());
                d.tab = 0;
                d
            },
            {
                let mut d = detail("- [ ] a\n", Vec::new());
                d.tab = 9;
                d
            },
            // `artifact-folds`' own eighth and ninth `Detail` values, per
            // `content_lines_total` above: three collapsed sections, and
            // three sections with `expanded` holding `0`, `1`, `2`, and `7`
            // — the last past the end, folded shut rather than panicking.
            three_spec_detail(std::collections::BTreeSet::new()),
            three_spec_detail(std::collections::BTreeSet::from([0, 1, 2, 7])),
        ];

        // No clock read here: `src/ui/` — tests included — names no clock API (`NOBLOCK`).
        // Runtime is a verification-run concern, not an in-test assertion: this sweep (131
        // widths x 9 `Detail` x 4 `change`, the widest matrix in the plan) measured well
        // under a second — `cargo test --lib ui::detail::tests::no_content_lines_line_exceeds_its_width_at_any_width`
        // alone reported "finished in 0.61s" — so it is not narrowed by input.
        for width in 0u16..=130 {
            for change in [None, Some(&marked), Some(&unmarked), Some(&no_artifacts)] {
                for d in &details {
                    let lines = content_lines(d, change, width);
                    for line in &lines {
                        assert!(
                            columns(&line.text()) <= width as usize,
                            "width {width}, source {:?}: {:?} exceeds its width",
                            d.sections,
                            line.text()
                        );
                    }
                }
            }
        }

        // The mandated pair, asserted explicitly by this sweep too.
        let mandated: [u16; 2] = [78, 58];
        for width in mandated {
            for change in [None, Some(&marked), Some(&unmarked), Some(&no_artifacts)] {
                for d in &details {
                    let lines = content_lines(d, change, width);
                    for line in &lines {
                        assert!(columns(&line.text()) <= width as usize, "width {width}");
                    }
                }
            }
        }

        // The `No content yet` case at every width in the range where the literal is
        // longer than the region — the range no mandated-width test can reach.
        let empty = detail("", Vec::new());
        for width in 0u16..=13 {
            let lines = content_lines(&empty, None, width);
            for line in &lines {
                assert!(columns(&line.text()) <= width as usize, "width {width}");
            }
        }
    }
}
