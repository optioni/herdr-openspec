## Context

Herdr persists a session's layout and, after a server restart, restores every pane — but
only agents with a native or reported resume path get their process back. A dashboard pane
returns as a fresh login shell in its saved cwd, keeping its pane id and `OpenSpec` label
and losing plugin ownership (`notes/measurements.md` M2, M6). Herdr stores the pane's
`launch_argv` in `session.json` and does not re-run it.

Herdr 0.7.5 added one-shot `[[startup]]` plugin hooks, run once per enabled plugin after the
session is restored and again after a live handoff, with the normal runtime plugin
environment. `src/open.rs` is already the crate's one-shot `HerdrCli` consumer: it lists
panes, focuses or opens a dashboard, and exits, with every decision a pure function over
Herdr's stdout and a `Report { warnings, outcome }` that `src/main.rs` turns into stderr
lines and an exit status without a branch of its own.

## Goals / Non-Goals

**Goals:**

- After a restart, every dashboard pane in the saved layout is a live dashboard again, in
  the same slot, without user action.
- The "OpenSpec: dashboard" actions never focus a dead shell, and always raise what they
  open or replace.
- Nothing that is not a dead dashboard is ever replaced.

**Non-Goals:** as the proposal lists — no dashboard state carried over, no new state file,
no agent-resume route, no retrying daemon.

## Boundaries

- **`src/open.rs`** gains a three-way classifier (`classify(&str) -> Classification`, with
  `Stale`, `Live`, and `Unreadable(reason)`), `restore_candidates`, the `replace` operation,
  the `restore` sweep, a pure `plugin_id(env)` that `context` and `restore_from_env` both
  call, and `restore_from_env`, its production binding. It follows `open::run`'s shape
  exactly: `&dyn HerdrCli` injected, JSON parsed on the testable side of the seam, failures
  carried as `herdr_reason` strings, a `Report` returned. It starts no thread.
  `LAUNCHSEAM` leg 2 rejects `Rect|Frame|Buffer|Style` anywhere in its production slice,
  comments included, so the slot measurements are described in lower case there.
- **`src/lib.rs`**: `Invocation::Restore`, `parse`, `usage`; `open::placement_for` gains a
  `Restore => None` arm.
