#!/usr/bin/env bash
# mouse-text-selection — alternate-screen probe.
#
# probe.sh has a flaw: it ran on the PRIMARY screen. The dashboard runs on the
# ALTERNATE screen, and the difference decides this change.
#
# On the primary screen the wheel scrolls the terminal's own scrollback, so an
# app with no mouse reporting sees nothing — which is what probe.sh's `off` row
# recorded. On the alternate screen, most terminals instead translate the wheel
# into ARROW KEYS so that pagers and TUIs scroll. If that holds here, a TUI can
# have wheel scrolling AND native drag-selection by enabling no reporting at all.
#
#     bash openspec/changes/mouse-text-selection/notes/probe-altscreen.sh
#
# Appends to notes/measurements.md.

set -u

ESC=$(printf '\033')
CSI="${ESC}["
HERE=$(cd "$(dirname "$0")" && pwd)
RESULTS="${HERE}/measurements.md"

flag() { [ -n "${1:-}" ] && echo yes || echo no; }

# Override with PROBE_SETS to re-measure a subset, e.g.
#   PROBE_SETS="press-only press-sgr" bash notes/probe-altscreen.sh
SET_IDS="${PROBE_SETS:-alt-off alt-minimal alt-full}"

set_modes() {
  case "$1" in
    alt-off)     echo "" ;;
    alt-minimal) echo "1000 1006" ;;
    alt-full)    echo "1000 1002 1003 1015 1006" ;;
    press-only)  echo "1000" ;;
    press-sgr)   echo "1000 1006" ;;
    copilot-set) echo "1003 1006" ;;
    motion-only) echo "1003" ;;
  esac
}

set_label() {
  case "$1" in
    alt-off)     echo "ALTERNATE screen, no reporting — the hypothesis under test" ;;
    alt-minimal) echo "ALTERNATE screen, press/release + SGR" ;;
    alt-full)    echo "ALTERNATE screen, today's full bundle" ;;
    press-only)  echo "?1000 ONLY — no motion asked for. DOES PLAIN DRAG STILL SELECT?" ;;
    press-sgr)   echo "?1000 + ?1006 — same, with extended coords. PLAIN DRAG?" ;;
    copilot-set) echo "?1003 + ?1006 — EXACTLY what Copilot CLI sets. PLAIN DRAG?" ;;
    motion-only) echo "?1003 alone — any-event tracking, no ?1000. PLAIN DRAG?" ;;
  esac
}

enable_modes()  { local m; for m in $(set_modes "$1"); do printf '%s?%sh' "$CSI" "$m"; done; }
disable_modes() { local m; for m in $(set_modes "$1"); do printf '%s?%sl' "$CSI" "$m"; done; }

STTY_SAVED=$(stty -g 2>/dev/null || true)

cleanup() {
  local id
  [ -t 1 ] || return 0
  for id in $SET_IDS; do disable_modes "$id"; done
  printf '%s?1049l' "$CSI"      # leave the alternate screen
  [ -n "$STTY_SAVED" ] && stty "$STTY_SAVED" 2>/dev/null
  printf '%s?25h' "$CSI"
  printf '\n'
}
trap cleanup EXIT INT TERM

at()  { printf '%s%s;1H%sK' "$CSI" "$1" "$CSI"; }
clr() { printf '%s2J%sH' "$CSI" "$CSI"; }

read_event() {
  local ch seq
  IFS= read -rsn1 ch || return 1
  if [ "$ch" != "$ESC" ]; then printf '%s' "$ch"; return 0; fi
  seq="$ESC"
  IFS= read -rsn1 ch || { printf '%s' "$seq"; return 0; }
  seq="${seq}${ch}"
  if [ "$ch" != "[" ] && [ "$ch" != "O" ]; then printf '%s' "$seq"; return 0; fi
  while IFS= read -rsn1 ch; do
    seq="${seq}${ch}"
    case "$ch" in [A-Za-z~]) break ;; esac
  done
  printf '%s' "$seq"
}

LOG_TOP=15
LOG_ROWS=6

