//! The terminal seam: raw mode and the alternate screen, entered and left
//! in a fixed, mirrored order, behind an injected `TerminalOps` trait so
//! this is testable with no real terminal. See
//! `openspec/changes/tui-shell/specs/terminal-lifecycle/spec.md`.

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

/// The four fallible terminal-mode operations. `CrosstermOps` is the one
/// implementation that touches a real terminal; every test uses a double.
pub trait TerminalOps {
    fn enable_raw(&self) -> Result<(), TerminalError>;
    fn enter_alternate(&self) -> Result<(), TerminalError>;
    fn leave_alternate(&self) -> Result<(), TerminalError>;
    fn disable_raw(&self) -> Result<(), TerminalError>;
}

/// Enters `enable_raw` then `enter_alternate` on construction, and calls
/// `leave_alternate` then `disable_raw` — the exact reverse — on drop,
/// ignoring each teardown operation's error rather than panicking inside
/// `Drop`. Restores exactly once: there is no way to clone, copy, or
/// re-drop the same guard.
pub struct TerminalGuard<'a> {
    ops: &'a dyn TerminalOps,
}

impl fmt::Debug for TerminalGuard<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TerminalGuard").finish_non_exhaustive()
    }
}

impl<'a> TerminalGuard<'a> {
    /// Enter raw mode then the alternate screen, in that order. If
    /// `enable_raw` fails, nothing else is attempted and there is no guard
    /// to undo. If `enter_alternate` fails, `disable_raw` is called before
    /// returning the error, so a partially entered terminal is never left
    /// behind.
    pub fn enter(ops: &'a dyn TerminalOps) -> Result<Self, TerminalError> {
        ops.enable_raw()?;
        if let Err(err) = ops.enter_alternate() {
            let _ = ops.disable_raw();
            return Err(err);
        }
        Ok(Self { ops })
    }
}

impl Drop for TerminalGuard<'_> {
    fn drop(&mut self) {
        restore_then(self.ops, &mut || {});
    }
}

/// Leave the alternate screen and disable raw mode, ignoring either
/// operation's error, then run `next`. This is the pure body both
/// `TerminalGuard::drop` and the panic hook installed by
/// [`install_panic_hook`] delegate to — `next` restores before the previous
/// panic hook runs, so the panic message lands on a cooked terminal.
pub fn restore_then(ops: &dyn TerminalOps, next: &mut dyn FnMut()) {
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
/// down. Chains to, never discards, the previous hook. Restores only when
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
        use std::collections::HashSet;
        use std::panic::AssertUnwindSafe;

        use crate::ui::terminal::{TerminalError, TerminalGuard, TerminalOps, restore_then};

        #[derive(Default)]
        struct Recorder {
            calls: RefCell<Vec<&'static str>>,
            failing: RefCell<HashSet<&'static str>>,
        }

        impl Recorder {
            fn fail(&self, op: &'static str) {
                self.failing.borrow_mut().insert(op);
            }

            fn calls(&self) -> Vec<&'static str> {
                self.calls.borrow().clone()
            }

            fn note(&self, label: &'static str) {
                self.calls.borrow_mut().push(label);
            }

            fn record(&self, op: &'static str) -> Result<(), TerminalError> {
                self.calls.borrow_mut().push(op);
                if self.failing.borrow().contains(op) {
                    Err(TerminalError {
                        op,
                        detail: "recorder configured to fail".to_string(),
                    })
                } else {
                    Ok(())
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
            fn leave_alternate(&self) -> Result<(), TerminalError> {
                self.record("leave_alternate")
            }
            fn disable_raw(&self) -> Result<(), TerminalError> {
                self.record("disable_raw")
            }
        }

        #[test]
        fn normal_lifetime_is_enter_enter_leave_disable() {
            let rec = Recorder::default();
            {
                let _guard = TerminalGuard::enter(&rec).expect("enter succeeds");
            }
            assert_eq!(
                rec.calls(),
                vec![
                    "enable_raw",
                    "enter_alternate",
                    "leave_alternate",
                    "disable_raw"
                ]
            );
        }

        #[test]
        fn enable_raw_failure_attempts_nothing_further() {
            let rec = Recorder::default();
            rec.fail("enable_raw");
            let err = TerminalGuard::enter(&rec).expect_err("enable_raw fails");
            assert_eq!(err.op, "enable_raw");
            assert_eq!(rec.calls(), vec!["enable_raw"]);
        }

        #[test]
        fn alternate_screen_failure_unwinds_raw_mode() {
            let rec = Recorder::default();
            rec.fail("enter_alternate");
            let err = TerminalGuard::enter(&rec).expect_err("enter_alternate fails");
            assert_eq!(err.op, "enter_alternate");
            assert_eq!(
                rec.calls(),
                vec!["enable_raw", "enter_alternate", "disable_raw"]
            );
        }

        #[test]
        fn teardown_errors_do_not_panic_and_both_are_attempted() {
            let rec = Recorder::default();
            rec.fail("leave_alternate");
            rec.fail("disable_raw");
            {
                let _guard = TerminalGuard::enter(&rec).expect("enter succeeds");
            }
            let calls = rec.calls();
            assert_eq!(
                &calls[calls.len() - 2..],
                ["leave_alternate", "disable_raw"]
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
                vec!["leave_alternate", "disable_raw", "previous_hook"]
            );
        }

        #[test]
        fn restore_then_delegates_even_when_both_restores_fail() {
            let rec = Recorder::default();
            rec.fail("leave_alternate");
            rec.fail("disable_raw");
            restore_then(&rec, &mut || rec.note("previous_hook"));
            assert_eq!(
                rec.calls(),
                vec!["leave_alternate", "disable_raw", "previous_hook"]
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

            assert_eq!(
                rec.calls(),
                vec!["leave_alternate", "disable_raw", "previous_hook"]
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
