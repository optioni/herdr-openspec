## ADDED Requirements

### Requirement: The configuration directory is resolved from the process environment

The plugin SHALL determine its configuration directory by reading the process
environment, and SHALL NOT spawn any process to obtain it. `HERDR_PLUGIN_CONFIG_DIR`,
which Herdr sets on every plugin pane and action process, is authoritative when it is
set to a non-empty value. When it is unset or empty — the binary was run outside a
Herdr-managed pane — the directory SHALL fall back to
`$HOME/.config/herdr/plugins/config/herdr-openspec`, the same path
`herdr plugin config-dir herdr-openspec` prints. When neither `HERDR_PLUGIN_CONFIG_DIR`
nor `HOME` is available there SHALL be no configuration directory, and every value
SHALL take its default.

The resolution SHALL be a function of an environment lookup passed in by the caller, so
that it is exercised without mutating the real process environment; exactly one thin
wrapper SHALL bind it to `std::env::var`.

#### Scenario: Herdr supplies the directory

- **WHEN** the directory is resolved with `HERDR_PLUGIN_CONFIG_DIR` set to
  `/tmp/cfg-fixture` and `HOME` set to `/home/someone`
- **THEN** the resolved directory is `/tmp/cfg-fixture`
- **AND** `HOME` is not consulted, proven by the result being unchanged when `HOME` is
  absent from the same lookup

#### Scenario: Run outside a Herdr pane

- **WHEN** the directory is resolved with `HERDR_PLUGIN_CONFIG_DIR` absent and `HOME`
  set to `/home/someone`
- **THEN** the resolved directory is
  `/home/someone/.config/herdr/plugins/config/herdr-openspec`

#### Scenario: An empty environment variable is not a directory

- **WHEN** the directory is resolved with `HERDR_PLUGIN_CONFIG_DIR` set to the empty
  string and `HOME` set to `/home/someone`
- **THEN** the resolved directory is
  `/home/someone/.config/herdr/plugins/config/herdr-openspec`, not the empty path or
  the current directory

#### Scenario: Neither variable is available

- **WHEN** the directory is resolved with both `HERDR_PLUGIN_CONFIG_DIR` and `HOME`
  absent
- **THEN** no directory is resolved
- **AND** loading configuration for that outcome yields exactly the default `Config` —
  `openspec_bin` absent, `agent_kind` `claude`, `archived_count` 5 — and reports no
  problem, because an absent directory is a supported state and not a fault

#### Scenario: Resolution spawns nothing

- **WHEN** the crate's own tests are run on a `PATH` from which the `herdr` binary
  cannot be resolved
- **THEN** every directory-resolution and configuration-loading test still passes
- **AND** a search of `src/` finds no `std::process::Command`, no `Stdio`, and no
  `herdr` invocation, because the module that is permitted to spawn (`cli`) does not
  exist yet and this change does not create one

### Requirement: `config.toml` yields three values, each with a documented default

The plugin SHALL read `config.toml` from the resolved configuration directory into a
`Config` value carrying exactly three settings:

| Key | Type in TOML | Default when absent |
|---|---|---|
| `openspec_bin` | string | absent — the caller falls back to its own probe chain |
| `agent_kind` | string | `claude` |
| `archived_count` | integer | 5 |

A missing directory, a missing file, an empty file, and a file that sets only some of
the keys SHALL each yield the documented default for every key not supplied. Keys the
plugin does not recognise SHALL be ignored without comment, so a newer plugin's
configuration can be read by an older binary. Configuration SHALL be read once per
process; nothing in this capability re-reads or watches the file.

#### Scenario: Every key is set

- **WHEN** `config.toml` contains `openspec_bin = "/opt/bin/openspec"`,
  `agent_kind = "codex"`, and `archived_count = 12`
- **THEN** the loaded `Config` carries that binary path, the agent kind `codex`, and an
  archived count of 12
- **AND** no problem is reported

#### Scenario: The file does not exist

- **WHEN** the configuration directory exists but contains no `config.toml`
- **THEN** the loaded `Config` is the default — `openspec_bin` absent, `agent_kind`
  `claude`, `archived_count` 5
- **AND** no problem is reported, because an absent file is the normal case rather than
  a fault

#### Scenario: The directory does not exist

- **WHEN** the resolved configuration directory does not exist on disk
- **THEN** the loaded `Config` is the default
- **AND** no problem is reported
- **AND** the directory is not created

#### Scenario: An empty file is not a malformed file

- **WHEN** `config.toml` exists and is zero bytes
- **THEN** the loaded `Config` is the default
- **AND** no problem is reported, distinguishing this outcome from the malformed-file
  scenario below, which reports one

#### Scenario: Only one key is set

- **WHEN** `config.toml` contains `archived_count = 0` and nothing else
- **THEN** `archived_count` is 0, meaning no archived changes are listed
- **AND** `agent_kind` is `claude` and `openspec_bin` is absent
- **AND** no problem is reported, because 0 is a legitimate value and not an absent one

