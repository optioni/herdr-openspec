#!/usr/bin/env bash
# mouse-text-selection — measurement harness.
#
# The proposal stops at an investigation: before any spec is written we need to
# know what a real terminal does with each DEC mouse mode set, and whether the
# modifier bypass already covers the complaint.
#
# This script enables one mode set at a time, echoes every input byte the
# application receives, and asks you what the terminal did. Run it once per
# terminal you care about, and once inside a Herdr pane, since Herdr sits
# between this plugin and the terminal.
#
#     bash openspec/changes/mouse-text-selection/notes/probe.sh
#
# Answers are appended to notes/measurements.md next to this file.
#
# Deliberately not Rust and not part of the crate: adding a second binary would
# drag `tests/manifest.rs`'s manifest/README/binary-name triangle into a spike.

set -u

ESC=$(printf '\033')
CSI="${ESC}["
HERE=$(cd "$(dirname "$0")" && pwd)
RESULTS="${HERE}/measurements.md"

# The four mode sets worth separating. `full` is exactly what crossterm 0.29's
# EnableMouseCapture writes today (src/ui/terminal.rs -> CrosstermOps::enable_mouse).
SET_IDS="off minimal drag full"

set_modes() {           # $1 = set id -> echoes the mode numbers, space separated
  case "$1" in
    off)     echo "" ;;
    minimal) echo "1000 1006" ;;
    drag)    echo "1000 1002 1006" ;;
    full)    echo "1000 1002 1003 1015 1006" ;;
  esac
}

set_label() {
  case "$1" in
    off)     echo "no reporting at all (control: selection must work here)" ;;
    minimal) echo "press/release + SGR coords" ;;
    drag)    echo "press/release + button-motion + SGR coords" ;;
    full)    echo "the whole crossterm bundle — what the plugin does today" ;;
  esac
}

enable_modes() {
  local m
  for m in $(set_modes "$1"); do printf '%s?%sh' "$CSI" "$m"; done
}

disable_modes() {
  local m
  for m in $(set_modes "$1"); do printf '%s?%sl' "$CSI" "$m"; done
}

# "yes"/"no" for whether a variable is set and non-empty — `${V:+yes}${V:-no}`
# would print "yes" followed by the value itself whenever V is set.
flag() { [ -n "${1:-}" ] && echo yes || echo no; }

STTY_SAVED=$(stty -g 2>/dev/null || true)

cleanup() {
  local id
  # Nothing to restore, and nothing that should reach a pipe, when the EXIT
  # trap fires on the no-terminal path above.
  [ -t 1 ] || return 0
  for id in $SET_IDS; do disable_modes "$id"; done
  [ -n "$STTY_SAVED" ] && stty "$STTY_SAVED" 2>/dev/null
  printf '%s?25h' "$CSI"        # cursor back on
  printf '\n'
}
trap cleanup EXIT INT TERM

at()   { printf '%s%s;1H%sK' "$CSI" "$1" "$CSI"; }   # position at row $1, clear it
clr()  { printf '%s2J%sH' "$CSI" "$CSI"; }

# Read one input event: a single byte, or a whole CSI/SS3 escape sequence.
# Structural rather than timeout-based, because macOS's system bash is 3.2 and
# does not accept a fractional `read -t`.
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

LOG_TOP=14
LOG_ROWS=6

run_set() {
  local id="$1" ev vis i
  local -a log
  log=()

  clr
  printf '%s?25l' "$CSI"        # cursor off, so it does not sit inside the target text
  at 1;  printf 'SET "%s" — %s' "$id" "$(set_label "$id")"
  at 2;  printf 'modes on: %s' "$(set_modes "$id" | sed 's/^$/(none)/')"
  at 4;  printf 'Try to select the three lines below with the mouse:'
  at 5;  printf '    alpha bravo charlie delta echo foxtrot golf hotel'
  at 6;  printf '    india juliett kilo lima mike november oscar papa'
  at 7;  printf '    quebec romeo sierra tango uniform victor whiskey'
  at 9;  printf 'Do each of these, watching the event log below:'
  at 10; printf '  1. plain click-drag    2. Shift+drag    3. Option/Alt+drag'
  at 11; printf '  4. single click        5. scroll wheel'
  at 12; printf 'Then press  q  to finish this set.'
  at "$LOG_TOP"; printf 'events the APPLICATION received (empty = the terminal kept them):'

  enable_modes "$id"
  stty raw -echo 2>/dev/null

  while :; do
    ev=$(read_event) || break
    case "$ev" in
      q|$'\003') break ;;
    esac
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
  printf '%s?25h' "$CSI"
  clr
}

ask() {                 # $1 = prompt -> echoes y / n / ?
  local a
  while :; do
    printf '  %s [y/n/?] ' "$1" >&2
    read -r a
    case "$a" in
      y|Y) echo "yes"; return ;;
      n|N) echo "no";  return ;;
      ''|'?') echo "?"; return ;;
    esac
  done
}

main() {
  # Refuse without a real terminal on both ends, rather than letting every
  # `read` hit EOF and recording a table of "?" that looks like a measurement.
  # Exit 3 deliberately mirrors `ui`'s own refusal (SPEC.md -> "No terminal is
  # not a degraded state"): a harness whose whole subject is what a terminal
  # does has nothing to degrade to when there is no terminal.
  if [ ! -t 0 ] || [ ! -t 1 ]; then
    echo "probe.sh: not a terminal — run this directly in a terminal tab," >&2
    echo "  not through an agent, a pipe, or a non-interactive shell." >&2
    exit 3
  fi

  printf 'mouse-text-selection measurement harness\n\n'
  printf 'Terminal: %s   TERM=%s\n' "${TERM_PROGRAM:-unknown}" "${TERM:-unset}"
  printf 'Inside Herdr: %s   Inside tmux: %s\n\n' \
    "$(flag "${HERDR_WORKSPACE_ID:-}")" "$(flag "${TMUX:-}")"
  printf 'Press Enter to begin. Ctrl-C at any point restores the terminal.\n'
  read -r _

  {
    printf '\n## %s — %s, TERM=%s, herdr=%s, tmux=%s\n\n' \
      "$(date +%Y-%m-%d)" "${TERM_PROGRAM:-unknown}" "${TERM:-unset}" \
      "$(flag "${HERDR_WORKSPACE_ID:-}")" "$(flag "${TMUX:-}")"
    printf '| mode set | plain drag selects | Shift+drag selects | Option+drag selects | app saw click | app saw wheel |\n'
    printf '|---|---|---|---|---|---|\n'
  } >> "$RESULTS"

  local id plain shift opt click wheel
  for id in $SET_IDS; do
    run_set "$id"
    printf '\nSET "%s" — what did the TERMINAL do?\n' "$id"
    plain=$(ask "plain click-drag selected text natively?")
    shift=$(ask "Shift+drag selected text natively?")
    opt=$(ask   "Option/Alt+drag selected text natively?")
    click=$(ask "the app received a click event (shown in the log)?")
    wheel=$(ask "the app received wheel events?")
    printf '| `%s` | %s | %s | %s | %s | %s |\n' \
      "$id" "$plain" "$shift" "$opt" "$click" "$wheel" >> "$RESULTS"
    printf '\n'
  done

  printf '\nRecorded to %s\n' "$RESULTS"
}

main "$@"
