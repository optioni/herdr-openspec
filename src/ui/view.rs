//! The render seam's pure side: `render(frame, &Dashboard)`. Performs no
//! filesystem, process, environment, network, or terminal I/O, reads no
//! clock and no global state. See
//! `openspec/changes/tui-shell/specs/responsive-layout/spec.md`.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Block;

use crate::ui::app::{Dashboard, Route};
use crate::ui::detail;
use crate::ui::layout::{
    columns, interior, scroll_offset, split_body, split_detail, split_frame, truncate_columns,
    viewport,
};
use crate::ui::list;
use crate::ui::markdown::Face;

/// The footer's key hints, in the order they are drawn and dropped from.
const FOOTER_HINTS: [&str; 3] = ["q quit", "Enter detail", "Esc back"];

/// Draw `dashboard` into `frame`. A pure function of its two arguments.
pub fn render(frame: &mut Frame, dashboard: &Dashboard) {
    let (header, body, footer) = split_frame(frame.area());
    render_header(frame, header, dashboard);
    render_footer(frame, footer, dashboard);
    render_body(frame, body, dashboard);
}

/// The body: one or two bordered regions, per `layout::split_body`. The list
/// region's interior is filled by `render_list`; the detail region's by
/// `render_detail` — `detail-view` is what will populate `detail.source`.
fn render_body(frame: &mut Frame, body: Rect, dashboard: &Dashboard) {
    let (list_area, detail) = split_body(body, dashboard.route);
    if let Some(area) = list_area {
        render_region(frame, area, "Changes", dashboard.route == Route::List);
        render_list(frame, interior(area), dashboard);
    }
    if let Some(area) = detail {
        render_region(frame, area, "Detail", dashboard.route == Route::Detail);
        render_detail(frame, interior(area), dashboard);
    }
}

/// Draw the change header, the tab bar, and the content area into
/// `interior`, through `layout::split_detail` — nothing at all when
/// `Dashboard::visible()` is empty (`selected_change()` is `None`), which is
/// what leaves every interior cell blank in that state. When a change
/// **is** selected, all three rows are always drawn: the header and the
/// tab bar unconditionally, and the content area holds at least one line
/// because `ui::detail::content_lines` returns `No content yet` rather
/// than nothing.
fn render_detail(frame: &mut Frame, interior: Rect, dashboard: &Dashboard) {
    let Some(change) = dashboard.selected_change() else {
        return;
    };
    let (header, tabs, content) = split_detail(interior);
    render_detail_header(frame, header, change);
    render_detail_tabs(frame, tabs, change, dashboard.detail.tab);
    render_detail_content(frame, content, dashboard);
}

/// The change header: `ui::detail::header_row`, bold, at the row's first
/// column. Draws nothing at zero width or zero height.
fn render_detail_header(frame: &mut Frame, header: Rect, change: &crate::changes::Change) {
    if header.width == 0 || header.height == 0 {
        return;
    }
    let text = detail::header_row(&change.name, &change.schema, &change.progress, header.width);
    frame.buffer_mut().set_string(
        header.x,
        header.y,
        &text,
        Style::default().add_modifier(Modifier::BOLD),
    );
}

/// The tab bar: every `ui::detail::Tab` at `tabs.x + tab.x`, bold for the
/// selected cell and plain for every other. Draws nothing at zero width or
/// zero height.
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
        let mut style = Style::default();
        if cell.selected {
            style = style.add_modifier(Modifier::BOLD);
        }
        buf.set_string(x, tabs.y, &cell.text, style);
    }
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
            // `view-fidelity` -> Decision 8: the guard above is now correct, since `x`
            // advances by consumed columns rather than characters — but it fires only
            // *between* segments, so a single segment wider than the space remaining
            // would still cross the border. The clamp below is kept alongside it, on the
            // same terms the markdown parser's own defaults are asserted three ways
            // elsewhere in this crate: each mechanism alone is dodgeable.
            let remaining = (last_col - x) as usize;
            let text = truncate_columns(&segment.text, remaining);
            let style = style_for(&segment.face);
            buf.set_string(x, y, text, style);
            x += columns(text) as u16;
        }
    }
}

