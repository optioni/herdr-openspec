//! The worker thread: the only module in the crate spawning a thread. See
//! `specs/refresh-worker/spec.md` and `openspec/changes/live-refresh/design.md`
//! -> Boundaries. Reaches the `openspec` program only through
//! `Arc<dyn crate::cli::OpenspecCli>` — it spawns no process, and
//! `src/cli.rs` stays the crate's one spawn site.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use crate::changes::{ArchivedScope, ChangeSet, Selection};
use crate::cli::{CliError, GitCli, OpenspecCli};

/// One refresh cycle's two inputs, folded together: `list-sections`'
/// addition, replacing the bare `Selection` the channel used to carry. The
/// two fields fold by different rules — `selection` unions, `archived`
/// takes the last value — so carrying them as one named value gives
/// `drain_and_fold` one thing to fold rather than two parallel channels
/// that could drift out of step. See
/// `openspec/changes/list-sections/design.md` -> Decision 9.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub selection: Selection,
    pub archived: ArchivedScope,
}

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

/// `worktree-overlay`'s idle re-check cadence (design.md -> Decision 6): how long the
/// worker waits with no request arriving before it re-derives the worktree family and
/// re-reads every owned change from its member's files. Never written as a bare literal
/// at the worker's own timed-wait call site; a test drives `worker_for_test`'s own
/// `recheck` parameter with a few milliseconds instead, so no scenario waits two
/// seconds.
const WORKTREE_RECHECK: Duration = Duration::from_secs(2);

/// The non-blocking seam between the render path and the worker thread.
/// Neither method may block, sleep, join a thread, or wait on a channel. See
/// `specs/refresh-worker/spec.md`.
pub trait Refresher {
    /// Ask for a refresh over `selection`, resolving the archived tier
    /// under `archived`. Records the request; does not wait for an answer.
    /// `archived` is `list-sections`' addition: the archived section can be
    /// folded and unfolded at any moment, so the scope is a property of the
    /// cycle rather than of the worker.
    fn request(&mut self, selection: Selection, archived: ArchivedScope);
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
    fn request(&mut self, _selection: Selection, _archived: ArchivedScope) {}

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
    request_tx: mpsc::Sender<Request>,
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
    /// `list-sections`' addition: carries the `ArchivedScope` of the
    /// **most recent** suppressed request, not the scope of the request
    /// that first set it, so the archive the reader has since folded or
    /// opened is the one the next cycle resolves (design.md -> Decision 9).
    pending_all: Option<ArchivedScope>,
}

