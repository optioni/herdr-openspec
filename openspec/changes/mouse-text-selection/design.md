## Context

The reader cannot select text in the pane. `mouse-input` enabled capture, and a terminal with
reporting on routes drags to the application rather than selecting natively.

Everything here rests on `notes/measurements.md`, which took several turns to get right and
overturned two conclusions on the way. The three facts the design stands on:

1. **Native selection and mouse reporting are exclusive.** `?1000` alone — press and release,
   no motion requested — still suppresses drag-selection. There is no narrower ask.
2. **Copilot CLI has both because it does not use native selection at all.** Captured through
   a pty, the terminal hands it a press, eighteen `left+motion` reports and a release, with
   the `Shift` bit on 0 of 79. Its output carries OSC 52 writes and reverse-video spans.
3. **It partitions the screen rather than disambiguating gestures.** A drag cannot select its
   top two rows, which are its tab bar.

Point 3 is what makes this affordable. The partition already exists here as tested code:
`ui::layout::zone` resolves any point to exactly one of six `Zone` values.

## Goals / Non-Goals

**Goals:**

- Drag-select and copy artifact text, with clicks still working, and **no toggle key**.
- Press-count gestures — one arms, two select a word, three select the row — with no clock.
- Reuse the existing partition rather than inventing a gesture-disambiguation rule.

**Non-Goals:**

- No auto-scroll when a drag runs off the edge; a selection covers what is drawn.
- No selection in the list region, the tab bar, or on a section header row.
- No toggle key, no capture-mode narrowing, no keyboard selection path.

## Boundaries

| Piece | Where | Pattern it follows |
|---|---|---|
| drag + press-count resolution | `src/ui/driver.rs` — **`run_loop` and `mouse_action` both live here**, not in `src/ui/mod.rs` | the existing zone-dispatch `match` in `mouse_action` |
| `Action::Select`, `Selection`, `apply` | `src/ui/app.rs` | `Action::Click(Target)` — one variant carrying its payload |
| span → text, word bounds | `src/ui/detail.rs` | `content_lines`, already pure and already returning `Vec<ContentRow>` |
| the highlight | `src/ui/view.rs` | the existing span-to-role mapping, composed not replaced |
| `Role::Selected` | `src/ui/palette.rs` | the crate's only `Color` site |
| OSC 52 | `src/ui/terminal.rs` behind `TerminalOps` | the six existing mode methods; no new *trait* |
| the clipboard **injection** | a `ClipboardWriter<'a>` parameter on `run_loop`, bound in `src/ui/mod.rs` | `ArtifactReader`, injected into `run_loop` the same way for the same reason |

`ui::layout::zone` is **used, not changed**: `Zone` gains no variant.

## Contracts

Crate-internal only. No Herdr surface, no `config.toml` key, no manifest field, no CLI
argument. Additive for the keyboard — no key changes meaning.

**One removal:** `Target::DetailLine`, four `Target` members to three. A reader who clicked a
body line to move the detail cursor uses `j`/`k` or the arrows, which always did the same.

Error surface: `TerminalOps::write_clipboard` returns `Result<(), TerminalError>`; a failure
is rendered as a `!`-marked problem row. A terminal that silently ignores OSC 52 cannot be
detected, and the pane claims nothing about the clipboard — see Decision 6.

## Persistence and Rollout

- **migration** — none. **backfill** — none. **seeding** — none.
- **cache invalidation** — none. The `(change directory, tab)` artifact cache is untouched;
  a selection is cleared on reload rather than keyed into it.
- **index rebuild** — none. **authorization** — none; the pane has no actors.
- **observability** — a failed clipboard write becomes a problem row and a new
  `SPEC.md` degraded-states row bound to a named test in `tests/degraded-coverage.toml`.
- **deployment** — one `make build`; the manifest is untouched, so an already-linked plugin
  picks it up.
