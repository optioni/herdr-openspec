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
/// state, spawns nothing, and never panics for any combination of arguments. The order is
/// exactly: an unreachable socket makes every intent inert; `Focus` resolves against `pane`
/// alone and returns without consulting `change`; no selected change means no launch; a
/// derived name already live in the session is refused before any request is produced; and
/// otherwise the request goes ahead. See `specs/agent-launch/spec.md` -> "The launch decision
/// is a pure, total function that refuses before it reaches Herdr".
pub fn decide(
    intent: Intent,
    change: Option<&str>,
    pane: Option<&str>,
    reachable: bool,
    live_names: &[&str],
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
                    problem: None,
                },
                Err(err) => Outcome {
                    named: None,
                    problem: Some(herdr_reason(&err)),
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
                        problem: Some(herdr_reason(&err)),
                    };
                }
            };
            let pane = match pane_id(&payload) {
                Ok(p) => p,
                Err(reason) => {
                    return Outcome {
                        named: None,
                        problem: Some(reason),
                    };
                }
            };

            let start = start_args(&agent, kind, &pane);
            let start_refs: Vec<&str> = start.iter().map(String::as_str).collect();
            if let Err(err) = cli.run(&start_refs) {
                return Outcome {
                    named: None,
                    problem: Some(format!(
                        "{} (agent {agent}, pane {pane})",
                        herdr_reason(&err)
                    )),
                };
            }

            // `state::record` runs between `agent start` and `agent prompt` (design.md ->
            // Decisions 12): a failure here is reported but does not undo the start, and the
            // prompt is still sent.
            let record_problem = crate::state::record(state_dir, &agent, &change)
                .err()
                .map(|e| e.to_string());

            let prompt_text_value = prompt_text(intent, &change);
            let prompt = prompt_args(&agent, &prompt_text_value);
            let prompt_refs: Vec<&str> = prompt.iter().map(String::as_str).collect();
            let problem = match cli.run(&prompt_refs) {
                Ok(_) => record_problem,
                Err(err) => Some(herdr_reason(&err)),
            };
            Outcome {
                named: Some((agent, change)),
                problem,
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
                let result = decide(intent, Some("add-auth"), Some("w8:p3"), false, &[]);
                assert_eq!(result, Decision::Nothing, "{intent:?}");
            }
        }

        #[test]
        fn no_change_means_no_launch_and_no_agent_means_no_focus() {
            for intent in [Intent::Apply, Intent::Continue, Intent::Archive] {
                assert_eq!(
                    decide(intent, None, None, true, &[]),
                    Decision::Nothing,
                    "{intent:?}"
                );
            }
            assert_eq!(
                decide(Intent::Focus, None, None, true, &[]),
                Decision::Nothing
            );
            assert_eq!(
                decide(Intent::Focus, None, Some("w8:p3"), true, &[]),
                Decision::Go(Request::Focus {
                    pane_id: "w8:p3".to_string()
                })
            );
        }

        #[test]
        fn each_intent_carries_its_own_change_and_name() {
            let mut results = Vec::new();
            for intent in [Intent::Apply, Intent::Continue, Intent::Archive] {
                let result = decide(intent, Some("2fa-support"), None, true, &[]);
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
            );
            assert!(matches!(go, Decision::Go(_)), "expected Go, got {go:?}");
        }

        #[test]
        fn every_combination_is_total() {
            for intent in [
                Intent::Apply,
                Intent::Continue,
                Intent::Archive,
                Intent::Focus,
            ] {
                let result = decide(intent, Some(""), Some(""), true, &[""]);
                if intent == Intent::Focus {
                    assert_eq!(
                        result,
                        Decision::Go(Request::Focus {
                            pane_id: String::new()
                        })
                    );
                } else {
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

        #[test]
        fn decide_reads_nothing_but_its_arguments() {
            // No filesystem, process, environment, network, terminal, or clock read: calling
            // it twice with identical arguments, with nothing else touched in between, must
            // yield identical results.
            let a = decide(Intent::Apply, Some("add-auth"), Some("w8:p1"), true, &["x"]);
            let b = decide(Intent::Apply, Some("add-auth"), Some("w8:p1"), true, &["x"]);
            assert_eq!(a, b);
        }

        #[test]
        fn the_derived_name_is_state_agent_name() {
            let result = decide(Intent::Continue, Some("2FA_Support!"), None, true, &[]);
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
                    problem: None,
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
            let Outcome { named, problem } = outcome;
            assert_eq!(named, None);
            let problem = problem.expect("a reason must be present");
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
            let Outcome { named, problem } = outcome;
            let problem = problem.expect("a reason must be present");
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
            let Outcome { named, problem } = outcome;
            let problem = problem.expect("a reason must be present");
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
            let Outcome { named, problem } = outcome;
            assert_eq!(
                named,
                Some(("c-2fa-support".to_string(), "2fa-support".to_string())),
                "the agent exists even though the prompt did not land"
            );
            let problem = problem.expect("a reason must be present");
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
            let Outcome { named, problem } = outcome;
            assert_eq!(
                named,
                Some(("c-2fa-support".to_string(), "2fa-support".to_string()))
            );
            let problem = problem.expect("a reason must be present, naming the recording failure");
            assert!(
                problem.contains(&blocked.display().to_string()),
                "{problem}"
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
                    problem: None,
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
                    problem: None,
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
                    problem: None,
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
            let Outcome { named: _, problem } = outcome;
            let problem = problem.expect("a reason must be present");
            assert!(problem.contains("agent_name_taken"), "{problem}");
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
                    problem: None,
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

            let Outcome { named, problem } = outcome;
            assert_eq!(named, None);
            let problem = problem.expect("a reason must be present");
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
            assert_eq!(outcome.problem, None);
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
            assert_eq!(outcome.problem, None);

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
                problem: None,
            };
            let Outcome { named, problem } = outcome;
            assert_eq!(named, None);
            assert_eq!(problem, None);
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
