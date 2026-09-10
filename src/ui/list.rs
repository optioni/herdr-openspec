//! The row grammar: every function here is a pure total transformation,
//! parameterised by an interior width, with no ratatui styling. See
//! `openspec/changes/list-view/specs/change-rows/spec.md` and design.md ->
//! Decisions ("The row grammar is fixed-field and right-aligned").

use crate::ui::app::{Dashboard, SectionKey, Target, matches};
use crate::ui::layout::{columns, truncate_columns};

/// What kind of thing a `Row` represents, so a caller can tell a change row
/// from a section header, a message, or a repository-level problem without
/// parsing its text. `Item`'s `index` is into the *visible* list `rows`
/// emits — active then archived, filtered — never into `ChangeSet` itself.
/// `Section` replaces `list-view`'s unaddressable separator: `list-sections`
/// folds each tier behind its own header, carrying the `key` `Space` toggles,
/// a nesting `depth` — always `0` today, reserved for a later date grouping
/// under `archived` (design.md -> Decision 8) — and whether it is currently
/// collapsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    Problem,
    Section {
        key: SectionKey,
        depth: u8,
        collapsed: bool,
    },
    Item {
        index: usize,
    },
    Message,
}

/// Where a row's agent badge sits, and what status it was derived from.
/// `x` is the badge character's **display-column offset from the interior's
/// first column**, so `text[x]` is that character; `status` is carried
/// rather than re-parsed from the glyph, since `w`, `i`, `b`, `d`, and `?`
/// all occur in change names. Plain data: no ratatui type, so `ui::view`
/// alone decides what a badge looks like (`color-palette` design.md ->
/// Decision 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BadgeCell {
    pub x: u16,
    pub status: crate::agents::AgentStatus,
}

/// One drawn row: its text, exactly `width` display columns; its kind;
/// whether it carries the selection marker; and, when the row's text
/// carries a badge cell that was not dropped, where that one column is.
/// `list.rs` never styles a row — `ui::view` applies `Modifier::BOLD` to
/// the selected one and the badge's own colour to [`BadgeCell::x`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub text: String,
    pub kind: RowKind,
    pub selected: bool,
    pub badge: Option<BadgeCell>,
}

/// The progress cell: `[<completed>/<total>]`, or the three characters
/// `[-]` when `total` is zero, so a change with no tasks still ends its row
/// in the same column as one that has them. `pub(crate)` rather than
/// private: `ui::detail::header_row` calls this rather than copying it, so
/// the crate keeps exactly one progress-cell implementation.
pub(crate) fn progress_cell(progress: &crate::tasks::Progress) -> String {
    if progress.total == 0 {
        "[-]".to_string()
    } else {
        format!("[{}/{}]", progress.completed, progress.total)
    }
}

/// Pad `text` with trailing spaces to `width` **display columns** when it
/// fits; otherwise [`truncate_columns`] to `width - 1` columns and append
/// `…`, then pad the result back to exactly `width` columns — the crate's
/// one right-truncation implementation, shared by the name field, the
/// problem row, and the message rows. The trailing pad in the truncating
/// arm is load-bearing: `truncate_columns` drops a grapheme cluster whole,
/// so the prefix can measure `width - 2` when the dropped cluster was two
/// columns wide, and without the pad the row would be one column short of
/// the interior and leave a stale cell behind it. See design.md ->
/// Decision 3. `width == 0` truncates to the empty string: there is no room
/// even for the ellipsis. `pub(crate)` rather than private: `ui::detail`'s
/// header, tab bar, and problem-line grammar calls this rather than
/// copying it.
pub(crate) fn pad_or_truncate_right(text: &str, width: usize) -> String {
    let measured = columns(text);
    if measured <= width {
        let mut s = text.to_string();
        s.push_str(&" ".repeat(width - measured));
        return s;
    }
    if width == 0 {
        return String::new();
    }
    let mut s = format!("{}…", truncate_columns(text, width - 1));
    let drawn = columns(&s);
    if drawn < width {
        s.push_str(&" ".repeat(width - drawn));
    }
    s
}

/// The keep-the-tail truncation `change-rows`' no-repository block and
/// `responsive-layout`'s header share: whole when `text` fits in `width`
/// display columns, else `…` followed by the longest suffix — ending on a
/// grapheme-cluster boundary — whose columns are at most `width - 1`,
/// empty at `width == 0`. Does **not** pad — callers that need a
/// full-width row pad the result themselves, since the header uses this
/// un-padded (it right-aligns within its own remaining space).
///
/// Reaches the crate's one display-width measure only through [`columns`]
/// and [`truncate_columns`], never through a grapheme split of its own: the
/// suffix's own boundary is found by growing a `truncate_columns` prefix
/// budget one column at a time until the dropped prefix's columns reach the
/// amount that must be dropped, so the cut this returns always lands on the
/// same cluster boundary `truncate_columns` itself would have chosen.
pub(crate) fn shorten_left(text: &str, width: usize) -> String {
    let total = columns(text);
    if total <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let must_drop = total - (width - 1);
    let mut probe = must_drop;
    loop {
        let prefix = truncate_columns(text, probe);
        if columns(prefix) >= must_drop || prefix.len() == text.len() {
            return format!("…{}", &text[prefix.len()..]);
        }
        probe += 1;
    }
}

/// `shorten_left`, then padded with trailing spaces to `width` display
/// columns — the no-repository block's third row needs a full-width row
/// like every other kind, unlike the header's un-padded, right-aligned use
/// of the same rule. A third measuring site, named here because it is easy
/// to miss beside its un-padded sibling.
fn shorten_left_row(text: &str, width: usize) -> String {
    let shortened = shorten_left(text, width);
    let len = columns(&shortened);
    if len >= width {
        shortened
    } else {
        format!("{shortened}{}", " ".repeat(width - len))
    }
}

/// The active-change grammar: `[marker][space][name field][space][badge]
/// [space][progress]`, with the badge cell offered only together with the
/// progress cell and dropped whole first (reclaiming its own separating
/// space) as soon as the name field would fall below one column with it;
/// the progress cell (and its separating space) drops next on the same
/// condition; and the row degenerates to the first `width` display columns
/// of `"{marker} "` below two columns. `badge` `None` reproduces exactly
/// today's `[marker][space][name field][space][progress]` grammar, byte
/// for byte — see `agent-attribution` -> Decisions 3. Shared,
/// unparameterised by date, by both the active row and the final
/// degenerate branch of the archived row.
///
/// Returns the row's text together with the badge's own column, `None`
/// whenever no badge was offered or the badge cell was dropped whole — the
/// column is *reported* from the arithmetic that already placed the badge,
/// never decided a second time (`color-palette` design.md -> Decision 4).
fn active_style_row(
    marker: char,
    name: &str,
    progress: Option<&str>,
    badge: Option<char>,
    width: u16,
) -> (String, Option<u16>) {
    let w = i64::from(width);
    if w < 2 {
        let head = format!("{marker} ");
        return (truncate_columns(&head, w.max(0) as usize).to_string(), None);
    }
    if let Some(progress) = progress {
        let progress_len = columns(progress) as i64;
        if let Some(badge) = badge {
            let name_field_w = w - 2 - 1 - 1 - 1 - progress_len;
            if name_field_w >= 1 {
                let name_field = pad_or_truncate_right(name, name_field_w as usize);
                return (
                    format!("{marker} {name_field} {badge} {progress}"),
                    Some(badge_column(2, name_field_w)),
                );
            }
        }
        let name_field_w = w - 2 - 1 - progress_len;
        if name_field_w >= 1 {
            let name_field = pad_or_truncate_right(name, name_field_w as usize);
            return (format!("{marker} {name_field} {progress}"), None);
        }
    }
    let name_field_w = (w - 2) as usize;
    let name_field = pad_or_truncate_right(name, name_field_w);
    (format!("{marker} {name_field}"), None)
}

/// The archived-change grammar: `[marker][space][date field: 10][space]`
/// then the active grammar's `[name field][space][badge][space][progress]`,
/// with the date field ten spaces when `date` is `None`. Dropped whole, in
/// order — the badge cell (and its separating space) first, then the
/// progress cell (and its separating space), then the date field (and its
/// separating space), then degenerating to `active_style_row` with neither
/// a badge nor a progress cell ever offered — exactly as `change-rows`'
/// "Archived changes sit below a separator" requirement states.
///
/// Returns the row's text together with the badge's own column, on exactly
/// [`active_style_row`]'s terms.
fn archived_row_text(
    marker: char,
    date: Option<&str>,
    name: &str,
    progress: &str,
    badge: Option<char>,
    width: u16,
) -> (String, Option<u16>) {
    let w = i64::from(width);
    let date_field = date.map_or_else(|| " ".repeat(10), str::to_string);
    let progress_len = columns(progress) as i64;

    // Full form with the badge: marker + space + date(10) + space + name +
    // space + badge + space + progress.
    if let Some(badge) = badge {
        let name_field_w = w - 14 - 1 - 1 - progress_len;
        if name_field_w >= 1 {
            let name_field = pad_or_truncate_right(name, name_field_w as usize);
            return (
                format!("{marker} {date_field} {name_field} {badge} {progress}"),
                Some(badge_column(13, name_field_w)),
            );
        }
    }

    // Full form without the badge (dropped, or never offered): marker +
    // space + date(10) + space + name + space + progress.
    let name_field_full = w - 14 - progress_len;
    if name_field_full >= 1 {
        let name_field = pad_or_truncate_right(name, name_field_full as usize);
        return (
            format!("{marker} {date_field} {name_field} {progress}"),
            None,
        );
    }

    // Drop the progress cell and its separating space: marker + space +
    // date(10) + space + name.
    let name_field_no_progress = w - 13;
    if name_field_no_progress >= 1 {
        let name_field = pad_or_truncate_right(name, name_field_no_progress as usize);
        return (format!("{marker} {date_field} {name_field}"), None);
    }

    // Drop the date field too: degenerate to the active grammar, with
    // neither a badge nor a progress cell ever offered.
    active_style_row(marker, name, None, None, width)
}

/// The badge's display column: the columns preceding the name field —
/// `2` on an active row (the marker and its space), `13` on an archived one
/// (those two plus the ten-column date field and its space) — then the name
/// field itself, then its one separating space. One helper rather than two
/// expressions, because the two grammars must agree: both put the badge two
/// columns left of the progress cell, which is exactly what
/// `a_badged_archived_row_reports_the_column_its_badge_occupies` asserts by
/// comparing the two.
fn badge_column(prefix: i64, name_field_w: i64) -> u16 {
    (prefix + name_field_w + 1) as u16
}

/// A `Problem` row: `! `, then the text, truncated with `…` when the width
/// falls below three columns to the first `width` display columns of
/// `"! "` itself, exactly as `change-rows`' empty-state requirement states.
fn problem_row_text(text: &str, width: u16) -> String {
    let w = width as usize;
    if w < 3 {
        return truncate_columns("! ", w).to_string();
    }
    format!("! {}", pad_or_truncate_right(text, w - 2))
}

/// A `Message` row: the whole width, no prefix, padded or truncated by the
/// shared right-truncation rule.
fn message_row_text(text: &str, width: u16) -> String {
    pad_or_truncate_right(text, width as usize)
}

/// A section header row: `[marker][space][glyph][space][label][space]
/// [(count)]`, where `marker` is `>` when the header carries the cursor and
/// a space otherwise — the same column every other row's selection marker
/// occupies — and `glyph` is `v` when the section is open and `>` when it
/// is collapsed. The `>` glyph and the `>` selection marker collide on a
/// selected collapsed section (`> > archived (22)`); that collision is
/// accepted rather than avoided, since column 0 is the cursor on every row
/// of this list and column 2 is the fold state on section rows alone (see
/// `specs/change-rows/spec.md` and design.md -> Decision 4).
///
/// Below two columns this degenerates the same way [`active_style_row`]
/// does: the first `width` display columns of `"{marker} "` alone, with no
/// glyph, label, or count ever offered — there is no room for them and no
/// room to say so with an ellipsis either.
fn section_row_text(
    selected: bool,
    collapsed: bool,
    label: &str,
    count: usize,
    width: u16,
) -> String {
    let marker = if selected { '>' } else { ' ' };
    let w = i64::from(width);
    if w < 2 {
        let head = format!("{marker} ");
        return truncate_columns(&head, w.max(0) as usize).to_string();
    }
    let glyph = if collapsed { '>' } else { 'v' };
    let text = format!("{marker} {glyph} {label} ({count})");
    pad_or_truncate_right(&text, width as usize)
}

