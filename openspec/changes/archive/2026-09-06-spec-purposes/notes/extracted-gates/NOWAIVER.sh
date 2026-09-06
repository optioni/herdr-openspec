# NOWAIVER — the coverage floor was not lowered, excluded, or annotated away.
# Carried forward from list-view BYTE-IDENTICALLY.
grep -q -- '--fail-under-lines 80' Makefile \
  || { echo "NOWAIVER FAIL: Makefile no longer names --fail-under-lines 80" >&2; exit 1; }
bad=$(grep -rnE 'coverage\(off\)|--ignore-filename-regex|--exclude|fail-under-lines ([0-7][0-9]?|[0-9])\b' \
      Makefile .github/workflows src tests 2>/dev/null || true)
[ -z "$bad" ] || { echo "NOWAIVER FAIL: a coverage waiver entered the tree:" >&2
                   echo "$bad" >&2; exit 1; }
echo "NOWAIVER OK: floor is 80, no exclusion, no coverage attribute"
