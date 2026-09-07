# WIRED — NEW in agent-polling. The composition root actually starts what it composes, and
# the residue left in `run` is small enough that reading it is a proof.
#
# Extracted to scripts/gates/ by degraded-states: this check lived outside `make check` since
# it was written, and went red on `main` for a whole change (`plugin-actions` put a branch into
# `run`'s own body; nothing forced this script to run and notice). Composed into `make gates`
# from here on, with leg 5b added for the same repair.
#
# WHY THIS EXISTS: live-refresh shipped a `ui::run` that called neither watch::start nor
# refresh::start. Every test passed, because unit and view tests drove the seams directly and
# nothing exercised the function that composes them. This check is the source-level half of
# the answer; the behavioural half is ui::tests::wiring's acceptance test, and NEITHER is
# sufficient alone — a grep proves a name appears, not that its result reaches `Live`, and a
# test proves the wiring at the moment it ran, not that a later edit kept it.
#
# Leg 2 is the one that keeps this honest over time. It requires `pub fn run`'s own body to
# hold no branch and no loop, so the untested residue cannot grow a condition under which a
# collaborator is silently dropped. Everything with a decision in it lives in run_wired, which
# the acceptance test drives.
set -u
MOD="${MOD:-src/ui/mod.rs}"
CLI="${CLI:-src/cli.rs}"
STATE="${STATE:-src/state.rs}"
LAUNCH="${LAUNCH:-src/launch.rs}"
TERMINAL="${TERMINAL:-src/ui/terminal.rs}"
UIDIR="${UIDIR:-src/ui}"
fail() { echo "WIRED FAIL: $1" >&2; exit 1; }

[ -f "$MOD" ] || fail "$MOD missing - the subject is gone"
[ -f "$CLI" ] || fail "$CLI missing - the positive controls have nothing to match"
[ -f "$STATE" ] || fail "$STATE missing - the mapping-read control has nothing to match"
[ -f "$LAUNCH" ] || fail "$LAUNCH missing - leg 1's ninth name would point at nothing"
[ -f "$TERMINAL" ] || fail "$TERMINAL missing - the panic-hook positive control has nothing to match"
[ -d "$UIDIR" ] || fail "$UIDIR missing - leg 4 has nothing to search"

# Guard D — $MOD holds EXACTLY ONE line-anchored #[cfg(test)], since prod() truncates at the
# first and every leg below it is silent. Measured: with the names present only inside
# `mod tests` and a single TRAILING SPACE after the attribute (so the anchored count is 0 and
# prod() keeps the whole file), an entirely unwired start_collaborators reported WIRED OK.
# NOBLOCK carries this guard for its seam modules; WIRED needs it for the same reason.
c=$(grep -c '^#\[cfg(test)\]$' "$MOD" || true)
[ "$c" -eq 1 ] || fail "$MOD holds $c line-anchored #[cfg(test)] attributes, expected exactly 1 - every leg below the first one is silent"

prod() { awk 'BEGIN{p=1} /^#\[cfg\(test\)\]$/{p=0} p{print}' "$1"; }
# CODE, not text. Every leg below searches the production slice with comments STRIPPED, and
# that is a measured requirement rather than tidiness. src/ui/mod.rs's doc comment on `run`
# names `watch::start` and `crate::refresh::start` in prose; measured, a plant that replaced
# the real `refresh::start(...)` call with `refresh::none()` left that comment in place and
# leg 1 stayed GREEN on a genuinely unwired composition root - the exact defect this check
# exists to catch. Stripping `//` to end-of-line can also truncate a `//` inside a string
# literal, which only makes a must-be-present leg stricter and a must-be-absent leg no
# weaker, so it fails in the safe direction.
#
# G5/G8: `//` alone left a name surviving inside a `/* ... */` block comment satisfying every
# leg below - measured, replacing the real `crate::launch::start(` call with
# `/* crate::launch::start( is gone */ crate::launch::begin(` reported `OK: twelve names
# present` on a launcher that was unwired. strip_block_comments is a hand-rolled scanner
# rather than a single sed/awk regex DELIBERATELY: a block comment can span lines, and a
# regex-based multi-line strip is exactly where BSD sed/awk and GNU sed/awk diverge (this
# gate runs on both ubuntu-latest and macos-latest). The scanner below uses only
# index()/substr() with an `incomment` flag carried across awk's per-line NR loop - no RS
# trick, no multi-line regex - so it behaves identically under BSD awk (macOS) and gawk/mawk
# (Linux). Over-stripping (treating `/*` inside a string literal as a real comment opener) is
# the same "fails in the safe direction" tradeoff the `//` strip above already accepts.
strip_block_comments() {
  awk '
  {
    line = $0
    out = ""
    while (length(line) > 0) {
      if (incomment) {
        idx = index(line, "*/")
        if (idx > 0) {
          line = substr(line, idx + 2)
          incomment = 0
        } else {
          line = ""
        }
      } else {
        idx = index(line, "/*")
        if (idx > 0) {
          out = out substr(line, 1, idx - 1)
          line = substr(line, idx + 2)
          incomment = 1
        } else {
          out = out line
          line = ""
        }
      }
    }
    print out
  }
  '
}
code() { prod "$1" | sed 's://.*::' | strip_block_comments; }

