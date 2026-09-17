## ADDED Requirements

### Requirement: The recorded agent kind is read from `settings.toml` and never written here

The plugin SHALL read an optional recorded agent kind from `settings.toml` inside the
resolved state directory, beside `agent-names.toml`:

```toml
agent_kind = "codex"
```

```rust
pub fn recorded_kind(dir: Option<&Path>) -> (Option<String>, Vec<String>);
```

It SHALL be step 2 of `integration-status`' precedence: what `settings-window` will write once
it lands, consulted after a hand-edited `config.toml` and before any evidence the plugin
gathered itself.

This change SHALL **read** it and SHALL NOT write it. No code path added by this change
SHALL create `settings.toml`, and the plugin's own writes SHALL remain exactly
`agent-names.toml` under `HERDR_PLUGIN_STATE_DIR` — the write boundary
`SPEC.md` → "The plugin's own writes are scoped to its state directory" already fixes. Writing
the choice into `config.toml` is forbidden on the same terms: the configuration directory is
the user's to hand-edit.

Reading SHALL never fail, panic, or return an error to the caller, on exactly `state::read`'s
terms. An absent directory, an absent file, and an empty file SHALL each yield `None` with no
problem. A file that is not valid TOML, an `agent_kind` that is not a string, and an
`agent_kind` that is blank or whitespace-only SHALL each yield `None` with exactly one
problem. Keys other than `agent_kind` SHALL be ignored without comment, so the file
`settings-window` grows later is readable by this binary.

#### Scenario: A recorded kind is read back

- **WHEN** `settings.toml` in the state directory contains `agent_kind = "codex"`
- **THEN** `recorded_kind` returns `(Some("codex"), [])`
- **AND** a value with surrounding whitespace, `agent_kind = "  codex  "`, returns the same
  trimmed `Some("codex")` with no problem

#### Scenario: Absent directory, absent file, and empty file are all silent

- **WHEN** `recorded_kind` is called with `None`, then with a directory holding no
  `settings.toml`, then with a zero-byte `settings.toml`
- **THEN** all three return `(None, [])`
- **AND** no directory and no file is created by any of the three calls

#### Scenario: An unusable file yields `None` and exactly one problem

- **WHEN** `settings.toml` contains `agent_kind = = "codex"`, then `agent_kind = 7`, then
  `agent_kind = "   "`
- **THEN** all three return `None`
- **AND** each reports exactly one problem naming `settings.toml`
- **AND** no panic and no error reaches the caller in any of the three

#### Scenario: Unrecognised keys are ignored

- **WHEN** `settings.toml` contains `agent_kind = "codex"` alongside `theme = "dark"` and a
  `[window]` table
- **THEN** `recorded_kind` returns `(Some("codex"), [])`
- **AND** the two unrecognised keys contribute no problem

#### Scenario: Nothing in this change writes `settings.toml`

- **WHEN** `run_wired` is driven against a scratch state directory through a full launch that
  reads the recorded kind, starts an agent, and records its derived name
- **THEN** the state directory afterwards holds `agent-names.toml` and no `settings.toml`
- **AND** a run started with a `settings.toml` already present leaves that file byte-identical
- **AND** the configuration directory is byte-identical in both runs, so the choice is never
  written back into `config.toml`
