//! The worker thread: the only module in the crate spawning a thread. See
//! `specs/refresh-worker/spec.md` and `openspec/changes/live-refresh/design.md`
//! -> Boundaries. Reaches the `openspec` program only through
//! `Arc<dyn crate::cli::OpenspecCli>` — it spawns no process, and
//! `src/cli.rs` stays the crate's one spawn site.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;

use crate::changes::{ChangeSet, Selection};
use crate::cli::OpenspecCli;

/// One request's two answers, in the order the worker sends them: the
/// file-sourced set first, sub-millisecond; the CLI-merged one 200-400ms
/// later. Neither variant names `CliChanges`, `OpenspecCli`, or `from_cli`,
/// so `src/ui/driver.rs` can hold a `&mut dyn Refresher` while `NOCLI-SHELL`
/// stays green unweakened.
///
/// `seam-resilience`'s addition: `Stopped(reason)`, synthesised by the
/// `Refresher` itself — never sent by the worker — once its result channel
/// disconnects. It carries no `ChangeSet`: the change set already on screen
/// is the last true one, and replacing it with an empty one on worker death
/// would make a degraded pane look like an empty repository. See
/// `specs/refresh-worker/spec.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshResult {
    Files(ChangeSet),
    Merged(ChangeSet),
    Stopped(String),
}

/// The reason a latched dead worker's one `Stopped` result carries.
const WORKER_STOPPED_REASON: &str = "the refresh worker has stopped answering";

/// The non-blocking seam between the render path and the worker thread.
/// Neither method may block, sleep, join a thread, or wait on a channel. See
/// `specs/refresh-worker/spec.md`.
pub trait Refresher {
    /// Ask for a refresh over `selection`. Records the request; does not
    /// wait for an answer.
    fn request(&mut self, selection: Selection);
    /// The next result the worker produced, if one is ready. Never blocks
    /// and never waits on the channel — implemented with a non-blocking
    /// poll, not a loop over the receiver.
    fn take_result(&mut self) -> Option<RefreshResult>;
}

/// The inert implementation: `request` records nothing and does nothing,
/// `take_result` is always `None`. No thread is spawned and no process is
/// started.
struct NoRefresher;

impl Refresher for NoRefresher {
    fn request(&mut self, _selection: Selection) {}

    fn take_result(&mut self) -> Option<RefreshResult> {
        None
    }
}

/// The inert `Refresher`. See [`NoRefresher`].
pub fn none() -> Box<dyn Refresher> {
    Box::new(NoRefresher)
}

/// The real `Refresher`: a request channel out, and `take_result` reading
/// the worker's own result channel with `try_recv`, never a blocking
/// receive. Dropping it drops the request `Sender`, which is what makes
/// the worker return on its next `recv`.
///
/// `seam-resilience`'s addition, on `agents::RealAgentPoll`'s and
/// `launch::RealLauncher`'s model: `dead` and `pending_death` latch a
/// worker's death so it is reported exactly once and then degrades to
/// silence; `outstanding` and `pending_all` implement "at most one refresh
/// cycle outstanding", with a `Selection::All` arriving behind a narrower
/// one remembered rather than discarded. See `specs/refresh-worker/spec.md`.
struct RealRefresher {
    request_tx: mpsc::Sender<Selection>,
    result_rx: mpsc::Receiver<RefreshResult>,
    /// Set once the worker's result channel disconnects, so the standing
    /// "worker stopped" result is reported exactly once and every `request`
    /// after that is discarded.
    dead: bool,
    /// Set the instant a `SendError` is first observed from `request`, and
    /// consumed by the very next `take_result` — which reports it once and
    /// then sets `dead`. Kept separate from `dead` because `request` can
    /// detect the death before any `take_result` runs, and the one report
    /// must still happen on a `take_result` call.
    pending_death: bool,
    /// Whether a request sent to the worker has not yet been answered by a
    /// `Merged` result (or a latched `Stopped`). While set, `request`
    /// discards further narrower selections and only remembers a
    /// `Selection::All` in `pending_all`.
    outstanding: bool,
    /// A `Selection::All` that arrived while a narrower selection was
    /// outstanding, sent as its own cycle once the outstanding one answers.
    pending_all: bool,
}

