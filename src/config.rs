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
/// whitespace, mirroring what `state::state_dir` needs for its own variables.
fn non_blank(value: Option<String>) -> Option<String> {
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

/// Load configuration from `dir`, or produce the default when no directory is
/// resolved. `dir` and file-reading behaviour are implemented in full by the
/// `config.toml`-reading requirement; see
/// `openspec/changes/plugin-config/design.md` -> Contracts.
pub fn load(_dir: Option<&std::path::Path>, _env: &dyn Fn(&str) -> Option<String>) -> Config {
    Config::default()
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

        let empty = env(&[
            ("HERDR_PLUGIN_CONFIG_DIR", ""),
            ("HOME", "/home/someone"),
        ]);
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
            ("XDG_CONFIG_HOME", "/xdg"),
            ("HOME", "/home/someone"),
        ]);
        assert_eq!(
            super::config_dir(&lookup),
            Some(std::path::PathBuf::from(
                "/home/someone/.config/herdr/plugins/config/herdr-openspec"
            ))
        );
        // The state-directory side of this scenario — that state_dir on the same
        // lookup *does* honour XDG_STATE_HOME — is asserted in state.rs once
        // state_dir exists (group 4), so the asymmetry is pinned end to end.
    }

    #[test]
    fn neither_variable_is_available() {
        let lookup = env(&[]);
        assert_eq!(super::config_dir(&lookup), None);
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
        assert_eq!(
            lookup("HERDR_OPENSPEC_DEFINITELY_UNSET_9f3a2b1c"),
            None
        );
    }
}
