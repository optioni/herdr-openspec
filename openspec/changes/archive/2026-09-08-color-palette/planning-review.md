## Reviewed Artifacts

- `proposal.md`
- `design.md`
- `tasks.md`
- `specs/view-palette/spec.md` (new capability)
- `specs/artifact-tabs/spec.md`
- `specs/change-rows/spec.md`
- `specs/detail-header/spec.md`
- `specs/detail-scroll/spec.md`
- `specs/list-selection/spec.md`
- `specs/responsive-layout/spec.md`

The finding pass was delegated to four `planning-reviewer` subagents dispatched simultaneously,
one slice each, none of which wrote the plan: (A) capability coverage, scenario quality, and
cross-artifact contradictions; (B) design completeness, test boundaries, and whether each
proposed check can fail at all; (C) task alignment, lifecycle discipline, and `parallel-after`
independence; (D) factual verification of every empirical claim by running the command or
reading the source. They reported findings and changed nothing; this session made every repair
below.

## Reviewed Against

- This repository HEAD: `84574f3` — the planning package's own commits. The last commit
  touching code, scripts, or the `Makefile` is `d4b1221`, which is what every measured number
  below was taken against.
- Sibling repository (`~/Code/openspec-schemas`, source of the vendored `tdd` schema and the
  orchestration agents) HEAD: Not applicable — this change touches neither the schema nor an
  agent definition, and adds no rule that would belong in one.
