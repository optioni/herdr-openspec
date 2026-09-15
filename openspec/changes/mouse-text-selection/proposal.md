## Why

You cannot select text in the dashboard pane with the mouse. `mouse-input` enabled mouse
capture, and a terminal with reporting on routes drags to the application instead of doing
native selection. The pane gained click, wheel, fold and tab-switch; the reader lost copying
a requirement out of a spec.

This proposal was scoped as an investigation. The investigation is finished, and it changed
the answer twice. Everything below rests on `notes/measurements.md`, not on report.

**What was measured.** Native drag-selection and mouse reporting are genuinely exclusive:
`?1000` alone — press and release, no motion requested — still suppresses selection, and
there is no narrower request to make. `Shift`+drag works under every mode set, which
falsifies a sentence already in `specs/mouse-input/spec.md` claiming `Option` on macOS.
Herdr is not involved.

**What that first looked like, and why it was wrong.** The obvious conclusion was a key that
releases capture on demand. It was the wrong conclusion, because GitHub Copilot CLI has both
working at once — and the reason is not a mode set. Captured through a pty, the terminal
delivers Copilot a complete drag: a press, eighteen consecutive `left+motion` reports, a
release, with the `Shift` bit set on **0 of 79** reports. Its own output carries two
`ESC]52;` clipboard writes and sixty-five reverse-video sequences. Copilot holds
`?1003 ?1006` for the whole session, consumes the drag, paints its own highlight, and copies
through OSC 52. What looks native is a reimplementation.

**And it partitions the screen rather than disambiguating gestures.** In Copilot a drag
cannot select the top two rows; it starts at the third. Those rows are its tab bar. Regions
that take clicks do not take selection, and there is no press-versus-drag rule at all.

This pane is unusually well placed to copy that, because the partition already exists as
tested code. `ui::layout::zone` resolves any point in the frame to exactly one of six
`Zone` values, and the split falls out of them.

## What Changes

- **The detail region's content area becomes selectable.** A left drag beginning on a
  `Zone::DetailRow` that is not a section header starts a text selection, extends with
  motion, and completes on release.
- **Every other region keeps its click, unchanged.** `Zone::ListRow` still selects a change,
  opens it, and folds a section; `Zone::DetailTab` still switches tab; a `Zone::DetailRow`
  that **is** a section header still folds. No binding changes its dispatch timing, because
  the selectable region and the clickable regions do not overlap.
- **One existing binding gives way:** `Target::DetailLine`, which moves the detail cursor to
  a clicked line. It is the weakest click in the pane, and it is the only one inside the
  region that becomes selectable.
- **The selection is painted by the pane**, as a reverse-video span over the selected cells,
  through a palette role of its own.
- **The selected text is copied on release, through OSC 52.** The text comes from
  `ui::detail::content_lines`, which already returns `Vec<ContentRow>` from a pure function —
  no `ratatui::buffer::Buffer` is read back.
- **`SPEC.md`'s bypass sentence is corrected** from `Option` to `Shift`, scoped to what was
  measured, and bound by a doc-conformance leg so it cannot silently regress.
- Not **BREAKING** for the keyboard: no key changes, and no mouse gesture outside the detail
  content area behaves differently.

## Non-Goals

- **No toggle key.** Two designs were considered and discarded on evidence: releasing capture
  on demand, and inverting the default. The partition needs neither.
- **No auto-scroll at the edges.** A drag that runs past the top or bottom of the content
  area does not scroll it; a selection is bounded by what is on screen. This bounds the
  change, and Copilot's own behaviour here is unmeasured. A follow-up can lift it.
- **No selection in the list region, the tab bar, or on a section header row.** Those are the
  clickable half of the partition, and blurring it reintroduces exactly the disambiguation
  problem the partition avoids.
- **No selection across a fold boundary being "expanded"** — what is selected is what is
  drawn, which is what the reader sees.
