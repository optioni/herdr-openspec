## Why

`mouse_bindings_match_spec_md` is a **set equality over backticked `Action::` names**. It
compares the names `SPEC.md` → Keys' mouse table mentions against the names
`ui::driver::mouse_action`'s source text mentions, and nothing else. So the table can
describe a gesture the pane no longer has, describe it with the wrong effect, or omit a
gesture entirely, and the check stays green as long as the *vocabulary* on both sides
matches.

That is not hypothetical. `mouse-text-selection`'s Change Review found four false statements
in that one table, past a green `make check`, caught only by a human reading it
(`git show 9beadc2 -- SPEC.md`): a row still documenting the removed `Target::DetailLine`,
no row at all for the press that replaced it, and "any drag" and "the detail content area
when the selected artifact is not foldable" both still listed under `Action::Ignore`. The
repository's own rule is that a documented claim with a computable second site is bound to
that site inside `cargo test`; this claim has one and is not bound to it.

## What Changes

- `sweep_mouse_actions` stops collapsing its result to a set of names. It already executes
  `mouse_action` at every cell of three frames across fourteen `MouseEventKind`s; it will
  retain a **claim** per cell — the information each table row makes a statement about, and
  which is currently discarded.
- A claim carries four axes: the gesture, the **set** of `Zone` variants the row covers, the
  overlay state as an explicit axis, and the outcome as the `Action` variant **plus its
  payload's constructor name** (`Click(Change)`, `Click(Section)`, `Click(DetailHeader)`,
  `Select(Begin)`, `Select(Extend)`). The payload discriminant is the axis that makes the
  check able to tell one documented row from another at all.
- The sweep gains a **dashboard-fixture axis**, pinned and counted, because `mouse_action` is
  a function of the whole `Dashboard`: its `Drag` arm branches on `selection.is_some()`, and
  today's single fixture pins `selection: None`, so the clamp behaviour `SPEC.md` documents
  is never observed.
- Each row of `SPEC.md` → Keys' mouse table names the `Zone` variants it applies to, in
  backticks.
- **Two rows are added** for the wheel over an open help overlay, which the pane binds
  (`src/ui/driver.rs:340-341`) and the table has never documented — it lives in a prose
  paragraph the extractor does not read. One row per wheel direction, because the closed
  gesture vocabulary maps `Wheel down` and `Wheel up` to one `MouseEventKind` each, exactly
  as the table already documents the list and detail wheels.
- `mouse_bindings_match_spec_md` becomes a **two-way coverage** assertion: every claim the
  sweep observes is covered by some row, and every row covers at least one observed claim. A
  row describing a binding that no longer exists fails as **vacuous**; a binding with no row
  fails as **undocumented**. The existing name-set equality is kept as leg 1.

## Non-Goals

- **No behaviour change.** `mouse_action`, `Action`, `Zone`, and every binding the pane has
  are untouched. This change adds no gesture and removes none.
- **Not the key table.** `SPEC.md` → Keys' *key* bindings are already bound from three sides
  through `ui::help::INVENTORY`, which is row-level data rather than prose.
- **No new claim.** `tests/doc_contract.rs`'s `CLAIM_COUNT` stays **13**. This strengthens an
  existing claim rather than adding a fourteenth, so the three sites pinned to that count do
  not move — and the `SPEC.md` bullet the documentation group rewrites must stay **one**
  bullet, since that list is counted.
- **The catch-all row's prose is not parsed.** Two of the four false statements
  `mouse-text-selection` left behind lived inside the catch-all row, and this check does not
  reach them. It catches the other two. That limit is stated rather than glossed.
- Not a general prose parser. Semantic parsing of English table cells is explicitly rejected
  in favour of a small, closed, explicitly-listed gesture vocabulary that fails loudly on an
  unrecognised phrase.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `doc-conformance`: "The documented bindings are bound to the functions that produce them"
  gains a stronger mouse leg, stated in full as a new requirement — the mouse table is bound
  by **executing** `mouse_action` rather than by parsing its source text.
- `mouse-input`: "`SPEC.md` names the mouse bindings and the drag-to-select cost" states
  normatively that the leg "extracts the backticked `Action::<Variant>` names from the table
  and compares that set against `mouse_action`'s body, as a **set equality**". This change
  makes that false, so the requirement moves with it.
- `binding-inventory`: its contrast paragraph justifies `INVENTORY`'s binding as "one layer
  stronger" than the mouse table's, because "that check **parses** a markdown table and
  compares it against parsed source". After this change both are executed, and the contrast
  no longer holds.

## Impact

- `tests/doc_contract.rs` — `sweep_mouse_actions`, `swept_mouse_action_names`,
  `documented_mouse_actions`, and `mouse_bindings_match_spec_md`. `swept_mouse_action_names`
  has **three** direct callers — `swept_action_names`,
  `select_is_the_only_mouse_only_action_by_name_and_count`, and
  `the_sweep_covers_the_mouse_under_both_overlay_states`, the last of which asserts the closed
  and open name sets by equality and is the one most likely to break — all of which keep a
  name-set view over the richer data.
- `SPEC.md` § Keys — each mouse table row gains the `Zone` variants it covers, and **two rows
  are added** for the overlay wheel. § Testing and quality gates → Doc-conformance checks gains
  a reworded mouse bullet, still exactly one bullet.
- `AGENTS.md` — the contract-tier paragraph's mouse item. `CLAUDE.md` is a symlink to it, not
  a second file.
- No file under `src/` is touched. The coverage export names 28 files, all under `src/`, so
  neither floor moves.
- Roadmap: unplanned post-roadmap work, like `doc-conformance`, `foldable-spec-sections`,
  `help-overlay`, `gate-script-count` and `header-progress-bar` before it. `doc-conformance`
  built the contract tier without noticing that one of its own members proved less than it
  appeared to.
