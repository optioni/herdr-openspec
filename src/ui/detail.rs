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

use crate::ui::layout::{columns, truncate_columns};

/// The gauge cell's fixed budget: the number of `█`/`░` display columns
/// `header_row` reserves for `ui::tasks::gauge_of`, wherever it draws one at
/// all. A named module-local constant beside the grammar that spends it, on
/// the same terms every other measurement in this file is a fact about the
/// row rather than about the frame. See design.md -> Decision 4: twelve
/// columns leaves a 32-column name field at the 58-column interior, wider
/// than the longest change name this repository has.
const HEADER_GAUGE_COLUMNS: u16 = 12;

/// The change header's fixed-field grammar. When `progress.total > 0` it is
/// `[name field][space][schema cell][space][gauge cell][space][progress
/// cell]`, exactly `width` display columns; when `progress.total == 0` it
/// is `[name field][space][schema cell][space][progress cell]` — no gauge
/// cell and no separating space at all, byte-identically to what this
/// function produced before the gauge cell existed (design.md -> Decision
/// 7: a gauge with no denominator would have to invent a fill).
///
/// The progress cell is `ui::list::progress_cell` and the gauge cell is
/// `ui::tasks::gauge_of` — one implementation of each, so the header can
/// never disagree with a list row about a change's progress, or with the
/// tracked-tasks tab's own bar about how full a change is. A cell too wide
/// for the row is dropped **whole**, in this order: the gauge cell first
/// (design.md -> Decision 2 — this keeps the schema and progress cells at
/// exactly the positions they held before the gauge was added, so every
/// band below the full form is unchanged), then the schema cell, then the
/// progress cell, leaving the name field alone; the name is truncated only
/// after all cells that apply have been dropped.
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

    // Full form: name + space + schema cell + space + gauge cell + space +
    // progress cell. Skipped entirely at `total == 0`, so that row falls
    // through to the three pre-gauge bands below and is byte-identical to
    // what this function produced before the gauge existed — rather than
    // reserving the gauge's columns and then declining to draw it.
    if progress.total > 0 {
        let gauge_len = i64::from(HEADER_GAUGE_COLUMNS);
        let name_field_full = w - 3 - schema_len - gauge_len - progress_len;
        if name_field_full >= 1 {
            let name_field = crate::ui::list::pad_or_truncate_right(name, name_field_full as usize);
            let gauge_cell = crate::ui::tasks::gauge_of(progress, HEADER_GAUGE_COLUMNS);
            return format!("{name_field} {schema_cell} {gauge_cell} {progress_cell}");
        }
    }

    // Gauge dropped, with its separating space: name + space + schema + space
    // + progress. This and the two bands below it are the three the header had
    // before the gauge cell, at the same boundaries and producing the same
    // strings, which is what places the gauge first in the drop order.
    let name_field_no_gauge = w - 2 - schema_len - progress_len;
    if name_field_no_gauge >= 1 {
        let name_field = crate::ui::list::pad_or_truncate_right(name, name_field_no_gauge as usize);
        return format!("{name_field} {schema_cell} {progress_cell}");
    }

    // Drop the schema cell too: name + space + progress.
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

/// A foldable tab's own section-header row: `<indent><glyph> <label>`, where
/// `indent` is two spaces per unit of `depth` and the glyph is
/// `ui::list::fold_glyph`'s own — the crate's one site for that pair, so one
/// fold reads the same glyph in both regions (design.md -> Decision 9).
/// Passed through the crate's one right-truncation grammar,
/// `ui::list::pad_or_truncate_right`, so the indent, the glyph and its
/// separating space — emitted first — survive any truncation the label needs
/// and the row still measures at most `width` display columns. Below the
/// indent's own columns the row degrades to truncated indent rather than to a
/// dropped glyph (`heading-sections` -> design.md -> D7: the indent precedes
/// the glyph precisely so truncation eats the label first).
///
/// Body rows carry no indent at all: indenting them would take the narrow
/// interior's text column away from exactly the artifacts this change exists
/// to make readable.
///
/// `tasks-emphasis` adds the right-aligned **progress cell** a tracked-tasks
/// group header carries. It is `ui::list::progress_cell` — the crate's one
/// progress cell, on exactly the terms `tasks-progress-bar`'s bar and
/// `detail-header`'s header row already reach it — and is **dropped whole**
/// when the row cannot hold the indent, the glyph, its separating space, at
/// least one column of label, one further separating space, and the cell
/// itself. Never truncated, and never allowed to push the label out.
///
/// `spec-emphasis` adds the two-column **badge** a requirement section's
/// `operation` carries: `+`/`~`/`-` followed by one space, emitted with the
/// indent and the glyph, before the label (design.md -> Decision 5). The
/// drop-whole order as the row narrows is therefore the cell first,
/// reclaiming its own padding; then the label, truncated with the `…` rule;
/// then the badge; then the glyph and the indent — the same discipline the
/// bar and the change header already use, so a narrowing pane loses fields
/// in one order everywhere.
///
/// Returns the row's own `Line` rather than a bare `String`: a badged row
/// carries **four** segments — the `<indent><glyph> ` prefix, the badge,
/// the label, and the blank columns that pad the row to its width — so the
/// badge and (on a `Removed` requirement) the label can be faced apart
/// while the padding stays plain, and a strike ends with the word rather
/// than running to the region's edge. An unbadged row still carries the
/// single plain segment it always did, plus the progress cell's own
/// segment when it draws one (design.md -> Decision 8, as amended in
/// tasks.md 6.7). `ui::detail`
/// names no `palette::Role` here: the badge segment carries
/// `Face { delta: Some(op), .. }` and nothing else, exactly as the row
/// carries a *kind* and not a style. Every test that cares only about the
/// row's own text reads it through [`crate::ui::markdown::Line::text`].
fn header(
    label: &str,
    depth: usize,
    expanded: bool,
    progress: Option<&crate::tasks::Progress>,
    operation: Option<crate::specs::DeltaOp>,
    width: u16,
) -> crate::ui::markdown::Line {
    let (area, cell) = label_area(depth, progress, operation, width as usize);
    let mut segments = match badged_pieces(label, depth, expanded, operation, area) {
        Some((prefix, badge, label, pad)) => vec![
            crate::ui::markdown::Segment {
                text: prefix,
                face: crate::ui::markdown::Face::plain(),
            },
            crate::ui::markdown::Segment {
                text: badge,
                face: crate::ui::markdown::Face {
                    delta: operation,
                    ..crate::ui::markdown::Face::plain()
                },
            },
            crate::ui::markdown::Segment {
                text: label,
                face: crate::ui::markdown::Face {
                    strikethrough: matches!(operation, Some(crate::specs::DeltaOp::Removed)),
                    ..crate::ui::markdown::Face::plain()
                },
            },
            // The blank columns after the label carry no modifier: a struck
            // heading must read as a struck word, not as a rule drawn across
            // the region.
            crate::ui::markdown::Segment {
                text: pad,
                face: crate::ui::markdown::Face::plain(),
            },
        ],
        None => vec![crate::ui::markdown::Segment {
            text: unbadged_row(label, depth, expanded, area),
            face: crate::ui::markdown::Face::plain(),
        }],
    };
    if let Some(cell) = cell {
        segments.push(crate::ui::markdown::Segment {
            text: format!(" {cell}"),
            face: crate::ui::markdown::Face::plain(),
        });
    }
    crate::ui::markdown::Line { segments }
}

/// The width left for the badge/label portion of a header row once a
/// progress cell has claimed its own, plus the cell's own text when it is
/// drawn — `None` when `progress` is absent or the row cannot hold the
/// cell. Factored out of `header` so its cell arithmetic answers this
/// question exactly once, whether or not a badge is also drawn.
///
/// `operation` is what makes that last clause true rather than merely
/// stated. The cell yields **before** the badge does
/// (specs/artifact-folds -> "A section header row names the file and shows
/// its fold state": the progress cell first, then the label, then the
/// badge), so a row that will draw a badge must reserve the badge's
/// columns here — otherwise the cell is kept at a width where
/// `badged_pieces` then refuses, the cell outlives the badge, and widening
/// the row by one column makes the badge disappear and reappear.
fn label_area(
    depth: usize,
    progress: Option<&crate::tasks::Progress>,
    operation: Option<crate::specs::DeltaOp>,
    width: usize,
) -> (usize, Option<String>) {
    let Some(progress) = progress else {
        return (width, None);
    };
    let cell = crate::ui::list::progress_cell(progress);
    let cell_cols = columns(&cell);
    let indent_cols = columns(&"  ".repeat(depth));
    let badge_cols = operation.map_or(0, |op| columns(badge_text(op)));
    // The indent, the glyph, its space, the badge when one is drawn, one
    // column of label, one separating space, and the cell.
    let minimum = indent_cols + 3 + badge_cols + 1 + cell_cols;
    if width >= minimum {
        (width - cell_cols - 1, Some(cell))
    } else {
        (width, None)
    }
}

/// `Some((prefix, badge, label, pad))` when a badge is drawn at `width` —
/// `operation` is `Some` and there is room for the indent, the glyph, its
/// space, the badge, and at least one column of label — split at the seams
/// `header` faces apart into four segments, the last being the blank
/// columns that fill the row, plain-faced so a `Removed` label's strike
/// ends with the word. `None` when no badge is
/// drawn: `operation` is `None`, or the row is too narrow to hold one, in
/// which case the caller falls back to `unbadged_row`, truncating the
/// indent, the glyph and the label as one unit exactly as it always has.
fn badged_pieces(
    label: &str,
    depth: usize,
    expanded: bool,
    operation: Option<crate::specs::DeltaOp>,
    width: usize,
) -> Option<(String, String, String, String)> {
    let op = operation?;
    let glyph = crate::ui::list::fold_glyph(!expanded);
    let indent = "  ".repeat(depth);
    let prefix = format!("{indent}{glyph} ");
    let badge = badge_text(op).to_string();
    let minimum = columns(&prefix) + columns(&badge) + 1;
    if width < minimum {
        return None;
    }
    let label_width = width - columns(&prefix) - columns(&badge);
    let (label, pad) = crate::ui::list::truncate_right(label, label_width);
    Some((prefix, badge, label, pad))
}

/// The row this capability drew before it existed: `<indent><glyph>
/// <label>`, truncated as one unit through the crate's one truncation
/// rule, so a depth whose indent alone exceeds `width` still degrades to
/// truncated indent rather than a dropped glyph.
fn unbadged_row(label: &str, depth: usize, expanded: bool, width: usize) -> String {
    let glyph = crate::ui::list::fold_glyph(!expanded);
    let indent = "  ".repeat(depth);
    crate::ui::list::pad_or_truncate_right(&format!("{indent}{glyph} {label}"), width)
}

/// The badge's own two columns for a delta operation: the marker — `+` for
/// `Added`, `~` for `Modified`, `-` for `Removed` — and the one separating
/// space every badge carries (specs/artifact-folds -> "A section header row
/// names the file and shows its fold state").
fn badge_text(op: crate::specs::DeltaOp) -> &'static str {
    match op {
        crate::specs::DeltaOp::Added => "+ ",
        crate::specs::DeltaOp::Modified => "~ ",
        crate::specs::DeltaOp::Removed => "- ",
    }
}

/// The blank separator row, padded to `width` on the same terms every other
/// row this function adds is. `Face::plain()`, `ContentKind::Body`: it is not
/// a header, and the cursor never emphasises it.
fn separator_row(width: u16) -> ContentRow {
    ContentRow {
        line: crate::ui::markdown::Line {
            segments: vec![crate::ui::markdown::Segment {
                text: crate::ui::list::pad_or_truncate_right("", width as usize),
                face: crate::ui::markdown::Face::plain(),
            }],
        },
        kind: ContentKind::Body,
    }
}

/// The display columns of interior a body row keeps when the deepest body on
/// the tab is indented. A measured trade, not a derivation: the archive's
/// deepest spec section is depth 3, costing six columns, which leaves 72 at
/// the 78-column wide interior and 52 at the 58-column narrow one, and 64 is
/// placed between the two — so the wide layout gains the alignment and the
/// narrow layout keeps the text column the previous rule protected
/// (specs/artifact-folds -> "A section header row names the file and shows
/// its fold state"; design.md -> Decision 3 carries the derivation table).
const BODY_INDENT_FLOOR: u16 = 64;

/// The columns a body at `depth` is indented by: two per unit of depth, the
/// same unit `unbadged_row` gives a header row, written once here so the
/// header's indent and the body's cannot drift apart.
fn indent_columns(depth: usize) -> u16 {
    u16::try_from(2 * depth).unwrap_or(u16::MAX)
}

