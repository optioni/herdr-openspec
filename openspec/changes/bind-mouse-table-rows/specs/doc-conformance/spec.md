## ADDED Requirements

### Requirement: `SPEC.md`'s mouse table is bound by executing `mouse_action`, row by row

`documented_mouse_actions` parses `SPEC.md` → Keys' mouse table for backticked `Action::`
names, `mouse_action_body` parses `src/ui/driver.rs` for the same, and
`mouse_bindings_match_spec_md` asserts the two **sets** are equal. That check cannot see a row
whose *prose* is wrong, because prose is not in either set. `mouse-text-selection` left four
false statements in the table past a green `make check` (`git show 9beadc2 -- SPEC.md`).

The information needed is largely computed and then discarded. `sweep_mouse_actions` executes
`mouse_action` at **every cell** of three frames — the wide frame at `Route::List`, the narrow
frame at both routes — across **fourteen** `MouseEventKind` values, and reduces the result to
a `BTreeSet<String>` of action names, discarding everything that distinguishes one documented
row from another.

`tests/doc_contract.rs` SHALL retain that discarded detail and bind the table to it.

**The claim.** The sweep SHALL produce a set of **claims**. A claim carries four axes:

1. the `MouseEventKind` dispatched;
2. the **overlay state**, as an explicit axis — not a value written into the zone slot;
3. the `Zone` the cell resolved to, as its **variant name**, payload discarded;
4. the **outcome**: the `Action` variant name **together with its payload's constructor
   name** where it has one — `Click(Change)`, `Click(Section)`, `Click(DetailHeader)`,
   `Select(Begin)`, `Select(Extend)`, `SelectTab`, `Ignore`.

Axis 4 is load-bearing and SHALL NOT be reduced to the bare variant name. `action_name`
collapses every `Click(_)` to `"Click"`, and under that collapse three separate documented
rows — a click on a change row, a second click on the row already selected, and a click on a
section header — all reduce to one claim, as do the artifact-section-header click and the
content-row press that `mouse-text-selection` confused. Measured at HEAD, the bare-name form
yields 104 distinct triples of which 18 are non-`Ignore`; the stale row that change actually
deleted claims one of them, so under the bare-name form it is **not** vacuous and the check
passes. The payload discriminant is what makes the vacuity direction able to fire at all.

**The dashboard-fixture axis.** `mouse_action` is a total function of a `Dashboard`, a `Rect`
and a `MouseEvent` — the `Dashboard` included. Its `Drag(MouseButton::Left)` arm returns
`Action::Select` from `Zone::ListRow`, `DetailTab`, `List`, `Detail` and `Outside` when
`dashboard.selection.is_some()`, and `Action::Ignore` otherwise; today's single fixture pins
`selection: None`, so the clamp behaviour the table documents is never observed and a row
describing it would be reported vacuous and be right to. The sweep SHALL therefore run over an
**explicitly listed set of dashboard fixtures**, at minimum one with no selection and one with
a selection present, and the set's length SHALL be asserted in the check's own source on the
terms `EXEMPT_ACTIONS` is pinned at two. A row resting on a dashboard precondition no fixture
varies SHALL NOT be tolerated silently: either the fixture set gains it, or the row is named in
a pinned, counted exemption list.

**The documented set.** Each row SHALL name, in backticks, **one or more** `Zone` variants —
the wheel rows legitimately span several, `Zone::List` and `Zone::ListRow` for the list region
and `Zone::Detail`, `Zone::DetailTab` and `Zone::DetailRow` for the detail region — together
with the outcome in axis-4 form, naming the payload constructor where the `Action` has one. A
row's claims are the cross product of its gestures, its zones and its outcomes. A backticked
`Zone` token that names no real variant SHALL be an error naming the row and the token, never
left to surface as an unexplained vacuity.