impl Refresher for RealRefresher {
    fn request(&mut self, selection: Selection, archived: ArchivedScope) {
        if self.dead {
            return;
        }
        if self.outstanding {
            if matches!(selection, Selection::All) {
                self.pending_all = Some(archived);
            }
            return;
        }
        if self
            .request_tx
            .send(Request {
                selection,
                archived,
            })
            .is_err()
        {
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
                if let Some(archived) = self.pending_all.take() {
                    if self
                        .request_tx
                        .send(Request {
                            selection: Selection::All,
                            archived,
                        })
                        .is_err()
                    {
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
/// `git` is never optional (`worktree-overlay` design.md -> Decision 14): an
/// absent `git` binary is a degraded state the worker itself absorbs, not a
/// second way to say "no worker".
pub fn start(
    repo: Option<&std::path::Path>,
    cli: Option<Arc<dyn OpenspecCli>>,
    git: Arc<dyn GitCli>,
) -> Box<dyn Refresher> {
    let (Some(repo), Some(cli)) = (repo, cli) else {
        return none();
    };
    let repo = repo.to_path_buf();
    let (request_tx, request_rx) = mpsc::channel();
    let (result_tx, result_rx) = mpsc::channel();
    std::thread::spawn(move || {
        worker_body(repo, cli, git, WORKTREE_RECHECK, request_rx, result_tx)
    });
    Box::new(RealRefresher {
        request_tx,
        result_rx,
        dead: false,
        pending_death: false,
        outstanding: false,
        pending_all: None,
    })
}

// Everything below this point is the worker's own body — reached only from
// inside the `thread::spawn` closure above, never from the render path, and
// therefore free to block. `NOBLOCK` leg 3 relies on this ordering: it cuts
// `src/refresh.rs`'s production slice at its single `thread::spawn` and
// only searches the half before it.

/// Fold `first` with every further `Request` already queued on `rx`,
/// non-blocking: drains with `try_recv` until the channel is empty (or
/// disconnected) and folds each into the accumulator. Named and exposed
/// so the folding rule is provable single-threaded, by handing it a
/// receiver whose sender has already queued values and been dropped — a
/// rule proved only through a live worker is a rule proved by whichever
/// interleaving happened to occur.
///
/// The two fields fold by different rules (design.md -> Decision 9):
/// `selection` unions, because a cycle that answers about too many changes
/// is merely wasteful; `archived` takes the **last** queued value, because
/// it describes the pane's current fold and an older value is simply
/// wrong.
fn drain_and_fold(first: Request, rx: &mpsc::Receiver<Request>) -> Request {
    let mut acc = first;
    while let Ok(next) = rx.try_recv() {
        acc = Request {
            selection: acc.selection.union(next.selection),
            archived: next.archived,
        };
    }
    acc
}

/// One cycle's worktree derivation: the family in `git worktree list`'s own order
/// (never the base), each member's `Touched` set — parallel and index-aligned with
/// `members` — and every problem the derivation itself produced (a `git` too old to
/// accept `worktree list -z`, or one member's failing query). Shared, unchanged, by an
/// ordinary cycle's step 2 and by the idle re-check (design.md -> Decision 6 and task
/// 8.5's REFACTOR): the two can never derive a family by two different rules because
/// both call exactly [`derive_family`].
#[derive(Debug, Clone, Default)]
struct FamilyDerivation {
    members: Vec<crate::worktrees::Worktree>,
    touched: Vec<crate::worktrees::Touched>,
    problems: Vec<String>,
}

/// The one problem `derive_family` records when `git worktree list --porcelain -z`
/// itself fails with exit `129` — a `git` too old to accept `-z`, and per design.md ->
/// D16 the one absence of a family the reader can fix by upgrading.
const TOO_OLD_GIT_PROBLEM: &str = "git worktree list --porcelain -z failed: git is too old to accept -z, so worktree \
     changes are not shown";

/// Run `git` through `GitCli`, with `--no-optional-locks -c core.fsmonitor=false -C
/// <dir>` ahead of `rest` on every call — `worktree-overlay`'s one shared invocation
/// shape (its own "Reading the family writes nothing" requirement), so every git call
/// this module makes begins with the same five elements before its own command name.
fn git_call(git: &dyn GitCli, dir: &Path, rest: &[&str]) -> Result<String, CliError> {
    let dir_str = dir.to_string_lossy().into_owned();
    let mut args: Vec<&str> = vec![
        "--no-optional-locks",
        "-c",
        "core.fsmonitor=false",
        "-C",
        &dir_str,
    ];
    args.extend_from_slice(rest);
    git.run(&args)
}

/// The index of the record whose canonical top level is the longest one that is equal
/// to, or an ancestor of, `pane_root` — `worktrees::family`'s own base-selection rule,
/// duplicated here because that pure module does not hand back which record it chose
/// as the base, and this is the one extra fact the worker needs from it: the base's own
/// `HEAD`, which every member's `merge-base` is compared against. `None` when no record
/// contains `pane_root` at all — an ordinary repository without git, or a listing that
/// never named it.
fn base_record_index(canonical: &[Option<PathBuf>], pane_root: &Path) -> Option<usize> {
    canonical
        .iter()
        .enumerate()
        .filter_map(|(index, top)| top.as_ref().map(|path| (index, path)))
        .filter(|(_, top)| pane_root.starts_with(top.as_path()))
        .max_by_key(|(_, top)| top.as_os_str().len())
        .map(|(index, _)| index)
}

/// The one problem shape for a member whose `merge-base` (other than the
/// unrelated-history exit), `diff-tree`, or `status` call failed: names the member's
/// top level and the failing command's reason.
fn member_query_failed(top: &Path, command: &str, err: &CliError) -> String {
    let reason = match err {
        CliError::NotStarted { reason, .. } => reason.clone(),
        CliError::Failed { code, stderr, .. } => {
            let code = code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            match stderr.lines().find(|line| !line.trim().is_empty()) {
                Some(line) => format!("exited with code {code}: {line}"),
                None => format!("exited with code {code}"),
            }
        }
        CliError::TimedOut { after, .. } => format!("timed out after {after:?}"),
    };
    format!("worktree {}: git {command} failed: {reason}", top.display())
}

/// The full family derivation: `git worktree list`, then, per member, `merge-base`,
/// `diff-tree`, and `status`, in that order — `worktree-overlay`'s exactly four
/// commands — degrading per its own "Without git, or without a family, the pane is
/// exactly what it was" requirement. Shared by an ordinary cycle's step 2 and by the
/// idle re-check.
fn derive_family(repo: &Path, git: &dyn GitCli) -> FamilyDerivation {
    let stdout = match git_call(git, repo, &["worktree", "list", "--porcelain", "-z"]) {
        Ok(stdout) => stdout,
        Err(CliError::Failed {
            code: Some(129), ..
        }) => {
            return FamilyDerivation {
                members: Vec::new(),
                touched: Vec::new(),
                problems: vec![TOO_OLD_GIT_PROBLEM.to_string()],
            };
        }
        // No git binary, not a git repository, a timeout, or any other failure: an
        // ordinary repository without git, or without worktrees, silently.
        Err(_) => return FamilyDerivation::default(),
    };

    let records = crate::worktrees::parse_list(&stdout);
    let canonical: Vec<Option<PathBuf>> = records
        .iter()
        .map(|record| std::fs::canonicalize(&record.path).ok())
        .collect();

    let Some(base_index) = base_record_index(&canonical, repo) else {
        // No record contains the pane's own root at all.
        return FamilyDerivation::default();
    };
    let Some(base_head) = records[base_index].head.clone() else {
        return FamilyDerivation::default();
    };

    // `<changes>`: the OpenSpec prefix joined to `openspec/changes`, per
    // `worktree-overlay`'s "A member owns exactly the changes it touched since it
    // forked from the base". Every member shares one prefix — the base's own record
    // determines it — so it is computed once, not per member.
    let (prefix, members_with_tops) =
        crate::worktrees::family_with_tops(&records, &canonical, repo);
    let changes = if prefix.as_os_str().is_empty() {
        "openspec/changes".to_string()
    } else {
        format!("{}/openspec/changes", prefix.to_string_lossy())
    };
    let members: Vec<crate::worktrees::Worktree> = members_with_tops
        .iter()
        .map(|(_, worktree)| worktree.clone())
        .collect();
    let mut touched = Vec::with_capacity(members.len());
    let mut problems = Vec::new();

    // `top` is the member's own canonical top level — never `Worktree::root`, its
    // OpenSpec root — because `-C` must point at the checkout `diff-tree`/`status`
    // report paths relative to; `<changes>` above is the pathspec that recovers the
    // OpenSpec prefix those reported paths carry.
    for (top, _) in &members_with_tops {
        match git_call(git, top, &["merge-base", "HEAD", &base_head]) {
            Ok(out) => {
                let merge_base = out.trim();
                if merge_base.is_empty() {
                    touched.push(crate::worktrees::Touched::default());
                    continue;
                }
                let diff_out = match git_call(
                    git,
                    top,
                    &[
                        "diff-tree",
                        "-r",
                        "--name-only",
                        "-z",
                        "--no-renames",
                        merge_base,
                        "HEAD",
                        "--",
                        changes.as_str(),
                    ],
                ) {
                    Ok(out) => out,
                    Err(err) => {
                        problems.push(member_query_failed(top, "diff-tree", &err));
                        touched.push(crate::worktrees::Touched::default());
                        continue;
                    }
                };
                let status_out = match git_call(
                    git,
                    top,
                    &[
                        "status",
                        "--porcelain=v1",
                        "-z",
                        "--no-renames",
                        "--untracked-files=all",
                        "--",
                        changes.as_str(),
                    ],
                ) {
                    Ok(out) => out,
                    Err(err) => {
                        problems.push(member_query_failed(top, "status", &err));
                        touched.push(crate::worktrees::Touched::default());
                        continue;
                    }
                };
                touched.push(crate::worktrees::touched(&diff_out, &status_out, &changes));
            }
            // Unrelated history — an orphan-branch worktree is a normal thing to have,
            // per `worktree-overlay` — is silent, and the other two commands do not
            // run for this member.
            Err(CliError::Failed { code: Some(1), .. }) => {
                touched.push(crate::worktrees::Touched::default());
            }
            Err(err) => {
                problems.push(member_query_failed(top, "merge-base", &err));
                touched.push(crate::worktrees::Touched::default());
            }
        }
    }

    FamilyDerivation {
        members,
        touched,
        problems,
    }
}

/// The base's own archive directory names, read the same way `changes::from_files`'s
/// own enumeration does — `worktree-overlay`'s "the base archive directory names step
/// 1's enumeration read" — so `overlay` never re-adds a directory the base already
/// holds.
fn base_archive_dir_names(repo: &Path) -> Vec<String> {
    crate::changes::archived_entries(repo, false)
        .0
        .into_iter()
        .filter_map(|entry| {
            entry
                .dir
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_string)
        })
        .collect()
}

/// Everything the worker remembers between cycles: for `worktree-overlay`'s idle
/// re-check, which has no request of its own, and for the next cycle's fast,
/// file-sourced answer, which overlays with whatever family the previous cycle or
/// re-check last derived rather than deriving one of its own.
struct Remembered {
    /// Step 2's un-overlaid `merge(files, cli_changes)` — what the idle re-check
    /// overlays onto, never `files` alone, so a re-check's answer still carries the
    /// CLI's own corrections.
    merged: ChangeSet,
    /// The request's own `ArchivedScope`, since the idle re-check has none of its own
    /// to read.
    archived: ArchivedScope,
    base_archive_dirs: Vec<String>,
    family: FamilyDerivation,
    /// The last set actually sent — whichever of `Files`/`Merged` was sent last — so
    /// the idle re-check's "unchanged" comparison is against what the pane last saw,
    /// not against an accumulating value.
    last_sent: ChangeSet,
}

/// The worker's whole body: fold any queued requests into one, send the file-sourced
/// result under the folded request's `archived` scope, then the CLI-merged one, and
/// repeat until either channel disconnects — waiting for the next request with a plain
/// `recv` until the first cycle completes, and with `recv_timeout(recheck)` afterwards
/// so an idle interval re-derives the worktree family and re-reads every owned change
/// from its member's files (`worktree-overlay`'s idle re-check). Owns one `CliCache`
/// for its whole lifetime.
fn worker_body(
    repo: PathBuf,
    cli: Arc<dyn OpenspecCli>,
    git: Arc<dyn GitCli>,
    recheck: Duration,
    request_rx: mpsc::Receiver<Request>,
    result_tx: mpsc::Sender<RefreshResult>,
) {
    let mut cache = crate::changes::CliCache::default();
    let mut remembered: Option<Remembered> = None;

    loop {
        let has_cycled = remembered.is_some();
        let request = if !has_cycled {
            match request_rx.recv() {
                Ok(first) => drain_and_fold(first, &request_rx),
                Err(_) => return, // the Refresher was dropped
            }
        } else {
            match request_rx.recv_timeout(recheck) {
                Ok(first) => drain_and_fold(first, &request_rx),
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let rem = remembered
                        .as_mut()
                        .expect("has_cycled implies remembered is Some");
                    let derivation = derive_family(&repo, git.as_ref());
                    // `crate::changes::overlay_family` composes `from_files_owned` and
                    // `overlay` and hands back the freshly built `ChangeSet` — kept in
                    // `src/changes.rs`, on `NOLIT-CHANGE`'s own terms, rather than
                    // wrapped in a function here that would return one by value itself.
                    let mut overlaid = crate::changes::overlay_family(
                        rem.merged.clone(),
                        &rem.base_archive_dirs,
                        &derivation.members,
                        &derivation.touched,
                        rem.archived,
                    );
                    overlaid
                        .problems
                        .extend(derivation.problems.iter().cloned());
                    if overlaid != rem.last_sent {
                        if result_tx
                            .send(RefreshResult::Files(overlaid.clone()))
                            .is_err()
                        {
                            return;
                        }
                        rem.last_sent = overlaid;
                    }
                    rem.family = derivation;
                    continue;
                }
            }
        };

        // Step 1: the fast, file-sourced answer, overlaid with whatever family the
        // previous cycle or an idle re-check last derived; on the worker's first
        // cycle there is no previous family, and the file result is sent un-overlaid.
        let files = crate::changes::from_files(&repo, request.archived);
        let base_archive_dirs = base_archive_dir_names(&repo);
        let files_overlaid = match &remembered {
            Some(rem) => {
                let mut overlaid = crate::changes::overlay_family(
                    files.clone(),
                    &base_archive_dirs,
                    &rem.family.members,
                    &rem.family.touched,
                    request.archived,
                );
                overlaid
                    .problems
                    .extend(rem.family.problems.iter().cloned());
                overlaid
            }
            None => files.clone(),
        };
        if result_tx
            .send(RefreshResult::Files(files_overlaid))
            .is_err()
        {
            return; // nobody reads the result any more
        }

        // Step 2: derive the family afresh through `GitCli`, ask the CLI, and send the
        // authoritative, merged answer — `files` here is step 1's own un-overlaid set,
        // so the CLI is layered over the pane's own changes only.
        let derivation = derive_family(&repo, git.as_ref());
        let cli_changes =
            crate::changes::from_cli_cached(cli.as_ref(), &repo, &request.selection, &mut cache);
        let merged = crate::changes::merge(files, cli_changes);
        let mut merged_overlaid = crate::changes::overlay_family(
            merged.clone(),
            &base_archive_dirs,
            &derivation.members,
            &derivation.touched,
            request.archived,
        );
        merged_overlaid
            .problems
            .extend(derivation.problems.iter().cloned());
        if result_tx
            .send(RefreshResult::Merged(merged_overlaid.clone()))
            .is_err()
        {
            return;
        }

        remembered = Some(Remembered {
            merged,
            archived: request.archived,
            base_archive_dirs,
            family: derivation,
            last_sent: merged_overlaid,
        });
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
        request_tx: mpsc::Sender<Request>,
    }

    impl Refresher for TestRefresher {
        fn request(&mut self, selection: Selection, archived: ArchivedScope) {
            let _ = self.request_tx.send(Request {
                selection,
                archived,
            });
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
        git: Arc<dyn GitCli>,
        recheck: Duration,
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
            worker_body(repo, cli, git, recheck, request_rx, result_tx);
        });
        (Box::new(TestRefresher { request_tx }), result_rx, exit_rx)
    }

    /// The common five-element prefix every git call this crate makes begins with,
    /// followed by `rest` — the same shape `git_call` builds in production, restated
    /// here so a test can register a `FakeCli` response with the exact argument vector
    /// the worker will send.
    fn git_args<'a>(dir: &'a str, rest: &[&'a str]) -> Vec<&'a str> {
        let mut args: Vec<&str> = vec![
            "--no-optional-locks",
            "-c",
            "core.fsmonitor=false",
            "-C",
            dir,
        ];
        args.extend_from_slice(rest);
        args
    }

    /// A `git worktree list --porcelain -z` record for one member: `worktree <path>`,
    /// `HEAD <head>`, and `branch refs/heads/<label>` — matching `worktrees::Record`'s
    /// own shape for every scenario in this module, none of which needs a detached or
    /// bare record.
    fn wt_record(path: &std::path::Path, head: &str, branch: &str) -> String {
        format!(
            "worktree {}\0HEAD {head}\0branch refs/heads/{branch}\0\0",
            path.display()
        )
    }

    /// A scratch OpenSpec root — repository or worktree member alike — vendored with
    /// the `tdd` schema, ready for `from_files`/`from_files_owned` to read.
    fn scratch_root() -> (crate::testutil::ScratchDir, PathBuf) {
        let scratch = crate::testutil::ScratchDir::new();
        let root = crate::testutil::canonical(scratch.path());
        vendor_tdd_schema(&root);
        (scratch, root)
    }

    #[test]
    fn no_refresher_never_yields() {
        let mut r = none();
        for _ in 0..10 {
            r.request(Selection::All, ArchivedScope::Names);
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
        let (request_tx, request_rx) = mpsc::channel::<Request>();
        let (result_tx, result_rx) = mpsc::channel::<RefreshResult>();
        drop(result_tx);
        drop(request_rx);
        let mut r = RealRefresher {
            request_tx,
            result_rx,
            dead: false,
            pending_death: false,
            outstanding: false,
            pending_all: None,
        };

        match r.take_result() {
            Some(RefreshResult::Stopped(reason)) => {
                assert!(reason.to_lowercase().contains("refresh worker"), "{reason}");
            }
            other => panic!("expected Stopped, got {other:?}"),
        }

        r.request(Selection::All, ArchivedScope::Names);
        assert_eq!(r.take_result(), None, "reported once, then silence");
        r.request(Selection::All, ArchivedScope::Names);
        assert_eq!(r.take_result(), None);
    }

    /// `seam-resilience`: the other route to the same latch, and the dominant one in
    /// production — the render loop's `request` runs before its `take_result` every
    /// iteration, so on the first pass after the worker dies it is `request`'s own
    /// `SendError` that first observes the death, not `take_result`'s `try_recv`. Only
    /// the request `Receiver` is dropped here; `result_tx` stays alive, so `request`'s
    /// `send` is what fails, latching `pending_death` — the sibling branch to
    /// `a_dead_refresh_worker_is_reported_once_and_then_stops_being_reported` above, which
    /// detects the death through a disconnected result channel instead. Both must latch
    /// identically. See `specs/refresh-worker/spec.md` -> "SHALL NOT discard a
    /// `SendError` from `request`".
    #[test]
    fn a_send_error_from_request_is_reported_once_and_then_stops_being_reported() {
        let (request_tx, request_rx) = mpsc::channel::<Request>();
        let (result_tx, result_rx) = mpsc::channel::<RefreshResult>();
        drop(request_rx);
        let mut r = RealRefresher {
            request_tx,
            result_rx,
            dead: false,
            pending_death: false,
            outstanding: false,
            pending_all: None,
        };

        r.request(Selection::All, ArchivedScope::Names);
        match r.take_result() {
            Some(RefreshResult::Stopped(reason)) => {
                assert!(reason.to_lowercase().contains("refresh worker"), "{reason}");
            }
            other => panic!("expected Stopped, got {other:?}"),
        }

        r.request(Selection::All, ArchivedScope::Names);
        assert_eq!(r.take_result(), None, "reported once, then silence");
        r.request(Selection::All, ArchivedScope::Names);
        assert_eq!(r.take_result(), None);

        // `result_tx` is kept alive for the whole test: proves the `SendError` branch
        // alone, without a disconnected result channel ever coming into play.
        drop(result_tx);
    }

    /// `seam-resilience`: `refresh-worker` -> "A refresh outstanding does not queue further
    /// selections". Drives `RealRefresher::request` directly against a channel whose other
    /// end nothing ever drains, so what actually reached the worker is asserted on the
    /// channel's own contents.
    #[test]
    fn a_refresh_outstanding_does_not_queue_further_selections() {
        let (request_tx, request_rx) = mpsc::channel::<Request>();
        let (result_tx, result_rx) = mpsc::channel::<RefreshResult>();
        let mut r = RealRefresher {
            request_tx,
            result_rx,
            dead: false,
            pending_death: false,
            outstanding: false,
            pending_all: None,
        };

        r.request(
            Selection::Only(std::collections::BTreeSet::from(["alpha".to_string()])),
            ArchivedScope::Names,
        );
        r.request(
            Selection::Only(std::collections::BTreeSet::from(["beta".to_string()])),
            ArchivedScope::Names,
        );
        r.request(
            Selection::Only(std::collections::BTreeSet::from(["gamma".to_string()])),
            ArchivedScope::Names,
        );

        let received: Vec<Request> = request_rx.try_iter().collect();
        assert_eq!(
            received,
            vec![Request {
                selection: Selection::Only(std::collections::BTreeSet::from(["alpha".to_string()])),
                archived: ArchivedScope::Names,
            }],
            "exactly one request reached the worker's request channel"
        );

        result_tx
            .send(RefreshResult::Merged(crate::changes::empty_set()))
            .expect("channel still connected");
        assert!(matches!(r.take_result(), Some(RefreshResult::Merged(_))));

        r.request(
            Selection::Only(std::collections::BTreeSet::from(["delta".to_string()])),
            ArchivedScope::Names,
        );
        let received2: Vec<Request> = request_rx.try_iter().collect();
        assert_eq!(
            received2,
            vec![Request {
                selection: Selection::Only(std::collections::BTreeSet::from(["delta".to_string()])),
                archived: ArchivedScope::Names,
            }],
            "the suppression is per-cycle, not permanent"
        );
    }

    /// `seam-resilience`: `refresh-worker` -> "A forced refresh outstanding behind a
    /// narrower one is not lost".
    #[test]
    fn a_forced_refresh_outstanding_behind_a_narrower_one_is_not_lost() {
        let (request_tx, request_rx) = mpsc::channel::<Request>();
        let (result_tx, result_rx) = mpsc::channel::<RefreshResult>();
        let mut r = RealRefresher {
            request_tx,
            result_rx,
            dead: false,
            pending_death: false,
            outstanding: false,
            pending_all: None,
        };

        r.request(
            Selection::Only(std::collections::BTreeSet::from(["alpha".to_string()])),
            ArchivedScope::Names,
        );
        r.request(Selection::All, ArchivedScope::Names);
        // A second suppressed `All`, carrying a different scope: the
        // remembered request must carry *this* scope, not the first
        // suppressed one's (design.md -> Decision 9).
        r.request(Selection::All, ArchivedScope::Full);

        let first_batch: Vec<Request> = request_rx.try_iter().collect();
        assert_eq!(
            first_batch,
            vec![Request {
                selection: Selection::Only(std::collections::BTreeSet::from(["alpha".to_string()])),
                archived: ArchivedScope::Names,
            }]
        );

        result_tx
            .send(RefreshResult::Files(crate::changes::empty_set()))
            .expect("channel still connected");
        assert!(matches!(r.take_result(), Some(RefreshResult::Files(_))));
        let mid_batch: Vec<Request> = request_rx.try_iter().collect();
        assert!(
            mid_batch.is_empty(),
            "a Files result does not answer the cycle"
        );

        result_tx
            .send(RefreshResult::Merged(crate::changes::empty_set()))
            .expect("channel still connected");
        assert!(matches!(r.take_result(), Some(RefreshResult::Merged(_))));

        let second_batch: Vec<Request> = request_rx.try_iter().collect();
        assert_eq!(
            second_batch,
            vec![Request {
                selection: Selection::All,
                archived: ArchivedScope::Full,
            }],
            "the remembered All is sent once the outstanding cycle answers, carrying the \
             most recent scope rather than the first suppressed one's"
        );
    }

    #[test]
    fn start_without_a_binary_is_inert() {
        let scratch = crate::testutil::ScratchDir::new();
        let git = Arc::new(crate::cli::FakeCli::new());
        let mut r = start(Some(scratch.path()), None, git.clone());
        r.request(Selection::All, ArchivedScope::Names);
        assert_eq!(r.take_result(), None);
        assert!(
            git.calls().is_empty(),
            "no binary means no worker, so the git seam is never reached either"
        );
    }

    #[test]
    fn start_without_a_repo_is_inert() {
        let cli: Arc<dyn OpenspecCli> = Arc::new(crate::cli::FakeCli::new());
        let git = Arc::new(crate::cli::FakeCli::new());
        let mut r = start(None, Some(cli), git.clone());
        r.request(Selection::All, ArchivedScope::Names);
        assert_eq!(r.take_result(), None);
        assert!(
            git.calls().is_empty(),
            "no repository means no worker, so the git seam is never reached either"
        );
    }

    /// `refresh-worker` -> "No binary means no worker": both the no-`cli` and the
    /// no-`repo` cases return the inert `Refresher` and never touch `GitCli` at all —
    /// file mode never reads the worktree family.
    #[test]
    fn file_mode_reads_no_worktree_family() {
        let scratch = crate::testutil::ScratchDir::new();
        let cli: Arc<dyn OpenspecCli> = Arc::new(crate::cli::FakeCli::new());
        let git_a = Arc::new(crate::cli::FakeCli::new());
        let mut r = start(Some(scratch.path()), None, git_a.clone());
        for _ in 0..10 {
            r.request(Selection::All, ArchivedScope::Names);
            assert_eq!(r.take_result(), None);
        }
        assert!(git_a.calls().is_empty());

        let git_b = Arc::new(crate::cli::FakeCli::new());
        let mut r = start(None, Some(cli), git_b.clone());
        for _ in 0..10 {
            r.request(Selection::All, ArchivedScope::Names);
            assert_eq!(r.take_result(), None);
        }
        assert!(git_b.calls().is_empty());
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
        let root_str = root.display().to_string();
        let git_fake = crate::cli::FakeCli::new();
        git_fake.register_git(
            &git_args(&root_str, &["worktree", "list", "--porcelain", "-z"]),
            Err(crate::cli::CliError::NotStarted {
                program: "git".to_string(),
                args: vec!["worktree".to_string(), "list".to_string()],
                reason: "not found".to_string(),
            }),
        );
        let git: Arc<dyn GitCli> = Arc::new(git_fake);

        let (mut refresher, results_rx, _exit_rx) =
            worker_for_test(root, cli, git, WORKTREE_RECHECK);
        refresher.request(Selection::All, ArchivedScope::Names);

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
        let root_str = root.display().to_string();
        let git_fake = crate::cli::FakeCli::new();
        git_fake.register_git(
            &git_args(&root_str, &["worktree", "list", "--porcelain", "-z"]),
            Err(crate::cli::CliError::NotStarted {
                program: "git".to_string(),
                args: vec!["worktree".to_string(), "list".to_string()],
                reason: "not found".to_string(),
            }),
        );
        let git: Arc<dyn GitCli> = Arc::new(git_fake);

        let (mut refresher, results_rx, _exit_rx) =
            worker_for_test(root, cli, git, WORKTREE_RECHECK);
        refresher.request(Selection::All, ArchivedScope::Names);

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
        let root_str = root.display().to_string();
        let git_fake = crate::cli::FakeCli::new();
        git_fake.register_git(
            &git_args(&root_str, &["worktree", "list", "--porcelain", "-z"]),
            Err(crate::cli::CliError::NotStarted {
                program: "git".to_string(),
                args: vec!["worktree".to_string(), "list".to_string()],
                reason: "not found".to_string(),
            }),
        );
        let git: Arc<dyn GitCli> = Arc::new(git_fake);

        let (mut refresher, results_rx, _exit_rx) =
            worker_for_test(root, cli, git, WORKTREE_RECHECK);
        refresher.request(Selection::All, ArchivedScope::Names);

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

    /// `refresh-worker` -> "The scope on the request is the scope the file
    /// tier runs under": `worker_body` no longer resolves a fixed scope —
    /// it runs `from_files` under the folded request's own `archived`
    /// field, so a `Names` cycle counts the archive without opening a
    /// single archived change and a `Full` cycle over the same tree resolves
    /// every one, with the active list and the problem set unaffected
    /// either way.
    #[test]
    fn the_scope_on_the_request_is_the_scope_the_file_tier_runs_under() {
        let scratch = crate::testutil::ScratchDir::new();
        let root = crate::testutil::canonical(scratch.path());
        vendor_tdd_schema(&root);
        for (day, name) in [
            ("01", "one"),
            ("02", "two"),
            ("03", "three"),
            ("04", "four"),
        ] {
            std::fs::create_dir_all(
                root.join("openspec/changes/archive")
                    .join(format!("2026-01-{day}-{name}")),
            )
            .expect("create archived directory");
        }
        let fake = crate::cli::FakeCli::new();
        fake.register_openspec(
            &["list", "--json"],
            Ok(format!(
                r#"{{"changes":[],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                root.display().to_string()
            )),
        );
        let cli: Arc<dyn OpenspecCli> = Arc::new(fake);
        let root_str = root.display().to_string();
        let git_fake = crate::cli::FakeCli::new();
        git_fake.register_git(
            &git_args(&root_str, &["worktree", "list", "--porcelain", "-z"]),
            Err(crate::cli::CliError::NotStarted {
                program: "git".to_string(),
                args: vec!["worktree".to_string(), "list".to_string()],
                reason: "not found".to_string(),
            }),
        );
        let git: Arc<dyn GitCli> = Arc::new(git_fake);

        let (mut refresher, results_rx, _exit_rx) =
            worker_for_test(root, cli, git, WORKTREE_RECHECK);

        refresher.request(Selection::All, ArchivedScope::Names);
        let names_files = results_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("the worker did not send the Names file result within 10s");
        let names_merged = results_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("the worker did not send the Names merged result within 10s");
        for (label, result) in [("Files", &names_files), ("Merged", &names_merged)] {
            match result {
                RefreshResult::Files(set) | RefreshResult::Merged(set) => {
                    assert!(
                        set.archived.is_empty(),
                        "{label}: Names must build no archived Change"
                    );
                    assert_eq!(set.archived_total, 4, "{label}");
                    assert!(set.problems.is_empty(), "{label}: {:?}", set.problems);
                }
                other => panic!("{label}: expected Files/Merged, got {other:?}"),
            }
        }

        refresher.request(Selection::All, ArchivedScope::Full);
        let full_files = results_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("the worker did not send the Full file result within 10s");
        let full_merged = results_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("the worker did not send the Full merged result within 10s");
        for (label, result) in [("Files", &full_files), ("Merged", &full_merged)] {
            match result {
                RefreshResult::Files(set) | RefreshResult::Merged(set) => {
                    assert_eq!(set.archived.len(), 4, "{label}");
                    assert_eq!(set.archived_total, 4, "{label}");
                    assert!(set.problems.is_empty(), "{label}: {:?}", set.problems);
                }
                other => panic!("{label}: expected Files/Merged, got {other:?}"),
            }
        }

        fn active(r: &RefreshResult) -> Vec<crate::changes::Change> {
            match r {
                RefreshResult::Files(set) | RefreshResult::Merged(set) => set.active.clone(),
                RefreshResult::Stopped(_) => unreachable!("no Stopped result in this test"),
            }
        }
        assert_eq!(
            active(&names_files),
            active(&full_files),
            "the scope reached from_files and changed nothing else"
        );
        assert_eq!(active(&names_merged), active(&full_merged));
    }

    /// `refresh-worker` -> "`drain_and_fold` unions selections and takes the
    /// last scope". Driven single-threaded, on the scenario's own terms: a
    /// threaded version cannot fail (design.md -> spec's own reasoning for
    /// why this is not a worker-driven test), so `drain_and_fold` is called
    /// directly against a receiver whose sender queued values and was
    /// dropped.
    #[test]
    fn drain_and_fold_unions_selections_and_takes_the_last_scope() {
        let (tx, rx) = mpsc::channel::<Request>();
        tx.send(Request {
            selection: Selection::Only(std::collections::BTreeSet::from(["b".to_string()])),
            archived: ArchivedScope::Names,
        })
        .unwrap();
        tx.send(Request {
            selection: Selection::Only(std::collections::BTreeSet::from(["c".to_string()])),
            archived: ArchivedScope::Names,
        })
        .unwrap();
        drop(tx);

        let result = drain_and_fold(
            Request {
                selection: Selection::Only(std::collections::BTreeSet::from(["a".to_string()])),
                archived: ArchivedScope::Full,
            },
            &rx,
        );
        assert_eq!(
            result,
            Request {
                selection: Selection::Only(std::collections::BTreeSet::from([
                    "a".to_string(),
                    "b".to_string(),
                    "c".to_string()
                ])),
                archived: ArchivedScope::Names,
            },
            "selection unions; archived takes the last queued value"
        );

        // The same call with the two queued requests carrying `Names` then `Full` returns
        // `Full`, so the rule is "last wins" and not "widest wins".
        let (tx2, rx2) = mpsc::channel::<Request>();
        tx2.send(Request {
            selection: Selection::Only(std::collections::BTreeSet::new()),
            archived: ArchivedScope::Names,
        })
        .unwrap();
        tx2.send(Request {
            selection: Selection::Only(std::collections::BTreeSet::new()),
            archived: ArchivedScope::Full,
        })
        .unwrap();
        drop(tx2);
        assert_eq!(
            drain_and_fold(
                Request {
                    selection: Selection::Only(std::collections::BTreeSet::new()),
                    archived: ArchivedScope::Names,
                },
                &rx2,
            )
            .archived,
            ArchivedScope::Full
        );

        // An empty, dropped receiver returns `first` unchanged, both fields included.
        let (tx3, rx3) = mpsc::channel::<Request>();
        drop(tx3);
        let first = Request {
            selection: Selection::Only(std::collections::BTreeSet::new()),
            archived: ArchivedScope::Full,
        };
        assert_eq!(drain_and_fold(first.clone(), &rx3), first);
    }

    #[test]
    fn dropping_the_refresher_disconnects_the_channel() {
        let scratch = crate::testutil::ScratchDir::new();
        let root = crate::testutil::canonical(scratch.path());
        let fake: Arc<dyn OpenspecCli> = Arc::new(crate::cli::FakeCli::new());
        let git: Arc<dyn GitCli> = Arc::new(crate::cli::FakeCli::new());

        let (refresher, _results_rx, exit_rx) = worker_for_test(root, fake, git, WORKTREE_RECHECK);
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
        let root_str = root.display().to_string();
        let git_fake = crate::cli::FakeCli::new();
        git_fake.register_git(
            &git_args(&root_str, &["worktree", "list", "--porcelain", "-z"]),
            Err(crate::cli::CliError::NotStarted {
                program: "git".to_string(),
                args: vec!["worktree".to_string(), "list".to_string()],
                reason: "not found".to_string(),
            }),
        );
        let git: Arc<dyn GitCli> = Arc::new(git_fake);

        let before = crate::testutil::snapshot(&root);
        let (mut refresher, results_rx, exit_rx) =
            worker_for_test(root.clone(), cli, git, WORKTREE_RECHECK);
        refresher.request(Selection::All, ArchivedScope::Names);
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
    // --- group 8: `worktree-overlay` reaches the refresh worker ---

    /// `refresh-worker` -> "A worktree copy reaches the merged result first and the
    /// file result after".
    #[test]
    fn a_worktree_copy_reaches_the_merged_result_first_and_the_file_result_after() {
        let (base_scratch, base_root) = scratch_root();
        crate::testutil::write_with_mode(
            &base_root.join("openspec/changes/alpha/tasks.md"),
            "- [ ] a\n".repeat(9).as_bytes(),
            0o644,
        );
        let (member_scratch, member_root) = scratch_root();
        let mut member_tasks = "- [x] a\n".repeat(5);
        member_tasks.push_str(&"- [ ] a\n".repeat(4));
        crate::testutil::write_with_mode(
            &member_root.join("openspec/changes/alpha/tasks.md"),
            member_tasks.as_bytes(),
            0o644,
        );

        let openspec_fake = Arc::new(crate::cli::FakeCli::new());
        openspec_fake.register_openspec(
            &["list", "--json"],
            Ok(format!(
                r#"{{"changes":[{{"name":"alpha","completedTasks":0,"totalTasks":9,"lastModified":"x","status":"y"}}],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                base_root.display().to_string()
            )),
        );
        openspec_fake.register_openspec(
            &["instructions", "apply", "--change", "alpha", "--json"],
            Ok(format!(
                r#"{{"schemaName":"tdd","changeDir":{:?},"contextFiles":{{}}}}"#,
                base_root
                    .join("openspec/changes/alpha")
                    .display()
                    .to_string()
            )),
        );
        let cli: Arc<dyn OpenspecCli> = openspec_fake.clone();

        let base_str = base_root.display().to_string();
        let member_str = member_root.display().to_string();
        let git_fake = Arc::new(crate::cli::FakeCli::new());
        git_fake.register_git(
            &git_args(&base_str, &["worktree", "list", "--porcelain", "-z"]),
            Ok(format!(
                "{}{}",
                wt_record(&base_root, "aaaa", "main"),
                wt_record(&member_root, "bbbb", "feat"),
            )),
        );
        git_fake.register_git(
            &git_args(&member_str, &["merge-base", "HEAD", "aaaa"]),
            Ok("basecommit".to_string()),
        );
        git_fake.register_git(
            &git_args(
                &member_str,
                &[
                    "diff-tree",
                    "-r",
                    "--name-only",
                    "-z",
                    "--no-renames",
                    "basecommit",
                    "HEAD",
                    "--",
                    "openspec/changes",
                ],
            ),
            Ok(String::new()),
        );
        git_fake.register_git(
            &git_args(
                &member_str,
                &[
                    "status",
                    "--porcelain=v1",
                    "-z",
                    "--no-renames",
                    "--untracked-files=all",
                    "--",
                    "openspec/changes",
                ],
            ),
            Ok(" M openspec/changes/alpha/tasks.md\0".to_string()),
        );
        let git: Arc<dyn GitCli> = git_fake.clone();

        let (mut refresher, results_rx, _exit_rx) =
            worker_for_test(base_root, cli, git, Duration::from_millis(5));
        refresher.request(Selection::All, ArchivedScope::Names);

        let first_files = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("first files");
        match first_files {
            RefreshResult::Files(set) => {
                let alpha = set.active.iter().find(|c| c.name == "alpha").unwrap();
                assert_eq!(alpha.progress.completed, 0);
                assert!(set.worktrees.is_empty());
            }
            other => panic!("expected Files, got {other:?}"),
        }
        let first_merged = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("first merged");
        match first_merged {
            RefreshResult::Merged(set) => {
                let alpha = set.active.iter().find(|c| c.name == "alpha").unwrap();
                assert_eq!(alpha.progress.completed, 5);
                assert_eq!(alpha.progress.total, 9);
                assert!(alpha.dir.starts_with(&member_root));
                assert_eq!(set.worktrees.len(), 1);
                assert_eq!(set.worktrees[0].label, "feat");
            }
            other => panic!("expected Merged, got {other:?}"),
        }

        refresher.request(Selection::All, ArchivedScope::Names);
        let second_files = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("second files");
        match second_files {
            RefreshResult::Files(set) => {
                let alpha = set.active.iter().find(|c| c.name == "alpha").unwrap();
                assert_eq!(
                    alpha.progress.completed, 5,
                    "the fast answer must not regress a worktree row to the base's copy"
                );
            }
            other => panic!("expected Files, got {other:?}"),
        }
        let _second_merged = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("second merged");

        assert!(
            openspec_fake
                .calls()
                .iter()
                .all(|(_, args)| !args.iter().any(|a| a.contains(&member_str))),
            "the openspec CLI must be asked about the base only"
        );
        let _ = base_scratch;
        let _ = member_scratch;
    }

    /// `refresh-worker` -> "A worktree created after the last cycle appears without a
    /// request".
    #[test]
    fn a_worktree_created_after_the_last_cycle_appears_without_a_request() {
        let (base_scratch, base_root) = scratch_root();
        crate::testutil::write_with_mode(
            &base_root.join("openspec/changes/alpha/tasks.md"),
            format!("{}{}", "- [x] a\n".repeat(4), "- [ ] a\n".repeat(5)).as_bytes(),
            0o644,
        );
        let (member_scratch, member_root) = scratch_root();
        crate::testutil::write_with_mode(
            &member_root.join("openspec/changes/beta/tasks.md"),
            b"- [ ] a\n",
            0o644,
        );

        let openspec_fake = Arc::new(crate::cli::FakeCli::new());
        openspec_fake.register_openspec(
            &["list", "--json"],
            Ok(format!(
                r#"{{"changes":[{{"name":"alpha","completedTasks":7,"totalTasks":9,"lastModified":"x","status":"y"}}],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                base_root.display().to_string()
            )),
        );
        openspec_fake.register_openspec(
            &["instructions", "apply", "--change", "alpha", "--json"],
            Ok(format!(
                r#"{{"schemaName":"tdd","changeDir":{:?},"contextFiles":{{}}}}"#,
                base_root
                    .join("openspec/changes/alpha")
                    .display()
                    .to_string()
            )),
        );
        let cli: Arc<dyn OpenspecCli> = openspec_fake.clone();

        let base_str = base_root.display().to_string();
        let member_str = member_root.display().to_string();
        let git_fake = Arc::new(crate::cli::FakeCli::new());
        let list_args = git_args(&base_str, &["worktree", "list", "--porcelain", "-z"]);
        git_fake.register_git(&list_args, Ok(wt_record(&base_root, "aaaa", "main")));
        git_fake.register_git(
            &list_args,
            Ok(format!(
                "{}{}",
                wt_record(&base_root, "aaaa", "main"),
                wt_record(&member_root, "bbbb", "feat"),
            )),
        );
        git_fake.register_git(
            &git_args(&member_str, &["merge-base", "HEAD", "aaaa"]),
            Ok("msha".to_string()),
        );
        git_fake.register_git(
            &git_args(
                &member_str,
                &[
                    "diff-tree",
                    "-r",
                    "--name-only",
                    "-z",
                    "--no-renames",
                    "msha",
                    "HEAD",
                    "--",
                    "openspec/changes",
                ],
            ),
            Ok(String::new()),
        );
        git_fake.register_git(
            &git_args(
                &member_str,
                &[
                    "status",
                    "--porcelain=v1",
                    "-z",
                    "--no-renames",
                    "--untracked-files=all",
                    "--",
                    "openspec/changes",
                ],
            ),
            Ok("?? openspec/changes/beta/tasks.md\0".to_string()),
        );
        let git: Arc<dyn GitCli> = git_fake.clone();

        let (mut refresher, results_rx, _exit_rx) =
            worker_for_test(base_root, cli, git, Duration::from_millis(5));
        refresher.request(Selection::All, ArchivedScope::Names);

        let files = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("first files");
        assert!(matches!(files, RefreshResult::Files(_)));
        let merged = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("first merged");
        match merged {
            RefreshResult::Merged(set) => {
                let alpha = set.active.iter().find(|c| c.name == "alpha").unwrap();
                assert_eq!(alpha.progress.completed, 7);
                assert!(set.worktrees.is_empty());
            }
            other => panic!("expected Merged, got {other:?}"),
        }

        let unsolicited = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the re-check did not report the new member within 10s");
        match unsolicited {
            RefreshResult::Files(set) => {
                assert!(set.active.iter().any(|c| c.name == "beta"));
                assert_eq!(set.worktrees.len(), 1);
                assert_eq!(set.worktrees[0].label, "feat");
                let alpha = set.active.iter().find(|c| c.name == "alpha").unwrap();
                assert_eq!(
                    alpha.progress.completed, 7,
                    "the re-check overlaid the remembered merged base, not a fresh file read"
                );
            }
            other => panic!("expected an unsolicited Files, got {other:?}"),
        }

        assert_eq!(
            openspec_fake.calls().len(),
            2,
            "the re-check must not call the openspec CLI"
        );
        let _ = base_scratch;
        let _ = member_scratch;
    }

    /// `refresh-worker` -> "A re-check's discovery survives the next request's fast
    /// answer".
    #[test]
    fn a_re_checks_discovery_survives_the_next_requests_fast_answer() {
        let (base_scratch, base_root) = scratch_root();
        crate::testutil::write_with_mode(
            &base_root.join("openspec/changes/alpha/tasks.md"),
            format!("{}{}", "- [x] a\n".repeat(4), "- [ ] a\n".repeat(5)).as_bytes(),
            0o644,
        );
        let (member_scratch, member_root) = scratch_root();
        crate::testutil::write_with_mode(
            &member_root.join("openspec/changes/beta/tasks.md"),
            b"- [ ] a\n",
            0o644,
        );

        let openspec_fake = Arc::new(crate::cli::FakeCli::new());
        openspec_fake.register_openspec(
            &["list", "--json"],
            Ok(format!(
                r#"{{"changes":[{{"name":"alpha","completedTasks":7,"totalTasks":9,"lastModified":"x","status":"y"}}],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                base_root.display().to_string()
            )),
        );
        openspec_fake.register_openspec(
            &["instructions", "apply", "--change", "alpha", "--json"],
            Ok(format!(
                r#"{{"schemaName":"tdd","changeDir":{:?},"contextFiles":{{}}}}"#,
                base_root
                    .join("openspec/changes/alpha")
                    .display()
                    .to_string()
            )),
        );
        let cli: Arc<dyn OpenspecCli> = openspec_fake.clone();

        let base_str = base_root.display().to_string();
        let member_str = member_root.display().to_string();
        let git_fake = Arc::new(crate::cli::FakeCli::new());
        let list_args = git_args(&base_str, &["worktree", "list", "--porcelain", "-z"]);
        git_fake.register_git(&list_args, Ok(wt_record(&base_root, "aaaa", "main")));
        git_fake.register_git(
            &list_args,
            Ok(format!(
                "{}{}",
                wt_record(&base_root, "aaaa", "main"),
                wt_record(&member_root, "bbbb", "feat"),
            )),
        );
        git_fake.register_git(
            &git_args(&member_str, &["merge-base", "HEAD", "aaaa"]),
            Ok("msha".to_string()),
        );
        git_fake.register_git(
            &git_args(
                &member_str,
                &[
                    "diff-tree",
                    "-r",
                    "--name-only",
                    "-z",
                    "--no-renames",
                    "msha",
                    "HEAD",
                    "--",
                    "openspec/changes",
                ],
            ),
            Ok(String::new()),
        );
        git_fake.register_git(
            &git_args(
                &member_str,
                &[
                    "status",
                    "--porcelain=v1",
                    "-z",
                    "--no-renames",
                    "--untracked-files=all",
                    "--",
                    "openspec/changes",
                ],
            ),
            Ok("?? openspec/changes/beta/tasks.md\0".to_string()),
        );
        let git: Arc<dyn GitCli> = git_fake.clone();

        let (mut refresher, results_rx, _exit_rx) =
            worker_for_test(base_root, cli, git, Duration::from_millis(5));
        refresher.request(Selection::All, ArchivedScope::Names);
        let _files = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("first files");
        let _merged = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("first merged");
        let unsolicited = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the re-check did not report the new member within 10s");
        assert!(
            matches!(&unsolicited, RefreshResult::Files(set) if set.active.iter().any(|c| c.name == "beta"))
        );

        refresher.request(Selection::All, ArchivedScope::Names);
        let third_files = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("third files");
        match third_files {
            RefreshResult::Files(set) => {
                assert!(set.active.iter().any(|c| c.name == "beta"))
            }
            other => panic!("expected Files, got {other:?}"),
        }
        let third_merged = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("third merged");
        match third_merged {
            RefreshResult::Merged(set) => {
                assert!(set.active.iter().any(|c| c.name == "beta"))
            }
            other => panic!("expected Merged, got {other:?}"),
        }
        let _ = base_scratch;
        let _ = member_scratch;
    }

    /// `refresh-worker` -> "An unchanged overlay sends nothing".
    #[test]
    fn an_unchanged_overlay_sends_nothing() {
        let (base_scratch, base_root) = scratch_root();
        let (feat_scratch, feat_root) = scratch_root();
        crate::testutil::write_with_mode(
            &feat_root.join("openspec/changes/x/tasks.md"),
            b"- [ ] a\n",
            0o644,
        );
        let (fix_scratch, fix_root) = scratch_root();
        crate::testutil::write_with_mode(
            &fix_root.join("openspec/changes/x/tasks.md"),
            b"- [x] a\n",
            0o644,
        );

        let openspec_fake = Arc::new(crate::cli::FakeCli::new());
        openspec_fake.register_openspec(
            &["list", "--json"],
            Ok(format!(
                r#"{{"changes":[],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                base_root.display().to_string()
            )),
        );
        let cli: Arc<dyn OpenspecCli> = openspec_fake.clone();

        let base_str = base_root.display().to_string();
        let feat_str = feat_root.display().to_string();
        let fix_str = fix_root.display().to_string();
        let git_fake = Arc::new(crate::cli::FakeCli::new());
        let list_args = git_args(&base_str, &["worktree", "list", "--porcelain", "-z"]);
        git_fake.register_git(
            &list_args,
            Ok(format!(
                "{}{}{}",
                wt_record(&base_root, "aaaa", "main"),
                wt_record(&feat_root, "bbbb", "feat"),
                wt_record(&fix_root, "cccc", "fix"),
            )),
        );
        for (root_str, sha) in [(&feat_str, "bbbb"), (&fix_str, "cccc")] {
            git_fake.register_git(
                &git_args(root_str, &["merge-base", "HEAD", "aaaa"]),
                Ok(format!("base-{sha}")),
            );
            git_fake.register_git(
                &git_args(
                    root_str,
                    &[
                        "diff-tree",
                        "-r",
                        "--name-only",
                        "-z",
                        "--no-renames",
                        &format!("base-{sha}"),
                        "HEAD",
                        "--",
                        "openspec/changes",
                    ],
                ),
                Ok(String::new()),
            );
            git_fake.register_git(
                &git_args(
                    root_str,
                    &[
                        "status",
                        "--porcelain=v1",
                        "-z",
                        "--no-renames",
                        "--untracked-files=all",
                        "--",
                        "openspec/changes",
                    ],
                ),
                Ok("?? openspec/changes/x/tasks.md\0".to_string()),
            );
        }
        let git: Arc<dyn GitCli> = git_fake.clone();

        let (mut refresher, results_rx, _exit_rx) =
            worker_for_test(base_root, cli, git, Duration::from_millis(5));
        refresher.request(Selection::All, ArchivedScope::Names);
        let _files = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("files");
        let merged = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("merged");
        match merged {
            RefreshResult::Merged(set) => assert_eq!(set.problems.len(), 1, "{:?}", set.problems),
            other => panic!("expected Merged, got {other:?}"),
        }

        match results_rx.recv_timeout(Duration::from_millis(150)) {
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            other => panic!("expected no further result, got {other:?}"),
        }

        let list_key: Vec<String> = list_args.iter().map(|s| s.to_string()).collect();
        let list_calls = git_fake
            .calls()
            .into_iter()
            .filter(|(program, args)| *program == crate::cli::Program::Git && *args == list_key)
            .count();
        assert!(
            list_calls >= 3,
            "expected at least 3 `worktree list` calls (1 cycle + >=2 re-checks), got {list_calls}"
        );
        let _ = base_scratch;
        let _ = feat_scratch;
        let _ = fix_scratch;
    }

    /// `refresh-worker` -> "A ticked task inside a worktree reaches the pane".
    #[test]
    fn a_ticked_task_inside_a_worktree_reaches_the_pane() {
        let (base_scratch, base_root) = scratch_root();
        let (member_scratch, member_root) = scratch_root();
        let mut tasks = "- [x] a\n".repeat(5);
        tasks.push_str(&"- [ ] a\n".repeat(4));
        crate::testutil::write_with_mode(
            &member_root.join("openspec/changes/gamma/tasks.md"),
            tasks.as_bytes(),
            0o644,
        );

        let openspec_fake = Arc::new(crate::cli::FakeCli::new());
        openspec_fake.register_openspec(
            &["list", "--json"],
            Ok(format!(
                r#"{{"changes":[],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                base_root.display().to_string()
            )),
        );
        let cli: Arc<dyn OpenspecCli> = openspec_fake.clone();

        let base_str = base_root.display().to_string();
        let member_str = member_root.display().to_string();
        let git_fake = Arc::new(crate::cli::FakeCli::new());
        git_fake.register_git(
            &git_args(&base_str, &["worktree", "list", "--porcelain", "-z"]),
            Ok(format!(
                "{}{}",
                wt_record(&base_root, "aaaa", "main"),
                wt_record(&member_root, "bbbb", "feat"),
            )),
        );
        git_fake.register_git(
            &git_args(&member_str, &["merge-base", "HEAD", "aaaa"]),
            Ok("msha".to_string()),
        );
        git_fake.register_git(
            &git_args(
                &member_str,
                &[
                    "diff-tree",
                    "-r",
                    "--name-only",
                    "-z",
                    "--no-renames",
                    "msha",
                    "HEAD",
                    "--",
                    "openspec/changes",
                ],
            ),
            Ok(String::new()),
        );
        git_fake.register_git(
            &git_args(
                &member_str,
                &[
                    "status",
                    "--porcelain=v1",
                    "-z",
                    "--no-renames",
                    "--untracked-files=all",
                    "--",
                    "openspec/changes",
                ],
            ),
            Ok("?? openspec/changes/gamma/tasks.md\0".to_string()),
        );
        let git: Arc<dyn GitCli> = git_fake.clone();

        let (mut refresher, results_rx, _exit_rx) =
            worker_for_test(base_root, cli, git, Duration::from_millis(5));
        refresher.request(Selection::All, ArchivedScope::Names);
        let _files = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("files");
        let merged = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("merged");
        match merged {
            RefreshResult::Merged(set) => {
                let gamma = set.active.iter().find(|c| c.name == "gamma").unwrap();
                assert_eq!(gamma.progress.completed, 5);
            }
            other => panic!("expected Merged, got {other:?}"),
        }

        let mut ticked = "- [x] a\n".repeat(6);
        ticked.push_str(&"- [ ] a\n".repeat(3));
        let sibling = member_root.join("openspec/changes/gamma/tasks.md.next");
        std::fs::write(&sibling, ticked).expect("write sibling");
        std::fs::rename(
            &sibling,
            member_root.join("openspec/changes/gamma/tasks.md"),
        )
        .expect("rename over tasks.md");

        let unsolicited = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the re-check did not report the ticked task within 10s");
        match unsolicited {
            RefreshResult::Files(set) => {
                let gamma = set.active.iter().find(|c| c.name == "gamma").unwrap();
                assert_eq!(gamma.progress.completed, 6);
                assert!(gamma.dir.starts_with(&member_root));
            }
            other => panic!("expected an unsolicited Files, got {other:?}"),
        }
        let _ = base_scratch;
        let _ = member_scratch;
    }

    /// `worktree-overlay` -> "A member owns exactly the changes it touched since it
    /// forked from the base": when the pane's own root sits inside a subdirectory of
    /// its checkout's top level (a nonempty OpenSpec prefix, `sub`), every member
    /// query SHALL run `-C` against the member's own canonical **top level** — never
    /// its OpenSpec root, which the top level joined with `sub` — and the pathspec
    /// SHALL be `sub/openspec/changes`. `diff-tree`/`status` report paths relative to
    /// the repository top, not to `-C`'s directory, so a `diff-tree` answer naming
    /// `sub/openspec/changes/x/tasks.md` overlays the member's own copy of `x` only
    /// when both are right.
    #[test]
    fn a_member_query_runs_at_the_members_top_level_when_the_pane_has_a_nested_openspec_root() {
        let base_scratch = crate::testutil::ScratchDir::new();
        let base_top = crate::testutil::canonical(base_scratch.path());
        let base_root = base_top.join("sub");
        vendor_tdd_schema(&base_root);
        crate::testutil::write_with_mode(
            &base_root.join("openspec/changes/x/tasks.md"),
            b"- [ ] a\n",
            0o644,
        );

        let member_scratch = crate::testutil::ScratchDir::new();
        let member_top = crate::testutil::canonical(member_scratch.path());
        let member_root = member_top.join("sub");
        vendor_tdd_schema(&member_root);
        crate::testutil::write_with_mode(
            &member_root.join("openspec/changes/x/tasks.md"),
            b"- [x] a\n",
            0o644,
        );

        let openspec_fake = Arc::new(crate::cli::FakeCli::new());
        openspec_fake.register_openspec(
            &["list", "--json"],
            Ok(format!(
                r#"{{"changes":[],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                base_root.display().to_string()
            )),
        );
        let cli: Arc<dyn OpenspecCli> = openspec_fake;

        let base_root_str = base_root.display().to_string();
        let member_top_str = member_top.display().to_string();
        let git_fake = Arc::new(crate::cli::FakeCli::new());
        let list_args = git_args(&base_root_str, &["worktree", "list", "--porcelain", "-z"]);
        git_fake.register_git(
            &list_args,
            Ok(format!(
                "{}{}",
                wt_record(&base_top, "aaaa", "main"),
                wt_record(&member_top, "bbbb", "feat"),
            )),
        );
        // Registered only at the member's own top level, never at its OpenSpec root
        // (`member_top.join("sub")`) — a `derive_family` that still runs `-C` against
        // the OpenSpec root finds no registered response and the fake panics inside
        // the worker thread, which is this test's RED failure.
        git_fake.register_git(
            &git_args(&member_top_str, &["merge-base", "HEAD", "aaaa"]),
            Ok("msha".to_string()),
        );
        git_fake.register_git(
            &git_args(
                &member_top_str,
                &[
                    "diff-tree",
                    "-r",
                    "--name-only",
                    "-z",
                    "--no-renames",
                    "msha",
                    "HEAD",
                    "--",
                    "sub/openspec/changes",
                ],
            ),
            Ok("sub/openspec/changes/x/tasks.md\0".to_string()),
        );
        git_fake.register_git(
            &git_args(
                &member_top_str,
                &[
                    "status",
                    "--porcelain=v1",
                    "-z",
                    "--no-renames",
                    "--untracked-files=all",
                    "--",
                    "sub/openspec/changes",
                ],
            ),
            Ok(String::new()),
        );
        let git: Arc<dyn GitCli> = git_fake.clone();

        let (mut refresher, results_rx, _exit_rx) =
            worker_for_test(base_root.clone(), cli, git, Duration::from_millis(5));
        refresher.request(Selection::All, ArchivedScope::Names);
        let _files = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("files");
        let merged = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("merged");
        match merged {
            RefreshResult::Merged(set) => {
                assert!(set.problems.is_empty(), "{:?}", set.problems);
                let x = set.active.iter().find(|c| c.name == "x").unwrap();
                assert_eq!(
                    x.progress.completed, 1,
                    "expected the member's own copy of x"
                );
                assert!(
                    x.dir.starts_with(&member_root),
                    "expected x's dir under the member's OpenSpec root {}, got {:?}",
                    member_root.display(),
                    x.dir
                );
            }
            other => panic!("expected Merged, got {other:?}"),
        }

        let expected_calls: Vec<Vec<String>> = [
            git_args(&member_top_str, &["merge-base", "HEAD", "aaaa"]),
            git_args(
                &member_top_str,
                &[
                    "diff-tree",
                    "-r",
                    "--name-only",
                    "-z",
                    "--no-renames",
                    "msha",
                    "HEAD",
                    "--",
                    "sub/openspec/changes",
                ],
            ),
            git_args(
                &member_top_str,
                &[
                    "status",
                    "--porcelain=v1",
                    "-z",
                    "--no-renames",
                    "--untracked-files=all",
                    "--",
                    "sub/openspec/changes",
                ],
            ),
        ]
        .into_iter()
        .map(|args| args.into_iter().map(str::to_string).collect())
        .collect();
        let calls = git_fake.calls();
        for expected in expected_calls {
            assert!(
                calls.contains(&(crate::cli::Program::Git, expected.clone())),
                "expected a call {expected:?} at the member's top level {member_top_str}, got {calls:?}"
            );
        }

        let _ = base_scratch;
        let _ = member_scratch;
    }

    /// `refresh-worker` -> "No re-check before the first cycle".
    #[test]
    fn no_re_check_before_the_first_cycle() {
        let (base_scratch, base_root) = scratch_root();
        let cli: Arc<dyn OpenspecCli> = Arc::new(crate::cli::FakeCli::new());
        let git_fake = Arc::new(crate::cli::FakeCli::new());
        let git: Arc<dyn GitCli> = git_fake.clone();

        let (_refresher, results_rx, _exit_rx) =
            worker_for_test(base_root, cli, git, Duration::from_millis(5));

        match results_rx.recv_timeout(Duration::from_millis(50)) {
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            other => panic!("expected Timeout, got {other:?}"),
        }
        assert!(git_fake.calls().is_empty());
        let _ = base_scratch;
    }

    /// `refresh-worker` -> "A worker that has cycled still returns when its refresher
    /// is dropped".
    #[test]
    fn a_worker_that_has_cycled_still_returns_when_its_refresher_is_dropped() {
        let (base_scratch, base_root) = scratch_root();
        let openspec_fake = Arc::new(crate::cli::FakeCli::new());
        openspec_fake.register_openspec(
            &["list", "--json"],
            Ok(format!(
                r#"{{"changes":[],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                base_root.display().to_string()
            )),
        );
        let cli: Arc<dyn OpenspecCli> = openspec_fake;
        let base_str = base_root.display().to_string();
        let git_fake = Arc::new(crate::cli::FakeCli::new());
        git_fake.register_git(
            &git_args(&base_str, &["worktree", "list", "--porcelain", "-z"]),
            Err(crate::cli::CliError::NotStarted {
                program: "git".to_string(),
                args: vec!["worktree".to_string(), "list".to_string()],
                reason: "not found".to_string(),
            }),
        );
        let git: Arc<dyn GitCli> = git_fake;

        let (mut refresher, results_rx, exit_rx) =
            worker_for_test(base_root, cli, git, Duration::from_millis(5));
        refresher.request(Selection::All, ArchivedScope::Names);
        results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("files");
        results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("merged");

        drop(refresher);
        match exit_rx.recv_timeout(Duration::from_secs(10)) {
            Err(mpsc::RecvTimeoutError::Disconnected) => {}
            other => panic!(
                "the worker did not return within 10s after its Refresher was dropped: {other:?}"
            ),
        }
        let _ = base_scratch;
    }

    /// `worktree-overlay` -> "No git binary".
    #[test]
    fn no_git_binary_leaves_the_set_unoverlaid() {
        let (base_scratch, base_root) = scratch_root();
        crate::testutil::write_with_mode(
            &base_root.join("openspec/changes/alpha/tasks.md"),
            b"- [x] a\n- [ ] b\n",
            0o644,
        );
        let openspec_fake = Arc::new(crate::cli::FakeCli::new());
        openspec_fake.register_openspec(
            &["list", "--json"],
            Ok(format!(
                r#"{{"changes":[{{"name":"alpha","completedTasks":1,"totalTasks":2,"lastModified":"x","status":"y"}}],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                base_root.display().to_string()
            )),
        );
        openspec_fake.register_openspec(
            &["instructions", "apply", "--change", "alpha", "--json"],
            Ok(format!(
                r#"{{"schemaName":"tdd","changeDir":{:?},"contextFiles":{{}}}}"#,
                base_root
                    .join("openspec/changes/alpha")
                    .display()
                    .to_string()
            )),
        );
        let cli: Arc<dyn OpenspecCli> = openspec_fake;
        let base_str = base_root.display().to_string();
        let git_fake = Arc::new(crate::cli::FakeCli::new());
        git_fake.register_git(
            &git_args(&base_str, &["worktree", "list", "--porcelain", "-z"]),
            Err(crate::cli::CliError::NotStarted {
                program: "git".to_string(),
                args: vec!["worktree".to_string(), "list".to_string()],
                reason: "not found".to_string(),
            }),
        );
        let git: Arc<dyn GitCli> = git_fake;

        let (mut refresher, results_rx, _exit_rx) =
            worker_for_test(base_root, cli, git, WORKTREE_RECHECK);
        refresher.request(Selection::All, ArchivedScope::Names);
        let _files = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("files");
        let merged = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("merged");
        match merged {
            RefreshResult::Merged(set) => {
                assert!(set.worktrees.is_empty());
                assert!(
                    !set.problems
                        .iter()
                        .any(|p| p.to_lowercase().contains("git")),
                    "{:?}",
                    set.problems
                );
            }
            other => panic!("expected Merged, got {other:?}"),
        }
        let _ = base_scratch;
    }

    /// `worktree-overlay` -> "Not a git repository, a timeout, or no record for the
    /// root".
    #[test]
    fn not_a_git_repository_a_timeout_or_no_record_for_the_root() {
        fn assert_unoverlaid(response: Result<String, crate::cli::CliError>) {
            let (base_scratch, base_root) = scratch_root();
            let openspec_fake = Arc::new(crate::cli::FakeCli::new());
            openspec_fake.register_openspec(
                &["list", "--json"],
                Ok(format!(
                    r#"{{"changes":[],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                    base_root.display().to_string()
                )),
            );
            let cli: Arc<dyn OpenspecCli> = openspec_fake;
            let base_str = base_root.display().to_string();
            let git_fake = Arc::new(crate::cli::FakeCli::new());
            git_fake.register_git(
                &git_args(&base_str, &["worktree", "list", "--porcelain", "-z"]),
                response,
            );
            let git: Arc<dyn GitCli> = git_fake;

            let (mut refresher, results_rx, _exit_rx) =
                worker_for_test(base_root, cli, git, WORKTREE_RECHECK);
            refresher.request(Selection::All, ArchivedScope::Names);
            let _files = results_rx
                .recv_timeout(Duration::from_secs(10))
                .expect("files");
            let merged = results_rx
                .recv_timeout(Duration::from_secs(10))
                .expect("merged");
            match merged {
                RefreshResult::Merged(set) => {
                    assert!(set.worktrees.is_empty());
                    assert!(set.problems.is_empty(), "{:?}", set.problems);
                }
                other => panic!("expected Merged, got {other:?}"),
            }
            let _ = base_scratch;
        }

        assert_unoverlaid(Err(crate::cli::CliError::Failed {
            program: "git".to_string(),
            args: vec!["worktree".to_string(), "list".to_string()],
            code: Some(128),
            stderr: "fatal: not a git repository".to_string(),
        }));
        assert_unoverlaid(Err(crate::cli::CliError::TimedOut {
            args: vec!["worktree".to_string(), "list".to_string()],
            after: crate::cli::RUN_DEADLINE,
        }));

        let (elsewhere_scratch, elsewhere_root) = scratch_root();
        assert_unoverlaid(Ok(wt_record(&elsewhere_root, "zzzz", "somewhere")));
        let _ = elsewhere_scratch;
    }

    /// `worktree-overlay` -> "A git too old for the listing is named once".
    #[test]
    fn a_git_too_old_for_the_listing_is_named_once() {
        let (base_scratch, base_root) = scratch_root();
        let openspec_fake = Arc::new(crate::cli::FakeCli::new());
        openspec_fake.register_openspec(
            &["list", "--json"],
            Ok(format!(
                r#"{{"changes":[],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                base_root.display().to_string()
            )),
        );
        let cli: Arc<dyn OpenspecCli> = openspec_fake;
        let base_str = base_root.display().to_string();
        let git_fake = Arc::new(crate::cli::FakeCli::new());
        git_fake.register_git(
            &git_args(&base_str, &["worktree", "list", "--porcelain", "-z"]),
            Err(crate::cli::CliError::Failed {
                program: "git".to_string(),
                args: vec![
                    "worktree".to_string(),
                    "list".to_string(),
                    "--porcelain".to_string(),
                    "-z".to_string(),
                ],
                code: Some(129),
                stderr: "error: unknown switch `z'".to_string(),
            }),
        );
        let git: Arc<dyn GitCli> = git_fake;

        let (mut refresher, results_rx, _exit_rx) =
            worker_for_test(base_root, cli, git, WORKTREE_RECHECK);
        refresher.request(Selection::All, ArchivedScope::Names);
        let _files = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("files");
        let merged = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("merged");
        match merged {
            RefreshResult::Merged(set) => {
                assert!(set.worktrees.is_empty());
                assert_eq!(set.problems.len(), 1, "{:?}", set.problems);
                assert!(set.problems[0].contains("worktree list"));
                assert!(set.problems[0].to_lowercase().contains("not shown"));
            }
            other => panic!("expected Merged, got {other:?}"),
        }
        let _ = base_scratch;
    }

    /// `worktree-overlay` -> "One member's query fails and the other still overlays".
    #[test]
    fn a_failing_member_contributes_nothing_and_is_named() {
        let (base_scratch, base_root) = scratch_root();
        let (feat_scratch, feat_root) = scratch_root();
        crate::testutil::write_with_mode(
            &feat_root.join("openspec/changes/x/tasks.md"),
            b"- [ ] a\n",
            0o644,
        );
        let (broken_scratch, broken_root) = scratch_root();

        let openspec_fake = Arc::new(crate::cli::FakeCli::new());
        openspec_fake.register_openspec(
            &["list", "--json"],
            Ok(format!(
                r#"{{"changes":[],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                base_root.display().to_string()
            )),
        );
        let cli: Arc<dyn OpenspecCli> = openspec_fake;

        let base_str = base_root.display().to_string();
        let feat_str = feat_root.display().to_string();
        let broken_str = broken_root.display().to_string();
        let git_fake = Arc::new(crate::cli::FakeCli::new());
        git_fake.register_git(
            &git_args(&base_str, &["worktree", "list", "--porcelain", "-z"]),
            Ok(format!(
                "{}{}{}",
                wt_record(&base_root, "aaaa", "main"),
                wt_record(&feat_root, "bbbb", "feat"),
                wt_record(&broken_root, "cccc", "broken"),
            )),
        );
        git_fake.register_git(
            &git_args(&feat_str, &["merge-base", "HEAD", "aaaa"]),
            Ok("fbase".to_string()),
        );
        git_fake.register_git(
            &git_args(
                &feat_str,
                &[
                    "diff-tree",
                    "-r",
                    "--name-only",
                    "-z",
                    "--no-renames",
                    "fbase",
                    "HEAD",
                    "--",
                    "openspec/changes",
                ],
            ),
            Ok(String::new()),
        );
        git_fake.register_git(
            &git_args(
                &feat_str,
                &[
                    "status",
                    "--porcelain=v1",
                    "-z",
                    "--no-renames",
                    "--untracked-files=all",
                    "--",
                    "openspec/changes",
                ],
            ),
            Ok("?? openspec/changes/x/tasks.md\0".to_string()),
        );
        git_fake.register_git(
            &git_args(&broken_str, &["merge-base", "HEAD", "aaaa"]),
            Err(crate::cli::CliError::Failed {
                program: "git".to_string(),
                args: vec![
                    "merge-base".to_string(),
                    "HEAD".to_string(),
                    "aaaa".to_string(),
                ],
                code: Some(128),
                stderr: "fatal: bad object aaaa".to_string(),
            }),
        );
        let git: Arc<dyn GitCli> = git_fake;

        let (mut refresher, results_rx, _exit_rx) =
            worker_for_test(base_root, cli, git, WORKTREE_RECHECK);
        refresher.request(Selection::All, ArchivedScope::Names);
        let _files = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("files");
        let merged = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("merged");
        match merged {
            RefreshResult::Merged(set) => {
                let x = set.active.iter().find(|c| c.name == "x").unwrap();
                assert!(x.dir.starts_with(&feat_root));
                assert_eq!(set.worktrees.len(), 2);
                assert_eq!(set.problems.len(), 1, "{:?}", set.problems);
                assert!(set.problems[0].contains(&broken_str));
                assert!(set.problems[0].contains("merge-base"));
            }
            other => panic!("expected Merged, got {other:?}"),
        }
        let _ = feat_scratch;
        let _ = broken_scratch;
        let _ = base_scratch;
    }

    /// `worktree-overlay` -> "A member with unrelated history owns nothing and is not
    /// a problem".
    #[test]
    fn a_member_with_unrelated_history_owns_nothing_and_is_not_a_problem() {
        let (base_scratch, base_root) = scratch_root();
        let (orphan_scratch, orphan_root) = scratch_root();
        let (broken_scratch, broken_root) = scratch_root();

        let openspec_fake = Arc::new(crate::cli::FakeCli::new());
        openspec_fake.register_openspec(
            &["list", "--json"],
            Ok(format!(
                r#"{{"changes":[],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                base_root.display().to_string()
            )),
        );
        let cli: Arc<dyn OpenspecCli> = openspec_fake;

        let base_str = base_root.display().to_string();
        let orphan_str = orphan_root.display().to_string();
        let broken_str = broken_root.display().to_string();
        let git_fake = Arc::new(crate::cli::FakeCli::new());
        git_fake.register_git(
            &git_args(&base_str, &["worktree", "list", "--porcelain", "-z"]),
            Ok(format!(
                "{}{}{}",
                wt_record(&base_root, "aaaa", "main"),
                wt_record(&orphan_root, "dddd", "gh-pages"),
                wt_record(&broken_root, "cccc", "broken"),
            )),
        );
        git_fake.register_git(
            &git_args(&orphan_str, &["merge-base", "HEAD", "aaaa"]),
            Err(crate::cli::CliError::Failed {
                program: "git".to_string(),
                args: vec![
                    "merge-base".to_string(),
                    "HEAD".to_string(),
                    "aaaa".to_string(),
                ],
                code: Some(1),
                stderr: String::new(),
            }),
        );
        git_fake.register_git(
            &git_args(&broken_str, &["merge-base", "HEAD", "aaaa"]),
            Err(crate::cli::CliError::Failed {
                program: "git".to_string(),
                args: vec![
                    "merge-base".to_string(),
                    "HEAD".to_string(),
                    "aaaa".to_string(),
                ],
                code: Some(128),
                stderr: "fatal: bad object aaaa".to_string(),
            }),
        );
        let git: Arc<dyn GitCli> = git_fake.clone();

        let (mut refresher, results_rx, _exit_rx) =
            worker_for_test(base_root, cli, git, WORKTREE_RECHECK);
        refresher.request(Selection::All, ArchivedScope::Names);
        let _files = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("files");
        let merged = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("merged");
        match merged {
            RefreshResult::Merged(set) => {
                assert_eq!(set.worktrees.len(), 2);
                assert_eq!(set.problems.len(), 1, "{:?}", set.problems);
                assert!(set.problems[0].contains(&broken_str));
                assert!(!set.problems[0].contains(&orphan_str));
            }
            other => panic!("expected Merged, got {other:?}"),
        }

        let diff_or_status_for_orphan = git_fake.calls().into_iter().any(|(program, args)| {
            program == crate::cli::Program::Git
                && args.iter().any(|a| a == &orphan_str)
                && (args.contains(&"diff-tree".to_string()) || args.contains(&"status".to_string()))
        });
        assert!(
            !diff_or_status_for_orphan,
            "no diff-tree/status call should be recorded for the unrelated-history member"
        );
        let _ = orphan_scratch;
        let _ = broken_scratch;
        let _ = base_scratch;
    }

    /// `worktree-overlay` -> "A prunable record and an unresolvable path record no
    /// problem through the worker".
    #[test]
    fn a_prunable_record_and_an_unresolvable_path_record_no_problem() {
        let (base_scratch, base_root) = scratch_root();
        let (live_scratch, live_root) = scratch_root();
        crate::testutil::write_with_mode(
            &live_root.join("openspec/changes/x/tasks.md"),
            b"- [ ] a\n",
            0o644,
        );

        let openspec_fake = Arc::new(crate::cli::FakeCli::new());
        openspec_fake.register_openspec(
            &["list", "--json"],
            Ok(format!(
                r#"{{"changes":[],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                base_root.display().to_string()
            )),
        );
        let cli: Arc<dyn OpenspecCli> = openspec_fake;

        let base_str = base_root.display().to_string();
        let live_str = live_root.display().to_string();
        let gone_path = base_scratch.path().join("gone-worktree");
        let prunable_path = base_scratch.path().join("prunable-worktree");
        let git_fake = Arc::new(crate::cli::FakeCli::new());
        git_fake.register_git(
            &git_args(&base_str, &["worktree", "list", "--porcelain", "-z"]),
            Ok(format!(
                "worktree {}\0HEAD aaaa\0branch refs/heads/main\0\0\
worktree {}\0HEAD bbbb\0branch refs/heads/feat\0\0\
worktree {}\0HEAD dddd\0detached\0prunable gitdir file points to non-existent location\0\0\
worktree {}\0HEAD eeee\0detached\0\0",
                base_root.display(),
                live_root.display(),
                prunable_path.display(),
                gone_path.display(),
            )),
        );
        git_fake.register_git(
            &git_args(&live_str, &["merge-base", "HEAD", "aaaa"]),
            Ok("lbase".to_string()),
        );
        git_fake.register_git(
            &git_args(
                &live_str,
                &[
                    "diff-tree",
                    "-r",
                    "--name-only",
                    "-z",
                    "--no-renames",
                    "lbase",
                    "HEAD",
                    "--",
                    "openspec/changes",
                ],
            ),
            Ok(String::new()),
        );
        git_fake.register_git(
            &git_args(
                &live_str,
                &[
                    "status",
                    "--porcelain=v1",
                    "-z",
                    "--no-renames",
                    "--untracked-files=all",
                    "--",
                    "openspec/changes",
                ],
            ),
            Ok("?? openspec/changes/x/tasks.md\0".to_string()),
        );
        let git: Arc<dyn GitCli> = git_fake;

        let (mut refresher, results_rx, _exit_rx) =
            worker_for_test(base_root, cli, git, WORKTREE_RECHECK);
        refresher.request(Selection::All, ArchivedScope::Names);
        let _files = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("files");
        let merged = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("merged");
        match merged {
            RefreshResult::Merged(set) => {
                assert_eq!(set.worktrees.len(), 1);
                assert_eq!(set.worktrees[0].label, "feat");
                assert!(set.problems.is_empty(), "{:?}", set.problems);
            }
            other => panic!("expected Merged, got {other:?}"),
        }
        let _ = live_scratch;
        let _ = base_scratch;
    }

    /// `worktree-overlay` -> "A member with no OpenSpec tree owns nothing".
    #[test]
    fn a_member_with_no_openspec_tree_owns_nothing() {
        let (base_scratch, base_root) = scratch_root();
        crate::testutil::write_with_mode(
            &base_root.join("openspec/changes/x/tasks.md"),
            b"- [ ] a\n",
            0o644,
        );
        let bare_scratch = crate::testutil::ScratchDir::new();
        let bare_root = crate::testutil::canonical(bare_scratch.path());
        // No `openspec/` tree at all under `bare_root`.

        let openspec_fake = Arc::new(crate::cli::FakeCli::new());
        openspec_fake.register_openspec(
            &["list", "--json"],
            Ok(format!(
                r#"{{"changes":[],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                base_root.display().to_string()
            )),
        );
        let cli: Arc<dyn OpenspecCli> = openspec_fake;

        let base_str = base_root.display().to_string();
        let bare_str = bare_root.display().to_string();
        let git_fake = Arc::new(crate::cli::FakeCli::new());
        git_fake.register_git(
            &git_args(&base_str, &["worktree", "list", "--porcelain", "-z"]),
            Ok(format!(
                "{}{}",
                wt_record(&base_root, "aaaa", "main"),
                wt_record(&bare_root, "bbbb", "empty"),
            )),
        );
        git_fake.register_git(
            &git_args(&bare_str, &["merge-base", "HEAD", "aaaa"]),
            Ok("mbase".to_string()),
        );
        git_fake.register_git(
            &git_args(
                &bare_str,
                &[
                    "diff-tree",
                    "-r",
                    "--name-only",
                    "-z",
                    "--no-renames",
                    "mbase",
                    "HEAD",
                    "--",
                    "openspec/changes",
                ],
            ),
            Ok(String::new()),
        );
        git_fake.register_git(
            &git_args(
                &bare_str,
                &[
                    "status",
                    "--porcelain=v1",
                    "-z",
                    "--no-renames",
                    "--untracked-files=all",
                    "--",
                    "openspec/changes",
                ],
            ),
            Ok(String::new()),
        );
        let git: Arc<dyn GitCli> = git_fake;

        let (mut refresher, results_rx, _exit_rx) =
            worker_for_test(base_root, cli, git, WORKTREE_RECHECK);
        refresher.request(Selection::All, ArchivedScope::Names);
        let files = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("files");
        let base_active_names: Vec<String> = match files {
            RefreshResult::Files(set) => set.active.iter().map(|c| c.name.clone()).collect(),
            other => panic!("expected Files, got {other:?}"),
        };
        let merged = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("merged");
        match merged {
            RefreshResult::Merged(set) => {
                let names: Vec<String> = set.active.iter().map(|c| c.name.clone()).collect();
                assert_eq!(
                    names, base_active_names,
                    "the empty member must own nothing"
                );
                assert_eq!(set.worktrees.len(), 1);
                assert!(set.problems.is_empty(), "{:?}", set.problems);
            }
            other => panic!("expected Merged, got {other:?}"),
        }
        let _ = base_scratch;
        let _ = bare_scratch;
    }

    /// `worktree-overlay` -> "Only the four commands are run".
    #[test]
    fn only_the_four_commands_are_run() {
        let (base_scratch, base_root) = scratch_root();
        let (feat_scratch, feat_root) = scratch_root();
        crate::testutil::write_with_mode(
            &feat_root.join("openspec/changes/x/tasks.md"),
            b"- [ ] a\n",
            0o644,
        );
        let (fix_scratch, fix_root) = scratch_root();
        crate::testutil::write_with_mode(
            &fix_root.join("openspec/changes/y/tasks.md"),
            b"- [ ] a\n",
            0o644,
        );

        let openspec_fake = Arc::new(crate::cli::FakeCli::new());
        openspec_fake.register_openspec(
            &["list", "--json"],
            Ok(format!(
                r#"{{"changes":[],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                base_root.display().to_string()
            )),
        );
        let cli: Arc<dyn OpenspecCli> = openspec_fake;

        let base_str = base_root.display().to_string();
        let feat_str = feat_root.display().to_string();
        let fix_str = fix_root.display().to_string();
        let git_fake = Arc::new(crate::cli::FakeCli::new());
        git_fake.register_git(
            &git_args(&base_str, &["worktree", "list", "--porcelain", "-z"]),
            Ok(format!(
                "{}{}{}",
                wt_record(&base_root, "aaaa", "main"),
                wt_record(&feat_root, "bbbb", "feat"),
                wt_record(&fix_root, "cccc", "fix"),
            )),
        );
        for (root_str, sha) in [(&feat_str, "bbbb"), (&fix_str, "cccc")] {
            git_fake.register_git(
                &git_args(root_str, &["merge-base", "HEAD", "aaaa"]),
                Ok(format!("base-{sha}")),
            );
            git_fake.register_git(
                &git_args(
                    root_str,
                    &[
                        "diff-tree",
                        "-r",
                        "--name-only",
                        "-z",
                        "--no-renames",
                        &format!("base-{sha}"),
                        "HEAD",
                        "--",
                        "openspec/changes",
                    ],
                ),
                Ok(String::new()),
            );
            git_fake.register_git(
                &git_args(
                    root_str,
                    &[
                        "status",
                        "--porcelain=v1",
                        "-z",
                        "--no-renames",
                        "--untracked-files=all",
                        "--",
                        "openspec/changes",
                    ],
                ),
                Ok(String::new()),
            );
        }
        let git: Arc<dyn GitCli> = git_fake.clone();

        let (mut refresher, results_rx, _exit_rx) =
            worker_for_test(base_root, cli, git, WORKTREE_RECHECK);
        refresher.request(Selection::All, ArchivedScope::Names);
        let _files = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("files");
        let _merged = results_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("merged");

        let git_calls: Vec<Vec<String>> = git_fake
            .calls()
            .into_iter()
            .filter(|(program, _)| *program == crate::cli::Program::Git)
            .map(|(_, args)| args)
            .collect();
        assert_eq!(
            git_calls.len(),
            7,
            "1 worktree list + 3 per member x 2 members"
        );
        for args in &git_calls {
            assert_eq!(args[0], "--no-optional-locks");
            assert_eq!(args[1], "-c");
            assert_eq!(args[2], "core.fsmonitor=false");
            assert_eq!(args[3], "-C");
            assert!(
                matches!(
                    args[5].as_str(),
                    "worktree" | "merge-base" | "diff-tree" | "status"
                ),
                "{args:?}"
            );
        }
        let _ = base_scratch;
        let _ = feat_scratch;
        let _ = fix_scratch;
    }

    // --- group 8.2: real-git tests, driving the real `git` binary through `GitCli` ---

    /// Refuses to run a real-git test if the process environment already names
    /// `GIT_DIR`, `GIT_INDEX_FILE`, or `GIT_WORK_TREE` — design.md -> Test Boundaries:
    /// these would redirect a scratch `-C`-qualified command at whatever repository
    /// they name rather than at the scratch trees this test builds.
    fn assert_no_git_env_leak() {
        for var in ["GIT_DIR", "GIT_INDEX_FILE", "GIT_WORK_TREE"] {
            assert!(
                std::env::var_os(var).is_none(),
                "{var} is set in the test environment; refusing to run a real-git test \
                 rather than risk touching the real repository"
            );
        }
    }

    fn real_git() -> Arc<dyn GitCli> {
        crate::cli::git_cli_via(Path::new(crate::cli::GIT_PROGRAM))
    }

    /// Run a real `git` setup command in `dir` with a scratch identity and every
    /// setting design.md -> Test Boundaries names, panicking on failure — test-only
    /// scaffolding reached only through the crate's own `GitCli` seam, never a spawn
    /// API named directly (`NOSPAWN-GREP` scans this file's test module too).
    fn real_git_setup(git: &dyn GitCli, dir: &Path, args: &[&str]) {
        let dir_str = dir.to_string_lossy().into_owned();
        let mut full: Vec<&str> = vec![
            "-C",
            &dir_str,
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "init.defaultBranch=main",
            "-c",
            "user.email=scratch@example.com",
            "-c",
            "user.name=scratch",
        ];
        full.extend_from_slice(args);
        git.run(&full)
            .unwrap_or_else(|e| panic!("git {full:?} failed in {}: {e:?}", dir.display()));
    }

    /// `worktree-overlay` -> "A full cycle over a real repository and worktree leaves
    /// git's files untouched".
    #[test]
    fn a_full_cycle_over_a_real_repository_and_worktree_leaves_gits_files_untouched() {
        assert_no_git_env_leak();
        let git = real_git();

        let scratch = crate::testutil::ScratchDir::new();
        let base_root = scratch.path().join("base");
        let worktree_root = scratch.path().join("feat");
        std::fs::create_dir_all(&base_root).expect("create base dir");

        real_git_setup(git.as_ref(), &base_root, &["init", "-q", "-b", "main"]);
        real_git_setup(
            git.as_ref(),
            &base_root,
            &["config", "core.fsmonitor", "false"],
        );
        real_git_setup(git.as_ref(), &base_root, &["config", "gc.auto", "0"]);
        real_git_setup(
            git.as_ref(),
            &base_root,
            &["config", "maintenance.auto", "false"],
        );

        vendor_tdd_schema(&base_root);
        crate::testutil::write_with_mode(
            &base_root.join("openspec/changes/y/tasks.md"),
            b"- [ ] a\n",
            0o644,
        );
        real_git_setup(git.as_ref(), &base_root, &["add", "-A"]);
        real_git_setup(git.as_ref(), &base_root, &["commit", "-q", "-m", "initial"]);

        let worktree_str = worktree_root.to_string_lossy().into_owned();
        real_git_setup(
            git.as_ref(),
            &base_root,
            &["worktree", "add", "-q", "-b", "feat", &worktree_str],
        );

        // An uncommitted edit under the worktree's own `openspec/changes/x/`, and a
        // file whose timestamp is moved without changing its bytes — design.md ->
        // Test Boundaries' own two measured hazards.
        crate::testutil::write_with_mode(
            &worktree_root.join("openspec/changes/x/tasks.md"),
            b"- [x] a\n- [ ] b\n",
            0o644,
        );
        let y_path = worktree_root.join("openspec/changes/y/tasks.md");
        let current = std::fs::metadata(&y_path)
            .expect("stat committed y/tasks.md")
            .modified()
            .expect("modified time");
        let advanced = current + std::time::Duration::from_secs(120);
        let file = std::fs::File::options()
            .write(true)
            .open(&y_path)
            .expect("open y/tasks.md for touching");
        file.set_modified(advanced)
            .expect("move y/tasks.md's mtime without touching its bytes");
        drop(file);

        let base_canonical = crate::testutil::canonical(&base_root);
        let worktree_canonical = crate::testutil::canonical(&worktree_root);
        let before_base = crate::testutil::snapshot(&base_canonical);
        let before_worktree = crate::testutil::snapshot(&worktree_canonical);

        let openspec_fake = Arc::new(crate::cli::FakeCli::new());
        openspec_fake.register_openspec(
            &["list", "--json"],
            Ok(format!(
                r#"{{"changes":[{{"name":"y","completedTasks":0,"totalTasks":1,"lastModified":"x","status":"y"}}],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                base_canonical.display().to_string()
            )),
        );
        openspec_fake.register_openspec(
            &["instructions", "apply", "--change", "y", "--json"],
            Ok(format!(
                r#"{{"schemaName":"tdd","changeDir":{:?},"contextFiles":{{}}}}"#,
                base_canonical
                    .join("openspec/changes/y")
                    .display()
                    .to_string()
            )),
        );
        let cli: Arc<dyn OpenspecCli> = openspec_fake;

        let (mut refresher, results_rx, _exit_rx) = worker_for_test(
            base_canonical.clone(),
            cli,
            git.clone(),
            Duration::from_millis(5),
        );
        refresher.request(Selection::All, ArchivedScope::Names);

        let _files = results_rx
            .recv_timeout(Duration::from_secs(30))
            .expect("files");
        let merged = results_rx
            .recv_timeout(Duration::from_secs(30))
            .expect("merged");
        match merged {
            RefreshResult::Merged(set) => {
                let x = set.active.iter().find(|c| c.name == "x").unwrap();
                assert!(x.dir.starts_with(&worktree_canonical), "{:?}", x.dir);
                assert_eq!(set.worktrees.len(), 1);
            }
            other => panic!("expected Merged, got {other:?}"),
        }

        // No further request: wait long enough for several idle re-checks (5ms each)
        // to run with nothing changing. A silent re-check leaves nothing on the
        // channel to receive by design (`Files` is sent only when the overlay
        // changes), so a `recv_timeout` on the result channel — never a sleep — is
        // both the wait and the proof that nothing was written or corrupted while it
        // ran, the same technique `an_unchanged_overlay_sends_nothing` uses.
        match results_rx.recv_timeout(Duration::from_millis(200)) {
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            other => panic!("expected no further result, got {other:?}"),
        }

        let after_base = crate::testutil::snapshot(&base_canonical);
        let after_worktree = crate::testutil::snapshot(&worktree_canonical);
        assert_eq!(
            before_base, after_base,
            "the base's own files or git directory changed"
        );
        assert_eq!(
            before_worktree, after_worktree,
            "the worktree's own files changed"
        );
    }

    /// `worktree-overlay` -> "A member owns exactly the changes it touched since it
    /// forked from the base", over a real repository whose pane root sits inside a
    /// subdirectory (`sub`) of its checkout's top level: real `diff-tree`/`status`
    /// output names paths relative to the top level (`sub/openspec/changes/x/...`),
    /// so a `derive_family` that ran `-C` at the OpenSpec root or matched against the
    /// bare `openspec/changes` pathspec would find `x` untouched.
    #[test]
    fn a_real_cycle_finds_a_members_change_when_the_panes_openspec_root_is_nested() {
        assert_no_git_env_leak();
        let git = real_git();

        let scratch = crate::testutil::ScratchDir::new();
        let base_top = scratch.path().join("base");
        let worktree_top = scratch.path().join("feat");
        std::fs::create_dir_all(&base_top).expect("create base dir");

        real_git_setup(git.as_ref(), &base_top, &["init", "-q", "-b", "main"]);
        real_git_setup(
            git.as_ref(),
            &base_top,
            &["config", "core.fsmonitor", "false"],
        );
        real_git_setup(git.as_ref(), &base_top, &["config", "gc.auto", "0"]);
        real_git_setup(
            git.as_ref(),
            &base_top,
            &["config", "maintenance.auto", "false"],
        );

        vendor_tdd_schema(&base_top.join("sub"));
        crate::testutil::write_with_mode(
            &base_top.join("sub/openspec/changes/y/tasks.md"),
            b"- [ ] a\n",
            0o644,
        );
        real_git_setup(git.as_ref(), &base_top, &["add", "-A"]);
        real_git_setup(git.as_ref(), &base_top, &["commit", "-q", "-m", "initial"]);

        let worktree_str = worktree_top.to_string_lossy().into_owned();
        real_git_setup(
            git.as_ref(),
            &base_top,
            &["worktree", "add", "-q", "-b", "feat", &worktree_str],
        );

        // An uncommitted change under the worktree's own nested `sub/openspec/changes/x/`.
        crate::testutil::write_with_mode(
            &worktree_top.join("sub/openspec/changes/x/tasks.md"),
            b"- [x] a\n",
            0o644,
        );

        let base_canonical = crate::testutil::canonical(&base_top);
        let base_root = base_canonical.join("sub");
        let worktree_canonical = crate::testutil::canonical(&worktree_top);
        let member_root = worktree_canonical.join("sub");

        let openspec_fake = Arc::new(crate::cli::FakeCli::new());
        openspec_fake.register_openspec(
            &["list", "--json"],
            Ok(format!(
                r#"{{"changes":[],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                base_root.display().to_string()
            )),
        );
        let cli: Arc<dyn OpenspecCli> = openspec_fake;

        let (mut refresher, results_rx, _exit_rx) = worker_for_test(
            base_root.clone(),
            cli,
            git.clone(),
            Duration::from_millis(5),
        );
        refresher.request(Selection::All, ArchivedScope::Names);

        let _files = results_rx
            .recv_timeout(Duration::from_secs(30))
            .expect("files");
        let merged = results_rx
            .recv_timeout(Duration::from_secs(30))
            .expect("merged");
        match merged {
            RefreshResult::Merged(set) => {
                assert!(set.problems.is_empty(), "{:?}", set.problems);
                let x = set
                    .active
                    .iter()
                    .find(|c| c.name == "x")
                    .unwrap_or_else(|| panic!("x missing from active: {:?}", set.active));
                assert!(x.dir.starts_with(&member_root), "{:?}", x.dir);
                assert_eq!(set.worktrees.len(), 1);
            }
            other => panic!("expected Merged, got {other:?}"),
        }
    }

    /// `worktree-overlay` -> "A worktree that forked before the base moved on owns
    /// nothing it did not touch".
    #[test]
    fn a_worktree_that_forked_before_the_base_moved_on_owns_nothing_it_did_not_touch() {
        assert_no_git_env_leak();
        let git = real_git();

        let scratch = crate::testutil::ScratchDir::new();
        let base_root = scratch.path().join("base");
        let worktree_root = scratch.path().join("feat");
        std::fs::create_dir_all(&base_root).expect("create base dir");

        real_git_setup(git.as_ref(), &base_root, &["init", "-q", "-b", "main"]);
        real_git_setup(
            git.as_ref(),
            &base_root,
            &["config", "core.fsmonitor", "false"],
        );
        real_git_setup(git.as_ref(), &base_root, &["config", "gc.auto", "0"]);
        real_git_setup(
            git.as_ref(),
            &base_root,
            &["config", "maintenance.auto", "false"],
        );

        vendor_tdd_schema(&base_root);
        crate::testutil::write_with_mode(
            &base_root.join("openspec/changes/x/tasks.md"),
            b"- [ ] a\n",
            0o644,
        );
        crate::testutil::write_with_mode(
            &base_root.join("openspec/changes/y/tasks.md"),
            b"- [ ] a\n",
            0o644,
        );
        real_git_setup(git.as_ref(), &base_root, &["add", "-A"]);
        real_git_setup(git.as_ref(), &base_root, &["commit", "-q", "-m", "initial"]);

        let worktree_str = worktree_root.to_string_lossy().into_owned();
        real_git_setup(
            git.as_ref(),
            &base_root,
            &["worktree", "add", "-q", "-b", "feat", &worktree_str],
        );

        // The base moves on after the fork: `x` is edited and `y` is archived, both
        // committed — the member touches neither.
        crate::testutil::write_with_mode(
            &base_root.join("openspec/changes/x/tasks.md"),
            b"- [x] a\n",
            0o644,
        );
        std::fs::create_dir_all(base_root.join("openspec/changes/archive"))
            .expect("create archive dir");
        std::fs::rename(
            base_root.join("openspec/changes/y"),
            base_root.join("openspec/changes/archive/2026-09-24-y"),
        )
        .expect("archive y");
        real_git_setup(git.as_ref(), &base_root, &["add", "-A"]);
        real_git_setup(
            git.as_ref(),
            &base_root,
            &["commit", "-q", "-m", "edit x, archive y"],
        );

        let base_canonical = crate::testutil::canonical(&base_root);
        let worktree_canonical = crate::testutil::canonical(&worktree_root);

        let openspec_fake = Arc::new(crate::cli::FakeCli::new());
        openspec_fake.register_openspec(
            &["list", "--json"],
            Ok(format!(
                r#"{{"changes":[{{"name":"x","completedTasks":1,"totalTasks":1,"lastModified":"x","status":"y"}}],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                base_canonical.display().to_string()
            )),
        );
        openspec_fake.register_openspec(
            &["instructions", "apply", "--change", "x", "--json"],
            Ok(format!(
                r#"{{"schemaName":"tdd","changeDir":{:?},"contextFiles":{{}}}}"#,
                base_canonical
                    .join("openspec/changes/x")
                    .display()
                    .to_string()
            )),
        );
        let cli: Arc<dyn OpenspecCli> = openspec_fake;

        let (mut refresher, results_rx, _exit_rx) =
            worker_for_test(base_canonical.clone(), cli, git, WORKTREE_RECHECK);
        refresher.request(Selection::All, ArchivedScope::Full);

        let _files = results_rx
            .recv_timeout(Duration::from_secs(30))
            .expect("files");
        let merged = results_rx
            .recv_timeout(Duration::from_secs(30))
            .expect("merged");
        match merged {
            RefreshResult::Merged(set) => {
                assert_eq!(set.worktrees.len(), 1);
                let x = set.active.iter().find(|c| c.name == "x").unwrap();
                assert!(
                    x.dir.starts_with(&base_canonical) && !x.dir.starts_with(&worktree_canonical),
                    "the member forked before the edit and must not own x: {:?}",
                    x.dir
                );
                assert_eq!(x.progress.completed, 1);
                assert!(set.active.iter().all(|c| c.name != "y"));
                assert!(set.archived.iter().any(|c| c.name == "y"));
                assert!(set.problems.is_empty(), "{:?}", set.problems);
            }
            other => panic!("expected Merged, got {other:?}"),
        }
    }
}
