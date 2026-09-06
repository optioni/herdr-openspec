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

#[cfg(test)]
mod tests {
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
}
