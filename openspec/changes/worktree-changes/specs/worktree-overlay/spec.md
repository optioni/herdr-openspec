## ADDED Requirements

### Requirement: The worktree family is read from `git worktree list`, and only live members join it

The refresh worker SHALL learn the repository's worktree family by running, through the
`GitCli` seam, exactly:

```
git --no-optional-locks -C <root> worktree list --porcelain -z
```

where `<root>` is the pane's own repository root — the canonical directory `repo-discovery`
found, which holds `openspec/`. `worktrees::parse_list(stdout: &str) -> Vec<Record>` SHALL be a
pure, total function over that output: records are separated by an empty NUL-terminated field;
within a record, the field `worktree <path>` names the member's top level, `HEAD <sha>` its
commit, `branch <ref>` its branch, and the bare words `detached`, `bare`, `locked`, and
`prunable` — each optionally followed by a space and a reason — its state. A field this
requirement does not name SHALL be ignored, and a record carrying no `worktree` field SHALL be
skipped, so a newer git that adds a field never empties the family.

The worker SHALL canonicalize each record's path, since git records a worktree's path as it
was given at creation and the pane's root is canonical. The **base** member SHALL be the
record whose canonical top level is the **longest** one that is equal to, or an ancestor of,
the pane's root; the pane's root relative to that top level is the **OpenSpec prefix**, empty
when the two are equal. Every other record SHALL join the family as a **member** unless it is
`bare`, carries `prunable`, or has a path that cannot be canonicalized — each of which SHALL be
skipped **without** a problem, because a worktree whose directory is gone is git's own
bookkeeping, not a fault in the pane's repository. A member's **OpenSpec root** SHALL be its
canonical top level joined to the OpenSpec prefix.

A member's **label** SHALL be its branch with a leading `refs/heads/` removed, the full ref
when it has another prefix, and the first seven characters of its `HEAD` when it is
`detached`.

`ChangeSet::worktrees` SHALL list the family's members — never the base — in the order
`git worktree list` gave them, each as its OpenSpec root and its label.

#### Scenario: A porcelain listing with a main checkout, a branch, a detached head, and a prunable entry

- **WHEN** `parse_list` is given
  `worktree /r\0HEAD aaaa\0branch refs/heads/main\0\0worktree /w/feat\0HEAD bbbb\0branch refs/heads/feat\0\0worktree /w/det\0HEAD cccccccccc\0detached\0\0worktree /w/gone\0HEAD dddd\0detached\0prunable gitdir file points to non-existent location\0\0`
- **THEN** it returns four records in that order, the last carrying `prunable`
- **AND** with the pane's root `/r`, `/r` is the base with an empty OpenSpec prefix, and
  `ChangeSet::worktrees` is exactly `[(/w/feat, "feat"), (/w/det, "ccccccc")]` — the prunable
  record skipped, and no problem recorded for it

#### Scenario: An unknown field and a record with no path are tolerated

- **WHEN** `parse_list` is given a record carrying an extra `future-field value` line and a
  second record holding only `HEAD eeee`
- **THEN** the first record parses as if the extra field were absent and the second is skipped
- **AND** `parse_list("")` returns an empty list and never panics

#### Scenario: The OpenSpec root inside a member follows the pane's own prefix

- **WHEN** the pane's root is `/r/sub`, git reports the base top level `/r` and a member at
  `/w/feat`
- **THEN** the OpenSpec prefix is `sub` and the member's OpenSpec root is `/w/feat/sub`

#### Scenario: A pane opened inside a linked worktree treats the main checkout as a member

- **WHEN** the pane's root is `/w/feat` and git lists `/r` (branch `main`) first and `/w/feat`
  (branch `feat`) second
- **THEN** `/w/feat` is the base, and `ChangeSet::worktrees` is `[(/r, "main")]`
- **AND** the overlay below runs identically, with `main`'s touched changes laid over the
  worktree's own

### Requirement: A member owns exactly the changes it touched since it forked from the base

For each member the worker SHALL run, through `GitCli` and in this order, with `<top>` the
member's canonical top level and `<changes>` the OpenSpec prefix joined to
`openspec/changes`:

```
git --no-optional-locks -C <top> merge-base HEAD <base HEAD>
git --no-optional-locks -C <top> diff-tree -r --name-only -z --no-renames <merge-base> HEAD -- <changes>
git --no-optional-locks -C <top> status --porcelain=v1 -z --no-renames --untracked-files=all -- <changes>
```

