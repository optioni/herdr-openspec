## Reviewed Artifacts

- `proposal.md`
- `design.md`
- `tasks.md`
- `specs/artifact-folds/spec.md` (new capability)
- `specs/artifact-content/spec.md`
- `specs/detail-scroll/spec.md`
- `specs/list-selection/spec.md`
- `specs/mouse-input/spec.md`
- `specs/view-palette/spec.md` (added during review)
- `specs/responsive-layout/spec.md` (added during review)

## Reviewed Against

- This repository HEAD: `c9820c6` (`docs(foldable-spec-sections): propose foldable per-file
  artifact sections`). The finding pass began at `43de01e`/`027db45`; the package was
  repaired mid-review, so every finding below was re-verified against the committed text.
- Sibling repositories: **Not applicable.** This change adds no dependency, touches no
  manifest, and crosses no repository boundary. `~/Code/openspec-schemas` supplies the
  vendored `tdd` schema and `.claude/agents/`, neither of which this change edits.
- Working tree: clean apart from this change's own artifacts and the repairs recorded below.
- Four `planning-reviewer` subagents, one slice each — (A) capability coverage and
  cross-artifact contradictions, (B) design completeness, test boundaries, and check
  falsifiability, (C) task alignment and lifecycle, (D) factual verification. None was a fork
  of the planning session.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | proposal.md | `view-palette` is modified (two new `Role` variants) with no delta spec; four live requirements enumerate the role set exhaustively | Added the capability and a delta carrying the enum, modifier, colour, and `ui::view` mapping requirements | `proposal.md` → Capabilities; `specs/view-palette/spec.md` |
