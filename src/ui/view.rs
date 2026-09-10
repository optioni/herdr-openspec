//! The render seam's pure side: `render(frame, &Dashboard)`. Performs no
//! filesystem, process, environment, network, or terminal I/O, reads no
//! clock and no global state. See
//! `openspec/changes/tui-shell/specs/responsive-layout/spec.md`.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::ui::app::{Dashboard, Route};
use crate::ui::detail;
use crate::ui::layout::{
    Gutters, columns, interior, scroll_offset, split_body, split_detail, split_frame,
    truncate_columns,
};
use crate::ui::list;
use crate::ui::markdown::Face;
use crate::ui::palette::{self, Role};

/// The footer's key hints, in the order they are drawn and dropped from.
const FOOTER_HINTS: [&str; 3] = ["q quit", "Enter detail", "Esc back"];

/// Draw `dashboard` into `frame`. A pure function of its two arguments.
///
/// `pane-chrome` removes the frame's header row entirely: `layout::split_frame`
/// now returns only `(body, footer)`, and the repository's identity moves into
/// the list region's own heading row (`render_region`), drawn as part of the
/// body rather than above it.
pub fn render(frame: &mut Frame, dashboard: &Dashboard) {
    let (body, footer) = split_frame(frame.area());
    render_footer(frame, footer, dashboard);
    render_body(frame, body, dashboard);
}

/// The body: one or two borderless regions, per `layout::split_body`, with a
/// one-column divider between them at `LayoutMode::Wide`. The list region's
/// interior is filled by `render_list`; the detail region's by
/// `render_detail`.
///
/// `render_region` draws only the **list** region's own heading — the
/// repository directory name and the `file mode` badge. The detail region's
/// heading is the selected change's own header, which `detail-header` draws
/// directly into its area (group 4); until then this call site draws no
/// heading for it at all, which is correct rather than a placeholder, since
/// nothing has claimed that row yet.
///
/// The one-column divider between the two wide-layout regions is drawn here
/// — `render_body` is the one place that knows both rectangles — as
/// `Role::RegionRule`, down every row of the body, belonging to neither
/// region. Costing five chrome columns for a divider with a blank column on
/// both sides leaves only four at a 120-column frame (D5), so the detail
/// region gives up its trailing gutter: `Gutters::LeftOnly`, its interior
/// running to the frame's own last column.
fn render_body(frame: &mut Frame, body: Rect, dashboard: &Dashboard) {
    let (list_area, divider, detail) = split_body(body, dashboard.route);
    if let Some(area) = list_area {
        render_region(frame, area, dashboard, dashboard.route == Route::List);
        render_list(frame, interior(area, Gutters::Both), dashboard);
    }
    if let Some(area) = detail {
        // `layout::detail_gutters`: `Gutters::LeftOnly` only at the wide
        // layout, where the divider (D5) spends the detail region's
        // trailing gutter; below the breakpoint there is no divider to
        // make room for, so the detail region keeps both gutters exactly
        // as the narrow list region does
        // (`the_detail_interior_is_78_columns_at_120_and_58_at_60`).
        render_detail(
            frame,
            area,
            crate::ui::layout::detail_gutters(divider),
            dashboard,
        );
    }
    if let Some(col) = divider {
        let style = palette::style(Role::RegionRule);
        let buf = frame.buffer_mut();
        for y in body.y..body.y.saturating_add(body.height) {
            buf.set_string(col, y, "│", style);
        }
    }
}

/// Draw the change header into the detail region's own heading row, then
/// the tab bar, the rule, and the content area into its interior — nothing
/// at all when `Dashboard::visible()` is empty (`selected_change()` is
/// `None`), which is what leaves the heading row and every interior cell
/// blank in that state. When a change **is** selected, all four rows are
/// always drawn: the header, the tab bar, and the rule unconditionally, and
/// the content area holds at least one line because
/// `ui::detail::content_lines` returns `No content yet` rather than
/// nothing.
///
/// `area` is the whole detail **region**, not its interior — mirroring
/// `render_region`/`render_list`'s own split for the list region. The
/// heading row sits at `area`'s own first row, sharing the interior's `x`
/// and `width` (`design.md` -> D2), with the region's blank padding row
/// between it and the interior `layout::interior` returns
/// (`detail-header` -> "The header row is drawn into the detail region's
/// first interior row").
fn render_detail(frame: &mut Frame, area: Rect, gutters: Gutters, dashboard: &Dashboard) {
    let Some(change) = dashboard.selected_change() else {
        return;
    };
    let region_interior = interior(area, gutters);
    let header = Rect {
        x: region_interior.x,
        y: area.y,
        width: region_interior.width,
        height: area.height.min(1),
    };
    render_detail_header(frame, header, change, dashboard.route == Route::Detail);
    let (tabs, rule, content) = split_detail(region_interior);
    render_detail_tabs(frame, tabs, change, dashboard.detail.tab);
    render_detail_rule(frame, rule);
    render_detail_content(frame, content, dashboard);
}

/// The change header: `ui::detail::header_row` under `Role::RegionHeadingFocused`
/// when the detail region is routed, else `Role::RegionHeading` — the same
/// region-heading pair every other region's heading takes (`view-palette` ->
/// draw-span mapping). Draws nothing at zero width or zero height.
fn render_detail_header(
    frame: &mut Frame,
    header: Rect,
    change: &crate::changes::Change,
    focused: bool,
) {
    if header.width == 0 || header.height == 0 {
        return;
    }
    let role = if focused {
        Role::RegionHeadingFocused
    } else {
        Role::RegionHeading
    };
    let text = detail::header_row(&change.name, &change.schema, &change.progress, header.width);
    frame
        .buffer_mut()
        .set_string(header.x, header.y, &text, palette::style(role));
}

/// The tab bar: every `ui::detail::Tab` at `tabs.x + tab.x`, under
/// `Role::TabActive` for the selected cell and `Role::TabInactive` for every
/// other. The chip's two padding columns are part of `Tab::text`
/// (`color-palette` -> design.md -> Decision 5), so one `set_string` paints
/// exactly the chip's own span and leaves the single separating column
/// between two chips untouched. Draws nothing at zero width or zero height.
fn render_detail_tabs(frame: &mut Frame, tabs: Rect, change: &crate::changes::Change, tab: usize) {
    if tabs.width == 0 || tabs.height == 0 {
        return;
    }
    let buf = frame.buffer_mut();
    let last_col = tabs.x + tabs.width;
    for cell in detail::tab_bar(&change.artifacts, tab, tabs.width) {
        let x = tabs.x + cell.x;
        if x >= last_col {
            continue;
        }
        let role = if cell.selected {
            Role::TabActive
        } else {
            Role::TabInactive
        };
        buf.set_string(x, tabs.y, &cell.text, palette::style(role));
    }
}

/// The horizontal rule beneath the tab bar: `─` repeated across the row's
/// full width, under `Role::RegionRule` — the same role the wide layout's
/// column divider carries, since both are chrome that separates content
/// rather than content itself (`artifact-tabs` -> "The tab bar is the
/// detail interior's first row, above a rule and a padding row"). Draws
/// nothing at zero width or zero height.
fn render_detail_rule(frame: &mut Frame, rule: Rect) {
    if rule.width == 0 || rule.height == 0 {
        return;
    }
    let text = "─".repeat(rule.width as usize);
    frame
        .buffer_mut()
        .set_string(rule.x, rule.y, &text, palette::style(Role::RegionRule));
}

/// The content area: the slice of `ui::detail::content_lines`
/// `layout::scroll_offset` selects, one rendered line per terminal row
/// starting at the content area's first row and column, each segment drawn
/// left to right with `style_for(&segment.face)` and stopping at the
/// interior's last column. Draws nothing when the content area has zero
/// width or zero height — `content_lines` always returns at least one
/// line, but there may be no row to draw it into.
fn render_detail_content(frame: &mut Frame, content: Rect, dashboard: &Dashboard) {
    if content.width == 0 || content.height == 0 {
        return;
    }
    let lines = detail::content_lines(
        &dashboard.detail,
        dashboard.selected_change(),
        content.width,
    );
    let offset = scroll_offset(lines.len(), dashboard.detail.scroll, content.height);
    let buf = frame.buffer_mut();
    for (i, line) in lines
        .iter()
        .skip(offset)
        .take(content.height as usize)
        .enumerate()
    {
        let y = content.y + i as u16;
        let mut x = content.x;
        let last_col = content.x + content.width;
        for segment in &line.segments {
            if x >= last_col {
                break;
            }
            // `view-fidelity` -> Decision 8 (corrected): the guard above is now correct,
            // since `x` advances by consumed columns rather than characters, and
            // `content_lines` never hands this loop a line whose `columns` exceeds
            // `content.width` — so `truncate_columns(&segment.text, last_col - x)` below
            // can never actually truncate anything; it is provably unreachable under that
            // contract, not independently dodgeable (an earlier draft of this comment
            // claimed the latter, which the crate's own tests disprove: removing this
            // clamp leaves every test green). It stays because the invariant it would
            // enforce lives in a *different* module — `ui::markdown`/`ui::detail` — and a
            // later change there could break it without ever touching this loop.
            let remaining = (last_col - x) as usize;
            let text = truncate_columns(&segment.text, remaining);
            let style = style_for(&segment.face);
            buf.set_string(x, y, text, style);
            x += columns(text) as u16;
        }
    }
}

/// The crate's only `Face`-to-`Style` mapping, and it constructs no style of
/// its own: the palette's face roles are folded onto `Style::default()` with
/// `Style::patch` in the fixed order `Quoted`, `Link`, `Code`, `Emphasis`,
/// `Strong`, `Heading` (`specs/view-palette/spec.md`).
///
/// Because `patch` lets the later value win, modifiers accumulate — a bold
/// link's cells carry `BOLD` and `UNDERLINED` together, exactly as before —
/// while the **foreground** of a span carrying several coloured faces is
/// decided by the last one in that order: heading over code over link, so a
/// heading line reads as one colour even where it contains a code span or a
/// link (design.md -> Decision 8). Total: no `Face` panics, and
/// `Face::plain()` maps to `Style::default()`.
fn style_for(face: &Face) -> Style {
    let mut style = Style::default();
    if face.quoted {
        style = style.patch(palette::style(Role::Quoted));
    }
    // Second, immediately after `Quoted`: carrying no foreground, a struck face
    // cannot displace a coloured role's colour wherever it sits, so it is placed
    // beside the other uncoloured always-composing face rather than inserted
    // into the coloured precedence chain, which stays heading over code over
    // link (design.md -> Decision 9).
    if face.strikethrough {
        style = style.patch(palette::style(Role::Strikethrough));
    }
    if face.link {
        style = style.patch(palette::style(Role::Link));
    }
    if face.code {
        style = style.patch(palette::style(Role::Code));
    }
    if face.emphasis {
        style = style.patch(palette::style(Role::Emphasis));
    }
    if face.strong {
        style = style.patch(palette::style(Role::Strong));
    }
    if let Some(level) = face.heading {
        style = style.patch(palette::style(Role::Heading(level)));
    }
    style
}

/// Draw `list::rows(dashboard, interior.width)` into `interior`: the slice
/// `layout::viewport` selects, one row per terminal row starting at the
/// interior's first row and column, each under the palette role [`row_role`]
/// gives it. Draws nothing when the interior has zero width or zero height —
/// there is nothing to index into.
/// The palette role a drawn list row carries: a `Section` row keeps
/// `Role::ListSeparator` whether or not it holds the cursor — `render_list`
/// is what patches `Modifier::BOLD` onto a selected one, since `list-sections`
/// adds no new role for it (design.md -> Decision 3) — `ListRowSelected` for
/// any other row holding the selection — which is why a badge on it stays
/// bold, the badge role carrying no modifier of its own — and otherwise the
/// role its `RowKind` names. Split out of [`render_list`] so the mapping
/// reads as one table rather than as a branch inside a drawing loop.
fn row_role(row: &list::Row) -> Role {
    match row.kind {
        list::RowKind::Section { .. } => Role::ListSeparator,
        _ if row.selected => Role::ListRowSelected,
        list::RowKind::Problem => Role::ListProblem,
        list::RowKind::Message => Role::ListMessage,
        list::RowKind::Item { .. } => Role::ListRow,
    }
}

fn render_list(frame: &mut Frame, interior: Rect, dashboard: &Dashboard) {
    if interior.width == 0 || interior.height == 0 {
        return;
    }
    // `mouse-input`: the three lines this loop used to derive inline now live in
    // `list::drawn_rows`, so `list::row_at` resolves a click against the very
    // slice drawn here rather than against a second derivation of it.
    let (rows, offset) = list::drawn_rows(dashboard, interior);
    let buf = frame.buffer_mut();
    for (i, row) in rows
        .iter()
        .skip(offset)
        .take(interior.height as usize)
        .enumerate()
    {
        let y = interior.y + i as u16;
        let mut style = palette::style(row_role(row));
        // `list-sections` design.md -> Decision 3: a `Section` row keeps
        // `Role::ListSeparator`'s colour whether or not it carries the cursor —
        // `row_role` never returns `ListRowSelected` for one — so the cursor is shown
        // by patching `BOLD` on here instead, the one place `Role::ListRowSelected`'s
        // own `BOLD` is added for every other row kind.
        if row.selected && matches!(row.kind, list::RowKind::Section { .. }) {
            style = style.add_modifier(Modifier::BOLD);
        }
        buf.set_string(interior.x, y, &row.text, style);
        // The badge is one column inside a row already drawn: re-write that single
        // cell with the row's own style **patched** by the badge role, so it keeps
        // every modifier the row carries — a badge on the selected row is bold and
        // coloured — and gains only the status colour. `badge.x` at or past the
        // interior's width is skipped rather than clamped, so no badge is ever drawn
        // over a border.
        if let Some(badge) = row.badge
            && badge.x < interior.width
            && let Some(cell) = buf.cell_mut((interior.x + badge.x, y))
        {
            cell.set_style(style.patch(palette::style(Role::AgentBadge(badge.status))));
        }
    }
}

/// The `file mode` badge's own text — nine columns, drawn dim and yellow,
/// right-aligned against the heading row's last column. See
/// `specs/responsive-layout/spec.md`.
const FILE_MODE_BADGE: &str = "file mode";

/// The list region's own heading row: the repository directory name (or
/// `no repository`), shortened from the left when it does not fit —
/// `ui::list::shorten_left` — and, when `dashboard.file_mode` is set, the
/// `file mode` badge right-aligned against the heading row's last column.
/// Draws no `Block`, no border, and nothing outside the heading row itself;
/// the blank padding row below it and the interior are the caller's job.
/// `emphasised` picks `Role::RegionHeadingFocused` over `Role::RegionHeading`,
/// the same pair every region's heading takes.
///
/// The badge is dropped **whole**, before the name is shortened, whenever the
/// heading row cannot hold the name, a separating blank, and the badge's nine
/// columns together — `change-rows`' own drop-whole rule. `a` is therefore the
/// full heading width when the badge does not fit alongside the name, and the
/// heading width less ten (nine badge columns plus one separating blank)
/// exactly when it does; the two together make the shortening branch
/// reachable only when the badge was dropped, per
/// `specs/responsive-layout/spec.md` -> "The list region's heading names the
/// repository directory".
fn render_region(frame: &mut Frame, area: Rect, dashboard: &Dashboard, emphasised: bool) {
    if area.height == 0 {
        return;
    }
    // The heading row's own span — `interior(area, Gutters::Both)`'s `x` and
    // `width`, not its `y` or `height`: this draws into the region's own
    // heading row at `area.y` itself, not into the interior two rows below
    // it. Reusing `interior` here rather than a hand-written
    // `area.width.saturating_sub(2)`/`area.x + 1` is what keeps the gutter
    // arithmetic written in the one place that owns it (Change Review of
    // `pane-chrome`'s `detail_gutters` fix).
    let inner = interior(area, Gutters::Both);
    let iw = inner.width;
    if iw == 0 {
        return;
    }
    let role = if emphasised {
        Role::RegionHeadingFocused
    } else {
        Role::RegionHeading
    };
    let name = repo_heading_name(dashboard);
    let full_with_badge = iw.saturating_sub(10);
    let show_badge = dashboard.file_mode && columns(&name) <= full_with_badge as usize;
    let a = if show_badge { full_with_badge } else { iw };
    let buf = frame.buffer_mut();
    let x0 = inner.x;
    if a > 0 {
        let shown = if columns(&name) <= a as usize {
            name
        } else {
            list::shorten_left(&name, a as usize)
        };
        buf.set_string(x0, area.y, &shown, palette::style(role));
    }
    if show_badge {
        let badge_x = x0 + iw - 9;
        buf.set_string(
            badge_x,
            area.y,
            FILE_MODE_BADGE,
            palette::style(Role::FileMode),
        );
    }
}

/// The list region's heading text: the repository root's final path
/// component, the whole display path when the root has none (the filesystem
/// root `/`), or the literal `no repository` when there is no root at all.
/// See `specs/responsive-layout/spec.md` -> "The list region's heading names
/// the repository directory".
fn repo_heading_name(dashboard: &Dashboard) -> String {
    match &dashboard.repo {
        Some(path) => match path.file_name() {
            Some(name) => name.to_string_lossy().into_owned(),
            None => path.display().to_string(),
        },
        None => "no repository".to_string(),
    }
}

/// The footer row, in one of three forms — `list-filtering` -> "The footer
/// shows the filter prompt while filtering and the query after":
/// - filtering: the prompt `/` + query + `_`, replacing the hints (and the
///   unattributed count) entirely, keeping its **tail** when it overflows
///   the footer;
/// - not filtering, a non-empty query: `/` + query leads the hint list,
///   dropped last rather than first, with the count still last;
/// - otherwise: `q quit`, `Enter detail`, and `Esc back`, then —
///   `agent-launch`'s addition — `a/c/s launch` and `g focus`, each its own
///   hint, when `Dashboard::agents.reachable`, then — `agent-attribution`'s
///   addition — `<n> unattributed` when `Dashboard::attribution().unattributed`
///   is greater than zero. Hints are dropped whole, one at a time, from the
///   **end** of this list as the width falls: `<n> unattributed` first, then
///   `g focus`, then `a/c/s launch`, then the landed hints from `Esc back`
///   backward — never a partial hint.
fn render_footer(frame: &mut Frame, footer: Rect, dashboard: &Dashboard) {
    if footer.height == 0 {
        return;
    }
    let filter = &dashboard.filter;
    let text = if filter.active {
        footer_prompt(&filter.query, footer.width)
    } else {
        let mut hints = Vec::new();
        if !filter.query.is_empty() {
            hints.push(format!("/{}", filter.query));
        }
        hints.extend(FOOTER_HINTS.iter().map(|s| (*s).to_string()));
        // `agent-launch`: the action hints, offered only when the socket is reachable — read
        // from `Dashboard::agents` and nowhere else, never from `ChangeSet::problems`, which
        // `adopt` replaces wholesale on every refresh.
        if dashboard.agents.reachable {
            hints.push("a/c/s launch".to_string());
            hints.push("g focus".to_string());
        }
        let unattributed = dashboard.attribution().unattributed;
        if unattributed > 0 {
            hints.push(format!("{unattributed} unattributed"));
        }
        fit_hints(&hints, footer.width)
    };
    if !text.is_empty() {
        frame
            .buffer_mut()
            .set_string(footer.x, footer.y, &text, palette::style(Role::Footer));
    }
}

/// The filter prompt: `/` + `query` + `_`, whole when it fits `width`,
/// otherwise its **tail** — its last `width` **columns** — so the cursor
/// (the trailing `_`) and the characters just typed stay visible. The
/// query is the reader's own typed text and is not ASCII-bound, so the tail
/// is found the same way `ui::list::shorten_left` finds its own kept
/// suffix: growing the dropped-prefix budget until enough columns are
/// actually gone, since a boundary-respecting drop MAY fall short of an
/// exact column target.
fn footer_prompt(query: &str, width: u16) -> String {
    let text = format!("/{query}_");
    let w = width as usize;
    let total = columns(&text);
    if total <= w {
        return text;
    }
    let must_drop = total - w;
    let mut probe = must_drop;
    loop {
        let prefix = truncate_columns(&text, probe);
        if columns(prefix) >= must_drop || prefix.len() == text.len() {
            return text[prefix.len()..].to_string();
        }
        probe += 1;
    }
}

/// `hints`, joined by two spaces, dropping whole hints from the **end**
/// when the remaining width cannot hold the next one whole — the rule
/// `render_footer`'s three-hint form already used, generalised to any hint
/// list so the accepted-query form can lead with a fourth hint.
fn fit_hints(hints: &[String], width: u16) -> String {
    let mut shown: Vec<&str> = Vec::new();
    let mut used = 0u16;
    for (i, hint) in hints.iter().enumerate() {
        let separator = if i == 0 { 0 } else { 2 };
        let needed = columns(hint) as u16 + separator;
        let Some(next_used) = used.checked_add(needed) else {
            break;
        };
        if next_used > width {
            break;
        }
        used = next_used;
        shown.push(hint.as_str());
    }
    shown.join("  ")
}

#[cfg(test)]
mod tests {
    use ratatui::backend::TestBackend;
    use ratatui::buffer::{Buffer, Cell};
    use ratatui::style::{Modifier, Style};

    use super::style_for;
    use crate::changes::{Change, empty_set, fixture};
    use crate::testutil::{cell, render_at, row_text};
    use crate::ui::app::{Action, Dashboard, Detail, Filter, Route, SectionKey};
    use crate::ui::layout::columns;
    use crate::ui::markdown::Face;
    use crate::ui::palette::{self, Role};

    fn empty_filter() -> Filter {
        Filter {
            query: String::new(),
            active: false,
        }
    }

    /// `mouse-input`: the two view scenarios that bind the hit test to the
    /// frame the reader is actually looking at — `change-rows`' "Every drawn row
    /// is reported by the row it occupies" and `responsive-layout`'s "The hit
    /// test agrees with what was drawn".
    mod hit_test {
        use ratatui::layout::Rect;

        use crate::changes::fixture;
        use crate::testutil::render_at;
        use crate::ui::app::{Dashboard, Route};
        use crate::ui::layout::{
            Gutters, Zone, detail_gutters, interior, split_body, split_frame, viewport, zone,
        };
        use crate::ui::list::{row_at, rows};

        /// One repository-level problem, three active changes, three archived
        /// ones — every row kind the grammar emits except `Message`.
        fn populated(route: Route) -> Dashboard {
            let active: Vec<_> = ["alpha", "beta", "gamma"]
                .iter()
                .map(|n| fixture::active(n, 1, 3))
                .collect();
            let archived: Vec<_> = ["delta", "epsilon", "zeta"]
                .iter()
                .map(|n| fixture::archived(Some("2026-01-01"), n, 4, 4))
                .collect();
            let mut dashboard = super::dashboard_with(active, archived, 1, route);
            dashboard.changes.problems = vec!["a repository problem".to_string()];
            dashboard
        }

        #[test]
        fn every_drawn_row_is_reported_by_row_at() {
            // Both mandated frames, written unsuffixed: `WIDTHS`' number scan is
            // `\b(\d+)\b` and does not see `120u16`.
            let frames: [(u16, u16); 2] = [(120, 40), (60, 20)];
            for (width, height) in frames {
                let dashboard = populated(Route::List);
                let buffer = render_at(width, height, &dashboard);
                let area = Rect::new(0, 0, width, height);
                let (body, _) = split_frame(area);
                let list = interior(
                    split_body(body, dashboard.route)
                        .0
                        .expect("the list region is drawn"),
                    Gutters::Both,
                );

                let all = rows(&dashboard, list.width);
                let cursor = all.iter().position(|r| r.selected).unwrap_or(0);
                let offset = viewport(all.len(), cursor, list.height);

                for row in 0..list.height {
                    let drawn = all.get(offset + row as usize);
                    assert_eq!(
                        row_at(&dashboard, list, row),
                        drawn.map(|r| r.kind),
                        "{width}x{height} offset {row}"
                    );
                    if let Some(drawn) = drawn {
                        // Cell by cell rather than by slicing `row_text`: the
                        // border characters either side are multi-byte, so a byte
                        // index into that string is not a column index.
                        let painted: String = (list.x..list.x + list.width)
                            .map(|x| buffer[(x, list.y + row)].symbol().to_string())
                            .collect();
                        assert_eq!(
                            painted, drawn.text,
                            "{width}x{height} offset {row} draws its own row"
                        );
                    }
                }
                // Past the last drawn row, nothing is reported.
                assert_eq!(row_at(&dashboard, list, list.height), None);
                assert_eq!(row_at(&dashboard, list, u16::MAX), None);
            }
        }

        #[test]
        fn the_hit_test_agrees_with_what_was_drawn() {
            // Both mandated frames, written unsuffixed: `WIDTHS`' number scan is
            // `\b(\d+)\b` and does not see `120u16`.
            let frames: [(u16, u16); 2] = [(120, 40), (60, 20)];
            for (width, height) in frames {
                for route in [Route::List, Route::Detail] {
                    let dashboard = populated(route);
                    let buffer = render_at(width, height, &dashboard);
                    let area = Rect::new(0, 0, width, height);
                    let (body, _) = split_frame(area);
                    let (list_area, divider, detail_area) = split_body(body, route);
                    // `layout::detail_gutters`: `Gutters::LeftOnly` only at the wide
                    // layout, where the divider spends the detail region's trailing
                    // gutter — `render_body`'s own choice, derived rather than
                    // re-decided here (Change Review of `pane-chrome`'s
                    // `detail_gutters` fix).
                    let detail_gutters = detail_gutters(divider);

                    let drawn: Option<(Rect, Vec<crate::ui::list::Row>, usize)> =
                        list_area.map(|a| {
                            let inner = interior(a, Gutters::Both);
                            let all = rows(&dashboard, inner.width);
                            let cursor = all.iter().position(|r| r.selected).unwrap_or(0);
                            let offset = viewport(all.len(), cursor, inner.height);
                            (inner, all, offset)
                        });

                    for y in 0..height {
                        for x in 0..width {
                            let symbol = buffer[(x, y)].symbol().to_string();
                            match zone(area, route, x, y) {
                                Zone::ListRow {
                                    interior: inner,
                                    row,
                                } => {
                                    let (_, all, offset) = drawn
                                        .as_ref()
                                        .expect("a ListRow implies a drawn list region");
                                    let text = all
                                        .get(offset + row as usize)
                                        .map(|r| r.text.as_str())
                                        .unwrap_or("");
                                    let column = (x - inner.x) as usize;
                                    let expected = text
                                        .chars()
                                        .nth(column)
                                        .map(|c| c.to_string())
                                        .unwrap_or_else(|| " ".to_string());
                                    assert_eq!(
                                        symbol, expected,
                                        "{width}x{height} {route:?} ({x}, {y}) is a ListRow \
                                         holding a character from list::rows' own output"
                                    );
                                }
                                Zone::List => {
                                    let region =
                                        list_area.expect("a List zone implies a list region");
                                    // `Gutters::Both`: both edge columns are gutters.
                                    let gutter = x == region.x || x + 1 == region.x + region.width;
                                    if gutter {
                                        assert_eq!(
                                            symbol, " ",
                                            "{width}x{height} {route:?} ({x}, {y}) is a List \
                                             zone in a gutter column"
                                        );
                                    }
                                }
                                Zone::Detail => {
                                    let region =
                                        detail_area.expect("a Detail zone implies a detail region");
                                    let gutter = x == region.x
                                        || (detail_gutters == Gutters::Both
                                            && x + 1 == region.x + region.width);
                                    if gutter {
                                        assert_eq!(
                                            symbol, " ",
                                            "{width}x{height} {route:?} ({x}, {y}) is a Detail \
                                             zone in a gutter column"
                                        );
                                    }
                                }
                                Zone::DetailTab { .. } | Zone::Outside => {}
                            }
                        }
                    }

                    if let Some(col) = divider {
                        for y in body.y..body.y + body.height {
                            assert_eq!(
                                buffer[(col, y)].symbol(),
                                "│",
                                "{width}x{height} {route:?} the divider column holds │"
                            );
                        }
                    }

                    // No cell is classified as belonging to a region the draw path
                    // did not draw.
                    if list_area.is_none() {
                        for y in 0..height {
                            for x in 0..width {
                                assert!(!matches!(
                                    zone(area, route, x, y),
                                    Zone::List | Zone::ListRow { .. }
                                ));
                            }
                        }
                    }
                    if detail_area.is_none() {
                        for y in 0..height {
                            for x in 0..width {
                                assert!(!matches!(
                                    zone(area, route, x, y),
                                    Zone::Detail | Zone::DetailTab { .. }
                                ));
                            }
                        }
                    }
                }
            }
        }