A row's **gesture** SHALL be derived from a **closed, explicitly-listed** vocabulary mapping a
leading phrase of the Gesture cell to one or more `MouseEventKind` values. The vocabulary SHALL
cover the phrases the real table uses — `Wheel down`, `Wheel up`, `Left click`, `Left drag`,
`Anything else` — SHALL be pinned by length, and SHALL error by row and phrase on anything
unlisted. Semantic parsing of the cell's English SHALL NOT be attempted.

**The catch-all row.** **Exactly one** row SHALL be the catch-all: the row that names no
`Zone` and whose only outcome is `Ignore`. It SHALL claim exactly those claims no other row
claims, it SHALL claim at least one, and a **second** zone-less `Ignore`-only row SHALL be an
error rather than a silently accepted duplicate. The catch-all's own prose is **not** parsed,
which is the reason two of `mouse-text-selection`'s four false statements — both inside that
row — are out of this check's reach.

**The assertion.** The check SHALL hold in **both** directions:

1. every claim the sweep observed is covered by at least one row; one covered by none SHALL
   fail as **undocumented**, naming the gesture, the overlay state, the zone and the outcome;
2. every row covers at least one observed claim; a row covering none SHALL fail as
   **vacuous**, naming the row's own text and the claims actually observed at that row's zones,
   so the reader is told what the row should have said.

Both directions SHALL be reported when both fail, rather than the first masking the second.

**The admitted limitation.** Two rows may legitimately share one claim, and the vacuity
direction cannot tell them apart. At HEAD the one such pair is the click on a change row and
the second click on the row already selected: both produce `Click(Change)`, because "a second
click opens the detail" is decided in `Dashboard::apply`, not in `mouse_action`. Such pairs
SHALL be listed by name in the check's own source and pinned by count, so a third row joining
the collision fails rather than passing unnoticed.

The existing name-set equality SHALL be kept as the check's **first** leg, so a plain
vocabulary mismatch is reported in its existing terms before the stricter legs run.

`CLAIM_COUNT` SHALL remain **13**: this strengthens an existing claim rather than adding a new
one, so the three sites pinned to that count do not move.

#### Scenario: A row describing a removed binding fails as vacuous

- **WHEN** the check runs against a copy of the tree in which the mouse table carries a row
  reading `` | Left click on any other row of a foldable artifact's content | `Action::Click`
  naming `Target::DetailLine` in `Zone::DetailRow` | `` — the row `mouse-text-selection`
  actually deleted — while `mouse_action` produces `Select(Begin)` there
- **THEN** the check fails naming that row as covering no observed claim
- **AND** the failure names the claims actually observed at `Zone::DetailRow` under a left
  press — `Click(DetailHeader)` and `Select(Begin)` — so the reader is told what the row
  should have said
- **AND** leg 1, the name-set equality, is **not** what fails: `Action::Click` is still
  produced elsewhere in the table and in `mouse_action` alike, which is exactly why the
  shipped check passed this row

#### Scenario: Two rows that differ only in their payload are told apart

- **WHEN** the check runs against a copy of the tree in which the row documenting the click on
  a section header names `Target::Change` instead of `Target::Section`
- **THEN** the check fails: no observed claim pairs `Zone::ListRow` with `Click(Section)` any
  longer from that row, and the row claiming `Click(Change)` twice does not cover it
- **AND** this scenario fails under the axis-4 form and passes under the bare-variant form,
  which is the difference the payload discriminant exists to make

#### Scenario: A binding added with no row fails as undocumented

- **WHEN** the check runs against a copy of the tree in which the observed claim set carries
  `(Down(Middle), overlay closed, Zone::List, ToggleHelp)`, and no table row covers it
- **THEN** the check fails naming the gesture, the overlay state, the zone, and `ToggleHelp`
- **AND** the catch-all row does not absorb it, because the catch-all covers only claims whose
  outcome is `Ignore`

#### Scenario: The table and the pane agree at HEAD

