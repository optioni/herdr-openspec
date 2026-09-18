//! Each setting's effective value and the level that produced it, as a pure,
//! total function of values the composition root already holds.
//!
//! Pure and total on exactly `crate::specs`' and `crate::integration`'s
//! terms: no filesystem, process, environment, network, or standard-I/O API,
//! no clock, no global mutable state, and no panic for any input.
//!
//! It lives outside `src/ui/` deliberately, for the reason those two modules
//! do: a new pure-view file would move `NOIO-VIEW`'s and `COLWIDTH`'s
//! pure-file counts, two counts several documents carry. The cost is that no
//! `scripts/gates/` script sweeps this file at all, so its freedom from I/O
//! — and from the render crate's own types, which no gate covers here either
//! — is a `tests/doc_contract.rs` claim over its production slice instead.
//! See `specs/doc-conformance/spec.md`.
//!
//! [`Provenance`] is derived from [`integration::Source`] rather than
//! restating the five-step precedence, and carries [`resolve::BinSource`]
//! whole: one table of precedence levels, not two that could drift — the
//! same rule that makes `ui::tasks::gauge_of` and `ui::list::progress_cell`
//! the crate's one gauge and one progress cell (design.md -> Decision 6). See
//! `openspec/changes/settings-window/specs/setting-provenance/spec.md`.

use crate::{config, integration, resolve};

/// What the launcher's worker answers with when the reader opens the
/// settings panel: the agent-kind choice and the installed integrations,
/// both owned values with no trait, no handle, and no thread. Produced
/// outside `src/ui/`, on `agents::AgentSnapshot`'s terms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KindResolution {
    pub choice: integration::Choice,
    pub installed: Vec<String>,
}

/// The level that produced a setting's effective value, and the crate's one
/// site that turns a level into reader-facing text.
///
/// Derived from [`integration::Source`] by the total [`From`] impl below,
/// with [`resolve::BinSource`] carried whole inside `Probe`. `Pending` and
/// `Ambiguous` are not reachable from `integration::Source`: the first comes
/// from a settings call with `kind: None` — the launcher's worker has not
/// answered yet — and the second from `integration::Choice::Ambiguous`,
/// which carries no `Source` at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    Configured,
    Recorded,
    Probe(resolve::BinSource),
    SoleIntegration,
    LastResort,
    Default,
    Unresolved,
    Pending,
    Ambiguous,
}

/// The four-step agent-kind precedence, carried whole. No wildcard arm: a
/// sixth precedence level added to `integration::Source` later fails to
/// compile here rather than falling silently into a wrong label.
impl From<integration::Source> for Provenance {
    fn from(source: integration::Source) -> Self {
        match source {
            integration::Source::Configured => Provenance::Configured,
            integration::Source::Recorded => Provenance::Recorded,
            integration::Source::SoleIntegration => Provenance::SoleIntegration,
            integration::Source::LastResort => Provenance::LastResort,
        }
    }
}

impl Provenance {
    /// The reader-facing label for this level, in the reader's own terms
    /// rather than the enum's. Total: never panics, and allocates no more
    /// than the returned `String`.
    pub fn label(self) -> String {
        match self {
            Provenance::Configured => "config.toml".to_string(),
            Provenance::Recorded => "settings.toml".to_string(),
            Provenance::Probe(source) => match source {
                resolve::BinSource::Configured => "config.toml".to_string(),
                resolve::BinSource::Path => "PATH".to_string(),
                resolve::BinSource::Nvm => "nvm".to_string(),
                resolve::BinSource::NpmPrefix => "npm prefix -g".to_string(),
            },
            Provenance::SoleIntegration => "the sole installed integration".to_string(),
            Provenance::LastResort => "the last resort".to_string(),
            Provenance::Default => "the default".to_string(),
            Provenance::Unresolved => "not found".to_string(),
            Provenance::Pending => "resolving".to_string(),
            Provenance::Ambiguous => "two or more installed, none chosen".to_string(),
        }
    }
}

