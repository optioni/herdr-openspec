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
//! `decide` is a pure, total policy function; `pane_id` navigates the `pane split` envelope;
//! `run_request` issues the real three-call sequence (or the single `agent focus` call for
//! `Focus`) against a real `HerdrCli`; and `start`/`worker_body` run that sequence on the
//! crate's third worker thread, behind the `Launcher` trait every consumer reaches it through.

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
/// keep `Dashboard::agent_names` current without re-reading the file; `problems` carries every
/// reason the worker accumulated, in the order it occurred, and is empty on complete success.
/// **`degraded-states`' repair of row 23**: a single `Option<String>` could not hold both a
/// `state::record` failure and an `agent prompt` failure, so the one path where both fail
/// silently discarded the record's reason. `problems` holds **at most two** entries — the
/// record failure and the prompt failure are the only pair that can co-occur — never more,
/// since every earlier failure point returns immediately with exactly one. Never `Default`,
/// anywhere in the crate; every construction and destructuring names both fields, with no
/// `..` rest — on exactly `agents::AgentSnapshot`'s and `agents::Attribution`'s terms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub named: Option<(String, String)>,
    pub problems: Vec<String>,
}

/// The whole launch policy, a pure total function of its six arguments. Performs no
/// filesystem, process, environment, network, or terminal I/O, reads no clock and no global
/// state, spawns nothing, and never panics for any combination of arguments. The order is
/// exactly: an unreachable socket makes every intent inert; `Focus` resolves against `pane`
/// alone and returns without consulting `change` or `in_flight` — a user waiting through a
/// slow launch can still press `g`; no selected change means no launch; a launch already in
/// flight is refused before any live-name check; a derived name already live in the session is
/// refused before any request is produced; and otherwise the request goes ahead. See
/// `specs/agent-launch/spec.md` -> "The launch decision is a pure, total function that refuses
/// before it reaches Herdr".
pub fn decide(
    intent: Intent,
    change: Option<&str>,
    pane: Option<&str>,
    reachable: bool,
    live_names: &[&str],
    in_flight: bool,
) -> Decision {
    if !reachable {
        return Decision::Nothing;
    }
    if intent == Intent::Focus {
        return match pane {
            Some(pane_id) => Decision::Go(Request::Focus {
                pane_id: pane_id.to_string(),
            }),
            None => Decision::Nothing,
        };
    }
    let Some(change) = change else {
        return Decision::Nothing;
    };
    // `seam-resilience`: a launch already in flight is refused before the live-name check —
    // it is the more specific and more recent fact, and it closes a window `live_names` alone
    // cannot: between the press and the agent appearing in a poll, `live_names` does not yet
    // contain the derived name. See specs/agent-launch/spec.md -> "A second press while a
    // launch is in flight is refused, not queued".
    if in_flight {
        return Decision::Refuse(
            "a launch is already running - wait for it to finish".to_string(),
        );
    }
    // The derived name is `state::agent_name`'s own output, named once here — see task 3.5.
    let agent = crate::state::agent_name(change);
    if live_names.contains(&agent.as_str()) {
        return Decision::Refuse(format!(
            "{agent} is already running for this change - press g to focus it"
        ));
    }
    Decision::Go(Request::Launch {
        change: change.to_string(),
        agent,
        intent,
    })
}

/// Navigate Herdr's `pane split` envelope to `result.pane.pane_id`, following
/// `agents::parse_list`'s style: a `serde_json::Value`, then each key in turn, with
/// `Err(reason)` naming what was missing or wrong-shaped. Pure, and never panics for any
/// input, including the empty string. Reports Herdr's own error envelope
/// (`{"id":…,"error":{"code":…,"message":…}}`) by its `code` and `message` rather than as a
/// generic shape mismatch. See `specs/agent-launch/spec.md` -> "The pane id is read from `pane
/// split`'s envelope, and an unusable payload stops the launch".
pub fn pane_id(text: &str) -> Result<String, String> {
    let value: serde_json::Value = serde_json::from_str(text)
        .map_err(|e| format!("pane split payload is not valid JSON: {e}"))?;
    let obj = value
        .as_object()
        .ok_or_else(|| "pane split payload is not a JSON object".to_string())?;

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
            "herdr pane split reported an error: {code}: {message}"
        ));
    }

    let result = obj
        .get("result")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "pane split payload has no \"result\" object".to_string())?;
    let pane = result
        .get("pane")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "pane split payload's \"result\" has no \"pane\" object".to_string())?;
    let pane_id = pane
        .get("pane_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "pane split payload's \"pane\" has no usable \"pane_id\"".to_string())?;
    Ok(pane_id.to_string())
}

/// The exact argument vector for call 1, `pane split`. `--direction` is not optional and no
/// pane argument is given — see `specs/agent-launch/spec.md` -> "A launch is exactly three
/// Herdr calls". `repo` is rendered with `Path::to_string_lossy`, so a non-UTF-8 root is passed
/// lossily rather than failing the launch.
fn split_args(repo: &std::path::Path) -> Vec<String> {
    vec![
        "pane".to_string(),
        "split".to_string(),
        "--cwd".to_string(),
        repo.to_string_lossy().into_owned(),
        "--direction".to_string(),
        "right".to_string(),
        "--no-focus".to_string(),
    ]
}

/// The exact argument vector for call 2, `agent start`.
fn start_args(agent: &str, kind: &str, pane_id: &str) -> Vec<String> {
    vec![
        "agent".to_string(),
        "start".to_string(),
        agent.to_string(),
        "--kind".to_string(),
        kind.to_string(),
        "--pane".to_string(),
        pane_id.to_string(),
    ]
}

/// The exact argument vector for call 3, `agent prompt`. `text` is one element, exactly as
/// `prompt_text` produced it; the seam passes the vector to the program directly with no
/// shell, so a space needs no quoting.
fn prompt_args(agent: &str, text: &str) -> Vec<String> {
    vec![
        "agent".to_string(),
        "prompt".to_string(),
        agent.to_string(),
        text.to_string(),
    ]
}

/// The exact argument vector `g` issues: one Herdr call and no other.
fn focus_args(pane_id: &str) -> Vec<String> {
    vec![
        "agent".to_string(),
        "focus".to_string(),
        pane_id.to_string(),
    ]
}

/// The prompt text for `intent`: `/opsx:apply <change>`, `/opsx:continue <change>`, or
/// `/opsx:archive <change>` — one argument-vector element containing exactly one space.
/// `Intent::Focus` never reaches this function; `run_request` handles it through
/// `focus_args` instead.
pub fn prompt_text(intent: Intent, change: &str) -> String {
    let command = match intent {
        Intent::Apply => "apply",
        Intent::Continue => "continue",
        Intent::Archive => "archive",
        Intent::Focus => "apply",
    };
    format!("/opsx:{command} {change}")
}

/// Format a failed Herdr call's reason, on `agents::herdr_error_problem`'s established terms:
/// Herdr's diagnostic goes to **stderr** as a JSON error envelope, unlike the OpenSpec CLI's
/// stdout diagnostic, so the reason is available and carried verbatim rather than parsed.
fn herdr_reason(err: &crate::cli::CliError) -> String {
    match err {
        crate::cli::CliError::Failed { code, stderr, .. } => {
            let code = code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            format!("herdr exited with code {code}: {stderr}")
        }
        crate::cli::CliError::NotStarted { reason, .. } => {
            format!("could not start herdr: {reason}")
        }
        crate::cli::CliError::TimedOut { after, .. } => {
            format!("herdr timed out after {after:?}")
        }
    }
}

