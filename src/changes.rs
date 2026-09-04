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
#[allow(clippy::too_many_arguments)]
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
            .get("name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
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
                    },
                    ArtifactRef {
                        id: "specs".to_string(),
                        paths: vec![],
                    },
                    ArtifactRef {
                        id: "design".to_string(),
                        paths: vec![repo.join("openspec/changes/add-auth/design.md")],
                    },
                    ArtifactRef {
                        id: "tasks".to_string(),
                        paths: vec![repo.join("openspec/changes/add-auth/tasks.md")],
                    },
                    ArtifactRef {
                        id: "planning-review".to_string(),
                        paths: vec![],
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
}