        /// `Gutters::LeftOnly` was hardcoded in `layout::zone` regardless of
        /// layout mode, while `render_body` derives it — `Gutters::Both` at
        /// the narrow layout, since there is no divider to spend a gutter on.
        /// The tab bar's own width therefore disagreed by one column between
        /// the hit test and the draw path, so a click past the drawn bar
        /// could still resolve to a tab.
        ///
        /// Two artifacts, sized so their joined width fits the hit test's
        /// (wrong) 59-column bar but not the drawn 58-column one, pin the
        /// mismatch directly: `" a "` is 3 columns, and `" " + "b" * 53 + "
        /// "` is 55, so the two together with their one separating column
        /// (3 + 55 + 1 = 59) fit a 59-column bar but not a 58-column one.
        ///
        /// Only the narrow layout is exercised here: at the wide layout the
        /// divider is always present, so `zone`'s hardcoded `Gutters::LeftOnly`
        /// already agreed with `render_body`'s derived choice there even
        /// before this fix.
        #[test]
        fn the_hit_test_agrees_with_the_drawn_tab_bar_at_the_narrow_layout() {
            // Only 60 is exercised below: at 120 the divider is always
            // present, so `zone`'s hardcoded `Gutters::LeftOnly` already
            // agreed with `render_body`'s derived choice there even before
            // this fix.
            let second_id = "b".repeat(53);
            let change = fixture::with_artifacts(
                fixture::active("detail-view", 4, 9),
                &[("a", &[]), (second_id.as_str(), &[])],
            );
            let d = super::dashboard_with_detail(
                vec![change],
                Vec::new(),
                1, // `list-sections`: target 0 is the active header.
                Route::Detail,
                super::empty_detail_with_tab("", Vec::new(), 0),
            );

            let width = 60u16;
            let height = 20u16;
            let buffer = render_at(width, height, &d);
            let area = Rect::new(0, 0, width, height);
            let (body, _) = split_frame(area);
            let detail_area = split_body(body, Route::Detail)
                .2
                .expect("a detail region is drawn at the narrow layout");
            // The real, drawn bar: `render_body` chooses `Gutters::Both` at
            // the narrow layout, since there is no divider to spend a gutter
            // on.
            let drawn_bar = interior(detail_area, Gutters::Both);
            let bar_row = drawn_bar.y;

            // Column 10 of the bar: past the single chip the drawn 58-column
            // bar actually shows (` a ` occupies columns 0..3), but inside
            // the second chip the hit test's own (wrong) 59-column bar would
            // place there.
            let column = 10u16;
            let x = drawn_bar.x + column;
            assert_eq!(
                buffer[(x, bar_row)].symbol(),
                " ",
                "the drawn 58-column bar has nothing at column {column}"
            );

            match zone(area, Route::Detail, x, bar_row) {
                Zone::DetailTab {
                    bar,
                    column: hit_column,
                } => {
                    assert_eq!(
                        bar.width, drawn_bar.width,
                        "the hit test's own bar must be the width `render_body` actually \
                         drew, not a hardcoded `Gutters::LeftOnly`"
                    );
                    let resolved = crate::ui::detail::tab_at(
                        &d.selected_change().unwrap().artifacts,
                        d.detail.tab,
                        bar.width,
                        hit_column,
                    );
                    assert_eq!(
                        resolved, None,
                        "a click past the drawn bar resolved to a tab: {resolved:?}"
                    );
                }
                other => panic!("expected a DetailTab zone at ({x}, {bar_row}), got {other:?}"),
            }

            // A cell actually inside the drawn chip still resolves to it.
            let inside_x = drawn_bar.x + 1;
            match zone(area, Route::Detail, inside_x, bar_row) {
                Zone::DetailTab {
                    bar,
                    column: hit_column,
                } => {
                    let resolved = crate::ui::detail::tab_at(
                        &d.selected_change().unwrap().artifacts,
                        d.detail.tab,
                        bar.width,
                        hit_column,
                    );
                    assert_eq!(
                        resolved,
                        Some(0),
                        "the drawn chip's own column must resolve to it"
                    );
                }
                other => {
                    panic!("expected a DetailTab zone at ({inside_x}, {bar_row}), got {other:?}")
                }
            }
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

    /// A dashboard over real change fixtures, for the row-drawing scenarios
    /// this group adds. `changes-rows` and `list-selection` render this
    /// change set every way; the plain `dashboard` helper above stays as it
    /// is for the pre-existing frame/footer/border scenarios that render
    /// `changes::empty_set()`.
    fn dashboard_with(
        active: Vec<Change>,
        archived: Vec<Change>,
        selected: usize,
        route: Route,
    ) -> Dashboard {
        Dashboard {
            repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
            searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
            changes: fixture::set(active, archived, Vec::new()),
            route,
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
            sections: crate::ui::app::Sections {
                collapsed: std::collections::BTreeSet::new(),
            },
            file_mode: false,
        }
    }

    /// `dashboard_with`, but with an explicit `Detail` — `detail-view`'s
    /// header/tab-bar/content scenarios need a selected change **and** a
    /// specific `detail.tab`, `detail.source`, or `detail.problems`.
    fn dashboard_with_detail(
        active: Vec<Change>,
        archived: Vec<Change>,
        selected: usize,
        route: Route,
        detail: Detail,
    ) -> Dashboard {
        Dashboard {
            repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
            searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
            changes: fixture::set(active, archived, Vec::new()),
            route,
            quit: false,
            selected,
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

    /// `count` active changes named `change-00`, `change-01`, ... — the
    /// generated (not hand-written) fixture design.md -> Decisions calls for
    /// the boundary and overflow scenarios.
    fn changes_named(count: usize) -> Vec<Change> {
        (0..count)
            .map(|i| fixture::active(&format!("change-{i:02}"), 1, 2))
            .collect()
    }

    // `Dashboard::changes` is `changes::empty_set()`, per responsive-layout's
    // preamble — every scenario in this capability renders an empty `ChangeSet`.
    fn dashboard(repo: Option<&str>, route: Route) -> Dashboard {
        Dashboard {
            repo: repo.map(std::path::PathBuf::from),
            searched_from: std::path::PathBuf::from("/tmp/searched-from"),
            changes: empty_set(),
            route,
            quit: false,
            selected: 0,
            filter: crate::ui::app::Filter {
                query: String::new(),
                active: false,
            },
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
            sections: crate::ui::app::Sections {
                collapsed: std::collections::BTreeSet::new(),
            },
            file_mode: false,
        }
    }

    /// `dashboard`'s twin with `file_mode: true` — `degraded-states`' addition, kept as its
    /// own fully-spelled literal rather than a `..dashboard(repo, route)` update, on this
    /// module's own established rule that every construction names all thirteen fields.
    fn dashboard_in_file_mode(repo: Option<&str>, route: Route) -> Dashboard {
        Dashboard {
            repo: repo.map(std::path::PathBuf::from),
            searched_from: std::path::PathBuf::from("/tmp/searched-from"),
            changes: empty_set(),
            route,
            quit: false,
            selected: 0,
            filter: crate::ui::app::Filter {
                query: String::new(),
                active: false,
            },
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
            sections: crate::ui::app::Sections {
                collapsed: std::collections::BTreeSet::new(),
            },
            file_mode: true,
        }
    }

    /// An in-scope `Agent` at `/tmp/demo-repo` named `name`, matching no change —
    /// `agent-attribution`'s own footer-count fixtures build every unattributed
    /// agent this way rather than through a `Change` literal.
    fn unattributed_agent(name: &str) -> crate::agents::Agent {
        crate::agents::Agent {
            name: Some(name.to_string()),
            kind: None,
            status: crate::agents::AgentStatus::Working,
            cwd: Some(std::path::PathBuf::from("/tmp/demo-repo")),
            pane_id: "p".to_string(),
            tab_id: "t".to_string(),
            workspace_id: "w".to_string(),
            terminal_title: None,
        }
    }

    /// Columns `range` of `text`, by character index — never by byte offset,
    /// since a box-drawing border or the header's `…` are multi-byte.
    fn cols(text: &str, range: std::ops::Range<usize>) -> String {
        text.chars().skip(range.start).take(range.len()).collect()
    }

    /// The list region's interior columns of buffer row `y` — the 38 or 58
    /// character-index range the buffer's width mandates, derived once
    /// rather than re-typed as `1..39` or `1..59` at each call site. The
    /// *expectations* stay written out per test, per design.md -> Risks; only
    /// the geometry is shared.
    fn interior_cols(buf: &Buffer, y: u16) -> String {
        let last = if buf.area.width == 60 { 58 } else { 38 };
        cols(&row_text(buf, y), 1..last + 1)
    }

    fn count_char(buf: &Buffer, needle: char) -> usize {
        (0..buf.area.height)
            .map(|y| row_text(buf, y).chars().filter(|c| *c == needle).count())
            .sum()
    }

    fn buffer_contains(buf: &Buffer, needle: &str) -> bool {
        (0..buf.area.height).any(|y| row_text(buf, y).contains(needle))
    }

    /// The style an untouched cell carries. Ratatui fills a cell it has not coloured
    /// with its own reset colour rather than leaving the field empty, so "carries no
    /// colour" is equality with this — read from `Cell::default()` because this file may
    /// not name a colour at all (design.md -> Decision 2).
    fn uncoloured() -> Style {
        Cell::default().style()
    }

    fn is_bold(cell: &Cell) -> bool {
        cell.style().add_modifier.contains(Modifier::BOLD)
    }

    fn is_dim(cell: &Cell) -> bool {
        cell.style().add_modifier.contains(Modifier::DIM)
    }

    #[test]
    fn body_and_footer_occupy_their_rows_at_both_widths() {
        let d = dashboard(Some("/tmp/demo-repo"), Route::List);
        // `agent-launch`: pinned unreachable, so this footer is the one this capability
        // specified before the action hints existed.
        assert!(!d.agents.reachable);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            // Row 0 is the routed region's heading row — it spells `demo-repo` from
            // column 1 — and the literal `OpenSpec` appears nowhere: `pane-chrome`
            // removes the frame's own header row entirely.
            assert_eq!(
                cols(&row_text(&buf, 0), 1..10),
                "demo-repo",
                "width {width}"
            );
            assert!(!buffer_contains(&buf, "OpenSpec"), "width {width}");

            let footer = row_text(&buf, 19);
            assert!(
                footer.starts_with("q quit  Enter detail  Esc back"),
                "width {width}"
            );
            assert!(
                footer.chars().skip(30).all(|c| c == ' '),
                "width {width}: footer remainder must be all spaces: {footer:?}"
            );

            // No box-drawing character appears in column 0 or in the last column of
            // any row, so the body occupies rows 0 through 18 with no bordered block
            // in it and nothing is drawn in row 19 by the body.
            for y in 0..=18u16 {
                assert_eq!(cell(&buf, 0, y).symbol(), " ", "width {width}: y={y}");
                assert_eq!(
                    cell(&buf, width - 1, y).symbol(),
                    " ",
                    "width {width}: y={y}"
                );
            }
        }
    }

    #[test]
    fn zero_height_frame_draws_nothing() {
        // responsive-layout: "at height 0 render SHALL draw nothing." Closes a
        // Change Review finding — split_frame's own height-0 case was tested at
        // the layout tier (ui::layout::tests::split_frame_is_body_then_footer)
        // but render's two early returns (render_header, render_footer) were
        // not exercised at the view tier at all. A 0-height buffer has no rows
        // to read, so the assertion is "did not panic" — the strongest claim a
        // zero-cell buffer admits.
        let d = dashboard(Some("/tmp/demo-repo"), Route::List);
        for width in [60, 120] {
            let buf = render_at(width, 0, &d);
            assert_eq!(buf.area.height, 0);
        }
    }

    #[test]
    fn a_one_row_frame_renders_the_body_s_heading_row_and_nothing_else() {
        let d = dashboard(Some("/tmp/demo-repo"), Route::List);
        for width in [60, 120] {
            let buf = render_at(width, 1, &d);
            // Row 0 spells `demo-repo` from column 1 — the routed region's heading
            // row, which is the body's only row at this height.
            assert_eq!(
                cols(&row_text(&buf, 0), 1..10),
                "demo-repo",
                "width {width}"
            );
            // No `q quit` appears anywhere: the footer was not drawn into the
            // body's single row.
            assert!(!buffer_contains(&buf, "q quit"), "width {width}");
        }
    }

    #[test]
    fn a_two_row_frame_renders_one_body_row_and_the_footer() {
        let d = dashboard(Some("/tmp/demo-repo"), Route::List);
        for width in [60, 120] {
            let buf = render_at(width, 2, &d);
            assert_eq!(
                cols(&row_text(&buf, 0), 1..10),
                "demo-repo",
                "width {width}"
            );
            assert_eq!(cols(&row_text(&buf, 1), 0..6), "q quit", "width {width}");
            // No list row is drawn anywhere: the body received one row and the
            // heading row consumed it.
            assert!(!buffer_contains(&buf, "No changes yet"), "width {width}");
        }
    }

    #[test]
    fn a_one_column_frame_renders_without_panicking() {
        let d = dashboard(Some("/tmp/demo-repo"), Route::List);
        let buf = render_at(1, 1, &d);
        let _ = buf;
        let buf = render_at(1, 20, &d);
        // The region's one column is its left gutter, its interior is zero columns
        // wide, so the heading row is truncated to nothing rather than drawn over
        // the gutter — row 0 is a single space.
        assert_eq!(row_text(&buf, 0), " ");
        // `q quit` needs six columns, so the first hint is dropped whole rather
        // than truncated — row 19 is a single space too.
        assert_eq!(row_text(&buf, 19), " ");
        // Extended: a 2x20 render, so a zero-column list-region interior is
        // exercised too (a 2-wide frame's list region has width 2, and
        // `layout::interior` of that is width 0).
        let buf = render_at(2, 20, &d);
        let _ = buf;
        // The two mandated widths, as contrasting controls: the one-column result
        // above is a width branch rather than the heading and footer being absent
        // everywhere.
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert_eq!(
                cols(&row_text(&buf, 0), 1..10),
                "demo-repo",
                "width {width}"
            );
            assert_eq!(cols(&row_text(&buf, 19), 0..6), "q quit", "width {width}");
        }

        // `agent-launch`: the same holds with `agents.reachable` `true`, which adds no hint
        // that could fit in one column and therefore changes no cell of the 1x20 buffer.
        let mut reachable = d.clone();
        reachable.agents.reachable = true;
        let buf1 = render_at(1, 20, &d);
        let buf1_reachable = render_at(1, 20, &reachable);
        assert_eq!(buf1, buf1_reachable);
    }

    #[test]
    fn footer_drops_whole_hints() {
        let d = dashboard(Some("/tmp/demo-repo"), Route::List);
        // `agent-launch`: pinned unreachable, so the action hints never enter this test's
        // own drop sequence.
        assert!(!d.agents.reachable);
        let buf = render_at(18, 20, &d);
        assert_eq!(row_text(&buf, 19), format!("q quit{}", " ".repeat(12)));

        let buf = render_at(20, 20, &d);
        assert_eq!(row_text(&buf, 19), "q quit  Enter detail");

        let buf = render_at(60, 20, &d);
        assert_eq!(
            row_text(&buf, 19),
            format!("q quit  Enter detail  Esc back{}", " ".repeat(30))
        );

        let buf = render_at(120, 20, &d);
        assert_eq!(
            row_text(&buf, 19),
            format!("q quit  Enter detail  Esc back{}", " ".repeat(90))
        );
    }

    /// `responsive-layout`: "The unattributed count is the footer's last hint at
    /// both widths".
    #[test]
    fn the_unattributed_count_is_the_last_hint() {
        let mut d = dashboard_with(
            vec![fixture::active("alpha", 0, 1)],
            Vec::new(),
            0,
            Route::List,
        );
        d.agents.agents = vec![unattributed_agent("nothing-like-a-change")];

        let expected = "q quit  Enter detail  Esc back  1 unattributed";
        assert_eq!(columns(expected), 46);
        let buf60 = render_at(60, 20, &d);
        assert_eq!(
            row_text(&buf60, 19),
            format!("{expected}{}", " ".repeat(14))
        );
        let buf120 = render_at(120, 20, &d);
        assert_eq!(
            row_text(&buf120, 19),
            format!("{expected}{}", " ".repeat(74))
        );

        let mut without_agents = d.clone();
        without_agents.agents.agents = Vec::new();
        assert_eq!(
            row_text(&render_at(60, 20, &without_agents), 19),
            format!("q quit  Enter detail  Esc back{}", " ".repeat(30)),
            "byte-identical to the row this capability specified before the count existed"
        );

        d.agents.agents.push(unattributed_agent("also-nothing"));
        for width in [60u16, 120u16] {
            assert!(
                row_text(&render_at(width, 20, &d), 19)
                    .starts_with("q quit  Enter detail  Esc back  2 unattributed"),
                "width {width}: the count is over agents, adding a second must read 2"
            );
        }

        // `agent-launch`: with `agents.reachable` set, the action hints sit between
        // `Esc back` and the count, and at 60 the count is the one dropped.
        let mut reachable = d.clone();
        reachable.agents.agents = vec![unattributed_agent("nothing-like-a-change")];
        reachable.agents.reachable = true;
        let expected_120 = "q quit  Enter detail  Esc back  a/c/s launch  g focus  1 unattributed";
        assert_eq!(columns(expected_120), 69);
        assert_eq!(
            row_text(&render_at(120, 20, &reachable), 19),
            format!("{expected_120}{}", " ".repeat(51))
        );
        let expected_60 = "q quit  Enter detail  Esc back  a/c/s launch  g focus";
        assert_eq!(
            row_text(&render_at(60, 20, &reachable), 19),
            format!("{expected_60}{}", " ".repeat(7))
        );
    }

    /// `responsive-layout`: "The count is reported with an empty change list".
    #[test]
    fn the_count_is_reported_with_an_empty_list() {
        let mut d = dashboard(Some("/tmp/demo-repo"), Route::List);
        d.agents.agents = vec![
            unattributed_agent("nothing-like-a-change"),
            unattributed_agent("also-nothing"),
        ];
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(
                row_text(&buf, 19).starts_with("q quit  Enter detail  Esc back  2 unattributed")
            );
            assert!(buffer_contains(&buf, "No changes yet"));
        }

        let agentless_no_match = {
            let mut x = dashboard(Some("/tmp/demo-repo"), Route::List);
            x.filter.query = "zzz".to_string();
            x
        };
        let mut no_match = agentless_no_match.clone();
        no_match.agents.agents = d.agents.agents.clone();
        for width in [60, 120] {
            let buf = render_at(width, 20, &no_match);
            let buf_agentless = render_at(width, 20, &agentless_no_match);
            for y in 2..=17u16 {
                assert_eq!(
                    row_text(&buf, y),
                    row_text(&buf_agentless, y),
                    "width {width} row {y}: the message rows must be byte-identical"
                );
            }
            assert!(
                row_text(&buf, 19).contains("2 unattributed"),
                "width {width}"
            );
        }

        // `agent-launch`: with `agents.reachable` set, the list region's interior is
        // unchanged, cell for cell, at both widths — the action hints live in the footer
        // and never in the list.
        let mut reachable = d.clone();
        reachable.agents.reachable = true;
        for width in [60, 120] {
            let unreachable_buf = render_at(width, 20, &d);
            let reachable_buf = render_at(width, 20, &reachable);
            for y in 2..=17u16 {
                assert_eq!(
                    row_text(&unreachable_buf, y),
                    row_text(&reachable_buf, y),
                    "width {width} row {y}"
                );
            }
        }
    }

    /// `responsive-layout`: "The count is dropped whole before the three key hints".
    #[test]
    fn the_count_drops_before_the_key_hints() {
        let mut d = dashboard(Some("/tmp/demo-repo"), Route::List);
        d.agents.agents = vec![unattributed_agent("nothing-like-a-change")];

        let buf46 = render_at(46, 20, &d);
        assert_eq!(
            row_text(&buf46, 19),
            "q quit  Enter detail  Esc back  1 unattributed"
        );
        let buf45 = render_at(45, 20, &d);
        assert_eq!(
            row_text(&buf45, 19),
            format!("q quit  Enter detail  Esc back{}", " ".repeat(15))
        );

        // 60 and 120 as controls: comfortably wide enough that the count is never
        // dropped there.
        for width in [60, 120] {
            assert!(row_text(&render_at(width, 20, &d), 19).contains("1 unattributed"));
        }

        // `agent-launch`: with `agents.reachable` set, the boundary moves one hint list
        // further out — `g focus` is `agent-launch`'s addition and the count still drops
        // before the (now longer) key hint list.
        let mut reachable = d.clone();
        reachable.agents.reachable = true;
        let buf69 = render_at(69, 20, &reachable);
        assert_eq!(
            row_text(&buf69, 19),
            "q quit  Enter detail  Esc back  a/c/s launch  g focus  1 unattributed"
        );
        let buf68 = render_at(68, 20, &reachable);
        assert_eq!(
            row_text(&buf68, 19),
            format!(
                "q quit  Enter detail  Esc back  a/c/s launch  g focus{}",
                " ".repeat(15)
            )
        );
    }

    /// `responsive-layout`: "The filter prompt replaces the count along with the
    /// hints", carrying both its own fixture and `agent-attribution`'s "The `/`
    /// filter hides rows without changing the count" — one function, per
    /// design.md -> Test Strategy.
    #[test]
    fn the_count_survives_a_filter() {
        // `agent-attribution`'s own fixture: alpha + beta, one agent matching
        // alpha and two unattributable, query "beta".
        let mut ab = dashboard_with(
            vec![
                fixture::active("alpha", 0, 1),
                fixture::active("beta", 0, 1),
            ],
            Vec::new(),
            0,
            Route::List,
        );
        let mut alpha_agent = unattributed_agent("alpha");
        alpha_agent.status = crate::agents::AgentStatus::Working;
        ab.agents.agents = vec![
            alpha_agent,
            unattributed_agent("nothing-like-a-change"),
            unattributed_agent("also-nothing"),
        ];
        ab.filter.query = "beta".to_string();
        for width in [120, 60] {
            let buf = render_at(width, 20, &ab);
            assert!(
                !buffer_contains(&buf, "alpha"),
                "width {width}: the alpha row must be hidden by the filter"
            );
            assert!(
                row_text(&buf, 19).contains("2 unattributed"),
                "width {width}"
            );
        }

        // `responsive-layout`'s own fixture: one change, one unattributed agent.
        let mut base = dashboard(Some("/tmp/demo-repo"), Route::List);
        base.agents.agents = vec![unattributed_agent("nothing-like-a-change")];

        let mut active_form = base.clone();
        active_form.filter = Filter {
            query: "be".to_string(),
            active: true,
        };
        let mut accepted_form = base.clone();
        accepted_form.filter = Filter {
            query: "be".to_string(),
            active: false,
        };
        let mut active_form_no_agents = active_form.clone();
        active_form_no_agents.agents.agents = Vec::new();
        let mut accepted_form_no_agents = accepted_form.clone();
        accepted_form_no_agents.agents.agents = Vec::new();

        for width in [60, 120] {
            let active_footer = row_text(&render_at(width, 20, &active_form), 19);
            assert!(active_footer.starts_with("/be_"), "width {width}");
            assert!(!active_footer.contains("unattributed"), "width {width}");
            assert_eq!(
                active_footer,
                row_text(&render_at(width, 20, &active_form_no_agents), 19),
                "width {width}: byte-identical once agents are emptied"
            );

            let accepted_footer = row_text(&render_at(width, 20, &accepted_form), 19);
            assert!(
                accepted_footer.starts_with("/be  q quit  Enter detail  Esc back  1 unattributed"),
                "width {width}: {accepted_footer:?}"
            );
            let accepted_no_agents_footer =
                row_text(&render_at(width, 20, &accepted_form_no_agents), 19);
            assert!(accepted_no_agents_footer.starts_with("/be  q quit  Enter detail  Esc back"));
            assert!(!accepted_no_agents_footer.contains("unattributed"));
        }

        // `agent-launch`: with `agents.reachable` set, the active-filter row is still
        // exactly `/be_` (the prompt replaces the whole row, action hints included), and
        // the accepted-query row still leads with the query and still trails with the count.
        let mut active_reachable = active_form.clone();
        active_reachable.agents.reachable = true;
        let mut accepted_reachable = accepted_form.clone();
        accepted_reachable.agents.reachable = true;
        for width in [60, 120] {
            let active_footer = row_text(&render_at(width, 20, &active_reachable), 19);
            assert!(active_footer.starts_with("/be_"), "width {width}");
            assert!(!active_footer.contains("a/c/s launch"), "width {width}");
            assert!(!active_footer.contains("g focus"), "width {width}");
        }
        assert!(
            row_text(&render_at(120, 20, &accepted_reachable), 19).starts_with(
                "/be  q quit  Enter detail  Esc back  a/c/s launch  g focus  1 unattributed"
            )
        );
        assert!(
            row_text(&render_at(60, 20, &accepted_reachable), 19)
                .starts_with("/be  q quit  Enter detail  Esc back  a/c/s launch  g focus"),
            "width 60: the count is dropped, the action hints are kept"
        );
    }

    /// `change-rows`: "A badged row carries its status between the name and the
    /// progress cell" at the view tier.
    #[test]
    fn badged_rows_render_at_both_widths() {
        let mut d = three_active();
        let mut working = unattributed_agent("add-token-refresh");
        working.status = crate::agents::AgentStatus::Working;
        let mut blocked = unattributed_agent("fix-empty-basket");
        blocked.status = crate::agents::AgentStatus::Blocked;
        let mut unknown = unattributed_agent("migrate-ai-sdk-v7");
        unknown.status = crate::agents::AgentStatus::Unknown;
        d.agents.agents = vec![working, blocked, unknown];

        // `list-sections`: buffer row 2 is now the active section header; the three
        // change rows follow it at rows 3-5.
        let buf120 = render_at(120, 20, &d);
        assert_eq!(
            interior_cols(&buf120, 2),
            format!("{:<38}", "  ▾ active (3)")
        );
        assert_eq!(
            interior_cols(&buf120, 3),
            "> add-token-refresh            w [4/9]"
        );
        assert_eq!(
            interior_cols(&buf120, 4),
            "  fix-empty-basket             b [7/7]"
        );
        assert_eq!(
            interior_cols(&buf120, 5),
            "  migrate-ai-sdk-v7              ? [-]"
        );

        let buf60 = render_at(60, 20, &d);
        for y in [3u16, 4, 5] {
            assert_eq!(columns(&interior_cols(&buf60, y)), 58);
        }
        assert_eq!(interior_cols(&buf60, 3).chars().nth(51), Some('w'));
        assert_eq!(interior_cols(&buf60, 4).chars().nth(51), Some('b'));
        assert_eq!(interior_cols(&buf60, 5).chars().nth(53), Some('?'));
    }

    /// `agent-attribution`: "An unreachable socket yields no badge, no count, and
    /// no problem" — the rendered frame is byte-identical to the one the same
    /// dashboard produces with `agents.problem` set to `None`.
    #[test]
    fn an_unreachable_socket_renders_the_agentless_pane() {
        let mut with_problem = dashboard_with(
            vec![
                fixture::active("alpha", 0, 1),
                fixture::active("beta", 0, 1),
            ],
            Vec::new(),
            0,
            Route::List,
        );
        with_problem.agents = crate::agents::AgentSnapshot {
            agents: Vec::new(),
            reachable: false,
            stalled: false,
            problem: Some("herdr agent list: could not start herdr".to_string()),
        };
        let mut without_problem = with_problem.clone();
        without_problem.agents.problem = None;

        for width in [120, 60] {
            assert_eq!(
                render_at(width, 20, &with_problem),
                render_at(width, 20, &without_problem),
                "width {width}: the recorded reason must be carried and never drawn"
            );
        }
        // `agent-launch`: `changes.problems`, `refresh.problems`, and `launch.problems` are
        // all still empty, so nothing about the socket reached any problem list.
        assert!(with_problem.changes.problems.is_empty());
        assert!(with_problem.refresh.problems.is_empty());
        assert!(with_problem.launch.problems.is_empty());
    }

    #[test]
    fn at_120_columns_both_regions_are_drawn_with_the_divider_at_column_40() {
        let d = dashboard_with(
            vec![fixture::active("alpha", 1, 3)],
            Vec::new(),
            1,
            Route::List,
        );
        let buf = render_at(120, 20, &d);
        // Row 0 spells `demo-repo` from column 1 and `alpha`'s own change header
        // from column 42, so both regions drew their heading rows.
        assert_eq!(cols(&row_text(&buf, 0), 1..10), "demo-repo");
        assert!(cols(&row_text(&buf, 0), 42..47).starts_with("alpha"));

        for y in 0..=18u16 {
            assert_eq!(cell(&buf, 40, y).symbol(), "│", "y={y}");
            assert_eq!(cell(&buf, 0, y).symbol(), " ", "y={y}");
            assert_eq!(cell(&buf, 39, y).symbol(), " ", "y={y}");
            assert_eq!(cell(&buf, 41, y).symbol(), " ", "y={y}");
        }
        assert!(!buffer_contains(&buf, "Changes"));
        assert!(!buffer_contains(&buf, "Detail"));

        // The 60-column buffer, as the contrasting control at the mandated narrow
        // width, holds no divider at all: it is a width branch, not something
        // drawn unconditionally.
        let buf60 = render_at(60, 20, &d);
        assert!(!buffer_contains(&buf60, "│"));
    }

    #[test]
    fn at_60_columns_only_the_routed_region_is_drawn() {
        let d = dashboard_with(
            vec![fixture::active("alpha", 1, 3)],
            Vec::new(),
            1,
            Route::List,
        );
        let buf = render_at(60, 20, &d);
        // Row 0 spells `demo-repo` from column 1, and the change's own header —
        // identified by its `(tdd)` schema cell, which no list row carries —
        // appears in no row, because the detail region was not drawn.
        assert_eq!(cols(&row_text(&buf, 0), 1..10), "demo-repo");
        assert!(!buffer_contains(&buf, "(tdd)"));
        assert!(!buffer_contains(&buf, "│"));

        // The 120-column buffer, as the contrasting control at the mandated wide
        // width, does hold the change's header, so the absence at 60 columns is the
        // breakpoint and not the detail heading being missing.
        let buf120 = render_at(120, 20, &d);
        assert!(buffer_contains(&buf120, "(tdd)"));
    }