fn no_repo_rows(searched_from: &std::path::Path, width: u16) -> Vec<Row> {
    let path_text = searched_from.display().to_string();
    vec![
        Row {
            text: message_row_text("No OpenSpec repository found", width),
            kind: RowKind::Message,
            selected: false,
            badge: None,
        },
        Row {
            text: message_row_text("searched from:", width),
            kind: RowKind::Message,
            selected: false,
            badge: None,
        },
        Row {
            text: shorten_left_row(&path_text, width as usize),
            kind: RowKind::Message,
            selected: false,
            badge: None,
        },
    ]
}

/// The list region's content: every row, in the order `change-rows` states
/// — problem rows, then the active section header (only when its count is
/// greater than zero) followed by the visible active changes or the
/// empty-state message for the state the dashboard is in, then the archived
/// section header on the same condition followed by the visible archived
/// changes. Pure and total over every `Dashboard` value and every `u16`
/// width, including `0`: never panics, performs no I/O, reads no clock and
/// no global state. See `specs/change-rows/spec.md`.
pub fn rows(dashboard: &Dashboard, width: u16) -> Vec<Row> {
    if dashboard.repo.is_none() {
        return no_repo_rows(&dashboard.searched_from, width);
    }

    let attribution = dashboard.attribution();
    let query = dashboard.filter.query.as_str();
    let active: Vec<&crate::changes::Change> = dashboard
        .changes
        .active
        .iter()
        .filter(|c| matches(&c.name, query))
        .collect();
    let archived: Vec<&crate::changes::Change> = dashboard
        .changes
        .archived
        .iter()
        .filter(|c| matches(&c.name, query))
        .collect();

    // `list-sections` design.md -> Decision 10: the active tier is always fully
    // resolved, so its count is simply the matched entries. The archived tier can be
    // **unresolved** — `changes.archived` empty while `archived_total` is greater than
    // zero, the one-cycle window between opening the section and the refresh that
    // resolves it — in which case the header's own count falls through to the true
    // total rather than the (necessarily zero) matched count.
    let active_count = active.len();
    let archived_unresolved =
        dashboard.changes.archived.is_empty() && dashboard.changes.archived_total > 0;
    let archived_display_count = if archived_unresolved {
        dashboard.changes.archived_total
    } else {
        archived.len()
    };
    // The second correction to Decision 10: during that same one-cycle window, under a
    // non-empty query, nothing is yet known about whether any of the unresolved
    // archived changes will actually match — so the choice between `No active changes`
    // and the two `No changes match` rows below treats that window as zero matches,
    // even though the header above still carries the true total. With an empty query
    // an unresolved archive is known to match everything once it resolves, so this
    // count agrees with `archived_display_count` in every other case.
    let archived_message_count = if archived_unresolved && !query.is_empty() {
        0
    } else {
        archived_display_count
    };

    let mut out = Vec::new();
    // The one problem-row constructor every source below shares: `! `-prefixed, padded or
    // truncated to the interior width, never addressable by `selected`. Collapsing the five
    // call sites into this closure is `seam-resilience`'s own REFACTOR step — each source
    // still gets its own doc comment explaining *why* it sits where it does, since that
    // reasoning differs source to source even though the row it produces does not.
    let push_problem = |out: &mut Vec<Row>, text: &str| {
        out.push(Row {
            text: problem_row_text(text, width),
            kind: RowKind::Problem,
            selected: false,
            badge: None,
        });
    };
    // `agent-launch`: launch problems lead the whole list, ahead of even refresh problems —
    // they are the only rows that answer a key the reader has just pressed, and burying the
    // reply under a standing condition is how a reader concludes the key did nothing.
    // `launch.problems` holds at most **two** entries (`degraded-states`' repair of row 23: a
    // recording failure and a prompt failure can co-occur) and is replaced wholesale, so this
    // costs at most two rows.
    for problem in &dashboard.launch.problems {
        push_problem(&mut out, problem);
    }
    // `seam-resilience`: a **stalled** agent snapshot's reason sits directly below the
    // launch problems, because — like them — it explains why a key the reader just pressed
    // did nothing: the action keys are offered only while reachable, and a socket that has
    // stopped answering entirely withdraws them along with the badges. A non-stalled
    // snapshot's `problem` (an unreachable or erroring socket) stays silent, on the
    // documented standalone-TUI terms.
    if dashboard.agents.stalled
        && let Some(reason) = dashboard.agents.problem.as_deref()
    {
        push_problem(&mut out, reason);
    }
    // `seam-resilience`: `refresh.startup` precedes `refresh.problems` because it is the
    // older and more general fact — a missing `openspec` binary explains the whole session,
    // a watcher error explains this moment. Written once by `run_wired` and never replaced,
    // so it cannot be erased by the first watcher error the way `refresh.problems` is.
    for problem in &dashboard.refresh.startup {
        push_problem(&mut out, problem);
    }
    // `live-refresh`: refresh problems (a watcher that would not start, or a
    // drain error) lead the list, ahead of change-set problems — they are
    // the ones that outlive a reload, while a `ChangeSet` problem is
    // re-derived every cycle and may vanish on the next one. Same grammar,
    // same row kind: `change-rows` requires exactly one problem kind, so
    // neither is addressable by `selected` and `ui::view` styles both alike.
    for problem in &dashboard.refresh.problems {
        push_problem(&mut out, problem);
    }
    for problem in &dashboard.changes.problems {
        push_problem(&mut out, problem);
    }

    // `targets()` is the index space `dashboard.selected` addresses (design.md ->
    // Decision 2 and Decision 15): a section header's own position, or
    // `Target::Change(i)` where `i` is exactly the running `visible()`-order counter
    // below. Resolving it once, here, is what lets every push site below decide its own
    // marker by comparison rather than by re-deriving the index space.
    let selected_target = dashboard.targets().get(dashboard.selected).copied();
    let mut index = 0usize;

    if active_count > 0 {
        let active_open = dashboard.section_open(SectionKey::Active);
        let header_selected = selected_target == Some(Target::Section(SectionKey::Active));
        out.push(Row {
            text: section_row_text(header_selected, !active_open, "active", active_count, width),
            kind: RowKind::Section {
                key: SectionKey::Active,
                depth: 0,
                collapsed: !active_open,
            },
            selected: header_selected,
            badge: None,
        });
        if active_open {
            for change in &active {
                let selected = selected_target == Some(Target::Change(index));
                let marker = if selected { '>' } else { ' ' };
                let status = attribution.badges.get(&change.name).copied();
                let (text, badge_x) = active_style_row(
                    marker,
                    &change.name,
                    Some(&progress_cell(&change.progress)),
                    status.map(badge_char),
                    width,
                );
                out.push(Row {
                    text,
                    kind: RowKind::Item { index },
                    selected,
                    badge: badge_x
                        .zip(status)
                        .map(|(x, status)| BadgeCell { x, status }),
                });
                index += 1;
            }
        }
    } else if archived_message_count > 0 {
        // `list-sections` design.md -> Decision 10's second correction: keyed on the
        // archived section's own count, never on whether its rows are actually drawn,
        // so a collapsed-but-populated archive reads `No active changes` rather than
        // the stronger `No changes yet`/`No changes match` this active tier alone earns.
        out.push(Row {
            text: message_row_text("No active changes", width),
            kind: RowKind::Message,
            selected: false,
            badge: None,
        });
    } else if query.is_empty() {
        out.push(Row {
            text: message_row_text("No changes yet", width),
            kind: RowKind::Message,
            selected: false,
            badge: None,
        });
    } else {
        out.push(Row {
            text: message_row_text("No changes match", width),
            kind: RowKind::Message,
            selected: false,
            badge: None,
        });
        out.push(Row {
            text: message_row_text(&format!("/{query}"), width),
            kind: RowKind::Message,
            selected: false,
            badge: None,
        });
    }

    if archived_display_count > 0 {
        let archived_open = dashboard.section_open(SectionKey::Archived);
        let header_selected = selected_target == Some(Target::Section(SectionKey::Archived));
        out.push(Row {
            text: section_row_text(
                header_selected,
                !archived_open,
                "archived",
                archived_display_count,
                width,
            ),
            kind: RowKind::Section {
                key: SectionKey::Archived,
                depth: 0,
                collapsed: !archived_open,
            },
            selected: header_selected,
            badge: None,
        });
        if archived_open {
            for change in &archived {
                let selected = selected_target == Some(Target::Change(index));
                let marker = if selected { '>' } else { ' ' };
                let date = match &change.origin {
                    crate::changes::Origin::Archived { date } => date.as_deref(),
                    crate::changes::Origin::Active => None,
                };
                let status = attribution.badges.get(&change.name).copied();
                let (text, badge_x) = archived_row_text(
                    marker,
                    date,
                    &change.name,
                    &progress_cell(&change.progress),
                    status.map(badge_char),
                    width,
                );
                out.push(Row {
                    text,
                    kind: RowKind::Item { index },
                    selected,
                    badge: badge_x
                        .zip(status)
                        .map(|(x, status)| BadgeCell { x, status }),
                });
                index += 1;
            }
        }
    }

    out
}

/// The rows `render_list` draws into `interior`, paired with the index of the
/// first one — `layout::viewport`'s answer for the emitted row count, the
/// cursor's position among them, and the interior's own height.
///
/// `mouse-input`'s extraction of the three lines `ui::view::render_list`
/// performed inline. It exists exactly once so the draw path and
/// [`row_at`] cannot derive a different first row: a second derivation could
/// drift from the first by a resize, a filter edit, or a fold, and a click
/// would then land on a row the reader is not looking at. Pure and total over
/// every `Dashboard` and every `Rect`.
pub fn drawn_rows(dashboard: &Dashboard, interior: ratatui::layout::Rect) -> (Vec<Row>, usize) {
    let rows = rows(dashboard, interior.width);
    let cursor = rows.iter().position(|r| r.selected).unwrap_or(0);
    let offset = crate::ui::layout::viewport(rows.len(), cursor, interior.height);
    (rows, offset)
}

/// The [`RowKind`] of the row `ui::view::render_list` draws at
/// `interior.y + row`, and `None` when that terminal row holds no drawn row —
/// an interior shorter than the row offset, a zero-width or zero-height
/// interior, or a row past the end of what [`rows`] emitted.
///
/// Pure and total: no I/O, no clock, no mutation, and no panic for any
/// `Dashboard`, any `Rect`, and any `row`.
///
/// `RowKind::Problem` and `RowKind::Message` rows stay **unaddressable** — but
/// not here. This reports them faithfully, because they are what is drawn
/// there; `ui::driver::mouse_action` is what refuses to act on them, so the row
/// grammar gains no notion of clickability (design.md -> Decision 11).
pub fn row_at(dashboard: &Dashboard, interior: ratatui::layout::Rect, row: u16) -> Option<RowKind> {
    if interior.width == 0 || interior.height == 0 || row >= interior.height {
        return None;
    }
    let (rows, offset) = drawn_rows(dashboard, interior);
    rows.get(offset + row as usize).map(|r| r.kind)
}

/// The badge glyph for `status` — one ASCII column, collision-free against
/// every other glyph this row grammar uses (`>`, `!`, `…`, `-`, `[`). See
/// `specs/change-rows/spec.md` -> "The agent badge".
fn badge_char(status: crate::agents::AgentStatus) -> char {
    match status {
        crate::agents::AgentStatus::Working => 'w',
        crate::agents::AgentStatus::Idle => 'i',
        crate::agents::AgentStatus::Blocked => 'b',
        crate::agents::AgentStatus::Done => 'd',
        crate::agents::AgentStatus::Unknown => '?',
    }
}

#[cfg(test)]
mod tests {
    use crate::agents::{Agent, AgentStatus};
    use crate::changes::fixture;
    use crate::ui::app::{Dashboard, Detail, Filter, Route, SectionKey};
    use crate::ui::layout::columns;
    use crate::ui::list::{Row, RowKind, problem_row_text, rows};

