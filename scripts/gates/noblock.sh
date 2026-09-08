# NOBLOCK — NEW in live-refresh. The render path never blocks on anything but the terminal
# event source, and it never reads a clock. This is hazard 2 ("the watcher must not block the
# draw") and hazard 1 ("no timing-based assertion") made structural.
#
# Two legs, each with its own positive control, and each pattern verified against the real
# tree at planning time rather than assumed:
#
#  1. The PRODUCTION slice of every `.rs` file under src/ui/ names no channel, thread, or
#     blocking-wait API. src/ui/driver.rs is the paradigm case — run_loop reaches the watcher
#     and the worker only through the two trait objects ui::driver::Live carries, whose every
#     method is non-blocking by contract, so there is no expression in that file that can wait
#     on a worker — but the same property is required of the WHOLE directory, not driver.rs
#     alone: a blocking `rx.recv()` behind a `Mutex::lock()` planted in src/ui/app.rs (or any
#     other file under src/ui/) is exactly as much a hazard as one in driver.rs, and was
#     measured, at planning time, to pass every gate in the tree including this one when leg 1
#     read driver.rs only.
#     `\.join\(\)` — with EMPTY parentheses — not a bare `\.join\(`: Path::join and
#     str::join both take an argument and are ordinary, correct code; a JoinHandle's join
#     takes none. A bare pattern would be a false red waiting to happen the first time the
#     loop built a path.
#
#     GAIN-INTEGRITY (G4): leg 1 now sweeps the SAME file set leg 2 already reads — every
#     `.rs` file under src/ui/, found the same way — but stays PRODUCTION-ONLY where leg 2
#     stays WHOLE-FILE, and that asymmetry is deliberate, not an oversight now that the two
#     legs share a file set: a test that spawns a thread to drive a seam double (an
#     `mpsc`/`thread::spawn` fixture standing in for `Refresher`/`AgentPoll`/`Launcher`) is
#     ordinary, correct test code, and stripping `#[cfg(test)]` before this leg's sweep is
#     what keeps such a fixture from ever tripping it. A test that reads the clock is a
#     different animal entirely — it is precisely the timing flake leg 2 exists to prevent, so
#     leg 2 sweeps the whole file, tests included, and stripping would defeat its own purpose.
#     Measured at planning time: the widening is free — no production slice under src/ui/
#     matches leg 1's pattern on `main`, so this repair changes no production code, only the
#     file set this leg walks and its own `OK` line, which now names the file COUNT it swept
#     rather than a single path so a sweep that silently narrowed back to one file is visible.
#  2. NO file under src/ui/ reads a clock, tests included. This one is deliberately
#     whole-file rather than production-only: a TEST that reads the clock is precisely the
#     timing flake this change exists not to reintroduce, and the debounce is a pure state
#     machine taking `now` as a PARAMETER, so no view test has any reason to call
#     Instant::now(). Measured at planning time: zero hits under src/ui on `main`.
#
#     BECAUSE IT IS COMMENT-INCLUSIVE, no doc comment under src/ui/ may spell a clock path
#     either. Verified at planning time: a comment reading "the crate's one `Instant::now()`
#     lives in `watch::RealFsEvents::drain`" — exactly the sentence tasks 9.4 and 13.2 push an
#     implementer toward — turns this leg RED on otherwise-correct code. Say "the clock" or "a
#     clock", never Instant::now(). Same rule MDSEAM imposes on src/ui/markdown.rs's prose
#     about ratatui, and NOTABSEAM shipped red on an unmodified tree in this repository once
#     already for exactly this reason. Prose without `::` is safe.
#
#  3. The two SEAM modules honour their own non-blocking contract. Legs 1 and 2 only ever look
#     under src/ui/, so a blocking `drain` or a blocking `take_result` — the two functions
#     run_loop calls on EVERY FRAME — are invisible to them. That is the gap this leg closes:
#     the traits' "every method is non-blocking" is a doc comment, and a doc comment is not a
#     check. A `drain` written `self.rx.recv_timeout(Duration::from_millis(150))` and a
#     `take_result` written `self.rx.recv_timeout(Duration::from_millis(400)).ok()` satisfy
#     NOCLI-SHELL, both legs above, NOSLEEP, WATCHSEAM, every trait signature, and every
#     frame-and-poll count this change asserts — while delaying each draw by half a second.
#     Guards D and E are what keep leg 3 from being dodged by moving code around rather than by
#     fixing it: D counts each seam module's line-anchored #[cfg(test)] attributes (a second one
#     above `start` hides the worker's whole body from prod()), and E requires `take_result` to
#     be declared above the spawn (below it, a blocking recv lands in the worker half and the
#     leg reports OK). Both measured against stub modules at planning time.
#
# #[cfg(test)] modules are stripped for legs 1 and 3 the way READONLY-UI strips them, because
# src/ui/driver.rs's own tests may legitimately name a channel in a double. Every src/ui/*.rs
# file holds at most ONE line-anchored #[cfg(test)] whose module runs to EOF — verified at
# planning time.
#
# That fact is NOT free for the two new modules, and Guard D below is why. src/refresh.rs is
# REQUIRED to hold a second #[cfg(test)] item, worker_for_test. prod() discards everything from
# the FIRST one to EOF, so a worker_for_test placed above start() hides the worker's entire body
# from this check and from READONLY-UI: measured, a std::fs::write in the worker's start path is
# then reported green, and this check's own spawn control false-reds on a correct tree. A COUNT
# is a check; a placement rule is a convention, so Guard D counts.
set -u
UIDIR="${UIDIR:-src/ui}"
UI_MIN="${UI_MIN:-12}"
fail() { echo "NOBLOCK FAIL: $1" >&2; exit 1; }

