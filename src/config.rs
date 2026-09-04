//! Plugin configuration: resolving the configuration directory from the process
//! environment and reading `config.toml` into a [`Config`] value.
//!
//! See `openspec/changes/plugin-config/design.md` for the full contract.

use std::path::PathBuf;

/// Configuration read from `config.toml`, with a documented default for every
/// key so a missing, empty, or malformed file never fails the caller. See
/// `openspec/changes/plugin-config/design.md` -> Contracts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Path to the `openspec` binary, expanded. Absent when not configured, when
    /// the value was blank, or when the key was of the wrong type.
    pub openspec_bin: Option<PathBuf>,
    /// Herdr agent kind launched by the plugin. Defaults to `claude`.
    pub agent_kind: String,
    /// Number of archived changes listed below the separator. Defaults to `5`.
    pub archived_count: usize,
    /// Human-readable descriptions of every fallback this load took, so a
    /// degraded read is observably different from one that found nothing.
    /// `degraded-states` owns rendering these; this change only produces them.
    pub problems: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            openspec_bin: None,
            agent_kind: "claude".to_string(),
            archived_count: 5,
            problems: Vec::new(),
        }
    }
}

/// Treat an environment value as unset when it is empty or contains only
/// whitespace. Shared with `state`, whose own directory resolution needs the
/// identical "first non-blank variable wins" rule.
pub(crate) fn non_blank(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.trim().is_empty())
}

/// Resolve the plugin's configuration directory purely from an injected
/// environment lookup, spawning nothing. `HERDR_PLUGIN_CONFIG_DIR` is
/// authoritative when neither empty nor whitespace-only; otherwise the
/// fallback is `$HOME/.config/herdr/plugins/config/herdr-openspec`, the same
/// path `herdr plugin config-dir herdr-openspec` prints. `XDG_CONFIG_HOME` is
/// deliberately never consulted — see
/// `openspec/changes/plugin-config/design.md` -> Decisions.
pub fn config_dir(env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    if let Some(dir) = non_blank(env("HERDR_PLUGIN_CONFIG_DIR")) {
        return Some(PathBuf::from(dir));
    }
    let home = non_blank(env("HOME"))?;
    Some(
        PathBuf::from(home)
            .join(".config")
            .join("herdr")
            .join("plugins")
            .join("config")
            .join("herdr-openspec"),
    )
}

/// Expand a configured `openspec_bin` value using the same injected
/// environment lookup that resolved the configuration directory. A leading
/// `~/` or `$HOME/`, and a bare `~`, become `HOME`; a `~user` form and any
/// value that cannot be expanded (`HOME` unavailable) are returned verbatim.
/// A blank value is `None`. No filesystem access is made either way — see
/// `openspec/changes/plugin-config/design.md` -> Contracts.
fn expand_openspec_bin(raw: &str, env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let home = non_blank(env("HOME"));

    if trimmed == "~" {
        return Some(match home {
            Some(h) => PathBuf::from(h),
            None => PathBuf::from(trimmed),
        });
    }
    if let Some(rest) = trimmed.strip_prefix("~/") {
        return Some(match home {
            Some(h) => PathBuf::from(h).join(rest),
            None => PathBuf::from(trimmed),
        });
    }
    if let Some(rest) = trimmed.strip_prefix("$HOME/") {
        return Some(match home {
            Some(h) => PathBuf::from(h).join(rest),
            None => PathBuf::from(trimmed),
        });
    }
    // A `~user` form, or anything else: returned verbatim. Resolving another
    // user's home would require a lookup this plugin does not perform.
    Some(PathBuf::from(trimmed))
}

/// Load configuration from `dir`, or produce the default when no directory is
/// resolved, no `config.toml` exists, or the file is empty. Never fails,
/// panics, or returns an error: every fallback is instead recorded as a
/// human-readable string on `Config::problems`. See
/// `openspec/changes/plugin-config/design.md` -> Contracts.
pub fn load(dir: Option<&std::path::Path>, env: &dyn Fn(&str) -> Option<String>) -> Config {
    let mut config = Config::default();

    let Some(dir) = dir else {
        return config;
    };

    let path = dir.join("config.toml");
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return config,
        Err(e) => {
            config
                .problems
                .push(format!("config.toml could not be read: {e}"));
            return config;
        }
    };

    if contents.trim().is_empty() {
        return config;
    }

    let table: toml::Table = match contents.parse() {
        Ok(table) => table,
        Err(e) => {
            config
                .problems
                .push(format!("config.toml is not valid TOML: {e}"));
            return config;
        }
    };

    if let Some(value) = table.get("openspec_bin") {
        match value.as_str() {
            Some(s) => config.openspec_bin = expand_openspec_bin(s, env),
            None => config
                .problems
                .push("openspec_bin is not a string".to_string()),
        }
    }

    if let Some(value) = table.get("agent_kind") {
        match value.as_str() {
            Some(s) => config.agent_kind = s.to_string(),
            None => config
                .problems
                .push("agent_kind is not a string".to_string()),
        }
    }

    if let Some(value) = table.get("archived_count") {
        match value.as_integer() {
            Some(n) if n >= 0 => config.archived_count = n as usize,
            _ => config
                .problems
                .push("archived_count is not a non-negative integer".to_string()),
        }
    }

    config
}

