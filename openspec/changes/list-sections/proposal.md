## Why

The pane hides most of the archive and does not say so. `Config::archived_count` defaults
to `5`, and `changes::from_files` truncates the archived tier to it before anything is
rendered. Measured in this repository today: **22 archived changes on disk, five in the
pane, seventeen invisible with no row, badge, or count anywhere admitting it.** A reader
who knows `agent-polling` was archived and cannot find it has no way to tell whether it is
missing because of a cap, a filter, or a bug.

That cap exists for a reason, and the reason is the shape this change fixes. The list is
one flat run — problem rows, active changes, a separator, archived changes — where the
separator is not addressable and nothing folds. With no way to put the archive away,
truncating it was the only lever available, so the configuration key is standing in for a
missing interaction. Give the list a fold and the cap stops being necessary.

The `/` filter is not that lever either: it is a search, not a fold. A reader who wants to
stop seeing `2026-09-04` has to compose a query that excludes it.

This is **unplanned work**. `openspec/IMPLEMENTATION-ORDER.md` ends at Phase 6
(`degraded-states`) and the roadmap is complete. `list-view` chose the cap against a
repository that had almost no archive yet, when five of five was the whole archive and
truncation cost nothing; the plan never asked what the key would mean after twenty changes
landed.

## What Changes

- **BREAKING — one new keybinding.** `Space` toggles the section the cursor is on or in.
  No existing key moves. While filtering, `Space` types itself into the query like every
  other printable key, so the filter's "printable keys type" rule is unchanged.
- **BREAKING — `archived_count` stops limiting what is rendered.** An expanded archived
  section shows **every** archived change, regardless of the key's value; a collapsed one
  shows none. The key is still accepted and still parsed, so no existing `config.toml`
  becomes invalid and no reader loses a setting to a hard error — but it no longer
  truncates the list, and it is documented as having no effect on it. This is a change to
  the plugin configuration format's semantics, which is why it is marked here.
  Marked as an interface change, not as a loss: the reader who set `archived_count = 25`
  wanted the archive visible, and gets it.
- **Two section-header rows replace the single separator**: `v active (9)` and
  `> archived (21)`, a new `RowKind::Section`. **Archived starts collapsed; active starts
  expanded.**
- **Collapse state lives on `Dashboard`**, not derived per frame like `LayoutMode` and the
  scroll offset. It is a user decision, and a decision the next frame must not discard.
- **A collapsed section emits its header alone.** Its changes are not rows, are not
  addressable, and do not count toward `RowKind::Item { index }` — which `change-rows`
  already documents as an index into the *visible* list, never into `ChangeSet`. This
  change relies on that promise rather than weakening it.
- **The archived header carries the true total**, not the rendered count: `> archived (22)`
  where twenty-two exist. The count is what makes the fold honest — a reader can see how
  much is behind it before deciding to open it, which is exactly what the cap never told
  them.
- **A collapsed section costs no work, not just no rows.** `changes::from_files` truncates
  the archived tier *before* resolving schemas, artifacts, and task counts, so lifting the
  cap outright would make every refresh cycle resolve every archived change — four times
  the archived-tier file work in this repository, and unbounded in a larger one. A
  collapsed section needs names and a count only. The change SHALL NOT make a collapsed
  pane pay for an expanded one; `design.md` chooses the mechanism.
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
- **`archived_count` is not removed, renamed, or repurposed.** It keeps parsing, keeps its
  default of `5`, and keeps reporting a malformed value on `Config::problems`; it simply
  stops truncating. Deleting a configuration key is a migration this change does not owe,
  and repurposing one silently is worse than leaving it inert.
- **No change to the archived tier's ordering.** Date descending, same-date by name
  descending, undated last, exactly as `change-enumeration` states — the fold changes how
  many are shown, never which or in what order.
- **No mouse.** Clicking a section header belongs to `mouse-input`, which depends on this
  change for the target.
- **No new grouping, sorting, or filtering dimension.** Active and archived are the two
  sections OpenSpec itself defines; this change adds no third. In particular it does not
  group the archived tier by date, which was considered and deferred: every archived row
  already carries a ten-column date field, no archived name yet exceeds the narrow
  layout's 19-column name field (longest is 18), and a date grouping's value depends
  entirely on how the archive happens to cluster in time. The section model SHALL
  nonetheless leave the door open — a section is identified by a key and carries a nesting
  depth, rather than the two booleans this change's own behaviour would need — so a later
  change can nest date groups under `archived` without reworking the collapse state,
  the selection index, or the filter's force-open rule.
- **No empty-state rewording.** `No changes yet`, `No active changes`, and the two-row
  `No changes match` state are unchanged.
- Crosses no PRD non-goal: nothing is written inside `openspec/`, no change is authored,
  nothing orchestrates across changes, no Windows.

## Capabilities

### New Capabilities

None. Collapsing is a property of the rows emitted and of what is selectable, and both
already have owners.

### Modified Capabilities

- `change-model`: `ChangeSet` gains `archived_total`, the archive's true size, because
  `archived` is no longer always populated and a collapsed section's header still has to say
  how many changes are behind it. Not predicted when this proposal was first written — it is
  the field the "a collapsed section costs no work" rule above forces — and added here rather
  than left for `design.md` to smuggle in.
- `change-rows`: the separator becomes two section-header rows carrying a glyph, a label,
  and a count; a collapsed section suppresses its change rows; the emitted order is
  restated with sections in it. `RowKind::Separator` is replaced by
  `RowKind::Section { key, depth, collapsed }`, which keeps the separator's palette role.
- `list-selection`: a section header is addressable, and `Space` toggles the section the
  cursor is on or in.
- `list-filtering`: a non-empty query forces both sections open and the reader's own
  collapse state is restored when it clears.
- `dashboard-loop`: `Space` gains an `Action` outside filter mode and types itself inside
  it.
- `change-enumeration`: the archived tier is no longer truncated to `archived_count`; the
  ordering is unchanged and the full count is carried to the view.
- `refresh-worker`: `archived_count` leaves the worker's signature, and the archived tier's
  resolution becomes conditional on the section being open.
- `plugin-config`: `archived_count` is documented as accepted-but-inert rather than as the
  archived-list limit; its default, its type check, and its problem reporting are unchanged.

## Impact

- **Code:** `src/ui/list.rs` (row emission), `src/ui/app.rs` (collapse state, key mapping),
  `src/ui/driver.rs`, `src/ui/view.rs` (section-header styling), and — new since the cap
  decision — `src/changes.rs` (the truncation, the carried total, and `ArchivedScope`) and
  `src/refresh.rs` (the signature and the conditional resolution). This change is no longer
  confined to `src/ui/`, which is worth stating plainly: it now crosses into the data layer,
  and the "views do no I/O" boundary is what keeps that crossing honest.
- **Gate floors:** `NODEFAULT-UI`'s view-layer type set gains `Sections`, so that leg's
  `SCAN_MIN` on the `Makefile` line moves with it, and `notes/gate-floors.md` records the
  new measurement beside the five it already holds.
- **Docs:** `SPEC.md` → List view, → Keys, and → Resolution chain's `config.toml`
  description; `README.md`'s configuration table; `AGENTS.md`'s list-region description.
- **Depends on `view-fidelity`.** Its `columns`/`truncate_columns` primitives and its
  `colwidth.sh` gate must exist first, or this change adds row-grammar sites in the exact
  arithmetic that change exists to remove.
- No manifest, no config format, no dependency, no data model, no external service, no
  sibling repository.
