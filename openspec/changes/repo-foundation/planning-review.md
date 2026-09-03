## Reviewed Artifacts

- `proposal.md`
- `specs/plugin-build/spec.md`
- `specs/quality-gates/spec.md`
- `specs/plugin-manifest/spec.md`
- `design.md`
- `tasks.md`

The finding pass was delegated to three independent reviewers, none of which wrote the
planning package and none of which was a fork of the planning session. Each received the
change directory and one slice of the review list: capability and scenario coverage;
design completeness, test boundaries, and test strategy; task alignment, contradictions,
and repository invariants. The reviewers edited nothing; every repair below was made in
this session, in the artifact that owns it.

## Reviewed Against

- This repository HEAD: `5c2b6a0aadb6ca199f9a271272ea796a9c627d9a`
  (`docs(openspec): fill in project context, artifact rules, and operation guidance`)
- Sibling repository HEAD: Not applicable. `herdr` and `openspec` are consumed as
  installed binaries, not as source siblings.
- Working tree: clean apart from this change's own untracked planning directory,
  `openspec/changes/repo-foundation/`. No source file exists to be dirty — the
  repository has no `Cargo.toml` at review time.
- Environment facts confirmed during review: `openspec/specs/` is empty, `.gitignore`
  contains `/target`, no `LICENSE` file exists, `README.md` → Development already
  documents `make check` and both one-time installs, and `openspec validate
  repo-foundation --strict` reports valid.

