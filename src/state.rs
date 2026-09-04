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

/// Read the mapping from `agent-names.toml` inside `dir`. Never fails,
/// panics, or returns an error: an absent directory, an absent file, and an
/// empty file all yield an empty mapping with no problem. Full file-reading
/// behaviour — parsing, per-entry validation — lands with the mapping-file
/// requirement; see `openspec/changes/plugin-config/design.md` -> Contracts.
pub fn read(dir: Option<&Path>) -> Mapping {
    let _ = dir;
    Mapping::default()
}

/// Record that `agent` is the derived name for `change` inside `dir`. The
/// `agent == change` short-circuit runs before the directory is consulted, so
/// an unchanged name succeeds even when no state directory can be resolved —
/// this order is part of the contract. Full atomic-write behaviour lands with
/// the mapping-file requirement; see
/// `openspec/changes/plugin-config/design.md` -> Contracts.
pub fn record(dir: Option<&Path>, agent: &str, change: &str) -> std::io::Result<()> {
    if agent == change {
        return Ok(());
    }
    let Some(_dir) = dir else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no state directory could be resolved",
        ));
    };
    Ok(())
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
}
