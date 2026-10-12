## MODIFIED Requirements

### Requirement: A minimal manifest Herdr can link from the working tree

`herdr-plugin.toml` SHALL exist at the repository root and SHALL declare
`id = "herdr-openspec"`, `name = "OpenSpec"`, `version = "0.1.0"`,
`min_herdr_version = "0.7.5"`, and `platforms = ["macos", "linux"]`. `version`
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

exactly **one** `[[startup]]` entry (`dashboard-restore`'s addition), with
`command = ["./target/release/herdr-openspec", "restore"]` and no other key,
and exactly **two** `[[actions]]` entries, both with `contexts = ["workspace"]`:

| `id` | `title` | `command` |
|---|---|---|
| `open` | `OpenSpec: dashboard` | `["./target/release/herdr-openspec", "open"]` |
| `open-tab` | `OpenSpec: dashboard (tab)` | `["./target/release/herdr-openspec", "open-tab"]` |

`min_herdr_version` SHALL be `0.7.5`, raised from `0.7.0` by `dashboard-restore`: Herdr's
own changelog records manifest-declared actions, managed plugin panes, plugin pane
placement, plugin invocation context and environment injection, and `pane.process_info` as
0.7.0 additions, and one-shot plugin `[[startup]]` hooks as a 0.7.5 addition — the newest
key this manifest declares, so it sets the floor.
Every key SHALL match the manifest format documented in `SPEC.md` → Herdr integration →
Manifest, which this change corrects: the tab action's command is `open-tab`, not
`["open", "--tab"]`.

#### Scenario: Herdr links the working tree

- **WHEN** `herdr plugin link .` is run from the repository root with Herdr 0.7.5 or later
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
- **THEN** parsing succeeds and all nine required top-level keys above are present and
  non-empty
- **AND** the two `[[panes]]`, the one `[[startup]]`, and the two `[[actions]]` tables are
  present with exactly the ids, titles, placements, contexts, and commands stated above,
  and no others
- **AND** the `[[startup]]` command's first element names the same path as the panes' and
  actions' commands, and its second element is a token `parse` classifies as
  `Invocation::Restore` — so a manifest that omits `[[startup]]`, declares a second one, or
  misspells `restore` fails this check rather than failing silently in Herdr's plugin log
- **AND** `scripts/build.sh` — the one `command` path that is committed to the repository —
  exists and is executable, resolved relative to the repository root

`target/release/herdr-openspec` is **not** asserted to exist here. `make check` does not run
`make build` and `cargo test` builds the debug profile, so a release-artifact assertion
would be red on every clean checkout and in CI. Its existence is verified after `make build`
by `plugin-build` → "The crate produces one binary from an argued dependency set" and by
this change's live Herdr check.
