# Planning Review — markdown-constructs

## Reviewed Artifacts

- `proposal.md`
- `specs/markdown-render/spec.md`
- `specs/view-palette/spec.md`
- `specs/detail-scroll/spec.md`
- `specs/degraded-coverage/spec.md`
- `design.md`
- `tasks.md`

The finding pass was delegated to four `planning-reviewer` subagents, each given the change
directory and one slice, none of them a fork of the session that wrote the plan: **(A)**
capability coverage, scenario quality, cross-artifact contradictions; **(B)** design
completeness, test boundaries, and whether each proposed check could fail at all; **(C)** task
alignment, lifecycle discipline, `parallel-after` independence; **(D)** factual verification of
every empirical claim, by running the command or reading the source. The reviewers edited
nothing; every repair below was made by this session in the artifact that owns it.

## Reviewed Against

- This repository HEAD: `e63442b` (`docs(markdown-constructs): draft proposal deltas, design,
  and tasks`). Three commits earlier than the plan's own recorded baseline `94efd85`, because
  another session archived `color-palette` (`4fd997c`, `e9b18b1`, `6ba1480`) while this plan
  was being written. `src/`, `tests/`, `SPEC.md`, `Makefile`, and `scripts/` are byte-identical
  across that span, so every planning-time check recorded in `tasks.md` remains valid; what
  changed is that `openspec/specs/view-palette/` became live, which this change depends on.
- Sibling repositories: `~/Code/openspec-schemas` is the graft source for
  `openspec/schemas/tdd/` and `.claude/agents/`. Not applicable — this change touches neither
  and adds no schema or agent.
- Pinned crates verified directly: `pulldown-cmark 0.13.4`, `ratatui 0.30.2` /
  `ratatui-core 0.1.2`.
- Working tree: clean at review start. The four artifacts under review were intentionally
  committed first, because `scripts/gates/openspec-untouched.sh` fails `make check` on any
  untracked file inside `openspec/`.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | `specs/markdown-render`, `specs/degraded-coverage` | `openspec validate --strict` **failed**: both deltas renamed a `#### Scenario:` inside a MODIFIED requirement. OpenSpec's `RENAMED` is requirement-level only, and a MODIFIED block may not drop a scenario the live spec still has. | Restored both headers verbatim and narrowed only their bodies, adding the merge-key note `detail-scroll` already uses twice. The departed constructs became each scenario's **discriminating control**, so the scenario now fails if the narrowing is not real. | `specs/markdown-render/spec.md` → "A table renders as its literal source text, one row per line"; `specs/degraded-coverage/spec.md` → "A footnote, strikethrough, and a table each render as literal source" |
