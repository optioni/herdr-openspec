## Context

`artifact-folds` derives the detail region's sections from a file's own headings and
normalises their depth against that file's smallest heading level. For a delta spec that is
exactly right. For a `tasks.md` that opens with a document title it is not: the title becomes
a labelled section with a header row, a fold, and a `[-]` progress cell, and every real group
drops to `depth` 1 and gains a two-column body indent.

Measured live, `herdr-openspec ui` in a 72×30 pty against `~/Code/slot-car-racing`'s
`drift-window` change draws `▾ drift-window — tasks` with `[-]`, then **174** rows of the
title's prose (counted by driving `ui::detail::content_lines` over the real file at the pane's
70-column interior), then `▸ 1. Measure the model as it stands` indented two columns.

The shape is common rather than exotic. Surveying both trees for a file whose **first**
heading is the **only** heading at its smallest level and whose own body holds **no**
checkbox — the three clauses of the rule this change adds:

| Tree | `tasks.md` files | would demote |
|---|---|---|
| `herdr-openspec` | 49 | **17** |
| `slot-car-racing` | 9 | 2 |

All 19 are level-1 titles; all 19 pass clause 3 as well as clauses 1 and 2, so no file in
either tree is excluded by the no-items clause alone; and **none** of them has a contribution
count of 1 — the minimum is 7 — so the split decision changes for none of them, under either
reading of the emptiness predicate (see D8).

**Three** of the 19 — `agent-attribution`, `agent-launch`, and `agent-polling` — carry a title
body of exactly `"\n"`. **No** file in either tree has a byte-empty title body, so the shape
D4's worked example uses is real markdown but not a shape the corpus contains; the shape the
corpus actually has is the whitespace-only one, and D8 is the decision that settles it.

The 49 includes this change's own `tasks.md`, committed in the planning commit; at its parent
`8ccd255` the figure was 48, which is what the checks recorded in `tasks.md` were measured
against.

The constraint the fix must respect: `sync_detail` already reads each path exactly once per
key change and calls `split_headings` at most once per path, and `ui::app` is a pure view file
under `NOIO-VIEW`. Nothing here may read a file, spawn a process, or consult a clock.

## Goals / Non-Goals

**Goals:**

- A tracked-tasks file's document title owns no header row, no fold, and no progress cell.
- Its prose still renders, above the first group, as ordinary body rows.
- The real groups return to `depth` 0 and lose the indent that level cost them.
- A spec tab is bit-for-bit what it was.
- No file's *split* decision changes as a side effect, and no heading line is lost by one.

**Non-Goals:**

- Changing what a labelled heading section's `progress` counts (its own body, not its
  subtree). That disagreement with `seed_expanded` is real and deliberately left alone.
- Replacing the collapse-everything gesture the title header incidentally provided.
- Drawing the title's text anywhere. The detail region's own heading row already names the
  change.
- Any change to `tasks::parse`, to checkbox counting, or to the progress bar and its gauge.

## Boundaries

| Module | What changes | Pattern followed |
|---|---|---|
| `src/ui/app.rs` | `Dashboard::sync_detail`'s per-path walk, plus one new private pure helper beside `preamble_len` and `seed_expanded` | the existing private-helper-plus-doc-comment shape those two already have |
| `src/ui/detail.rs` | nothing | `visible_sections`, `bodies_are_indented`, and `content_lines` already treat `label.is_none()` as always-open, un-foldable, and header-less |
| `src/ui/tasks.rs` | nothing | `group_body` already renders a headingless group's blocks, which is what a preamble is |
| `src/ui/view.rs` (tests only) | the render half of the two rendering scenarios | its `WIDTHS` gate requires every `#[test]` there to name `60` and `120`, which is what the scenarios render at; `WIDTHS_MIN` is a floor, so added tests are fine |
| `tests/title_corpus.rs` (new) | the corpus guard | a `tests/` integration file, **not** a `src/ui/` one: `noio-view.sh`'s `PURE` list covers `src/ui/app.rs` and its grep is **not** `#[cfg(test)]`-stripped, so even a unit test there could not read the corpus |
| `SPEC.md` (two sites), `AGENTS.md`, and a doc comment in `src/ui/detail.rs` | the prose describing the derivation | named site by site in the documentation group, because **no** test binds this prose — `tests/doc_contract.rs` reads none of it |

