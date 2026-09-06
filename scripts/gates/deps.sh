# DEPS — the argued dependency set, the one binary target, the MSRV floor, and the
# genuinely-needed experiment. Reads `cargo metadata` JSON through python3 rather than
# adding a crate to do it.
#
# Extracted from a per-change command-level check into a repository file by spec-purposes:
# a gate that lives only as prose inside archived tasks.md documents is not forced to run,
# and DEPS was red on `main` for three changes (since `live-refresh` added `notify`)
# before anyone noticed. Two repairs applied here, WRITTEN OUT rather than described:
# (1) leg 2a's want dict gains "notify": ["macos_fsevent"], and "exactly five" becomes
# "exactly six"; (2) leg 5 gains a sixth removal experiment for notify, arguing it on the
# same terms as the other five.
#
#   env: WORK        scratch directory; defaulted to a fresh mktemp -d and cleaned up on
#                     exit if the caller does not set one. Leg 5 copies the crate into it,
#                     because `cargo build` REWRITES Cargo.lock during resolution before it
#                     reaches the compile error the leg waits for, so an edit-and-restore in
#                     place would silently discard the resolution this change verified.
#        DEPS_FULL=1  runs leg 1b (builds the release binary) and leg 5 (six removal
#                     experiments, each rebuilding the crate). Unset — the `make gates` case
#                     — skips both: they are what let this gate die outside `make check` in
#                     the first place, so they get their own `make gates-full` and CI job
#                     instead of costing every local `make check` several rebuilds.
set -u
if [ -z "${WORK:-}" ]; then
  WORK=$(mktemp -d) || { echo "DEPS FAIL: mktemp -d failed" >&2; exit 1; }
  trap 'rm -rf "$WORK"' EXIT
fi
[ -d "$WORK" ] || { echo "DEPS FAIL: WORK=$WORK is not a directory" >&2; exit 1; }
[ -f Cargo.toml ] && [ -d src ] || { echo "DEPS FAIL: run from the crate root" >&2; exit 1; }
TRIPLES='aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu'

# --- leg 1a: exactly one bin target -------------------------------------------
cargo metadata --no-deps --format-version 1 | python3 -c '
import json,sys
m=json.load(sys.stdin); p=m["packages"][0]
bins=[t for t in p["targets"] if "bin" in t["kind"]]
assert p["name"]=="herdr-openspec", p["name"]
assert len(bins)==1 and bins[0]["name"]=="herdr-openspec", bins
assert p["edition"]=="2024", p["edition"]
print("DEPS OK (leg 1a): exactly one bin target, herdr-openspec, edition 2024")
' || { echo "DEPS FAIL: leg 1a" >&2; exit 1; }

# --- leg 1b: build.sh actually produces the binary — DEPS_FULL only ----------
# Rebuilds the release binary; kept out of `make gates` so a stale want-list still fails
# in two seconds rather than after a full release build.
if [ "${DEPS_FULL:-0}" = "1" ]; then
  # The deletion and the EXIT STATUS are both load-bearing: build.sh has a real failure
  # path (`error: cargo not found`, exit 1) and a leftover binary from any earlier build
  # satisfies an existence check regardless of what the script did.
  rm -f target/release/herdr-openspec
  /bin/sh scripts/build.sh >/dev/null 2>&1 \
    || { echo "DEPS FAIL: leg 1b scripts/build.sh exited non-zero" >&2; exit 1; }
  [ -x target/release/herdr-openspec ] \
    || { echo "DEPS FAIL: leg 1b no executable at target/release/herdr-openspec" >&2; exit 1; }
  echo "DEPS OK (leg 1b): scripts/build.sh exited 0 and produced an executable binary"
fi

# --- leg 2: the declared set, read from RESOLVED metadata, not manifest text --
# Re-derived from Cargo.toml, reconciled against plugin-build's "The crate produces one
# binary from an argued dependency set" requirement — rebuilt whole rather than patched, so
# a second stale entry cannot survive a partial repair.
cargo metadata --no-deps --format-version 1 | python3 -c '
import json,sys
want={"serde_json":["std"],
      "toml":["display","parse","serde","std"],
      "yaml-rust2":[],
      "ratatui":["crossterm"],
      "pulldown-cmark":[],
      "notify":["macos_fsevent"]}
