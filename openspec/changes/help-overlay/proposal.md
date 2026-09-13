## Why

Most of the pane's bindings are undiscoverable. The footer shows at most **five** hints —
`FOOTER_HINTS` is a three-element const (`q quit`, `Enter detail`, `Esc back`) with
`a/c/s launch` and `g focus` pushed conditionally — and `render_footer` drops whole hints from
the **end** when the width cannot hold the next one, so the full five appear only at 120
columns and a narrow pane shows three.

Against that: roughly seventeen keys (`q`, `Ctrl-C`, `j`, `k`, arrows, `/`, `Enter`, `Esc`,
`Space`, `1`–`9`, `[`, `]`, `r`, `a`, `c`, `s`, `g`) and five mouse gestures (scroll a region,
select a row, double-click to open, fold a section header, switch a tab). `r`, `Space`, `/`,
the tab keys and the movement keys are never shown anywhere on screen. Two queued changes add
more: `settings-window` a key of its own, `mouse-text-selection` possibly a capture toggle.

There is a second reason to do this one **first**. `settings-window` introduces the crate's
first overlay — `Route` is `List`/`Detail` today and no modal concept exists — and it does so
while also adding editing, state writes, and provenance. A help overlay is **read-only**: same
machinery, none of the risk. Building the overlay here and letting `settings-window` add a
panel to proven machinery is the cheaper order.

## What Changes

- **A help overlay**, opened by `?`, closed by `Esc` or `?`. Read-only: it renders, it does not
  act.
- **It lists every key and every mouse gesture**, grouped, with the route each applies to —
  `Space` folds a list section at the list route and a content section at the detail route, and
  a help that flattens that is worse than none.
- **Its content is derived, not hand-listed.** A second place bindings are written down is a
  second place they go stale; this repository's contract tier already binds `SPEC.md`'s mouse
  table to the real `Action` variants (`tests/doc_contract.rs:1833`). The overlay's rows are
  bound the same way, so a binding added without a help row fails `cargo test`.
- **It is the overlay other changes reuse.** `settings-window` becomes a second panel rather
  than a second mechanism.
- Not **BREAKING**: one new keybinding, no manifest or config change.

## Non-Goals

- Acting on anything. No key rebinding, no jumping to a feature from the help, no scrolling
  side effects beyond the overlay's own.
- Documenting OpenSpec itself, the schema, or the workflow. This is the pane's own controls.
- Replacing the footer. The footer stays as the always-visible minimum; this is the full list.
- Context-sensitive help that explains the selected change or artifact.
- A second overlay mechanism — if `settings-window` lands first, this uses that one instead.

## Capabilities

### New Capabilities

- `help-overlay`: when it opens, what it renders, how it groups and orders rows, which keys it
  takes, how it closes, and how it behaves when the pane is too small to hold it.
- `binding-inventory`: the bindings as **data** — key, gesture, route, and description — which
  the overlay renders and the contract tier checks against the driver.

### Modified Capabilities

- `dashboard-loop`: an overlay route and what keys mean while it is open; `/` filter mode is
  the existing precedent for a mode that changes key meaning.
- `responsive-layout`: how the overlay sizes at and below the 100-column breakpoint, and what
  it does when the pane cannot hold the full list.
- `doc-conformance`: the help rows join the claims bound to a computable second site.
- `mouse-input`: clicking outside the overlay dismisses it.

## Impact

- `src/ui/` — a new pure view file plus the overlay route and key handling. No I/O: the view
  receives the inventory as state.
- `tests/doc_contract.rs` — the binding inventory bound against the driver's real arms.
- `SPEC.md`, `README.md` — the `?` binding, and the inventory as the single source the overlay
  and the docs both read.
- View tests at 60 and 120 columns. No dependency, no spawn, no seam change.

## Sequencing

Independent of every other queued change, and a **prerequisite by preference** for
`settings-window`: whichever lands first owns the overlay machinery, and this one is the
lower-risk candidate because it writes nothing. If `settings-window` goes first instead, this
change shrinks to a panel.

## Open Questions for Review

1. **How is the inventory derived without a second list?** Deriving rows from the driver's match
   arms is ideal and probably not reachable in Rust without a macro; a hand-written inventory
   bound by a test is the realistic shape. Worth settling before specs — it is the whole
   anti-drift claim.
2. **What happens when the pane is shorter than the list?** Scrolling inside an overlay means a
   cursor inside a mode inside a route.
3. **Does `?` conflict at the `/` filter route**, where printable keys type instead of
   commanding? Almost certainly it must not open there.
4. **Does the footer shrink once this exists?** It could drop to `q quit  ? help` and give the
   width back to the unattributed-agent count.
