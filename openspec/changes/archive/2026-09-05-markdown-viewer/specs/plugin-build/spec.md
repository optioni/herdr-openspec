## MODIFIED Requirements

### Requirement: The crate produces one binary from an argued dependency set

The Cargo package SHALL be named `herdr-openspec` and SHALL produce exactly one target of
kind `bin`, also named `herdr-openspec`, so a release build lands at
`target/release/herdr-openspec`. The crate SHALL declare edition 2024.

The crate's direct third-party dependencies SHALL be exactly those a change has argued in
its `design.md`, and SHALL be declared with `default-features = false` and an explicit
feature list, so that a future change to a crate's defaults is a reviewable diff rather than
a silent addition to what is built. After this change that set is:

| Crate | Version | Features |
|---|---|---|
| `toml` | at least `1.1.5` | `std`, `parse`, `display`, `serde`; defaults off |
| `yaml-rust2` | at least `0.12.0` | `features = []`, written explicitly; defaults off — the default `encoding` feature exists only for `load_from_bytes` BOM and UTF-16 detection, and this crate reads YAML through `std::fs::read_to_string` |
| `serde_json` | at least `1.0.151` | `std`; defaults off — the default set is exactly `std`, so turning defaults off and naming it changes nothing that is built and everything about whether a future default is adopted silently. The crate is used through `serde_json::Value` only, with no `serde::Deserialize` derive, so no proc-macro crate enters the graph |
| `ratatui` | at least `0.30.2` | `crossterm`; defaults off — the default set additionally carries `all-widgets`, `macros`, and `layout-cache`, none of which this plugin uses, and `all-widgets` alone would pull `time` into the build for the calendar widget. `underline-color` is nominally dropped too but arrives anyway, because `ratatui` declares `ratatui-crossterm` with **its** defaults on and those include it — so a rendered `Cell`'s `Style` still carries an `underline_color` field, which is why an untouched cell's style is `fg(Reset).bg(Reset).underline_color(Reset)` rather than `Style::default()`. The `crossterm` feature also re-enables `std` transitively, so `default-features = false` is narrower on its face than in effect |
| `pulldown-cmark` | at least `0.13.4` | `features = []`, written explicitly; defaults off — the default set is `getopts` and `html`, neither of which this plugin uses. `getopts` exists for the crate's own example binary and `html` pulls `pulldown-cmark-escape` for an HTML renderer this plugin has no surface for: `ui::markdown` walks the event stream itself. Turning both off costs nothing the plugin uses and keeps two packages out of the graph |

`crossterm` SHALL NOT be declared as a direct dependency. It is reached through
`ratatui::crossterm`, the re-export of the exact `crossterm` version `ratatui-crossterm`
resolved, so a backend type and an event type from two different `crossterm` releases can
never coexist in this crate.

`pulldown_cmark` SHALL be named in `src/ui/markdown.rs` and nowhere else in the crate, so
the markdown parser is replaceable by editing one file. That confinement is stated and
checked by `markdown-render`; it is recorded here because it is the reason the dependency's
surface is one module wide.

Each declared version's own `rust-version` SHALL be no higher than this crate's
`rust-version`, so the declared MSRV stays true. This crate's `rust-version` SHALL be
`1.88`, raised from `1.85` by `ratatui` 0.30.2 and its three sibling crates, which remain
the tightest packages in the graph — `pulldown-cmark` declares `1.71.1` and `unicase`
declares none, so adding markdown support does not move the floor.

The normal build graph SHALL contain no proc-macro crate other than those on this enumerated
allowlist: `darling_macro`, `derive_more-impl`, `document-features`, `indoc`, `instability`,
`rustversion`, `strum_macros`, and `thiserror-impl`. Every one arrives through `ratatui`,
which cannot be built without them; `pulldown-cmark` adds none. The rule is an allowlist
rather than an absolute prohibition because that prohibition is unsatisfiable with any
terminal-UI crate in this ecosystem; an allowlist keeps the property mechanical, so a
proc-macro crate arriving from a *new* direct dependency still fails the check and still has
to be argued.

