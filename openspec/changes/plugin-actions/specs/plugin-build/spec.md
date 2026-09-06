## MODIFIED Requirements

### Requirement: The dashboard's own starting directory prefers the workspace context over the process cwd

**Added correcting a finding from this change's own group 10 live check** — see
`specs/pane-open/spec.md` -> "`open`/`open-tab` never pass `--cwd`" for the full
measurement. Because `open`/`open-tab` never pass `--cwd`, every dashboard pane Herdr
starts (whether opened by an action or the pre-existing manual
`herdr plugin pane open --entrypoint dashboard`) runs with the **plugin root** as its OS
working directory. `ui::run` SHALL therefore prefer the workspace cwd from its own
injected `HERDR_PLUGIN_CONTEXT_JSON` / `HERDR_WORKSPACE_ID` environment — read through
`open::context`, exactly as `open::run_from_env` reads it — over
`std::env::current_dir()`, falling back to `std::env::current_dir()` only when no usable
Herdr context is present at all (a bare terminal, `cargo test`, or a Herdr older than
0.7.0's context injection). This is a pure decision, `ui::startup_cwd(env) ->
Option<PathBuf>`, taking the same injected-lookup shape `config::config_dir` and
`state::state_dir` already use.

#### Scenario: The workspace cwd is preferred when Herdr's context names one

- **WHEN** `ui::run`'s process environment carries `HERDR_WORKSPACE_ID` and a
  `HERDR_PLUGIN_CONTEXT_JSON` naming `workspace_cwd`
- **THEN** the dashboard searches for `openspec/` starting from that workspace cwd, not
  from the process's own working directory

#### Scenario: No Herdr context at all falls back to the process's own working directory

- **WHEN** `ui::run`'s process environment carries no Herdr variables
- **THEN** the dashboard searches starting from `std::env::current_dir()`, exactly as
  before this change — unchanged for a bare-terminal run, and for every existing
  `tui-shell`/`repo-resolution`/`changes-from-files` scenario, none of which sets Herdr
  context

### Requirement: The `ui` invocation runs the dashboard and needs a terminal

The binary SHALL accept exactly three arguments, each on its own and each with no flags:
`ui`, `open`, and `open-tab`. On `ui` it SHALL run the dashboard through `ui::run`, which
reads no arguments of its own; on `open` and `open-tab` it SHALL run `open::run_from_env`,
which never enters raw mode, never enters the alternate screen, and needs no terminal.
Argument classification SHALL remain a pure function in the library, with `src/main.rs`
restricted to reading arguments, dispatching, writing stderr, and setting the exit status —
no branch in `main` may compute anything a test cannot reach through `parse` or through
`open::run`.

Exit statuses SHALL be distinct and stable:

| Outcome | Status | Streams |
|---|---|---|
| The dashboard ran and the user quit | 0 | whatever the alternate screen held, which is discarded on leaving it |
| `open` or `open-tab` focused or opened the pane | 0 | stdout empty; stderr empty unless a degraded step recorded a warning, in which case each warning is one line |
| An unrecognised argument, no argument, or an argument after `ui`, `open`, or `open-tab` | 2 | usage on stderr, stdout empty |
| `ui` with stdout not a terminal | 3 | a message on stderr naming `herdr-openspec` and the words `not a terminal`, stdout empty |
| `ui` failed to start for any other reason | 1 | the error's `Display` text on stderr, stdout empty |
| `open` or `open-tab` could not read its Herdr context, or a `herdr` call failed | 1 | the reason on stderr, stdout empty |

Status **3 stays `ui`'s alone**: `open` and `open-tab` run from Herdr's action menu, where
stdout is measured not to be a terminal, so requiring one would make them unrunnable.

On status 3 the process SHALL exit without waiting on stdin. The previous behaviour blocked
until stdin reached EOF, so a test or script that piped the binary's output would hang
rather than fail; this change makes that case terminate.

#### Scenario: `ui` with stdout piped exits 3 without blocking

- **WHEN** the built binary is run as `herdr-openspec ui` with stdout and stderr captured
  through pipes and with stdin attached to a pipe whose write end is deliberately left
  **open**, so a wrongly blocking implementation would hang
- **THEN** it exits with status 3, polled to a ten-second deadline that fails the test if
  the process is still alive when it expires
- **AND** stdout is empty
- **AND** stderr contains `herdr-openspec` and the words `not a terminal`

#### Scenario: The three failing statuses are distinct

- **WHEN** the built binary is run four ways with stdout piped and stdin at EOF: `ui`,
  `wat`, `ui --tab`, and with no arguments
- **THEN** the exit statuses are 3, 2, 2, and 2 respectively
- **AND** the three status-2 runs still print the usage text, which now lists `ui`, `open`,
  and `open-tab`, and the `wat` and `ui --tab` runs still name the rejected token
- **AND** no run prints anything to stdout

#### Scenario: The open family never reaches status 3

- **WHEN** the built binary is run as `herdr-openspec open` and `herdr-openspec open-tab`
  with stdout piped, stdin at EOF, and every `HERDR_*` variable removed from the child's
  environment
- **THEN** both exit 1, never 3, within a ten-second deadline
- **AND** stderr names the missing Herdr context and stdout is empty
- **AND** `herdr-openspec open --tab` exits 2 with usage, proving no flag grammar was added

#### Scenario: Every binary-integration run pipes stdout

- **WHEN** `tests/cli.rs` reads its own source through `std::fs::read_to_string(file!())`
  and inspects every place it spawns the crate's own binary — an executing test, not the
  manual inspection this scenario was satisfied by before this change
- **THEN** each spawn site sets stdout to a pipe or to null, never inheriting the test
  process's own terminal, so the status-3 branch is the branch the suite reaches
- **AND** every spawn site whose argument list names `open` or `open-tab` also goes through
  the helper that removes every `HERDR_*` variable from the child's environment, so a suite
  running inside a Herdr pane cannot inherit a live context and reach the real socket
- **AND** the inspection reports the number of spawn sites it found and fails below the
  count this change measures, so a rewritten harness that hid every site cannot pass
- **AND** the tree-wide containment of the crossterm terminal-mode functions is
  `terminal-lifecycle`'s requirement, not restated here — one check, one owner
