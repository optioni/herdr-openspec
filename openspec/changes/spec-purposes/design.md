## Context

Three defects, one cause.

`openspec validate --specs --strict` exits 1 at HEAD: **26 of 40** capabilities fail, every
one on the identical warning — `overview: Purpose section is still a placeholder rather than a
Purpose anyone wrote`. `openspec archive` writes `TBD - created by archiving change <name>`
when it creates a capability, and nothing has ever replaced it. The remaining 14 already carry
a written Purpose and are **not** touched here.

`DEPS` and `GRAPH-SNAP` have been red on `main` since `live-refresh`, three changes ago.
Neither is a file in this repository: both exist only as fenced shell blocks inside archived
`tasks.md` documents, extracted by hand into a scratch directory at the start of every change
and run outside `make check`. Nothing forces them to run, so nothing noticed.

Measured at planning time, on `main` at `f6b4c3f`, from the scripts extracted into
`openspec/changes/spec-purposes/notes/extracted-gates/`:

| Gate | Exit | Verbatim failure |
|---|---|---|
| `DEPS` (`WORK=<scratch> DEPS_SKIP_LEG5=1`) | 1 | `AssertionError: normal deps are ['notify', 'pulldown-cmark', 'ratatui', 'serde_json', 'toml', 'yaml-rust2'], expected ['pulldown-cmark', 'ratatui', 'serde_json', 'toml', 'yaml-rust2']` then `DEPS FAIL: leg 2a` |
| `GRAPH-SNAP` | 1 | `GRAPH-SNAP FAIL: macOS/Linux differ by [fsevent-sys inotify inotify-sys linux-raw-sys ], expected [linux-raw-sys ]` |
| `AGENTSEAM` (`MIN=23 ALLOWED='src/cli.rs src/agents.rs src/ui/mod.rs src/launch.rs src/open.rs'`) | **0** | `AGENTSEAM OK: 23 files searched (>= 23); no spawn and no view type in src/agents.rs; Herdr handle only in:…` |

**`AGENTSEAM` is green.** The brief carried it as red at `MIN=23` against a realized 22, and
that figure is real but stale: `22` is `plugin-actions`' **task 0.3 baseline**, measured before
that change's own files existed. At HEAD `find src tests -name '*.rs'` returns **28**, less the
five `ALLOWED` paths is **23**, and `MIN=23` passes. The defect is in the *recorded reasoning*,
not the floor — see Decisions → 6.

`GRAPH-SNAP`'s snapshot is likewise not the problem. `tests/fixtures/build-graph.txt` was
regenerated in `574b87d` and holds `notify v8.2.0`, `fsevent-sys`, `inotify`, `inotify-sys`;
the script passes its `diff -u` leg and fails four legs later on
`[ "$d" = "linux-raw-sys " ]`. Regenerating the snapshot changes nothing.

## Goals / Non-Goals

**Goals:**

- Every capability spec carries a Purpose derived from its own requirements.
- `DEPS`'s want-list is re-derived from `Cargo.toml` and reconciled against `plugin-build`'s
  argued dependency set, which already names `notify` correctly.
- `GRAPH-SNAP`'s platform assertion is direction-aware and portable.
- Both gates become checked-in files invoked by `make check`, and the Purpose rule becomes a
  test inside `cargo test`. **Nothing repaired here can rot outside a gate that always runs.**

**Non-Goals:**

- Bringing the other twenty-eight extracted gates into the repository. They are green, and
  moving thirty scripts is a change of its own. Recorded as a follow-up in `HANDOFF.md`.
- Deriving the seam gates' hand-maintained `MIN` floors instead of hardcoding them. Same
  reason — worth doing, not here.
- Regenerating `tests/fixtures/build-graph.txt`, adding or removing a dependency, or editing
  `Cargo.toml` outside a throwaway copy.
- Removing `src/changes.rs:966`'s vestigial `#[allow]`. This change does not touch that file.
- Any `src/` behaviour change. The plugin binary is byte-identical in function afterwards.

## Boundaries

