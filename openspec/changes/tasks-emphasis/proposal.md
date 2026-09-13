## Why

The tasks tab renders every line with equal weight, which is the opposite of what you need: a
change is mostly *done* by the time you are reading it, and the done part is the part you no
longer care about. Three signals already exist in the data and none of them reach the screen.

- **Per-group progress.** `tasks::Group::progress()` (`src/tasks.rs:96`) already computes each
  group's own checkbox count. `ui::tasks::lines` renders the heading text and discards it.
- **Lifecycle kind.** Every group carries `<!-- kind: behavior | refactor | operational -->`.
  `tasks::parse` recognises only headings and checkbox lines (`src/tasks.rs:152-156`), so the
  marker is dropped: you cannot tell a behavior group from an operational one in the pane.
- **Task labels.** 1,989 labelled tasks across the archive — `VERIFY` 557, `CHECK` 427, `RED`
  329, `GREEN` 313, `CHANGE` 212, `REFACTOR` 140, `CHARACTERIZE` 11 — rendered as plain text.

## What Changes

- **Completed items are de-emphasised**, label included, so a finished row reads as finished
  rather than leaving a bright `VERIFY:` on it. Read-only still: no key toggles an item.
- **Each group heading carries its own progress**, which is also what lets `heading-sections`
  fold a completed group without the bar disagreeing with the folds.
- **Each group heading carries its `kind`** as a badge. Turning dropped text into signal.
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
  | Label | anything else matching ALL-CAPS + `:` at the start of a task |

  Four roles rather than seven-plus. `RED` and `GREEN` land on red and green naturally because
  they sit in different families.
- Not **BREAKING**: no manifest, config, or keybinding change.

## Non-Goals

- Editing or toggling tasks. The plugin does not write inside `openspec/`.
- Folding — that is `heading-sections`, which this change depends on for the group sections it
  decorates.
- Changing `tasks::parse`'s counting rule, which must keep agreeing with the CLI's.
- A schema-specific keyword list. The four roles above are a **general testing vocabulary**,
  not `tdd`'s task prefixes: a schema using ARRANGE/ACT/ASSERT is styled identically, and an
  unrecognised token still gets the generic `Label` role. Nothing privileges the active schema.
- Per-word colours. Seven-plus hues is a rainbow nobody learns.

## Capabilities

### New Capabilities

- `task-labels`: recognising a task's leading label, classifying known testing vocabulary into
  the three lifecycle positions, and falling back to the generic role.

### Modified Capabilities

- `tasks-checklist`: completed items de-emphasised; labels styled.
- `tasks-progress-bar`: the gauge is segmented, and each group heading carries its own.
- `task-groups`: `parse` retains the `kind` marker it currently drops.
- `view-palette`: four new roles.

## Impact

- `src/tasks.rs` — the `kind` marker; `Group::progress()` reaches a caller at last.
- `src/ui/tasks.rs` — heading grammar, item styling, the segmented gauge.
- `src/ui/palette.rs` — four roles. Colour literals stay in its own tests.
- View tests at 60 and 120 columns; colour asserted against `palette::style(role)`.
- No dependency, manifest, or seam change.

## Open Questions for Review

1. **Does `Evidence` share red with `Role::ListProblem`?** Problem rows render in the same
   detail region, so red would carry two meanings at once. Distinguishable by shape (a full
   `!`-marked row against a short leading token) — but it is a deliberate overlap, not an
   accident. `Confirm` should take a colour still free in this tab; group headings are
   `Heading(2)` = cyan and `Heading(4)` = green does not occur in a `tasks.md`.
2. **Proportional or equal segments?** Proportional is more honest and degrades better at 22
   groups; equal is easier to read as "group 3 of 12".
3. **Is the `kind` badge worth a cell** on a heading row that already carries a name and a
   progress pair?