# Guard E — code() ACTUALLY STRIPS block comments, and is not line-scoped, isolated from leg
# 1's real search over $MOD. Two inline fixtures, in the shape leg 1's own
# wired-launch-comment control plants into $MOD (`/* crate::launch::start( is gone */
# crate::launch::begin(`), written to a SCRATCH FILE and read through code() itself (not
# strip_block_comments called directly) - so a plant that reverts only code()'s composition
# (drops "| strip_block_comments" and leaves the function that scanner defines untouched) is
# still caught here rather than only in a unit that no longer runs. A single-line block
# comment hides `launch::start`, and the same shape spans three lines. This is what makes
# "reverting only the /* ... */ half of code(), leaving the // strip in place" fail on its
# own: an identity-function control alone proves nothing about the addition (planning
# review), but a control anchored on the // strip's own fixed point - text with no `//` in it
# at all - fails the moment the block-comment half is missing, whatever else code() still does.
guard_e_tmp=$(mktemp "${TMPDIR:-/tmp}/wired-guard-XXXXXX")
trap 'rm -f "$guard_e_tmp"' EXIT

printf '%s\n' '/* crate::launch::start( is gone */ crate::launch::begin(' > "$guard_e_tmp"
s=$(code "$guard_e_tmp")
printf '%s\n' "$s" | grep -q 'launch::start' \
  && fail "block-comment guard: code() leaves 'launch::start' intact inside a single-line /* ... */ comment - a stripper reduced to the // strip alone cannot pass"
printf '%s\n' "$s" | grep -q 'launch::begin' \
  || fail "block-comment guard: code() deleted text outside the single-line comment too - it is not a targeted strip"

printf '%s\n' '/* crate::launch::start(' 'is gone, spanning' 'three lines */ crate::launch::begin(' > "$guard_e_tmp"
m=$(code "$guard_e_tmp")
printf '%s\n' "$m" | grep -q 'launch::start' \
  && fail "block-comment guard: code() leaves 'launch::start' intact inside a comment spanning three lines - the stripper is line-scoped"
printf '%s\n' "$m" | grep -q 'launch::begin' \
  || fail "block-comment guard: code() deleted text outside the multi-line comment too - it is not a targeted strip"

rm -f "$guard_e_tmp"
trap - EXIT

# Guard A — positive controls live in the OTHER file, so a renamed binding fails HERE rather
# than leaving leg 1 searching for a name that is no longer defined anywhere. Each is anchored
# on a definition form, never a bare substring: an unanchored `agent_cli_via` is satisfied by
# a doc comment mentioning it.
grep -qE '^pub fn agent_cli_via\(' "$CLI" \
  || fail "positive control - $CLI defines no 'pub fn agent_cli_via('"
grep -qE '^pub fn worker_cli_from_env\(' "$CLI" \
  || fail "positive control - $CLI defines no 'pub fn worker_cli_from_env('"
grep -qE '^pub fn worker_cli\(' "$CLI" \
  || fail "positive control - $CLI defines no 'pub fn worker_cli('"
grep -qE '^pub fn npm_probe_hook\(' "$CLI" \
  || fail "positive control - $CLI defines no 'pub fn npm_probe_hook(' - degraded-states's wrapper around the real npm-prefix binding, named so ui/ can reach it without NOCLI-SHELL's CLI_RE firing"