- **WHEN** the check runs against the tree at the end of this change
- **THEN** every leg passes
- **AND** all fourteen `MouseEventKind` values appear in the observed claim set
- **AND** all six `Zone` variants appear in it, and the overlay-open axis is represented
- **AND** the wheel-over-an-open-overlay row added by this change covers
  `(ScrollDown, overlay open, ScrollDown)` and `(ScrollUp, overlay open, ScrollUp)`, which
  before this change were bound by no row at all
- **AND** no row is vacuous and no claim is uncovered, and the catch-all covers at least one

#### Scenario: The clamp is observed rather than reported vacuous

- **WHEN** the check runs at HEAD with the dashboard-fixture axis in place
- **THEN** the fixture whose `selection` is present yields
  `(Drag(Left), overlay closed, Zone::List, Select(Extend))` among its claims
- **AND** the table's drag row, which documents the clamp, covers it and is not vacuous
- **AND** with the selection-absent fixture alone that claim is absent, which is the state the
  check would have shipped in without this axis

#### Scenario: An unrecognised gesture phrase is an error, not a silent skip

- **WHEN** the check runs against a copy of the tree in which a Gesture cell begins
  `Left quadruple-click`, a phrase the vocabulary does not list
- **THEN** the check fails naming the row and the unrecognised phrase
- **AND** the row is not skipped, so a typo cannot quietly remove that row from both
  directions of the assertion

#### Scenario: A mistyped zone token is named rather than left as an unexplained vacuity

- **WHEN** the check runs against a copy of the tree in which a row names `` `Zone::DetailRows` ``
- **THEN** the check fails naming the row and the token, and states that it matches no `Zone`
  variant
- **AND** the message is not the vacuity message, which would point the reader at the wrong
  problem

#### Scenario: Zone-less rows are rejected, and a second catch-all is an error

- **WHEN** the check runs against a copy of the tree in which a row naming `Action::SelectTab`
  carries no backticked `Zone`
- **THEN** the check fails naming that row as missing its zone, and states that only the single
  `Ignore`-only catch-all may omit one
- **AND** when a second zone-less `Ignore`-only row is added, the check fails naming both rows,
  rather than accepting one and ignoring the other

#### Scenario: A gutted table fails as a broken control

- **WHEN** the check runs against a copy of the tree in which the mouse table is reduced to its
  header row and separator row, and again against one in which the `| Gesture | Action |`
  header is deleted outright
- **THEN** both runs fail with a message naming the empty table or the missing header
- **AND** neither reports a pass, on exactly the terms `documented_mouse_actions` already holds
  to — an empty documented set is never compared against an empty observed set

#### Scenario: The overlay axis keeps the two passes apart

- **WHEN** the check runs against a copy of the tree in which the row documenting the dismissing
  click outside the help band names the overlay-closed state instead of overlay-open
- **THEN** the check fails, because no closed-overlay cell produces `ToggleHelp` and the
  open-overlay claims are left uncovered
- **AND** both failures are reported — the row as vacuous and the overlay claims as
  undocumented — rather than one masking the other

#### Scenario: A third row joining a known collision fails

- **WHEN** the check runs against a copy of the tree in which a third row also claims
  `(Down(Left), overlay closed, Zone::ListRow, Click(Change))`, beyond the two listed in the
  pinned collision list
- **THEN** the check fails naming the collision list's pinned count and the row that exceeded it
- **AND** the two listed rows continue to pass, so the limitation is bounded rather than open

## MODIFIED Requirements

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
2. requires `SPEC.md` → Keys' **key** table to name every key `INVENTORY` holds, and to name
   no key `INVENTORY` does not;
3. requires `README.md` → Keys to name the same set.

Legs 2 and 3 compare **key atoms**, not `input` strings verbatim, and the normalisation SHALL
be stated here rather than left to the implementer — an earlier draft said "every `input` string
`INVENTORY` holds", which is unsatisfiable against either document as written and was caught by
running the comparison rather than by re-reading it. The rule is:

- The **`Mouse`** group is excluded. Its gestures are bound by the mouse table's own check
  against `SPEC.md` → Keys' **mouse** table, which is a different table with a different
  grammar, and `README.md` documents no gesture at all.
