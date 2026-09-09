//! The terminal seam: raw mode, the alternate screen, and mouse capture,
//! entered and left in a fixed, mirrored order, behind an injected
//! `TerminalOps` trait so this is testable with no real terminal. See
//! `openspec/changes/tui-shell/specs/terminal-lifecycle/spec.md` and
//! `openspec/changes/mouse-input/specs/terminal-lifecycle/spec.md`.

use std::fmt;

/// The failing operation's name and the underlying error's `Display` text.
/// Deliberately not `std::io::Error`, so a test double can produce one
/// without fabricating an I/O error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalError {
    pub op: &'static str,
    pub detail: String,
}

impl fmt::Display for TerminalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.op, self.detail)
    }
}

/// The six fallible terminal-mode operations — `mouse-input` added the
/// capture pair, which the guard enters last and leaves first.
/// `CrosstermOps` is the one implementation that touches a real terminal;
/// every test uses a double.
pub trait TerminalOps {
    fn enable_raw(&self) -> Result<(), TerminalError>;
    fn enter_alternate(&self) -> Result<(), TerminalError>;
    fn enable_mouse(&self) -> Result<(), TerminalError>;
    fn disable_mouse(&self) -> Result<(), TerminalError>;
    fn leave_alternate(&self) -> Result<(), TerminalError>;
    fn disable_raw(&self) -> Result<(), TerminalError>;
}

/// Enters `enable_raw`, `enter_alternate`, then `enable_mouse` on
/// construction, and calls `disable_mouse`, `leave_alternate`, then
/// `disable_raw` — the exact reverse — on drop, ignoring each teardown
/// operation's error rather than panicking inside `Drop`. Restores exactly
/// once: there is no way to clone, copy, or re-drop the same guard.
///
/// A refused `enable_mouse` is **not** an error: it is stored and reported
/// through [`TerminalGuard::mouse_problem`], because the pane is fully
/// usable by key without capture and `SPEC.md`'s "never fail closed" rule
/// makes refusing to start the wrong answer.
pub struct TerminalGuard<'a> {
    ops: &'a dyn TerminalOps,
    /// The `TerminalError`'s `Display` text when `enable_mouse` was refused,
    /// `None` when it succeeded. Never consulted by teardown — see
    /// [`restore_then`] and design.md -> Decision 7.
    mouse_problem: Option<String>,
}

impl fmt::Debug for TerminalGuard<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TerminalGuard").finish_non_exhaustive()
    }
}

impl<'a> TerminalGuard<'a> {
    /// Enter raw mode, then the alternate screen, then mouse capture, in
    /// that order. If `enable_raw` fails, nothing else is attempted and
    /// there is no guard to undo. If `enter_alternate` fails, `disable_raw`
    /// is called before returning the error, so a partially entered
    /// terminal is never left behind. If `enable_mouse` fails, the error is
    /// **stored** rather than returned: a guard is still produced and the
    /// pane still starts.
    pub fn enter(ops: &'a dyn TerminalOps) -> Result<Self, TerminalError> {
        ops.enable_raw()?;
        if let Err(err) = ops.enter_alternate() {
            let _ = ops.disable_raw();
            return Err(err);
        }
        let mouse_problem = ops.enable_mouse().err().map(|err| err.to_string());
        Ok(Self { ops, mouse_problem })
    }

    /// The reason mouse capture was refused, as the `TerminalError`'s own
    /// `Display` text, or `None` when it was entered. `ui::run` threads this
    /// onto `Startup::mouse_problem`, from where `run_wired` appends it to
    /// `refresh.startup` as that list's last entry.
    pub fn mouse_problem(&self) -> Option<String> {
        self.mouse_problem.clone()
    }
}

impl Drop for TerminalGuard<'_> {
    fn drop(&mut self) {
        restore_then(self.ops, &mut || {});
    }
}

/// Release mouse capture, leave the alternate screen, and disable raw mode,
/// ignoring each operation's error, then run `next`. This is the pure body
/// both `TerminalGuard::drop` and the panic hook installed by
/// [`install_panic_hook`] delegate to — `next` restores before the previous
/// panic hook runs, so the panic message lands on a cooked terminal.
///
/// `disable_mouse` is called **unconditionally**, whether or not
/// `enable_mouse` succeeded: writing the disable sequence to a terminal that
/// never enabled capture is inert, the hook has no guard to consult, and a
/// conditional teardown would be a branch whose false arm no test could
/// observe from outside (`mouse-input` -> design.md -> Decision 7). A panic
/// that left capture enabled would leave the user's terminal emitting escape
/// sequences for every click and scroll into whatever shell the panic message
/// landed in.
pub fn restore_then(ops: &dyn TerminalOps, next: &mut dyn FnMut()) {
    let _ = ops.disable_mouse();
    let _ = ops.leave_alternate();
    let _ = ops.disable_raw();
    next();
}

