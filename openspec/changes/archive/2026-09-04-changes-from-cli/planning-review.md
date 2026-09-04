## Reviewed Artifacts

- `openspec/changes/changes-from-cli/proposal.md`
- `openspec/changes/changes-from-cli/design.md`
- `openspec/changes/changes-from-cli/tasks.md`
- `openspec/changes/changes-from-cli/specs/cli-changes/spec.md` (new capability)
- `openspec/changes/changes-from-cli/specs/change-merge/spec.md` (new capability)
- `openspec/changes/changes-from-cli/specs/schema-cli-fallback/spec.md` (new capability)
- `openspec/changes/changes-from-cli/specs/change-model/spec.md` (delta, added during this review)
- `openspec/changes/changes-from-cli/specs/plugin-build/spec.md` (delta)

The finding pass was delegated to **three independent reviewers**, none of which wrote the
plan and none of which was a fork of the planning session, each given one slice of the
review list and required to write findings to a scratchpad file incrementally rather than
only in a final message. Slice A: capability coverage, scenario quality and coverage, PRD
non-goals, contradictions. Slice B: design completeness, test boundaries, verification
matrix, coverage floor. Slice C: the verification apparatus — every command interrogated
with "what would make this go red?", run against scratch copies, with deliberate attempts
to defeat each check. Combined: **9 CRITICAL, 15 WARNING, 9 SUGGESTION**.

## Reviewed Against

- This repository HEAD: `99877d1` (`docs(changes-from-cli): draft proposal, specs, design,
  and tasks`), on `main`. Baseline before the change: `c4f88df`.
- Sibling repository HEAD: `Not applicable` — no sibling repository carries a contract this
  change touches. The external contract it *does* depend on is the installed
  `@fission-ai/openspec@1.11.0` CLI, read at
  `/Users/…/.nvm/versions/node/v24.20.0/lib/node_modules/@fission-ai/openspec/dist/` and
  independently re-verified by two of the three reviewers.
- Working tree: clean apart from this change's own planning files, which were intentionally
  included in the review.

## Gaps Found and Fixed