| CRITICAL | proposal.md | `responsive-layout` is modified (`Zone` gains a variant) with no delta; the live spec says "exactly five variants" | Added the capability and a delta carrying the `zone` requirement at six variants | `proposal.md` → Capabilities; `specs/responsive-layout/spec.md` |
| CRITICAL | design.md | **The header row's style could not reach the buffer at all.** `content_lines` returned `Vec<markdown::Line>`; the detail content area is styled only by `style_for(&Face)`, and `Face` is seven markdown-construct flags with no way to express a header. `REVERSED` appears nowhere in `src/` | `content_lines` returns `Vec<ContentRow>` carrying a `ContentKind`; `ui::view` alone maps kind → `Role`, on `ui::list::RowKind`'s precedent. `src/ui/markdown.rs` stays untouched | `design.md` → Decision 12; `specs/artifact-content/spec.md`; `specs/artifact-folds/spec.md`; `specs/view-palette/spec.md` |
| CRITICAL | design.md | Every `cargo test` filter in the verification matrix matched **zero** tests — the crate's tests live under `tests` modules (`ui::app::tests::…`), and a filter that matches nothing exits 0 | Inserted the `tests::` segment in all 122 Command cells and in `tasks.md`; verified the module paths against `cargo test --lib -- --list` | `design.md` → Test Strategy; `tasks.md` |
| CRITICAL | tasks.md | `NODEFAULT-UI` with `TYPES='Section'` fails on pre-existing `RowKind::Section {` patterns — half B's pattern is `(?<![A-Za-z0-9_])Section\s*\{` and `:` is not a word character. Reproduced: exit 1, nine false hits | Renamed the type `ArtifactSection`, which matches none of them. Re-measured in a scratch tree; the collision evidence is recorded in the group's own preamble | `tasks.md` group 4; every artifact naming the type |
| CRITICAL | specs/artifact-folds | `section_at`'s declared signature named a `Rect`, and `scripts/gates/notabseam.sh` greps the whole of `src/ui/detail.rs` — comments included — for `Rect`, so `make gates` would go red. "Mirror `ui::list::row_at`" misleads here: `src/ui/list.rs` is excluded from that sweep and this file is not | Dropped the parameter; `section_at(rows, offset, row)` is a saturating lookup, and bounding the click to the drawn region is `zone`'s job | `specs/artifact-folds/spec.md`; `design.md` → Boundaries; `tasks.md` 5.6, 5.8 |
| WARNING | design.md | `apply_click` opens with a `targets()` membership guard, and `targets()` yields only `Section`/`Change` — both new click targets would have returned early and every detail click would be silently inert | Scoped the guard to the two list arms and recorded why detail targets are exempt (resolved against the frame just drawn) | `design.md` → Decision 7; `tasks.md` 8.6; `specs/mouse-input/spec.md` |
| WARNING | tasks.md | Five spec scenarios had no owning task; one (`A click on a section header folds it exactly as Space does`) would have gone red at a "no regressions" step with nothing explaining why | Tasked all five, the click one in the group whose `ToggleSection` split causes it | `tasks.md` 5.3, 6.3, 7.2, 7.3 |
| WARNING | tasks.md | Task 2.6 named eleven scenarios but only three matched its own filter, and scheduled `expanded` assertions in a group where the field does not yet exist | Split: five in group 2, the four `expanded` ones in group 3 | `tasks.md` 2.6, 3.3 |
| WARNING | tasks.md | Group 2 mixed a `Makefile` edit into a `behavior` group, and asked for a per-list `SCAN_MIN` the gate cannot express | Lifted into operational group 4 with CHECK → CHANGE → VERIFY and a sixth recipe line carrying its own floor | `tasks.md` group 4 |
| WARNING | tasks.md | Seven behavior groups had neither a REFACTOR task nor a statement that none was needed | Added the statement to every group's verification task | `tasks.md` 1.3, 2.7, 3.5, 5.10, 6.7, 7.8, 8.9 |
| WARNING | tasks.md | Four GREEN tasks authored new assertions, inverting RED-before-GREEN | Moved the assertion-authoring halves into their groups' RED tasks | `tasks.md` 6.1–6.3, 7.3 |
| WARNING | tasks.md | The new capability's `## Purpose` and three now-false existing Purposes had no task; `openspec archive` writes a placeholder `tests/spec_purposes.rs` rejects, and a delta carries no Purpose | Added the task naming all four | `tasks.md` 11.5 |
| WARNING | tasks.md | Group 8's three new view scenarios were single-width, against this repository's 60-and-120 rule | Added the 60x40 leg | `tasks.md` 8.1 |
| WARNING | specs/artifact-content | The rewritten `content_lines` contract enumerated only `detail.problems`, dropping the mandated change-problems-first rule from a requirement a MODIFIED block replaces wholesale | Carried the owning requirement into the delta and stated that both problem sources are `ContentKind::Problem` and precede every header | `specs/artifact-content/spec.md` |
| WARNING | specs/list-selection | `Target`'s canonical enum declaration lives in a requirement not in any delta, while `mouse-input` added two variants — two capabilities would declare it differently | Carried the requirement, declaring four variants and noting the two detail ones are not in `targets()` | `specs/list-selection/spec.md` |
| WARNING | specs/mouse-input | The unmodified "Nothing becomes mouse-only" says "no key SHALL change its meaning", contradicting this change's own BREAKING `Space`; and the documented-binding requirement names five bindings where there are now seven | Carried both: narrowed the clause to "because of a mouse binding" with the reason, and extended the list to seven | `specs/mouse-input/spec.md` |
| WARNING | delta set | Deleting `Detail.source` left 25 sites in live specs naming it | Carried five owning requirements into the `detail-scroll`, `artifact-content`, and `responsive-layout` deltas (13 sites) and repaired the prose; the remaining 12 are tasked | `specs/*/spec.md`; `tasks.md` 11.4 |
| WARNING | specs/detail-scroll | Two re-asserted claims were false against the tree: the `TYPES` list is per-invocation, not one list; and the `Dashboard` companion names **fourteen** fields, not nine | Corrected both, and recorded that the stale count is fixed here rather than rediscovered | `specs/detail-scroll/spec.md` |
| WARNING | design.md | `cargo test --lib nodefault` runs zero tests — `nodefault` appears only in `tests/gate-controls.toml` | Named the real companion test | `design.md` → Test Strategy |
| WARNING | tasks.md | `tests/doc_contract.rs` has no key-binding leg; its one binding leg locates a `\| Gesture \| Action \|` table and its own control asserts `Err` on a `\| Key \| Action \|` one — so the BREAKING `Space` re-documentation had no computable second site | Stated that plainly and named the test that does prove the behaviour | `tasks.md` 7.6 |
| WARNING | tasks.md | The mouse doc-contract leg's subject is the backticked `Action::` variant set; both new gestures resolve to already-documented variants, so it is green with or without the new rows | Recorded the leg's real subject in the spec and required a delete/restore negative control | `specs/mouse-input/spec.md`; `tasks.md` 8.8 |
| WARNING | tasks.md | The persistence check (`grep -c 'agent-names'` unchanged) could not catch a *new* write and was unfalsifiable | Replaced with `scripts/gates/readonly-ui.sh` plus a plant/remove control | `tasks.md` 3.4 |
| WARNING | tasks.md | The acceptance fixture used a `ScratchDir` in a module that has none, a boundary the Test Boundaries table does not grant | Rebuilt on `changes::fixture` values and the recording reader | `tasks.md` 0.1 |
| WARNING | design.md | Two matrix rows verified against "the pre-change path" — no obtainable artifact after the change lands | Reduced to the property it stands for: rows equal `markdown::lines` cell for cell, every `kind` is `Body`, no cell reports `REVERSED` | `design.md`; `specs/artifact-folds/spec.md`; `specs/artifact-content/spec.md` |
| WARNING | tasks.md | The baseline block named HEAD `68b345e` and recorded the suite as red at HEAD | Corrected to `027db45`: the suite is green idle (1222/0), and the `ui::tests::wiring` failures are the load flake already diagnosed in `markdown-legibility`'s own design, with the signature that separates it from a regression | `tasks.md` → Baseline |
| WARNING | design.md | The in-flight-change analysis was wrong in both directions and missed a third change entirely: `markdown-legibility` touches neither `src/ui/detail.rs` nor `src/ui/layout.rs`, and `pane-chrome` rewrites `split_detail` and `zone` and replaces five palette roles | Replaced the assertion with a measured four-column table read from each change's own Boundaries, and recorded the resulting **sequencing constraint** — `pane-chrome` and this change are ordered, not parallel | `design.md` → Non-Goals, Risks |
| SUGGESTION | tasks.md | `parallel-after: 0` on group 1 was inert (a one-member set) and would have failed the attributability criterion anyway — worktrees are forbidden here and group 1's own test compiles the whole library | Dropped the marker; the sequencing paragraph now names group 1 and the reason | `tasks.md` → Sequencing |
| SUGGESTION | tasks.md | Task 2.5 mis-scoped the `.source` count as "53 outside `app.rs`" | Corrected to 31 in `app.rs` and 22 outside it | `tasks.md` 2.5 |
| SUGGESTION | tasks.md | Group 9's REFACTOR ended the group unverified | Added the re-run | `tasks.md` 9.2 |
| SUGGESTION | tasks.md | Two tasks both owned `SPEC.md`'s mouse table | Gave it to 8.8, which must edit it in-group or the doc-contract leg goes red | `tasks.md` 8.8, 11.1 |
| SUGGESTION | tasks.md | Task 1.1 argued a point the design already settles | Trimmed to the citation | `tasks.md` 1.1, 1.2 |
| SUGGESTION | specs/artifact-folds | A backward reference to `detail.source` read oddly for an archived spec | Reworded to "the concatenated source before this change" | `specs/artifact-folds/spec.md` |

