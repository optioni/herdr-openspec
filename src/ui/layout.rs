//! The 100-column breakpoint and the frame split. Pure `Rect` arithmetic —
//! no filesystem, process, environment, network, or standard-I/O API. See
//! `openspec/changes/tui-shell/specs/responsive-layout/spec.md`.

use ratatui::buffer::CellWidth;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::Style;
use ratatui::text::Span;

use crate::ui::app::Route;

/// The narrow/wide breakpoint, in columns. `SPEC.md` -> Responsive layout.
pub const WIDE_MIN_WIDTH: u16 = 100;

/// Whether the body shows one region (below the breakpoint) or two (at or
/// above it). Derived from the current frame area on every draw — never
/// stored on `Dashboard` — so a resize across the breakpoint changes layout
/// on the very next frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Narrow,
    Wide,
}

/// A total function of `width` alone.
pub fn mode(width: u16) -> LayoutMode {
    if width >= WIDE_MIN_WIDTH {
        LayoutMode::Wide
    } else {
        LayoutMode::Narrow
    }
}

/// Split `area` into a header row, a body, and a footer row.
///
/// Heights 0, 1, and 2 are branched on explicitly rather than handed to the
/// constraint solver: measured against ratatui 0.30.2,
/// `Layout::vertical([Length(1), Min(0), Length(1)])` at height 1 gives the
/// single row to the footer, not the header, which is not the contract
/// `responsive-layout` wants. See design.md -> Decisions.
pub fn split_frame(area: Rect) -> (Rect, Rect, Rect) {
    let row = |y: u16, height: u16| Rect {
        x: area.x,
        y,
        width: area.width,
        height,
    };
    match area.height {
        0 => (row(area.y, 0), row(area.y, 0), row(area.y, 0)),
        1 => (row(area.y, 1), row(area.y + 1, 0), row(area.y + 1, 0)),
        2 => (row(area.y, 1), row(area.y + 1, 0), row(area.y + 1, 1)),
        h => (
            row(area.y, 1),
            row(area.y + 1, h - 2),
            row(area.y + h - 1, 1),
        ),
    }
}

/// Split the body into the change-list region and the artifact-detail
/// region. At [`LayoutMode::Wide`] both are drawn, divided at column 40;
/// below the breakpoint only the routed region is drawn and the other is
/// `None`.
pub fn split_body(area: Rect, route: Route) -> (Option<Rect>, Option<Rect>) {
    match mode(area.width) {
        LayoutMode::Wide => {
            let [list, detail] =
                Layout::horizontal([Constraint::Length(40), Constraint::Min(0)]).areas(area);
            (Some(list), Some(detail))
        }
        LayoutMode::Narrow => match route {
            Route::List => (Some(area), None),
            Route::Detail => (None, Some(area)),
        },
    }
}

/// The index of the first row to draw, so the selected row (`cursor`, an
/// index into the emitted row vector — not `Dashboard::selected`, since
/// problem, separator, and message rows shift it) stays inside a `height`-row
/// slice of `rows` total rows. Pure and total over its three arguments,
/// derived on every draw rather than stored on `Dashboard`: the interior
/// height is a property of the current frame, exactly like `LayoutMode`, and
/// a stored offset would be stale after a resize. See
/// `specs/list-selection/spec.md` -> "The visible slice follows the
/// selection".
pub fn viewport(rows: usize, cursor: usize, height: u16) -> usize {
    let height = height as usize;
    if height == 0 || rows <= height {
        return 0;
    }
    cursor.saturating_sub(height / 2).min(rows - height)
}

/// The index of the first detail line to draw, so a `height`-row slice of
/// `lines` total lines never runs past the end. Pure and total over its
/// three arguments, derived on every draw rather than stored — the
/// interior height is a property of the current frame, exactly like
/// `viewport`, which this sits beside for the same reason.
pub fn scroll_offset(lines: usize, scroll: usize, height: u16) -> usize {
    if height == 0 {
        return 0;
    }
    scroll.min(lines.saturating_sub(height as usize))
}

