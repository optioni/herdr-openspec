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

## Correction: the first probe had the wrong screen

`probe.sh` ran on the **primary screen**. The dashboard runs on the **alternate
screen**, and the difference is load-bearing, so the `off` control row does not mean what
it appears to.

On the primary screen the wheel scrolls the terminal's own scrollback, so an application
with no mouse reporting receives nothing — which is exactly what the `off` row recorded.
On the alternate screen, terminals conventionally translate the wheel into **arrow keys**
instead, so that pagers and TUIs scroll without asking for mouse reporting at all.

If that holds here, then "mouse controls **and** native dragging" is not a contradiction
and needs no toggle: an application that enables **no reporting** keeps the terminal's own
drag-selection and still gets wheel scrolling, for free, as arrow keys. What it gives up is
everything that needs a button — click-to-select-row, second-click-to-open, click-to-fold,
and click-to-switch-tab, all four of which `mouse-input` shipped.

`probe-altscreen.sh` measures exactly this. Until it has run, the conclusion recorded
above — "the shipped answer is a key that releases capture" — is **not established**, and
the artifacts written on top of it are provisional.

### Measuring another TUI directly

The proposal also asked for a measurement of what Copilot CLI actually emits, which has not
been done: the binary is not installed on this machine. For any TUI, on macOS:

```sh
script -q /dev/null <the-tui> 2>&1 | LC_ALL=C grep -ao $'\033\[?1[0-9]*[hl]' | sort -u
```

That lists every DEC private mode the program turns on or off. `?1049h` is the alternate
screen; `?1000`/`?1002`/`?1003`/`?1006`/`?1015` are the mouse modes. A program that shows
`?1049h` and **no** `?100Xh` is using the arrow-key translation above.

## Results

<!-- probe.sh appends one section per run below this line. -->

## 2026-09-15 — ghostty, TERM=xterm-256color, herdr=yes, tmux=no

| mode set | plain drag selects | Shift+drag selects | Option+drag selects | app saw click | app saw wheel |
|---|---|---|---|---|---|
| `off` | yes | yes | yes | no | no |
| `minimal` | no | yes | no | yes | yes |
| `drag` | no | yes | no | yes | yes |
| `full` | no | yes | no | yes | yes |

## 2026-09-15 — ghostty, TERM=xterm-ghostty, herdr=no, tmux=no

| mode set | plain drag selects | Shift+drag selects | Option+drag selects | app saw click | app saw wheel |
|---|---|---|---|---|---|
| `off` | yes | yes | yes | no | no |
| `minimal` | no | yes | no | yes | yes |
| `drag` | no | yes | no | yes | yes |
| `full` | no | yes | no | yes | yes |

## What the measurement decided

Two runs, Ghostty inside a Herdr pane and Ghostty bare. **Both tables are
identical, row for row.** Three conclusions, in the order they matter:

1. **Shift+drag restores native selection under every mode set, today's
   included.** The complaint is a discoverability gap, not a defect. Nothing in
   `src/` has to change for a user to select and copy a requirement out of a spec
   right now — they have to know to hold Shift.

2. **Narrowing the mode set buys nothing for selection.** `minimal`
   (`?1000 ?1006`) suppresses plain drag-selection exactly as `full` does. The
   proposal's "or a narrower capture mode, if the measurement shows selection
   survives one" branch is **dead**: selection does not survive one. Dropping
   `?1002`/`?1003`/`?1015` remains defensible as removing cost the dashboard
   never asked for — no binding needs them, and `minimal` was measured to
   deliver click and wheel intact — but it must not be sold as a selection fix.

3. **Herdr is not in the way.** Identical results inside and outside a pane, so
   the plugin owns this question end to end and there is nothing to raise with
   Herdr.

Two further facts worth keeping:

- **Option+drag does not work in Ghostty** — `no` under every mode set with
  reporting on. The Option bypass is an iTerm2/Terminal.app convention; the
  proposal's guess that it was the macOS answer is wrong for this terminal.
  Anything written down should say **Shift**, and say Option is terminal-specific.
