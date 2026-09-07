//! Turning `openspec/changes/` on disk into the `Change` values every
//! consumer of this plugin reads.
//!
//! See `openspec/changes/changes-from-files/design.md` for the full contract.

use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;

/// A non-negative integer read from a JSON value, representable as `usize`.
/// Shared by `parse_list`'s `completedTasks`/`totalTasks` fields — a `Value`
/// that is missing, negative, fractional, a string, or too large for
/// `usize` all yield `None` uniformly, through `serde_json::Value::as_u64`.
fn non_negative_usize(value: Option<&serde_json::Value>) -> Option<usize> {
    usize::try_from(value?.as_u64()?).ok()
}

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
    /// True at exactly the one position the schema's tasks-artifact rule
    /// names — the first index of `Schema::artifacts` equal to
    /// `Schema::tasks` — false everywhere else and everywhere when the
    /// schema names no tasks artifact.
    pub tracks_tasks: bool,
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
            // `date: _` rather than a `..` rest pattern: `Origin` is part of
            // the two-producer surface too, and `..` here would let a field
            // added to `Archived` escape the same silent-absorption risk
            // mechanism 2 exists to close on `Change` itself. Found in
            // Change Review.
            Origin::Archived { date: _ } => assert!(
                dir_final.ends_with(name.as_str()),
                "an Archived Change's dir must end with its name"
            ),
        }
    }
}

/// Test-only `Change`/`ChangeSet` builders shared by `ui::app`, `ui::list`, and
/// `ui::view`'s tests. Naming every field explicitly, with no `..` and no
/// `Default`, keeps every construction site inside this file — the one
/// `change-model`'s `GATE-MECH1` searches — so `NOLIT-CHANGE` has nothing to
/// catch. See `openspec/changes/list-view/design.md` -> Boundaries and
/// Decisions ("`Change` fixtures live in `src/changes.rs`, behind
/// `#[cfg(test)]`").
#[cfg(test)]
pub(crate) mod fixture {
    use std::path::PathBuf;

    use super::{Change, ChangeSet, Origin};

    /// An active change named `name`, at `completed` of `total` tasks, whose
    /// `dir` ends in exactly `name` — `conformance::assert_invariants`'s
    /// requirement for `Origin::Active`.
    pub(crate) fn active(name: &str, completed: usize, total: usize) -> Change {
        Change {
            name: name.to_string(),
            dir: PathBuf::from(format!("/repo/openspec/changes/{name}")),
            origin: Origin::Active,
            schema: "tdd".to_string(),
            artifacts: Vec::new(),
            progress: crate::tasks::Progress { completed, total },
            problems: Vec::new(),
        }
    }

    /// An archived change named `name`, at `completed` of `total` tasks,
    /// dated `date` (or undated when `None`), whose `dir` ends *with* `name`
    /// — `conformance::assert_invariants`'s (weaker) requirement for
    /// `Origin::Archived`, since the directory carries the date prefix too.
    pub(crate) fn archived(
        date: Option<&str>,
        name: &str,
        completed: usize,
        total: usize,
    ) -> Change {
        let dir_name = match date {
            Some(d) => format!("{d}-{name}"),
            None => name.to_string(),
        };
        Change {
            name: name.to_string(),
            dir: PathBuf::from(format!("/repo/openspec/changes/archive/{dir_name}")),
            origin: Origin::Archived {
                date: date.map(str::to_string),
            },
            schema: "tdd".to_string(),
            artifacts: Vec::new(),
            progress: crate::tasks::Progress { completed, total },
            problems: Vec::new(),
        }
    }

    /// `change` with its `artifacts` replaced by `ArtifactRef` values built
    /// from `artifacts`' ids and path strings, in the given order, with
    /// **no** de-duplication — `schema-artifacts` requires that a schema
    /// declaring the same artifact id at two positions keeps both.
    /// `ui::detail`'s and `ui::view`'s tests reach every artifact-carrying
    /// `Change` through this function, so no `Change {` or `ArtifactRef {`
    /// literal is needed outside this file, which is exactly what keeps
    /// `NOLIT-CHANGE` honest.
    pub(crate) fn with_artifacts(change: Change, artifacts: &[(&str, &[&str])]) -> Change {
        // Every field named explicitly, with no `..` rest — `Change`
        // implements no `Default` and this file's own gate (`GATE-MECH1`)
        // forbids a functional-update expression here on the same terms as
        // everywhere else in the crate.
        let Change {
            name,
            dir,
            origin,
            schema,
            artifacts: _,
            progress,
            problems,
        } = change;
        Change {
            name,
            dir,
            origin,
            schema,
            artifacts: artifacts
                .iter()
                .map(|(id, paths)| super::ArtifactRef {
                    id: (*id).to_string(),
                    paths: paths.iter().map(PathBuf::from).collect(),
                    tracks_tasks: false,
                })
                .collect(),
            progress,
            problems,
        }
    }

    /// `change` with `artifacts[index]`'s `tracks_tasks` set `true` and every
    /// other entry's left `false` — the one place a view test reaches for a
    /// `Change` carrying a marked artifact, on `with_artifacts`'s terms:
    /// `NOLIT-CHANGE` forbids an `ArtifactRef {` literal outside this file, so
    /// a test cannot build one for itself. `index` past the end of
    /// `artifacts` marks nothing and does not panic.
    pub(crate) fn track_tasks_at(change: Change, index: usize) -> Change {
        let Change {
            name,
            dir,
            origin,
            schema,
            artifacts,
            progress,
            problems,
        } = change;
        Change {
            name,
            dir,
            origin,
            schema,
            artifacts: artifacts
                .into_iter()
                .enumerate()
                .map(|(i, artifact)| {
                    let super::ArtifactRef {
                        id,
                        paths,
                        tracks_tasks: _,
                    } = artifact;
                    super::ArtifactRef {
                        id,
                        paths,
                        tracks_tasks: i == index,
                    }
                })
                .collect(),
            progress,
            problems,
        }
    }

    /// A `Change` carrying `artifacts`' ids and paths (via
    /// [`with_artifacts`]), its `progress` set explicitly, and — when
    /// `marked` is `Some` — that position's `tracks_tasks` flipped `true`
    /// (via [`track_tasks_at`]). The one place `ui::detail`'s, `ui::app`'s,
    /// and `ui::view`'s tests reach for a `Change` carrying a possibly
    /// marked artifact and an explicit progress pair, so no test module
    /// needs a `-> Change` (or `-> crate::changes::Change`) helper of its
    /// own — `NOLIT-CHANGE`'s pattern catches a `Change {` return-type
    /// signature exactly as it catches a literal, and Change Review found
    /// three such helpers red on the unmodified pattern; the fix is here,
    /// not an exemption.
    pub(crate) fn with_marked_artifacts(
        artifacts: &[(&str, &[&str])],
        marked: Option<usize>,
        progress: crate::tasks::Progress,
    ) -> Change {
        let mut change = with_artifacts(active("x", 0, 0), artifacts);
        change.progress = progress;
        match marked {
            Some(index) => track_tasks_at(change, index),
            None => change,
        }
    }

    /// `change` with its `schema` replaced — `active` and `archived` both
    /// hardcode `"tdd"`, and `detail-header`'s archived-change scenario
    /// needs one whose schema is its own rather than the fixture's
    /// default, on the same "no `..` rest, every field named" terms as
    /// `with_artifacts`.
    pub(crate) fn with_schema(change: Change, schema: &str) -> Change {
        let Change {
            name,
            dir,
            origin,
            schema: _,
            artifacts,
            progress,
            problems,
        } = change;
        Change {
            name,
            dir,
            origin,
            schema: schema.to_string(),
            artifacts,
            progress,
            problems,
        }
    }

    /// `change` with its `problems` replaced — `degraded-states`' addition, the one place
    /// `ui::detail`'s tests reach for a `Change` carrying its own problems, on
    /// `with_schema`'s "no `..` rest, every field named" terms.
    pub(crate) fn with_problems(change: Change, problems: Vec<String>) -> Change {
        let Change {
            name,
            dir,
            origin,
            schema,
            artifacts,
            progress,
            problems: _,
        } = change;
        Change {
            name,
            dir,
            origin,
            schema,
            artifacts,
            progress,
            problems,
        }
    }

    /// A `ChangeSet` from already-built `active` and `archived` vectors and
    /// `problems`, preserving each vector's order untouched — `rows` and
    /// `Dashboard::visible` are what sort or filter, never the fixture.
    pub(crate) fn set(
        active: Vec<Change>,
        archived: Vec<Change>,
        problems: Vec<String>,
    ) -> ChangeSet {
        ChangeSet {
            active,
            archived,
            problems,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{active, archived, set, with_artifacts, with_schema};
        use crate::changes::conformance::assert_invariants;

        #[test]
        fn every_builder_satisfies_the_change_invariants() {
            assert_invariants(&active("add-token-refresh", 4, 9));
            assert_invariants(&archived(Some("2026-08-14"), "add-auth", 7, 7));
            assert_invariants(&archived(None, "legacy-cleanup", 3, 3));
        }

        #[test]
        fn with_artifacts_replaces_only_the_artifacts_field_in_order_and_undeduplicated() {
            let base = active("detail-view", 4, 9);
            let with = with_artifacts(
                base.clone(),
                &[
                    ("proposal", &["/repo/proposal.md"]),
                    ("spec", &["/repo/a/spec.md", "/repo/b/spec.md"]),
                    ("spec", &["/repo/c/spec.md"]),
                ],
            );
            assert_eq!(with.name, base.name);
            assert_eq!(with.dir, base.dir);
            assert_eq!(with.origin, base.origin);
            assert_eq!(with.schema, base.schema);
            assert_eq!(with.progress, base.progress);
            assert_eq!(with.problems, base.problems);
            assert_eq!(with.artifacts.len(), 3);
            assert_eq!(with.artifacts[0].id, "proposal");
            assert_eq!(
                with.artifacts[0].paths,
                vec![std::path::PathBuf::from("/repo/proposal.md")]
            );
            assert_eq!(with.artifacts[1].id, "spec");
            assert_eq!(with.artifacts[2].id, "spec");
            assert_eq!(
                with.artifacts[1].paths,
                vec![
                    std::path::PathBuf::from("/repo/a/spec.md"),
                    std::path::PathBuf::from("/repo/b/spec.md"),
                ]
            );
            assert_invariants(&with);
        }

        #[test]
        fn with_artifacts_over_an_empty_list_yields_no_artifacts() {
            let with = with_artifacts(active("alpha", 1, 2), &[]);
            assert!(with.artifacts.is_empty());
        }

        #[test]
        fn with_schema_replaces_only_the_schema_field() {
            let base = archived(Some("2026-08-14"), "add-auth", 7, 7);
            let with = with_schema(base.clone(), "spec-driven");
            assert_eq!(with.schema, "spec-driven");
            assert_eq!(with.name, base.name);
            assert_eq!(with.dir, base.dir);
            assert_eq!(with.origin, base.origin);
            assert_eq!(with.artifacts, base.artifacts);
            assert_eq!(with.progress, base.progress);
            assert_eq!(with.problems, base.problems);
        }

        #[test]
        fn the_five_change_fixture_is_ordered_active_then_archived() {
            let s = set(
                vec![active("a", 1, 2), active("b", 1, 2)],
                vec![archived(Some("2026-01-01"), "c", 1, 1)],
                Vec::new(),
            );
            assert_eq!(s.active[0].name, "a");
            assert_eq!(s.active[1].name, "b");
            assert_eq!(s.archived[0].name, "c");
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
            // `PathBuf`'s `Ord` compares path *components*, not the raw
            // path string: `"specs/api"` sorts before `"specs/api.md"`
            // because `"api"` is a strict prefix of `"api.md"`, but full
            // *byte-string* order (what the spec requires) puts
            // `"specs/api.md"` first, since `.` (0x2E) sorts before `/`
            // (0x2F). Sort on the raw OS-string bytes instead — found in
            // Change Review, before any test exercised the divergence.
            matches.sort_by(|a, b| a.as_os_str().as_bytes().cmp(b.as_os_str().as_bytes()));
            (matches, None)
        }
        Err(pattern) => (
            Vec::new(),
            Some(format!("unsupported glob pattern: {pattern:?}")),
        ),
    }
}

/// The index of `schema.tasks` within `schema.artifacts` — the **first**
/// entry equal to it, matching the OpenSpec CLI's own `find`. `None` when
/// `schema.tasks` is `None`. The one implementation both `change_artifacts`
/// and `cli_artifacts` call, so the two producers apply one rule rather
/// than two copies. See `change-artifacts` -> "The tracked-tasks artifact
/// is marked at its schema position".
pub(crate) fn tasks_index(schema: &crate::schema::Schema) -> Option<usize> {
    let tasks = schema.tasks.as_ref()?;
    schema.artifacts.iter().position(|a| a == tasks)
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
    let tasks_index = tasks_index(schema);

    for (index, artifact) in schema.artifacts.iter().enumerate() {
        let (paths, problem) = resolve_artifact(change_dir, &artifact.generates);
        if let Some(reason) = problem {
            problems.push(format!("artifact {:?}: {reason}", artifact.id));
        }
        artifacts.push(ArtifactRef {
            id: artifact.id.clone(),
            paths,
            tracks_tasks: tasks_index == Some(index),
        });
    }

    (artifacts, problems)
}

/// The change's task pair: resolve `tasks`' `generates` value through
/// [`resolve_artifact`], substitute `[change_dir/tasks.md]` when that
/// resolves to no files — because no tasks artifact is declared, because
/// the schema failed to load so no artifact is available at all, or
/// because the artifact's glob matched nothing — and sum every target with
/// `tasks::read`. Reproduces the OpenSpec CLI's own
/// `getTaskProgressDetailForChange` (`dist/utils/task-progress.js:120-133`),
/// including the fallback: omitting it would report `0/0` for exactly the
/// changes whose schema is unusual.
pub(crate) fn change_progress(
    change_dir: &std::path::Path,
    tasks: Option<&crate::schema::Artifact>,
) -> (crate::tasks::Progress, Vec<String>) {
    let resolved = tasks.map(|artifact| resolve_artifact(change_dir, &artifact.generates).0);
    let targets = match resolved {
        Some(paths) if !paths.is_empty() => paths,
        _ => vec![change_dir.join("tasks.md")],
    };

    let mut progress = crate::tasks::Progress {
        completed: 0,
        total: 0,
    };
    let mut problems = Vec::new();
    for target in &targets {
        let document = crate::tasks::read(target);
        progress += document.progress();
        problems.extend(document.problems);
    }

    (progress, problems)
}

/// Decode one directory entry's raw OS name to UTF-8, or record one problem
/// naming it (lossily, for the message only) and skip it. Kept as its own
/// pure step — rather than folded into the walk below — because APFS
/// refuses to create a directory whose name is not valid UTF-8 (`EILSEQ`),
/// so a filesystem test cannot reach this branch on the reference machine
/// even though the behaviour matters on Linux; this function is what a unit
/// test drives directly with a hand-built `OsString`.
fn decode_entry_name(raw: std::ffi::OsString, problems: &mut Vec<String>) -> Option<String> {
    match raw.into_string() {
        Ok(name) => Some(name),
        Err(raw) => {
            problems.push(format!(
                "a directory entry name is not valid UTF-8: {}",
                raw.to_string_lossy()
            ));
            None
        }
    }
}

/// Directory entries within an already-open `read_dir` iterator that are
/// themselves directories, decoded to `String` — the rule both listings
/// below share and must not drift on: `DirEntry::file_type`, never
/// `Path::is_dir`, so a directory reached only through a symbolic link is
/// excluded from both. Deliberately does **not** apply the active listing's
/// `archive`-name exclusion or the archived listing's dot-prefix exclusion:
/// those differ between the two listings on purpose, and folding them in
/// here is exactly how a later "simplification" would collapse that
/// difference. Every inner directory-entry read is `.flatten()`-based, per
/// design.md -> Boundaries: an unreadable *entry* within an otherwise good
/// listing just means "skip it", the way `resolve::nvm_candidates` treats
/// every inner `Err`.
fn directory_names(read_dir: std::fs::ReadDir, problems: &mut Vec<String>) -> Vec<String> {
    let mut names = Vec::new();
    for entry in read_dir.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        if let Some(name) = decode_entry_name(entry.file_name(), problems) {
            names.push(name);
        }
    }
    names
}

/// Active change names directly under `<repo>/openspec/changes/`: every
/// directory entry other than one named exactly `archive`, sorted ascending
/// by byte order. `(names, problems, unreadable)`, where `unreadable` is
/// true only when the directory exists but could not itself be read — the
/// signal `archived_entries` needs to short-circuit rather than report the
/// same permission failure a second time. An absent `openspec/changes/` is
/// a supported empty state, matching `openspec list --json`'s own
/// treatment (`dist/core/list.js`).
pub(crate) fn active_change_names(repo: &std::path::Path) -> (Vec<String>, Vec<String>, bool) {
    let dir = repo.join("openspec").join("changes");
    let mut problems = Vec::new();

    let mut names = match std::fs::read_dir(&dir) {
        Ok(read_dir) => directory_names(read_dir, &mut problems),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (Vec::new(), problems, false),
        Err(e) => {
            problems.push(format!("{} could not be read: {e}", dir.display()));
            return (Vec::new(), problems, true);
        }
    };

    names.retain(|name| name != "archive");
    names.sort();
    (names, problems, false)
}

/// One archived change: its date and stripped name — see
/// [`split_archive_name`] — and the archived directory it was found in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArchivedEntry {
    pub(crate) date: Option<String>,
    pub(crate) name: String,
    pub(crate) dir: PathBuf,
}

/// Archived entries directly under `<repo>/openspec/changes/archive/`,
/// excluding dot-prefixed names (unlike the active listing — the two are
/// deliberately opposite, see `change-enumeration`'s spec), ordered dated
/// entries newest first with same-date ties broken by name descending, then
/// every undated entry ordered among itself by name descending, then
/// truncated to `archived_count`.
///
/// `active_unreadable` short-circuits the read entirely when the caller
/// already found `openspec/changes/` itself unreadable: `read_dir` on the
/// archive directory beneath an unreadable parent fails with a permission
/// error of its own (`EACCES`, not `NotFound`), which would otherwise
/// record a second, redundant problem describing the same fault.
pub(crate) fn archived_entries(
    repo: &std::path::Path,
    archived_count: usize,
    active_unreadable: bool,
) -> (Vec<ArchivedEntry>, Vec<String>) {
    if active_unreadable {
        return (Vec::new(), Vec::new());
    }

    let dir = repo.join("openspec").join("changes").join("archive");
    let mut problems = Vec::new();

    let raw_names = match std::fs::read_dir(&dir) {
        Ok(read_dir) => directory_names(read_dir, &mut problems),
        // Absent, or present as a regular file rather than a directory — an
        // `archive` that is a plain file is simply not an archive, not a
        // fault, matching `openspec/changes/archive/`'s absence.
        Err(e)
            if e.kind() == std::io::ErrorKind::NotFound
                || e.kind() == std::io::ErrorKind::NotADirectory =>
        {
            return (Vec::new(), problems);
        }
        Err(e) => {
            problems.push(format!("{} could not be read: {e}", dir.display()));
            return (Vec::new(), problems);
        }
    };

    let mut entries: Vec<ArchivedEntry> = raw_names
        .into_iter()
        .filter(|name| !name.starts_with('.'))
        .map(|raw| {
            let (date, name) = split_archive_name(&raw);
            ArchivedEntry {
                date,
                name,
                dir: dir.join(&raw),
            }
        })
        .collect();

    entries.sort_by(|a, b| match (&a.date, &b.date) {
        (Some(date_a), Some(date_b)) => date_b.cmp(date_a).then_with(|| b.name.cmp(&a.name)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => b.name.cmp(&a.name),
    });
    entries.truncate(archived_count);

    (entries, problems)
}

/// The composition group 8's `from_files` calls: active names, ordered and
/// truncated archived entries, and every problem from either listing —
/// `active_change_names`' plus `archived_entries`', with the short-circuit
/// above already applied.
pub(crate) fn list_changes(
    repo: &std::path::Path,
    archived_count: usize,
) -> (Vec<String>, Vec<ArchivedEntry>, Vec<String>) {
    let (active, mut problems, active_unreadable) = active_change_names(repo);
    let (archived, archived_problems) = archived_entries(repo, archived_count, active_unreadable);
    problems.extend(archived_problems);
    (active, archived, problems)
}

/// The human-readable problem a `schema::LoadError` renders as. Mirrors
/// `schema::load_error_problem`'s wording — private to that module, so this
/// change reproduces the shape rather than exporting it, since `schema` is
/// consumed unchanged (design.md -> Boundaries).
fn schema_load_problem(err: &crate::schema::LoadError) -> String {
    match err {
        crate::schema::LoadError::NotVendored { path } => {
            format!("{} is not vendored: no schema.yaml there", path.display())
        }
        crate::schema::LoadError::Unreadable { path, reason } => {
            format!("{} could not be read: {reason}", path.display())
        }
        crate::schema::LoadError::Invalid { path, reason } => {
            format!("{} is not a usable schema: {reason}", path.display())
        }
        crate::schema::LoadError::IllegalName { name } => {
            format!("{name:?} is not a legal schema name")
        }
    }
}

/// One schema, loaded at most once per name per `from_files` call: the
/// schema itself (`None` when it did not load) and every problem loading it
/// produced — a `parse`-level per-entry problem on success, or the one
/// rendered `LoadError` message on failure.
struct CachedSchemaLoad {
    schema: Option<crate::schema::Schema>,
    problems: Vec<String>,
}

/// Look `name` up in `cache`, loading it from `repo` on a miss. The cache is
/// a plain `HashMap` owned by one `from_files` call, never a `static` —
/// `resolve::BinCache`'s reason: the suite runs the crate's tests in
/// parallel threads of one process, and a process-global cache would let
/// the first test decide the answer for every other.
fn load_schema_cached<'a>(
    repo: &std::path::Path,
    name: &str,
    cache: &'a mut std::collections::HashMap<String, CachedSchemaLoad>,
) -> &'a CachedSchemaLoad {
    cache
        .entry(name.to_string())
        .or_insert_with(|| match crate::schema::load(repo, name) {
            Ok(parsed) => CachedSchemaLoad {
                schema: Some(parsed.schema),
                problems: parsed.problems,
            },
            Err(err) => CachedSchemaLoad {
                schema: None,
                problems: vec![schema_load_problem(&err)],
            },
        })
}

/// Build one `Change` from a directory already known to be active or
/// archived: select and load its schema (through the per-call cache),
/// resolve its artifacts, and count its tasks — applying the CLI's fallback
/// through `change_progress` even when the schema failed to load, so a
/// change with an unvendored schema still reports a real progress pair
/// rather than `0/0`. Problems accumulate in the order `schema::resolve`
/// already establishes: selection, then load, then artifacts, then tasks.
///
/// `degraded-states` removes the vestigial `#[allow(clippy::too_many_arguments)]` that
/// once sat here (design.md -> Decision 13): `build_change` takes seven parameters, and
/// the lint's default threshold fires at eight, not seven — the allow suppressed nothing.
fn build_change(
    repo: &std::path::Path,
    dir: &std::path::Path,
    name: &str,
    origin: Origin,
    project_config_path: &std::path::Path,
    project_config_text: &crate::schema::FileText,
    schema_cache: &mut std::collections::HashMap<String, CachedSchemaLoad>,
) -> Change {
    let change_config_path = dir.join(".openspec.yaml");
    let change_config_text = crate::schema::read_file(&change_config_path);

    let selection = crate::schema::declared_name(
        Some((change_config_path.as_path(), &change_config_text)),
        (project_config_path, project_config_text),
    );

    let mut problems = selection.problems;

    let cached = load_schema_cached(repo, &selection.name, schema_cache);
    problems.extend(cached.problems.iter().cloned());

    let (artifacts, artifact_problems) = match &cached.schema {
        Some(schema) => change_artifacts(dir, schema),
        None => (Vec::new(), Vec::new()),
    };
    problems.extend(artifact_problems);

    let tasks_artifact = cached
        .schema
        .as_ref()
        .and_then(|schema| schema.tasks.as_ref());
    let (progress, task_problems) = change_progress(dir, tasks_artifact);
    problems.extend(task_problems);

    Change {
        name: name.to_string(),
        dir: dir.to_path_buf(),
        origin,
        schema: selection.name,
        artifacts,
        progress,
        problems,
    }
}

/// One `openspec list --json` entry, reduced to what `from_cli` needs: the
/// name and the task-progress pair. `lastModified` and `status` are read and
/// discarded by `parse_list` — `change-model` forbids storing either.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ListEntry {
    pub(crate) name: String,
    pub(crate) progress: crate::tasks::Progress,
}

/// `openspec list --json`'s envelope, parsed: the repository root it
/// answered for (when the envelope carries a usable one), the well-formed
/// entries in payload order, and one problem per entry `parse_list` could
/// not use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ListPayload {
    pub(crate) root: Option<PathBuf>,
    pub(crate) changes: Vec<ListEntry>,
    pub(crate) problems: Vec<String>,
}