    /// `mouse-input`: the row a point lands on is the row drawn there. Every
    /// expected offset here is derived through `layout::viewport` independently,
    /// never from `row_at`'s own answer.
    mod row_at {
        use ratatui::layout::Rect;

        use crate::changes::fixture;
        use crate::ui::app::SectionKey;
        use crate::ui::layout::viewport;
        use crate::ui::list::{RowKind, row_at, rows};

        /// Six active changes and six archived ones, the cursor on the last
        /// target — more rows than a short interior can hold, which is what makes
        /// the scrolled slice observable.
        fn crowded() -> crate::ui::app::Dashboard {
            let active: Vec<_> = ["a1", "a2", "a3", "a4", "a5", "a6"]
                .iter()
                .map(|n| fixture::active(n, 0, 1))
                .collect();
            let archived: Vec<_> = ["z1", "z2", "z3", "z4", "z5", "z6"]
                .iter()
                .map(|n| fixture::archived(Some("2026-01-01"), n, 1, 1))
                .collect();
            let mut dashboard = super::dashboard_with(active, archived, Vec::new(), 0);
            dashboard.selected = dashboard.targets().len() - 1;
            dashboard
        }

        #[test]
        fn the_reported_row_follows_the_scrolled_slice() {
            let dashboard = crowded();
            // Both mandated list interiors — 38 columns from the wide layout's
            // Length(40) column, 58 from the narrow 60-column frame — against a
            // six-row interior and fourteen rows, so the slice is scrolled.
            let widths: [u16; 2] = [38, 58];
            for width in widths {
                let interior = Rect::new(1, 2, width, 6);
                let all = rows(&dashboard, interior.width);
                let cursor = all
                    .iter()
                    .position(|r| r.selected)
                    .expect("the cursor is on a drawn row");
                let offset = viewport(all.len(), cursor, interior.height);
                assert!(
                    offset > 0,
                    "width {width}: the fixture must actually scroll"
                );

                assert_eq!(row_at(&dashboard, interior, 0), Some(all[offset].kind));
                assert_ne!(
                    row_at(&dashboard, interior, 0),
                    Some(all[0].kind),
                    "width {width}: row 0 reports the first row of the scrolled slice, not \
                     of the emitted list"
                );
                for row in 0..interior.height {
                    assert_eq!(
                        row_at(&dashboard, interior, row),
                        all.get(offset + row as usize).map(|r| r.kind),
                        "width {width} offset {row}"
                    );
                }
            }
        }

        #[test]
        fn a_collapsed_section_reports_only_its_header() {
            let mut dashboard = crowded();
            dashboard.selected = 0;
            dashboard.sections.collapsed.insert(SectionKey::Archived);
            let widths: [u16; 2] = [38, 58];
            for width in widths {
                let interior = Rect::new(1, 2, width, 36);
                let all = rows(&dashboard, interior.width);
                let header = all
                    .iter()
                    .position(|r| {
                        matches!(
                            r.kind,
                            RowKind::Section {
                                key: SectionKey::Archived,
                                ..
                            }
                        )
                    })
                    .expect("the archived header is drawn");

                assert!(matches!(
                    row_at(&dashboard, interior, header as u16),
                    Some(RowKind::Section {
                        key: SectionKey::Archived,
                        collapsed: true,
                        ..
                    })
                ));
                // Nothing behind it: the six archived changes contribute no `Item`
                // row, so every reported `Item` index belongs to an active change.
                let active_visible = 6usize;
                for row in 0..interior.height {
                    if let Some(RowKind::Item { index }) = row_at(&dashboard, interior, row) {
                        assert!(
                            index < active_visible,
                            "width {width} offset {row} reached archived change {index} \
                             behind a collapsed section"
                        );
                    }
                }
            }
        }

        #[test]
        fn degenerate_interiors_report_nothing() {
            let with_changes = crowded();
            let empty = super::dashboard_with(Vec::new(), Vec::new(), Vec::new(), 0);
            let mut no_repo = empty.clone();
            no_repo.repo = None;

            for dashboard in [&with_changes, &empty, &no_repo] {
                // The zero-width and zero-height cases, including both mandated
                // list interiors — 38 and 58 columns — reduced to zero height,
                // which `render_list` also draws nothing into.
                for interior in [
                    Rect::new(1, 2, 0, 0),
                    Rect::new(1, 2, 1, 0),
                    Rect::new(1, 2, 0, 1),
                    Rect::new(1, 2, 38, 0),
                    Rect::new(1, 2, 58, 0),
                ] {
                    for row in [0u16, 1, 65535] {
                        assert_eq!(
                            row_at(dashboard, interior, row),
                            None,
                            "{interior:?} draws nothing, so it addresses nothing"
                        );
                    }
                }
                // A 1x1 interior draws exactly one row and addresses only it.
                let one = Rect::new(1, 2, 1, 1);
                assert!(row_at(dashboard, one, 0).is_some());
                assert_eq!(row_at(dashboard, one, 1), None);
                assert_eq!(row_at(dashboard, one, 65535), None);
            }
        }
    }

    /// An in-scope `Agent` named `name` at `/tmp/demo-repo` — this module's fixture
    /// dashboards' repository root — carrying `status`. `agent-attribution`'s own
    /// tests build agents this way rather than through a `Change` literal, on
    /// exactly `NOLIT-CHANGE`'s terms.
    fn agent_at(name: &str, status: AgentStatus) -> Agent {
        Agent {
            name: Some(name.to_string()),
            kind: None,
            status,
            cwd: Some(std::path::PathBuf::from("/tmp/demo-repo")),
            pane_id: "p".to_string(),
            tab_id: "t".to_string(),
            workspace_id: "w".to_string(),
            terminal_title: None,
        }
    }

    fn empty_filter() -> Filter {
        Filter {
            query: String::new(),
            active: false,
        }
    }

    fn empty_detail() -> Detail {
        Detail {
            source: String::new(),
            scroll: 0,
            tab: 0,
            problems: Vec::new(),
            loaded: None,
        }
    }

    fn dashboard_with(
        active: Vec<crate::changes::Change>,
        archived: Vec<crate::changes::Change>,
        problems: Vec<String>,
        selected: usize,
    ) -> Dashboard {
        Dashboard {
            repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
            searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
            changes: fixture::set(active, archived, problems),
            route: Route::List,
            quit: false,
            selected,
            filter: empty_filter(),
            detail: empty_detail(),
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
            // `list-sections`: compile-forced by `Dashboard`'s new field; group 5 owns
            // this file's real work.
            sections: crate::ui::app::Sections {
                collapsed: std::collections::BTreeSet::new(),
            },
            file_mode: false,
        }
    }

    fn three_active() -> Dashboard {
        dashboard_with(
            vec![
                fixture::active("add-token-refresh", 4, 9),
                fixture::active("fix-empty-basket", 7, 7),
                fixture::active("migrate-ai-sdk-v7", 0, 0),
            ],
            Vec::new(),
            Vec::new(),
            // `list-sections`: `selected` **1** addresses `Target::Change(0)` —
            // `add-token-refresh`, the first change — since the active section
            // header is target 0. Kept marking the same change this fixture always
            // marked.
            1,
        )
    }

    /// Every dashboard fixture this module's tests build, for
    /// `rows_never_panic_at_any_width` and `every_row_is_exactly_the_requested_width`.
    fn every_fixture() -> Vec<Dashboard> {
        vec![
            three_active(),
            dashboard_with(
                vec![fixture::active("alpha", 4, 9)],
                Vec::new(),
                Vec::new(),
                0,
            ),
            dashboard_with(
                Vec::new(),
                vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
                Vec::new(),
                0,
            ),
            dashboard_with(
                vec![fixture::active("fix-empty-basket", 7, 7)],
                vec![
                    fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                    fixture::archived(None, "legacy-cleanup", 3, 3),
                ],
                Vec::new(),
                0,
            ),
            dashboard_with(Vec::new(), Vec::new(), Vec::new(), 0),
            {
                let mut d = dashboard_with(Vec::new(), Vec::new(), Vec::new(), 0);
                d.filter.query = "zzz".to_string();
                d
            },
            dashboard_with(
                Vec::new(),
                vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
                Vec::new(),
                0,
            ),
            dashboard_with(
                vec![fixture::active("fix-empty-basket", 7, 7)],
                Vec::new(),
                vec!["openspec/changes: Permission denied (os error 13)".to_string()],
                0,
            ),
            Dashboard {
                repo: None,
                searched_from: std::path::PathBuf::from(
                    "/home/dev/workspaces/openspec-demos/a-rather-long-repository-name-here",
                ),
                changes: fixture::set(Vec::new(), Vec::new(), Vec::new()),
                route: Route::List,
                quit: false,
                selected: 0,
                filter: empty_filter(),
                detail: empty_detail(),
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
                // `list-sections`: compile-forced by `Dashboard`'s new field; group 5 owns
                // this file's real work.
                sections: crate::ui::app::Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
            },
        ]
    }

    #[test]
    fn active_row_grammar_at_38_and_58() {
        let d = three_active();
        for (width, expected) in [
            (
                38,
                [
                    "> add-token-refresh              [4/9]",
                    "  fix-empty-basket               [7/7]",
                    "  migrate-ai-sdk-v7                [-]",
                ],
            ),
            (
                58,
                [
                    "> add-token-refresh                                  [4/9]",
                    "  fix-empty-basket                                   [7/7]",
                    "  migrate-ai-sdk-v7                                    [-]",
                ],
            ),
        ] {
            let rows = rows(&d, width);
            // `list-sections`: rows[0] is now the active section header.
            assert_eq!(rows.len(), 4);
            for (row, expect) in rows[1..].iter().zip(expected.iter()) {
                assert_eq!(&row.text, expect, "width {width}");
            }
        }

        // `agent-attribution`: the identical dashboard, now carrying agents, still
        // renders every row byte-identically when none of them is in scope.
        let mut unattributed = three_active();
        unattributed.agents.agents = vec![agent_at("scratch-work", AgentStatus::Working)];
        for width in [38, 58] {
            let badgeless = rows(&d, width);
            let still_unbadged = rows(&unattributed, width);
            assert_eq!(
                badgeless, still_unbadged,
                "width {width}: an unmatched agent must badge nothing"
            );
        }
    }

    /// `agent-list` :: "An unreachable socket renders a list with no badge column at both
    /// widths" — whole-rows equality against the reachable-empty control, since
    /// `Dashboard::attribution` reads only `agents.agents`, never `agents.reachable`: an
    /// unreachable snapshot and a reachable-but-empty one must render identically. A
    /// reachable snapshot that DOES match an agent is the discriminating control.
    #[test]
    fn unreachable_socket_renders_no_badge_column() {
        let mut unreachable = three_active();
        unreachable.agents = crate::agents::AgentSnapshot {
            agents: Vec::new(),
            reachable: false,
            stalled: false,
            problem: None,
        };
        let mut reachable_empty = three_active();
        reachable_empty.agents = crate::agents::AgentSnapshot {
            agents: Vec::new(),
            reachable: true,
            stalled: false,
            problem: None,
        };
        let mut reachable_matched = three_active();
        reachable_matched.agents = crate::agents::AgentSnapshot {
            agents: vec![agent_at("add-token-refresh", AgentStatus::Working)],
            reachable: true,
            stalled: false,
            problem: None,
        };

        for width in [38, 58] {
            assert_eq!(
                rows(&unreachable, width),
                rows(&reachable_empty, width),
                "width {width}: reachable-but-empty must render identically to unreachable"
            );
            assert_ne!(
                rows(&unreachable, width),
                rows(&reachable_matched, width),
                "width {width}: a matched agent must actually add a badge, or the equality \
                 above proves nothing"
            );
        }
    }

