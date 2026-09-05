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

### Requirement: The `ui` invocation runs the dashboard and needs a terminal

The binary SHALL accept `ui` as its only argument. On `ui` it SHALL run the dashboard
through `ui::run`, which reads no arguments of its own. Argument classification SHALL
remain a pure function in the library, with `src/main.rs` restricted to reading arguments,
dispatching, writing stderr, and setting the exit status.

Exit statuses SHALL be distinct and stable:

| Outcome | Status | Streams |
|---|---|---|
| The dashboard ran and the user quit | 0 | whatever the alternate screen held, which is discarded on leaving it |
| An unrecognised argument, no argument, or an argument after `ui` | 2 | usage on stderr, stdout empty |
| `ui` with stdout not a terminal | 3 | a message on stderr naming `herdr-openspec` and the words `not a terminal`, stdout empty |
| `ui` failed to start for any other reason | 1 | the error's `Display` text on stderr, stdout empty |

On status 3 the process SHALL exit without waiting on stdin. The previous behaviour blocked
until stdin reached EOF, so a test or script that piped the binary's output would hang
rather than fail; this change makes that case terminate.

#### Scenario: `ui` with stdout piped exits 3 without blocking

- **WHEN** the built binary is run as `herdr-openspec ui` with stdout and stderr captured
  through pipes and with stdin attached to a pipe whose write end is deliberately left
  **open**, so a wrongly blocking implementation would hang
- **THEN** it exits with status 3, polled to a ten-second deadline that fails the test if
  the process is still alive when it expires
- **AND** stdout is empty
- **AND** stderr contains `herdr-openspec` and the words `not a terminal`

#### Scenario: The three failing statuses are distinct

- **WHEN** the built binary is run four ways with stdout piped and stdin at EOF: `ui`,
  `wat`, `ui --tab`, and with no arguments
- **THEN** the exit statuses are 3, 2, 2, and 2 respectively
- **AND** the three status-2 runs still print the usage text listing `ui`, and the `wat`
  and `ui --tab` runs still name the rejected token, unchanged by this change
- **AND** no run prints anything to stdout

#### Scenario: Every binary-integration run pipes stdout

- **WHEN** `tests/cli.rs` is inspected for every place it spawns the crate's own binary
- **THEN** each one sets stdout to a pipe or to null, never inheriting the test process's
  own terminal, so the status-3 branch is the branch the suite reaches
- **AND** the tree-wide containment of the crossterm terminal-mode functions is
  `terminal-lifecycle`'s requirement, not restated here — one check, one owner

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
| `notify` | at least `8.2.0` | `macos_fsevent`; defaults off — `notify`'s own default set is exactly `["macos_fsevent"]`, so this declaration resolves a graph **byte-identical** to a bare `notify = "8.2.0"` and changes only whether a future default is adopted silently. `default-features = false` **alone** does not compile on macOS: `notify`'s `fsevent` module is gated on `not(feature = "macos_kqueue")` rather than on the presence of `macos_fsevent`, so an empty feature list yields `error[E0432]: unresolved import fsevent_sys`. Exactly one of the two macOS backends must therefore be named, and FSEvents is chosen over kqueue because it recurses in the kernel — one watch for the whole `openspec/` subtree — while `notify`'s kqueue backend opens a file descriptor per entry and reports only that *something* in a directory changed. `crossbeam-channel`, `flume`, `serde`, and `serialization-compat-6` all stay off: `std::sync::mpsc::Sender<notify::Result<Event>>` implements `notify::EventHandler` unconditionally, with no feature flag. On Linux `inotify` and `mio` are unconditional target dependencies and need no feature at all |

`crossterm` SHALL NOT be declared as a direct dependency. It is reached through
`ratatui::crossterm`, the re-export of the exact `crossterm` version `ratatui-crossterm`
resolved, so a backend type and an event type from two different `crossterm` releases can
never coexist in this crate.

`pulldown_cmark` SHALL be named in `src/ui/markdown.rs` and nowhere else in the crate, so
the markdown parser is replaceable by editing one file. That confinement is stated and
checked by `markdown-render`; it is recorded here because it is the reason the dependency's
surface is one module wide. `notify` SHALL be named in `src/watch.rs` and nowhere else, on
the identical terms, checked by `watch-invalidation`.

