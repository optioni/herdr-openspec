# NOSPAWN-GREP — `cli` is the only module in the crate that spawns a process.
# Carried forward from markdown-viewer with its executable logic BYTE-IDENTICAL. Only its
# MIN argument moves, from 17 to 18, and MIN is a parameter precisely so that is a change to
# a task's invocation rather than to the check.
SRC="${SRC:-src}"
MIN="${MIN:-25}"
fail() { echo "NOSPAWN FAIL: $1" >&2; exit 1; }

# G2 — the seam grep resists an import alias (design.md -> Decision 3). Two alternatives
# join the original process::Command|Command::new|Stdio: process::(Command|Child|Stdio|
# Output|ChildStd) (the spawn items of std::process, matched at their import site as well
# as at a fully-qualified call) and process::\{ (ANY brace-grouped import from
# std::process). Not a bare `std::process`: measured, src/lib.rs:33 calls
# std::process::id() and src/main.rs:1 reads `use std::process::exit;`, two legitimate
# non-spawning uses a bare module-path pattern would turn red.
#
# COST ACCEPTED: `use std::process::{exit, id};` — a brace group of only safe items — is
# refused too, by process::\{ alone. The workaround is one `use` line per item.
#
# TWO KNOWN LIMITS, stated rather than engineered around:
#   (a) a crate-root alias is not matched: `use std as s; s::process::Command::new`.
#   (b) a re-export through the exempted seam is not matched either: `pub use
#       std::process::Command;` in src/cli.rs — the one file where that line is legal and
#       invisible — then `use crate::cli::Command as Proc;` anywhere else.
SPAWN_RE='process::Command|Command::new|Stdio|process::\{|process::(Command|Child|Stdio|Output|ChildStd)'

# Guard A — the tree and the one allowed spawner exist. A renamed, split, or moved seam
# (src/cli/mod.rs, say) must be a deliberate update to this block, never a silent stop.
[ -d "$SRC" ] || fail "no such directory: $SRC"
[ -f "$SRC/cli.rs" ] || fail "$SRC/cli.rs missing - the exclusion has nothing to exclude"

# Guard B — the excluded file actually spawns. Without this, a tree where the seam was
# gutted (or never written) passes, and the exclusion protects nothing.
grep -qE 'process::Command|Command::new' "$SRC/cli.rs" \
  || fail "$SRC/cli.rs names no spawn API - exclusion is vacuous"

# Positive control for the widened pattern — anchored on src/cli.rs:14
# (`use std::process::{Command, Stdio};`). This does NOT by itself prove either new
# alternative is load-bearing: the pre-existing pattern already matches this line via the
# bare `Stdio` alternative, so deleting both additions would leave this control green too
# (see the isolating plants recorded in tests/gate-controls.toml for that proof).
grep -qF 'use std::process::{Command, Stdio};' "$SRC/cli.rs" \
  || fail "positive control - $SRC/cli.rs no longer names 'use std::process::{Command, Stdio};'"
echo 'use std::process::{Command, Stdio};' | grep -qE "$SPAWN_RE" \
  || fail "positive control - the widened pattern does not match $SRC/cli.rs's own import line"

# Guard C — the searched set is the crate's real module set, not an empty list. grep exits
# 2 on a missing file and `!` would pass that; this is the test -f-shaped guard.
# The exclusion is BY PATH, not by base name: `! -name 'cli.rs'` would silently exempt a
# future src/ui/cli.rs.
n=$(find "$SRC" -name '*.rs' ! -path "$SRC/cli.rs" | wc -l | tr -d ' ')
[ "$n" -ge "$MIN" ] || fail "searched only $n files under $SRC (expected >= $MIN)"

# The check itself. Judged on OUTPUT EMPTINESS, never on a pipeline's exit status.
# The trailing /dev/null makes grep's argument list unconditionally non-empty, so the
# GNU-xargs empty-input case cannot make grep read stdin and hang.
hits=$(find "$SRC" -name '*.rs' ! -path "$SRC/cli.rs" -print0 \
       | xargs -0 -I{} grep -nE "$SPAWN_RE" {} /dev/null 2>&1 || true)
[ -z "$hits" ] || { echo "NOSPAWN FAIL: spawn API outside $SRC/cli.rs:" >&2
                    echo "$hits" >&2; exit 1; }
echo "NOSPAWN OK: $n files checked under $SRC (>= $MIN), only $SRC/cli.rs may spawn"

# MODULE-SCOPED, stricter, and unweakened: these five files name no process API at all,
# spawning or not. The [ -f ... ] guards are load-bearing for the same exit-code-2 reason.
for f in src/resolve.rs src/config.rs src/state.rs src/schema.rs src/tasks.rs; do
  [ -f "$f" ] || { echo "MODULE-SCOPED FAIL: $f missing" >&2; exit 1; }
done
m=$(grep -nE 'std::process|Command|Stdio' src/resolve.rs src/config.rs src/state.rs \
    src/schema.rs src/tasks.rs || true)
[ -z "$m" ] || { echo "MODULE-SCOPED FAIL:" >&2; echo "$m" >&2; exit 1; }
