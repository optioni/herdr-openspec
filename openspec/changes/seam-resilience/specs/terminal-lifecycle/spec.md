## MODIFIED Requirements

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

#### Scenario: Unwinding past the guard still restores

- **WHEN** a closure that constructs a `TerminalGuard` over a recording double and then
  panics is run under `std::panic::catch_unwind`
- **THEN** `catch_unwind` reports the panic
- **AND** the double's recorded call list is exactly
  `["enable_raw", "enter_alternate", "leave_alternate", "disable_raw"]`, proving the drop
  ran during unwinding rather than being skipped

#### Scenario: The hook restores before the previous hook runs

- **WHEN** the restore step and the delegation step are driven through
  `ui::terminal::restore_then(&dyn TerminalOps, &mut dyn FnMut())` — the pure function
  `install_panic_hook`'s closure body consists of — with a recording double and a closure
  that appends `"previous_hook"` to the same recording
- **THEN** the recorded list is exactly
  `["leave_alternate", "disable_raw", "previous_hook"]`, in that order
- **AND** the delegation runs even when both restore operations return errors

#### Scenario: A panic on a worker thread restores nothing

- **WHEN** the hook's body is driven with an `installed_on` thread id captured on the test's
  own thread and a `current` thread id taken from a second, spawned thread — the shape a
  refresh, poller, or launcher worker panicking beside a live render loop has — over a
  recording double and a delegate that appends `"previous_hook"`
- **THEN** the recorded list is exactly `["previous_hook"]`: no `leave_alternate` and no
  `disable_raw` were called
- **AND** the delegation still ran, so the panic message is not swallowed by the guard

#### Scenario: A panic on the render thread still restores

- **WHEN** the same body is driven with `installed_on` and `current` both equal to the
  calling thread's own id
- **THEN** the recorded list is exactly
  `["leave_alternate", "disable_raw", "previous_hook"]`, byte-identical to the hook's
  behaviour before this change
- **AND** the main-thread case is therefore proved unchanged rather than assumed unchanged