No new module, no new type, no new field on `ArtifactSection`, no new dependency. The change
is confined to which sections the existing walk pushes and at which depth.

The new helper is `fn title_heading(headings: &[HeadingSection], tracks_tasks: bool) ->
Option<usize>`: pure, total, returning `Some(0)` when the three clauses hold and `None`
otherwise. It is sited in `src/ui/app.rs` beside `preamble_len` for the same reason that
function is there — it measures no display width, so `COLWIDTH` and the `*WIDTHS` gates are
unaffected, and `src/ui/app.rs` is not swept for a `58`/`78` pair.

**The helper stays private, and the corpus guard does not call it.** An integration test links
only against the public API, so a guard that re-derived the three clauses and the contribution
arithmetic would be a second implementation of the rule under guard — green even if
`title_heading` were never written. The guard instead drives the real derivation end to end:
it reads each `tasks.md` itself, builds a one-artifact `Dashboard` with `tracks_tasks: true`,
calls `Dashboard::sync_detail` with a closure returning those bytes, and asserts
`detail.sections.len() > 1`. Every field it needs is `pub`, and `NOLIT-CHANGE` scans `src/`
only, so a `Change { … }` literal in `tests/` is allowed. It finds the tree through
`env!("CARGO_MANIFEST_DIR")`.

## Contracts

`ArtifactSection` is unchanged in shape: five fields, no `Default`, `NODEFAULT-UI`'s type list
untouched. What changes is which values the derivation produces, and both consumers —
`ui::detail::content_lines` and `ui::app::seed_expanded` — already handle a `None` label and a
`None` progress, so the change is **additive** at the type level and behavioural only.

No interface a separate process depends on moves: no manifest key, no config key, no CLI
argument, no keybinding. `Space` on a demoted title's row becomes inert; that is the existing
preamble behaviour reached by a new input, not a new behaviour.

## Persistence and Rollout

- **migration** — none. The section list is derived per key change and never stored.
- **backfill** — none.
- **seeding** — `seed_expanded` is unaffected in rule but improves in effect: a demoted title
  is no longer a seeded parent, and each group seeds itself over its own subtree.
- **cache invalidation** — none. The `(change directory, tab)` cache key does not change.
- **index rebuild** — none.
- **authorization** — none; the pane is read-only and this touches no write path.
- **observability** — none; nothing here produces a problem row.
- **deployment** — `make build` and the pane picks it up on next open. No manifest change, so
  no `herdr plugin link` step.

## Test Boundaries

| Dependency | In the corpus guard | In unit and view tests |
|---|---|---|
| Filesystem (artifact read) | **real, read-only** — `tests/title_corpus.rs` reads each committed `tasks.md` and hands its bytes to the reader closure | replaced — the injected `ArtifactReader` closure returning canned text, per `artifact-content`'s one-binding rule |
| `openspec` binary | not reached — `sync_detail` never touches the CLI seam | not reached |
| Herdr socket / `herdr` binary | not reached | not reached |
| Terminal | replaced — `ratatui::backend::TestBackend` at 120×40 and 60×40 | replaced, identically |
| Clock, watcher, worker threads | not reached — no `Instant`, no channel, no thread | not reached |
| `tasks::parse` / `tasks::count` | real — pure functions over `&str` | real |
| `ui::app::split_headings` | real — pure function over `&str` | real |

Every collaborator this change touches is above; there is no dependency left unstated. No
**unit or view** test may introduce a real filesystem, a real binary, or a real terminal; the
corpus guard's read-only walk of the committed tree is the one real-filesystem boundary this
change adds, and it is named here rather than invented by a task. Nothing in this change ever
reaches a real binary or a real terminal.

## Test Strategy

Tiers, per project context: **unit** (`cargo test --lib`, pure module tests over
`sync_detail`'s derived state), **view** (the same command; render into a `TestBackend` at 60
and 120 columns), **corpus** (`cargo test --test title_corpus`, a read-only survey over the
committed `openspec/changes` tree — no process, no network, and sited in `tests/` because
`NOIO-VIEW` forbids a pure view file from naming a filesystem API at all). All three are
reached by `make test` and therefore by `make check`.

