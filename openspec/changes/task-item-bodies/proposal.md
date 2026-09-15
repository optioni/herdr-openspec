## Why

The tracked-tasks tab shows **20% of the file**. `tasks::parse` (`src/tasks.rs:279-291`)
keeps exactly two kinds of line — ATX headings and checkbox bullets — and discards every
other line before the view is ever reached. Measured over this repository's 41 `tasks.md`
files:

| | Lines |
|---|---|
| Kept (headings + checkbox bullets) | 4,674 |
| **Dropped, non-blank** | **18,314** |
| — of which fenced code (156 blocks) | 312 delimiters + their content |
| — of which table rows | 582 |
| — of which HTML comments | 530 |

Two distinct losses hide in that number, and the second is the damaging one.

- **Fenced code, tables, and prose between items vanish.** 23 of the 41 files contain a
  code fence that renders as nothing at all. Every other tab renders fences correctly —
  `ui::markdown` handles `Tag::CodeBlock` at `src/ui/markdown.rs:820` — so this is specific
  to the checklist grammar.
- **A task item is truncated at its first physical line, silently.** The parser is
  line-based, so an item's continuation lines are not "prose between items", they are *that
  item's own sentence*. **1,827 of 2,751 items (66%)** lose text this way; the longest
  dropped run after a single item is 41 lines. It renders as a complete-looking row that
  is a half-sentence — task 1.3 of `mouse-text-selection` ends the pane row on a dangling
  `|` inside an unbalanced backtick.

The counting rule is not the problem and does not move. `tasks::count` copies the OpenSpec
CLI's `countTasksFromContent` byte-for-byte and must stay line-based (`SPEC.md` → Dual-source
model). What happened is that `tasks-tab` reused the *counter's* model as the *renderer's*
model. The archived designs argue the counter at length and nowhere argue the renderer;
`task-groups`' scenario "Prose between items is dropped and does not split a group" is a
statement about grouping, written for the count.

## What Changes

- **`tasks::Item` gains a `body`** — the lines following its bullet up to the next
  checkbox, heading, or dedented block — and **`tasks::Group` gains ordered `blocks`** for
  content sitting between items. `count` and `progress` are computed from checkbox lines
  exactly as today and are asserted byte-identical over the whole archive.
- **Bodies and blocks render through `ui::markdown::lines`**, so fences, tables, and block
  quotes draw with the grammar every other tab already uses. `ui::tasks` gains a call to
  `ui::markdown`; it already names that module's `Line` type, and `TASKSEAM` forbids only
  `ratatui` types and filesystem edges, so no seam moves.
- **`ui::markdown` gains one public function, `inline`**, and item *text* goes through it
  rather than through `lines`. An item's text is a **fragment**, not a document: handing
  `- [ ] # not a heading` or `- [ ] 1. first` to a block parser reinterprets the fragment as
  a heading or an ordered list and eats its marker. Measured over the archive, **0 of 2,751**
  item texts would be reinterpreted today — so this is a structural argument, not a live bug,
  and it is stated as such. It earns its place for a second reason: a single-paragraph render
  yields one flat run list, which is what makes the label offsets below tractable at all.
- **Item text becomes faced.** `tasks-tab`'s Decision 9 (item text is deliberately unfaced)
  is **retired**: facing bodies but not the item line renders one sentence two ways, with
  literal backticks on its first row and a styled code span on its second.
- **A label is matched against the leading plain segment**, never against faced text.
  `tasks::label_of` returns byte offsets into plain text, which no longer address the
  rendered row once inline spans fold. An emphasised label (`- [ ] **RED**: …`) degrades to
  unlabelled — a miss, never a wrong colour, the error direction `task-labels` already
  chose. Measured: 2,369 plain labels across the archive, **0** emphasised.
- **Bodies are always visible under an open item.** No second fold level, no new keybinding.

Not **BREAKING**: no manifest, config-format, or keybinding change.

## Non-Goals

- **No second fold level.** Folding an item's body is the better end state — this repository's
  items average three lines, so a 12-item group grows from ~12 rows to ~40 — but it needs the
  detail cursor to address items, which `detail-scroll` does not do today. Deliberately
  deferred so this change stays within three capabilities.
- **No change to the counting rule.** `tasks::count`, `task-checkboxes`, and the CLI-parity
  guarantee are untouched.
- **No indentation change to section bodies.** `artifact-folds` states that body rows are not
  indented by depth, with a stated rationale. Revisiting it is separate work.
- **No writing.** The tab stays read-only: no key toggles an item, and nothing writes inside
  `openspec/`.
- **Not a markdown-parser change.** The `pulldown-cmark` option set stays exactly
  `ENABLE_TABLES | ENABLE_STRIKETHROUGH | ENABLE_TASKLISTS`, and no construct gains or loses
  a rendering path. `inline` is a second entry point into the parser already vendored, not a
  second parser and not a wider one.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `task-groups`: `parse` SHALL retain each item's body and each group's inter-item blocks
  instead of discarding them. Replaces the "Prose between items is dropped" scenario.
- `tasks-checklist`: the grammar gains body rows and block rows; item text is faced; the
  label is matched against the leading plain segment.
- `artifact-folds`: a tracked-tasks section's body is no longer "its items and nothing
  else" — it is that section's items *and* its blocks, in document order.
- `markdown-render`: the module's public surface gains
  `ui::markdown::inline(text, width) -> Vec<Line>`, which renders a fragment as a single
  paragraph — inline faces and wrapping, no block construct recognised.

## Impact

- **Code:** `src/tasks.rs` (parse only; `count` untouched), `src/ui/markdown.rs` (the new
  `inline` entry point), `src/ui/tasks.rs`, `src/ui/detail.rs` (the tracked-tasks branch of
  the section walk).
- **Roadmap:** **unplanned**. `openspec/IMPLEMENTATION-ORDER.md` scoped `tasks-tab` as
  "render the task file as a checklist" and never distinguished the counter's model from the
  renderer's, so the roadmap had no row where this could have been anticipated.
- **PRD non-goals:** clear. No OpenSpec file is edited, no change is authored, nothing is
  orchestrated across changes, and no Windows path is added.
- **Gates:** `TASKWIDTHS` requires every `#[test]` in `src/ui/tasks.rs` to name both `58` and
  `78`; the new body and block rows are width-parameterised and inherit it. `COLWIDTH` sweeps
  the file for `.chars()`-based measurement. Coverage floor unchanged at 80%.
