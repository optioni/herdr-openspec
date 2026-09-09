## MODIFIED Requirements

### Requirement: Terminal setup and teardown sit behind an injected seam

The crate SHALL define `ui::terminal::TerminalOps`, a trait with exactly six fallible
operations — `enable_raw`, `enter_alternate`, `enable_mouse`, `disable_mouse`,
`leave_alternate`, `disable_raw` — each returning `Result<(), TerminalError>`.
`ui::terminal::CrosstermOps` SHALL be the one implementation that touches a real terminal,
and each of its six methods SHALL do nothing but call the corresponding `ratatui::crossterm`
function or command and map its error. No decision, no ordering, and no state SHALL live
inside `CrosstermOps`: the ordering lives in `TerminalGuard`, which is what makes it testable
without a terminal.

`TerminalError` SHALL carry the failing operation's name and the underlying error's
`Display` text, and SHALL NOT be `std::io::Error`, so that a test double can produce one
without fabricating an I/O error.

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
- **AND** the check fails when either capture command is dropped from the searched pattern
  while `src/ui/terminal.rs` still names it, so the pattern cannot silently stop covering
  the two operations this change added

#### Scenario: No test constructs the real terminal implementation

- **WHEN** every `*.rs` file under `src/` and under `tests/` is searched for `CrosstermOps`
- **THEN** every match is in `src/ui/terminal.rs`, where it is defined, or in
  `src/ui/mod.rs`, where `ui::run` wires it — no match is under `tests/`, and none is in a
  `#[cfg(test)]` module
- **AND** at least two matches exist, so a tree where the type was deleted or renamed fails
  rather than reporting a clean result
- **AND** every test taking `&dyn TerminalOps` therefore receives the recording double,
  which returns `Ok(())` or a configured `TerminalError` and touches no terminal

### Requirement: The guard enters and leaves in a fixed, mirrored order

`ui::terminal::TerminalGuard::enter(&dyn TerminalOps)` SHALL call `enable_raw`, then
`enter_alternate`, then `enable_mouse`, in that order, and SHALL return a guard value.
Dropping the guard SHALL call `disable_mouse`, then `leave_alternate`, then `disable_raw`,
in that order — the exact reverse — and SHALL ignore each operation's error rather than
panicking inside `Drop`.

When `enable_raw` fails, `enter` SHALL return that error and SHALL NOT call
`enter_alternate` and SHALL NOT leave anything to undo. When `enter_alternate` fails,
`enter` SHALL call `disable_raw` before returning the error, so a partially entered
terminal is never left behind.

When `enable_mouse` fails, `enter` SHALL NOT return an error and SHALL NOT unwind: it SHALL
return a guard whose `mouse_problem()` is `Some(text)`, where `text` is the `TerminalError`'s
`Display` form. Mouse capture is the one entry operation the pane does not need in order to
render — a terminal that refuses it is a terminal the reader can still drive entirely by key
— and `SPEC.md`'s "never fail closed" rule makes refusing to start the wrong answer. On a
guard whose capture succeeded, `mouse_problem()` SHALL be `None`.

Teardown SHALL call `disable_mouse` unconditionally, whether or not `enable_mouse`
succeeded: writing the disable sequence to a terminal that never enabled capture is inert,
and a conditional teardown would be a branch whose false arm no test could observe.

A guard SHALL restore exactly once. Restoring SHALL NOT be repeatable by cloning,
copying, or re-dropping the same guard.

#### Scenario: Normal lifetime records the four operations mirrored

- **WHEN** a `TerminalGuard` is created over a recording double and then dropped
- **THEN** the double's recorded call list, with the two capture operations filtered out,
  is exactly `["enable_raw", "enter_alternate", "leave_alternate", "disable_raw"]` — the
  four operations this scenario has asserted since `tui-shell`, still in the same mirrored
  order after `mouse-input` added two around them

#### Scenario: Normal lifetime records all six operations mirrored

