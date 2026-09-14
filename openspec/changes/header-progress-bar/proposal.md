## Why

Task progress is the number you most want while reading a change, and it is the one thing that
disappears when you leave the tasks tab. `ui::detail::header_row` does show a compact progress
*cell* on every tab (`ui::list::progress_cell`, e.g. `3/10`), but the **bar** —
`ui::tasks::progress_bar`, the thing you can read at a glance without parsing two numbers — is
drawn only by the tasks tab's own body. Reading `proposal.md` or `design.md` therefore costs
you the sense of how far along the change is, and getting it back means a tab switch and a tab
switch home.

Unplanned work past Phase 6: a legibility gap the roadmap never scoped, surfaced by using the
dashboard rather than by a defect.

## What Changes

- The detail region's heading area gains a progress **bar** for the selected change, visible on
  every artifact tab rather than only the tracked-tasks one.
- The bar and the numeric cell state the same `Change::progress`, so the two can never
  disagree; whether the numeric cell survives beside the bar, and at which widths, is the
  layout question this change answers.
- The bar degrades by width like every other header cell: `header_row` already drops cells
  **whole** rather than truncating them (schema first, then progress), and the bar joins that
  ordering rather than inventing a second rule.
- A change whose schema tracks no tasks, or whose tasks artifact could not be read, shows no
  bar — not an empty or zero-length one.
- Not **BREAKING**: no manifest, config, or keybinding change.

## Non-Goals

- Changing what `progress` *means* or where it comes from. The dual-source rule stands: the
  CLI's `completedTasks`/`totalTasks` pair corrects the file-sourced count, and this change
  reads the result rather than recomputing it.
- The tasks tab's own body, which keeps its bar and its checklist grammar unchanged.
- The list region's per-row progress cells.
- Any new colour. The bar asks `palette::style(Role::…)` like everything else.
- Making the bar interactive, clickable, or a scrub target.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `detail-header`: the heading gains the gauge cell and the width-degradation ordering that
  governs it.
- `tasks-progress-bar`: the `█`/`░` run becomes a function two call sites render, not one, so
  its contract must say what it owes a caller that is not the tasks tab.

`responsive-layout` was listed here before the shape below was settled and is **not** modified:
the mandated 78- and 58-column detail interiors do not move, the gauge's budget is fixed by
`detail-header` rather than by a layout split, and the display-column rule the gauge inherits is
already that capability's standing one. Writing a delta for it would say nothing.

## Impact

- `src/ui/detail.rs` — `header_row` and the heading's layout.
- `src/ui/tasks.rs` — `progress_bar`'s contract at a second call site; no behaviour change
  expected to its existing one.
- Possibly `src/ui/layout.rs` if the heading needs a width split it does not already have.
- View tests at 60 and 120 columns, and the 78/58-column interior assertions.
- No dependency, manifest, or gate change. No process spawn, no I/O — this is a pure view change
  on both sides of the seam.

## Settled: the shape and the width split

The open question this proposal raised — whether the bar **replaces** the numeric cell or sits
beside it — was resolved before the specs artifact, in favour of **beside**, in the one form
where that is coherent.

`ui::tasks::progress_bar` already *contains* the numeric cell (`gauge ␣ [n/m] ␣ pct%`), so
"beside" cannot mean calling it: the header would state `[4/9]` twice. What the header renders
instead is the **bare gauge run** — `tasks::gauge_of`, raised from private to `pub(crate)` — as
a fourth cell in the existing grammar, with no percent cell at any width.

- **Drop order: gauge, then schema, then numeric.** The new cell goes *first* in the order, so
  the two existing cells keep their relative positions and every width band below the full
  form produces byte-identically what it produced before. Below 26 columns this change is not
  observable at all.
- **Fixed 12-column gauge budget; the name field absorbs the remainder.** The gauge reads the
  same at 58 and at 120 columns, and every column a wider frame brings goes to the name — the
  field already being squeezed at 58. A proportional split was rejected for turning every width
  assertion into a computed expectation; a name minimum with a remainder gauge was rejected for
  leaving a long name truncated at 120 columns where there is plainly room for it.
- **No gauge at `total == 0`,** and no separating space reserved for one, so a change with no
  tasks renders exactly the row it renders today.

`progress_bar` itself is untouched — this is the correction to the Capabilities note above.
