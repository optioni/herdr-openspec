## MODIFIED Requirements

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
