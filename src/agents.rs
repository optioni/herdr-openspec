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
use std::time::{Duration, Instant};

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

/// The result of `attribute`: which change each in-scope agent was attributed to, and
/// how many in-scope agents no tier could place. Never `Default`, anywhere in the
/// crate; every construction and destructuring names both fields. See
/// `specs/agent-attribution/spec.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribution {
    pub badges: std::collections::BTreeMap<String, AgentStatus>,
    /// `agent-launch`'s addition: for every change that has a badge, the `pane_id` of the
    /// **same** agent whose status that badge shows — maintained in lockstep with `badges` so
    /// the two maps always hold exactly the same key set. `g` focuses the pane this map names.
    pub panes: std::collections::BTreeMap<String, String>,
    pub unattributed: usize,
}

/// Map live agents onto changes: a pure, total function of its four arguments that
/// refuses to guess. An agent is first placed in or out of the repository's scope
/// (`repo.is_some() && cwd.is_some() && cwd.starts_with(root)`); an in-scope agent is
/// then decided by exactly three tiers, in this order: the plugin-local mapping,
/// then an exact `name` match against `change_names`, then the count. Several agents
/// on one change fold to the single highest-precedence status — see `rank` below. See
/// `specs/agent-attribution/spec.md`.
pub fn attribute(
    agents: &[Agent],
    repo: Option<&std::path::Path>,
    change_names: &[&str],
    mapping: &std::collections::BTreeMap<String, String>,
) -> Attribution {
    let mut badges: std::collections::BTreeMap<String, AgentStatus> =
        std::collections::BTreeMap::new();
    // `agent-launch`'s addition: filled in the same fold as `badges`, below.
    let mut panes: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    let mut unattributed = 0usize;

    let Some(root) = repo else {
        return Attribution {
            badges,
            panes,
            unattributed,
        };
    };

    for agent in agents {
        let in_scope = agent
            .cwd
            .as_deref()
            .is_some_and(|cwd| cwd.starts_with(root));
        if !in_scope {
            continue;
        }

        let placed = agent.name.as_deref().and_then(|name| {
            // Tier 1: the plugin-local mapping, consulted only when it names a
            // change still in `change_names` — a stale record falls through
            // rather than stranding an agent the weaker tier can still place.
            let via_mapping = mapping
                .get(name)
                .and_then(|mapped| change_names.iter().find(|&&cn| cn == mapped));
            // Tier 2: an exact, byte-for-byte match against a change name.
            via_mapping.or_else(|| change_names.iter().find(|&&cn| cn == name))
        });

        match placed {
            Some(&change_name) => {
                let key = change_name.to_string();
                // `agent-launch`'s addition: `panes` follows the same strictly-greater fold as
                // `badges`, in lockstep, so a tie keeps the first agent's pane exactly as it
                // keeps the first agent's status — the two maps can never disagree about which
                // agent a change's row is talking about.
                let wins = match badges.get(&key) {
                    None => true,
                    Some(existing) => rank(agent.status) > rank(*existing),
                };
                if wins {
                    badges.insert(key.clone(), agent.status);
                    panes.insert(key, agent.pane_id.clone());
                }
            }
            None => unattributed += 1,
        }
    }

    Attribution {
        badges,
        panes,
        unattributed,
    }
}

/// The one total precedence order `attribute` folds several agents on one change by:
/// `Blocked` > `Working` > `Idle` > `Done` > `Unknown`, highest first. `Blocked` leads
/// because it is the only status asking for a person; `Unknown` trails because it
/// carries no information. Written once, beside `decode_entry`'s status `match`, on
/// exactly the terms design.md -> Boundaries names.
fn rank(status: AgentStatus) -> u8 {
    match status {
        AgentStatus::Blocked => 4,
        AgentStatus::Working => 3,
        AgentStatus::Idle => 2,
        AgentStatus::Done => 1,
        AgentStatus::Unknown => 0,
    }
}

/// Decode one entry of the `agents` array. `None` when the entry is not a JSON object,
/// or is missing (or holds a non-string) `pane_id`, `tab_id`, or `workspace_id` — the
/// three of Herdr's seven required fields this crate reads — with one line pushed onto
/// `problems` naming the fault and `position`. Every other field degrades to `None`
/// rather than skipping the entry: an agent Herdr has not named, in a pane whose working
/// directory it does not report, is an ordinary agent.
fn decode_entry(
    value: &serde_json::Value,
    position: usize,
    problems: &mut Vec<String>,
) -> Option<Agent> {
    let Some(obj) = value.as_object() else {
        problems.push(format!(
            "agent list entry at position {position} is not a JSON object"
        ));
        return None;
    };
    let Some(pane_id) = obj.get("pane_id").and_then(|v| v.as_str()) else {
        problems.push(format!(
            "agent list entry at position {position} has no usable \"pane_id\""
        ));
        return None;
    };
    let Some(tab_id) = obj.get("tab_id").and_then(|v| v.as_str()) else {
        problems.push(format!(
            "agent list entry at position {position} has no usable \"tab_id\""
        ));
        return None;
    };
    let Some(workspace_id) = obj.get("workspace_id").and_then(|v| v.as_str()) else {
        problems.push(format!(
            "agent list entry at position {position} has no usable \"workspace_id\""
        ));
        return None;
    };
    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let kind = obj
        .get("agent")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let cwd = obj.get("cwd").and_then(|v| v.as_str()).map(PathBuf::from);
    let terminal_title = obj
        .get("terminal_title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let status = match obj.get("agent_status").and_then(|v| v.as_str()) {
        Some("working") => AgentStatus::Working,
        Some("idle") => AgentStatus::Idle,
        Some("blocked") => AgentStatus::Blocked,
        Some("done") => AgentStatus::Done,
        // Any other string, and an absent field, decode to `Unknown` — a
        // forward-compatible fact about a newer Herdr, not a broken payload.
        _ => AgentStatus::Unknown,
    };
    Some(Agent {
        name,
        kind,
        status,
        cwd,
        pane_id: pane_id.to_string(),
        tab_id: tab_id.to_string(),
        workspace_id: workspace_id.to_string(),
        terminal_title,
    })
}

/// Parse `herdr agent list`'s payload — an envelope,
/// `{"id":…,"result":{"agents":[…],"type":"agent_list"}}` — into a [`Listed`], following
/// `changes::parse_list`'s navigation style: `serde_json::Value`, then `result.agents`,
/// then one [`Agent`] per entry. `Err(reason)` naming what was missing or wrong-shaped
/// for text that is not JSON, JSON that is not an object, an object with no `result`, a
/// `result` with no `agents`, an `agents` value that is not an array, or Herdr's own
/// error envelope (`{"id":…,"error":{"code":…,"message":…}}`) — reported by its `code`
/// and `message` rather than as a generic shape mismatch.
pub fn parse_list(text: &str) -> Result<Listed, String> {
    let value: serde_json::Value = serde_json::from_str(text)
        .map_err(|e| format!("agent list payload is not valid JSON: {e}"))?;
    let obj = value
        .as_object()
        .ok_or_else(|| "agent list payload is not a JSON object".to_string())?;

    if let Some(error) = obj.get("error").and_then(|v| v.as_object()) {
        let code = error
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or("<unknown code>");
        let message = error
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("<no message>");
        return Err(format!(
            "herdr agent list reported an error: {code}: {message}"
        ));
    }

    let result = obj
        .get("result")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "agent list payload has no \"result\" object".to_string())?;
    let entries = result
        .get("agents")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "agent list payload's \"result\" has no \"agents\" array".to_string())?;

    let mut agents = Vec::with_capacity(entries.len());
    let mut problems = Vec::new();
    for (position, entry) in entries.iter().enumerate() {
        if let Some(agent) = decode_entry(entry, position, &mut problems) {
            agents.push(agent);
        }
    }

    Ok(Listed { agents, problems })
}

