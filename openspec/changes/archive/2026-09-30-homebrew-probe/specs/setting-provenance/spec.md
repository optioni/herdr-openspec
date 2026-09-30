## MODIFIED Requirements

### Requirement: `Provenance` is one enum whose labels are written once

`Provenance` SHALL name the level that produced a value, and SHALL be the crate's **one**
site that turns a level into reader-facing text:

```rust
pub enum Provenance {
    Configured,
    Recorded,
    Probe(resolve::BinSource),
    SoleIntegration,
    LastResort,
    Default,
    Unresolved,
    Pending,
    Ambiguous,
}

impl Provenance {
    pub fn label(self) -> String;
}
```

`Provenance` SHALL be **derived from** the two enums that already carry this fact rather
than restate them. `integration::Source` — `agent-client-choice`'s five-step precedence —
SHALL be converted by a total `From<integration::Source> for Provenance`, and
`resolve::BinSource` SHALL be carried whole inside `Probe`. Neither enum SHALL be duplicated,
matched on by string, or re-ordered here: they are the contract, and a second table of the
same levels is the drift this rule exists to prevent, exactly as `ui::tasks::gauge_of` and
`ui::list::progress_cell` are the crate's one gauge and one progress cell.

`Pending` and `Ambiguous` are **not** reachable from `integration::Source`, and that is why
they are variants here rather than a wider `From`:

- `Pending` is the state before the launcher's worker has answered — `settings()` was given
  `kind: None`. It is not an error and not a failure; it is the honest answer to "what decided
  this?" for the two or three frames before the worker replies.
- `Ambiguous` is `integration::Choice::Ambiguous` — two or more integrations installed and
  none chosen. `Choice` is `Use { kind, source } | Ambiguous { installed }`, so `Source`
  **exists only on `Use`** and a `From<Source>` can never produce this. `Ambiguous` is also
  the state in which the panel is most often opened, since `agent-launch` opens it on exactly
  that outcome, so leaving it unrepresentable would have left the headline path undefined.

`label` SHALL be total, SHALL allocate no more than its returned `String`, and SHALL name
the level in the reader's terms rather than the enum's — `config.toml`, `settings.toml`, the
probe step that won, the sole installed integration, the last resort, the default, "not
found", "resolving", and "two or more installed, none chosen".

This module SHALL join `NODEFAULT-UI`'s scanned type sets as a **ninth** subject, with
`HOMEFILE=src/settings.rs` and its own measured `SCAN_MIN` on its own `Makefile` recipe line,
on exactly the terms the eighth was added: one shared floor across nine sets would either pass
vacuously for the smallest or fail legitimately for the largest.

Its `TYPES` list SHALL be `Setting KindResolution PanelState` — the module's **structs**, and
only those. `scripts/gates/nodefault-ui.sh`'s positive control is `grep -qE "struct[[:space:]]+$T[[:space:]]*\{"`
against the `HOMEFILE`, so naming `Provenance`, `Editable`, or `Reason` there would fail the
control outright rather than scan them. Those three are enums and SHALL be covered by an
exhaustive `match` with no wildcard arm, on exactly `launch::Intent`'s terms.

No type in this module SHALL derive or implement `Default`, enum or struct.

#### Scenario: Every `integration::Source` maps to a `Provenance` and back to one label

- **WHEN** each of `Source::Configured`, `Source::Recorded`, `Source::SoleIntegration`, and
  `Source::LastResort` is converted through `From`
- **THEN** it yields `Configured`, `Recorded`, `SoleIntegration`, and `LastResort`
  respectively
- **AND** each resulting `label()` is non-empty, and the four labels are pairwise distinct,
  so no two precedence levels read the same on screen
- **AND** the conversion is exhaustive by construction — it matches on `Source` with no
  wildcard arm — so a sixth precedence level added later fails to compile here rather than
  falling silently into a wrong label
- **AND** `Pending` and `Ambiguous` are produced by neither `From` arm, since neither has a
  `Source` to convert: the first comes from `kind: None` and the second from
  `Choice::Ambiguous`, which carries no `Source` at all

#### Scenario: Every probe step is named by the step that won

- **WHEN** `Provenance::Probe(source).label()` is called for each of `resolve::BinSource`'s
  five variants — `Configured`, `Path`, `Nvm`, `NpmPrefix`, `Homebrew`
- **THEN** each label is non-empty and names that step distinctly from the other four
- **AND** `Homebrew`'s label is `Homebrew`, naming the package manager the reader installed
  with rather than the directory the step happened to find
- **AND** the labels for two steps that resolved the **same path** still differ, which is
  the reason `openspec-binary` made the winning step part of the contract rather than a
  debugging aid

#### Scenario: `Unresolved` is file mode's own label and is not an error

- **WHEN** `settings` is called with a `BinResolution` whose `found` is `None`
- **THEN** the `openspec_bin` entry's provenance is `Unresolved`
- **AND** its `value` names that no binary was found rather than being empty
- **AND** its `label()` is non-empty, so the row reads as a stated state rather than a blank

#### Scenario: A pending kind renders as resolving and edits nothing

- **WHEN** `settings` is called with `kind` `None` — the panel opened before the launcher's
  worker has answered
- **THEN** the `agent_kind` entry's provenance is `Pending` and its `label()` is non-empty
- **AND** its `value` names that the kind is still resolving rather than being empty or
  guessing a kind
- **AND** its `editable` is `No`, so `Enter` begins no edit against a shortlist that does not
  exist yet
- **AND** the `openspec_bin` and `prompts` entries are fully populated in the same call, so
  two thirds of the panel is useful on the first frame

#### Scenario: An ambiguous kind has no effective value and offers both candidates

- **WHEN** `settings` is called with `kind` `Some(KindResolution { choice:
  Choice::Ambiguous { installed: ["claude", "codex"] }, installed: ["claude", "codex"] })`
- **THEN** the `agent_kind` entry's provenance is `Ambiguous` and its `value` says no kind has
  been chosen rather than naming one
- **AND** its `editable` is `Kind { shortlist: ["claude", "codex"] }` — both candidates, in
  Herdr's order — so the panel `agent-launch` just opened can resolve the very refusal that
  opened it
- **AND** this is the one state where provenance reports no deciding level and the row is
  still editable, which is the point: there is nothing deciding it, and that is the problem
  the reader is here to fix