/// Why a setting's [`Editable`] is `No`, so a test can assert which refusal
/// fired rather than match on prose. The panel turns each variant into its
/// own reader-facing wording; this module states only which one applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// `openspec_bin` and `prompts` — set-once values read from
    /// `config.toml`, refused for that reason rather than for `config.toml`
    /// having decided them.
    SetOnce,
    /// `config.toml` is deciding this value; a commit would never be seen.
    Configured,
    /// The agent kind is still resolving on the launcher's worker.
    Resolving,
    /// No integration is installed; `config.toml` is the only way to set it.
    NoIntegration,
}

/// Whether the panel may edit a setting, and why not when it may not.
///
/// `Kind` is the only editable variant, and `agent_kind` the only setting
/// that can carry it: `Editable` stays an enum rather than an
/// `Option<Vec<String>>` because `No` carries a [`Reason`] the panel renders,
/// and because a second editable setting added later extends this enum
/// rather than reshaping every match on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Editable {
    Kind { shortlist: Vec<String> },
    No { reason: Reason },
}

/// One setting the panel renders: its key, its effective value, the level
/// that produced it, and whether the panel may edit it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setting {
    pub key: &'static str,
    pub value: String,
    pub provenance: Provenance,
    pub editable: Editable,
}

/// `settings-window`'s addition: `Dashboard::settings`, the seventeenth field
/// — the rows [`settings`] last produced, and the reader's own selected row
/// among them. Lives here, beside [`settings`] and [`Setting`] themselves,
/// rather than on `ui::app`'s own terms, per design.md -> Boundaries: task
/// 10.4 adds it to `NODEFAULT-UI`'s ninth scanned set with
/// `HOMEFILE=src/settings.rs`, which is what settles the placement — that
/// gate's positive control is a `struct $T { ... }` grep, so `PanelState`
/// stays a plain struct rather than becoming a tuple type or a type alias.
///
/// `rows` is populated by [`settings`] at startup, on every adopted
/// `resolution`, and on every commit (design.md -> "How the panel's three
/// inputs reach a pure view"); `cursor` indexes `rows`, not any rendered row
/// list, and is translated to the row grammar's own coordinates by
/// `ui::settings::render` before being handed to `ui::layout::viewport`.
/// Deliberately implements no `Default`, on the same terms as every other
/// state type `NODEFAULT-UI` covers: every construction and destructuring
/// names both fields, with no `..` rest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelState {
    pub rows: Vec<Setting>,
    pub cursor: usize,
}

/// The three settings the panel renders, in this fixed order: `openspec_bin`,
/// `agent_kind`, `prompts`. The order is the module's, not the view's, so the
/// panel renders what it is given rather than deciding an order a second
/// time. `archived_count` is never one of them — `list-sections` made the key
/// accepted and inert, and a panel whose job is to say what is *deciding* a
/// setting has nothing to say about a value nothing reads.
///
/// `config` is read only for `prompts`; `binary` carries everything
/// `openspec_bin` needs, including the winning probe step; `kind` is `None`
/// for the two or three frames before the launcher's worker has answered —
/// see design.md -> "How the panel's three inputs reach a pure view".
///
/// Performs no I/O of any kind and never panics.
pub fn settings(
    config: &config::Config,
    binary: &resolve::BinResolution,
    kind: Option<&KindResolution>,
) -> Vec<Setting> {
    vec![
        openspec_bin_setting(binary),
        agent_kind_setting(kind),
        prompts_setting(config),
    ]
}

/// `openspec_bin`: always read-only. When `binary.found` names the step that
/// won, that step is carried whole as `Provenance::Probe`; a binary found via
/// `resolve::BinSource::Configured` is refused for naming `config.toml`
/// rather than for being merely set-once, so the two refusals stay
/// distinguishable (`setting-provenance` :: "Read-only settings stay
/// read-only however they were resolved").
fn openspec_bin_setting(binary: &resolve::BinResolution) -> Setting {
    match &binary.found {
        Some(found) => {
            let reason = if found.source == resolve::BinSource::Configured {
                Reason::Configured
            } else {
                Reason::SetOnce
            };
            Setting {
                key: "openspec_bin",
                value: found.path.display().to_string(),
                provenance: Provenance::Probe(found.source),
                editable: Editable::No { reason },
            }
        }
        None => Setting {
            key: "openspec_bin",
            value: "no openspec binary was found".to_string(),
            provenance: Provenance::Unresolved,
            editable: Editable::No {
                reason: Reason::SetOnce,
            },
        },
    }
}

