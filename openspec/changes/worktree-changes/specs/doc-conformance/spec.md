## ADDED Requirements

### Requirement: `src/worktrees.rs`' freedom from I/O is the seventeenth claim

`tests/doc_contract.rs` SHALL carry a **seventeenth** claim: that the production slice of
`src/worktrees.rs` names no filesystem, process, environment, network, or standard-I/O API, no
clock, no `ratatui` type, and no CLI handle — none of `GitCli`, `OpenspecCli`, `HerdrCli`,
`git_cli_via`, or `crate::cli`.

It is owed for exactly the reason the eleventh (`src/specs.rs`), the fourteenth
(`src/integration.rs`), and the sixteenth (`src/settings.rs`) are owed. `src/worktrees.rs` is a
pure module placed **outside** `src/ui/`, so adding it moves neither `NOIO-VIEW`'s pure-file
count nor `COLWIDTH`'s, and **no `make gates` script sweeps it**; this claim replaces the sweep.
The CLI-handle needles are this claim's own addition, on `src/integration.rs`' model: the module
parses the stdout of `git` commands the refresh worker runs, and a handle reaching it would move
a blocking call into code the design keeps pure. Canonicalizing a worktree path is a filesystem
call and SHALL stay in the worker, which passes the canonical paths in.

The check SHALL read the file's production slice — the text above its first line-anchored
`#[cfg(test)]`, by the same helper the eleventh claim uses — and SHALL fail naming both the
needle and the line when one occurs.

Adding this claim moves the same **four** sites the sixteenth moved, and `cargo test --test
doc_contract` stays red until all four agree: `CLAIM_COUNT` from 16 to 17; `CLAIM_COUNT_WORDS`
extended with `("seventeen", 17)`; `AGENTS.md`'s "sixteen further claims" and its enumerated
list; and a new bullet under `SPEC.md` → `### Doc-conformance checks`, inserted **above** the
trailing meta-statement.

#### Scenario: A planted I/O name or CLI handle fails the claim

- **WHEN** `use std::fs;` is added above `src/worktrees.rs`' first line-anchored `#[cfg(test)]`
- **THEN** `cargo test --test doc_contract` fails, naming both the needle and the line
- **AND** removing it makes the claim pass again
- **AND** the same holds for a planted `std::fs::canonicalize`, a planted `Instant::now()`, and a
  planted `use crate::cli::GitCli;`

#### Scenario: An I/O name in the test module alone does not fail the claim

- **WHEN** `src/worktrees.rs`' own `#[cfg(test)] mod tests` names `read_to_string` and the
  production slice does not
- **THEN** the claim passes

#### Scenario: The claim count is bound at all four sites

- **WHEN** `CLAIM_COUNT` is raised to 17 while `AGENTS.md` still reads "sixteen further claims"
- **THEN** `cargo test --test doc_contract` fails with a message naming both counts
- **AND** raising it while `CLAIM_COUNT_WORDS` lacks `("seventeen", 17)` makes
  `agents_md_claim_count` return `Err` on that word rather than pass
