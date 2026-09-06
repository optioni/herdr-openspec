//! The `open` and `open-tab` subcommands: open or focus the OpenSpec dashboard for the
//! workspace the action was invoked from. The crate's third `cli::HerdrCli` consumer,
//! after `src/agents.rs` and `src/launch.rs` — see `openspec/changes/plugin-actions/design.md`
//! -> Boundaries. Starts no thread and holds no state: unlike `launch`, `agents`, `watch`,
//! and `refresh`, this is a one-shot process that exits, never a collaborator beside a
//! render loop.
//!
//! `context` reads Herdr's injected invocation context through one injected lookup;
//! `existing_pane` decides whether a listed pane is this workspace's dashboard;
//! `open_args`/`focus_args` are the exact Herdr argument vectors; `run` is the impure
//! driver over `&dyn HerdrCli`; `placement_for`/`report_output` are the two pure
//! decisions `src/main.rs` would otherwise make itself (design.md -> Decision 10); and
//! `run_from_env` is the one-line production binding, on `cli::worker_cli_from_env`'s
//! terms.

use std::path::Path;

/// The dashboard's pane title, and the matcher's only ownership signal — `herdr pane
/// list` exposes no plugin ownership field. Both `[[panes]]` entries in
/// `herdr-plugin.toml` carry this same title (design.md -> Decision 3).
pub const DASHBOARD_LABEL: &str = "OpenSpec";
/// The manifest pane id `open` targets.
pub const DASHBOARD_ENTRYPOINT: &str = "dashboard";
/// The manifest pane id `open-tab` targets.
pub const DASHBOARD_TAB_ENTRYPOINT: &str = "dashboard-tab";

/// The invocation context read from Herdr's injected environment. See
/// `specs/pane-open/spec.md` -> "The invocation context is read from Herdr's injected
/// environment through one injected lookup". No `Default`, anywhere in the crate; every
/// construction and destructuring names every field, with no `..` rest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub plugin_id: String,
    pub workspace_id: String,
    pub workspace_cwd: Option<String>,
    pub focused_pane_id: Option<String>,
}

/// Read a string field from a parsed `HERDR_PLUGIN_CONTEXT_JSON` object, blank-filtered
/// on `config::non_blank`'s terms. `obj` is `None` when the variable was absent, was not
/// valid JSON, or was valid JSON that was not an object — every case folds to "no JSON
/// context", never a failure (`specs/pane-open/spec.md` -> "Unreadable context JSON falls
/// back rather than failing").
fn json_field(
    obj: &Option<serde_json::Map<String, serde_json::Value>>,
    key: &str,
) -> Option<String> {
    let raw = obj
        .as_ref()
        .and_then(|o| o.get(key))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    crate::config::non_blank(raw)
}

/// Read the invocation context purely from an injected lookup, exactly as
/// `config::config_dir` and `state::state_dir` do. Performs no other I/O. Only an absent
/// **workspace id** is fatal — see `specs/pane-open/spec.md` for the exact field-by-field
/// fallback table.
pub fn context(env: &dyn Fn(&str) -> Option<String>) -> Result<Context, String> {
    let json_obj: Option<serde_json::Map<String, serde_json::Value>> = env(
        "HERDR_PLUGIN_CONTEXT_JSON",
    )
    .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
    .and_then(|v| v.as_object().cloned());

    let plugin_id = crate::config::non_blank(env("HERDR_PLUGIN_ID"))
        .unwrap_or_else(|| "herdr-openspec".to_string());

    let workspace_id = json_field(&json_obj, "workspace_id")
        .or_else(|| crate::config::non_blank(env("HERDR_WORKSPACE_ID")))
        .ok_or_else(|| {
            "HERDR_WORKSPACE_ID is not set - this command must be invoked from Herdr".to_string()
        })?;

    let workspace_cwd = json_field(&json_obj, "workspace_cwd")
        .or_else(|| json_field(&json_obj, "focused_pane_cwd"));

    let focused_pane_id =
        json_field(&json_obj, "focused_pane_id").or_else(|| crate::config::non_blank(env("HERDR_PANE_ID")));

    Ok(Context {
        plugin_id,
        workspace_id,
        workspace_cwd,
        focused_pane_id,
    })
}

