//! Turning `openspec/changes/` on disk into the `Change` values every
//! consumer of this plugin reads.
//!
//! See `openspec/changes/changes-from-files/design.md` for the full contract.

use std::path::PathBuf;

/// Whether a change is active or archived, and — for an archived one — the
/// date its directory name was prefixed with, when it parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    Active,
    Archived { date: Option<String> },
}

/// One schema artifact's id and the concrete paths it resolved to on disk,
/// in the schema's declared order. `paths` is empty when nothing is written
/// yet — that is the "No content yet" state, not an error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRef {
    pub id: String,
    pub paths: Vec<PathBuf>,
}

/// One OpenSpec change, reduced to what a dashboard needs. Carries no
/// derived or source-specific state — see `openspec/changes/changes-from-files/design.md`
/// -> Contracts and Decisions 9.
///
/// Deliberately does **not** derive or implement `Default`, and no producer
/// of this type may use a `..` functional-update expression in a `Change`
/// literal. Rust requires every field of a struct literal to be named unless
/// `..` supplies the rest; without `Default` there is no value for `..` to
/// borrow from a blanket source, so removing the field from a literal is a
/// compile error (`E0063`) at every construction site rather than a silent
/// default at one of them. This is `changes::from_files`' and (from Phase 3)
/// `changes::from_cli`'s only enforced agreement mechanism alongside
/// `conformance::assert_invariants` below.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub name: String,
    pub dir: PathBuf,
    pub origin: Origin,
    pub schema: String,
    pub artifacts: Vec<ArtifactRef>,
    pub progress: crate::tasks::Progress,
    pub problems: Vec<String>,
}

/// Every change this plugin found: active changes, archived changes, and
/// problems that belong to the repository rather than to any one change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeSet {
    pub active: Vec<Change>,
    pub archived: Vec<Change>,
    pub problems: Vec<String>,
}

/// The shared gate that keeps `changes::from_files` and (from Phase 3)
/// `changes::from_cli` producing the same `Change`. Test-only: both
/// producers' test suites call `assert_invariants` on every value they
/// build, so a field neither producer's tests exercise is still checked by
/// the one function both call.
#[cfg(test)]
pub(crate) mod conformance {
    use super::{Change, Origin};

