## Reviewed Artifacts

- `openspec/changes/tasks-tab/proposal.md`
- `openspec/changes/tasks-tab/design.md`
- `openspec/changes/tasks-tab/tasks.md`
- `openspec/changes/tasks-tab/specs/tasks-checklist/spec.md` (new capability)
- `openspec/changes/tasks-tab/specs/tasks-progress-bar/spec.md` (new capability)
- `openspec/changes/tasks-tab/specs/change-model/spec.md` (delta)
- `openspec/changes/tasks-tab/specs/change-artifacts/spec.md` (delta)
- `openspec/changes/tasks-tab/specs/cli-changes/spec.md` (delta)
- `openspec/changes/tasks-tab/specs/change-merge/spec.md` (delta)
- `openspec/changes/tasks-tab/specs/artifact-content/spec.md` (delta)
- `openspec/changes/tasks-tab/specs/detail-scroll/spec.md` (delta)
- `openspec/changes/tasks-tab/specs/dashboard-loop/spec.md` (delta)

Nine spec files, 60 scenarios, 14 requirements' worth of delta.

**How the finding pass was run.** Four subagents, none of which wrote the plan, each given
the change directory and one slice of the review list, each reporting findings and editing
nothing. Rounds one to three ran in parallel over the first draft; round four verified the
repairs on the revised draft and looked for anything the repair pass broke.

| Round | Slice | Findings |
|---|---|---|
| 1 | Command-level checks: extract every block, run it, plant a violation, verify every measured floor | 1 CRITICAL, 2 WARNING, 4 SUGGESTION |
| 2 | Coherence: capability coverage, scenario↔matrix↔task closure, delta fidelity, PRD non-goals, task-group discipline, grammar arithmetic, buffer coordinates | 2 CRITICAL, 2 WARNING, 3 SUGGESTION |
| 3 | Codebase truth: every factual claim about the existing tree, and every `SPEC.md`/`AGENTS.md` statement this change makes false | 2 CRITICAL (same two), 7 WARNING, 4 SUGGESTION |
| 4 | Verification of the repairs on the revised draft | 1 WARNING, 4 SUGGESTION |

Every reviewer wrote its findings incrementally to a scratchpad file as it worked, on the
standing instruction added after two earlier rounds died on `529 Overloaded` and lost their
reports. Nothing was lost this time either.

## Reviewed Against

- This repository HEAD: `85a5631dfcfb0c02c4091ccabb3232c194ab61f2`
- Sibling repository HEAD: `Not applicable` — this crate has no sibling; `graft.lock` pins
  vendored files from `optioni/openspec-schemas`, and this change touches none of them.
- Working tree: clean apart from this change's own `openspec/changes/tasks-tab/` directory.
  Reviewers 1 and 4 planted violations under `src/` to test gates and restored each one;
  `git status --porcelain` was confirmed to show only `?? openspec/changes/tasks-tab/` after
  every plant.
- Verified against the running tree, not from memory: 625 lib tests (641 across all targets),
  **97.81% line coverage over 14,178 lines** (the `cargo llvm-cov` TOTAL row leads with
  regions at 97.39% and functions at 96.62% — the line column is the gated one), `make check`
  green, `cargo fmt` and `cargo clippy` clean.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | `tasks.md` | `TASKSEAM`'s parse leg (`grep -qE 'tasks::parse'`) was unpassable at task 4.4: group 4 creates `src/ui/tasks.rs` holding only `progress_bar`, which legitimately calls no parser — the parse arrives with `lines` in group 5. Reproduced by writing a group-4-shaped file and running the extracted block: `TASKSEAM FAIL: … does not call tasks::parse`. The cheapest escape from that false red is to name `tasks::parse` in a comment, which rubber-stamps the leg for the rest of the change | Leg made conditional on `grep -qE '^pub fn lines'`, so it arms itself exactly when group 5 lands, and its two planted-violation controls moved from task 0.4/4.4 to task 5.5 | `tasks.md` → `TASKSEAM` block, tasks 0.4, 4.4, 5.5 |