**Refreshed during apply**, at commit `142eff7` (`build(make): add the four quality
gates behind make check`) and again while still inside group 5: implementation reached
the manifest group with Herdr 0.7.0+ (0.8.2, installed) genuinely available, and found
two live discrepancies neither this review nor any of its three reviewers could have
caught without a real `herdr plugin link .` run — see the two repair-log rows below.
`SPEC.md`, `design.md`, `specs/plugin-manifest/spec.md`, `tasks.md`, and `proposal.md`
were corrected in place per the drift protocol before task 5.2 (first finding) and
before task 5.5 (second finding) were marked done. No other planned contract moved
during implementation; groups 1 through 4 landed exactly as reviewed.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | SPEC.md, discovered during apply (group 5) | `SPEC.md` → Build and distribution claimed `herdr plugin link .` "builds from the working tree for local development," and `specs/plugin-manifest/spec.md`'s "Herdr links the working tree" scenario asserted link "runs the `[[build]]` step." Herdr's own docs (plugins.mdx → Build commands) state build commands run only during a GitHub-managed `plugin install`, and that `plugin link` explicitly does not run them; confirmed empirically — `target/` stayed absent after `herdr plugin link .` with `target/` removed beforehand, and `herdr plugin log list` showed no build entry for the plugin. | Corrected `SPEC.md` → Build and distribution to state `plugin link` does not run build commands and the local author builds the tree themselves; corrected the "Herdr links the working tree" scenario to assert only `exit 0` plus the prerequisite that the tree is already built; updated task 5.5 to build before linking. | SPEC.md → Build and distribution; design.md → Persistence and Rollout; specs/plugin-manifest/spec.md; tasks.md 5.5 |
| CRITICAL | SPEC.md, discovered during apply (group 5) | `SPEC.md` → Herdr integration → Manifest documented `id`, `name`, `min_herdr_version`, and `platforms` as the manifest's identifying keys, omitting `version`, which Herdr 0.7.0+ requires at the top level (`herdr plugin link .` on the installed 0.8.2 rejected the manifest with `missing field 'version'`). None of the three independent planning reviewers caught this because it required a live `herdr plugin link .` run against the real binary, which happens only in task 5.5/5.1, after the manifest is written — the planning review had no manifest to link against. | Added `version = "0.1.0"` (kept equal to `Cargo.toml`'s version by convention, not by an enforced link) to `SPEC.md`'s manifest example, this change's `herdr-plugin.toml`, and updated the required-key count from six to seven everywhere it was cited. `herdr plugin link .` then succeeded. | SPEC.md → Herdr integration → Manifest; design.md → Test Strategy matrix; specs/plugin-manifest/spec.md; tasks.md 5.2, 5.4; proposal.md → What Changes |
| CRITICAL | design.md | The `~/.cargo/env` fallback check used `paste -sd:` with no file operand. BSD `paste` prints usage and yields an empty string, so the script would run with an empty `PATH`, source `~/.cargo/env`, and exit 0 — a false pass on a different scenario. | Replaced with `tr '\n' ':'`, and the command copied into the task so the implementer does not reach for the broken one. | design.md → Test Strategy matrix; tasks.md 3.6 |
| CRITICAL | design.md, specs/plugin-build | Even with `paste` fixed, the check asserted only "exit 0" and could pass without the fallback branch ever running, if a Homebrew, asdf, or mise cargo satisfied the first probe. | The script now announces on stderr when it sources `~/.cargo/env`; the check first asserts `command -v cargo` fails on the stripped `PATH`, then greps for that notice. | specs/plugin-build → requirement text and two scenarios; design.md matrix; tasks.md 3.2, 3.5, 3.6 |
| WARNING | specs/plugin-build | The load-bearing "holds the pane open" clause had no scenario that could fail on it — the only automated scenario ran with stdin already at EOF, so it passed identically with the blocking deleted. | Added scenario "`ui` holds the process open until stdin closes", spawning with a piped stdin left open. | specs/plugin-build; design.md matrix; tasks.md 2.2 |
| WARNING | specs/quality-gates | "The threshold argument is present and unmodified" asserted the flag string back at itself — the exact shape the schema forbids — and neither coverage scenario proved the floor was enforced. | Replaced with "The floor actually fails a build below it", running the same command with `--fail-under-lines 100`. The no-exclusion-flag inspection stays as a contract gate in the task, not as a scenario. | specs/quality-gates; design.md matrix; tasks.md 4.7 |
| WARNING | specs/quality-gates | The `rustfmt.toml` requirement's only scenario was `cargo fmt --check`, which passes with the file deleted, because cargo passes the edition itself. | Added scenario "A bare `rustfmt` uses the configured edition", running `rustfmt --check` with no `--edition`. | specs/quality-gates; design.md matrix; tasks.md 4.8 |
| WARNING | specs/quality-gates, design.md | Both missing-tool checks ran `env PATH=/usr/bin:/bin`, which hides `cargo` itself, so they could not distinguish "llvm-cov absent" from "cargo absent" — the realistic case the requirement is written for. | Scenarios reworded to "a `PATH` on which `cargo` still resolves but the tool does not"; the matrix and task now build a scratch bin directory symlinking `cargo` only. | specs/quality-gates; design.md matrix and Test Boundaries; tasks.md 4.11 |
| WARNING | specs/plugin-build, design.md | The bashism search was anchored (`^function `, `^source `) so an indented `source ~/.cargo/env` — which the script necessarily has, inside an `if` — sailed through, and arrays were named but unchecked. `sh -n` also proves little on macOS, where `/bin/sh` is bash in POSIX mode. | Unanchored pattern covering `[[`, `function`, `source`, and array assignment, plus `dash -n` where `dash` exists. | specs/plugin-build; design.md matrix; tasks.md 3.4 |
| WARNING | design.md | `grep -c '"herdr-openspec"'` over one-line `cargo metadata` JSON counts matching lines, not occurrences, and filtered neither target `kind` nor the package name — it returned 1 whether there were zero, one, or two bin targets. | Replaced with a `python3` filter over targets whose `kind` contains `bin`, asserting exactly one named `herdr-openspec`. | design.md matrix; tasks.md 1.6 |
| WARNING | design.md | `cargo build --release --offline` was cited as evidence of no third-party dependencies, but a warm registry lets a dependent build succeed offline — it cannot detect the failure it claims to. | Primary evidence is now a single `[[package]]` entry in `Cargo.lock` plus an empty dependency list from `cargo metadata`; `--offline` is kept as a secondary signal. | specs/plugin-build; design.md matrix; tasks.md 1.7 |
| WARNING | design.md, tasks.md | The two gate-failure checks damaged a tracked source file and restored it with `git checkout --`, which would destroy the group-2 implementation if that group were not yet committed. `git` was also a collaborator no Test Boundaries row named. | Switched to copy-aside-and-restore, and added an explicit Test Boundaries row saying `git` is deliberately not used as the restore mechanism. | design.md → Test Boundaries and matrix; tasks.md 4.9, 4.10 |
| WARNING | design.md | The manifest checks depend on `python3` with `tomllib`, which needs Python 3.11+; stock macOS Command Line Tools ships 3.9. No row, no fallback. | Added a Test Boundaries row with the fallback (`herdr plugin link .` plus a line-oriented read), and the task now records which path was used. | design.md → Test Boundaries; tasks.md 5.4 |
| WARNING | tasks.md, design.md | `rust-version = "1.85"` appeared only in tasks.md, with no basis in the proposal or design, and appeared to contradict `AGENTS.md` → Environment ("Rust stable, 1.91+"). | design.md → Decisions now argues the number: 1.85 is the *edition floor* for edition 2024, deliberately below the toolchain actually in use, and unverified until CI adds an MSRV job. | design.md → Decisions; tasks.md 1.2 |
| WARNING | tasks.md | The Documentation group inverted the operational lifecycle: its only CHECK came after the task that rewrote `AGENTS.md`. | Renumbered to CHECK (6.1) → CHANGE (6.2, 6.3) → VERIFY (6.4). | tasks.md group 6 |
| WARNING | tasks.md, proposal.md | `README.md` → Install tells the reader to open the dashboard from Herdr's action menu, which the `plugin-manifest` spec asserts will not exist after this change. The plan cleared README on its Development section alone. | Added task 6.3 to qualify that instruction, and listed `README.md` under proposal Impact → Modified. | tasks.md 6.1, 6.3; proposal.md → Impact |
| SUGGESTION | design.md, specs/plugin-build | `ui` with a trailing argument was unspecified, yet design.md leaned on "the exit-2 default" to make `plugin-actions` safe. | Requirement extended to reject any trailing argument, with scenario "Extra arguments after `ui`". | specs/plugin-build; design.md → Contracts and matrix; tasks.md 2.1, 2.2; proposal.md |
| SUGGESTION | specs/plugin-manifest | `name` was left as "a human `name`" — unfalsifiable — while tasks.md and SPEC.md both fix `name = "OpenSpec"`. | Pinned in the requirement, and the parse check now asserts all six required keys carry exactly the specified values. | specs/plugin-manifest; design.md matrix; tasks.md 5.4 |
| SUGGESTION | specs/plugin-build, specs/plugin-manifest | Two capabilities owned one check: a plugin-build scenario reached into `herdr-plugin.toml` to assert what a plugin-manifest scenario already asserted. | plugin-build's scenario narrowed to the Cargo side; the cross-file agreement check lives only in plugin-manifest, whose requirement was also retitled to match what it actually asserts. | specs/plugin-build; specs/plugin-manifest |
| SUGGESTION | specs/quality-gates, tasks.md | The `fmt` and `build` targets were required by a SHALL that no scenario or task exercised. | Folded into the "All gates pass on a clean tree" scenario and task 4.5. | specs/quality-gates; tasks.md 4.5 |
| SUGGESTION | design.md | The design did not answer the config.yaml rule about the `Change` type, did not mention the `AGENTS.md` edit in Boundaries or Rollout, and pre-decided whether `plugin-actions` should mark its manifest extension **BREAKING**. | All three fixed: an explicit "the `Change` type is neither introduced nor altered here", a Boundaries row and a Rollout bullet for the documentation edits, and the BREAKING determination handed back to `plugin-actions`. | design.md → Boundaries, Contracts, Persistence and Rollout |
| SUGGESTION | design.md, tasks.md | `license = "MIT"` was scheduled with no `LICENSE` file in the repository, leaving a dangling declaration. | Dropped the key, with the reason recorded: the crate is never published to crates.io, and adding a `LICENSE` is not this roadmap row's work. | design.md → Decisions; tasks.md 1.2 |
| SUGGESTION | tasks.md | Several small gaps: the placeholder files in 1.4 could leave cargo's generated `Hello, world!` as untested production code; `Stdio` handling was unspecified, and a piped stdin with `child.wait()` hangs; `make coverage` omits `--all-features` while `make test` has it; the `herdr`-absent check named no technique; and `openspec validate` fails on this machine unless nvm is ahead on `PATH`. | Each written into the owning task or decision. | tasks.md 1.4, 2.2, 2.5, 2.7, 4.3, 4.12, 8.8; design.md → Decisions |
| SUGGESTION | design.md | Three matrix commands omitted an assertion their own Verification cell promised (mtime capture, output grep, per-path resolution), and `stat` differs between BSD and GNU. | Commands completed, with a portable `mtime` helper. | design.md matrix; tasks.md 3.7 |

