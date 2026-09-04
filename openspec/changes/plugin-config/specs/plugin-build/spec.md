## ADDED Requirements

### Requirement: The crate produces one binary from a declared dependency set

The Cargo package SHALL be named `herdr-openspec` and SHALL produce exactly one target
of kind `bin`, also named `herdr-openspec`, so a release build lands at
`target/release/herdr-openspec`. The crate SHALL declare edition 2024.

The crate's direct third-party dependencies SHALL be exactly those a change has argued
in its `design.md`, and SHALL be declared with `default-features = false` and an
explicit feature list, so that what is compiled is what was argued rather than whatever
a crate enables by default. After this change that set is exactly one crate:

| Crate | Version | Features |
|---|---|---|
| `toml` | `1.1.5` | `std`, `parse`, `display`, `serde`; defaults off |

`Cargo.lock` SHALL be committed and `cargo build --locked` SHALL succeed, so a
dependency cannot change without a reviewable diff.

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
- **AND** `Cargo.toml` declares it with `default-features = false` and the feature list
  `["std", "parse", "display", "serde"]`
- **AND** `cargo build --locked` succeeds with `Cargo.lock` committed, so the resolved
  versions in a fresh checkout are the ones this change verified

#### Scenario: The dependency is genuinely needed rather than incidental

- **WHEN** the `toml` dependency is removed from `Cargo.toml`
- **THEN** `cargo build` fails, because `config` and `state` parse and emit TOML through
  it
- **AND** with the dependency restored, `cargo test --all-features` is green again

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
set", which keeps both binary-target scenarios verbatim and replaces the no-dependency
clause with an enumerated, feature-pinned dependency table plus a committed `Cargo.lock`.
Each future dependency amends that table in the change that argues it, so the addition
stays a reviewable diff rather than an unchecked default.
