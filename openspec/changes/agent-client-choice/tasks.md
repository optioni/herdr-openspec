<!-- Ordering: sequential, with no `parallel-after` marker anywhere. This is the standing
     parallelism veto recorded at `openspec/config.yaml:81`: one crate, one compile, and every
     gate sweeps the whole tree, so any group's gate run reaches every other group's
     half-written files and criterion 3 fails for every pair. The rule already records it, so
     the pair walk is skipped rather than rediscovered.

     Within that, groups 1–9 run deepest dependency first, and **every group that changes a
     signature moves that signature's call sites in the same group**. `cargo test --lib` builds
     the whole crate, so a call site left stale is a compile error, not a skipped test — the
     first planning review found three groups that could not have built at their own
     verification task. Each such group names the call site it moves. -->

<!-- Checks below were run at HEAD on 2026-09-16 against commit 8238737. Every result records
     what the command SELECTED as well as its exit status. -->

<!-- TEST FILTERS — read this before running any group gate. Two spellings that look right are
     wrong, and both exit cleanly or loudly in ways that hide it:

     1. The filter is a substring of the FULL test path, and the tests live under a `tests`
        module. `cargo test --lib launch::decide` selects **0** and exits **0**; the real path
        is `launch::tests::decide` (10). `src/ui/mod.rs` IS module `ui`, so `ui::mod::wiring`
        selects **0** while `ui::tests::wiring` selects **29**.
     2. `cargo test` takes ONE TESTNAME. `cargo test --lib config:: launch::` is not a
        two-filter run — it exits 1 with `error: unexpected argument 'launch::' found`. The
        working form is `cargo test --lib -- config:: launch::`, measured at **69** = 24 + 45.

     Baselines measured at HEAD, each by running the command:

     | Filter | Selected at HEAD |
     |---|---|
     | `integration::` | 0 — the module does not exist; this is the RED state |
     | `decide` | 10 |
     | `wiring` | 29 |
     | `launch::tests::prompt` | 2 |
     | `launch::` | 45 |
     | `config::` | 24 |
     | `state::` | 29 |
     | `ui::view::` | 159 |
     | `ui::list::` | 48 |
     | `ui::app::` | 169 |
     | `ui::tests::` | 62 |
     | whole `--lib` suite | 1491 |

     Spelling, measured: a bare leaf token (`decide`, `wiring`) is a substring of the full
     path at any nesting, so it survives both a `mod tests` rename and a file move, and is
     preferred where it selects exactly the intended set. Where no bare token isolates the set
     it is not used — `prompt` selects **9** against `launch::tests::prompt`'s **2** — and where
     a group is a whole file the source-file prefix is used (`config::`, `state::`, `ui::view::`).
     The spelling is not the mechanism, though: `cargo test` exits 0 on an empty selection and
     has no flag to change that, so the recorded count is the only thing that makes a zero
     visible, and it also catches a bare token that starts over-selecting later.

     EVERY group gate below must report a selected count STRICTLY GREATER than its baseline,
     because this change only adds tests. A count equal to the baseline means the new tests did
     not run; a count of zero means the filter names nothing. The count, not the exit status, is
     the evidence. -->

## 0. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

- [x] 0.1 Extend the wiring harness in `src/ui/mod.rs`'s test module so the scratch `herdr`
  program answers `integration status` with the recorded seventeen-line corpus, honouring the
  real/replaced split in design.md → Test Boundaries. Verify by asserting the scratch program
  echoes that branch when invoked by hand.
- [x] 0.2 RED: Write the failing end-to-end test for `agent-launch` :: "Pressing `a` splits a
  pane, starts an agent, and sends the prompt" — four **non-`agent list`** entries, `--kind`
  `codex`, and an `agent prompt` element naming the scratch `openspec` program's absolute path
  and containing no `/opsx:`. Name the fixture change `2fa-support`, never a name containing
  the substring `openspec`, or the "no standalone `openspec`" assertion cannot fail.