| Piece | Where | Pattern it follows |
|---|---|---|
| `scripts/gates/deps.sh` | new | `scripts/build.sh` — a `/bin/sh` script invoked by one `Makefile` target and nowhere else |
| `scripts/gates/build-graph.sh` | new | same |
| `make gates`, `make gates-full` | `Makefile` | the existing `lint`/`coverage` pattern: the command is written once, in the Makefile |
| `Gates` step ×2, `gates-full` job | `.github/workflows/ci.yml` | the existing `Format`/`Lint`/`Test` steps — every CI step invokes `make <target>`, never a raw command, which `ci_workflow.rs` already enforces |
| `tests/spec_purposes.rs` | new | `tests/ci_workflow.rs` — an integration test that reads repository files relative to `CARGO_MANIFEST_DIR` and asserts a repository-level contract |
| `tests/ci_workflow.rs` | modified | its own existing `phony_targets` / `find_job` helpers; the Makefile↔CI contract widens from four gates to six |
| 26 × `openspec/specs/<cap>/spec.md` | modified | the `## Purpose` section every capability already has |

`src/` is not touched. No module gains a dependency, no seam moves, nothing new spawns a
process: the two scripts run `cargo`, and a `/bin/sh` script is not crate source, so
`NOSPAWN-GREP`'s subject is unchanged.

**The one thing that does move under `src`-adjacent gates:** `tests/spec_purposes.rs` is a
29th `.rs` file, so every gate whose searched set is `find src tests -name '*.rs'` gains one.
Floors, measured-plus-enumerated:

| Gate | Searched set | Measured at `f6b4c3f` | New | Floor |
|---|---|---|---|---|
| `AGENTSEAM` | `find src tests` less 5 `ALLOWED` | 23 | +1 | **`MIN=24`** |
| `LAUNCHSEAM` (both invocations) | same set, same `ALLOWED` | 23 | +1 | **`MIN=24`** |
| `NOSLEEP` | `find src tests` | 28 | +1 | **`MIN=29`**, and **`SLEEP_MIN=6`** — re-measured; it has been invoked at 5 against a realized 6 since `plugin-actions` |
| `WATCHSEAM` | `find src tests` less `src/watch.rs` | 27 | +1 | **`MIN=28`** |
| `NOSPAWN-GREP`, `NOLIT-CHANGE` | `find src` less `src/changes.rs` | 24 | 0 | `MIN=24`, unchanged — the new file is under `tests/` |
| `MDSEAM` | `find src` less `src/ui/markdown.rs` | 24 | 0 | `MIN=24`, unchanged |
| `GATE-MECH1` | `find src` | 25 | 0 | floor hardcoded at 8; no edit |
| `NOCLI-SHELL`, `NOBLOCK`, `READONLY-UI` | `find src/ui` | 11 | 0 | `UI_MIN=11`, unchanged |
| `READSEAM` | `find src/ui` | 10 | 0 | `UI_MIN=10`, unchanged |
| `WIDTHS` / `LISTWIDTHS` / `MDWIDTHS` / `TASKWIDTHS` / `DETAILWIDTHS` | `#[test]` counts under `src/ui/` | 94 / 29 / 24 / 16 / 23 | 0 | unchanged; **every one invoked with its floor named**, never bare |

Library test count is unchanged at **940**: the new file is an integration test, and
`cargo llvm-cov` measures `src/` lines, so the 97.05%-over-25,674-lines figure is unmoved by
construction. Task 8.6 re-measures rather than assuming it.

## Contracts

`make check`'s composition is the one interface a separate consumer depends on, and that
consumer is `.github/workflows/ci.yml` — mediated by `tests/ci_workflow.rs`, which fails when
the two drift. The change is **additive**: `fmt-check`, `lint`, `test`, `coverage` keep their
exact commands and relative order; `gates` is inserted third and `gates-full` is added
uncomposed. No existing invocation breaks. `make check` gains roughly 0.5 s locally.

`scripts/gates/*.sh` have no consumer but `make`. Their contract is: exit 0 with one `OK` line
naming what was proved, or exit non-zero with a message naming the failing leg.

**The `Change` type is not altered.** No field is added, removed, or retyped, so the
`from_files`/`from_cli` agreement question does not arise; neither producer is touched.