/// Decide whether `listing` (a `herdr pane list` payload) names an existing dashboard
/// pane for this workspace, and if so, its pane id. A listed pane counts when **all
/// four** hold: it carries a string `pane_id`, its `label` equals [`DASHBOARD_LABEL`],
/// its `workspace_id` equals `workspace_id`, and — when `cwd` is `Some` — its `cwd` is
/// `std::path::Path`-equal to it (component-wise, no canonicalization: both strings
/// originate from Herdr itself, so a `stat` would buy nothing). When `cwd` is `None` the
/// cwd test is skipped rather than matching every pane. The first match in the list's
/// own order wins. `Err` carries a reason when `listing` is not JSON or carries no
/// `result.panes` array; the caller degrades on it rather than failing
/// (`specs/pane-open/spec.md` -> Decision 4).
pub fn existing_pane(
    listing: &str,
    workspace_id: &str,
    cwd: Option<&str>,
) -> Result<Option<String>, String> {
    let value: serde_json::Value =
        serde_json::from_str(listing).map_err(|e| format!("pane list payload is not valid JSON: {e}"))?;
    let panes = value
        .get("result")
        .and_then(|r| r.get("panes"))
        .and_then(|p| p.as_array())
        .ok_or_else(|| "pane list payload has no \"result\".\"panes\" array".to_string())?;

    for pane in panes {
        let Some(obj) = pane.as_object() else {
            continue;
        };
        let Some(pane_id) = obj.get("pane_id").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(label) = obj.get("label").and_then(|v| v.as_str()) else {
            continue;
        };
        if label != DASHBOARD_LABEL {
            continue;
        }
        let Some(pane_workspace) = obj.get("workspace_id").and_then(|v| v.as_str()) else {
            continue;
        };
        if pane_workspace != workspace_id {
            continue;
        }
        if let Some(want_cwd) = cwd {
            let pane_cwd = obj.get("cwd").and_then(|v| v.as_str());
            match pane_cwd {
                Some(pc) if Path::new(pc) == Path::new(want_cwd) => {}
                _ => continue,
            }
        }
        return Ok(Some(pane_id.to_string()));
    }
    Ok(None)
}

/// Which kind of pane an `open`/`open-tab` invocation targets. See
/// `specs/pane-open/spec.md` -> "`open` splits the pane the action was invoked from" and
/// -> "`open-tab` opens a tab in the invoking workspace".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    Split,
    Tab,
}

/// The exact `herdr plugin pane open` argument vector for `placement`, built purely from
/// `ctx`. `--target-pane` is omitted (split only) when `ctx.focused_pane_id` is absent,
/// and `--cwd` is omitted (both) when `ctx.workspace_cwd` is absent — the pane then
/// inherits the plugin root, a degraded view rather than a refusal. Never issues
/// `--workspace` for a split or `--target-pane`/`--direction` for a tab: measured against
/// Herdr 0.8.2, a split targets an existing pane and a tab needs none.
pub fn open_args(placement: Placement, ctx: &Context) -> Vec<String> {
    let mut argv = vec![
        "plugin".to_string(),
        "pane".to_string(),
        "open".to_string(),
        "--plugin".to_string(),
        ctx.plugin_id.clone(),
    ];
    match placement {
        Placement::Split => {
            argv.push("--entrypoint".to_string());
            argv.push(DASHBOARD_ENTRYPOINT.to_string());
            argv.push("--placement".to_string());
            argv.push("split".to_string());
            argv.push("--direction".to_string());
            argv.push("right".to_string());
            if let Some(pane) = &ctx.focused_pane_id {
                argv.push("--target-pane".to_string());
                argv.push(pane.clone());
            }
        }
        Placement::Tab => {
            argv.push("--entrypoint".to_string());
            argv.push(DASHBOARD_TAB_ENTRYPOINT.to_string());
            argv.push("--placement".to_string());
            argv.push("tab".to_string());
            argv.push("--workspace".to_string());
            argv.push(ctx.workspace_id.clone());
        }
    }
    if let Some(cwd) = &ctx.workspace_cwd {
        argv.push("--cwd".to_string());
        argv.push(cwd.clone());
    }
    argv.push("--focus".to_string());
    argv
}