/// Run one `Request` to completion against a real `HerdrCli`, in the order the spec fixes:
/// split, parse the pane id, start, record, prompt — stopping at the first failure and
/// carrying Herdr's own reason verbatim. `Focus` is one call and nothing else. Never issues
/// `pane close`, on any path — a pane this fails to use is left in place, named in the
/// problem, because `agent start`'s failure set includes the readiness *timeout*, in which an
/// agent may be starting. See `specs/agent-launch/spec.md` -> "A failed call stops the launch
/// at that call and carries Herdr's own reason".
fn run_request(
    cli: &dyn crate::cli::HerdrCli,
    repo: &std::path::Path,
    kind: &str,
    state_dir: Option<&std::path::Path>,
    request: Request,
) -> Outcome {
    match request {
        Request::Focus { pane_id } => {
            let args = focus_args(&pane_id);
            let refs: Vec<&str> = args.iter().map(String::as_str).collect();
            match cli.run(&refs) {
                Ok(_) => Outcome {
                    named: None,
                    problems: Vec::new(),
                },
                Err(err) => Outcome {
                    named: None,
                    problems: vec![herdr_reason(&err)],
                },
            }
        }
        Request::Launch {
            change,
            agent,
            intent,
        } => {
            let split = split_args(repo);
            let split_refs: Vec<&str> = split.iter().map(String::as_str).collect();
            let payload = match cli.run(&split_refs) {
                Ok(text) => text,
                Err(err) => {
                    return Outcome {
                        named: None,
                        problems: vec![herdr_reason(&err)],
                    };
                }
            };
            let pane = match pane_id(&payload) {
                Ok(p) => p,
                Err(reason) => {
                    return Outcome {
                        named: None,
                        problems: vec![reason],
                    };
                }
            };

            let start = start_args(&agent, kind, &pane);
            let start_refs: Vec<&str> = start.iter().map(String::as_str).collect();
            if let Err(err) = cli.run(&start_refs) {
                return Outcome {
                    named: None,
                    problems: vec![format!(
                        "{} (agent {agent}, pane {pane})",
                        herdr_reason(&err)
                    )],
                };
            }

            // `state::record` runs between `agent start` and `agent prompt` (design.md ->
            // Decisions 12): a failure here is reported but does not undo the start, and the
            // prompt is still sent. `degraded-states`' repair of row 23: both this and the
            // prompt's own failure are pushed onto `problems` in occurrence order, rather than
            // the prompt's outcome silently replacing the record's.
            let mut problems = Vec::new();
            if let Err(e) = crate::state::record(state_dir, &agent, &change) {
                problems.push(e.to_string());
            }

            let prompt_text_value = prompt_text(intent, &change);
            let prompt = prompt_args(&agent, &prompt_text_value);
            let prompt_refs: Vec<&str> = prompt.iter().map(String::as_str).collect();
            if let Err(err) = cli.run(&prompt_refs) {
                problems.push(herdr_reason(&err));
            }
            Outcome {
                named: Some((agent, change)),
                problems,
            }
        }
    }
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
/// reaches it through `Box<dyn Launcher>`. `request` sends to the worker thread and `drain`
/// `try_recv`s its answers; the worker itself runs `run_request`'s real three-call sequence.
struct RealLauncher {
    request_tx: std::sync::mpsc::Sender<Request>,
    result_rx: std::sync::mpsc::Receiver<Outcome>,
    /// Set once the worker's death has been reported to a caller, on exactly
    /// `agents::RealAgentPoll`'s `dead` model: every `request` after this point is discarded
    /// and every `drain` answers `None`, so a dead launcher degrades to silence rather than to
    /// a growing list. `seam-resilience`.
    dead: bool,
    /// Set the instant a `SendError` or a `Disconnected` `try_recv` is first observed, and
    /// consumed by the very next `drain` — which reports it once and then sets `dead`. Kept
    /// separate from `dead` because `request` can detect the death before any `drain` runs,
    /// and the one report must still happen on a `drain` call, not a `request` call.
    pending_death: bool,
}

impl Launcher for RealLauncher {
    fn request(&mut self, request: Request) {
        if self.dead {
            return;
        }
        if self.request_tx.send(request).is_err() {
            self.pending_death = true;
        }
    }

    fn drain(&mut self) -> Option<Outcome> {
        if self.dead {
            return None;
        }
        if self.pending_death {
            self.dead = true;
            self.pending_death = false;
            return Some(dead_worker_outcome());
        }
        match self.result_rx.try_recv() {
            Ok(outcome) => Some(outcome),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.dead = true;
                Some(dead_worker_outcome())
            }
        }
    }
}

/// The `Outcome` reported exactly once when the launcher's worker has stopped answering, on
/// `agents::RealAgentPoll`'s "worker stopped" model. `named` is `None` — no agent was started
/// by this non-event — and `problems` carries the one reason.
fn dead_worker_outcome() -> Outcome {
    Outcome {
        named: None,
        problems: vec!["the launcher's worker has stopped answering".to_string()],
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
        dead: false,
        pending_death: false,
    })
}

/// How long [`settle`] waits for a launch already in flight to finish before giving up:
/// strictly above the measured thirty seconds `herdr agent start` spends waiting for
/// interactive readiness, and strictly below [`crate::cli::RUN_DEADLINE`] — both pinned by an
/// assertion, on `watch::DEBOUNCE`'s and `agents::POLL_INTERVAL`'s named-constant-plus-assertion
/// terms. `seam-resilience` -> design.md -> Decision 6.
pub const SETTLE_BUDGET: std::time::Duration = std::time::Duration::from_secs(35);

