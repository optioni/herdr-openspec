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
use std::time::Duration;

use ratatui::Terminal;
use ratatui::backend::Backend;

use crate::config::Config;
use crate::ui::app::{ArtifactReader, Dashboard, Detail, Route};
use crate::ui::driver::{LoopError, TICK};
use crate::ui::event::{CrosstermEvents, EventSource};
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

/// This pane's starting point: the working directory to search from, the
/// loaded configuration, the `herdr` program to poll, and the plugin's
/// state directory — a parameter rather than the bare program name written
/// down, precisely so a test drives [`run_wired`] against a scratch
/// `#!/bin/sh` program and a scratch state directory. Bundled into one
/// struct for cohesion, the same way [`driver::Live`] bundles the loop's
/// three collaborators: the four fields are one concept, "where this pane
/// starts" — not clippy's `too_many_arguments`. A landed doc comment once
/// claimed the flattened form would sit just under that lint's threshold;
/// this change is the one that would actually trip it, since flattening
/// `Startup`'s four fields into [`run_wired`]'s five parameters gives eight,
/// and eight is measured, on this crate and toolchain, to be exactly where
/// the lint fires. Cohesion is the real reason for the struct, on both
/// sides of that threshold.
pub struct Startup<'a> {
    pub cwd: &'a Path,
    pub config: &'a Config,
    pub herdr: &'a Path,
    /// The plugin's state directory, resolved once by `run` and passed in as
    /// a parameter — `agent-attribution`'s addition, on exactly `herdr`'s
    /// terms — so `load` reads the agent-name mapping from a directory a
    /// test can point at a scratch tree without touching the process
    /// environment. `None` is an ordinary case: no directory could be
    /// resolved, and `load` yields an empty mapping.
    pub state_dir: Option<&'a Path>,
    /// `degraded-states`' addition (design.md -> Decision 14): the environment lookup the
    /// binary probe's `PATH` and nvm steps read, injected on exactly `state_dir`'s terms so
    /// a test drives the case where nothing resolves without touching the real `PATH`. `run`
    /// passes `config::env_lookup()`.
    pub env: &'a dyn Fn(&str) -> Option<String>,
    /// The probe's fourth-step hook, injected on the same terms as `env`. `run` passes
    /// `cli::npm_probe_hook` — a wrapper around the real fourth-step binding, exposed under
    /// a name the `NOCLI-SHELL` gate does not forbid, following `agent_cli_via`'s and
    /// `HERDR_PROGRAM`'s established pattern: the dashboard shell reaches a production CLI
    /// binding only through a name that says nothing about the CLI seam itself.
    pub npm_hook: &'a dyn Fn() -> Option<std::path::PathBuf>,
}

/// The live tier's three collaborators, plus any problem folded in while
/// starting them (a watcher that would not start, for instance) —
/// everything [`start_collaborators`] produces for [`run_wired`] to wire
/// into a [`driver::Live`].
pub struct Collaborators {
    pub fs: Box<dyn crate::watch::FsEvents>,
    pub refresher: Box<dyn crate::refresh::Refresher>,
    pub agents: Box<dyn crate::agents::AgentPoll>,
    /// `agent-launch`'s addition: the fourth collaborator, reached only through this trait
    /// object.
    pub launcher: Box<dyn crate::launch::Launcher>,
    pub problems: Vec<String>,
    /// `degraded-states`' addition: whether the binary probe resolved nothing, so
    /// `run_wired` can set `Dashboard::file_mode`.
    pub file_mode: bool,
}

/// Build the one-entry `PATH` overlay for the resolved `openspec` binary: `bin_parent`
/// followed by the inherited `PATH`, read through the injected `env` lookup — never the
/// real process environment directly. See
/// `openspec/changes/seam-resilience/design.md` -> Decision 2 and
/// `specs/refresh-worker/spec.md` -> "The resolved `openspec` binary is spawned in an
/// environment where its interpreter resolves". When `PATH` is not inherited at all, the
/// overlay is `bin_parent` alone, with no trailing separator. A pure function, deliberately:
/// it derives nothing from the filesystem and decides nothing about which probe step found
/// the binary — the same rule applies uniformly to all four.
fn openspec_path_overlay(
    bin_parent: &Path,
    env: &dyn Fn(&str) -> Option<String>,
) -> Vec<(String, String)> {
    let parent = bin_parent.display().to_string();
    let value = match env("PATH") {
        Some(inherited) => format!("{parent}:{inherited}"),
        None => parent,
    };
    vec![("PATH".to_string(), value)]
}

/// Start the live tier's collaborators for `repo`. The watcher and the
/// worker are about a repository, and are the inert doubles when none was
/// found; the poller is about the Herdr session — unrelated to any
/// repository — and is started **unconditionally**: `agent-launch` reads
/// `reachable` to decide whether to offer its keys in a pane that never
/// found one.
///
/// The launcher is about a repository too, on exactly the watcher's and the worker's terms:
/// `state_dir` is threaded into `launch::start` for the state directory a launched agent's
/// name is recorded under, and `repo` decides between the real launcher and `launch::none()`
/// — a pane with no repository has no change to launch onto and no badge to focus, so the
/// inert double costs nothing and represents the truth.
///
/// `degraded-states`' addition: `env` and `npm_hook` reach the binary probe
/// (`resolve::openspec_bin`) directly rather than through `cli::worker_cli_from_env`, which
/// hardcodes the real environment and the real `npm`. `Collaborators::problems` folds three
/// standing-condition sources in causal order (design.md -> Decision 4): the configuration's
/// own fallbacks lead, then the probe's, then the watcher's — the reader meets them in the
/// order they actually happened.
///
/// `seam-resilience`'s addition (design.md -> Decision 2, S7/S10): `repo` — the same
/// canonical root already handed to `refresh::start` and `watch::start` — is now also the
/// `openspec` child's working directory, and a one-entry `PATH` overlay is built from the
/// resolved binary's own parent directory and the inherited `PATH`, read through `env`.
/// Both reach the real CLI implementation through `cli::worker_cli`. Applied for every
/// probe step, not only the ones the reference machine's defect happened to reach:
/// prepending a directory that already holds the resolved binary is a no-op for a binary
/// found on `PATH`.
pub fn start_collaborators(
    repo: Option<&Path>,
    config: &Config,
    herdr: &Path,
    state_dir: Option<&Path>,
    env: &dyn Fn(&str) -> Option<String>,
    npm_hook: &dyn Fn() -> Option<std::path::PathBuf>,
) -> Collaborators {
    let mut problems = config.problems.clone();

    let resolution = crate::resolve::openspec_bin(config.openspec_bin.as_deref(), env, npm_hook);
    let overlay: Vec<(String, String)> = resolution
        .found
        .as_ref()
        .and_then(|found| found.path.parent())
        .map(|parent| openspec_path_overlay(parent, env))
        .unwrap_or_default();
    let (cli, bin_problems) = crate::cli::worker_cli(resolution, repo, &overlay);
    let file_mode = cli.is_none();
    problems.extend(bin_problems);

    // `seam-resilience`'s addition (design.md -> Decision 8): the watch is narrowed to
    // `<repo>/openspec`, never the repository root — `watch::start` itself is unchanged,
    // and stays ignorant of the repository's own layout; the join happens here, at the
    // composition root, exactly once.
    let (fs, watch_problems) = match repo {
        Some(root) => crate::watch::start(&root.join("openspec")),
        None => (crate::watch::none(), Vec::new()),
    };
    problems.extend(watch_problems);

    let refresher = crate::refresh::start(repo, cli, config.archived_count);
    let agents = crate::agents::start(crate::cli::agent_cli_via(herdr));
    // `agent-launch`: the launcher, on the watcher's and the worker's terms rather than the
    // poller's — every launch argument vector carries the repository root as `--cwd`, and a
    // pane with no repository has no change to launch onto and no badge to focus, so the
    // inert double costs nothing and represents the truth.
    let launcher = match repo {
        Some(root) => crate::launch::start(
            crate::cli::agent_cli_via(herdr),
            root.to_path_buf(),
            config.agent_kind.clone(),
            state_dir.map(Path::to_path_buf),
        ),
        None => crate::launch::none(),
    };
    Collaborators {
        fs,
        refresher,
        agents,
        launcher,
        problems,
        file_mode,
    }
}

/// Everything `run` does once a terminal exists: load startup state, start
/// the live tier's collaborators, fold any starting problem into
/// `dashboard.refresh.problems`, build the loop's `Live`, and run it to
/// completion — returning the final `Dashboard` so a test can assert on
/// state the frame does not show.
pub fn run_wired<B: Backend, E: EventSource>(
    terminal: &mut Terminal<B>,
    events: &mut E,
    startup: &Startup<'_>,
    read: ArtifactReader<'_>,
    tick: Duration,
) -> Result<Dashboard, StartError> {
    let mut dashboard = load(startup.cwd, startup.config, startup.state_dir);
    let mut collaborators = start_collaborators(
        dashboard.repo.as_deref(),
        startup.config,
        startup.herdr,
        startup.state_dir,
        startup.env,
        startup.npm_hook,
    );
    dashboard.refresh.problems = collaborators.problems;
    dashboard.file_mode = collaborators.file_mode;
    let mut live = crate::ui::driver::Live {
        fs: &mut *collaborators.fs,
        refresher: &mut *collaborators.refresher,
        agents: &mut *collaborators.agents,
        launcher: &mut *collaborators.launcher,
    };
    driver::run_loop(terminal, &mut dashboard, events, &mut live, read, tick)?;
    Ok(dashboard)
}

/// The directory to search for a repository from, preferring the invoking workspace's own
/// cwd over the process's own OS-level working directory. `open::context`'s `workspace_id`
/// requirement is deliberately ignored here — `Err` (no Herdr context at all, or no
/// workspace cwd within it) means "fall back to the caller's own `current_dir()`", never a
/// failure of `run()` itself.
///
/// **Corrected during `plugin-actions`' group 10 live check**: the first draft of this
/// design passed `--cwd` to `herdr plugin pane open` to solve exactly this problem. Measured
/// live against Herdr 0.8.2, that call also changes what the manifest's *relative* pane
/// `command` resolves against, so a workspace directory holding no `target/release/`
/// binary of its own makes the pane fail to open at all — confirmed directly and
/// independently corroborated by `herdr-file-viewer` (installed locally), whose own
/// launcher never passes `--cwd` either and instead reads this same variable from its own
/// pane process's environment (`src/host.rs` there). Every process Herdr starts for a
/// plugin — a `[[panes]]` entry no less than an `[[actions]]` one — receives the same
/// injected context (`AGENTS.md` -> Architecture rules already documents this), so `ui::run`
/// reads it exactly where `open::context` already does, rather than `open_args` passing
/// `--cwd` at all. See `specs/pane-open/spec.md` -> "The dashboard's own starting directory
/// prefers the workspace context over the process cwd" and design.md -> Decision 6
/// (corrected).
pub(crate) fn startup_cwd(env: &dyn Fn(&str) -> Option<String>) -> Option<std::path::PathBuf> {
    crate::open::context(env)
        .ok()
        .and_then(|ctx| ctx.workspace_cwd)
        .map(std::path::PathBuf::from)
}

/// `degraded-states`' repair of `WIRED`: the decision `run`'s own body held —
/// `match startup_cwd(env) { Some(cwd) => cwd, None => std::env::current_dir()? }` — moved
/// here so both arms are driven by a test, which is the whole point of moving it: `run`'s
/// own body reaches no test, so a decision left there is a decision nothing ever runs.
/// `fallback` arrives as an **injected** closure rather than being called directly, on
/// exactly `AGENTS.md`'s rule for the environment lookup: `cargo test` runs tests in
/// parallel threads of one process, so a test that changed the real working directory would
/// corrupt its neighbours. `run` is the one caller that passes `&|| std::env::current_dir()`.
///
/// Moving the decision here rather than into `run_wired` is deliberate (design.md ->
/// Decision 5): `Startup::cwd` is a `&Path` every acceptance test and every construction site
/// already builds, so widening it to `Option<&Path>` to let `run_wired` resolve the fallback
/// would ripple through all of them to make one two-line decision testable, and would put an
/// `std::env` read inside the function whose whole purpose is to be driveable from a test.
pub(crate) fn startup_dir(
    env: &dyn Fn(&str) -> Option<String>,
    fallback: &dyn Fn() -> std::io::Result<std::path::PathBuf>,
) -> std::io::Result<std::path::PathBuf> {
    match startup_cwd(env) {
        Some(cwd) => Ok(cwd),
        None => fallback(),
    }
}