/// `agent_kind`: the only editable setting.
///
/// `kind: None` — the launcher's worker has not answered yet — renders as
/// `Pending` and edits nothing. Otherwise the `Configured` rule is checked
/// before any per-setting rule, so a configured kind is refused for naming
/// `config.toml` rather than for its shortlist; `Choice::Ambiguous` has no
/// `Source` to convert and is `Kind`-editable with both candidates; and a
/// resolved kind with no installed integration is refused for naming
/// `config.toml` as the way to set it.
///
/// `pub(crate)`, not private: this is also the one function `ui::app::Dashboard::
/// adopt_launch_outcome` needs to update `settings.rows`'s `agent_kind` entry in place
/// when an outcome's `resolution` is `Some` — the "every adopted resolution" moment
/// `PanelState::rows`'s own doc comment names beside startup and a commit — without
/// reaching for `settings::settings` itself, which would also need `Config` and
/// `resolve::BinResolution` that `Dashboard` does not hold.
pub(crate) fn agent_kind_setting(kind: Option<&KindResolution>) -> Setting {
    let Some(resolution) = kind else {
        return Setting {
            key: "agent_kind",
            value: "the agent kind is still resolving".to_string(),
            provenance: Provenance::Pending,
            editable: Editable::No {
                reason: Reason::Resolving,
            },
        };
    };

    match &resolution.choice {
        integration::Choice::Ambiguous { installed } => Setting {
            key: "agent_kind",
            value: "no kind has been chosen".to_string(),
            provenance: Provenance::Ambiguous,
            editable: Editable::Kind {
                shortlist: installed.clone(),
            },
        },
        integration::Choice::Use { kind, source } => {
            let provenance = Provenance::from(*source);
            let editable = if matches!(provenance, Provenance::Configured) {
                Editable::No {
                    reason: Reason::Configured,
                }
            } else if resolution.installed.is_empty() {
                Editable::No {
                    reason: Reason::NoIntegration,
                }
            } else {
                Editable::Kind {
                    shortlist: resolution.installed.clone(),
                }
            };
            Setting {
                key: "agent_kind",
                value: kind.clone(),
                provenance,
                editable,
            }
        }
    }
}