| CRITICAL | `specs/markdown-render` | "A table that fits" asserted three `text()` literals of 24, 24, and 22 columns implying three different column allocations, in a bullet also asserting every line measures the same total. None matched the requirement's own rule, which gives `w = [6, 12]`, `total = 25`. A test transcribed from them is red against a correct allocator. | Replaced with the four lines the rule actually produces, shown as a fenced block, with the derivation (`nat`, `w`, `total`) stated so the literals are checkable rather than trusted. | `specs/markdown-render/spec.md` → "A table that fits renders as aligned columns at both mandated widths" |
| CRITICAL | `specs/markdown-render` | The alignment scenario's fixture used single-character cells, so `nat = [1,1,1]`, no padding exists, and left/centre/right render **byte-identically** — the scenario would pass against an implementation that ignores `Alignment` entirely. Confirmed by running the fixture through pulldown-cmark 0.13.4. | Widened the header cells to `left`/`cent`/`rght` so `w = [4,4,4]` and the asserted `x   ` / ` x  ` / `   x` are reachable, and added a leg requiring the three renderings to differ from one another. | `specs/markdown-render/spec.md` → "Alignment markers pad the cell on the side they name" |
| CRITICAL | `specs/detail-scroll`, `tasks.md` | The outer-loop acceptance scenario asserted the wide layout's divider columns (39, 40) against **both** buffers. Below the breakpoint `layout::split_body` gives the detail region the whole body, and a table wrapping at 58 necessarily covers those columns — so the one test the outer loop exists to vouch for was permanently red against a correct implementation. | Split the assertion per buffer (0/39/40/119 at 120 columns; 0/59 at 60) and recorded why the lists differ, so the next reader does not "fix" it back. | `specs/detail-scroll/spec.md` → "A table reaches the buffer aligned and inside the region"; `tasks.md` 0.2 |
| CRITICAL | `tasks.md` | Group 1 could not reach green: enabling `ENABLE_TABLES` reddens two HEAD tests that pin table-as-literal-source, but their removal was scheduled in group 2, while 1.3 and 1.8 claimed "every pre-existing markdown test still passes" and "no regressions". | Added `1.2 CHANGE` deleting both before the flag is turned on, with its own recorded HEAD result, and reworded 1.4 and 1.9 to except them explicitly. | `tasks.md` group 1 |
| WARNING | `design.md`, `tasks.md` | **Seven** test names cited in the verification matrix and the tasks did not exist — invented by transcribing scenario titles instead of reading `src/`. Each filter returns `ok. 0 passed; 1112 filtered out`: green, having run nothing. | Corrected all seven against the tree, then added a mechanical audit of every identifier in the matrix against `fn` definitions in `src/`+`tests/`; the only non-resolving names are now the nine tests this change adds. | `design.md` → Test Strategy; `tasks.md` 5.1 |
| WARNING | `design.md` | The verification matrix carried 1 of `degraded-coverage`'s 13 scenarios. A MODIFIED requirement reproduces every scenario it does not change, and the repository convention (archived `color-palette`, 56 rows) is to carry them all. | Added the 12 missing rows as regression rows naming their existing proofs, then verified the matrix against the specs **in both directions** — 45 scenarios, 0 missing, 0 rows naming a scenario that does not exist. | `design.md` → Test Strategy |
| WARNING | `design.md`, `specs/markdown-render` | Decision 8 ("pad short, drop surplus") rested on a false premise: pulldown-cmark already pads short rows and truncates long ones before `fold` sees them, so the code would be unreachable and the scenario would pass against an implementation that does nothing. | Restated the decision as a parser observation, removed the invented logic, and rewrote the scenario as a **regression guard on the parser** that asserts the event stream directly. | `design.md` → Decision 8; `specs/markdown-render/spec.md` → ragged-table requirement and scenario |
| WARNING | `proposal.md`, `design.md`, `specs/markdown-render` | The corpus figure "89 of 238 files / 3,764 rows" reproduces at no commit under any detection variant (the only 238-file commit measures 94 / 3,525), and my own re-measurement moved with every artifact edit (115 / 120 / 121 depending on tree state and whether `git grep` or GNU `grep` ran). | Withdrew the number from all three artifacts rather than patching it, kept the qualitative claim plus a runnable command, and promoted the **zero** measurements — which are stable and are what actually bound the change — to the load-bearing role. | `proposal.md` → Why; `design.md` → Context; `specs/markdown-render/spec.md` → narrowing rationale |
| WARNING | `specs/markdown-render` | The line-grammar bullet omitted the interior `|`, producing `\| a  b \|` and contradicting both the `total = 3n + 1 + sum(w)` formula and the ragged scenario's "exactly four `\|`". | Restated the per-column unit as ` ` + cell + ` ` + `\|` after a leading `\|`, with the pipe count (`n + 1`) stated so the formula is checkable. | `specs/markdown-render/spec.md` → table requirement |
| WARNING | `specs/markdown-render` | "Every line a table emits SHALL measure exactly `total`" contradicted the narrow fallback in the same requirement, which emits no pipes and is bound by `width`. | Scoped the sentence to the pipe grammar and stated the fallback's own bound beside it. | `specs/markdown-render/spec.md` → table requirement |
| WARNING | `specs/markdown-render` | A table nested in a block quote or list item was unaddressed, and the container-prefix requirement and the table requirement contradicted each other for that case. | Specified it: a nested table lays out in the columns its container leaves, every line carries the prefix, `total` is measured against the reduced width, and the fallback applies inside the container. | `specs/markdown-render/spec.md` → table requirement; `tasks.md` 1.6 |
| WARNING | `tasks.md` | The parallelism paragraph named the wrong groups in all three of its claims (`markdown.rs` is 1/2/3 not 1/2/4; `view.rs` is 0/5/6 not 3/5; documents are group 7 not 6). The conclusion held but its evidence was unverifiable. | Rewrote it as an explicit file map, and stated the one genuinely close call (groups 7 and 8) with the reason it stays sequential. | `tasks.md` → preamble |
| WARNING | `tasks.md` | Contract gates were misplaced: 3.4 inspected `ui::palette::Role` before group 4 creates it, and group 4 — which does change `Role` — had no gate at all. | Scoped 3.4 to `Face` and added 4.3 for `Role` and its `style_for` consumer. | `tasks.md` 3.4, 4.3 |
| WARNING | `tasks.md` | Task 3.4's check could not find what it claimed: a line-wise `grep -v '..'` reports every multi-line `Face` literal as unspread, because the spread sits on a later line. Recorded as "one hit"; actually 11. | Replaced with a block-reading check, **executed at planning time with a negative control** — planting a second fully-spelled literal makes it report `count=2` and name the line; removing the plant returns `count=1` with `git status --porcelain src/` clean. | `tasks.md` 3.4 |
| WARNING | `tasks.md` | 7.3 claimed `tests/degraded_coverage.rs` fails on "a `covers` range the suite does not execute". It does not — that checker is `#[ignore]`d and needs a coverage run, which is why the suite reports "10 passed; 1 ignored". | Corrected the claim and named `make coverage` as what actually proves execution. | `tasks.md` 7.1, 7.3 |
| WARNING | `tasks.md` | 7.3 re-pointed `covers` at the new `Options` line. Semantically wrong: the row that **remains** is about footnotes and task-list items rendering literally, which `Event::Text` produces; the `Options` line is what makes the departed constructs leave the row. Either passes the gate, so only semantics distinguish them. | Kept `covers` on the literal-text line, re-measured for the line shift, with the reasoning recorded in Decision 10 rather than in the task. | `tasks.md` 7.3; `design.md` → Decision 10 |
| WARNING | `tasks.md`, `design.md` | `scripts/gates/widths.sh` (`WIDTHS`, floor 113 over `src/ui/view.rs`) is a gate this change moves — groups 0, 5, and 6 all add tests there — and neither Test Boundaries nor the gate-floor group named it. | Added it to both, with its floor recorded at HEAD. | `design.md` → Test Boundaries; `tasks.md` 8.1, 8.2, 11.1 |
| WARNING | `tasks.md` | Group 2's struck-run fixture leg could not be written until 3.2 lands, so the group's own verification was unreachable. | Split the fixture work: group 2 carries the table legs, group 3 carries the struck-run legs alongside the flag that makes them parseable. | `tasks.md` groups 2 and 3 |
| WARNING | `openspec/specs/markdown-render` (live) | The live `## Purpose` states that tables and strikethrough "render as their literal source". A delta carries no `## Purpose` block and `tests/spec_purposes.rs` checks only non-emptiness, so this contradiction would survive the archive silently. | Added an explicit task to narrow it on sync, and named it in the proposal's Impact → Docs. | `tasks.md` 7.4; `proposal.md` → Impact |
| WARNING | `tasks.md` | 10.2 claimed to *rewrite* an `AGENTS.md` sentence about a fixed parser subset. No such sentence exists (`grep` over `AGENTS.md` returns nothing; the prose lives only in `SPEC.md`), so it is an add, and the net-size claim was false. | Restated as an add of one sentence with a CHECK task carrying the recorded grep, and noted that `CLAUDE.md` is a symlink to `AGENTS.md` so only one file is edited. | `tasks.md` 10.1, 10.2 |
| WARNING | `tasks.md` | 10.1 was a no-op checkbox restating work already done in 7.2, which an implementer would meet as an unchecked box with nothing to do. | Deleted it; the cross-reference is now one line of group preamble. | `tasks.md` group 10 |
| WARNING | `tasks.md` | Two behavior groups carried operational `CHANGE:` markers, and group 10 carried no lifecycle markers at all. | Reworked group 10 into CHECK → CHANGE → VERIFY; folded 5.3's extension work into 5.1 (RED) so group 5 is RED → GREEN → REFACTOR; 1.2 and 2.x now sit in a lifecycle that admits them. | `tasks.md` groups 1, 5, 10 |
| WARNING | `design.md` | Test Boundaries omitted three real collaborators: `ui::tasks::lines`, `ui::layout::columns`/`truncate_columns`, and `ui::detail::content_lines`. | Added all three with their real/replaced status. | `design.md` → Test Boundaries |
| SUGGESTION | `design.md` | The `color-palette` sequencing constraint read as an open blocker; it was satisfied in `6ba1480`, the parent of the commit that added the file. | Restated as satisfied, naming the commit. | `design.md` → Open Questions |
| SUGGESTION | `specs/markdown-render` | Three counting slips: "the two widths at which the table cannot hold its pipe grammar" (four qualify); the narrow-region scenario's WHEN sampled 0–4, 8, 9 while its THEN asserted 1–8; the wide-cell scenario left its header row and first column unspecified while asserting `w[0]`'s behaviour. | Replaced the count with the qualifying widths, made the WHEN sweep 0 through 10, and gave the wide-cell fixture an explicit header and first column. | `specs/markdown-render/spec.md`, three scenarios |
| SUGGESTION | `specs/markdown-render` | The zero-column-table clause asserted an observable rendering, but no `&str` source yields `Tag::Table([])` — `\|\|` and `\|-\|` both give one column. | Restated as a `fold`-level totality guard with its unreachability recorded, and deliberately given no scenario. | `specs/markdown-render/spec.md` → table requirement |
| SUGGESTION | `tasks.md` | Decision 1 asks for both wildcard comments to be updated; only one task said so. | 1.3 now names both the `Event::Start`/`End` wildcard and the bare-`Event` one. | `tasks.md` 1.3 |

