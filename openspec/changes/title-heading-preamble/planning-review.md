## Method

Four `planning-reviewer` subagents were dispatched simultaneously, none of them a fork of the
writing session, each given the change directory and one slice: **A** capability coverage,
delta fidelity, scenario quality and cross-artifact contradictions; **B** design completeness,
test boundaries and whether each proposed check could fail at all; **C** task alignment,
lifecycle discipline and group independence; **D** factual verification of every empirical
claim. Reviewers reported findings only and edited nothing; `git status` was clean after each.

Reviewed against HEAD `6386318`. A, B and D each built a probe against the real crate
(`ui::app::split_headings`, `tasks::count`/`parse`, `ui::detail::content_lines`) in a scratch
copy rather than re-implementing the rules, so every corpus number below comes from the crate's
own functions. D additionally drove the pane live in a 72×30 tmux pty.

**1 CRITICAL, 17 WARNING, 5 SUGGESTION.** Every CRITICAL and WARNING is repaired below. The
CRITICAL and four WARNINGs were raised independently by two reviewers each, which is the
signal that the slicing worked rather than that the reviewers agreed with each other.

## Delta fidelity — the comparison that was run

A extracted both then-carried requirement blocks from the live specs at HEAD and from the
delta files by script and diffed them:

- `artifact-folds` → *A multi-file artifact's content is a list of named sections* — live 251
  lines, four hunks, **zero deletions of landed work**, every existing scenario byte-identical.
- `artifact-content` → ``Dashboard::sync_detail` resolves the selected tab's content`` — live
  213 lines, **one** hunk (step 4 only), all eleven scenarios byte-identical.

Three further blocks were carried during the repair (below). The same script-extract-then-diff
was re-run over all five afterwards: no block is byte-identical, every removed line is one of
this change's intended edits, and no block reverts landed work. Re-run it before implementing
if anything archives in the meantime — a carried body goes stale silently.

## CRITICAL

**C1 — the corpus guard could not reach the rule it guards.** *(B, and C independently)*
`title_heading` was specified as a private helper in `src/ui/app.rs` while the guard was sited
in `tests/`, which links only against the public API; `RecordingReader` and
`changes::fixture::with_artifacts` are both `pub(crate)`. The only way to satisfy the task as
written was to re-implement the three clauses and the contribution arithmetic inside the test —
a second implementation of the rule under guard, **green even if `title_heading` were never
written**. B confirmed the sound alternative compiles from `tests/`: every field it needs is
`pub`, and `NOLIT-CHANGE` scans `src/` only.

*Repaired* in design.md → Boundaries and tasks.md 2.2: the guard reads each `tasks.md`, builds
a one-artifact `Dashboard` with `tracks_tasks: true`, calls the real `Dashboard::sync_detail`
with a closure over those bytes, and asserts `detail.sections.len() > 1` — exercising the real
derivation and the real split gate. The helper stays private.

## WARNING — specification

**W1 — the emptiness predicate was undefined, and the corpus sits exactly on the boundary.**
*(A, B, D)* A raised this as CRITICAL and downgraded it itself after re-measuring. The three
files design.md named as having an *empty* title body carry `"\n"`; **no** file in either tree
has a byte-empty one. Byte emptiness and trimmed emptiness are not equivalent in general, and
the spec said only "non-empty".

*Repaired*: the delta now states **trimmed**, records that this deliberately differs from the
preamble's byte predicate and why the two are not unified, and design.md → Decisions **D8**
carries the argument. Measured both ways over 58 task files in two trees: **zero** change their
split decision between the readings (minimum post-demotion contribution count 7), so the choice
is between a section that draws zero rows and no section at all. A new scenario, *A
whitespace-only title body contributes no section*, pins the shape the corpus actually has.

**W2 — the delta contradicted itself on when a title is recognised.** *(A)* It said a title is
recognised "only in a file that **splits**" while the split was decided *from* the
post-demotion contribution count. *Repaired*: recognition is now from the heading list alone,
and the delta states the order — recognise, derive, count, then decide — and says why the
reverse is circular.

**W3 — three sentences of an unmodified `artifact-folds` requirement are falsified.** *(A)*
*A section header row names the file and shows its fold state* asserts at `:529` that "a task
file that opens with a level-1 title puts every `## ` group at depth 1", describes a scenario
fixture at `:914` as that same shape, and says at `:739` "only heading sections gain a cell",
which the delta's own "**labelled** heading sections" wording contradicts. *Repaired*: the
requirement is carried into the delta as a third MODIFIED block with all three corrected.

**W4 — the split-gate requirement did not carry the contribution condition.** *(A)*
*Repaired*: carried as a second MODIFIED block, with the two conditions and their order stated.

**W5 — proposal.md promised an `artifact-content` edit the delta did not make.** *(A)* The
stale sentences were in a different requirement: *The content area renders the artifact…*
enumerates `None`-labelled rows as *preamble* rows at `:321` and `:326`, and a demoted title's
rows are neither. *Repaired*: that requirement is carried as a second `artifact-content`
MODIFIED block with both enumerations widened.

**W6 — S5's `ToggleSection` assertion passed vacuously.** *(B)* `apply` dispatches on
`self.route`, and `detail_cursor_section` returns `None` whenever `drawn_width` is `None`, so
the check was green by construction. *Repaired*: the WHEN now requires `route: Route::Detail`
and `drawn_width: Some(78)`, and says why.

**W7 — S2 asserted a label its WHEN did not pin.** *(A)* *Repaired*: the path is named.