/// The exact `herdr plugin pane focus` argument vector: one call, naming `pane_id`.
pub fn focus_args(pane_id: &str) -> Vec<String> {
    vec![
        "plugin".to_string(),
        "pane".to_string(),
        "focus".to_string(),
        pane_id.to_string(),
    ]
}

/// The subcommand-to-placement mapping, pure and total, so a dispatch that sent
/// `open-tab` to the split placement is a unit-test failure rather than an invisible
/// one (design.md -> Decision 10).
pub fn placement_for(invocation: &crate::Invocation) -> Option<Placement> {
    match invocation {
        crate::Invocation::Open => Some(Placement::Split),
        crate::Invocation::OpenTab => Some(Placement::Tab),
        crate::Invocation::Ui | crate::Invocation::Reject(_) => None,
    }
}

/// `open::run`'s answer: every warning gathered along the way (a degraded step that did
/// not stop the command), and the outcome — `Ok(())` on success, `Err(reason)` on the
/// failure that stopped it. No `Default`; every construction and destructuring names
/// both fields, with no `..` rest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub warnings: Vec<String>,
    pub outcome: Result<(), String>,
}

/// The stderr lines (every warning, then the error if there is one) and the exit status
/// (0 on `Ok`, 1 on `Err` — never 2, never 3) for `report`. `main`'s two new arms hold no
/// branch of their own; this is the whole decision (design.md -> Decision 10).
pub fn report_output(report: &Report) -> (Vec<String>, i32) {
    let mut lines = report.warnings.clone();
    let status = match &report.outcome {
        Ok(()) => 0,
        Err(reason) => {
            lines.push(reason.clone());
            1
        }
    };
    (lines, status)
}

/// Format a failed Herdr call's reason, on `launch::herdr_reason`'s and
/// `agents::herdr_error_problem`'s established terms: Herdr's diagnostic is carried
/// verbatim, never parsed.
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