This change takes **no outer-loop acceptance test** of its own, and the reason is not that the
outer loop is undrivable — `ui::driver::run_loop` is driven 68 times in `src/ui/driver.rs`'s
own tests against a `TestBackend` and the `none()` seams. The reason is that `run_loop` adds
only event plumbing this change does not touch: the behaviour is fully determined by
`sync_detail`'s derived `detail.sections` and by the rows `content_lines` returns for it, both
observable one layer in. An outer-loop test here would exercise the loop, not the rule.
`artifact-folds`' existing scenarios sit at those same two tiers for the same reason.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| A document title heading is demoted to an unlabelled section | new test asserting `(label, depth)` through `shape_of` and each section's `progress` read directly off `detail.sections`, then a render at 120×40 and 60×40 counting header rows and resolving `section_at` | unit + view | reader replaced, terminal replaced | `cargo test --lib ui::app` and `… ui::view` |
| A title heading with no prose under it does not split the file | new test asserting one section, its `Some("tasks.md")` label, its verbatim `text`, `foldable()` false, and both heading rows present in the drawn buffer at 120×20 and 60×20 | unit + view | reader replaced, terminal replaced | `cargo test --lib ui::app` and `… ui::view` |
| A whitespace-only title body contributes no section | new test asserting two sections, no unlabelled one, and that the file still splits | unit | reader replaced | `cargo test --lib ui::app` |
| A leading heading holding its own items is a group, not a title | new test asserting the pre-change triples are reproduced exactly | unit | reader replaced | `cargo test --lib ui::app` |
| Two headings at the file's shallowest level are both groups | new test asserting two labelled sections and no unlabelled one | unit | reader replaced | `cargo test --lib ui::app` |
| A preamble and a demoted title are two unlabelled sections | new test asserting three sections and, at `Route::Detail` with `drawn_width: Some(78)` so the check is not vacuous, that ten `ToggleSection` actions on either unlabelled row leave `expanded` unchanged | unit | reader replaced | `cargo test --lib ui::app` |
| A spec tab's lone operation heading keeps its header row | new test asserting three labelled sections and `operation: Some(Added)` on the requirement | unit | reader replaced | `cargo test --lib ui::app` |
| **Corpus guard** (no spec scenario; design's own) | `tests/title_corpus.rs`: drives the real `Dashboard::sync_detail` over every committed `openspec/changes/**/tasks.md` and asserts each demoted file still yields more than one section, so no file's split decision changes; prints the scan count and fails at zero | corpus | real committed tree read-only, reader closure over its bytes | `cargo test --test title_corpus` |
| The three spec files of a change become three labelled sections | carried unchanged — existing `ui::app` test re-run | unit | reader replaced | `make test` |
| A spec glob nests requirements under their capability | carried unchanged — existing `ui::app` test re-run | unit | reader replaced | `make test` |
| A preamble becomes an unlabelled section | carried unchanged — existing test re-run; its fixture opens at `## 1. Setup` with prose above, so no title is recognised | unit + view | reader replaced, terminal replaced | `make test` |
| A single-file artifact is one section and is not foldable | carried unchanged — existing test re-run | unit + view | reader replaced, terminal replaced | `make test` |
| An artifact with no resolved paths has no sections | carried unchanged — existing test re-run | unit | reader replaced | `make test` |
| An unreadable file drops its section and keeps its siblings | carried unchanged — existing test re-run | unit | reader replaced | `make test` |
| The label derivation is total over adversarial paths | carried unchanged — existing test re-run | unit | none | `make test` |
| A delta spec's requirement sections carry their operation and nothing else does | carried unchanged — existing test re-run, and the operation walk is deliberately untouched | unit | reader replaced | `make test` |
| A requirement above every operation heading carries none | carried unchanged — existing test re-run | unit | reader replaced | `make test` |
| A main spec's requirements are entirely unbadged | carried unchanged — existing test re-run | unit | reader replaced | `make test` |
| Only a level-3 `Requirement:` heading is attributed | carried unchanged — existing test re-run | unit | reader replaced | `make test` |
| A non-spec artifact is attributed nothing | carried unchanged — existing test re-run | unit | reader replaced | `make test` |
| A tracked-tasks tab's sections carry progress and no operation | carried unchanged — existing test re-run | unit | reader replaced | `make test` |
| The selected tab's file is read once and reused | carried unchanged — existing `ui::app` test re-run | unit | recording reader replaced | `make test` |
| A forced reload re-reads the same key and keeps the scroll | carried unchanged — existing test re-run | unit | recording reader replaced | `make test` |
| A tab move under a forced reload still resets the scroll | carried unchanged — existing test re-run | unit | recording reader replaced | `make test` |
| Switching the tab re-reads, and so does switching the change | carried unchanged — existing test re-run | unit | recording reader replaced | `make test` |
| Two changes with the same name are distinguished by directory | carried unchanged — existing test re-run | unit | recording reader replaced | `make test` |
| A multi-file artifact is concatenated in path order with a separating newline | carried unchanged — existing test re-run | unit | reader replaced | `make test` |
| A split file is partitioned rather than copied | carried unchanged — existing test re-run; its fixture is a delta spec, which the title rule never reaches | unit | reader replaced | `make test` |
| The tasks tab seeds its folds once, on the key change | carried unchanged — existing test re-run | unit | reader replaced | `make test` |
| An unreadable file names its reason and does not lose its siblings | carried unchanged — existing test re-run | unit | reader replaced | `make test` |
| An artifact with no resolved paths reads nothing at all | carried unchanged — existing test re-run | unit | recording reader replaced | `make test` |
| A tab out of range for the newly selected change is clamped before the read | carried unchanged — existing test re-run | unit | reader replaced | `make test` |
| An empty visible list clears the detail | carried unchanged — existing test re-run | unit | reader replaced | `make test` |
| The loop syncs before it draws | carried unchanged — existing test re-run | unit | reader replaced | `make test` |

The eleven `artifact-content` scenarios and thirteen of the `artifact-folds` ones are carried
byte-identical by their MODIFIED blocks; each is listed because the matrix must account for
every scenario in the change, and each row's verification is "the existing test still passes",
which is what proves the carried text was not quietly reverted.

## Decisions

**D1 — Demote the title to an unlabelled section rather than fix the cell.**
Alternatives: make a heading section's `progress` count its **subtree** (so the title reads
`[54/85]`), or draw no cell where a section's own body holds no items. Both leave every group
one level down and 140 rows of prose inside a fold, which is the larger half of what the
reader complained about. The subtree variant is also a change to a rule three other places
read, and it is left as a separate argument; see Non-Goals.

