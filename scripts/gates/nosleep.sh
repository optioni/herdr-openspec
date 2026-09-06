# NOSLEEP — NEW in live-refresh. Hazard 1, mechanised: no test in this crate sleeps a fixed
# interval and then asserts that something has already happened. Every sleep in the tree must
# sit inside a DEADLINE-BOUNDED POLL, which is the only shape that stays green on a cold or
# loaded machine.
#
# This is deliberately NOT "no sleep anywhere". Measured at planning time, `main` already
# carries THREE sleeps — src/cli.rs's `a_program_that_reads_stdin_returns_rather_than_blocking`
# and tests/cli.rs's two `try_wait` polls — and all three are correct: each is the pause
# inside a `while Instant::now() < deadline` loop, and tests/cli.rs:141 even carries the
# comment recording the flake that produced the rule ("Poll to a deadline rather than
# sleeping a fixed interval"). A blanket prohibition would be RED on the unmodified tree,
# and the only ways out would be to delete three correct tests or to exempt two files —
# which is how a check rots into a rubber stamp.
#
# `yield_now` is deliberately NOT in the pattern: it has no duration, so a loop around it is
# still a condition poll and cannot make an assertion premature. Group 6's real-watcher test
# uses it.
#
# Two legs, and a self-contained negative control that runs on EVERY invocation rather than
# only at plant time.
#   env: MIN  minimum number of .rs files searched (guards a vacuous search)
#        SLEEP_MIN  minimum number of sleep sites the scan must FIND (guards a scan that
#                   matched nothing and reported a clean tree)
set -u
MIN="${MIN:-30}"
SLEEP_MIN="${SLEEP_MIN:-6}"
[ -d src ] && [ -d tests ] || { echo "NOSLEEP FAIL: run from the crate root" >&2; exit 1; }
n=$(find src tests -name '*.rs' | wc -l | tr -d ' ')
[ "$n" -ge "$MIN" ] || { echo "NOSLEEP FAIL: searched only $n files (expected >= $MIN)" >&2; exit 1; }

MIN="$MIN" SLEEP_MIN="$SLEEP_MIN" python3 - <<'PY'
import os, re, subprocess, sys

SLEEP = re.compile(r"thread::sleep|sleep_ms|park_timeout|std::thread::park\b")
# A span is "deadline-bounded" when it names a deadline AND loops. Both halves matter:
# `deadline` alone could be a variable nothing loops on, and a bare `while` could spin
# forever.
DEADLINE = re.compile(r"\bdeadline\b")
LOOP = re.compile(r"\b(while|loop)\b")

def spans(text):
    """Split a Rust file at line-anchored #[test] attributes, the same splitter WIDTHS,
    LISTWIDTHS, MDWIDTHS, DETAILWIDTHS and TASKWIDTHS use. Index 0 is the pre-test
    prelude (production code); every later part is one test function PLUS EVERYTHING
    DEFINED AFTER IT up to the next #[test]. That limit is stated rather than discovered:
    a sleeping helper written below a correct deadline-bounded test inherits that test's
    verdict. Leg 2's absolute ban over src/ui is what closes it where this change's own
    tests live; elsewhere the limit stands and a reviewer is the backstop."""
    return re.split(r"(?m)^[ \t]*#\[test\][ \t]*$", text)

def offenders(path, text):
    out = []
    for i, part in enumerate(spans(text)):
        if not SLEEP.search(part):
            continue
        name = re.search(r"fn\s+([a-z0-9_]+)", part)
        name = name.group(1) if name else ("<prelude>" if i == 0 else "<unnamed>")
        ok = bool(DEADLINE.search(part)) and bool(LOOP.search(part))
        out.append((path, name, ok))
    return out

# --- the self-contained negative control, run on EVERY invocation ------------------
control_bad = "#[test]\nfn c() { std::thread::sleep(d); assert!(happened); }\n"
control_good = ("#[test]\nfn c() { let deadline = Instant::now() + d;\n"
                "  while Instant::now() < deadline { if done() { break } "
                "std::thread::sleep(d); } assert!(happened); }\n")
