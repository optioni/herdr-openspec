//! The dashboard shell: composition root for the render seam. See
//! `openspec/changes/tui-shell/design.md` for the full contract.

pub mod app;
pub mod driver;
pub mod event;
pub mod layout;
pub mod terminal;
pub mod view;

use std::path::Path;

use crate::config::Config;
use crate::ui::app::{Dashboard, Route};

/// Startup state, read from files only: `resolve::find_repo` then, when a
/// root was found, `changes::from_files`. Makes no CLI call, spawns no
/// process, and consults no `openspec` binary, so the dashboard opens with
/// a complete change list on a machine where `openspec` is not installed.
/// Always returns a `Dashboard`, never a `Result`, and never panics.
pub fn load(start: &Path, config: &Config) -> Dashboard {
    match crate::resolve::find_repo(start) {
        crate::resolve::RepoSearch::Found { root } => {
            let changes = crate::changes::from_files(&root, config.archived_count);
            let searched_from =
                std::fs::canonicalize(start).unwrap_or_else(|_| start.to_path_buf());
            Dashboard {
                repo: Some(root),
                searched_from,
                changes,
                route: Route::List,
                quit: false,
            }
        }
        crate::resolve::RepoSearch::NotFound { searched_from } => Dashboard {
            repo: None,
            searched_from,
            changes: crate::changes::empty_set(),
            route: Route::List,
            quit: false,
        },
    }
}

#[cfg(test)]
mod tests {
    mod load {
        use crate::changes::ChangeSet;
        use crate::config::Config;
        use crate::testutil::{ScratchDir, canonical, snapshot, write_with_mode};
        use crate::ui::app::Route;

        fn write(path: &std::path::Path, contents: &str) {
            write_with_mode(path, contents.as_bytes(), 0o644);
        }

        fn config_with_archived_count(archived_count: usize) -> Config {
            Config {
                archived_count,
                ..Config::default()
            }
        }

        #[test]
        fn a_scratch_repository_is_loaded_from_files() {
            let scratch = ScratchDir::new();
            let root = scratch.path();
            write(&root.join("openspec/changes/alpha/proposal.md"), "# P\n");
            write(
                &root.join("openspec/changes/alpha/tasks.md"),
                "- [x] a\n- [x] b\n- [ ] c\n",
            );
            let start = root.join("a").join("b");
            std::fs::create_dir_all(&start).expect("create a two-level-deep start directory");

            let dashboard = super::super::load(&start, &config_with_archived_count(5));

            assert_eq!(dashboard.repo, Some(canonical(root)));
            assert_eq!(dashboard.changes.active.len(), 1);
            assert_eq!(dashboard.changes.active[0].name, "alpha");
            assert_eq!(dashboard.changes.active[0].progress.completed, 2);
            assert_eq!(dashboard.changes.active[0].progress.total, 3);
            assert_eq!(dashboard.route, Route::List);
            assert!(!dashboard.quit);
        }

        #[test]
        fn no_repository_above_the_start() {
            let scratch = ScratchDir::new();
            let start = scratch.path();

            // Precondition: no ancestor of `start`, up to and including `/`,
            // holds an `openspec` directory — matching resolve's own
            // `NotFound` test guard, so a stray one on some machine fails
            // this test with a named ancestor rather than confusingly.
            let canonical_start = canonical(start);
            let mut ancestor = Some(canonical_start.as_path());
            while let Some(a) = ancestor {
                assert!(
                    !a.join("openspec").is_dir(),
                    "ancestor {} unexpectedly holds an openspec directory; \
                     the fixture assumption for this test is violated on this machine",
                    a.display()
                );
                ancestor = a.parent();
            }

            let expected_searched_from = match crate::resolve::find_repo(start) {
                crate::resolve::RepoSearch::NotFound { searched_from } => searched_from,
                other => panic!("expected NotFound, got {other:?}"),
            };

            let dashboard = super::super::load(start, &config_with_archived_count(5));

            assert_eq!(dashboard.repo, None);
            assert_eq!(dashboard.searched_from, expected_searched_from);
            assert_eq!(
                dashboard.changes,
                ChangeSet {
                    active: Vec::new(),
                    archived: Vec::new(),
                    problems: Vec::new(),
                }
            );
        }

        #[test]
        fn archived_count_from_config_is_honoured() {
            let scratch = ScratchDir::new();
            let root = scratch.path();
            for (i, day) in (1..=7).enumerate() {
                write(
                    &root
                        .join(format!(
                            "openspec/changes/archive/2026-01-{day:02}-entry{i}"
                        ))
                        .join("proposal.md"),
                    "# P\n",
                );
            }

            let three = super::super::load(root, &config_with_archived_count(3));
            assert_eq!(three.changes.archived.len(), 3);

            let seven = super::super::load(root, &config_with_archived_count(7));
            assert_eq!(seven.changes.archived.len(), 7);
        }

        #[test]
        fn loading_writes_nothing() {
            let scratch = ScratchDir::new();
            let root = scratch.path();
            write(&root.join("openspec/changes/alpha/proposal.md"), "# P\n");
            write(
                &root.join("openspec/changes/alpha/tasks.md"),
                "- [x] a\n- [ ] b\n",
            );

            let before = snapshot(root);
            let _ = super::super::load(root, &config_with_archived_count(5));
            let after = snapshot(root);
            assert_eq!(before, after, "ui::load wrote inside the repository");
        }
    }
}
