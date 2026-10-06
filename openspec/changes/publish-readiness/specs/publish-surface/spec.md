## ADDED Requirements

### Requirement: The licence is declared once and agreed in three places

The repository SHALL carry an MIT licence that is stated identically in three places:

1. `LICENSE` at the repository root. Its first non-empty line SHALL be `MIT License`.
2. `Cargo.toml`'s `[package]` table, as `license = "MIT"`.
3. `README.md`'s `## Licence` section. It SHALL name `MIT` and link the `LICENSE` file.

`tests/publish_surface.rs` SHALL check all three inside `cargo test`. It reads `Cargo.toml` by
parsing it as TOML with the crate's existing `toml` dependency, never by spawning
`cargo metadata`. A failure SHALL name each site that disagrees. Reading these files is the
test's only effect, and it writes nothing anywhere.

#### Scenario: All three sites agree

- **WHEN** `LICENSE` begins `MIT License`, `Cargo.toml` declares `license = "MIT"`, and
  `README.md` → Licence reads `MIT` with a link to `LICENSE`
- **THEN** the licence check passes

#### Scenario: The LICENSE file is missing

- **WHEN** the repository root has no `LICENSE` file, while `README.md` → Licence still reads
  `MIT`
- **THEN** the licence check fails with a message naming `LICENSE` as missing
- **AND** no file is created to satisfy it

#### Scenario: Cargo.toml declares no licence

- **WHEN** `LICENSE` and `README.md` both say MIT but `Cargo.toml`'s `[package]` has no
  `license` key
- **THEN** the licence check fails with a message naming `Cargo.toml`'s `license`

#### Scenario: The README names a different licence

- **WHEN** `README.md` → Licence reads `Apache-2.0` while the other two sites say MIT
- **THEN** the licence check fails with a message naming both `README.md` → Licence and the
  value it disagrees with

#### Scenario: The README has no Licence section

- **WHEN** `README.md` has no `## Licence` heading
- **THEN** the licence check fails with a message naming the missing section, rather than
  passing because there is nothing to compare

### Requirement: The package's repository URL agrees with the README's install command

`Cargo.toml`'s `[package]` SHALL declare a non-empty `description` and a `repository` of the
form `https://github.com/<owner>/<repo>`. `README.md`'s install command
(`herdr plugin install <owner>/<repo>`) SHALL name that same `<owner>/<repo>`.
`tests/publish_surface.rs` SHALL check that the two slugs are equal, so a rename or transfer
that updates one of them without the other fails `cargo test`.

#### Scenario: The slugs agree

- **WHEN** `Cargo.toml` declares `repository = "https://github.com/optioni/herdr-openspec"`
  and `README.md` contains `herdr plugin install optioni/herdr-openspec`
- **THEN** the repository check passes

#### Scenario: The repository moved but the README did not

- **WHEN** `Cargo.toml`'s `repository` names `example/herdr-openspec` while `README.md` still
  installs `optioni/herdr-openspec`
- **THEN** the repository check fails, naming both slugs

#### Scenario: Metadata is absent

- **WHEN** `Cargo.toml` has no `description`, or its `description` is the empty string, or it
  has no `repository`
- **THEN** the repository check fails, naming the missing key

### Requirement: The README shows a demo image the tape renders

`README.md` SHALL embed exactly one image whose relative path is under `docs/`, and that
image SHALL appear before the `## Install` heading. The file at that path SHALL exist, SHALL
be non-empty, SHALL begin with a GIF signature (`GIF87a` or `GIF89a`), and SHALL be no larger
than 2 MiB. The size cap has a reason: `tests/gate_controls.rs` copies the whole tracked tree
once per planted control, more than sixty times per suite run, so every committed megabyte is
paid for many times over in every `make check`. The checked-in
`vhs` tape at `docs/demo/demo.tape` SHALL contain exactly one `Output` line, and that line
SHALL name the same path, so re-rendering the tape replaces the image the README shows rather
than writing a new one beside it.

`tests/publish_surface.rs` SHALL check all of this by reading files only. It SHALL NOT run
`vhs`, build the release binary, or spawn any process. `vhs` is an author's tool, and is not a
dependency of `make check` or CI.

#### Scenario: The image, the file, and the tape agree

- **WHEN** `README.md` embeds `docs/demo.gif` above `## Install`, `docs/demo.gif` begins
  `GIF89a`, and `docs/demo/demo.tape` contains the single line `Output docs/demo.gif`
- **THEN** the demo-image check passes

#### Scenario: The image is referenced but was never rendered

- **WHEN** `README.md` embeds `docs/demo.gif` but that file does not exist
- **THEN** the demo-image check fails, naming `docs/demo.gif`

