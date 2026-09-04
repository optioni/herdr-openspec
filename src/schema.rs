//! Schema resolution: which OpenSpec schema applies, and the ordered artifact
//! list it names.
//!
//! See `openspec/changes/schema-model/design.md` for the full contract.

use std::path::{Path, PathBuf};

use yaml_rust2::{Yaml, YamlLoader};

/// The schema assumed when nothing declares one. The OpenSpec CLI's own default.
pub const DEFAULT_SCHEMA: &str = "spec-driven";

/// Which of the three sources answered. Two sources routinely declare the
/// same name, so a name alone cannot prove the ordering — this is the same
/// argument `resolve::BinSource` makes, for the same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameSource {
    Change,
    Project,
    Default,
}

/// The resolved schema name, which of the three sources answered, and every
/// fallback recorded along the way, in source order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub name: String,
    pub source: NameSource,
    pub problems: Vec<String>,
}

/// What one configuration file contributed. Produced by the filesystem edge
/// (group 3's `read_file`) so that the selection ordering below is pure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileText {
    /// Does not exist. Contributes nothing and records no problem.
    Absent,
    /// Read successfully.
    Read(String),
    /// Exists but could not be read; carries the I/O message.
    Unreadable(String),
}

/// `Ok(Some(name))` when the first document declares a non-blank string
/// `schema:`.
///
/// `Ok(None)` — the normal, silent case — when the text holds no document at
/// all, **or** when its first document is a mapping carrying no `schema:`
/// key. That second branch is the ordinary shape of a repository that pins
/// nothing, and it is the one an implementation gets wrong by accident:
/// `yaml-rust2` yields `Yaml::BadValue` for a missing key, whose `as_str()`
/// is `None` — indistinguishable from `schema: [a]` or `schema: 42` through a
/// string accessor. The wrong-type branch is therefore keyed on *the node
/// exists and is not a `Yaml::String`* (`!is_badvalue()`), never on
/// `as_str().is_none()`.
///
/// `Err(reason)` when the text is not valid YAML, when its first document is
/// not a mapping, or when `schema:` is present with a non-string value —
/// including an explicit `null`.
/// Parse `text` as YAML and return its first document, or `None` when the
/// text holds no document at all — an empty or comment-only input, which
/// `yaml-rust2` reports as zero documents rather than an error. Shared by
/// `schema_key` and group 4's `parse`: both need "first document or
/// nothing, never a panic on an empty result."
fn first_document(text: &str) -> Result<Option<Yaml>, String> {
    let docs = YamlLoader::load_from_str(text).map_err(|e| e.to_string())?;
    Ok(docs.into_iter().next())
}

pub(crate) fn schema_key(text: &str) -> Result<Option<String>, String> {
    let Some(doc) = first_document(text)? else {
        return Ok(None);
    };
    if !doc.is_hash() {
        return Err("the document is not a mapping".to_string());
    }
    let value = &doc["schema"];
    if value.is_badvalue() {
        return Ok(None);
    }
    match value.as_str() {
        Some(s) => match crate::config::non_blank(Some(s.to_string())) {
            // Trimmed, per tasks.md 2.8 — `is_legal_name` also trims before
            // validating, so accept and use must agree on the trimmed form,
            // never a padded one that would join into a schema path with
            // leading or trailing whitespace in a path segment.
            Some(s) => Ok(Some(s.trim().to_string())),
            None => Err("schema is blank".to_string()),
        },
        None => Err("schema is not a string".to_string()),
    }
}

/// A single, non-escaping path segment: non-blank, no separator, not `.` or
/// `..`, not absolute, no NUL. The separator set is `/` **and** `\` — a
/// backslash is a legal filename character on both supported platforms, so
/// rejecting it is a choice made to mirror the CLI, which splits a schema
/// name on `[\\/]+`.
pub(crate) fn is_legal_name(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." {
        return false;
    }
    if trimmed.contains(['/', '\\', '\0']) {
        return false;
    }
    !Path::new(trimmed).is_absolute()
}

/// One source's contribution to `declared_name`: `None` when it does not win
/// (absent, unreadable, undeclared, invalid, or illegal), each non-`Absent`
/// failure recording exactly one problem naming `path`.
fn source_contribution(path: &Path, text: &FileText, problems: &mut Vec<String>) -> Option<String> {
    match text {
        FileText::Absent => None,
        FileText::Unreadable(reason) => {
            problems.push(format!("{} could not be read: {reason}", path.display()));
            None
        }
        FileText::Read(contents) => match schema_key(contents) {
            Ok(None) => None,
            Ok(Some(name)) => {
                if is_legal_name(&name) {
                    Some(name)
                } else {
                    problems.push(format!(
                        "{} declares an illegal schema name: {name:?}",
                        path.display()
                    ));
                    None
                }
            }
            Err(reason) => {
                problems.push(format!("{}: {reason}", path.display()));
                None
            }
        },
    }
}

/// The three-source ordering, pure. `change` is `None` when no change
/// directory was supplied; `project` is always a path, possibly `Absent`.
/// Every source in turn either wins (returns immediately) or contributes a
/// problem and falls through — a source never wins on a fallback and never
/// stops the search on one.
pub(crate) fn declared_name(
    change: Option<(&Path, &FileText)>,
    project: (&Path, &FileText),
) -> Selection {
    let mut problems = Vec::new();

    if let Some((path, text)) = change
        && let Some(name) = source_contribution(path, text, &mut problems)
    {
        return Selection {
            name,
            source: NameSource::Change,
            problems,
        };
    }

    let (path, text) = project;
    if let Some(name) = source_contribution(path, text, &mut problems) {
        return Selection {
            name,
            source: NameSource::Project,
            problems,
        };
    }

    Selection {
        name: DEFAULT_SCHEMA.to_string(),
        source: NameSource::Default,
        problems,
    }
}