- **WHEN** a `TerminalGuard` is created over a recording double and then dropped
- **THEN** the double's recorded call list is exactly
  `["enable_raw", "enter_alternate", "enable_mouse", "disable_mouse", "leave_alternate",
  "disable_raw"]`
- **AND** the guard's `mouse_problem()` was `None` for its whole lifetime
- **AND** `enable_mouse` is the last entry operation and `disable_mouse` the first teardown
  one, so the pair mirrors around the four it wraps

#### Scenario: Raw mode fails and nothing else is attempted

- **WHEN** the double is configured to fail `enable_raw` with `TerminalError` naming
  `enable_raw`, and `TerminalGuard::enter` is called
- **THEN** `enter` returns that error and no guard value exists to drop
- **AND** the double's recorded call list is exactly `["enable_raw"]` — in particular it
  contains no `disable_raw` and no `enable_mouse`, because raw mode was never entered

#### Scenario: The alternate screen fails and raw mode is unwound

- **WHEN** the double succeeds on `enable_raw` and fails `enter_alternate`
- **THEN** `enter` returns the `enter_alternate` error
- **AND** the recorded call list is exactly
  `["enable_raw", "enter_alternate", "disable_raw"]`, so raw mode was undone rather than
  left set on a terminal the user is still typing into, and `enable_mouse` was never
  attempted

#### Scenario: Mouse capture fails and the guard is still returned

- **WHEN** the double succeeds on `enable_raw` and `enter_alternate` and fails
  `enable_mouse` with a `TerminalError` whose detail is `no mouse`
- **THEN** `enter` returns `Ok`, not `Err`
- **AND** the guard's `mouse_problem()` is `Some` and its text names both `enable_mouse` and
  `no mouse`
- **AND** dropping the guard records `["disable_mouse", "leave_alternate", "disable_raw"]`,
  so teardown is unchanged by the failed entry

#### Scenario: Teardown errors are swallowed rather than panicking in Drop

- **WHEN** the double succeeds on all three entry operations and fails **all three**
  `disable_mouse`, `leave_alternate`, and `disable_raw`, and the guard is dropped
- **THEN** the drop completes without panicking
- **AND** the recorded call list still ends
  `["disable_mouse", "leave_alternate", "disable_raw"]` — every later teardown operation is
  attempted even though the one before it failed, because abandoning them would leave raw
  mode set

## ADDED Requirements

### Requirement: A failed mouse capture is named rather than swallowed

`ui::Startup` SHALL carry a `mouse_problem: Option<String>` field, and `ui::run` SHALL fill
it from the `TerminalGuard`'s own `mouse_problem()` — a field expression, not a branch, so
`ui::run`'s body still holds no branch and no loop and the `WIRED` gate's leg 2 stays green.

`ui::run_wired` SHALL append that reason, when present, to `dashboard.refresh.startup`,
**after** every problem `start_collaborators` reported. It is appended last rather than
first because it explains the least: a missing `openspec` binary or a watcher that would not
start changes what the pane can show, while a refused mouse capture only withdraws a second
way to reach what the keys already reach, and it must not push a probe reason off the top of
a short list.

`SPEC.md`'s degraded-states table SHALL gain a row for this condition, and
`tests/degraded-coverage.toml` SHALL bind that row to a named, passing test.

#### Scenario: A refused capture becomes a leading problem row, below the probe's own

- **WHEN** `run_wired` is driven with a `Startup` whose `mouse_problem` is
  `Some("enable_mouse: no mouse")` and whose binary probe resolves nothing
- **THEN** the resulting `dashboard.refresh.startup` holds the probe's own reasons first and
  `enable_mouse: no mouse` last
- **AND** the first frame renders each of them as a `!`-marked row above the change rows
- **AND** the loop runs normally: the pane draws, the keys work, and no error screen replaces
  the dashboard

#### Scenario: A successful capture adds no row

- **WHEN** `run_wired` is driven with a `Startup` whose `mouse_problem` is `None`
- **THEN** `dashboard.refresh.startup` is exactly what `start_collaborators` reported, with
  no extra entry
- **AND** the rendered frame is byte-identical to the frame the same startup produced before
  this change