**`rules.specs`' six mandated boundary states are inapplicable and are declared so rather than
invented.** No `openspec` directory, no active changes, a missing artifact file, a schema the
CLI rejects, an unreachable Herdr socket, and a pane narrower than 100 columns are all states
of the *running plugin*. This change adds no runtime code path, renders nothing, and spawns
nothing, so each is unchanged from the capability that owns it. The analogous boundary states
that **do** apply here are covered: an empty capability set (`the test cannot pass vacuously`),
a dependency set with no entries (`deps.sh` leg 2a's vacuity leg), and a missing snapshot file.

**Not BREAKING.** No plugin manifest change, no config-format change, no keybinding change.
`herdr-plugin.toml` is untouched, so `tests/manifest.rs` is unaffected.

## Persistence and Rollout

- **Migration:** none. No data, no state file, no on-disk format.
- **Backfill:** none — though writing 26 Purposes is a backfill in spirit, it is a one-time
  content edit with no runtime consequence.
- **Seeding:** none. **Cache invalidation:** none. **Index rebuild:** none.
- **Authorization:** none. The workflow stays `permissions: contents: read` and uses no secret.
- **Observability:** each gate prints one `OK`/`FAIL` line naming its leg; the CI step names
  the gate, so a log identifies the failure without opening the script.
- **Deployment:** none. The shipped binary is unchanged; no release, no registry submission.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| The filesystem (`openspec/specs/`, `Makefile`, `.github/workflows/ci.yml`) | **real** — `tests/spec_purposes.rs` and `tests/ci_workflow.rs` read the actual repository tree via `CARGO_MANIFEST_DIR` | n/a; no `src/` unit test is added |
| `cargo metadata` / `cargo tree` | **real**, invoked by the gate scripts from `make gates`; offline-clean, resolving from `Cargo.lock` | n/a |
| `Cargo.toml` (as a gate subject) | **real** at HEAD; **a `cp -R` copy** in every planted-defect run, so the tree is never mutated | n/a |
| `tests/fixtures/build-graph.txt` | **real**, read-only; `GRAPH_WRITE` is never set by `make gates` | n/a |
| The `openspec` binary / `node` | **replaced by a Rust test.** `openspec validate --specs --strict` is run by hand at tasks 1.5 and 8.9 as the authority, but is never wired into `make check` — it is nvm-installed here and absent from both CI runners | n/a |
| The `herdr` binary / Herdr socket | **not touched.** `quality-gates`' "The gates do not depend on Herdr" requirement still holds; nothing in this change names `herdr`, and task 7.1 asks the reviewer to confirm it | n/a |
| The terminal | **not touched.** Nothing here renders | n/a |
| `python3` | **real**, invoked by `deps.sh` leg 2a to parse `cargo metadata`'s JSON; newly required by `make check` and recorded in `AGENTS.md` at task 6.2 | n/a |
| `git` | **real**, in `OPENSPEC-UNTOUCHED-SP` (both legs; leg 2 reads `git show $BASE:<path>`) and task 8.11's signing loop, against the `BASE` SHA re-derived in task 0.1 | n/a |
| GitHub Actions runners | **replaced** — CI is verified by reading `ci.yml` in `tests/ci_workflow.rs`, never by running a workflow. Unchanged from `ci-pipeline` | n/a |
| The release binary (`target/release/herdr-openspec`) | **real**, but only under `make gates-full` and `DEPS` leg 1b — deliberately outside `make gates` | n/a |

## Test Strategy

Three tiers apply. **Rust integration tests** (`cargo test --all-features`) cover everything
that can be decided by reading a repository file. **Gate scripts run against a planted
defect in a `cp -R` copy** cover the two shell gates, because a gate is only verified once it
has been seen to fail. **A hand-run command** covers the two facts no automated tier can
reach: the real `openspec validate --specs --strict`, and the Linux half of the platform
difference, which is only observable on a Linux host or in CI.

**This change takes no outer-loop acceptance test through `ui::run`.** The `live-refresh`
lesson — that unit tests can all pass over components nothing wires together — has a direct
analogue here and it is honoured differently: the wiring under test is
`Makefile` → `ci.yml`, and `tests/ci_workflow.rs` is exactly the outer-loop test for it. It
does **not** yet force it: as landed, `every_gate_the_makefile_composes_runs_in_ci` compares
against a hardcoded four-item list rather than parsing `check:`'s own prerequisites, so a fifth
gate joining `check` with no CI step leaves it green. Task 4.1 rewrites it to read `check:`'s
prerequisite list out of the `Makefile`, which is what makes the guarantee real — for this gate
and for the next one — and task 5.5 proves it by planting the omission.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| All gates pass on a clean tree | `make check` end to end at the final commit | hand-run | real cargo, real scripts | `make check` |
| Format gate fails and stops the run | misformat a copied `src/*.rs`, confirm `gates` never runs | planted defect | real cargo | task 5.0 |
| Lint gate fails on a clippy warning | unchanged from `ci-pipeline`; re-run to confirm still true | planted defect | real cargo | task 5.0 |
| The hygiene gates fail before the test and coverage runs | add a 7th dep to a copied `Cargo.toml`; assert `cargo test` produces no output in the run | planted defect | real cargo | task 5.7 |
| Both hygiene gates are checked-in files invoked from the Makefile | `test -f` both paths; `phony_targets` contains `gates`; grep the tree for a second definition | integration test | real filesystem | `cargo test --test ci_workflow -- makefile` |
| A gate that cannot fail is itself a failure | the full planted-defect matrix, task group 5 | planted defect | real cargo, copied tree | tasks 5.0–5.9 |
| A gate that cannot fail is itself a failure | vacuity leg: strip `[dependencies]` from a copied `Cargo.toml`, assert `deps.sh` fails rather than passing over an empty set | planted defect | real cargo, copied tree | task 5.2 |
| The declared set matches the argued set | `deps.sh` at HEAD | gate script | real `cargo metadata` | `make gates` |
| A dependency added without an argument fails the gate | add `once_cell` to a copied `Cargo.toml` | planted defect | real cargo | task 5.1 |
| Every dependency is proved load-bearing | six removal experiments, one per dependency | gate script (heavy) | real cargo, 6 builds | `make gates-full` |
| The graph matches the snapshot and the platform difference is exact | `build-graph.sh` at HEAD; assert `git diff --exit-code tests/fixtures/build-graph.txt` after | gate script | real `cargo tree`, real snapshot | `make gates` |
| A package moving between platforms fails the gate | swap the macOS-only and Linux-only lists in a copied script | planted defect | real cargo | task 5.3 |
| A dependency feature change that moves the graph fails the gate | set `notify` `features = ["macos_kqueue"]` in a copied tree, do not regenerate | planted defect | real cargo | task 5.4 |
| Every capability has a written Purpose | `tests/spec_purposes.rs` over all 40 directories | integration test | real filesystem | `cargo test --test spec_purposes` |
| Every capability has a written Purpose | the authority, run by hand | hand-run | real `openspec` CLI | `openspec validate --specs --strict` |
| A newly archived capability's placeholder fails the test | write the placeholder into a copied tree's spec, run the test with the root overridden | integration test | real filesystem | `cargo test --test spec_purposes -- placeholder` |
| The test cannot pass vacuously | point the test at an empty temp directory | integration test | real filesystem (temp) | `cargo test --test spec_purposes -- vacuous` |
| The matrix names both runners and no others | unchanged assertion in `ci_workflow.rs`; re-run | integration test | real `ci.yml` | `cargo test --test ci_workflow` |
| All four gates run on each runner | extend `every_gate_the_makefile_composes_runs_in_ci` and the step-order assertion to six targets | integration test | real `ci.yml`, real `Makefile` | `cargo test --test ci_workflow` |
| All four gates run on each runner | plant the omission: delete the `Gates` step from a copied `ci.yml`, assert the test fails | planted defect | real filesystem | task 5.5 |
| A failing gate fails the run rather than being skipped | `no_gate_step_can_be_skipped_or_ignored`, widened to the new step and job | integration test | real `ci.yml` | `cargo test --test ci_workflow` |
| One runner's failure does not cancel the other | unchanged assertion; re-run | integration test | real `ci.yml` | `cargo test --test ci_workflow` |
| The heavy legs run on every push and are required to pass | new assertions: the job exists, invokes `make gates-full`, has `timeout-minutes`, no `if:`, and is in the aggregate `needs` | integration test | real `ci.yml` | `cargo test --test ci_workflow` |
| The heavy legs are not duplicated into the per-platform job | assert no `check`-job step names `gates-full`, and `check:` in the Makefile does not list it | integration test | real `ci.yml`, real `Makefile` | `cargo test --test ci_workflow` |

## Decisions

**1. The gates become files in `scripts/gates/`, not prose in this change's `tasks.md`.**
This is the whole point. The alternative — leave them as extracted blocks and add a task to
every future change — was considered and rejected on evidence: `HANDOFF.md` recorded that
obligation three changes ago and it was not met, because nothing enforced it. A file plus a
`Makefile` line plus `tests/ci_workflow.rs` is enforcement.

**2. `gates` joins `check` third, between `lint` and `test`.** It costs ~0.5 s and needs no
compilation, so a stale want-list fails in two seconds instead of after the coverage run.
Alternative — append after `coverage` — keeps the existing diff smaller but makes the cheapest
failure the slowest to observe.

**3. `DEPS` splits: `gates` gets the cheap legs, `gates-full` gets the rebuilding ones.**
Leg 1b builds `target/release/herdr-openspec` and leg 5 rebuilds the crate once per dependency
— six builds. Putting either in `make check` makes the composite something a developer stops
running, which is the same rot in a different costume. But *outside* `make check` is where
these gates died, so `gates-full` is not left to a per-change task: it gets its own CI job on
`ubuntu-latest`, unconditional and in the aggregate `needs`, so every push runs it.
Alternative — a nightly schedule — was rejected: a failure discovered a day later is attached
to no pull request.

**4. `GRAPH-SNAP`'s platform assertion becomes two named, direction-aware lists.** The
realized difference is asymmetric — macOS-only `fsevent-sys`; Linux-only `inotify`,
`inotify-sys`, `linux-raw-sys` — and the current unordered single literal cannot express which
side a package sits on, so a package migrating between platforms would pass a merely-updated
literal. Alternative — derive the expected difference from the manifests instead of naming it
— is **not available**: `Cargo.toml` declares no `[target.*]` table at all; the entire split
lives inside `notify`'s and `rustix`'s own manifests, so there is nothing local to derive from.
Naming it is the honest option, and naming it per direction is what makes it discriminating.

**5. The Purpose rule is a Rust test, not `openspec validate --specs --strict` in `make
check`.** The validator is the authority and is run by hand at tasks 1.5 and 8.9. It is not
wired in, because it needs `node` and the `openspec` binary: nvm-installed here and absent
from both CI runners, and `quality-gates` already requires that the gates do not depend on
tooling the plugin treats as optional. A `make check` line that silently skips when a binary is
missing is not a gate. The Rust test guards the one rot mode that has actually occurred — the
archiver's placeholder — runs on every `cargo test`, and carries its own anti-vacuity leg.
Alternative — a CI-only step that installs node — was rejected: it would leave local
`make check` blind to exactly the regression this change repairs.

**6. `AGENTSEAM` is not repaired, because it is not broken; the *record* is.** Measured green
at `MIN=23`. `plugin-actions` justified that floor as "22 + `src/open.rs`", and that
justification is wrong twice in ways that cancel: `src/open.rs` was added to `ALLOWED` in the
same task, so it contributes **zero**; the real `+1` is `tests/manifest.rs`, which the same
change added and which nothing recorded. `22 + 0 + 1 = 23`. **This is neither a missing-test
case nor an over-set floor** — the floor is correct for a reason nobody wrote down, which is
its own hazard: the next person to re-derive it from the recorded arithmetic gets 23 by
accident. The correction is written into `HANDOFF.md` at tasks 6.3 and 6.4 and the floor moves to
`MIN=24` here only because this change adds `tests/spec_purposes.rs`.

**7. `OPENSPEC-UNTOUCHED`'s carve-out is enumerated, and paired with a second leg.**
As normally written the gate fails by construction here: it excludes exactly one path — the
change's own artifact directory — and this change deliberately writes 26 files under
`openspec/specs/`. A `openspec/specs/` **prefix** exclusion is refused: it would blind the gate
to a runtime write anywhere under a tree the dashboard reads, which is the one thing the gate
exists to catch. Instead the invocation excludes `openspec/changes/spec-purposes/` **plus the
26 spec paths by name**, and a second leg asserts that each of the 26 diffs against `BASE`
touches the Purpose section only — no `### Requirement:` or `#### Scenario:` line added,
removed, or changed. A 27th path under `openspec/` still fails leg 1, and a requirement edited
under cover of a Purpose edit still fails leg 2. Both legs get a planted-defect run.

**8. Only the 26 placeholders are edited.** The 14 capabilities with written Purposes —
`agent-attribution`, `agent-launch`, `agent-list`, `agent-poller`, `change-artifacts`,
`change-enumeration`, `change-model`, `openspec-binary`, `repo-discovery`, `schema-artifacts`,
`schema-selection`, `subprocess-seam`, `task-checkboxes`, `task-groups` — are left alone.
Rewriting a correct Purpose to a house style is churn that the OPENSPEC-UNTOUCHED carve-out
would then have to widen to cover.

## Risks / Trade-offs

- **`GRAPH-SNAP` leg 5's proc-macro set is read from the *host* graph** (`cargo tree` with no
  `--target`; cargo omits the `(proc-macro)` tag for cross-target resolutions, so it cannot be
  pinned per triple). Its eight-name allowlist has only ever been measured on macOS. → Task
  3.4 records the eight names and task 8.12 reproduces the platform-independent intersection
  measured at planning time (identical on the macOS and Linux triples) and confirms it against
  a Linux CI run before the change is called done; if the sets ever differ, the allowlist
  becomes platform-aware in the same shape leg 4 now uses. Not deferred to a later change: it
  would be a gate hardcoding one platform's output, which is defect 3 all over again.
