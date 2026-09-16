# Planning review — consolidated findings

Four reviewers (slices A–D) against HEAD `f9a7cc7`, before any repair. Deduplicated:
several findings were reached independently by two or three slices, which is noted.

**Status: none repaired.** The package's premise was retired by the alternate-screen
measurement mid-review (see `measurements.md`), and the final shape depends on an
unmeasured cell. Each finding below is tagged for whether it survives that.

- **SURVIVES** — holds under any design that adds an `Action` variant.
- **CONTINGENT** — dies if the change ships no toggle key and no badge.

## CRITICAL

| # | Finding | Tag |
|---|---|---|
| 1 | `view-palette` is modified (`Role::MouseOff`) with **no delta spec**. Its spec reproduces the entire `Role` enum, pins a per-role modifier table, and asserts "exactly five groups" of shared coloured styles. Code side: `table()`, two exhaustive matches, and a "thirty-six rows" comment — none named by any task. (A, C) | CONTINGENT |
| 2 | `help-overlay` is modified **twice over** with no delta. (a) It pins `content_rows` = 42 and hardcodes `1-37/42`, `6-42/42`, `1-17/42`, `42 + 2 = 44`. (b) Its requirement "The overlay answers seven actions and every other one is inert" asserts `17 + 7 = 24`, and `apply_help_action` is exhaustive with no wildcard — so a new variant is a hard compile error until someone decides whether `m` acts or is inert. Nothing in the change decides it. (A, D) | SURVIVES |
| 3 | `binding-inventory`'s scenario "The sweep finds the twenty-two bound actions and exactly two exemptions" is falsified and not in the delta. Its header carries a count, so it needs REMOVED+ADDED, not MODIFIED. (A) | SURVIVES |
| 4 | The 60-column footer scenario is arithmetically impossible. `fit_hints` **breaks** at the first hint that does not fit rather than skipping it, so the badge drops `a/c/s launch` as well as `g focus` — and `g focus` is *already* dropped at 60 columns at HEAD, so the stated cost names the wrong hint. False on all three bullets. (A, B, D) | CONTINGENT |
| 5 | Where a refused capture's reason is stored is **unspecified**, and all four existing problem lists are replaced wholesale (`launch.problems`, `refresh.problems`, `changes.problems`) or written exactly once (`refresh.startup`). Needs a new field → `Dashboard` becomes **seventeen**, not sixteen, falsifying the delta, the destructure companion, and task 2.3. `change-rows` then needs a delta for the fourth source. Name it `capture_refusal`, **not** `mouse_problem`: `scripts/gates/wired.sh` leg 5c greps that literal and would fire with a message naming the wrong subject. (A, B, C) | CONTINGENT |
| 6 | `run_loop` lives at `src/ui/driver.rs:101`, not `src/ui/mod.rs`. design.md says driver.rs "is **not** touched" — false. The `ArtifactReader` alias it claims to copy lives at `src/ui/app.rs:17` and is named by two files, so task 3.3's "must print nothing" grep fails on correct code. (B) | SURVIVES |
| 7 | The help overlay swallows the toggle. `apply` dispatches to `apply_help_action` first; with the seam call in the loop and the flag move in `apply`, the terminal stops reporting while the pane says otherwise — the exact divergence Decision 8 forbids. **Exhaustiveness pushes toward the bug**: the lowest-friction arm appends `ToggleMouse` to the ignore list beside `Refresh`, which is the arm that produces it. Fix by putting the seam call and the flag move at one site. (B) | CONTINGENT |
| 8 | **The plan deadlocks at its first fan-out.** Adding `Action::ToggleMouse` breaks `tests/doc_contract.rs`'s exhaustive `action_name` match, so group 2 cannot compile, so its gate can never go green — and groups 4 and 5 dispatch only after it does. (C) | SURVIVES |
| 9 | Group 5's own VERIFY is unsatisfiable: `m` enters `INVENTORY` in 5.2 but neither document until group 6, and `compare_key_atoms` tolerates no residual. (C) | SURVIVES |
| 10 | `parallel-after: 2` fails the schema's third criterion — a shared working tree with a whole-tree gate means each group's intermediate state reds the other. File-disjointness itself checks out; attributability is what fails. (C) | SURVIVES |
| 11 | The spec scenario "A released capture withdraws the gestures and nothing else" has no task at all, though it is in the design matrix. (C) | CONTINGENT |
| 12 | Group 7 is misclassified `behavior` and its RED cannot be red: task 6.1 rewrites the `SPEC.md` paragraph three tasks before group 7 asserts it is still wrong. The schema forbids inventing a RED that cannot honestly fail. (C) | SURVIVES |
| 13 | The row-count site inventory is wrong. `grep -n '\b42\b' src/ui/help.rs` prints **eleven** lines, not the four claimed — all eleven genuine — **plus seven more the grep cannot see**, including `assert_eq!(band.height, 44)` (fails the build) and two clamp *values* (`5` → 6, `25` → 26). The rest rot silently as stale prose. `src/ui/layout.rs:989`'s `CONTENT_ROWS = 42` must **not** move — it is deliberately decoupled. (D) | SURVIVES |

## WARNING

| # | Finding | Tag |
|---|---|---|
| 14 | Task 1.3's check can never pass: it forbids `disable_mouse` diff lines in the group whose job is adding a method that calls it. Scope the diff to `restore_then`'s body. (B, D) | CONTINGENT |
| 15 | Three further pinned figures move, named nowhere: `doc_contract`'s `union.len() == 24`, `bound.len() == 22`, and the test's own name. (D) | SURVIVES |
| 16 | A twelfth `doc_contract` claim is added, but `AGENTS.md`'s "eleven further claims" and `SPEC.md` § Doc-conformance checks are not in the task list — and neither is machine-bound, so nothing fails when they drift. (D) | SURVIVES |
| 17 | `proposal.md` puts the badge in the list region's heading row while design, specs and tasks all say footer; its Impact file list names `src/ui/list.rs` and omits `view.rs` and `palette.rs`. (A) | CONTINGENT |
| 18 | The existing footer requirement says hints start "at column 0" and is not MODIFIED, so after merge two requirements disagree whenever capture is released. (A) | CONTINGENT |
| 19 | The badge's behaviour is unspecified in the footer's two filter forms — one replaces the hints entirely, the other puts `/{query}` at hint 0, and both collide with "first". Reachable: `m` types while filtering, so capture can be released and then `/` pressed. (A, B) | CONTINGENT |
| 20 | `mouse_capture` initialises `true` even when start-up capture was **refused**, so the pane would claim a state the terminal never granted. (A) | CONTINGENT |
| 21 | The badge is not the conditional-hint pattern the Boundaries table claims: a hint inside `fit_hints` is dropped whole at 12 columns, but the spec requires it never dropped and truncated by the frame. It must be drawn ahead of `fit_hints` against `width - 15`, which is a `render_footer` restructure no task names. (B) | CONTINGENT |
| 22 | Decision 9 names no **colour** for `Role::MouseOff`. Bare `DIM` is `Style`-equal to `RegionHeading`, `RegionRule`, `Quoted` and `Muted`, so the "distinguishable from `file mode`" scenario passes while the badge is indistinguishable from four other things. (C) | CONTINGENT |

## The shared root cause

Nine of these thirteen CRITICALs are the same mistake: **a count or a file location asserted
from reading rather than from running a command.** The repository's own task instructions say
"every number in the plan comes from a command"; the plan's header comment cites a count that
was never run. Any rewrite must derive every figure — binding counts, row counts, field
counts, action counts, footer column budgets, and the set of files a literal appears in — by
executing the search, and record the command beside the number.
