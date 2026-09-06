<!-- Parallelism, examined. Groups 1, 2, and 3 share no file — group 1 writes
     `openspec/specs/**` and `tests/spec_purposes.rs`, group 2 writes `scripts/gates/deps.sh`,
     group 3 writes `scripts/gates/build-graph.sh` — none needs another's code to exist, and
     each group's own verification is scoped to its own subject (`cargo test --test
     spec_purposes`, and the two scripts run directly), so a failure stays attributable.
     Groups 1 and 3 are therefore marked `parallel-after: 0`. Everything after that is
     sequential and not by inheritance: group 4 needs both scripts to exist before the
     `Makefile` can name them and before `tests/ci_workflow.rs` can go green; group 5 plants
     defects against the finished scripts and the finished wiring; groups 6-8 are ordered by
     definition. -->

## Measured at planning time

Every figure below came from the command shown, run on `main` at `f6b4c3f`. **That SHA is
recorded for comparison only** — task 0.1 re-derives `BASE=$(git rev-parse HEAD)` fresh,
because this repository is shared with other live sessions and its history was rewritten once.

| Figure | Command | Value |
|---|---|---|
| Capability specs | `ls -1 openspec/specs \| wc -l` | **40** |
| Specs holding the archiver placeholder | `grep -l 'TBD - created by archiving' openspec/specs/*/spec.md \| wc -l` | **26** |
| `openspec validate --specs --strict` | as written | exit **1**, `Totals: 14 passed, 26 failed (40 items)`; all 26 failures are the identical `Purpose section is still a placeholder` warning promoted by `--strict` |
| Library tests | `cargo test --all-features --lib -- --list \| grep -c ': test$'` | **940**, unchanged by this change — the one new test file is an integration test |
| `tests/ci_workflow.rs` tests | `grep -c '#\[test\]' tests/ci_workflow.rs` | **11** → **17** (group 4 enumerates the six) |
| `tests/manifest.rs` / `tests/cli.rs` tests | same | **4** / **9**, unchanged |
| `find src -name '*.rs'` | as written | **25**, unchanged — no `src/` file is added |
| `find src tests -name '*.rs'` | as written | **28** → **29** (`tests/spec_purposes.rs`) |
| `find src/ui -name '*.rs'` | as written | **11**, unchanged |
| `#[test]` in `src/ui/view.rs` / `list.rs` / `app.rs` / `driver.rs` / `markdown.rs` / `tasks.rs` / `detail.rs` | `grep -c '^[[:space:]]*#\[test\][[:space:]]*$' <file>` | **94 / 29 / 72 / 34 / 24 / 16 / 23**, all unchanged |
| Line coverage | `cargo llvm-cov --summary-only \| tail -1` | baseline **97.05%** over **25,674** lines; re-measured in task 8.6, not assumed |
| Normal dependencies | `cargo metadata --no-deps --format-version 1` | **6**: `notify`, `pulldown-cmark`, `ratatui`, `serde_json`, `toml`, `yaml-rust2` |
| `python3` / `cargo` on the reference machine | `python3 --version`; `cargo --version` | **3.14.6** / **1.91.1** |

## Gate floors — measured plus this change's one enumerated new file

`tests/spec_purposes.rs` is the only file this change adds to the `find src tests` set, so
every floor below moves by exactly one or not at all. **Every gate is invoked with its floor
named**; a gate invoked bare runs at its block default, which is a threshold nobody chose.

| Gate | Searched set | Measured | New | Floor to invoke with |
|---|---|---|---|---|
| `AGENTSEAM` | `find src tests` less the 5 `ALLOWED` | 23 | +1 | `MIN=24 ALLOWED='src/cli.rs src/agents.rs src/ui/mod.rs src/launch.rs src/open.rs'` |
| `LAUNCHSEAM` (both invocations) | same | 23 | +1 | `MIN=24`, same `ALLOWED`, `ENTRY` as landed |
| `NOSLEEP` | `find src tests` | 28 | +1 | `SLEEP_MIN=6 MIN=29` — **6**, not the 5 `plugin-actions` carried: re-measured at `f6b4c3f`, a floor below its realized count is the defect this table exists to prevent |
| `WATCHSEAM` | `find src tests` less `src/watch.rs` | 27 | +1 | `MIN=28` |
| `NOSPAWN-GREP`, `NOLIT-CHANGE` | `find src` less `src/changes.rs` | 24 | 0 | `MIN=24` |
| `MDSEAM` | `find src` less `src/ui/markdown.rs` | 24 | 0 | `MIN=24` |
| `NOCLI-SHELL`, `NOBLOCK`, `READONLY-UI` | `find src/ui` | 11 | 0 | `UI_MIN=11` |
| `READSEAM` | `find src/ui` | 10 | 0 | `UI_MIN=10` |
| `WIDTHS` / `LISTWIDTHS` / `MDWIDTHS` / `TASKWIDTHS` / `DETAILWIDTHS` | `#[test]` under `src/ui/` | 94 / 29 / 24 / 16 / 23 | 0 | `WIDTHS_MIN=94`, `LIST_MIN=29`, `MD_MIN=24`, `TASK_MIN=16`, `DETAIL_MIN=23` |
| `GATE-MECH1` | `find src` | 25 | 0 | floor hardcoded at 8; no edit |

## 0. Baseline, gate extraction, and the carve-out
<!-- kind: operational -->

- [x] 0.1 CHECK: Derive the base SHA fresh — `BASE=$(git rev-parse HEAD)` — and record it here.
  Every later `OPENSPEC-UNTOUCHED` run uses **this** SHA, never a re-derived `HEAD`, which
  after the first commit compares the change against itself.

  **BASE=`1f9f29ae1fc2870733671bcdd75cb184617a3c99`** (`f6b4c3f` is the planning-time
  value and is recorded for comparison only; the plan's own last commit landed as `1f9f29a`).

- [x] 0.2 CHANGE: Extract the gate roster into a scratch directory `$CHECKS` following
  `openspec/changes/archive/2026-09-06-plugin-actions/tasks.md` task 0.2, and confirm
  `ls -1 "$CHECKS" | wc -l` is **30**. This change adds none to the roster: it *removes* two
  from it by making them repository files instead (group 4), which is recorded in task 7.2 and
  takes effect for the next change, not this one.

  `DEPS.sh`, `GRAPH-SNAP.sh`, and `AGENTSEAM.sh` as extracted at planning time are already
  committed under `openspec/changes/spec-purposes/notes/extracted-gates/` and may be copied
  from there rather than re-extracted.

  **Done**: copied all 30 files from `notes/extracted-gates/` into a scratchpad `$CHECKS`;
  `ls -1 "$CHECKS" | wc -l` = **30**, confirmed.

- [x] 0.3 CHECK: Record each gate's exit status and verbatim output at `BASE`, at the floor
  named in the table above. Expected, and confirmed at planning time:

  | Gate | Exit | Verbatim |
  |---|---|---|
  | `WORK=$(mktemp -d) DEPS_SKIP_LEG5=1 sh $CHECKS/DEPS.sh` | **1** | `AssertionError: normal deps are ['notify', 'pulldown-cmark', 'ratatui', 'serde_json', 'toml', 'yaml-rust2'], expected ['pulldown-cmark', 'ratatui', 'serde_json', 'toml', 'yaml-rust2']` then `DEPS FAIL: leg 2a` |
  | `sh $CHECKS/GRAPH-SNAP.sh` | **1** | `GRAPH-SNAP FAIL: macOS/Linux differ by [fsevent-sys inotify inotify-sys linux-raw-sys ], expected [linux-raw-sys ]` |
  | `MIN=24 ALLOWED='src/cli.rs src/agents.rs src/ui/mod.rs src/launch.rs src/open.rs' sh $CHECKS/AGENTSEAM.sh` | **1** at `MIN=24` | `searched only 23 files (expected >= 24)` — correct, and it goes green the moment group 1 adds `tests/spec_purposes.rs` |

  `AGENTSEAM` at the **landed** `MIN=23` is **green** at `BASE`
  (`AGENTSEAM OK: 23 files searched (>= 23)`). Every other gate in the roster runs green at
  its floor from the table above; record any that does not, because it is then a fourth
  inherited condition this plan did not anticipate.

  **Recorded, run against real `BASE`:** `DEPS` and `GRAPH-SNAP` reproduce the table's
  verbatim output exactly. `AGENTSEAM` at `MIN=24` fails exactly as predicted
  (`searched only 23 files (expected >= 24)`); at the landed `MIN=23` it is green. Both
  `LAUNCHSEAM` invocations reproduce the same pattern: green at the landed `MIN=23`, red at
  `MIN=24` (`searched only 23 files`). `WATCHSEAM` green at landed `MIN=27`, red at new
  `MIN=28` (`searched only 27 files`). `NOSLEEP`: **6** sleep sites measured (not 5 — the
  landed invocation `SLEEP_MIN=5 MIN=28` undercounts; `SLEEP_MIN=6 MIN=28` is green at
  `BASE`, confirming the planning-review correction), and the new-change floor
  `SLEEP_MIN=6 MIN=29` is red as predicted (`searched only 28 files`).
  `NOSPAWN-GREP`/`NOLIT-CHANGE`/`MDSEAM` green at `MIN=24` (unchanged). `NOCLI-SHELL`
  `UI_MIN=11`, `READSEAM` `UI_MIN=10`, `NOBLOCK` `UI_MIN=11`, `READONLY-UI`
  `EXTRA='src/watch.rs src/refresh.rs src/agents.rs src/launch.rs src/open.rs' UI_MIN=11` —
  all green, unchanged. `WIDTHS_MIN=94`, `LIST_MIN=29`, `MD_MIN=24`, `TASK_MIN=16`,
  `DETAIL_MIN=23` all green. `GATE-MECH1` (run with `python3`, not `sh` — it is a Python
  script) green: half A 25 files, half B 84 constructions. `NOTABSEAM`, `NORAW-GREP`,
  `NOWAIVER`, `TASKSEAM`, `NOJSON-SEAM`, `NOIO-VIEW` all green. All four `NODEFAULT-UI`
  invocations (app `SCAN_MIN=174`, agents `SCAN_MIN=103`, launch `SCAN_MIN=23`, open
  `SCAN_MIN=12`) green. `TESTCOUNT` is a sourced helper, not a standalone script;
  `cargo test --all-features --lib -- --list | grep -c ': test$'` = **940**, matching the
  baseline. `OPENSPEC-UNTOUCHED.sh` (the pre-existing single-leg form, `CHANGE=spec-purposes`)
  is green at `BASE` (nothing has touched `openspec/` yet). `EXTENDED` is skipped: it names
  per-change test-extension pairs and this change extends no existing Rust test.

  **A fourth inherited condition, unanticipated by this plan: `WIRED` is RED at `BASE`.**
  `sh $CHECKS/WIRED.sh` fails leg 2 — `'pub fn run()' holds a branch or a loop` — because
  `src/ui/mod.rs::run()` now contains
  `match startup_cwd(&crate::config::env_lookup()) { Some(cwd) => cwd, None => std::env::current_dir()? }`,
  added by `plugin-actions`' final cwd-resolution fix (see `HANDOFF.md` →
  "`plugin-actions` found a bug in already-shipped code") after `WIRED` was last confirmed
  green. **Not repaired here**: fixing it means moving the branch into `run_wired` inside
  `src/ui/mod.rs`, a `src/` behaviour edit this change's Non-Goals explicitly rule out
  ("Not new plugin behaviour... Impact: `src/` is not modified"). Recorded as a new
  follow-up in `HANDOFF.md` (task 6.3/6.4) rather than fixed under cover of this change.

- [x] 0.4 CHANGE: Write `$CHECKS/OPENSPEC-UNTOUCHED-SP.sh` — this change's two-leg carve-out,
  per design.md → Decisions → 7. Leg 1 excludes this change's artifact directory and the 26
  spec paths **by name**; leg 2 strips the `## Purpose` section from both the `BASE` blob and the
  worktree file and requires the remainder to be byte-identical, so an edit to requirement
  *prose* — where the SHALL sentences live, producing no heading line at all — is caught too.

  ```sh
  # OPENSPEC-UNTOUCHED (spec-purposes). Leg 1: nothing under openspec/ changed against BASE
  # except this change's own artifact directory and the 26 capability specs named below.
  # Leg 2: outside its ## Purpose section each of the 26 is byte-identical to BASE. A line
  # grep over added/removed headings is NOT enough: the normative SHALL sentences are plain
  # prose, and an edit to one produces no heading line at all.
  [ -n "$BASE" ] || { echo "FAIL: BASE sha not set" >&2; exit 1; }
  ROOT=$(git rev-parse --show-toplevel) || { echo "FAIL: not a git repo" >&2; exit 1; }
  git -C "$ROOT" rev-parse --verify "$BASE^{commit}" >/dev/null || { echo "FAIL: bad BASE" >&2; exit 1; }
  SPECS="artifact-content artifact-tabs change-merge change-rows ci-workflow cli-changes dashboard-loop detail-header detail-scroll list-filtering list-selection live-updates markdown-render pane-open plugin-build plugin-config plugin-manifest plugin-state quality-gates refresh-worker responsive-layout schema-cli-fallback tasks-checklist tasks-progress-bar terminal-lifecycle watch-invalidation"
  n=$(printf '%s\n' $SPECS | wc -l | tr -d ' ')
  [ "$n" -eq 26 ] || { echo "FAIL: SPECS names $n paths, expected 26" >&2; exit 1; }
  allow=$(printf 'openspec/specs/%s/spec.md\n' $SPECS)
  stray=$( { git -C "$ROOT" diff --name-only "$BASE" -- ':(top)openspec/'
             git -C "$ROOT" ls-files --others --exclude-standard -- ':(top)openspec/'
             git -C "$ROOT" ls-files --others --ignored --exclude-standard -- ':(top)openspec/'; } \
           | grep -v '^openspec/changes/spec-purposes/' | grep -vxF "$allow" | sort -u || true)
  [ -z "$stray" ] || { echo "FAIL: wrote inside openspec/ outside this change:" >&2; echo "$stray" >&2; exit 1; }
  strip() { awk '/^## /{skip=($0=="## Purpose")} !skip'; }
  T=$(mktemp -d); rc=0
  for c in $SPECS; do
    git -C "$ROOT" show "$BASE:openspec/specs/$c/spec.md" | strip > "$T/base" \
      || { echo "FAIL: $c missing at BASE" >&2; rc=1; continue; }
    [ -f "$ROOT/openspec/specs/$c/spec.md" ] || { echo "FAIL: $c/spec.md deleted" >&2; rc=1; continue; }
    strip < "$ROOT/openspec/specs/$c/spec.md" > "$T/now"
    diff -u "$T/base" "$T/now" > "$T/d" \
      || { echo "FAIL: $c's diff reaches outside the Purpose section:" >&2; sed -n '1,20p' "$T/d" >&2; rc=1; }
  done
  rm -rf "$T"; [ $rc -eq 0 ] || exit 1
  echo "OPENSPEC-UNTOUCHED OK: 26 Purpose-only spec edits, nothing else under openspec/"
  ```

  **Run at planning time against `f6b4c3f`, with both negative controls:** clean tree → exit
  **0**, `OPENSPEC-UNTOUCHED OK`. Plant a stray `openspec/specs/agent-list/STRAY.txt` → exit
  **1**, leg 1 names the file. Remove it, append a `#### Scenario: planted` block to
  `openspec/specs/pane-open/spec.md` → exit **1**, leg 2 prints the planted lines. Restore →
  exit **0** again. Leg 2's strip-and-diff form was additionally proved against the case a
  line grep misses: flipping `SHALL NOT` to `SHALL` in one requirement's prose → exit **1**
  (the earlier grep form returned exit 0 on the same plant), a legitimate multi-line Purpose →
  exit **0**, and one of the 26 deleted outright → exit **1**.

  **Re-run against real `BASE` (`1f9f29a`), all six controls reproduced exactly:** clean
  tree → exit 0, `OPENSPEC-UNTOUCHED OK: 26 Purpose-only spec edits, nothing else under
  openspec/`. Stray `openspec/specs/agent-list/STRAY.txt` → exit 1, leg 1 names it; removed
  → exit 0. Appended `#### Scenario: planted` to `openspec/specs/pane-open/spec.md` → exit 1,
  leg 2 prints the planted lines; restored → exit 0. `SHALL NOT`→`SHALL` flip in
  `openspec/specs/plugin-config/spec.md` prose (no heading line produced) → exit 1, leg 2
  prints the prose diff; restored → exit 0. A legitimate multi-line Purpose body in
  `openspec/specs/quality-gates/spec.md` → exit 0. `openspec/specs/tasks-checklist/spec.md`
  deleted outright → exit 1 (`tasks-checklist/spec.md deleted`); restored → exit 0.

- [x] 0.5 VERIFY: `git status --porcelain` is empty apart from this change's own artifact
  directory, and `ls -1 "$CHECKS" | wc -l` is 30. Commit nothing in this group.

  **Verified:** `git status --porcelain` empty; `ls -1 "$CHECKS" | wc -l` = 30. Nothing
  committed for this group — it is baseline recording only, folded into this tasks.md edit.

## 1. Every capability carries a written Purpose
<!-- kind: behavior -->
<!-- parallel-after: 0 -->

- [x] 1.1 RED: Write `tests/spec_purposes.rs` with three tests named for their scenarios —
  `every_capability_has_a_written_purpose`, `an_archived_placeholder_fails_the_test`, and
  `the_test_cannot_pass_vacuously`. The first walks every directory under
  `openspec/specs/` (resolved from `CARGO_MANIFEST_DIR`, as `tests/ci_workflow.rs` does),
  reads each `spec.md`, and asserts its `## Purpose` body is non-empty and does not contain
  `TBD - created by archiving change`. Factor the walk into a function taking the specs
  directory as a parameter so the other two tests can point it at a temp directory.

  **Red at `BASE`**, by the shell equivalent of the first test:

  ```sh
  n=$(grep -l 'TBD - created by archiving' openspec/specs/*/spec.md | wc -l | tr -d ' ')
  [ "$n" -eq 0 ] || { echo "FAIL: $n capability specs still hold the archiver placeholder" >&2; exit 1; }
  ```

  Run at planning time: exit **1**, `FAIL: 26 capability specs still hold the archiver
  placeholder`. It cannot be green before group 1.2 — the behaviour does not exist yet.

- [x] 1.2 GREEN: Write the Purpose for the first thirteen capabilities — `artifact-content`,
  `artifact-tabs`, `change-merge`, `change-rows`, `ci-workflow`, `cli-changes`,
  `dashboard-loop`, `detail-header`, `detail-scroll`, `list-filtering`, `list-selection`,
  `live-updates`, `markdown-render`. Read each capability's own `### Requirement:` headings
  first and derive the Purpose from them; `notes/purposes-draft.md` holds a planning-time
  draft per capability with the requirements it was derived from, and is a reference, not an
  authority. Two capabilities must not end up with interchangeable Purposes.

  **Done.** Each of the 13 was re-derived from its own `### Requirement:` headings (the
  drafts were used as a starting point, lightly tightened where they referenced a stale
  count).

