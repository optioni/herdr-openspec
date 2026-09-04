## ADDED Requirements

### Requirement: The crate produces one binary from a declared dependency set

The Cargo package SHALL be named `herdr-openspec` and SHALL produce exactly one target
of kind `bin`, also named `herdr-openspec`, so a release build lands at
`target/release/herdr-openspec`. The crate SHALL declare edition 2024.

The crate's direct third-party dependencies SHALL be exactly those a change has argued
in its `design.md`, and SHALL be declared with `default-features = false` and an
explicit feature list, so that a future change to a crate's defaults is a reviewable
diff rather than a silent addition to what is built. After this change that set is
exactly one crate:

| Crate | Version | Features |
|---|---|---|
| `toml` | at least `1.1.5` | `std`, `parse`, `display`, `serde`; defaults off |

The declared version's own `rust-version` SHALL be no higher than this crate's
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

#### Scenario: The declared dependency set is exactly one crate

- **WHEN** `cargo metadata --no-deps --format-version 1` is inspected for the package's
  dependencies of kind `null` (normal, not dev or build)
- **THEN** there is exactly one, named `toml`
- **AND** that dependency's `uses_default_features` is `false` and its `features` are
  exactly `display`, `parse`, `serde`, and `std` — read from the resolved metadata, not
  from the text of `Cargo.toml`
- **AND** `cargo build --locked` succeeds with `Cargo.lock` committed, so the resolved
  versions in a fresh checkout are the ones this change verified

#### Scenario: The resolved build graph is small and proc-macro-free

- **WHEN** `cargo tree -e normal` is inspected
- **THEN** the packages it names, besides `herdr-openspec` itself, are exactly `toml`,
  `serde_core`, `serde_spanned`, `toml_datetime`, `toml_parser`, `toml_writer`, and
  `winnow`
- **AND** it names no `syn`, `quote`, `proc-macro2`, or `serde_derive`, so nothing in
  the build graph runs a proc macro
- **AND** `Cargo.lock` holds more entries than that — optional resolutions cargo never
  builds — which is why no check counts lock entries

#### Scenario: The dependency is genuinely needed rather than incidental

- **WHEN** the `toml` dependency is removed from `Cargo.toml`
- **THEN** `cargo build` fails, because `config` and `state` parse and emit TOML through
  it
- **AND** with the dependency and the committed lock restored, `cargo build --locked`
  and `cargo test --all-features` are green again

## REMOVED Requirements

### Requirement: The crate produces one binary, with no third-party dependencies

**Reason**: The clause "SHALL add no third-party dependency: `Cargo.lock` therefore
contains exactly one package" described the scaffold, not a design rule. `SPEC.md` →
Overview names `ratatui`, `crossterm`, `notify`, `serde_json`, `serde_yaml`, and
`pulldown-cmark` as the intended stack, so the dependency-free property was always
temporary; `plugin-config` is simply the first change to spend it, on a TOML parser.
Keeping the requirement would mean either hand-rolling a TOML reader that silently
misreads valid configuration a user wrote, or leaving a live requirement that the
repository contradicts.

**Migration**: Replaced by "The crate produces one binary from a declared dependency
set", which keeps the binary-target scenario verbatim and replaces the no-dependency
scenario with three: an enumerated, feature-pinned dependency table read from resolved
metadata, a `cargo tree -e normal` assertion on the whole build graph, and a check that
removing the dependency breaks the build. The `cargo build --release --offline`
secondary signal is retired: a registry dependency makes it environment-dependent, so it
would report the state of the local cargo cache rather than of the crate. Each future
dependency amends the table in the change that argues it, so the addition stays a
reviewable diff rather than an unchecked default.