- **No narrowing of the capture mode set.** Measured to buy nothing for selection. Dropping
  `?1002`/`?1003`/`?1015` remains defensible as dead-cost removal and is a separate change —
  except that `?1003` is now **load-bearing**, since drag motion is what this change consumes.
- Windows terminals, out of scope repository-wide.

### Two non-goals this proposal explicitly reverses

The investigation-era version of this document ruled both of these out, before any of it was
measured. They are reversed deliberately, and they are the whole mechanism:

1. *"Implementing selection, a clipboard, or a copy buffer inside the TUI. Rendering a
   selection overlay and owning copy is a large feature and the wrong one."* It is a large
   feature. It is not the wrong one: it is the only thing that delivers selection and clicks
   together, and it is what the comparison program does.
2. *"OSC 52 clipboard writes, or any escape sequence that reaches outside the pane."* A
   clipboard write is the point. It is confined to `src/ui/terminal.rs`, which already owns
   every terminal escape in the crate.

## Capabilities

### New Capabilities

- `text-selection`: the drag state machine, the region partition that decides where a drag
  may begin, what the selected span covers, and what is copied.

### Modified Capabilities

- `mouse-input`: left drag and left release are resolved rather than ignored; the documented
  gesture table grows; the falsified `Option` bypass sentence is corrected.
- `dashboard-loop`: new `Dashboard` fields for the drag anchor and focus (**15** today, by
  `awk '/^pub struct Dashboard/,/^}/' src/ui/app.rs | grep -cE '^\s+pub [a-z_]+:'`), and new
  `Action` variants (**24** today, by the same method over `pub enum Action`).
- `binding-inventory`: the action sweep's pinned pair — `union.len() == 24`,
  `bound.len() == 22` — and the test's own name, plus a `Mouse` group row per new action.
- `help-overlay`: `apply_help_action` is an exhaustive match with no wildcard, and its
  requirement asserts "seventeen plus seven is the twenty-four `Action` carries". Both move.
  If `INVENTORY` grows, `content_rows` moves from **42** (`src/ui/help.rs:290`) with it.
- `view-palette`: one new `Role` for the selection highlight (**30** today).
- `terminal-lifecycle`: `TerminalOps` gains a clipboard write (**six** methods today).
- `artifact-content`: `ContentRow` is the source of the copied text.
- `list-selection`: pins `Target::DetailLine`, the binding that gives way.
- `doc-conformance`: a twelfth claim, and `AGENTS.md`'s "eleven further claims" with it.
- `degraded-coverage`: a refused or unsupported clipboard write is a new degraded-states row.

## Impact

- `src/ui/driver.rs` — drag resolution. **`run_loop` lives here**, not in `src/ui/mod.rs`.
- `src/ui/app.rs` — the actions, the drag state, and applying it.
- `src/ui/detail.rs` — the selected span over `ContentRow`s, and extracting its text.
- `src/ui/view.rs` — painting the highlight.
- `src/ui/palette.rs` — the role.
- `src/ui/terminal.rs` — the OSC 52 write, behind `TerminalOps`.
- `src/ui/help.rs` — the inventory rows and the row count.
- `SPEC.md`, `README.md`, `AGENTS.md` — keys, gestures, the corrected bypass, the claim count.
- `tests/doc_contract.rs`, `tests/degraded-coverage.toml` — the moved counts and the new rows.
- No dependency change. No I/O added to a pure view file.

## Open Questions for Review

1. **How many `Action` variants the drag needs.** Each one costs a `Mouse` group row, and
   each row moves `content_rows`, `binding-inventory`'s counts, and `help-overlay`'s
   arithmetic. One phase-carrying variant is cheaper than three; whether it is clearer is a
   judgement design.md must make and defend.
2. **What a terminal that ignores OSC 52 should show.** The write cannot be confirmed, so the
   pane cannot know it failed. Silence risks the reader believing a copy happened.
3. **Whether the selection survives a refresh.** The live tier can replace the change set
   mid-drag, and the rows under the selection can move.
