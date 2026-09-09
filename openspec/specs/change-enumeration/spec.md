# change-enumeration Specification

## Purpose

Deciding which directories on disk are changes. Active changes come from
`openspec/changes/`, archived ones from `openspec/changes/archive/`, and both lists have to
hold the same names the OpenSpec CLI would report — the file path paints the pane and the
CLI path corrects it, so a directory one side counts and the other does not is a row that
appears or vanishes by itself after the pane has been read.

The active rule is therefore copied from `openspec list --json` rather than designed, the
same way `task-checkboxes` copied the counting rule. The archived rule cannot be copied. The
CLI has two archived listings — one behind shell completion, one inside `openspec validate`
— but neither is a command whose output a user reads, neither splits the date prefix, and
both simply sort raw directory names. The ordering a dashboard needs is therefore the
plugin's own, and this capability is where it is written down.

## Requirements

### Requirement: An active change is any directory under `openspec/changes/` other than `archive`

The plugin SHALL treat every entry directly under `<repo>/openspec/changes/` that is a
directory, and whose name is not exactly `archive`, as an active change. The change's name
is the directory's name.

No marker file SHALL be required. An empty directory is a change, and so is one holding
only `.openspec.yaml` — the OpenSpec CLI removed its own `proposal.md` requirement because
`openspec new change` scaffolds only `.openspec.yaml`, and a plugin that required more
would hide a change the moment it was created.

The `archive` exclusion SHALL be an exact whole-name comparison, not a prefix match: a
directory named `archives` or `archived-ideas` is an ordinary change. The comparison is
byte equality, which on a case-sensitive filesystem also makes `Archive/` an ordinary
change; that half is deliberately left without a scenario, because the reference machine's
APFS volume is case-insensitive and could not hold `archive/` and `Archive/` side by side to
observe it.

The directory test SHALL be the one that does **not** follow symbolic links, so a symbolic
link pointing at a directory is not an active change. This is the CLI's behaviour rather
than a choice — its listing reads directory entries with `withFileTypes`, whose
`isDirectory()` is `lstat`-based — and in Rust it is `DirEntry::file_type()`, never
`Path::is_dir()`, which follows links and would list a change the CLI does not. It is
deliberately the opposite rule from `repo-discovery`'s, which follows links when testing
for `openspec/` because that test answers a different question.

Entries whose names begin with `.` SHALL be included, matching `openspec list --json`.

#### Scenario: A directory with no marker file is a change

- **WHEN** `openspec/changes/` holds `empty-dir/` containing nothing, `only-yaml/`
  containing only `.openspec.yaml`, and `only-proposal/` containing only `proposal.md`
- **THEN** all three appear in `active`, named `empty-dir`, `only-proposal`, and
  `only-yaml`
- **AND** none of the three records a problem for its missing artifacts

#### Scenario: A regular file is not a change

- **WHEN** `openspec/changes/` holds a regular file `notes.md` alongside a directory
  `real-change/`
- **THEN** `active` holds exactly one change, named `real-change`
- **AND** no problem is recorded for the file

#### Scenario: A symbolic link to a directory is not a change

- **WHEN** `openspec/changes/` holds a directory `real-target/` and a symbolic link
  `linked/` pointing at it
- **THEN** `active` holds exactly one change, named `real-target`
- **AND** a dangling symbolic link in the same directory is likewise not a change and
  causes no failure

#### Scenario: The archive exclusion is by exact name

- **WHEN** `openspec/changes/` holds `archive/`, `archives-not-excluded/`, and
  `archive-notes/`
- **THEN** `active` holds `archive-notes` and `archives-not-excluded` and does not hold
  `archive`

#### Scenario: A dot-directory is a change

- **WHEN** `openspec/changes/` holds `.dot-change/` and `plain/`
- **THEN** `active` holds both, named `.dot-change` and `plain`
- **AND** the result matches `openspec list --json`, which applies no dot filter, rather
  than the CLI's shell-completion listing, which does

### Requirement: Active changes are ordered by name ascending, in byte order

The plugin SHALL sort `active` by directory name ascending, comparing bytes rather than
applying a locale.

The ordering is specified here because both producers must use it. `openspec list --json`
defaults to most-recently-modified first, so `changes-from-cli` SHALL re-sort by name; if it
did not, the list would reorder itself the moment the CLI result arrived, which is the
dual-source model's one visible failure. Byte order is chosen over a locale comparison
because it is the order the CLI's own identifier listings use and because it is the same on
every machine.

#### Scenario: Case and digits order by byte, not by locale

- **WHEN** `openspec/changes/` holds `Beta/`, `alpha/`, `10-late/`, and `2-early/`
- **THEN** `active` is ordered `10-late`, `2-early`, `Beta`, `alpha`
- **AND** the order does not depend on the machine's locale