grep -qE '^pub const HERDR_PROGRAM' "$CLI" \
  || fail "positive control - $CLI defines no 'pub const HERDR_PROGRAM'"
grep -qE '^pub fn read\(' "$STATE" || fail "positive control - $STATE defines no 'pub fn read('"
grep -qE '^pub fn start\(' "$LAUNCH" || fail "positive control - $LAUNCH defines no 'pub fn start('"
# G5: anchored on the DEFINING file rather than left to leg 1 alone, so a rename fails HERE,
# naming $TERMINAL, rather than leg 1 hunting a name nobody defines any more.
grep -qE '^pub fn install_panic_hook\(' "$TERMINAL" \
  || fail "positive control - $TERMINAL defines no 'pub fn install_panic_hook('"

# Leg 1 — every collaborator the loop needs is started BY NAME in the production slice.
# degraded-states: worker_cli_from_env dropped out of this list (start_collaborators reaches
# the probe through resolve::openspec_bin and worker_cli directly, injecting env/npm_hook
# rather than hardcoding the real bindings inside worker_cli_from_env - design.md ->
# Decision 14); config::env_lookup( and npm_probe_hook joined it, naming the two real
# bindings the composition root itself supplies. gate-integrity (G5): install_panic_hook
# joined it too - the call was named nowhere on this list, so deleting the single
# `terminal::install_panic_hook();` line in `run` left every gate and every test green.
for n in run_wired start_collaborators 'watch::start' 'refresh::start' 'agents::start' \
         'resolve::openspec_bin' 'cli::worker_cli' agent_cli_via 'state::read' 'launch::start' \
         'config::env_lookup(' npm_probe_hook install_panic_hook; do
  code "$MOD" | grep -q -- "$n" \
    || fail "leg 1: $MOD's production slice does not name $n - the name is gone; leg 1 sees names, not values, so a call whose result is dropped still passes here and is the acceptance test's job"
done

# Leg 2 — `pub fn run`'s own body holds no branch and no loop. The body is cut from the
# production slice between its signature line and the next column-zero `}`.
# KNOWN LIMIT, stated rather than left to be discovered: the pattern is keyword-based, so a
# keyword-free decision (`x.unwrap_or_else(|| ...)`, `?` on an Option, a match hidden in a
# closure) passes. Measured. Leg 2's claim is only what it checked; the residue's real
# guarantee is its length plus the acceptance test below it.
body=$(code "$MOD" | awk '/^pub fn run\(\)/{f=1} f{print} f && /^\}$/{exit}')
[ -n "$body" ] || fail "leg 2: could not find 'pub fn run()' in $MOD's production slice"
printf '%s\n' "$body" | grep -q '^}$' \
  || fail "leg 2: 'pub fn run()' has no closing brace at column zero - the cut ran to EOF"
b=$(printf '%s\n' "$body" | grep -nE '(^|[^A-Za-z0-9_])(if|match|for|while|loop|else)([^A-Za-z0-9_]|$)' || true)
[ -z "$b" ] || { echo "WIRED FAIL (leg 2): 'pub fn run()' holds a branch or a loop:" >&2
                 echo "$b" >&2
                 echo "everything with a decision in it belongs in run_wired, which is tested" >&2
                 exit 1; }

# Leg 3 — the residue delegates: run calls run_wired, rather than naming it in a comment.
printf '%s\n' "$body" | grep -qE 'run_wired\(' \
  || fail "leg 3: 'pub fn run()' does not call run_wired("

# Leg 5 — NEW in agent-attribution. `run` resolves the state directory and threads the result
# into Startup. Leg 1 sees `state::read` wherever it appears, and it will appear inside `load`,
# which fifteen tests drive; the untested link is the VALUE `run` passes, and a shipped binary
# passing `None` has a permanently dead tier-1 mapping with every other gate green.
printf '%s\n' "$body" | grep -q 'state::state_dir(' \
  || fail "leg 5: 'pub fn run()' does not resolve state::state_dir( - Startup.state_dir would be a literal"
if printf '%s\n' "$body" | grep -q 'state_dir: None'; then
  fail "leg 5: 'pub fn run()' hardcodes 'state_dir: None' - the mapping tier would be dead in the shipped binary"
