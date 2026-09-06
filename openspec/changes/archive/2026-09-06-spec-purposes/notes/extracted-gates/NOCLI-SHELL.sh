# NOCLI-SHELL — the dashboard shell never names the CLI seam. This is the mechanical form
# of the roadmap's "detail-view depends on list-view and markdown-viewer, and through them on
# changes-from-files, not on the CLI".
# Carried forward from markdown-viewer with its executable logic BYTE-IDENTICAL; only
# UI_MIN's invocation moves, 9 -> 10.
CLI_RE='from_cli|OpenspecCli|HerdrCli|CliChanges|npm_prefix'
UI_MIN="${UI_MIN:-7}"

# Guard A — src/ui/ exists and holds Rust files. A count of zero would make the search
# vacuous. Parameterised for the same reason NOSPAWN-GREP's MIN is: a later change that
# merges two files would turn a real check into a spurious failure, which is how a check
# gets weakened into a rubber stamp.
[ -d src/ui ] || { echo "NOCLI-SHELL FAIL: src/ui missing" >&2; exit 1; }
n=$(find src/ui -name '*.rs' | wc -l | tr -d ' ')
[ "$n" -ge "$UI_MIN" ] || { echo "NOCLI-SHELL FAIL: only $n files under src/ui (expected >= $UI_MIN)" >&2
                    exit 1; }

# Guard B — positive control. src/changes.rs MUST name OpenspecCli, or the pattern is
# broken and a clean result means nothing.
grep -qE 'OpenspecCli' src/changes.rs \
  || { echo "NOCLI-SHELL FAIL: positive control - src/changes.rs does not name OpenspecCli" >&2
       exit 1; }

hits=$(find src/ui -name '*.rs' -print0 | xargs -0 -I{} grep -nE "$CLI_RE" {} /dev/null 2>&1 || true)
[ -z "$hits" ] || { echo "NOCLI-SHELL FAIL: the shell names the CLI seam:" >&2
                    echo "$hits" >&2; exit 1; }
echo "NOCLI-SHELL OK: $n files under src/ui name no CLI seam; positive control matched"