## No Remaining Implementation-Blocking Gaps

None remain. Every CRITICAL is repaired in the artifact that owns it, and both gate-level
CRITICALs were reproduced in a scratch tree before and after the repair rather than reasoned
about. All 122 scenarios have a verification-matrix row and an owning task, checked
mechanically rather than by reading: scenario names extracted from the seven spec files and
diffed against the matrix's row labels, `missing 0 / orphan 0`.

One scheduling decision is the user's, not a planning gap: `pane-chrome` and this change
conflict on `src/ui/layout.rs` and `src/ui/palette.rs`, and `design.md` → Risks recommends
landing `pane-chrome` first because this change is additive onto its new shapes.

## Deferred Non-Blocking Notes

- **A scenario title survives that contradicts its own body.** `artifact-content` ::
  `A multi-file artifact is concatenated in path order with a separating newline` now has a
  body stating the opposite — one section per path, verbatim, no separator. The title cannot
  be corrected: `openspec validate --strict` requires a MODIFIED block to carry every scenario
  name the live spec has, and renaming fails validation (observed). Resolution point: the
  archive step, or a follow-up change that can drop and re-add the requirement.
- **Twelve `detail.source` sites remain in three capabilities this change does not otherwise
  touch** — `tasks-checklist`, `tasks-progress-bar`, `live-updates`. Their behaviour is
  unchanged, so carrying five behaviourally identical requirements as MODIFIED blocks would be
  several hundred lines of copy for a field rename. Resolution point: `tasks.md` 11.4, which
  renames the prose and verifies with `grep -rn 'detail\.source' openspec/specs/` returning
  nothing.
- **`make gates` was exiting 2 at review time** solely on `OPENSPEC-UNTOUCHED`, naming this
  change's own untracked artifacts. Resolved by committing them: re-run at `c9820c6` gives
  exit 0 with 53 OK lines and no failures.
