## Reviewed Artifacts

- `proposal.md`
- `specs/plugin-config/spec.md`
- `specs/plugin-state/spec.md`
- `specs/plugin-build/spec.md` (delta: REMOVED + ADDED)
- `specs/ci-workflow/spec.md` (delta: MODIFIED — added *during* this review; see the
  finding that produced it)
- `design.md`
- `tasks.md`

The finding pass was delegated to three independent reviewers, none of which wrote the
planning package and none of which was a fork of the planning session. Each received the
change directory and one slice of the review list: capability coverage and scenario
quality; design completeness, test boundaries, and test strategy; task alignment,
contradictions, and repository invariants. Each was given an explicit scratchpad path and
instructed to append findings as it discovered them rather than only in its final
message — the mitigation `HANDOFF.md` asks for after two review rounds were lost to
`529 Overloaded`. All three reports survived and are the source of the table below. The
reviewers edited nothing; every repair was made in this session, in the artifact that
owns it.

They returned **5 CRITICAL, 26 WARNING, 17 SUGGESTION** across 48 findings. Every
CRITICAL and every WARNING is repaired below. Fifteen of the seventeen SUGGESTIONs are
applied; the two that are not are recorded under Deferred Non-Blocking Notes.

## Reviewed Against

- This repository HEAD: `876e3c601f0ba8bc9a5731a08470cdd575074bd5`
  (`docs(plugin-config): add proposal, specs, design, and tasks`)
- Sibling repository HEAD: Not applicable. `herdr` and `openspec` are consumed as
  installed binaries, not as source siblings.
- Working tree: clean apart from this change's own planning directory,
  `openspec/changes/plugin-config/`, which was committed at the HEAD above before the
  review was dispatched so the reviewers read a fixed tree. No source file is touched by
  this change yet.
- Environment facts confirmed during review, on this machine:
  - `herdr 0.8.2`, installed at `/opt/homebrew/bin/herdr`; a Herdr server is running.
  - `openspec 1.11.0`, nvm-installed and not on the default `PATH`.
  - `cargo 1.91.1`; `cargo info toml` reports `1.1.5+spec-1.1.0`, `rust-version: 1.85`,
    `default = [std, serde, parse, display]`.
  - `openspec/specs/` holds four live capabilities: `ci-workflow`, `plugin-build`,
    `plugin-manifest`, `quality-gates`.
  - `openspec validate plugin-config --strict` reports the change valid, before and
    after the repairs.

**The design's central premise was established empirically before planning, and
re-confirmed during it.** `SPEC.md` says the plugin configuration directory comes from
`herdr plugin config-dir herdr-openspec`, which a plugin process cannot call without a
spawn outside `cli` — a module that does not exist until Phase 3. Two throwaway plugins
were linked, exercised, and unlinked against the running Herdr 0.8.2:

| Probe | Entrypoint | `HERDR_PLUGIN_CONFIG_DIR` | `HERDR_PLUGIN_STATE_DIR` |
|---|---|---|---|
| `envprobe` | `[[panes]]`, opened with `herdr plugin pane open --no-focus` | `~/.config/herdr/plugins/config/envprobe` | `~/.local/state/herdr/plugins/envprobe` |
| `envprobe2` | `[[actions]]`, invoked with `herdr plugin action invoke` | `~/.config/herdr/plugins/config/envprobe2` | `~/.local/state/herdr/plugins/envprobe2` |

