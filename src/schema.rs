//! Schema resolution: which OpenSpec schema applies, and the ordered artifact
//! list it names.
//!
//! See `openspec/changes/schema-model/design.md` for the full contract.

use std::path::Path;

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
            Some(s) => Ok(Some(s)),
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

    if let Some((path, text)) = change {
        if let Some(name) = source_contribution(path, text, &mut problems) {
            return Selection {
                name,
                source: NameSource::Change,
                problems,
            };
        }
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