No debouncing crate SHALL be declared. `notify-debouncer-mini` 0.7.0 — whose entire job is
the coalescing this change needs — was measured at **five further packages** beyond `notify`,
`tempfile`, `fastrand`, `getrandom`, and `once_cell` among them, as **normal** dependencies
of the shipped binary; `notify-debouncer-full` 0.7.0 costs two further packages (`file-id`,
for rename tracking this plugin does not do), raises a second crate's release cadence onto
the maintenance burden, and declares an MSRV of 1.85. The hand-rolled `watch::Debounce` is a
pure state machine taking `now` as a parameter, which is additionally what makes the window
testable without sleeping — a property neither crate offers.

Each declared version's own `rust-version` SHALL be no higher than this crate's
`rust-version`, so the declared MSRV stays true. This crate's `rust-version` SHALL be
`1.88`, raised from `1.85` by `ratatui` 0.30.2 and its three sibling crates, which remain
the tightest packages in the graph. `notify` 8.2.0 declares `1.77` and its transitive
`notify-types` 2.1.0 declares `1.85`, so this change does not move the floor and adds nothing
*at* it. (`notify` 9.0.0-rc.5 declares exactly `1.88`; it is a pre-release, `cargo add`
resolves 8.2.0, and adopting it later would put a package at the floor for the first time
outside the `ratatui` family.)

The normal build graph SHALL contain no proc-macro crate other than those on this enumerated
allowlist: `darling_macro`, `derive_more-impl`, `document-features`, `indoc`, `instability`,
`rustversion`, `strum_macros`, and `thiserror-impl`. Every one arrives through `ratatui`,
which cannot be built without them; `pulldown-cmark` adds none and `notify` adds none. The
rule is an allowlist rather than an absolute prohibition because that prohibition is
unsatisfiable with any terminal-UI crate in this ecosystem; an allowlist keeps the property
mechanical, so a proc-macro crate arriving from a *new* direct dependency still fails the
check and still has to be argued.

The resolved normal build graph SHALL be pinned by a committed snapshot,
`tests/fixtures/build-graph.txt`, holding one `<name> <version>` line per package, sorted,
for each of the four supported triples, with the per-triple sets recorded separately — they
are not identical. Enumerating the graph in prose was workable at sixteen packages and
is not at roughly eighty; a committed snapshot is exactly as brittle as the committed
`Cargo.lock` that determines it, and every graph change becomes a reviewable diff. This
change's addition to that snapshot SHALL be exactly **five** packages per macOS triple —
`fsevent-sys`, `notify`, `notify-types`, `same-file`, and `walkdir` — and exactly **six** per
Linux triple, the same five with `fsevent-sys` replaced by `inotify` and `inotify-sys`. The
file grows from 310 lines to **332**. `libc`, `log`, `mio`, and `bitflags` are `notify`
dependencies that the baseline graph already carries through `crossterm`, `signal-hook-mio`,
and `ratatui`, so they cost nothing.

The macOS and Linux sets, previously differing by exactly one package, SHALL now differ by
exactly **four**: `fsevent-sys` and `linux-raw-sys` as before plus `inotify` and
`inotify-sys`. The check that asserts this SHALL be edited **on disk** to the new four-name
list rather than left asserting one — its previous form is a hard-coded string comparison
against `linux-raw-sys `, and this change is the first to make that comparison false.

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
- **THEN** there are exactly **six**, named `notify`, `pulldown-cmark`, `ratatui`,
  `serde_json`, `toml`, and `yaml-rust2`
- **AND** all six report `uses_default_features` as `false` — read from the resolved
  metadata, not from the text of `Cargo.toml`
- **AND** `toml`'s features are exactly `display`, `parse`, `serde`, and `std`,
  `serde_json`'s are exactly `std`, `ratatui`'s are exactly `crossterm`, `notify`'s are
  exactly `macos_fsevent`, and `yaml-rust2`'s and `pulldown-cmark`'s feature lists are both
  empty
