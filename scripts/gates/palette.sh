# PALETTE — the colour table is confined to ONE module. src/ui/palette.rs is the only
# file in the crate that may name a ratatui Color, on exactly MDSEAM's terms: excluded
# BY PATH so a future src/palette.rs is still searched, with the positive control
# checked BEFORE the sweep so a gutted table is reported as vacuous rather than clean.
PAL="${PAL:-src/ui/palette.rs}"
MIN="${MIN:-25}"
fail() { echo "PALETTE FAIL: $1" >&2; exit 1; }

[ -d src ] || fail "no src directory"
[ -f "$PAL" ] || fail "$PAL missing - the exclusion has nothing to exclude"

# Guard B - positive control: the allowed file must actually name a Color.
grep -qE 'Color::' "$PAL" || fail "$PAL names no Color - exclusion is vacuous"

# Guard C - the searched set is real.
n=$(find src -name '*.rs' ! -path "$PAL" | wc -l | tr -d ' ')
[ "$n" -ge "$MIN" ] || fail "searched only $n files (expected >= $MIN)"

hits=$(find src -name '*.rs' ! -path "$PAL" -print0 \
       | xargs -0 -I{} grep -nE 'Color::|ratatui::style::Color' {} /dev/null 2>&1 || true)
[ -z "$hits" ] || { echo "PALETTE FAIL: a ratatui Color is named outside $PAL:" >&2
                    echo "$hits" >&2; exit 1; }

# Second leg - named ANSI indices only.
r=$(grep -nE 'Color::Rgb|Color::Indexed|Color::Reset' "$PAL" || true)
[ -z "$r" ] || { echo "PALETTE FAIL: $PAL names a non-ANSI colour:" >&2
                 echo "$r" >&2; exit 1; }

# Third leg - the two standing view gates actually sweep the new module. Both
# hard-code their PURE lists and check only that the files they name exist, so
# without this a forgotten edit leaves palette.rs unswept while both print OK.
for g in scripts/gates/noio-view.sh scripts/gates/colwidth.sh; do
  [ -f "$g" ] || fail "$g missing"
  grep -q "$PAL" "$g" || fail "$g does not list $PAL in its PURE set"
done
echo "PALETTE OK: $n files searched (>= $MIN), Color only in $PAL, named ANSI indices only, swept by NOIO-VIEW and COLWIDTH"
