//! The dashboard shell: composition root for the render seam. See
//! `openspec/changes/tui-shell/design.md` for the full contract.

pub mod app;
pub mod detail;
pub mod driver;
pub mod event;
pub mod layout;
pub mod list;
pub mod markdown;
pub mod tasks;
pub mod terminal;
pub mod view;

use std::io::IsTerminal;
use std::path::Path;

use crate::config::Config;
use crate::ui::app::{Dashboard, Detail, Route};
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
    let mut fs = crate::watch::none();
    let mut refresher = crate::refresh::none();
    let mut live = crate::ui::driver::Live {
        fs: &mut *fs,
        refresher: &mut *refresher,
    };
    driver::run_loop(
        &mut term,
        &mut dashboard,
        &mut CrosstermEvents,
        &mut live,
        &read_artifact,
        TICK,
    )?;
    Ok(())
}

/// The crate's one artifact-file read: `std::fs::read_to_string`, mapping
/// its error to the error's `Display` text. The only binding under
/// `src/ui/` naming `read_to_string` — `READSEAM` is the mechanical form —
/// so the read can be replaced, faked, or moved behind a worker thread by
/// editing this one file. `ui::run` is the only caller that passes it.
pub fn read_artifact(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| e.to_string())
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
                detail: Detail {
                    source: String::new(),
                    scroll: 0,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
                },
                refresh: crate::ui::app::Refresh {
                    requested: true,
                    reload: false,
                    problems: Vec::new(),
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
            detail: Detail {
                source: String::new(),
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
            },
            refresh: crate::ui::app::Refresh {
                requested: true,
                reload: false,
                problems: Vec::new(),
            },
        },
    }
}

#[cfg(test)]
mod tests {
    /// `ui::read_artifact` — the crate's third one-line binding to the real
    /// world, carrying its own assertions on the same terms
    /// `config::env_lookup` does. Real filesystem, through
    /// `crate::testutil::ScratchDir`: this is the one thing the binding
    /// exists to do, so it is proven against a real directory rather than a
    /// double.
    mod read_artifact {
        use crate::testutil::{ScratchDir, write_with_mode};

        #[test]
        fn agrees_with_the_standard_library_on_a_written_file() {
            let scratch = ScratchDir::new();
            let path = scratch.path().join("proposal.md");
            write_with_mode(&path, b"# proposal\n", 0o644);

            let got = super::super::read_artifact(&path);
            let want = std::fs::read_to_string(&path);
            assert_eq!(got, want.map_err(|e| e.to_string()));
            assert_eq!(got.unwrap(), "# proposal\n");
        }

        #[test]
        fn names_its_failure_on_a_missing_path() {
            let scratch = ScratchDir::new();
            let path = scratch.path().join("does-not-exist.md");

            let got = super::super::read_artifact(&path);
            match got {
                Err(e) => assert!(!e.is_empty()),
                Ok(_) => panic!("expected an Err for a missing path"),
            }
        }
    }

    /// The outer-loop acceptance test: `ui::app::action_for` ->
    /// `Dashboard::apply` -> `ui::markdown::lines` -> `ui::view::render` ->
    /// `Dashboard::normalise_scroll` is a path no unit test crosses. See
    /// design.md -> Test Strategy.
    mod detail {
        use ratatui::layout::Rect;
        use ratatui::widgets::Block;

        use crate::testutil::{RecordingReader, Script, press};
        use crate::ui::app::{Dashboard, Detail, Filter, Route};
        use crate::ui::driver::run_loop;
        use crate::ui::layout::{split_body, split_frame};

        /// A dashboard whose one selected active change carries one
        /// artifact resolving to `/repo/p.md` — `detail-view`'s
        /// `sync_detail` is what fills `detail.source` now, driven by the
        /// injected reader, rather than this fixture setting it directly.
        fn dashboard() -> Dashboard {
            let change = crate::changes::fixture::with_artifacts(
                crate::changes::fixture::active("detail-view", 4, 9),
                &[("proposal", &["/repo/p.md"])],
            );
            Dashboard {
                repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
                searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
                changes: crate::changes::fixture::set(vec![change], Vec::new(), Vec::new()),
                route: Route::Detail,
                quit: false,
                selected: 0,
                filter: Filter {
                    query: String::new(),
                    active: false,
                },
                detail: Detail {
                    source: String::new(),
                    scroll: 0,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
                },
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    problems: Vec::new(),
                },
            }
        }

