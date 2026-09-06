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
}

impl AgentPoll for RealAgentPoll {
    /// Group 7 RED shape: every call both takes an answer, if one is ready, and fires a
    /// fresh request — no in-flight suppression and no schedule yet, so the tests
    /// asserting "exactly one run" and "reported once" are red on behaviour rather than
    /// on a stub that never touched the channels at all. The real schedule (design.md ->
    /// Decisions 2) lands in the same group's GREEN step.
    fn drain(&mut self) -> Option<AgentSnapshot> {
        let result = self.result_rx.try_recv().ok();
        let _ = self.request_tx.send(());
        result
    }

    fn pending_in(&self) -> Option<Duration> {
        None
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
    })
}

// Everything below this point is the worker's own body — reached only from inside a
// `thread::spawn` closure, never from the render path, and therefore free to block.
// `NOBLOCK` leg 3 relies on this ordering: it cuts `src/agents.rs`'s production slice at
// its single `thread::spawn` and only searches the half before it. Group 7 stub: shared
// by `agents::poller_for_test` already; `agents::start` grows its own `thread::spawn` of
// this same function in task 7.2, replacing the trivial closure above.

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
            let (mut poller, result_rx, _exit_rx) = super::poller_for_test(cli);

            assert_eq!(poller.drain(), None);
            let snapshot = result_rx
                .recv_timeout(Duration::from_secs(10))
                .expect("the worker did not answer within 10s");
            assert!(snapshot.reachable);

            assert_eq!(poller.drain(), None);
            assert_eq!(poller.drain(), None);
            assert_eq!(poller.pending_in(), None);

            let runs = std::fs::read_to_string(&log).unwrap_or_default();
            assert_eq!(
                runs.lines().count(),
                1,
                "exactly one run across the three drains: {runs:?}"
            );
            assert_eq!(runs.lines().next(), Some("agent list"));
        }

        #[test]
        fn a_poll_in_flight_suppresses_the_next() {
            let scratch = ScratchDir::new();
            let log = scratch.path().join("log");
            // Blocks forever on stdin so the run never completes and the poll never
            // clears the in-flight state within the test's short life.
            let prog = script(
                &scratch,
                "herdr",
                &format!("printf '%s\\n' \"$*\" >> \"{}\"\ncat\n", log.display()),
            );
            let cli = crate::cli::agent_cli_via(&prog);
            let (mut poller, _result_rx, _exit_rx) = super::poller_for_test(cli);

            for _ in 0..5 {
                assert_eq!(poller.drain(), None);
                assert_eq!(poller.pending_in(), None);
            }

            let deadline = Instant::now() + Duration::from_secs(5);
            let mut runs = 0;
            while Instant::now() < deadline {
                runs = std::fs::read_to_string(&log)
                    .map(|s| s.lines().count())
                    .unwrap_or(0);
                if runs >= 1 {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            assert_eq!(runs, 1, "exactly one run, not five");
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
                std::thread::sleep(Duration::from_millis(10));
            }
            let first = first.expect("the first drain after disconnection must report once");
            assert!(!first.reachable);
            assert!(first.problem.is_some());

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
                std::thread::sleep(Duration::from_millis(10));
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
                std::thread::sleep(Duration::from_millis(10));
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
                std::thread::sleep(Duration::from_millis(10));
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
}