/// A bordered region's interior: the one place in the crate that computes
/// this, so `Dashboard::normalise_scroll` can derive it without
/// constructing a `ratatui::widgets::Block`. Performs exactly the
/// arithmetic `Block::bordered().inner` does: the origin advanced by one
/// column and one row and clamped to the rectangle's own right and bottom
/// edges, with the width and height each reduced by two, saturating to
/// zero. The clamp is not decoration — at a 1x1 or 0x0 rectangle it is the
/// difference between `x: 0` and `x: 1`.
pub fn interior(area: Rect) -> Rect {
    let x = area.x.saturating_add(1).min(area.x + area.width);
    let y = area.y.saturating_add(1).min(area.y + area.height);
    Rect {
        x,
        y,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}

/// Split the detail region's interior into a one-row header, a one-row tab
/// bar, and the content area below — each the interior's full width and
/// carrying the interior's own `x` and `width`. Heights 0, 1, and 2 are
/// branched on explicitly rather than handed to the constraint solver,
/// exactly as `split_frame` is and for the same measured reason.
pub fn split_detail(interior: Rect) -> (Rect, Rect, Rect) {
    let row = |y: u16, height: u16| Rect {
        x: interior.x,
        y,
        width: interior.width,
        height,
    };
    match interior.height {
        0 => (row(interior.y, 0), row(interior.y, 0), row(interior.y, 0)),
        1 => (
            row(interior.y, 1),
            row(interior.y + 1, 0),
            row(interior.y + 1, 0),
        ),
        2 => (
            row(interior.y, 1),
            row(interior.y + 1, 1),
            row(interior.y + 2, 0),
        ),
        h => (
            row(interior.y, 1),
            row(interior.y + 1, 1),
            row(interior.y + 2, h - 2),
        ),
    }
}

/// The part of the frame drawn at a point — `mouse-input`'s hit-test result.
/// Named `Zone` rather than `Target`, which `list-sections` had already taken
/// for "one addressable row the cursor can land on" (design.md -> Decision 1).
///
/// `ListRow` and `DetailTab` carry the rectangle they were derived from rather
/// than only an offset, so the caller that resolves the offset to a row or a
/// tab uses the very geometry the hit test used: recomputing the interior at
/// the call site would be a second derivation of the same rectangle, and the
/// two could drift (design.md -> Decision 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    /// A row of the list region's interior. `interior` is that interior's own
    /// rectangle; `row` is the offset of the addressed row below its first
    /// interior row.
    ListRow { interior: Rect, row: u16 },
    /// The list region, but not one of its interior rows — its border.
    List,
    /// The detail region's tab-bar row. `bar` is that row's own rectangle;
    /// `column` is the offset of the addressed column right of its first.
    DetailTab { bar: Rect, column: u16 },
    /// The detail region, anywhere but the tab-bar row: its border, its header
    /// row, or its content area.
    Detail,
    /// The frame's header row, its footer row, or a point outside the frame.
    Outside,
}

/// Map a terminal coordinate to the part of the frame drawn there, deriving
/// the geometry through the same [`split_frame`], [`split_body`], [`interior`],
/// and [`split_detail`] the draw path uses and holding no arithmetic of its own
/// beyond a rectangle containment test.
///
/// Pure and total: every `Rect`, every `Route`, and every `(column, row)` pair
/// including `(0, 0)` and `(u16::MAX, u16::MAX)` returns a `Zone` and none
/// panics. Names no ratatui widget, no crossterm type, and no mouse type — it
/// takes two integers, which is what keeps it in the pure view set and testable
/// with no event at all.
///
/// The route is respected exactly as [`split_body`] respects it: at
/// [`LayoutMode::Narrow`] only the routed region exists, so every point in the
/// body resolves to that region's zones and none to the other's; at
/// [`LayoutMode::Wide`] both exist at both routes and the route changes nothing.
pub fn zone(area: Rect, route: Route, column: u16, row: u16) -> Zone {
    let point = Position::new(column, row);
    let (_, body, _) = split_frame(area);
    let (list, detail) = split_body(body, route);
    if let Some(list_area) = list
        && list_area.contains(point)
    {
        let inner = interior(list_area);
        return if inner.contains(point) {
            Zone::ListRow {
                interior: inner,
                row: row - inner.y,
            }
        } else {
            Zone::List
        };
    }
    if let Some(detail_area) = detail
        && detail_area.contains(point)
    {
        let (_, bar, _) = split_detail(interior(detail_area));
        return if bar.contains(point) {
            Zone::DetailTab {
                bar,
                column: column - bar.x,
            }
        } else {
            Zone::Detail
        };
    }
    Zone::Outside
}

