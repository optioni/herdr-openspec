//! Plugin state: resolving the state directory, deriving a Herdr-legal agent
//! name from a change name, and recording and reading back the mapping from a
//! derived agent name to its change.
//!
//! See `openspec/changes/plugin-config/design.md` for the full contract.

use crate::config::non_blank;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The mapping read from `agent-names.toml`, plus a human-readable problem for
/// every entry or file that could not be used. See
/// `openspec/changes/plugin-config/design.md` -> Contracts.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Mapping {
    /// Agent name to change name, for every well-formed entry.
    pub names: BTreeMap<String, String>,
    /// Human-readable descriptions of every entry or file this read ignored.
    pub problems: Vec<String>,
}

/// Resolve the plugin's state directory purely from an injected environment
/// lookup, spawning nothing. `HERDR_PLUGIN_STATE_DIR` is authoritative when
/// neither empty nor whitespace-only; otherwise `XDG_STATE_HOME` is honoured
/// before `HOME`, unlike `config::config_dir`'s deliberate asymmetry — see
/// `openspec/changes/plugin-config/design.md` -> Decisions.
pub fn state_dir(env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    if let Some(dir) = non_blank(env("HERDR_PLUGIN_STATE_DIR")) {
        return Some(PathBuf::from(dir));
    }
    if let Some(xdg) = non_blank(env("XDG_STATE_HOME")) {
        return Some(
            PathBuf::from(xdg)
                .join("herdr")
                .join("plugins")
                .join("herdr-openspec"),
        );
    }
    let home = non_blank(env("HOME"))?;
    Some(
        PathBuf::from(home)
            .join(".local")
            .join("state")
            .join("herdr")
            .join("plugins")
            .join("herdr-openspec"),
    )
}

/// The 32-bit FNV-1a hash. Not `DefaultHasher`: its output is explicitly not
/// stable across Rust releases, and this value is written to disk. See
/// `openspec/changes/plugin-config/design.md` -> Decisions.
fn fnv1a32(s: &str) -> u32 {
    const OFFSET_BASIS: u32 = 0x811c9dc5;
    const PRIME: u32 = 0x0100_0193;
    let mut hash = OFFSET_BASIS;
    for byte in s.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// Render `n` as exactly four lowercase base-36 digits, zero-padded. Callers
/// pass `n` already reduced modulo 36^4 so the result never truncates.
fn base36_4(mut n: u32) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = [b'0'; 4];
    for slot in out.iter_mut().rev() {
        *slot = DIGITS[(n % 36) as usize];
        n /= 36;
    }
    String::from_utf8(out.to_vec()).expect("base-36 digits are ASCII")
}

/// Convert any change name into a Herdr-legal agent name
/// (`[a-z][a-z0-9_-]{0,31}`) by a pure, total, deterministic function of the
/// change name alone. See `openspec/changes/plugin-config/design.md` ->
/// Contracts and Decisions for the six-step derivation and the reasoning
/// behind the hash-suffix reduction.
pub fn agent_name(change: &str) -> String {
    // 1. ASCII-lowercase; 2. replace illegal characters with '-'.
    let replaced: String = change
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();

    // 3. Collapse runs of '-' to a single '-'.
    let mut collapsed = String::with_capacity(replaced.len());
    let mut prev_dash = false;
    for c in replaced.chars() {
        if c == '-' {
            if prev_dash {
                continue;
            }
            prev_dash = true;
        } else {
            prev_dash = false;
        }
        collapsed.push(c);
    }

    // 3. (cont'd) trim leading and trailing '-' and '_'.
    let trimmed = collapsed.trim_matches(['-', '_']).to_string();

    // 4. An empty result becomes `change`.
    let mut result = if trimmed.is_empty() {
        "change".to_string()
    } else {
        trimmed
    };

    // 5. Prefix `c-` when the first character cannot begin an agent name.
    if !result.starts_with(|c: char| c.is_ascii_lowercase()) {
        result = format!("c-{result}");
    }

    // 6. Truncate to at most 32 characters, with a deterministic hash suffix
    // computed from the whole original change name.
    if result.len() > 32 {
        let prefix: String = result.chars().take(27).collect();
        let prefix = prefix.trim_end_matches(['-', '_']);
        let suffix = base36_4(fnv1a32(change) % 36u32.pow(4));
        result = format!("{prefix}-{suffix}");
    }

    result
}