/// Read a path into a `FileText`, mapping `NotFound` to `Absent` and every
/// other failure — including a permission error or a directory where a file
/// was expected — to `Unreadable`.
///
/// `pub(crate)`, like `FileText` and `declared_name`: `changes-from-files`
/// lives in this crate and is the caller that wants to read
/// `openspec/config.yaml` once and hand the same `FileText` to
/// `declared_name` for every change, rather than re-reading it once per
/// change through `select`.
pub(crate) fn read_file(path: &Path) -> FileText {
    match std::fs::read_to_string(path) {
        Ok(text) => FileText::Read(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => FileText::Absent,
        Err(e) => FileText::Unreadable(e.to_string()),
    }
}

/// Which schema applies to `repo`, or to `change_dir` inside it. Reads
/// `<change_dir>/.openspec.yaml` only when `change_dir` is `Some`, and
/// `<repo>/openspec/config.yaml` always, and hands both to `declared_name`.
pub fn select(repo: &Path, change_dir: Option<&Path>) -> Selection {
    let project_path = repo.join("openspec").join("config.yaml");
    let project_text = read_file(&project_path);

    match change_dir {
        Some(dir) => {
            let change_path = dir.join(".openspec.yaml");
            let change_text = read_file(&change_path);
            declared_name(
                Some((change_path.as_path(), &change_text)),
                (project_path.as_path(), &project_text),
            )
        }
        None => declared_name(None, (project_path.as_path(), &project_text)),
    }
}

/// One artifact declared by a schema: its id and the (verbatim, unresolved)
/// `generates` value that names the file or glob it produces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    pub id: String,
    pub generates: String,
}

/// A loaded schema: the name that located it, the ordered artifact list —
/// the tab order — and the artifact holding the task checklist, when the
/// schema has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    pub name: String,
    pub artifacts: Vec<Artifact>,
    pub tasks: Option<Artifact>,
}

/// A schema that parsed, plus one problem per skipped entry or other
/// non-fatal defect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSchema {
    pub schema: Schema,
    pub problems: Vec<String>,
}

/// Parse one `schema.yaml` document. `Err` is the reason it is not a usable
/// schema at all; `Ok` carries the schema plus one problem per skipped
/// entry. Group 4's GREEN reads no key but the top-level `artifacts:` and
/// `name:`; group 5 adds the `apply:`-driven `Schema::tasks` rule.
pub fn parse(name: &str, text: &str) -> Result<ParsedSchema, String> {
    let doc = first_document(text)?.ok_or_else(|| "the document is empty".to_string())?;
    if !doc.is_hash() {
        return Err("the document is not a mapping".to_string());
    }

    let sequence = doc["artifacts"]
        .as_vec()
        .ok_or_else(|| "artifacts is absent or not a sequence".to_string())?;
    if sequence.is_empty() {
        return Err("artifacts is an empty sequence".to_string());
    }

    let mut artifacts = Vec::new();
    let mut problems = Vec::new();
    for (position, entry) in sequence.iter().enumerate() {
        match artifact_from(entry, position) {
            Ok(artifact) => artifacts.push(artifact),
            Err(reason) => problems.push(reason),
        }
    }
    if artifacts.is_empty() {
        return Err("every artifact entry is unusable".to_string());
    }

    let (tasks, tasks_problem) = tasks_artifact(&doc["apply"], &artifacts);
    if let Some(problem) = tasks_problem {
        problems.push(problem);
    }

    if let Some(declared) = non_blank_str(&doc["name"])
        && declared != name
    {
        problems.push(format!(
            "the schema directory {name:?} does not match its file's declared name {declared:?}"
        ));
    }

    Ok(ParsedSchema {
        schema: Schema {
            name: name.to_string(),
            artifacts,
            tasks,
        },
        problems,
    })
}

/// The tasks-artifact rule the OpenSpec CLI's own `findTrackedTasksArtifact`
/// applies: when `apply.tracks` is a non-null string, the artifact whose
/// `generates` equals it exactly, with no id fallback on a miss; otherwise —
/// `apply` absent, not a mapping, or `tracks` absent, null, or the wrong
/// type — the artifact whose id is `tasks`. At most one problem: a `tracks`
/// string that matched nothing, a `tracks` of the wrong type, or neither
/// rule finding an artifact.
fn tasks_artifact(apply: &Yaml, artifacts: &[Artifact]) -> (Option<Artifact>, Option<String>) {
    if apply.is_hash() {
        let tracks = &apply["tracks"];
        if !tracks.is_badvalue() && !tracks.is_null() {
            return match tracks.as_str() {
                Some(value) => match artifacts.iter().find(|a| a.generates == value) {
                    Some(artifact) => (Some(artifact.clone()), None),
                    None => (
                        None,
                        Some(format!(
                            "apply.tracks names {value:?}, which no artifact generates"
                        )),
                    ),
                },
                None => id_fallback(artifacts, Some("apply.tracks is not a string")),
            };
        }
    }
    id_fallback(artifacts, None)
}

/// The id-`tasks` fallback shared by every branch of `tasks_artifact` that
/// does not have a matched `tracks` value. `extra` is the problem to record
/// alongside a hit (a wrong-typed `tracks`, say); on a miss it is folded
/// into the "no tasks artifact" message so at most one problem is ever
/// produced.
fn id_fallback(artifacts: &[Artifact], extra: Option<&str>) -> (Option<Artifact>, Option<String>) {
    match artifacts.iter().find(|a| a.id == "tasks") {
        Some(artifact) => (Some(artifact.clone()), extra.map(str::to_string)),
        None => {
            let reason = match extra {
                Some(extra) => {
                    format!("no tasks artifact: {extra}, and no artifact has id \"tasks\"")
                }
                None => "no tasks artifact: no apply.tracks value and no artifact has id \"tasks\""
                    .to_string(),
            };
            (None, Some(reason))
        }
    }
}

/// A non-blank string read from a YAML node, applying `config::non_blank`'s
/// rule. `None` for a missing key (`Yaml::BadValue`), a non-string value, or
/// a string that is empty or whitespace-only.
fn non_blank_str(node: &Yaml) -> Option<String> {
    node.as_str()
        .and_then(|s| crate::config::non_blank(Some(s.to_string())))
}

/// A relative path with no `..` segment and no NUL byte — the constraint the
/// OpenSpec CLI applies to `generates`, applied here because
/// `changes-from-files` joins the value onto a change directory.
fn is_safe_relative_path(value: &str) -> bool {
    if value.contains('\0') {
        return false;
    }
    let path = Path::new(value);
    if path.is_absolute() {
        return false;
    }
    !path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
}

