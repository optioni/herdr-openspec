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

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use notify::Watcher as _;

use crate::changes::Selection;

/// The debounce window's length, exposed so `SPEC.md` -> Refresh's
/// "approximately 150ms" has one place it is written down.
pub const DEBOUNCE: Duration = Duration::from_millis(150);

/// The ceiling on how long a continuous writer can defer a batch. Without
/// it a *sliding* window — one that extends on every push with no cap —
/// starves indefinitely under a writer saving more often than every 150ms,
/// which is this change's own motivating case: an agent editing a change's
/// `tasks.md`, then its spec files, then its design.
pub const DEBOUNCE_MAX: Duration = Duration::from_secs(1);

/// A pure state machine coalescing touched paths over a [`DEBOUNCE`]
/// window, taking the current instant as a **parameter** on every method
/// rather than reading a clock — the one decision that answers hazard 1.
/// Paths are coalesced without duplicates in a deterministic order: FSEvents
/// emits several events for one edit, and the pane must not run the CLI
/// several times for one save.
#[derive(Debug, Default)]
pub struct Debounce {
    paths: std::collections::BTreeSet<PathBuf>,
    end: Option<Instant>,
    first_push: Option<Instant>,
}

impl Debounce {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record `paths` and set the window's end to `now + DEBOUNCE`,
    /// extending it on every later push — but never past
    /// `first_push + DEBOUNCE_MAX`.
    pub fn push(&mut self, paths: Vec<PathBuf>, now: Instant) {
        self.paths.extend(paths);
        let first_push = *self.first_push.get_or_insert(now);
        let capped_at = first_push + DEBOUNCE_MAX;
        let candidate = now + DEBOUNCE;
        self.end = Some(candidate.min(capped_at));
    }

    /// The accumulated paths, and empty the state — including `first_push`,
    /// so the next batch gets a fresh `DEBOUNCE_MAX` window — exactly when
    /// the window has ended. `None` otherwise, including when nothing was
    /// ever pushed.
    pub fn take_due(&mut self, now: Instant) -> Option<Vec<PathBuf>> {
        let end = self.end?;
        if now < end {
            return None;
        }
        self.end = None;
        self.first_push = None;
        Some(std::mem::take(&mut self.paths).into_iter().collect())
    }

    /// The time remaining before the window ends, saturating at zero, or
    /// `None` when nothing is pending. `end` is the receiver and `now` the
    /// argument: `Instant - Instant` panics when its argument is the later
    /// of the two, which `now` past `end` legitimately is.
    pub fn pending_in(&self, now: Instant) -> Option<Duration> {
        self.end.map(|end| end.saturating_duration_since(now))
    }
}

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

/// A touched path's classification relative to `<repo>/openspec/`. Pure and
/// total over `classify`'s two arguments: no filesystem, no
/// `canonicalize`, no clock, so a path that no longer exists classifies
/// exactly as one that does. See `specs/watch-invalidation/spec.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Touch {
    /// A path under `openspec/changes/<name>/`, at least one component past
    /// `<name>`, where `<name>` is not `archive`.
    Change(String),
    /// The change directory itself, the `changes` directory, `config.yaml`,
    /// `schemas/…`, or anything else at or below `openspec/` this rule does
    /// not otherwise recognise. Conservative on purpose: a wrong
    /// `Repository` costs one extra `list --json` on a cycle that was
    /// happening anyway; a wrong `Outside` would silently stop the pane
    /// updating.
    Repository,
    /// `openspec/changes/archive` or anything below it.
    Archived,
    /// Not below `<repo>/openspec/` at all, including a relative path.
    Outside,
}

/// Classify `path`'s position relative to `<repo>/openspec/`. Pure and
/// total: touches no filesystem, canonicalizes nothing, reads no clock.
pub fn classify(repo: &Path, path: &Path) -> Touch {
    use std::path::Component;

    let openspec_root = repo.join("openspec");
    let Ok(rel) = path.strip_prefix(&openspec_root) else {
        return Touch::Outside;
    };

    let mut components = rel.components();
    let Some(Component::Normal(first)) = components.next() else {
        // `openspec/` itself, or a path whose first segment is `.`/`..`/a
        // root — conservative: treat as a repository-level touch.
        return Touch::Repository;
    };
    if first != "changes" {
        return Touch::Repository;
    }
    let Some(Component::Normal(name)) = components.next() else {
        // `openspec/changes` itself.
        return Touch::Repository;
    };
    let name = name.to_string_lossy().into_owned();
    if name == "archive" {
        return Touch::Archived;
    }
    match components.next() {
        None => Touch::Repository, // `openspec/changes/<name>` exactly
        Some(_) => Touch::Change(name),
    }
}