    /// `degraded-coverage` :: "An out-of-scope agent and a worktree agent are both
    /// invisible" — rows 29/30. No cwd, a foreign cwd, and a linked-worktree-shaped cwd
    /// (`<repo-parent>/.worktrees/<repo>-<branch>`, per `SPEC.md` row 30) all fail the same
    /// `cwd.starts_with(root)` containment check row 29's own mechanism applies — there is
    /// no separate worktree-detection code. Whole-rows equality against a control carrying
    /// only the in-scope agent, so the three additions change nothing; the in-scope agent's
    /// own badge is the discriminating control.
    #[test]
    fn out_of_scope_and_worktree_agents_are_invisible() {
        fn agent_with_cwd(name: &str, cwd: Option<&str>) -> Agent {
            Agent {
                name: Some(name.to_string()),
                kind: None,
                status: AgentStatus::Working,
                cwd: cwd.map(std::path::PathBuf::from),
                pane_id: "p".to_string(),
                tab_id: "t".to_string(),
                workspace_id: "w".to_string(),
                terminal_title: None,
            }
        }

        let in_scope = agent_at("add-token-refresh", AgentStatus::Working);
        let no_cwd = agent_with_cwd("fix-empty-basket", None);
        let foreign_cwd = agent_with_cwd("fix-empty-basket", Some("/definitely/elsewhere"));
        let worktree_cwd = agent_with_cwd(
            "fix-empty-basket",
            Some("/tmp/demo-repo-worktrees/.worktrees/demo-repo-feature"),
        );

        let mut control = three_active();
        control.agents = crate::agents::AgentSnapshot {
            agents: vec![in_scope.clone()],
            reachable: true,
            stalled: false,
            problem: None,
        };
        let mut with_invisible_agents = three_active();
        with_invisible_agents.agents = crate::agents::AgentSnapshot {
            agents: vec![in_scope, no_cwd, foreign_cwd, worktree_cwd],
            reachable: true,
            stalled: false,
            problem: None,
        };

        for width in [38, 58] {
            assert_eq!(
                rows(&control, width),
                rows(&with_invisible_agents, width),
                "width {width}: the three out-of-scope agents must change nothing"
            );
            // Discriminating control: the in-scope agent's own badge must actually be
            // present, or the equality above is satisfied by two badgeless renders.
            assert!(
                rows(&control, width)
                    .iter()
                    .any(|r| r.text.contains(" w [")),
                "width {width}: the in-scope agent's badge must render: {:?}",
                rows(&control, width)
            );
        }
    }

    #[test]
    fn progress_cell_is_dash_when_total_is_zero() {
        let d = three_active();
        for width in [38, 58] {
            let rows = rows(&d, width);
            // `list-sections`: rows[0] is now the active section header; the three
            // change rows follow it at rows[1..4].
            assert!(rows[1].text.ends_with(']'));
            assert!(rows[3].text.ends_with(']'));
            // migrate-ai-sdk-v7 is the 0-of-0 fixture: its cell is the dash
            // form, not a numeric one — found in Change Review, whose test
            // only compared the two rows' last characters and would have
            // passed a `[0/0]` cell too.
            assert!(rows[3].text.contains("[-]"), "width {width}");
            assert_eq!(
                rows[1].text.chars().last(),
                rows[3].text.chars().last(),
                "width {width}"
            );
        }
    }

    #[test]
    fn every_row_is_exactly_the_requested_width() {
        for d in every_fixture() {
            for width in [38, 58] {
                for row in rows(&d, width) {
                    assert_eq!(columns(&row.text), width as usize);
                }
            }
        }

        // `agent-attribution`: a badged fixture is exactly the requested width too.
        let mut badged = three_active();
        badged.agents.agents = vec![agent_at("add-token-refresh", AgentStatus::Working)];
        for width in [38, 58] {
            for row in rows(&badged, width) {
                assert_eq!(columns(&row.text), width as usize, "badged row");
            }
        }
    }

    #[test]
    fn a_long_name_is_truncated_with_an_ellipsis() {
        let d = dashboard_with(
            vec![fixture::active(
                "a-very-long-change-name-that-will-not-fit-here",
                2,
                5,
            )],
            Vec::new(),
            Vec::new(),
            usize::MAX,
        );
        // `list-sections`: rows[0] is the active section header.
        let rows38 = rows(&d, 38);
        assert_eq!(rows38[1].text, "  a-very-long-change-name-that-… [2/5]");
        let rows58 = rows(&d, 58);
        assert_eq!(
            rows58[1].text,
            "  a-very-long-change-name-that-will-not-fit-here     [2/5]"
        );
        assert!(!rows58[1].text.contains('…'));

        // `agent-attribution`: the 46-character name is unreachable by Herdr's own
        // 32-character cap, so it is badged only through the mapping tier — and doing
        // so truncates the name two columns earlier to make room for the badge.
        let long_name = "a-very-long-change-name-that-will-not-fit-here";
        let derived = crate::state::agent_name(long_name);
        let mut badged = d;
        badged.agents.agents = vec![agent_at(&derived, AgentStatus::Working)];
        badged.agent_names.names =
            std::collections::BTreeMap::from([(derived, long_name.to_string())]);
        let rows38_badged = rows(&badged, 38);
        assert_eq!(
            rows38_badged[1].text,
            "  a-very-long-change-name-tha… w [2/5]"
        );
    }

    #[test]
    fn a_narrow_width_drops_the_progress_cell_whole() {
        // `list-sections`: `selected` **1** addresses `Target::Change(0)` — the active
        // section header is target 0 — so the change carries the cursor and the row
        // grammar's `>` marker below.
        let d = dashboard_with(
            vec![fixture::active("alpha", 4, 9)],
            Vec::new(),
            Vec::new(),
            1,
        );
        assert_eq!(rows(&d, 10)[1].text, "> a… [4/9]");
        assert_eq!(rows(&d, 9)[1].text, "> … [4/9]");
        assert_eq!(rows(&d, 8)[1].text, "> alpha ");
        assert_eq!(rows(&d, 3)[1].text, "> …");
        assert_eq!(rows(&d, 2)[1].text, "> ");
        assert_eq!(rows(&d, 1)[1].text, ">");
        assert_eq!(rows(&d, 0)[1].text, "");
        assert!(rows(&d, 38)[1].text.contains("[4/9]"));
        assert!(rows(&d, 58)[1].text.contains("[4/9]"));
    }

