# MDSEAM — the markdown parser is confined to ONE module, and that module names no ratatui
# type and no pulldown_cmark::html path. Carried forward from markdown-viewer with its
# executable logic BYTE-IDENTICAL; only its MIN invocation moves, 17 -> 18.
#
# It is re-run rather than edited because src/ui/detail.rs calls `ui::markdown::lines` — the
# module's public, plain-data entry point — and names pulldown_cmark nowhere. If a future
# implementer reaches for the parser directly from ui::detail, this is the check that stops
# them.
MD="${MD:-src/ui/markdown.rs}"
MIN="${MIN:-16}"
fail() { echo "MDSEAM FAIL: $1" >&2; exit 1; }

[ -d src ] || fail "no src directory"
[ -f "$MD" ] || fail "$MD missing - the exclusion has nothing to exclude"

# Guard B — positive control: the allowed file must actually name the parser, or the
# exclusion protects nothing and a clean result means nothing. Checked BEFORE the count, so
# a gutted markdown.rs is reported as vacuous rather than as a file-count shortfall.
grep -qE 'pulldown_cmark' "$MD" || fail "$MD names no pulldown_cmark - exclusion is vacuous"

# Guard C — the searched set is real. Excluded BY PATH, never by base name, so a future
# src/markdown.rs is still searched.
n=$(find src -name '*.rs' ! -path "$MD" | wc -l | tr -d ' ')
[ "$n" -ge "$MIN" ] || fail "searched only $n files (expected >= $MIN)"

hits=$(find src -name '*.rs' ! -path "$MD" -print0 \
       | xargs -0 -I{} grep -nE 'pulldown_cmark|pulldown-cmark' {} /dev/null 2>&1 || true)
[ -z "$hits" ] || { echo "MDSEAM FAIL: the markdown parser is named outside $MD:" >&2
                    echo "$hits" >&2; exit 1; }

# Second leg — the confined module names no ratatui type. Judged on output emptiness too.
r=$(grep -nE 'ratatui|Modifier|Style|Span' "$MD" || true)
[ -z "$r" ] || { echo "MDSEAM FAIL: $MD names a ratatui type:" >&2; echo "$r" >&2; exit 1; }

# Third leg — the html feature is observably off in SOURCE terms: no file anywhere names a
# pulldown_cmark::html path. The graph half of the same claim is GRAPH-SNAP's named
# absences, and the manifest half is DEPS legs 2b and 2d.
h=$(find src -name '*.rs' -print0 | xargs -0 -I{} grep -nE 'pulldown_cmark::html|cmark::html' {} /dev/null 2>&1 || true)
[ -z "$h" ] || { echo "MDSEAM FAIL: a pulldown_cmark::html path is named:" >&2; echo "$h" >&2; exit 1; }
echo "MDSEAM OK: $n files searched (>= $MIN), pulldown_cmark only in $MD, no ratatui type there, no html path"