- **`src/main.rs`**: one `Invocation::Restore` arm calling the same reporter `run_open` uses,
  with no branch of its own (on `open`'s existing terms).
- **`src/cli.rs`**: unchanged. Every new Herdr call (`pane process-info`, `pane close`) goes
  through the existing `HerdrCli::run`. **No process spawn is added outside `cli`.**
- **`src/ui/`**: unchanged. No view, no I/O added to a view. `src/ui/mod.rs`'s
  `set_current_dir` (commit `49e3fc7`) is what makes a restored shell land in the repo
  rather than the plugin root; this change depends on it only cosmetically.
- **`herdr-plugin.toml`**: `[[startup]]`, `min_herdr_version`.
- The `Change` type is **not** altered; `from_files` and `from_cli` are untouched.
- Seam gates: `src/open.rs` is already on `LAUNCHSEAM`'s and `AGENTSEAM`'s five-file
  `HerdrCli` lists and is already outside `NOBLOCK`'s seam modules because it starts no
  thread; none of those lists moves. `serde_json` stays out of `src/cli.rs`.
  `tests/cli.rs`'s `every_run_pipes_and_scrubs_herdr` guard learns `restore`, so a test
  spawning it cannot inherit the developer's live `HERDR_SOCKET_PATH`.

## Contracts

- **Plugin manifest (consumer: Herdr):** additive `[[startup]]` table; `min_herdr_version`
  `0.7.0` → `0.7.5` is **breaking** for users on Herdr 0.7.0–0.7.4, who must upgrade.
- **Binary CLI (consumer: the manifest only):** additive `restore` subcommand. Exit status 0
  when the sweep ran, 1 when `pane list` failed; stderr carries one line per warning. Never
  exits 2 or 3. `ui`/`open`/`open-tab` keep their exit statuses.
- **`open`/`open-tab` behaviour (consumer: the user):** one extra `pane process-info` call
  before focusing an existing pane; a stale pane is replaced in place and the replacement is
  raised through the existing post-open re-list and focus.
- **Herdr CLI surfaces consumed:** `pane list` (already), `pane process-info --pane <id>`
  (Herdr 0.7.0+), `plugin pane open … --target-pane` (already), `plugin pane focus`
  (already), `pane close <id>`.

## Persistence and Rollout

Migration: none. Backfill: none. Seeding: none. Cache invalidation: none. Index rebuild:
none. Authorization: none — Herdr's install preview lists the startup command for the user
to review. Observability: `restore`'s stderr lands in Herdr's plugin command log
(`herdr plugin log`). Deployment: users get the hook on `herdr plugin install`/update.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| Herdr (the `herdr` CLI / socket) | replaced — a stub `herdr` script on `PATH` that logs argv and answers from a script (`tests/cli.rs`'s `write_stub_herdr`) | replaced — `cli::FakeCli` scripted per argv |
| Herdr, live | real — an **isolated** Herdr instance for the live check: `HOME` and `XDG_CONFIG_HOME` pointed at a scratch directory (measured: its session directory and plugin registry then live there), the working tree linked into it, driven from tmux. Never the user's own instance | not used |
| tmux | live check only, to host the isolated Herdr client | not used |
| Process environment | real, scrubbed (`tests/cli.rs`'s `scrubbed()`) | replaced — injected `&dyn Fn(&str) -> Option<String>` for `plugin_id` |
| Filesystem | the stub's argv log in a `ScratchDir`; `restore` itself touches none | not touched |
| `openspec` binary | not touched | not touched |
| Terminal | not touched — `restore` runs with piped stdio | not touched |
| `git` | not touched | not touched |

## Test Strategy

This change **takes the outer-loop acceptance test**: `tests/cli.rs` runs the real binary
as `herdr-openspec restore` against a stub `herdr` listing two stale dashboards in two
workspaces and one live one, and asserts the logged call sequence, stderr, and exit status.
It fails first on `unrecognized argument: restore`. Everything below it is unit-tested in
`src/open.rs`'s `#[cfg(test)]` module against `FakeCli`, and in `src/lib.rs` for `parse`/
`usage`. The manifest is checked by `tests/manifest.rs`. What no stub can prove — the hook
firing, its environment and timing, slot preservation, the replacement's context — is the
isolated live check, recorded in `notes/measurements.md`.

Filters below were run at HEAD `e678de5` and their selected counts recorded; a row naming a
new test selects it by its exact name.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| A restored shell is stale | `classify` over M6's restored-shell JSON | unit | none | `cargo test -q --lib open::tests::a_restored_shell_is_stale` |
| A live dashboard is not stale | `classify` over M6's live-dashboard JSON → `Live` | unit | none | `cargo test -q --lib open::tests::a_live_dashboard_is_not_stale` |
| A command the user started in the restored shell is not stale | `classify`: different pid; two foreground processes → `Live` | unit | none | `cargo test -q --lib open::tests::a_command_the_user_started` |
| An unreadable answer is never stale, and says why | `classify` over each malformed shape → `Unreadable` with a reason, table-driven | unit | none | `cargo test -q --lib open::tests::an_unreadable_answer` |
| An unlabelled pane is never a candidate | `restore_candidates` over a mixed listing | unit | none | `cargo test -q --lib open::tests::an_unlabelled_pane` |
| The full call sequence for one stale pane | `replace` against `FakeCli`, recorded argv | unit | FakeCli | `cargo test -q --lib open::tests::the_full_call_sequence` |
| A failed open closes nothing | `replace`, open scripted to fail | unit | FakeCli | `cargo test -q --lib open::tests::a_failed_open_closes_nothing` |
| A failed close keeps the replacement and warns | `replace`, close scripted to fail | unit | FakeCli | `cargo test -q --lib open::tests::a_failed_close_keeps` |
| Two stale dashboards in two workspaces are both replaced | `restore` against `FakeCli`, focus flags asserted | unit | FakeCli | `cargo test -q --lib open::tests::two_stale_dashboards` |
| Two stale dashboards in two workspaces are both replaced | binary against stub `herdr`, full argv including `--plugin herdr-openspec` | acceptance | stub herdr, scrubbed env | `cargo test -q --test cli restore_replaces` |
| A live dashboard is left alone | `restore`, call list exactly two entries, no warning | unit | FakeCli | `cargo test -q --lib open::tests::a_live_dashboard_is_left_alone` |
| A live dashboard is left alone | the acceptance run's live pane gets no open/close | acceptance | stub herdr | `cargo test -q --test cli restore_replaces` |
| No dashboard pane at all | `restore`, empty and unlabelled listings | unit | FakeCli | `cargo test -q --lib open::tests::no_dashboard_pane_at_all` |
| One failed replacement does not stop the sweep | `restore`, first open fails | unit | FakeCli | `cargo test -q --lib open::tests::one_failed_replacement` |
| An unreadable candidate is left alone and warned about | `restore`, process-info failing and `not json` | unit | FakeCli | `cargo test -q --lib open::tests::an_unreadable_candidate` |
| Herdr unreachable | `restore`: NotStarted, Failed, payload without `result.panes`; `report_output` status 1 | unit | FakeCli | `cargo test -q --lib open::tests::herdr_unreachable` |
| Herdr unreachable | binary with a stub exiting 1 on `pane list`: exit 1, argv log exactly one line | acceptance | stub herdr | `cargo test -q --test cli restore_exits_one` |
| No workspace in the environment is not a failure | the acceptance run itself: scrubbed env, piped stdio, exit 0 (never 3) | acceptance | stub herdr, scrubbed env | `cargo test -q --test cli restore_replaces` |
| The plugin id comes from `HERDR_PLUGIN_ID`, falling back to `herdr-openspec` | `plugin_id(env)` set / blank / unset | unit | injected env | `cargo test -q --lib open::tests::plugin_id` |
| The plugin id comes from `HERDR_PLUGIN_ID`, falling back to `herdr-openspec` | acceptance run asserts `--plugin herdr-openspec` with the variable scrubbed | acceptance | stub herdr | `cargo test -q --test cli restore_replaces` |
| `open` alone classifies as the split-pane invocation | existing test, unchanged | unit | none | `cargo test -q --lib classifies_as` (3 at HEAD) |
| `open-tab` alone classifies as the tab invocation | existing test, unchanged | unit | none | `cargo test -q --lib classifies_as` (3 at HEAD) |
| A flag after a subcommand is rejected, carrying the token | existing test plus `restore --all` | unit | none | `cargo test -q --lib a_flag_after_a_subcommand` (1 at HEAD) |
| `restore` alone classifies as the restore invocation | `parse(&["restore"])` | unit | none | `cargo test -q --lib classifies_as` (4 after) |
| Usage names all four commands | `usage()` whole-token command lines and summary; `tests/cli.rs`'s usage assertion | unit + acceptance | none | `cargo test -q --lib usage_` (3 at HEAD); `cargo test -q --test cli` |
| A matching pane is focused and nothing is opened | `open::run`, live classification: three calls | unit | FakeCli | `cargo test -q --lib open::tests::the_already_open_path_is_untouched` |
| No listed pane matches, so one is opened | existing tests, unchanged (no classification call) | unit | FakeCli | `cargo test -q --lib open::` (53 at HEAD) |
| A matching entry with no usable `pane_id` is not a match | existing test, unchanged | unit | FakeCli | `cargo test -q --lib open::` |
| A labelled pane in another workspace is not this workspace's dashboard | existing test, unchanged | unit | FakeCli | `cargo test -q --lib open::` |
| Two matches focus the first in list order | existing pure `dashboard_panes` test, unchanged | unit | none | `cargo test -q --lib open::` |
| `open-tab` focuses a split dashboard, and `open` focuses a tab one | `cross_placement_focus`, plus a live classification | unit | FakeCli | `cargo test -q --lib open::tests::cross_placement_focus` |
| A stale match is replaced in place and the replacement focused | `open::run`, stale classification, six recorded calls ending in focus of the new id | unit | FakeCli | `cargo test -q --lib open::tests::a_stale_match_is_replaced` |
| A failed replacement open stops the command and closes nothing | `open::run`, replacement open fails | unit | FakeCli | `cargo test -q --lib open::tests::a_failed_replacement_open` |
| A failed close after a successful replacement warns and still focuses | `open::run`, close fails | unit | FakeCli | `cargo test -q --lib open::tests::a_failed_close_after` |
| An unclassifiable match is focused, never replaced | `open::run`, process-info failing and malformed; live answer records no warning | unit | FakeCli | `cargo test -q --lib open::tests::an_unclassifiable_match` |
| A domain error is carried verbatim and stops the command | existing test, unchanged | unit | FakeCli | `cargo test -q --lib open::` |
| A usage error on the open call is treated identically, without being parsed | existing test, unchanged | unit | FakeCli | `cargo test -q --lib open::` |
| An unstartable `herdr` names the program | existing test, unchanged | unit | FakeCli | `cargo test -q --lib open::` |
| A domain-error focus stops the command and opens nothing | `focus_domain_error_stops_and_opens_nothing`, three calls | unit | FakeCli | `cargo test -q --lib open::tests::focus_domain_error` |
| A usage-error focus warns and opens once | `focus_usage_error_warns_and_opens_once`, process-info second | unit | FakeCli | `cargo test -q --lib open::tests::focus_usage_error` |
| A failed listing warns and still opens | existing test, unchanged | unit | FakeCli | `cargo test -q --lib open::` |
| An unparseable listing warns and still opens | existing test, unchanged | unit | FakeCli | `cargo test -q --lib open::` |
| A successful open is silent | existing test, unchanged | unit | FakeCli | `cargo test -q --lib open::` |
| A tab open is followed by a listing and a focus | existing test, unchanged | unit | FakeCli | `cargo test -q --lib open::` |
| A split open takes the identical path, with no placement branch | existing test, unchanged | unit | FakeCli | `cargo test -q --lib open::` |
| A failed second listing warns and still reports success | existing test, unchanged | unit | FakeCli | `cargo test -q --lib open::` |
| An unparseable second listing warns and still reports success | existing test, unchanged | unit | FakeCli | `cargo test -q --lib open::` |
| An empty second listing warns rather than focusing nothing | existing test, unchanged | unit | FakeCli | `cargo test -q --lib open::` |
| A post-open focus failure warns, on every code, including the one that stops the pre-open focus | existing test, unchanged | unit | FakeCli | `cargo test -q --lib open::` |
| The post-open focus names the newly opened pane, not the one already listed | `the_post_open_focus_names_the_newly_opened_pane`, six calls | unit | FakeCli | `cargo test -q --lib open::tests::the_post_open_focus_names` |
| A failed open makes no post-open call at all | existing test, unchanged | unit | FakeCli | `cargo test -q --lib open::` |
| The already-open path is untouched | `the_already_open_path_is_untouched`, three calls | unit | FakeCli | `cargo test -q --lib open::tests::the_already_open_path_is_untouched` |
| Herdr links the working tree | `herdr plugin link .` into the isolated instance; `plugin list` and `plugin action list` | live check | isolated Herdr | recorded in `notes/measurements.md` |
| The dashboard pane launches the binary and stays open | open the `dashboard` entry in the isolated instance | live check | isolated Herdr | recorded in `notes/measurements.md` |
| The tab pane opens in its own tab | open the `dashboard-tab` entry in the isolated instance | live check | isolated Herdr | recorded in `notes/measurements.md` |
| The manifest parses and its declared paths resolve | `tests/manifest.rs`: exactly one `[[startup]]`, only a `command` key of two elements, path equal to the panes', argument parsing as `Invocation::Restore`; `min_herdr_version` deliberately unasserted, on that file's existing terms | contract | repository files | `cargo test -q --test manifest` (4 at HEAD) |
| (the hook, end to end) | restart the isolated instance with a split and a tab dashboard open; Herdr itself runs the hook; both return live in their slots; `herdr plugin log` shows the run and its environment | live check | isolated Herdr, tmux | recorded in `notes/measurements.md` |

## Decisions

1. **A `[[startup]]` hook that replaces dead panes**, over Herdr's agent resume-command
   route (needs Herdr 0.10.0 and a `PATH`-resolvable binary, and surfaces the dashboard as an
   agent — in this plugin's own badges too) and over waiting for Herdr to re-run
   `launch_argv` (not in our control).
2. **No record of where dashboards were open.** The restored pane's id, label, and process
   are enough evidence (M2, M5, M6), and every restored shell is fresh, so replacing one loses
   no process. A record would add a third state file — breaking the "exactly
   `agent-names.toml` and `settings.toml`" write set — and would need keying per named
   session, for no extra safety.
3. **Stale = exactly one foreground process, its pid is `shell_pid`, and its `name` is not
   `herdr-openspec`.** Measured on M6's ten real panes with no misclassification.
   Alternatives: `shell_pid` alone (wrong — a live dashboard satisfies it, M5); a login-shell
   `argv[0]` starting with `-` (depends on how Herdr spawns shells); a list of shell names
   (always incomplete, and `name` is not reliably a program's name — M6). A user's idle shell
   renamed `OpenSpec` will be replaced; after a restart it holds nothing.
4. **Classification is three-way.** `Unreadable` carries its reason so callers can warn,
   while `Live` stays silent; both mean "never replace".
5. **Split-then-close in the stale pane's slot** (M3), over opening a fresh tab or split
   elsewhere (loses the slot and leaves the dead shell), and over typing `exec … ui` into
   the shell with `pane run` (the pane is no longer plugin-owned, lacks the plugin
   environment, and the command lands in shell history). Always the `dashboard` entrypoint,
   because the operation is always a split.
6. **`open`'s replacement joins the existing post-open path.** `--focus` on an open does not
   raise a background tab (archived `tab-open-focus`, measured on Herdr 0.9.0); the
   re-list-and-focus that follows every successful open does, and identifies the new pane by
   difference because the stale id is in the pre-open list and gone after the close.
7. **Lives in `src/open.rs`**, not a new `src/restore.rs`: a new `HerdrCli` consumer file
   would move `LAUNCHSEAM`'s and `AGENTSEAM`'s five-file lists, the module map, and the
   tested-modules list in `tests/doc_contract.rs`, for a module of a few functions sharing
   `open`'s constants, `herdr_reason`, and `Report`.
8. **Focus mirrors the stale pane's own `focused` flag** in `restore` (session-wide, exactly
   one pane is focused — confirmed live), so a restart leaves focus where it was; `open`'s
   replacement always focuses, because the user asked to see the dashboard.
9. **`open` classifies only its first match, and an unreadable match is focused as before**
   with a warning. Never replacing on weak evidence is the rule this project already applies
   to agent attribution.
10. **`restore` exits 0 whenever the listing succeeded.** Per-pane failures are warnings;
    Herdr does not act on a hook's status beyond logging it.
11. **`min_herdr_version = "0.7.5"`**, the release that introduced `[[startup]]`.
12. **New `SPEC.md` degraded-states rows**, each bound in `tests/degraded-coverage.toml` with
    all five fields (`tier`, `proof`, `verdict`, `why`, `covers`): `restore`'s `pane list`
    failing; a candidate unreadable (left alone, warned); a replacement's open failing (shell
    left, sweep continues); a replacement's close failing (warning); `open`'s classification
    unreadable (focus as before); `open`'s replacement open failing (exit 1). Group 5 shifts
    `open::run`'s lines, so the six existing `src/open.rs` `covers` ranges are re-pointed in
    the same task.
13. **The live check runs in an isolated Herdr instance**, not a named session of the user's
    own: named sessions share `~/.config/herdr/plugins.json`, so linking the working tree
    there would re-point the user's default session too. A scratch `HOME`/`XDG_CONFIG_HOME`
    gives Herdr its own session directory and plugin registry (measured), so the working tree
    can be linked and the real hook observed.

## Risks / Trade-offs

- [The hook may run before a client attaches, and `plugin pane open` may fail without a
  terminal size] → observed in the isolated live check; if it fails, the stale pane is left
  in place (Decision 5's failure rule) and `open`'s replace path recovers it on the user's
  first "OpenSpec: dashboard" action. Recorded either way.
- [A restored login shell may still be running its profile when the hook fires, so its
  foreground set is not the lone shell] → it classifies `Live` and is left alone; `open`
  recovers it later. The live check records whether this happens in practice.
- [The hook's environment might not carry the named session's `HERDR_SOCKET_PATH`, sending
  the sweep to the default session] → Herdr documents injecting `HERDR_SOCKET_PATH` into
  every runtime command; the live check records the hook's environment from `herdr plugin
  log`. If it is missing, `restore` must refuse rather than sweep another session — a
  follow-up, not silently accepted.
- [A replacement opened from the hook may receive a context without `workspace_cwd`] →
  observed in the live check; `ui::startup_cwd` already falls back to `focused_pane_cwd`,
  and the replacement targets the stale pane, whose cwd is the repo after `49e3fc7`.
- [Herdr runs the hook again after a live handoff] → live dashboards classify `Live` (M5, M6)
  and are left alone.
- [An `open` action racing the startup hook on the same stale pane could produce two
  replacements] → accepted: the hook runs at server start, before a user can invoke an
  action in practice; a duplicate dashboard is closable and harmless.
- [A user's idle shell labelled `OpenSpec` is replaced] → accepted (Decision 3); documented
  in `README.md`.
- [Herdr changes `process-info`'s shape] → every unreadable shape is `Unreadable`, so the
  failure mode is "no restore, with a warning", never "a wrong pane closed".

## Migration Plan

None for data. Users on Herdr below 0.7.5 must upgrade before updating the plugin.
Rollback: revert the commits; the `[[startup]]` table disappears with them and Herdr stops
running the hook.

## Open Questions

None blocking. The unmeasured items in `notes/measurements.md` (hook timing, hook
environment, the replacement's context) are observed by the isolated live check in tasks
group 9, after the manifest is wired, because only a manifest carrying `[[startup]]` lets
Herdr run the real hook. No decision waits on them: each has a stated fallback in Risks, and
the one outcome that would demand new behaviour (a missing socket path) is named there.
