# AGENTSEAM — NEW in agent-polling. The agent poller is confined to ONE module, on exactly
# WATCHSEAM's, MDSEAM's, and NOSPAWN-GREP's single-file terms: the Herdr program is reachable
# by editing src/agents.rs and src/cli.rs, and no other file in the crate — tests/ included —
# may reach for it.
#
# Leg 1 is the file-scoped restatement of NOSPAWN-GREP, because src/agents.rs is the new file
# where reaching for a `herdr` subprocess directly would be most plausible.
# Leg 2 is WATCHSEAM leg 2's rule verbatim: the confined module is PLAIN DATA, so a polled
# agent never arrives at the view already styled. COMMENT-INCLUSIVE on purpose — a doc comment
# naming a view type is itself the coupling this forbids, so say "the view", never `Frame`.
# Leg 3 confines the Herdr handle itself. The pattern is HerdrCli|RealHerdrCli|agent_cli_via,
# not HerdrCli alone, and that is measured rather than cautious: agent_cli_via returns
# Arc<dyn HerdrCli> and inference hides the type, so a bare HerdrCli pattern reported OK on a
# tree carrying crate::cli::agent_cli_via(Path::new("/bin/herdr")) planted in src/ui/list.rs.
# src/ui/mod.rs is in ALLOWED because the composition root legitimately calls agent_cli_via;
# it is still forbidden to name HerdrCli itself by NOCLI-SHELL, which sweeps all of src/ui/,
# so the two checks compose without either one being weakened. agent-launch adds
# src/launch.rs to ALLOWED as a deliberate edit; a silent extra namer is what this catches.
set -u
AGENTS="${AGENTS:-src/agents.rs}"
ALLOWED="${ALLOWED-src/cli.rs src/agents.rs src/ui/mod.rs src/launch.rs src/open.rs}"
MIN="${MIN:-25}"
fail() { echo "AGENTSEAM FAIL: $1" >&2; exit 1; }

[ -d src ] || fail "no src directory"
[ -f "$AGENTS" ] || fail "$AGENTS missing - the subject is gone"
case "$ALLOWED" in *[![:space:]]*) ;;
  *) fail "ALLOWED is empty - leg 3 would forbid the declaration itself";; esac

# Guard A — positive control, checked BEFORE every leg: the module must actually name the
# Herdr seam trait, or this is not the poller and a clean result means nothing.
grep -q 'HerdrCli' "$AGENTS" || fail "$AGENTS names no HerdrCli - the subject is not the poller"

# Guard B — the searched set is real. Excluded BY PATH, never by base name, so a future
# src/ui/agents.rs is still searched.
pruned=""
for f in $ALLOWED; do
  [ -f "$f" ] || fail "ALLOWED names $f, which does not exist - the exclusion is vacuous"
  pruned="$pruned ! -path $f"
done
# shellcheck disable=SC2086
n=$(find src tests -name '*.rs' $pruned | wc -l | tr -d ' ')
[ "$n" -ge "$MIN" ] || fail "searched only $n files (expected >= $MIN)"

# Leg 1 — the module spawns no process.
p=$(grep -nE 'process::Command|Command::new|Stdio' "$AGENTS" || true)
[ -z "$p" ] || { echo "AGENTSEAM FAIL (leg 1): $AGENTS spawns a process:" >&2
                 echo "$p" >&2; exit 1; }

# Leg 2 — the module names no view type, comments included.
r=$(grep -nE 'ratatui|Modifier|Style|Span|Rect|Frame|Buffer' "$AGENTS" || true)
[ -z "$r" ] || { echo "AGENTSEAM FAIL (leg 2): $AGENTS names a ratatui type:" >&2
                 echo "$r" >&2; exit 1; }

# Leg 3 — the Herdr handle is reached only from the allowed files.
HANDLE_RE='HerdrCli|RealHerdrCli|agent_cli_via'
# shellcheck disable=SC2086
hits=$(find src tests -name '*.rs' $pruned -print0 \
       | xargs -0 -I{} grep -nE "$HANDLE_RE" {} /dev/null 2>&1 || true)
[ -z "$hits" ] || { echo "AGENTSEAM FAIL (leg 3): the Herdr handle is reached outside:$ALLOWED" >&2
                    echo "$hits" >&2; exit 1; }

# Guard C — the exclusion is not vacuous in the other direction either: src/cli.rs must
# declare the trait, or leg 3 is excluding a file that says nothing.
# ANCHORED on both sides. An unanchored `trait HerdrCli` is satisfied by
# `trait HerdrClient` after a rename, so the control would pass while leg 3 searched for a
# name that is no longer declared anywhere. Measured: the unanchored form stayed green
# against exactly that rename.
grep -qE 'trait[[:space:]]+HerdrCli[[:space:]]*:' src/cli.rs \
  || fail "positive control - src/cli.rs declares no 'trait HerdrCli:'"

echo "AGENTSEAM OK: $n files searched (>= $MIN); no spawn and no view type in $AGENTS; Herdr handle only in:$ALLOWED"
