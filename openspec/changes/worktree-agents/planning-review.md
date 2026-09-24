## Reviewed Artifacts

- `proposal.md`
- `design.md`
- `tasks.md`
- `specs/agent-attribution/spec.md`
- `specs/agent-launch/spec.md`

## Method

Four `planning-reviewer` subagents were dispatched simultaneously. None was a fork of the
writing session. Each got the change directory and one slice:

- **A** — capability coverage, delta fidelity, and contradictions;
- **B** — design completeness, test boundaries, and whether each check could fail;
- **C** — task alignment and lifecycle;
- **D** — factual verification.

Each also read `worktree-changes` for context. Reviewers reported findings only and edited
nothing. Two of them ran their own measurements. A built a nested `.worktrees/` layout with real
git. D confirmed on a nested scratch tree that `openspec` resolves the innermost `openspec/`
from its working directory, and that Herdr 0.9.0's `agent list` is still session-global.

**1 CRITICAL, 13 WARNING, about 15 SUGGESTION.** The CRITICAL was raised by A and B and flagged
by C from the launch side. It is the same nested-worktree defect `worktree-changes` repaired.
Every CRITICAL and WARNING is repaired below.

## Reviewed Against

- This repository HEAD: `2363e09`. Its `src/` and `tests/` are identical to `0d44237`.
  `worktree-changes` is planned but not implemented at this HEAD.
- Sibling repository HEAD: Not applicable.
- Working tree: clean apart from the repairs recorded here and `worktree-changes`' own planning
  repairs.
- MODIFIED deltas diffed against the live spec at HEAD: **4 of 4**. The repair carried one
  more, `agent-launch` → *The launch decision is a pure, total function…*. All four were
  re-extracted by script after the repairs and diffed:

  | Block | Live lines | Hunks |
  |---|---|---|
  | attribution purity | 130 | 4 |
  | attribution scope | 104 | 3 |
  | launch decision | 188 | 4 |
  | three calls | 93 | 4 |

  No carried scenario is dropped, and every removed line is an intended edit.

  The launch-decision block's `Request` quote also gains `Resolve`. That repairs pre-existing
  drift, because `settings-window` added the variant without re-carrying this block.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | agent-launch ADDED, design D4 | "The first worktree root the dir lies under" chose the main checkout for every row of a pane opened in a nested worktree *(A, B, C)* | The root comes from `worktrees::member_of` (`worktree-changes`), which matches `<root>/openspec/changes`; nested-layout scenario added | agent-launch; design D4 |
