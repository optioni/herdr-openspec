//! The filesystem watch: the only module in the crate naming `notify`. See
//! `specs/watch-invalidation/spec.md` and `openspec/changes/live-refresh/design.md`
//! -> Boundaries. Confined on exactly `src/cli.rs`'s (`NOSPAWN-GREP`) and
//! `src/ui/markdown.rs`'s (`MDSEAM`) single-file terms: the watch crate is
//! replaceable by editing this one file, and no other file in the crate —
//! `tests/` included — may reach for it. The module is plain data: it names
//! no view type and spawns no process.
//!
//! Group 1 populates only the inert halves: the trait, the error type, and
//! stub free functions with no rule inside them. The real debounce arrives
//! in groups 4-6.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::changes::Selection;

/// Why a watch could not be started, or why a drain failed. Carries the
/// reason as text, exactly as `WatchError`'s callers need it for a
/// `!`-marked problem row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchError(pub String);

impl std::fmt::Display for WatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A touched path's classification relative to `<repo>/openspec/`. Pure and
/// total over `classify`'s two arguments: no filesystem, no
/// `canonicalize`, no clock, so a path that no longer exists classifies
/// exactly as one that does. See `specs/watch-invalidation/spec.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Touch {
    /// A path under `openspec/changes/<name>/`, at least one component past
    /// `<name>`, where `<name>` is not `archive`.
    Change(String),
    /// The change directory itself, the `changes` directory, `config.yaml`,
    /// `schemas/…`, or anything else at or below `openspec/` this rule does
    /// not otherwise recognise. Conservative on purpose: a wrong
    /// `Repository` costs one extra `list --json` on a cycle that was
    /// happening anyway; a wrong `Outside` would silently stop the pane
    /// updating.
    Repository,
    /// `openspec/changes/archive` or anything below it.
    Archived,
    /// Not below `<repo>/openspec/` at all, including a relative path.
    Outside,
}

/// Classify `path`'s position relative to `<repo>/openspec/`. Pure and
/// total: touches no filesystem, canonicalizes nothing, reads no clock.
pub fn classify(repo: &Path, path: &Path) -> Touch {
    use std::path::Component;

    let openspec_root = repo.join("openspec");
    let Ok(rel) = path.strip_prefix(&openspec_root) else {
        return Touch::Outside;
    };

    let mut components = rel.components();
    let Some(Component::Normal(first)) = components.next() else {
        // `openspec/` itself, or a path whose first segment is `.`/`..`/a
        // root — conservative: treat as a repository-level touch.
        return Touch::Repository;
    };
    if first != "changes" {
        return Touch::Repository;
    }
    let Some(Component::Normal(name)) = components.next() else {
        // `openspec/changes` itself.
        return Touch::Repository;
    };
    let name = name.to_string_lossy().into_owned();
    if name == "archive" {
        return Touch::Archived;
    }
    match components.next() {
        None => Touch::Repository, // `openspec/changes/<name>` exactly
        Some(_) => Touch::Change(name),
    }
}

/// Fold a batch of touched paths into the `Selection` the CLI producer
/// needs re-asked about: `All` when any path is `Repository` or `Archived`,
/// otherwise `Only` of the `Change` names — `Only` of the empty set when
/// every path is `Outside`, which is meaningful and not the same as doing
/// nothing: the worker still re-reads files and still runs
/// `openspec list --json`.
pub fn invalidate(repo: &Path, paths: &[PathBuf]) -> Selection {
    let mut names = std::collections::BTreeSet::new();
    for path in paths {
        match classify(repo, path) {
            Touch::Repository | Touch::Archived => return Selection::All,
            Touch::Change(name) => {
                names.insert(name);
            }
            Touch::Outside => {}
        }
    }
    Selection::Only(names)
}

/// A non-blocking source of filesystem touches. Every method SHALL be
/// non-blocking — the render path calls `drain` and `pending_in` on every
/// frame, and neither may wait on anything. See `specs/watch-invalidation/spec.md`.
pub trait FsEvents: Send {
    /// A debounced batch of touched paths, or `None` when nothing is ready
    /// yet. Never blocks, never sleeps, never waits on a channel. Never
    /// returns `Ok(Some(vec![]))`: an empty batch and no batch are the same
    /// fact.
    fn drain(&mut self) -> Result<Option<Vec<PathBuf>>, WatchError>;
    /// The time remaining before the next batch becomes due, or `None` when
    /// nothing is pending. Takes no `Instant`: the real implementation
    /// returns the value its own last `drain` already computed.
    fn pending_in(&self) -> Option<Duration>;
}

/// The inert implementation: `drain` is always `Ok(None)`, `pending_in` is
/// always `None`. What `watch::start` returns on failure, and what `ui::run`
/// passes when no repository was found.
struct NoFsEvents;

impl FsEvents for NoFsEvents {
    fn drain(&mut self) -> Result<Option<Vec<PathBuf>>, WatchError> {
        Ok(None)
    }

    fn pending_in(&self) -> Option<Duration> {
        None
    }
}

/// The inert `FsEvents`. See [`NoFsEvents`].
pub fn none() -> Box<dyn FsEvents> {
    Box::new(NoFsEvents)
}

