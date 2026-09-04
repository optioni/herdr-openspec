//! The dashboard shell: composition root for the render seam. See
//! `openspec/changes/tui-shell/design.md` for the full contract.

pub mod app;
pub mod driver;
pub mod event;
pub mod layout;
pub mod list;
pub mod terminal;
pub mod view;

use std::io::IsTerminal;
use std::path::Path;

use crate::config::Config;
use crate::ui::app::{Dashboard, Route};
use crate::ui::driver::{LoopError, TICK};
use crate::ui::event::CrosstermEvents;
use crate::ui::terminal::{CrosstermOps, TerminalError, TerminalGuard, TerminalOps};

/// Why the dashboard failed to start.
#[derive(Debug)]
pub enum StartError {
    /// `stdout` is not a terminal — refused before any configuration read
    /// or filesystem walk. `plugin-build` owns the exit status and message
    /// this becomes; this type owns only the decision.
    NotATerminal,
    /// A `TerminalOps` operation failed while entering the guard.
    Terminal(TerminalError),
    /// Anything else — reading the working directory, constructing the
    /// real terminal, or the event loop itself.
    Io(String),
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StartError::NotATerminal => write!(f, "not a terminal"),
            StartError::Terminal(e) => write!(f, "{e}"),
            StartError::Io(s) => write!(f, "{s}"),
        }
    }
}

impl From<std::io::Error> for StartError {
    fn from(err: std::io::Error) -> Self {
        StartError::Io(err.to_string())
    }
}

impl From<LoopError> for StartError {
    fn from(err: LoopError) -> Self {
        match err {
            LoopError::Draw(detail) => StartError::Io(format!("draw failed: {detail}")),
            LoopError::Events(e) => StartError::Io(format!("event source failed: {}", e.0)),
        }
    }
}

/// The one pure decision: refuse without touching any `TerminalOps`
/// operation when `is_terminal` is false, otherwise enter the guard.
pub fn enter_if_terminal(
    is_terminal: bool,
    ops: &dyn TerminalOps,
) -> Result<TerminalGuard<'_>, StartError> {
    if !is_terminal {
        return Err(StartError::NotATerminal);
    }
    TerminalGuard::enter(ops).map_err(StartError::Terminal)
}

