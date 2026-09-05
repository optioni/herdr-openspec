//! The event loop: draw first, then wait. See
//! `openspec/changes/tui-shell/specs/dashboard-loop/spec.md`.

use std::time::Duration;

use ratatui::Terminal;
use ratatui::backend::Backend;

use crate::ui::app::{ArtifactReader, Dashboard, action_for};
use crate::ui::event::{EventError, EventSource};
use crate::ui::view;

/// The tick `ui::run` passes to `run_loop`. A later change adding periodic
/// work has a wake-up already in place.
pub const TICK: Duration = Duration::from_millis(250);

/// Draws performed and `next_event` calls made, over a run that ended
/// because `dashboard.quit` was set. Draw count and poll count are
/// otherwise unobservable, and are what distinguishes "drew before
/// waiting" from "waited first".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoopSummary {
    pub frames: usize,
    pub polls: usize,
}

/// Why the loop ended without `dashboard.quit` being set.
#[derive(Debug)]
pub enum LoopError {
    /// A draw failed. Carries the backend error's `Display` text.
    Draw(String),
    /// The event source failed.
    Events(EventError),
}

/// Draw, then wait up to `tick` for an event, applying its action and
/// breaking when `dashboard.quit` is set — without drawing again. A
/// timeout (`Ok(None)`) is not an event and does not end the loop. Neither
/// a draw error nor an event-source error is retried in a loop that could
/// spin. `read` is not yet called: `detail-view`'s group 10 is what wires
/// `dashboard.sync_detail(read)` into this loop, before the draw.
pub fn run_loop<B: Backend, E: EventSource>(
    terminal: &mut Terminal<B>,
    dashboard: &mut Dashboard,
    events: &mut E,
    read: ArtifactReader<'_>,
    tick: Duration,
) -> Result<LoopSummary, LoopError> {
    let _ = read;
    let mut frames = 0usize;
    let mut polls = 0usize;
    loop {
        let completed = terminal
            .draw(|frame| view::render(frame, dashboard))
            .map_err(|e| LoopError::Draw(e.to_string()))?;
        // `CompletedFrame` borrows `terminal`; `area` is `Copy`, so it is
        // copied out here and the borrow ends before `normalise_scroll`
        // takes `dashboard` mutably.
        let area = completed.area;
        frames += 1;
        dashboard.normalise_scroll(area);

        let event = events.next_event(tick).map_err(LoopError::Events)?;
        polls += 1;

        if let Some(event) = event {
            let action = action_for(&event, dashboard.filter.active);
            dashboard.apply(action);
            if dashboard.quit {
                break;
            }
        }
    }
    Ok(LoopSummary { frames, polls })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ratatui::backend::{Backend, ClearType, TestBackend};
    use ratatui::buffer::Cell;
    use ratatui::crossterm::event::{Event, KeyCode, KeyModifiers};
    use ratatui::layout::{Position, Size};

    use crate::changes::empty_set;
    use crate::testutil::{Script, cell, press, row_text};
    use crate::ui::app::{Dashboard, Route};
    use crate::ui::driver::{LoopError, LoopSummary, TICK, run_loop};
    use crate::ui::view;

    fn dashboard() -> Dashboard {
        Dashboard {
            repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
            searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
            changes: empty_set(),
            route: Route::List,
            quit: false,
            selected: 0,
            filter: crate::ui::app::Filter {
                query: String::new(),
                active: false,
            },
            detail: crate::ui::app::Detail {
                source: String::new(),
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
            },
        }
    }

    /// A `Backend` whose `draw` always errors. The other nine methods are
    /// never expected to matter to `run_loop`.
    #[derive(Debug)]
    struct FailingBackendError;

    impl std::fmt::Display for FailingBackendError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "FailingBackend always fails to draw")
        }
    }

    impl std::error::Error for FailingBackendError {}

    struct FailingBackend;

    impl Backend for FailingBackend {
        type Error = FailingBackendError;

        fn draw<'a, I>(&mut self, _content: I) -> Result<(), Self::Error>
        where
            I: Iterator<Item = (u16, u16, &'a Cell)>,
        {
            Err(FailingBackendError)
        }

        fn hide_cursor(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }

        fn show_cursor(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }

        fn get_cursor_position(&mut self) -> Result<Position, Self::Error> {
            Ok(Position::default())
        }

        fn set_cursor_position<P: Into<Position>>(
            &mut self,
            _position: P,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn clear(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }

        fn clear_region(&mut self, _clear_type: ClearType) -> Result<(), Self::Error> {
            Ok(())
        }

        fn size(&self) -> Result<Size, Self::Error> {
            Ok(Size::new(60, 20))
        }

        fn window_size(&mut self) -> Result<ratatui::backend::WindowSize, Self::Error> {
            Ok(ratatui::backend::WindowSize {
                columns_rows: Size::new(60, 20),
                pixels: Size::new(0, 0),
            })
        }

        fn flush(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[test]
    fn tick_is_250_milliseconds() {
        // dashboard-loop: "ui::driver::TICK SHALL be 250 milliseconds and SHALL
        // be what ui::run passes" — the second half is structurally visible at
        // src/ui/mod.rs's run_loop call site; this pins the value itself, the
        // same way ui::layout pins WIDE_MIN_WIDTH.
        assert_eq!(TICK, Duration::from_millis(250));
    }

    #[test]
    fn first_frame_precedes_the_first_poll() {
        let backend = TestBackend::new(120, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = dashboard();
        let mut events = Script::new(Vec::new());

        let result = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        );
        assert!(matches!(result, Err(LoopError::Events(_))));

        let buf = terminal.backend().buffer();
        assert_eq!(&row_text(buf, 0)[0..8], "OpenSpec");
        assert_eq!(cell(buf, 0, 1).symbol(), "┌");
        assert_eq!(cell(buf, 40, 1).symbol(), "┌");
    }

    #[test]
    fn timeouts_are_not_events() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = dashboard();
        let tick = Duration::from_millis(37);
        let mut events = Script::new(vec![
            Ok(None),
            Ok(None),
            Ok(None),
            Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
        ]);

        let summary = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &|_: &std::path::Path| Ok(String::new()),
            tick,
        )
        .expect("loop ends");
        assert_eq!(
            summary,
            LoopSummary {
                frames: 4,
                polls: 4
            }
        );
        assert!(dashboard.quit);
        assert!(events.timeouts().iter().all(|t| *t == tick));
    }

    #[test]
    fn ctrl_c_ends_the_loop() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = dashboard();
        let mut events = Script::new(vec![Ok(Some(press(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        )))]);

        let summary = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("loop ends");
        assert_eq!(
            summary,
            LoopSummary {
                frames: 1,
                polls: 1
            }
        );
        assert!(dashboard.quit);
    }

    #[test]
    fn filter_mode_is_passed_to_action_for() {
        // dashboard-loop's one call site now passes dashboard.filter.active
        // through: `/` starts filter mode, and a `q` typed while filtering
        // does not quit the loop — only `Ctrl-C` does.
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = dashboard();
        let mut events = Script::new(vec![
            Ok(Some(press(KeyCode::Char('/'), KeyModifiers::NONE))),
            Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
            Ok(Some(press(KeyCode::Char('c'), KeyModifiers::CONTROL))),
        ]);

        let summary = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("loop ends");
        assert_eq!(
            summary,
            LoopSummary {
                frames: 3,
                polls: 3
            }
        );
        assert_eq!(dashboard.filter.query, "q");
        assert!(dashboard.quit);

        // Render the same dashboard at 120x20 too, so this test names both
        // mandated widths.
        let _ = crate::testutil::render_at(120, 20, &dashboard);
    }

    #[test]
    fn ignored_input_redraws_and_continues() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = dashboard();
        let mut events = Script::new(vec![
            Ok(Some(press(KeyCode::Char('Q'), KeyModifiers::SHIFT))),
            Ok(Some(Event::Resize(60, 20))),
            Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
        ]);

        let summary = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("loop ends");
        assert_eq!(
            summary,
            LoopSummary {
                frames: 3,
                polls: 3
            }
        );
        assert_eq!(dashboard.route, Route::List);
    }

    #[test]
    fn route_change_shows_in_the_next_frame() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = dashboard();
        let mut events = Script::new(vec![
            Ok(Some(press(KeyCode::Enter, KeyModifiers::NONE))),
            Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
        ]);

        let summary = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("loop ends");
        assert_eq!(
            summary,
            LoopSummary {
                frames: 2,
                polls: 2
            }
        );

        let buf = terminal.backend().buffer();
        let row1: String = row_text(buf, 1).chars().skip(1).take(6).collect();
        assert_eq!(row1, "Detail");
        assert!(!(0..buf.area.height).any(|y| row_text(buf, y).contains("Changes")));
    }

    fn twenty_line_detail_dashboard() -> Dashboard {
        Dashboard {
            repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
            searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
            changes: empty_set(),
            route: Route::Detail,
            quit: false,
            selected: 0,
            filter: crate::ui::app::Filter {
                query: String::new(),
                active: false,
            },
            detail: crate::ui::app::Detail {
                source: (0..20).map(|i| format!("- line-{i:02}\n")).collect(),
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
            },
        }
    }

    #[test]
    fn scrolling_past_the_end_is_normalised_on_the_next_frame() {
        for width in [120u16, 60] {
            let backend = TestBackend::new(width, 20);
            let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
            let mut dashboard = twenty_line_detail_dashboard();
            let mut presses: Vec<_> = (0..10)
                .map(|_| Ok(Some(press(KeyCode::Char('j'), KeyModifiers::NONE))))
                .collect();
            presses.push(Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))));
            let mut events = Script::new(presses);

            let summary = run_loop(
                &mut terminal,
                &mut dashboard,
                &mut events,
                &|_: &std::path::Path| Ok(String::new()),
                Duration::from_millis(1),
            )
            .expect("loop ends");

            assert_eq!(dashboard.detail.scroll, 4, "width {width}");
            assert_eq!(
                summary,
                LoopSummary {
                    frames: 11,
                    polls: 11
                },
                "width {width}"
            );

            let buf = terminal.backend().buffer();
            let base = if width == 60 { 1 } else { 41 };
            let row_at = |y: u16| -> String {
                (base..base + 9)
                    .map(|x| row_text(buf, y).chars().nth(x as usize).unwrap())
                    .collect()
            };
            assert_eq!(row_at(2), "- line-04", "width {width}");
            assert_eq!(row_at(17), "- line-19", "width {width}");
        }
    }

    #[test]
    fn a_resize_renormalises_the_offset() {
        // Drives the pair `terminal.draw` then `normalise_scroll` directly
        // — exactly one iteration of `run_loop` — since the loop borrows
        // the terminal for its whole run and no scripted source can
        // resize the backend from inside it.
        let base = twenty_line_detail_dashboard();
        let mut dashboard = Dashboard {
            repo: base.repo,
            searched_from: base.searched_from,
            changes: base.changes,
            route: base.route,
            quit: base.quit,
            selected: base.selected,
            filter: base.filter,
            detail: crate::ui::app::Detail {
                source: base.detail.source,
                scroll: 4,
                tab: base.detail.tab,
                problems: base.detail.problems,
                loaded: base.detail.loaded,
            },
        };
        let backend = TestBackend::new(120, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");

        let frame = terminal
            .draw(|f| view::render(f, &dashboard))
            .expect("draw first frame");
        let area = frame.area;
        dashboard.normalise_scroll(area);
        assert_eq!(dashboard.detail.scroll, 4);

        terminal.backend_mut().resize(120, 30);
        let frame = terminal
            .draw(|f| view::render(f, &dashboard))
            .expect("draw second frame");
        let area = frame.area;
        dashboard.normalise_scroll(area);
        assert_eq!(dashboard.detail.scroll, 0);

        let buf = terminal.backend().buffer();
        let row2: String = row_text(buf, 2).chars().skip(41).take(9).collect();
        assert_eq!(row2, "- line-00");
    }

    #[test]
    fn a_draw_failure_stops_before_polling() {
        let mut terminal = ratatui::Terminal::new(FailingBackend).expect("construct terminal");
        let mut dashboard = dashboard();
        let mut events = Script::new(vec![Ok(Some(press(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
        )))]);

        let result = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        );
        match result {
            Err(LoopError::Draw(detail)) => {
                assert!(detail.contains("FailingBackend always fails to draw"));
            }
            other => panic!("expected Err(LoopError::Draw(_)), got {other:?}"),
        }
        assert_eq!(events.calls(), 0);
    }
}
