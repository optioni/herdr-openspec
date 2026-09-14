## Why

The tasks tab renders every line with equal weight, which is the opposite of what you need: a
change is mostly *done* by the time you are reading it, and the done part is the part you no
longer care about. Three signals already exist in the data and none of them reach the screen.

- **Per-group progress.** `tasks::Group::progress()` (`src/tasks.rs:96`) already computes each
  group's own checkbox count. `ui::tasks::lines` renders the heading text and discards it.
- **Lifecycle kind.** Every group carries `<!-- kind: behavior | refactor | operational -->`.
  `tasks::parse` recognises only headings and checkbox lines (`src/tasks.rs:152-156`), so the
  marker is dropped: you cannot tell a behavior group from an operational one in the pane.
  **Dropped from this change's scope** — see the resolved questions below. It stays a real
  signal and a later change may take it; this one does not, because a badge did not earn a
  cell on a heading row that now also carries a progress pair.
- **Task labels.** 1,989 labelled tasks across the archive — `VERIFY` 557, `CHECK` 427, `RED`
  329, `GREEN` 313, `CHANGE` 212, `REFACTOR` 140, `CHARACTERIZE` 11 — rendered as plain text.

## What Changes

- **Completed items are de-emphasised**, label included, so a finished row reads as finished
  rather than leaving a bright `VERIFY:` on it. Read-only still: no key toggles an item.
- **Each group heading carries its own progress**, which is also what lets `heading-sections`
  fold a completed group without the bar disagreeing with the folds. Since that change
  archived, a real `tasks.md` group heading is a **fold header row**, so the progress cell
  lands there — right-aligned, dropped whole when it does not fit — and this change therefore
  carries an `artifact-folds` delta the first draft did not anticipate.
- **The progress bar is segmented by group.** Constraints from the measurement: the gauge gets
  `width - 11` (the count and percent cells take the rest), so ~67 columns at the wide interior
  and ~47 at the narrow one; at 22 groups that is 3 and 2.1 columns each. So boundaries are
  marked by **alternating shade, not separator characters** — 21 separators would eat 21 of 47
  narrow columns — and segments are sized **proportional to item count**, not equally.
  Unsegmented below whatever width the design fixes.
- **Task labels are styled by lifecycle position, not by word.** The schema's own instruction
  says the three lifecycles are parallel — "RED before GREEN for behavior, CHARACTERIZE before
  REFACTOR for refactors, and CHECK before CHANGE for operational work" — so the vocabulary is
  three positions wearing different names:

  | Role | Keywords |
  |---|---|
  | Evidence | RED, CHARACTERIZE, CHECK, GIVEN, ARRANGE |
  | Change | GREEN, REFACTOR, CHANGE, WHEN, ACT |
  | Confirm | VERIFY, THEN, ASSERT |
  | Label | any other leading ALL-CAPS run, per `task-labels`' recognition rule |

  Four roles rather than seven-plus. `RED` and `GREEN` land on red and green naturally because
  they sit in different families.
- Not **BREAKING**: no manifest, config, or keybinding change.

## Non-Goals

- Editing or toggling tasks. The plugin does not write inside `openspec/`.
- Folding — that is `heading-sections`, which this change depends on for the group sections it
  decorates.
- Changing `tasks::parse`'s counting rule or its grouping, which must keep agreeing with the
  CLI's. `label_of` reads an item's text and changes nothing about what is parsed or counted.
- A schema-specific keyword list. The four roles above are a **general testing vocabulary**,
  not `tdd`'s task prefixes: a schema using ARRANGE/ACT/ASSERT is styled identically, and an
  unrecognised token still gets the generic `Label` role. Nothing privileges the active schema.
- Per-word colours. Seven-plus hues is a rainbow nobody learns.

## Capabilities

### New Capabilities

- `task-labels`: recognising a task's leading label, classifying known testing vocabulary into
  the three lifecycle positions, and falling back to the generic role. Lives in `src/tasks.rs`
  rather than under `src/ui/`, so the pure-view file count two capability specs bind does not
  move.