**D2 — Gate the rule on `tracks_tasks`, not on the heading's level or its text.**
A delta spec whose only level-2 heading is `## ADDED Requirements` satisfies all three clauses
and must keep its header row — it is the operation the badge walk attributes from. "Looks like
a title" (matching the change name, or ending `— tasks`) was rejected outright: it is the
weak-evidence guessing `SPEC.md` forbids for agent attribution, and the same argument applies.
On a tracked-tasks tab a heading section **is** a task group, which is the whole justification
for treating a group-less leading heading as something else.

**D3 — Clause 3 asks `tasks::count(body).total == 0`, not "the body is prose".**
A leading heading that carries items of its own is a real group — the schema's `tasks.md` may
legitimately open with unnumbered setup work — and demoting it would hide a header row that
owns a progress cell. Measured: 19 of 57 task files across the two trees pass clauses 1 and 2,
and all 19 also pass clause 3, so the clause costs nothing on the real corpus and is there for
the shape it excludes.

**D4 — The split decision counts the sections the derivation yields, not the headings.**
This is the subtle half. `# drift — tasks\n## 1. Setup\n\n- [x] a\n` has a byte-empty title body — empty under either
reading, see D8 —
without this rule the file would still split, yield exactly one labelled section, be
**non**-foldable by `sections.len() > 1`, and fall to `ui::tasks::lines` over a text that no
longer carries its `## 1. Setup` line — a heading lost off the screen. Counting contributions
after demotion keeps it unsplit, and `ui::tasks::lines` then draws both heading lines. The
alternative — refusing to demote a title whose body is empty — was rejected because it
reintroduces the `[-]` header for exactly the three files that have the least to say under it.
Measured: no file in either tree has a post-demotion contribution count of 1, so this rule
fires on none of them today and exists to keep a future one correct.

