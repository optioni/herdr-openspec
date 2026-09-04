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

## Test-module convention — read before writing any test

`src/changes.rs`'s `mod tests` is flat today (86 functions, no submodules), and libtest
matches a filter against the **full path** of a test. Each behaviour group below therefore
nests its tests in a named submodule of `mod tests`, and every `testcount` gate and every
`Command` cell in design.md's verification matrix addresses tests through that path:

| Group | Module | Filter |
|---|---|---|
| 2 | `mod list_json` | `list_json::` |
| 3 | `mod apply_json` | `apply_json::` |
| 4 | `mod which_json` | `which_json::` |
| 5 | `mod cli_artifacts` | `cli_artifacts::` |
| 6 | `mod join_artifacts` | `join_artifacts::` |
| 7 | `mod schema_fallback` | `schema_fallback::` |
| 8 | `mod from_cli` | `from_cli::` |
| 9 | `mod merge` | `merge::` |

Without this, a filter such as `parse_list` matches **zero** of the test names its own
group prescribes, and `cargo test` exits 0 on a filter that matches nothing — which is the
"verification that cannot fail" defect in its purest form.

---

## Command-level checks, written out once

These are referenced by label from the tasks below. They are written here rather than
inside a table cell because a `|` cannot appear unescaped in a Markdown table and an
escaped `\|` inside an ERE means a **literal** pipe — `grep -E 'a\|b'` matches the string
`a|b` and finds nothing, so the check would pass against the very code it is meant to
catch. Every one is judged on **output emptiness, a counted minimum, or a specific error
code**, never on a bare pipeline exit status: `grep` exits 1 for no-match and 2 for a bad
file, and `!` turns both into a pass.

Extract each block to `$CHECKS/<LABEL>.<ext>` in task 1.1 and run the extracted,
byte-identical file thereafter, so the run and the record cannot drift.

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

`GATE-MECH1.py` — mechanism 1 of `change-model`'s two-producer gate. Written in Python
rather than shell because a shell version was **defeated at planning time** by three real,
compiling, formatter-clean forms: a `// comment` between the comma and the `..`, a
`/* block comment */` in the same position, and `impl std::default::Default for Change`.
It strips comments properly (leaving string and character literals intact, and preserving
byte offsets so line numbers stay exact) and searches every file under `src/`, because an
`impl Default for Change` is legal in any file of the crate.

```python
#!/usr/bin/env python3
"""GATE-MECH1 - mechanism 1 of change-model's two-producer gate.

Half A: no `Default` is reachable for Change, ChangeSet, ArtifactRef, or Origin,
        anywhere under the source tree - not just in changes.rs, since
        `impl Default for Change` is legal in any file of the crate.
Half B: no `..` functional update and no `..` rest pattern in changes.rs.

    usage: python3 GATE-MECH1.py [SRC_DIR]      (default: src)
    exit 0 only when BOTH halves pass.
"""
import re
import sys
from pathlib import Path

TYPES = ("Change", "ChangeSet", "ArtifactRef", "Origin")


def strip_comments(text: str) -> str:
    """Replace every comment with an equal number of spaces/newlines, so byte
    offsets - and therefore reported line numbers - stay exact. String and char
    literals are left intact; a comment marker inside one is not a comment."""
    out = []
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if c == '"':                                   # string literal
            out.append(c); i += 1
            while i < n:
                if text[i] == "\\" and i + 1 < n:
                    out.append(text[i:i + 2]); i += 2; continue
                out.append(text[i])
                if text[i] == '"':
                    i += 1; break
                i += 1
            continue
        if c == "'" and i + 1 < n:                     # char literal or lifetime
            m = re.match(r"'(?:\\.|[^\\'])'", text[i:])
            if m:
                out.append(m.group(0)); i += len(m.group(0)); continue
            out.append(c); i += 1; continue
        if text.startswith("//", i):                   # line comment
            j = text.find("\n", i)
            j = n if j == -1 else j
            out.append(" " * (j - i)); i = j; continue
        if text.startswith("/*", i):                   # block comment, nesting
            depth, j = 1, i + 2
            while j < n and depth:
                if text.startswith("/*", j):
                    depth += 1; j += 2
                elif text.startswith("*/", j):
                    depth -= 1; j += 2
                else:
                    j += 1
            out.append("".join(ch if ch == "\n" else " " for ch in text[i:j]))
            i = j; continue
        out.append(c); i += 1
    return "".join(out)


def lineno(text: str, pos: int) -> int:
    return text.count("\n", 0, pos) + 1


def fail(msg: str) -> None:
    print(f"GATE-MECH1 FAIL: {msg}", file=sys.stderr)
    sys.exit(1)


src = Path(sys.argv[1] if len(sys.argv) > 1 else "src")
if not src.is_dir():
    fail(f"no such directory: {src}")
files = sorted(src.rglob("*.rs"))
# Vacuity guard A - the searched set is the crate's real module set, not an
# empty list. 8 is today's count; a module deliberately removed is a deliberate
# edit here.
if len(files) < 8:
    fail(f"searched only {len(files)} .rs files under {src} (expected >= 8)")

changes = src / "changes.rs"
if not changes.is_file():
    fail(f"{changes} missing - nothing to check")
changes_body = strip_comments(changes.read_text())

# Vacuity guard B - all four types must be declared where we expect them, or a
# search that found no Default found nothing because it looked in the wrong place.
for decl in ("pub struct Change {", "pub struct ChangeSet {",
             "pub struct ArtifactRef {", "pub enum Origin {"):
    if decl not in changes_body:
        fail(f"{changes} does not declare '{decl}'")

# --- Half A: no Default for the four types, anywhere under src/ --------------
impl_re = re.compile(
    r"impl\b[^;{]*?\b(?:std::default::|core::default::)?Default\s+for\s+"
    r"(?:crate::)?(?:changes::)?(" + "|".join(TYPES) + r")\b")
decl_re = re.compile(r"(?:pub\s+)?(?:struct|enum)\s+(" + "|".join(TYPES) + r")\b")

hits_a = []
for f in files:
    body = strip_comments(f.read_text())
    for m in impl_re.finditer(body):
        hits_a.append(f"{f}:{lineno(body, m.start())}: {m.group(0).strip()}")
    for m in decl_re.finditer(body):
        # Walk back over the attribute block immediately above the declaration,
        # so a multi-line `#[derive(\n ... Default,\n)]` is caught too.
        head = body[:m.start()]
        block = head[head.rfind("\n\n") + 1:] if "\n\n" in head else head
        for d in re.finditer(r"#\[derive\(([^)]*)\)\]", block, re.S):
            if re.search(r"\bDefault\b", d.group(1)):
                hits_a.append(
                    f"{f}:{lineno(body, m.start())}: derive(Default) on {m.group(1)}")
if hits_a:
    print("GATE-MECH1 FAIL (half A): Default reachable for a Change type:",
          file=sys.stderr)
    for h in hits_a:
        print("  " + h, file=sys.stderr)
    sys.exit(1)

# --- Half B: no `..` functional update and no `..` rest pattern in changes.rs -
# A `..` whose nearest preceding non-whitespace character is `,` or `{` is a
# functional update or a rest pattern; those are the only two positions either
# can occupy. `segment[..star]` is preceded by `[` and is therefore not a hit -
# that slice index is this half's own discriminating positive control.
ctor_re = re.compile(r"(?:Change|ChangeSet|ArtifactRef|Origin::Archived)\s*\{")
n_ctor = len(ctor_re.findall(changes_body))
if n_ctor < 3:
    fail(f"found only {n_ctor} struct constructions in {changes} (expected >= 3)")

hits_b = [f"{changes}:{lineno(changes_body, m.start())}: "
          f"{changes_body[m.start():m.start() + 40].strip()!r}"
          for m in re.finditer(r"[,{]\s*\.\.", changes_body)]
