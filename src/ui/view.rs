//! The render seam's pure side: `render(frame, &Dashboard)`. Performs no
//! filesystem, process, environment, network, or terminal I/O, reads no
//! clock and no global state. See
//! `openspec/changes/tui-shell/specs/responsive-layout/spec.md`.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Block;

use crate::ui::app::{Dashboard, Route};
use crate::ui::layout::{split_body, split_frame, viewport};
use crate::ui::list;

/// The footer's key hints, in the order they are drawn and dropped from.
const FOOTER_HINTS: [&str; 3] = ["q quit", "Enter detail", "Esc back"];

/// Draw `dashboard` into `frame`. A pure function of its two arguments.
pub fn render(frame: &mut Frame, dashboard: &Dashboard) {
    let (header, body, footer) = split_frame(frame.area());
    render_header(frame, header, dashboard);
    render_footer(frame, footer);
    render_body(frame, body, dashboard);
}

/// The body: one or two bordered regions, per `layout::split_body`. The list
/// region's interior is filled by `render_list`; the detail region's is left
/// untouched — `markdown-viewer` and `detail-view` fill it.
fn render_body(frame: &mut Frame, body: Rect, dashboard: &Dashboard) {
    let (list_area, detail) = split_body(body, dashboard.route);
    if let Some(area) = list_area {
        render_region(frame, area, "Changes", dashboard.route == Route::List);
        let interior = Block::bordered().inner(area);
        render_list(frame, interior, dashboard);
    }
    if let Some(area) = detail {
        render_region(frame, area, "Detail", dashboard.route == Route::Detail);
    }
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
    let a = header_width.saturating_sub(9);
    let char_count = text.chars().count() as u16;
    if char_count <= a {
        Some(text.to_string())
    } else if a >= 8 {
        let tail_len = (a - 1) as usize;
        let tail: String = text
            .chars()
            .rev()
            .take(tail_len)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        Some(format!("…{tail}"))
    } else {
        None
    }
}

/// The footer row: `q quit`, `Enter detail`, and `Esc back`, separated by
/// two spaces, dropping whole hints from the end when the remaining width
/// cannot hold the next one whole.
fn render_footer(frame: &mut Frame, footer: Rect) {
    if footer.height == 0 {
        return;
    }
    let mut shown: Vec<&str> = Vec::new();
    let mut used = 0u16;
    for (i, hint) in FOOTER_HINTS.iter().enumerate() {
        let separator = if i == 0 { 0 } else { 2 };
        let needed = hint.chars().count() as u16 + separator;
        let Some(next_used) = used.checked_add(needed) else {
            break;
        };
        if next_used > footer.width {
            break;
        }
        used = next_used;
        shown.push(hint);
    }
    let text = shown.join("  ");
    if !text.is_empty() {
        frame
            .buffer_mut()
            .set_string(footer.x, footer.y, &text, Style::default());
    }
}

#[cfg(test)]
mod tests {
    use ratatui::backend::TestBackend;
    use ratatui::buffer::{Buffer, Cell};
    use ratatui::style::Modifier;

    use crate::changes::{Change, empty_set, fixture};
    use crate::testutil::{cell, render_at, row_text};
    use crate::ui::app::{Action, Dashboard, Filter, Route};

    fn empty_filter() -> Filter {
        Filter {
            query: String::new(),
            active: false,
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
        let d = dashboard(Some("/tmp/demo-repo"), Route::List);
        let default_style = Cell::default().style();

        let buf = render_at(120, 20, &d);
        for y in 2..=17u16 {
            for x in (1..=38u16).chain(41..=118u16) {
                let c = cell(&buf, x, y);
                assert_eq!(c.symbol(), " ", "x={x} y={y}");
                assert_eq!(c.style(), default_style, "x={x} y={y}");
            }
        }

        let buf = render_at(60, 20, &d);
        for y in 2..=17u16 {
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
        let d = dashboard(None, Route::List);
        let buf = render_at(60, 20, &d);
        assert_eq!(cols(&row_text(&buf, 0), 47..60), "no repository");

        let buf = render_at(120, 20, &d);
        assert_eq!(cols(&row_text(&buf, 0), 107..120), "no repository");
        assert!(!buffer_contains(&buf, "/tmp/searched-from"));
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
        let d = three_active();
        let default_style = Cell::default().style();
        let buf120 = render_at(120, 20, &d);
        for y in 2..=17u16 {
            for x in 41..=118u16 {
                let c = cell(&buf120, x, y);
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
        let d = three_active_at_route(Route::Detail);
        let buf60 = render_at(60, 20, &d);
        for name in ["add-token-refresh", "fix-empty-basket", "migrate-ai-sdk-v7"] {
            assert!(!buffer_contains(&buf60, name));
        }
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
        d.apply(Action::SelectNext);
        d.apply(Action::SelectNext);
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
            d.apply(Action::SelectNext);
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
        d.apply(Action::SelectNext);
        d.apply(Action::SelectNext);
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
        let d = dashboard_with(names, Vec::new(), 0, Route::List);

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
}
