//! The event loop: draw first, then wait. See
//! `openspec/changes/tui-shell/specs/dashboard-loop/spec.md`.

use std::time::Duration;

use ratatui::Terminal;
use ratatui::backend::Backend;

use ratatui::crossterm::event::{Event, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::ui::app::{Action, ArtifactReader, Dashboard, Target, action_for};
use crate::ui::event::{EventError, EventSource};
use crate::ui::layout::Zone;
use crate::ui::list::RowKind;
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
    // The area of the frame most recently drawn — the geometry a mouse event is
    // resolved against, and the value carried forward across a skipped draw.
    // The first iteration always draws, so this initial value is never the one
    // any event is resolved against.
    let mut area = Rect::ZERO;
    // `mouse-input` -> design.md -> Decision 12: cleared for exactly one
    // iteration after a pointer-motion event, so moving a pointer across the
    // pane costs no frames at all.
    let mut draw = true;
    loop {
        drive_live_tier(dashboard, live);

        if draw {
            dashboard.sync_detail(read);
            let completed = terminal
                .draw(|frame| view::render(frame, dashboard))
                .map_err(|e| LoopError::Draw(e.to_string()))?;
            // `CompletedFrame` borrows `terminal`; `area` is `Copy`, so it is
            // copied out here and the borrow ends before `normalise_scroll`
            // takes `dashboard` mutably.
            area = completed.area;
            frames += 1;
            dashboard.normalise_scroll(area);
        }
        draw = true;

        let timeout = crate::watch::poll_timeout(
            tick,
            crate::watch::soonest(live.fs.pending_in(), live.agents.pending_in()),
        );
        let event = events.next_event(timeout).map_err(LoopError::Events)?;
        polls += 1;

        if let Some(event) = event {
            // One action per event, and the quit check is unchanged: a mouse
            // event can neither apply two actions nor bypass it.
            let action = match &event {
                // The mouse is resolved against the frame just drawn — never a
                // stored size and never the size at startup. A resize between
                // the draw and the click costs at most one mis-targeted event,
                // which the next frame corrects: the same one-frame window
                // `normalise_scroll` already accepts.
                Event::Mouse(mouse) => {
                    // No filter flag: a click is unambiguous where a keystroke
                    // is not, so the rule is structural rather than a branch.
                    if matches!(mouse.kind, MouseEventKind::Moved | MouseEventKind::Drag(_)) {
                        draw = false;
                    }
                    // One consequence the skipped draw carries, named here because
                    // it is the exemption's own cost: `drive_live_tier` still runs
                    // on a skipped-draw iteration, so an adopted `ChangeSet` can
                    // change `targets()` while the frame on screen predates that
                    // adopt — and a click read at the end of that same iteration
                    // resolves against rows the reader is not looking at. Without
                    // the exemption the window does not exist, because an adopt is
                    // always followed by a draw before the next event is read.
                    // Bounded by the rule design.md -> Risks already states:
                    // `Action::Click` names a `Target`, never a row index, and
                    // `apply` does nothing when that target is absent from
                    // `targets()`. A target that still exists and now names a
                    // different change does move the cursor — accepted, at the
                    // width of one adopt landing between a pointer motion and a
                    // click, and corrected by the very next frame.
                    mouse_action(dashboard, area, mouse)
                }
                _ => action_for(&event, dashboard.filter.active),
            };
            dashboard.apply(action);
            if dashboard.quit {
                break;
            }
        }
    }
    Ok(LoopSummary { frames, polls })
}