if hits_b:
    print("GATE-MECH1 FAIL (half B): rest pattern or functional update:",
          file=sys.stderr)
    for h in hits_b:
        print("  " + h, file=sys.stderr)
    sys.exit(1)

print(f"GATE-MECH1 OK (half A): no Default for {', '.join(TYPES)} "
      f"in {len(files)} files under {src}")
print(f"GATE-MECH1 OK (half B): {n_ctor} constructions in {changes}, "
      f"no rest pattern and no functional update")
```

`defeat_mech1.py` — the mutator `GATE-MECH2`'s variant (b) uses to defeat mechanism 1 in a
throwaway copy. It finds construction sites by asking the **compiler** for them (`E0063`
spans) rather than by pattern-matching, so it cannot miss a literal a regex would not
reach — and, importantly, does not touch `-> Change {`, which is a function body.

```python
#!/usr/bin/env python3
"""Defeat mechanism 1 of the two-producer gate in a THROWAWAY copy of the crate:
give `Change` a `Default` and fill every `Change` literal from it, so no `E0063`
remains. Run from the copy's root, after a field has been added to `Change`."""
import json
import re
import subprocess
import sys

for path, old, new in [
    ("src/changes.rs",
     "#[derive(Debug, Clone, PartialEq, Eq)]\npub struct Change {",
     "#[derive(Debug, Clone, PartialEq, Eq, Default)]\npub struct Change {"),
    ("src/changes.rs",
     "#[derive(Debug, Clone, PartialEq, Eq)]\npub enum Origin {",
     "#[derive(Debug, Clone, PartialEq, Eq, Default)]\npub enum Origin {\n    #[default]"),
]:
    s = open(path).read()
    if s.count(old) != 1:
        sys.exit(f"defeat_mech1: anchor not unique in {path}: {old!r}")
    open(path, "w").write(s.replace(old, new, 1))

# `Change` derives Default, so every field type must too. `Progress` does not - that
# is an accident of a sibling type, not part of the gate, so the mutator supplies it
# rather than letting an E0277 mask the E0027 under test.
s = open("src/tasks.rs").read()
s2 = re.sub(r"(#\[derive\()([^)]*?)(\)\]\npub struct Progress \{)", r"\1\2, Default\3",
            s, count=1)
if s2 == s:
    sys.exit("defeat_mech1: could not add Default to tasks::Progress")
open("src/tasks.rs", "w").write(s2)


def e0063_sites():
    """(file, byte_start) for every E0063 the compiler reports."""
    out = subprocess.run(["cargo", "build", "--tests", "--message-format=json"],
                         capture_output=True, text=True).stdout
    sites = set()
    for line in out.splitlines():
        try:
            m = json.loads(line)
        except ValueError:
            continue
        d = m.get("message")
        if not d or not d.get("code") or d["code"]["code"] != "E0063":
            continue
        for sp in d["spans"]:
            sites.add((sp["file_name"], sp["byte_start"]))
    return sites


for _ in range(10):
    sites = e0063_sites()
    if not sites:
        print("defeat_mech1: no E0063 remains - mechanism 1 is defeated")
        sys.exit(0)
    by_file = {}
    for f, b in sites:
        by_file.setdefault(f, []).append(b)
    for f, offsets in by_file.items():
        raw = open(f, "rb").read()
        # Insert into the LAST site first, so earlier byte offsets stay valid.
        for b in sorted(offsets, reverse=True):
            open_brace = raw.index(b"{", b)
            depth, i = 1, open_brace + 1
            while depth:
                if raw[i:i + 1] == b"{":
                    depth += 1
                elif raw[i:i + 1] == b"}":
                    depth -= 1
                i += 1
            raw = raw[:i - 1] + b"..Default::default()\n" + raw[i - 1:]
        open(f, "wb").write(raw)
sys.exit("defeat_mech1: E0063 sites did not converge after 10 passes")
```

```sh
# GATE-MECH2 — mechanism 2 of change-model's two-producer gate, and the proof that
# neither mechanism alone is sufficient. Four throwaway copies of the crate: a green
# control that must BUILD, then three mutants that must each fail with the SPECIFIC
# codes named and NOT with the code the other mechanism would have produced.
# Asserting on the codes, not on "it failed", is what stops a copy broken for an
# unrelated reason from being read as evidence — an earlier draft of this block was
# defeated exactly that way, by an E0277 that masked the E0027 under test.
#   env: WORK   a scratch directory (required, guarded — an unset WORK would rm -rf /)
#        MUT    the directory holding defeat_mech1.py (default: .)
set -u
: "${WORK:?GATE-MECH2 FAIL: WORK (a scratch directory) must be set}"
[ -d "$WORK" ] || { echo "GATE-MECH2 FAIL: WORK=$WORK is not a directory" >&2; exit 1; }
MUT="${MUT:-.}"
[ -f "$MUT/defeat_mech1.py" ] || { echo "GATE-MECH2 FAIL: $MUT/defeat_mech1.py missing" >&2; exit 1; }
[ -f Cargo.toml ] && [ -d src ] || { echo "GATE-MECH2 FAIL: run from the crate root" >&2; exit 1; }

ADD_FIELD='
p="src/changes.rs"; s=open(p).read()
old="    pub problems: Vec<String>,\n}\n\n/// Every change this plugin found"
assert s.count(old)==1, "anchor for the added field not found"
open(p,"w").write(s.replace(old,"    pub problems: Vec<String>,\n    pub planted: u8,\n}\n\n/// Every change this plugin found",1))
'
ADD_REST='
p="src/changes.rs"; s=open(p).read()
old="            progress: _,\n            problems,\n        } = change;"
assert s.count(old)==1, "anchor for the rest pattern not found"
open(p,"w").write(s.replace(old,"            progress: _,\n            problems,\n            ..\n        } = change;",1))
'

copy() {
  d="$WORK/mech2-$1"; rm -rf "$d"; mkdir -p "$d" || return 1
  cp -R Cargo.toml Cargo.lock src tests rustfmt.toml "$d/" || return 1
  printf '%s' "$d"
}

# Green control FIRST. Without it, a mutant that fails because the copy is broken is
# indistinguishable from one that fails for the right reason — which is verification
# stopping short of the step it vouches for.
d=$(copy baseline) || { echo "GATE-MECH2 FAIL: could not copy the crate" >&2; exit 1; }
( cd "$d" && cargo test --all-features --no-run >/dev/null 2>&1 ) \
  || { echo "GATE-MECH2 FAIL: the unmutated copy does not build" >&2; exit 1; }
echo "GATE-MECH2 OK (control): the unmutated copy builds"

codes() { printf '%s\n' "$1" | grep -oE '^error\[E[0-9]+\]' | sort -u | tr '\n' ' '; }

# $1 label, $2 codes that MUST appear, $3 codes that must NOT, $4.. mutator commands
variant() {
  label=$1; want=$2; forbid=$3; shift 3
  d=$(copy "$label") || { echo "GATE-MECH2 FAIL: could not copy for $label" >&2; exit 1; }
  for mut in "$@"; do
    ( cd "$d" && eval "$mut" ) >/dev/null 2>&1 \
      || { echo "GATE-MECH2 FAIL: mutator for variant $label did not apply" >&2; exit 1; }
  done
  out=$( cd "$d" && cargo test --all-features --no-run 2>&1 )
  got=$(codes "$out")
  for c in $want; do
    case " $got " in *" error[$c] "*) ;; *)
      echo "GATE-MECH2 FAIL: variant $label must produce $c; produced: $got" >&2; exit 1;; esac
  done
  for c in $forbid; do
    case " $got " in *" error[$c] "*)
      echo "GATE-MECH2 FAIL: variant $label must NOT produce $c; produced: $got" >&2; exit 1;; esac
  done
  echo "GATE-MECH2 OK (variant $label): produced [$got], required [$want], forbade [${forbid:-none}]"
}

