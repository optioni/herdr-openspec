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

/// Sync the selected tab's content through `read`, draw, then wait up to
/// `tick` for an event, applying its action and breaking when
/// `dashboard.quit` is set — without syncing or drawing again. The sync
/// happens **before** the draw, so the very first frame carries the
/// selected artifact's content rather than a blank region that fills in on
/// the second. A timeout (`Ok(None)`) is not an event and does not end the
/// loop. Neither a draw error nor an event-source error is retried in a
/// loop that could spin.
pub fn run_loop<B: Backend, E: EventSource>(
    terminal: &mut Terminal<B>,
    dashboard: &mut Dashboard,
    events: &mut E,
    read: ArtifactReader<'_>,
    tick: Duration,
) -> Result<LoopSummary, LoopError> {
    let mut frames = 0usize;
    let mut polls = 0usize;
    loop {
        dashboard.sync_detail(read);
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
        // A real selected change with a real artifact PATH, not
        // `empty_set()` and not a path-free artifact: `run_loop` now
        // calls `sync_detail` before every draw, and a path-free artifact
        // would clear `detail.source` back to empty on the very first
        // iteration. Tests driving this dashboard through `run_loop` pass
        // a reader supplying `detail.source`'s own twenty-line text for
        // that path, so the manually-set source and the injected reader
        // agree, exactly as `ui::mod`'s acceptance test does.
        let change = crate::changes::fixture::with_artifacts(
            crate::changes::fixture::active("detail-view", 4, 9),
            &[("proposal", &["/repo/p.md"])],
        );
        Dashboard {
            repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
            searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
            changes: crate::changes::fixture::set(vec![change], Vec::new(), Vec::new()),
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

            let recorder =
                crate::testutil::RecordingReader::always(Ok(dashboard.detail.source.clone()));
            let read = |p: &std::path::Path| recorder.read(p);

            let summary = run_loop(
                &mut terminal,
                &mut dashboard,
                &mut events,
                &read,
                Duration::from_millis(1),
            )
            .expect("loop ends");

            assert_eq!(dashboard.detail.scroll, 6, "width {width}");
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
            // The content area starts two rows lower than the interior
            // (the header and tab bar rows above it) and is two rows
            // shorter, and `normalise_scroll` now clamps against the
            // content area's own height, so the 14-row content area draws
            // lines 6 through 19, not 4 through 19.
            assert_eq!(row_at(4), "- line-06", "width {width}");
            assert_eq!(row_at(17), "- line-19", "width {width}");
        }
    }

    /// The checklist's own text: one heading, twenty unchecked items —
    /// group 5's grammar over `task-parsing`'s parse, so its line count
    /// (heading + bar + blank + twenty items) genuinely differs from what
    /// `ui::markdown::lines` produces for the same bytes.
    fn twenty_task_source() -> String {
        std::iter::once("## Tasks\n".to_string())
            .chain((0..20).map(|i| format!("- [ ] line-{i:02}\n")))
            .collect()
    }

    /// A dashboard whose one selected change carries two artifacts —
    /// `proposal` (unmarked) at position 0 and `tasks` (marked
    /// `tracks_tasks`) at position 1 — with the tracked-tasks tab
    /// selected. `detail-scroll`'s two group-8 scenarios both need a real
    /// tab to move *away from* the checklist to, which a single-artifact
    /// change cannot provide.
    fn twenty_task_detail_dashboard() -> Dashboard {
        let change = crate::changes::fixture::track_tasks_at(
            crate::changes::fixture::with_artifacts(
                crate::changes::fixture::active("detail-view", 0, 20),
                &[
                    ("proposal", &["/repo/p.md"]),
                    ("tasks", &["/repo/tasks.md"]),
                ],
            ),
            1,
        );
        Dashboard {
            repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
            searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
            changes: crate::changes::fixture::set(vec![change], Vec::new(), Vec::new()),
            route: Route::Detail,
            quit: false,
            selected: 0,
            filter: crate::ui::app::Filter {
                query: String::new(),
                active: false,
            },
            detail: crate::ui::app::Detail {
                source: String::new(),
                scroll: 0,
                tab: 1,
                problems: Vec::new(),
                loaded: None,
            },
        }
    }

    #[test]
    fn checklist_scroll_is_clamped() {
        for width in [120u16, 60] {
            let backend = TestBackend::new(width, 20);
            let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
            let mut dashboard = twenty_task_detail_dashboard();
            let source = twenty_task_source();

            let mut presses: Vec<_> = (0..20)
                .map(|_| Ok(Some(press(KeyCode::Char('j'), KeyModifiers::NONE))))
                .collect();
            presses.push(Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))));
            let mut events = Script::new(presses);

            let recorder = crate::testutil::RecordingReader::always(Ok(source.clone()));
            let read = |p: &std::path::Path| recorder.read(p);

            run_loop(
                &mut terminal,
                &mut dashboard,
                &mut events,
                &read,
                Duration::from_millis(1),
            )
            .expect("loop ends");

            // heading (1) + bar (1) + blank (1) + twenty items (20) = 23
            // lines; a 14-row content area clamps to 23 - 14 = 9, not 20.
            assert_eq!(dashboard.detail.scroll, 9, "width {width}");

            let interior = if width == 60 { 58 } else { 78 };
            let markdown_len = crate::ui::markdown::lines(&source, interior).len();
            let markdown_clamp = markdown_len.saturating_sub(14);
            assert_ne!(
                dashboard.detail.scroll, markdown_clamp,
                "width {width}: the clamp must differ from the markdown body's own"
            );

            let buf = terminal.backend().buffer();
            let base = if width == 60 { 1 } else { 41 };
            let row_at = |y: u16, len: usize| -> String {
                (base..base + len as u16)
                    .map(|x| row_text(buf, y).chars().nth(x as usize).unwrap())
                    .collect()
            };
            assert_eq!(
                row_at(17, 11),
                "[ ] line-19",
                "width {width}: the clamp used the body that was actually drawn"
            );
        }
    }

    #[test]
    fn tab_move_resets_and_reclamps() {
        for width in [120u16, 60] {
            let backend = TestBackend::new(width, 20);
            let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
            let mut dashboard = twenty_task_detail_dashboard();
            let source = twenty_task_source();
            let recorder = crate::testutil::RecordingReader::always(Ok(source.clone()));
            let read = |p: &std::path::Path| recorder.read(p);

            // First: reach the same clamped state `checklist_scroll_is_clamped`
            // does — twenty `j` presses over the checklist, then quit.
            let mut first: Vec<_> = (0..20)
                .map(|_| Ok(Some(press(KeyCode::Char('j'), KeyModifiers::NONE))))
                .collect();
            first.push(Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))));
            let mut events = Script::new(first);
            run_loop(
                &mut terminal,
                &mut dashboard,
                &mut events,
                &read,
                Duration::from_millis(1),
            )
            .expect("first stage ends");
            assert_eq!(dashboard.detail.scroll, 9, "width {width}: precondition");

            // The same run continues: a Press of `1` — selecting the
            // unmarked `proposal` artifact — then quit, to check the
            // reset in isolation before the further `j` presses reclamp
            // it against a new body.
            dashboard.quit = false;
            let mut select_tab = vec![
                Ok(Some(press(KeyCode::Char('1'), KeyModifiers::NONE))),
                Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
            ];
            let mut events_select = Script::new(std::mem::take(&mut select_tab));
            run_loop(
                &mut terminal,
                &mut dashboard,
                &mut events_select,
                &read,
                Duration::from_millis(1),
            )
            .expect("tab-move stage ends");
            assert_eq!(dashboard.detail.tab, 0, "width {width}");
            assert_eq!(
                dashboard.detail.scroll, 0,
                "width {width}: reset immediately, because sync_detail's key changed"
            );

            // Ten more `j` presses, then quit.
            dashboard.quit = false;
            let mut second: Vec<_> = (0..10)
                .map(|_| Ok(Some(press(KeyCode::Char('j'), KeyModifiers::NONE))))
                .collect();
            second.push(Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))));
            let mut events2 = Script::new(second);
            run_loop(
                &mut terminal,
                &mut dashboard,
                &mut events2,
                &read,
                Duration::from_millis(1),
            )
            .expect("second stage ends");
            let interior = if width == 60 { 58 } else { 78 };
            let markdown_len = crate::ui::markdown::lines(&source, interior).len();
            let markdown_clamp = markdown_len.saturating_sub(14);
            assert_eq!(
                dashboard.detail.scroll, markdown_clamp,
                "width {width}: clamped against the markdown body's own length, \
                 not the checklist's"
            );
            assert_ne!(
                dashboard.detail.scroll, 9,
                "width {width}: not the checklist's own clamp"
            );
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
        let row: String = row_text(buf, 4).chars().skip(41).take(9).collect();
        assert_eq!(row, "- line-00");
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

    /// A dashboard whose one selected active change carries one artifact
    /// resolving to a real path, for the two `sync_detail`-before-the-draw
    /// tests below — `twenty_line_detail_dashboard`'s artifact resolves to
    /// no paths at all, so its reader would never be called.
    fn dashboard_with_one_artifact_path() -> Dashboard {
        let change = crate::changes::fixture::with_artifacts(
            crate::changes::fixture::active("detail-view", 4, 9),
            &[("proposal", &["/repo/p.md"])],
        );
        Dashboard {
            repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
            searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
            changes: crate::changes::fixture::set(vec![change], Vec::new(), Vec::new()),
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

    #[test]
    fn the_loop_syncs_before_it_draws() {
        let backend = TestBackend::new(120, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = dashboard_with_one_artifact_path();
        let mut events = Script::new(vec![Ok(Some(press(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
        )))]);
        let recorder = crate::testutil::RecordingReader::always(Ok("# proposal\n".to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        let summary = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &read,
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
        let buf = terminal.backend().buffer();
        let row: String = row_text(buf, 4).chars().skip(41).take(10).collect();
        assert_eq!(row, "# proposal");
        assert_eq!(recorder.calls(), 1);
    }

    #[test]
    fn a_backend_draw_failure_still_records_the_syncs_one_call() {
        let mut terminal = ratatui::Terminal::new(FailingBackend).expect("construct terminal");
        let mut dashboard = dashboard_with_one_artifact_path();
        let mut events = Script::new(vec![Ok(Some(press(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
        )))]);
        let recorder = crate::testutil::RecordingReader::always(Ok("# proposal\n".to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        let result = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &read,
            Duration::from_millis(1),
        );
        assert!(matches!(result, Err(LoopError::Draw(_))));
        assert_eq!(events.calls(), 0);
        assert_eq!(recorder.calls(), 1, "the sync precedes the draw");
    }
}
