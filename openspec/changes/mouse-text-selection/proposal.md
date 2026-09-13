## Why

You cannot select text in the dashboard pane with the mouse. That is a direct consequence of
`mouse-input`: `TerminalGuard::enter` issues `EnableMouseCapture`, and a terminal with mouse
reporting on routes drags to the application instead of doing native selection. The dashboard
gets click-to-select, scroll, fold, and tab-switch; the user loses copying a requirement out of
a spec.

This is a genuine trade, not a defect — but it was never made deliberately, and the losing side
is the one the user notices. GitHub Copilot CLI reportedly supports the mouse *and* leaves
selection working, which is the observation worth chasing before deciding anything.

Unplanned work past Phase 6. **Scoped as an investigation first**: the proposal's first output
is a measurement, and the implementation follows from it.

## What Changes

The investigation, which is the part that must happen first:

- Measure what a terminal actually does with each mouse mode. Crossterm's `EnableMouseCapture`
  turns on several DEC modes at once (button, drag, any-motion, SGR). Determine which of them
  the dashboard's bindings actually need — a pointer *motion* already costs no frame
  (`mouse-input`), so any-motion reporting may be pure cost.
- Measure the modifier bypass on the terminals this project supports: holding Option (iTerm,
  Ghostty) or Shift (xterm-likes) generally restores native selection while reporting is on. If
  that works, a large part of the complaint is a documentation gap.
- Measure what Copilot CLI does, rather than trusting the report. If it enables a narrower mode
  set, that is the answer and it is cheap.

Then, whichever the measurement supports:

- **A key that releases capture on demand** is the likely shipped outcome — `TerminalOps`
  already has `disable_mouse`, and `TerminalGuard` already treats a refused `enable_mouse` as a
  non-fatal stored problem, so the seam for toggling exists. Capture off, select normally,
  capture back on.
- **Or a narrower capture mode**, if the measurement shows selection survives one.
- **Or documentation only**, if the modifier bypass covers it — in which case this change
  closes having spent a day and written down why, which is a real outcome.
- Not **BREAKING** in any branch: adding a key is additive, and the default capture state does
  not change.

## Non-Goals

- Implementing selection, a clipboard, or a copy buffer inside the TUI. Rendering a selection
  overlay and owning copy is a large feature and the wrong one — the terminal already does this
  well when we let it.
- Removing mouse support. The bindings `mouse-input` shipped stay.
- OSC 52 clipboard writes, or any escape sequence that reaches outside the pane.
- Herdr's own mouse handling, which sits above this plugin.
- Windows terminals, out of scope repository-wide.

## Capabilities

### New Capabilities

None expected. If a toggle ships it belongs to `terminal-lifecycle` and `mouse-input`, not to a
new capability.

### Modified Capabilities

- `terminal-lifecycle`: capture becomes something that can be left and re-entered mid-session,
  not only at start-up and teardown. The mirrored enter/leave ordering and the panic-hook
  guarantee must survive that — a toggle must not let a panic leave capture on.
- `mouse-input`: what the pane does while capture is off.
- `dashboard-loop`: the new key, if one ships.

## Impact

- `src/ui/terminal.rs` — the only file permitted to name a crossterm terminal-mode function;
  any mode change lands here and the `NORAW` gate's per-name control must still pass.
- `src/ui/` — the key binding and the state of whether capture is currently on.
- `SPEC.md` — the accepted trade-off, written down either way.
- No dependency change. No I/O added to a view: the toggle goes through the injected
  `TerminalOps`, never a direct crossterm call from a view file.

## Open Questions for Review

1. **Does this need a change proposal at all, or a spike first?** If the measurement says
   "document the Option-key bypass", the result is two paragraphs in `SPEC.md` and no code. That
   is worth knowing before writing specs, which is why this proposal deliberately stops at the
   investigation.
2. **What the pane shows while capture is off.** Silent is confusing — the mouse simply stops
   working. A header badge is the obvious answer and `degraded-states` already established one
   (`file mode`), so the pattern exists.
3. **Panic safety of a toggle.** `restore_then` currently calls `disable_mouse`
   unconditionally, deliberately, so the hook needs no state to consult. A toggle must not
   introduce a state the hook has to read.
