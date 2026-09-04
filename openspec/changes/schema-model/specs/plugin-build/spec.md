## REMOVED Requirements

### Requirement: The crate produces one binary from a declared dependency set

**Reason**: The dependency set is no longer one crate, and the count is written into a
scenario **heading** — "The declared dependency set is exactly one crate". A `MODIFIED`
block cannot rename a scenario: `openspec validate --strict` rejects a MODIFIED
requirement that omits any scenario name the live spec still carries, because archive
replaces the whole block and refuses to drop one. Keeping the heading would leave a
permanent live spec whose scenario is named "exactly one crate" while its own THEN
asserts two, which is the stale-spec-text defect this repository's reviews exist to
catch. The requirement is therefore replaced whole, under a name that carries no count
and so never needs this treatment again.

**Migration**: None for any consumer — nothing outside this repository reads these specs,
and no code, manifest, or command changes because of the rename. The replacement below
carries every clause and every scenario of the removed requirement: the binary-target
scenario is verbatim, the build-graph and genuinely-needed scenarios gain `yaml-rust2`,
the dependency-set scenario is the same check with the count corrected, and one scenario
is added for the MSRV clause the removed requirement stated in prose and never verified.

## ADDED Requirements

### Requirement: The crate produces one binary from an argued dependency set

The Cargo package SHALL be named `herdr-openspec` and SHALL produce exactly one target
of kind `bin`, also named `herdr-openspec`, so a release build lands at
`target/release/herdr-openspec`. The crate SHALL declare edition 2024.

The crate's direct third-party dependencies SHALL be exactly those a change has argued
in its `design.md`, and SHALL be declared with `default-features = false` and an
explicit feature list, so that a future change to a crate's defaults is a reviewable
diff rather than a silent addition to what is built. After this change that set is:

| Crate | Version | Features |
|---|---|---|
| `toml` | at least `1.1.5` | `std`, `parse`, `display`, `serde`; defaults off |
| `yaml-rust2` | at least `0.12.0` | none; defaults off — the default `encoding` feature exists only for `load_from_bytes` BOM and UTF-16 detection, and this crate reads YAML through `std::fs::read_to_string` |

Each declared version's own `rust-version` SHALL be no higher than this crate's
`rust-version`, so the declared MSRV stays true. No dependency, direct or transitive,
SHALL introduce a proc-macro crate into the normal build graph: the resolved graph is
what determines build time and audit surface, and it is the property that changes when a
feature is added or a default is re-enabled. `Cargo.lock` SHALL be committed, and the
change that introduces or alters a dependency SHALL verify `cargo build --locked`
succeeds at the commit that lands it.

#### Scenario: Exactly one binary target is produced at the release path

- **WHEN** `cargo metadata --no-deps --format-version 1` is inspected for targets whose
  `kind` contains `bin`
- **THEN** exactly one such target exists and it is named `herdr-openspec`
- **AND** after `/bin/sh scripts/build.sh` has run, `target/release/herdr-openspec`
  exists and is executable

#### Scenario: The declared dependency set is exactly the argued crates

- **WHEN** `cargo metadata --no-deps --format-version 1` is inspected for the package's
  dependencies of kind `null` (normal, not dev or build)
- **THEN** there are exactly two, named `toml` and `yaml-rust2`
- **AND** both report `uses_default_features` as `false` — read from the resolved
  metadata, not from the text of `Cargo.toml`
- **AND** `toml`'s features are exactly `display`, `parse`, `serde`, and `std`, and
  `yaml-rust2`'s feature list is empty
- **AND** `cargo build --locked` succeeds with `Cargo.lock` committed, so the resolved
  versions in a fresh checkout are the ones this change verified

#### Scenario: Every package in the normal build graph declares an MSRV no higher than the crate's

- **WHEN** the package names `cargo tree -e normal` reports are intersected with the
  `rust_version` each carries in `cargo metadata --format-version 1`
- **THEN** none exceeds this crate's declared `rust-version` of `1.85`, and `yaml-rust2`'s
  own `1.85.0` is the tightest of them
- **AND** the intersection with `cargo tree -e normal` is load-bearing: `cargo metadata`
  alone also reports optional and dev-only resolutions cargo never builds — `syn`,
  `serde_derive`, and `indexmap` among them — so a check over every metadata package
  would report on crates that are not in the build
- **AND** the check reads resolved metadata rather than a crates.io listing, so a future
  version bump that raises an MSRV fails here rather than in a contributor's build

#### Scenario: The resolved build graph is small and proc-macro-free

- **WHEN** `cargo tree -e normal` is inspected
- **THEN** the packages it names, besides `herdr-openspec` itself, are exactly `toml`,
  `serde_core`, `serde_spanned`, `toml_datetime`, `toml_parser`, `toml_writer`, `winnow`,
  `yaml-rust2`, `arraydeque`, `hashlink`, `hashbrown`, and `foldhash`
- **AND** it names no `syn`, `quote`, `proc-macro2`, or `serde_derive`, so nothing in
  the build graph runs a proc macro
- **AND** it names no `encoding_rs`, which `yaml-rust2`'s default features would have
  pulled in, so `default-features = false` is observably in effect rather than merely
  written down
- **AND** `Cargo.lock` holds more entries than that — optional resolutions cargo never
  builds — which is why no check counts lock entries

#### Scenario: Each dependency is genuinely needed rather than incidental

- **WHEN** the `toml` dependency is removed from `Cargo.toml`
- **THEN** `cargo build` fails, because `config` and `state` parse and emit TOML through
  it
- **AND** when `yaml-rust2` is removed instead, `cargo build` fails, because `schema`
  parses `config.yaml`, `.openspec.yaml`, and `schema.yaml` through it
- **AND** with both dependencies and the committed lock restored, `cargo build --locked`
  and `cargo test --all-features` are green again
