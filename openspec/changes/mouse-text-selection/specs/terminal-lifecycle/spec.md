## MODIFIED Requirements

### Requirement: Terminal setup and teardown sit behind an injected seam

The crate SHALL define `ui::terminal::TerminalOps`, a trait with exactly seven fallible
operations — `enable_raw`, `enter_alternate`, `enable_mouse`, `disable_mouse`,
`leave_alternate`, `disable_raw`, and `write_clipboard(&str)` — each returning `Result<(), TerminalError>`.
`ui::terminal::CrosstermOps` SHALL be the one implementation that touches a real terminal,
and **six** of its seven methods SHALL do nothing but call the corresponding
`ratatui::crossterm` function or command and map its error. The seventh,
`write_clipboard`, SHALL write an **OSC 52** sequence to stdout directly, because
crossterm models no clipboard command at all. It is the one method whose body is an
escape sequence rather than a call, and it lives here for exactly the reason the other
six do: `src/ui/terminal.rs` is the only file in the crate permitted to name a terminal
escape, so confining the clipboard write here adds no new seam and no new exemption. No decision, no ordering, and no state SHALL live
inside `CrosstermOps`: the ordering lives in `TerminalGuard`, which is what makes it testable
without a terminal.

`TerminalError` SHALL carry the failing operation's name and the underlying error's
`Display` text, and SHALL NOT be `std::io::Error`, so that a test double can produce one
without fabricating an I/O error.

No other module SHALL name a crossterm terminal-mode function, and **no other module
SHALL name the OSC 52 introducer**; a doc-conformance check SHALL bind that second
confinement the way the first is bound.

No other module SHALL name a crossterm terminal-mode function. `enable_raw_mode`,
`disable_raw_mode`, `EnterAlternateScreen`, `LeaveAlternateScreen`, `EnableMouseCapture`,
and `DisableMouseCapture` SHALL appear in `src/ui/terminal.rs` and nowhere else in the
crate, `tests/` included.

#### Scenario: The real implementation is the only place naming a terminal-mode function

- **WHEN** the whole tree — every `*.rs` under `src/` and under `tests/` — is searched for
  `enable_raw_mode`, `disable_raw_mode`, `EnterAlternateScreen`, `LeaveAlternateScreen`,
  `EnableMouseCapture`, and `DisableMouseCapture`
- **THEN** every match is in `src/ui/terminal.rs`
- **AND** `src/ui/terminal.rs` itself matches at least one of them, so a search that found
  nothing because the file was renamed or gutted fails rather than passing vacuously
- **AND** the check fails when `src/ui/terminal.rs` is absent, rather than reporting a
  clean tree
- **AND** the check's positive control is per name, not one-of-any: for **each** of the six
  it asserts both that the name matches the searched pattern and that
  `src/ui/terminal.rs` contains it. The one-of-any form this replaces is satisfied by
  `enable_raw_mode` alone, so it would have covered the two capture commands vacuously
- **AND** the check therefore fails in **both** directions — when a capture command is
  dropped from the searched pattern while `src/ui/terminal.rs` still names it, and when it
  is dropped from `src/ui/terminal.rs` while the pattern still searches for it — with a
  planted control recorded for each of the two capture names

#### Scenario: No test constructs the real terminal implementation

- **WHEN** every `*.rs` file under `src/` and under `tests/` is searched for `CrosstermOps`
- **THEN** every match is in `src/ui/terminal.rs`, where it is defined, or in
  `src/ui/mod.rs`, where `ui::run` wires it — no match is under `tests/`, and none is in a
  `#[cfg(test)]` module
- **AND** at least two matches exist, so a tree where the type was deleted or renamed fails
  rather than reporting a clean result
- **AND** every test taking `&dyn TerminalOps` therefore receives the recording double,
  which returns `Ok(())` or a configured `TerminalError` and touches no terminal


#### Scenario: The clipboard write is confined and reports its own failure

- **WHEN** the crate is swept for the OSC 52 introducer, `tests/` included
- **THEN** it appears in `src/ui/terminal.rs` and nowhere else
- **AND** the sweep fails when that file is absent or names no such sequence, so the exclusion
  cannot pass vacuously
- **AND** a `write_clipboard` whose underlying write fails returns a `TerminalError` naming
  `write_clipboard`, whose text the pane stores on `Selection::problem` and renders as a
  **detail-region** problem row beside `detail.problems` — not on any list-region problem
  list, every one of which is replaced wholesale by its own producer

#### Scenario: A clipboard write changes no terminal mode

- **WHEN** a guard is entered over a recording `TerminalOps`, `write_clipboard` is called
  twice, and the guard is dropped
- **THEN** the recorded mode operations are exactly the six a guard with no clipboard write
  records, in the same order
- **AND** the two writes appear between them without disturbing the mirrored entry and
  teardown
