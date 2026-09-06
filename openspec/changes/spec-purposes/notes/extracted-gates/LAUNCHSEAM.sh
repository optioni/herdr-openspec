# LAUNCHSEAM — NEW in agent-launch. The launcher is confined to ONE module, on exactly
# AGENTSEAM's, WATCHSEAM's, and MDSEAM's single-file terms: the Herdr program is reachable by
# editing src/cli.rs, src/agents.rs, src/ui/mod.rs, and src/launch.rs, and no other file in
# the crate — tests/ included — may reach for it.
#
# Leg 1 is the file-scoped restatement of NOSPAWN-GREP, and it is the whole point of this
# check: src/launch.rs is the file whose JOB is to open a pane, so reaching for
# std::process::Command here — rather than asking the session through the injected handle —
# is the single most plausible way this change goes wrong. A launcher that spawns its own
# child is untestable and unobservable, and every other gate stays green while it does it.
# Leg 2 is AGENTSEAM leg 2's rule: the confined module is PLAIN DATA, so an Outcome never
# arrives at the view already styled. COMMENT-INCLUSIVE on purpose — a doc comment naming a
# view type is itself the coupling this forbids, so say "the view", never `Frame`.
# Leg 3 confines the Herdr handle itself. The pattern is HerdrCli|RealHerdrCli|agent_cli_via,
# not HerdrCli alone, and that is measured rather than cautious: agent_cli_via returns
# Arc<dyn HerdrCli> and inference hides the type, so a bare HerdrCli pattern reported OK on a
# tree carrying crate::cli::agent_cli_via(Path::new("/bin/herdr")) planted in src/ui/list.rs.
# src/ui/mod.rs is in ALLOWED because the composition root legitimately calls agent_cli_via;
# it is still forbidden to name HerdrCli itself by NOCLI-SHELL, which sweeps all of src/ui/,
# so the two checks compose without either one being weakened.
#
# Legs 1 and 2 read the PRODUCTION slice only, so a test that names Command::new to prove the
# launcher does NOT reach for it is not itself a violation. That makes Guard D below
# load-bearing rather than decorative: with two line-anchored #[cfg(test)] attributes the
# first one still truncates, but with ZERO (an attribute with a trailing space, say) prod()
# keeps the whole file and the legs get STRICTER, never weaker — the dangerous direction is
# a stray `#[cfg(test)]` ABOVE the real code, which the count guard is what catches.
set -u
LAUNCH="${LAUNCH:-src/launch.rs}"
ALLOWED="${ALLOWED-src/cli.rs src/agents.rs src/ui/mod.rs src/launch.rs}"
# Measured on the agent-launch tree: src/ + tests/ hold 26 *.rs files, four of them ALLOWED,
# so leg 3 searches 22. The floor is the realized count, not a round number below it: this
# repository only ever adds files, so a drop below 22 means a file was deleted or the find
# expression rotted, and either way leg 3's clean result would mean nothing.
MIN="${MIN:-22}"
fail() { echo "LAUNCHSEAM FAIL: $1" >&2; exit 1; }

[ -d src ] || fail "no src directory"
[ -d tests ] || fail "no tests directory - leg 3 would search half the crate"
[ -f "$LAUNCH" ] || fail "$LAUNCH missing - the subject is gone"
case "$ALLOWED" in *[![:space:]]*) ;;
  *) fail "ALLOWED is empty - leg 3 would forbid the declaration itself";; esac

# Guard A — positive control, checked BEFORE every leg: the module must actually define the
# launcher's entry point, or this is not the launcher and a clean result means nothing.
# ANCHORED on a definition form, never a bare substring: an unanchored `start` is satisfied
# by the doc comment on `none()` that mentions it.
# PARAMETERISED in plugin-actions: ENTRY defaults to LAUNCHSEAM's original subject
# (src/launch.rs's `pub fn start(`) but can be pointed at a second file's own entry point
# (src/open.rs's `pub fn run_from_env(`) so this same block guards both without a second
# gate joining the roster (design.md -> Decision 9).
ENTRY="${ENTRY:-pub fn start\(}"
grep -qE "^$ENTRY" "$LAUNCH" \
  || fail "positive control - $LAUNCH defines no '$ENTRY'"