Two SUGGESTIONs were considered and **not** taken, each with its reason:

- *Mark groups 7 and 8 `parallel-after: 6`.* They share no file and both depend only on group
  6, but group 8 measures gate floors on the finished tree and 8.3 copies that tree into a
  scratch directory, so a mid-edit `SPEC.md` from group 7 is copied into the run. The schema's
  own rule is to mark conservatively; the cost of a false positive is two agents in one tree.
- *Add a task recording the `color-palette` archive-ordering constraint.* The constraint is
  satisfied at HEAD, so a task for it would be a checkbox with nothing to do — the same defect
  as the deleted 10.1.

## No Remaining Implementation-Blocking Gaps

None remain. `openspec validate markdown-constructs --strict` reports **valid**. The
verification matrix covers all 45 scenarios with no orphans in either direction, checked
mechanically rather than by reading. Every test name cited in an artifact either resolves to a
`fn` in `src/`/`tests/` or is one of the nine tests this change adds. Every planning-time check
has a recorded exit status, and the one check that pins an existing invariant carries an
executed negative control.

No decision requires user input.

## Deferred Non-Blocking Notes

- **The narrow one-cell-per-line fallback has no visual design beyond "preserve every
  character".** It fires only below `4n + 1` columns — nine columns for a two-column table —
  which is far below the 58-column narrow interior and is reached in practice only by the
  all-widths sweep. Recorded in `design.md` → Decision 7 with its alternatives; if a real
  narrow-pane reading experience is ever wanted, that decision is where it is reopened.
- **`ENABLE_TABLES` changes how existing prose parses**, in principle. The false-positive
  surface is narrow (GFM requires a delimiter row) and is guarded by the unchanged
  paragraph-wrap and block-separation tests plus the composite-fixture sweep. Recorded in
  `design.md` → Risks; no separate corpus-diff task, because parsing the corpus with both new
  flags was measured during review and yields exactly one strikethrough span and no
  reinterpreted prose.
