## Why

The pane is a mouse-shaped surface that ignores the mouse. It sits in a Herdr split beside
other panes the reader is already clicking between, it draws a list of clickable-looking
rows and a bar of clickable-looking numbered tabs, and it responds to none of it:
`ui::app::action_for` maps every `Event::Mouse` to `Action::Ignore`, and mouse capture is
never enabled, so no mouse event reaches the loop at all.

The gap is widest exactly where the keyboard is worst. Reaching the eleventh archived
change means eleven `j` presses or a `/` query; reaching the fourth artifact tab means
knowing it is the fourth. Both are one click. Scrolling a long spec — the pane's single
most common activity — is a held `j`.

This is **unplanned work**. The roadmap ends at Phase 6 (`degraded-states`). `tui-shell`
specified the event loop around `KeyEvent` and every later change inherited that shape;
no row of the plan ever asked whether the pane had a second input device.

## What Changes

- **BREAKING — mouse capture is enabled, and it costs drag-to-select.** With capture on,
  the terminal stops handling mouse gestures itself, so selecting and copying text out of
  the pane requires holding `Option` (macOS) or `Shift` (most Linux terminals). This is the
  change's one real regression. It is stated here, in the proposal, as an accepted cost —
  not discovered after the fact — because it affects a reader who never presses a mouse
  button.
- **`src/ui/terminal.rs` gains capture as a fifth and sixth operation**, entered and left
  in the same fixed, mirrored order the existing four keep, and released on normal return,
  error return, and panic alike. It remains the only file in the crate permitted to name a
  crossterm terminal-mode function.
- **A pure hit-test in `ui::layout`**: frame area, route, and a `(column, row)` pair to a
  `Target`. Pure `Rect` arithmetic and a row lookup, no I/O — the views' rule is not bent
  for input.
- **Wheel scrolls the region under the pointer** — the list's selection at the list region,
  the detail content at the detail region — which in the wide layout means the two regions
  scroll independently for the first time.
- **Click selects a list row; a second click on the selected row opens its detail; a click
  on an artifact tab switches to it; a click on a section header toggles it.**
- **The mouse acts while filtering**, unlike printable keys, because a click is unambiguous
  where a keystroke is not.
- **Nothing becomes mouse-only.** Every target keeps its key, no key moves, and the pane
  stays fully usable over SSH in a terminal that reports no mouse.

## Non-Goals

- **No drag of any kind** — no drag-select, no drag-scroll, no drag-resize of the 40-column
  split. Motion events are ignored, which also keeps the render loop from redrawing on
  every pointer move.
- **No hover styling**, for the same reason: a frame per pointer motion is a cost the pane
  does not need to pay for a cosmetic.
- **No right-click, no context menu, no mouse-driven agent launch.** `a`/`c`/`s`/`g` start
  processes and stay deliberate keystrokes; a mis-click must not launch an agent.
- **No text selection of the pane's own.** The terminal's selection is what capture
  displaces, and reimplementing it inside the pane is a far larger change than this one.
- **No new dependency.** Crossterm's mouse types are already reachable through
  `ratatui::crossterm`, which is how every other crossterm type in the crate is reached.
- **No I/O in a view.** The hit-test is a pure function; the event source stays
  `src/ui/event.rs`.
- Crosses no PRD non-goal: read-only, no change authoring, no orchestration, no Windows.

## Capabilities

### New Capabilities

- `mouse-input`: capture's lifecycle, the pure hit-test and its `Target` set, the
  wheel/click/second-click bindings, the filter-mode rule, and the requirement that every
  target remain reachable by key.

### Modified Capabilities

- `terminal-lifecycle`: the guard enters and leaves capture in the mirrored order, and
  releases it on the panic path.
- `dashboard-loop`: `Event::Mouse` stops mapping to `Action::Ignore`.
- `responsive-layout`: the hit-test primitive, and the rule that a point outside every
  region is a no-op.
- `list-selection`: a click selects; a second click on the selected row opens detail.
- `artifact-tabs`: a click switches tab.
- `detail-scroll`: the wheel scrolls the detail region, independently of the list.
- `change-rows`: a click on a section header toggles it.

## Impact

- **Code:** `src/ui/terminal.rs`, `src/ui/layout.rs`, `src/ui/app.rs`, `src/ui/driver.rs`,
  `src/ui/list.rs`.
- **Docs:** `SPEC.md` → Keys gains a mouse table and the drag-to-select cost;
  `AGENTS.md` → the terminal-seam rule's list of confined functions grows.
- **Depends on all three siblings and one audit change.** `view-fidelity` first, because a
  hit-test maps *terminal columns* to rows and one built on `char` counts mis-targets every
  row containing an emoji or a CJK name. `list-sections`, for the section-header target.
  `seam-resilience`, which is already rewriting the panic hook capture must release from.
  `color-palette` is independent of it.
- No manifest, no config format, no dependency, no data model, no external service, no
  sibling repository.