| CRITICAL | `tasks.md` | The same leg was satisfiable by a **comment**. Verified: a file whose doc comment reads "Built on crate::tasks::parse" while `lines()` hand-rolls `for l in source.lines()` printed `TASKSEAM OK` | Leg now matches a **call shape** (`tasks::parse\s*\(`) over a comment-stripped copy. The other two legs stay comment-inclusive on purpose — a comment naming a `ratatui` type or a read *is* the thing they forbid; a comment naming the parse is not the parse. Re-verified: shape (b) now FAILs, shape (c) passes | `tasks.md` → `TASKSEAM` block |
| CRITICAL | `specs/tasks-checklist/spec.md` | `No tasks yet` was triggered by "`tasks::parse` returns no **groups**", and its own scenario used the source `# Plan\n\nNothing checkable here.\n`. But `task-groups` requires `parse` to emit a group for every heading it recognises, and `src/tasks.rs:213-217` does exactly that — so that source returns **one** group and zero items, and the state could never fire for the documents it exists for. Three places asserted the impossible; `tasks-progress-bar`'s buffer scenario repeated it | Trigger restated over **items** (`Tasks::progress().total == 0`), with a paragraph reconciling it against "A heading with no items still renders its heading" and a decided consequence: a document with zero items renders no heading line at all. Both scenarios now say so explicitly | `specs/tasks-checklist/spec.md` → "A source holding no task lines renders `No tasks yet`"; `specs/tasks-progress-bar/spec.md`; `design.md` → Decisions 10 |
| CRITICAL | `proposal.md` | `dashboard-loop` was a modified capability the proposal did not list. Its landed requirement enumerates **seven** pure view files and its scenario asserts "there is no match in any of the seven"; this change makes it eight. Precedent is explicit — both `markdown-viewer` and `detail-view` grew that set and both shipped a `dashboard-loop` delta. The same requirement also pins `NOCLI-SHELL`'s ten-file floor, which moves to eleven | Added as a modified capability with a full MODIFIED delta covering all three moving parts and all four scenarios | `proposal.md` → Modified Capabilities; new `specs/dashboard-loop/spec.md`; `design.md` → verification matrix (four rows) |
| WARNING | `tasks.md` | `NOIO-VIEW`'s pattern could not see `crate::tasks::read` — the one filesystem call a checklist renderer would plausibly reach for. Planted in `src/ui/detail.rs`'s production code, it was invisible to `NOIO-VIEW`, `READSEAM`, and `READONLY-UI` alike, and group 6 edits exactly that function | `tasks::read` added to `IO_RE`. Verified green on the seven pre-existing pure files (no hits on `main`) and verified to catch the plant at `src/ui/detail.rs:174` | `tasks.md` → `NOIO-VIEW` block; `specs/dashboard-loop/spec.md`; `specs/artifact-content/spec.md` |
| WARNING | `proposal.md`, `design.md`, `tasks.md` | "35 `ArtifactRef` literal sites" was a grep count, not a construction count. `grep -c 'ArtifactRef {'` gives 36, of which the struct definition, two doc comments, and a `-> ArtifactRef {` signature are not constructions. The real number is **32**. Separately, "9 `content_lines` call sites" counted the definition; the real number is **8** | All three artifacts corrected to 32 and 8, each with the four false-positive shapes named so the number is not re-derived from the grep, and with the explicit note that the gate is the compiler (`E0063`), not the grep | `proposal.md` → Impact; `design.md` → Contracts and Risks; `tasks.md` → tasks 0.1, 1.2, 1.4 |
| WARNING | `proposal.md` | `change-model` was a modified capability the proposal did not list: its landed requirement enumerates `ArtifactRef` as "carrying the schema artifact's `id` and the concrete `paths`" | Added as a modified capability with a full MODIFIED delta, including why `tracks_tasks` is not an exception to that requirement's own "SHALL NOT carry" list | `proposal.md`; new `specs/change-model/spec.md`; `design.md` → matrix (two rows) |
| WARNING | `design.md` | The Test Boundaries table said the filesystem is "replaced … no view test opens a directory" in unit tests, while seven of its own matrix rows declared `Unit | filesystem real (ScratchDir)` — `changes::from_files` has a real filesystem edge and faking it would test a fake | Row split into two: replaced for `ui::` unit tests, real for `changes::` producer tests, each with its reason | `design.md` → Test Boundaries |
| WARNING | `specs/tasks-checklist/spec.md` | Two arithmetic errors: "two checked and three unchecked items" asserted `Progress { completed: 2, total: 3 }`; and the missing-tasks-artifact scenario inherited a `{0,0}` dashboard while claiming "the change's `progress` is non-zero" — the one scenario `design.md` relies on to show the legitimate header/content divergence | Corrected to `2 / 5`, and the second scenario given its own `Progress { completed: 4, total: 9 }` dashboard and an explicit statement of the divergence | `specs/tasks-checklist/spec.md` |
| WARNING | `specs/dashboard-loop/spec.md` | The first repair pass **rewrote** the `Dashboard has no Default` scenario instead of copying it, silently dropping four landed assertions: the compile-time companion, the planted-violation control, the multi-line-rustfmt form, and the missing-file guard. A MODIFIED requirement replaces on archive, so those would have been deleted from the live spec while their tests stayed | All four restored from the live text, with the one clause that *is* genuinely stale — `markdown-viewer`'s "the compile-time companion is what covers it", which `detail-view` retired — corrected in place rather than carried forward | `specs/dashboard-loop/spec.md` |
| WARNING | `design.md` | Eight verification-matrix commands named view-test functions that do not exist (`ui::view::tests::missing_artifact_no_content_yet` and seven more). Run as written they match nothing and `cargo test` exits 0 — the plan's own named risk, in the plan | All eight replaced with the real function names read off `src/ui/view.rs` | `design.md` → verification matrix |
| WARNING | `design.md` | The 78-column mock showed a 68-character gauge with **32** `█` and 36 `░`; the spec requires `68 × 4 / 9 = 30` filled | Mock corrected to 30 `█` and 38 `░` | `design.md` → Visual Design |
| WARNING | `design.md` | Two `SPEC.md` corrections were missed. `SPEC.md`'s degraded-states row "Markdown source holds a construct the parser does not model (… **a task-list item**) → Renders as its literal source text" is unqualified by tab and is now false on the tracked-tasks tab. And `AGENTS.md`'s detail-region width rule — "Every test in `ui::markdown` and `ui::detail` asserts both" — needs `ui::tasks`, a third width-parameterised pure module | Correction list grown from four to six; the `AGENTS.md` documentation task grown from one bullet to three | `design.md` → `SPEC.md` corrections; `tasks.md` → tasks 12.2, 12.3, 12.5 |
| SUGGESTION | `tasks.md` | `SCAN_MIN=90` against a measured 97 left seven of slack, and task 7.4 is explicitly a span-**reducing** refactor | Lowered to 80 | `tasks.md` → task 0.1 and every `NODEFAULT-UI` invocation |
| SUGGESTION | `tasks.md` | `BASE` was an exported shell variable. It fails closed in a fresh shell, but the natural recovery — re-exporting from the current `HEAD` — silently defeats `OPENSPEC-UNTOUCHED`, since every commit the change makes would then be inside the baseline | Task 0.1 now recommends the literal `BASE=85a5631` at each invocation and says why | `tasks.md` → tasks 0.1, 13.6 |
| SUGGESTION | `tasks.md` | Groups 2, 6, 8, and 9 had no REFACTOR task and no sentence saying none was needed, which the schema's behavior lifecycle requires | All four given an explicit REFACTOR task; group 2's says why none is possible while its test is red and points at task 10.2 | `tasks.md` → tasks 2.4, 6.4, 8.4, 9.4 |
| SUGGESTION | `tasks.md` | Task 13.1 said "the eight `testcount` floors", which matched nothing countable; task 11.1 handed the reviewer "the seven spec files", now nine; the preamble called the current `NOIO-VIEW` "six-file", now seven | Corrected to nine invocations across seven distinct filters, nine spec files, and seven-file | `tasks.md` → preamble, tasks 11.1, 13.1 |
| SUGGESTION | `specs/tasks-checklist/spec.md` | `Item.indent` is `task-parsing`'s count of whitespace **characters**, so a tab-indented item renders with one space — surprising if undocumented | Stated as a decision in the grammar requirement, with the reason not to re-derive a column width here | `specs/tasks-checklist/spec.md` |
| SUGGESTION | `design.md` | Two `change-model` matrix rows were labelled "Existing test, extended" while their commands name tests group 3 creates | Relabelled, and the "six of the sixty are carried by unchanged verification" sentence corrected from eight to six | `design.md` → verification matrix |
| SUGGESTION | `tasks.md` | `AGENTS.md`'s landed-changes enumeration ends at `detail-view`, and the documentation task said only "Current repo state gains one sentence" | Task 12.5(c) now names both the enumeration and the running description | `tasks.md` → task 12.5 |