impl Refresher for RealRefresher {
    fn request(&mut self, selection: Selection) {
        if self.dead {
            return;
        }
        if self.outstanding {
            if matches!(selection, Selection::All) {
                self.pending_all = true;
            }
            return;
        }
        if self.request_tx.send(selection).is_err() {
            self.pending_death = true;
            return;
        }
        self.outstanding = true;
    }

    fn take_result(&mut self) -> Option<RefreshResult> {
        if self.dead {
            return None;
        }
        if self.pending_death {
            self.dead = true;
            self.pending_death = false;
            return Some(RefreshResult::Stopped(WORKER_STOPPED_REASON.to_string()));
        }
        match self.result_rx.try_recv() {
            Ok(RefreshResult::Merged(set)) => {
                self.outstanding = false;
                if self.pending_all {
                    self.pending_all = false;
                    if self.request_tx.send(Selection::All).is_err() {
                        self.pending_death = true;
                    } else {
                        self.outstanding = true;
                    }
                }
                Some(RefreshResult::Merged(set))
            }
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.dead = true;
                Some(RefreshResult::Stopped(WORKER_STOPPED_REASON.to_string()))
            }
        }
    }
}

/// Start the worker: the inert `Refresher` when either `repo` or `cli` is
/// `None` — the no-binary and no-repository cases cost nothing, no thread
/// and no process — otherwise one `thread::spawn` running [`worker_body`].
pub fn start(
    repo: Option<&std::path::Path>,
    cli: Option<Arc<dyn OpenspecCli>>,
    archived_count: usize,
) -> Box<dyn Refresher> {
    let (Some(repo), Some(cli)) = (repo, cli) else {
        return none();
    };
    let repo = repo.to_path_buf();
    let (request_tx, request_rx) = mpsc::channel();
    let (result_tx, result_rx) = mpsc::channel();
    std::thread::spawn(move || worker_body(repo, cli, archived_count, request_rx, result_tx));
    Box::new(RealRefresher {
        request_tx,
        result_rx,
        dead: false,
        pending_death: false,
        outstanding: false,
        pending_all: false,
    })
}

// Everything below this point is the worker's own body — reached only from
// inside the `thread::spawn` closure above, never from the render path, and
// therefore free to block. `NOBLOCK` leg 3 relies on this ordering: it cuts
// `src/refresh.rs`'s production slice at its single `thread::spawn` and
// only searches the half before it.

/// Fold `first` with every further `Selection` already queued on `rx`,
/// non-blocking: drains with `try_recv` until the channel is empty (or
/// disconnected) and unions each into the accumulator. Named and exposed
/// so the folding rule is provable single-threaded, by handing it a
/// receiver whose sender has already queued values and been dropped — a
/// rule proved only through a live worker is a rule proved by whichever
/// interleaving happened to occur.
fn drain_and_fold(first: Selection, rx: &mpsc::Receiver<Selection>) -> Selection {
    let mut acc = first;
    while let Ok(next) = rx.try_recv() {
        acc = acc.union(next);
    }
    acc
}

