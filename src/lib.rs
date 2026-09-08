//! Pure invocation classification for the `herdr-openspec` binary.
//!
//! Nothing here performs I/O: `parse` turns argument strings into an
//! [`Invocation`], and `usage` and `rejection_text` produce the strings
//! `src/main.rs` writes to stderr. Keeping this logic here rather than in
//! `main` is what makes it unit-testable — see design.md -> Decisions
//! ("Library plus thin `main`").

pub mod agents;
pub mod changes;
pub mod cli;
pub mod config;
pub mod launch;
pub mod open;
pub mod refresh;
pub mod resolve;
pub mod schema;
pub mod state;
pub mod tasks;
pub mod ui;
pub mod watch;

/// The current process id. Exists so `state::record`'s temporary-file name
/// can include it without `src/state.rs` itself naming the standard-library
/// process module: that module is checked by the stricter, module-scoped
/// form of the "resolution spawns nothing" gate that applies to
/// `src/config.rs` and `src/state.rs` only, forbidding any process API there
/// at all — spawning or not. See `openspec/changes/plugin-config/design.md`
/// -> Boundaries and Test Strategy for the exact check (deliberately not
/// reproduced here, so this comment cannot itself trip the tree-wide half of
/// that same check). Reading the current process id is not a spawn.
pub(crate) fn pid() -> u32 {
    std::process::id()
}

/// Test-only helpers shared by `config` and `state`'s unit tests.
#[cfg(test)]
pub(crate) mod testutil {
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A uniquely named directory under `std::env::temp_dir()`, created on
    /// construction and removed (recursively) on drop. Named
    /// `herdr-openspec-test-<pid>-<counter>` — predictable, so a test can assert a
    /// path's absence up front and a leak-check can search for the prefix if a
    /// panicking test ever leaves one behind. No `tempfile` dependency: see
    /// `openspec/changes/plugin-config/design.md` -> Decisions.
    pub(crate) struct ScratchDir {
        path: PathBuf,
    }

    impl ScratchDir {
        pub(crate) fn new() -> Self {
            static COUNTER: AtomicUsize = AtomicUsize::new(0);
            let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("herdr-openspec-test-{}-{counter}", crate::pid()));
            std::fs::create_dir_all(&path).expect("create scratch dir");
            Self { path }
        }

        pub(crate) fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    /// A snapshot of a directory's listing, every file's bytes, and every file's
    /// modification time — read through [`std::fs::Metadata`], never by shelling
    /// out to `stat`, whose flags differ between BSD and GNU. Two snapshots taken
    /// around an operation and compared for equality is how "the tree is
    /// untouched" is proven, rather than by checking the listing alone.
    ///
    /// Directory entries are recorded too (path, empty bytes, own modification
    /// time), not only files: `collect` originally pushed an entry only for a
    /// non-directory, so an empty directory contributed nothing and a
    /// `create_dir_all` — the likeliest accidental write — was invisible to a
    /// comparison of two snapshots. `config` and `state`'s existing snapshot
    /// assertions are equality comparisons over two snapshots taken with the
    /// same (extended) function, so they stay green with directory entries added
    /// to both sides.
    #[derive(Debug, PartialEq, Eq)]
    pub(crate) struct Snapshot(Vec<(PathBuf, Vec<u8>, std::time::SystemTime)>);

    pub(crate) fn snapshot(dir: &Path) -> Snapshot {
        let mut entries = Vec::new();
        collect(dir, &mut entries);
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        Snapshot(entries)
    }