[ -d "$UIDIR" ] || fail "no such directory: $UIDIR"
[ -f "$UIDIR/driver.rs" ] || fail "$UIDIR/driver.rs missing - the subject is gone"
[ -f src/refresh.rs ] || fail "src/refresh.rs missing - the positive control has nothing to match"
[ -f src/watch.rs ] || fail "src/watch.rs missing - the positive control has nothing to match"
[ -f src/agents.rs ] || fail "src/agents.rs missing - leg 3 has a third seam module to sweep"

n=$(find "$UIDIR" -name '*.rs' | wc -l | tr -d ' ')
[ "$n" -ge "$UI_MIN" ] || fail "only $n files under $UIDIR (expected >= $UI_MIN)"

prod() { awk 'BEGIN{p=1} /^#\[cfg\(test\)\]$/{p=0} p{print}' "$1"; }

# Guard D — each seam module holds EXACTLY ONE line-anchored #[cfg(test)]. See the header.
for f in src/watch.rs src/refresh.rs src/agents.rs src/launch.rs; do
  c=$(grep -c '^#\[cfg(test)\]$' "$f" || true)
  [ "$c" -eq 1 ] || fail "$f holds $c line-anchored #[cfg(test)] attributes, expected exactly 1 - prod() truncates at the first and every sweep below it is silent"
done

BLOCK_RE='\.recv\(|recv_timeout|try_recv|\.join\(\)|JoinHandle|thread::spawn|thread::sleep|mpsc|Mutex|RwLock|Condvar'
# Leg 3's own pattern is wider than leg 1's: it must also see the ITERATOR forms, which block
# without naming `recv` at all. `self.rx.iter()` and `for r in &self.rx` are both blocking
# receives and both were measured to pass a recv-only pattern.
#
# It is anchored on a receiver-shaped binding (`…rx`) rather than on a bare `\.iter\(\)`:
# `Vec::iter()` is ordinary code that any of these modules may use, and a bare pattern would be
# a false red the first time the worker iterated a path list. `rx` is this crate's own naming
# convention for a Receiver, established by src/cli.rs's existing test.
BLOCK3_RE='\.recv\(|recv_timeout|\.join\(\)|park_timeout|thread::park|rx\.iter\(\)|for +[a-z_]+ +in +&?[a-z_.]*rx'

# Guard A — positive control for leg 1: src/refresh.rs's production slice MUST name a
# channel and a thread spawn, or the worker is not a worker and the pattern is broken.
prod src/refresh.rs | grep -qE 'mpsc' \
  || fail "positive control - src/refresh.rs's production slice names no mpsc"
prod src/refresh.rs | grep -qE 'thread::spawn' \
  || fail "positive control - src/refresh.rs's production slice does not spawn a thread"
prod src/agents.rs | grep -qE 'mpsc' \
  || fail "positive control - src/agents.rs's production slice names no mpsc"
prod src/agents.rs | grep -qE 'thread::spawn' \
  || fail "positive control - src/agents.rs's production slice does not spawn a thread"

hits1=""
for f in $(find "$UIDIR" -name '*.rs' | sort); do
  m=$(prod "$f" | grep -nE "$BLOCK_RE" | sed "s|^|$f:|" || true)
  hits1="$hits1$m"