/// `prompts`: always read-only, whatever the table holds. An empty table's
/// provenance is `Default` — nothing configured, the built-in prompts are
/// used; a populated one's is `Configured` — `config.toml`'s `[prompts]`
/// table is the only source a per-kind override can come from. Either way
/// the reason is `SetOnce`, never `Configured`: the per-setting rule for a
/// set-once value does not defer to the general `Configured` check, exactly
/// as `openspec_bin`'s does not for its probed and unresolved states.
fn prompts_setting(config: &config::Config) -> Setting {
    let (value, provenance) = if config.prompts.is_empty() {
        (
            "no per-kind overrides; the built-in prompts are used".to_string(),
            Provenance::Default,
        )
    } else {
        (
            format!("{} kind(s) overridden", config.prompts.len()),
            Provenance::Configured,
        )
    };
    Setting {
        key: "prompts",
        value,
        provenance,
        editable: Editable::No {
            reason: Reason::SetOnce,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn empty_binary() -> resolve::BinResolution {
        resolve::BinResolution {
            found: None,
            problems: Vec::new(),
        }
    }

    fn found_binary(source: resolve::BinSource) -> resolve::BinResolution {
        resolve::BinResolution {
            found: Some(resolve::FoundBin {
                path: PathBuf::from("/usr/bin/openspec"),
                source,
            }),
            problems: Vec::new(),
        }
    }

    /// `setting-provenance` :: "Three settings in a fixed order, whatever the
    /// inputs".
    #[test]
    fn three_settings_in_a_fixed_order_whatever_the_inputs() {
        let default_config = config::Config::default();
        let empty = settings(&default_config, &empty_binary(), None);
        let empty_keys: Vec<&str> = empty.iter().map(|s| s.key).collect();
        assert_eq!(empty_keys, vec!["openspec_bin", "agent_kind", "prompts"]);
        assert!(empty.iter().all(|s| s.key != "archived_count"));

        let mut populated_config = config::Config {
            openspec_bin: Some(PathBuf::from("/usr/bin/openspec")),
            agent_kind: Some("codex".to_string()),
            archived_count: 42,
            ..config::Config::default()
        };
        let mut per_kind = BTreeMap::new();
        per_kind.insert("apply".to_string(), "do it".to_string());
        populated_config
            .prompts
            .insert("claude".to_string(), per_kind);

        let populated_binary = found_binary(resolve::BinSource::Path);
        let populated_kind = KindResolution {
            choice: integration::Choice::Use {
                kind: "claude".to_string(),
                source: integration::Source::SoleIntegration,
            },
            installed: vec!["claude".to_string()],
        };
        let populated = settings(&populated_config, &populated_binary, Some(&populated_kind));
        let populated_keys: Vec<&str> = populated.iter().map(|s| s.key).collect();
        assert_eq!(
            populated_keys,
            vec!["openspec_bin", "agent_kind", "prompts"],
            "the same three keys in the same order come back when every input is populated"
        );
        // Not `archived_count`, including when it is set to a non-default value.
        assert!(populated.iter().all(|s| s.key != "archived_count"));
        assert_eq!(populated_config.archived_count, 42);
    }

    /// `setting-provenance` :: "Every `integration::Source` maps to a
    /// `Provenance` and back to one label".
    #[test]
    fn every_integration_source_maps_to_a_provenance_and_back_to_one_label() {
        assert_eq!(
            Provenance::from(integration::Source::Configured),
            Provenance::Configured
        );
        assert_eq!(
            Provenance::from(integration::Source::Recorded),
            Provenance::Recorded
        );
        assert_eq!(
            Provenance::from(integration::Source::SoleIntegration),
            Provenance::SoleIntegration
        );
        assert_eq!(
            Provenance::from(integration::Source::LastResort),
            Provenance::LastResort
        );

        let labels = [
            Provenance::Configured.label(),
            Provenance::Recorded.label(),
            Provenance::SoleIntegration.label(),
            Provenance::LastResort.label(),
        ];
        for label in &labels {
            assert!(!label.is_empty(), "every label must be non-empty");
        }
        let unique: std::collections::BTreeSet<_> = labels.iter().cloned().collect();
        assert_eq!(
            unique.len(),
            labels.len(),
            "the four labels must be pairwise distinct: {labels:?}"
        );

        // `Pending` and `Ambiguous` are produced by neither `From` arm: the
        // first comes from `kind: None`, the second from
        // `Choice::Ambiguous`, which carries no `Source` at all.
        let empty_config = config::Config::default();
        let pending = settings(&empty_config, &empty_binary(), None);
        let agent_kind = pending
            .iter()
            .find(|s| s.key == "agent_kind")
            .expect("agent_kind row");
        assert_eq!(agent_kind.provenance, Provenance::Pending);

        let ambiguous_kind = KindResolution {
            choice: integration::Choice::Ambiguous {
                installed: vec!["claude".to_string(), "codex".to_string()],
            },
            installed: vec!["claude".to_string(), "codex".to_string()],
        };
        let ambiguous = settings(&empty_config, &empty_binary(), Some(&ambiguous_kind));
        let agent_kind = ambiguous
            .iter()
            .find(|s| s.key == "agent_kind")
            .expect("agent_kind row");
        assert_eq!(agent_kind.provenance, Provenance::Ambiguous);
    }

    /// `setting-provenance` :: "Every probe step is named by the step that
    /// won".
    #[test]
    fn every_probe_step_is_named_by_the_step_that_won() {
        let sources = [
            resolve::BinSource::Configured,
            resolve::BinSource::Path,
            resolve::BinSource::Nvm,
            resolve::BinSource::NpmPrefix,
        ];
        let labels: Vec<String> = sources
            .iter()
            .map(|source| Provenance::Probe(*source).label())
            .collect();
        for label in &labels {
            assert!(
                !label.is_empty(),
                "every probe-step label must be non-empty"
            );
        }
        let unique: std::collections::BTreeSet<_> = labels.iter().cloned().collect();
        assert_eq!(
            unique.len(),
            labels.len(),
            "the labels for two steps that resolved the same path must still differ: {labels:?}"
        );
    }

    /// `setting-provenance` :: "`Unresolved` is file mode's own label and is
    /// not an error".
    #[test]
    fn unresolved_is_file_modes_own_label_and_is_not_an_error() {
        let config = config::Config::default();
        let rows = settings(&config, &empty_binary(), None);
        let openspec_bin = rows
            .iter()
            .find(|s| s.key == "openspec_bin")
            .expect("openspec_bin row");
        assert_eq!(openspec_bin.provenance, Provenance::Unresolved);
        assert!(
            !openspec_bin.value.is_empty(),
            "the value names that no binary was found rather than being empty"
        );
        assert!(!openspec_bin.provenance.label().is_empty());
    }

    /// `setting-provenance` :: "The `Configured` rule outranks the shortlist
    /// rule".
    #[test]
    fn the_configured_rule_outranks_the_shortlist_rule() {
        let config = config::Config::default();
        let binary = empty_binary();

        let configured_kind = KindResolution {
            choice: integration::Choice::Use {
                kind: "codex".to_string(),
                source: integration::Source::Configured,
            },
            installed: vec!["claude".to_string(), "codex".to_string()],
        };
        let rows = settings(&config, &binary, Some(&configured_kind));
        let agent_kind = rows
            .iter()
            .find(|s| s.key == "agent_kind")
            .expect("agent_kind row");
        match &agent_kind.editable {
            Editable::No { reason } => assert_eq!(*reason, Reason::Configured),
            Editable::Kind { shortlist } => panic!(
                "a configured agent_kind must be refused, not offered a shortlist {shortlist:?}"
            ),
        }

        let ambiguous_kind = KindResolution {
            choice: integration::Choice::Ambiguous {
                installed: vec!["claude".to_string(), "codex".to_string()],
            },
            installed: vec!["claude".to_string(), "codex".to_string()],
        };
        let rows = settings(&config, &binary, Some(&ambiguous_kind));
        let agent_kind = rows
            .iter()
            .find(|s| s.key == "agent_kind")
            .expect("agent_kind row");
        match &agent_kind.editable {
            Editable::Kind { shortlist } => {
                assert_eq!(shortlist, &vec!["claude".to_string(), "codex".to_string()])
            }
            Editable::No { reason } => {
                panic!("with no configured agent_kind the row must be a shortlist, got {reason:?}")
            }
        }
    }

    /// `setting-provenance` :: "Read-only settings stay read-only however
    /// they were resolved".
    #[test]
    fn read_only_settings_stay_read_only_however_they_were_resolved() {
        let mut populated_prompts_config = config::Config::default();
        let mut per_kind = BTreeMap::new();
        per_kind.insert("apply".to_string(), "do it".to_string());
        populated_prompts_config
            .prompts
            .insert("claude".to_string(), per_kind);

        let cases = [
            (
                found_binary(resolve::BinSource::Configured),
                Reason::Configured,
            ),
            (found_binary(resolve::BinSource::Path), Reason::SetOnce),
            (empty_binary(), Reason::SetOnce),
        ];
        for (binary, expected_reason) in &cases {
            for config in [&config::Config::default(), &populated_prompts_config] {
                let rows = settings(config, binary, None);
                let openspec_bin = rows
                    .iter()
                    .find(|s| s.key == "openspec_bin")
                    .expect("openspec_bin row");
                let prompts = rows
                    .iter()
                    .find(|s| s.key == "prompts")
                    .expect("prompts row");

                match &openspec_bin.editable {
                    Editable::No { reason } => assert_eq!(reason, expected_reason),
                    Editable::Kind { .. } => panic!("openspec_bin must never be editable"),
                }
                match &prompts.editable {
                    Editable::No { reason } => assert_eq!(*reason, Reason::SetOnce),
                    Editable::Kind { .. } => panic!("prompts must never be editable"),
                }
            }
        }

        // The two refusals are distinguishable.
        assert_ne!(Reason::Configured, Reason::SetOnce);
    }

    /// `setting-provenance` :: "An empty shortlist never reaches the view as
    /// `Kind`".
    #[test]
    fn an_empty_shortlist_never_reaches_the_view_as_kind() {
        let config = config::Config::default();
        let kind = KindResolution {
            choice: integration::Choice::Use {
                kind: integration::LAST_RESORT.to_string(),
                source: integration::Source::LastResort,
            },
            installed: Vec::new(),
        };
        let rows = settings(&config, &empty_binary(), Some(&kind));
        let agent_kind = rows
            .iter()
            .find(|s| s.key == "agent_kind")
            .expect("agent_kind row");
        match &agent_kind.editable {
            Editable::No { reason } => assert_eq!(*reason, Reason::NoIntegration),
            Editable::Kind { shortlist } => {
                panic!("an empty shortlist must never reach the view as Kind: {shortlist:?}")
            }
        }
    }

    /// `setting-provenance` :: "A pending kind renders as resolving and edits
    /// nothing".
    #[test]
    fn a_pending_kind_renders_as_resolving_and_edits_nothing() {
        let config = config::Config {
            openspec_bin: Some(PathBuf::from("/usr/bin/openspec")),
            ..config::Config::default()
        };
        let binary = found_binary(resolve::BinSource::Path);

        let rows = settings(&config, &binary, None);
        assert_eq!(rows.len(), 3);

        let agent_kind = rows
            .iter()
            .find(|s| s.key == "agent_kind")
            .expect("agent_kind row");
        assert_eq!(agent_kind.provenance, Provenance::Pending);
        assert!(!agent_kind.provenance.label().is_empty());
        assert!(
            !agent_kind.value.is_empty(),
            "the value names that the kind is still resolving rather than being empty"
        );
        assert!(matches!(
            agent_kind.editable,
            Editable::No {
                reason: Reason::Resolving
            }
        ));

        // Two thirds of the panel is useful on the first frame.
        let openspec_bin = rows
            .iter()
            .find(|s| s.key == "openspec_bin")
            .expect("openspec_bin row");
        let prompts = rows
            .iter()
            .find(|s| s.key == "prompts")
            .expect("prompts row");
        assert!(!openspec_bin.value.is_empty());
        assert!(!prompts.value.is_empty());
    }

    /// `setting-provenance` :: "An ambiguous kind has no effective value and
    /// offers both candidates".
    #[test]
    fn an_ambiguous_kind_has_no_effective_value_and_offers_both_candidates() {
        let config = config::Config::default();
        let kind = KindResolution {
            choice: integration::Choice::Ambiguous {
                installed: vec!["claude".to_string(), "codex".to_string()],
            },
            installed: vec!["claude".to_string(), "codex".to_string()],
        };
        let rows = settings(&config, &empty_binary(), Some(&kind));
        let agent_kind = rows
            .iter()
            .find(|s| s.key == "agent_kind")
            .expect("agent_kind row");
        assert_eq!(agent_kind.provenance, Provenance::Ambiguous);
        assert!(!agent_kind.value.is_empty());
        assert!(
            !agent_kind.value.contains("claude") && !agent_kind.value.contains("codex"),
            "no kind is named: {}",
            agent_kind.value
        );
        match &agent_kind.editable {
            Editable::Kind { shortlist } => {
                assert_eq!(shortlist, &vec!["claude".to_string(), "codex".to_string()])
            }
            Editable::No { .. } => {
                panic!(
                    "this is the one state where nothing decides the value and it must still be editable"
                )
            }
        }
    }
}