    /// Panics naming the invariant that broke. The opening pattern is
    /// exhaustive and carries **no** `..` rest pattern: every field must be
    /// bound (as a name, or as `field: _` for one no invariant reads), so
    /// adding a field to `Change` makes this function fail to compile
    /// (`E0027`) rather than silently pass unread.
    pub(crate) fn assert_invariants(change: &Change) {
        let Change {
            name,
            dir,
            origin,
            schema,
            artifacts,
            progress: _,
            problems,
        } = change;

        assert!(!name.is_empty(), "Change::name must not be empty");
        assert!(!schema.is_empty(), "Change::schema must not be empty");

        for artifact in artifacts {
            assert!(
                !artifact.id.is_empty(),
                "every ArtifactRef::id must be non-empty"
            );
        }
        // Artifact ids are deliberately not asserted unique here: the landed
        // `schema-artifacts` capability requires a schema's artifact list to
        // be kept verbatim and never de-duplicated, so a schema declaring
        // the same id twice legitimately produces two `ArtifactRef`s
        // carrying it.

        for problem in problems {
            assert!(
                !problem.is_empty(),
                "Change::problems must hold no empty string"
            );
        }

        let dir_final = dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        match origin {
            Origin::Active => assert_eq!(
                dir_final,
                name.as_str(),
                "an Active Change's dir must end in exactly its name"
            ),
            Origin::Archived { .. } => assert!(
                dir_final.ends_with(name.as_str()),
                "an Archived Change's dir must end with its name"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::conformance::assert_invariants;
    use super::*;

    fn well_formed_active() -> Change {
        Change {
            name: "add-auth".to_string(),
            dir: PathBuf::from("/repo/openspec/changes/add-auth"),
            origin: Origin::Active,
            schema: "tdd".to_string(),
            artifacts: vec![ArtifactRef {
                id: "proposal".to_string(),
                paths: vec![PathBuf::from("/repo/openspec/changes/add-auth/proposal.md")],
            }],
            progress: crate::tasks::Progress {
                completed: 4,
                total: 9,
            },
            problems: vec![],
        }
    }

    fn well_formed_archived() -> Change {
        Change {
            name: "add-auth".to_string(),
            dir: PathBuf::from("/repo/openspec/changes/archive/2026-08-14-add-auth"),
            origin: Origin::Archived {
                date: Some("2026-08-14".to_string()),
            },
            schema: "tdd".to_string(),
            artifacts: vec![],
            progress: crate::tasks::Progress {
                completed: 3,
                total: 3,
            },
            problems: vec![],
        }
    }

    // --- 2.1: assert_invariants ------------------------------------------

    #[test]
    fn a_well_formed_active_value_passes() {
        assert_invariants(&well_formed_active());
    }

    #[test]
    fn a_well_formed_archived_value_passes() {
        assert_invariants(&well_formed_archived());
    }

    #[test]
    #[should_panic(expected = "Change::name must not be empty")]
    fn an_empty_name_panics() {
        let mut change = well_formed_active();
        change.name = String::new();
        assert_invariants(&change);
    }

    #[test]
    #[should_panic(expected = "Change::schema must not be empty")]
    fn an_empty_schema_panics() {
        let mut change = well_formed_active();
        change.schema = String::new();
        assert_invariants(&change);
    }

    #[test]
    #[should_panic(expected = "every ArtifactRef::id must be non-empty")]
    fn an_artifact_ref_with_an_empty_id_panics() {
        let mut change = well_formed_active();
        change.artifacts = vec![ArtifactRef {
            id: String::new(),
            paths: vec![],
        }];
        assert_invariants(&change);
    }

    #[test]
    #[should_panic(expected = "Change::problems must hold no empty string")]
    fn an_empty_string_in_problems_panics() {
        let mut change = well_formed_active();
        change.problems = vec![String::new()];
        assert_invariants(&change);
    }

    #[test]
    #[should_panic(expected = "an Active Change's dir must end in exactly its name")]
    fn an_active_value_whose_dir_final_component_differs_from_name_panics() {
        let mut change = well_formed_active();
        change.name = "different-name".to_string();
        assert_invariants(&change);
    }

    #[test]
    fn an_archived_value_whose_dir_final_component_ends_with_name_passes() {
        // Already covered by `a_well_formed_archived_value_passes`, but
        // named to pair explicitly with the failing case below.
        assert_invariants(&well_formed_archived());
    }

    #[test]
    #[should_panic(expected = "an Archived Change's dir must end with its name")]
    fn an_archived_value_whose_dir_final_component_does_not_end_with_name_panics() {
        let mut change = well_formed_archived();
        change.dir = PathBuf::from("/repo/openspec/changes/archive/2026-08-14-other-change");
        assert_invariants(&change);
    }

    #[test]
    fn artifacts_holding_two_refs_with_the_same_id_pass() {
        // Artifact ids are not unique — `schema-artifacts` requires a
        // schema's list to be kept verbatim and never de-duplicated, and it
        // ships a scenario producing the ids `zeta, alpha, middle, zeta`. A
        // uniqueness invariant here would panic on a value `from_files`
        // legitimately produces.
        let mut change = well_formed_active();
        change.artifacts = vec![
            ArtifactRef {
                id: "zeta".to_string(),
                paths: vec![],
            },
            ArtifactRef {
                id: "alpha".to_string(),
                paths: vec![],
            },
            ArtifactRef {
                id: "middle".to_string(),
                paths: vec![],
            },
            ArtifactRef {
                id: "zeta".to_string(),
                paths: vec![],
            },
        ];
        assert_invariants(&change);
    }

    // --- 2.2: the three-way status split, from the type's side -----------

    #[test]
    fn the_three_way_status_split_is_derived_from_progress() {
        // Close to a tautology over `Progress` on its own — the check that
        // actually goes red when someone adds a `status` field is 9.3's
        // source scan. This is the worked example a future reader finds
        // when they wonder where the split lives, and it has to be deleted
        // to make room for a stored one.
        let no_tasks = crate::tasks::Progress {
            completed: 0,
            total: 0,
        };
        let complete = crate::tasks::Progress {
            completed: 3,
            total: 3,
        };
        let in_progress = crate::tasks::Progress {
            completed: 1,
            total: 3,
        };

        assert_eq!(no_tasks.total, 0);
        assert!(!no_tasks.is_complete());

        assert!(complete.total > 0);
        assert!(complete.is_complete());

        assert!(in_progress.total > 0);
        assert!(!in_progress.is_complete());
    }
}