/// Fold a batch of touched paths into the `Selection` the CLI producer
/// needs re-asked about: `All` when any path is `Repository` or `Archived`,
/// otherwise `Only` of the `Change` names — `Only` of the empty set when
/// every path is `Outside`, which is meaningful and not the same as doing
/// nothing: the worker still re-reads files and still runs
/// `openspec list --json`.
pub fn invalidate(repo: &Path, paths: &[PathBuf]) -> Selection {
    let mut names = std::collections::BTreeSet::new();
    for path in paths {
        match classify(repo, path) {
            Touch::Repository | Touch::Archived => return Selection::All,
            Touch::Change(name) => {
                names.insert(name);
            }
            Touch::Outside => {}
        }
    }
    Selection::Only(names)
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

/// The one real implementation: a `notify::RecommendedWatcher` (kept alive
/// for its `Drop`, which stops the watch), the `mpsc::Receiver` it sends
/// to, a [`Debounce`], and the `Duration` the previous `drain` computed for
/// [`FsEvents::pending_in`] to return verbatim — the crate's one binding
/// from the debounce's injected `now` to the real clock, `Instant::now()`,
/// alongside `cli::npm_prefix`, `config::env_lookup`, and
/// `ui::read_artifact`.
pub struct RealFsEvents {
    _watcher: notify::RecommendedWatcher,
    rx: std::sync::mpsc::Receiver<notify::Result<notify::Event>>,
    debounce: Debounce,
    pending: Option<Duration>,
}

impl FsEvents for RealFsEvents {
    fn drain(&mut self) -> Result<Option<Vec<PathBuf>>, WatchError> {
        let mut paths = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(Ok(event)) => paths.extend(event.paths),
                // A per-event error from notify's own backend (for
                // example, a dropped-event overflow) is not this pane's
                // failure to watch — it is skipped rather than surfaced,
                // the same way a single bad line does not fail an entire
                // read elsewhere in this crate.
                Ok(Err(_)) => {}
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    return Err(WatchError(
                        "the filesystem watcher's event channel disconnected".to_string(),
                    ));
                }
            }
        }
        // The crate's one real clock read, captured once and reused for
        // both the debounce and the cached `pending_in` this call leaves
        // behind — never a second call to `Instant::now()`.
        let now = Instant::now();
        if !paths.is_empty() {
            self.debounce.push(paths, now);
        }
        let due = self.debounce.take_due(now);
        self.pending = self.debounce.pending_in(now);
        Ok(due)
    }

    fn pending_in(&self) -> Option<Duration> {
        self.pending
    }
}

/// Start a recursive watch on `root`. Never returns a `Result`: a watcher
/// that will not start is a degraded state, not a failure to open the
/// pane, so it returns the inert implementation and a one-line problem
/// naming `root` and the reason instead.
pub fn start(root: &Path) -> (Box<dyn FsEvents>, Vec<String>) {
    let (tx, rx) = std::sync::mpsc::channel::<notify::Result<notify::Event>>();
    let watcher = match notify::Watcher::new(tx, notify::Config::default()) {
        Ok(w) => w,
        Err(e) => {
            return (
                none(),
                vec![format!(
                    "filesystem watch unavailable for {}: {e}",
                    root.display()
                )],
            );
        }
    };
    let mut watcher: notify::RecommendedWatcher = watcher;
    if let Err(e) = watcher.watch(root, notify::RecursiveMode::Recursive) {
        return (
            none(),
            vec![format!(
                "filesystem watch unavailable for {}: {e}",
                root.display()
            )],
        );
    }
    (
        Box::new(RealFsEvents {
            _watcher: watcher,
            rx,
            debounce: Debounce::new(),
            pending: None,
        }),
        Vec::new(),
    )
}