    #[test]
    fn at_60_columns_the_detail_route_replaces_the_list_region() {
        let d = dashboard_with(
            vec![fixture::active("alpha", 1, 3)],
            Vec::new(),
            1,
            Route::Detail,
        );
        let buf = render_at(60, 20, &d);
        // Row 0 spells the selected change's header from column 1, and
        // `demo-repo` appears in no row, because the list region — and with it
        // the repository heading — is not drawn.
        assert!(cols(&row_text(&buf, 0), 1..6).starts_with("alpha"));
        assert!(!buffer_contains(&buf, "demo-repo"));

        // The 120-column buffer still shows both, because the route selects
        // emphasis rather than visibility above the breakpoint.
        let buf120 = render_at(120, 20, &d);
        assert_eq!(cols(&row_text(&buf120, 0), 1..10), "demo-repo");
        assert!(cols(&row_text(&buf120, 0), 42..47).starts_with("alpha"));
    }

    #[test]
    fn the_breakpoint_is_exact_at_99_100_and_101_columns() {
        use crate::ui::layout::{LayoutMode, mode};

        for width in [0u16, 1, 40, 60, 99] {
            assert_eq!(mode(width), LayoutMode::Narrow, "width {width}");
        }
        for width in [100u16, 101, 120, u16::MAX] {
            assert_eq!(mode(width), LayoutMode::Wide, "width {width}");
        }

        let d = dashboard(Some("/tmp/demo-repo"), Route::List);
        for width in [60, 99] {
            let buf = render_at(width, 20, &d);
            assert!(!buffer_contains(&buf, "│"), "width {width}");
        }
        for width in [100, 101, 120] {
            let buf = render_at(width, 20, &d);
            for y in 0..=18u16 {
                assert_eq!(cell(&buf, 40, y).symbol(), "│", "width {width}: y={y}");
            }
        }
    }

    #[test]
    fn the_mode_follows_the_current_frame_not_the_startup_size() {
        // `Dashboard` exposes no field naming a width, a layout mode, or a column
        // count — a structural claim, unenforceable at this tier, but a `Dashboard`
        // literal that named one would fail to compile against every other test in
        // this module.
        let d = dashboard(Some("/tmp/demo-repo"), Route::List);
        let backend = TestBackend::new(120, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        terminal
            .draw(|frame| super::render(frame, &d))
            .expect("draw first frame");
        let first = terminal.backend().buffer().clone();
        terminal.backend_mut().resize(60, 20);
        terminal
            .draw(|frame| super::render(frame, &d))
            .expect("draw second frame");
        let second = terminal.backend().buffer().clone();
        assert_eq!(
            count_char(&first, '│'),
            19,
            "the first frame holds the divider"
        );
        assert_eq!(
            count_char(&second, '│'),
            0,
            "the second frame holds no divider"
        );
    }

    #[test]
    fn the_routed_region_s_border_is_bold_and_the_other_s_is_not() {
        // The scenario's name is kept verbatim (design.md -> Verification matrix):
        // there is no border any more, and the claim it made — the routed region is
        // the emphasised one — is now made by the heading row.
        let list = dashboard_with(
            vec![fixture::active("alpha", 1, 3)],
            Vec::new(),
            1,
            Route::List,
        );
        let buf = render_at(120, 20, &list);
        assert!(is_bold(cell(&buf, 1, 0)));
        assert!(!is_bold(cell(&buf, 42, 0)));
        assert!(is_dim(cell(&buf, 42, 0)));

        let detail = dashboard_with(
            vec![fixture::active("alpha", 1, 3)],
            Vec::new(),
            1,
            Route::Detail,
        );
        let buf = render_at(120, 20, &detail);
        assert!(!is_bold(cell(&buf, 1, 0)));
        assert!(is_bold(cell(&buf, 42, 0)));

        for route in [Route::List, Route::Detail] {
            let d = dashboard_with(vec![fixture::active("alpha", 1, 3)], Vec::new(), 1, route);
            let buf = render_at(60, 20, &d);
            assert!(is_bold(cell(&buf, 1, 0)), "route {route:?}");
        }
    }

    #[test]
    fn region_interiors_are_blank() {
        // Rewritten for list-view: only the DETAIL interior is blank now —
        // change-rows owns every cell of the list interior, and an empty
        // ChangeSet with a repository root renders "No changes yet" there.
        let d = dashboard(Some("/tmp/demo-repo"), Route::List);
        let default_style = Cell::default().style();

        let buf = render_at(120, 20, &d);
        for y in 2..=17u16 {
            for x in 41..=118u16 {
                let c = cell(&buf, x, y);
                assert_eq!(c.symbol(), " ", "x={x} y={y}");
                assert_eq!(c.style(), default_style, "x={x} y={y}");
            }
        }
        assert!(interior_cols(&buf, 2).starts_with("No changes yet"));

        let buf = render_at(60, 20, &d);
        assert!(interior_cols(&buf, 2).starts_with("No changes yet"));
        for y in 3..=17u16 {
            for x in 1..=58u16 {
                let c = cell(&buf, x, y);
                assert_eq!(c.symbol(), " ", "x={x} y={y}");
                assert_eq!(c.style(), default_style, "x={x} y={y}");
            }
        }
    }

    /// `responsive-layout` :: "The heading names the directory, not the path, at both
    /// widths" — `pane-chrome`'s replacement for the frame header's absolute repository
    /// path: the list region's own heading row now names the directory alone.
    #[test]
    fn the_heading_names_the_directory_not_the_path_at_both_widths() {
        let d = dashboard(Some("/Users/dev/Code/herdr-openspec"), Route::List);
        for width in [120, 60] {
            let buf = render_at(width, 20, &d);
            assert_eq!(
                cols(&row_text(&buf, 0), 1..15),
                "herdr-openspec",
                "width {width}"
            );
            assert!(
                !buffer_contains(&buf, "/Users/dev/Code"),
                "width {width}: the absolute path must not appear"
            );
        }
        let buf = render_at(120, 20, &d);
        assert_eq!(
            cols(&row_text(&buf, 0), 38..39),
            " ",
            "the heading stayed inside the list region's interior"
        );
    }

    /// `responsive-layout` :: "The badge is right-aligned and dropped whole" — the badge
    /// sits at the heading row's own right edge, not at a fixed column after a label that no
    /// longer exists, and disappears whole rather than being cut short once the row cannot
    /// hold the name, a separating blank, and its own nine columns together. `120` and `60`
    /// exercise the mandated widths; `21` and `20` pin the drop rule's own boundary — one
    /// column narrower than the rule needs is the first width at which the badge goes.
    #[test]
    fn the_badge_is_right_aligned_and_dropped_whole() {
        let repo = Some("/tmp/demo-repo");
        for width in [120u16, 60] {
            let d = dashboard_in_file_mode(repo, Route::List);
            let buf = render_at(width, 20, &d);
            let iw = if width >= 100 { 38u16 } else { width - 2 };
            assert_eq!(
                cols(&row_text(&buf, 0), 1..10),
                "demo-repo",
                "width {width}"
            );
            let badge_start = 1 + iw - 9;
            assert_eq!(
                cols(
                    &row_text(&buf, 0),
                    badge_start as usize..(badge_start + 9) as usize
                ),
                "file mode",
                "width {width}"
            );
            for x in badge_start..badge_start + 9 {
                assert!(
                    cell(&buf, x, 0)
                        .style()
                        .add_modifier
                        .contains(Modifier::DIM),
                    "width {width}: column {x} of the badge is not dim"
                );
                assert_eq!(
                    cell(&buf, x, 0).style().fg,
                    palette::style(Role::FileMode).fg,
                    "width {width}: column {x} of the badge does not carry FileMode's foreground"
                );
            }
            // The name is bold — the routed region's own heading — and carries no colour, so
            // the badge is distinguishable from it by weight and by colour, not only by
            // position.
            for x in 1..10u16 {
                assert!(is_bold(cell(&buf, x, 0)), "width {width}: name column {x}");
                assert_eq!(
                    cell(&buf, x, 0).style().fg,
                    uncoloured().fg,
                    "width {width}: name column {x} carries a foreground"
                );
            }
        }

        // 21 columns: a heading row of 19 columns — `demo-repo`'s nine, one separating
        // blank, and the badge's nine fit exactly.
        let d = dashboard_in_file_mode(repo, Route::List);
        let buf21 = render_at(21, 20, &d);
        assert_eq!(cols(&row_text(&buf21, 0), 1..10), "demo-repo");
        assert_eq!(cols(&row_text(&buf21, 0), 10..11), " ");
        assert_eq!(cols(&row_text(&buf21, 0), 11..20), "file mode");

        // 20 columns: a heading row of 18 columns — the badge is dropped whole rather than
        // cut, and `demo-repo` is drawn whole in its place.
        let buf20 = render_at(20, 20, &d);
        assert!(
            !row_text(&buf20, 0).contains("file mode"),
            "the badge must be dropped whole below 21 columns: {:?}",
            row_text(&buf20, 0)
        );
        assert_eq!(cols(&row_text(&buf20, 0), 1..10), "demo-repo");

        // Additive: with `file_mode` false the 120-column row is byte-identical except for
        // the missing badge — the name's own columns are unaffected by whether it is drawn.
        let plain_buf = render_at(120, 20, &dashboard(repo, Route::List));
        assert_eq!(
            cols(&row_text(&plain_buf, 0), 1..10),
            cols(&row_text(&buf21, 0), 1..10)
        );
        assert!(!buffer_contains(&plain_buf, "file mode"));
    }

    /// `responsive-layout` :: "A name longer than the heading row keeps its tail" — the
    /// heading row's own degraded state, reachable through no happy-path fixture above:
    /// a fifty-character directory name is shortened from the **left**, the same
    /// `shorten_left` rule `change-rows` uses for a row too long for its field, keeping
    /// the characters that distinguish a directory from its siblings.
    #[test]
    fn a_name_longer_than_the_heading_row_keeps_its_tail() {
        let name: String = (0..50).map(|i| char::from(b'a' + (i % 26) as u8)).collect();
        assert_eq!(name.chars().count(), 50);
        let repo = format!("/tmp/{name}");
        let d = dashboard(Some(&repo), Route::List);

        // At 120 columns the list region's 38-column interior cannot hold the name
        // whole: `…` plus the name's last 37 columns fills it exactly.
        let buf120 = render_at(120, 20, &d);
        let tail: String = name.chars().skip(50 - 37).collect();
        assert_eq!(
            cols(&row_text(&buf120, 0), 1..39),
            format!("…{tail}"),
            "the ellipsis plus the last 37 columns must fill the 38-column heading"
        );

        // At 60 columns the 58-column interior holds all fifty characters whole, so the
        // ellipsis above is the narrower list column and not a constant.
        let buf60 = render_at(60, 20, &d);
        assert_eq!(
            cols(&row_text(&buf60, 0), 1..51),
            name,
            "50 columns fit in 58, so no shortening is needed at the wider row"
        );
    }

    /// `responsive-layout` :: "No repository names itself in the heading" — the heading
    /// row's other degraded state: with no repository root at all, the row spells the
    /// literal `no repository` rather than being left blank, and the list interior still
    /// carries `change-rows`' own three-row "no repository" message beneath it.
    #[test]
    fn no_repository_names_itself_in_the_heading() {
        let mut d = dashboard(None, Route::List);
        d.searched_from = std::path::PathBuf::from("/tmp/not-a-repo/deep/here");

        for width in [120, 60] {
            let buf = render_at(width, 20, &d);
            assert_eq!(
                cols(&row_text(&buf, 0), 1..14),
                "no repository",
                "width {width}"
            );
            // Row 1 is the region's blank padding row; the interior — `change-rows`' own
            // three-row "no repository" message — starts at row 2, with the searched
            // path itself on row 4.
            assert!(
                interior_cols(&buf, 2).starts_with("No OpenSpec repository found"),
                "width {width}"
            );
            assert!(
                interior_cols(&buf, 4).contains("not-a-repo"),
                "width {width}: the searched path must appear in the interior"
            );
            assert!(
                !cols(&row_text(&buf, 0), 0..width as usize).contains("not-a-repo"),
                "width {width}: the searched path must not appear in the heading row"
            );
        }
    }

    /// `responsive-layout` :: "The routed region's heading is bold and the other's is
    /// dim" — distinct from "The routed region's border is bold and the other's is not"
    /// further below: `pane-chrome` deletes the border that scenario named, but
    /// design.md keeps both spec scenarios, since task 3.9 could not re-baseline one
    /// that asserts a shape rather than a row index, and the two are drawn by different
    /// call sites (`render_region` for the list heading, `render_detail_header` for the
    /// detail one).
    #[test]
    fn the_routed_region_s_heading_is_bold_and_the_other_s_is_dim() {
        let list = dashboard_with(
            vec![fixture::active("alpha", 1, 3)],
            Vec::new(),
            1,
            Route::List,
        );
        let buf = render_at(120, 20, &list);
        assert!(is_bold(cell(&buf, 1, 0)));
        assert!(!is_dim(cell(&buf, 1, 0)));
        assert!(!is_bold(cell(&buf, 42, 0)));
        assert!(is_dim(cell(&buf, 42, 0)));

        let detail = dashboard_with(
            vec![fixture::active("alpha", 1, 3)],
            Vec::new(),
            1,
            Route::Detail,
        );
        let buf = render_at(120, 20, &detail);
        assert!(!is_bold(cell(&buf, 1, 0)));
        assert!(is_dim(cell(&buf, 1, 0)));
        assert!(is_bold(cell(&buf, 42, 0)));
        assert!(!is_dim(cell(&buf, 42, 0)));

        for route in [Route::List, Route::Detail] {
            let d = dashboard_with(vec![fixture::active("alpha", 1, 3)], Vec::new(), 1, route);
            let buf = render_at(60, 20, &d);
            assert!(is_bold(cell(&buf, 1, 0)), "route {route:?}");
        }
    }

    /// `responsive-layout` :: "A one-, two-, and three-column frame degenerates without
    /// drawing over a gutter" — the divider requirement's own degenerate case, both
    /// below the breakpoint: at one and two columns the list region's interior is zero
    /// columns wide, and at three columns it is exactly one, wide enough to draw into
    /// but not to leave any margin either side of it.
    #[test]
    fn a_one_two_and_three_column_frame_degenerates_without_drawing_over_a() {
        let d = dashboard_with(
            vec![fixture::active("alpha", 1, 3)],
            Vec::new(),
            1,
            Route::List,
        );

        // Neither of the three panics.
        let buf1 = render_at(1, 20, &d);
        let buf2 = render_at(2, 20, &d);
        let buf3 = render_at(3, 20, &d);

        // At 1 and 2 columns the interior is zero columns wide: no heading text and
        // no list row is drawn at all.
        for buf in [&buf1, &buf2] {
            assert!(!buffer_contains(buf, "demo-repo"));
            assert!(!buffer_contains(buf, "alpha"));
        }

        // At 3 columns the interior is exactly one column: the heading row and every
        // list row occupy column 1 alone, and columns 0 and 2 stay spaces on every row.
        for y in 0..=18u16 {
            assert_eq!(cell(&buf3, 0, y).symbol(), " ", "y={y}");
            assert_eq!(cell(&buf3, 2, y).symbol(), " ", "y={y}");
        }
        assert_ne!(
            cell(&buf3, 1, 0).symbol(),
            " ",
            "the heading row must occupy column 1"
        );
        // Row 3 is the selected `alpha` row (row 2 is the active section header,
        // unselected and therefore blank at a one-column field): its marker alone
        // still draws into column 1.
        assert_ne!(
            cell(&buf3, 1, 3).symbol(),
            " ",
            "the selected list row must occupy column 1"
        );

        // None of the three holds a divider: all three are below the breakpoint.
        for buf in [&buf1, &buf2, &buf3] {
            assert!(!buffer_contains(buf, "│"));
        }

        // The two mandated widths as contrasting controls: comfortably wide enough
        // that the heading and the selected row draw whole, so the degenerate cases
        // above are a width branch rather than something wrong at every size.
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(buffer_contains(&buf, "demo-repo"));
            assert_eq!(cell(&buf, 1, 3).symbol(), ">", "width {width}");
        }
    }

    /// `responsive-layout` :: "A region draws a heading, a blank row, and no border at both
    /// widths" — the padding row between a region's heading and its interior is never
    /// painted, and the interior's first row is the buffer's row 2 at both mandated widths.
    /// Twenty changes so the interior's own **last** row (18) holds real content rather than
    /// a blank cell, which is what distinguishes "no border" from "a border that happens to
    /// be blank".
    #[test]
    fn a_region_draws_a_heading_a_blank_row_and_no_border_at_both_widths() {
        const BORDERS: [&str; 4] = ["┌", "┐", "└", "┘"];
        for width in [120u16, 60] {
            let d = dashboard_with(changes_named(20), Vec::new(), 1, Route::List);
            let buf = render_at(width, 20, &d);
            let last = if width == 60 { 58u16 } else { 38 };

            // The padding row (buffer row 1) is untouched by construction: every cell of it,
            // inside the interior's own columns, is a space at the default style.
            for x in 1..=last {
                let c = cell(&buf, x, 1);
                assert_eq!(c.symbol(), " ", "width {width}: padding row column {x}");
                assert_eq!(
                    c.style(),
                    uncoloured(),
                    "width {width}: padding row column {x} carries a style"
                );
            }

            // The interior's first row is the buffer's row 2 — the active section header —
            // at both mandated widths.
            assert!(
                interior_cols(&buf, 2).contains("active"),
                "width {width}: row 2 must be the interior's first row"
            );

            // The list region's left gutter (column 0) and right gutter (column 39 at
            // 120, the frame's own last column at 60, where the single region takes
            // `Gutters::Both`) stay empty on every row of the body. At 120 the *frame's*
            // last column belongs to the detail region, which has no right gutter of its
            // own (D5) — that column is content, not a gutter, so it is not asserted here.
            let list_right_gutter = if width == 60 { width - 1 } else { 39 };
            for y in 0..=18u16 {
                assert_eq!(cell(&buf, 0, y).symbol(), " ", "width {width}: y={y}");
                assert_eq!(
                    cell(&buf, list_right_gutter, y).symbol(),
                    " ",
                    "width {width}: y={y}"
                );
            }

            // No box-drawing character survives — there is no `Block` any more.
            assert!(
                !BORDERS.iter().any(|b| buffer_contains(&buf, b)),
                "width {width}: no border character may appear"
            );

            // The interior's last row (18) holds real content, not a blank cell — twenty
            // changes fill all seventeen interior rows at both widths.
            assert!(
                !interior_cols(&buf, 18).trim().is_empty(),
                "width {width}: row 18 must hold a drawn list row"
            );
        }
    }

    /// `responsive-layout` :: "The divider has a blank column on each side at 120 columns"
    /// — named per design.md's Verification matrix, which titles the scenario `120`-only;
    /// `scripts/gates/widths.sh` requires every `#[test]` in this file to name both mandated
    /// widths, so the `60`-column half (no divider at all below the breakpoint) is folded
    /// into the same test rather than left to a second one.
    #[test]
    fn the_divider_has_a_blank_column_on_each_side_at_120_columns() {
        let d = dashboard_with(
            vec![fixture::active("alpha", 1, 3)],
            Vec::new(),
            1,
            Route::List,
        );

        let buf120 = render_at(120, 20, &d);
        for y in 0..=18u16 {
            assert_eq!(cell(&buf120, 40, y).symbol(), "│", "y={y}");
            assert!(
                cell(&buf120, 40, y)
                    .style()
                    .add_modifier
                    .contains(Modifier::DIM),
                "y={y}: the divider must be dim"
            );
            assert_eq!(
                cell(&buf120, 40, y).style().fg,
                uncoloured().fg,
                "y={y}: the divider must carry no colour"
            );
            assert_eq!(cell(&buf120, 39, y).symbol(), " ", "y={y}");
            assert_eq!(cell(&buf120, 41, y).symbol(), " ", "y={y}");
            assert_eq!(cell(&buf120, 0, y).symbol(), " ", "y={y}");
        }
        // The footer row holds no divider: it is confined to the body.
        assert!(!row_text(&buf120, 19).contains('│'));
        // The detail region has no right gutter: content reaches the frame's own last
        // column rather than stopping one short of it. The change header draws into the
        // region's own heading row, buffer row 0, on the same terms every other region's
        // heading does — drawn at both routes at the wide layout (`detail-header` -> "The
        // header is drawn whenever the detail region is drawn").
        assert_ne!(
            cell(&buf120, 119, 0).symbol(),
            " ",
            "the detail region's content must reach the frame's last column"
        );

        // Below the breakpoint there is no divider column at all.
        let buf60 = render_at(60, 20, &d);
        assert!(!buffer_contains(&buf60, "│"));
    }

    fn three_active() -> Dashboard {
        dashboard_with(
            vec![
                fixture::active("add-token-refresh", 4, 9),
                fixture::active("fix-empty-basket", 7, 7),
                fixture::active("migrate-ai-sdk-v7", 0, 0),
            ],
            Vec::new(),
            // `list-sections`: `selected` **1** addresses `Target::Change(0)` —
            // `add-token-refresh`, the first change — since the active section
            // header is target 0. Kept marking the same change this fixture always
            // marked.
            1,
            Route::List,
        )
    }

    #[test]
    fn list_rows_render_at_60_and_120() {
        let d = three_active();

        // `list-sections`: buffer row 2 is now the active section header; the three
        // change rows follow it at rows 3-5.
        let buf120 = render_at(120, 20, &d);
        assert_eq!(
            interior_cols(&buf120, 2),
            format!("{:<38}", "  ▾ active (3)")
        );
        assert_eq!(
            interior_cols(&buf120, 3),
            "> add-token-refresh              [4/9]"
        );
        assert_eq!(
            interior_cols(&buf120, 4),
            "  fix-empty-basket               [7/7]"
        );
        assert_eq!(
            interior_cols(&buf120, 5),
            "  migrate-ai-sdk-v7                [-]"
        );
        for y in 6..=17u16 {
            assert!(cols(&row_text(&buf120, y), 1..39).chars().all(|c| c == ' '));
        }

        let buf60 = render_at(60, 20, &d);
        assert_eq!(
            interior_cols(&buf60, 2),
            format!("{:<58}", "  ▾ active (3)")
        );
        assert_eq!(
            interior_cols(&buf60, 3),
            "> add-token-refresh                                  [4/9]"
        );
        assert_eq!(
            interior_cols(&buf60, 4),
            "  fix-empty-basket                                   [7/7]"
        );
        assert_eq!(
            interior_cols(&buf60, 5),
            "  migrate-ai-sdk-v7                                    [-]"
        );
        for y in 6..=17u16 {
            assert!(cols(&row_text(&buf60, y), 1..59).chars().all(|c| c == ' '));
        }
    }

    #[test]
    fn a_long_name_is_truncated_at_38_but_not_at_58() {
        let d = dashboard_with(
            vec![fixture::active(
                "a-very-long-change-name-that-will-not-fit-here",
                2,
                5,
            )],
            Vec::new(),
            0,
            Route::List,
        );
        // `list-sections`: buffer row 2 is the active section header; the change
        // row is row 3.
        let buf120 = render_at(120, 20, &d);
        let row3 = interior_cols(&buf120, 3);
        assert!(row3.contains('…'));
        assert!(row3.contains("[2/5]"));

        let buf60 = render_at(60, 20, &d);
        let row3_60 = interior_cols(&buf60, 3);
        assert!(row3_60.contains("a-very-long-change-name-that-will-not-fit-here"));
        assert!(!buffer_contains(&buf60, "…"));
    }

    /// `change-rows` :: "A CJK change name stays inside the list region at both mandated
    /// widths" — driven through a full `ui::view::render` rather than through
    /// `ui::list::rows` directly, since the whole point is that the render path (not just
    /// the row-text primitive) never lets the name cross into the neighbouring region.
    /// `list.rs`'s own test already carries this exact fixture at its own tier; this one is
    /// the view-tier instance the manifest asks for. `render_list` draws each row with a
    /// single `set_string` call rather than the per-segment loop this group's fix touches,
    /// so this scenario is not expected to discriminate the segment-loop bug specifically —
    /// see this group's own report for that observation.
    #[test]
    fn view_render_keeps_a_cjk_change_name_inside_the_list_region() {
        let name = "日本語の変更名前です";
        assert_eq!(columns(name), 20);
        let cjk = dashboard_with(
            vec![fixture::active(name, 4, 9)],
            Vec::new(),
            0,
            Route::List,
        );
        let ascii = dashboard_with(
            vec![fixture::active("add-token-refresh", 4, 9)],
            Vec::new(),
            0,
            Route::List,
        );

        let buf120 = render_at(120, 20, &cjk);
        let control120 = render_at(120, 20, &ascii);
        let buf60 = render_at(60, 20, &cjk);
        let control60 = render_at(60, 20, &ascii);
        // `list-sections`: buffer row 2 is now the active section header; the
        // change row is buffer row 3.
        for (width, border_x, buf, control) in [
            (120u16, 39u16, &buf120, &control120),
            (60u16, 59u16, &buf60, &control60),
        ] {
            assert_eq!(
                cell(buf, border_x, 3).symbol(),
                " ",
                "width {width}: the region's right gutter must stay a blank space"
            );
            assert_eq!(
                cell(buf, border_x, 3).symbol(),
                cell(control, border_x, 3).symbol(),
                "width {width}: the gutter must be unmoved from the ASCII-named control"
            );
            assert_eq!(
                cell(buf, border_x - 1, 3).symbol(),
                "]",
                "width {width}: the progress cell ends in the interior's last column"
            );
        }
    }

    /// `change-rows` -> "The section header and archived rows render at both
    /// mandated widths": one active change, two archived (one dated, one not),
    /// both sections open, `selected` on the active change — target 0 is the
    /// active section header. `list-sections`' rewrite of the landed
    /// `separator_and_archived_rows_render_at_both_widths`.
    #[test]
    fn the_section_header_and_archived_rows_render_at_both_mandated_widths() {
        let d = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            vec![
                fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                fixture::archived(None, "legacy-cleanup", 3, 3),
            ],
            1,
            Route::List,
        );
        let buf120 = render_at(120, 20, &d);
        assert_eq!(
            interior_cols(&buf120, 2),
            format!("{:<38}", "  ▾ active (1)")
        );
        assert_eq!(
            interior_cols(&buf120, 3),
            "> fix-empty-basket               [7/7]"
        );
        assert_eq!(
            interior_cols(&buf120, 4),
            format!("{:<38}", "  ▾ archived (2)")
        );
        assert_eq!(
            interior_cols(&buf120, 5),
            "  2026-08-14 add-auth            [7/7]"
        );
        assert_eq!(
            interior_cols(&buf120, 6),
            "             legacy-cleanup      [3/3]"
        );

        let buf60 = render_at(60, 20, &d);
        assert_eq!(
            interior_cols(&buf60, 2),
            format!("{:<58}", "  ▾ active (1)")
        );
        assert_eq!(
            interior_cols(&buf60, 3),
            "> fix-empty-basket                                   [7/7]"
        );
        assert_eq!(
            interior_cols(&buf60, 4),
            format!("{:<58}", "  ▾ archived (2)")
        );
        assert_eq!(
            interior_cols(&buf60, 5),
            "  2026-08-14 add-auth                                [7/7]"
        );
        assert_eq!(
            interior_cols(&buf60, 6),
            "             legacy-cleanup                          [3/3]"
        );

        assert!(!buffer_contains(&buf120, "-- archived"));
        assert!(!buffer_contains(&buf60, "-- archived"));
    }

