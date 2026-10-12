## Reviewed Artifacts

- proposal.md
- design.md
- tasks.md
- specs/dashboard-restore/spec.md
- specs/pane-open/spec.md
- specs/plugin-manifest/spec.md
- notes/measurements.md

Four independent `planning-reviewer` agents, one slice each: (A) capability coverage,
scenario quality, contradictions; (B) design, test boundaries, falsifiability of checks;
(C) task alignment and lifecycle; (D) factual verification against the repository, Herdr
0.9.1, and Herdr's changelog and docs. None of them wrote the plan, and none edited anything.

## Reviewed Against

- This repository HEAD: `e678de5` (planning commit; code identical to `49e3fc7`)
- Sibling repository HEAD: Not applicable. External contract: Herdr 0.9.1 (local), Herdr
  docs and CHANGELOG at tag `v0.9.3`.
- Working tree: clean at review start; repairs below touch only `openspec/changes/dashboard-restore/`.
- MODIFIED deltas diffed against the live spec at that HEAD: 4 of 4 (`pane-open` × 3,
  `plugin-manifest` × 1). Every difference is an intended edit; none is a revert. Reviewer A
  recorded the comparison for the first two, and the two blocks added in repair were derived
  from the live text by asserted string replacement, so every difference is one this change
  wrote. The REMOVED+ADDED pair in `pane-open` keeps every live scenario.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | specs/pane-open | `open`'s stale replacement ended at open-then-close with no focus; `--focus` does not raise a background tab (archived `tab-open-focus`), so the user would see nothing | The replacement open now joins the existing post-open re-list and focus; scenario lists six calls ending in a focus of the new pane id | specs/pane-open → stale paragraph and scenario; design.md → Decision 6 |
| CRITICAL | specs/pane-open, tasks.md | Live scenarios with exact call lists ("A domain-error focus…", "A usage-error focus…", "The post-open focus names…", "The already-open path…") and the "parse only `pane list`" rule became false and were not in the delta; task 5.1 named tests that do not exist | Added MODIFIED blocks for "Herdr's own reason is carried verbatim…" and "A newly opened dashboard pane is focused after opening" with the `process-info` call inserted and the parse rule widened; 5.1 now names the five real tests | specs/pane-open; tasks.md 5.1, 5.4 |
| WARNING | tasks.md, specs | A boolean classifier could not tell an unreadable answer (warn) from a live one (silent) | Classification is three-way (`Stale`/`Live`/`Unreadable(reason)`) | specs/dashboard-restore R1; design.md → Decision 4; tasks.md 2.2 |
| WARNING | specs/pane-open | `open`'s replace failure paths unspecified | Added "A failed replacement open stops…" and "A failed close after a successful replacement…" | specs/pane-open; matrix; tasks.md 5.1 |
| WARNING | specs/dashboard-restore | `restore` with a failing/unparseable `process-info` had no scenario or sweep-level test | Added "An unreadable candidate is left alone and warned about" and its test | specs/dashboard-restore; tasks.md 4.1; Decision 12 |
| WARNING | design.md, tasks.md | `HERDR_PLUGIN_ID` handling untested; acceptance argv elided `--plugin` | Pure `plugin_id(env)` shared with `open::context`, unit-tested set/blank/unset; acceptance asserts the full argv; new scenario | specs/dashboard-restore; tasks.md 0.2, 2.1–2.2 |
| WARNING | design.md | `cargo test --lib parse` selected 42 unrelated tests and none of `src/lib.rs`'s | Matrix uses `classifies_as` (3), `a_flag_after_a_subcommand` (1), `usage_` (3), exact `open::tests::` names otherwise | design.md matrix; tasks.md 1.4 |
| WARNING | design.md, tasks.md | Live check ran `restore` by hand, could not observe the hook; named sessions share the global plugin registry, so linking would touch the user's default session; Open Questions/Risks contradicted task order | Live check moved to an isolated Herdr (`HOME`/`XDG_CONFIG_HOME` → scratch; measured: own session dir and registry), linking the working tree and observing the real hook; Risks gain socket scoping, shell-init race, and action/hook race; Open Questions reworded | design.md → Test Boundaries, Decision 13, Risks, Open Questions; tasks.md 9 |
| WARNING | notes/measurements.md | M2/M5 JSON recorded only as prose; whether a restored shell reports `name` was unmeasured | Recorded M6: verbatim `process-info` for all ten `OpenSpec` panes in a real restored session — 6/6 restored shells `name "zsh"`, 4/4 live dashboards `name "herdr-openspec"` | notes/measurements.md M6; tasks.md 0.1, 2.1 |
| WARNING | tasks.md | `0.7.0` floor text left at `SPEC.md` ~1229 (bound verbatim at `tests/degraded-coverage.toml` ~207) and `PRD.md` ~107 | Both sites added, the bound pair changed together | tasks.md 7.2, 11.3 |
| WARNING | tasks.md | Six `src/open.rs` `covers` ranges shift with group 5; new rows need all five fields | 7.2 re-points them and writes every field | tasks.md 7.2 |
| WARNING | tasks.md | `tests/cli.rs`'s usage substring breaks and was not touched before 12.5 | Updated in 1.1, run in 1.4 | tasks.md 1.1, 1.4 |
| WARNING | tasks.md | Groups 3–5 had no REFACTOR step; group 6 lifecycle malformed (non-task 6.4) | REFACTOR steps added; group 6 is CHECK → CHANGE → CHECK (contract gate) → VERIFY | tasks.md 3.3, 4.3, 5.3, 6 |
| WARNING | tasks.md | Doc sites made false were missing (SPEC Overview, module map row, unit-tested modules entry, PRD, AGENTS subcommand/manifest sentences); README "rewrite" targeted text that does not exist | Group 11 names each site; README is "Add" in Install | tasks.md 11 |
| SUGGESTION | proposal.md | Impact omitted `src/lib.rs`, `SPEC.md` → Degraded states, `tests/degraded-coverage.toml` | Added | proposal.md → Impact |
| SUGGESTION | specs/pane-open | REMOVED Reason misstated why | Reworded: the requirement title changes | specs/pane-open → REMOVED |
| SUGGESTION | specs/dashboard-restore | "Needs no terminal" had no scenario; "startup environment names no workspace" stated as fact unmeasured | Never-exits-3 clause added to the acceptance scenario; reworded to "may name no workspace" | specs/dashboard-restore R3 |
| SUGGESTION | tasks.md | `every_run_pipes_and_scrubs_herdr` only guarded `open`/`open-tab`; `placement_for` needs a `Restore` arm; LAUNCHSEAM rejects capitalised `Rect` in comments; manifest test should assert "no other key"; 6.1 predicted 1 failure, not 2; matrix/task fixture mismatch; diff base; archive leaves a placeholder Purpose | Each folded into the owning task or Boundaries note | tasks.md 0.1, 1.2, 6.1, 10.1, 12.8; design.md → Boundaries |

Number sweep: the one corrected count (6.1's "1 failing" → 2) appears nowhere else; the
`--lib parse` filter appeared in design.md's matrix (5 rows) and tasks.md 1.4, both replaced.

## No Remaining Implementation-Blocking Gaps

None remain. `openspec validate dashboard-restore --strict` passes after the repairs, and a
script check confirms all 52 spec scenarios appear in design.md's verification matrix.

## Deferred Non-Blocking Notes

- Hook timing relative to client attach, the hook's environment, the replacement's context,
  and the shell-init race are observed in tasks.md group 9; each has a fallback in design.md →
  Risks, and a missing `HERDR_SOCKET_PATH` in the hook is named there as requiring a
  follow-up rather than acceptance.