/// Run one `open`/`open-tab` invocation to completion against a real `HerdrCli`: list,
/// then focus or open. A failed or unparseable listing warns and still opens
/// (`existing_pane`'s `Err` case). A focus failing with `CliError::Failed { code:
/// Some(2), .. }` — the measured usage shape, what a Herdr without `plugin pane focus`
/// produces — warns and falls through to opening once; any other focus failure stops.
/// The open response is never parsed: exit status alone carries success
/// (`specs/pane-open/spec.md` -> Decisions 4 and 5).
pub fn run(cli: &dyn crate::cli::HerdrCli, ctx: &Context, placement: Placement) -> Report {
    let mut warnings = Vec::new();

    let existing = match cli.run(&["pane", "list"]) {
        Ok(listing) => match existing_pane(&listing, &ctx.workspace_id, ctx.workspace_cwd.as_deref()) {
            Ok(found) => found,
            Err(reason) => {
                warnings.push(reason);
                None
            }
        },
        Err(err) => {
            warnings.push(herdr_reason(&err));
            None
        }
    };

    if let Some(pane_id) = existing {
        let focus = focus_args(&pane_id);
        let refs: Vec<&str> = focus.iter().map(String::as_str).collect();
        match cli.run(&refs) {
            Ok(_) => {
                return Report {
                    warnings,
                    outcome: Ok(()),
                };
            }
            Err(err) => {
                let is_usage_error =
                    matches!(&err, crate::cli::CliError::Failed { code: Some(2), .. });
                if is_usage_error {
                    warnings.push(herdr_reason(&err));
                    // Fall through to opening once: refusing here would fail closed on a
                    // Herdr this manifest's min_herdr_version still declares supported.
                } else {
                    return Report {
                        warnings,
                        outcome: Err(herdr_reason(&err)),
                    };
                }
            }
        }
    }

    let open = open_args(placement, ctx);
    let refs: Vec<&str> = open.iter().map(String::as_str).collect();
    match cli.run(&refs) {
        Ok(_) => Report {
            warnings,
            outcome: Ok(()),
        },
        Err(err) => Report {
            warnings,
            outcome: Err(herdr_reason(&err)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build an environment lookup closure over a fixture map — `cli::tests::env`'s and
    /// `resolve::tests::env`'s shape repeated locally, since it is a two-line test helper
    /// rather than part of any module's public contract.
    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        let map: std::collections::BTreeMap<&str, &str> = pairs.iter().copied().collect();
        move |name| map.get(name).map(|s| s.to_string())
    }

    #[test]
    fn context_full() {
        let json = r#"{"workspace_id":"w8","workspace_cwd":"/repo","tab_id":"w8:t1","focused_pane_id":"w8:p1","focused_pane_cwd":"/repo"}"#;
        let pairs = [
            ("HERDR_PLUGIN_CONTEXT_JSON", json),
            ("HERDR_PLUGIN_ID", "herdr-openspec"),
        ];
        let ctx = context(&env(&pairs)).expect("context reads");
        assert_eq!(ctx.plugin_id, "herdr-openspec");
        assert_eq!(ctx.workspace_id, "w8");
        assert_eq!(ctx.workspace_cwd.as_deref(), Some("/repo"));
        assert_eq!(ctx.focused_pane_id.as_deref(), Some("w8:p1"));
    }

    #[test]
    fn context_from_discrete_variables() {
        let pairs = [
            ("HERDR_WORKSPACE_ID", "w8"),
            ("HERDR_PANE_ID", "w8:p1"),
        ];
        let ctx = context(&env(&pairs)).expect("context reads");
        assert_eq!(ctx.workspace_id, "w8");
        assert_eq!(ctx.focused_pane_id.as_deref(), Some("w8:p1"));
        assert_eq!(ctx.workspace_cwd, None);
    }

    #[test]
    fn context_ignores_unreadable_json() {
        for bad_json in ["not json at all", r#"["w8"]"#] {
            let pairs = [
                ("HERDR_PLUGIN_CONTEXT_JSON", bad_json),
                ("HERDR_WORKSPACE_ID", "w8"),
            ];
            let ctx = context(&env(&pairs)).expect("falls back rather than failing");
            assert_eq!(ctx.workspace_id, "w8");
        }
    }

    #[test]
    fn context_workspace_cwd_falls_back_to_focused_pane_cwd() {
        let json = r#"{"workspace_id":"w8","focused_pane_cwd":"/repo","focused_pane_id":"w8:p1"}"#;
        let pairs = [("HERDR_PLUGIN_CONTEXT_JSON", json)];
        let ctx = context(&env(&pairs)).expect("context reads");
        assert_eq!(ctx.workspace_cwd.as_deref(), Some("/repo"));

        let json_neither = r#"{"workspace_id":"w8"}"#;
        let pairs_neither = [("HERDR_PLUGIN_CONTEXT_JSON", json_neither)];
        let ctx_neither = context(&env(&pairs_neither)).expect("context reads");
        assert_eq!(ctx_neither.workspace_cwd, None);
    }

    #[test]
    fn context_treats_blank_as_absent() {
        let pairs = [("HERDR_WORKSPACE_ID", "   "), ("HERDR_PLUGIN_ID", "")];
        let err = context(&env(&pairs)).expect_err("blank workspace id is absent");
        assert!(err.contains("HERDR_WORKSPACE_ID"), "{err}");

        let pairs2 = [("HERDR_WORKSPACE_ID", "w8"), ("HERDR_PLUGIN_ID", "")];
        let ctx = context(&env(&pairs2)).expect("context reads");
        assert_eq!(ctx.plugin_id, "herdr-openspec");
    }

    #[test]
    fn context_without_a_workspace_is_the_one_fatal_absence() {
        let err = context(&env(&[])).expect_err("no workspace id at all");
        assert!(err.contains("HERDR_WORKSPACE_ID"), "{err}");
        assert!(
            err.to_lowercase().contains("herdr"),
            "should say it must be invoked from Herdr: {err}"
        );
    }

    // --- group 4: the pane matcher -----------------------------------------------------

    #[test]
    fn matches_label_workspace_and_cwd() {
        let listing = r#"{"id":"cli:pane:list","result":{"panes":[{"pane_id":"w8:pG","label":"OpenSpec","workspace_id":"w8","cwd":"/repo"}],"type":"pane_list"}}"#;
        let result = existing_pane(listing, "w8", Some("/repo")).expect("parses");
        assert_eq!(result, Some("w8:pG".to_string()));
    }

    #[test]
    fn no_match_opens_instead() {
        let unlabelled = r#"{"result":{"panes":[{"pane_id":"w8:pX","workspace_id":"w8","cwd":"/repo"}]}}"#;
        assert_eq!(existing_pane(unlabelled, "w8", Some("/repo")), Ok(None));

        let empty = r#"{"result":{"panes":[]}}"#;
        assert_eq!(existing_pane(empty, "w8", Some("/repo")), Ok(None));
    }

    #[test]
    fn entry_without_a_pane_id_is_no_match() {
        let no_id = r#"{"result":{"panes":[{"label":"OpenSpec","workspace_id":"w8","cwd":"/repo"}]}}"#;
        assert_eq!(existing_pane(no_id, "w8", Some("/repo")), Ok(None));

        let non_string_id = r#"{"result":{"panes":[{"pane_id":5,"label":"OpenSpec","workspace_id":"w8","cwd":"/repo"}]}}"#;
        assert_eq!(existing_pane(non_string_id, "w8", Some("/repo")), Ok(None));
    }

    #[test]
    fn other_workspace_is_no_match() {
        let listing = r#"{"result":{"panes":[{"pane_id":"wA:pG","label":"OpenSpec","workspace_id":"wA","cwd":"/repo"}]}}"#;
        assert_eq!(existing_pane(listing, "w8", Some("/repo")), Ok(None));
    }

    #[test]
    fn other_cwd_is_no_match() {
        let listing = r#"{"result":{"panes":[{"pane_id":"w8:pG","label":"OpenSpec","workspace_id":"w8","cwd":"/Users/x/.config/herdr/plugins/github/herdr-openspec-abc"}]}}"#;
        assert_eq!(existing_pane(listing, "w8", Some("/repo")), Ok(None));
    }

    #[test]
    fn cwd_unknown_matches_on_label_and_workspace() {
        let listing = r#"{"result":{"panes":[{"pane_id":"w8:pG","label":"OpenSpec","workspace_id":"w8","cwd":"/anything"}]}}"#;
        assert_eq!(
            existing_pane(listing, "w8", None),
            Ok(Some("w8:pG".to_string()))
        );
    }

    #[test]
    fn two_matches_take_the_first() {
        let listing = r#"{"result":{"panes":[
            {"pane_id":"w8:pG","label":"OpenSpec","workspace_id":"w8","cwd":"/repo"},
            {"pane_id":"w8:pH","label":"OpenSpec","workspace_id":"w8","cwd":"/repo"}
        ]}}"#;
        assert_eq!(
            existing_pane(listing, "w8", Some("/repo")),
            Ok(Some("w8:pG".to_string()))
        );
    }

    // --- group 5: the argument vectors and the two pure `main` decisions ---------------

    fn full_context() -> Context {
        Context {
            plugin_id: "herdr-openspec".to_string(),
            workspace_id: "w8".to_string(),
            workspace_cwd: Some("/repo".to_string()),
            focused_pane_id: Some("w8:p1".to_string()),
        }
    }

    #[test]
    fn split_argv() {
        let argv = open_args(Placement::Split, &full_context());
        assert_eq!(
            argv,
            vec![
                "plugin", "pane", "open", "--plugin", "herdr-openspec", "--entrypoint",
                "dashboard", "--placement", "split", "--direction", "right", "--target-pane",
                "w8:p1", "--cwd", "/repo", "--focus"
            ]
        );
        assert!(!argv.contains(&"--workspace".to_string()));
        assert!(!argv.contains(&"--no-focus".to_string()));
    }

    #[test]
    fn split_argv_without_target_pane() {
        let mut ctx = full_context();
        ctx.focused_pane_id = None;
        let argv = open_args(Placement::Split, &ctx);
        assert!(!argv.contains(&"--target-pane".to_string()));
    }

    #[test]
    fn split_argv_without_cwd() {
        let mut ctx = full_context();
        ctx.workspace_cwd = None;
        let argv = open_args(Placement::Split, &ctx);
        assert!(!argv.contains(&"--cwd".to_string()));
    }

    #[test]
    fn tab_argv() {
        let argv = open_args(Placement::Tab, &full_context());
        assert_eq!(
            argv,
            vec![
                "plugin", "pane", "open", "--plugin", "herdr-openspec", "--entrypoint",
                "dashboard-tab", "--placement", "tab", "--workspace", "w8", "--cwd", "/repo",
                "--focus"
            ]
        );
        assert!(!argv.contains(&"--target-pane".to_string()));
        assert!(!argv.contains(&"--direction".to_string()));
    }

    #[test]
    fn the_two_vectors_differ_only_in_placement_and_target() {
        let ctx = full_context();
        let split = open_args(Placement::Split, &ctx);
        let tab = open_args(Placement::Tab, &ctx);
        assert_eq!(split[3], tab[3]); // --plugin
        assert_eq!(split[4], tab[4]); // herdr-openspec
        assert!(split.contains(&"--cwd".to_string()) && tab.contains(&"--cwd".to_string()));
        assert!(split.last().unwrap() == "--focus");
        assert!(tab.last().unwrap() == "--focus");
        // The differing keys: --entrypoint value, --placement value, --direction/--target-pane
        // (split only), --workspace (tab only).
        assert_ne!(
            split[split.iter().position(|s| s == "--entrypoint").unwrap() + 1],
            tab[tab.iter().position(|s| s == "--entrypoint").unwrap() + 1]
        );
        assert_ne!(
            split[split.iter().position(|s| s == "--placement").unwrap() + 1],
            tab[tab.iter().position(|s| s == "--placement").unwrap() + 1]
        );
        assert!(split.contains(&"--direction".to_string()));
        assert!(!tab.contains(&"--direction".to_string()));
        assert!(split.contains(&"--target-pane".to_string()));
        assert!(!tab.contains(&"--target-pane".to_string()));
        assert!(!split.contains(&"--workspace".to_string()));
        assert!(tab.contains(&"--workspace".to_string()));
    }

    #[test]
    fn placement_for_maps_each_subcommand() {
        assert_eq!(
            placement_for(&crate::Invocation::Open),
            Some(Placement::Split)
        );
        assert_eq!(
            placement_for(&crate::Invocation::OpenTab),
            Some(Placement::Tab)
        );
        assert_eq!(placement_for(&crate::Invocation::Ui), None);
        assert_eq!(placement_for(&crate::Invocation::Reject(None)), None);
    }

    #[test]
    fn report_output_follows_the_report() {
        let with_error = Report {
            warnings: vec!["w1".to_string(), "w2".to_string()],
            outcome: Err("boom".to_string()),
        };
        assert_eq!(
            report_output(&with_error),
            (
                vec!["w1".to_string(), "w2".to_string(), "boom".to_string()],
                1
            )
        );

        let with_warnings_ok = Report {
            warnings: vec!["w1".to_string()],
            outcome: Ok(()),
        };
        assert_eq!(
            report_output(&with_warnings_ok),
            (vec!["w1".to_string()], 0)
        );

        let empty = Report {
            warnings: Vec::new(),
            outcome: Ok(()),
        };
        assert_eq!(report_output(&empty), (Vec::new(), 0));
    }

    // --- group 6: the driver -------------------------------------------------------------

    use crate::cli::{CliError, FakeCli, Program};

    fn empty_listing() -> String {
        r#"{"result":{"panes":[]}}"#.to_string()
    }

    fn labelled_listing() -> String {
        r#"{"result":{"panes":[{"pane_id":"w8:pG","label":"OpenSpec","workspace_id":"w8","cwd":"/repo"}]}}"#.to_string()
    }

    fn open_refs(placement: Placement, ctx: &Context) -> Vec<String> {
        open_args(placement, ctx)
    }

    #[test]
    fn cross_placement_focus() {
        let ctx = full_context();

        let fake_tab = FakeCli::new();
        fake_tab.register_herdr(&["pane", "list"], Ok(labelled_listing()));
        fake_tab.register_herdr(&["plugin", "pane", "focus", "w8:pG"], Ok(String::new()));
        let report = run(&fake_tab, &ctx, Placement::Tab);
        assert_eq!(
            report,
            Report {
                warnings: Vec::new(),
                outcome: Ok(())
            }
        );
        assert_eq!(
            fake_tab.calls(),
            vec![
                (Program::Herdr, vec!["pane".to_string(), "list".to_string()]),
                (
                    Program::Herdr,
                    vec![
                        "plugin".to_string(),
                        "pane".to_string(),
                        "focus".to_string(),
                        "w8:pG".to_string()
                    ]
                ),
            ]
        );

        let fake_split = FakeCli::new();
        fake_split.register_herdr(&["pane", "list"], Ok(labelled_listing()));
        fake_split.register_herdr(&["plugin", "pane", "focus", "w8:pG"], Ok(String::new()));
        let report2 = run(&fake_split, &ctx, Placement::Split);
        assert_eq!(report2.outcome, Ok(()));
        assert_eq!(fake_split.calls().len(), 2);
    }

    #[test]
    fn open_domain_error_stops() {
        let ctx = full_context();
        let fake = FakeCli::new();
        fake.register_herdr(&["pane", "list"], Ok(empty_listing()));
        let argv = open_refs(Placement::Split, &ctx);
        let refs: Vec<&str> = argv.iter().map(String::as_str).collect();
        let stderr = r#"{"error":{"code":"plugin_pane_not_found","message":"plugin pane entrypoint 'nope' not found"},"id":"cli:plugin"}"#;
        fake.register_herdr(
            &refs,
            Err(CliError::Failed {
                program: "herdr".to_string(),
                args: argv.clone(),
                code: Some(1),
                stderr: stderr.to_string(),
            }),
        );
        let report = run(&fake, &ctx, Placement::Split);
        match report.outcome {
            Err(reason) => {
                assert!(reason.contains("plugin_pane_not_found"), "{reason}");
                assert!(
                    reason.contains("plugin pane entrypoint 'nope' not found"),
                    "{reason}"
                );
            }
            Ok(()) => panic!("expected Err"),
        }
    }

    #[test]
    fn open_usage_error_stops() {
        let ctx = full_context();
        let fake = FakeCli::new();
        fake.register_herdr(&["pane", "list"], Ok(empty_listing()));
        let argv = open_refs(Placement::Split, &ctx);
        let refs: Vec<&str> = argv.iter().map(String::as_str).collect();
        fake.register_herdr(
            &refs,
            Err(CliError::Failed {
                program: "herdr".to_string(),
                args: argv.clone(),
                code: Some(2),
                stderr: "missing required --plugin".to_string(),
            }),
        );
        let report = run(&fake, &ctx, Placement::Split);
        match report.outcome {
            Err(reason) => assert!(reason.contains("missing required --plugin"), "{reason}"),
            Ok(()) => panic!("expected Err"),
        }
    }

    #[test]
    fn herdr_not_started_names_the_program() {
        let ctx = full_context();
        let fake = FakeCli::new();
        fake.register_herdr(&["pane", "list"], Ok(empty_listing()));
        let argv = open_refs(Placement::Split, &ctx);
        let refs: Vec<&str> = argv.iter().map(String::as_str).collect();
        fake.register_herdr(
            &refs,
            Err(CliError::NotStarted {
                program: "/scratch/herdr".to_string(),
                args: argv.clone(),
                reason: "No such file or directory (os error 2)".to_string(),
            }),
        );
        let report = run(&fake, &ctx, Placement::Split);
        match report.outcome {
            Err(reason) => {
                assert!(reason.to_lowercase().contains("herdr"), "{reason}");
                assert!(reason.contains("No such file or directory"), "{reason}");
            }
            Ok(()) => panic!("expected Err"),
        }
    }

    #[test]
    fn focus_domain_error_stops_and_opens_nothing() {
        let ctx = full_context();
        let fake = FakeCli::new();
        fake.register_herdr(&["pane", "list"], Ok(labelled_listing()));
        fake.register_herdr(
            &["plugin", "pane", "focus", "w8:pG"],
            Err(CliError::Failed {
                program: "herdr".to_string(),
                args: vec![
                    "plugin".to_string(),
                    "pane".to_string(),
                    "focus".to_string(),
                    "w8:pG".to_string(),
                ],
                code: Some(1),
                stderr: "plugin_pane_not_found".to_string(),
            }),
        );
        let report = run(&fake, &ctx, Placement::Split);
        assert!(report.outcome.is_err());
        assert_eq!(fake.calls().len(), 2);
        assert!(
            !fake
                .calls()
                .iter()
                .any(|(_, argv)| argv.contains(&"open".to_string()) && argv[1] == "pane")
        );
    }

    #[test]
    fn focus_usage_error_warns_and_opens_once() {
        let ctx = full_context();
        let fake = FakeCli::new();
        fake.register_herdr(&["pane", "list"], Ok(labelled_listing()));
        fake.register_herdr(
            &["plugin", "pane", "focus", "w8:pG"],
            Err(CliError::Failed {
                program: "herdr".to_string(),
                args: vec![
                    "plugin".to_string(),
                    "pane".to_string(),
                    "focus".to_string(),
                    "w8:pG".to_string(),
                ],
                code: Some(2),
                stderr: "unrecognized subcommand".to_string(),
            }),
        );
        let argv = open_refs(Placement::Split, &ctx);
        let refs: Vec<&str> = argv.iter().map(String::as_str).collect();
        fake.register_herdr(&refs, Ok(String::new()));

        let report = run(&fake, &ctx, Placement::Split);
        assert_eq!(fake.calls().len(), 3);
        assert!(!report.warnings.is_empty());
        assert_eq!(report.outcome, Ok(()));
    }

    #[test]
    fn listing_failure_warns_and_still_opens() {
        let ctx = full_context();
        let fake = FakeCli::new();
        fake.register_herdr(
            &["pane", "list"],
            Err(CliError::Failed {
                program: "herdr".to_string(),
                args: vec!["pane".to_string(), "list".to_string()],
                code: Some(1),
                stderr: "boom".to_string(),
            }),
        );
        let argv = open_refs(Placement::Split, &ctx);
        let refs: Vec<&str> = argv.iter().map(String::as_str).collect();
        fake.register_herdr(&refs, Ok(String::new()));

        let report = run(&fake, &ctx, Placement::Split);
        assert!(!report.warnings.is_empty());
        assert_eq!(report.outcome, Ok(()));
        assert_eq!(fake.calls().len(), 2);
    }

    #[test]
    fn listing_unparseable_warns_and_still_opens() {
        let ctx = full_context();
        for bad in ["not json", r#"{"result":{}}"#] {
            let fake = FakeCli::new();
            fake.register_herdr(&["pane", "list"], Ok(bad.to_string()));
            let argv = open_refs(Placement::Split, &ctx);
            let refs: Vec<&str> = argv.iter().map(String::as_str).collect();
            fake.register_herdr(&refs, Ok(String::new()));

            let report = run(&fake, &ctx, Placement::Split);
            assert!(!report.warnings.is_empty(), "listing: {bad}");
            assert_eq!(report.outcome, Ok(()));
        }
    }

    #[test]
    fn successful_open_is_silent() {
        let ctx = full_context();
        let fake = FakeCli::new();
        fake.register_herdr(&["pane", "list"], Ok(empty_listing()));
        let argv = open_refs(Placement::Split, &ctx);
        let refs: Vec<&str> = argv.iter().map(String::as_str).collect();
        fake.register_herdr(&refs, Ok(String::new()));

        let report = run(&fake, &ctx, Placement::Split);
        assert_eq!(
            report,
            Report {
                warnings: Vec::new(),
                outcome: Ok(())
            }
        );
    }
}