- [x] 0.3 Confirm it fails because the behavior is missing, not because the harness is
  misconfigured: the run must fail on the prompt text and the missing `integration status`
  entry, while the existing three-entry wiring test still passes unchanged.

## 1. `integration::parse`, and the module's own doc-conformance
<!-- kind: behavior -->

- [x] 1.1 Record the measured status corpus as an `include_str!` fixture under
  `tests/fixtures/`, byte-for-byte as `herdr integration status` printed it on the reference
  machine (17 lines; `claude` `current (v9)`, `codex` `current (v8)`, fifteen `not installed`).
- [x] 1.2 RED: Write failing tests for: "The measured 17-line status parses whole", "A version
  in the status does not become the status", "A parenthesised version on an absent integration
  discriminates the split", "An unrecognised status is treated as installed", "A malformed line
  is skipped and the rest survive", "Empty and blank input yield nothing and report nothing".
  CHECK run at HEAD: `cargo test --lib integration::` → exit 0, **0 selected, 1491 filtered
  out**. Zero selected is a failed check, and it is the RED state: the module does not exist.
- [x] 1.3 GREEN: Add `src/integration.rs` with `Integration { kind, status, installed }` and
  `parse`, splitting each line at the **first** `: ` and the **last** ` (`, and declare
  `pub mod integration;` in `src/lib.rs`. `status` is a field, not an inference — per
  design.md → Decisions 14 it is what makes the split rule falsifiable at all.
