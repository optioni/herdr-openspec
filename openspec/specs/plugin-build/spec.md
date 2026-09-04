# plugin-build Specification

## Purpose
TBD - created by archiving change repo-foundation. Update Purpose after archive.

## Requirements

### Requirement: The build script produces the release binary

The repository SHALL provide `scripts/build.sh`, runnable from the repository root as
`/bin/sh scripts/build.sh`. The script SHALL build the release binary with
`cargo build --release`. When `cargo` is not resolvable on `PATH`, it SHALL source
`~/.cargo/env` when that file is readable, announce that it did so on stderr, and
retry, because Herdr may launch the build without `~/.cargo/bin` on `PATH`. When
`cargo` is still not resolvable, the script SHALL fail without attempting a build.

#### Scenario: Build succeeds with cargo already on PATH

- **WHEN** `/bin/sh scripts/build.sh` is run from the repository root with `cargo` on `PATH`
- **THEN** the script exits 0
- **AND** `target/release/herdr-openspec` exists and is executable
- **AND** stderr carries no `~/.cargo/env` notice, because the fallback was not needed

#### Scenario: Build succeeds when cargo is reachable only through `~/.cargo/env`

- **WHEN** `/bin/sh scripts/build.sh` is run with a `PATH` on which `command -v cargo`
  is confirmed to fail beforehand, and `~/.cargo/env` present and readable
- **THEN** stderr carries the notice that `~/.cargo/env` was sourced, proving the
  fallback branch ran rather than the first probe succeeding
- **AND** the script exits 0 and `target/release/herdr-openspec` exists and is executable

#### Scenario: Cargo cannot be found at all

- **WHEN** `/bin/sh scripts/build.sh` is run under `env -i` with `HOME` pointed at an
  empty temporary directory and a `PATH` carrying no `cargo`
- **THEN** the script exits with a non-zero status
- **AND** stderr contains a message naming `cargo` and directing the reader to
  `https://rustup.rs`
- **AND** no `cargo build` is attempted: the mtime of a pre-existing
  `target/release/herdr-openspec`, captured immediately before the run, is unchanged

#### Scenario: Script is POSIX shell, not bash

- **WHEN** the script is syntax-checked with `sh -n`, and additionally with `dash -n`
  where `dash` is available
- **THEN** both exit 0 with no syntax error
- **AND** an unanchored search finds no bash-only construct anywhere in the file,
  indented or not — no `[[`, no `function` keyword, no `source` builtin, and no array
  assignment; `.` is used to load `~/.cargo/env`

### Requirement: The `ui` invocation prints a placeholder and holds the pane open

The binary SHALL accept `ui` as its only argument. On `ui` it SHALL write a banner to
stdout naming the plugin id `herdr-openspec`, the crate version, and a line stating the
dashboard is not implemented yet; it SHALL then block until stdin reaches EOF and exit
0, so a Herdr pane running it stays visible instead of closing immediately. Any other
argument list — an unrecognised first argument, no arguments, or `ui` followed by
anything — SHALL print usage on stderr and exit 2, so that the `open` subcommand added
by `plugin-actions` is distinguishable from today's behaviour. Argument classification
SHALL live in the library as a pure function, with `src/main.rs` restricted to reading
arguments, writing streams, blocking on stdin, and setting the exit status.

#### Scenario: `ui` prints the placeholder banner

- **WHEN** the built binary is run as `herdr-openspec ui` with stdin already at EOF
- **THEN** it exits 0
- **AND** stdout contains `herdr-openspec`, the value of `CARGO_PKG_VERSION`, and a line
  stating the dashboard is not implemented yet
- **AND** stderr is empty

#### Scenario: `ui` holds the process open until stdin closes

- **WHEN** the built binary is run as `herdr-openspec ui` with stdin attached to a pipe
  that is left open
- **THEN** the banner is emitted and the process is still running when polled after a
  short interval, rather than having exited
- **AND** when the write end of the pipe is dropped, the process exits 0

#### Scenario: Unknown subcommand

- **WHEN** the built binary is run as `herdr-openspec wat`
- **THEN** it exits with status 2
- **AND** stderr contains usage text listing `ui` and the rejected argument `wat`
- **AND** stdout is empty

#### Scenario: Extra arguments after `ui`

- **WHEN** the built binary is run as `herdr-openspec ui --tab`
- **THEN** it exits with status 2
- **AND** stderr contains the usage text and the rejected argument `--tab`
- **AND** stdout is empty, and the process does not block waiting on stdin

#### Scenario: No subcommand

- **WHEN** the built binary is run with no arguments
- **THEN** it exits with status 2
- **AND** stderr contains the same usage text listing `ui`
- **AND** stdout is empty

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
| `yaml-rust2` | at least `0.12.0` | `features = []`, written explicitly; defaults off — the default `encoding` feature exists only for `load_from_bytes` BOM and UTF-16 detection, and this crate reads YAML through `std::fs::read_to_string` |

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
- **AND** with `target/release/herdr-openspec` deleted first, `/bin/sh scripts/build.sh`
  **exits 0** and the binary then exists and is executable. Both the deletion and the exit
  status are load-bearing: the script has a real failure path (`error: cargo not found`,
  `exit 1`) and a leftover binary from any earlier build satisfies an existence check
  regardless of what the script did