- **The plugin's own writes are unchanged.** OSC 52 writes to the *terminal*, not to disk;
  `agent-names.toml` under `HERDR_PLUGIN_STATE_DIR` remains the only file the plugin writes.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| The real terminal (raw mode, alternate screen, capture, **OSC 52**) | **replaced** — never real at any tier; `cargo test` spawns this binary, so reaching `CrosstermOps` corrupts the developer's session *and* their clipboard | **replaced** — the existing `Recorder` implementing `TerminalOps`, extended to record `write_clipboard` |
| `TerminalOps` | replaced (`Recorder`) | replaced (`Recorder`) |
| The system clipboard | **replaced everywhere.** No test asserts against the real clipboard: it is shared mutable state on the developer's machine | replaced |
| ratatui backend | **real** `TestBackend`, in memory, at 120x20 and 60x20 | real `TestBackend` |
| `ui::layout::zone` | **real** — it is pure, and replacing the thing under test would prove nothing | real |
| `ui::detail::content_lines` | **real** — pure over `&str`, and the span's text comes from it | real |
| The artifact reader `&dyn Fn(&Path) -> Result<String, String>` | replaced — the injected reader every loop test already uses | replaced |
| The clipboard writer `&dyn Fn(&str) -> Result<(), String>` | **replaced** — a closure recording calls and answering `Ok`/`Err` on demand | replaced — same |
| Filesystem (`openspec/` tree) | real, a scratch directory, only where a test already builds one | not reached |
| `openspec` CLI, Herdr CLI | **not touched**; replaced wherever an existing test reaches them | not touched |
| `notify` watcher, refresh worker, agent poller, launcher | **not touched**; already behind non-blocking trait objects | not touched |
| The process environment | replaced — the injected `&dyn Fn(&str) -> Option<String>` | replaced |
| A clock | **none exists.** `NOBLOCK` forbids `src/ui/` naming one, and the press counting is deliberately state-based so none is introduced | none |

## Test Strategy

Tiers: **unit** (inline `#[cfg(test)]`, `cargo test`), **contract** (`tests/*.rs`, also
`cargo test`), **gates** (`make gates`). No containerised or network tier, and none is added.

**No outer-loop acceptance group.** The only true end-to-end proof would drive a real terminal
and a real clipboard, which this repository forbids at every tier because `cargo test` spawns
this binary. The outermost honest test is `run_loop` over a `TestBackend` with every
collaborator replaced.

