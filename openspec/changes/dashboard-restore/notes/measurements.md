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

## Not yet measured

- When `[[startup]]` runs relative to client attach, and which `HERDR_PLUGIN_*` variables it
  receives (the docs say "the normal runtime plugin environment" plus `HERDR_PLUGIN_EVENT=startup`).
- Whether `herdr plugin pane open` succeeds from a startup hook before any client has
  attached (M3 ran with a client attached).
- Which `HERDR_PLUGIN_CONTEXT_JSON` a replacement pane opened by the hook receives — in
  particular whether it carries the workspace's `workspace_cwd`, which `ui::startup_cwd`
  reads.