# (a) a field added and nothing else: BOTH mechanisms must fire.
variant a "E0027 E0063" "" "python3 -c '$ADD_FIELD'"
# (b) mechanism 1 fully defeated — Default derived and every Change literal filled
#     from it, driven by the compiler's own E0063 spans so no literal is missed —
#     and E0063 must be GONE. Mechanism 2 must still fire. This is the proof that
#     "no Default" alone would not hold the gate.
variant b "E0027" "E0063" "python3 -c '$ADD_FIELD'" "python3 '$MUT/defeat_mech1.py'"
# (c) mechanism 2 defeated by the rest pattern change-model forbids in
#     assert_invariants, and E0027 must be GONE. Mechanism 1 must still fire.
variant c "E0063" "E0027" "python3 -c '$ADD_FIELD'" "python3 -c '$ADD_REST'"
echo "GATE-MECH2 OK: both mechanisms fire independently; neither alone is sufficient"
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
# OPENSPEC-UNTOUCHED — no code path writes inside openspec/. Diffed against the BASE SHA
# captured in task 1.1, never against the index: this project commits per task group, so
# `git diff --exit-code` between working tree and index passes over the very change it
# exists to catch.
#
# `git diff` alone is NOT enough: it lists tracked paths only, and a file the plugin
# WRITES at runtime is untracked, so the one violation this check exists to catch would
# be invisible to it. The `ls-files --others` sweep is the half that catches it.
#
# `:(top)` anchors both pathspecs to the repository root and `git -C "$ROOT"` anchors the
# reported paths there too, so the check gives the same answer from any working directory.
#
# Exactly TWO exclusions, both hand-edited planning documents rather than code-path
# writes: this change's own artifact directory, and the roadmap row group 12 corrects.
# They are excluded BY NAME, never by a broad prefix, so a stray file anywhere else under
# openspec/ still fails.
[ -n "$BASE" ] || { echo "FAIL: BASE sha not set" >&2; exit 1; }
ROOT=$(git rev-parse --show-toplevel) || { echo "FAIL: not a git repo" >&2; exit 1; }
git -C "$ROOT" rev-parse --verify "$BASE^{commit}" >/dev/null \
  || { echo "FAIL: bad BASE" >&2; exit 1; }
stray=$( { git -C "$ROOT" diff --name-only "$BASE" -- ':(top)openspec/'
           git -C "$ROOT" ls-files --others --exclude-standard -- ':(top)openspec/'; } \
         | grep -v '^openspec/changes/changes-from-cli/' \
         | grep -vx 'openspec/IMPLEMENTATION-ORDER.md' | sort -u || true)
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
#
# The SCOPE parameter is load-bearing and was added after review: without it the helper
# summed every test binary, so the whole-suite gate (316 today: 300 lib + 11 ci_workflow
# + 5 binary-integration) passed against a lib baseline of 300 with zero new lib tests,
# and stayed green even if the lib count FELL.
#   usage: testcount <scope> <filter> <minimum>      scope: --lib | --all-targets
testcount() {
  scope=$1; filter=$2; min=$3
  case "$scope" in --lib) sel="--lib";; --all-targets) sel="";; *)
    echo "TESTCOUNT FAIL: scope must be --lib or --all-targets, got '$scope'" >&2; return 1;; esac
  out=$(cargo test --all-features $sel "$filter" 2>&1) \
    || { printf '%s\n' "$out" >&2; return 1; }
  n=$(printf '%s\n' "$out" | sed -n 's/^test result: ok\. \([0-9][0-9]*\) passed.*/\1/p' \
      | awk '{t+=$1} END{print t+0}')
  [ "$n" -ge "$min" ] || {
    echo "TESTCOUNT FAIL: $scope filter '$filter' ran $n tests, expected >= $min" >&2
    return 1; }
  echo "TESTCOUNT OK: $scope filter '$filter' ran $n tests (>= $min)"
}
```

```sh
# DEPS — the argued dependency set, the one binary target, the resolved graph, the MSRV
# floor, and the genuinely-needed experiment. Reads `cargo metadata` JSON through
# python3 rather than adding a crate to do it, as subprocess-seam's DEPS does.
#   env: WORK  scratch directory (required, guarded); leg 5 copies the crate into it,
#              because `cargo build` REWRITES Cargo.lock during resolution before it
#              reaches the compile error the leg waits for, so an edit-and-restore in
#              place would silently discard the resolution this change verified.
#        DEPS_SKIP_LEG5=1 skips the three full builds.
set -u
: "${WORK:?DEPS FAIL: WORK (a scratch directory) must be set}"
[ -d "$WORK" ] || { echo "DEPS FAIL: WORK=$WORK is not a directory" >&2; exit 1; }
[ -f Cargo.toml ] && [ -d src ] || { echo "DEPS FAIL: run from the crate root" >&2; exit 1; }
EXPECTED_GRAPH='arraydeque foldhash hashbrown hashlink itoa memchr serde_core serde_json serde_spanned toml toml_datetime toml_parser toml_writer winnow yaml-rust2 zmij'
TRIPLES='aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu'

# --- leg 1: exactly one bin target, and build.sh actually produces it ---------
cargo metadata --no-deps --format-version 1 | python3 -c '
import json,sys
m=json.load(sys.stdin); p=m["packages"][0]
bins=[t for t in p["targets"] if "bin" in t["kind"]]
assert p["name"]=="herdr-openspec", p["name"]
assert len(bins)==1 and bins[0]["name"]=="herdr-openspec", bins
assert p["edition"]=="2024", p["edition"]
print("DEPS OK (leg 1a): exactly one bin target, herdr-openspec, edition 2024")
' || { echo "DEPS FAIL: leg 1a" >&2; exit 1; }
# The deletion and the EXIT STATUS are both load-bearing: build.sh has a real failure
# path (`error: cargo not found`, exit 1) and a leftover binary from any earlier build
# satisfies an existence check regardless of what the script did.
rm -f target/release/herdr-openspec
/bin/sh scripts/build.sh >/dev/null 2>&1 \
  || { echo "DEPS FAIL: leg 1b scripts/build.sh exited non-zero" >&2; exit 1; }
[ -x target/release/herdr-openspec ] \
  || { echo "DEPS FAIL: leg 1b no executable at target/release/herdr-openspec" >&2; exit 1; }
echo "DEPS OK (leg 1b): scripts/build.sh exited 0 and produced an executable binary"

# --- leg 2: the declared set, read from RESOLVED metadata, not manifest text --
cargo metadata --no-deps --format-version 1 | python3 -c '
import json,sys
want={"serde_json":["std"],
      "toml":["display","parse","serde","std"],
      "yaml-rust2":[]}
deps=[d for d in json.load(sys.stdin)["packages"][0]["dependencies"] if d["kind"] is None]
got={d["name"]:d for d in deps}
assert sorted(got)==sorted(want), f"normal deps are {sorted(got)}, expected {sorted(want)}"
for n,f in want.items():
    assert got[n]["uses_default_features"] is False, f"{n} uses default features"
    assert sorted(got[n]["features"])==sorted(f), "%s features %s != %s" % (n, got[n]["features"], f)
print("DEPS OK (leg 2a): exactly", len(deps), "normal deps, defaults off, features exact")
' || { echo "DEPS FAIL: leg 2a" >&2; exit 1; }
# `cargo metadata` reports [] for both `features = []` and an omitted key and cannot
# tell them apart; the requirement is about what the manifest says on its face, so
# this clause is read from the TEXT.
grep -q 'yaml-rust2 = .*features = \[\]' Cargo.toml \
  || { echo "DEPS FAIL: leg 2b yaml-rust2 does not spell out features = [] in Cargo.toml" >&2; exit 1; }
