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

        let dir_final = dir.file_name().and_then(|s| s.to_str()).unwrap_or_default();
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

/// Split a leading `YYYY-MM-DD-` prefix off `dir_name`, using exactly the
/// OpenSpec CLI's own pattern: four ASCII digits, `-`, two, `-`, two, `-` —
/// with no calendar validation, because the CLI writes with that pattern and
/// validates no further. `(None, whole_name)` when the pattern does not
/// match, or matches with nothing after it — an archived entry never gets
/// an empty `name`.
///
/// Slicing is byte-index-safe here without a UTF-8 boundary check: every
/// byte inspected up to and including the third hyphen is confirmed ASCII
/// (a digit or `-`) before any slice is taken, so index 10 and index 11
/// always fall on a character boundary regardless of what follows.
pub(crate) fn split_archive_name(dir_name: &str) -> (Option<String>, String) {
    let bytes = dir_name.as_bytes();
    let digit = |i: usize| bytes.get(i).is_some_and(u8::is_ascii_digit);
    let hyphen = |i: usize| bytes.get(i) == Some(&b'-');

    let matches_prefix = digit(0)
        && digit(1)
        && digit(2)
        && digit(3)
        && hyphen(4)
        && digit(5)
        && digit(6)
        && hyphen(7)
        && digit(8)
        && digit(9)
        && hyphen(10);

    if matches_prefix {
        let remainder = &dir_name[11..];
        if !remainder.is_empty() {
            return (Some(dir_name[0..10].to_string()), remainder.to_string());
        }
    }

    (None, dir_name.to_string())
}

/// The final segment of a glob: a literal filename, or a literal prefix and
/// suffix around exactly one `*`. `Prefixed { prefix: "", suffix: "" }` is a
/// bare `*`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FilePattern {
    Literal(String),
    Prefixed { prefix: String, suffix: String },
}

/// The classified shape of a `generates` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Shape {
    /// Not a glob: one relative path, joined and tested for being a regular
    /// file.
    Literal(String),
    /// `dirs` are the literal directory segments; `recursive` is true when
    /// the last directory segment was `**`.
    Glob {
        dirs: Vec<String>,
        recursive: bool,
        file: FilePattern,
    },
}

/// Is `generates` a glob? Reproduces the OpenSpec CLI's own `isGlobPattern`
/// (`dist/core/artifact-graph/outputs.js`) character for character: contains
/// `*`, `?`, or `[`. `{` is deliberately **not** in that set — a value such
/// as `specs/{alpha,zeta}/spec.md` takes the literal-path branch in both
/// tools.
pub(crate) fn is_glob(generates: &str) -> bool {
    generates.contains(['*', '?', '['])
}

/// One directory segment or the final filename segment contains a
/// metacharacter (`*`, `?`, or `[`). Shared by `shape`'s directory-segment
/// loop and its file-pattern check so the two rules cannot drift apart.
fn has_metacharacter(segment: &str) -> bool {
    is_glob(segment)
}

/// Parse the final (filename) segment of a glob into a [`FilePattern`],
/// or `None` when it falls outside the supported subset: any `?` or `[`,
/// or a `*` count other than exactly one.
fn file_pattern(segment: &str) -> Option<FilePattern> {
    if !has_metacharacter(segment) {
        return Some(FilePattern::Literal(segment.to_string()));
    }
    if segment.contains(['?', '[']) {
        return None;
    }
    if segment.matches('*').count() != 1 {
        return None;
    }
    let star = segment.find('*').expect("exactly one '*' was just counted");
    Some(FilePattern::Prefixed {
        prefix: segment[..star].to_string(),
        suffix: segment[star + 1..].to_string(),
    })
}