        /// The detail interior, derived the same way the view seam derives
        /// it, so this test fails loudly if the breakpoint or the frame
        /// split ever moves rather than silently reading a stale rectangle.
        fn detail_interior(width: u16, height: u16, route: Route) -> Rect {
            let (_, body, _) = split_frame(Rect::new(0, 0, width, height));
            let (_, detail) = split_body(body, route);
            let detail = detail.expect("detail region must be drawn for this test's routes");
            Block::bordered().inner(detail)
        }

        #[test]
        fn a_markdown_document_renders_and_scrolls_through_the_loop() {
            // Extended from two `j` presses to ten at group 7 (task 7.6):
            // with only two, the acceptance test started passing as soon
            // as render_detail existed, well before ui::driver normalises
            // the stored offset — group 8's RED would have been vacuous.
            // Ten presses run well past the twenty-item source's six-line
            // overshoot (the content area is 14 rows, two narrower than
            // the 16-row interior since detail-view's header and tab bar
            // now take the first two), so the assertion on the FINAL
            // stored `detail.scroll` (6, not 10) is the one that stays red
            // until `run_loop` calls `normalise_scroll` against the
            // content area's own height.
            for width in [120u16, 60u16] {
                let mut dashboard = dashboard();
                let interior = detail_interior(width, 20, Route::Detail);
                let content_y = interior.y + 2;
                let content_height = interior.height - 2;

                let backend = ratatui::backend::TestBackend::new(width, 20);
                let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
                let mut presses: Vec<_> = (0..10)
                    .map(|_| {
                        Ok(Some(press(
                            ratatui::crossterm::event::KeyCode::Char('j'),
                            ratatui::crossterm::event::KeyModifiers::NONE,
                        )))
                    })
                    .collect();
                presses.push(Ok(Some(press(
                    ratatui::crossterm::event::KeyCode::Char('q'),
                    ratatui::crossterm::event::KeyModifiers::NONE,
                ))));
                let mut events = Script::new(presses);

                let twenty_lines: String = (0..20).map(|i| format!("- line-{i:02}\n")).collect();
                let recorder = RecordingReader::always(Ok(twenty_lines));
                let read = |p: &std::path::Path| recorder.read(p);

                let mut fs = crate::watch::none();
                let mut refresher = crate::refresh::none();
                let mut live = crate::ui::driver::Live {
                    fs: &mut *fs,
                    refresher: &mut *refresher,
                };
                let summary = run_loop(
                    &mut terminal,
                    &mut dashboard,
                    &mut events,
                    &mut live,
                    &read,
                    std::time::Duration::from_millis(1),
                )
                .expect("loop ends");

                let buf = terminal.backend().buffer();
                let row_at = |y: u16| -> String {
                    (interior.x..interior.x + 9)
                        .map(|x| buf[(x, y)].symbol().to_string())
                        .collect()
                };
                assert_eq!(row_at(content_y), "- line-06", "width {width}");
                assert_eq!(
                    row_at(content_y + content_height - 1),
                    "- line-19",
                    "width {width}"
                );

                assert_eq!(dashboard.detail.scroll, 6, "width {width}");
                assert_eq!(summary.frames, 11, "width {width}");
            }
        }

