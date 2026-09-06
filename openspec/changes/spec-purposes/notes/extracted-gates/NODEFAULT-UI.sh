# NODEFAULT-UI — Dashboard, Filter, and Detail have no Default, and no construction or
# destructuring elides a field. EDITED here, and this is the ONE landed check detail-view
# makes STRONGER rather than merely re-running. Both halves, the TYPES list, and every
# positive control are kept; what changes is HOW half B looks.
#
# WHY it changes: half B was a same-line grep — `$T[[:space:]]*\{[^}]*\.\.` — and this
# repository has already shipped a version of it that missed FIFTEEN `..base` struct-update
# elisions, because rustfmt put the `..` on a later line than the opening brace. `Detail`
# growing from two fields to five is exactly the shape rustfmt spreads over seven lines, so
# the blind spot would be wider after this change than before it. The old note said "a
# multi-line elision is caught by the compile-time companions in app.rs's tests instead" —
# that claim is FALSE and is retired here: the companion destructures ONE value, so it
# catches a field added to the type, never an elision at some other site. Half B is now a
# brace-matching pass that sees past a line break, and the companion is kept for what it
# genuinely does.
#
# change-model's GATE-MECH1 covers Change/ChangeSet/ArtifactRef/Origin in src/changes.rs
# ONLY, so a state type in src/ui/app.rs is outside it.
SRC="${SRC:-src}"
HOMEFILE="${HOMEFILE:-$SRC/ui/app.rs}"
# `${TYPES-...}` and NOT `${TYPES:-...}`: with the colon, an explicitly empty TYPES silently
# falls back to the default and the emptiness guard below can never fire. The guard also
# rejects a whitespace-only value, which `[ -n ]` alone would accept and which would make
# the loop body run zero times while still printing OK.
TYPES="${TYPES-Dashboard Filter Detail}"
# The number of literal/pattern spans half B must FIND before a clean result means anything.
# Measured in task 0.1; a broken regex that matches nothing would otherwise print OK.
# Measured at planning time on unmodified `main`: 68 spans, 0 elisions. The default is 60,
# comfortably below that and comfortably above zero; the invocation raises it if a later
# change wants it tighter.
SCAN_MIN="${SCAN_MIN:-60}"
[ -d "$SRC" ] || { echo "NODEFAULT-UI FAIL: no such directory: $SRC" >&2; exit 1; }
[ -f "$HOMEFILE" ] || { echo "NODEFAULT-UI FAIL: $HOMEFILE missing" >&2; exit 1; }
case "$TYPES" in *[![:space:]]*) ;;
  *) echo "NODEFAULT-UI FAIL: TYPES is empty - the loop would run zero times" >&2; exit 1;; esac

for T in $TYPES; do
  # Positive control — the file must actually declare the type, or every search below is
  # searching for a name that is not there and a clean result means nothing. ANCHORED on
  # both sides: an unanchored `struct $T` is satisfied by `struct FilterState` after a
  # rename, so the control would pass while every leg below searched for a name that is
  # no longer there.
  grep -qE "struct[[:space:]]+$T[[:space:]]*\{" "$HOMEFILE" \
    || { echo "NODEFAULT-UI FAIL: positive control - $HOMEFILE has no 'struct $T {'" >&2
         exit 1; }

  # Half A — no Default impl anywhere under src/, hand-written or derived. The impl form
  # now also matches a PATH-QUALIFIED target (`impl Default for crate::ui::app::Detail`),
  # which the previous `for[[:space:]]+$T` form silently missed. The derive form is caught
  # by taking the two lines preceding each `struct <T>` and looking for Default inside a
  # #[derive(...)] there.
  a=$(find "$SRC" -name '*.rs' -print0 \
      | xargs -0 -I{} grep -nE "impl[[:space:]]+Default[[:space:]]+for[[:space:]]+([A-Za-z0-9_]+::)*$T([[:space:]]|\{|$)" {} /dev/null 2>&1 || true)
  b=$(find "$SRC" -name '*.rs' -print0 \
      | xargs -0 -I{} grep -B2 -nE "struct[[:space:]]+$T[[:space:]]*\{" {} /dev/null 2>&1 \
      | grep 'derive' | grep 'Default' || true)
  [ -z "$a$b" ] || { echo "NODEFAULT-UI FAIL: $T has a Default:" >&2
                     printf '%s\n%s\n' "$a" "$b" >&2; exit 1; }
done

# Half B — no `..` inside a <T> literal or pattern, SINGLE-LINE OR MULTI-LINE. Brace-matched
# from the opening `{` to its partner, so rustfmt's own seven-line spread of a five-field
# literal is seen. Line comments are stripped first, so a `..` inside prose is not reported;
# a `..` in a nested literal inside a <T> literal IS reported, which errs strict on purpose.
# A range expression (`0..10`, `a..=b`) is excluded by requiring the `..` to follow a brace,
# a comma, or whitespace.
SRC="$SRC" TYPES="$TYPES" SCAN_MIN="$SCAN_MIN" python3 - <<'PY' || exit 1
import os, re, sys, pathlib

src = pathlib.Path(os.environ["SRC"])
types = os.environ["TYPES"].split()
scan_min = int(os.environ["SCAN_MIN"])
# Preceded by one of these keywords, a `<T> {` is a declaration or a signature, not a
# literal: `struct T {`, `impl T {`, `-> T {`, `enum`, `union`, `trait`, `impl Default for T {`.
SKIP_BEFORE = re.compile(r"(struct|enum|union|trait|impl|for|->)\s*$")
ELISION = re.compile(r"(?:^|[\s,{])\.\.(?!\.)")
LINE_COMMENT = re.compile(r"//.*$")

def strip_comments(text):
    return "\n".join(LINE_COMMENT.sub("", line) for line in text.splitlines())

scanned = 0
bad = []
files = sorted(src.rglob("*.rs"))
if not files:
    print(f"NODEFAULT-UI FAIL: no *.rs files under {src}", file=sys.stderr); sys.exit(1)
for path in files:
    text = strip_comments(path.read_text())
    for T in types:
        for m in re.finditer(r"(?<![A-Za-z0-9_])" + re.escape(T) + r"\s*\{", text):
            if SKIP_BEFORE.search(text[:m.start()]):
                continue
            scanned += 1
            depth, i, n = 0, m.end() - 1, len(text)
            while i < n:
                if text[i] == "{":
                    depth += 1
                elif text[i] == "}":
                    depth -= 1
                    if depth == 0:
                        break
                i += 1
            span = text[m.end():i]
            if ELISION.search(span):
                line = text[:m.start()].count("\n") + 1
                bad.append(f"{path}:{line}: {T} literal or pattern elides a field")
if scanned < scan_min:
    print(f"NODEFAULT-UI FAIL: half B found only {scanned} literal/pattern spans "
          f"(expected >= {scan_min}) - the scan is vacuous", file=sys.stderr)
    sys.exit(1)
if bad:
    print("NODEFAULT-UI FAIL: multi-line-aware half B:", file=sys.stderr)
    print("\n".join(bad), file=sys.stderr)
    sys.exit(1)
print(f"NODEFAULT-UI OK (half B): {scanned} literal/pattern spans scanned (>= {scan_min}), none elides a field")
PY
echo "NODEFAULT-UI OK: no Default for [$TYPES], no elided field; positive controls matched"
