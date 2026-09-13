## ADDED Requirements

### Requirement: The documented bindings are bound to the functions that produce them

`SPEC.md` → Keys and `README.md` → Keys both list the pane's bindings in prose, and
`ui::help::INVENTORY` now lists them a third time as data the pane renders. Three lists of
one thing is three places to go stale, and this repository's rule is that a documented
claim with a computable second site is bound to that site inside `cargo test`. This
requirement binds all three.

`tests/doc_contract.rs` SHALL carry a check that:

1. derives the set of bound actions by **executing** `ui::app::action_for` and
   `ui::driver::mouse_action` over a swept input space, per `binding-inventory`;
2. requires `ui::help::INVENTORY` to name exactly that set, less the closed two-name
   exemption `binding-inventory` fixes;
3. requires `SPEC.md` → Keys' **key** table to name every `input` string `INVENTORY`
   holds, and to name no key `INVENTORY` does not;
4. requires `README.md` → Keys to name the same set of `input` strings.

Legs 3 and 4 compare the prose against `INVENTORY`, not against the swept functions, and
that is deliberate: leg 2 already binds `INVENTORY` to the functions, so binding the prose
to `INVENTORY` makes the inventory the single hinge every other list turns on, and a
binding added to the driver fails leg 2 before it can reach legs 3 and 4 at all.

Each of the four legs SHALL fail loudly rather than vacuously. A missing `### Keys`
section, a missing table, or a table from which every row has been deleted SHALL be an
error naming what was missing, on exactly the terms `documented_mouse_actions` already
holds to — it returns `Err` naming the absence rather than comparing an empty set against
an empty set and passing.

`SPEC.md` → Keys' existing **mouse** table and its existing binding to `mouse_action`'s
`Action` variants SHALL be left in place and SHALL gain one row for the dismissing click
`mouse-input` adds. The two checks are complementary rather than redundant: that one
parses source text for `Action::` variants and this one executes the function, and a
disagreement between them is itself informative.

#### Scenario: A binding added to the driver and not to the docs fails `make check`

- **WHEN** a key is bound in `action_for` to an existing `Action` for which `INVENTORY`
  holds no row
- **THEN** the check fails naming that action, the swept function it came from, and the
  inventory that omitted it
- **AND** when the inventory row is added but `SPEC.md` → Keys is not, leg 3 fails naming
  the `input` string and the document
- **AND** when `SPEC.md` is updated but `README.md` is not, leg 4 fails on the same terms

#### Scenario: The documented key set and the inventory agree at HEAD

- **WHEN** the check is run against the tree at the end of this change
- **THEN** all four legs pass
- **AND** leg 3's extraction finds `SPEC.md` → Keys' key table by its own header row, not
  by position, and reports at least one row
- **AND** leg 4's extraction finds `README.md` → Keys the same way

#### Scenario: A gutted document fails as a broken control rather than a clean tree

- **WHEN** the check is run against a copy of the tree in which `SPEC.md`'s `### Keys`
  section has been deleted, and again against one in which its key table has been reduced
  to a header row and a separator row
- **THEN** both runs fail with a message naming the missing section or the empty table
- **AND** neither reports a pass, so a document that stopped documenting is a failure and
  not an agreement between two empty sets

### Requirement: The pure view set and the module map name the new module

`SPEC.md`'s Module map SHALL name `ui::help`, and `SPEC.md` → § Unit-tested modules SHALL
name it too. `tests/doc_contract.rs`'s existing module-map and tested-modules checks
SHALL therefore cover it with no change to either check: both are computed from
`src/ui/`'s own contents, which is why adding a module is a `make check` failure before it
is a documentation review.

`AGENTS.md`'s "The pure set is **nine** files" and `SPEC.md`'s corresponding sentence
SHALL both read **ten**, and SHALL list `src/ui/help.rs` among them. `scripts/gates/
noio-view.sh`'s `PURE` list SHALL hold ten entries and `scripts/gates/colwidth.sh`'s
**nine**.

#### Scenario: Adding the module fails the map checks until the documents name it

- **WHEN** `src/ui/help.rs` is added and `SPEC.md`'s Module map is not
- **THEN** `tests/doc_contract.rs`'s module-map check fails naming `ui::help` as present
  in `src/ui/` and absent from the map
- **AND** the tested-modules check fails on the same terms until § Unit-tested modules
  names it
- **AND** both pass once the two passages are written, with no edit to either check

#### Scenario: The gate lists and the prose counts agree

- **WHEN** `scripts/gates/noio-view.sh` and `scripts/gates/colwidth.sh` are run against
  the tree at the end of this change
- **THEN** the first reports ten pure files and the second nine
- **AND** both include `src/ui/help.rs`, and both fail when it is removed from their
  `PURE` list rather than reporting a clean tree over the remaining files
