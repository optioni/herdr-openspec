## ADDED Requirements

### Requirement: The clipboard write's confinement is bound inside `cargo test`

`tests/doc_contract.rs` SHALL carry a **twelfth** claim: that the OSC 52 introducer appears
in `src/ui/terminal.rs` and nowhere else in the crate, `tests/` included. It joins the eleven
already there on the same terms — a documented claim with a computable second site is bound
to that site inside `cargo test`, not left to a human re-reading it.

The claim SHALL fail when its exclusion goes **vacuous**: when `src/ui/terminal.rs` is absent,
or is present and names no OSC 52 sequence. A confinement check that passes because the
confined thing has disappeared is worse than none, because it is believed.

`AGENTS.md`'s "eleven further claims" and `SPEC.md` → § Doc-conformance checks SHALL both move
to twelve, and the count SHALL be bound to the number of claims the test file actually carries
rather than left as prose. Neither site is machine-bound today, which is why both have drifted
before; this change binds them.

#### Scenario: The twelfth claim holds and is falsifiable

- **WHEN** `cargo test --test doc_contract` runs against the tree at the end of this change
- **THEN** the OSC 52 confinement claim passes
- **AND** planting the OSC 52 introducer in any other file under `src/` or `tests/` fails it
- **AND** removing it from `src/ui/terminal.rs` fails it too, rather than passing vacuously

#### Scenario: The documented claim count matches the file

- **WHEN** the claim count stated in `AGENTS.md` and in `SPEC.md` is compared against the
  number of claims `tests/doc_contract.rs` carries
- **THEN** all three agree at twelve
- **AND** the comparison is executed by a test, so a thirteenth claim added without moving the
  prose fails `cargo test` rather than drifting unnoticed