Both values are character-identical to what `herdr plugin config-dir <id>` prints. Both
processes also received `HERDR_ENV=1`, `HERDR_PLUGIN_ROOT`, `HERDR_PLUGIN_ID`,
`HERDR_SOCKET_PATH`, `HERDR_PLUGIN_CONTEXT_JSON`, and the workspace, tab, and pane ids;
the pane process carried `HERDR_PLUGIN_ENTRYPOINT_ID` and the action process
`HERDR_PLUGIN_ACTION_ID`. Both plugins were unlinked and the directories Herdr created
for them removed. The action probe was added *during* this review, in response to a
finding that the claim "every pane and action process" rested on a pane-only probe.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | specs/plugin-state | The bad-entry fixture was `good-agent = "a-real-change"`, `bad-agent = 7`, `Bad_Name! = "another-change"`. `!` is not permitted in a TOML bare key, so the file is a **syntax error**, not a file with one bad entry — verified with `tomllib`. The scenario asserted one surviving pair and two problems; the correct behaviour for that input, under the sibling clause of the same requirement, is an empty mapping and one problem. The two scenarios contradicted each other on the same input, and the RED task could not be made to pass without corrupting the parser. | Fixture rewritten to four entries that parse: `good-agent`, `bad-agent = 7`, `Bad_Name` (a **legal bare key** that fails the agent-name pattern on its uppercase letters — the case that actually exercises the regex guard), and `"has spaces!"` (legal *quoted* key, illegal agent name). Three problems now expected. | specs/plugin-state; design.md matrix; tasks.md 5.1 |
| CRITICAL | design.md | `env PATH="/usr/bin:/bin" cargo test --all-features` cannot run: `env` execs through the **new** `PATH`, and cargo lives in `~/.cargo/bin`. Verified: exit 127. This is a regression of a defect `repo-foundation`'s review already found and repaired, reintroduced verbatim. | Replaced with a `PATH` that keeps the toolchain and removes only the directory holding `herdr`, built with `tr` (never `paste -sd:`, a BSD usage error), preceded by an assertion that `herdr` is genuinely unresolvable on it. Verified working: `herdr` unresolvable, `cargo 1.91.1` still resolvable. | design.md → Test Strategy shorthand and matrix; tasks.md 6.3 |
| CRITICAL | design.md | The dependency-set check ended in `print(...)`: it exits 0 for any dependency list. The requirement it is the sole evidence for — the whole justification for retiring "no third-party dependencies" — would have been unenforced from day one. It also claimed to verify `default-features` and the feature list while inspecting neither. | `DEPS` now asserts the normal-kind dependency list is exactly `["toml"]`, that `uses_default_features` is `false`, and that `features` are the four named, all from `cargo metadata`'s resolved output. Verified both ways: fails against this repository today (no dependency yet), passes against a probe crate declaring `toml` as specified. | design.md → Test Strategy shorthand; specs/plugin-build; tasks.md 1.6 |
| CRITICAL | design.md, specs/plugin-state, tasks.md | The atomicity evidence could not distinguish `rename` from truncate-in-place. Reviewer 2 proved it: both produce identical final bytes, and "no `.tmp-` entry remains" is *satisfied* by a plain `fs::write` — the exact non-atomic implementation the requirement forbids. The spec's own wording ("at no point does the file contain…") was unobservable by a single-threaded test, and `tasks.md` 5.3 described a third, different, also-insufficient check. | One technique everywhere: hard-link the existing file to a second path in the same directory before recording, then assert the target holds the new mapping **and the link still holds the previous bytes**. True of a rename, false of every in-place write. Scenario retitled "The file is replaced by rename, not written in place" and rewritten to assert the mechanism; the design row and task 5.3 now say the same thing and name the two insufficient forms so they are not reintroduced. | specs/plugin-state; design.md matrix; tasks.md 5.3 |
| CRITICAL | design.md, tasks.md | The only test of `load_from_env` compared `f(env)` with `f(env)`: the "explicit lookup" was itself built from `std::env::var`. On any machine with no real `config.toml` — every machine, since the plugin never creates one — both sides are `Config::default()`, so a `load_from_env` that ignored the environment entirely would pass. The Test Boundaries table claimed the wrapper was covered, which is what stops anyone noticing it is not. | Split the untestable composition from the testable binding: `env_lookup()` is now a public function, asserted against two discriminating facts (`env_lookup()("PATH")` equals `std::env::var("PATH").ok()` and is `Some`; an unset name is `None`), and `load_from_env` is a one-line composition with nothing left to assert. The boundary row says so honestly. The task explicitly forbids the tautological form. | design.md → Contracts and Test Boundaries; tasks.md 2.4, 2.5, 2.6 |
| WARNING | specs/plugin-state, design.md | "Four base-36 digits of a 32-bit FNV-1a hash, left-padded with `0`" is not implementable as written: a `u32` needs seven base-36 digits, so the hash must be *reduced*, and the spec never said how. "First four", "last four", and `% 36⁴` give three different agent names — for a value written to disk. | Step 6 now says `fnv1a32(original) % 36^4`, rendered lowercase base-36 and zero-padded to four digits, with the reason stated. Every truncation scenario now pins an **exact literal** output rather than a length and a regex, so the reduction is locked by a test: `add-really-long-change-name-8jqt`, `…-alft`, `…-mmky`, `abcdefghijklmnopqrstuvwxyz-lhun`. All four independently computed and cross-checked by two reviewers and this session. | specs/plugin-state; design.md → Decisions and matrix; tasks.md 4.1, 4.4 |
| WARNING | proposal.md, specs/plugin-build | The proposal promised the replacement requirement "forbid[s] a proc-macro dependency"; the ADDED requirement contained no such clause, so the guard would evaporate at archive and the delta granted less than the proposal advertised. Separately, the feature-list assertion read `Cargo.toml` and asserted `Cargo.toml` — the exact shape `tasks.md`'s own header comment forbids. | New scenario "The resolved build graph is small and proc-macro-free" asserts the `cargo tree -e normal` set exactly and the absence of `syn`, `quote`, `proc-macro2`, `serde_derive`; the requirement carries the SHALL. The declaration check now reads `cargo metadata`'s resolved `uses_default_features`/`features` rather than the manifest text. Both verified against a probe crate. | specs/plugin-build; proposal.md; design.md matrix and shorthand; tasks.md 1.6, 1.7 |
| WARNING | specs/plugin-build, tasks.md | The delta pinned `toml = 1.1.5` in a table that becomes a live requirement, while task 1.2 explicitly authorised recording a newer version, and no scenario asserted the version — so the divergence was undetectable and could silently falsify the MSRV argument. | The table cell is now "at least `1.1.5`" and the requirement adds "the declared version's own `rust-version` SHALL be no higher than this crate's". Task 1.2 must confirm the MSRV and update the cell if it moves. | specs/plugin-build; tasks.md 1.2 |
| WARNING | live `openspec/specs/ci-workflow/spec.md` | The live requirement "CI invokes every gate through `make`" justifies its single-line `run:` rule with "**the crate may take no dependency**, so the guard is std-only string matching". This change makes that premise false. `plugin-build` was being carefully amended for exactly this reason while the identical assumption was left standing next door. | Added a fourth delta file, `specs/ci-workflow/spec.md`, MODIFIED with the full requirement and only the rationale clause rewritten: the guard is deliberately std-only because a YAML parser as a dev-dependency to read four lines is not a trade this repository makes. No scenario changes; all five are carried verbatim and verified byte-identical against the live spec by `diff`. Listed in proposal.md → Modified Capabilities. | specs/ci-workflow (new); proposal.md; design.md → Test Strategy matrix (five rows at the "existing test" tier) |
| WARNING | specs/plugin-config, specs/plugin-state, tasks.md | Two new degraded conditions — a malformed or wrong-typed `config.toml`, and an unusable `agent-names.toml` — were introduced with no row in `SPEC.md`'s degraded-states table, which `openspec/config.yaml` → `rules.specs` names as the contract and which `degraded-states` will be planned from. | Added task 8.5: two rows in `SPEC.md` → Degraded states, worded so `degraded-states` inherits them rather than inventing them. Also named in design.md → Persistence and Rollout → Observability and in the `problems` risk bullet. | tasks.md 8.5; design.md; proposal.md → Impact |
| WARNING | specs/plugin-state | "A name of exactly 32 characters is not truncated" named no concrete input, and its "produces a different, **32-character** result" clause is true only for some inputs of that class — step 6 trims trailing separators from the 27-character prefix, so a name breaking at a dash yields 31. The scenario would go green or red depending on a fixture the spec declined to name. | The inputs are now named (`add-really-long-change-name-that` / `…-thatx`) with exact expected outputs, and a **new** scenario "A truncated name may be shorter than 32 characters" pins the 31-character case explicitly, making "at most 32" the stated rule. | specs/plugin-state; design.md matrix; tasks.md 4.1 |
| WARNING | specs/plugin-config | "Resolution spawns nothing" asserted a property of the whole `src/` tree — no `Command`, no `Stdio`, "because `cli` does not exist yet". `subprocess-seam` creates `src/cli.rs` containing exactly those, at which point a live archived scenario is false. The scenario also forbade "a `herdr` invocation", which after this change is not a testable form: `src/` necessarily contains the literal `herdr` inside the fallback paths and the doc comments this change requires. | The scenario is rescoped to `src/config.rs` and `src/state.rs` and to named process APIs and program-name literals, with an explicit note that prose naming `herdr plugin config-dir` is documentation and not an invocation. The "`cli` does not exist yet" observation moved to design.md → Boundaries, where it can go stale harmlessly. | specs/plugin-config; design.md → Boundaries and matrix; tasks.md 6.2 |
| WARNING | design.md, tasks.md | Following on from the above: the proposed pattern forbade `std::process` tree-wide. Verified against the real tree — it fires on `src/main.rs:2: use std::process::exit;`, a legitimate exit and not a spawn. The check would have gone red on landing. | Two patterns now, both verified against the real tree and against a simulated doc comment: tree-wide `process::Command|Command::new|Stdio`, and a stricter module-scoped `std::process|Command|Stdio` over `src/config.rs` and `src/state.rs` only. Both traps are named in the task so they are not reintroduced. | design.md → Boundaries, `SPAWN` block; tasks.md 6.2 |
| WARNING | specs/plugin-state | Recording an agent name already bound to a **different** change was unspecified, and task 5.6 quietly settled it as last-writer-wins. This is not hypothetical: `agent_name` is many-to-one — `""`, `!!!`, and `---` all derive to `change` — so the unspecified case lands in `agent-attribution` as *misattribution*, which the repository's "attribution must not guess" concentration point exists to prevent. | The requirement now states the rule (the new binding replaces the old, because the most recent launch is the live one, and it is not reported as a problem) with a scenario pinning it. design.md → Decisions records it as the collision handling the hash suffix does not eliminate. | specs/plugin-state; design.md → Decisions; tasks.md 5.1, 5.6 |
| WARNING | design.md, herdr-plugin.toml | `min_herdr_version` staying at 0.7.0 was defended by "the variables were not introduced by 0.8". Every piece of evidence in the change is about the installed 0.8.2; nothing tests 0.7.x and nothing on this machine can. The conclusion is right, the stated reason is not — and the reason is what a later change inherits. | Risk bullet rewritten to rest on the fallback instead: the floor stays because the variables' *absence* is a supported state resolving to the same paths, explicitly **not** because 0.7 is known to inject them, with a warning that a later change needing a Herdr-injected value with no computable fallback must revisit the floor. `min_herdr_version` is unchanged. | design.md → Risks |
| WARNING | proposal.md, design.md, specs | "Every pane **and action process**" was asserted in normative spec prose on the strength of a pane-only probe. `agent-launch` would have inherited it as settled fact. | Probed empirically during this review (see Reviewed Against): an `[[actions]]` entrypoint receives both variables. Task 6.1 now requires **both** entrypoint kinds, and design.md → Context records both results and the two entrypoint-specific variables that differ. | design.md → Context and Decisions; tasks.md 6.1 |
| WARNING | design.md, tasks.md | The binary-target row's Verification promised "assert the built file is executable" while its Command contained only the `cargo metadata` filter. A release build broken by the first dependency — the classic failure mode here, and `scripts/build.sh` has its own `~/.cargo/env` path — could not fail that check. | Both halves now run: the metadata filter, then `/bin/sh scripts/build.sh && test -x target/release/herdr-openspec`. | design.md matrix; tasks.md 1.8 |
| WARNING | design.md, tasks.md | The dependency-removal check backed up and restored only `Cargo.toml`, but removing the dependency and running `cargo build` **rewrites `Cargo.lock`** (verified: the lock's hash changes). The change's own central build guarantee is that `Cargo.lock` is committed and `--locked` succeeds; a check that transiently invalidates it, in a tree the tasks say may hold uncommitted work, is the wrong shape. | Both files are copied aside and both restored, with an explicit post-condition — `cargo build --locked` exits 0 and `Cargo.lock` is byte-identical to the copy — and a note that `cargo build` is *expected* to fail mid-sequence, so it must not run under `set -e`. | design.md matrix; tasks.md 6.4 |
| WARNING | design.md → Test Boundaries | Tasks 6.1 and 6.5 read and **delete** directories under the user's real `~/.config/herdr` and `~/.local/state/herdr`, which no boundary row named — the table's own contract is that silence is not an answer, and a destructive collaborator is the last one that should be unlisted. A mistyped id would remove a real plugin's configuration. | New row naming the user's real Herdr directories as real and destructive, restricted to probe ids that are never `herdr-openspec`, with removal by the exact path `herdr plugin config-dir <id>` prints and never by a glob. Task 6.1 says the same, and a matching risk bullet was added. | design.md → Test Boundaries and Risks; tasks.md 6.1 |
| WARNING | design.md → Test Boundaries | Three collaborators unlisted: the POSIX shell userland every command check depends on (precisely where the BSD/GNU traps live), `cargo llvm-cov` (run by 9.5), and `std::env::temp_dir()`, which reads `TMPDIR` from the real environment and therefore contradicted the `std::env::var` row's "not called". | All three added. The shell row states macOS/BSD as the reference platform and names the four GNU-only forms that must not appear, `paste -sd:` among them. The `std::env::var` row now says exactly where the real environment is read and that it is never mutated. | design.md → Test Boundaries |
| WARNING | proposal.md | Impact → Docs listed only `SPEC.md` and `README.md`, while tasks 8.5/8.6 **rewrote an `AGENTS.md` architecture rule** — the highest-leverage, least visible documentation edit in the change — and no artifact named `openspec/IMPLEMENTATION-ORDER.md`. | Impact → Docs now enumerates all five documents and their sections; What Changes gains a clause about the two `AGENTS.md` rules and the roadmap row. | proposal.md → What Changes and Impact |
| WARNING | openspec/IMPLEMENTATION-ORDER.md | The Phase 1 `plugin-config` row still reads "Read `config.toml` from the directory reported by `herdr plugin config-dir`" — the literal claim design.md → Context proves impossible for a plugin process, and this change's *own* roadmap row. `repo-resolution` is planned from it. The repository's precedent defers stale roadmap rows to archive time, but archive fires after the whole implementation, so the wrong text would ship through it. | Added task 8.7 to correct the row now, with the dependency graph confirmed unchanged (`subprocess-seam` is correctly absent as a dependency). Added to proposal Impact. | tasks.md 8.7; proposal.md |
| WARNING | SPEC.md → Herdr integration → Attributing an agent | Tier 1 says only that "longer change names are **truncated** and the mapping is recorded", and asserts change names "are already kebab-case". `agent_name` also lowercases, replaces, trims, substitutes, and prefixes, and records whenever the derived name differs — `2fa-support` becomes `c-2fa-support` at 13 characters and is unattributable without the mapping. `agent-attribution` planned from that paragraph would implement a lookup for over-long names only. | Added task 8.6 to rewrite the paragraph with the real derivation and the real recording rule, and to record explicitly that Launch flow's two `<change>` argument lines now mean the *derived* name and that correcting them is `agent-launch`'s — so the obligation is handed over rather than lost. | tasks.md 8.6; design.md → Decisions |
| WARNING | tasks.md, AGENTS.md | Task 8.6 (old numbering) added a Conventions line restating `AGENTS.md`'s existing "Never write to OpenSpec files" architecture rule in a second section, instead of correcting that rule — which now reads as "the plugin writes nothing", no longer true. | Split: the injected-lookup line stays in Conventions (genuinely new); the write rule is a **rewrite in place** of the Architecture-rules bullet, alongside the spawn bullet, in one task that says "rewrite both; do not append a third". | tasks.md 8.9, 8.10 |
| WARNING | tasks.md → group 8 | The Documentation group was marked `operational` but had no CHECK and opened with an edit, so 8.1 would rewrite `SPEC.md` from memory of the design document rather than from the file. `repo-foundation`'s review flagged and repaired this exact defect in its own doc group. | Group renumbered to CHECK (8.1, re-reading all ten target sections as they stand at implementation time) → CHANGE (8.2–8.11, each marked) → VERIFY (8.12). | tasks.md group 8 |
| WARNING | tasks.md → header comment | The opening comment presented itself as an exhaustive audit of `openspec/config.yaml`'s concentration points but covered seven of nine. "Agent attribution must not guess" and "coverage counts the whole crate" were neither scheduled nor recorded — and the coverage one is live here, since 2.4 exists precisely because of it. | Comment extended to all nine, with attribution recorded as not applicable and *why* (no attribution path exists; `read` skips a bad entry rather than repairing it), and coverage recorded as scheduled, citing 2.4, 2.6, and 9.5. | tasks.md header |
| WARNING | specs/plugin-build | The new requirement said "`cargo build --locked` SHALL succeed" with nothing recurring to enforce it: `make check` runs `cargo test --all-features`, whose exact text is pinned by the live `quality-gates` requirement, so adding `--locked` would need a second delta into a capability this change does not otherwise touch. The requirement would have been decorative on the day it landed, while the continuously-checked scenario it replaces was deleted. | The requirement is scoped to what is genuinely enforced: the change that introduces or alters a dependency verifies `--locked` at the commit that lands it. The gap between such changes is recorded in design.md → Risks rather than left implied. | specs/plugin-build; design.md → Risks |
| WARNING | specs/plugin-config, specs/plugin-state | Two containment SHALLs had no evidence. The config requirement promised "SHALL write nothing anywhere under the repository's `openspec/` directory" while neither of its scenarios went near a repository; and "The configuration directory is not written to" asserted only the **listing**, where its sibling asserts listing, bytes, and mtimes — so an in-place rewrite of identical length would pass. | The `openspec/` sentence is dropped from the config requirement, which names no repository path at all, with the obligation delegated explicitly to `plugin-state`, where the fixture exists. The configuration-directory scenario now makes all three assertions. | specs/plugin-config; specs/plugin-state; design.md matrix; tasks.md 5.2 |
| WARNING | design.md, tasks.md | The matrix Command column used `\|` for both a shell pipe and a `grep -E` alternation. `grep -E 'a\|b'` matches the **literal** string `a|b` — verified to return exit 1 against a file containing `std::process::Command`, so `! grep …` succeeds and the check passes against a spawning implementation. | Every command containing a pipe or an alternation moved out of the table into labelled fenced blocks (`SPAWN`, `NOHERDR-RUN`, `TREE`, `METADATA`) above it, with the reason stated so it is not undone. The matrix cells name the blocks. | design.md → Test Strategy |
| SUGGESTION | specs/plugin-config, specs/plugin-state | The specs said an environment variable is authoritative "when non-empty" while tasks 2.3 and 4.3 implemented "empty or whitespace-only", leaving `HERDR_PLUGIN_CONFIG_DIR="  "` undecided in the one function everything else depends on. | Both specs now say "neither empty nor whitespace-only", and the scenarios test `"   "` alongside `""`. | specs/plugin-config; specs/plugin-state |
| SUGGESTION | specs/plugin-config | `XDG_CONFIG_HOME` is deliberately ignored while `XDG_STATE_HOME` is honoured; the asymmetry was argued at length in design.md and pinned by nothing, so the first implementer to notice it would "fix" it with no test going red. | New negative scenario asserting the `HOME` path wins over `/xdg`, and asserting in the same test that `state_dir` on the same lookup *does* honour `XDG_STATE_HOME` — so the asymmetry is the thing under test. | specs/plugin-config; design.md matrix; tasks.md 2.1 |
| SUGGESTION | specs/plugin-state, design.md | "Two changes sharing a 27-character prefix **do not collide**" is a probability stated as a property. | Reworded to "overwhelmingly unlikely", with the collision handling pointed at the recording rule that now specifies it. | specs/plugin-state; design.md → Decisions |
| SUGGESTION | specs/plugin-build | The Migration note said the replacement "keeps both binary-target scenarios verbatim"; there was one binary-target scenario, and the second, dropped one carried a `cargo build --release --offline` assertion whose retirement went unmentioned. | Migration rewritten to say what is kept, what replaces the no-dependency scenario, and that `--offline` is retired because a registry dependency makes it report the local cargo cache rather than the crate. | specs/plugin-build |
| SUGGESTION | design.md → Decisions | The `toml` paragraph justified `default-features = false` as "what is compiled is what was argued" without noting that the four features named **are** the crate's defaults, so nothing is pruned today — inviting a later "optimisation" on the false belief that they were. | One clause added stating it, and reframing the explicit list as a policy that makes a future default change a reviewable diff. | design.md → Decisions; specs/plugin-build requirement text |
| SUGGESTION | design.md → Test Strategy | The tier prose said "45 of the 48" and described the command tier as "the three dependency-set scenarios", miscounting and miscategorising the spawn check — the change's most important claim. | Recounted after the repairs: 57 scenarios, 47 unit, 5 command check, 5 existing test. The spawn check is named separately. Reconciled mechanically: 57 scenarios, 57 matrix rows, one-to-one by name, no duplicates, no orphans. | design.md → Test Strategy |
| SUGGESTION | design.md, tasks.md | `record(None, "x", "x")` was ambiguous: the `agent == change` short-circuit and the "no directory is an error" rule disagreed, and the matrix row named no arguments — so a test written with equal names would assert the wrong thing. `agent-launch` calls `record` on every launch, including that common case. | The order is now part of the contract: the short-circuit runs first, so `record(None, "x", "x")` is `Ok`. Stated in the requirement, the scenario, design.md → Contracts, the matrix row (with distinct arguments), and task 5.6. | specs/plugin-state; design.md → Contracts and matrix; tasks.md 5.6 |
| SUGGESTION | tasks.md | Task 2.5 carried a `RED then GREEN` marker in one checkbox, and sat after two GREEN tasks — so nothing could record that the test had ever been red. | Split into 2.5 RED and 2.6 GREEN, with the tautological form the old task described explicitly forbidden. | tasks.md 2.5, 2.6 |
| SUGGESTION | tasks.md | The leak check searched `std::env::temp_dir()` for a "`herdr-openspec` temporary file", but the atomic-write temp file lives inside the *state* directory and the scratch **directories** — the leak design.md actually admits to — had no specified name to match. The one check written for the one admitted leak could not see it. | Task 1.5 now specifies the name shape `herdr-openspec-test-<pid>-<counter>`, and 6.5 searches for that prefix and says what finding one means. | tasks.md 1.5, 6.5; design.md → Test Boundaries |
| SUGGESTION | AGENTS.md → Current repo state | Still says only `repo-foundation` has landed; `ci-pipeline` has since. Both prior changes scheduled an explicit refresh of this paragraph and `plugin-config` scheduled none. | Added task 8.11 as a rewrite, bringing the first sentence current and adding what the crate now does. | tasks.md 8.11 |
| SUGGESTION | SPEC.md → Architecture | "The untestable residue is two thin wrappers and `main`" is now an incomplete enumeration — this change adds `config::env_lookup`, a third one-line binding to the real world (which, after the CRITICAL repair above, is tested). No task reached that sentence. | Folded into task 8.4: inspect it in the same pass as the module map and either amend the enumeration or record that no change is needed because the binding has assertions of its own. | tasks.md 8.4 |

Confirmed clean, with no repair needed: proposal ↔ delta-file agreement (three
capabilities named, three delta files present before the `ci-workflow` addition, names
matching the live tree); the `plugin-build` REMOVED header, byte-compared against
`openspec/specs/plugin-build/spec.md:51` and identical; the `ci-workflow` MODIFIED
requirement, `diff`ed against the live text with the rationale clause as the only
difference; every requirement carrying at least one scenario and every scenario at
exactly four hashes; the agent-name algorithm itself, hand-traced by one reviewer and
independently recomputed by this session on all twelve named inputs, with step ordering
verified sound (the `c-` prefix cannot push a name past 32, the trailing trim cannot
empty the prefix, and `change` is never re-prefixed); the Rust API's implementability
under edition 2024 and MSRV 1.85, compiled and run by a reviewer — `&dyn Fn` threading,
`toml::Table`'s `serde` gating, its `Display` emitting valid sorted deterministic TOML
that quotes keys needing it, `fs::rename`'s semantics, and 27 + `-` + 4 fitting exactly
32; the five `rules.design` requirements, satisfied in substance rather than mentioned;
the decision to take no outer-loop acceptance group and its stated reason; the absence of
any RED task for plumbing; the contract gates (3.7, 5.9) and the persistence gate (5.8);
the required final validation sequence with `make check` as the single gate; portability
of every remaining command on BSD userland, with mtimes compared in Rust rather than
through `stat`; and every repository invariant — nothing writes inside `openspec/`,
nothing spawns outside `cli` (and no `cli` is created early), the coverage floor is
neither lowered nor narrowed, no task edits the graft-vendored `openspec/schemas/tdd/`
or `.claude/agents/`, no PRD non-goal is crossed, and the change is correctly **not**
marked BREAKING because it introduces the config format rather than altering one.

After the repairs the verification matrix carries 57 rows against 57 spec scenarios,
matched one-to-one by name with no duplicates and no orphans in either direction, and
every scenario has at least one owning task.

## No Remaining Implementation-Blocking Gaps

None remain. All five CRITICALs and all twenty-six WARNINGs are repaired in the artifact
that owns each; fifteen of seventeen SUGGESTIONs are applied and the remaining two are
recorded below. Two findings were resolved by gathering new evidence rather than by
rewording — the action-process probe and the `mod 36⁴` reduction — and both results are
recorded in design.md → Context and Decisions. Every repaired shell command was executed
against this machine before being written down: the `NOHERDR` `PATH` construction, the
two `grep` patterns (against the real `src/` and against a simulated doc comment), the
`cargo tree` extraction, and the `DEPS` filter both failing correctly against this
repository today and passing against a probe crate. `openspec validate plugin-config
--strict` reports the change valid. No unresolved decision requires user input.

## Apply-Time Live Verification (task 6.1)

Re-confirmed empirically during implementation, independent of the planning-time probe
recorded above, against `herdr 0.8.2` (`/opt/homebrew/bin/herdr`), with a running Herdr
server:

Two throwaway plugins, ids `envprobe-pc` and `envprobe-pc2` — never `herdr-openspec` —
linked from a scratch directory outside this repository:

| Probe | Entrypoint | `HERDR_PLUGIN_CONFIG_DIR` | `HERDR_PLUGIN_STATE_DIR` |
|---|---|---|---|
| `envprobe-pc` | `[[panes]]`, opened with `herdr plugin pane open --plugin envprobe-pc --entrypoint dump --placement split --direction down --no-focus` | `/Users/juusopiikkila/.config/herdr/plugins/config/envprobe-pc` | `/Users/juusopiikkila/.local/state/herdr/plugins/envprobe-pc` |
| `envprobe-pc2` | `[[actions]]`, invoked with `herdr plugin action invoke dump --plugin envprobe-pc2` | `/Users/juusopiikkila/.config/herdr/plugins/config/envprobe-pc2` | `/Users/juusopiikkila/.local/state/herdr/plugins/envprobe-pc2` |

Both `HERDR_PLUGIN_CONFIG_DIR` values are character-identical to
`herdr plugin config-dir <id>`'s own output, confirmed separately for both ids. The pane
process additionally carried `HERDR_PLUGIN_ENTRYPOINT_ID=dump`; the action process
carried `HERDR_PLUGIN_ACTION_ID=dump` instead. Both processes also carried `HERDR_ENV=1`
and `HERDR_PLUGIN_ROOT`. Both plugins were unlinked with `herdr plugin unlink`, and the
configuration and state directories Herdr created for them were removed by the exact
paths above — never by a glob, and never for `herdr-openspec`. This re-confirms the
planning-time probe rather than replacing it: the design's central premise held at
implementation time on the same Herdr version.

## Change Review (task 7.1) Findings and Repairs

An independent `outside-in-tdd-reviewer` (not a fork of the implementing session)
reviewed the full diff (`git diff 8ccb37e..1073b87`) against all planning artifacts.
Findings and their disposition:

| Severity | Location | Problem | Repair |
|---|---|---|---|
| CRITICAL | `src/lib.rs`'s `pid()` doc comment | The doc comment explaining why `crate::pid()` exists quoted the module-scoped spawn-check regex verbatim, including the literal substring `Stdio` — which made design.md's **tree-wide** `grep -rnE 'process::Command\|Command::new\|Stdio' src/` gate fail against the very comment written to satisfy the module-scoped gate. | Reworded the comment to describe the check in prose without reproducing its literal patterns, so it cannot trip either half of the SPAWN check. Re-ran all four SPAWN lines: clean (module-scoped and tree-wide `Command`/`Stdio`/`process::Command` checks pass; the two `"herdr"`/`"openspec"` literal checks still show the known false positives below). |
| WARNING | `src/config.rs`, `neither_variable_is_available` | Asserted only that `config_dir` returns `None`; the scenario's second THEN clause — `load(None, …)` equals `Config::default()` with empty `problems` — was unasserted, so `load`'s `dir: None` branch had no dedicated test. | Added the missing assertion. |
| WARNING | `src/state.rs`, `read` | A non-`NotFound` I/O error (e.g. `agent-names.toml` existing as a directory) was silently treated the same as an absent file — zero problems — unlike `config::load`'s identical case, and unlike design.md → Contracts' general statement that `state::read` is "infallible by construction: every failure becomes a default plus a string in problems." No spec scenario named this case explicitly (unlike `plugin-config`'s "The file cannot be read" scenario, `plugin-state`'s spec has no analogous scenario for `agent-names.toml`), but the general contract text applies to both modules. | Mirrored `config::load`'s `NotFound`-vs-other-error split; a non-`NotFound` read failure now yields an empty mapping plus one problem naming `agent-names.toml`. Added test `the_mapping_file_cannot_be_read`. |
| WARNING | `src/state.rs`, `record` | `record` reconstructs the file from `read`'s already-filtered `Mapping`, so any entry `read` would have skipped (bad type, illegal agent name) is silently dropped from the file on the next successful `record` call, with no problem surfaced (`record` returns `io::Result<()>`, not a `Mapping`). No scenario requires preserving invalid entries. | **Accepted as intentional**, one-line reason: an entry `read` already treats as unusable is one Herdr would reject if handed to it, so pruning it on the next rewrite is self-healing rather than data loss — nothing valid is lost, and no spec scenario asks for the alternative (preserving known-bad bytes verbatim across a write it did not need to touch). Not changed. |
| WARNING | This file | The `crate::pid()` fix and the SPAWN-block imprecision below were recorded only in a commit message, not here, so the archive would not carry them. | This section and the one below. |

## Design.md Imprecision Found During Implementation (non-blocking)

design.md → Test Strategy's `SPAWN` block includes `! grep -rn '"herdr"' src/` and
`! grep -rn '"openspec"' src/`, intended to catch a program-name literal passed to a
spawn API. This change's own legitimate code trips both: `.join("herdr")` appears in
`config::config_dir`'s and `state::state_dir`'s fallback-path construction
(`src/config.rs:59`, `src/state.rs:34,44`), and a test fixture builds a path through
`.join("openspec")` (`src/state.rs`, the repository-containment test) — both are quoted
string literals used as **path segments**, not program names, so the literal grep
false-positives against code the spec itself requires.

The actual invariant — no `Command::new`/`Stdio`/`process::Command` anywhere in `src/`
— is independently and correctly enforced by the SPAWN block's other two lines, which
stay clean. The reviewer's assessment, confirmed here: a program-name literal can only
reach a spawn through `Command::new` or `Stdio`, both of which the tree-wide line already
forbids, so the `"herdr"`/`"openspec"` lines are strictly redundant with the first line
for this codebase and are better read as documentation of intent than as an independently
load-bearing check. Not fixed here — design.md is this change's own settled planning
artifact, not something to relitigate to squeeze out a rewording; recorded so
`subprocess-seam`, which introduces the crate's first real `Command::new`, does not
inherit a check believed clean that is actually failing on false positives it never
looked past.

## Deferred Non-Blocking Notes

- **`cargo build --locked` is not a recurring gate.** The `quality-gates` capability
  pins `make test` to exactly `cargo test --all-features`, so adding `--locked` would
  require a second delta into a capability this change does not otherwise touch, for a
  property that is genuinely verified once here. The requirement is worded to claim only
  what is enforced, and the gap is recorded in design.md → Risks. If lock drift ever
  bites, adding `--locked` to `make test` is one Makefile line plus a `quality-gates`
  delta, and `ci-workflow`'s parity guard is already satisfied because CI runs
  `make test`.
- **The `XDG_CONFIG_HOME` asymmetry is verified on macOS only.** `herdr-plugin.toml`
  declares `platforms = ["macos", "linux"]`, and the config fallback deliberately ignores
  `XDG_CONFIG_HOME` while the state fallback honours `XDG_STATE_HOME`, copied from
  `herdr-navigator`. It is reachable only outside a Herdr-started process, where
  `config.toml` will not usually exist, and it is now pinned by a negative scenario so it
  cannot drift silently. Resolution point: design.md → Risks records that honouring
  `XDG_CONFIG_HOME` is a one-line change and names the scenario to invert.
- **Two SUGGESTIONs not applied.** (1) Adding a second illegal-key case to the bad-entry
  fixture beyond the two now present — the fixture already carries a legal bare key that
  fails the pattern and a legal quoted key that fails it, which is both shapes; a third
  would restate one. (2) Having `record` expose its temporary filename so a test can
  assert the path is a sibling of the target — rejected as widening the public surface
  for a property the hard-link witness already establishes indirectly; the requirement
  states it and 5.7 implements it.
- **`SPEC.md` → Launch flow's `herdr agent start <change>` / `agent prompt <change>`
  lines** now mean the derived agent name rather than the raw change name. Task 8.6
  records that explicitly and hands the correction of those two lines to `agent-launch`,
  which owns the flow, rather than editing a section this change does not implement.
