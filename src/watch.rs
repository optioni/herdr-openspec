//! The filesystem watch: the only module in the crate naming `notify`. See
//! `specs/watch-invalidation/spec.md` and `openspec/changes/live-refresh/design.md`
//! -> Boundaries. Confined on exactly `src/cli.rs`'s (`NOSPAWN-GREP`) and
//! `src/ui/markdown.rs`'s (`MDSEAM`) single-file terms: the watch crate is
//! replaceable by editing this one file, and no other file in the crate —
//! `tests/` included — may reach for it. The module is plain data: it names
//! no view type and spawns no process.
//!
//! Group 1 populates only the inert halves: the trait, the error type, and
//! stub free functions with no rule inside them. The real debounce arrives
//! in groups 4-6.

use std::path::PathBuf;
use std::time::Duration;

/// Why a watch could not be started, or why a drain failed. Carries the
/// reason as text, exactly as `WatchError`'s callers need it for a
/// `!`-marked problem row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchError(pub String);

impl std::fmt::Display for WatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A non-blocking source of filesystem touches. Every method SHALL be
/// non-blocking — the render path calls `drain` and `pending_in` on every
/// frame, and neither may wait on anything. See `specs/watch-invalidation/spec.md`.
pub trait FsEvents: Send {
    /// A debounced batch of touched paths, or `None` when nothing is ready
    /// yet. Never blocks, never sleeps, never waits on a channel. Never
    /// returns `Ok(Some(vec![]))`: an empty batch and no batch are the same
    /// fact.
    fn drain(&mut self) -> Result<Option<Vec<PathBuf>>, WatchError>;
    /// The time remaining before the next batch becomes due, or `None` when
    /// nothing is pending. Takes no `Instant`: the real implementation
    /// returns the value its own last `drain` already computed.
    fn pending_in(&self) -> Option<Duration>;
}

/// The inert implementation: `drain` is always `Ok(None)`, `pending_in` is
/// always `None`. What `watch::start` returns on failure, and what `ui::run`
/// passes when no repository was found.
struct NoFsEvents;

impl FsEvents for NoFsEvents {
    fn drain(&mut self) -> Result<Option<Vec<PathBuf>>, WatchError> {
        Ok(None)
    }

    fn pending_in(&self) -> Option<Duration> {
        None
    }
}

/// The inert `FsEvents`. See [`NoFsEvents`].
pub fn none() -> Box<dyn FsEvents> {
    Box::new(NoFsEvents)
}

/// A pure function of two `Duration`s, reading no clock: `tick` when nothing
/// is pending, otherwise the smaller of `tick` and the remaining window,
/// floored at one millisecond so a faulty `FsEvents` degrades to a hot pane
/// rather than a hung one. This stub always returns `tick`; the real rule
/// arrives in group 5.
pub fn poll_timeout(tick: Duration, pending_in: Option<Duration>) -> Duration {
    let _ = pending_in;
    tick
}

// The one line-anchored `#[cfg(test)]` `NOBLOCK` and `READONLY-UI` require of
// this file (their Guard D). Left empty in group 1, which adds no behaviour
// and therefore no test: `watch::tests::` starts truly empty here, so
// group 4's eleven-test floor is measured against zero, not against a
// placeholder this group would otherwise have left behind.
#[cfg(test)]
mod tests {}
