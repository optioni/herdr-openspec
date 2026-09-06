# plugin-manifest Specification

## Purpose
Pins the contents of `herdr-plugin.toml` — the one contract this crate has with a consumer
outside itself: the required top-level keys, the single `[[build]]` step, the two
`[[panes]]` entries (a split placement and a tab placement, both titled `OpenSpec`), and the
two workspace `[[actions]]` that invoke `open` and `open-tab`. It also holds the agreement
checks that keep the manifest from drifting from its second sites — the Cargo binary name,
the entrypoint/placement literals `open`'s argument builder emits, the subcommand tokens the
binary's argument parser accepts, and the action titles named in `README.md` — deliberately
leaving `version`, `min_herdr_version`, and `platforms` unpinned because they have no second
site.

## Requirements

### Requirement: A minimal manifest Herdr can link from the working tree

`herdr-plugin.toml` SHALL exist at the repository root and SHALL declare
`id = "herdr-openspec"`, `name = "OpenSpec"`, `version = "0.1.0"`,
`min_herdr_version = "0.7.0"`, and `platforms = ["macos", "linux"]`. `version`
is a Herdr-required top-level manifest key, independent of the crate's own
`Cargo.toml` version, and is kept equal to it by convention rather than by any
enforced link. It SHALL declare exactly one `[[build]]` step with
`command = ["/bin/sh", "scripts/build.sh"]`.

It SHALL declare exactly **two** `[[panes]]` entries, both titled `OpenSpec` and both
running `command = ["./target/release/herdr-openspec", "ui"]`:

| `id` | `placement` |
|---|---|
| `dashboard` | `split` |
| `dashboard-tab` | `tab` |

and exactly **two** `[[actions]]` entries, both with `contexts = ["workspace"]`:

| `id` | `title` | `command` |
|---|---|---|
| `open` | `OpenSpec: dashboard` | `["./target/release/herdr-openspec", "open"]` |
| `open-tab` | `OpenSpec: dashboard (tab)` | `["./target/release/herdr-openspec", "open-tab"]` |

`min_herdr_version` SHALL remain `0.7.0`: Herdr's own changelog records manifest-declared
actions, managed plugin panes, plugin pane placement, and plugin invocation context and
environment injection as 0.7.0 additions, so nothing this change declares raises the floor.
Every key SHALL match the manifest format documented in `SPEC.md` → Herdr integration →
Manifest, which this change corrects: the tab action's command is `open-tab`, not
`["open", "--tab"]`.

#### Scenario: Herdr links the working tree

- **WHEN** `herdr plugin link .` is run from the repository root with Herdr 0.7.0 or later
- **THEN** the command exits 0 — `plugin link` does not run `[[build]]` steps; those
  run only during a GitHub-managed `plugin install`, so the working tree must already
  be built (`scripts/build.sh` or `make build`) for the pane to have a binary to run
- **AND** `herdr plugin list` includes `herdr-openspec`
- **AND** `herdr plugin action list --plugin herdr-openspec` reports both `open` and
  `open-tab` with the titles above, proving Herdr accepted the `[[actions]]` table

#### Scenario: The dashboard pane launches the binary and stays open

- **WHEN** the `dashboard` pane is opened from Herdr after a successful link
- **THEN** a split pane titled `OpenSpec` opens running
  `./target/release/herdr-openspec ui`
- **AND** it remains open rather than exiting immediately

#### Scenario: The tab pane opens in its own tab

- **WHEN** the `dashboard-tab` pane is opened from Herdr with `--placement tab`
- **THEN** Herdr accepts `placement = "tab"` and a new tab holding a pane titled
  `OpenSpec` is created in the named workspace
- **AND** the pane runs the same `./target/release/herdr-openspec ui` command as the split
  pane, so the two entries differ only in placement

#### Scenario: The manifest parses and its declared paths resolve

- **WHEN** `herdr-plugin.toml` is parsed as TOML
- **THEN** parsing succeeds and all eight required top-level keys above are present and
  non-empty
- **AND** the two `[[panes]]` and two `[[actions]]` tables are present with exactly the
  ids, titles, placements, contexts, and commands tabulated above, and no others
- **AND** `scripts/build.sh` — the one `command` path that is committed to the repository —
  exists and is executable, resolved relative to the repository root