The resolved normal build graph SHALL be pinned by a committed snapshot,
`tests/fixtures/build-graph.txt`, holding one `<name> <version>` line per package, sorted,
for each of the four supported triples, with the per-triple sets recorded separately — they
are not identical, because `rustix`, reached through `crossterm`, pulls `linux-raw-sys` on
Linux and not on macOS. Enumerating the graph in prose was workable at sixteen packages and
is not at roughly eighty; a committed snapshot is exactly as brittle as the committed
`Cargo.lock` that determines it, and every graph change becomes a reviewable diff. This
change's addition to that snapshot SHALL be exactly two packages per triple,
`pulldown-cmark` and `unicase`.

`Cargo.lock` SHALL be committed, and the change that introduces or alters a dependency SHALL
verify `cargo build --locked` succeeds at the commit that lands it.

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
- **THEN** there are exactly five, named `pulldown-cmark`, `ratatui`, `serde_json`, `toml`,
  and `yaml-rust2`
- **AND** all five report `uses_default_features` as `false` — read from the resolved
  metadata, not from the text of `Cargo.toml`
- **AND** `toml`'s features are exactly `display`, `parse`, `serde`, and `std`,
  `serde_json`'s are exactly `std`, `ratatui`'s are exactly `crossterm`, and `yaml-rust2`'s
  and `pulldown-cmark`'s feature lists are both empty
- **AND** the `features` key is written out as `features = []` in the manifest, for both
  `yaml-rust2` and `pulldown-cmark`, rather than omitted. `cargo metadata` reports `[]` for
  both spellings and so cannot tell them apart; the requirement's own rationale — a
  reviewable diff rather than a silent addition — is about what the manifest says on its
  face, so the check for this clause is reading `Cargo.toml`
- **AND** `crossterm` is **not** among the declared dependencies, while `crossterm` **is**
  present in the resolved graph, so the re-export rule is observably in effect rather than
  merely written down
- **AND** `cargo build --locked` succeeds with `Cargo.lock` committed, so the resolved
  versions in a fresh checkout are the ones this change verified

#### Scenario: Every package in the normal build graph declares an MSRV no higher than the crate's

- **WHEN** the `(name, version)` pairs `cargo tree -e normal` reports, over the same four
  supported triples the build-graph scenario names, are intersected with the `rust_version`
  each carries in `cargo metadata --format-version 1`. Matching on the pair rather than on
  the name alone matters because a dev- or build-dependency can resolve a second version of
  a normal-graph package, whose MSRV would otherwise be checked as if it were in the build
- **THEN** none exceeds this crate's declared `rust-version`, read from `Cargo.toml` rather
  than written into the check as a second copy — a check carrying its own literal floor
  keeps enforcing the old value when the crate's `rust-version` moves, and the requirement
  is a claim about the relationship between the two
- **AND** the packages sitting exactly at the floor are **reported rather than asserted** —
  the check prints the set it found and requires only that it is non-empty, because a
  hard-coded set rots the moment a dependency bumps its own `rust-version`. On the resolution
  this change lands that set includes `ratatui`, `ratatui-core`, `ratatui-crossterm`, and
  `ratatui-widgets`, and also `darling`, `darling_core`, `darling_macro`, `instability`, and
  `herdr-openspec` itself. The previous wording named only the four `ratatui` crates and read
  as an exhaustive list, which it never was; it is corrected here. `pulldown-cmark` declares
  `1.71.1` and is therefore well below the floor, and `unicase` declares no `rust-version` at
  all and is skipped, so the crate's floor stays at `1.88`
- **AND** the intersection with `cargo tree -e normal` is load-bearing: `cargo metadata`
  alone also reports optional and dev-only resolutions cargo never builds, so a check over
  every metadata package would report on crates that are not in the build
- **AND** the check reads resolved metadata rather than a crates.io listing, so a future
  version bump that raises an MSRV fails here rather than in a contributor's build

#### Scenario: The resolved build graph is small and proc-macro-free

The title is kept verbatim from the requirement this block replaces, because a MODIFIED
requirement must carry every scenario name the live spec has. Its content is what changed:
the snapshot gains exactly two packages, and two further named absences prove
`pulldown-cmark`'s defaults are off.