/// Format a failed `herdr agent list` call's reason. Unlike
/// `changes::cli_error_problem`, this carries `stderr` and the OS `reason` verbatim:
/// the OpenSpec CLI writes its diagnostic to stdout, so `CliError::Failed`'s `stderr` is
/// empty for it, but Herdr does the reverse — an unreachable socket exits 1 with an
/// empty stdout and a JSON error envelope on stderr — so for `herdr` the reason *is*
/// available and the pane should carry it. See `specs/agent-list/spec.md` -> "A failed
/// run is an unreachable socket carrying the program's own reason".
fn herdr_error_problem(err: &crate::cli::CliError) -> String {
    match err {
        crate::cli::CliError::Failed { code, stderr, .. } => {
            let code = code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            format!("herdr agent list exited with code {code}: {stderr}")
        }
        crate::cli::CliError::NotStarted { reason, .. } => {
            format!("herdr agent list: could not start herdr: {reason}")
        }
    }
}

/// Map one `herdr agent list` call to an [`AgentSnapshot`], which has no error case of
/// its own: every outcome — a clean payload, a partial one, a failed run, an absent
/// program — becomes a snapshot. `problem` carries `Listed::problems` joined into one
/// string when a successful payload held a bad entry; a payload with one bad entry is
/// still `reachable: true`. See `specs/agent-list/spec.md` -> "A failed run is an
/// unreachable socket carrying the program's own reason".
pub fn poll_once(cli: &dyn HerdrCli) -> AgentSnapshot {
    match cli.run(&["agent", "list"]) {
        Ok(text) => match parse_list(&text) {
            Ok(listed) => AgentSnapshot {
                agents: listed.agents,
                reachable: true,
                problem: (!listed.problems.is_empty()).then(|| listed.problems.join("; ")),
            },
            Err(reason) => AgentSnapshot {
                agents: Vec::new(),
                reachable: false,
                problem: Some(reason),
            },
        },
        Err(err) => AgentSnapshot {
            agents: Vec::new(),
            reachable: false,
            problem: Some(herdr_error_problem(&err)),
        },
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
    request_tx: std::sync::mpsc::Sender<()>,
    result_rx: std::sync::mpsc::Receiver<AgentSnapshot>,
    /// The instant the next poll becomes due, computed from the same `now` an answer
    /// arrived at. `None` means "due now, or never yet decided" — the first `drain` on a
    /// freshly started poller.
    next_due: Option<Instant>,
    /// At most one poll in flight at a time: set when a request is sent, cleared when
    /// its answer arrives.
    in_flight: bool,
    /// The `Duration` `pending_in` returns until the next call — left behind by `drain`
    /// each time, so `pending_in` reads a clock never.
    pending: Option<Duration>,
    /// Set once the worker's result channel disconnects, so the standing "worker
    /// stopped" snapshot is reported exactly once and the poller degrades to silence —
    /// on `agents::none()`'s own terms — thereafter.
    dead: bool,
}

impl AgentPoll for RealAgentPoll {
    /// The schedule lives here, on the render side (design.md -> Decisions 2): read the
    /// clock once; take the worker's answer with `try_recv`, and on an answer set the
    /// next due instant to that same `now` plus `POLL_INTERVAL`; then, when no poll is
    /// in flight and the next due instant has arrived or was never set, send one request
    /// to the worker; then leave behind the `Duration` `pending_in` returns.
    fn drain(&mut self) -> Option<AgentSnapshot> {
        if self.dead {
            return None;
        }
        let now = Instant::now();
        let mut result = None;
        match self.result_rx.try_recv() {
            Ok(snapshot) => {
                self.next_due = Some(now + POLL_INTERVAL);
                self.in_flight = false;
                result = Some(snapshot);
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.dead = true;
                self.pending = None;
                return Some(AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    problem: Some("the agent poller's worker has stopped answering".to_string()),
                });
            }
        }

        let due = match self.next_due {
            None => true,
            Some(next) => now >= next,
        };
        if !self.in_flight && due {
            // The request channel disconnecting here (the worker already died between
            // this drain and the last) is handled identically to a mid-flight death: the
            // next drain's `try_recv` observes `Disconnected` and reports it once.
            let _ = self.request_tx.send(());
            self.in_flight = true;
        }

        self.pending = if self.in_flight {
            None
        } else {
            self.next_due
                .map(|next| next.saturating_duration_since(now))
        };
        result
    }

    fn pending_in(&self) -> Option<Duration> {
        self.pending
    }
}

/// Start the crate's second worker thread. `cli` is the same seam `poll_once` takes,
/// reached only through the trait object — the worker spawns no process itself, and
/// `src/cli.rs` remains the crate's single spawn site.
pub fn start(cli: std::sync::Arc<dyn HerdrCli>) -> Box<dyn AgentPoll> {
    let (request_tx, request_rx) = std::sync::mpsc::channel();
    let (result_tx, result_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || worker_body(cli, request_rx, result_tx));
    Box::new(RealAgentPoll {
        request_tx,
        result_rx,
        next_due: None,
        in_flight: false,
        pending: None,
        dead: false,
    })
}

// Everything below this point is the worker's own body — reached only from inside a
// `thread::spawn` closure, never from the render path, and therefore free to block.
// `NOBLOCK` leg 3 relies on this ordering: it cuts `src/agents.rs`'s production slice at
// its single `thread::spawn` and only searches the half before it.

/// The worker's whole body: block on the request channel, call [`poll_once`] for each
/// request, and send the resulting [`AgentSnapshot`] back, returning when either channel
/// disconnects — the same lifecycle `refresh::worker_body` already has, so dropping the
/// poller drops the request `Sender` and the thread returns.
fn worker_body(
    cli: std::sync::Arc<dyn HerdrCli>,
    request_rx: std::sync::mpsc::Receiver<()>,
    result_tx: std::sync::mpsc::Sender<AgentSnapshot>,
) {
    loop {
        if request_rx.recv().is_err() {
            return; // the poller was dropped
        }
        let snapshot = poll_once(cli.as_ref());
        if result_tx.send(snapshot).is_err() {
            return; // nobody reads the result any more
        }
    }
}

#[cfg(test)]
mod tests {
    /// A poller for the module's own worker tests, shaped exactly like
    /// `refresh::worker_for_test`: its own `drain` **never yields a snapshot** — a
    /// single-consumer channel cannot answer both `drain` and a raw receiver — so the
    /// test reads the worker's answers off the returned `mpsc::Receiver<AgentSnapshot>`
    /// directly, with a deadline-bounded `recv_timeout`. The third channel's `Sender` is
    /// owned by the worker thread, so its return after a drop is observable on the third
    /// element rather than assumed. Declared **inside** `mod tests` so the single
    /// line-anchored `#[cfg(test)]` count `NOBLOCK`'s Guard D requires is unaffected.
    pub(crate) fn poller_for_test(
        cli: std::sync::Arc<dyn crate::cli::HerdrCli>,
    ) -> (
        Box<dyn super::AgentPoll>,
        std::sync::mpsc::Receiver<super::AgentSnapshot>,
        std::sync::mpsc::Receiver<()>,
    ) {
        let (request_tx, request_rx) = std::sync::mpsc::channel::<()>();
        let (result_tx, result_rx) = std::sync::mpsc::channel::<super::AgentSnapshot>();
        // `exit_tx` is never sent on: its `Sender` is simply owned by the worker thread
        // and moved into the closure, so it drops only when `worker_body` returns and
        // the closure ends — observable on `exit_rx` as a channel disconnection, never
        // as a received value. Mirrors `refresh::worker_for_test` exactly.
        let (exit_tx, exit_rx) = std::sync::mpsc::channel::<()>();
        std::thread::spawn(move || {
            let _exit_tx = exit_tx;
            super::worker_body(cli, request_rx, result_tx);
        });
        (
            Box::new(TestAgentPoll {
                request_tx,
                sent: false,
            }),
            result_rx,
            exit_rx,
        )
    }

    /// `poller_for_test`'s own poller: forwards a single request the first time
    /// `drain` is called and never again, and never yields a snapshot of its own — the
    /// test reads answers off the raw receiver `poller_for_test` hands back instead.
    struct TestAgentPoll {
        request_tx: std::sync::mpsc::Sender<()>,
        sent: bool,
    }