### Blocking — the verification apparatus

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | tasks.md | `DEPS` was a fenced block of ten **comment lines**. Extracted as task 1.1 prescribes, `sh DEPS.sh` exits 0 unconditionally — while design.md named it as the verification for five `plugin-build` scenarios. | Written out as a real five-leg check: one bin target plus a delete-then-`scripts/build.sh` run judged on exit status; resolved-metadata dependency assertions plus the `features = []` spelling read from manifest text and `cargo build --locked`; `cargo tree -e normal` over four triples against an enumerated set; MSRV against `Cargo.toml`'s own `rust-version`; and the removal experiment in copies with an undeclared-crate guard. Legs 1–4 verified green in a scratch copy; leg 2a and leg 5/`serde_json` verified **red** against today's tree. | tasks.md → Command-level checks, `DEPS`; tasks 1.3, 1.5, 10.8 |
| CRITICAL | tasks.md | `GATE-COMPILE` defined `gate_compile()` and **never called it**; the three mutators were never written; `$WORK` was undefined (an unset `WORK` would have made `rm -rf "$WORK/…"` catastrophic); the helper only `return 1`d. The check design.md called "the load-bearing one" reduced to "the crate builds". | Replaced by `GATE-MECH2`, fully executable and run end to end at planning time: a green control plus three variants asserting codes **present and absent**. `WORK` is guarded with `:?`. | tasks.md → `GATE-MECH2`; task 10.4 |
| CRITICAL | tasks.md, design.md | Test-name/filter mismatch. `src/changes.rs`'s `mod tests` is flat, so libtest matches on the function name; `testcount 'parse_list'`, `'parse_apply'`, `'cli_artifacts'`, `'join_artifacts'`, `'schema_fallback'` each matched **zero** of the names their own groups prescribed, and 55 of 58 matrix `Command` cells named tests no task creates. `cargo test` exits 0 on a filter matching nothing. | Adopted a test-module convention (`mod list_json`, `apply_json`, `which_json`, `cli_artifacts`, `join_artifacts`, `schema_fallback`, `from_cli`, `merge`), stated once in both files, and rewrote every `testcount` gate and every matrix `Command` to address tests through `<module>::<exact fn name>`. Cross-checked mechanically: all 69 scenarios appear in the matrix, no orphan rows, every matrix test name appears in tasks.md, every `testcount` filter names a real module. | tasks.md → Test-module convention; design.md → Test Strategy and the whole matrix |
| CRITICAL | tasks.md | `GATE-REST` was **green on two compiling, formatter-clean functional updates** — a `//` comment and a `/* */` comment between the comma and the `..` — because it stripped only whole-line comments. Demonstrated by reviewer C. | Replaced by `GATE-MECH1.py`, which strips comments with a real state machine (string and character literals left intact, byte offsets preserved so line numbers stay exact). Both forms now exit 1; verified at planning time. | tasks.md → `GATE-MECH1.py`; task 10.3 controls (d) and (e) |
| CRITICAL | tasks.md | `GATE-DEFAULT` was **green on `impl std::default::Default for Change`** (the ERE required a bare `Default` after `impl`), on a multi-line `#[derive(…)]`, and on an `impl Default for crate::changes::Origin` in any file other than `src/changes.rs`, which it never searched. | Folded into `GATE-MECH1.py` half A: matches a path-qualified `Default`, walks the attribute block above each declaration so a multi-line derive is caught, and searches **every** `.rs` file under `src/`. All three forms now exit 1; verified. | tasks.md → `GATE-MECH1.py`; task 10.3 controls (f)–(h) |
| CRITICAL | tasks.md | `TESTCOUNT` at task 13.6 summed **every** test binary while the task-1.1 baseline was `--lib`. The suite is already 316 (300 lib + 11 `ci_workflow` + 5 binary-integration), so the gate passed with zero new lib tests and stayed green even if the lib count *fell*. | Added a required `scope` parameter (`--lib` \| `--all-targets`) that rejects any other value, and scoped every gate `--lib`. | tasks.md → `TESTCOUNT`; every group VERIFY; task 13.6 |
| CRITICAL | tasks.md | Mechanism 2 of the two-producer gate could be removed with **every runnable check still green**, because `GATE-COMPILE` was inert. Reviewer C demonstrated two removals (`assert_invariants` rewritten to field access; a `..` added with a trailing comment) that left `GATE-DEFAULT`, `GATE-REST`, and `cargo fmt` all green. | Fixing `GATE-MECH2` closes it: variant (a) requires `E0027`, so an `assert_invariants` that no longer destructures — or that was deleted — fails the check. Variants (b) and (c) additionally prove each mechanism catches what the other misses. | tasks.md → `GATE-MECH2`; task 10.4 |
| CRITICAL | tasks.md, design.md | `OPENSPEC-UNTOUCHED` filtered only `^openspec/changes/changes-from-cli/`, so it would go **red on this change's own task 12.6**, which rewrites the tracked `openspec/IMPLEMENTATION-ORDER.md`. It passed only because task 10.7 ran before group 12 — an ordering nothing stated was load-bearing. | Excluded exactly two paths, **by name**: this change's artifact directory and `openspec/IMPLEMENTATION-ORDER.md`, both hand-edited planning documents rather than code-path writes. A stray file anywhere else still fails. Added task 12.9 re-running the check after the documentation group. | tasks.md → `OPENSPEC-UNTOUCHED`; tasks 10.7, 12.9 |
| CRITICAL | design.md | `GATE-MECH2`'s first draft reported a **false failure**: `tasks::Progress` implements no `Default`, so `#[derive(Default)]` on `Change` raised `E0277`, which masked the `E0027` under test. A check that fails for the wrong reason is not evidence. | `defeat_mech1.py` supplies that `Default` itself and finds construction sites from the compiler's own `E0063` spans (so it cannot miss a literal, and does not touch `-> Change {`, which is a function body). The incidental brake is now explicitly **not relied on**. | design.md → Decision 8; tasks.md → `defeat_mech1.py` |

