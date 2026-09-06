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
                        ..
                    }),
                    Decision::Go(Request::Launch {
                        change: c2,
                        agent: a2,
                        ..
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
}