`<base HEAD>` is the `HEAD` field of the base's own record, and `<merge-base>` is the first
command's stdout, trimmed. `worktrees::touched(diff_tree: &str, status: &str, changes: &str) ->
Touched` SHALL be a pure, total function that reads every path from both outputs — a
`status` entry is two status characters, a space, and the path — keeps those beginning with
`<changes>/`, and classifies each by what follows that prefix: `archive/<dir>/…` touches the
archived directory `<dir>`, and `<name>/…` with `<name>` not `archive` touches the active
change `<name>`. A path naming a file directly inside `openspec/changes/` or directly inside
`archive/` touches nothing.

The member **owns** exactly the changes in its `Touched` set. Ownership SHALL NOT be decided by
comparing file contents between the member and the base: a byte comparison cannot tell which
side changed, so a member that forked before the base edited or archived a change would
otherwise show its stale copy or bring an archived change back to life.

Rename detection SHALL be off in both path-listing commands: measured on git 2.48, `diff-tree`
and `diff` with renames on report an archived change's move as its destination alone, which
loses the active change it was moved from. `git diff` against the working tree SHALL NOT be used:
measured, it rewrites `.git/index` on every call even under `--no-optional-locks`, while
`status --no-optional-locks` leaves the index untouched and still ignores a file whose
timestamp changed but whose bytes did not.

#### Scenario: A committed edit, a committed archive, and uncommitted work are all owned

- **WHEN** a member's `diff-tree` output is
  `openspec/changes/archive/2026-09-24-y/tasks.md\0openspec/changes/y/tasks.md\0` and its
  `status` output is ` M openspec/changes/x/tasks.md\0?? openspec/changes/z/proposal.md\0`
- **THEN** `touched` returns the active names `{x, y, z}` and the archived directory
  `{2026-09-24-y}`

#### Scenario: Paths outside a change directory touch nothing

- **WHEN** the outputs name `openspec/changes/README.md`, `openspec/changes/archive/notes.md`,
  `openspec/specs/a/spec.md`, and `sub/openspec/changes/x/tasks.md` with `<changes>` equal to
  `openspec/changes`
- **THEN** `touched` returns empty sets, and never panics on an empty or unterminated input

#### Scenario: A worktree that forked before the base moved on owns nothing it did not touch

- **WHEN** a real scratch repository commits changes `x` and `y`, adds a linked worktree on a
  new branch, then archives `y` and edits `x` on the base branch, and the member touches
  neither
- **THEN** the member's `Touched` set is empty, and the overlaid set is the base's own: `x` at
  the base's progress and `y` under `archived` only

### Requirement: The overlay replaces, adds, and hides by ownership and nothing else

`changes::overlay` SHALL be a pure, total function from the base's change set, the base
archive's directory names, and each member's worktree, `Touched` set, and file-sourced
changes to one `ChangeSet`. A member's changes SHALL be built by the same per-change builder
`changes::from_files` uses, over the member's OpenSpec root, and only for the changes it
owns; the member's copy is never corrected by the `openspec` CLI, exactly as an archived change
never is.

For every active name some member owns, the **owner** is the first owning member in
`ChangeSet::worktrees` order, and:

1. when the owner holds an active change directory of that name, the owner's `Change`
   SHALL replace the base's change of that name, or be added when the base has none;
2. otherwise, when the owner owns an archived directory whose name, with its date prefix split
   off by `change-enumeration`'s rule, equals that name and which exists in the owner's
   archive, the base's active change of that name SHALL be **removed**;
3. otherwise — the member deleted the directory without archiving it — the base's change SHALL
   be kept unchanged.

Every archived directory a member owns that exists in its archive and whose directory name is
**not** among the base archive's directory names SHALL be added to the archived tier, the first
member in order winning when two own the same one; an archived directory the base already
holds is the base's. `archived_total` SHALL be the base's `archived_total` plus the number of
directories added, under either `ArchivedScope`; under `Full` the added changes are built and
`archived` holds all of them, and under `Names` none is built.

The result's `active` SHALL be ordered by name in byte order and its `archived` by
`change-enumeration`'s archived ordering, so the overlay never re-orders the pane by
provenance. A change nobody owns SHALL be the base's value, byte-identical.

#### Scenario: A worktree's live progress replaces the base's stale copy

- **WHEN** the base's active `x` counts 0 of 12 and the member `feat` owns `x`, whose own
  `tasks.md` counts 7 of 12
- **THEN** the overlaid `active` holds one `x`, at 7 of 12, whose `dir` is under the member's
  OpenSpec root

#### Scenario: A change created in a worktree is added

- **WHEN** the base has active `a` and `c`, and the member owns an untracked active `b`
- **THEN** the overlaid `active` is `a`, `b`, `c` in that order

#### Scenario: A change archived in a worktree leaves the active list and joins the archive

- **WHEN** the base has active `y` and 40 archived directories, and the member owns
  `openspec/changes/y/…` and `openspec/changes/archive/2026-09-24-y/…`, has no `y` directory,
  and has `archive/2026-09-24-y/`, and the set is built under `ArchivedScope::Names`
- **THEN** the overlaid `active` holds no `y`, `archived` is empty, and `archived_total` is 41
- **AND** under `ArchivedScope::Full` the same inputs yield 41 archived changes with `y`
  (dated `2026-09-24`) first, and `conformance::assert_set_invariants` accepts both results

#### Scenario: A deletion without an archive keeps the base's row

- **WHEN** the member owns active `y`, has no `y` directory, and owns no archived directory
  stripping to `y`
- **THEN** the base's `y` is in the overlaid `active`, byte-identical

#### Scenario: An archive the base already holds is not duplicated

- **WHEN** the base's archive holds `2026-08-01-old` and a member owns the same directory in its
  own archive
- **THEN** `archived_total` is unchanged and, under `Full`, `archived` holds one
  `2026-08-01-old` — the base's

#### Scenario: No member owns anything

- **WHEN** the family has two members whose `Touched` sets are both empty
- **THEN** the overlaid `active`, `archived`, `problems`, and `archived_total` equal the base's,
  and only `worktrees` differs

### Requirement: Two members owning one change is reported, and the first one shown

When two or more members own the same **active** name, the overlay SHALL apply rule 1–3 above
for the first of them only and SHALL append exactly one problem to `ChangeSet::problems` naming
the change and every owning member's label, in `worktrees` order — e.g.
`change x is modified in worktrees feat and fix; showing feat`. Two members owning the same
archived directory SHALL add it once and record no problem: an archived change is finished work
and the two copies do not compete for a row. The problem is re-derived on every cycle and
disappears when the second member stops owning the change.

#### Scenario: Two worktrees touching one proposal

- **WHEN** members `feat` and `fix`, in that order, both own active `x`, at 3 of 9 and 5 of 9
- **THEN** the overlaid `x` is `feat`'s, at 3 of 9
- **AND** `ChangeSet::problems` holds exactly one entry naming `x`, `feat`, and `fix`, which the
  list region renders as a leading `!` row

#### Scenario: The problem clears when the conflict does

- **WHEN** the next cycle finds only `feat` owning `x`
- **THEN** `ChangeSet::problems` holds no entry naming `x`

### Requirement: Without git, or without a family, the pane is exactly what it was

The overlay SHALL degrade to the base's own change set, never to an error screen and never to
an empty list:

- `git` cannot be started, `worktree list` exits non-zero (the pane's root is not in a git
  repository), times out, or reports no record containing the pane's root: `worktrees` SHALL be
  empty, the base's set SHALL be returned unchanged, and **no** problem SHALL be recorded — a
  repository without git, or without worktrees, is an ordinary one;
- a member's `merge-base`, `diff-tree`, or `status` fails or times out: that member SHALL own
  nothing this cycle, SHALL stay listed in `worktrees`, and exactly one problem SHALL be
  recorded naming the member's top level and the failing command's reason;
- a member whose checkout has no `openspec/` at its OpenSpec root owns nothing, because no
  path it reports falls under `<changes>/`, and records no problem;
- in file mode no refresh worker exists, so `worktrees` is empty and no worktree change is
  shown; `SPEC.md`'s degraded-states table records this as a limitation of file mode.

#### Scenario: No git binary

- **WHEN** the worker's `GitCli` returns `CliError::NotStarted` for `worktree list`
- **THEN** the `Files` and `Merged` results equal the un-overlaid sets, `worktrees` is empty, and
  no problem mentions git

#### Scenario: Not a git repository

- **WHEN** `worktree list` returns `CliError::Failed` with exit code 128 and
  `fatal: not a git repository`
- **THEN** the results are un-overlaid, `worktrees` is empty, and no problem is recorded

#### Scenario: One member's query fails and the other still overlays

- **WHEN** member `feat` owns `x` and member `broken`'s `merge-base` exits 1
- **THEN** `x` is `feat`'s copy, `worktrees` lists both members, and `ChangeSet::problems` holds
  exactly one entry naming `broken`'s top level and `merge-base`
- **AND** no change of the base's is removed or replaced on `broken`'s account

### Requirement: Reading the family writes nothing, in the repository or in git

Every git invocation the overlay makes SHALL carry `--no-optional-locks` and SHALL be one of
the four commands named above — `worktree list`, `merge-base`, `diff-tree`, `status` — none of
which writes a git object, a ref, or the index under that option. The overlay SHALL NOT run
`git diff` against the working tree, `worktree prune`, `fetch`, `gc`, `update-index`, or any
command taking a lock, because an agent may be committing in the very worktree being read and a
held `index.lock` makes its commit fail. The plugin's own writes remain exactly those
`plugin-state` names, under its state directory.

#### Scenario: A full cycle over a real repository and worktree leaves git's files untouched

- **WHEN** a real scratch repository with one linked worktree holding an uncommitted edit under
  `openspec/changes/x/` and a file whose timestamp was refreshed without changing its bytes is
  snapshotted — every path under both top levels and the common git directory, with bytes and
  modification times — and a real worker is driven through one request-to-`Merged` cycle and one
  idle re-check over it with the real `git`
- **THEN** a second snapshot equals the first
- **AND** the overlaid set holds the worktree's `x`, so the cycle did read the member

#### Scenario: Only the four commands are run

- **WHEN** a worker over a recording fake `GitCli` completes a cycle over a family of two members
- **THEN** every recorded argument vector begins with `--no-optional-locks`, `-C`, and a top
  level, and its fourth element is one of `worktree`, `merge-base`, `diff-tree`, and `status`
