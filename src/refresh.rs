//! The worker thread: the only module in the crate spawning a thread. See
//! `specs/refresh-worker/spec.md` and `openspec/changes/live-refresh/design.md`
//! -> Boundaries. Reaches the `openspec` program only through
//! `Arc<dyn crate::cli::OpenspecCli>` — it spawns no process, and
//! `src/cli.rs` stays the crate's one spawn site.
//!
//! Group 1 populates only the inert halves: the two result variants, the
//! trait, and a stub free function with no thread and no rule inside it. The
//! real worker arrives in group 7.

use crate::changes::ChangeSet;

/// One request's two answers, in the order the worker sends them: the
/// file-sourced set first, sub-millisecond; the CLI-merged one 200-400ms
/// later. Neither variant names `CliChanges`, `OpenspecCli`, or `from_cli`,
/// so `src/ui/driver.rs` can hold a `&mut dyn Refresher` while `NOCLI-SHELL`
/// stays green unweakened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshResult {
    Files(ChangeSet),
    Merged(ChangeSet),
}

/// The non-blocking seam between the render path and the worker thread.
/// Neither method may block, sleep, join a thread, or wait on a channel. See
/// `specs/refresh-worker/spec.md`.
pub trait Refresher {
    /// Ask for a refresh over `selection`. Records the request; does not
    /// wait for an answer.
    fn request(&mut self, selection: crate::changes::Selection);
    /// The next result the worker produced, if one is ready. Never blocks:
    /// implemented with `try_recv`, never `recv`, `recv_timeout`, or a `for`
    /// loop over a receiver.
    fn take_result(&mut self) -> Option<RefreshResult>;
}

/// The inert implementation: `request` records nothing and does nothing,
/// `take_result` is always `None`. No thread is spawned and no process is
/// started.
struct NoRefresher;

impl Refresher for NoRefresher {
    fn request(&mut self, _selection: crate::changes::Selection) {}

    fn take_result(&mut self) -> Option<RefreshResult> {
        None
    }
}

/// The inert `Refresher`. See [`NoRefresher`].
pub fn none() -> Box<dyn Refresher> {
    Box::new(NoRefresher)
}

// The one line-anchored `#[cfg(test)]` `NOBLOCK` and `READONLY-UI` require of
// this file (their Guard D). Left empty in group 1, which adds no thread and
// therefore no test: `refresh::tests::` starts truly empty here, so group
// 7's eight-test floor is measured against zero.
#[cfg(test)]
mod tests {}