/// One `artifacts:` sequence entry, or the reason it is unusable, naming its
/// zero-based `position` in the message — kept out of `Artifact` itself,
/// which exists only for the problem text.
fn artifact_from(node: &Yaml, position: usize) -> Result<Artifact, String> {
    if !node.is_hash() {
        return Err(format!("artifact at position {position} is not a mapping"));
    }
    let id = non_blank_str(&node["id"])
        .ok_or_else(|| format!("artifact at position {position} has no usable id"))?;
    let generates = non_blank_str(&node["generates"])
        .ok_or_else(|| format!("artifact at position {position} has no usable generates"))?;
    if !is_safe_relative_path(&generates) {
        return Err(format!(
            "artifact at position {position} has a generates value that escapes the change directory: {generates:?}"
        ));
    }
    Ok(Artifact { id, generates })
}

/// Why a named schema did not load. Three variants rather than one message,
/// because `changes-from-cli` must branch on `NotVendored` specifically, and
/// a consumer matching substrings in a message is a consumer that breaks
/// when the message is reworded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// No `schema.yaml` under `openspec/schemas/<name>/`. Normal for a
    /// schema the CLI ships rather than the repository.
    NotVendored { path: PathBuf },
    /// The path exists but its bytes could not be obtained.
    Unreadable { path: PathBuf, reason: String },
    /// The bytes were read and are not a usable schema.
    Invalid { path: PathBuf, reason: String },
}

/// The composition, for the common case: which name applies, which source
/// answered, and either the loaded schema or the reason it did not, plus
/// every problem recorded selecting and loading it, selection's first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaResolution {
    pub name: String,
    pub source: NameSource,
    pub schema: Result<Schema, LoadError>,
    pub problems: Vec<String>,
}

/// Read and parse `<dir>/schema.yaml`. `dir` is *any* schema directory: the
/// repository's own, `$XDG_DATA_HOME/openspec/schemas/<name>`, or the
/// absolute path `openspec schema which <name> --json` reports for a schema
/// the CLI package ships. `name` is only what the result is labelled with
/// and what the file's `name:` key is compared against.
pub fn load_dir(dir: &Path, name: &str) -> Result<ParsedSchema, LoadError> {
    let path = dir.join("schema.yaml");
    match read_file(&path) {
        FileText::Absent => Err(LoadError::NotVendored { path }),
        FileText::Unreadable(reason) => Err(LoadError::Unreadable { path, reason }),
        FileText::Read(text) => {
            parse(name, &text).map_err(|reason| LoadError::Invalid { path, reason })
        }
    }
}

/// The repository tier: `load_dir(&repo/openspec/schemas/<name>, name)`.
pub fn load(repo: &Path, name: &str) -> Result<ParsedSchema, LoadError> {
    load_dir(&repo.join("openspec").join("schemas").join(name), name)
}

/// The human-readable problem a `LoadError` renders as, for the composition.
fn load_error_problem(err: &LoadError) -> String {
    match err {
        LoadError::NotVendored { path } => {
            format!("{} is not vendored: no schema.yaml there", path.display())
        }
        LoadError::Unreadable { path, reason } => {
            format!("{} could not be read: {reason}", path.display())
        }
        LoadError::Invalid { path, reason } => {
            format!("{} is not a usable schema: {reason}", path.display())
        }
    }
}

