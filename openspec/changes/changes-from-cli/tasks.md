# changes-from-cli — tasks

Read `design.md` first: the module boundaries, the Test Boundaries table, and the
verification matrix are the contract these tasks implement. No task may invent a
collaborator that table does not name.

**No acceptance-test group.** design.md → Test Strategy states why: an outer loop for this
change would have to run the real `openspec` binary, which the roadmap's "the subprocess
seam exists before anything crosses it" principle exists to prevent and which would make
the suite fail on a machine without the CLI — a state `SPEC.md` requires supporting.

**No parallel groups.** Every behaviour group edits `src/changes.rs`, so no two are
independent in the sense the schema requires.

---

## Command-level checks, written out once

These are referenced by label from the tasks below. They are written here rather than
inside a table cell because a `|` cannot appear unescaped in a Markdown table and an
escaped `\|` inside an ERE means a **literal** pipe — `grep -E 'a\|b'` matches the string
`a|b` and finds nothing, so the check would pass against the very code it is meant to
catch. Every one is judged on **output emptiness or a counted minimum**, never on a
pipeline's exit status: `grep` exits 1 for no-match and 2 for a bad file, and `!` turns
both into a pass.

Extract each block to `$CHECKS/<LABEL>.sh` in task 1.1 and run the extracted, byte-identical
file thereafter, so the run and the record cannot drift.

```sh
# NOSPAWN-GREP — `cli` is the only module in the crate that spawns a process.
# Carried forward BYTE-IDENTICALLY from subprocess-seam's design.md. $SRC lets the same
# block be pointed at a copy of src/ for the negative-control runs.
SRC="${SRC:-src}"
fail() { echo "NOSPAWN FAIL: $1" >&2; exit 1; }
SPAWN_RE='process::Command|Command::new|Stdio'

# Guard A — the tree and the one allowed spawner exist. A renamed, split, or moved seam
# (src/cli/mod.rs, say) must be a deliberate update to this block, never a silent stop.
[ -d "$SRC" ] || fail "no such directory: $SRC"
[ -f "$SRC/cli.rs" ] || fail "$SRC/cli.rs missing - the exclusion has nothing to exclude"

# Guard B — the excluded file actually spawns. Without this, a tree where the seam was
# gutted (or never written) passes, and the exclusion protects nothing.
grep -qE 'process::Command|Command::new' "$SRC/cli.rs" \
  || fail "$SRC/cli.rs names no spawn API - exclusion is vacuous"

# Guard C — the searched set is the crate's real module set, not an empty list. This is
# the `test -f`-shaped guard: grep exits 2 on a missing file and `!` would pass that.
# 8 is today's count (changes, config, lib, main, resolve, schema, state, tasks); a
# module deliberately removed is a deliberate edit here.
#
# The exclusion is BY PATH, not by base name. `! -name 'cli.rs'` would silently exempt a
# future `src/ui/cli.rs` — demonstrated at planning time: a spawn planted there passed
# the base-name form with the count still reading 8.
n=$(find "$SRC" -name '*.rs' ! -path "$SRC/cli.rs" | wc -l | tr -d ' ')
[ "$n" -ge 8 ] || fail "searched only $n files under $SRC (expected >= 8)"

# The check itself. Judged on OUTPUT EMPTINESS, never on a pipeline's exit status:
# grep exits 1 for no-match and 2 for a bad file, and `!` turns BOTH into a pass, which
# is the "verification that cannot fail" defect this repository keeps finding.
# The trailing /dev/null makes grep's argument list unconditionally non-empty, so the
# GNU-xargs empty-input case cannot make grep read stdin and hang. Guard C already makes
# that unreachable; this costs nothing and removes the reliance on it.
hits=$(find "$SRC" -name '*.rs' ! -path "$SRC/cli.rs" -print0 \
       | xargs -0 -I{} grep -nE "$SPAWN_RE" {} /dev/null 2>&1 || true)
[ -z "$hits" ] || { echo "NOSPAWN FAIL: spawn API outside $SRC/cli.rs:" >&2
                    echo "$hits" >&2; exit 1; }
echo "NOSPAWN OK: $n files checked under $SRC, only $SRC/cli.rs may spawn"

# MODULE-SCOPED, stricter, and unweakened from repo-resolution and its predecessors:
# these five files name no process API at all, spawning or not. The `[ -f ... ]` guards
# are load-bearing for the same exit-code-2 reason.
for f in src/resolve.rs src/config.rs src/state.rs src/schema.rs src/tasks.rs; do
  [ -f "$f" ] || { echo "MODULE-SCOPED FAIL: $f missing" >&2; exit 1; }
done
m=$(grep -nE 'std::process|Command|Stdio' src/resolve.rs src/config.rs src/state.rs \
    src/schema.rs src/tasks.rs || true)
[ -z "$m" ] || { echo "MODULE-SCOPED FAIL:" >&2; echo "$m" >&2; exit 1; }
```

```sh
# GATE-DEFAULT — no `Default` reachable for Change, ChangeSet, ArtifactRef, or Origin.
# Half 1 of change-model's two-producer gate, carried forward from changes-from-files
# with a vacuity guard added. $F points it at a copy for the negative-control runs.
F="${F:-src/changes.rs}"
fail() { echo "GATE-DEFAULT FAIL: $1" >&2; exit 1; }

# Guard A — the file exists. grep exits 2 on a missing file and `!` would pass that.
[ -f "$F" ] || fail "$F missing - nothing to check"

# Guard B — vacuity. All four types must be declared in this file, or a search that found
# no Default found nothing because it looked in the wrong place.
grep -q '^pub struct Change {'      "$F" || fail "$F does not declare 'pub struct Change'"
grep -q '^pub struct ChangeSet {'   "$F" || fail "$F does not declare 'pub struct ChangeSet'"
grep -q '^pub struct ArtifactRef {' "$F" || fail "$F does not declare 'pub struct ArtifactRef'"
grep -q '^pub enum Origin {'        "$F" || fail "$F does not declare 'pub enum Origin'"

hits=$(grep -nE 'derive\([^)]*Default|impl +Default +for +(Change|ChangeSet|ArtifactRef|Origin)' "$F" || true)
[ -z "$hits" ] || { echo "GATE-DEFAULT FAIL: Default reachable in $F:" >&2
                    echo "$hits" >&2; exit 1; }
echo "GATE-DEFAULT OK: no Default on Change, ChangeSet, ArtifactRef, or Origin in $F"
```