92 scenarios across nine capabilities — **33 new, 59 carried** (counted by enumerating
`#### Scenario:` across `specs/*/spec.md` and diffing each name against the base spec). A
carried scenario is a regression that must stay green; it is listed because a `MODIFIED` or
re-`ADDED` requirement replaces its whole block, so every one is this change's responsibility.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| binding-inventory · An action added without a help row fails `cargo test` | `#[test]` + executed sweep — **carried**, must stay green | unit + contract | none | `cargo test` |
| binding-inventory · A binding removed from the driver and left in the help fails | `#[test]` + executed sweep — **carried**, must stay green | unit + contract | none | `cargo test` |
| binding-inventory · The sweep's totals and the exemption set are pinned | `#[test]` + executed sweep | unit + contract | none | `cargo test` |
| binding-inventory · The sweep covers the mouse under both overlay states | `#[test]` + executed sweep — **carried**, must stay green | unit + contract | none | `cargo test` |
| binding-inventory · `Action::Select` has a `Mouse` row and no exemption | `#[test]` + executed sweep | unit + contract | none | `cargo test` |
| binding-inventory · The inventory's shape is asserted, not described | `#[test]` + executed sweep — **carried**, must stay green | unit + contract | none | `cargo test` |
| binding-inventory · `Space` and `Esc` each appear under their route | `#[test]` + executed sweep — **carried**, must stay green | unit + contract | none | `cargo test` |
| binding-inventory · Both quit keys have a row | `#[test]` + executed sweep — **carried**, must stay green | unit + contract | none | `cargo test` |
| dashboard-loop · Pointer motion does not cost a frame | `#[test]` in `src/ui/app.rs` — **carried**, must stay green | unit | seam replaced | `cargo test ui::app` |
| dashboard-loop · A click after motion still resolves against the drawn frame | `#[test]` in `src/ui/app.rs` — **carried**, must stay green | unit | seam replaced | `cargo test ui::app` |
| dashboard-loop · A held-button drag draws and a free pointer motion does not | `#[test]` in `src/ui/app.rs` | unit | seam replaced | `cargo test ui::app` |
| dashboard-loop · `Dashboard` has no `Default` and no site elides a field | `#[test]` in `src/ui/app.rs` — **carried**, must stay green | unit | seam replaced | `cargo test ui::app` |
| dashboard-loop · The pure view files name no I/O API | `#[test]` in `src/ui/app.rs` — **carried**, must stay green | unit | seam replaced | `cargo test ui::app` |
| dashboard-loop · The shell never names the CLI seam | `#[test]` in `src/ui/app.rs` — **carried**, must stay green | unit | seam replaced | `cargo test ui::app` |
| dashboard-loop · Change literals live only in the gated file | `#[test]` in `src/ui/app.rs` — **carried**, must stay green | unit | seam replaced | `cargo test ui::app` |
| dashboard-loop · The render path names no channel, thread, lock, or clock | `#[test]` in `src/ui/app.rs` — **carried**, must stay green | unit | seam replaced | `cargo test ui::app` |
| dashboard-loop · No test sleeps and then asserts something has already happened | `#[test]` in `src/ui/app.rs` — **carried**, must stay green | unit | seam replaced | `cargo test ui::app` |
| dashboard-loop · `file_mode` is set by the composition root and by nothing else | `#[test]` in `src/ui/app.rs` — **carried**, must stay green | unit | seam replaced | `cargo test ui::app` |
| dashboard-loop · Every field is named at every construction site | `#[test]` in `src/ui/app.rs` | unit | seam replaced | `cargo test ui::app` |
| dashboard-loop · `selection` starts empty and is cleared rather than reloaded | `#[test]` in `src/ui/app.rs` | unit | seam replaced | `cargo test ui::app` |
| doc-conformance · The twelfth claim holds and is falsifiable | leg in `tests/doc_contract.rs` | contract | files on disk | `cargo test --test doc_contract` |
| doc-conformance · The documented claim count matches the file | leg in `tests/doc_contract.rs` | contract | files on disk | `cargo test --test doc_contract` |
| help-overlay · The agent keys launch nothing while the overlay is open | render `#[test]` — **carried**, must stay green | unit | `TestBackend` real | `cargo test ui::help` |
| help-overlay · Both quit keys still quit from inside the overlay | render `#[test]` — **carried**, must stay green | unit | `TestBackend` real | `cargo test ui::help` |
| help-overlay · The overlay swallows every inert action | render `#[test]` | unit | `TestBackend` real | `cargo test ui::help` |
| help-overlay · `j` and `k` scroll the overlay rather than the frame beneath | render `#[test]` — **carried**, must stay green | unit | `TestBackend` real | `cargo test ui::help` |
| help-overlay · The overlay lists the agent keys when the socket is unreachable | render `#[test]` — **carried**, must stay green | unit | `TestBackend` real | `cargo test ui::help` |
| help-overlay · `Action::Select` is inert while the overlay is open | render `#[test]` | unit | `TestBackend` real | `cargo test ui::help` |
| help-overlay · The grammar renders at 120 columns | render `#[test]` — **carried**, must stay green | unit | `TestBackend` real | `cargo test ui::help` |
| help-overlay · The grammar renders at 60 columns | render `#[test]` — **carried**, must stay green | unit | `TestBackend` real | `cargo test ui::help` |
| help-overlay · The key column is measured in display columns | render `#[test]` — **carried**, must stay green | unit | `TestBackend` real | `cargo test ui::help` |
| list-selection · The first target is selected on startup at both widths | `#[test]` in `src/ui/app.rs` — **carried**, must stay green | unit | none | `cargo test ui::app ui::list` |
| list-selection · `j`, `k`, and the arrows move the cursor over headers and changes | `#[test]` in `src/ui/app.rs` — **carried**, must stay green | unit | none | `cargo test ui::app ui::list` |
| list-selection · The cursor clamps at both ends rather than wrapping | `#[test]` in `src/ui/app.rs` — **carried**, must stay green | unit | none | `cargo test ui::app ui::list` |
| list-selection · The cursor crosses the archived header into the archived rows | `#[test]` in `src/ui/app.rs` — **carried**, must stay green | unit | none | `cargo test ui::app ui::list` |
| list-selection · A collapsed section's changes are neither visible nor addressable | `#[test]` in `src/ui/app.rs` — **carried**, must stay green | unit | none | `cargo test ui::app ui::list` |
| list-selection · Navigation over an empty visible list is inert | `#[test]` in `src/ui/app.rs` — **carried**, must stay green | unit | none | `cargo test ui::app ui::list` |
| list-selection · `Enter` on a section header does nothing | `#[test]` in `src/ui/app.rs` — **carried**, must stay green | unit | none | `cargo test ui::app ui::list` |
| list-selection · `Target` carries three members and no resolver emits a fourth | `#[test]` in `src/ui/app.rs` | unit | none | `cargo test ui::app ui::list` |
| mouse-input · The key table is unchanged | `#[test]` in `src/ui/driver.rs` — **carried**, must stay green | unit | none — pure resolver | `cargo test ui::driver` |
| mouse-input · Every mouse action has a key that produces the same effect | `#[test]` in `src/ui/driver.rs` — **carried**, must stay green | unit | none — pure resolver | `cargo test ui::driver` |
| mouse-input · `Action::Select` is the only mouse-only action, by name and count | `#[test]` in `src/ui/driver.rs` | unit | none — pure resolver | `cargo test ui::driver` |
| mouse-input · The pane is still complete without a pointer | `#[test]` in `src/ui/driver.rs` | unit | none — pure resolver | `cargo test ui::driver` |
| mouse-input · The documented bindings match the resolver | `#[test]` in `src/ui/driver.rs` — **carried**, must stay green | unit | none — pure resolver | `cargo test ui::driver` |
| mouse-input · The documented confined set matches the gate | `#[test]` in `src/ui/driver.rs` — **carried**, must stay green | unit | none — pure resolver | `cargo test ui::driver` |
| mouse-input · The documented bypass names what was measured | `#[test]` in `src/ui/driver.rs` | unit | none — pure resolver | `cargo test ui::driver` |
| mouse-input · A click selects a change row and a second click opens it | `#[test]` in `src/ui/driver.rs` — **carried**, must stay green | unit | none — pure resolver | `cargo test ui::driver` |
| mouse-input · A click on a section header folds it exactly as `Space` does | `#[test]` in `src/ui/driver.rs` — **carried**, must stay green | unit | none — pure resolver | `cargo test ui::driver` |
| mouse-input · A click on an archived header opens an unresolved archive and requests its refresh | `#[test]` in `src/ui/driver.rs` — **carried**, must stay green | unit | none — pure resolver | `cargo test ui::driver` |
| mouse-input · A click on an artifact-section header folds it exactly as `Space` does | `#[test]` in `src/ui/driver.rs` — **carried**, must stay green | unit | none — pure resolver | `cargo test ui::driver` |
| mouse-input · A click in an open section's body arms a selection and folds nothing | `#[test]` in `src/ui/driver.rs` | unit | none — pure resolver | `cargo test ui::driver` |
| mouse-input · A non-foldable tab's content is selectable too | `#[test]` in `src/ui/driver.rs` | unit | none — pure resolver | `cargo test ui::driver` |
| mouse-input · A click on a tab cell switches to that artifact | `#[test]` in `src/ui/driver.rs` — **carried**, must stay green | unit | none — pure resolver | `cargo test ui::driver` |
| mouse-input · Clicks that address nothing are inert | `#[test]` in `src/ui/driver.rs` — **carried**, must stay green | unit | none — pure resolver | `cargo test ui::driver` |
| mouse-input · The other buttons and the non-press kinds are inert | `#[test]` in `src/ui/driver.rs` — **carried**, must stay green | unit | none — pure resolver | `cargo test ui::driver` |
| mouse-input · A click on a task group's header folds that group | `#[test]` in `src/ui/driver.rs` — **carried**, must stay green | unit | none — pure resolver | `cargo test ui::driver` |
| mouse-input · A click on a nested scenario header folds only that scenario | `#[test]` in `src/ui/driver.rs` — **carried**, must stay green | unit | none — pure resolver | `cargo test ui::driver` |
| terminal-lifecycle · The real implementation is the only place naming a terminal-mode function | `#[test]` with the `Recorder` double — **carried**, must stay green | unit | `TerminalOps` replaced | `cargo test ui::terminal` |
| terminal-lifecycle · No test constructs the real terminal implementation | `#[test]` with the `Recorder` double — **carried**, must stay green | unit | `TerminalOps` replaced | `cargo test ui::terminal` |
| terminal-lifecycle · The clipboard write is confined and reports its own failure | `#[test]` with the `Recorder` double | unit | `TerminalOps` replaced | `cargo test ui::terminal` |
| terminal-lifecycle · A clipboard write changes no terminal mode | `#[test]` with the `Recorder` double | unit | `TerminalOps` replaced | `cargo test ui::terminal` |
| text-selection · A drag begins only in the detail content area | new `#[test]` | unit | seam replaced, `TestBackend` real | `cargo test ui::` |
| text-selection · A section header stays clickable and is never selectable | new `#[test]` | unit | seam replaced, `TestBackend` real | `cargo test ui::` |
| text-selection · Anchor holds while the focus follows | new `#[test]` | unit | seam replaced, `TestBackend` real | `cargo test ui::` |
| text-selection · A drag past the edge clamps and does not scroll | new `#[test]` | unit | seam replaced, `TestBackend` real | `cargo test ui::` |
| text-selection · A single press with no motion selects nothing | new `#[test]` | unit | seam replaced, `TestBackend` real | `cargo test ui::` |
| text-selection · The span is highlighted at both mandated widths | new `#[test]` | unit | seam replaced, `TestBackend` real | `cargo test ui::` |
| text-selection · The highlight persists after release and clears on the next interaction | new `#[test]` | unit | seam replaced, `TestBackend` real | `cargo test ui::` |
| text-selection · A single-line selection copies exactly the selected columns | new `#[test]` | unit | seam replaced, `TestBackend` real | `cargo test ui::` |
| text-selection · A multi-line selection joins with newlines and drops padding | new `#[test]` | unit | seam replaced, `TestBackend` real | `cargo test ui::` |
| text-selection · The clipboard write cannot be confirmed, and the pane claims nothing | new `#[test]` | unit | seam replaced, `TestBackend` real | `cargo test ui::` |
| text-selection · One press arms, two select a word, three select the row | new `#[test]` | unit | seam replaced, `TestBackend` real | `cargo test ui::` |
| text-selection · A press elsewhere restarts the count | new `#[test]` | unit | seam replaced, `TestBackend` real | `cargo test ui::` |
| text-selection · A click selects the whole token, not a fragment | new `#[test]` | unit | seam replaced, `TestBackend` real | `cargo test ui::` |
| text-selection · A dragged span arms rather than widening | new `#[test]` | unit | seam replaced, `TestBackend` real | `cargo test ui::` |
| view-palette · The palette answers every role with a `Style` | render `#[test]` — **carried**, must stay green | unit | `TestBackend` real | `cargo test ui::palette ui::view` |
| view-palette · The confinement gate catches a `Color` named outside the palette | render `#[test]` — **carried**, must stay green | unit | `TestBackend` real | `cargo test ui::palette ui::view` |
| view-palette · The palette module reaches no I/O and measures no width | render `#[test]` — **carried**, must stay green | unit | `TestBackend` real | `cargo test ui::palette ui::view` |
| view-palette · The enum's membership is exactly this list | render `#[test]` — **carried**, must stay green | unit | `TestBackend` real | `cargo test ui::palette ui::view` |
| view-palette · The three delta roles carry their colour and no modifier | render `#[test]` — **carried**, must stay green | unit | `TestBackend` real | `cargo test ui::palette ui::view` |
| view-palette · The full set of shared coloured styles is still exactly five groups | render `#[test]` — **carried**, must stay green | unit | `TestBackend` real | `cargo test ui::palette ui::view` |
| view-palette · A badged header row's colours survive the row's own role | render `#[test]` — **carried**, must stay green | unit | `TestBackend` real | `cargo test ui::palette ui::view` |
| view-palette · A delta badge and a clause keyword are the same style in one frame | render `#[test]` — **carried**, must stay green | unit | `TestBackend` real | `cargo test ui::palette ui::view` |
| view-palette · `style_for` maps each `DeltaOp` to its own role | render `#[test]` — **carried**, must stay green | unit | `TestBackend` real | `cargo test ui::palette ui::view` |
| view-palette · `Selected` reverses and colours nothing | render `#[test]` | unit | `TestBackend` real | `cargo test ui::palette ui::view` |
| view-palette · Faces reach the buffer as coloured styles at both mandated widths | render `#[test]` — **carried**, must stay green | unit | `TestBackend` real | `cargo test ui::palette ui::view` |
| view-palette · A section header's role is selected by its kind, not by its face | render `#[test]` — **carried**, must stay green | unit | `TestBackend` real | `cargo test ui::palette ui::view` |
| view-palette · Heading foreground wins over a code span inside it | render `#[test]` — **carried**, must stay green | unit | `TestBackend` real | `cargo test ui::palette ui::view` |
| view-palette · A plain face is the default style | render `#[test]` — **carried**, must stay green | unit | `TestBackend` real | `cargo test ui::palette ui::view` |
| view-palette · The two new face fields compose in their stated positions | render `#[test]` — **carried**, must stay green | unit | `TestBackend` real | `cargo test ui::palette ui::view` |
| view-palette · A checklist row reaches the buffer with its label coloured | render `#[test]` — **carried**, must stay green | unit | `TestBackend` real | `cargo test ui::palette ui::view` |
| view-palette · A selected cell keeps its own role and gains the reversal | render `#[test]` | unit | `TestBackend` real | `cargo test ui::palette ui::view` |