**W8 — `min_level` was undefined when demotion leaves no labelled heading.** *(B)* Reachable
via `- [ ] x\n\n# T\n\nprose\n`. *Repaired*: the delta states `0` and names the shape.

## WARNING — design

**W9 — the stated reason for taking no outer-loop test was factually false.** *(B)* design.md
said `run_loop` "cannot be driven without a terminal"; it is driven **68 times** in
`src/ui/driver.rs`'s own tests against a `TestBackend` and the `none()` seams. The conclusion
was sound and is kept; *repaired*: the reason is now that `run_loop` adds only event plumbing
this change does not touch.

**W10 — Test Boundaries omitted the corpus tier and then forbade it.** *(B)* The table had only
acceptance and unit columns and closed "No task may introduce a real filesystem", which is
exactly what the corpus guard does. *Repaired*: the columns are now *corpus guard* and *unit
and view tests*, the filesystem row names the guard's real read-only access, and the
prohibition is scoped to unit and view.

**W11 — design.md and tasks.md disagreed about where the render tests live.** *(A, B)* The
matrix named `ui::app` and `ui::detail`; task 2.1 sited them in `src/ui/view.rs`. *Repaired*:
`src/ui/view.rs` throughout, with a Boundaries row and corrected matrix commands.

**W12 — the doc-drift mitigation named a binding that does not exist.** *(B, C, D)* Nothing in
`tests/doc_contract.rs` reads `SPEC.md`'s or `AGENTS.md`'s section-derivation prose; it binds
the module map's *names* column, the tested-modules list, the MSRV, the gate programs, the key
and mouse tables, the seam names, OSC 52 and the drag-to-select note. The same false claim
appeared in three artifacts. *Repaired in all three*: proposal.md → Impact, design.md →
Boundaries and → Risks, and tasks.md 4.4, which now says explicitly that the run is a
regression check and not evidence for the rewrites above it.

## WARNING — tasks

**W13 — group 3 was marked `behavior` with a RED that cannot be red.** *(C)* Its check is green
at HEAD and proven by a negative control, which the schema labels CHECK, not RED, and the group
had no GREEN at all. *Repaired*: it is now group 2, `operational`, CHECK → CHANGE → VERIFY.

**W14 — group 2 was a `behavior` group whose RED was already green.** *(C)* Groups are
sequential, so by the time it ran the derivation had landed; 2.2 itself said "No production
change is expected", and it carried no at-HEAD evidence. *Repaired*: its render tests moved
into group 1 as task 1.2, where they are genuinely red, and the group is gone.

**W15 — task 1.1 named `shape_of` for assertions it cannot make.** *(C, D)* `shape_of` returns
`(label, depth)` **pairs**; the demoted title's `progress` becoming `None` is the headline
`SHALL` and is invisible to it, as is S6's `operation`. *Repaired* in tasks.md 1.1 and in the
design matrix.

**W16 — one scenario's unit half was owned by no task.** *(C)* *A title heading with no prose
under it does not split the file* appeared only in the render group. *Repaired*: it is in 1.1's
list with its assertions.

**W17 — 1.6's regression run was narrower than group 1's blast radius.** *(C)* `sync_detail` is
driven from `src/ui/view.rs`, `src/ui/detail.rs` and `src/ui/driver.rs` as well. *Repaired*:
task 1.7 runs `cargo test --lib`, and says why.

## SUGGESTION

- **S1 — the corpus denominator was one stale.** *(A, D)* 48 at `8ccd255`, **49** at HEAD —
  this change's own `tasks.md` joined the corpus it measures. Swept: proposal.md, design.md,
  and tasks.md all corrected, with the measurement commit named where a check recorded 48.
- **S2 — the prose is 174 rows, not "roughly 140".** *(D)* Corrected in proposal.md and
  design.md.
- **S3 — group 4 left a second false sentence standing in `SPEC.md`.** *(C)* The `| ui |`
  module-map row at `SPEC.md:99` carries the same claim as the derivation paragraph. Taken:
  task 4.1 names both sites, and 4.3 adds the stale doc comment at `src/ui/detail.rs:4749`.
- **S4 — two blank rows now precede the first fold header on a demoted file.** *(B)* Existing
  preamble behaviour, newly visible on 17 files. Accepted, not repaired: it is `markdown`'s
  trailing blank plus the section separator, and changing it is `artifact-content`'s argument.
- **S5 — the guard's tree lookup was unstated.** *(B)* Taken: `env!("CARGO_MANIFEST_DIR")`,
  recorded in design.md → Boundaries and tasks.md 2.2.

## Number sweep

Every figure the artifacts assert was re-derived from a command, and each was fixed in **all**
artifacts that quoted it rather than only where it was raised: the corpus denominator
(48 → 49), the prose row count (140 → 174), the title-body shape (`""` → `"\n"`, with the
byte/trim consequence), the minimum post-demotion contribution count (7, newly stated), and the
`run_loop` drive count (68, newly stated). D confirmed the remaining figures unchanged: 17
demoted here, 2 in `slot-car-racing`, all 19 level-1, all 19 passing clause 3, and the RED
probe's exact left value.

## Verification

`openspec validate title-heading-preamble --strict` → valid. tasks.md is 155 lines against
design.md's 290, so the plan has not outgrown the design it implements.

## Open questions

None. The one design question — fix the cell, the nesting, or both — was put to the user before
drafting and answered: demote the title, leave the cell rule alone.
