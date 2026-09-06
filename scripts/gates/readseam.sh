# READSEAM — NEW in detail-view. The artifact read has exactly ONE binding under src/ui/,
# on the same terms src/cli.rs is the crate's only process spawner: `read_to_string` is
# named under src/ui/ only in src/ui/mod.rs, so the read can be replaced, faked, or moved
# behind a worker thread by editing one file. NOIO-VIEW is the complementary half: it proves
# the seven pure files name no I/O API at all.
#
# Known limit, stated rather than discovered later: this is a grep over whole files,
# comments included. src/ui/detail.rs's doc comment must therefore say "the reader" rather
# than naming read_to_string, exactly as src/ui/markdown.rs's must say "the view" rather
# than naming ratatui. That is a deliberate cost: an exemption for comments is how a
# confinement check rots into a rubber stamp.
UIDIR="${UIDIR:-src/ui}"
BINDING="${BINDING:-src/ui/mod.rs}"
UI_MIN="${UI_MIN:-10}"
fail() { echo "READSEAM FAIL: $1" >&2; exit 1; }

[ -d "$UIDIR" ] || fail "no such directory: $UIDIR"
[ -f "$BINDING" ] || fail "$BINDING missing - the exclusion has nothing to exclude"

# Guard B — positive control, checked BEFORE the count so a gutted binding is reported as
# vacuous rather than as a file-count shortfall. The allowed file must actually perform the
# read, or the exclusion protects nothing and a clean result means nothing.
grep -qE 'read_to_string' "$BINDING" \
  || fail "$BINDING names no read_to_string - exclusion is vacuous"

# Guard C — the searched set is real. Excluded BY PATH, never by base name, so a future
# src/ui/detail/mod.rs is still searched.
n=$(find "$UIDIR" -name '*.rs' ! -path "$BINDING" | wc -l | tr -d ' ')
[ "$n" -ge "$UI_MIN" ] || fail "searched only $n files under $UIDIR (expected >= $UI_MIN)"

hits=$(find "$UIDIR" -name '*.rs' ! -path "$BINDING" -print0 \
       | xargs -0 -I{} grep -nE 'read_to_string' {} /dev/null 2>&1 || true)
[ -z "$hits" ] || { echo "READSEAM FAIL: read_to_string outside $BINDING:" >&2
                    echo "$hits" >&2; exit 1; }

# Second leg — the binding is wired exactly once in production. `read_artifact` is named in
# src/ui/mod.rs (its definition and ui::run's use of it) and may be named in a test module,
# but it must NOT be named in any other src/ui/ file: run_loop and sync_detail take a
# `&dyn Fn`, never this function by name, or the injection would be decorative.
bad=$(find "$UIDIR" -name '*.rs' ! -path "$BINDING" -print0 \
      | xargs -0 -I{} grep -nE 'read_artifact' {} /dev/null 2>&1 || true)
[ -z "$bad" ] || { echo "READSEAM FAIL: read_artifact named outside $BINDING:" >&2
                   echo "$bad" >&2; exit 1; }
echo "READSEAM OK: $n files searched under $UIDIR (>= $UI_MIN), read_to_string and read_artifact only in $BINDING"
