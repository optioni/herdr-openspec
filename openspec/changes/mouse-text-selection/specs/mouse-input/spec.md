## MODIFIED Requirements

### Requirement: Nothing becomes mouse-only

Every action the mouse can produce SHALL remain reachable by key, and **no key SHALL change
its meaning because of a mouse binding**. `Action::SelectNext`, `Action::SelectPrev`,
`Action::ScrollDown`, and `Action::ScrollUp` are what `j`, `k`, and the arrows already reach
through `Action::Next` and `Action::Prev` at the matching route;
`Action::Click(Target::Section(_))` is what `Space` at `Route::List` already reaches;
`Action::Click(Target::Change(_))` is what `j`/`k` and `Enter` already reach;
`Action::SelectTab(i)` is what `1`–`9`, `[`, and `]` already reach;
`Action::Click(Target::DetailHeader { .. })` is what `Space` at `Route::Detail` reaches, and
`Action::Click(Target::DetailLine(_))` is what `j`/`k` and the arrows reach there.

The clause is narrowed from "no key SHALL change its meaning" to "no key SHALL change its
meaning **because of a mouse binding**", and the narrowing is this change's, stated rather
than left as a contradiction. `foldable-spec-sections` does change one key's meaning —
`Space` at `Route::Detail` folds an artifact section instead of a list section, marked
**BREAKING** in its own proposal — and it does so for reasons that have nothing to do with
the mouse. What this requirement guards is the property it was written for: that adding a
gesture never silently rebinds a key, and that the pane stays fully usable with no mouse at
all. Both remain true — every gesture above names the key that already reached it.

The pane SHALL therefore stay fully usable in a terminal that reports no mouse event at
all, including over SSH, with no feature reachable only by pointer.

**`text-selection` adds the one exemption this requirement has ever had, and it is pinned
rather than reasoned about at each call site.** `Action::Select` has no key that produces
the same effect, and cannot: selecting a span of rendered text is a pointing gesture, and
the reader's only other route to it — the terminal's own `Shift`+drag — is a pointer gesture
too. What this requirement was written to protect is untouched: every *function of the pane*
stays reachable by key, and getting text out of the pane was not a function of the pane
before this change. The exemption SHALL be asserted **by name and by length**, exactly as
`tests/doc_contract.rs` asserts `EXEMPT_ACTIONS`, so a second mouse-only action costs a spec
change rather than passing under a predicate.

#### Scenario: The key table is unchanged

- **WHEN** `action_for` is called with the full table of inputs `list-sections` asserted —
  every key and every near miss, under `filtering` false and again under `filtering` true
- **THEN** each returns exactly the action it returned before this change
- **AND** no new key is mapped: the two mapping tables gain no row

#### Scenario: Every mouse action has a key that produces the same effect

- **WHEN** for each of the four list-and-detail outcomes — advance the selection, retreat
  the selection, scroll the content down, scroll the content up — the dashboard is driven
  once by the mouse action and once by the corresponding key at the corresponding route
- **THEN** the two resulting `Dashboard` values are equal, field for field
- **AND** the same holds for a section toggle driven by `Action::Click(Target::Section(k))`
  against `Space`, and for a tab switch driven by a tab click against the matching digit key


#### Scenario: `Action::Select` is the only mouse-only action, by name and count

- **WHEN** every action `ui::driver::mouse_action` can produce is swept and each is checked
  for a key at any route that produces the same `Dashboard` change
- **THEN** exactly one has none, and it is `Action::Select`
- **AND** the mouse-only set is asserted to hold that one name and to have length one, so a
  second mouse-only action fails `cargo test`
- **AND** every other gesture still names the key that already reached it, so the pane
  remains fully usable with no mouse in a terminal reporting none

#### Scenario: The pane is still complete without a pointer

- **WHEN** a dashboard is driven through a full session — list, filter, detail, tabs, folds,
  agent keys and the help overlay — using keys only
- **THEN** every route, every artifact and every fold state is reachable
- **AND** the only thing unavailable is copying text, which has no pane function behind it

### Requirement: `SPEC.md` names the mouse bindings and the drag-to-select cost

`SPEC.md` → Keys SHALL carry a mouse table naming each binding this capability defines —
the wheel over each region, the click on a change row, the second click, the click on a
list section header, the click on a tab cell, the click on an artifact-section header, and
the click on any other content row of a foldable artifact: **seven**, five before
`foldable-spec-sections` — and SHALL state that enabling mouse capture costs the terminal's
own drag-to-select **outside the detail content area**.

**The bypass sentence is corrected here, because this change measured it and it was wrong.**
`Option`+drag was observed **not** to restore selection under any mode set in Ghostty on
macOS, while `Shift`+drag restored it under every one. `SPEC.md` SHALL name `Shift`, SHALL
name the terminal it was measured on, and SHALL NOT claim the `Option` bypass at all — it is
an iTerm2 and Terminal.app convention this project has no measurement for. iTerm2,
Terminal.app and the Linux terminals are unmeasured and the document SHALL say so.

`SPEC.md` SHALL further record that inside the detail content area the cost no longer
applies, because the pane does the selecting itself, and that the mouse table gains a drag
row. The measured facts it SHALL NOT contradict: narrowing the enabled DEC mode set does
not restore plain drag-selection, `?1003` is load-bearing because drag motion is what the
selection consumes, and Herdr is not involved.

`AGENTS.md`'s terminal-seam rule SHALL name the two capture commands alongside the four
terminal-mode functions it already lists, so the confined set the `NORAW-GREP` gate enforces
and the set the document claims are the same set.

A doc-conformance test SHALL bind both claims to the files that determine them, so a binding
added, removed, or renamed in `ui::driver::mouse_action` without the document following
fails `cargo test`.

**What that test's subject actually is, stated because it is narrower than it reads:** the
leg extracts the backticked `Action::<Variant>` names from the table and compares that set
against `mouse_action`'s body. This change's two new gestures resolve to `Action::Click` and
`Action::Ignore`, both already named, so the leg is green before and after and **cannot fail
for the two new rows**. The rows are still required above; what verifies them is the
resolver's own scenarios, and the leg's own falsifiability SHALL be re-established by a
recorded negative control — delete one documented row, confirm the leg fires, restore it —
rather than inferred from its passing.

#### Scenario: The documented bindings match the resolver

- **WHEN** `tests/doc_contract.rs` reads `SPEC.md` → Keys' mouse table and compares it
  against the bindings `src/ui/driver.rs`'s resolver actually implements
- **THEN** every documented binding is implemented and every implemented binding is
  documented
- **AND** the check fails when the mouse table is absent, rather than passing vacuously

#### Scenario: The documented confined set matches the gate

- **WHEN** `tests/doc_contract.rs` reads the terminal-seam rule's list of confined function
  names from `AGENTS.md` and the `RAW_RE` pattern from `scripts/gates/noraw-grep.sh`
- **THEN** the two name the same six functions —  `enable_raw_mode`, `disable_raw_mode`,
  `EnterAlternateScreen`, `LeaveAlternateScreen`, `EnableMouseCapture`, and
  `DisableMouseCapture`


#### Scenario: The documented bypass names what was measured

- **WHEN** `tests/doc_contract.rs` reads `SPEC.md` → Keys' mouse table and its drag-to-select
  paragraph
- **THEN** the paragraph names `Shift` and names the terminal it was measured on
- **AND** the literal `Option` appears nowhere in it as a claimed bypass, so the falsified
  sentence cannot be reintroduced without failing `cargo test`
- **AND** the mouse table carries a drag row whose action is `Action::Select`
