//! The event loop: draw first, then wait. See
//! `openspec/changes/tui-shell/specs/dashboard-loop/spec.md`.

use std::time::Duration;

use ratatui::Terminal;
use ratatui::backend::Backend;

use ratatui::crossterm::event::{Event, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::ui::app::{
    Action, ArtifactReader, ClipboardWriter, Dashboard, SelectPhase, Target, action_for,
};
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
///
/// `text-selection`'s addition: `write` is threaded in beside `read`, on exactly its terms
/// (design.md -> Decision 11) — `run_loop` is the one place that both applies a mouse event
/// and holds a `ClipboardWriter`, so it is also the one place that can tell a completing
/// press or drag apart from an ordinary one. See `maybe_copy_selection`.
pub fn run_loop<B: Backend, E: EventSource>(
    terminal: &mut Terminal<B>,
    dashboard: &mut Dashboard,
    events: &mut E,
    live: &mut Live<'_>,
    read: ArtifactReader<'_>,
    write: ClipboardWriter<'_>,
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
            dashboard.normalise_help_scroll(area);
        }
        draw = true;

        let timeout = crate::watch::poll_timeout(
            tick,
            crate::watch::soonest(live.fs.pending_in(), live.agents.pending_in()),
        );
        let event = events.next_event(timeout).map_err(LoopError::Events)?;
        polls += 1;

        if let Some(event) = event {
            // `text-selection`: the granularity standing before this event is applied — the
            // one piece `maybe_copy_selection` needs that `dashboard.apply` overwrites, so it
            // is read here, before the match below, on every event rather than only mouse
            // ones (cheap: `Option<Granularity>` is `Copy`).
            let before_granularity = dashboard.selection.as_ref().map(|s| s.granularity);
            // One action per event, and the quit check is unchanged: a mouse
            // event can neither apply two actions nor bypass it.
            let mouse_kind = match &event {
                // The mouse is resolved against the frame just drawn — never a
                // stored size and never the size at startup. A resize between
                // the draw and the click costs at most one mis-targeted event,
                // which the next frame corrects: the same one-frame window
                // `normalise_scroll` already accepts.
                Event::Mouse(mouse) => Some(mouse.kind),
                _ => None,
            };
            let action = match &event {
                Event::Mouse(mouse) => {
                    // No filter flag: a click is unambiguous where a keystroke
                    // is not, so the rule is structural rather than a branch.
                    let resolved = mouse_action(dashboard, area, mouse);
                    // `text-selection`: the exemption is keyed on the event
                    // kind rather than on "any motion" — `Moved` still costs
                    // no frame unconditionally, but a `Drag` costs no frame
                    // only when it resolved to `Action::Ignore` (outside the
                    // selectable region). A `Drag` that began or extended a
                    // selection draws, because the selection's focus moved
                    // and the highlight is what tells the reader what they
                    // are selecting — a drag that did not redraw would render
                    // the feature invisible while it was being used.
                    let motion_ignored = match mouse.kind {
                        MouseEventKind::Moved => true,
                        MouseEventKind::Drag(_) => resolved == Action::Ignore,
                        _ => false,
                    };
                    if motion_ignored {
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
                    resolved
                }
                _ => action_for(&event, dashboard.filter.active),
            };
            dashboard.apply(action);
            if let Some(kind) = mouse_kind {
                maybe_copy_selection(dashboard, kind, before_granularity, write);
            }
            if dashboard.quit {
                break;
            }
        }
    }
    Ok(LoopSummary { frames, polls })
}

/// `text-selection`: copy the selection's text through `write` when, and only when, `kind`
/// and the granularity transition `apply` just performed together mean this event **completed**
/// a selection — a drag's finish, the second press, or the third (`specs/text-selection/spec.md`
/// -> "The selected text is copied through the terminal seam", design.md -> Decisions 11 and 12).
///
/// A left press completes a selection exactly when `apply` widened it from `Armed` to `Word` or
/// from `Word` to `Row` — compared against `before`, the granularity standing immediately before
/// this event was applied, so a fourth-or-later press at an already-`Row` cell (unchanged by
/// `apply_select`) is told apart from the second and third, which are not. A left release
/// completes a selection exactly when the current granularity is `Span`: only a drag sets that
/// granularity, through `SelectPhase::Extend`, so a bare click's release (no drag in between)
/// never reaches this arm. Every other event kind completes nothing.
///
/// The range copied is exactly the range [`view::highlight_span`] highlights, resolved against
/// `dashboard.detail.drawn_width` — the content width the frame just drawn actually used
/// (`Dashboard::normalise_scroll`'s own one exception to storing no geometry) — rather than
/// recomputed from the current mouse position, which a release outside the content area would
/// leave with no zone to read one from. `None` from either — no frame drawn yet, or a granularity
/// (`Word` over whitespace) that highlights nothing — copies nothing, matching "second press
/// whose cell holds only whitespace... selects nothing" copying nothing either.
///
/// `write`'s result becomes `Selection::problem`: `Ok` clears it, `Err` stores the reason,
/// exactly the moments design.md -> Decision 12 names. Neither a non-completing gesture nor an
/// unrelated action (a click elsewhere, a scroll) reaches this function's body at all — the
/// caller only invokes it for the mouse event `apply` just processed, and every non-completing
/// path returns before calling `write`, so `Selection::problem`'s existing `apply_select`-set
/// `None` is left standing for a fresh gesture (design.md -> Decision 12's own note in
/// `apply_select`'s doc comment).
fn maybe_copy_selection(
    dashboard: &mut Dashboard,
    kind: MouseEventKind,
    before: Option<crate::ui::app::Granularity>,
    write: crate::ui::app::ClipboardWriter<'_>,
) {
    use crate::ui::app::Granularity;

    let after = dashboard.selection.as_ref().map(|s| s.granularity);
    let completing = match kind {
        MouseEventKind::Down(MouseButton::Left) => {
            after != before && matches!(after, Some(Granularity::Word) | Some(Granularity::Row))
        }
        MouseEventKind::Up(MouseButton::Left) => matches!(after, Some(Granularity::Span)),
        _ => false,
    };
    if !completing {
        return;
    }
    let Some(width) = dashboard.detail.drawn_width else {
        return;
    };
    let Some(selection) = dashboard.selection.clone() else {
        return;
    };
    let rows =
        crate::ui::detail::content_lines(&dashboard.detail, dashboard.selected_change(), width);
    let Some((start, end)) = view::highlight_span(&rows, &selection) else {
        return;
    };
    let text = crate::ui::detail::span_text(&rows, start, end);
    let result = write(&text).err();
    if let Some(selection) = dashboard.selection.as_mut() {
        selection.problem = result;
    }
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
    // `help-overlay`: while the overlay is open every event resolves against
    // the **band** and `layout::zone` is not consulted at all. Written here
    // rather than in a helper of its own, and deliberately: `SPEC.md` ->
    // Keys' mouse table is bound to *this function's* body by
    // `tests/doc_contract.rs`'s `mouse_bindings_match_spec_md`, which cuts
    // from `pub fn mouse_action(` to the next `}` at column zero — a branch
    // extracted into a sibling function would take `Action::ToggleHelp` out
    // of that slice and let the table and the resolver drift with every test
    // still green.
    if dashboard.help.open {
        let point = ratatui::layout::Position::new(mouse.column, mouse.row);
        // Outside the frame is `Action::Ignore` on every event kind, checked
        // before anything else: such a point is outside the band too, but it
        // is not a gesture the pane received, and it must not dismiss the
        // overlay. Same reasoning as the `Zone::Outside` arms below.
        if !area.contains(point) {
            return Action::Ignore;
        }
        return match mouse.kind {
            // The wheel scrolls the overlay from anywhere in the frame, not
            // only from over the band: while a modal is open the reader's
            // attention is the modal, and a wheel over the rows of margin or
            // footer that are not the band doing nothing would read as a dead
            // pointer.
            MouseEventKind::ScrollDown => Action::ScrollDown,
            MouseEventKind::ScrollUp => Action::ScrollUp,
            MouseEventKind::Down(MouseButton::Left) => {
                // The band derived from the same `area`, through exactly the
                // pair `ui::view::render` hands `ui::help::render` — so the
                // mouse target is the rectangle the reader is looking at
                // rather than a second one computed here.
                let (body, _) = crate::ui::layout::split_frame(area);
                let band = crate::ui::layout::help_band(body, crate::ui::help::content_rows());
                if band.contains(point) {
                    // Read-only and holding no control: no row is a button,
                    // and there is nothing inside it a click could mean.
                    Action::Ignore
                } else {
                    Action::ToggleHelp
                }
            }
            // Both horizontal wheel directions, right and middle presses,
            // every release, every drag, and pointer motion — so `run_loop`'s
            // motion exemption is unchanged and the overlay scrolls in one
            // dimension only.
            _ => Action::Ignore,
        };
    }
    let zone = crate::ui::layout::zone(area, dashboard.route, mouse.column, mouse.row);
    match mouse.kind {
        MouseEventKind::ScrollDown => match zone {
            Zone::List | Zone::ListRow { .. } => Action::SelectNext,
            Zone::Detail | Zone::DetailTab { .. } | Zone::DetailRow { .. } => Action::ScrollDown,
            Zone::Outside => Action::Ignore,
        },
        MouseEventKind::ScrollUp => match zone {
            Zone::List | Zone::ListRow { .. } => Action::SelectPrev,
            Zone::Detail | Zone::DetailTab { .. } | Zone::DetailRow { .. } => Action::ScrollUp,
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
            // `text-selection`: a header row still folds exactly as before;
            // any other row of the content area arms a selection instead of
            // moving the detail cursor, replacing the removed detail-line
            // target (design.md -> Decision 9). `detail_cell` resolves only
            // the cell; the `Action` it becomes is constructed here.
            Zone::DetailRow { content, row } => {
                match detail_cell(dashboard, content, row, mouse.column) {
                    Some((line, _, Some(section))) => {
                        Action::Click(Target::DetailHeader { line, section })
                    }
                    Some((line, column, None)) => {
                        Action::Select(SelectPhase::Begin { line, column })
                    }
                    None => Action::Ignore,
                }
            }
            Zone::List | Zone::Detail | Zone::Outside => Action::Ignore,
        },
        // `text-selection`: a left drag begins only on a non-header
        // `Zone::DetailRow` (design.md -> Decision 1); every other zone
        // produces `Action::Ignore` when nothing is in progress, so no
        // existing binding's dispatch timing changes and folding a section
        // can never begin a selection.
        MouseEventKind::Drag(MouseButton::Left) => match zone {
            Zone::DetailRow { content, row } => {
                match detail_cell(dashboard, content, row, mouse.column) {
                    Some((line, column, None)) => {
                        Action::Select(SelectPhase::Extend { line, column })
                    }
                    Some((_, _, Some(_))) | None => Action::Ignore,
                }
            }
            // `text-selection`'s clamp: a drag that is extending a
            // selection already in progress and has left the content area
            // — vertically or horizontally — follows the pointer to that
            // area's nearest edge rather than freezing at its last
            // in-area position (`specs/text-selection/spec.md` -> "A drag
            // past the edge clamps and does not scroll"). `dashboard-loop`'s
            // "a drag outside the selectable region... costs no frame" is
            // about a drag with **no** selection in progress, and stays
            // `Action::Ignore` here on exactly that condition: a header
            // row inside the content area is excluded above and never
            // reaches this arm at all, since it is never "outside" the
            // area in the sense this clamp means.
            Zone::ListRow { .. }
            | Zone::DetailTab { .. }
            | Zone::List
            | Zone::Detail
            | Zone::Outside => {
                if dashboard.selection.is_some()
                    && let Some(content) = content_area(dashboard, area)
                {
                    let (line, column) =
                        clamp_to_content(dashboard, content, mouse.column, mouse.row);
                    Action::Select(SelectPhase::Extend { line, column })
                } else {
                    Action::Ignore
                }
            }
        },
        // Right and middle presses, every release, and both horizontal wheel
        // directions: there is no context menu and the pane scrolls in one
        // dimension only.
        _ => Action::Ignore,
    }
}

/// Resolve a `Zone::DetailRow` press or drag to its content-line index, its
/// own section index when the row is a section header, and its display
/// column relative to the content area's own left edge — `mouse_action` has
/// `content`'s own width and can call `ui::detail::content_lines` and
/// `ui::detail::section_at`; `Dashboard::apply`/`apply_select` has neither,
/// so every value this returns is already resolved against the frame just
/// drawn (design.md -> Decision 7 and Decision 8).
///
/// `None` for a row past the last row `content_lines` produced — a press or
/// drag below a foldable artifact's own last drawn row, or below a short
/// document's. Unlike the click resolver this replaces, this is **not**
/// gated on `Detail::foldable()`: `text-selection`'s selection must not
/// inherit that short-circuit (design.md -> Decision 8), and a header row can
/// only exist when the content actually split, which already implies
/// `foldable()` is true.
fn detail_cell(
    dashboard: &Dashboard,
    content: Rect,
    row: u16,
    column: u16,
) -> Option<(usize, u16, Option<usize>)> {
    let rows = crate::ui::detail::content_lines(
        &dashboard.detail,
        dashboard.selected_change(),
        content.width,
    );
    let offset = crate::ui::layout::viewport(rows.len(), dashboard.detail.scroll, content.height);
    let line = offset + row as usize;
    if line >= rows.len() {
        return None;
    }
    let column = column.saturating_sub(content.x);
    Some((
        line,
        column,
        crate::ui::detail::section_at(&rows, offset, row),
    ))
}

/// The detail region's content area for `area` at `dashboard.route`, or
/// `None` when no detail region is drawn at all — the narrow layout's
/// `Route::List`. `zone` already derives this same `Rect` for a point that
/// falls inside it (`Zone::DetailRow`'s own `content` field); `text-selection`'s
/// clamp needs it for a point that has left it, where `zone` returns no
/// `Rect` at all. Mirrors `zone`'s own derivation through `split_frame`,
/// `split_body`, `interior`, and `split_detail` exactly, holding no
/// arithmetic of its own.
fn content_area(dashboard: &Dashboard, area: Rect) -> Option<Rect> {
    let (body, _) = crate::ui::layout::split_frame(area);
    let (_, divider, detail) = crate::ui::layout::split_body(body, dashboard.route);
    let inner = crate::ui::layout::interior(detail?, crate::ui::layout::detail_gutters(divider));
    let (_, _, content) = crate::ui::layout::split_detail(inner);
    Some(content)
}