        /// `detail-view`'s outer-loop acceptance test:
        /// `ui::app::action_for` -> `Dashboard::apply` ->
        /// `Dashboard::sync_detail` -> `ui::detail::*` -> `ui::view::render`
        /// -> `Dashboard::normalise_scroll` is a path no unit test crosses.
        /// RED until group 10: the header row is not drawn at all until
        /// group 9, and `normalise_scroll` does not use the content area's
        /// height until group 10.
        #[test]
        fn a_selected_changes_artifact_is_read_shown_and_scrolled_through_the_loop() {
            let proposal_path =
                std::path::PathBuf::from("/repo/openspec/changes/detail-view/proposal.md");
            let specs_path =
                std::path::PathBuf::from("/repo/openspec/changes/detail-view/specs/spec.md");
            let twenty_item_source: String = (0..20).map(|i| format!("- line-{i:02}\n")).collect();

            let dashboard = || -> Dashboard {
                let selected = crate::changes::fixture::with_artifacts(
                    crate::changes::fixture::active("detail-view", 4, 9),
                    &[
                        (
                            "proposal",
                            &["/repo/openspec/changes/detail-view/proposal.md"],
                        ),
                        (
                            "specs",
                            &["/repo/openspec/changes/detail-view/specs/spec.md"],
                        ),
                        ("design", &["/repo/openspec/changes/detail-view/design.md"]),
                        ("tasks", &["/repo/openspec/changes/detail-view/tasks.md"]),
                        (
                            "planning-review",
                            &["/repo/openspec/changes/detail-view/planning-review.md"],
                        ),
                    ],
                );
                let other = crate::changes::fixture::active("fix-empty-basket", 7, 7);
                Dashboard {
                    repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
                    searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
                    changes: crate::changes::fixture::set(
                        vec![selected, other],
                        Vec::new(),
                        Vec::new(),
                    ),
                    route: Route::Detail,
                    quit: false,
                    selected: 0,
                    filter: Filter {
                        query: String::new(),
                        active: false,
                    },
                    detail: Detail {
                        source: String::new(),
                        scroll: 0,
                        tab: 0,
                        problems: Vec::new(),
                        loaded: None,
                    },
                    refresh: crate::ui::app::Refresh {
                        requested: false,
                        reload: false,
                        problems: Vec::new(),
                    },
                }
            };

            for width in [120u16, 60u16] {
                let mut dashboard = dashboard();
                let interior = detail_interior(width, 20, Route::Detail);

                let recorder = crate::testutil::RecordingReader::new(
                    vec![(specs_path.clone(), Ok(twenty_item_source.clone()))],
                    Ok("# proposal\n".to_string()),
                );
                let read = |p: &std::path::Path| recorder.read(p);

                let backend = ratatui::backend::TestBackend::new(width, 20);
                let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
                let mut presses = vec![Ok(Some(press(
                    ratatui::crossterm::event::KeyCode::Char(']'),
                    ratatui::crossterm::event::KeyModifiers::NONE,
                )))];
                presses.extend((0..10).map(|_| {
                    Ok(Some(press(
                        ratatui::crossterm::event::KeyCode::Char('j'),
                        ratatui::crossterm::event::KeyModifiers::NONE,
                    )))
                }));
                presses.push(Ok(Some(press(
                    ratatui::crossterm::event::KeyCode::Char('q'),
                    ratatui::crossterm::event::KeyModifiers::NONE,
                ))));
                let mut events = Script::new(presses);

                let mut fs = crate::watch::none();
                let mut refresher = crate::refresh::none();
                let mut live = crate::ui::driver::Live {
                    fs: &mut *fs,
                    refresher: &mut *refresher,
                };
                run_loop(
                    &mut terminal,
                    &mut dashboard,
                    &mut events,
                    &mut live,
                    &read,
                    std::time::Duration::from_millis(1),
                )
                .expect("loop ends");

                let buf = terminal.backend().buffer();
                let row_cols = |y: u16, len: u16| -> String {
                    (interior.x..interior.x + len)
                        .map(|x| buf[(x, y)].symbol().to_string())
                        .collect()
                };
                let header_row = row_cols(interior.y, interior.width);
                assert!(
                    header_row.starts_with("detail-view"),
                    "width {width}: header row does not start with the change name: {header_row:?}"
                );
                assert!(
                    header_row.ends_with("[4/9]"),
                    "width {width}: header row does not end in its progress cell: {header_row:?}"
                );

                let tab_row = row_cols(interior.y + 1, 20);
                assert!(
                    tab_row.starts_with("1 proposal  2 specs"),
                    "width {width}: tab row does not begin with the first two tabs: {tab_row:?}"
                );

                let content_row_at = |y: u16| -> String { row_cols(y, 9) };
                assert_eq!(content_row_at(interior.y + 2), "- line-06", "width {width}");
                assert_eq!(
                    content_row_at(interior.y + interior.height - 1),
                    "- line-19",
                    "width {width}"
                );

                assert_eq!(dashboard.detail.tab, 1, "width {width}");
                assert_eq!(
                    dashboard.detail.scroll, 6,
                    "width {width}: not 10 — stays red until normalise_scroll uses the \
                     content area's fourteen rows rather than the whole interior"
                );

                assert_eq!(recorder.calls(), 2, "width {width}");
                assert_eq!(
                    recorder.paths(),
                    vec![proposal_path.clone(), specs_path.clone()],
                    "width {width}: exactly the first artifact's path, then the second's, \
                     and nothing more across the ten j presses"
                );
            }
        }