    /// `change-rows` -> "A collapsed archived section shows its count and no
    /// rows".
    #[test]
    fn a_collapsed_archived_section_shows_its_count_and_no_rows() {
        let mut d = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            vec![
                fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                fixture::archived(None, "legacy-cleanup", 3, 3),
            ],
            1,
            Route::List,
        );
        d.sections.collapsed.insert(SectionKey::Archived);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(!buffer_contains(&buf, "add-auth"));
            assert!(!buffer_contains(&buf, "legacy-cleanup"));
        }
        let buf120 = render_at(120, 20, &d);
        assert_eq!(
            interior_cols(&buf120, 2),
            format!("{:<38}", "  ▾ active (1)")
        );
        assert_eq!(
            interior_cols(&buf120, 3),
            "> fix-empty-basket               [7/7]"
        );
        assert_eq!(
            interior_cols(&buf120, 4),
            format!("{:<38}", "  ▸ archived (2)")
        );

        // An unresolved tier's header counts from `archived_total`, not from the
        // rows it holds.
        let mut unresolved = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            Vec::new(),
            1,
            Route::List,
        );
        unresolved.changes.archived_total = 22;
        unresolved.sections.collapsed.insert(SectionKey::Archived);
        let buf = render_at(120, 20, &unresolved);
        assert_eq!(
            interior_cols(&buf, 4),
            format!("{:<38}", "  ▸ archived (22)")
        );
    }

    /// `change-rows` -> "An expanded but unresolved archived section shows its
    /// header alone": the archive is open but nothing has resolved it yet — one
    /// active change so no empty-state message intervenes.
    #[test]
    fn an_expanded_but_unresolved_archived_section_shows_its_header_alone() {
        let mut d = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            Vec::new(),
            1,
            Route::List,
        );
        d.changes.archived_total = 22;
        // The archived section is open by default (`sections.collapsed` starts
        // empty).
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(!buffer_contains(&buf, "No changes match"));
            assert!(!buffer_contains(&buf, "No active changes"));
            assert!(!buffer_contains(&buf, "!"));
        }
        let buf120 = render_at(120, 20, &d);
        assert_eq!(
            interior_cols(&buf120, 2),
            format!("{:<38}", "  ▾ active (1)")
        );
        assert_eq!(
            interior_cols(&buf120, 3),
            "> fix-empty-basket               [7/7]"
        );
        assert_eq!(
            interior_cols(&buf120, 4),
            format!("{:<38}", "  ▾ archived (22)")
        );
        for y in 5..=17u16 {
            assert!(cols(&row_text(&buf120, y), 1..39).chars().all(|c| c == ' '));
        }

        // The same dashboard with the twenty-two archived changes present renders
        // the header identically and rows below it.
        let mut resolved = d.clone();
        resolved.changes.archived = (0..22)
            .map(|i| fixture::archived(None, &format!("archived-{i:02}"), 1, 1))
            .collect();
        let buf_resolved = render_at(120, 20, &resolved);
        assert_eq!(
            interior_cols(&buf_resolved, 4),
            format!("{:<38}", "  ▾ archived (22)")
        );
        assert!(buffer_contains(&buf_resolved, "archived-00"));
    }

    /// `change-rows` -> "An archived change carries a badge in the same column
    /// as an active one".
    #[test]
    fn an_archived_change_carries_a_badge_in_the_same_column_as_an_active_one() {
        let mut d = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            vec![
                fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                fixture::archived(None, "legacy-cleanup", 3, 3),
            ],
            1,
            Route::List,
        );
        d.agents.agents = vec![unattributed_agent("add-auth")];
        d.agents.agents[0].status = crate::agents::AgentStatus::Blocked;

        let buf120 = render_at(120, 20, &d);
        assert_eq!(
            interior_cols(&buf120, 5),
            "  2026-08-14 add-auth          b [7/7]"
        );
        assert_eq!(
            interior_cols(&buf120, 6),
            "             legacy-cleanup      [3/3]"
        );
        assert_eq!(
            interior_cols(&buf120, 3),
            "> fix-empty-basket               [7/7]"
        );

        let buf60 = render_at(60, 20, &d);
        let row = interior_cols(&buf60, 5);
        assert_eq!(columns(&row), 58);
        assert_eq!(row.chars().nth(51), Some('b'));
        assert_eq!(row.chars().nth(50), Some(' '));
        assert_eq!(row.chars().nth(52), Some(' '));

        // Both section-header rows are byte-identical at both widths to the
        // agentless rendering, so no badge column was reserved on a row that
        // cannot carry one.
        let agentless = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            vec![
                fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                fixture::archived(None, "legacy-cleanup", 3, 3),
            ],
            1,
            Route::List,
        );
        let agentless_buf = render_at(120, 20, &agentless);
        assert_eq!(interior_cols(&buf120, 2), interior_cols(&agentless_buf, 2));
        assert_eq!(interior_cols(&buf120, 4), interior_cols(&agentless_buf, 4));
    }

    /// `change-rows` -> "A query against an unresolved archive counts from
    /// `archived_total`": the archived tier is unresolved, and a query matching
    /// nothing in the active tier is accepted before the refresh it requests has
    /// answered.
    #[test]
    fn a_query_against_an_unresolved_archive_counts_from_archived_total() {
        let mut d = dashboard_with(
            vec![
                fixture::active("fix-empty-basket", 7, 7),
                fixture::active("migrate-ai-sdk-v7", 0, 0),
            ],
            Vec::new(),
            usize::MAX,
            Route::List,
        );
        d.changes.archived_total = 28;
        d.sections.collapsed.insert(SectionKey::Archived);
        d.filter.query = "zzz".to_string();
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(interior_cols(&buf, 2).starts_with("No changes match"));
            assert!(interior_cols(&buf, 3).starts_with("/zzz"));
            assert_eq!(
                interior_cols(&buf, 4),
                format!(
                    "{:<w$}",
                    "  ▾ archived (28)",
                    w = if width == 60 { 58 } else { 38 }
                )
            );
            assert!(!buffer_contains(&buf, "! "));
            for y in 5..=17u16 {
                let last = if width == 60 { 58 } else { 38 };
                assert!(
                    cols(&row_text(&buf, y), 1..last + 1)
                        .chars()
                        .all(|c| c == ' ')
                );
            }
        }

        // Once the twenty-eight archived changes have arrived, none of which
        // matches `zzz`, no archived header is emitted at all: that section's
        // count is then zero, so the `(28)` above was the one-cycle unresolved
        // window rather than a lasting count.
        let mut resolved = d.clone();
        resolved.changes.archived = (0..28)
            .map(|i| fixture::archived(None, &format!("old-change-{i:02}"), 1, 1))
            .collect();
        let buf = render_at(120, 20, &resolved);
        assert!(!buffer_contains(&buf, "archived"));
    }

    /// `change-rows` -> "No active changes with archived ones still browsable".
    /// `list-sections`' rewrite of the landed `no_active_changes_keeps_archived_browsable`.
    #[test]
    fn no_active_changes_with_archived_ones_still_browsable() {
        let d = dashboard_with(
            Vec::new(),
            vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
            usize::MAX,
            Route::List,
        );
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(interior_cols(&buf, 2).starts_with("No active changes"));
            assert_eq!(
                interior_cols(&buf, 3),
                format!(
                    "{:<w$}",
                    "  ▾ archived (1)",
                    w = if width == 60 { 58 } else { 38 }
                )
            );
            assert!(interior_cols(&buf, 4).contains("add-auth"));
            assert!(!buffer_contains(&buf, "No changes yet"));
            assert!(!buffer_contains(&buf, "-- archived"));
        }
    }

    /// `change-rows` -> "A collapsed but non-empty archive is not \"no changes
    /// yet\"".
    #[test]
    fn a_collapsed_but_non_empty_archive_is_not_no_changes_yet() {
        let mut d = dashboard_with(Vec::new(), Vec::new(), usize::MAX, Route::List);
        d.changes.archived_total = 28;
        d.sections.collapsed.insert(SectionKey::Archived);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(interior_cols(&buf, 2).starts_with("No active changes"));
            assert_eq!(
                interior_cols(&buf, 3),
                format!(
                    "{:<w$}",
                    "  ▸ archived (28)",
                    w = if width == 60 { 58 } else { 38 }
                )
            );
            assert!(!buffer_contains(&buf, "No changes yet"));
            assert!(!buffer_contains(&buf, "No changes match"));
        }
    }

    /// `change-rows` -> "A collapsed active section shows its header and no
    /// message row".
    #[test]
    fn a_collapsed_active_section_shows_its_header_and_no_message_row() {
        let active: Vec<Change> = (0..9)
            .map(|i| fixture::active(&format!("change-{i:02}"), 1, 2))
            .collect();
        let mut d = dashboard_with(active, Vec::new(), usize::MAX, Route::List);
        d.sections.collapsed.insert(SectionKey::Active);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert_eq!(
                interior_cols(&buf, 2),
                format!(
                    "{:<w$}",
                    "  ▸ active (9)",
                    w = if width == 60 { 58 } else { 38 }
                )
            );
            assert!(!buffer_contains(&buf, "No active changes"));
            assert!(!buffer_contains(&buf, "No changes yet"));
            assert!(!buffer_contains(&buf, "change-00"));
        }
        // Expanding the same section renders the nine rows below it.
        let mut opened = d.clone();
        opened.sections.collapsed.clear();
        let buf = render_at(120, 20, &opened);
        assert!(buffer_contains(&buf, "change-00"));
    }

    #[test]
    fn no_archived_changes_means_no_archived_header() {
        let d = three_active();
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(!buffer_contains(&buf, "-- archived"));
            assert!(!buffer_contains(&buf, "archived ("));
        }
        let with_archived = dashboard_with(
            vec![
                fixture::active("add-token-refresh", 4, 9),
                fixture::active("fix-empty-basket", 7, 7),
                fixture::active("migrate-ai-sdk-v7", 0, 0),
            ],
            vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
            1,
            Route::List,
        );
        for width in [60, 120] {
            let buf = render_at(width, 20, &with_archived);
            assert!(buffer_contains(&buf, "archived (1)"));
        }
    }

    #[test]
    fn no_repository_names_the_directory_searched() {
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
            sections: crate::ui::app::Sections {
                collapsed: std::collections::BTreeSet::new(),
            },
            file_mode: false,
        };
        let buf120 = render_at(120, 20, &d);
        assert!(interior_cols(&buf120, 2).starts_with("No OpenSpec repository found"));
        assert!(interior_cols(&buf120, 3).starts_with("searched from:"));
        assert!(interior_cols(&buf120, 4).starts_with("…os/a-rather-long-repository-name-here"));
        // `pane-chrome`: there is no border and no `Changes` title any more — the list
        // region's own heading row (row 0) names `no repository` instead, and row 1 is the
        // blank padding row above the no-repository block asserted above.
        assert_eq!(cols(&row_text(&buf120, 0), 1..14), "no repository");
        assert_eq!(interior_cols(&buf120, 1).trim_end(), "");

        let buf60 = render_at(60, 20, &d);
        assert!(interior_cols(&buf60, 2).starts_with("No OpenSpec repository found"));
        assert!(interior_cols(&buf60, 3).starts_with("searched from:"));
        assert!(
            interior_cols(&buf60, 4)
                .starts_with("…kspaces/openspec-demos/a-rather-long-repository-name-here")
        );
        assert_eq!(cols(&row_text(&buf60, 0), 1..14), "no repository");
    }

    #[test]
    fn a_repository_with_no_changes_says_so() {
        let d = dashboard_with(Vec::new(), Vec::new(), 0, Route::List);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(interior_cols(&buf, 2).starts_with("No changes yet"));
            assert!(!buffer_contains(&buf, "-- archived"));
            assert!(!buffer_contains(&buf, "No active changes"));
            assert!(!buffer_contains(&buf, "No changes match"));
        }
    }

    #[test]
    fn repository_problems_are_named_above_the_rows() {
        let mut d = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            Vec::new(),
            0,
            Route::List,
        );
        d.changes.problems = vec!["openspec/changes: Permission denied (os error 13)".to_string()];

        // `list-sections`: row 3 is now the active section header; the change
        // row is row 4.
        let buf120 = render_at(120, 20, &d);
        assert_eq!(
            interior_cols(&buf120, 2),
            "! openspec/changes: Permission denied…"
        );
        assert!(interior_cols(&buf120, 4).contains("fix-empty-basket"));

        let buf60 = render_at(60, 20, &d);
        assert!(
            interior_cols(&buf60, 2)
                .starts_with("! openspec/changes: Permission denied (os error 13)")
        );
        assert!(!interior_cols(&buf60, 2).contains('…'));
        assert!(interior_cols(&buf60, 4).contains("fix-empty-basket"));
    }

    // `live-refresh` -> "The list region's leading rows name refresh
    // problems first" and the view half of the dual-source claim. See
    // `specs/live-updates/spec.md`.

    #[test]
    fn a_watch_problem_is_the_first_row() {
        let mut d = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            Vec::new(),
            0,
            Route::List,
        );
        d.refresh.problems =
            vec!["filesystem watch unavailable for /r/openspec: No path was found".to_string()];

        for (width, height) in [(120, 20), (60, 20)] {
            let buf = render_at(width, height, &d);
            assert!(
                interior_cols(&buf, 2).starts_with("! filesystem watch unavailable"),
                "width {width}: {}",
                interior_cols(&buf, 2)
            );
            // `list-sections`: row 3 is now the active section header.
            assert!(
                interior_cols(&buf, 4).contains("fix-empty-basket"),
                "width {width}"
            );
        }
    }

    #[test]
    fn refresh_problems_precede_change_problems() {
        let mut d = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            Vec::new(),
            0,
            Route::List,
        );
        d.refresh.problems = vec!["watch failed".to_string()];
        d.changes.problems = vec!["openspec/changes unreadable".to_string()];

        for (width, height) in [(120, 20), (60, 20)] {
            let buf = render_at(width, height, &d);
            assert!(
                interior_cols(&buf, 2).starts_with("! watch failed"),
                "width {width}: the refresh problem must lead"
            );
            assert!(
                interior_cols(&buf, 3).starts_with("! openspec/changes unreadable"),
                "width {width}"
            );
            // `list-sections`: row 4 is now the active section header.
            assert!(
                interior_cols(&buf, 5).contains("fix-empty-basket"),
                "width {width}"
            );
        }
    }

    #[test]
    fn no_refresh_problem_draws_no_extra_row() {
        // `refresh.problems` is empty by construction — the closest a typed
        // `Dashboard` value can come to "the field absent", since every
        // value must name all nine fields. Compared against a second,
        // independently constructed dashboard (not a `.clone()`) so the
        // equality is not tautological.
        let d = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            Vec::new(),
            0,
            Route::List,
        );
        let same = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            Vec::new(),
            0,
            Route::List,
        );

        for (width, height) in [(120, 20), (60, 20)] {
            let buf = render_at(width, height, &d);
            let buf_same = render_at(width, height, &same);
            assert_eq!(buf, buf_same, "width {width}");
            // `list-sections`: row 2 is the active section header, not a problem
            // row — the change row follows it at row 3.
            assert!(
                !interior_cols(&buf, 2).starts_with('!'),
                "width {width}: no leading problem row"
            );
            assert!(
                interior_cols(&buf, 3).contains("fix-empty-basket"),
                "width {width}: the change row must be first, no leading problem row"
            );
        }
    }

    #[test]
    fn the_corrected_progress_reaches_the_buffer() {
        // The view has no notion of "corrected" — it draws whatever
        // `Change::progress` holds. This is the render-only half of the
        // dual-source claim; `ui::driver::tests::a_result_is_adopted_before_the_frame`
        // and `ui::tests::live::files_paint_then_the_cli_corrects` are the
        // halves that prove the loop feeds it the CLI's corrected number.
        let d = dashboard_with(
            vec![fixture::active("alpha", 7, 9)],
            Vec::new(),
            0,
            Route::List,
        );
        for (width, height) in [(120, 20), (60, 20)] {
            let buf = render_at(width, height, &d);
            // `list-sections`: row 2 is the active section header.
            assert!(interior_cols(&buf, 3).contains("[7/9]"), "width {width}");
        }
    }

    #[test]
    fn a_removed_repo_shows_the_problem_row() {
        // `watch-invalidation`: "openspec/ is removed while the watcher
        // runs" — the view's half of the claim.
        // `ui::driver::tests::a_watch_error_is_recorded_once` proves the
        // drain error is recorded and the loop keeps drawing; this proves
        // what it draws.
        let mut d = dashboard_with(
            vec![fixture::active("alpha", 4, 9)],
            Vec::new(),
            0,
            Route::List,
        );
        d.refresh.problems =
            vec!["the filesystem watcher's event channel disconnected".to_string()];

        for (width, height) in [(120, 20), (60, 20)] {
            let buf = render_at(width, height, &d);
            assert!(
                interior_cols(&buf, 2).starts_with("! the filesystem watcher"),
                "width {width}: {}",
                interior_cols(&buf, 2)
            );
            // `list-sections`: row 3 is now the active section header.
            assert!(
                interior_cols(&buf, 4).contains("alpha"),
                "width {width}: the list still draws despite the watcher failure"
            );
        }
    }

    #[test]
    fn the_detail_region_stays_blank_while_the_list_fills() {
        // Kept verbatim per the list-view era; its assertion changes with
        // detail-view: the wide layout draws the detail region at
        // Route::List too, and a change **is** selected here, so the
        // region is no longer blank — it shows that change's header and
        // tab bar. The blankness this test's name promises is now a
        // property of an *empty visible list*, asserted as the
        // discriminating companion below.
        // `list-sections`: target 0 is now the active header, which carries no
        // selected change, so this needs a dashboard selecting a real change
        // rather than `three_active()`'s header-addressing default.
        let d = dashboard_with(
            vec![
                fixture::active("add-token-refresh", 4, 9),
                fixture::active("fix-empty-basket", 7, 7),
                fixture::active("migrate-ai-sdk-v7", 0, 0),
            ],
            Vec::new(),
            1,
            Route::List,
        );
        let buf120 = render_at(120, 20, &d);
        assert!(detail_interior_cols(&buf120, 0, 78).contains("add-token-refresh"));

        let empty = dashboard_with(Vec::new(), Vec::new(), 0, Route::List);
        let default_style = Cell::default().style();
        let buf_empty = render_at(120, 20, &empty);
        // `pane-chrome`: the interior grew to seventeen rows (2 through 18, not 2
        // through 17) and the wide detail region's gutter-free interior now reaches
        // column 119, not 118.
        for y in 2..=18u16 {
            for x in 41..=119u16 {
                let c = cell(&buf_empty, x, y);
                assert_eq!(c.symbol(), " ", "x={x} y={y}");
                assert_eq!(c.style(), default_style, "x={x} y={y}");
            }
        }

        // `pane-chrome`: there is no border any more — columns 0 and 59 are simply
        // the narrow layout's own gutters, spaces on every row of the body.
        let buf60 = render_at(60, 20, &d);
        for y in 0..=18u16 {
            assert_eq!(cell(&buf60, 0, y).symbol(), " ", "y={y}");
            assert_eq!(cell(&buf60, 59, y).symbol(), " ", "y={y}");
        }
    }

    #[test]
    fn the_narrow_detail_route_draws_no_rows() {
        // The narrow detail route draws no LIST rows: the selected
        // change's name is legitimately visible via the detail header
        // (detail-view), but the other two — which only a list row could
        // have named — are not.
        let d = three_active_at_route(Route::Detail);
        let buf60 = render_at(60, 20, &d);
        for name in ["fix-empty-basket", "migrate-ai-sdk-v7"] {
            assert!(!buffer_contains(&buf60, name));
        }
        assert!(buffer_contains(&buf60, "add-token-refresh"));
        let buf120 = render_at(120, 20, &d);
        for name in ["add-token-refresh", "fix-empty-basket", "migrate-ai-sdk-v7"] {
            assert!(buffer_contains(&buf120, name));
        }
    }

    fn three_active_at_route(route: Route) -> Dashboard {
        dashboard_with(
            vec![
                fixture::active("add-token-refresh", 4, 9),
                fixture::active("fix-empty-basket", 7, 7),
                fixture::active("migrate-ai-sdk-v7", 0, 0),
            ],
            Vec::new(),
            // `list-sections`: 1, not 0 — target 0 is the active header, and
            // this fixture's one caller needs a real selected change for the
            // detail header it asserts on.
            1,
            route,
        )
    }

    #[test]
    fn more_changes_than_rows_do_not_overflow() {
        let d = dashboard_with(changes_named(30), Vec::new(), 0, Route::List);
        // `list-sections`: row 2 is the active section header, so the interior's
        // remaining sixteen rows (3 through 18 — `pane-chrome` grew the interior
        // by one row, moving only its last index) hold change-00 through
        // change-15.
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(interior_cols(&buf, 3).contains("change-00"));
            assert!(interior_cols(&buf, 18).contains("change-15"));
            for y in [0u16, 1, 19] {
                assert!(!row_text(&buf, y).contains("change-"));
            }
        }
    }

    #[test]
    fn the_selected_row_carries_the_marker_and_bold() {
        // `list-sections`: row 2 is the active section header (unselected here);
        // the marked change row is row 3.
        let d = three_active();
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            let last = if width == 60 { 58 } else { 38 };
            assert_eq!(cell(&buf, 1, 3).symbol(), ">");
            for x in 1..=last {
                assert!(is_bold(cell(&buf, x, 3)), "x={x} width={width}");
            }
            assert!(!is_bold(cell(&buf, 1, 4)));
        }
    }

    #[test]
    fn navigation_moves_the_marker() {
        // `list-sections`: `three_active()` starts on `add-token-refresh` (target
        // 1); two `Next` presses land on `migrate-ai-sdk-v7` (target 3), the
        // interior's row 5 — row 2 is the active header, rows 3-5 the three
        // changes.
        let mut d = three_active();
        d.apply(Action::Next);
        d.apply(Action::Next);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert_eq!(cell(&buf, 1, 5).symbol(), ">");
            assert!(is_bold(cell(&buf, 1, 5)));
        }
    }

    #[test]
    fn selection_clamps_at_both_ends_on_screen() {
        let mut d = three_active();
        for _ in 0..4 {
            d.apply(Action::Next);
        }
        // `list-sections`: the clamp is against `targets().len()` (the active
        // header plus three changes, 4), so the cursor stops on the third
        // active change's *target* — `selected` 3 — one past `three_active()`'s
        // starting `selected` 1.
        assert_eq!(d.selected, 3);
        assert_eq!(d.selected_change().unwrap().name, "migrate-ai-sdk-v7");
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            // Row 2 is the active header; the marker is now correctly on row 5,
            // `migrate-ai-sdk-v7`'s row, and nowhere else.
            assert_eq!(cell(&buf, 1, 5).symbol(), ">", "width {width}");
            assert!(is_bold(cell(&buf, 1, 5)), "width {width}");
            for y in [2u16, 3, 4] {
                assert_ne!(cell(&buf, 1, y).symbol(), ">", "width {width} y={y}");
            }
        }
    }

    #[test]
    fn selection_crosses_the_separator() {
        // `list-sections`: `targets()` is `[Section(Active), Change(0)=fix-
        // empty-basket, Section(Archived), Change(1)=add-auth, Change(2)=
        // legacy-cleanup]`. Starting on `fix-empty-basket` (target 1), three
        // `Next` presses cross the archived header (target 2, itself a stop
        // now that it is selectable) and land on `legacy-cleanup` (target 4).
        let mut d = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            vec![
                fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                fixture::archived(None, "legacy-cleanup", 3, 3),
            ],
            1,
            Route::List,
        );
        d.apply(Action::Next);
        d.apply(Action::Next);
        d.apply(Action::Next);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            // Rows: 2 active header, 3 fix-empty-basket, 4 archived header, 5
            // add-auth, 6 legacy-cleanup.
            assert!(interior_cols(&buf, 6).contains("legacy-cleanup"));
            assert_eq!(cell(&buf, 1, 6).symbol(), ">", "width {width}");
            assert!(is_bold(cell(&buf, 1, 6)), "width {width}");
            // The archived header the cursor passed through carries no marker.
            assert_ne!(cell(&buf, 1, 4).symbol(), ">", "width {width}");
            assert!(!is_bold(cell(&buf, 1, 4)), "width {width}");
        }
    }

    #[test]
    fn an_empty_visible_list_draws_no_marker() {
        let d = dashboard_with(Vec::new(), Vec::new(), 0, Route::List);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            let last = if width == 60 { 58 } else { 38 };
            // "No changes yet" carries no marker column of its own — the
            // point of this scenario is that no `>` appears, not that the
            // message row's own leading character is a space.
            assert_ne!(cell(&buf, 1, 2).symbol(), ">");
            for y in 2..=17u16 {
                for x in 1..=last {
                    assert!(!is_bold(cell(&buf, x, y)), "x={x} y={y}");
                }
            }
        }
    }

    #[test]
    fn a_selection_past_the_interior_scrolls_the_slice() {
        // `list-sections`: `selected` **21** addresses `Target::Change(20)` since
        // the active section header is target 0; the resulting buffer rows are
        // unchanged, since the header shifts `rows.len()` and the cursor position
        // by the same one row.
        let d = dashboard_with(changes_named(30), Vec::new(), 21, Route::List);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(interior_cols(&buf, 2).contains("change-12"));
            assert!(interior_cols(&buf, 17).contains("change-27"));
            let y20 = 2 + (20 - 12);
            assert!(interior_cols(&buf, y20).contains("change-20"));
            assert_eq!(cell(&buf, 1, y20).symbol(), ">");
            assert!(is_bold(cell(&buf, 1, y20)));
        }
    }

    #[test]
    fn the_last_change_is_reachable() {
        // `list-sections`: `selected` **30** addresses `Target::Change(29)`.
        // `pane-chrome`: the interior grew from sixteen rows to seventeen
        // (2 through 18, not 2 through 17), which shifts `layout::viewport`'s
        // own offset by one item earlier — the first visible row is now
        // `change-13`, not `change-14`.
        let d = dashboard_with(changes_named(30), Vec::new(), 30, Route::List);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(interior_cols(&buf, 2).contains("change-13"));
            assert!(interior_cols(&buf, 18).contains("change-29"));
            assert_eq!(cell(&buf, 1, 18).symbol(), ">");
            for y in 2..=18u16 {
                assert!(!interior_cols(&buf, y).chars().all(|c| c == ' '));
            }
        }
    }

    #[test]
    fn the_viewport_boundary_is_rendered() {
        // `list-sections`: `selected` **10** addresses `Target::Change(9)`.
        // `pane-chrome`'s taller interior (seventeen rows, not sixteen) changes
        // `layout::viewport`'s own offset for this cursor and row count: 1, not 2.
        let d = dashboard_with(changes_named(17), Vec::new(), 10, Route::List);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(interior_cols(&buf, 2).contains("change-00"));
        }
    }

    #[test]
    fn resizing_changes_the_slice_on_the_next_frame() {
        // `list-sections`: `selected` **21** addresses `Target::Change(20)`.
        let d = dashboard_with(changes_named(30), Vec::new(), 21, Route::List);
        let backend = TestBackend::new(120, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        terminal
            .draw(|frame| super::render(frame, &d))
            .expect("draw first frame");
        let first = terminal.backend().buffer().clone();
        assert!(interior_cols(&first, 2).contains("change-12"));

        terminal.backend_mut().resize(120, 12);
        terminal
            .draw(|frame| super::render(frame, &d))
            .expect("draw second frame");
        let second = terminal.backend().buffer().clone();
        assert!(interior_cols(&second, 2).contains("change-16"));

        // The 60-column control, naming both mandated widths in this test.
        let buf60 = render_at(60, 20, &d);
        assert!(interior_cols(&buf60, 2).contains("change-12"));
    }

    #[test]
    fn rows_do_not_overwrite_the_borders() {
        let names: Vec<Change> = (0..30)
            .map(|i| {
                fixture::active(
                    &format!("a-very-long-change-name-that-will-not-fit-here-{i:02}"),
                    1,
                    2,
                )
            })
            .collect();
        let mut d = dashboard_with(names, Vec::new(), 0, Route::List);
        // detail-scroll: the border assertion below must hold for a
        // markdown document too, not only for over-wide list rows.
        d.detail.source = (0..30).map(|_| format!("{}\n", "x".repeat(200))).collect();

        // `pane-chrome`: there is no border any more. Columns 0 and 59 are the
        // narrow layout's own gutters — spaces on every row of the body — and at
        // 120 columns 0, 39, and 41 are the wide layout's gutters and column 40
        // is the divider `│`. No change is selected here (`selected` 0 addresses
        // the active section header), so the detail region's own interior —
        // including its gutter-free last column, 119 — stays blank; that claim
        // belongs to `the_divider_has_a_blank_column_on_each_side_at_120_columns`
        // and to `detail-header`, not to this over-wide-list-row scenario.
        let buf60 = render_at(60, 20, &d);
        for y in 0..=18u16 {
            assert_eq!(cell(&buf60, 0, y).symbol(), " ", "y={y}");
            assert_eq!(cell(&buf60, 59, y).symbol(), " ", "y={y}");
        }

        let buf120 = render_at(120, 20, &d);
        for y in 0..=18u16 {
            assert_eq!(cell(&buf120, 0, y).symbol(), " ", "y={y}");
            assert_eq!(cell(&buf120, 39, y).symbol(), " ", "y={y}");
            assert_eq!(cell(&buf120, 40, y).symbol(), "│", "y={y}");
            assert_eq!(cell(&buf120, 41, y).symbol(), " ", "y={y}");
        }
    }

    fn five_change_dashboard(selected: usize) -> Dashboard {
        dashboard_with(
            vec![
                fixture::active("add-token-refresh", 4, 9),
                fixture::active("fix-empty-basket", 7, 7),
                fixture::active("migrate-ai-sdk-v7", 0, 0),
            ],
            vec![
                fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                fixture::archived(None, "legacy-cleanup", 3, 3),
            ],
            selected,
            Route::List,
        )
    }

    #[test]
    fn the_prompt_replaces_the_hints_while_filtering() {
        let mut d = five_change_dashboard(0);
        d.filter = Filter {
            query: "add".to_string(),
            active: true,
        };
        let buf60 = render_at(60, 20, &d);
        assert_eq!(row_text(&buf60, 19), format!("/add_{}", " ".repeat(55)));
        let buf120 = render_at(120, 20, &d);
        assert_eq!(row_text(&buf120, 19), format!("/add_{}", " ".repeat(115)));
        for buf in [&buf60, &buf120] {
            let footer = row_text(buf, 19);
            assert!(!footer.contains("q quit"));
            assert!(!footer.contains("Enter detail"));
            assert!(!footer.contains("Esc back"));
        }

        // `agent-attribution`: the unattributed count is replaced along with the
        // hints — the prompt replaces the whole row, not the three key hints alone.
        d.agents.agents = vec![
            unattributed_agent("nothing-like-a-change"),
            unattributed_agent("also-nothing"),
        ];
        for width in [60u16, 120u16] {
            assert!(!row_text(&render_at(width, 20, &d), 19).contains("unattributed"));
        }

        // `agent-launch`: the same holds with `agents.reachable` set to `true` — the prompt
        // replaces the action hints too.
        d.agents.reachable = true;
        for width in [60u16, 120u16] {
            let footer = row_text(&render_at(width, 20, &d), 19);
            assert!(!footer.contains("a/c/s launch"), "width {width}");
            assert!(!footer.contains("g focus"), "width {width}");
        }
    }

    #[test]
    fn an_accepted_query_leads_the_hint_list() {
        let mut d = five_change_dashboard(0);
        d.filter = Filter {
            query: "add".to_string(),
            active: false,
        };
        let expected = "/add  q quit  Enter detail  Esc back";
        assert_eq!(columns(expected), 36);
        let buf60 = render_at(60, 20, &d);
        assert_eq!(
            row_text(&buf60, 19),
            format!("{expected}{}", " ".repeat(24))
        );
        let buf120 = render_at(120, 20, &d);
        assert_eq!(
            row_text(&buf120, 19),
            format!("{expected}{}", " ".repeat(84))
        );

        d.filter.query = String::new();
        let buf60 = render_at(60, 20, &d);
        assert_eq!(
            row_text(&buf60, 19),
            format!("q quit  Enter detail  Esc back{}", " ".repeat(30))
        );
        let buf120 = render_at(120, 20, &d);
        assert_eq!(
            row_text(&buf120, 19),
            format!("q quit  Enter detail  Esc back{}", " ".repeat(90))
        );

        // `agent-attribution`: with a query leading and one unattributed agent, the
        // count still trails the query at both widths.
        d.filter.query = "add".to_string();
        d.agents.agents = vec![unattributed_agent("nothing-like-a-change")];
        let with_count = "/add  q quit  Enter detail  Esc back  1 unattributed";
        assert_eq!(columns(with_count), 52);
        let buf60 = render_at(60, 20, &d);
        assert_eq!(
            row_text(&buf60, 19),
            format!("{with_count}{}", " ".repeat(8))
        );
        let buf120 = render_at(120, 20, &d);
        assert_eq!(
            row_text(&buf120, 19),
            format!("{with_count}{}", " ".repeat(68))
        );

        // `agent-launch`: with `agents.reachable` set, the action hints sit between
        // `Esc back` and the count, and at 60 the count is the one dropped.
        d.agents.reachable = true;
        let with_hints_and_count =
            "/add  q quit  Enter detail  Esc back  a/c/s launch  g focus  1 unattributed";
        assert_eq!(columns(with_hints_and_count), 75);
        assert!(
            row_text(&render_at(120, 20, &d), 19).starts_with(with_hints_and_count),
            "width 120"
        );
        let with_hints_only = "/add  q quit  Enter detail  Esc back  a/c/s launch  g focus";
        assert!(
            row_text(&render_at(60, 20, &d), 19).starts_with(with_hints_only),
            "width 60: the count is the one dropped"
        );
    }

    #[test]
    fn a_prompt_longer_than_the_footer_keeps_its_tail() {
        let mut d = five_change_dashboard(0);
        d.filter = Filter {
            query: "aaaaaaaaaa".repeat(7),
            active: true,
        };
        let buf60 = render_at(60, 20, &d);
        let row60 = row_text(&buf60, 19);
        assert_eq!(columns(&row60), 60);
        assert!(row60.ends_with('_'));
        assert_eq!(row60.chars().next(), Some('a'));

        let buf120 = render_at(120, 20, &d);
        let row120 = row_text(&buf120, 19);
        let expected = format!("/{}_{}", "a".repeat(70), " ".repeat(48));
        assert_eq!(row120, expected);

        // `agent-launch`: the same holds with `agents.reachable` `true`, because the prompt
        // form has no hint list to grow.
        d.agents.reachable = true;
        assert_eq!(row_text(&render_at(60, 20, &d), 19), row60);
        assert_eq!(row_text(&render_at(120, 20, &d), 19), row120);
    }

    #[test]
    fn a_query_narrows_both_tiers() {
        // `list-sections`: `selected` **1** addresses `Target::Change(0)` —
        // `add-token-refresh`, the query's one active match — both before and
        // after filtering, since the active header stays target 0 either way.
        let mut d = five_change_dashboard(1);
        d.filter.query = "add".to_string();
        let buf120 = render_at(120, 20, &d);
        assert_eq!(
            interior_cols(&buf120, 2),
            format!("{:<38}", "  ▾ active (1)")
        );
        assert_eq!(
            interior_cols(&buf120, 3),
            "> add-token-refresh              [4/9]"
        );
        assert_eq!(
            interior_cols(&buf120, 4),
            format!("{:<38}", "  ▾ archived (1)")
        );
        assert_eq!(
            interior_cols(&buf120, 5),
            "  2026-08-14 add-auth            [7/7]"
        );
        let buf60 = render_at(60, 20, &d);
        assert_eq!(
            interior_cols(&buf60, 2),
            format!("{:<58}", "  ▾ active (1)")
        );
        assert_eq!(
            interior_cols(&buf60, 3),
            "> add-token-refresh                                  [4/9]"
        );
        assert_eq!(
            interior_cols(&buf60, 4),
            format!("{:<58}", "  ▾ archived (1)")
        );
        assert_eq!(
            interior_cols(&buf60, 5),
            "  2026-08-14 add-auth                                [7/7]"
        );
        for buf in [&buf120, &buf60] {
            assert!(!buffer_contains(buf, "fix-empty-basket"));
            assert!(!buffer_contains(buf, "migrate-ai-sdk-v7"));
            assert!(!buffer_contains(buf, "legacy-cleanup"));
            assert!(!buffer_contains(buf, "-- archived"));
        }
    }

    #[test]
    fn matching_ignores_case() {
        // Compares the drawn *rows*, not the whole buffer: the footer
        // legitimately echoes the literal query text typed (`/ADD` vs.
        // `/add`), which is a separate, un-normalised concern from which
        // changes match.
        let lower = {
            let mut d = five_change_dashboard(0);
            d.filter.query = "add".to_string();
            d
        };
        for query in ["ADD", "Add"] {
            let mut d = five_change_dashboard(0);
            d.filter.query = query.to_string();
            for width in [60, 120] {
                let buf = render_at(width, 20, &d);
                let buf_lower = render_at(width, 20, &lower);
                for y in 2..=17u16 {
                    assert_eq!(
                        interior_cols(&buf, y),
                        interior_cols(&buf_lower, y),
                        "query {query:?} width {width} row {y}"
                    );
                }
            }
        }

        // The lowercasing applies to both sides: an upper-case change name
        // still matches a lower-case query.
        let mut upper_name = dashboard_with(
            vec![fixture::active("ADD-TOKEN-REFRESH", 4, 9)],
            Vec::new(),
            0,
            Route::List,
        );
        upper_name.filter.query = "add".to_string();
        let buf = render_at(120, 20, &upper_name);
        assert!(buffer_contains(&buf, "ADD-TOKEN-REFRESH"));
    }

    #[test]
    fn a_query_matching_only_an_archived_change() {
        // `list-sections`: `selected` **1** addresses `Target::Change(0)` —
        // `add-auth`, the only match — since the archived header is target 0
        // (the active section's count is zero and it emits no header).
        let mut d = five_change_dashboard(1);
        d.filter.query = "auth".to_string();
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(interior_cols(&buf, 2).starts_with("No active changes"));
            assert_eq!(
                interior_cols(&buf, 3),
                format!(
                    "{:<w$}",
                    "  ▾ archived (1)",
                    w = if width == 60 { 58 } else { 38 }
                )
            );
            assert!(interior_cols(&buf, 4).contains("add-auth"));
            assert_eq!(cell(&buf, 1, 4).symbol(), ">");
            assert!(!buffer_contains(&buf, "No changes match"));
            assert!(!buffer_contains(&buf, "No changes yet"));
            assert!(!buffer_contains(&buf, "-- archived"));
        }
    }

    #[test]
    fn a_query_matching_nothing_names_itself() {
        let mut d = five_change_dashboard(0);
        d.filter.query = "zzz".to_string();
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(interior_cols(&buf, 2).starts_with("No changes match"));
            assert!(interior_cols(&buf, 3).starts_with("/zzz"));
            assert!(!buffer_contains(&buf, "-- archived"));
            for name in [
                "add-token-refresh",
                "fix-empty-basket",
                "migrate-ai-sdk-v7",
                "add-auth",
                "legacy-cleanup",
            ] {
                assert!(!buffer_contains(&buf, name));
            }
            for y in 2..=17u16 {
                assert_ne!(cell(&buf, 1, y).symbol(), ">");
            }
        }
    }

    #[test]
    fn shrinking_the_visible_list_clamps_the_marker() {
        let mut d = five_change_dashboard(4);
        d.apply(Action::FilterStart);
        for c in ['a', 'd', 'd'] {
            d.apply(Action::FilterPush(c));
        }
        // `list-sections`: 3, not 1 — the query "add" matches one active and
        // one archived change, so targets are the active header,
        // `add-token-refresh`, the archived header, and `add-auth`; the
        // clamp lands on the last of those four.
        assert_eq!(d.selected, 3);
        assert_eq!(d.selected_change().unwrap().name, "add-auth");
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            // Rows: 2 active header, 3 add-token-refresh, 4 archived header, 5
            // add-auth — the marker is now correctly on row 5.
            assert!(interior_cols(&buf, 5).contains("add-auth"));
            assert_eq!(cell(&buf, 1, 5).symbol(), ">", "width {width}");
            assert!(is_bold(cell(&buf, 1, 5)), "width {width}");
            for y in [2u16, 3, 4] {
                assert_ne!(cell(&buf, 1, y).symbol(), ">", "width {width} y={y}");
            }
        }
    }

    #[test]
    fn slash_starts_filter_mode_and_the_list_is_shown() {
        // `pane-chrome`: there is no `Changes` title any more — the list being
        // shown is asserted by one of its own rows appearing instead.
        let mut d = five_change_dashboard(0);
        d.route = Route::Detail;
        d.apply(Action::FilterStart);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(buffer_contains(&buf, "add-token-refresh"));
        }
    }

    /// The twenty bullet items every detail-scroll scenario in this module
    /// renders: `- line-00` through `- line-19`, generated rather than
    /// hand-written for the same reason `changes_named` is.
    fn twenty_line_source() -> String {
        (0..20).map(|i| format!("- line-{i:02}\n")).collect()
    }

    /// A dashboard whose one selected active change carries one artifact —
    /// `detail-scroll`'s scenarios are about a selected change's content,
    /// and after `detail-view` an empty visible list draws nothing at all,
    /// so every one of them needs a change to keep exercising what it was
    /// written for. That one artifact is explicitly **not** marked
    /// `tracks_tasks` — `fixture::with_artifacts` sets it `false` and this
    /// helper never flips it, so a caller's markdown-rendering assumption
    /// is stated by the fixture it reaches for, not by leaving a bit at
    /// its zero value.
    fn detail_dashboard(source: String, scroll: usize, route: Route) -> Dashboard {
        let change =
            fixture::with_artifacts(fixture::active("detail-view", 4, 9), &[("proposal", &[])]);
        Dashboard {
            repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
            searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
            changes: fixture::set(vec![change], Vec::new(), Vec::new()),
            route,
            quit: false,
            // `list-sections`: 1, not 0 — target 0 is the active header.
            selected: 1,
            filter: empty_filter(),
            detail: Detail {
                source,
                scroll,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
            },
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

    /// `artifact-content` :: "A wide-character document stays inside the detail region" —
    /// a discriminating instance of the scenario `detail.rs`'s own test already carries.
    /// That landed fixture (a CJK paragraph, a heading, and a family-emoji bullet) passes
    /// today "by construction": every one of its lines is a **single** segment, so the
    /// per-segment loop never consults its own (buggy) cursor before drawing it — the
    /// segment is already bounded to the line's width by `content_lines` itself, and
    /// `Buffer::set_string` draws it correctly regardless of how `x` was tracked.
    ///
    /// This fixture instead builds a line with **two** segments so the cursor is actually
    /// consulted between them. The first (plain) segment is a run of `a`s with seven
    /// trailing zero-width combining marks — `columns` does not count them, `chars().count()`
    /// does, so the old cursor **over**-counts what it consumed. Over-counting, not
    /// under-counting, is the direction that actually crosses a border: it leaves the
    /// second (bold) segment's start too far **right**, so that segment's own — entirely
    /// correct — width carries it past `last_col`. (A pure CJK run under-counts instead,
    /// which shifts a following segment left into an overlap, never past the border; that
    /// is the accident the landed fixture relies on.) The two segments are sized so the
    /// line's total is exactly the interior width — `content_lines`' own width contract —
    /// so the fix's clamp is provably a no-op here: the discriminator is the corrected
    /// advance in `render_detail_content`, not the clamp. See this group's own report for
    /// why no test here exercises the clamp.
    fn combining_mark_overrun_source(interior: usize) -> String {
        let plain_cols = interior - 8;
        format!(
            "{}{}**OVERRUN!**",
            "a".repeat(plain_cols),
            "\u{301}".repeat(7)
        )
    }

    #[test]
    fn a_combining_mark_inflated_segment_stays_inside_the_detail_region() {
        // 60-column frame: the detail interior is 58 columns, content starts at row 5,
        // column 1, and the region's own trailing gutter sits at column 59 — the narrow
        // layout keeps both gutters, unlike the wide one (D5).
        let d58 = detail_dashboard(combining_mark_overrun_source(58), 0, Route::Detail);
        let buf58 = render_at(60, 20, &d58);
        for x in 1..50u16 {
            assert_eq!(cell(&buf58, x, 5).symbol(), "a", "x={x}");
        }
        assert!(
            cell(&buf58, 50, 5).symbol().starts_with('a'),
            "the last cluster of the plain run carries the combining marks: {:?}",
            cell(&buf58, 50, 5).symbol()
        );
        assert!(
            row_text(&buf58, 5).contains("OVERRUN!"),
            "the bold segment must still be drawn, just inside the gutter: {:?}",
            row_text(&buf58, 5)
        );
        assert_eq!(
            cell(&buf58, 59, 5).symbol(),
            " ",
            "the region's trailing gutter must survive the over-counted segment"
        );

        // 120-column frame: the detail interior is 78 columns, content starts at column
        // 42, and — the wide detail region having no right gutter (D5) — reaches the
        // frame's own last column, 119, rather than stopping one short of it.
        let d78 = detail_dashboard(combining_mark_overrun_source(78), 0, Route::Detail);
        let buf78 = render_at(120, 20, &d78);
        for x in 42..111u16 {
            assert_eq!(cell(&buf78, x, 5).symbol(), "a", "x={x}");
        }
        assert!(cell(&buf78, 111, 5).symbol().starts_with('a'));
        assert!(row_text(&buf78, 5).contains("OVERRUN!"));
        assert_ne!(
            cell(&buf78, 119, 5).symbol(),
            " ",
            "the over-counted segment must not have been clipped short of the frame's last \
             column"
        );
    }

    /// Column range `1..=9` at 60, `42..=50` at 120 — the first nine
    /// columns of the detail interior at either mandated width.
    fn detail_marker_cols(buf: &Buffer, y: u16) -> String {
        let (from, to) = if buf.area.width == 60 {
            (1, 9)
        } else {
            (42, 50)
        };
        cols(&row_text(buf, y), from..to + 1)
    }

    #[test]
    fn the_detail_document_fills_the_interior_at_60_and_120() {
        let base = detail_dashboard(twenty_line_source(), 0, Route::List);
        let mut d = Dashboard {
            repo: base.repo,
            searched_from: base.searched_from,
            changes: fixture::set(
                vec![fixture::active("fix-empty-basket", 7, 7)],
                Vec::new(),
                Vec::new(),
            ),
            route: base.route,
            quit: base.quit,
            selected: base.selected,
            filter: base.filter,
            detail: base.detail,
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
        };

        let buf120 = render_at(120, 20, &d);
        assert_eq!(detail_marker_cols(&buf120, 5), "- line-00");
        assert_eq!(detail_marker_cols(&buf120, 18), "- line-13");
        // `list-sections`: row 2 is now the active section header; the change
        // row is row 3. `base.selected` (1) addresses `Target::Change(0)` —
        // the one active change — since the active header is target 0, so
        // the row carries the marker.
        assert_eq!(
            cols(&row_text(&buf120, 3), 1..39),
            "> fix-empty-basket               [7/7]"
        );

        d.route = Route::Detail;
        let buf60 = render_at(60, 20, &d);
        assert_eq!(detail_marker_cols(&buf60, 5), "- line-00");
        assert_eq!(detail_marker_cols(&buf60, 18), "- line-13");
    }

    /// The style of the first cell of the first (by-char, never by-byte —
    /// a box-drawing border is multi-byte) occurrence of `needle` anywhere
    /// in `buf`.
    fn find_cell_style(buf: &Buffer, needle: &str) -> ratatui::style::Style {
        let needle_chars: Vec<char> = needle.chars().collect();
        for y in 0..buf.area.height {
            let row_chars: Vec<char> = row_text(buf, y).chars().collect();
            if let Some(pos) = row_chars
                .windows(needle_chars.len())
                .position(|w| w == needle_chars.as_slice())
            {
                return cell(buf, pos as u16, y).style();
            }
        }
        panic!("{needle:?} not found in the buffer");
    }

    #[test]
    fn faces_reach_the_buffer_as_styles() {
        // `color-palette`: the heading fixture becomes the `## Heading` this scenario has
        // named since `markdown-viewer` — the delta's colour claim is about `Heading(2)`,
        // and the level-1 form is exercised by `faces_reach_the_buffer_as_coloured_styles`.
        let source = "## Heading\n\nA **bold** and *italic* line with `code` and [a link](x) \
                      and ~~struck~~.\n\n> quoted\n";
        let d = detail_dashboard(source.to_string(), 0, Route::Detail);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);

            let title_row: Vec<char> = row_text(&buf, 5).chars().collect();
            let needle: Vec<char> = "## Heading".chars().collect();
            let title_start = title_row
                .windows(needle.len())
                .position(|w| w == needle.as_slice())
                .expect("heading present");
            for i in 0..needle.len() {
                let x = (title_start + i) as u16;
                assert!(
                    cell(&buf, x, 5)
                        .style()
                        .add_modifier
                        .contains(Modifier::BOLD),
                    "width {width}: heading cell {i} not bold"
                );
                assert_eq!(
                    cell(&buf, x, 5).style().fg,
                    palette::style(Role::Heading(2)).fg,
                    "width {width}: heading cell {i} does not carry Heading(2)'s foreground"
                );
            }

            assert!(
                find_cell_style(&buf, "bold")
                    .add_modifier
                    .contains(Modifier::BOLD),
                "width {width}"
            );
            assert!(
                find_cell_style(&buf, "italic")
                    .add_modifier
                    .contains(Modifier::ITALIC),
                "width {width}"
            );
            assert!(
                find_cell_style(&buf, "code")
                    .add_modifier
                    .contains(Modifier::DIM),
                "width {width}"
            );
            assert!(
                find_cell_style(&buf, "a link")
                    .add_modifier
                    .contains(Modifier::UNDERLINED),
                "width {width}"
            );
            assert!(
                find_cell_style(&buf, "quoted")
                    .add_modifier
                    .contains(Modifier::DIM),
                "width {width}"
            );
            assert!(
                find_cell_style(&buf, "struck")
                    .add_modifier
                    .contains(Modifier::CROSSED_OUT),
                "width {width}: the struck run is not crossed out"
            );

            let and_style = find_cell_style(&buf, "and");
            assert!(
                !and_style.add_modifier.contains(Modifier::BOLD),
                "width {width}"
            );
            assert!(
                !and_style.add_modifier.contains(Modifier::ITALIC),
                "width {width}"
            );
            assert!(
                !and_style.add_modifier.contains(Modifier::DIM),
                "width {width}"
            );
            assert!(
                !and_style.add_modifier.contains(Modifier::UNDERLINED),
                "width {width}"
            );
            assert!(
                !and_style.add_modifier.contains(Modifier::CROSSED_OUT),
                "width {width}"
            );

            // `color-palette`: three of these faces now carry a foreground as well, and
            // three still carry none. The plain control carries none either.
            assert_eq!(
                find_cell_style(&buf, "code").fg,
                palette::style(Role::Code).fg,
                "width {width}"
            );
            assert_eq!(
                find_cell_style(&buf, "a link").fg,
                palette::style(Role::Link).fg,
                "width {width}"
            );
            assert_eq!(
                find_cell_style(&buf, "bold").fg,
                uncoloured().fg,
                "width {width}"
            );
            assert_eq!(
                find_cell_style(&buf, "italic").fg,
                uncoloured().fg,
                "width {width}"
            );
            assert_eq!(
                find_cell_style(&buf, "quoted").fg,
                uncoloured().fg,
                "width {width}"
            );
            assert_eq!(
                find_cell_style(&buf, "struck").fg,
                uncoloured().fg,
                "width {width}: struck must carry no foreground"
            );
            assert_eq!(and_style.fg, uncoloured().fg, "width {width}");
        }
    }

    /// The character offsets of every `|` in `row`, counted in characters and
    /// never in bytes.
    fn pipe_offsets(row: &str) -> Vec<usize> {
        row.chars()
            .enumerate()
            .filter(|(_, c)| *c == '|')
            .map(|(i, _)| i)
            .collect()
    }

    /// `detail-scroll` :: "A table reaches the buffer aligned and inside the region".
    ///
    /// The outer-loop acceptance test, written RED before any parser or allocator
    /// work: it is the only test that proves the whole path — the `Options` flag,
    /// `fold`, the column allocation, the wrap, the alignment, the header face,
    /// the palette, and `Buffer::set_string` — reaches a real buffer without
    /// touching a border. Every unit test in `ui::markdown` sits inside it.
    ///
    /// The fixture's natural widths are `[6, 12, 40]` and `n` is 3, so the pipe
    /// overhead is `3n + 1 = 10`. At the 78-column interior `avail = 68` and the
    /// natural widths sum to 58, so `w = nat` and no cell wraps. At the
    /// 58-column interior `avail = 48`, the max-min rule gives `w = [6, 12, 30]`,
    /// and the 40-column note wraps into two lines — which is what makes the
    /// wrapping row taller at 60 than at 120 rather than the fixture merely
    /// being long.
    #[test]
    fn a_table_reaches_the_buffer_aligned() {
        let source = "| Gate | Runner | Notes |\n\
                      |---|---|---|\n\
                      | Format | cargo fmt | quick |\n\
                      | Lint | cargo clippy | this note wraps at the narrow width only |\n\
                      | Test | cargo test | short |\n";
        let d = detail_dashboard(source.to_string(), 0, Route::Detail);

        let mut table_row_counts = Vec::new();
        for width in [120u16, 60u16] {
            let buf = render_at(width, 20, &d);

            // The content area is rows 5 through 18: the region's heading row,
            // its padding row, the tab bar, the rule, and the content's own
            // padding row all sit above it, and the frame's footer below.
            let interior = if width == 60 { 58 } else { 78 };
            let drawn: Vec<(u16, String)> = (5..=18u16)
                .map(|y| (y, detail_interior_cols(&buf, y, interior)))
                .filter(|(_, text)| text.contains('|'))
                .collect();
            assert!(
                drawn.len() >= 5,
                "width {width}: the table drew {} lines, not the header, delimiter, and \
                 three body rows at least: {drawn:?}",
                drawn.len()
            );
            table_row_counts.push(drawn.len());

            // The delimiter line is the one whose content is only pipes and
            // dashes; its pipe offsets are what every row line's must equal.
            let delimiter = drawn
                .iter()
                .find(|(_, text)| {
                    let t = text.trim_end();
                    !t.is_empty() && t.chars().all(|c| c == '|' || c == '-')
                })
                .expect("a delimiter line is drawn");
            let expected = pipe_offsets(&delimiter.1);
            assert_eq!(
                expected.len(),
                4,
                "width {width}: a three-column table's delimiter line holds n + 1 = 4 pipes: \
                 {:?}",
                delimiter.1
            );
            for (y, text) in &drawn {
                assert_eq!(
                    pipe_offsets(text),
                    expected,
                    "width {width}, row {y}: the columns are not aligned with the delimiter \
                     line's: {text:?}"
                );
            }

            // The header cells read bold; the pipes and the padding spaces
            // around them carry no modifier at all.
            let (header_y, header_text) = &drawn[0];
            let offset = if width == 60 { 1u16 } else { 42 };
            for label in ["Gate", "Runner", "Notes"] {
                let start = header_text
                    .find(label)
                    .unwrap_or_else(|| panic!("width {width}: {label} is drawn"));
                let start = header_text[..start].chars().count();
                for i in 0..label.chars().count() {
                    let x = offset + (start + i) as u16;
                    assert!(
                        cell(&buf, x, *header_y)
                            .style()
                            .add_modifier
                            .contains(Modifier::BOLD),
                        "width {width}: {label}'s cell {i} is not bold"
                    );
                }
            }
            for (i, c) in header_text.chars().enumerate() {
                if c != '|' && c != ' ' {
                    continue;
                }
                let x = offset + i as u16;
                assert_eq!(
                    cell(&buf, x, *header_y).style().add_modifier,
                    Modifier::empty(),
                    "width {width}: the pipe or padding cell at interior column {i} carries \
                     a modifier"
                );
            }

            // No table cell reaches a gutter or the divider. Column 119 is
            // deliberately not in this list — the wide detail region has no
            // right gutter (D5) — and below the breakpoint `layout::split_body`
            // gives the detail region the whole body, so columns 39 and 40 are
            // ordinary interior content at 60 and a table wrapping at 58
            // necessarily covers them.
            let gutters: &[u16] = if width == 60 { &[0, 59] } else { &[0, 39, 41] };
            for y in 0..=18u16 {
                for x in gutters {
                    assert_eq!(
                        cell(&buf, *x, y).symbol(),
                        " ",
                        "width {width}: x={x} y={y} is not blank"
                    );
                }
            }
            if width == 120 {
                for y in 0..=18u16 {
                    assert_eq!(cell(&buf, 40, y).symbol(), "│", "y={y}");
                }
            }
        }

        // The wrapping row occupies more rows at 60 than at 120.
        assert!(
            table_row_counts[1] > table_row_counts[0],
            "the row that wraps at 58 must make the table taller at 60 ({} lines) than at \
             120 ({} lines)",
            table_row_counts[1],
            table_row_counts[0]
        );
    }

    #[test]
    fn an_empty_detail_source_leaves_the_interior_blank() {
        // The scenario's name is kept verbatim; its subject moves from "an
        // empty source" to "no change selected" — with a change selected
        // the region is never blank, `No content yet` is drawn instead.
        let no_change = Dashboard {
            repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
            searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
            changes: fixture::set(Vec::new(), Vec::new(), Vec::new()),
            route: Route::Detail,
            quit: false,
            selected: 0,
            filter: empty_filter(),
            detail: Detail {
                source: String::new(),
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
            },
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
        };
        let default_style = Cell::default().style();

        let buf120 = render_at(120, 20, &no_change);
        for y in 0..=18u16 {
            for x in 41..=119u16 {
                let c = cell(&buf120, x, y);
                assert_eq!(c.symbol(), " ", "x={x} y={y}");
                assert_eq!(c.style(), default_style, "x={x} y={y}");
            }
        }

        let buf60 = render_at(60, 20, &no_change);
        for y in 0..=18u16 {
            for x in 1..=58u16 {
                let c = cell(&buf60, x, y);
                assert_eq!(c.symbol(), " ", "x={x} y={y}");
                assert_eq!(c.style(), default_style, "x={x} y={y}");
            }
        }

        // Discriminating companion: a change selected with an empty source
        // shows "No content yet" and is therefore not blank.
        let with_change = detail_dashboard(String::new(), 0, Route::Detail);
        for width in [120, 60] {
            let buf = render_at(width, 20, &with_change);
            assert!(
                detail_interior_cols(&buf, 5, 14).starts_with("No content yet"),
                "width {width}"
            );
        }

        // A fourth dashboard, identical to the third but with its artifact
        // marked `tracks_tasks`: still "No content yet", not blank, and no
        // progress bar — a marked tab with no file is not a checklist.
        let marked_change = fixture::track_tasks_at(
            fixture::with_artifacts(fixture::active("detail-view", 4, 9), &[("proposal", &[])]),
            0,
        );
        let with_marked_change = dashboard_with_detail(
            vec![marked_change],
            Vec::new(),
            1, // `list-sections`: target 0 is the active header.
            Route::Detail,
            empty_detail_with_tab("", Vec::new(), 0),
        );
        for width in [120, 60] {
            let buf = render_at(width, 20, &with_marked_change);
            assert!(
                detail_interior_cols(&buf, 5, 14).starts_with("No content yet"),
                "width {width}: marked-but-empty"
            );
            assert!(
                !row_text(&buf, 5).contains('█') && !row_text(&buf, 5).contains('░'),
                "width {width}: no progress bar"
            );
        }
    }

    /// `pane-chrome` removed the border: every cell of columns 0, 39, and 41
    /// is a space and column 40 is the divider `│` at the wide layout, and
    /// every cell of columns 0 and 59 is a space at the narrow one — over
    /// every row of the frame, heading and padding rows included. Column
    /// 119 is deliberately absent from the wide sweep: the wide detail
    /// region has no right gutter (D5), so content legitimately reaches it.
    fn assert_gutters_and_divider_untouched(buf: &Buffer, label: &str) {
        let width = buf.area.width;
        for y in 0..=18u16 {
            if width == 60 {
                for x in [0u16, 59] {
                    assert_eq!(cell(buf, x, y).symbol(), " ", "{label} x={x} y={y}");
                }
            } else {
                for x in [0u16, 39, 41] {
                    assert_eq!(cell(buf, x, y).symbol(), " ", "{label} x={x} y={y}");
                }
                assert_eq!(cell(buf, 40, y).symbol(), "│", "{label} y={y}");
            }
        }
    }

    #[test]
    fn content_never_overwrites_the_detail_region_s_border() {
        let source: String = (0..20).map(|_| format!("{}\n", "x".repeat(200))).collect();
        let d = detail_dashboard(source, 0, Route::Detail);

        let buf120 = render_at(120, 20, &d);
        assert_gutters_and_divider_untouched(&buf120, "base 120");
        // The wide detail region has no right gutter (D5): column 119 does
        // carry content, rather than staying blank like the gutters above.
        assert_ne!(cell(&buf120, 119, 5).symbol(), " ", "base 120: column 119");
        // Every content row (5 through 18) holds some of the 200-character
        // lines; row 4, the content's own padding row, is excluded — it is
        // blank by design, not a failure of this sweep.
        for y in [2u16, 3].into_iter().chain(5..=18u16) {
            assert!(
                !cols(&row_text(&buf120, y), 42..120)
                    .chars()
                    .all(|c| c == ' '),
                "base 120 row {y} is unexpectedly blank"
            );
        }

        let buf60 = render_at(60, 20, &d);
        assert_gutters_and_divider_untouched(&buf60, "base 60");
        for y in [2u16, 3].into_iter().chain(5..=18u16) {
            assert!(
                !cols(&row_text(&buf60, y), 1..59).chars().all(|c| c == ' '),
                "base 60 row {y} is unexpectedly blank"
            );
        }

        // The same holds with the selected tab marked `tracks_tasks` and
        // the same source turned into thirty 200-character task lines
        // under a 200-character heading — neither the checklist's wrap,
        // the heading's truncation, nor the progress bar's gauge can
        // reach a gutter or the divider. Found in Change Review: the
        // heading line originally had no width treatment at all.
        let task_source: String = std::iter::once(format!("## {}\n", "h".repeat(200)))
            .chain((0..30).map(|_| format!("- [ ] {}\n", "x".repeat(200))))
            .collect();
        let marked_change = fixture::track_tasks_at(
            fixture::with_artifacts(fixture::active("detail-view", 4, 9), &[("proposal", &[])]),
            0,
        );
        let marked_d = dashboard_with_detail(
            vec![marked_change],
            Vec::new(),
            1, // `list-sections`: target 0 is the active header.
            Route::Detail,
            empty_detail_with_tab(&task_source, Vec::new(), 0),
        );

        let mbuf120 = render_at(120, 20, &marked_d);
        assert_gutters_and_divider_untouched(&mbuf120, "marked 120");
        let mbuf60 = render_at(60, 20, &marked_d);
        assert_gutters_and_divider_untouched(&mbuf60, "marked 60");

        // `markdown-constructs`: and the same again with a twelve-column table
        // whose every cell is 200 characters long. Neither the pipe grammar nor
        // the one-cell-per-line fallback it degrades to can reach a gutter or
        // the divider — and this fixture is the tightest case there is, because
        // the allocator spends the whole interior: `3n + 1 = 37` plus `avail`
        // is exactly 78 at the wide interior and exactly 58 at the narrow one.
        let wide_cell = "x".repeat(200);
        let cells = format!(" {wide_cell} |").repeat(12);
        let table_source = format!("|{cells}\n|{}\n|{cells}\n", "---|".repeat(12));
        let table_d = detail_dashboard(table_source, 0, Route::Detail);

        let tbuf120 = render_at(120, 20, &table_d);
        assert_gutters_and_divider_untouched(&tbuf120, "table 120");
        let tbuf60 = render_at(60, 20, &table_d);
        assert_gutters_and_divider_untouched(&tbuf60, "table 60");
        // The table really is drawn: the fixture would satisfy the gutter
        // assertions above by rendering nothing at all.
        for (label, buf) in [("120", &tbuf120), ("60", &tbuf60)] {
            assert!(
                (5..=18u16).any(|y| row_text(buf, y).contains('|')),
                "width {label}: no table line was drawn"
            );
        }

        // Added during Change Review: the twelve-column fixture never reaches
        // the one-cell-per-line fallback — `3n + 1` is 37, so `avail` is 41 and
        // 21, both at least `n`, and the pipe grammar is used at both widths. A
        // fifteen-column one is what measures the other half of the scenario's
        // claim: its `4n + 1` of 61 the 78-column interior clears and the
        // 58-column one does not, so the fallback is what draws at 60.
        let cells15 = format!(" {wide_cell} |").repeat(15);
        let fallback_source = format!("|{cells15}\n|{}\n|{cells15}\n", "---|".repeat(15));
        let fallback_d = detail_dashboard(fallback_source, 0, Route::Detail);

        let fbuf120 = render_at(120, 20, &fallback_d);
        assert_gutters_and_divider_untouched(&fbuf120, "fallback 120");
        let fbuf60 = render_at(60, 20, &fallback_d);
        assert_gutters_and_divider_untouched(&fbuf60, "fallback 60");
        // The two widths take the two different paths, which is the whole point
        // of this fixture: pipes at 120, none at 60.
        assert!(
            (5..=18u16).any(|y| row_text(&fbuf120, y).contains('|')),
            "at 120 the fifteen-column table still fits the pipe grammar"
        );
        assert!(
            (5..=18u16).all(|y| !row_text(&fbuf60, y).contains('|')),
            "at 60 the fifteen-column table must degrade to one cell per line"
        );
        assert!(
            (5..=18u16).any(|y| row_text(&fbuf60, y).contains('x')),
            "at 60 the fallback must still draw the cells"
        );
    }

    #[test]
    fn a_degenerate_detail_interior_draws_nothing_and_does_not_panic() {
        // The frame height a detail interior row costs is three — one
        // frame-footer row and the region's own heading and padding rows
        // — so the interior first has one row at a frame height of 4,
        // two at 5, three at 6, and four at 7. `split_detail` spends the
        // first three of those on the tab bar, the rule, and the
        // content's own padding row, so the content area first has one
        // row at a frame height of 7. `detail-scroll` -> "A degenerate
        // detail interior draws nothing and does not panic".
        let d = detail_dashboard(twenty_line_source(), 0, Route::Detail);

        // 1x20 and 2x20: no render panics; the spec states no further
        // assertion at these degenerate widths.
        for (w, h) in [(1u16, 20u16), (2, 20)] {
            let buf = render_at(w, h, &d);
            let _ = buf;
        }

        // 120x3 and 60x3: the interior has zero rows, so nothing at all
        // inside it is drawn — not a tab cell, not the rule, not a
        // markdown line — though the region's own heading row still
        // names the change.
        for width in [120u16, 60] {
            let buf = render_at(width, 3, &d);
            assert!(buffer_contains(&buf, "detail-view"), "width {width}");
            assert!(!buffer_contains(&buf, " proposal "), "width {width}");
            assert!(!buffer_contains(&buf, "─"), "width {width}");
            assert!(!buffer_contains(&buf, "line-00"), "width {width}");
        }

        // 120x4 and 60x4: the tab bar is drawn; no rule and no markdown
        // line appears anywhere in the frame.
        for width in [120u16, 60] {
            let buf = render_at(width, 4, &d);
            assert!(buffer_contains(&buf, " proposal "), "width {width}");
            assert!(!buffer_contains(&buf, "─"), "width {width}");
            assert!(!buffer_contains(&buf, "line-00"), "width {width}");
        }

        // 120x5 and 60x5: the tab bar and the rule are drawn; no markdown
        // line appears.
        for width in [120u16, 60] {
            let buf = render_at(width, 5, &d);
            assert!(buffer_contains(&buf, " proposal "), "width {width}");
            assert!(buffer_contains(&buf, "─"), "width {width}");
            assert!(!buffer_contains(&buf, "line-00"), "width {width}");
        }

        // 120x6 and 60x6: the same two are drawn and still no markdown
        // line, the interior's third row being the content padding row.
        for width in [120u16, 60] {
            let buf = render_at(width, 6, &d);
            assert!(buffer_contains(&buf, " proposal "), "width {width}");
            assert!(buffer_contains(&buf, "─"), "width {width}");
            assert!(!buffer_contains(&buf, "line-00"), "width {width}");
        }

        // 120x7 and 60x7: exactly one content row is drawn, holding the
        // source's first rendered line.
        for width in [120u16, 60] {
            let buf = render_at(width, 7, &d);
            assert!(buffer_contains(&buf, "detail-view"), "width {width}");
            assert!(buffer_contains(&buf, " proposal "), "width {width}");
            assert!(buffer_contains(&buf, "line-00"), "width {width}");
            assert!(!buffer_contains(&buf, "line-01"), "width {width}");
        }

        // Contrasting controls: content is present at both mandated widths.
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(buffer_contains(&buf, "line-00"));
        }

        // Every one of the above, repeated with the selected tab marked
        // `tracks_tasks`: the one content row at 120x7 and 60x7 holds the
        // progress bar rather than a task item.
        let task_source: String = (0..20).map(|i| format!("- [ ] line-{i:02}\n")).collect();
        let marked_change = fixture::track_tasks_at(
            fixture::with_artifacts(fixture::active("detail-view", 4, 9), &[("proposal", &[])]),
            0,
        );
        let md = dashboard_with_detail(
            vec![marked_change],
            Vec::new(),
            1, // `list-sections`: target 0 is the active header.
            Route::Detail,
            empty_detail_with_tab(&task_source, Vec::new(), 0),
        );

        for (w, h) in [(1u16, 20u16), (2, 20)] {
            let buf = render_at(w, h, &md);
            let _ = buf;
        }

        for width in [120u16, 60] {
            let buf = render_at(width, 3, &md);
            assert!(buffer_contains(&buf, "detail-view"), "marked width {width}");
            assert!(!buffer_contains(&buf, " proposal "), "marked width {width}");
            assert!(!buffer_contains(&buf, "line-00"), "marked width {width}");
        }

        for width in [120u16, 60] {
            let buf = render_at(width, 4, &md);
            assert!(buffer_contains(&buf, " proposal "), "marked width {width}");
            assert!(!buffer_contains(&buf, "─"), "marked width {width}");
            assert!(!buffer_contains(&buf, "line-00"), "marked width {width}");
        }

        for width in [120u16, 60] {
            let buf = render_at(width, 5, &md);
            assert!(buffer_contains(&buf, " proposal "), "marked width {width}");
            assert!(buffer_contains(&buf, "─"), "marked width {width}");
            assert!(!buffer_contains(&buf, "line-00"), "marked width {width}");
        }

        for width in [120u16, 60] {
            let buf = render_at(width, 6, &md);
            assert!(buffer_contains(&buf, " proposal "), "marked width {width}");
            assert!(buffer_contains(&buf, "─"), "marked width {width}");
            assert!(!buffer_contains(&buf, "line-00"), "marked width {width}");
        }

        for width in [120u16, 60] {
            let buf = render_at(width, 7, &md);
            assert!(buffer_contains(&buf, "detail-view"), "marked width {width}");
            assert!(buffer_contains(&buf, " proposal "), "marked width {width}");
            assert!(
                !buffer_contains(&buf, "line-00"),
                "marked width {width}: bar, not an item"
            );
            assert!(
                buffer_contains(&buf, "█") || buffer_contains(&buf, "[0/20]"),
                "marked width {width}: the one content row holds the progress bar"
            );
        }

        for width in [60, 120] {
            let buf = render_at(width, 20, &md);
            assert!(buffer_contains(&buf, "line-00"));
        }

        // `markdown-constructs`: and every one of those renders once more with a
        // table as the `detail.source`. 1x20 and 2x20 — where the interior is one
        // or zero columns wide — still draw nothing and still do not panic.
        let table_source: String = std::iter::once("| a | line-00 |\n|---|---|\n".to_string())
            .chain((1..20).map(|i| format!("| a | line-{i:02} |\n")))
            .collect();
        let td = detail_dashboard(table_source, 0, Route::Detail);

        for (w, h) in [(1u16, 20u16), (2, 20)] {
            let buf = render_at(w, h, &td);
            assert!(!buffer_contains(&buf, "line-00"), "table {w}x{h}");
        }
        for (height, tabs, content) in [
            (3u16, false, false),
            (4, true, false),
            (5, true, false),
            (6, true, false),
            (7, true, true),
        ] {
            for width in [120u16, 60] {
                let buf = render_at(width, height, &td);
                assert_eq!(
                    buffer_contains(&buf, " proposal "),
                    tabs,
                    "table {width}x{height}: tab bar"
                );
                assert_eq!(
                    buffer_contains(&buf, "line-00"),
                    content,
                    "table {width}x{height}: content"
                );
            }
        }
        for width in [60, 120] {
            let buf = render_at(width, 20, &td);
            assert!(buffer_contains(&buf, "line-00"), "table width {width}");
        }
    }

    #[test]
    fn a_scroll_past_the_end_draws_the_last_screenful() {
        let d = detail_dashboard(twenty_line_source(), 99, Route::Detail);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert_eq!(detail_marker_cols(&buf, 5), "- line-06", "width {width}");
            assert_eq!(detail_marker_cols(&buf, 18), "- line-19", "width {width}");
        }
    }

    #[test]
    fn scrolling_moves_the_detail_content() {
        let mut d = detail_dashboard(twenty_line_source(), 0, Route::Detail);
        d.apply(Action::Next);
        d.apply(Action::Next);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert_eq!(detail_marker_cols(&buf, 5), "- line-02", "width {width}");
            assert_eq!(detail_marker_cols(&buf, 18), "- line-15", "width {width}");
        }
    }

    #[test]
    fn the_list_route_still_moves_the_marker_with_detail_content_present() {
        let base = detail_dashboard(twenty_line_source(), 0, Route::List);
        let mut d = Dashboard {
            repo: base.repo,
            searched_from: base.searched_from,
            changes: fixture::set(
                vec![
                    fixture::active("add-token-refresh", 4, 9),
                    fixture::active("fix-empty-basket", 7, 7),
                    fixture::active("migrate-ai-sdk-v7", 0, 0),
                ],
                Vec::new(),
                Vec::new(),
            ),
            route: base.route,
            quit: base.quit,
            selected: base.selected,
            filter: base.filter,
            detail: base.detail,
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
        };
        d.apply(Action::Next);
        d.apply(Action::Next);
        // `list-sections`: 3, not 2 — `base.selected` starts at 1 (the active
        // header is target 0), and two `Next` presses land on target 3, the
        // third active change.
        assert_eq!(d.selected, 3);
        assert_eq!(d.selected_change().unwrap().name, "migrate-ai-sdk-v7");
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            // Row 2 is the active header; migrate-ai-sdk-v7, the third change,
            // is row 5 and correctly carries the marker.
            assert_eq!(cell(&buf, 1, 5).symbol(), ">", "width {width}");
            assert!(is_bold(cell(&buf, 1, 5)), "width {width}");
            for y in [2u16, 3, 4] {
                assert_ne!(cell(&buf, 1, y).symbol(), ">", "width {width} y={y}");
            }
        }
        // The detail content, when drawn (wide layout only), is unmoved.
        let buf120 = render_at(120, 20, &d);
        assert_eq!(detail_marker_cols(&buf120, 5), "- line-00");
    }

    #[test]
    fn scrolling_stops_at_the_top_on_screen() {
        let mut d = detail_dashboard(twenty_line_source(), 0, Route::Detail);
        for _ in 0..4 {
            d.apply(Action::Prev);
        }
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert_eq!(detail_marker_cols(&buf, 5), "- line-00", "width {width}");
        }
    }

    #[test]
    fn route_moves_reset_the_scroll_on_screen() {
        let mut d = detail_dashboard(twenty_line_source(), 3, Route::Detail);
        d.apply(Action::Back);
        d.apply(Action::OpenDetail);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert_eq!(detail_marker_cols(&buf, 5), "- line-00", "width {width}");
        }
    }

    /// Column range `1..=to` at 60, `42..=(41+to)` at 120 — the detail
    /// interior's first `to` columns of row `y`, at either mandated width.
    fn detail_interior_cols(buf: &Buffer, y: u16, to: usize) -> String {
        let from = if buf.area.width == 60 { 1 } else { 42 };
        cols(&row_text(buf, y), from..from + to)
    }

    fn empty_detail_with_tab(source: &str, problems: Vec<String>, tab: usize) -> Detail {
        Detail {
            source: source.to_string(),
            scroll: 0,
            tab,
            problems,
            loaded: None,
        }
    }

    #[test]
    fn the_header_names_the_selected_change_at_both_mandated_widths() {
        let d = dashboard_with(
            vec![
                fixture::active("add-token-refresh", 4, 9),
                fixture::active("fix-empty-basket", 7, 7),
            ],
            Vec::new(),
            // `list-sections`: 1, not 0 — target 0 is the active header.
            1,
            Route::Detail,
        );
        let progress = crate::tasks::Progress {
            completed: 4,
            total: 9,
        };
        for (width, w) in [(120, 78), (60, 58)] {
            let buf = render_at(width, 20, &d);
            let expected = crate::ui::detail::header_row("add-token-refresh", "tdd", &progress, w);
            assert_eq!(
                detail_interior_cols(&buf, 0, w as usize),
                expected,
                "width {width}"
            );
            // The wide detail region has no right gutter (D5): its header
            // reaches the frame's own last column, 119, not 118.
            let last_col = if width == 60 { 58u16 } else { 119 };
            for x in (last_col - 4)..=last_col {
                assert!(
                    cell(&buf, x, 0)
                        .style()
                        .add_modifier
                        .contains(Modifier::BOLD),
                    "width {width} x {x}"
                );
            }
        }
    }

    #[test]
    fn moving_the_selection_moves_the_header() {
        let mut d = dashboard_with(
            vec![
                fixture::active("add-token-refresh", 4, 9),
                fixture::active("fix-empty-basket", 7, 7),
            ],
            Vec::new(),
            // `list-sections`: 1, not 0 — target 0 is the active header.
            1,
            Route::Detail,
        );
        let buf_first = render_at(120, 20, &d);
        assert!(detail_interior_cols(&buf_first, 0, 78).contains("add-token-refresh"));

        // `list-sections`: 2, not 1 — target 1 is `add-token-refresh`,
        // target 2 is `fix-empty-basket`.
        d.selected = 2;
        for (width, w) in [(120, 78), (60, 58)] {
            let buf = render_at(width, 20, &d);
            let header = detail_interior_cols(&buf, 0, w);
            assert!(header.contains("fix-empty-basket"), "width {width}");
            assert!(header.contains("[7/7]"), "width {width}");
            assert!(!header.contains("add-token-refresh"), "width {width}");
        }
    }

    #[test]
    fn an_archived_change_s_header_carries_its_stripped_name_and_its_own_schema() {
        let archived_change = fixture::with_schema(
            fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
            "spec-driven",
        );
        // `list-sections`: 1, not 0 — with no active changes, target 0 is
        // the archived header.
        let d = dashboard_with(Vec::new(), vec![archived_change], 1, Route::Detail);
        for width in [120, 60] {
            let buf = render_at(width, 20, &d);
            let row = row_text(&buf, 0);
            assert!(
                row.starts_with("add-auth") || row.contains("add-auth"),
                "width {width}"
            );
            assert!(!row.contains("2026-08-14-add-auth"), "width {width}");
            assert!(row.contains("(spec-driven)"), "width {width}: {row:?}");
            assert!(row.contains("[7/7]"), "width {width}");
        }
    }

    #[test]
    fn an_empty_visible_list_leaves_the_whole_detail_interior_blank() {
        let default_style = Cell::default().style();
        let d = dashboard_with(Vec::new(), Vec::new(), 0, Route::List);
        let buf = render_at(120, 20, &d);
        // Rows 0 and 1 are the region's heading and padding rows; 2 through 18
        // are its seventeen-row interior — none of the three is drawn when
        // nothing is selected.
        for y in 0..=18u16 {
            for x in 41..=119u16 {
                assert_eq!(cell(&buf, x, y).symbol(), " ", "x={x} y={y}");
                assert_eq!(cell(&buf, x, y).style(), default_style, "x={x} y={y}");
            }
        }

        let mut zzz = dashboard_with(
            vec![fixture::active("alpha", 1, 2)],
            Vec::new(),
            0,
            Route::Detail,
        );
        zzz.filter.query = "zzz".to_string();
        for width in [120, 60] {
            let buf = render_at(width, 20, &zzz);
            for y in 0..=18u16 {
                let last = if width == 60 { 58u16 } else { 119 };
                let from = if width == 60 { 1u16 } else { 41 };
                for x in from..=last {
                    assert_eq!(cell(&buf, x, y).symbol(), " ", "width {width} x={x} y={y}");
                }
            }
        }

        // Discriminating companion: with a change selected, the region is
        // never blank. `list-sections`: `selected` **1** addresses
        // `Target::Change(0)` — `alpha` — since the active section header is
        // target 0.
        let with_change = dashboard_with(
            vec![fixture::active("alpha", 1, 2)],
            Vec::new(),
            1,
            Route::Detail,
        );
        let buf = render_at(120, 20, &with_change);
        assert!(row_text(&buf, 0).contains("alpha"));
    }

    #[test]
    fn the_tab_bar_reaches_the_buffer_at_both_mandated_widths() {
        let change = fixture::with_artifacts(
            fixture::active("detail-view", 4, 9),
            &[
                ("proposal", &[]),
                ("specs", &[]),
                ("design", &[]),
                ("tasks", &[]),
                ("planning-review", &[]),
            ],
        );
        let d = dashboard_with_detail(
            vec![change],
            Vec::new(),
            1, // `list-sections`: target 0 is the active header.
            Route::Detail,
            empty_detail_with_tab("", Vec::new(), 2),
        );
        // Five chips — 10, 7, 8, 7 and 17 columns — separated by one unpainted
        // column each: 53 columns, inside both the 78- and the 58-column
        // interior. Three spaces read between two chips: each chip's own
        // trailing and leading padding plus that one separator.
        let expected = " proposal   specs   design   tasks   planning-review ";
        assert_eq!(columns(expected), 53);
        for width in [120, 60] {
            let buf = render_at(width, 20, &d);
            // The tab bar is the detail interior's own first row — buffer
            // row 2, one row above the interior's own row 0 — not the
            // second, now that the change header sits in the region's
            // heading row instead of the interior.
            assert_eq!(detail_interior_cols(&buf, 2, 53), expected, "width {width}");
            let from = if width == 60 { 1u16 } else { 42 };
            let interior_width = if width == 60 { 58usize } else { 78 };

            // The selected chip: every one of its eight columns, its two
            // padding columns included, carries `TabActive` — bold and
            // coloured together, so neither reading alone identifies it.
            let active = palette::style(Role::TabActive);
            for x in (from + 19)..(from + 27) {
                let style = cell(&buf, x, 2).style();
                assert!(
                    style.add_modifier.contains(Modifier::BOLD),
                    "width {width} x {x}: ` design ` should be bold"
                );
                assert_eq!(style.fg, active.fg, "width {width} x {x}");
                assert_eq!(style.bg, active.bg, "width {width} x {x}");
            }

            // An inactive chip: all ten of its columns carry `TabInactive`'s
            // background and none is bold.
            let inactive = palette::style(Role::TabInactive);
            for x in from..(from + 10) {
                let style = cell(&buf, x, 2).style();
                assert!(
                    !style.add_modifier.contains(Modifier::BOLD),
                    "width {width} x {x}: ` proposal ` should not be bold"
                );
                assert_eq!(style.bg, inactive.bg, "width {width} x {x}");
            }

            // The four separating columns are painted by nothing, so two
            // adjacent chips show an edge rather than one continuous field.
            for offset in [10u16, 18, 27, 35] {
                assert_eq!(
                    cell(&buf, from + offset, 2).style().bg,
                    uncoloured().bg,
                    "width {width} offset {offset}: a separator carries a background"
                );
            }

            // Row 3 is the rule beneath the bar: `─` repeated across the
            // interior's width, dim and uncoloured.
            let rule_row = detail_interior_cols(&buf, 3, interior_width);
            assert_eq!(rule_row, "─".repeat(interior_width), "width {width}");
            for x in from..(from + interior_width as u16) {
                let style = cell(&buf, x, 3).style();
                assert!(
                    style.add_modifier.contains(Modifier::DIM),
                    "width {width} x {x}: the rule should be dim"
                );
                assert_eq!(style.fg, uncoloured().fg, "width {width} x {x}");
            }

            // Rows 1 and 4 — the region's own padding row and the content's —
            // carry no background anywhere inside the interior's columns.
            for y in [1u16, 4] {
                for x in from..(from + interior_width as u16) {
                    assert_eq!(
                        cell(&buf, x, y).style().bg,
                        uncoloured().bg,
                        "width {width} x {x} y {y}"
                    );
                }
            }
        }
    }

    /// `artifact-tabs` :: "The tab bar never overwrites a border or the rows
    /// around it" — the painted span stops inside the interior, in both
    /// directions: no gutter or divider column carries a chip background,
    /// and the heading row above the bar is untouched. `pane-chrome` removed
    /// the border: what a chip must not reach is now a gutter or the
    /// divider, and column 119 is no longer one of them — the wide detail
    /// region has no right gutter (D5).
    #[test]
    fn the_tab_bar_never_overwrites_a_border_or_the_rows_around_it() {
        let ids: Vec<String> = (0..12)
            .map(|i| format!("{}{i}", "x".repeat(40 - i.to_string().len())))
            .collect();
        let pairs: Vec<(&str, &[&str])> = ids.iter().map(|id| (id.as_str(), &[][..])).collect();
        let change = fixture::with_artifacts(fixture::active("detail-view", 4, 9), &pairs);
        let d = dashboard_with_detail(
            vec![change],
            Vec::new(),
            1, // `list-sections`: target 0 is the active header.
            Route::Detail,
            empty_detail_with_tab("", Vec::new(), 0),
        );

        let active = palette::style(Role::TabActive);
        let inactive = palette::style(Role::TabInactive);
        for width in [120, 60] {
            let buf = render_at(width, 20, &d);
            let gutter_cols: Vec<u16> = if width == 60 {
                vec![0, 59]
            } else {
                vec![0, 39, 41]
            };
            for y in 0..=18u16 {
                for x in &gutter_cols {
                    let c = cell(&buf, *x, y);
                    assert_eq!(c.symbol(), " ", "width {width} x={x} y={y}");
                    assert_ne!(c.style().bg, active.bg, "width {width} x={x} y={y}");
                    assert_ne!(c.style().bg, inactive.bg, "width {width} x={x} y={y}");
                }
            }
            // The region's heading row still holds the change's own name, so
            // no chip wrapped upward into it.
            assert!(
                detail_interior_cols(&buf, 0, 11).starts_with("detail-view"),
                "width {width}"
            );
        }
    }

    #[test]
    fn a_select_tab_at_route_list_is_visible_in_the_tab_row_at_120() {
        let change = fixture::with_artifacts(
            fixture::active("detail-view", 4, 9),
            &[("proposal", &[]), ("specs", &[]), ("design", &[])],
        );
        let mut d = dashboard_with_detail(
            vec![change],
            Vec::new(),
            1, // `list-sections`: target 0 is the active header.
            Route::List,
            empty_detail_with_tab("", Vec::new(), 0),
        );
        d.apply(Action::SelectTab(2));
        let buf = render_at(120, 20, &d);
        assert!(row_text(&buf, 2).contains(" design "));
        // The third chip — ` design `, eight columns from interior offset 19,
        // after ` proposal ` (10) and ` specs ` (7) and their two separators —
        // carries the active style, which is what makes the list-route press
        // visible at the wide layout.
        let active = palette::style(Role::TabActive);
        for x in 42 + 19..42 + 19 + 8 {
            let style = cell(&buf, x, 2).style();
            assert!(style.add_modifier.contains(Modifier::BOLD), "x {x}");
            assert_eq!(style.fg, active.fg, "x {x}");
            assert_eq!(style.bg, active.bg, "x {x}");
        }
        // Discriminating companion, naming the check's other mandated
        // width: at 60, Route::List, the narrow layout draws only the list
        // region — no detail region, and so no tab row at all.
        let buf60 = render_at(60, 20, &d);
        assert!(!row_text(&buf60, 2).contains(" design "));
    }

    #[test]
    fn the_bar_the_rule_and_the_content_never_leave_the_interior() {
        let ids: Vec<String> = (0..12)
            .map(|i| format!("{}{i}", "x".repeat(40 - i.to_string().len())))
            .collect();
        let pairs: Vec<(&str, &[&str])> = ids.iter().map(|id| (id.as_str(), &[][..])).collect();
        let change = fixture::with_artifacts(fixture::active("detail-view", 4, 9), &pairs);
        let source: String = (0..30).map(|_| format!("{}\n", "y".repeat(200))).collect();
        let d = dashboard_with_detail(
            vec![change],
            Vec::new(),
            1, // `list-sections`: target 0 is the active header.
            Route::Detail,
            empty_detail_with_tab(&source, Vec::new(), 0),
        );
        // `pane-chrome` removed the border entirely: the columns that used to
        // carry box-drawing glyphs are now the region's own gutters — plain
        // spaces — with a lone `│` divider at column 40 in the wide layout.
        let buf60 = render_at(60, 20, &d);
        for y in 0..=18u16 {
            assert_eq!(cell(&buf60, 0, y).symbol(), " ", "y={y}");
            assert_eq!(cell(&buf60, 59, y).symbol(), " ", "y={y}");
        }
        let buf120 = render_at(120, 20, &d);
        for y in 0..=18u16 {
            for x in [0u16, 39, 41] {
                assert_eq!(cell(&buf120, x, y).symbol(), " ", "x={x} y={y}");
                assert_ne!(cell(&buf120, x, y).symbol(), "─", "x={x} y={y}");
            }
            assert_eq!(cell(&buf120, 40, y).symbol(), "│", "y={y}");
        }
        // Neither the gutter columns nor the divider ever carry a chip
        // background: the twelve wide chips and the thirty 200-character
        // lines both stopped inside the interior.
        let active = palette::style(Role::TabActive);
        let inactive = palette::style(Role::TabInactive);
        for buf in [&buf60, &buf120] {
            let gutters: Vec<u16> = if buf.area.width == 60 {
                vec![0, 59]
            } else {
                vec![0, 39, 40, 41]
            };
            for y in 0..=18u16 {
                for x in &gutters {
                    let bg = cell(buf, *x, y).style().bg;
                    assert_ne!(bg, active.bg, "x={x} y={y}");
                    assert_ne!(bg, inactive.bg, "x={x} y={y}");
                }
            }
        }
        // The region's heading row still holds the change's own name, so no
        // chip and no content line wrapped upward into it.
        assert!(detail_interior_cols(&buf60, 0, 11).starts_with("detail-view"));
        assert!(detail_interior_cols(&buf120, 0, 11).starts_with("detail-view"));
    }

    #[test]
    fn a_missing_artifact_still_shows_its_tab_and_reads_no_content_yet() {
        let change = fixture::with_artifacts(
            fixture::active("detail-view", 4, 9),
            &[
                ("proposal", &[]),
                ("specs", &[]),
                ("design", &[]),
                ("tasks", &[]),
                ("planning-review", &[]),
            ],
        );
        let d = dashboard_with_detail(
            vec![change],
            Vec::new(),
            1, // `list-sections`: target 0 is the active header.
            Route::Detail,
            empty_detail_with_tab("", Vec::new(), 1),
        );
        for width in [120, 60] {
            let buf = render_at(width, 20, &d);
            assert!(
                detail_interior_cols(&buf, 5, 14).starts_with("No content yet"),
                "width {width}"
            );
            assert!(
                row_text(&buf, 2).contains(" specs "),
                "width {width}: tab bar still intact"
            );
            // `artifact-content` :: "in both buffers that row measures exactly the
            // interior width ... because the literal is now padded like every line
            // around it" — pinned at the view tier, alongside the unit-tier assertion
            // in `ui::detail`'s own tests.
            let interior = interior_width(width) as usize;
            assert_eq!(
                columns(&detail_interior_cols(&buf, 5, interior)),
                interior,
                "width {width}: the No content yet row is padded to the interior width"
            );
        }

        // The same holds at the tracked-tasks position (3): a missing
        // tasks artifact reads `No content yet` and shows no progress
        // bar, even though the change's `progress` is `[4/9]` one row up.
        let marked = fixture::track_tasks_at(
            fixture::with_artifacts(
                fixture::active("detail-view", 4, 9),
                &[
                    ("proposal", &[]),
                    ("specs", &[]),
                    ("design", &[]),
                    ("tasks", &[]),
                    ("planning-review", &[]),
                ],
            ),
            3,
        );
        let d = dashboard_with_detail(
            vec![marked],
            Vec::new(),
            1, // `list-sections`: target 0 is the active header.
            Route::Detail,
            empty_detail_with_tab("", Vec::new(), 3),
        );
        for width in [120, 60] {
            let buf = render_at(width, 20, &d);
            assert!(
                detail_interior_cols(&buf, 5, 14).starts_with("No content yet"),
                "width {width}: marked tab"
            );
            assert!(
                !row_text(&buf, 5).contains('█') && !row_text(&buf, 5).contains('░'),
                "width {width}: no progress bar for a marked tab with no file"
            );
            // `artifact-content` :: the same "measures exactly the interior width"
            // clause, for the tracked-tasks position.
            let interior = interior_width(width) as usize;
            assert_eq!(
                columns(&detail_interior_cols(&buf, 5, interior)),
                interior,
                "width {width}: marked tab: the No content yet row is padded to the \
                 interior width"
            );
        }
    }

    #[test]
    fn a_read_failure_is_named_above_the_content_at_both_widths() {
        let change =
            fixture::with_artifacts(fixture::active("detail-view", 4, 9), &[("proposal", &[])]);
        let d = dashboard_with_detail(
            vec![change],
            Vec::new(),
            1, // `list-sections`: target 0 is the active header.
            Route::Detail,
            empty_detail_with_tab(
                "# b\n",
                vec!["/repo/specs/a/spec.md: permission denied".to_string()],
                0,
            ),
        );
        for width in [120, 60] {
            let buf = render_at(width, 20, &d);
            assert!(
                detail_interior_cols(&buf, 5, 24).starts_with("! /repo/specs/a/spec.md:"),
                "width {width}"
            );
            assert!(
                detail_interior_cols(&buf, 6, 3).starts_with("# b"),
                "width {width}"
            );
            assert!(
                !row_text(&buf, 5).contains("No content yet"),
                "width {width}"
            );
        }
    }

    #[test]
    fn the_twenty_item_list_fills_the_content_area_rows_5_through_18() {
        let change =
            fixture::with_artifacts(fixture::active("detail-view", 4, 9), &[("proposal", &[])]);
        let d = dashboard_with_detail(
            vec![change],
            Vec::new(),
            1, // `list-sections`: target 0 is the active header.
            Route::Detail,
            empty_detail_with_tab(&twenty_line_source(), Vec::new(), 0),
        );
        for width in [120, 60] {
            let buf = render_at(width, 20, &d);
            assert_eq!(
                detail_interior_cols(&buf, 5, 9),
                "- line-00",
                "width {width}"
            );
            assert_eq!(
                detail_interior_cols(&buf, 18, 9),
                "- line-13",
                "width {width}"
            );
        }
    }

    // --- group 7: the rendered buffer ----------------------------------

    /// The repeated "build a dashboard with a marked tab and this source"
    /// setup, folded into one helper — it appeared more than three times
    /// among this group's new tests. Reaches
    /// `changes::fixture::with_marked_artifacts` for the change value,
    /// never a literal of its own type or `ArtifactRef`'s, and never a
    /// local helper whose own return type names that type — Change Review
    /// found three such helpers tripping `NOLIT-CHANGE` on their
    /// return-type signature alone.
    fn dashboard_with_marked_change(
        ids: &[&str],
        marked: Option<usize>,
        progress: crate::tasks::Progress,
        source: &str,
        problems: Vec<String>,
        tab: usize,
    ) -> Dashboard {
        let pairs: Vec<(&str, &[&str])> = ids.iter().map(|id| (*id, &[][..])).collect();
        let change = fixture::with_marked_artifacts(&pairs, marked, progress);
        dashboard_with_detail(
            vec![change],
            Vec::new(),
            1, // `list-sections`: target 0 is the active header.
            Route::Detail,
            empty_detail_with_tab(source, problems, tab),
        )
    }

    fn interior_width(width: u16) -> u16 {
        if width == 120 { 78 } else { 58 }
    }

    #[test]
    fn tasks_tab_shows_checkboxes() {
        let progress = crate::tasks::Progress {
            completed: 1,
            total: 3,
        };
        let ids = ["proposal", "specs", "design", "tasks", "planning-review"];
        let source = "## 1. Setup\n\n- [x] 1.1 first\n- [ ] 1.2 second\n";

        for width in [120, 60] {
            let d3 = dashboard_with_marked_change(&ids, Some(3), progress, source, Vec::new(), 3);
            let buf3 = render_at(width, 20, &d3);
            let bar = crate::ui::tasks::progress_bar(&progress, interior_width(width));
            assert_eq!(
                detail_interior_cols(&buf3, 5, columns(&bar)),
                bar,
                "width {width}"
            );
            assert_eq!(
                detail_interior_cols(&buf3, 6, interior_width(width) as usize).trim(),
                "",
                "width {width}: blank row"
            );
            assert_eq!(
                detail_interior_cols(&buf3, 7, 11),
                "## 1. Setup",
                "width {width}"
            );
            assert_eq!(
                detail_interior_cols(&buf3, 8, 13),
                "[x] 1.1 first",
                "width {width}"
            );
            assert_eq!(
                detail_interior_cols(&buf3, 9, 14),
                "[ ] 1.2 second",
                "width {width}"
            );

            let d0 = dashboard_with_marked_change(&ids, Some(3), progress, source, Vec::new(), 0);
            let buf0 = render_at(width, 20, &d0);
            assert_eq!(
                detail_interior_cols(&buf0, 5, 11),
                "## 1. Setup",
                "width {width}: no bar row above it"
            );
            assert_eq!(
                detail_interior_cols(&buf0, 7, 15),
                "- [x] 1.1 first",
                "width {width}: source bullet intact"
            );
            assert_eq!(
                detail_interior_cols(&buf0, 8, 16),
                "- [ ] 1.2 second",
                "width {width}: source bullet intact"
            );

            assert_eq!(
                row_text(&buf3, 2),
                row_text(&buf0, 2),
                "width {width}: tab bar byte-identical between the two renders"
            );
        }
    }

    #[test]
    fn tasks_tab_chosen_by_flag() {
        let progress = crate::tasks::Progress {
            completed: 1,
            total: 1,
        };
        let ids = ["checklist", "tasks"];
        let source = "- [x] done\n";

        for width in [120, 60] {
            let d0 = dashboard_with_marked_change(&ids, Some(0), progress, source, Vec::new(), 0);
            let buf0 = render_at(width, 20, &d0);
            let bar = crate::ui::tasks::progress_bar(&progress, interior_width(width));
            assert_eq!(
                detail_interior_cols(&buf0, 5, columns(&bar)),
                bar,
                "width {width}: id checklist carries the flag"
            );
            assert_eq!(
                detail_interior_cols(&buf0, 7, 8),
                "[x] done",
                "width {width}"
            );

            let d1 = dashboard_with_marked_change(&ids, Some(0), progress, source, Vec::new(), 1);
            let buf1 = render_at(width, 20, &d1);
            assert_eq!(
                detail_interior_cols(&buf1, 5, 10),
                "- [x] done",
                "width {width}: id tasks does not carry the flag"
            );
            assert_ne!(
                detail_interior_cols(&buf1, 5, columns(&bar)),
                bar,
                "width {width}"
            );

            assert!(row_text(&buf0, 2).contains(" checklist "), "width {width}");
            assert!(row_text(&buf0, 2).contains(" tasks "), "width {width}");
        }
    }

    #[test]
    fn no_marked_artifact_renders_markdown() {
        let progress = crate::tasks::Progress {
            completed: 0,
            total: 0,
        };
        let ids = ["alpha", "beta", "gamma"];
        let source = "- [ ] a\n";

        for width in [120, 60] {
            for tab in [0usize, 1, 2] {
                let d = dashboard_with_marked_change(&ids, None, progress, source, Vec::new(), tab);
                let buf = render_at(width, 20, &d);
                assert!(
                    !row_text(&buf, 5).contains('█') && !row_text(&buf, 5).contains('░'),
                    "width {width} tab {tab}: no progress-bar row"
                );
                assert_eq!(
                    detail_interior_cols(&buf, 5, 7),
                    "- [ ] a",
                    "width {width} tab {tab}"
                );
                assert!(
                    row_text(&buf, 2).contains(" alpha ")
                        && row_text(&buf, 2).contains(" beta ")
                        && row_text(&buf, 2).contains(" gamma "),
                    "width {width} tab {tab}: no tab removed"
                );
            }
        }
    }

    #[test]
    fn prose_only_reads_no_tasks_yet() {
        let progress = crate::tasks::Progress {
            completed: 0,
            total: 0,
        };
        let d = dashboard_with_marked_change(
            &["proposal", "tasks"],
            Some(1),
            progress,
            "# Plan\n\nNothing checkable here.\n",
            Vec::new(),
            1,
        );
        for width in [120, 60] {
            let buf = render_at(width, 20, &d);
            assert_eq!(detail_interior_cols(&buf, 5, 3), "[-]", "width {width}");
            assert_eq!(
                detail_interior_cols(&buf, 7, 12),
                "No tasks yet",
                "width {width}"
            );
            assert!(!buffer_contains(&buf, "No content yet"), "width {width}");
            assert!(!buffer_contains(&buf, "# Plan"), "width {width}");
        }
    }

    #[test]
    fn missing_tasks_artifact_no_content_yet() {
        let progress = crate::tasks::Progress {
            completed: 4,
            total: 9,
        };
        let d = dashboard_with_marked_change(
            &["proposal", "tasks"],
            Some(1),
            progress,
            "",
            Vec::new(),
            1,
        );
        for width in [120, 60] {
            let buf = render_at(width, 20, &d);
            assert_eq!(
                detail_interior_cols(&buf, 5, 14),
                "No content yet",
                "width {width}"
            );
            assert!(!buffer_contains(&buf, "No tasks yet"), "width {width}");
            assert!(
                !row_text(&buf, 5).contains('█') && !row_text(&buf, 5).contains('░'),
                "width {width}: no progress-bar row"
            );
        }
    }

    #[test]
    fn tasks_tab_read_failure() {
        let progress = crate::tasks::Progress {
            completed: 1,
            total: 1,
        };
        let d = dashboard_with_marked_change(
            &["proposal", "tasks"],
            Some(1),
            progress,
            "",
            vec!["/repo/openspec/changes/x/tasks.md: permission denied".to_string()],
            1,
        );
        for width in [120, 60] {
            let buf = render_at(width, 20, &d);
            // The problem row is padded to the interior's own width and
            // stops exactly there: the last interior column is still part
            // of the padded row (a space, not a border character), and
            // the border column one past it is a box-drawing character —
            // `detail_interior_cols(buf, y, interior).len() == interior`
            // alone cannot fail (`cols` always returns exactly the length
            // asked for), so this checks the buffer directly instead.
            // `pane-chrome`: there is no border any more to survive one column
            // past the interior. At width 60 the narrow region keeps its
            // trailing gutter — one column past the interior — untouched; the
            // wide detail region has no right gutter at all (D5), so there is
            // no further column to check there.
            if width == 60 {
                assert_eq!(
                    cell(&buf, 59, 5).symbol(),
                    " ",
                    "width {width}: the region's trailing gutter is untouched"
                );
            }
            assert!(
                detail_interior_cols(&buf, 5, 24).starts_with("! /repo/openspec/changes"),
                "width {width}"
            );
            assert!(!buffer_contains(&buf, "No content yet"), "width {width}");
            assert!(!buffer_contains(&buf, "No tasks yet"), "width {width}");
            assert!(
                !row_text(&buf, 5).contains('█') && !row_text(&buf, 5).contains('░'),
                "width {width}: no progress-bar row"
            );
        }
    }

    #[test]
    fn progress_bar_in_the_buffer() {
        let progress = crate::tasks::Progress {
            completed: 4,
            total: 9,
        };
        let source: String = (0..9).map(|i| format!("- [ ] t{i}\n")).collect();
        let d = dashboard_with_marked_change(
            &["proposal", "tasks"],
            Some(1),
            progress,
            &source,
            Vec::new(),
            1,
        );

        let buf120 = render_at(120, 20, &d);
        assert_eq!(
            cols(&row_text(&buf120, 5), 42..120),
            crate::ui::tasks::progress_bar(&progress, 78),
        );
        assert!(cols(&row_text(&buf120, 5), 42..120).ends_with("[4/9] 44%"));

        let buf60 = render_at(60, 20, &d);
        assert_eq!(
            cols(&row_text(&buf60, 5), 1..59),
            crate::ui::tasks::progress_bar(&progress, 58),
        );
        assert!(cols(&row_text(&buf60, 5), 1..59).ends_with("[4/9] 44%"));

        for buf in [&buf120, &buf60] {
            let iw = interior_width(buf.area.width) as usize;
            assert!(detail_interior_cols(buf, 6, iw).trim().is_empty());
            assert!(buffer_contains(buf, "[ ] t0"));
        }
    }

    #[test]
    fn no_tasks_bar_in_the_buffer() {
        let progress = crate::tasks::Progress {
            completed: 0,
            total: 0,
        };
        let d = dashboard_with_marked_change(
            &["proposal", "tasks"],
            Some(1),
            progress,
            "# Plan\n\nprose only\n",
            Vec::new(),
            1,
        );

        let buf120 = render_at(120, 20, &d);
        assert_eq!(cols(&row_text(&buf120, 5), 42..45), "[-]");
        assert_eq!(cell(&buf120, 45, 5).symbol(), " ");

        let buf60 = render_at(60, 20, &d);
        assert_eq!(cols(&row_text(&buf60, 5), 1..4), "[-]");

        for buf in [&buf120, &buf60] {
            assert_eq!(
                detail_interior_cols(buf, 7, 12),
                "No tasks yet",
                "row 7 reads No tasks yet"
            );
        }
    }

    #[test]
    fn marked_tab_renders_checklist_body() {
        let progress = crate::tasks::Progress {
            completed: 0,
            total: 0,
        };
        let d_unmarked = dashboard_with_marked_change(
            &["proposal"],
            None,
            progress,
            &twenty_line_source(),
            Vec::new(),
            0,
        );
        let d_marked = dashboard_with_marked_change(
            &["proposal"],
            Some(0),
            progress,
            &twenty_line_source(),
            Vec::new(),
            0,
        );

        for width in [120, 60] {
            let buf_unmarked = render_at(width, 20, &d_unmarked);
            let buf_marked = render_at(width, 20, &d_marked);

            assert_eq!(
                detail_interior_cols(&buf_marked, 5, 3),
                "[-]",
                "width {width}"
            );
            assert!(
                detail_interior_cols(&buf_marked, 6, interior_width(width) as usize)
                    .trim()
                    .is_empty(),
                "width {width}"
            );
            assert_eq!(
                detail_interior_cols(&buf_marked, 7, 12),
                "No tasks yet",
                "width {width}: the twenty bullets hold no task lines"
            );

            assert_eq!(
                row_text(&buf_unmarked, 0),
                row_text(&buf_marked, 0),
                "width {width}: header row byte-identical"
            );
            assert_eq!(
                row_text(&buf_unmarked, 2),
                row_text(&buf_marked, 2),
                "width {width}: tab row byte-identical"
            );
        }
    }

    /// `agent-polling`: nothing renders `Dashboard.agents` in this change — the same
    /// `Dashboard` rendered with three different `agents` snapshots must draw the exact
    /// same buffer, cell for cell, at both mandated widths. `SPEC.md` -> Degraded states
    /// records an unreachable socket as "runs as a standalone TUI", which is silence,
    /// not a message, so even a long `problem` string must appear nowhere on screen.
    #[test]
    fn agents_change_no_pixel() {
        for width in [120, 60] {
            let base = dashboard_with(
                vec![fixture::active("alpha", 4, 9)],
                Vec::new(),
                0,
                Route::List,
            );

            // `base` already carries `ui::load`'s own startup default: no agents,
            // unreachable, no problem — this is the "initial" snapshot.
            let initial = base.clone();

            let mut reachable = base.clone();
            reachable.agents = crate::agents::AgentSnapshot {
                agents: vec![
                    crate::agents::Agent {
                        name: Some("agent-polling".to_string()),
                        kind: Some("claude".to_string()),
                        status: crate::agents::AgentStatus::Working,
                        cwd: Some(std::path::PathBuf::from("/repo")),
                        pane_id: "w8:p1".to_string(),
                        tab_id: "w8:t1".to_string(),
                        workspace_id: "w8".to_string(),
                        terminal_title: None,
                    },
                    crate::agents::Agent {
                        name: None,
                        kind: Some("claude".to_string()),
                        status: crate::agents::AgentStatus::Idle,
                        cwd: None,
                        pane_id: "w8:p2".to_string(),
                        tab_id: "w8:t2".to_string(),
                        workspace_id: "w8".to_string(),
                        terminal_title: None,
                    },
                ],
                // `agent-launch`: held at `false` here, like every fixture in this test —
                // `reachable` now moves the footer, so comparing a reachable buffer against
                stalled: false,
                // an unreachable one would say nothing about scope.
                reachable: false,
                problem: None,
            };

            let mut unreachable = base.clone();
            unreachable.agents = crate::agents::AgentSnapshot {
                agents: Vec::new(),
                reachable: false,
                stalled: false,
                problem: Some(
                    "herdr agent list: could not start herdr: No such file or directory \
                     (os error 2) — a deliberately long reason, long enough to overflow \
                     any reasonable terminal width if it were ever rendered anywhere"
                        .to_string(),
                ),
            };

            let buf_initial = render_at(width, 20, &initial);
            let buf_reachable = render_at(width, 20, &reachable);
            let buf_unreachable = render_at(width, 20, &unreachable);

            assert_eq!(
                buf_initial, buf_reachable,
                "width {width}: a reachable snapshot must change no pixel"
            );
            assert_eq!(
                buf_initial, buf_unreachable,
                "width {width}: an unreachable snapshot, even with a long problem, must \
                 change no pixel"
            );

            // `agent-attribution`: an agent with no `cwd` at all is out of scope, on
            // exactly the same terms as one outside the repository root.
            let mut absent_cwd = base.clone();
            absent_cwd.agents = crate::agents::AgentSnapshot {
                agents: vec![crate::agents::Agent {
                    name: Some("agent-polling".to_string()),
                    kind: Some("claude".to_string()),
                    status: crate::agents::AgentStatus::Working,
                    cwd: None,
                    pane_id: "w8:p3".to_string(),
                    tab_id: "w8:t3".to_string(),
                    workspace_id: "w8".to_string(),
                    terminal_title: None,
                }],
                reachable: false,
                stalled: false,
                problem: None,
            };
            let buf_absent_cwd = render_at(width, 20, &absent_cwd);
            assert_eq!(
                buf_initial, buf_absent_cwd,
                "width {width}: an agent with no cwd at all must change no pixel"
            );

            // Discriminating control: moving one agent INTO the repository root and
            // naming it after the change on screen must change a pixel — the identity
            // above is a scope branch, not a constant.
            let mut moved_into_repo = reachable.clone();
            moved_into_repo.agents.agents[0].cwd = Some(std::path::PathBuf::from("/tmp/demo-repo"));
            moved_into_repo.agents.agents[0].name = Some("alpha".to_string());
            let buf_moved = render_at(width, 20, &moved_into_repo);
            assert_ne!(
                buf_initial, buf_moved,
                "width {width}: an in-scope, matched agent must change a pixel"
            );

            // `agent-launch`'s own discriminating control: flipping `reachable` to `true` on
            // the first fixture alone must change a pixel too — in the footer and only the
            // footer — which is exactly why the four fixtures above hold it constant.
            let mut reachable_flag_only = initial.clone();
            reachable_flag_only.agents.reachable = true;
            let buf_reachable_flag = render_at(width, 20, &reachable_flag_only);
            assert_ne!(
                buf_initial, buf_reachable_flag,
                "width {width}: flipping reachable alone must change the footer"
            );
            for y in 0..=17u16 {
                assert_eq!(
                    row_text(&buf_initial, y),
                    row_text(&buf_reachable_flag, y),
                    "width {width} row {y}: only the footer may differ"
                );
            }
        }
    }

    // --- agent-launch: the action hints -----------------------------------------------

    /// `responsive-layout`: "The action hints follow `Esc back` when the socket is
    /// reachable."
    #[test]
    fn the_action_hints_follow_esc_back_when_reachable() {
        let mut d = dashboard(Some("/tmp/demo-repo"), Route::List);
        d.agents.reachable = true;
        let expected = "q quit  Enter detail  Esc back  a/c/s launch  g focus";
        assert_eq!(columns(expected), 53);
        let buf60 = render_at(60, 20, &d);
        assert_eq!(row_text(&buf60, 19), format!("{expected}{}", " ".repeat(7)));
        let buf120 = render_at(120, 20, &d);
        assert_eq!(
            row_text(&buf120, 19),
            format!("{expected}{}", " ".repeat(67))
        );

        let mut unreachable = d.clone();
        unreachable.agents.reachable = false;
        assert_eq!(
            row_text(&render_at(60, 20, &unreachable), 19),
            format!("q quit  Enter detail  Esc back{}", " ".repeat(30)),
            "byte-identical to the row this capability specified before the action hints existed"
        );

        for width in [60u16, 120u16] {
            let reachable_buf = render_at(width, 20, &d);
            let unreachable_buf = render_at(width, 20, &unreachable);
            for y in 0..=18u16 {
                assert_eq!(
                    row_text(&reachable_buf, y),
                    row_text(&unreachable_buf, y),
                    "width {width} row {y}: the reachability flag moves the footer and nothing else"
                );
            }
        }
    }

    /// `responsive-layout`: "The action hints are dropped whole, `g focus` first."
    #[test]
    fn the_action_hints_are_dropped_whole_g_focus_first() {
        let mut d = dashboard(Some("/tmp/demo-repo"), Route::List);
        d.agents.reachable = true;

        let buf53 = render_at(53, 20, &d);
        assert_eq!(
            row_text(&buf53, 19),
            "q quit  Enter detail  Esc back  a/c/s launch  g focus"
        );

        let buf52 = render_at(52, 20, &d);
        assert_eq!(
            row_text(&buf52, 19),
            format!(
                "q quit  Enter detail  Esc back  a/c/s launch{}",
                " ".repeat(8)
            ),
            "g focus and its separator need nine columns and only eight remain"
        );

        let buf44 = render_at(44, 20, &d);
        assert_eq!(
            row_text(&buf44, 19),
            "q quit  Enter detail  Esc back  a/c/s launch"
        );

        let buf43 = render_at(43, 20, &d);
        assert_eq!(
            row_text(&buf43, 19),
            format!("q quit  Enter detail  Esc back{}", " ".repeat(13)),
            "both action hints are dropped before any of the three key hints"
        );

        // 60 and 120 as contrasting controls: comfortably wide enough that both hints are
        // never dropped there.
        for width in [60, 120] {
            let row = row_text(&render_at(width, 20, &d), 19);
            assert!(row.contains("a/c/s launch"), "width {width}");
            assert!(row.contains("g focus"), "width {width}");
        }
    }

    /// `agent-launch`: "An unreachable socket hides both hints at both widths."
    #[test]
    fn an_unreachable_socket_hides_both_hints() {
        let d = dashboard(Some("/tmp/demo-repo"), Route::List);
        assert!(!d.agents.reachable);
        for width in [60, 120] {
            let row = row_text(&render_at(width, 20, &d), 19);
            assert!(
                row.starts_with("q quit  Enter detail  Esc back"),
                "width {width}"
            );
            assert!(!row.contains("a/c/s launch"), "width {width}");
            assert!(!row.contains("g focus"), "width {width}");
        }
    }

    /// `agent-launch`: "The count is dropped before the action hints as the width falls."
    #[test]
    fn the_count_is_dropped_before_the_action_hints() {
        let mut d = dashboard(Some("/tmp/demo-repo"), Route::List);
        d.agents.reachable = true;
        d.agents.agents = vec![unattributed_agent("nothing-like-a-change")];

        let buf120 = row_text(&render_at(120, 20, &d), 19);
        assert!(
            buf120.starts_with(
                "q quit  Enter detail  Esc back  a/c/s launch  g focus  1 unattributed"
            )
        );

        let buf60 = render_at(60, 20, &d);
        assert_eq!(
            row_text(&buf60, 19),
            format!(
                "q quit  Enter detail  Esc back  a/c/s launch  g focus{}",
                " ".repeat(7)
            ),
            "the count is 69 columns in and does not fit; fit_hints drops it whole"
        );

        let mut unreachable = d.clone();
        unreachable.agents.reachable = false;
        assert!(
            row_text(&render_at(60, 20, &unreachable), 19).contains("1 unattributed"),
            "with the two hints absent, the count reappears at 60"
        );
    }

    /// `dashboard-loop`: "The action keys type into the query while filtering."
    #[test]
    fn the_action_hints_survive_a_filter() {
        let mut d = dashboard(Some("/tmp/demo-repo"), Route::List);
        d.agents.reachable = true;
        d.filter = Filter {
            query: "acsg".to_string(),
            active: true,
        };
        for width in [60, 120] {
            let footer = row_text(&render_at(width, 20, &d), 19);
            assert!(footer.starts_with("/acsg_"), "width {width}: {footer:?}");
            assert!(!footer.contains("a/c/s launch"), "width {width}");
            assert!(!footer.contains("g focus"), "width {width}");
        }
    }

    /// `agent-launch`: "A launch problem leads the refresh and change-set problems", at the
    /// view tier.
    #[test]
    fn a_launch_problem_renders_above_a_watch_problem() {
        let mut d = dashboard_with(
            vec![fixture::active("alpha", 1, 2)],
            Vec::new(),
            0,
            Route::List,
        );
        d.refresh.problems = vec!["watch failed".to_string()];
        d.launch.problems = vec!["launch failed".to_string()];
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(
                interior_cols(&buf, 2).contains("launch failed"),
                "width {width}"
            );
            assert!(
                interior_cols(&buf, 3).contains("watch failed"),
                "width {width}"
            );
        }
    }

    // `color-palette` — every colour below is asserted against `palette::style(role)`
    // rather than a literal: this file is inside the confinement gate's search set, and
    // the literal table is asserted once, in `ui::palette`'s own tests (design.md ->
    // Decision 2).

    /// `view-palette` :: "Faces reach the buffer as coloured styles at both mandated
    /// widths".
    #[test]
    fn faces_reach_the_buffer_as_coloured_styles() {
        let source = "# Title\n\n## Heading\n\n**bold** and *italic* and `code` and [link](u) and \
             ~~struck~~\n";
        let d = detail_dashboard(source.to_string(), 0, Route::Detail);
        for width in [120, 60] {
            let buf = render_at(width, 20, &d);

            let title = find_cell_style(&buf, "# Title");
            assert!(
                title.add_modifier.contains(Modifier::BOLD),
                "width {width}: the level-1 heading is not bold"
            );
            assert_eq!(
                title.fg,
                palette::style(Role::Heading(1)).fg,
                "width {width}: the level-1 heading does not carry Heading(1)'s foreground"
            );

            let heading = find_cell_style(&buf, "## Heading");
            assert!(
                heading.add_modifier.contains(Modifier::BOLD),
                "width {width}: the level-2 heading is not bold"
            );
            assert_eq!(
                heading.fg,
                palette::style(Role::Heading(2)).fg,
                "width {width}: the level-2 heading does not carry Heading(2)'s foreground"
            );

            let code = find_cell_style(&buf, "code");
            assert!(
                code.add_modifier.contains(Modifier::DIM),
                "width {width}: the code span is not dim"
            );
            assert_eq!(
                code.fg,
                palette::style(Role::Code).fg,
                "width {width}: the code span does not carry Code's foreground"
            );

            let link = find_cell_style(&buf, "link");
            assert!(
                link.add_modifier.contains(Modifier::UNDERLINED),
                "width {width}: the link is not underlined"
            );
            assert_eq!(
                link.fg,
                palette::style(Role::Link).fg,
                "width {width}: the link does not carry Link's foreground"
            );

            // The uncoloured faces are discriminated from the coloured ones.
            let bold = find_cell_style(&buf, "bold");
            assert!(bold.add_modifier.contains(Modifier::BOLD), "width {width}");
            assert_eq!(
                bold.fg,
                uncoloured().fg,
                "width {width}: bold must carry no foreground"
            );
            let italic = find_cell_style(&buf, "italic");
            assert!(
                italic.add_modifier.contains(Modifier::ITALIC),
                "width {width}"
            );
            assert_eq!(
                italic.fg,
                uncoloured().fg,
                "width {width}: italic must carry no foreground"
            );
            let struck = find_cell_style(&buf, "struck");
            assert!(
                struck.add_modifier.contains(Modifier::CROSSED_OUT),
                "width {width}"
            );
            assert_eq!(
                struck.fg,
                uncoloured().fg,
                "width {width}: struck must carry no foreground"
            );

            // The heading assertions discriminate: the two levels are different colours.
            assert_ne!(title.fg, heading.fg, "width {width}");
        }
    }

    /// `view-palette` :: "Heading foreground wins over a code span inside it".
    #[test]
    fn heading_foreground_wins_over_a_code_span_inside_it() {
        // A pure `style_for` test: no frame is drawn here, so the two mandated widths are
        // named rather than exercised — the 60- and 120-column render half of this same
        // rule is `faces_reach_the_buffer_as_coloured_styles`.
        let heading_code = Face {
            heading: Some(2),
            code: true,
            ..Face::plain()
        };
        let style = style_for(&heading_code);
        assert_eq!(
            style.fg,
            palette::style(Role::Heading(2)).fg,
            "the heading's foreground must win over the code span's"
        );
        assert!(
            style.add_modifier.contains(Modifier::BOLD),
            "the heading's BOLD must survive the fold"
        );
        assert!(
            style.add_modifier.contains(Modifier::DIM),
            "the code span's DIM must survive the fold"
        );

        let code_link = Face {
            code: true,
            link: true,
            ..Face::plain()
        };
        let style = style_for(&code_link);
        assert_eq!(
            style.fg,
            palette::style(Role::Code).fg,
            "code follows link in the fold order, so its foreground wins"
        );
        assert!(style.add_modifier.contains(Modifier::DIM));
        assert!(style.add_modifier.contains(Modifier::UNDERLINED));

        // `markdown-constructs`: an uncoloured strikethrough neither loses its own
        // modifier nor displaces the link's colour, which is why it is folded second
        // rather than into the coloured precedence chain.
        let struck_bold_link = Face {
            strikethrough: true,
            strong: true,
            link: true,
            ..Face::plain()
        };
        let style = style_for(&struck_bold_link);
        assert!(style.add_modifier.contains(Modifier::CROSSED_OUT));
        assert!(style.add_modifier.contains(Modifier::BOLD));
        assert!(style.add_modifier.contains(Modifier::UNDERLINED));
        assert_eq!(
            style.fg,
            palette::style(Role::Link).fg,
            "an uncoloured strikethrough must not displace the link's foreground"
        );

        // The assertions discriminate rather than comparing one colour with itself.
        assert_ne!(
            palette::style(Role::Heading(2)).fg,
            palette::style(Role::Code).fg
        );
        assert_ne!(palette::style(Role::Code).fg, palette::style(Role::Link).fg);
    }

    /// `view-palette` :: "A plain face is the default style".
    #[test]
    fn a_plain_face_is_the_default_style() {
        assert_eq!(style_for(&Face::plain()), Style::default());
        assert!(
            !Face::plain().strikethrough,
            "the new field must not change what a plain face maps to"
        );

        let d = detail_dashboard("plain text only\n".to_string(), 0, Route::Detail);
        let default_style = Cell::default().style();
        for (width, first, last) in [(120, 42, 119), (60, 1, 58)] {
            let buf = render_at(width, 20, &d);
            assert!(
                row_text(&buf, 5).contains("plain text only"),
                "width {width}: the document was not drawn"
            );
            for y in 5..=18u16 {
                for x in first..=last {
                    assert_eq!(
                        cell(&buf, x, y).style(),
                        default_style,
                        "width {width}: content cell {x},{y} is not the default style"
                    );
                }
            }
        }
    }

    /// `detail-header` :: "The detail header is bold and uncoloured at both mandated
    /// widths".
    #[test]
    fn the_detail_header_is_bold_and_uncoloured_at_both_mandated_widths() {
        let d = dashboard_with(
            vec![
                fixture::active("add-token-refresh", 4, 9),
                fixture::active("fix-empty-basket", 7, 7),
            ],
            Vec::new(),
            // `list-sections`: 1, not 0 — target 0 is the active header.
            1,
            Route::Detail,
        );
        let progress = crate::tasks::Progress {
            completed: 4,
            total: 9,
        };
        // The wide detail region has no right gutter (D5), so its header
        // spans columns 42 through 119, not 41 through 118.
        for (width, first, last, w) in [(120, 42, 119, 78), (60, 1, 58, 58)] {
            let buf = render_at(width, 20, &d);
            assert_eq!(
                detail_interior_cols(&buf, 0, w as usize),
                crate::ui::detail::header_row("add-token-refresh", "tdd", &progress, w),
                "width {width}"
            );
            for x in first..=last {
                let style = cell(&buf, x, 0).style();
                assert!(
                    style.add_modifier.contains(Modifier::BOLD),
                    "width {width}: header cell {x} is not bold"
                );
                assert_eq!(
                    style.fg,
                    uncoloured().fg,
                    "width {width}: header cell {x} carries a foreground"
                );
                assert_eq!(
                    style.bg,
                    uncoloured().bg,
                    "width {width}: header cell {x} carries a background"
                );
            }
            // The tab-bar row two rows below it — buffer row 2 — does carry a
            // background, so the two rows are distinguishable and the header
            // was not left unstyled by accident.
            assert!(
                (first..=last).any(|x| cell(&buf, x, 2).style().bg != uncoloured().bg),
                "width {width}: the tab bar carries no background at all"
            );
        }
    }

    /// `responsive-layout` :: "No heading or rule cell carries a colour" — the
    /// replacement for the removed border's own colourlessness scenario: there is no
    /// border any more, but the same claim holds of a region's heading row and of
    /// both rules (the wide layout's divider and the detail region's horizontal
    /// rule below its tab bar), whichever region is routed.
    #[test]
    fn no_heading_or_rule_cell_carries_a_colour() {
        let list = dashboard_with(
            vec![fixture::active("alpha", 1, 3)],
            Vec::new(),
            1,
            Route::List,
        );
        let detail = dashboard_with(
            vec![fixture::active("alpha", 1, 3)],
            Vec::new(),
            1,
            Route::Detail,
        );

        for d in [&list, &detail] {
            for width in [120u16, 60] {
                let buf = render_at(width, 20, d);

                // No cell of the list region's own heading row carries a colour.
                let last = if width == 60 { 58u16 } else { 38 };
                for x in 1..=last {
                    let style = cell(&buf, x, 0).style();
                    assert_eq!(style.fg, uncoloured().fg, "width {width}: list heading {x}");
                    assert_eq!(style.bg, uncoloured().bg, "width {width}: list heading {x}");
                }

                if width == 120 {
                    // Nor the detail region's own heading row.
                    for x in 42..=119u16 {
                        let style = cell(&buf, x, 0).style();
                        assert_eq!(style.fg, uncoloured().fg, "detail heading {x}");
                        assert_eq!(style.bg, uncoloured().bg, "detail heading {x}");
                    }
                    // Nor the vertical divider, which is dim throughout the body.
                    for y in 0..=18u16 {
                        let style = cell(&buf, 40, y).style();
                        assert_eq!(style.fg, uncoloured().fg, "divider y={y}");
                        assert_eq!(style.bg, uncoloured().bg, "divider y={y}");
                        assert!(is_dim(cell(&buf, 40, y)), "divider y={y}");
                    }
                    // Nor the detail region's own horizontal rule, buffer row 3.
                    for x in 42..=119u16 {
                        let style = cell(&buf, x, 3).style();
                        assert_eq!(style.fg, uncoloured().fg, "rule {x}");
                        assert_eq!(style.bg, uncoloured().bg, "rule {x}");
                        assert!(is_dim(cell(&buf, x, 3)), "rule {x}");
                    }
                }
            }
        }

        // With `file_mode` true the nine badge cells of the list heading row do
        // carry `Role::FileMode`'s own foreground, so the absence of colour above
        // is a property of the heading role rather than of the row.
        let file_mode = dashboard_in_file_mode(Some("/tmp/demo-repo"), Route::List);
        let buf = render_at(120, 20, &file_mode);
        for x in 30..=38u16 {
            assert_eq!(
                cell(&buf, x, 0).style().fg,
                palette::style(Role::FileMode).fg,
                "badge col {x}"
            );
        }
    }

    /// `list-selection` :: "The selected row is bold and uncoloured at both mandated
    /// widths".
    #[test]
    fn the_selected_row_is_bold_and_uncoloured() {
        // `list-sections`: row 2 is the active section header; add-token-refresh
        // (target 1) is row 3, and fix-empty-basket (unselected) is row 4.
        let d = three_active();
        for width in [120, 60] {
            let buf = render_at(width, 20, &d);
            let last = if width == 60 { 58 } else { 38 };
            assert_eq!(cell(&buf, 1, 3).symbol(), ">", "width {width}");
            assert_eq!(cell(&buf, 1, 4).symbol(), " ", "width {width}");
            for x in 1..=last {
                let selected = cell(&buf, x, 3).style();
                assert!(
                    selected.add_modifier.contains(Modifier::BOLD),
                    "width {width}: selected cell {x} is not bold"
                );
                assert_eq!(
                    selected.fg,
                    uncoloured().fg,
                    "width {width}: selected cell {x} carries a foreground"
                );
                let next = cell(&buf, x, 4).style();
                assert!(
                    !next.add_modifier.contains(Modifier::BOLD),
                    "width {width}: unselected cell {x} is bold"
                );
                assert_eq!(
                    next.fg,
                    uncoloured().fg,
                    "width {width}: unselected cell {x} carries a foreground"
                );
            }
        }
    }

    /// Three active changes whose names and progress cells contain neither `w` nor
    /// `b`, so [`only_column_of`] below can find a badge by scanning the drawn row
    /// rather than by re-deriving `ui::list`'s own arithmetic. `alpha` is selected,
    /// `alpha` and `gamma` are badged, `epsilon` is not.
    fn badged_dashboard(selected: usize) -> Dashboard {
        let mut d = dashboard_with(
            vec![
                fixture::active("alpha", 4, 9),
                fixture::active("gamma", 7, 7),
                fixture::active("epsilon", 0, 0),
            ],
            Vec::new(),
            selected,
            Route::List,
        );
        let mut working = unattributed_agent("alpha");
        working.status = crate::agents::AgentStatus::Working;
        let mut blocked = unattributed_agent("gamma");
        blocked.status = crate::agents::AgentStatus::Blocked;
        d.agents.agents = vec![working, blocked];
        d
    }

    /// The one interior column of buffer row `y` carrying `glyph`, asserting there
    /// is exactly one. The badge cell is located by reading the frame, never by
    /// recomputing the row grammar the frame was drawn from.
    fn only_column_of(buf: &Buffer, y: u16, glyph: &str, last: u16) -> u16 {
        let found: Vec<u16> = (1..=last)
            .filter(|x| cell(buf, *x, y).symbol() == glyph)
            .collect();
        assert_eq!(
            found.len(),
            1,
            "row {y} must carry exactly one {glyph:?}: {found:?}"
        );
        found[0]
    }

    /// `change-rows` :: "The badge cell reaches the buffer coloured and the rest of
    /// the row does not".
    #[test]
    fn the_badge_cell_reaches_the_buffer_coloured_and_the_rest_of_the_row_does_not() {
        // `list-sections`: `selected` **1** addresses `Target::Change(0)` —
        // `alpha` — since the active section header is target 0; rows 3, 4, and
        // 5 (not 2, 3, and 4) are alpha, gamma, and epsilon.
        let d = badged_dashboard(1);
        for width in [120, 60] {
            let buf = render_at(width, 20, &d);
            let last = if width == 60 { 58 } else { 38 };

            let wx = only_column_of(&buf, 3, "w", last);
            let working = cell(&buf, wx, 3).style();
            assert_eq!(
                working.fg,
                palette::style(Role::AgentBadge(crate::agents::AgentStatus::Working)).fg,
                "width {width}: the working badge is not the palette's colour"
            );
            assert!(
                working.add_modifier.contains(Modifier::BOLD),
                "width {width}: the badge on the selected row lost its bold"
            );

            let bx = only_column_of(&buf, 4, "b", last);
            let blocked = cell(&buf, bx, 4).style();
            assert_eq!(
                blocked.fg,
                palette::style(Role::AgentBadge(crate::agents::AgentStatus::Blocked)).fg,
                "width {width}: the blocked badge is not the palette's colour"
            );
            assert!(
                !blocked.add_modifier.contains(Modifier::BOLD),
                "width {width}: an unselected row's badge is bold"
            );

            // Exactly one column was painted on each badged row: the separating
            // spaces on either side of the badge carry no foreground at all.
            for (x, y) in [(wx, 3u16), (bx, 4)] {
                for neighbour in [x - 1, x + 1] {
                    assert_eq!(
                        cell(&buf, neighbour, y).style().fg,
                        uncoloured().fg,
                        "width {width}: cell {neighbour},{y} beside the badge is coloured"
                    );
                }
            }

            // The unbadged third row carries no foreground anywhere.
            for x in 1..=last {
                assert_eq!(
                    cell(&buf, x, 5).style().fg,
                    uncoloured().fg,
                    "width {width}: unbadged cell {x} is coloured"
                );
            }
        }
    }

    /// `change-rows` :: "A badged selected row keeps its bold under the badge colour"
    /// — and the discriminating half: moving the selection moves the bold without
    /// moving the colour.
    #[test]
    fn a_badged_selected_row_keeps_its_bold_under_the_badge_colour() {
        let working = palette::style(Role::AgentBadge(crate::agents::AgentStatus::Working)).fg;
        let blocked = palette::style(Role::AgentBadge(crate::agents::AgentStatus::Blocked)).fg;
        // `list-sections`: `selected` **1** addresses `Target::Change(0)` — alpha
        // — and **2** addresses `Target::Change(1)` — gamma — since the active
        // section header is target 0; alpha and gamma's rows are 3 and 4.
        for width in [120, 60] {
            let last = if width == 60 { 58 } else { 38 };
            for selected in [1usize, 2] {
                let buf = render_at(width, 20, &badged_dashboard(selected));
                let wx = only_column_of(&buf, 3, "w", last);
                let bx = only_column_of(&buf, 4, "b", last);
                assert_eq!(cell(&buf, wx, 3).style().fg, working, "width {width}");
                assert_eq!(cell(&buf, bx, 4).style().fg, blocked, "width {width}");
                assert_eq!(
                    cell(&buf, wx, 3)
                        .style()
                        .add_modifier
                        .contains(Modifier::BOLD),
                    selected == 1,
                    "width {width}: alpha's badge bold disagrees with the selection"
                );
                assert_eq!(
                    cell(&buf, bx, 4)
                        .style()
                        .add_modifier
                        .contains(Modifier::BOLD),
                    selected == 2,
                    "width {width}: gamma's badge bold disagrees with the selection"
                );
            }
        }
    }

    /// `change-rows` :: "Problem rows are red and change rows are not, at both
    /// mandated widths".
    #[test]
    fn problem_rows_are_red_and_change_rows_are_not_at_both_mandated_widths() {
        // `list-sections`: `selected` **1** addresses `Target::Change(0)` —
        // `alpha` — since the active section header is target 0. Rows: 2-4
        // problems, 5 the active header, 6-8 alpha/gamma/epsilon, 9 the
        // archived header, 10 old-change.
        let mut d = dashboard_with(
            vec![
                fixture::active("alpha", 4, 9),
                fixture::active("gamma", 7, 7),
                fixture::active("epsilon", 0, 0),
            ],
            vec![fixture::archived(Some("2026-08-14"), "old-change", 3, 3)],
            1,
            Route::List,
        );
        d.launch.problems = vec!["herdr agent start: refused".to_string()];
        d.refresh.problems = vec!["watch: could not start".to_string()];
        d.changes.problems = vec!["openspec/changes: unreadable".to_string()];

        for width in [120, 60] {
            let buf = render_at(width, 20, &d);
            let last = if width == 60 { 58 } else { 38 };
            assert!(
                interior_cols(&buf, 2).starts_with("! herdr"),
                "width {width}"
            );
            assert!(
                interior_cols(&buf, 3).starts_with("! watch"),
                "width {width}"
            );
            assert!(
                interior_cols(&buf, 4).starts_with("! openspec"),
                "width {width}"
            );
            assert!(interior_cols(&buf, 9).contains("archived"), "width {width}");

            for x in 1..=last {
                for y in [2u16, 3, 4] {
                    let style = cell(&buf, x, y).style();
                    assert_eq!(
                        style.fg,
                        palette::style(Role::ListProblem).fg,
                        "width {width}: problem cell {x},{y} is not the problem colour"
                    );
                    assert!(
                        style.add_modifier.is_empty(),
                        "width {width}: problem cell {x},{y} gained a modifier"
                    );
                }
                let active_header = cell(&buf, x, 5).style();
                assert_eq!(
                    active_header.fg,
                    palette::style(Role::ListSeparator).fg,
                    "width {width}: active header cell {x} is not the separator colour"
                );
                assert!(
                    active_header.add_modifier.is_empty(),
                    "width {width}: unselected active header cell {x} gained a modifier"
                );
                let separator = cell(&buf, x, 9).style();
                assert_eq!(
                    separator.fg,
                    palette::style(Role::ListSeparator).fg,
                    "width {width}: archived header cell {x} is not the separator colour"
                );
                assert!(
                    separator.add_modifier.is_empty(),
                    "width {width}: archived header cell {x} gained a modifier"
                );
                for y in [6u16, 7, 8, 10] {
                    assert_eq!(
                        cell(&buf, x, y).style().fg,
                        uncoloured().fg,
                        "width {width}: change cell {x},{y} is coloured"
                    );
                }
                assert!(
                    cell(&buf, x, 6)
                        .style()
                        .add_modifier
                        .contains(Modifier::BOLD),
                    "width {width}: the selected change row {x} lost its bold"
                );
            }
        }
    }

    /// `change-rows` :: "An empty-state message row is not a problem row".
    #[test]
    fn an_empty_state_message_row_is_not_a_problem_row() {
        let empty = dashboard(Some("/tmp/demo-repo"), Route::List);
        let mut filtered = dashboard_with(
            vec![fixture::active("alpha", 4, 9)],
            Vec::new(),
            0,
            Route::List,
        );
        filtered.filter.query = "zzz".to_string();
        let no_repo = dashboard(None, Route::List);

        for width in [120, 60] {
            let last = if width == 60 { 58 } else { 38 };

            let buf = render_at(width, 20, &empty);
            assert!(
                interior_cols(&buf, 2).starts_with("No changes yet"),
                "width {width}"
            );
            let buf_filtered = render_at(width, 20, &filtered);
            assert!(
                interior_cols(&buf_filtered, 2).starts_with("No changes match"),
                "width {width}"
            );
            assert!(
                interior_cols(&buf_filtered, 3).starts_with("/zzz"),
                "width {width}"
            );
            let buf_no_repo = render_at(width, 20, &no_repo);
            assert!(
                interior_cols(&buf_no_repo, 2).starts_with("No OpenSpec repository"),
                "width {width}"
            );

            for x in 1..=last {
                for (label, buf, rows) in [
                    ("empty", &buf, vec![2u16]),
                    ("filtered", &buf_filtered, vec![2, 3]),
                    ("no repository", &buf_no_repo, vec![2, 3, 4]),
                ] {
                    for y in rows {
                        assert_eq!(
                            cell(buf, x, y).style().fg,
                            uncoloured().fg,
                            "width {width}: {label} message cell {x},{y} is coloured"
                        );
                    }
                }
            }
        }
    }

    /// A frame carrying every span this change touches at once: file mode, a repository
    /// problem row, three active changes with the second badged `Working`, an archived
    /// change (so a separator row is drawn), and a selected change whose detail source
    /// carries a heading and the four inline faces.
    fn monochrome_dashboard(route: Route) -> Dashboard {
        let selected = fixture::with_artifacts(
            fixture::active("add-token-refresh", 4, 9),
            &[("proposal", &[])],
        );
        let mut d = dashboard_with_detail(
            vec![
                selected,
                fixture::active("fix-empty-basket", 7, 7),
                fixture::active("migrate-ai-sdk-v7", 0, 0),
            ],
            vec![fixture::archived(Some("2026-01-01"), "old-change", 3, 3)],
            // `list-sections`: 1, not 0 — target 0 is the active header.
            1,
            route,
            Detail {
                source: "## Heading\n\n**bold** and *italic* and `code` and [link](u)\n"
                    .to_string(),
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
            },
        );
        d.file_mode = true;
        d.changes.problems = vec!["openspec/changes: unreadable".to_string()];
        let mut working = unattributed_agent("fix-empty-basket");
        working.status = crate::agents::AgentStatus::Working;
        d.agents.agents = vec![working];
        d
    }

    /// `view-palette` :: "A monochrome reading of the frame is unchanged" — the modifier
    /// of every cell outside the tab-bar row is exactly what it was before this change.
    #[test]
    fn a_monochrome_reading_of_the_frame_is_unchanged() {
        // The list half. Rows: 2 the problem, 3 the active section header, 4 the
        // selected change (`add-token-refresh`), 5 the badged one
        // (`fix-empty-basket`), 6 the third, 7 the archived header, 8 the
        // archived change.
        //
        // `monochrome_dashboard`'s `selected` field is 1, addressing
        // `add-token-refresh` in `targets()` space (the active header is target
        // 0) — what the detail half below reads too, and now what
        // `ui::list::rows` marks as well.
        for width in [120, 60] {
            let d = monochrome_dashboard(Route::List);
            let buf = render_at(width, 20, &d);
            let last = if width == 60 { 58 } else { 38 };

            // The list region's own heading row (`pane-chrome` -> D2): "demo-repo"
            // bold at columns 1 through 9, one gutter column in from the region's
            // own left edge, and the `file mode` badge right-aligned against the
            // heading's own last column, nine columns wide.
            for x in 1..10u16 {
                assert!(is_bold(cell(&buf, x, 0)), "width {width}: heading cell {x}");
            }
            let badge_start = last - 8;
            for x in badge_start..=last {
                assert!(
                    cell(&buf, x, 0)
                        .style()
                        .add_modifier
                        .contains(Modifier::DIM),
                    "width {width}: badge cell {x} is not dim"
                );
            }

            assert!(
                interior_cols(&buf, 2).starts_with("! openspec/changes"),
                "width {width}: row 2 is not the problem row"
            );
            assert!(
                interior_cols(&buf, 7).contains("archived"),
                "width {width}: row 7 is not the separator row"
            );
            let badge = interior_cols(&buf, 5);
            assert!(
                badge.contains("fix-empty-basket") && badge.contains('w'),
                "width {width}: row 5 does not carry the working badge: {badge:?}"
            );

            for x in 1..=last {
                assert!(
                    is_bold(cell(&buf, x, 4)),
                    "width {width}: the marked row's cell {x} is not bold"
                );
                for y in [2u16, 3, 5, 7] {
                    assert!(
                        cell(&buf, x, y).style().add_modifier.is_empty(),
                        "width {width}: cell {x},{y} carries a modifier it did not before"
                    );
                }
            }
            // The footer carries no modifier either.
            for x in 0..width {
                assert!(
                    cell(&buf, x, 19).style().add_modifier.is_empty(),
                    "width {width}: footer cell {x} carries a modifier"
                );
            }
        }

        // The detail half, at the same two widths.
        for width in [120, 60] {
            let d = monochrome_dashboard(Route::Detail);
            let buf = render_at(width, 20, &d);
            let (first, last) = if width == 60 {
                (1u16, 58u16)
            } else {
                (42, 119)
            };

            for x in first..=last {
                assert!(
                    is_bold(cell(&buf, x, 0)),
                    "width {width}: detail header cell {x} is not bold"
                );
            }
            assert!(
                find_cell_style(&buf, "## Heading")
                    .add_modifier
                    .contains(Modifier::BOLD),
                "width {width}"
            );
            assert!(
                find_cell_style(&buf, "bold")
                    .add_modifier
                    .contains(Modifier::BOLD),
                "width {width}"
            );
            assert!(
                find_cell_style(&buf, "italic")
                    .add_modifier
                    .contains(Modifier::ITALIC),
                "width {width}"
            );
            assert!(
                find_cell_style(&buf, "code")
                    .add_modifier
                    .contains(Modifier::DIM),
                "width {width}"
            );
            assert!(
                find_cell_style(&buf, "link")
                    .add_modifier
                    .contains(Modifier::UNDERLINED),
                "width {width}"
            );

            // The one excepted row: in the tab bar the selected chip's span is the only
            // BOLD one, so a monochrome reader still learns which tab is current.
            let change = d.selected_change().expect("a change is selected");
            let tabs = crate::ui::detail::tab_bar(&change.artifacts, 0, last - first + 1);
            let selected_tab = tabs.iter().find(|t| t.selected).expect("a selected chip");
            let expected: Vec<u16> = (0..columns(&selected_tab.text) as u16)
                .map(|i| first + selected_tab.x + i)
                .collect();
            let bold: Vec<u16> = (first..=last)
                .filter(|x| is_bold(cell(&buf, *x, 2)))
                .collect();
            assert_eq!(
                bold, expected,
                "width {width}: the selected chip must be the tab bar's only bold span"
            );
        }

        // `markdown-constructs`: the same source with `~~struck~~` appended renders
        // that word's cells with CROSSED_OUT and leaves every other cell's modifier
        // unchanged, so the new role adds a modifier only where the new construct
        // appears — a construct the parser could not emit at all before.
        for width in [120, 60] {
            let before = monochrome_dashboard(Route::Detail);
            let mut after = monochrome_dashboard(Route::Detail);
            after.detail.source.push_str("and ~~struck~~\n");
            let buf_before = render_at(width, 20, &before);
            let buf_after = render_at(width, 20, &after);
            assert!(
                find_cell_style(&buf_after, "struck")
                    .add_modifier
                    .contains(Modifier::CROSSED_OUT),
                "width {width}: the appended struck run is not crossed out"
            );
            for y in 0..20u16 {
                for x in 0..width {
                    let a = cell(&buf_after, x, y).style().add_modifier;
                    if a.contains(Modifier::CROSSED_OUT) {
                        continue;
                    }
                    assert_eq!(
                        cell(&buf_before, x, y).style().add_modifier,
                        a,
                        "width {width}: cell {x},{y} changed modifier"
                    );
                }
            }
        }
    }
}