/// Classify `generates`: a literal relative path, or — when [`is_glob`]
/// holds — a glob within design.md's deliberately small supported subset.
/// `Err` names the pattern verbatim so the caller can record it as a
/// problem: every directory segment must be a literal free of `*?[`,
/// except that the **last** directory segment may be exactly `**`; the
/// final segment must be a literal filename, or a literal prefix, exactly
/// one `*`, and a literal suffix.
pub(crate) fn shape(generates: &str) -> Result<Shape, String> {
    if !is_glob(generates) {
        return Ok(Shape::Literal(generates.to_string()));
    }

    let segments: Vec<&str> = generates.split('/').collect();
    // `split_last` returns `(last_element, everything_before_it)` — the
    // *file* segment first, then the directory segments.
    let (file_segment, dir_segments) = segments
        .split_last()
        .expect("split('/') always yields at least one segment");

    let mut dirs = Vec::new();
    let mut recursive = false;
    let last_dir_index = dir_segments.len().checked_sub(1);
    for (index, segment) in dir_segments.iter().copied().enumerate() {
        let is_last = Some(index) == last_dir_index;
        if is_last && segment == "**" {
            recursive = true;
            continue;
        }
        if has_metacharacter(segment) {
            return Err(generates.to_string());
        }
        dirs.push(segment.to_string());
    }

    let file = file_pattern(file_segment).ok_or_else(|| generates.to_string())?;

    Ok(Shape::Glob {
        dirs,
        recursive,
        file,
    })
}

/// Is `path` a regular file, reached through symbolic links? `fs::metadata`
/// follows links — `symlink_metadata` does not — matching the CLI's own
/// `statSync().isFile()` and `resolve::is_usable_binary`'s shape. Every
/// filesystem `Err` (absent, dangling link, permission denied) means "not
/// usable", never a panic.
fn is_regular_file_through_links(path: &std::path::Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| m.is_file())
}

/// Does `name` match `file`? A [`FilePattern::Literal`] requires an exact
/// name; a [`FilePattern::Prefixed`] requires the name to start with the
/// prefix and end with the suffix (and be long enough to hold both without
/// overlap, so `"md"` does not "match" a one-character name under a
/// `prefix: "m", suffix: "d"` pattern).
fn matches_file_pattern(name: &str, file: &FilePattern) -> bool {
    match file {
        FilePattern::Literal(literal) => name == literal,
        FilePattern::Prefixed { prefix, suffix } => {
            name.len() >= prefix.len() + suffix.len()
                && name.starts_with(prefix.as_str())
                && name.ends_with(suffix.as_str())
        }
    }
}

/// Walk `dir`, collecting every regular file (reached through links) whose
/// name matches `file` into `out`. Skips every entry whose name begins with
/// `.`. Recurses into a real subdirectory only when `recursive` holds, and
/// **never** into a directory reached through a symbolic link — `file_type`
/// is `DirEntry::file_type`, which does not follow links, so a symlinked
/// directory takes the non-recursing branch below regardless of
/// `recursive`, which is what makes a `specs/loop` pointing back at `specs/`
/// terminate rather than cycle. An unreadable `dir` contributes nothing,
/// matching every other directory walk in this crate.
fn collect_glob_matches(
    dir: &std::path::Path,
    recursive: bool,
    file: &FilePattern,
    out: &mut Vec<PathBuf>,
) {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read_dir.flatten() {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            if recursive {
                collect_glob_matches(&path, recursive, file, out);
            }
            continue;
        }
        if matches_file_pattern(&name, file) && is_regular_file_through_links(&path) {
            out.push(path);
        }
    }
}

