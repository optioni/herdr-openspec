# OPENSPEC-UNTOUCHED (tree-only legs) — no code path writes an UNTRACKED file inside
# openspec/. Extracted by degraded-states (design.md -> Decision 9): the archived
# OPENSPEC-UNTOUCHED also diffs the tracked tree against a per-change BASE SHA, which needs
# a value only a specific change's own apply session has — that leg stays a per-change
# invocation and is NOT here. These two `git ls-files --others` sweeps need nothing but the
# working tree, hold on any commit, and are the only mechanical guard on the PRD non-goal
# that nothing writes inside openspec/ that composes into `make check` itself.
#
# `git diff` (the leg NOT here) lists tracked paths only, and a file the plugin WRITES at
# runtime is untracked, so the one violation this half exists to catch would be invisible
# to a tracked-diff alone. The second sweep additionally covers IGNORED untracked files,
# since a runtime write is exactly the kind of file someone adds a gitignore line for.
ROOT=$(git rev-parse --show-toplevel) || { echo "OPENSPEC-UNTOUCHED FAIL: not a git repo" >&2; exit 1; }
stray=$( { git -C "$ROOT" ls-files --others --exclude-standard -- ':(top)openspec/'
           git -C "$ROOT" ls-files --others --ignored --exclude-standard -- ':(top)openspec/'; } \
         | sort -u || true)
[ -z "$stray" ] || { echo "OPENSPEC-UNTOUCHED FAIL: an untracked file exists inside openspec/:" >&2
                     echo "$stray" >&2; exit 1; }
echo "OPENSPEC-UNTOUCHED OK (tree-only legs): no untracked file inside openspec/"
