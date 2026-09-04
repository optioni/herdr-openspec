## ADDED Requirements

### Requirement: The repository is the nearest ancestor holding an `openspec/` directory

The plugin SHALL locate the OpenSpec repository by examining a starting directory and
then each of its ancestors in turn, stopping at the first one that contains a child
named `openspec` that is a **directory**. A regular file named `openspec` SHALL NOT
satisfy the search, and a symbolic link resolving to a directory SHALL. The walk SHALL
terminate at the filesystem root, and SHALL be a function of a starting path supplied by
the caller — deriving that path from Herdr's invocation context belongs to the change
that first renders a pane.

The starting path SHALL be canonicalized before the walk when the filesystem can resolve
it, so that a path containing `..` or a symbolic link yields a repository root a human
can act on; when it cannot be resolved — most commonly because it does not exist — the
walk SHALL proceed over the path as given rather than failing.

When no ancestor qualifies, the result SHALL name the directory the search began from,
because `SPEC.md` → Degraded states requires the empty state to print it. A starting
path that does not exist, or that names a regular file rather than a directory, SHALL
produce that same not-found result rather than an error or a panic.

#### Scenario: The starting directory is itself the repository

- **WHEN** discovery runs from a scratch directory `R` that contains `R/openspec/`
- **THEN** the repository root is `R` (canonicalized), not `R`'s parent and not
  `R/openspec`

#### Scenario: The repository is an ancestor several levels up

- **WHEN** discovery runs from `R/a/b/c`, where only `R/openspec/` exists and none of
  `a`, `b`, `c` contains an `openspec` entry
- **THEN** the repository root is `R`

#### Scenario: The innermost repository wins

- **WHEN** both `R/openspec/` and `R/inner/openspec/` exist and discovery runs from
  `R/inner/deep`
- **THEN** the repository root is `R/inner`, not `R`

#### Scenario: A regular file named `openspec` is not a repository

- **WHEN** `R/openspec` exists as a regular file, `R`'s parent `P` contains
  `P/openspec/` as a directory, and discovery runs from `R`
- **THEN** the repository root is `P`, because the walk skipped `R` rather than
  accepting a file of the right name

#### Scenario: A symbolic link to a directory is a repository

- **WHEN** `R/openspec` is a symbolic link whose target is an existing directory
  elsewhere in the scratch tree, and discovery runs from `R`
- **THEN** the repository root is `R`

#### Scenario: A dangling symbolic link named `openspec` is not a repository

- **WHEN** `R/openspec` is a symbolic link to a path that does not exist, `P/openspec/`
  is a real directory in `R`'s parent, and discovery runs from `R`
- **THEN** the repository root is `P`, so a broken link behaves like an absent entry
  rather than like a directory

#### Scenario: A starting path containing `..` is resolved before the walk

- **WHEN** `R/openspec/` and `R/a/` exist and discovery runs from the path `R/a/../a`
- **THEN** the repository root is exactly the canonical `R`, with no `..` component left
  in it, so the walk did not treat `R/a/..` as a distinct ancestor

#### Scenario: A starting path naming a regular file is walked from its parent

- **WHEN** `R/openspec/` exists, `R/a/notes.md` is a regular file, and discovery runs
  from `R/a/notes.md`
- **THEN** the repository root is `R`

#### Scenario: A starting directory that does not exist is not an error

- **WHEN** discovery runs from `S/nope`, where the scratch directory `S` exists, `S/nope`
  does not, and no ancestor of `S` contains an `openspec` directory
- **THEN** no repository is found
- **AND** the result names `S/nope` — the path as given, because it cannot be
  canonicalized — rather than an empty path, the current directory, or a panic
- **AND** `S/nope` still does not exist afterwards

#### Scenario: No repository anywhere up to the filesystem root

- **WHEN** discovery runs from a scratch directory none of whose ancestors, up to and
  including `/`, contains an `openspec` directory
- **THEN** no repository is found, the result names the canonicalized scratch directory,
  and the call returns rather than looping at the root
- **AND** the test asserts that precondition on the ancestor chain before calling, and
  fails loudly naming the offending ancestor if the machine violates it, rather than
  silently passing or skipping

### Requirement: Repository discovery reads and never writes

Locating the repository SHALL be a read-only operation. It SHALL NOT create, modify,
remove, or touch any file or directory — neither inside the repository it finds, where
an agent may be editing `tasks.md` in another pane, nor on the path it walked.

#### Scenario: A repository tree is byte-identical after discovery

- **WHEN** a scratch fixture holding `R/openspec/changes/x/tasks.md`, `R/openspec/specs/`
  and an unrelated `R/README.md` is snapshotted — every path, every file's bytes, and
  every file's modification time, read through `std::fs::Metadata` — and discovery is
  then run twice from `R/openspec/changes/x`
- **THEN** a second snapshot equals the first exactly, with no entry added, removed, or
  modified

#### Scenario: A missing starting directory is not created

- **WHEN** discovery runs from a scratch path that was never created
- **THEN** neither that path nor any ancestor below the scratch root exists afterwards