/// Start the dashboard: refuse without a terminal, install the panic hook,
/// then hand everything that can be miswired to [`run_wired`], which a test
/// drives. Holds no branch and no loop of its own beyond `?` — see the
/// `WIRED` check.
pub fn run() -> Result<(), StartError> {
    let _guard = enter_if_terminal(std::io::stdout().is_terminal(), &CrosstermOps)?;
    terminal::install_panic_hook();
    let config = crate::config::load_from_env();
    let env = crate::config::env_lookup();
    let cwd = startup_dir(&env, &|| std::env::current_dir())?;
    let mut term = Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stdout()))?;
    let state_dir = crate::state::state_dir(&env);
    let startup = Startup {
        cwd: &cwd,
        config: &config,
        herdr: Path::new(crate::cli::HERDR_PROGRAM),
        state_dir: state_dir.as_deref(),
        env: &env,
        npm_hook: &crate::cli::npm_probe_hook,
    };
    run_wired(
        &mut term,
        &mut CrosstermEvents,
        &startup,
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
pub fn load(start: &Path, config: &Config, state_dir: Option<&Path>) -> Dashboard {
    // The one further file `load` reads, on both branches below: Herdr agents exist
    // independently of an OpenSpec repository. `state::read` is infallible by
    // construction — every unusable input degrades to an empty mapping, with
    // `state::read`'s own problem string riding on `Mapping::problems` where
    // `plugin-state` put it.
    let agent_names = crate::state::read(state_dir);
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
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names,
                launch: crate::ui::app::Launch {
                    pending: None,
                    problems: Vec::new(),
                },
                file_mode: false,
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
            agents: crate::agents::AgentSnapshot {
                agents: Vec::new(),
                reachable: false,
                stalled: false,
                problem: None,
            },
            agent_names,
            launch: crate::ui::app::Launch {
                pending: None,
                problems: Vec::new(),
            },
            file_mode: false,
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

    /// `ui::startup_cwd` — added correcting group 10's live-check discovery that
    /// `--cwd` on `plugin pane open` breaks the manifest's relative command instead of
    /// solving the plugin-root problem. See its own doc comment.
    mod startup_cwd {
        fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
            let map: std::collections::BTreeMap<&str, &str> = pairs.iter().copied().collect();
            move |name| map.get(name).map(|s| s.to_string())
        }

        #[test]
        fn prefers_the_workspace_cwd_from_context() {
            let json = r#"{"workspace_id":"w8","workspace_cwd":"/repo"}"#;
            let pairs = [("HERDR_PLUGIN_CONTEXT_JSON", json)];
            assert_eq!(
                super::super::startup_cwd(&env(&pairs)),
                Some(std::path::PathBuf::from("/repo"))
            );
        }

        #[test]
        fn no_herdr_context_at_all_falls_back_to_none() {
            assert_eq!(super::super::startup_cwd(&env(&[])), None);
        }

        #[test]
        fn a_workspace_with_no_cwd_known_falls_back_to_none() {
            let pairs = [("HERDR_WORKSPACE_ID", "w8")];
            assert_eq!(super::super::startup_cwd(&env(&pairs)), None);
        }
    }

    /// `ui::startup_dir` — `degraded-states`' repair of `WIRED`: the decision `run`'s own
    /// body held before this change. Distinguished from `startup_cwd` above: `startup_cwd`
    /// (`src/ui/mod.rs:220`, three tests) answers "what does the Herdr context say", while
    /// `startup_dir` holds the whole fallback decision `run` used to make in its own body.
    mod startup_dir {
        use std::path::PathBuf;

        fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
            let map: std::collections::BTreeMap<&str, &str> = pairs.iter().copied().collect();
            move |name| map.get(name).map(|s| s.to_string())
        }

        #[test]
        fn prefers_the_workspace_cwd_and_falls_back_when_there_is_none() {
            let counter = std::cell::Cell::new(0);
            let fallback = || {
                counter.set(counter.get() + 1);
                Ok(PathBuf::from("/tmp/never-used"))
            };
            let json = r#"{"workspace_id":"w8","workspace_cwd":"/tmp/workspace-a"}"#;
            let pairs = [("HERDR_PLUGIN_CONTEXT_JSON", json)];
            let got = super::super::startup_dir(&env(&pairs), &fallback);
            assert_eq!(got.unwrap(), PathBuf::from("/tmp/workspace-a"));
            assert_eq!(
                counter.get(),
                0,
                "the fallback closure must not be called when the workspace cwd resolves"
            );

            let got = super::super::startup_dir(&env(&[]), &fallback);
            assert_eq!(got.unwrap(), PathBuf::from("/tmp/never-used"));
            assert_eq!(counter.get(), 1, "both arms must be driven");
        }

        #[test]
        fn propagates_a_failing_fallback_rather_than_panicking() {
            let fallback = || Err(std::io::Error::from(std::io::ErrorKind::NotFound));
            let got = super::super::startup_dir(&env(&[]), &fallback);
            let err = got.expect_err("a failing fallback must be propagated, not substituted");
            assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
        }

        #[test]
        fn a_herdr_context_with_no_workspace_cwd_is_the_fallback_case_not_a_failure() {
            let fallback = || Ok(PathBuf::from("/tmp/never-used"));

            let no_cwd = [("HERDR_PLUGIN_CONTEXT_JSON", r#"{"workspace_id":"w8"}"#)];
            assert_eq!(
                super::super::startup_dir(&env(&no_cwd), &fallback).unwrap(),
                PathBuf::from("/tmp/never-used")
            );

            let unparseable = [("HERDR_PLUGIN_CONTEXT_JSON", "not json at all")];
            assert_eq!(
                super::super::startup_dir(&env(&unparseable), &fallback).unwrap(),
                PathBuf::from("/tmp/never-used")
            );
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
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    problems: Vec::new(),
                },
                file_mode: false,
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
                let mut agents = crate::agents::none();
                let mut launcher = crate::launch::none();
                let mut live = crate::ui::driver::Live {
                    fs: &mut *fs,
                    refresher: &mut *refresher,
                    agents: &mut *agents,
                    launcher: &mut *launcher,
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

        /// `artifact-content` :: "The line count drives the scroll clamp" —
        /// `degraded-states`' addition. Twenty of the selected change's own `problems`, no
        /// markdown source at all, forty `j` presses (well past the six-line overshoot the
        /// fourteen-row content area allows), on exactly
        /// `a_markdown_document_renders_and_scrolls_through_the_loop`'s terms: the scroll
        /// clamp must be driven by `content_lines`' full returned length — problem lines
        /// included — not merely by the markdown body's.
        #[test]
        fn change_problems_are_inside_the_scrolled_region() {
            for width in [120u16, 60u16] {
                let twenty_problems: Vec<String> = (0..20).map(|i| format!("p-{i:02}")).collect();
                let change = crate::changes::fixture::with_problems(
                    crate::changes::fixture::with_artifacts(
                        crate::changes::fixture::active("detail-view", 4, 9),
                        &[("proposal", &["/repo/p.md"])],
                    ),
                    twenty_problems,
                );
                let mut dashboard = Dashboard {
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
                    agents: crate::agents::AgentSnapshot {
                        agents: Vec::new(),
                        reachable: false,
                        stalled: false,
                        problem: None,
                    },
                    agent_names: crate::state::Mapping::default(),
                    launch: crate::ui::app::Launch {
                        pending: None,
                        problems: Vec::new(),
                    },
                    file_mode: false,
                };
                let interior = detail_interior(width, 20, Route::Detail);
                let content_y = interior.y + 2;
                let content_height = interior.height - 2;

                let backend = ratatui::backend::TestBackend::new(width, 20);
                let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
                let mut presses: Vec<_> = (0..40)
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

                // The artifact itself reads empty — every line on screen comes from
                // `change.problems`, none from a markdown body, so this test cannot pass by
                // accident of the body's own line count.
                let recorder = RecordingReader::always(Ok(String::new()));
                let read = |p: &std::path::Path| recorder.read(p);

                let mut fs = crate::watch::none();
                let mut refresher = crate::refresh::none();
                let mut agents = crate::agents::none();
                let mut launcher = crate::launch::none();
                let mut live = crate::ui::driver::Live {
                    fs: &mut *fs,
                    refresher: &mut *refresher,
                    agents: &mut *agents,
                    launcher: &mut *launcher,
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
                let row_at = |y: u16| -> String {
                    (interior.x..interior.x + 6)
                        .map(|x| buf[(x, y)].symbol().to_string())
                        .collect()
                };
                assert_eq!(row_at(content_y), "! p-06", "width {width}");
                assert_eq!(
                    row_at(content_y + content_height - 1),
                    "! p-19",
                    "width {width}"
                );
                assert_eq!(
                    dashboard.detail.scroll, 6,
                    "width {width}: the clamp must be driven by the full 20-line \
                     content_lines() output, problem lines included"
                );
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
                    agents: crate::agents::AgentSnapshot {
                        agents: Vec::new(),
                        reachable: false,
                        stalled: false,
                        problem: None,
                    },
                    agent_names: crate::state::Mapping::default(),
                    launch: crate::ui::app::Launch {
                        pending: None,
                        problems: Vec::new(),
                    },
                    file_mode: false,
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
                let mut agents = crate::agents::none();
                let mut launcher = crate::launch::none();
                let mut live = crate::ui::driver::Live {
                    fs: &mut *fs,
                    refresher: &mut *refresher,
                    agents: &mut *agents,
                    launcher: &mut *launcher,
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
                let mut dashboard = super::super::load(root, &config, None);

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
                let mut agents = crate::agents::none();
                let mut launcher = crate::launch::none();
                let mut live = crate::ui::driver::Live {
                    fs: &mut *fs,
                    refresher: &mut *refresher,
                    agents: &mut *agents,
                    launcher: &mut *launcher,
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
                let mut agents = crate::agents::none();
                let mut launcher = crate::launch::none();
                let mut live = crate::ui::driver::Live {
                    fs: &mut *fs,
                    refresher: &mut *refresher,
                    agents: &mut *agents,
                    launcher: &mut *launcher,
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

                // `agent-launch`: `a`, `c`, `s`, and `g` reach `launch::decide` for real, over
                // a fresh dashboard and a recording launcher, driven **before** the sweep
                // above would otherwise carry them into filter mode as query characters (the
                // sweep's own `/` switches into filtering, and every character after it —
                // including every lower-case letter — types into the query rather than
                // launching, on `list-filtering`'s own terms). The snapshot claim is worth
                // nothing if the keys did nothing.
                let mut launch_dashboard = super::super::load(root, &config, None);
                launch_dashboard.agents.reachable = true;
                let mut launch_events = Script::new(vec![
                    Ok(Some(press(
                        ratatui::crossterm::event::KeyCode::Char('a'),
                        ratatui::crossterm::event::KeyModifiers::NONE,
                    ))),
                    Ok(Some(press(
                        ratatui::crossterm::event::KeyCode::Char('c'),
                        ratatui::crossterm::event::KeyModifiers::NONE,
                    ))),
                    Ok(Some(press(
                        ratatui::crossterm::event::KeyCode::Char('s'),
                        ratatui::crossterm::event::KeyModifiers::NONE,
                    ))),
                    Ok(Some(press(
                        ratatui::crossterm::event::KeyCode::Char('g'),
                        ratatui::crossterm::event::KeyModifiers::NONE,
                    ))),
                    Ok(Some(press(
                        ratatui::crossterm::event::KeyCode::Char('c'),
                        ratatui::crossterm::event::KeyModifiers::CONTROL,
                    ))),
                ]);
                let launch_backend = ratatui::backend::TestBackend::new(width, 20);
                let mut launch_terminal =
                    ratatui::Terminal::new(launch_backend).expect("construct terminal");
                let mut fs2 = crate::watch::none();
                let mut refresher2 = crate::refresh::none();
                let mut agents2 = crate::agents::none();
                let mut recording_launcher = crate::testutil::RecordingLauncher::new();
                let mut live2 = crate::ui::driver::Live {
                    fs: &mut *fs2,
                    refresher: &mut *refresher2,
                    agents: &mut *agents2,
                    launcher: &mut recording_launcher,
                };
                run_loop(
                    &mut launch_terminal,
                    &mut launch_dashboard,
                    &mut launch_events,
                    &mut live2,
                    &crate::ui::read_artifact,
                    std::time::Duration::from_millis(1),
                )
                .expect("the launch-keys run ends");
                assert!(
                    !recording_launcher.requests().is_empty(),
                    "width {width}: the recording launcher must have received requests from \
                     the a/c/s/g presses"
                );
                let after_launch_keys = crate::testutil::snapshot(root);
                assert_eq!(
                    before, after_launch_keys,
                    "width {width}: the tree must still be byte-identical after a/c/s/g \
                     genuinely reached decide"
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

        /// `dashboard-loop` :: "`file_mode` is set by the composition root and by nothing
        /// else" — the unit half. `ui::load` never claims either value; `file_mode` is
        /// `false` on both branches, and it is `run_wired` (via `start_collaborators`'s real
        /// probe) that assigns the true value afterward. See
        /// `mod wiring::run_wired_sets_file_mode_from_the_probe` for the outer half.
        #[test]
        fn load_never_claims_file_mode() {
            let scratch = ScratchDir::new();
            let root = scratch.path();
            write(&root.join("openspec/changes/alpha/proposal.md"), "# P\n");
            write(
                &root.join("openspec/changes/alpha/tasks.md"),
                "- [x] a\n- [ ] b\n",
            );
            let found = super::super::load(root, &Config::default(), None);
            assert!(
                !found.file_mode,
                "a repository being found must not itself claim file_mode"
            );

            let empty = ScratchDir::new();
            let not_found = super::super::load(empty.path(), &Config::default(), None);
            assert!(
                !not_found.file_mode,
                "no repository found is unrelated to file_mode"
            );
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

            let dashboard = super::super::load(&start, &config_with_archived_count(5), None);

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
            assert!(
                dashboard.agent_names.names.is_empty(),
                "agent-attribution: a None state_dir must be an empty mapping"
            );
            assert!(dashboard.agent_names.problems.is_empty());
        }

        /// `dashboard-loop`: "The startup request is issued before the first wait" extended
        /// by `agent-launch` to name the initial launch tier — on both branches of `load`.
        #[test]
        fn load_initialises_an_empty_launch_tier() {
            let scratch = ScratchDir::new();
            let root = scratch.path();
            write(&root.join("openspec/changes/alpha/proposal.md"), "# P\n");
            write(
                &root.join("openspec/changes/alpha/tasks.md"),
                "- [x] a\n- [ ] b\n",
            );
            let found = super::super::load(root, &Config::default(), None);
            assert_eq!(
                found.launch,
                crate::ui::app::Launch {
                    pending: None,
                    problems: Vec::new(),
                }
            );

            let empty = ScratchDir::new();
            let not_found = super::super::load(empty.path(), &Config::default(), None);
            assert_eq!(
                not_found.launch,
                crate::ui::app::Launch {
                    pending: None,
                    problems: Vec::new(),
                }
            );
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

            let dashboard = super::super::load(start, &config_with_archived_count(5), None);

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

            let three = super::super::load(root, &config_with_archived_count(3), None);
            assert_eq!(three.changes.archived.len(), 3);

            let seven = super::super::load(root, &config_with_archived_count(7), None);
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

            let dashboard = super::super::load(root, &config_with_archived_count(5), None);

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
            let state = ScratchDir::new();
            write(
                &state.path().join("agent-names.toml"),
                "[names]\nc-2fa-support = \"2fa-support\"\n",
            );

            let before = snapshot(root);
            let state_before = snapshot(state.path());
            let _ = super::super::load(root, &config_with_archived_count(5), Some(state.path()));
            let after = snapshot(root);
            let state_after = snapshot(state.path());
            assert_eq!(before, after, "ui::load wrote inside the repository");
            assert_eq!(
                state_before, state_after,
                "agent-attribution: ui::load reads the mapping and must never write it"
            );
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
            let mut dashboard = super::super::load(root, &config_with_archived_count(5), None);
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
            // `degraded-states`: a config-less repository's default schema
            // (`spec-driven`) is genuinely not vendored here, which now renders as the
            // change's own leading problem line (this change's whole point) and would make
            // this test's "no content yet" assertion fail for an unrelated reason. Vendoring
            // a minimal schema keeps this test about what it was always about — startup
            // state before the first read — rather than about group 5's new behaviour.
            write(
                &root.join("openspec/schemas/tdd/schema.yaml"),
                "name: tdd\nartifacts:\n  - id: proposal\n    generates: proposal.md\napply:\n  tracks: proposal.md\n",
            );
            write(&root.join("openspec/config.yaml"), "schema: tdd\n");
            write(&root.join("openspec/changes/alpha/proposal.md"), "# P\n");

            let found = super::super::load(root, &config_with_archived_count(5), None);
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
            let not_found =
                super::super::load(scratch2.path(), &config_with_archived_count(5), None);
            assert_eq!(not_found.repo, None);
            assert_eq!(not_found.detail.source, "");
            assert_eq!(not_found.detail.scroll, 0);
            assert!(
                !not_found.refresh.reload,
                "live-refresh: startup must not force a reload"
            );
            assert_detail_blank_at_120(&not_found);
        }

        /// `dashboard-loop`: "`load` reads the agent-name mapping from the directory it
        /// was given" — real scratch directories, both for the mapping being observable
        /// through the field and, via `attribution()`, through what it actually does.
        #[test]
        fn load_reads_the_agent_name_mapping() {
            let scratch = ScratchDir::new();
            let root = scratch.path();
            write(
                &root.join("openspec/changes/2fa-support/proposal.md"),
                "# P\n",
            );

            let state = ScratchDir::new();
            write(
                &state.path().join("agent-names.toml"),
                "[names]\nc-2fa-support = \"2fa-support\"\n",
            );

            let with_state =
                super::super::load(root, &config_with_archived_count(5), Some(state.path()));
            assert_eq!(
                with_state.agent_names.names,
                std::collections::BTreeMap::from([(
                    "c-2fa-support".to_string(),
                    "2fa-support".to_string()
                )])
            );
            assert!(with_state.agent_names.problems.is_empty());

            let without_state = super::super::load(root, &config_with_archived_count(5), None);
            assert!(
                without_state.agent_names.names.is_empty(),
                "the pair came from the directory, not from anywhere else"
            );

            // The mapping reaching the dashboard is observable in the attribution, not
            // only in the field: an in-scope agent named `c-2fa-support` badges
            // `2fa-support` on the first result, and neither on the second.
            let mut attributed = with_state.clone();
            attributed.agents.agents = vec![crate::agents::Agent {
                name: Some("c-2fa-support".to_string()),
                kind: None,
                status: crate::agents::AgentStatus::Working,
                cwd: with_state.repo.clone(),
                pane_id: "p".to_string(),
                tab_id: "t".to_string(),
                workspace_id: "w".to_string(),
                terminal_title: None,
            }];
            let attribution = attributed.attribution();
            assert_eq!(
                attribution.badges,
                std::collections::BTreeMap::from([(
                    "2fa-support".to_string(),
                    crate::agents::AgentStatus::Working
                )])
            );

            let mut unattributed = without_state.clone();
            unattributed.agents.agents = attributed.agents.agents.clone();
            assert!(unattributed.attribution().badges.is_empty());
        }

        /// `dashboard-loop`: "An unusable mapping file is an empty mapping with a named
        /// problem" — a malformed `agent-names.toml`, and the rest of `Dashboard`
        /// otherwise exactly what a well-formed mapping produces.
        #[test]
        fn a_none_state_dir_is_an_empty_mapping() {
            let scratch = ScratchDir::new();
            let root = scratch.path();
            write(&root.join("openspec/changes/alpha/proposal.md"), "# P\n");

            let none_result = super::super::load(root, &config_with_archived_count(5), None);
            assert!(none_result.agent_names.names.is_empty());
            assert!(none_result.agent_names.problems.is_empty());

            let malformed = ScratchDir::new();
            write(&malformed.path().join("agent-names.toml"), "[names");
            let malformed_result =
                super::super::load(root, &config_with_archived_count(5), Some(malformed.path()));
            assert!(malformed_result.agent_names.names.is_empty());
            assert_eq!(malformed_result.agent_names.problems.len(), 1);
            assert!(
                malformed_result.agent_names.problems[0].contains("agent-names.toml"),
                "{:?}",
                malformed_result.agent_names.problems
            );

            // An unusable mapping degrades the badge tier and nothing else: every other
            // field is exactly what the same call produces with a well-formed mapping.
            assert_eq!(malformed_result.repo, none_result.repo);
            assert_eq!(malformed_result.changes, none_result.changes);
            assert_eq!(malformed_result.route, none_result.route);
            assert_eq!(malformed_result.selected, none_result.selected);
            assert_eq!(malformed_result.filter, none_result.filter);
            assert_eq!(malformed_result.detail, none_result.detail);
            assert_eq!(malformed_result.refresh, none_result.refresh);
            assert_eq!(malformed_result.agents, none_result.agents);
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
            RecordingRefresher, Script, ScriptedFs, press, row_text, snapshot, write_with_mode,
        };
        use crate::ui::driver::{LoopSummary, run_loop};

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
                let mut dashboard = super::super::load(root, &config, None);

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
                let mut agents = crate::agents::none();
                let mut launcher = crate::launch::none();
                let mut live = crate::ui::driver::Live {
                    fs: &mut fs,
                    refresher: &mut refresher,
                    agents: &mut *agents,
                    launcher: &mut *launcher,
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
                let mut agents2 = crate::agents::none();
                let mut launcher = crate::launch::none();
                let mut live2 = crate::ui::driver::Live {
                    fs: &mut fs2,
                    refresher: &mut refresher2,
                    agents: &mut *agents2,
                    launcher: &mut *launcher,
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

        /// `r`, then every ASCII printable character from `!` to `~`, then
        /// `Enter`, `Esc`, `Backspace`, `Tab`, the four arrows, and finally
        /// `Ctrl-C` — `tasks-checklist`'s read-only sweep, with an explicit
        /// `r` press first. `/` occurs partway through the `!..=~` sweep, so
        /// filter mode is active by the time the sweep's own `r` character
        /// comes around; that later `r` types into the query rather than
        /// refreshing again, which is what keeps the total at exactly one
        /// `r`-driven request.
        fn key_script()
        -> Vec<Result<Option<ratatui::crossterm::event::Event>, crate::ui::event::EventError>>
        {
            let mut presses = vec![Ok(Some(press(
                ratatui::crossterm::event::KeyCode::Char('r'),
                ratatui::crossterm::event::KeyModifiers::NONE,
            )))];
            presses.extend(('!'..='~').map(|c| {
                Ok(Some(press(
                    ratatui::crossterm::event::KeyCode::Char(c),
                    ratatui::crossterm::event::KeyModifiers::NONE,
                )))
            }));
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
            presses
        }

        #[test]
        fn r_forces_a_refresh_through_the_loop() {
            for width in [120u16, 60u16] {
                let scratch = scratch_repo_with_alpha();
                let root = scratch.path();
                let config = Config::default();
                let mut dashboard = super::super::load(root, &config, None);

                let backend = ratatui::backend::TestBackend::new(width, 20);
                let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");

                let mut events = Script::new(key_script());
                // `drain` always `Ok(None)`: this `ScriptedFs` can
                // contribute no third request under any timing, which is
                // what makes the exact equality below sound.
                let mut fs = ScriptedFs::new(Vec::new(), Vec::new());
                let mut refresher = RecordingRefresher::new(Vec::new());
                let mut agents = crate::agents::none();
                let mut launcher = crate::launch::none();
                let mut live = crate::ui::driver::Live {
                    fs: &mut fs,
                    refresher: &mut refresher,
                    agents: &mut *agents,
                    launcher: &mut *launcher,
                };

                run_loop(
                    &mut terminal,
                    &mut dashboard,
                    &mut events,
                    &mut live,
                    &super::super::read_artifact,
                    Duration::from_millis(1),
                )
                .expect("loop ends");

                assert_eq!(
                    refresher.requests(),
                    vec![Selection::All, Selection::All],
                    "width {width}: the startup request and the r-driven one, nothing else"
                );
            }
        }

        #[test]
        fn a_live_watcher_over_the_tree_writes_nothing() {
            for width in [120u16, 60u16] {
                let scratch = scratch_repo_with_alpha();
                let root = scratch.path();
                let config = Config::default();
                let mut dashboard = super::super::load(root, &config, None);

                let backend = ratatui::backend::TestBackend::new(width, 20);
                let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");

                let before = snapshot(root);

                let (mut fs, problems) = crate::watch::start(root);
                assert!(
                    problems.is_empty(),
                    "width {width}: a real watch must start on a real directory: {problems:?}"
                );
                let mut refresher = RecordingRefresher::new(Vec::new());
                let mut agents = crate::agents::none();
                let mut launcher = crate::launch::none();
                let mut live = crate::ui::driver::Live {
                    fs: &mut *fs,
                    refresher: &mut refresher,
                    agents: &mut *agents,
                    launcher: &mut *launcher,
                };

                let mut events = Script::new(key_script());
                run_loop(
                    &mut terminal,
                    &mut dashboard,
                    &mut events,
                    &mut live,
                    &super::super::read_artifact,
                    Duration::from_millis(1),
                )
                .expect("loop ends");

                let after = snapshot(root);
                assert_eq!(
                    before, after,
                    "width {width}: a live watcher open on the tree must write nothing"
                );

                let tasks_path = root.join("openspec/changes/alpha/tasks.md");
                let reread = crate::tasks::read(&tasks_path);
                assert_eq!(
                    reread.progress(),
                    crate::tasks::Progress {
                        completed: 4,
                        total: 9
                    },
                    "width {width}"
                );

                // Discriminating control: the same comparison must fail
                // when a single byte of tasks.md is rewritten between two
                // further snapshots, or the equality assertion above proves
                // nothing.
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
                drop(fs); // stop the watch before the ScratchDir's own Drop removes the tree
            }
        }
    }

    /// The outer-loop acceptance test — see design.md -> Test Strategy. Drives
    /// `ui::run_wired`, the composition root itself, with the real `ui::load`,
    /// the real `watch::start`, the real `refresh::start`, the real
    /// `agents::start`, and the real `ui::read_artifact`, against a scratch
    /// repository and two scratch programs. RED from group 2 — group 1's
    /// `start_collaborators` hardcodes the inert `agents::none()` poller, so
    /// every scenario here fails on `agents.reachable` being `false` — until
    /// group 10 wires `agents::start` for real. See design.md -> Decisions 13
    /// for why `make check` is not run unqualified across that span.
    mod wiring {
        use std::path::{Path, PathBuf};
        use std::time::Duration;

        use crate::config::Config;
        use crate::testutil::{
            ScratchDir, UntilReady, canonical, render_at, row_text, snapshot, write_with_mode,
        };
        use crate::ui::app::{Dashboard, Route};
        use crate::ui::{StartError, Startup};

        /// Drive `run_wired` at `width`x20 over a real `TestBackend`, with
        /// `startup.cwd` at `root`, the real `ui::read_artifact`, and a
        /// `testutil::UntilReady` event source built from `predicate`.
        /// Returns the result plus every drawn row as a `String`, so a caller
        /// can search the buffer without repeating the row-reading dance.
        /// The neutral environment `run_wired_at` and `run_wired_staged` inject by default: no
        /// `PATH`, no `HOME`, no `NVM_DIR` — so the binary probe's `PATH` and nvm steps never
        /// resolve anything, on design.md -> Decision 14's terms. Every test in this module
        /// that needs a specific probe outcome either configures `Config::openspec_bin`
        /// directly (step 1, which does not consult `env` at all) or drives the probe through
        /// `run_wired_probed` below, which takes its own `env`/`npm_hook`.
        fn no_env(_: &str) -> Option<String> {
            None
        }

        fn no_npm_hook() -> Option<PathBuf> {
            None
        }

        fn run_wired_at(
            width: u16,
            root: &Path,
            config: &Config,
            herdr: &Path,
            state_dir: Option<&Path>,
            predicate: &dyn Fn() -> bool,
        ) -> (Result<Dashboard, StartError>, Vec<String>) {
            let backend = ratatui::backend::TestBackend::new(width, 20);
            let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
            let mut events = UntilReady::new(predicate);
            let startup = Startup {
                cwd: root,
                config,
                herdr,
                state_dir,
                env: &no_env,
                npm_hook: &no_npm_hook,
            };
            let result = super::super::run_wired(
                &mut terminal,
                &mut events,
                &startup,
                &crate::ui::read_artifact,
                Duration::from_millis(1),
            );
            let buf = terminal.backend().buffer().clone();
            let rows: Vec<String> = (0..buf.area.height)
                .map(|y| {
                    (0..buf.area.width)
                        .map(|x| buf[(x, y)].symbol().to_string())
                        .collect::<String>()
                })
                .collect();
            (result, rows)
        }

        /// `run_wired_at`'s parameters, bundled into one struct rather than widened past
        /// seven positional arguments (`clippy::too_many_arguments` fires at eight) — used
        /// only by the file-mode/probe scenarios below that need a non-neutral `env` or
        /// `npm_hook`, so `run_wired_at` itself stays untouched for every other test in this
        /// module.
        struct ProbedStartup<'a> {
            width: u16,
            root: &'a Path,
            config: &'a Config,
            herdr: &'a Path,
            state_dir: Option<&'a Path>,
            env: &'a dyn Fn(&str) -> Option<String>,
            npm_hook: &'a dyn Fn() -> Option<PathBuf>,
        }

        /// `run_wired_at`'s twin, driving `run_wired` with an explicit `env`/`npm_hook` — see
        /// design.md -> Decision 14. `p.env`/`p.npm_hook` are what let a test drive the binary
        /// probe's failing and resolving cases without touching the real `PATH` or spawning
        /// the real `npm`.
        fn run_wired_probed(
            p: ProbedStartup<'_>,
            predicate: &dyn Fn() -> bool,
        ) -> (Result<Dashboard, StartError>, Vec<String>) {
            let backend = ratatui::backend::TestBackend::new(p.width, 20);
            let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
            let mut events = UntilReady::new(predicate);
            let startup = Startup {
                cwd: p.root,
                config: p.config,
                herdr: p.herdr,
                state_dir: p.state_dir,
                env: p.env,
                npm_hook: p.npm_hook,
            };
            let result = super::super::run_wired(
                &mut terminal,
                &mut events,
                &startup,
                &crate::ui::read_artifact,
                Duration::from_millis(1),
            );
            let buf = terminal.backend().buffer().clone();
            let rows: Vec<String> = (0..buf.area.height)
                .map(|y| {
                    (0..buf.area.width)
                        .map(|x| buf[(x, y)].symbol().to_string())
                        .collect::<String>()
                })
                .collect();
            (result, rows)
        }

        fn vendor_tdd_schema(repo: &Path) {
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
        fn scratch_repo_with_alpha() -> ScratchDir {
            let scratch = ScratchDir::new();
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

        /// Write a scratch `#!/bin/sh` program under `<dir>/bin/<name>` — a
        /// `bin/` subdirectory rather than `dir` itself, since the scratch
        /// repository's own `openspec/` directory already lives at `dir` and
        /// a program literally named `openspec` would collide with it.
        fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
            let path = dir.join("bin").join(name);
            write_with_mode(&path, format!("#!/bin/sh\n{body}").as_bytes(), 0o755);
            path
        }

        /// A scratch `herdr` program: appends its arguments to `log`, one line
        /// per invocation, then prints the reference one-agent `agent_list`
        /// envelope captured verbatim from Herdr 0.8.2 — see
        /// `specs/agent-list/spec.md` -> "The reference payload parses into
        /// one agent".
        fn herdr_script(dir: &Path, log: &Path) -> PathBuf {
            write_script(
                dir,
                "herdr",
                &format!(
                    "printf '%s\\n' \"$*\" >> \"{log}\"\n\
                     printf '%s' '{{\"id\":\"cli:agent:list\",\"result\":{{\"agents\":[{{\"agent\":\"claude\",\"agent_session\":{{\"agent\":\"claude\",\"kind\":\"id\",\"source\":\"herdr:claude\",\"value\":\"0e80c276-952e-4150-b32f-06cc6247ce01\"}},\"agent_status\":\"idle\",\"cwd\":\"/repo\",\"name\":\"alpha\",\"focused\":true,\"foreground_cwd\":\"/repo\",\"pane_id\":\"w8:p1\",\"revision\":35,\"state_change_seq\":963,\"tab_id\":\"w8:t1\",\"terminal_id\":\"term_65a34df386c314\",\"terminal_title\":\"\u{2733} a title\",\"terminal_title_stripped\":\"a title\",\"workspace_id\":\"w8\"}}],\"type\":\"agent_list\"}}}}'\n",
                    log = log.display(),
                ),
            )
        }

        /// One `agent list` JSON entry. `name` `None` omits the field entirely, matching
        /// Herdr's own behaviour for an agent nobody has renamed; `cwd` `None` omits it
        /// too, matching an agent whose working directory Herdr does not report. `index`
        /// gives each entry a distinct `pane_id`/`tab_id`, two of the three of Herdr's
        /// seven required fields this crate reads.
        fn agent_entry(
            index: usize,
            name: Option<&str>,
            status: &str,
            cwd: Option<&str>,
        ) -> String {
            let mut entry = format!(
                r#"{{"agent":"claude","agent_status":"{status}","pane_id":"w8:p{index}","tab_id":"w8:t{index}","workspace_id":"w8""#
            );
            if let Some(name) = name {
                entry.push_str(&format!(r#","name":"{name}""#));
            }
            if let Some(cwd) = cwd {
                entry.push_str(&format!(r#","cwd":"{cwd}""#));
            }
            entry.push('}');
            entry
        }

        /// `agent-attribution`'s generalisation of `herdr_script`: a scratch `herdr`
        /// program that appends its arguments to `log`, one line per invocation, then
        /// prints the `agent_list` envelope built from `entries` — parameterised on the
        /// agent list it prints, per design.md -> Test Strategy, rather than always the
        /// reference single agent `herdr_script` prints.
        fn herdr_script_for(dir: &Path, log: &Path, entries: &[String]) -> PathBuf {
            let agents_json = entries.join(",");
            write_script(
                dir,
                "herdr",
                &format!(
                    "printf '%s\\n' \"$*\" >> \"{log}\"\n\
                     printf '%s' '{{\"id\":\"cli:agent:list\",\"result\":{{\"agents\":[{agents_json}],\"type\":\"agent_list\"}}}}'\n",
                    log = log.display(),
                ),
            )
        }

        /// A scratch `openspec` program: appends its arguments to `log`, one
        /// line per invocation, and answers `list --json` with an empty
        /// change list whose `root.path` agrees with `root` — enough for the
        /// worker's own CLI-merge step to accept it without a single
        /// `instructions apply` call, which this test does not need.
        fn openspec_script(dir: &Path, log: &Path, root: &Path) -> PathBuf {
            write_script(
                dir,
                "openspec",
                &format!(
                    "printf '%s\\n' \"$*\" >> \"{log}\"\n\
                     printf '%s' '{{\"changes\":[],\"root\":{{\"path\":\"{root}\",\"source\":\"nearest\"}}}}'\n",
                    log = log.display(),
                    root = root.display(),
                ),
            )
        }

        /// `degraded-coverage`'s two CLI-cycle failure fixtures: a scratch `openspec`
        /// program that either reports a repository root disagreeing with `root`, or exits
        /// non-zero — both after logging its own argv, on `openspec_script`'s terms.
        fn openspec_script_failing(dir: &Path, log: &Path, kind: &str) -> PathBuf {
            let body = match kind {
                "wrong_root" => "printf '%s' '{\"changes\":[],\"root\":{\"path\":\"/definitely/elsewhere\",\"source\":\"nearest\"}}'\n".to_string(),
                "nonzero" => "exit 1\n".to_string(),
                other => unreachable!("unexpected kind {other}"),
            };
            write_script(
                dir,
                "openspec",
                &format!(
                    "printf '%s\\n' \"$*\" >> \"{log}\"\n{body}",
                    log = log.display(),
                ),
            )
        }

        /// The number of lines a scratch program's log currently holds — `0`
        /// when the log does not exist yet, so a predicate can poll a log no
        /// invocation has produced.
        fn log_lines(path: &Path) -> usize {
            std::fs::read_to_string(path)
                .map(|s| s.lines().count())
                .unwrap_or(0)
        }

        /// Append one byte to `path` — the discriminating write scenario
        /// 2.2's own predicate makes on purpose, since a real filesystem event
        /// is the only thing that distinguishes a real watcher from the inert
        /// double.
        fn append_one_byte(path: &Path) {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(path)
                .expect("open the file to append one byte");
            f.write_all(b"x").expect("append one byte");
        }

        // --- agent-launch: the outer-loop acceptance harness ------------------------------

        /// A scratch `herdr` program supporting the whole launch flow — `pane split`,
        /// `agent start`, `agent prompt`, `agent focus`, and `agent list` — logging every
        /// invocation to `log`, one line per call. `agent start` additionally writes `marker`,
        /// recording the derived agent name it was given; `agent list` reports that agent, at
        /// the fixed pane id `pane split` handed out and with `cwd` the canonicalized `root`,
        /// once `marker` exists, and an empty list otherwise. This is what lets the wiring test
        /// observe "the started agent reaches a later poll" through a real, if scripted, round
        /// trip rather than a shortcut.
        fn launch_herdr_script(dir: &Path, log: &Path, marker: &Path, root: &Path) -> PathBuf {
            let root = root.display().to_string();
            write_script(
                dir,
                "herdr",
                &format!(
                    r#"printf '%s\n' "$*" >> "{log}"
case "$1 $2" in
  "pane split")
    printf '%s' '{{"id":"cli:pane:split","result":{{"pane":{{"agent_status":"unknown","cwd":"{root}","pane_id":"wD:pJ","tab_id":"wD:t2","workspace_id":"wD"}},"type":"pane_info"}}}}'
    ;;
  "agent start")
    name="$3"
    printf 'name=%s\n' "$name" > "{marker}"
    printf '%s' '{{"id":"cli:agent:start","result":{{"agent":{{"name":"'"$name"'","pane_id":"wD:pJ"}},"argv":["claude"],"type":"agent_started"}}}}'
    ;;
  "agent prompt")
    printf '%s' '{{"id":"cli:agent:prompt","result":{{"agent":{{}},"type":"agent_prompted"}}}}'
    ;;
  "agent focus")
    printf '%s' '{{"id":"cli:agent:focus","result":{{"agent":{{}},"type":"agent_focused"}}}}'
    ;;
  "agent list")
    if [ -f "{marker}" ]; then
      name=$(sed -n 's/^name=//p' "{marker}")
      printf '%s' '{{"id":"cli:agent:list","result":{{"agents":[{{"agent":"claude","agent_status":"working","cwd":"{root}","name":"'"$name"'","pane_id":"wD:pJ","tab_id":"wD:t2","workspace_id":"wD"}}],"type":"agent_list"}}}}'
    else
      printf '%s' '{{"id":"cli:agent:list","result":{{"agents":[],"type":"agent_list"}}}}'
    fi
    ;;
esac
"#,
                    log = log.display(),
                    marker = marker.display(),
                    root = root,
                ),
            )
        }

        /// The number of the log's lines that are **not** `agent list` — every predicate and
        /// every ordering assertion in these tests counts only these, because the poller
        /// writes `agent list` to the same log on its own one-second cadence and an absolute
        /// count is therefore a race.
        fn non_agent_list_lines(path: &Path) -> Vec<String> {
            std::fs::read_to_string(path)
                .map(|s| {
                    s.lines()
                        .filter(|l| *l != "agent list")
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default()
        }

        /// Whether an `agent list` line appears in the log **after** the first `agent start`
        /// line — the proxy this harness uses for "the started agent now reaches a poll",
        /// since a scratch program has no way to signal that directly: our `launch_herdr_script`
        /// only reports the started agent from `agent list` once its marker file exists, and
        /// that file is written inside the same `agent start` invocation this checks for.
        fn agent_seen_in_a_later_poll(log: &Path) -> bool {
            let text = std::fs::read_to_string(log).unwrap_or_default();
            let lines: Vec<&str> = text.lines().collect();
            let Some(start_idx) = lines.iter().position(|l| l.starts_with("agent start")) else {
                return false;
            };
            lines[start_idx + 1..].contains(&"agent list")
        }

        /// Drive `run_wired` at `width`x20 with a `testutil::Stages` event source built from
        /// `stages`, on `run_wired_at`'s terms.
        fn run_wired_staged(
            width: u16,
            root: &Path,
            config: &Config,
            herdr: &Path,
            state_dir: Option<&Path>,
            stages: Vec<(&dyn Fn() -> bool, ratatui::crossterm::event::Event)>,
        ) -> (Result<Dashboard, StartError>, Vec<String>) {
            let backend = ratatui::backend::TestBackend::new(width, 20);
            let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
            let mut events = crate::testutil::Stages::new(stages);
            let startup = Startup {
                cwd: root,
                config,
                herdr,
                state_dir,
                env: &no_env,
                npm_hook: &no_npm_hook,
            };
            let result = super::super::run_wired(
                &mut terminal,
                &mut events,
                &startup,
                &crate::ui::read_artifact,
                Duration::from_millis(1),
            );
            let buf = terminal.backend().buffer().clone();
            let rows: Vec<String> = (0..buf.area.height)
                .map(|y| {
                    (0..buf.area.width)
                        .map(|x| buf[(x, y)].symbol().to_string())
                        .collect::<String>()
                })
                .collect();
            (result, rows)
        }

        /// A key press, on `testutil::press`'s terms — a local alias so the stage tables below
        /// read as a plain list of `(predicate, key)` pairs.
        fn key(c: char) -> ratatui::crossterm::event::Event {
            crate::testutil::press(
                ratatui::crossterm::event::KeyCode::Char(c),
                ratatui::crossterm::event::KeyModifiers::NONE,
            )
        }

        /// A scratch repository holding one active change, `2fa-support`, whose derived agent
        /// name is `c-2fa-support` — `state::agent_name`'s own output, since a leading digit
        /// cannot begin an agent name.
        fn scratch_repo_with_2fa_support() -> ScratchDir {
            let scratch = ScratchDir::new();
            let root = scratch.path();
            vendor_tdd_schema(root);
            write_with_mode(
                &root.join("openspec/changes/2fa-support/proposal.md"),
                b"# 2fa-support\n",
                0o644,
            );
            write_with_mode(
                &root.join("openspec/changes/2fa-support/tasks.md"),
                b"- [x] a\n- [x] b\n- [x] c\n- [x] d\n- [ ] e\n- [ ] f\n- [ ] g\n- [ ] h\n- [ ] i\n",
                0o644,
            );
            scratch
        }

        #[test]
        fn the_real_wiring_polls_a_scratch_herdr() {
            for width in [120u16, 60u16] {
                let scratch = scratch_repo_with_alpha();
                let root = scratch.path();
                let herdr_log = root.join("herdr.log");
                let openspec_log = root.join("openspec.log");
                let herdr = herdr_script(root, &herdr_log);
                let openspec = openspec_script(root, &openspec_log, root);

                let config = Config {
                    openspec_bin: Some(openspec),
                    ..Config::default()
                };

                let written = std::cell::Cell::new(false);
                let proposal = root.join("openspec/changes/alpha/proposal.md");
                let predicate = || {
                    if log_lines(&herdr_log) < 1 || log_lines(&openspec_log) < 1 {
                        return false;
                    }
                    if !written.get() {
                        append_one_byte(&proposal);
                        written.set(true);
                    }
                    log_lines(&openspec_log) >= 2
                };

                let (result, buf) = run_wired_at(width, root, &config, &herdr, None, &predicate);
                let dashboard = result.expect("run_wired must return Ok for a supported state");

                assert!(
                    dashboard.agents.reachable,
                    "width {width}: the poller must be reachable through the real seam"
                );
                assert_eq!(
                    dashboard.agents.agents.len(),
                    1,
                    "width {width}: exactly one agent from the scratch herdr program"
                );
                assert_eq!(dashboard.agents.agents[0].pane_id, "w8:p1", "width {width}");

                let herdr_calls = std::fs::read_to_string(&herdr_log).unwrap_or_default();
                assert!(
                    herdr_calls.lines().any(|l| l == "agent list"),
                    "width {width}: the herdr log must record an 'agent list' call: {herdr_calls:?}"
                );
                assert!(
                    !herdr_calls.lines().any(|l| l.starts_with("pane split")),
                    "width {width}: polling alone must launch nothing: {herdr_calls:?}"
                );

                // Two separately discriminating assertions, not one shared threshold —
                // a Change Review finding: `refresh::none()` in place of `refresh::start`
                // leaves the log at zero entries (the startup request never fires at
                // all), while `watch::none()` in place of `watch::start` leaves it at
                // exactly one (the startup request fires, but the one-byte write is
                // never detected, so no second request follows). A single `>= 2` bound
                // catches both, but names neither.
                assert!(
                    log_lines(&openspec_log) >= 1,
                    "width {width}: the openspec log must show at least one run, proving \
                     refresh::start was wired rather than refresh::none()"
                );
                assert!(
                    log_lines(&openspec_log) >= 2,
                    "width {width}: the openspec log must reach a second run, proving \
                     watch::start was wired rather than watch::none()"
                );
                assert!(
                    dashboard.refresh.problems.is_empty(),
                    "width {width}: supporting assertion only — both watch::start and \
                     watch::none() produce an empty problems vector"
                );

                assert!(
                    buf.iter()
                        .any(|row| row.contains("alpha") && row.contains("[4/9]")),
                    "width {width}: the real ui::load and the real artifact reader must have run"
                );
            }
        }

        #[test]
        fn an_unreachable_scratch_herdr_is_a_standalone_tui() {
            for width in [120u16, 60u16] {
                let scratch = scratch_repo_with_alpha();
                let root = scratch.path();
                let openspec_tree = root.join("openspec");
                let herdr = root.join("does-not-exist-herdr");
                let openspec_log = root.join("openspec.log");
                let openspec = openspec_script(root, &openspec_log, root);

                let config = Config {
                    openspec_bin: Some(openspec),
                    ..Config::default()
                };

                // `agent-attribution`: a real state directory holding a mapping that
                // would badge `alpha`, so the no-badge/no-count assertions below are
                // discriminating rather than vacuous.
                let state = ScratchDir::new();
                write_with_mode(
                    &state.path().join("agent-names.toml"),
                    b"[names]\nc-alpha = \"alpha\"\n",
                    0o644,
                );

                // Snapshotted at `openspec/`, not the whole scratch root: the
                // scratch programs and their logs live alongside it, outside
                // the repository tree, and the crate's byte-identity promise
                // ("the plugin never writes inside openspec/") is scoped to
                // that tree, not to this test's own harness artifacts.
                let before = snapshot(&openspec_tree);
                let state_before = snapshot(state.path());

                let predicate = || log_lines(&openspec_log) >= 1;

                let (result, buf) =
                    run_wired_at(width, root, &config, &herdr, Some(state.path()), &predicate);
                let dashboard = result.expect("an unreachable socket is a supported state");

                assert!(
                    !dashboard.agents.reachable,
                    "width {width}: no herdr program at all is the standalone-TUI case"
                );
                assert!(dashboard.agents.agents.is_empty(), "width {width}");
                assert!(
                    dashboard.agents.problem.is_some(),
                    "width {width}: the reason must be recorded"
                );

                assert!(
                    buf.iter()
                        .any(|row| row.contains("alpha") && row.contains("[4/9]")),
                    "width {width}: nothing is hidden and no error screen replaces the pane"
                );
                assert!(
                    !buf.iter().any(|row| row.contains("unattributed")),
                    "width {width}: an unreachable socket must carry no count"
                );
                assert!(
                    !buf.iter()
                        .any(|row| row.contains("alpha") && row.contains(" b ")),
                    "width {width}: an unreachable socket must carry no badge"
                );

                let after = snapshot(&openspec_tree);
                assert_eq!(
                    before, after,
                    "width {width}: this run writes nothing of its own"
                );
                let state_after = snapshot(state.path());
                assert_eq!(
                    state_before, state_after,
                    "width {width}: the state directory must be read, never written"
                );
                assert_eq!(
                    dashboard.launch.pending, None,
                    "width {width}: an unreachable socket leaves nothing pending"
                );
                assert!(
                    dashboard.launch.problems.is_empty(),
                    "width {width}: an unreachable socket produces no launch problem"
                );

                // Discriminating control: the same comparison must fail when a
                // single byte of tasks.md is rewritten between two further
                // snapshots, or the equality assertion above proves nothing.
                let tasks_path = root.join("openspec/changes/alpha/tasks.md");
                let original = std::fs::read_to_string(&tasks_path).expect("read tasks.md back");
                let control_before = snapshot(&openspec_tree);
                let mutated = original.replacen("[ ] e", "[x] e", 1);
                write_with_mode(&tasks_path, mutated.as_bytes(), 0o644);
                let control_after = snapshot(&openspec_tree);
                assert_ne!(
                    control_before, control_after,
                    "width {width}: the snapshot comparison must discriminate a rewritten byte"
                );
                write_with_mode(&tasks_path, original.as_bytes(), 0o644);
            }
        }

        #[test]
        fn no_repository_still_polls_for_agents() {
            let scratch = ScratchDir::new();
            let root = scratch.path();
            let herdr_log = root.join("herdr.log");
            let herdr = herdr_script(root, &herdr_log);
            let openspec_log = root.join("openspec.log");
            // A scratch `openspec` program exists here too, and `Config::openspec_bin`
            // names it — same as every other test in this module — so the assertion
            // below that its log stays empty is a real proof that `refresh::start` is
            // the inert double, not merely a fact about no program having been
            // configured at all. Without this, the real probe chain's later steps
            // (`PATH`, nvm, `npm prefix -g`) would still run on a machine where one of
            // them resolves, which `design.md` -> Test Boundaries rules out.
            let openspec = openspec_script(root, &openspec_log, root);

            let config = Config {
                openspec_bin: Some(openspec),
                ..Config::default()
            };

            // `agent-launch`: `a` then `g` are pressed too, before `q` — with no
            // repository selected, both keys must produce no Herdr call beyond the
            // poller's own `agent list`, and the footer must still offer both hints
            // since the socket is reachable.
            let stage1 = || log_lines(&herdr_log) >= 1;
            let stages: Vec<(&dyn Fn() -> bool, ratatui::crossterm::event::Event)> = vec![
                (&stage1, key('a')),
                (&stage1, key('g')),
                (&stage1, key('q')),
            ];
            let (result, buf) = run_wired_staged(120, root, &config, &herdr, None, stages);
            let dashboard = result.expect("no repository is a supported state");

            assert_eq!(dashboard.repo, None);
            assert!(
                dashboard.agents.reachable,
                "the poller runs unconditionally"
            );
            assert_eq!(dashboard.agents.agents.len(), 1);
            assert!(
                !openspec_log.exists() || log_lines(&openspec_log) == 0,
                "with no repository, refresh::start must be the inert double"
            );
            assert!(dashboard.refresh.problems.is_empty());
            assert_eq!(dashboard.route, Route::List);
            assert!(
                buf.iter()
                    .any(|row| row.contains("No OpenSpec repository found")),
                "the list region's no-repository empty state must be on screen: {buf:?}"
            );
            assert!(
                dashboard.agent_names.names.is_empty(),
                "agent-attribution: state_dir was None, so the mapping must be empty"
            );
            assert!(
                !buf.iter().any(|row| row.contains("unattributed")),
                "agent-attribution: with no repository no agent is in scope, so no count"
            );
            assert!(
                buf.iter()
                    .any(|row| row.contains("a/c/s launch") && row.contains("g focus")),
                "the footer must still carry both action hints: {buf:?}"
            );
            let herdr_calls = std::fs::read_to_string(&herdr_log).unwrap_or_default();
            assert!(
                herdr_calls.lines().all(|l| l == "agent list"),
                "pressing a and g with no repository selected must produce no Herdr call \
                 beyond agent list: {herdr_calls:?}"
            );
            assert_eq!(dashboard.launch.pending, None);
            assert!(dashboard.launch.problems.is_empty());
        }

        /// `agent-attribution`'s outer-loop acceptance test: a polled agent must reach a
        /// rendered badge and a rendered footer count, not merely `Dashboard.agents` — see
        /// design.md -> Test Strategy. RED until group 8 wires the mapping read, the badge
        /// cell, and the footer count together.
        #[test]
        fn a_polled_agent_reaches_a_rendered_badge() {
            for width in [120u16, 60u16] {
                let scratch = scratch_repo_with_alpha();
                let root = scratch.path();
                // Replace `alpha`'s tasks.md with the two changes this scenario names —
                // `2fa-support` and `alpha`, each counting 4 of 9 — so the fixture matches
                // design.md's scenario exactly rather than reusing the single-change one.
                std::fs::remove_dir_all(root.join("openspec/changes/alpha"))
                    .expect("remove the single-change fixture");
                for name in ["2fa-support", "alpha"] {
                    write_with_mode(
                        &root.join(format!("openspec/changes/{name}/proposal.md")),
                        format!("# {name}\n").as_bytes(),
                        0o644,
                    );
                    write_with_mode(
                        &root.join(format!("openspec/changes/{name}/tasks.md")),
                        b"- [x] a\n- [x] b\n- [x] c\n- [x] d\n- [ ] e\n- [ ] f\n- [ ] g\n- [ ] h\n- [ ] i\n",
                        0o644,
                    );
                }

                let state = ScratchDir::new();
                write_with_mode(
                    &state.path().join("agent-names.toml"),
                    b"[names]\nc-2fa-support = \"2fa-support\"\n",
                    0o644,
                );

                let herdr_log = root.join("herdr.log");
                let repo_root = canonical(root).display().to_string();
                let entries = vec![
                    agent_entry(1, Some("c-2fa-support"), "working", Some(&repo_root)),
                    agent_entry(2, Some("alpha"), "blocked", Some(&repo_root)),
                    agent_entry(3, None, "idle", Some(&repo_root)),
                    agent_entry(
                        4,
                        Some("nothing-like-a-change"),
                        "working",
                        Some("/definitely/elsewhere"),
                    ),
                ];
                let herdr = herdr_script_for(root, &herdr_log, &entries);

                let openspec_log = root.join("openspec.log");
                let openspec = openspec_script(root, &openspec_log, root);
                let config = Config {
                    openspec_bin: Some(openspec),
                    ..Config::default()
                };

                let predicate = || log_lines(&herdr_log) >= 1;

                let (result, buf) =
                    run_wired_at(width, root, &config, &herdr, Some(state.path()), &predicate);
                let dashboard = result.expect("run_wired must return Ok for a supported state");

                assert!(
                    dashboard.agents.reachable,
                    "width {width}: the poller must be reachable through the real seam"
                );
                assert_eq!(
                    dashboard.agents.agents.len(),
                    4,
                    "width {width}: all four scratch agents must have been polled"
                );

                let two_fa_row = buf
                    .iter()
                    .find(|row| row.contains("2fa-support"))
                    .unwrap_or_else(|| panic!("width {width}: no row named 2fa-support: {buf:?}"));
                assert!(
                    two_fa_row.contains(" w [4/9]"),
                    "width {width}: 2fa-support's row must carry the working badge: {two_fa_row:?}"
                );

                let alpha_row = buf
                    .iter()
                    .find(|row| row.contains("alpha"))
                    .unwrap_or_else(|| panic!("width {width}: no row named alpha: {buf:?}"));
                assert!(
                    alpha_row.contains(" b [4/9]"),
                    "width {width}: alpha's row must carry the blocked badge: {alpha_row:?}"
                );

                // `agent-launch`: the socket is reachable, so the two action hints are
                // offered too — at 120 the count still fits after them; at 60 it no longer
                // does and is dropped whole.
                let footer = if width == 120 {
                    "q quit  Enter detail  Esc back  a/c/s launch  g focus  1 unattributed"
                } else {
                    "q quit  Enter detail  Esc back  a/c/s launch  g focus"
                };
                let expected_footer = format!(
                    "{footer}{}",
                    " ".repeat(width as usize - footer.chars().count())
                );
                assert_eq!(
                    buf[19], expected_footer,
                    "width {width}: the footer must read exactly '{footer}'"
                );
            }
        }

        /// `agent-launch`'s headline scenario: "Pressing `a` splits a pane, starts an agent,
        /// and sends the prompt". Drives the real `run_wired` at both mandated widths, feeding
        /// it an actual `a` key event and observing the three Herdr invocations, in order, at
        /// the seam. RED until group 12: today `Dashboard::apply`'s `LaunchApply` arm does
        /// nothing, so no Herdr call beyond `agent list` is ever logged.
        #[test]
        fn a_keypress_launches_an_agent() {
            for width in [120u16, 60u16] {
                let scratch = scratch_repo_with_2fa_support();
                let root = scratch.path();
                let canon_root = canonical(root);
                let herdr_log = root.join("herdr.log");
                let marker = root.join("marker");
                let herdr = launch_herdr_script(root, &herdr_log, &marker, &canon_root);
                let openspec_log = root.join("openspec.log");
                let openspec = openspec_script(root, &openspec_log, root);
                let state = ScratchDir::new();

                let config = Config {
                    openspec_bin: Some(openspec),
                    agent_kind: "codex".to_string(),
                    ..Config::default()
                };

                let before = snapshot(&root.join("openspec"));

                let stage1 = || log_lines(&herdr_log) >= 1;
                let stage2 = || non_agent_list_lines(&herdr_log).len() >= 3;
                let stages: Vec<(&dyn Fn() -> bool, ratatui::crossterm::event::Event)> =
                    vec![(&stage1, key('a')), (&stage2, key('q'))];

                let (result, buf) =
                    run_wired_staged(width, root, &config, &herdr, Some(state.path()), stages);
                let dashboard = result.expect("run_wired must return Ok for a supported state");

                let calls = non_agent_list_lines(&herdr_log);
                assert_eq!(
                    calls.len(),
                    3,
                    "width {width}: exactly three non-agent-list Herdr calls: {calls:?}"
                );
                assert_eq!(
                    calls[0],
                    format!(
                        "pane split --cwd {} --direction right --no-focus",
                        canon_root.display()
                    ),
                    "width {width}: call 1 must be the split, with the canonicalized root"
                );
                assert_eq!(
                    calls[1], "agent start c-2fa-support --kind codex --pane wD:pJ",
                    "width {width}: call 2 must start the derived agent name on the pane split returned"
                );
                assert_eq!(
                    calls[2], "agent prompt c-2fa-support /opsx:apply 2fa-support",
                    "width {width}: call 3 must send the /opsx:apply prompt"
                );

                let mapping_path = state.path().join("agent-names.toml");
                let mapping_text = std::fs::read_to_string(&mapping_path).unwrap_or_default();
                assert!(
                    mapping_text.contains("c-2fa-support = \"2fa-support\""),
                    "width {width}: agent-names.toml must map the derived name to the change: {mapping_text:?}"
                );

                assert_eq!(
                    dashboard.agent_names.names.get("c-2fa-support"),
                    Some(&"2fa-support".to_string()),
                    "width {width}: the returned dashboard's mapping must hold the same pair"
                );
                assert_eq!(
                    dashboard.launch.pending, None,
                    "width {width}: the request must have been taken"
                );
                assert!(
                    dashboard.launch.problems.is_empty(),
                    "width {width}: a successful launch reports no problem"
                );

                let footer_row = &buf[19];
                assert!(
                    footer_row.contains("a/c/s launch") && footer_row.contains("g focus"),
                    "width {width}: the footer must carry both action hints: {footer_row:?}"
                );

                let after = snapshot(&root.join("openspec"));
                assert_eq!(
                    before, after,
                    "width {width}: the plugin must write nothing inside openspec/"
                );
            }
        }

        /// `agent-launch`: "Pressing `g` after the launch focuses the pane the launch
        /// created." Two configured `agent_kind`s across this test and the one above (`codex`
        /// here would be wrong; this one is `gemini`) is what makes the claim that
        /// `start_collaborators` threads `config.agent_kind` through discriminating rather
        /// than a presence check alone. RED until group 12.
        #[test]
        fn g_focuses_the_agent_the_launch_started() {
            for width in [120u16, 60u16] {
                let scratch = scratch_repo_with_2fa_support();
                let root = scratch.path();
                let canon_root = canonical(root);
                let herdr_log = root.join("herdr.log");
                let marker = root.join("marker");
                let herdr = launch_herdr_script(root, &herdr_log, &marker, &canon_root);
                let openspec_log = root.join("openspec.log");
                let openspec = openspec_script(root, &openspec_log, root);
                let state = ScratchDir::new();

                let config = Config {
                    openspec_bin: Some(openspec),
                    agent_kind: "gemini".to_string(),
                    ..Config::default()
                };

                let stage1 = || log_lines(&herdr_log) >= 1;
                let stage2 = || agent_seen_in_a_later_poll(&herdr_log);
                let stage3 = || non_agent_list_lines(&herdr_log).len() >= 4;
                let stages: Vec<(&dyn Fn() -> bool, ratatui::crossterm::event::Event)> = vec![
                    (&stage1, key('a')),
                    (&stage2, key('g')),
                    (&stage3, key('q')),
                ];

                let (result, buf) =
                    run_wired_staged(width, root, &config, &herdr, Some(state.path()), stages);
                let dashboard = result.expect("run_wired must return Ok for a supported state");

                let calls = non_agent_list_lines(&herdr_log);
                assert_eq!(
                    calls.len(),
                    4,
                    "width {width}: the launch's three calls plus one focus: {calls:?}"
                );
                assert_eq!(
                    calls[1], "agent start c-2fa-support --kind gemini --pane wD:pJ",
                    "width {width}: two different configured kinds across the two tests"
                );
                assert_eq!(
                    calls[3], "agent focus wD:pJ",
                    "width {width}: the last call must be exactly one focus, on the split's pane"
                );

                let two_fa_row = buf
                    .iter()
                    .find(|row| row.contains("2fa-support"))
                    .unwrap_or_else(|| panic!("width {width}: no row named 2fa-support: {buf:?}"));
                assert!(
                    two_fa_row.contains(" w ["),
                    "width {width}: the mapping written by the launch must be read back through \
                     attribution's first tier within the same run: {two_fa_row:?}"
                );

                assert_eq!(dashboard.launch.pending, None, "width {width}");
                assert!(dashboard.launch.problems.is_empty(), "width {width}");
            }
        }

        /// `agent-launch`: "An unreachable socket leaves every key inert and the pane a
        /// working TUI." No scratch `herdr` program exists at all, so the poller is never
        /// reachable; `a` and `g` are pressed regardless and must produce no Herdr call and no
        /// mapping. RED until group 12 makes `an unreachable socket makes every action key
        /// inert` true end to end — today the assertions already hold vacuously, because
        /// `LaunchApply`'s `apply` arm does nothing yet, so this scenario is included for
        /// completeness and reverified at group 12 rather than treated as one of the three
        /// deliberately RED tests.
        #[test]
        fn an_unreachable_socket_leaves_every_key_inert() {
            let scratch = scratch_repo_with_2fa_support();
            let root = scratch.path();
            let herdr = root.join("does-not-exist-herdr");
            let state = ScratchDir::new();

            let config = Config::default();

            let before = snapshot(&root.join("openspec"));
            let state_before = snapshot(state.path());

            let mut events = crate::testutil::Script::new(vec![
                Ok(Some(key('a'))),
                Ok(Some(key('g'))),
                Ok(Some(key('q'))),
            ]);
            let backend = ratatui::backend::TestBackend::new(120, 20);
            let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
            let startup = Startup {
                cwd: root,
                config: &config,
                herdr: &herdr,
                state_dir: Some(state.path()),
                env: &no_env,
                npm_hook: &no_npm_hook,
            };
            let result = super::super::run_wired(
                &mut terminal,
                &mut events,
                &startup,
                &crate::ui::read_artifact,
                Duration::from_millis(1),
            );
            let dashboard = result.expect("an unreachable socket is a supported state");

            assert!(!dashboard.agents.reachable);
            assert_eq!(dashboard.launch.pending, None);
            assert!(dashboard.launch.problems.is_empty());
            assert!(dashboard.agent_names.names.is_empty());

            let after = snapshot(&root.join("openspec"));
            assert_eq!(before, after);
            let state_after = snapshot(state.path());
            assert_eq!(state_before, state_after);
        }

        // --- degraded-states: the probe seam, file_mode, and the startup problems that
        // --- reach the pane (task group 3) -------------------------------------------------

        /// `openspec-binary` :: "An outer test drives a failing probe without touching the
        /// machine" — the SAME neutral `env` (no `PATH`, no `HOME`, no `NVM_DIR`) proves both
        /// the failing case, through `run_wired`'s real `resolve::openspec_bin` call, and —
        /// via the resolving control — that the injected `npm_hook` is what the fourth probe
        /// step actually reads, never the real production binding. Neither sub-case touches
        /// the developer's real `PATH` or spawns the real `npm` (design.md -> Decision 14).
        #[test]
        fn run_wired_probes_through_the_injected_hook() {
            let scratch = scratch_repo_with_alpha();
            let root = scratch.path();
            let herdr = root.join("does-not-exist-herdr");

            let (result, _buf) = run_wired_probed(
                ProbedStartup {
                    width: 120,
                    root,
                    config: &Config::default(),
                    herdr: &herdr,
                    state_dir: None,
                    env: &no_env,
                    npm_hook: &no_npm_hook,
                },
                &|| true,
            );
            let dashboard = result.expect("no usable binary anywhere is a supported state");
            assert!(
                dashboard.file_mode,
                "with PATH, nvm, and the npm hook all unusable, file_mode must be true"
            );

            // The resolving control: the SAME env (no PATH) but the injected npm hook
            // resolves a usable binary — discriminates the assertion above from "file_mode
            // is always true regardless of the hook".
            let prefix = ScratchDir::new();
            let openspec_log = root.join("openspec-via-npm.log");
            let _ = openspec_script(prefix.path(), &openspec_log, root);
            let prefix_path = prefix.path().to_path_buf();
            let resolving_npm = move || Some(prefix_path.clone());
            let predicate = || log_lines(&openspec_log) >= 1;
            let (result2, _buf2) = run_wired_probed(
                ProbedStartup {
                    width: 120,
                    root,
                    config: &Config::default(),
                    herdr: &herdr,
                    state_dir: None,
                    env: &no_env,
                    npm_hook: &resolving_npm,
                },
                &predicate,
            );
            let dashboard2 = result2.expect("a resolving npm hook is a supported state");
            assert!(
                !dashboard2.file_mode,
                "the resolving control must not be file mode"
            );
            assert!(
                log_lines(&openspec_log) >= 1,
                "the resolving control must have spawned the scratch openspec program through \
                 the injected npm hook, proving the probe reached step 4 rather than a cached \
                 negative"
            );
        }

        /// `dashboard-loop` :: "`file_mode` is set by the composition root and by nothing
        /// else" — the outer half; `mod load::load_never_claims_file_mode` is the unit half.
        #[test]
        fn run_wired_sets_file_mode_from_the_probe() {
            let scratch = scratch_repo_with_alpha();
            let root = scratch.path();
            let herdr = root.join("does-not-exist-herdr");

            let (result, _buf) =
                run_wired_at(120, root, &Config::default(), &herdr, None, &|| true);
            let dashboard = result.expect("no binary anywhere is a supported state");
            assert!(dashboard.file_mode);

            let openspec_log = root.join("openspec-fm.log");
            let openspec = openspec_script(root, &openspec_log, root);
            let config = Config {
                openspec_bin: Some(openspec),
                ..Config::default()
            };
            let predicate = || log_lines(&openspec_log) >= 1;
            let (result2, _buf2) = run_wired_at(120, root, &config, &herdr, None, &predicate);
            let dashboard2 = result2.expect("a configured usable binary is a supported state");
            assert!(!dashboard2.file_mode);
        }

        // --- seam-resilience group 3: the composition root supplies the resolved root as
        // --- the child's working directory, and a one-entry `PATH` overlay -----------------

        /// A scratch `openspec` program like `openspec_script`, additionally appending its
        /// own working directory (via `pwd`) to `cwd_log` on each invocation — S7's own
        /// observation, made from outside the seam rather than by instrumenting it.
        fn openspec_script_logging_cwd(dir: &Path, cwd_log: &Path, root: &Path) -> PathBuf {
            write_script(
                dir,
                "openspec",
                &format!(
                    "pwd >> \"{cwd_log}\"\n\
                     printf '%s' '{{\"changes\":[],\"root\":{{\"path\":\"{root}\",\"source\":\"nearest\"}}}}'\n",
                    cwd_log = cwd_log.display(),
                    root = root.display(),
                ),
            )
        }

        /// An `openspec` script written directly at `dir.join("openspec")` — never under a
        /// `bin/` subdirectory, unlike `write_script` — so a test can put it on a fabricated
        /// `PATH` value exactly as `resolve::path_candidates` expects: `<entry>/openspec`.
        /// Logs its own inherited `PATH` to `path_log` on each invocation — S10's own
        /// observation, made from outside the seam rather than by instrumenting it.
        fn openspec_at_logging_path(dir: &Path, path_log: &Path, root: &Path) -> PathBuf {
            let path = dir.join("openspec");
            write_with_mode(
                &path,
                format!(
                    "#!/bin/sh\nprintf '%s\\n' \"$PATH\" >> \"{path_log}\"\n\
                     printf '%s' '{{\"changes\":[],\"root\":{{\"path\":\"{root}\",\"source\":\"nearest\"}}}}'\n",
                    path_log = path_log.display(),
                    root = root.display(),
                )
                .as_bytes(),
                0o755,
            );
            path
        }

        /// An `openspec` script placed at the nvm layout `openspec_bin`'s own step 3 walks:
        /// `<home>/.nvm/versions/node/<version>/bin/openspec`. Logs its own inherited `PATH`
        /// to `path_log` on each invocation, on `openspec_at_logging_path`'s terms.
        fn nvm_openspec_script_logging_path(
            home: &Path,
            version: &str,
            path_log: &Path,
            root: &Path,
        ) -> PathBuf {
            let path = home
                .join(".nvm")
                .join("versions")
                .join("node")
                .join(version)
                .join("bin")
                .join("openspec");
            write_with_mode(
                &path,
                format!(
                    "#!/bin/sh\nprintf '%s\\n' \"$PATH\" >> \"{path_log}\"\n\
                     printf '%s' '{{\"changes\":[],\"root\":{{\"path\":\"{root}\",\"source\":\"nearest\"}}}}'\n",
                    path_log = path_log.display(),
                    root = root.display(),
                )
                .as_bytes(),
                0o755,
            );
            path
        }

        /// `refresh-worker` :: "The CLI is constructed with the resolved repository root" —
        /// driven through `run_wired` (never a direct sleep loop against `start_collaborators`:
        /// `NOSLEEP` leg 2 bans a real sleep anywhere under `src/ui`, tests included, since
        /// every wait there goes through the render loop's own predicate-driven event source)
        /// with a real scratch `openspec` program that reports its own working directory.
        /// Before this change the child inherited the *process's* working directory (`cargo
        /// test`'s own), not the resolved repository root, which is exactly S7 (design.md ->
        /// S7/S10 measurement).
        #[test]
        fn collaborators_construct_the_cli_with_the_resolved_repository_root() {
            let scratch = scratch_repo_with_alpha();
            let root = scratch.path();
            let cwd_log = root.join("cwd.log");
            let openspec = openspec_script_logging_cwd(root, &cwd_log, root);
            let config = Config {
                openspec_bin: Some(openspec),
                ..Config::default()
            };
            let herdr = root.join("does-not-exist-herdr");
            let predicate = || log_lines(&cwd_log) >= 1;

            let (result, _buf) = run_wired_at(120, root, &config, &herdr, None, &predicate);
            let _dashboard = result.expect("a configured usable binary is a supported state");

            let logged = std::fs::read_to_string(&cwd_log).expect("cwd.log should exist");
            let printed_cwd = logged.lines().next().expect("pwd printed a first line");
            assert_eq!(
                canonical(Path::new(printed_cwd)),
                canonical(root),
                "the openspec child must run with the resolved repository root as its cwd, \
                 not the test process's own"
            );
        }

        /// `refresh-worker` :: "The overlay prepends the resolved binary's own directory to
        /// `PATH`" — the binary resolves via the nvm step (3), which is exactly the
        /// structural condition design.md -> Decision 1b/S10 argues the overlay exists for.
        #[test]
        fn collaborators_overlay_prepends_the_resolved_binarys_own_directory_to_path() {
            let scratch = scratch_repo_with_alpha();
            let root = scratch.path();
            let path_log = root.join("path.log");
            let home = scratch.path().join("home");
            let bin = nvm_openspec_script_logging_path(&home, "v24.18.0", &path_log, root);
            let bin_parent = bin.parent().expect("nvm bin has a parent").to_path_buf();

            let home_str = home.display().to_string();
            let inherited = "/usr/bin:/bin".to_string();
            let inherited_for_env = inherited.clone();
            let env = move |name: &str| match name {
                "HOME" => Some(home_str.clone()),
                "PATH" => Some(inherited_for_env.clone()),
                _ => None,
            };
            let config = Config::default();
            let herdr = root.join("does-not-exist-herdr");
            let predicate = || log_lines(&path_log) >= 1;

            let (result, _buf) = run_wired_probed(
                ProbedStartup {
                    width: 120,
                    root,
                    config: &config,
                    herdr: &herdr,
                    state_dir: None,
                    env: &env,
                    npm_hook: &no_npm_hook,
                },
                &predicate,
            );
            let _dashboard = result.expect("an nvm-resolved binary is a supported state");

            let logged = std::fs::read_to_string(&path_log).expect("path.log should exist");
            let printed_path = logged.lines().next().expect("PATH printed a first line");
            assert_eq!(
                printed_path,
                format!("{}:{inherited}", bin_parent.display())
            );
        }

        /// `refresh-worker` :: "An absent inherited `PATH` yields the directory alone".
        #[test]
        fn collaborators_overlay_is_the_binarys_directory_alone_when_path_is_absent() {
            let scratch = scratch_repo_with_alpha();
            let root = scratch.path();
            let path_log = root.join("path-absent.log");
            let home = scratch.path().join("home");
            let bin = nvm_openspec_script_logging_path(&home, "v24.18.0", &path_log, root);
            let bin_parent = bin.parent().expect("nvm bin has a parent").to_path_buf();

            let home_str = home.display().to_string();
            let env = move |name: &str| {
                if name == "HOME" {
                    Some(home_str.clone())
                } else {
                    None
                }
            };
            let config = Config::default();
            let herdr = root.join("does-not-exist-herdr");
            let predicate = || log_lines(&path_log) >= 1;

            let (result, _buf) = run_wired_probed(
                ProbedStartup {
                    width: 120,
                    root,
                    config: &config,
                    herdr: &herdr,
                    state_dir: None,
                    env: &env,
                    npm_hook: &no_npm_hook,
                },
                &predicate,
            );
            let _dashboard = result.expect("an nvm-resolved binary is a supported state");

            let logged = std::fs::read_to_string(&path_log).expect("path.log should exist");
            let printed_path = logged.lines().next().expect("PATH printed a first line");
            assert_eq!(printed_path, bin_parent.display().to_string());
        }

        /// `refresh-worker` :: "A binary already on `PATH` gets the same overlay,
        /// harmlessly" — the probe resolves via step 2, and the overlay still prepends the
        /// binary's own (already-present) directory rather than special-casing this step.
        #[test]
        fn collaborators_overlay_a_binary_already_on_path_harmlessly() {
            let scratch = scratch_repo_with_alpha();
            let root = scratch.path();
            let path_log = root.join("path-dup.log");
            let bindir = scratch.path().join("localbin");
            let bin = openspec_at_logging_path(&bindir, &path_log, root);
            let bin_parent = bin.parent().expect("bin has a parent").to_path_buf();

            let inherited = format!("{}:/usr/bin", bin_parent.display());
            let inherited_for_env = inherited.clone();
            let env = move |name: &str| {
                if name == "PATH" {
                    Some(inherited_for_env.clone())
                } else {
                    None
                }
            };
            let config = Config::default();
            let herdr = root.join("does-not-exist-herdr");
            let predicate = || log_lines(&path_log) >= 1;

            let (result, _buf) = run_wired_probed(
                ProbedStartup {
                    width: 120,
                    root,
                    config: &config,
                    herdr: &herdr,
                    state_dir: None,
                    env: &env,
                    npm_hook: &no_npm_hook,
                },
                &predicate,
            );
            let _dashboard = result.expect("a PATH-resolved binary is a supported state");

            let logged = std::fs::read_to_string(&path_log).expect("path.log should exist");
            let printed_path = logged.lines().next().expect("PATH printed a first line");
            assert_eq!(
                printed_path,
                format!("{}:{inherited}", bin_parent.display())
            );
        }

        /// `watch-invalidation` :: "The composition root watches `openspec/`, not the
        /// repository root" (`seam-resilience`). Driven directly through
        /// `start_collaborators` with the real `watch::start` rather than a recording
        /// double: `NOSLEEP` bans a real sleep anywhere under `src/ui`, so a live batch
        /// cannot be proven from here (see
        /// `watch::tests::a_write_outside_openspec_produces_no_batch` for that half
        /// instead). The two roots are told apart synchronously instead, the same way
        /// `watch::tests::start_on_a_missing_path_degrades_and_names_the_reason` already
        /// proves `notify`'s own `.watch()` fails immediately for a path that does not
        /// exist: a repository root that exists but holds no `openspec/` subdirectory
        /// watches successfully today — no problem is ever reported — and starts failing,
        /// naming `<repo>/openspec`, once the watch is rooted there instead.
        #[test]
        fn collaborators_watch_root_is_openspec_not_the_repository_root() {
            let scratch = ScratchDir::new();
            let root = scratch.path();
            // Deliberately no `openspec/` subdirectory under `root`: the repository root
            // itself exists and would watch successfully, so only a watch correctly rooted
            // at `<root>/openspec` can fail here.
            let config = Config::default();
            let herdr = root.join("does-not-exist-herdr");

            let collaborators = super::super::start_collaborators(
                Some(root),
                &config,
                &herdr,
                None,
                &no_env,
                &no_npm_hook,
            );

            let openspec_path = root.join("openspec").display().to_string();
            assert!(
                collaborators
                    .problems
                    .iter()
                    .any(|p| p.contains(&openspec_path)),
                "the watcher must be started on <repo>/openspec, not the repository root \
                 itself, which exists and would watch successfully: {:?}",
                collaborators.problems
            );
        }

        /// `openspec-binary` :: "A configured path that cannot be used reaches the list as a
        /// problem row" — rows 27/31's "true but unobservable" defect, made observable.
        #[test]
        fn unusable_openspec_bin_renders_as_a_problem_row() {
            for width in [120u16, 60u16] {
                let scratch = scratch_repo_with_alpha();
                let root = scratch.path();
                let herdr = root.join("does-not-exist-herdr");
                let bad_bin = root.join("not-a-real-openspec-binary");
                let config = Config {
                    openspec_bin: Some(bad_bin.clone()),
                    ..Config::default()
                };
                let (result, buf) = run_wired_at(width, root, &config, &herdr, None, &|| true);
                let dashboard = result.expect("an unusable configured binary is a supported state");
                assert!(dashboard.file_mode, "width {width}");
                assert!(
                    dashboard
                        .refresh
                        .problems
                        .iter()
                        .any(|p| p.contains("openspec_bin")
                            && p.contains(&bad_bin.display().to_string())),
                    "width {width}: {:?}",
                    dashboard.refresh.problems
                );
                assert!(
                    buf.iter()
                        .any(|row| row.contains('!') && row.contains("openspec_bin")),
                    "width {width}: no leading problem row names openspec_bin: {buf:?}"
                );
            }
        }

        /// `openspec-binary` :: "No binary anywhere is reported as file mode rather than as
        /// an error".
        #[test]
        fn no_binary_is_file_mode_not_an_error() {
            let scratch = scratch_repo_with_alpha();
            let root = scratch.path();
            let herdr = root.join("does-not-exist-herdr");
            let (result, buf) = run_wired_at(120, root, &Config::default(), &herdr, None, &|| true);
            let dashboard = result.expect("no binary anywhere must be Ok, not an error");
            assert!(dashboard.file_mode);
            assert!(
                dashboard.refresh.problems.is_empty(),
                "an absent CLI records no problem: {:?}",
                dashboard.refresh.problems
            );
            assert!(
                buf.iter()
                    .any(|row| row.contains("alpha") && row.contains("[4/9]")),
                "the pane must still render real content: {buf:?}"
            );
        }

        /// `openspec-binary` :: "A resolved binary contributes nothing" — whole-buffer
        /// equality, with a discriminating control (an unusable configured binary, which
        /// DOES add a problem row) so the equality is not satisfied by two empty buffers.
        #[test]
        fn a_resolved_binary_adds_no_problem_and_no_badge() {
            for width in [120u16, 60u16] {
                let scratch = scratch_repo_with_alpha();
                let root = scratch.path();
                let herdr = root.join("does-not-exist-herdr");
                let openspec_log = root.join("openspec-resolved.log");
                let openspec = openspec_script(root, &openspec_log, root);
                let config = Config {
                    openspec_bin: Some(openspec),
                    ..Config::default()
                };
                let predicate = || log_lines(&openspec_log) >= 1;
                let (result, buf) = run_wired_at(width, root, &config, &herdr, None, &predicate);
                let dashboard = result.expect("a resolved binary is a supported state");
                assert!(!dashboard.file_mode, "width {width}");
                assert!(
                    dashboard.refresh.problems.is_empty(),
                    "width {width}: {:?}",
                    dashboard.refresh.problems
                );
                assert!(
                    !buf.iter().any(|row| row.trim_start().starts_with('!')),
                    "width {width}: a resolved binary must add no leading problem row: {buf:?}"
                );

                // Discriminating control.
                let bad_bin = root.join("not-a-real-openspec-binary");
                let bad_config = Config {
                    openspec_bin: Some(bad_bin),
                    ..Config::default()
                };
                let (bad_result, bad_buf) =
                    run_wired_at(width, root, &bad_config, &herdr, None, &|| true);
                let _ =
                    bad_result.expect("an unusable configured binary is still a supported state");
                assert_ne!(
                    buf, bad_buf,
                    "width {width}: the control must render differently from the \
                     resolved-binary buffer"
                );
            }
        }

        /// `plugin-config` :: "A malformed key renders as a leading problem row at both
        /// widths" — `Config::problems`, populated by `config::load` (not exercised here;
        /// this test constructs the value directly, on exactly `plugin-config`'s own unit
        /// tests' terms), now reaches the pane through `start_collaborators`.
        #[test]
        fn config_fallback_renders_as_a_leading_problem_row() {
            for width in [120u16, 60u16] {
                let scratch = scratch_repo_with_alpha();
                let root = scratch.path();
                let herdr = root.join("does-not-exist-herdr");
                let config = Config {
                    problems: vec!["config.toml is not valid TOML: bad".to_string()],
                    ..Config::default()
                };
                let (result, buf) = run_wired_at(width, root, &config, &herdr, None, &|| true);
                let dashboard = result.expect("a configuration fallback is a supported state");
                assert_eq!(
                    dashboard.refresh.problems.first().map(String::as_str),
                    Some("config.toml is not valid TOML: bad"),
                    "width {width}: the configuration's own problem must lead"
                );
                assert!(
                    buf.iter()
                        .any(|row| row.contains("config.toml is not valid TOML")),
                    "width {width}: {buf:?}"
                );
            }
        }

        /// `plugin-config` :: "A clean configuration contributes nothing" — whole-buffer
        /// equality, with a discriminating control (a dirty configuration, which DOES add a
        /// row).
        #[test]
        fn a_clean_config_adds_no_row() {
            for width in [120u16, 60u16] {
                let scratch = scratch_repo_with_alpha();
                let root = scratch.path();
                let herdr = root.join("does-not-exist-herdr");
                let config = Config::default();
                let (result, buf) = run_wired_at(width, root, &config, &herdr, None, &|| true);
                let dashboard = result.expect("a clean configuration is a supported state");
                assert!(
                    dashboard.refresh.problems.is_empty(),
                    "width {width}: {:?}",
                    dashboard.refresh.problems
                );
                assert!(
                    !buf.iter().any(|row| row.trim_start().starts_with('!')),
                    "width {width}: {buf:?}"
                );

                let dirty_config = Config {
                    problems: vec!["config.toml is not valid TOML: bad".to_string()],
                    ..Config::default()
                };
                let (dirty_result, dirty_buf) =
                    run_wired_at(width, root, &dirty_config, &herdr, None, &|| true);
                let _ = dirty_result.expect("a dirty configuration is still a supported state");
                assert_ne!(
                    buf, dirty_buf,
                    "width {width}: the control must render differently"
                );
            }
        }

        /// `plugin-config` :: "Configuration problems precede binary and watcher problems" —
        /// the one scenario in this group needing a REAL watcher failure (design.md -> Test
        /// Boundaries: "notify real (unwatchable root)"). Driven through
        /// `start_collaborators` and `ui::view::render` directly rather than through
        /// `run_wired`: a genuinely unwatchable root on this platform (`notify`'s FSEvents
        /// backend) requires the directory to exist when `ui::load` resolves it and be gone
        /// by the time `watch::start` reaches it — measured directly against this crate's
        /// `notify` backend (a chmod-000 directory and an internal symlink loop both still
        /// watch successfully here; only a path absent from the filesystem at `.watch()` time
        /// fails). `run_wired`'s own single synchronous call gives a test no seam to remove
        /// the directory in between without racing, so this test calls `start_collaborators`
        /// directly instead, sequencing the removal deterministically — every other
        /// collaborator, and `notify` itself, stays real.
        #[test]
        fn startup_problems_are_ordered_config_then_binary_then_watcher() {
            for width in [120u16, 60u16] {
                let scratch = scratch_repo_with_alpha();
                let root = scratch.path().to_path_buf();
                let herdr = root.join("does-not-exist-herdr");

                let config = Config {
                    openspec_bin: Some(root.join("not-a-real-openspec-binary")),
                    problems: vec!["config.toml is not valid TOML: bad".to_string()],
                    ..Config::default()
                };

                let mut dashboard = super::super::load(&root, &config, None);
                assert_eq!(
                    dashboard.repo.as_deref(),
                    Some(canonical(&root).as_path()),
                    "width {width}"
                );

                // Remove the repository root entirely so the real `notify` watcher
                // genuinely cannot start — see the doc comment above.
                std::fs::remove_dir_all(&root).expect("remove the scratch repository");

                let collaborators = super::super::start_collaborators(
                    dashboard.repo.as_deref(),
                    &config,
                    &herdr,
                    None,
                    &no_env,
                    &no_npm_hook,
                );
                assert_eq!(
                    collaborators.problems.len(),
                    3,
                    "width {width}: {:?}",
                    collaborators.problems
                );
                assert!(
                    collaborators.problems[0].contains("config.toml is not valid TOML"),
                    "width {width}: {:?}",
                    collaborators.problems
                );
                assert!(
                    collaborators.problems[1].contains("openspec_bin"),
                    "width {width}: {:?}",
                    collaborators.problems
                );
                assert!(
                    collaborators.problems[2].contains("filesystem watch unavailable"),
                    "width {width}: {:?}",
                    collaborators.problems
                );

                dashboard.refresh.problems = collaborators.problems;
                dashboard.file_mode = collaborators.file_mode;

                let buf = render_at(width, 20, &dashboard);
                let rows: Vec<String> = (0..20).map(|y| row_text(&buf, y)).collect();
                let config_line = rows
                    .iter()
                    .position(|r| r.contains("config.toml is not valid TOML"));
                let bin_line = rows.iter().position(|r| r.contains("openspec_bin"));
                let watch_line = rows
                    .iter()
                    .position(|r| r.contains("filesystem watch unavailable"));
                assert!(
                    config_line.is_some() && bin_line.is_some() && watch_line.is_some(),
                    "width {width}: {rows:?}"
                );
                assert!(
                    config_line < bin_line && bin_line < watch_line,
                    "width {width}: not in causal order (config, binary, watcher): {rows:?}"
                );
            }
        }

        // --- degraded-states: group 8 proofs — launch, agent, and CLI/watcher rows ---------

        /// `degraded-coverage` :: "`g` with no attributed agent changes nothing the pane
        /// shows" — SPEC.md row 24. `herdr_script`'s reference payload names no `name` field
        /// at all, so attribution's name-equality tier matches nothing and `g` finds no pane
        /// to focus. Whole-buffer equality between a run that presses `g` and one that does
        /// not, plus the empty-argv-log proof that `g` issued no Herdr call at all.
        #[test]
        fn g_with_no_agent_renders_the_same_buffer() {
            for width in [120u16, 60u16] {
                let scratch = scratch_repo_with_alpha();
                let root = scratch.path();
                let herdr_log = root.join("herdr.log");
                let herdr = herdr_script(root, &herdr_log);
                let config = Config::default();

                let stage1 = || log_lines(&herdr_log) >= 1;
                let with_g: Vec<(&dyn Fn() -> bool, ratatui::crossterm::event::Event)> =
                    vec![(&stage1, key('g')), (&stage1, key('q'))];
                let (result_g, buf_g) =
                    run_wired_staged(width, root, &config, &herdr, None, with_g);
                let dashboard_g = result_g.expect("no attributed agent is a supported state");

                assert!(
                    non_agent_list_lines(&herdr_log).is_empty(),
                    "width {width}: g must issue no Herdr call at all: {:?}",
                    non_agent_list_lines(&herdr_log)
                );

                let herdr_log2 = root.join("herdr2.log");
                let herdr2 = herdr_script(root, &herdr_log2);
                let stage1b = || log_lines(&herdr_log2) >= 1;
                let without_g: Vec<(&dyn Fn() -> bool, ratatui::crossterm::event::Event)> =
                    vec![(&stage1b, key('q'))];
                let (result_plain, buf_plain) =
                    run_wired_staged(width, root, &config, &herdr2, None, without_g);
                let dashboard_plain = result_plain.expect("the control run is supported too");

                assert_eq!(
                    buf_g, buf_plain,
                    "width {width}: pressing g must render the identical buffer"
                );
                assert_eq!(dashboard_g.launch.pending, None, "width {width}");
                assert_eq!(dashboard_plain.launch.pending, None, "width {width}");
            }
        }

        /// A scratch `herdr` program for one of `every_launch_failure_renders_as_a_leading_row`'s
        /// six cases. Every case logs its argv exactly like `launch_herdr_script`; `case`
        /// selects which call (if any) fails and how. `"refusal"` reports `derived_name` as
        /// already live from the very first `agent list` poll, so `launch::decide` refuses
        /// before any of the other three calls are ever reached.
        fn launch_case_herdr_script(
            dir: &Path,
            log: &Path,
            marker: &Path,
            case: &str,
            derived_name: &str,
        ) -> PathBuf {
            let split_body = match case {
                "split" => "exit 3\n".to_string(),
                "malformed" => "printf '%s' '{\"id\":\"cli:pane:split\",\"result\":{}}'\n".to_string(),
                _ => "printf '%s' '{\"id\":\"cli:pane:split\",\"result\":{\"pane\":{\"agent_status\":\"unknown\",\"cwd\":\"/repo\",\"pane_id\":\"wD:pJ\",\"tab_id\":\"wD:t2\",\"workspace_id\":\"wD\"}}}'\n".to_string(),
            };
            let start_body = if case == "start" {
                "exit 3\n".to_string()
            } else {
                format!(
                    "printf 'name=%s\\n' \"$3\" > \"{}\"\nprintf '%s' '{{\"id\":\"cli:agent:start\",\"result\":{{\"agent\":{{\"name\":\"'\"$3\"'\",\"pane_id\":\"wD:pJ\"}}}},\"argv\":[\"claude\"],\"type\":\"agent_started\"}}}}'\n",
                    marker.display()
                )
            };
            let prompt_body = if case == "prompt" {
                "exit 3\n".to_string()
            } else {
                "printf '%s' '{\"id\":\"cli:agent:prompt\",\"result\":{\"agent\":{}},\"type\":\"agent_prompted\"}'\n"
                    .to_string()
            };
            let agent_list_body = if case == "refusal" {
                format!(
                    "printf '%s' '{{\"id\":\"cli:agent:list\",\"result\":{{\"agents\":[{{\"agent\":\"claude\",\"agent_status\":\"working\",\"name\":\"{derived_name}\",\"pane_id\":\"wD:pJ\",\"tab_id\":\"wD:t2\",\"workspace_id\":\"wD\"}}],\"type\":\"agent_list\"}}}}'\n"
                )
            } else {
                "printf '%s' '{\"id\":\"cli:agent:list\",\"result\":{\"agents\":[],\"type\":\"agent_list\"}}'\n"
                    .to_string()
            };

            write_script(
                dir,
                "herdr",
                &format!(
                    "printf '%s\\n' \"$*\" >> \"{log}\"\ncase \"$1 $2\" in\n  \"pane split\")\n    {split_body}    ;;\n  \"agent start\")\n    {start_body}    ;;\n  \"agent prompt\")\n    {prompt_body}    ;;\n  \"agent list\")\n    {agent_list_body}    ;;\nesac\n",
                    log = log.display(),
                ),
            )
        }

        /// `degraded-coverage` :: "A CLI root disagreement and a non-zero exit both leave
        /// the file numbers standing" — rows 33/34/38. The worker's initial file-sourced
        /// result already painted the pane before the failing CLI cycle completes, and
        /// `merge`'s own per-change fallback (an empty CLI-merged active list) leaves every
        /// file-sourced change's progress untouched — no special-case is needed to prove.
        #[test]
        fn a_failed_cli_cycle_keeps_the_file_numbers() {
            for kind in ["wrong_root", "nonzero"] {
                for width in [120u16, 60u16] {
                    let scratch = scratch_repo_with_alpha();
                    let root = scratch.path();
                    let herdr = root.join("does-not-exist-herdr");
                    let openspec_log = root.join("openspec.log");
                    let openspec = openspec_script_failing(root, &openspec_log, kind);
                    let config = Config {
                        openspec_bin: Some(openspec),
                        ..Config::default()
                    };
                    let predicate = || log_lines(&openspec_log) >= 1;
                    let (result, buf) =
                        run_wired_at(width, root, &config, &herdr, None, &predicate);
                    let dashboard = result.expect("a failed CLI cycle is a supported state");
                    assert!(
                        buf.iter()
                            .any(|row| row.contains("alpha") && row.contains("[4/9]")),
                        "kind {kind}, width {width}: the file-sourced numbers must still show: {buf:?}"
                    );
                    assert!(
                        !dashboard.changes.problems.is_empty(),
                        "kind {kind}, width {width}: the CLI failure must be recorded"
                    );
                    assert!(
                        buf.iter().any(|row| row.contains('!')),
                        "kind {kind}, width {width}: {buf:?}"
                    );
                }
            }
        }

        /// `degraded-coverage` :: "A watcher failure and a mid-run removal both keep the
        /// loop drawing" — rows 36/37. Sub-case 1: a genuinely unwatchable root — real
        /// `notify`, which on this platform's `notify` backend (FSEvents) only fails for a
        /// path absent from the filesystem at `.watch()`'s own call (measured directly:
        /// chmod 000 and an internal symlink loop both still succeed), so it is obtained
        /// independently of the dashboard's own, fully real, existing repository rather
        /// than by racing a removal against `ui::load`. Sub-case 2: `openspec/` removed
        /// while the watcher runs, driven through the real `run_wired` with the removal
        /// performed as a side effect of the predicate itself, on
        /// `the_real_wiring_polls_a_scratch_herdr`'s own established pattern for a mid-run
        /// filesystem change. Both sub-cases confirm `r` still forces a refresh request.
        #[test]
        fn a_watch_failure_keeps_the_loop_drawing() {
            // Sub-case 1.
            for width in [120u16, 60u16] {
                let scratch = scratch_repo_with_alpha();
                let root = scratch.path();
                let config = Config::default();
                let mut dashboard = super::super::load(root, &config, None);

                let unwatchable = root.join("does-not-exist-at-all");
                let (mut fs, watch_problems) = crate::watch::start(&unwatchable);
                assert!(
                    !watch_problems.is_empty(),
                    "width {width}: the watcher must genuinely fail here"
                );
                dashboard.refresh.problems = watch_problems;

                let mut refresher = crate::testutil::RecordingRefresher::new(Vec::new());
                let mut agents = crate::agents::none();
                let mut launcher = crate::launch::none();
                let mut live = crate::ui::driver::Live {
                    fs: &mut *fs,
                    refresher: &mut refresher,
                    agents: &mut *agents,
                    launcher: &mut *launcher,
                };

                let backend = ratatui::backend::TestBackend::new(width, 20);
                let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
                let mut events = crate::testutil::Script::new(vec![
                    Ok(Some(crate::testutil::press(
                        ratatui::crossterm::event::KeyCode::Char('r'),
                        ratatui::crossterm::event::KeyModifiers::NONE,
                    ))),
                    Ok(Some(crate::testutil::press(
                        ratatui::crossterm::event::KeyCode::Char('q'),
                        ratatui::crossterm::event::KeyModifiers::NONE,
                    ))),
                ]);
                let summary = crate::ui::driver::run_loop(
                    &mut terminal,
                    &mut dashboard,
                    &mut events,
                    &mut live,
                    &crate::ui::read_artifact,
                    std::time::Duration::from_millis(1),
                )
                .expect("the loop keeps drawing despite the watcher failure");
                assert!(summary.frames >= 2, "width {width}");

                let buf = terminal.backend().buffer();
                let saw_problem = (0..20u16).any(|y| {
                    (0..width)
                        .map(|x| buf[(x, y)].symbol().to_string())
                        .collect::<String>()
                        .contains('!')
                });
                assert!(
                    saw_problem,
                    "width {width}: the watch problem must still render"
                );
                assert!(
                    !refresher.requests().is_empty(),
                    "width {width}: r must still reach the refresher despite the degraded watcher"
                );
            }

            // Sub-case 2: `openspec/` removed mid-run.
            for width in [120u16, 60u16] {
                let scratch = scratch_repo_with_alpha();
                let root = scratch.path().to_path_buf();
                let herdr = root.join("does-not-exist-herdr");
                let removed = std::cell::Cell::new(false);
                let openspec_dir = root.join("openspec");
                let predicate = || {
                    if !removed.get() {
                        std::fs::remove_dir_all(&openspec_dir).ok();
                        removed.set(true);
                    }
                    true
                };
                let (result, buf) =
                    run_wired_at(width, &root, &Config::default(), &herdr, None, &predicate);
                let _dashboard =
                    result.expect("openspec/ removed mid-run must still be a supported state");
                assert!(
                    !buf.is_empty(),
                    "width {width}: the loop must still have drawn something: {buf:?}"
                );
            }
        }

        /// `degraded-coverage` :: "Each of the six launch failures renders as a leading
        /// problem row" — rows 19, 20, 21, 22, 23, and 25, each driven by a real `a`
        /// keypress through the real `run_wired`. Six cases, both mandated widths.
        #[test]
        fn every_launch_failure_renders_as_a_leading_row() {
            for width in [120u16, 60u16] {
                for case in ["split", "start", "prompt", "malformed", "record", "refusal"] {
                    let scratch = scratch_repo_with_2fa_support();
                    let root = scratch.path();
                    let herdr_log = root.join("herdr.log");
                    let marker = root.join("marker");
                    let herdr =
                        launch_case_herdr_script(root, &herdr_log, &marker, case, "c-2fa-support");
                    let openspec_log = root.join("openspec.log");
                    let openspec = openspec_script(root, &openspec_log, root);
                    let config = Config {
                        openspec_bin: Some(openspec),
                        ..Config::default()
                    };
                    // Every case but "record" gets a real, writable state directory, so
                    // `state::record` succeeds silently and contributes no problem of its
                    // own — otherwise a `None` state dir's own "no state directory could be
                    // resolved" problem would double up with the case under test. "record"
                    // itself points state_dir at an existing regular file instead, so
                    // `state::record` fails while the launch itself still succeeds.
                    let state = ScratchDir::new();
                    let bad_state_path = state.path().join("not-a-directory");
                    if case == "record" {
                        write_with_mode(&bad_state_path, b"x", 0o644);
                    }
                    let state_dir: Option<&Path> = if case == "record" {
                        Some(&bad_state_path)
                    } else {
                        Some(state.path())
                    };

                    let stage1 = || log_lines(&herdr_log) >= 1;
                    let expected_calls = match case {
                        "refusal" => 0,
                        "split" => 1,
                        "start" => 2,
                        "malformed" => 1,
                        "prompt" | "record" => 3,
                        other => unreachable!("unexpected case {other}"),
                    };
                    let stage2 = || non_agent_list_lines(&herdr_log).len() >= expected_calls.max(1);
                    let stages: Vec<(&dyn Fn() -> bool, ratatui::crossterm::event::Event)> =
                        if expected_calls == 0 {
                            vec![(&stage1, key('a')), (&stage1, key('q'))]
                        } else {
                            vec![(&stage1, key('a')), (&stage2, key('q'))]
                        };

                    let (result, buf) =
                        run_wired_staged(width, root, &config, &herdr, state_dir, stages);
                    let dashboard = result.expect("every launch failure is a supported state");

                    let calls = non_agent_list_lines(&herdr_log);
                    assert_eq!(
                        calls.len(),
                        expected_calls,
                        "case {case}, width {width}: {calls:?}"
                    );
                    assert_eq!(
                        dashboard.launch.problems.len(),
                        1,
                        "case {case}, width {width}: {:?}",
                        dashboard.launch.problems
                    );
                    if case == "refusal" {
                        let expected = "c-2fa-support is already running for this change - press g to focus it";
                        assert_eq!(
                            dashboard.launch.problems[0], expected,
                            "case {case}, width {width}"
                        );
                        if width == 120 {
                            assert!(
                                buf.iter().any(|row| row.contains('!')
                                    && row.contains("c-2fa-support is already running")),
                                "case {case}, width {width}: {buf:?}"
                            );
                        }
                    } else {
                        assert!(
                            buf.iter().any(|row| row.contains('!')),
                            "case {case}, width {width}: no leading problem row: {buf:?}"
                        );
                    }
                }
            }
        }
    }
}