    impl super::AgentPoll for TestAgentPoll {
        fn drain(&mut self) -> Option<super::AgentSnapshot> {
            if !self.sent {
                let _ = self.request_tx.send(());
                self.sent = true;
            }
            None
        }

        fn pending_in(&self) -> Option<std::time::Duration> {
            if self.sent {
                None
            } else {
                Some(std::time::Duration::ZERO)
            }
        }
    }

    mod parse {
        use crate::agents::{Agent, AgentStatus, Listed};

        /// Wrap `agents_json` — the contents of the `agents` array, comma-separated
        /// entries as a raw string — into the envelope Herdr 0.8.2 actually emits.
        fn envelope(agents_json: &str) -> String {
            format!(
                r#"{{"id":"cli:agent:list","result":{{"agents":[{agents_json}],"type":"agent_list"}}}}"#
            )
        }

        #[test]
        fn the_reference_payload_parses() {
            let text = r#"{"id":"cli:agent:list","result":{"agents":[{"agent":"claude","agent_session":{"agent":"claude","kind":"id","source":"herdr:claude","value":"0e80c276-952e-4150-b32f-06cc6247ce01"},"agent_status":"idle","cwd":"/repo","focused":true,"foreground_cwd":"/repo","pane_id":"w8:p1","revision":35,"state_change_seq":963,"tab_id":"w8:t1","terminal_id":"term_65a34df386c314","terminal_title":"✳ a title","terminal_title_stripped":"a title","workspace_id":"w8"}],"type":"agent_list"}}"#;
            let listed = super::super::parse_list(text).expect("should parse");
            assert_eq!(
                listed,
                Listed {
                    agents: vec![Agent {
                        name: None,
                        kind: Some("claude".to_string()),
                        status: AgentStatus::Idle,
                        cwd: Some(std::path::PathBuf::from("/repo")),
                        pane_id: "w8:p1".to_string(),
                        tab_id: "w8:t1".to_string(),
                        workspace_id: "w8".to_string(),
                        terminal_title: Some("\u{2733} a title".to_string()),
                    }],
                    problems: Vec::new(),
                }
            );
            assert_eq!(listed.agents[0].name, None);
            assert_eq!(listed.agents[0].kind, Some("claude".to_string()));
        }

        #[test]
        fn an_empty_agents_array_is_ok_and_empty() {
            let text = envelope("");
            let listed = super::super::parse_list(&text).expect("should parse");
            assert_eq!(
                listed,
                Listed {
                    agents: Vec::new(),
                    problems: Vec::new(),
                }
            );
        }

        #[test]
        fn a_named_agent_carries_its_name() {
            let text = envelope(
                r#"{"pane_id":"p1","tab_id":"t1","workspace_id":"w1","agent":"claude","name":"agent-polling"}"#,
            );
            let listed = super::super::parse_list(&text).expect("should parse");
            assert_eq!(listed.agents.len(), 1);
            assert_eq!(listed.agents[0].name, Some("agent-polling".to_string()));
            assert_eq!(listed.agents[0].kind, Some("claude".to_string()));
        }