done
[ -z "$hits1" ] || { echo "NOBLOCK FAIL (leg 1): a channel, thread, or blocking wait in production code under $UIDIR:" >&2
                     echo "$hits1" >&2; exit 1; }
echo "NOBLOCK OK (leg 1): $n files under $UIDIR's production slice name no channel, thread, or blocking wait"

CLOCK_RE='Instant::now|SystemTime::now|\.elapsed\(\)|Instant::'

# Guard B — positive control for leg 2: src/watch.rs MUST read the clock, since it holds the
# crate's one binding from the debounce's injected `now` to the real one. If it does not,
# either the pattern is broken or the debounce grew a hidden clock somewhere else.
grep -qE "$CLOCK_RE" src/watch.rs \
  || fail "positive control - src/watch.rs names no clock; the one real clock binding is missing"

c=$(find "$UIDIR" -name '*.rs' -print0 | xargs -0 -I{} grep -nE "$CLOCK_RE" {} /dev/null 2>&1 || true)
[ -z "$c" ] || { echo "NOBLOCK FAIL (leg 2): a clock is read under $UIDIR:" >&2
                 echo "$c" >&2; exit 1; }
echo "NOBLOCK OK (leg 2): $n files under $UIDIR read no clock, tests included; both controls matched"

# Leg 3 — the two seam modules honour their own non-blocking contract.
# src/watch.rs may not block on a channel AT ALL: RealFsEvents::drain is a try_recv loop and
# nothing in that module runs off the render path.
w=$(prod src/watch.rs | grep -nE "$BLOCK3_RE" || true)
[ -z "$w" ] || { echo "NOBLOCK FAIL (leg 3): src/watch.rs blocks on a channel:" >&2
                 echo "$w" >&2; exit 1; }
prod src/watch.rs | grep -q 'try_recv' \
  || fail "positive control - src/watch.rs's production slice names no try_recv; drain is not draining"

# src/refresh.rs MAY block, but only inside the worker body, which is everything after the
# single thread::spawn in the production slice. Refresher::take_result is declared before it
# and must be try_recv.
# awk is used ONLY to CUT the file at its single thread::spawn; the matching is grep's, because
# an ERE passed to awk through -v is re-escaped as a string literal and awk rejects it
# ("illegal primary in regular expression") — measured, and it fails OPEN, printing to stderr
# while the leg reports OK. One tool, one job.
#
# The cut SKIPS COMMENT LINES. Measured: a module doc comment on line 1 reading "the worker is
# started by a single `thread::spawn` in `start`" — the natural sentence to write, and one
# SPEC.md is being edited to say — would otherwise cut at line 1 and leave nothing to search.
# `^[^/]*thread::spawn` requires the token to be preceded on its line by no `/`, which excludes
# `//`, `///`, and `//!`.
# Guard E — take_result is DECLARED BEFORE the spawn, so the textual half this leg searches is
# the render-path half. Without it the rule is a convention rather than a check: moving
# take_result below start() puts a blocking recv into the worker half and the leg reports OK.
# Measured. specs/refresh-worker states the ordering; this is what enforces it.
#
# REPAIRED in live-refresh's own Change Review: `grep -n 'fn take_result' | head -1` always
# matches the Refresher TRAIT's abstract signature (declared once, near the top of the file,
# necessarily before any impl and any thread::spawn), so moving the CONCRETE impl's
# take_result below start() left tr_line pinned to the trait line and never tripped this
# guard. Measured: the production slice holds three `fn take_result` lines (the trait,
# NoRefresher's impl, RealRefresher's impl, in that file order); moving the third below
# start() still reported OK under `head -1`. `tail -1` takes the LAST such line instead — the
# concrete impl closest to EOF, the one whose position this guard must actually track — and
# correctly reports FAIL once that line moves past thread::spawn. Verified against that exact
# move: red under the fixed check, OK again once reverted.
tr_line=$(prod src/refresh.rs | grep -n 'fn take_result' | tail -1 | cut -d: -f1)
sp_line=$(prod src/refresh.rs | grep -nE '^[^/]*thread::spawn' | head -1 | cut -d: -f1)
[ -n "$tr_line" ] || fail "src/refresh.rs's production slice declares no take_result"
[ -n "$sp_line" ] || fail "src/refresh.rs's production slice has no thread::spawn outside a comment"
[ "$tr_line" -lt "$sp_line" ] \
  || fail "src/refresh.rs declares take_result at line $tr_line, below its thread::spawn at line $sp_line; leg 3 would then search the worker half instead of the render-path half"