**D5 — `min_level` excludes the demoted title.**
Without this the groups stay at `depth` 1 under a header row that no longer exists, and the
body indent stays with them. It is the half that un-indents.

**D6 — One walk, one branch; the operation walk is untouched.**
The title is handled inside the existing `for heading in headings` loop, at index 0, by
pushing an unlabelled section instead of a labelled one. `current_operation` therefore still
advances over every heading in document order, so `spec-emphasis`' derivation cannot be
perturbed even by a tracked-tasks file that happens to carry an operation heading. The
alternative — skipping the title before the loop — would have required arguing that case away
in prose instead of by construction.

**D7 — Two unlabelled sections rather than one merged preamble.**
A file with prose above its title would otherwise need a `text` value synthesised from two
non-contiguous byte ranges, which falsifies `artifact-folds`' own "a split file's sections
partition those same bytes" sentence. Two sections cost one reworded sentence and keep the
partition property exactly true.

**D8 — a title body is measured after trimming whitespace, and the preamble's is not.**
The corpus's real shape is `# T\n\n## 1. G`, whose title body is exactly `"\n"`. Byte
emptiness — the predicate the adjacent preamble rule uses, `!preamble.is_empty()` — would make
that a section carrying one newline, which `content_lines` draws as **zero** rows: a section in
`detail.sections` that no row can address, on three of this repository's seventeen demoted
files. Trimming removes it. The two readings were measured to be equivalent everywhere else —
across 58 task files in two trees, **zero** change their split decision between them, the
minimum post-demotion contribution count being 7 — so the choice is between an unreachable
section and none. The two predicates are deliberately **not** unified: changing the preamble's
would alter behaviour this change has no business touching, so the difference is stated in the
spec rather than left for an implementer to "tidy up".

## Risks / Trade-offs

- **The collapse-everything gesture disappears** for the 19 files that gain a title → named as
  a Non-Goal rather than discovered; the groups still fold individually, and the seed now opens
  exactly the unfinished ones instead of one parent that swallowed them.
- **Three files lose a header row and gain nothing**, their title bodies being empty → they
  lose a row that said `[-]` about nothing; that is the intended outcome, not a regression.
- **A carried MODIFIED block silently reverts landed work** → the two blocks were extracted
  from `openspec/specs/` at this HEAD by script, and each was diffed against its live source
  with every difference accounted for; the diff is reproduced in the change's own commit.
- **A file's split decision changes as a side effect** → D4 plus the corpus guard in the
  matrix, which asserts over this repository's own committed tree that every demoted file
  still yields more than one section.
- **Prose in `SPEC.md`/`AGENTS.md` drifts from the new rule** → **not** machine-bound:
  `tests/doc_contract.rs` binds the module map's *names* column, the tested-modules list, the
  MSRV, the gate programs, the key and mouse tables, the seam names, OSC 52 and the
  drag-to-select note — **nothing** that reads either document's section-derivation prose. So
  the mitigation is the documentation group naming every stale site explicitly (there are two
  in `SPEC.md`, not one) and a reviewer checking them, not a test that would pass either way.
- **Four further live requirements restate the derivation and are falsified by it** → all four
  are carried into the deltas and repaired: `artifact-folds`' split gate and its header-row
  requirement, and `artifact-content`'s content-area requirement. Each was extracted by script
  and diffed against HEAD, every difference accounted for.

## Migration Plan

None needed. The section list is derived from the file on every key change, nothing is
persisted, and the plugin's own state directory is untouched. Rollback is `git revert`; the
pane picks up the previous binary on the next `make build`.

## Open Questions

None. The one design question the change opened — whether to fix the cell, the nesting, or
both — was put to the user and answered: demote the title, leave the cell rule alone.