- [x] 1.4 CHANGE: Add `integration` to `SPEC.md`'s Module map and to its `### Unit-tested
  modules` section, **in this group**, so `make check` is green at its end rather than red for
  nine groups.
  CHECK run at HEAD with the module planted: `cargo test --test doc_contract` → **FAILED, 105
  passed, 2 failed, 107 selected** — `module_map_matches_lib_rs` ("in src/lib.rs's pub mod set
  but not SPEC.md's Module map: [\"integration\"]") and `tested_modules_names_every_module`.
  Plant reverted; `grep -c '^pub mod ' src/lib.rs` → **14** again.
- [x] 1.5 REFACTOR: Clean up while tests stay green, or state that no refactor was needed.
- [x] 1.6 Run the group tests — `cargo test --lib integration::parse` and
  `cargo test --test doc_contract` — both green; the first must select more than **0** and the
  second **exactly 107**, unchanged: this group adds no doc-contract claim, it only repairs
  the two that 1.4's plant showed failing. (Corrected during apply: the drafted "more than
  107" could not hold for a group that writes no new claim — group 11 is where that count
  moves, to 109.)

## 2. `integration::resolve`
<!-- kind: behavior -->

- [x] 2.1 RED: Write failing tests for: "A configured kind beats every other source", "A
  recorded choice beats the evidence but not the configuration", "A sole installed integration
  is used without asking", "Two installed integrations are refused, not guessed between",
  "Nothing installed and nothing configured reaches `claude` and says so", "A blank configured
  or recorded value is not a value", `integration-status` :: "Every combination is total".
- [x] 2.2 RED: Write failing tests for: "A configured kind with no integration warns and still
  resolves", "A kind Herdr never listed is warned about on the same terms", "A sole integration
  and a last resort carry no absence warning".
- [x] 2.3 GREEN: Implement `Source`, `Choice`, `Resolved`, and `resolve` with the five-step
  precedence, reusing `config::non_blank`'s blank rule rather than restating it.
- [x] 2.4 GREEN: Add the `claude` last-resort literal to `src/integration.rs` as a named
  constant. Do **not** touch `src/config.rs` here — three green tests at `src/config.rs:352`,
  `:375` and `:413` still assert the `"claude"` default and are rewritten in group 3, where the
  type changes with them.
- [x] 2.5 REFACTOR: Clean up while tests stay green, or state that no refactor was needed.
- [x] 2.6 Run the group tests — `cargo test --lib integration::` — green, selecting more than
  group 1 left behind and far more than the **0** baseline.

## 3. `config`: `agent_kind` becomes an override, `[prompts]` arrives
<!-- kind: behavior -->
<!-- Moves the `src/ui/mod.rs:237` call site in the same group; see the ordering note. -->

- [ ] 3.1 RED: Write failing tests for: "Every key is set", "The file does not exist", "The
  directory does not exist", "An empty file is not a malformed file", "Only one key is set", "A
  blank `agent_kind` is not a value", `plugin-config` :: "Unrecognised keys are ignored", "A
  pre-`list-sections` configuration loads unchanged". This rewrites the three
  `assert_eq!(cfg.agent_kind, "claude")` assertions named in 2.4.
- [ ] 3.2 RED: Write failing tests for: "A well-formed override table reaches `Config`", "No
  `[prompts]` table at all", "One malformed entry is skipped and the rest survive", "A
  `prompts` value that is not a table degrades wholesale with one problem", "A blank override
  is treated as absent".
- [ ] 3.3 GREEN: Change `Config::agent_kind` to `Option<String>` with no default, trimming and
  reporting a blank value, add `prompts` per design.md → Decisions 9, and remove the `"claude"`
  literal from `src/config.rs`.
- [ ] 3.4 GREEN: Move the call site — widen `launch::start`'s `kind` parameter to
  `Option<String>` and have the worker fall back to `integration`'s last-resort constant, so
  `src/ui/mod.rs:237` compiles without reintroducing a `"claude"` literal under `src/ui/`.
  Group 7 replaces this parameter entirely with `Settings`.
- [ ] 3.5 CHECK: Re-inspect `Config::agent_kind` against its one consumer,
  `ui::start_collaborators`, and review the diff for breaking changes — the contract gate this
  interface change owes.
- [ ] 3.6 CHECK: Re-read `SPEC.md`'s `config.toml` description and `README.md`'s configuration
  table against the new key set and record every divergence for group 13 — the config-format
  contract gate this repository's rules require.
- [ ] 3.7 Run the group tests — `cargo test --lib -- config:: launch::` — green, selecting more
  than the **69** baseline, and `/bin/sh scripts/gates/wired.sh` still exits 0.

## 4. `state::recorded_kind`
<!-- kind: behavior -->

- [ ] 4.1 RED: Write failing tests for: "A recorded kind is read back", "Absent directory,
  absent file, and empty file are all silent", "An unusable file yields `None` and exactly one
  problem", `plugin-state` :: "Unrecognised keys are ignored".
- [ ] 4.2 GREEN: Implement `recorded_kind(dir) -> (Option<String>, Vec<String>)` reading
  `settings.toml`, on `state::read`'s never-fails, one-problem-per-fault contract.
- [ ] 4.3 Run the group tests — `cargo test --lib state::` — green, selecting more than the
  **29** baseline; state that no refactor was needed if none was.

## 5. `launch::prompt_text`: the CLI-driven shape
<!-- kind: behavior -->

- [ ] 5.1 RED: Write failing tests for: "Each intent produces its own text against the same
  binary and change", "The kind does not reach the prompt", "An empty change name and a lossy
  path are rendered, not refused", "An override replaces one intent's text and leaves the
  others built-in", "A placeholder appearing twice is substituted twice", "An unknown
  placeholder is left verbatim", "An override with no placeholder is sent as written".
- [ ] 5.2 GREEN: Rewrite `prompt_text` to take `(intent, change, openspec, overrides)` and
  produce `agent-prompts`' three built-in rows, substituting `{openspec}` and `{change}` in an
  override and leaving any other brace text verbatim.
- [ ] 5.3 CHECK: Re-inspect the `prompt_text` signature against its one consumer,
  `launch::run_request` (`src/launch.rs:326`), and review the diff.
- [ ] 5.4 Run the group tests — `cargo test --lib launch::tests::prompt` — green, selecting more
  than the **2** baseline; state that no refactor was needed if none was.

## 6. `launch::decide` gains `file_mode`
<!-- kind: behavior -->
<!-- Moves the `src/ui/app.rs:1355` call site in the same group; see the ordering note. -->

- [ ] 6.1 RED: Write failing tests for the two new scenarios — "File mode refuses the three
  launch keys and names the missing binary", "Focus is exempt from file mode" — and for the
  widened `agent-launch` :: "Every combination is total" (sixteen cases). Extend, do not
  replace, the six carried scenarios: "An unreachable socket makes every action key inert",
  "No selected change means no launch, and no agent means no focus", "Each launch intent
  carries its own change and derived name", "A derived name already live in the session is
  refused before any Herdr call", "A second press while a launch is in flight is refused, not
  queued", "Focus still works while a launch is in flight".
- [ ] 6.2 GREEN: Append `file_mode: bool` as the seventh and last parameter and insert the
  refusal as step 3, per design.md → Decisions 6 and 7.
- [ ] 6.3 GREEN: Move the call site — pass `self.file_mode` from
  `ui::app::apply_launch_action` (`src/ui/app.rs:1355`) so the crate compiles.
- [ ] 6.4 CHECK: Re-inspect `decide`'s signature against that consumer and confirm `in_flight`
  is still the sixth argument, per design.md → Decisions 6.
- [ ] 6.5 CHANGE: Update `src/launch.rs:70`'s doc comment, which says `decide` is "a pure total
  function of its six arguments", and `src/launch.rs:18`'s `Intent` doc comment, the one
  remaining `/opsx:` site no other task names. Both are production prose that this change
  falsifies and no gate reads.
- [ ] 6.6 Run the group tests — `cargo test --lib -- decide ui::app::` — green,
  selecting more than the **179** baseline (10 + 169); state that no refactor was needed if none
  was.

## 7. `launch::Settings`, lazy resolution, and the ambiguous stop
<!-- kind: behavior -->
<!-- Moves the `src/ui/mod.rs:234` call site in the same group; see the ordering note. -->

- [ ] 7.1 RED: Write failing tests for: "The first launch reads the status and the second does
  not", "Focus never reads the status", "A failed status call still launches the configured
  kind", "A failed status call with nothing configured reaches `claude` with two problems",
  "Unparseable output is not a failed call", "`g` still works with no binary".
- [ ] 7.2 RED: Write failing tests for the ambiguous stop: "Two installed integrations stop the
  launch with one problem and no pane", "The ambiguous refusal clears the in-flight flag and
  leaves `g` working", "A configured kind suppresses the stop entirely".
- [ ] 7.3 RED: Write failing tests for: "The three calls appear in order with the split's own
  pane id", "Each intent sends its own `/opsx:*` command and nothing else" (title kept verbatim
  per design.md → Decisions 13; the body asserts the CLI shape), "An archived change launches
  on the same terms as an active one", "The prompt carries the probe's own path, not a bare
  command".
- [ ] 7.4 RED: Write failing tests for the failure ladder: "A failed split leaves nothing
  behind", "A failed start leaves the pane and names it", "A failed prompt leaves a running,
  un-prompted agent that is still attributable", "A failed recording does not undo a successful
  start", "A failed recording and a failed prompt are both reported", "Two resolution problems,
  a failed recording, and a failed prompt are all four reported", "A success clears both
  entries".
- [ ] 7.5 GREEN: Introduce `launch::Settings` (no `Default`), change `launch::start` to
  `(cli, Settings)`, and move the `src/ui/mod.rs:234` call site to construct one.
- [ ] 7.6 GREEN: Resolve the kind in the worker on the first `Request::Launch` only and cache
  the `Choice` for the process. A `Choice::Ambiguous` **stops** the launch before `pane split`
  with one problem and `named` `None`; every other resolution problem is pushed before the
  three calls run, summarised to at most two per design.md → Decisions 11.
- [ ] 7.7 CHECK: Re-inspect `launch::start`'s signature against its one consumer,
  `ui::start_collaborators`, and review the diff — the contract gate this interface change owes.
- [ ] 7.8 CHECK: Confirm the migration, backfill, cache invalidation, and index rebuild steps
  this change requires — design.md → Persistence and Rollout records none, and states that the
  cached `Choice` is deliberately not invalidated mid-session.
- [ ] 7.9 CHECK: Run `/bin/sh scripts/gates/launchseam.sh` and confirm the `HerdrCli` handle is
  still confined to the same five files, `src/integration.rs` not among them.
  CHECK at HEAD (green invariant + negative control): baseline → exit **0**, reporting
  `35 files searched (>= 25)`; plant `// planted: fn _p(_c: &dyn crate::cli::HerdrCli) {}`
  appended to `src/ui/view.rs` → exit **1**, one `LAUNCHSEAM FAIL (leg 3)` line naming it;
  `git checkout -- src/ui/view.rs` → exit **0**.