```sh
# GATE-REST — no `..` functional update and no `..` rest pattern in src/changes.rs.
# The OTHER half of gate mechanism 1, and it is not implied by GATE-DEFAULT:
# `Change { name, ..other }` compiles with no `Default` anywhere in the crate.
F="${F:-src/changes.rs}"
fail() { echo "GATE-REST FAIL: $1" >&2; exit 1; }

# Guard A — the file exists (grep's exit code 2 again).
[ -f "$F" ] || fail "$F missing - nothing to check"

# Strip whole-line `//` comments: every comment in this file is whole-line, and the doc
# comments discuss `..` in prose. Then look for a `..` whose nearest preceding
# non-whitespace character is `,` or `{` — the only two positions a functional update or
# a rest pattern can occupy. A slice index such as `segment[..star]` is preceded by `[`
# and is therefore not a hit, which is this check's own discriminating positive control.
body=$(sed 's|^[[:space:]]*//.*$||' "$F")

# Guard B — vacuity. Both producers plus the merge build the type here; a check that
# found no construction at all searched the wrong file.
n=$(printf '%s\n' "$body" | grep -cE '(Change|ChangeSet|ArtifactRef|Origin::Archived)[[:space:]]*\{')
[ "$n" -ge 3 ] || fail "found only $n struct constructions in $F (expected >= 3)"