### Requirement: An archived directory's date prefix is split off its name

The plugin SHALL treat every entry directly under `<repo>/openspec/changes/archive/` that is
a directory and whose name does not begin with `.` as an archived change, applying the same
non-link-following directory test as the active listing.

For each, the plugin SHALL split a leading `YYYY-MM-DD-` prefix off the directory name using
exactly the CLI's own pattern — four digits, a hyphen, two digits, a hyphen, two digits, a
hyphen — with **no** calendar validation, because that is the pattern the CLI writes with
and validating further would classify a directory the CLI created as undated. The remainder
becomes the change's `name` and the prefix's date becomes the archived origin's date.

An entry whose name does not match the pattern, or whose remainder after the prefix is
empty, SHALL keep its whole directory name as its `name` and carry no date. A prefix with
an empty remainder is kept whole rather than yielding a nameless row.

Dot-prefixed entries are excluded here, unlike the active listing. The two differ on
purpose: the active rule matches `openspec list --json`, because that is the command
`changes-from-cli` parses and the one list whose disagreement would be visible in the pane;
the archive rule matches both of the CLI's archived listings, which filter them, and no
producer will ever contradict it. Because the two rules are deliberately opposite, an
implementation SHALL NOT share one filtering helper between them.

#### Scenario: A normal archived directory splits into a date and a name

- **WHEN** `openspec/changes/archive/` holds `2026-08-14-add-token-refresh/`
- **THEN** the archived change has `name` `add-token-refresh` and date `2026-08-14`
- **AND** its `dir` still ends with the full directory name `2026-08-14-add-token-refresh`

#### Scenario: An impossible date is still a date prefix

- **WHEN** `openspec/changes/archive/` holds `9999-99-99-far-future/`
- **THEN** the archived change has `name` `far-future` and date `9999-99-99`
- **AND** no problem is recorded, because the pattern is the CLI's and the CLI does not
  validate the calendar either

#### Scenario: A malformed or absent prefix keeps the whole name

- **WHEN** `openspec/changes/archive/` holds `2026-1-1-single-digits/`, `20260814-nohyphen/`,
  and `no-date-prefix/`
- **THEN** all three are archived changes named `2026-1-1-single-digits`,
  `20260814-nohyphen`, and `no-date-prefix`, each carrying no date
- **AND** no problem is recorded for any of them

#### Scenario: A prefix with nothing after it keeps its whole name

- **WHEN** `openspec/changes/archive/` holds `2026-08-14-/`
- **THEN** the archived change is named `2026-08-14-` and carries no date
- **AND** no archived change ever has an empty `name`

#### Scenario: A dot-prefixed archive directory is not an archived change

- **WHEN** `openspec/changes/archive/` holds `.hidden-archived/` and `2026-08-14-real/`
- **THEN** the archived list holds only `real`
- **AND** the active listing over the same repository still lists a `.dot-change/` under
  `openspec/changes/`, so the two rules are observably opposite in one test

#### Scenario: Two archived directories can strip to the same name

- **WHEN** `openspec/changes/archive/` holds `2026-01-05-retry-policy/` and
  `2026-07-22-retry-policy/`
- **THEN** both appear, each named `retry-policy`, with dates `2026-07-22` and `2026-01-05`
- **AND** neither suppresses the other, because they are two distinct pieces of work

### Requirement: Archived changes are ordered newest first, with undated entries last

The plugin SHALL order archived changes by date descending, and SHALL place every entry
carrying no date after every entry carrying one, ordering those among themselves by
directory name descending.

The undated-last rule follows `openspec-binary`'s existing treatment of an nvm version
directory whose name does not parse: the unparseable entry is kept and sorted after every
parsed one, rather than being dropped or allowed to float to the top of a list sorted by
raw name. Two entries carrying the same date SHALL order by directory name descending, so
the result is total and identical on every machine rather than falling back to whatever
order the filesystem returned the entries in.

#### Scenario: Dated entries come newest first

- **WHEN** the archive holds `2026-01-05-a/`, `2026-09-01-c/`, and `2026-07-22-b/`
- **THEN** the archived list is ordered `c`, `b`, `a`

#### Scenario: Two entries sharing a date order by name descending

- **WHEN** the archive holds `2026-05-01-alpha/` and `2026-05-01-zeta/`
- **THEN** the archived list is ordered `zeta`, `alpha`
- **AND** the order does not depend on the order the filesystem returned the entries in

#### Scenario: An undated entry sorts after every dated one

