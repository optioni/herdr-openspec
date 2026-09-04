# schema-selection Specification

## Purpose
Deciding which OpenSpec schema a repository, or one change inside it, is written under:
a change's own `.openspec.yaml`, then the repository's `openspec/config.yaml`, then the
CLI's default `spec-driven` — reporting which of the three answered, because two sources
routinely name the same schema and a name alone cannot prove the ordering. Selection is
read-only and total: a file that cannot be read, a document that is not YAML, a value of
the wrong type, and a name that would escape `openspec/schemas/` each fall through to the
next source with a named problem rather than winning, stopping the search, or failing. An
absent file, and a file that simply declares no `schema:` key, are the normal case and are
silent.

## Requirements

### Requirement: The schema name comes from the change, then the project, then the default

The plugin SHALL determine which OpenSpec schema applies by consulting, in order:

1. `<change directory>/.openspec.yaml` → the top-level `schema:` key, when a change
   directory is supplied;
2. `<repository>/openspec/config.yaml` → the top-level `schema:` key;
3. the constant default `spec-driven`.

The result SHALL name both the schema and **which of the three sources answered**, so the
ordering is an observable property rather than one inferred from the name. Two sources
frequently declare the same name — a repository whose changes all use the project schema
is the normal case — so a name alone cannot distinguish them.

Only the **first YAML document** of each file is consulted. A value SHALL count as a
declaration only when it is a YAML string that is neither empty nor whitespace-only,
applying the same blank rule `config::non_blank` establishes across this crate.

Every fallback SHALL be recorded as a human-readable problem string, appended in source
order to a list in the shape `Config::problems` and `BinResolution::problems` already use —
with one exception: a file that simply does not exist, or that parses to a mapping carrying
no `schema:` key at all, is the normal case and SHALL record nothing. A problem means
*something was there and could not be used*, never *nothing was there*. An absent key and a
present key of the wrong type SHALL therefore be distinguished, which is possible but not
automatic: the parser reports both as "no string here", and only the absent one is a
missing node.

Selection SHALL never fail, return an error, or panic. A file that cannot be read, a
document that is not valid YAML, a document that is not a mapping, and a `schema:` value of
the wrong type each fall through to the next source.

#### Scenario: A change's own declaration wins over the project's

- **WHEN** the change directory holds `.openspec.yaml` containing `schema: tdd` and the
  repository holds `openspec/config.yaml` containing `schema: spec-driven`
- **THEN** the resolved name is `tdd` and the source is the change
- **AND** no problem is recorded
- **AND** re-running with the same repository but **no** change directory argument yields
  `spec-driven` from the project, so the assertion is about precedence and not about which
  file happened to exist

#### Scenario: The project declaration answers when the change declares nothing

- **WHEN** the change directory exists but holds no `.openspec.yaml`, and
  `openspec/config.yaml` contains `schema: tdd`
- **THEN** the resolved name is `tdd` and the source is the project
- **AND** no problem is recorded

#### Scenario: Nothing declared anywhere yields the default

- **WHEN** neither `.openspec.yaml` nor `openspec/config.yaml` exists
- **THEN** the resolved name is `spec-driven` and the source is the default
- **AND** no problem is recorded, because absence is the normal case and not a fallback
- **AND** the same holds for a repository root with no `openspec/` directory at all, so a
  path that is not a repository degrades rather than failing

#### Scenario: A file that declares other keys but no `schema:` records no problem

- **WHEN** `openspec/config.yaml` is a valid YAML mapping carrying `context:` and `rules:`
  but no `schema:` key — the ordinary shape of a repository that pins nothing — and no
  change declares one
- **THEN** the resolved name is `spec-driven` from the default
- **AND** **no** problem is recorded
- **AND** the same holds for a `.openspec.yaml` carrying only `created:`
- **AND** this is the case an implementation keyed on "the value is not a string" gets
  wrong: an absent key and a `schema:` of the wrong type look identical through a
  string accessor, and only the second is a fallback worth telling a user about

#### Scenario: No change directory is supplied at all

- **WHEN** selection is asked for a repository at `<scratch>/repo` with no change directory
  argument, `openspec/config.yaml` contains `schema: tdd`, and a `.openspec.yaml` containing
  `schema: planted` — a name appearing nowhere else in the fixture — is written to **both**
  `<scratch>/repo/.openspec.yaml` and `<scratch>/.openspec.yaml`
- **THEN** the resolved name is `tdd` from the project, and never `planted`
- **AND** the two planted files are what make the assertion bite: an implementation that
  answers a `None` change directory by reading the repository root reads the first, and one
  that reads the root's parent reads the second

#### Scenario: A fallback at both sources records both problems, in order

- **WHEN** `.openspec.yaml` contains `schema: "   "` and `openspec/config.yaml` contains a
  `schema:` whose value is a nested mapping
- **THEN** the resolved name is `spec-driven` from the default
- **AND** **two** problems are recorded, the first naming `.openspec.yaml` and the second
  naming `openspec/config.yaml`
- **AND** this is the only scenario in which both sources fail at once, and it is what
  rejects an implementation that returns on the first problem, keeps only the last, or
  stores a single `Option<String>` rather than the list the requirement invokes

#### Scenario: A blank declaration is not a declaration

- **WHEN** `.openspec.yaml` contains `schema: "   "` and `openspec/config.yaml` contains
  `schema: tdd`
- **THEN** the resolved name is `tdd` from the project
- **AND** exactly one problem is recorded, naming `.openspec.yaml`
- **AND** the same run with `schema: ""` in place of the whitespace value gives the same
  result, so the rule covers empty as well as whitespace-only