/// The worker's whole body: fold any queued requests into one selection,
/// send the file-sourced result, then the CLI-merged one, and repeat until
/// either channel disconnects. Owns one `CliCache` for its whole lifetime.
fn worker_body(
    repo: PathBuf,
    cli: Arc<dyn OpenspecCli>,
    archived_count: usize,
    request_rx: mpsc::Receiver<Selection>,
    result_tx: mpsc::Sender<RefreshResult>,
) {
    let mut cache = crate::changes::CliCache::default();
    loop {
        let Ok(first) = request_rx.recv() else {
            return; // the Refresher was dropped
        };
        let selection = drain_and_fold(first, &request_rx);

        let files = crate::changes::from_files(&repo, archived_count);
        if result_tx.send(RefreshResult::Files(files.clone())).is_err() {
            return; // nobody reads the result any more
        }

        let cli_changes =
            crate::changes::from_cli_cached(cli.as_ref(), &repo, &selection, &mut cache);
        let merged = crate::changes::merge(files, cli_changes);
        if result_tx.send(RefreshResult::Merged(merged)).is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vendor_tdd_schema(repo: &std::path::Path) {
        let yaml = "\
name: tdd
artifacts:
  - id: proposal
    generates: proposal.md
  - id: specs
    generates: specs/**/*.md
  - id: design
    generates: design.md
  - id: tasks
    generates: tasks.md
  - id: planning-review
    generates: planning-review.md
apply:
  tracks: tasks.md
";
        crate::testutil::write_with_mode(
            &repo.join("openspec/schemas/tdd/schema.yaml"),
            yaml.as_bytes(),
            0o644,
        );
        crate::testutil::write_with_mode(
            &repo.join("openspec/config.yaml"),
            b"schema: tdd\n",
            0o644,
        );
    }

    /// A `Refresher` whose `take_result` always yields `None`: the test
    /// reads results directly off the second element `worker_for_test`
    /// hands back instead. `request` forwards to the worker unchanged.
    struct TestRefresher {
        request_tx: mpsc::Sender<Selection>,
    }

    impl Refresher for TestRefresher {
        fn request(&mut self, selection: Selection) {
            let _ = self.request_tx.send(selection);
        }

        fn take_result(&mut self) -> Option<RefreshResult> {
            None
        }
    }

    /// A worker seam for tests: the second element is the worker's result
    /// `Receiver`, handed to the test directly rather than stored on the
    /// `Refresher` (whose own `take_result` therefore always yields `None`
    /// in these tests); the third receives from a channel whose `Sender`
    /// the worker thread owns and drops only when its body returns —
    /// dropping the returned `Refresher` drops the request `Sender` alone,
    /// which is what makes that disconnection observable.
    pub(crate) fn worker_for_test(
        repo: PathBuf,
        cli: Arc<dyn OpenspecCli>,
        archived_count: usize,
    ) -> (
        Box<dyn Refresher>,
        mpsc::Receiver<RefreshResult>,
        mpsc::Receiver<()>,
    ) {
        let (request_tx, request_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        // `exit_tx` is never sent on: its `Sender` is simply owned by the
        // worker thread and moved into the closure, so it drops only when
        // `worker_body` returns and the closure ends — observable on
        // `exit_rx` as a channel disconnection, never as a received value.
        let (exit_tx, exit_rx) = mpsc::channel::<()>();
        std::thread::spawn(move || {
            let _exit_tx = exit_tx;
            worker_body(repo, cli, archived_count, request_rx, result_tx);
        });
        (Box::new(TestRefresher { request_tx }), result_rx, exit_rx)
    }

    #[test]
    fn no_refresher_never_yields() {
        let mut r = none();
        for _ in 0..10 {
            r.request(Selection::All);
            let result = r.take_result();
            assert_eq!(result, None);
            // `seam-resilience`: the inert refresher has no worker to lose, so it never
            // reports one stopped.
            assert!(!matches!(result, Some(RefreshResult::Stopped(_))));
        }
    }

    /// `seam-resilience`: `refresh-worker` -> "A dead refresh worker is reported once and
    /// then stops being reported". Constructs a `RealRefresher` directly — the same private
    /// struct `start` returns, reachable from this child module on exactly
    /// `agents::RealAgentPoll`'s terms — over channels whose worker-side ends are dropped, so
    /// both `result_rx` and `request_tx` are disconnected from the very first call.
    #[test]
    fn a_dead_refresh_worker_is_reported_once_and_then_stops_being_reported() {
        let (request_tx, request_rx) = mpsc::channel::<Selection>();
        let (result_tx, result_rx) = mpsc::channel::<RefreshResult>();
        drop(result_tx);
        drop(request_rx);
        let mut r = RealRefresher {
            request_tx,
            result_rx,
            dead: false,
            pending_death: false,
            outstanding: false,
            pending_all: false,
        };

        match r.take_result() {
            Some(RefreshResult::Stopped(reason)) => {
                assert!(reason.to_lowercase().contains("refresh worker"), "{reason}");
            }
            other => panic!("expected Stopped, got {other:?}"),
        }

        r.request(Selection::All);
        assert_eq!(r.take_result(), None, "reported once, then silence");
        r.request(Selection::All);
        assert_eq!(r.take_result(), None);
    }

    /// `seam-resilience`: `refresh-worker` -> "A refresh outstanding does not queue further
    /// selections". Drives `RealRefresher::request` directly against a channel whose other
    /// end nothing ever drains, so what actually reached the worker is asserted on the
    /// channel's own contents.
    #[test]
    fn a_refresh_outstanding_does_not_queue_further_selections() {
        let (request_tx, request_rx) = mpsc::channel::<Selection>();
        let (result_tx, result_rx) = mpsc::channel::<RefreshResult>();
        let mut r = RealRefresher {
            request_tx,
            result_rx,
            dead: false,
            pending_death: false,
            outstanding: false,
            pending_all: false,
        };

        r.request(Selection::Only(std::collections::BTreeSet::from([
            "alpha".to_string()
        ])));
        r.request(Selection::Only(std::collections::BTreeSet::from([
            "beta".to_string()
        ])));
        r.request(Selection::Only(std::collections::BTreeSet::from([
            "gamma".to_string()
        ])));

        let received: Vec<Selection> = request_rx.try_iter().collect();
        assert_eq!(
            received,
            vec![Selection::Only(std::collections::BTreeSet::from([
                "alpha".to_string()
            ]))],
            "exactly one selection reached the worker's request channel"
        );

        result_tx
            .send(RefreshResult::Merged(crate::changes::empty_set()))
            .expect("channel still connected");
        assert!(matches!(r.take_result(), Some(RefreshResult::Merged(_))));

        r.request(Selection::Only(std::collections::BTreeSet::from([
            "delta".to_string()
        ])));
        let received2: Vec<Selection> = request_rx.try_iter().collect();
        assert_eq!(
            received2,
            vec![Selection::Only(std::collections::BTreeSet::from([
                "delta".to_string()
            ]))],
            "the suppression is per-cycle, not permanent"
        );
    }

    /// `seam-resilience`: `refresh-worker` -> "A forced refresh outstanding behind a
    /// narrower one is not lost".
    #[test]
    fn a_forced_refresh_outstanding_behind_a_narrower_one_is_not_lost() {
        let (request_tx, request_rx) = mpsc::channel::<Selection>();
        let (result_tx, result_rx) = mpsc::channel::<RefreshResult>();
        let mut r = RealRefresher {
            request_tx,
            result_rx,
            dead: false,
            pending_death: false,
            outstanding: false,
            pending_all: false,
        };

        r.request(Selection::Only(std::collections::BTreeSet::from([
            "alpha".to_string()
        ])));
        r.request(Selection::All);

        let first_batch: Vec<Selection> = request_rx.try_iter().collect();
        assert_eq!(
            first_batch,
            vec![Selection::Only(std::collections::BTreeSet::from([
                "alpha".to_string()
            ]))]
        );

        result_tx
            .send(RefreshResult::Files(crate::changes::empty_set()))
            .expect("channel still connected");
        assert!(matches!(r.take_result(), Some(RefreshResult::Files(_))));
        let mid_batch: Vec<Selection> = request_rx.try_iter().collect();
        assert!(
            mid_batch.is_empty(),
            "a Files result does not answer the cycle"
        );

        result_tx
            .send(RefreshResult::Merged(crate::changes::empty_set()))
            .expect("channel still connected");
        assert!(matches!(r.take_result(), Some(RefreshResult::Merged(_))));

        let second_batch: Vec<Selection> = request_rx.try_iter().collect();
        assert_eq!(
            second_batch,
            vec![Selection::All],
            "the remembered All is sent once the outstanding cycle answers"
        );
    }

    #[test]
    fn start_without_a_binary_is_inert() {
        let scratch = crate::testutil::ScratchDir::new();
        let mut r = start(Some(scratch.path()), None, 5);
        r.request(Selection::All);
        assert_eq!(r.take_result(), None);
    }

    #[test]
    fn start_without_a_repo_is_inert() {
        let fake: Arc<dyn OpenspecCli> = Arc::new(crate::cli::FakeCli::new());
        let mut r = start(None, Some(fake), 5);
        r.request(Selection::All);
        assert_eq!(r.take_result(), None);
    }

    #[test]
    fn the_worker_answers_with_files_then_merged() {
        let scratch = crate::testutil::ScratchDir::new();
        let root = crate::testutil::canonical(scratch.path());
        vendor_tdd_schema(&root);
        crate::testutil::write_with_mode(
            &root.join("openspec/changes/alpha/tasks.md"),
            b"- [x] a\n- [x] b\n- [x] c\n- [x] d\n- [ ] e\n- [ ] f\n- [ ] g\n- [ ] h\n- [ ] i\n",
            0o644,
        );
        let fake = crate::cli::FakeCli::new();
        fake.register_openspec(
            &["list", "--json"],
            Ok(format!(
                r#"{{"changes":[{{"name":"alpha","completedTasks":7,"totalTasks":9,"lastModified":"x","status":"y"}}],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                root.display().to_string()
            )),
        );
        fake.register_openspec(
            &["instructions", "apply", "--change", "alpha", "--json"],
            Ok(format!(
                r#"{{"schemaName":"tdd","changeDir":{:?},"contextFiles":{{}}}}"#,
                root.join("openspec/changes/alpha").display().to_string()
            )),
        );
        let cli: Arc<dyn OpenspecCli> = Arc::new(fake);

        let (mut refresher, results_rx, _exit_rx) = worker_for_test(root, cli, 5);
        refresher.request(Selection::All);

        let files = results_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("the worker did not send the file result within 10s");
        match files {
            RefreshResult::Files(set) => {
                let alpha = set.active.iter().find(|c| c.name == "alpha").unwrap();
                assert_eq!(alpha.progress.completed, 4);
                assert_eq!(alpha.progress.total, 9);
            }
            other => panic!("expected Files, got {other:?}"),
        }

        let merged = results_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("the worker did not send the merged result within 10s");
        match merged {
            RefreshResult::Merged(set) => {
                let alpha = set.active.iter().find(|c| c.name == "alpha").unwrap();
                assert_eq!(alpha.progress.completed, 7);
                assert_eq!(alpha.progress.total, 9);
                assert_eq!(alpha.schema, "tdd");
            }
            other => panic!("expected Merged, got {other:?}"),
        }
    }

    #[test]
    fn a_failing_cli_still_sends_the_files_result() {
        let scratch = crate::testutil::ScratchDir::new();
        let root = crate::testutil::canonical(scratch.path());
        vendor_tdd_schema(&root);
        crate::testutil::write_with_mode(
            &root.join("openspec/changes/alpha/tasks.md"),
            b"- [x] a\n- [ ] b\n",
            0o644,
        );
        let fake = crate::cli::FakeCli::new();
        fake.register_openspec(
            &["list", "--json"],
            Err(crate::cli::CliError::NotStarted {
                program: "openspec".to_string(),
                args: vec!["list".to_string(), "--json".to_string()],
                reason: "not found".to_string(),
            }),
        );
        let cli: Arc<dyn OpenspecCli> = Arc::new(fake);

        let (mut refresher, results_rx, _exit_rx) = worker_for_test(root, cli, 5);
        refresher.request(Selection::All);

        let files = results_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("the worker did not send the file result within 10s");
        match files {
            RefreshResult::Files(set) => {
                assert_eq!(set.active.len(), 1);
                assert_eq!(set.active[0].name, "alpha");
            }
            other => panic!("expected Files, got {other:?}"),
        }

        let merged = results_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("the worker did not send the merged result within 10s");
        match merged {
            RefreshResult::Merged(set) => {
                assert!(!set.problems.is_empty());
                assert_eq!(set.active.len(), 1);
            }
            other => panic!("expected Merged, got {other:?}"),
        }
    }

    /// `refresh-worker` :: "A shim whose interpreter is unreachable exits 127 and renders a
    /// problem row" — S10's shape (design.md -> Decision 1b): the child never execs, so
    /// `openspec list --json` fails with a non-zero exit rather than answering. Nothing in
    /// this crate's error mapping needs to change for this: `cli_error_problem` already
    /// turns any `CliError::Failed` into a problem row naming the command and its exit
    /// code, and the worker already keeps the file-sourced set on any CLI failure — this
    /// pins that a 127 specifically degrades the pane rather than emptying it, panicking,
    /// or surfacing a `LoopError`.
    #[test]
    fn a_shim_whose_interpreter_is_unreachable_exits_127_and_renders_a_problem_row() {
        let scratch = crate::testutil::ScratchDir::new();
        let root = crate::testutil::canonical(scratch.path());
        vendor_tdd_schema(&root);
        crate::testutil::write_with_mode(
            &root.join("openspec/changes/alpha/tasks.md"),
            b"- [x] a\n- [ ] b\n",
            0o644,
        );
        let fake = crate::cli::FakeCli::new();
        fake.register_openspec(
            &["list", "--json"],
            Err(crate::cli::CliError::Failed {
                program: "openspec".to_string(),
                args: vec!["list".to_string(), "--json".to_string()],
                code: Some(127),
                stderr: String::new(),
            }),
        );
        let cli: Arc<dyn OpenspecCli> = Arc::new(fake);

        let (mut refresher, results_rx, _exit_rx) = worker_for_test(root, cli, 5);
        refresher.request(Selection::All);

        let files = results_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("the worker did not send the file result within 10s");
        match files {
            RefreshResult::Files(set) => {
                assert_eq!(set.active.len(), 1);
                assert_eq!(set.active[0].name, "alpha");
            }
            other => panic!("expected Files, got {other:?}"),
        }

        let merged = results_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("the worker did not send the merged result within 10s");
        match merged {
            RefreshResult::Merged(set) => {
                assert_eq!(
                    set.active.len(),
                    1,
                    "the file-sourced change must survive a 127, not be emptied"
                );
                assert_eq!(set.active[0].name, "alpha");
                assert_eq!(set.problems.len(), 1);
                assert!(
                    set.problems[0].contains("list --json"),
                    "{:?}",
                    set.problems
                );
                assert!(set.problems[0].contains("127"), "{:?}", set.problems);
            }
            other => panic!("expected Merged, got {other:?}"),
        }
    }

    #[test]
    fn drain_and_fold_unions_queued_requests() {
        let (tx, rx) = mpsc::channel::<Selection>();
        tx.send(Selection::Only(std::collections::BTreeSet::from([
            "b".to_string()
        ])))
        .unwrap();
        tx.send(Selection::Only(std::collections::BTreeSet::from([
            "c".to_string()
        ])))
        .unwrap();
        drop(tx);

        let result = drain_and_fold(
            Selection::Only(std::collections::BTreeSet::from(["a".to_string()])),
            &rx,
        );
        assert_eq!(
            result,
            Selection::Only(std::collections::BTreeSet::from([
                "a".to_string(),
                "b".to_string(),
                "c".to_string()
            ]))
        );

        let (tx2, rx2) = mpsc::channel::<Selection>();
        tx2.send(Selection::All).unwrap();
        drop(tx2);
        assert_eq!(
            drain_and_fold(Selection::Only(std::collections::BTreeSet::new()), &rx2),
            Selection::All
        );

        let (tx3, rx3) = mpsc::channel::<Selection>();
        drop(tx3);
        assert_eq!(
            drain_and_fold(Selection::Only(std::collections::BTreeSet::new()), &rx3),
            Selection::Only(std::collections::BTreeSet::new())
        );
    }

    #[test]
    fn dropping_the_refresher_disconnects_the_channel() {
        let scratch = crate::testutil::ScratchDir::new();
        let root = crate::testutil::canonical(scratch.path());
        let fake: Arc<dyn OpenspecCli> = Arc::new(crate::cli::FakeCli::new());

        let (refresher, _results_rx, exit_rx) = worker_for_test(root, fake, 5);
        drop(refresher);

        match exit_rx.recv_timeout(std::time::Duration::from_secs(10)) {
            Err(mpsc::RecvTimeoutError::Disconnected) => {}
            other => panic!(
                "the worker did not return within 10s after its Refresher was dropped: {other:?}"
            ),
        }
    }

    #[test]
    fn the_worker_writes_nothing() {
        let scratch = crate::testutil::ScratchDir::new();
        let root = crate::testutil::canonical(scratch.path());
        vendor_tdd_schema(&root);
        crate::testutil::write_with_mode(
            &root.join("openspec/changes/alpha/tasks.md"),
            b"- [x] a\n- [ ] b\n",
            0o644,
        );
        let fake = crate::cli::FakeCli::new();
        fake.register_openspec(
            &["list", "--json"],
            Ok(format!(
                r#"{{"changes":[],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                root.display().to_string()
            )),
        );
        let cli: Arc<dyn OpenspecCli> = Arc::new(fake);

        let before = crate::testutil::snapshot(&root);
        let (mut refresher, results_rx, exit_rx) = worker_for_test(root.clone(), cli, 5);
        refresher.request(Selection::All);
        results_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("files result");
        results_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("merged result");
        drop(refresher);
        match exit_rx.recv_timeout(std::time::Duration::from_secs(10)) {
            Err(mpsc::RecvTimeoutError::Disconnected) => {}
            other => panic!("expected the worker to return within 10s: {other:?}"),
        }
        let after = crate::testutil::snapshot(&root);
        assert_eq!(before, after, "the worker wrote inside the repository");
    }
}
