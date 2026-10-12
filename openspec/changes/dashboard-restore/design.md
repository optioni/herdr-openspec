## Context

Herdr persists a session's layout and, after a server restart, restores every pane — but
only agents with a native or reported resume path get their process back. A dashboard pane
returns as a fresh login shell in its saved cwd, keeping its pane id and `OpenSpec` label
and losing plugin ownership (`notes/measurements.md` M2). Herdr stores the pane's
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
- The "OpenSpec: dashboard" actions never focus a dead shell.
- Nothing that is not a dead dashboard is ever replaced.

**Non-Goals:** as the proposal lists — no dashboard state carried over, no new state file,
no agent-resume route, no retrying daemon.

## Boundaries

- **`src/open.rs`** gains the stale-pane classifier, the replace operation, the `restore`
  sweep, and its `run_from_env`-shaped production binding. It follows `open::run`'s
  existing shape exactly: `&dyn HerdrCli` injected, JSON parsed on the testable side of the
  seam, failures carried as `herdr_reason` strings, a `Report` returned. It starts no thread.
- **`src/lib.rs`**: `Invocation::Restore`, `parse`, `usage`.
- **`src/main.rs`**: one `Invocation::Restore` arm calling the existing `run_open`-style
  reporter (no branch of its own, on `open`'s Decision 10 terms).
- **`src/cli.rs`**: unchanged. Every new Herdr call (`pane process-info`, `pane close`) goes
  through the existing `HerdrCli::run`. **No process spawn is added outside `cli`.**
- **`src/ui/`**: unchanged. No view, no I/O added to a view. `src/ui/mod.rs`'s
  `set_current_dir` (commit `49e3fc7`) is what makes a restored shell land in the repo
  rather than the plugin root; this change depends on it only cosmetically.
- **`herdr-plugin.toml`**: `[[startup]]`, `min_herdr_version`.
- The `Change` type is **not** altered; `from_files` and `from_cli` are untouched.
- Seam gates: `src/open.rs` is already on `LAUNCHSEAM`'s five-file `HerdrCli` list and is
  already excluded from `NOBLOCK`'s seam modules because it starts no thread; neither list
  moves. `serde_json` stays out of `src/cli.rs`.

## Contracts

- **Plugin manifest (consumer: Herdr):** additive `[[startup]]` table; `min_herdr_version`
  `0.7.0` → `0.7.5` is **breaking** for users on Herdr 0.7.0–0.7.4, who must upgrade
  (Herdr refuses to load a plugin above its version).
- **Binary CLI (consumer: the manifest only):** additive `restore` subcommand. Exit status 0
  when the sweep ran, 1 when `pane list` failed; stderr carries one line per warning.
  `ui`/`open`/`open-tab` keep their exit statuses.
- **`open`/`open-tab` behaviour (consumer: the user):** one extra `pane process-info` call
  before focusing an existing pane; a stale pane is replaced rather than focused.
- **Herdr CLI surfaces consumed:** `pane list` (already), `pane process-info --pane <id>`
  (Herdr 0.7.0+), `plugin pane open … --target-pane` (already), `pane close <id>`.

## Persistence and Rollout

Migration: none. Backfill: none. Seeding: none. Cache invalidation: none. Index rebuild:
none. Authorization: none — Herdr's install preview lists the startup command for the user
to review. Observability: `restore`'s stderr lands in Herdr's plugin command log
(`herdr plugin log`). Deployment: users get the hook on `herdr plugin install`/update; a
linked working tree needs `make build` and a server restart to exercise it.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| Herdr (the `herdr` CLI / socket) | replaced — a stub `herdr` script on `PATH` that records argv and answers from a script (`tests/cli.rs`'s `write_stub_herdr` pattern) | replaced — `cli::FakeCli` scripted per argv |
| Herdr, live | real — the task-group live check in a throwaway named session (`herdr --session restore-test`), never the user's default session | not used |
| Process environment | real, scrubbed (`tests/cli.rs`'s `scrubbed()`), with `HERDR_PLUGIN_ID` set or unset per case | replaced — injected `&dyn Fn(&str) -> Option<String>` |
| Filesystem | not touched by `restore` | not touched |
| `openspec` binary | not touched | not touched |
| Terminal | not touched — `restore` never renders and needs no tty | not touched |
| `git` | not touched | not touched |

## Test Strategy

This change **takes the outer-loop acceptance test**: `tests/cli.rs` runs the real binary
as `herdr-openspec restore` against a stub `herdr` that lists one stale and one live
dashboard pane, and asserts the recorded call sequence, stderr, and exit status. It fails
first on `unrecognized argument: restore`. Everything below it is unit-tested in
`src/open.rs`'s `#[cfg(test)]` module against `FakeCli`, and in `src/lib.rs` for `parse`/
`usage`. The manifest is checked by `tests/manifest.rs`. Live Herdr behaviour that no stub
can prove — hook timing, the replacement's context, slot preservation — is a recorded live
check, written into `notes/measurements.md`.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| A restored shell is stale | `is_stale` over the M2 `process-info` JSON | unit | none | `cargo test --lib open::` |
| A live dashboard is not stale | `is_stale` over the M5 JSON | unit | none | `cargo test --lib open::` |
| A command the user started in the restored shell is not stale | `is_stale`, different pid; two foreground processes | unit | none | `cargo test --lib open::` |
| An unreadable answer is never stale | `is_stale` over each malformed shape, table-driven | unit | none | `cargo test --lib open::` |
| An unlabelled pane is never a candidate | `restore_candidates` over a mixed listing | unit | none | `cargo test --lib open::` |
| The full call sequence for one stale pane | `replace` against `FakeCli`, recorded argv | unit | FakeCli | `cargo test --lib open::` |
| A failed open closes nothing | `replace` with the open scripted to fail | unit | FakeCli | `cargo test --lib open::` |
| A failed close keeps the replacement and warns | `replace` with the close scripted to fail | unit | FakeCli | `cargo test --lib open::` |
| Two stale dashboards in two workspaces are both replaced | `restore::run` against `FakeCli`, focus flags asserted | unit | FakeCli | `cargo test --lib open::` |
| Two stale dashboards in two workspaces are both replaced | binary run against stub `herdr` | acceptance | stub herdr, real env | `cargo test --test cli restore` |
| A live dashboard is left alone | `restore::run`, call list exactly two entries | unit | FakeCli | `cargo test --lib open::` |
| A live dashboard is left alone | the acceptance run's live pane gets no open/close | acceptance | stub herdr | `cargo test --test cli restore` |
| No dashboard pane at all | `restore::run`, empty and unlabelled listings | unit | FakeCli | `cargo test --lib open::` |
| One failed replacement does not stop the sweep | `restore::run`, first open fails | unit | FakeCli | `cargo test --lib open::` |
| Herdr unreachable | `restore::run`: NotStarted, Failed, payload without `result.panes`; `report_output` status 1 | unit | FakeCli | `cargo test --lib open::` |
| Herdr unreachable | binary run with a stub `herdr` exiting 1: exit status 1, stderr reason | acceptance | stub herdr | `cargo test --test cli restore` |
| No workspace in the environment is not a failure | binary run with scrubbed env, no workspace vars | acceptance | stub herdr, real env | `cargo test --test cli restore` |
| `open` alone classifies as the split-pane invocation | existing `parse` test, unchanged | unit | none | `cargo test --lib parse` |
| `open-tab` alone classifies as the tab invocation | existing `parse` test, unchanged | unit | none | `cargo test --lib parse` |
| A flag after a subcommand is rejected, carrying the token | existing test plus `restore --all` | unit | none | `cargo test --lib parse` |
| `restore` alone classifies as the restore invocation | `parse(&["restore"])` | unit | none | `cargo test --lib parse` |
| Usage names all four commands | `usage()` whole-token command lines and summary line | unit | none | `cargo test --lib usage` |
| A matching pane is focused and nothing is opened | `open::run` with a live classification: three calls | unit | FakeCli | `cargo test --lib open::` |
| No listed pane matches, so one is opened | existing test, unchanged (no classification call) | unit | FakeCli | `cargo test --lib open::` |
| A matching entry with no usable `pane_id` is not a match | existing test, unchanged | unit | FakeCli | `cargo test --lib open::` |
| A labelled pane in another workspace is not this workspace's dashboard | existing test, unchanged | unit | FakeCli | `cargo test --lib open::` |
| Two matches focus the first in list order | existing test, plus a live classification for the first | unit | FakeCli | `cargo test --lib open::` |
| `open-tab` focuses a split dashboard, and `open` focuses a tab one | existing test, plus a live classification | unit | FakeCli | `cargo test --lib open::` |
| A stale match is replaced in place and the replacement focused | `open::run` with a stale classification, recorded argv | unit | FakeCli | `cargo test --lib open::` |
| An unclassifiable match is focused, never replaced | `open::run`, process-info failing and malformed | unit | FakeCli | `cargo test --lib open::` |
| Herdr links the working tree | live: `herdr plugin link .` in the throwaway session, `plugin list` | live check | real Herdr | recorded in `notes/measurements.md` |
| The dashboard pane launches the binary and stays open | live check, unchanged behaviour | live check | real Herdr | recorded in `notes/measurements.md` |
| The tab pane opens in its own tab | live check, unchanged behaviour | live check | real Herdr | recorded in `notes/measurements.md` |
| The manifest parses and its declared paths resolve | `tests/manifest.rs`: exactly one `[[startup]]`, its path equal to the panes' and its argument parsing as `Invocation::Restore`; `min_herdr_version` deliberately unasserted, on that file's existing terms | contract | filesystem (repo) | `cargo test --test manifest` |
| (restore, end to end) | live: restart the throwaway session with two dashboards open; both come back live in their slots | live check | real Herdr | recorded in `notes/measurements.md` |

## Decisions

1. **A `[[startup]]` hook that replaces dead panes**, over Herdr's agent resume-command
   route (needs Herdr 0.10.0 and a `PATH`-resolvable binary, and surfaces the dashboard as an
   agent — in this plugin's own badges too) and over waiting for Herdr to re-run
   `launch_argv` (not in our control).
2. **No record of where dashboards were open.** The restored pane's id, label, and process
   are enough evidence (M2, M5), and every restored shell is fresh, so replacing one loses
   no process. A record would add a third state file — breaking the "exactly
   `agent-names.toml` and `settings.toml`" write set — and would need keying per named
   session, for no extra safety.
3. **Stale = exactly one foreground process, its pid is `shell_pid`, and its `name` is not
   `herdr-openspec`.** Alternatives: `shell_pid` alone (wrong — a live dashboard satisfies it,
   M5); a login-shell `argv[0]` starting with `-` (depends on how Herdr spawns shells); a
   list of shell names (always incomplete). A user's idle shell they renamed `OpenSpec` will
   be replaced; that pane holds nothing after a restart, so the cost is a shell that becomes
   a dashboard.
4. **Split-then-close in the stale pane's slot** (M3), over opening a fresh tab or split
   elsewhere (loses the slot and leaves the dead shell), and over typing `exec … ui` into
   the shell with `pane run` (the pane is no longer plugin-owned, lacks the plugin
   environment, and the command lands in shell history). Always the `dashboard` entrypoint,
   because the operation is always a split.
5. **Lives in `src/open.rs`**, not a new `src/restore.rs`: a new `HerdrCli` consumer file
   would move `LAUNCHSEAM`'s five-file list, the module map, and the tested-modules list in
   `tests/doc_contract.rs`, for a module of three functions sharing `open`'s constants,
   `herdr_reason`, and `Report`.
6. **Focus mirrors the stale pane's own `focused` flag** in `restore`, so a restart leaves
   focus where it was; `open`'s replacement always focuses, because the user asked to see
   the dashboard.
7. **`open` classifies only its first match, and an unclassifiable match is focused as
   before.** Never replacing on weak evidence is the rule this project already applies to
   agent attribution.
8. **`restore` exits 0 whenever the listing succeeded.** Per-pane failures are warnings;
   Herdr does not act on a hook's status beyond logging it, and a non-zero status for one
   stuck pane would read as "restore is broken" in the plugin log.
9. **`min_herdr_version = "0.7.5"`**, the release that introduced `[[startup]]`; a Herdr
   older than that would reject or ignore the table, and the version gate makes that explicit.
10. **New `SPEC.md` degraded-states rows**, each bound to a proving test in
    `tests/degraded-coverage.toml`: `restore`'s `pane list` failing; a candidate's
    `process-info` failing or unparseable (left alone); a replacement's open failing (shell
    left, sweep continues); a replacement's close failing (warning); `open`'s
    classification failing (focus as before).

## Risks / Trade-offs

- [The hook may run before a client attaches, and `plugin pane open` may fail without a
  terminal size] → measured in the live check before implementation proceeds; if it fails,
  the stale pane is left in place (Decision 4's failure rule) and `open`'s replace path still
  recovers it on the user's first "OpenSpec: dashboard" action. Record the outcome either way.
- [A replacement opened from the hook may receive a context without `workspace_cwd`] →
  measured in the same live check; `ui::startup_cwd` already falls back to
  `focused_pane_cwd`, and the replacement targets the stale pane, whose cwd is the repo after
  `49e3fc7`.
- [Herdr runs the hook again after a live handoff] → live dashboards classify as not stale
  (M5) and are left alone; covered by "A live dashboard is left alone".
- [A user's idle shell labelled `OpenSpec` is replaced] → accepted (Decision 3); documented
  in `README.md`.
- [Herdr changes `process-info`'s shape] → every unreadable shape classifies as not stale, so
  the failure mode is "no restore", never "a wrong pane closed".

## Migration Plan

None for data. Users on Herdr below 0.7.5 must upgrade before updating the plugin.
Rollback: revert the commits; the `[[startup]]` table disappears with them and Herdr stops
running the hook.

## Open Questions

The three items under `notes/measurements.md` → "Not yet measured" are resolved by the first
task group's live check, before the sweep is wired into the manifest. No design decision
waits on them: each has a stated fallback above.
