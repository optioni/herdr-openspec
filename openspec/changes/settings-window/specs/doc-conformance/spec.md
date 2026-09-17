## ADDED Requirements

### Requirement: `src/settings.rs`' freedom from I/O is the sixteenth claim

`tests/doc_contract.rs` SHALL carry a **sixteenth** claim: that the production slice of
`src/settings.rs` names no filesystem, process, environment, network, or standard-I/O API, no
clock, and no `ratatui` type.

It is owed for exactly the reason the eleventh (`src/specs.rs`) and the fourteenth
(`src/integration.rs`) are owed. `src/settings.rs` is a pure module deliberately placed
**outside** `src/ui/`, so that adding it moves neither `NOIO-VIEW`'s pure-file count nor
`COLWIDTH`'s. The cost of that placement is that **no `make gates` script sweeps it at all**,
and this claim is what replaces the sweep.

The `ratatui` needle is this claim's own addition, beyond what the eleventh requires: the
module produces the values a view renders, so a `Span`, a `Style`, or a `Line` reaching it
would put rendering decisions on the wrong side of the seam that `PALETTE` and `MDSEAM`
already police from the other direction. `src/integration.rs`' claim forbids the render
crate's own types on the same reasoning.

The check SHALL read the file's production slice — the text above its first line-anchored
`#[cfg(test)]`, by the same helper the eleventh claim uses — and SHALL fail naming both the
needle and the line when one occurs.

Adding this claim moves **four** sites, and `cargo test --test doc_contract` stays red until
all four agree: `CLAIM_COUNT` in `tests/doc_contract.rs` from 15 to 16; `CLAIM_COUNT_WORDS`,
today a fixed `[(&str, usize); 6]` topping out at `("fifteen", 15)`, extended to seven entries
with `("sixteen", 16)`; `AGENTS.md`'s "fifteen further claims" and its enumerated list; and a
new bullet under `SPEC.md` → `### Doc-conformance checks`. That section holds one more bullet
than it has claims — the last, "A claim with no second site is argued in review, not checked",
is a meta-statement — so the new bullet is inserted **above** it.

#### Scenario: A planted I/O name fails the claim

- **WHEN** `use std::fs;` is added above `src/settings.rs`' first line-anchored `#[cfg(test)]`
- **THEN** `cargo test --test doc_contract` fails, naming both the needle and the line
- **AND** removing it makes the claim pass again
- **AND** the same holds for a planted `ratatui::style::Style` and a planted `Instant::now()`

#### Scenario: An I/O name in the test module alone does not fail the claim

- **WHEN** `src/settings.rs`' own `#[cfg(test)] mod tests` names `read_to_string` — as a test
  reading a fixture would — and the production slice does not
- **THEN** the claim passes
- **AND** the slice boundary is therefore load-bearing rather than an exemption

#### Scenario: The claim count is bound at all four sites

- **WHEN** `CLAIM_COUNT` is raised to 16 but `CLAIM_COUNT_WORDS` is left at six entries
- **THEN** `agents_md_claim_count` returns `Err` on the word "sixteen" rather than passing
- **AND** raising `CLAIM_COUNT` while leaving `AGENTS.md` at "fifteen further claims" fails
  with a message naming both counts
- **AND** adding the `SPEC.md` bullet below the trailing meta-statement rather than above it
  leaves that statement no longer last, which the section's own ordering assertion catches