- An `INVENTORY` `input` is split on `" / "` into atoms, so `j / ↓` contributes `j` and `↓`.
  This is what lets one overlay row stand for a key and its arrow synonym without forcing the
  prose to split into two rows.
- A document's atoms are the **backticked spans in the Key column** of its Keys table, the
  table bounded by its own contiguous `|` rows — never a scan of the whole document, which
  picks up every backticked identifier in it.
- Exactly **two** normalisations are permitted, and they SHALL be named in the check's own
  source: the bare word `arrows` in a Key cell contributes `↑` and `↓`; and a backticked pair
  joined by an en-dash, `` `1`–`9` ``, contributes the single range atom `1–9` rather than the
  two endpoints. No third alias SHALL be added — a future binding whose prose spelling does not
  atomise to its `INVENTORY` spelling SHALL be made to agree by editing the document, not by
  growing this list. The check SHALL assert the alias table's length is two, on the same terms
  leg 1's exemption set is pinned at two.

Under that rule the two documents SHALL gain four things, which are real gaps rather than
artifacts of the comparison: `?` in both (it is this change's own binding); `Space` in
`README.md`, which omits it while `SPEC.md` carries it; and `Backspace` in **both**, which each
document mentions only inside the `/` row's prose and neither names as a key of its own,
though `INVENTORY`'s `While filtering` group binds it to `FilterPop`.

Legs 2 and 3 compare the prose against `INVENTORY`, not against the swept functions, and
that is deliberate: leg 1 already binds `INVENTORY` to the functions, so binding the prose
to `INVENTORY` makes the inventory the single hinge every other list turns on, and a
binding added to the driver fails leg 1 before it can reach legs 2 and 3 at all.

Each of the three legs SHALL fail loudly rather than vacuously. A missing `### Keys`
section, a missing table, or a table from which every row has been deleted SHALL be an
error naming what was missing, on exactly the terms `documented_mouse_actions` already
holds to — it returns `Err` naming the absence rather than comparing an empty set against
an empty set and passing.

`SPEC.md` → Keys' **mouse** table is bound separately, and that binding is **executed**
rather than parsed: it is stated in full by "`SPEC.md`'s mouse table is bound by executing
`mouse_action`, row by row" above. The earlier framing here — that the mouse leg "parses
source text for `Action::` variants" while the key leg "executes the function", and that the
disagreement between the two styles is "itself informative" — is **superseded**. Parsing
source text was the weaker of the two and let four false statements survive a green
`make check`; the name-set comparison survives only as the first leg of the executed check,
where it reports a plain vocabulary mismatch in its existing terms before the stricter legs
run. Both tables are now bound by executing the function that produces the behaviour.

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
- **AND** the atom sets agree exactly, in both directions, with the alias table at length two —
  no residual difference is tolerated and none is exempted
- **AND** the extraction is bounded to the table's own contiguous `|` rows: a check that
  scanned the whole of `SPEC.md` would collect every backticked identifier in it — sixty-odd
  role names, CLI fragments and paths — and pass vacuously in the doc-has-extra direction,
  which is the failure this bullet exists to forbid

#### Scenario: A gutted document fails as a broken control rather than a clean tree

- **WHEN** the check is run against a copy of the tree in which `SPEC.md`'s `### Keys`
  section has been deleted, and again against one in which its key table has been reduced
  to a header row and a separator row
- **THEN** both runs fail with a message naming the missing section or the empty table
- **AND** neither reports a pass, so a document that stopped documenting is a failure and
  not an agreement between two empty sets

#### Scenario: The key legs are unaffected by the mouse table's stronger binding

- **WHEN** the mouse table's executed check is added and the key legs are re-run unchanged
- **THEN** legs 1, 2 and 3 pass exactly as before
- **AND** the `Mouse` group remains excluded from leg 2's and leg 3's atom comparison, since
  `README.md` still documents no gesture