- **WHEN** `cargo tree -e normal --target <triple>` is run once for each of the four
  supported triples — `aarch64-apple-darwin`, `x86_64-apple-darwin`,
  `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-gnu` — rather than once for the host,
  because `cargo tree` resolves only the host target by default and a transitive dependency
  scoped to the other supported platform would be invisible on either runner alone. Not
  `--target all`, which reports optional resolutions cargo never builds
- **THEN** each triple's sorted `<name> <version>` set equals the set recorded for it in
  `tests/fixtures/build-graph.txt`, and the check fails when the fixture is missing or names
  a triple the run did not produce
- **AND** the regenerated fixture differs from its predecessor by exactly two added lines
  per triple, `pulldown-cmark` and `unicase`, and by nothing else — a diff a reviewer reads
  rather than a wholesale rewrite
- **AND** the two macOS sets are equal to each other and the two Linux sets are equal to
  each other, while macOS and Linux differ by exactly one package, `linux-raw-sys`, which
  `rustix` pulls in only on Linux — asserted as a named difference rather than the sets
  being asserted identical, which they are not
- **AND** the proc-macro crates present are exactly the eight on the allowlist:
  `darling_macro`, `derive_more-impl`, `document-features`, `indoc`, `instability`,
  `rustversion`, `strum_macros`, and `thiserror-impl` — a set equality, so a ninth arriving
  from a future dependency fails and an eighth disappearing does too
- **AND** it names no `encoding_rs`, which `yaml-rust2`'s default features would have pulled
  in, no `time`, which `ratatui`'s default `all-widgets` feature would have pulled in for
  the calendar widget, and neither `getopts` nor `pulldown-cmark-escape`, which
  `pulldown-cmark`'s own default features would have pulled in — so every
  `default-features = false` in the manifest is observably in effect rather than merely
  written down

#### Scenario: Each dependency is genuinely needed rather than incidental

- **WHEN** the crate's sources, `Cargo.toml`, and `Cargo.lock` are copied to a throwaway
  directory and the `toml` dependency is removed from the copy's manifest
- **THEN** `cargo build` in the copy fails, because `config` and `state` parse and emit TOML
  through it
- **AND** with the manifest restored in the copy and `yaml-rust2` removed instead,
  `cargo build` fails again, because `schema` parses `config.yaml`, `.openspec.yaml`, and
  `schema.yaml` through it
- **AND** with the manifest restored again and `serde_json` removed instead, `cargo build`
  fails a third time, because `changes::from_cli` parses `openspec list --json`,
  `openspec instructions apply --json`, and `openspec schema which --json` through it
- **AND** with the manifest restored again and `ratatui` removed instead, `cargo build`
  fails a fourth time, because `ui` renders every frame and reads every key through it and
  through its `crossterm` re-export
- **AND** with the manifest restored again and `pulldown-cmark` removed instead,
  `cargo build` fails a fifth time, because `ui::markdown` parses every artifact's markdown
  through it
- **AND** removing a dependency that is not declared is reported as a failure of the check
  rather than counted as a pass, so no leg can silently succeed against a manifest that
  never carried the crate
- **AND** the working tree is byte-identical afterwards, `Cargo.lock` included. The
  experiment is run in a copy rather than in place because `cargo build` **rewrites
  `Cargo.lock` during resolution**, before it reaches the compile error the check waits for
  — removing `toml` deletes fourteen package blocks — so an edit-and-restore of `Cargo.toml`
  alone would leave the tree unbuildable under `--locked` and silently discard the
  resolution this change verified
- **AND** in the working tree itself, `cargo build --locked` and `cargo test --all-features`
  are green, which is where the "committed lock still resolves" half of the claim is made

#### Scenario: No JSON parsing reaches the subprocess seam

- **WHEN** `src/cli.rs` is searched for `serde_json`
- **THEN** there is no match, so the seam still returns stdout verbatim and every parse
  lives on the testable side of it
- **AND** the check fails when `src/cli.rs` is absent, and it is paired with a positive
  control asserting that `src/changes.rs` **does** name `serde_json`, so a search that found
  nothing because it searched nothing fails instead of passing
