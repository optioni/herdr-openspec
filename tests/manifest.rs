//! Contract tier: `herdr-plugin.toml`, the Cargo binary name, and `README.md`'s
//! advertised action titles agree — see `openspec/changes/plugin-actions/design.md` ->
//! Decision 8. This is the one gate in this repository named after a contract with a
//! consumer outside the crate, deliberately placed inside `make check` rather than as a
//! per-change shell command, because every named gate that lives outside `make check` has
//! rotted at least once (`DEPS`/`GRAPH-SNAP`, `HANDOFF.md` -> "Open: the dependency gate
//! has been red on main since live-refresh").
//!
//! Confines itself to values with a **second site** — a value that must agree with
//! something else, whose disagreement is silent. It does NOT assert `version`,
//! `min_herdr_version`, or `platforms`: those have no second site, a wrong value is
//! rejected loudly by `herdr plugin link`, and pinning `version` would make a legitimate
//! bump red. It does NOT assert that `target/release/herdr-openspec` exists: `make check`
//! never runs `make build`, and `cargo test` builds the debug profile, so that assertion
//! would be red on every clean checkout and in CI.

use std::path::Path;

fn manifest() -> toml::Table {
    let text = std::fs::read_to_string("herdr-plugin.toml").expect("read herdr-plugin.toml");
    text.parse::<toml::Table>().expect("parse herdr-plugin.toml as TOML")
}

fn table_array<'a>(manifest: &'a toml::Table, key: &str) -> &'a Vec<toml::Value> {
    manifest
        .get(key)
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("herdr-plugin.toml has no [[{key}]] array"))
}

fn str_field<'a>(table: &'a toml::Value, key: &str) -> &'a str {
    table
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("entry has no string \"{key}\": {table:?}"))
}

/// The crate's binary basename, from `env!` rather than a `cargo metadata` subprocess —
/// no subprocess, no release build, no Herdr needed (`quality-gates` -> "The gates do not
/// depend on Herdr").
fn crate_binary_basename() -> &'static str {
    static NAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    NAME.get_or_init(|| {
        Path::new(env!("CARGO_BIN_EXE_herdr-openspec"))
            .file_stem()
            .expect("bin exe has a file stem")
            .to_string_lossy()
            .into_owned()
    })
}

#[test]
fn manifest_parses_and_declares_the_two_tables() {
    let manifest = manifest();

    for key in [
        "id",
        "name",
        "version",
        "min_herdr_version",
        "platforms",
        "build",
        "panes",
        "actions",
    ] {
        assert!(
            manifest.contains_key(key),
            "herdr-plugin.toml has no top-level key {key:?}"
        );
    }

    let panes = table_array(&manifest, "panes");
    assert_eq!(panes.len(), 2, "expected exactly two [[panes]] entries");
    let pane_ids: Vec<&str> = panes.iter().map(|p| str_field(p, "id")).collect();
    assert_eq!(pane_ids, vec!["dashboard", "dashboard-tab"]);

    let actions = table_array(&manifest, "actions");
    assert_eq!(actions.len(), 2, "expected exactly two [[actions]] entries");
    let action_ids: Vec<&str> = actions.iter().map(|a| str_field(a, "id")).collect();
    assert_eq!(action_ids, vec!["open", "open-tab"]);

    assert!(
        Path::new("scripts/build.sh").is_file(),
        "scripts/build.sh is not a file, resolved from the repository root"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata("scripts/build.sh")
            .expect("stat scripts/build.sh")
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, 0o111, "scripts/build.sh is not executable");
    }
}

#[test]
fn manifest_paths_and_the_cargo_binary_name_agree() {
    let manifest = manifest();
    let basename = crate_binary_basename();

    let panes = table_array(&manifest, "panes");
    let actions = table_array(&manifest, "actions");

    let mut checked = 0usize;
    for pane in panes {
        let id = str_field(pane, "id");
        let command = pane
            .get("command")
            .and_then(|v| v.as_array())
            .unwrap_or_else(|| panic!("pane {id} has no command array"));
        let path = command[0].as_str().expect("command[0] is a string");
        let file_name = Path::new(path)
            .file_name()
            .expect("command path has a file name")
            .to_string_lossy();
        assert_eq!(file_name, basename, "pane {id}'s command basename");
        checked += 1;
    }
    for action in actions {
        let id = str_field(action, "id");
        let command = action
            .get("command")
            .and_then(|v| v.as_array())
            .unwrap_or_else(|| panic!("action {id} has no command array"));
        let path = command[0].as_str().expect("command[0] is a string");
        let file_name = Path::new(path)
            .file_name()
            .expect("command path has a file name")
            .to_string_lossy();
        assert_eq!(file_name, basename, "action {id}'s command basename");
        checked += 1;
    }
    assert_eq!(checked, 4, "expected to check exactly four command basenames");
    assert_eq!(env!("CARGO_PKG_NAME"), basename);
}

#[test]
fn pane_titles_ids_and_placements_match_open_args() {
    let manifest = manifest();
    let panes = table_array(&manifest, "panes");

    for pane in panes {
        let id = str_field(pane, "id");
        let title = str_field(pane, "title");
        assert_eq!(
            title,
            herdr_openspec::open::DASHBOARD_LABEL,
            "pane {id}'s title must match open::DASHBOARD_LABEL — the matcher's only \
             ownership signal"
        );
    }

    let dashboard = panes
        .iter()
        .find(|p| str_field(p, "id") == "dashboard")
        .expect("a dashboard pane exists");
    assert_eq!(
        str_field(dashboard, "id"),
        herdr_openspec::open::DASHBOARD_ENTRYPOINT
    );
    assert_eq!(str_field(dashboard, "placement"), "split");

    let dashboard_tab = panes
        .iter()
        .find(|p| str_field(p, "id") == "dashboard-tab")
        .expect("a dashboard-tab pane exists");
    assert_eq!(
        str_field(dashboard_tab, "id"),
        herdr_openspec::open::DASHBOARD_TAB_ENTRYPOINT
    );
    assert_eq!(str_field(dashboard_tab, "placement"), "tab");
}

#[test]
fn readme_and_the_manifest_agree_on_the_action_titles() {
    let manifest = manifest();
    let actions = table_array(&manifest, "actions");
    let readme = std::fs::read_to_string("README.md").expect("read README.md");
    let collapsed: String = readme.split_whitespace().collect::<Vec<_>>().join(" ");

    let action_titles: Vec<&str> = actions.iter().map(|a| str_field(a, "title")).collect();
    assert_eq!(action_titles.len(), 2);

    let short_title = action_titles
        .iter()
        .find(|t| !t.contains("(tab)"))
        .expect("one action title has no \"(tab)\"");
    let long_title = action_titles
        .iter()
        .find(|t| t.contains("(tab)"))
        .expect("one action title contains \"(tab)\"");
    assert!(
        collapsed.contains(*long_title),
        "README.md does not mention {long_title:?}"
    );
    // The longer title is a prefix-superset of the shorter one, so the shorter title's
    // presence is asserted with the longer occurrence's span removed — two `contains`
    // calls alone could both be satisfied by the longer title.
    let without_long = collapsed.replacen(*long_title, "", 1);
    assert!(
        without_long.contains(*short_title),
        "README.md does not mention {short_title:?} outside the longer title"
    );

    assert!(
        !collapsed.contains("these action-menu entries arrive with a later change"),
        "README.md still carries the deferral sentence"
    );
}
