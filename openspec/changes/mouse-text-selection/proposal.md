## Why

You cannot select text in the dashboard pane with the mouse. That is a direct consequence of
`mouse-input`: `TerminalGuard::enter` issues `EnableMouseCapture`, and a terminal with mouse
reporting on routes drags to the application instead of doing native selection. The dashboard
gets click-to-select, scroll, fold, and tab-switch; the user loses copying a requirement out of
a spec.

This proposal was written as an investigation, and the investigation is done. `notes/probe.sh`
was run twice — Ghostty inside a Herdr pane and Ghostty bare — and the two tables are identical
row for row (`notes/measurements.md`). What they establish:

- **Plain drag-selection is suppressed under every mode set**, today's included, and works only
  with reporting fully off. This is the terminal's decision; no mode set and no code in this
  crate changes it.
- **Shift+drag restores selection under every mode set.** Useful, and worth writing down, but
  it is a modifier the user has to know about — not the thing asked for.
- **Narrowing the mode set buys nothing.** `?1000 ?1006` suppresses plain drag exactly as the
  full bundle does, while delivering click and wheel intact.
- **Herdr is not in the way.** Identical inside and outside a pane.

So the shipped answer is the one the investigation predicted: a key that releases capture on
demand. With capture off, plain drag behaves exactly as the measurement's `off` control row —
select and copy natively — and the key puts capture back.

Unplanned work past Phase 6.

## What Changes

- **A key, `m`, toggles mouse capture for the running pane.** Capture stays **on** at startup:
  nothing about today's behaviour changes until the key is pressed. Pressing it releases
  capture, so the terminal does its own selection; pressing it again re-enters capture.
- **The pane says so while capture is off.** A dim badge in the list region's heading row, on
  the pattern `degraded-states` established for `file mode` — silence would read as the mouse
  having broken.
- **While capture is off, every key still works and no mouse gesture arrives.** The pane is
  fully usable by keyboard, which is the property that made a refused `enable_mouse` a non-fatal
  stored problem in the first place.
- **The toggle goes through the existing `TerminalOps` seam**, injected into `run_loop` as a
  `&dyn Fn(bool) -> Result<(), String>` on exactly the terms `ui::read_artifact` is injected
  today. No view file gains an I/O call, and `src/ui/terminal.rs` remains the only file naming
  a crossterm terminal-mode function.
- **A re-enable that the terminal refuses leaves capture off and names the reason**, rather
  than claiming a state the terminal did not grant.
- **`SPEC.md` and `README.md` record the trade-off and the Shift bypass**, scoped to what was
  actually proven: Shift, Ghostty, macOS. Option+drag was measured **not** to work in Ghostty,
  so nothing claims it does.
- Not **BREAKING**: the key is additive and the default capture state does not move.

## Non-Goals

- Implementing selection, a clipboard, or a copy buffer inside the TUI. Rendering a selection
  overlay and owning copy is a large feature and the wrong one — the terminal already does this
  well when we let it, which is the whole mechanism this change uses.
- **Narrowing the capture mode set.** Measured to buy nothing for selection. Dropping
  `?1002`/`?1003`/`?1015` remains defensible purely as removing cost no binding asks for, but it
  is a separate change with a separate justification and must never be sold as a selection fix.
- Removing mouse support, or changing the default capture state. The bindings `mouse-input`
  shipped stay, and stay on at startup.
- OSC 52 clipboard writes, or any escape sequence that reaches outside the pane.
- Herdr's own mouse handling, which the measurement showed is not involved.
- Claiming the Shift bypass on terminals it was not measured on. iTerm2, Terminal.app, and the
  Linux terminals are unmeasured; the documentation says so rather than generalising.
- Windows terminals, out of scope repository-wide.

## Capabilities

### New Capabilities

None. A toggle belongs to the capabilities that already own capture, the loop, and the
inventory.

### Modified Capabilities

- `terminal-lifecycle`: capture becomes something that can be left and re-entered mid-session,
  not only at start-up and teardown. The mirrored enter/leave ordering and the panic-hook
  guarantee must survive that. `restore_then` already calls `disable_mouse` unconditionally and
  deliberately, so the hook still consults no state — the property that makes this safe is
  already in place and must not be traded away.
- `mouse-input`: what the pane does while capture is off — no gesture arrives, and every key
  binding is unaffected.
- `dashboard-loop`: the `m` key and the capture state it owns.
- `binding-inventory`: `m` joins the `Pane` group. The spec pins six groups with binding counts
  5, 7, 4, 4, 5, 6 summing to 31; this moves the fourth count to 5 and the sum to 32, and the
  overlay's row count with it. Every one of those figures is asserted in `cargo test`.
- `responsive-layout`: the footer gains a `mouse off (m)` badge while capture is released,
  placed **first** for the reason `? help` is — hints drop from the end, and a badge explaining
  a silent mouse must not be the first thing lost. The footer rather than the list region's
  heading row, where `file mode` sits, because below the 100-column breakpoint at
  `Route::Detail` that region is not drawn at all and the detail route is exactly where a
  reader releases capture to copy out of a spec.

`degraded-coverage` is **not** modified. A refused capture change is a new row of `SPEC.md`'s
degraded-states table and needs a named proving test in `tests/degraded-coverage.toml`, but
that obligation is already this repository's standing rule for any new row, and
`terminal-lifecycle` states it inline exactly as the start-up refusal row does today. A delta
restating the general requirement would add no constraint.

## Impact

- `src/ui/terminal.rs` — a guard method for setting capture mid-session; still the only file
  permitted to name a crossterm terminal-mode function, and `NORAW`'s per-name control must
  still pass.
- `src/ui/mod.rs` — the injected capture seam and its one production binding, beside
  `read_artifact`.
- `src/ui/app.rs`, `src/ui/help.rs`, `src/ui/list.rs` — the action, the inventory row, the badge.
- `SPEC.md` — Keys, the degraded-states table, and the accepted trade-off.
- `README.md` — Keys, bound to `INVENTORY` by `tests/doc_contract.rs`.
- `tests/degraded-coverage.toml` — the new row's proving test.
- `openspec/specs/binding-inventory/spec.md` — the counts above.
- No dependency change. No I/O added to a pure view file.

## Open Questions for Review

1. **Where the badge goes when `file mode` is already there.** Both want the list region's
   heading row, and that row already drops the `file mode` badge whole rather than truncating it
   when the row is too narrow. Two badges need an order and a combined drop rule, or a second
   home.
2. **Whether `m` is the right key.** It is free, and mnemonic, but it is also a plain letter
   next to `a`/`c`/`s`/`g`, which all start agents. A mis-keyed `m` is harmless; a mis-keyed
   neighbour is not.