echo "DEPS OK (leg 2b): yaml-rust2's empty feature list is written out in Cargo.toml"
cargo build --locked >/dev/null 2>&1 \
  || { echo "DEPS FAIL: leg 2c cargo build --locked" >&2; exit 1; }
echo "DEPS OK (leg 2c): cargo build --locked"

# --- leg 3: the resolved normal graph, identical on all four supported triples -
prev=""
for t in $TRIPLES; do
  set=$(cargo tree -e normal --target "$t" 2>/dev/null \
        | grep -oE '[A-Za-z0-9_-]+ v[0-9]' | sed 's/ v[0-9]//' \
        | grep -v '^herdr-openspec$' | sort -u | tr '\n' ' ')
  [ -n "$set" ] || { echo "DEPS FAIL: leg 3 empty graph for $t" >&2; exit 1; }
  [ "$set" = "$(printf '%s ' $EXPECTED_GRAPH)" ] \
    || { echo "DEPS FAIL: leg 3 graph for $t is:" >&2; echo "  $set" >&2
         echo "  expected: $(printf '%s ' $EXPECTED_GRAPH)" >&2; exit 1; }
  [ -z "$prev" ] || [ "$set" = "$prev" ] \
    || { echo "DEPS FAIL: leg 3 triples disagree" >&2; exit 1; }
  prev="$set"
  for bad in syn quote proc-macro2 serde_derive encoding_rs ryu; do
    case " $set " in *" $bad "*)
      echo "DEPS FAIL: leg 3 $bad is in the normal graph for $t" >&2; exit 1;; esac
  done
done
echo "DEPS OK (leg 3): all four triples resolve the same 16 packages; no proc macro, no encoding_rs, no ryu"

# --- leg 4: MSRV, compared against Cargo.toml's OWN rust-version -------------
# A check carrying its own literal floor keeps enforcing the old value when the
# crate's rust-version moves, and the requirement is a claim about the relationship.
{ for t in $TRIPLES; do cargo tree -e normal --target "$t" 2>/dev/null; done; } \
  | grep -oE '[A-Za-z0-9_-]+ v[0-9][^ ]*' | sed 's/ v/\t/' | sort -u > "$WORK/graph-pairs.tsv"
cargo metadata --format-version 1 | WORK="$WORK" python3 -c '
import json,sys,re,os
pairs=set()
for line in open(os.environ["WORK"]+"/graph-pairs.tsv"):
    n,v=line.rstrip("\n").split("\t"); pairs.add((n,v.split()[0]))
m=json.load(sys.stdin)
floor=None
for p in m["packages"]:
    if p["name"]=="herdr-openspec": floor=p["rust_version"]
assert floor, "the crate declares no rust-version"
def key(v):
    # Zero-pad to three components: "1.85" and "1.85.0" are the same floor, and a
    # bare tuple comparison would rank (1,85,0) above (1,85) and report a false
    # violation for every package declaring the patch digit.
    n=[int(x) for x in re.findall(r"\d+", v)[:3]]
    return tuple(n+[0]*(3-len(n)))
over=[]; at=[]
for p in m["packages"]:
    if (p["name"], p["version"]) not in pairs: continue
    rv=p.get("rust_version")
    if not rv: continue
    if key(rv)>key(floor): over.append((p["name"],p["version"],rv))
    elif key(rv)==key(floor): at.append(p["name"])
assert not over, f"packages above the {floor} floor: {over}"
assert at, "no package sits at the floor - the intersection matched nothing"
print(f"DEPS OK (leg 4): floor {floor} from Cargo.toml; at the floor: {sorted(set(at))}")
' || { echo "DEPS FAIL: leg 4" >&2; exit 1; }

# --- leg 5: each dependency is genuinely needed, tested in a COPY ------------
[ "${DEPS_SKIP_LEG5:-0}" = "1" ] && { echo "DEPS SKIPPED (leg 5) by DEPS_SKIP_LEG5=1"; exit 0; }
before=$(git status --porcelain -- Cargo.toml Cargo.lock)
needed() { # $1 crate name, $2 the module whose failure is expected
  d="$WORK/deps-$1"; rm -rf "$d"; mkdir -p "$d"
  cp -R Cargo.toml Cargo.lock src tests rustfmt.toml "$d/"
  # The guard: removing a crate the manifest does not carry must be a FAILURE OF THE
  # CHECK, never counted as a pass — otherwise the leg silently succeeds against a
  # manifest that never declared it.
  grep -q "^$1 = " "$d/Cargo.toml" \
    || { echo "DEPS FAIL: leg 5 '$1' is not declared in Cargo.toml - nothing to remove" >&2; exit 1; }
  grep -v "^$1 = " "$d/Cargo.toml" > "$d/Cargo.toml.new" && mv "$d/Cargo.toml.new" "$d/Cargo.toml"
  if ( cd "$d" && cargo build >/dev/null 2>&1 ); then
    echo "DEPS FAIL: leg 5 the crate still builds without $1 - it is not genuinely needed" >&2
    exit 1
  fi
  echo "DEPS OK (leg 5/$1): removing it breaks the build ($2 depends on it)"
}
needed toml "config and state"
needed yaml-rust2 "schema"
needed serde_json "changes::from_cli"
# The guard itself, exercised: a crate the manifest does not carry must be reported as
# a failure of the check. Run in a subshell so its `exit 1` does not end this script.
if ( needed notacrate "nothing" ) >/dev/null 2>&1; then
  echo "DEPS FAIL: leg 5's not-declared guard did not fire" >&2; exit 1
fi
echo "DEPS OK (leg 5 guard): removing an undeclared crate is reported as a failure"
after=$(git status --porcelain -- Cargo.toml Cargo.lock)
[ "$before" = "$after" ] \
  || { echo "DEPS FAIL: leg 5 changed Cargo.toml or Cargo.lock in the working tree" >&2; exit 1; }
echo "DEPS OK: the working tree is unchanged, Cargo.lock included"
```

---

## 1. Foundation: baseline, the dependency, and the extracted checks
<!-- kind: operational -->

- [x] 1.1 CHECK: Capture the baseline that every later check measures against, and write
      each value into this file in place of the placeholder beside it.
      - `git rev-parse HEAD` → record as `BASE`. This is the commit the planning
        artifacts land on. Every `OPENSPEC-UNTOUCHED` run uses this value.
      - `cargo test --all-features --lib 2>&1 | sed -n 's/^test result: ok\. \([0-9]*\)
        passed.*/\1/p'` → record as the **lib** baseline. On `main` at planning time:
        **300** (the whole suite is 316: 300 lib + 11 `ci_workflow` + 5 binary-integration,
        which is why the final gate is scoped `--lib`).
      - `export CHECKS=<scratchpad>/changes-from-cli-checks` and extract every fenced block
        above to `$CHECKS/<LABEL>.sh` (and `GATE-MECH1.py`, `defeat_mech1.py`),
        byte-identically. Run the extracted files from here on, never a retyped copy.
      - `export WORK=<scratchpad>/changes-from-cli-work` and `mkdir -p "$WORK"`.
        `GATE-MECH2` and `DEPS` both refuse to run with `WORK` unset — an unset `WORK`
        would make their `rm -rf "$WORK/..."` catastrophic.
      **Red when:** `BASE` is empty or not a commit (`OPENSPEC-UNTOUCHED` aborts on both);
      the lib baseline is recorded higher than reality, which makes `TESTCOUNT`
      unsatisfiable rather than falsely green.