| WARNING | agent-launch | After archive, the capability would quote `Request` three ways; rule 7 and a scenario literal lacked `root` *(A)* | The decision requirement is carried with `root` in the quote, in rule 7, and in the per-intent scenario; the ADDED block no longer overrides it from outside | agent-launch MODIFIED |
| WARNING | design → Contracts | `dashboard-loop` ×2 and `live-updates` ×1 quote three-field `Request::Launch` literals *(A)* | Recorded why they are not carried: every one is a base-row launch, so it means `root: None`, and carrying them is 971 lines for three literals | design → Contracts |
| WARNING | tasks group 4 | The "`cwd` absent or outside the root" row goes stale: its condition, and a `covers` range the new parameter shifts *(A, C, D)* | 4.2 rewrites its condition in `SPEC.md` and the toml together; 4.4 re-measures both rows' `covers` | tasks 4.2, 4.4; proposal |
| WARNING | agent-attribution | The third agent in the member scenario could not show a leak (`Working` ties keep the first pane), and could not catch a textual prefix test *(A, B)* | Made `Blocked` at `/w/feat-other` | agent-attribution |
| WARNING | agent-attribution | *A member's OpenSpec root bounds its scope* and the dashboard scenario omitted inputs their outcomes depend on *(A, B)* | `repo`, names, and mapping stated; badges asserted | agent-attribution |
| WARNING | agent-attribution, tasks 1.1 | The dashboard scenario asked for a footer at a tier (`ui::list`) that has none, and its footer clause could not fail *(B, C)* | Badge asserted through `rows()` at 38/58; new view scenario *The footer counts an unplaced agent in a member worktree* | agent-attribution; design matrix; tasks 1.1 |
| WARNING | design → Test Boundaries, tasks 2.1 | Named a scratch-`herdr` launch-worker harness that does not exist; the landed tests use `FakeCli` through `run_request`/`handle` with a `ScratchDir` state dir *(B, D)* | Rows corrected; the test goes through `handle`, and `split_ok` gains a root-parameterised variant | design; tasks 2.1 |
| WARNING | agent-launch, tasks 2 | The spec's `..` notation, and the obvious Go-branch fill, fail `NODEFAULT-UI`'s `Launch` leg (floor = current 99 spans) *(C, D)* | Every literal and pattern names all four fields | agent-launch; design → Boundaries; tasks 2.1–2.2 |
| WARNING | design D4 | "One place decides the checkout" was untrue: three sites *(C, D)* | `member_of` is that place | design D4 |
| WARNING | design, tasks | The pane-map scenario was called "extended" in the design and "carried" in the tasks *(C)* | Carried unchanged | design matrix |
| WARNING | tasks group 4 | Its CHECK was itself a change, could not fail, and 4.2–4.4 lacked CHANGE labels *(C)* | 4.1 records non-zero stale-site greps; 4.2–4.4 are CHANGE; 4.5 VERIFY | tasks group 4 |
| WARNING | tasks 1.4, 2.4 | `agents`/`launch` filters select 63/103, not 58/74 *(C, D)* | Commands use `agents::` / `launch::` | tasks; design |
| WARNING | tasks 4.2 | `SPEC.md`'s `agents::attribute` tested-modules bullet was not named *(C)* | Named | tasks 4.2 |
| SUGGESTION | agent-launch | `g` onto a worktree agent was not shown, though the Why cites it *(B)* | Added to the refusal scenario | agent-launch |
| SUGGESTION | design | Three new tests pass against an empty implementation *(B)* | Labelled regression guards; the RED evidence is the discriminating tests against a stub | design → Test Strategy; tasks 1.1 |
| SUGGESTION | design D1 | A launched agent named like its change is placed by the name tier, since `state::record` records nothing then *(D)* | Restated | design D1 |
| SUGGESTION | agent-attribution | "Only a worktree the family lists is admitted" overstated it when the main checkout is a member *(A)* | Reworded to "only a root the family lists widens the scope" | agent-attribution |
| SUGGESTION | tasks 0 | 0.2's reason was false, since `worktree-changes` touches neither capability; 0.1 did not check the fixture or `member_of` *(C)* | Both fixed | tasks 0.1–0.2 |
| SUGGESTION | proposal, tasks | Rows cited by 0-based index and by text `worktree-changes` will already have rewritten *(C, D)* | Cited by condition text; "whatever `worktree-changes` left there" | proposal; tasks 4.2 |
| SUGGESTION | tasks | The roadmap-row task wrote inside `openspec/` outside the change *(A)* | Moved to the archive step with the two Purpose/parenthetical hand edits | design → Persistence and Rollout (archive) |

## No Remaining Implementation-Blocking Gaps

None remain. `openspec validate worktree-agents --strict` passes. A script confirms that every
one of the 32 spec scenarios has an exact row in design.md's verification matrix.

## Deferred Non-Blocking Notes

- **What Herdr does with a `--cwd` that no longer exists is unmeasured.** A worktree can be
  removed within the up-to-two-second window before the family catches up, and then `a` on its
  row would send that path. Measuring it means splitting a pane in the user's live session, so
  task 2.4 does it in a throwaway workspace with the user's go-ahead.
- **Three edits inside `openspec/` belong to the archive step**, because a delta cannot make
  them (design → Persistence and Rollout):
  - `agent-attribution`'s Purpose;
  - `agent-poller`'s out-of-scope parenthetical;
  - the roadmap row.