/// A pure function of two `Duration`s, reading no clock: `tick` when nothing
/// is pending, otherwise the smaller of `tick` and the remaining window,
/// floored at one millisecond. The floor is load-bearing rather than
/// cosmetic: a zero timeout returned every iteration would let `run_loop`
/// spin without bound if a watcher ever reported a pending batch it then
/// declined to yield. One millisecond converts an unbounded spin into a
/// bounded one — the correct case never reaches it, because the next
/// iteration's `drain` yields the batch and `pending_in` returns `None`
/// again.
pub fn poll_timeout(tick: Duration, pending_in: Option<Duration>) -> Duration {
    match pending_in {
        None => tick,
        Some(remaining) => tick.min(remaining).max(Duration::from_millis(1)),
    }
}

/// The minimum of two optional `Duration`s, reading no clock: `None` when both are
/// `None`, the present one when exactly one is, and the smaller when both are. One tick
/// serves two pollers — see `specs/watch-invalidation/spec.md` -> "One tick serves two
/// pollers".
pub fn soonest(a: Option<Duration>, b: Option<Duration>) -> Option<Duration> {
    match (a, b) {
        (None, None) => None,
        (Some(d), None) | (None, Some(d)) => Some(d),
        (Some(a), Some(b)) => Some(a.min(b)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(names: &[&str]) -> std::collections::BTreeSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_change_file_invalidates_only_that_change() {
        let repo = Path::new("/r");
        let result = invalidate(repo, &[PathBuf::from("/r/openspec/changes/alpha/tasks.md")]);
        assert_eq!(
            result,
            Selection::Only(set(&["alpha"])),
            "beta must not be in the set — an assertion that everything was \
             invalidated would pass either way and prove nothing"
        );

        let nested = invalidate(
            repo,
            &[PathBuf::from("/r/openspec/changes/alpha/specs/x/spec.md")],
        );
        assert_eq!(nested, Selection::Only(set(&["alpha"])));
    }

    #[test]
    fn a_nested_change_file_invalidates_only_that_change() {
        let repo = Path::new("/r");
        assert_eq!(
            classify(repo, Path::new("/r/openspec/changes/alpha/specs/x/spec.md")),
            Touch::Change("alpha".to_string())
        );
        assert_eq!(
            classify(repo, Path::new("/r/openspec/changes/alpha/tasks.md")),
            Touch::Change("alpha".to_string())
        );
    }

    #[test]
    fn the_change_directory_itself_is_a_repository_touch() {
        let repo = Path::new("/r");
        assert_eq!(
            classify(repo, Path::new("/r/openspec/changes/gamma")),
            Touch::Repository
        );
    }

    #[test]
    fn the_changes_directory_is_a_repository_touch() {
        let repo = Path::new("/r");
        assert_eq!(
            classify(repo, Path::new("/r/openspec/changes")),
            Touch::Repository
        );
    }

    #[test]
    fn archive_and_schema_are_repository_touches() {
        let repo = Path::new("/r");
        assert_eq!(
            classify(repo, Path::new("/r/openspec/changes/archive")),
            Touch::Archived
        );
        assert_eq!(
            classify(
                repo,
                Path::new("/r/openspec/changes/archive/2026-01-01-x/tasks.md")
            ),
            Touch::Archived
        );
        assert_eq!(
            classify(repo, Path::new("/r/openspec/config.yaml")),
            Touch::Repository
        );
        assert_eq!(
            classify(repo, Path::new("/r/openspec/schemas/tdd/schema.yaml")),
            Touch::Repository
        );

        let batch = invalidate(repo, &[PathBuf::from("/r/openspec/changes/archive")]);
        assert_eq!(batch, Selection::All);
        let batch2 = invalidate(
            repo,
            &[PathBuf::from(
                "/r/openspec/changes/archive/2026-01-01-x/tasks.md",
            )],
        );
        assert_eq!(batch2, Selection::All);
        let batch3 = invalidate(repo, &[PathBuf::from("/r/openspec/config.yaml")]);
        assert_eq!(batch3, Selection::All);
        let batch4 = invalidate(
            repo,
            &[PathBuf::from("/r/openspec/schemas/tdd/schema.yaml")],
        );
        assert_eq!(batch4, Selection::All);
    }

    #[test]
    fn a_path_outside_the_repository_is_outside() {
        let repo = Path::new("/r");
        assert_eq!(
            classify(repo, Path::new("/other/openspec/changes/alpha/tasks.md")),
            Touch::Outside
        );
        assert_eq!(classify(repo, Path::new("/r")), Touch::Outside);
        assert_eq!(classify(repo, Path::new("/r/README.md")), Touch::Outside);
    }

    #[test]
    fn a_relative_path_is_outside() {
        let repo = Path::new("/r");
        assert_eq!(
            classify(repo, Path::new("openspec/changes/alpha/tasks.md")),
            Touch::Outside
        );
    }

    #[test]
    fn an_empty_path_is_outside() {
        let repo = Path::new("/r");
        assert_eq!(classify(repo, Path::new("")), Touch::Outside);
    }

    #[test]
    fn a_mixed_batch_unions_its_classifications() {
        let repo = Path::new("/r");
        let result = invalidate(
            repo,
            &[
                PathBuf::from("/r/openspec/changes/alpha/tasks.md"),
                PathBuf::from("/r/openspec/changes/beta/design.md"),
                PathBuf::from("/tmp/unrelated"),
            ],
        );
        assert_eq!(result, Selection::Only(set(&["alpha", "beta"])));
    }

    #[test]
    fn a_repository_path_in_a_batch_absorbs_the_rest() {
        let repo = Path::new("/r");
        let result = invalidate(
            repo,
            &[
                PathBuf::from("/r/openspec/changes/alpha/tasks.md"),
                PathBuf::from("/r/openspec/changes/beta/design.md"),
                PathBuf::from("/tmp/unrelated"),
                PathBuf::from("/r/openspec/config.yaml"),
            ],
        );
        assert_eq!(
            result,
            Selection::All,
            "one repository-class path absorbs every per-change one"
        );
    }

    #[test]
    fn a_batch_of_only_outside_paths_is_an_empty_only() {
        let repo = Path::new("/r");
        let result = invalidate(
            repo,
            &[
                PathBuf::from("/other/openspec/changes/alpha/tasks.md"),
                PathBuf::from("/tmp/unrelated"),
            ],
        );
        assert_eq!(
            result,
            Selection::Only(std::collections::BTreeSet::new()),
            "an empty batch must not escalate to a full CLI reload"
        );
    }

    // --- group 5: the debounce, with `now` as a parameter -----------------
    //
    // Every test below captures `Instant::now()` exactly ONCE, as `t0`, and
    // asserts against fabricated instants derived from it. No sleep, no
    // second clock read — that is hazard 1 answered by a signature rather
    // than a convention.

    #[test]
    fn nothing_is_due_before_the_window() {
        let t0 = Instant::now();
        let mut d = Debounce::new();
        d.push(vec![PathBuf::from("a")], t0);
        assert_eq!(d.take_due(t0), None);
        assert_eq!(d.take_due(t0 + Duration::from_millis(149)), None);
    }

    #[test]
    fn the_batch_is_due_at_the_window() {
        let t0 = Instant::now();
        let mut d = Debounce::new();
        d.push(vec![PathBuf::from("a")], t0);
        assert_eq!(
            d.take_due(t0 + Duration::from_millis(150)),
            Some(vec![PathBuf::from("a")])
        );
    }

    #[test]
    fn taking_a_due_batch_empties_the_state() {
        let t0 = Instant::now();
        let mut d = Debounce::new();
        d.push(vec![PathBuf::from("a")], t0);
        assert_eq!(
            d.take_due(t0 + Duration::from_millis(150)),
            Some(vec![PathBuf::from("a")])
        );
        assert_eq!(d.take_due(t0 + Duration::from_secs(10)), None);
    }

    #[test]
    fn a_later_push_extends_the_window() {
        let t0 = Instant::now();
        let mut d = Debounce::new();
        d.push(vec![PathBuf::from("a")], t0);
        d.push(vec![PathBuf::from("b")], t0 + Duration::from_millis(100));

        assert_eq!(d.take_due(t0 + Duration::from_millis(150)), None);
        let due = d.take_due(t0 + Duration::from_millis(250));
        assert_eq!(due, Some(vec![PathBuf::from("a"), PathBuf::from("b")]));
    }

    #[test]
    fn pending_in_counts_down() {
        let t0 = Instant::now();
        let mut d = Debounce::new();
        d.push(vec![PathBuf::from("a")], t0);
        d.push(vec![PathBuf::from("b")], t0 + Duration::from_millis(100));

        assert_eq!(
            d.pending_in(t0 + Duration::from_millis(100)),
            Some(Duration::from_millis(150))
        );
        assert_eq!(
            d.pending_in(t0 + Duration::from_millis(250)),
            Some(Duration::from_millis(0))
        );
        // Past the window's end: saturates at zero rather than panicking,
        // which the obvious `end - now` would do (`Instant - Instant`
        // panics when its argument is the later of the two).
        assert_eq!(
            d.pending_in(t0 + Duration::from_millis(400)),
            Some(Duration::from_millis(0))
        );
    }

    #[test]
    fn pending_in_is_none_when_empty() {
        let t0 = Instant::now();
        let d = Debounce::new();
        assert_eq!(d.pending_in(t0), None);
    }

    #[test]
    fn repeated_paths_are_coalesced() {
        let t0 = Instant::now();
        let mut d = Debounce::new();
        d.push(
            vec![
                PathBuf::from("a"),
                PathBuf::from("a"),
                PathBuf::from("b"),
                PathBuf::from("a"),
            ],
            t0,
        );
        d.push(vec![PathBuf::from("b")], t0 + Duration::from_millis(10));

        let due = d.take_due(t0 + Duration::from_millis(160));
        assert_eq!(due, Some(vec![PathBuf::from("a"), PathBuf::from("b")]));
    }

    #[test]
    fn a_continuous_writer_cannot_defer_forever() {
        let t0 = Instant::now();
        let mut d = Debounce::new();
        for i in 0..=11 {
            d.push(
                vec![PathBuf::from(format!("f{i}"))],
                t0 + Duration::from_millis(100 * i),
            );
        }
        let due = d.take_due(t0 + Duration::from_secs(1));
        assert!(due.is_some(), "the window's end was capped at DEBOUNCE_MAX");
        assert_eq!(due.unwrap().len(), 12, "every path pushed up to that point");

        // The cap was reset with the taken batch: a fresh push is not due
        // at its own instant, and is due 150ms later.
        d.push(vec![PathBuf::from("c")], t0 + Duration::from_secs(2));
        assert_eq!(d.take_due(t0 + Duration::from_secs(2)), None);
        assert_eq!(
            d.take_due(t0 + Duration::from_secs(2) + Duration::from_millis(150)),
            Some(vec![PathBuf::from("c")])
        );

        assert_eq!(DEBOUNCE_MAX, Duration::from_secs(1));
    }

    #[test]
    fn debounce_window_is_150_milliseconds() {
        assert_eq!(DEBOUNCE, Duration::from_millis(150));
    }

    #[test]
    fn poll_timeout_is_the_tick_when_nothing_is_pending() {
        assert_eq!(
            poll_timeout(Duration::from_millis(250), None),
            Duration::from_millis(250)
        );
    }

    #[test]
    fn poll_timeout_is_the_remaining_window_and_never_zero() {
        assert_eq!(
            poll_timeout(Duration::from_millis(250), Some(Duration::from_millis(90))),
            Duration::from_millis(90)
        );
        assert_eq!(
            poll_timeout(Duration::from_millis(250), Some(Duration::from_millis(400))),
            Duration::from_millis(250)
        );
        assert_eq!(
            poll_timeout(Duration::from_millis(250), Some(Duration::from_millis(0))),
            Duration::from_millis(1)
        );
    }

    // --- group 6: `soonest` — one tick serves two pollers -------------

    #[test]
    fn soonest_of_two_absent_is_absent() {
        assert_eq!(soonest(None, None), None);
    }

    #[test]
    fn soonest_of_one_present_is_that_one() {
        assert_eq!(
            soonest(Some(Duration::from_millis(90)), None),
            Some(Duration::from_millis(90))
        );
        assert_eq!(
            soonest(None, Some(Duration::from_millis(40))),
            Some(Duration::from_millis(40))
        );
    }

    #[test]
    fn soonest_of_two_present_is_the_smaller_and_commutative() {
        assert_eq!(
            soonest(
                Some(Duration::from_millis(90)),
                Some(Duration::from_millis(40))
            ),
            Some(Duration::from_millis(40))
        );
        assert_eq!(
            soonest(
                Some(Duration::from_millis(40)),
                Some(Duration::from_millis(90))
            ),
            Some(Duration::from_millis(40)),
            "soonest must be commutative"
        );
    }

    #[test]
    fn soonest_composes_with_poll_timeout() {
        assert_eq!(
            poll_timeout(
                Duration::from_millis(250),
                soonest(
                    Some(Duration::from_millis(900)),
                    Some(Duration::from_millis(120))
                )
            ),
            Duration::from_millis(120),
            "an agent poll becoming due inside the tick shortens the wait exactly as a \
             debounce deadline does"
        );
        assert_eq!(
            poll_timeout(
                Duration::from_millis(250),
                soonest(
                    Some(Duration::from_millis(900)),
                    Some(Duration::from_secs(3))
                )
            ),
            Duration::from_millis(250),
            "a deadline further away than the tick must never lengthen the wait"
        );
    }

    // --- group 6: the real watcher ------------------------------------

    #[test]
    fn no_fs_events_never_yields() {
        let mut fs = none();
        for _ in 0..10 {
            assert_eq!(fs.drain(), Ok(None));
            assert_eq!(fs.pending_in(), None);
        }
    }

    #[test]
    fn watch_error_display_names_the_reason() {
        let err = WatchError("boom".to_string());
        assert_eq!(err.to_string(), "boom");
    }

    #[test]
    fn start_on_a_missing_path_degrades_and_names_the_reason() {
        let scratch = crate::testutil::ScratchDir::new();
        let missing = scratch.path().join("does-not-exist");
        let (mut fs, problems) = start(&missing);

        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains(&missing.display().to_string()));

        // The returned FsEvents is the inert one, forever.
        for _ in 0..10 {
            assert_eq!(fs.drain(), Ok(None));
            assert_eq!(fs.pending_in(), None);
        }
    }

    #[test]
    fn start_on_a_real_directory_yields_the_touched_path() {
        let scratch = crate::testutil::ScratchDir::new();
        let root = scratch.path();
        std::fs::create_dir_all(root.join("changes/alpha")).expect("create fixture dir");
        let (mut fs, problems) = start(root);
        assert!(problems.is_empty(), "{problems:?}");

        let target = root.join("changes/alpha/tasks.md");
        std::fs::write(&target, "- [x] a\n").expect("write fixture file");

        // Deadline-bounded poll, never a fixed sleep followed by an
        // assertion: the condition is re-tested after every 10ms sleep, so
        // a cold or loaded machine cannot make this assertion premature.
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut found = false;
        while Instant::now() < deadline {
            if let Ok(Some(paths)) = fs.drain()
                && paths.iter().any(|p| {
                    let mut components: Vec<_> = p
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy().into_owned())
                        .collect();
                    components.len() >= 2
                        && components.pop().as_deref() == Some("tasks.md")
                        && components.pop().as_deref() == Some("alpha")
                })
            {
                found = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            found,
            "no event named .../alpha/tasks.md arrived within 10s; \
             this filesystem may not support change notification"
        );
    }

    #[test]
    fn a_started_watcher_writes_nothing() {
        let scratch = crate::testutil::ScratchDir::new();
        let root = scratch.path();
        std::fs::create_dir_all(root.join("changes/alpha")).expect("create fixture dir");
        std::fs::write(root.join("changes/alpha/tasks.md"), "- [x] a\n")
            .expect("write fixture file");

        let before = crate::testutil::snapshot(root);
        let (fs, _problems) = start(root);
        drop(fs);
        let after = crate::testutil::snapshot(root);
        assert_eq!(
            before, after,
            "starting and dropping a watch wrote inside the tree"
        );
    }
}