/// The single binding of a name to `std::env::var` in this crate. Treated as
/// its own collaborator so it can be asserted against directly — see
/// `openspec/changes/plugin-config/design.md` -> Decisions ("The environment is
/// an injected lookup").
pub fn env_lookup() -> impl Fn(&str) -> Option<String> {
    |name: &str| std::env::var(name).ok()
}

/// `load(config_dir(&env_lookup()), &env_lookup())`, and nothing else — the
/// crate's one binding to the real environment for configuration, kept as a
/// composition with nothing left to assert. See
/// `openspec/changes/plugin-config/design.md` -> Contracts.
pub fn load_from_env() -> Config {
    let env = env_lookup();
    load(config_dir(&env).as_deref(), &env)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    /// Build an environment lookup closure over a fixture map — never
    /// `std::env::set_var`, which is `unsafe` in edition 2024 and races parallel
    /// tests. `pairs` maps variable name to value; a name absent from `pairs` is
    /// `None`.
    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        let map: BTreeMap<&str, &str> = pairs.iter().copied().collect();
        move |name| map.get(name).map(|s| s.to_string())
    }

    #[test]
    fn herdr_supplies_the_directory() {
        let lookup = env(&[
            ("HERDR_PLUGIN_CONFIG_DIR", "/tmp/cfg-fixture"),
            ("HOME", "/home/someone"),
        ]);
        assert_eq!(
            super::config_dir(&lookup),
            Some(std::path::PathBuf::from("/tmp/cfg-fixture"))
        );

        // HOME is not consulted: the result is unchanged when it is absent.
        let lookup_no_home = env(&[("HERDR_PLUGIN_CONFIG_DIR", "/tmp/cfg-fixture")]);
        assert_eq!(
            super::config_dir(&lookup_no_home),
            Some(std::path::PathBuf::from("/tmp/cfg-fixture"))
        );
    }

    #[test]
    fn run_outside_a_herdr_pane() {
        let lookup = env(&[("HOME", "/home/someone")]);
        assert_eq!(
            super::config_dir(&lookup),
            Some(std::path::PathBuf::from(
                "/home/someone/.config/herdr/plugins/config/herdr-openspec"
            ))
        );
    }

    #[test]
    fn an_empty_or_blank_environment_variable_is_not_a_directory() {
        let expected = Some(std::path::PathBuf::from(
            "/home/someone/.config/herdr/plugins/config/herdr-openspec",
        ));

        let empty = env(&[("HERDR_PLUGIN_CONFIG_DIR", ""), ("HOME", "/home/someone")]);
        assert_eq!(super::config_dir(&empty), expected);

        let blank = env(&[
            ("HERDR_PLUGIN_CONFIG_DIR", "   "),
            ("HOME", "/home/someone"),
        ]);
        assert_eq!(super::config_dir(&blank), expected);
    }

    #[test]
    fn xdg_config_home_is_deliberately_ignored() {
        let lookup = env(&[
            ("XDG_CONFIG_HOME", "/xdg-config"),
            ("XDG_STATE_HOME", "/xdg-state"),
            ("HOME", "/home/someone"),
        ]);
        assert_eq!(
            super::config_dir(&lookup),
            Some(std::path::PathBuf::from(
                "/home/someone/.config/herdr/plugins/config/herdr-openspec"
            ))
        );
        // The asymmetry is a recorded decision, not an oversight: state_dir on
        // the identical lookup *does* honour XDG_STATE_HOME.
        assert_eq!(
            crate::state::state_dir(&lookup),
            Some(std::path::PathBuf::from(
                "/xdg-state/herdr/plugins/herdr-openspec"
            ))
        );
    }

    #[test]
    fn neither_variable_is_available() {
        let lookup = env(&[]);
        assert_eq!(super::config_dir(&lookup), None);

        // Loading configuration for that outcome yields exactly the default
        // Config and reports no problem, because an absent directory is a
        // supported state and not a fault.
        let cfg = super::load(None, &lookup);
        assert_eq!(cfg, super::Config::default());
        assert!(cfg.problems.is_empty());
    }

    #[test]
    fn env_lookup_agrees_with_std_env_var_for_a_present_variable() {
        let lookup = super::env_lookup();
        assert_eq!(lookup("PATH"), std::env::var("PATH").ok());
        assert!(lookup("PATH").is_some());
    }

    #[test]
    fn env_lookup_is_none_for_a_name_nothing_sets() {
        let lookup = super::env_lookup();
        assert_eq!(lookup("HERDR_OPENSPEC_DEFINITELY_UNSET_9f3a2b1c"), None);
    }

    // --- Reading config.toml (group 3) -------------------------------------

    use crate::testutil::{ScratchDir, snapshot};
    use std::fs;

    fn write_config(dir: &std::path::Path, contents: &str) {
        fs::write(dir.join("config.toml"), contents).expect("write config.toml");
    }

    #[test]
    fn every_key_is_set() {
        let scratch = ScratchDir::new();
        write_config(
            scratch.path(),
            "openspec_bin = \"/opt/bin/openspec\"\nagent_kind = \"codex\"\narchived_count = 12\n",
        );
        let cfg = super::load(Some(scratch.path()), &env(&[]));
        assert_eq!(
            cfg.openspec_bin,
            Some(std::path::PathBuf::from("/opt/bin/openspec"))
        );
        assert_eq!(cfg.agent_kind, "codex");
        assert_eq!(cfg.archived_count, 12);
        assert!(cfg.problems.is_empty());
    }

    #[test]
    fn the_file_does_not_exist() {
        let scratch = ScratchDir::new();
        let cfg = super::load(Some(scratch.path()), &env(&[]));
        assert_eq!(cfg, super::Config::default());
    }

    #[test]
    fn the_directory_does_not_exist() {
        let scratch = ScratchDir::new();
        let missing = scratch.path().join("nonexistent");
        let cfg = super::load(Some(&missing), &env(&[]));
        assert_eq!(cfg, super::Config::default());
        assert!(!missing.exists());
    }

    #[test]
    fn an_empty_file_is_not_a_malformed_file() {
        let scratch = ScratchDir::new();
        write_config(scratch.path(), "");
        let cfg = super::load(Some(scratch.path()), &env(&[]));
        assert_eq!(cfg, super::Config::default());
        assert!(cfg.problems.is_empty());
    }

    #[test]
    fn only_one_key_is_set() {
        let scratch = ScratchDir::new();
        write_config(scratch.path(), "archived_count = 0\n");
        let cfg = super::load(Some(scratch.path()), &env(&[]));
        assert_eq!(cfg.archived_count, 0);
        assert_eq!(cfg.agent_kind, "claude");
        assert_eq!(cfg.openspec_bin, None);
        assert!(cfg.problems.is_empty());
    }

    #[test]
    fn unrecognised_keys_are_ignored() {
        let scratch = ScratchDir::new();
        write_config(
            scratch.path(),
            "agent_kind = \"codex\"\nfuture_setting = \"x\"\n[some_table]\nkey = 1\n",
        );
        let cfg = super::load(Some(scratch.path()), &env(&[]));
        assert_eq!(cfg.agent_kind, "codex");
        assert!(cfg.problems.is_empty());
    }

    #[test]
    fn the_file_is_not_valid_toml() {
        let scratch = ScratchDir::new();
        write_config(scratch.path(), "agent_kind = = \"codex\"\n");
        let cfg = super::load(Some(scratch.path()), &env(&[]));
        assert_eq!(cfg.openspec_bin, None);
        assert_eq!(cfg.agent_kind, "claude");
        assert_eq!(cfg.archived_count, 5);
        assert_eq!(cfg.problems.len(), 1);
        assert!(cfg.problems[0].contains("config.toml"));
    }

    #[test]
    fn one_key_has_the_wrong_type_the_rest_survive() {
        let scratch = ScratchDir::new();
        write_config(
            scratch.path(),
            "openspec_bin = true\nagent_kind = \"codex\"\narchived_count = \"many\"\n",
        );
        let cfg = super::load(Some(scratch.path()), &env(&[]));
        assert_eq!(cfg.openspec_bin, None);
        assert_eq!(cfg.archived_count, 5);
        assert_eq!(cfg.agent_kind, "codex");
        assert_eq!(cfg.problems.len(), 2);
        assert!(cfg.problems.iter().any(|p| p.contains("openspec_bin")));
        assert!(cfg.problems.iter().any(|p| p.contains("archived_count")));
    }

    #[test]
    fn a_negative_count_is_not_a_count() {
        let scratch = ScratchDir::new();
        write_config(scratch.path(), "archived_count = -1\n");
        let cfg = super::load(Some(scratch.path()), &env(&[]));
        assert_eq!(cfg.archived_count, 5);
        assert_eq!(cfg.problems.len(), 1);
        assert!(cfg.problems[0].contains("archived_count"));
    }

    #[test]
    fn the_file_cannot_be_read() {
        let scratch = ScratchDir::new();
        fs::create_dir(scratch.path().join("config.toml")).expect("create dir as config.toml");
        let cfg = super::load(Some(scratch.path()), &env(&[]));
        assert_eq!(cfg.openspec_bin, None);
        assert_eq!(cfg.agent_kind, "claude");
        assert_eq!(cfg.archived_count, 5);
        assert_eq!(cfg.problems.len(), 1);
        assert!(cfg.problems[0].contains("config.toml"));
    }

    #[test]
    fn a_tilde_path_is_expanded() {
        let scratch = ScratchDir::new();
        write_config(
            scratch.path(),
            "openspec_bin = \"~/.nvm/versions/node/v24/bin/openspec\"\n",
        );
        let cfg = super::load(Some(scratch.path()), &env(&[("HOME", "/home/someone")]));
        assert_eq!(
            cfg.openspec_bin,
            Some(std::path::PathBuf::from(
                "/home/someone/.nvm/versions/node/v24/bin/openspec"
            ))
        );
    }

    #[test]
    fn a_dollar_home_path_is_expanded_and_a_bare_tilde_is_the_home_directory() {
        let scratch = ScratchDir::new();
        write_config(scratch.path(), "openspec_bin = \"$HOME/bin/openspec\"\n");
        let cfg = super::load(Some(scratch.path()), &env(&[("HOME", "/home/someone")]));
        assert_eq!(
            cfg.openspec_bin,
            Some(std::path::PathBuf::from("/home/someone/bin/openspec"))
        );

        let scratch2 = ScratchDir::new();
        write_config(scratch2.path(), "openspec_bin = \"~\"\n");
        let cfg2 = super::load(Some(scratch2.path()), &env(&[("HOME", "/home/someone")]));
        assert_eq!(
            cfg2.openspec_bin,
            Some(std::path::PathBuf::from("/home/someone"))
        );
    }

    #[test]
    fn an_unexpandable_value_is_returned_verbatim() {
        let scratch = ScratchDir::new();
        write_config(scratch.path(), "openspec_bin = \"~/bin/openspec\"\n");
        let cfg = super::load(Some(scratch.path()), &env(&[]));
        assert_eq!(
            cfg.openspec_bin,
            Some(std::path::PathBuf::from("~/bin/openspec"))
        );

        let scratch2 = ScratchDir::new();
        write_config(
            scratch2.path(),
            "openspec_bin = \"~otheruser/bin/openspec\"\n",
        );
        let cfg2 = super::load(Some(scratch2.path()), &env(&[("HOME", "/home/someone")]));
        assert_eq!(
            cfg2.openspec_bin,
            Some(std::path::PathBuf::from("~otheruser/bin/openspec"))
        );
    }

    #[test]
    fn an_empty_value_is_an_absent_value() {
        let scratch = ScratchDir::new();
        write_config(scratch.path(), "openspec_bin = \"   \"\n");
        let cfg = super::load(Some(scratch.path()), &env(&[]));
        assert_eq!(cfg.openspec_bin, None);
        assert!(cfg.problems.is_empty());
    }

    #[test]
    fn a_path_that_does_not_exist_is_still_returned() {
        let scratch = ScratchDir::new();
        write_config(scratch.path(), "openspec_bin = \"/nowhere/openspec\"\n");
        let cfg = super::load(Some(scratch.path()), &env(&[]));
        assert_eq!(
            cfg.openspec_bin,
            Some(std::path::PathBuf::from("/nowhere/openspec"))
        );
        assert!(!std::path::Path::new("/nowhere/openspec").exists());
        assert!(cfg.problems.is_empty());
    }

    #[test]
    fn a_configuration_read_leaves_the_tree_byte_identical() {
        let scratch = ScratchDir::new();
        write_config(scratch.path(), "agent_kind = \"codex\"\n");
        fs::write(scratch.path().join("unrelated.txt"), b"hello").expect("write unrelated file");

        let before = snapshot(scratch.path());
        super::load(Some(scratch.path()), &env(&[]));
        super::load(Some(scratch.path()), &env(&[]));
        let after = snapshot(scratch.path());
        assert_eq!(before, after);
    }

    #[test]
    fn a_missing_configuration_directory_stays_missing() {
        let scratch = ScratchDir::new();
        let missing = scratch.path().join("nonexistent");
        super::load(Some(&missing), &env(&[]));
        assert!(!missing.exists());
    }
}
