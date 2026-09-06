# GRAPH-SNAP — the resolved normal build graph equals the committed per-triple snapshot, and
# the macOS/Linux platform difference is named per direction rather than compared against a
# single literal.
#
# Extracted from a per-change command-level check into a repository file by spec-purposes.
# The snapshot itself was current (regenerated in 574b87d in an earlier change); the gate
# failed four legs later on a hardcoded platform literal that could not express the realized,
# asymmetric difference — macOS-only fsevent-sys against Linux-only inotify, inotify-sys, and
# linux-raw-sys. Repaired here with two direction-aware assertions instead of one merged set,
# per design.md -> Decisions -> 4: deriving the expected difference from the manifests is not
# available, because Cargo.toml declares no [target.*] table at all, so naming each direction
# is the honest option and the only discriminating one.
#
# Regenerate the snapshot with: sh scripts/gates/build-graph.sh write
#   (an AMBIENT GRAPH_WRITE is deliberately NOT read: a gate artefact regenerated because a
#   caller's shell happened to export a variable blesses the current state without review.
#   `make gates` additionally runs with GRAPH_WRITE cleared, belt and suspenders.)
set -u
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

if [ "${1:-}" = "write" ]; then
  mkdir -p "$(dirname "$SNAP")"; cp "$TMP/all" "$SNAP"; echo "GRAPH-SNAP WROTE $SNAP"; exit 0
fi

[ -f "$SNAP" ] || { echo "GRAPH-SNAP FAIL: $SNAP missing" >&2; exit 1; }
diff -u "$SNAP" "$TMP/all" || { echo "GRAPH-SNAP FAIL: graph differs from the snapshot" >&2; exit 1; }

# The macOS pair and the Linux pair each agree.
diff -q "$TMP/aarch64-apple-darwin" "$TMP/x86_64-apple-darwin" >/dev/null \
  || { echo "GRAPH-SNAP FAIL: the two macOS triples disagree" >&2; exit 1; }
diff -q "$TMP/aarch64-unknown-linux-gnu" "$TMP/x86_64-unknown-linux-gnu" >/dev/null \
  || { echo "GRAPH-SNAP FAIL: the two Linux triples disagree" >&2; exit 1; }

# Direction-aware platform difference: macOS-only and Linux-only are each asserted against
# their own named list, so a package migrating from one platform to the other — which an
# unordered merged-set compare cannot see — fails here by naming which side it moved to.
# Compared by PACKAGE NAME alone (comm on full "name vX.Y.Z" lines would misreport a shared
# package at two different versions as present on both sides), each list sorted-unique so
# comm's precondition holds.
cut -d' ' -f1 "$TMP/aarch64-apple-darwin" | sort -u > "$TMP/names-macos"
cut -d' ' -f1 "$TMP/aarch64-unknown-linux-gnu" | sort -u > "$TMP/names-linux"
macos_only=$(comm -23 "$TMP/names-macos" "$TMP/names-linux" | tr '\n' ' ')
linux_only=$(comm -13 "$TMP/names-macos" "$TMP/names-linux" | tr '\n' ' ')
expected_macos_only="fsevent-sys "
expected_linux_only="inotify inotify-sys linux-raw-sys "
[ "$macos_only" = "$expected_macos_only" ] \
  || { echo "GRAPH-SNAP FAIL: macOS-only packages are [$macos_only], expected [$expected_macos_only]" >&2; exit 1; }
[ "$linux_only" = "$expected_linux_only" ] \
  || { echo "GRAPH-SNAP FAIL: Linux-only packages are [$linux_only], expected [$expected_linux_only]" >&2; exit 1; }

# Proc-macro allowlist, as a SET EQUALITY: a ninth arriving fails, and an eighth vanishing
# fails too, so the list cannot rot into a description of whatever happens to be there.
# Measured from the HOST graph (cargo tree with no --target; cargo omits the "(proc-macro)"
# tag for cross-target resolutions), and confirmed platform-independent by task 8.12.
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
echo "GRAPH-SNAP OK: four triples match the snapshot; macOS-only [$macos_only] and Linux-only [$linux_only] exact; proc-macro set exact; four named absences hold"
