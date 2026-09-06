## ADDED Requirements

### Requirement: The configuration's fallbacks reach the pane

`Config::problems` names every key that fell back to its documented default, and `SPEC.md` →
Degraded states promises exactly that: "the affected key falls back to its documented default
while every other key that parsed correctly is still honoured; `Config::problems` names each
fallback". The vector is populated correctly and **never read**: `ui::load` consults
`config.archived_count` and nothing else, so a reader running against a malformed
`config.toml` sees a pane that silently ignores their settings.

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
- **AND** `archived_count` in effect is still `5`, so the fallback was applied as well as
  reported

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