/// The one implementation that touches a real terminal. Each method calls
/// exactly one `ratatui::crossterm` terminal-mode function and maps its
/// error — no decision, no ordering, no state.
pub struct CrosstermOps;

impl TerminalOps for CrosstermOps {
    fn enable_raw(&self) -> Result<(), TerminalError> {
        ratatui::crossterm::terminal::enable_raw_mode().map_err(|e| TerminalError {
            op: "enable_raw",
            detail: e.to_string(),
        })
    }

    fn enter_alternate(&self) -> Result<(), TerminalError> {
        ratatui::crossterm::execute!(
            std::io::stdout(),
            ratatui::crossterm::terminal::EnterAlternateScreen
        )
        .map_err(|e| TerminalError {
            op: "enter_alternate",
            detail: e.to_string(),
        })
    }

    fn enable_mouse(&self) -> Result<(), TerminalError> {
        ratatui::crossterm::execute!(
            std::io::stdout(),
            ratatui::crossterm::event::EnableMouseCapture
        )
        .map_err(|e| TerminalError {
            op: "enable_mouse",
            detail: e.to_string(),
        })
    }

    fn disable_mouse(&self) -> Result<(), TerminalError> {
        ratatui::crossterm::execute!(
            std::io::stdout(),
            ratatui::crossterm::event::DisableMouseCapture
        )
        .map_err(|e| TerminalError {
            op: "disable_mouse",
            detail: e.to_string(),
        })
    }

    fn leave_alternate(&self) -> Result<(), TerminalError> {
        ratatui::crossterm::execute!(
            std::io::stdout(),
            ratatui::crossterm::terminal::LeaveAlternateScreen
        )
        .map_err(|e| TerminalError {
            op: "leave_alternate",
            detail: e.to_string(),
        })
    }

    fn disable_raw(&self) -> Result<(), TerminalError> {
        ratatui::crossterm::terminal::disable_raw_mode().map_err(|e| TerminalError {
            op: "disable_raw",
            detail: e.to_string(),
        })
    }
}

/// Restore the terminal and then run `next` only when `current` — the
/// panicking thread's id — equals `installed_on` — the id captured when the
/// hook was installed, always the render thread's. Off the render thread
/// (a `refresh`, `agents`, or `launch` worker panicking beside a live
/// render loop) nothing is restored: the loop still owns the terminal, and
/// restoring out from under it would leave the alternate screen and raw
/// mode disabled while `run_loop` keeps drawing, with the guard's own
/// `Drop` then restoring a second time. `next` always runs, on the
/// render-thread and worker-thread paths alike, so the panic message is
/// never swallowed.
///
/// A thread id, not a thread name, is compared: a name is optional, is not
/// unique, and none of the three workers sets one.
pub fn restore_then_if(
    ops: &dyn TerminalOps,
    installed_on: std::thread::ThreadId,
    current: std::thread::ThreadId,
    next: &mut dyn FnMut(),
) {
    if current == installed_on {
        restore_then(ops, next);
    } else {
        next();
    }
}

/// Install a panic hook that restores the terminal before delegating to the
/// hook already installed, so a panic message lands on a cooked terminal on
/// the main screen rather than a raw alternate screen about to be torn
/// down, and releases mouse capture along with them. Chains to, never
/// discards, the previous hook. Restores only when
/// the panicking thread is the one that installed the hook — the render
/// thread — via `restore_then_if`; a worker thread's panic delegates
/// without restoring. Not itself unit tested — installing a hook is
/// process-global and `cargo test` runs tests in parallel threads of one
/// process — its body is `restore_then_if`, which is.
pub fn install_panic_hook() {
    let installed_on = std::thread::current().id();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_then_if(
            &CrosstermOps,
            installed_on,
            std::thread::current().id(),
            &mut || previous(info),
        );
    }));
}

#[cfg(test)]
mod tests {
    mod guard {
        use std::cell::RefCell;
        use std::collections::BTreeMap;
        use std::panic::AssertUnwindSafe;

        use crate::ui::terminal::{TerminalError, TerminalGuard, TerminalOps, restore_then};

        /// The two capture operations `mouse-input` added, filtered out of a
        /// recorded list so the four-operation claim `tui-shell` made can be
        /// asserted verbatim beside the six-operation one.
        const CAPTURE: [&str; 2] = ["enable_mouse", "disable_mouse"];