- **One terminal family measured.** Ghostty only, macOS only. Shift is the xterm
  convention and very likely generalises, but iTerm2, Terminal.app, and the Linux
  terminals are unmeasured. Whatever ships should not claim more than Shift-on-
  Ghostty was actually proven.

### The cost of putting it in the help overlay

The obvious home for "Shift+drag selects text" is `ui::help::INVENTORY`'s `Mouse`
group. That is **not** free. `tests/doc_contract.rs` binds every `Binding` row to
an `Action` reachable by executing `action_for`/`mouse_action`, and its
`EXEMPT_ACTIONS` is a closed set asserted by name *and* by length
(`["FilterPush", "Ignore"]`, `assert_eq!(EXEMPT_ACTIONS.len(), 2)`). A Shift+drag
row describes a gesture the application deliberately never receives — the terminal
keeps it — so it reaches no `Action` and would need a third exemption, which costs
a `binding-inventory` spec change by construction.

`SPEC.md` and `README.md` carry no such constraint.

## 2026-09-15 — ALTERNATE SCREEN — ghostty, TERM=xterm-ghostty, herdr=no, tmux=no

| mode set | plain drag selects | wheel arrives as ARROW KEYS | app saw click | Shift+drag selects |
|---|---|---|---|---|
| `alt-off` | yes | yes | no | yes |
| `alt-minimal` |  |  |  |  |
| `alt-full` |  |  |  |  |

## 2026-09-15 — ALTERNATE SCREEN — ghostty, TERM=xterm-ghostty, herdr=no, tmux=no

| mode set | plain drag selects | wheel arrives as ARROW KEYS | app saw click | Shift+drag selects |
|---|---|---|---|---|
| `alt-off` | yes | yes | no | yes |
| `alt-minimal` | ? | no | yes | yes |
| `alt-full` | no | no | yes | yes |

## What the alternate-screen measurement decided

(The first `alt-` section above is a partial run — only `alt-off` was answered. The second
is complete and is the one that counts. Both agree on `alt-off`.)

**The hypothesis holds.** On the alternate screen with **no** mouse reporting:

| | selection | wheel | click |
|---|---|---|---|
| reporting **off** | **plain drag works** | **works, delivered as arrow keys** | does not reach the app |
| reporting **on** | needs `Shift` | works, as SGR mouse events | reaches the app |

**Read that middle column carefully.** The probe asked "did the wheel produce *arrow keys*",
so `alt-minimal` and `alt-full` answer **no** — but the wheel was not dead in those sets. It
produced `^[[<64;…M` / `^[[<65;…M`, SGR mouse events, which is what
`ui::driver::mouse_action` already consumes. Only the **shape** of the event changes between
the two rows, never whether the wheel works:

| reporting | wheel event the app receives | consumed by | region-aware? |
|---|---|---|---|
| off | `^[[A` / `^[[B` — arrow keys | `ui::app::action_for`, already | **no** — acts on the routed region |
| on | `^[[<64;x;yM` / `^[[<65;x;yM` — SGR | `ui::driver::mouse_action`, already | **yes** — carries x/y, acts under the pointer |

Both paths are already implemented in this crate. Neither default needs new scroll code; the
choice between them is a choice about region-awareness and the four button gestures, nothing
else.

So "mouse controls **and** native dragging" was never a contradiction, and needs no toggle
key to achieve. A TUI that enables no reporting keeps the terminal's own selection *and*
still scrolls, because the terminal translates the wheel into `Up`/`Down`. This is almost
certainly what Copilot CLI does, and it is measurable on any TUI with the `script` recipe
above.

**This retires the change's whole premise.** The proposal, specs, design, and tasks were
written on "capture stays on, `m` releases it". That is now the *inferior* default: it makes
the reader press a key to get back a behaviour they could have had for free.

What the pane actually loses with reporting off is narrower than it first appears:

- **Wheel scrolling survives** — and survives through machinery that already exists.
  `action_for` already maps `Up`/`Down` to `Prev`/`Next`, so a wheel event with reporting
  off is already a binding this pane implements. What is lost is *region-awareness*: the
  arrow keys act on the routed region, while `mouse-input`'s wheel acts on the region under
  the pointer.