        /// `tasks-checklist`'s outer-loop acceptance test:
        /// `ui::app::action_for` -> `Dashboard::apply` -> `Dashboard::sync_detail`
        /// -> `ui::detail::content_lines` -> `ui::tasks::lines` ->
        /// `ui::view::render` -> `Dashboard::normalise_scroll` is a path no unit
        /// test crosses, and the read-only claim is only meaningful against a
        /// real change directory. See `specs/tasks-checklist/spec.md` ->
        /// "Every printable key leaves the change tree byte-identical".
        ///
        /// The rendering half and the read-only half are checked through two
        /// separate `run_loop` calls sharing one `Dashboard` and one
        /// `TestBackend`, not one: `list-filtering`'s own landed and
        /// unchanged rule ("`/` reaches the list from either route") means
        /// the ASCII sweep's `/` unconditionally sends `route` to
        /// `Route::List`, and nothing later in the sequence — `Enter`
        /// closes filtering without reopening detail, by that same landed
        /// rule — sends it back. The spec scenario's own `THEN` clauses
        /// (termination, snapshot equality, unchanged progress, a
        /// discriminating control) never depend on the final route, so
        /// splitting costs nothing the spec asks for while still routing
        /// the *entire* key sequence through one real loop over one real
        /// directory.
        fn vendor_tdd_schema(repo: &std::path::Path) {
            let yaml = "\
name: tdd
artifacts:
  - id: proposal
    generates: proposal.md
  - id: specs
    generates: specs/**/*.md
  - id: design
    generates: design.md
  - id: tasks
    generates: tasks.md
  - id: planning-review
    generates: planning-review.md
apply:
  tracks: tasks.md
";
            crate::testutil::write_with_mode(
                &repo.join("openspec/schemas/tdd/schema.yaml"),
                yaml.as_bytes(),
                0o644,
            );
            crate::testutil::write_with_mode(
                &repo.join("openspec/config.yaml"),
                b"schema: tdd\n",
                0o644,
            );
        }

