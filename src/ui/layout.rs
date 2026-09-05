//! The 100-column breakpoint and the frame split. Pure `Rect` arithmetic —
//! no filesystem, process, environment, network, or standard-I/O API. See
//! `openspec/changes/tui-shell/specs/responsive-layout/spec.md`.

use ratatui::layout::{Constraint, Layout, Rect};

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

#[cfg(test)]
mod tests {
    use crate::ui::app::Route;
    use crate::ui::layout::{
        LayoutMode, WIDE_MIN_WIDTH, interior, mode, scroll_offset, split_body, split_frame,
        viewport,
    };
    use ratatui::layout::Rect;
    use ratatui::widgets::Block;

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
}
