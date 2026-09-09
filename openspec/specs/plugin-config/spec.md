# plugin-config Specification

## Purpose
Reads the plugin's own `config.toml` into the settings the dashboard is tuned by —
`openspec_bin`, `agent_kind`, and `archived_count` — each with a documented default, after
resolving the configuration directory from an injected environment lookup
(`HERDR_PLUGIN_CONFIG_DIR`, else the `$HOME/.config/herdr/...` path Herdr itself prints,
deliberately ignoring `XDG_CONFIG_HOME`). Loading never fails: malformed TOML, wrong-typed
keys, and a negative count each degrade to the default while recording a human-readable
problem, and a configured `openspec_bin` is tilde/`$HOME`-expanded but never tested for
existence — probing it belongs to `openspec-binary`. Reading configuration spawns nothing and
writes nothing; the writing half of the plugin's own directories is `plugin-state`'s.

## Requirements

### Requirement: The configuration directory is resolved from the process environment

The plugin SHALL determine its configuration directory by reading the process
environment, and SHALL NOT spawn any process to obtain it. `HERDR_PLUGIN_CONFIG_DIR`,
which Herdr sets on the plugin processes it starts — pane processes and action
processes alike — is authoritative when it is set to a value that is neither empty nor
whitespace-only. When it is unset, empty, or whitespace-only — the binary was run
outside a Herdr-managed process — the directory SHALL fall back to
`$HOME/.config/herdr/plugins/config/herdr-openspec`, the same path
`herdr plugin config-dir herdr-openspec` prints. `XDG_CONFIG_HOME` SHALL NOT be
consulted, because the fallback's job is to agree with Herdr, and Herdr's own
configuration root is `$HOME/.config/herdr`. When neither `HERDR_PLUGIN_CONFIG_DIR` nor
`HOME` is available there SHALL be no configuration directory, and every value SHALL
take its default.

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

#### Scenario: An empty or blank environment variable is not a directory

- **WHEN** the directory is resolved with `HERDR_PLUGIN_CONFIG_DIR` set to the empty
  string and `HOME` set to `/home/someone`
- **THEN** the resolved directory is
  `/home/someone/.config/herdr/plugins/config/herdr-openspec`, not the empty path or
  the current directory
- **AND** the same holds for the value `"   "`, so a blank variable falls through rather
  than naming a directory of three spaces

#### Scenario: `XDG_CONFIG_HOME` is deliberately ignored

- **WHEN** the directory is resolved with `HERDR_PLUGIN_CONFIG_DIR` absent,
  `XDG_CONFIG_HOME` set to `/xdg`, and `HOME` set to `/home/someone`
- **THEN** the resolved directory is
  `/home/someone/.config/herdr/plugins/config/herdr-openspec`, not `/xdg/...`, because
  the fallback exists to agree with the path `herdr plugin config-dir` prints
- **AND** the state directory resolved from the same lookup *does* honour
  `XDG_STATE_HOME`, so the asymmetry is a recorded decision rather than an oversight

#### Scenario: Neither variable is available

- **WHEN** the directory is resolved with both `HERDR_PLUGIN_CONFIG_DIR` and `HOME`
  absent
- **THEN** no directory is resolved
- **AND** loading configuration for that outcome yields exactly the default `Config` —
  `openspec_bin` absent, `agent_kind` `claude`, `archived_count` 5 — and reports no
  problem, because an absent directory is a supported state and not a fault

#### Scenario: Resolution spawns nothing

- **WHEN** the crate's own tests are run on a `PATH` on which `cargo` resolves and the
  `herdr` binary does not
- **THEN** every directory-resolution and configuration-loading test still passes
- **AND** neither `src/config.rs` nor `src/state.rs` names a process API — no
  `std::process`, no `Command`, no `Stdio` — and neither passes `"herdr"` or
  `"openspec"` as a program name, because this capability's code sits on the pure side
  of the spawn seam that `cli` will own. Prose naming `herdr plugin config-dir` is
  documentation and is not an invocation

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

`archived_count` is **accepted and inert** as of `list-sections`. It is still read, still
type-checked, still defaults to `5`, and still reports a malformed value as a problem — every
scenario below is unchanged — but nothing consumes the value: `change-enumeration` no longer
truncates the archived tier, and the archived section's fold is what decides how much of the
archive is shown. The key is kept rather than removed so that no existing `config.toml`
becomes invalid and no reader loses a setting to a hard error; it SHALL be documented as
having no effect on the list in `README.md`'s configuration table and in `SPEC.md`'s
`config.toml` description, so a reader is not left setting a value that does nothing while
the documentation says it does something.