hits=$(printf '%s\n' "$body" | grep -nE '[,{][[:space:]]*\.\.' || true)
# The newline-separated form too: a line whose first non-space token is `..`, where the
# previous non-blank line ended in `,` or `{`.
multi=$(printf '%s\n' "$body" | awk '
  { line=$0; sub(/^[[:space:]]+/,"",line); sub(/[[:space:]]+$/,"",line)
    if (line ~ /^\.\./ && prev ~ /[,{]$/) printf "%d:%s\n", NR, $0
    if (line != "") prev=line }')
all=$(printf '%s\n%s\n' "$hits" "$multi" | grep -v '^$' || true)
[ -z "$all" ] || { echo "GATE-REST FAIL: rest pattern or functional update in $F:" >&2
                   echo "$all" >&2; exit 1; }
echo "GATE-REST OK: $n constructions in $F, no rest pattern and no functional update"
```

```sh
# GATE-COMPILE — mechanism 2 of the two-producer gate, and the proof that neither half
# alone suffices. Three throwaway copies of the crate; each must FAIL TO COMPILE, and
# each must fail with the SPECIFIC error code named, not merely "fail".
#   $WORK is a scratch directory. Nothing is written inside the repository.
gate_compile() { # $1 = variant label, $2 = required error code, $3 = python mutator
  d="$WORK/gate-$1"; rm -rf "$d"; mkdir -p "$d"
  cp -R Cargo.toml Cargo.lock src tests rustfmt.toml "$d/"
  ( cd "$d" && python3 -c "$3" ) || { echo "GATE-COMPILE FAIL: mutator $1 did not apply" >&2; return 1; }
  out=$( cd "$d" && cargo test --all-features --no-run 2>&1 )
  if printf '%s\n' "$out" | grep -q "^error\[$2\]"; then
    echo "GATE-COMPILE OK: variant $1 fails with $2"
  else
    echo "GATE-COMPILE FAIL: variant $1 did not produce $2" >&2
    printf '%s\n' "$out" | grep -oE 'error\[E[0-9]+\][^\n]*' | sort -u >&2
    return 1
  fi
}
# A green control FIRST: an unmutated copy must BUILD. Without it, a variant that fails
# because the copy is broken is indistinguishable from one that fails for the right
# reason, which is verification stopping short of the step it vouches for.
d="$WORK/gate-baseline"; rm -rf "$d"; mkdir -p "$d"
cp -R Cargo.toml Cargo.lock src tests rustfmt.toml "$d/"
( cd "$d" && cargo test --all-features --no-run >/dev/null 2>&1 ) \
  || { echo "GATE-COMPILE FAIL: the unmutated copy does not build" >&2; exit 1; }
echo "GATE-COMPILE OK: unmutated copy builds"
```

```sh
# NOJSON-SEAM — the seam returns stdout verbatim; every parse lives on the other side.
fail() { echo "NOJSON-SEAM FAIL: $1" >&2; exit 1; }
[ -f src/cli.rs ] || fail "src/cli.rs missing - nothing to check"
[ -f src/changes.rs ] || fail "src/changes.rs missing"
# Positive control: the crate must actually use serde_json, or its absence from cli.rs
# proves nothing at all.
grep -q 'serde_json' src/changes.rs \
  || fail "src/changes.rs does not name serde_json - the check would pass vacuously"
hits=$(grep -n 'serde_json' src/cli.rs || true)
[ -z "$hits" ] || { echo "NOJSON-SEAM FAIL: serde_json in src/cli.rs:" >&2
                    echo "$hits" >&2; exit 1; }
echo "NOJSON-SEAM OK: serde_json used in src/changes.rs, absent from src/cli.rs"
```

```sh
# NOSPAWN-RUN — the whole suite passes with npm, node, and openspec all unresolvable and
# the toolchain intact. Inherited from repo-resolution / subprocess-seam unchanged; its
# five preconditions come FIRST and must ABORT. A bare `! command -v ...` sequence
# without them exits 0 even when npm and node are still resolvable, and hiding `cargo`
# instead of the tools under test is the same defect in the other direction.
NOTOOLS=$(printf '%s\n' "$PATH" | tr ':' '\n' | while IFS= read -r d; do
  [ -n "$d" ] || continue
  if [ -x "$d/npm" ] || [ -x "$d/node" ] || [ -x "$d/openspec" ]; then continue; fi
  printf '%s\n' "$d"
done | tr '\n' ':' | sed 's/:$//')
for t in npm node openspec; do
  if env PATH="$NOTOOLS" sh -c "command -v $t" >/dev/null 2>&1; then
    echo "PRECONDITION FAILED: $t still resolvable on NOTOOLS" >&2; exit 1
  fi
done
for t in cargo rustc; do
  env PATH="$NOTOOLS" sh -c "command -v $t" >/dev/null 2>&1 || {
    echo "PRECONDITION FAILED: $t not resolvable on NOTOOLS" >&2; exit 1; }
done
env PATH="$NOTOOLS" cargo test --all-features
```

```sh
# OPENSPEC-UNTOUCHED — nothing under openspec/ changed except this change's own artifacts.
# Diffed against the BASE SHA captured in task 1.1, never against the index: this project
# commits per task group, so `git diff --exit-code` between working tree and index passes
# over the very change it exists to catch.
#
# `git diff` alone is NOT enough: it lists tracked paths only, and a file the plugin
# WRITES at runtime is untracked, so the one violation this check exists to catch would
# be invisible to it. The `ls-files --others` sweep is the half that catches it.
#
# `:(top)` anchors both pathspecs to the repository root and `git -C "$ROOT"` anchors the
# reported paths there too, so the check gives the same answer from any working directory.
[ -n "$BASE" ] || { echo "FAIL: BASE sha not set" >&2; exit 1; }
ROOT=$(git rev-parse --show-toplevel) || { echo "FAIL: not a git repo" >&2; exit 1; }
git -C "$ROOT" rev-parse --verify "$BASE^{commit}" >/dev/null \
  || { echo "FAIL: bad BASE" >&2; exit 1; }
stray=$( { git -C "$ROOT" diff --name-only "$BASE" -- ':(top)openspec/'
           git -C "$ROOT" ls-files --others --exclude-standard -- ':(top)openspec/'; } \
         | grep -v '^openspec/changes/changes-from-cli/' | sort -u || true)
[ -z "$stray" ] || { echo "FAIL: wrote inside openspec/ outside this change:" >&2
                     echo "$stray" >&2; exit 1; }
echo "OPENSPEC-UNTOUCHED OK"
```

```sh
# TESTCOUNT — a `cargo test` filter that matches NOTHING exits 0. Verified in a previous
# change: `cargo test --all-features this_name_does_not_exist` prints
# "0 passed; ... filtered out" and returns 0. Every VERIFY that is a filtered run
# therefore passes if the module is renamed, if the tests are never written, or if a typo
# is made in the filter. The gate is a counted minimum, taken from the group's own RED.
#   usage: testcount <filter> <minimum>
testcount() {
  out=$(cargo test --all-features "$1" 2>&1) || { printf '%s\n' "$out" >&2; return 1; }
  n=$(printf '%s\n' "$out" | sed -n 's/^test result: ok\. \([0-9][0-9]*\) passed.*/\1/p' \
      | awk '{t+=$1} END{print t+0}')
  [ "$n" -ge "$2" ] || { echo "TESTCOUNT FAIL: filter '$1' ran $n tests, expected >= $2" >&2
                         return 1; }
  echo "TESTCOUNT OK: filter '$1' ran $n tests (>= $2)"
}
```

```sh
# DEPS — the argued dependency set, the resolved graph, the MSRV floor, and the
# genuinely-needed experiment. Reads `cargo metadata` JSON through python3 rather than
# adding a crate to do it, exactly as subprocess-seam's DEPS does.
#   $WORK is a scratch directory; every removal experiment runs in a COPY, because
#   `cargo build` rewrites Cargo.lock during resolution before it reaches the compile
#   error the check waits for.
#
# Leg 5's guard: removing a crate the manifest does not carry must be reported as a
# FAILURE of the check, never counted as a pass — otherwise the leg silently succeeds
# against a manifest that never declared it.
```

---

## 1. Foundation: baseline, the dependency, and the extracted checks
<!-- kind: operational -->

- [ ] 1.1 CHECK: Capture the baseline that every later check measures against, and write
      each value into this file in place of the placeholder beside it.
      - `git rev-parse HEAD` → record as `BASE`. This is the commit the planning
        artifacts land on; at planning time it was `c4f88df` plus the planning commit.
        Every `OPENSPEC-UNTOUCHED` run uses this value.
      - `cargo test --all-features --lib 2>&1 | sed -n 's/^test result: ok\. \([0-9]*\)
        passed.*/\1/p'` → record as the lib-test baseline. On `main` at planning time:
        **300**. `TESTCOUNT` at the end requires **strictly more**.
      - `export CHECKS=<scratchpad>/changes-from-cli-checks` and extract each fenced block
        above to `$CHECKS/<LABEL>.sh`, byte-identically. Run the extracted files from here
        on, never a retyped copy.
      **Red when:** `BASE` is empty or not a commit (`OPENSPEC-UNTOUCHED` aborts on both);
      the baseline count is recorded higher than reality, which would make `TESTCOUNT`
      unsatisfiable rather than falsely green.

- [ ] 1.2 CHECK: Run `NOSPAWN-GREP`, `GATE-DEFAULT`, and `GATE-REST` against the tree as
      it stands, before any edit. All three must **pass**, establishing that this change
      starts from a clean gate rather than inheriting a broken one.
      Expected, and observed at planning time:
      `NOSPAWN OK: 8 files checked under src, only src/cli.rs may spawn`;
      `GATE-DEFAULT OK: no Default on Change, ChangeSet, ArtifactRef, or Origin in src/changes.rs`;
      `GATE-REST OK: 34 constructions in src/changes.rs, no rest pattern and no functional update`.
      **Red when:** the tree already carries a `Default`, a rest pattern, or a spawn API
      outside `src/cli.rs` — in which case this change is not the place to fix it and the
      finding is reported before any code is written.

- [ ] 1.3 CHECK: Run `NOJSON-SEAM` now. It must **fail** with
      `src/changes.rs does not name serde_json - the check would pass vacuously`, proving
      the positive control is load-bearing rather than decorative. Record the message.
      **Red when:** it passes today, which would mean the positive control is not wired.

- [ ] 1.4 CHANGE: Add to `Cargo.toml`:
      `serde_json = { version = "1.0.151", default-features = false, features = ["std"] }`
      — written with an explicit `features` list, `default-features = false`, and the
      version checked against the registry rather than remembered (`cargo info serde_json`
      reported `1.0.151` at planning time). Commit `Cargo.lock`.

- [ ] 1.5 VERIFY: Run `DEPS` legs 1–4 and record the output verbatim.
      - `cargo metadata --no-deps --format-version 1` → exactly **three** normal
        dependencies, `serde_json`, `toml`, `yaml-rust2`, all with
        `uses_default_features == false`; features exactly `["std"]`, `["display",
        "parse", "serde", "std"]`, and `[]` respectively.
      - `grep -n 'features = \[\]' Cargo.toml` finds the `yaml-rust2` line, so the
        explicit-empty spelling is read from the manifest text (`cargo metadata` reports
        `[]` for both spellings and cannot tell them apart).
      - `cargo build --locked` exits 0.
      - `cargo tree -e normal --target <triple>` for `aarch64-apple-darwin`,
        `x86_64-apple-darwin`, `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-gnu`
        → all four identical, and the set besides `herdr-openspec` is exactly the sixteen
        packages `plugin-build`'s delta enumerates. No `syn`, `quote`, `proc-macro2`,
        `serde_derive`, `encoding_rs`, or `ryu`.
      - MSRV: the maximum `rust_version` over the `(name, version)` pairs
        `cargo tree -e normal` reports, intersected with `cargo metadata --format-version 1`,
        is `1.85`, compared against `Cargo.toml`'s own `rust-version` read at run time
        rather than a literal in the check.
      **Red when:** a fourth dependency appears; a default is re-enabled; a proc-macro
      crate enters the graph; `serde_json` resolves a version whose MSRV exceeds the
      crate's floor. Verified at planning time in a scratch copy: the graph gains exactly
      `serde_json`, `itoa`, `memchr`, `zmij`, identical on all four triples.

- [ ] 1.6 VERIFY: Run `make check` — must still be green with the dependency added and no
      code using it yet. Commit.

---

## 2. `parse_list` — the list envelope
<!-- kind: behavior -->

- [ ] 2.1 RED: Write failing unit tests in `src/changes.rs`'s `mod tests` for:
      `a_bare_array_is_rejected_rather_than_parsed`;
      `an_empty_change_list_is_a_supported_empty_state` (parser half: zero entries, zero
      problems, a root); `a_change_entry_missing_a_required_field_is_skipped_not_fatal`
      (three entries; the middle has no `totalTasks`, another has a numeric `name`; one
      `ListEntry` survives and two problems name positions `1` and `2`);
      `empty_stdout_from_list_is_a_parse_failure` (`""` → `Err`);
      `a_list_envelope_carries_the_root_path`;
      `an_envelope_with_no_root_yields_no_root`;
      `a_root_whose_path_is_not_a_string_yields_no_root`;
      `list_entries_keep_the_payload_order` (the sort is not this function's job).
      Confirm each fails because `parse_list` does not exist, not because of a typo:
      `cargo test --all-features --lib parse_list` must report a **compile** error naming
      `parse_list`, and the count of failing tests must equal the count written.

- [ ] 2.2 GREEN: Implement `pub(crate) struct ListEntry { name: String, progress:
      tasks::Progress }`, `pub(crate) struct ListPayload { root: Option<PathBuf>, changes:
      Vec<ListEntry>, problems: Vec<String> }`, and
      `pub(crate) fn parse_list(text: &str) -> Result<ListPayload, String>`.
      `Err` for: not JSON, not an object, no `changes` key, `changes` not an array.
      Per-entry: `name` must be a string, `completedTasks` and `totalTasks` must be
      numbers representable as `usize`; anything else records one problem naming the
      entry's zero-based position and skips it. `lastModified` and `status` are read and
      discarded — `change-model` forbids storing either.

- [ ] 2.3 REFACTOR: Extract the "non-negative integer from a `Value`" helper if it is used
      more than twice; otherwise state that no refactor was needed.

- [ ] 2.4 VERIFY: `testcount 'parse_list' <count written in 2.1>` — a counted minimum, not
      a bare filtered run, because a filter matching nothing exits 0. Commit.

---

## 3. `parse_apply` — the apply payload
<!-- kind: behavior -->

- [ ] 3.1 RED: Write failing unit tests for:
      `an_apply_payload_yields_schema_name_change_dir_and_context_files`;
      `an_apply_payload_missing_context_files_is_an_error`;
      `an_apply_payload_missing_schema_name_is_an_error`;
      `an_apply_payload_missing_change_dir_is_an_error`;
      `an_error_envelope_is_an_error` (the CLI's `{"status":[{severity,code,message}]}`
      shape, which arrives on stdout with exit 1 — this test proves the parser rejects it
      rather than silently producing an empty payload should a future release exit 0);
      `context_files_values_that_are_not_string_arrays_are_rejected`;
      `an_empty_context_files_object_is_valid_and_yields_no_paths`;
      `malformed_apply_json_is_an_error`.
      Confirm the failures are missing-function failures.

- [ ] 3.2 GREEN: Implement `pub(crate) struct ApplyPayload { schema_name: String,
      change_dir: PathBuf, context_files: BTreeMap<String, Vec<PathBuf>> }` and
      `pub(crate) fn parse_apply(text: &str) -> Result<ApplyPayload, String>`.
      `schemaName` must be a non-empty string, `changeDir` a non-empty string,
      `contextFiles` an object whose every value is an array of strings. A `BTreeMap` is
      used rather than a `HashMap` so a rendered problem naming several keys is
      deterministic across runs.

- [ ] 3.3 REFACTOR: Share the "required non-empty string field" helper with `parse_list`
      if it fits, or state that none was needed.

- [ ] 3.4 VERIFY: `testcount 'parse_apply' <count>`. Commit.

---

## 4. `parse_schema_which` — the schema-directory payload
<!-- kind: behavior -->

- [ ] 4.1 RED: Write failing unit tests for:
      `a_schema_which_payload_yields_the_directory_path`;
      `a_leading_non_json_line_is_not_tolerated` (the exact string
      `"Note: Schema commands are experimental and may change.\n{\"path\":\"/x\"}"` → `Err`);
      `a_schema_which_error_body_is_an_error` (`{"error": "...", "available": [...]}`,
      which is what the CLI writes to **stdout** on exit 1);
      `malformed_schema_which_payloads_are_errors` — the four payloads `""`, `"null"`,
      `"{\"path\": 7}"`, `"{\"name\":\"x\"}"`, each asserted individually so a single
      failure names which one;
      `an_empty_path_is_an_error`;
      `source_and_shadows_are_ignored` (a payload carrying `source: "package"` and a
      non-empty `shadows` array still yields just the path).

- [ ] 4.2 GREEN: Implement `pub(crate) fn parse_schema_which(text: &str) ->
      Result<PathBuf, String>`. Parse the **whole** of `text` as one JSON document with
      `serde_json::from_str` — no line skipping, no prefix trimming. Require an object
      with a non-empty string `path`. Ignore every other key.

- [ ] 4.3 REFACTOR: None expected; state so explicitly if none was made.

- [ ] 4.4 VERIFY: `testcount 'schema_which' <count>`. Commit.

---

## 5. `cli_artifacts` — placing paths at schema positions
<!-- kind: behavior -->

- [ ] 5.1 RED: Write failing unit tests for:
      `an_omitted_context_files_key_becomes_an_empty_path_list_at_its_position` (a `tdd`-
      shaped five-artifact schema with keys for three);
      `a_multi_file_artifact_keeps_the_cli_list_in_the_cli_order` (two absolute paths,
      asserted in order, asserted absolute, asserted not joined onto any change dir);
      `a_context_files_key_naming_no_schema_artifact_is_ignored` (extra key `legacy` →
      absent from `artifacts`, one problem naming `legacy`);
      `a_duplicate_schema_id_gives_both_positions_the_same_paths` (ids `zeta`, `alpha`,
      `zeta`; positions 0 and 2 carry the entry, position 1 carries none; the list is
      neither de-duplicated nor truncated);
      `an_empty_context_files_map_yields_every_artifact_with_no_paths`;
      `artifact_order_is_the_schemas_declared_order_not_the_map_order` — build the
      `BTreeMap` so its own key order (`alpha`, `design`, `proposal`, …) differs from the
      schema's declared order, and assert the schema's. Without this the test would pass
      for an implementation that iterated the map.

- [ ] 5.2 GREEN: Implement `pub(crate) fn cli_artifacts(schema: &schema::Schema,
      context_files: &BTreeMap<String, Vec<PathBuf>>) -> (Vec<ArtifactRef>, Vec<String>)`.
      Walk `schema.artifacts` in declared order; each position's `paths` is
      `context_files.get(&artifact.id).cloned().unwrap_or_default()`. After the walk,
      record one problem per `context_files` key no schema artifact declares.

- [ ] 5.3 REFACTOR: State whether any was needed.

- [ ] 5.4 VERIFY: `testcount 'cli_artifacts' <count>`. Commit.

---

## 6. `join_artifacts` — the positional cross-producer join
<!-- kind: behavior -->

- [ ] 6.1 RED: Write failing unit tests, one per rule, all against hand-built vectors so
      no CLI and no filesystem is involved:
      `equal_length_lists_with_equal_ids_take_the_cli_paths_positionally`;
      `a_duplicate_id_is_joined_by_index_rather_than_collapsed` — ids `zeta`, `alpha`,
      `zeta` in both lists, with the CLI's index 2 carrying a path index 0 does not, and
      the assertion is that index 2 keeps **its own** path. An id-keyed implementation
      gives both positions index 0's paths and fails here; this is the test that
      discriminates the required join from the forbidden one;
      `an_empty_file_list_takes_the_cli_list`;
      `an_empty_cli_list_keeps_the_file_list`;
      `two_empty_lists_join_to_an_empty_list`;
      `differing_lengths_keep_the_file_list_and_name_both_counts` (problem contains `5`
      and `3`);
      `a_differing_id_at_one_index_keeps_the_file_list_and_names_the_index` — four
      entries differing at indices 2 **and** 3, asserting the problem names index `2`,
      `design`, and `plan`, and does **not** name index `3`. Without the second
      difference the "first differing index" clause is untested;
      `the_join_never_reads_a_path_as_a_key` — file and CLI lists whose ids agree at every
      position but whose paths share no string at all; the result is the CLI's, proving
      no path comparison gates the join.

- [ ] 6.2 GREEN: Implement `pub(crate) fn join_artifacts(file: &[ArtifactRef], cli:
      &[ArtifactRef]) -> (Vec<ArtifactRef>, Option<String>)` applying the six rules in
      `change-merge`'s spec, in order.

- [ ] 6.3 REFACTOR: State whether any was needed.

- [ ] 6.4 VERIFY: `testcount 'join_artifacts' <count>`. Commit.

---

## 7. Schema resolution through the CLI fallback tier
<!-- kind: behavior -->

- [ ] 7.1 RED: Write failing unit tests. Every one uses `cli::FakeCli` plus a real
      `testutil::ScratchDir` holding real `schema.yaml` files — **no process is spawned**,
      because the fake answers `["schema","which",…]` with a payload whose `path` names a
      scratch directory that genuinely holds the file `schema::load_dir` then reads:
      `a_schema_absent_from_the_repository_is_loaded_from_the_directory_the_cli_names`;
      `a_vendored_schema_never_reaches_the_cli` — register **no** `schema which` response;
      the fake panics on an unregistered pair, so a test that passes proves the call was
      never made;
      `an_unreadable_vendored_schema_is_not_repaired_by_the_cli` — `schema.yaml` created
      as a **directory**, no `schema which` registration;
      `an_invalid_vendored_schema_is_not_repaired_by_the_cli`;
      `a_which_path_naming_a_directory_with_no_schema_yaml_stops_the_tier` — assert
      exactly one recorded `schema which` call;
      `a_which_failure_is_a_problem_naming_the_vector_and_the_exit_code`;
      `three_changes_sharing_one_unvendored_schema_ask_the_cli_once`;
      `a_failed_lookup_is_cached_rather_than_retried_per_change`;
      `two_different_schema_names_are_asked_for_separately`.

- [ ] 7.2 GREEN: Implement the per-call schema resolver: a `HashMap<String, CachedCliSchema>`
      owned by the `from_cli` call (never a `static` — `resolve::BinCache`'s reason: the
      suite runs this crate's tests in parallel threads of one process). On a miss, call
      `schema::load(repo, name)`; on `Ok` cache it; on `Err(NotVendored)` run
      `["schema", "which", name, "--json"]`, `parse_schema_which`, then
      `schema::load_dir(dir, name)`; on `Err(Unreadable)` or `Err(Invalid)` cache the
      failure without calling the CLI. Cache failures as well as successes.

- [ ] 7.3 CHECK: Confirm no process API name entered `src/changes.rs` while wiring the
      fallback — run `NOSPAWN-GREP`. **Red when:** the tier was implemented by reaching for
      `Command` rather than the trait object.

- [ ] 7.4 VERIFY: `testcount 'schema_fallback' <count>`. Commit.

---

## 8. `from_cli` — the composition
<!-- kind: behavior -->

- [ ] 8.1 RED: Write failing unit tests for:
      `a_two_change_repository_drives_exactly_three_invocations` — assert
      `FakeCli::calls()` **equals** the exact three-element vector, not merely contains
      them, so an extra `status` call fails;
      `no_status_invocation_is_made_even_when_an_apply_call_fails` — no `status`
      registration at all;
      `the_argument_vector_carries_no_sort_flag` — assert the recorded list vector equals
      `["list", "--json"]` exactly;
      `progress_is_read_from_the_list_payload_pair` — the apply payload deliberately
      carries a conflicting `progress`, and `0/0` must appear nowhere in the result;
      `the_most_recently_modified_default_order_is_replaced_by_byte_order`;
      `case_and_digits_order_by_byte_not_by_locale` — `Beta` before `alpha`, which a
      `localeCompare` sort reverses;
      `an_absent_openspec_binary_yields_an_empty_result_and_one_problem`;
      `a_schema_the_cli_rejects_removes_one_change_and_keeps_the_others` — the problem
      names the change, the vector, and exit code `1`, and asserts the problem does **not**
      contain any CLI message text, because stderr was empty;
      `malformed_json_from_a_single_apply_call_is_contained_to_that_change`;
      `an_apply_payload_missing_context_files_is_a_per_change_failure`;
      `empty_stdout_from_list_is_a_parse_failure_not_an_empty_repository` — asserts one
      problem, and asserts the empty-list case records none, so the two are distinguished;
      `a_mismatched_root_discards_the_whole_cli_result` — asserts exactly one recorded
      call, so the guard runs before the per-change calls;
      `a_symlinked_repository_root_is_not_a_disagreement` — a real scratch symlink;
      `an_envelope_with_no_root_is_treated_as_a_disagreement`;
      `every_change_from_cli_satisfies_the_shared_invariants` — every produced `Change`
      through `conformance::assert_invariants`.

- [ ] 8.2 GREEN: Implement `pub struct CliChanges` and
      `pub fn from_cli(cli: &dyn cli::OpenspecCli, repo: &Path) -> CliChanges`, plus
      `pub(crate) fn same_directory(a: &Path, b: &Path) -> bool` (canonicalize both;
      compare the canonical forms when both succeed, otherwise compare as given).
      Order of operations: `list --json` → parse → root guard → per-change apply → schema
      resolution → `cli_artifacts` → `Change` → sort by name in byte order.
      Build each `Change` naming all seven fields; no `Default`, no `..`.

- [ ] 8.3 REFACTOR: Clean up while green; state explicitly if none was needed.

- [ ] 8.4 VERIFY: RED then GREEN for
      `a_full_from_cli_run_leaves_the_tree_byte_identical` — `testutil::snapshot` over a
      scratch tree and `testutil::shallow_snapshot` over the process's own working
      directory, taken around one call covering a successful change, a change whose apply
      call failed, a schema resolved through the CLI fallback tier, and a malformed
      payload. The cwd snapshot is shallow for the reason `cli.rs`'s equivalent test
      records: under `cargo test` the real cwd is this repository, whose `target/` holds
      tens of thousands of files.

- [ ] 8.5 VERIFY: `testcount 'from_cli' <count>` and run `NOSPAWN-GREP`. Commit.

---

## 9. `merge` — layering CLI over files
<!-- kind: behavior -->

- [ ] 9.1 RED: Write failing unit tests, all pure (hand-built `ChangeSet` and
      `CliChanges`, no CLI, no filesystem):
      `the_cli_schema_progress_and_artifacts_replace_the_files`;
      `the_merged_dir_comes_from_the_file_change`;
      `a_change_only_the_cli_reported_is_inserted_in_name_order`;
      `a_change_only_the_file_producer_saw_survives_the_merge` (and records no problem);
      `an_empty_cli_result_leaves_the_file_result_intact` — assert the merged set `==` the
      input set except for the appended problems, which a weaker per-field assertion
      would not catch;
      `archived_changes_pass_through_untouched` — including the case where the CLI reports
      an active change of the same name;
      `a_file_side_message_survives_beside_a_corrected_artifact_list`;
      `duplicate_messages_from_both_producers_are_collapsed` — two entries, file order
      preserved;
      `a_join_problem_is_appended_after_both_producers_problems` — three entries, join
      last;
      `every_merged_value_satisfies_the_shared_invariants` — all seven values through
      `conformance::assert_invariants`.

- [ ] 9.2 GREEN: Implement `pub fn merge(files: ChangeSet, cli: CliChanges) -> ChangeSet`
      per `change-merge`'s field table: pair by name, `dir` from the file change, `schema`
      and `progress` from the CLI change, `artifacts` from `join_artifacts`, `problems`
      concatenated file-first then CLI then join, exactly-equal strings collapsed to the
      first occurrence. Union the two name sets, re-sort by name in byte order, pass
      `archived` through unchanged, and concatenate the two problem lists onto
      `ChangeSet::problems`.

- [ ] 9.3 CHECK: Contract gate — re-read `openspec/specs/change-model/spec.md` and confirm
      the merged `Change` still carries exactly seven fields, no `status`, no ratio, no
      formatted string, no `lastModified`, and no producer discriminant, and that
      `archived` is still a separate vector rather than a filter over one list.

- [ ] 9.4 VERIFY: `testcount 'merge' <count>`. Commit.

---

## 10. Architectural checks and their negative controls
<!-- kind: operational -->

- [ ] 10.1 VERIFY: `NOSPAWN-GREP` against `src` — must pass, reporting at least eight
      files. Then run it four more times against scratch copies of `src/` to prove it is
      not decoration, recording each message:
      (a) `Command::new("openspec")` planted in `src/changes.rs` → `NOSPAWN FAIL: spawn API
      outside .../cli.rs:` naming that line;
      (b) `cli.rs` deleted → `NOSPAWN FAIL: .../cli.rs missing - the exclusion has nothing
      to exclude`;
      (c) `cli.rs` emptied of its spawn → `NOSPAWN FAIL: .../cli.rs names no spawn API -
      exclusion is vacuous`;
      (d) the spawn planted at `ui/cli.rs` rather than the top level → still fails, proving
      the exclusion is by path and not by base name.
      **Red when:** any of (a)–(d) passes.

- [ ] 10.2 VERIFY: `GATE-DEFAULT` against `src/changes.rs` — must pass. Then four negative
      controls, each recorded: `#[derive(..., Default)]` on `Change` → exit 1;
      `impl Default for Origin` appended → exit 1; a missing file → exit 1 naming the
      missing file; a file that declares none of the four types → exit 1 naming the
      vacuity. All four were demonstrated red at planning time against the pre-change tree.

- [ ] 10.3 VERIFY: `GATE-REST` against `src/changes.rs` — must pass, and its message must
      now report **more** constructions than the 34 recorded at planning time, since
      `from_cli` and `merge` both build the type. Then five negative controls, each
      recorded: an inline `Change { name: …, ..other }` → exit 1; the same spread over
      lines → exit 1; a `let Change { name, .. }` pattern → exit 1; a missing file → exit 1;
      a file with no constructions → exit 1 naming the vacuity. All five were demonstrated
      red at planning time; the green run on the real file is itself discriminating,
      because that file contains a `segment[..star]` slice index the check must not fire on.

- [ ] 10.4 VERIFY: `GATE-COMPILE` — the green control first (an unmutated copy builds),
      then three mutated copies, each required to produce a **specific** error code:
      (a) a field added to `Change`, nothing else → must produce `E0027` (the pattern in
      `conformance::assert_invariants` does not mention it) **and** `E0063` at every
      construction site, which after this change includes `from_cli`'s and `merge`'s;
      (b) the same field plus `#[derive(Default)]` on `Change` and `..Default::default()`
      at every construction site → must **still** produce `E0027`, proving mechanism 2
      catches what mechanism 1 misses;
      (c) the same field, no `Default`, but a `..` rest pattern added to
      `assert_invariants` → must **still** produce `E0063`, proving mechanism 1 catches
      what mechanism 2 misses.
      **Red when:** any variant compiles, or fails with some other error code — the
      assertion is on the code, not on "it failed". Verified against the pre-change tree at
      planning time: (a) gave `E0027` plus `E0063` at five sites; (b) gave `E0027`; (c)
      gave `E0063` at five sites.
      Note deliberately **not** relied on: `tasks::Progress` implements no `Default`, so
      variant (b) also raises `E0277`. That is an accident of a sibling type and could
      vanish; the assertion is on `E0027` alone.

- [ ] 10.5 VERIFY: `NOJSON-SEAM` — must now **pass**, reporting `serde_json used in
      src/changes.rs, absent from src/cli.rs`, having failed in task 1.3 before the crate
      used it. Then one negative control: a scratch copy of `src/cli.rs` with a
      `use serde_json::Value;` line → the check exits 1.

- [ ] 10.6 VERIFY: `NOSPAWN-RUN` — the whole suite on a `PATH` from which every directory
      holding `npm`, `node`, or `openspec` has been removed, with the five preconditions
      passing first (all three unresolvable, `cargo` and `rustc` still resolvable). Every
      test must pass, including this change's own. **Red when:** any test in this change
      reached the real `openspec` binary.

- [ ] 10.7 VERIFY: `OPENSPEC-UNTOUCHED` with `BASE` from task 1.1 — must report
      `OPENSPEC-UNTOUCHED OK`. Then one negative control: create
      `openspec/specs/planted-probe.md`, re-run, confirm it **fails** naming that path,
      then delete it and confirm the check passes again. **Red when:** the untracked sweep
      is missing, which the planted file is exactly what detects.

- [ ] 10.8 VERIFY: `DEPS` leg 5 — the genuinely-needed experiment, run three times in
      **copies**: remove `toml` → `cargo build` fails; restore, remove `yaml-rust2` →
      fails; restore, remove `serde_json` → fails, and the failure names `changes`.
      Confirm the guard fires when asked to remove a crate the manifest does not carry
      (try removing `notacrate` → the check reports a failure of itself, not a pass).
      Confirm the working tree is byte-identical afterwards, `Cargo.lock` included.

---

## 11. Change Review
<!-- kind: operational -->

- [ ] 11.1 CHECK: Dispatch an independent reviewer — an agent that did **not** write the
      implementation and is **not** a fork of the implementing session — against
      `proposal.md`, all four spec files, `design.md`, `tasks.md`, and the diff. Give it
      the concentration points from `openspec/config.yaml` → `rules.tasks`, and these two
      first: (1) for every spec scenario, name the test that would go red if the behaviour
      were deleted; (2) both halves of the two-producer gate, and whether any check would
      still pass if one half were removed.

- [ ] 11.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
      one-line reason, note each SUGGESTION, and re-run the affected tests.

- [ ] 11.3 VERIFY: Confirm no blocking or unowned finding remains, and record the
      finding counts by severity in this file.

---

## 12. Documentation
<!-- kind: operational -->

- [ ] 12.1 Rewrite in `SPEC.md` → Data layer → Resolution chain → **Changes** (audience:
      every later change): replace the sentence describing `openspec list --json`'s output
      with the real envelope `{"changes": [...], "root": {"path", "source"}}`, and add that
      the CLI's own `--sort name` is `localeCompare` and therefore cannot be used to obtain
      the byte order both producers must share. Durable because two later changes read this
      paragraph as the contract and the current text would send them to a bare array.

- [ ] 12.2 Rewrite in `SPEC.md` → Data layer → Dual-source model (audience: every later
      change): state that the CLI-side task counts come from `list --json`'s
      `completedTasks`/`totalTasks` and **not** from `instructions apply --json`'s
      `progress`, which resolves `apply.tracks` without globbing and therefore disagrees
      whenever `tracks` is a glob. Durable because the whole model rests on the two
      producers reporting one number.

- [ ] 12.3 Add to `SPEC.md` → Data layer → Resolution chain → **Artifact files**
      (audience: `live-refresh` and any later CLI consumer): the plugin runs
      `instructions apply` and **not** `status --change`, and why — same
      `resolveArtifactOutputs`, and `status`'s `artifacts` array is in topological build
      order rather than the schema's declared order, so it is not a source for a
      positional join.

- [ ] 12.4 Add three rows to `SPEC.md` → Degraded states (audience: `degraded-states`,
      which audits that table end to end): a schema declaring the same artifact id twice
      (the CLI rejects the schema outright, so the change is permanently file-mode); a CLI
      reporting a repository root other than the resolved one (the whole CLI result is
      discarded); and a CLI command failing (the reason is unavailable, because the CLI
      writes its diagnostic to stdout and the seam's `Failed` carries stderr only).

- [ ] 12.5 Rewrite in `openspec/IMPLEMENTATION-ORDER.md` → Phase 3 → the
      `changes-from-cli` row (audience: whoever reads the roadmap next): drop
      `openspec status --change <n> --json` from the list of parsed commands and record in
      one clause why. Rewrite rather than append — leaving the old list beside a correction
      is what makes a roadmap stop being read.

- [ ] 12.6 Rewrite in `AGENTS.md` → **Current repo state** (audience: every future
      session): fold `changes-from-cli` into the landed list and state in one clause what
      it added, replacing the "the CLI path is not built yet" implication rather than
      appending a paragraph beside it. Net addition must stay under ten lines.

- [ ] 12.7 Rewrite in `AGENTS.md` → **Architecture rules** (audience: every future
      session): the existing "Nothing spawns a process outside `cli`" bullet gains one
      clause naming that the seam also parses nothing — `serde_json` must not appear in
      `src/cli.rs` — and the "Checkbox counting follows the OpenSpec CLI's rule exactly"
      bullet gains the `list --json`-not-apply-`progress` clause. Edit both in place; add
      no new bullet.

---

## 13. Lint & Verify
<!-- kind: operational -->

- [ ] 13.1 CHECK: Inspect the intended verification commands and affected tiers. The
      gated code is all of `src/changes.rs`; the affected tier is the unit tier plus the
      command-level checks in group 10. Confirm `make check` is the composite and that
      each sub-command is also run individually below, so a failure names itself.

- [ ] 13.2 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.

- [ ] 13.3 VERIFY: `cargo fmt --all -- --check` — clean.

- [ ] 13.4 VERIFY: `cargo test --all-features` — green. (Rust's type checker runs as part
      of `cargo test` and `cargo clippy`; there is no separate type-check command in this
      repository.)

- [ ] 13.5 VERIFY: `cargo llvm-cov --fail-under-lines 80` — the floor is 80% and is never
      lowered, waived, or excluded. `main` stands at 98.67% over 5499 lines; record the
      new figure. **Red when:** the new code is added without tests, which is what the
      floor exists to catch.

- [ ] 13.6 VERIFY: `TESTCOUNT` over the whole `--lib` run — strictly more than the 300
      recorded in task 1.1.

- [ ] 13.7 VERIFY: `make check` — the single composite gate, exit 0. If it fails, name the
      failing sub-command rather than summarising.

- [ ] 13.8 VERIFY: `openspec validate changes-from-cli --strict` — valid.