### Modified Capabilities

- `tasks-checklist`: completed items de-emphasised; labels split out and styled.
- `tasks-progress-bar`: the gauge is segmented by group, proportionally.
- `artifact-folds`: `ArtifactSection` carries an optional `progress`, and a section header row
  carries a right-aligned progress cell when it has one. This is where a group heading's own
  progress lands — on a real `tasks.md` the group headings are **fold headers**, not
  `Face { heading }` lines, since `heading-sections` archived.
- `view-palette`: five new roles.

## Impact

- `src/tasks.rs` — `LabelRole` and `label_of`; `Group::progress()` reaches a caller at last.
- `src/ui/tasks.rs` — item styling, the segmented gauge, the group-progress slice.
- `src/ui/markdown.rs` — `Face` gains two fields `ui::markdown` itself never sets.
- `src/ui/app.rs` / `src/ui/detail.rs` — `ArtifactSection::progress` and the header row's cell.
- `src/ui/palette.rs` — five roles. Colour literals stay in its own tests.
- View tests at 60 and 120 columns; colour asserted against `palette::style(role)`.
- No dependency, manifest, or seam change. No new file under `src/ui/`, so the pure-view
  count stays at ten.

## Resolved before specs were written

The three questions this proposal raised were answered before `specs/` was written. The
answers are requirements now, not options; the reasoning is kept so a reviewer can disagree
with the decision rather than re-derive it.

1. **`Evidence` does not share red with `Role::ListProblem`.** It takes **`LightRed`**, which
   shares instead with `AgentBadge(Blocked)` — and that share is the *safe* one: an agent badge
   is drawn only in the list region, while a problem row is drawn in the **detail** region, the
   same region as a task label. The justification is the palette's own existing one for
   `FileMode`/`Code`: two roles may share a style when they cannot meet. The four roles are
   `Evidence` `LightRed`, `Change` `Green`, `Confirm` `Blue`, `Label` `DarkGray`. `Blue` is free
   on this path: `ui::tasks` emits no link face at all, and a level-3 heading occurs once in 37
   archived task files.

2. **Segments are proportional to item count**, not equal. Boundaries are alternating shade,
   never separator characters. Segmentation changes *which glyph pair* each column is drawn
   with and never how many columns are filled, so `tasks-progress-bar`'s existing
   "full exactly when complete" and saturation contracts are untouched by construction.

3. **No `kind` badge**, and `kind` leaves this change's scope entirely. The `task-groups` delta
   is not written: retaining a marker nothing renders would be spec'ing dead data. Group
   heading rows carry their own progress and nothing else.

A fourth decision fell out of the first three. The proposal said "four new roles"; it is
**five**. De-emphasising a completed item needs a role of its own (`Muted`, plain `DIM`) beside
the four label roles, and reusing `Quoted` for it would mean a block quote and a finished task
were the same thing.

## Measurements

Run at HEAD on 2026-09-14 over `openspec/changes/archive/*/tasks.md`.

| Figure | Result |
|---|---|
| Task items in the archive | 2563 |
| Items carrying a recognisable label | 2272 |
| Items whose label is `<RUN>:` with the colon immediately following | 2196 |
| Items whose label is a compound form (`CHANGE — rewrite in \`SPEC.md\`:`, `RED then GREEN:`, `RED→GREEN:`, `RED-by-addition:`, `CHECK (contract gate):`) | 76 |
| Items with a leading uppercase run that are **not** a label | 0 |
| Distinct recognised tokens | `VERIFY` 609, `CHECK` 471, `RED` 358, `GREEN` 348, `CHANGE` 236, `REFACTOR` 157, `CHARACTERIZE` 16, `NOTE` 1 |
| Groups per task file | n=37, min 5, median 12, max 22 |

The last row is why the segmentation threshold is stated in columns rather than assumed: at 22
groups and the narrow interior's ~47 gauge columns, a segment is 2.1 columns.