The plugin SHALL NOT repurpose the key, SHALL NOT warn about its presence, and SHALL NOT
report a problem for a well-formed value: a `config.toml` written before this change loads
exactly as it did, with exactly the same `Config::problems`.

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
- **THEN** `archived_count` is 0
- **AND** `agent_kind` is `claude` and `openspec_bin` is absent
- **AND** no problem is reported, because 0 is a legitimate value and not an absent one
- **AND** loading a repository whose archive holds three changes with that `Config`, with the
  archived section expanded, still lists all three: the value parsed and reached `Config`, and
  nothing read it

#### Scenario: Unrecognised keys are ignored

- **WHEN** `config.toml` contains `agent_kind = "codex"` alongside
  `future_setting = "x"` and a `[some_table]` section
- **THEN** `agent_kind` is `codex` and the other two keys are ignored
- **AND** no problem is reported

#### Scenario: A pre-`list-sections` configuration loads unchanged

- **WHEN** `config.toml` contains `openspec_bin = "/opt/bin/openspec"`, `agent_kind = "codex"`,
  and `archived_count = 25`
- **THEN** the loaded `Config` carries that binary path, the agent kind `codex`, and an
  archived count of 25, and reports no problem
- **AND** the dashboard that `Config` produces lists **every** archived change once the
  archived section is expanded, whatever the number is — 25, 5, or 0 — so the key is inert
  rather than reinterpreted

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
modification time. It names no repository path at all, so it cannot reach a repository's
`openspec/` tree; `plugin-state` carries the requirement and the fixture proving that a
repository is untouched by the one code path in this change that does write.

#### Scenario: A configuration read leaves the tree byte-identical

- **WHEN** a configuration directory holding a valid `config.toml` and one unrelated
  file is loaded twice
- **THEN** the directory listing, every file's bytes, and every file's modification time
  are unchanged afterwards
- **AND** no temporary file, lock file, or backup remains

#### Scenario: A missing configuration directory stays missing

- **WHEN** configuration is loaded from a directory path that does not exist
- **THEN** that path still does not exist afterwards

### Requirement: The configuration's fallbacks reach the pane

`Config::problems` names every key that fell back to its documented default, and `SPEC.md` →
Degraded states promises exactly that: "the affected key falls back to its documented default
while every other key that parsed correctly is still honoured; `Config::problems` names each
fallback". The vector was, before `degraded-states`, populated correctly and never read. It is read now,
and this requirement is what reads it — but the clause naming *what else* `ui::load` consults
is retired here: as of `list-sections`, `ui::load` reads `openspec_bin` and `agent_kind` and
**does not consult `config.archived_count` at all**, because that key no longer limits the
archived tier.

`ui::start_collaborators` SHALL fold `Config::problems` into the `problems` vector it hands to
`run_wired`, in the vector's own order, so each fallback reaches `Dashboard::refresh.problems`
and renders as a leading `! `-marked row of the list — the same grammar and the same lifetime
as a watcher that would not start, which is what a configuration fallback is: a standing
condition that outlives every reload.

Configuration problems SHALL be folded in **before** the binary resolution's, and both before
the watcher's, so the order a reader meets them is the order they occurred in: what the
configuration said, then what the binary probe made of it, then what the watcher did with the
result.

A `config.toml` that parses cleanly, and an absent `config.toml`, SHALL contribute **no**
problem: `Config::problems` is empty in both cases and the pane renders exactly as it did.

#### Scenario: A malformed key renders as a leading problem row at both widths

- **WHEN** `run_wired` is driven at 120x20 and again at 60x20 over a scratch repository with a
  `Config` carrying `problems: ["archived_count: expected an integer, found a string - using
  the default 5"]`, and an event source that presses `q`
- **THEN** the returned dashboard's `refresh.problems` holds that entry
- **AND** the list region's first interior row, at both widths, begins `! archived_count:` and
  the change rows follow below it
- **AND** the same run with a `Config` whose `archived_count` is `5`, `0`, or `25` produces a
  byte-identical buffer at both widths: the fallback is still applied to the `Config` value
  and still reported, and nothing downstream consumes it

#### Scenario: Configuration problems precede binary and watcher problems

- **WHEN** the same run is driven with a `Config` carrying one problem, an `openspec_bin`
  naming an unusable path, and a repository root that cannot be watched
- **THEN** `refresh.problems` holds all three entries, in that order: the configuration's, the
  binary probe's, then the watcher's
- **AND** the list's first three interior rows name them in that same order at both widths

#### Scenario: A clean configuration contributes nothing

- **WHEN** the same run is driven with a `Config` whose `problems` is empty
- **THEN** `refresh.problems` is empty and the list's first interior row is a change row
- **AND** the buffers are byte-identical to the ones the same dashboard produced before this
  change existed