/// Poll `launcher.drain()` until it answers or `budget` elapses, returning whichever comes
/// first. A launch in flight at exit is given a bounded chance to finish rather than being
/// orphaned mid-sequence — `run_request` performs `pane split`, then `agent start`, then
/// `state::record`, then `agent prompt`, and `main` calls `exit(0)` as soon as `ui::run`
/// returns with the three workers detached and never joined.
///
/// A `while Instant::now() < deadline` poll, never a fixed sleep, on exactly `NOSLEEP` leg 1's
/// terms; declared here, below `start`'s single `thread::spawn`, so `NOBLOCK` leg 3's cut of
/// `src/launch.rs`'s production slice already excludes it — no gate is edited to admit it. The
/// `Launcher` trait still carries exactly two non-blocking methods; this free function is the
/// only place in the crate that waits on one. `seam-resilience` -> design.md -> Decision 6.
pub fn settle(launcher: &mut dyn Launcher, budget: std::time::Duration) -> Option<Outcome> {
    let deadline = std::time::Instant::now() + budget;
    while std::time::Instant::now() < deadline {
        if let Some(outcome) = launcher.drain() {
            return Some(outcome);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    None
}

// Everything below this point is the worker's own body — reached only from inside a
// `thread::spawn` closure, never from the render path, and therefore free to block. `NOBLOCK`
// leg 3 relies on this ordering: it cuts `src/launch.rs`'s production slice at its single
// `thread::spawn` and only searches the half before it.

/// The worker's whole body: consumes every request and runs it through `run_request`, sending
/// its `Outcome` back. Returns when the request channel disconnects, on exactly
/// `refresh::worker_body`'s and `agents::worker_body`'s lifecycle.
fn worker_body(
    cli: std::sync::Arc<dyn crate::cli::HerdrCli>,
    repo: std::path::PathBuf,
    kind: String,
    state_dir: Option<std::path::PathBuf>,
    request_rx: std::sync::mpsc::Receiver<Request>,
    result_tx: std::sync::mpsc::Sender<Outcome>,
) {
    loop {
        let Ok(request) = request_rx.recv() else {
            return; // the launcher was dropped
        };
        let outcome = run_request(cli.as_ref(), &repo, &kind, state_dir.as_deref(), request);
        if result_tx.send(outcome).is_err() {
            return; // nobody reads the result any more
        }
    }
}

#[cfg(test)]
mod tests {
    mod decide {
        use crate::launch::{Decision, Intent, Request, decide};

        #[test]
        fn an_unreachable_socket_makes_every_key_inert() {
            for intent in [
                Intent::Apply,
                Intent::Continue,
                Intent::Archive,
                Intent::Focus,
            ] {
                let result = decide(intent, Some("add-auth"), Some("w8:p3"), false, &[], false);
                assert_eq!(result, Decision::Nothing, "{intent:?}");
            }
        }

        #[test]
        fn no_change_means_no_launch_and_no_agent_means_no_focus() {
            for intent in [Intent::Apply, Intent::Continue, Intent::Archive] {
                assert_eq!(
                    decide(intent, None, None, true, &[], false),
                    Decision::Nothing,
                    "{intent:?}"
                );
            }
            assert_eq!(
                decide(Intent::Focus, None, None, true, &[], false),
                Decision::Nothing
            );
            assert_eq!(
                decide(Intent::Focus, None, Some("w8:p3"), true, &[], false),
                Decision::Go(Request::Focus {
                    pane_id: "w8:p3".to_string()
                })
            );
        }

        #[test]
        fn each_intent_carries_its_own_change_and_name() {
            let mut results = Vec::new();
            for intent in [Intent::Apply, Intent::Continue, Intent::Archive] {
                let result = decide(intent, Some("2fa-support"), None, true, &[], false);
                assert_eq!(
                    result,
                    Decision::Go(Request::Launch {
                        change: "2fa-support".to_string(),
                        agent: "c-2fa-support".to_string(),
                        intent,
                    })
                );
                results.push(result);
            }
            // The three differ only in `intent`.
            for pair in results.windows(2) {
                let (
                    Decision::Go(Request::Launch {
                        change: c1,
                        agent: a1,
                        intent: _,
                    }),
                    Decision::Go(Request::Launch {
                        change: c2,
                        agent: a2,
                        intent: _,
                    }),
                ) = (&pair[0], &pair[1])
                else {
                    panic!("expected two Go(Launch) values");
                };
                assert_eq!(c1, c2);
                assert_eq!(a1, a2);
            }
        }

        #[test]
        fn a_live_derived_name_is_refused() {
            let result = decide(
                Intent::Apply,
                Some("2fa-support"),
                None,
                true,
                &["c-2fa-support", "other"],
                false,
            );
            match result {
                Decision::Refuse(reason) => {
                    assert!(reason.contains("c-2fa-support"), "{reason}");
                    assert!(reason.contains('g'), "{reason}");
                }
                other => panic!("expected Refuse, got {other:?}"),
            }
            // A near-miss on either side of the derived name does not collide.
            let go = decide(
                Intent::Apply,
                Some("2fa-support"),
                None,
                true,
                &["c-2fa-support-x", "2fa-support"],
                false,
            );
            assert!(matches!(go, Decision::Go(_)), "expected Go, got {go:?}");
        }

        /// `seam-resilience`: a second press while a launch is already in flight must be
        /// refused rather than run a second `pane split` + `agent start` + `agent prompt`
        /// sequence behind the first. `live_names` is deliberately empty — the poller has not
        /// yet seen the agent, which is the whole point of the window this closes. See
        /// specs/agent-launch/spec.md -> "A second press while a launch is in flight is
        /// refused, not queued".
        #[test]
        fn a_second_press_while_a_launch_is_in_flight_is_refused_not_queued() {
            for intent in [Intent::Apply, Intent::Continue, Intent::Archive] {
                let result = decide(intent, Some("2fa-support"), None, true, &[], true);
                match result {
                    Decision::Refuse(reason) => {
                        assert!(
                            reason.to_lowercase().contains("already running"),
                            "{intent:?}: {reason}"
                        );
                        assert!(reason.to_lowercase().contains("wait"), "{intent:?}: {reason}");
                    }
                    other => panic!("expected Refuse for {intent:?}, got {other:?}"),
                }
                // The same three calls with `in_flight` false return `Go`, so the refusal is
                // caused by the flag and by nothing else in the fixture.
                let go = decide(intent, Some("2fa-support"), None, true, &[], false);
                assert!(
                    matches!(go, Decision::Go(_)),
                    "{intent:?} expected Go with in_flight=false, got {go:?}"
                );
            }
        }

        /// `seam-resilience`: `Focus` is exempt from the in-flight guard — it splits no pane
        /// and starts no agent, so a user waiting through a slow launch can still press `g`.
        /// See specs/agent-launch/spec.md -> "Focus still works while a launch is in flight".
        #[test]
        fn focus_still_works_while_a_launch_is_in_flight() {
            let result = decide(Intent::Focus, None, Some("w8:p3"), true, &[], true);
            assert_eq!(
                result,
                Decision::Go(Request::Focus {
                    pane_id: "w8:p3".to_string()
                })
            );
        }

        #[test]
        fn every_combination_is_total() {
            for in_flight in [false, true] {
                for intent in [
                    Intent::Apply,
                    Intent::Continue,
                    Intent::Archive,
                    Intent::Focus,
                ] {
                    let result = decide(intent, Some(""), Some(""), true, &[""], in_flight);
                    match (intent, in_flight) {
                        (Intent::Focus, _) => {
                            assert_eq!(
                                result,
                                Decision::Go(Request::Focus {
                                    pane_id: String::new()
                                }),
                                "{intent:?} in_flight={in_flight}"
                            );
                        }
                        (_, true) => {
                            assert!(
                                matches!(result, Decision::Refuse(_)),
                                "{intent:?} expected Refuse, got {result:?}"
                            );
                        }
                        (_, false) => {
                            assert_eq!(
                                result,
                                Decision::Go(Request::Launch {
                                    change: String::new(),
                                    agent: "change".to_string(),
                                    intent,
                                }),
                                "{intent:?}"
                            );
                        }
                    }
                }
            }
        }

        #[test]
        fn decide_reads_nothing_but_its_arguments() {
            // No filesystem, process, environment, network, terminal, or clock read: calling
            // it twice with identical arguments, with nothing else touched in between, must
            // yield identical results.
            let a = decide(Intent::Apply, Some("add-auth"), Some("w8:p1"), true, &["x"], false);
            let b = decide(Intent::Apply, Some("add-auth"), Some("w8:p1"), true, &["x"], false);
            assert_eq!(a, b);
        }

        #[test]
        fn the_derived_name_is_state_agent_name() {
            let result = decide(Intent::Continue, Some("2FA_Support!"), None, true, &[], false);
            assert_eq!(
                result,
                Decision::Go(Request::Launch {
                    change: "2FA_Support!".to_string(),
                    agent: crate::state::agent_name("2FA_Support!"),
                    intent: Intent::Continue,
                })
            );
        }
    }

    mod argv {
        use crate::launch::{focus_args, prompt_args, split_args, start_args};

        #[test]
        fn split_args_are_exact() {
            let args = split_args(std::path::Path::new("/tmp/demo-repo"));
            assert_eq!(
                args,
                vec![
                    "pane",
                    "split",
                    "--cwd",
                    "/tmp/demo-repo",
                    "--direction",
                    "right",
                    "--no-focus"
                ]
            );
        }

        #[test]
        fn start_args_are_exact() {
            let args = start_args("c-2fa-support", "codex", "wD:pJ");
            assert_eq!(
                args,
                vec![
                    "agent",
                    "start",
                    "c-2fa-support",
                    "--kind",
                    "codex",
                    "--pane",
                    "wD:pJ"
                ]
            );
        }

        #[test]
        fn prompt_args_are_exact() {
            let args = prompt_args("c-2fa-support", "/opsx:apply 2fa-support");
            assert_eq!(
                args,
                vec![
                    "agent",
                    "prompt",
                    "c-2fa-support",
                    "/opsx:apply 2fa-support"
                ]
            );
        }

        #[test]
        fn focus_args_are_exact() {
            let args = focus_args("wD:pJ");
            assert_eq!(args, vec!["agent", "focus", "wD:pJ"]);
        }

        #[test]
        fn a_non_utf8_repo_root_is_lossy() {
            use std::ffi::OsStr;
            use std::os::unix::ffi::OsStrExt;
            let bytes: &[u8] = b"/tmp/\xffbad";
            let os_str = OsStr::from_bytes(bytes);
            let path = std::path::Path::new(os_str);
            let args = split_args(path);
            assert_eq!(args[3], path.to_string_lossy().into_owned());
            assert!(args[3].contains('\u{FFFD}'));
        }
    }

    mod prompt {
        use crate::launch::{Intent, prompt_text};

        #[test]
        fn each_intent_has_its_own_opsx_command() {
            assert_eq!(
                prompt_text(Intent::Apply, "add-auth"),
                "/opsx:apply add-auth"
            );
            assert_eq!(
                prompt_text(Intent::Continue, "add-auth"),
                "/opsx:continue add-auth"
            );
            assert_eq!(
                prompt_text(Intent::Archive, "add-auth"),
                "/opsx:archive add-auth"
            );
        }

        #[test]
        fn the_prompt_text_is_one_argument() {
            let text = prompt_text(Intent::Apply, "add-auth");
            assert_eq!(text.matches(' ').count(), 1);
            assert!(!text.contains('"'));
        }
    }

    mod pane_id {
        use crate::launch::pane_id;

        #[test]
        fn a_well_formed_envelope_yields_the_pane_id() {
            let text = r#"{"id":"cli:pane:split","result":{"pane":{"agent_status":"unknown","cwd":"/r","pane_id":"wD:pJ","tab_id":"wD:t2","workspace_id":"wD"},"type":"pane_info"}}"#;
            assert_eq!(pane_id(text), Ok("wD:pJ".to_string()));
            let with_trailing = format!("{text}\n  \n");
            assert_eq!(pane_id(&with_trailing), Ok("wD:pJ".to_string()));
        }

        #[test]
        fn every_unusable_payload_is_an_error() {
            let cases = [
                "",
                "not json",
                "[]",
                "{}",
                r#"{"result":{}}"#,
                r#"{"result":{"pane":{}}}"#,
                r#"{"result":{"pane":{"pane_id":7}}}"#,
                r#"{"id":"cli:pane:split","error":{"code":"pane_not_found","message":"no such pane"}}"#,
            ];
            for case in cases {
                assert!(pane_id(case).is_err(), "expected Err for {case:?}");
            }
        }

        #[test]
        fn an_error_envelope_is_reported_by_code_and_message() {
            let text = r#"{"id":"cli:pane:split","error":{"code":"pane_not_found","message":"no such pane"}}"#;
            let err = pane_id(text).unwrap_err();
            assert!(err.contains("pane_not_found"), "{err}");
            assert!(err.contains("no such pane"), "{err}");
        }
    }

    mod run_request {
        use crate::cli::{CliError, FakeCli};
        use crate::launch::{Intent, Outcome, Request, run_request};
        use crate::testutil::{ScratchDir, snapshot};
        use std::path::Path;

        const REPO: &str = "/repo";
        const KIND: &str = "codex";

        fn split_ok(fake: &FakeCli, pane: &str) {
            fake.register_herdr(
                &["pane", "split", "--cwd", REPO, "--direction", "right", "--no-focus"],
                Ok(format!(
                    r#"{{"id":"cli:pane:split","result":{{"pane":{{"pane_id":"{pane}","tab_id":"t","workspace_id":"w"}},"type":"pane_info"}}}}"#
                )),
            );
        }

        fn start_ok(fake: &FakeCli, agent: &str, pane: &str) {
            fake.register_herdr(
                &["agent", "start", agent, "--kind", KIND, "--pane", pane],
                Ok(format!(
                    r#"{{"id":"cli:agent:start","result":{{"agent":{{"name":"{agent}"}},"argv":["{KIND}"],"type":"agent_started"}}}}"#
                )),
            );
        }

        fn prompt_ok(fake: &FakeCli, agent: &str, text: &str) {
            fake.register_herdr(
                &["agent", "prompt", agent, text],
                Ok(
                    r#"{"id":"cli:agent:prompt","result":{"agent":{},"type":"agent_prompted"}}"#
                        .to_string(),
                ),
            );
        }

        fn failed(code: i32, stderr: &str) -> Result<String, CliError> {
            Err(CliError::Failed {
                program: "herdr".to_string(),
                args: Vec::new(),
                code: Some(code),
                stderr: stderr.to_string(),
            })
        }

        #[test]
        fn the_three_calls_appear_in_order_with_the_splits_pane_id() {
            let fake = FakeCli::new();
            split_ok(&fake, "wD:pJ");
            start_ok(&fake, "c-2fa-support", "wD:pJ");
            prompt_ok(&fake, "c-2fa-support", "/opsx:apply 2fa-support");
            let state = ScratchDir::new();

            let outcome = run_request(
                &fake,
                Path::new(REPO),
                KIND,
                Some(state.path()),
                Request::Launch {
                    change: "2fa-support".to_string(),
                    agent: "c-2fa-support".to_string(),
                    intent: Intent::Apply,
                },
            );

            assert_eq!(fake.calls().len(), 3);
            assert_eq!(
                fake.calls()[0].1,
                vec![
                    "pane",
                    "split",
                    "--cwd",
                    REPO,
                    "--direction",
                    "right",
                    "--no-focus"
                ]
            );
            assert_eq!(
                fake.calls()[1].1,
                vec![
                    "agent",
                    "start",
                    "c-2fa-support",
                    "--kind",
                    KIND,
                    "--pane",
                    "wD:pJ"
                ]
            );
            assert_eq!(
                fake.calls()[2].1,
                vec![
                    "agent",
                    "prompt",
                    "c-2fa-support",
                    "/opsx:apply 2fa-support"
                ]
            );
            assert_eq!(
                outcome,
                Outcome {
                    named: Some(("c-2fa-support".to_string(), "2fa-support".to_string())),
                    problems: Vec::new(),
                }
            );

            let mapping = crate::state::read(Some(state.path()));
            assert_eq!(
                mapping.names.get("c-2fa-support"),
                Some(&"2fa-support".to_string())
            );
        }

        #[test]
        fn an_unusable_payload_stops_before_agent_start() {
            let fake = FakeCli::new();
            fake.register_herdr(
                &[
                    "pane",
                    "split",
                    "--cwd",
                    REPO,
                    "--direction",
                    "right",
                    "--no-focus",
                ],
                Ok("{}".to_string()),
            );
            let state = ScratchDir::new();

            let outcome = run_request(
                &fake,
                Path::new(REPO),
                KIND,
                Some(state.path()),
                Request::Launch {
                    // A change/agent pair that genuinely differ, so a `state::record` call
                    // that erroneously ran before this failure would produce a file — an
                    // `add-auth`/`add-auth` pair would pass this assertion vacuously, since
                    // `state::record` itself writes nothing for an unchanged name.
                    change: "2fa-support".to_string(),
                    agent: "c-2fa-support".to_string(),
                    intent: Intent::Apply,
                },
            );

            assert_eq!(fake.calls().len(), 1);
            let Outcome { named, problems } = outcome;
            assert_eq!(named, None);
            assert_eq!(problems.len(), 1, "a reason must be present");
            let problem = &problems[0];
            assert!(problem.contains("result"), "{problem}");
            assert!(
                !state.path().join("agent-names.toml").exists(),
                "a launch stopped before agent start must record no mapping"
            );
        }

        #[test]
        fn a_failed_split_leaves_nothing_behind() {
            let fake = FakeCli::new();
            fake.register_herdr(
                &[
                    "pane",
                    "split",
                    "--cwd",
                    REPO,
                    "--direction",
                    "right",
                    "--no-focus",
                ],
                failed(
                    1,
                    r#"{"error":{"code":"pane_split_failed","message":"no space to split"}}"#,
                ),
            );
            let state = ScratchDir::new();
            let before = snapshot(state.path());

            let outcome = run_request(
                &fake,
                Path::new(REPO),
                KIND,
                Some(state.path()),
                Request::Launch {
                    change: "add-auth".to_string(),
                    agent: "add-auth".to_string(),
                    intent: Intent::Apply,
                },
            );

            assert_eq!(fake.calls().len(), 1);
            let Outcome { named, problems } = outcome;
            assert_eq!(problems.len(), 1, "a reason must be present");
            let problem = &problems[0];
            assert!(problem.contains("pane_split_failed"), "{problem}");
            assert!(problem.contains("no space to split"), "{problem}");
            assert_eq!(named, None);
            assert_eq!(before, snapshot(state.path()));
        }

        #[test]
        fn a_failed_start_leaves_the_pane_and_names_it() {
            let fake = FakeCli::new();
            split_ok(&fake, "wD:pJ");
            fake.register_herdr(
                &["agent", "start", "c-2fa-support", "--kind", KIND, "--pane", "wD:pJ"],
                failed(
                    1,
                    r#"{"error":{"code":"agent_pane_not_found","message":"agent target wD:pJ not found"}}"#,
                ),
            );

            let outcome = run_request(
                &fake,
                Path::new(REPO),
                KIND,
                None,
                Request::Launch {
                    change: "2fa-support".to_string(),
                    agent: "c-2fa-support".to_string(),
                    intent: Intent::Apply,
                },
            );

            assert_eq!(fake.calls().len(), 2);
            assert!(
                !fake
                    .calls()
                    .iter()
                    .any(|(_, args)| args.first().map(String::as_str) == Some("pane")
                        && args.get(1).map(String::as_str) == Some("close")),
                "the plugin must never issue pane close"
            );
            let Outcome { named, problems } = outcome;
            assert_eq!(problems.len(), 1, "a reason must be present");
            let problem = &problems[0];
            assert!(problem.contains("agent_pane_not_found"), "{problem}");
            assert!(problem.contains("c-2fa-support"), "{problem}");
            assert!(problem.contains("wD:pJ"), "{problem}");
            assert_eq!(named, None);
        }

        #[test]
        fn a_failed_prompt_leaves_a_recorded_agent() {
            let fake = FakeCli::new();
            split_ok(&fake, "wD:pJ");
            start_ok(&fake, "c-2fa-support", "wD:pJ");
            fake.register_herdr(
                &[
                    "agent",
                    "prompt",
                    "c-2fa-support",
                    "/opsx:apply 2fa-support",
                ],
                failed(
                    1,
                    r#"{"error":{"code":"agent_blocked","message":"agent is blocked"}}"#,
                ),
            );
            let state = ScratchDir::new();

            let outcome = run_request(
                &fake,
                Path::new(REPO),
                KIND,
                Some(state.path()),
                Request::Launch {
                    change: "2fa-support".to_string(),
                    agent: "c-2fa-support".to_string(),
                    intent: Intent::Apply,
                },
            );

            assert_eq!(fake.calls().len(), 3);
            let mapping = crate::state::read(Some(state.path()));
            assert_eq!(
                mapping.names.get("c-2fa-support"),
                Some(&"2fa-support".to_string())
            );
            let Outcome { named, problems } = outcome;
            assert_eq!(
                named,
                Some(("c-2fa-support".to_string(), "2fa-support".to_string())),
                "the agent exists even though the prompt did not land"
            );
            assert_eq!(problems.len(), 1, "a reason must be present");
            let problem = &problems[0];
            assert!(problem.contains("agent_blocked"), "{problem}");
        }

        #[test]
        fn a_failed_recording_does_not_undo_the_start() {
            let fake = FakeCli::new();
            split_ok(&fake, "wD:pJ");
            start_ok(&fake, "c-2fa-support", "wD:pJ");
            prompt_ok(&fake, "c-2fa-support", "/opsx:apply 2fa-support");
            let scratch = ScratchDir::new();
            let blocked = scratch.path().join("blocked");
            std::fs::write(&blocked, b"not a directory").expect("write blocking file");

            let outcome = run_request(
                &fake,
                Path::new(REPO),
                KIND,
                Some(&blocked),
                Request::Launch {
                    change: "2fa-support".to_string(),
                    agent: "c-2fa-support".to_string(),
                    intent: Intent::Apply,
                },
            );

            assert_eq!(
                fake.calls().len(),
                3,
                "the prompt must still be sent despite the recording failure"
            );
            let Outcome { named, problems } = outcome;
            assert_eq!(
                named,
                Some(("c-2fa-support".to_string(), "2fa-support".to_string()))
            );
            assert_eq!(
                problems.len(),
                1,
                "a reason must be present, naming the recording failure"
            );
            let problem = &problems[0];
            assert!(
                problem.contains(&blocked.display().to_string()),
                "{problem}"
            );
        }

        /// `degraded-states`' repair of row 23: the landed worker's
        /// `match cli.run(&prompt_refs) { Ok(_) => record_problem, Err(err) => Some(...) }`
        /// discarded the record's reason whenever the prompt also failed. RED at `main`: this
        /// test fails there because `outcome.problem` (a single `Option<String>`) can hold only
        /// the prompt's reason, never both.
        #[test]
        fn a_failed_record_and_a_failed_prompt_are_both_reported() {
            let fake = FakeCli::new();
            split_ok(&fake, "wD:pJ");
            start_ok(&fake, "c-2fa-support", "wD:pJ");
            fake.register_herdr(
                &[
                    "agent",
                    "prompt",
                    "c-2fa-support",
                    "/opsx:apply 2fa-support",
                ],
                failed(
                    1,
                    r#"{"error":{"code":"agent_blocked","message":"agent is blocked"}}"#,
                ),
            );
            let scratch = ScratchDir::new();
            let blocked = scratch.path().join("blocked");
            std::fs::write(&blocked, b"not a directory").expect("write blocking file");

            let outcome = run_request(
                &fake,
                Path::new(REPO),
                KIND,
                Some(&blocked),
                Request::Launch {
                    change: "2fa-support".to_string(),
                    agent: "c-2fa-support".to_string(),
                    intent: Intent::Apply,
                },
            );

            assert_eq!(fake.calls().len(), 3);
            let Outcome { named, problems } = outcome;
            assert_eq!(
                named,
                Some(("c-2fa-support".to_string(), "2fa-support".to_string())),
                "the agent is running and must stay attributable"
            );
            assert_eq!(
                problems.len(),
                2,
                "both the record failure and the prompt failure must be reported: {problems:?}"
            );
            assert!(
                problems[0].contains(&blocked.display().to_string()),
                "the record failure must be first: {problems:?}"
            );
            assert!(
                problems[1].contains("agent_blocked"),
                "the prompt failure must be second: {problems:?}"
            );
        }

        #[test]
        fn a_name_past_the_cap_is_truncated_hashed_and_recorded() {
            let change = "a-very-long-change-name-that-exceeds-the-cap";
            assert_eq!(change.len(), 44);
            let derived = crate::state::agent_name(change);
            assert!(derived.len() <= 32);
            assert!(derived.chars().next().unwrap().is_ascii_lowercase());
            assert!(
                derived
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
            );
            assert_eq!(derived, crate::state::agent_name(change));

            let fake = FakeCli::new();
            split_ok(&fake, "wD:pJ");
            fake.register_herdr(
                &["agent", "start", &derived, "--kind", KIND, "--pane", "wD:pJ"],
                Ok(format!(
                    r#"{{"id":"cli:agent:start","result":{{"agent":{{"name":"{derived}"}},"type":"agent_started"}}}}"#
                )),
            );
            let text = format!("/opsx:apply {change}");
            fake.register_herdr(&["agent", "prompt", &derived, &text], Ok(String::new()));
            let state = ScratchDir::new();

            let outcome = run_request(
                &fake,
                Path::new(REPO),
                KIND,
                Some(state.path()),
                Request::Launch {
                    change: change.to_string(),
                    agent: derived.clone(),
                    intent: Intent::Apply,
                },
            );

            assert_eq!(
                fake.calls()[1].1[2],
                derived,
                "the agent start argument must be the derived name"
            );
            let mapping = crate::state::read(Some(state.path()));
            assert_eq!(mapping.names.get(&derived), Some(&change.to_string()));
            assert_eq!(
                outcome,
                Outcome {
                    named: Some((derived, change.to_string())),
                    problems: Vec::new(),
                }
            );
        }

        #[test]
        fn a_legal_but_derived_name_is_recorded_too() {
            let change = "2FA_Support!";
            let derived = crate::state::agent_name(change);
            assert_eq!(derived, "c-2fa_support");

            let fake = FakeCli::new();
            split_ok(&fake, "wD:pJ");
            start_ok(&fake, &derived, "wD:pJ");
            let text = format!("/opsx:apply {change}");
            prompt_ok(&fake, &derived, &text);
            let state = ScratchDir::new();

            let outcome = run_request(
                &fake,
                Path::new(REPO),
                KIND,
                Some(state.path()),
                Request::Launch {
                    change: change.to_string(),
                    agent: derived.clone(),
                    intent: Intent::Apply,
                },
            );

            let mapping = crate::state::read(Some(state.path()));
            assert_eq!(mapping.names.get(&derived), Some(&change.to_string()));
            assert_eq!(
                outcome,
                Outcome {
                    named: Some((derived, change.to_string())),
                    problems: Vec::new(),
                }
            );
        }

        #[test]
        fn an_unchanged_name_writes_no_file() {
            let fake = FakeCli::new();
            split_ok(&fake, "wD:pJ");
            start_ok(&fake, "add-auth", "wD:pJ");
            prompt_ok(&fake, "add-auth", "/opsx:apply add-auth");
            let state = ScratchDir::new();
            let before = snapshot(state.path());

            let outcome = run_request(
                &fake,
                Path::new(REPO),
                KIND,
                Some(state.path()),
                Request::Launch {
                    change: "add-auth".to_string(),
                    agent: "add-auth".to_string(),
                    intent: Intent::Apply,
                },
            );

            assert_eq!(before, snapshot(state.path()));
            assert_eq!(
                outcome,
                Outcome {
                    named: Some(("add-auth".to_string(), "add-auth".to_string())),
                    problems: Vec::new(),
                }
            );
        }

        #[test]
        fn a_collision_herdr_sees_is_reported_with_its_reason() {
            let fake = FakeCli::new();
            split_ok(&fake, "wD:pJ");
            fake.register_herdr(
                &["agent", "start", "c-2fa-support", "--kind", KIND, "--pane", "wD:pJ"],
                failed(
                    1,
                    r#"{"error":{"code":"agent_name_taken","message":"agent name c-2fa-support is already used; candidates: c-2fa-support-2"}}"#,
                ),
            );

            let outcome = run_request(
                &fake,
                Path::new(REPO),
                KIND,
                None,
                Request::Launch {
                    change: "2fa-support".to_string(),
                    agent: "c-2fa-support".to_string(),
                    intent: Intent::Apply,
                },
            );

            assert_eq!(fake.calls().len(), 2);
            assert!(
                !fake
                    .calls()
                    .iter()
                    .any(|(_, args)| args.get(1).map(String::as_str) == Some("close")),
                "no pane close entry"
            );
            let Outcome { named: _, problems } = outcome;
            assert_eq!(problems.len(), 1, "a reason must be present");
            let problem = &problems[0];
            assert!(problem.contains("agent_name_taken"), "{problem}");
        }
    }

    /// `seam-resilience`: `herdr_reason`'s third `CliError` arm names the command and the
    /// deadline rather than failing to compile.
    mod herdr_reason {
        use crate::cli::CliError;

        #[test]
        fn a_timed_out_call_names_the_deadline() {
            let err = CliError::TimedOut {
                args: vec!["agent".to_string(), "list".to_string()],
                after: std::time::Duration::from_secs(60),
            };
            let reason = super::super::herdr_reason(&err);
            assert!(reason.contains("herdr"), "{reason}");
            assert!(reason.contains("timed out"), "{reason}");
            assert!(reason.contains("60s"), "{reason}");
        }
    }

    /// `seam-resilience`: the settle budget, pinned on `watch::DEBOUNCE`'s and
    /// `agents::POLL_INTERVAL`'s named-constant-plus-assertion terms. See
    /// specs/agent-launch/spec.md -> "The settle budget is a named constant and is asserted".
    mod budget {
        #[test]
        fn settle_budget_is_thirty_five_seconds_and_sits_between_the_two_deadlines() {
            assert_eq!(
                super::super::SETTLE_BUDGET,
                std::time::Duration::from_secs(35)
            );
            // Above the measured thirty seconds `herdr agent start` spends waiting for
            // interactive readiness, and below `cli::RUN_DEADLINE`, so the budget cannot
            // silently drift below the wait it exists to cover.
            assert!(super::super::SETTLE_BUDGET > std::time::Duration::from_secs(30));
            assert!(super::super::SETTLE_BUDGET < crate::cli::RUN_DEADLINE);
        }
    }

    mod focus {
        use crate::cli::FakeCli;
        use crate::launch::{Outcome, Request, run_request};
        use std::path::Path;

        #[test]
        fn a_focus_request_is_one_call() {
            let fake = FakeCli::new();
            fake.register_herdr(
                &["agent", "focus", "wD:pJ"],
                Ok(r#"{"id":"cli:agent:focus","result":{}}"#.to_string()),
            );

            let outcome = run_request(
                &fake,
                Path::new("/repo"),
                "codex",
                None,
                Request::Focus {
                    pane_id: "wD:pJ".to_string(),
                },
            );

            assert_eq!(
                fake.calls(),
                vec![(
                    crate::cli::Program::Herdr,
                    vec![
                        "agent".to_string(),
                        "focus".to_string(),
                        "wD:pJ".to_string()
                    ]
                )]
            );
            assert_eq!(
                outcome,
                Outcome {
                    named: None,
                    problems: Vec::new(),
                }
            );
        }

        #[test]
        fn a_failed_focus_is_reported() {
            let fake = FakeCli::new();
            fake.register_herdr(
                &["agent", "focus", "wD:pJ"],
                Err(crate::cli::CliError::Failed {
                    program: "herdr".to_string(),
                    args: Vec::new(),
                    code: Some(1),
                    stderr: r#"{"error":{"code":"agent_not_found","message":"no such agent"}}"#
                        .to_string(),
                }),
            );

            let outcome = run_request(
                &fake,
                Path::new("/repo"),
                "codex",
                None,
                Request::Focus {
                    pane_id: "wD:pJ".to_string(),
                },
            );

            let Outcome { named, problems } = outcome;
            assert_eq!(named, None);
            assert_eq!(problems.len(), 1, "a reason must be present");
            let problem = &problems[0];
            assert!(problem.contains("agent_not_found"), "{problem}");
        }
    }

    mod seam {
        use crate::cli::FakeCli;
        use crate::launch::{Intent, Launcher, Outcome, Request};
        use crate::testutil::{ScratchDir, snapshot};
        use std::path::PathBuf;
        use std::time::{Duration, Instant};

        /// A launcher seam for tests, on `agents::poller_for_test`'s and
        /// `refresh::worker_for_test`'s exact terms: the second element is the worker's result
        /// `Receiver`, handed to the test directly, so this seam's own `drain` always answers
        /// `None`; the third receives from a channel whose `Sender` the worker thread owns and
        /// drops only when its body returns.
        fn launcher_for_test(
            cli: std::sync::Arc<dyn crate::cli::HerdrCli>,
        ) -> (
            Box<dyn Launcher>,
            std::sync::mpsc::Receiver<Outcome>,
            std::sync::mpsc::Receiver<()>,
        ) {
            let (request_tx, request_rx) = std::sync::mpsc::channel::<Request>();
            let (result_tx, result_rx) = std::sync::mpsc::channel::<Outcome>();
            let (exit_tx, exit_rx) = std::sync::mpsc::channel::<()>();
            std::thread::spawn(move || {
                let _exit_tx = exit_tx;
                super::super::worker_body(
                    cli,
                    PathBuf::from("/repo"),
                    "codex".to_string(),
                    None,
                    request_rx,
                    result_tx,
                );
            });
            (Box::new(TestLauncher { request_tx }), result_rx, exit_rx)
        }

        struct TestLauncher {
            request_tx: std::sync::mpsc::Sender<Request>,
        }

        impl Launcher for TestLauncher {
            fn request(&mut self, request: Request) {
                let _ = self.request_tx.send(request);
            }

            fn drain(&mut self) -> Option<Outcome> {
                None
            }
        }

        #[test]
        fn the_inert_launcher_answers_nothing() {
            let mut launcher = super::super::none();
            launcher.request(Request::Launch {
                change: "x".to_string(),
                agent: "x".to_string(),
                intent: Intent::Apply,
            });
            for _ in 0..10 {
                assert_eq!(launcher.drain(), None);
            }

            // `seam-resilience`: `settle` is a deadline-bounded poll of `drain`, never a fixed
            // sleep, so it returns as soon as its own budget elapses rather than blocking past
            // it - which is what keeps a no-repository pane from paying `SETTLE_BUDGET` on
            // quit if it were ever called there. A short budget stands in for the real
            // constant so the test itself stays fast.
            let budget = Duration::from_millis(50);
            let started = Instant::now();
            let outcome = super::super::settle(launcher.as_mut(), budget);
            assert_eq!(outcome, None);
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "settle over the inert launcher must return once its budget elapses, took {:?}",
                started.elapsed()
            );
        }

        /// A `HerdrCli` whose first call blocks until the test releases it, on a channel it
        /// owns — the only synchronisation this change's tests own, and the reason none of
        /// them sleeps. Every call after the first proceeds immediately, since `.take()`
        /// yields `None` from then on.
        struct GatedCli {
            release_rx: std::sync::Mutex<Option<std::sync::mpsc::Receiver<()>>>,
        }

        impl crate::cli::HerdrCli for GatedCli {
            fn run(&self, args: &[&str]) -> Result<String, crate::cli::CliError> {
                if let Some(rx) = self.release_rx.lock().expect("gate mutex poisoned").take() {
                    let _ = rx.recv();
                }
                match (args.first().copied(), args.get(1).copied()) {
                    (Some("pane"), Some("split")) => Ok(r#"{"id":"cli:pane:split","result":{"pane":{"pane_id":"wD:pJ","tab_id":"t","workspace_id":"w"}},"type":"pane_info"}"#.to_string()),
                    _ => Ok(String::new()),
                }
            }
        }

        #[test]
        fn the_real_launcher_answers_on_a_later_drain() {
            let (release_tx, release_rx) = std::sync::mpsc::channel();
            let cli: std::sync::Arc<dyn crate::cli::HerdrCli> = std::sync::Arc::new(GatedCli {
                release_rx: std::sync::Mutex::new(Some(release_rx)),
            });
            let mut launcher =
                super::super::start(cli, PathBuf::from("/repo"), "codex".to_string(), None);
            launcher.request(Request::Launch {
                change: "add-auth".to_string(),
                agent: "add-auth".to_string(),
                intent: Intent::Apply,
            });

            // The drain immediately after the request must answer None — the worker is still
            // blocked on the gate, so the work is on the worker thread, not the caller's.
            assert_eq!(launcher.drain(), None);

            release_tx.send(()).expect("release the gate");

            let deadline = Instant::now() + Duration::from_secs(10);
            let mut outcome = None;
            while Instant::now() < deadline {
                if let Some(o) = launcher.drain() {
                    outcome = Some(o);
                    break;
                }
                std::thread::yield_now();
            }
            let outcome = outcome.expect("a later drain must answer within 10s");
            assert_eq!(outcome.problems, Vec::<String>::new());
        }

        #[test]
        fn dropping_the_launcher_stops_its_worker() {
            let cli: std::sync::Arc<dyn crate::cli::HerdrCli> = std::sync::Arc::new(FakeCli::new());
            let (launcher, _result_rx, exit_rx) = launcher_for_test(cli);
            drop(launcher);

            match exit_rx.recv_timeout(Duration::from_secs(10)) {
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {}
                other => panic!(
                    "the worker did not return within 10s after the launcher was dropped: {other:?}"
                ),
            }
        }

        /// `seam-resilience`: on exactly `agents::RealAgentPoll`'s model. Constructed directly
        /// with the worker's result `Sender` and request `Receiver` already dropped, standing
        /// in for a worker that has exited, with no thread involved - the same "real channels,
        /// no thread" shape `refresh::a_dead_refresh_worker_is_reported_once...` uses. Before
        /// this fix, a dead worker was indistinguishable from a working one for the rest of the
        /// session: `request` silently discarded the `SendError` and `drain`'s `try_recv`
        /// collapsed `Disconnected` into `None`. See specs/agent-launch/spec.md -> "A dead
        /// launcher worker is reported once and then stops being reported".
        #[test]
        fn a_dead_launcher_worker_is_reported_once_and_then_stops_being_reported() {
            let (request_tx, request_rx) = std::sync::mpsc::channel::<Request>();
            let (result_tx, result_rx) = std::sync::mpsc::channel::<Outcome>();
            drop(request_rx);
            drop(result_tx);
            let mut launcher: Box<dyn Launcher> = Box::new(super::super::RealLauncher {
                request_tx,
                result_rx,
                dead: false,
                pending_death: false,
            });

            let request = || Request::Launch {
                change: "add-auth".to_string(),
                agent: "add-auth".to_string(),
                intent: Intent::Apply,
            };

            launcher.request(request());
            match launcher.drain() {
                Some(Outcome { named, problems }) => {
                    assert_eq!(named, None);
                    assert_eq!(problems.len(), 1, "a reason must be present: {problems:?}");
                    assert!(problems[0].contains("launch"), "{}", problems[0]);
                }
                None => panic!("the first drain after the worker died must report it"),
            }

            launcher.request(request());
            assert_eq!(
                launcher.drain(),
                None,
                "a dead launcher degrades to silence, not a growing list"
            );

            launcher.request(request());
            assert_eq!(launcher.drain(), None);
        }

        /// The other route to the same latch: `request` never fails here (the request
        /// channel is held open), so `drain`'s own `try_recv` is what first observes
        /// `Disconnected` - the sibling branch to the one above, which detects the death
        /// through a failed `send` instead. Both must latch identically.
        #[test]
        fn a_disconnected_result_channel_is_reported_once_and_then_stops_being_reported() {
            let (request_tx, _request_rx) = std::sync::mpsc::channel::<Request>();
            let (result_tx, result_rx) = std::sync::mpsc::channel::<Outcome>();
            drop(result_tx);
            let mut launcher: Box<dyn Launcher> = Box::new(super::super::RealLauncher {
                request_tx,
                result_rx,
                dead: false,
                pending_death: false,
            });

            match launcher.drain() {
                Some(Outcome { named, problems }) => {
                    assert_eq!(named, None);
                    assert_eq!(problems.len(), 1, "a reason must be present: {problems:?}");
                    assert!(problems[0].contains("launch"), "{}", problems[0]);
                }
                None => panic!(
                    "the first drain after the result channel disconnects must report it"
                ),
            }
            assert_eq!(
                launcher.drain(),
                None,
                "a dead launcher degrades to silence, not a growing list"
            );
        }

        /// `seam-resilience`: `settle`'s happy path — a launch that answers before the budget
        /// elapses is returned from `settle` itself, not merely from a raw `drain` loop the
        /// test drives by hand. Uses a real worker thread, on this file's own "real thread,
        /// recording fake" terms.
        #[test]
        fn settle_returns_the_outcome_once_the_worker_answers() {
            let fake = FakeCli::new();
            fake.register_herdr(
                &["agent", "focus", "wD:pJ"],
                Ok(r#"{"id":"cli:agent:focus","result":{}}"#.to_string()),
            );
            let cli: std::sync::Arc<dyn crate::cli::HerdrCli> = std::sync::Arc::new(fake);
            let mut launcher =
                super::super::start(cli, PathBuf::from("/repo"), "codex".to_string(), None);
            launcher.request(Request::Focus {
                pane_id: "wD:pJ".to_string(),
            });

            let outcome = super::super::settle(launcher.as_mut(), Duration::from_secs(10))
                .expect("settle must return the outcome once the worker answers");
            assert_eq!(
                outcome,
                Outcome {
                    named: None,
                    problems: Vec::new(),
                }
            );
        }

        #[test]
        fn the_launcher_writes_only_the_state_directory() {
            let fake = FakeCli::new();
            fake.register_herdr(
                &["pane", "split", "--cwd", "/repo", "--direction", "right", "--no-focus"],
                Ok(r#"{"id":"cli:pane:split","result":{"pane":{"pane_id":"wD:pJ","tab_id":"t","workspace_id":"w"}},"type":"pane_info"}"#.to_string()),
            );
            fake.register_herdr(
                &[
                    "agent",
                    "start",
                    "c-2fa-support",
                    "--kind",
                    "codex",
                    "--pane",
                    "wD:pJ",
                ],
                Ok(
                    r#"{"id":"cli:agent:start","result":{"agent":{},"type":"agent_started"}}"#
                        .to_string(),
                ),
            );
            fake.register_herdr(
                &[
                    "agent",
                    "prompt",
                    "c-2fa-support",
                    "/opsx:apply 2fa-support",
                ],
                Ok(String::new()),
            );
            let cli: std::sync::Arc<dyn crate::cli::HerdrCli> = std::sync::Arc::new(fake);

            // Stands in for the repository: the launcher never touches it on disk, only
            // passes its path as --cwd.
            let repo_scratch = ScratchDir::new();
            let state = ScratchDir::new();
            let before_repo = snapshot(repo_scratch.path());
            let before_state = snapshot(state.path());

            let mut launcher = super::super::start(
                cli,
                PathBuf::from("/repo"),
                "codex".to_string(),
                Some(state.path().to_path_buf()),
            );
            launcher.request(Request::Launch {
                change: "2fa-support".to_string(),
                agent: "c-2fa-support".to_string(),
                intent: Intent::Apply,
            });

            let deadline = Instant::now() + Duration::from_secs(10);
            let mut outcome = None;
            while Instant::now() < deadline {
                if let Some(o) = launcher.drain() {
                    outcome = Some(o);
                    break;
                }
                std::thread::yield_now();
            }
            let outcome = outcome.expect("the launch must complete within 10s");
            assert_eq!(outcome.problems, Vec::<String>::new());

            let after_repo = snapshot(repo_scratch.path());
            assert_eq!(
                before_repo, after_repo,
                "the repository must never be written to"
            );

            let after_state = snapshot(state.path());
            assert_ne!(before_state, after_state);
            let mut names: Vec<_> = std::fs::read_dir(state.path())
                .expect("read state dir")
                .map(|e| e.expect("entry").file_name())
                .collect();
            names.sort();
            assert_eq!(names, vec![std::ffi::OsString::from("agent-names.toml")]);
        }

        /// The compile-time companion for this module's own no-`Default` type: exhaustive
        /// destructuring, no `..` rest, so a field added to `Outcome` fails to compile here
        /// rather than defaulting silently.
        #[test]
        fn outcome_destructures_exhaustively_with_no_default() {
            let outcome = Outcome {
                named: None,
                problems: Vec::new(),
            };
            let Outcome { named, problems } = outcome;
            assert_eq!(named, None);
            assert_eq!(problems, Vec::<String>::new());
        }

        /// An exhaustive match with no wildcard arm: a new `Intent` variant fails to compile
        /// here.
        #[test]
        fn intent_match_is_exhaustive() {
            fn assert_known(intent: Intent) {
                match intent {
                    Intent::Apply | Intent::Continue | Intent::Archive | Intent::Focus => {}
                }
            }
            for intent in [
                Intent::Apply,
                Intent::Continue,
                Intent::Archive,
                Intent::Focus,
            ] {
                assert_known(intent);
            }
        }

        /// An exhaustive match with no wildcard arm and no `..` inside either variant's
        /// pattern: a new `Request` variant, or a field added to either one, fails to compile
        /// here.
        #[test]
        fn request_match_is_exhaustive() {
            fn assert_known(request: &Request) {
                match request {
                    Request::Launch {
                        change: _,
                        agent: _,
                        intent: _,
                    } => {}
                    Request::Focus { pane_id: _ } => {}
                }
            }
            assert_known(&Request::Launch {
                change: "x".to_string(),
                agent: "x".to_string(),
                intent: Intent::Apply,
            });
            assert_known(&Request::Focus {
                pane_id: "x".to_string(),
            });
        }
    }
}