/// The composition, for the common case: `select` then `load`, appending the
/// load step's problem — or `parse`'s per-entry problems — after the
/// selection's, never rebuilding the problem list from the load step alone.
pub fn resolve(repo: &Path, change_dir: Option<&Path>) -> SchemaResolution {
    let selection = select(repo, change_dir);
    let mut problems = selection.problems;

    let schema = match load(repo, &selection.name) {
        Ok(parsed) => {
            problems.extend(parsed.problems);
            Ok(parsed.schema)
        }
        Err(err) => {
            problems.push(load_error_problem(&err));
            Err(err)
        }
    };

    SchemaResolution {
        name: selection.name,
        source: selection.source,
        schema,
        problems,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{ScratchDir, snapshot};

    fn read(text: &str) -> FileText {
        FileText::Read(text.to_string())
    }

    #[test]
    fn a_changes_own_declaration_wins_over_the_projects() {
        let change_path = Path::new(".openspec.yaml");
        let change = read("schema: tdd\n");
        let project_path = Path::new("openspec/config.yaml");
        let project = read("schema: spec-driven\n");

        let selection = declared_name(Some((change_path, &change)), (project_path, &project));
        assert_eq!(
            selection,
            Selection {
                name: "tdd".to_string(),
                source: NameSource::Change,
                problems: Vec::new(),
            }
        );

        // Discriminating: dropping the change source yields the project's
        // name, so the assertion is about precedence and not about which
        // file happened to exist.
        let dropped = declared_name(None, (project_path, &project));
        assert_eq!(
            dropped,
            Selection {
                name: "spec-driven".to_string(),
                source: NameSource::Project,
                problems: Vec::new(),
            }
        );
    }

    #[test]
    fn the_project_declaration_answers_when_the_change_declares_nothing() {
        let change_path = Path::new(".openspec.yaml");
        let change = FileText::Absent;
        let project_path = Path::new("openspec/config.yaml");
        let project = read("schema: tdd\n");

        let selection = declared_name(Some((change_path, &change)), (project_path, &project));
        assert_eq!(
            selection,
            Selection {
                name: "tdd".to_string(),
                source: NameSource::Project,
                problems: Vec::new(),
            }
        );
    }

    #[test]
    fn nothing_declared_anywhere_yields_the_default() {
        let change_path = Path::new(".openspec.yaml");
        let change = FileText::Absent;
        let project_path = Path::new("openspec/config.yaml");
        let project = FileText::Absent;

        let selection = declared_name(Some((change_path, &change)), (project_path, &project));
        assert_eq!(
            selection,
            Selection {
                name: DEFAULT_SCHEMA.to_string(),
                source: NameSource::Default,
                problems: Vec::new(),
            }
        );
    }

    #[test]
    fn a_file_that_declares_other_keys_but_no_schema_records_no_problem() {
        assert_eq!(schema_key("context: {}\nrules: []\n"), Ok(None));
        assert_eq!(schema_key("created: today\n"), Ok(None));

        let project_path = Path::new("openspec/config.yaml");
        let project = read("context: {}\nrules: []\n");
        let change_path = Path::new(".openspec.yaml");
        let change = read("created: today\n");

        let selection = declared_name(Some((change_path, &change)), (project_path, &project));
        assert_eq!(selection.source, NameSource::Default);
        assert!(selection.problems.is_empty());
    }

    #[test]
    fn a_fallback_at_both_sources_records_both_problems_in_order() {
        let change_path = Path::new(".openspec.yaml");
        let change = read("schema: \"   \"\n");
        let project_path = Path::new("openspec/config.yaml");
        let project = read("schema:\n  nested: mapping\n");

        let selection = declared_name(Some((change_path, &change)), (project_path, &project));
        assert_eq!(selection.source, NameSource::Default);
        assert_eq!(selection.problems.len(), 2);
        assert!(selection.problems[0].contains(".openspec.yaml"));
        assert!(selection.problems[1].contains("config.yaml"));
    }

    #[test]
    fn a_blank_declaration_is_not_a_declaration() {
        for value in ["\"   \"", "\"\""] {
            let change_path = Path::new(".openspec.yaml");
            let change = read(&format!("schema: {value}\n"));
            let project_path = Path::new("openspec/config.yaml");
            let project = read("schema: tdd\n");

            let selection = declared_name(Some((change_path, &change)), (project_path, &project));
            assert_eq!(selection.name, "tdd", "value {value}");
            assert_eq!(selection.source, NameSource::Project, "value {value}");
            assert_eq!(selection.problems.len(), 1, "value {value}");
            assert!(selection.problems[0].contains(".openspec.yaml"));
        }
    }

    #[test]
    fn a_padded_declaration_is_trimmed_before_use() {
        // Found in Change Review: `is_legal_name` trims before validating,
        // so a padded name was accepted as legal and then used un-trimmed —
        // `schema: " tdd "` joined into `openspec/schemas/ tdd /schema.yaml`,
        // a path `tdd` never occupies. Accept and use must agree.
        assert_eq!(
            schema_key("schema: \" tdd \"\n"),
            Ok(Some("tdd".to_string()))
        );

        let project_path = Path::new("openspec/config.yaml");
        let project = read("schema: \" tdd \"\n");
        let selection = declared_name(None, (project_path, &project));
        assert_eq!(selection.name, "tdd");
        assert!(selection.problems.is_empty());
    }

    #[test]
    fn a_declaration_of_the_wrong_type_is_not_a_declaration() {
        for text in [
            "schema:\n  nested: mapping\n",
            "schema: [a, b]\n",
            "schema: null\n",
            "just a scalar document\n",
        ] {
            let project_path = Path::new("openspec/config.yaml");
            let project = read(text);
            let selection = declared_name(None, (project_path, &project));
            assert_eq!(selection.name, DEFAULT_SCHEMA, "text {text:?}");
            assert_eq!(selection.source, NameSource::Default, "text {text:?}");
            assert_eq!(
                selection.problems.len(),
                1,
                "text {text:?} should record exactly one problem"
            );
        }
    }

    #[test]
    fn a_file_that_is_not_valid_yaml_falls_through_and_is_named() {
        let bad_text = "schema: tdd\nbad:\n\tindented: with a tab\n";
        let parser_err = schema_key(bad_text).expect_err("tab-indented YAML must be invalid");

        let change_path = Path::new(".openspec.yaml");
        let change = read(bad_text);
        let project_path = Path::new("openspec/config.yaml");
        let project = read("schema: tdd\n");

        let selection = declared_name(Some((change_path, &change)), (project_path, &project));
        assert_eq!(selection.name, "tdd");
        assert_eq!(selection.source, NameSource::Project);
        assert_eq!(selection.problems.len(), 1);
        assert!(selection.problems[0].contains(&parser_err));
    }

    #[test]
    fn an_empty_or_comment_only_document_declares_nothing_without_complaint() {
        for text in ["", "# nothing\n"] {
            assert_eq!(schema_key(text), Ok(None), "text {text:?}");
            let project_path = Path::new("openspec/config.yaml");
            let project = read(text);
            let selection = declared_name(None, (project_path, &project));
            assert_eq!(selection.source, NameSource::Default, "text {text:?}");
            assert!(selection.problems.is_empty(), "text {text:?}");
        }
    }

    #[test]
    fn only_the_first_yaml_document_is_consulted() {
        let text = "schema: tdd\n---\nschema: other\n";
        assert_eq!(schema_key(text), Ok(Some("tdd".to_string())));

        let project_path = Path::new("openspec/config.yaml");
        let project = read(text);
        let selection = declared_name(None, (project_path, &project));
        assert_eq!(selection.name, "tdd");
        assert!(selection.problems.is_empty());
    }

    #[test]
    fn a_name_containing_a_path_separator_is_rejected_and_falls_through() {
        let change_path = Path::new(".openspec.yaml");
        let change = read("schema: ../../etc\n");
        let project_path = Path::new("openspec/config.yaml");
        let project = read("schema: tdd\n");

        let selection = declared_name(Some((change_path, &change)), (project_path, &project));
        assert_eq!(selection.name, "tdd");
        assert_eq!(selection.source, NameSource::Project);
        assert_eq!(selection.problems.len(), 1);
        assert!(selection.problems[0].contains("../../etc"));
    }

    #[test]
    fn every_rejected_shape_is_rejected_and_a_legal_name_with_a_dot_is_not() {
        let project_path = Path::new("openspec/config.yaml");

        for bad in ["..", ".", "a/b", "a\\b", "/abs"] {
            let project = read(&format!("schema: {bad}\n"));
            let selection = declared_name(None, (project_path, &project));
            assert_eq!(selection.name, DEFAULT_SCHEMA, "bad name {bad:?}");
            assert_eq!(selection.source, NameSource::Default, "bad name {bad:?}");
            assert_eq!(selection.problems.len(), 1, "bad name {bad:?}");
        }

        // The NUL fixture is the YAML double-quoted escape, never a literal
        // NUL byte in a plain scalar — which the parser silently truncates
        // to the perfectly legal name `a`.
        let nul_project = read("schema: \"a\\0b\"\n");
        let selection = declared_name(None, (project_path, &nul_project));
        assert_eq!(selection.name, DEFAULT_SCHEMA);
        assert_eq!(selection.source, NameSource::Default);
        assert_eq!(selection.problems.len(), 1);

        // Without this positive case, an `is_legal_name` that returns
        // `false` unconditionally passes every clause above.
        let legal_project = read("schema: v1.2-tdd\n");
        let selection = declared_name(None, (project_path, &legal_project));
        assert_eq!(selection.name, "v1.2-tdd");
        assert_eq!(selection.source, NameSource::Project);
        assert!(selection.problems.is_empty());
    }

    #[test]
    fn an_illegal_project_name_still_falls_through_to_the_default() {
        let project_path = Path::new("openspec/config.yaml");
        let project = read("schema: ../escape\n");

        let selection = declared_name(None, (project_path, &project));
        assert_eq!(selection.name, DEFAULT_SCHEMA);
        assert_eq!(selection.source, NameSource::Default);
        assert_eq!(selection.problems.len(), 1);
    }

    // --- group 3: the selection filesystem edge -----------------------

    fn mkdir(path: &Path) {
        std::fs::create_dir_all(path).expect("create fixture directory");
    }

    #[test]
    fn no_change_directory_is_supplied_at_all() {
        let scratch = ScratchDir::new();
        let root = scratch.path();
        let repo = root.join("repo");
        mkdir(&repo.join("openspec"));
        std::fs::write(repo.join("openspec").join("config.yaml"), b"schema: tdd\n")
            .expect("write project config");
        // A third name appearing nowhere else in the fixture, planted at
        // both the path a `change_dir = repo` bug would read and the path a
        // `change_dir = repo.parent()` bug would read.
        std::fs::write(repo.join(".openspec.yaml"), b"schema: planted\n")
            .expect("write repo-root plant");
        std::fs::write(root.join(".openspec.yaml"), b"schema: planted\n")
            .expect("write scratch-root plant");

        let selection = select(&repo, None);
        assert_eq!(
            selection,
            Selection {
                name: "tdd".to_string(),
                source: NameSource::Project,
                problems: Vec::new(),
            }
        );
    }

    #[test]
    fn an_unreadable_file_falls_through_and_is_named() {
        let scratch = ScratchDir::new();
        let repo = scratch.path();
        // `openspec/config.yaml` as a directory: `read_to_string` returns an
        // I/O error rather than `NotFound`, which is what separates
        // `Unreadable` from `Absent`.
        mkdir(&repo.join("openspec").join("config.yaml"));

        let selection = select(repo, None);
        assert_eq!(selection.name, DEFAULT_SCHEMA);
        assert_eq!(selection.source, NameSource::Default);
        assert_eq!(selection.problems.len(), 1);
        assert!(
            selection.problems[0].contains(
                &repo
                    .join("openspec")
                    .join("config.yaml")
                    .display()
                    .to_string()
            )
        );
    }

    #[test]
    fn a_repository_tree_is_byte_identical_after_selection() {
        let scratch = ScratchDir::new();
        let repo = scratch.path();
        mkdir(&repo.join("openspec").join("changes").join("x"));
        std::fs::write(
            repo.join("openspec")
                .join("changes")
                .join("x")
                .join("tasks.md"),
            b"- [ ] 1 do it\n",
        )
        .expect("write tasks.md");
        mkdir(&repo.join("openspec").join("specs"));
        std::fs::write(repo.join("openspec").join("config.yaml"), b"schema: tdd\n")
            .expect("write config.yaml");
        std::fs::write(repo.join("README.md"), b"# Fixture\n").expect("write README.md");

        assert!(
            repo.join("openspec").join("config.yaml").exists(),
            "fixture must exist before the call under test"
        );

        let before = snapshot(repo);
        let _ = select(repo, None);
        let _ = select(repo, None);
        let after = snapshot(repo);
        assert_eq!(before, after);
    }

    #[test]
    fn a_missing_configuration_file_is_not_created() {
        let scratch = ScratchDir::new();
        let repo = scratch.path();
        mkdir(&repo.join("openspec"));
        let change_dir = repo.join("nonexistent-change");

        let selection = select(repo, Some(&change_dir));
        assert!(!repo.join("openspec").join("config.yaml").exists());
        assert!(!change_dir.exists());
        assert_eq!(selection.name, DEFAULT_SCHEMA);
        assert_eq!(selection.source, NameSource::Default);

        // The other half of "Nothing declared anywhere yields the default":
        // a repository root with no `openspec/` directory at all degrades
        // rather than failing. Asserted here, through `select`, rather than
        // in group 2, which is pure and creates no file.
        let bare = scratch.path().join("bare-root");
        mkdir(&bare);
        let bare_selection = select(&bare, None);
        assert_eq!(bare_selection.name, DEFAULT_SCHEMA);
        assert_eq!(bare_selection.source, NameSource::Default);
        assert!(bare_selection.problems.is_empty());
        assert!(!bare.join("openspec").exists());
    }

    // --- group 4: parsing a schema document into artifacts ------------
    //
    // Every test here asserts `schema.name`, `schema.artifacts`, and
    // `problems` only — `Schema::tasks` is group 5's, because 4.9's GREEN
    // deliberately reads no key but `artifacts:` and `name:`.

    fn art(id: &str, generates: &str) -> Artifact {
        Artifact {
            id: id.to_string(),
            generates: generates.to_string(),
        }
    }

    #[test]
    fn artifact_order_is_the_files_order_not_alphabetical() {
        let text = "\
name: test
artifacts:
  - id: zeta
    generates: zeta.md
    requires: [middle]
  - id: alpha
    generates: alpha.md
    requires: [zeta]
  - id: middle
    generates: middle.md
  - id: zeta
    generates: zeta2.md
";
        let parsed = parse("test", text).expect("schema should parse");
        assert_eq!(
            parsed.schema.artifacts,
            vec![
                art("zeta", "zeta.md"),
                art("alpha", "alpha.md"),
                art("middle", "middle.md"),
                art("zeta", "zeta2.md"),
            ]
        );
    }

    #[test]
    fn only_the_first_yaml_document_of_the_schema_file_is_used() {
        let text = "\
name: test
artifacts:
  - id: first
    generates: first.md
---
name: test
artifacts:
  - id: second
    generates: second.md
";
        let parsed = parse("test", text).expect("schema should parse");
        assert_eq!(parsed.schema.artifacts, vec![art("first", "first.md")]);
    }

    #[test]
    fn entries_missing_id_or_generates_are_skipped_and_the_rest_survive() {
        let text = "\
name: test
artifacts:
  - id: proposal
    generates: proposal.md
  - generates: orphan.md
  - id: noname
  - id: tasks
    generates: tasks.md
";
        let parsed = parse("test", text).expect("schema should parse");
        assert_eq!(
            parsed.schema.artifacts,
            vec![art("proposal", "proposal.md"), art("tasks", "tasks.md")]
        );
        assert_eq!(parsed.problems.len(), 2);
        assert!(parsed.problems[0].contains('1'));
        assert!(parsed.problems[1].contains('2'));
        // The spec's own AND clause: one bad neighbour does not cost the
        // tasks tab. Asserted directly rather than only through the problem
        // count, which would be 3 (not 2) if the tasks rule had failed here.
        assert_eq!(parsed.schema.tasks, Some(art("tasks", "tasks.md")));
    }

    #[test]
    fn a_blank_id_or_generates_is_skipped_like_an_absent_one() {
        let text = "\
name: test
artifacts:
  - id: \"  \"
    generates: valid.md
  - id: valid
    generates: \"\"
  - id: tasks
    generates: ok.md
";
        let parsed = parse("test", text).expect("schema should parse");
        // The surviving artifact's id is `tasks`, so it also satisfies the
        // id fallback: this fixture stays free of group 5's separate
        // "no tasks artifact" problem, per task 5.6.
        assert_eq!(parsed.schema.artifacts, vec![art("tasks", "ok.md")]);
        assert_eq!(parsed.problems.len(), 2);
    }

    #[test]
    fn an_entry_that_is_not_a_mapping_is_skipped() {
        let text = "\
name: test
artifacts:
  - \"proposal\"
  - [nested, sequence]
  - id: tasks
    generates: ok.md
";
        let parsed = parse("test", text).expect("schema should parse");
        assert_eq!(parsed.schema.artifacts, vec![art("tasks", "ok.md")]);
        assert_eq!(parsed.problems.len(), 2);
    }

    #[test]
    fn a_generates_that_escapes_the_change_directory_is_skipped() {
        let text = "\
name: test
artifacts:
  - id: a
    generates: ../outside.md
  - id: b
    generates: /etc/passwd
  - id: c
    generates: a/../../b.md
  - id: d
    generates: specs/**/*.md
  - id: tasks
    generates: sub/dir/file.md
";
        let parsed = parse("test", text).expect("schema should parse");
        assert_eq!(
            parsed.schema.artifacts,
            vec![art("d", "specs/**/*.md"), art("tasks", "sub/dir/file.md")]
        );
        assert_eq!(parsed.problems.len(), 3);
    }

    #[test]
    fn unused_keys_being_absent_or_malformed_does_not_make_an_entry_unusable() {
        let text = "\
name: test
artifacts:
  - id: a
    generates: a.md
  - id: tasks
    generates: b.md
    requires: not-a-sequence
";
        let parsed = parse("test", text).expect("schema should parse");
        assert_eq!(
            parsed.schema.artifacts,
            vec![art("a", "a.md"), art("tasks", "b.md")]
        );
        assert!(parsed.problems.is_empty());
    }

    #[test]
    fn a_schema_whose_every_entry_is_unusable_is_invalid_rather_than_empty() {
        let text = "\
name: test
artifacts:
  - generates: a.md
  - generates: b.md
";
        assert!(parse("test", text).is_err());
    }

    #[test]
    fn an_empty_comment_only_or_non_mapping_document_is_invalid() {
        for text in ["", "# nothing\n", "just text\n", "- a\n- b\n"] {
            assert!(parse("test", text).is_err(), "text {text:?}");
        }
    }

    #[test]
    fn an_absent_non_sequence_or_empty_artifacts_key_is_invalid() {
        for text in [
            "name: test\nversion: 1\n",
            "name: test\nversion: 1\nartifacts: nope\n",
            "name: test\nversion: 1\nartifacts: []\n",
        ] {
            assert!(parse("test", text).is_err(), "text {text:?}");
        }
    }

    #[test]
    fn flow_style_anchors_and_crlf_all_parse() {
        // Flow style.
        let flow =
            "name: test\nartifacts: [{id: a, generates: a.md}, {id: tasks, generates: tasks.md}]\n";
        let parsed = parse("test", flow).expect("flow-style schema should parse");
        assert_eq!(
            parsed.schema.artifacts,
            vec![art("a", "a.md"), art("tasks", "tasks.md")]
        );
        // Group 5: no `apply:` block, so the id fallback selects `tasks`.
        assert_eq!(parsed.schema.tasks, Some(art("tasks", "tasks.md")));

        // Anchors and aliases.
        let anchored = "\
name: test
template: &tpl
  id: shared
  generates: shared.md
artifacts:
  - *tpl
";
        let parsed = parse("test", anchored).expect("anchored schema should parse");
        assert_eq!(parsed.schema.artifacts, vec![art("shared", "shared.md")]);

        // CRLF versus LF.
        let lf = "name: test\nartifacts:\n  - id: a\n    generates: a.md\n";
        let crlf = lf.replace('\n', "\r\n");
        let parsed_lf = parse("test", lf).expect("LF schema should parse");
        let parsed_crlf = parse("test", &crlf).expect("CRLF schema should parse");
        assert_eq!(parsed_lf.schema, parsed_crlf.schema);

        // These three are what distinguish a YAML parser from a line
        // scanner: each is legal YAML that an indentation-and-colon scanner
        // reads wrongly or not at all. Kept even though each sub-case is
        // individually simple, so the dependency choice stays load-bearing.
    }

    #[test]
    fn a_schema_file_whose_block_scalars_mimic_structure_still_parses_correctly() {
        // Modelled on the vendored `tdd` schema, whose `apply.instruction`
        // content genuinely sits at four spaces — the same column as
        // `generates:` under an artifact.
        let text = "\
name: test
artifacts:
  - id: real1
    generates: real1.md
  - id: real2
    generates: real2.md
apply:
  tracks: real1.md
  instruction: |
    Do the thing.
    - id: fake
      generates: fake.md
      tracks: fake.md
";
        let parsed = parse("test", text).expect("schema should parse");
        assert_eq!(
            parsed.schema.artifacts,
            vec![art("real1", "real1.md"), art("real2", "real2.md")]
        );
        // Group 5: the tasks artifact comes from the real `apply.tracks:
        // real1.md`, not from the `tracks: fake.md` line inside the block
        // scalar — the half of this scenario group 4 could not express.
        assert_eq!(parsed.schema.tasks, Some(art("real1", "real1.md")));
    }

    // --- group 5: the tasks artifact -----------------------------------

    #[test]
    fn apply_tracks_selects_an_artifact_whose_id_is_not_tasks() {
        let text = "\
name: test
artifacts:
  - id: checklist
    generates: tasks.md
  - id: tasks
    generates: notes.md
apply:
  tracks: tasks.md
";
        let parsed = parse("test", text).expect("schema should parse");
        assert_eq!(parsed.schema.tasks, Some(art("checklist", "tasks.md")));
        assert_ne!(parsed.schema.tasks, Some(art("tasks", "notes.md")));
        assert_eq!(
            parsed.schema.artifacts,
            vec![art("checklist", "tasks.md"), art("tasks", "notes.md")]
        );
    }

    #[test]
    fn an_absent_apply_block_falls_back_to_the_artifact_with_id_tasks() {
        let text = "\
name: test
artifacts:
  - id: proposal
    generates: proposal.md
  - id: tasks
    generates: checklist.md
";
        let parsed = parse("test", text).expect("schema should parse");
        assert_eq!(parsed.schema.tasks, Some(art("tasks", "checklist.md")));
        assert!(parsed.problems.is_empty());
    }

    #[test]
    fn an_apply_block_without_tracks_and_an_explicit_tracks_null_both_fall_back_to_the_id() {
        let absent_key = "\
name: test
artifacts:
  - id: tasks
    generates: checklist.md
apply:
  requires: []
";
        let explicit_null = "\
name: test
artifacts:
  - id: tasks
    generates: checklist.md
apply:
  requires: []
  tracks: null
";
        // `apply:` itself a bare scalar rather than a mapping, which must
        // also fall back to the id without panicking — indexing a scalar
        // node reached from the other direction.
        let apply_not_a_mapping = "\
name: test
artifacts:
  - id: tasks
    generates: checklist.md
apply: not-a-mapping
";
        for text in [absent_key, explicit_null, apply_not_a_mapping] {
            let parsed = parse("test", text).expect("schema should parse");
            assert_eq!(
                parsed.schema.tasks,
                Some(art("tasks", "checklist.md")),
                "text {text:?}"
            );
        }
    }

    #[test]
    fn a_tracks_value_matching_nothing_yields_no_tasks_artifact_even_when_an_id_tasks_exists() {
        let text = "\
name: test
artifacts:
  - id: tasks
    generates: tasks.md
  - id: design
    generates: design.md
apply:
  tracks: nowhere.md
";
        let parsed = parse("test", text).expect("schema should parse");
        assert!(parsed.schema.tasks.is_none());
        assert_ne!(parsed.schema.tasks, Some(art("tasks", "tasks.md")));
        assert_eq!(parsed.problems.len(), 1);
        assert!(parsed.problems[0].contains("nowhere.md"));
    }

    #[test]
    fn a_schema_with_neither_a_tracks_match_nor_an_id_tasks_loads_without_one() {
        let text = "\
name: test
artifacts:
  - id: proposal
    generates: proposal.md
  - id: design
    generates: design.md
";
        let parsed = parse("test", text).expect("schema should still load");
        assert_eq!(
            parsed.schema.artifacts,
            vec![art("proposal", "proposal.md"), art("design", "design.md")]
        );
        assert!(parsed.schema.tasks.is_none());
        assert_eq!(parsed.problems.len(), 1);
    }

    #[test]
    fn tracks_matches_on_generates_not_on_a_filename_suffix() {
        let text = "\
name: test
artifacts:
  - id: a
    generates: sub/tasks.md
  - id: b
    generates: tasks.md
apply:
  tracks: tasks.md
";
        let parsed = parse("test", text).expect("schema should parse");
        assert_eq!(parsed.schema.tasks, Some(art("b", "tasks.md")));
        assert_ne!(parsed.schema.tasks, Some(art("a", "sub/tasks.md")));
    }

    #[test]
    fn a_tracks_value_of_the_wrong_type_falls_back_to_the_id() {
        let text = "\
name: test
artifacts:
  - id: tasks
    generates: checklist.md
apply:
  tracks: [a, b]
";
        let parsed = parse("test", text).expect("schema should parse");
        assert_eq!(parsed.schema.tasks, Some(art("tasks", "checklist.md")));
        assert_eq!(parsed.problems.len(), 1);
        assert!(parsed.problems[0].contains("tracks"));
    }

    // --- group 6: loading from disk, the three reasons, and the composition

    fn write_schema(repo: &Path, name: &str, text: &str) {
        let dir = repo.join("openspec").join("schemas").join(name);
        mkdir(&dir);
        std::fs::write(dir.join("schema.yaml"), text).expect("write schema.yaml");
    }

    fn write_config(repo: &Path, text: &str) {
        mkdir(&repo.join("openspec"));
        std::fs::write(repo.join("openspec").join("config.yaml"), text).expect("write config.yaml");
    }

    #[test]
    fn the_repositorys_own_vendored_tdd_schema_loads() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
        let parsed = load(repo, "tdd").expect("the vendored tdd schema should load");

        let ids: Vec<&str> = parsed
            .schema
            .artifacts
            .iter()
            .map(|a| a.id.as_str())
            .collect();
        assert_eq!(
            ids,
            ["proposal", "specs", "design", "tasks", "planning-review"]
        );
        assert_eq!(parsed.schema.name, "tdd");
        assert_eq!(parsed.schema.tasks, Some(art("tasks", "tasks.md")));
        let specs = parsed
            .schema
            .artifacts
            .iter()
            .find(|a| a.id == "specs")
            .expect("specs artifact should exist");
        assert_eq!(specs.generates, "specs/**/*.md");
        assert!(parsed.problems.is_empty());
    }

    #[test]
    fn a_schema_that_is_not_vendored_is_reported_as_such_and_not_as_broken() {
        let scratch = ScratchDir::new();
        let repo = scratch.path();
        write_schema(
            repo,
            "tdd",
            "name: tdd\nartifacts:\n  - id: a\n    generates: a.md\n",
        );

        let expected_path = repo
            .join("openspec")
            .join("schemas")
            .join("spec-driven")
            .join("schema.yaml");
        match load(repo, "spec-driven") {
            Err(LoadError::NotVendored { path }) => assert_eq!(path, expected_path),
            other => panic!("expected NotVendored, got {other:?}"),
        }

        write_config(repo, "schema: spec-driven\n");
        let resolution = resolve(repo, None);
        assert_eq!(resolution.problems.len(), 1);
        assert!(resolution.problems[0].contains(&expected_path.display().to_string()));

        // A repository with no `openspec/schemas/` directory at all yields
        // the same `NotVendored` reason rather than `Unreadable`.
        let scratch2 = ScratchDir::new();
        let bare = scratch2.path();
        mkdir(bare);
        assert!(matches!(
            load(bare, "anything"),
            Err(LoadError::NotVendored { .. })
        ));
    }

    #[test]
    fn a_schema_directory_with_no_schema_yaml_is_not_vendored() {
        let scratch = ScratchDir::new();
        let repo = scratch.path();
        mkdir(&repo.join("openspec").join("schemas").join("half"));

        assert!(matches!(
            load(repo, "half"),
            Err(LoadError::NotVendored { .. })
        ));
    }

    #[test]
    fn a_schema_yaml_that_cannot_be_read_is_reported_as_unreadable() {
        let scratch = ScratchDir::new();
        let repo = scratch.path();
        // `schema.yaml` as a directory: `read_to_string` fails with an I/O
        // error rather than `NotFound`.
        mkdir(
            &repo
                .join("openspec")
                .join("schemas")
                .join("odd")
                .join("schema.yaml"),
        );

        let expected_path = repo
            .join("openspec")
            .join("schemas")
            .join("odd")
            .join("schema.yaml");
        match load(repo, "odd") {
            Err(LoadError::Unreadable { path, reason }) => {
                assert_eq!(path, expected_path);
                assert!(!reason.is_empty());
            }
            other => panic!("expected Unreadable, got {other:?}"),
        }

        write_config(repo, "schema: odd\n");
        let resolution = resolve(repo, None);
        assert_eq!(resolution.problems.len(), 1);
        assert!(resolution.problems[0].contains(&expected_path.display().to_string()));
    }

    #[test]
    fn a_composed_resolution_carries_the_selections_problems_as_well_as_the_loads() {
        let scratch = ScratchDir::new();
        let repo = scratch.path();
        write_config(repo, "schema: absent\n");
        let change_dir = repo.join("changes").join("x");
        mkdir(&change_dir);
        // Invalid YAML: selection records a problem and falls through to
        // the project's `schema: absent`, which is not vendored.
        std::fs::write(
            change_dir.join(".openspec.yaml"),
            "schema: tdd\nbad:\n\tindented: with a tab\n",
        )
        .expect("write invalid change config");

        let resolution = resolve(repo, Some(&change_dir));
        assert_eq!(resolution.name, "absent");
        assert_eq!(resolution.source, NameSource::Project);
        assert!(matches!(
            resolution.schema,
            Err(LoadError::NotVendored { .. })
        ));
        assert_eq!(resolution.problems.len(), 2);
    }

    #[test]
    fn the_directory_name_and_the_files_name_key_disagreeing_is_recorded() {
        let scratch = ScratchDir::new();
        let repo = scratch.path();
        // `id: tasks` throughout, so the fixture stays free of group 5's
        // separate "no tasks artifact" problem — this scenario is only
        // about the name/directory disagreement.
        write_schema(
            repo,
            "renamed",
            "name: original\nartifacts:\n  - id: tasks\n    generates: a.md\n",
        );

        let parsed = load(repo, "renamed").expect("should load");
        assert_eq!(parsed.schema.name, "renamed");
        assert_eq!(parsed.problems.len(), 1);
        assert!(parsed.problems[0].contains("renamed"));
        assert!(parsed.problems[0].contains("original"));

        // No `name:` key, a blank `name:`, and a mapping-valued `name:` are
        // each loaded with the directory's name and no problem — an absent
        // or unusable `name` is not a disagreement.
        for (dir, text) in [
            (
                "no-name",
                "artifacts:\n  - id: tasks\n    generates: a.md\n",
            ),
            (
                "blank-name",
                "name: \"  \"\nartifacts:\n  - id: tasks\n    generates: a.md\n",
            ),
            (
                "mapping-name",
                "name: {a: b}\nartifacts:\n  - id: tasks\n    generates: a.md\n",
            ),
        ] {
            write_schema(repo, dir, text);
            let parsed = load(repo, dir).unwrap_or_else(|e| panic!("dir {dir} should load: {e:?}"));
            assert_eq!(parsed.schema.name, dir, "dir {dir}");
            assert!(parsed.problems.is_empty(), "dir {dir}");
        }
    }

    #[test]
    fn invalid_yaml_is_reported_with_the_parsers_own_message() {
        let scratch = ScratchDir::new();
        let repo = scratch.path();
        write_schema(repo, "bad", "name: bad\nartifacts:\n\tbad: indentation\n");

        let expected_path = repo
            .join("openspec")
            .join("schemas")
            .join("bad")
            .join("schema.yaml");
        match load(repo, "bad") {
            Err(LoadError::Invalid { path, reason }) => {
                assert_eq!(path, expected_path);
                assert!(!reason.is_empty());
            }
            other => panic!("expected Invalid, got {other:?}"),
        }

        write_config(repo, "schema: bad\n");
        let resolution = resolve(repo, None);
        assert_eq!(resolution.problems.len(), 1);
    }

    #[test]
    fn a_repository_tree_is_byte_identical_after_loading() {
        let scratch = ScratchDir::new();
        let repo = scratch.path();
        write_schema(
            repo,
            "tdd",
            "name: tdd\nartifacts:\n  - id: a\n    generates: a.md\n",
        );
        write_config(repo, "schema: tdd\n");
        mkdir(&repo.join("openspec").join("changes").join("x"));
        std::fs::write(
            repo.join("openspec")
                .join("changes")
                .join("x")
                .join("tasks.md"),
            b"- [ ] 1 do it\n",
        )
        .expect("write tasks.md");
        mkdir(&repo.join("openspec").join("specs"));
        std::fs::write(repo.join("README.md"), b"# Fixture\n").expect("write README.md");

        let before = snapshot(repo);
        let _ = resolve(repo, None);
        let _ = resolve(repo, None);
        let after = snapshot(repo);
        assert_eq!(before, after);

        // A third run naming a schema that is not vendored: the case that
        // catches an implementation creating the schema directory on a miss.
        let _ = load(repo, "not-vendored-name");
        let after_miss = snapshot(repo);
        assert_eq!(before, after_miss);
    }
}