Confirmed clean, with no repair needed: capability coverage (three capabilities, three
delta specs, names consistent with the empty `openspec/specs/` tree); the proposal rules
(problem before detail, explicit Non-Goals, PRD non-goals answered, roadmap row named and
matching the IMPLEMENTATION-ORDER dependency graph exactly, **BREAKING** correctly not
claimed for a manifest that is being introduced); the "tests invented for plumbing" check
in both directions; the decision to take no outer-loop acceptance group, and its stated
reason; and every repository invariant — nothing writes inside `openspec/`, nothing
spawns a process outside `cli` in production code, the coverage gate is neither lowered
nor narrowed, no task edits the graft-vendored `openspec/schemas/tdd/` or
`.claude/agents/`, and `platforms` matches the Windows non-goal.

After the repairs the verification matrix carries 26 rows against 26 spec scenarios,
matched one-to-one by name, and every scenario has at least one owning task.

## No Remaining Implementation-Blocking Gaps

None remain. Both CRITICAL findings and all twelve WARNINGs are repaired in the artifact
that owns them; the accepted SUGGESTIONs are applied and the deferred ones are recorded
below. `openspec validate repo-foundation --strict` reports the change valid. No
unresolved decision requires user input.

## Deferred Non-Blocking Notes

- **`~/.cargo/env` present but unusable** (readable yet failing to define `cargo`) is not
  covered by a scenario. The `set -eu` interaction that makes it interesting is already
  recorded as a risk in design.md → Risks, and the cargo-absent scenario exercises the
  same error path; adding a third branch would specify a shape of `~/.cargo/env` that
  rustup does not produce.
