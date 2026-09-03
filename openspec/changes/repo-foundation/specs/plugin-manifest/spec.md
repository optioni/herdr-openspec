## ADDED Requirements

### Requirement: A minimal manifest Herdr can link from the working tree

`herdr-plugin.toml` SHALL exist at the repository root and SHALL declare
`id = "herdr-openspec"`, `name = "OpenSpec"`, `version = "0.1.0"`,
`min_herdr_version = "0.7.0"`, and `platforms = ["macos", "linux"]`. `version`
is a Herdr-required top-level manifest key, independent of the crate's own
`Cargo.toml` version, and is kept equal to it by convention rather than by any
enforced link. It SHALL declare exactly one `[[build]]` step with
`command = ["/bin/sh", "scripts/build.sh"]`, and exactly one `[[panes]]` entry with
`id = "dashboard"`, `title = "OpenSpec"`, `placement = "split"`, and
`command = ["./target/release/herdr-openspec", "ui"]`. Every key SHALL match the
manifest format documented in `SPEC.md` → Herdr integration → Manifest.

#### Scenario: Herdr links the working tree

- **WHEN** `herdr plugin link .` is run from the repository root with Herdr 0.7.0 or later
- **THEN** the command exits 0 and runs the `[[build]]` step
- **AND** `herdr plugin list` includes `herdr-openspec`

#### Scenario: The dashboard pane launches the binary and stays open

- **WHEN** the `dashboard` pane is opened from Herdr after a successful link
- **THEN** a split pane titled `OpenSpec` opens running
  `./target/release/herdr-openspec ui`
- **AND** it shows the placeholder banner and remains open rather than exiting
  immediately

#### Scenario: The manifest parses and its declared paths resolve

- **WHEN** `herdr-plugin.toml` is parsed as TOML after the build step has run
- **THEN** parsing succeeds and the seven required keys above are present with exactly
  those values
- **AND** every path named by a `command` array — `scripts/build.sh` and
  `target/release/herdr-openspec` — exists and is executable, resolved relative to the
  repository root

### Requirement: Actions and the tab pane are deliberately absent

The manifest at this change SHALL declare no `[[actions]]` entries and no
`dashboard-tab` pane. Those are added by `plugin-actions`, which needs the `open`
subcommand that does not exist yet; declaring them now would give Herdr menu entries
that exit 2 when invoked.

#### Scenario: No action or tab-pane entry is declared

- **WHEN** `herdr-plugin.toml` is read at this change's HEAD
- **THEN** it contains no `[[actions]]` table and no `[[panes]]` entry with
  `id = "dashboard-tab"`
- **AND** Herdr's action menu offers no `OpenSpec:` entry after linking

### Requirement: The manifest path and the Cargo binary name agree

The binary path `./target/release/herdr-openspec` in `herdr-plugin.toml` SHALL agree
with the Cargo package's binary target name and with the artifact `scripts/build.sh`
produces, so that renaming one site without the others is caught here rather than
surfacing later as a pane that fails to start.

#### Scenario: The manifest path and the Cargo binary name agree

- **WHEN** the file name component of the `[[panes]]` command path is compared with the
  name of the single `bin` target reported by
  `cargo metadata --no-deps --format-version 1`
- **THEN** both are `herdr-openspec`
- **AND** the file at that path exists and is executable after
  `/bin/sh scripts/build.sh` has run
