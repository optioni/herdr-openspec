# NORAW-GREP — crossterm's terminal-mode functions appear ONLY in src/ui/terminal.rs, and
# CrosstermOps is constructed only by ui::run. Searches tests/ as well as src/, because the
# failure this prevents is a TEST putting the developer's own terminal into raw mode.
# Extracted to scripts/gates/ by degraded-states: its file-count guard was hardcoded at
# 16 (list-view's own measurement); re-measured here at 29 and fixed to match, since a
# stale hardcoded floor is exactly this project's own recurring defect class.
RAW_RE='enable_raw_mode|disable_raw_mode|EnterAlternateScreen|LeaveAlternateScreen'

[ -f src/ui/terminal.rs ] || { echo "NORAW FAIL: src/ui/terminal.rs missing" >&2; exit 1; }

# Guard — positive control: the allowed file must actually name them, or the exclusion
# protects nothing.
grep -qE "$RAW_RE" src/ui/terminal.rs \
  || { echo "NORAW FAIL: src/ui/terminal.rs names no terminal-mode function - vacuous" >&2
       exit 1; }

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