        #[derive(Default)]
        struct Recorder {
            calls: RefCell<Vec<&'static str>>,
            /// The operation name to the `TerminalError::detail` its failure
            /// carries — a map rather than `mouse-input`'s predecessor's set,
            /// because `mouse_failure_still_returns_a_guard` asserts the
            /// detail text reaches `mouse_problem()` unchanged.
            failing: RefCell<BTreeMap<&'static str, String>>,
        }

        impl Recorder {
            fn fail(&self, op: &'static str) {
                self.fail_with(op, "recorder configured to fail");
            }

            fn fail_with(&self, op: &'static str, detail: &str) {
                self.failing.borrow_mut().insert(op, detail.to_string());
            }

            fn calls(&self) -> Vec<&'static str> {
                self.calls.borrow().clone()
            }

            /// `calls()` without the two capture operations — the list the
            /// four-operation scenario asserts.
            fn calls_without_capture(&self) -> Vec<&'static str> {
                self.calls()
                    .into_iter()
                    .filter(|op| !CAPTURE.contains(op))
                    .collect()
            }

            fn note(&self, label: &'static str) {
                self.calls.borrow_mut().push(label);
            }

            fn record(&self, op: &'static str) -> Result<(), TerminalError> {
                self.calls.borrow_mut().push(op);
                match self.failing.borrow().get(op) {
                    Some(detail) => Err(TerminalError {
                        op,
                        detail: detail.clone(),
                    }),
                    None => Ok(()),
                }
            }
        }

        impl TerminalOps for Recorder {
            fn enable_raw(&self) -> Result<(), TerminalError> {
                self.record("enable_raw")
            }
            fn enter_alternate(&self) -> Result<(), TerminalError> {
                self.record("enter_alternate")
            }
            fn enable_mouse(&self) -> Result<(), TerminalError> {
                self.record("enable_mouse")
            }
            fn disable_mouse(&self) -> Result<(), TerminalError> {
                self.record("disable_mouse")
            }
            fn leave_alternate(&self) -> Result<(), TerminalError> {
                self.record("leave_alternate")
            }
            fn disable_raw(&self) -> Result<(), TerminalError> {
                self.record("disable_raw")
            }
        }

        #[test]
        fn normal_lifetime_is_enter_enter_leave_disable() {
            // `terminal-lifecycle`: "Normal lifetime records the four operations
            // mirrored". The two capture operations are filtered out, so the claim
            // this scenario has asserted since `tui-shell` survives verbatim after
            // `mouse-input` added two around them.
            let rec = Recorder::default();
            {
                let _guard = TerminalGuard::enter(&rec).expect("enter succeeds");
            }
            assert_eq!(
                rec.calls_without_capture(),
                vec![
                    "enable_raw",
                    "enter_alternate",
                    "leave_alternate",
                    "disable_raw"
                ]
            );
        }

        #[test]
        fn normal_lifetime_records_all_six() {
            // `terminal-lifecycle`: "Normal lifetime records all six operations
            // mirrored".
            let rec = Recorder::default();
            {
                let guard = TerminalGuard::enter(&rec).expect("enter succeeds");
                assert_eq!(
                    guard.mouse_problem(),
                    None,
                    "capture succeeded, so nothing is reported for its whole lifetime"
                );
            }
            let calls = rec.calls();
            assert_eq!(
                calls,
                vec![
                    "enable_raw",
                    "enter_alternate",
                    "enable_mouse",
                    "disable_mouse",
                    "leave_alternate",
                    "disable_raw"
                ]
            );
            // The pair mirrors around the four it wraps: `enable_mouse` is the last
            // entry operation and `disable_mouse` the first teardown one.
            assert_eq!(calls[2], "enable_mouse");
            assert_eq!(calls[3], "disable_mouse");
        }

        #[test]
        fn mouse_failure_still_returns_a_guard() {
            // `terminal-lifecycle`: "Mouse capture fails and the guard is still
            // returned". Capture is the one entry operation the pane does not need
            // in order to render, and `SPEC.md`'s "never fail closed" rule makes
            // refusing to start the wrong answer.
            let rec = Recorder::default();
            rec.fail_with("enable_mouse", "no mouse");
            {
                let guard = TerminalGuard::enter(&rec).expect("enter returns Ok, not Err");
                let problem = guard.mouse_problem().expect("the refusal is reported");
                assert!(problem.contains("enable_mouse"), "{problem}");
                assert!(problem.contains("no mouse"), "{problem}");
            }
            assert_eq!(
                &rec.calls()[3..],
                ["disable_mouse", "leave_alternate", "disable_raw"],
                "teardown is unchanged by the failed entry"
            );
        }

        #[test]
        fn enable_raw_failure_attempts_nothing_further() {
            let rec = Recorder::default();
            rec.fail("enable_raw");
            let err = TerminalGuard::enter(&rec).expect_err("enable_raw fails");
            assert_eq!(err.op, "enable_raw");
            // In particular no `disable_raw` and no `enable_mouse`: raw mode was
            // never entered.
            assert_eq!(rec.calls(), vec!["enable_raw"]);
        }

        #[test]
        fn alternate_screen_failure_unwinds_raw_mode() {
            let rec = Recorder::default();
            rec.fail("enter_alternate");
            let err = TerminalGuard::enter(&rec).expect_err("enter_alternate fails");
            assert_eq!(err.op, "enter_alternate");
            // Raw mode was undone rather than left set on a terminal the user is
            // still typing into, and `enable_mouse` was never attempted.
            assert_eq!(
                rec.calls(),
                vec!["enable_raw", "enter_alternate", "disable_raw"]
            );
        }

        #[test]
        fn teardown_errors_do_not_panic_and_both_are_attempted() {
            // `terminal-lifecycle`: every later teardown operation is attempted
            // even though the one before it failed, because abandoning them would
            // leave raw mode set. All **three** fail after `mouse-input`.
            let rec = Recorder::default();
            rec.fail("disable_mouse");
            rec.fail("leave_alternate");
            rec.fail("disable_raw");
            {
                let _guard = TerminalGuard::enter(&rec).expect("enter succeeds");
            }
            let calls = rec.calls();
            assert_eq!(
                &calls[calls.len() - 3..],
                ["disable_mouse", "leave_alternate", "disable_raw"]
            );
        }

        #[test]
        fn a_panic_still_restores() {
            let rec = Recorder::default();
            let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
                let _guard = TerminalGuard::enter(&rec).expect("enter succeeds");
                panic!("boom");
            }));
            assert!(result.is_err());
            assert_eq!(
                rec.calls(),
                vec![
                    "enable_raw",
                    "enter_alternate",
                    "enable_mouse",
                    "disable_mouse",
                    "leave_alternate",
                    "disable_raw"
                ]
            );
        }

        #[test]
        fn restore_then_restores_before_delegating() {
            let rec = Recorder::default();
            restore_then(&rec, &mut || rec.note("previous_hook"));
            assert_eq!(
                rec.calls(),
                vec![
                    "disable_mouse",
                    "leave_alternate",
                    "disable_raw",
                    "previous_hook"
                ]
            );
        }

        #[test]
        fn restore_then_delegates_even_when_both_restores_fail() {
            let rec = Recorder::default();
            rec.fail("disable_mouse");
            rec.fail("leave_alternate");
            rec.fail("disable_raw");
            restore_then(&rec, &mut || rec.note("previous_hook"));
            assert_eq!(
                rec.calls(),
                vec![
                    "disable_mouse",
                    "leave_alternate",
                    "disable_raw",
                    "previous_hook"
                ]
            );
        }

        #[test]
        fn a_panic_on_a_worker_thread_restores_nothing() {
            use crate::ui::terminal::restore_then_if;

            let rec = Recorder::default();
            let installed_on = std::thread::current().id();
            let current = std::thread::spawn(|| std::thread::current().id())
                .join()
                .expect("spawned thread does not panic");
            assert_ne!(
                installed_on, current,
                "a spawned thread must have a different id from the test's own"
            );

            restore_then_if(&rec, installed_on, current, &mut || {
                rec.note("previous_hook")
            });

            assert_eq!(rec.calls(), vec!["previous_hook"]);
        }

        #[test]
        fn a_panic_on_the_render_thread_still_restores() {
            use crate::ui::terminal::restore_then_if;

            let rec = Recorder::default();
            let id = std::thread::current().id();

            restore_then_if(&rec, id, id, &mut || rec.note("previous_hook"));

            // The same list the render-thread path produced before `mouse-input`,
            // with `disable_mouse` prepended and nothing else moved.
            assert_eq!(
                rec.calls(),
                vec![
                    "disable_mouse",
                    "leave_alternate",
                    "disable_raw",
                    "previous_hook"
                ]
            );
        }

        #[test]
        fn terminal_error_names_the_failing_op_and_detail() {
            let err = TerminalError {
                op: "enable_raw",
                detail: "device busy".to_string(),
            };
            let text = err.to_string();
            assert!(text.contains("enable_raw"));
            assert!(text.contains("device busy"));
        }

        #[test]
        fn a_guard_formats_for_debug() {
            // Closes a Change Review finding: the hand-written Debug impl
            // (derive doesn't reach through the `&dyn TerminalOps` field) was
            // never actually exercised.
            let rec = Recorder::default();
            let guard = TerminalGuard::enter(&rec).expect("enter succeeds");
            let text = format!("{guard:?}");
            assert!(text.contains("TerminalGuard"));
        }
    }
}
