## REMOVED Requirements

### Requirement: `archived_count` truncates the archived list after it is ordered

**Reason**: The cap was standing in for a missing interaction. The list was one flat run with
no fold, so truncating the archive was the only lever available for keeping it readable;
`list-sections` gives the archived tier a collapsible section, which is the lever the cap was
approximating. With a fold present, a cap that silently drops entries — twenty-two archived
changes on disk, five in the pane, seventeen with no row, badge, or count anywhere admitting
it — is a defect rather than a setting.

**Migration**: None is required of a reader or of a `config.toml`. `archived_count` is still
parsed, still defaults to `5`, and still reports a malformed value on `Config::problems`
(`plugin-config`); it simply no longer limits what is enumerated or rendered. A reader who
set `archived_count = 25` wanted the archive visible and now gets all of it; a reader who
left it at `5` sees the archived section **collapsed** on startup, with its true total on the
header, and opens it with `Space`.

## ADDED Requirements

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

- **WHEN** an archive holds three dated directories, the newest of which has had its own read
  permission removed so that resolving it would record a problem on that `Change`, and
  `from_files` is called with `ArchivedScope::Names` and again with `ArchivedScope::Full`
- **THEN** the `Names` result's `archived` is empty, its `archived_total` is 3, and its
  `problems` is empty — nothing beneath the unreadable directory was reached
- **AND** the `Full` result's `archived` holds three changes and the newest one's own
  `Change::problems` is non-empty, so the difference is resolution being skipped rather than
  the fixture being inert
- **AND** the `Names` call leaves the tree byte-identical, per this capability's
  reads-and-never-writes requirement

#### Scenario: An empty archive counts zero under either scope

- **WHEN** a repository with two active changes and no `openspec/changes/archive/` at all is
  passed to `from_files` under both scopes
- **THEN** both results hold both active changes, an empty `archived`, an `archived_total` of
  `0`, and an empty `problems`
- **AND** the same holds when `archive/` exists and is empty, and when `archive` is a regular
  file rather than a directory

#### Scenario: An unreadable archive counts nothing and reports once, under either scope

- **WHEN** `openspec/changes/` holds two readable change directories and an `archive/` whose
  read permission has been removed, under both scopes
- **THEN** both results hold both active changes, an empty `archived`, an `archived_total` of
  `0`, and exactly one problem naming the archive directory
- **AND** neither scope records a second problem, so the count being unavailable is not itself
  reported