run_set() {
  local id="$1" ev vis i
  local -a log
  log=()

  printf '%s?1049h' "$CSI"       # ENTER the alternate screen — the whole point
  clr
  printf '%s?25l' "$CSI"
  at 1;  printf 'SET "%s" — %s' "$id" "$(set_label "$id")"
  at 2;  printf 'modes on: %s' "$(set_modes "$id" | sed 's/^$/(none)/')"
  at 4;  printf 'Try to select these three lines with a plain drag (no modifier):'
  at 5;  printf '    alpha bravo charlie delta echo foxtrot golf hotel'
  at 6;  printf '    india juliett kilo lima mike november oscar papa'
  at 7;  printf '    quebec romeo sierra tango uniform victor whiskey'
  at 9;  printf 'Then, watching the log below:'
  at 10; printf '  1. PLAIN DRAG across the words above — does the TERMINAL highlight them?'
  at 11; printf '  2. single click      3. scroll wheel      4. Shift+drag'
  at 13; printf 'Press  q  to finish this set.'
  at "$LOG_TOP"; printf 'events the APPLICATION received:'

  enable_modes "$id"
  stty raw -echo 2>/dev/null

  while :; do
    ev=$(read_event) || break
    case "$ev" in q|$'\003') break ;; esac
    vis=$(printf '%s' "$ev" | cat -v)
    log[${#log[@]}]="$vis"
    i=${#log[@]}
    [ "$i" -gt "$LOG_ROWS" ] && log=("${log[@]:$((i - LOG_ROWS))}")
    for i in $(seq 0 $((LOG_ROWS - 1))); do
      at $((LOG_TOP + 1 + i))
      [ "$i" -lt "${#log[@]}" ] && printf '    %s' "${log[$i]}"
    done
  done

  stty "$STTY_SAVED" 2>/dev/null
  disable_modes "$id"
  printf '%s?25h%s?1049l' "$CSI" "$CSI"   # leave the alternate screen
}

ask() {
  local a
  while :; do
    printf '  %s [y/n/?] ' "$1" >&2
    read -r a
    case "$a" in y|Y) echo yes; return ;; n|N) echo no; return ;; ''|'?') echo '?'; return ;; esac
  done
}

main() {
  if [ ! -t 0 ] || [ ! -t 1 ]; then
    echo "probe-altscreen.sh: not a terminal — run this directly in a terminal tab." >&2
    exit 3
  fi

  printf 'alternate-screen probe\n\n'
  printf 'Terminal: %s   TERM=%s   herdr=%s   tmux=%s\n\n' \
    "${TERM_PROGRAM:-unknown}" "${TERM:-unset}" \
    "$(flag "${HERDR_WORKSPACE_ID:-}")" "$(flag "${TMUX:-}")"
  printf 'The question: on the ALTERNATE screen with NO mouse reporting, does the\n'
  printf 'wheel arrive as arrow keys while plain drag-selection still works?\n\n'
  printf 'Press Enter to begin.\n'
  read -r _

  {
    printf '\n## %s — ALTERNATE SCREEN — %s, TERM=%s, herdr=%s, tmux=%s\n\n' \
      "$(date +%Y-%m-%d)" "${TERM_PROGRAM:-unknown}" "${TERM:-unset}" \
      "$(flag "${HERDR_WORKSPACE_ID:-}")" "$(flag "${TMUX:-}")"
    printf '| mode set | plain drag selects | wheel arrives as ARROW KEYS | app saw click | Shift+drag selects |\n'
    printf '|---|---|---|---|---|\n'
  } >> "$RESULTS"

  local id plain arrows click shift
  for id in $SET_IDS; do
    run_set "$id"
    printf '\nSET "%s":\n' "$id"
    plain=$(ask  "plain click-drag selected text natively?")
    arrows=$(ask "the wheel produced arrow keys (^[[A / ^[[B) in the log?")
    click=$(ask  "the app received a click event?")
    shift=$(ask  "Shift+drag selected text natively?")
    printf '| `%s` | %s | %s | %s | %s |\n' "$id" "$plain" "$arrows" "$click" "$shift" >> "$RESULTS"
    printf '\n'
  done

  printf '\nRecorded to %s\n' "$RESULTS"
}

main "$@"