- **AND** the `features` key is written out as `features = []` in the manifest, for both
  `yaml-rust2` and `pulldown-cmark`, rather than omitted. `cargo metadata` reports `[]` for
  both spellings and so cannot tell them apart; the requirement's own rationale — a
  reviewable diff rather than a silent addition — is about what the manifest says on its
  face, so the check for this clause is reading `Cargo.toml`. `notify` is **not** in that
  loop: its feature list is non-empty, so the resolved metadata already distinguishes it
- **AND** `notify` declares `default-features = false` in the manifest text as well as in
  the resolved metadata, the same two-sided check `pulldown-cmark` carries
- **AND** `crossterm` is **not** among the declared dependencies, while `crossterm` **is**
  present in the resolved graph, so the re-export rule is observably in effect rather than
  merely written down
- **AND** neither `notify-debouncer-mini` nor `notify-debouncer-full` appears anywhere in
  the resolved graph, so the hand-rolled debounce is observably in effect
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
  `herdr-openspec` itself — unchanged by this change, because the highest MSRV `notify`
  brings in is `notify-types`' `1.85`, below the floor
- **AND** the intersection with `cargo tree -e normal` is load-bearing: `cargo metadata`
  alone also reports optional and dev-only resolutions cargo never builds, so a check over
  every metadata package would report on crates that are not in the build
- **AND** the check reads resolved metadata rather than a crates.io listing, so a future
  version bump that raises an MSRV fails here rather than in a contributor's build

#### Scenario: The resolved build graph is small and proc-macro-free

The title is kept verbatim from the requirement this block replaces, because a MODIFIED
requirement must carry every scenario name the live spec has. Its content is what changed:
the snapshot gains five lines per macOS triple and six per Linux triple, and the macOS/Linux
delta becomes four named packages rather than one.

- **WHEN** `cargo tree -e normal --target <triple>` is run once for each of the four
  supported triples — `aarch64-apple-darwin`, `x86_64-apple-darwin`,
  `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-gnu` — rather than once for the host,
  because `cargo tree` resolves only the host target by default and a transitive dependency
  scoped to the other supported platform would be invisible on either runner alone. Not
  `--target all`, which reports optional resolutions cargo never builds
- **THEN** each triple's sorted `<name> <version>` set equals the set recorded for it in
  `tests/fixtures/build-graph.txt`, and the check fails when the fixture is missing or names
  a triple the run did not produce
- **AND** the regenerated fixture differs from its predecessor by exactly five added lines
  per macOS triple — `fsevent-sys`, `notify`, `notify-types`, `same-file`, `walkdir` — and
  exactly six per Linux triple — `inotify`, `inotify-sys`, `notify`, `notify-types`,
  `same-file`, `walkdir` — with nothing removed on any triple, taking the file from 310 lines
  to 332: a diff a reviewer reads rather than a wholesale rewrite
- **AND** the two macOS sets are equal to each other and the two Linux sets are equal to
  each other, while macOS and Linux differ by exactly **four** packages —
  `fsevent-sys`, `inotify`, `inotify-sys`, and `linux-raw-sys` — asserted as a named
  difference rather than the sets being asserted identical, which they are not. The
  previously-asserted single-package difference was `linux-raw-sys` alone, which `rustix`
  pulls in only on Linux; the three new names are `notify`'s own target-scoped backends
- **AND** the proc-macro crates present are exactly the eight on the allowlist:
  `darling_macro`, `derive_more-impl`, `document-features`, `indoc`, `instability`,
  `rustversion`, `strum_macros`, and `thiserror-impl` — a set equality, so a ninth arriving
  from a future dependency fails and an eighth disappearing does too. `notify` adds none,
  which is part of why it was chosen over a debouncer crate
- **AND** it names no `encoding_rs`, which `yaml-rust2`'s default features would have pulled
  in, no `time`, which `ratatui`'s default `all-widgets` feature would have pulled in for
  the calendar widget, neither `getopts` nor `pulldown-cmark-escape`, which
  `pulldown-cmark`'s own default features would have pulled in, and neither `kqueue` nor
  `kqueue-sys`, which `notify`'s `macos_kqueue` feature would have pulled in — so every
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
- **AND** with the manifest restored again and `notify` removed instead, `cargo build` fails
  a sixth time, because `watch::RealFsEvents` is the crate's only source of filesystem change
  events
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