- Working tree: clean apart from the change's own artifacts, which were committed as they
  landed (`96505fa`, `c3a9203`, `3806cb2`, `84574f3`).

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | every render scenario; `design.md` | The `Color` confinement gate sweeps `src/` whole-file, and this crate's view tests are inline `#[cfg(test)]` modules there, so a scenario demanding `reports foreground Color::Green` would have turned `make gates` red on its own RED task. Reproduced by reviewer D against a scratch tree | Colour is asserted by comparing against `palette::style(role)`; the literal table is asserted once, in the palette's own tests. The pair is jointly falsifiable — the comparison catches a forgotten role, the table catches a wrong colour | `specs/view-palette/spec.md` → "How a test asserts a colour"; every render scenario in six spec files; `design.md` → Decision 2 |
| CRITICAL | `specs/artifact-tabs/spec.md`; `tasks.md` | The capability's fourth requirement was left untouched, and its last scenario asserts a `3 …` bold label the chip grammar removes; twelve landed assertions in `src/ui/view.rs` spell a numbered label with no task owning them | The fourth requirement is restated in full with the chip label; task 4.1 names all twelve lines by number | `specs/artifact-tabs/spec.md`; `tasks.md` 4.1; `proposal.md` |
| CRITICAL | `tasks.md` 0.2/1.4/5.1; `design.md` | The outer-loop acceptance test was signed off by `cargo test --test gate_controls palette`, which runs **zero** tests and exits 0: controls are table rows iterated by one `#[test]`, so no test name contains `palette`. `design.md` also named a function that does not exist | Every task names `cargo test --test gate_controls catch_their_plants` or the whole target; `design.md` names `gate_controls_catch_their_plants` over the `palette-outside` row, and says why the filter vouches for nothing | `tasks.md` 0.2, 1.4, 5.1; `design.md` → Test Strategy |
| CRITICAL | `design.md` → Decision 3; `tasks.md` 2.5 | "No modifier expectation moves" is false for the tab-bar row, and `grep \| wc -l` cannot see an *edited* line, so the check could not fail | Decision 3 is scoped to exclude the tab bar and names the two tests group 4 rewrites; 2.5's check becomes a diff over `Modifier::` lines | `design.md` → Decision 3; `tasks.md` 2.5, 4.2 |
| WARNING | `specs/view-palette/spec.md` | The monochrome-invariance requirement claimed the whole frame, which is false for the row `artifact-tabs` relabels and moves | Scoped to "every cell outside the artifact tab-bar row", with the bar's own narrower guarantee stated: the selected chip stays that row's only `BOLD` span | `specs/view-palette/spec.md` → req 3 |
| WARNING | `specs/change-rows/spec.md` | The archived-badge scenario's prose gave an offset the fixture disproves; the requirement above it was right | Reworded to `name_field_width + 14` and to assert equality with the **active** row's badge column at the same width, which discriminates | `specs/change-rows/spec.md` |
| WARNING | `specs/artifact-tabs/spec.md` | "A tenth artifact is labelled without a digit" is vacuous at 58 columns, where no tenth cell fits | Split by width: twelve cells at 78, nine at 58, so the width is load-bearing | `specs/artifact-tabs/spec.md` |
| WARNING | `proposal.md`; `specs/responsive-layout/spec.md` | `RegionBorder`/`RegionBorderFocused` were mapped in `view-palette` and rewritten by a task, but no requirement owned border styling and no scenario covered it | ADDED requirement with a scenario that swaps the route, so it discriminates rather than asserting a constant | `specs/responsive-layout/spec.md`; `proposal.md` |
| WARNING | `specs/view-palette/spec.md`; `tasks.md` | `NOIO-VIEW` and `COLWIDTH` hard-code their `PURE` lists and only check that the files they name exist, so skipping the list edit would leave the palette unswept while both printed `OK` | `palette.sh` gains a third leg asserting membership in both lists, with its own planted control; the leg was written and run RED-then-GREEN at planning time | `specs/view-palette/spec.md` → req 1; `tasks.md` 0.1, 0.2, 1.3 |
| WARNING | `tasks.md` 0.2 | A gate with three failure modes carried one planted control; the other two were "verified" by a green run on a clean tree | Three controls: `palette-outside`, `palette-rgb`, `palette-unswept` | `tasks.md` 0.2 |
| WARNING | `specs/view-palette/spec.md`; `tasks.md` 2.3 | `render_footer` writes a bare `Style::default()`, so "`ui::view` constructs no `Style` of its own" was false on the tree this change produces | `Role::Footer` added to the enum and both tables; `render_footer` added to the task | `specs/view-palette/spec.md`; `tasks.md` 2.3 |
| WARNING | `tasks.md` group 6 | Three floors were re-measured; adding one file under `src/ui/` moves seven more, every one of them sitting exactly on its measured value today | The CHECK and CHANGE tasks cover all ten defaults, plus a check on the coverage production floor | `tasks.md` 6.1–6.5 |
| WARNING | `tasks.md` group 8 | `SPEC.md:434-437`'s tab-label sentence goes false and nothing in `tests/doc_contract.rs` binds it | Its own task, and a `8.0 CHECK` that reads every such sentence against the tree | `tasks.md` 8.0, 8.4 |
| WARNING | `tasks.md` 2.1 | `WIDTHS` requires **every** `#[test]` in `src/ui/view.rs` to name 60 and 120, with no exemption; the task promised it only of render tests | Stated for the two pure `style_for` tests too | `tasks.md` 2.1 |
| WARNING | `design.md` → Test Boundaries and matrix | One scenario was tiered onto a real `ScratchDir` inside `src/ui/view.rs`, which `NOIO-VIEW` sweeps whole-file — a view test there cannot open a directory | Retiered to an in-memory `Dashboard`; the filesystem row now reads "replaced" with the reason | `design.md` |
| WARNING | `tasks.md` groups 2, 3, 4 | No REFACTOR step, which the behavior lifecycle requires either as a task or as an explicit "none was needed" | A REFACTOR task in each | `tasks.md` 2.6, 3.6, 4.7 |
| WARNING | `tasks.md` group 8 | An operational group led with a CHANGE task | `8.0 CHECK` added and 8.1–8.6 labelled CHANGE | `tasks.md` group 8 |
| SUGGESTION | `specs/artifact-tabs/spec.md` | The drop-whole exception named only the *selected* chip, leaving the `no artifacts` placeholder's truncation unruled — pre-existing in the live spec | Widened to "the anchor chip", which Decision 7 already implied | `specs/artifact-tabs/spec.md` |
| SUGGESTION | `specs/view-palette/spec.md` | "the production slice of `src/ui/palette.rs`" was looser than the gate, which greps the whole file | Dropped | `specs/view-palette/spec.md` |
| SUGGESTION | `specs/artifact-tabs/spec.md` | `TabActive`'s `Black`-on-`Cyan` contrast is the change's one brightness assumption and no scenario asserted the foreground | Added to the buffer scenario | `specs/artifact-tabs/spec.md` |
| SUGGESTION | `tasks.md` 0.1 | The recipe position was wrong, and the recorded plant line `4286` is past the end of a 4284-line file | Corrected to after `openspec-untouched.sh`, and to `:4285` with the `wc -l` that gives it | `tasks.md` 0.1 |
| SUGGESTION | `tasks.md` group 4 | The x offsets were attributed to a command that prints only `57 53` | The second command that produces them is quoted | `tasks.md` group 4 |
| SUGGESTION | `tasks.md` 1.3 | Gate configuration inside a behavior group reads as mixed kind | One clause saying why it cannot wait for group 6, and `noio-view.sh`'s now-stale header comment folded into the same edit | `tasks.md` 1.3 |
| SUGGESTION | `tasks.md` group 8 | `tests/degraded-coverage.toml`'s nine `covers` ranges into the three edited files rot silently — `validate_covers` checks only path, bounds, and one non-comment line | A task re-points them | `tasks.md` 8.6 |
| SUGGESTION | `tasks.md` 2.1 | Renaming either of the two tests `tests/degraded-coverage.toml:18` names as its proof fails `cargo test --test degraded_coverage` | Both named in the task | `tasks.md` 2.1 |
| WARNING | `design.md` → Decision 1 | The rationale rejecting a `const` table was factually wrong: `Style::new`, `fg`, `bg`, and `add_modifier` **are** `const fn` (`ratatui-core-0.1.2/src/style.rs:300`, `:335`, `:352`, `:408`), so no `LazyLock` would have been needed | The false sentence is replaced with the real reason — a struct needs one field per `(status, level)` pair and answers `Heading(255)` with a lookup miss | `design.md` → Decision 1 |
| SUGGESTION | `specs/view-palette/spec.md` | The confinement was written as "the only file in the crate — `tests/` included —", but the gate searches `src/` only, so the clause read as enforced when it was not | Scoped to `src/`, with the reason the wider claim is not made | `specs/view-palette/spec.md` → req 1 |
| SUGGESTION | `specs/view-palette/spec.md` | Two role pairs share a style (`FileMode`/`Code`, `AgentBadge(Unknown)`/`ListSeparator`) and the distinctness scenario excluded both partners, so it passed while the proposal's own `DIM`-overloading complaint quietly recurred | Both pairs are stated as decisions with their reasons, and the scenario now asserts them **equal** rather than stepping around them | `specs/view-palette/spec.md` |
| SUGGESTION | `specs/view-palette/spec.md`; `tasks.md` 1.1 | "Every `Role` variant" was hand-enumerated, so a role added later would escape all three table-driven tests | The list is built from an exhaustive `match`, which fails to compile until a new role is covered | `specs/view-palette/spec.md`; `tasks.md` 1.1 |
| SUGGESTION | `design.md`; `tasks.md` 8.2, 9.6 | Three cross-artifact drifts: "the RED of group 5" (it is group 4); both stale `AGENTS.md` sentences are in § Architecture rules; a bare `cargo llvm-cov` produces no JSON for `scripts/coverage-prod.py` | All three corrected; 9.6 names `make coverage` | `design.md` → Risks; `tasks.md` 8.2, 9.6 |