        #[test]
        fn tasks_tab_is_read_only() {
            let scratch = crate::testutil::ScratchDir::new();
            let root = scratch.path();
            vendor_tdd_schema(root);
            crate::testutil::write_with_mode(
                &root.join("openspec/changes/tasks-tab-demo/.openspec.yaml"),
                b"schema: tdd\n",
                0o644,
            );
            let tasks_path = root.join("openspec/changes/tasks-tab-demo/tasks.md");
            let tasks_source = "## 1. Setup\n- [x] 1.1 first\n- [x] 1.2 second\n- [ ] 1.3 third\n\n\
                 ## 2. Build\n- [ ] 2.1 fourth\n- [ ] 2.2 fifth\n";
            crate::testutil::write_with_mode(&tasks_path, tasks_source.as_bytes(), 0o644);

            for width in [120u16, 60u16] {
                let config = crate::config::Config::default();
                let mut dashboard = super::super::load(root, &config);

                let backend = ratatui::backend::TestBackend::new(width, 20);
                let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");

                let before = crate::testutil::snapshot(root);

                // Stage 1: open the one active change and select the
                // tracked-tasks tab (position 3 of the `tdd` schema), then
                // quit cleanly — a run untouched by `/`, so its final
                // buffer is the one the checklist grammar actually drew.
                let mut stage1 = Script::new(vec![
                    Ok(Some(press(
                        ratatui::crossterm::event::KeyCode::Enter,
                        ratatui::crossterm::event::KeyModifiers::NONE,
                    ))),
                    Ok(Some(press(
                        ratatui::crossterm::event::KeyCode::Char('4'),
                        ratatui::crossterm::event::KeyModifiers::NONE,
                    ))),
                    Ok(Some(press(
                        ratatui::crossterm::event::KeyCode::Char('c'),
                        ratatui::crossterm::event::KeyModifiers::CONTROL,
                    ))),
                ]);
                let mut fs = crate::watch::none();
                let mut refresher = crate::refresh::none();
                let mut live = crate::ui::driver::Live {
                    fs: &mut *fs,
                    refresher: &mut *refresher,
                };
                run_loop(
                    &mut terminal,
                    &mut dashboard,
                    &mut stage1,
                    &mut live,
                    &crate::ui::read_artifact,
                    std::time::Duration::from_millis(1),
                )
                .expect("stage 1 ends");

                assert_eq!(dashboard.route, Route::Detail, "width {width}");
                assert_eq!(dashboard.detail.tab, 3, "width {width}");

                let interior = detail_interior(width, 20, Route::Detail);
                let content_y = interior.y + 2;
                let buf = terminal.backend().buffer();
                let row_cols = |y: u16, len: u16| -> String {
                    (interior.x..interior.x + len)
                        .map(|x| buf[(x, y)].symbol().to_string())
                        .collect()
                };
                let bar_row = row_cols(content_y, interior.width);
                assert!(
                    bar_row.trim_end().ends_with("[2/5] 40%"),
                    "width {width}: bar row does not end in its progress cell: {bar_row:?}"
                );
                assert_eq!(
                    row_cols(content_y + 1, interior.width).trim(),
                    "",
                    "width {width}: no blank row after the bar"
                );
                let first_item_row = row_cols(content_y + 3, 12);
                assert!(
                    first_item_row.starts_with("[x]") || first_item_row.starts_with("[ ]"),
                    "width {width}: no checklist glyph row below the heading: {first_item_row:?}"
                );

                // Stage 2: the full read-only proof, over the same
                // dashboard, terminal, and reader.
                dashboard.quit = false;
                let mut presses: Vec<_> = ('!'..='~')
                    .map(|c| {
                        Ok(Some(press(
                            ratatui::crossterm::event::KeyCode::Char(c),
                            ratatui::crossterm::event::KeyModifiers::NONE,
                        )))
                    })
                    .collect();
                presses.push(Ok(Some(press(
                    ratatui::crossterm::event::KeyCode::Enter,
                    ratatui::crossterm::event::KeyModifiers::NONE,
                ))));
                presses.push(Ok(Some(press(
                    ratatui::crossterm::event::KeyCode::Esc,
                    ratatui::crossterm::event::KeyModifiers::NONE,
                ))));
                presses.push(Ok(Some(press(
                    ratatui::crossterm::event::KeyCode::Backspace,
                    ratatui::crossterm::event::KeyModifiers::NONE,
                ))));
                presses.push(Ok(Some(press(
                    ratatui::crossterm::event::KeyCode::Tab,
                    ratatui::crossterm::event::KeyModifiers::NONE,
                ))));
                for code in [
                    ratatui::crossterm::event::KeyCode::Left,
                    ratatui::crossterm::event::KeyCode::Right,
                    ratatui::crossterm::event::KeyCode::Up,
                    ratatui::crossterm::event::KeyCode::Down,
                ] {
                    presses.push(Ok(Some(press(
                        code,
                        ratatui::crossterm::event::KeyModifiers::NONE,
                    ))));
                }
                presses.push(Ok(Some(press(
                    ratatui::crossterm::event::KeyCode::Char('c'),
                    ratatui::crossterm::event::KeyModifiers::CONTROL,
                ))));
                let mut stage2 = Script::new(presses);

                let mut fs = crate::watch::none();
                let mut refresher = crate::refresh::none();
                let mut live = crate::ui::driver::Live {
                    fs: &mut *fs,
                    refresher: &mut *refresher,
                };
                run_loop(
                    &mut terminal,
                    &mut dashboard,
                    &mut stage2,
                    &mut live,
                    &crate::ui::read_artifact,
                    std::time::Duration::from_millis(1),
                )
                .expect("stage 2 ends");

                let after = crate::testutil::snapshot(root);
                assert_eq!(
                    before, after,
                    "width {width}: the change tree changed across the printable-key run"
                );

                let reread = crate::tasks::read(&tasks_path);
                assert_eq!(
                    reread.progress(),
                    crate::tasks::Progress {
                        completed: 2,
                        total: 5
                    },
                    "width {width}"
                );

                // Discriminating control: the same comparison must fail
                // when a single byte of tasks.md is rewritten between two
                // snapshots, or the equality assertion above proves
                // nothing.
                let control_before = crate::testutil::snapshot(root);
                let mutated = tasks_source.replacen("[ ] 1.3", "[x] 1.3", 1);
                crate::testutil::write_with_mode(&tasks_path, mutated.as_bytes(), 0o644);
                let control_after = crate::testutil::snapshot(root);
                assert_ne!(
                    control_before, control_after,
                    "width {width}: the snapshot comparison does not discriminate a rewritten byte"
                );
                crate::testutil::write_with_mode(&tasks_path, tasks_source.as_bytes(), 0o644);
            }
        }
    }

    mod load {
        use crate::config::Config;
        use crate::testutil::{
            ScratchDir, canonical, render_at, row_text, snapshot, write_with_mode,
        };
        use crate::ui::app::{Filter, Route};

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
            assert_eq!(dashboard.selected, 0);
            assert_eq!(
                dashboard.filter,
                Filter {
                    query: String::new(),
                    active: false
                }
            );
            assert!(dashboard.refresh.requested);
            assert!(!dashboard.refresh.reload);
            assert!(dashboard.refresh.problems.is_empty());
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
            assert_eq!(dashboard.selected, 0);
            assert_eq!(
                dashboard.filter,
                Filter {
                    query: String::new(),
                    active: false
                }
            );
            assert!(dashboard.refresh.requested);
            assert!(!dashboard.refresh.reload);
            assert!(dashboard.refresh.problems.is_empty());
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

        #[test]
        fn loading_then_syncing_with_the_real_reader_writes_nothing() {
            // The same claim, extended past `ui::load` into
            // `Dashboard::sync_detail` driven with the real
            // `ui::read_artifact` binding — resolving and reading an
            // artifact's content must leave the tree byte-identical too.
            let scratch = ScratchDir::new();
            let root = scratch.path();
            write(&root.join("openspec/changes/alpha/proposal.md"), "# P\n");
            write(
                &root.join("openspec/changes/alpha/tasks.md"),
                "- [x] a\n- [ ] b\n",
            );

            let before = snapshot(root);
            let mut dashboard = super::super::load(root, &config_with_archived_count(5));
            dashboard.sync_detail(&super::super::read_artifact);
            let after = snapshot(root);
            assert_eq!(
                before, after,
                "ui::load + sync_detail(read_artifact) wrote inside the repository"
            );
        }

        #[test]
        fn startup_leaves_the_detail_empty_and_unscrolled() {
            // Rendered at both mandated widths, but the detail-interior
            // blankness assertion only applies at 120: at 60, `route`
            // starts `List`, so the narrow layout draws the list region
            // (which change-rows already asserts elsewhere), not the
            // detail one at all. `render_at` at 60 here is only proving
            // startup doesn't panic there, on the same terms the wide
            // check proves it for the pane that IS drawn.
            fn assert_detail_blank_at_120(dashboard: &crate::ui::app::Dashboard) {
                let buf = render_at(120, 20, dashboard);
                for y in 2..=17u16 {
                    for x in 41..=118u16 {
                        assert_eq!(
                            row_text(&buf, y).chars().nth(x as usize).unwrap(),
                            ' ',
                            "x={x} y={y}"
                        );
                    }
                }
                let _ = render_at(60, 20, dashboard);
            }

            // A change **is** selected and nothing has been read yet — the
            // header and tab bar are drawn (a change is selected) and the
            // content area reads `No content yet` (`sync_detail` has not
            // run), which is exactly the state `run_loop`'s first
            // `sync_detail` replaces.
            fn assert_detail_shows_header_and_no_content_yet_at_120(
                dashboard: &crate::ui::app::Dashboard,
                name: &str,
            ) {
                let buf = render_at(120, 20, dashboard);
                assert!(row_text(&buf, 2).contains(name));
                assert!(
                    row_text(&buf, 4).trim_end().starts_with("No content yet")
                        || row_text(&buf, 4).contains("No content yet")
                );
                let _ = render_at(60, 20, dashboard);
            }

            let scratch = ScratchDir::new();
            let root = scratch.path();
            write(&root.join("openspec/changes/alpha/proposal.md"), "# P\n");

            let found = super::super::load(root, &config_with_archived_count(5));
            assert_eq!(found.detail.source, "");
            assert_eq!(found.detail.scroll, 0);
            assert_eq!(found.detail.tab, 0);
            assert!(found.detail.problems.is_empty());
            assert_eq!(found.detail.loaded, None);
            assert!(
                !found.refresh.reload,
                "live-refresh: startup must not force a reload"
            );
            assert_detail_shows_header_and_no_content_yet_at_120(&found, "alpha");

            // The RepoSearch::NotFound arm is a second Dashboard
            // construction site and therefore a second place the field
            // can be got wrong. Guarded the same way
            // `no_repository_above_the_start` is: no ancestor of the
            // scratch path may itself hold an `openspec` directory.
            let scratch2 = ScratchDir::new();
            let canonical_start = canonical(scratch2.path());
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
            let not_found = super::super::load(scratch2.path(), &config_with_archived_count(5));
            assert_eq!(not_found.repo, None);
            assert_eq!(not_found.detail.source, "");
            assert_eq!(not_found.detail.scroll, 0);
            assert!(
                !not_found.refresh.reload,
                "live-refresh: startup must not force a reload"
            );
            assert_detail_blank_at_120(&not_found);
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

    /// The outer-loop acceptance test: `ui::app::action_for` ->
    /// `Dashboard::apply` -> `run_loop`'s live tier -> `Refresher` ->
    /// `Dashboard::adopt` -> `Dashboard::sync_detail` -> `ui::view::render`
    /// is a path no unit test crosses, and "files paint, the CLI corrects"
    /// is a claim about what the loop shows in successive frames rather
    /// than about what a function returns. RED from group 2, green at
    /// group 11. See design.md -> Test Strategy.
    mod live {
        use std::time::Duration;

        use crate::changes::Selection;
        use crate::config::Config;
        use crate::refresh::RefreshResult;
        use crate::testutil::{
            RecordingRefresher, ScriptedFs, press, row_text, snapshot, write_with_mode,
        };
        use crate::ui::driver::{Live, LoopSummary, run_loop};

        fn vendor_tdd_schema(repo: &std::path::Path) {
            let yaml = "\
name: tdd
artifacts:
  - id: proposal
    generates: proposal.md
  - id: specs
    generates: specs/**/*.md
  - id: design
    generates: design.md
  - id: tasks
    generates: tasks.md
  - id: planning-review
    generates: planning-review.md
apply:
  tracks: tasks.md
";
            write_with_mode(
                &repo.join("openspec/schemas/tdd/schema.yaml"),
                yaml.as_bytes(),
                0o644,
            );
            write_with_mode(&repo.join("openspec/config.yaml"), b"schema: tdd\n", 0o644);
        }

        /// A scratch repository holding one active change, `alpha`, whose
        /// `tasks.md` counts 4 of 9.
        fn scratch_repo_with_alpha() -> crate::testutil::ScratchDir {
            let scratch = crate::testutil::ScratchDir::new();
            let root = scratch.path();
            vendor_tdd_schema(root);
            write_with_mode(
                &root.join("openspec/changes/alpha/proposal.md"),
                b"# alpha\n",
                0o644,
            );
            write_with_mode(
                &root.join("openspec/changes/alpha/tasks.md"),
                b"- [x] a\n- [x] b\n- [x] c\n- [x] d\n- [ ] e\n- [ ] f\n- [ ] g\n- [ ] h\n- [ ] i\n",
                0o644,
            );
            scratch
        }

        #[test]
        fn files_paint_then_the_cli_corrects() {
            for width in [120u16, 60u16] {
                let scratch = scratch_repo_with_alpha();
                let root = scratch.path();
                let config = Config::default();
                let mut dashboard = super::super::load(root, &config);

                let before = snapshot(root);

                // Stage 1: no CLI result has arrived yet.
                let backend = ratatui::backend::TestBackend::new(width, 20);
                let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
                let mut events = crate::testutil::Script::new(vec![Ok(Some(press(
                    ratatui::crossterm::event::KeyCode::Char('q'),
                    ratatui::crossterm::event::KeyModifiers::NONE,
                )))]);
                let mut fs = ScriptedFs::new(Vec::new(), Vec::new());
                let mut refresher = RecordingRefresher::new(Vec::new());
                let mut live = Live {
                    fs: &mut fs,
                    refresher: &mut refresher,
                };

                let summary = run_loop(
                    &mut terminal,
                    &mut dashboard,
                    &mut events,
                    &mut live,
                    &super::super::read_artifact,
                    Duration::from_millis(1),
                )
                .expect("stage 1 ends");

                assert_eq!(
                    summary,
                    LoopSummary {
                        frames: 1,
                        polls: 1
                    },
                    "width {width}"
                );
                let buf = terminal.backend().buffer();
                assert!(
                    row_text(buf, 2).contains("[4/9]"),
                    "width {width}: files must paint before any CLI result existed: {}",
                    row_text(buf, 2)
                );
                assert_eq!(
                    refresher.requests(),
                    vec![Selection::All],
                    "width {width}: the startup request must go out"
                );
                assert!(
                    !dashboard.refresh.requested,
                    "width {width}: the flag must be cleared once the request was made"
                );

                // Stage 2: the CLI's file result, then its merged result.
                dashboard.quit = false;
                let files_set = crate::changes::fixture::set(
                    vec![crate::changes::fixture::active("alpha", 4, 9)],
                    Vec::new(),
                    Vec::new(),
                );
                let merged_set = crate::changes::fixture::set(
                    vec![crate::changes::fixture::active("alpha", 7, 9)],
                    Vec::new(),
                    Vec::new(),
                );
                let mut fs2 = ScriptedFs::new(Vec::new(), Vec::new());
                let mut refresher2 = RecordingRefresher::new(vec![
                    Some(RefreshResult::Files(files_set)),
                    Some(RefreshResult::Merged(merged_set)),
                ]);
                let mut live2 = Live {
                    fs: &mut fs2,
                    refresher: &mut refresher2,
                };
                let mut events2 = crate::testutil::Script::new(vec![
                    Ok(None),
                    Ok(None),
                    Ok(Some(press(
                        ratatui::crossterm::event::KeyCode::Char('q'),
                        ratatui::crossterm::event::KeyModifiers::NONE,
                    ))),
                ]);

                run_loop(
                    &mut terminal,
                    &mut dashboard,
                    &mut events2,
                    &mut live2,
                    &super::super::read_artifact,
                    Duration::from_millis(1),
                )
                .expect("stage 2 ends");

                let buf2 = terminal.backend().buffer();
                assert!(
                    row_text(buf2, 2).contains("[7/9]"),
                    "width {width}: the CLI must have corrected it: {}",
                    row_text(buf2, 2)
                );
                assert_eq!(
                    refresher2.takes(),
                    3,
                    "width {width}: one take_result per iteration"
                );

                let after = snapshot(root);
                assert_eq!(
                    before, after,
                    "width {width}: a full live run must leave the change tree byte-identical"
                );

                // Discriminating control: the same comparison must fail when
                // a single byte of alpha's tasks.md is rewritten between two
                // further snapshots, or the equality assertion above proves
                // nothing.
                let tasks_path = root.join("openspec/changes/alpha/tasks.md");
                let original = std::fs::read_to_string(&tasks_path).expect("read tasks.md back");
                let control_before = snapshot(root);
                let mutated = original.replacen("[ ] e", "[x] e", 1);
                write_with_mode(&tasks_path, mutated.as_bytes(), 0o644);
                let control_after = snapshot(root);
                assert_ne!(
                    control_before, control_after,
                    "width {width}: the snapshot comparison must discriminate a rewritten byte"
                );
                write_with_mode(&tasks_path, original.as_bytes(), 0o644);
            }
        }
    }
}
