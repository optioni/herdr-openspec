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

/// The loop's live-tier collaborators: the watcher, the worker, and the agent
/// poller, all reached only as non-blocking trait objects. A struct rather
/// than three further parameters, so `run_loop`'s signature stays at six
/// arguments and the three collaborators are named as one concept. Group 1
/// plumbs the third field through with no behaviour; `run_loop`'s body reads
/// it starting group 9.
pub struct Live<'a> {
    pub fs: &'a mut dyn crate::watch::FsEvents,
    pub refresher: &'a mut dyn crate::refresh::Refresher,
    pub agents: &'a mut dyn crate::agents::AgentPoll,
    /// `agent-launch`'s addition: a fourth field, not a seventh parameter — the launcher is
    /// reached only through this trait object, never through the state value.
    pub launcher: &'a mut dyn crate::launch::Launcher,
}

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

/// Drive the live tier's six one-shot steps, then sync the selected tab's
/// content through `read`, draw, then wait up to
/// `watch::poll_timeout(tick, watch::soonest(live.fs.pending_in(), live.agents.pending_in()))`
/// for an event, applying its action and breaking when `dashboard.quit` is
/// set — without syncing or drawing again. The sync happens **before** the
/// draw, so the very first frame carries the selected artifact's content
/// rather than a blank region that fills in on the second. A timeout
/// (`Ok(None)`) is not an event and does not end the loop. Neither a draw
/// error nor an event-source error is retried in a loop that could spin.
///
/// The live tier, once per iteration, before the sync:
///
/// 1. `dashboard.launch.pending.take()`, if set, is handed to
///    `live.launcher.request` — `agent-launch`'s addition, leading the
///    iteration because it answers a key pressed at the end of the previous
///    one; taking the request is what makes "handed over exactly once" true
///    even when `apply` set it on the very last event before a quit.
/// 2. When `dashboard.refresh.requested` is set, request `Selection::All`
///    and clear the flag — checked **before** the filesystem drain below, so
///    the startup (or `r`-triggered) request is recorded before the request
///    an already-pending batch produces on the same iteration:
///    `ui::load`'s startup flag and a live watcher's first-ever batch can
///    both be pending on iteration one.
/// 3. `live.fs.drain()`; on `Ok(Some(paths))`, request
///    `watch::invalidate(repo, &paths)`; on `Err(e)`, the reason replaces
///    `dashboard.refresh.problems` wholesale (never grown) — a watcher
///    failing on every poll must not accumulate an unbounded list.
/// 4. `live.refresher.take_result()`; any result — file-sourced or
///    CLI-merged — is adopted immediately, so it reaches the very frame
///    that follows rather than the one after.
/// 5. `live.agents.drain()`; on `Some(snapshot)`, `dashboard.agents` is
///    replaced wholesale with it — never merged, independently of step 4,
///    so a refresh landing on the same iteration as a poll cannot discard
///    the poll (`agent-polling`).
/// 6. `live.launcher.drain()`, if it answers, folds `Outcome::named` into
///    `dashboard.agent_names.names` and replaces `dashboard.launch.problems`
///    wholesale with `Outcome::problem` — `agent-launch`'s addition,
///    trailing the iteration so a launch that has just recorded a mapping
///    is visible to the very next `attribution()` call, in the frame the
///    draw below produces.
///
/// Every one of the six steps is non-blocking by the traits' contract, so
/// the sequence adds no wait to the render path. See
/// `specs/live-updates/spec.md` -> "The loop drives the live tier without
/// ever waiting on it".
pub fn run_loop<B: Backend, E: EventSource>(
    terminal: &mut Terminal<B>,
    dashboard: &mut Dashboard,
    events: &mut E,
    live: &mut Live<'_>,
    read: ArtifactReader<'_>,
    tick: Duration,
) -> Result<LoopSummary, LoopError> {
    let mut frames = 0usize;
    let mut polls = 0usize;
    loop {
        drive_live_tier(dashboard, live);

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

        let timeout = crate::watch::poll_timeout(
            tick,
            crate::watch::soonest(live.fs.pending_in(), live.agents.pending_in()),
        );
        let event = events.next_event(timeout).map_err(LoopError::Events)?;
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

/// The live tier's four one-shot steps — see `run_loop`'s doc comment for
/// the order and the reason for it. Non-blocking throughout: it calls only
/// `FsEvents::drain`, `Refresher::request`, `Refresher::take_result`, and
/// `AgentPoll::drain`, every one of which is non-blocking by the traits'
/// own contract, so extracting this into its own function grows no ability
/// to block that `run_loop`'s body did not already have.
fn drive_live_tier(dashboard: &mut Dashboard, live: &mut Live<'_>) {
    // `agent-launch`'s step 1: leads the iteration because it answers a key pressed at the
    // end of the previous one. Taking the request — leaving `None` — is what makes "handed
    // over exactly once" true even when `apply` set it on the very last event before a quit.
    // `seam-resilience`'s addition: `in_flight` is set here, and only here, when and only
    // when the request taken was a `Request::Launch` — a `Request::Focus` starts no agent
    // and splits no pane and therefore cannot leak one.
    if let Some(request) = dashboard.launch.pending.take() {
        // Named fields, not `..`: `NODEFAULT-UI`'s `TYPES='Launch'` leg matches this bare
        // identifier too (it has no way to tell `launch::Request::Launch` from
        // `ui::app::Launch` apart), so a `..` here would read as an elided field on a type
        // that is not this one at all.
        if matches!(
            request,
            crate::launch::Request::Launch {
                change: _,
                agent: _,
                intent: _
            }
        ) {
            dashboard.launch.in_flight = true;
        }
        live.launcher.request(request);
    }
    if dashboard.refresh.requested {
        live.refresher.request(crate::changes::Selection::All);
        dashboard.refresh.requested = false;
    }
    match live.fs.drain() {
        Ok(Some(paths)) => {
            if let Some(repo) = dashboard.repo.as_deref() {
                let selection = crate::watch::invalidate(repo, &paths);
                // `seam-resilience`: an empty `Selection::Only` invalidates nothing, and
                // requesting one costs a full `changes::from_files` re-walk plus an
                // `openspec list --json` Node start for no reason — the short-circuit lives
                // here, at the caller, so `invalidate` itself stays a pure total function.
                let invalidates_nothing =
                    matches!(&selection, crate::changes::Selection::Only(s) if s.is_empty());
                if !invalidates_nothing {
                    live.refresher.request(selection);
                }
            }
        }
        Ok(None) => {}
        Err(e) => {
            dashboard.refresh.problems = vec![e.0];
        }
    }
    if let Some(result) = live.refresher.take_result() {
        match result {
            crate::refresh::RefreshResult::Files(set)
            | crate::refresh::RefreshResult::Merged(set) => dashboard.adopt(set),
            // `seam-resilience`: a stopped worker replaces `refresh.problems` wholesale and
            // runs no `adopt` — the change set on screen is the last true one, and replacing
            // it with an empty set would make a dead worker look like an empty repository.
            crate::refresh::RefreshResult::Stopped(reason) => {
                dashboard.refresh.problems = vec![reason];
            }
        }
    }
    if let Some(snapshot) = live.agents.drain() {
        dashboard.agents = snapshot;
    }
    // `agent-launch`'s step 6: follows step 5 so a launch that has just recorded a mapping is
    // visible to the very next `attribution()` call, in the frame the draw below produces.
    // `seam-resilience`'s addition: `in_flight` is cleared here, whether the outcome is a
    // real answer or the launcher reporting its own worker dead — no further answer will
    // ever come either way.
    if let Some(outcome) = live.launcher.drain() {
        dashboard.launch.in_flight = false;
        if let Some((agent, change)) = outcome.named {
            dashboard.agent_names.names.insert(agent, change);
        }
        dashboard.launch.problems = outcome.problems;
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ratatui::backend::{Backend, ClearType, TestBackend};
    use ratatui::buffer::Cell;
    use ratatui::crossterm::event::{Event, KeyCode, KeyModifiers};
    use ratatui::layout::{Position, Size};

    use crate::changes::empty_set;
    use crate::testutil::{RecordingRefresher, Script, ScriptedFs, cell, press, row_text};
    use crate::ui::app::{Dashboard, Route};
    use crate::ui::driver::{Live, LoopError, LoopSummary, TICK, run_loop};
    use crate::ui::view;

    /// A `Route::List` dashboard over one active change, `name`, at
    /// `completed` of `total` — group 9's live-tier tests need a dashboard
    /// whose list row carries an assertable progress pair, not
    /// `dashboard()`'s empty set.
    fn dashboard_with_change(repo: &str, name: &str, completed: usize, total: usize) -> Dashboard {
        Dashboard {
            repo: Some(std::path::PathBuf::from(repo)),
            searched_from: std::path::PathBuf::from(repo),
            changes: crate::changes::fixture::set(
                vec![crate::changes::fixture::active(name, completed, total)],
                Vec::new(),
                Vec::new(),
            ),
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

        let mut fs = crate::watch::none();
        let mut refresher = crate::refresh::none();
        let mut agents = crate::agents::none();
        let mut launcher = crate::launch::none();
        let mut live = crate::ui::driver::Live {
            fs: &mut *fs,
            refresher: &mut *refresher,
            agents: &mut *agents,
            launcher: &mut *launcher,
        };
        let result = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
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

        let mut fs = crate::watch::none();
        let mut refresher = crate::refresh::none();
        let mut agents = crate::agents::none();
        let mut launcher = crate::launch::none();
        let mut live = crate::ui::driver::Live {
            fs: &mut *fs,
            refresher: &mut *refresher,
            agents: &mut *agents,
            launcher: &mut *launcher,
        };
        let summary = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
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

        let mut fs = crate::watch::none();
        let mut refresher = crate::refresh::none();
        let mut agents = crate::agents::none();
        // `agent-launch`: a recording launcher, so "Ctrl-C did not launch" is asserted rather
        // than merely assumed of the inert double.
        let mut launcher = crate::testutil::RecordingLauncher::new();
        let mut live = crate::ui::driver::Live {
            fs: &mut *fs,
            refresher: &mut *refresher,
            agents: &mut *agents,
            launcher: &mut launcher,
        };
        let summary = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
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
        assert!(
            launcher.requests().is_empty(),
            "Ctrl-C must not launch anything"
        );
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

        let mut fs = crate::watch::none();
        let mut refresher = crate::refresh::none();
        let mut agents = crate::agents::none();
        let mut launcher = crate::launch::none();
        let mut live = crate::ui::driver::Live {
            fs: &mut *fs,
            refresher: &mut *refresher,
            agents: &mut *agents,
            launcher: &mut *launcher,
        };
        let summary = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
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

        let mut fs = crate::watch::none();
        let mut refresher = crate::refresh::none();
        let mut agents = crate::agents::none();
        let mut launcher = crate::launch::none();
        let mut live = crate::ui::driver::Live {
            fs: &mut *fs,
            refresher: &mut *refresher,
            agents: &mut *agents,
            launcher: &mut *launcher,
        };
        let summary = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
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

        let mut fs = crate::watch::none();
        let mut refresher = crate::refresh::none();
        let mut agents = crate::agents::none();
        let mut launcher = crate::launch::none();
        let mut live = crate::ui::driver::Live {
            fs: &mut *fs,
            refresher: &mut *refresher,
            agents: &mut *agents,
            launcher: &mut *launcher,
        };
        let summary = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
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

            let mut fs = crate::watch::none();
            let mut refresher = crate::refresh::none();
            let mut agents = crate::agents::none();
            let mut launcher = crate::launch::none();
            let mut live = crate::ui::driver::Live {
                fs: &mut *fs,
                refresher: &mut *refresher,
                agents: &mut *agents,
                launcher: &mut *launcher,
            };
            let summary = run_loop(
                &mut terminal,
                &mut dashboard,
                &mut events,
                &mut live,
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

            let mut fs = crate::watch::none();
            let mut refresher = crate::refresh::none();
            let mut agents = crate::agents::none();
            let mut launcher = crate::launch::none();
            let mut live = crate::ui::driver::Live {
                fs: &mut *fs,
                refresher: &mut *refresher,
                agents: &mut *agents,
                launcher: &mut *launcher,
            };
            run_loop(
                &mut terminal,
                &mut dashboard,
                &mut events,
                &mut live,
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
            let mut fs = crate::watch::none();
            let mut refresher = crate::refresh::none();
            let mut agents = crate::agents::none();
            let mut launcher = crate::launch::none();
            let mut live = crate::ui::driver::Live {
                fs: &mut *fs,
                refresher: &mut *refresher,
                agents: &mut *agents,
                launcher: &mut *launcher,
            };
            run_loop(
                &mut terminal,
                &mut dashboard,
                &mut events,
                &mut live,
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
            let mut fs = crate::watch::none();
            let mut refresher = crate::refresh::none();
            let mut agents = crate::agents::none();
            let mut launcher = crate::launch::none();
            let mut live = crate::ui::driver::Live {
                fs: &mut *fs,
                refresher: &mut *refresher,
                agents: &mut *agents,
                launcher: &mut *launcher,
            };
            run_loop(
                &mut terminal,
                &mut dashboard,
                &mut events_select,
                &mut live,
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
            let mut fs = crate::watch::none();
            let mut refresher = crate::refresh::none();
            let mut agents = crate::agents::none();
            let mut launcher = crate::launch::none();
            let mut live = crate::ui::driver::Live {
                fs: &mut *fs,
                refresher: &mut *refresher,
                agents: &mut *agents,
                launcher: &mut *launcher,
            };
            run_loop(
                &mut terminal,
                &mut dashboard,
                &mut events2,
                &mut live,
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

        let mut fs = crate::watch::none();
        let mut refresher = crate::refresh::none();
        let mut agents = crate::agents::none();
        let mut launcher = crate::launch::none();
        let mut live = crate::ui::driver::Live {
            fs: &mut *fs,
            refresher: &mut *refresher,
            agents: &mut *agents,
            launcher: &mut *launcher,
        };
        let result = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
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

        let mut fs = crate::watch::none();
        let mut refresher = crate::refresh::none();
        let mut agents = crate::agents::none();
        let mut launcher = crate::launch::none();
        let mut live = crate::ui::driver::Live {
            fs: &mut *fs,
            refresher: &mut *refresher,
            agents: &mut *agents,
            launcher: &mut *launcher,
        };
        let summary = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
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

        let mut fs = crate::watch::none();
        let mut refresher = crate::refresh::none();
        let mut agents = crate::agents::none();
        let mut launcher = crate::launch::none();
        let mut live = crate::ui::driver::Live {
            fs: &mut *fs,
            refresher: &mut *refresher,
            agents: &mut *agents,
            launcher: &mut *launcher,
        };
        let result = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
            &read,
            Duration::from_millis(1),
        );
        assert!(matches!(result, Err(LoopError::Draw(_))));
        assert_eq!(events.calls(), 0);
        assert_eq!(recorder.calls(), 1, "the sync precedes the draw");
    }

    // `live-refresh` -> "The loop drives the live tier without ever waiting
    // on it". See `specs/live-updates/spec.md`.

    #[test]
    fn the_startup_request_precedes_the_first_wait() {
        for width in [120u16, 60u16] {
            let backend = TestBackend::new(width, 20);
            let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
            let mut dashboard = dashboard();
            dashboard.refresh.requested = true;
            let mut events = Script::new(vec![Ok(Some(press(
                KeyCode::Char('q'),
                KeyModifiers::NONE,
            )))]);
            let mut fs = ScriptedFs::new(Vec::new(), Vec::new());
            let mut refresher = RecordingRefresher::new(Vec::new());
            let mut agents = crate::agents::none();
            let mut launcher = crate::launch::none();
            let mut live = crate::ui::driver::Live {
                fs: &mut fs,
                refresher: &mut refresher,
                agents: &mut *agents,
                launcher: &mut *launcher,
            };
            let summary = run_loop(
                &mut terminal,
                &mut dashboard,
                &mut events,
                &mut live,
                &|_: &std::path::Path| Ok(String::new()),
                Duration::from_millis(1),
            )
            .expect("loop ends");

            assert_eq!(
                summary,
                LoopSummary {
                    frames: 1,
                    polls: 1
                },
                "width {width}"
            );
            assert_eq!(
                refresher.requests(),
                vec![crate::changes::Selection::All],
                "width {width}"
            );
            assert!(
                !dashboard.refresh.requested,
                "width {width}: one flag must produce one request"
            );
        }
    }

    #[test]
    fn a_result_is_adopted_before_the_frame() {
        for width in [120u16, 60u16] {
            let backend = TestBackend::new(width, 20);
            let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
            let mut dashboard = dashboard_with_change("/r", "alpha", 4, 9);
            let merged = crate::changes::fixture::set(
                vec![crate::changes::fixture::active("alpha", 7, 9)],
                Vec::new(),
                Vec::new(),
            );
            let mut events = Script::new(vec![Ok(Some(press(
                KeyCode::Char('q'),
                KeyModifiers::NONE,
            )))]);
            let mut fs = ScriptedFs::new(Vec::new(), Vec::new());
            let mut refresher =
                RecordingRefresher::new(vec![Some(crate::refresh::RefreshResult::Merged(merged))]);
            let mut agents = crate::agents::none();
            let mut launcher = crate::launch::none();
            let mut live = crate::ui::driver::Live {
                fs: &mut fs,
                refresher: &mut refresher,
                agents: &mut *agents,
                launcher: &mut *launcher,
            };
            let summary = run_loop(
                &mut terminal,
                &mut dashboard,
                &mut events,
                &mut live,
                &|_: &std::path::Path| Ok(String::new()),
                Duration::from_millis(1),
            )
            .expect("loop ends");

            assert_eq!(
                summary,
                LoopSummary {
                    frames: 1,
                    polls: 1
                },
                "width {width}: one frame, not two"
            );
            let buf = terminal.backend().buffer();
            assert!(
                row_text(buf, 2).contains("[7/9]"),
                "width {width}: the result must reach the very frame that consumed it: {}",
                row_text(buf, 2)
            );
            assert_eq!(
                refresher.takes(),
                1,
                "width {width}: one take_result per iteration"
            );
        }
    }

    #[test]
    fn an_fs_batch_becomes_one_selection() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = dashboard();
        dashboard.repo = Some(std::path::PathBuf::from("/r"));
        // Mimics `ui::load`'s startup state: the flag and an
        // already-pending filesystem batch can coincide on the very first
        // iteration.
        dashboard.refresh.requested = true;
        let mut events = Script::new(vec![
            Ok(None),
            Ok(None),
            Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
        ]);
        let mut fs = ScriptedFs::new(
            vec![Ok(Some(vec![std::path::PathBuf::from(
                "/r/openspec/changes/alpha/tasks.md",
            )]))],
            Vec::new(),
        );
        let mut refresher = RecordingRefresher::new(Vec::new());
        let mut agents = crate::agents::none();
        let mut launcher = crate::launch::none();
        let mut live = crate::ui::driver::Live {
            fs: &mut fs,
            refresher: &mut refresher,
            agents: &mut *agents,
            launcher: &mut *launcher,
        };
        run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("loop ends");

        let mut alpha = std::collections::BTreeSet::new();
        alpha.insert("alpha".to_string());
        assert_eq!(
            refresher.requests(),
            vec![
                crate::changes::Selection::All,
                crate::changes::Selection::Only(alpha)
            ],
            "an implementation that requested All for every batch must fail here"
        );
    }

    #[test]
    fn a_watch_error_is_recorded_once() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = dashboard();
        let mut events = Script::new(vec![
            Ok(None),
            Ok(None),
            Ok(None),
            Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
        ]);
        let mut fs = ScriptedFs::new(
            vec![
                Err(crate::watch::WatchError("watch failed".to_string())),
                Err(crate::watch::WatchError("watch failed".to_string())),
            ],
            Vec::new(),
        );
        let mut refresher = RecordingRefresher::new(Vec::new());
        let mut agents = crate::agents::none();
        let mut launcher = crate::launch::none();
        let mut live = crate::ui::driver::Live {
            fs: &mut fs,
            refresher: &mut refresher,
            agents: &mut *agents,
            launcher: &mut *launcher,
        };
        let summary = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("loop ends");

        assert_eq!(
            summary,
            LoopSummary {
                frames: 4,
                polls: 4
            },
            "a watch error must never end the loop"
        );
        assert_eq!(
            dashboard.refresh.problems.len(),
            1,
            "a watcher failing on every poll must not grow an unbounded problem list"
        );
        assert!(dashboard.refresh.problems[0].contains("watch failed"));
    }

    #[test]
    fn the_wait_shortens_to_the_debounce_deadline() {
        let make_dashboard = dashboard;
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = make_dashboard();
        let mut events = Script::new(vec![
            Ok(None),
            Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
        ]);
        let mut fs = ScriptedFs::new(
            vec![Ok(None), Ok(None)],
            vec![Some(Duration::from_millis(90)), None],
        );
        let mut refresher = RecordingRefresher::new(Vec::new());
        let mut agents = crate::agents::none();
        let mut launcher = crate::launch::none();
        let mut live = crate::ui::driver::Live {
            fs: &mut fs,
            refresher: &mut refresher,
            agents: &mut *agents,
            launcher: &mut *launcher,
        };
        run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(250),
        )
        .expect("loop ends");

        let timeouts = events.timeouts();
        assert_eq!(timeouts[0], Duration::from_millis(90));
        assert_eq!(timeouts[1], Duration::from_millis(250));

        // A `pending_in` of zero must still floor at 1ms, never 0ms — a
        // zero timeout returned every iteration would let the loop spin
        // without bound.
        let backend2 = TestBackend::new(60, 20);
        let mut terminal2 = ratatui::Terminal::new(backend2).expect("construct terminal");
        let mut dashboard2 = make_dashboard();
        let mut events2 = Script::new(vec![Ok(Some(press(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
        )))]);
        let mut fs2 = ScriptedFs::new(vec![Ok(None)], vec![Some(Duration::from_millis(0))]);
        let mut refresher2 = RecordingRefresher::new(Vec::new());
        let mut agents2 = crate::agents::none();
        let mut launcher = crate::launch::none();
        let mut live2 = crate::ui::driver::Live {
            fs: &mut fs2,
            refresher: &mut refresher2,
            agents: &mut *agents2,
            launcher: &mut *launcher,
        };
        run_loop(
            &mut terminal2,
            &mut dashboard2,
            &mut events2,
            &mut live2,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(250),
        )
        .expect("loop ends");
        assert_eq!(events2.timeouts()[0], Duration::from_millis(1));
    }

    #[test]
    fn the_wait_is_the_tick_when_nothing_is_pending() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = dashboard();
        let mut events = Script::new(vec![Ok(Some(press(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
        )))]);
        let mut fs = ScriptedFs::new(vec![Ok(None)], vec![None]);
        let mut refresher = RecordingRefresher::new(Vec::new());
        let mut agents = crate::agents::none();
        let mut launcher = crate::launch::none();
        let mut live = crate::ui::driver::Live {
            fs: &mut fs,
            refresher: &mut refresher,
            agents: &mut *agents,
            launcher: &mut *launcher,
        };
        run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(250),
        )
        .expect("loop ends");
        assert_eq!(events.timeouts()[0], Duration::from_millis(250));
    }

    #[test]
    fn a_files_result_and_a_merged_result_are_both_adopted() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = dashboard_with_change("/r", "alpha", 4, 9);
        let files_set = crate::changes::fixture::set(
            vec![crate::changes::fixture::active("alpha", 4, 9)],
            Vec::new(),
            Vec::new(),
        );
        let merged_set = crate::changes::fixture::set(
            vec![crate::changes::fixture::active("alpha", 7, 9)],
            Vec::new(),
            Vec::new(),
        );
        let mut events = Script::new(vec![
            Ok(None),
            Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
        ]);
        let mut fs = ScriptedFs::new(Vec::new(), Vec::new());
        let mut refresher = RecordingRefresher::new(vec![
            Some(crate::refresh::RefreshResult::Files(files_set)),
            Some(crate::refresh::RefreshResult::Merged(merged_set)),
        ]);
        let mut agents = crate::agents::none();
        let mut launcher = crate::launch::none();
        let mut live = crate::ui::driver::Live {
            fs: &mut fs,
            refresher: &mut refresher,
            agents: &mut *agents,
            launcher: &mut *launcher,
        };
        let summary = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
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
        assert_eq!(dashboard.changes.active[0].progress.completed, 7);
        assert_eq!(refresher.takes(), 2);
    }

    #[test]
    fn an_inert_live_tier_takes_nothing_and_requests_nothing() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = dashboard();
        // `agent-launch`: a pending request set before the run must be discarded by
        // `launch::none()`, on exactly the terms every other inert collaborator already meets.
        dashboard.launch.pending = Some(crate::launch::Request::Focus {
            pane_id: "w8:p1".to_string(),
        });
        let mut events = Script::new(vec![Ok(Some(press(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
        )))]);
        let mut fs = ScriptedFs::new(Vec::new(), Vec::new());
        let mut refresher = RecordingRefresher::new(Vec::new());
        let mut agents = crate::agents::none();
        let mut launcher = crate::launch::none();
        let mut live = crate::ui::driver::Live {
            fs: &mut fs,
            refresher: &mut refresher,
            agents: &mut *agents,
            launcher: &mut *launcher,
        };
        let summary = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
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
        assert!(
            refresher.requests().is_empty(),
            "no drain batch and no requested flag must issue no request"
        );
        assert_eq!(
            refresher.takes(),
            1,
            "take_result is still polled once, even though it yields nothing"
        );
        assert_eq!(dashboard.changes, empty_set());
        assert_eq!(
            dashboard.launch.pending, None,
            "launch::none() discards the request it was given"
        );
        assert!(dashboard.launch.problems.is_empty());
    }

    // `agent-polling`: `Live`'s third field and the loop's fourth live step.

    #[test]
    fn the_wait_takes_the_soonest_of_two_deadlines() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = dashboard();
        let mut events = Script::new(vec![
            Ok(None),
            Ok(None),
            Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
        ]);
        let mut fs = ScriptedFs::new(
            Vec::new(),
            vec![
                Some(Duration::from_millis(900)),
                Some(Duration::from_millis(90)),
                None,
            ],
        );
        let mut refresher = RecordingRefresher::new(Vec::new());
        let mut agents = crate::testutil::ScriptedAgents::new(
            Vec::new(),
            vec![
                Some(Duration::from_millis(40)),
                Some(Duration::from_millis(900)),
                Some(Duration::from_secs(3)),
            ],
        );
        let mut launcher = crate::launch::none();
        let mut live = crate::ui::driver::Live {
            fs: &mut fs,
            refresher: &mut refresher,
            agents: &mut agents,
            launcher: &mut *launcher,
        };
        run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(250),
        )
        .expect("loop ends");

        let timeouts = events.timeouts();
        assert_eq!(timeouts[0], Duration::from_millis(40));
        assert_eq!(timeouts[1], Duration::from_millis(90));
        assert_eq!(
            timeouts[2],
            Duration::from_millis(250),
            "a deadline further away than the tick must never lengthen the wait"
        );
    }

    #[test]
    fn a_snapshot_reaches_the_frame_and_survives_adopt() {
        for width in [120u16, 60u16] {
            let backend = TestBackend::new(width, 20);
            let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
            let mut dashboard = dashboard_with_change("/r", "alpha", 4, 9);
            let merged = crate::changes::fixture::set(
                vec![crate::changes::fixture::active("alpha", 7, 9)],
                Vec::new(),
                Vec::new(),
            );
            let mut events = Script::new(vec![
                Ok(None),
                Ok(None),
                Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
            ]);
            let mut fs = ScriptedFs::new(Vec::new(), Vec::new());
            let mut refresher =
                RecordingRefresher::new(vec![Some(crate::refresh::RefreshResult::Merged(merged))]);
            let agent = crate::agents::Agent {
                name: None,
                kind: Some("claude".to_string()),
                status: crate::agents::AgentStatus::Working,
                cwd: None,
                pane_id: "w8:p1".to_string(),
                tab_id: "w8:t1".to_string(),
                workspace_id: "w8".to_string(),
                terminal_title: None,
            };
            let mut agents = crate::testutil::ScriptedAgents::new(
                vec![
                    Some(crate::agents::AgentSnapshot {
                        agents: vec![agent],
                        reachable: true,
                        stalled: false,
                        problem: None,
                    }),
                    None,
                    None,
                ],
                Vec::new(),
            );
            let mut launcher = crate::launch::none();
            let mut live = crate::ui::driver::Live {
                fs: &mut fs,
                refresher: &mut refresher,
                agents: &mut agents,
                launcher: &mut *launcher,
            };
            run_loop(
                &mut terminal,
                &mut dashboard,
                &mut events,
                &mut live,
                &|_: &std::path::Path| Ok(String::new()),
                Duration::from_millis(1),
            )
            .expect("loop ends");

            assert!(dashboard.agents.reachable, "width {width}");
            assert_eq!(dashboard.agents.agents.len(), 1, "width {width}");
            assert_eq!(
                dashboard.changes.active[0].progress.completed, 7,
                "width {width}: the adopted result must still take effect"
            );

            let buf = terminal.backend().buffer();
            let inert_agents_buf = {
                let mut dashboard2 = dashboard_with_change("/r", "alpha", 4, 9);
                let mut events2 = Script::new(vec![
                    Ok(None),
                    Ok(None),
                    Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
                ]);
                let mut fs2 = ScriptedFs::new(Vec::new(), Vec::new());
                let merged2 = crate::changes::fixture::set(
                    vec![crate::changes::fixture::active("alpha", 7, 9)],
                    Vec::new(),
                    Vec::new(),
                );
                let mut refresher2 = RecordingRefresher::new(vec![Some(
                    crate::refresh::RefreshResult::Merged(merged2),
                )]);
                let mut agents2 = crate::agents::none();
                let mut launcher = crate::launch::none();
                let mut live2 = crate::ui::driver::Live {
                    fs: &mut fs2,
                    refresher: &mut refresher2,
                    agents: &mut *agents2,
                    launcher: &mut *launcher,
                };
                let backend2 = TestBackend::new(width, 20);
                let mut terminal2 = ratatui::Terminal::new(backend2).expect("construct terminal");
                run_loop(
                    &mut terminal2,
                    &mut dashboard2,
                    &mut events2,
                    &mut live2,
                    &|_: &std::path::Path| Ok(String::new()),
                    Duration::from_millis(1),
                )
                .expect("loop ends");
                terminal2.backend().buffer().clone()
            };
            // `agent-launch`: the footer row is excepted from "the snapshot changed state
            // without changing the frame" — `agents.reachable` now moves the footer's action
            // hints directly, which is this change's own addition and is proved separately by
            // `ui::view::tests::the_action_hints_follow_esc_back_when_reachable`. Every row
            // above the footer must still be byte-identical.
            for y in 0..buf.area.height.saturating_sub(1) {
                assert_eq!(
                    row_text(buf, y),
                    row_text(&inert_agents_buf, y),
                    "width {width} row {y}: the badge and count must not move without a rendered pixel elsewhere"
                );
            }
        }
    }

    #[test]
    fn an_unreachable_socket_is_not_a_problem_row() {
        for width in [120u16, 60u16] {
            let backend = TestBackend::new(width, 20);
            let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
            let mut dashboard = dashboard();
            let mut events = Script::new(vec![Ok(Some(press(
                KeyCode::Char('q'),
                KeyModifiers::NONE,
            )))]);
            let mut fs = ScriptedFs::new(Vec::new(), Vec::new());
            let mut refresher = RecordingRefresher::new(Vec::new());
            let mut agents = crate::testutil::ScriptedAgents::new(
                vec![Some(crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: Some("herdr agent list exited 1: server_not_running".to_string()),
                })],
                Vec::new(),
            );
            let mut launcher = crate::launch::none();
            let mut live = crate::ui::driver::Live {
                fs: &mut fs,
                refresher: &mut refresher,
                agents: &mut agents,
                launcher: &mut *launcher,
            };
            run_loop(
                &mut terminal,
                &mut dashboard,
                &mut events,
                &mut live,
                &|_: &std::path::Path| Ok(String::new()),
                Duration::from_millis(1),
            )
            .expect("loop ends");

            assert!(
                dashboard.refresh.problems.is_empty(),
                "width {width}: an unreachable socket must never become a problem row"
            );
            assert!(
                dashboard.launch.problems.is_empty(),
                "width {width}: an unreachable agent poll must not become a launch problem either"
            );
            assert_eq!(
                dashboard.agents.problem,
                Some("herdr agent list exited 1: server_not_running".to_string()),
                "width {width}: the reason is available to a later change without being \
                 rendered by this one"
            );
            let buf = terminal.backend().buffer();
            for y in 0..buf.area.height {
                assert!(
                    !row_text(buf, y).contains('!'),
                    "width {width}: no !-marked row: {}",
                    row_text(buf, y)
                );
            }
        }
    }

    #[test]
    fn live_destructures_into_exactly_four_fields() {
        let mut fs = crate::watch::none();
        let mut refresher = crate::refresh::none();
        let mut agents = crate::agents::none();
        let mut launcher = crate::launch::none();
        let live = crate::ui::driver::Live {
            fs: &mut *fs,
            refresher: &mut *refresher,
            agents: &mut *agents,
            launcher: &mut *launcher,
        };
        let Live {
            fs: _,
            refresher: _,
            agents: _,
            launcher: _,
        } = live;
    }

    #[test]
    fn the_agent_poller_is_drained_exactly_once_per_frame() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = dashboard();
        let mut events = Script::new(vec![
            Ok(None),
            Ok(None),
            Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
        ]);
        let mut fs = ScriptedFs::new(Vec::new(), Vec::new());
        let mut refresher = RecordingRefresher::new(Vec::new());
        let mut agents = crate::testutil::ScriptedAgents::new(vec![None, None, None], Vec::new());
        let mut launcher = crate::launch::none();
        let mut live = crate::ui::driver::Live {
            fs: &mut fs,
            refresher: &mut refresher,
            agents: &mut agents,
            launcher: &mut *launcher,
        };
        let summary = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
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
        assert_eq!(
            agents.drains().len(),
            3,
            "exactly one drain call per iteration, three iterations"
        );
    }

    #[test]
    fn an_adopted_change_set_does_not_clear_the_agent_snapshot() {
        let mut dashboard = dashboard_with_change("/r", "alpha", 4, 9);
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut events = Script::new(vec![Ok(Some(press(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
        )))]);
        let mut fs = ScriptedFs::new(Vec::new(), Vec::new());
        let merged = crate::changes::fixture::set(
            vec![crate::changes::fixture::active("alpha", 7, 9)],
            Vec::new(),
            Vec::new(),
        );
        let mut refresher =
            RecordingRefresher::new(vec![Some(crate::refresh::RefreshResult::Merged(merged))]);
        let agent = crate::agents::Agent {
            name: None,
            kind: Some("claude".to_string()),
            status: crate::agents::AgentStatus::Idle,
            cwd: None,
            pane_id: "w8:p1".to_string(),
            tab_id: "w8:t1".to_string(),
            workspace_id: "w8".to_string(),
            terminal_title: None,
        };
        let mut agents = crate::testutil::ScriptedAgents::new(
            vec![Some(crate::agents::AgentSnapshot {
                agents: vec![agent],
                reachable: true,
                stalled: false,
                problem: None,
            })],
            Vec::new(),
        );
        let mut launcher = crate::launch::none();
        let mut live = crate::ui::driver::Live {
            fs: &mut fs,
            refresher: &mut refresher,
            agents: &mut agents,
            launcher: &mut *launcher,
        };
        run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("loop ends");

        assert_eq!(
            dashboard.changes.active[0].progress.completed, 7,
            "the refresh result was adopted"
        );
        assert!(
            dashboard.agents.reachable,
            "adopting a refresh result must not clear the agent snapshot taken the same iteration"
        );
        assert_eq!(dashboard.agents.agents.len(), 1);
    }

    #[test]
    fn a_second_snapshot_replaces_the_first_wholesale() {
        let mut dashboard = dashboard();
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut events = Script::new(vec![
            Ok(None),
            Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
        ]);
        let mut fs = ScriptedFs::new(Vec::new(), Vec::new());
        let mut refresher = RecordingRefresher::new(Vec::new());
        let first_agent = crate::agents::Agent {
            name: None,
            kind: Some("claude".to_string()),
            status: crate::agents::AgentStatus::Working,
            cwd: None,
            pane_id: "w8:p1".to_string(),
            tab_id: "w8:t1".to_string(),
            workspace_id: "w8".to_string(),
            terminal_title: None,
        };
        let mut agents = crate::testutil::ScriptedAgents::new(
            vec![
                Some(crate::agents::AgentSnapshot {
                    agents: vec![first_agent],
                    reachable: true,
                    stalled: false,
                    problem: None,
                }),
                Some(crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: true,
                    stalled: false,
                    problem: None,
                }),
            ],
            Vec::new(),
        );
        let mut launcher = crate::launch::none();
        let mut live = crate::ui::driver::Live {
            fs: &mut fs,
            refresher: &mut refresher,
            agents: &mut agents,
            launcher: &mut *launcher,
        };
        run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("loop ends");

        assert!(
            dashboard.agents.agents.is_empty(),
            "a poll that found no agents means there are no agents — replaced wholesale, \
             never merged with the first snapshot's one agent"
        );
        assert!(dashboard.agents.reachable);
    }

    // --- agent-launch: the loop's dispatch and drain --------------------------------------

    #[test]
    fn a_pending_launch_request_is_handed_over_exactly_once() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = dashboard();
        dashboard.agents.reachable = true;
        dashboard.changes = crate::changes::fixture::set(
            vec![crate::changes::fixture::active("add-auth", 1, 2)],
            Vec::new(),
            Vec::new(),
        );
        dashboard.launch.pending = Some(crate::launch::Request::Launch {
            change: "add-auth".to_string(),
            agent: "add-auth".to_string(),
            intent: crate::launch::Intent::Apply,
        });
        let mut events = Script::new(vec![
            Ok(None),
            Ok(None),
            Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
        ]);
        let mut fs = crate::watch::none();
        let mut refresher = crate::refresh::none();
        let mut agents = crate::agents::none();
        let mut launcher = crate::testutil::RecordingLauncher::new();
        let mut live = crate::ui::driver::Live {
            fs: &mut *fs,
            refresher: &mut *refresher,
            agents: &mut *agents,
            launcher: &mut launcher,
        };
        run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("loop ends");

        assert_eq!(
            launcher.requests(),
            vec![crate::launch::Request::Launch {
                change: "add-auth".to_string(),
                agent: "add-auth".to_string(),
                intent: crate::launch::Intent::Apply,
            }],
            "exactly one request, handed over on the first iteration"
        );
        assert_eq!(
            dashboard.launch.pending, None,
            "a request cannot be handed over twice"
        );
    }

    #[test]
    fn a_launch_outcome_updates_the_mapping_and_replaces_the_problem() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = dashboard();
        let mut events = Script::new(vec![
            Ok(None),
            Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
        ]);
        let mut fs = crate::watch::none();
        let mut refresher = crate::refresh::none();
        let mut agents = crate::agents::none();
        let mut launcher = crate::testutil::ScriptedLauncher::new(vec![
            Some(crate::launch::Outcome {
                named: None,
                problems: vec!["split failed".to_string()],
            }),
            Some(crate::launch::Outcome {
                named: Some(("c-2fa-support".to_string(), "2fa-support".to_string())),
                problems: Vec::new(),
            }),
        ]);
        let mut live = crate::ui::driver::Live {
            fs: &mut *fs,
            refresher: &mut *refresher,
            agents: &mut *agents,
            launcher: &mut launcher,
        };
        run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("loop ends");

        assert_eq!(
            dashboard.launch.problems,
            Vec::<String>::new(),
            "the second, successful outcome must replace the first's problem wholesale"
        );
        assert_eq!(
            dashboard.agent_names.names.get("c-2fa-support"),
            Some(&"2fa-support".to_string())
        );
        assert_eq!(dashboard.changes, empty_set());
        assert_eq!(
            dashboard.agents,
            crate::agents::AgentSnapshot {
                agents: Vec::new(),
                reachable: false,
                stalled: false,
                problem: None,
            }
        );
        assert_eq!(
            dashboard.refresh,
            crate::ui::app::Refresh {
                requested: false,
                reload: false,
                startup: Vec::new(),
                problems: Vec::new(),
            }
        );
    }

    /// `degraded-states`' task 6.2, view half: a launch outcome carrying **two** problems
    /// (`degraded-states`' repair of row 23) is replaced wholesale by the next outcome's
    /// success, on exactly `a_launch_outcome_updates_the_mapping_and_replaces_the_problem`'s
    /// terms, at both mandated widths — so the list's first interior row is a change row
    /// again rather than a leftover problem row.
    #[test]
    fn a_success_clears_both_entries() {
        for width in [120u16, 60u16] {
            let backend = TestBackend::new(width, 20);
            let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
            let mut dashboard = dashboard_with_change("/tmp/demo-repo", "2fa-support", 4, 9);
            let mut events = Script::new(vec![
                Ok(None),
                Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
            ]);
            let mut fs = crate::watch::none();
            let mut refresher = crate::refresh::none();
            let mut agents = crate::agents::none();
            let mut launcher = crate::testutil::ScriptedLauncher::new(vec![
                Some(crate::launch::Outcome {
                    named: Some(("c-2fa-support".to_string(), "2fa-support".to_string())),
                    problems: vec![
                        "/state/dir: Not a directory (os error 20)".to_string(),
                        "herdr agent prompt exited with code 1: agent is blocked".to_string(),
                    ],
                }),
                Some(crate::launch::Outcome {
                    named: Some(("c-2fa-support".to_string(), "2fa-support".to_string())),
                    problems: Vec::new(),
                }),
            ]);
            let mut live = crate::ui::driver::Live {
                fs: &mut *fs,
                refresher: &mut *refresher,
                agents: &mut *agents,
                launcher: &mut launcher,
            };
            run_loop(
                &mut terminal,
                &mut dashboard,
                &mut events,
                &mut live,
                &|_: &std::path::Path| Ok(String::new()),
                Duration::from_millis(1),
            )
            .expect("loop ends");

            assert_eq!(
                dashboard.launch.problems,
                Vec::<String>::new(),
                "width {width}: the second, successful outcome must replace both entries wholesale"
            );
            let buf = terminal.backend().buffer();
            assert!(
                row_text(buf, 2).contains("2fa-support"),
                "width {width}: the list's first interior row must be the change row again: {:?}",
                row_text(buf, 2)
            );
        }
    }

    #[test]
    fn a_quit_on_the_same_event_as_a_launch_dispatches_nothing() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut first_dashboard = dashboard();
        first_dashboard.agents.reachable = true;
        first_dashboard.changes = crate::changes::fixture::set(
            vec![crate::changes::fixture::active("add-auth", 1, 2)],
            Vec::new(),
            Vec::new(),
        );
        let mut events = Script::new(vec![
            Ok(Some(press(KeyCode::Char('a'), KeyModifiers::NONE))),
            Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
        ]);
        let mut fs = crate::watch::none();
        let mut refresher = crate::refresh::none();
        let mut agents = crate::agents::none();
        let mut launcher = crate::testutil::RecordingLauncher::new();
        let mut live = crate::ui::driver::Live {
            fs: &mut *fs,
            refresher: &mut *refresher,
            agents: &mut *agents,
            launcher: &mut launcher,
        };
        run_loop(
            &mut terminal,
            &mut first_dashboard,
            &mut events,
            &mut live,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("loop ends");

        assert_eq!(
            launcher.requests().len(),
            1,
            "the request dispatched on the iteration between the two presses"
        );

        // Driving the same script with the `q` press first leaves the launcher with zero
        // requests, because the loop broke before the next iteration's step 1.
        let mut dashboard2 = dashboard();
        let mut events2 = Script::new(vec![Ok(Some(press(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
        )))]);
        let mut fs2 = crate::watch::none();
        let mut refresher2 = crate::refresh::none();
        let mut agents2 = crate::agents::none();
        let mut launcher2 = crate::testutil::RecordingLauncher::new();
        let backend2 = TestBackend::new(60, 20);
        let mut terminal2 = ratatui::Terminal::new(backend2).expect("construct terminal");
        let mut live2 = crate::ui::driver::Live {
            fs: &mut *fs2,
            refresher: &mut *refresher2,
            agents: &mut *agents2,
            launcher: &mut launcher2,
        };
        run_loop(
            &mut terminal2,
            &mut dashboard2,
            &mut events2,
            &mut live2,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("loop ends");
        assert!(launcher2.requests().is_empty());
    }

    #[test]
    fn a_launch_failure_is_recorded_once_and_the_loop_continues() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = dashboard();
        let mut events = Script::new(vec![
            Ok(None),
            Ok(None),
            Ok(None),
            Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
        ]);
        let mut fs = crate::watch::none();
        let mut refresher = crate::refresh::none();
        let mut agents = crate::agents::none();
        let failure = || {
            Some(crate::launch::Outcome {
                named: None,
                problems: vec!["herdr pane split exited with code 1: no space".to_string()],
            })
        };
        let mut launcher =
            crate::testutil::ScriptedLauncher::new(vec![failure(), failure(), failure(), None]);
        let mut live = crate::ui::driver::Live {
            fs: &mut *fs,
            refresher: &mut *refresher,
            agents: &mut *agents,
            launcher: &mut launcher,
        };
        let summary = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("loop ends");

        assert_eq!(
            summary,
            LoopSummary {
                frames: 4,
                polls: 4
            },
            "a failed launch is never a LoopError and never ends the loop"
        );
        assert_eq!(
            dashboard.launch.problems,
            vec!["herdr pane split exited with code 1: no space".to_string()],
            "replaced wholesale, not three entries"
        );
        // Rendered as the list region's first !-marked row is `ui::list::tests::
        // a_launch_problem_is_the_lists_first_row`'s own claim (group 9), which is what
        // actually renders `launch.problems`; this test's job is the loop's own bookkeeping.
    }

    // --- seam-resilience: in-flight lifecycle, startup problems, the empty-selection
    // --- short-circuit, Stopped, the stall row, and the artifact-read cache ----------------

    #[test]
    fn the_flag_is_set_on_hand_over_and_cleared_on_outcome() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = dashboard();
        dashboard.launch.pending = Some(crate::launch::Request::Launch {
            change: "add-auth".to_string(),
            agent: "add-auth".to_string(),
            intent: crate::launch::Intent::Apply,
        });

        // Stage 1: hand-over, no answer yet — the flag must already be set.
        let mut events1 = Script::new(vec![Ok(Some(press(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
        )))]);
        let mut fs = crate::watch::none();
        let mut refresher = crate::refresh::none();
        let mut agents = crate::agents::none();
        let mut launcher1 = crate::testutil::ScriptedLauncher::new(vec![None]);
        let mut live1 = crate::ui::driver::Live {
            fs: &mut *fs,
            refresher: &mut *refresher,
            agents: &mut *agents,
            launcher: &mut launcher1,
        };
        run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events1,
            &mut live1,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("stage 1 ends");
        assert!(dashboard.launch.in_flight, "set on hand-over");

        // Stage 2: the same dashboard, driven again — the launcher now answers.
        dashboard.quit = false;
        let mut events2 = Script::new(vec![Ok(Some(press(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
        )))]);
        let mut launcher2 =
            crate::testutil::ScriptedLauncher::new(vec![Some(crate::launch::Outcome {
                named: Some(("add-auth".to_string(), "add-auth".to_string())),
                problems: Vec::new(),
            })]);
        let mut live2 = crate::ui::driver::Live {
            fs: &mut *fs,
            refresher: &mut *refresher,
            agents: &mut *agents,
            launcher: &mut launcher2,
        };
        run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events2,
            &mut live2,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("stage 2 ends");
        assert!(!dashboard.launch.in_flight, "cleared on outcome");
        assert_eq!(
            dashboard.agent_names.names.get("add-auth"),
            Some(&"add-auth".to_string())
        );
    }

    #[test]
    fn a_focus_request_never_sets_the_flag() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = dashboard();
        dashboard.launch.pending = Some(crate::launch::Request::Focus {
            pane_id: "w8:p1".to_string(),
        });
        let mut events = Script::new(vec![Ok(Some(press(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
        )))]);
        let mut fs = crate::watch::none();
        let mut refresher = crate::refresh::none();
        let mut agents = crate::agents::none();
        let mut launcher = crate::testutil::RecordingLauncher::new();
        let mut live = crate::ui::driver::Live {
            fs: &mut *fs,
            refresher: &mut *refresher,
            agents: &mut *agents,
            launcher: &mut launcher,
        };
        run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("loop ends");
        assert!(!dashboard.launch.in_flight);
    }

    #[test]
    fn a_dead_launcher_clears_the_flag() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = dashboard();
        dashboard.launch.in_flight = true;
        let mut events = Script::new(vec![Ok(Some(press(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
        )))]);
        let mut fs = crate::watch::none();
        let mut refresher = crate::refresh::none();
        let mut agents = crate::agents::none();
        let mut launcher =
            crate::testutil::ScriptedLauncher::new(vec![Some(crate::launch::Outcome {
                named: None,
                problems: vec!["the launcher's worker has stopped answering".to_string()],
            })]);
        let mut live = crate::ui::driver::Live {
            fs: &mut *fs,
            refresher: &mut *refresher,
            agents: &mut *agents,
            launcher: &mut launcher,
        };
        run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("loop ends");
        assert!(!dashboard.launch.in_flight);
        assert_eq!(
            dashboard.launch.problems,
            vec!["the launcher's worker has stopped answering".to_string()]
        );
    }

    #[test]
    fn a_second_launch_key_while_in_flight_renders_a_problem_row_and_issues_nothing() {
        for width in [120u16, 60u16] {
            let backend = TestBackend::new(width, 20);
            let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
            let mut dashboard = dashboard();
            dashboard.agents.reachable = true;
            dashboard.launch.in_flight = true;
            dashboard.changes = crate::changes::fixture::set(
                vec![crate::changes::fixture::active("add-auth", 1, 2)],
                Vec::new(),
                Vec::new(),
            );
            let mut events = Script::new(vec![
                Ok(Some(press(KeyCode::Char('a'), KeyModifiers::NONE))),
                Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
            ]);
            let mut fs = crate::watch::none();
            let mut refresher = crate::refresh::none();
            let mut agents = crate::agents::none();
            let mut launcher = crate::testutil::RecordingLauncher::new();
            let mut live = crate::ui::driver::Live {
                fs: &mut *fs,
                refresher: &mut *refresher,
                agents: &mut *agents,
                launcher: &mut launcher,
            };
            run_loop(
                &mut terminal,
                &mut dashboard,
                &mut events,
                &mut live,
                &|_: &std::path::Path| Ok(String::new()),
                Duration::from_millis(1),
            )
            .expect("loop ends");

            assert!(launcher.requests().is_empty(), "width {width}");
            let buf = terminal.backend().buffer();
            assert!(
                row_text(buf, 2).contains("! "),
                "width {width}: {}",
                row_text(buf, 2)
            );
            assert!(
                row_text(buf, 2).contains("already running"),
                "width {width}"
            );
        }
    }

    #[test]
    fn a_stalled_poller_becomes_a_problem_row_and_withdraws_the_badges() {
        for width in [120u16, 60u16] {
            let backend = TestBackend::new(width, 20);
            let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
            let mut dashboard = dashboard_with_change("/tmp/demo-repo", "2fa-support", 1, 2);
            let mut events = Script::new(vec![Ok(Some(press(
                KeyCode::Char('q'),
                KeyModifiers::NONE,
            )))]);
            let mut fs = crate::watch::none();
            let mut refresher = crate::refresh::none();
            let mut agents = crate::testutil::ScriptedAgents::new(
                vec![Some(crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: true,
                    problem: Some("herdr agent list has not answered in over 5s".to_string()),
                })],
                Vec::new(),
            );
            let mut launcher = crate::launch::none();
            let mut live = crate::ui::driver::Live {
                fs: &mut *fs,
                refresher: &mut *refresher,
                agents: &mut agents,
                launcher: &mut *launcher,
            };
            run_loop(
                &mut terminal,
                &mut dashboard,
                &mut events,
                &mut live,
                &|_: &std::path::Path| Ok(String::new()),
                Duration::from_millis(1),
            )
            .expect("loop ends");

            let buf = terminal.backend().buffer();
            assert!(
                row_text(buf, 2).contains("! "),
                "width {width}: {}",
                row_text(buf, 2)
            );
            assert!(
                row_text(buf, 2).contains("has not answered"),
                "width {width}"
            );
            assert!(dashboard.agents.stalled, "width {width}");
        }
    }

    #[test]
    fn a_stopped_refresh_worker_becomes_a_problem_row_and_keeps_the_list() {
        for width in [120u16, 60u16] {
            let backend = TestBackend::new(width, 20);
            let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
            let mut dashboard = dashboard();
            dashboard.changes = crate::changes::fixture::set(
                vec![
                    crate::changes::fixture::active("alpha", 1, 2),
                    crate::changes::fixture::active("beta", 1, 2),
                ],
                Vec::new(),
                Vec::new(),
            );
            let mut events = Script::new(vec![
                Ok(None),
                Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
            ]);
            let mut fs = crate::watch::none();
            let mut refresher = RecordingRefresher::new(vec![Some(
                crate::refresh::RefreshResult::Stopped("refresh worker stopped".to_string()),
            )]);
            let mut agents = crate::agents::none();
            let mut launcher = crate::launch::none();
            let mut live = crate::ui::driver::Live {
                fs: &mut *fs,
                refresher: &mut refresher,
                agents: &mut *agents,
                launcher: &mut *launcher,
            };
            run_loop(
                &mut terminal,
                &mut dashboard,
                &mut events,
                &mut live,
                &|_: &std::path::Path| Ok(String::new()),
                Duration::from_millis(1),
            )
            .expect("loop ends");

            assert_eq!(
                dashboard.refresh.problems,
                vec!["refresh worker stopped".to_string()],
                "width {width}"
            );
            assert_eq!(
                dashboard.changes.active.len(),
                2,
                "width {width}: no adopt ran, so a dead worker does not empty the list"
            );
            let buf = terminal.backend().buffer();
            assert!(row_text(buf, 2).contains("! "), "width {width}");
        }
    }

    #[test]
    fn a_watcher_error_does_not_erase_the_startup_problems() {
        for width in [120u16, 60u16] {
            let backend = TestBackend::new(width, 20);
            let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
            let mut dashboard = dashboard();
            dashboard.refresh.startup = vec![
                "openspec binary not found on PATH".to_string(),
                "config.toml: archived_count not set, using 5".to_string(),
            ];
            let mut events = Script::new(vec![
                Ok(None),
                Ok(None),
                Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
            ]);
            let mut fs = ScriptedFs::new(
                vec![
                    Err(crate::watch::WatchError("watch failed".to_string())),
                    Ok(None),
                ],
                Vec::new(),
            );
            let mut refresher = RecordingRefresher::new(Vec::new());
            let mut agents = crate::agents::none();
            let mut launcher = crate::launch::none();
            let mut live = crate::ui::driver::Live {
                fs: &mut fs,
                refresher: &mut refresher,
                agents: &mut *agents,
                launcher: &mut *launcher,
            };
            run_loop(
                &mut terminal,
                &mut dashboard,
                &mut events,
                &mut live,
                &|_: &std::path::Path| Ok(String::new()),
                Duration::from_millis(1),
            )
            .expect("loop ends");

            assert_eq!(
                dashboard.refresh.startup,
                vec![
                    "openspec binary not found on PATH".to_string(),
                    "config.toml: archived_count not set, using 5".to_string(),
                ],
                "width {width}"
            );
            assert_eq!(
                dashboard.refresh.problems,
                vec!["watch failed".to_string()],
                "width {width}"
            );
            let buf = terminal.backend().buffer();
            assert!(
                row_text(buf, 2).contains("openspec binary not found"),
                "width {width}: {}",
                row_text(buf, 2)
            );
            assert!(
                row_text(buf, 3).contains("archived_count"),
                "width {width}: {}",
                row_text(buf, 3)
            );
            assert!(
                row_text(buf, 4).contains("watch failed"),
                "width {width}: {}",
                row_text(buf, 4)
            );
        }
    }

    #[test]
    fn a_forced_refresh_does_not_repopulate_or_duplicate_the_startup_problems() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = dashboard();
        dashboard.refresh.startup = vec!["openspec binary not found".to_string()];
        dashboard.refresh.requested = true;

        let merged = crate::changes::fixture::set(Vec::new(), Vec::new(), Vec::new());
        let mut events1 = Script::new(vec![Ok(Some(press(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
        )))]);
        let mut fs1 = ScriptedFs::new(Vec::new(), Vec::new());
        let mut refresher1 =
            RecordingRefresher::new(vec![Some(crate::refresh::RefreshResult::Merged(merged))]);
        let mut agents1 = crate::agents::none();
        let mut launcher1 = crate::launch::none();
        let mut live1 = crate::ui::driver::Live {
            fs: &mut fs1,
            refresher: &mut refresher1,
            agents: &mut *agents1,
            launcher: &mut *launcher1,
        };
        run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events1,
            &mut live1,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("stage 1 ends");

        dashboard.quit = false;
        let mut events2 = Script::new(vec![Ok(Some(press(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
        )))]);
        let mut fs2 = ScriptedFs::new(
            vec![Err(crate::watch::WatchError(
                "watch failed again".to_string(),
            ))],
            Vec::new(),
        );
        let mut refresher2 = RecordingRefresher::new(Vec::new());
        let mut agents2 = crate::agents::none();
        let mut launcher2 = crate::launch::none();
        let mut live2 = crate::ui::driver::Live {
            fs: &mut fs2,
            refresher: &mut refresher2,
            agents: &mut *agents2,
            launcher: &mut *launcher2,
        };
        run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events2,
            &mut live2,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("stage 2 ends");

        assert_eq!(
            dashboard.refresh.startup,
            vec!["openspec binary not found".to_string()]
        );
        assert_eq!(
            dashboard.refresh.problems,
            vec!["watch failed again".to_string()]
        );
    }

    #[test]
    fn a_batch_that_invalidates_nothing_issues_no_refresh() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = dashboard();
        dashboard.repo = Some(std::path::PathBuf::from("/r"));
        let mut events = Script::new(vec![
            Ok(None),
            Ok(None),
            Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
        ]);
        let mut fs = ScriptedFs::new(
            vec![Ok(Some(vec![
                std::path::PathBuf::from("/r/target/debug/build.log"),
                std::path::PathBuf::from("/r/.git/index"),
            ]))],
            Vec::new(),
        );
        let mut refresher = RecordingRefresher::new(Vec::new());
        let mut agents = crate::agents::none();
        let mut launcher = crate::launch::none();
        let mut live = crate::ui::driver::Live {
            fs: &mut fs,
            refresher: &mut refresher,
            agents: &mut *agents,
            launcher: &mut *launcher,
        };
        run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("loop ends");

        assert!(
            refresher.requests().is_empty(),
            "an all-Outside batch invalidates nothing"
        );
        assert!(dashboard.refresh.problems.is_empty());
    }

    #[test]
    fn a_held_key_causes_no_read() {
        for width in [120u16, 60u16] {
            let backend = TestBackend::new(width, 20);
            let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
            let mut dashboard = dashboard_with_one_artifact_path();
            dashboard.route = Route::Detail;
            let mut presses: Vec<_> = (0..10)
                .map(|_| Ok(Some(press(KeyCode::Char('j'), KeyModifiers::NONE))))
                .collect();
            presses.push(Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))));
            let mut events = Script::new(presses);
            let recorder = crate::testutil::RecordingReader::always(Ok("# proposal\n".to_string()));
            let read = |p: &std::path::Path| recorder.read(p);

            let mut fs = crate::watch::none();
            let mut refresher = crate::refresh::none();
            let mut agents = crate::agents::none();
            let mut launcher = crate::launch::none();
            let mut live = crate::ui::driver::Live {
                fs: &mut *fs,
                refresher: &mut *refresher,
                agents: &mut *agents,
                launcher: &mut *launcher,
            };
            run_loop(
                &mut terminal,
                &mut dashboard,
                &mut events,
                &mut live,
                &read,
                Duration::from_millis(1),
            )
            .expect("loop ends");

            assert_eq!(recorder.calls(), 1, "width {width}");
        }
    }

    #[test]
    fn a_tab_switch_and_an_adopted_refresh_each_cause_exactly_one_read() {
        let backend = TestBackend::new(120, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let change = crate::changes::fixture::with_artifacts(
            crate::changes::fixture::active("detail-view", 4, 9),
            &[("proposal", &["/repo/p.md"]), ("design", &["/repo/d.md"])],
        );
        let mut dashboard = Dashboard {
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
                problems: Vec::new(),
                in_flight: false,
            },
            file_mode: false,
        };
        let merged = crate::changes::fixture::set(
            vec![crate::changes::fixture::with_artifacts(
                crate::changes::fixture::active("detail-view", 7, 9),
                &[("proposal", &["/repo/p.md"]), ("design", &["/repo/d.md"])],
            )],
            Vec::new(),
            Vec::new(),
        );
        let mut events = Script::new(vec![
            Ok(Some(press(KeyCode::Char(']'), KeyModifiers::NONE))),
            Ok(None),
            Ok(None),
            Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
        ]);
        let mut fs = ScriptedFs::new(Vec::new(), Vec::new());
        // The tab switch's own read (iteration 2) and the adopted `Merged`'s forced reload
        // (iteration 3) are kept on separate iterations, so a single read cannot satisfy
        // both: `None` on the first two `take_result` calls, `Merged` only on the third.
        let mut refresher = RecordingRefresher::new(vec![
            None,
            None,
            Some(crate::refresh::RefreshResult::Merged(merged)),
        ]);
        let mut agents = crate::agents::none();
        let mut launcher = crate::launch::none();
        let mut live = crate::ui::driver::Live {
            fs: &mut fs,
            refresher: &mut refresher,
            agents: &mut *agents,
            launcher: &mut *launcher,
        };
        let recorder = crate::testutil::RecordingReader::always(Ok("content".to_string()));
        let read = |p: &std::path::Path| recorder.read(p);
        run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
            &read,
            Duration::from_millis(1),
        )
        .expect("loop ends");

        assert_eq!(
            recorder.calls(),
            3,
            "one for the first frame, one for the new tab, one for the adopted reload"
        );
    }

    #[test]
    fn live_cannot_be_built_without_naming_the_launcher() {
        let mut fs = crate::watch::none();
        let mut refresher = crate::refresh::none();
        let mut agents = crate::agents::none();
        let mut launcher = crate::launch::none();
        let Live {
            fs: _,
            refresher: _,
            agents: _,
            launcher: _,
        } = crate::ui::driver::Live {
            fs: &mut *fs,
            refresher: &mut *refresher,
            agents: &mut *agents,
            launcher: &mut *launcher,
        };
    }
}