## Decisions

**Decision 1 — Partition by region, never disambiguate by motion.** `ui::layout::zone` already
answers "what is under this point". A drag begins only on a non-header `Zone::DetailRow`.
*Alternative considered:* press/release disambiguation — dispatch a click on release only when
no motion intervened. Rejected: it changes the dispatch timing of all four existing click
bindings, and Copilot demonstrates the partition works without it.

**Decision 2 — Press counting, not a clock.** `NOBLOCK` sweeps `src/ui/` for `Instant::now`,
`SystemTime::now` and `.elapsed()`, and crossterm synthesises no double-click event.
Consecutive presses at one cell are counted. *Alternative considered:* timestamping at the
`EventSource` seam and passing a monotonic `u64` inward. Rejected as a seam change for one
gesture, when `list-selection`'s "second click opens" already establishes the state-based
idiom in this pane.

**Decision 3 — `Selection` is one `Dashboard` field carrying a granularity.** Armed, word, row
or span. Armed is the state after a first press: anchor and focus at one cell, drawn as
nothing. *Alternative considered:* a separate `last_click` field. Rejected — two fields that
must be cleared together are one field that cannot be forgotten.

**Decision 4 — A sibling of `detail`, not a field inside it.** `sync_detail` reloads `Detail`
wholesale on a tab switch, a selection change, or an adopted refresh. A selection living
inside it would be *silently discarded* by a reload rather than *deliberately cleared* by one,
which is the difference between a rule and an accident.

