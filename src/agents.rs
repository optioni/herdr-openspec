//! The Herdr agent poller: the crate's second worker thread. Confined to this one
//! module, on exactly `src/watch.rs`'s and `src/refresh.rs`'s single-file terms — see
//! `openspec/changes/agent-polling/design.md` -> Boundaries. Reaches the `herdr` program
//! only through `Arc<dyn crate::cli::HerdrCli>`: it spawns no process itself, and
//! `src/cli.rs` stays the crate's single spawn site.
//!
//! Group 1 populates only the inert halves: the types, the parse stub, the poll stub, and
//! a worker that answers nothing. The real parse arrives in group 3, the real poll mapping
//! in group 4, and the real schedule and worker body in group 7.

use std::path::PathBuf;
use std::time::Duration;

use crate::cli::HerdrCli;

/// The poll cadence: `SPEC.md` -> Agent status by polling's "roughly one-second
/// intervals" has one place it is written down.
pub const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// An agent's status, decoded from Herdr's own `agent_status` field. `Unknown` is what
/// any unrecognised string, and an absent field, decode to — a forward-compatible fact
/// about a newer Herdr, not a broken payload.
///
/// Deliberately outside `NODEFAULT-UI`'s reach: the check's positive control is anchored
/// on `struct <T> {`, so an enum cannot be added to its swept type list without breaking
/// that control, and a `Default` on this enum would change nothing because every
/// construction site is a `match` arm naming a variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    Working,
    Idle,
    Blocked,
    Done,
    Unknown,
}

/// One agent Herdr reports. `name` is the name a user or this plugin gave the agent,
/// omitted by Herdr when unset; `kind` is the agent *kind* (`"claude"` on every live
/// agent measured) — the two are separate fields carrying separate JSON keys, and the
/// distinction is load-bearing rather than cosmetic (see `specs/agent-list/spec.md`).
///
/// Carries no `terminal_id`, even though Herdr's own schema marks it required: nothing
/// in Phase 5 addresses a terminal, and a field no consumer reads is a field every
/// construction site must still fill. No `Default`, anywhere in the crate; every
/// construction and every destructuring names every field, with no `..` rest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agent {
    pub name: Option<String>,
    pub kind: Option<String>,
    pub status: AgentStatus,
    pub cwd: Option<PathBuf>,
    pub pane_id: String,
    pub tab_id: String,
    pub workspace_id: String,
    pub terminal_title: Option<String>,
}

/// The result of parsing one `agent list` payload: the agents that parsed, and one line
/// per entry that could not be, naming the fault and its position — a single bad row
/// does not fail the whole read. No `Default`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    pub agents: Vec<Agent>,
    pub problems: Vec<String>,
}

/// The latest poll's outcome: `reachable` is false for an unreachable socket, and
/// `problem` then names why, without becoming a `!`-marked problem row anywhere — see
/// `specs/agent-poller/spec.md` -> "The dashboard carries the latest snapshot and
/// nothing renders it". No `Default`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSnapshot {
    pub agents: Vec<Agent>,
    pub reachable: bool,
    pub problem: Option<String>,
}

/// Parse `herdr agent list`'s payload — an envelope,
/// `{"id":…,"result":{"agents":[…],"type":"agent_list"}}` — into a [`Listed`].
///
/// Group 1 stub: always `Err`, so group 4's `poll_once` tests are red on behaviour
/// rather than on a fixture typo. The real navigation arrives in group 3.
pub fn parse_list(_text: &str) -> Result<Listed, String> {
    Err("not implemented".to_string())
}

/// Map one `herdr agent list` call to an [`AgentSnapshot`], which has no error case of
/// its own: every outcome — a clean payload, a partial one, a failed run, an absent
/// program — becomes a snapshot.
///
/// Group 1 stub: a constant answer, deliberately wrong in every direction (`reachable`
/// true with no agents, whatever the CLI actually said) so group 4's six tests are red
/// on behaviour rather than on a stub that already guessed right. The real four-way
/// mapping arrives in group 4.
pub fn poll_once(_cli: &dyn HerdrCli) -> AgentSnapshot {
    AgentSnapshot {
        agents: Vec::new(),
        reachable: true,
        problem: None,
    }
}

/// A non-blocking source of agent snapshots. Every method SHALL be non-blocking, on
/// exactly `watch::FsEvents`'s and `refresh::Refresher`'s terms — the render path calls
/// both on every frame, and neither may wait on anything.
pub trait AgentPoll: Send {
    /// Fire a poll if one is due and take the worker's answer if one is ready. Never
    /// blocks, never sleeps, never waits on a channel.
    fn drain(&mut self) -> Option<AgentSnapshot>;
    /// The time remaining before the next poll is due, or `None` when a poll is
    /// already in flight and there is nothing to wake for. Takes no `Instant`: the real
    /// implementation returns the value its own last `drain` computed.
    fn pending_in(&self) -> Option<Duration>;
}

/// The inert implementation: `drain` is always `None`, `pending_in` is always `None`,
/// no thread, no process. What `agents::none()` returns.
struct NoAgentPoll;

impl AgentPoll for NoAgentPoll {
    fn drain(&mut self) -> Option<AgentSnapshot> {
        None
    }

    fn pending_in(&self) -> Option<Duration> {
        None
    }
}

/// The inert `AgentPoll`. See [`NoAgentPoll`].
pub fn none() -> Box<dyn AgentPoll> {
    Box::new(NoAgentPoll)
}

/// The real implementation, private on exactly `refresh::RealRefresher`'s terms: nothing
/// outside this module names it, since every consumer reaches it through
/// `Box<dyn AgentPoll>`. Structure only in this group: `drain` and `pending_in` both
/// answer `None` unconditionally, exactly like [`NoAgentPoll`], until group 7 adds the
/// schedule (design.md -> Decisions 2) that reads a clock on the render side and hands
/// requests to the worker below.
struct RealAgentPoll {
    _worker: std::thread::JoinHandle<()>,
}

impl AgentPoll for RealAgentPoll {
    fn drain(&mut self) -> Option<AgentSnapshot> {
        None
    }

    fn pending_in(&self) -> Option<Duration> {
        None
    }
}

/// Start the crate's second worker thread. `cli` is the same seam `poll_once` takes,
/// reached only through the trait object — the worker spawns no process itself, and
/// `src/cli.rs` remains the crate's single spawn site.
///
/// Group 1 stub: the spawned thread answers nothing yet — it exists so later groups
/// extend a real worker rather than introduce one. The real request/response channel
/// and the `recv` loop calling [`poll_once`] arrive in group 7.
pub fn start(cli: std::sync::Arc<dyn HerdrCli>) -> Box<dyn AgentPoll> {
    let worker = std::thread::spawn(move || {
        let _cli = cli;
    });
    Box::new(RealAgentPoll { _worker: worker })
}
