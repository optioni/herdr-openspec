## Why

The list view's `[4/9]` and the detail view's tasks tab both come from a change's
`tasks.md`, and today nothing reads it: `changes-from-files` cannot report progress
and the tasks tab has nothing to render. The counts also have to be the *same*
numbers `openspec list --json` reports, because the dual-source model layers CLI
results over file results on every refresh — a file count that disagrees with the
CLI count makes the pane flicker between two numbers for the same change.

This is the Phase 2 `task-parsing` row of `openspec/IMPLEMENTATION-ORDER.md`:
"`tasks` — parse a markdown task file into groups under their headings, items with
checked state, and completion counts."

## What Changes

- A new `tasks` module, pure and dependency-free, with three entry points:
  - `count(text) -> Progress` — the flat checkbox count, deliberately identical to
    the OpenSpec CLI's own `countTasksFromContent`.
  - `parse(text) -> Tasks` — the same lines, arranged into groups under the ATX
    headings above them, each item carrying its checked state, text, and indent.
  - `read(path) -> Tasks` — the one filesystem edge: an absent file is zero tasks
    (never a problem), any other read failure is zero tasks plus a named problem.
- The counting rule is adopted verbatim from the CLI rather than invented: a `-` or
  `*` bullet at any indent carrying a one-character `[ ]` / `[x]` / `[X]` box, with
  **no code-fence and no HTML-comment exemption**. The CLI counts a checkbox inside
  a fence on purpose; matching it is what keeps the two sources in agreement.
- Grouping is layered on top of that scan and is proven never to change the count:
  `parse(text).progress() == count(text)` for every input, pinned by a test.
- `Progress` gains `+` and `+=` so `changes-from-files` can sum a multi-file tasks
  artifact (`generates: tasks/**/*.md`) the way the CLI does.
- No new dependency. Not `pulldown-cmark`, not `regex` — see design.md → Decisions.
  `Cargo.toml` is untouched, so `plugin-build`'s argued-dependency-set requirement
  still holds unchanged and needs no delta.
- `SPEC.md` → Data layer gains a short paragraph naming the counting rule and its
  deliberate agreement with `openspec list --json`, which the spec does not state
  today and which every later change depends on.

Nothing here is **BREAKING**: the plugin manifest, the `config.toml` format, and the
keybindings are all untouched, and no consumer of the crate exists yet.

## Non-Goals

- **No rendering.** Checkbox glyphs, the progress bar, and the tasks tab belong to
  Phase 4's `tasks-tab`; this change adds no `ui` code and no view test.
- **No writing, and no checkbox toggling.** Tasks are read-only by design — an agent
  may be editing `tasks.md` in another pane. This holds the PRD non-goal "no editing
  of OpenSpec files".
- **No file selection.** Which path holds a change's tasks artifact, and how a
  `<id>/` directory artifact expands to several files, is `changes-from-files`' job.
  `read` takes a path it is given.
- **No subprocess and no CLI.** `openspec list --json` is `changes-from-cli`'s, in
  Phase 3. This change spawns nothing and needs no injected hook to defer a spawn.
- **No `Change` type.** It arrives with `changes-from-files`; this change only has to
  produce a shape that both of its producers can agree on.
- **No CommonMark conformance.** This is a checkbox and heading scanner, not a
  markdown parser. `markdown-viewer` still owns `pulldown-cmark` for the other tabs.
- No orchestration across changes, no change authoring, no Windows support.

## Capabilities

### New Capabilities

- `task-checkboxes`: which lines of a markdown file are tasks, whether each is
  checked, how they are counted, and why that rule is the OpenSpec CLI's rather than
  this crate's own.
- `task-groups`: arranging those lines into the document model the detail view
  renders — groups under their headings, items in document order, per-group counts —
  and the filesystem edge that reads a task file without writing anything.

### Modified Capabilities

None. No existing requirement changes: `plugin-build`'s dependency set is unchanged
because no dependency is added, and `schema-artifacts` already identifies *which*
artifact holds the tasks without saying anything about its contents.

## Impact

- **Code:** new `src/tasks.rs`; one `pub mod tasks;` line in `src/lib.rs`. No other
  module changes. `Cargo.toml`, `Makefile`, `herdr-plugin.toml`, and
  `.github/workflows/ci.yml` are untouched.
- **Tests:** unit tests inside `src/tasks.rs` (the established idiom), plus a
  real-world corpus fixture under `tests/fixtures/tasks/` pulled in with
  `include_str!` so no test resolves a path in the live repository.
- **Docs:** `SPEC.md` → Data layer (the counting rule), → Degraded states (one row for a
  tasks file that exists and cannot be read), → Unit-tested modules (the `tasks::parse`
  entry, rewritten to name all three entry points); `openspec/IMPLEMENTATION-ORDER.md` →
  the Phase 2 `task-parsing` row (its Spec-refs cell cites only the detail view, which is
  where the result is rendered rather than where its contract lives); `AGENTS.md` →
  Current repo state (one sentence, replacing the "`task-parsing` is next" clause rather
  than appending beside it).
- **Downstream:** `changes-from-files` consumes `read` and `Progress`;
  `changes-from-cli` must produce the same `Progress` from `completedTasks` /
  `totalTasks`; `tasks-tab` consumes `Tasks`, `Group`, `Item`, and `Heading`;
  `live-refresh` re-invokes `read`; `degraded-states` surfaces `Tasks::problems`. No
  sibling repository, deployment manifest, or external service is affected.