/// A pure function of two `Duration`s, reading no clock: `tick` when nothing
/// is pending, otherwise the smaller of `tick` and the remaining window,
/// floored at one millisecond so a faulty `FsEvents` degrades to a hot pane
/// rather than a hung one. This stub always returns `tick`; the real rule
/// arrives in group 5.
pub fn poll_timeout(tick: Duration, pending_in: Option<Duration>) -> Duration {
    let _ = pending_in;
    tick
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(names: &[&str]) -> std::collections::BTreeSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_change_file_invalidates_only_that_change() {
        let repo = Path::new("/r");
        let result = invalidate(repo, &[PathBuf::from("/r/openspec/changes/alpha/tasks.md")]);
        assert_eq!(
            result,
            Selection::Only(set(&["alpha"])),
            "beta must not be in the set — an assertion that everything was \
             invalidated would pass either way and prove nothing"
        );

        let nested = invalidate(
            repo,
            &[PathBuf::from("/r/openspec/changes/alpha/specs/x/spec.md")],
        );
        assert_eq!(nested, Selection::Only(set(&["alpha"])));
    }

    #[test]
    fn a_nested_change_file_invalidates_only_that_change() {
        let repo = Path::new("/r");
        assert_eq!(
            classify(repo, Path::new("/r/openspec/changes/alpha/specs/x/spec.md")),
            Touch::Change("alpha".to_string())
        );
        assert_eq!(
            classify(repo, Path::new("/r/openspec/changes/alpha/tasks.md")),
            Touch::Change("alpha".to_string())
        );
    }

    #[test]
    fn the_change_directory_itself_is_a_repository_touch() {
        let repo = Path::new("/r");
        assert_eq!(
            classify(repo, Path::new("/r/openspec/changes/gamma")),
            Touch::Repository
        );
    }

    #[test]
    fn the_changes_directory_is_a_repository_touch() {
        let repo = Path::new("/r");
        assert_eq!(
            classify(repo, Path::new("/r/openspec/changes")),
            Touch::Repository
        );
    }

    #[test]
    fn archive_and_schema_are_repository_touches() {
        let repo = Path::new("/r");
        assert_eq!(
            classify(repo, Path::new("/r/openspec/changes/archive")),
            Touch::Archived
        );
        assert_eq!(
            classify(
                repo,
                Path::new("/r/openspec/changes/archive/2026-01-01-x/tasks.md")
            ),
            Touch::Archived
        );
        assert_eq!(
            classify(repo, Path::new("/r/openspec/config.yaml")),
            Touch::Repository
        );
        assert_eq!(
            classify(repo, Path::new("/r/openspec/schemas/tdd/schema.yaml")),
            Touch::Repository
        );

        let batch = invalidate(repo, &[PathBuf::from("/r/openspec/changes/archive")]);
        assert_eq!(batch, Selection::All);
        let batch2 = invalidate(
            repo,
            &[PathBuf::from(
                "/r/openspec/changes/archive/2026-01-01-x/tasks.md",
            )],
        );
        assert_eq!(batch2, Selection::All);
        let batch3 = invalidate(repo, &[PathBuf::from("/r/openspec/config.yaml")]);
        assert_eq!(batch3, Selection::All);
        let batch4 = invalidate(
            repo,
            &[PathBuf::from("/r/openspec/schemas/tdd/schema.yaml")],
        );
        assert_eq!(batch4, Selection::All);
    }

    #[test]
    fn a_path_outside_the_repository_is_outside() {
        let repo = Path::new("/r");
        assert_eq!(
            classify(repo, Path::new("/other/openspec/changes/alpha/tasks.md")),
            Touch::Outside
        );
        assert_eq!(classify(repo, Path::new("/r")), Touch::Outside);
        assert_eq!(classify(repo, Path::new("/r/README.md")), Touch::Outside);
    }

    #[test]
    fn a_relative_path_is_outside() {
        let repo = Path::new("/r");
        assert_eq!(
            classify(repo, Path::new("openspec/changes/alpha/tasks.md")),
            Touch::Outside
        );
    }

    #[test]
    fn an_empty_path_is_outside() {
        let repo = Path::new("/r");
        assert_eq!(classify(repo, Path::new("")), Touch::Outside);
    }

    #[test]
    fn a_mixed_batch_unions_its_classifications() {
        let repo = Path::new("/r");
        let result = invalidate(
            repo,
            &[
                PathBuf::from("/r/openspec/changes/alpha/tasks.md"),
                PathBuf::from("/r/openspec/changes/beta/design.md"),
                PathBuf::from("/tmp/unrelated"),
            ],
        );
        assert_eq!(result, Selection::Only(set(&["alpha", "beta"])));
    }

    #[test]
    fn a_repository_path_in_a_batch_absorbs_the_rest() {
        let repo = Path::new("/r");
        let result = invalidate(
            repo,
            &[
                PathBuf::from("/r/openspec/changes/alpha/tasks.md"),
                PathBuf::from("/r/openspec/changes/beta/design.md"),
                PathBuf::from("/tmp/unrelated"),
                PathBuf::from("/r/openspec/config.yaml"),
            ],
        );
        assert_eq!(
            result,
            Selection::All,
            "one repository-class path absorbs every per-change one"
        );
    }

    #[test]
    fn a_batch_of_only_outside_paths_is_an_empty_only() {
        let repo = Path::new("/r");
        let result = invalidate(
            repo,
            &[
                PathBuf::from("/other/openspec/changes/alpha/tasks.md"),
                PathBuf::from("/tmp/unrelated"),
            ],
        );
        assert_eq!(
            result,
            Selection::Only(std::collections::BTreeSet::new()),
            "an empty batch must not escalate to a full CLI reload"
        );
    }
}
