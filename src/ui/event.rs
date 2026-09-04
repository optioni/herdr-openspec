//! The real event stream. See
//! `openspec/changes/tui-shell/specs/dashboard-loop/spec.md`.

use std::time::Duration;

use ratatui::crossterm::event::Event;

/// An event-source error's message. Not `std::io::Error`, so a test double
/// can produce one without fabricating an I/O error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventError(pub String);

/// A source of terminal events. `CrosstermEvents` is the one implementation
/// that reads the real event stream; every test uses a scripted double.
pub trait EventSource {
    /// Wait up to `timeout` for the next event. `Ok(None)` is a timeout —
    /// never itself an event and never ending the loop.
    fn next_event(&mut self, timeout: Duration) -> Result<Option<Event>, EventError>;
}

/// The real event stream: poll then read. Two lines, no branch of its own
/// beyond the poll result. Not unit tested — see design.md -> Test Strategy,
/// which names this type and the terminal seam's real implementation as
/// the two deliberately-uncovered one-line bindings in the crate.
pub struct CrosstermEvents;

impl EventSource for CrosstermEvents {
    fn next_event(&mut self, timeout: Duration) -> Result<Option<Event>, EventError> {
        if ratatui::crossterm::event::poll(timeout).map_err(|e| EventError(e.to_string()))? {
            Ok(Some(
                ratatui::crossterm::event::read().map_err(|e| EventError(e.to_string()))?,
            ))
        } else {
            Ok(None)
        }
    }
}