Reviewer D verified fourteen empirical claims: thirteen CONFIRMED, one FALSE (the plant line
number above). Reviewer C confirmed the sequential-group finding independently — groups 2, 3,
and 4 all edit `src/ui/view.rs`, no pair is wrongly parallel and none wrongly serialized — and
found all 48 spec scenarios owned by a task and every design Decision owned by one.

## No Remaining Implementation-Blocking Gaps

None remain. Every CRITICAL is repaired in the artifact that owns it, every WARNING is
repaired rather than accepted, and `openspec validate color-palette --strict` reports the
change valid. No unresolved decision requires user input.

## Deferred Non-Blocking Notes

- **`SPEC.md:757`'s degraded-states row still describes the badge as "dim" only.** That
  remains true after this change — the badge is dim *and* yellow — and the row's text is the
  `condition` key `tests/degraded-coverage.toml` binds it by, so rewording it means editing
  both sides for no new fact. Resolution point: `tasks.md` 8.0's CHECK, which now names the
  row and requires the decision to be recorded either way.
- **`openspec/specs/artifact-tabs/spec.md`'s `## Purpose` and the new
  `openspec/specs/view-palette/spec.md`'s missing one** are archive-time work, not apply
  work: the tracked-diff leg of `OPENSPEC-UNTOUCHED` fails any apply session that edits
  `openspec/specs/`. Resolution point: the note at the end of `tasks.md`, which
  `tests/spec_purposes.rs` will otherwise enforce loudly on the first `make check` after
  archiving.