/// The crate's only display-width measure: the number of terminal cells
/// `Buffer::set_stringn` consumes for `text`, computed the way `set_stringn` itself
/// computes it — summing each grapheme cluster's `cell_width()` — rather than by an
/// independent table. See
/// `openspec/changes/view-fidelity/specs/responsive-layout/spec.md` -> "Display width is
/// measured in terminal columns by one pair of primitives".
///
/// `Span::raw(text).styled_graphemes(Style::default())` is named here and again in
/// [`truncate_columns`] rather than behind a shared helper: `styled_graphemes`'s own
/// signature is `fn styled_graphemes<S>(&'a self, ...) -> impl Iterator<Item =
/// StyledGrapheme<'a>>` on `Span<'a>`, where `'a` is the `Span`'s own generic parameter, not
/// a fresh borrow of `&self` — so a helper returning the iterator would have to return a
/// value borrowing the `Span` it just constructed locally, which does not live long enough
/// to satisfy that bound. The one-line call is the whole of the duplication; splitting it
/// out would need an owned wrapper or a macro, either of which would cost more clarity than
/// the duplication does.
pub(crate) fn columns(text: &str) -> usize {
    Span::raw(text)
        .styled_graphemes(Style::default())
        .map(|g| g.symbol.cell_width() as usize)
        .sum()
}

/// The longest prefix of `text`, ending on a grapheme-cluster boundary, whose [`columns`]
/// is at most `max`. Never splits a cluster and never panics for any `text` and any `max`.
///
/// The cut point is derived from each grapheme's own byte offset within `text` — computed
/// from the returned symbol's own pointer position, which is a slice of `text` since
/// `Span::raw` borrows rather than copies — and never from a running sum of the returned
/// symbols' lengths: `styled_graphemes` drops clusters that hold a control character, so a
/// running sum of what it yields is shifted by every dropped byte and would slice `text`
/// mid-character. See the same requirement's second scenario. The grapheme split is
/// duplicated from [`columns`] rather than shared; see that function's doc comment for why.
///
/// The bare `as usize` pointer subtraction below is sound, not merely convenient: it never
/// underflows because `grapheme.symbol` is always a sub-slice of `text` itself —
/// `Span::raw(text)` borrows rather than copies, so `styled_graphemes` yields symbols whose
/// pointers fall inside `text`'s own allocation, never before it. A `checked_sub` fallback
/// was deliberately not added: it would be an uncovered production branch this crate's own
/// coverage floor has no headroom for, defending against a case that provably cannot occur.
pub(crate) fn truncate_columns(text: &str, max: usize) -> &str {
    if max == 0 {
        return "";
    }
    let mut used = 0usize;
    let mut end = 0usize;
    let mut truncated = false;
    for grapheme in Span::raw(text).styled_graphemes(Style::default()) {
        let width = grapheme.symbol.cell_width() as usize;
        if used + width > max {
            truncated = true;
            break;
        }
        used += width;
        let offset = grapheme.symbol.as_ptr() as usize - text.as_ptr() as usize;
        end = offset + grapheme.symbol.len();
    }
    if truncated { &text[..end] } else { text }
}

#[cfg(test)]
mod tests {
    use crate::ui::app::Route;
    use crate::ui::layout::{
        LayoutMode, WIDE_MIN_WIDTH, columns, interior, mode, scroll_offset, split_body,
        split_detail, split_frame, truncate_columns, viewport,
    };
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Style;
    use ratatui::widgets::Block;

    /// `mouse-input`'s hit test. Every expected interior here is derived from
    /// `split_frame`/`split_body`/`interior`/`split_detail` independently, never
    /// from `zone`'s own answer — otherwise the test would pin whatever `zone`
    /// happens to do rather than what the draw path does.
    mod zone {
        use crate::ui::app::Route;
        use crate::ui::layout::{
            LayoutMode, Zone, interior, mode, split_body, split_detail, split_frame, zone,
        };
        use ratatui::layout::Rect;

        /// The list region's interior, derived the way the draw path derives it.
        fn list_interior(area: Rect, route: Route) -> Rect {
            let (_, body, _) = split_frame(area);
            interior(split_body(body, route).0.expect("a list region is drawn"))
        }

