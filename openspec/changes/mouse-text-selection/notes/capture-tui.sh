#!/usr/bin/env bash
# Capture a REAL interactive TUI session and report every mouse-mode change in
# the order it happened.
#
# The earlier one-shot capture ran a TUI with stdin at /dev/null: it started,
# painted, and exited, so it could only ever show the start-up set. A program
# that turns reporting OFF again while idle — which would let the terminal
# select text — looks identical at start-up and completely different in use.
#
#     bash notes/capture-tui.sh copilot
#
# Use the program normally. Select some text with a plain drag. Then quit it.
# The mode timeline is printed and appended to measurements.md.

set -u
[ $# -ge 1 ] || { echo "usage: $0 <program> [args...]" >&2; exit 2; }
[ -t 0 ] || { echo "capture-tui.sh: needs a real terminal." >&2; exit 3; }

HERE=$(cd "$(dirname "$0")" && pwd)
LOG=$(mktemp /tmp/capture-tui.XXXXXX)
PROG="$1"; shift

printf 'Capturing %s. Use it normally, TRY A PLAIN DRAG-SELECT, then quit it.\n\n' "$PROG"
script -q "$LOG" "$PROG" "$@"

printf '\n\n=== mouse-mode timeline (in order) ===\n'
# Byte offset of each mode change, so the ORDER and any mid-session change show.
LC_ALL=C grep -abo $'\033\[?\(1000\|1002\|1003\|1006\|1015\|1049\)[hl]' "$LOG" \
  | sed 's/\x1b/ESC/' \
  | awk -F: '{printf "  @%-9s %s\n", $1, $2}'

printf '\n=== totals ===\n'
LC_ALL=C grep -ao $'\033\[?[0-9]*[hl]' "$LOG" | sed 's/\x1b/ESC/' | sort | uniq -c | sort -rn

printf '\nA mid-session `?1003l` or `?1006l` followed by a later `h` means the program\n'
printf 'releases reporting while idle — which is how it keeps native selection.\n'
printf '\nRaw log: %s\n' "$LOG"

{
  printf '\n### Interactive capture of `%s` — %s\n\n```\n' "$PROG" "$(date +%Y-%m-%d)"
  LC_ALL=C grep -abo $'\033\[?\(1000\|1002\|1003\|1006\|1015\|1049\)[hl]' "$LOG" \
    | sed 's/\x1b/ESC/' | awk -F: '{printf "@%-9s %s\n", $1, $2}'
  printf '```\n'
} >> "$HERE/measurements.md"
printf 'Timeline appended to %s\n' "$HERE/measurements.md"