- **Genuinely lost:** click-to-select-a-row, second-click-to-open, click-to-fold-a-section,
  and click-to-switch-a-tab — the four gestures that need a button.

So the live question is no longer "how do we add a toggle" but "which way round is the
default", and the honest answer depends on how much the four click bindings are worth
against selection working without anyone pressing anything.


## The open cell: can we have both, with no toggle?

The requirement is now "native selection **and** click events, no toggle". Whether that is
achievable turns on exactly one cell that has never been answered: **`alt-minimal`'s plain
drag**, recorded as `?` in the run above.

The reasoning that makes it worth measuring rather than assuming:

- `alt-full` suppresses plain drag, but `alt-full` enables `?1002` (button-motion) and
  `?1003` (any-motion) — the two modes that *are* the terminal handing drags to the
  application. Its `no` says nothing about a mode set that asks for no motion.
- `?1000` asks for **press and release only**. A terminal that is not being asked to report
  motion has no reason to stop doing its own drag-selection, and `alt-minimal` already
  measured `app saw click: yes`. If plain drag also selects there, the pane can have both
  with no key, no badge, and no new state — the smallest version of this change by a wide
  margin.
- The known cost if it works: a press that begins a native drag is *also* delivered to the
  app as a click, so dragging to select text would additionally select the row under the
  press. Harmless in this pane — selecting a row renders a different artifact and writes
  nothing — but it must be specified rather than discovered.

Measured with:

```sh
PROBE_SETS="press-only press-sgr" bash openspec/changes/mouse-text-selection/notes/probe-altscreen.sh
```

`press-only` is `?1000` alone; `press-sgr` is `?1000 ?1006`. The pair separates the
coordinate extension from the tracking mode, in case the terminal treats them differently.

**If plain drag is `no` in both**, then native selection and click reporting are mutually
exclusive on this terminal, no arrangement of DEC modes changes it, and the change must pick
a trade rather than deliver both. Two fallbacks would remain, and neither is plugin code:
the `Shift` bypass, already measured to work under every mode set; and Ghostty's own
configuration, which may expose a setting permitting selection while an application is
reporting.

## 2026-09-15 — ALTERNATE SCREEN — ghostty, TERM=xterm-ghostty, herdr=no, tmux=no

| mode set | plain drag selects | wheel arrives as ARROW KEYS | app saw click | Shift+drag selects |
|---|---|---|---|---|
| `press-only` | no | no (corrected) | yes | yes |
| `press-sgr` | no | no (corrected) | yes | yes |

The wheel column was misclicked at entry and is corrected above: the events were SGR mouse
reports, not `^[[A`/`^[[B`. Consistent with `?1000`, which reports the wheel as buttons 64
and 65. The decisive column — plain drag — was not the misclicked one.

## Conclusion: native selection and click reporting are mutually exclusive here

`?1000` alone asks the terminal for **press and release only**, no motion of any kind, and
plain drag-selection is still suppressed. There is no narrower request to make. So on Ghostty
the two cannot coexist, and no arrangement of DEC private modes changes that — the terminal
decides to stop selecting the moment an application asks for any mouse reporting at all.

The full picture, every row measured:

| reporting | plain drag selects | `Shift`+drag | wheel reaches app | click reaches app |
|---|---|---|---|---|
| none | **yes** | yes | yes, as `^[[A`/`^[[B` | no |
| `?1000` | no | yes | yes, as SGR | yes |
| `?1000 ?1006` | no | yes | yes, as SGR | yes |
| `?1000 ?1002 ?1006` | no | yes | yes, as SGR | yes |
| full bundle (today) | no | yes | yes, as SGR | yes |

So the change cannot deliver "both, with no toggle". What is actually available is a choice
between two defaults, plus one escape hatch that costs nothing:

1. **Reporting off** — plain drag selects, the wheel still scrolls through `action_for`'s
   existing `Up`/`Down` arms, and the four button gestures go.