        /// The detail region's tab-bar row, derived the same way.
        fn detail_bar(area: Rect, route: Route) -> Rect {
            let (_, body, _) = split_frame(area);
            split_detail(interior(
                split_body(body, route).1.expect("a detail region is drawn"),
            ))
            .1
        }

        #[test]
        fn the_zones_tile_the_frame() {
            let area = Rect::new(0, 0, 120, 40);
            for route in [Route::List, Route::Detail] {
                let list = list_interior(area, route);
                let bar = detail_bar(area, route);
                let last_row = list.height - 1;

                let cases: [((u16, u16), Zone); 10] = [
                    ((0, 0), Zone::Outside),
                    ((0, 39), Zone::Outside),
                    ((0, 1), Zone::List),
                    (
                        (list.x, list.y),
                        Zone::ListRow {
                            interior: list,
                            row: 0,
                        },
                    ),
                    (
                        (list.x, list.y + last_row),
                        Zone::ListRow {
                            interior: list,
                            row: last_row,
                        },
                    ),
                    ((40, 5), Zone::Detail),
                    ((bar.x, bar.y - 1), Zone::Detail),
                    ((bar.x, bar.y), Zone::DetailTab { bar, column: 0 }),
                    ((bar.x, bar.y + 1), Zone::Detail),
                    ((200, 5), Zone::Outside),
                ];
                for ((column, row), expected) in cases {
                    assert_eq!(
                        zone(area, route, column, row),
                        expected,
                        "at ({column}, {row}) under {route:?}"
                    );
                }
            }
        }

        #[test]
        fn below_the_breakpoint_only_the_routed_region() {
            let area = Rect::new(0, 0, 60, 20);
            let list = list_interior(area, Route::List);

            assert_eq!(
                zone(area, Route::List, list.x, list.y),
                Zone::ListRow {
                    interior: list,
                    row: 0
                }
            );
            assert!(matches!(
                zone(area, Route::Detail, list.x, list.y),
                Zone::Detail | Zone::DetailTab { .. }
            ));

            // No point anywhere in the body resolves to the other route's region.
            let (_, body, _) = split_frame(area);
            for row in body.y..body.y + body.height {
                for column in body.x..body.x + body.width {
                    assert!(
                        !matches!(
                            zone(area, Route::Detail, column, row),
                            Zone::List | Zone::ListRow { .. }
                        ),
                        "({column}, {row}) is a list zone at Route::Detail"
                    );
                    assert!(
                        !matches!(
                            zone(area, Route::List, column, row),
                            Zone::Detail | Zone::DetailTab { .. }
                        ),
                        "({column}, {row}) is a detail zone at Route::List"
                    );
                }
            }
        }

        #[test]
        fn the_breakpoint_is_exact_for_zone() {
            // Column 10, row 5: inside the list region at the wide layout, inside
            // the (single) detail region at the narrow one.
            for width in [99u16, 100, 101] {
                let area = Rect::new(0, 0, width, 20);
                let resolved = zone(area, Route::Detail, 10, 5);
                match mode(width) {
                    LayoutMode::Narrow => assert!(
                        matches!(resolved, Zone::Detail | Zone::DetailTab { .. }),
                        "width {width} resolved {resolved:?}"
                    ),
                    LayoutMode::Wide => assert!(
                        matches!(resolved, Zone::List | Zone::ListRow { .. }),
                        "width {width} resolved {resolved:?}"
                    ),
                }
            }
            assert_eq!(mode(99), LayoutMode::Narrow);
            assert_eq!(mode(100), LayoutMode::Wide);
            assert_eq!(mode(101), LayoutMode::Wide);
        }

        #[test]
        fn degenerate_frames_resolve() {
            let areas = [
                Rect::new(0, 0, 0, 0),
                Rect::new(0, 0, 1, 1),
                Rect::new(0, 0, 2, 2),
                Rect::new(0, 0, 3, 3),
                Rect::new(0, 0, 120, 2),
            ];
            for area in areas {
                let (_, body, _) = split_frame(area);
                for route in [Route::List, Route::Detail] {
                    for row in 0..area.height {
                        for column in 0..area.width {
                            let resolved = zone(area, route, column, row);
                            if body.height == 0 {
                                assert_eq!(
                                    resolved,
                                    Zone::Outside,
                                    "{area:?} has no body, so ({column}, {row}) is Outside"
                                );
                            }
                        }
                    }
                    // Total over the extremes too.
                    let _ = zone(area, route, u16::MAX, u16::MAX);
                    let _ = zone(area, route, 0, 0);
                }
            }
        }
    }