#### Scenario: The file is not a GIF

- **WHEN** `docs/demo.gif` exists but is empty, or begins with a PNG signature
- **THEN** the demo-image check fails, saying that the file is not a GIF

#### Scenario: The image outgrows its budget

- **WHEN** `docs/demo.gif` is a valid GIF of 2 MiB plus one byte
- **THEN** the demo-image check fails, stating the file's size and the 2 MiB cap

#### Scenario: The tape renders somewhere else

- **WHEN** `docs/demo/demo.tape`'s `Output` line names `demo.gif` while `README.md` embeds
  `docs/demo.gif`
- **THEN** the demo-image check fails, naming both paths

#### Scenario: The README carries no image

- **WHEN** `README.md` embeds no image, or embeds its only image below `## Install`
- **THEN** the demo-image check fails rather than passing vacuously

### Requirement: The demo repository loads cleanly through the file loader

`docs/demo/` SHALL be a self-contained OpenSpec repository: an `openspec/` directory holding
its own `config.yaml`, its own schema under `openspec/schemas/`, at least two active changes,
at least one archived change under `openspec/changes/archive/`, and at least one capability
under `openspec/specs/`. Because it carries its own schema, it SHALL load in file mode, with no
`openspec` binary and no `SchemaLocations`. This is the same degraded path the dashboard takes
on a machine without the CLI.

`tests/publish_surface.rs` SHALL load it with
`herdr_openspec::changes::from_files(docs/demo, ArchivedScope::Full)`, the crate's own
read-only loader, and SHALL require all of the following:

- the `ChangeSet`'s `problems` is empty;
- every change's `problems` is empty;
- `active` holds at least two changes, and `archived` at least one;
- at least one active change has partial progress (`0 < done < total`), and at least one
  change has full progress (`done == total`, `total > 0`), so the recorded demo shows the
  gauge mid-way and complete.

The check reads the fixture and SHALL leave it byte-identical. It SHALL NOT touch the
repository's own top-level `openspec/` directory.

#### Scenario: The demo loads with no problems

- **WHEN** `from_files` is called on `docs/demo` with `ArchivedScope::Full`
- **THEN** the returned set has no set-level problem, and no change carries a problem
- **AND** it holds at least two active changes and at least one archived change

#### Scenario: The demo's schema is removed

- **WHEN** `docs/demo/openspec/schemas/` is deleted, so the demo's configured schema cannot be
  resolved without a CLI
- **THEN** the demo check fails, quoting the problem `from_files` recorded for the affected
  change, rather than passing because the change still enumerates

#### Scenario: A demo change loses its tasks file

- **WHEN** one active demo change's tasks artifact is deleted
- **THEN** the demo check fails, naming that change and quoting its problem

#### Scenario: The demo has no partial progress to show

- **WHEN** every active demo change is either untouched (`done == 0`) or complete
- **THEN** the demo check fails, saying that no change shows partial progress

#### Scenario: The fixture is left untouched

- **WHEN** the demo check has run
- **THEN** no file under `docs/demo/` has been created, modified, or removed
- **AND** no file under the repository's own top-level `openspec/` has been created, modified,
  or removed

### Requirement: Deferred work survives the removal of the handoff records

Before `HANDOFF.md`, `HANDOFF-AUDIT.md`, and `NIGHT.md` are deleted, every item they record as
deferred and still open SHALL be moved to `docs/known-issues.md`, with enough detail to act on
it without the deleted file. Each entry SHALL carry its statement, its evidence (file paths,
test names, or the measured failure), and the reason it was deferred. An item that is verified
to be already resolved SHALL NOT be carried over.

`docs/known-issues.md` is a living list: an entry is removed when its issue is fixed. So this
requirement is verified once, when the change is reviewed, and is not bound by a standing
`cargo test` check. A check that pinned the entries would fail on the commit that fixes one.

#### Scenario: The audit's three deferred items are carried over

- **WHEN** `HANDOFF-AUDIT.md` → "Three deferred items" lists the display-width primitives'
  placement in `src/ui/layout.rs`, the two `scripts/gates/nosleep.sh` gaps, and the three
  load-sensitive `ui::tests::wiring` tests, and each is verified still open at this change's
  base commit
- **THEN** `docs/known-issues.md` holds one entry per item, naming
  `columns`/`truncate_columns`, `nosleep.sh` legs 1 and 2b, and the three test names
- **AND** `HANDOFF-AUDIT.md` no longer exists in the working tree

#### Scenario: A resolved item is not resurrected

- **WHEN** a deferred item recorded in a handoff file is verified fixed at this change's base
  commit
- **THEN** it has no entry in `docs/known-issues.md`