/// Parse `openspec list --json`'s stdout: the envelope
/// `{"changes": [...], "root": {"path", "source"}}`, never a bare array. See
/// `cli-changes` -> "The list payload is an envelope, and progress comes
/// from it".
pub(crate) fn parse_list(text: &str) -> Result<ListPayload, String> {
    let value: serde_json::Value = serde_json::from_str(text)
        .map_err(|e| format!("openspec list --json payload is not valid JSON: {e}"))?;
    let obj = value
        .as_object()
        .ok_or_else(|| "openspec list --json payload is not a JSON object".to_string())?;
    let entries = obj
        .get("changes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "openspec list --json payload has no \"changes\" array".to_string())?;

    let root = obj
        .get("root")
        .and_then(|v| v.as_object())
        .and_then(|r| r.get("path"))
        .and_then(|p| p.as_str())
        .map(PathBuf::from);

    let mut changes = Vec::new();
    let mut problems = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for (position, entry) in entries.iter().enumerate() {
        let Some(name) = entry
            .as_object()
            .and_then(|o| required_non_empty_str(o, "name"))
        else {
            problems.push(format!(
                "openspec list --json entry at position {position} has no usable \"name\""
            ));
            continue;
        };
        let Some(completed) = non_negative_usize(entry.get("completedTasks")) else {
            problems.push(format!(
                "openspec list --json entry at position {position} ({name:?}) has no usable \"completedTasks\""
            ));
            continue;
        };
        let Some(total) = non_negative_usize(entry.get("totalTasks")) else {
            problems.push(format!(
                "openspec list --json entry at position {position} ({name:?}) has no usable \"totalTasks\""
            ));
            continue;
        };
        if !seen.insert(name.to_string()) {
            problems.push(format!(
                "openspec list --json reported {name:?} more than once; keeping the first"
            ));
            continue;
        }
        changes.push(ListEntry {
            name: name.to_string(),
            progress: crate::tasks::Progress { completed, total },
        });
    }

    Ok(ListPayload {
        root,
        changes,
        problems,
    })
}

/// One `openspec instructions apply --change <name> --json` payload, parsed:
/// the schema name, the change directory, and each schema artifact id's
/// context files, verbatim and in the CLI's own order per key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ApplyPayload {
    pub(crate) schema_name: String,
    pub(crate) change_dir: PathBuf,
    pub(crate) context_files: std::collections::BTreeMap<String, Vec<PathBuf>>,
}

/// A required non-empty string field, read from a JSON object. Shared by
/// `parse_list`'s `name` and `parse_apply`'s `schemaName`/`changeDir`.
fn required_non_empty_str<'a>(
    obj: &'a serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Option<&'a str> {
    obj.get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
}

/// Parse `openspec instructions apply --change <name> --json`'s stdout: a
/// non-empty `schemaName`, a non-empty `changeDir`, and a `contextFiles`
/// object whose every value is an array of strings — including an empty
/// object, the valid "nothing written yet" state. See `cli-changes` ->
/// "A change's artifacts are placed by schema position from `contextFiles`".
pub(crate) fn parse_apply(text: &str) -> Result<ApplyPayload, String> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|e| {
        format!("openspec instructions apply --json payload is not valid JSON: {e}")
    })?;
    let obj = value.as_object().ok_or_else(|| {
        "openspec instructions apply --json payload is not a JSON object".to_string()
    })?;

    let schema_name = required_non_empty_str(obj, "schemaName").ok_or_else(|| {
        "openspec instructions apply --json payload has no usable \"schemaName\"".to_string()
    })?;
    if !crate::schema::is_legal_name(schema_name) {
        return Err(format!(
            "openspec instructions apply --json payload's \"schemaName\" is not a legal schema name: {schema_name}"
        ));
    }
    let schema_name = schema_name.trim().to_string();

    let change_dir = required_non_empty_str(obj, "changeDir").ok_or_else(|| {
        "openspec instructions apply --json payload has no usable \"changeDir\"".to_string()
    })?;
    let change_dir = PathBuf::from(change_dir);

    let context_files_obj = obj
        .get("contextFiles")
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            "openspec instructions apply --json payload has no usable \"contextFiles\"".to_string()
        })?;

    let mut context_files = std::collections::BTreeMap::new();
    for (key, value) in context_files_obj {
        let array = value.as_array().ok_or_else(|| {
            format!("openspec instructions apply --json payload's \"contextFiles.{key}\" is not an array")
        })?;
        let mut paths = Vec::with_capacity(array.len());
        for entry in array {
            let s = entry.as_str().ok_or_else(|| {
                format!(
                    "openspec instructions apply --json payload's \"contextFiles.{key}\" holds a non-string entry"
                )
            })?;
            paths.push(PathBuf::from(s));
        }
        context_files.insert(key.clone(), paths);
    }

    Ok(ApplyPayload {
        schema_name,
        change_dir,
        context_files,
    })
}

/// Parse `openspec schema which <name> --json`'s stdout: the whole of
/// `text` as one JSON document — no line skipping, no prefix trimming, so a
/// future release moving its "experimental" note from stderr to stdout
/// degrades visibly rather than being silently absorbed. Requires an object
/// with a non-empty string `path`; `source` and `shadows` are ignored. See
/// `schema-cli-fallback` -> "A not-vendored schema is repaired through
/// `openspec schema which`".
pub(crate) fn parse_schema_which(text: &str) -> Result<PathBuf, String> {
    let value: serde_json::Value = serde_json::from_str(text)
        .map_err(|e| format!("openspec schema which --json payload is not valid JSON: {e}"))?;
    let obj = value
        .as_object()
        .ok_or_else(|| "openspec schema which --json payload is not a JSON object".to_string())?;
    let path = required_non_empty_str(obj, "path")
        .ok_or_else(|| "openspec schema which --json payload has no usable \"path\"".to_string())?;
    Ok(PathBuf::from(path))
}

/// Place each schema artifact's `contextFiles` paths at its schema position:
/// one [`ArtifactRef`] per `schema.artifacts` entry, in the schema's
/// declared order, whose `paths` are `context_files[id]` verbatim when the
/// key is present and empty otherwise — a missing key is the normal
/// "No content yet" state, not a problem. One problem is recorded per
/// `context_files` key no schema artifact declares. See `cli-changes` ->
/// "A change's artifacts are placed by schema position from `contextFiles`".
pub(crate) fn cli_artifacts(
    schema: &crate::schema::Schema,
    context_files: &std::collections::BTreeMap<String, Vec<PathBuf>>,
) -> (Vec<ArtifactRef>, Vec<String>) {
    let tasks_index = tasks_index(schema);
    let artifacts: Vec<ArtifactRef> = schema
        .artifacts
        .iter()
        .enumerate()
        .map(|(index, artifact)| ArtifactRef {
            id: artifact.id.clone(),
            paths: context_files.get(&artifact.id).cloned().unwrap_or_default(),
            tracks_tasks: tasks_index == Some(index),
        })
        .collect();

    let schema_ids: std::collections::HashSet<&str> =
        schema.artifacts.iter().map(|a| a.id.as_str()).collect();
    let problems = context_files
        .keys()
        .filter(|key| !schema_ids.contains(key.as_str()))
        .map(|key| format!("contextFiles reported {key:?}, which the schema does not declare"))
        .collect();

    (artifacts, problems)
}

/// Join the two producers' artifact lists by **index**, never by path and
/// never by id. See `change-merge` -> "Artifact lists are joined by
/// position, never by path and never by id" for the six rules, applied in
/// order.
pub(crate) fn join_artifacts(
    file: &[ArtifactRef],
    cli: &[ArtifactRef],
) -> (Vec<ArtifactRef>, Option<String>) {
    if file.is_empty() && cli.is_empty() {
        return (Vec::new(), None);
    }
    if cli.is_empty() {
        return (file.to_vec(), None);
    }
    if file.is_empty() {
        return (cli.to_vec(), None);
    }
    if file.len() != cli.len() {
        return (
            file.to_vec(),
            Some(format!(
                "the file artifact list has {} entries and the CLI's has {}; keeping the file list",
                file.len(),
                cli.len()
            )),
        );
    }
    for (index, (f, c)) in file.iter().zip(cli.iter()).enumerate() {
        if f.id != c.id {
            return (
                file.to_vec(),
                Some(format!(
                    "the file and CLI artifact lists disagree at index {index}: {:?} vs {:?}; keeping the file list",
                    f.id, c.id
                )),
            );
        }
    }
    (cli.to_vec(), None)
}

/// The first non-blank line of `stderr` whose trimmed form does not begin
/// `Note: `, trimmed — or `None` when no such line exists (a blank
/// `stderr`, or one holding nothing but `Note:` banners). `openspec schema
/// which` writes `Note: Schema commands are experimental and may change.`
/// to stderr on every invocation, success and failure alike, so skipping
/// it is not optional: without the skip, every failure the fallback tier
/// reports would end with a banner that looks like an explanation and is
/// not. See `cli-changes` -> "Every CLI failure degrades to the file
/// result and names itself".
fn first_diagnostic_line(stderr: &str) -> Option<&str> {
    stderr
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("Note: "))
}

/// Render a CLI invocation failure as one problem naming `subject` (what was
/// being resolved), the argument vector, and, for the two shapes
/// `subprocess-seam`'s `CliError` carries: for `NotStarted`, the operating
/// system's own message about the spawn itself, appended unconditionally —
/// it distinguishes a binary deleted between probe and run from one with an
/// unusable interpreter line; for `Failed`, the exit code and (whenever one
/// exists) `first_diagnostic_line` of stderr, appended after the code.
/// `openspec`'s own diagnostics still go to stdout, which `Failed` never
/// carries, so a domain failure (an unknown schema, say) still names only
/// its exit code — but a failure of the interpreter running `openspec`
/// itself (an unrunnable shim, `code: Some(127)`) is self-diagnosing on
/// stderr, and dropping that stderr unconditionally left the one failure in
/// the crate that explains itself rendering as an unexplained number. See
/// `cli-changes` -> "Every CLI failure degrades to the file result and
/// names itself".
fn cli_error_problem(subject: &str, args: &[&str], err: &crate::cli::CliError) -> String {
    let vector = args.join(" ");
    match err {
        crate::cli::CliError::NotStarted {
            program,
            args: _,
            reason,
        } => {
            format!("{subject}: could not start {program} for openspec {vector}: {reason}")
        }
        crate::cli::CliError::Failed {
            program: _,
            args: _,
            code,
            stderr,
        } => {
            let code = code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            match first_diagnostic_line(stderr) {
                Some(line) => {
                    format!("{subject}: openspec {vector} exited with code {code}: {line}")
                }
                None => format!("{subject}: openspec {vector} exited with code {code}"),
            }
        }
    }
}

/// One schema resolved through `from_cli`'s CLI-fallback tier: `None` when
/// it did not resolve at all, plus every problem resolving it produced.
struct CachedCliSchema {
    schema: Option<crate::schema::Schema>,
    problems: Vec<String>,
}

/// The uncached half of [`resolve_cli_schema`]: repository tier, then — on
/// `LoadError::NotVendored` only — the `openspec schema which` tier. See
/// `schema-cli-fallback` -> "A not-vendored schema is repaired through
/// `openspec schema which`" and "Every failure of the fallback tier
/// degrades and names itself".
fn resolve_cli_schema_uncached(
    cli: &dyn crate::cli::OpenspecCli,
    repo: &std::path::Path,
    name: &str,
) -> CachedCliSchema {
    match crate::schema::load(repo, name) {
        Ok(parsed) => CachedCliSchema {
            schema: Some(parsed.schema),
            problems: parsed.problems,
        },
        Err(crate::schema::LoadError::NotVendored { path: _ }) => {
            let args = ["schema", "which", name, "--json"];
            match cli.run(&args) {
                Ok(text) => match parse_schema_which(&text) {
                    Ok(dir) => match crate::schema::load_dir(&dir, name) {
                        Ok(parsed) => CachedCliSchema {
                            schema: Some(parsed.schema),
                            problems: parsed.problems,
                        },
                        Err(err) => CachedCliSchema {
                            schema: None,
                            problems: vec![schema_load_problem(&err)],
                        },
                    },
                    Err(reason) => CachedCliSchema {
                        schema: None,
                        problems: vec![format!(
                            "openspec schema which {name:?} --json payload is unusable: {reason}"
                        )],
                    },
                },
                Err(err) => CachedCliSchema {
                    schema: None,
                    problems: vec![cli_error_problem(&format!("schema {name:?}"), &args, &err)],
                },
            }
        }
        Err(err @ crate::schema::LoadError::Unreadable { path: _, reason: _ })
        | Err(err @ crate::schema::LoadError::Invalid { path: _, reason: _ })
        | Err(err @ crate::schema::LoadError::IllegalName { name: _ }) => CachedCliSchema {
            schema: None,
            problems: vec![schema_load_problem(&err)],
        },
    }
}

/// Resolve `name`'s schema through the repository tier, then — on a miss —
/// the CLI fallback tier, caching the result (success or failure) by name
/// for the duration of one `from_cli` call, never a `static`
/// (`resolve::BinCache`'s reason: the suite runs this crate's tests in
/// parallel threads of one process). Every caller sharing `cache` for the
/// same `name` receives a clone of the same problems, not only the first.
/// See `schema-cli-fallback` -> "A schema is resolved at most once per name
/// per call".
fn resolve_cli_schema(
    cli: &dyn crate::cli::OpenspecCli,
    repo: &std::path::Path,
    name: &str,
    cache: &mut std::collections::HashMap<String, CachedCliSchema>,
) -> (Option<crate::schema::Schema>, Vec<String>) {
    let cached = cache
        .entry(name.to_string())
        .or_insert_with(|| resolve_cli_schema_uncached(cli, repo, name));
    (cached.schema.clone(), cached.problems.clone())
}

/// Sort `changes` by `name` ascending, comparing bytes — the order both
/// producers must share (`change-enumeration`). Shared by [`from_cli`] and
/// [`merge`] so the byte-order rule is stated once.
fn sort_by_name_byte_order(changes: &mut [Change]) {
    changes.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
}

/// Every change the OpenSpec CLI reported, and every problem producing them.
/// Carries no `archived` list: `openspec list --json` filters `archive` out
/// of its own walk, so archived changes stay permanently file-sourced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliChanges {
    pub active: Vec<Change>,
    pub problems: Vec<String>,
}

/// Which changes the CLI must be re-asked about on a cycle. Describes CLI
/// work only: the file producer always re-reads every file, since walking
/// `openspec/changes/` costs well under a millisecond and is the only way to
/// learn a change appeared or vanished. `refresh-worker` states the full
/// contract; `union` (group 3) is what the worker uses to coalesce requests
/// that queued while it was busy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    All,
    Only(std::collections::BTreeSet<String>),
}

impl Selection {
    /// Pure and total: `All` absorbs anything, and two `Only` sets union.
    /// Never escalates two `Only` sets — including two empty ones — to
    /// `All`.
    pub fn union(self, other: Selection) -> Selection {
        match (self, other) {
            (Selection::All, _) | (_, Selection::All) => Selection::All,
            (Selection::Only(mut a), Selection::Only(b)) => {
                a.extend(b);
                Selection::Only(a)
            }
        }
    }
}

/// Do `a` and `b` name the same directory? Canonicalizes both and compares
/// the canonical forms when the filesystem can resolve both, falling back
/// to comparing the paths as given otherwise — so a symbolic link in either
/// path is not read as a disagreement. Used by the repository-root guard:
/// `resolve::find_repo` walks up from the invocation context's workspace
/// working directory, while the CLI resolves its own root by walking up
/// from the **process** working directory, and `subprocess-seam` forbids
/// the real implementation from setting `current_dir`.
pub(crate) fn same_directory(a: &std::path::Path, b: &std::path::Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => a == b,
    }
}

/// One change's CLI-fetched data, cached across cycles: the schema name, its
/// directory, its placed artifacts, and its problems — everything but
/// `progress`, which always comes from the fresh `openspec list --json`
/// entry rather than from here, so a mis-classified path can never leave a
/// change's `[completed/total]` stale. `#[derive(Default)]` only through
/// `CliCache` below; not one of `change-model`'s no-`Default` types.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CliCacheEntry {
    dir: PathBuf,
    schema: String,
    artifacts: Vec<ArtifactRef>,
    problems: Vec<String>,
}

/// The one cache `live-refresh` adds: per-change CLI results, keyed by name,
/// held in memory for as long as the worker that owns it runs. Implements
/// `Default` — clippy's `new_without_default` would demand it anyway, and
/// `change-model`'s no-`Default` gate covers `Change`, `ChangeSet`,
/// `ArtifactRef`, and `Origin` in this file, none of which this is. See
/// `specs/refresh-worker/spec.md`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CliCache {
    entries: std::collections::HashMap<String, CliCacheEntry>,
}

/// Build one `Change` from CLI-sourced parts, naming every field explicitly
/// with no `Default` and no `..` — the one construction site both the
/// freshly-fetched and the cache-reused branches of [`from_cli_cached`]
/// share, so `cli-changes`' "every field comes from CLI data" rule has
/// exactly one place to hold.
fn build_cli_change(
    name: String,
    dir: PathBuf,
    schema: String,
    artifacts: Vec<ArtifactRef>,
    progress: crate::tasks::Progress,
    problems: Vec<String>,
) -> Change {
    Change {
        name,
        dir,
        origin: Origin::Active,
        schema,
        artifacts,
        progress,
        problems,
    }
}

/// Build the CLI's view of the active changes, re-asking about only the
/// changes `selection` names or that `cache` holds no entry for.
/// `openspec list --json` runs unconditionally, on every call — it carries
/// the repository-root guard and every change's fresh progress pair, which
/// is why a mis-classified path costs nothing but one extra CLI cycle
/// rather than a stale number. Total — never a `Result`, never panics,
/// never `unwrap`s on any input. See `specs/refresh-worker/spec.md` for the
/// full contract this function implements.
pub fn from_cli_cached(
    cli: &dyn crate::cli::OpenspecCli,
    repo: &std::path::Path,
    selection: &Selection,
    cache: &mut CliCache,
) -> CliChanges {
    let list_args = ["list", "--json"];
    let list_text = match cli.run(&list_args) {
        Ok(text) => text,
        Err(err) => {
            return CliChanges {
                active: Vec::new(),
                problems: vec![cli_error_problem("openspec list --json", &list_args, &err)],
            };
        }
    };

    let list_payload = match parse_list(&list_text) {
        Ok(payload) => payload,
        Err(reason) => {
            return CliChanges {
                active: Vec::new(),
                problems: vec![format!(
                    "openspec list --json payload is unusable: {reason}"
                )],
            };
        }
    };

    let root_agrees = list_payload
        .root
        .as_ref()
        .is_some_and(|root| same_directory(root, repo));
    if !root_agrees {
        let reported = list_payload
            .root
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<no root reported>".to_string());
        return CliChanges {
            active: Vec::new(),
            problems: vec![format!(
                "openspec list --json reported repository root {reported:?}, which disagrees with the resolved root {:?}",
                repo.display().to_string()
            )],
        };
    }

    let mut problems = list_payload.problems;
    let mut active = Vec::with_capacity(list_payload.changes.len());
    let mut schema_cache: std::collections::HashMap<String, CachedCliSchema> =
        std::collections::HashMap::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for entry in &list_payload.changes {
        seen.insert(entry.name.clone());

        let selected = match selection {
            Selection::All => true,
            Selection::Only(names) => names.contains(&entry.name),
        };
        if !selected && let Some(cached) = cache.entries.get(&entry.name) {
            active.push(build_cli_change(
                entry.name.clone(),
                cached.dir.clone(),
                cached.schema.clone(),
                cached.artifacts.clone(),
                entry.progress,
                cached.problems.clone(),
            ));
            continue;
        }

        let apply_args = [
            "instructions",
            "apply",
            "--change",
            entry.name.as_str(),
            "--json",
        ];
        let apply_text = match cli.run(&apply_args) {
            Ok(text) => text,
            Err(err) => {
                problems.push(cli_error_problem(
                    &format!("change {:?}", entry.name),
                    &apply_args,
                    &err,
                ));
                continue;
            }
        };

        let apply = match parse_apply(&apply_text) {
            Ok(payload) => payload,
            Err(reason) => {
                problems.push(format!(
                    "the apply payload for change {:?} is unusable: {reason}",
                    entry.name
                ));
                continue;
            }
        };

        let dir_final = apply
            .change_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if dir_final != entry.name {
            problems.push(format!(
                "the apply payload for change {:?} reports changeDir {:?}, whose final component is not the change's name",
                entry.name, apply.change_dir
            ));
            continue;
        }

        let (schema, mut change_problems) =
            resolve_cli_schema(cli, repo, &apply.schema_name, &mut schema_cache);

        let (artifacts, artifact_problems) = match &schema {
            Some(schema) => cli_artifacts(schema, &apply.context_files),
            None => (Vec::new(), Vec::new()),
        };
        change_problems.extend(artifact_problems);

        cache.entries.insert(
            entry.name.clone(),
            CliCacheEntry {
                dir: apply.change_dir.clone(),
                schema: apply.schema_name.clone(),
                artifacts: artifacts.clone(),
                problems: change_problems.clone(),
            },
        );

        active.push(build_cli_change(
            entry.name.clone(),
            apply.change_dir,
            apply.schema_name,
            artifacts,
            entry.progress,
            change_problems,
        ));
    }

    cache.entries.retain(|name, _| seen.contains(name));
    sort_by_name_byte_order(&mut active);

    CliChanges { active, problems }
}

/// Build the CLI's view of the active changes: `openspec list --json`, the
/// repository-root guard, then one `openspec instructions apply` call per
/// change, schema resolution through [`resolve_cli_schema`], and artifact
/// placement through [`cli_artifacts`]. Total — never a `Result`, never
/// panics, never `unwrap`s on any input. Defined as [`from_cli_cached`] with
/// `Selection::All` and an empty cache, so there is exactly one
/// implementation of the CLI producer. See `cli-changes` for the full
/// contract this function implements.
pub fn from_cli(cli: &dyn crate::cli::OpenspecCli, repo: &std::path::Path) -> CliChanges {
    from_cli_cached(cli, repo, &Selection::All, &mut CliCache::default())
}

/// Concatenate `items`, collapsing exactly-equal strings to their first
/// occurrence and preserving the remaining order. Shared by [`merge`]'s
/// per-change problem list.
fn dedup_preserve_order(items: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    items
        .into_iter()
        .filter(|s| seen.insert(s.clone()))
        .collect()
}

/// Layer `cli` over `files`: `change-merge`'s field table, applied per
/// change paired by name. Pure — no filesystem, no CLI — and total: never a
/// `Result`, never panics. See `change-merge` for the full contract.
pub fn merge(files: ChangeSet, cli: CliChanges) -> ChangeSet {
    let ChangeSet {
        active: file_active,
        archived,
        problems: file_problems,
    } = files;
    let CliChanges {
        active: cli_active,
        problems: cli_problems,
    } = cli;

    let mut cli_by_name: std::collections::HashMap<String, Change> = cli_active
        .into_iter()
        .map(|change| (change.name.clone(), change))
        .collect();

    let mut merged: Vec<Change> = Vec::new();

    for file_change in file_active {
        match cli_by_name.remove(&file_change.name) {
            Some(cli_change) => {
                let (artifacts, join_problem) =
                    join_artifacts(&file_change.artifacts, &cli_change.artifacts);
                let mut combined = file_change.problems;
                combined.extend(cli_change.problems);
                if let Some(problem) = join_problem {
                    combined.push(problem);
                }
                merged.push(Change {
                    name: file_change.name,
                    dir: file_change.dir,
                    origin: Origin::Active,
                    schema: cli_change.schema,
                    artifacts,
                    progress: cli_change.progress,
                    problems: dedup_preserve_order(combined),
                });
            }
            None => merged.push(file_change),
        }
    }

    // Whatever remains in `cli_by_name` is CLI-only: insert unchanged.
    merged.extend(cli_by_name.into_values());

    sort_by_name_byte_order(&mut merged);

    let mut problems = file_problems;
    problems.extend(cli_problems);

    ChangeSet {
        active: merged,
        archived,
        problems,
    }
}

/// The `ChangeSet` for the "no repository found" case: `ui::load` needs one
/// and constructs it here, naming every field explicitly, rather than in
/// `src/ui/mod.rs` — a construction site outside this file would sit
/// outside `change-model`'s no-`Default`/no-`..` gate, which searches this
/// file. See `openspec/changes/tui-shell/design.md` -> Decisions.
pub fn empty_set() -> ChangeSet {
    ChangeSet {
        active: Vec::new(),
        archived: Vec::new(),
        problems: Vec::new(),
    }
}

