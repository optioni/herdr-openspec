## MODIFIED Requirements

### Requirement: A supported glob resolves to every matching regular file, in ascending path order

The plugin SHALL resolve a glob `generates` value over the change directory when its shape
falls inside a deliberately small supported subset:

- every directory segment is a literal containing none of `*`, `?`, `[`, except that the
  **last** directory segment may be exactly `**`; and
- the final segment is either a literal filename, or a literal prefix, exactly one `*`, and
  a literal suffix, with either part possibly empty.

`**` SHALL match zero or more directory levels, so `specs/**/*.md` matches a file directly
inside `specs/` as well as one nested below it — the behaviour of the CLI's own matcher.

Resolution SHALL yield regular files only, tested through symbolic links. It SHALL skip
every entry whose name begins with `.`, matching the CLI's matcher, whose dot option is left
at its default. Results SHALL be ordered by full path ascending in byte order and SHALL
contain no duplicate.

Resolution SHALL NOT descend into a directory reached through a symbolic link, so no walk
can loop. This is a **second knowing divergence from the CLI**, alongside the invalid-UTF-8
one `SPEC.md` → Degraded states already records, and it SHALL be carried as its own row of
that table rather than only as a remark on a scenario here.

The divergence's shape was measured against `@fission-ai/openspec` 1.12.0, and it is
narrower than a reading of `followSymbolicLinks: true` alone suggests. The CLI has **three**
behaviours where this plugin has one:

- a symlinked directory whose target canonicalizes **inside** the change directory — the
  CLI follows it and lists the files behind it (`followSymbolicLinks: true`,
  `dist/core/artifact-graph/outputs.js:93`), the plugin does not. **This, and only this, is
  the divergence.**
- a symlinked directory whose target canonicalizes **outside** the change directory — the
  CLI does not over-list, it **fails closed**: `assertGlobDirectoryTraversal` canonicalizes
  each candidate and `assertPathWithin` throws `Path is outside the allowed directory`
  (`dist/core/artifact-graph/outputs.js:23-27`, `dist/utils/file-system.js:95-106`), which
  reaches the surface as `openspec list --json` answering an empty `changes` array with a
  `list_error`. The plugin quietly lists the rest of the change instead, which is the
  never-fail-closed rule working as intended and is **not** the divergence row's subject.
- a **linked cycle** — the CLI throws `Cannot resolve artifact outputs through a linked
  directory cycle`, which this plugin may not do for the same reason.

The vendored `tdd` schema's own `specs/**/*.md` pattern
(`openspec/schemas/tdd/schema.yaml:37`) is where a repository would meet the first case:
a `specs/shared -> ../inner` link, whose target is still inside the change directory, is
listed by the CLI and skipped by the file tier.

The divergence SHALL be bounded to the **artifact list**, never to a change's `progress`:
both the vendored `tdd` schema and the CLI's bundled `spec-driven` schema declare
`apply.tracks` as a literal filename, so the tracked-tasks artifact resolves through the
non-glob path where no directory is walked at all. A schema whose tracked-tasks artifact
*is* glob-shaped and whose task files sit behind a directory symlink would diverge in
progress too; no such schema is in use, and the row records the limit rather than claiming
it cannot happen.

The CLI's own list SHALL correct the artifact list when it arrives, on `change-merge`'s
ordinary terms, so the divergence is visible only in the interval before the CLI answers
and permanently only in file mode.

#### Scenario: A nested spec tree resolves in path order

- **WHEN** `generates` is `specs/**/*.md` and the change directory holds
  `specs/Beta/spec.md`, `specs/alpha/spec.md`, `specs/alpha/nested/deep.md`, and
  `specs/top.md`
- **THEN** the `ArtifactRef` carries all four paths, ordered `specs/Beta/spec.md`,
  `specs/alpha/nested/deep.md`, `specs/alpha/spec.md`, `specs/top.md`
- **AND** the order is byte order on the full path, so `Beta` precedes `alpha`

#### Scenario: A glob matching nothing is an empty path list, not a problem

- **WHEN** `generates` is `specs/**/*.md` and the change directory holds no `specs/`
  directory at all
- **THEN** the `ArtifactRef` carries no paths and records no problem
- **AND** the same holds when `specs/` exists and is empty

#### Scenario: Dot entries and non-files are skipped

- **WHEN** `generates` is `specs/**/*.md` and `specs/` holds `.hidden.md`, a
  `.hidden-cap/spec.md`, a directory named `looks-like.md`, and one real `alpha/spec.md`
- **THEN** only `specs/alpha/spec.md` is carried
- **AND** no problem is recorded for any of the skipped entries

#### Scenario: A directory symbolic link is not descended into

- **WHEN** `generates` is `specs/**/*.md` and `specs/` holds a real `alpha/spec.md` and a
  symbolic link `loop/` pointing back at `specs/`
- **THEN** only `specs/alpha/spec.md` is carried, and the call terminates
- **AND** a file reached only through the link is absent, which is the knowing divergence
  from the CLI's matcher recorded in `SPEC.md`'s degraded-states table and corrected by the
  CLI path

#### Scenario: A non-looping directory symlink resolving inside the change is skipped too

- **WHEN** `generates` is `specs/**/*.md`, the change directory holds a real
  `specs/alpha/spec.md` and an `inner/one.md`, and `specs/shared` is a symbolic link to
  `../inner` — a link with no cycle whose target canonicalizes **inside** the change
  directory, the one case where the CLI follows and succeeds
- **THEN** only `specs/alpha/spec.md` is carried, and no problem is recorded
- **AND** `inner/one.md` is absent from the artifact's paths even though it is inside the
  change directory and reachable, so the skip is unconditional rather than cycle-detecting:
  the plugin diverges from the CLI exactly where the CLI would succeed, which is what makes
  this a divergence row and not a cycle-safety measure

#### Scenario: A symlink resolving outside the change is skipped without failing closed

- **WHEN** `generates` is `specs/**/*.md`, `specs/alpha/spec.md` is real, and
  `specs/escape` is a symbolic link to a directory **outside** the change directory
- **THEN** only `specs/alpha/spec.md` is carried, no problem is recorded, and the change is
  still produced with every other artifact resolved
- **AND** this is deliberately **not** the divergence row's subject: the CLI fails closed
  here (`Path is outside the allowed directory`, surfacing as an empty `list --json`
  result), so skipping is the stricter-and-safer answer rather than the lossy one

#### Scenario: A glob-shaped tasks artifact behind a symlink is the divergence's known limit

- **WHEN** a schema declares `apply.tracks: tasks/**/*.md` selecting an artifact generating
  the same, and the change directory holds `tasks/real.md` at 1/2 and a symlink
  `tasks/linked` pointing at `../other` — **inside** the change directory — whose
  `extra.md` is at 0/3
- **THEN** the change's `progress` is `Progress { completed: 1, total: 2 }`
- **AND** the CLI reports `1/5` for the same tree, recorded in the test as a documented
  constant with its measurement noted, not re-measured by invoking the CLI; the link's
  target must stay inside the change directory or the constant does not hold, because an
  outside-resolving link makes the CLI report no changes at all

#### Scenario: A prefix-and-suffix file pattern is supported

- **WHEN** `generates` is `specs/spec-*.md` and `specs/` holds `spec-a.md`, `spec-b.md`, and
  `other.md`
- **THEN** the `ArtifactRef` carries `specs/spec-a.md` and `specs/spec-b.md` in that order