/// Whether `name` matches Herdr's agent-name pattern `[a-z][a-z0-9_-]{0,31}`.
fn is_legal_agent_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    name.len() <= 32
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// Read the mapping from `agent-names.toml` inside `dir`. Never fails,
/// panics, or returns an error to the caller. An absent directory, an absent
/// file, and an empty file all yield an empty mapping with no problem. A file
/// that is not valid TOML, or whose `[names]` table is missing or is not a
/// table, yields an empty mapping and one problem. Per entry, a non-string
/// value or an agent name that fails `^[a-z][a-z0-9_-]{0,31}$` is skipped with
/// one problem, while every well-formed entry in the same file still comes
/// back. See `openspec/changes/plugin-config/design.md` -> Contracts.
pub fn read(dir: Option<&Path>) -> Mapping {
    let Some(dir) = dir else {
        return Mapping::default();
    };

    let path = dir.join("agent-names.toml");
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(_) => return Mapping::default(),
    };

    if contents.trim().is_empty() {
        return Mapping::default();
    }

    let table: toml::Table = match contents.parse() {
        Ok(table) => table,
        Err(_) => {
            return Mapping {
                names: BTreeMap::new(),
                problems: vec!["agent-names.toml is not valid TOML".to_string()],
            };
        }
    };

    let names_value = match table.get("names") {
        Some(value) => value,
        None => {
            return Mapping {
                names: BTreeMap::new(),
                problems: vec!["agent-names.toml has no [names] table".to_string()],
            };
        }
    };

    let Some(names_table) = names_value.as_table() else {
        return Mapping {
            names: BTreeMap::new(),
            problems: vec!["agent-names.toml's names key is not a table".to_string()],
        };
    };

    let mut mapping = Mapping::default();
    for (key, value) in names_table {
        match value.as_str() {
            None => mapping
                .problems
                .push(format!("agent-names.toml: {key} is not a string")),
            Some(change) if is_legal_agent_name(key) => {
                mapping.names.insert(key.clone(), change.to_string());
            }
            Some(_) => mapping
                .problems
                .push(format!("agent-names.toml: {key} is not a legal agent name")),
        }
    }

    mapping
}

