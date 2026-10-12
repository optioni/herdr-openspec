# dashboard-restore — live measurements

Measured 2026-10-12 against Herdr 0.9.1 on macOS, in a throwaway named session
(`herdr --session restore-test`, driven from tmux, `HERDR_SOCKET_PATH` pointed at
`~/.config/herdr/sessions/restore-test/herdr.sock`). The default session was not touched.
The plugin under test was the GitHub-installed copy (pre-`49e3fc7`, so its panes still
reported the plugin root as their cwd).

## M1 — Herdr follows a pane process's live cwd

A pane split with `--cwd /tmp` whose shell `exec`s a process that `chdir`s two seconds later:
`herdr pane get` reported the new directory as both `cwd` and `foreground_cwd` within ~2 s.
Re-run with the dashboard binary built from `49e3fc7`, started in a scratch directory with
`HERDR_PLUGIN_CONTEXT_JSON={"workspace_cwd":"<repo>"}`: the pane reported `<repo>`.

## M2 — what a server restart does to a dashboard pane

Before the restart, `session.json` stored each dashboard pane as
`{"cwd": <plugin root>, "label": "OpenSpec", "launch_argv": [<plugin root>/target/release/herdr-openspec, "ui"]}`.

After `herdr session stop restore-test` and a fresh `herdr --session restore-test`:

- **Pane ids survive.** `w1:p2` (tab placement) and `w1:p3` (split placement) came back
  under the same ids, in the same tabs, at the same rects.
- **The label survives.** Both still report `"label": "OpenSpec"`.
- **The process does not.** `herdr pane process-info --pane w1:p2` reports one foreground
  process, `argv ["-zsh"]`, whose pid equals `shell_pid` — a fresh login shell in the saved cwd.
  `launch_argv` is stored but never re-run.
- **Plugin ownership does not survive.** `herdr plugin pane close w1:p2` →
  `{"error":{"code":"plugin_pane_not_found"}}`.

## M3 — split-then-close puts a new dashboard in the restored pane's exact slot

For each restored pane `X`:
`herdr plugin pane open --plugin herdr-openspec --entrypoint dashboard --placement split --target-pane X --direction right --no-focus`,
then `herdr pane close X`.

- Split case (`w1:p3`, right half of tab 1): the new pane `w1:p4` took rect
  `{x:87, y:0, w:87, h:49}` — identical to `w1:p3`'s — and the tab's one split kept ratio `0.5`.
- Tab case (`w1:p2`, sole pane of tab 2): the new pane `w1:p5` became tab 2's sole pane;
  tab order and count unchanged.
- The new pane ids differ (`p4`, `p5`) and are plugin-owned again.

## M4 — `open` focuses a restored shell

`open::dashboard_panes` (`src/open.rs:127`) matches a pane by label `OpenSpec` and workspace
id only. After M2, both restored shells match, so the `open` action focuses a dead shell
instead of opening a dashboard.

## M5 — a live dashboard pane reports itself as `shell_pid`

`herdr pane process-info --pane w8:p1B` on a **live** dashboard pane in the default session
(Herdr 0.9.1): one foreground process, `name "herdr-openspec"`, `argv [<plugin root>/target/release/herdr-openspec, "ui"]`,
`pid 38099`, and `"shell_pid": 38099`. A plugin pane's command is spawned directly, with no
shell above it, so "the only foreground process is the pane's own `shell_pid`" holds for a
live dashboard **and** for a restored shell (M2). The classifier must therefore also exclude
a foreground process named `herdr-openspec`.

## M6 — verbatim `process-info` for every `OpenSpec` pane in a real restored session

`herdr pane process-info --pane <id>` (Herdr 0.9.1, the default session, after a real server
restart on 2026-10-10), for every pane `herdr pane list` labels `OpenSpec`. Ten panes: four live
dashboards, six restored shells. `result.process_info` for one of each, verbatim:

Live dashboard (`w8:p1B`):

```json
{"foreground_processes":[{"argv":["/Users/juusopiikkila/.config/herdr/plugins/github/herdr-openspec-80dd366f7f1a/target/release/herdr-openspec","ui"],"argv0":"herdr-openspec","cmdline":"/Users/juusopiikkila/.config/herdr/plugins/github/herdr-openspec-80dd366f7f1a/target/release/herdr-openspec ui","cwd":"/Users/juusopiikkila/.config/herdr/plugins/github/herdr-openspec-80dd366f7f1a","name":"herdr-openspec","pid":38099}],"pane_id":"w8:p1B","shell_pid":38099}
```