- [ ] 7.10 CHANGE: Update `src/launch.rs:55-61`'s `Outcome` doc comment, which states the
  `problems` bound as "at most two", to the bound design.md → Decisions 11 now fixes at four.
  Production prose no gate reads, falsified by this change.
- [ ] 7.11 REFACTOR: Clean up while tests stay green, or state that no refactor was needed.
- [ ] 7.12 Run the group tests — `cargo test --lib -- launch:: ui::tests::` — green, selecting
  more than the **107** baseline (45 + 62).

## 8. The view layer: the footer, the problem rows, and the stale descriptions
<!-- kind: behavior -->

- [ ] 8.1 RED: Write failing tests for: `agent-launch` :: "The action hints appear at both
  mandated widths when the socket is reachable", "File mode drops the launch hint and keeps the
  focus hint", "An unreachable socket hides both hints at both widths", "The count is dropped
  before the action hints as the width falls", "The action keys type into the query while
  filtering", and `responsive-layout` :: "File mode drops `a/c/s launch` and keeps `g focus`" —
  each at 60 **and** 120 columns.
- [ ] 8.2 RED: Write failing tests for: "A refused launch renders one row at both widths", "A
  file-mode refusal renders one row at both widths", "Four outcome problems render as four
  leading rows", "The ambiguous stop renders as one leading problem row at both widths", "A
  launch problem leads the refresh and change-set problems", "A later success clears an earlier
  failure" — each at the mandated 38 and 58 interior widths.