- **WHEN** the archive holds `zeta-undated/`, `2026-01-05-a/`, and `alpha-undated/`
- **THEN** the archived list is ordered `a`, `zeta-undated`, `alpha-undated`
- **AND** the undated pair is ordered by name descending, so a name beginning with a letter
  cannot outrank a dated entry the way a plain descending sort of raw names would

### Requirement: A missing or unreadable changes directory degrades rather than failing

The plugin SHALL treat an absent `openspec/`, an absent `openspec/changes/`, and an absent
`openspec/changes/archive/` as empty results carrying **no** problem: a repository with no
changes, or with nothing archived yet, is a supported state rather than a fault, and the
OpenSpec CLI treats the same absences the same way.

An `openspec/changes/` that exists but cannot be read SHALL yield empty `active` and
`archived` lists plus exactly one problem on `ChangeSet::problems` naming the directory and
the reason. The archive walk SHALL NOT be attempted beneath it: reading
`openspec/changes/archive/` under an unreadable parent fails with a permission error rather
than a not-found one, and reporting it would produce a second problem describing the same
fault. An `openspec/changes/archive/` that exists but cannot be read SHALL leave
`active` intact, yield an empty `archived`, and record exactly one problem. An `archive`
entry that is a regular file rather than a directory SHALL yield an empty `archived` with
no problem, because it is simply not an archive.

#### Scenario: A repository with no `openspec` directory yields an empty set

- **WHEN** `from_files` is given a directory holding no `openspec/` at all
- **THEN** `active`, `archived`, and `problems` are all empty
- **AND** nothing is created on disk, including the directories that were looked for

#### Scenario: No active changes still lists the archive

- **WHEN** `openspec/changes/` holds only `archive/`, which holds two dated directories
- **THEN** `active` is empty and `archived` holds both
- **AND** `problems` is empty

#### Scenario: An unreadable changes directory is one named problem

- **WHEN** `openspec/changes/` exists with its read permission removed
- **THEN** `active` and `archived` are empty and `ChangeSet::problems` holds exactly one
  entry naming the directory
- **AND** the process does not panic, no partial change is invented, and no second problem
  is recorded for the archive directory beneath it, whose own read would fail with a
  permission error rather than a not-found one

#### Scenario: An unreadable archive leaves the active list intact

- **WHEN** `openspec/changes/` holds two readable change directories and an `archive/`
  whose read permission has been removed
- **THEN** `active` holds both changes and `archived` is empty
- **AND** `ChangeSet::problems` holds exactly one entry naming the archive directory

#### Scenario: An `archive` that is a regular file is not an archive

- **WHEN** `openspec/changes/` holds a regular file named `archive` and one change
  directory
- **THEN** `active` holds the one change, `archived` is empty, and `problems` is empty
- **AND** the file named `archive` is not listed as a change, which is already true of every
  regular file and does not depend on its name

### Requirement: Enumeration reads and never writes

Enumerating changes SHALL create, modify, delete, and touch nothing. No directory is
created for a path that was looked for and not found, no file is opened for writing, and no
lock, cache, or marker file is left behind, anywhere — inside `openspec/` most of all,
because an agent may be editing a file there in another pane.

#### Scenario: The repository tree is byte-identical after enumeration

- **WHEN** a full `from_files` — including the archive listing and the per-change work — is
  run over a repository tree, with a snapshot of every directory entry, every file's bytes,
  and every modification time taken immediately before and after
- **THEN** the two snapshots compare equal
- **AND** the paths the walk looked for and did not find, such as an absent
  `openspec/changes/archive/`, still do not exist afterwards

### Requirement: The archived tier is enumerated in full and resolved only when it is shown

`changes::from_files(repo: &Path, archived: ArchivedScope) -> ChangeSet` SHALL take a scope
in place of the `archived_count` limit, where `changes::ArchivedScope` is an enum with
exactly two variants:

```rust
pub enum ArchivedScope {
    /// Enumerate and count the archive; build no archived `Change`.
    Names,
    /// Build every enumerated archived change.
    Full,
}
```

The **enumeration** SHALL be identical under both scopes and SHALL NOT be truncated: every
directory directly under `openspec/changes/archive/` whose name does not begin with `.` is
listed, its `YYYY-MM-DD-` prefix split off, and the result ordered dated-newest-first with
same-date ties broken by name descending, then every undated entry ordered among itself by
name descending — exactly the ordering this capability already requires, with the final
`truncate` step removed and nothing else changed. `ChangeSet::archived_total` SHALL be the
length of that ordered list under both scopes.

Under `ArchivedScope::Full` every enumerated entry SHALL be built into a `Change` exactly as
before, so `archived` holds `archived_total` entries in the enumerated order.