/// Start the dashboard: refuse without a terminal, install the panic hook,
/// load configuration and startup state, and run the event loop to
/// completion. Straight-line wiring with no branch of its own beyond `?`.
pub fn run() -> Result<(), StartError> {
    let _guard = enter_if_terminal(std::io::stdout().is_terminal(), &CrosstermOps)?;
    terminal::install_panic_hook();
    let config = crate::config::load_from_env();
    let cwd = std::env::current_dir()?;
    let mut dashboard = load(&cwd, &config);
    let mut term =
        ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stdout()))?;
    driver::run_loop(&mut term, &mut dashboard, &mut CrosstermEvents, TICK)?;
    Ok(())
}

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
                selected: 0,
                filter: crate::ui::app::Filter {
                    query: String::new(),
                    active: false,
                },
            }
        }
        crate::resolve::RepoSearch::NotFound { searched_from } => Dashboard {
            repo: None,
            searched_from,
            changes: crate::changes::empty_set(),
            route: Route::List,
            quit: false,
            selected: 0,
            filter: crate::ui::app::Filter {
                query: String::new(),
                active: false,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    mod load {
        use crate::config::Config;
        use crate::testutil::{
            ScratchDir, canonical, render_at, row_text, snapshot, write_with_mode,
        };
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
            assert_eq!(dashboard.changes, crate::changes::empty_set());
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

        /// The outer-loop acceptance test: `ui::load` -> `changes::from_files` ->
        /// `ui::list::rows` -> `ui::view::render` is a path no unit test crosses.
        /// A real scratch repository, rendered through a `TestBackend`, must show
        /// real rows. See design.md -> Test Strategy.
        #[test]
        fn a_scratch_repository_renders_its_change_rows() {
            let scratch = ScratchDir::new();
            let root = scratch.path();
            write(
                &root.join("openspec/changes/add-token-refresh/tasks.md"),
                "- [x] a\n- [x] b\n- [x] c\n- [x] d\n- [ ] e\n- [ ] f\n- [ ] g\n- [ ] h\n- [ ] i\n",
            );
            write(
                &root.join("openspec/changes/fix-empty-basket/tasks.md"),
                "- [x] a\n- [x] b\n- [x] c\n- [x] d\n- [x] e\n- [x] f\n- [x] g\n",
            );
            write(
                &root.join("openspec/changes/migrate-ai-sdk-v7/proposal.md"),
                "# P\n",
            );
            write(
                &root.join("openspec/changes/archive/2026-08-14-add-auth/tasks.md"),
                "- [x] a\n- [x] b\n- [x] c\n- [x] d\n- [x] e\n- [x] f\n- [x] g\n",
            );

            let dashboard = super::super::load(root, &config_with_archived_count(5));

            fn cols(text: &str, from: usize, to_inclusive: usize) -> String {
                text.chars()
                    .skip(from)
                    .take(to_inclusive - from + 1)
                    .collect()
            }

            let buf120 = render_at(120, 20, &dashboard);
            assert_eq!(
                cols(&row_text(&buf120, 2), 1, 38),
                "> add-token-refresh              [4/9]"
            );
            assert_eq!(
                cols(&row_text(&buf120, 3), 1, 38),
                "  fix-empty-basket               [7/7]"
            );
            assert_eq!(
                cols(&row_text(&buf120, 4), 1, 38),
                "  migrate-ai-sdk-v7                [-]"
            );
            assert_eq!(
                cols(&row_text(&buf120, 5), 1, 38),
                "  -- archived ------------------------"
            );
            assert_eq!(
                cols(&row_text(&buf120, 6), 1, 38),
                "  2026-08-14 add-auth            [7/7]"
            );

            let buf60 = render_at(60, 20, &dashboard);
            assert_eq!(
                cols(&row_text(&buf60, 2), 1, 58),
                "> add-token-refresh                                  [4/9]"
            );
            assert_eq!(
                cols(&row_text(&buf60, 3), 1, 58),
                "  fix-empty-basket                                   [7/7]"
            );
            assert_eq!(
                cols(&row_text(&buf60, 4), 1, 58),
                "  migrate-ai-sdk-v7                                    [-]"
            );
            assert_eq!(
                cols(&row_text(&buf60, 5), 1, 58),
                "  -- archived --------------------------------------------"
            );
            assert_eq!(
                cols(&row_text(&buf60, 6), 1, 58),
                "  2026-08-14 add-auth                                [7/7]"
            );
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

    mod start {
        use std::cell::RefCell;

        use crate::ui::terminal::{TerminalError, TerminalOps};

        #[derive(Default)]
        struct Recorder {
            calls: RefCell<Vec<&'static str>>,
            fail_enable_raw: bool,
        }

        impl Recorder {
            fn calls(&self) -> Vec<&'static str> {
                self.calls.borrow().clone()
            }
        }

        impl TerminalOps for Recorder {
            fn enable_raw(&self) -> Result<(), TerminalError> {
                self.calls.borrow_mut().push("enable_raw");
                if self.fail_enable_raw {
                    return Err(TerminalError {
                        op: "enable_raw",
                        detail: "device busy".to_string(),
                    });
                }
                Ok(())
            }
            fn enter_alternate(&self) -> Result<(), TerminalError> {
                self.calls.borrow_mut().push("enter_alternate");
                Ok(())
            }
            fn leave_alternate(&self) -> Result<(), TerminalError> {
                self.calls.borrow_mut().push("leave_alternate");
                Ok(())
            }
            fn disable_raw(&self) -> Result<(), TerminalError> {
                self.calls.borrow_mut().push("disable_raw");
                Ok(())
            }
        }

        #[test]
        fn not_a_terminal_records_no_call() {
            let rec = Recorder::default();
            let result = super::super::enter_if_terminal(false, &rec);
            assert!(matches!(
                result,
                Err(super::super::StartError::NotATerminal)
            ));
            assert!(rec.calls().is_empty());
        }

        #[test]
        fn a_terminal_enters_the_guard() {
            let rec = Recorder::default();
            let result = super::super::enter_if_terminal(true, &rec);
            assert!(result.is_ok());
            assert_eq!(rec.calls(), vec!["enable_raw", "enter_alternate"]);
        }

        #[test]
        fn a_failing_enable_raw_becomes_start_error_terminal() {
            let rec = Recorder {
                fail_enable_raw: true,
                ..Recorder::default()
            };
            let result = super::super::enter_if_terminal(true, &rec);
            match result {
                Err(super::super::StartError::Terminal(e)) => assert_eq!(e.op, "enable_raw"),
                other => panic!("expected StartError::Terminal, got {other:?}"),
            }
        }

        // Closes a Change Review finding: StartError's Display and From impls
        // are pure, need no terminal, and were entirely uncovered even though
        // quality-gates' coverage scenario claims the uncoverable residue is
        // confined to a named, smaller set. TerminalError's own Display is
        // already tested in ui::terminal — this brings StartError's up to the
        // same standard.
        #[test]
        fn display_names_the_right_text_per_variant() {
            assert_eq!(
                super::super::StartError::NotATerminal.to_string(),
                "not a terminal"
            );
            let terminal_err = super::super::StartError::Terminal(TerminalError {
                op: "enable_raw",
                detail: "device busy".to_string(),
            });
            let text = terminal_err.to_string();
            assert!(text.contains("enable_raw"));
            assert!(text.contains("device busy"));
            assert_eq!(
                super::super::StartError::Io("boom".to_string()).to_string(),
                "boom"
            );
        }

        #[test]
        fn io_error_converts_to_start_error_io() {
            let io_err = std::io::Error::other("disk full");
            let start_err: super::super::StartError = io_err.into();
            assert!(start_err.to_string().contains("disk full"));
        }

        #[test]
        fn loop_error_converts_to_start_error_io() {
            let draw: super::super::StartError =
                crate::ui::driver::LoopError::Draw("panel broke".to_string()).into();
            assert!(draw.to_string().contains("draw failed"));
            assert!(draw.to_string().contains("panel broke"));

            let events: super::super::StartError = crate::ui::driver::LoopError::Events(
                crate::ui::event::EventError("script exhausted".to_string()),
            )
            .into();
            assert!(events.to_string().contains("event source failed"));
            assert!(events.to_string().contains("script exhausted"));
        }
    }
}