/// The crate's only `Face`-to-`Style` mapping: `heading` present or
/// `strong` -> `BOLD`; `emphasis` -> `ITALIC`; `code` -> `DIM`; `link` ->
/// `UNDERLINED`; `quoted` -> `DIM`. Flags compose, so a bold link's cells
/// carry `BOLD` and `UNDERLINED` together.
fn style_for(face: &Face) -> Style {
    let mut style = Style::default();
    if face.heading.is_some() || face.strong {
        style = style.add_modifier(Modifier::BOLD);
    }
    if face.emphasis {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if face.code {
        style = style.add_modifier(Modifier::DIM);
    }
    if face.link {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    if face.quoted {
        style = style.add_modifier(Modifier::DIM);
    }
    style
}

/// Draw `list::rows(dashboard, interior.width)` into `interior`: the slice
/// `layout::viewport` selects, one row per terminal row starting at the
/// interior's first row and column, with `Modifier::BOLD` applied to the
/// selected row's cells and `Style::default()` to every other row's. Draws
/// nothing when the interior has zero width or zero height — there is
/// nothing to index into.
fn render_list(frame: &mut Frame, interior: Rect, dashboard: &Dashboard) {
    if interior.width == 0 || interior.height == 0 {
        return;
    }
    let rows = list::rows(dashboard, interior.width);
    let cursor = rows.iter().position(|r| r.selected).unwrap_or(0);
    let offset = viewport(rows.len(), cursor, interior.height);
    let buf = frame.buffer_mut();
    for (i, row) in rows
        .iter()
        .skip(offset)
        .take(interior.height as usize)
        .enumerate()
    {
        let y = interior.y + i as u16;
        let mut style = Style::default();
        if row.selected {
            style = style.add_modifier(Modifier::BOLD);
        }
        buf.set_string(interior.x, y, &row.text, style);
    }
}

/// One bordered region with a title. `emphasised` bolds the border — the
/// routed region always is, whether or not the other region is drawn
/// alongside it.
fn render_region(frame: &mut Frame, area: Rect, title: &'static str, emphasised: bool) {
    let mut border_style = Style::default();
    if emphasised {
        border_style = border_style.add_modifier(Modifier::BOLD);
    }
    let block = Block::bordered().title(title).border_style(border_style);
    frame.render_widget(block, area);
}

/// The `file mode` badge's own text — nine columns, drawn dim, immediately
/// after `OpenSpec`'s separating blank. See `specs/responsive-layout/spec.md`.
const FILE_MODE_BADGE: &str = "file mode";

/// The narrowest header the badge is drawn in at all — `degraded-states`'
/// addition. Below this the badge is dropped whole, before the path's own
/// shortening arithmetic ever runs, rather than being truncated itself.
const BADGE_MIN_WIDTH: u16 = 18;

/// The header row: `OpenSpec`, bold, at column 0; when `dashboard.file_mode`
/// and the header is wide enough, the dim `file mode` badge in columns 9-17
/// (`degraded-states`' addition — `SPEC.md` row 2); then the repository's
/// display path (or `no repository`) right-aligned, shortened from the left
/// when the remaining width cannot hold it whole.
fn render_header(frame: &mut Frame, header: Rect, dashboard: &Dashboard) {
    if header.height == 0 {
        return;
    }
    let buf = frame.buffer_mut();
    buf.set_string(
        header.x,
        header.y,
        "OpenSpec",
        Style::default().add_modifier(Modifier::BOLD),
    );

    let show_badge = dashboard.file_mode && header.width >= BADGE_MIN_WIDTH;
    if show_badge {
        buf.set_string(
            header.x + 9,
            header.y,
            FILE_MODE_BADGE,
            Style::default().add_modifier(Modifier::DIM),
        );
    }

    let text = match &dashboard.repo {
        Some(path) => path.display().to_string(),
        None => "no repository".to_string(),
    };
    // `A` is the budget left for the path: the header width minus 9 (the eight columns of
    // `OpenSpec` plus one separating blank) when no badge is drawn, or minus 19 (that same
    // nine, plus the badge's own nine columns, plus a second separating blank) when it is —
    // the badge's columns come from the path's own budget, never from `OpenSpec`'s.
    let a = if show_badge {
        header.width.saturating_sub(19)
    } else {
        header.width.saturating_sub(9)
    };
    if let Some(shown) = shorten_for_header(&text, a) {
        let shown_len = columns(&shown) as u16;
        let x = header.x + header.width.saturating_sub(shown_len);
        buf.set_string(x, header.y, &shown, Style::default());
    }
}

/// The header-shortening rule, isolated so it is readable independently of
/// the frame: `text` fitting in `a` **columns** is shown whole; longer text
/// is shown as `…` plus its last `a - 1` columns when `a >= 8`; otherwise
/// nothing is shown at all, and only the (possibly itself truncated)
/// `OpenSpec` label — and the badge, when [`render_header`] drew one — is
/// shown.
fn shorten_for_header(text: &str, a: u16) -> Option<String> {
    let a = a as usize;
    let text_cols = columns(text);
    if text_cols <= a {
        Some(text.to_string())
    } else if a >= 8 {
        Some(list::shorten_left(text, a))
    } else {
        None
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
            .set_string(footer.x, footer.y, &text, Style::default());
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
    use ratatui::style::Modifier;

    use crate::changes::{Change, empty_set, fixture};
    use crate::testutil::{cell, render_at, row_text};
    use crate::ui::app::{Action, Dashboard, Detail, Filter, Route};
    use crate::ui::layout::columns;

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

    fn is_bold(cell: &Cell) -> bool {
        cell.style().add_modifier.contains(Modifier::BOLD)
    }

    #[test]
    fn frame_rows_at_60_and_120() {
        let d = dashboard(Some("/tmp/demo-repo"), Route::List);
        // `agent-launch`: pinned unreachable, so this footer is the one this capability
        // specified before the action hints existed.
        assert!(!d.agents.reachable);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert_eq!(cols(&row_text(&buf, 0), 0..8), "OpenSpec");
            assert!(is_bold(cell(&buf, 0, 0)));
            let footer = row_text(&buf, 19);
            assert!(footer.starts_with("q quit  Enter detail  Esc back"));
            assert!(
                footer.chars().skip(30).all(|c| c == ' '),
                "footer remainder must be all spaces: {footer:?}"
            );
            assert_eq!(cell(&buf, 0, 1).symbol(), "┌");
            assert_eq!(cell(&buf, 0, 18).symbol(), "└");
        }
    }

    #[test]
    fn zero_height_frame_draws_nothing() {
        // responsive-layout: "at height 0 render SHALL draw nothing." Closes a
        // Change Review finding — split_frame's own height-0 case was tested at
        // the layout tier (ui::layout::tests::split_frame_degenerate_heights)
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
    fn one_row_frame_draws_header_only() {
        let d = dashboard(Some("/tmp/demo-repo"), Route::List);
        for width in [60, 120] {
            let buf = render_at(width, 1, &d);
            assert_eq!(cols(&row_text(&buf, 0), 0..8), "OpenSpec");
            assert!(!buffer_contains(&buf, "┌"));
            assert!(!buffer_contains(&buf, "q quit"));
        }
    }

    #[test]
    fn two_row_frame_draws_no_body() {
        let d = dashboard(Some("/tmp/demo-repo"), Route::List);
        for width in [60, 120] {
            let buf = render_at(width, 2, &d);
            assert_eq!(cols(&row_text(&buf, 0), 0..8), "OpenSpec");
            assert_eq!(cols(&row_text(&buf, 1), 0..6), "q quit");
            assert!(!buffer_contains(&buf, "┌"));
        }
    }

    #[test]
    fn one_column_frame_does_not_panic() {
        let d = dashboard(Some("/tmp/demo-repo"), Route::List);
        let buf = render_at(1, 1, &d);
        let _ = buf;
        let buf = render_at(1, 20, &d);
        assert_eq!(row_text(&buf, 0), "O");
        assert_eq!(row_text(&buf, 19), " ");
        // Extended: a 2x20 render, so a zero-column list-region interior is
        // exercised too (a 2-wide frame's list region has width 2, and
        // Block::bordered().inner() of that is width 0).
        let buf = render_at(2, 20, &d);
        let _ = buf;
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert_eq!(cols(&row_text(&buf, 0), 0..8), "OpenSpec");
            assert_eq!(cols(&row_text(&buf, 19), 0..6), "q quit");
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
        assert_eq!(expected.chars().count(), 46);
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
        assert_eq!(expected_120.chars().count(), 69);
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

        let buf120 = render_at(120, 20, &d);
        assert_eq!(
            interior_cols(&buf120, 2),
            "> add-token-refresh            w [4/9]"
        );
        assert_eq!(
            interior_cols(&buf120, 3),
            "  fix-empty-basket             b [7/7]"
        );
        assert_eq!(
            interior_cols(&buf120, 4),
            "  migrate-ai-sdk-v7              ? [-]"
        );

        let buf60 = render_at(60, 20, &d);
        for y in [2u16, 3, 4] {
            assert_eq!(interior_cols(&buf60, y).chars().count(), 58);
        }
        assert_eq!(interior_cols(&buf60, 2).chars().nth(51), Some('w'));
        assert_eq!(interior_cols(&buf60, 3).chars().nth(51), Some('b'));
        assert_eq!(interior_cols(&buf60, 4).chars().nth(53), Some('?'));
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
    fn wide_draws_two_regions_divided_at_40() {
        let d = dashboard(Some("/tmp/demo-repo"), Route::List);
        let buf = render_at(120, 20, &d);
        assert_eq!(cell(&buf, 0, 1).symbol(), "┌");
        assert_eq!(cell(&buf, 39, 1).symbol(), "┐");
        assert_eq!(cell(&buf, 40, 1).symbol(), "┌");
        assert_eq!(cell(&buf, 119, 1).symbol(), "┐");
        assert_eq!(cols(&row_text(&buf, 1), 1..8), "Changes");
        assert_eq!(cols(&row_text(&buf, 1), 41..47), "Detail");
        assert_eq!(cell(&buf, 0, 18).symbol(), "└");
        assert_eq!(cell(&buf, 39, 18).symbol(), "┘");
        assert_eq!(cell(&buf, 40, 18).symbol(), "└");
        assert_eq!(cell(&buf, 119, 18).symbol(), "┘");

        let buf60 = render_at(60, 20, &d);
        assert_eq!(count_char(&buf60, '┌'), 1);
    }

    #[test]
    fn narrow_draws_only_the_list_region() {
        let d = dashboard(Some("/tmp/demo-repo"), Route::List);
        let buf = render_at(60, 20, &d);
        assert_eq!(cell(&buf, 0, 1).symbol(), "┌");
        assert_eq!(cell(&buf, 59, 1).symbol(), "┐");
        assert_eq!(count_char(&buf, '┌'), 1);
        assert_eq!(cols(&row_text(&buf, 1), 1..8), "Changes");
        assert!(!buffer_contains(&buf, "Detail"));

        let buf120 = render_at(120, 20, &d);
        assert_eq!(count_char(&buf120, '┌'), 2);
    }

    #[test]
    fn narrow_detail_route_replaces_the_list_region() {
        let d = dashboard(Some("/tmp/demo-repo"), Route::Detail);
        let buf = render_at(60, 20, &d);
        assert_eq!(cols(&row_text(&buf, 1), 1..7), "Detail");
        assert!(!buffer_contains(&buf, "Changes"));

        let buf120 = render_at(120, 20, &d);
        assert_eq!(cols(&row_text(&buf120, 1), 1..8), "Changes");
        assert_eq!(cols(&row_text(&buf120, 1), 41..47), "Detail");
    }

    #[test]
    fn breakpoint_is_exact_at_the_boundary() {
        let d = dashboard(Some("/tmp/demo-repo"), Route::List);
        for width in [60, 99] {
            let buf = render_at(width, 20, &d);
            assert_eq!(count_char(&buf, '┌'), 1, "width {width}");
        }
        for width in [100, 101, 120] {
            let buf = render_at(width, 20, &d);
            assert_eq!(count_char(&buf, '┌'), 2, "width {width}");
            assert_eq!(cell(&buf, 40, 1).symbol(), "┌");
        }
    }

    #[test]
    fn resizing_the_backend_changes_the_next_frame() {
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
        assert_eq!(count_char(&first, '┌'), 2);
        assert_eq!(count_char(&second, '┌'), 1);
    }

    #[test]
    fn routed_region_border_is_bold() {
        let list = dashboard(Some("/tmp/demo-repo"), Route::List);
        let buf = render_at(120, 20, &list);
        assert!(is_bold(cell(&buf, 0, 1)));
        assert!(!is_bold(cell(&buf, 40, 1)));

        let detail = dashboard(Some("/tmp/demo-repo"), Route::Detail);
        let buf = render_at(120, 20, &detail);
        assert!(!is_bold(cell(&buf, 0, 1)));
        assert!(is_bold(cell(&buf, 40, 1)));

        for route in [Route::List, Route::Detail] {
            let d = dashboard(Some("/tmp/demo-repo"), route);
            let buf = render_at(60, 20, &d);
            assert!(is_bold(cell(&buf, 0, 1)), "route {route:?}");
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

    #[test]
    fn header_path_right_aligned_whole() {
        let d = dashboard(Some("/tmp/demo-repo"), Route::List);
        let buf = render_at(60, 20, &d);
        assert_eq!(cols(&row_text(&buf, 0), 46..60), "/tmp/demo-repo");
        assert!(
            cols(&row_text(&buf, 0), 8..46).chars().all(|c| c == ' '),
            "columns 8..46 must be exactly spaces"
        );

        let buf = render_at(120, 20, &d);
        assert_eq!(cols(&row_text(&buf, 0), 106..120), "/tmp/demo-repo");
    }

    #[test]
    fn header_path_shortened_from_the_left() {
        let repo = "/home/dev/workspaces/openspec-demos/a-rather-long-repository-name-here";
        assert_eq!(repo.chars().count(), 70);
        let d = dashboard(Some(repo), Route::List);

        let buf = render_at(60, 20, &d);
        assert_eq!(
            cols(&row_text(&buf, 0), 9..60),
            "…/openspec-demos/a-rather-long-repository-name-here"
        );

        let buf = render_at(120, 20, &d);
        assert_eq!(cols(&row_text(&buf, 0), 50..120), repo);
        assert!(!buffer_contains(&buf, "…"));
    }

    #[test]
    fn header_omits_the_path_when_too_narrow() {
        let repo = "/home/dev/workspaces/openspec-demos/a-rather-long-repository-name-here";
        let d = dashboard(Some(repo), Route::List);

        let buf = render_at(16, 20, &d);
        assert_eq!(row_text(&buf, 0), format!("OpenSpec{}", " ".repeat(8)));
        assert!(!buffer_contains(&buf, "…"));

        let buf = render_at(60, 20, &d);
        assert_eq!(
            cols(&row_text(&buf, 0), 9..60),
            "…/openspec-demos/a-rather-long-repository-name-here"
        );

        let buf = render_at(120, 20, &d);
        assert_eq!(cols(&row_text(&buf, 0), 50..120), repo);
    }

    #[test]
    fn header_says_no_repository() {
        // Rewritten for list-view: the landed assertion checked the WHOLE
        // buffer for the searched-from path's absence. change-rows' own
        // no-repository block now names it on purpose (row 2 of the body),
        // so the assertion narrows to row 0 — the header row — which is
        // what this requirement was ever about.
        let d = dashboard(None, Route::List);
        let buf = render_at(60, 20, &d);
        assert_eq!(cols(&row_text(&buf, 0), 47..60), "no repository");
        assert!(!row_text(&buf, 0).contains("/tmp/searched-from"));
        assert!(row_text(&buf, 4).contains("/tmp/searched-from"));

        let buf = render_at(120, 20, &d);
        assert_eq!(cols(&row_text(&buf, 0), 107..120), "no repository");
        assert!(!row_text(&buf, 0).contains("/tmp/searched-from"));
        assert!(row_text(&buf, 4).contains("/tmp/searched-from"));
    }

    /// `responsive-layout` :: "The badge is drawn dim after the label at both widths" —
    /// reads the buffer's `Modifier`, not only its characters, since a badge that renders
    /// the right text with the wrong style is still a defect a plain string comparison would
    /// miss.
    #[test]
    fn file_mode_badge_is_dim_after_the_label() {
        for width in [120, 60] {
            let d = dashboard_in_file_mode(Some("/tmp/demo-repo"), Route::List);
            let buf = render_at(width, 20, &d);
            assert_eq!(
                cols(&row_text(&buf, 0), 9..18),
                "file mode",
                "width {width}"
            );
            for x in 9..18u16 {
                assert!(
                    cell(&buf, x, 0)
                        .style()
                        .add_modifier
                        .contains(Modifier::DIM),
                    "width {width}: column {x} of the badge is not dim"
                );
            }
            // Discriminating control: column 8 (the separator) and the OpenSpec label itself
            // must not carry DIM, so the assertion above is not satisfied by the whole row
            // being dim.
            assert!(
                !cell(&buf, 0, 0)
                    .style()
                    .add_modifier
                    .contains(Modifier::DIM),
                "width {width}: the OpenSpec label must not be dim"
            );
        }
    }

    /// `responsive-layout` :: "A false flag renders the header that landed before this
    /// change" — whole-buffer equality at both widths, with a discriminating control (the
    /// same fixture with `file_mode` true) so the equality is not satisfied by two blank
    /// headers.
    #[test]
    fn no_badge_is_byte_identical_to_the_landed_header() {
        for width in [120, 60] {
            let plain = dashboard(Some("/tmp/demo-repo"), Route::List);
            let buf = render_at(width, 20, &plain);
            assert_eq!(
                row_text(&buf, 0),
                format!(
                    "OpenSpec{}/tmp/demo-repo",
                    " ".repeat(width as usize - 8 - "/tmp/demo-repo".chars().count())
                ),
                "width {width}: the landed header must be unchanged"
            );

            let badged = dashboard_in_file_mode(Some("/tmp/demo-repo"), Route::List);
            let badged_buf = render_at(width, 20, &badged);
            assert_ne!(
                row_text(&buf, 0),
                row_text(&badged_buf, 0),
                "width {width}: the control must render differently"
            );
        }
    }

    /// `responsive-layout` :: "The badge takes its columns from the path, not from the
    /// label" — a path long enough to be shortened differently by the two budgets (`width -
    /// 9` without the badge, `width - 19` with it) at BOTH mandated widths.
    #[test]
    fn badge_rebases_the_shortening_arithmetic() {
        let long_path = format!("/repo/{}", "x".repeat(99));
        assert_eq!(long_path.chars().count(), 105);

        for width in [120, 60] {
            let plain = dashboard(Some(&long_path), Route::List);
            let plain_buf = render_at(width, 20, &plain);
            let plain_a = width as usize - 9;
            let plain_shown = crate::ui::list::shorten_left(&long_path, plain_a);
            assert_eq!(
                cols(
                    &row_text(&plain_buf, 0),
                    (width as usize - plain_shown.chars().count())..width as usize
                ),
                plain_shown,
                "width {width}: no-badge path"
            );

            let badged = dashboard_in_file_mode(Some(&long_path), Route::List);
            let badged_buf = render_at(width, 20, &badged);
            let badged_a = width as usize - 19;
            let badged_shown = crate::ui::list::shorten_left(&long_path, badged_a);
            assert_eq!(
                cols(
                    &row_text(&badged_buf, 0),
                    (width as usize - badged_shown.chars().count())..width as usize
                ),
                badged_shown,
                "width {width}: badged path"
            );
            assert_ne!(
                plain_shown, badged_shown,
                "width {width}: the two budgets must actually differ for this fixture"
            );
        }
    }

    /// `responsive-layout` :: "A header too narrow for the badge drops it whole" — rendered
    /// at 17, 18, 60, and 120: below 18 the badge is dropped before the path's own
    /// shortening arithmetic ever runs; at 18 and above it is drawn.
    #[test]
    fn badge_drops_whole_below_eighteen_columns() {
        for width in [17, 18, 60, 120] {
            let d = dashboard_in_file_mode(Some("/tmp/demo-repo"), Route::List);
            let buf = render_at(width, 20, &d);
            let row = row_text(&buf, 0);
            if width < 18 {
                assert!(
                    !row.contains("file mode"),
                    "width {width}: the badge must be dropped whole below 18 columns: {row:?}"
                );
            } else {
                assert_eq!(cols(&row, 9..18), "file mode", "width {width}");
                for x in 9..18u16 {
                    assert!(
                        cell(&buf, x, 0)
                            .style()
                            .add_modifier
                            .contains(Modifier::DIM),
                        "width {width}: column {x} of the badge is not dim"
                    );
                }
            }
        }
    }

    /// Column range `range` of buffer row `y`, read **cell by cell** rather than through
    /// `row_text`/`cols`: a CJK cluster occupies two buffer columns but the row-text
    /// reconstruction folds its trailing (shadow) cell to an empty string, so a `char`-index
    /// slice of that reconstruction no longer lines up with real buffer columns once any
    /// cell is more than one column wide. `view-fidelity`'s own wide-character scenarios use
    /// this instead of `cols(&row_text(...), ..)` for exactly that reason.
    fn cell_range(buf: &Buffer, y: u16, range: std::ops::Range<u16>) -> String {
        range
            .map(|x| cell(buf, x, y).symbol().to_string())
            .collect()
    }

    /// `cell_range`, with every wide cluster's trailing shadow cell — reset to a plain
    /// space by ratatui itself — dropped. Safe whenever the expected text is known to
    /// contain no real space of its own, which every fixture this helper is used against
    /// does not.
    fn cell_range_no_shadow(buf: &Buffer, y: u16, range: std::ops::Range<u16>) -> String {
        cell_range(buf, y, range)
            .chars()
            .filter(|c| *c != ' ')
            .collect()
    }

    /// `responsive-layout` :: "A wide-character path is shortened by columns and stays
    /// inside the header" — the fixture (44 characters, 67 display columns) is chosen to
    /// exceed `A` at 60 columns both badged (41) and unbadged (51), so both branches
    /// actually shorten. Every check reads the buffer cell-by-cell (`cell_range`), never by
    /// `chars()`-indexed string slicing, because the fixture is exactly the content that
    /// slicing gets wrong.
    #[test]
    fn header_wide_character_path_shortens_by_columns_and_stays_inside_the_header() {
        let repo = "/home/dev/workspaces/日本語のリポジトリ名前がとても長いディレクトリ";
        assert_eq!(repo.chars().count(), 44, "fixture must be 44 characters");
        assert_eq!(columns(repo), 67, "fixture must be 67 display columns");

        let plain = dashboard(Some(repo), Route::List);
        let badged = dashboard_in_file_mode(Some(repo), Route::List);

        // The 60-column, non-badged buffer: A is 51. The shortened text begins with `…`
        // no earlier than column 9, ends in the final column, and its `columns` is at
        // most 51.
        let a_unbadged = 51usize;
        let expected_unbadged = crate::ui::list::shorten_left(repo, a_unbadged);
        assert!(expected_unbadged.starts_with('…'));
        assert!(columns(&expected_unbadged) <= a_unbadged);
        let buf = render_at(60, 20, &plain);
        let start = 60u16 - columns(&expected_unbadged) as u16;
        assert!(
            start >= 9,
            "ellipsis must start no earlier than column 9: {start}"
        );
        assert_eq!(
            cell_range_no_shadow(&buf, 0, start..60),
            expected_unbadged,
            "the shortened text must be right-aligned against the final column"
        );

        // Discriminating clause: a `char`-counted shortening would keep the last `A - 1`
        // **characters** of the path rather than the last `A - 1` **columns**. Since the
        // path is only 44 characters long — fewer than the 50 characters such a rule would
        // try to keep — a char-counted rule keeps the WHOLE path, which measures 67
        // columns: far more than the 51-column budget, and it would have run past the
        // frame.
        let char_based_keep = a_unbadged - 1;
        let total_chars = repo.chars().count();
        let start_char = total_chars.saturating_sub(char_based_keep);
        let char_based_shown: String = repo.chars().skip(start_char).collect();
        assert!(
            columns(&char_based_shown) > a_unbadged,
            "a char-counted shortening keeps {char_based_shown:?} at {} columns, which must \
             exceed the {a_unbadged}-column budget for this fixture to discriminate",
            columns(&char_based_shown)
        );

        // The 60-column, badged buffer: the badge takes columns 9..18, column 18 is a
        // blank separator, and A is 41.
        let a_badged = 41usize;
        let expected_badged = crate::ui::list::shorten_left(repo, a_badged);
        assert!(columns(&expected_badged) <= a_badged);
        let buf = render_at(60, 20, &badged);
        assert_eq!(cell_range(&buf, 0, 9..18), "file mode");
        assert_eq!(cell_range(&buf, 0, 18..19), " ");
        let start = 60u16 - columns(&expected_badged) as u16;
        assert_eq!(cell_range_no_shadow(&buf, 0, start..60), expected_badged);

        // The 120-column buffer: A (111 unbadged, 101 badged) comfortably holds the whole
        // 67-column path, so it is drawn whole with no ellipsis, starting no earlier than
        // column 50.
        for d in [&plain, &badged] {
            let buf = render_at(120, 20, d);
            let start = 120u16 - columns(repo) as u16;
            assert!(
                start >= 50,
                "path must start no earlier than column 50: {start}"
            );
            assert_eq!(cell_range_no_shadow(&buf, 0, start..120), repo);
            assert!(
                !row_text(&buf, 0).contains('…'),
                "no ellipsis at 120 columns"
            );
        }

        // 16, 18, 19, and 1 columns: rendering never panics, and the row never exceeds
        // the frame — `row_text` itself is exactly `width` cells wide by construction, so
        // the real assertion here is simply that render_at returns without panicking.
        // Below A=8 (widths 16 and 1, where A is 7 and 0) the label alone is drawn and no
        // ellipsis appears at all; at 18 and 19 (A 9 and 10) the path is shortened same as
        // at 60, so an ellipsis is expected there too.
        for width in [16u16, 1] {
            let buf = render_at(width, 20, &plain);
            assert!(
                !row_text(&buf, 0).contains('…'),
                "width {width}: below A=8, no ellipsis is drawn at all"
            );
        }
        for width in [18u16, 19] {
            let buf = render_at(width, 20, &plain);
            assert!(
                row_text(&buf, 0).contains('…'),
                "width {width}: A is 9 or 10, so the path is shortened with an ellipsis"
            );
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
            0,
            Route::List,
        )
    }

    #[test]
    fn list_rows_render_at_60_and_120() {
        let d = three_active();

        let buf120 = render_at(120, 20, &d);
        assert_eq!(
            interior_cols(&buf120, 2),
            "> add-token-refresh              [4/9]"
        );
        assert_eq!(
            interior_cols(&buf120, 3),
            "  fix-empty-basket               [7/7]"
        );
        assert_eq!(
            interior_cols(&buf120, 4),
            "  migrate-ai-sdk-v7                [-]"
        );
        for y in 5..=17u16 {
            assert!(cols(&row_text(&buf120, y), 1..39).chars().all(|c| c == ' '));
        }

        let buf60 = render_at(60, 20, &d);
        assert_eq!(
            interior_cols(&buf60, 2),
            "> add-token-refresh                                  [4/9]"
        );
        assert_eq!(
            interior_cols(&buf60, 3),
            "  fix-empty-basket                                   [7/7]"
        );
        assert_eq!(
            interior_cols(&buf60, 4),
            "  migrate-ai-sdk-v7                                    [-]"
        );
        for y in 5..=17u16 {
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
        let buf120 = render_at(120, 20, &d);
        let row2 = interior_cols(&buf120, 2);
        assert!(row2.contains('…'));
        assert!(row2.contains("[2/5]"));

        let buf60 = render_at(60, 20, &d);
        let row2_60 = interior_cols(&buf60, 2);
        assert!(row2_60.contains("a-very-long-change-name-that-will-not-fit-here"));
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
        for (width, border_x, buf, control) in [
            (120u16, 39u16, &buf120, &control120),
            (60u16, 59u16, &buf60, &control60),
        ] {
            assert_eq!(
                cell(buf, border_x, 2).symbol(),
                "│",
                "width {width}: the list block's own right border must be intact"
            );
            assert_eq!(
                cell(buf, border_x, 2).symbol(),
                cell(control, border_x, 2).symbol(),
                "width {width}: the border must be unmoved from the ASCII-named control"
            );
            assert_eq!(
                cell(buf, border_x - 1, 2).symbol(),
                "]",
                "width {width}: the progress cell ends in the interior's last column"
            );
        }
    }

    #[test]
    fn separator_and_archived_rows_render_at_both_widths() {
        let d = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            vec![
                fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                fixture::archived(None, "legacy-cleanup", 3, 3),
            ],
            usize::MAX,
            Route::List,
        );
        let buf120 = render_at(120, 20, &d);
        assert_eq!(
            interior_cols(&buf120, 2),
            "  fix-empty-basket               [7/7]"
        );
        assert_eq!(
            interior_cols(&buf120, 3),
            "  -- archived ------------------------"
        );
        assert_eq!(
            interior_cols(&buf120, 4),
            "  2026-08-14 add-auth            [7/7]"
        );
        assert_eq!(
            interior_cols(&buf120, 5),
            "             legacy-cleanup      [3/3]"
        );

        let buf60 = render_at(60, 20, &d);
        assert_eq!(
            interior_cols(&buf60, 2),
            "  fix-empty-basket                                   [7/7]"
        );
        assert_eq!(
            interior_cols(&buf60, 3),
            "  -- archived --------------------------------------------"
        );
        assert_eq!(
            interior_cols(&buf60, 4),
            "  2026-08-14 add-auth                                [7/7]"
        );
        assert_eq!(
            interior_cols(&buf60, 5),
            "             legacy-cleanup                          [3/3]"
        );
    }

    #[test]
    fn no_archived_changes_means_no_separator() {
        let d = three_active();
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(!buffer_contains(&buf, "-- archived"));
        }
        let with_archived = dashboard_with(
            vec![
                fixture::active("add-token-refresh", 4, 9),
                fixture::active("fix-empty-basket", 7, 7),
                fixture::active("migrate-ai-sdk-v7", 0, 0),
            ],
            vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
            0,
            Route::List,
        );
        for width in [60, 120] {
            let buf = render_at(width, 20, &with_archived);
            assert!(buffer_contains(&buf, "-- archived"));
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
            file_mode: false,
        };
        let buf120 = render_at(120, 20, &d);
        assert!(interior_cols(&buf120, 2).starts_with("No OpenSpec repository found"));
        assert!(interior_cols(&buf120, 3).starts_with("searched from:"));
        assert!(interior_cols(&buf120, 4).starts_with("…os/a-rather-long-repository-name-here"));
        assert_eq!(cell(&buf120, 0, 1).symbol(), "┌");
        assert_eq!(cols(&row_text(&buf120, 1), 1..8), "Changes");

        let buf60 = render_at(60, 20, &d);
        assert!(interior_cols(&buf60, 2).starts_with("No OpenSpec repository found"));
        assert!(interior_cols(&buf60, 3).starts_with("searched from:"));
        assert!(
            interior_cols(&buf60, 4)
                .starts_with("…kspaces/openspec-demos/a-rather-long-repository-name-here")
        );
        assert_eq!(cell(&buf60, 0, 1).symbol(), "┌");
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
    fn no_active_changes_keeps_archived_browsable() {
        let d = dashboard_with(
            Vec::new(),
            vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
            0,
            Route::List,
        );
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(interior_cols(&buf, 2).starts_with("No active changes"));
            assert!(interior_cols(&buf, 3).contains("-- archived"));
            assert!(interior_cols(&buf, 4).contains("add-auth"));
            assert!(!buffer_contains(&buf, "No changes yet"));
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

        let buf120 = render_at(120, 20, &d);
        assert_eq!(
            interior_cols(&buf120, 2),
            "! openspec/changes: Permission denied…"
        );
        assert!(interior_cols(&buf120, 3).contains("fix-empty-basket"));

        let buf60 = render_at(60, 20, &d);
        assert!(
            interior_cols(&buf60, 2)
                .starts_with("! openspec/changes: Permission denied (os error 13)")
        );
        assert!(!interior_cols(&buf60, 2).contains('…'));
        assert!(interior_cols(&buf60, 3).contains("fix-empty-basket"));
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
            assert!(
                interior_cols(&buf, 3).contains("fix-empty-basket"),
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
            assert!(
                interior_cols(&buf, 4).contains("fix-empty-basket"),
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
            assert!(
                interior_cols(&buf, 2).contains("fix-empty-basket"),
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
            assert!(interior_cols(&buf, 2).contains("[7/9]"), "width {width}");
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
            assert!(
                interior_cols(&buf, 3).contains("alpha"),
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
        let d = three_active();
        let buf120 = render_at(120, 20, &d);
        assert!(detail_interior_cols(&buf120, 2, 78).contains("add-token-refresh"));

        let empty = dashboard_with(Vec::new(), Vec::new(), 0, Route::List);
        let default_style = Cell::default().style();
        let buf_empty = render_at(120, 20, &empty);
        for y in 2..=17u16 {
            for x in 41..=118u16 {
                let c = cell(&buf_empty, x, y);
                assert_eq!(c.symbol(), " ", "x={x} y={y}");
                assert_eq!(c.style(), default_style, "x={x} y={y}");
            }
        }

        let buf60 = render_at(60, 20, &d);
        for y in 2..=17u16 {
            assert!(matches!(cell(&buf60, 0, y).symbol(), "│" | "┌" | "└"));
            assert!(matches!(cell(&buf60, 59, y).symbol(), "│" | "┐" | "┘"));
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
            0,
            route,
        )
    }

    #[test]
    fn more_changes_than_rows_do_not_overflow() {
        let d = dashboard_with(changes_named(30), Vec::new(), 0, Route::List);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(interior_cols(&buf, 2).contains("change-00"));
            assert!(interior_cols(&buf, 17).contains("change-15"));
            for y in [0u16, 1, 18, 19] {
                assert!(!row_text(&buf, y).contains("change-"));
            }
        }
    }

    #[test]
    fn the_selected_row_carries_the_marker_and_bold() {
        let d = three_active();
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            let last = if width == 60 { 58 } else { 38 };
            assert_eq!(cell(&buf, 1, 2).symbol(), ">");
            for x in 1..=last {
                assert!(is_bold(cell(&buf, x, 2)), "x={x} width={width}");
            }
            assert!(!is_bold(cell(&buf, 1, 3)));
        }
    }

    #[test]
    fn navigation_moves_the_marker() {
        let mut d = three_active();
        d.apply(Action::Next);
        d.apply(Action::Next);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert_eq!(cell(&buf, 1, 4).symbol(), ">");
            assert!(is_bold(cell(&buf, 1, 4)));
        }
    }

    #[test]
    fn selection_clamps_at_both_ends_on_screen() {
        let mut d = three_active();
        for _ in 0..4 {
            d.apply(Action::Next);
        }
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert_eq!(cell(&buf, 1, 4).symbol(), ">");
        }
    }

    #[test]
    fn selection_crosses_the_separator() {
        let mut d = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            vec![
                fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                fixture::archived(None, "legacy-cleanup", 3, 3),
            ],
            0,
            Route::List,
        );
        d.apply(Action::Next);
        d.apply(Action::Next);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(interior_cols(&buf, 5).contains("legacy-cleanup"));
            assert_eq!(cell(&buf, 1, 3).symbol(), " ");
            assert!(!is_bold(cell(&buf, 1, 3)));
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
        let d = dashboard_with(changes_named(30), Vec::new(), 20, Route::List);
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
        let d = dashboard_with(changes_named(30), Vec::new(), 29, Route::List);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(interior_cols(&buf, 2).contains("change-14"));
            assert!(interior_cols(&buf, 17).contains("change-29"));
            assert_eq!(cell(&buf, 1, 17).symbol(), ">");
            for y in 2..=17u16 {
                assert!(!interior_cols(&buf, y).chars().all(|c| c == ' '));
            }
        }
    }

    #[test]
    fn the_viewport_boundary_is_rendered() {
        let d = dashboard_with(changes_named(17), Vec::new(), 9, Route::List);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(interior_cols(&buf, 2).contains("change-01"));
        }
    }

    #[test]
    fn resizing_changes_the_slice_on_the_next_frame() {
        let d = dashboard_with(changes_named(30), Vec::new(), 20, Route::List);
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

        let buf60 = render_at(60, 20, &d);
        for y in 1..=18u16 {
            assert!(matches!(cell(&buf60, 0, y).symbol(), "│" | "┌" | "└"));
            assert!(matches!(cell(&buf60, 59, y).symbol(), "│" | "┐" | "┘"));
        }

        let buf120 = render_at(120, 20, &d);
        for y in 1..=18u16 {
            for x in [0u16, 39, 40, 119] {
                let s = cell(&buf120, x, y).symbol();
                assert!(
                    matches!(s, "│" | "┌" | "└" | "┐" | "┘"),
                    "x={x} y={y} symbol={s:?}"
                );
            }
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
        assert_eq!(expected.chars().count(), 36);
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
        assert_eq!(with_count.chars().count(), 52);
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
        assert_eq!(with_hints_and_count.chars().count(), 75);
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
        assert_eq!(row60.chars().count(), 60);
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
        let mut d = five_change_dashboard(0);
        d.filter.query = "add".to_string();
        let buf120 = render_at(120, 20, &d);
        assert_eq!(
            interior_cols(&buf120, 2),
            "> add-token-refresh              [4/9]"
        );
        assert_eq!(
            interior_cols(&buf120, 3),
            "  -- archived ------------------------"
        );
        assert_eq!(
            interior_cols(&buf120, 4),
            "  2026-08-14 add-auth            [7/7]"
        );
        let buf60 = render_at(60, 20, &d);
        assert_eq!(
            interior_cols(&buf60, 2),
            "> add-token-refresh                                  [4/9]"
        );
        assert_eq!(
            interior_cols(&buf60, 3),
            "  -- archived --------------------------------------------"
        );
        assert_eq!(
            interior_cols(&buf60, 4),
            "  2026-08-14 add-auth                                [7/7]"
        );
        for buf in [&buf120, &buf60] {
            assert!(!buffer_contains(buf, "fix-empty-basket"));
            assert!(!buffer_contains(buf, "migrate-ai-sdk-v7"));
            assert!(!buffer_contains(buf, "legacy-cleanup"));
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
        let mut d = five_change_dashboard(0);
        d.filter.query = "auth".to_string();
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(interior_cols(&buf, 2).starts_with("No active changes"));
            assert!(interior_cols(&buf, 3).contains("-- archived"));
            assert!(interior_cols(&buf, 4).contains("add-auth"));
            assert_eq!(cell(&buf, 1, 4).symbol(), ">");
            assert!(!buffer_contains(&buf, "No changes match"));
            assert!(!buffer_contains(&buf, "No changes yet"));
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
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(interior_cols(&buf, 4).contains("add-auth"));
            assert_eq!(cell(&buf, 1, 4).symbol(), ">");
        }
    }

    #[test]
    fn slash_starts_filter_mode_and_the_list_is_shown() {
        let mut d = five_change_dashboard(0);
        d.route = Route::Detail;
        d.apply(Action::FilterStart);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert!(buffer_contains(&buf, "Changes"));
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
            selected: 0,
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
        // 60-column frame: the detail interior is 58 columns, content starts at row 4,
        // column 1, and the frame's own right border sits at column 59.
        let d58 = detail_dashboard(combining_mark_overrun_source(58), 0, Route::Detail);
        let buf58 = render_at(60, 20, &d58);
        for x in 1..50u16 {
            assert_eq!(cell(&buf58, x, 4).symbol(), "a", "x={x}");
        }
        assert!(
            cell(&buf58, 50, 4).symbol().starts_with('a'),
            "the last cluster of the plain run carries the combining marks: {:?}",
            cell(&buf58, 50, 4).symbol()
        );
        assert!(
            row_text(&buf58, 4).contains("OVERRUN!"),
            "the bold segment must still be drawn, just inside the border: {:?}",
            row_text(&buf58, 4)
        );
        assert_eq!(
            cell(&buf58, 59, 4).symbol(),
            "│",
            "the frame's right border must survive the over-counted segment"
        );

        // 120-column frame: the detail interior is 78 columns, content starts at column
        // 41, and the frame's own right border sits at column 119.
        let d78 = detail_dashboard(combining_mark_overrun_source(78), 0, Route::Detail);
        let buf78 = render_at(120, 20, &d78);
        for x in 41..110u16 {
            assert_eq!(cell(&buf78, x, 4).symbol(), "a", "x={x}");
        }
        assert!(cell(&buf78, 110, 4).symbol().starts_with('a'));
        assert!(row_text(&buf78, 4).contains("OVERRUN!"));
        assert_eq!(
            cell(&buf78, 119, 4).symbol(),
            "│",
            "the frame's right border must survive the over-counted segment"
        );
    }

    /// Column range `1..=9` at 60, `41..=49` at 120 — the first nine
    /// columns of the detail interior at either mandated width.
    fn detail_marker_cols(buf: &Buffer, y: u16) -> String {
        let (from, to) = if buf.area.width == 60 {
            (1, 9)
        } else {
            (41, 49)
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
            file_mode: false,
        };

        let buf120 = render_at(120, 20, &d);
        assert_eq!(detail_marker_cols(&buf120, 4), "- line-00");
        assert_eq!(detail_marker_cols(&buf120, 17), "- line-13");
        assert_eq!(
            cols(&row_text(&buf120, 2), 1..39),
            "> fix-empty-basket               [7/7]"
        );

        d.route = Route::Detail;
        let buf60 = render_at(60, 20, &d);
        assert_eq!(detail_marker_cols(&buf60, 4), "- line-00");
        assert_eq!(detail_marker_cols(&buf60, 17), "- line-13");
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
        let source =
            "# Title\n\nA **bold** and *italic* line with `code` and [a link](x).\n\n> quoted\n";
        let d = detail_dashboard(source.to_string(), 0, Route::Detail);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);

            let title_row: Vec<char> = row_text(&buf, 4).chars().collect();
            let needle: Vec<char> = "# Title".chars().collect();
            let title_start = title_row
                .windows(needle.len())
                .position(|w| w == needle.as_slice())
                .expect("heading present");
            for i in 0..7 {
                let x = (title_start + i) as u16;
                assert!(
                    cell(&buf, x, 4)
                        .style()
                        .add_modifier
                        .contains(Modifier::BOLD),
                    "width {width}: heading cell {i} not bold"
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
        }
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
            file_mode: false,
        };
        let default_style = Cell::default().style();

        let buf120 = render_at(120, 20, &no_change);
        for y in 2..=17u16 {
            for x in 41..=118u16 {
                let c = cell(&buf120, x, y);
                assert_eq!(c.symbol(), " ", "x={x} y={y}");
                assert_eq!(c.style(), default_style, "x={x} y={y}");
            }
        }

        let buf60 = render_at(60, 20, &no_change);
        for y in 2..=17u16 {
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
                detail_interior_cols(&buf, 4, 14).starts_with("No content yet"),
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
            0,
            Route::Detail,
            empty_detail_with_tab("", Vec::new(), 0),
        );
        for width in [120, 60] {
            let buf = render_at(width, 20, &with_marked_change);
            assert!(
                detail_interior_cols(&buf, 4, 14).starts_with("No content yet"),
                "width {width}: marked-but-empty"
            );
            assert!(
                !row_text(&buf, 4).contains('█') && !row_text(&buf, 4).contains('░'),
                "width {width}: no progress bar"
            );
        }
    }

    #[test]
    fn detail_content_never_overwrites_the_border() {
        let source: String = (0..20).map(|_| format!("{}\n", "x".repeat(200))).collect();
        let d = detail_dashboard(source, 0, Route::Detail);

        let buf120 = render_at(120, 20, &d);
        for y in 1..=18u16 {
            for x in [39u16, 40, 119] {
                let s = cell(&buf120, x, y).symbol();
                assert!(
                    matches!(s, "│" | "┌" | "└" | "┐" | "┘"),
                    "x={x} y={y}: {s:?}"
                );
            }
        }
        for y in 2..=17u16 {
            assert!(
                !cols(&row_text(&buf120, y), 41..119)
                    .chars()
                    .all(|c| c == ' ')
            );
        }

        let buf60 = render_at(60, 20, &d);
        for y in 1..=18u16 {
            for x in [0u16, 59] {
                let s = cell(&buf60, x, y).symbol();
                assert!(
                    matches!(s, "│" | "┌" | "└" | "┐" | "┘"),
                    "x={x} y={y}: {s:?}"
                );
            }
        }
        for y in 2..=17u16 {
            assert!(!cols(&row_text(&buf60, y), 1..59).chars().all(|c| c == ' '));
        }

        // The same holds with the selected tab marked `tracks_tasks` and
        // the same source turned into thirty 200-character task lines
        // under a 200-character heading — neither the checklist's wrap,
        // the heading's truncation, nor the progress bar's gauge can
        // reach the border. Found in Change Review: the heading line
        // originally had no width treatment at all.
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
            0,
            Route::Detail,
            empty_detail_with_tab(&task_source, Vec::new(), 0),
        );

        let mbuf120 = render_at(120, 20, &marked_d);
        for y in 1..=18u16 {
            for x in [39u16, 40, 119] {
                let s = cell(&mbuf120, x, y).symbol();
                assert!(
                    matches!(s, "│" | "┌" | "└" | "┐" | "┘"),
                    "marked x={x} y={y}: {s:?}"
                );
            }
        }
        let mbuf60 = render_at(60, 20, &marked_d);
        for y in 1..=18u16 {
            for x in [0u16, 59] {
                let s = cell(&mbuf60, x, y).symbol();
                assert!(
                    matches!(s, "│" | "┌" | "└" | "┐" | "┘"),
                    "marked x={x} y={y}: {s:?}"
                );
            }
        }
    }

    #[test]
    fn a_degenerate_detail_interior_draws_nothing() {
        // The frame height a detail interior row costs is four — one
        // frame-header row, one frame-footer row, and the region's two
        // border rows — so the interior first has one row at a frame
        // height of 5, two at 6, and three at 7; heights 3 and 4 give a
        // zero-row interior and exercise only the earliest guard.
        // `detail-scroll` -> "A degenerate detail interior draws nothing
        // and does not panic".
        let d = detail_dashboard(twenty_line_source(), 0, Route::Detail);

        // 1x20 and 2x20: no render panics; the spec states no further
        // assertion at these degenerate widths.
        for (w, h) in [(1u16, 20u16), (2, 20)] {
            let buf = render_at(w, h, &d);
            let _ = buf;
        }

        // 120x4 and 60x4: the interior has zero rows, so nothing at all —
        // not the header, not a tab cell, not a markdown line — is drawn
        // inside the region.
        for width in [120u16, 60] {
            let buf = render_at(width, 4, &d);
            assert!(!buffer_contains(&buf, "detail-view"), "width {width}");
            assert!(!buffer_contains(&buf, "1 proposal"), "width {width}");
            assert!(!buffer_contains(&buf, "line-00"), "width {width}");
        }

        // 120x5 and 60x5: the header row is drawn; no tab cell and no
        // markdown line appears anywhere in the frame.
        for width in [120u16, 60] {
            let buf = render_at(width, 5, &d);
            assert!(buffer_contains(&buf, "detail-view"), "width {width}");
            assert!(!buffer_contains(&buf, "1 proposal"), "width {width}");
            assert!(!buffer_contains(&buf, "line-00"), "width {width}");
        }

        // 120x6 and 60x6: the header row and the tab bar are drawn; no
        // markdown line appears.
        for width in [120u16, 60] {
            let buf = render_at(width, 6, &d);
            assert!(buffer_contains(&buf, "detail-view"), "width {width}");
            assert!(buffer_contains(&buf, "1 proposal"), "width {width}");
            assert!(!buffer_contains(&buf, "line-00"), "width {width}");
        }

        // 120x7 and 60x7: exactly one content row is drawn, holding the
        // source's first rendered line.
        for width in [120u16, 60] {
            let buf = render_at(width, 7, &d);
            assert!(buffer_contains(&buf, "detail-view"), "width {width}");
            assert!(buffer_contains(&buf, "1 proposal"), "width {width}");
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
            0,
            Route::Detail,
            empty_detail_with_tab(&task_source, Vec::new(), 0),
        );

        for (w, h) in [(1u16, 20u16), (2, 20)] {
            let buf = render_at(w, h, &md);
            let _ = buf;
        }

        for width in [120u16, 60] {
            let buf = render_at(width, 4, &md);
            assert!(
                !buffer_contains(&buf, "detail-view"),
                "marked width {width}"
            );
            assert!(!buffer_contains(&buf, "1 proposal"), "marked width {width}");
            assert!(!buffer_contains(&buf, "line-00"), "marked width {width}");
        }

        for width in [120u16, 60] {
            let buf = render_at(width, 5, &md);
            assert!(buffer_contains(&buf, "detail-view"), "marked width {width}");
            assert!(!buffer_contains(&buf, "1 proposal"), "marked width {width}");
            assert!(!buffer_contains(&buf, "line-00"), "marked width {width}");
        }

        for width in [120u16, 60] {
            let buf = render_at(width, 6, &md);
            assert!(buffer_contains(&buf, "detail-view"), "marked width {width}");
            assert!(buffer_contains(&buf, "1 proposal"), "marked width {width}");
            assert!(!buffer_contains(&buf, "line-00"), "marked width {width}");
        }

        for width in [120u16, 60] {
            let buf = render_at(width, 7, &md);
            assert!(buffer_contains(&buf, "detail-view"), "marked width {width}");
            assert!(buffer_contains(&buf, "1 proposal"), "marked width {width}");
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
    }

    #[test]
    fn a_scroll_past_the_end_draws_the_last_screenful() {
        let d = detail_dashboard(twenty_line_source(), 99, Route::Detail);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert_eq!(detail_marker_cols(&buf, 4), "- line-06", "width {width}");
            assert_eq!(detail_marker_cols(&buf, 17), "- line-19", "width {width}");
        }
    }

    #[test]
    fn scrolling_moves_the_detail_content() {
        let mut d = detail_dashboard(twenty_line_source(), 0, Route::Detail);
        d.apply(Action::Next);
        d.apply(Action::Next);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert_eq!(detail_marker_cols(&buf, 4), "- line-02", "width {width}");
            assert_eq!(detail_marker_cols(&buf, 17), "- line-15", "width {width}");
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
            file_mode: false,
        };
        d.apply(Action::Next);
        d.apply(Action::Next);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert_eq!(cell(&buf, 1, 4).symbol(), ">", "width {width}");
        }
        // The detail content, when drawn (wide layout only), is unmoved.
        let buf120 = render_at(120, 20, &d);
        assert_eq!(detail_marker_cols(&buf120, 4), "- line-00");
    }

    #[test]
    fn scrolling_stops_at_the_top_on_screen() {
        let mut d = detail_dashboard(twenty_line_source(), 0, Route::Detail);
        for _ in 0..4 {
            d.apply(Action::Prev);
        }
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert_eq!(detail_marker_cols(&buf, 4), "- line-00", "width {width}");
        }
    }

    #[test]
    fn route_moves_reset_the_scroll_on_screen() {
        let mut d = detail_dashboard(twenty_line_source(), 3, Route::Detail);
        d.apply(Action::Back);
        d.apply(Action::OpenDetail);
        for width in [60, 120] {
            let buf = render_at(width, 20, &d);
            assert_eq!(detail_marker_cols(&buf, 4), "- line-00", "width {width}");
        }
    }

    /// Column range `1..=to` at 60, `41..=(40+to)` at 120 — the detail
    /// interior's first `to` columns of row `y`, at either mandated width.
    fn detail_interior_cols(buf: &Buffer, y: u16, to: usize) -> String {
        let from = if buf.area.width == 60 { 1 } else { 41 };
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
            0,
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
                detail_interior_cols(&buf, 2, w as usize),
                expected,
                "width {width}"
            );
            let last_col = if width == 60 { 58u16 } else { 118 };
            for x in (last_col - 4)..=last_col {
                assert!(
                    cell(&buf, x, 2)
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
            0,
            Route::Detail,
        );
        let buf_first = render_at(120, 20, &d);
        assert!(detail_interior_cols(&buf_first, 2, 78).contains("add-token-refresh"));

        d.selected = 1;
        for (width, w) in [(120, 78), (60, 58)] {
            let buf = render_at(width, 20, &d);
            let header = detail_interior_cols(&buf, 2, w);
            assert!(header.contains("fix-empty-basket"), "width {width}");
            assert!(header.contains("[7/7]"), "width {width}");
            assert!(!header.contains("add-token-refresh"), "width {width}");
        }
    }

    #[test]
    fn an_archived_changes_header_carries_its_stripped_name_and_its_own_schema() {
        let archived_change = fixture::with_schema(
            fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
            "spec-driven",
        );
        let d = dashboard_with(Vec::new(), vec![archived_change], 0, Route::Detail);
        for width in [120, 60] {
            let buf = render_at(width, 20, &d);
            let row = row_text(&buf, 2);
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
        for y in 2..=17u16 {
            for x in 41..=118u16 {
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
            for y in 2..=17u16 {
                let last = if width == 60 { 58u16 } else { 118 };
                let from = if width == 60 { 1u16 } else { 41 };
                for x in from..=last {
                    assert_eq!(cell(&buf, x, y).symbol(), " ", "width {width} x={x} y={y}");
                }
            }
        }

        // Discriminating companion: with a change selected, the region is
        // never blank.
        let with_change = dashboard_with(
            vec![fixture::active("alpha", 1, 2)],
            Vec::new(),
            0,
            Route::Detail,
        );
        let buf = render_at(120, 20, &with_change);
        assert!(row_text(&buf, 2).contains("alpha"));
    }

    #[test]
    fn the_five_tab_bars_exact_string_with_the_selected_tab_bold() {
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
            0,
            Route::Detail,
            empty_detail_with_tab("", Vec::new(), 2),
        );
        for (width, w) in [(120, 78), (60, 58)] {
            let buf = render_at(width, 20, &d);
            let expected = "1 proposal  2 specs  3 design  4 tasks  5 planning-review";
            assert_eq!(
                detail_interior_cols(&buf, 3, w.min(expected.chars().count())),
                &expected[..expected.chars().count().min(w)],
                "width {width}"
            );
            let from = if width == 60 { 1u16 } else { 41 };
            // "3 design" is bold; "1 proposal" is not.
            for x in (from + 21)..(from + 21 + 8) {
                assert!(
                    cell(&buf, x, 3)
                        .style()
                        .add_modifier
                        .contains(Modifier::BOLD),
                    "width {width} x {x}: `3 design` should be bold"
                );
            }
            for x in from..(from + 10) {
                assert!(
                    !cell(&buf, x, 3)
                        .style()
                        .add_modifier
                        .contains(Modifier::BOLD),
                    "width {width} x {x}: `1 proposal` should not be bold"
                );
            }
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
            0,
            Route::List,
            empty_detail_with_tab("", Vec::new(), 0),
        );
        d.apply(Action::SelectTab(2));
        let buf = render_at(120, 20, &d);
        assert!(row_text(&buf, 3).contains("3 design"));
        for x in 41 + 21..41 + 21 + 8 {
            assert!(
                cell(&buf, x, 3)
                    .style()
                    .add_modifier
                    .contains(Modifier::BOLD)
            );
        }
        // Discriminating companion, naming the check's other mandated
        // width: at 60, Route::List, the narrow layout draws only the list
        // region — no detail region, and so no tab row at all.
        let buf60 = render_at(60, 20, &d);
        assert!(!row_text(&buf60, 3).contains("3 design"));
    }

    #[test]
    fn the_border_sweep_holds_with_twelve_forty_character_tab_ids_and_two_hundred_character_lines()
    {
        let ids: Vec<String> = (0..12)
            .map(|i| format!("{}{i}", "x".repeat(40 - i.to_string().len())))
            .collect();
        let pairs: Vec<(&str, &[&str])> = ids.iter().map(|id| (id.as_str(), &[][..])).collect();
        let change = fixture::with_artifacts(fixture::active("detail-view", 4, 9), &pairs);
        let source: String = (0..30).map(|_| format!("{}\n", "y".repeat(200))).collect();
        let d = dashboard_with_detail(
            vec![change],
            Vec::new(),
            0,
            Route::Detail,
            empty_detail_with_tab(&source, Vec::new(), 0),
        );
        let buf60 = render_at(60, 20, &d);
        for y in 1..=18u16 {
            assert!(matches!(cell(&buf60, 0, y).symbol(), "│" | "┌" | "└"));
            assert!(matches!(cell(&buf60, 59, y).symbol(), "│" | "┐" | "┘"));
        }
        let buf120 = render_at(120, 20, &d);
        for y in 1..=18u16 {
            for x in [0u16, 39, 40, 119] {
                let s = cell(&buf120, x, y).symbol();
                assert!(
                    matches!(s, "│" | "┌" | "└" | "┐" | "┘"),
                    "x={x} y={y} symbol={s:?}"
                );
            }
        }
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
            0,
            Route::Detail,
            empty_detail_with_tab("", Vec::new(), 1),
        );
        for width in [120, 60] {
            let buf = render_at(width, 20, &d);
            assert!(
                detail_interior_cols(&buf, 4, 14).starts_with("No content yet"),
                "width {width}"
            );
            assert!(
                row_text(&buf, 3).contains("2 specs"),
                "width {width}: tab bar still intact"
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
            0,
            Route::Detail,
            empty_detail_with_tab("", Vec::new(), 3),
        );
        for width in [120, 60] {
            let buf = render_at(width, 20, &d);
            assert!(
                detail_interior_cols(&buf, 4, 14).starts_with("No content yet"),
                "width {width}: marked tab"
            );
            assert!(
                !row_text(&buf, 4).contains('█') && !row_text(&buf, 4).contains('░'),
                "width {width}: no progress bar for a marked tab with no file"
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
            0,
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
                detail_interior_cols(&buf, 4, 24).starts_with("! /repo/specs/a/spec.md:"),
                "width {width}"
            );
            assert!(
                detail_interior_cols(&buf, 5, 3).starts_with("# b"),
                "width {width}"
            );
            assert!(
                !row_text(&buf, 4).contains("No content yet"),
                "width {width}"
            );
        }
    }

    #[test]
    fn the_twenty_item_list_fills_the_content_area_rows_4_through_17() {
        let change =
            fixture::with_artifacts(fixture::active("detail-view", 4, 9), &[("proposal", &[])]);
        let d = dashboard_with_detail(
            vec![change],
            Vec::new(),
            0,
            Route::Detail,
            empty_detail_with_tab(&twenty_line_source(), Vec::new(), 0),
        );
        for width in [120, 60] {
            let buf = render_at(width, 20, &d);
            assert_eq!(
                detail_interior_cols(&buf, 4, 9),
                "- line-00",
                "width {width}"
            );
            assert_eq!(
                detail_interior_cols(&buf, 17, 9),
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
            0,
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
                detail_interior_cols(&buf3, 4, bar.chars().count()),
                bar,
                "width {width}"
            );
            assert_eq!(
                detail_interior_cols(&buf3, 5, interior_width(width) as usize).trim(),
                "",
                "width {width}: blank row"
            );
            assert_eq!(
                detail_interior_cols(&buf3, 6, 11),
                "## 1. Setup",
                "width {width}"
            );
            assert_eq!(
                detail_interior_cols(&buf3, 7, 13),
                "[x] 1.1 first",
                "width {width}"
            );
            assert_eq!(
                detail_interior_cols(&buf3, 8, 14),
                "[ ] 1.2 second",
                "width {width}"
            );

            let d0 = dashboard_with_marked_change(&ids, Some(3), progress, source, Vec::new(), 0);
            let buf0 = render_at(width, 20, &d0);
            assert_eq!(
                detail_interior_cols(&buf0, 4, 11),
                "## 1. Setup",
                "width {width}: no bar row above it"
            );
            assert_eq!(
                detail_interior_cols(&buf0, 6, 15),
                "- [x] 1.1 first",
                "width {width}: source bullet intact"
            );
            assert_eq!(
                detail_interior_cols(&buf0, 7, 16),
                "- [ ] 1.2 second",
                "width {width}: source bullet intact"
            );

            assert_eq!(
                row_text(&buf3, 3),
                row_text(&buf0, 3),
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
                detail_interior_cols(&buf0, 4, bar.chars().count()),
                bar,
                "width {width}: id checklist carries the flag"
            );
            assert_eq!(
                detail_interior_cols(&buf0, 6, 8),
                "[x] done",
                "width {width}"
            );

            let d1 = dashboard_with_marked_change(&ids, Some(0), progress, source, Vec::new(), 1);
            let buf1 = render_at(width, 20, &d1);
            assert_eq!(
                detail_interior_cols(&buf1, 4, 10),
                "- [x] done",
                "width {width}: id tasks does not carry the flag"
            );
            assert_ne!(
                detail_interior_cols(&buf1, 4, bar.chars().count()),
                bar,
                "width {width}"
            );

            assert!(row_text(&buf0, 3).contains("1 checklist"), "width {width}");
            assert!(row_text(&buf0, 3).contains("2 tasks"), "width {width}");
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
                    !row_text(&buf, 4).contains('█') && !row_text(&buf, 4).contains('░'),
                    "width {width} tab {tab}: no progress-bar row"
                );
                assert_eq!(
                    detail_interior_cols(&buf, 4, 7),
                    "- [ ] a",
                    "width {width} tab {tab}"
                );
                assert!(
                    row_text(&buf, 3).contains("1 alpha")
                        && row_text(&buf, 3).contains("2 beta")
                        && row_text(&buf, 3).contains("3 gamma"),
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
            assert_eq!(detail_interior_cols(&buf, 4, 3), "[-]", "width {width}");
            assert_eq!(
                detail_interior_cols(&buf, 6, 12),
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
                detail_interior_cols(&buf, 4, 14),
                "No content yet",
                "width {width}"
            );
            assert!(!buffer_contains(&buf, "No tasks yet"), "width {width}");
            assert!(
                !row_text(&buf, 4).contains('█') && !row_text(&buf, 4).contains('░'),
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
            let border_x = if width == 60 { 59 } else { 119 };
            let last_interior_x = border_x - 1;
            assert_ne!(
                cell(&buf, last_interior_x, 4).symbol(),
                "│",
                "width {width}: the interior's last column is still row content"
            );
            assert_eq!(
                cell(&buf, border_x, 4).symbol(),
                "│",
                "width {width}: the border one column past it is untouched"
            );
            assert!(
                detail_interior_cols(&buf, 4, 24).starts_with("! /repo/openspec/changes"),
                "width {width}"
            );
            assert!(!buffer_contains(&buf, "No content yet"), "width {width}");
            assert!(!buffer_contains(&buf, "No tasks yet"), "width {width}");
            assert!(
                !row_text(&buf, 4).contains('█') && !row_text(&buf, 4).contains('░'),
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
            cols(&row_text(&buf120, 4), 41..119),
            crate::ui::tasks::progress_bar(&progress, 78),
        );
        assert!(cols(&row_text(&buf120, 4), 41..119).ends_with("[4/9] 44%"));

        let buf60 = render_at(60, 20, &d);
        assert_eq!(
            cols(&row_text(&buf60, 4), 1..59),
            crate::ui::tasks::progress_bar(&progress, 58),
        );
        assert!(cols(&row_text(&buf60, 4), 1..59).ends_with("[4/9] 44%"));

        for buf in [&buf120, &buf60] {
            let iw = interior_width(buf.area.width) as usize;
            assert!(detail_interior_cols(buf, 5, iw).trim().is_empty());
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
        assert_eq!(cols(&row_text(&buf120, 4), 41..44), "[-]");
        assert_eq!(cell(&buf120, 44, 4).symbol(), " ");

        let buf60 = render_at(60, 20, &d);
        assert_eq!(cols(&row_text(&buf60, 4), 1..4), "[-]");

        for buf in [&buf120, &buf60] {
            assert_eq!(
                detail_interior_cols(buf, 6, 12),
                "No tasks yet",
                "row 6 reads No tasks yet"
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
                detail_interior_cols(&buf_marked, 4, 3),
                "[-]",
                "width {width}"
            );
            assert!(
                detail_interior_cols(&buf_marked, 5, interior_width(width) as usize)
                    .trim()
                    .is_empty(),
                "width {width}"
            );
            assert_eq!(
                detail_interior_cols(&buf_marked, 6, 12),
                "No tasks yet",
                "width {width}: the twenty bullets hold no task lines"
            );

            assert_eq!(
                row_text(&buf_unmarked, 2),
                row_text(&buf_marked, 2),
                "width {width}: header row byte-identical"
            );
            assert_eq!(
                row_text(&buf_unmarked, 3),
                row_text(&buf_marked, 3),
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
        assert_eq!(expected.chars().count(), 53);
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
}
