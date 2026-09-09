# NORAW-GREP — crossterm's terminal-mode functions appear ONLY in src/ui/terminal.rs, and
# CrosstermOps is constructed only by ui::run. Searches tests/ as well as src/, because the
# failure this prevents is a TEST putting the developer's own terminal into raw mode.
# Extracted to scripts/gates/ by degraded-states: its file-count guard was hardcoded at
# 16 (list-view's own measurement); re-measured here at 29 and fixed to match, since a
# stale hardcoded floor is exactly this project's own recurring defect class.
# mouse-input: the two mouse-capture commands join the four terminal-mode functions. With
# capture on, a stray EnableMouseCapture written anywhere else would leave the user's own
# terminal emitting escape sequences for every click into whatever shell it landed in —
# the same class of damage raw mode itself does, and confined the same way.
RAW_RE='enable_raw_mode|disable_raw_mode|EnterAlternateScreen|LeaveAlternateScreen|EnableMouseCapture|DisableMouseCapture'
# The six names RAW_RE searches for, as a list, because the positive control below is
# PER NAME. The one-of-any form this replaces (`grep -qE "$RAW_RE" src/ui/terminal.rs`) is
# satisfied by `enable_raw_mode` alone, so it would have covered the two capture commands
# vacuously in BOTH directions: a name dropped from RAW_RE while terminal.rs still uses it,
# and a name dropped from terminal.rs while RAW_RE still searches for it.
RAW_NAMES='enable_raw_mode disable_raw_mode EnterAlternateScreen LeaveAlternateScreen EnableMouseCapture DisableMouseCapture'

[ -f src/ui/terminal.rs ] || { echo "NORAW FAIL: src/ui/terminal.rs missing" >&2; exit 1; }

# Guard — positive control, per name. For EACH of the six the run fails unless the name
# matches RAW_RE *and* appears in src/ui/terminal.rs, so the exclusion cannot protect a name
# nobody searches for, and the search cannot cover a name nobody uses.
for name in $RAW_NAMES; do
  printf '%s\n' "$name" | grep -qE "$RAW_RE" \
    || { echo "NORAW FAIL: positive control - $name is not matched by RAW_RE" >&2; exit 1; }
  grep -qF "$name" src/ui/terminal.rs \
    || { echo "NORAW FAIL: positive control - src/ui/terminal.rs does not name $name" >&2
         exit 1; }
done

# Guard — the searched set is real. src + tests together are at least 29 files today.
n=$(find src tests -name '*.rs' ! -path 'src/ui/terminal.rs' | wc -l | tr -d ' ')
[ "$n" -ge 29 ] || { echo "NORAW FAIL: searched only $n files (expected >= 29)" >&2; exit 1; }

hits=$(find src tests -name '*.rs' ! -path 'src/ui/terminal.rs' -print0 \
       | xargs -0 -I{} grep -nE "$RAW_RE" {} /dev/null 2>&1 || true)
[ -z "$hits" ] || { echo "NORAW FAIL: terminal-mode function outside src/ui/terminal.rs:" >&2
                    echo "$hits" >&2; exit 1; }

# Second leg — CrosstermOps is named only where it is defined and where ui::run wires it.
# Any other site, especially under tests/, would mean a test can reach a real terminal.
sites=$(find src tests -name '*.rs' -print0 | xargs -0 -I{} grep -nE 'CrosstermOps' {} /dev/null 2>&1 || true)
bad=$(printf '%s\n' "$sites" | grep -v '^src/ui/terminal.rs:' | grep -v '^src/ui/mod.rs:' || true)
[ -z "$bad" ] || { echo "NORAW FAIL: CrosstermOps named outside terminal.rs and mod.rs:" >&2
                   echo "$bad" >&2; exit 1; }
c=$(printf '%s\n' "$sites" | grep -c 'CrosstermOps' || true)
[ "$c" -ge 2 ] || { echo "NORAW FAIL: CrosstermOps named $c times (expected >= 2: definition and wiring)" >&2
                    exit 1; }
echo "NORAW OK: $n files searched, mode functions only in src/ui/terminal.rs, CrosstermOps at $c sites"