    fn collect(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>, std::time::SystemTime)>) {
        let Ok(read_dir) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Ok(metadata) = entry.metadata() {
                    let mtime = metadata.modified().expect("modified time");
                    out.push((path.clone(), Vec::new(), mtime));
                }
                collect(&path, out);
            } else if let Ok(metadata) = entry.metadata() {
                let bytes = std::fs::read(&path).unwrap_or_default();
                let mtime = metadata.modified().expect("modified time");
                out.push((path, bytes, mtime));
            }
        }
    }

    /// A *shallow* (non-recursive) snapshot of a directory's direct entries:
    /// each child's path, whether it is a directory, and its modification
    /// time — never its bytes, and never anything below it. Reserved for a
    /// directory whose full recursive [`snapshot`] would be prohibitively
    /// expensive or unstable to take, such as the test process's own
    /// working directory under `cargo test`, which is this crate's own
    /// repository root — including a `target/` directory that can hold tens
    /// of thousands of build-artifact files and hundreds of megabytes,
    /// changing across unrelated builds. See
    /// `openspec/changes/subprocess-seam/design.md` -> Test Strategy and
    /// -> Decisions for why the "not in the working directory" clause of
    /// "running a program writes nothing" is proven with this shallow form
    /// rather than [`snapshot`]: nothing in `cli` computes any path
    /// relative to the current directory, so a stray write this seam
    /// caused would surface as a new, removed, or modified **top-level**
    /// entry — the one thing this snapshot would catch.
    #[derive(Debug, PartialEq, Eq)]
    pub(crate) struct ShallowSnapshot(Vec<(PathBuf, bool, std::time::SystemTime)>);

    pub(crate) fn shallow_snapshot(dir: &Path) -> ShallowSnapshot {
        let mut entries = Vec::new();
        if let Ok(read_dir) = std::fs::read_dir(dir) {
            for entry in read_dir.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    let mtime = metadata.modified().expect("modified time");
                    entries.push((entry.path(), metadata.is_dir(), mtime));
                }
            }
        }
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        ShallowSnapshot(entries)
    }

    /// Write `contents` to `path`, creating parent directories as needed, and
    /// set its permission bits explicitly to `mode`. Never relies on the
    /// process umask, which differs between an interactive shell and a CI
    /// runner and would make "mode 0644" mean something else depending on who
    /// runs the suite. The fixture "binary" this repository's tests build is
    /// always a short text file, never a copied real executable.
    pub(crate) fn write_with_mode(path: &Path, contents: &[u8], mode: u32) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create fixture parent directory");
        }
        std::fs::write(path, contents).expect("write fixture file");
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
            .expect("set fixture file mode");
    }

    /// Create a symbolic link at `link` pointing to `original`, creating
    /// `link`'s parent directories as needed. `original` need not exist —
    /// several scenarios need a dangling link.
    pub(crate) fn symlink(original: &Path, link: &Path) {
        if let Some(parent) = link.parent() {
            std::fs::create_dir_all(parent).expect("create fixture parent directory");
        }
        std::os::unix::fs::symlink(original, link).expect("create fixture symlink");
    }

    /// Canonicalize `path`, on the assumption that it exists. Every discovery
    /// assertion compares against `canonicalize(expected)` rather than a raw
    /// `ScratchDir` path: on macOS `std::env::temp_dir()` sits under
    /// `/var/folders/...`, and `/var` is a symlink to `/private/var`, so an
    /// assertion against the raw path passes on Linux and fails on macOS.
    pub(crate) fn canonical(path: &Path) -> PathBuf {
        std::fs::canonicalize(path).expect("canonicalize expected path")
    }

    /// Render `dashboard` at `width`x`height` through `ui::view::render` into
    /// a `ratatui::backend::TestBackend`, and return the resulting buffer.
    /// Every view scenario in this change and in every later view change
    /// uses this, `row_text`, and `cell` — never a real terminal.
    pub(crate) fn render_at(
        width: u16,
        height: u16,
        dashboard: &crate::ui::app::Dashboard,
    ) -> ratatui::buffer::Buffer {
        let backend = ratatui::backend::TestBackend::new(width, height);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct TestBackend terminal");
        terminal
            .draw(|frame| crate::ui::view::render(frame, dashboard))
            .expect("draw a frame into the TestBackend");
        terminal.backend().buffer().clone()
    }

    /// Read row `y` of `buffer` as a `String`, one character per column.
    pub(crate) fn row_text(buffer: &ratatui::buffer::Buffer, y: u16) -> String {
        (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol().to_string())
            .collect()
    }

    /// The cell at `(x, y)`, for reading its symbol and `Style`.
    pub(crate) fn cell(buffer: &ratatui::buffer::Buffer, x: u16, y: u16) -> &ratatui::buffer::Cell {
        &buffer[(x, y)]
    }

    /// A scripted `EventSource`: returns each queued result in order,
    /// recording every call's `timeout`, and errors with a named message
    /// once the script is exhausted — never `Ok(None)` forever, which would
    /// turn a loop bug into a hung test. Lifted from `src/ui/driver.rs`'s
    /// private test module, which `crate::ui::tests::detail` — a sibling
    /// module, not a descendant — cannot reach: `mod tests` there carries no
    /// visibility modifier, so path privacy makes even a `pub(crate)` item
    /// inside it unreachable from outside `ui::driver`. This is the one
    /// shape that compiles from every test module that needs it.
    pub(crate) struct Script {
        queue: std::cell::RefCell<
            Vec<Result<Option<ratatui::crossterm::event::Event>, crate::ui::event::EventError>>,
        >,
        timeouts: std::cell::RefCell<Vec<std::time::Duration>>,
    }

    impl Script {
        pub(crate) fn new(
            queue: Vec<
                Result<Option<ratatui::crossterm::event::Event>, crate::ui::event::EventError>,
            >,
        ) -> Self {
            Self {
                queue: std::cell::RefCell::new(queue),
                timeouts: std::cell::RefCell::new(Vec::new()),
            }
        }

        pub(crate) fn calls(&self) -> usize {
            self.timeouts.borrow().len()
        }

        pub(crate) fn timeouts(&self) -> Vec<std::time::Duration> {
            self.timeouts.borrow().clone()
        }
    }

    impl crate::ui::event::EventSource for Script {
        fn next_event(
            &mut self,
            timeout: std::time::Duration,
        ) -> Result<Option<ratatui::crossterm::event::Event>, crate::ui::event::EventError>
        {
            self.timeouts.borrow_mut().push(timeout);
            if self.queue.borrow().is_empty() {
                return Err(crate::ui::event::EventError("script exhausted".to_string()));
            }
            self.queue.borrow_mut().remove(0)
        }
    }

    /// An in-memory `ArtifactReader` double: constructed from a list of
    /// `(path, Result<String, String>)` entries plus a default result for
    /// any path not named there, it records every call's path in order and
    /// exposes `calls()` and `paths()`. Performs no I/O, so it can be used
    /// from any test module without a scratch directory. Because
    /// `crate::ui::app::ArtifactReader` (`&dyn Fn`) is not a `&mut`
    /// receiver, calls are recorded into a `RefCell`; every call site wraps
    /// this in a closure — `let read = |p: &Path| recorder.read(p);` then
    /// `&read` — which is the shape every test in groups 8, 10, and 11
    /// repeats. This is the double that makes "was not read again"
    /// assertable at all: a real filesystem has no way to prove a path was
    /// *not* read.
    pub(crate) struct RecordingReader {
        scripted: Vec<(PathBuf, Result<String, String>)>,
        default: Result<String, String>,
        calls: std::cell::RefCell<Vec<PathBuf>>,
    }

    impl RecordingReader {
        pub(crate) fn new(
            scripted: Vec<(PathBuf, Result<String, String>)>,
            default: Result<String, String>,
        ) -> Self {
            Self {
                scripted,
                default,
                calls: std::cell::RefCell::new(Vec::new()),
            }
        }

        /// A reader that returns `default` for every path — the common case
        /// where every call resolves to the same text or the same error.
        pub(crate) fn always(default: Result<String, String>) -> Self {
            Self::new(Vec::new(), default)
        }

        pub(crate) fn read(&self, path: &Path) -> Result<String, String> {
            self.calls.borrow_mut().push(path.to_path_buf());
            self.scripted
                .iter()
                .find(|(p, _)| p == path)
                .map(|(_, r)| r.clone())
                .unwrap_or_else(|| self.default.clone())
        }

        pub(crate) fn calls(&self) -> usize {
            self.calls.borrow().len()
        }

        pub(crate) fn paths(&self) -> Vec<PathBuf> {
            self.calls.borrow().clone()
        }
    }

    /// A key-press `Event`, for building a `Script`'s queue.
    pub(crate) fn press(
        code: ratatui::crossterm::event::KeyCode,
        modifiers: ratatui::crossterm::event::KeyModifiers,
    ) -> ratatui::crossterm::event::Event {
        ratatui::crossterm::event::Event::Key(ratatui::crossterm::event::KeyEvent::new(
            code, modifiers,
        ))
    }

    /// A scripted `watch::FsEvents` double, on `Script`'s terms: a queue of
    /// `drain` results and a queue of `pending_in` results, each recording
    /// every call. Synchronous and thread-free — it answers from a `RefCell`
    /// held queue, spawns nothing, sleeps nothing, and reads no clock, so
    /// every `ui::` test that drives the live tier stays deterministic.
    /// Exhausting either queue yields `Ok(None)` / `None` forever rather than
    /// panicking: an `FsEvents` that errors once the script runs out would
    /// end a test for the wrong reason.
    pub(crate) struct ScriptedFs {
        drain_queue: std::cell::RefCell<
            std::collections::VecDeque<Result<Option<Vec<PathBuf>>, crate::watch::WatchError>>,
        >,
        pending_queue: std::cell::RefCell<std::collections::VecDeque<Option<std::time::Duration>>>,
        drain_calls:
            std::cell::RefCell<Vec<Result<Option<Vec<PathBuf>>, crate::watch::WatchError>>>,
        pending_calls: std::cell::RefCell<Vec<Option<std::time::Duration>>>,
    }

    impl ScriptedFs {
        pub(crate) fn new(
            drains: Vec<Result<Option<Vec<PathBuf>>, crate::watch::WatchError>>,
            pendings: Vec<Option<std::time::Duration>>,
        ) -> Self {
            Self {
                drain_queue: std::cell::RefCell::new(drains.into()),
                pending_queue: std::cell::RefCell::new(pendings.into()),
                drain_calls: std::cell::RefCell::new(Vec::new()),
                pending_calls: std::cell::RefCell::new(Vec::new()),
            }
        }

        /// Every result `drain` returned, in call order.
        pub(crate) fn drains(&self) -> Vec<Result<Option<Vec<PathBuf>>, crate::watch::WatchError>> {
            self.drain_calls.borrow().clone()
        }

        /// Every result `pending_in` returned, in call order.
        pub(crate) fn pendings(&self) -> Vec<Option<std::time::Duration>> {
            self.pending_calls.borrow().clone()
        }
    }

    impl crate::watch::FsEvents for ScriptedFs {
        fn drain(&mut self) -> Result<Option<Vec<PathBuf>>, crate::watch::WatchError> {
            let result = self
                .drain_queue
                .borrow_mut()
                .pop_front()
                .unwrap_or(Ok(None));
            self.drain_calls.borrow_mut().push(result.clone());
            result
        }

        fn pending_in(&self) -> Option<std::time::Duration> {
            let result = self.pending_queue.borrow_mut().pop_front().unwrap_or(None);
            self.pending_calls.borrow_mut().push(result);
            result
        }
    }

    /// A recording `refresh::Refresher` double: a queue of `take_result`
    /// answers, plus `requests()` and `takes()`. Synchronous and
    /// thread-free, on `ScriptedFs`'s terms — it is what makes "exactly one
    /// request, carrying `Selection::All`" assertable at all, since a real
    /// worker's timing cannot be pinned down deterministically.
    pub(crate) struct RecordingRefresher {
        queue:
            std::cell::RefCell<std::collections::VecDeque<Option<crate::refresh::RefreshResult>>>,
        requests: std::cell::RefCell<Vec<crate::changes::Selection>>,
        takes: std::cell::RefCell<usize>,
    }

    impl RecordingRefresher {
        pub(crate) fn new(results: Vec<Option<crate::refresh::RefreshResult>>) -> Self {
            Self {
                queue: std::cell::RefCell::new(results.into()),
                requests: std::cell::RefCell::new(Vec::new()),
                takes: std::cell::RefCell::new(0),
            }
        }

        /// Every `Selection` passed to `request`, in call order.
        pub(crate) fn requests(&self) -> Vec<crate::changes::Selection> {
            self.requests.borrow().clone()
        }

        /// The number of times `take_result` was called.
        pub(crate) fn takes(&self) -> usize {
            *self.takes.borrow()
        }
    }

    impl crate::refresh::Refresher for RecordingRefresher {
        // `list-sections` group 2 note: `Refresher::request` gained a second
        // parameter, `archived: ArchivedScope`, but every `src/ui/driver.rs`
        // assertion against `requests()` predates the archived section and
        // checks `Selection` alone — so `archived` is recorded nowhere and
        // `requests()` keeps its pre-`list-sections` signature. Nothing in
        // this crate reads this double's archived scope until a later group
        // gives `ui::driver`'s own tests a reason to.
        fn request(
            &mut self,
            selection: crate::changes::Selection,
            _archived: crate::changes::ArchivedScope,
        ) {
            self.requests.borrow_mut().push(selection);
        }

        fn take_result(&mut self) -> Option<crate::refresh::RefreshResult> {
            *self.takes.borrow_mut() += 1;
            self.queue.borrow_mut().pop_front().unwrap_or(None)
        }
    }

    /// A scripted `agents::AgentPoll` double, on `ScriptedFs`'s terms: a queue of
    /// `drain` results and a queue of `pending_in` results, each recording every
    /// call. Synchronous and thread-free — spawns nothing, sleeps nothing, reads
    /// no clock — so every `ui::` test that drives the live tier's third
    /// collaborator stays deterministic. Exhausting either queue yields
    /// `None` forever rather than panicking.
    pub(crate) struct ScriptedAgents {
        drain_queue:
            std::cell::RefCell<std::collections::VecDeque<Option<crate::agents::AgentSnapshot>>>,
        pending_queue: std::cell::RefCell<std::collections::VecDeque<Option<std::time::Duration>>>,
        drain_calls: std::cell::RefCell<Vec<Option<crate::agents::AgentSnapshot>>>,
        pending_calls: std::cell::RefCell<Vec<Option<std::time::Duration>>>,
    }

    impl ScriptedAgents {
        pub(crate) fn new(
            drains: Vec<Option<crate::agents::AgentSnapshot>>,
            pendings: Vec<Option<std::time::Duration>>,
        ) -> Self {
            Self {
                drain_queue: std::cell::RefCell::new(drains.into()),
                pending_queue: std::cell::RefCell::new(pendings.into()),
                drain_calls: std::cell::RefCell::new(Vec::new()),
                pending_calls: std::cell::RefCell::new(Vec::new()),
            }
        }

        /// Every result `drain` returned, in call order.
        pub(crate) fn drains(&self) -> Vec<Option<crate::agents::AgentSnapshot>> {
            self.drain_calls.borrow().clone()
        }

        /// Every result `pending_in` returned, in call order.
        pub(crate) fn pendings(&self) -> Vec<Option<std::time::Duration>> {
            self.pending_calls.borrow().clone()
        }
    }

    impl crate::agents::AgentPoll for ScriptedAgents {
        fn drain(&mut self) -> Option<crate::agents::AgentSnapshot> {
            let result = self.drain_queue.borrow_mut().pop_front().unwrap_or(None);
            self.drain_calls.borrow_mut().push(result.clone());
            result
        }

        fn pending_in(&self) -> Option<std::time::Duration> {
            let result = self.pending_queue.borrow_mut().pop_front().unwrap_or(None);
            self.pending_calls.borrow_mut().push(result);
            result
        }
    }

    /// A recording `launch::Launcher` double: records every `request`, on `RecordingRefresher`'s
    /// terms — `drain` always answers `None`, so this double is for asserting on the recorded
    /// request vector, never on an outcome. Synchronous and thread-free.
    pub(crate) struct RecordingLauncher {
        requests: std::cell::RefCell<Vec<crate::launch::Request>>,
    }

    impl RecordingLauncher {
        pub(crate) fn new() -> Self {
            Self {
                requests: std::cell::RefCell::new(Vec::new()),
            }
        }

        /// Every `Request` passed to `request`, in call order.
        pub(crate) fn requests(&self) -> Vec<crate::launch::Request> {
            self.requests.borrow().clone()
        }
    }

    impl crate::launch::Launcher for RecordingLauncher {
        fn request(&mut self, request: crate::launch::Request) {
            self.requests.borrow_mut().push(request);
        }

        fn drain(&mut self) -> Option<crate::launch::Outcome> {
            None
        }
    }

    /// A scripted `launch::Launcher` double, on `ScriptedAgents`'s terms: a queue of `drain`
    /// answers; `request` discards, since these tests script an outcome without needing the
    /// request that would have produced it — `RecordingLauncher` is the double for that.
    /// Exhausting the queue yields `None` forever rather than panicking.
    pub(crate) struct ScriptedLauncher {
        drain_queue: std::cell::RefCell<std::collections::VecDeque<Option<crate::launch::Outcome>>>,
    }

    impl ScriptedLauncher {
        pub(crate) fn new(drains: Vec<Option<crate::launch::Outcome>>) -> Self {
            Self {
                drain_queue: std::cell::RefCell::new(drains.into()),
            }
        }
    }

    impl crate::launch::Launcher for ScriptedLauncher {
        fn request(&mut self, _request: crate::launch::Request) {}

        fn drain(&mut self) -> Option<crate::launch::Outcome> {
            self.drain_queue.borrow_mut().pop_front().unwrap_or(None)
        }
    }

    /// An `EventSource` whose wait is a predicate poll, not a fixed sleep: it
    /// calls `std::thread::yield_now()` — which has no duration, so it is
    /// deliberately outside `NOSLEEP`'s pattern — and returns `Ok(None)` until
    /// a caller-supplied predicate has held for a 50ms settle window, or until
    /// a 5s deadline passes, and then presses `q` exactly once. The deadline
    /// is the backstop that turns a wiring regression into a red assertion
    /// rather than a hung suite.
    ///
    /// Its clock lives here, in `src/lib.rs`, and nowhere under `src/ui/` —
    /// `NOBLOCK` leg 2 forbids a clock there, tests included.
    pub(crate) struct UntilReady<'a> {
        predicate: &'a dyn Fn() -> bool,
        deadline: std::time::Instant,
        settle_since: Option<std::time::Instant>,
        pressed: bool,
        timeouts: std::cell::RefCell<Vec<std::time::Duration>>,
    }

    impl<'a> UntilReady<'a> {
        const SETTLE: std::time::Duration = std::time::Duration::from_millis(50);
        const DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);

        pub(crate) fn new(predicate: &'a dyn Fn() -> bool) -> Self {
            Self {
                predicate,
                deadline: std::time::Instant::now() + Self::DEADLINE,
                settle_since: None,
                pressed: false,
                timeouts: std::cell::RefCell::new(Vec::new()),
            }
        }

        /// Every `timeout` passed to `next_event`, in call order — `Script`'s
        /// own recorder, on the same terms.
        pub(crate) fn timeouts(&self) -> Vec<std::time::Duration> {
            self.timeouts.borrow().clone()
        }
    }

    impl crate::ui::event::EventSource for UntilReady<'_> {
        fn next_event(
            &mut self,
            timeout: std::time::Duration,
        ) -> Result<Option<ratatui::crossterm::event::Event>, crate::ui::event::EventError>
        {
            self.timeouts.borrow_mut().push(timeout);
            if self.pressed {
                return Err(crate::ui::event::EventError(
                    "UntilReady exhausted after pressing q".to_string(),
                ));
            }
            let now = std::time::Instant::now();
            if (self.predicate)() {
                let since = *self.settle_since.get_or_insert(now);
                if now.saturating_duration_since(since) >= Self::SETTLE {
                    self.pressed = true;
                    return Ok(Some(press(
                        ratatui::crossterm::event::KeyCode::Char('q'),
                        ratatui::crossterm::event::KeyModifiers::NONE,
                    )));
                }
            } else {
                self.settle_since = None;
            }
            if now >= self.deadline {
                self.pressed = true;
                return Ok(Some(press(
                    ratatui::crossterm::event::KeyCode::Char('q'),
                    ratatui::crossterm::event::KeyModifiers::NONE,
                )));
            }
            std::thread::yield_now();
            Ok(None)
        }
    }

    /// A staged extension of [`UntilReady`], for `agent-launch`'s outer-loop acceptance test:
    /// it yields timeouts until each stage's predicate has held for a settle window, presses
    /// that stage's key, and moves to the next stage — finishing with `q` once the last stage's
    /// key has been pressed. One overall deadline, shared across every stage, is the backstop
    /// that turns a missing link into a red assertion rather than a hung suite: a launch that
    /// never happens leaves a later stage's predicate permanently false, and the deadline
    /// presses `q` immediately regardless of which stage is current, so the run still returns
    /// `Ok(dashboard)` for the assertions to inspect. Its clock lives here, in `src/lib.rs`, and
    /// nowhere under `src/ui/` — `NOBLOCK` leg 2 forbids a clock there, tests included.
    pub(crate) struct Stages<'a> {
        stages: Vec<(&'a dyn Fn() -> bool, ratatui::crossterm::event::Event)>,
        index: usize,
        settle_since: Option<std::time::Instant>,
        deadline: std::time::Instant,
        finished: bool,
    }

    impl<'a> Stages<'a> {
        const SETTLE: std::time::Duration = std::time::Duration::from_millis(50);
        const DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);

        /// `stages` is a non-empty ordered list of `(predicate, key to press)` pairs. The last
        /// pair's key is expected to end the run (`q`, on every scenario this change writes).
        pub(crate) fn new(
            stages: Vec<(&'a dyn Fn() -> bool, ratatui::crossterm::event::Event)>,
        ) -> Self {
            assert!(!stages.is_empty(), "Stages needs at least one stage");
            Self {
                stages,
                index: 0,
                settle_since: None,
                deadline: std::time::Instant::now() + Self::DEADLINE,
                finished: false,
            }
        }
    }

    impl crate::ui::event::EventSource for Stages<'_> {
        fn next_event(
            &mut self,
            _timeout: std::time::Duration,
        ) -> Result<Option<ratatui::crossterm::event::Event>, crate::ui::event::EventError>
        {
            if self.finished {
                return Err(crate::ui::event::EventError(
                    "Stages exhausted after its last press".to_string(),
                ));
            }
            let now = std::time::Instant::now();
            if self.index < self.stages.len() {
                let ready = (self.stages[self.index].0)();
                if ready {
                    let since = *self.settle_since.get_or_insert(now);
                    if now.saturating_duration_since(since) >= Self::SETTLE {
                        let event = self.stages[self.index].1.clone();
                        self.index += 1;
                        self.settle_since = None;
                        if self.index == self.stages.len() {
                            self.finished = true;
                        }
                        return Ok(Some(event));
                    }
                } else {
                    self.settle_since = None;
                }
            }
            if now >= self.deadline {
                self.finished = true;
                return Ok(Some(press(
                    ratatui::crossterm::event::KeyCode::Char('q'),
                    ratatui::crossterm::event::KeyModifiers::NONE,
                )));
            }
            std::thread::yield_now();
            Ok(None)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{
            RecordingReader, RecordingRefresher, ScratchDir, ScriptedFs, cell, render_at, row_text,
            snapshot,
        };
        use crate::changes::empty_set;
        use crate::refresh::Refresher;
        use crate::ui::app::{Dashboard, Route};
        use crate::watch::FsEvents;

        /// `ScriptedFs` is synchronous and thread-free: it answers from a
        /// queue, and an exhausted queue yields `Ok(None)`/`None` forever
        /// rather than panicking. This module's own tests are the ones that
        /// exercise it directly; group 2's acceptance test and group 9's
        /// live-tier tests use it as a collaborator rather than testing it
        /// for its own sake.
        #[test]
        fn scripted_fs_answers_its_queue_then_falls_back_to_the_inert_default() {
            let mut fs = ScriptedFs::new(
                vec![Ok(Some(vec![std::path::PathBuf::from("/r/openspec/x")]))],
                vec![Some(std::time::Duration::from_millis(90))],
            );
            assert_eq!(
                fs.drain(),
                Ok(Some(vec![std::path::PathBuf::from("/r/openspec/x")]))
            );
            assert_eq!(fs.pending_in(), Some(std::time::Duration::from_millis(90)));
            // The queue is now exhausted: both fall back to the inert
            // default rather than panicking.
            assert_eq!(fs.drain(), Ok(None));
            assert_eq!(fs.pending_in(), None);
            assert_eq!(fs.drains().len(), 2);
            assert_eq!(fs.pendings().len(), 2);
        }

        #[test]
        fn recording_refresher_records_requests_and_takes() {
            let mut refresher = RecordingRefresher::new(vec![None]);
            refresher.request(
                crate::changes::Selection::All,
                crate::changes::ArchivedScope::Names,
            );
            assert_eq!(refresher.take_result(), None);
            // The queue is now exhausted: falls back to `None` rather than
            // panicking.
            assert_eq!(refresher.take_result(), None);
            assert_eq!(refresher.requests(), vec![crate::changes::Selection::All]);
            assert_eq!(refresher.takes(), 2);
        }

        #[test]
        fn recording_reader_returns_the_scripted_result_and_records_the_call() {
            let path = std::path::PathBuf::from("/repo/p.md");
            let recorder = RecordingReader::new(
                vec![(path.clone(), Ok("# proposal\n".to_string()))],
                Err("no such path".to_string()),
            );
            assert_eq!(recorder.read(&path), Ok("# proposal\n".to_string()));
            assert_eq!(recorder.calls(), 1);
            assert_eq!(recorder.paths(), vec![path]);
        }

        #[test]
        fn recording_reader_falls_back_to_the_default_and_records_every_call_in_order() {
            let recorder = RecordingReader::always(Ok("# doc\n".to_string()));
            let a = std::path::PathBuf::from("/repo/a.md");
            let b = std::path::PathBuf::from("/repo/b.md");
            assert_eq!(recorder.read(&a), Ok("# doc\n".to_string()));
            assert_eq!(recorder.read(&b), Ok("# doc\n".to_string()));
            assert_eq!(recorder.calls(), 2);
            assert_eq!(recorder.paths(), vec![a, b]);
        }

        #[test]
        fn recording_reader_can_script_an_error() {
            let recorder = RecordingReader::always(Err("boom".to_string()));
            let path = std::path::PathBuf::from("/repo/missing.md");
            assert_eq!(recorder.read(&path), Err("boom".to_string()));
        }

        fn empty_dashboard() -> Dashboard {
            Dashboard {
                repo: None,
                searched_from: std::path::PathBuf::from("/tmp/does-not-matter"),
                changes: empty_set(),
                route: Route::List,
                quit: false,
                selected: 0,
                filter: crate::ui::app::Filter {
                    query: String::new(),
                    active: false,
                },
                detail: crate::ui::app::Detail {
                    source: String::new(),
                    scroll: 0,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
                },
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: crate::ui::app::Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
            }
        }

        #[test]
        fn render_at_touches_no_directory() {
            let scratch = ScratchDir::new();
            let before = snapshot(scratch.path());
            let dashboard = empty_dashboard();

            let buf60 = render_at(60, 20, &dashboard);
            let buf120 = render_at(120, 20, &dashboard);

            let after = snapshot(scratch.path());
            assert_eq!(
                before, after,
                "render_at wrote inside the owned scratch dir"
            );

            for buf in [&buf60, &buf120] {
                assert_eq!(&row_text(buf, 0)[0..8], "OpenSpec");
                assert!(row_text(buf, buf.area.height - 1).starts_with("q quit"));
            }
        }

        #[test]
        fn row_text_reads_a_whole_row() {
            let dashboard = empty_dashboard();
            for width in [60u16, 120u16] {
                let buf = render_at(width, 20, &dashboard);
                assert_eq!(row_text(&buf, 0).chars().count(), width as usize);
            }
        }

        #[test]
        fn cell_reads_symbol_and_style() {
            let dashboard = empty_dashboard();
            for width in [60u16, 120u16] {
                let buf = render_at(width, 20, &dashboard);
                let top_left = cell(&buf, 0, 0);
                assert_eq!(top_left.symbol(), "O");
                assert!(
                    top_left
                        .style()
                        .add_modifier
                        .contains(ratatui::style::Modifier::BOLD)
                );
            }
        }

        /// `ScriptedAgents` is synchronous and thread-free, on `ScriptedFs`'s
        /// terms: it answers from a queue, and an exhausted queue yields
        /// `None` forever rather than panicking.
        #[test]
        fn scripted_agents_answers_its_queue_then_falls_back_to_none() {
            use crate::agents::{AgentPoll, AgentSnapshot};

            let mut agents = super::ScriptedAgents::new(
                vec![Some(AgentSnapshot {
                    agents: Vec::new(),
                    reachable: true,
                    stalled: false,
                    problem: None,
                })],
                vec![Some(std::time::Duration::from_millis(40))],
            );
            assert_eq!(
                agents.drain(),
                Some(AgentSnapshot {
                    agents: Vec::new(),
                    reachable: true,
                    stalled: false,
                    problem: None,
                })
            );
            assert_eq!(
                agents.pending_in(),
                Some(std::time::Duration::from_millis(40))
            );
            // The queue is now exhausted: both fall back to the inert
            // default rather than panicking.
            assert_eq!(agents.drain(), None);
            assert_eq!(agents.pending_in(), None);
            assert_eq!(agents.drains().len(), 2);
            assert_eq!(agents.pendings().len(), 2);
        }

        /// `UntilReady` presses `q` once the predicate has held for its
        /// settle window, and its own deadline is the backstop when the
        /// predicate never becomes true — never a hang, never a panic.
        #[test]
        fn until_ready_presses_q_once_settled_and_again_never_hangs_past_its_deadline() {
            use crate::ui::event::EventSource;

            let ready = std::cell::Cell::new(false);
            let predicate = || ready.get();
            let mut source = super::UntilReady::new(&predicate);

            // Not ready yet: yields Ok(None) rather than pressing q.
            assert_eq!(
                source
                    .next_event(std::time::Duration::from_millis(1))
                    .expect("not ready yet"),
                None
            );

            ready.set(true);
            // Poll until it presses q, bounded by its own 5s deadline so a
            // regression here fails this test rather than hanging it.
            let mut pressed = false;
            for _ in 0..1_000_000 {
                match source
                    .next_event(std::time::Duration::from_millis(1))
                    .expect("predicate is true")
                {
                    Some(_) => {
                        pressed = true;
                        break;
                    }
                    None => continue,
                }
            }
            assert!(pressed, "UntilReady never pressed q once settled");
            assert!(source.timeouts().len() >= 2);

            // Never a fixed sleep followed by an assertion: a predicate that
            // never becomes true still ends, at the deadline, rather than
            // hanging the suite.
            let never = || false;
            let mut deadline_source = super::UntilReady::new(&never);
            deadline_source.deadline = std::time::Instant::now();
            let event = deadline_source
                .next_event(std::time::Duration::from_millis(1))
                .expect("deadline path does not error");
            assert!(event.is_some(), "the deadline path must still press q");
        }
    }
}