        #[test]
        fn every_status_decodes_and_others_are_unknown() {
            fn entry(status: &str) -> String {
                format!(
                    r#"{{"pane_id":"p","tab_id":"t","workspace_id":"w","agent_status":"{status}"}}"#
                )
            }

            let five = envelope(&format!(
                "{},{},{},{},{}",
                entry("working"),
                entry("idle"),
                entry("blocked"),
                entry("done"),
                entry("unknown"),
            ));
            let listed = super::super::parse_list(&five).expect("should parse");
            assert_eq!(
                listed.agents.iter().map(|a| a.status).collect::<Vec<_>>(),
                vec![
                    AgentStatus::Working,
                    AgentStatus::Idle,
                    AgentStatus::Blocked,
                    AgentStatus::Done,
                    AgentStatus::Unknown,
                ]
            );
            assert!(listed.problems.is_empty());

            let sixth = envelope(&entry("reticulating"));
            let listed = super::super::parse_list(&sixth).expect("should parse");
            assert_eq!(listed.agents[0].status, AgentStatus::Unknown);
            assert!(listed.problems.is_empty());

            let seventh = envelope(r#"{"pane_id":"p","tab_id":"t","workspace_id":"w"}"#);
            let listed = super::super::parse_list(&seventh).expect("should parse");
            assert_eq!(listed.agents[0].status, AgentStatus::Unknown);
            assert!(listed.problems.is_empty());

            let eighth =
                envelope(r#"{"pane_id":"p","tab_id":"t","workspace_id":"w","agent_status":3}"#);
            let listed = super::super::parse_list(&eighth).expect("should parse");
            assert_eq!(listed.agents[0].status, AgentStatus::Unknown);
            assert!(listed.problems.is_empty());
        }

        #[test]
        fn only_the_required_fields_still_parses() {
            let text = envelope(
                r#"{"pane_id":"w1:p1","tab_id":"w1:t1","workspace_id":"w1","focused":false,"revision":1,"terminal_id":"t","agent_status":"working"}"#,
            );
            let listed = super::super::parse_list(&text).expect("should parse");
            assert_eq!(
                listed,
                Listed {
                    agents: vec![Agent {
                        name: None,
                        kind: None,
                        status: AgentStatus::Working,
                        cwd: None,
                        pane_id: "w1:p1".to_string(),
                        tab_id: "w1:t1".to_string(),
                        workspace_id: "w1".to_string(),
                        terminal_title: None,
                    }],
                    problems: Vec::new(),
                }
            );
        }

        fn three_entries(middle: &str) -> String {
            format!(
                r#"{{"pane_id":"p0","tab_id":"t0","workspace_id":"w0"}},{middle},{{"pane_id":"p2","tab_id":"t2","workspace_id":"w2"}}"#
            )
        }

        #[test]
        fn a_bad_entry_is_skipped_and_named() {
            // Primary variant: the middle entry has no `pane_id` key at all.
            let text = envelope(&three_entries(r#"{"tab_id":"t1","workspace_id":"w1"}"#));
            let listed = super::super::parse_list(&text).expect("should parse");
            assert_eq!(listed.agents.len(), 2);
            assert_eq!(listed.agents[0].pane_id, "p0");
            assert_eq!(listed.agents[1].pane_id, "p2");
            assert_eq!(listed.problems.len(), 1);
            assert!(listed.problems[0].contains("pane_id"));
            assert!(listed.problems[0].contains('1'));
        }

        #[test]
        fn a_bad_entry_is_skipped_and_named_by_type_or_by_a_different_field() {
            // Second variant: pane_id is present but not a string.
            let text = envelope(&three_entries(
                r#"{"pane_id":7,"tab_id":"t1","workspace_id":"w1"}"#,
            ));
            let listed = super::super::parse_list(&text).expect("should parse");
            assert_eq!(listed.agents.len(), 2);
            assert_eq!(listed.problems.len(), 1);
            assert!(listed.problems[0].contains("pane_id"));
            assert!(listed.problems[0].contains('1'));

            // Third variant: tab_id and workspace_id are the ones missing.
            let text = envelope(&three_entries(r#"{"pane_id":"p1"}"#));
            let listed = super::super::parse_list(&text).expect("should parse");
            assert_eq!(listed.agents.len(), 2);
            assert_eq!(listed.problems.len(), 1);
            assert!(
                listed.problems[0].contains("tab_id")
                    || listed.problems[0].contains("workspace_id")
            );
            assert!(listed.problems[0].contains('1'));
        }

        #[test]
        fn a_non_object_entry_is_skipped() {
            // Primary variant: the middle entry is a JSON string.
            let text = envelope(&three_entries("\"claude\""));
            let listed = super::super::parse_list(&text).expect("should parse");
            assert_eq!(listed.agents.len(), 2);
            assert_eq!(listed.agents[0].pane_id, "p0");
            assert_eq!(listed.agents[1].pane_id, "p2");
            assert_eq!(listed.problems.len(), 1);
            assert!(listed.problems[0].contains('1'));
            assert!(listed.problems[0].to_lowercase().contains("object"));
        }

        #[test]
        fn a_non_object_entry_is_skipped_for_other_json_types() {
            // Second variant: a JSON number.
            let text = envelope(&three_entries("7"));
            let listed = super::super::parse_list(&text).expect("should parse");
            assert_eq!(listed.agents.len(), 2);
            assert_eq!(listed.problems.len(), 1);
            assert!(listed.problems[0].contains('1'));

            // Third variant: JSON null.
            let text = envelope(&three_entries("null"));
            let listed = super::super::parse_list(&text).expect("should parse");
            assert_eq!(listed.agents.len(), 2);
            assert_eq!(listed.problems.len(), 1);
            assert!(listed.problems[0].contains('1'));
        }

        #[test]
        fn a_cwd_outside_the_repo_is_still_parsed() {
            let text = envelope(
                r#"{"pane_id":"p","tab_id":"t","workspace_id":"w","cwd":"/somewhere/else"}"#,
            );
            let listed = super::super::parse_list(&text).expect("should parse");
            assert_eq!(
                listed.agents[0].cwd,
                Some(std::path::PathBuf::from("/somewhere/else"))
            );
        }

        #[test]
        fn non_json_is_a_named_reason() {
            let result = super::super::parse_list("usage: herdr agent list");
            match result {
                Err(reason) => assert!(
                    reason.to_lowercase().contains("json"),
                    "reason should name the parse failure: {reason}"
                ),
                Ok(listed) => panic!("expected Err, got Ok({listed:?})"),
            }
        }

        #[test]
        fn wrong_shapes_are_named_reasons() {
            let cases: [(&str, &str); 4] = [
                ("{}", "result"),
                (r#"{"result":{}}"#, "agents"),
                (r#"{"result":{"agents":{}}}"#, "agents"),
                ("[]", "object"),
            ];
            for (text, expected) in cases {
                let result = super::super::parse_list(text);
                match result {
                    Err(reason) => assert!(
                        reason.to_lowercase().contains(expected),
                        "text {text}: reason {reason:?} should name {expected:?}"
                    ),
                    Ok(listed) => panic!("text {text}: expected Err, got Ok({listed:?})"),
                }
            }
        }

        #[test]
        fn herdrs_error_envelope_is_reported_by_code() {
            let text = r#"{"id":"cli:agent:list","error":{"code":"server_not_running","message":"no herdr server is running at /p/herdr.sock"}}"#;
            let result = super::super::parse_list(text);
            match result {
                Err(reason) => {
                    assert!(reason.contains("server_not_running"), "{reason}");
                    assert!(
                        reason.contains("no herdr server is running at /p/herdr.sock"),
                        "{reason}"
                    );
                }
                Ok(listed) => panic!("expected Err, got Ok({listed:?})"),
            }
        }
    }

    mod poll {
        use crate::cli::{CliError, FakeCli, Program};

        #[test]
        fn args_are_exactly_agent_list() {
            let fake = FakeCli::new();
            fake.register_herdr(&["agent", "list"], Ok(reference_payload()));

            let _ = super::super::poll_once(&fake);

            assert_eq!(
                fake.calls(),
                vec![(
                    Program::Herdr,
                    vec!["agent".to_string(), "list".to_string()]
                )]
            );
        }

        fn reference_payload() -> String {
            r#"{"id":"cli:agent:list","result":{"agents":[{"agent":"claude","agent_status":"idle","pane_id":"w8:p1","tab_id":"w8:t1","workspace_id":"w8"}],"type":"agent_list"}}"#.to_string()
        }

        #[test]
        fn a_clean_success_is_reachable_with_its_agents() {
            let fake = FakeCli::new();
            fake.register_herdr(&["agent", "list"], Ok(reference_payload()));

            let snapshot = super::super::poll_once(&fake);

            assert!(snapshot.reachable);
            assert_eq!(snapshot.agents.len(), 1);
            assert_eq!(snapshot.agents[0].pane_id, "w8:p1");
            assert_eq!(snapshot.problem, None);
        }

        #[test]
        fn a_failed_run_is_an_unreachable_snapshot() {
            let fake = FakeCli::new();
            fake.register_herdr(
                &["agent", "list"],
                Err(CliError::Failed {
                    program: "herdr".to_string(),
                    args: vec!["agent".to_string(), "list".to_string()],
                    code: Some(1),
                    stderr: r#"{"id":"cli:agent:list","error":{"code":"server_not_running","message":"no herdr server is running at /p/herdr.sock"}}"#
                        .to_string(),
                }),
            );

            let snapshot = super::super::poll_once(&fake);

            assert!(!snapshot.reachable);
            assert_eq!(snapshot.agents, Vec::new());
            let problem = snapshot.problem.expect("a reason must be present");
            assert!(problem.contains("herdr agent list"), "{problem}");
            assert!(problem.contains('1'), "{problem}");
            assert!(problem.contains("server_not_running"), "{problem}");
        }

        #[test]
        fn not_started_is_an_unreachable_snapshot() {
            let fake = FakeCli::new();
            fake.register_herdr(
                &["agent", "list"],
                Err(CliError::NotStarted {
                    program: "herdr".to_string(),
                    args: vec!["agent".to_string(), "list".to_string()],
                    reason: "No such file or directory (os error 2)".to_string(),
                }),
            );

            let snapshot = super::super::poll_once(&fake);

            assert!(!snapshot.reachable);
            assert_eq!(snapshot.agents, Vec::new());
            let problem = snapshot.problem.expect("a reason must be present");
            assert!(problem.contains("herdr"), "{problem}");
            assert!(problem.contains("No such file or directory"), "{problem}");
        }

        #[test]
        fn an_unparsable_success_is_an_unreachable_snapshot() {
            let fake = FakeCli::new();
            fake.register_herdr(
                &["agent", "list"],
                Ok("usage: herdr agent list".to_string()),
            );

            let snapshot = super::super::poll_once(&fake);

            assert!(!snapshot.reachable);
            assert_eq!(snapshot.agents, Vec::new());
            assert!(snapshot.problem.is_some());
        }

        #[test]
        fn a_partial_payload_is_still_reachable() {
            let fake = FakeCli::new();
            let text = r#"{"id":"cli:agent:list","result":{"agents":[
                {"pane_id":"p0","tab_id":"t0","workspace_id":"w0"},
                {"tab_id":"t1","workspace_id":"w1"},
                {"pane_id":"p2","tab_id":"t2","workspace_id":"w2"}
            ],"type":"agent_list"}}"#;
            fake.register_herdr(&["agent", "list"], Ok(text.to_string()));

            let snapshot = super::super::poll_once(&fake);

            assert!(snapshot.reachable);
            assert_eq!(snapshot.agents.len(), 2);
            let problem = snapshot.problem.expect("the skipped entry must be named");
            assert!(problem.contains("pane_id"), "{problem}");
        }
    }

    mod seam {
        use crate::agents::{Agent, AgentSnapshot, AgentStatus, Listed, POLL_INTERVAL};

        #[test]
        fn the_inert_poller_yields_nothing() {
            let mut poller = super::super::none();
            for _ in 0..10 {
                assert_eq!(poller.drain(), None);
                assert_eq!(poller.pending_in(), None);
            }
        }

        #[test]
        fn poll_interval_is_one_second() {
            assert_eq!(POLL_INTERVAL, std::time::Duration::from_secs(1));
        }

        /// The compile-time companion for this module's own three no-`Default` types:
        /// exhaustive destructuring, no `..` rest, so a field added to any of the three
        /// fails to compile here rather than defaulting silently.
        #[test]
        fn agent_types_destructure_exhaustively_with_no_default() {
            let agent = Agent {
                name: None,
                kind: None,
                status: AgentStatus::Unknown,
                cwd: None,
                pane_id: "p".to_string(),
                tab_id: "t".to_string(),
                workspace_id: "w".to_string(),
                terminal_title: None,
            };
            let Agent {
                name,
                kind,
                status,
                cwd,
                pane_id,
                tab_id,
                workspace_id,
                terminal_title,
            } = agent;
            assert_eq!(name, None);
            assert_eq!(kind, None);
            assert_eq!(status, AgentStatus::Unknown);
            assert_eq!(cwd, None);
            assert_eq!(pane_id, "p");
            assert_eq!(tab_id, "t");
            assert_eq!(workspace_id, "w");
            assert_eq!(terminal_title, None);

            let listed = Listed {
                agents: Vec::new(),
                problems: Vec::new(),
            };
            let Listed { agents, problems } = listed;
            assert!(agents.is_empty());
            assert!(problems.is_empty());

            let snapshot = AgentSnapshot {
                agents: Vec::new(),
                reachable: false,
                problem: None,
            };
            let AgentSnapshot {
                agents,
                reachable,
                problem,
            } = snapshot;
            assert!(agents.is_empty());
            assert!(!reachable);
            assert_eq!(problem, None);
        }
    }

    mod worker {
        use std::time::{Duration, Instant};

        use crate::testutil::{ScratchDir, write_with_mode};

        fn script(dir: &ScratchDir, name: &str, body: &str) -> std::path::PathBuf {
            let path = dir.path().join(name);
            write_with_mode(&path, format!("#!/bin/sh\n{body}").as_bytes(), 0o755);
            path
        }

        const ONE_AGENT: &str = r#"{"id":"cli:agent:list","result":{"agents":[{"agent":"claude","agent_status":"idle","pane_id":"w8:p1","tab_id":"w8:t1","workspace_id":"w8"}],"type":"agent_list"}}"#;

        const UNREACHABLE_STDERR: &str = r#"{"id":"cli:agent:list","error":{"code":"server_not_running","message":"no herdr server is running at /p/herdr.sock"}}"#;

        /// Drives the **real** `RealAgentPoll` through `agents::start`, not
        /// `poller_for_test`'s hand-written `TestAgentPoll` double — a Change Review
        /// finding: `TestAgentPoll`'s own "send once" schedule is hardcoded, so a test
        /// built on it cannot discriminate a broken `RealAgentPoll::drain`. Both this
        /// scenario and `a_poll_in_flight_suppresses_the_next` are explicitly about
        /// `RealAgentPoll`'s own in-flight/next-due bookkeeping, so both now go through
        /// the real seam, on `a_started_poller_yields_the_scratch_programs_agents`'s
        /// terms.
        #[test]
        fn the_first_drain_polls_immediately() {
            let scratch = ScratchDir::new();
            let log = scratch.path().join("log");
            let prog = script(
                &scratch,
                "herdr",
                &format!(
                    "printf '%s\\n' \"$*\" >> \"{}\"\nprintf '%s' '{ONE_AGENT}'\n",
                    log.display()
                ),
            );
            let cli = crate::cli::agent_cli_via(&prog);
            let mut poller = super::super::start(cli);

            // The first drain never yields synchronously — try_recv is empty before
            // any request has been answered — but it must have fired the request.
            assert_eq!(poller.drain(), None);

            // Deadline-bounded wait for the answer, which may arrive on any later
            // drain call — a scratch `#!/bin/sh` program's real latency is not
            // controlled here, only bounded.
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut snapshot = None;
            while Instant::now() < deadline {
                if let Some(s) = poller.drain() {
                    snapshot = Some(s);
                    break;
                }
                std::thread::yield_now();
            }
            let snapshot = snapshot.expect("the worker did not answer within 10s");
            assert!(snapshot.reachable);

            // Two more drains, well inside the one-second POLL_INTERVAL gap the
            // schedule now holds until the next poll is due: no further request.
            assert_eq!(poller.drain(), None);
            assert_eq!(poller.drain(), None);
            let pending = poller
                .pending_in()
                .expect("a completed poll must report a deadline for the next one");
            assert!(
                pending <= super::super::POLL_INTERVAL,
                "the reported deadline must be no further out than POLL_INTERVAL: {pending:?}"
            );

            let runs = std::fs::read_to_string(&log).unwrap_or_default();
            assert_eq!(
                runs.lines().count(),
                1,
                "exactly one run despite every later drain: {runs:?}"
            );
            assert_eq!(runs.lines().next(), Some("agent list"));
        }

        #[test]
        fn a_poll_in_flight_suppresses_the_next() {
            let scratch = ScratchDir::new();
            let log = scratch.path().join("log");
            // Answers after a bounded 300ms, rather than blocking forever: the
            // worker's own body is a single sequential loop, so a script that never
            // returns would make "exactly one run" true regardless of whether the
            // render side suppressed the other four requests or merely queued them
            // behind the first — the worker could never reach a queued one either
            // way. A bounded delay lets a queued backlog drain and show itself: if
            // `drain` sent five requests instead of one, the worker answers them
            // one after another once the first completes, and a second (or third,
            // or more) invocation appears in the log well within this test's wait.
            let prog = script(
                &scratch,
                "herdr",
                &format!(
                    "printf '%s\\n' \"$*\" >> \"{}\"\nsleep 0.3\nprintf '%s' '{ONE_AGENT}'\n",
                    log.display()
                ),
            );
            let cli = crate::cli::agent_cli_via(&prog);
            let mut poller = super::super::start(cli);

            for _ in 0..5 {
                assert_eq!(poller.drain(), None);
                assert_eq!(poller.pending_in(), None);
            }

            // Deadline-bounded poll, well short of POLL_INTERVAL, so a second
            // *legitimate* poll is not yet due either: if the render side queued
            // five requests instead of one, the worker drains the backlog as soon
            // as the first 300ms answer arrives, and a second entry shows up in the
            // log well inside this window. If only one was ever queued, the count
            // stays at one for the whole 2s wait.
            let deadline = Instant::now() + Duration::from_millis(2000);
            let mut runs = 0;
            while Instant::now() < deadline {
                runs = std::fs::read_to_string(&log)
                    .map(|s| s.lines().count())
                    .unwrap_or(0);
                if runs >= 2 {
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            assert_eq!(
                runs, 1,
                "exactly one run, not a backlog of five processed once the worker freed up"
            );
        }

        #[test]
        fn a_dead_worker_is_reported_once() {
            // A dropped `Sender`/`Receiver` pair, on real channels: an unregistered
            // `FakeCli` panics inside the worker's own call to `poll_once`, which
            // unwinds the whole worker thread and drops both ends of its channels —
            // observed by the render-side poller exactly as a real crash would be,
            // rather than a Sender dropped by hand from outside the seam.
            let cli: std::sync::Arc<dyn crate::cli::HerdrCli> =
                std::sync::Arc::new(crate::cli::FakeCli::new());
            let mut poller = super::super::start(cli);

            let deadline = Instant::now() + Duration::from_secs(5);
            let mut first = None;
            while Instant::now() < deadline {
                if let Some(s) = poller.drain() {
                    first = Some(s);
                    break;
                }
                std::thread::yield_now();
            }
            let first = first.expect("the first drain after disconnection must report once");
            assert!(!first.reachable);
            let problem = first.problem.expect("a reason must be present");
            assert!(
                problem.to_lowercase().contains("worker"),
                "the reason must name the poller's worker as stopped: {problem}"
            );

            assert_eq!(poller.drain(), None);
            assert_eq!(poller.drain(), None);
            assert_eq!(poller.pending_in(), None);
        }

        #[test]
        fn a_started_poller_yields_the_scratch_programs_agents() {
            let scratch = ScratchDir::new();
            let log = scratch.path().join("log");
            let prog = script(
                &scratch,
                "herdr",
                &format!(
                    "printf '%s\\n' \"$*\" >> \"{}\"\nprintf '%s' '{ONE_AGENT}'\n",
                    log.display()
                ),
            );
            let cli = crate::cli::agent_cli_via(&prog);
            let mut poller = super::super::start(cli);

            let deadline = Instant::now() + Duration::from_secs(10);
            let mut snapshot = None;
            while Instant::now() < deadline {
                if let Some(s) = poller.drain() {
                    snapshot = Some(s);
                    break;
                }
                std::thread::yield_now();
            }
            let snapshot = snapshot.expect("a snapshot should arrive within 10s");
            assert!(snapshot.reachable);
            assert_eq!(snapshot.agents.len(), 1);
            assert_eq!(snapshot.agents[0].pane_id, "w8:p1");

            let runs = std::fs::read_to_string(&log).unwrap_or_default();
            assert_eq!(runs.lines().count(), 1);
            assert_eq!(runs.lines().next(), Some("agent list"));
        }

        #[test]
        fn a_failing_scratch_program_is_unreachable() {
            let scratch = ScratchDir::new();
            let prog = script(
                &scratch,
                "herdr",
                &format!("printf '%s' '{UNREACHABLE_STDERR}' >&2\nexit 1\n"),
            );
            let cli = crate::cli::agent_cli_via(&prog);
            let mut poller = super::super::start(cli);

            let deadline = Instant::now() + Duration::from_secs(10);
            let mut snapshot = None;
            while Instant::now() < deadline {
                if let Some(s) = poller.drain() {
                    snapshot = Some(s);
                    break;
                }
                std::thread::yield_now();
            }
            let snapshot = snapshot.expect("a snapshot should arrive within 10s");
            assert!(!snapshot.reachable);
            let problem = snapshot.problem.expect("a reason must be present");
            assert!(problem.contains('1'), "{problem}");
            assert!(problem.contains("server_not_running"), "{problem}");
        }

        #[test]
        fn a_failed_poll_is_recovered_from() {
            let scratch = ScratchDir::new();
            let log = scratch.path().join("log");
            let counter = scratch.path().join("counter");
            write_with_mode(&counter, b"0", 0o644);
            // Fails on its first run, then succeeds on every later one — deciding
            // by reading and rewriting a counter file, since a scratch program has
            // no other memory across invocations.
            let prog = script(
                &scratch,
                "herdr",
                &format!(
                    "printf '%s\\n' \"$*\" >> \"{log}\"\n\
                     n=$(cat \"{counter}\")\n\
                     n=$((n + 1))\n\
                     printf '%s' \"$n\" > \"{counter}\"\n\
                     if [ \"$n\" -eq 1 ]; then\n\
                     printf '%s' '{UNREACHABLE_STDERR}' >&2\n\
                     exit 1\n\
                     fi\n\
                     printf '%s' '{ONE_AGENT}'\n",
                    log = log.display(),
                    counter = counter.display(),
                ),
            );
            let cli = crate::cli::agent_cli_via(&prog);
            let mut poller = super::super::start(cli);

            let deadline = Instant::now() + Duration::from_secs(10);
            let mut snapshots = Vec::new();
            while Instant::now() < deadline && snapshots.len() < 2 {
                if let Some(s) = poller.drain() {
                    snapshots.push(s);
                }
                std::thread::yield_now();
            }
            assert_eq!(
                snapshots.len(),
                2,
                "expected two snapshots within the deadline"
            );
            assert!(!snapshots[0].reachable, "the first poll must fail");
            assert!(snapshots[1].reachable, "the second poll must recover");
            assert_eq!(snapshots[1].agents.len(), 1);

            let runs = std::fs::read_to_string(&log).unwrap_or_default();
            assert_eq!(
                runs.lines().count(),
                2,
                "the poller neither backed off nor stopped after the failure: {runs:?}"
            );
        }

        #[test]
        fn dropping_the_poller_stops_the_thread() {
            let cli = std::sync::Arc::new(crate::cli::RealHerdrCli::new("herdr"));
            let (poller, _result_rx, exit_rx) = super::poller_for_test(cli);
            drop(poller);

            match exit_rx.recv_timeout(Duration::from_secs(10)) {
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {}
                other => panic!(
                    "the worker did not return within 10s after the poller was dropped: {other:?}"
                ),
            }
        }
    }

    mod attribute {
        use crate::agents::{Agent, AgentStatus, attribute};
        use std::collections::BTreeMap;
        use std::path::PathBuf;

        /// Build an `Agent` naming every field, on the module's own no-`Default` terms.
        /// `pane_id`, `tab_id`, and `workspace_id` are never read by `attribute`, so a
        /// fixed placeholder is enough.
        fn agent(
            name: Option<&str>,
            kind: Option<&str>,
            status: AgentStatus,
            cwd: Option<&str>,
            terminal_title: Option<&str>,
        ) -> Agent {
            Agent {
                name: name.map(str::to_string),
                kind: kind.map(str::to_string),
                status,
                cwd: cwd.map(PathBuf::from),
                pane_id: "p".to_string(),
                tab_id: "t".to_string(),
                workspace_id: "w".to_string(),
                terminal_title: terminal_title.map(str::to_string),
            }
        }

        fn mapping(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        }

        fn empty_mapping() -> BTreeMap<String, String> {
            BTreeMap::new()
        }

        #[test]
        fn an_unmatched_in_scope_agent_is_counted() {
            let a = agent(
                Some("scratch-work"),
                None,
                AgentStatus::Working,
                Some("/repo"),
                None,
            );
            let m = empty_mapping();

            let result = attribute(
                std::slice::from_ref(&a),
                Some(std::path::Path::new("/repo")),
                &["add-auth", "fix-basket"],
                &m,
            );
            assert!(result.badges.is_empty());
            assert!(result.panes.is_empty());
            assert_eq!(result.unattributed, 1);

            let reordered = attribute(
                &[a],
                Some(std::path::Path::new("/repo")),
                &["fix-basket", "add-auth"],
                &m,
            );
            assert_eq!(result, reordered, "nothing was assigned by position");
        }

        #[test]
        fn the_name_tier_never_reads_the_kind() {
            let m = empty_mapping();
            let a = agent(None, Some("claude"), AgentStatus::Idle, Some("/repo"), None);
            let result = attribute(&[a], Some(std::path::Path::new("/repo")), &["claude"], &m);
            assert!(result.badges.is_empty());
            assert!(result.panes.is_empty());
            assert_eq!(result.unattributed, 1);

            let named = agent(
                Some("claude"),
                Some("claude"),
                AgentStatus::Idle,
                Some("/repo"),
                None,
            );
            let result = attribute(
                &[named],
                Some(std::path::Path::new("/repo")),
                &["claude"],
                &m,
            );
            assert_eq!(
                result.badges,
                BTreeMap::from([("claude".to_string(), AgentStatus::Idle)])
            );
            assert_eq!(
                result.panes,
                BTreeMap::from([("claude".to_string(), "p".to_string())])
            );
            assert_eq!(result.unattributed, 0);
        }

        #[test]
        fn a_terminal_title_attributes_nothing() {
            let m = empty_mapping();
            let with_title = agent(
                None,
                None,
                AgentStatus::Working,
                Some("/repo"),
                Some("\u{2733} add-auth: wiring the token refresh"),
            );
            let result = attribute(
                &[with_title],
                Some(std::path::Path::new("/repo")),
                &["add-auth"],
                &m,
            );
            assert!(result.badges.is_empty());
            assert!(result.panes.is_empty());
            assert_eq!(result.unattributed, 1);

            let without_title = agent(None, None, AgentStatus::Working, Some("/repo"), None);
            let result2 = attribute(
                &[without_title],
                Some(std::path::Path::new("/repo")),
                &["add-auth"],
                &m,
            );
            assert_eq!(result, result2, "the title changed no outcome at all");
        }

        #[test]
        fn every_empty_input_is_total() {
            let repo = Some(std::path::Path::new("/repo"));
            let m = empty_mapping();

            // 1. an empty agents slice.
            let r1 = attribute(&[], repo, &["alpha"], &m);
            assert!(r1.badges.is_empty());
            assert!(r1.panes.is_empty());
            assert_eq!(r1.unattributed, 0);

            // 2. one in-scope agent, an empty change_names.
            let working_alpha = agent(
                Some("alpha"),
                None,
                AgentStatus::Working,
                Some("/repo"),
                None,
            );
            let r2 = attribute(std::slice::from_ref(&working_alpha), repo, &[], &m);
            assert!(r2.badges.is_empty());
            assert!(r2.panes.is_empty());
            assert_eq!(
                r2.unattributed, 1,
                "an in-scope agent no tier could place is counted, even with no changes at all"
            );

            // 3. the same agent, change_names non-empty, no repository.
            let r3 = attribute(&[working_alpha], None, &["alpha"], &m);
            assert!(r3.badges.is_empty());
            assert!(r3.panes.is_empty());
            assert_eq!(r3.unattributed, 0);

            // 4. an agent with every optional field None.
            let bare = agent(None, None, AgentStatus::Working, None, None);
            let r4 = attribute(&[bare], repo, &["alpha"], &m);
            assert!(r4.badges.is_empty());
            assert!(r4.panes.is_empty());
            assert_eq!(r4.unattributed, 0);

            // 5. change_names holding a duplicate.
            let idle_alpha = agent(Some("alpha"), None, AgentStatus::Idle, Some("/repo"), None);
            let r5 = attribute(&[idle_alpha], repo, &["alpha", "alpha"], &m);
            assert_eq!(
                r5.badges,
                BTreeMap::from([("alpha".to_string(), AgentStatus::Idle)])
            );
            assert_eq!(
                r5.panes,
                BTreeMap::from([("alpha".to_string(), "p".to_string())]),
                "one entry each, not two, for a duplicated change name"
            );
            assert_eq!(r5.unattributed, 0);
        }

        #[test]
        fn an_agent_in_another_repository_is_invisible() {
            let m = empty_mapping();
            let outside = agent(
                Some("add-auth"),
                None,
                AgentStatus::Working,
                Some("/other/repo"),
                None,
            );
            let inside = agent(
                Some("nothing-like-a-change"),
                None,
                AgentStatus::Working,
                Some("/repo"),
                None,
            );
            let result = attribute(
                &[outside, inside],
                Some(std::path::Path::new("/repo")),
                &["add-auth"],
                &m,
            );
            assert!(result.badges.is_empty());
            assert_eq!(result.unattributed, 1);
        }

        #[test]
        fn containment_is_component_wise() {
            let m = empty_mapping();
            let at_root = agent(None, None, AgentStatus::Idle, Some("/repo"), None);
            let subdir = agent(
                None,
                None,
                AgentStatus::Idle,
                Some("/repo/openspec/changes/alpha"),
                None,
            );
            let sibling = agent(None, None, AgentStatus::Idle, Some("/repo-other"), None);
            let result = attribute(
                &[at_root, subdir, sibling],
                Some(std::path::Path::new("/repo")),
                &[],
                &m,
            );
            assert_eq!(
                result.unattributed, 2,
                "the sibling /repo-other must be excluded by component-wise containment"
            );
        }

        #[test]
        fn no_repository_attributes_nothing() {
            let m = mapping(&[("c-add-auth", "add-auth")]);
            let agents: Vec<Agent> = (0..3)
                .map(|_| agent(None, None, AgentStatus::Idle, Some("/repo"), None))
                .collect();
            let result = attribute(&agents, None, &["add-auth"], &m);
            assert!(result.badges.is_empty());
            assert_eq!(result.unattributed, 0);
        }

        #[test]
        fn the_mapping_resolves_a_derived_name() {
            let m = mapping(&[("c-2fa-support", "2fa-support")]);
            let a = agent(
                Some("c-2fa-support"),
                None,
                AgentStatus::Working,
                Some("/repo"),
                None,
            );
            let result = attribute(
                std::slice::from_ref(&a),
                Some(std::path::Path::new("/repo")),
                &["2fa-support", "add-auth"],
                &m,
            );
            assert_eq!(
                result.badges,
                BTreeMap::from([("2fa-support".to_string(), AgentStatus::Working)])
            );
            assert_eq!(result.unattributed, 0);

            let empty = empty_mapping();
            let result2 = attribute(
                &[a],
                Some(std::path::Path::new("/repo")),
                &["2fa-support", "add-auth"],
                &empty,
            );
            assert!(result2.badges.is_empty());
            assert_eq!(result2.unattributed, 1);
        }

        #[test]
        fn the_mapping_outranks_the_name() {
            let m = mapping(&[("alpha", "beta")]);
            let a = agent(
                Some("alpha"),
                None,
                AgentStatus::Blocked,
                Some("/repo"),
                None,
            );
            let result = attribute(
                &[a],
                Some(std::path::Path::new("/repo")),
                &["alpha", "beta"],
                &m,
            );
            assert_eq!(
                result.badges,
                BTreeMap::from([("beta".to_string(), AgentStatus::Blocked)])
            );
            assert!(!result.badges.contains_key("alpha"));
            assert_eq!(result.unattributed, 0);
        }

        #[test]
        fn a_stale_mapping_falls_through() {
            let m = mapping(&[("alpha", "long-since-archived")]);
            let a = agent(Some("alpha"), None, AgentStatus::Idle, Some("/repo"), None);
            let result = attribute(&[a], Some(std::path::Path::new("/repo")), &["alpha"], &m);
            assert_eq!(
                result.badges,
                BTreeMap::from([("alpha".to_string(), AgentStatus::Idle)])
            );

            let m2 = mapping(&[("c-gone", "long-since-archived")]);
            let gone = agent(Some("c-gone"), None, AgentStatus::Idle, Some("/repo"), None);
            let result2 = attribute(
                &[gone],
                Some(std::path::Path::new("/repo")),
                &["alpha"],
                &m2,
            );
            assert!(result2.badges.is_empty());
            assert_eq!(
                result2.unattributed, 1,
                "the fall-through must end in the count, never an invented change"
            );
        }

        #[test]
        fn an_empty_mapping_leaves_the_name_tier_working() {
            let m = empty_mapping();
            let a = agent(
                Some("alpha"),
                None,
                AgentStatus::Working,
                Some("/repo"),
                None,
            );
            let result = attribute(&[a], Some(std::path::Path::new("/repo")), &["alpha"], &m);
            assert_eq!(
                result.badges,
                BTreeMap::from([("alpha".to_string(), AgentStatus::Working)])
            );
            assert_eq!(result.unattributed, 0);
        }

        #[test]
        fn both_tiers_of_the_change_list_are_attributable() {
            let m = empty_mapping();
            let alpha = agent(
                Some("alpha"),
                None,
                AgentStatus::Working,
                Some("/repo"),
                None,
            );
            let legacy = agent(
                Some("2026-08-14-legacy"),
                None,
                AgentStatus::Working,
                Some("/repo"),
                None,
            );
            let result = attribute(
                &[alpha.clone(), legacy],
                Some(std::path::Path::new("/repo")),
                &["alpha", "legacy"],
                &m,
            );
            assert_eq!(
                result.badges,
                BTreeMap::from([("alpha".to_string(), AgentStatus::Working)])
            );
            assert_eq!(result.unattributed, 1);

            let renamed = agent(
                Some("legacy"),
                None,
                AgentStatus::Working,
                Some("/repo"),
                None,
            );
            let result2 = attribute(
                &[alpha, renamed],
                Some(std::path::Path::new("/repo")),
                &["alpha", "legacy"],
                &m,
            );
            assert_eq!(
                result2.badges,
                BTreeMap::from([
                    ("alpha".to_string(), AgentStatus::Working),
                    ("legacy".to_string(), AgentStatus::Working),
                ])
            );
            assert_eq!(result2.unattributed, 0);
        }

        #[test]
        fn matching_is_exact() {
            let m = empty_mapping();
            let near_misses: Vec<Agent> = ["Add-Auth", "add-auth-2", "add", " add-auth"]
                .iter()
                .map(|n| agent(Some(n), None, AgentStatus::Idle, Some("/repo"), None))
                .collect();
            let result = attribute(
                &near_misses,
                Some(std::path::Path::new("/repo")),
                &["add-auth"],
                &m,
            );
            assert!(result.badges.is_empty());
            assert_eq!(result.unattributed, 4);

            let mut with_hit = near_misses;
            with_hit.push(agent(
                Some("add-auth"),
                None,
                AgentStatus::Idle,
                Some("/repo"),
                None,
            ));
            let result2 = attribute(
                &with_hit,
                Some(std::path::Path::new("/repo")),
                &["add-auth"],
                &m,
            );
            assert_eq!(
                result2.badges,
                BTreeMap::from([("add-auth".to_string(), AgentStatus::Idle)])
            );
            assert_eq!(result2.unattributed, 4);
        }

        #[test]
        fn a_name_past_the_cap_is_mapping_only() {
            // A change name well past Herdr's 32-character cap — the exact count is not
            // load-bearing, only that it derives to a legal, shorter agent name.
            let long_name = "a-change-name-that-runs-well-past-thirty-two-characters";
            let derived = crate::state::agent_name(long_name);
            assert!(derived.len() <= 32);

            let m = empty_mapping();
            let a = agent(
                Some(&derived),
                None,
                AgentStatus::Working,
                Some("/repo"),
                None,
            );
            let result = attribute(
                std::slice::from_ref(&a),
                Some(std::path::Path::new("/repo")),
                &[long_name],
                &m,
            );
            assert!(result.badges.is_empty());
            assert_eq!(
                result.unattributed, 1,
                "no live agent can carry a name Herdr would reject"
            );

            let mapped = mapping(&[(derived.as_str(), long_name)]);
            let result2 = attribute(
                &[a],
                Some(std::path::Path::new("/repo")),
                &[long_name],
                &mapped,
            );
            assert_eq!(
                result2.badges,
                BTreeMap::from([(long_name.to_string(), AgentStatus::Working)])
            );
            assert_eq!(result2.unattributed, 0);
        }

        #[test]
        fn an_unnamed_agent_is_counted() {
            let m = mapping(&[("alpha", "alpha")]);
            let a = agent(None, Some("claude"), AgentStatus::Done, Some("/repo"), None);
            let result = attribute(&[a], Some(std::path::Path::new("/repo")), &["alpha"], &m);
            assert!(result.badges.is_empty());
            assert_eq!(result.unattributed, 1);
        }

        #[test]
        fn precedence_is_total_and_order_independent() {
            let m = empty_mapping();
            let statuses = [
                AgentStatus::Unknown,
                AgentStatus::Done,
                AgentStatus::Idle,
                AgentStatus::Working,
                AgentStatus::Blocked,
            ];
            let agents: Vec<Agent> = statuses
                .iter()
                .map(|s| agent(Some("alpha"), None, *s, Some("/repo"), None))
                .collect();
            let result = attribute(&agents, Some(std::path::Path::new("/repo")), &["alpha"], &m);
            assert_eq!(
                result.badges,
                BTreeMap::from([("alpha".to_string(), AgentStatus::Blocked)])
            );
            assert_eq!(
                result.panes,
                BTreeMap::from([("alpha".to_string(), "p".to_string())])
            );

            let mut reversed = agents.clone();
            reversed.reverse();
            let result_rev = attribute(
                &reversed,
                Some(std::path::Path::new("/repo")),
                &["alpha"],
                &m,
            );
            assert_eq!(result, result_rev);

            // Peel one rank at a time off the top of the precedence — Blocked, then
            // Working, then Idle, then Done — leaving the next-highest as the winner.
            let mut remaining = agents;
            for (peel, expected) in [
                (AgentStatus::Blocked, AgentStatus::Working),
                (AgentStatus::Working, AgentStatus::Idle),
                (AgentStatus::Idle, AgentStatus::Done),
                (AgentStatus::Done, AgentStatus::Unknown),
            ] {
                let pos = remaining
                    .iter()
                    .position(|a| a.status == peel)
                    .expect("the rank being peeled must still be present");
                remaining.remove(pos);
                let r = attribute(
                    &remaining,
                    Some(std::path::Path::new("/repo")),
                    &["alpha"],
                    &m,
                );
                assert_eq!(r.badges, BTreeMap::from([("alpha".to_string(), expected)]));
                assert_eq!(
                    r.panes,
                    BTreeMap::from([("alpha".to_string(), "p".to_string())]),
                    "panes follows each winner in turn, {expected:?}"
                );
            }
        }

        #[test]
        fn one_agent_per_change_keeps_its_status() {
            let m = empty_mapping();
            let names = ["a", "b", "c", "d", "e"];
            let statuses = [
                AgentStatus::Working,
                AgentStatus::Idle,
                AgentStatus::Blocked,
                AgentStatus::Done,
                AgentStatus::Unknown,
            ];
            let agents: Vec<Agent> = names
                .iter()
                .zip(statuses.iter())
                .map(|(n, s)| agent(Some(n), None, *s, Some("/repo"), None))
                .collect();
            let result = attribute(&agents, Some(std::path::Path::new("/repo")), &names, &m);
            assert_eq!(
                result.badges,
                names
                    .iter()
                    .zip(statuses.iter())
                    .map(|(n, s)| (n.to_string(), *s))
                    .collect::<BTreeMap<_, _>>()
            );
            assert_eq!(
                result.panes,
                names
                    .iter()
                    .map(|n| (n.to_string(), "p".to_string()))
                    .collect::<BTreeMap<_, _>>()
            );
            assert_eq!(result.unattributed, 0);
        }

        /// `agent-launch`'s addition: `panes` and `badges` always hold the same key set, and
        /// for every key, `panes[key]` is the `pane_id` of the agent whose status
        /// `badges[key]` shows — proved across every fixture the tiers, the reordering, the
        /// out-of-scope, the no-repository, and the precedence scenarios above use.
        #[test]
        fn the_pane_map_and_the_badge_map_agree() {
            fn assert_agree(result: &super::super::Attribution) {
                let badge_keys: std::collections::BTreeSet<_> = result.badges.keys().collect();
                let pane_keys: std::collections::BTreeSet<_> = result.panes.keys().collect();
                assert_eq!(
                    badge_keys, pane_keys,
                    "badges and panes must share a key set"
                );
            }

            let m = empty_mapping();

            let a = agent(
                Some("scratch-work"),
                None,
                AgentStatus::Working,
                Some("/repo"),
                None,
            );
            assert_agree(&attribute(
                &[a],
                Some(std::path::Path::new("/repo")),
                &["add-auth", "fix-basket"],
                &m,
            ));

            let named = agent(
                Some("claude"),
                Some("claude"),
                AgentStatus::Idle,
                Some("/repo"),
                None,
            );
            assert_agree(&attribute(
                &[named],
                Some(std::path::Path::new("/repo")),
                &["claude"],
                &m,
            ));

            let names = ["a", "b", "c", "d", "e"];
            let statuses = [
                AgentStatus::Working,
                AgentStatus::Idle,
                AgentStatus::Blocked,
                AgentStatus::Done,
                AgentStatus::Unknown,
            ];
            let agents: Vec<Agent> = names
                .iter()
                .zip(statuses.iter())
                .map(|(n, s)| agent(Some(n), None, *s, Some("/repo"), None))
                .collect();
            assert_agree(&attribute(
                &agents,
                Some(std::path::Path::new("/repo")),
                &names,
                &m,
            ));

            assert_agree(&attribute(
                &[],
                Some(std::path::Path::new("/repo")),
                &[],
                &m,
            ));
        }

        /// `agent-attribution`'s "A tie keeps the first agent's pane": two agents on the same
        /// change, the same status, distinct panes — the fold's strictly-greater comparison
        /// never fires for the second, so its pane never overwrites the first's.
        #[test]
        fn a_tie_keeps_the_first_agents_pane() {
            fn agent_at(pane_id: &str) -> Agent {
                Agent {
                    name: Some("alpha".to_string()),
                    kind: None,
                    status: AgentStatus::Working,
                    cwd: Some(PathBuf::from("/repo")),
                    pane_id: pane_id.to_string(),
                    tab_id: "t".to_string(),
                    workspace_id: "w".to_string(),
                    terminal_title: None,
                }
            }
            let m = empty_mapping();
            let first = agent_at("w:p1");
            let second = agent_at("w:p2");

            let result = attribute(
                &[first.clone(), second.clone()],
                Some(std::path::Path::new("/repo")),
                &["alpha"],
                &m,
            );
            assert_eq!(
                result.badges,
                BTreeMap::from([("alpha".to_string(), AgentStatus::Working)])
            );
            assert_eq!(
                result.panes,
                BTreeMap::from([("alpha".to_string(), "w:p1".to_string())])
            );

            let reversed = attribute(
                &[second, first],
                Some(std::path::Path::new("/repo")),
                &["alpha"],
                &m,
            );
            assert_eq!(
                reversed.panes,
                BTreeMap::from([("alpha".to_string(), "w:p2".to_string())]),
                "the payload order is Herdr's own and is stable across polls"
            );
        }
    }
}
