## Why

The change list gives permanent, equal weight to a growing archive nobody navigates. It
is one flat run — problem rows, active changes, a separator, archived changes — and the
archive only ever grows: every change this project completes becomes another permanent
row. In this repository the ratio is already 21 archived to 9 active, and the separator
is not addressable, so there is no way to put the archive away. The only tool for
shrinking the list is `/`, which is a search, not a fold: a reader who wants to stop
seeing 2026-09-04 has to type a query that excludes it.

A reader opens this pane to see what is in flight. History should be one keystroke away,
not permanently in the way.

This is **unplanned work**. `openspec/IMPLEMENTATION-ORDER.md` ends at Phase 6
(`degraded-states`) and the roadmap is complete. `list-view` built the flat list against
a repository that had almost no archive yet, so the plan never asked what the list looks
like after twenty changes land.

## What Changes

- **BREAKING — one new keybinding.** `Space` toggles the section the cursor is on or in.
  No existing key moves. While filtering, `Space` types itself into the query like every
  other printable key, so the filter's "printable keys type" rule is unchanged.
- **Two section-header rows replace the single separator**: `v active (9)` and
  `> archived (21)`, a new `RowKind::Section`. **Archived starts collapsed; active starts
  expanded.**
- **Collapse state lives on `Dashboard`**, not derived per frame like `LayoutMode` and the
  scroll offset. It is a user decision, and a decision the next frame must not discard.
- **A collapsed section emits its header alone.** Its changes are not rows, are not
  addressable, and do not count toward `RowKind::Item { index }` — which `change-rows`
  already documents as an index into the *visible* list, never into `ChangeSet`. This
  change relies on that promise rather than weakening it.
- **A non-empty `/` query forces both sections open** for as long as it is non-empty, and
  restores the reader's own collapse state when it clears. A filter that silently hides
  matches is worse than a long list.
- **Section headers are selectable**, so `Space` has something to act on and `j`/`k` can
  reach it.
- **Every new cell is measured in display columns**, through `view-fidelity`'s
  `ui::layout::columns` / `truncate_columns`. No `char`-counted arithmetic is added, and
  the new rows are covered by `colwidth.sh` like every existing one.

## Non-Goals

- **No change to what a change row contains** — its cells, its right-alignment, or its
  measurement. `view-fidelity` owns the row grammar's arithmetic and this change adds no
  competing site.
- **No persistence.** Collapse state is per-session. Nothing new is written under
  `HERDR_PLUGIN_STATE_DIR`, which stays exactly `agent-names.toml`.
- **No mouse.** Clicking a section header belongs to `mouse-input`, which depends on this
  change for the target.
- **No new grouping, sorting, or filtering dimension.** Active and archived are the two
  sections OpenSpec itself defines; this change adds no third.
- **No empty-state rewording.** `No changes yet`, `No active changes`, and the two-row
  `No changes match` state are unchanged.
- Crosses no PRD non-goal: nothing is written inside `openspec/`, no change is authored,
  nothing orchestrates across changes, no Windows.

## Capabilities

### New Capabilities

None. Collapsing is a property of the rows emitted and of what is selectable, and both
already have owners.

### Modified Capabilities

- `change-rows`: the separator becomes two section-header rows carrying a glyph, a label,
  and a count; a collapsed section suppresses its change rows; the emitted order is
  restated with sections in it.
- `list-selection`: a section header is addressable, and `Space` toggles the section the
  cursor is on or in.
- `list-filtering`: a non-empty query forces both sections open and the reader's own
  collapse state is restored when it clears.
- `dashboard-loop`: `Space` gains an `Action` outside filter mode and types itself inside
  it.

## Impact

- **Code:** `src/ui/list.rs` (row emission), `src/ui/app.rs` (collapse state, key mapping),
  `src/ui/driver.rs`, `src/ui/view.rs` (section-header styling).
- **Docs:** `SPEC.md` → List view and → Keys; `AGENTS.md`'s list-region description.
- **Depends on `view-fidelity`.** Its `columns`/`truncate_columns` primitives and its
  `colwidth.sh` gate must exist first, or this change adds row-grammar sites in the exact
  arithmetic that change exists to remove.
- No manifest, no config format, no dependency, no data model, no external service, no
  sibling repository.
