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
}
