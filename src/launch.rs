//! The launcher: the crate's third worker thread, and the only module besides `src/cli.rs`,
//! `src/agents.rs`, and `src/ui/mod.rs` that reaches the `herdr` program — through
//! `Arc<dyn crate::cli::HerdrCli>` and nothing else. `src/launch.rs` spawns no process of its
//! own; `src/cli.rs` remains the crate's single spawn site.
//!
//! Confined to this one module, on exactly `src/watch.rs`'s, `src/refresh.rs`'s, and
//! `src/agents.rs`'s single-file terms — see `openspec/changes/agent-launch/design.md` ->
//! Boundaries. `decide` is the whole launch policy, a pure function reachable with no
//! `Dashboard`, no `ChangeSet`, and no fixture. `herdr agent start` blocks for up to thirty
//! seconds waiting for interactive readiness, which is why the launcher cannot be a synchronous
//! call from the render path and must be a worker thread instead.
//!
//! Group 1 populates only the inert halves: the types, a `decide` returning
//! `Decision::Nothing`, a `pane_id` returning `Err`, and a worker that answers nothing. The
//! real policy arrives in group 3, the real Herdr calls in group 4, and the real seam in
//! group 5.

/// Which `/opsx:*` command a launch sends, or that `g` is a focus rather than a launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Apply,
    Continue,
    Archive,
    Focus,
}

/// What the launcher's worker is asked to do: start an agent on a change, or focus one
/// already running. Plain data — no trait, no channel, no handle — so it stays `Clone`,
/// `PartialEq`, and comparable in a test with no thread. See `agents::attribute`'s `Attribution`
/// for the same shape of plain-data contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Launch {
        change: String,
        agent: String,
        intent: Intent,
    },
    Focus {
        pane_id: String,
    },
}

/// `decide`'s answer: do nothing, refuse with a reason, or go ahead with a `Request`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Nothing,
    Refuse(String),
    Go(Request),
}

/// The launcher's answer to one request, once it has one. `named` carries the
/// `(derived agent name, change name)` pair exactly when an agent was started, so the loop can
/// keep `Dashboard::agent_names` current without re-reading the file; `problem` carries the
/// failure, and is `None` on complete success. Never `Default`, anywhere in the crate; every
/// construction and destructuring names both fields, with no `..` rest — on exactly
/// `agents::AgentSnapshot`'s and `agents::Attribution`'s terms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub named: Option<(String, String)>,
    pub problem: Option<String>,
}

/// The whole launch policy, a pure total function of its five arguments. Performs no
/// filesystem, process, environment, network, or terminal I/O, reads no clock and no global
/// state, spawns nothing, and never panics for any combination of arguments. See
/// `specs/agent-launch/spec.md` -> "The launch decision is a pure, total function that refuses
/// before it reaches Herdr" for the order group 3 fills in.
pub fn decide(
    _intent: Intent,
    _change: Option<&str>,
    _pane: Option<&str>,
    _reachable: bool,
    _live_names: &[&str],
) -> Decision {
    Decision::Nothing
}

/// Navigate Herdr's `pane split` envelope to `result.pane.pane_id`, following
/// `agents::parse_list`'s style. Pure, and never panics for any input, including the empty
/// string. Inert in this group: always `Err`, with an empty reason. See
/// `specs/agent-launch/spec.md` -> "The pane id is read from `pane split`'s envelope, and an
/// unusable payload stops the launch".
pub fn pane_id(_text: &str) -> Result<String, String> {
    Err(String::new())
}

/// A non-blocking source of launch outcomes. Every method SHALL be non-blocking, on exactly
/// `watch::FsEvents`'s, `refresh::Refresher`'s, and `agents::AgentPoll`'s terms: the render path
/// calls both on every iteration and neither may wait on anything. Carries no `pending_in`: the
/// launcher has no schedule of its own — it acts only when a key is pressed — so it contributes
/// nothing to the loop's wake-up.
pub trait Launcher: Send {
    /// Send `request` to the worker and return. Never blocks.
    fn request(&mut self, request: Request);
    /// Take the worker's next answer, if one is ready. Never blocks.
    fn drain(&mut self) -> Option<Outcome>;
}

/// The inert implementation: `request` discards, `drain` is always `None`, no thread and no
/// process. What `start_collaborators` uses when no repository was found.
struct NoLauncher;

impl Launcher for NoLauncher {
    fn request(&mut self, _request: Request) {}

    fn drain(&mut self) -> Option<Outcome> {
        None
    }
}

/// The inert `Launcher`. See [`NoLauncher`].
pub fn none() -> Box<dyn Launcher> {
    Box::new(NoLauncher)
}

/// The real implementation, private on exactly `refresh::RealRefresher`'s and
/// `agents::RealAgentPoll`'s terms: nothing outside this module names it, since every consumer
/// reaches it through `Box<dyn Launcher>`. Structure only in this group: `request` sends and
/// `drain` `try_recv`s, but the worker below answers nothing yet — group 5 fills in the real
/// three-call sequence.
struct RealLauncher {
    request_tx: std::sync::mpsc::Sender<Request>,
    result_rx: std::sync::mpsc::Receiver<Outcome>,
}

impl Launcher for RealLauncher {
    fn request(&mut self, request: Request) {
        let _ = self.request_tx.send(request);
    }

    fn drain(&mut self) -> Option<Outcome> {
        self.result_rx.try_recv().ok()
    }
}

/// Start the crate's third worker thread, on `refresh::start`'s and `agents::start`'s shape:
/// one request channel in, one result channel out, the worker body written **below** the
/// single `thread::spawn` so `NOBLOCK`'s leg 3 can cut the production slice there. The worker
/// reaches the `herdr` program only through `Arc<dyn HerdrCli>`; this module names no
/// process-spawn API of its own.
pub fn start(
    cli: std::sync::Arc<dyn crate::cli::HerdrCli>,
    repo: std::path::PathBuf,
    kind: String,
    state_dir: Option<std::path::PathBuf>,
) -> Box<dyn Launcher> {
    let (request_tx, request_rx) = std::sync::mpsc::channel();
    let (result_tx, result_rx) = std::sync::mpsc::channel::<Outcome>();
    std::thread::spawn(move || worker_body(cli, repo, kind, state_dir, request_rx, result_tx));
    Box::new(RealLauncher {
        request_tx,
        result_rx,
    })
}

// Everything below this point is the worker's own body — reached only from inside a
// `thread::spawn` closure, never from the render path, and therefore free to block. `NOBLOCK`
// leg 3 relies on this ordering: it cuts `src/launch.rs`'s production slice at its single
// `thread::spawn` and only searches the half before it.

/// The worker's whole body. Group 1: consumes every request and answers nothing — the real
/// three-call sequence arrives in group 4. Returns when the request channel disconnects, on
/// exactly `refresh::worker_body`'s and `agents::worker_body`'s lifecycle.
fn worker_body(
    _cli: std::sync::Arc<dyn crate::cli::HerdrCli>,
    _repo: std::path::PathBuf,
    _kind: String,
    _state_dir: Option<std::path::PathBuf>,
    request_rx: std::sync::mpsc::Receiver<Request>,
    _result_tx: std::sync::mpsc::Sender<Outcome>,
) {
    loop {
        if request_rx.recv().is_err() {
            return; // the launcher was dropped
        }
        // group 1: the request is discarded here; group 4 implements run_request.
    }
}

#[cfg(test)]
mod tests {}