    #[test]
    fn archived_narrow_widths_drop_progress_then_date() {
        // `list-sections`: `targets()` here is `[Section(Active), Change(0), Section(Archived),
        // Change(1)]`, so `selected` **3** addresses the archived change — the same one this
        // test always meant, `add-auth` — and rows[3] is now its row (rows[0] and rows[2] are
        // the two section headers).
        let d = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
            Vec::new(),
            3,
        );
        let cases: [(u16, &str); 7] = [
            (20, "> 2026-08-14 … [7/7]"),
            (19, "> 2026-08-14 add-a…"),
            (14, "> 2026-08-14 …"),
            (13, "> add-auth   "),
            (3, "> …"),
            (1, ">"),
            (0, ""),
        ];
        for (width, expected) in cases {
            let row = &rows(&d, width)[3];
            assert_eq!(row.text, expected, "width {width}");
            assert_eq!(columns(&row.text), width as usize);
        }
        assert!(rows(&d, 38)[3].text.contains("2026-08-14"));
        assert!(rows(&d, 38)[3].text.contains("[7/7]"));
        assert!(rows(&d, 58)[3].text.contains("2026-08-14"));
        assert!(rows(&d, 58)[3].text.contains("[7/7]"));
    }

    /// `change-rows`: "A badged row carries its status between the name and the
    /// progress cell" — exact strings at 38, the badge's 0-based column index at 58.
    #[test]
    fn a_badged_active_row_at_both_widths() {
        let mut d = three_active();
        d.agents.agents = vec![
            agent_at("add-token-refresh", AgentStatus::Working),
            agent_at("fix-empty-basket", AgentStatus::Blocked),
            agent_at("migrate-ai-sdk-v7", AgentStatus::Unknown),
        ];

        // `list-sections`: rows[0] is the active section header.
        let rows38 = rows(&d, 38);
        assert_eq!(rows38[1].text, "> add-token-refresh            w [4/9]");
        assert_eq!(rows38[2].text, "  fix-empty-basket             b [7/7]");
        assert_eq!(rows38[3].text, "  migrate-ai-sdk-v7              ? [-]");

        let rows58 = rows(&d, 58);
        for row in &rows58 {
            assert_eq!(columns(&row.text), 58);
        }
        assert_eq!(rows58[1].text.chars().nth(51), Some('w'));
        assert_eq!(rows58[2].text.chars().nth(51), Some('b'));
        assert_eq!(rows58[3].text.chars().nth(53), Some('?'));
        for (row, idx) in [(&rows58[1], 51), (&rows58[2], 51), (&rows58[3], 53)] {
            assert_eq!(
                row.text.chars().nth(idx - 1),
                Some(' '),
                "column left of the badge"
            );
            assert_eq!(
                row.text.chars().nth(idx + 1),
                Some(' '),
                "column right of the badge"
            );
        }
        assert!(rows58[1].text.ends_with("[4/9]"));
        assert!(rows58[2].text.ends_with("[7/7]"));
        assert!(rows58[3].text.ends_with("[-]"));
    }

    /// `change-rows`: "An archived change carries a badge in the same column as an
    /// active one".
    #[test]
    fn a_badged_archived_row_at_both_widths() {
        // `list-sections`: `selected` **1** addresses `Target::Change(0)` —
        // `fix-empty-basket` — since the active section header is target 0.
        let unbadged = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            vec![
                fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                fixture::archived(None, "legacy-cleanup", 3, 3),
            ],
            Vec::new(),
            1,
        );
        let mut d = unbadged.clone();
        d.agents.agents = vec![agent_at("add-auth", AgentStatus::Blocked)];

        // rows[0] and rows[2] are the active and archived section headers.
        let rows38 = rows(&d, 38);
        assert_eq!(rows38[1].text, "> fix-empty-basket               [7/7]");
        assert_eq!(rows38[3].text, "  2026-08-14 add-auth          b [7/7]");
        assert_eq!(rows38[4].text, "             legacy-cleanup      [3/3]");

        let rows58 = rows(&d, 58);
        for row in &rows58 {
            assert_eq!(columns(&row.text), 58);
        }
        assert_eq!(rows58[3].text.chars().nth(51), Some('b'));
        assert_eq!(rows58[3].text.chars().nth(50), Some(' '));
        assert_eq!(rows58[3].text.chars().nth(52), Some(' '));
        assert_eq!(
            rows58[4].text,
            rows(&unbadged, 58)[4].text,
            "the archived header and every unbadged row must be byte-identical"
        );
    }

    /// `change-rows`: "An unattributed agent badges nothing" — the agentless
    /// rendering, byte-identical, then the discriminating half: the same agent
    /// renamed to a change does badge.
    #[test]
    fn an_unattributed_agent_badges_nothing() {
        let agentless = three_active();
        let mut with_unattributed = three_active();
        with_unattributed.agents.agents = vec![
            agent_at("scratch", AgentStatus::Working),
            Agent {
                name: None,
                kind: None,
                status: AgentStatus::Working,
                cwd: Some(std::path::PathBuf::from("/tmp/demo-repo")),
                pane_id: "p".to_string(),
                tab_id: "t".to_string(),
                workspace_id: "w".to_string(),
                terminal_title: None,
            },
        ];
        for width in [38, 58] {
            assert_eq!(
                rows(&agentless, width),
                rows(&with_unattributed, width),
                "width {width}: an unmatched or unnamed agent must badge nothing"
            );
        }

        // Discriminating half: the second agent renamed to a change does badge.
        let mut renamed = three_active();
        renamed.agents.agents = vec![
            agent_at("scratch", AgentStatus::Working),
            agent_at("migrate-ai-sdk-v7", AgentStatus::Working),
        ];
        for width in [38, 58] {
            assert_ne!(
                rows(&agentless, width),
                rows(&renamed, width),
                "width {width}: renaming the agent to a change must badge that row"
            );
            // `list-sections`: rows[0] is the active section header, so
            // migrate-ai-sdk-v7 (the third change) is rows[3].
            assert!(
                rows(&renamed, width)[3].text.contains(" w ["),
                "width {width}: migrate-ai-sdk-v7's row must carry the working badge: {:?}",
                rows(&renamed, width)[3].text
            );
        }
    }

    /// `change-rows`: "A field too narrow for both drops the progress cell whole" —
    /// the badged form.
    #[test]
    fn a_badged_row_drops_the_badge_first() {
        // `list-sections`: `selected` **1** addresses `Target::Change(0)` — `alpha` —
        // since the active section header is target 0.
        let mut d = dashboard_with(
            vec![fixture::active("alpha", 4, 9)],
            Vec::new(),
            Vec::new(),
            1,
        );
        d.agents.agents = vec![agent_at("alpha", AgentStatus::Working)];

        let cases: [(u16, &str); 5] = [
            (12, "> a… w [4/9]"),
            (11, "> … w [4/9]"),
            (10, "> a… [4/9]"),
            (9, "> … [4/9]"),
            (8, "> alpha "),
        ];
        for (width, expected) in cases {
            assert_eq!(rows(&d, width)[1].text, expected, "width {width}");
        }
        assert_eq!(rows(&d, 1)[1].text, ">");
        assert_eq!(rows(&d, 0)[1].text, "");
        assert!(rows(&d, 38)[1].text.contains(" w ["));
        assert!(rows(&d, 58)[1].text.contains(" w ["));
    }

    /// `change-rows`: "An archived row drops the progress cell, then the date, as
    /// the width falls" — the badged form.
    #[test]
    fn a_badged_archived_row_drops_the_badge_first() {
        // `list-sections`: `targets()` here is `[Section(Active), Change(0),
        // Section(Archived), Change(1)]`, so `selected` **3** addresses `add-auth`
        // and rows[3] is its row.
        let mut d = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
            Vec::new(),
            3,
        );
        d.agents.agents = vec![agent_at("add-auth", AgentStatus::Blocked)];

        let cases: [(u16, &str); 9] = [
            (22, "> 2026-08-14 … b [7/7]"),
            (21, "> 2026-08-14 a… [7/7]"),
            (20, "> 2026-08-14 … [7/7]"),
            (19, "> 2026-08-14 add-a…"),
            (14, "> 2026-08-14 …"),
            (13, "> add-auth   "),
            (3, "> …"),
            (1, ">"),
            (0, ""),
        ];
        for (width, expected) in cases {
            assert_eq!(rows(&d, width)[3].text, expected, "width {width}");
        }
        assert!(rows(&d, 38)[3].text.contains("2026-08-14"));
        assert!(rows(&d, 38)[3].text.contains(" b ["));
        assert!(rows(&d, 58)[3].text.contains("2026-08-14"));
        assert!(rows(&d, 58)[3].text.contains(" b ["));
    }

    /// `change-rows`: "A badged active row reports the column its badge occupies, at
    /// both mandated widths". The row's own `text` is asserted against the same
    /// literals `a_badged_active_row_at_both_widths` above already pins, unedited by
    /// this change — which is what makes "adding the field moves no cell" falsifiable
    /// rather than merely stated.
    #[test]
    fn a_badged_active_row_reports_the_column_its_badge_occupies() {
        // `list-sections`: `selected` **1** addresses `Target::Change(0)` —
        // `add-token-refresh` — since the active section header is target 0.
        let mut d = dashboard_with(
            vec![
                fixture::active("add-token-refresh", 4, 9),
                fixture::active("fix-empty-basket", 7, 7),
            ],
            Vec::new(),
            Vec::new(),
            1,
        );
        d.agents.agents = vec![agent_at("add-token-refresh", AgentStatus::Working)];

        // rows[0] is the active section header.
        let rows38 = rows(&d, 38);
        assert_eq!(rows38[1].text, "> add-token-refresh            w [4/9]");
        assert_eq!(rows38[2].text, "  fix-empty-basket               [7/7]");

        let rows58 = rows(&d, 58);
        assert_eq!(columns(&rows58[1].text), 58);
        assert!(rows58[1].text.starts_with("> add-token-refresh"));
        assert!(rows58[1].text.ends_with(" w [4/9]"));

        // "[4/9]" is five columns, so the name field is `width - 10` and the badge
        // sits three columns past its start: marker, its space, the name field, and
        // the badge's own separating space.
        for (width, badged, unbadged) in [
            (38u16, &rows38[1], &rows38[2]),
            (58, &rows58[1], &rows58[2]),
        ] {
            let badge = badged.badge.expect("the badged row must report its cell");
            assert_eq!(badge.status, AgentStatus::Working, "width {width}");
            assert_eq!(badge.x, (width - 10) + 3, "width {width}");
            assert_eq!(
                badged.text.chars().nth(badge.x as usize),
                Some('w'),
                "width {width}: text[x] is not the badge character"
            );
            assert_eq!(unbadged.badge, None, "width {width}");
        }
    }

    /// `change-rows`: "A badged archived row reports the column its badge occupies" —
    /// with the equality against an active row of the same width as the
    /// discriminating half, since a date field forgotten on one side moves one of
    /// the two.
    #[test]
    fn a_badged_archived_row_reports_the_column_its_badge_occupies() {
        let mut archived = dashboard_with(
            Vec::new(),
            vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
            Vec::new(),
            0,
        );
        archived.agents.agents = vec![agent_at("add-auth", AgentStatus::Blocked)];
        let mut active = dashboard_with(
            vec![fixture::active("add-auth", 7, 7)],
            Vec::new(),
            Vec::new(),
            0,
        );
        active.agents.agents = vec![agent_at("add-auth", AgentStatus::Blocked)];

        let widths: [u16; 2] = [38, 58];
        for width in widths {
            // The archived block: a `No active changes` message, the separator, then
            // the change itself.
            let row = &rows(&archived, width)[2];
            let badge = row.badge.expect("the badged row must report its cell");
            assert_eq!(badge.status, AgentStatus::Blocked, "width {width}");
            assert_eq!(
                row.text.chars().nth(badge.x as usize),
                Some('b'),
                "width {width}: text[x] is not the badge character"
            );
            // marker, space, date field (10), space, name field, space.
            let progress = 5u16;
            let name_field_width = width - 14 - 1 - 1 - progress;
            assert_eq!(badge.x, name_field_width + 14, "width {width}");

            // `list-sections`: rows[0] is the active section header.
            let active_badge = rows(&active, width)[1]
                .badge
                .expect("the active row must report its cell too");
            assert_eq!(
                badge.x, active_badge.x,
                "width {width}: both grammars put the badge two columns left of the \
                 progress cell"
            );
        }
    }

    /// `change-rows`: "A dropped badge cell reports no badge" — the field never
    /// points at a column the row does not have.
    #[test]
    fn a_dropped_badge_cell_reports_no_badge() {
        // `list-sections`: `selected` **1** addresses `Target::Change(0)` — `demo` —
        // since the active section header is target 0; rows[0] is that header.
        let mut d = dashboard_with(
            vec![fixture::active("demo", 4, 9)],
            Vec::new(),
            Vec::new(),
            1,
        );
        d.agents.agents = vec![agent_at("demo", AgentStatus::Working)];

        // 38 and 58 are the two mandated interiors; 12 and 11 are the narrowest at
        // which the badge cell still fits.
        let fits: [u16; 4] = [38, 58, 12, 11];
        for width in fits {
            let row = &rows(&d, width)[1];
            let badge = row.badge.expect("the badge cell still fits");
            assert_eq!(badge.status, AgentStatus::Working, "width {width}");
            assert_eq!(
                row.text.chars().nth(badge.x as usize),
                Some('w'),
                "width {width}: text[x] is not the badge character"
            );
        }
        let dropped: [u16; 4] = [10, 9, 1, 0];
        for width in dropped {
            assert_eq!(
                rows(&d, width)[1].badge,
                None,
                "width {width}: a dropped badge cell must report no badge"
            );
        }
    }

    /// `change-rows`: "No non-change row carries a badge" — every `Problem`,
    /// `Separator`, and `Message` row, at both mandated widths, whatever
    /// `attribution().badges` holds.
    #[test]
    fn no_non_change_row_carries_a_badge() {
        let mut problems = dashboard_with(
            vec![fixture::active("alpha", 4, 9)],
            vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
            vec!["openspec/changes: unreadable".to_string()],
            0,
        );
        problems.launch.problems = vec!["herdr agent start: refused".to_string()];
        problems.refresh.problems = vec!["watch: could not start".to_string()];
        problems.agents.agents = vec![
            agent_at("alpha", AgentStatus::Working),
            agent_at("add-auth", AgentStatus::Blocked),
        ];
        let mut filtered = problems.clone();
        filtered.filter.query = "zzz".to_string();
        let mut no_repo = problems.clone();
        no_repo.repo = None;

        let widths: [u16; 2] = [38, 58];
        for width in widths {
            for (label, d) in [
                ("problems", &problems),
                ("filtered", &filtered),
                ("no repository", &no_repo),
            ] {
                for row in rows(d, width) {
                    if matches!(row.kind, RowKind::Item { .. }) {
                        continue;
                    }
                    assert_eq!(
                        row.badge, None,
                        "width {width}: {label}'s {:?} row carries a badge: {:?}",
                        row.kind, row.text
                    );
                }
            }
            // Discriminating control: the change rows the same dashboard produces
            // *are* badged, so the sweep above is not satisfied by a badgeless render.
            let badged = rows(&problems, width)
                .iter()
                .filter(|r| r.badge.is_some())
                .count();
            assert_eq!(
                badged, 2,
                "width {width}: both change rows must carry a badge"
            );
        }
    }

    #[test]
    fn archived_rows_carry_a_ten_column_date_field() {
        let d = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
            Vec::new(),
            usize::MAX,
        );
        // `list-sections`: rows[0] and rows[2] are the active and archived section
        // headers; the archived change is rows[3].
        assert_eq!(
            rows(&d, 38)[3].text,
            "  2026-08-14 add-auth            [7/7]"
        );
        assert_eq!(
            rows(&d, 58)[3].text,
            "  2026-08-14 add-auth                                [7/7]"
        );
    }

    #[test]
    fn an_undated_archived_row_uses_ten_spaces() {
        let d = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            vec![fixture::archived(None, "legacy-cleanup", 3, 3)],
            Vec::new(),
            usize::MAX,
        );
        assert_eq!(
            rows(&d, 38)[3].text,
            "             legacy-cleanup      [3/3]"
        );
        assert_eq!(
            rows(&d, 58)[3].text,
            "             legacy-cleanup                          [3/3]"
        );

        let dated = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            vec![fixture::archived(
                Some("2026-08-14"),
                "legacy-cleanup",
                3,
                3,
            )],
            Vec::new(),
            usize::MAX,
        );
        for width in [38, 58] {
            let undated_start = rows(&d, width)[3].text.find('l');
            let dated_start = rows(&dated, width)[3].text.find('l');
            assert_eq!(undated_start, dated_start, "width {width}");
        }
    }

    /// `change-rows` -> "An archived row drops the progress cell, then the date, as
    /// the width falls": one archived change, no active changes, the archived
    /// section open, `selected` 1 so the cursor is on the change. The scenario now
    /// names the change row `rows()[2]`, with `No active changes` at `rows()[0]` and
    /// the archived header at `rows()[1]` — an empty active section emits that
    /// message row whenever the archived section's count is not zero, exactly the
    /// shape here (`planning-review.md` I2 repaired the scenario, which had said
    /// `rows()[1]`). This test asserts that order at every width and then reads
    /// `rows()[2]`, so it is bound to the scenario's own index rather than to
    /// whichever row happens to be the first `RowKind::Item`.
    #[test]
    fn an_archived_row_drops_the_progress_cell_then_the_date_as_the_width_falls() {
        let mut d = dashboard_with(
            Vec::new(),
            vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
            Vec::new(),
            1,
        );
        d.agents.agents = vec![agent_at("add-auth", AgentStatus::Blocked)];
        let change_row = |width: u16| -> String {
            let rows = rows(&d, width);
            // The scenario's own index, and its own row order: an active section whose
            // count is zero puts `No active changes` at 0 and the archived header at 1.
            assert!(
                matches!(rows[2].kind, RowKind::Item { .. }),
                "width {width}: the change row is rows()[2]: {:?}",
                rows.iter().map(|r| &r.kind).collect::<Vec<_>>()
            );
            rows[2].text.clone()
        };
        let expected = [
            (22, "> 2026-08-14 … b [7/7]"),
            (21, "> 2026-08-14 a… [7/7]"),
            (20, "> 2026-08-14 … [7/7]"),
            (19, "> 2026-08-14 add-a…"),
            (14, "> 2026-08-14 …"),
            (13, "> add-auth   "),
            (3, "> …"),
            (1, ">"),
            (0, ""),
        ];
        for (width, text) in expected {
            assert_eq!(change_row(width), text, "width {width}");
        }
        // The contrasting controls at the mandated interiors: the date, the badge,
        // and the progress cell all survive.
        for width in [38, 58] {
            let text = change_row(width);
            assert!(text.contains("2026-08-14"), "width {width}");
            assert!(text.contains(" b ["), "width {width}");
            assert!(text.ends_with("[7/7]"), "width {width}");
        }
    }

    /// `change-rows` -> "A section header degrades by truncation at every width":
    /// the archived section collapsed over an unresolved archive of twenty-two
    /// changes.
    #[test]
    fn a_section_header_degrades_by_truncation_at_every_width() {
        // `selected` is out of range so nothing carries the cursor — this test is
        // about the header's own truncation, not its selection marker.
        let mut d = dashboard_with(Vec::new(), Vec::new(), Vec::new(), usize::MAX);
        d.changes.archived_total = 22;
        d.sections.collapsed.insert(SectionKey::Archived);
        let archived_header = |width: u16| -> String {
            rows(&d, width)
                .into_iter()
                .find(|r| {
                    matches!(
                        r.kind,
                        RowKind::Section {
                            key: SectionKey::Archived,
                            ..
                        }
                    )
                })
                .expect("an archived header row")
                .text
        };
        let expected = [
            (17, "  > archived (22)"),
            (16, "  > archived (2…"),
            (5, "  > …"),
            (1, " "),
            (0, ""),
        ];
        for (width, text) in expected {
            let header = archived_header(width);
            assert_eq!(header, text, "width {width}");
            assert_eq!(columns(&header), width as usize, "width {width}");
        }
        for width in [38, 58] {
            let header = archived_header(width);
            assert_eq!(
                header,
                format!("{:<w$}", "  > archived (22)", w = width as usize),
                "width {width}"
            );
        }
    }

    /// `list-sections`' rewrite of the landed `the_separator_is_emitted_only_when_archived_rows_follow`:
    /// the archived section header replaces the separator, and it is emitted on
    /// exactly the same condition — at least one archived change exists.
    #[test]
    fn the_archived_header_is_emitted_only_when_archived_changes_exist() {
        let no_archived = three_active();
        for width in [38, 58] {
            assert!(!rows(&no_archived, width).iter().any(|r| matches!(
                r.kind,
                RowKind::Section {
                    key: SectionKey::Archived,
                    ..
                }
            )));
        }
        let with_archived = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
            Vec::new(),
            0,
        );
        for width in [38, 58] {
            let count = rows(&with_archived, width)
                .iter()
                .filter(|r| {
                    matches!(
                        r.kind,
                        RowKind::Section {
                            key: SectionKey::Archived,
                            ..
                        }
                    )
                })
                .count();
            assert_eq!(count, 1);
        }
    }

    /// `list-sections`' rewrite of the landed
    /// `row_order_is_problems_then_active_then_separator_then_archived`: the separator
    /// becomes two section headers, and the active/archived `Item` indices stay
    /// exactly what they were — `RowKind::Item { index }` is unchanged by this
    /// change, only the header rows around it are new.
    #[test]
    fn row_order_is_problems_then_active_header_then_active_then_archived_header_then_archived() {
        let mut d = dashboard_with(
            vec![
                fixture::active("add-token-refresh", 4, 9),
                fixture::active("fix-empty-basket", 7, 7),
            ],
            vec![
                fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                fixture::archived(None, "legacy-cleanup", 3, 3),
            ],
            vec!["openspec/changes: broken".to_string()],
            0,
        );
        // `agent-launch`: a launch problem leads even the refresh and change-set problems.
        d.launch.problems = vec!["launch failed".to_string()];
        for width in [38, 58] {
            let kinds: Vec<RowKind> = rows(&d, width).into_iter().map(|r| r.kind).collect();
            assert_eq!(
                kinds,
                vec![
                    RowKind::Problem,
                    RowKind::Problem,
                    RowKind::Section {
                        key: SectionKey::Active,
                        depth: 0,
                        collapsed: false,
                    },
                    RowKind::Item { index: 0 },
                    RowKind::Item { index: 1 },
                    RowKind::Section {
                        key: SectionKey::Archived,
                        depth: 0,
                        collapsed: false,
                    },
                    RowKind::Item { index: 2 },
                    RowKind::Item { index: 3 },
                ],
                "width {width}"
            );
        }
    }

    #[test]
    fn rows_preserve_the_change_set_order() {
        let d = dashboard_with(
            vec![
                fixture::active("zeta", 1, 2),
                fixture::active("alpha", 1, 2),
                fixture::active("mid", 1, 2),
            ],
            Vec::new(),
            Vec::new(),
            usize::MAX,
        );
        for width in [38, 58] {
            let rows = rows(&d, width);
            // `list-sections`: rows[0] is the active section header.
            assert!(rows[1].text.contains("zeta"));
            assert!(rows[2].text.contains("alpha"));
            assert!(rows[3].text.contains("mid"));
        }
    }

    #[test]
    fn the_selected_flag_marks_exactly_the_selected_change() {
        // `list-sections`: `selected` now indexes `targets()`, not `visible()` or
        // `Item { index }` directly. `targets()` here is `[Section(Active),
        // Change(0), Change(1), Change(2), Section(Archived), Change(3),
        // Change(4)]`, so `Target::Change(3)` — the same change this test always
        // meant, `add-auth`, the first archived entry — sits at position 5.
        let d = dashboard_with(
            vec![
                fixture::active("add-token-refresh", 4, 9),
                fixture::active("fix-empty-basket", 7, 7),
                fixture::active("migrate-ai-sdk-v7", 0, 0),
            ],
            vec![
                fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                fixture::archived(None, "legacy-cleanup", 3, 3),
            ],
            Vec::new(),
            5,
        );
        for width in [38, 58] {
            let rows = rows(&d, width);
            let selected: Vec<&Row> = rows.iter().filter(|r| r.selected).collect();
            assert_eq!(selected.len(), 1, "width {width}");
            assert_eq!(selected[0].kind, RowKind::Item { index: 3 });
            for r in &rows {
                if matches!(r.kind, RowKind::Section { .. }) {
                    assert!(!r.selected);
                }
            }
        }
    }

    #[test]
    fn the_no_repository_block_is_three_rows() {
        let d = Dashboard {
            repo: None,
            searched_from: std::path::PathBuf::from(
                "/home/dev/workspaces/openspec-demos/a-rather-long-repository-name-here",
            ),
            changes: fixture::set(Vec::new(), Vec::new(), Vec::new()),
            route: Route::List,
            quit: false,
            selected: 0,
            filter: empty_filter(),
            detail: empty_detail(),
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
            // `list-sections`: compile-forced by `Dashboard`'s new field; group 5 owns
            // this file's real work.
            sections: crate::ui::app::Sections {
                collapsed: std::collections::BTreeSet::new(),
            },
            file_mode: false,
        };
        let rows38 = rows(&d, 38);
        assert_eq!(rows38.len(), 3);
        assert!(rows38[0].text.starts_with("No OpenSpec repository found"));
        assert!(rows38[1].text.starts_with("searched from:"));
        assert!(
            rows38[2]
                .text
                .starts_with("…os/a-rather-long-repository-name-here")
        );
        assert!(
            !rows38
                .iter()
                .any(|r| matches!(r.kind, RowKind::Item { .. }))
        );

        let rows58 = rows(&d, 58);
        assert!(
            rows58[2]
                .text
                .starts_with("…kspaces/openspec-demos/a-rather-long-repository-name-here")
        );
        assert!(
            !rows58
                .iter()
                .any(|r| matches!(r.kind, RowKind::Item { .. }))
        );

        // The no-repository block replaces EVERY other row, problem rows
        // included — `ui::load` cannot build this value (it only ever
        // constructs `changes::empty_set()` alongside `repo: None`), but
        // `rows` is a pure total function over every `Dashboard` a test can
        // construct, and this precedence is stated for exactly that case.
        // Found in Change Review: no scenario exercised it, so a guard
        // narrowed to `repo.is_none() && problems.is_empty()` would have
        // passed the whole suite.
        let with_problems = Dashboard {
            repo: None,
            searched_from: d.searched_from.clone(),
            changes: fixture::set(Vec::new(), Vec::new(), vec!["broken".to_string()]),
            route: Route::List,
            quit: false,
            selected: 0,
            filter: empty_filter(),
            detail: empty_detail(),
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
            // `list-sections`: compile-forced by `Dashboard`'s new field; group 5 owns
            // this file's real work.
            sections: crate::ui::app::Sections {
                collapsed: std::collections::BTreeSet::new(),
            },
            file_mode: false,
        };
        let rows_with_problems = rows(&with_problems, 38);
        assert_eq!(rows_with_problems.len(), 3);
        assert!(
            !rows_with_problems
                .iter()
                .any(|r| r.kind == RowKind::Problem)
        );
    }

    #[test]
    fn the_four_message_states_are_distinct() {
        for width in [38, 58] {
            let empty = dashboard_with(Vec::new(), Vec::new(), Vec::new(), 0);
            let rows_empty = rows(&empty, width);
            assert!(rows_empty[0].text.starts_with("No changes yet"));

            // `agent-attribution`: two in-scope, unattributable agents change no pixel
            // of the message row — a `Message` row is never badged.
            let mut empty_with_agents = dashboard_with(Vec::new(), Vec::new(), Vec::new(), 0);
            empty_with_agents.agents.agents = vec![
                agent_at("nothing-like-a-change", AgentStatus::Working),
                agent_at("also-nothing", AgentStatus::Idle),
            ];
            assert_eq!(
                rows(&empty_with_agents, width),
                rows_empty,
                "width {width}: an unattributable agent must badge no message row"
            );

            let archived_only = dashboard_with(
                Vec::new(),
                vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
                Vec::new(),
                0,
            );
            let rows_archived_only = rows(&archived_only, width);
            assert!(rows_archived_only[0].text.starts_with("No active changes"));
            assert_eq!(
                rows_archived_only[1].kind,
                RowKind::Section {
                    key: SectionKey::Archived,
                    depth: 0,
                    collapsed: false,
                }
            );
            assert_eq!(rows_archived_only[2].kind, RowKind::Item { index: 0 });

            let mut no_match = dashboard_with(
                vec![fixture::active("fix-empty-basket", 7, 7)],
                Vec::new(),
                Vec::new(),
                0,
            );
            no_match.filter.query = "zzz".to_string();
            let rows_no_match = rows(&no_match, width);
            assert!(rows_no_match[0].text.starts_with("No changes match"));
            assert!(rows_no_match[1].text.starts_with("/zzz"));

            let mut matched = dashboard_with(
                vec![fixture::active("fix-empty-basket", 7, 7)],
                Vec::new(),
                Vec::new(),
                0,
            );
            matched.filter.query = "fix".to_string();
            let rows_matched = rows(&matched, width);
            assert!(!rows_matched.iter().any(|r| r.kind == RowKind::Message));
        }
    }

    #[test]
    fn problem_rows_are_truncated_to_the_width() {
        let d = dashboard_with(
            Vec::new(),
            Vec::new(),
            vec!["openspec/changes: Permission denied (os error 13)".to_string()],
            0,
        );
        let row38 = &rows(&d, 38)[0];
        assert_eq!(row38.text, "! openspec/changes: Permission denied…");
        let row58 = &rows(&d, 58)[0];
        assert_eq!(
            row58.text,
            "! openspec/changes: Permission denied (os error 13)       "
        );
    }

    #[test]
    fn rows_never_panic_at_any_width() {
        for d in every_fixture() {
            for width in [0u16, 1, 2, 8, 9, 10, 13, 14, 38, 58, 120, u16::MAX] {
                for row in rows(&d, width) {
                    assert_eq!(columns(&row.text), width as usize, "width {width}");
                }
            }
        }
    }

    // `live-refresh` -> "The list region's leading rows name refresh
    // problems first". See `specs/live-updates/spec.md`.

    #[test]
    fn refresh_problems_lead_the_rows() {
        let mut d = dashboard_with(
            vec![fixture::active("add-token-refresh", 4, 9)],
            Vec::new(),
            Vec::new(),
            0,
        );
        d.refresh.problems = vec!["filesystem watch unavailable for /r".to_string()];
        for width in [38, 58] {
            let all = rows(&d, width);
            assert_eq!(all[0].kind, RowKind::Problem);
            assert_eq!(
                all[0].text,
                problem_row_text("filesystem watch unavailable for /r", width),
                "width {width}"
            );
            // `list-sections`: all[1] is the active section header; the change row
            // follows it at all[2].
            assert_eq!(
                all[1].kind,
                RowKind::Section {
                    key: SectionKey::Active,
                    depth: 0,
                    collapsed: false
                },
                "width {width}"
            );
            assert_eq!(all[2].kind, RowKind::Item { index: 0 }, "width {width}");
        }

        // The no-problem control: with `refresh.problems` empty, the same
        // dashboard's rows are unchanged.
        let mut without = d.clone();
        without.refresh.problems = Vec::new();
        for width in [38, 58] {
            assert_eq!(rows(&without, width)[1].kind, RowKind::Item { index: 0 });
        }

        // `agent-launch`: the same holds for `launch.problems` — a pane that has launched
        // nothing renders byte-identically to one with no launch tier at all.
        assert!(without.launch.problems.is_empty());
        let mut with_launch_failure = without.clone();
        with_launch_failure.launch.problems = vec!["launch failed".to_string()];
        for width in [38, 58] {
            assert_eq!(
                rows(&with_launch_failure, width)[0].kind,
                RowKind::Problem,
                "width {width}"
            );
            assert_eq!(rows(&without, width)[1].kind, RowKind::Item { index: 0 });
        }
    }

    #[test]
    fn refresh_and_change_problems_in_order() {
        let mut d = dashboard_with(
            vec![fixture::active("add-token-refresh", 4, 9)],
            Vec::new(),
            vec!["openspec/changes unreadable".to_string()],
            0,
        );
        d.refresh.problems = vec!["watch failed".to_string()];
        // `agent-launch`: a launch problem leads even the refresh problem.
        d.launch.problems = vec!["launch failed".to_string()];
        for width in [38, 58] {
            let kinds: Vec<RowKind> = rows(&d, width).into_iter().map(|r| r.kind).collect();
            assert_eq!(
                kinds,
                vec![
                    RowKind::Problem,
                    RowKind::Problem,
                    RowKind::Problem,
                    RowKind::Section {
                        key: SectionKey::Active,
                        depth: 0,
                        collapsed: false,
                    },
                    RowKind::Item { index: 0 },
                ],
                "width {width}"
            );
            let texts: Vec<String> = rows(&d, width).into_iter().map(|r| r.text).collect();
            assert_eq!(
                texts[0],
                problem_row_text("launch failed", width),
                "the launch problem must come first, width {width}"
            );
            assert_eq!(
                texts[1],
                problem_row_text("watch failed", width),
                "the refresh problem must come second, width {width}"
            );
            assert_eq!(
                texts[2],
                problem_row_text("openspec/changes unreadable", width),
                "width {width}"
            );
        }

        // The empty control: with `launch.problems` emptied, the rows are byte-identical to
        // the two-problem list this scenario specified before `agent-launch` existed.
        let mut without_launch = d.clone();
        without_launch.launch.problems = Vec::new();
        for width in [38, 58] {
            let kinds: Vec<RowKind> = rows(&without_launch, width)
                .into_iter()
                .map(|r| r.kind)
                .collect();
            assert_eq!(
                kinds,
                vec![
                    RowKind::Problem,
                    RowKind::Problem,
                    RowKind::Section {
                        key: SectionKey::Active,
                        depth: 0,
                        collapsed: false,
                    },
                    RowKind::Item { index: 0 },
                ],
                "width {width}"
            );
        }

        // `agent-attribution`: a badged change below the three problem rows and the active
        // section header carries its badge, and none of the rows above it ever carries one —
        // `list-sections` adds the header row as a fourth non-badged row.
        d.agents.agents = vec![agent_at("add-token-refresh", AgentStatus::Blocked)];
        for width in [38, 58] {
            let rows = rows(&d, width);
            assert!(
                !rows[0].text.contains(" b ["),
                "width {width}: a problem row is never badged"
            );
            assert!(
                !rows[1].text.contains(" b ["),
                "width {width}: a problem row is never badged"
            );
            assert!(
                !rows[2].text.contains(" b ["),
                "width {width}: a problem row is never badged"
            );
            assert!(
                !rows[3].text.contains(" b ["),
                "width {width}: a section header is never badged"
            );
            assert!(
                rows[4].text.contains(" b ["),
                "width {width}: the change row below must carry its badge"
            );
        }
    }

    /// `agent-launch`: "A refused launch renders one row at both widths."
    #[test]
    fn a_launch_problem_is_the_lists_first_row() {
        let mut d = dashboard_with(
            vec![fixture::active("2fa-support", 1, 2)],
            Vec::new(),
            Vec::new(),
            0,
        );
        d.launch.problems = vec![
            "c-2fa-support is already running for this change - press g to focus it".to_string(),
        ];
        for width in [38, 58] {
            let all = rows(&d, width);
            assert_eq!(all[0].kind, RowKind::Problem, "width {width}");
            assert!(
                all[0].text.starts_with("! "),
                "width {width}: {}",
                all[0].text
            );
            assert!(all[0].text.contains("c-2fa-support"), "width {width}");
            assert!(all[0].text.contains('g'), "width {width}");
            assert_eq!(
                columns(&all[0].text),
                width as usize,
                "width {width}: padded or truncated to the interior width exactly"
            );
            assert_eq!(
                all[1].kind,
                RowKind::Section {
                    key: SectionKey::Active,
                    depth: 0,
                    collapsed: false,
                },
                "width {width}"
            );
            assert_eq!(all[2].kind, RowKind::Item { index: 0 }, "width {width}");

            // Pressing `a` twice more (replacing the entry wholesale, never growing it)
            // leaves exactly one such row.
            let count = rows(&d, width)
                .into_iter()
                .filter(|r| r.kind == RowKind::Problem)
                .count();
            assert_eq!(count, 1, "width {width}");
        }
    }

    /// `degraded-states`' repair of row 23 (task 6.2): the two reasons — a recording failure
    /// and a prompt failure — render as the list's first two `!`-marked interior rows, in
    /// occurrence order, at both widths. RED at `main`: `launch::Outcome::problem` could not
    /// hold both, so `Dashboard::launch.problems` never carried more than one entry.
    #[test]
    fn a_failed_record_and_a_failed_prompt_are_both_reported() {
        let mut d = dashboard_with(
            vec![fixture::active("2fa-support", 1, 2)],
            Vec::new(),
            Vec::new(),
            0,
        );
        d.launch.problems = vec![
            "/state/dir: not a directory".to_string(),
            "agent_blocked: agent is blocked".to_string(),
        ];
        for width in [38, 58] {
            let all = rows(&d, width);
            assert_eq!(all[0].kind, RowKind::Problem, "width {width}");
            assert_eq!(all[1].kind, RowKind::Problem, "width {width}");
            assert!(all[0].text.starts_with("! "), "width {width}");
            assert!(all[1].text.starts_with("! "), "width {width}");
            assert!(all[0].text.contains("/state/dir"), "width {width}");
            assert!(all[1].text.contains("agent_blocked"), "width {width}");
            assert_eq!(
                all[2].kind,
                RowKind::Section {
                    key: SectionKey::Active,
                    depth: 0,
                    collapsed: false,
                },
                "width {width}"
            );
            assert_eq!(all[3].kind, RowKind::Item { index: 0 }, "width {width}");
        }
    }

    /// `agent-launch`: "A launch problem leads the refresh and change-set problems."
    #[test]
    fn launch_refresh_and_change_problems_in_order() {
        let mut d = dashboard_with(
            vec![fixture::active("alpha", 1, 2)],
            Vec::new(),
            vec!["openspec/changes unreadable".to_string()],
            0,
        );
        d.refresh.problems = vec!["watch failed".to_string()];
        d.launch.problems = vec!["launch failed".to_string()];
        for width in [38, 58] {
            let all = rows(&d, width);
            assert_eq!(all[0].kind, RowKind::Problem, "width {width}");
            assert_eq!(all[1].kind, RowKind::Problem, "width {width}");
            assert_eq!(all[2].kind, RowKind::Problem, "width {width}");
            assert_eq!(all[0].text, problem_row_text("launch failed", width));
            assert_eq!(all[1].text, problem_row_text("watch failed", width));
            assert_eq!(
                all[2].text,
                problem_row_text("openspec/changes unreadable", width)
            );
            assert!(!all[0].selected && !all[1].selected && !all[2].selected);
        }
    }

    /// `agent-launch`: "No refresh problem draws no extra row" restated for
    /// `launch.problems` — a pane that has launched nothing renders exactly as before.
    #[test]
    fn no_launch_problem_draws_no_extra_row() {
        let d = dashboard_with(
            vec![fixture::active("alpha", 1, 2)],
            Vec::new(),
            Vec::new(),
            0,
        );
        assert!(d.launch.problems.is_empty());
        for width in [38, 58] {
            // `list-sections`: rows[0] is the active section header.
            assert_eq!(
                rows(&d, width)[1].kind,
                RowKind::Item { index: 0 },
                "width {width}"
            );
        }
    }

    /// `agent-launch`: a launch problem row carries no badge, whatever `attribution().badges`
    /// holds — only a change row is badged.
    #[test]
    fn a_launch_problem_row_carries_no_badge() {
        let mut d = dashboard_with(
            vec![fixture::active("alpha", 1, 2)],
            Vec::new(),
            Vec::new(),
            0,
        );
        d.launch.problems = vec!["launch failed".to_string()];
        d.agents.agents = vec![agent_at("alpha", AgentStatus::Working)];
        for width in [38, 58] {
            let all = rows(&d, width);
            assert_eq!(all[0].kind, RowKind::Problem, "width {width}");
            assert!(
                !all[0].text.contains(" w ["),
                "width {width}: a launch problem row must carry no badge"
            );
            assert!(
                !all[1].text.contains(" w ["),
                "width {width}: a section header carries no badge"
            );
            assert!(
                all[2].text.contains(" w ["),
                "width {width}: the change row below must still carry its badge"
            );
        }
    }

    /// `seam-resilience`: `live-updates` -> "All five problem sources render in their
    /// specified order" — launch, stall, startup, live, change-set.
    #[test]
    fn all_five_problem_sources_render_in_their_specified_order() {
        let mut d = dashboard_with(
            vec![fixture::active("alpha", 1, 2)],
            Vec::new(),
            vec!["openspec/changes unreadable".to_string()],
            0,
        );
        d.launch.problems = vec!["launch failed".to_string()];
        d.agents = crate::agents::AgentSnapshot {
            agents: Vec::new(),
            reachable: false,
            stalled: true,
            problem: Some("herdr agent list has not answered in 5s".to_string()),
        };
        d.refresh.startup = vec![
            "openspec binary not found".to_string(),
            "config.toml: archived_count not set".to_string(),
        ];
        d.refresh.problems = vec!["watch failed".to_string()];
        for width in [38, 58] {
            let all = rows(&d, width);
            let kinds: Vec<RowKind> = all.iter().map(|r| r.kind).collect();
            assert_eq!(
                kinds[0..6],
                [
                    RowKind::Problem,
                    RowKind::Problem,
                    RowKind::Problem,
                    RowKind::Problem,
                    RowKind::Problem,
                    RowKind::Problem,
                ],
                "width {width}"
            );
            // `list-sections`: all[6] is the active section header; the change row
            // follows it at all[7].
            assert_eq!(
                all[6].kind,
                RowKind::Section {
                    key: SectionKey::Active,
                    depth: 0,
                    collapsed: false,
                },
                "width {width}"
            );
            assert_eq!(all[7].kind, RowKind::Item { index: 0 }, "width {width}");
            assert!(all[0].text.contains("launch failed"), "width {width}");
            assert!(all[1].text.contains("has not answered"), "width {width}");
            assert!(
                all[2].text.contains("openspec binary not found"),
                "width {width}"
            );
            assert!(all[3].text.contains("archived_count"), "width {width}");
            assert!(all[4].text.contains("watch failed"), "width {width}");
            assert!(
                all[5].text.contains("openspec/changes unreadable"),
                "width {width}"
            );
            for row in &all[0..6] {
                assert_eq!(columns(&row.text), width as usize, "width {width}");
                assert!(!row.selected, "width {width}");
            }
        }
    }

    /// `seam-resilience`: "A non-stalled agent problem draws no row" — the documented
    /// silent standalone-TUI state, unchanged by this addition.
    #[test]
    fn a_non_stalled_agent_problem_draws_no_row() {
        let mut with_problem = three_active();
        with_problem.agents = crate::agents::AgentSnapshot {
            agents: Vec::new(),
            reachable: false,
            stalled: false,
            problem: Some("herdr agent list exited 1: server_not_running".to_string()),
        };
        let mut without = three_active();
        without.agents = crate::agents::AgentSnapshot {
            agents: Vec::new(),
            reachable: false,
            stalled: false,
            problem: None,
        };
        for width in [38, 58] {
            assert_eq!(
                rows(&with_problem, width),
                rows(&without, width),
                "width {width}"
            );
        }
    }

    #[test]
    fn a_refresh_problem_row_degrades_at_narrow_widths() {
        let mut d = dashboard_with(Vec::new(), Vec::new(), Vec::new(), 0);
        d.refresh.problems = vec!["watch failed entirely".to_string()];
        for width in [38, 58] {
            assert_eq!(rows(&d, width)[0].kind, RowKind::Problem, "width {width}");
            assert_eq!(
                columns(&rows(&d, width)[0].text),
                width as usize,
                "width {width}"
            );
        }
        // The degenerate widths this scenario is named for: the row falls
        // back to the first `width` characters of `"! "` itself.
        assert_eq!(rows(&d, 1)[0].text, "!");
        assert_eq!(rows(&d, 0)[0].text, "");
    }

    // `view-fidelity` -> `change-rows`: "Every cell of the row grammar is measured in
    // display columns". See `openspec/changes/view-fidelity/specs/change-rows/spec.md`.

    /// `change-rows`: "A CJK change name stays inside the list region at both mandated
    /// widths" — the row's own `columns()` is exact, and rendered into a real buffer the
    /// region's own right gutter is unmoved from an all-ASCII control render, which is the
    /// overwrite the audit measured.
    #[test]
    fn a_cjk_change_name_stays_inside_the_list_region_at_both_mandated_widths() {
        let name = "日本語の変更名前です";
        assert_eq!(name.chars().count(), 10);
        assert_eq!(columns(name), 20);

        // `list-sections`: `selected` **1** addresses `Target::Change(0)` since the
        // active section header is target 0.
        let cjk = dashboard_with(vec![fixture::active(name, 4, 9)], Vec::new(), Vec::new(), 1);
        let ascii = dashboard_with(
            vec![fixture::active("add-token-refresh", 4, 9)],
            Vec::new(),
            Vec::new(),
            1,
        );

        for width in [38, 58] {
            let text = rows(&cjk, width)[1].text.clone();
            assert_eq!(columns(&text), width as usize, "width {width}: {text:?}");
            assert!(text.ends_with("[4/9]"), "width {width}: {text:?}");
            assert!(text.contains(name), "width {width}: {text:?}");
        }

        // `list-sections`: buffer row 2 is now the active section header; the change
        // row is buffer row 3.
        for (width, border_x) in [(120u16, 39u16), (60u16, 59u16)] {
            let buf = crate::testutil::render_at(width, 20, &cjk);
            let control = crate::testutil::render_at(width, 20, &ascii);
            assert_eq!(
                crate::testutil::cell(&buf, border_x, 3).symbol(),
                " ",
                "width {width}: the region's right gutter must stay a blank space"
            );
            assert_eq!(
                crate::testutil::cell(&buf, border_x, 3).symbol(),
                crate::testutil::cell(&control, border_x, 3).symbol(),
                "width {width}: the gutter must be unmoved from the ASCII-named control"
            );
            assert_eq!(
                crate::testutil::cell(&buf, border_x - 1, 3).symbol(),
                "]",
                "width {width}: the progress cell ends in the interior's last column"
            );
        }
    }

    /// `change-rows`: "An emoji change name at 58 columns does not overwrite the border" —
    /// checked badged and not, at both mandated widths.
    #[test]
    fn an_emoji_change_name_at_58_columns_does_not_overwrite_the_border() {
        let name = "emoji-🎉-change";
        // `list-sections`: `selected` **1** addresses `Target::Change(0)` since the
        // active section header is target 0.
        let plain = dashboard_with(vec![fixture::active(name, 4, 9)], Vec::new(), Vec::new(), 1);
        let mut badged = plain.clone();
        badged.agents.agents = vec![agent_at(name, AgentStatus::Working)];

        let ascii_name = "emoji-plain-change";
        let ascii_plain = dashboard_with(
            vec![fixture::active(ascii_name, 4, 9)],
            Vec::new(),
            Vec::new(),
            1,
        );
        let mut ascii_badged = ascii_plain.clone();
        ascii_badged.agents.agents = vec![agent_at(ascii_name, AgentStatus::Working)];

        for width in [38, 58] {
            for (d, expect_badge) in [(&plain, false), (&badged, true)] {
                let text = rows(d, width)[1].text.clone();
                assert_eq!(
                    columns(&text),
                    width as usize,
                    "width {width} badge {expect_badge}: {text:?}"
                );
                assert!(
                    text.ends_with(']'),
                    "width {width} badge {expect_badge}: {text:?}"
                );
                if expect_badge {
                    assert!(text.contains(" w ["), "width {width}: {text:?}");
                }
            }
        }

        // `list-sections`: buffer row 3 is the change row; row 2 is now the active
        // section header.
        for (width, border_x) in [(120u16, 39u16), (60u16, 59u16)] {
            for (subject, control) in [(&plain, &ascii_plain), (&badged, &ascii_badged)] {
                let buf = crate::testutil::render_at(width, 20, subject);
                let ctl = crate::testutil::render_at(width, 20, control);
                assert_eq!(
                    crate::testutil::cell(&buf, border_x, 3).symbol(),
                    " ",
                    "width {width}"
                );
                assert_eq!(
                    crate::testutil::cell(&buf, border_x, 3).symbol(),
                    crate::testutil::cell(&ctl, border_x, 3).symbol(),
                    "width {width}"
                );
            }
        }
    }

    /// `change-rows`: "A wide name is truncated whole and padded back to the full width" —
    /// the ellipsis lands on a whole CJK character and the shortfall is a trailing space
    /// rather than a missing cell.
    #[test]
    fn a_wide_name_is_truncated_whole_and_padded_back_to_the_full_width() {
        let name = "日本語の変更名前です日本語の変更名前です日本語の変更名前です";
        assert_eq!(name.chars().count(), 30);
        assert_eq!(columns(name), 60);
        // `list-sections`: `selected` **1** addresses `Target::Change(0)` since the
        // active section header is target 0.
        let d = dashboard_with(vec![fixture::active(name, 4, 9)], Vec::new(), Vec::new(), 1);

        for width in [38, 58] {
            let text = rows(&d, width)[1].text.clone();
            assert_eq!(columns(&text), width as usize, "width {width}: {text:?}");
            assert!(text.ends_with("…  [4/9]"), "width {width}: {text:?}");

            let start = "> ".len();
            let ellipsis_idx = text.find('…').expect("an ellipsis");
            let prefix = &text[start..ellipsis_idx];
            assert!(
                name.starts_with(prefix),
                "width {width}: {prefix:?} is not a prefix of the change's own name"
            );
            let before = text[..ellipsis_idx]
                .chars()
                .last()
                .expect("a character before the ellipsis");
            assert!(
                name.contains(before),
                "width {width}: the character before the ellipsis, {before:?}, must be a whole \
                 character, not a cut one"
            );
        }
    }

    /// `change-rows`: "Rows are total over adversarial names at every width" — sweeps
    /// `0..=130`, the mandated pair named explicitly among the swept values per
    /// `view-fidelity`'s design.md -> Decision 4.
    #[test]
    fn rows_are_total_over_adversarial_names_at_every_width() {
        let wide_cjk = "日".repeat(100);
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}".to_string();
        let combining = format!("e{}", "\u{0301}".repeat(5));
        let katakana = "\u{FF9E}".to_string();
        let with_nul = format!("a{}b", '\u{0}');
        let empty = String::new();

        let names = [
            wide_cjk.as_str(),
            family.as_str(),
            combining.as_str(),
            katakana.as_str(),
            with_nul.as_str(),
            empty.as_str(),
        ];
        let active: Vec<crate::changes::Change> =
            names.iter().map(|n| fixture::active(n, 4, 9)).collect();
        let archived = vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)];
        let mut d = dashboard_with(active, archived, Vec::new(), 0);
        d.agents.agents = vec![agent_at(names[0], AgentStatus::Working)];

        for width in 0u16..=130 {
            let all = rows(&d, width);
            for row in &all {
                assert!(
                    columns(&row.text) <= width as usize,
                    "width {width}: {:?} measures more than {width}",
                    row.text
                );
                assert_eq!(
                    columns(&row.text),
                    width as usize,
                    "width {width}: {:?}",
                    row.text
                );
            }
            if width <= 2 {
                // At widths `0`, `1`, and `2` every row is empty or a bare marker — already
                // proven exactly by the columns equality above, restated here as the
                // scenario's own named claim.
                for row in &all {
                    assert!(columns(&row.text) <= 2, "width {width}: {:?}", row.text);
                }
            }
        }

        for row in rows(&d, 38) {
            assert!(columns(&row.text) <= 38);
        }
        for row in rows(&d, 58) {
            assert!(columns(&row.text) <= 58);
        }
    }

    /// `change-rows`: "The no-repository block shortens its search path by columns" —
    /// distinguished from a char-counted shortening, which for this path would measure
    /// more columns than either interior.
    #[test]
    fn the_no_repository_block_shortens_its_search_path_by_columns() {
        let path = "/home/dev/workspaces/日本語のディレクトリ名前がとても長い場合の例";
        assert_eq!(path.chars().count(), 43);
        assert_eq!(columns(path), 65);

        let d = Dashboard {
            repo: None,
            searched_from: std::path::PathBuf::from(path),
            changes: fixture::set(Vec::new(), Vec::new(), Vec::new()),
            route: Route::List,
            quit: false,
            selected: 0,
            filter: empty_filter(),
            detail: empty_detail(),
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
            // `list-sections`: compile-forced by `Dashboard`'s new field; group 5 owns
            // this file's real work.
            sections: crate::ui::app::Sections {
                collapsed: std::collections::BTreeSet::new(),
            },
            file_mode: false,
        };

        for width in [38, 58] {
            let all = rows(&d, width);
            assert_eq!(all.len(), 3, "width {width}");
            assert!(
                all.iter().all(|r| r.kind == RowKind::Message),
                "width {width}"
            );
            let text = all[2].text.clone();
            assert_eq!(columns(&text), width as usize, "width {width}: {text:?}");
            assert!(text.starts_with('…'), "width {width}: {text:?}");

            let suffix = &text['…'.len_utf8()..];
            let budget = width as usize - 1;
            assert!(columns(suffix) <= budget, "width {width}");
            assert!(
                path.ends_with(suffix.trim_end()),
                "width {width}: {suffix:?} does not slice back out of the searched path"
            );

            // A `char`-counted shortening would have kept the last `width - 1`
            // characters instead of columns — for this path that measures more
            // columns than the budget it was meant to fit, so the two measures
            // are distinguishable.
            let old_kept: String = path
                .chars()
                .rev()
                .take(budget)
                .collect::<Vec<char>>()
                .into_iter()
                .rev()
                .collect();
            assert!(
                columns(&old_kept) > budget,
                "width {width}: the char-counted suffix must measure more than the columns \
                 budget, or the scenario does not discriminate"
            );
        }

        for (width, border_x) in [(120u16, 39u16), (60u16, 59u16)] {
            let buf = crate::testutil::render_at(width, 20, &d);
            for y in [2u16, 3, 4] {
                assert_eq!(
                    crate::testutil::cell(&buf, border_x, y).symbol(),
                    " ",
                    "width {width} row {y}"
                );
            }
        }
    }
}