- [ ] 8.3 GREEN: Gate the `a/c/s launch` hint in `render_footer` on `reachable && !file_mode`
  while leaving `g focus` on `reachable` alone.
- [ ] 8.4 GREEN: Rewrite the three `/opsx:`-naming `description` strings in `src/ui/help.rs`
  and the one `/opsx:` doc comment in `src/ui/app.rs` to name the CLI shape.
  CHECK at HEAD (RED for the behavior this change adds):
  `for f in $(git ls-files 'src/*.rs' 'src/**/*.rs'); do awk '/^#\[cfg\(test\)\]/{exit} {print}' "$f"; done | grep -c '/opsx:'`
  → **8 matched** (`src/launch.rs` 4, `src/ui/help.rs` 3, `src/ui/app.rs` 1). It must read
  **0** when this change lands; 8 is the RED state and the count is what proves the sweep
  reaches the files.
- [ ] 8.5 CHECK: Run `cargo test --test doc_contract` and confirm the key-binding and
  mouse-table claims still pass — descriptions are not compared there
  (`grep -n description tests/doc_contract.rs` → **0 matches**), so 8.4 must not move them.
- [ ] 8.6 Run the group tests — `cargo test --lib -- ui::view:: ui::list:: ui::app::` — green,
  selecting more than the **376** baseline (159 + 48 + 169); state that no refactor was needed
  if none was.

## 9. The composition root
<!-- kind: behavior -->

- [ ] 9.1 RED: Write failing tests for: "File mode carries no path and builds no prompt", "With
  nothing configured, the sole installed integration is what launches", "File mode leaves `a`
  refusing and `g` working in the shipped root", "Startup issues no status call", "Nothing in
  this change writes `settings.toml`".
- [ ] 9.2 RED: Write failing tests for the two `Settings` threads no gate can see — "The
  recorded kind reaches `--kind` and outranks the installed evidence" and "A per-kind prompt
  override reaches the logged `agent prompt`". `WIRED`'s name list carries neither
  `state::recorded_kind` nor `config.prompts`, so a root that skips both passes `make check`.
