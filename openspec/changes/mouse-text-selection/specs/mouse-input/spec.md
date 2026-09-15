## MODIFIED Requirements

### Requirement: `SPEC.md` names the mouse bindings and the drag-to-select cost

`SPEC.md` → Keys SHALL carry a mouse table naming each binding this capability defines —
the wheel over each region, the click on a change row, the second click, the click on a
list section header, the click on a tab cell, the click on an artifact-section header, and
the click on any other content row of a foldable artifact: **seven**, five before
`foldable-spec-sections` — and SHALL state that enabling mouse capture costs the terminal's
own drag-to-select.

**The bypass sentence is corrected here, because this change measured it and it was wrong.**
`notes/probe.sh` was run twice — Ghostty inside a Herdr pane and Ghostty bare — and
`Option`+drag was observed **not** to restore selection under any mode set, while
`Shift`+drag restored it under every one, today's included. `SPEC.md` SHALL therefore name
`Shift` as the measured bypass, SHALL name Ghostty on macOS as what it was measured on, and
SHALL NOT claim the `Option` bypass at all — it is an iTerm2 and Terminal.app convention this
project has no measurement for. iTerm2, Terminal.app, and the Linux terminals are unmeasured
and the document SHALL say so rather than generalising from one terminal.

`SPEC.md` SHALL further state that the cost is now **escapable without the modifier**: `m`
releases capture for as long as the reader wants it released, after which plain drag-selection
behaves exactly as it does with no reporting at all. Both are documented, because they solve
the problem at different costs — the modifier needs no state change and works mid-gesture, the
toggle needs no modifier and works on a terminal whose bypass this project never measured.

The same measurement established two facts the document SHALL NOT contradict: narrowing the
enabled DEC mode set does **not** restore plain drag-selection, and Herdr's own mouse handling
is not involved.

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

- **WHEN** `tests/doc_contract.rs` reads `SPEC.md` → Keys' mouse table and its surrounding
  drag-to-select paragraph
- **THEN** the paragraph names `Shift` and names the terminal it was measured on
- **AND** the literal `Option` does not appear as a claimed bypass anywhere in it, so the
  falsified sentence cannot be reintroduced by a later edit without failing `cargo test`
- **AND** the paragraph names `m` as the toggle that escapes the cost without a modifier

## ADDED Requirements

### Requirement: No gesture arrives while capture is released

While `Dashboard::mouse_capture` is `false` the terminal is not reporting, so
`ui::driver::mouse_action` SHALL NOT be reached at all. The pane SHALL NOT compensate: it
SHALL NOT synthesise gestures, SHALL NOT keep a shadow pointer position, and SHALL NOT change
how any key behaves. Releasing capture withdraws a second way to reach what the keys already
reach, exactly as a refused start-up capture does.

`ui::driver::mouse_action` itself SHALL be unchanged by this capability — it stays a pure,
total function of the event and the frame, with no capture parameter and no branch on one.
Whether a mouse event arrives is the terminal's decision and the loop's, never the resolver's.

#### Scenario: The resolver gains no capture parameter

- **WHEN** `ui::driver::mouse_action`'s signature is read
- **THEN** it names no capture state, so a released capture cannot change how a gesture that
  does arrive is resolved
- **AND** every scenario this capability already specifies for the resolver passes unchanged

#### Scenario: A released capture withdraws the gestures and nothing else

- **WHEN** a `Dashboard` at `Route::Detail` with a scrolled, folded artifact has capture
  released and is then rendered at 120x20 and at 60x20
- **THEN** each rendered frame is byte-identical to the same dashboard with capture entered,
  apart from the footer's released-capture badge
- **AND** the `Mouse` group of `ui::help::INVENTORY` is still rendered in the help overlay in
  full, because the gestures are unavailable for the moment rather than removed from the pane