2. **Reporting on** (today) — the four gestures work, and selection needs `Shift`.
3. **`Shift`+drag** — measured working under **every** row above, including today's. Zero
   code. It is the only thing that gives selection without giving up a gesture, and the
   pane's own documentation currently gets it wrong (it claims `Option` on macOS).

A fourth path exists and is explicitly a non-goal: rendering a selection overlay and owning
copy inside the TUI. It is the only way to have both, and it is a large feature that would
also need OSC 52 to reach the system clipboard — itself a separate non-goal. Naming it here
so the option is on the record as rejected rather than unconsidered.


## Measured: what Copilot CLI actually does

The proposal's third measurement — *"Measure what Copilot CLI does, rather than trusting the
report"* — deferred because the binary was not installed. Now measured:

```sh
script -q copilot-raw.log /opt/homebrew/bin/copilot </dev/null
LC_ALL=C grep -ao $'\033\[?[0-9]*[hl]' copilot-raw.log | sort | uniq -c
```

stdin at `/dev/null` gives EOF, so the program starts, paints, and exits on its own — the
capture holds a complete lifecycle, setup and teardown both, and nothing is missing.

Copilot CLI sets, in order: `?1049h` (alternate screen), `?2004h` (bracketed paste),
`?1004h` (focus events), **`?1003h` (any-event mouse tracking)**, `?1006h` (SGR coordinates).
On teardown it resets `?1006l ?1003l ?1002l ?1004l ?2004l ?1049l` — including `?1002`, which
it never enabled, the same defensive over-reset crossterm's `DisableMouseCapture` performs.

**The report that motivated this change is false.** `?1003` is *any-event tracking*: every
pointer motion reported, the single most capture-heavy mouse mode available. Copilot CLI asks
for strictly more reporting than the `?1000`-only case measured above, which already
suppressed plain drag-selection. It therefore cannot be preserving native drag-selection by
any mechanism this plugin could copy.

Two details worth keeping, since they are the only places Copilot's set differs from this
crate's:

- It enables **neither `?1000` nor `?1015`**. `?1003` already implies button reporting, so
  `?1000` is redundant, and `?1015` is the rxvt coordinate scheme `?1006` supersedes. So
  crossterm's five-mode bundle really does contain two redundant modes — confirming the
  first measurement in this file from a second, independent direction.
- It also takes `?1004` (focus) and `?2004` (bracketed paste), neither of which this pane
  uses and neither of which bears on selection.

Whatever was observed in Copilot CLI was therefore either `Shift`+drag — which works there
exactly as it works here — or a different terminal, or a misrecollection. The behaviour is
not reproducible as described, and no arrangement of modes in this crate would reproduce it.

**This closes the investigation.** All three measurements the proposal asked for are done,
and they agree: there is no mode set that gives both, the modifier bypass is `Shift` rather
than `Option`, and the comparison program does not do what it was reported to do.

## 2026-09-15 — ALTERNATE SCREEN — ghostty, TERM=xterm-256color, herdr=yes, tmux=no

| mode set | plain drag selects | wheel arrives as ARROW KEYS | app saw click | Shift+drag selects |
|---|---|---|---|---|
| `copilot-set` | no | no | yes | yes |
| `motion-only` | no | no | yes | yes |

### Interactive capture of `copilot` — 2026-09-15

```
@20        ESC[?1049h
@44        ESC[?1003h
@52        ESC[?1006h
@25660     ESC[?1006l
@25668     ESC[?1003l
@25676     ESC[?1002l
@25714     ESC[?1049l
```

## 2026-09-15 — ALTERNATE SCREEN — ghostty, TERM=xterm-256color, herdr=yes, tmux=no

| mode set | plain drag selects | wheel arrives as ARROW KEYS | app saw click | Shift+drag selects |
|---|---|---|---|---|
| `cop-all` | no | no | yes | yes |

## 2026-09-15 — ALTERNATE SCREEN — ghostty, TERM=xterm-256color, herdr=yes, tmux=no

