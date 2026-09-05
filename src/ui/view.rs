//! The render seam's pure side: `render(frame, &Dashboard)`. Performs no
//! filesystem, process, environment, network, or terminal I/O, reads no
//! clock and no global state. See
//! `openspec/changes/tui-shell/specs/responsive-layout/spec.md`.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Block;

use crate::ui::app::{Dashboard, Filter, Route};
use crate::ui::detail;
use crate::ui::layout::{interior, scroll_offset, split_body, split_detail, split_frame, viewport};
use crate::ui::list;
use crate::ui::markdown::Face;

/// The footer's key hints, in the order they are drawn and dropped from.
const FOOTER_HINTS: [&str; 3] = ["q quit", "Enter detail", "Esc back"];

/// Draw `dashboard` into `frame`. A pure function of its two arguments.
pub fn render(frame: &mut Frame, dashboard: &Dashboard) {
    let (header, body, footer) = split_frame(frame.area());
    render_header(frame, header, dashboard);
    render_footer(frame, footer, &dashboard.filter);
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
            let style = style_for(&segment.face);
            buf.set_string(x, y, &segment.text, style);
            x += segment.text.chars().count() as u16;
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

/// The header row: `OpenSpec`, bold, at column 0, and the repository's
/// display path (or `no repository`) right-aligned, shortened from the left
/// when the header is too narrow to hold it whole.
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

    let text = match &dashboard.repo {
        Some(path) => path.display().to_string(),
        None => "no repository".to_string(),
    };
    if let Some(shown) = shorten_for_header(&text, header.width) {
        let shown_len = shown.chars().count() as u16;
        let x = header.x + header.width.saturating_sub(shown_len);
        buf.set_string(x, header.y, &shown, Style::default());
    }
}

/// The header-shortening rule, isolated so it is readable independently of
/// the frame: `A` is the header width minus 9 (the eight columns of
/// `OpenSpec` plus one separating blank), floored at zero. `text` fitting in
/// `A` characters is shown whole; longer text is shown as `…` plus its last
/// `A - 1` characters when `A >= 8`; otherwise nothing is shown at all, and
/// only the (possibly itself truncated) `OpenSpec` label is drawn.
fn shorten_for_header(text: &str, header_width: u16) -> Option<String> {
    let a = header_width.saturating_sub(9) as usize;
    let char_count = text.chars().count();
    if char_count <= a {
        Some(text.to_string())
    } else if a >= 8 {
        Some(list::shorten_left(text, a))
    } else {
        None
    }
}

/// The footer row, in one of three forms — `list-filtering` -> "The footer
/// shows the filter prompt while filtering and the query after":
/// - filtering: the prompt `/` + query + `_`, replacing the hints entirely,
///   keeping its **tail** when it overflows the footer;
/// - not filtering, a non-empty query: `/` + query leads the hint list,
///   dropped last rather than first;
/// - otherwise: `q quit`, `Enter detail`, and `Esc back`, unchanged.
fn render_footer(frame: &mut Frame, footer: Rect, filter: &Filter) {
    if footer.height == 0 {
        return;
    }
    let text = if filter.active {
        footer_prompt(&filter.query, footer.width)
    } else if filter.query.is_empty() {
        fit_hints(
            &FOOTER_HINTS
                .iter()
                .map(|s| (*s).to_string())
                .collect::<Vec<_>>(),
            footer.width,
        )
    } else {
        let mut hints = vec![format!("/{}", filter.query)];
        hints.extend(FOOTER_HINTS.iter().map(|s| (*s).to_string()));
        fit_hints(&hints, footer.width)
    };
    if !text.is_empty() {
        frame
            .buffer_mut()
            .set_string(footer.x, footer.y, &text, Style::default());
    }
}

/// The filter prompt: `/` + `query` + `_`, whole when it fits `width`,
/// otherwise its **tail** — the last `width` characters — so the cursor
/// (the trailing `_`) and the characters just typed stay visible.
fn footer_prompt(query: &str, width: u16) -> String {
    let text = format!("/{query}_");
    let chars: Vec<char> = text.chars().collect();
    let w = width as usize;
    if chars.len() <= w {
        text
    } else {
        chars[chars.len() - w..].iter().collect()
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
        let needed = hint.chars().count() as u16 + separator;
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
    }

    #[test]
    fn footer_drops_whole_hints() {
        let d = dashboard(Some("/tmp/demo-repo"), Route::List);
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
    /// written for.
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
        }
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
}
