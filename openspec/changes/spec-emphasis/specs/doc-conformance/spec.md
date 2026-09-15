## ADDED Requirements

### Requirement: A non-view pure module's freedom from I/O is checked inside `cargo test`

`tests/doc_contract.rs` SHALL carry an **eleventh** claim: that the production slice of
`src/specs.rs` names no filesystem, process, environment, network, or standard-I/O API, and no
schema-reading name.

The check SHALL read the file's production slice — the text above its first line-anchored
`#[cfg(test)]`, by the same helper `production_slice_cuts_before_cfg_test` and
`production_slice_whole_file_when_no_cfg_test` already prove and that `scripts/coverage-prod.py`
already uses — and SHALL fail naming both the needle and the line when one occurs.

It SHALL live in `tests/doc_contract.rs` and **not** in `src/specs.rs`, because a check written
inside the file it sweeps contains its own needles and can never pass. This is the same
substitution `terminal_seam_names_match_the_gate` already makes: binding needle names to source
text from inside `cargo test` rather than from a gate script.

It SHALL NOT be a `scripts/gates/` script, and SHALL NOT be folded into `NOIO-VIEW`. Both costs
are stated rather than assumed:

- `NOIO-VIEW`'s `PURE` list is the **render seam's**, and `src/specs.rs` is not a view. Adding
  it would move the "ten pure files" figure that `AGENTS.md`, `SPEC.md`, `view-palette`, and
  `responsive-layout` each carry in prose — the five-site cost that putting the module outside
  `src/ui/` exists to avoid, so folding in would defeat the decision it is meant to support.
- A thirty-second script under `scripts/gates/` would move the count
  `openspec/specs/quality-gates/spec.md` states in seven places, whose owning requirement is 201
  lines and would therefore need a full-content `MODIFIED` rewrite carrying no behaviour.

The claim SHALL be falsifiable through this tier's standing mechanism rather than asserted:
planting a `use std::fs;` above the `#[cfg(test)]` line SHALL make it fail, and removing the
plant SHALL make it pass.

This requirement generalises deliberately. The subject is "a pure module outside `src/ui/` that
a `NOIO-VIEW` file calls into" — `src/specs.rs` is the first, and a second such module SHALL
join this check rather than acquire one of its own, because the reason the property matters is
the same in both cases: I/O one call away from a swept file, with every gate green.

#### Scenario: The production slice of `src/specs.rs` carries no I/O or schema name

- **WHEN** `cargo test --test doc_contract` runs on a tree whose `src/specs.rs` reaches no I/O
- **THEN** the claim passes, having read the slice above the file's first `#[cfg(test)]` line
- **AND** the slice is non-empty, so the check cannot pass vacuously against a file it failed to
  read or cut at the wrong place

#### Scenario: An I/O name planted in the production slice fails the claim

- **WHEN** `use std::fs;` is inserted above `src/specs.rs`'s `#[cfg(test)]` line and
  `cargo test --test doc_contract` runs
- **THEN** the claim fails, and its message names both the needle `std::fs` and the line number
- **AND** removing the plant returns the claim to passing, which is the negative control this
  tier requires of a check that is green at HEAD

#### Scenario: An I/O name in the test module alone does not fail the claim

- **WHEN** `src/specs.rs`'s own `#[cfg(test)] mod tests` names `read_to_string` — as a test
  reading a fixture would — and the production slice does not
- **THEN** the claim passes
- **AND** the slice boundary is therefore load-bearing rather than an exemption: it is what lets
  the needles be searched for at all without the searching test matching itself