**Decision 5 — One `Action` variant carrying the phase.** Each variant costs a `Mouse`
inventory row, and each row moves `content_rows`, `binding-inventory`'s counts and
`help-overlay`'s arithmetic. One row reading "select text in the artifact area" is also what a
reader needs; three phase rows would describe an implementation.

**Decision 6 — The pane claims nothing about the clipboard.** An OSC 52 write cannot be
acknowledged, so a terminal that ignores it is indistinguishable from one that honoured it.
The pane therefore renders no "copied" message. The **persisting highlight** is the only claim
it makes: this is what was selected. A write that fails *locally* is observable and is
reported. *Alternative considered:* a footer confirmation. Rejected as a claim the pane cannot
support.

**Decision 7 — The highlight composes with the underlying role.** A selected cell is its own
role's style patched with `Role::Selected`, which is `REVERSED` and carries no colour.
*Alternative considered:* draw selected cells as `Role::Selected` alone. Rejected — it would
flatten headings, code spans and delta badges into one undifferentiated block, erasing exactly
what the content area exists to show.

**Decision 8 — Selection works on non-foldable artifacts.** `detail_row_click` short-circuits
on `!detail.foldable()` and returns `Ignore`. Selection must not inherit that: a
single-section artifact is a whole rendered document and the likeliest thing to copy. Stated
because inheriting it would be the path of least resistance.