/// Resolve one schema artifact's `generates` value against `change_dir`:
/// the paths it names on disk, and the one problem an unsupported glob
/// shape records. Never fails and never records a problem for an absent
/// file or an empty match — an unwritten artifact is the normal state of a
/// change in flight.
pub(crate) fn resolve_artifact(
    change_dir: &std::path::Path,
    generates: &str,
) -> (Vec<PathBuf>, Option<String>) {
    match shape(generates) {
        Ok(Shape::Literal(relative)) => {
            let path = change_dir.join(relative);
            if is_regular_file_through_links(&path) {
                (vec![path], None)
            } else {
                (Vec::new(), None)
            }
        }
        Ok(Shape::Glob {
            dirs,
            recursive,
            file,
        }) => {
            let mut base = change_dir.to_path_buf();
            for segment in &dirs {
                base.push(segment);
            }
            let mut matches = Vec::new();
            collect_glob_matches(&base, recursive, &file, &mut matches);
            matches.sort();
            (matches, None)
        }
        Err(pattern) => (
            Vec::new(),
            Some(format!("unsupported glob pattern: {pattern:?}")),
        ),
    }
}

/// Resolve every artifact `schema` declares, in the schema's declared
/// order, against `change_dir`. One [`ArtifactRef`] per schema artifact
/// entry — including a repeated id, which `schema-artifacts` requires kept
/// verbatim and never de-duplicated — plus every unsupported-shape problem,
/// named with the artifact's id so a reader can tell which tab is affected.
pub(crate) fn change_artifacts(
    change_dir: &std::path::Path,
    schema: &crate::schema::Schema,
) -> (Vec<ArtifactRef>, Vec<String>) {
    let mut artifacts = Vec::with_capacity(schema.artifacts.len());
    let mut problems = Vec::new();

    for artifact in &schema.artifacts {
        let (paths, problem) = resolve_artifact(change_dir, &artifact.generates);
        if let Some(reason) = problem {
            problems.push(format!("artifact {:?}: {reason}", artifact.id));
        }
        artifacts.push(ArtifactRef {
            id: artifact.id.clone(),
            paths,
        });
    }

    (artifacts, problems)
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

    // --- group 3: splitting the archive date prefix ----------------------

    #[test]
    fn a_normal_archived_directory_splits_into_a_date_and_a_name() {
        assert_eq!(
            split_archive_name("2026-08-14-add-token-refresh"),
            (
                Some("2026-08-14".to_string()),
                "add-token-refresh".to_string()
            )
        );
    }

    #[test]
    fn an_impossible_date_is_still_a_date_prefix() {
        assert_eq!(
            split_archive_name("9999-99-99-far-future"),
            (Some("9999-99-99".to_string()), "far-future".to_string())
        );
    }

    #[test]
    fn a_malformed_or_absent_prefix_keeps_the_whole_name() {
        // Each is discriminating against a specific loose rule: single
        // digits (a loose digit-run pattern would accept it), no hyphens (a
        // rule matching digits and hyphens anywhere would accept it), no
        // trailing hyphen (a rule testing only the first ten characters
        // would accept it), and a genuinely dated entry alongside them so an
        // implementation that never splits anything cannot pass by
        // returning every input verbatim.
        assert_eq!(
            split_archive_name("2026-1-1-single-digits"),
            (None, "2026-1-1-single-digits".to_string())
        );
        assert_eq!(
            split_archive_name("20260814-nohyphen"),
            (None, "20260814-nohyphen".to_string())
        );
        assert_eq!(
            split_archive_name("2026-08-14"),
            (None, "2026-08-14".to_string())
        );
        assert_eq!(
            split_archive_name("no-date-prefix"),
            (None, "no-date-prefix".to_string())
        );
        assert_eq!(
            split_archive_name("2026-08-14-genuinely-dated"),
            (
                Some("2026-08-14".to_string()),
                "genuinely-dated".to_string()
            )
        );
    }

    #[test]
    fn a_prefix_with_nothing_after_it_keeps_its_whole_name() {
        // A rule that strips eleven characters unconditionally would yield
        // an empty name here.
        assert_eq!(
            split_archive_name("2026-08-14-"),
            (None, "2026-08-14-".to_string())
        );
    }

    // --- group 4: classifying a `generates` value -------------------------

    #[test]
    fn a_plain_filename_is_not_a_glob() {
        assert!(!is_glob("tasks.md"));
        assert!(!is_glob("design.md"));
    }

    #[test]
    fn a_brace_expression_is_not_a_glob() {
        // The case an implementation gets wrong by being reasonable: `{` is
        // not in the CLI's metacharacter set.
        assert!(!is_glob("specs/{alpha,zeta}/spec.md"));
    }

    #[test]
    fn each_of_the_three_metacharacters_makes_a_value_a_glob() {
        assert!(is_glob("specs/**/*.md"));
        assert!(is_glob("notes/file?.md"));
        assert!(is_glob("notes/[ab].md"));
    }

    #[test]
    fn the_supported_glob_subset_parses() {
        assert_eq!(
            shape("specs/**/*.md"),
            Ok(Shape::Glob {
                dirs: vec!["specs".to_string()],
                recursive: true,
                file: FilePattern::Prefixed {
                    prefix: String::new(),
                    suffix: ".md".to_string(),
                },
            })
        );
        assert_eq!(
            shape("specs/spec-*.md"),
            Ok(Shape::Glob {
                dirs: vec!["specs".to_string()],
                recursive: false,
                file: FilePattern::Prefixed {
                    prefix: "spec-".to_string(),
                    suffix: ".md".to_string(),
                },
            })
        );
        assert_eq!(
            shape("specs/*"),
            Ok(Shape::Glob {
                dirs: vec!["specs".to_string()],
                recursive: false,
                file: FilePattern::Prefixed {
                    prefix: String::new(),
                    suffix: String::new(),
                },
            })
        );
        assert_eq!(
            shape("a/b/c.md"),
            Ok(Shape::Literal("a/b/c.md".to_string()))
        );
    }

    #[test]
    fn each_unsupported_glob_shape_is_an_err_naming_the_pattern_verbatim() {
        for pattern in [
            "specs/*/spec.md",
            "specs/?eta/spec.md",
            "specs/[az]*/spec.md",
            "specs/*-*.md",
            "specs/**/nested/*.md",
            "specs/**/nested/**/*.md",
        ] {
            match shape(pattern) {
                Err(reason) => assert_eq!(reason, pattern, "pattern {pattern:?}"),
                Ok(shape) => panic!("expected {pattern:?} to be unsupported, got {shape:?}"),
            }
        }
    }

    #[test]
    fn four_boundary_inputs_have_the_answer_the_subset_rule_already_determines() {
        // `.` is not a glob and is a literal, which 5.7 then resolves to
        // nothing because a directory is not a regular file.
        assert_eq!(shape("."), Ok(Shape::Literal(".".to_string())));
        // `specs/` is not a glob and is a literal, likewise nothing.
        assert_eq!(shape("specs/"), Ok(Shape::Literal("specs/".to_string())));
        // `*` is a glob whose directory list is empty and whose file
        // pattern is a bare `*`, matching every non-dot regular file
        // directly in the change directory.
        assert_eq!(
            shape("*"),
            Ok(Shape::Glob {
                dirs: vec![],
                recursive: false,
                file: FilePattern::Prefixed {
                    prefix: String::new(),
                    suffix: String::new(),
                },
            })
        );
        // `**` alone is `Err`, because the subset's final segment must be a
        // filename pattern and `**` is a directory segment.
        assert!(shape("**").is_err());
    }

    // --- group 5: resolving one artifact to paths -------------------------

    use crate::testutil::{ScratchDir, canonical, symlink};

    fn mkdir(path: &std::path::Path) {
        std::fs::create_dir_all(path).expect("create fixture directory");
    }

    fn write(path: &std::path::Path, contents: &str) {
        if let Some(parent) = path.parent() {
            mkdir(parent);
        }
        std::fs::write(path, contents).expect("write fixture file");
    }

    #[test]
    fn a_non_glob_generates_naming_an_existing_file_resolves_to_it() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        write(&dir.join("proposal.md"), "# Proposal\n");

        let (paths, problem) = resolve_artifact(&dir, "proposal.md");
        assert_eq!(paths, vec![dir.join("proposal.md")]);
        assert_eq!(problem, None);
    }

    #[test]
    fn an_artifact_whose_filename_differs_from_its_id_resolves_by_generates_not_id() {
        // Discriminating against an implementation that builds paths from
        // the artifact's id rather than its `generates` value: a decoy
        // `plan.md` sits beside the real file and must not appear.
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        write(&dir.join("implementation-plan.md"), "# Plan\n");
        write(&dir.join("plan.md"), "# Decoy\n");

        let (paths, problem) = resolve_artifact(&dir, "implementation-plan.md");
        assert_eq!(paths, vec![dir.join("implementation-plan.md")]);
        assert!(!paths.contains(&dir.join("plan.md")));
        assert_eq!(problem, None);
    }

    #[test]
    fn a_non_glob_generates_naming_something_that_is_not_a_regular_file_resolves_to_nothing() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        mkdir(&dir.join("notes"));

        let (paths, problem) = resolve_artifact(&dir, "notes");
        assert!(paths.is_empty());
        assert_eq!(problem, None);
    }

    #[test]
    fn an_absent_non_glob_artifact_file_resolves_to_nothing_with_no_problem() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());

        let (paths, problem) = resolve_artifact(&dir, "design.md");
        assert!(paths.is_empty());
        assert_eq!(problem, None);
    }

    #[test]
    fn a_symbolic_link_to_a_regular_file_is_a_resolved_artifact() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        write(&dir.join("real-proposal.md"), "# Real\n");
        symlink(&dir.join("real-proposal.md"), &dir.join("proposal.md"));

        let (paths, problem) = resolve_artifact(&dir, "proposal.md");
        assert_eq!(paths, vec![dir.join("proposal.md")]);
        assert!(!paths.contains(&dir.join("real-proposal.md")));
        assert_eq!(problem, None);

        // A dangling link resolves to nothing and records nothing.
        symlink(&dir.join("does-not-exist.md"), &dir.join("dangling.md"));
        let (paths, problem) = resolve_artifact(&dir, "dangling.md");
        assert!(paths.is_empty());
        assert_eq!(problem, None);
    }

    #[test]
    fn a_brace_expression_resolves_to_nothing_like_the_cli() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        write(&dir.join("specs/alpha/spec.md"), "# Alpha\n");
        write(&dir.join("specs/zeta/spec.md"), "# Zeta\n");

        let (paths, problem) = resolve_artifact(&dir, "specs/{alpha,zeta}/spec.md");
        assert!(paths.is_empty());
        assert_eq!(problem, None);
    }

    #[test]
    fn a_nested_spec_tree_resolves_in_byte_order_not_locale_order() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        write(&dir.join("specs/Beta/spec.md"), "# Beta\n");
        write(&dir.join("specs/alpha/spec.md"), "# Alpha\n");
        write(&dir.join("specs/alpha/nested/deep.md"), "# Deep\n");
        write(&dir.join("specs/top.md"), "# Top\n");

        let (paths, problem) = resolve_artifact(&dir, "specs/**/*.md");
        assert_eq!(
            paths,
            vec![
                dir.join("specs/Beta/spec.md"),
                dir.join("specs/alpha/nested/deep.md"),
                dir.join("specs/alpha/spec.md"),
                dir.join("specs/top.md"),
            ]
        );
        assert_eq!(problem, None);
    }

    #[test]
    fn a_glob_matching_nothing_is_an_empty_path_list_not_a_problem() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());

        // No `specs/` directory at all.
        let (paths, problem) = resolve_artifact(&dir, "specs/**/*.md");
        assert!(paths.is_empty());
        assert_eq!(problem, None);

        // `specs/` exists and is empty.
        mkdir(&dir.join("specs"));
        let (paths, problem) = resolve_artifact(&dir, "specs/**/*.md");
        assert!(paths.is_empty());
        assert_eq!(problem, None);
    }

    #[test]
    fn dot_entries_and_non_files_are_skipped() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        write(&dir.join("specs/.hidden.md"), "# Hidden\n");
        write(&dir.join("specs/.hidden-cap/spec.md"), "# Hidden cap\n");
        mkdir(&dir.join("specs/looks-like.md"));
        write(&dir.join("specs/alpha/spec.md"), "# Alpha\n");

        let (paths, problem) = resolve_artifact(&dir, "specs/**/*.md");
        assert_eq!(paths, vec![dir.join("specs/alpha/spec.md")]);
        assert_eq!(problem, None);
    }

    #[test]
    fn a_directory_symbolic_link_is_not_descended_into() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        write(&dir.join("specs/alpha/spec.md"), "# Alpha\n");
        symlink(&dir.join("specs"), &dir.join("specs/loop"));

        let (paths, problem) = resolve_artifact(&dir, "specs/**/*.md");
        assert_eq!(paths, vec![dir.join("specs/alpha/spec.md")]);
        assert_eq!(problem, None);
    }

    #[test]
    fn a_prefix_and_suffix_file_pattern_is_supported() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        write(&dir.join("specs/spec-a.md"), "# A\n");
        write(&dir.join("specs/spec-b.md"), "# B\n");
        write(&dir.join("specs/other.md"), "# Other\n");

        let (paths, problem) = resolve_artifact(&dir, "specs/spec-*.md");
        assert_eq!(
            paths,
            vec![dir.join("specs/spec-a.md"), dir.join("specs/spec-b.md")]
        );
        assert_eq!(problem, None);
    }

    #[test]
    fn an_unsupported_glob_shape_records_one_problem_naming_the_pattern() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        write(&dir.join("specs/zeta/spec.md"), "# Zeta\n");

        let (paths, problem) = resolve_artifact(&dir, "specs/*/spec.md");
        assert!(paths.is_empty());
        let problem = problem.expect("unsupported shape should record a problem");
        assert!(problem.contains("specs/*/spec.md"));
    }

    fn probe_schema() -> crate::schema::Schema {
        crate::schema::Schema {
            name: "tdd".to_string(),
            artifacts: vec![
                crate::schema::Artifact {
                    id: "proposal".to_string(),
                    generates: "proposal.md".to_string(),
                },
                crate::schema::Artifact {
                    id: "specs".to_string(),
                    generates: "specs/**/*.md".to_string(),
                },
                crate::schema::Artifact {
                    id: "design".to_string(),
                    generates: "design.md".to_string(),
                },
                crate::schema::Artifact {
                    id: "tasks".to_string(),
                    generates: "tasks.md".to_string(),
                },
                crate::schema::Artifact {
                    id: "planning-review".to_string(),
                    generates: "planning-review.md".to_string(),
                },
            ],
            tasks: None,
        }
    }

    #[test]
    fn tab_order_follows_the_schema_not_the_filesystem() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        write(&dir.join("tasks.md"), "- [ ] a\n");
        write(&dir.join("proposal.md"), "# Proposal\n");

        let (artifacts, problems) = change_artifacts(&dir, &probe_schema());
        assert_eq!(
            artifacts,
            vec![
                ArtifactRef {
                    id: "proposal".to_string(),
                    paths: vec![dir.join("proposal.md")],
                },
                ArtifactRef {
                    id: "specs".to_string(),
                    paths: vec![],
                },
                ArtifactRef {
                    id: "design".to_string(),
                    paths: vec![],
                },
                ArtifactRef {
                    id: "tasks".to_string(),
                    paths: vec![dir.join("tasks.md")],
                },
                ArtifactRef {
                    id: "planning-review".to_string(),
                    paths: vec![],
                },
            ]
        );
        assert!(problems.is_empty());
    }
}