fi

# Leg 5b — NEW in degraded-states. `plugin-actions` put the cwd-resolution branch into
# `run`'s own body (`match startup_cwd(...) { Some(cwd) => cwd, None => std::env::current_dir()? }`),
# which is exactly leg 2's failure mode and is why this check went red on `main` and stayed
# red for a whole change: it lived outside `make check`. The repair moves the decision into
# `ui::startup_dir`, a named function both of whose arms a test drives. BOTH halves below are
# needed, on design.md -> Decision 5's own terms: leg 2 alone passes on a body that dropped the
# `startup_dir(` call entirely and inlined `std::env::current_dir()?` again, so the name-leg is
# the one that catches that specific regression rather than merely "no branch, no loop".
printf '%s\n' "$body" | grep -q 'startup_dir(' \
  || fail "leg 5b: 'pub fn run()' does not name startup_dir( - the cwd-resolution decision may have been reinlined"

# Leg 6 — NEW in agent-launch. The launcher is started with the CONFIGURED agent kind, not a
# literal. Two halves, and both are needed:
#   (i)  $MOD's production slice must NAME config.agent_kind. Searched over the whole
#        production slice rather than `run`'s body, because the value is threaded in
#        start_collaborators, which `run` never sees.
#   (ii) the "claude" literal appears in NO production slice under $UIDIR. agent_kind already
#        defaults to "claude" in src/config.rs, so a start_collaborators that spells the
#        default itself can still name config.agent_kind elsewhere in the file and pass half
#        (i) while shipping a dashboard that ignores the operator's setting.
# SCOPED exactly like leg 4: under $UIDIR only, stripping at the first line-anchored
# #[cfg(test)]. Measured: the literal appears TEN times under src/ui/ today, every one inside
# a test slice, and ZERO times after stripping.
code "$MOD" | grep -q 'config.agent_kind' \
  || fail "leg 6: $MOD's production slice does not name config.agent_kind - the launcher would run some other agent than the configured one"
k=$(find "$UIDIR" -name '*.rs' -print0 \
    | xargs -0 -I{} sh -c 'awk "BEGIN{p=1} /^#\\[cfg\\(test\\)\\]\$/{p=0} p{print FILENAME\":\"FNR\": \"\$0}" "$1"' _ {} \
    | grep -E '"claude"' || true)
[ -z "$k" ] || { echo "WIRED FAIL (leg 6): the \"claude\" literal appears under $UIDIR:" >&2
                 echo "$k" >&2
                 echo "pass config.agent_kind instead - the default lives in src/config.rs, not here" >&2
                 exit 1; }

# Leg 4 — the composition root does not hardcode the program name. The "herdr" literal
# appears nowhere under src/ui/, so `run` must reach cli::HERDR_PROGRAM rather than spelling
# the program itself and bypassing the one place it is written down.
#
# SCOPED TO src/ui/ deliberately, and that is a measured scope rather than a cautious one: a
# tree-wide form is RED on unmodified `main`, because src/config.rs and src/state.rs both
# build `.join("herdr")` for the plugin's configuration and state directories, which is
# correct, unrelated code. A tree-wide leg would have had to exempt two files on its first
# run, which is how a check rots into a rubber stamp.
h=$(find "$UIDIR" -name '*.rs' -print0 \
    | xargs -0 -I{} sh -c 'awk "BEGIN{p=1} /^#\\[cfg\\(test\\)\\]\$/{p=0} p{print FILENAME\":\"FNR\": \"\$0}" "$1"' _ {} \
    | grep -E '"herdr"' || true)
[ -z "$h" ] || { echo "WIRED FAIL (leg 4): the \"herdr\" literal appears under $UIDIR:" >&2
                 echo "$h" >&2
                 echo "reach cli::HERDR_PROGRAM instead - it is the one place the name lives" >&2
                 exit 1; }

lines=$(printf '%s\n' "$body" | wc -l | tr -d ' ')
echo "WIRED OK: thirteen names present in $MOD; run resolves state::state_dir; run names startup_dir(; $MOD names config.agent_kind; 'pub fn run()' is $lines lines with no branch and no loop; no \"herdr\" and no \"claude\" literal under $UIDIR"
