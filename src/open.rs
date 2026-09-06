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
}
