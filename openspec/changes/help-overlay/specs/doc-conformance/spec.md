## ADDED Requirements

### Requirement: The documented bindings are bound to the functions that produce them

`SPEC.md` → Keys and `README.md` → Keys both list the pane's bindings in prose, and
`ui::help::INVENTORY` now lists them a third time as data the pane renders. Three lists of
one thing is three places to go stale, and this repository's rule is that a documented
claim with a computable second site is bound to that site inside `cargo test`. This
requirement binds all three.

`tests/doc_contract.rs` SHALL carry a check that:

1. derives the set of bound actions and compares it against `ui::help::INVENTORY`, exactly as
   `binding-inventory` requires. That requirement owns the sweep, the exemption set, and the
   failure directions; this one neither restates nor weakens them, and legs 2 and 3 below are
   what `doc-conformance` adds on top;
2. requires `SPEC.md` → Keys' **key** table to name every `input` string `INVENTORY`
   holds, and to name no key `INVENTORY` does not;
3. requires `README.md` → Keys to name the same set of `input` strings.

Legs 2 and 3 compare the prose against `INVENTORY`, not against the swept functions, and
that is deliberate: leg 1 already binds `INVENTORY` to the functions, so binding the prose
to `INVENTORY` makes the inventory the single hinge every other list turns on, and a
binding added to the driver fails leg 1 before it can reach legs 2 and 3 at all.

Each of the three legs SHALL fail loudly rather than vacuously. A missing `### Keys`
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
- **AND** when the inventory row is added but `SPEC.md` → Keys is not, leg 2 fails naming
  the `input` string and the document
- **AND** when `SPEC.md` is updated but `README.md` is not, leg 3 fails on the same terms

#### Scenario: The documented key set and the inventory agree at HEAD

- **WHEN** the check is run against the tree at the end of this change
- **THEN** all three legs pass
- **AND** leg 2's extraction finds `SPEC.md` → Keys' key table by its own header row, not
  by position, and reports at least one row
- **AND** leg 3's extraction finds `README.md` → Keys the same way

#### Scenario: A gutted document fails as a broken control rather than a clean tree

- **WHEN** the check is run against a copy of the tree in which `SPEC.md`'s `### Keys`
  section has been deleted, and again against one in which its key table has been reduced
  to a header row and a separator row
- **THEN** both runs fail with a message naming the missing section or the empty table
- **AND** neither reports a pass, so a document that stopped documenting is a failure and
  not an agreement between two empty sets

### Requirement: The new module's documentation is bound where a binding exists, and hand-written where none does

`src/ui/help.rs` is a **submodule**, and the two existing map checks do not see submodules.
`tests/doc_contract.rs`'s `pub_mod_names` reads `src/lib.rs`'s top-level `pub mod`
declarations, and `SPEC.md`'s Module map carries a single `ui` row for all thirteen files
under `src/ui/`; § Unit-tested modules is bound to "every **declared** module" on the same
terms. Adding `src/ui/help.rs` therefore fails **neither** check, exactly as adding
`src/ui/palette.rs` failed neither.

This requirement states that plainly rather than claiming a binding that does not exist. An
earlier draft of it asserted both checks would fail until the documents named `ui::help`,
which is false and would have made its own scenario unfalsifiable — a guard that is believed
and cannot fire is worse than no guard, which is this capability's own standing rule.

What SHALL be machine-bound is what has a computable second site:

- `scripts/gates/noio-view.sh`'s `PURE` list SHALL hold **ten** entries and
  `scripts/gates/colwidth.sh`'s **nine**, both naming `src/ui/help.rs`, and both gates SHALL
  fail when it is absent — `view-palette` owns that leg and this change extends it.
- **`AGENTS.md`'s** "the pure set is **nine** files" SHALL read **ten** and SHALL list
  `src/ui/help.rs`. The gate output is the second site: a prose count that disagrees with
  `NOIO-VIEW OK: 10 pure files` is a drift a reader can settle in one command. `SPEC.md` is
  **not** named here, and the omission is deliberate: it carries no pure-set count and no
  pure-view file list at all. Its § Architecture render-seam passage states the property in
  prose — "Views are pure functions … They perform no I/O" — and enumerates nothing, so there
  is no sentence in it to move from nine to ten. `AGENTS.md:339` is the crate's only prose
  copy of that list.
- **`AGENTS.md`'s** "`tests/doc_contract.rs` carries **nine** further claims" SHALL read
  **ten**, since this change adds the inventory/key-table binding to that file. `SPEC.md`
  § Doc-conformance checks carries the same nine claims as an unnumbered **bullet list** and
  no count, so what it SHALL gain is a **tenth bullet** naming the new binding — not a
  changed numeral.

Naming `SPEC.md` in either bullet would have mandated an edit to a sentence that does not
exist, which is the unfalsifiable guard this capability's own standing rule forbids and which
the paragraph above already refuses once. It was caught by executing the greps rather than by
re-reading the bullet.

What SHALL be hand-written, with no check claimed for it: `SPEC.md`'s Module map `ui` row
and § Unit-tested modules SHALL mention the overlay and the inventory in prose, as
documentation. Extending the two checks to submodule granularity is **not** in this change's
scope — the `ui` row is a prose cell describing responsibilities, not a file list, and
binding it to file names would be a fragile check written to satisfy a sentence.

#### Scenario: The submodule is invisible to both map checks, and that is asserted rather than assumed

- **WHEN** `src/ui/help.rs` is added and neither `SPEC.md`'s Module map nor § Unit-tested
  modules is touched
- **THEN** `tests/doc_contract.rs`'s module-map and tested-modules checks both still **pass**,
  because both read `src/lib.rs`'s top-level `pub mod` set and `ui` is already in it
- **AND** a test asserts exactly that — `pub_mod_names(src/lib.rs)` contains `ui` and does not
  contain `help` or `ui::help` — so a future change that extends either check to submodules
  fails this scenario and is told to update this requirement rather than discovering the
  granularity by surprise
- **AND** the checks that **do** fire for this module are `NOIO-VIEW` and `COLWIDTH`, covered
  by the scenario below

#### Scenario: The gate lists and the prose counts agree

- **WHEN** `scripts/gates/noio-view.sh` and `scripts/gates/colwidth.sh` are run against
  the tree at the end of this change
- **THEN** the first reports ten pure files and the second nine
- **AND** both include `src/ui/help.rs`, and both fail when it is removed from their
  `PURE` list rather than reporting a clean tree over the remaining files