**Decision 9 — `Target::DetailLine` is removed, not left unused.** A variant no resolver emits
rots, and the crate's `NODEFAULT-UI` discipline is built on exhaustive matches that would keep
demanding an arm for it.

**Decision 10 — No auto-scroll at the edges.** The focus clamps to the content area. This
bounds the change, keeps `detail.scroll` out of the drag path entirely, and leaves a
well-defined follow-up. Copilot's behaviour here is unmeasured, so copying it would be guessing.

**Decision 11 — The clipboard write reaches the terminal through a new `run_loop` parameter.**
`run_loop` takes no `TerminalOps` handle today: the guard is built in `ui::run` and never
travels inward, so "call `write_clipboard` on completion" had no reachable call site. A
`ClipboardWriter<'a> = &'a dyn Fn(&str) -> Result<(), String>` is threaded in beside
`ArtifactReader`, which solves exactly this problem in exactly this way and is already tested
with a closure. *Alternative considered:* record a request on `Dashboard` and let the loop
drain it, as `launch.pending` does. Rejected — it needs a field, and the sixteen-field pin
forbids one.

**Decision 12 — A failed write is reported on `Selection`, not on an existing problem list.**
All five `!`-marked lists are replaced wholesale on their own producer's cadence, so a reason
put in one would vanish before the reader saw it; and no dedicated field is available. The
reason is created and cleared at exactly the moments it becomes and stops being true, which is
the definition of belonging to `Selection`. It renders as a **detail-region** row beside
`detail.problems`, because the gesture that produced it is a detail-region gesture.

