## Reviewed Artifacts

- `proposal.md`
- `design.md`
- `tasks.md`
- `specs/doc-conformance/spec.md`
- `specs/mouse-input/spec.md` (added during this review)
- `specs/binding-inventory/spec.md` (added during this review)

Reviewed by four independent `planning-reviewer` subagents, one slice each, none of which
wrote the package: (A) capability coverage, scenario quality, cross-artifact contradictions;
(B) design completeness, test boundaries, and the audit of whether each proposed check could
fail at all; (C) task alignment, lifecycle discipline, `parallel-after` independence;
(D) factual verification by running commands.

## Reviewed Against

- This repository HEAD: `6c875b6`
- Sibling repository HEAD: Not applicable — no sibling repository's contract is involved.
- Working tree: clean except the change directory itself. Reviewers B and D each worked in a
  scratch copy or restored the tree; `git diff --quiet SPEC.md` verified clean after both.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | design.md, specs/doc-conformance | The claim key `(gesture, zone, action-name)` is too coarse to distinguish the rows it must police. `action_name` collapses `Click(_)` to `"Click"`, so the row `mouse-text-selection` actually deleted claims an **observed** triple and is not vacuous — the check would have passed its own motivating regression. Measured independently by B and D: 104 triples, 18 active, both `Down(Left)\|DetailRow\|Click` and `…\|Select` present. | The outcome axis keeps the `Action` variant **plus its payload's constructor name**, discarding only geometry. A projection of the returned value, not a re-derivation. | design.md → Decision 2; specs/doc-conformance → "The claim", axis 4 |
| CRITICAL | design.md, specs/doc-conformance, tasks.md | One `Zone` per row leaves six active claims uncovered: the list wheel spans `List\|ListRow`, the detail wheel spans `Detail\|DetailTab\|DetailRow`. Agree-at-HEAD would be red. | A row names a **set** of zones and claims the cross product. | design.md → Decision 4; specs → "The documented set"; tasks.md 4.2 |
| CRITICAL | proposal.md, specs/doc-conformance, tasks.md | The wheel over an open overlay is a real binding (`src/ui/driver.rs:340-341`) documented only in a prose paragraph the extractor never reads, so two active claims are uncovered at HEAD — and the catch-all cannot absorb them, being `Ignore`-only. The proposal promised "no row is added or removed". | A row **is** added. The promise is withdrawn and the addition is a task. | proposal.md → What Changes and Impact; tasks.md 4.3; specs → agree-at-HEAD scenario |
| CRITICAL | proposal.md | `openspec/specs/mouse-input/spec.md:264-266` pins this test's subject **normatively** as "a **set equality**". The change makes that false, and `mouse-input` had no delta — two capabilities would have disagreed about one test in the archived tree. | `mouse-input` added to Modified Capabilities with a MODIFIED delta superseding that paragraph. | proposal.md → Modified Capabilities; specs/mouse-input/spec.md |
| CRITICAL | design.md, specs/doc-conformance | `mouse_action` is a function of the whole `Dashboard`. Its `Drag` arm branches on `selection.is_some()` and `sweep_dashboard` pins `selection: None`, so the clamp the table documents is unobservable and its row would be reported vacuous — a false failure. (Found by the session; confirmed independently by A.) | The sweep gains a pinned, counted **dashboard-fixture axis**, at minimum selection-absent and selection-present. | design.md → Decision 9; specs → "The dashboard-fixture axis"; tasks.md group 2 |
| CRITICAL | tasks.md | Group 2 made the row parser strict before group 3 added the tokens, so its own gate could not be green, and it broke `documented_mouse_actions_fails_on_a_missing_table`. | `documented_mouse_rows` is added **beside** `documented_mouse_actions` and wired in only after the document carries its tokens; the fold moves to the comparator group as its REFACTOR. | tasks.md 3.2, 5.6 |
| CRITICAL | design.md | "The information needed to catch all four is already computed and then thrown away" is false — only one of the four would have been caught, and two live in the catch-all's unparsed prose. | The claim is removed and replaced with the measurement; the catch-all limit is stated in three places. | design.md → Context and Decision 7; proposal.md → Non-Goals |
| WARNING | proposal.md, design.md, tasks.md | `swept_mouse_action_names` was said to have "two existing callers"; it has **three** direct ones, and one of the two named was indirect. The omitted `the_sweep_covers_the_mouse_under_both_overlay_states` asserts both name sets by equality and is the likeliest to break. | Caller list corrected and that test named in the projection's verification. | proposal.md → Impact; design.md → Contracts; tasks.md 1.3 |
| WARNING | tasks.md | The planting script's row carries no zone, so after the change it would exit 101 via the *missing-zone* rule, never reaching the vacuity leg — the headline control would fire for the wrong reason. | The plant now carries `Zone::DetailRow` and `Target::DetailLine`, **replaces** the real row, and the task asserts on the failure message, not only the exit status. | tasks.md 5.7 |
| WARNING | tasks.md | Task 4.4 named `the_claim_count_matches_every_document`, which does not exist. | Renamed to `documented_claim_count_matches_the_file` (`tests/doc_contract.rs:3288`). | tasks.md 5.5 |
| WARNING | tasks.md | The zone-count expectation of 12 is unreachable: the help-overlay row carries the overlay state, not a `Zone` token. | Corrected to **11**, with the arithmetic shown (14 rows, less the catch-all, less the overlay row). | tasks.md 4.4 |
| WARNING | tasks.md | Group 3's tests are all synthetic, so the gesture vocabulary could be satisfied while covering nothing the real table uses; the gap would surface two groups later. | The five real phrases are named in the task and the table pinned to length 5. | tasks.md 3.4 |
| WARNING | specs/binding-inventory | `binding-inventory:250-254` justifies `INVENTORY`'s binding as "one layer stronger" because the mouse table's check "parses a markdown table". After this change that contrast is false. | MODIFIED delta added, superseding the paragraph and restating what actually separates the two: subject, not strength. | specs/binding-inventory/spec.md |
| WARNING | tasks.md | Group 4 (now 5) had neither a REFACTOR task nor the statement that none was needed. | The deferred fold landed there as 5.6. | tasks.md 5.6 |
| WARNING | tasks.md | Two spec assertions had no owning task: the per-kind contribution bullet, and the requirement that a vacuous report name the near-miss claims. | Both given owning tasks; the unfalsifiable bullet was also replaced (below). | tasks.md 5.3; specs → agree-at-HEAD |
| WARNING | tasks.md | The no-`parallel-after` argument gave a cost judgment where a disqualifying criterion exists — groups do share mutable state, `SPEC.md`, which their gates read. | Reason replaced with the attributability criterion. | tasks.md, opening comment |
| NOTE | tasks.md | The `Zone`-variant command returned **24**, not 6 — it matched call sites throughout the file. The underlying fact was true. | Replaced with an `awk` anchored to the enum body, verified to return 6. | tasks.md → Planning-time evidence |
| NOTE | tasks.md | "95.49% total" is the **region** column; `--fail-under-lines` governs **lines**, 95.32%. Production 96.33% confirmed exactly against HEAD's on-disk export. | Both figures stated with the column named. | tasks.md 8.5 |
| NOTE | tasks.md | `CLAUDE.md` is a **symlink** to `AGENTS.md`, not a copy — the task named a file that does not independently exist. | Corrected; the edit is `AGENTS.md` alone. | tasks.md 7.2 |
| NOTE | tasks.md | The `SPEC.md` bullet the documentation group rewrites is **counted** by `spec_md_doc_conformance_claim_count`; splitting it into two bullets would turn `documented_claim_count_matches_the_file` red. | The one-bullet constraint is stated in the task. | tasks.md 7.1 |
| NOTE | specs/doc-conformance | "At most one" catch-all (spec) contradicted "Exactly one" (design), and no scenario covered two. | "Exactly one" in both, with a scenario asserting a second is an error. | specs → "The catch-all row"; Zone-less scenario |
| NOTE | specs/doc-conformance | A mistyped `Zone` token had no error path — it would surface as an unexplained vacuity. | Validation required, with its own scenario. | specs → "The documented set"; mistyped-token scenario |
| NOTE | specs/doc-conformance | The agree-at-HEAD bullet "every cell contributed a triple" is true by construction of a set insertion and cannot fail. | Replaced with computable assertions: all fourteen kinds and all six `Zone` variants appear, and the overlay axis is represented. | specs → agree-at-HEAD scenario |
| NOTE | design.md | Risks framed sweep-unreachability as hypothetical ("a future `Zone` variant") when an existing documented behaviour — the clamp — already sat in it. | Reframed as actual, with Decision 9 as the fix and the pinned exemption as the escape hatch. | design.md → Risks |
| NOTE | design.md, tasks.md | The new sweep's cost was unaddressed; review measured 88.5 s in debug for six passes, and the fixture axis doubles the cell work. | Recorded as a risk with its measurement, a caching requirement, and a stated lever with a threshold. | design.md → Risks; tasks.md 1.2, 2.3 |
| NOTE | tasks.md | Task 3.1's second sentence justified rather than instructed, and was wrong about which count group 4 depends on. | Dropped. | tasks.md 4.1 |

