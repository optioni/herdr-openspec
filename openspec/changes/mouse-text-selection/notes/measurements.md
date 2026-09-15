# mouse-text-selection — measurements

The proposal stops at an investigation on purpose: its Open Question 1 asks whether
this change needs specs at all, because "document the Option-key bypass" and "ship a
toggle key" are different changes and only a measurement tells them apart. This file
is that measurement. Nothing in `specs/`, `design.md`, or `tasks.md` gets written
until the table below has rows in it.

## Already measured (no terminal needed)

`crossterm` 0.29.0, `src/event.rs` → `impl Command for EnableMouseCapture`, is what
`CrosstermOps::enable_mouse` writes today. It turns on **five** DEC private modes in
one string:

| Mode | Meaning | Does the dashboard bind anything that needs it? |
|---|---|---|
| `?1000` | normal tracking — button press and release | **yes** — click a row, second-click to open, click a tab, fold a section header |
| `?1002` | button-event tracking — motion **while a button is down** (drag) | no. No binding in `ui::driver::mouse_action` is a drag |
| `?1003` | any-event tracking — **every** pointer motion | no. `CLAUDE.md` already records that "a pointer *motion* costs no frame at all", so this mode's whole output is discarded |
| `?1015` | rxvt extended coordinates | no — superseded by `?1006`, which crossterm enables after it and which terminals prefer |
| `?1006` | SGR extended coordinates | **yes** — without it, coordinates past column 223 are unencodable, and the pane's mandated wide layout is 120 columns with no upper bound |

So two of the five (`?1003`, `?1015`) are pure cost today, and a third (`?1002`)
is unused. `DisableMouseCapture` writes the inverses in reverse order, which is why
`restore_then` can call it unconditionally on a terminal that never enabled capture.

What this does **not** tell us is the thing the change turns on: whether a terminal
restores native drag-selection once the motion modes are off, or whether `?1000`
alone is enough to suppress selection. That is emulator-specific and needs a human.

## Still to measure (needs a real terminal and a human)

1. Does plain click-drag select text natively under each mode set?
2. Does the Shift bypass work? The Option/Alt bypass? (iTerm2 and Terminal.app
   conventionally use Option on macOS; xterm-likes and Ghostty use Shift.)
3. Does the app still receive clicks and wheel events under the narrow sets — i.e.
   is `?1000 ?1006` actually sufficient for every binding `mouse-input` shipped?
4. Does Herdr change any of the above? The pane runs inside a Herdr pane, and
   Herdr's own mouse handling sits above this plugin (proposal → Non-Goals). Run
   the harness **both** inside a Herdr pane and in a bare terminal.

## How to measure

```sh
bash openspec/changes/mouse-text-selection/notes/probe.sh
```

It walks the four mode sets — `off` (control), `minimal` (`?1000 ?1006`), `drag`
(`?1000 ?1002 ?1006`), `full` (today's bundle) — showing a block of text to select
and a live log of every byte the *application* received. An empty log during a drag
means the terminal kept the gesture for itself, which is the whole question. Answers
append below. Run it once per terminal, and once inside Herdr.

## Results

<!-- probe.sh appends one section per run below this line. -->