Restored shell (`wW:pB`):

```json
{"foreground_processes":[{"argv":["-zsh"],"argv0":"zsh","cmdline":"-zsh","cwd":"/Users/juusopiikkila/.config/herdr/plugins/github/herdr-openspec-80dd366f7f1a","name":"zsh","pid":9304}],"pane_id":"wW:pB","shell_pid":9304}
```

All six restored shells carry `"name":"zsh"` and a `pid` equal to `shell_pid`; all four live
dashboards carry `"name":"herdr-openspec"` and a `pid` equal to `shell_pid`. So the rule in
specs/dashboard-restore holds on real data: 6 of 6 stale, 0 of 4 live misclassified.
`name` is not always the binary's name for other programs (a `claude` process was seen
reporting `name "2.1.296"`), which is why the rule excludes one name rather than requiring a
list of shell names.

## Isolated live check (tasks group 9)

Measured 2026-10-12 against Herdr 0.9.1 on macOS, in an **isolated instance**, not a named
session of the default one (design.md Decision 13). Every `herdr` invocation ran under
`env -i PATH=/opt/homebrew/bin:/usr/bin:/bin TERM=xterm-256color HOME=<scratch>/home XDG_CONFIG_HOME=<scratch>/cfg`,
so no `HERDR_*` variable reached it (`env -i ... env | grep -c HERDR` printed `0`; the
orchestrating shell itself carried seven). `herdr session list --json` in that environment
reported `session_dir` and `socket_path` under the scratch directory, `running: false` before the
first start and `true` after, never `~/.config/herdr`. A first attempt failed silently because the
scratch path made the socket path 137 bytes, over macOS's 104-byte `sun_path` limit (the server
logged "persistent user service context" and exited); a short symlink to the scratch directory
fixed it, and every later path in the log is the short one. The server ran in a private tmux
socket (`tmux -L isoherdr`) with `TERM=xterm-256color`. The working tree was linked with
`herdr plugin link .` (`source.kind: "local"`, `enabled: true`, `startup:
[{"command":["./target/release/herdr-openspec","restore"]}]`); `plugin action list` returned the
`open` and `open-tab` actions. The real registry `~/.config/herdr/plugins.json` kept its
2026-10-05 modification time throughout.

### M7 — the startup hook ran, once, and replaced both stale dashboards in their slots

One `dashboard` (split, `w1:p2` right of `w1:p1`, rect `{x:87,y:0,w:87,h:49}`, one split, ratio
`0.5`) and one `dashboard-tab` (`w1:p3`, sole pane of tab `w1:t2`) were opened with
`plugin pane open`; both live panes reported `name "herdr-openspec"`, `pid == shell_pid`, and
the repo as `cwd`. After `herdr server stop` and a fresh `herdr`:

- `herdr plugin log list`: one entry, `event "startup"`, command
  `["./target/release/herdr-openspec","restore"]`, `status "succeeded"`, `exit_code 0`,
  `stdout ""`, `stderr ""` (the hook prints nothing on success), 148 ms wall time.
- The server log shows exactly two `plugin.pane.open` and two `pane.close` requests, each open
  immediately before its close, the two replacements 68 ms apart, for old panes `2` and `3`.
- Result: `w1:p4` in tab `w1:t1` with rect `{x:87,y:0,w:87,h:49}`, the layout's one split
  unchanged at ratio `0.5`; `w1:p5` the sole pane of `w1:t2`. Pane ids changed (`p2`->`p4`,
  `p3`->`p5`), slots did not. `pane get` and `pane list`: label `OpenSpec`, `cwd` and
  `foreground_cwd` both the repo. `process-info`: one foreground process `herdr-openspec ui`,
  `pid == shell_pid`, process `cwd` the repo. A tmux capture of the attached client showed the
  list view rendered in the right-hand slot.

### M8 — the hook's environment, measured with a probe plugin