- **`make gates-full` adds a CI job that rebuilds the crate six times.** → It runs once, on
  `ubuntu-latest` only, with `Swatinem/rust-cache` and a `timeout-minutes` bound; it does not
  join the two-runner matrix.
- **`DEPS` adds `python3` to `make check`'s tool set.** → Present on both GitHub runners and on
  the reference machine (3.14.6). Recorded in `AGENTS.md` → Environment at task 6.2 rather than
  left as a surprise. Rewriting the leg in `sh` would trade a real dependency for fragile
  JSON-by-`sed`.
- **26 hand-written Purposes are a large, low-feedback edit.** → Group 1 works capability by
  capability, in two batches (tasks 1.2 and 1.3), re-reading each spec's requirements, and `openspec validate --specs --strict`
  gives a per-capability pass/fail after each batch rather than only at the end.
- **The `REMOVED` + `ADDED` shape of both delta specs** (rather than `RENAMED` + `MODIFIED`)
  was forced by the archiver: it refuses a `MODIFIED` block that drops a scenario the live spec
  has, and this change renames one. → Each `REMOVED` carries a **Migration** naming its
  replacement, and task 8.13 dry-runs the round trip — headers matched byte-for-byte, every
  scenario carried across — before the change is archived.

## Migration Plan

No deploy, no data migration, no rollback procedure. Every commit is independently revertible;
the riskiest single commit is the `Makefile` + `ci.yml` one, and reverting it restores the
previous four-gate composition exactly. Order: gates repaired and landed as files first
(groups 3–4), then wired into `make`/CI (group 5), then the Purposes (group 2 may run in
parallel — it shares no file). The Purpose test lands with the Purposes so it is never red
against a tree it cannot pass.

## Open Questions

None blocking. One is resolved *during* the change rather than before it: whether
`GRAPH-SNAP` leg 5's proc-macro allowlist is identical on Linux. Task 9.4 answers it from a
real CI run, and the two outcomes and their handling are written into that task, so the answer
does not become a decision made under time pressure.

**Visual design:** not applicable. This change modifies no user-facing view and no email
template; it adds no rendering code at all.