/// Paint the pane from disk: every active change under
/// `<repo>/openspec/changes/`, the `archived_count` most recent archived
/// changes under its `archive/`, and every problem recorded along the way.
/// Total — never a `Result`, never panics, never `unwrap`s. `openspec/config.yaml`
/// is read once and reused for every change; schemas are cached by name for
/// the duration of this one call. See
/// `openspec/changes/changes-from-files/design.md` for the full contract.
pub fn from_files(repo: &std::path::Path, archived_count: usize) -> ChangeSet {
    let project_config_path = repo.join("openspec").join("config.yaml");
    let project_config_text = crate::schema::read_file(&project_config_path);

    let (active_names, archived_list, problems) = list_changes(repo, archived_count);

    let mut schema_cache: std::collections::HashMap<String, CachedSchemaLoad> =
        std::collections::HashMap::new();

    let active = active_names
        .into_iter()
        .map(|name| {
            let dir = repo.join("openspec").join("changes").join(&name);
            build_change(
                repo,
                &dir,
                &name,
                Origin::Active,
                &project_config_path,
                &project_config_text,
                &mut schema_cache,
            )
        })
        .collect();

    let archived = archived_list
        .into_iter()
        .map(|entry| {
            build_change(
                repo,
                &entry.dir,
                &entry.name,
                Origin::Archived { date: entry.date },
                &project_config_path,
                &project_config_text,
                &mut schema_cache,
            )
        })
        .collect();

    ChangeSet {
        active,
        archived,
        problems,
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
                tracks_tasks: false,
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
            tracks_tasks: false,
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
                tracks_tasks: false,
            },
            ArtifactRef {
                id: "alpha".to_string(),
                paths: vec![],
                tracks_tasks: false,
            },
            ArtifactRef {
                id: "middle".to_string(),
                paths: vec![],
                tracks_tasks: false,
            },
            ArtifactRef {
                id: "zeta".to_string(),
                paths: vec![],
                tracks_tasks: false,
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
    fn ordering_is_byte_order_on_the_full_path_not_pathbuf_component_order() {
        // `PathBuf`'s `Ord` compares components, not raw bytes: `"api"` is a
        // strict prefix of `"api.md"`, so `PathBuf` sorts `specs/api/deep.md`
        // *before* `specs/api.md` — the opposite of what byte-string order on
        // the full path gives, since `.` (0x2E) precedes `/` (0x2F). Found in
        // Change Review: the existing ordering test's fixture never exercised
        // this divergence, so `Vec<PathBuf>::sort()` passed it undetected.
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        write(&dir.join("specs/api.md"), "# api.md\n");
        write(&dir.join("specs/api/deep.md"), "# nested\n");

        let (paths, _) = resolve_artifact(&dir, "specs/**/*.md");
        assert_eq!(
            paths,
            vec![dir.join("specs/api.md"), dir.join("specs/api/deep.md")]
        );
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
                    tracks_tasks: false,
                },
                ArtifactRef {
                    id: "specs".to_string(),
                    paths: vec![],
                    tracks_tasks: false,
                },
                ArtifactRef {
                    id: "design".to_string(),
                    paths: vec![],
                    tracks_tasks: false,
                },
                ArtifactRef {
                    id: "tasks".to_string(),
                    paths: vec![dir.join("tasks.md")],
                    tracks_tasks: false,
                },
                ArtifactRef {
                    id: "planning-review".to_string(),
                    paths: vec![],
                    tracks_tasks: false,
                },
            ]
        );
        assert!(problems.is_empty());
    }

    // --- tasks-tab group 3: the tracked-tasks flag, file producer ---------

    fn schema_with_tasks(
        artifacts: Vec<(&str, &str)>,
        tasks: Option<crate::schema::Artifact>,
    ) -> crate::schema::Schema {
        crate::schema::Schema {
            name: "tdd".to_string(),
            artifacts: artifacts
                .into_iter()
                .map(|(id, generates)| crate::schema::Artifact {
                    id: id.to_string(),
                    generates: generates.to_string(),
                })
                .collect(),
            tasks,
        }
    }

    #[test]
    fn tracks_tasks_tdd() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        let schema = schema_with_tasks(
            vec![
                ("proposal", "proposal.md"),
                ("specs", "specs/**/*.md"),
                ("design", "design.md"),
                ("tasks", "tasks.md"),
                ("planning-review", "planning-review.md"),
            ],
            Some(crate::schema::Artifact {
                id: "tasks".to_string(),
                generates: "tasks.md".to_string(),
            }),
        );

        let (artifacts, _problems) = change_artifacts(&dir, &schema);
        let flags: Vec<bool> = artifacts.iter().map(|a| a.tracks_tasks).collect();
        assert_eq!(flags, vec![false, false, false, true, false]);
        assert_eq!(flags.iter().filter(|&&f| f).count(), 1);
    }

    #[test]
    fn tracks_tasks_prefers_tracks() {
        // `apply.tracks: tasks.md` selects the artifact whose `generates`
        // is `tasks.md` — here `checklist`, at position 0 — even though a
        // different artifact at position 1 carries the id `tasks`. An id
        // comparison would mark position 1 instead; this is the case the
        // schema's own published rule exists to get right.
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        write(&dir.join("tasks.md"), "- [x] a\n");
        let schema = schema_with_tasks(
            vec![("checklist", "tasks.md"), ("tasks", "notes.md")],
            Some(crate::schema::Artifact {
                id: "checklist".to_string(),
                generates: "tasks.md".to_string(),
            }),
        );

        let (artifacts, _problems) = change_artifacts(&dir, &schema);
        assert!(artifacts[0].tracks_tasks);
        assert!(!artifacts[1].tracks_tasks);
        // The marked artifact's own paths still come from its `generates`
        // value — it renders `tasks.md` while being addressed as
        // `checklist`.
        assert_eq!(artifacts[0].paths, vec![dir.join("tasks.md")]);
    }

    #[test]
    fn tracks_tasks_id_fallback() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        let schema = schema_with_tasks(
            vec![("proposal", "proposal.md"), ("tasks", "tasks.md")],
            Some(crate::schema::Artifact {
                id: "tasks".to_string(),
                generates: "tasks.md".to_string(),
            }),
        );

        let (artifacts, _problems) = change_artifacts(&dir, &schema);
        assert!(!artifacts[0].tracks_tasks);
        assert!(artifacts[1].tracks_tasks);
    }

    #[test]
    fn tracks_tasks_none_marked() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        write(&dir.join("tasks.md"), "- [x] a\n- [ ] b\n");
        let schema = schema_with_tasks(vec![("alpha", "alpha.md"), ("beta", "beta.md")], None);

        let (artifacts, _problems) = change_artifacts(&dir, &schema);
        assert!(artifacts.iter().all(|a| !a.tracks_tasks));

        // The marking and the counting are decided separately: the
        // fallback still counts `<change dir>/tasks.md` even though no
        // artifact is marked.
        let (progress, _problems) = change_progress(&dir, schema.tasks.as_ref());
        assert_eq!(
            progress,
            crate::tasks::Progress {
                completed: 1,
                total: 2
            }
        );
    }

    #[test]
    fn tracks_tasks_duplicate_first_only() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        let schema = schema_with_tasks(
            vec![
                ("zeta", "tasks.md"),
                ("alpha", "alpha.md"),
                ("zeta", "tasks.md"),
            ],
            Some(crate::schema::Artifact {
                id: "zeta".to_string(),
                generates: "tasks.md".to_string(),
            }),
        );

        let (artifacts, _problems) = change_artifacts(&dir, &schema);
        assert_eq!(artifacts.len(), 3);
        assert!(artifacts[0].tracks_tasks);
        assert!(!artifacts[1].tracks_tasks);
        assert!(!artifacts[2].tracks_tasks);
    }

    #[test]
    fn tracks_tasks_empty_list() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        let schema = schema_with_tasks(vec![], None);

        let (artifacts, problems) = change_artifacts(&dir, &schema);
        assert!(artifacts.is_empty());
        assert!(problems.is_empty());

        let (progress, _problems) = change_progress(&dir, schema.tasks.as_ref());
        assert_eq!(
            progress,
            crate::tasks::Progress {
                completed: 0,
                total: 0
            }
        );
    }

    // --- group 6: task progress and the CLI's fallback --------------------

    use crate::testutil::snapshot;

    fn tasks_artifact(generates: &str) -> crate::schema::Artifact {
        crate::schema::Artifact {
            id: "tasks".to_string(),
            generates: generates.to_string(),
        }
    }

    #[test]
    fn a_single_tasks_file_gives_the_changes_progress() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        write(&dir.join("tasks.md"), "- [x] a\n- [ ] b\n");

        let artifact = tasks_artifact("tasks.md");
        let (progress, problems) = change_progress(&dir, Some(&artifact));
        assert_eq!(
            progress,
            crate::tasks::Progress {
                completed: 1,
                total: 2
            }
        );
        assert!(problems.is_empty());
    }

    #[test]
    fn a_glob_shaped_tasks_artifact_sums_across_its_files() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        write(&dir.join("tasks.md"), "- [x] a\n- [ ] b\n");
        write(&dir.join("sub/tasks.md"), "- [x] a\n- [x] b\n- [ ] c\n");
        write(&dir.join("sub/deeper/tasks.md"), "- [ ] a\n");

        // Absolute pair, not self-consistent: the same tree, driven through
        // the real `openspec list --json` at planning time, reported
        // `completedTasks: 3, totalTasks: 6` for the identical shape (see
        // design.md -> Test Strategy, "A glob-shaped tasks artifact sums
        // across its files").
        let artifact = tasks_artifact("**/tasks.md");
        let (progress, problems) = change_progress(&dir, Some(&artifact));
        assert_eq!(
            progress,
            crate::tasks::Progress {
                completed: 3,
                total: 6
            }
        );
        assert!(problems.is_empty());
    }

    #[test]
    fn a_schema_declaring_no_tasks_artifact_still_counts_tasks_md() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        write(&dir.join("tasks.md"), "- [x] a\n- [ ] b\n");

        let (progress, problems) = change_progress(&dir, None);
        assert_eq!(
            progress,
            crate::tasks::Progress {
                completed: 1,
                total: 2
            }
        );
        assert!(problems.is_empty());
    }

    #[test]
    fn a_schema_that_failed_to_load_still_counts_tasks_md() {
        // From `change_progress`'s side this is the identical input as "no
        // tasks artifact declared" — both mean "no artifact is available" —
        // but the *caller* reaches `None` for two different reasons
        // (`schema::LoadError` versus no `apply.tracks`/id-`tasks` match),
        // and both must fall back identically.
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        write(&dir.join("tasks.md"), "- [x] a\n- [ ] b\n");

        let (progress, problems) = change_progress(&dir, None);
        assert_eq!(
            progress,
            crate::tasks::Progress {
                completed: 1,
                total: 2
            }
        );
        assert!(problems.is_empty());
    }

    #[test]
    fn a_tasks_artifact_whose_glob_matched_nothing_still_counts_tasks_md() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        write(&dir.join("tasks.md"), "- [x] a\n- [ ] b\n");
        // A genuinely different tree from the two `None` variants above:
        // `Some` is passed, but its glob matches nothing, so the fallback
        // fires for a different reason.
        let artifact = tasks_artifact("**/nonexistent-tasks-glob.md");

        let (progress, problems) = change_progress(&dir, Some(&artifact));
        assert_eq!(
            progress,
            crate::tasks::Progress {
                completed: 1,
                total: 2
            }
        );
        assert!(problems.is_empty());
    }

    #[test]
    fn an_unreadable_tasks_file_is_zero_plus_one_named_problem() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        mkdir(&dir.join("tasks.md")); // a directory where a file was expected

        let (progress, problems) = change_progress(&dir, None);
        assert_eq!(
            progress,
            crate::tasks::Progress {
                completed: 0,
                total: 0
            }
        );
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains(&dir.join("tasks.md").display().to_string()));
    }

    #[test]
    fn a_change_with_no_tasks_file_at_all_is_zero_not_complete() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        mkdir(&dir);

        let (progress, problems) = change_progress(&dir, None);
        assert_eq!(
            progress,
            crate::tasks::Progress {
                completed: 0,
                total: 0
            }
        );
        assert!(problems.is_empty());
        assert!(!progress.is_complete());
    }

    #[test]
    fn a_change_directory_is_byte_identical_after_resolution_and_counting() {
        let scratch = ScratchDir::new();
        let dir = canonical(scratch.path());
        write(&dir.join("specs/alpha/spec.md"), "# Alpha\n");
        write(&dir.join("tasks.md"), "- [x] a\n- [ ] b\n");

        let artifact = tasks_artifact("tasks.md");
        let before = snapshot(&dir);
        let _ = change_progress(&dir, Some(&artifact));
        let _ = change_progress(&dir, Some(&artifact));
        let _ = change_progress(&dir, Some(&artifact));
        let after = snapshot(&dir);
        assert_eq!(before, after);

        // A change directory with no `tasks.md` at all: the fallback names
        // exactly this path for every change whose glob matched nothing,
        // and it must never be created.
        let empty_scratch = ScratchDir::new();
        let empty_dir = canonical(empty_scratch.path());
        mkdir(&empty_dir);
        let before_empty = snapshot(&empty_dir);
        let _ = change_progress(&empty_dir, None);
        let after_empty = snapshot(&empty_dir);
        assert_eq!(before_empty, after_empty);
        assert!(!empty_dir.join("tasks.md").exists());
    }

    // --- group 7: enumerating active and archived changes -----------------

    fn archived_names(entries: &[ArchivedEntry]) -> Vec<&str> {
        entries.iter().map(|e| e.name.as_str()).collect()
    }

    #[test]
    fn a_directory_with_no_marker_file_is_a_change() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        mkdir(&repo.join("openspec/changes/empty-dir"));
        write(&repo.join("openspec/changes/only-yaml/.openspec.yaml"), "");
        write(&repo.join("openspec/changes/only-proposal/proposal.md"), "");

        let (active, problems, unreadable) = active_change_names(&repo);
        assert_eq!(active, vec!["empty-dir", "only-proposal", "only-yaml"]);
        assert!(problems.is_empty());
        assert!(!unreadable);
    }

    #[test]
    fn a_regular_file_is_not_a_change() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        write(&repo.join("openspec/changes/notes.md"), "notes");
        mkdir(&repo.join("openspec/changes/real-change"));

        let (active, problems, _) = active_change_names(&repo);
        assert_eq!(active, vec!["real-change"]);
        assert!(problems.is_empty());
    }

    #[test]
    fn a_symbolic_link_to_a_directory_is_not_a_change() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        mkdir(&repo.join("openspec/changes/real-target"));
        symlink(
            &repo.join("openspec/changes/real-target"),
            &repo.join("openspec/changes/linked"),
        );
        symlink(
            &repo.join("openspec/changes/does-not-exist"),
            &repo.join("openspec/changes/dangling"),
        );

        let (active, problems, _) = active_change_names(&repo);
        assert_eq!(active, vec!["real-target"]);
        assert!(problems.is_empty());
    }

    #[test]
    fn the_archive_exclusion_is_by_exact_name() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        mkdir(&repo.join("openspec/changes/archive"));
        mkdir(&repo.join("openspec/changes/archives-not-excluded"));
        mkdir(&repo.join("openspec/changes/archive-notes"));

        let (active, _, _) = active_change_names(&repo);
        assert_eq!(active, vec!["archive-notes", "archives-not-excluded"]);
    }

    #[test]
    fn a_dot_directory_is_a_change() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        mkdir(&repo.join("openspec/changes/.dot-change"));
        mkdir(&repo.join("openspec/changes/plain"));

        let (active, _, _) = active_change_names(&repo);
        assert_eq!(active, vec![".dot-change", "plain"]);
    }

    #[test]
    fn case_and_digits_order_by_byte_not_by_locale() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        for name in ["Beta", "alpha", "10-late", "2-early"] {
            mkdir(&repo.join("openspec/changes").join(name));
        }

        let (active, _, _) = active_change_names(&repo);
        assert_eq!(active, vec!["10-late", "2-early", "Beta", "alpha"]);
    }

    #[test]
    fn a_normal_archived_directory_splits_into_a_date_and_a_name_via_archived_entries() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        mkdir(&repo.join("openspec/changes/archive/2026-08-14-add-token-refresh"));

        let (archived, problems) = archived_entries(&repo, 5, false);
        assert_eq!(archived.len(), 1);
        assert_eq!(archived[0].name, "add-token-refresh");
        assert_eq!(archived[0].date, Some("2026-08-14".to_string()));
        assert!(archived[0].dir.ends_with("2026-08-14-add-token-refresh"));
        assert!(problems.is_empty());
    }

    #[test]
    fn a_dot_prefixed_archive_directory_is_not_an_archived_change() {
        // Both halves in one test, since the two rules are deliberately
        // opposite: `.dot-change/` is listed under `changes/` while
        // `.hidden-archived/` is not listed under `archive/`.
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        mkdir(&repo.join("openspec/changes/.dot-change"));
        mkdir(&repo.join("openspec/changes/archive/.hidden-archived"));
        mkdir(&repo.join("openspec/changes/archive/2026-08-14-real"));

        let (active, _, _) = active_change_names(&repo);
        assert!(active.contains(&".dot-change".to_string()));

        let (archived, _) = archived_entries(&repo, 5, false);
        assert_eq!(archived_names(&archived), vec!["real"]);
    }

    #[test]
    fn two_archived_directories_can_strip_to_the_same_name() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        mkdir(&repo.join("openspec/changes/archive/2026-01-05-retry-policy"));
        mkdir(&repo.join("openspec/changes/archive/2026-07-22-retry-policy"));

        let (archived, _) = archived_entries(&repo, 5, false);
        assert_eq!(archived.len(), 2);
        assert!(archived.iter().all(|e| e.name == "retry-policy"));
        let dates: Vec<&str> = archived
            .iter()
            .map(|e| e.date.as_deref().unwrap())
            .collect();
        assert_eq!(dates, vec!["2026-07-22", "2026-01-05"]);
    }

    #[test]
    fn dated_entries_come_newest_first() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        mkdir(&repo.join("openspec/changes/archive/2026-01-05-a"));
        mkdir(&repo.join("openspec/changes/archive/2026-09-01-c"));
        mkdir(&repo.join("openspec/changes/archive/2026-07-22-b"));

        let (archived, _) = archived_entries(&repo, 5, false);
        assert_eq!(archived_names(&archived), vec!["c", "b", "a"]);
    }

    #[test]
    fn two_entries_sharing_a_date_order_by_name_descending() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        mkdir(&repo.join("openspec/changes/archive/2026-05-01-alpha"));
        mkdir(&repo.join("openspec/changes/archive/2026-05-01-zeta"));

        let (archived, _) = archived_entries(&repo, 5, false);
        assert_eq!(archived_names(&archived), vec!["zeta", "alpha"]);
    }

    #[test]
    fn an_undated_entry_sorts_after_every_dated_one() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        mkdir(&repo.join("openspec/changes/archive/zeta-undated"));
        mkdir(&repo.join("openspec/changes/archive/2026-01-05-a"));
        mkdir(&repo.join("openspec/changes/archive/alpha-undated"));

        let (archived, _) = archived_entries(&repo, 5, false);
        assert_eq!(
            archived_names(&archived),
            vec!["a", "zeta-undated", "alpha-undated"]
        );
    }

    #[test]
    fn the_limit_keeps_the_most_recent_entries() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        for day in 1..=7 {
            mkdir(
                &repo
                    .join("openspec/changes/archive")
                    .join(format!("2026-01-{day:02}-entry")),
            );
        }

        let (archived, _) = archived_entries(&repo, 5, false);
        assert_eq!(
            archived_names(&archived),
            vec!["entry", "entry", "entry", "entry", "entry"]
        );
        let dates: Vec<&str> = archived
            .iter()
            .map(|e| e.date.as_deref().unwrap())
            .collect();
        assert_eq!(
            dates,
            vec![
                "2026-01-07",
                "2026-01-06",
                "2026-01-05",
                "2026-01-04",
                "2026-01-03"
            ]
        );
    }

    #[test]
    fn a_limit_larger_than_the_archive_keeps_everything() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        mkdir(&repo.join("openspec/changes/archive/2026-01-01-a"));
        mkdir(&repo.join("openspec/changes/archive/2026-01-02-b"));

        let (archived, problems) = archived_entries(&repo, 50, false);
        assert_eq!(archived.len(), 2);
        assert!(problems.is_empty());

        let (archived_zero, problems_zero) = archived_entries(&repo, 0, false);
        assert!(archived_zero.is_empty());
        assert!(problems_zero.is_empty());

        let (active, active_problems, _) = active_change_names(&repo);
        assert!(active.is_empty());
        assert!(active_problems.is_empty());
    }

    #[test]
    fn a_repository_with_no_openspec_directory_yields_an_empty_set() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());

        let (active, archived, problems) = list_changes(&repo, 5);
        assert!(active.is_empty());
        assert!(archived.is_empty());
        assert!(problems.is_empty());
        assert!(!repo.join("openspec").exists());
    }

    #[test]
    fn no_openspec_changes_directory_yields_an_empty_set() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        mkdir(&repo.join("openspec"));

        let (active, archived, problems) = list_changes(&repo, 5);
        assert!(active.is_empty());
        assert!(archived.is_empty());
        assert!(problems.is_empty());
    }

    #[test]
    fn no_active_changes_still_lists_the_archive() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        mkdir(&repo.join("openspec/changes/archive/2026-01-01-a"));
        mkdir(&repo.join("openspec/changes/archive/2026-01-02-b"));

        let (active, archived, problems) = list_changes(&repo, 5);
        assert!(active.is_empty());
        assert_eq!(archived.len(), 2);
        assert!(problems.is_empty());
    }

    #[test]
    fn an_archive_that_is_a_regular_file_is_not_an_archive() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        write(&repo.join("openspec/changes/archive"), "not a directory");
        mkdir(&repo.join("openspec/changes/real-change"));

        let (active, archived, problems) = list_changes(&repo, 5);
        assert_eq!(active, vec!["real-change"]);
        assert!(archived.is_empty());
        assert!(problems.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_changes_directory_is_one_named_problem() {
        use std::os::unix::fs::PermissionsExt;

        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        let changes_dir = repo.join("openspec/changes");
        mkdir(&changes_dir.join("archive"));

        std::fs::set_permissions(&changes_dir, std::fs::Permissions::from_mode(0o000))
            .expect("set changes/ unreadable");
        let (active, archived, problems) = list_changes(&repo, 5);
        std::fs::set_permissions(&changes_dir, std::fs::Permissions::from_mode(0o755))
            .expect("restore changes/ permissions");

        assert!(active.is_empty());
        assert!(archived.is_empty());
        // Exactly one problem: the discriminating assertion. An
        // implementation that also walks the archive beneath the
        // unreadable parent records a second, redundant EACCES problem.
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains(&changes_dir.display().to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_archive_leaves_the_active_list_intact() {
        use std::os::unix::fs::PermissionsExt;

        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        mkdir(&repo.join("openspec/changes/one"));
        mkdir(&repo.join("openspec/changes/two"));
        let archive_dir = repo.join("openspec/changes/archive");
        mkdir(&archive_dir);

        std::fs::set_permissions(&archive_dir, std::fs::Permissions::from_mode(0o000))
            .expect("set archive/ unreadable");
        let (active, archived, problems) = list_changes(&repo, 5);
        std::fs::set_permissions(&archive_dir, std::fs::Permissions::from_mode(0o755))
            .expect("restore archive/ permissions");

        assert_eq!(active, vec!["one", "two"]);
        assert!(archived.is_empty());
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains(&archive_dir.display().to_string()));
    }

    #[test]
    fn a_directory_entry_whose_name_is_not_valid_utf_8_is_skipped_not_fatal() {
        use std::os::unix::ffi::OsStringExt;

        let mut problems = Vec::new();
        let normal = decode_entry_name(std::ffi::OsString::from("plain"), &mut problems);
        assert_eq!(normal, Some("plain".to_string()));
        assert!(problems.is_empty());

        let invalid = std::ffi::OsString::from_vec(vec![0x66, 0x6f, 0x80, 0x6f]);
        let decoded = decode_entry_name(invalid, &mut problems);
        assert_eq!(decoded, None);
        assert_eq!(problems.len(), 1);
    }

    #[test]
    fn the_repository_tree_is_byte_identical_after_enumeration() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        mkdir(&repo.join("openspec/changes/one"));
        mkdir(&repo.join("openspec/changes/archive/2026-01-01-a"));

        let before = snapshot(&repo);
        let _ = list_changes(&repo, 5);
        let after = snapshot(&repo);
        assert_eq!(before, after);
        assert!(
            !repo
                .join("openspec/changes/archive-was-never-here")
                .exists()
        );
    }

    // --- group 8: `from_files`, the composition and its degraded states ---

    /// A minimal vendored schema at `<repo>/openspec/schemas/<name>/schema.yaml`.
    fn vendor_schema(repo: &std::path::Path, name: &str, artifacts: &[(&str, &str)]) {
        let mut yaml = format!("name: {name}\nartifacts:\n");
        for (id, generates) in artifacts {
            yaml.push_str(&format!("  - id: {id}\n    generates: {generates}\n"));
        }
        write(
            &repo.join("openspec/schemas").join(name).join("schema.yaml"),
            &yaml,
        );
    }

    fn write_project_config(repo: &std::path::Path, schema: &str) {
        write(
            &repo.join("openspec/config.yaml"),
            &format!("schema: {schema}\n"),
        );
    }

    const TDD_ARTIFACTS: &[(&str, &str)] = &[
        ("proposal", "proposal.md"),
        ("specs", "specs/**/*.md"),
        ("design", "design.md"),
        ("tasks", "tasks.md"),
        ("planning-review", "planning-review.md"),
    ];

    #[test]
    fn a_fully_written_active_change_becomes_one_value() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
        write_project_config(&repo, "tdd");
        write(
            &repo.join("openspec/changes/add-auth/.openspec.yaml"),
            "schema: tdd\n",
        );
        write(&repo.join("openspec/changes/add-auth/proposal.md"), "# P\n");
        write(&repo.join("openspec/changes/add-auth/design.md"), "# D\n");
        write(
            &repo.join("openspec/changes/add-auth/tasks.md"),
            "- [x] a\n- [x] b\n- [x] c\n- [x] d\n- [ ] e\n- [ ] f\n- [ ] g\n- [ ] h\n- [ ] i\n",
        );

        let set = from_files(&repo, 5);
        assert_eq!(set.active.len(), 1);
        let change = &set.active[0];
        assert_eq!(
            change,
            &Change {
                name: "add-auth".to_string(),
                dir: repo.join("openspec/changes/add-auth"),
                origin: Origin::Active,
                schema: "tdd".to_string(),
                artifacts: vec![
                    ArtifactRef {
                        id: "proposal".to_string(),
                        paths: vec![repo.join("openspec/changes/add-auth/proposal.md")],
                        tracks_tasks: false,
                    },
                    ArtifactRef {
                        id: "specs".to_string(),
                        paths: vec![],
                        tracks_tasks: false,
                    },
                    ArtifactRef {
                        id: "design".to_string(),
                        paths: vec![repo.join("openspec/changes/add-auth/design.md")],
                        tracks_tasks: false,
                    },
                    ArtifactRef {
                        id: "tasks".to_string(),
                        paths: vec![repo.join("openspec/changes/add-auth/tasks.md")],
                        tracks_tasks: true,
                    },
                    ArtifactRef {
                        id: "planning-review".to_string(),
                        paths: vec![],
                        tracks_tasks: false,
                    },
                ],
                progress: crate::tasks::Progress {
                    completed: 4,
                    total: 9,
                },
                problems: vec![],
            }
        );
        assert!(set.archived.is_empty());
        assert!(set.problems.is_empty());
        assert_invariants(change);
    }

    #[test]
    fn an_archived_change_carries_the_date_split_off_its_directory_name() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
        write_project_config(&repo, "tdd");
        write(
            &repo.join("openspec/changes/archive/2026-08-14-add-auth/tasks.md"),
            "- [x] a\n- [x] b\n- [x] c\n",
        );

        let set = from_files(&repo, 5);
        assert!(set.active.is_empty());
        assert_eq!(set.archived.len(), 1);
        let change = &set.archived[0];
        assert_eq!(change.name, "add-auth");
        assert_eq!(
            change.origin,
            Origin::Archived {
                date: Some("2026-08-14".to_string())
            }
        );
        assert_eq!(
            change.dir,
            repo.join("openspec/changes/archive/2026-08-14-add-auth")
        );
        assert_eq!(
            change.progress,
            crate::tasks::Progress {
                completed: 3,
                total: 3
            }
        );
        assert!(change.progress.is_complete());
        assert_invariants(change);
    }

    #[test]
    fn a_change_whose_schema_did_not_load_is_still_a_complete_value() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        // No `openspec/schemas/outside-in-tdd/` vendored at all.
        write(
            &repo.join("openspec/changes/learning-tool/.openspec.yaml"),
            "schema: outside-in-tdd\n",
        );
        write(
            &repo.join("openspec/changes/learning-tool/tasks.md"),
            "- [x] a\n- [x] b\n- [ ] c\n",
        );

        let set = from_files(&repo, 5);
        assert_eq!(set.active.len(), 1);
        let change = &set.active[0];
        assert_eq!(change.schema, "outside-in-tdd");
        assert!(change.artifacts.is_empty());
        assert_eq!(
            change.progress,
            crate::tasks::Progress {
                completed: 2,
                total: 3
            }
        );
        assert_eq!(change.problems.len(), 1);
        assert!(change.problems[0].contains("outside-in-tdd"));
        assert_invariants(change);
    }

    #[test]
    fn the_same_repository_read_twice_produces_equal_values_even_after_a_touch() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
        write_project_config(&repo, "tdd");
        write(
            &repo.join("openspec/changes/add-auth/tasks.md"),
            "- [ ] a\n",
        );
        write(&repo.join("README.md"), "# unrelated\n");

        let first = from_files(&repo, 5);

        // Advance an unrelated file's modification time between the two
        // reads, deterministically rather than by sleeping past filesystem
        // mtime granularity. A `Change` carrying a `lastModified` field
        // could not survive this; a plain double read of an untouched tree
        // would pass either way, so the touch is what makes the assertion
        // mean something.
        let readme = repo.join("README.md");
        let current = std::fs::metadata(&readme)
            .expect("read fixture metadata")
            .modified()
            .expect("modified time");
        let advanced = current + std::time::Duration::from_secs(120);
        let file = std::fs::File::options()
            .write(true)
            .open(&readme)
            .expect("open fixture for touching");
        file.set_modified(advanced).expect("advance mtime");

        let second = from_files(&repo, 5);
        assert_eq!(first, second);
    }

    #[test]
    fn every_value_a_producer_builds_satisfies_the_shared_invariants() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
        write_project_config(&repo, "tdd");
        write(&repo.join("openspec/changes/one/tasks.md"), "- [ ] a\n");
        write(&repo.join("openspec/changes/two/tasks.md"), "- [x] a\n");
        write(
            &repo.join("openspec/changes/archive/2026-01-01-three/tasks.md"),
            "- [x] a\n",
        );
        write(
            &repo.join("openspec/changes/archive/2026-01-02-four/tasks.md"),
            "- [ ] a\n",
        );
        write(
            &repo.join("openspec/changes/archive/2026-01-03-five/tasks.md"),
            "- [x] a\n",
        );

        let set = from_files(&repo, 5);
        assert_eq!(set.active.len(), 2);
        assert_eq!(set.archived.len(), 3);
        for change in set.active.iter().chain(set.archived.iter()) {
            assert_invariants(change);
        }
    }

    #[test]
    fn an_archived_change_keeps_its_file_derived_values_when_the_cli_arrives() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
        write_project_config(&repo, "tdd");
        write(
            &repo.join("openspec/changes/add-auth/tasks.md"),
            "- [ ] a\n",
        );
        write(
            &repo.join("openspec/changes/archive/2026-08-14-add-auth/tasks.md"),
            "- [x] a\n",
        );

        let set = from_files(&repo, 5);
        assert_eq!(set.active.len(), 1);
        assert_eq!(set.archived.len(), 1);
        assert_eq!(set.active[0].name, "add-auth");
        assert_eq!(set.archived[0].name, "add-auth");
        assert_ne!(set.active[0].dir, set.archived[0].dir);
    }

    #[cfg(unix)]
    #[test]
    fn a_repository_level_failure_is_recorded_on_the_set_not_on_a_change() {
        use std::os::unix::fs::PermissionsExt;

        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());

        let (paths, problem) = resolve_artifact(&repo, "notes.md");
        assert!(paths.is_empty());
        assert_eq!(problem, None);

        let archive_dir = repo.join("openspec/changes/archive");
        mkdir(&archive_dir);

        std::fs::set_permissions(&archive_dir, std::fs::Permissions::from_mode(0o000))
            .expect("set archive/ unreadable");
        let set = from_files(&repo, 5);
        std::fs::set_permissions(&archive_dir, std::fs::Permissions::from_mode(0o755))
            .expect("restore archive/ permissions");

        assert!(set.active.is_empty());
        assert!(set.archived.is_empty());
        assert_eq!(set.problems.len(), 1);
        assert!(set.problems[0].contains(&archive_dir.display().to_string()));
    }

    #[test]
    fn every_degradation_lands_on_a_problems_list_rather_than_in_a_return_type() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
        write_project_config(&repo, "tdd");
        write(
            &repo.join("openspec/changes/unvendored/.openspec.yaml"),
            "schema: not-vendored\n",
        );
        mkdir(&repo.join("openspec/changes/directory-tasks/tasks.md"));

        let set = from_files(&repo, 5);
        assert_eq!(set.active.len(), 2);

        let unvendored = set
            .active
            .iter()
            .find(|c| c.name == "unvendored")
            .expect("unvendored change present");
        assert_eq!(unvendored.problems.len(), 1);
        assert!(unvendored.problems[0].contains("not-vendored"));

        let directory_tasks = set
            .active
            .iter()
            .find(|c| c.name == "directory-tasks")
            .expect("directory-tasks change present");
        assert_eq!(directory_tasks.problems.len(), 1);
        assert!(
            directory_tasks.problems[0].contains(
                &repo
                    .join("openspec/changes/directory-tasks/tasks.md")
                    .display()
                    .to_string()
            )
        );
    }

    #[test]
    fn the_whole_repository_is_byte_identical_after_from_files() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
        write_project_config(&repo, "tdd");
        write(&repo.join("openspec/changes/one/proposal.md"), "# P\n");

        let before = snapshot(&repo);
        let _ = from_files(&repo, 5);
        let _ = from_files(&repo, 5);
        let _ = from_files(&repo, 5);
        let after = snapshot(&repo);
        assert_eq!(before, after);
        assert!(!repo.join("openspec/changes/archive").exists());
        assert!(!repo.join("openspec/changes/one/tasks.md").exists());
    }

    #[test]
    fn a_changes_own_declaration_wins_over_the_projects_in_from_files() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
        vendor_schema(
            &repo,
            "probe",
            &[("notes", "notes.md"), ("plan", "plan.md")],
        );
        write_project_config(&repo, "tdd");
        write(&repo.join("openspec/changes/a/tasks.md"), "- [ ] a\n");
        write(
            &repo.join("openspec/changes/b/.openspec.yaml"),
            "schema: probe\n",
        );
        write(&repo.join("openspec/changes/b/tasks.md"), "- [ ] a\n");

        let set = from_files(&repo, 5);
        let a = set.active.iter().find(|c| c.name == "a").unwrap();
        let b = set.active.iter().find(|c| c.name == "b").unwrap();
        assert_eq!(a.schema, "tdd");
        assert_eq!(b.schema, "probe");
        let a_ids: Vec<&str> = a.artifacts.iter().map(|r| r.id.as_str()).collect();
        let b_ids: Vec<&str> = b.artifacts.iter().map(|r| r.id.as_str()).collect();
        assert_ne!(a_ids, b_ids);
    }

    #[test]
    fn a_repository_declaring_no_schema_falls_back_to_the_default() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
        write(&repo.join("openspec/changes/a/tasks.md"), "- [ ] a\n");

        let set = from_files(&repo, 5);
        let change = &set.active[0];
        assert_eq!(change.schema, crate::schema::DEFAULT_SCHEMA);
        assert!(change.artifacts.is_empty());
        assert_eq!(change.problems.len(), 1);
    }

    #[test]
    fn an_unsupported_glob_shape_records_one_problem_while_siblings_still_resolve() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        vendor_schema(
            &repo,
            "probe",
            &[
                ("specs", "specs/*/spec.md"),
                ("proposal", "proposal.md"),
                ("tasks", "tasks.md"),
            ],
        );
        write_project_config(&repo, "probe");
        write(&repo.join("openspec/changes/a/proposal.md"), "# P\n");
        write(&repo.join("openspec/changes/a/specs/zeta/spec.md"), "# Z\n");

        let set = from_files(&repo, 5);
        let change = &set.active[0];
        assert_eq!(change.problems.len(), 1);
        assert!(change.problems[0].contains("specs"));
        let proposal = change
            .artifacts
            .iter()
            .find(|r| r.id == "proposal")
            .unwrap();
        assert_eq!(proposal.paths.len(), 1);
    }

    #[test]
    fn a_wrong_typed_tracks_counts_tasks_md_the_same_pair_the_cli_reports() {
        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        write(
            &repo.join("openspec/schemas/tdd/schema.yaml"),
            "\
name: tdd
artifacts:
  - id: tasks
    generates: tasks/**/*.md
apply:
  tracks: 42
",
        );
        write_project_config(&repo, "tdd");
        write(
            &repo.join("openspec/changes/add-auth/tasks/a.md"),
            "- [x] a\n- [x] b\n",
        );
        write(
            &repo.join("openspec/changes/add-auth/tasks/b.md"),
            "- [ ] a\n- [ ] b\n",
        );

        let set = from_files(&repo, 5);
        assert_eq!(set.active.len(), 1);
        let change = &set.active[0];
        assert_eq!(
            change.progress,
            crate::tasks::Progress {
                completed: 0,
                total: 0,
            }
        );
        assert_ne!(
            change.progress,
            crate::tasks::Progress {
                completed: 2,
                total: 4,
            }
        );
        assert!(change.artifacts.iter().all(|a| !a.tracks_tasks));
    }

    // --- group 2: `parse_list` — the list envelope (`mod list_json`) -------

    mod list_json {
        use super::*;

        #[test]
        fn a_bare_array_is_rejected_rather_than_parsed() {
            let text = r#"[{"name":"alpha","completedTasks":1,"totalTasks":2,"lastModified":"x","status":"y"}]"#;
            assert!(parse_list(text).is_err());
        }

        #[test]
        fn an_empty_change_list_is_a_supported_empty_state() {
            let text = r#"{"changes":[],"root":{"path":"/repo","source":"nearest"}}"#;
            let payload = parse_list(text).expect("should parse");
            assert!(payload.changes.is_empty());
            assert!(payload.problems.is_empty());
            assert_eq!(payload.root, Some(PathBuf::from("/repo")));
        }

        #[test]
        fn a_change_entry_missing_a_required_field_is_skipped_not_fatal() {
            let text = r#"{"changes":[
                {"name":"alpha","completedTasks":1,"totalTasks":2,"lastModified":"x","status":"y"},
                {"name":"mike","totalTasks":3,"lastModified":"x","status":"y"},
                {"name":42,"completedTasks":0,"totalTasks":0,"lastModified":"x","status":"y"}
            ]}"#;
            let payload = parse_list(text).expect("should parse");
            assert_eq!(payload.changes.len(), 1);
            assert_eq!(payload.changes[0].name, "alpha");
            assert_eq!(payload.problems.len(), 2);
            assert!(payload.problems[0].contains('1'), "{}", payload.problems[0]);
            assert!(payload.problems[1].contains('2'), "{}", payload.problems[1]);
        }

        #[test]
        fn an_entry_whose_name_is_the_empty_string_is_skipped() {
            let text = r#"{"changes":[
                {"name":"","completedTasks":0,"totalTasks":0,"lastModified":"x","status":"y"},
                {"name":"alpha","completedTasks":1,"totalTasks":2,"lastModified":"x","status":"y"}
            ]}"#;
            let payload = parse_list(text).expect("should parse");
            assert_eq!(payload.changes.len(), 1);
            assert_eq!(payload.changes[0].name, "alpha");
            assert_eq!(payload.problems.len(), 1);
            assert!(payload.problems[0].contains('0'), "{}", payload.problems[0]);
        }

        #[test]
        fn a_repeated_name_keeps_the_first_entry_and_names_the_duplicate() {
            let text = r#"{"changes":[
                {"name":"alpha","completedTasks":1,"totalTasks":2,"lastModified":"x","status":"y"},
                {"name":"mike","completedTasks":0,"totalTasks":0,"lastModified":"x","status":"y"},
                {"name":"alpha","completedTasks":9,"totalTasks":9,"lastModified":"x","status":"y"}
            ]}"#;
            let payload = parse_list(text).expect("should parse");
            assert_eq!(payload.changes.len(), 2);
            let alpha = payload.changes.iter().find(|c| c.name == "alpha").unwrap();
            assert_eq!(
                alpha.progress,
                crate::tasks::Progress {
                    completed: 1,
                    total: 2
                }
            );
            assert!(payload.changes.iter().any(|c| c.name == "mike"));
            assert_eq!(payload.problems.len(), 1);
            assert!(
                payload.problems[0].contains("alpha"),
                "{}",
                payload.problems[0]
            );
        }

        #[test]
        fn empty_stdout_is_a_parse_failure() {
            assert!(parse_list("").is_err());
        }

        #[test]
        fn a_list_envelope_carries_the_root_path() {
            let text = r#"{"changes":[],"root":{"path":"/repo/root","source":"nearest"}}"#;
            let payload = parse_list(text).expect("should parse");
            assert_eq!(payload.root, Some(PathBuf::from("/repo/root")));
        }

        #[test]
        fn an_envelope_with_no_root_yields_no_root() {
            let text = r#"{"changes":[]}"#;
            let payload = parse_list(text).expect("should parse");
            assert_eq!(payload.root, None);
        }

        #[test]
        fn a_root_whose_path_is_not_a_string_yields_no_root() {
            let text = r#"{"changes":[],"root":{"path":7,"source":"nearest"}}"#;
            let payload = parse_list(text).expect("should parse");
            assert_eq!(payload.root, None);
        }

        #[test]
        fn list_entries_keep_the_payload_order() {
            let text = r#"{"changes":[
                {"name":"zulu","completedTasks":0,"totalTasks":0,"lastModified":"x","status":"y"},
                {"name":"mike","completedTasks":0,"totalTasks":0,"lastModified":"x","status":"y"},
                {"name":"alpha","completedTasks":0,"totalTasks":0,"lastModified":"x","status":"y"}
            ]}"#;
            let payload = parse_list(text).expect("should parse");
            let names: Vec<&str> = payload.changes.iter().map(|c| c.name.as_str()).collect();
            assert_eq!(names, vec!["zulu", "mike", "alpha"]);
        }
    }

    // --- group 3: `parse_apply` — the apply payload (`mod apply_json`) -----

    mod apply_json {
        use super::*;

        #[test]
        fn an_apply_payload_yields_schema_name_change_dir_and_context_files() {
            let text = r#"{
                "schemaName":"tdd",
                "changeDir":"/repo/openspec/changes/x",
                "contextFiles":{"proposal":["/repo/openspec/changes/x/proposal.md"],"specs":[]}
            }"#;
            let payload = parse_apply(text).expect("should parse");
            assert_eq!(payload.schema_name, "tdd");
            assert_eq!(
                payload.change_dir,
                PathBuf::from("/repo/openspec/changes/x")
            );
            assert_eq!(
                payload.context_files.get("proposal"),
                Some(&vec![PathBuf::from("/repo/openspec/changes/x/proposal.md")])
            );
            assert_eq!(payload.context_files.get("specs"), Some(&vec![]));
        }

        #[test]
        fn an_apply_payload_missing_context_files_is_an_error() {
            let text = r#"{"schemaName":"tdd","changeDir":"/repo/openspec/changes/x"}"#;
            assert!(parse_apply(text).is_err());
        }

        #[test]
        fn an_apply_payload_missing_schema_name_is_an_error() {
            let text = r#"{"changeDir":"/repo/openspec/changes/x","contextFiles":{}}"#;
            assert!(parse_apply(text).is_err());
        }

        #[test]
        fn an_apply_payload_missing_change_dir_is_an_error() {
            let text = r#"{"schemaName":"tdd","contextFiles":{}}"#;
            assert!(parse_apply(text).is_err());
        }

        #[test]
        fn an_error_envelope_is_an_error() {
            let text = r#"{"status":[{"severity":"error","code":"unknown_schema","message":"Unknown schema \"outside-in-tdd\""}]}"#;
            assert!(parse_apply(text).is_err());
        }

        #[test]
        fn context_files_values_that_are_not_string_arrays_are_rejected() {
            let text = r#"{
                "schemaName":"tdd",
                "changeDir":"/repo/openspec/changes/x",
                "contextFiles":{"proposal":[1,2]}
            }"#;
            assert!(parse_apply(text).is_err());

            let text = r#"{
                "schemaName":"tdd",
                "changeDir":"/repo/openspec/changes/x",
                "contextFiles":{"proposal":"not-an-array"}
            }"#;
            assert!(parse_apply(text).is_err());
        }

        #[test]
        fn an_empty_context_files_object_is_valid_and_yields_no_paths() {
            let text = r#"{
                "schemaName":"tdd",
                "changeDir":"/repo/openspec/changes/x",
                "contextFiles":{}
            }"#;
            let payload = parse_apply(text).expect("should parse");
            assert!(payload.context_files.is_empty());
        }

        #[test]
        fn malformed_apply_json_is_an_error() {
            assert!(parse_apply("{ this is not json").is_err());
        }
    }

    // --- group 4: `parse_schema_which` — the schema-directory payload ------
    // (`mod which_json`)

    mod which_json {
        use super::*;

        #[test]
        fn a_schema_which_payload_yields_the_directory_path() {
            let text = r#"{"name":"spec-driven","source":"package","path":"/pkg/spec-driven","shadows":[]}"#;
            assert_eq!(
                parse_schema_which(text),
                Ok(PathBuf::from("/pkg/spec-driven"))
            );
        }

        #[test]
        fn a_leading_non_json_line_is_not_tolerated() {
            let text = "Note: Schema commands are experimental and may change.\n{\"path\":\"/x\"}";
            assert!(parse_schema_which(text).is_err());
        }

        #[test]
        fn a_schema_which_error_body_is_an_error() {
            let text = r#"{"error":"Unknown schema \"outside-in-tdd\"","available":["tdd","spec-driven"]}"#;
            assert!(parse_schema_which(text).is_err());
        }

        #[test]
        fn an_empty_payload_is_an_error() {
            assert!(parse_schema_which("").is_err());
        }

        #[test]
        fn a_null_payload_is_an_error() {
            assert!(parse_schema_which("null").is_err());
        }

        #[test]
        fn a_non_string_path_is_an_error() {
            assert!(parse_schema_which(r#"{"path": 7}"#).is_err());
        }

        #[test]
        fn a_payload_with_no_path_is_an_error() {
            assert!(parse_schema_which(r#"{"name":"x"}"#).is_err());
        }

        #[test]
        fn an_empty_path_is_an_error() {
            assert!(parse_schema_which(r#"{"path":""}"#).is_err());
        }

        #[test]
        fn source_and_shadows_are_ignored() {
            let text = r#"{"name":"spec-driven","source":"package","path":"/pkg/spec-driven","shadows":["/user/spec-driven"]}"#;
            assert_eq!(
                parse_schema_which(text),
                Ok(PathBuf::from("/pkg/spec-driven"))
            );
        }
    }

    // --- group 5: `cli_artifacts` — placing paths at schema positions ------
    // (`mod cli_artifacts`)

    mod cli_artifacts {
        // Explicit import first: the module and the function under test
        // share a name, and an explicit `use` shadows the same name a glob
        // import would otherwise bind to the enclosing `mod cli_artifacts`
        // declaration itself.
        use super::super::cli_artifacts;
        use super::*;
        use std::collections::BTreeMap;

        fn tdd_schema() -> crate::schema::Schema {
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
        fn an_omitted_context_files_key_becomes_an_empty_path_list_at_its_position() {
            let mut context_files = BTreeMap::new();
            context_files.insert(
                "proposal".to_string(),
                vec![PathBuf::from("/repo/x/proposal.md")],
            );
            context_files.insert(
                "specs".to_string(),
                vec![PathBuf::from("/repo/x/specs/a.md")],
            );
            context_files.insert("tasks".to_string(), vec![PathBuf::from("/repo/x/tasks.md")]);

            let (artifacts, problems) = cli_artifacts(&tdd_schema(), &context_files);
            let ids: Vec<&str> = artifacts.iter().map(|a| a.id.as_str()).collect();
            assert_eq!(
                ids,
                vec!["proposal", "specs", "design", "tasks", "planning-review"]
            );
            let design = artifacts.iter().find(|a| a.id == "design").unwrap();
            assert!(design.paths.is_empty());
            let planning_review = artifacts
                .iter()
                .find(|a| a.id == "planning-review")
                .unwrap();
            assert!(planning_review.paths.is_empty());
            assert!(problems.is_empty());
        }

        #[test]
        fn a_multi_file_artifact_keeps_the_cli_list_in_the_cli_order() {
            let mut context_files = BTreeMap::new();
            context_files.insert(
                "specs".to_string(),
                vec![
                    PathBuf::from("/repo/x/specs/cap-one/spec.md"),
                    PathBuf::from("/repo/x/specs/cap-two/spec.md"),
                ],
            );
            let (artifacts, _problems) = cli_artifacts(&tdd_schema(), &context_files);
            let specs = artifacts.iter().find(|a| a.id == "specs").unwrap();
            assert_eq!(
                specs.paths,
                vec![
                    PathBuf::from("/repo/x/specs/cap-one/spec.md"),
                    PathBuf::from("/repo/x/specs/cap-two/spec.md"),
                ]
            );
        }

        #[test]
        fn a_context_files_key_naming_no_schema_artifact_is_ignored() {
            let mut context_files = BTreeMap::new();
            context_files.insert(
                "legacy".to_string(),
                vec![PathBuf::from("/repo/x/legacy.md")],
            );
            let (artifacts, problems) = cli_artifacts(&tdd_schema(), &context_files);
            assert!(!artifacts.iter().any(|a| a.id == "legacy"));
            let ids: Vec<&str> = artifacts.iter().map(|a| a.id.as_str()).collect();
            assert_eq!(
                ids,
                vec!["proposal", "specs", "design", "tasks", "planning-review"]
            );
            assert_eq!(problems.len(), 1);
            assert!(problems[0].contains("legacy"));
        }

        #[test]
        fn a_duplicate_schema_id_gives_both_positions_the_same_paths() {
            let schema = crate::schema::Schema {
                name: "dup".to_string(),
                artifacts: vec![
                    crate::schema::Artifact {
                        id: "zeta".to_string(),
                        generates: "zeta.md".to_string(),
                    },
                    crate::schema::Artifact {
                        id: "alpha".to_string(),
                        generates: "alpha.md".to_string(),
                    },
                    crate::schema::Artifact {
                        id: "zeta".to_string(),
                        generates: "zeta2.md".to_string(),
                    },
                ],
                tasks: None,
            };
            let mut context_files = BTreeMap::new();
            context_files.insert("zeta".to_string(), vec![PathBuf::from("/repo/x/zeta.md")]);

            let (artifacts, _problems) = cli_artifacts(&schema, &context_files);
            assert_eq!(artifacts.len(), 3);
            assert_eq!(artifacts[0].id, "zeta");
            assert_eq!(artifacts[0].paths, vec![PathBuf::from("/repo/x/zeta.md")]);
            assert_eq!(artifacts[1].id, "alpha");
            assert!(artifacts[1].paths.is_empty());
            assert_eq!(artifacts[2].id, "zeta");
            assert_eq!(artifacts[2].paths, vec![PathBuf::from("/repo/x/zeta.md")]);
        }

        #[test]
        fn an_empty_context_files_map_yields_every_artifact_with_no_paths() {
            let context_files = BTreeMap::new();
            let (artifacts, problems) = cli_artifacts(&tdd_schema(), &context_files);
            assert_eq!(artifacts.len(), 5);
            assert!(artifacts.iter().all(|a| a.paths.is_empty()));
            assert!(problems.is_empty());
        }

        #[test]
        fn artifact_order_is_the_schemas_declared_order_not_the_map_order() {
            // The map's own key order (alpha < design < proposal, ...) differs
            // from the schema's declared order, so an implementation that
            // iterated the map would produce the wrong order here.
            let mut context_files = BTreeMap::new();
            context_files.insert("alpha".to_string(), vec![]);
            context_files.insert("design".to_string(), vec![]);
            context_files.insert("proposal".to_string(), vec![]);
            let (artifacts, _problems) = cli_artifacts(&tdd_schema(), &context_files);
            let ids: Vec<&str> = artifacts.iter().map(|a| a.id.as_str()).collect();
            assert_eq!(
                ids,
                vec!["proposal", "specs", "design", "tasks", "planning-review"]
            );
        }

        // --- tasks-tab group 3: the tracked-tasks flag, CLI producer -----

        fn tdd_schema_with_tasks() -> crate::schema::Schema {
            let mut schema = tdd_schema();
            schema.tasks = Some(crate::schema::Artifact {
                id: "tasks".to_string(),
                generates: "tasks.md".to_string(),
            });
            schema
        }

        #[test]
        fn cli_tracks_tasks_tdd() {
            let mut context_files = BTreeMap::new();
            context_files.insert(
                "proposal".to_string(),
                vec![PathBuf::from("/repo/x/proposal.md")],
            );
            context_files.insert("tasks".to_string(), vec![PathBuf::from("/repo/x/tasks.md")]);

            let (artifacts, _problems) = cli_artifacts(&tdd_schema_with_tasks(), &context_files);
            let flags: Vec<bool> = artifacts.iter().map(|a| a.tracks_tasks).collect();
            assert_eq!(flags, vec![false, false, false, true, false]);
            assert_eq!(artifacts[3].paths, vec![PathBuf::from("/repo/x/tasks.md")]);
        }

        #[test]
        fn both_producers_mark_the_same_position() {
            use crate::testutil::{ScratchDir, canonical};

            fn write(path: &std::path::Path, contents: &str) {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).expect("create fixture parent");
                }
                std::fs::write(path, contents).expect("write fixture file");
            }

            let cases: Vec<crate::schema::Schema> = vec![
                // The `tdd` schema.
                tdd_schema_with_tasks(),
                // `apply.tracks` names an artifact whose id is not `tasks`.
                crate::schema::Schema {
                    name: "custom".to_string(),
                    artifacts: vec![
                        crate::schema::Artifact {
                            id: "checklist".to_string(),
                            generates: "tasks.md".to_string(),
                        },
                        crate::schema::Artifact {
                            id: "notes".to_string(),
                            generates: "notes.md".to_string(),
                        },
                    ],
                    tasks: Some(crate::schema::Artifact {
                        id: "checklist".to_string(),
                        generates: "tasks.md".to_string(),
                    }),
                },
                // No `apply` block; an artifact with id `tasks` is present.
                crate::schema::Schema {
                    name: "fallback".to_string(),
                    artifacts: vec![
                        crate::schema::Artifact {
                            id: "proposal".to_string(),
                            generates: "proposal.md".to_string(),
                        },
                        crate::schema::Artifact {
                            id: "tasks".to_string(),
                            generates: "tasks.md".to_string(),
                        },
                    ],
                    tasks: Some(crate::schema::Artifact {
                        id: "tasks".to_string(),
                        generates: "tasks.md".to_string(),
                    }),
                },
                // A schema naming no tasks artifact at all.
                crate::schema::Schema {
                    name: "none".to_string(),
                    artifacts: vec![
                        crate::schema::Artifact {
                            id: "alpha".to_string(),
                            generates: "alpha.md".to_string(),
                        },
                        crate::schema::Artifact {
                            id: "beta".to_string(),
                            generates: "beta.md".to_string(),
                        },
                    ],
                    tasks: None,
                },
            ];

            for schema in cases {
                let scratch = ScratchDir::new();
                let dir = canonical(scratch.path());
                for artifact in &schema.artifacts {
                    write(&dir.join(&artifact.generates), "- [x] a\n");
                }
                let (file_artifacts, _) = super::super::change_artifacts(&dir, &schema);

                let context_files: std::collections::BTreeMap<String, Vec<PathBuf>> = schema
                    .artifacts
                    .iter()
                    .map(|a| (a.id.clone(), vec![dir.join(&a.generates)]))
                    .collect();
                let (cli_artifacts_list, _) = cli_artifacts(&schema, &context_files);

                let file_flags: Vec<bool> = file_artifacts.iter().map(|a| a.tracks_tasks).collect();
                let cli_flags: Vec<bool> =
                    cli_artifacts_list.iter().map(|a| a.tracks_tasks).collect();
                assert_eq!(
                    file_flags, cli_flags,
                    "schema {:?}: producers disagree on tracks_tasks",
                    schema.name
                );
                // Red-when guard: a naive "always false" implementation on
                // both sides would trivially pass an equality-only check,
                // so also assert the expected `true` position by name.
                if let Some(expected_true) = schema
                    .tasks
                    .as_ref()
                    .and_then(|tasks| schema.artifacts.iter().position(|a| a == tasks))
                {
                    assert!(
                        file_flags[expected_true],
                        "schema {:?}: expected position {expected_true} marked",
                        schema.name
                    );
                    assert!(cli_flags[expected_true]);
                } else {
                    assert!(file_flags.iter().all(|&f| !f));
                    assert!(cli_flags.iter().all(|&f| !f));
                }
            }
        }

        #[test]
        fn cli_only_schema_marks() {
            let schema = crate::schema::Schema {
                name: "cli-only".to_string(),
                artifacts: vec![
                    crate::schema::Artifact {
                        id: "alpha".to_string(),
                        generates: "alpha.md".to_string(),
                    },
                    crate::schema::Artifact {
                        id: "tasks".to_string(),
                        generates: "tasks.md".to_string(),
                    },
                ],
                tasks: Some(crate::schema::Artifact {
                    id: "tasks".to_string(),
                    generates: "tasks.md".to_string(),
                }),
            };
            let context_files = BTreeMap::new();

            let (artifacts, _problems) = cli_artifacts(&schema, &context_files);
            assert_eq!(artifacts.len(), 2);
            assert!(!artifacts[0].tracks_tasks);
            assert!(artifacts[1].tracks_tasks);
        }

        #[test]
        fn cli_duplicate_first_only() {
            let schema = crate::schema::Schema {
                name: "dup".to_string(),
                artifacts: vec![
                    crate::schema::Artifact {
                        id: "zeta".to_string(),
                        generates: "tasks.md".to_string(),
                    },
                    crate::schema::Artifact {
                        id: "alpha".to_string(),
                        generates: "alpha.md".to_string(),
                    },
                    crate::schema::Artifact {
                        id: "zeta".to_string(),
                        generates: "tasks.md".to_string(),
                    },
                ],
                tasks: Some(crate::schema::Artifact {
                    id: "zeta".to_string(),
                    generates: "tasks.md".to_string(),
                }),
            };
            let mut context_files = BTreeMap::new();
            context_files.insert("zeta".to_string(), vec![PathBuf::from("/repo/x/tasks.md")]);

            let (artifacts, _problems) = cli_artifacts(&schema, &context_files);
            assert_eq!(artifacts.len(), 3);
            assert!(artifacts[0].tracks_tasks);
            assert!(!artifacts[1].tracks_tasks);
            assert!(!artifacts[2].tracks_tasks);
            assert_eq!(artifacts[0].paths, vec![PathBuf::from("/repo/x/tasks.md")]);
            assert_eq!(artifacts[2].paths, vec![PathBuf::from("/repo/x/tasks.md")]);
        }
    }

    // --- group 6: `join_artifacts` — the positional cross-producer join ----
    // (`mod join_artifacts`)

    mod join_artifacts {
        // See `mod cli_artifacts` above for why the function is imported
        // explicitly ahead of the glob: the module and the function share a
        // name.
        use super::super::join_artifacts;
        use super::*;

        fn r(id: &str, paths: Vec<&str>) -> ArtifactRef {
            ArtifactRef {
                id: id.to_string(),
                paths: paths.into_iter().map(PathBuf::from).collect(),
                tracks_tasks: false,
            }
        }

        /// Like `r`, with an explicit `tracks_tasks` — the tasks-tab join
        /// tests need at least one marked entry, which the all-`false` `r`
        /// helper cannot produce.
        fn rt(id: &str, paths: Vec<&str>, tracks_tasks: bool) -> ArtifactRef {
            ArtifactRef {
                id: id.to_string(),
                paths: paths.into_iter().map(PathBuf::from).collect(),
                tracks_tasks,
            }
        }

        #[test]
        fn equal_length_lists_with_equal_ids_take_the_cli_paths_positionally() {
            let file = vec![
                r("proposal", vec!["/repo/a/proposal.md"]),
                r("specs", vec![]),
                r("design", vec![]),
            ];
            let cli = vec![
                r("proposal", vec!["/private/var/.../a/proposal.md"]),
                r(
                    "specs",
                    vec![
                        "/private/var/.../a/specs/one.md",
                        "/private/var/.../a/specs/two.md",
                    ],
                ),
                r("design", vec![]),
            ];
            let (joined, problem) = join_artifacts(&file, &cli);
            assert_eq!(problem, None);
            let ids: Vec<&str> = joined.iter().map(|a| a.id.as_str()).collect();
            assert_eq!(ids, vec!["proposal", "specs", "design"]);
            assert_eq!(joined[0].paths, cli[0].paths);
            assert_eq!(joined[1].paths, cli[1].paths);
            assert_eq!(joined[1].paths.len(), 2);
        }

        #[test]
        fn a_duplicate_id_is_joined_by_index_rather_than_collapsed() {
            let file = vec![
                r("zeta", vec!["/f/zeta-0"]),
                r("alpha", vec![]),
                r("zeta", vec!["/f/zeta-2"]),
            ];
            let cli = vec![
                r("zeta", vec![]),
                r("alpha", vec![]),
                r("zeta", vec!["/c/zeta-2"]),
            ];
            let (joined, problem) = join_artifacts(&file, &cli);
            assert_eq!(problem, None);
            assert_eq!(joined.len(), 3);
            assert_eq!(joined[0].id, "zeta");
            assert_eq!(joined[2].id, "zeta");
            // Index 2 keeps its own CLI path, not index 0's — an id-keyed
            // join would give both positions the same (first-matching)
            // entry and fail this assertion.
            assert_eq!(joined[2].paths, vec![PathBuf::from("/c/zeta-2")]);
            assert!(joined[0].paths.is_empty());
        }

        #[test]
        fn an_empty_file_list_takes_the_cli_list() {
            let cli = vec![
                r("a", vec![]),
                r("b", vec![]),
                r("c", vec![]),
                r("d", vec![]),
                r("e", vec![]),
            ];
            let (joined, problem) = join_artifacts(&[], &cli);
            assert_eq!(joined, cli);
            assert_eq!(problem, None);
        }

        #[test]
        fn an_empty_cli_list_keeps_the_file_list() {
            let file = vec![
                r("a", vec![]),
                r("b", vec![]),
                r("c", vec![]),
                r("d", vec![]),
                r("e", vec![]),
            ];
            let (joined, problem) = join_artifacts(&file, &[]);
            assert_eq!(joined, file);
            assert_eq!(problem, None);
        }

        #[test]
        fn two_empty_lists_join_to_an_empty_list() {
            let (joined, problem) = join_artifacts(&[], &[]);
            assert!(joined.is_empty());
            assert_eq!(problem, None);
        }

        #[test]
        fn differing_lengths_keep_the_file_list_and_name_both_counts() {
            let file = vec![
                r("a", vec![]),
                r("b", vec![]),
                r("c", vec![]),
                r("d", vec![]),
                r("e", vec![]),
            ];
            let cli = vec![r("a", vec![]), r("b", vec![]), r("c", vec![])];
            let (joined, problem) = join_artifacts(&file, &cli);
            assert_eq!(joined, file);
            let problem = problem.expect("should record a problem");
            assert!(problem.contains('5'), "{problem}");
            assert!(problem.contains('3'), "{problem}");
        }

        #[test]
        fn a_differing_id_at_one_index_keeps_the_file_list_and_names_the_index() {
            let file = vec![
                r("proposal", vec![]),
                r("specs", vec![]),
                r("design", vec![]),
                r("tasks", vec![]),
            ];
            let cli = vec![
                r("proposal", vec![]),
                r("specs", vec![]),
                r("plan", vec![]),
                r("other", vec![]),
            ];
            let (joined, problem) = join_artifacts(&file, &cli);
            assert_eq!(joined, file);
            let problem = problem.expect("should record a problem");
            assert!(problem.contains('2'), "{problem}");
            assert!(problem.contains("design"), "{problem}");
            assert!(problem.contains("plan"), "{problem}");
            // Only the FIRST differing index is named, even though index 3
            // also differs.
            assert!(!problem.contains("tasks"), "{problem}");
            assert!(!problem.contains("other"), "{problem}");
        }

        #[test]
        fn the_join_never_reads_a_path_as_a_key() {
            let file = vec![
                r("proposal", vec!["/f/proposal.md"]),
                r("specs", vec!["/f/specs.md"]),
            ];
            let cli = vec![
                r("proposal", vec!["/c/totally-different.md"]),
                r("specs", vec!["/c/also-different.md"]),
            ];
            let (joined, problem) = join_artifacts(&file, &cli);
            assert_eq!(problem, None);
            assert_eq!(joined, cli);
        }

        // --- tasks-tab group 3: the tracked-tasks flag rides the join ----

        #[test]
        fn join_takes_cli_flag() {
            let file = vec![
                r("proposal", vec![]),
                r("specs", vec![]),
                rt("tasks", vec!["/f/tasks.md"], true),
            ];
            let cli = vec![
                r("proposal", vec![]),
                r("specs", vec![]),
                rt("tasks", vec!["/c/tasks.md"], true),
            ];
            let (joined, problem) = join_artifacts(&file, &cli);
            assert_eq!(problem, None);
            assert!(!joined[0].tracks_tasks);
            assert!(!joined[1].tracks_tasks);
            assert!(joined[2].tracks_tasks);
            // The CLI's paths, confirming the CLI list is what won.
            assert_eq!(joined[2].paths, vec![PathBuf::from("/c/tasks.md")]);
        }

        #[test]
        fn join_cli_flag_moves_the_tab() {
            let file = vec![
                rt("checklist", vec![], false),
                r("notes", vec![]),
                rt("tasks", vec![], true),
            ];
            let cli = vec![
                rt("checklist", vec![], true),
                r("notes", vec![]),
                rt("tasks", vec![], false),
            ];
            let (joined, problem) = join_artifacts(&file, &cli);
            assert_eq!(problem, None);
            assert!(joined[0].tracks_tasks);
            assert!(!joined[1].tracks_tasks);
            assert!(!joined[2].tracks_tasks);
        }

        #[test]
        fn join_rejected_keeps_file_flag() {
            // Differing lengths: five file entries, three CLI entries.
            let file = vec![
                r("a", vec![]),
                r("b", vec![]),
                r("c", vec![]),
                rt("tasks", vec![], true),
                r("e", vec![]),
            ];
            let cli = vec![r("a", vec![]), r("b", vec![]), r("c", vec![])];
            let (joined, problem) = join_artifacts(&file, &cli);
            assert_eq!(joined, file);
            assert!(joined[3].tracks_tasks);
            let problem = problem.expect("should record a problem");
            assert!(problem.contains('5'), "{problem}");
            assert!(problem.contains('3'), "{problem}");

            // Equal length, disagreeing id at one index: rule 5 keeps the
            // file list and its flag too.
            let file = vec![
                r("proposal", vec![]),
                rt("tasks", vec![], true),
                r("design", vec![]),
            ];
            let cli = vec![
                r("proposal", vec![]),
                r("plan", vec![]),
                r("design", vec![]),
            ];
            let (joined, problem) = join_artifacts(&file, &cli);
            assert_eq!(joined, file);
            assert!(joined[1].tracks_tasks);
            assert!(problem.is_some());
        }

        #[test]
        fn join_never_two_marked() {
            fn assert_at_most_one_marked(
                joined: &[ArtifactRef],
                file: &[ArtifactRef],
                cli: &[ArtifactRef],
            ) {
                assert!(joined.iter().filter(|a| a.tracks_tasks).count() <= 1);
                for a in joined {
                    if a.tracks_tasks {
                        let carried_by_file = file.iter().any(|f| f.id == a.id && f.tracks_tasks);
                        let carried_by_cli = cli.iter().any(|c| c.id == a.id && c.tracks_tasks);
                        assert!(carried_by_file || carried_by_cli);
                    }
                }
            }

            // Rule: both empty.
            let (joined, _) = join_artifacts(&[], &[]);
            assert_at_most_one_marked(&joined, &[], &[]);

            // Rule: empty file list, CLI marks position 1.
            let cli = vec![r("a", vec![]), rt("b", vec![], true)];
            let (joined, _) = join_artifacts(&[], &cli);
            assert_at_most_one_marked(&joined, &[], &cli);

            // Rule: empty CLI list, file marks position 0.
            let file = vec![rt("a", vec![], true), r("b", vec![])];
            let (joined, _) = join_artifacts(&file, &[]);
            assert_at_most_one_marked(&joined, &file, &[]);

            // Rule: differing lengths, file marks position 1.
            let file = vec![rt("a", vec![], false), rt("b", vec![], true)];
            let cli = vec![r("a", vec![])];
            let (joined, _) = join_artifacts(&file, &cli);
            assert_at_most_one_marked(&joined, &file, &cli);

            // Rule: differing id at an index, file marks position 0.
            let file = vec![rt("a", vec![], true), r("b", vec![])];
            let cli = vec![r("a", vec![]), r("different", vec![])];
            let (joined, _) = join_artifacts(&file, &cli);
            assert_at_most_one_marked(&joined, &file, &cli);

            // Rule: equal-shape lists, marked positions DIFFER between the
            // two producers — the CLI's own flag wins, at its own position.
            let file = vec![rt("a", vec![], true), rt("b", vec![], false)];
            let cli = vec![rt("a", vec![], false), rt("b", vec![], true)];
            let (joined, _) = join_artifacts(&file, &cli);
            assert_at_most_one_marked(&joined, &file, &cli);
            assert!(!joined[0].tracks_tasks);
            assert!(joined[1].tracks_tasks);
        }
    }

    // --- group 6a: `cli_error_problem` — every diagnostic the seam carried -
    // (`mod cli_error_problem`)

    mod cli_error_problem {
        // Explicit import first: the module and the function under test
        // share a name, and an explicit `use` shadows the same name a glob
        // import would otherwise bind to the enclosing `mod cli_error_problem`
        // declaration itself.
        use super::super::cli_error_problem;
        use crate::cli::CliError;

        fn failed(code: Option<i32>, stderr: &str) -> CliError {
            CliError::Failed {
                program: "openspec".to_string(),
                args: vec!["list".to_string(), "--json".to_string()],
                code,
                stderr: stderr.to_string(),
            }
        }

        #[test]
        fn an_exec_failure_of_the_openspec_shim_reports_its_own_stderr() {
            // Measured on this machine: `env -i PATH=/usr/bin:/bin
            // "$(readlink -f ~/.nvm/versions/node/v24.18.0/bin/openspec)"
            // list --json` -> exit=127, stdout=[], stderr=[env: node: No
            // such file or directory].
            let err = failed(Some(127), "env: node: No such file or directory\n");
            let problem = cli_error_problem("openspec list --json", &["list", "--json"], &err);
            assert!(problem.contains("127"));
            assert!(problem.contains("list"));
            assert!(problem.contains("env: node: No such file or directory"));
            assert!(!problem.contains("env: node: No such file or directory\n"));
        }

        #[test]
        fn a_multi_line_stderr_contributes_only_its_first_non_blank_line_trimmed() {
            let err = failed(Some(1), "\n\n  first line  \nsecond line\nthird line");
            let problem = cli_error_problem("openspec list --json", &["list", "--json"], &err);
            assert_eq!(
                problem,
                "openspec list --json: openspec list --json exited with code 1: first line"
            );
        }

        #[test]
        fn a_note_banner_is_skipped_and_the_next_line_carried() {
            // Measured: `openspec schema which nosuchschema --json` from
            // `/tmp` -> exit=1, the real answer on stdout, stderr=[Note:
            // Schema commands are experimental and may change.].
            let err = failed(
                Some(1),
                "Note: Schema commands are experimental and may change.\nreal diagnosis here",
            );
            let problem = cli_error_problem("openspec list --json", &["list", "--json"], &err);
            assert!(problem.ends_with("real diagnosis here"));
            assert!(!problem.contains("Note:"));
            assert!(!problem.contains("experimental"));

            let banner_only = failed(
                Some(1),
                "Note: Schema commands are experimental and may change.",
            );
            let banner_only_problem =
                cli_error_problem("openspec list --json", &["list", "--json"], &banner_only);
            let empty = failed(Some(1), "");
            let empty_problem =
                cli_error_problem("openspec list --json", &["list", "--json"], &empty);
            assert_eq!(banner_only_problem, empty_problem);

            let padded = failed(Some(1), "   Note: padded banner\nkept");
            let padded_problem =
                cli_error_problem("openspec list --json", &["list", "--json"], &padded);
            assert!(padded_problem.ends_with("kept"));
        }

        #[test]
        fn a_whitespace_only_stderr_appends_nothing() {
            let err = failed(Some(1), "   \n\t\n");
            let problem = cli_error_problem("openspec list --json", &["list", "--json"], &err);
            let empty = failed(Some(1), "");
            let empty_problem =
                cli_error_problem("openspec list --json", &["list", "--json"], &empty);
            assert_eq!(problem, empty_problem);
        }
    }

    // --- group 7: schema resolution through the CLI fallback tier ----------
    // (`mod schema_fallback`)

    mod schema_fallback {
        use super::*;
        use crate::cli::{CliError, FakeCli, OpenspecCli};
        use std::collections::HashMap;

        /// A `schema.yaml` directly under `dir` — unlike `vendor_schema`,
        /// which nests it under `<repo>/openspec/schemas/<name>/`, this is
        /// the shape of the arbitrary directory `openspec schema which`
        /// names: a CLI-package or `$XDG_DATA_HOME` tier the file producer
        /// never reads.
        fn write_schema_yaml(dir: &std::path::Path, name: &str, artifacts: &[(&str, &str)]) {
            let mut yaml = format!("name: {name}\nartifacts:\n");
            for (id, generates) in artifacts {
                yaml.push_str(&format!("  - id: {id}\n    generates: {generates}\n"));
            }
            write(&dir.join("schema.yaml"), &yaml);
        }

        fn which_response(dir: &std::path::Path, name: &str) -> Result<String, CliError> {
            Ok(format!(
                r#"{{"name":{name:?},"source":"package","path":{:?},"shadows":[]}}"#,
                dir.display()
            ))
        }

        #[test]
        fn a_schema_absent_from_the_repository_is_loaded_from_the_directory_the_cli_names() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            let pkg_dir = repo.join("pkg/spec-driven");
            write_schema_yaml(
                &pkg_dir,
                "spec-driven",
                &[("proposal", "proposal.md"), ("tasks", "tasks.md")],
            );

            let fake = FakeCli::new();
            fake.register_openspec(
                &["schema", "which", "spec-driven", "--json"],
                which_response(&pkg_dir, "spec-driven"),
            );

            let mut cache = HashMap::new();
            let (schema, problems) = resolve_cli_schema(&fake, &repo, "spec-driven", &mut cache);
            let schema = schema.expect("should resolve");
            let ids: Vec<&str> = schema.artifacts.iter().map(|a| a.id.as_str()).collect();
            assert_eq!(ids, vec!["proposal", "tasks"]);
            assert!(problems.is_empty());
        }

        #[test]
        fn a_vendored_schema_never_reaches_the_cli() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);

            let fake = FakeCli::new();
            // No "schema which" registration — the fake panics on an
            // unregistered pair, so a passing test proves the call was
            // never made.
            let mut cache = HashMap::new();
            let (schema, problems) = resolve_cli_schema(&fake, &repo, "tdd", &mut cache);
            assert!(schema.is_some());
            assert!(problems.is_empty());
        }

        #[test]
        fn an_unreadable_vendored_schema_is_not_repaired_by_the_cli() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            mkdir(&repo.join("openspec/schemas/odd/schema.yaml"));

            let fake = FakeCli::new();
            let mut cache = HashMap::new();
            let (schema, problems) = resolve_cli_schema(&fake, &repo, "odd", &mut cache);
            assert!(schema.is_none());
            assert_eq!(problems.len(), 1);
            assert!(problems[0].contains("odd") || problems[0].contains("schema.yaml"));
        }

        #[test]
        fn an_invalid_vendored_schema_is_not_repaired_by_the_cli() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            write(
                &repo.join("openspec/schemas/bad/schema.yaml"),
                "not: [valid",
            );

            let fake = FakeCli::new();
            let mut cache = HashMap::new();
            let (schema, problems) = resolve_cli_schema(&fake, &repo, "bad", &mut cache);
            assert!(schema.is_none());
            assert_eq!(problems.len(), 1);
        }

        #[test]
        fn an_unknown_schema_name_degrades_that_change_alone() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let pkg_dir = repo.join("pkg/spec-driven");
            write_schema_yaml(
                &pkg_dir,
                "spec-driven",
                &[("proposal", "proposal.md"), ("tasks", "tasks.md")],
            );

            let fake = FakeCli::new();
            fake.register_openspec(
                &["schema", "which", "outside-in-tdd", "--json"],
                Err(CliError::Failed {
                    program: "openspec".to_string(),
                    args: vec![
                        "schema".to_string(),
                        "which".to_string(),
                        "outside-in-tdd".to_string(),
                        "--json".to_string(),
                    ],
                    code: Some(1),
                    stderr: String::new(),
                }),
            );
            fake.register_openspec(
                &["schema", "which", "spec-driven", "--json"],
                which_response(&pkg_dir, "spec-driven"),
            );

            let mut cache = HashMap::new();
            let (tdd, tdd_problems) = resolve_cli_schema(&fake, &repo, "tdd", &mut cache);
            let (unknown, unknown_problems) =
                resolve_cli_schema(&fake, &repo, "outside-in-tdd", &mut cache);
            let (spec_driven, spec_driven_problems) =
                resolve_cli_schema(&fake, &repo, "spec-driven", &mut cache);

            assert!(tdd.is_some());
            assert!(tdd_problems.is_empty());
            assert!(unknown.is_none());
            assert_eq!(unknown_problems.len(), 1);
            assert!(unknown_problems[0].contains("outside-in-tdd"));
            assert!(unknown_problems[0].contains('1'));
            assert!(spec_driven.is_some());
            assert!(spec_driven_problems.is_empty());
        }

        #[test]
        fn a_which_path_naming_a_directory_with_no_schema_yaml_stops_the_tier() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            let empty_dir = repo.join("empty");
            mkdir(&empty_dir);

            let fake = FakeCli::new();
            fake.register_openspec(
                &["schema", "which", "ghost", "--json"],
                which_response(&empty_dir, "ghost"),
            );

            let mut cache = HashMap::new();
            let (schema, problems) = resolve_cli_schema(&fake, &repo, "ghost", &mut cache);
            assert!(schema.is_none());
            assert_eq!(problems.len(), 1);
            assert!(problems[0].contains("schema.yaml"));
            let schema_which_calls = fake
                .calls()
                .iter()
                .filter(|(_, args)| args.first().map(String::as_str) == Some("schema"))
                .count();
            assert_eq!(schema_which_calls, 1);
        }

        #[test]
        fn an_unstartable_openspec_during_the_fallback_degrades_that_change() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());

            let fake = FakeCli::new();
            fake.register_openspec(
                &["schema", "which", "spec-driven", "--json"],
                Err(CliError::NotStarted {
                    program: "openspec".to_string(),
                    args: vec![
                        "schema".to_string(),
                        "which".to_string(),
                        "spec-driven".to_string(),
                        "--json".to_string(),
                    ],
                    reason: "No such file or directory".to_string(),
                }),
            );

            let mut cache = HashMap::new();
            let (schema, problems) = resolve_cli_schema(&fake, &repo, "spec-driven", &mut cache);
            assert!(schema.is_none());
            assert_eq!(problems.len(), 1);
            assert!(problems[0].contains("spec-driven"));
            assert!(problems[0].contains("openspec"));
            assert!(problems[0].contains("No such file or directory"));
        }

        #[test]
        fn an_exec_failure_during_the_fallback_tier_carries_its_stderr() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());

            let failed = |stderr: &str| {
                Err(CliError::Failed {
                    program: "openspec".to_string(),
                    args: vec![
                        "schema".to_string(),
                        "which".to_string(),
                        "spec-driven".to_string(),
                        "--json".to_string(),
                    ],
                    code: Some(127),
                    stderr: stderr.to_string(),
                })
            };

            let fake = FakeCli::new();
            // Measured on this machine: `env -i PATH=/usr/bin:/bin
            // "$(readlink -f ~/.nvm/versions/node/v24.18.0/bin/openspec)"
            // list --json` -> exit=127, stdout=[], stderr=[env: node: No
            // such file or directory].
            fake.register_openspec(
                &["schema", "which", "spec-driven", "--json"],
                failed("env: node: No such file or directory\n"),
            );
            let mut cache = HashMap::new();
            let (schema, problems) = resolve_cli_schema(&fake, &repo, "spec-driven", &mut cache);
            assert!(schema.is_none());
            assert_eq!(problems.len(), 1);
            assert!(problems[0].contains("spec-driven"));
            assert!(problems[0].contains("127"));
            assert!(problems[0].contains("env: node: No such file or directory"));

            let fake_empty = FakeCli::new();
            fake_empty.register_openspec(&["schema", "which", "spec-driven", "--json"], failed(""));
            let mut cache_empty = HashMap::new();
            let (_, empty_problems) =
                resolve_cli_schema(&fake_empty, &repo, "spec-driven", &mut cache_empty);
            assert_eq!(empty_problems.len(), 1);

            // Measured: `openspec schema which nosuchschema --json` from
            // `/tmp` -> exit=1, the real answer on stdout, stderr=[Note:
            // Schema commands are experimental and may change.].
            let fake_banner = FakeCli::new();
            fake_banner.register_openspec(
                &["schema", "which", "spec-driven", "--json"],
                failed("Note: Schema commands are experimental and may change.\n"),
            );
            let mut cache_banner = HashMap::new();
            let (_, banner_problems) =
                resolve_cli_schema(&fake_banner, &repo, "spec-driven", &mut cache_banner);
            assert_eq!(banner_problems, empty_problems);
        }

        #[test]
        fn an_unusable_schema_yaml_at_the_cli_named_path_degrades_that_change() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());

            let invalid_dir = repo.join("invalid");
            write(&invalid_dir.join("schema.yaml"), "not: [valid");
            let unreadable_dir = repo.join("unreadable");
            mkdir(&unreadable_dir.join("schema.yaml"));

            let fake = FakeCli::new();
            fake.register_openspec(
                &["schema", "which", "invalid-one", "--json"],
                which_response(&invalid_dir, "invalid-one"),
            );
            fake.register_openspec(
                &["schema", "which", "unreadable-one", "--json"],
                which_response(&unreadable_dir, "unreadable-one"),
            );

            let schema_which_call_count = |fake: &FakeCli| {
                fake.calls()
                    .iter()
                    .filter(|(_, args)| args.first().map(String::as_str) == Some("schema"))
                    .count()
            };

            let mut cache = HashMap::new();
            let (invalid_schema, invalid_problems) =
                resolve_cli_schema(&fake, &repo, "invalid-one", &mut cache);
            assert!(invalid_schema.is_none());
            assert_eq!(invalid_problems.len(), 1);
            assert_eq!(schema_which_call_count(&fake), 1);

            let (unreadable_schema, unreadable_problems) =
                resolve_cli_schema(&fake, &repo, "unreadable-one", &mut cache);
            assert!(unreadable_schema.is_none());
            assert_eq!(unreadable_problems.len(), 1);
            assert_eq!(schema_which_call_count(&fake), 2);
        }

        #[test]
        fn a_malformed_which_payload_is_a_problem_not_a_panic() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());

            for (name, payload) in [
                ("empty-body", ""),
                ("null-body", "null"),
                ("non-string-path", r#"{"path": 7}"#),
                ("no-path", r#"{"name":"x"}"#),
            ] {
                let fake = FakeCli::new();
                fake.register_openspec(
                    &["schema", "which", name, "--json"],
                    Ok(payload.to_string()),
                );
                let mut cache = HashMap::new();
                let (schema, problems) = resolve_cli_schema(&fake, &repo, name, &mut cache);
                assert!(schema.is_none(), "case {name}");
                assert_eq!(problems.len(), 1, "case {name}");
            }
        }

        #[test]
        fn a_leading_non_json_line_is_not_tolerated_by_the_fallback() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            let fake = FakeCli::new();
            fake.register_openspec(
                &["schema", "which", "noisy", "--json"],
                Ok(
                    "Note: Schema commands are experimental and may change.\n{\"path\":\"/x\"}"
                        .to_string(),
                ),
            );
            let mut cache = HashMap::new();
            let (schema, problems) = resolve_cli_schema(&fake, &repo, "noisy", &mut cache);
            assert!(schema.is_none());
            assert_eq!(problems.len(), 1);
        }

        #[test]
        fn a_parser_problem_from_the_cli_named_schema_reaches_every_change_using_it() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            let pkg_dir = repo.join("pkg/spec-driven");
            // A schema.yaml with one unusable artifact entry (no `id`)
            // alongside two usable ones, so `schema::load_dir` returns `Ok`
            // carrying two artifacts and one problem.
            write(
                &pkg_dir.join("schema.yaml"),
                "name: spec-driven\nartifacts:\n  - generates: no-id.md\n  - id: proposal\n    generates: proposal.md\n  - id: tasks\n    generates: tasks.md\n",
            );

            let fake = FakeCli::new();
            fake.register_openspec(
                &["schema", "which", "spec-driven", "--json"],
                which_response(&pkg_dir, "spec-driven"),
            );

            let mut cache = HashMap::new();
            let (first_schema, first_problems) =
                resolve_cli_schema(&fake, &repo, "spec-driven", &mut cache);
            let (second_schema, second_problems) =
                resolve_cli_schema(&fake, &repo, "spec-driven", &mut cache);

            assert!(first_schema.is_some());
            assert!(second_schema.is_some());
            assert_eq!(first_problems.len(), 1);
            assert_eq!(first_problems, second_problems);
        }

        #[test]
        fn three_changes_sharing_one_unvendored_schema_ask_the_cli_once() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            let pkg_dir = repo.join("pkg/spec-driven");
            write_schema_yaml(
                &pkg_dir,
                "spec-driven",
                &[("proposal", "proposal.md"), ("tasks", "tasks.md")],
            );

            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(r#"{"changes":[
                    {"name":"alpha","completedTasks":0,"totalTasks":0,"lastModified":"x","status":"y"},
                    {"name":"mike","completedTasks":0,"totalTasks":0,"lastModified":"x","status":"y"},
                    {"name":"zulu","completedTasks":0,"totalTasks":0,"lastModified":"x","status":"y"}
                ],"root":{"path":"/repo","source":"nearest"}}"#
                    .to_string()),
            );
            for name in ["alpha", "mike", "zulu"] {
                fake.register_openspec(
                    &["instructions", "apply", "--change", name, "--json"],
                    Ok(format!(
                        r#"{{"schemaName":"spec-driven","changeDir":"/repo/openspec/changes/{name}","contextFiles":{{}}}}"#
                    )),
                );
            }
            fake.register_openspec(
                &["schema", "which", "spec-driven", "--json"],
                which_response(&pkg_dir, "spec-driven"),
            );

            // Drive exactly the sequence `from_cli` will later automate,
            // through the traits directly — this test is what proves the
            // resolver's caching property end to end, ahead of `from_cli`
            // existing.
            let list_text = OpenspecCli::run(&fake, &["list", "--json"]).unwrap();
            let list_payload = parse_list(&list_text).unwrap();
            let mut cache = HashMap::new();
            for entry in &list_payload.changes {
                let apply_text = OpenspecCli::run(
                    &fake,
                    &["instructions", "apply", "--change", &entry.name, "--json"],
                )
                .unwrap();
                let apply = parse_apply(&apply_text).unwrap();
                let (schema, problems) =
                    resolve_cli_schema(&fake, &repo, &apply.schema_name, &mut cache);
                let schema = schema.expect("should resolve");
                let ids: Vec<&str> = schema.artifacts.iter().map(|a| a.id.as_str()).collect();
                assert_eq!(ids, vec!["proposal", "tasks"]);
                assert!(problems.is_empty());
            }

            let calls = fake.calls();
            assert_eq!(calls.len(), 5);
            let schema_which_calls = calls
                .iter()
                .filter(|(_, args)| args.first().map(String::as_str) == Some("schema"))
                .count();
            assert_eq!(schema_which_calls, 1);
        }

        #[test]
        fn a_failed_lookup_is_cached_rather_than_retried_per_change() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());

            let fake = FakeCli::new();
            fake.register_openspec(
                &["schema", "which", "outside-in-tdd", "--json"],
                Err(CliError::Failed {
                    program: "openspec".to_string(),
                    args: vec![
                        "schema".to_string(),
                        "which".to_string(),
                        "outside-in-tdd".to_string(),
                        "--json".to_string(),
                    ],
                    code: Some(1),
                    stderr: String::new(),
                }),
            );

            let mut cache = HashMap::new();
            for _ in 0..3 {
                let (schema, problems) =
                    resolve_cli_schema(&fake, &repo, "outside-in-tdd", &mut cache);
                assert!(schema.is_none());
                assert_eq!(problems.len(), 1);
            }
            let schema_which_calls = fake
                .calls()
                .iter()
                .filter(|(_, args)| args.first().map(String::as_str) == Some("schema"))
                .count();
            assert_eq!(schema_which_calls, 1);
        }

        #[test]
        fn two_different_schema_names_are_asked_for_separately() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            let a_dir = repo.join("pkg/spec-driven");
            write_schema_yaml(&a_dir, "spec-driven", &[("proposal", "proposal.md")]);
            let b_dir = repo.join("pkg/other-schema");
            write_schema_yaml(&b_dir, "other-schema", &[("notes", "notes.md")]);

            let fake = FakeCli::new();
            fake.register_openspec(
                &["schema", "which", "spec-driven", "--json"],
                which_response(&a_dir, "spec-driven"),
            );
            fake.register_openspec(
                &["schema", "which", "other-schema", "--json"],
                which_response(&b_dir, "other-schema"),
            );

            let mut cache = HashMap::new();
            let (a, _) = resolve_cli_schema(&fake, &repo, "spec-driven", &mut cache);
            let (b, _) = resolve_cli_schema(&fake, &repo, "other-schema", &mut cache);
            assert_eq!(
                a.unwrap()
                    .artifacts
                    .iter()
                    .map(|x| x.id.clone())
                    .collect::<Vec<_>>(),
                vec!["proposal"]
            );
            assert_eq!(
                b.unwrap()
                    .artifacts
                    .iter()
                    .map(|x| x.id.clone())
                    .collect::<Vec<_>>(),
                vec!["notes"]
            );
            let schema_which_calls = fake
                .calls()
                .iter()
                .filter(|(_, args)| args.first().map(String::as_str) == Some("schema"))
                .count();
            assert_eq!(schema_which_calls, 2);
        }
    }

    /// CHARACTERIZATION (`cli-parity` group 3): pins the illegal-name
    /// outcome `resolve_cli_schema_uncached`'s catch-all `Err(err) =>` arm
    /// already produces, before that arm is replaced with explicit
    /// `LoadError` arms. Placed directly in `changes::tests` rather than in
    /// `mod schema_fallback` to match the design's own filter,
    /// `changes::tests::an_illegal_schema_name_is_not_repaired`. See
    /// `schema-cli-fallback` -> "An illegal schema name is not repaired by
    /// the CLI".
    #[test]
    fn an_illegal_schema_name_is_not_repaired_by_the_cli() {
        use crate::testutil::{ScratchDir, canonical};

        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());

        let fake = crate::cli::FakeCli::new();
        // No "schema which" registration — the fake panics on an
        // unregistered pair, so a passing test proves no call was made.
        let mut cache = std::collections::HashMap::new();
        let (schema, problems) = resolve_cli_schema(&fake, &repo, "../../../../etc", &mut cache);
        assert!(schema.is_none());
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("../../../../etc"));
        assert!(fake.calls().is_empty());
    }

    // --- group 8: `from_cli` — the composition (`mod from_cli`) ------------

    mod from_cli {
        // The module and the function under test share a name; see `mod
        // cli_artifacts` above for why the explicit import comes first.
        use super::super::from_cli;
        use super::*;
        use crate::cli::{CliError, FakeCli};

        fn list_json(root: &std::path::Path, entries: &[(&str, usize, usize)]) -> String {
            let changes: Vec<String> = entries
                .iter()
                .copied()
                .map(|(name, completed, total)| {
                    format!(
                        r#"{{"name":{name:?},"completedTasks":{completed},"totalTasks":{total},"lastModified":"x","status":"y"}}"#
                    )
                })
                .collect();
            format!(
                r#"{{"changes":[{}],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                changes.join(","),
                root.display().to_string()
            )
        }

        fn apply_json(
            schema_name: &str,
            change_dir: &std::path::Path,
            context_files: &[(&str, &[&str])],
        ) -> String {
            let entries: Vec<String> = context_files
                .iter()
                .copied()
                .map(|(id, paths)| {
                    let paths: Vec<String> = paths.iter().map(|p| format!("{p:?}")).collect();
                    format!("{id:?}:[{}]", paths.join(","))
                })
                .collect();
            format!(
                r#"{{"schemaName":{schema_name:?},"changeDir":{:?},"contextFiles":{{{}}}}}"#,
                change_dir.display().to_string(),
                entries.join(",")
            )
        }

        fn failed(args: &[&str]) -> CliError {
            CliError::Failed {
                program: "openspec".to_string(),
                args: args.iter().map(|a| a.to_string()).collect(),
                code: Some(1),
                stderr: String::new(),
            }
        }

        #[test]
        fn a_two_change_repository_drives_exactly_three_invocations() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);

            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("zulu", 0, 0), ("alpha", 1, 2)])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "zulu", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/zulu"), &[])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/alpha"), &[])),
            );

            let result = from_cli(&fake, &repo);
            assert_eq!(result.active.len(), 2);
            assert!(result.problems.is_empty());

            assert_eq!(
                fake.calls(),
                vec![
                    (
                        crate::cli::Program::Openspec,
                        vec!["list".to_string(), "--json".to_string()]
                    ),
                    (
                        crate::cli::Program::Openspec,
                        vec![
                            "instructions".to_string(),
                            "apply".to_string(),
                            "--change".to_string(),
                            "zulu".to_string(),
                            "--json".to_string(),
                        ]
                    ),
                    (
                        crate::cli::Program::Openspec,
                        vec![
                            "instructions".to_string(),
                            "apply".to_string(),
                            "--change".to_string(),
                            "alpha".to_string(),
                            "--json".to_string(),
                        ]
                    ),
                ]
            );
        }

        #[test]
        fn no_status_invocation_is_made_even_when_an_apply_call_fails() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            let fake = FakeCli::new();
            fake.register_openspec(&["list", "--json"], Ok(list_json(&repo, &[("zulu", 0, 0)])));
            fake.register_openspec(
                &["instructions", "apply", "--change", "zulu", "--json"],
                Err(failed(&[
                    "instructions",
                    "apply",
                    "--change",
                    "zulu",
                    "--json",
                ])),
            );
            // No "status" registration at all — the fake would panic if
            // `from_cli` reached for it.
            let result = from_cli(&fake, &repo);
            assert!(result.active.is_empty());
            assert_eq!(result.problems.len(), 1);
        }

        #[test]
        fn the_argument_vector_carries_no_sort_flag() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            let fake = FakeCli::new();
            fake.register_openspec(&["list", "--json"], Ok(list_json(&repo, &[])));
            let _ = from_cli(&fake, &repo);
            let calls = fake.calls();
            assert_eq!(calls[0].1, vec!["list".to_string(), "--json".to_string()]);
        }

        #[test]
        fn progress_is_read_from_the_list_payload_pair() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 4, 9)])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(format!(
                    r#"{{"schemaName":"tdd","changeDir":{:?},"contextFiles":{{}},"progress":{{"total":0,"complete":0,"remaining":0}}}}"#,
                    repo.join("openspec/changes/alpha").display().to_string()
                )),
            );
            let result = from_cli(&fake, &repo);
            let alpha = &result.active[0];
            assert_eq!(
                alpha.progress,
                crate::tasks::Progress {
                    completed: 4,
                    total: 9
                }
            );
        }

        #[test]
        fn the_most_recently_modified_default_order_is_replaced_by_byte_order() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(
                    &repo,
                    &[("zulu", 0, 0), ("mike", 0, 0), ("alpha", 0, 0)],
                )),
            );
            for name in ["zulu", "mike", "alpha"] {
                fake.register_openspec(
                    &["instructions", "apply", "--change", name, "--json"],
                    Ok(apply_json(
                        "tdd",
                        &repo.join(format!("openspec/changes/{name}")),
                        &[],
                    )),
                );
            }
            let result = from_cli(&fake, &repo);
            let names: Vec<&str> = result.active.iter().map(|c| c.name.as_str()).collect();
            assert_eq!(names, vec!["alpha", "mike", "zulu"]);

            let calls = fake.calls();
            let apply_order: Vec<&str> = calls[1..]
                .iter()
                .map(|(_, args)| args[3].as_str())
                .collect();
            assert_eq!(apply_order, vec!["zulu", "mike", "alpha"]);
        }

        #[test]
        fn case_and_digits_order_by_byte_not_by_locale() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(
                    &repo,
                    &[
                        ("Beta", 0, 0),
                        ("alpha", 0, 0),
                        ("10-late", 0, 0),
                        ("2-early", 0, 0),
                    ],
                )),
            );
            for name in ["Beta", "alpha", "10-late", "2-early"] {
                fake.register_openspec(
                    &["instructions", "apply", "--change", name, "--json"],
                    Ok(apply_json(
                        "tdd",
                        &repo.join(format!("openspec/changes/{name}")),
                        &[],
                    )),
                );
            }
            let result = from_cli(&fake, &repo);
            let names: Vec<&str> = result.active.iter().map(|c| c.name.as_str()).collect();
            assert_eq!(names, vec!["10-late", "2-early", "Beta", "alpha"]);
        }

        #[test]
        fn a_change_dir_whose_final_component_is_not_the_change_name_is_rejected() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 0, 0)])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/beta"), &[])),
            );
            let result = from_cli(&fake, &repo);
            assert!(result.active.is_empty());
            assert_eq!(result.problems.len(), 1);
            assert!(result.problems[0].contains("alpha"));
            assert!(result.problems[0].contains("beta"));
        }

        #[test]
        fn every_produced_change_is_active_and_satisfies_the_shared_invariants() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(
                    &repo,
                    &[("alpha", 1, 2), ("mike", 0, 0), ("zulu", 3, 3)],
                )),
            );
            for name in ["alpha", "mike", "zulu"] {
                fake.register_openspec(
                    &["instructions", "apply", "--change", name, "--json"],
                    Ok(apply_json(
                        "tdd",
                        &repo.join(format!("openspec/changes/{name}")),
                        &[],
                    )),
                );
            }
            let result = from_cli(&fake, &repo);
            assert_eq!(result.active.len(), 3);
            for change in &result.active {
                assert_eq!(change.origin, Origin::Active);
                assert_invariants(change);
            }
        }

        #[test]
        fn an_absent_openspec_binary_yields_an_empty_result_and_one_problem() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Err(CliError::NotStarted {
                    program: "openspec".to_string(),
                    args: vec!["list".to_string(), "--json".to_string()],
                    reason: "No such file or directory (os error 2)".to_string(),
                }),
            );
            let result = from_cli(&fake, &repo);
            assert!(result.active.is_empty());
            assert_eq!(result.problems.len(), 1);
            assert!(result.problems[0].contains("openspec"));
            assert!(result.problems[0].contains("list"));
            assert!(result.problems[0].contains("No such file or directory (os error 2)"));
        }

        #[test]
        fn two_different_spawn_failures_produce_two_different_problems() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());

            let not_started = |reason: &str| {
                Err(CliError::NotStarted {
                    program: "openspec".to_string(),
                    args: vec!["list".to_string(), "--json".to_string()],
                    reason: reason.to_string(),
                })
            };

            let fake_a = FakeCli::new();
            fake_a.register_openspec(
                &["list", "--json"],
                not_started("No such file or directory (os error 2)"),
            );
            let result_a = from_cli(&fake_a, &repo);

            let fake_b = FakeCli::new();
            fake_b.register_openspec(
                &["list", "--json"],
                not_started("Exec format error (os error 8)"),
            );
            let result_b = from_cli(&fake_b, &repo);

            assert_eq!(result_a.problems.len(), 1);
            assert_eq!(result_b.problems.len(), 1);
            assert_ne!(result_a.problems[0], result_b.problems[0]);
        }

        #[test]
        fn a_non_zero_exit_from_list_yields_an_empty_result_and_one_problem() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            let fake = FakeCli::new();
            fake.register_openspec(&["list", "--json"], Err(failed(&["list", "--json"])));
            let result = from_cli(&fake, &repo);
            assert!(result.active.is_empty());
            assert_eq!(result.problems.len(), 1);
            assert!(result.problems[0].contains('1'));
        }

        #[test]
        fn a_schema_the_cli_rejects_removes_one_change_and_keeps_the_others() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(
                    &repo,
                    &[("alpha", 0, 0), ("mike", 0, 0), ("zulu", 0, 0)],
                )),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/alpha"), &[])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "mike", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/mike"), &[])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "zulu", "--json"],
                // Empty stderr: `instructions apply` was measured writing
                // its diagnostic ("Unknown schema \"outside-in-tdd\"") to
                // stdout with a 0-byte stderr, which `CliError::Failed`
                // never carries. The assertion below is the proof that no
                // reason is invented when the seam carried none — not that
                // a carried one is dropped.
                Err(CliError::Failed {
                    program: "openspec".to_string(),
                    args: vec![
                        "instructions".to_string(),
                        "apply".to_string(),
                        "--change".to_string(),
                        "zulu".to_string(),
                        "--json".to_string(),
                    ],
                    code: Some(1),
                    stderr: String::new(),
                }),
            );
            let result = from_cli(&fake, &repo);
            let names: Vec<&str> = result.active.iter().map(|c| c.name.as_str()).collect();
            assert_eq!(names, vec!["alpha", "mike"]);
            assert_eq!(result.problems.len(), 1);
            assert!(result.problems[0].contains("zulu"));
            assert!(result.problems[0].contains('1'));
            assert!(!result.problems[0].contains("Unknown schema"));
        }

        #[test]
        fn malformed_json_from_a_single_apply_call_is_contained_to_that_change() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(
                    &repo,
                    &[("alpha", 0, 0), ("mike", 0, 0), ("zulu", 0, 0)],
                )),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json(
                    "tdd",
                    &repo.join("openspec/changes/alpha"),
                    &[("proposal", &["/x/proposal.md"])],
                )),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "mike", "--json"],
                Ok("{ this is not json".to_string()),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "zulu", "--json"],
                Ok(apply_json(
                    "tdd",
                    &repo.join("openspec/changes/zulu"),
                    &[("proposal", &["/z/proposal.md"])],
                )),
            );
            let result = from_cli(&fake, &repo);
            let names: Vec<&str> = result.active.iter().map(|c| c.name.as_str()).collect();
            assert_eq!(names, vec!["alpha", "zulu"]);
            assert_eq!(result.problems.len(), 1);
            assert!(result.problems[0].contains("mike"));
            for change in &result.active {
                assert_eq!(change.artifacts.len(), TDD_ARTIFACTS.len());
            }
        }

        #[test]
        fn an_apply_payload_missing_context_files_is_a_per_change_failure() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 0, 0)])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(format!(
                    r#"{{"schemaName":"tdd","changeDir":{:?}}}"#,
                    repo.join("openspec/changes/alpha").display().to_string()
                )),
            );
            let result = from_cli(&fake, &repo);
            assert!(result.active.is_empty());
            assert_eq!(result.problems.len(), 1);
            assert!(result.problems[0].contains("alpha"));
        }

        #[test]
        fn empty_stdout_from_list_is_a_parse_failure_not_an_empty_repository() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            let fake = FakeCli::new();
            fake.register_openspec(&["list", "--json"], Ok(String::new()));
            let result = from_cli(&fake, &repo);
            assert!(result.active.is_empty());
            assert_eq!(result.problems.len(), 1);

            let fake2 = FakeCli::new();
            fake2.register_openspec(&["list", "--json"], Ok(list_json(&repo, &[])));
            let result2 = from_cli(&fake2, &repo);
            assert!(result2.active.is_empty());
            assert!(result2.problems.is_empty());
        }

        #[test]
        fn a_bare_array_is_rejected_at_the_composition() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            let fake = FakeCli::new();
            fake.register_openspec(&["list", "--json"], Ok("[]".to_string()));
            let result = from_cli(&fake, &repo);
            assert!(result.active.is_empty());
            assert_eq!(result.problems.len(), 1);
        }

        #[test]
        fn a_mismatched_root_discards_the_whole_cli_result() {
            let scratch = ScratchDir::new();
            let repo_a = canonical(scratch.path()).join("repo-a");
            mkdir(&repo_a);
            let repo_b = canonical(scratch.path()).join("repo-b");
            mkdir(&repo_b);

            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo_b, &[("alpha", 0, 0), ("mike", 0, 0)])),
            );
            let result = from_cli(&fake, &repo_a);
            assert!(result.active.is_empty());
            assert_eq!(result.problems.len(), 1);
            assert!(result.problems[0].contains(&repo_a.display().to_string()));
            assert!(result.problems[0].contains(&repo_b.display().to_string()));
            assert_eq!(fake.calls().len(), 1);
        }

        #[test]
        fn a_symlinked_repository_root_is_not_a_disagreement() {
            let scratch = ScratchDir::new();
            let real_repo = canonical(scratch.path()).join("real");
            mkdir(&real_repo);
            vendor_schema(&real_repo, "tdd", TDD_ARTIFACTS);
            let link = scratch.path().join("link");
            symlink(&real_repo, &link);

            let fake = FakeCli::new();
            // A non-empty payload, so "the changes are produced normally" is
            // actually exercised — an empty payload can only show that no
            // problem was recorded, not that a change survives this path.
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&real_repo, &[("alpha", 1, 2)])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json(
                    "tdd",
                    &real_repo.join("openspec/changes/alpha"),
                    &[],
                )),
            );
            let result = from_cli(&fake, &link);
            assert!(result.problems.is_empty());
            assert_eq!(result.active.len(), 1);
            assert_eq!(result.active[0].name, "alpha");
        }

        #[test]
        fn an_envelope_with_no_root_is_treated_as_a_disagreement() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(r#"{"changes":[{"name":"alpha","completedTasks":0,"totalTasks":0,"lastModified":"x","status":"y"}]}"#.to_string()),
            );
            let result = from_cli(&fake, &repo);
            assert!(result.active.is_empty());
            assert_eq!(result.problems.len(), 1);
            assert_eq!(fake.calls().len(), 1);
        }

        #[test]
        fn an_omitted_context_files_key_is_an_empty_path_list_end_to_end() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 0, 0)])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json(
                    "tdd",
                    &repo.join("openspec/changes/alpha"),
                    &[
                        ("proposal", &["/x/proposal.md"]),
                        ("specs", &["/x/specs/a.md"]),
                        ("tasks", &["/x/tasks.md"]),
                    ],
                )),
            );
            let result = from_cli(&fake, &repo);
            let alpha = &result.active[0];
            let ids: Vec<&str> = alpha.artifacts.iter().map(|a| a.id.as_str()).collect();
            assert_eq!(
                ids,
                vec!["proposal", "specs", "design", "tasks", "planning-review"]
            );
            let design = alpha.artifacts.iter().find(|a| a.id == "design").unwrap();
            assert!(design.paths.is_empty());
        }

        #[test]
        fn a_multi_file_artifact_survives_the_composition() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 0, 0)])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json(
                    "tdd",
                    &repo.join("openspec/changes/alpha"),
                    &[(
                        "specs",
                        &["/x/specs/cap-one/spec.md", "/x/specs/cap-two/spec.md"],
                    )],
                )),
            );
            let result = from_cli(&fake, &repo);
            let alpha = &result.active[0];
            let specs = alpha.artifacts.iter().find(|a| a.id == "specs").unwrap();
            assert_eq!(
                specs.paths,
                vec![
                    PathBuf::from("/x/specs/cap-one/spec.md"),
                    PathBuf::from("/x/specs/cap-two/spec.md"),
                ]
            );
        }

        #[test]
        fn a_context_files_key_naming_no_schema_artifact_is_ignored_end_to_end() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 0, 0)])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json(
                    "tdd",
                    &repo.join("openspec/changes/alpha"),
                    &[("legacy", &["/x/legacy.md"])],
                )),
            );
            let result = from_cli(&fake, &repo);
            let alpha = &result.active[0];
            assert!(!alpha.artifacts.iter().any(|a| a.id == "legacy"));
            assert_eq!(alpha.problems.len(), 1);
            assert!(alpha.problems[0].contains("legacy"));
        }

        #[test]
        fn a_full_from_cli_run_leaves_the_tree_byte_identical() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let pkg_dir = repo.join("pkg/spec-driven");
            write(
                &pkg_dir.join("schema.yaml"),
                "name: spec-driven\nartifacts:\n  - id: proposal\n    generates: proposal.md\n  - id: tasks\n    generates: tasks.md\n",
            );

            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(
                    &repo,
                    &[
                        ("alpha", 1, 2),
                        ("mike", 0, 0),
                        ("zulu", 0, 0),
                        ("nina", 0, 0),
                    ],
                )),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/alpha"), &[])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "mike", "--json"],
                Err(failed(&[
                    "instructions",
                    "apply",
                    "--change",
                    "mike",
                    "--json",
                ])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "zulu", "--json"],
                Ok(apply_json(
                    "spec-driven",
                    &repo.join("openspec/changes/zulu"),
                    &[],
                )),
            );
            fake.register_openspec(
                &["schema", "which", "spec-driven", "--json"],
                Ok(format!(
                    r#"{{"name":"spec-driven","source":"package","path":{:?},"shadows":[]}}"#,
                    pkg_dir.display().to_string()
                )),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "nina", "--json"],
                Ok("{ not json".to_string()),
            );

            let before = snapshot(&repo);
            let cwd = std::env::current_dir().expect("current dir");
            let before_cwd = crate::testutil::shallow_snapshot(&cwd);

            let _ = from_cli(&fake, &repo);
            let _ = from_cli(&fake, &repo);

            let after = snapshot(&repo);
            let after_cwd = crate::testutil::shallow_snapshot(&cwd);
            assert_eq!(before, after);
            assert_eq!(before_cwd, after_cwd);
        }

        // --- Change Review (task 11.2): closes two CRITICAL coverage gaps --
        // `schema_fallback`'s scenarios state their THEN as a property of a
        // produced `Change`, but every one of its tests calls
        // `resolve_cli_schema` directly and never builds one through
        // `from_cli`. This is the composition-level binding: a schema that
        // fails to resolve must still leave the change in `active`, with an
        // empty artifact list and one problem on *that change's own*
        // `problems` — never dropped, and never a top-level `CliChanges`
        // problem. Demonstrated in review to be unbound: `continue`-ing
        // (dropping the change) on `schema.is_none()` left the whole suite
        // green before this test existed.
        #[test]
        fn a_change_whose_schema_fails_to_resolve_still_appears_in_active() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(
                    &repo,
                    &[("alpha", 0, 0), ("mike", 0, 0), ("zulu", 0, 0)],
                )),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/alpha"), &[])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "mike", "--json"],
                Ok(apply_json(
                    "outside-in-tdd",
                    &repo.join("openspec/changes/mike"),
                    &[],
                )),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "zulu", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/zulu"), &[])),
            );
            fake.register_openspec(
                &["schema", "which", "outside-in-tdd", "--json"],
                Err(failed(&["schema", "which", "outside-in-tdd", "--json"])),
            );

            let result = from_cli(&fake, &repo);
            let names: Vec<&str> = result.active.iter().map(|c| c.name.as_str()).collect();
            assert_eq!(names, vec!["alpha", "mike", "zulu"]);
            assert!(result.problems.is_empty());

            let mike = result.active.iter().find(|c| c.name == "mike").unwrap();
            assert!(mike.artifacts.is_empty());
            assert_eq!(mike.problems.len(), 1);
            assert!(mike.problems[0].contains("outside-in-tdd"));
            assert!(mike.problems[0].contains('1'));

            for name in ["alpha", "zulu"] {
                let change = result.active.iter().find(|c| c.name == name).unwrap();
                assert_eq!(change.artifacts.len(), TDD_ARTIFACTS.len());
                assert!(change.problems.is_empty());
            }
        }

        // Binds `schema-cli-fallback`'s "at most once per name per call" to
        // the actual composition. `schema_fallback`'s tests of the same
        // shape hand-drive `run`/`parse_list`/`parse_apply`/
        // `resolve_cli_schema` with a cache *they* allocate, ahead of
        // `from_cli` existing; that cache was never lifted into `from_cli`
        // once group 8 landed, so it proved nothing about the real cache's
        // lifetime. Demonstrated in review: moving `from_cli`'s cache
        // allocation inside the per-change loop — one `schema which` per
        // change, the N+1 regression the requirement exists to prevent —
        // left the suite green before this test existed.
        #[test]
        fn from_cli_asks_the_cli_once_for_a_schema_shared_by_three_changes() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            let pkg_dir = repo.join("pkg/spec-driven");
            write(
                &pkg_dir.join("schema.yaml"),
                "name: spec-driven\nartifacts:\n  - id: proposal\n    generates: proposal.md\n  - id: tasks\n    generates: tasks.md\n",
            );
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(
                    &repo,
                    &[("alpha", 0, 0), ("mike", 0, 0), ("zulu", 0, 0)],
                )),
            );
            for name in ["alpha", "mike", "zulu"] {
                fake.register_openspec(
                    &["instructions", "apply", "--change", name, "--json"],
                    Ok(apply_json(
                        "spec-driven",
                        &repo.join(format!("openspec/changes/{name}")),
                        &[],
                    )),
                );
            }
            fake.register_openspec(
                &["schema", "which", "spec-driven", "--json"],
                Ok(format!(
                    r#"{{"name":"spec-driven","source":"package","path":{:?},"shadows":[]}}"#,
                    pkg_dir.display().to_string()
                )),
            );

            let result = from_cli(&fake, &repo);
            assert_eq!(result.active.len(), 3);
            for change in &result.active {
                let ids: Vec<&str> = change.artifacts.iter().map(|a| a.id.as_str()).collect();
                assert_eq!(ids, vec!["proposal", "tasks"]);
                assert!(change.problems.is_empty());
            }

            let calls = fake.calls();
            assert_eq!(
                calls.len(),
                5,
                "list + 3 applies + exactly one schema which"
            );
            let schema_which_calls = calls
                .iter()
                .filter(|(_, args)| args.first().map(String::as_str) == Some("schema"))
                .count();
            assert_eq!(schema_which_calls, 1);
        }
    }

    // --- cli-parity group 4: `parse_apply` refuses an illegal `schemaName` -
    // Both tests below sit directly in `mod tests` (not nested in `mod
    // from_cli`) so their full path matches design.md's Test Strategy
    // filters `changes::tests::an_apply_payload_whose_schema_name` and
    // `changes::tests::every_other_shape_is_legal_name`.

    fn schema_guard_list_json(repo: &std::path::Path, names: &[&str]) -> String {
        let changes: Vec<String> = names
            .iter()
            .map(|name| {
                format!(
                    r#"{{"name":{name:?},"completedTasks":0,"totalTasks":0,"lastModified":"x","status":"y"}}"#
                )
            })
            .collect();
        format!(
            r#"{{"changes":[{}],"root":{{"path":{:?},"source":"nearest"}}}}"#,
            changes.join(","),
            repo.display().to_string()
        )
    }

    fn schema_guard_apply_json(schema_name: &str, change_dir: &std::path::Path) -> String {
        format!(
            r#"{{"schemaName":{schema_name:?},"changeDir":{:?},"contextFiles":{{}}}}"#,
            change_dir.display().to_string()
        )
    }

    #[test]
    fn an_apply_payload_whose_schema_name_would_escape_the_schema_directory_is_refused() {
        use crate::cli::FakeCli;

        let scratch = ScratchDir::new();
        let repo = canonical(scratch.path());
        vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
        let fake = FakeCli::new();
        fake.register_openspec(
            &["list", "--json"],
            Ok(schema_guard_list_json(&repo, &["alpha", "mike"])),
        );
        fake.register_openspec(
            &["instructions", "apply", "--change", "alpha", "--json"],
            Ok(schema_guard_apply_json(
                "../../../../etc",
                &repo.join("openspec/changes/alpha"),
            )),
        );
        fake.register_openspec(
            &["instructions", "apply", "--change", "mike", "--json"],
            Ok(schema_guard_apply_json(
                "tdd",
                &repo.join("openspec/changes/mike"),
            )),
        );

        let result = from_cli(&fake, &repo);

        let names: Vec<&str> = result.active.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["mike"],
            "alpha must be dropped, mike unaffected"
        );
        assert_eq!(result.problems.len(), 1);
        assert!(result.problems[0].contains("alpha"));
        assert!(result.problems[0].contains("../../../../etc"));

        assert!(
            !fake
                .calls()
                .iter()
                .any(|(_, args)| args.first().map(String::as_str) == Some("schema")),
            "a rejected schemaName must never reach `schema which`: {:?}",
            fake.calls()
        );
        assert!(
            result.active.iter().all(|c| c.schema != "../../../../etc"),
            "the rejected string must reach no produced Change::schema"
        );
    }

    #[test]
    fn every_other_shape_is_legal_name_rejects_is_refused_the_same_way() {
        use crate::cli::FakeCli;

        for value in ["   ", ".", "..", "a/b", "a\\b", "/etc/passwd"] {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(schema_guard_list_json(&repo, &["alpha"])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(schema_guard_apply_json(
                    value,
                    &repo.join("openspec/changes/alpha"),
                )),
            );

            let result = from_cli(&fake, &repo);
            assert!(
                result.active.is_empty(),
                "schemaName {value:?} must drop the change"
            );
            assert_eq!(result.problems.len(), 1, "schemaName {value:?}");
            assert!(result.problems[0].contains("alpha"), "schemaName {value:?}");
            assert!(
                result.problems[0].contains(value),
                "schemaName {value:?}: problem was {:?}",
                result.problems[0]
            );
        }

        for value in ["spec-driven", "spec-driven.v2"] {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, value, &[("proposal", "proposal.md")]);
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(schema_guard_list_json(&repo, &["alpha"])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(schema_guard_apply_json(
                    value,
                    &repo.join("openspec/changes/alpha"),
                )),
            );

            let result = from_cli(&fake, &repo);
            let names: Vec<&str> = result.active.iter().map(|c| c.name.as_str()).collect();
            assert_eq!(
                names,
                vec!["alpha"],
                "schemaName {value:?} must be accepted"
            );
        }

        // The trimmed `" tdd "` is accepted, and the produced `Change`
        // carries the trimmed `tdd`, never the padded value.
        {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(schema_guard_list_json(&repo, &["alpha"])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(schema_guard_apply_json(
                    " tdd ",
                    &repo.join("openspec/changes/alpha"),
                )),
            );

            let result = from_cli(&fake, &repo);
            assert_eq!(result.active.len(), 1);
            assert_eq!(
                result.active[0].schema, "tdd",
                "the trimmed value must be stored, never the padded one"
            );
        }

        // `""` also drops the change, but through the existing
        // missing-field branch rather than the guard — asserted by message,
        // not by outcome, since both outcomes drop the change.
        {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(schema_guard_list_json(&repo, &["alpha"])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(schema_guard_apply_json(
                    "",
                    &repo.join("openspec/changes/alpha"),
                )),
            );

            let result = from_cli(&fake, &repo);
            assert!(result.active.is_empty());
            assert_eq!(result.problems.len(), 1);
            assert!(
                result.problems[0].contains("no usable \"schemaName\""),
                "empty schemaName must take the existing missing-field branch, not the guard: {:?}",
                result.problems[0]
            );
        }
    }

    // --- live-refresh group 3: per-change CLI invalidation ------------------

    mod from_cli_cached {
        use super::super::{CliCache, Selection, from_cli, from_cli_cached};
        use super::*;
        use crate::cli::{CliError, FakeCli};
        use crate::testutil::snapshot;

        fn list_json(root: &std::path::Path, entries: &[(&str, usize, usize)]) -> String {
            let changes: Vec<String> = entries
                .iter()
                .copied()
                .map(|(name, completed, total)| {
                    format!(
                        r#"{{"name":{name:?},"completedTasks":{completed},"totalTasks":{total},"lastModified":"x","status":"y"}}"#
                    )
                })
                .collect();
            format!(
                r#"{{"changes":[{}],"root":{{"path":{:?},"source":"nearest"}}}}"#,
                changes.join(","),
                root.display().to_string()
            )
        }

        fn apply_json(
            schema_name: &str,
            change_dir: &std::path::Path,
            context_files: &[(&str, &[&str])],
        ) -> String {
            let entries: Vec<String> = context_files
                .iter()
                .copied()
                .map(|(id, paths)| {
                    let paths: Vec<String> = paths.iter().map(|p| format!("{p:?}")).collect();
                    format!("{id:?}:[{}]", paths.join(","))
                })
                .collect();
            format!(
                r#"{{"schemaName":{schema_name:?},"changeDir":{:?},"contextFiles":{{{}}}}}"#,
                change_dir.display().to_string(),
                entries.join(",")
            )
        }

        fn failed(args: &[&str]) -> CliError {
            CliError::Failed {
                program: "openspec".to_string(),
                args: args.iter().map(|a| a.to_string()).collect(),
                code: Some(1),
                stderr: String::new(),
            }
        }

        fn only(names: &[&str]) -> Selection {
            Selection::Only(names.iter().map(|s| s.to_string()).collect())
        }

        /// The calls recorded strictly after `before`, so an assertion can
        /// scope to exactly one `from_cli_cached` invocation rather than
        /// the cumulative history every fake accumulates.
        fn calls_since(fake: &FakeCli, before: usize) -> Vec<(crate::cli::Program, Vec<String>)> {
            fake.calls()[before..].to_vec()
        }

        #[test]
        fn selection_union_all_absorbs_only() {
            assert_eq!(Selection::All.union(only(&["a"])), Selection::All);
            assert_eq!(only(&["a"]).union(Selection::All), Selection::All);
        }

        #[test]
        fn selection_union_of_two_onlys() {
            assert_eq!(only(&["a"]).union(only(&["b"])), only(&["a", "b"]));
            assert_eq!(
                Selection::Only(std::collections::BTreeSet::new())
                    .union(Selection::Only(std::collections::BTreeSet::new())),
                Selection::Only(std::collections::BTreeSet::new()),
                "an empty batch does not escalate to All"
            );
        }

        #[test]
        fn only_reruns_the_named_change() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 1, 2), ("beta", 0, 0)])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/alpha"), &[])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "beta", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/beta"), &[])),
            );

            let mut cache = CliCache::default();
            let first = from_cli_cached(&fake, &repo, &Selection::All, &mut cache);
            let before = fake.calls().len();

            let second = from_cli_cached(&fake, &repo, &only(&["alpha"]), &mut cache);

            assert_eq!(
                calls_since(&fake, before),
                vec![
                    (
                        crate::cli::Program::Openspec,
                        vec!["list".to_string(), "--json".to_string()]
                    ),
                    (
                        crate::cli::Program::Openspec,
                        vec![
                            "instructions".to_string(),
                            "apply".to_string(),
                            "--change".to_string(),
                            "alpha".to_string(),
                            "--json".to_string(),
                        ]
                    ),
                ],
                "beta must not be re-asked about"
            );
            let first_beta = first.active.iter().find(|c| c.name == "beta").unwrap();
            let second_beta = second.active.iter().find(|c| c.name == "beta").unwrap();
            assert_eq!(first_beta, second_beta, "byte-identical, reused from cache");
        }

        #[test]
        fn only_still_runs_an_uncached_change() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 1, 2), ("beta", 0, 0)])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/alpha"), &[])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "beta", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/beta"), &[])),
            );

            let mut cache = CliCache::default();
            let result = from_cli_cached(&fake, &repo, &only(&["alpha"]), &mut cache);

            assert_eq!(result.active.len(), 2, "a cold cache degrades no change");
            let apply_names: Vec<String> = fake
                .calls()
                .iter()
                .filter(|(_, args)| args.first().map(String::as_str) == Some("instructions"))
                .map(|(_, args)| args[3].clone())
                .collect();
            assert_eq!(apply_names, vec!["alpha".to_string(), "beta".to_string()]);
        }

        #[test]
        fn cached_progress_comes_from_the_fresh_list() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let fake = FakeCli::new();
            // Both `list` responses are registered up front, in the order
            // the two calls below consume them: `FakeCli`'s queue only pops
            // once a SECOND registration exists — a single registration
            // just repeats, so registering the second reply *between* the
            // two calls would make the second call consume the FIRST
            // (stale) reply rather than the fresh one.
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 4, 9), ("beta", 0, 0)])),
            );
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 7, 9), ("beta", 0, 0)])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/alpha"), &[])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "beta", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/beta"), &[])),
            );

            let mut cache = CliCache::default();
            let _first = from_cli_cached(&fake, &repo, &Selection::All, &mut cache);
            let second = from_cli_cached(&fake, &repo, &only(&["beta"]), &mut cache);

            let alpha = second.active.iter().find(|c| c.name == "alpha").unwrap();
            assert_eq!(
                alpha.progress,
                crate::tasks::Progress {
                    completed: 7,
                    total: 9
                }
            );
        }

        #[test]
        fn an_unlisted_cached_change_is_evicted() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let fake = FakeCli::new();
            // All three `list` replies registered up front, in call order —
            // see `cached_progress_comes_from_the_fresh_list` for why.
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 1, 2), ("beta", 0, 0)])),
            );
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 1, 2)])),
            );
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 1, 2), ("beta", 0, 0)])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/alpha"), &[])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "beta", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/beta"), &[])),
            );

            let mut cache = CliCache::default();
            let _first = from_cli_cached(&fake, &repo, &Selection::All, &mut cache);

            let second = from_cli_cached(&fake, &repo, &Selection::All, &mut cache);
            assert_eq!(second.active.len(), 1);
            assert_eq!(second.active[0].name, "alpha");

            let before = fake.calls().len();
            let third = from_cli_cached(&fake, &repo, &only(&["alpha"]), &mut cache);
            let apply_names: Vec<String> = calls_since(&fake, before)
                .iter()
                .filter(|(_, args)| args.first().map(String::as_str) == Some("instructions"))
                .map(|(_, args)| args[3].clone())
                .collect();
            assert!(
                apply_names.contains(&"beta".to_string()),
                "beta's evicted cache entry must be treated as new: {apply_names:?}"
            );
            assert_eq!(third.active.len(), 2);
        }

        #[test]
        fn cached_change_keeps_its_artifacts_and_schema() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            write(&repo.join("openspec/changes/beta/proposal.md"), "# beta\n");
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 1, 2), ("beta", 0, 0)])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/alpha"), &[])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "beta", "--json"],
                Ok(apply_json(
                    "tdd",
                    &repo.join("openspec/changes/beta"),
                    &[(
                        "proposal",
                        &[repo
                            .join("openspec/changes/beta/proposal.md")
                            .to_str()
                            .unwrap()],
                    )],
                )),
            );

            let mut cache = CliCache::default();
            let first = from_cli_cached(&fake, &repo, &Selection::All, &mut cache);
            let second = from_cli_cached(&fake, &repo, &only(&["alpha"]), &mut cache);

            let first_beta = first.active.iter().find(|c| c.name == "beta").unwrap();
            let second_beta = second.active.iter().find(|c| c.name == "beta").unwrap();
            assert_eq!(first_beta.schema, second_beta.schema);
            assert_eq!(first_beta.artifacts, second_beta.artifacts);
            assert!(!second_beta.artifacts.is_empty());
        }

        #[test]
        fn cached_change_keeps_its_problems() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 1, 2), ("beta", 0, 0)])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/alpha"), &[])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "beta", "--json"],
                Ok(apply_json(
                    "outside-in-tdd",
                    &repo.join("openspec/changes/beta"),
                    &[],
                )),
            );
            fake.register_openspec(
                &["schema", "which", "outside-in-tdd", "--json"],
                Err(failed(&["schema", "which", "outside-in-tdd", "--json"])),
            );

            let mut cache = CliCache::default();
            let first = from_cli_cached(&fake, &repo, &Selection::All, &mut cache);
            let second = from_cli_cached(&fake, &repo, &only(&["alpha"]), &mut cache);

            let first_beta = first.active.iter().find(|c| c.name == "beta").unwrap();
            let second_beta = second.active.iter().find(|c| c.name == "beta").unwrap();
            assert_eq!(first_beta.problems.len(), 1);
            assert_eq!(first_beta.problems, second_beta.problems);
        }

        #[test]
        fn a_list_failure_leaves_the_cache_untouched() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let fake = FakeCli::new();
            // All three `list` replies registered up front, in call order —
            // see `cached_progress_comes_from_the_fresh_list` for why.
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 1, 2)])),
            );
            fake.register_openspec(&["list", "--json"], Err(failed(&["list", "--json"])));
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 1, 2)])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/alpha"), &[])),
            );
            let mut cache = CliCache::default();
            let _first = from_cli_cached(&fake, &repo, &Selection::All, &mut cache);

            let second = from_cli_cached(&fake, &repo, &Selection::All, &mut cache);
            assert!(second.active.is_empty());
            assert_eq!(second.problems.len(), 1);

            let before = fake.calls().len();
            let third = from_cli_cached(
                &fake,
                &repo,
                &Selection::Only(std::collections::BTreeSet::new()),
                &mut cache,
            );
            assert_eq!(
                calls_since(&fake, before).len(),
                1,
                "only the list call — the cache still holds alpha from before the failure"
            );
            assert_eq!(third.active.len(), 1);
        }

        #[test]
        fn garbage_apply_payload_leaves_the_others_intact() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 1, 2), ("beta", 0, 0)])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/alpha"), &[])),
            );
            // Both `beta` apply replies registered up front, in call order —
            // see `cached_progress_comes_from_the_fresh_list` for why.
            fake.register_openspec(
                &["instructions", "apply", "--change", "beta", "--json"],
                Ok("{{{".to_string()),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "beta", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/beta"), &[])),
            );

            let mut cache = CliCache::default();
            let result = from_cli_cached(&fake, &repo, &Selection::All, &mut cache);
            assert!(result.active.iter().any(|c| c.name == "alpha"));
            assert!(!result.active.iter().any(|c| c.name == "beta"));
            assert!(result.problems.iter().any(|p| p.contains("beta")));

            let before = fake.calls().len();
            let second = from_cli_cached(
                &fake,
                &repo,
                &Selection::Only(std::collections::BTreeSet::new()),
                &mut cache,
            );
            let apply_names: Vec<String> = calls_since(&fake, before)
                .iter()
                .filter(|(_, args)| args.first().map(String::as_str) == Some("instructions"))
                .map(|(_, args)| args[3].clone())
                .collect();
            assert!(
                apply_names.contains(&"beta".to_string()),
                "no cache entry, so beta is retried: {apply_names:?}"
            );
            assert_eq!(second.active.len(), 2);
        }

        #[test]
        fn a_rejected_schema_is_cached_like_any_other() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 1, 2), ("beta", 0, 0)])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json(
                    "outside-in-tdd",
                    &repo.join("openspec/changes/alpha"),
                    &[],
                )),
            );
            fake.register_openspec(
                &["schema", "which", "outside-in-tdd", "--json"],
                Err(failed(&["schema", "which", "outside-in-tdd", "--json"])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "beta", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/beta"), &[])),
            );

            let mut cache = CliCache::default();
            let first = from_cli_cached(&fake, &repo, &Selection::All, &mut cache);
            let alpha = first.active.iter().find(|c| c.name == "alpha").unwrap();
            assert!(alpha.artifacts.is_empty());
            assert_eq!(alpha.problems.len(), 1);
            assert!(alpha.problems[0].contains("outside-in-tdd"));

            let before = fake.calls().len();
            let second = from_cli_cached(&fake, &repo, &only(&["beta"]), &mut cache);
            assert_eq!(
                calls_since(&fake, before),
                vec![
                    (
                        crate::cli::Program::Openspec,
                        vec!["list".to_string(), "--json".to_string()]
                    ),
                    (
                        crate::cli::Program::Openspec,
                        vec![
                            "instructions".to_string(),
                            "apply".to_string(),
                            "--change".to_string(),
                            "beta".to_string(),
                            "--json".to_string(),
                        ]
                    ),
                ],
                "no instructions apply and no schema which for alpha"
            );
            let alpha2 = second.active.iter().find(|c| c.name == "alpha").unwrap();
            assert_eq!(alpha2.problems, alpha.problems);
            assert_eq!(
                alpha2.progress,
                crate::tasks::Progress {
                    completed: 1,
                    total: 2
                }
            );

            let before3 = fake.calls().len();
            let third = from_cli_cached(&fake, &repo, &Selection::All, &mut cache);
            assert_eq!(
                calls_since(&fake, before3),
                vec![
                    (
                        crate::cli::Program::Openspec,
                        vec!["list".to_string(), "--json".to_string()]
                    ),
                    (
                        crate::cli::Program::Openspec,
                        vec![
                            "instructions".to_string(),
                            "apply".to_string(),
                            "--change".to_string(),
                            "alpha".to_string(),
                            "--json".to_string(),
                        ]
                    ),
                    (
                        crate::cli::Program::Openspec,
                        vec![
                            "schema".to_string(),
                            "which".to_string(),
                            "outside-in-tdd".to_string(),
                            "--json".to_string(),
                        ]
                    ),
                    (
                        crate::cli::Program::Openspec,
                        vec![
                            "instructions".to_string(),
                            "apply".to_string(),
                            "--change".to_string(),
                            "beta".to_string(),
                            "--json".to_string(),
                        ]
                    ),
                ],
                "Selection::All re-asks about alpha too, schema rejection included — \
                 not merely a cache hit that happens to carry the same problem count"
            );
            let alpha3 = third.active.iter().find(|c| c.name == "alpha").unwrap();
            assert_eq!(
                alpha3.problems.len(),
                1,
                "r is the way out of a stale change"
            );
        }

        // --- degraded-states: group 7 proofs — schema, artifact, and tasks rows -----------

        /// `degraded-coverage` :: "A schema the CLI rejects falls back per change and names
        /// the reason" — SPEC.md row 3. Real `from_files`, a real `FakeCli` driving the real
        /// `from_cli`/`merge`, over a repository holding TWO changes: `learning-tool`, whose
        /// schema neither producer can vendor or resolve, and `sibling`, an ordinary `tdd`
        /// change — proving the fallback is genuinely per-change, not repository-wide.
        #[test]
        fn a_cli_rejected_schema_names_its_reason_per_change() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            write(
                &repo.join("openspec/changes/learning-tool/.openspec.yaml"),
                "schema: outside-in-tdd\n",
            );
            write(
                &repo.join("openspec/changes/learning-tool/proposal.md"),
                "# learning-tool\n",
            );
            write(
                &repo.join("openspec/changes/sibling/.openspec.yaml"),
                "schema: tdd\n",
            );
            write(&repo.join("openspec/changes/sibling/proposal.md"), "# P\n");
            write(&repo.join("openspec/changes/sibling/tasks.md"), "- [x] a\n");

            let files = from_files(&repo, 5);
            let learning_tool_file = files
                .active
                .iter()
                .find(|c| c.name == "learning-tool")
                .expect("learning-tool is read from files");
            assert!(learning_tool_file.artifacts.is_empty());
            assert!(!learning_tool_file.problems.is_empty());

            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(
                    &repo,
                    &[("learning-tool", 0, 0), ("sibling", 1, 1)],
                )),
            );
            fake.register_openspec(
                &[
                    "instructions",
                    "apply",
                    "--change",
                    "learning-tool",
                    "--json",
                ],
                Ok(apply_json(
                    "outside-in-tdd",
                    &repo.join("openspec/changes/learning-tool"),
                    &[],
                )),
            );
            fake.register_openspec(
                &["schema", "which", "outside-in-tdd", "--json"],
                Err(failed(&["schema", "which", "outside-in-tdd", "--json"])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "sibling", "--json"],
                Ok(apply_json(
                    "tdd",
                    &repo.join("openspec/changes/sibling"),
                    &[("proposal", &["proposal.md"])],
                )),
            );

            let cli = from_cli(&fake, &repo);
            let merged = merge(files, cli);

            let learning_tool = merged
                .active
                .iter()
                .find(|c| c.name == "learning-tool")
                .expect("learning-tool survives the merge");
            assert!(
                learning_tool
                    .problems
                    .iter()
                    .any(|p| p.contains("outside-in-tdd")),
                "{:?}",
                learning_tool.problems
            );
            assert!(learning_tool.artifacts.is_empty());

            let sibling = merged
                .active
                .iter()
                .find(|c| c.name == "sibling")
                .expect("sibling survives the merge");
            assert!(
                sibling.problems.is_empty(),
                "per-change: the sibling must be unaffected: {:?}",
                sibling.problems
            );
            assert!(!sibling.artifacts.is_empty());

            // The rendering half: the same problem reaches ui::detail::content_lines' leading
            // "! "-prefixed line for learning-tool, and no such line for sibling — both mandated
            // detail-interior widths.
            for width in [78, 58] {
                let empty_detail = crate::ui::app::Detail {
                    source: String::new(),
                    scroll: 0,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
                };
                let lt_lines =
                    crate::ui::detail::content_lines(&empty_detail, Some(learning_tool), width);
                assert!(
                    lt_lines[0].text().starts_with('!'),
                    "width {width}: {:?}",
                    lt_lines[0].text()
                );
                let sib_lines =
                    crate::ui::detail::content_lines(&empty_detail, Some(sibling), width);
                assert!(
                    !sib_lines.iter().any(|l| l.text().starts_with('!')),
                    "width {width}: sibling must show no problem row"
                );
            }
        }

        #[test]
        fn all_reruns_every_change() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 1, 2), ("beta", 0, 0)])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/alpha"), &[])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "beta", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/beta"), &[])),
            );

            let mut cache = CliCache::default();
            let _first = from_cli_cached(&fake, &repo, &Selection::All, &mut cache);
            let before = fake.calls().len();
            let _second = from_cli_cached(&fake, &repo, &Selection::All, &mut cache);
            let apply_names: Vec<String> = calls_since(&fake, before)
                .iter()
                .filter(|(_, args)| args.first().map(String::as_str) == Some("instructions"))
                .map(|(_, args)| args[3].clone())
                .collect();
            assert_eq!(apply_names, vec!["alpha".to_string(), "beta".to_string()]);
        }

        #[test]
        fn from_cli_is_from_cli_cached_with_all() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 1, 2)])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/alpha"), &[])),
            );
            let result = from_cli(&fake, &repo);

            let fake2 = FakeCli::new();
            fake2.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 1, 2)])),
            );
            fake2.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/alpha"), &[])),
            );
            let mut cache = CliCache::default();
            let expected = from_cli_cached(&fake2, &repo, &Selection::All, &mut cache);

            assert_eq!(result, expected);
        }

        #[test]
        fn from_cli_cached_writes_nothing() {
            let scratch = ScratchDir::new();
            let repo = canonical(scratch.path());
            vendor_schema(&repo, "tdd", TDD_ARTIFACTS);
            write(&repo.join("openspec/changes/alpha/tasks.md"), "- [x] a\n");
            let fake = FakeCli::new();
            fake.register_openspec(
                &["list", "--json"],
                Ok(list_json(&repo, &[("alpha", 1, 2)])),
            );
            fake.register_openspec(
                &["instructions", "apply", "--change", "alpha", "--json"],
                Ok(apply_json("tdd", &repo.join("openspec/changes/alpha"), &[])),
            );

            let before = snapshot(&repo);
            let mut cache = CliCache::default();
            let _first = from_cli_cached(&fake, &repo, &Selection::All, &mut cache);
            let _second = from_cli_cached(
                &fake,
                &repo,
                &Selection::Only(std::collections::BTreeSet::new()),
                &mut cache,
            );
            let after = snapshot(&repo);
            assert_eq!(before, after, "from_cli_cached wrote inside the repository");
        }
    }

    // --- group 9: `merge` — layering CLI over files (`mod merge`) ----------

    mod merge {
        // The module and the function under test share a name; see `mod
        // cli_artifacts` above for why the explicit import comes first.
        use super::super::merge;
        use super::*;

        fn active(
            name: &str,
            schema: &str,
            artifacts: Vec<ArtifactRef>,
            progress: (usize, usize),
        ) -> Change {
            Change {
                name: name.to_string(),
                dir: PathBuf::from(format!("/repo/openspec/changes/{name}")),
                origin: Origin::Active,
                schema: schema.to_string(),
                artifacts,
                progress: crate::tasks::Progress {
                    completed: progress.0,
                    total: progress.1,
                },
                problems: vec![],
            }
        }

        fn archived(name: &str, date: &str) -> Change {
            Change {
                name: name.to_string(),
                dir: PathBuf::from(format!("/repo/openspec/changes/archive/{date}-{name}")),
                origin: Origin::Archived {
                    date: Some(date.to_string()),
                },
                schema: "tdd".to_string(),
                artifacts: vec![],
                progress: crate::tasks::Progress {
                    completed: 0,
                    total: 0,
                },
                problems: vec![],
            }
        }

        fn empty_cli() -> CliChanges {
            CliChanges {
                active: vec![],
                problems: vec![],
            }
        }

        #[test]
        fn the_cli_schema_progress_and_artifacts_replace_the_files() {
            let mut files = empty_set();
            let mut file_alpha = active("alpha", "stale-name", vec![], (2, 3));
            file_alpha.dir = PathBuf::from("/repo/openspec/changes/alpha");
            files.active.push(file_alpha);

            let mut cli = empty_cli();
            let mut cli_alpha = active(
                "alpha",
                "tdd",
                vec![
                    ArtifactRef {
                        id: "proposal".to_string(),
                        paths: vec![],
                        tracks_tasks: false,
                    },
                    ArtifactRef {
                        id: "tasks".to_string(),
                        paths: vec![],
                        tracks_tasks: false,
                    },
                ],
                (4, 9),
            );
            cli_alpha.dir = PathBuf::from("/repo/openspec/changes/alpha");
            cli.active.push(cli_alpha);

            let merged = merge(files, cli);
            let alpha = &merged.active[0];
            assert_eq!(alpha.schema, "tdd");
            let rendered = format!("{alpha:?}");
            assert!(!rendered.contains("stale-name"));
            assert_eq!(alpha.artifacts.len(), 2);
            assert_eq!(
                alpha.progress,
                crate::tasks::Progress {
                    completed: 4,
                    total: 9
                }
            );
        }

        #[test]
        fn the_merged_dir_comes_from_the_file_change() {
            let mut files = empty_set();
            files.active.push(active("alpha", "tdd", vec![], (0, 0)));

            let mut cli = empty_cli();
            let mut cli_alpha = active("alpha", "tdd", vec![], (0, 0));
            cli_alpha.dir = PathBuf::from("/some/other/canonicalized/path/alpha");
            cli.active.push(cli_alpha);

            let merged = merge(files, cli);
            assert_eq!(
                merged.active[0].dir,
                PathBuf::from("/repo/openspec/changes/alpha")
            );
        }

        #[test]
        fn a_change_only_the_cli_reported_is_inserted_in_name_order() {
            let mut files = empty_set();
            files.active.push(active("alpha", "tdd", vec![], (0, 0)));
            files.active.push(active("zulu", "tdd", vec![], (0, 0)));

            let mut cli = empty_cli();
            cli.active.push(active("alpha", "tdd", vec![], (0, 0)));
            cli.active.push(active("mike", "tdd", vec![], (0, 0)));
            cli.active.push(active("zulu", "tdd", vec![], (0, 0)));

            let merged = merge(files, cli);
            let names: Vec<&str> = merged.active.iter().map(|c| c.name.as_str()).collect();
            assert_eq!(names, vec!["alpha", "mike", "zulu"]);
            let mike = merged.active.iter().find(|c| c.name == "mike").unwrap();
            assert!(mike.problems.is_empty());
        }

        #[test]
        fn a_change_only_the_file_producer_saw_survives_the_merge() {
            let mut files = empty_set();
            files.active.push(active("alpha", "tdd", vec![], (0, 0)));
            let newly_created = active("newly-created", "tdd", vec![], (0, 0));
            files.active.push(newly_created.clone());

            let mut cli = empty_cli();
            cli.active.push(active("alpha", "tdd", vec![], (0, 0)));

            let merged = merge(files, cli);
            let found = merged
                .active
                .iter()
                .find(|c| c.name == "newly-created")
                .unwrap();
            assert_eq!(found, &newly_created);
        }

        #[test]
        fn an_empty_cli_result_leaves_the_file_result_intact() {
            let mut files = empty_set();
            files.active.push(active("alpha", "tdd", vec![], (0, 0)));
            files.active.push(active("mike", "tdd", vec![], (0, 0)));
            files.archived.push(archived("old-one", "2026-01-01"));
            files.archived.push(archived("old-two", "2026-01-02"));
            files.archived.push(archived("old-three", "2026-01-03"));

            let mut cli = empty_cli();
            cli.problems
                .push("could not start openspec: list --json".to_string());

            let merged = merge(files.clone(), cli.clone());
            assert_eq!(merged.active, files.active);
            assert_eq!(merged.archived, files.archived);
            let mut expected_problems = files.problems.clone();
            expected_problems.extend(cli.problems.clone());
            assert_eq!(merged.problems, expected_problems);
        }

        #[test]
        fn archived_changes_pass_through_untouched() {
            let mut files = empty_set();
            files.archived.push(archived("add-auth", "2026-08-14"));

            let mut cli = empty_cli();
            cli.active.push(active("add-auth", "tdd", vec![], (0, 0)));

            let merged = merge(files.clone(), cli);
            assert_eq!(merged.archived, files.archived);
            assert!(
                merged
                    .active
                    .iter()
                    .any(|c| c.name == "add-auth" && c.origin == Origin::Active)
            );
        }

        #[test]
        fn a_file_side_message_survives_beside_a_corrected_artifact_list() {
            let mut files = empty_set();
            let mut file_alpha = active("alpha", "tdd", vec![], (0, 0));
            file_alpha.problems = vec!["x is not vendored: no schema.yaml there".to_string()];
            files.active.push(file_alpha);

            let mut cli = empty_cli();
            let cli_alpha = active(
                "alpha",
                "tdd",
                vec![
                    ArtifactRef {
                        id: "a".to_string(),
                        paths: vec![],
                        tracks_tasks: false,
                    },
                    ArtifactRef {
                        id: "b".to_string(),
                        paths: vec![],
                        tracks_tasks: false,
                    },
                    ArtifactRef {
                        id: "c".to_string(),
                        paths: vec![],
                        tracks_tasks: false,
                    },
                    ArtifactRef {
                        id: "d".to_string(),
                        paths: vec![],
                        tracks_tasks: false,
                    },
                    ArtifactRef {
                        id: "e".to_string(),
                        paths: vec![],
                        tracks_tasks: false,
                    },
                ],
                (0, 0),
            );
            cli.active.push(cli_alpha);

            let merged = merge(files, cli);
            let alpha = &merged.active[0];
            assert_eq!(alpha.artifacts.len(), 5);
            assert_eq!(
                alpha.problems,
                vec!["x is not vendored: no schema.yaml there".to_string()]
            );
        }

        #[test]
        fn duplicate_messages_from_both_producers_are_collapsed() {
            let mut files = empty_set();
            let mut file_alpha = active("alpha", "tdd", vec![], (0, 0));
            file_alpha.problems = vec![
                "a different message".to_string(),
                "shared duplicate message".to_string(),
            ];
            files.active.push(file_alpha);

            let mut cli = empty_cli();
            let mut cli_alpha = active("alpha", "tdd", vec![], (0, 0));
            cli_alpha.problems = vec!["shared duplicate message".to_string()];
            cli.active.push(cli_alpha);

            let merged = merge(files, cli);
            assert_eq!(
                merged.active[0].problems,
                vec![
                    "a different message".to_string(),
                    "shared duplicate message".to_string(),
                ]
            );
        }

        #[test]
        fn a_join_problem_is_appended_after_both_producers_problems() {
            let mut files = empty_set();
            let mut file_alpha = active(
                "alpha",
                "tdd",
                vec![
                    ArtifactRef {
                        id: "a".to_string(),
                        paths: vec![],
                        tracks_tasks: false,
                    },
                    ArtifactRef {
                        id: "b".to_string(),
                        paths: vec![],
                        tracks_tasks: false,
                    },
                ],
                (0, 0),
            );
            file_alpha.problems = vec!["file problem".to_string()];
            files.active.push(file_alpha);

            let mut cli = empty_cli();
            let mut cli_alpha = active(
                "alpha",
                "tdd",
                vec![
                    ArtifactRef {
                        id: "a".to_string(),
                        paths: vec![],
                        tracks_tasks: false,
                    },
                    ArtifactRef {
                        id: "b".to_string(),
                        paths: vec![],
                        tracks_tasks: false,
                    },
                    ArtifactRef {
                        id: "c".to_string(),
                        paths: vec![],
                        tracks_tasks: false,
                    },
                ],
                (0, 0),
            );
            cli_alpha.problems = vec!["cli problem".to_string()];
            cli.active.push(cli_alpha);

            let merged = merge(files, cli);
            let problems = &merged.active[0].problems;
            assert_eq!(problems.len(), 3);
            assert_eq!(problems[0], "file problem");
            assert_eq!(problems[1], "cli problem");
            assert!(problems[2].contains('2'));
            assert!(problems[2].contains('3'));
        }

        #[test]
        fn every_merged_value_satisfies_the_shared_invariants() {
            let mut files = empty_set();
            files.active.push(active("alpha", "tdd", vec![], (1, 2)));
            files.active.push(active("beta", "tdd", vec![], (0, 0)));
            files.archived.push(archived("old-one", "2026-01-01"));
            files.archived.push(archived("old-two", "2026-01-02"));
            files.archived.push(archived("old-three", "2026-01-03"));

            let mut cli = empty_cli();
            cli.active.push(active("alpha", "tdd", vec![], (3, 4)));
            cli.active.push(active("beta", "tdd", vec![], (0, 0)));
            cli.active.push(active("gamma", "tdd", vec![], (0, 0)));

            let merged = merge(files, cli);
            // 3 merged active names (alpha, beta, gamma) + 3 archived.
            assert_eq!(merged.active.len() + merged.archived.len(), 6);
            for change in merged.active.iter().chain(merged.archived.iter()) {
                assert_invariants(change);
            }
        }
    }
}
