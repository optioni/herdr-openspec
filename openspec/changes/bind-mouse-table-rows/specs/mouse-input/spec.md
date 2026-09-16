## MODIFIED Requirements

### Requirement: `SPEC.md` names the mouse bindings and the drag-to-select cost

`SPEC.md` → Keys SHALL carry a mouse table naming each binding this capability defines —
the wheel over each region, the click on a change row, the second click, the click on a
list section header, the click on a tab cell, the click on an artifact-section header, and
the click on any other content row of a foldable artifact: **seven**, five before
`foldable-spec-sections` — and SHALL state that enabling mouse capture costs the terminal's
own drag-to-select **outside the detail content area**.

The table is the pane's **whole** mouse surface, not only this capability's seven: it also
carries rows for bindings other capabilities define, `help-overlay`'s dismissing click and
its overlay wheel among them. A binding the pane has and the table omits is a defect of this
table wherever the binding was defined.

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

**What that test's subject actually is, stated because it is narrower than it reads —
and this paragraph is now superseded.** It read: "the leg extracts the backticked
`Action::<Variant>` names from the table and compares that set against `mouse_action`'s body,
as a **set equality**." That set equality is no longer the whole of the leg. It survives as
the leg's **first** step, reporting a plain vocabulary mismatch in its existing terms; beyond
it the table is bound by **executing** `mouse_action` at every cell of the swept frames and
requiring, in both directions, that every observed behaviour is covered by a row and every row
covers an observed behaviour. `doc-conformance` → "`SPEC.md`'s mouse table is bound by
executing `mouse_action`, row by row" states that binding in full and owns its details.

The reason for the change belongs here, because this requirement is where the narrower
subject was written down as if it were sufficient: a set equality over names cannot see a row
whose **prose** is wrong. `mouse-text-selection` left four false statements in this very table
past a green `make check`, including a row still describing `Target::DetailLine` after that
binding was removed. The narrowness was documented and then relied upon; documenting a
weakness does not make it safe.

**This change inverts the note that stood here.** `foldable-spec-sections` recorded that its
two new gestures resolved to `Action::Click` and `Action::Ignore`, both already named, so the
leg was green before and after and could not fail for its new rows. That is **not** true here:
`text-selection`'s gesture resolves to `Action::Select`, a name the table does not carry, so
the leg goes **red** the moment `mouse_action` can produce it and stays red until
`SPEC.md` → Keys' `| Gesture | Action |` table names it. The leg is therefore this change's
own proof rather than something needing a separate negative control — though the control
`foldable-spec-sections` recorded still stands for the rows it added.

#### Scenario: The documented bindings match the resolver

- **WHEN** `tests/doc_contract.rs` reads `SPEC.md` → Keys' mouse table and compares it
  against the bindings `src/ui/driver.rs`'s resolver actually implements
- **THEN** every documented binding is implemented and every implemented binding is
  documented
- **AND** the check fails when the mouse table is absent, rather than passing vacuously

#### Scenario: The comparison is executed, not parsed

- **WHEN** a row of the mouse table describes a binding `mouse_action` no longer produces,
  while every `Action::` name in the table still appears in `mouse_action`'s source
- **THEN** the check fails, because the row covers no behaviour the executed sweep observed
- **AND** the set-equality step alone would have passed, which is the case that motivated
  replacing it

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