- [x] 1.2 CHECK: Run `NOSPAWN-GREP` and `GATE-MECH1` against the tree as it stands, before
      any edit. Both must **pass**, establishing that this change starts from a clean gate
      rather than inheriting a broken one. Expected, and observed at planning time:
      `NOSPAWN OK: 8 files checked under src, only src/cli.rs may spawn`;
      `GATE-MECH1 OK (half A): no Default for Change, ChangeSet, ArtifactRef, Origin in 9 files under src`;
      `GATE-MECH1 OK (half B): 34 constructions in src/changes.rs, no rest pattern and no functional update`.
      **Red when:** the tree already carries a `Default`, a rest pattern, or a spawn API
      outside `src/cli.rs` — in which case this change is not the place to fix it and the
      finding is reported before any code is written.

- [x] 1.3 CHECK: Run the three checks that must **fail** today, and record each message.
      They are the proof that the corresponding green runs later in the change are earned
      rather than structural.
      - `NOJSON-SEAM` → `src/changes.rs does not name serde_json - the check would pass
        vacuously` (the positive control is load-bearing, not decorative).
      - `DEPS` leg 2a → `normal deps are ['toml', 'yaml-rust2'], expected ['serde_json',
        'toml', 'yaml-rust2']`.
      - `DEPS` leg 5 for `serde_json` → `the crate still builds without serde_json - it is
        not genuinely needed` (true today, because nothing parses JSON yet).
      All three were demonstrated red at planning time.
      **Red when:** any of the three passes today, which would mean it cannot discriminate.

- [x] 1.4 CHANGE: Add to `Cargo.toml`:
      `serde_json = { version = "1.0.151", default-features = false, features = ["std"] }`
      — an explicit `features` list, `default-features = false`, and the version checked
      against the registry rather than remembered (`cargo info serde_json` reported
      `1.0.151` at planning time). Commit `Cargo.lock`.

- [x] 1.5 VERIFY: Run `DEPS` with `DEPS_SKIP_LEG5=1` — legs 1 through 4 must all pass, and
      the output recorded verbatim. Verified in a scratch copy at planning time:
      `leg 1a` one bin target, edition 2024; `leg 1b` `scripts/build.sh` exits 0 after the
      binary is deleted; `leg 2a` exactly three normal deps with defaults off and exact
      features; `leg 2b` the `features = []` spelling read from the manifest text;
      `leg 2c` `cargo build --locked`; `leg 3` all four triples resolve the same sixteen
      packages with no `syn`/`quote`/`proc-macro2`/`serde_derive`/`encoding_rs`/`ryu`;
      `leg 4` floor `1.85` read from `Cargo.toml`, with nine packages at it.
      Leg 5 is deferred to task 10.8, because `serde_json` is not used yet and it would
      correctly fail here.

- [x] 1.6 VERIFY: `make check` — must still be green with the dependency added and no code
      using it yet. Commit.

---

## 2. `parse_list` — the list envelope (`mod list_json`)
<!-- kind: behavior -->