`target/release/herdr-openspec` is **not** asserted to exist here. `make check` does not run
`make build` and `cargo test` builds the debug profile, so a release-artifact assertion
would be red on every clean checkout and in CI. Its existence is verified after `make build`
by `plugin-build` → "The crate produces one binary from an argued dependency set" and by
this change's live Herdr check.

### Requirement: The manifest path and the Cargo binary name agree

The binary path `./target/release/herdr-openspec` in every `[[panes]]` and `[[actions]]`
`command` in `herdr-plugin.toml` SHALL agree with the Cargo package's binary target name
and with the artifact `scripts/build.sh` produces, so that renaming one site without the
others is caught here rather than surfacing later as a pane or menu entry that fails to
start.

This check SHALL be a Rust test under `tests/`, run by `cargo test` and therefore by
`make check` and by CI, rather than a per-change shell command. Every named gate in this
repository that lives outside `make check` has rotted at least once; the manifest is the
one contract with a consumer outside the crate, so its check is the one that must not.
The test SHALL take the binary name from `env!("CARGO_BIN_EXE_herdr-openspec")`'s file stem
and from `env!("CARGO_PKG_NAME")` rather than by spawning `cargo metadata`, so it depends on
no subprocess and, per `quality-gates` → "The gates do not depend on Herdr", on no Herdr
installation either.

The test SHALL confine itself to assertions with a **second site** — a value that must
agree with something else, whose disagreement is silent. Those are: the four command
basenames against the crate's binary name; both pane `title`s against
`open::DASHBOARD_LABEL`; both pane `id`s against the `--entrypoint` literals
`open::open_args` emits (`open::DASHBOARD_ENTRYPOINT`, `open::DASHBOARD_TAB_ENTRYPOINT`);
both `placement`s against the `--placement` literals it emits; the two action `command`
tails against the two subcommand tokens `parse` accepts; and both action `title`s against
`README.md`. The test SHALL NOT pin `version`, `min_herdr_version`, or `platforms` to
literals: they have no second site, a wrong value is rejected loudly by
`herdr plugin link`, and pinning `version` would make a legitimate bump red.

The same test SHALL assert that `README.md` names both action titles and no longer carries
the sentence deferring them to a later change, so the doc claim and the manifest cannot
drift apart again. Because `OpenSpec: dashboard` is a prefix of
`OpenSpec: dashboard (tab)`, the two SHALL be asserted as distinct whole lines or with the
longer title's presence checked separately, never by two `contains` calls that one string
satisfies.

#### Scenario: The manifest path and the Cargo binary name agree

- **WHEN** the file name component of each `[[panes]]` and `[[actions]]` command path is
  compared with `env!("CARGO_PKG_NAME")` and with the file stem of
  `env!("CARGO_BIN_EXE_herdr-openspec")`
- **THEN** all four are `herdr-openspec`, and the two `env!` values agree with each other
- **AND** the assertion needs no subprocess, no release build, and no Herdr

#### Scenario: A renamed binary fails the gate

- **WHEN** one `[[actions]]` command path is changed to
  `./target/release/herdr-openspec-2` and `cargo test` is run
- **THEN** the manifest test fails, naming the offending entry's id
- **AND** the same happens for a changed `[[panes]]` command, so the check is not blind to
  either table

#### Scenario: The pane title and the pane matcher are the same string

- **WHEN** both `[[panes]]` titles are compared with `open::DASHBOARD_LABEL`, and each
  pane's `id` and `placement` with the `--entrypoint` and `--placement` literals the
  corresponding `open::open_args` vector carries
- **THEN** all of them agree
- **AND** changing either pane's title, id, or placement without changing the constant
  fails the test — the silent failure this pairing exists to catch, because a renamed
  title makes the matcher never match and `open` stacks a new pane on every invocation

#### Scenario: README and the manifest agree on the action titles

- **WHEN** `README.md` is read, its whitespace collapsed, and searched for each
  `[[actions]]` `title`
- **THEN** `OpenSpec: dashboard (tab)` appears, and `OpenSpec: dashboard` appears at least
  once outside that longer title, so the shorter title's assertion cannot be satisfied by
  the longer one alone
- **AND** the sentence `these action-menu entries arrive with a later change` no longer
  appears anywhere in `README.md`