- [ ] 9.3 GREEN: Populate `launch::Settings` in `start_collaborators` from `config.agent_kind`,
  `state::recorded_kind`, `config.prompts`, `resolution.found.as_ref().map(|f| f.path)`, and
  the state directory, keeping `file_mode` and `openspec_bin.is_none()` the same fact. Keep the
  literal text `config.agent_kind` in that production slice — `WIRED` leg 6 is a bare substring
  grep, so a destructure that drops those words fails it.
- [ ] 9.4 CHECK: Re-inspect every consumer named in design.md → Contracts and confirm each call
  site moved. Run `/bin/sh scripts/gates/wired.sh` and confirm leg 6 still finds
  `config.agent_kind` in `src/ui/mod.rs`'s production slice.
- [ ] 9.5 Run the group tests — `cargo test --lib ui::tests::` — green, selecting more than the
  **62** baseline; state that no refactor was needed if none was.

## 10. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [ ] 10.1 VERIFY: Confirm group 0's test passes end to end, plus "Pressing `g` after the launch
  focuses the pane the launch created" and "An unreachable socket leaves every key inert and the
  pane a working TUI".
- [ ] 10.2 VERIFY: Run the three recorded plants for "The wiring test fails when the launcher is
  replaced by the inert double" — `launch::none()` substituted, `run_loop` not handing `pending`
  over, and `Settings::openspec_bin` hardcoded to `None` — and confirm each fails a distinct
  assertion set. The third fails on a missing `integration status` **and** `pane split`, which
  is only true because the worker refuses a `Launch` carrying `None` (per `agent-prompts`);
  `Collaborators::file_mode` comes from `cli.is_none()` at `src/ui/mod.rs:211`, not from
  `Settings`, so `decide` still returns `Go` under that plant.
- [ ] 10.3 VERIFY: Measure both wiring scenarios' wall time at 120x20 and 60x20 and record it
  against `testutil::Stages`' 30-second deadline (`src/lib.rs:689`), which is shared across
  every stage. This change takes "Pressing `a`" to four non-`agent list` entries and "Pressing
  `g`" to five under that unchanged budget; `ui::tests::wiring` measured **43.4 s for 29 tests**
  at HEAD, so the per-scenario margin is what needs stating, not assuming.
- [ ] 10.4 REFACTOR: Clean up harness setup and the scratch programs if warranted.

## 11. Gates and doc-conformance
<!-- kind: operational -->

- [ ] 11.1 CHANGE: Add an eighth `NODEFAULT-UI` line to the `Makefile` for
  `HOMEFILE=src/launch.rs TYPES='Settings'`, measure its true floor, and record the measurement
  in `openspec/changes/agent-client-choice/notes/gate-floors.md`, which this task creates —
  `notes/` is change-local in this repository and no such file exists at the root.
  CHECK at HEAD: `grep -c 'nodefault-ui.sh' Makefile` → **7**, so the new line is the eighth.
- [ ] 11.2 CHANGE: Add a `tests/doc_contract.rs` claim that `src/integration.rs`'s production
  slice names no filesystem, process, environment, network, standard-I/O, or `ratatui` name —
  `ratatui` included because `LAUNCHSEAM` covers `HerdrCli` but **no** `make gates` script
  sweeps `src/integration.rs` at all — the
  **fourteenth** bound claim, on `src/specs.rs`' terms, since no `make gates` script sweeps
  either file. Prove it falsifiable by planting `use std::fs;` above the `#[cfg(test)]` line,
  showing the claim fires, removing it, and showing it goes quiet.
  CHECK at HEAD: `SPEC.md` → § Doc-conformance checks lists **13** bound claims and
  `AGENTS.md:289` says "thirteen further claims", so this is the fourteenth.