### `SPEC.md` corrections this change makes

Six, listed in `design.md` → "`SPEC.md` corrections this change makes" with their reasons.
The before/after of the two that replace existing text:

**1. Degraded states, the "Schema loads with no tasks artifact" row.**

> *Before:* "Every other tab renders; the tasks tab is absent, which is `tasks-tab`'s
> rendering decision. The task **count** is not deferred: it falls back to counting
> `<change dir>/tasks.md` directly …"

> *After:* "No artifact is marked, so every tab renders as markdown and none renders the
> checklist; no tab is added, removed, or hidden. The task **count** is not deferred: it
> falls back to counting `<change dir>/tasks.md` directly …"

Wrong twice over: there is no tasks tab to be absent (the bar is built from the schema's
declared artifacts and no entry is marked), and `detail-view` plus
`openspec/IMPLEMENTATION-ORDER.md` both state that `tasks-tab` adds no tab-bar code and
removes no tab. The row's second sentence is true and carries forward verbatim.

**2. Degraded states, the "construct the parser does not model" row.**

> *Before:* "Markdown source holds a construct the parser does not model (a table, a
> footnote, strikethrough, a task-list item) | Renders as its literal source text …"

> *After:* the same, scoped to every tab **other** than the tracked-tasks one — the scoping
> the twin sentence in Detail view already carries ("Every other tab is rendered by
> `markdown-viewer`'s markdown viewer"), which is why that sentence survives and this row
> does not.

The other four are additive or in-place rewrites: two new degraded-states rows (a tasks file
that yields no items; a marked tab resolving to no file while the header shows a non-zero
pair), `ui::tasks` added to Detail view, the Module map, Unit-tested modules, and View tests,
and the tasks-tab paragraph rewritten in place rather than appended to. Every one is carried
as a numbered task in `tasks.md` group 12, so none is a note that only lives here.

Running totals of `SPEC.md` corrections by change: `tui-shell` 9, `list-view` 17,
`markdown-viewer` 12, `detail-view` 10, `tasks-tab` 6.

## No Remaining Implementation-Blocking Gaps

None remain. Both CRITICALs from the first round and both from the second are repaired and
the repairs were re-verified by a fourth reviewer against the revised draft, by running the
edited check blocks against four hand-written `src/ui/tasks.rs` shapes and against planted
violations. `openspec validate tasks-tab --strict` passes. No decision requires user input:
the four that could have been open — which value the bar renders, whether the bar is a
widget, where the tracked-tasks flag lives, and whether item text is re-parsed as markdown —
are decided in `design.md` → Decisions 1, 2, 3, and 9.

Two claims that were *checked rather than assumed*, because both are the kind of reading that
becomes a standing instruction if it is wrong:

- **`schema::Schema::tasks` is always equal to some entry of `Schema::artifacts`.** Confirmed
  by reading `src/schema.rs`: `tasks` is a clone of an element of the list, and both lookups
  are first-match, so `position(|a| a == tasks)` reproduces the CLI's `find` and can never be
  `None` while `tasks` is `Some`. Had it been otherwise, the whole `tracks_tasks` design
  would have had a hole in it.
- **`Dashboard::normalise_scroll` still borrow-checks** with `self.selected_change()` inside
  the same statement as `&self.detail`: two shared reborrows of `*self` in one expression
  whose result is a `usize`, so no borrow outlives the statement and the following
  `self.detail.scroll = …` is fine under NLL.

## Deferred Non-Blocking Notes

- **`openspec/IMPLEMENTATION-ORDER.md`'s `tasks-tab` row and its "Notes on the ordering"
  paragraph** must be confirmed to still describe what was built, and corrected if not. It
  cannot be done during implementation: `OPENSPEC-UNTOUCHED` forbids writing anywhere under
  `openspec/` outside this change's own directory. Recorded as task 12.6 and discharged by
  `openspec archive`, the way `list-view` (10.4b), `markdown-viewer` (11.8), and `detail-view`
  (13.4) each did.
- **`detail-scroll`'s landed "The stored scroll offset is normalised against the frame just
  drawn" requirement** keeps its current wording. It names `ui::detail::content_lines` without
  arguments, so nothing in it becomes false; what this change adds — that the clamp must use
  the same `selected_change()` the draw used — is carried as an ADDED requirement beside it
  rather than by rewriting it. Accepted rather than repaired: rewriting a requirement whose
  every sentence is still true, to add a cross-reference, is how a delta grows without saying
  anything.
- **The eight matrix rows whose verification is an existing check or test** — the four
  `dashboard-loop` scenarios, the two `artifact-content` read-binding scenarios, and the two
  `change-model` scenarios — are rows for requirements whose *text* moves while their
  verification does not. They are kept in the matrix deliberately: a row saying "unchanged" is
  what proves the requirement is still met after the edit.