    /// `responsive-layout` :: "`columns` agrees with what the buffer consumed" — the oracle
    /// is `Buffer::set_stringn`'s own **return value**, never a first-blank-cell scan: a
    /// reset cell is byte-identical to an untouched one, so a first-blank scan reports `1`
    /// for `日本語`, for `🎉`, and for the family emoji, and a measurement built to satisfy
    /// it would be wrong in exactly the direction this change exists to fix.
    #[test]
    fn columns_agrees_with_what_the_buffer_consumed() {
        let family_emoji = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}";
        let cases: [&str; 8] = [
            "abc",
            "日本語",
            "🎉",
            "e\u{0301}",
            family_emoji,
            "\u{FF76}\u{FF9E}",
            "\u{0007}",
            "",
        ];
        for s in cases {
            let mut buf = Buffer::empty(Rect::new(0, 0, 40, 1));
            let (x, _y) = buf.set_stringn(0, 0, s, usize::MAX, Style::default());
            assert_eq!(
                columns(s) as u16,
                x,
                "columns({s:?}) must equal set_stringn's own returned x, the number of \
                 cells it consumed"
            );
        }

        assert_eq!(columns(""), 0);
        assert_eq!(columns("\u{0007}"), 0);
    }

    /// `responsive-layout` :: "`truncate_columns` never splits a cluster and never
    /// overruns" — the BEL fixture is what pins the byte-offset rule: `styled_graphemes`
    /// drops the control cluster, so an implementation deriving its cut point from a
    /// running sum of *returned* symbol lengths computes an offset shifted by the dropped
    /// byte and slices mid-character.
    #[test]
    fn truncate_columns_never_splits_a_cluster_and_never_overruns() {
        let sources: [(&str, usize); 3] = [
            ("日本語の変更", 14),
            ("abc🎉def", 10),
            ("ab\u{0007}日本語", 10),
        ];
        for (source, max_max) in sources {
            for max in 0..=max_max {
                let got = truncate_columns(source, max);
                assert!(
                    columns(got) <= max,
                    "truncate_columns({source:?}, {max}) = {got:?} measures more than {max}"
                );
                assert!(
                    source.as_bytes().starts_with(got.as_bytes()),
                    "truncate_columns({source:?}, {max}) = {got:?} is not a byte prefix of the \
                     input"
                );
                // Re-slicing the input at the result's own length must not panic — the cut
                // landed on a character boundary.
                let _ = &source[..got.len()];
            }
        }

        assert_eq!(truncate_columns("日本語の変更", 3), "日");
        assert_eq!(columns(truncate_columns("日本語の変更", 3)), 2);

        for (source, _) in sources {
            assert_eq!(truncate_columns(source, 0), "");
        }

        for (source, _) in sources {
            let whole = columns(source);
            assert_eq!(truncate_columns(source, whole), source);
            assert_eq!(truncate_columns(source, whole + 5), source);
        }
    }

    #[test]
    fn scroll_offset_is_exact_at_its_boundaries() {
        let table = [
            (0usize, 0usize, 16u16, 0usize),
            (16, 0, 16, 0),
            (16, 9, 16, 0),
            (17, 0, 16, 0),
            (17, 1, 16, 1),
            (17, 2, 16, 1),
            (20, 4, 16, 4),
            (20, 99, 16, 4),
            (20, 4, 0, 0),
        ];
        for (lines, scroll, height, expect) in table {
            let got = scroll_offset(lines, scroll, height);
            assert_eq!(
                got, expect,
                "scroll_offset({lines}, {scroll}, {height}) = {got}, expected {expect}"
            );
            assert!(
                got <= lines.saturating_sub(height as usize),
                "scroll_offset({lines}, {scroll}, {height}) = {got} runs past the last line"
            );
        }
    }

    #[test]
    fn the_detail_interior_is_78_at_120_and_58_at_60() {
        let (_, body120, _) = split_frame(Rect::new(0, 0, 120, 20));
        let (_, detail120) = split_body(body120, Route::Detail);
        assert_eq!(
            interior(detail120.expect("detail region at 120")),
            Rect::new(41, 2, 78, 16)
        );

        let (_, body60, _) = split_frame(Rect::new(0, 0, 60, 20));
        let (_, detail60) = split_body(body60, Route::Detail);
        assert_eq!(
            interior(detail60.expect("detail region at 60, detail route")),
            Rect::new(1, 2, 58, 16)
        );

        let (_, detail_at_list_route) = split_body(body60, Route::List);
        assert_eq!(
            detail_at_list_route, None,
            "no detail rectangle at 60, list route"
        );
    }

    #[test]
    fn interior_agrees_with_a_bordered_block() {
        for area in [
            Rect::new(0, 1, 40, 18),
            Rect::new(40, 1, 80, 18),
            Rect::new(0, 1, 60, 18),
            Rect::new(0, 0, 2, 2),
            Rect::new(0, 0, 1, 1),
            Rect::new(0, 0, 0, 0),
        ] {
            assert_eq!(
                interior(area),
                Block::bordered().inner(area),
                "area {area:?}"
            );
        }
    }

    #[test]
    fn viewport_is_zero_when_everything_fits() {
        assert_eq!(viewport(0, 0, 16), 0);
        assert_eq!(viewport(16, 15, 16), 0);
        assert_eq!(viewport(17, 0, 16), 0);
        assert_eq!(viewport(30, 20, 0), 0);
    }

    #[test]
    fn viewport_centres_and_clamps() {
        assert_eq!(viewport(30, 20, 16), 12);
        assert_eq!(viewport(30, 29, 16), 14);
        assert_eq!(viewport(30, 20, 8), 16);
        assert_eq!(viewport(30, 0, 16), 0);
    }

    #[test]
    fn viewport_boundaries_are_exact() {
        assert_eq!(viewport(0, 0, 16), 0);
        assert_eq!(viewport(16, 15, 16), 0);
        assert_eq!(viewport(17, 0, 16), 0);
        assert_eq!(viewport(17, 8, 16), 0);
        assert_eq!(viewport(17, 9, 16), 1);
        assert_eq!(viewport(17, 16, 16), 1);
        assert_eq!(viewport(30, 20, 0), 0);
    }

    #[test]
    fn mode_is_narrow_below_100() {
        assert_eq!(mode(0), LayoutMode::Narrow);
        assert_eq!(mode(1), LayoutMode::Narrow);
        assert_eq!(mode(40), LayoutMode::Narrow);
        assert_eq!(mode(60), LayoutMode::Narrow);
        assert_eq!(mode(99), LayoutMode::Narrow);
    }

    #[test]
    fn mode_is_wide_at_100_and_above() {
        assert_eq!(WIDE_MIN_WIDTH, 100);
        assert_eq!(mode(100), LayoutMode::Wide);
        assert_eq!(mode(101), LayoutMode::Wide);
        assert_eq!(mode(120), LayoutMode::Wide);
        assert_eq!(mode(u16::MAX), LayoutMode::Wide);
    }

    #[test]
    fn split_frame_gives_header_body_footer_at_normal_height() {
        for width in [60u16, 120u16] {
            let area = Rect::new(0, 0, width, 20);
            let (header, body, footer) = split_frame(area);
            assert_eq!(header, Rect::new(0, 0, width, 1));
            assert_eq!(body, Rect::new(0, 1, width, 18));
            assert_eq!(footer, Rect::new(0, 19, width, 1));
        }
    }

    #[test]
    fn split_frame_degenerate_heights() {
        for width in [60u16, 120u16] {
            let (header, body, footer) = split_frame(Rect::new(0, 0, width, 0));
            assert_eq!(header.height, 0);
            assert_eq!(body.height, 0);
            assert_eq!(footer.height, 0);

            let (header, body, footer) = split_frame(Rect::new(0, 0, width, 1));
            assert_eq!(header, Rect::new(0, 0, width, 1));
            assert_eq!(body.height, 0);
            assert_eq!(footer.height, 0);

            let (header, body, footer) = split_frame(Rect::new(0, 0, width, 2));
            assert_eq!(header.y, 0);
            assert_eq!(footer.y, 1);
            assert_eq!(body.height, 0);
        }
    }

    #[test]
    fn split_body_wide_puts_the_divider_at_40() {
        let body = Rect::new(0, 1, 120, 18);
        let (list, detail) = split_body(body, Route::List);
        assert_eq!(list, Some(Rect::new(0, 1, 40, 18)));
        assert_eq!(detail, Some(Rect::new(40, 1, 80, 18)));

        let body = Rect::new(0, 1, 100, 18);
        let (list, detail) = split_body(body, Route::List);
        assert_eq!(list, Some(Rect::new(0, 1, 40, 18)));
        assert_eq!(detail, Some(Rect::new(40, 1, 60, 18)));
    }

    #[test]
    fn split_body_narrow_yields_one_region_for_the_route() {
        let body = Rect::new(0, 1, 60, 18);
        let (list, detail) = split_body(body, Route::List);
        assert_eq!(list, Some(body));
        assert_eq!(detail, None);

        let (list, detail) = split_body(body, Route::Detail);
        assert_eq!(list, None);
        assert_eq!(detail, Some(body));
    }

    /// `split_detail` at both mandated interior widths, at heights 0, 1, and
    /// 2 — the three degenerate cases branched on explicitly, exactly as
    /// `split_frame_degenerate_heights` covers `split_frame`'s. Base `y` is
    /// non-zero so a hard-coded `0` could not pass by accident.
    #[test]
    fn split_detail_degenerate_heights() {
        for width in [78u16, 58u16] {
            let at = |height: u16| Rect::new(3, 5, width, height);

            let (header, tabs, content) = split_detail(at(0));
            assert_eq!(header, Rect::new(3, 5, width, 0), "width {width} height 0");
            assert_eq!(tabs, Rect::new(3, 5, width, 0), "width {width} height 0");
            assert_eq!(content, Rect::new(3, 5, width, 0), "width {width} height 0");

            let (header, tabs, content) = split_detail(at(1));
            assert_eq!(header, Rect::new(3, 5, width, 1), "width {width} height 1");
            assert_eq!(tabs, Rect::new(3, 6, width, 0), "width {width} height 1");
            assert_eq!(content, Rect::new(3, 6, width, 0), "width {width} height 1");

            let (header, tabs, content) = split_detail(at(2));
            assert_eq!(header, Rect::new(3, 5, width, 1), "width {width} height 2");
            assert_eq!(tabs, Rect::new(3, 6, width, 1), "width {width} height 2");
            assert_eq!(content, Rect::new(3, 7, width, 0), "width {width} height 2");
        }
    }

    /// `split_detail` above the degenerate heights: the header and tab bar
    /// stay one row each and the content area is `height - 2` rows,
    /// starting two rows below the interior's own `y`. `artifact-tabs` ->
    /// "`split_detail` is exact at its degenerate heights" — heights 3 and
    /// 16 are the two samples above the degenerate band.
    #[test]
    fn split_detail_gives_a_header_row_a_tab_row_and_a_content_area() {
        for width in [78u16, 58u16] {
            let at = |height: u16| Rect::new(3, 5, width, height);

            let (header, tabs, content) = split_detail(at(3));
            assert_eq!(header, Rect::new(3, 5, width, 1), "width {width} height 3");
            assert_eq!(tabs, Rect::new(3, 6, width, 1), "width {width} height 3");
            assert_eq!(content, Rect::new(3, 7, width, 1), "width {width} height 3");

            let (header, tabs, content) = split_detail(at(16));
            assert_eq!(header, Rect::new(3, 5, width, 1), "width {width} height 16");
            assert_eq!(tabs, Rect::new(3, 6, width, 1), "width {width} height 16");
            assert_eq!(
                content,
                Rect::new(3, 7, width, 14),
                "width {width} height 16"
            );
        }
    }

    /// Every one of the three returned rects carries the interior's own `x`
    /// and `width`, at both mandated widths and at a non-zero `x` — the
    /// clause `split_detail`'s degenerate-heights scenario names alongside
    /// the row heights, checked here as its own discriminating assertion
    /// rather than folded into the height tables above.
    #[test]
    fn split_detail_rects_all_carry_the_interiors_x_and_width() {
        for width in [78u16, 58u16] {
            for height in [0u16, 1, 2, 3, 16] {
                let interior = Rect::new(11, 5, width, height);
                let (header, tabs, content) = split_detail(interior);
                for (name, rect) in [("header", header), ("tabs", tabs), ("content", content)] {
                    assert_eq!(
                        rect.x, interior.x,
                        "width {width} height {height}: {name}.x"
                    );
                    assert_eq!(
                        rect.width, interior.width,
                        "width {width} height {height}: {name}.width"
                    );
                }
            }
        }
    }
}