Verified clean and unchanged: the MODIFIED block for `doc-conformance` reproduces the entire
live requirement — reviewer A diffed it rather than reading it, finding exactly three
intentional hunks and no byte else different; every kind marker is valid and no group is mixed;
the evidence task precedes the change task in every group; all design decisions have owning
tasks; task lines instruct rather than justify; and `tasks.md` is smaller than the `design.md`
it implements.

## No Remaining Implementation-Blocking Gaps

None remain. Every CRITICAL and WARNING above is repaired in the artifact that owns it, and
`openspec validate bind-mouse-table-rows --strict` passes.

Two limitations are **specified rather than fixed**, deliberately and with their reasons
recorded, so neither is an unresolved decision:

1. The catch-all row's prose is not parsed, so two of the four false statements that motivated
   this change stay out of the check's reach.
2. Two rows may share one claim — at HEAD, the click on a change row and the second click on
   the row already selected, which differ only in a decision `Dashboard::apply` makes — and the
   vacuity direction cannot tell them apart. The pair is listed by name and pinned by count so
   the blindness cannot grow unnoticed.

## Deferred Non-Blocking Notes

- **Sweep wall time is measured during implementation, not now.** design.md → Risks names the
  threshold (roughly three minutes for the `doc_contract` binary) and the lever (the second
  fixture on the wide frame only); tasks.md 2.3 is the resolution point and requires the figure
  in the commit message.
- **`documented_mouse_actions`' lenient parse may have to survive** for
  `documented_mouse_actions_fails_on_a_missing_table`, whose fixture row is zone-less by design.
  tasks.md 5.6 resolves it either way and requires the reason to be stated if the fold is
  declined.