#### Scenario: The declared dependency set is exactly the argued crates

- **WHEN** `cargo metadata --no-deps --format-version 1` is inspected for the package's
  dependencies of kind `null` (normal, not dev or build)
- **THEN** there are exactly two, named `toml` and `yaml-rust2`
- **AND** both report `uses_default_features` as `false` — read from the resolved
  metadata, not from the text of `Cargo.toml`
- **AND** `toml`'s features are exactly `display`, `parse`, `serde`, and `std`, and
  `yaml-rust2`'s feature list is empty
- **AND** the `features` key is written out as `features = []` in the manifest rather than
  omitted. `cargo metadata` reports `[]` for both spellings and so cannot tell them apart;
  the requirement's own rationale — a reviewable diff rather than a silent addition — is
  about what the manifest says on its face, so the check for this clause is reading
  `Cargo.toml`
- **AND** `cargo build --locked` succeeds with `Cargo.lock` committed, so the resolved
  versions in a fresh checkout are the ones this change verified

#### Scenario: Every package in the normal build graph declares an MSRV no higher than the crate's

- **WHEN** the `(name, version)` pairs `cargo tree -e normal` reports, over the same four
  supported triples the build-graph scenario names, are intersected with the `rust_version`
  each carries in `cargo metadata --format-version 1`. Matching on the pair rather than on
  the name alone matters because a dev- or build-dependency can resolve a second version of
  a normal-graph package, whose MSRV would otherwise be checked as if it were in the build
- **THEN** none exceeds this crate's declared `rust-version`, read from `Cargo.toml`
  rather than written into the check as a second copy — a check carrying its own literal
  floor keeps enforcing the old value when the crate's `rust-version` moves, and the
  requirement is a claim about the relationship between the two
- **AND** the packages sitting exactly at the floor are reported rather than assumed: on
  the resolution this change lands, `yaml-rust2`, `hashbrown`, `hashlink`, `toml`,
  `toml_datetime`, `toml_parser`, `toml_writer`, and `serde_spanned` all declare `1.85`,
  so no single crate is uniquely the tightest
- **AND** the intersection with `cargo tree -e normal` is load-bearing: `cargo metadata`
  alone also reports optional and dev-only resolutions cargo never builds — `syn`,
  `serde_derive`, and `indexmap` among them — so a check over every metadata package
  would report on crates that are not in the build
- **AND** the check reads resolved metadata rather than a crates.io listing, so a future
  version bump that raises an MSRV fails here rather than in a contributor's build

#### Scenario: The resolved build graph is small and proc-macro-free

- **WHEN** `cargo tree -e normal --target <triple>` is inspected once for each of the four
  supported triples — `aarch64-apple-darwin`, `x86_64-apple-darwin`,
  `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-gnu` — rather than once for the host,
  because `cargo tree` resolves only the host target by default and a transitive dependency
  scoped to the other supported platform would be invisible on either runner alone. Not
  `--target all`, which reports optional resolutions cargo never builds: on this graph it
  lists `syn`, `quote`, `proc-macro2`, `serde_derive`, and `unicode-ident` through
  `serde_core`'s optional `derive` feature
- **THEN** each of the four produces the same set, and the packages it names, besides
  `herdr-openspec` itself, are exactly `toml`,
  `serde_core`, `serde_spanned`, `toml_datetime`, `toml_parser`, `toml_writer`, `winnow`,
  `yaml-rust2`, `arraydeque`, `hashlink`, `hashbrown`, and `foldhash`
- **AND** it names no `syn`, `quote`, `proc-macro2`, or `serde_derive`, so nothing in
  the build graph runs a proc macro. A dependency scoped to a platform the manifest does
  not declare — Windows — is out of scope by construction and is not searched for
- **AND** it names no `encoding_rs`, which `yaml-rust2`'s default features would have
  pulled in, so `default-features = false` is observably in effect rather than merely
  written down
- **AND** `Cargo.lock` holds more entries than that — optional resolutions cargo never
  builds — which is why no check counts lock entries

#### Scenario: Each dependency is genuinely needed rather than incidental

- **WHEN** the crate's sources, `Cargo.toml`, and `Cargo.lock` are copied to a throwaway
  directory and the `toml` dependency is removed from the copy's manifest
- **THEN** `cargo build` in the copy fails, because `config` and `state` parse and emit TOML
  through it
- **AND** with the manifest restored in the copy and `yaml-rust2` removed instead,
  `cargo build` fails again, because `schema` parses `config.yaml`, `.openspec.yaml`, and
  `schema.yaml` through it
- **AND** removing a dependency that is not declared is reported as a failure of the check
  rather than counted as a pass, so the second leg cannot silently succeed against a
  manifest that never carried the crate
- **AND** the working tree is byte-identical afterwards, `Cargo.lock` included. The
  experiment is run in a copy rather than in place because `cargo build` **rewrites
  `Cargo.lock` during resolution**, before it reaches the compile error the check waits for
  — removing `toml` deletes fourteen package blocks — so an edit-and-restore of `Cargo.toml`
  alone would leave the tree unbuildable under `--locked` and silently discard the
  resolution this change verified
- **AND** in the working tree itself, `cargo build --locked` and `cargo test --all-features`
  are green, which is where the "committed lock still resolves" half of the claim is made