head3=$(prod src/refresh.rs | awk '/^[^\/]*thread::spawn/{exit} {print}')
b=$(printf '%s\n' "$head3" | grep -nE "$BLOCK3_RE" || true)
[ -z "$b" ] || { echo "NOBLOCK FAIL (leg 3): src/refresh.rs blocks before its thread::spawn," >&2
                 echo "i.e. on the render path rather than in the worker:" >&2
                 echo "$b" >&2; exit 1; }
prod src/refresh.rs | grep -q 'try_recv' \
  || fail "positive control - src/refresh.rs's production slice names no try_recv; take_result blocks"

# src/agents.rs — the same shape as src/refresh.rs: it MAY block, but only inside the worker
# body, which is everything after the single thread::spawn in the production slice.
# AgentPoll::drain is declared before it and must not block.
# Guard E for src/agents.rs — `tail -1`, not `head -1`: src/agents.rs declares `fn drain` three
# times (the trait's abstract signature, NoAgentPoll's impl, and RealAgentPoll's impl) and the
# first necessarily precedes every spawn. Measured: under `head -1`, moving RealAgentPoll's
# drain below start reported OK; under `tail -1` it reported FAIL.
dr_line=$(prod src/agents.rs | grep -n 'fn drain' | tail -1 | cut -d: -f1)
sp_line3=$(prod src/agents.rs | grep -nE '^[^/]*thread::spawn' | head -1 | cut -d: -f1)
[ -n "$dr_line" ] || fail "src/agents.rs's production slice declares no drain"
[ -n "$sp_line3" ] || fail "src/agents.rs's production slice has no thread::spawn outside a comment"
[ "$dr_line" -lt "$sp_line3" ] \
  || fail "src/agents.rs declares drain at line $dr_line, below its thread::spawn at line $sp_line3; leg 3 would then search the worker half instead of the render-path half"

head4=$(prod src/agents.rs | awk '/^[^\/]*thread::spawn/{exit} {print}')
a=$(printf '%s\n' "$head4" | grep -nE "$BLOCK3_RE" || true)
[ -z "$a" ] || { echo "NOBLOCK FAIL (leg 3): src/agents.rs blocks before its thread::spawn," >&2
                 echo "i.e. on the render path rather than in the worker:" >&2
                 echo "$a" >&2; exit 1; }
prod src/agents.rs | grep -q 'try_recv' \
  || fail "positive control - src/agents.rs's production slice names no try_recv; drain blocks"

# src/launch.rs — the same shape again, and the crate's FOURTH seam module: it MAY block, but
# only inside the worker body, which is everything after the single thread::spawn in the
# production slice. Launcher::drain is declared before it and must not block.
# Guard E for src/launch.rs — `tail -1`, not `head -1`, for the reason recorded on
# src/agents.rs above: src/launch.rs declares `fn drain` three times (the Launcher trait's
# abstract signature, NoLaunch's impl, and Live's impl) and the first necessarily precedes
# every spawn. Measured: under `head -1`, moving Live's drain below start reported OK; under
# `tail -1` it reported FAIL.
dr_line2=$(prod src/launch.rs | grep -n 'fn drain' | tail -1 | cut -d: -f1)
sp_line4=$(prod src/launch.rs | grep -nE '^[^/]*thread::spawn' | head -1 | cut -d: -f1)
[ -n "$dr_line2" ] || fail "src/launch.rs's production slice declares no drain"
[ -n "$sp_line4" ] || fail "src/launch.rs's production slice has no thread::spawn outside a comment"
[ "$dr_line2" -lt "$sp_line4" ] \
  || fail "src/launch.rs declares drain at line $dr_line2, below its thread::spawn at line $sp_line4; leg 3 would then search the worker half instead of the render-path half"

head5=$(prod src/launch.rs | awk '/^[^\/]*thread::spawn/{exit} {print}')
l=$(printf '%s\n' "$head5" | grep -nE "$BLOCK3_RE" || true)
[ -z "$l" ] || { echo "NOBLOCK FAIL (leg 3): src/launch.rs blocks before its thread::spawn," >&2
                 echo "i.e. on the render path rather than in the worker:" >&2
                 echo "$l" >&2; exit 1; }
prod src/launch.rs | grep -q 'try_recv' \
  || fail "positive control - src/launch.rs's production slice names no try_recv; drain blocks"

echo "NOBLOCK OK (leg 3): src/watch.rs never blocks; src/refresh.rs, src/agents.rs and src/launch.rs block only after thread::spawn"
