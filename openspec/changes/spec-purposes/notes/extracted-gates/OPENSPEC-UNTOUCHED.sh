# OPENSPEC-UNTOUCHED — no code path writes inside openspec/. Diffed against the BASE SHA
# captured in task 0.1, never against the index: this project commits per task group, so
# `git diff --exit-code` between working tree and index passes over the very change it
# exists to catch.
#
# `git diff` alone is NOT enough: it lists tracked paths only, and a file the plugin WRITES
# at runtime is untracked, so the one violation this check exists to catch would be
# invisible to it. The `ls-files --others` sweeps are the half that catch it, the second of
# them covering IGNORED untracked files, since a runtime write is exactly the kind of file
# someone adds a gitignore line for.
#
# This change WATCHES openspec/, which is what makes a stray write plausible: a watcher is one
# edit away from a marker file, a lock file, or a "last seen" stamp beside the tree it reads.
# Group 11's scripted every-printable-key run, over a real ScratchDir repository with a REAL
# notify watcher open on it, is the runtime half of the same claim. This grep is the source
# half, and it is the half that sees an UNTRACKED runtime write, which `git diff` alone cannot.
#
# Exactly ONE exclusion: this change's own artifact directory, a hand-edited planning
# document rather than a code-path write. Excluded BY NAME, never by a broad prefix, so a
# stray file anywhere else under openspec/ still fails. Carried forward from tasks-tab
# with that one path updated.
[ -n "$BASE" ] || { echo "FAIL: BASE sha not set" >&2; exit 1; }
ROOT=$(git rev-parse --show-toplevel) || { echo "FAIL: not a git repo" >&2; exit 1; }
CHANGE="${CHANGE:-agent-polling}"
case "$CHANGE" in
  '') echo "FAIL: CHANGE is empty" >&2; exit 1;;
  *[!A-Za-z0-9._-]*) echo "FAIL: illegal character in CHANGE: $CHANGE" >&2; exit 1;;
esac
[ -d "$ROOT/openspec/changes/$CHANGE" ] || { echo "FAIL: no such change directory: openspec/changes/$CHANGE - the exclusion is vacuous" >&2; exit 1; }
git -C "$ROOT" rev-parse --verify "$BASE^{commit}" >/dev/null \
  || { echo "FAIL: bad BASE" >&2; exit 1; }
stray=$( { git -C "$ROOT" diff --name-only "$BASE" -- ':(top)openspec/'
           git -C "$ROOT" ls-files --others --exclude-standard -- ':(top)openspec/'
           git -C "$ROOT" ls-files --others --ignored --exclude-standard -- ':(top)openspec/'; } \
         | grep -v "^openspec/changes/$CHANGE/" | sort -u || true)
[ -z "$stray" ] || { echo "FAIL: wrote inside openspec/ outside this change:" >&2
                     echo "$stray" >&2; exit 1; }
echo "OPENSPEC-UNTOUCHED OK"