deps=[d for d in json.load(sys.stdin)["packages"][0]["dependencies"] if d["kind"] is None]
got={d["name"]:d for d in deps}
assert sorted(got)==sorted(want), f"normal deps are {sorted(got)}, expected {sorted(want)}"
assert "crossterm" not in got, "crossterm is declared directly - it must be reached through ratatui only"
for n,f in want.items():
    assert got[n]["uses_default_features"] is False, f"{n} uses default features"
    assert sorted(got[n]["features"])==sorted(f), "%s features %s != %s" % (n, got[n]["features"], f)
print("DEPS OK (leg 2a): exactly", len(deps), "normal deps, defaults off, features exact")
' || { echo "DEPS FAIL: leg 2a" >&2; exit 1; }
# `cargo metadata` reports [] for both `features = []` and an omitted key and cannot
# tell them apart; the requirement is about what the manifest says on its face, so
# this clause is read from the TEXT. Both empty-feature crates are checked, in a loop,
# so adding a third is one word rather than a copied line.
for c in yaml-rust2 pulldown-cmark; do
  grep -q "$c = .*features = \[\]" Cargo.toml \
    || { echo "DEPS FAIL: leg 2b $c does not spell out features = [] in Cargo.toml" >&2; exit 1; }
done
echo "DEPS OK (leg 2b): yaml-rust2 and pulldown-cmark spell out features = [] in Cargo.toml"
# leg 2a-bis: crossterm is ABSENT from the declared set (asserted above) and PRESENT in the
# resolved graph. Both halves are needed: absent-and-absent would mean the backend is gone,
# and declared-and-present is the two-crossterm-versions hazard the design forbids.
cargo tree -e normal 2>/dev/null | grep -qE '(^|[^A-Za-z0-9_-])crossterm v' \
  || { echo "DEPS FAIL: leg 2a-bis crossterm is not in the resolved normal graph" >&2; exit 1; }
echo "DEPS OK (leg 2a-bis): crossterm undeclared but resolved, reached through ratatui"
cargo build --locked >/dev/null 2>&1 \
  || { echo "DEPS FAIL: leg 2c cargo build --locked" >&2; exit 1; }
echo "DEPS OK (leg 2c): cargo build --locked"
# leg 2d: pulldown-cmark's own defaults are off, in MANIFEST terms. The source half of the
# same claim is MDSEAM's third leg (no pulldown_cmark::html path anywhere) and the graph
# half is build-graph.sh's named absences (no getopts, no pulldown-cmark-escape). Asserted
# three ways because each alone is dodgeable: a manifest can say one thing while a feature
# is re-enabled transitively, a graph can be clean while dead code names the module, and a
# source sweep says nothing about what is built.
grep -q 'pulldown-cmark = .*default-features = false' Cargo.toml \
  || { echo "DEPS FAIL: leg 2d pulldown-cmark does not declare default-features = false" >&2; exit 1; }
cargo metadata --no-deps --format-version 1 | python3 -c '
import json,sys
d=[x for x in json.load(sys.stdin)["packages"][0]["dependencies"] if x["name"]=="pulldown-cmark"]
assert len(d)==1, d
assert d[0]["uses_default_features"] is False
assert d[0]["features"]==[], d[0]["features"]
assert d[0]["kind"] is None, "pulldown-cmark must be a normal dependency, not dev or build"
print("DEPS OK (leg 2d): pulldown-cmark is a normal dependency with no features and no defaults")
' || { echo "DEPS FAIL: leg 2d" >&2; exit 1; }

# --- leg 3: DELETED. Superseded by scripts/gates/build-graph.sh, which compares the
# resolved graph against tests/fixtures/build-graph.txt per triple and enforces a
# proc-macro ALLOWLIST. TRIPLES is kept: leg 4 still uses it.

# --- leg 4: MSRV, compared against Cargo.toml's OWN rust-version -------------
# A check carrying its own literal floor keeps enforcing the old value when the
# crate's rust-version moves, and the requirement is a claim about the relationship.
# The at-the-floor set is PRINTED, not asserted: a fixed expected set rots the moment any
# dependency bumps its own rust-version. The assertion is only that the set is non-empty
# (so the intersection matched something) and that nothing is above the floor.
{ for t in $TRIPLES; do cargo tree -e normal --target "$t" 2>/dev/null; done; } \
  | grep -oE '[A-Za-z0-9_-]+ v[0-9][^ ]*' | sed 's/ v/\t/' | sort -u > "$WORK/graph-pairs.tsv"
