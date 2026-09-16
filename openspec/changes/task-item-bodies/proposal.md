## Why

The tracked-tasks tab shows **20% of the file**. `tasks::parse` (`src/tasks.rs:279-291`)
keeps exactly two kinds of line — ATX headings and checkbox bullets — and discards every
other line before the view is ever reached. Measured over this repository's 44 `tasks.md`
files:

| | Lines |
|---|---|
| Kept (headings + checkbox bullets) | 4,791 |
| **Dropped, non-blank** | **18,684** |
| — of which fenced code (159 blocks) | 318 delimiters + their content |
| — of which HTML comments | 530 |

The three sub-rows do not partition the 18,684: 169 of the corpus's `|` table rows sit
*inside* the fenced blocks counted above them, so a table-row row would double-count them.
The two shown are disjoint.

Two distinct losses hide in that number, and the second is the damaging one.

- **Fenced code, tables, and prose between items vanish.** 25 of the 44 files contain a
  code fence that renders as nothing at all. Every other tab renders fences correctly —
  `ui::markdown` handles `Tag::CodeBlock` at `src/ui/markdown.rs:820` — so this is specific
  to the checklist grammar.
- **A task item is truncated at its first physical line, silently.** The parser is
  line-based, so an item's continuation lines are not "prose between items", they are *that
  item's own sentence*. **1,924 of 2,842 items (67.7%)** lose text this way; the longest
  dropped run after a single item is **100** lines
  (`archive/2026-09-09-mouse-input/tasks.md:626`). It renders as a complete-looking row that
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
  a heading or an ordered list. It does **not** eat the marker, and the earlier claim that it
  did was measured false: `lines("# not a heading")` keeps the `#` and wrongly sets
  `heading: Some(1)`, `- ` and `> ` are *replaced* by `• ` and `│ `, and a leading `` ``` ``
  makes the fragment vanish to zero rows. Reinterpretation here means restyled, re-segmented,
  mis-faced, or dropped — not deleted. Measured over the archive, **0 of 2,842** item texts
  would be reinterpreted today — so this is a structural argument, not a live bug, and it is
  stated as such. It earns its place for a second reason: a single-paragraph render
  yields one flat run list, which is what makes the label offsets below tractable at all.
- **Item text becomes faced.** `tasks-tab`'s Decision 9 (item text is deliberately unfaced)
  is **retired**: facing bodies but not the item line renders one sentence two ways, with
  literal backticks on its first row and a styled code span on its second.
- **A label is matched against the leading plain segment**, never against faced text.
  `tasks::label_of` returns byte offsets into plain text, which no longer address the
  rendered row once inline spans fold. An emphasised label (`- [ ] **RED**: …`) degrades to
  unlabelled — a miss, never a wrong colour, the error direction `task-labels` already
  chose. Measured: 2,531 plain labels across the archive, **0** emphasised.
- **The hanging indent falls after the task number, not under it.** A wrapped item's
  continuation rows and its body rows hang at `prefix + task number` rather than at `prefix`,
  so the number column stays clear and a reader scanning for `2.3` reads a column of numbers
  instead of numbers interleaved with wrapped prose. Measured: **2,842 of 2,842** items carry
  a number, of width 4 (2,168), 5 (663), or 6 (11), so the hang is 8 columns for three items
  in four and never more than 10. The number hang is dropped whole before the prefix is, so a
  narrow width never loses the text to it.
- **Bodies are always visible under an open item.** No second fold level, no new keybinding.

Not **BREAKING**: no manifest, config-format, or keybinding change.

## Non-Goals

- **No second fold level.** Folding an item's body is the better end state — this repository's
  items carry 4.72 body lines each, so a 12-item group grows from ~12 rows to ~68 — but it needs the
  detail cursor to address items, which `detail-scroll` does not do today. Deliberately
  deferred so this change stays within its six capabilities.
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

- `task-groups`: **three** requirements. `parse` SHALL retain each item's body and each
  group's inter-item blocks instead of discarding them, replacing the "Prose between items is
  dropped" scenario; and the leading headingless group SHALL now be emitted when it carries
  blocks but no items, without which 736 preamble lines across 27 of the 44 corpus files are
  retained into a group that is then discarded.
- `tasks-checklist`: the grammar gains body rows and block rows; item text is faced; the
  label is matched against the leading plain segment.
- `artifact-folds`: **two** requirements. A tracked-tasks section's body is no longer "its
  items and nothing else" — it is that section's items, their bodies, *and* its blocks, in
  document order; and "`Space` toggles the artifact section the cursor is on or in" carries a
  scenario whose preamble row set is fixed at "the progress-bar row and its blank line, and
  those two only", which the retained preamble block falsifies. The action stays inert for the
  reason it always was — a body row is not a fold target — but the rows it is inert over
  change, so the scenario is amended rather than left to rot.
- `task-labels`: the leading task-number skip that `label_of` already performs is exposed as
  `tasks::task_number_len`, sharing one helper with `label_of` so the rule is not copied.
- `markdown-render`: the module's public surface gains
  `ui::markdown::inline(text, width) -> Vec<Line>`, which renders a fragment as a single
  paragraph — inline faces and wrapping, no block construct recognised.
- `artifact-content`: the content area's own requirement names the tracked-tasks body
  grammar and the set of functions whose faces it admits; both sentences move from
  `ui::tasks::items` and its "items-only grammar" to `group_body` and the group grammar.
  Without this delta the rename leaves a live requirement naming a function that no longer
  exists.

## Impact

- **Code:** `src/tasks.rs` (parse, the emitted blocks-only leading group, and the exposed
  `task_number_len`; `count` untouched),
  `src/ui/markdown.rs` (the new
  `inline` entry point), `src/ui/tasks.rs`, `src/ui/detail.rs` (the tracked-tasks branch of
  the section walk).
- **Sequencing: settled.** This change and `section-body-indent` both carried an
  `artifact-folds` delta on the **same** requirement, "A section header row names the file and
  shows its fold state", and each held the other's untouched original — so whichever archived
  second would silently have reverted the first. `section-body-indent` was the smaller and
  landed first, archived `2026-09-16`, and **this change's `artifact-folds` delta has since
  been re-diffed against the archived requirement**: it is now that requirement's live text
  plus this change's own edits, carrying all 23 of its scenarios plus this change's 2. Four
  places where the two rules meet were resolved in the re-diff rather than left to the merge:
  a tracked-tasks body is obtained at `body_width`, not `width`, so a group's rows are
  indented on the same terms as every other body; the no-exemption paragraph now names a
  group's item bodies and blocks beside its items; "A depth-0 tracked-tasks tab is unmoved at
  every width" no longer claims byte-identity, which this change falsifies by adding rows, and
  claims the horizontal reading it was written for; and "A depth-1 tracked-tasks tab indents
  its items" now names every row of a group's body rather than its item rows alone. What the
  delta no longer carries is the deleted sentence "Body rows SHALL NOT be indented by depth",
  which it held verbatim until the re-diff.
- **Roadmap:** **unplanned**. `openspec/IMPLEMENTATION-ORDER.md` scoped `tasks-tab` as
  "render the task file as a checklist" and never distinguished the counter's model from the
  renderer's, so the roadmap had no row where this could have been anticipated.
- **PRD non-goals:** clear. No OpenSpec file is edited, no change is authored, nothing is
  orchestrated across changes, and no Windows path is added.
- **Gates:** `TASKWIDTHS` requires every `#[test]` in `src/ui/tasks.rs` to name both `58` and
  `78`; the new body and block rows are width-parameterised and inherit it. `COLWIDTH` sweeps
  the file for `.chars()`-based measurement. Coverage floor unchanged at 80%.