#### Scenario: Unrecognised keys are ignored

- **WHEN** `config.toml` contains `agent_kind = "codex"` alongside
  `future_setting = "x"` and a `[some_table]` section
- **THEN** `agent_kind` is `codex` and the other two keys are ignored
- **AND** no problem is reported

### Requirement: Malformed configuration degrades to defaults and reports what it ignored

Loading configuration SHALL NOT fail, panic, or return an error to the caller under any
input. A file that is not valid TOML SHALL yield the complete set of defaults. A key
whose value is of the wrong TOML type, or an `archived_count` that is negative, SHALL
yield that key's default while every other key that parsed correctly is still honoured.
Each such fallback SHALL be recorded as a human-readable problem string on the returned
`Config`, so a degraded read is observably different from a read that found nothing —
this change produces those strings and renders none of them; `degraded-states` owns
surfacing them.

#### Scenario: The file is not valid TOML

- **WHEN** `config.toml` contains `agent_kind = = "codex"`
- **THEN** the loaded `Config` is the default in all three settings
- **AND** exactly one problem is reported, and it names `config.toml`

#### Scenario: One key has the wrong type, the rest survive

- **WHEN** `config.toml` contains `openspec_bin = true`, `agent_kind = "codex"`, and
  `archived_count = "many"`
- **THEN** `openspec_bin` is absent and `archived_count` is 5, each having fallen back
- **AND** `agent_kind` is still `codex`
- **AND** two problems are reported, naming `openspec_bin` and `archived_count`

#### Scenario: A negative count is not a count

- **WHEN** `config.toml` contains `archived_count = -1`
- **THEN** `archived_count` is 5
- **AND** one problem is reported, naming `archived_count`

#### Scenario: The file cannot be read

- **WHEN** the path `config.toml` inside the configuration directory exists but is a
  directory rather than a file, so reading it fails at the operating system
- **THEN** the loaded `Config` is the default in all three settings
- **AND** one problem is reported, naming `config.toml`
- **AND** no panic and no error reaches the caller

### Requirement: `openspec_bin` is expanded and never probed

A configured `openspec_bin` SHALL be expanded before it is returned: a leading `~/` and
a leading `$HOME/` each become the value of `HOME`, and a bare `~` becomes `HOME`
itself. Expansion SHALL use the same environment lookup that resolved the configuration
directory. When `HOME` is unavailable the value SHALL be returned verbatim rather than
half-expanded. A value that is empty or whitespace-only SHALL be treated as absent. A
`~user` form SHALL be returned verbatim, because resolving another user's home would
require a lookup this plugin does not perform. The plugin SHALL NOT check that the path
exists, is a file, or is executable — `repo-resolution` owns the probe chain that
decides.

#### Scenario: A tilde path is expanded

- **WHEN** `config.toml` contains `openspec_bin = "~/.nvm/versions/node/v24/bin/openspec"`
  and `HOME` is `/home/someone`
- **THEN** `openspec_bin` is `/home/someone/.nvm/versions/node/v24/bin/openspec`

#### Scenario: A `$HOME` path is expanded and a bare tilde is the home directory

- **WHEN** `config.toml` contains `openspec_bin = "$HOME/bin/openspec"` and `HOME` is
  `/home/someone`
- **THEN** `openspec_bin` is `/home/someone/bin/openspec`
- **AND** the same input with the value `~` yields `/home/someone`

#### Scenario: An unexpandable value is returned verbatim

- **WHEN** `config.toml` contains `openspec_bin = "~/bin/openspec"` and `HOME` is absent
- **THEN** `openspec_bin` is the unexpanded string `~/bin/openspec`
- **AND** the same input with the value `~otheruser/bin/openspec` is returned verbatim
  even when `HOME` is set

#### Scenario: An empty value is an absent value

- **WHEN** `config.toml` contains `openspec_bin = "   "`
- **THEN** `openspec_bin` is absent, exactly as if the key had not been written
- **AND** no problem is reported

#### Scenario: A path that does not exist is still returned

- **WHEN** `config.toml` contains `openspec_bin = "/nowhere/openspec"` and that path
  does not exist
- **THEN** `openspec_bin` is `/nowhere/openspec`
- **AND** no filesystem access is made to test it, and no problem is reported

### Requirement: Reading configuration writes nothing

Loading configuration SHALL be read-only. It SHALL NOT create the configuration
directory, create or truncate `config.toml`, or modify any file's contents or
modification time. It SHALL write nothing anywhere under the repository's `openspec/`
directory, which this capability never reads or names.

#### Scenario: A configuration read leaves the tree byte-identical

- **WHEN** a configuration directory holding a valid `config.toml` and one unrelated
  file is loaded twice
- **THEN** the directory listing, every file's bytes, and every file's modification time
  are unchanged afterwards
- **AND** no temporary file, lock file, or backup remains

#### Scenario: A missing configuration directory stays missing

- **WHEN** configuration is loaded from a directory path that does not exist
- **THEN** that path still does not exist afterwards
