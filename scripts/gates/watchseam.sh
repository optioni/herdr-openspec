# WATCHSEAM — NEW in live-refresh. The filesystem-watch crate is confined to ONE module, on
# exactly MDSEAM's and NOSPAWN-GREP's single-file terms: `notify` is replaceable by editing
# src/watch.rs, and no other file in the crate — tests/ included — may reach for it.
#
# The pattern is PATH-QUALIFIED and TOKEN-SPECIFIC, and that is not caution, it is a
# measured requirement. A bare `EventKind` alternative — the obvious one to reach for, since
# it is notify's own event type — matches `KeyEventKind` and `MouseEventKind` in
# src/ui/app.rs at TWELVE sites of correct, unmodified code, so the check would have been RED
# on `main` before this change wrote a line. Verified at planning time by running the bare
# pattern. NOTABSEAM shipped red on an unmodified tree in this repository once already; this
# is that lesson applied before the fact rather than after.
#
# `notify` as a bare word is not in the pattern either: it is an ordinary English verb and
# would match a doc comment saying "notify the caller".
#
# This check FAILS with "src/watch.rs missing" until the module exists, and with "names no
# notify API" until group 6 adds the dependency. Both are the guards doing their job, not
# defects: without them, a renamed or gutted module makes the check report a clean tree.
set -u
WATCH="${WATCH:-src/watch.rs}"
MIN="${MIN:-29}"
fail() { echo "WATCHSEAM FAIL: $1" >&2; exit 1; }

# G2 — leg 3 resists an import alias (design.md -> Decision 3). Two alternatives join the
# original process::Command|Command::new|Stdio: process::(Command|Child|Stdio|Output|
# ChildStd) (the spawn items of std::process, matched at their import site as well as at a
# fully-qualified call) and process::\{ (ANY brace-grouped import from std::process). Not
# a bare `std::process`: measured, src/lib.rs:33 calls std::process::id() and
# src/main.rs:1 reads `use std::process::exit;`, two legitimate non-spawning uses a bare
# module-path pattern would turn red.
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

[ -d src ] || fail "no src directory"
[ -f "$WATCH" ] || fail "$WATCH missing - the exclusion has nothing to exclude"

NOTIFY_RE='notify::|notify_types::|RecommendedWatcher|RecursiveMode|FsEventWatcher|INotifyWatcher|use notify\b|extern crate notify'

# Guard A — positive control, checked BEFORE the count: the allowed file must actually name
# the crate, or the exclusion protects nothing and a clean result means nothing.
grep -qE "$NOTIFY_RE" "$WATCH" || fail "$WATCH names no notify API - exclusion is vacuous"

# Guard B — the searched set is real. Excluded BY PATH, never by base name, so a future
# src/ui/watch.rs is still searched.
n=$(find src tests -name '*.rs' ! -path "$WATCH" | wc -l | tr -d ' ')
[ "$n" -ge "$MIN" ] || fail "searched only $n files (expected >= $MIN)"

hits=$(find src tests -name '*.rs' ! -path "$WATCH" -print0 \
       | xargs -0 -I{} grep -nE "$NOTIFY_RE" {} /dev/null 2>&1 || true)
[ -z "$hits" ] || { echo "WATCHSEAM FAIL: the watch crate is named outside $WATCH:" >&2
                    echo "$hits" >&2; exit 1; }

# Leg 2 — the confined module is PLAIN DATA: it names no ratatui type, so a watched path
# never arrives at the view already styled and every function in it is assertable without a
# frame. Same terms as MDSEAM's second leg and NOTABSEAM. Comment-inclusive on purpose: a
# doc comment naming a ratatui type is itself the coupling this forbids, so say "the view".
r=$(grep -nE 'ratatui|Modifier|Style|Span|Rect|Frame|Buffer' "$WATCH" || true)
[ -z "$r" ] || { echo "WATCHSEAM FAIL: $WATCH names a ratatui type:" >&2; echo "$r" >&2; exit 1; }

# Leg 3 — the module spawns no process. NOSPAWN-GREP already forbids this tree-wide; this is
# the file-scoped restatement, because src/watch.rs is the new file where reaching for an
# `fswatch` subprocess would be plausible.
p=$(grep -nE "$SPAWN_RE" "$WATCH" || true)
[ -z "$p" ] || { echo "WATCHSEAM FAIL: $WATCH spawns a process:" >&2; echo "$p" >&2; exit 1; }

# Positive control for the widened pattern — anchored on src/cli.rs:14
# (`use std::process::{Command, Stdio};`). This does NOT by itself prove either new
# alternative is load-bearing: the pre-existing pattern already matches this line via the
# bare `Stdio` alternative, so deleting both additions would leave this control green too
# (see the isolating plants recorded in tests/gate-controls.toml for that proof).
[ -f src/cli.rs ] || fail "src/cli.rs missing - the widened-pattern control has nothing to match"
grep -qF 'use std::process::{Command, Stdio};' src/cli.rs \
  || fail "positive control - src/cli.rs no longer names 'use std::process::{Command, Stdio};'"
echo 'use std::process::{Command, Stdio};' | grep -qE "$SPAWN_RE" \
  || fail "positive control - the widened pattern does not match src/cli.rs's own import line"

echo "WATCHSEAM OK: $n files searched (>= $MIN), notify only in $WATCH, no ratatui type there, no spawn"
