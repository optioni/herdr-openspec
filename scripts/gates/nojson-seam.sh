# NOJSON-SEAM — the seam returns stdout verbatim; every parse lives on the other side.
fail() { echo "NOJSON-SEAM FAIL: $1" >&2; exit 1; }
[ -f src/cli.rs ] || fail "src/cli.rs missing - nothing to check"
[ -f src/changes.rs ] || fail "src/changes.rs missing"
# Positive control: the crate must actually use serde_json, or its absence from cli.rs
# proves nothing at all.
grep -q 'serde_json' src/changes.rs \
  || fail "src/changes.rs does not name serde_json - the check would pass vacuously"
hits=$(grep -n 'serde_json' src/cli.rs || true)
[ -z "$hits" ] || { echo "NOJSON-SEAM FAIL: serde_json in src/cli.rs:" >&2
                    echo "$hits" >&2; exit 1; }
echo "NOJSON-SEAM OK: serde_json used in src/changes.rs, absent from src/cli.rs"