/// Clamp a point that has left `content` to its nearest edge and resolve it
/// to a content-line index and display column, on exactly [`detail_cell`]'s
/// terms — `specs/text-selection/spec.md` -> "A drag past the edge clamps
/// and does not scroll": the focus follows the pointer to the content
/// area's own edge and no further, and `detail.scroll` is never consulted
/// to move toward it, only to locate what is already drawn.
///
/// The vertical bound is the **lesser** of the content area's own last row
/// and the document's own last drawn line — a document shorter than the
/// area clamps to its own last line, never to an empty row the area's
/// height would otherwise offer, which is what keeps this agree with
/// `detail_cell` returning `None` for exactly that row when a press or a
/// drag lands on it directly.
fn clamp_to_content(dashboard: &Dashboard, content: Rect, column: u16, row: u16) -> (usize, u16) {
    let rows = crate::ui::detail::content_lines(
        &dashboard.detail,
        dashboard.selected_change(),
        content.width,
    );
    let offset = crate::ui::layout::viewport(rows.len(), dashboard.detail.scroll, content.height);
    let visible = rows
        .len()
        .saturating_sub(offset)
        .min(content.height as usize);
    let max_row_offset = visible.saturating_sub(1) as u16;
    let row_offset = row.saturating_sub(content.y).min(max_row_offset);
    let line = offset + row_offset as usize;
    let max_column = content.width.saturating_sub(1);
    let column = column.saturating_sub(content.x).min(max_column);
    (line, column)
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
    use crate::ui::app::{ArtifactSection, Dashboard, Route};
    use crate::ui::driver::{Live, LoopError, LoopSummary, TICK, run_loop};
    use crate::ui::view;

    /// A `Route::List` dashboard over one active change, `name`, at
    /// `completed` of `total` — group 9's live-tier tests need a dashboard
    /// whose list row carries an assertable progress pair, not
    /// `dashboard()`'s empty set.
    fn dashboard_with_change(repo: &str, name: &str, completed: usize, total: usize) -> Dashboard {
        Dashboard {
            selection: None,
            help: crate::ui::app::Help {
                open: false,
                scroll: 0,
            },
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
                sections: Vec::new(),
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
                expanded: std::collections::BTreeSet::new(),
                drawn_width: None,
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
            selection: None,
            help: crate::ui::app::Help {
                open: false,
                scroll: 0,
            },
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
                sections: Vec::new(),
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
                expanded: std::collections::BTreeSet::new(),
                drawn_width: None,
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
            &|_: &str| Ok(()),
            Duration::from_millis(1),
        );
        assert!(matches!(result, Err(LoopError::Events(_))));

        let buf = terminal.backend().buffer();
        assert_eq!(&row_text(buf, 0)[1..10], "demo-repo");
        assert_eq!(cell(buf, 40, 1).symbol(), "│");
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
            &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
        assert!(
            !(0..buf.area.height).any(|y| row_text(buf, y).contains("demo-repo")),
            "the route change stopped drawing the list region's own heading"
        );
        assert!(!(0..buf.area.height).any(|y| row_text(buf, y).contains("Changes")));
    }

    fn twenty_line_detail_dashboard() -> Dashboard {
        // A real selected change with a real artifact PATH, not
        // `empty_set()` and not a path-free artifact: `run_loop` now
        // calls `sync_detail` before every draw, and a path-free artifact
        // would clear `detail.sections` back to empty on the very first
        // iteration. Tests driving this dashboard through `run_loop` pass
        // a reader supplying `detail.sections`'s own twenty-line text for
        // that path, so the manually-set source and the injected reader
        // agree, exactly as `ui::mod`'s acceptance test does.
        let change = crate::changes::fixture::with_artifacts(
            crate::changes::fixture::active("detail-view", 4, 9),
            &[("proposal", &["/repo/p.md"])],
        );
        Dashboard {
            selection: None,
            help: crate::ui::app::Help {
                open: false,
                scroll: 0,
            },
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
                sections: vec![ArtifactSection {
                    label: Some(String::new()),
                    text: (0..20).map(|i| format!("- line-{i:02}\n")).collect(),
                    depth: 0,
                    progress: None,
                    operation: None,
                }],
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
                expanded: std::collections::BTreeSet::new(),
                drawn_width: None,
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

            let recorder = crate::testutil::RecordingReader::always(Ok(dashboard.detail.sections
                [0]
            .text
            .clone()));
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
                &|_: &str| Ok(()),
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
            let base = if width == 60 { 1 } else { 42 };
            let row_at = |y: u16| -> String {
                (base..base + 9)
                    .map(|x| row_text(buf, y).chars().nth(x as usize).unwrap())
                    .collect()
            };
            // The content area starts three rows below the interior (the
            // heading, tab bar, rule, and content padding rows above it)
            // and is two rows shorter than the interior, and
            // `normalise_scroll` now clamps against the content area's own
            // height, so the 14-row content area draws lines 6 through 19,
            // not 4 through 19.
            assert_eq!(row_at(5), "• line-06", "width {width}");
            assert_eq!(row_at(18), "• line-19", "width {width}");
        }
    }

    /// The checklist's own text: twenty unchecked items under **no**
    /// heading — the checklist grammar over `task-parsing`'s parse, so its
    /// line count (bar + blank + twenty items) genuinely differs from what
    /// `ui::markdown::lines` produces for the same bytes.
    ///
    /// Headingless on purpose. A tracked-tasks file carrying items now
    /// splits at its headings, so one heading would put `## Tasks` into a
    /// section `label` and leave a single section — not foldable, and with
    /// the heading row gone the checklist and the markdown body would clamp
    /// to the same line, which is the one thing the assertion below exists
    /// to deny. Twenty items under no heading is the **offset-rule** half of
    /// task 6.1; group 6 adds the cursor-rule half, twenty under two.
    fn twenty_task_source() -> String {
        (0..20).map(|i| format!("- [ ] line-{i:02}\n")).collect()
    }

    /// The very same twenty unchecked items, split across **two** `##`
    /// headings — the **cursor-rule** half of task 6.1. Two headings yield two
    /// sections, which is what makes the tab foldable and puts
    /// `normalise_scroll` on the line-cursor branch; the headingless source
    /// above yields one and keeps the offset branch, so the two runs differ in
    /// exactly the one thing the scenario exists to discriminate.
    fn twenty_task_source_under_two_headings() -> String {
        let first: String = (0..10).map(|i| format!("- [ ] line-{i:02}\n")).collect();
        let second: String = (10..20).map(|i| format!("- [ ] line-{i:02}\n")).collect();
        format!("## A\n\n{first}\n## B\n\n{second}")
    }

    /// A task file of three **complete** groups of two items each — every
    /// subtree finished, so `sync_detail`'s seed opens none of them and the
    /// tab draws three collapsed headers over six hidden item lines.
    const THREE_COMPLETE_GROUPS: &str = "## 1. A\n\n- [x] a1\n- [x] a2\n\n\
         ## 2. B\n\n- [x] b1\n- [x] b2\n\n\
         ## 3. C\n\n- [x] c1\n- [x] c2\n";

    /// A dashboard whose one selected change carries two artifacts —
    /// `proposal` (unmarked) at position 0 and `tasks` (marked
    /// `tracks_tasks`) at position 1 — with the tracked-tasks tab
    /// selected. `detail-scroll`'s two group-8 scenarios both need a real
    /// tab to move *away from* the checklist to, which a single-artifact
    /// change cannot provide.
    fn twenty_task_detail_dashboard() -> Dashboard {
        task_detail_dashboard(0, 20)
    }

    /// `twenty_task_detail_dashboard`'s shape with the change's own
    /// `progress` supplied: the progress bar counts the **change**, not the
    /// file, so a fixture whose file is wholly finished still needs its own
    /// pair rather than the twenty-item one.
    fn task_detail_dashboard(completed: usize, total: usize) -> Dashboard {
        let change = crate::changes::fixture::track_tasks_at(
            crate::changes::fixture::with_artifacts(
                crate::changes::fixture::active("detail-view", completed, total),
                &[
                    ("proposal", &["/repo/p.md"]),
                    ("tasks", &["/repo/tasks.md"]),
                ],
            ),
            1,
        );
        Dashboard {
            selection: None,
            help: crate::ui::app::Help {
                open: false,
                scroll: 0,
            },
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
                sections: Vec::new(),
                scroll: 0,
                tab: 1,
                problems: Vec::new(),
                loaded: None,
                expanded: std::collections::BTreeSet::new(),
                drawn_width: None,
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
                &|_: &str| Ok(()),
                Duration::from_millis(1),
            )
            .expect("loop ends");

            // bar (1) + blank (1) + twenty items (20) = 22 lines; a 14-row
            // content area clamps to 22 - 14 = 8, not 20. The source carries
            // no heading, so it contributes one `None`-labelled section and
            // the tab stays non-foldable — this is the offset rule, and the
            // clamp is still an offset rather than a line cursor.
            assert_eq!(dashboard.detail.scroll, 8, "width {width}");

            let interior = if width == 60 { 58 } else { 78 };
            let markdown_len = crate::ui::markdown::lines(&source, interior).len();
            let markdown_clamp = markdown_len.saturating_sub(14);
            assert_ne!(
                dashboard.detail.scroll, markdown_clamp,
                "width {width}: the clamp must differ from the markdown body's own"
            );

            let buf = terminal.backend().buffer();
            let base = if width == 60 { 1 } else { 42 };
            let row_at = |y: u16, len: usize| -> String {
                (base..base + len as u16)
                    .map(|x| row_text(buf, y).chars().nth(x as usize).unwrap())
                    .collect()
            };
            assert_eq!(
                row_at(18, 11),
                "[ ] line-19",
                "width {width}: the clamp used the body that was actually drawn"
            );
        }
    }

    /// `detail-scroll` :: "Scrolling the checklist is clamped against the
    /// checklist's own length" — the **cursor-rule** half: the same twenty
    /// items under **two** headings, which split into two sections and make
    /// the tab foldable.
    ///
    /// Deviation from the scenario's letter, recorded here rather than left
    /// to be rediscovered: the scenario scripts twenty `j` presses, but the
    /// foldable body is twenty-five rows (bar, blank, two open headers, the
    /// twenty items, and one blank separator), so twenty presses land at
    /// twenty and the clamp never binds at all — the scenario's own "not to
    /// `20`" and "the last row holds the checklist's last item" clauses both
    /// require it to bind. Thirty presses are scripted so it does.
    #[test]
    fn checklist_scroll_is_clamped_to_the_cursor_when_the_file_splits() {
        for width in [120u16, 60] {
            let backend = TestBackend::new(width, 20);
            let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
            let mut dashboard = twenty_task_detail_dashboard();
            let source = twenty_task_source_under_two_headings();

            let mut presses: Vec<_> = (0..30)
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
                &|_: &str| Ok(()),
                Duration::from_millis(1),
            )
            .expect("loop ends");

            assert_eq!(
                dashboard.detail.sections.len(),
                2,
                "width {width}: two headings, two sections"
            );
            assert!(
                dashboard.detail.foldable(),
                "width {width}: two sections make the tab foldable"
            );

            let interior = if width == 60 { 58 } else { 78 };
            let rows = crate::ui::detail::content_lines(
                &dashboard.detail,
                dashboard.selected_change(),
                interior,
            );
            // bar + blank + two open headers + twenty items + one separator.
            assert_eq!(rows.len(), 25, "width {width}");
            assert_eq!(
                dashboard.detail.scroll,
                rows.len() - 1,
                "width {width}: the cursor rule clamps to the last line"
            );
            assert_ne!(dashboard.detail.scroll, 20, "width {width}");

            let markdown_len = crate::ui::markdown::lines(&source, interior).len();
            assert_ne!(
                dashboard.detail.scroll,
                markdown_len.saturating_sub(14),
                "width {width}: the clamp must differ from the markdown body's own"
            );

            let buf = terminal.backend().buffer();
            let base = if width == 60 { 1 } else { 42 };
            let row_at = |y: u16, len: usize| -> String {
                (base..base + len as u16)
                    .map(|x| row_text(buf, y).chars().nth(x as usize).unwrap())
                    .collect()
            };
            assert_eq!(
                row_at(18, 11),
                "[ ] line-19",
                "width {width}: the clamp used the body that was actually drawn"
            );
        }
    }

    /// `detail-scroll` :: "`j` walks the groups rather than scrolling the
    /// lines".
    #[test]
    fn j_walks_the_groups_rather_than_scrolling_the_lines() {
        for width in [120u16, 60] {
            let backend = TestBackend::new(width, 40);
            let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
            let mut dashboard = task_detail_dashboard(6, 6);
            let recorder =
                crate::testutil::RecordingReader::always(Ok(THREE_COMPLETE_GROUPS.to_string()));
            let read = |p: &std::path::Path| recorder.read(p);

            let interior = if width == 60 { 58 } else { 78 };
            let base = if width == 60 { 1u16 } else { 42 };
            let content_first_row = 5u16;

            // Stage one: two `j` presses over the collapsed groups.
            drive(&mut terminal, &mut dashboard, &read, &['j', 'j']);

            assert_eq!(
                dashboard.detail.expanded,
                std::collections::BTreeSet::new(),
                "width {width}: every subtree is complete, so the seed opened none"
            );
            assert_eq!(
                dashboard.detail.scroll, 2,
                "width {width}: rows 0 and 1 are the progress bar and its blank line"
            );
            {
                let rows = crate::ui::detail::content_lines(
                    &dashboard.detail,
                    dashboard.selected_change(),
                    interior,
                );
                assert_eq!(
                    rows.len(),
                    5,
                    "width {width}: three headers over six hidden items"
                );
                let buf = terminal.backend().buffer();
                assert_eq!(
                    cell(buf, base, content_first_row + 2).style(),
                    Cell::default().style().patch(crate::ui::palette::style(
                        crate::ui::palette::Role::DetailSectionSelected,
                    )),
                    "width {width}: the cursor's own header row is the emphasised one"
                );
                for y in content_first_row..content_first_row + 5 {
                    assert!(
                        !row_text(buf, y).contains("[✓]"),
                        "width {width}: no item line is drawn behind a collapsed header"
                    );
                }
            }

            // Stage two: `Space` opens the first group.
            drive(&mut terminal, &mut dashboard, &read, &[' ']);
            assert_eq!(dashboard.detail.scroll, 2, "width {width}");
            {
                let buf = terminal.backend().buffer();
                let row_at = |y: u16, len: usize| -> String {
                    (base..base + len as u16)
                        .map(|x| row_text(buf, y).chars().nth(x as usize).unwrap())
                        .collect()
                };
                assert_eq!(row_at(content_first_row + 3, 6), "[✓] a1", "width {width}");
                assert_eq!(row_at(content_first_row + 4, 6), "[✓] a2", "width {width}");
            }

            // Stage three: two further `j` presses.
            drive(&mut terminal, &mut dashboard, &read, &['j', 'j']);
            assert_eq!(
                dashboard.detail.scroll, 4,
                "width {width}: the cursor walks the RENDERED list"
            );
            let rows = crate::ui::detail::content_lines(
                &dashboard.detail,
                dashboard.selected_change(),
                interior,
            );
            assert_eq!(
                rows[4].text().trim_end(),
                "[✓] a2",
                "width {width}: row 4 is the first group's second item, not a section index"
            );
            assert!(
                matches!(
                    rows[6].kind,
                    crate::ui::detail::ContentKind::SectionHeader { section: 1, .. }
                ),
                "width {width}: the second group's header moved down with the rows drawn \
                 above it, which is what a rendered-list cursor means"
            );
        }
    }

    /// One `run_loop` stage: the scripted `keys` followed by `q`, over a
    /// terminal and dashboard that persist across stages. Extracted because
    /// `j_walks_the_groups_rather_than_scrolling_the_lines` has to observe
    /// the buffer between presses, which a single run cannot show.
    fn drive(
        terminal: &mut ratatui::Terminal<TestBackend>,
        dashboard: &mut Dashboard,
        read: &dyn Fn(&std::path::Path) -> Result<String, String>,
        keys: &[char],
    ) {
        dashboard.quit = false;
        let mut presses: Vec<_> = keys
            .iter()
            .map(|k| Ok(Some(press(KeyCode::Char(*k), KeyModifiers::NONE))))
            .collect();
        presses.push(Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))));
        let mut events = Script::new(presses);
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
            terminal,
            dashboard,
            &mut events,
            &mut live,
            read,
            &|_: &str| Ok(()),
            Duration::from_millis(1),
        )
        .expect("stage ends");
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
                &|_: &str| Ok(()),
                Duration::from_millis(1),
            )
            .expect("first stage ends");
            // 8, for the reason recorded in `checklist_scroll_is_clamped`:
            // the shared fixture carries no heading row.
            assert_eq!(dashboard.detail.scroll, 8, "width {width}: precondition");

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
                &|_: &str| Ok(()),
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
                &|_: &str| Ok(()),
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
            selection: None,
            help: crate::ui::app::Help {
                open: false,
                scroll: 0,
            },
            repo: base.repo,
            searched_from: base.searched_from,
            changes: base.changes,
            route: base.route,
            quit: base.quit,
            selected: base.selected,
            filter: base.filter,
            detail: crate::ui::app::Detail {
                sections: base.detail.sections,
                scroll: 4,
                tab: base.detail.tab,
                problems: base.detail.problems,
                loaded: base.detail.loaded,
                expanded: std::collections::BTreeSet::new(),
                drawn_width: None,
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
        let row: String = row_text(buf, 5).chars().skip(42).take(9).collect();
        assert_eq!(row, "• line-00");

        // A foldable dashboard whose `detail.scroll` is `2` over three
        // collapsed sections is `2` after all three areas below, because a
        // cursor does not move when the pane resizes.
        let mut foldable = Dashboard {
            selection: None,
            help: dashboard.help.clone(),
            repo: dashboard.repo.clone(),
            searched_from: dashboard.searched_from.clone(),
            changes: dashboard.changes.clone(),
            route: dashboard.route,
            quit: dashboard.quit,
            selected: dashboard.selected,
            filter: dashboard.filter.clone(),
            detail: crate::ui::app::Detail {
                sections: vec![
                    ArtifactSection {
                        label: Some("a".to_string()),
                        text: "one\n".to_string(),
                        depth: 0,
                        progress: None,
                        operation: None,
                    },
                    ArtifactSection {
                        label: Some("b".to_string()),
                        text: "two\n".to_string(),
                        depth: 0,
                        progress: None,
                        operation: None,
                    },
                    ArtifactSection {
                        label: Some("c".to_string()),
                        text: "three\n".to_string(),
                        depth: 0,
                        progress: None,
                        operation: None,
                    },
                ],
                scroll: 2,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
                expanded: std::collections::BTreeSet::new(),
                drawn_width: None,
            },
            refresh: dashboard.refresh.clone(),
            agents: dashboard.agents.clone(),
            agent_names: dashboard.agent_names.clone(),
            launch: dashboard.launch.clone(),
            sections: dashboard.sections.clone(),
            file_mode: dashboard.file_mode,
        };
        let backend2 = TestBackend::new(120, 20);
        let mut terminal2 = ratatui::Terminal::new(backend2).expect("construct terminal");
        let frame = terminal2
            .draw(|f| view::render(f, &foldable))
            .expect("draw foldable first frame");
        foldable.normalise_scroll(frame.area);
        assert_eq!(foldable.detail.scroll, 2);

        terminal2.backend_mut().resize(120, 40);
        let frame = terminal2
            .draw(|f| view::render(f, &foldable))
            .expect("draw foldable second frame");
        foldable.normalise_scroll(frame.area);
        assert_eq!(foldable.detail.scroll, 2);

        terminal2.backend_mut().resize(60, 20);
        let frame = terminal2
            .draw(|f| view::render(f, &foldable))
            .expect("draw foldable third frame");
        foldable.normalise_scroll(frame.area);
        assert_eq!(foldable.detail.scroll, 2);
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
            selection: None,
            help: crate::ui::app::Help {
                open: false,
                scroll: 0,
            },
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
                sections: vec![ArtifactSection {
                    label: Some(String::new()),
                    text: (0..20).map(|i| format!("- line-{i:02}\n")).collect(),
                    depth: 0,
                    progress: None,
                    operation: None,
                }],
                scroll: 3,
                tab: 0,
                problems: Vec::new(),
                loaded: Some((dir, 0)),
                expanded: std::collections::BTreeSet::new(),
                drawn_width: None,
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
            &|_: &str| Ok(()),
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
        // Compared field by field rather than as a whole `Detail`, because one
        // field legitimately differs: `normalise_scroll` records `drawn_width`
        // on every frame, so a loop that drew one sets it while `Enter` itself
        // does not (design.md -> Decision 13). Asserting it separately below
        // pins the recorder instead of hiding it behind a re-baselined clone.
        assert_eq!(
            dashboard.detail.sections, before.detail.sections,
            "Enter at the detail route must not touch the sections"
        );
        assert_eq!(
            dashboard.detail.scroll, before.detail.scroll,
            "Enter at the detail route must not reset the scroll"
        );
        assert_eq!(dashboard.detail.tab, before.detail.tab);
        assert_eq!(dashboard.detail.problems, before.detail.problems);
        assert_eq!(dashboard.detail.loaded, before.detail.loaded);
        assert_eq!(
            dashboard.detail.expanded, before.detail.expanded,
            "Enter at the detail route must not fold anything"
        );
        assert_eq!(
            dashboard.detail.drawn_width,
            Some(78),
            "the frame that was drawn recorded its own content width -- the wide              layout's mandated 78-column detail interior"
        );
        assert_eq!(dashboard.changes, before.changes);

        let buf = terminal.backend().buffer();
        let row: String = row_text(buf, 5).chars().skip(42).take(9).collect();
        assert_eq!(row, "• line-03");
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
            &|_: &str| Ok(()),
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
            selection: None,
            help: crate::ui::app::Help {
                open: false,
                scroll: 0,
            },
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
                sections: Vec::new(),
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
                expanded: std::collections::BTreeSet::new(),
                drawn_width: None,
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
            &|_: &str| Ok(()),
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
        let row: String = row_text(buf, 5).chars().skip(42).take(10).collect();
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
            &|_: &str| Ok(()),
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
                &|_: &str| Ok(()),
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
                &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
                &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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

    /// `specs/dashboard-loop/spec.md` -> "The overlay's scroll is clamped
    /// where the detail region's is not". At 60x20 on `Route::List`,
    /// `split_body` returns no detail region at all, so
    /// `normalise_scroll` returns early — this is the one fixture that
    /// proves `normalise_help_scroll` is a genuinely separate call, not
    /// folded into a `normalise_scroll` that would have left `help.scroll`
    /// unclamped here (design.md -> Decision 10).
    #[test]
    fn the_overlay_s_scroll_is_clamped_where_the_detail_region_s_is_not() {
        let base = dashboard();
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = base.clone();
        dashboard.help = crate::ui::app::Help {
            open: true,
            scroll: 99,
        };
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
        run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
            &|_: &std::path::Path| Ok(String::new()),
            &|_: &str| Ok(()),
            Duration::from_millis(1),
        )
        .expect("loop ends");

        assert_eq!(
            dashboard.help.scroll, 26,
            "43 content rows less a 17-row interior"
        );
        assert_eq!(
            dashboard.detail.scroll, 0,
            "normalise_scroll still changes nothing when the detail region is not drawn"
        );

        // The same dashboard with the overlay closed leaves `help.scroll`
        // untouched: `normalise_help_scroll` clamps nothing while it is
        // false.
        let backend2 = TestBackend::new(60, 20);
        let mut terminal2 = ratatui::Terminal::new(backend2).expect("construct terminal");
        let mut closed = base;
        closed.help = crate::ui::app::Help {
            open: false,
            scroll: 99,
        };
        let mut events2 = Script::new(vec![Ok(Some(press(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
        )))]);
        let mut fs2 = ScriptedFs::new(Vec::new(), Vec::new());
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
            &mut terminal2,
            &mut closed,
            &mut events2,
            &mut live2,
            &|_: &std::path::Path| Ok(String::new()),
            &|_: &str| Ok(()),
            Duration::from_millis(1),
        )
        .expect("loop ends");
        assert_eq!(closed.help.scroll, 99, "a closed overlay is not clamped");
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
            &|_: &str| Ok(()),
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
                &|_: &str| Ok(()),
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
                    &|_: &str| Ok(()),
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
                &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
                &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
                &|_: &str| Ok(()),
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
                &|_: &str| Ok(()),
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
                &|_: &str| Ok(()),
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
                &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
                &|_: &str| Ok(()),
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
            selection: None,
            help: crate::ui::app::Help {
                open: false,
                scroll: 0,
            },
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
                sections: Vec::new(),
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
                expanded: std::collections::BTreeSet::new(),
                drawn_width: None,
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
            &|_: &str| Ok(()),
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

    use crate::ui::app::{Action, Granularity, SectionKey, SelectPhase, Target, action_for};
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

    fn drag(column: u16, row: u16) -> MouseEvent {
        m(MouseEventKind::Drag(MouseButton::Left), column, row)
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
        let (body, _) = crate::ui::layout::split_frame(area);
        crate::ui::layout::interior(
            crate::ui::layout::split_body(body, route)
                .0
                .expect("a list region is drawn"),
            crate::ui::layout::Gutters::Both,
        )
    }

    /// The detail region's tab-bar row for `area` at `route`.
    fn tab_bar_row(area: Rect, route: Route) -> Rect {
        let (body, _) = crate::ui::layout::split_frame(area);
        let (_, divider, detail_area) = crate::ui::layout::split_body(body, route);
        crate::ui::layout::split_detail(crate::ui::layout::interior(
            detail_area.expect("a detail region is drawn"),
            crate::ui::layout::detail_gutters(divider),
        ))
        .0
    }

    /// The terminal row the list interior's `offset`-th drawn row occupies.
    fn list_row(area: Rect, route: Route, offset: u16) -> u16 {
        list_interior(area, route).y + offset
    }

    /// The detail region's content area for `area` at `route` —
    /// `foldable-spec-sections`' addition, `split_detail`'s third rectangle
    /// rather than its first (`tab_bar_row`'s sibling).
    fn detail_content_area(area: Rect, route: Route) -> Rect {
        let (body, _) = crate::ui::layout::split_frame(area);
        let (_, divider, detail_area) = crate::ui::layout::split_body(body, route);
        crate::ui::layout::split_detail(crate::ui::layout::interior(
            detail_area.expect("a detail region is drawn"),
            crate::ui::layout::detail_gutters(divider),
        ))
        .2
    }

    /// A `Route::Detail` dashboard whose selected artifact resolves to
    /// three sections, mirroring `ui::app::tests::foldable_dashboard` /
    /// `ui::detail::tests::three_spec_detail` — a reader who has seen either
    /// recognises this one. `sections` is passed in rather than fixed, so a
    /// scenario that needs a multi-line body can supply one.
    fn foldable_dashboard(
        sections: Vec<ArtifactSection>,
        expanded: std::collections::BTreeSet<usize>,
        scroll: usize,
    ) -> Dashboard {
        let change = crate::changes::fixture::with_artifacts(
            crate::changes::fixture::active("s0", 0, 1),
            &[],
        );
        let mut d = dashboard_with_change("/repo", "unused", 0, 0);
        d.changes = crate::changes::fixture::set(vec![change], Vec::new(), Vec::new());
        d.selected = 1;
        d.route = Route::Detail;
        d.detail = crate::ui::app::Detail {
            sections,
            scroll,
            tab: 0,
            problems: Vec::new(),
            loaded: None,
            expanded,
            drawn_width: Some(78),
        };
        d
    }

    /// The three short, non-wrapping sections `ui::app::tests::three_spec_detail`
    /// and `ui::detail::tests::three_spec_detail` already fix for the same shape.
    fn three_spec_sections() -> Vec<ArtifactSection> {
        vec![
            ArtifactSection {
                label: Some("degraded-coverage".to_string()),
                text: "one\n".to_string(),
                depth: 0,
                progress: None,
                operation: None,
            },
            ArtifactSection {
                label: Some("markdown-render".to_string()),
                text: "two\n".to_string(),
                depth: 0,
                progress: None,
                operation: None,
            },
            ArtifactSection {
                label: Some("tasks-checklist".to_string()),
                text: "three\n".to_string(),
                depth: 0,
                progress: None,
                operation: None,
            },
        ]
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
            dashboard.detail.sections = vec![ArtifactSection {
                label: Some(String::new()),
                text: (0..20).map(|i| format!("- line-{i:02}\n")).collect(),
                depth: 0,
                progress: None,
                operation: None,
            }];

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
    fn the_wheel_acts_over_a_border_and_not_over_the_chrome() {
        // `mouse-input`: "The wheel acts over a border and not over the chrome".
        let dashboard = mouse_dashboard(3, 0);
        for (column, row, label) in [
            (0u16, 1u16, "the list region's own left gutter column"),
            (10, 0, "the list region's own heading row"),
        ] {
            assert_eq!(
                mouse_action(&dashboard, WIDE, &m(MouseEventKind::ScrollUp, column, row)),
                Action::SelectPrev,
                "{label} at ({column}, {row})"
            );
        }
        for (column, row) in [(10u16, 39u16), (200, 10)] {
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
        // `list-selection`: `ToggleSection` is route-dependent since group 7 —
        // pinned here, since `mouse_dashboard` already builds at `Route::List`.
        assert_eq!(spaced.route, Route::List);
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

        // `foldable-spec-sections`: a press below the last drawn line of a
        // foldable artifact — three collapsed sections draw three rows, so
        // an empty region under them is not a fourth section.
        let folded =
            foldable_dashboard(three_spec_sections(), std::collections::BTreeSet::new(), 0);
        let content = detail_content_area(WIDE, Route::Detail);
        let past_last_row = content.y + 3;
        let action = mouse_action(&folded, WIDE, &left(content.x, past_last_row));
        assert_eq!(action, Action::Ignore, "past the last drawn content row");
        let mut applied = folded.clone();
        applied.apply(action);
        assert_eq!(applied, folded, "changed the dashboard");
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

        // `foldable-spec-sections`: the same five kinds over a drawn
        // artifact-section header row — task 8.2's extension.
        let folded =
            foldable_dashboard(three_spec_sections(), std::collections::BTreeSet::new(), 0);
        let content = detail_content_area(WIDE, Route::Detail);
        let header_row = content.y + 1;
        for kind in [
            MouseEventKind::Down(MouseButton::Right),
            MouseEventKind::Down(MouseButton::Middle),
            MouseEventKind::Up(MouseButton::Left),
            MouseEventKind::Drag(MouseButton::Left),
            MouseEventKind::Moved,
        ] {
            let action = mouse_action(&folded, WIDE, &m(kind, content.x, header_row));
            assert_eq!(action, Action::Ignore, "{kind:?} over a header row");
            assert_ne!(action, Action::LaunchApply);
            assert_ne!(action, Action::LaunchContinue);
            assert_ne!(action, Action::LaunchArchive);
            assert_ne!(action, Action::FocusAgent);
        }
    }

    #[test]
    fn a_detail_header_click_equals_space() {
        // `mouse-input`: "A click on an artifact-section header folds it
        // exactly as `Space` does".
        for (width, height) in [(120u16, 40u16), (60, 40)] {
            let area = Rect::new(0, 0, width, height);
            let content = detail_content_area(area, Route::Detail);
            // Every section starts collapsed, so the content rows are the
            // three headers in order; the second content row is the second
            // header.
            let row = content.y + 1;

            let mut clicked =
                foldable_dashboard(three_spec_sections(), std::collections::BTreeSet::new(), 0);
            let action = mouse_action(&clicked, area, &left(content.x, row));
            assert_eq!(
                action,
                Action::Click(Target::DetailHeader {
                    line: 1,
                    section: 1
                }),
                "{width}x{height}"
            );

            clicked.apply(action);
            assert!(
                clicked.detail.expanded.contains(&1),
                "{width}x{height}: opened"
            );
            assert_eq!(
                clicked.detail.scroll, 1,
                "{width}x{height}: cursor on the header"
            );

            // Equal, field for field, to `Space` at `Route::Detail` with the
            // cursor already on that header.
            let mut spaced =
                foldable_dashboard(three_spec_sections(), std::collections::BTreeSet::new(), 1);
            spaced.apply(Action::ToggleSection);
            assert_eq!(clicked, spaced, "{width}x{height}: field for field");

            // A second click folds it again.
            let unfold = mouse_action(&clicked, area, &left(content.x, row));
            clicked.apply(unfold);
            assert!(
                !clicked.detail.expanded.contains(&1),
                "{width}x{height}: folded again"
            );
        }

        // The same press at `Route::List` on the 120-column frame — where
        // the wide layout draws both regions — returns and applies the same
        // action, so the detail region's headers are clickable at either
        // route.
        let content = detail_content_area(WIDE, Route::List);
        let row = content.y + 1;
        let mut at_list =
            foldable_dashboard(three_spec_sections(), std::collections::BTreeSet::new(), 0);
        at_list.route = Route::List;
        let at_detail =
            foldable_dashboard(three_spec_sections(), std::collections::BTreeSet::new(), 0);
        let action_at_list = mouse_action(&at_list, WIDE, &left(content.x, row));
        let action_at_detail = mouse_action(&at_detail, WIDE, &left(content.x, row));
        assert_eq!(action_at_list, action_at_detail);
        at_list.apply(action_at_list);
        assert!(at_list.detail.expanded.contains(&1));
    }

    #[test]
    fn a_body_click_arms_a_selection_and_folds_nothing() {
        // `mouse-input`: "A click in an open section's body arms a selection
        // and folds nothing".
        let sections = vec![
            ArtifactSection {
                label: Some("degraded-coverage".to_string()),
                text: "one\n".to_string(),
                depth: 0,
                progress: None,
                operation: None,
            },
            ArtifactSection {
                label: Some("markdown-render".to_string()),
                // A bullet list, not a five-line paragraph: a soft break
                // folds into the paragraph, so only a list keeps five
                // rendered body lines for the click to land in.
                text: (0..5).map(|i| format!("- line-{i:02}\n")).collect(),
                depth: 0,
                progress: None,
                operation: None,
            },
            ArtifactSection {
                label: Some("tasks-checklist".to_string()),
                text: "three\n".to_string(),
                depth: 0,
                progress: None,
                operation: None,
            },
        ];
        for (width, height) in [(120u16, 40u16), (60, 40)] {
            let area = Rect::new(0, 0, width, height);
            let content = detail_content_area(area, Route::Detail);

            let mut dashboard =
                foldable_dashboard(sections.clone(), std::collections::BTreeSet::from([1]), 0);
            // Rows: 0 = header 0 (collapsed), 1 = header 1 (open), 2..7 =
            // "line-00".."line-04", 7 = header 2 (collapsed). The third
            // rendered body line, "line-02", is content-line index 4.
            let row = content.y + 4;

            let action = mouse_action(&dashboard, area, &left(content.x, row));
            assert_eq!(
                action,
                Action::Select(SelectPhase::Begin { line: 4, column: 0 }),
                "{width}x{height}: arms rather than moving the detail cursor"
            );

            let before = dashboard.clone();
            dashboard.apply(action);
            assert_eq!(
                dashboard.detail.scroll, before.detail.scroll,
                "{width}x{height}: the cursor no longer jumps to the clicked line"
            );
            assert_eq!(
                dashboard.detail.expanded, before.detail.expanded,
                "{width}x{height}: nothing folded"
            );
            assert_eq!(dashboard.route, before.route, "{width}x{height}");
            assert_eq!(dashboard.selected, before.selected, "{width}x{height}");
            assert_eq!(dashboard.detail.tab, before.detail.tab, "{width}x{height}");
            assert_eq!(
                dashboard.selection,
                Some(crate::ui::app::Selection {
                    anchor: (4, 0),
                    focus: (4, 0),
                    granularity: crate::ui::app::Granularity::Armed,
                    problem: None,
                }),
                "{width}x{height}: a first press arms, drawing nothing"
            );

            // A second press at that same cell selects the word there, per
            // `text-selection`.
            let second = mouse_action(&dashboard, area, &left(content.x, row));
            dashboard.apply(second);
            assert_eq!(
                dashboard.selection.as_ref().map(|s| s.granularity),
                Some(crate::ui::app::Granularity::Word),
                "{width}x{height}: the second press widens to the word"
            );
        }
    }

    #[test]
    fn a_non_foldable_tabs_content_is_selectable_too() {
        // `mouse-input`/`text-selection`: "A non-foldable tab's content is
        // selectable too" — a deliberate departure from the removed
        // `detail_row_click`'s `!foldable()` short-circuit (design.md ->
        // Decision 8): a single-section artifact is a whole rendered
        // document and exactly the thing a reader wants to copy out of.
        let sections = vec![ArtifactSection {
            label: Some(String::new()),
            text: (0..20).map(|i| format!("- line-{i:02}\n")).collect(),
            depth: 0,
            progress: None,
            operation: None,
        }];
        for (width, height) in [(120u16, 40u16), (60, 40)] {
            let area = Rect::new(0, 0, width, height);
            let dashboard =
                foldable_dashboard(sections.clone(), std::collections::BTreeSet::new(), 0);
            assert!(
                !dashboard.detail.foldable(),
                "{width}x{height}: one section"
            );

            let content = detail_content_area(area, Route::Detail);
            // The first, fifth, and last drawn content rows — the twenty-item
            // list fits entirely within either mandated height.
            for line in [0usize, 4, 19] {
                let row = content.y + line as u16;
                let action = mouse_action(&dashboard, area, &left(content.x, row));
                assert_eq!(
                    action,
                    Action::Select(SelectPhase::Begin { line, column: 0 }),
                    "{width}x{height} row {row}: arms, not `Action::Ignore`"
                );
                let mut applied = dashboard.clone();
                applied.apply(action);
                let second = mouse_action(&applied, area, &left(content.x, row));
                applied.apply(second);
                assert_eq!(
                    applied.selection.as_ref().map(|s| s.granularity),
                    Some(crate::ui::app::Granularity::Word),
                    "{width}x{height} row {row}: a second press selects the word"
                );
            }
        }
    }

    /// `mouse-input`: "A click on a task group's header folds that group",
    /// verbatim — two groups, the first complete and the second not.
    const TWO_TASK_GROUPS: &str = "## 1. Done\n\n- [x] a\n\n## 2. Doing\n\n- [ ] b\n";

    /// The same file's items with no heading at all: it does not split, so the
    /// tab carries one section and is not foldable.
    const HEADLESS_TASKS: &str = "- [x] a\n- [ ] b\n";

    /// A `Route::Detail` dashboard over a `tracks_tasks` artifact whose
    /// sections — and whose fold seed — `sync_detail` derives from `source`,
    /// with `drawn_width` set to the interior the frame will hand
    /// `content_lines`. Built through `sync_detail` rather than by writing
    /// sections out here, so the seed a click lands on is the production one.
    fn task_source_dashboard(source: &str, width: u16) -> Dashboard {
        let mut dashboard = task_detail_dashboard(1, 2);
        let recorder = crate::testutil::RecordingReader::always(Ok(source.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);
        dashboard.sync_detail(&read);
        dashboard.detail.drawn_width = Some(width);
        dashboard
    }

    #[test]
    fn a_click_on_a_task_groups_header_folds_that_group() {
        // `mouse-input`: "A click on a task group's header folds that group".
        for (width, height) in [(120u16, 40u16), (60, 40)] {
            let area = Rect::new(0, 0, width, height);
            let content = detail_content_area(area, Route::Detail);
            let dashboard = task_source_dashboard(TWO_TASK_GROUPS, content.width);
            // Rows: the progress bar (0), its blank line (1), `> 1. Done` (2),
            // `v 2. Doing` (3), and that group's one item (4). The seed opened
            // the incomplete group and left the finished one shut.
            assert_eq!(
                dashboard.detail.expanded,
                std::collections::BTreeSet::from([1]),
                "{width}x{height}: the seed"
            );

            let action = mouse_action(&dashboard, area, &left(content.x, content.y + 2));
            assert_eq!(
                action,
                Action::Click(Target::DetailHeader {
                    line: 2,
                    section: 0
                }),
                "{width}x{height}: the `1. Done` header row"
            );

            let mut opened = dashboard.clone();
            let before = opened.clone();
            opened.apply(action);
            assert!(
                opened.detail.expanded.contains(&0),
                "{width}x{height}: the group opened"
            );
            assert_eq!(
                opened.detail.scroll, 2,
                "{width}x{height}: the cursor is on that group's header row"
            );
            assert_eq!(opened.route, before.route, "{width}x{height}");
            assert_eq!(opened.selected, before.selected, "{width}x{height}");
            assert_eq!(opened.detail.tab, before.detail.tab, "{width}x{height}");

            // The progress-bar row and its blank line, tested from the
            // untouched `dashboard` fixture rather than `opened`: neither
            // is a header row and neither belongs to a section, so a press
            // there arms a selection and folds nothing — replacing the
            // removed detail-line target, which used to move the detail
            // cursor onto the row instead. Because arming does not move
            // `detail.scroll`, the cursor is exactly where it was before the
            // press, which is what keeps `Space` from there inert too.
            for row in 0..2u16 {
                let action = mouse_action(&dashboard, area, &left(content.x, content.y + row));
                assert_eq!(
                    action,
                    Action::Select(SelectPhase::Begin {
                        line: row as usize,
                        column: 0
                    }),
                    "{width}x{height}: preamble row {row} arms a selection"
                );
                let mut moved = dashboard.clone();
                moved.apply(action);
                assert_eq!(
                    moved.detail.expanded, dashboard.detail.expanded,
                    "{width}x{height}: preamble row {row} folded nothing"
                );
                assert_eq!(
                    moved.detail.scroll, dashboard.detail.scroll,
                    "{width}x{height}: preamble row {row} moved no cursor"
                );
                moved.apply(Action::ToggleSection);
                assert_eq!(
                    moved.detail.expanded, dashboard.detail.expanded,
                    "{width}x{height}: and `Space` from there is inert too"
                );
            }

            // A task file holding items but no heading does not split, so the
            // tab is not foldable — but, per design.md -> Decision 8 and
            // `specs/mouse-input/spec.md`'s own "A non-foldable tab's content
            // is selectable too", a non-foldable artifact's content is still
            // selectable rather than inert. (This departs from that same
            // spec's "A click on a task group's header folds that group"
            // scenario, whose own headless clause still reads
            // `Action::Ignore` — a residual inconsistency between the two
            // scenarios that a follow-up documentation pass should reconcile;
            // flagged in this change's own report rather than silently
            // picking one spec sentence over the other.)
            let headless = task_source_dashboard(HEADLESS_TASKS, content.width);
            assert!(
                !headless.detail.foldable(),
                "{width}x{height}: one section, so not foldable"
            );
            for row in 0..2u16 {
                assert_eq!(
                    mouse_action(&headless, area, &left(content.x, content.y + row)),
                    Action::Select(SelectPhase::Begin {
                        line: row as usize,
                        column: 0
                    }),
                    "{width}x{height}: headless row {row} is selectable, not `Ignore`"
                );
            }
        }
    }

    /// `specs/artifact-folds/spec.md`'s delta-spec fixture, verbatim: one
    /// operation heading, a requirement carrying a nested scenario, and a
    /// second requirement.
    const DELTA_SPEC: &str = "## ADDED Requirements\n\n### Requirement: Alpha\nAlpha text.\n\n#### Scenario: A works\n- **WHEN** a\n- **THEN** b\n\n### Requirement: Beta\nBeta text.\n";

    /// A `Route::Detail` dashboard whose one selected change carries a single
    /// `specs` artifact resolving to **one** delta-spec path, its sections
    /// derived from `source` by `sync_detail` and its folds set afterwards —
    /// `sync_detail` seeds none on a tab that tracks no tasks.
    fn spec_source_dashboard(
        source: &str,
        expanded: std::collections::BTreeSet<usize>,
        width: u16,
    ) -> Dashboard {
        let change = crate::changes::fixture::with_artifacts(
            crate::changes::fixture::active("detail-view", 4, 9),
            &[("specs", &["/repo/openspec/changes/c/specs/a/spec.md"])],
        );
        let mut dashboard = dashboard_with_change("/repo", "unused", 0, 0);
        dashboard.changes = crate::changes::fixture::set(vec![change], Vec::new(), Vec::new());
        dashboard.selected = 1;
        dashboard.route = Route::Detail;
        let recorder = crate::testutil::RecordingReader::always(Ok(source.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);
        dashboard.sync_detail(&read);
        dashboard.detail.expanded = expanded;
        dashboard.detail.drawn_width = Some(width);
        dashboard
    }

    #[test]
    fn a_click_on_a_nested_scenario_header_folds_only_that_scenario() {
        // `mouse-input`: "A click on a nested scenario header folds only that
        // scenario".
        for (width, height) in [(120u16, 40u16), (60, 40)] {
            let area = Rect::new(0, 0, width, height);
            let content = detail_content_area(area, Route::Detail);
            // The operation heading and its first requirement open; the
            // scenario nested under that requirement, and the second
            // requirement, shut.
            let mut dashboard = spec_source_dashboard(
                DELTA_SPEC,
                std::collections::BTreeSet::from([0, 1]),
                content.width,
            );
            assert_eq!(
                dashboard
                    .detail
                    .sections
                    .iter()
                    .map(|s| (s.label.as_deref(), s.depth))
                    .collect::<Vec<_>>(),
                vec![
                    (Some("ADDED Requirements"), 0),
                    (Some("Requirement: Alpha"), 1),
                    (Some("Scenario: A works"), 2),
                    (Some("Requirement: Beta"), 1),
                ],
                "{width}x{height}: the delta spec's own four sections"
            );

            // Rows: header 0, header 1, `Alpha text.`, the blank separator,
            // the scenario's own header, header 3. The scenario is the fifth
            // drawn row and the **third** section — which is the distinction
            // this scenario exists to hold.
            let scenario_row = content.y + 4;
            let action = mouse_action(&dashboard, area, &left(content.x, scenario_row));
            assert_eq!(
                action,
                Action::Click(Target::DetailHeader {
                    line: 4,
                    section: 2
                }),
                "{width}x{height}: its index into `detail.sections`, not its drawn position"
            );

            let before = dashboard.clone();
            dashboard.apply(action);
            assert_eq!(
                dashboard.detail.expanded,
                std::collections::BTreeSet::from([0, 1, 2]),
                "{width}x{height}: only that scenario's membership changed"
            );
            assert_eq!(dashboard.route, before.route, "{width}x{height}");
            assert_eq!(dashboard.selected, before.selected, "{width}x{height}");
            assert_eq!(dashboard.detail.tab, before.detail.tab, "{width}x{height}");

            // A press on one of that scenario's own body rows arms a
            // selection, folds nothing, and moves no cursor.
            let rows = crate::ui::detail::content_lines(
                &dashboard.detail,
                dashboard.selected_change(),
                content.width,
            );
            let body = rows
                .iter()
                .enumerate()
                .skip(5)
                .find_map(|(index, r)| {
                    matches!(r.kind, crate::ui::detail::ContentKind::Body).then_some(index)
                })
                .expect("the opened scenario draws its own body below its header");
            assert_eq!(
                body, 5,
                "{width}x{height}: the row immediately below the scenario's own header"
            );
            let action = mouse_action(&dashboard, area, &left(content.x, content.y + body as u16));
            assert_eq!(
                action,
                Action::Select(SelectPhase::Begin {
                    line: body,
                    column: 0
                }),
                "{width}x{height}: a body row arms a selection at its own arming phase"
            );
            let folds = dashboard.detail.expanded.clone();
            let scroll_before = dashboard.detail.scroll;
            dashboard.apply(action);
            assert_eq!(
                dashboard.detail.scroll, scroll_before,
                "{width}x{height}: moves no cursor"
            );
            assert_eq!(
                dashboard.detail.expanded, folds,
                "{width}x{height}: and folds nothing"
            );
        }
    }

    // ------------------------------------------------------------------
    // `text-selection`: the drag resolver and the press-count state machine.
    // ------------------------------------------------------------------

    #[test]
    fn a_drag_begins_only_in_the_detail_content_area() {
        // `text-selection`: "A drag begins only in the detail content area".
        let dashboard = foldable_dashboard(
            three_spec_sections(),
            std::collections::BTreeSet::from([1]),
            0,
        );
        let area = WIDE;
        let content = detail_content_area(area, Route::Detail);
        let bar = tab_bar_row(area, Route::Detail);

        // One point in each of the five zones a drag must ignore: a list row,
        // the list region's own left gutter (`Zone::List`), the tab-bar row
        // (`Zone::DetailTab`), a header row of the content area (a
        // `Zone::DetailRow`, but a header one), the content padding row below
        // the tab bar (`Zone::Detail`), and outside the frame entirely.
        let ignored = [
            (5u16, list_row(area, Route::Detail, 0)),
            (0u16, 1u16),
            (bar.x, bar.y),
            (content.x, content.y + 1),
            (content.x, content.y - 1),
            (200u16, 10u16),
        ];
        for (column, row) in ignored {
            assert_eq!(
                mouse_action(&dashboard, area, &drag(column, row)),
                Action::Ignore,
                "({column}, {row}) must not begin a selection"
            );
        }

        // The sixth zone — a non-header `Zone::DetailRow`, the open section's
        // own body row — is the only one that begins a selection.
        let (column, row) = (content.x, content.y + 2);
        assert!(
            matches!(
                mouse_action(&dashboard, area, &drag(column, row)),
                Action::Select(SelectPhase::Extend { .. })
            ),
            "the detail content area's own body row begins a selection"
        );
    }

    #[test]
    fn a_section_header_stays_clickable_and_is_never_selectable() {
        // `text-selection`: "A section header stays clickable and is never
        // selectable".
        let dashboard = foldable_dashboard(
            three_spec_sections(),
            std::collections::BTreeSet::from([1]),
            0,
        );
        let area = WIDE;
        let content = detail_content_area(area, Route::Detail);
        let header_row = content.y + 1;

        let pressed = mouse_action(&dashboard, area, &left(content.x, header_row));
        assert_eq!(
            pressed,
            Action::Click(Target::DetailHeader {
                line: 1,
                section: 1
            }),
            "a header row is still clickable, unchanged"
        );

        let dragged = mouse_action(&dashboard, area, &drag(content.x, header_row));
        assert_eq!(
            dragged,
            Action::Ignore,
            "folding a section can never begin a selection"
        );
    }

    #[test]
    fn a_drag_past_the_edge_clamps_and_does_not_scroll() {
        // `text-selection`: "A drag past the edge clamps and does not
        // scroll" — a drag that is extending a selection already in
        // progress and leaves the content area, vertically or
        // horizontally, follows the pointer to that area's nearest edge
        // rather than freezing at its last in-area position or being
        // dropped. Driven as a **coarse jump** straight to a point well
        // outside the frame, deliberately — the freeze this fix corrects
        // is invisible under a per-cell walk, since a one-cell step across
        // the boundary looks the same clamped or frozen.
        let dashboard_with_selection = |anchor_line: usize| {
            let mut d = foldable_dashboard(
                three_spec_sections(),
                std::collections::BTreeSet::from([1]),
                0,
            );
            d.selection = Some(crate::ui::app::Selection {
                anchor: (anchor_line, 0),
                focus: (anchor_line, 0),
                granularity: Granularity::Span,
                problem: None,
            });
            d
        };
        let area = WIDE;
        let content = detail_content_area(area, Route::Detail);

        // Above the area's first row: a coarse jump to the frame's own
        // top-left corner, well above `content.y`.
        let above = dashboard_with_selection(2);
        let action = mouse_action(&above, area, &drag(content.x, 0));
        assert_eq!(
            action,
            Action::Select(SelectPhase::Extend { line: 0, column: 0 }),
            "clamps to the content area's own first drawn line"
        );
        let mut applied = above.clone();
        applied.apply(action);
        assert_eq!(
            applied.detail.scroll, above.detail.scroll,
            "clamping never scrolls — the content did not move"
        );
        assert_eq!(
            applied.selection.map(|s| s.focus),
            Some((0, 0)),
            "the focus itself is the clamped cell, not a frozen last-in-area one"
        );

        // Below the area's last row: a coarse jump far past the bottom of
        // the frame entirely — `three_spec_sections()` with section 1 open
        // draws five rows (indices 0-4: its header, section 1's header and
        // open body, the blank separator before the next header, and
        // section 2's header), far short of the content area's own height
        // at 120x40, so the clamp lands on the document's own last drawn
        // line, not on the content rectangle's geometric last row.
        let below = dashboard_with_selection(0);
        let action = mouse_action(&below, area, &drag(content.x, 1000));
        assert_eq!(
            action,
            Action::Select(SelectPhase::Extend { line: 4, column: 0 }),
            "clamps to the content area's own last drawn line"
        );
        let mut applied = below.clone();
        applied.apply(action);
        assert_eq!(
            applied.detail.scroll, below.detail.scroll,
            "clamping never scrolls — the content did not move"
        );

        // Past the right edge: a coarse jump far past the frame's own
        // width, at a row that is otherwise a valid content row.
        let right = dashboard_with_selection(1);
        let action = mouse_action(&right, area, &drag(1000, content.y + 1));
        assert_eq!(
            action,
            Action::Select(SelectPhase::Extend {
                line: 1,
                column: content.width - 1
            }),
            "clamps to the content area's own rightmost column"
        );

        // A drag that never began a selection is untouched: the same
        // out-of-area points still resolve to `Action::Ignore` when
        // nothing is in progress, exactly as
        // `a_drag_begins_only_in_the_detail_content_area` already
        // requires — named here again because it is this fix's own
        // boundary, not only that scenario's.
        let mut untouched = foldable_dashboard(
            three_spec_sections(),
            std::collections::BTreeSet::from([1]),
            0,
        );
        untouched.selection = None;
        assert_eq!(
            mouse_action(&untouched, area, &drag(content.x, 0)),
            Action::Ignore,
            "no selection in progress: the edge clamp does not apply"
        );
        assert_eq!(
            mouse_action(&untouched, area, &drag(content.x, 1000)),
            Action::Ignore,
            "no selection in progress: the edge clamp does not apply"
        );
    }

    #[test]
    fn anchor_holds_while_the_focus_follows() {
        // `text-selection`: "Anchor holds while the focus follows" — a drag
        // begun at one cell, extended to a second, and then to a third: the
        // anchor is the first cell throughout and the focus is whichever cell
        // came last, and the span the pair selects does not depend on which
        // of the two came first.
        let lines: String = (0..7)
            .map(|i| format!("- line{i} {}\n", "x".repeat(45)))
            .collect();
        let dashboard = word_dashboard(&lines);
        let area = WIDE;
        let content = detail_content_area(area, Route::Detail);
        let start = (content.x + 10, content.y + 4);
        let mid = (content.x + 2, content.y + 6);
        let end = (content.x + 30, content.y + 5);

        let mut d = dashboard.clone();
        let begin = mouse_action(&d, area, &left(start.0, start.1));
        d.apply(begin);
        assert_eq!(d.selection.as_ref().map(|s| s.anchor), Some((4, 10)));
        assert_eq!(d.selection.as_ref().map(|s| s.focus), Some((4, 10)));

        let extend_down = mouse_action(&d, area, &drag(mid.0, mid.1));
        d.apply(extend_down);
        assert_eq!(
            d.selection.as_ref().map(|s| s.anchor),
            Some((4, 10)),
            "the anchor does not move while the focus follows"
        );
        assert_eq!(d.selection.as_ref().map(|s| s.focus), Some((6, 2)));

        let extend_up = mouse_action(&d, area, &drag(end.0, end.1));
        d.apply(extend_up);
        assert_eq!(
            d.selection.as_ref().map(|s| s.anchor),
            Some((4, 10)),
            "the anchor still has not moved, even after the drag reversed direction"
        );
        assert_eq!(d.selection.as_ref().map(|s| s.focus), Some((5, 30)));

        // The selected span is computed from the pair in either order, so
        // dragging upward across two points selects the same text as
        // dragging downward across them.
        let rows = crate::ui::detail::content_lines(
            &d.detail,
            d.selected_change(),
            d.detail.drawn_width.expect("a frame has been drawn"),
        );
        let selection = d.selection.clone().expect("a selection is in progress");
        let forward = crate::ui::detail::span_text(&rows, selection.anchor, selection.focus);
        let backward = crate::ui::detail::span_text(&rows, selection.focus, selection.anchor);
        assert_eq!(
            forward, backward,
            "the pair is order-independent regardless of which point is the anchor"
        );
    }

    /// A dashboard whose one selected, non-foldable artifact renders `text`
    /// verbatim as a bullet list — one word per line, so a press over a
    /// known column lands on a known word.
    fn word_dashboard(text: &str) -> Dashboard {
        foldable_dashboard(
            vec![ArtifactSection {
                label: Some(String::new()),
                text: text.to_string(),
                depth: 0,
                progress: None,
                operation: None,
            }],
            std::collections::BTreeSet::new(),
            0,
        )
    }

    #[test]
    fn one_press_arms_two_select_a_word_three_select_the_row() {
        // `text-selection`: "One press arms, two select a word, three select
        // the row".
        let dashboard = word_dashboard("- zone call\n");
        let area = WIDE;
        let content = detail_content_area(area, Route::Detail);
        // The rendered row is `• zone call`; column 2 is inside `zone`.
        let point = (content.x + 2, content.y);

        let mut d = dashboard.clone();
        let press = |d: &Dashboard| mouse_action(d, area, &left(point.0, point.1));

        let a1 = press(&d);
        assert!(
            matches!(a1, Action::Select(SelectPhase::Begin { .. })),
            "{a1:?}"
        );
        d.apply(a1);
        assert_eq!(
            d.selection.as_ref().map(|s| s.granularity),
            Some(Granularity::Armed),
            "first press arms"
        );
        assert_eq!(d.selection.as_ref().map(|s| s.anchor), Some((0, 2)));
        assert_eq!(d.selection.as_ref().map(|s| s.focus), Some((0, 2)));

        let a2 = press(&d);
        d.apply(a2);
        assert_eq!(
            d.selection.as_ref().map(|s| s.granularity),
            Some(Granularity::Word),
            "second press widens to the word"
        );

        let a3 = press(&d);
        d.apply(a3);
        assert_eq!(
            d.selection.as_ref().map(|s| s.granularity),
            Some(Granularity::Row),
            "third press widens to the row"
        );
        let after_third = d.selection.clone();

        let a4 = press(&d);
        d.apply(a4);
        assert_eq!(
            d.selection, after_third,
            "fourth press changes nothing further"
        );
    }

    #[test]
    fn a_press_elsewhere_restarts_the_count() {
        // `text-selection`: "A press elsewhere restarts the count".
        let dashboard = word_dashboard("- zone call\n- other line\n");
        let area = WIDE;
        let content = detail_content_area(area, Route::Detail);
        let first_cell = (content.x + 2, content.y);
        let second_cell = (content.x + 2, content.y + 1);

        let mut d = dashboard.clone();
        for _ in 0..2 {
            let action = mouse_action(&d, area, &left(first_cell.0, first_cell.1));
            d.apply(action);
        }
        assert_eq!(
            d.selection.as_ref().map(|s| s.granularity),
            Some(Granularity::Word),
            "two presses at the first cell select the word"
        );

        let elsewhere = mouse_action(&d, area, &left(second_cell.0, second_cell.1));
        d.apply(elsewhere);
        assert_eq!(
            d.selection.as_ref().map(|s| s.granularity),
            Some(Granularity::Armed),
            "a press at a different cell restarts the count at one"
        );
        assert_eq!(d.selection.as_ref().map(|s| s.anchor), Some((1, 2)));

        // A second press at that new cell selects the word there.
        let second_press = mouse_action(&d, area, &left(second_cell.0, second_cell.1));
        d.apply(second_press);
        assert_eq!(
            d.selection.as_ref().map(|s| s.granularity),
            Some(Granularity::Word)
        );
    }

    #[test]
    fn a_dragged_span_arms_rather_than_widening() {
        // `text-selection`: "A dragged span arms rather than widening".
        let dashboard = word_dashboard("- zone call\n");
        let area = WIDE;
        let content = detail_content_area(area, Route::Detail);
        let anchor_cell = (content.x + 2, content.y);
        let focus_cell = (content.x + 6, content.y);

        let mut d = dashboard.clone();
        let begin = mouse_action(&d, area, &left(anchor_cell.0, anchor_cell.1));
        d.apply(begin);
        let extend = mouse_action(&d, area, &drag(focus_cell.0, focus_cell.1));
        d.apply(extend);
        assert_eq!(
            d.selection.as_ref().map(|s| s.granularity),
            Some(Granularity::Span),
            "the drag completes a span"
        );

        // A press then lands on that span's own anchor cell.
        let after_drag = mouse_action(&d, area, &left(anchor_cell.0, anchor_cell.1));
        d.apply(after_drag);
        assert_eq!(
            d.selection.as_ref().map(|s| s.granularity),
            Some(Granularity::Armed),
            "a press at a dragged span's anchor arms rather than widening"
        );

        // A second press there selects the word.
        let second = mouse_action(&d, area, &left(anchor_cell.0, anchor_cell.1));
        d.apply(second);
        assert_eq!(
            d.selection.as_ref().map(|s| s.granularity),
            Some(Granularity::Word)
        );
    }

    // ------------------------------------------------------------------
    // `text-selection` group 7: copy on completion.
    // ------------------------------------------------------------------

    /// `len` columns of `buffer`'s row `y`, starting at `x` — one symbol per column, on
    /// `ui::mod`'s own `row_cols` terms (`src/ui/mod.rs`'s `detail` test module), since
    /// `crate::testutil::row_text` reads a whole row and the bullet glyph `word_dashboard`
    /// renders is multi-byte, which rules out a byte-offset slice of it.
    fn row_cols(buffer: &ratatui::buffer::Buffer, x: u16, y: u16, len: u16) -> String {
        (x..x + len)
            .map(|x| buffer[(x, y)].symbol().to_string())
            .collect()
    }

    /// A `ClipboardWriter` double, on `crate::testutil::RecordingReader`'s terms: records
    /// every text passed to it and answers `Ok` or a scripted `Err`, so a test can assert
    /// both that a write happened (or did not) and what it carried.
    struct ClipboardRecorder {
        calls: std::cell::RefCell<Vec<String>>,
        fail_with: Option<&'static str>,
    }

    impl ClipboardRecorder {
        fn ok() -> Self {
            Self {
                calls: std::cell::RefCell::new(Vec::new()),
                fail_with: None,
            }
        }

        fn failing(reason: &'static str) -> Self {
            Self {
                calls: std::cell::RefCell::new(Vec::new()),
                fail_with: Some(reason),
            }
        }

        fn write(&self, text: &str) -> Result<(), String> {
            self.calls.borrow_mut().push(text.to_string());
            match self.fail_with {
                Some(reason) => Err(reason.to_string()),
                None => Ok(()),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    /// Run `dashboard` through `run_loop` over `mouse_events` (each queued with `MouseEventKind`
    /// values as returned by `crate::testutil::mouse`), followed by `q`, at `WIDE` — the
    /// standard shape every test below shares, so each test's own body is only the events and
    /// the assertions.
    fn drive_mouse_events(
        dashboard: &mut Dashboard,
        mouse_events: Vec<MouseEvent>,
        read: crate::ui::app::ArtifactReader<'_>,
        write: crate::ui::app::ClipboardWriter<'_>,
    ) {
        drive_events(
            dashboard,
            mouse_events.into_iter().map(Event::Mouse).collect(),
            read,
            write,
        );
    }

    /// The shared body behind [`drive_mouse_events`] and
    /// `the_pane_is_still_complete_without_a_pointer`'s key-only session: a
    /// full `run_loop` drive over the given events, `q` appended last, through
    /// a real `TestBackend` terminal and every worker as `crate::*::none()`.
    /// Generalised from mouse-only events because the latter needs a mixed
    /// or key-only queue to prove the pane needs no mouse event at all.
    fn drive_events(
        dashboard: &mut Dashboard,
        events: Vec<Event>,
        read: crate::ui::app::ArtifactReader<'_>,
        write: crate::ui::app::ClipboardWriter<'_>,
    ) {
        // `run_loop` breaks the instant `dashboard.quit` is set, which a
        // previous call left `true` — this harness re-runs `run_loop` from
        // scratch each time, so that leftover would end the loop before its
        // own first real event's effect ever reached a second sync-and-draw
        // pass (the one that matters for a tab switch: `sync_detail` runs
        // only at the top of the *next* iteration, after the event that
        // changed `detail.tab` was applied).
        dashboard.quit = false;
        let mut queue: Vec<Result<Option<Event>, crate::ui::event::EventError>> =
            events.into_iter().map(|event| Ok(Some(event))).collect();
        // `Ctrl-C` rather than a bare `q`: `action_for` types a bare `q` into
        // the query while filtering, per `specs/list-filtering/spec.md`, so a
        // caller here whose events leave filter mode active would otherwise
        // starve the script rather than quit — `Ctrl-C` quits unconditionally
        // at either mode.
        queue.push(Ok(Some(press(KeyCode::Char('c'), KeyModifiers::CONTROL))));
        let mut events = Script::new(queue);
        let backend = TestBackend::new(WIDE.width, WIDE.height);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
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
            dashboard,
            &mut events,
            &mut live,
            read,
            write,
            Duration::from_millis(1),
        )
        .expect("loop ends");
    }

    /// A `Route::Detail` dashboard whose one selected active change resolves one artifact —
    /// `word_dashboard`'s own fixture presets `detail.sections` directly, which a real
    /// `run_loop` drive discards on its very first `sync_detail`; this fixture instead gives
    /// `sync_detail` a real path to (re-)read, so the same markdown source survives a full
    /// loop drive the way `word_dashboard` only ever survived a bare `mouse_action` + `apply`.
    fn word_content_dashboard() -> Dashboard {
        let change = crate::changes::fixture::with_artifacts(
            crate::changes::fixture::active("s0", 0, 1),
            &[("proposal", &["/repo/p.md"])],
        );
        let mut d = dashboard_with_change("/repo", "unused", 0, 0);
        d.changes = crate::changes::fixture::set(vec![change], Vec::new(), Vec::new());
        d.selected = 1;
        d.route = Route::Detail;
        d
    }

    /// `specs/text-selection/spec.md` -> "A press arms, a second selects the word, a third
    /// selects the row": only the second and third presses at one cell **complete** a
    /// selection, so only they reach the clipboard — a first press arms nothing to copy, and a
    /// fourth leaves the row selection unchanged from the third (design.md -> Decision 12,
    /// `text-selection` -> "The selected text is copied through the terminal seam").
    #[test]
    fn only_the_second_and_third_presses_at_one_cell_copy() {
        let mut d = word_content_dashboard();
        let read = |_: &std::path::Path| Ok("- zone call\n".to_string());
        let content = detail_content_area(WIDE, Route::Detail);
        let point = (content.x + 2, content.y);
        let recorder = ClipboardRecorder::ok();
        let write = |t: &str| recorder.write(t);

        drive_mouse_events(&mut d, vec![left(point.0, point.1)], &read, &write);
        assert!(
            recorder.calls().is_empty(),
            "a first press arms nothing to copy: {:?}",
            recorder.calls()
        );

        drive_mouse_events(&mut d, vec![left(point.0, point.1)], &read, &write);
        assert_eq!(
            d.selection.as_ref().map(|s| s.granularity),
            Some(Granularity::Word)
        );
        assert_eq!(
            recorder.calls().len(),
            1,
            "the second press completes the word: {:?}",
            recorder.calls()
        );
        assert_eq!(recorder.calls()[0], "zone");
        assert_eq!(d.selection.as_ref().and_then(|s| s.problem.clone()), None);

        drive_mouse_events(&mut d, vec![left(point.0, point.1)], &read, &write);
        assert_eq!(
            d.selection.as_ref().map(|s| s.granularity),
            Some(Granularity::Row)
        );
        assert_eq!(
            recorder.calls().len(),
            2,
            "the third press completes the row: {:?}",
            recorder.calls()
        );
        assert_eq!(recorder.calls()[1], "• zone call");

        drive_mouse_events(&mut d, vec![left(point.0, point.1)], &read, &write);
        assert_eq!(
            recorder.calls().len(),
            2,
            "a fourth press changes nothing further and copies nothing again: {:?}",
            recorder.calls()
        );
    }

    /// `specs/text-selection/spec.md` -> "A single press with no motion selects nothing": a
    /// press followed by a release, with no intervening drag, arms the cell — recorded, so a
    /// second press there can widen to the word — but highlights nothing and copies nothing.
    #[test]
    fn a_single_press_with_no_motion_selects_nothing() {
        let mut d = word_content_dashboard();
        let read = |_: &std::path::Path| Ok("- zone call\n".to_string());
        let content = detail_content_area(WIDE, Route::Detail);
        let point = (content.x + 2, content.y);
        let recorder = ClipboardRecorder::ok();
        let write = |t: &str| recorder.write(t);

        drive_mouse_events(
            &mut d,
            vec![
                left(point.0, point.1),
                m(MouseEventKind::Up(MouseButton::Left), point.0, point.1),
            ],
            &read,
            &write,
        );

        assert_eq!(
            d.selection.as_ref().map(|s| s.granularity),
            Some(Granularity::Armed),
            "the press is recorded, arming the cell"
        );
        assert!(
            recorder.calls().is_empty(),
            "a press with no drag copies nothing: {:?}",
            recorder.calls()
        );
        let rows = crate::ui::detail::content_lines(
            &d.detail,
            d.selected_change(),
            d.detail.drawn_width.expect("a frame has been drawn"),
        );
        let selection = d.selection.clone().expect("the press was recorded");
        assert_eq!(
            view::highlight_span(&rows, &selection),
            None,
            "an armed selection highlights nothing"
        );

        // A second press at that same cell, with no drag before it, widens to
        // the word — proving the first press really was recorded rather than
        // dropped by the release.
        drive_mouse_events(&mut d, vec![left(point.0, point.1)], &read, &write);
        assert_eq!(
            d.selection.as_ref().map(|s| s.granularity),
            Some(Granularity::Word),
            "a second press at the same cell widens to the word"
        );
    }

    /// `specs/text-selection/spec.md` -> "The selected text is copied through the terminal
    /// seam": a drag's finishing release — not any of the motion in between — is what copies
    /// its span (design.md -> Decision 11 and 12).
    #[test]
    fn a_completed_drag_copies_its_span_on_release_and_not_before() {
        let mut d = word_content_dashboard();
        let read = |_: &std::path::Path| Ok("- zone call\n".to_string());
        let content = detail_content_area(WIDE, Route::Detail);
        let anchor = (content.x + 2, content.y);
        let focus = (content.x + 11, content.y);
        let recorder = ClipboardRecorder::ok();
        let write = |t: &str| recorder.write(t);

        drive_mouse_events(
            &mut d,
            vec![left(anchor.0, anchor.1), drag(focus.0, focus.1)],
            &read,
            &write,
        );
        assert_eq!(
            d.selection.as_ref().map(|s| s.granularity),
            Some(Granularity::Span),
            "the drag itself only extends the span"
        );
        assert!(
            recorder.calls().is_empty(),
            "motion copies nothing before release: {:?}",
            recorder.calls()
        );

        drive_mouse_events(
            &mut d,
            vec![m(MouseEventKind::Up(MouseButton::Left), focus.0, focus.1)],
            &read,
            &write,
        );
        let rows = crate::ui::detail::content_lines(
            &d.detail,
            d.selected_change(),
            d.detail.drawn_width.expect("a frame has been drawn"),
        );
        let selection = d.selection.clone().expect("the span survives the release");
        let (start, end) =
            view::highlight_span(&rows, &selection).expect("a span always highlights something");
        let expected = crate::ui::detail::span_text(&rows, start, end);
        assert_eq!(
            recorder.calls(),
            vec![expected],
            "the release copies exactly the highlighted span, once"
        );
    }

    /// `specs/text-selection/spec.md` -> "The clipboard write cannot be confirmed, and the
    /// pane claims nothing": an `Ok` write leaves `Selection::problem` at `None` and renders
    /// no row (Decision 6 and 12) — the `Err` half is
    /// `a_failed_write_is_recorded_as_a_problem_and_rendered` below.
    #[test]
    fn a_successful_write_leaves_no_problem_and_renders_no_row() {
        let mut d = word_content_dashboard();
        let read = |_: &std::path::Path| Ok("- zone call\n".to_string());
        let content = detail_content_area(WIDE, Route::Detail);
        let point = (content.x + 2, content.y);
        let recorder = ClipboardRecorder::ok();
        let write = |t: &str| recorder.write(t);

        drive_mouse_events(
            &mut d,
            vec![left(point.0, point.1), left(point.0, point.1)],
            &read,
            &write,
        );
        assert_eq!(recorder.calls().len(), 1, "the second press copies once");
        assert_eq!(
            d.selection.as_ref().and_then(|s| s.problem.clone()),
            None,
            "an Ok write leaves no problem"
        );

        let backend = TestBackend::new(WIDE.width, WIDE.height);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        terminal
            .draw(|frame| view::render(frame, &d))
            .expect("draw");
        let below = row_cols(terminal.backend().buffer(), content.x, content.y + 1, 20);
        assert!(
            !below.trim_start().starts_with('!'),
            "no problem row is drawn when the write succeeded: {below:?}"
        );
    }

    /// `specs/text-selection/spec.md` -> "The clipboard write cannot be confirmed, and the
    /// pane claims nothing" -> "when the seam returns Err, the reason is recorded and
    /// rendered as a `!`-marked row, because that failure **is** observable" (design.md ->
    /// Decision 12).
    #[test]
    fn a_failed_write_is_recorded_as_a_problem_and_rendered() {
        let mut d = word_content_dashboard();
        let read = |_: &std::path::Path| Ok("- zone call\n".to_string());
        let content = detail_content_area(WIDE, Route::Detail);
        let point = (content.x + 2, content.y);
        let recorder = ClipboardRecorder::failing("clipboard unavailable");
        let write = |t: &str| recorder.write(t);

        drive_mouse_events(
            &mut d,
            vec![left(point.0, point.1), left(point.0, point.1)],
            &read,
            &write,
        );
        assert_eq!(
            d.selection.as_ref().and_then(|s| s.problem.clone()),
            Some("clipboard unavailable".to_string()),
            "a failing write's reason is stored on Selection::problem"
        );
        // The highlight is the pane's only claim, and it survives the failure —
        // Decision 6: the selection itself is untouched by a failed write.
        assert_eq!(
            d.selection.as_ref().map(|s| s.granularity),
            Some(Granularity::Word)
        );

        let backend = TestBackend::new(WIDE.width, WIDE.height);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        terminal
            .draw(|frame| view::render(frame, &d))
            .expect("draw");
        let below = row_cols(terminal.backend().buffer(), content.x, content.y + 1, 30);
        assert!(
            below.starts_with("! clipboard unavailable"),
            "the reason is rendered as a detail-region problem row: {below:?}"
        );

        // A later completing gesture that succeeds clears the standing problem.
        let ok_recorder = ClipboardRecorder::ok();
        let ok_write = |t: &str| ok_recorder.write(t);
        drive_mouse_events(&mut d, vec![left(point.0, point.1)], &read, &ok_write);
        assert_eq!(
            d.selection.as_ref().and_then(|s| s.problem.clone()),
            None,
            "a fresh gesture supersedes the standing problem"
        );
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

        // A change-row click against `j`, and the second click that opens it
        // against `Enter` — `text-selection`'s addition, closing the gap the
        // three cases below left: `Click(Target::Change)` is the one `Click`
        // sub-case this test did not yet prove against a key, though
        // `a_detail_header_click_equals_space` already proves
        // `Click(Target::DetailHeader)` against `Space` at `Route::Detail`.
        let base = mouse_dashboard(4, 0);
        // Interior row 0 is the `active` header; row 1 is the first change —
        // the same row `selected` (starting on the header, at 0) reaches by
        // moving one step, so `Next` and this click land on the same target.
        let row = list_row(WIDE, Route::List, 1);
        let click = mouse_action(&base, WIDE, &left(5, row));
        assert_eq!(click, Action::Click(Target::Change(0)));

        let mut clicked = base.clone();
        let mut keyed = base.clone();
        clicked.apply(click);
        keyed.apply(Action::Next);
        assert_eq!(clicked, keyed, "a click on the next row against `j`");

        let second_click = mouse_action(&clicked, WIDE, &left(5, row));
        let mut opened_by_click = clicked.clone();
        let mut opened_by_key = clicked.clone();
        opened_by_click.apply(second_click);
        opened_by_key.apply(Action::OpenDetail);
        assert_eq!(
            opened_by_click, opened_by_key,
            "a second click on the selected row against `Enter`"
        );

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
    fn the_pane_is_still_complete_without_a_pointer() {
        // `mouse-input`: "The pane is still complete without a pointer" — a
        // full session driven by keys alone: list navigation, an archived
        // fold, the filter, opening a change, switching between its two
        // artifacts, folding the foldable one, an agent key, and the help
        // overlay. `drive_events` sends only `Event::Key` — `mouse_action` is
        // never called at all — and `dashboard.selection` stays `None`
        // throughout, which is the one thing this session cannot reach:
        // copying text has no keyboard path (`text-selection`).
        let c0 = crate::changes::fixture::with_artifacts(
            crate::changes::fixture::active("c0", 0, 1),
            &[
                ("proposal", &["/repo/openspec/changes/c0/proposal.md"]),
                ("spec", &["/repo/openspec/changes/c0/spec.md"]),
            ],
        );
        let a0 = crate::changes::fixture::archived(Some("2026-01-01"), "a0", 1, 1);
        let mut d = dashboard_with_change("/repo", "unused", 0, 0);
        d.changes = crate::changes::fixture::set(vec![c0], vec![a0], Vec::new());
        d.agents.reachable = true;
        d.sections.collapsed = std::collections::BTreeSet::from([SectionKey::Archived]);

        let read = |path: &std::path::Path| {
            if path.ends_with("spec.md") {
                Ok("### Requirement: A\n\nBody A.\n\n### Requirement: B\n\nBody B.\n".to_string())
            } else {
                Ok("proposal body\n".to_string())
            }
        };
        let recorder = ClipboardRecorder::ok();
        let write = |t: &str| recorder.write(t);

        // List navigation: move onto the archived header and unfold it with
        // `Space`, exactly as `foldable-spec-sections` -> `Action::ToggleSection`
        // does when a click drives it.
        assert!(
            d.sections.collapsed.contains(&SectionKey::Archived),
            "archived starts collapsed"
        );
        d.selected = d
            .targets()
            .iter()
            .position(|t| *t == Target::Section(SectionKey::Archived))
            .expect("the archived header is drawn");
        drive_events(
            &mut d,
            vec![press(KeyCode::Char(' '), KeyModifiers::NONE)],
            &read,
            &write,
        );
        assert!(
            !d.sections.collapsed.contains(&SectionKey::Archived),
            "`Space` unfolds the archived section from the keyboard alone"
        );
        assert_eq!(d.route, Route::List, "still at the list route");
        assert_eq!(d.selection, None);

        // The filter: start it, type a query, and cancel it with `Esc` — the
        // layered dismissal clears both the mode and the query.
        drive_events(
            &mut d,
            vec![
                press(KeyCode::Char('/'), KeyModifiers::NONE),
                press(KeyCode::Char('c'), KeyModifiers::NONE),
                press(KeyCode::Char('0'), KeyModifiers::NONE),
            ],
            &read,
            &write,
        );
        assert!(d.filter.active, "filter mode is active");
        assert_eq!(d.filter.query, "c0");
        drive_events(
            &mut d,
            vec![press(KeyCode::Esc, KeyModifiers::NONE)],
            &read,
            &write,
        );
        assert!(!d.filter.active, "`Esc` cancels filter mode");
        assert_eq!(
            d.filter.query, "",
            "and clears the query, both from the keyboard"
        );
        assert_eq!(d.selection, None);

        // Open `c0`'s detail, then switch from its first artifact to its
        // second with a digit key.
        d.selected = d
            .targets()
            .iter()
            .position(|t| matches!(t, Target::Change(_)))
            .expect("c0 is drawn");
        drive_events(
            &mut d,
            vec![press(KeyCode::Enter, KeyModifiers::NONE)],
            &read,
            &write,
        );
        assert_eq!(d.route, Route::Detail, "`Enter` opens the detail route");
        assert_eq!(d.detail.tab, 0);
        assert_eq!(d.detail.sections.len(), 1, "the proposal is a single file");
        assert!(!d.detail.foldable(), "one section is not foldable");
        drive_events(
            &mut d,
            vec![press(KeyCode::Char('2'), KeyModifiers::NONE)],
            &read,
            &write,
        );
        assert_eq!(d.detail.tab, 1, "the digit key reaches the second artifact");
        assert_eq!(
            d.detail.sections.len(),
            2,
            "the spec's two `Requirement:` headings split it in two"
        );
        assert!(d.detail.foldable(), "two sections is foldable");

        // Fold the first section from the keyboard, cursor already at line 0.
        assert_eq!(d.detail.scroll, 0);
        assert!(
            d.detail.expanded.is_empty(),
            "every section starts collapsed"
        );
        drive_events(
            &mut d,
            vec![press(KeyCode::Char(' '), KeyModifiers::NONE)],
            &read,
            &write,
        );
        assert!(
            d.detail.expanded.contains(&0),
            "`Space` opened the first section from the keyboard alone"
        );

        // An agent key: `a` launches with no mouse ever having named a change.
        drive_events(
            &mut d,
            vec![press(KeyCode::Char('a'), KeyModifiers::NONE)],
            &read,
            &write,
        );
        assert!(
            d.launch.in_flight,
            "`a` reaches the launcher from the keyboard alone: {:?}",
            d.launch
        );
        assert!(d.launch.problems.is_empty(), "{:?}", d.launch);

        // The help overlay: open it and close it again, both with `?`.
        drive_events(
            &mut d,
            vec![press(KeyCode::Char('?'), KeyModifiers::NONE)],
            &read,
            &write,
        );
        assert!(d.help.open, "`?` opened the overlay");
        drive_events(
            &mut d,
            vec![press(KeyCode::Char('?'), KeyModifiers::NONE)],
            &read,
            &write,
        );
        assert!(!d.help.open, "`?` closed it again");

        // Every step above drove the pane with keys alone: `mouse_action` was
        // never called, and `Dashboard::selection` — the one field a mouse
        // gesture reaches and a key does not — was never touched.
        assert_eq!(
            d.selection, None,
            "no keyboard session can select or copy text"
        );
        assert!(
            recorder.calls().is_empty(),
            "nothing was ever written to the clipboard: {:?}",
            recorder.calls()
        );
    }

    #[test]
    fn pointer_motion_does_not_draw() {
        // `dashboard-loop`: "Pointer motion does not cost a frame". Since
        // `text-selection` this is `Moved` alone — a held-button drag is
        // covered separately by
        // `a_held_button_drag_draws_and_a_free_pointer_motion_does_not`,
        // because whether a drag draws now depends on where it lands rather
        // than on its kind alone.
        fn drive(kind: MouseEventKind) -> LoopSummary {
            let mut queue: Vec<_> = (0..20)
                .map(|i| Ok(Some(crate::testutil::mouse(kind, i as u16, 10))))
                .collect();
            queue.push(Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))));
            drive_queue(queue)
        }

        assert_eq!(
            drive(MouseEventKind::Moved),
            LoopSummary {
                frames: 1,
                polls: 21
            }
        );

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

    #[test]
    fn a_held_button_drag_draws_and_a_free_pointer_motion_does_not() {
        // `dashboard-loop`: "A held-button drag draws and a free pointer
        // motion does not" — `text-selection`'s correction of the pointer-
        // motion exemption from "any motion" to `Moved` alone: a drag over
        // the selectable content area draws, because the selection's focus
        // moved and the highlight is what tells the reader what they are
        // selecting.
        let mut dashboard = foldable_dashboard(
            three_spec_sections(),
            std::collections::BTreeSet::from([1]),
            0,
        );
        // `sync_detail` re-reads and overwrites `detail.sections` whenever
        // its `(change directory, tab)` cache key does not already match —
        // `foldable_dashboard` leaves `loaded` at `None`, which run_loop's
        // own pre-draw sync would otherwise treat as unloaded and replace
        // with whatever the injected reader below returns. Priming the key
        // here is what lets the hand-built three sections survive the loop's
        // own draw.
        dashboard.detail.loaded = dashboard.selected_change().map(|c| (c.dir.clone(), 0));
        let content = detail_content_area(WIDE, Route::Detail);
        let (column, row) = (content.x, content.y + 2);

        let moved = drive_queue_on(
            dashboard.clone(),
            vec![
                Ok(Some(crate::testutil::mouse(
                    MouseEventKind::Moved,
                    column,
                    row,
                ))),
                Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
            ],
        );
        assert_eq!(
            moved,
            LoopSummary {
                frames: 1,
                polls: 2
            },
            "a free pointer motion over the content area produces no draw"
        );

        let dragged = drive_queue_on(
            dashboard.clone(),
            vec![
                Ok(Some(crate::testutil::mouse(
                    MouseEventKind::Drag(MouseButton::Left),
                    column,
                    row,
                ))),
                Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
            ],
        );
        assert_eq!(
            dragged,
            LoopSummary {
                frames: 2,
                polls: 2
            },
            "a held-button drag over the selectable content area draws exactly once"
        );

        let ignored = drive_queue(vec![
            Ok(Some(crate::testutil::mouse(
                MouseEventKind::Drag(MouseButton::Left),
                5,
                list_row(WIDE, Route::List, 0),
            ))),
            Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))),
        ]);
        assert_eq!(
            ignored,
            LoopSummary {
                frames: 1,
                polls: 2
            },
            "a drag over Zone::ListRow, which resolves to Action::Ignore, produces no draw"
        );
    }

    /// Drive `run_loop` at 120x40 over `mouse_dashboard(3, 0)` with `queue`,
    /// returning the summary. Every collaborator is the inert double.
    fn drive_queue(queue: Vec<Result<Option<Event>, crate::ui::event::EventError>>) -> LoopSummary {
        drive_queue_on(mouse_dashboard(3, 0), queue)
    }

    /// [`drive_queue`] over a caller-supplied `Dashboard` — `help-overlay`'s
    /// own motion scenario needs the same drive with `help.open` set, and the
    /// fixture is the only thing that differs.
    fn drive_queue_on(
        mut dashboard: Dashboard,
        queue: Vec<Result<Option<Event>, crate::ui::event::EventError>>,
    ) -> LoopSummary {
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
            &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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
        // not. At 120x40 the detail region's tab bar is row 2 spanning columns
        // 42-119, and the third `tdd` cell (` design `) is painted at columns
        // 60-67 — past the 60-column frame's right edge entirely. So:
        //
        //   - resolved against the **stale** 120-column frame, (62, 2) is a
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
                2,
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
            third >= 60 && bar.y == 2,
            "the fixture must place the third tab cell past the 60-column frame: \
             cell at {third}, bar row {}",
            bar.y
        );
        assert_eq!(
            mouse_action(&dashboard, WIDE, &left(62, bar.y)),
            Action::SelectTab(2),
            "against the 120-column frame the press is a tab click"
        );
        assert_eq!(
            mouse_action(&dashboard, NARROW, &left(62, bar.y)),
            Action::Ignore,
            "against the 60-column frame it is outside the frame"
        );

        let summary = run_loop(
            &mut terminal,
            &mut dashboard,
            &mut events,
            &mut live,
            &|_: &std::path::Path| Ok(String::new()),
            &|_: &str| Ok(()),
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
            // `detail.sections` from the reader before every draw, and
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
                &|_: &str| Ok(()),
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
            &|_: &str| Ok(()),
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

    // ---------------------------------------------------------------------
    // `foldable-spec-sections`' outer-loop acceptance tier (tasks.md group 0).
    //
    // Two scenarios are true only of the assembled loop: a fold must read no
    // file, and the per-frame clamp must leave the cursor on the last *line*
    // rather than the last *screenful*. Both are about what happens between a
    // keypress and the next frame, which no unit test sees.
    //
    // Built from `changes::fixture` values, a `RecordingReader`, the inert
    // `FsEvents`/`Refresher`/`AgentPoll`/`Launcher` stubs this module already
    // uses, and the scripted event source. **No `ScratchDir`** — design.md ->
    // Test Boundaries grants a real filesystem only to the `ui::load` startup
    // scenarios.
    // ---------------------------------------------------------------------

    /// The three paths a `specs/**/*.md` glob resolves to for the fixture
    /// change, in resolution order. Their capability directories are what
    /// `artifact-folds` derives the three section labels from.
    const SPEC_PATHS: [&str; 3] = [
        "/repo/openspec/changes/foldable/specs/alpha/spec.md",
        "/repo/openspec/changes/foldable/specs/beta/spec.md",
        "/repo/openspec/changes/foldable/specs/gamma/spec.md",
    ];

    /// A reader answering each of [`SPEC_PATHS`] with its own one-line body,
    /// and recording every call so a fold can be shown to read nothing. Its
    /// default is an **error**, so a read of any other path shows up as a
    /// problem row rather than passing silently.
    fn three_section_reader() -> crate::testutil::RecordingReader {
        crate::testutil::RecordingReader::new(
            SPEC_PATHS
                .iter()
                .map(|p| {
                    let label = std::path::Path::new(p)
                        .parent()
                        .and_then(|d| d.file_name())
                        .and_then(|n| n.to_str())
                        .expect("fixture path has a capability directory")
                        .to_string();
                    (std::path::PathBuf::from(p), Ok(format!("# {label} body\n")))
                })
                .collect(),
            Err("no such fixture path".to_string()),
        )
    }

    /// A dashboard whose selected change carries one artifact resolving to
    /// **three** paths, which is what makes it foldable. `route` lets one
    /// fixture serve both the scenario that opens the detail with `Enter` and
    /// the one that starts there.
    fn three_section_dashboard(route: Route) -> Dashboard {
        let change = crate::changes::fixture::with_artifacts(
            crate::changes::fixture::active("foldable", 2, 7),
            &[("specs", &SPEC_PATHS)],
        );
        Dashboard {
            selection: None,
            help: crate::ui::app::Help {
                open: false,
                scroll: 0,
            },
            repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
            searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
            changes: crate::changes::fixture::set(vec![change], Vec::new(), Vec::new()),
            route,
            quit: false,
            // `list-sections`: 1, not 0 — target 0 is the active header.
            selected: 1,
            filter: crate::ui::app::Filter {
                query: String::new(),
                active: false,
            },
            detail: crate::ui::app::Detail {
                sections: Vec::new(),
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
                expanded: std::collections::BTreeSet::new(),
                drawn_width: None,
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

    /// What one acceptance run leaves behind: the dashboard as the loop left
    /// it, every path the reader was asked for in order, the final buffer, and
    /// the loop's own summary.
    struct AcceptanceRun {
        dashboard: Dashboard,
        reads: Vec<std::path::PathBuf>,
        buffer: ratatui::buffer::Buffer,
        summary: LoopSummary,
    }

    /// Run the fixture's loop over `script` at `width` x `height`. One place, so
    /// the three scenarios below differ only in their script and their size.
    fn run_three_section_loop(
        route: Route,
        script: Vec<ratatui::crossterm::event::Event>,
        width: u16,
        height: u16,
    ) -> AcceptanceRun {
        let backend = TestBackend::new(width, height);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let mut dashboard = three_section_dashboard(route);
        let mut events = Script::new(script.into_iter().map(|e| Ok(Some(e))).collect());
        let recorder = three_section_reader();
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
            &|_: &str| Ok(()),
            Duration::from_millis(1),
        )
        .expect("loop ends");

        AcceptanceRun {
            dashboard,
            reads: recorder.paths(),
            buffer: terminal.backend().buffer().clone(),
            summary,
        }
    }

    /// The content area's first column at a given frame width: column 42 in the
    /// wide layout's detail region, column 1 in the narrow one. Derived from the
    /// breakpoint rather than written twice.
    fn content_x(width: u16) -> u16 {
        if width >= 100 { 42 } else { 1 }
    }

    /// The content area's first row, which `pane-chrome` put below the region's
    /// heading row, its padding row, the tab bar, the rule, and the content
    /// padding row.
    const CONTENT_Y: u16 = 5;

    /// The text of the content row `n` rows below the content area's first,
    /// trimmed of the padding `pad_or_truncate_right` adds.
    fn content_row(run: &AcceptanceRun, width: u16, n: u16) -> String {
        row_text(&run.buffer, CONTENT_Y + n)
            .chars()
            .skip(content_x(width) as usize)
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    /// `artifact-folds` :: "A fold reads no file".
    ///
    /// The acceptance scenario for the whole point of the fold: it is a view
    /// operation over already-read text, so four of them must not touch the
    /// reader. `Enter` opens the detail and is what does read — once per
    /// resolved path — and nothing after it reads at all.
    #[test]
    fn a_fold_reads_no_file() {
        let run = run_three_section_loop(
            Route::List,
            vec![
                press(KeyCode::Enter, KeyModifiers::NONE),
                press(KeyCode::Char(' '), KeyModifiers::NONE),
                press(KeyCode::Char(' '), KeyModifiers::NONE),
                press(KeyCode::Char(' '), KeyModifiers::NONE),
                press(KeyCode::Char(' '), KeyModifiers::NONE),
                press(KeyCode::Char('q'), KeyModifiers::NONE),
            ],
            120,
            40,
        );

        // One call per resolved path, in resolution order, and no call after
        // them: the four folds that follow the `Enter` read nothing.
        assert_eq!(
            run.reads,
            SPEC_PATHS
                .iter()
                .map(std::path::PathBuf::from)
                .collect::<Vec<_>>(),
            "the three paths once each, and nothing during the folds"
        );

        // The resulting fold state: four toggles of the section the cursor is
        // on leave it shut, so the tab reads as three collapsed headers.
        // The glyph comes from `ui::list::fold_glyph`, the pair's one production
        // site, rather than being written here as a character: `artifact-folds`
        // requires this capability's own tests to compare function results, and
        // `ui::detail`'s header tests already do. The landed list-region fixtures
        // in `list.rs`, `mod.rs`, and `view.rs` do still spell the glyphs out —
        // re-baselining those is `pane-chrome`'s legacy, not this change's job.
        let shut = crate::ui::list::fold_glyph(true);
        assert_eq!(content_row(&run, 120, 0), format!("{shut} alpha"));
        assert_eq!(content_row(&run, 120, 1), format!("{shut} beta"));
        assert_eq!(content_row(&run, 120, 2), format!("{shut} gamma"));
        assert_eq!(
            content_row(&run, 120, 3),
            "",
            "no body is drawn beneath a shut section"
        );

        // The ratchet. Four presses is an **even** number, so the assertions
        // above are equally satisfied by a detail-route `Space` that does
        // nothing at all — before group 7 this test went red only because the
        // route-blind `Space` folded the *list* section and emptied the detail,
        // and that accident is gone. One press, through the same helper, is
        // what keeps the scenario able to fail for the behaviour it is the
        // outer loop for (Change Review, group 10).
        let opened = run_three_section_loop(
            Route::List,
            vec![
                press(KeyCode::Enter, KeyModifiers::NONE),
                press(KeyCode::Char(' '), KeyModifiers::NONE),
                press(KeyCode::Char('q'), KeyModifiers::NONE),
            ],
            120,
            40,
        );
        let open = crate::ui::list::fold_glyph(false);
        assert_eq!(
            content_row(&opened, 120, 0),
            format!("{open} alpha"),
            "one press opens the section the cursor is on"
        );
        assert_eq!(
            content_row(&opened, 120, 1),
            "# alpha body",
            "and its body is drawn beneath its own header"
        );
        // `heading-sections`: a blank separator row sits between the open
        // section's non-empty body and the next visible header.
        assert_eq!(
            content_row(&opened, 120, 2),
            "",
            "a blank row separates the open body from the next header"
        );
        assert_eq!(
            content_row(&opened, 120, 3),
            format!("{shut} beta"),
            "while its siblings stay shut"
        );
        assert_eq!(
            opened.reads, run.reads,
            "and opening a section reads no file either"
        );
    }

    /// `detail-scroll` :: "A foldable tab is clamped to its last line, not its
    /// last screenful".
    ///
    /// The second acceptance scenario, and the reason `detail.scroll` becomes a
    /// cursor at a foldable tab: `layout::scroll_offset` clamps the offset to
    /// `0` whenever the content fits the region, so three collapsed headers in
    /// a forty-row pane would leave only the first reachable.
    #[test]
    fn a_foldable_tab_is_clamped_to_its_last_line() {
        for width in [120u16, 60] {
            let mut script: Vec<ratatui::crossterm::event::Event> = (0..10)
                .map(|_| press(KeyCode::Char('j'), KeyModifiers::NONE))
                .collect();
            script.push(press(KeyCode::Char('q'), KeyModifiers::NONE));
            let run = run_three_section_loop(Route::Detail, script, width, 40);

            assert_eq!(
                run.dashboard.detail.scroll, 2,
                "width {width}: the cursor stops on the last line, not the last screenful"
            );

            // The third header row is the emphasised one, compared against the
            // palette rather than against a `Modifier` written here.
            //
            // Patched over `Cell::default().style()`, not compared bare: ratatui
            // fills an untouched cell's foreground, background and underline
            // with its own reset value rather than leaving them empty, so a
            // drawn cell's style is that reset ground with the role's own
            // attributes patched on top. Comparing against the bare palette
            // value asserts a `Style` the buffer can never hold. This is
            // `ui::view::tests`' `uncoloured()` convention, which landed in
            // group 6 — after this test was first written.
            //
            // The wording above avoids naming the ratatui colour type, which
            // `scripts/gates/palette.sh` greps the whole of `src/` for, comments
            // included — the known limit that gate states, whose stated repair
            // is exactly this rewording.
            let ground = ratatui::buffer::Cell::default().style();
            let selected = ground.patch(crate::ui::palette::style(
                crate::ui::palette::Role::DetailSectionSelected,
            ));
            let unselected = ground.patch(crate::ui::palette::style(
                crate::ui::palette::Role::DetailSection,
            ));
            assert_eq!(
                cell(&run.buffer, content_x(width), CONTENT_Y + 2).style(),
                selected,
                "width {width}: the cursor's own section header carries the selected role"
            );
            // The other two carry the unselected role, and neither equals the
            // selected one — so the check discriminates `REVERSED` rather than
            // passing on any two styles that happen to differ.
            for n in [0u16, 1] {
                let got = cell(&run.buffer, content_x(width), CONTENT_Y + n).style();
                assert_eq!(
                    got, unselected,
                    "width {width}: header {n} carries the unselected role"
                );
                assert_ne!(
                    got, selected,
                    "width {width}: header {n} must not be the emphasised one"
                );
            }
        }
    }

    /// The control for the two acceptance scenarios above (tasks.md 0.4): the
    /// same fixture, with the script reduced to `q`, draws one frame and returns
    /// `Ok`. So when either of them fails it is because the behaviour is
    /// missing, not because the fixture cannot run.
    #[test]
    fn the_acceptance_fixture_runs_with_no_behaviour_under_test() {
        for (route, width) in [
            (Route::List, 120u16),
            (Route::Detail, 120),
            (Route::Detail, 60),
        ] {
            let run = run_three_section_loop(
                route,
                vec![press(KeyCode::Char('q'), KeyModifiers::NONE)],
                width,
                40,
            );
            assert_eq!(
                run.summary,
                LoopSummary {
                    frames: 1,
                    polls: 1
                },
                "width {width}: one frame drawn and one poll taken"
            );
            // The reader was reachable and answered, so a failing acceptance
            // assertion above cannot be a mis-wired fixture.
            if route == Route::Detail {
                assert_eq!(run.reads.len(), 3, "width {width}: three paths read");
            }
        }
    }

    // ------------------------------------------------------------------
    // `help-overlay` / `mouse-input`: the open overlay captures the mouse.
    // Every one of these resolves against the **band**, never `layout::zone`
    // — while a modal covers the body there is no region under the pointer
    // to resolve to.
    // ------------------------------------------------------------------

    /// A 120x60 frame — the one `mouse-input`'s dismissal scenarios use,
    /// because it is the only mandated-shape frame tall enough for the band
    /// to be smaller than the body and so for "outside the band but inside
    /// the frame" to name any row at all.
    const TALL: Rect = Rect {
        x: 0,
        y: 0,
        width: 120,
        height: 60,
    };

    /// The band `mouse_action` resolves against at `area`, derived here the
    /// way the draw path derives it — `split_frame`'s body, then
    /// `layout::help_band` over `ui::help::content_rows()` — so the tests
    /// compare against the rectangle rather than restating it.
    fn band_for(area: Rect) -> Rect {
        let (body, _) = crate::ui::layout::split_frame(area);
        crate::ui::layout::help_band(body, crate::ui::help::content_rows())
    }

    /// `mouse_dashboard(6, 1)` with the overlay open and a twenty-line
    /// artifact loaded — the fixture every scenario in this section shares.
    fn overlay_open(selected: usize, route: Route) -> Dashboard {
        let mut dashboard = mouse_dashboard(6, 1);
        dashboard.route = route;
        dashboard.selected = selected;
        dashboard.detail.sections = vec![ArtifactSection {
            label: Some(String::new()),
            text: (0..20).map(|i| format!("- line-{i:02}\n")).collect(),
            depth: 0,
            progress: None,
            operation: None,
        }];
        dashboard.help.open = true;
        dashboard
    }

    #[test]
    fn the_wheel_scrolls_the_overlay_from_every_region() {
        // `mouse-input`: "The wheel scrolls the overlay from every region".
        // The wheel scrolls the overlay from anywhere **in the frame** — over
        // the list region, over the detail region, over the divider column,
        // and over the footer row alike — and a point past the frame's own
        // right edge is `Ignore`, because it is not a gesture the pane
        // received.
        let wide_points: [(u16, u16); 4] = [(10, 10), (80, 10), (40, 10), (10, 39)];
        let narrow_points: [(u16, u16); 4] = [(10, 10), (30, 10), (0, 0), (10, 19)];
        const PAST_RIGHT_EDGE: u16 = 200;

        for (area, points) in [(WIDE, wide_points), (NARROW, narrow_points)] {
            let dashboard = overlay_open(2, Route::List);
            for (column, row) in points {
                assert_eq!(
                    mouse_action(
                        &dashboard,
                        area,
                        &m(MouseEventKind::ScrollDown, column, row)
                    ),
                    Action::ScrollDown,
                    "{area:?}: ScrollDown at ({column}, {row})"
                );
                assert_eq!(
                    mouse_action(&dashboard, area, &m(MouseEventKind::ScrollUp, column, row)),
                    Action::ScrollUp,
                    "{area:?}: ScrollUp at ({column}, {row})"
                );

                // Applying either advances the overlay and moves nothing
                // beneath it.
                let mut applied = dashboard.clone();
                applied.apply(Action::ScrollDown);
                assert_eq!(applied.help.scroll, 1, "{area:?}: ({column}, {row})");
                assert_eq!(applied.selected, dashboard.selected);
                assert_eq!(applied.detail.scroll, dashboard.detail.scroll);

                for kind in [MouseEventKind::ScrollLeft, MouseEventKind::ScrollRight] {
                    assert_eq!(
                        mouse_action(&dashboard, area, &m(kind, column, row)),
                        Action::Ignore,
                        "{area:?}: {kind:?} at ({column}, {row}) scrolls nothing"
                    );
                }
            }

            // Past the frame's right edge: outside the band, and outside the
            // frame too, so not a gesture the pane received.
            for kind in [
                MouseEventKind::ScrollDown,
                MouseEventKind::ScrollUp,
                MouseEventKind::ScrollLeft,
                MouseEventKind::ScrollRight,
            ] {
                assert_eq!(
                    mouse_action(&dashboard, area, &m(kind, PAST_RIGHT_EDGE, 10)),
                    Action::Ignore,
                    "{area:?}: {kind:?} past the right edge"
                );
            }
        }
    }

    #[test]
    fn a_click_inside_the_band_does_nothing() {
        // `mouse-input`: "A click inside the band does nothing". The overlay
        // is read-only and holds no control, so there is nothing inside it a
        // click could mean.
        let dashboard = overlay_open(2, Route::List);
        let band = band_for(WIDE);
        let last_interior = band.y + band.height - 2;
        let points: [(u16, u16); 6] = [
            (60, band.y),                          // the top rule row
            (60, band.y + 1),                      // the first interior row
            (60, last_interior),                   // the last interior row
            (60, band.y + band.height - 1),        // the bottom rule row
            (band.x, band.y + 1),                  // its leftmost column
            (band.x + band.width - 1, band.y + 1), // its rightmost column
        ];
        for (column, row) in points {
            assert_eq!(
                mouse_action(&dashboard, WIDE, &left(column, row)),
                Action::Ignore,
                "a press at ({column}, {row}) is inside the band"
            );
        }

        // Applying `Ignore` leaves the dashboard equal, field for field.
        let mut applied = dashboard.clone();
        applied.apply(Action::Ignore);
        assert_eq!(applied, dashboard);
        assert!(applied.help.open);
        assert_eq!(applied.help.scroll, 0);
    }

    #[test]
    fn a_click_outside_the_band_dismisses_it_and_selects_nothing() {
        // `mouse-input`: "A click outside the band dismisses it and selects
        // nothing".
        let mut dashboard = overlay_open(2, Route::List);
        dashboard.help.scroll = 3;
        let band = band_for(TALL);
        // 45 rows centred in a 59-row body: rows 0 through 6 and rows 52
        // through 58 are outside it, and row 59 is the footer.
        assert_eq!((band.y, band.height), (7, 45));

        for (column, row) in [(5u16, 4u16), (5, 55), (5, 59)] {
            assert_eq!(
                mouse_action(&dashboard, TALL, &left(column, row)),
                Action::ToggleHelp,
                "a press at ({column}, {row}) is outside the band and inside the frame"
            );
        }

        // The click that dismissed the overlay did not also move the cursor
        // to the row under it.
        let mut dismissed = dashboard.clone();
        dismissed.apply(mouse_action(&dashboard, TALL, &left(5, 4)));
        assert!(!dismissed.help.open);
        assert_eq!(dismissed.help.scroll, 0);
        assert_eq!(dismissed.selected, 2);

        // A second identical click, now that the overlay is closed, resolves
        // through the click table this requirement took precedence over — so
        // the precedence is conditional on `help.open`, not permanent.
        assert!(
            matches!(
                mouse_action(&dismissed, TALL, &left(5, 4)),
                Action::Click(Target::Change(_))
            ),
            "with the overlay closed the same press names the row under it"
        );

        // Past the frame's right edge: `Ignore`, and **not** `ToggleHelp` — a
        // click the pane never received does not dismiss the overlay.
        let outside = mouse_action(&dashboard, TALL, &left(200, 4));
        assert_ne!(outside, Action::ToggleHelp);
        assert_eq!(outside, Action::Ignore);
    }

    #[test]
    fn the_band_s_edges_are_inside_it() {
        // `mouse-input`: "The band's edges are inside it" — both rule rows
        // belong to the band, pinned from both sides.
        let dashboard = overlay_open(2, Route::List);
        let band = band_for(TALL);
        assert_eq!((band.y, band.height), (7, 45));
        let first = band.y;
        let last = band.y + band.height - 1;
        for (row, want) in [
            (first - 1, Action::ToggleHelp),
            (first, Action::Ignore),
            (last, Action::Ignore),
            (last + 1, Action::ToggleHelp),
        ] {
            assert_eq!(
                mouse_action(&dashboard, TALL, &left(60, row)),
                want,
                "a press at row {row}, against a band of rows {first}..={last}"
            );
        }
    }

    #[test]
    fn motion_still_costs_no_frame_while_the_overlay_is_open() {
        // `mouse-input`: "Motion still costs no frame while the overlay is
        // open". The exemption is `run_loop`'s and is structural, so the
        // overlay does not make a motion event interesting.
        let dashboard = overlay_open(2, Route::List);
        let band = band_for(WIDE);
        for kind in [
            MouseEventKind::Moved,
            MouseEventKind::Drag(MouseButton::Left),
        ] {
            for (column, row) in [(60u16, band.y + 1), (200u16, 10u16)] {
                assert_eq!(
                    mouse_action(&dashboard, WIDE, &m(kind, column, row)),
                    Action::Ignore,
                    "{kind:?} at ({column}, {row})"
                );
            }
        }

        // And `run_loop` skips the draw for each, exactly as it does with the
        // overlay closed: one frame for twenty motion events plus the quit.
        for kind in [
            MouseEventKind::Moved,
            MouseEventKind::Drag(MouseButton::Left),
        ] {
            let mut queue: Vec<_> = (0..20)
                .map(|i| Ok(Some(crate::testutil::mouse(kind, i as u16, 10))))
                .collect();
            queue.push(Ok(Some(press(KeyCode::Char('q'), KeyModifiers::NONE))));
            assert_eq!(
                drive_queue_on(overlay_open(2, Route::List), queue),
                LoopSummary {
                    frames: 1,
                    polls: 21
                },
                "{kind:?} with the overlay open"
            );
        }
    }

    #[test]
    fn nothing_in_the_overlay_is_mouse_only() {
        // `mouse-input`: "Nothing in the overlay is mouse-only". Every
        // gesture the open overlay maps to a non-`Ignore` action is collected
        // by sweeping both mandated frames, and each is shown to have a key
        // that does the same thing — asserted by applying both and comparing
        // the dashboards, since `j` maps to `Next` and the wheel to
        // `ScrollDown` and it is their *effect* that has to agree.
        let kinds = [
            MouseEventKind::Down(MouseButton::Left),
            MouseEventKind::Down(MouseButton::Right),
            MouseEventKind::Down(MouseButton::Middle),
            MouseEventKind::Up(MouseButton::Left),
            MouseEventKind::Drag(MouseButton::Left),
            MouseEventKind::Moved,
            MouseEventKind::ScrollDown,
            MouseEventKind::ScrollUp,
            MouseEventKind::ScrollLeft,
            MouseEventKind::ScrollRight,
        ];
        let dashboard = overlay_open(2, Route::List);
        let mut reachable: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for area in [WIDE, NARROW, TALL] {
            for kind in kinds {
                for row in 0..area.height {
                    for column in 0..area.width {
                        let action = mouse_action(&dashboard, area, &m(kind, column, row));
                        if action != Action::Ignore {
                            reachable.insert(format!("{action:?}"));
                        }
                    }
                }
            }
        }
        assert_eq!(
            reachable,
            ["ScrollDown", "ScrollUp", "ToggleHelp"]
                .iter()
                .map(|s| s.to_string())
                .collect::<std::collections::BTreeSet<String>>(),
            "exactly the wheel's two directions and the dismissing click"
        );

        // Each has a key that does the same thing.
        for (mouse, keys) in [
            (
                Action::ScrollDown,
                vec![
                    (KeyCode::Char('j'), KeyModifiers::NONE),
                    (KeyCode::Down, KeyModifiers::NONE),
                ],
            ),
            (
                Action::ScrollUp,
                vec![
                    (KeyCode::Char('k'), KeyModifiers::NONE),
                    (KeyCode::Up, KeyModifiers::NONE),
                ],
            ),
            (
                Action::ToggleHelp,
                vec![
                    (KeyCode::Esc, KeyModifiers::NONE),
                    (KeyCode::Char('?'), KeyModifiers::NONE),
                ],
            ),
        ] {
            let mut by_mouse = dashboard.clone();
            by_mouse.apply(mouse);
            for (code, modifiers) in keys {
                let mut by_key = dashboard.clone();
                by_key.apply(action_for(&press(code, modifiers), false));
                assert_eq!(by_key, by_mouse, "{code:?} does not do what {mouse:?} does");
            }
        }
    }
}