bad = offenders("<control>", control_bad)
good = offenders("<control>", control_good)
if not (len(bad) == 1 and bad[0][2] is False):
    print("NOSLEEP FAIL: the negative control was not reported - the scan is broken",
          file=sys.stderr); sys.exit(1)
if not (len(good) == 1 and good[0][2] is True):
    print("NOSLEEP FAIL: the positive control was reported - the scan rejects a correct poll",
          file=sys.stderr); sys.exit(1)

files = subprocess.run(["find", "src", "tests", "-name", "*.rs"],
                       capture_output=True, text=True, check=True).stdout.split()
found, bad_sites = [], []
for f in sorted(files):
    for path, name, ok in offenders(f, open(f).read()):
        found.append(f"{path}::{name}")
        if not ok:
            bad_sites.append(f"{path}::{name}")

floor = int(os.environ["SLEEP_MIN"])
if len(found) < floor:
    print(f"NOSLEEP FAIL: found {len(found)} sleep sites, expected >= {floor}. "
          "A scan that matches nothing reports a clean tree; if a sleep was legitimately "
          "removed, lower SLEEP_MIN deliberately in the invocation.", file=sys.stderr)
    sys.exit(1)
if bad_sites:
    print("NOSLEEP FAIL: sleep outside a deadline-bounded poll: " + ", ".join(bad_sites),
          file=sys.stderr)
    sys.exit(1)
print(f"NOSLEEP OK (leg 1): {len(found)} sleep sites, all inside a deadline-bounded poll: "
      + ", ".join(found))
PY
[ $? -eq 0 ] || exit 1

# --- leg 2: an ABSOLUTE prohibition where it is satisfiable. Every file of the render seam
# names no sleep at all: its tests drive scripted doubles and an injected `now`, so there is
# nothing to wait for.
#
# It deliberately does NOT cover src/watch.rs. That module's one real-watcher test polls
# `drain` to a deadline with a 10ms sleep between iterations — the shape tests/cli.rs already
# uses here and the shape leg 1 accepts. Banning it there would force a `yield_now` busy-spin
# that holds a core for the whole window and, on a loaded two-core runner, competes for CPU
# with the notify thread producing the event it waits for. A sleep INSIDE a deadline-bounded
# poll cannot make an assertion premature, because the condition is re-tested after it; that
# is the whole point, and leg 1 is what governs it.
ABS="src/ui"
missing=""
for p in $ABS; do [ -e "$p" ] || missing="$missing $p"; done
[ -z "$missing" ] || { echo "NOSLEEP FAIL (leg 2):$missing missing" >&2; exit 1; }
h=$(find $ABS -name '*.rs' -print0 2>/dev/null \
    | xargs -0 -I{} grep -nE 'thread::sleep|sleep_ms|park_timeout' {} /dev/null 2>&1 || true)
[ -z "$h" ] || { echo "NOSLEEP FAIL (leg 2): a sleep under $ABS:" >&2; echo "$h" >&2; exit 1; }
echo "NOSLEEP OK (leg 2): no sleep at all under $ABS"

# --- leg 2b: the two seam modules are NOT under leg 2, because src/watch.rs legitimately holds
# the one deadline-bounded 10ms poll. They are capped instead: AT MOST ONE sleep each. Leg 1's
# span splitter attributes a sleeping helper to the preceding test, so a 300ms settle() written
# below group 6's correct deadline-bounded test would inherit its verdict and pass — measured.
# The cap is the plan's own claim ("exactly one sleep, in the one real-watcher test") made
# checkable, and it costs nothing because no other test in either module waits for anything.
for f in src/watch.rs src/refresh.rs src/agents.rs; do
  [ -f "$f" ] || { echo "NOSLEEP FAIL (leg 2b): $f missing" >&2; exit 1; }
  k=$(grep -c 'thread::sleep\|sleep_ms\|park_timeout' "$f" || true)
  [ "$k" -le 1 ] || { echo "NOSLEEP FAIL (leg 2b): $f names a sleep $k times; the seam modules get at most the one deadline-bounded poll" >&2; exit 1; }
done
echo "NOSLEEP OK (leg 2b): src/watch.rs, src/refresh.rs and src/agents.rs name at most one sleep each"
