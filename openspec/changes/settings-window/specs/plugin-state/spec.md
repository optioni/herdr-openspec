## REMOVED Requirements

### Requirement: The recorded agent kind is read from `settings.toml` and never written here

**Reason**: Its title and its closing scenario are both a standing claim that nothing writes
`settings.toml`, and this change is the one that writes it — `agent-client-choice` wrote that
requirement naming `settings-window` as the change that would. The requirement has also
accumulated two behaviours that now move apart: reading the file, which is unchanged, and the
write boundary, which is not. It is split rather than reworded, because a MODIFIED block that
kept the header would leave the live spec asserting the opposite of what the code does.

**Migration**: Replaced by two ADDED requirements below — "The recorded agent kind is read from
`settings.toml`", which carries every reading scenario forward unchanged, and "The settings
panel's commit writes `settings.toml` atomically", which states the new write and re-states the
boundary it does **not** cross. No reading behaviour changes: `state::recorded_kind` keeps its
signature, its totality, and its problem wording, and callers need no edit.

## ADDED Requirements

### Requirement: The recorded agent kind is read from `settings.toml`

The plugin SHALL read an optional recorded agent kind from `settings.toml` inside the resolved
state directory, beside `agent-names.toml`:

```toml
agent_kind = "codex"
```

```rust
pub fn recorded_kind(dir: Option<&Path>) -> (Option<String>, Vec<String>);
```

It SHALL be step 2 of `integration-status`' precedence: what the settings panel writes,
consulted after a hand-edited `config.toml` and before any evidence the plugin gathered itself.

`agent_kind` SHALL remain the file's **only** recognised key. `settings-window` drops the
`archived_count` row from the panel — `list-sections` made that key inert — so nothing this
change adds has a second value to persist, and the file's schema does not grow.

Reading SHALL never fail, panic, or return an error to the caller, on exactly `state::read`'s
terms. An absent directory, an absent file, and an empty file SHALL each yield `None` with no
problem. A file that is not valid TOML, an `agent_kind` that is not a string, and an
`agent_kind` that is blank or whitespace-only SHALL each yield `None` with exactly one
problem. Keys other than `agent_kind` SHALL be ignored without comment, so a file a later
version grows is readable by this binary.

Reading SHALL create nothing. No read path SHALL create the state directory or the file.

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

### Requirement: The settings panel's commit writes `settings.toml` atomically

The plugin SHALL gain one writer, and it SHALL be the only code path in the crate that creates
or replaces `settings.toml`:

```rust
pub fn record_kind(dir: Option<&Path>, kind: &str) -> std::io::Result<()>;
```

It SHALL be called from **exactly one** place — the commit of an `agent_kind` edit in the
settings panel — and from no other. Nothing at startup, on a refresh, on a launch, or on a poll
SHALL write it, so a reader who never opens the panel never has the file created.

The write SHALL be **atomic and shall create only what it needs**, on exactly `state::record`'s
established terms: a temporary file in the same directory, written and then renamed over the
target, so a crash mid-write cannot leave a truncated file that the next read reports as
unusable TOML. The state directory SHALL be created when absent, and nothing above it SHALL be.

The write SHALL rewrite the file from the value it is given. Unrecognised keys a previous
version or a hand-edit left behind SHALL NOT be preserved: the file is the plugin's own, its
schema is the one key above, and a merge would mean parsing and re-emitting arbitrary TOML —
including comments this writer cannot round-trip — for a file no human is expected to edit.

`record_kind(None, kind)` — no state directory could be resolved — SHALL return `Err` with
`state::record`'s own wording and SHALL create nothing, on exactly that function's terms.

A failed write SHALL NOT panic and SHALL NOT abandon the edit silently. The commit SHALL report
the failure as a problem row carrying the operating system's own reason, and the in-memory value
SHALL still update and SHALL still reach `Launcher::set_kind`, so the choice holds for the rest
of the session even when it could not be persisted for the next one.

This SHALL NOT widen the write boundary. The plugin's own writes SHALL remain exactly
`agent-names.toml` and `settings.toml`, both under `HERDR_PLUGIN_STATE_DIR` and nowhere else.
Writing into `openspec/` remains forbidden — an agent may be editing `tasks.md` in another pane
— and writing `config.toml` remains forbidden on its own terms: the configuration directory is
the user's to hand-edit, and a program rewriting it would destroy its comments and formatting
and race with the editor that owns it. `settings-window`'s refusal rule keeps the two apart at
the other end too: a setting `config.toml` owns cannot be edited, so no commit can ever be an
attempt to overrule that file.

#### Scenario: A commit creates the file with the chosen kind

- **WHEN** `record_kind` is called against a scratch state directory that holds no
  `settings.toml`, with `codex`
- **THEN** the directory afterwards holds a `settings.toml` parsing to exactly `agent_kind =
  "codex"`
- **AND** `recorded_kind` reads it back as `(Some("codex"), [])`
- **AND** `agent-names.toml` is untouched if present and is not created if absent

#### Scenario: A commit rewrites rather than merges

- **WHEN** a `settings.toml` holding `agent_kind = "claude"` and `theme = "dark"` is written
  again with `codex`
- **THEN** the file afterwards holds `agent_kind = "codex"` and no `theme`
- **AND** it parses cleanly and `recorded_kind` reports no problem

#### Scenario: Nothing outside the commit writes the file

- **WHEN** `run_wired` is driven against a scratch state directory through a startup, a full
  refresh, an agent poll, and a complete launch that records a derived agent name, with the
  settings panel never opened
- **THEN** the state directory afterwards holds `agent-names.toml` and no `settings.toml`
- **AND** a run started with a `settings.toml` already present leaves that file byte-identical
- **AND** the configuration directory is byte-identical in both runs

#### Scenario: A write failure is reported and does not lose the session's value

- **WHEN** a commit is attempted against a state directory path that exists as a **regular
  file**, so the directory cannot be created
- **THEN** `record_kind` returns `Err` rather than panicking
- **AND** the panel records exactly one problem carrying the operating system's own reason
- **AND** the committed in-memory value is the edited one and `Launcher::set_kind` was still
  called, so the rest of the session launches under the choice the reader made

#### Scenario: Nothing is written outside the state directory

- **WHEN** a commit runs against a scratch tree holding the state directory, the configuration
  directory, and an `openspec/` repository, with every file's path, bytes, and mtime digested
  before and after
- **THEN** the only path that differs afterwards is `settings.toml` inside the state directory
- **AND** every file under `openspec/` and under the configuration directory is byte-identical