- [ ] 11.3 CHANGE: Add a fifteenth `tests/doc_contract.rs` claim that the production slice
  holds no `/opsx:` literal, so 8.4's one-off grep becomes a committed regression guard.
  CHECK at HEAD: `grep -rn 'opsx' scripts/gates/ tests/ Makefile` → **0 matches**, so nothing
  in `make check` sweeps for it today and the guard is genuinely new.
- [ ] 11.4 CHANGE: Add the new degraded-state rows to `SPEC.md`'s table — unreadable
  `integration status`, ambiguous resolution, last-resort `claude`, absent integration for the
  chosen kind, and the file-mode launch refusal — and bind each to a named passing test in
  `tests/degraded-coverage.toml`.
- [ ] 11.5 VERIFY: Run `make gates`, `cargo test --test doc_contract` (the suite 11.2 and 11.3
  extend), and `cargo test --test degraded_coverage` — all green, each with a non-zero selected
  count.

## 12. Change Review
<!-- kind: operational -->

- [ ] 12.1 CHECK: Dispatch the `outside-in-tdd-reviewer` subagent against proposal.md, all
  seven spec files, design.md, and tasks.md with only the diff — not the implementing session's
  reasoning.
- [ ] 12.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
  one-line reason, note SUGGESTIONs, and re-run affected tests.
- [ ] 12.3 VERIFY: Confirm no blocking or unowned finding remains.

## 13. Documentation
<!-- kind: operational -->

- [ ] 13.1 Rewrite in `README.md`: the `a`/`c`/`s` rows of the Keys table and the `agent_kind`
  row of the configuration table at line 88 (audience: plugin users) — **four** rows naming
  `/opsx:apply` and a `claude` default that no longer exist. Replaces four stale rows rather
  than adding beside them.
- [ ] 13.2 Rewrite in `SPEC.md`: the `a`/`c`/`s` Keys rows, the `launch` row of the module
  table, and the Launch-flow prose showing `herdr agent prompt <name> "/opsx:apply <change>"`
  (audience: maintainers) — `grep -n opsx SPEC.md` prints **seven lines across six sites**
  (93, 659, 660, 661, 884, 902–903, the last a single wrapped prose site), every one now false.
- [ ] 13.3 Add to `SPEC.md`: the `[prompts.<kind>]` config format, `settings.toml`'s one read
  key, and the two new doc-conformance bullets from 11.2 and 11.3, in the `config.toml`,
  state-directory, and § Doc-conformance checks sections respectively (audience: maintainers) —
  the on-disk contract and the bound claims a future change must not break.
- [ ] 13.4 Rewrite in `AGENTS.md`: the "Current repo state" sentence describing `a`/`c`/`s` as
  sending `/opsx:*`, the architecture bullet's file inventory to name `src/integration.rs` and
  the lazy once-per-session `integration status` read, and the claim count at `AGENTS.md:289`
  from "thirteen further claims" to fifteen (audience: every future session) — a durable seam
  rule a future change would otherwise get wrong by resolving the kind on the render path.
  Rewrites existing sentences; net addition under ten lines.
- [ ] 13.5 Rewrite in `PRD.md`: the Non-Goals bullet at line 52 naming `/opsx:*` as the
  authoring route (audience: maintainers) — the non-goal still holds, but its example command
  is one this change removes.

## 14. Lint & Verify
<!-- kind: operational -->

- [ ] 14.1 CHECK: Inspect the intended verification commands and affected tiers — `make check`
  runs format, lint, gates, test, and coverage, and every tier this change touches is inside it.
- [ ] 14.2 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 14.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.
- [ ] 14.4 VERIFY: `make gates` — every script green.
- [ ] 14.5 VERIFY: `cargo test --all-features` — green, with the selected count above the 1491
  measured at HEAD.
- [ ] 14.6 VERIFY: `cargo llvm-cov --fail-under-lines 80` and the production-slice floor via
  `scripts/coverage-prod.py` — both green, neither lowered nor waived.
- [ ] 14.7 VERIFY: `make check` as the single gate; if it fails, name the failing sub-command.
- [ ] 14.8 VERIFY: `openspec validate agent-client-choice --strict` — valid.