/// Whether this `content_lines` call indents body rows at all — one decision
/// per render rather than one per section, so a tab's left edge is either
/// consistently aligned or consistently flush and never a mixture.
///
/// `max_depth` is the greatest `depth` among the sections whose own `text` is
/// non-empty, read from `detail.sections` and never from the visible or
/// expanded set: a tab's text column must not widen as a deep section is
/// collapsed and narrow again as it is opened, which would make `Space`
/// reflow the prose of every sibling that stayed open.
///
/// The subtraction saturates because `width` is a `u16` and this capability's
/// width sweeps run from `0`, reaching every width below `2 * max_depth`; an
/// unguarded `-` panics there in the debug build `cargo test` uses. It is
/// also what makes the *other* subtraction safe — an indented body's own
/// `width - indent_columns(depth)` is at least `BODY_INDENT_FLOOR` and
/// therefore never zero.
fn bodies_are_indented(sections: &[crate::ui::app::ArtifactSection], width: u16) -> bool {
    let max_depth = sections
        .iter()
        .filter(|s| !s.text.is_empty())
        .map(|s| s.depth)
        .max()
        .unwrap_or(0);
    width.saturating_sub(indent_columns(max_depth)) >= BODY_INDENT_FLOOR
}

/// `line` with `indent` spaces prepended as a plain-faced segment of its own,
/// or `line` itself when the indent is empty — so a tab drawn at column zero
/// produces exactly the segments it produced before this rule existed.
fn indented_line(indent: &str, line: crate::ui::markdown::Line) -> crate::ui::markdown::Line {
    if indent.is_empty() {
        return line;
    }
    let mut segments = vec![crate::ui::markdown::Segment {
        text: indent.to_string(),
        face: crate::ui::markdown::Face::plain(),
    }];
    segments.extend(line.segments);
    crate::ui::markdown::Line { segments }
}

