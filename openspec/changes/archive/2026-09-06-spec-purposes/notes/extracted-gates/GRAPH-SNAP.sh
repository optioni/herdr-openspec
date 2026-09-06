# GRAPH-SNAP — the resolved normal build graph equals the committed per-triple snapshot.
# Regenerate with: GRAPH_WRITE=1 sh GRAPH-SNAP.sh
#
# Carried forward from tui-shell with ONE deliberate edit, APPLIED HERE AND WRITTEN OUT: the
# named-absence list grows from `encoding_rs time` to `encoding_rs time getopts
# pulldown-cmark-escape`, so pulldown-cmark's own default features are proven off in GRAPH
# terms as well as in manifest terms (DEPS legs 2b and 2d) and source terms (MDSEAM leg 3).
# The snapshot file itself is regenerated in task 3.3 and its diff reviewed in task 3.4.
#
# NOTE on the named absences' negative control: appending a stray line to the SNAPSHOT only
# trips `diff -u`, because the absence loop reads the REGENERATED graph, never $SNAP. The
# control that exercises the loop is task 10.2 item 17: turn on the `html` feature in a copy,
# regenerate the snapshot there (GRAPH_WRITE=1) so the diff passes, then run the check.
SNAP=tests/fixtures/build-graph.txt
TRIPLES="aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu"
TMP=$(mktemp -d) || exit 1
trap 'rm -rf "$TMP"' EXIT

for t in $TRIPLES; do
  # Strip cargo's "(*)" repeat marker and "(proc-macro)" tag, drop this crate's own line,
  # and sort. `--target <triple>` once per triple, never `--target all`, which reports
  # optional resolutions cargo never builds.
  cargo tree -e normal --target "$t" --prefix none 2>/dev/null \
    | sed -e 's/ (\*)$//' -e 's/ (proc-macro)$//' -e '/^$/d' \
    | grep -v '^herdr-openspec ' | sort -u > "$TMP/$t"
  lines=$(wc -l < "$TMP/$t" | tr -d ' ')
  [ "$lines" -ge 40 ] || { echo "GRAPH-SNAP FAIL: $t resolved only $lines packages" >&2; exit 1; }
  { echo "## $t"; cat "$TMP/$t"; } >> "$TMP/all"
done

if [ -n "${GRAPH_WRITE:-}" ]; then
  mkdir -p "$(dirname "$SNAP")"; cp "$TMP/all" "$SNAP"; echo "GRAPH-SNAP WROTE $SNAP"; exit 0
fi

[ -f "$SNAP" ] || { echo "GRAPH-SNAP FAIL: $SNAP missing" >&2; exit 1; }
diff -u "$SNAP" "$TMP/all" || { echo "GRAPH-SNAP FAIL: graph differs from the snapshot" >&2; exit 1; }

# The macOS pair and the Linux pair each agree; macOS and Linux differ by exactly
# linux-raw-sys. Asserted as a NAMED difference, because the two are no longer identical.
diff -q "$TMP/aarch64-apple-darwin" "$TMP/x86_64-apple-darwin" >/dev/null \
  || { echo "GRAPH-SNAP FAIL: the two macOS triples disagree" >&2; exit 1; }
diff -q "$TMP/aarch64-unknown-linux-gnu" "$TMP/x86_64-unknown-linux-gnu" >/dev/null \
  || { echo "GRAPH-SNAP FAIL: the two Linux triples disagree" >&2; exit 1; }
d=$(diff "$TMP/aarch64-apple-darwin" "$TMP/aarch64-unknown-linux-gnu" \
    | grep -E '^[<>]' | sed 's/^[<>] //' | cut -d' ' -f1 | sort -u | tr '\n' ' ')
[ "$d" = "linux-raw-sys " ] \
  || { echo "GRAPH-SNAP FAIL: macOS/Linux differ by [$d], expected [linux-raw-sys ]" >&2; exit 1; }

# Proc-macro allowlist, as a SET EQUALITY: a ninth arriving fails, and an eighth vanishing
# fails too, so the list cannot rot into a description of whatever happens to be there.
# pulldown-cmark adds none, which is part of why it was chosen.
allow="darling_macro derive_more-impl document-features indoc instability rustversion strum_macros thiserror-impl"
got=$(cargo tree -e normal --prefix none 2>/dev/null | grep '(proc-macro)' \
      | sed -e 's/ (\*)//' -e 's/ (proc-macro)//' | cut -d' ' -f1 | sort -u | tr '\n' ' ')
want=$(printf '%s\n' $allow | sort -u | tr '\n' ' ')
[ "$got" = "$want" ] || { echo "GRAPH-SNAP FAIL: proc-macro set is [$got], expected [$want]" >&2; exit 1; }

# Four named absences, each proving a default-feature decision is in effect rather than
# merely written down. encoding_rs would arrive with yaml-rust2's defaults, time with
# ratatui's all-widgets, and getopts and pulldown-cmark-escape with pulldown-cmark's own
# `getopts` and `html` defaults.
for absent in encoding_rs time getopts pulldown-cmark-escape; do
  if grep -qE "^$absent " "$TMP/all"; then
    echo "GRAPH-SNAP FAIL: $absent is in the normal build graph" >&2; exit 1
  fi
done
echo "GRAPH-SNAP OK: four triples match the snapshot; proc-macro set exact; four named absences hold"