Under `ArchivedScope::Names` `archived` SHALL be empty, and building SHALL be skipped
entirely: for no archived change SHALL its `.openspec.yaml` be read, its schema be resolved
or loaded, its artifact paths be resolved, or its task file be opened. A collapsed section
therefore costs one `read_dir` of the archive directory and a sort, and no per-change file
work at all — which is the whole reason the scope exists, since lifting the cap without it
would make every refresh cycle resolve every archived change.

That "no file work" claim SHALL be verified by a **counting seam**, never by an assertion on
the returned `ChangeSet`. Every field of the return value is pinned to a scope-independent
value by this requirement — `active`, `problems` and the ordering unchanged, `archived_total`
identical, `archived` empty — so an implementation that resolved every archived change and
discarded the result would satisfy every value-level assertion that can be written. A
`#[cfg(test)]` thread-local path recorder in `schema::read_file` and `tasks::read` SHALL
record each path they open, and the test SHALL assert **zero** recorded paths beneath
`openspec/changes/archive/` under `Names`, with the `Full` run over the same tree as its
positive control.

The scope SHALL NOT affect the active tier, the repository-level `problems` list, or the
ordering of either tier, and SHALL NOT cause a problem of its own: `Names` is a normal mode
of operation, not a degraded one.

#### Scenario: The full archive is enumerated and counted under either scope

- **WHEN** an archive holding seven dated directories with seven distinct dates is passed to
  `from_files` with `ArchivedScope::Full` and again with `ArchivedScope::Names`
- **THEN** the `Full` result's `archived` holds all seven, newest first, and its
  `archived_total` is 7
- **AND** the `Names` result's `archived` is empty and its `archived_total` is 7
- **AND** both results' `active` lists and `problems` lists are equal, and neither records a
  problem

#### Scenario: A collapsed archive opens no file beneath an archived change

- **WHEN** a scratch archive holds three dated directories, each with an `.openspec.yaml` and
  a `tasks.md`, the recorder is cleared, and `from_files` is called with
  `ArchivedScope::Names`
- **THEN** the recorder holds **zero** paths beneath `openspec/changes/archive/`, and the
  result's `archived` is empty with `archived_total` 3 and `problems` empty
- **AND** clearing the recorder and calling `from_files` with `ArchivedScope::Full` over the
  same tree records **at least three** paths beneath that directory — the positive control,
  without which a recorder wired to nothing would report zero for both arms and prove nothing
- **AND** both calls leave the tree byte-identical, per this capability's
  reads-and-never-writes requirement
- **AND** the recorder is a `thread_local!`, never a `static`, because the suite runs this
  crate's tests in parallel threads of one process

#### Scenario: An unresolvable archived change is a `Change` problem under `Full` and absent under `Names`

- **WHEN** an archive holds three dated directories, the newest of which has had its mode set
  to `0o000` — read **and** execute removed, so opening a file beneath it fails rather than
  only listing it — and `from_files` is called under both scopes
- **THEN** the `Full` result's `archived` holds three changes and the newest one's own
  `Change::problems` is non-empty
- **AND** the `Names` result's `archived` is empty and its `problems` is empty
- **AND** `ChangeSet::problems` is empty in **both** results, because a change's own problems
  are never merged upward — which is why this scenario is evidence about degradation and the
  scenario above, not this one, is the evidence about cost

#### Scenario: An empty archive counts zero under either scope

- **WHEN** a repository with two active changes and no `openspec/changes/archive/` at all is
  passed to `from_files` under both scopes
- **THEN** both results hold both active changes, an empty `archived`, an `archived_total` of
  `0`, and an empty `problems`
- **AND** the same holds when `archive/` exists and is empty, and when `archive` is a regular
  file rather than a directory

#### Scenario: A surviving scenario names its scope

- **WHEN** `openspec/changes/` holds only `archive/`, which holds two dated directories, and
  `from_files` is called with `ArchivedScope::Full`
- **THEN** `active` is empty, `archived` holds both, `archived_total` is 2, and `problems` is
  empty
- **AND** the same call with `ArchivedScope::Names` yields an empty `archived` and the same
  `archived_total` of 2 — the landed *No active changes still lists the archive* scenario is
  pinned to `Full`, since the scope is now the deciding parameter and an unqualified assertion
  would pass under one arm and fail under the other

#### Scenario: An unreadable archive counts nothing and reports once, under either scope

- **WHEN** `openspec/changes/` holds two readable change directories and an `archive/` whose
  read permission has been removed, under both scopes
- **THEN** both results hold both active changes, an empty `archived`, an `archived_total` of
  `0`, and exactly one problem naming the archive directory
- **AND** neither scope records a second problem, so the count being unavailable is not itself
  reported