/// Map a mouse event to one of the actions `Action` already carries, using
/// `area` — the area of the frame `run_loop` has just drawn — as the geometry
/// the pointer is resolved against.
///
/// Pure and total: it performs no I/O, reads no clock, mutates nothing, and
/// returns an `Action` for every `MouseEvent` value, every `Rect` including a
/// zero-sized one, and every `Dashboard` value, without panicking.
///
/// It takes **no filter flag** and resolves identically whether or not
/// `dashboard.filter.active` is set: a printable key is ambiguous while
/// filtering and is resolved as a character, but a click is not ambiguous
/// (design.md -> Decision 10). It reads `MouseEvent::modifiers` nowhere either,
/// so a `Shift`-click resolves exactly as a bare one — the drag-select override
/// is the *terminal's*, applied before any sequence is sent (design.md ->
/// Decision 13).
///
/// The wheel names the region under the pointer — the whole region, its border
/// and the detail region's header and tab bar included — and the click names
/// the row. Problem and message rows are refused here rather than in the row
/// grammar, so `change-rows` gains no notion of clickability (design.md ->
/// Decision 11). No gesture reaches a launch action: a mis-click must not start
/// or focus an agent.
///
/// Lives here rather than in `ui::app` because `run_loop` already copies the
/// drawn frame's `area` out of the `CompletedFrame` for `normalise_scroll`, so
/// the resolver's one input that nothing else has is already in hand
/// (design.md -> Decision 8).
pub fn mouse_action(dashboard: &Dashboard, area: Rect, mouse: &MouseEvent) -> Action {
    let zone = crate::ui::layout::zone(area, dashboard.route, mouse.column, mouse.row);
    match mouse.kind {
        MouseEventKind::ScrollDown => match zone {
            Zone::List | Zone::ListRow { .. } => Action::SelectNext,
            Zone::Detail | Zone::DetailTab { .. } => Action::ScrollDown,
            Zone::Outside => Action::Ignore,
        },
        MouseEventKind::ScrollUp => match zone {
            Zone::List | Zone::ListRow { .. } => Action::SelectPrev,
            Zone::Detail | Zone::DetailTab { .. } => Action::ScrollUp,
            Zone::Outside => Action::Ignore,
        },
        MouseEventKind::Down(MouseButton::Left) => match zone {
            Zone::ListRow { interior, row } => {
                match crate::ui::list::row_at(dashboard, interior, row) {
                    Some(RowKind::Item { index }) => Action::Click(Target::Change(index)),
                    Some(RowKind::Section { key, .. }) => Action::Click(Target::Section(key)),
                    Some(RowKind::Problem) | Some(RowKind::Message) | None => Action::Ignore,
                }
            }
            Zone::DetailTab { bar, column } => dashboard
                .selected_change()
                .and_then(|change| {
                    crate::ui::detail::tab_at(
                        &change.artifacts,
                        dashboard.detail.tab,
                        bar.width,
                        column,
                    )
                })
                .map_or(Action::Ignore, Action::SelectTab),
            Zone::List | Zone::Detail | Zone::Outside => Action::Ignore,
        },
        // Right and middle presses, every release, every drag, pointer motion,
        // and both horizontal wheel directions: there is no context menu, no
        // drag of any kind, and the pane scrolls in one dimension only.
        _ => Action::Ignore,
    }
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
        // `list-sections` group 6: the scope is `dashboard.archived_scope()` — `Full` when
        // the archived section is open, `Names` when it is collapsed — so the startup
        // request (and every `r`-triggered one) resolves exactly as much of the archive as
        // the reader can currently see.
        live.refresher
            .request(crate::changes::Selection::All, dashboard.archived_scope());
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
                    // `list-sections` group 6: a watch event landing while the archived
                    // section is open must resolve that section too, or the next file
                    // change would silently empty an archive the reader had just opened
                    // (dashboard-loop's own rule for this call site).
                    live.refresher
                        .request(selection, dashboard.archived_scope());
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
            // `list-sections`: 1, not 0 — target 0 is the active header.
            selected: 1,
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
            sections: crate::ui::app::Sections {
                collapsed: std::collections::BTreeSet::new(),
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
            sections: crate::ui::app::Sections {
                collapsed: std::collections::BTreeSet::new(),
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
            // `list-sections`: 1, not 0 — target 0 is the active header.
            selected: 1,
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
            sections: crate::ui::app::Sections {
                collapsed: std::collections::BTreeSet::new(),
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
            // `list-sections`: 1, not 0 — target 0 is the active header.
            selected: 1,
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
            sections: crate::ui::app::Sections {
                collapsed: std::collections::BTreeSet::new(),
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
            sections: crate::ui::app::Sections {
                collapsed: std::collections::BTreeSet::new(),
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

    /// `detail-scroll`: "Enter at the detail route moves nothing and keeps
    /// the scroll" — the loop-tier row of the scenario `ui::app`'s
    /// `enter_at_the_detail_route_is_a_noop` proves at the `apply` level.
    /// This drives the real `action_for` -> `apply` path through a scripted
    /// `run_loop`, at a scroll value strictly inside `normalise_scroll`'s
    /// own clamp (max 6 at height 20 over the twenty-line source, per
    /// `a_resize_renormalises_the_offset` above), so any difference here is
    /// attributable only to `OpenDetail`'s guard and not to that
    /// pre-existing, unrelated clamp. `detail.loaded` is pre-set to the
    /// selected change's own `(dir, tab)` key so `sync_detail` never
    /// re-reads and never touches the scroll itself — the guard is the only
    /// mechanism this test can be exercising.
    #[test]
    fn enter_at_the_detail_route_moves_nothing_through_run_loop() {
        let backend = TestBackend::new(120, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");

        let change = crate::changes::fixture::with_artifacts(
            crate::changes::fixture::active("add-auth", 1, 2),
            &[("tasks", &["/repo/tasks.md"])],
        );
        let dir = change.dir.clone();
        let mut dashboard = Dashboard {
            repo: Some(std::path::PathBuf::from("/repo")),
            searched_from: std::path::PathBuf::from("/repo"),
            changes: crate::changes::fixture::set(vec![change], Vec::new(), Vec::new()),
            route: Route::Detail,
            quit: false,
            // `list-sections`: 1, not 0 — target 0 is the active header.
            selected: 1,
            filter: crate::ui::app::Filter {
                query: String::new(),
                active: false,
            },
            detail: crate::ui::app::Detail {
                source: (0..20).map(|i| format!("- line-{i:02}\n")).collect(),
                scroll: 3,
                tab: 0,
                problems: Vec::new(),
                loaded: Some((dir, 0)),
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
        let before = dashboard.clone();

        let mut events = Script::new(vec![
            Ok(Some(press(KeyCode::Enter, KeyModifiers::NONE))),
            Ok(Some(press(KeyCode::Enter, KeyModifiers::NONE))),
            Ok(Some(press(KeyCode::Enter, KeyModifiers::NONE))),
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
                frames: 4,
                polls: 4
            }
        );
        assert!(dashboard.quit);
        assert_eq!(
            dashboard.route, before.route,
            "Enter must not move the route away from Detail"
        );
        assert_eq!(dashboard.selected, before.selected);
        assert_eq!(dashboard.filter, before.filter);
        assert_eq!(
            dashboard.detail, before.detail,
            "Enter at the detail route must not reset the scroll or touch any other field"
        );
        assert_eq!(dashboard.changes, before.changes);

        let buf = terminal.backend().buffer();
        let row: String = row_text(buf, 4).chars().skip(41).take(9).collect();
        assert_eq!(row, "- line-03");
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
            // `list-sections`: 1, not 0 — target 0 is the active header.
            selected: 1,
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
            sections: crate::ui::app::Sections {
                collapsed: std::collections::BTreeSet::new(),
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
            // `list-sections`: row 2 is now the active section header; the change
            // row is row 3.
            let buf = terminal.backend().buffer();
            assert!(
                row_text(buf, 3).contains("[7/9]"),
                "width {width}: the result must reach the very frame that consumed it: {}",
                row_text(buf, 3)
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

    /// `dashboard-loop`: **both** `request` call sites — the `refresh.requested`
    /// one and the watch-invalidate one — carry `dashboard.archived_scope()`,
    /// not a constant. Driven twice over the same script, once with the
    /// archived section collapsed and once with it open, so a constant of
    /// either value fails one leg: the two legs disagree on every recorded
    /// scope, which is the property a constant cannot have.
    #[test]
    fn both_request_call_sites_carry_the_dashboards_archived_scope() {
        for (collapsed, expected) in [
            (true, crate::changes::ArchivedScope::Names),
            (false, crate::changes::ArchivedScope::Full),
        ] {
            let backend = TestBackend::new(60, 20);
            let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
            let mut dashboard = dashboard();
            dashboard.repo = Some(std::path::PathBuf::from("/r"));
            dashboard.refresh.requested = true;
            dashboard.sections = crate::ui::app::Sections {
                collapsed: if collapsed {
                    let mut set = std::collections::BTreeSet::new();
                    set.insert(crate::ui::app::SectionKey::Archived);
                    set
                } else {
                    std::collections::BTreeSet::new()
                },
            };
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

            assert_eq!(
                refresher.requests().len(),
                2,
                "collapsed {collapsed}: both call sites must have fired"
            );
            assert_eq!(
                refresher.scopes(),
                vec![expected, expected],
                "collapsed {collapsed}: a constant scope at either call site fails here"
            );
        }
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
            // `list-sections`: row 2 is now the active section header; the change
            // row is row 3.
            let buf = terminal.backend().buffer();
            assert!(
                row_text(buf, 3).contains("2fa-support"),
                "width {width}: the list's first interior row must be the change row again: {:?}",
                row_text(buf, 3)
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
        // `list-sections`: target 0 is now the active header; `add-auth` is
        // target 1.
        first_dashboard.selected = 1;
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
            // `list-sections`: target 0 is now the active header; `add-auth`
            // is target 1.
            dashboard.selected = 1;
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
            // `seam-resilience`: seed a `Working` badge attributed to the visible row
            // *before* the stall drain — the scenario's own words are "whose visible row
            // carries a `working` badge from an earlier snapshot". Without this, the
            // fixture's badge-less starting point makes the assertion below pass
            // vacuously even for an implementation that never withdraws a stale badge.
            dashboard.agents = crate::agents::AgentSnapshot {
                agents: vec![crate::agents::Agent {
                    name: Some("2fa-support".to_string()),
                    kind: None,
                    status: crate::agents::AgentStatus::Working,
                    cwd: Some(std::path::PathBuf::from("/tmp/demo-repo")),
                    pane_id: "w8:p1".to_string(),
                    tab_id: "w8:t1".to_string(),
                    workspace_id: "w8".to_string(),
                    terminal_title: None,
                }],
                reachable: true,
                stalled: false,
                problem: None,
            };
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
            // The badge withdrawal itself: the stall problem row above occupies row 2, so
            // the change's own row — which still carries its progress cell — is row 3. The
            // "w " badge that would otherwise sit just before the progress cell is gone.
            assert!(
                !row_text(buf, 3).contains("w [1/2]"),
                "width {width}: badge should be withdrawn: {}",
                row_text(buf, 3)
            );
            assert!(dashboard.agents.stalled, "width {width}");
            assert!(
                dashboard.agents.agents.is_empty(),
                "width {width}: the stalled snapshot carries no agents"
            );
            assert!(
                dashboard.refresh.problems.is_empty(),
                "width {width}: an agent stall must not touch refresh.problems"
            );
            assert!(
                dashboard.refresh.startup.is_empty(),
                "width {width}: an agent stall must not touch refresh.startup"
            );
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
            // `list-sections`: 1, not 0 — target 0 is the active header.
            selected: 1,
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
            sections: crate::ui::app::Sections {
                collapsed: std::collections::BTreeSet::new(),
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

    // ------------------------------------------------------------------
    // `mouse-input`: the resolver and the loop.
    // ------------------------------------------------------------------

    use ratatui::crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    use ratatui::layout::Rect;

    use crate::ui::app::{Action, SectionKey, Target, action_for};
    use crate::ui::driver::mouse_action;

    /// The two mandated frames. Every geometry below is derived from these
    /// through `layout`'s own splits, never written down twice.
    const WIDE: Rect = Rect {
        x: 0,
        y: 0,
        width: 120,
        height: 40,
    };
    const NARROW: Rect = Rect {
        x: 0,
        y: 0,
        width: 60,
        height: 20,
    };

    /// A bare `MouseEvent`, for the unit calls that need no `Event` wrapper.
    fn m(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn left(column: u16, row: u16) -> MouseEvent {
        m(MouseEventKind::Down(MouseButton::Left), column, row)
    }

    /// A `Route::List` dashboard over `active` active changes and `archived`
    /// archived ones, cursor on the active header.
    fn mouse_dashboard(active: usize, archived: usize) -> Dashboard {
        let a: Vec<_> = (0..active)
            .map(|i| crate::changes::fixture::active(&format!("s{i}"), 0, 1))
            .collect();
        let z: Vec<_> = (0..archived)
            .map(|i| crate::changes::fixture::archived(Some("2026-01-01"), &format!("z{i}"), 1, 1))
            .collect();
        let mut dashboard = dashboard_with_change("/repo", "unused", 0, 0);
        dashboard.changes = crate::changes::fixture::set(a, z, Vec::new());
        dashboard.selected = 0;
        dashboard
    }

    /// The list region's interior for `area` at `route`, derived the way the
    /// draw path derives it.
    fn list_interior(area: Rect, route: Route) -> Rect {
        let (_, body, _) = crate::ui::layout::split_frame(area);
        crate::ui::layout::interior(
            crate::ui::layout::split_body(body, route)
                .0
                .expect("a list region is drawn"),
        )
    }

    /// The detail region's tab-bar row for `area` at `route`.
    fn tab_bar_row(area: Rect, route: Route) -> Rect {
        let (_, body, _) = crate::ui::layout::split_frame(area);
        crate::ui::layout::split_detail(crate::ui::layout::interior(
            crate::ui::layout::split_body(body, route)
                .1
                .expect("a detail region is drawn"),
        ))
        .1
    }

    /// The terminal row the list interior's `offset`-th drawn row occupies.
    fn list_row(area: Rect, route: Route, offset: u16) -> u16 {
        list_interior(area, route).y + offset
    }

    #[test]
    fn mouse_action_is_total() {
        // `mouse-input`: "Resolution is total over adversarial geometry".
        let kinds = [
            MouseEventKind::Down(MouseButton::Left),
            MouseEventKind::Down(MouseButton::Right),
            MouseEventKind::Down(MouseButton::Middle),
            MouseEventKind::Up(MouseButton::Left),
            MouseEventKind::Up(MouseButton::Right),
            MouseEventKind::Up(MouseButton::Middle),
            MouseEventKind::Drag(MouseButton::Left),
            MouseEventKind::Drag(MouseButton::Right),
            MouseEventKind::Drag(MouseButton::Middle),
            MouseEventKind::Moved,
            MouseEventKind::ScrollDown,
            MouseEventKind::ScrollUp,
            MouseEventKind::ScrollLeft,
            MouseEventKind::ScrollRight,
        ];
        let coords = [0u16, 1, 39, 40, 59, 119, 65535];
        let areas = [
            Rect::new(0, 0, 0, 0),
            Rect::new(0, 0, 1, 1),
            Rect::new(0, 0, 2, 2),
            NARROW,
            WIDE,
        ];
        let mut no_repo = mouse_dashboard(0, 0);
        no_repo.repo = None;
        let dashboards = [no_repo, mouse_dashboard(0, 0), mouse_dashboard(3, 3)];
        let modifiers = [
            KeyModifiers::NONE,
            KeyModifiers::SHIFT,
            KeyModifiers::CONTROL,
            KeyModifiers::ALT,
        ];

        // The route loop is outermost and the clone is hoisted out of the
        // coordinate cross product: the inputs and the assertions are exactly
        // those the spec enumerates, but the dashboard is cloned **six** times
        // (three dashboards by two routes) rather than once per coordinate
        // tuple. The inner form cost 20,580 `Dashboard` clones and made this
        // one test heavy enough to destabilise the deadline-bounded
        // `ui::tests::wiring::` suite running beside it.
        for dashboard in &dashboards {
            let before = dashboard.clone();
            for route in [Route::List, Route::Detail] {
                let mut routed = dashboard.clone();
                routed.route = route;
                for kind in kinds {
                    for column in coords {
                        for row in coords {
                            for area in areas {
                                let bare = mouse_action(&routed, area, &m(kind, column, row));
                                for modifiers in modifiers {
                                    let event = MouseEvent {
                                        kind,
                                        column,
                                        row,
                                        modifiers,
                                    };
                                    assert_eq!(
                                        mouse_action(&routed, area, &event),
                                        bare,
                                        "{kind:?} at ({column}, {row}) in {area:?} under \
                                         {route:?}: {modifiers:?} must resolve as NONE does"
                                    );
                                }
                            }
                        }
                    }
                }
            }
            assert_eq!(
                dashboard, &before,
                "the dashboard is passed by shared reference and nothing mutated it"
            );
        }
    }

    #[test]
    fn the_two_regions_scroll_independently() {
        // `mouse-input`: "The two regions scroll independently at 120 columns".
        for route in [Route::Detail, Route::List] {
            let mut dashboard = mouse_dashboard(6, 0);
            dashboard.route = route;
            dashboard.selected = 1;
            dashboard.detail.source = (0..20).map(|i| format!("- line-{i:02}\n")).collect();

            let over_list = mouse_action(&dashboard, WIDE, &m(MouseEventKind::ScrollDown, 10, 10));
            let over_detail =
                mouse_action(&dashboard, WIDE, &m(MouseEventKind::ScrollDown, 80, 10));
            assert_eq!(over_list, Action::SelectNext, "{route:?}");
            assert_eq!(over_detail, Action::ScrollDown, "{route:?}");

            let mut listed = dashboard.clone();
            listed.apply(over_list);
            assert_eq!(listed.selected, dashboard.selected + 1);
            assert_eq!(listed.detail.scroll, 0);

            let mut detailed = dashboard.clone();
            detailed.apply(over_detail);
            assert_eq!(detailed.detail.scroll, 1);
            assert_eq!(detailed.selected, dashboard.selected);
        }
    }

    #[test]
    fn the_wheel_acts_over_a_border_and_not_the_chrome() {
        // `mouse-input`: "The wheel acts over a border and not over the chrome".
        let dashboard = mouse_dashboard(3, 0);
        assert_eq!(
            mouse_action(&dashboard, WIDE, &m(MouseEventKind::ScrollUp, 0, 1)),
            Action::SelectPrev,
            "the list region's own border column and row"
        );
        for (column, row) in [(10u16, 0u16), (10, 39), (200, 10)] {
            assert_eq!(
                mouse_action(&dashboard, WIDE, &m(MouseEventKind::ScrollUp, column, row)),
                Action::Ignore,
                "({column}, {row})"
            );
        }

        // The region under the wheel is the **whole** region — for the detail
        // region, its header row and its tab bar as well as its content area,
        // and its border. Without these four the wheel arms could stop
        // answering for `Zone::DetailTab` entirely with every test still green.
        let detail_border = 40u16;
        let header_row = tab_bar_row(WIDE, Route::List).y - 1;
        let bar_row = tab_bar_row(WIDE, Route::List).y;
        for (column, row, label) in [
            (detail_border, 5u16, "the detail region's own border column"),
            (50, header_row, "the detail region's header row"),
            (50, bar_row, "the detail region's tab-bar row"),
            (50, 10, "the detail content area"),
        ] {
            assert_eq!(
                mouse_action(&dashboard, WIDE, &m(MouseEventKind::ScrollUp, column, row)),
                Action::ScrollUp,
                "{label} at ({column}, {row})"
            );
            assert_eq!(
                mouse_action(
                    &dashboard,
                    WIDE,
                    &m(MouseEventKind::ScrollDown, column, row)
                ),
                Action::ScrollDown,
                "{label} at ({column}, {row})"
            );
        }
    }

    #[test]
    fn at_60_columns_only_the_routed_region_answers() {
        // `mouse-input`: "At 60 columns only the routed region answers the wheel".
        let mut listed = mouse_dashboard(3, 0);
        listed.route = Route::List;
        let mut detailed = mouse_dashboard(3, 0);
        detailed.route = Route::Detail;
        detailed.selected = 1;

        assert_eq!(
            mouse_action(&listed, NARROW, &m(MouseEventKind::ScrollDown, 30, 10)),
            Action::SelectNext
        );
        assert_eq!(
            mouse_action(&detailed, NARROW, &m(MouseEventKind::ScrollDown, 30, 10)),
            Action::ScrollDown
        );
    }

    #[test]
    fn horizontal_wheel_events_do_nothing() {
        // `mouse-input`: "Horizontal wheel events do nothing".
        let dashboard = mouse_dashboard(3, 0);
        for area in [WIDE, NARROW] {
            for kind in [MouseEventKind::ScrollLeft, MouseEventKind::ScrollRight] {
                // Over the list region, over the detail region, over the footer.
                for (column, row) in [(10u16, 10u16), (80, 10), (10, area.height - 1)] {
                    assert_eq!(
                        mouse_action(&dashboard, area, &m(kind, column, row)),
                        Action::Ignore,
                        "{kind:?} at ({column}, {row}) in {area:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_click_selects_and_a_second_click_opens() {
        // `mouse-input`: "A click selects a change row and a second click opens it".
        let mut dashboard = mouse_dashboard(4, 0);
        dashboard.detail.tab = 0;
        // Interior row 0 is the `active` header; rows 1..4 are the changes, so
        // the third drawn row is the second change.
        let row = list_row(WIDE, Route::List, 2);
        let action = mouse_action(&dashboard, WIDE, &left(5, row));
        assert_eq!(action, Action::Click(Target::Change(1)));

        dashboard.apply(action);
        let index = dashboard
            .targets()
            .iter()
            .position(|t| *t == Target::Change(1))
            .expect("drawn");
        assert_eq!(dashboard.selected, index);
        assert_eq!(dashboard.detail.tab, 0);
        assert_eq!(dashboard.detail.scroll, 0);
        assert_eq!(dashboard.route, Route::List);

        let second = mouse_action(&dashboard, WIDE, &left(5, row));
        assert_eq!(second, action);
        dashboard.apply(second);
        assert_eq!(dashboard.route, Route::Detail);
        assert_eq!(dashboard.detail.scroll, 0);
        assert_eq!(dashboard.selected, index);

        let third = mouse_action(&dashboard, WIDE, &left(5, row));
        assert_eq!(third, action);
        let before = dashboard.clone();
        dashboard.apply(third);
        assert_eq!(dashboard, before);
    }

    #[test]
    fn a_header_click_equals_space() {
        // `mouse-input`: "A click on a section header folds it exactly as `Space` does".
        let mut clicked = mouse_dashboard(3, 3);
        let row = list_row(WIDE, Route::List, 0);
        let action = mouse_action(&clicked, WIDE, &left(5, row));
        assert_eq!(action, Action::Click(Target::Section(SectionKey::Active)));

        clicked.apply(action);
        assert!(clicked.sections.collapsed.contains(&SectionKey::Active));
        assert_eq!(
            clicked.targets().get(clicked.selected),
            Some(&Target::Section(SectionKey::Active))
        );

        let mut spaced = mouse_dashboard(3, 3);
        spaced.selected = spaced
            .targets()
            .iter()
            .position(|t| *t == Target::Section(SectionKey::Active))
            .expect("drawn");
        spaced.apply(Action::ToggleSection);
        assert_eq!(clicked, spaced, "field for field");

        let unfold = mouse_action(&clicked, WIDE, &left(5, row));
        clicked.apply(unfold);
        assert!(!clicked.sections.collapsed.contains(&SectionKey::Active));
    }

    #[test]
    fn a_header_click_requests_the_archive_refresh() {
        // `mouse-input`: "A click on an archived header opens an unresolved
        // archive and requests its refresh".
        let mut dashboard = mouse_dashboard(3, 0);
        dashboard.changes.archived_total = 22;
        dashboard.sections.collapsed.insert(SectionKey::Archived);
        assert!(dashboard.changes.archived.is_empty());

        // Rows: the `active` header, three changes, then the archived header.
        let row = list_row(WIDE, Route::List, 4);
        let action = mouse_action(&dashboard, WIDE, &left(5, row));
        assert_eq!(action, Action::Click(Target::Section(SectionKey::Archived)));

        dashboard.apply(action);
        assert!(!dashboard.sections.collapsed.contains(&SectionKey::Archived));
        assert!(dashboard.refresh.requested);
    }

    /// A change carrying the five `tdd` artifacts, selected, at `Route::List`.
    fn tdd_dashboard() -> Dashboard {
        let change = crate::changes::fixture::with_artifacts(
            crate::changes::fixture::active("s0", 0, 1),
            &[
                ("proposal", &[]),
                ("specs", &[]),
                ("design", &[]),
                ("tasks", &[]),
                ("planning-review", &[]),
            ],
        );
        let mut dashboard = mouse_dashboard(0, 0);
        dashboard.changes = crate::changes::fixture::set(vec![change], Vec::new(), Vec::new());
        dashboard.selected = 1;
        dashboard
    }

    /// The terminal column of the `index`-th tab cell's first painted column.
    fn tab_cell_start(dashboard: &Dashboard, area: Rect, route: Route, index: usize) -> u16 {
        let bar = tab_bar_row(area, route);
        let change = dashboard.selected_change().expect("a change is selected");
        let cell = crate::ui::detail::tab_bar(&change.artifacts, dashboard.detail.tab, bar.width)
            .into_iter()
            .find(|t| t.index == Some(index))
            .expect("the cell is drawn");
        bar.x + cell.x
    }

    #[test]
    fn a_tab_click_switches_the_tab() {
        // `mouse-input`: "A click on a tab cell switches to that artifact".
        let mut dashboard = tdd_dashboard();
        dashboard.detail.scroll = 7;
        let bar = tab_bar_row(WIDE, Route::List);
        let third = tab_cell_start(&dashboard, WIDE, Route::List, 2);

        let action = mouse_action(&dashboard, WIDE, &left(third + 1, bar.y));
        assert_eq!(action, Action::SelectTab(2));
        dashboard.apply(action);
        assert_eq!(dashboard.detail.tab, 2);
        assert_eq!(dashboard.detail.scroll, 0);

        // The one separating column between two cells.
        let fresh = tdd_dashboard();
        let second = tab_cell_start(&fresh, WIDE, Route::List, 1);
        assert_eq!(
            mouse_action(&fresh, WIDE, &left(second - 1, bar.y)),
            Action::Ignore,
            "the separating column belongs to neither cell"
        );

        // A change with no artifacts draws only the `no artifacts` placeholder.
        let mut bare = mouse_dashboard(1, 0);
        bare.selected = 1;
        for column in bar.x..bar.x + bar.width {
            assert_eq!(
                mouse_action(&bare, WIDE, &left(column, bar.y)),
                Action::Ignore,
                "column {column} over the placeholder"
            );
        }
    }

    #[test]
    fn a_tab_click_equals_its_digit_key() {
        // `artifact-tabs`: "A tab click and its digit key are indistinguishable".
        let mut clicked = tdd_dashboard();
        clicked.detail.scroll = 7;
        let mut keyed = clicked.clone();

        let bar = tab_bar_row(WIDE, Route::List);
        let fourth = tab_cell_start(&clicked, WIDE, Route::List, 3);
        let action = mouse_action(&clicked, WIDE, &left(fourth + 1, bar.y));
        clicked.apply(action);
        keyed.apply(action_for(
            &press(KeyCode::Char('4'), KeyModifiers::NONE),
            false,
        ));

        assert_eq!(clicked, keyed, "field for field");
        assert_eq!(
            clicked.route,
            Route::List,
            "clicking a tab does not open the detail"
        );
    }

    #[test]
    fn a_tab_click_on_the_selected_cell_resets_nothing() {
        // `artifact-tabs`: "A tab click on the already-selected cell resets nothing".
        let mut dashboard = tdd_dashboard();
        dashboard.detail.tab = 2;
        dashboard.detail.scroll = 5;
        let bar = tab_bar_row(WIDE, Route::List);
        let third = tab_cell_start(&dashboard, WIDE, Route::List, 2);

        let action = mouse_action(&dashboard, WIDE, &left(third + 1, bar.y));
        assert_eq!(action, Action::SelectTab(2));
        dashboard.apply(action);
        assert_eq!(dashboard.detail.tab, 2);
        assert_eq!(dashboard.detail.scroll, 5);
    }

    #[test]
    fn clicks_that_address_nothing_are_inert() {
        // `mouse-input`: "Clicks that address nothing are inert".
        let mut dashboard = mouse_dashboard(0, 0);
        dashboard.changes.problems = vec!["a repository problem".to_string()];
        assert!(dashboard.visible().is_empty());

        let points = [
            (5u16, list_row(WIDE, Route::List, 0)), // the problem row
            (5, list_row(WIDE, Route::List, 1)),    // the `No changes yet` row
            (5, list_row(WIDE, Route::List, 5)),    // below the last drawn row
            (0, 1),                                 // the list region's border
            (41, 2),                                // the detail region's header row
            (41, 10),                               // the detail content area
            (10, 0),                                // the frame's header
            (10, 39),                               // the frame's footer
            (200, 10),                              // past the frame
        ];
        let before = dashboard.clone();
        for (column, row) in points {
            let action = mouse_action(&dashboard, WIDE, &left(column, row));
            assert_eq!(action, Action::Ignore, "({column}, {row})");
            let mut applied = dashboard.clone();
            applied.apply(action);
            assert_eq!(applied, before, "({column}, {row}) changed the dashboard");
        }
    }

    #[test]
    fn the_other_buttons_are_inert() {
        // `mouse-input`: "The other buttons and the non-press kinds are inert".
        let dashboard = mouse_dashboard(4, 0);
        let row = list_row(WIDE, Route::List, 2);
        for kind in [
            MouseEventKind::Down(MouseButton::Right),
            MouseEventKind::Down(MouseButton::Middle),
            MouseEventKind::Up(MouseButton::Left),
            MouseEventKind::Drag(MouseButton::Left),
            MouseEventKind::Moved,
        ] {
            let action = mouse_action(&dashboard, WIDE, &m(kind, 5, row));
            assert_eq!(action, Action::Ignore, "{kind:?}");
            // In particular, no press of any button reaches a launch action.
            // `mouse_action` is pure and takes no `Launcher`, so this is an
            // assertion on the returned `Action`, not on a double.
            assert_ne!(action, Action::LaunchApply);
            assert_ne!(action, Action::LaunchContinue);
            assert_ne!(action, Action::LaunchArchive);
            assert_ne!(action, Action::FocusAgent);
        }
    }

    #[test]
    fn a_click_acts_while_filtering() {
        // `mouse-input`: "A click selects while the filter is open".
        let mut filtering = mouse_dashboard(6, 0);
        filtering.filter.active = true;
        filtering.filter.query = "s".to_string();
        let closed = mouse_dashboard(6, 0);

        let row = list_row(WIDE, Route::List, 2);
        let action = mouse_action(&filtering, WIDE, &left(5, row));
        assert_eq!(
            action,
            mouse_action(&closed, WIDE, &left(5, row)),
            "the same press returns the same action with the filter closed"
        );

        filtering.apply(action);
        assert_eq!(
            filtering.targets().get(filtering.selected),
            Some(&Target::Change(1))
        );
        assert!(
            filtering.filter.active,
            "the click neither cancels the filter"
        );
        assert_eq!(filtering.filter.query, "s", "nor accepts it");
    }

    #[test]
    fn the_wheel_acts_while_filtering() {
        // `mouse-input`: "The wheel scrolls while the filter is open".
        let mut filtering = mouse_dashboard(6, 0);
        filtering.filter.active = true;
        filtering.filter.query = "s".to_string();
        let closed = mouse_dashboard(6, 0);

        for (column, expected) in [(10u16, Action::SelectNext), (80, Action::ScrollDown)] {
            let event = m(MouseEventKind::ScrollDown, column, 10);
            assert_eq!(mouse_action(&filtering, WIDE, &event), expected);
            assert_eq!(
                mouse_action(&closed, WIDE, &event),
                expected,
                "identical with the filter closed"
            );
        }
    }

    #[test]
    fn every_mouse_action_has_an_equal_key() {
        // `mouse-input`: "Every mouse action has a key that produces the same effect".
        // Four list-and-detail outcomes, each driven once by the mouse action and
        // once by the corresponding key at the corresponding route.
        let cases: [(Action, Action, Route); 4] = [
            (Action::SelectNext, Action::Next, Route::List),
            (Action::SelectPrev, Action::Prev, Route::List),
            (Action::ScrollDown, Action::Next, Route::Detail),
            (Action::ScrollUp, Action::Prev, Route::Detail),
        ];
        for (wheel, key, route) in cases {
            let mut wheeled = mouse_dashboard(6, 0);
            wheeled.route = route;
            wheeled.selected = 2;
            wheeled.detail.scroll = 3;
            let mut keyed = wheeled.clone();
            wheeled.apply(wheel);
            keyed.apply(key);
            assert_eq!(wheeled, keyed, "{wheel:?} against {key:?} at {route:?}");
        }

        // A section toggle driven by a click against `Space`.
        let mut clicked = mouse_dashboard(3, 3);
        let mut spaced = mouse_dashboard(3, 3);
        clicked.apply(Action::Click(Target::Section(SectionKey::Archived)));
        spaced.selected = spaced
            .targets()
            .iter()
            .position(|t| *t == Target::Section(SectionKey::Archived))
            .expect("drawn");
        spaced.apply(Action::ToggleSection);
        assert_eq!(clicked, spaced);

        // A tab switch driven by a tab click against the matching digit key.
        let mut tab_clicked = tdd_dashboard();
        let mut tab_keyed = tab_clicked.clone();
        let bar = tab_bar_row(WIDE, Route::List);
        let second = tab_cell_start(&tab_clicked, WIDE, Route::List, 1);
        tab_clicked.apply(mouse_action(
            &tab_clicked.clone(),
            WIDE,
            &left(second + 1, bar.y),
        ));
        tab_keyed.apply(action_for(
            &press(KeyCode::Char('2'), KeyModifiers::NONE),
            false,
        ));
        assert_eq!(tab_clicked, tab_keyed);
    }

    #[test]
    fn pointer_motion_does_not_draw() {
        // `dashboard-loop`: "Pointer motion does not cost a frame".
        fn drive(kind: MouseEventKind) -> LoopSummary {
            let mut queue: Vec<_> = (0..20)
                .map(|i| Ok(Some(crate::testutil::mouse(kind, i as u16, 10))))
                .collect();
            queue.push(Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))));
            drive_queue(queue)
        }

        for kind in [
            MouseEventKind::Moved,
            MouseEventKind::Drag(MouseButton::Left),
        ] {
            assert_eq!(
                drive(kind),
                LoopSummary {
                    frames: 1,
                    polls: 21
                },
                "{kind:?}"
            );
        }

        // An ignored **key** still redraws: the exemption is scoped to pointer
        // motion and did not become a general ignore-means-no-draw rule.
        let mut queue: Vec<_> = (0..20)
            .map(|_| Ok(Some(press(KeyCode::Char('z'), KeyModifiers::NONE))))
            .collect();
        queue.push(Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))));
        assert_eq!(
            drive_queue(queue),
            LoopSummary {
                frames: 21,
                polls: 21
            }
        );
    }

    /// Drive `run_loop` at 120x40 over `mouse_dashboard(3, 0)` with `queue`,
    /// returning the summary. Every collaborator is the inert double.
    fn drive_queue(queue: Vec<Result<Option<Event>, crate::ui::event::EventError>>) -> LoopSummary {
        let mut dashboard = mouse_dashboard(3, 0);
        let backend = TestBackend::new(120, 40);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut events = Script::new(queue);
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
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("loop ends")
    }

    #[test]
    fn a_click_after_motion_resolves_against_the_frame() {
        // `dashboard-loop`: "A click after motion still resolves against the
        // drawn frame".
        let row = list_row(WIDE, Route::List, 2);
        let mut dashboard = mouse_dashboard(3, 0);
        let backend = TestBackend::new(120, 40);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut queue: Vec<Result<Option<Event>, crate::ui::event::EventError>> = (0..5)
            .map(|i| Ok(Some(crate::testutil::mouse(MouseEventKind::Moved, i, 10))))
            .collect();
        queue.push(Ok(Some(crate::testutil::mouse(
            MouseEventKind::Down(MouseButton::Left),
            5,
            row,
        ))));
        queue.push(Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))));
        let mut events = Script::new(queue);
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
            dashboard.targets().get(dashboard.selected),
            Some(&Target::Change(1)),
            "the press selects that row, exactly as it does with no motion before it"
        );
        assert_eq!(
            summary,
            LoopSummary {
                frames: 2,
                polls: 7
            }
        );
    }

    /// A `TestBackend` that reports 120x40 until its first `draw` and 60x20
    /// after — the shape a real terminal resize has, since
    /// `Terminal::autoresize` reads `Backend::size` at the top of every `draw`.
    struct ShrinkingBackend {
        inner: TestBackend,
        drawn: std::cell::Cell<bool>,
    }

    impl Backend for ShrinkingBackend {
        type Error = <TestBackend as Backend>::Error;

        fn draw<'a, I>(&mut self, content: I) -> Result<(), Self::Error>
        where
            I: Iterator<Item = (u16, u16, &'a Cell)>,
        {
            self.drawn.set(true);
            self.inner.draw(content)
        }

        fn hide_cursor(&mut self) -> Result<(), Self::Error> {
            self.inner.hide_cursor()
        }

        fn show_cursor(&mut self) -> Result<(), Self::Error> {
            self.inner.show_cursor()
        }

        fn get_cursor_position(&mut self) -> Result<Position, Self::Error> {
            self.inner.get_cursor_position()
        }

        fn set_cursor_position<P: Into<Position>>(
            &mut self,
            position: P,
        ) -> Result<(), Self::Error> {
            self.inner.set_cursor_position(position)
        }

        fn clear(&mut self) -> Result<(), Self::Error> {
            self.inner.clear()
        }

        fn clear_region(&mut self, clear_type: ClearType) -> Result<(), Self::Error> {
            self.inner.clear_region(clear_type)
        }

        fn size(&self) -> Result<Size, Self::Error> {
            Ok(if self.drawn.get() {
                Size::new(60, 20)
            } else {
                Size::new(120, 40)
            })
        }

        fn window_size(&mut self) -> Result<ratatui::backend::WindowSize, Self::Error> {
            self.inner.window_size()
        }

        fn flush(&mut self) -> Result<(), Self::Error> {
            self.inner.flush()
        }
    }

    #[test]
    fn a_resize_before_a_click_costs_one_frame() {
        // `dashboard-loop`: "A resize between the draw and the click costs one
        // frame, not a panic", and — the half this test exists for — "the area
        // used SHALL be the one just drawn, never a stored size and never the
        // size at startup".
        //
        // The point is chosen to **discriminate**, which the obvious one does
        // not. At 120x40 the detail region's tab bar is row 3 spanning columns
        // 41-118, and the third `tdd` cell (` design `) is painted at columns
        // 60-67 — past the 60-column frame's right edge entirely. So:
        //
        //   - resolved against the **stale** 120-column frame, (62, 3) is a
        //     drawn tab cell and yields `Action::SelectTab(2)`, moving
        //     `detail.tab` to 2;
        //   - resolved against the **fresh** 60-column frame, column 62 is
        //     outside the frame, `Zone::Outside`, and yields `Action::Ignore`.
        //
        // A point in the detail *content* area would not discriminate: it
        // resolves to `Action::Ignore` under both geometries, one via
        // `Zone::Detail` and the other via `Zone::Outside`, so the assertion
        // would hold on a loop that had pinned the startup frame forever.
        let mut dashboard = tdd_dashboard();
        let backend = ShrinkingBackend {
            inner: TestBackend::new(120, 40),
            drawn: std::cell::Cell::new(false),
        };
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut events = Script::new(vec![
            Ok(Some(Event::Resize(60, 20))),
            Ok(Some(crate::testutil::mouse(
                MouseEventKind::Down(MouseButton::Left),
                62,
                3,
            ))),
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
        let before = dashboard.clone();
        // The geometry this test's discrimination rests on, derived rather than
        // asserted from memory: the third cell really is painted past column 59.
        let bar = tab_bar_row(WIDE, Route::List);
        let third = tab_cell_start(&dashboard, WIDE, Route::List, 2);
        assert!(
            third >= 60 && bar.y == 3,
            "the fixture must place the third tab cell past the 60-column frame: \
             cell at {third}, bar row {}",
            bar.y
        );
        assert_eq!(
            mouse_action(&dashboard, WIDE, &left(62, 3)),
            Action::SelectTab(2),
            "against the 120-column frame the press is a tab click"
        );
        assert_eq!(
            mouse_action(&dashboard, NARROW, &left(62, 3)),
            Action::Ignore,
            "against the 60-column frame it is outside the frame"
        );

        let summary = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("the loop completes without panicking");

        assert_eq!(
            summary,
            LoopSummary {
                frames: 3,
                polls: 3
            },
            "the resize costs one frame, not a panic"
        );
        assert_eq!(
            dashboard.detail.tab, before.detail.tab,
            "the press was resolved against the 60-column frame drawn after the \
             resize, not against the 120-column frame drawn before it"
        );
        assert_eq!(dashboard.selected, before.selected);
    }

    #[test]
    fn the_loop_routes_mouse_and_key_to_different_mappers() {
        // `dashboard-loop`: "The loop routes mouse and key events to different
        // mappers". Two runs, because the state between two events inside one
        // `run_loop` call is not observable from outside it.
        fn drive(queue: Vec<Result<Option<Event>, crate::ui::event::EventError>>) -> Dashboard {
            let mut dashboard = mouse_dashboard(3, 0);
            // A change carrying a readable artifact, and sixty rendered lines
            // against a 34-row content area: the loop's own `sync_detail` fills
            // `detail.source` from the reader before every draw, and
            // `normalise_scroll` would clamp a one-line scroll straight back to
            // zero against anything shorter than the area.
            dashboard.changes = crate::changes::fixture::set(
                vec![
                    crate::changes::fixture::with_artifacts(
                        crate::changes::fixture::active("s0", 0, 1),
                        &[("proposal", &["/repo/proposal.md"])],
                    ),
                    crate::changes::fixture::active("s1", 0, 1),
                    crate::changes::fixture::active("s2", 0, 1),
                ],
                Vec::new(),
                Vec::new(),
            );
            dashboard.selected = 1;
            let source: String = (0..60).map(|i| format!("- line-{i:02}\n")).collect();
            let backend = TestBackend::new(120, 40);
            let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
            let mut events = Script::new(queue);
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
                &|_: &std::path::Path| Ok(source.clone()),
                Duration::from_millis(1),
            )
            .expect("loop ends");
            dashboard
        }

        let wheel = crate::testutil::mouse(MouseEventKind::ScrollDown, 80, 10);
        let quit = press(KeyCode::Char('q'), KeyModifiers::NONE);

        let after_wheel = drive(vec![Ok(Some(wheel.clone())), Ok(Some(quit.clone()))]);
        assert_eq!(after_wheel.detail.scroll, 1, "the wheel reached ScrollDown");
        assert_eq!(after_wheel.selected, 1, "and did not move the selection");

        let after_key = drive(vec![
            Ok(Some(wheel)),
            Ok(Some(press(KeyCode::Char('j'), KeyModifiers::NONE))),
            Ok(Some(quit)),
        ]);
        assert_eq!(after_key.selected, 2, "the key reached Next");
        assert_eq!(
            after_key.detail.scroll, 0,
            "a selection move resets the scroll, so neither took the other's path"
        );
    }

    /// `mouse-input`'s acceptance harness: a `Route::List` dashboard over three
    /// active changes with the cursor on the **first** of them — target 1, since
    /// target 0 is the `active` header — drawn at 120x40, driven with `event`
    /// and then `q`, returning the last frame's buffer.
    ///
    /// Every collaborator is replaced (the inert doubles), the artifact reader
    /// is a closure, and the terminal is a `TestBackend`: design.md -> Test
    /// Boundaries names exactly this set.
    fn drive_one_event(event: Event) -> ratatui::buffer::Buffer {
        let mut dashboard = dashboard_with_change("/repo", "alpha", 0, 0);
        dashboard.changes = crate::changes::fixture::set(
            vec![
                crate::changes::fixture::active("alpha", 0, 0),
                crate::changes::fixture::active("beta", 0, 0),
                crate::changes::fixture::active("gamma", 0, 0),
            ],
            Vec::new(),
            Vec::new(),
        );
        dashboard.selected = 1;

        let backend = TestBackend::new(120, 40);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut events = Script::new(vec![
            Ok(Some(event)),
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
        run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
            &|_: &std::path::Path| Ok(String::new()),
            Duration::from_millis(1),
        )
        .expect("loop ends");
        terminal.backend().buffer().clone()
    }

    #[test]
    fn a_mouse_event_moves_the_selection_through_the_loop() {
        // `mouse-input`: "A mouse event is resolved through the loop and a key is
        // not". At 120x40 the list interior is `x: 1, y: 2, width: 38, height: 36`,
        // so interior row 2 — the `active` header, `alpha`, then `beta` — is
        // terminal row 4, and the selection marker is the interior's own first
        // column.
        let buf = drive_one_event(crate::testutil::mouse(
            ratatui::crossterm::event::MouseEventKind::Down(
                ratatui::crossterm::event::MouseButton::Left,
            ),
            5,
            4,
        ));
        assert_eq!(
            cell(&buf, 1, 4).symbol(),
            ">",
            "the frame after the press carries the selection marker on the clicked row: {:?}",
            row_text(&buf, 4)
        );

        // The key mapper is unchanged and the loop's own mouse handling is what
        // moved the selection.
        let event = crate::testutil::mouse(
            ratatui::crossterm::event::MouseEventKind::Down(
                ratatui::crossterm::event::MouseButton::Left,
            ),
            5,
            4,
        );
        assert_eq!(
            crate::ui::app::action_for(&event, false),
            crate::ui::app::Action::Ignore
        );
        assert_eq!(
            crate::ui::app::action_for(&event, true),
            crate::ui::app::Action::Ignore
        );
    }
}
