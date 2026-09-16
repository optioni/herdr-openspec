## MODIFIED Requirements

### Requirement: `config.toml` yields three values, each with a documented default

The plugin SHALL read `config.toml` from the resolved configuration directory into a
`Config` value carrying exactly three scalar settings:

| Key | Type in TOML | Default when absent |
|---|---|---|
| `openspec_bin` | string | absent — the caller falls back to its own probe chain |
| `agent_kind` | string | **absent** — the caller resolves the kind by `integration-status`' precedence |
| `archived_count` | integer | 5 |

`agent_kind` SHALL be `Option<String>` and SHALL have **no** default. Its documented default
of `claude` is removed: a constant that fires whether or not the reader has ever used Claude
Code is the guess this change exists to stop making, and `claude` now appears only as
`integration::resolve`'s last resort, in `src/integration.rs`. `Config::default()` SHALL
therefore carry `agent_kind: None`, and the literal `claude` SHALL NOT appear in
`src/config.rs` at all.

A value that is empty or whitespace-only SHALL yield `None` and SHALL report one problem
naming `agent_kind`, on `config::non_blank`'s established "first non-blank wins" rule — a
blank string in a hand-edited file is a reader who meant to set something, not a reader who
meant to set nothing, and silently falling through to a resolved kind would hide the typo.
A value with surrounding whitespace SHALL be trimmed and honoured.

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
exactly as it did, with exactly the same `Config::problems` — with the one stated exception
that a `config.toml` carrying a **blank** `agent_kind` now reports a problem it did not
before, which is the point.

#### Scenario: Every key is set

- **WHEN** `config.toml` contains `openspec_bin = "/opt/bin/openspec"`,
  `agent_kind = "codex"`, and `archived_count = 12`
- **THEN** the loaded `Config` carries that binary path, `agent_kind` `Some("codex")`, and an
  archived count of 12
- **AND** no problem is reported

#### Scenario: The file does not exist

- **WHEN** the configuration directory exists but contains no `config.toml`
- **THEN** the loaded `Config` is the default — `openspec_bin` absent, `agent_kind`
  **`None`**, `archived_count` 5
- **AND** no problem is reported, because an absent file is the normal case rather than
  a fault
- **AND** `agent_kind` is `None` rather than `Some("claude")`, so a reader who has never
  written a `config.toml` reaches `integration-status`' precedence rather than a constant

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
- **AND** `agent_kind` is `None` and `openspec_bin` is absent
- **AND** no problem is reported, because 0 is a legitimate value and not an absent one
- **AND** loading a repository whose archive holds three changes with that `Config`, with the
  archived section expanded, still lists all three: the value parsed and reached `Config`, and
  nothing read it

#### Scenario: A blank `agent_kind` is not a value

- **WHEN** `config.toml` contains `agent_kind = ""`, and again `agent_kind = "   "`
- **THEN** `agent_kind` is `None` in both cases
- **AND** exactly one problem is reported in each, naming `agent_kind`
- **AND** `config.toml` containing `agent_kind = "  codex  "` yields `Some("codex")` with no
  problem, so the value is trimmed and honoured rather than rejected for its whitespace

#### Scenario: Unrecognised keys are ignored

- **WHEN** `config.toml` contains `agent_kind = "codex"` alongside
  `future_setting = "x"` and a `[some_table]` section
- **THEN** `agent_kind` is `Some("codex")` and the other two keys are ignored
- **AND** no problem is reported

#### Scenario: A pre-`list-sections` configuration loads unchanged

- **WHEN** `config.toml` contains `openspec_bin = "/opt/bin/openspec"`, `agent_kind = "codex"`,
  and `archived_count = 25`
- **THEN** the loaded `Config` carries that binary path, `agent_kind` `Some("codex")`, and an
  archived count of 25, and reports no problem
- **AND** the dashboard that `Config` produces lists **every** archived change once the
  archived section is expanded, whatever the number is — 25, 5, or 0 — so the key is inert
  rather than reinterpreted

## ADDED Requirements

### Requirement: Per-kind prompt overrides are read from `config.toml` and reported entry by entry

`Config` SHALL carry a fourth setting, the per-kind prompt overrides, read from an optional
`[prompts]` table whose sub-tables are keyed by agent kind:

```toml
[prompts.codex]
apply = "work on {change} using {openspec}"
archive = "archive {change}"
```

It SHALL be held as `prompts: BTreeMap<String, BTreeMap<String, String>>` — kind, then intent
name, then text — rather than a new struct, so that adding an intent later moves no type and
`NODEFAULT-UI`'s scanned sets gain no eighth member for this key alone. Its default SHALL be
the empty map.

Only the three intent names `apply`, `continue`, and `archive` SHALL be read from a kind's
sub-table. Any other key SHALL be ignored **without comment**, on exactly the top-level
unrecognised-key rule's terms, so a newer plugin's configuration can be read by an older
binary.

Malformed entries SHALL degrade per entry rather than wholesale, on `state::read`'s
established rule: a `prompts` value that is not a table, a kind whose value is not a table, and
an intent whose value is not a string SHALL each be skipped with exactly one problem naming
what was skipped, while every well-formed entry in the same file still reaches `Config`. A
blank override text SHALL be treated as absent and SHALL report one problem, on exactly
`agent_kind`'s terms — the built-in prompt is then used for that intent.

Overrides SHALL be looked up by the kind `integration::resolve` actually returned. A kind with
no `[prompts.<kind>]` table SHALL use the built-in text for all three intents.

#### Scenario: A well-formed override table reaches `Config`

- **WHEN** `config.toml` contains `[prompts.codex]` with `apply = "do {change}"` and
  `archive = "archive {change}"`, and `[prompts.gemini]` with `continue = "next"`
- **THEN** `config.prompts` holds two kinds, `codex` with two intents and `gemini` with one
- **AND** no problem is reported
- **AND** `config.prompts` is empty for any other kind, so a lookup for `claude` finds nothing
  and the built-in text is used

#### Scenario: No `[prompts]` table at all

- **WHEN** `config.toml` sets `agent_kind = "codex"` and nothing else
- **THEN** `config.prompts` is the empty map
- **AND** no problem is reported, because an absent table is the normal case

#### Scenario: One malformed entry is skipped and the rest survive

- **WHEN** `config.toml` contains `[prompts.codex]` with `apply = "do {change}"`,
  `continue = 7`, and `unknown_intent = "x"`
- **THEN** `codex`'s `apply` override is present and its `continue` override is absent
- **AND** exactly one problem is reported, naming `prompts.codex.continue`
- **AND** `unknown_intent` contributes no problem, because an unrecognised key is ignored
  without comment

#### Scenario: A `prompts` value that is not a table degrades wholesale with one problem

- **WHEN** `config.toml` contains `prompts = "yes"`
- **THEN** `config.prompts` is the empty map
- **AND** exactly one problem is reported, naming `prompts`
- **AND** every other key in the same file still loads normally, so one bad key costs one
  setting

#### Scenario: A blank override is treated as absent

- **WHEN** `config.toml` contains `[prompts.codex]` with `apply = "   "`
- **THEN** `codex` carries no `apply` override and the built-in `Apply` text is used
- **AND** exactly one problem is reported, naming `prompts.codex.apply`