/// The classified shape of an invocation of the binary.
#[derive(Debug, PartialEq, Eq)]
pub enum Invocation {
    /// `ui` alone: run the dashboard.
    Ui,
    /// `open` alone: open or focus the dashboard, split from the invoking pane.
    Open,
    /// `open-tab` alone: open or focus the dashboard, in a new tab.
    OpenTab,
    /// Anything else. Carries the offending token, when there is one — the
    /// unrecognised first argument, or the trailing argument after `ui`,
    /// `open`, or `open-tab`. `None` means the argument list was empty.
    Reject(Option<String>),
}

/// Classify a program's arguments (excluding argv[0]). No flag grammar: a subcommand
/// followed by anything else is a rejection carrying that token, exactly as `ui --tab`
/// already was — see `specs/pane-open/spec.md` -> "`open` and `open-tab` are subcommands
/// of their own, with no flag grammar" and design.md -> Decision 1.
pub fn parse(args: &[&str]) -> Invocation {
    match args {
        [] => Invocation::Reject(None),
        [first] if *first == "ui" => Invocation::Ui,
        [first] if *first == "open" => Invocation::Open,
        [first] if *first == "open-tab" => Invocation::OpenTab,
        ["ui", rest, ..] => Invocation::Reject(Some((*rest).to_string())),
        ["open", rest, ..] => Invocation::Reject(Some((*rest).to_string())),
        ["open-tab", rest, ..] => Invocation::Reject(Some((*rest).to_string())),
        [first, ..] => Invocation::Reject(Some((*first).to_string())),
    }
}