- **Shell linting in CI.** `sh -n` and the bashism search run as verification tasks in
  group 3 rather than as a fifth `make check` gate, because `PRD.md`, `SPEC.md`, and
  `AGENTS.md` all name exactly four gates. A `shellcheck` step belongs to `ci-pipeline`,
  which owns the workflow file.
- **`make coverage` without `--all-features`.** Kept verbatim from `SPEC.md` → Testing
  and quality gates. Harmless until the crate has a feature; reconciling it is
  `ci-pipeline`'s, and the divergence is recorded in design.md → Decisions.
- **`IMPLEMENTATION-ORDER.md` → Phase 6** describes `plugin-actions` as delivering
  `min_herdr_version` and `platforms`, which `repo-foundation` in fact ships. Not this
  change's file to rewrite mid-flight; `openspec/config.yaml` → `operations.archive`
  already requires the roadmap row to be corrected at archive time.
- **An MSRV job.** `rust-version = "1.85"` is asserted but never built against. If that
  floor is to mean anything it needs a job on that toolchain, which is `ci-pipeline`'s
  decision to take or decline.

## Change Review (group 7)

An independent reviewer — not a fork of the implementing session — reviewed the
completed diff against all planning artifacts. Findings: 0 CRITICAL, 3 WARNING,
5 SUGGESTION. All three WARNINGs were fixed (no WARNING was accepted unfixed):

| Severity | Problem | Repair | Updated Location |
|---|---|---|---|
| WARNING | `AGENTS.md` → Development still said `herdr plugin link .` "build[s] and load[s] the working tree," which group 5's live discovery (`plugin link` does not build) had already falsified in every other document except this one. | Comment corrected to "does not build"; a `make build` step inserted between `link` and `make check`, mirroring the `README.md` repair. | AGENTS.md → Development |
| WARNING | The "A bare `rustfmt` uses the configured edition" check ran against `src/lib.rs`, which formats identically under edition 2015 and 2024 — the check passed whether or not `rustfmt.toml` existed and proved nothing. `src/main.rs`'s multi-item `use` statement is edition-sensitive and genuinely discriminates (confirmed: exits 1 with `rustfmt.toml` moved aside, 0 with it restored). | Scenario and task retargeted to `src/main.rs`, with the reasoning recorded in both. | specs/quality-gates/spec.md; tasks.md 4.8 |
| WARNING | `tests/cli.rs::extra_arguments_after_ui` asserted "does not block waiting on stdin" while running with `Stdio::null()`, which reaches EOF instantly regardless of whether the implementation would have blocked — the clause was unverified. | Test rewritten to leave stdin's write end open (`Stdio::piped()`) and assert `try_wait()` returns `Some` after a short interval, so a wrongly-blocking implementation would now fail the test. | tests/cli.rs |

All affected checks re-run after the fixes: `cargo test --all-features` green (11
tests), `cargo fmt --all -- --check` clean, `make check` exits 0 with coverage
unchanged at 100.00%, and `openspec validate repo-foundation --strict` valid.

SUGGESTIONs noted, not implemented (schema calls for noting, not fixing):
tightening `ui_prints_placeholder_banner` to also assert the "not implemented"
line at the binary-integration tier (already covered at the unit tier); probing
`command -v cargo` before the tool-specific `Makefile` guards for a more precise
message when `cargo` itself is absent; a `.NOTPARALLEL:` guard against `make -j
check` running gates out of order; listing `SPEC.md` under `proposal.md` →
Impact → Modified, alongside `AGENTS.md` and `README.md`; and a pre-apply
"six required keys" reference at planning-review.md line 62 that the apply-time
repair row did not retroactively edit (left as an accurate record of what the
original review saw).