- [x] 1.3 GREEN: The same for the remaining thirteen — `pane-open`, `plugin-build`,
  `plugin-config`, `plugin-manifest`, `plugin-state`, `quality-gates`, `refresh-worker`,
  `responsive-layout`, `schema-cli-fallback`, `tasks-checklist`, `tasks-progress-bar`,
  `terminal-lifecycle`, `watch-invalidation`. The `tasks-*` cluster and
  `schema-cli-fallback`'s neighbours must each be distinguishable from their Purpose alone.

  The fourteen capabilities that already carry a written Purpose are **not** edited
  (design.md → Decisions → 8): `agent-attribution`, `agent-launch`, `agent-list`,
  `agent-poller`, `change-artifacts`, `change-enumeration`, `change-model`, `openspec-binary`,
  `repo-discovery`, `schema-artifacts`, `schema-selection`, `subprocess-seam`,
  `task-checkboxes`, `task-groups`.

  **Done.** `plugin-build`'s draft was updated to also name `scripts/gates/deps.sh` and
  `scripts/gates/build-graph.sh` (this change's own repair) rather than the pre-repair
  command-level gates. `quality-gates`'s draft was updated the same way, plus naming the
  Purpose-guard test itself. `tasks-checklist` (the grammar) and `tasks-progress-bar` (the
  leading gauge line) read as clearly distinct subjects; `schema-cli-fallback` (asking the
  CLI when a schema is not vendored) does not overlap the already-written
  `schema-artifacts` (parsing a vendored schema) or `schema-selection` (choosing a schema
  name).

- [x] 1.4 REFACTOR: Read the 26 Purposes as a set and remove any that restates its
  capability's name or duplicates a sibling's scope claim. If none does, state that here and
  say what was compared.

  **None does.** Compared all 26 opening sentences pairwise for restated names and
  overlapping scope claims (see the grep of each spec's fourth line, run during this task):
  every one opens on a distinct verb and subject (e.g. "Governs...", "Owns...", "Fixes...",
  "Specifies...", "Recovers...", "Turns..."), and each closes by naming the sibling
  capability its own scope stops at (e.g. `artifact-content` → `artifact-tabs`/
  `detail-scroll`; `tasks-checklist` → `tasks-progress-bar`). No edit was needed.

- [x] 1.5 VERIFY: `cargo test --test spec_purposes` — three tests, green.
  `grep -l 'TBD - created by archiving' openspec/specs/*/spec.md | wc -l` is **0**.
  `openspec validate --specs --strict` exits **0** with `Totals: 40 passed, 0 failed`.
  `BASE=<0.1> sh $CHECKS/OPENSPEC-UNTOUCHED-SP.sh` exits 0. Commit.

  **Verified:** `cargo test --test spec_purposes` — 3 passed, 0 failed. Placeholder grep
  count = 0. `openspec validate --specs --strict` → `Totals: 40 passed, 0 failed (40 items)`
  (INFO-level "Requirement text is very long" notices on several capabilities are
  pre-existing and not warnings or failures). `OPENSPEC-UNTOUCHED-SP.sh` at
  `BASE=1f9f29a` → `OPENSPEC-UNTOUCHED OK: 26 Purpose-only spec edits, nothing else under
  openspec/`.

## 2. The dependency gate becomes a file, with a re-derived want-list
<!-- kind: operational -->

- [ ] 2.1 CHECK: Run the extracted `DEPS.sh` at `BASE` and confirm the failure is byte-identical
  to the one task 0.3 recorded — leg 2a, five wanted against six declared. A different failure
  means the baseline moved and this group's plan needs re-deriving before it starts.

- [ ] 2.2 CHANGE: Write `scripts/gates/deps.sh` from the extracted block, and **re-derive** the
  leg 2a want-list from `cargo metadata --no-deps` reconciled against `plugin-build`'s "The
  crate produces one binary from an argued dependency set" requirement — which already names
  all six crates and their exact feature lists correctly. Do not patch `notify` into the
  existing five-entry dict; rebuild the dict from that table so a second stale entry cannot
  survive the repair.

  The resulting sixth entry is `"notify": ["macos_fsevent"]`, with `uses_default_features`
  false, matching `Cargo.toml`'s
  `notify = { version = "8.2.0", default-features = false, features = ["macos_fsevent"] }`.

- [ ] 2.3 CHANGE: Default `WORK` inside the script — `WORK="${WORK:-$(mktemp -d)}"` with a
  `trap` that removes it — replacing the `: "${WORK:?...}"` hard requirement the extracted
  block carries. `WORK` is used by leg **4**, not only leg 5, so `env -u WORK sh deps.sh`
  exits 1 at HEAD and the `make gates` recipe could never pass without this.

- [ ] 2.4 CHANGE: Add the missing `needed notify "..."` removal experiment to leg 5, so all six
  dependencies are argued on the same terms, and gate legs **1b** and **5** behind
  `${DEPS_FULL:-}` so `make gates` runs neither. Leg 1b builds the release binary and leg 5
  rebuilds the crate six times; both run under `make gates-full` (design.md → Decisions → 3).

- [ ] 2.5 CHECK: Re-read `plugin-build`'s dependency table and confirm every crate, version
  floor, and feature list in the new want-list matches it exactly. This is the contract gate:
  the want-list and that requirement are two sites that must agree, and the gate's whole value
  is that they do.

- [ ] 2.6 VERIFY: `env -u WORK sh scripts/gates/deps.sh` exits **0**, printing one `DEPS OK (leg 2a)` line
  naming six normal deps. `DEPS_FULL=1 sh scripts/gates/deps.sh` also exits 0, and its leg 5
  output names six removal experiments, not five. No refactor was needed beyond the want-list
  rebuild itself, which is 2.2's subject. Commit.

## 3. The build-graph gate becomes a file, with a direction-aware platform assertion
<!-- kind: operational -->
<!-- parallel-after: 0 -->

- [ ] 3.1 CHECK: Run the extracted `GRAPH-SNAP.sh` at `BASE` and confirm two things: the
  `diff -u "$SNAP"` leg **passes** (the snapshot is current — regenerated in `574b87d`), and
  the run fails four legs later on
  `macOS/Linux differ by [fsevent-sys inotify inotify-sys linux-raw-sys ], expected
  [linux-raw-sys ]`. If the snapshot diff fails instead, stop: the diagnosis this group is
  built on no longer holds.

- [ ] 3.2 CHANGE: Write `scripts/gates/build-graph.sh` from the extracted block, replacing the
  single unordered literal at the old leg 4 —

  ```sh
  d=$(diff "$TMP/aarch64-apple-darwin" "$TMP/aarch64-unknown-linux-gnu" \
      | grep -E '^[<>]' | sed 's/^[<>] //' | cut -d' ' -f1 | sort -u | tr '\n' ' ')
  [ "$d" = "linux-raw-sys " ] || { echo "GRAPH-SNAP FAIL: macOS/Linux differ by [$d], expected [linux-raw-sys ]" >&2; exit 1; }
  ```

  — with two direction-aware assertions, per design.md → Decisions → 4: the macOS-only set
  (packages in the macOS graph and not the Linux one) against `fsevent-sys`, and the
  Linux-only set against `inotify inotify-sys linux-raw-sys`. Each failure message names which
  side the unexpected package appeared on.

- [ ] 3.3 CHANGE: Make the script ignore an **ambient** `GRAPH_WRITE`: as extracted, any value
  in the environment makes it rewrite the snapshot and exit 0 with **zero assertions run**, and
  task 3.5's `git diff --exit-code` stays clean because the rewrite is byte-identical. Read the
  write mode from an explicit argument instead, and have `make gates` clear the variable. A gate
  artefact regenerated unattended blesses the current state without review.

- [ ] 3.4 CHECK: Read leg 5's proc-macro allowlist and record here that it is derived from the
  **host** graph (`cargo tree` with no `--target`, because cargo omits the `(proc-macro)` tag
  for cross-target resolutions) and has only ever been measured on macOS. Task 9.4 confirms it
  on Linux in CI; record what the eight names are so that comparison is possible.

- [ ] 3.5 VERIFY: `GRAPH_WRITE=1 sh scripts/gates/build-graph.sh` exits **0** and its output
  **contains the `OK` line** — not merely exits 0, which a silent snapshot rewrite also does.
  `git diff --exit-code tests/fixtures/build-graph.txt` is clean afterwards, proving the run
  did not rewrite its own subject. No refactor was needed: the change is one leg's assertion.
  Commit.

## 4. `make gates` joins `make check`, and CI follows it
<!-- kind: behavior -->

- [ ] 4.1 RED: Extend `tests/ci_workflow.rs` with six tests — `gates_target_exists_and_names_both_scripts`,
  `check_composes_gates_third`, `gates_full_is_not_composed_into_check`,
  `both_runners_run_the_gates_step`, `gates_full_has_its_own_unconditional_job`, and
  `gates_full_is_in_the_aggregate_needs` — and **rewrite**
  `every_gate_the_makefile_composes_runs_in_ci` so it parses `check:`'s own prerequisite list
  out of the `Makefile` instead of comparing against a hardcoded four-item literal. As landed
  it would stay green when a fifth gate joins `check` with no CI step, so it does not force
  what design.md → Test Strategy relies on it to force; after the rewrite it does, for this
  gate and the next one. Widen `no_gate_step_can_be_skipped_or_ignored`
  to the new targets. Also update `parser_preconditions_hold`, which asserts
  `sections.len() == 3`: task 4.3 adds a fourth job. `grep -c '#\[test\]' tests/ci_workflow.rs`
  moves **11 → 17** (the brackets must be escaped; unescaped it is a character class and
  returns 0).

  **Red at `BASE`**, by the shell equivalent:

  ```sh
  grep -qE '^gates:' Makefile && grep -q 'make gates' .github/workflows/ci.yml \
    || { echo "FAIL: no gates target and no CI step" >&2; exit 1; }
  ```

  Run at planning time: exit **1**. `grep -c gates Makefile` is `0` at `BASE`.

- [ ] 4.2 GREEN: Add `gates` and `gates-full` to the `Makefile`'s `.PHONY` list and define them
  as design.md → Contracts specifies, and recompose `check` as
  `fmt-check lint gates test coverage`.

- [ ] 4.3 GREEN: Add a `Gates` step running `make gates` to the `check` job, positioned between
  `Lint` and `Test` so the CI order matches the composite's, and a `gates-full` job on
  `ubuntu-latest` running `make gates-full`, with `timeout-minutes`, no `if:`, and added to
  the aggregate job's `needs`. It compiles, so it takes the same `actions/checkout`,
  `dtolnay/rust-toolchain@stable`, and `Swatinem/rust-cache` preamble the `coverage` job has —
  `ci-workflow`'s live "Each job installs the toolchain it needs" and "Caching speeds a run up"
  requirements apply to it unchanged, and a job without them would violate both.

- [ ] 4.4 CHECK: Contract gate. `make check`'s composition is consumed by `.github/workflows/ci.yml`
  through `tests/ci_workflow.rs`. Re-read all three and confirm every gate command is written
  in exactly one place — the `Makefile` — and that no CI step restates a `cargo` invocation.
  Re-read `ci-workflow`'s live "CI invokes every gate through `make`" and "The composite target
  is not used" requirements against the new steps and confirm both still hold as written; if
  either now needs different words, the delta spec is short a requirement and this task says so
  rather than leaving the live spec stale.

- [ ] 4.5 REFACTOR: If `tests/ci_workflow.rs`'s gate list is now spelled out in more than one
  test, lift it to a single `const`. If it is not, state that here.

- [ ] 4.6 VERIFY: `cargo test --test ci_workflow` — 17 tests, green. `make gates` exits 0.
  `make check` exits 0 end to end. Commit.

## 5. Every gate is proved able to fail, and to see what it guards
<!-- kind: operational -->

Every plant below is made in a `cp -R` copy of the tree, or in a file restored from a
set-aside copy; the working tree is never left modified. Record each exit status and the
relevant output line beside the task.

- [ ] 5.0 CHECK: The two pre-existing gates still stop the run. Misformat a set-aside copy of
  `src/config.rs` and run `make check` → non-zero at `cargo fmt --all -- --check`, with no
  lint, gates, test, or coverage output; restore → exit 0. Then add `let _ = x.clone();` on a
  `Copy` value and run `make lint` → non-zero, reported by clippy rather than `cargo build`;
  restore → exit 0. These two scenarios are carried forward unchanged from `ci-pipeline` and
  are re-run because `check`'s composition moved underneath them.

- [ ] 5.1 CHECK: `deps.sh` catches an unargued dependency. Add `once_cell = "1"` **inside the
  `[dependencies]` table** of a copied `Cargo.toml` — appended at the end of the file it lands
  in `[dev-dependencies]`, which leg 2a correctly ignores, and the plant passes — run the script there → non-zero, leg 2a naming `once_cell` in the declared set.
  Remove it → exit 0.

- [ ] 5.2 CHECK: `deps.sh` cannot pass vacuously. Delete the whole `[dependencies]` table in a
  copy → the script must **fail**, not report success over an empty set. If it passes, add the
  guard that makes it fail and re-run both halves.

- [ ] 5.3 CHECK: `build-graph.sh`'s platform assertion is direction-aware. Swap the macOS-only
  and Linux-only expected lists in a copied script → non-zero, and the message names which
  side each package appeared on rather than one merged set. Restore → exit 0.

- [ ] 5.4 CHECK: `build-graph.sh` catches a graph-moving feature change. Set `notify`'s
  features to `["macos_kqueue"]` in a copied tree and run without regenerating → non-zero at
  the snapshot diff. Confirm `tests/fixtures/build-graph.txt` in the copy is unmodified by the
  failing run.

- [ ] 5.5 CHECK: The CI wiring test catches an omitted step. Delete the `Gates` step from
  `.github/workflows/ci.yml`, run `cargo test --test ci_workflow` → red on
  `every_gate_the_makefile_composes_runs_in_ci`. Restore from the set-aside copy → green. This
  is the `live-refresh` wiring lesson's analogue: it proves the Makefile→CI link is enforced
  rather than merely written.

- [ ] 5.6 CHECK: The Purpose test catches a fresh placeholder. Replace one capability's Purpose
  body with `TBD - created by archiving change some-change`, run
  `cargo test --test spec_purposes` → red, naming that capability. Restore → green.

- [ ] 5.7 CHECK: `make check` stops at the gates rather than after coverage. In a copied tree,
  add a seventh dependency and run `make check` → non-zero at `scripts/gates/deps.sh`, with no
  `cargo test` and no `cargo llvm-cov` output in the run.

- [ ] 5.8 CHECK: The two `OPENSPEC-UNTOUCHED-SP` legs. Plant a stray file under
  `openspec/specs/agent-list/` → leg 1 fires naming it; remove it. Append a
  `#### Scenario:` block to one of the 26 → leg 2 fires printing the planted lines; restore.
  Confirm the gate goes quiet again both times.

- [ ] 5.9 VERIFY: `git status --porcelain` is empty. Every plant above is recorded with its
  exit status and output line. Nothing in this group is committed except this file's updates.

## 6. Documentation
<!-- kind: operational -->

- [ ] 6.0 CHECK: Before editing, grep each target document for the text each task below claims
  to correct, and record what is actually there. `AGENTSEAM` appears in **neither**
  `HANDOFF.md` nor `openspec/IMPLEMENTATION-ORDER.md` at `f6b4c3f`, so 6.4 adds a fact rather
  than correcting a sentence, and 6.5 is a confirmation rather than an edit.

- [ ] 6.1 CHANGE — Rewrite in `AGENTS.md` → Quality gates (audience: every future agent session): the
  gate table gains `gates` and `gates-full` rows and the composite becomes five targets, and
  the sentence describing CI's four steps becomes five. Replaces the current four-gate text
  rather than appending beside it. Durable because every session reads this to know what
  `make check` covers, and a gate list that omits two gates sends the reader to run them by
  hand — which is how they rotted.

- [ ] 6.2 CHANGE — Rewrite in `AGENTS.md` → Environment (audience: same): `python3` is now required by
  `make check`, alongside the existing `clippy` and `cargo-llvm-cov` note. One line; it
  corrects a list that is now incomplete rather than adding a new rule.

- [ ] 6.3 CHANGE — Rewrite in `HANDOFF.md` → "Open: the dependency gate has been red on `main`"
  (audience: the next session picking this repository up): replace that whole section with
  what actually happened — both gates repaired and now files under `scripts/gates/` run by
  `make check`; the twenty-eight remaining extracted gates named as the follow-up; and the
  `AGENTSEAM` correction, that `plugin-actions`' "22 + `src/open.rs`" credited a file it had
  simultaneously added to `ALLOWED` while omitting `tests/manifest.rs`, so the floor was right
  for a reason nobody recorded. **Net removal**: the old section is longer than its
  replacement.

- [ ] 6.4 CHANGE — Add to `HANDOFF.md` → "Standing rule: pass every gate its explicit floor"
  (audience: same): the rule stands unchanged; add one entry recording that `AGENTSEAM` is
  **not** a third instance of a gate running below its realized count but the opposite failure
  — a floor that is correct while its recorded arithmetic is not, so a reader who re-derives it
  gets the right number by accident. Add alongside it that `NOSLEEP` **was** a third instance:
  it has been invoked at `SLEEP_MIN=5` against a realized 6 since `plugin-actions`.

- [ ] 6.5 VERIFY: Confirm `openspec/IMPLEMENTATION-ORDER.md`'s Phase 6 `spec-purposes` row still
  describes what was built. It currently says `AGENTSEAM` is not in scope and describes both
  other repairs correctly; correct the row if the scope moved, per
  `openspec/config.yaml` → `operations.archive`.

## 7. Change Review
<!-- kind: operational -->

- [ ] 7.1 CHECK: Dispatch an independent reviewer — not a fork of this session — against
  `proposal.md`, both delta specs, `design.md`, `tasks.md`, and the full diff. Instruct it to
  write findings to a scratchpad file incrementally rather than returning them only in its
  final message. Concentration points for this change specifically: (a) every one of the 26
  Purposes is derived from its own capability's requirements and no two are interchangeable;
  (b) every check in group 5 was actually run and its output recorded, not asserted; (c) the
  want-list and `plugin-build`'s dependency table agree crate for crate and feature for
  feature; (d) no floor in the table above was invoked bare.

- [ ] 7.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
  one-line reason, note SUGGESTIONs, and re-run the affected tests.

- [ ] 7.3 VERIFY: No blocking or unowned finding remains. Record the finding counts by severity.

## 8. Lint & Verify
<!-- kind: operational -->

- [ ] 8.1 CHECK: Inspect the verification commands below against what this change actually
  touched, and name any tier that does not apply.

- [ ] 8.2 VERIFY: `cargo fmt --all -- --check` — clean.

- [ ] 8.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings. No
  `#[allow]` was added; `src/changes.rs`'s existing one is untouched, because this change does
  not touch that file. (Rust has no separate type-check step: `clippy` is it.)

- [ ] 8.4 VERIFY: `cargo test --all-features` — green. Library test count is still **940**
  (`cargo test --all-features --lib -- --list | grep -c ': test$'`); `tests/ci_workflow.rs` is
  at 17 tests and `tests/spec_purposes.rs` at 3.

- [ ] 8.5 VERIFY: `make gates` and `DEPS_FULL=1 make gates-full` each exit 0.

- [ ] 8.6 VERIFY: `cargo llvm-cov --fail-under-lines 80` passes, and
  `cargo llvm-cov --summary-only | tail -1` is re-measured and recorded as a **line**
  percentage over a line count — baseline 97.05% over 25,674 lines. No `src/` line was added,
  so a material move is a finding, not a result.

- [ ] 8.7 VERIFY: Run the full gate roster at the floors named in the "Gate floors" table —
  `AGENTSEAM` and both `LAUNCHSEAM` invocations at `MIN=24`, `NOSLEEP` at
  `SLEEP_MIN=6 MIN=29`, `WATCHSEAM` at `MIN=28`, and every unchanged gate at its listed floor.
  Not one is invoked bare.

- [ ] 8.8 VERIFY: `BASE=<the SHA from task 0.1> sh $CHECKS/OPENSPEC-UNTOUCHED-SP.sh` exits 0 —
  never a re-derived `HEAD`, which after the first commit compares the change against itself.

- [ ] 8.9 VERIFY: `openspec validate --specs --strict` exits 0 with `Totals: 40 passed, 0
  failed`, and `openspec validate spec-purposes --strict` reports the change valid.

- [ ] 8.10 VERIFY: `make check` exits 0 end to end, in one run, as the single gate.

- [ ] 8.11 VERIFY: Every commit in this change is signed —
  `git log --format=%H <BASE>..HEAD | while read s; do git cat-file commit $s | grep -q '^gpgsig' || echo "UNSIGNED $s"; done`
  prints nothing. Never `%G?`: `gpg.ssh.allowedSignersFile` is unset, so it reports `N` for
  signed commits too.

- [ ] 8.12 CHECK: Confirm leg 5's proc-macro allowlist is platform-independent. Measured at
  planning time by intersecting `cargo metadata`'s proc-macro target kinds with each triple's
  own `cargo tree`: the same **eight** names on the macOS and the Linux triples. Reproduce that
  intersection here, and confirm it against the `ubuntu-latest` CI run. If the two ever differ,
  make the allowlist platform-aware in the direction-aware shape leg 4 now uses and re-run
  tasks 3.5 and 5.3 — a gate hardcoding one platform's output is the defect this change repairs.

- [ ] 8.13 VERIFY: Dry-run the archive round trip for both delta specs. For each, confirm the
  `REMOVED` header matches the live spec's header byte-for-byte and the `ADDED` replacement
  carries every scenario the removed requirement had, then diff the resulting merged spec
  against the pre-change one and confirm the only losses are the two renamed headers. Both
  deltas use `REMOVED`+`ADDED` rather than `RENAMED`+`MODIFIED` (design.md → Risks), so this is
  the check that the round trip drops nothing.
