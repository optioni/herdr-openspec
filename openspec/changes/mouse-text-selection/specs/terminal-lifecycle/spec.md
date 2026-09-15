## ADDED Requirements

### Requirement: Mouse capture can be released and re-entered mid-session

`TerminalGuard` SHALL expose a method that sets mouse capture on a live guard, calling
`TerminalOps::enable_mouse` to enter it and `TerminalOps::disable_mouse` to leave it, and
SHALL return the underlying `TerminalError`'s own `Display` text on failure rather than a
message of its own. It SHALL NOT consult or update `mouse_problem`, which names what happened
at **start-up** and must keep saying so for the guard's whole lifetime.

The guard's teardown SHALL be unchanged by this method in every respect: `Drop` SHALL call
`disable_mouse`, `leave_alternate`, and `disable_raw` in that order regardless of how many
times capture was released and re-entered, and regardless of the state it is in when the guard
drops. `restore_then` SHALL keep calling `disable_mouse` **unconditionally** — it consults no
state before this change and SHALL consult none after it, so the panic hook stays a pure
function of `ops` and cannot leave a terminal reporting mouse events into the shell a panic
message lands in.

This is the property that makes a mid-session toggle safe, and it SHALL NOT be traded for an
optimisation: skipping a redundant `disable_mouse` would require the hook to read the live
capture state across a thread boundary during a panic.

#### Scenario: Releasing and re-entering records exactly the two operations

- **WHEN** a guard is entered over a recording `TerminalOps`, capture is released, then
  re-entered, then released again, and the guard is dropped
- **THEN** the recorded operations are `enable_raw`, `enter_alternate`, `enable_mouse`,
  then `disable_mouse`, `enable_mouse`, `disable_mouse` for the three toggles, then
  `disable_mouse`, `leave_alternate`, `disable_raw` for the teardown
- **AND** the teardown's three operations are byte-identical to the teardown the same guard
  records with no toggle at all, so a toggle changes nothing about how the guard unwinds

#### Scenario: Teardown is unconditional whatever state capture is left in

- **WHEN** a guard is entered, capture is released, and the guard is dropped while released
- **THEN** `disable_mouse` is still called, first, before `leave_alternate` and `disable_raw`
- **AND** the same holds when the guard is dropped by a panic rather than a normal return, so
  the hook's unconditional call is exercised on both paths

#### Scenario: A start-up refusal is not overwritten by a later success

- **WHEN** a guard is entered over an `ops` whose `enable_mouse` fails, and capture is then
  successfully re-entered mid-session
- **THEN** `mouse_problem()` still returns the start-up refusal's text
- **AND** the start-up problem row `run_wired` seeded from it is unchanged, because
  `refresh.startup` is written exactly once and nothing downstream rewrites it

### Requirement: A refused capture change is named rather than assumed

When the seam answers `Err`, the pane SHALL leave `Dashboard::mouse_capture` at the value the
terminal actually granted — the value it already held — and SHALL record the reason as a
problem row, never silently flip the flag to a state the terminal refused. A pane claiming
capture is off while the terminal is still reporting would leave the reader dragging against a
mouse that is still captured with a badge telling them it is not.

`SPEC.md`'s degraded-states table SHALL gain a row for this condition, and
`tests/degraded-coverage.toml` SHALL bind that row to a named, passing test, on exactly the
terms the start-up refusal row is bound today.

#### Scenario: A refused release is a problem row and no state change

- **WHEN** the reader presses `m` with `mouse_capture` `true` and the seam answers
  `Err("disable_mouse: device busy")`
- **THEN** `mouse_capture` is still `true` and the footer draws no released-capture badge
- **AND** `device busy` is rendered as a `!`-marked row above the change rows
- **AND** pressing `m` again re-attempts the change rather than being latched off by the
  earlier failure