### Blocking — specification and design gaps

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| WARNING | specs/cli-changes | "SHALL run **exactly two kinds** of invocation" contradicted `schema-cli-fallback`, which requires the same function to run `["schema","which",…]`. | Reworded to "two kinds **of its own**", with the fallback's invocation named as the only other permitted vector. | cli-changes → "Two commands produce the CLI's view…" |
| WARNING | specs/cli-changes | The delta pinned five of `Change`'s seven fields and never said where the CLI producer's **`dir`** and **`origin`** come from; the contract lived only in design.md. | Added a requirement with a field-source table, plus the boundary validation the gate needs (see the next row). | cli-changes → "Every one of `Change`'s seven fields comes from CLI data" |
| WARNING | specs/cli-changes | Three degenerate payloads would make `conformance::assert_invariants` **panic** rather than degrade: an empty `name` (the parser required only "a string"), a `changeDir` whose final component is not the change's name, and a name repeated in the `changes` array (which `merge` pairs on). | `name` must be non-empty; a repeated name keeps the first and records a problem; a mismatched `changeDir` is a per-change failure. Three scenarios and three RED tests added. | cli-changes → the list-envelope and seven-fields requirements; tasks 2.1, 8.1 |
| WARNING | specs/schema-cli-fallback | `schema::load`/`load_dir` return `ParsedSchema { schema, problems }`, and nothing said what the CLI side does with the problems. On a repository-unvendored schema the file producer never read the file, so they would be lost outright. | Required them recorded on **every** change using the cached schema, not only the one that populated the cache. Scenario and RED test added; recorded as design Decision 12. | schema-cli-fallback → "A not-vendored schema is repaired…"; design Decision 12; task 7.1 |
| WARNING | specs/change-merge | The headline scenario gave **both** producers the schema `outside-in-tdd` and never asserted the merged schema, so an implementation keeping the file's schema passed. The change's central claim was unverified. | The two producers are now given different names (`stale-name` and `tdd`), and the scenario asserts the merged schema is `tdd` and that `stale-name` appears nowhere. | change-merge → "The CLI's schema, progress, and artifacts replace the file's"; task 9.1 |
| WARNING | specs/cli-changes | The failure bullet for `list --json` **exiting non-zero** had no scenario and no task — and it is the route by which "no `openspec/` directory" reaches this module, a boundary `config.yaml` → `rules.specs` names explicitly. | Scenario and RED test added. | cli-changes → failure requirement; task 8.1 |
| WARNING | specs/schema-cli-fallback | Two of five fallback-failure bullets had no scenario: `schema which` failing to start, and the CLI-named directory's `schema.yaml` being unreadable or invalid. | Two scenarios and their RED tests added. | schema-cli-fallback → failure requirement; task 7.1 |
| WARNING | specs/change-merge | The capability restated and **widened** `change-model`'s landed gate from inside a different capability — adding `ChangeSet`/`Origin`, forbidding `..` file-wide, and treating `merge` as a producer, which `change-model`'s wording does not. Two owners with different scope is the drift the gate exists to prevent. | Written as a **MODIFIED delta on `change-model`** instead. `change-merge` now carries one short requirement pointing at it and adds no second copy. | new `specs/change-model/spec.md`; change-merge → "The merge is a third construction site…"; proposal Capabilities |
| WARNING | proposal.md, design.md | Non-Goals said "No writing anywhere under `openspec/`" while Impact and tasks edit `openspec/IMPLEMENTATION-ORDER.md` — a self-contradiction, and the reason the untouched-check was mis-scoped. | Narrowed to "no **runtime** write", naming the two hand-edited planning documents. | proposal Non-Goals; design Goals/Non-Goals |
| WARNING | proposal.md | Impact omitted `AGENTS.md`, which design.md's Boundaries table and tasks 12.7/12.8 both edit. | Added, alongside the `change-model` delta. | proposal Impact and Capabilities |
| WARNING | design.md | `plugin-build`'s "Exactly one binary target is produced at the release path" scenario was scheduled by **no task**: nothing ran `scripts/build.sh` and nothing asserted the single bin target. | `DEPS` leg 1 now does both, with the deletion and the exit status both load-bearing. | tasks.md → `DEPS` leg 1; task 1.5 |
| WARNING | design.md | Task 13.8 runs `openspec validate --strict`, spawning the real `openspec`, while the Test Boundaries table read "not reached" and task 10.6 proves the suite passes with it unresolvable — the two statements read as a contradiction. | Added an explicit boundary row for the final validation task, stating it is real, deliberately outside every test tier, and reached by no test. | design → Test Boundaries; task 13.8 |
| WARNING | tasks.md | Groups 7 and 9 had no REFACTOR task and their VERIFY did not state that none was needed, breaking the schema's RED → GREEN → REFACTOR lifecycle. | REFACTOR tasks added to both (7.3, 9.3), each naming a concrete candidate or requiring an explicit "none was needed". | tasks 7.3, 9.3 |
| WARNING | tasks.md | Group 12 (Documentation) had no CHECK bookend and no VERIFY, so an operational group ran without the CHECK → CHANGE → VERIFY lifecycle. | Added 12.1 (re-read each section before editing, red when another change already corrected it) and 12.9 (re-run `OPENSPEC-UNTOUCHED` after the roadmap edit). | tasks 12.1, 12.9 |
| WARNING | tasks.md | `$WORK` was used unguarded in two checks; unset, `rm -rf "$WORK/…"` becomes `rm -rf "/…"`. | Both `GATE-MECH2` and `DEPS` now open with `: "${WORK:?…}"` and a directory test. | tasks.md → both blocks; task 1.1 |