#### Scenario: A declaration of the wrong type is not a declaration

- **WHEN** `openspec/config.yaml` contains `schema:` followed by a nested mapping rather
  than a scalar, and no change declares one
- **THEN** the resolved name is `spec-driven` from the default
- **AND** exactly one problem is recorded, naming `openspec/config.yaml`
- **AND** the same holds for a sequence value, for `schema: null`, and for a document that
  is a bare top-level scalar rather than a mapping at all, each asserted in the same test,
  so an implementation that string-formats any YAML node fails

#### Scenario: A file that is not valid YAML falls through and is named

- **WHEN** `.openspec.yaml` contains YAML with a tab used for indentation — which every
  YAML parser rejects — and `openspec/config.yaml` contains `schema: tdd`
- **THEN** the resolved name is `tdd` from the project
- **AND** exactly one problem is recorded, naming `.openspec.yaml` and carrying the
  parser's own message, so a user can find the offending line

#### Scenario: An unreadable file falls through and is named

- **WHEN** `openspec/config.yaml` exists as a **directory** rather than a regular file, so
  reading it fails with an I/O error rather than a parse error, and no change declares a
  schema
- **THEN** the resolved name is `spec-driven` from the default
- **AND** exactly one problem is recorded, naming the path
- **AND** the call returns rather than panicking, so no `unwrap` on the read result survives

#### Scenario: An empty or comment-only document declares nothing without complaint

- **WHEN** `openspec/config.yaml` exists but contains only a comment line, and no change
  declares a schema
- **THEN** the resolved name is `spec-driven` from the default
- **AND** **no** problem is recorded, because an empty document is indistinguishable from
  an absent one for this purpose and a parser that yields zero documents must not be
  treated as a failure
- **AND** the same holds for a completely empty file, asserted in the same test

#### Scenario: Only the first YAML document is consulted

- **WHEN** `openspec/config.yaml` holds two documents separated by `---`, the first
  containing `schema: tdd` and the second containing `schema: other`
- **THEN** the resolved name is `tdd`
- **AND** no problem is recorded

### Requirement: A declared name that would escape the schema directory is rejected

The resolved name is joined into `<repository>/openspec/schemas/<name>/schema.yaml`, so it
SHALL be a single path segment. A name that is empty after trimming, that contains a path
separator, that is exactly `.` or `..` or contains a `..` segment, that is absolute, or that
contains a NUL byte SHALL NOT be accepted from any source. The separator set is `/` **and**
`\`: a backslash is a legal filename character on both supported platforms, so rejecting it
is a choice rather than a necessity, and it is made to mirror the CLI, which splits a schema
name on `[\\/]+`. The CLI's further rejection of a `C:`-style drive prefix is deliberately
not mirrored — Windows is a PRD non-goal.

A rejected name SHALL fall through to the next source in the ordering — never win, and
never end the search — and the rejection SHALL be recorded as a problem naming the offending
value. This mirrors `repo-resolution`'s treatment of a configured `openspec_bin` that names
no usable binary: a user's mistake degrades visibly rather than either silently taking
effect or failing closed.

#### Scenario: A name containing a path separator is rejected and falls through

- **WHEN** `.openspec.yaml` contains `schema: ../../etc` and `openspec/config.yaml`
  contains `schema: tdd`
- **THEN** the resolved name is `tdd` from the project
- **AND** exactly one problem is recorded, containing the text `../../etc`

#### Scenario: Every rejected shape is rejected, and a legal name with a dot is not

- **WHEN** each of `..`, `.`, `a/b`, `a\b`, `/abs`, and a value containing a NUL byte is
  placed in turn as the only declaration, with no project declaration
- **THEN** each yields `spec-driven` from the default with exactly one problem
- **AND** in the same test the name `v1.2-tdd` is accepted and wins, so the rule rejects
  traversal rather than rejecting every name containing a dot or a digit. Without this
  positive case an `is_legal_name` that returns `false` unconditionally passes every other
  clause here, because they all expect the fall-through
- **AND** the NUL fixture is written as the YAML double-quoted escape — the document text
  `schema: "a\0b"` — not as a literal NUL byte in a plain scalar, which the parser
  silently truncates to the perfectly legal name `a`

#### Scenario: An illegal project name still falls through to the default

- **WHEN** `openspec/config.yaml` contains `schema: ../escape` and no change declares one
- **THEN** the resolved name is `spec-driven` from the default with exactly one problem,
  rather than an error or a resolution against a path outside the repository

### Requirement: Schema selection reads and never writes

Determining which schema applies SHALL be a read-only operation. It SHALL NOT create,
modify, remove, or touch any file or directory — neither the repository's `openspec/` tree,
where an agent may be editing `tasks.md` in another pane, nor the change directory, nor a
path that does not exist.

#### Scenario: A repository tree is byte-identical after selection

- **WHEN** a scratch fixture holding `openspec/config.yaml`, `openspec/changes/x/tasks.md`,
  an **empty** `openspec/specs/`, and `README.md` is snapshotted — every path, **including
  every directory**, every file's bytes, and every entry's modification time, read through
  `std::fs::Metadata` — and selection is then run twice against it
- **THEN** a second snapshot equals the first exactly, with no entry added, removed, or
  modified

#### Scenario: A missing configuration file is not created

- **WHEN** selection runs against a repository holding an `openspec/` directory with no
  `config.yaml`, and against a change directory that does not exist
- **THEN** neither `openspec/config.yaml` nor the change directory nor any
  `.openspec.yaml` exists afterwards
- **AND** the resolved name is `spec-driven` from the default