- [x] 2.1 RED: Write failing unit tests in a new `mod list_json` inside `src/changes.rs`'s
      `mod tests`:
      `a_bare_array_is_rejected_rather_than_parsed`;
      `an_empty_change_list_is_a_supported_empty_state`;
      `a_change_entry_missing_a_required_field_is_skipped_not_fatal` (three entries; the
      middle has no `totalTasks`, another has a numeric `name`; one entry survives and two
      problems name positions `1` and `2`);
      `an_entry_whose_name_is_the_empty_string_is_skipped`;
      `a_repeated_name_keeps_the_first_entry_and_names_the_duplicate` (`alpha` `1/2`,
      `mike`, `alpha` `9/9` → one `alpha` carrying `1/2`, one problem);
      `empty_stdout_is_a_parse_failure`;
      `a_list_envelope_carries_the_root_path`;
      `an_envelope_with_no_root_yields_no_root`;
      `a_root_whose_path_is_not_a_string_yields_no_root`;
      `list_entries_keep_the_payload_order` (the sort is not this function's job).
      Confirm each fails because `parse_list` does not exist, not because of a typo:
      `cargo test --all-features --lib list_json::` must report a **compile** error naming
      `parse_list`, and the number of failing tests must equal the number written.

- [x] 2.2 GREEN: Implement `pub(crate) struct ListEntry { name: String, progress:
      tasks::Progress }`, `pub(crate) struct ListPayload { root: Option<PathBuf>, changes:
      Vec<ListEntry>, problems: Vec<String> }`, and
      `pub(crate) fn parse_list(text: &str) -> Result<ListPayload, String>`.
      `Err` for: not JSON, not an object, no `changes` key, `changes` not an array.
      Per entry: `name` a **non-empty** string, `completedTasks` and `totalTasks`
      non-negative integers representable as `usize`, and the name not already seen;
      anything else records one problem naming the entry's zero-based position and skips
      it. `lastModified` and `status` are read and discarded — `change-model` forbids
      storing either.

- [x] 2.3 REFACTOR: Extract the "non-negative integer from a `Value`" helper if it is used
      more than twice; otherwise state explicitly that no refactor was needed.

- [x] 2.4 VERIFY: `testcount --lib 'list_json::' <count written in 2.1>` — a counted
      minimum, not a bare filtered run, because a filter matching nothing exits 0. Commit.

---

## 3. `parse_apply` — the apply payload (`mod apply_json`)
<!-- kind: behavior -->

- [x] 3.1 RED: Write failing unit tests in `mod apply_json` for:
      `an_apply_payload_yields_schema_name_change_dir_and_context_files`;
      `an_apply_payload_missing_context_files_is_an_error`;
      `an_apply_payload_missing_schema_name_is_an_error`;
      `an_apply_payload_missing_change_dir_is_an_error`;
      `an_error_envelope_is_an_error` (the CLI's `{"status":[{severity,code,message}]}`
      shape, which arrives on stdout with exit 1 — this proves the parser rejects it rather
      than silently producing an empty payload should a future release exit 0);
      `context_files_values_that_are_not_string_arrays_are_rejected`;
      `an_empty_context_files_object_is_valid_and_yields_no_paths`;
      `malformed_apply_json_is_an_error`.
      Confirm the failures are missing-function failures.

- [x] 3.2 GREEN: Implement `pub(crate) struct ApplyPayload { schema_name: String,
      change_dir: PathBuf, context_files: BTreeMap<String, Vec<PathBuf>> }` and
      `pub(crate) fn parse_apply(text: &str) -> Result<ApplyPayload, String>`.
      `schemaName` a non-empty string, `changeDir` a non-empty string, `contextFiles` an
      object whose every value is an array of strings. A `BTreeMap` rather than a
      `HashMap`, so a rendered problem naming several keys is deterministic across runs.

- [x] 3.3 REFACTOR: Share the "required non-empty string field" helper with `parse_list` if
      it fits; otherwise state explicitly that none was needed.

- [x] 3.4 VERIFY: `testcount --lib 'apply_json::' <count>`. Commit.

---

## 4. `parse_schema_which` — the schema-directory payload (`mod which_json`)
<!-- kind: behavior -->

- [ ] 4.1 RED: Write failing unit tests in `mod which_json` for:
      `a_schema_which_payload_yields_the_directory_path`;
      `a_leading_non_json_line_is_not_tolerated` (the exact string
      `"Note: Schema commands are experimental and may change.\n{\"path\":\"/x\"}"` → `Err`);
      `a_schema_which_error_body_is_an_error` (`{"error": "...", "available": [...]}`,
      what the CLI writes to **stdout** on exit 1);
      `an_empty_payload_is_an_error`, `a_null_payload_is_an_error`,
      `a_non_string_path_is_an_error`, `a_payload_with_no_path_is_an_error` — the four
      malformed cases asserted individually so a failure names which one;
      `an_empty_path_is_an_error`;
      `source_and_shadows_are_ignored` (a payload carrying `source: "package"` and a
      non-empty `shadows` array still yields just the path).

- [ ] 4.2 GREEN: Implement `pub(crate) fn parse_schema_which(text: &str) ->
      Result<PathBuf, String>`. Parse the **whole** of `text` as one JSON document with
      `serde_json::from_str` — no line skipping, no prefix trimming. Require an object with
      a non-empty string `path`. Ignore every other key.

- [ ] 4.3 REFACTOR: None expected; state so explicitly if none was made.

- [ ] 4.4 VERIFY: `testcount --lib 'which_json::' <count>`. Commit.

---

## 5. `cli_artifacts` — placing paths at schema positions (`mod cli_artifacts`)
<!-- kind: behavior -->

- [ ] 5.1 RED: Write failing unit tests in `mod cli_artifacts` for:
      `an_omitted_context_files_key_becomes_an_empty_path_list_at_its_position` (a
      five-artifact `tdd`-shaped schema with keys for three);
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

- [ ] 5.3 REFACTOR: State explicitly whether any was needed.

- [ ] 5.4 VERIFY: `testcount --lib 'cli_artifacts::' <count>`. Commit.

---

## 6. `join_artifacts` — the positional cross-producer join (`mod join_artifacts`)
<!-- kind: behavior -->

- [ ] 6.1 RED: Write failing unit tests in `mod join_artifacts`, one per rule, all against
      hand-built vectors so no CLI and no filesystem is involved:
      `equal_length_lists_with_equal_ids_take_the_cli_paths_positionally`;
      `a_duplicate_id_is_joined_by_index_rather_than_collapsed` — ids `zeta`, `alpha`,
      `zeta` in both lists, with the CLI's index 2 carrying a path index 0 does not, and
      the assertion is that index 2 keeps **its own** path. An id-keyed implementation
      gives both positions index 0's paths and fails here; this is the test that
      discriminates the required join from the forbidden one;
      `an_empty_file_list_takes_the_cli_list`;
      `an_empty_cli_list_keeps_the_file_list`;
      `two_empty_lists_join_to_an_empty_list`;
      `differing_lengths_keep_the_file_list_and_name_both_counts` (the problem contains `5`
      and `3`);
      `a_differing_id_at_one_index_keeps_the_file_list_and_names_the_index` — four entries
      differing at indices 2 **and** 3, asserting the problem names index `2`, `design`,
      and `plan`, and does **not** name index `3`. Without the second difference the "first
      differing index" clause is untested;
      `the_join_never_reads_a_path_as_a_key` — file and CLI lists whose ids agree at every
      position but whose paths share no string at all; the result is the CLI's, proving no
      path comparison gates the join.

- [ ] 6.2 GREEN: Implement `pub(crate) fn join_artifacts(file: &[ArtifactRef], cli:
      &[ArtifactRef]) -> (Vec<ArtifactRef>, Option<String>)` applying `change-merge`'s six
      rules, in order.

- [ ] 6.3 REFACTOR: State explicitly whether any was needed.

- [ ] 6.4 VERIFY: `testcount --lib 'join_artifacts::' <count>`. Commit.

---

## 7. Schema resolution through the CLI fallback tier (`mod schema_fallback`)
<!-- kind: behavior -->

- [ ] 7.1 RED: Write failing unit tests in `mod schema_fallback`. Every one uses
      `cli::FakeCli` plus a real `testutil::ScratchDir` holding real `schema.yaml` files —
      **no process is spawned**, because the fake answers `["schema","which",…]` with a
      payload whose `path` names a scratch directory that genuinely holds the file
      `schema::load_dir` then reads:
      `a_schema_absent_from_the_repository_is_loaded_from_the_directory_the_cli_names`;
      `a_vendored_schema_never_reaches_the_cli` — register **no** `schema which` response;
      the fake panics on an unregistered pair, so a passing test proves the call was never
      made;
      `an_unreadable_vendored_schema_is_not_repaired_by_the_cli` — `schema.yaml` created as
      a **directory**, no `schema which` registration;
      `an_invalid_vendored_schema_is_not_repaired_by_the_cli`;
      `an_unknown_schema_name_degrades_that_change_alone` (three changes; the middle one's
      `schema which` answers `Err(CliError::Failed { code: Some(1), stderr: "" })`);
      `a_which_path_naming_a_directory_with_no_schema_yaml_stops_the_tier` — assert exactly
      one recorded `schema which` call;
      `an_unstartable_openspec_during_the_fallback_degrades_that_change`;
      `an_unusable_schema_yaml_at_the_cli_named_path_degrades_that_change` — both the
      invalid-bytes and the `schema.yaml`-is-a-directory cases;
      `a_malformed_which_payload_is_a_problem_not_a_panic` — the four payloads asserted
      individually;
      `a_parser_problem_from_the_cli_named_schema_reaches_every_change_using_it` — two
      changes sharing one cached, CLI-named schema whose `schema.yaml` has one unusable
      artifact entry; **both** changes must carry the parser's problem, which an
      implementation attaching it only on the cache miss would fail;
      `three_changes_sharing_one_unvendored_schema_ask_the_cli_once` — assert five
      recorded calls in total;
      `a_failed_lookup_is_cached_rather_than_retried_per_change`;
      `two_different_schema_names_are_asked_for_separately`.

- [ ] 7.2 GREEN: Implement the per-call schema resolver: a `HashMap<String,
      CachedCliSchema>` owned by the `from_cli` call (never a `static` —
      `resolve::BinCache`'s reason: the suite runs this crate's tests in parallel threads
      of one process). On a miss, call `schema::load(repo, name)`; on `Ok` cache the schema
      **and its `ParsedSchema::problems`**; on `Err(NotVendored)` run
      `["schema", "which", name, "--json"]`, `parse_schema_which`, then
      `schema::load_dir(dir, name)`, caching the same pair; on `Err(Unreadable)` or
      `Err(Invalid)` cache the failure without calling the CLI. Every change using a cached
      entry receives a clone of its problems, not only the change that populated it.

- [ ] 7.3 REFACTOR: Fold the fallback's five failure paths into one problem-rendering
      helper if that removes duplication; otherwise state explicitly that no refactor was
      needed.

- [ ] 7.4 CHECK: Run `NOSPAWN-GREP` — confirm no process API name entered `src/changes.rs`
      while wiring the fallback. **Red when:** the tier was implemented by reaching for
      `Command` rather than the trait object.

- [ ] 7.5 VERIFY: `testcount --lib 'schema_fallback::' <count>`. Commit.

---

## 8. `from_cli` — the composition (`mod from_cli`)
<!-- kind: behavior -->

- [ ] 8.1 RED: Write failing unit tests in `mod from_cli` for:
      `a_two_change_repository_drives_exactly_three_invocations` — assert
      `FakeCli::calls()` **equals** the exact three-element vector, not merely contains
      them, so an extra `status` call fails;
      `no_status_invocation_is_made_even_when_an_apply_call_fails` — no `status`
      registration at all;
      `the_argument_vector_carries_no_sort_flag` — the recorded list vector equals
      `["list", "--json"]` exactly;
      `progress_is_read_from_the_list_payload_pair` — the apply payload deliberately
      carries a conflicting `progress`, and `0/0` must appear nowhere in the result;
      `the_most_recently_modified_default_order_is_replaced_by_byte_order`;
      `case_and_digits_order_by_byte_not_by_locale` — `Beta` before `alpha`, which a
      `localeCompare` sort reverses;
      `a_change_dir_whose_final_component_is_not_the_change_name_is_rejected`;
      `every_produced_change_is_active_and_satisfies_the_shared_invariants` — every value
      through `conformance::assert_invariants`;
      `an_absent_openspec_binary_yields_an_empty_result_and_one_problem`;
      `a_non_zero_exit_from_list_yields_an_empty_result_and_one_problem`;
      `a_schema_the_cli_rejects_removes_one_change_and_keeps_the_others` — the problem
      names the change, the vector, and exit code `1`, and the test asserts the problem
      does **not** contain any CLI message text, because stderr was empty;
      `malformed_json_from_a_single_apply_call_is_contained_to_that_change`;
      `an_apply_payload_missing_context_files_is_a_per_change_failure`;
      `empty_stdout_from_list_is_a_parse_failure_not_an_empty_repository` — asserts one
      problem, and asserts the empty-list case records none, so the two are distinguished;
      `a_bare_array_is_rejected_at_the_composition`;
      `a_mismatched_root_discards_the_whole_cli_result` — exactly one recorded call, so the
      guard runs before the per-change calls;
      `a_symlinked_repository_root_is_not_a_disagreement` — a real scratch symlink;
      `an_envelope_with_no_root_is_treated_as_a_disagreement`;
      `an_omitted_context_files_key_is_an_empty_path_list_end_to_end`;
      `a_multi_file_artifact_survives_the_composition`;
      `a_context_files_key_naming_no_schema_artifact_is_ignored_end_to_end`.

- [ ] 8.2 GREEN: Implement `pub struct CliChanges` and
      `pub fn from_cli(cli: &dyn cli::OpenspecCli, repo: &Path) -> CliChanges`, plus
      `pub(crate) fn same_directory(a: &Path, b: &Path) -> bool` (canonicalize both;
      compare the canonical forms when both succeed, otherwise compare as given).
      Order of operations: `list --json` → parse → root guard → per-change apply → schema
      resolution → `cli_artifacts` → `Change` → sort by name in byte order.
      Build each `Change` naming all seven fields; no `Default`, no `..`.

- [ ] 8.3 REFACTOR: Clean up while green; state explicitly if none was needed.

- [ ] 8.4 RED then GREEN: `a_full_from_cli_run_leaves_the_tree_byte_identical` —
      `testutil::snapshot` over a scratch tree and `testutil::shallow_snapshot` over the
      process's own working directory, taken around one call covering a successful change,
      a change whose apply call failed, a schema resolved through the CLI fallback tier,
      and a malformed payload. The cwd snapshot is shallow for the reason `cli.rs`'s
      equivalent test records: under `cargo test` the real cwd is this repository, whose
      `target/` holds tens of thousands of files.

- [ ] 8.5 VERIFY: `testcount --lib 'from_cli::' <count>`, then run `NOSPAWN-GREP` and
      `GATE-MECH1` — the latter must now report **more** than 34 constructions, since
      `from_cli` builds the type. Commit.

---

## 9. `merge` — layering CLI over files (`mod merge`)
<!-- kind: behavior -->

- [ ] 9.1 RED: Write failing unit tests in `mod merge`, all pure (hand-built `ChangeSet`
      and `CliChanges`, no CLI, no filesystem):
      `the_cli_schema_progress_and_artifacts_replace_the_files` — the two producers are
      given **different** schema names (`stale-name` and `tdd`) and the test asserts the
      merged schema is `tdd` and that `stale-name` appears nowhere; equal names would leave
      the change's headline claim unverified;
      `the_merged_dir_comes_from_the_file_change`;
      `a_change_only_the_cli_reported_is_inserted_in_name_order`;
      `a_change_only_the_file_producer_saw_survives_the_merge` (and records no problem);
      `an_empty_cli_result_leaves_the_file_result_intact` — assert the merged set `==` the
      input set except for the appended problems, which a weaker per-field assertion would
      not catch;
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
      concatenated file-first then CLI then join with exactly-equal strings collapsed to
      the first occurrence. Union the two name sets, re-sort by name in byte order, pass
      `archived` through unchanged, and concatenate the two problem lists onto
      `ChangeSet::problems`.

- [ ] 9.3 REFACTOR: Share the byte-order sort with `from_cli`'s if both ended up with a
      copy; otherwise state explicitly that no refactor was needed.

- [ ] 9.4 CHECK: Contract gate — re-read `openspec/specs/change-model/spec.md` and this
      change's delta on it, and confirm the merged `Change` still carries exactly seven
      fields, no `status`, no ratio, no formatted string, no `lastModified`, and no
      producer discriminant, and that `archived` is still a separate vector rather than a
      filter over one list.

- [ ] 9.5 VERIFY: `testcount --lib 'merge::' <count>`. Commit.

---

## 10. Architectural checks and their negative controls
<!-- kind: operational -->

- [ ] 10.1 CHECK: Before running the controls, confirm each check is green on the tree as
      it now stands, so a red control below is attributable to the plant rather than to the
      tree: `NOSPAWN-GREP`, `GATE-MECH1`, `NOJSON-SEAM`, `OPENSPEC-UNTOUCHED`.

- [ ] 10.2 VERIFY: `NOSPAWN-GREP` negative controls — four scratch copies of `src/`, each
      message recorded:
      (a) `Command::new("openspec")` planted in `src/changes.rs` → `NOSPAWN FAIL: spawn API
      outside .../cli.rs:` naming that line;
      (b) `cli.rs` deleted → `NOSPAWN FAIL: .../cli.rs missing - the exclusion has nothing
      to exclude`;
      (c) `cli.rs` emptied of its spawn → `NOSPAWN FAIL: .../cli.rs names no spawn API -
      exclusion is vacuous`;
      (d) the spawn planted at `ui/cli.rs` rather than the top level → still fails, proving
      the exclusion is by path and not by base name.
      **Red when:** any of (a)–(d) passes.

- [ ] 10.3 VERIFY: `GATE-MECH1` negative controls — ten scratch copies of `src/`, each
      message recorded. All ten were demonstrated red at planning time; the last two of the
      first five are the forms that defeated an earlier shell version of this check and are
      the reason it is written in Python:
      (a) `Change { name: …, ..other }` on one line;
      (b) the same functional update spread across lines;
      (c) `let Change { name, .. }`;
      (d) a functional update with a `// line comment` between the comma and the `..`;
      (e) the same with a `/* block comment */` between them;
      (f) `impl std::default::Default for Change` — the path-qualified form;
      (g) `impl Default for crate::changes::Origin` planted in `src/state.rs`, proving the
      search covers every file rather than only `changes.rs`;
      (h) a multi-line `#[derive(\n …, Default,\n)]` on `Change`;
      (i) `src/changes.rs` deleted;
      (j) a copy holding fewer than eight `.rs` files.
      **Red when:** any of (a)–(j) passes. The green run on the real tree is itself
      discriminating: `src/changes.rs` contains a `segment[..star]` slice index the check
      must not fire on.

- [ ] 10.4 VERIFY: `GATE-MECH2` — the green control first (an unmutated copy builds), then
      the three variants, each asserting the error codes that must be **present** and the
      one that must be **absent**:
      (a) a field added to `Change` and nothing else → `E0027` **and** `E0063`;
      (b) mechanism 1 fully defeated by `defeat_mech1.py` → `E0027` present, `E0063`
      **absent**, proving mechanism 2 catches what mechanism 1 misses;
      (c) mechanism 2 defeated by a `..` in `assert_invariants` → `E0063` present, `E0027`
      **absent**, proving mechanism 1 catches what mechanism 2 misses.
      Verified end to end against the pre-change tree at planning time; variant (a)
      reported five `E0063` initializer sites, which after this change must be more.
      **Red when:** any variant compiles, or produces the code it must not, or the
      unmutated control fails to build. Asserting on codes rather than on "it failed" is
      load-bearing: an earlier draft was defeated by an `E0277` from `tasks::Progress`
      lacking `Default`, which masked the `E0027` under test — which is also why
      `defeat_mech1.py` supplies that `Default` rather than relying on the accident.

- [ ] 10.5 VERIFY: `NOJSON-SEAM` — must now **pass** (`serde_json used in src/changes.rs,
      absent from src/cli.rs`), having failed in task 1.3 before the crate used it. Then
      one negative control: a scratch copy of `src/cli.rs` carrying `use serde_json::Value;`
      → the check exits 1.

- [ ] 10.6 VERIFY: `NOSPAWN-RUN` — the whole suite on a `PATH` from which every directory
      holding `npm`, `node`, or `openspec` has been removed, with the five preconditions
      passing first (all three unresolvable, `cargo` and `rustc` still resolvable). Every
      test must pass, including this change's own. **Red when:** any test in this change
      reached the real `openspec` binary.

- [ ] 10.7 VERIFY: `OPENSPEC-UNTOUCHED` with `BASE` from task 1.1 — must report
      `OPENSPEC-UNTOUCHED OK`. Then one negative control: create
      `openspec/specs/planted-probe.md`, re-run, confirm it **fails** naming that path,
      then delete it and confirm the check passes again. **Red when:** the untracked sweep
      is missing, which the planted file is exactly what detects. Note the check excludes
      exactly two paths by name — this change's artifact directory and
      `openspec/IMPLEMENTATION-ORDER.md`, both hand-edited planning documents rather than
      code-path writes — so it stays meaningful after group 12 and is re-run there.

- [ ] 10.8 VERIFY: `DEPS` in full, leg 5 included — the genuinely-needed experiment run
      three times in **copies**: remove `toml` → `cargo build` fails; remove `yaml-rust2` →
      fails; remove `serde_json` → fails, which is the leg that was correctly **red** in
      task 1.3 and must now be green. Then the guard itself: asking it to remove a crate
      the manifest does not carry must be reported as a failure of the check, not a pass.
      Confirm the working tree is byte-identical afterwards, `Cargo.lock` included.

---

## 11. Change Review
<!-- kind: operational -->

- [ ] 11.1 CHECK: Dispatch an independent reviewer — an agent that did **not** write the
      implementation and is **not** a fork of the implementing session — against
      `proposal.md`, all five spec files, `design.md`, `tasks.md`, and the diff. Give it
      the concentration points from `openspec/config.yaml` → `rules.tasks`, and these two
      first: (1) for every spec scenario, name the test that would go red if the behaviour
      were deleted; (2) both mechanisms of the two-producer gate, and whether any check
      would still pass if one mechanism were removed.

- [ ] 11.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
      one-line reason, note each SUGGESTION, and re-run the affected tests.

- [ ] 11.3 VERIFY: Confirm no blocking or unowned finding remains, and record the finding
      counts by severity in this file.

---

## 12. Documentation
<!-- kind: operational -->

- [ ] 12.1 CHECK: Re-read the sections named below before editing them, and confirm each
      still says what this change believes it says. **Red when:** a section was already
      corrected by another change and this group would reintroduce a stale claim.

- [ ] 12.2 Rewrite in `SPEC.md` → Data layer → Resolution chain → **Changes** (audience:
      every later change): replace the sentence describing `openspec list --json`'s output
      with the real envelope `{"changes": [...], "root": {"path", "source"}}`, and add that
      the CLI's own `--sort name` is `localeCompare` and therefore cannot be used to obtain
      the byte order both producers must share. Durable because two later changes read this
      paragraph as the contract and the current text would send them to a bare array.

- [ ] 12.3 Rewrite in `SPEC.md` → Data layer → Dual-source model (audience: every later
      change): state that the CLI-side task counts come from `list --json`'s
      `completedTasks`/`totalTasks` and **not** from `instructions apply --json`'s
      `progress`, which resolves `apply.tracks` without globbing and therefore disagrees
      whenever `tracks` is a glob. Durable because the whole model rests on the two
      producers reporting one number.

- [ ] 12.4 Add to `SPEC.md` → Data layer → Resolution chain → **Artifact files**
      (audience: `live-refresh` and any later CLI consumer): the plugin runs `instructions
      apply` and **not** `status --change`, and why — the same `resolveArtifactOutputs`, and
      `status`'s `artifacts` array is in topological build order rather than the schema's
      declared order, so it is not a source for a positional join.

- [ ] 12.5 Add three rows to `SPEC.md` → Degraded states (audience: `degraded-states`,
      which audits that table end to end): a schema declaring the same artifact id twice
      (the CLI rejects the schema outright, so the change is permanently file-mode); a CLI
      reporting a repository root other than the resolved one (the whole CLI result is
      discarded); and a CLI command failing (the reason is unavailable, because the CLI
      writes its diagnostic to stdout and the seam's `Failed` carries stderr only).

- [ ] 12.6 Rewrite in `openspec/IMPLEMENTATION-ORDER.md` → Phase 3 → the
      `changes-from-cli` row (audience: whoever reads the roadmap next): drop
      `openspec status --change <n> --json` from the list of parsed commands and record in
      one clause why. Rewrite rather than append — leaving the old list beside a correction
      is what makes a roadmap stop being read.

- [ ] 12.7 Rewrite in `AGENTS.md` → **Current repo state** (audience: every future
      session): fold `changes-from-cli` into the landed list and state in one clause what
      it added, replacing the "the CLI path is not built yet" implication rather than
      appending a paragraph beside it. Net addition must stay under ten lines.

- [ ] 12.8 Rewrite in `AGENTS.md` → **Architecture rules** (audience: every future
      session): the existing "Nothing spawns a process outside `cli`" bullet gains one
      clause naming that the seam also parses nothing — `serde_json` must not appear in
      `src/cli.rs` — and the "Checkbox counting follows the OpenSpec CLI's rule exactly"
      bullet gains the `list --json`-not-apply-`progress` clause. Edit both in place; add
      no new bullet.

- [ ] 12.9 VERIFY: Re-run `OPENSPEC-UNTOUCHED` after the roadmap edit — it must still
      report OK, because `openspec/IMPLEMENTATION-ORDER.md` is one of its two named
      exclusions. **Red when:** the documentation group touched any other path under
      `openspec/`.

---

## 13. Lint & Verify
<!-- kind: operational -->

- [ ] 13.1 CHECK: Inspect the intended verification commands and affected tiers. The gated
      code is all of `src/changes.rs`; the affected tier is the unit tier plus the
      command-level checks in group 10. Confirm `make check` is the composite and that each
      sub-command is also run individually below, so a failure names itself.

- [ ] 13.2 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.

- [ ] 13.3 VERIFY: `cargo fmt --all -- --check` — clean.

- [ ] 13.4 VERIFY: `cargo test --all-features` — green. (Rust's type checker runs as part
      of `cargo test` and `cargo clippy`; this repository has no separate type-check
      command.)

- [ ] 13.5 VERIFY: `cargo llvm-cov --fail-under-lines 80` — the floor is 80% and is never
      lowered, waived, or given an exclusion. `main` stands at 98.67% over 5499 lines;
      record the new figure. **Red when:** the new code is added without tests, which is
      what the floor exists to catch.

- [ ] 13.6 VERIFY: `testcount --lib '' <lib baseline from 1.1 + 1>` — the **lib** test
      count must be strictly greater than the 300 recorded in task 1.1. The `--lib` scope
      is load-bearing: an unscoped run sums 316 tests across three binaries and would pass
      with zero new lib tests, and would stay green even if the lib count fell.

- [ ] 13.7 VERIFY: `make check` — the single composite gate, exit 0. If it fails, name the
      failing sub-command rather than summarising.

- [ ] 13.8 VERIFY: `openspec validate changes-from-cli --strict` — valid. This is the one
      command in the change that runs the real `openspec` binary; design.md → Test
      Boundaries names it, and no test depends on it.