The restore hook's own environment is not logged, so a second scratch plugin (`id =
"envprobe"`, a `[[startup]]` of `sh -c 'env | sort; pwd; ps -o pid,tty,command -p $$'`) was linked
into the same isolated registry and the server restarted. Its `plugin log` stdout, which is
the environment Herdr builds for a startup command (inferred: the restore hook is a
sibling `[[startup]]` entry and its log is empty), carried:

```text
HERDR_BIN_PATH=/opt/homebrew/bin/herdr
HERDR_ENV=1
HERDR_PANE_ID=w1:p1
HERDR_PLUGIN_CONFIG_DIR=<cfg>/herdr/plugins/config/envprobe
HERDR_PLUGIN_CONTEXT_JSON={"workspace_id":"w1","workspace_label":"herdr-openspec","workspace_cwd":"<repo>","tab_id":"w1:t1","tab_label":"1","focused_pane_id":"w1:p1","focused_pane_cwd":"<repo>","focused_pane_status":"unknown","invocation_source":"startup","correlation_id":"plugin.startup"}
HERDR_PLUGIN_EVENT=startup
HERDR_PLUGIN_ID=envprobe
HERDR_PLUGIN_ROOT=<probe dir>
HERDR_PLUGIN_STATE_DIR=<home>/.local/state/herdr/plugins/envprobe
HERDR_SOCKET_PATH=<cfg>/herdr/herdr.sock
HERDR_TAB_ID=w1:t1
HERDR_WORKSPACE_ID=w1
```

`HERDR_SOCKET_PATH` names the **named instance's** socket (the isolated one), not a default
session, so the sweep cannot reach another session; `HERDR_PLUGIN_ID` is present, so `restore`'s
fallback to `herdr-openspec` is not exercised here; the context JSON carries `workspace_cwd`
and `focused_pane_cwd`. `PWD` was the plugin root and the process had a controlling tty
(`ttys020`). The event is `startup` and `invocation_source` is `startup`, but the workspace
variables name the restored workspace's active tab and focused pane (`w1:p1`, a plain shell), not
a dashboard.

### M9 — the hook runs before any client attaches, and the opens still succeed

Two orderings were measured.

1. **Client attached by the same command** (`herdr` in tmux after `server stop`): the hook's
   `started_unix_ms` was 12:13:15.145; the client connected at 12:13:15.164 (19 ms later); the
   hook's opens were requested at 12:13:15.184 and 12:13:15.252. So the hook *started*
   before the client attached and its opens *landed* after. The new panes spawned at the
   attached client's size (`rows=49 cols=87`).
2. **No client at all** (`herdr server`, the headless server, run alone in tmux): the hook
   still ran (`status succeeded`, `exit_code 0`) and both opens and closes succeeded. With no
   client the layout area is Herdr's default `120x40`: `w1:p6` was `{x:60,y:0,w:60,h:40}`, the
   tab pane `w1:p7` was sole in `w1:t2`, and `viewport_rows` was 40. When a client then attached
   (174x49) the same panes re-laid out to `{x:87,y:0,w:87,h:49}` and `viewport_rows` 49 / 47, with
   ids and the split unchanged.

So `plugin pane open` does **not** need a terminal size or an attached client, and the slot
is preserved either way. The risk "the open may fail without a terminal" did not materialise.

### M10 — `restore` run by hand makes no open or close call

Inside the live instance (`herdr pane run w1:p1 '... herdr-openspec restore ...'`, the pane's own
environment carrying six `HERDR_*` variables), with both dashboards live: exit status 0, no output,
and the server log's `api.request` lines stayed at 10 (it logs only UI-changing requests): no
`plugin.pane.open` and no `pane.close`. `pane list` was identical before and after. The "run
again after a live handoff" claim holds: live dashboards classify `Live` and are left alone.

### M11 — the `open` action replaces and raises a stale pane

Scratch state: both plugins `herdr plugin disable`d, server stopped and restarted, so no hook
ran (`plugin log list` empty) and `w1:p6` (split) and `w1:p7` (tab) were restored shells (`bash`;
the scratch `env -i` carries no `SHELL`). `herdr plugin enable herdr-openspec` did **not** run the
startup hook (log still empty). Then `herdr plugin action invoke open --plugin herdr-openspec` with
focus on `w1:p1`:

- The action's context carried `workspace_cwd` and `focused_pane_cwd` (the repo),
  `invocation_source "cli"`; the log entry was `succeeded`, exit code 0, empty output.
- `w1:p6` was replaced by `w1:p8` in the same slot (`{x:87,y:0,w:87,h:49}`, split ratio `0.5`),
  and `w1:p8` became the **focused** pane (`focused: true`, `focused_pane_id "w1:p8"`), so the
  action raised it. Its process was `herdr-openspec ui`, `cwd` the repo.
- `w1:p7`, the tab-placed stale shell, was left alone. That is the specified behaviour: `open`
  replaces the **first** match only (specs/pane-open: "the first match `w8:pG` classifies as
  stale"); the hook is what sweeps every stale pane.

Cleanup: server stopped with `herdr server stop` inside the isolated environment, tmux server killed,
no process left matching the scratch path, scratch directory and the short symlink deleted.