/// The indices of `sections` that are **visible**, in order: walking the list
/// once, a **collapsed labelled** section at depth `d` hides every following
/// section of depth strictly greater than `d` — header and body alike — until
/// the first section of depth at or below `d`.
///
/// A `None`-labelled section is a split file's preamble: it is always open,
/// owns no header row, and is never a fold target, so it hides nothing
/// (design.md -> D2). Indentation is therefore never the only signal of
/// nesting — a fold hides a whole subtree, which is what makes a two-level
/// spec tab navigable at the 58-column interior.
///
/// Extracted rather than walked inline because the emission needs to look
/// **ahead**: a blank separator follows a non-empty open body only when a
/// further visible section follows it, which is a question about the visible
/// list rather than about the next index.
fn visible_sections(
    sections: &[crate::ui::app::ArtifactSection],
    expanded: &std::collections::BTreeSet<usize>,
) -> Vec<usize> {
    let mut visible = Vec::new();
    let mut hidden_below: Option<usize> = None;
    for (index, section) in sections.iter().enumerate() {
        if let Some(d) = hidden_below {
            if section.depth > d {
                continue;
            }
            hidden_below = None;
        }
        visible.push(index);
        if section.label.is_some() && !expanded.contains(&index) {
            hidden_below = Some(section.depth);
        }
    }
    visible
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
/// The body is dispatched on `Detail::foldable` — derived rather than
/// stored (`artifact-folds` -> Decision 3) — and not on the artifact's
/// kind. At a **foldable** tab it is `ui::tasks::bar_lines` first — given
/// `change.progress` and a group slice built from `detail.sections`' own
/// `progress` values in section order, skipping the sections carrying `None`,
/// which is what `tasks-progress-bar` segments the gauge by; the **empty**
/// slice is what a test call site passes and would leave the gauge unsegmented
/// in every frame — when `change` is `Some` and the `ArtifactRef` at
/// `detail.tab` carries `tracks_tasks == true`, as leading body owned by no
/// section and hidden by no fold; then `artifact-folds`' walk — a header
/// row per visible labelled section, that section's own rendered body
/// beneath it exactly when it is open, and a blank separator after a
/// non-empty open body a further visible section follows. An open section's
/// body is `ui::tasks::items` over its parsed items on a tracked-tasks tab
/// and `ui::markdown::lines(&section.text, width)` otherwise. The
/// tracked-tasks tab folding at all **reverses** `artifact-folds` ->
/// Decision 8, which held that tab never foldable at any section count
/// because its whole-change progress bar would disagree with a per-section
/// fold: the bar now leads the body above every header, so it and a fold
/// answer different questions (`heading-sections` -> design.md -> D8).
///
/// At a **non-foldable** tab the body is taken over the concatenation of
/// every section's text: `ui::tasks::lines(&text, &change.progress, width)`
/// — `tasks-checklist`'s grammar and `tasks-progress-bar`'s leading line —
/// on a tracked-tasks tab, and `ui::markdown::lines(&text, width)` in every
/// other case — a single section, no section at all, a `None` change, a
/// `detail.tab` past the end of the artifact list, and a change carrying no
/// artifacts at all.
///
/// Each header row carries `ContentKind::SectionHeader {
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

    if detail.foldable() {
        // `heading-sections` -> design.md -> D8: the progress bar leads the
        // whole body, above every header, as leading body owned by no
        // section and hidden by no fold. A bar that counts the change and a
        // fold that hides a group answer different questions, which is what
        // retires `artifact-folds` Decision 8's objection to a foldable
        // tracked-tasks tab.
        if let Some(progress) = tracked_tasks_progress {
            // `tasks-emphasis`: the gauge's group slice comes from
            // `detail.sections`' own `progress` values, in section order,
            // skipping the sections carrying `None` — the number is already
            // computed once per sync, and re-parsing the file here would be a
            // second derivation that can disagree with the header cells drawn
            // beside it.
            //
            // Skipping `None` has a stated consequence: `artifact-folds` sets
            // `progress` on heading sections only, so a split file's preamble
            // contributes no span even when it holds items. Those items are
            // still counted by the bar's own `progress`, which is the
            // `Change`'s field, so the gauge's fill is unaffected; only the
            // boundary marking omits them. A preamble holding task items is
            // not a shape any workflow's `tasks.md` produces, and the
            // alternative — a span with no header row to match it — would mark
            // a boundary the reader cannot see.
            let groups: Vec<crate::tasks::Progress> =
                detail.sections.iter().filter_map(|s| s.progress).collect();
            out.extend(
                crate::ui::tasks::bar_lines(progress, &groups, width)
                    .into_iter()
                    .map(body_row),
            );
        }
        // `artifact-folds`: a header row per **visible** labelled section,
        // in order, each followed by that section's own rendered body
        // exactly when it is open, and a blank separator row after a
        // non-empty open body that a further visible section follows.
        let visible = visible_sections(&detail.sections, &detail.expanded);
        // `artifact-folds`: one indent decision for the whole call, taken
        // before the walk from the content width and the tab's own deepest
        // body-bearing section.
        let indented = bodies_are_indented(&detail.sections, width);
        for (position, &index) in visible.iter().enumerate() {
            let section = &detail.sections[index];
            // A `None`-labelled preamble is always open and owns no
            // header row at all (design.md -> D2).
            let open = section.label.is_none() || detail.expanded.contains(&index);
            if let Some(label) = section.label.as_deref() {
                out.push(ContentRow {
                    line: header(
                        label,
                        section.depth,
                        open,
                        section.progress.as_ref(),
                        section.operation,
                        width,
                    ),
                    kind: ContentKind::SectionHeader {
                        section: index,
                        selected: false,
                    },
                });
            }
            if open {
                // A tracked-tasks section's body is its items and nothing
                // else — `ui::tasks::items`, the very function
                // `ui::tasks::lines` calls per group, so a folded group and
                // an unfolded one cannot disagree about an item line
                // (design.md -> D9). Its own heading is already its header
                // row above.
                //
                // The body is *wrapped* at `width - indent_cols` and then
                // indented, never wrapped at `width` and prefixed: prefixing
                // would push every long line past an interior the region does
                // not clip. Both grammars take the reduced width; the
                // progress bar above every header and the blank separator
                // below this body do not — neither is a section's body, and
                // the separator is already exactly `width` blank columns.
                let indent_cols = if indented {
                    indent_columns(section.depth)
                } else {
                    0
                };
                let indent = " ".repeat(indent_cols as usize);
                let body_width = width.saturating_sub(indent_cols);
                let body = match tracked_tasks_progress {
                    Some(_) => crate::ui::tasks::items(
                        &crate::tasks::parse(&section.text)
                            .groups
                            .into_iter()
                            .flat_map(|g| g.items)
                            .collect::<Vec<_>>(),
                        body_width,
                    ),
                    None => crate::ui::markdown::lines(&section.text, body_width),
                };
                let non_empty = !body.is_empty();
                out.extend(
                    body.into_iter()
                        .map(|line| body_row(indented_line(&indent, line))),
                );
                if non_empty && position + 1 < visible.len() {
                    out.push(separator_row(width));
                }
            }
        }
    } else {
        // A non-foldable tab is byte-identical to what it drew before this
        // change: the whole checklist grammar over the one section's text
        // (which is a task file holding no items, or one whose single group
        // left no second section to fold against), or the markdown one.
        let text: String = detail.sections.iter().map(|s| s.text.as_str()).collect();
        let body = match tracked_tasks_progress {
            Some(progress) => crate::ui::tasks::lines(&text, progress, width),
            None => crate::ui::markdown::lines(&text, width),
        };
        out.extend(body.into_iter().map(body_row));
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

/// The half-open display-column range `[start, end)` of the whitespace-
/// delimited word covering `column` in `rows[line]`'s own rendered text.
/// `None` when `line` is past the end of `rows`, or `column` lands on a
/// whitespace cell — including one in a row's own trailing padding, which
/// is whitespace like any other — or past the row's own rendered end.
///
/// See `specs/text-selection/spec.md` -> "A press arms, a second selects
/// the word, a third selects the row": "A word is a maximal run of
/// non-whitespace display columns in the rendered row, so an identifier, a
/// path, or a backticked span selects whole — `ui::layout::zone` is one
/// word, not three" and "A second press whose cell holds only whitespace
/// SHALL select nothing".
///
/// Word boundaries are found with `char_indices`, a textual boundary
/// search rather than a width measurement; every column reported is
/// measured through [`columns`], never a `char` count (`COLWIDTH`).
pub fn word_at(rows: &[ContentRow], line: usize, column: u16) -> Option<(u16, u16)> {
    let text = rows.get(line)?.text();
    let mut run_start: Option<usize> = None;
    for (byte, ch) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')))
    {
        if ch.is_whitespace() {
            if let Some(start) = run_start.take() {
                let start_col = columns(&text[..start]) as u16;
                let end_col = columns(&text[..byte]) as u16;
                if (start_col..end_col).contains(&column) {
                    return Some((start_col, end_col));
                }
            }
        } else if run_start.is_none() {
            run_start = Some(byte);
        }
    }
    None
}

/// The text a selection from `anchor` to `focus` copies. Each is a
/// `(line, column)` pair into `rows` and is accepted in either order: the
/// pair is sorted before either endpoint is used, so dragging upward
/// selects the same text as dragging downward across the same two points
/// (`specs/text-selection/spec.md` -> "Anchor holds while the focus
/// follows").
///
/// Every covered line contributes its own display columns `[from, to)` —
/// clamped to that line's own rendered length, `from` defaulting to `0` and
/// `to` to the line's own end for every line strictly between the two
/// endpoints — with the trailing whitespace a rendered row is padded to
/// dropped from every contributed line rather than copied verbatim: what is
/// copied is the text, not the cells (`specs/text-selection/spec.md` -> "The
/// selected text is copied through the terminal seam"). The contributed
/// lines are joined with a single `\n`.
///
/// A line index past the end of `rows` truncates the span there rather than
/// panicking — the same total behaviour every other function in this file
/// holds for an out-of-range index.
pub fn span_text(rows: &[ContentRow], anchor: (usize, u16), focus: (usize, u16)) -> String {
    let (start, end) = if anchor <= focus {
        (anchor, focus)
    } else {
        (focus, anchor)
    };
    let mut lines: Vec<String> = Vec::new();
    for line in start.0..=end.0 {
        let Some(row) = rows.get(line) else {
            break;
        };
        let text = row.text();
        let total = columns(&text) as u16;
        let from = if line == start.0 {
            start.1.min(total)
        } else {
            0
        };
        let to = if line == end.0 {
            end.1.min(total)
        } else {
            total
        };
        let slice = if to <= from {
            ""
        } else {
            let start_byte = truncate_columns(&text, from as usize).len();
            let end_byte = truncate_columns(&text, to as usize).len();
            &text[start_byte..end_byte]
        };
        lines.push(slice.trim_end().to_string());
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{
        ContentKind, ContentRow, Tab, content_lines, header, header_row, section_at, span_text,
        tab_bar, word_at,
    };
    use crate::changes::fixture;
    use crate::tasks::Progress;
    use crate::testutil::{cell, render_at, row_text};
    use crate::ui::app::{ArtifactSection, Dashboard, Detail, Filter, Route};
    use crate::ui::layout::columns;
    use crate::ui::list::pad_or_truncate_right;

    /// A bare single-segment body row over `text`, unpadded and untruncated
    /// — the same shape [`body_row`] wraps a rendered [`crate::ui::markdown::Line`]
    /// in, but built directly so a `word_at`/`span_text` test can name its
    /// own row content byte for byte rather than reasoning about
    /// `ui::markdown`'s wrapping.
    fn plain_row(text: &str) -> ContentRow {
        ContentRow {
            line: crate::ui::markdown::Line {
                segments: vec![crate::ui::markdown::Segment {
                    text: text.to_string(),
                    face: crate::ui::markdown::Face::plain(),
                }],
            },
            kind: ContentKind::Body,
        }
    }

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
                    label: Some("degraded-coverage".to_string()),
                    text: "one\n".to_string(),
                    depth: 0,
                    progress: None,
                    operation: None,
                },
                ArtifactSection {
                    label: Some("markdown-render".to_string()),
                    text: "two\n".to_string(),
                    depth: 0,
                    progress: None,
                    operation: None,
                },
                ArtifactSection {
                    label: Some("tasks-checklist".to_string()),
                    text: "three\n".to_string(),
                    depth: 0,
                    progress: None,
                    operation: None,
                },
            ],
            scroll: 0,
            tab: 0,
            problems: Vec::new(),
            loaded: None,
            expanded,
            // 78 columns: the mandated wide detail interior. These bodies
            // are single words, so no wrap -- the row list is the same at
            // any width, and the fold resolves rather than going inert.
            drawn_width: Some(78),
        }
    }

    /// `heading-sections`: the seven-section spec-glob `Detail` — the shape
    /// `ui::app::tests::a_spec_glob_nests_requirements_under_their_capability`
    /// derives through `sync_detail`, at depths `0, 1, 2, 3, 2, 0, 0`. Its
    /// depth-3 section is what makes a width sweep from `0` reach the band in
    /// which a header's indent alone exceeds the region.
    fn seven_section_detail(expanded: std::collections::BTreeSet<usize>) -> Detail {
        let section = |label: &str, text: &str, depth: usize| ArtifactSection {
            label: Some(label.to_string()),
            text: text.to_string(),
            depth,
            progress: None,
            operation: None,
        };
        Detail {
            sections: vec![
                section("degraded-coverage", "", 0),
                section("ADDED Requirements", "\n", 1),
                section("Requirement: Alpha", "Alpha text.\n\n", 2),
                section("Scenario: A works", "- **WHEN** a\n- **THEN** b\n\n", 3),
                section("Requirement: Beta", "Beta text.\n", 2),
                section("markdown-render", "## MODIFIED Requirements\n", 0),
                section("tasks-checklist", "## MODIFIED Requirements\n", 0),
            ],
            scroll: 0,
            tab: 0,
            problems: Vec::new(),
            loaded: None,
            expanded,
            drawn_width: Some(78),
        }
    }

    /// `heading-sections`: a `None`-labelled preamble section followed by two
    /// depth-0 task groups — the shape `sync_detail` derives from the spec's
    /// own two-group task file, whose first line is `Intro prose.`. The
    /// preamble owns no header row and is never a fold target, which is what
    /// this fixture exists to drive through the width properties.
    fn preamble_detail(expanded: std::collections::BTreeSet<usize>) -> Detail {
        Detail {
            sections: vec![
                ArtifactSection {
                    label: None,
                    text: "Intro prose.\n\n".to_string(),
                    depth: 0,
                    progress: None,
                    operation: None,
                },
                ArtifactSection {
                    label: Some("1. Setup".to_string()),
                    text: "- [x] 1.1 first\n- [ ] 1.2 second\n\n".to_string(),
                    depth: 0,
                    progress: None,
                    operation: None,
                },
                ArtifactSection {
                    label: Some("2. Build".to_string()),
                    text: "- [ ] 2.1 third\n".to_string(),
                    depth: 0,
                    progress: None,
                    operation: None,
                },
            ],
            scroll: 0,
            tab: 0,
            problems: Vec::new(),
            loaded: None,
            expanded,
            drawn_width: Some(78),
        }
    }

    fn detail(source: &str, problems: Vec<String>) -> Detail {
        Detail {
            sections: if source.is_empty() {
                Vec::new()
            } else {
                vec![ArtifactSection {
                    label: Some(String::new()),
                    text: source.to_string(),
                    depth: 0,
                    progress: None,
                    operation: None,
                }]
            },
            scroll: 0,
            tab: 0,
            problems,
            loaded: None,
            expanded: std::collections::BTreeSet::new(),
            drawn_width: None,
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

            let at_18 = header(label, 0, false, None, None, 18).text();
            assert_eq!(columns(&at_18), 18);
            assert_eq!(at_18, format!("{glyph} degraded-covera…"));

            let at_13 = header(label, 0, false, None, None, 13).text();
            assert_eq!(columns(&at_13), 13);
            assert_eq!(at_13, format!("{glyph} degraded-c…"));

            // The glyph and its separating space survive the truncation at
            // both widths: the row still opens with them, unchanged.
            assert!(at_18.starts_with(&format!("{glyph} ")));
            assert!(at_13.starts_with(&format!("{glyph} ")));

            // `heading-sections` -> design.md -> D7: a depth-2 header carries
            // four columns of indent, emitted **before** the glyph, so
            // truncation eats the label first and the depth survives it.
            let nested = "Requirement: Alpha";
            let nested_18 = header(nested, 2, false, None, None, 18).text();
            assert_eq!(columns(&nested_18), 18);
            assert_eq!(nested_18, format!("    {glyph} Requirement…"));
            // At 13 the same grammar keeps six columns of indent-and-glyph and
            // spends the remaining seven on the label: six of its columns plus
            // the `…`. (The spec's own literal for this case, `    > Requi…`,
            // measures twelve rather than thirteen columns — one short of the
            // region — so the row below is what the crate's one truncation
            // rule, `ui::list::pad_or_truncate_right`, actually returns.)
            let nested_13 = header(nested, 2, false, None, None, 13).text();
            assert_eq!(columns(&nested_13), 13);
            assert_eq!(nested_13, format!("    {glyph} Requir…"));
            assert!(nested_18.starts_with(&format!("    {glyph} ")));
            assert!(nested_13.starts_with(&format!("    {glyph} ")));

            // A depth-3 header's six columns of indent alone exceed a
            // four-column region: the row degrades to truncated **indent**
            // rather than to a dropped glyph, and does not panic.
            let deep = header("Scenario: A works", 3, false, None, None, 4).text();
            assert_eq!(columns(&deep), 4);
            assert_eq!(deep, "   …");
            assert!(
                !deep.contains(glyph),
                "the glyph was not kept at the cost of the indent"
            );
            for width in 0u16..=20 {
                let got = header("Scenario: A works", 3, false, None, None, width).text();
                assert!(
                    columns(&got) <= width as usize,
                    "depth 3 width {width}: {got:?}"
                );
            }

            // The mandated pair, named explicitly per DETAILWIDTHS: the
            // label fits whole at both, so the row is padded rather than
            // truncated, and still opens with the indent, the glyph and its
            // space — at depth 0, 2, and 3 alike.
            for width in [78, 58] {
                for depth in [0usize, 2, 3] {
                    let got = header(label, depth, false, None, None, width).text();
                    assert_eq!(columns(&got), width as usize, "width {width} depth {depth}");
                    assert!(
                        got.trim_end().ends_with(label),
                        "width {width} depth {depth}: {got:?}"
                    );
                    assert!(
                        got.starts_with(&format!("{}{glyph} ", "  ".repeat(depth))),
                        "width {width} depth {depth}: {got:?}"
                    );
                }
            }
        }

        /// A CJK label is truncated in display columns, not in `char`s, so
        /// its header still measures at most the content width even though
        /// its `chars().count()` would be smaller than that.
        #[test]
        fn a_cjk_label_is_truncated_in_columns() {
            let label = "日本語のラベルです見出しの続き";
            for width in [18u16, 13, 78, 58] {
                for depth in [0usize, 2, 3] {
                    let got = header(label, depth, false, None, None, width).text();
                    assert!(
                        columns(&got) <= width as usize,
                        "width {width} depth {depth}: {got:?} exceeds its width"
                    );
                }
            }
        }

        /// Total and never panicking from `0` through `20`, plus the
        /// mandated pair — `header` is one of the functions `DETAILWIDTHS`
        /// requires to name both.
        /// `artifact-folds` :: "The progress cell is dropped whole rather than
        /// truncated".
        ///
        /// `DETAILWIDTHS` carries no exemption list, so 58 and 78 are named as
        /// the contrasted pair even though the interesting widths are the
        /// 0..=40 sweep: the drop fires well below either.
        #[test]
        fn the_progress_cell_is_dropped_whole_rather_than_truncated() {
            let group = crate::tasks::Progress {
                completed: 1,
                total: 2,
            };
            let cell = crate::ui::list::progress_cell(&group);
            assert_eq!(cell, "[1/2]");

            let mut present = 0usize;
            let mut absent = 0usize;
            for width in 0u16..=40 {
                let got = header("1. Setup", 0, false, Some(&group), None, width).text();
                assert!(
                    columns(&got) <= width as usize,
                    "width {width}: {got:?} exceeds it"
                );
                if got.contains(&cell) {
                    present += 1;
                    // The cell yields to the label rather than the other way
                    // round: at least one column of label survives beside it.
                    assert!(
                        got.contains("1") || got.contains('…'),
                        "width {width}: the label was pushed out entirely: {got:?}"
                    );
                } else {
                    absent += 1;
                    // No partial cell is ever drawn.
                    for fragment in ['[', ']', '/'] {
                        assert!(
                            !got.contains(fragment),
                            "width {width}: a partial cell was drawn: {got:?}"
                        );
                    }
                }
            }
            assert!(
                present > 0 && absent > 0,
                "the sweep must cross the drop, not sit on one side of it \
                 (present {present}, absent {absent})"
            );

            // The two mandated interior widths, where the cell always fits and
            // sits flush against the row's own last column.
            // Unsuffixed: `DETAILWIDTHS`' number scan is `\b(\d+)\b` and does
            // not see `58u16`.
            for width in [58, 78] {
                let got = header("1. Setup", 0, false, Some(&group), None, width).text();
                assert_eq!(columns(&got), width as usize, "width {width}");
                assert!(got.ends_with(&cell), "width {width}: {got:?}");
                // Byte-identical to `ui::list::progress_cell` on the same
                // value, so the row provably does not format its own.
                assert!(
                    got.ends_with(&crate::ui::list::progress_cell(&group)),
                    "width {width}"
                );
                // And a section carrying no progress draws no cell at all,
                // byte-identical to the row this capability drew before.
                assert_eq!(
                    header("1. Setup", 0, false, None, None, width).text(),
                    crate::ui::list::pad_or_truncate_right("▸ 1. Setup", width as usize),
                    "width {width}"
                );
            }
        }

        /// `artifact-folds` :: "A row carrying both a badge and a progress
        /// cell drops them in the stated order". The combination the
        /// requirement specifies and no other test constructs: the Change
        /// Review found the cell outliving the badge here, so that widening
        /// the row from 8 to 9 columns made the badge vanish and reappear at
        /// 11. `label_area` now reserves the badge's own columns.
        ///
        /// Both mandated detail widths, 58 and 78, are swept by the 0..=78
        /// range below rather than named as special cases — the property is
        /// total, and a band that misbehaves is exactly what was wrong.
        #[test]
        fn a_row_carrying_both_a_badge_and_a_progress_cell_drops_them_in_the_stated_order() {
            let progress = crate::tasks::Progress {
                completed: 1,
                total: 2,
            };
            for depth in [0usize, 1] {
                let mut badge_from: Option<u16> = None;
                let mut cell_from: Option<u16> = None;
                for width in 0u16..=78 {
                    let line = header(
                        "Requirement: A",
                        depth,
                        false,
                        Some(&progress),
                        Some(crate::specs::DeltaOp::Added),
                        width,
                    );
                    let text = line.text();
                    assert_eq!(
                        columns(&text),
                        width as usize,
                        "depth {depth} width {width}: {text:?}"
                    );
                    let badge = line.segments.len() >= 4;
                    let cell = text.ends_with("[1/2]");

                    // Monotonic: once drawn, never dropped by widening.
                    if badge {
                        badge_from.get_or_insert(width);
                    } else {
                        assert!(
                            badge_from.is_none(),
                            "depth {depth}: the badge was drawn at {:?} and is gone at {width}: \
                             {text:?}",
                            badge_from
                        );
                    }
                    if cell {
                        cell_from.get_or_insert(width);
                    } else {
                        assert!(
                            cell_from.is_none(),
                            "depth {depth}: the cell was drawn at {:?} and is gone at {width}: \
                             {text:?}",
                            cell_from
                        );
                    }

                    // The cell yields first, so it never outlives the badge.
                    assert!(
                        !cell || badge,
                        "depth {depth} width {width}: the cell is drawn and the badge is not, \
                         reversing the drop-whole order: {text:?}"
                    );
                }
                let (b, c) = (
                    badge_from.expect("the badge is drawn somewhere in 0..=78"),
                    cell_from.expect("the cell is drawn somewhere in 0..=78"),
                );
                assert!(
                    b < c,
                    "depth {depth}: the badge first appears at {b} and the cell at {c}; the cell \
                     must be the later of the two"
                );

                // The two mandated detail interior widths, named rather than
                // merely swept: at 58 and at 78 a row carrying both fields
                // draws both, which is the case the requirement is actually
                // about — the bands above only prove it degrades in order.
                for width in [58u16, 78] {
                    let line = header(
                        "Requirement: A",
                        depth,
                        false,
                        Some(&progress),
                        Some(crate::specs::DeltaOp::Added),
                        width,
                    );
                    let text = line.text();
                    assert_eq!(
                        line.segments.len(),
                        5,
                        "depth {depth} width {width}: {text:?}"
                    );
                    assert!(
                        text.ends_with("[1/2]"),
                        "depth {depth} width {width}: no progress cell: {text:?}"
                    );
                    assert!(
                        text.contains("+ Requirement: A"),
                        "depth {depth} width {width}: no badge: {text:?}"
                    );
                }
            }
        }

        #[test]
        fn header_is_total_from_zero_through_twenty_columns() {
            let label = "degraded-coverage";
            for expanded in [false, true] {
                for depth in 0usize..=3 {
                    for width in 0u16..=20 {
                        let got = header(label, depth, expanded, None, None, width).text();
                        assert!(
                            columns(&got) <= width as usize,
                            "expanded {expanded} depth {depth} width {width}: {got:?}"
                        );
                    }
                    for width in [78, 58] {
                        let got = header(label, depth, expanded, None, None, width).text();
                        assert_eq!(columns(&got), width as usize, "width {width} depth {depth}");
                    }
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

    /// `spec-emphasis` :: "The three operations draw three different
    /// markers".
    #[test]
    fn the_three_operations_draw_three_different_markers() {
        use crate::specs::DeltaOp;
        for width in [78, 58] {
            let detail = Detail {
                sections: vec![
                    ArtifactSection {
                        label: Some("specs".to_string()),
                        text: String::new(),
                        depth: 0,
                        progress: None,
                        operation: None,
                    },
                    ArtifactSection {
                        label: Some("Requirement: Alpha".to_string()),
                        text: "Alpha text.\n".to_string(),
                        depth: 1,
                        progress: None,
                        operation: Some(DeltaOp::Added),
                    },
                    ArtifactSection {
                        label: Some("Requirement: Beta".to_string()),
                        text: "Beta text.\n".to_string(),
                        depth: 1,
                        progress: None,
                        operation: Some(DeltaOp::Modified),
                    },
                    ArtifactSection {
                        label: Some("Requirement: Gamma".to_string()),
                        text: "Gamma text.\n".to_string(),
                        depth: 1,
                        progress: None,
                        operation: Some(DeltaOp::Removed),
                    },
                ],
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
                // The file section (0) must be open for its depth-1
                // children to be visible at all; the three requirement
                // headers stay collapsed themselves, matching the
                // scenario's "three collapsed requirement headers".
                expanded: std::collections::BTreeSet::from([0]),
                drawn_width: Some(width),
            };
            let rows = content_lines(&detail, None, width);
            assert_eq!(rows.len(), 4, "width {width}");

            // The file section is open (its children must be, to be
            // visible at all), so its own glyph is the *open* one; the
            // three requirement headers stay collapsed, carrying the
            // closed glyph.
            let open_glyph = crate::ui::list::fold_glyph(false);
            let glyph = crate::ui::list::fold_glyph(true);
            assert_eq!(
                rows[0].text(),
                crate::ui::list::pad_or_truncate_right(
                    &format!("{open_glyph} specs"),
                    width as usize
                ),
                "width {width}"
            );
            assert_eq!(
                rows[1].text(),
                crate::ui::list::pad_or_truncate_right(
                    &format!("  {glyph} + Requirement: Alpha"),
                    width as usize
                ),
                "width {width}"
            );
            assert_eq!(
                rows[2].text(),
                crate::ui::list::pad_or_truncate_right(
                    &format!("  {glyph} ~ Requirement: Beta"),
                    width as usize
                ),
                "width {width}"
            );
            assert_eq!(
                rows[3].text(),
                crate::ui::list::pad_or_truncate_right(
                    &format!("  {glyph} - Requirement: Gamma"),
                    width as usize
                ),
                "width {width}"
            );

            for row in &rows[1..4] {
                // `spec-emphasis`: four, the fourth being the row's padding,
                // plain-faced so no badge or label face reaches the blank
                // columns that fill the row to its width.
                assert_eq!(row.line.segments.len(), 4, "width {width}: {row:?}");
                assert_eq!(
                    row.line.segments[3].face,
                    crate::ui::markdown::Face::plain(),
                    "width {width}: the padding segment is faced: {row:?}"
                );
            }
            let added = rows[1].line.segments[1].face;
            let modified = rows[2].line.segments[1].face;
            let removed = rows[3].line.segments[1].face;
            assert_eq!(
                added,
                crate::ui::markdown::Face {
                    delta: Some(DeltaOp::Added),
                    ..crate::ui::markdown::Face::plain()
                },
                "width {width}"
            );
            assert_eq!(
                modified,
                crate::ui::markdown::Face {
                    delta: Some(DeltaOp::Modified),
                    ..crate::ui::markdown::Face::plain()
                },
                "width {width}"
            );
            assert_eq!(
                removed,
                crate::ui::markdown::Face {
                    delta: Some(DeltaOp::Removed),
                    ..crate::ui::markdown::Face::plain()
                },
                "width {width}"
            );
            assert_ne!(added, modified, "width {width}");
            assert_ne!(modified, removed, "width {width}");
            assert_ne!(added, removed, "width {width}");
        }
    }

    /// `spec-emphasis` :: "An unbadged header row is unchanged in every
    /// column".
    #[test]
    fn an_unbadged_header_row_is_unchanged_in_every_column() {
        for width in [78, 58] {
            let detail = three_spec_detail(std::collections::BTreeSet::new());
            let rows = content_lines(&detail, None, width);
            assert_eq!(rows.len(), 3, "width {width}");
            for row in &rows {
                assert_eq!(row.line.segments.len(), 1, "width {width}: {row:?}");
                assert_eq!(
                    row.line.segments[0].face,
                    crate::ui::markdown::Face::plain(),
                    "width {width}"
                );
            }
            let glyph = crate::ui::list::fold_glyph(true);
            assert_eq!(
                rows[0].text(),
                crate::ui::list::pad_or_truncate_right(
                    &format!("{glyph} degraded-coverage"),
                    width as usize
                ),
                "width {width}"
            );
            assert_eq!(
                rows[1].text(),
                crate::ui::list::pad_or_truncate_right(
                    &format!("{glyph} markdown-render"),
                    width as usize
                ),
                "width {width}"
            );
            assert_eq!(
                rows[2].text(),
                crate::ui::list::pad_or_truncate_right(
                    &format!("{glyph} tasks-checklist"),
                    width as usize
                ),
                "width {width}"
            );
        }
    }

    /// `spec-emphasis` :: "A removed requirement's heading is struck and its
    /// body is not".
    #[test]
    fn a_removed_requirements_heading_is_struck_and_its_body_is_not() {
        use crate::specs::DeltaOp;
        for width in [78, 58] {
            let make = |op: DeltaOp| Detail {
                sections: vec![
                    ArtifactSection {
                        label: Some("specs".to_string()),
                        text: String::new(),
                        depth: 0,
                        progress: None,
                        operation: None,
                    },
                    ArtifactSection {
                        label: Some("Requirement: Alpha".to_string()),
                        text: "Alpha text.\n\n- **WHEN** a\n- **THEN** b\n".to_string(),
                        depth: 1,
                        progress: None,
                        operation: Some(op),
                    },
                ],
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
                expanded: std::collections::BTreeSet::from([0, 1]),
                drawn_width: Some(width),
            };

            let removed = make(DeltaOp::Removed);
            let rows = content_lines(&removed, None, width);
            let header_row = rows
                .iter()
                .find(|r| matches!(r.kind, ContentKind::SectionHeader { section: 1, .. }))
                .expect("the requirement's header row is drawn");
            assert_eq!(header_row.line.segments.len(), 4, "width {width}");
            assert_eq!(header_row.line.segments[1].text, "- ", "width {width}");
            assert!(
                header_row.line.segments[2].face.strikethrough,
                "width {width}"
            );

            for row in &rows {
                if matches!(row.kind, ContentKind::SectionHeader { .. }) {
                    continue;
                }
                for segment in &row.line.segments {
                    assert!(
                        !segment.face.strikethrough,
                        "width {width}: a body row was struck: {row:?}"
                    );
                }
            }

            let added = make(DeltaOp::Added);
            let rows = content_lines(&added, None, width);
            let header_row = rows
                .iter()
                .find(|r| matches!(r.kind, ContentKind::SectionHeader { section: 1, .. }))
                .expect("the requirement's header row is drawn");
            assert!(
                !header_row.line.segments[2].face.strikethrough,
                "width {width}"
            );
        }
    }

    /// `spec-emphasis` :: "The label truncates before the badge is
    /// dropped".
    #[test]
    fn the_label_truncates_before_the_badge_is_dropped() {
        use crate::specs::DeltaOp;
        let label = "a".repeat(200);
        for width in [78, 58] {
            let detail = Detail {
                sections: vec![
                    ArtifactSection {
                        label: Some("specs".to_string()),
                        text: String::new(),
                        depth: 0,
                        progress: None,
                        operation: None,
                    },
                    ArtifactSection {
                        label: Some(label.clone()),
                        text: String::new(),
                        depth: 1,
                        progress: None,
                        operation: Some(DeltaOp::Modified),
                    },
                ],
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
                expanded: std::collections::BTreeSet::from([0]),
                drawn_width: Some(width),
            };
            let rows = content_lines(&detail, None, width);
            let header_row = rows
                .iter()
                .find(|r| matches!(r.kind, ContentKind::SectionHeader { section: 1, .. }))
                .expect("the requirement's header row is drawn");
            assert_eq!(columns(&header_row.text()), width as usize, "width {width}");
            assert_eq!(
                header_row.line.segments.len(),
                4,
                "width {width}: {header_row:?}"
            );
            assert_eq!(header_row.line.segments[1].text, "~ ", "width {width}");
            assert!(
                header_row.line.segments[2].text.ends_with('…'),
                "width {width}: {header_row:?}"
            );
            let glyph = crate::ui::list::fold_glyph(true);
            assert!(
                header_row.text().starts_with(&format!("  {glyph} ")),
                "width {width}: {:?}",
                header_row.text()
            );
        }
    }

    /// `spec-emphasis` :: "The badge is dropped whole at a width that
    /// cannot hold it".
    #[test]
    fn the_badge_is_dropped_whole_at_a_width_that_cannot_hold_it() {
        use crate::specs::DeltaOp;
        let label = "a".repeat(200);
        let make = |width: u16| Detail {
            sections: vec![
                ArtifactSection {
                    label: Some("specs".to_string()),
                    text: String::new(),
                    depth: 0,
                    progress: None,
                    operation: None,
                },
                ArtifactSection {
                    label: Some(label.clone()),
                    text: String::new(),
                    depth: 1,
                    progress: None,
                    operation: Some(DeltaOp::Modified),
                },
            ],
            scroll: 0,
            tab: 0,
            problems: Vec::new(),
            loaded: None,
            expanded: std::collections::BTreeSet::from([0]),
            drawn_width: Some(width),
        };
        let mut present = 0usize;
        let mut absent = 0usize;
        for width in 0u16..=20 {
            let detail = make(width);
            let rows = content_lines(&detail, None, width);
            let header_row = rows
                .iter()
                .find(|r| matches!(r.kind, ContentKind::SectionHeader { section: 1, .. }))
                .expect("the requirement's header row is drawn");
            assert!(
                columns(&header_row.text()) <= width as usize,
                "width {width}: {:?}",
                header_row.text()
            );
            match header_row.line.segments.len() {
                1 => {
                    absent += 1;
                    assert!(
                        !header_row.text().contains('~'),
                        "width {width}: a stray marker: {:?}",
                        header_row.text()
                    );
                }
                4 => {
                    present += 1;
                    assert_eq!(header_row.line.segments[1].text, "~ ", "width {width}");
                }
                n => panic!("width {width}: unexpected segment count {n}: {header_row:?}"),
            }
        }
        assert!(
            present > 0 && absent > 0,
            "the sweep must cross the drop, not sit on one side of it \
             (present {present}, absent {absent})"
        );

        // The two mandated interior widths, named explicitly per
        // `DETAILWIDTHS`: at both the badge is comfortably present, since
        // the drop fires well below either.
        for width in [78, 58] {
            let detail = make(width);
            let rows = content_lines(&detail, None, width);
            let header_row = rows
                .iter()
                .find(|r| matches!(r.kind, ContentKind::SectionHeader { section: 1, .. }))
                .expect("the requirement's header row is drawn");
            assert_eq!(header_row.line.segments.len(), 4, "width {width}");
            assert_eq!(header_row.line.segments[1].text, "~ ", "width {width}");
        }
    }

    /// `spec-emphasis` :: "A badged header row is still addressed by its
    /// own section index".
    #[test]
    fn a_badged_header_row_is_still_addressed_by_its_own_section_index() {
        use crate::specs::DeltaOp;
        for width in [78, 58] {
            let make = |expanded: std::collections::BTreeSet<usize>| Detail {
                sections: vec![
                    ArtifactSection {
                        label: Some("specs".to_string()),
                        text: String::new(),
                        depth: 0,
                        progress: None,
                        operation: None,
                    },
                    ArtifactSection {
                        label: Some("Requirement: Alpha".to_string()),
                        text: "Alpha text.\n".to_string(),
                        depth: 1,
                        progress: None,
                        operation: Some(DeltaOp::Added),
                    },
                    ArtifactSection {
                        label: Some("Requirement: Beta".to_string()),
                        text: "Beta text.\n".to_string(),
                        depth: 1,
                        progress: None,
                        operation: Some(DeltaOp::Modified),
                    },
                    ArtifactSection {
                        label: Some("Requirement: Gamma".to_string()),
                        text: "Gamma text.\n".to_string(),
                        depth: 1,
                        progress: None,
                        operation: Some(DeltaOp::Removed),
                    },
                ],
                // The cursor sits on row 2 — the second requirement's own
                // header.
                scroll: 2,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
                expanded,
                drawn_width: Some(width),
            };

            let collapsed = make(std::collections::BTreeSet::from([0]));
            let rows = content_lines(&collapsed, None, width);
            assert_eq!(rows.len(), 4, "width {width}");
            assert_eq!(section_at(&rows, 0, 2), Some(2), "width {width}");
            assert!(
                matches!(rows[2].kind, ContentKind::SectionHeader { section: 2, .. }),
                "width {width}: {:?}",
                rows[2].kind
            );

            // Toggling that same index into `expanded` — the effect a
            // `Space` press has — opens section 2's own body and no other
            // section's; the badge changes nothing about which section a
            // fold reaches.
            let mut opened_set = std::collections::BTreeSet::from([0]);
            opened_set.insert(2);
            let opened = make(opened_set);
            let rows2 = content_lines(&opened, None, width);
            // `section-body-indent`: every body-bearing section here is at
            // depth 1, so this tab's floor is `64 + 2 * 1 = 66` — the body
            // carries two columns of indent at 78 and none at 58.
            let indent = if width == 78 { "  " } else { "" };
            assert!(
                rows2
                    .iter()
                    .any(|r| r.text() == format!("{indent}Beta text.")),
                "width {width}: section 2's body did not open"
            );
            assert!(
                !rows2.iter().any(|r| r.text().trim() == "Alpha text."),
                "width {width}: section 1 opened when it should not have"
            );
            assert!(
                !rows2.iter().any(|r| r.text().trim() == "Gamma text."),
                "width {width}: section 3 opened when it should not have"
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
            selection: None,
            help: crate::ui::app::Help {
                open: false,
                scroll: 0,
            },
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

    /// `detail-header` :: "The full header grammar at both mandated interior widths".
    #[test]
    fn the_full_header_grammar_at_both_mandated_interior_widths() {
        let progress = Progress {
            completed: 4,
            total: 42,
        };
        for (width, name_field_width) in [(78, 52usize), (58, 32)] {
            let got = header_row("detail-view", "tdd", &progress, width);
            assert_eq!(columns(&got), width as usize, "width {width}");
            let expected_name_field =
                format!("{:<width$}", "detail-view", width = name_field_width);
            // `12 * 4 / 42` truncates to 1: one filled cell, eleven empty.
            let expected = format!("{expected_name_field} (tdd) █░░░░░░░░░░░ [4/42]");
            assert_eq!(got, expected, "width {width}");
            assert!(got.ends_with("[4/42]"), "width {width}");
        }
    }

    /// `detail-header` :: "A complete change renders a full gauge and an untouched
    /// one renders an empty gauge".
    #[test]
    fn a_complete_change_renders_a_full_gauge() {
        for width in [78, 58] {
            let full = Progress {
                completed: 7,
                total: 7,
            };
            let got = header_row("fix-empty-basket", "tdd", &full, width);
            assert_eq!(columns(&got), width as usize, "width {width}");
            assert!(got.ends_with("[7/7]"), "width {width}: {got:?}");
            assert_eq!(
                got.chars().filter(|&c| c == '█').count(),
                12,
                "width {width}: {got:?}"
            );
            assert!(!got.contains('░'), "width {width}: {got:?}");

            let empty = Progress {
                completed: 0,
                total: 7,
            };
            let got = header_row("fix-empty-basket", "tdd", &empty, width);
            assert_eq!(columns(&got), width as usize, "width {width}");
            assert!(got.ends_with("[0/7]"), "width {width}: {got:?}");
            assert_eq!(
                got.chars().filter(|&c| c == '░').count(),
                12,
                "width {width}: {got:?}"
            );
            assert!(!got.contains('█'), "width {width}: {got:?}");

            // A one-task-short change never renders a full gauge, the same
            // `filled == g` iff `is_complete()` property `tasks-progress-bar` fixes.
            let almost = Progress {
                completed: 6,
                total: 7,
            };
            let got = header_row("fix-empty-basket", "tdd", &almost, width);
            assert!(got.contains('░'), "width {width}: {got:?}");
        }
    }

    /// `detail-header` :: "A change with no tasks still ends its row in the same
    /// column".
    #[test]
    fn a_change_with_no_tasks_still_ends_its_row_in_the_same_column() {
        let progress = Progress {
            completed: 0,
            total: 0,
        };
        for width in [78, 58, 26, 13, 12, 7, 6, 1, 0] {
            let got = header_row("migrate-ai-sdk-v7", "tdd", &progress, width);
            assert_eq!(columns(&got), width as usize, "width {width}");
            assert!(
                !got.contains('█') && !got.contains('░'),
                "width {width}: {got:?}"
            );
            if width == 78 || width == 58 {
                assert!(got.ends_with("[-]"), "width {width}: {got:?}");
                // The scenario's own claim, pinned as a literal rather than
                // inferred from the width: a 68-column name field at 78 and a
                // 48-column one at 58 — the fields this grammar produced before
                // the gauge existed, not ones narrowed by 13 columns for a gauge
                // that is never drawn. `columns(&got) == width` alone cannot tell
                // those apart, since a budget-reserving implementation would pad
                // the name field back to the same total.
                let name_field = width as usize - 2 - "(tdd)".len() - "[-]".len();
                assert_eq!(name_field, if width == 78 { 68 } else { 48 });
                let expected = format!("{:<name_field$} (tdd) [-]", "migrate-ai-sdk-v7");
                assert_eq!(got, expected, "width {width}");
            }
            if width == 0 {
                assert_eq!(got, "", "width {width}");
            }
        }
    }

    /// `detail-header` :: "A long name is truncated with an ellipsis, never
    /// overflowing the row".
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
            assert_eq!(
                got.chars().filter(|&c| c == '█' || c == '░').count(),
                12,
                "width {width}: {got:?}"
            );
            let name_field_end = got.find(" (tdd)").expect("schema cell present");
            let name_field = &got[..name_field_end];
            assert!(
                name_field.ends_with('…'),
                "width {width}: name field does not end in an ellipsis: {name_field:?}"
            );
        }
    }

    /// `detail-header` :: "The cells are dropped whole in order as the row
    /// narrows".
    #[test]
    fn the_cells_are_dropped_whole_in_order_as_the_row_narrows() {
        let progress = Progress {
            completed: 4,
            total: 9,
        };
        for w in [78, 58, 26, 25, 13, 12, 7, 6, 5, 1, 0] {
            let got = header_row("add-token-refresh", "tdd", &progress, w);
            assert_eq!(columns(&got), w as usize, "width {w}");
            if w >= 26 {
                assert!(got.contains("(tdd)"), "width {w}: {got:?}");
                assert!(got.contains("[4/9]"), "width {w}: {got:?}");
                assert_eq!(
                    got.chars().filter(|&c| c == '█' || c == '░').count(),
                    12,
                    "width {w}: {got:?}"
                );
            } else if (13..=25).contains(&w) {
                assert!(got.contains("(tdd)"), "width {w}: {got:?}");
                assert!(got.contains("[4/9]"), "width {w}: {got:?}");
                assert!(
                    !got.contains('█') && !got.contains('░'),
                    "width {w}: {got:?}"
                );
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
            if got.contains('█') || got.contains('░') {
                assert_eq!(
                    got.chars().filter(|&c| c == '█' || c == '░').count(),
                    12,
                    "width {w}: partial gauge cell"
                );
            }
        }
    }

    /// `detail-header` :: "Below the full-form band the header is byte-identical
    /// to the pre-gauge grammar".
    #[test]
    fn below_the_full_form_band_the_header_is_byte_identical() {
        let progress = Progress {
            completed: 4,
            total: 9,
        };
        // Captured from HEAD's `header_row`, before the gauge cell existed —
        // group 0's baseline (`/tmp/header-pregauge.txt`), written here as
        // literals rather than recomputed from the post-gauge implementation,
        // which would make this test a tautology.
        let pre_gauge: [(u16, &str); 26] = [
            (0, ""),
            (1, "…"),
            (2, "a…"),
            (3, "ad…"),
            (4, "add…"),
            (5, "add-…"),
            (6, "add-t…"),
            (7, "… [4/9]"),
            (8, "a… [4/9]"),
            (9, "ad… [4/9]"),
            (10, "add… [4/9]"),
            (11, "add-… [4/9]"),
            (12, "add-t… [4/9]"),
            (13, "… (tdd) [4/9]"),
            (14, "a… (tdd) [4/9]"),
            (15, "ad… (tdd) [4/9]"),
            (16, "add… (tdd) [4/9]"),
            (17, "add-… (tdd) [4/9]"),
            (18, "add-t… (tdd) [4/9]"),
            (19, "add-to… (tdd) [4/9]"),
            (20, "add-tok… (tdd) [4/9]"),
            (21, "add-toke… (tdd) [4/9]"),
            (22, "add-token… (tdd) [4/9]"),
            (23, "add-token-… (tdd) [4/9]"),
            (24, "add-token-r… (tdd) [4/9]"),
            (25, "add-token-re… (tdd) [4/9]"),
        ];
        for (w, expected) in pre_gauge {
            let got = header_row("add-token-refresh", "tdd", &progress, w);
            assert_eq!(got, expected, "width {w}");
        }
        // The other half of the claim: at both mandated interiors the row
        // does differ from what the pre-gauge grammar produced, so a no-op
        // implementation — one that never draws a gauge at all — fails this
        // scenario too, not only the ones above that require a gauge.
        for (width, name_field_width) in [(78, 66usize), (58, 46)] {
            let got = header_row("add-token-refresh", "tdd", &progress, width);
            let old_name_field =
                format!("{:<width$}", "add-token-refresh", width = name_field_width);
            let old = format!("{old_name_field} (tdd) [4/9]");
            assert_ne!(got, old, "width {width}");
        }
    }

    /// `detail-header` :: "An empty schema name is a cell of two characters, not
    /// an absent one".
    #[test]
    fn an_empty_schema_name_is_a_cell_of_two_characters_not_an_absent_one() {
        let progress = Progress {
            completed: 1,
            total: 2,
        };
        for (width, name_field_width) in [(78, 56usize), (58, 36)] {
            let got = header_row("alpha", "", &progress, width);
            assert_eq!(columns(&got), width as usize, "width {width}");
            let expected_name_field = format!("{:<width$}", "alpha", width = name_field_width);
            // `12 * 1 / 2` is 6: six filled cells, six empty.
            let expected = format!("{expected_name_field} () ██████░░░░░░ [1/2]");
            assert_eq!(got, expected, "width {width}: {got:?}");
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
                        label: Some(String::new()),
                        text: sources[tab].to_string(),
                        depth: 0,
                        progress: None,
                        operation: None,
                    }],
                    scroll: 0,
                    tab,
                    problems: Vec::new(),
                    loaded: None,
                    expanded: std::collections::BTreeSet::new(),
                    drawn_width: None,
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
                drawn_width: None,
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
                    drawn_width: None,
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

            // `heading-sections`: one blank separator row — that body is
            // non-empty and a further visible section follows it — and then
            // section 2's own header.
            let separator = &rows[2 + body.len()];
            assert_eq!(separator.kind, ContentKind::Body, "width {width}");
            assert!(
                separator.text().trim().is_empty(),
                "width {width}: {:?} is not blank",
                separator.text()
            );
            assert_eq!(
                columns(&separator.text()),
                width as usize,
                "width {width}: the separator is padded like every row around it"
            );

            let after = &rows[3 + body.len()];
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
    /// than folding". The scenario's name is kept verbatim because a delta's
    /// scenario headers are its merge key; it now pins the **reversal**
    /// (design.md -> D8): the tab folds, and the concatenation is gone.
    ///
    /// The four sections are exactly what `sync_detail` derives for a
    /// tracked-tasks artifact resolving to **two** paths — a depth-0 file
    /// section per path and that file's one group beneath it at depth 1 — so
    /// the shape asserted here is the derivation's, written out rather than
    /// re-derived, which is what keeps this a `content_lines` unit test.
    #[test]
    fn the_tracked_tasks_tab_concatenates_rather_than_folding() {
        let progress = Progress {
            completed: 1,
            total: 2,
        };
        let change =
            fixture::with_marked_artifacts(&paths_free(&["proposal", "tasks"]), Some(1), progress);
        let sections = |first_item: &str| {
            vec![
                ArtifactSection {
                    label: Some("a".to_string()),
                    text: String::new(),
                    depth: 0,
                    progress: None,
                    operation: None,
                },
                ArtifactSection {
                    label: Some("1. Setup".to_string()),
                    text: format!("- [{first_item}] a\n"),
                    depth: 1,
                    progress: None,
                    operation: None,
                },
                ArtifactSection {
                    label: Some("b".to_string()),
                    text: String::new(),
                    depth: 0,
                    progress: None,
                    operation: None,
                },
                ArtifactSection {
                    label: Some("2. Build".to_string()),
                    text: "- [ ] b\n".to_string(),
                    depth: 1,
                    progress: None,
                    operation: None,
                },
            ]
        };
        let detail_of = |sections, expanded| Detail {
            sections,
            scroll: 0,
            tab: 1,
            problems: Vec::new(),
            loaded: None,
            expanded,
            drawn_width: None,
        };
        for width in [78, 58] {
            // `section-body-indent`: the two groups sit at depth 1, so this
            // tab's floor is `64 + 2 * 1 = 66` — its items carry two columns
            // of indent at 78 and none at 58. This fixture is the live proof
            // that a tracked-tasks tab has no exemption of its own.
            let indent = if width == 78 { "  " } else { "" };
            // Both subtrees incomplete: the seed opened every section.
            let d = detail_of(
                sections(" "),
                std::collections::BTreeSet::from([0, 1, 2, 3]),
            );
            let rows = content_lines(&d, Some(&change), width);
            let bar = crate::ui::tasks::progress_bar(&progress, &[], width);
            assert_eq!(
                rows.iter().map(ContentRow::text).collect::<Vec<_>>(),
                vec![
                    bar.clone(),
                    // `ui::tasks`' own blank line, unpadded — `bar_lines`
                    // emits the very pair the flat tab leads with.
                    String::new(),
                    header("a", 0, true, None, None, width).text(),
                    header("1. Setup", 1, true, None, None, width).text(),
                    format!("{indent}[ ] a"),
                    // The fold walk's own separator row, which IS padded.
                    crate::ui::list::pad_or_truncate_right("", width as usize),
                    header("b", 0, true, None, None, width).text(),
                    header("2. Build", 1, true, None, None, width).text(),
                    format!("{indent}[ ] b"),
                ],
                "width {width}"
            );

            // The first file's item checked instead: the seed leaves section
            // `0` collapsed, which hides its own group entirely.
            let checked = detail_of(sections("x"), std::collections::BTreeSet::from([2, 3]));
            let rows = content_lines(&checked, Some(&change), width);
            assert_eq!(
                rows.iter().map(ContentRow::text).collect::<Vec<_>>(),
                vec![
                    bar,
                    String::new(),
                    header("a", 0, false, None, None, width).text(),
                    header("b", 0, true, None, None, width).text(),
                    header("2. Build", 1, true, None, None, width).text(),
                    format!("{indent}[ ] b"),
                ],
                "width {width}: a collapsed depth-0 file section hides its own group"
            );

            for row in &rows {
                assert!(
                    !row.text().starts_with("## "),
                    "width {width}: the two group headings became labels"
                );
                assert!(columns(&row.text()) <= width as usize, "width {width}");
            }
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
                    label: Some(String::new()),
                    text: source.to_string(),
                    depth: 0,
                    progress: None,
                    operation: None,
                }],
                scroll: 0,
                tab: 1,
                problems: Vec::new(),
                loaded: None,
                expanded: std::collections::BTreeSet::new(),
                drawn_width: None,
            };
            let lines = content_lines(&d, Some(&change), width);
            // Discriminating: the marked body's first line is the bar,
            // which the markdown rendering of the same source never
            // produces.
            assert_eq!(
                lines[0].text(),
                crate::ui::tasks::progress_bar(&progress, &[], width),
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
                    label: Some(String::new()),
                    text: source.to_string(),
                    depth: 0,
                    progress: None,
                    operation: None,
                }],
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
                expanded: std::collections::BTreeSet::new(),
                drawn_width: None,
            };
            let lines = content_lines(&d, Some(&change), width);
            let want = crate::ui::markdown::lines(source, width);
            assert_rows_equal_lines(&lines, &want);
            assert_ne!(
                lines[0].text(),
                crate::ui::tasks::progress_bar(&progress, &[], width),
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
                    label: Some(String::new()),
                    text: source.to_string(),
                    depth: 0,
                    progress: None,
                    operation: None,
                }],
                scroll: 0,
                tab: 7,
                problems: Vec::new(),
                loaded: None,
                expanded: std::collections::BTreeSet::new(),
                drawn_width: None,
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
            // `heading-sections`' own tenth and eleventh values: the
            // seven-section spec glob, every section open so the depth-3
            // header is actually drawn, and the `None`-labelled preamble
            // followed by two task groups.
            seven_section_detail(std::collections::BTreeSet::from([0, 1, 2, 3, 4, 5, 6])),
            preamble_detail(std::collections::BTreeSet::from([1, 2])),
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
                    // Every header's carried `section` addresses an entry of
                    // `detail.sections`: a hidden subtree shifts the drawn row
                    // positions, and must never shift a header's own index off
                    // its section.
                    for row in &lines {
                        if let ContentKind::SectionHeader { section, .. } = row.kind {
                            assert!(
                                section < d.sections.len(),
                                "width {width}: header names section {section} of {}",
                                d.sections.len()
                            );
                        }
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
        // `12 * 4 / 9` truncates to 5.
        let gauge = crate::ui::tasks::gauge_of(&progress, 12);
        for width in [78, 58] {
            let got = header_row(name, "tdd", &progress, width);
            assert_eq!(columns(&got), width as usize, "width {width}: {got:?}");
            assert!(got.ends_with("[4/9]"), "width {width}: {got:?}");
            // `(tdd)` is no longer immediately before the progress cell: a space, a
            // twelve-column gauge, and a space now sit between them.
            let expected_tail = format!("(tdd) {gauge} [4/9]");
            assert!(
                got.contains(&expected_tail),
                "width {width}: {got:?} missing {expected_tail:?}"
            );
            // Discriminating: proves the padding was computed in columns rather than in
            // characters — a `chars().count()`-based budget would have produced a header
            // whose `chars().count()` also equalled `width`, dropping the schema, gauge,
            // and progress cells off the row instead. The gauge's twelve characters are
            // also twelve columns, so they contribute equally to both counts and neither
            // strengthen nor weaken this claim.
            assert!(
                got.chars().count() < columns(&got),
                "width {width}: {got:?} was not measured in columns"
            );
        }
    }

    /// `detail-header` :: "The header reaches the buffer without crossing the region
    /// border". The tail is checked by exact column-indexed slicing, using the tail's
    /// **char** count rather than its byte length — the gauge glyphs are multi-byte —
    /// since one buffer cell holds one gauge character; the CJK name field itself is not
    /// reconstructed by slicing `row_text`'s per-column output, because a two-column
    /// grapheme cluster's own trailing cell is reset to a single blank space, which would
    /// otherwise be misread as a character the name never had.
    #[test]
    fn the_header_reaches_the_buffer_without_crossing_the_region_border() {
        let name = "日本語の変更名前です";
        let progress = Progress {
            completed: 4,
            total: 9,
        };
        // `12 * 4 / 9` truncates to 5: five filled cells, seven empty.
        let gauge = crate::ui::tasks::gauge_of(&progress, 12);
        let tail = format!(" (tdd) {gauge} [4/9]");
        let tail_len = tail.chars().count();
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
            let tail_start = first_col + interior - tail_len;
            assert_eq!(
                cols(&row_text(&buf, 0), tail_start..tail_start + tail_len),
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
        // content reaches the frame's own last column instead. No gauge character bled
        // left across the divider, which is the failure a gauge measured in `char`s
        // beside a wide name would produce.
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
        // Four `Progress` values: the band boundaries below are derived from a
        // five-column progress cell (`{4, 9}`), so the other three — a 24- and a
        // 43-column progress cell, and the always-dash `{0, 0}` — get only the
        // width-exactness, no-panic, and drop-whole checks that hold for any cell.
        let progresses = [
            Progress {
                completed: 4,
                total: 9,
            },
            Progress {
                completed: 0,
                total: 0,
            },
            Progress {
                completed: 0,
                total: usize::MAX,
            },
            Progress {
                completed: usize::MAX,
                total: usize::MAX,
            },
        ];
        for name in &names {
            for progress in &progresses {
                for width in 0u16..=130 {
                    let got = header_row(name, "tdd", progress, width);
                    if width == 0 {
                        assert_eq!(got, "", "name {name:?} progress {progress:?} width {width}");
                    } else {
                        assert_eq!(
                            columns(&got),
                            width as usize,
                            "name {name:?} progress {progress:?} width {width}: {got:?}"
                        );
                    }
                    if progress.total == 0 {
                        assert!(
                            !got.contains('█') && !got.contains('░'),
                            "name {name:?} width {width}: {got:?}"
                        );
                    }
                    if progress.completed == 4 && progress.total == 9 {
                        match width {
                            78 | 58 | 26 => {
                                assert!(
                                    got.contains("(tdd)"),
                                    "name {name:?} width {width}: {got:?}"
                                );
                                assert!(
                                    got.contains("[4/9]"),
                                    "name {name:?} width {width}: {got:?}"
                                );
                                assert_eq!(
                                    got.chars().filter(|&c| c == '█' || c == '░').count(),
                                    12,
                                    "name {name:?} width {width}: {got:?}"
                                );
                            }
                            25 | 13 => {
                                assert!(
                                    got.contains("(tdd)"),
                                    "name {name:?} width {width}: {got:?}"
                                );
                                assert!(
                                    got.contains("[4/9]"),
                                    "name {name:?} width {width}: {got:?}"
                                );
                                assert!(
                                    !got.contains('█') && !got.contains('░'),
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
                                assert!(
                                    !got.contains("[4/"),
                                    "name {name:?} width {width}: {got:?}"
                                );
                            }
                            _ => {}
                        }
                    } else if got.contains('[') {
                        // The drop-whole property, for a progress cell too wide for
                        // the documented boundaries to apply to: wherever a `[`
                        // appears, the whole cell is present, never cut short.
                        let cell = crate::ui::list::progress_cell(progress);
                        assert!(
                            got.contains(&cell),
                            "name {name:?} width {width}: {got:?} holds a partial progress cell"
                        );
                    }
                }
            }
            // The mandated pair, asserted explicitly by this sweep too.
            for progress in &progresses {
                for width in [78, 58] {
                    let got = header_row(name, "tdd", progress, width);
                    assert_eq!(
                        columns(&got),
                        width as usize,
                        "name {name:?} progress {progress:?} width {width}"
                    );
                }
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
                    label: Some("日本語ラベル".to_string()),
                    text: "one\n".to_string(),
                    depth: 0,
                    progress: None,
                    operation: None,
                },
                ArtifactSection {
                    label: Some("見出し二番目".to_string()),
                    text: paragraph.clone(),
                    depth: 0,
                    progress: None,
                    operation: None,
                },
                ArtifactSection {
                    label: Some("タスク一覧".to_string()),
                    text: "three\n".to_string(),
                    depth: 0,
                    progress: None,
                    operation: None,
                },
            ],
            scroll: 0,
            tab: 0,
            problems: Vec::new(),
            loaded: None,
            expanded: std::collections::BTreeSet::from([1]),
            drawn_width: None,
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
            // `heading-sections`' own tenth and eleventh values, per
            // `content_lines_total` above. The spec-glob case is what reaches
            // widths `0` through `13` with a depth-3 header whose six columns
            // of indent alone exceed the region: the row degrades to truncated
            // indent rather than to a dropped glyph, and does not panic.
            seven_section_detail(std::collections::BTreeSet::from([0, 1, 2, 3, 4, 5, 6])),
            preamble_detail(std::collections::BTreeSet::from([1, 2])),
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
                    for row in &lines {
                        if let ContentKind::SectionHeader { section, .. } = row.kind {
                            assert!(
                                section < d.sections.len(),
                                "width {width}: header names section {section} of {}",
                                d.sections.len()
                            );
                        }
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

    /// `text-selection` :: "A click selects the whole token, not a
    /// fragment". `ui::layout::zone` is one word, not three: the `::`
    /// atoms are non-whitespace and belong to the same run as the
    /// identifiers either side of them. Run at both mandated widths — the
    /// row itself carries no width, but the fixture it is drawn from does,
    /// so this stays a `src/ui/detail.rs` test in `DETAILWIDTHS`' sense.
    #[test]
    fn a_click_selects_the_whole_token_not_a_fragment() {
        let widths: [u16; 2] = [78, 58];
        for width in widths {
            let d = detail("the `ui::layout::zone` call\n", Vec::new());
            let rows = content_lines(&d, None, width);
            assert_eq!(rows.len(), 1, "width {width}");
            assert_eq!(rows[0].text(), "the ui::layout::zone call", "width {width}");

            // "ui::layout::zone" starts at column 4 (after "the ") and ends
            // at column 20 (4 + 16), a half-open range.
            for column in 4..20u16 {
                assert_eq!(
                    word_at(&rows, 0, column),
                    Some((4, 20)),
                    "width {width}, column {column}"
                );
            }
            // The whitespace either side of the token is not part of it,
            // and belongs to no run at all.
            assert_eq!(word_at(&rows, 0, 3), None, "width {width}");
            assert_eq!(word_at(&rows, 0, 20), None, "width {width}");
            // The leading word "the" is its own, shorter run.
            assert_eq!(word_at(&rows, 0, 0), Some((0, 3)), "width {width}");
            // A line index past the end of the row list resolves to nothing.
            assert_eq!(word_at(&rows, rows.len(), 0), None, "width {width}");
        }
    }

    /// `text-selection` :: "A single-line selection copies exactly the
    /// selected columns". The row is built directly rather than through
    /// `ui::markdown`, so the columns named here are the row's own text
    /// verbatim — a selection stopping well short of a row's end never
    /// touches whatever padding a real rendered row would carry, at either
    /// mandated width.
    #[test]
    fn a_single_line_selection_copies_exactly_the_selected_columns() {
        let widths: [u16; 2] = [78, 58];
        for width in widths {
            let text = "  - **WHEN** the reader presses q";
            let rows = vec![plain_row(&pad_or_truncate_right(text, width as usize))];
            // Columns 4 through 12 of the unpadded text are "**WHEN**".
            assert_eq!(
                span_text(&rows, (0, 4), (0, 12)),
                "**WHEN**",
                "width {width}"
            );
            // The pair is accepted in either order.
            assert_eq!(
                span_text(&rows, (0, 12), (0, 4)),
                "**WHEN**",
                "width {width}"
            );
        }
    }

    /// `text-selection` :: "A multi-line selection joins with newlines and
    /// drops padding". The middle row is padded to the full interior width
    /// at both 78 and 58, exactly as `problem_row`/`separator_row`/`header`
    /// pad a real rendered row — and the selection spans clean through it,
    /// which is what proves the padding is dropped rather than copied.
    #[test]
    fn a_multi_line_selection_joins_with_newlines_and_drops_padding() {
        let widths: [u16; 2] = [78, 58];
        for width in widths {
            let rows = vec![
                plain_row("alpha bravo"),
                plain_row(&pad_or_truncate_right("charlie", width as usize)),
                plain_row("delta"),
            ];
            let joined = span_text(&rows, (0, 0), (2, 5));
            assert_eq!(joined, "alpha bravo\ncharlie\ndelta", "width {width}");
            assert_eq!(joined.matches('\n').count(), 2, "width {width}");
            for line in joined.split('\n') {
                assert_eq!(
                    line,
                    line.trim_end(),
                    "width {width}: {line:?} has trailing padding"
                );
            }
        }
    }

    // --- group 7: the body indent ------------------------------------

    /// `text` padded with spaces to `width`, by `std::fmt`'s own width
    /// specifier rather than by `ui::list::pad_or_truncate_right`. The two
    /// CHARACTERIZE baselines below pin a row list **byte-identical** to the
    /// one this module produced before the indent existed, and a baseline
    /// that routes its expectation through the very production helper the
    /// rows are built with could not fail if that helper moved.
    fn padded_to(text: &str, width: usize) -> String {
        format!("{text:<width$}")
    }

    /// The seven-section fixture's rows, all sections open — the row list
    /// both `artifact-folds` indent scenarios are stated over.
    fn seven_section_rows(width: u16) -> Vec<String> {
        let every = std::collections::BTreeSet::from([0, 1, 2, 3, 4, 5, 6]);
        content_lines(&seven_section_detail(every), None, width)
            .iter()
            .map(ContentRow::text)
            .collect()
    }

    /// `artifact-folds` :: "The same tab draws its bodies at column zero at
    /// the narrow interior".
    ///
    /// CHARACTERIZE, not RED: every body row already sits at column zero at
    /// HEAD, and this baseline exists to stay green. It is asserted as the
    /// whole row list byte for byte rather than as "begins at column zero",
    /// because `ui::markdown`'s own hanging indent already starts some body
    /// rows with spaces and the weaker phrasing could not tell the two apart
    /// (tasks.md 1.1).
    #[test]
    fn the_same_tab_draws_its_bodies_at_column_zero_at_the_narrow_interior() {
        // The fixture's deepest body-bearing section is depth 3, so its floor
        // is `64 + 2 * 3 = 70`: 58 is below it and 78 above, which is why this
        // list is the pre-change one and the wide-interior scenario's is not.
        assert_eq!(
            seven_section_rows(58),
            vec![
                padded_to("▾ degraded-coverage", 58),
                padded_to("  ▾ ADDED Requirements", 58),
                padded_to("    ▾ Requirement: Alpha", 58),
                "Alpha text.".to_string(),
                padded_to("", 58),
                padded_to("      ▾ Scenario: A works", 58),
                "• WHEN a".to_string(),
                "• THEN b".to_string(),
                padded_to("", 58),
                padded_to("    ▾ Requirement: Beta", 58),
                "Beta text.".to_string(),
                padded_to("", 58),
                padded_to("▾ markdown-render", 58),
                "## MODIFIED Requirements".to_string(),
                padded_to("", 58),
                padded_to("▾ tasks-checklist", 58),
                "## MODIFIED Requirements".to_string(),
            ],
        );

        // The mandated pair, named explicitly per DETAILWIDTHS: every header
        // row keeps its own `"  " * depth` indent at both interiors, which
        // this rule does not touch at either.
        for width in [78, 58] {
            let rows = seven_section_rows(width);
            for (row, depth) in [(0usize, 0usize), (1, 1), (2, 2), (5, 3), (9, 2)] {
                assert!(
                    rows[row].starts_with(&format!("{}▾ ", "  ".repeat(depth))),
                    "width {width}: row {row} ({:?}) lost its depth-{depth} header indent",
                    rows[row]
                );
            }
        }
    }

    /// A tracked-tasks `Change` whose second artifact tracks the tasks, and
    /// whose progress is `1/3` — the pair every tracked-tasks scenario in
    /// this group renders against.
    fn tracked_tasks_change() -> (crate::changes::Change, Progress) {
        let progress = Progress {
            completed: 1,
            total: 3,
        };
        let change =
            fixture::with_marked_artifacts(&paths_free(&["proposal", "tasks"]), Some(1), progress);
        (change, progress)
    }

    /// `artifact-folds` :: "A depth-0 tracked-tasks tab is unmoved at every
    /// width".
    ///
    /// CHARACTERIZE. One resolved path and every group heading at one level,
    /// so `base` is 0 and every section is at depth 0 — and the rows are
    /// asserted byte for byte, because `task-item-bodies` is rewriting this
    /// same item grammar and a regression there must not be attributed to
    /// this change. That this tab does not move is a consequence of its own
    /// depth and of no tracked-tasks exemption, which the depth-1 scenario
    /// below fixes from the other side.
    #[test]
    fn a_depth_0_tracked_tasks_tab_is_unmoved_at_every_width() {
        let (change, progress) = tracked_tasks_change();
        let d = Detail {
            sections: vec![
                ArtifactSection {
                    label: Some("1. Setup".to_string()),
                    text: "- [x] a\n- [ ] b\n".to_string(),
                    depth: 0,
                    progress: None,
                    operation: None,
                },
                ArtifactSection {
                    label: Some("2. Build".to_string()),
                    text: "- [ ] c\n".to_string(),
                    depth: 0,
                    progress: None,
                    operation: None,
                },
            ],
            scroll: 0,
            tab: 1,
            problems: Vec::new(),
            loaded: None,
            expanded: std::collections::BTreeSet::from([0, 1]),
            drawn_width: None,
        };
        for width in [120, 78, 58, 20] {
            let rows = content_lines(&d, Some(&change), width);
            assert_eq!(
                rows.iter().map(ContentRow::text).collect::<Vec<_>>(),
                vec![
                    crate::ui::tasks::progress_bar(&progress, &[], width),
                    String::new(),
                    padded_to("▾ 1. Setup", width as usize),
                    "[✓] a".to_string(),
                    "[ ] b".to_string(),
                    padded_to("", width as usize),
                    padded_to("▾ 2. Build", width as usize),
                    "[ ] c".to_string(),
                ],
                "width {width}"
            );
        }
    }

    /// A foldable tab whose sections sit at depths 0, 1, 2 and 3 with
    /// one-word bodies. The words are short enough never to wrap above a
    /// body width of eight columns, which is what lets a sweep from `0`
    /// assert a row's whole text rather than reason about where
    /// `ui::markdown` broke it. Its deepest body-bearing section is depth 3,
    /// so its floor is `64 + 2 * 3 = 70`.
    fn three_depth_detail(expanded: std::collections::BTreeSet<usize>) -> Detail {
        let section = |label: &str, text: &str, depth: usize| ArtifactSection {
            label: Some(label.to_string()),
            text: text.to_string(),
            depth,
            progress: None,
            operation: None,
        };
        Detail {
            sections: vec![
                section("degraded-coverage", "", 0),
                section("ADDED Requirements", "alpha\n", 1),
                section("Requirement: One", "bravo\n", 2),
                section("Scenario: Two", "charlie\n", 3),
            ],
            scroll: 0,
            tab: 0,
            problems: Vec::new(),
            loaded: None,
            expanded,
            drawn_width: Some(78),
        }
    }

    /// The same shape one level shallower: its deepest body-bearing section
    /// is at depth `1`, so its floor is `64 + 2 * 1 = 66`. Neither
    /// `three_spec_detail` nor `preamble_detail` can stand in — every
    /// section of both is at depth 0, and `seven_section_detail` is depths
    /// 0 to 3, whose floor of 70 is the very thing this fixture exists to
    /// differ from (tasks.md 1.3).
    fn shallow_detail() -> Detail {
        let mut d = three_depth_detail(std::collections::BTreeSet::from([0, 1]));
        d.sections.truncate(2);
        d
    }

    /// The body rows drawn beneath section `section`'s own header row, up to
    /// the next header or the blank separator that closes the body. The
    /// header is found by its `ContentKind::SectionHeader`'s index and never
    /// by its label text: a sweep from `0` reaches widths at which the label
    /// is truncated away entirely, and a text match would panic there rather
    /// than assert anything.
    fn body_rows_under(rows: &[ContentRow], section: usize) -> Vec<String> {
        let header = rows
            .iter()
            .position(
                |r| matches!(r.kind, ContentKind::SectionHeader { section: s, .. } if s == section),
            )
            .unwrap_or_else(|| panic!("section {section}'s header row is drawn"));
        rows[header + 1..]
            .iter()
            .take_while(|r| {
                !matches!(r.kind, ContentKind::SectionHeader { .. }) && !r.text().trim().is_empty()
            })
            .map(ContentRow::text)
            .collect()
    }

    /// `artifact-folds` :: "A spec tab's bodies align under their headers at
    /// the wide interior".
    ///
    /// `seven_section_detail`'s own shape — a `specs` glob resolving to more
    /// than one file, so the file sections are depth 0 and the headings
    /// beneath the first are depths 1, 2 and 3 — with two of its bodies
    /// replaced: its depth-1 section carries `"\n"`, which is non-empty (so
    /// it raises no floor question) but renders no row at all, and its
    /// depth-3 body is too short to wrap. Both clauses this scenario states
    /// would pass vacuously over the fixture unaltered.
    #[test]
    fn a_spec_tabs_bodies_align_under_their_headers_at_the_wide_interior() {
        let every = std::collections::BTreeSet::from([0, 1, 2, 3, 4, 5, 6]);
        let mut d = seven_section_detail(every);
        d.sections[1].text = "Operation prose.\n".to_string();
        d.sections[3].text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, \
             sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.\n"
            .to_string();

        let rows = content_lines(&d, None, 78);

        // Each body sits flush beneath its own header: two columns at the
        // operation heading, four at the requirement, six at the scenario.
        assert_eq!(
            body_rows_under(&rows, 1),
            vec!["  Operation prose.".to_string()]
        );
        assert_eq!(
            body_rows_under(&rows, 2),
            vec!["    Alpha text.".to_string()]
        );

        // The deepest body is *wrapped* at `78 - 6` and then indented, not
        // wrapped at 78 and prefixed — which is the whole of Decision 1, and
        // the only reading under which no row overflows the interior.
        let deep = body_rows_under(&rows, 3);
        let want: Vec<String> = crate::ui::markdown::lines(&d.sections[3].text, 78 - 6)
            .iter()
            .map(|l| format!("      {}", l.text()))
            .collect();
        assert_eq!(deep, want);
        assert!(deep.len() > 1, "the deep body does not wrap: {deep:?}");
        assert!(
            deep.iter().any(|r| columns(r) > 66),
            "no row reaches the wrap width, so the width assertion is vacuous: {deep:?}"
        );
        for row in &rows {
            assert!(columns(&row.text()) <= 78, "{:?} exceeds 78", row.text());
        }

        // The mandated pair, named explicitly per DETAILWIDTHS: the same tab
        // at the narrow interior draws every one of those bodies at column
        // zero, 58 being below this tab's floor of 70.
        let narrow = content_lines(&d, None, 58);
        for section in [1usize, 2, 3] {
            for row in body_rows_under(&narrow, section) {
                assert!(
                    !row.starts_with(' '),
                    "width 58: section {section}'s body row {row:?} is indented"
                );
            }
        }
    }

    /// `artifact-folds` :: "The indent is all-or-nothing across one render".
    #[test]
    fn the_indent_is_all_or_nothing_across_one_render() {
        let every = std::collections::BTreeSet::from([0, 1, 2, 3]);
        let d = three_depth_detail(every.clone());

        // No width panics and no row overflows, including every width below
        // `2 * max_depth`, where the floor's own subtraction saturates.
        for width in 0..=120u16 {
            for row in content_lines(&d, None, width) {
                assert!(
                    columns(&row.text()) <= width as usize,
                    "width {width}: {:?} overflows",
                    row.text()
                );
            }
        }

        // The three bodies never wrap above a body width of eight, so from
        // there up each row's whole text is assertable. At every width the
        // three agree: all indented to their own depth, or all at column
        // zero — a per-section rule would indent `alpha` at 66 and leave
        // `charlie` flush.
        let mut indented_widths: Vec<u16> = Vec::new();
        for width in 8..=120u16 {
            let rows = content_lines(&d, None, width);
            let indented = width >= 70;
            for (section, word) in [(1usize, "alpha"), (2, "bravo"), (3, "charlie")] {
                let indent = if indented {
                    "  ".repeat(section)
                } else {
                    String::new()
                };
                assert_eq!(
                    body_rows_under(&rows, section),
                    vec![format!("{indent}{word}")],
                    "width {width}: section {section}"
                );
            }
            if indented {
                indented_widths.push(width);
            }
        }
        // Exactly one transition, at `64 + 2 * 3` for this tab's max depth.
        assert_eq!(indented_widths.first().copied(), Some(70));
        assert_eq!(indented_widths.len(), (70..=120u16).len());

        // The decision reads `detail.sections`, never the visible set.
        // Collapsing the depth-3 section itself leaves every still-open body
        // unchanged; and so does collapsing its **parent**, which hides the
        // depth-3 section outright — the case that discriminates, since an
        // implementation reading `visible_sections` would then see a max
        // depth of 2, a floor of 68, and indent at 69.
        let deep_shut = content_lines(
            &three_depth_detail(std::collections::BTreeSet::from([0, 1, 2])),
            None,
            78,
        );
        for (section, word) in [(1usize, "alpha"), (2, "bravo")] {
            assert_eq!(
                body_rows_under(&deep_shut, section),
                vec![format!("{}{word}", "  ".repeat(section))],
                "collapsing the depth-3 section moved section {section}'s body"
            );
        }
        let subtree_shut = content_lines(
            &three_depth_detail(std::collections::BTreeSet::from([0, 1])),
            None,
            69,
        );
        assert_eq!(
            body_rows_under(&subtree_shut, 1),
            vec!["alpha".to_string()],
            "the floor read the visible set rather than detail.sections"
        );

        // The mandated pair, named explicitly per DETAILWIDTHS.
        for (width, indent) in [(78u16, "      "), (58, "")] {
            assert_eq!(
                body_rows_under(&content_lines(&d, None, width), 3),
                vec![format!("{indent}charlie")],
                "width {width}"
            );
        }
    }

    /// `artifact-folds` :: "A shallower tab indents at a narrower width".
    #[test]
    fn a_shallower_tab_indents_at_a_narrower_width() {
        let shallow = shallow_detail();
        let deep = three_depth_detail(std::collections::BTreeSet::from([0, 1, 2, 3]));
        for (width, indent) in [(67u16, "  "), (66, "  "), (65, "")] {
            assert_eq!(
                body_rows_under(&content_lines(&shallow, None, width), 1),
                vec![format!("{indent}alpha")],
                "width {width}: the floor is 64 + 2 * 1 = 66"
            );
            // The same three widths leave a depth-3 tab flush throughout, its
            // own floor being 70 — so the rule reads the tab's maximum depth
            // rather than one fixed width.
            assert_eq!(
                body_rows_under(&content_lines(&deep, None, width), 1),
                vec!["alpha".to_string()],
                "width {width}: a depth-3 tab indented below its own floor of 70"
            );
        }

        // The mandated pair, named explicitly per DETAILWIDTHS: 78 clears
        // the shallow tab's floor of 66 and 58 does not.
        for (width, indent) in [(78u16, "  "), (58, "")] {
            assert_eq!(
                body_rows_under(&content_lines(&shallow, None, width), 1),
                vec![format!("{indent}alpha")],
                "width {width}"
            );
        }
    }

    /// `artifact-folds` :: "A depth-1 tracked-tasks tab indents its items
    /// like any other tab".
    ///
    /// The shape a task file opening with a level-1 title produces:
    /// `min_level` is 1, so every `## ` group lands at `base + 1` — 17 of
    /// this repository's own 44 task files. Its floor is `64 + 2 * 1 = 66`.
    #[test]
    fn a_depth_1_tracked_tasks_tab_indents_its_items_like_any_other_tab() {
        let (change, progress) = tracked_tasks_change();
        let group_text = "- [x] 1.1 first\n- [ ] 1.2 second\n";
        let d = Detail {
            sections: vec![
                ArtifactSection {
                    label: Some("Tasks".to_string()),
                    text: String::new(),
                    depth: 0,
                    progress: None,
                    operation: None,
                },
                ArtifactSection {
                    label: Some("1. Setup".to_string()),
                    text: group_text.to_string(),
                    depth: 1,
                    progress: None,
                    operation: None,
                },
                ArtifactSection {
                    label: Some("2. Build".to_string()),
                    text: "- [ ] 2.1 third\n".to_string(),
                    depth: 1,
                    progress: None,
                    operation: None,
                },
            ],
            scroll: 0,
            tab: 1,
            problems: Vec::new(),
            loaded: None,
            expanded: std::collections::BTreeSet::from([0, 1, 2]),
            drawn_width: None,
        };
        let parsed: Vec<crate::tasks::Item> = crate::tasks::parse(group_text)
            .groups
            .into_iter()
            .flat_map(|g| g.items)
            .collect();
        for (width, indent) in [(78u16, "  "), (58, "")] {
            let rows = content_lines(&d, Some(&change), width);
            // The items are `ui::tasks::items` at the reduced width, then
            // indented — at 78 wrapped at 76, at 58 at 58.
            let body_width = width - columns(indent) as u16;
            let want: Vec<String> = crate::ui::tasks::items(&parsed, body_width)
                .iter()
                .map(|l| format!("{indent}{}", l.text()))
                .collect();
            assert_eq!(body_rows_under(&rows, 1), want, "width {width}");

            // The group headers keep their own `"  " * depth` indent, and the
            // progress-bar rows above every header stay at column zero, owned
            // by no section.
            assert_eq!(
                rows.iter().map(ContentRow::text).next(),
                Some(crate::ui::tasks::progress_bar(&progress, &[], width)),
                "width {width}: the bar moved"
            );
            for label in ["1. Setup", "2. Build"] {
                let header_row = rows
                    .iter()
                    .find(|r| {
                        matches!(r.kind, ContentKind::SectionHeader { .. })
                            && r.text().contains(label)
                    })
                    .expect("the group header is drawn");
                assert!(
                    header_row.text().starts_with("  ▾ "),
                    "width {width}: {:?} lost its depth-1 header indent",
                    header_row.text()
                );
            }
        }
    }

    /// `artifact-folds` :: "A selection over an indented body row copies the
    /// indent".
    #[test]
    fn a_selection_over_an_indented_body_row_copies_the_indent() {
        let d = three_depth_detail(std::collections::BTreeSet::from([0, 1, 2, 3]));
        for (width, indent) in [(78u16, "      "), (58, "")] {
            let rows = content_lines(&d, None, width);
            let line = rows
                .iter()
                .position(|r| r.text().trim() == "charlie")
                .expect("the depth-3 body row is drawn");
            assert_eq!(
                rows[line].text(),
                format!("{indent}charlie"),
                "width {width}"
            );
            assert_eq!(
                span_text(&rows, (line, 0), (line, width)),
                format!("{indent}charlie"),
                "width {width}: the indent is rendered content, not trailing padding"
            );
        }

        // At 78 a double click inside the indent's own six columns selects
        // nothing, those cells holding only whitespace.
        let rows = content_lines(&d, None, 78);
        let line = rows
            .iter()
            .position(|r| r.text().trim() == "charlie")
            .expect("the depth-3 body row is drawn");
        for column in 0..6u16 {
            assert_eq!(word_at(&rows, line, column), None, "column {column}");
        }
        assert_eq!(word_at(&rows, line, 6), Some((6, 13)));
    }
}
