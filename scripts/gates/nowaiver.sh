# NOWAIVER — the coverage floor was not lowered, excluded, or annotated away.
#
# gate-integrity widened the scan set to scripts/ (a checker placed there — namely
# scripts/coverage-prod.py — must not be able to reintroduce a coverage-narrowing
# flag through a directory this gate did not previously read) and added the
# production-coverage floor to the must-name set below (deleting the second
# `make coverage` command is now caught the same way lowering the first one is).
#
# Widening the scan naively turns this red at HEAD, on three legitimate lines: this
# script's own pattern literal below (it *is* the pattern) and
# openspec-untouched.sh's two `git ls-files` calls passing git's own exclude-standard
# flag, which is a git flag, not a coverage one. Both are resolved by matching a
# coverage flag in a coverage CONTEXT rather than a bare substring — each forbidden
# spelling must be preceded by a space or line-start (nowaiver.sh's own alternatives
# are each preceded by a quote or a pipe, never a space, because they live inside a
# quoted regex, not as a real command-line argument) and the exclude spelling must
# additionally be followed by a space or line-end, so a longer flag sharing that
# prefix (exclude-standard) cannot satisfy it.
# See openspec/changes/gate-integrity/design.md -> quality-gates spec, "The coverage
# floor is 80% of lines, enforced and never waived". Do NOT resolve this by exempting
# either file by path: a path exemption inside the gate that guards against
# exemptions is exactly the vacuity this change removes.
#
# gate-integrity Change Review (task 9.3) — CORRECTED: the single shared "(^|[[:space:]])"
# prefix above never fit two of these four alternatives, so they were unreachable. Real
# usage never precedes `fail-under-lines` with whitespace: it always follows the SECOND
# dash of a real double-dash lowering flag, so the character right before it is `-`, not a
# space or line start — a real lowered floor matched nothing. Real usage never precedes a
# coverage-off attribute with whitespace either: the real attribute (written with a leading
# `#[` and a matching `]`) is preceded by a bracket, not a space. Both existing NOWAIVER
# controls happen to plant the ignore-filename-regex spelling — the one alternative the
# shared prefix DOES fit — so this went unnoticed until measured directly. Each alternative
# below now carries its own, independently-verified context instead of one shared prefix
# that fit neither of these two:
#   - `fail-under-lines` is anchored on a leading `-` or line start, matching the real
#     flag's shape (a dash immediately before the token) without also matching this file's
#     own pattern LITERAL below, where the token is instead preceded by `|` (alternation),
#     never a bare dash immediately followed by the token.
#   - the coverage-off attribute carries NO prefix requirement at all — a bare
#     `coverage\(off\)`. This cannot self-match this script's own source line, because the
#     source spells the alternative with the parentheses escaped, backslashes included: the
#     unescaped word run never appears contiguously in this file.
# `--ignore-filename-regex` and `--exclude` keep their original `(^|[[:space:]])` anchor —
# no defect was found in either.
grep -q -- '--fail-under-lines 80' Makefile \
  || { echo "NOWAIVER FAIL: Makefile no longer names --fail-under-lines 80" >&2; exit 1; }
grep -q -- 'scripts/coverage-prod.py' Makefile \
  || { echo "NOWAIVER FAIL: Makefile no longer names the production-coverage floor (scripts/coverage-prod.py)" >&2; exit 1; }
# The map path is the checker's second positional argument, and it is optional: omitted, the
# covers-range check (the whole point of naming a map at all) is skipped entirely. Dropping
# just this argument from the Makefile's coverage-prod.py line would silently disable that
# check while make gates, make coverage, and make check all stay green - the exact defect
# this gate exists to remove, one layer up. So the line that names the checker must also name
# its map argument, not merely the checker.
grep -- 'scripts/coverage-prod.py' Makefile | grep -q -- 'tests/degraded-coverage.toml' \
  || { echo "NOWAIVER FAIL: Makefile's production-coverage floor no longer names its map argument (tests/degraded-coverage.toml)" >&2; exit 1; }
# gate-integrity Change Review (task 9.3), second finding: the PRODUCTION floor's own
# NUMBER (scripts/coverage-prod.py's DEFAULT_PROD_MIN) was guarded by nothing above, and
# nothing forbade overriding it back down through the Makefile's own invocation either — a
# `PROD_MIN=0` prefix on the coverage-prod.py line would silence the production floor
# entirely while leaving every check above this line green. DEFAULT_PROD_MIN itself is
# guarded by tests/coverage_prod.rs (a Rust test, not this gate — this gate names only
# shell/text patterns); this leg closes the other half, the Makefile-level override.
grep -- 'scripts/coverage-prod.py' Makefile | grep -q -- 'PROD_MIN=' \
  && { echo "NOWAIVER FAIL: Makefile's production-coverage floor line carries a PROD_MIN= override" >&2; exit 1; }
bad=$(grep -rnE 'coverage\(off\)|(^|[[:space:]])--ignore-filename-regex|(^|[[:space:]])--exclude([[:space:]]|$)|(^|-)fail-under-lines ([0-7][0-9]?|[0-9])\b' \
      Makefile .github/workflows src tests scripts 2>/dev/null || true)
[ -z "$bad" ] || { echo "NOWAIVER FAIL: a coverage waiver entered the tree:" >&2
                   echo "$bad" >&2; exit 1; }
echo "NOWAIVER OK: floor is 80 with a production floor beside it, no exclusion, no coverage attribute, no PROD_MIN override (Makefile, .github/workflows, src, tests, scripts scanned)"