/// Record that `agent` is the derived name for `change` inside `dir`. The
/// `agent == change` short-circuit runs before the directory is consulted, so
/// an unchanged name succeeds even when no state directory can be resolved —
/// this order is part of the contract. Creates the state directory when
/// missing, writes the complete new contents to a temporary file inside that
/// same directory, and renames it over `agent-names.toml`, so a concurrent
/// reader observes either the previous file or the new one and never a
/// partial one. See `openspec/changes/plugin-config/design.md` -> Contracts.
pub fn record(dir: Option<&Path>, agent: &str, change: &str) -> std::io::Result<()> {
    if agent == change {
        return Ok(());
    }
    let Some(dir) = dir else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no state directory could be resolved",
        ));
    };

    std::fs::create_dir_all(dir)
        .map_err(|e| std::io::Error::new(e.kind(), format!("{}: {e}", dir.display())))?;

    let mut mapping = read(Some(dir));
    mapping.names.insert(agent.to_string(), change.to_string());

    let mut names_table = toml::Table::new();
    for (k, v) in &mapping.names {
        names_table.insert(k.clone(), toml::Value::String(v.clone()));
    }
    let mut table = toml::Table::new();
    table.insert("names".to_string(), toml::Value::Table(names_table));
    let contents = table.to_string();

    static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let counter = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp_path = dir.join(format!("agent-names.toml.tmp-{}-{counter}", crate::pid()));

    let write_and_rename = || -> std::io::Result<()> {
        std::fs::write(&tmp_path, &contents)?;
        std::fs::rename(&tmp_path, dir.join("agent-names.toml"))?;
        Ok(())
    };

    match write_and_rename() {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp_path);
            Err(std::io::Error::new(
                e.kind(),
                format!("{}: {e}", dir.display()),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    /// Build an environment lookup closure over a fixture map — see
    /// `src/config.rs`'s identical helper for why this is never
    /// `std::env::set_var`.
    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        let map: BTreeMap<&str, &str> = pairs.iter().copied().collect();
        move |name| map.get(name).map(|s| s.to_string())
    }

    // --- state_dir -----------------------------------------------------

    #[test]
    fn herdr_supplies_the_state_directory() {
        let lookup = env(&[
            ("HERDR_PLUGIN_STATE_DIR", "/tmp/state-fixture"),
            ("XDG_STATE_HOME", "/xdg"),
            ("HOME", "/home/someone"),
        ]);
        assert_eq!(
            super::state_dir(&lookup),
            Some(std::path::PathBuf::from("/tmp/state-fixture"))
        );
        assert_ne!(
            super::state_dir(&lookup),
            crate::config::config_dir(&lookup)
        );
    }

    #[test]
    fn xdg_state_home_is_honoured_before_home() {
        let lookup = env(&[("XDG_STATE_HOME", "/xdg"), ("HOME", "/home/someone")]);
        assert_eq!(
            super::state_dir(&lookup),
            Some(std::path::PathBuf::from("/xdg/herdr/plugins/herdr-openspec"))
        );
    }

    #[test]
    fn run_outside_a_herdr_pane_with_no_xdg_setting() {
        let expected = Some(std::path::PathBuf::from(
            "/home/someone/.local/state/herdr/plugins/herdr-openspec",
        ));

        let none = env(&[("HOME", "/home/someone")]);
        assert_eq!(super::state_dir(&none), expected);

        let empty = env(&[("XDG_STATE_HOME", ""), ("HOME", "/home/someone")]);
        assert_eq!(super::state_dir(&empty), expected);

        let blank = env(&[("XDG_STATE_HOME", "   "), ("HOME", "/home/someone")]);
        assert_eq!(super::state_dir(&blank), expected);
    }

    #[test]
    fn no_directory_can_be_resolved_at_all() {
        let lookup = env(&[]);
        assert_eq!(super::state_dir(&lookup), None);

        let mapping = super::read(None);
        assert!(mapping.names.is_empty());
        assert!(mapping.problems.is_empty());

        assert!(super::record(None, "c-2fa-support", "2fa-support").is_err());
        assert!(super::record(None, "x", "x").is_ok());
    }

    // --- agent_name ------------------------------------------------------

    #[test]
    fn a_short_kebab_case_change_name_is_unchanged() {
        assert_eq!(super::agent_name("add-token-refresh"), "add-token-refresh");
        assert!(is_legal_agent_name(&super::agent_name("add-token-refresh")));
    }

    #[test]
    fn a_name_of_exactly_32_characters_is_not_truncated() {
        let exact = "add-really-long-change-name-that";
        assert_eq!(exact.len(), 32);
        assert_eq!(super::agent_name(exact), exact);

        let one_longer = "add-really-long-change-name-thatx";
        assert_eq!(one_longer.len(), 33);
        let derived = super::agent_name(one_longer);
        assert_eq!(derived, "add-really-long-change-name-mmky");
        assert!(derived.len() <= 32);
        assert_ne!(derived, exact);
    }

    #[test]
    fn a_long_change_name_is_truncated_with_a_suffix_from_the_whole_name() {
        let alpha = "add-really-long-change-name-that-overflows-alpha";
        let beta = "add-really-long-change-name-that-overflows-beta";
        let derived_alpha = super::agent_name(alpha);
        let derived_beta = super::agent_name(beta);
        assert_eq!(derived_alpha, "add-really-long-change-name-8jqt");
        assert_eq!(derived_beta, "add-really-long-change-name-alft");
        assert!(derived_alpha.len() <= 32 && is_legal_agent_name(&derived_alpha));
        assert!(derived_beta.len() <= 32 && is_legal_agent_name(&derived_beta));
        assert_ne!(derived_alpha, derived_beta);
        assert_eq!(super::agent_name(alpha), derived_alpha);
    }

    #[test]
    fn a_truncated_name_may_be_shorter_than_32_characters() {
        let name = "abcdefghijklmnopqrstuvwxyz-abcdefg";
        assert_eq!(name.len(), 34);
        assert_eq!(name.as_bytes()[26], b'-');
        let derived = super::agent_name(name);
        assert_eq!(derived, "abcdefghijklmnopqrstuvwxyz-lhun");
        assert_eq!(derived.len(), 31);
    }

    #[test]
    fn illegal_characters_and_casing_are_normalised() {
        let derived = super::agent_name("Add Token Refresh (v2)!");
        assert_eq!(derived, "add-token-refresh-v2");
        assert!(!derived.starts_with('-'));
        assert!(!derived.ends_with('-'));
        assert!(!derived.contains("--"));
    }

    #[test]
    fn a_name_that_cannot_begin_an_agent_name_is_prefixed() {
        assert_eq!(super::agent_name("2fa-support"), "c-2fa-support");
        assert_eq!(super::agent_name("-leading-dash"), "leading-dash");
    }

    #[test]
    fn a_name_with_nothing_usable_in_it() {
        assert_eq!(super::agent_name(""), "change");
        assert_eq!(super::agent_name("!!!"), "change");
        assert_eq!(super::agent_name("---"), "change");
    }

    fn is_legal_agent_name(s: &str) -> bool {
        let mut chars = s.chars();
        match chars.next() {
            Some(c) if c.is_ascii_lowercase() => {}
            _ => return false,
        }
        s.len() <= 32
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    }

    // --- reading and recording the mapping file (group 5) -------------------

    use crate::testutil::{snapshot, ScratchDir};
    use std::fs;

    fn write_mapping(dir: &std::path::Path, contents: &str) {
        fs::write(dir.join("agent-names.toml"), contents).expect("write agent-names.toml");
    }

    #[test]
    fn absent_file_and_absent_directory_are_both_empty_not_faults() {
        let scratch = ScratchDir::new();

        // Uncreated directory.
        let uncreated = scratch.path().join("nonexistent");
        let mapping = super::read(Some(&uncreated));
        assert!(mapping.names.is_empty());
        assert!(mapping.problems.is_empty());

        // Directory exists but holds no agent-names.toml.
        let empty_dir = ScratchDir::new();
        let mapping = super::read(Some(empty_dir.path()));
        assert!(mapping.names.is_empty());
        assert!(mapping.problems.is_empty());

        // agent-names.toml exists and is zero bytes.
        let zero_byte = ScratchDir::new();
        write_mapping(zero_byte.path(), "");
        let mapping = super::read(Some(zero_byte.path()));
        assert!(mapping.names.is_empty());
        assert!(mapping.problems.is_empty());
    }

    #[test]
    fn a_malformed_file_is_empty_and_reports_one_problem() {
        let scratch = ScratchDir::new();
        write_mapping(scratch.path(), "[names");
        let mapping = super::read(Some(scratch.path()));
        assert!(mapping.names.is_empty());
        assert_eq!(mapping.problems.len(), 1);
        assert!(mapping.problems[0].contains("agent-names.toml"));
    }

    #[test]
    fn a_bad_entry_is_skipped_and_its_neighbours_survive() {
        let scratch = ScratchDir::new();
        write_mapping(
            scratch.path(),
            "[names]\ngood-agent = \"a-real-change\"\nbad-agent = 7\nBad_Name = \"another-change\"\n\"has spaces!\" = \"third-change\"\n",
        );
        let mapping = super::read(Some(scratch.path()));
        assert_eq!(mapping.names.len(), 1);
        assert_eq!(
            mapping.names.get("good-agent"),
            Some(&"a-real-change".to_string())
        );
        assert_eq!(mapping.problems.len(), 3);
        assert!(mapping.problems.iter().any(|p| p.contains("bad-agent")));
        assert!(mapping.problems.iter().any(|p| p.contains("Bad_Name")));
        assert!(mapping.problems.iter().any(|p| p.contains("has spaces!")));
    }

    #[test]
    fn names_is_present_but_is_not_a_table() {
        let scratch = ScratchDir::new();
        write_mapping(scratch.path(), "names = \"nope\"\n");
        let mapping = super::read(Some(scratch.path()));
        assert!(mapping.names.is_empty());
        assert_eq!(mapping.problems.len(), 1);
        assert!(mapping.problems[0].contains("names"));
    }

    #[test]
    fn a_truncated_name_is_recorded() {
        let scratch = ScratchDir::new();
        let change = "a".repeat(48);
        let agent = super::agent_name(&change);
        super::record(Some(scratch.path()), &agent, &change).expect("record");

        assert!(scratch.path().join("agent-names.toml").exists());
        let mapping = super::read(Some(scratch.path()));
        assert_eq!(mapping.names.len(), 1);
        assert_eq!(mapping.names.get(&agent), Some(&change));
    }

    #[test]
    fn an_unchanged_name_is_not_recorded() {
        let scratch = ScratchDir::new();
        let uncreated = scratch.path().join("nonexistent");
        super::record(Some(&uncreated), "add-token-refresh", "add-token-refresh")
            .expect("record");
        assert!(!uncreated.exists());
    }

    #[test]
    fn recording_the_same_pair_twice_changes_nothing() {
        let scratch = ScratchDir::new();
        super::record(Some(scratch.path()), "c-2fa-support", "2fa-support").expect("record 1");
        let first = fs::read(scratch.path().join("agent-names.toml")).expect("read after first");
        super::record(Some(scratch.path()), "c-2fa-support", "2fa-support").expect("record 2");
        let second = fs::read(scratch.path().join("agent-names.toml")).expect("read after second");
        assert_eq!(first, second);
    }

    #[test]
    fn a_second_mapping_is_added_beside_the_first() {
        let scratch = ScratchDir::new();
        super::record(Some(scratch.path()), "c-2fa-support", "2fa-support").expect("record 1");
        super::record(Some(scratch.path()), "c-another", "another!!!").expect("record 2");
        let mapping = super::read(Some(scratch.path()));
        assert_eq!(mapping.names.len(), 2);
        assert_eq!(
            mapping.names.get("c-2fa-support"),
            Some(&"2fa-support".to_string())
        );
        assert_eq!(
            mapping.names.get("c-another"),
            Some(&"another!!!".to_string())
        );
    }

    #[test]
    fn the_same_agent_name_recorded_for_a_different_change_replaces_it() {
        let scratch = ScratchDir::new();
        super::record(Some(scratch.path()), "change", "!!!").expect("record 1");
        super::record(Some(scratch.path()), "unrelated", "unrelated-change").expect("record 2");
        super::record(Some(scratch.path()), "change", "---").expect("record 3");

        let mapping = super::read(Some(scratch.path()));
        assert_eq!(mapping.names.get("change"), Some(&"---".to_string()));
        assert_eq!(
            mapping.names.get("unrelated"),
            Some(&"unrelated-change".to_string())
        );
        assert!(mapping.problems.is_empty());
    }

    #[test]
    fn the_state_directory_is_created_on_first_record() {
        let scratch = ScratchDir::new();
        let state_dir = scratch.path().join("nested").join("state");
        assert!(!state_dir.exists());
        super::record(Some(&state_dir), "c-x", "x-change").expect("record");
        assert!(state_dir.is_dir());
        let entries: Vec<_> = fs::read_dir(&state_dir)
            .expect("read state dir")
            .map(|e| e.expect("entry").file_name())
            .collect();
        assert_eq!(entries, vec![std::ffi::OsString::from("agent-names.toml")]);
    }

    #[test]
    fn the_file_is_replaced_by_rename_not_written_in_place() {
        let scratch = ScratchDir::new();
        super::record(Some(scratch.path()), "c-first", "first-change").expect("record 1");

        let target = scratch.path().join("agent-names.toml");
        let witness = scratch.path().join("witness.toml");
        fs::hard_link(&target, &witness).expect("hard link witness");
        let previous_bytes = fs::read(&witness).expect("read witness before");

        super::record(Some(scratch.path()), "c-second", "second-change").expect("record 2");

        let new_bytes = fs::read(&target).expect("read target after");
        let witness_bytes = fs::read(&witness).expect("read witness after");
        assert_eq!(witness_bytes, previous_bytes);
        assert_ne!(new_bytes, witness_bytes);
        assert!(new_bytes.windows(b"second-change".len()).any(|w| w == b"second-change"));

        let mut names: Vec<_> = fs::read_dir(scratch.path())
            .expect("read state dir")
            .map(|e| e.expect("entry").file_name())
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                std::ffi::OsString::from("agent-names.toml"),
                std::ffi::OsString::from("witness.toml"),
            ]
        );
    }

    #[test]
    fn recording_fails_without_panicking() {
        let scratch = ScratchDir::new();
        let blocked = scratch.path().join("blocked");
        fs::write(&blocked, b"not a directory").expect("write blocking file");
        let before = fs::read(&blocked).expect("read before");

        let result = super::record(Some(&blocked), "c-x", "x-change");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains(&blocked.display().to_string()));

        let after = fs::read(&blocked).expect("read after");
        assert_eq!(before, after);
        assert!(
            fs::read_dir(scratch.path())
                .expect("read scratch dir")
                .filter_map(|e| e.ok())
                .all(|e| !e.file_name().to_string_lossy().contains(".tmp-"))
        );
    }

    #[test]
    fn a_repository_tree_is_untouched_by_a_recording() {
        let repo = ScratchDir::new();
        let changes_dir = repo.path().join("openspec").join("changes").join("x");
        fs::create_dir_all(&changes_dir).expect("create fixture tree");
        fs::write(changes_dir.join("tasks.md"), b"- [ ] 1 do it\n").expect("write fixture file");

        let before = snapshot(repo.path());

        let state = ScratchDir::new();
        super::record(Some(state.path()), "c-x", "x-change").expect("record");

        let after = snapshot(repo.path());
        assert_eq!(before, after);
    }

    #[test]
    fn the_configuration_directory_is_not_written_to() {
        let config = ScratchDir::new();
        write_mapping(config.path(), "openspec_bin = \"/opt/bin/openspec\"\n");
        // (agent-names.toml is not a config file; reuse the writer to place a
        // recognisable fixture file at a known name inside the config dir.)
        fs::remove_file(config.path().join("agent-names.toml")).ok();
        fs::write(config.path().join("config.toml"), b"agent_kind = \"codex\"\n")
            .expect("write config.toml fixture");

        let before = snapshot(config.path());

        let state = ScratchDir::new();
        super::record(Some(state.path()), "c-x", "x-change").expect("record");

        let after = snapshot(config.path());
        assert_eq!(before, after);
    }
}