### Non-blocking, accepted as written

| Severity | Source Artifact | Problem | Resolution |
|---|---|---|---|
| SUGGESTION | specs/cli-changes | The "exactly three invocations" scenario did not say the schemas were vendored, so a `schema which` call could legitimately be due and the count would be four. | Fixed: the scenario now states the schema is vendored in the scratch repository. |
| SUGGESTION | specs/cli-changes | The byte-order requirement partly duplicates a landed `change-enumeration` requirement whose text already names `changes-from-cli`. | Accepted. The requirement cross-references `change-enumeration` explicitly and adds only what is new — the `--sort name` prohibition, which is this change's own finding. |
| SUGGESTION | specs/change-artifacts | Its progress wording reads oddly now that the merged value comes from `list --json`. | Accepted as non-blocking: `cli-changes` cites `task-progress.js` and `from_files`' rule directly, so no reader is misled. Revisit if `degraded-states` finds it confusing. |
| SUGGESTION | design.md | `join_artifacts` rule 3 (both lists empty) is subsumed by rule 6. | Accepted: kept as an explicit rule and an explicit scenario. Redundant but total, and the reader does not have to derive the empty case. |
| SUGGESTION | design.md | `src/changes.rs` will pass 3000 lines. | Accepted and stated in Non-Goals: a directory-module split is behaviour-free and belongs in its own change. |

## Corrections to durable documents this change makes

Nine statements — eight in `SPEC.md`, one in the roadmap — were established as wrong or
incomplete by reading the installed CLI's own compiled source and reproducing each against
throwaway fixtures. Each is corrected by a task in group 12; the before/after is recorded
here so the corrections are reviewable without reading the diff.

| # | Document | Before | After | Evidence |
|---|---|---|---|---|
| 1 | `SPEC.md` → Data layer → Changes | "`openspec list --json` yields `{name, completedTasks, totalTasks, lastModified, status}` per change" — reads as a bare array of those objects | The output is the envelope `{"changes": [...], "root": {"path", "source"}}`; the five fields are per element of `changes` | `dist/core/list.js:119-125`; `dist/cli/index.js:321` |
| 2 | `SPEC.md` → Data layer → Changes | "the CLI path re-sorts its own result by name to match" — silent on the CLI's own `--sort name` | `--sort name` exists but sorts with `a.name.localeCompare(b.name)`, a locale collation, whereas `from_files` sorts Rust `String`s in byte order; the flag therefore **cannot** be used and the re-sort is ours | `dist/core/list.js:111-116` |
| 3 | `SPEC.md` → Data layer → Dual-source model | silent on where the CLI-side counts come from | The counts come from `list --json`'s `completedTasks`/`totalTasks`, **not** from `instructions apply --json`'s `progress`: `list` glob-expands the tracked artifact and sums every match with a `tasks.md` fallback, while apply resolves `apply.tracks` as one path with no globbing. Reproduced: `tracks: multi/**/*.md` with three task files gives `list` `1/3` and apply `0/0` | `dist/utils/task-progress.js:120-133` vs `dist/commands/workflow/instructions.js:278-327` |
| 4 | `openspec/IMPLEMENTATION-ORDER.md` → Phase 3 row | parse `openspec list --json`, `openspec status --change <n> --json`, **and** `openspec instructions apply --change <n> --json` | `status --change` is dropped: it supplies nothing `instructions apply` does not (both compute paths through the same `resolveArtifactOutputs`), and it doubles the per-change Node start the PRD names as a risk | `instruction-loader.js:224` and `instructions.js:273` are the same function |
| 5 | `SPEC.md` → Data layer → Artifact files (new) | silent | `status --json`'s `artifacts` array is in **topological build order**, not the schema's declared order, so it is not a source for a positional join even if it were run. Its `artifactPaths` keys *are* declared order, but reading JSON object key order is a contract no parser guarantees | `instruction-loader.js:258-261`; `graph.js:81-118` |
| 6 | `SPEC.md` → Degraded states (new row) | silent | A schema declaring the same artifact id twice makes the CLI reject the schema **outright**, so such a change is permanently file-mode. This crate's `schema::parse` accepts duplicates, as `schema-artifacts` requires — the plugin's "usable" is strictly wider than the CLI's | `dist/core/schema.js:29-30, 40-48` (`Duplicate artifact ID`) |
| 7 | `SPEC.md` → Degraded states (new row) | silent | A CLI reporting a repository root other than the resolved one discards the whole CLI result. This is reachable: `resolve::find_repo` walks up from the workspace working directory while the CLI walks up from the **process** working directory, and `subprocess-seam` forbids setting `current_dir` | `SPEC.md` → Resolution chain; `subprocess-seam` → "The real implementations spawn and return stdout and do nothing else" |
| 8 | `SPEC.md` → Degraded states (new row) | silent | When a CLI command fails, its **reason is unavailable**: it writes `{"status":[{severity,code,message}]}` (or `{"error","available"}`) to **stdout** and exits 1 with an **empty stderr**, and `CliError::Failed` carries stderr only. Every recorded problem therefore names the change, the vector, and the exit code, and claims no reason | `dist/cli/index.js:40-46, 624-627`; `dist/commands/schema.js:450-464`; `subprocess-seam` spec |
| 9 | `SPEC.md` → Resolution chain → Schema | "ask `openspec schema which <name> --json` … where `path` is the schema's *directory*" — correct, but silent on `source` and on which root the CLI uses | Confirmed correct, and extended: `source` ∈ `{project, user, package}`, and the CLI resolves `schema which`'s project tier from `process.cwd()`, not from the resolved OpenSpec root. Harmless here because the root guard has already discarded the result when the cwd is a different repository | `dist/commands/schema.js:16-38, 400`; `dist/core/schema/resolver.js:125-153` |