## Risks / Trade-offs

- **A slow second press at one cell still widens.** No clock means no timeout. → Accepted on
  the same terms `list-selection`'s "second click opens" already accepts it; that rule has
  shipped without complaint.
- **A refresh can move the rows under a selection.** The live tier replaces the change set on
  its own cadence. → The selection is cleared whenever the detail content reloads. A cleared
  selection is honest; a stale span pointing at different text is not.
- **The clipboard is shared mutable state on the developer's machine.** → No test touches the
  real clipboard at any tier; `TerminalOps` is replaced everywhere, and the `NORAW`-style
  confinement keeps the real write in one file.
- **Five pinned counts move at once** — `Dashboard` fields to 16, `Action` to 25, bound
  actions to 23, `INVENTORY` to 32, `content_rows` to 43, `Target` down to 3. → Each is
  asserted in `cargo test` and the failure names both sides. The task list must move each
  count **in the same group as the code that changes it**, or `make check` is red between
  groups — the defect that deadlocked the previous plan.
- **`?1003` becomes load-bearing.** It was previously dead cost a future change might drop. →
  Recorded as a non-goal reversal in the proposal, so a later dead-code sweep does not remove
  the mode this feature consumes.

## Migration Plan

No migration, no backfill, no rollback procedure. Additive except for one removed `Target`
variant with a keyboard equivalent that predates it. Deploy order is a single `make build`;
the manifest is untouched.

## Open Questions

None. The proposal's three are closed: Decision 5 settles the variant count, Decision 6 the
unacknowledgeable write, and the refresh interaction is resolved by clearing on reload
(Decision 4 and the second risk above).
