## ADDED Requirements

### Requirement: Each setting's effective value and the level that produced it are one value

The crate SHALL gain `src/settings.rs`, a **pure** module outside `src/ui/` on exactly
`src/specs.rs`' and `src/integration.rs`' terms: no filesystem, process, environment,
network, or standard-I/O API, no clock, no global mutable state, and no panic for any input.

It SHALL produce, as a total function of values the composition root already holds, the
ordered list of settings the panel renders:

```rust
pub fn settings(
    config: &config::Config,
    binary: &resolve::BinResolution,
    kind: Option<&KindResolution>,
) -> Vec<Setting>;

/// What the launcher's worker answers with. Produced outside `src/ui/`, on
/// `agents::AgentSnapshot`'s terms: plain data, no trait, no handle, no thread.
pub struct KindResolution {
    pub choice: integration::Choice,
    pub installed: Vec<String>,
}
```

```rust
pub struct Setting {
    pub key: &'static str,
    pub value: String,
    pub provenance: Provenance,
    pub editable: Editable,
}
```

The list SHALL hold exactly three entries in this fixed order: `openspec_bin`, `agent_kind`,
`prompts`. The order is the module's, not the view's, so the panel renders what it is given
rather than deciding an order a second time.

`archived_count` SHALL NOT be one of them. `list-sections` made the key accepted and inert,
and `settings-window` states why a row for a value nothing consumes does not belong in a
panel whose job is to say what is deciding each setting.

Producing this list SHALL perform no I/O.

`config` and `binary` are available at **startup**: `ui::load` already receives a `&Config` —
today as `_config`, an unused parameter — and the composition root already holds the
`BinResolution` it derived `file_mode` from. `kind` is **not**, and that asymmetry is the
whole of this requirement's difficulty. `agent-client-choice` resolves the agent kind
**lazily, once per session, on the launcher's worker thread, on the first `a`/`c`/`s` press
and never at startup**, and that rule does not move here: resolving on the render path would
put a blocking `herdr integration status` call in `src/ui/`, and resolving at startup would
reverse a landed decision.

`kind` is therefore `Option<&KindResolution>`, and `None` — the worker has not answered — is a
**first-class state this capability specifies**, not an edge case. Opening the settings panel
SHALL send a non-blocking `launch::Request::Resolve` to that worker, which answers from its
existing session cache when it has one and performs the `integration status` call exactly once
when it does not. The panel renders immediately with `agent_kind` reading
[`Provenance::Pending`], and re-renders when the answer is adopted. Opening the panel SHALL
NOT block, and SHALL NOT resolve anything on the render thread.

`config::Config` SHALL gain **no** provenance field. `proposal.md` put provenance "beside each
value" in `src/config.rs`; it lands here instead, because `Config` is the parse result of one
file and cannot know about the four other levels — a probe step, an installed integration, a
last resort, a default — that outrank or underwrite it. Absence of a key in `Config` is the
only provenance signal `config.toml` can honestly give, and this module joins it to the rest.

#### Scenario: Three settings in a fixed order, whatever the inputs

- **WHEN** `settings` is called with a default `Config`, a `BinResolution` whose `found` is
  `None`, and `kind` `None`
- **THEN** it returns exactly three entries whose keys are `openspec_bin`, `agent_kind`,
  `prompts`, in that order
- **AND** the same three keys in the same order come back when every input is populated
  instead, so the order is fixed and not derived from which values are present
- **AND** no entry's key is `archived_count`, including when `Config::archived_count` is set
  to a non-default value

#### Scenario: The module names no I/O API

- **WHEN** `src/settings.rs`' production slice — everything above its `#[cfg(test)]` line —
  is searched for `std::fs`, `std::process`, `std::env`, `std::net`, `std::io`,
  `read_to_string`, `Command`, `Instant`, and `SystemTime`
- **THEN** none of them appears
- **AND** a planted `use std::fs;` above that line makes the check fail, so the check is not
  vacuous

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
  four variants — `Configured`, `Path`, `Nvm`, `NpmPrefix`
- **THEN** each label is non-empty and names that step distinctly from the other three
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

### Requirement: `Editable` carries the shortlist or the reason a setting cannot be edited

`Editable` SHALL state, per setting, whether the panel may edit it and — when it may not —
why:

```rust
pub enum Editable {
    Kind { shortlist: Vec<String> },
    No { reason: Reason },
}
```

`settings` SHALL decide this once, and the view SHALL NOT re-derive it. The rules SHALL be:

- `openspec_bin` and `prompts` are always `No`, with the reason that they are set-once
  values read from `config.toml`;
- a setting whose provenance is `Configured` is `No`, with the reason that `config.toml`
  is deciding it — the refusal `settings-window` requires;
- `agent_kind` whose provenance is `Pending` is `No`, with the reason that the kind is still
  resolving;
- `agent_kind` whose provenance is `Ambiguous` is `Kind`, carrying both candidates;
- `agent_kind` with **no** installed integration is `No`, with the reason naming
  `config.toml` as the way to set it;
- `agent_kind` otherwise is `Kind`, carrying the installed kinds in Herdr's printed order.

`Kind` is therefore the **only** editable variant, and `agent_kind` the only setting that can
carry it. `Editable` is still an enum rather than an `Option<Vec<String>>` because `No` carries
a `Reason` the panel renders, and because a second editable setting added later extends this
enum rather than reshaping every match on it.

The `Configured` rule SHALL be checked **before** the per-setting rules, so a configured
`agent_kind` is refused for naming `config.toml` rather than for its shortlist, and the
reader is told the thing that is actually deciding the value.

`Reason` SHALL be an enum, not a free `String`, so the panel's refusal wording lives in one
place and a test can assert which refusal fired rather than matching on prose.

#### Scenario: The `Configured` rule outranks the shortlist rule

- **WHEN** `settings` is called with `config.toml` setting `agent_kind = "codex"` **and** two
  installed integrations
- **THEN** the `agent_kind` entry's `editable` is `No` with the reason naming `config.toml`,
  not `Kind` with a two-entry shortlist
- **AND** with the same two integrations and **no** configured `agent_kind`, the same entry
  is `Kind` with a shortlist of exactly those two kinds in Herdr's order

#### Scenario: Read-only settings stay read-only however they were resolved

- **WHEN** `settings` is called across the cartesian product of a configured, a probed,
  and an unresolved `openspec_bin` with an empty and a populated `prompts` table
- **THEN** the `openspec_bin` and `prompts` entries are `No` in every combination
- **AND** the reason for a configured one names `config.toml` and the reason for the others
  names that they are set-once values, so the two refusals are distinguishable

#### Scenario: An empty shortlist never reaches the view as `Kind`

- **WHEN** `settings` is called with no configured `agent_kind` and no installed integration
- **THEN** the `agent_kind` entry's `editable` is `No` and never `Kind { shortlist: [] }`
- **AND** the reason names `config.toml`
- **AND** no code path in `src/ui/settings.rs` needs to test a shortlist for emptiness,
  because the empty case cannot be represented as `Kind` by the time it arrives