# Guard D — $LAUNCH holds EXACTLY ONE line-anchored #[cfg(test)], since prod() truncates at
# the first and legs 1 and 2 are blind below it. Measured elsewhere in this repository: with a
# single TRAILING SPACE after the attribute the anchored count is 0, and a file whose real
# code sits below a mis-spelled attribute is searched in full — safe — while a stray attribute
# ABOVE the real code hides everything. WIRED carries this guard for src/ui/mod.rs; LAUNCHSEAM
# needs it for the same reason.
c=$(grep -c '^#\[cfg(test)\]$' "$LAUNCH" || true)
[ "$c" -eq 1 ] || fail "$LAUNCH holds $c line-anchored #[cfg(test)] attributes, expected exactly 1 - legs 1 and 2 are blind below the first one"

prod() { awk 'BEGIN{p=1} /^#\[cfg\(test\)\]$/{p=0} p{print FILENAME":"FNR": "$0}' "$1"; }

# Guard B — the searched set is real. Excluded BY PATH, never by base name, so a future
# src/ui/launch.rs is still searched.
pruned=""
for f in $ALLOWED; do
  [ -f "$f" ] || fail "ALLOWED names $f, which does not exist - the exclusion is vacuous"
  pruned="$pruned ! -path $f"
done
# shellcheck disable=SC2086
n=$(find src tests -name '*.rs' $pruned | wc -l | tr -d ' ')
[ "$n" -ge "$MIN" ] || fail "searched only $n files (expected >= $MIN)"

# Leg 1 — the launcher spawns no process. It asks the session, through the handle.
p=$(prod "$LAUNCH" | grep -E 'process::Command|Command::new|Stdio' || true)
[ -z "$p" ] || { echo "LAUNCHSEAM FAIL (leg 1): $LAUNCH spawns a process:" >&2
                 echo "$p" >&2
                 echo "reach the session through Arc<dyn HerdrCli> instead - a child this module owns is unobservable" >&2
                 exit 1; }

# Leg 2 — the launcher names no view type, comments included.
r=$(prod "$LAUNCH" | grep -E 'ratatui|Frame|Rect|Buffer|Style' || true)
[ -z "$r" ] || { echo "LAUNCHSEAM FAIL (leg 2): $LAUNCH names a view type:" >&2
                 echo "$r" >&2
                 echo "Outcome is plain data - the view styles it, this module does not" >&2
                 exit 1; }

# Leg 3 — the Herdr handle is reached only from the allowed files.
HANDLE_RE='HerdrCli|RealHerdrCli|agent_cli_via'
# shellcheck disable=SC2086
hits=$(find src tests -name '*.rs' $pruned -print0 \
       | xargs -0 -I{} grep -nE "$HANDLE_RE" {} /dev/null 2>&1 || true)
[ -z "$hits" ] || { echo "LAUNCHSEAM FAIL (leg 3): the Herdr handle is reached outside:$ALLOWED" >&2
                    echo "$hits" >&2; exit 1; }

# Guard C — the exclusion is not vacuous in the other direction either: src/cli.rs must
# define the constructor leg 3 is confining, or leg 3 is excluding a file that says nothing.
# ANCHORED on a definition form for the same reason as Guard A.
grep -qE '^pub fn agent_cli_via\(' src/cli.rs \
  || fail "positive control - src/cli.rs defines no 'pub fn agent_cli_via('"

echo "LAUNCHSEAM OK: $n files searched (>= $MIN); no spawn and no view type in $LAUNCH's production slice; Herdr handle only in:$ALLOWED"
