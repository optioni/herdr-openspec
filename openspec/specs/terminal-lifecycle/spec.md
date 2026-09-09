# terminal-lifecycle Specification

## Purpose
Owns the process's terminal mode: a `TerminalOps` seam whose only real implementation calls
the crossterm functions and nothing else, a `TerminalGuard` that enters raw mode then the
alternate screen and leaves them in exactly the mirrored order, unwinding a partial entry when
the second step fails and swallowing teardown errors rather than panicking in `Drop`, and a
panic hook that restores before delegating to the hook it replaces. It also holds the pure
decision that refuses to start the dashboard when stdout is not a terminal without touching a
single terminal operation. The exit status and message that refusal produces belong to
`plugin-build`.

## Requirements

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

### Requirement: A panic restores the terminal

Dropping a `TerminalGuard` during stack unwinding SHALL restore the terminal exactly as a
normal drop does. In addition, `ui::terminal::install_panic_hook` SHALL install a hook
that restores the terminal **before** delegating to the previously installed hook, so the
panic message is printed to a cooked terminal on the main screen rather than to a raw
alternate screen that is about to be torn down.

`install_panic_hook` SHALL be called at most once per process and SHALL chain to, never
discard, the hook it replaces.

The hook SHALL restore the terminal **only when the panicking thread is the thread that
installed it** — the render thread. The crate runs three detached worker threads beside the
render loop (`refresh`, `agents`, `launch`), any of which can panic while `run_loop` is mid-frame.
Restoring from one of those leaves the alternate screen and disables raw mode under a loop
that is still drawing: the dashboard continues painting into the primary screen with the
user's keystrokes echoed and line-buffered, and the guard's own `Drop` then restores a
second time. `install_panic_hook` SHALL therefore capture
`std::thread::current().id()` at install time, and its hook SHALL compare that captured
`ThreadId` against the panicking thread's own before restoring. Off the render thread the
hook SHALL restore nothing and SHALL delegate to the previous hook unchanged, so the panic
message is still printed and no restoration is performed on a terminal the render loop
still owns.

The comparison SHALL be on the captured `ThreadId`, never on the thread's name: a thread
name is optional, is not unique, and none of the three workers sets one.

The decision SHALL live in a pure function — `ui::terminal::restore_then_if(ops, installed_on,
current, delegate)` or equivalent — so it is unit-tested from a spawned thread without
installing a process-global hook, on exactly `restore_then`'s existing terms.

Restoring SHALL release mouse capture as well, first, on every path that restores anything:
`restore_then` calls `disable_mouse`, then `leave_alternate`, then `disable_raw`. A panic
that left capture enabled would leave the user's terminal emitting escape sequences for
every click and scroll into whatever shell the panic message landed in, which is the same
class of damage raw mode itself does. The call is unconditional, on `TerminalGuard::drop`'s
own terms (`mouse-input` -> design.md -> Decision 7): the hook has no guard to consult and
writing the disable sequence to a terminal that never enabled capture is inert.

The worker-thread rule is unchanged and SHALL NOT grow an exception for capture: off the
render thread the hook restores **nothing**, capture included, because the render loop still
owns the terminal.

#### Scenario: Unwinding past the guard still restores

- **WHEN** a closure that constructs a `TerminalGuard` over a recording double and then
  panics is run under `std::panic::catch_unwind`
- **THEN** `catch_unwind` reports the panic
- **AND** the double's recorded call list is exactly
  `["enable_raw", "enter_alternate", "enable_mouse", "disable_mouse", "leave_alternate",
  "disable_raw"]`, proving the drop ran during unwinding rather than being skipped

#### Scenario: The hook restores before the previous hook runs

- **WHEN** the restore step and the delegation step are driven through
  `ui::terminal::restore_then(&dyn TerminalOps, &mut dyn FnMut())` — the pure function
  `install_panic_hook`'s closure body consists of — with a recording double and a closure
  that appends `"previous_hook"` to the same recording
- **THEN** the recorded list is exactly
  `["disable_mouse", "leave_alternate", "disable_raw", "previous_hook"]`, in that order
- **AND** the delegation runs even when all three restore operations return errors

#### Scenario: A panic on a worker thread restores nothing

- **WHEN** the hook's body is driven with an `installed_on` thread id captured on the test's
  own thread and a `current` thread id taken from a second, spawned thread — the shape a
  refresh, poller, or launcher worker panicking beside a live render loop has — over a
  recording double and a delegate that appends `"previous_hook"`
- **THEN** the recorded list is exactly `["previous_hook"]`: no `disable_mouse`, no
  `leave_alternate`, and no `disable_raw` were called
- **AND** the delegation still ran, so the panic message is not swallowed by the guard

#### Scenario: A panic on the render thread still restores

- **WHEN** the same body is driven with `installed_on` and `current` both equal to the
  calling thread's own id
- **THEN** the recorded list is exactly
  `["disable_mouse", "leave_alternate", "disable_raw", "previous_hook"]` — the same list the
  render-thread path produced before `mouse-input`, with `disable_mouse` prepended and
  nothing else moved
- **AND** the main-thread case is therefore proved to differ from the pre-change behaviour by
  exactly that one leading entry, rather than assumed unchanged

### Requirement: The dashboard refuses to start when stdout is not a terminal

The decision SHALL live in one pure function,
`ui::enter_if_terminal(is_terminal: bool, ops: &dyn TerminalOps) -> Result<TerminalGuard,
StartError>`, which returns `StartError::NotATerminal` **without calling any `TerminalOps`
method** when `is_terminal` is false, and enters the guard otherwise. `ui::run` SHALL call
it with `std::io::IsTerminal` on stdout as its first action, so no configuration read and
no filesystem walk happens in a process that cannot render.

The exit status and message `src/main.rs` produces from that error are `plugin-build`'s,
which owns the binary's argument and exit surface; this requirement owns only the decision
and its purity.

#### Scenario: The refusal touches no terminal operation

- **WHEN** `ui::enter_if_terminal(false, &double)` is called with a recording `TerminalOps`
  double
- **THEN** it returns `Err(StartError::NotATerminal)`
- **AND** the double's recorded call list is **empty** — the assertion that discriminates,
  since a guard entered and immediately dropped would record six calls and would still
  return an error to the caller
- **AND** `ui::enter_if_terminal(true, &double)` over the same double returns `Ok` and
  records `["enable_raw", "enter_alternate", "enable_mouse"]`, so the `false` case is proven
  to be a branch rather than a function that never does anything

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