/// Usage text printed to stderr for any rejected invocation.
pub fn usage() -> &'static str {
    "usage: herdr-openspec <ui|open|open-tab>\n\nCommands:\n  ui        Run the OpenSpec dashboard pane\n  open      Open or focus the dashboard pane, split from the invoking pane\n  open-tab  Open or focus the dashboard pane, in a new tab\n"
}

/// The full stderr text for a rejected invocation: an optional line naming
/// the offending token, followed by usage.
pub fn rejection_text(token: Option<&str>) -> String {
    match token {
        Some(token) => format!("error: unrecognized argument: {token}\n\n{}", usage()),
        None => usage().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_alone_classifies_as_ui() {
        assert_eq!(parse(&["ui"]), Invocation::Ui);
    }

    #[test]
    fn unrecognised_first_argument_is_a_rejection_carrying_the_token() {
        assert_eq!(parse(&["wat"]), Invocation::Reject(Some("wat".to_string())));
    }

    #[test]
    fn no_arguments_is_a_rejection_with_no_token() {
        assert_eq!(parse(&[]), Invocation::Reject(None));
    }

    #[test]
    fn ui_followed_by_an_argument_is_a_rejection_carrying_that_argument() {
        assert_eq!(
            parse(&["ui", "--tab"]),
            Invocation::Reject(Some("--tab".to_string()))
        );
    }

    #[test]
    fn open_alone_classifies_as_open() {
        assert_eq!(parse(&["open"]), Invocation::Open);
    }

    #[test]
    fn open_tab_alone_classifies_as_open_tab() {
        assert_eq!(parse(&["open-tab"]), Invocation::OpenTab);
    }

    #[test]
    fn a_flag_after_a_subcommand_is_rejected() {
        assert_eq!(
            parse(&["open", "--tab"]),
            Invocation::Reject(Some("--tab".to_string()))
        );
        assert_eq!(
            parse(&["open-tab", "x"]),
            Invocation::Reject(Some("x".to_string()))
        );
        assert_eq!(
            parse(&["ui", "--tab"]),
            Invocation::Reject(Some("--tab".to_string()))
        );
    }

    /// Split `usage()`'s `Commands:` block into lines and assert exactly three, whose
    /// first whitespace-delimited tokens are `ui`, `open`, and `open-tab` — asserted as
    /// whole tokens, since `open` is a substring of `open-tab` and `.contains("open")`
    /// would be satisfied by either alone.
    #[test]
    fn usage_lists_ui() {
        let text = usage();
        assert!(text.starts_with("usage: herdr-openspec <ui|open|open-tab>"));
        let commands_block = text
            .split("Commands:\n")
            .nth(1)
            .expect("usage() has a Commands: block");
        let lines: Vec<&str> = commands_block
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect();
        assert_eq!(lines.len(), 3, "commands block: {commands_block:?}");
        let first_tokens: Vec<&str> = lines
            .iter()
            .map(|l| l.split_whitespace().next().expect("non-empty line"))
            .collect();
        assert_eq!(first_tokens, vec!["ui", "open", "open-tab"]);
    }
}