cargo metadata --format-version 1 | WORK="$WORK" python3 -c '
import json,sys,re,os
pairs=set()
for line in open(os.environ["WORK"]+"/graph-pairs.tsv"):
    n,v=line.rstrip("\n").split("\t"); pairs.add((n,v.split()[0]))
m=json.load(sys.stdin)
floor=None
for p in m["packages"]:
    if p["name"]=="herdr-openspec": floor=p["rust_version"]
assert floor, "the crate declares no rust-version"
def key(v):
    # Zero-pad to three components: "1.85" and "1.85.0" are the same floor, and a
    # bare tuple comparison would rank (1,85,0) above (1,85) and report a false
    # violation for every package declaring the patch digit.
    n=[int(x) for x in re.findall(r"\d+", v)[:3]]
    return tuple(n+[0]*(3-len(n)))
over=[]; at=[]
for p in m["packages"]:
    if (p["name"], p["version"]) not in pairs: continue
    rv=p.get("rust_version")
    if not rv: continue
    if key(rv)>key(floor): over.append((p["name"],p["version"],rv))
    elif key(rv)==key(floor): at.append(p["name"])
assert not over, f"packages above the {floor} floor: {over}"
assert at, "no package sits at the floor - the intersection matched nothing"
print(f"DEPS OK (leg 4): floor {floor} from Cargo.toml; at the floor: {sorted(set(at))}")
' || { echo "DEPS FAIL: leg 4" >&2; exit 1; }

# --- leg 5: each dependency is genuinely needed, tested in a COPY — DEPS_FULL only -----
# Rebuilds the crate once per dependency; kept out of `make gates` for the same reason as
# leg 1b, and given its own `make gates-full` CI job so it still runs on every push.
if [ "${DEPS_FULL:-0}" = "1" ]; then
  before=$(git status --porcelain -- Cargo.toml Cargo.lock)
  needed() { # $1 crate name, $2 the module whose failure is expected
    d="$WORK/deps-$1"; rm -rf "$d"; mkdir -p "$d"
    cp -R Cargo.toml Cargo.lock src tests rustfmt.toml "$d/"
    # The guard: removing a crate the manifest does not carry must be a FAILURE OF THE
    # CHECK, never counted as a pass — otherwise the leg silently succeeds against a
    # manifest that never declared it.
    grep -q "^$1 = " "$d/Cargo.toml" \
      || { echo "DEPS FAIL: leg 5 '$1' is not declared in Cargo.toml - nothing to remove" >&2; exit 1; }
    grep -v "^$1 = " "$d/Cargo.toml" > "$d/Cargo.toml.new" && mv "$d/Cargo.toml.new" "$d/Cargo.toml"
    if ( cd "$d" && cargo build >/dev/null 2>&1 ); then
      echo "DEPS FAIL: leg 5 the crate still builds without $1 - it is not genuinely needed" >&2
      exit 1
    fi
    echo "DEPS OK (leg 5/$1): removing it breaks the build ($2 depends on it)"
  }
  needed toml "config and state"
  needed yaml-rust2 "schema"
  needed serde_json "changes::from_cli"
  needed ratatui "ui"
  needed pulldown-cmark "ui::markdown"
  needed notify "watch"
  # The guard itself, exercised: a crate the manifest does not carry must be reported as
  # a failure of the check. Run in a subshell so its `exit 1` does not end this script.
  if ( needed notacrate "nothing" ) >/dev/null 2>&1; then
    echo "DEPS FAIL: leg 5's not-declared guard did not fire" >&2; exit 1
  fi
  echo "DEPS OK (leg 5 guard): removing an undeclared crate is reported as a failure"
  after=$(git status --porcelain -- Cargo.toml Cargo.lock)
  [ "$before" = "$after" ] \
    || { echo "DEPS FAIL: leg 5 changed Cargo.toml or Cargo.lock in the working tree" >&2; exit 1; }
  echo "DEPS OK: the working tree is unchanged, Cargo.lock included"
fi

echo "DEPS OK"