| mode set | plain drag selects | wheel arrives as ARROW KEYS | app saw click | Shift+drag selects |
|---|---|---|---|---|
| `copilot-set` | no | ? | ? | yes |
| `cop-all` | no | ? | ? | yes |
| `alt-full` | no | ? | ? | yes |

## 2026-09-15 — ALTERNATE SCREEN — ghostty, TERM=xterm-ghostty, herdr=no, tmux=no

| mode set | plain drag selects | wheel arrives as ARROW KEYS | app saw click | Shift+drag selects |
|---|---|---|---|---|
| `copilot-set` | no | ? | ? | yes |
| `cop-all` | no | ? | ? | yes |
| `alt-full` | no | ? | ? | yes |

## SOLVED: Copilot does not use native selection at all — it implements its own

Captured with `capture-input.py`, which logs what the **terminal sends the program**. During
a drag that the user reports selects text normally, Copilot received:

```
index 34: btn=0   col=26 row=10  M   left            <- press
index 35: btn=32  col=27 row=10  M   left+motion     }
...       18 consecutive left+motion reports         }  the drag itself
index 52: btn=32  col=43 row=14  M   left+motion     }
index 53: btn=0   col=43 row=14  m   left            <- release
```

**Shift bit set on 0 of 79 reports.** The terminal delivered the complete drag — press, 18
motion events, release — to the application. A terminal doing native selection would have
kept that gesture for itself and sent nothing.

Corroborated from the output side, in the same session's log:

- **`ESC]52;` — OSC 52 clipboard writes: 2.** Copilot puts the selection on the system
  clipboard itself.
- **`SGR 7` — reverse video: 65.** Copilot paints the selection highlight itself.
- `copilot --help` carries `--mouse` / `--no-mouse`, "Enable mouse support in alt screen
  mode" — mouse handling is a feature it owns outright.

So Copilot CLI **implements text selection inside the application**: it holds `?1003 ?1006`
for the whole session, consumes the drag, renders its own highlight in reverse video, and
copies through OSC 52. What looks like native selection is a reimplementation of it.

**Every measurement in this file was correct.** Native drag-selection and mouse reporting
really are mutually exclusive — `?1000` alone suppresses selection, and no mode set escapes
it. The founding report was wrong about the *mechanism*, not about the observation: Copilot
does have both, and gets them by not using the terminal's selection at all.

### What this costs, and which non-goals it reverses

The approach is available to this pane, and the raw material is **already arriving**:
`EnableMouseCapture` includes `?1003`, so drag-motion events reach `ui::driver::mouse_action`
today and are discarded — `mouse-input` specifies that a pointer motion costs no frame. The
work is to stop discarding them.

It requires reversing **two** explicit non-goals in this change's own proposal:

1. *"Implementing selection, a clipboard, or a copy buffer inside the TUI. Rendering a
   selection overlay and owning copy is a large feature and the wrong one."*
2. *"OSC 52 clipboard writes, or any escape sequence that reaches outside the pane."*

Both were written before any of this was measured. They are now the only thing standing
between the pane and the behaviour asked for, and reversing them is a decision for the
repository's owner, not an inference from this file.

Scope, stated honestly, against this repository's own architecture rules:

- **Drag state** — anchor and focus cell on `Dashboard`, moved by press/motion/release in
  `ui::driver`. Pure, testable, and it is where the discarded events already land.
- **Selection rendering** — a reverse-video overlay over a cell range. `ui::view` is a pure
  function of `Dashboard`, so this is a render concern like any other, but it is the first
  thing the pane draws that is not derived from the change set.
- **Text extraction** — reading the selected span back out of the rendered `Buffer`.
- **OSC 52** — a write to stdout. Views do no I/O and `src/ui/terminal.rs` is the only file
  permitted to name a terminal escape, so it needs a seam and a gate exemption argument.
- **The conflict with existing bindings** — a click currently selects a row, opens a change,
  folds a section, and switches a tab. A press that begins a text selection is the same
  press. That interaction needs deciding, and it is the hardest part of the design, not the
  clipboard.