`AGENTS.md` is additionally rewritten in two places (tasks 12.7, 12.8): its repo-state
paragraph, and the two architecture-rule bullets that gain the "the seam parses nothing"
and "counts come from `list --json`" clauses. Both are edits in place, not additions.

## No Remaining Implementation-Blocking Gaps

None remain. Every CRITICAL and every WARNING above is repaired in the artifact that owns
it; the five SUGGESTIONs are accepted with a stated reason. `openspec validate
changes-from-cli --strict` reports the change valid.

Three mechanical cross-checks were run after the repairs and all pass:

- All **69** spec scenarios appear in design.md's verification matrix (77 rows — eight
  scenarios take two rows, proved once at the parser and once at the composition); no row
  names a scenario that does not exist.
- Every test name in the matrix's `Command` column appears in `tasks.md`, and every
  `testcount` filter names a module `tasks.md` creates.
- Every one of the eight command-level checks was **executed at planning time**, green on
  the tree where it should be green and red on every negative control: `NOSPAWN-GREP`
  (green, plus four controls red in an earlier change); `GATE-MECH1` (green on the real
  tree — including its `segment[..star]` slice index — and red on all ten controls);
  `GATE-MECH2` (green control builds; variant (a) `E0027`+`E0063`, variant (b) `E0027` with
  `E0063` gone, variant (c) `E0063` with `E0027` gone); `NOJSON-SEAM` (correctly red today);
  `DEPS` legs 1–4 green in a scratch copy with the dependency, legs 2a and 5/`serde_json`
  correctly red without it; `TESTCOUNT`, `NOSPAWN-RUN`, and `OPENSPEC-UNTOUCHED` carried
  forward from `subprocess-seam` with the scope and exclusion fixes above.

## Deferred Non-Blocking Notes

- **Splitting `src/changes.rs` into a directory module.** It passes 3000 lines with this
  change. The split carries no behaviour and is recorded in design.md → Non-Goals; it
  belongs in its own change.
- **Carrying stdout on `CliError::Failed`.** It would let the plugin surface the CLI's own
  diagnostic — "Unknown schema 'outside-in-tdd'" — instead of an exit code. It was
  considered and rejected here: it reopens a capability that landed this session and
  invalidates one of its scenarios, for text no view yet renders. Resolution point:
  design.md → Decision 6, to be revisited by `degraded-states`, which owns the audit of
  what each degraded row actually shows.
- **Batching the per-change CLI calls.** One `list` plus one `apply` per change is N+1, and
  the CLI offers no per-repository `contextFiles` dump. The mitigation is running the whole
  thing off the render path, which is `live-refresh`'s work. Resolution point: design.md →
  Risks / Trade-offs.
