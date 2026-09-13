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

Ten, of which six are the footer's ripple. The overlay itself touches four:

- `dashboard-loop`: the `ToggleHelp` action (taking `Action` to twenty-four), the `help: Help`
  field (taking `Dashboard` to fifteen), the overlay's precedence over every other dispatch,
  and `Esc` gaining an outermost layer. Also repairs a landed drift: the pure-view file list
  still read **eight** and omitted `src/ui/palette.rs`, which `view-palette` corrected in its
  own spec and not in this one.
- `responsive-layout`: `layout::help_band`, and the footer's new leading hint.
- `mouse-input`: the open overlay captures the wheel and the click, by a precedence rule rather
  than by rewriting the two landed gesture tables.
- `doc-conformance`: the inventory, `SPEC.md` → Keys and `README.md` → Keys bound to the swept
  functions; and the module map and pure-set counts naming `src/ui/help.rs`.
- `view-palette`: `NOIO-VIEW`'s `PURE` list goes to ten and `COLWIDTH`'s to nine. **No new
  `Role`** — the overlay reuses five.

And six carry landed footer strings that the new leading hint moves, each byte-exact in a
scenario: `list-filtering`, `agent-launch`, `agent-poller`, `agent-attribution`,
`quality-gates`, and `responsive-layout` again. This is the change's largest single cost and it
was invisible until the specs were written; see Sequencing.

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

## Open Questions for Review — answered

All four are settled; the specs implement the answers.

1. **How is the inventory derived without a second list?** Not by parsing source, and not by a
   macro. `action_for` and `mouse_action` are pure total functions, so the set of actions the
   pane binds is derived by **executing** them over a swept input space and comparing that set
   against `INVENTORY`'s. An exhaustive `match` in the test maps each `Action` to a name, so a
   variant added later is a compile error before it is a missing help row. The exemption list is
   closed at two names — `FilterPush` and `Ignore` — asserted by name and by length.
2. **What happens when the pane is shorter than the list?** It scrolls. `help.scroll` is a line
   offset clamped every frame by the same `layout::scroll_offset` the detail region already
   uses, and the bottom rule carries a `<first>-<last>/<total>` indicator when the content does
   not fit. No arrow glyphs: `▲`/`▼` are East Asian Ambiguous and would widen the exposure
   `SPEC.md` records. The overlay is a full-width **band**, not a box, for the same reason —
   four corner glyphs would widen it further, and `pane-chrome` removed every border in the pane.
3. **Does `?` conflict at the `/` filter route?** It does not open there. `?` types into the
   query like every other printable character; `list-filtering`'s rule that only `Ctrl-C` keeps a
   command meaning is not carved out for it. `?` is matched under both `NONE` and `SHIFT`,
   because on a US layout it is `Shift`+`/` and terminals disagree about reporting the modifier.
4. **Does the footer shrink?** No — `? help` is **prepended** and everything else stays.
   Prepending is the only position that works: hints drop from the end, and the key that reveals
   every other key must be the last one standing.

## Sequencing cost, measured

`? help` costs eight columns. The reachable footer goes from 53 to **61**, which no longer fits
the mandated 60-column frame: **`g focus` is dropped at 60 columns**. That is accepted rather
than worked around, and it repairs `agent-launch`'s own rationale rather than contradicting it.
That requirement rejected four separate action hints because dropping two would leave the reader
with "no indication that the other two exist" — an argument that only held while nothing on
screen pointed at the full list. `? help` is now the first hint on the row and the overlay lists
`g` under `Agents`. The footer is the always-visible minimum; the overlay is the full list.

**Archive-order hazard.** Six capabilities here carry landed byte-exact footer strings, and
`openspec/IMPLEMENTATION-ORDER.md` records what that costs when two changes modify one
requirement: the `MODIFIED` block carries the whole requirement, so archiving two such changes
silently discards the edits of whichever archived first, and git reports nothing. Seven other
changes are in flight in this checkout (`openspec list`, 2026-09-13). Before archiving this
change, re-extract each requirement block from `openspec/specs/<capability>/spec.md` rather than
trusting the block extracted here — and compare by **phrase**, never by line, since a later
archive may have re-wrapped it.

**`settings-window` is not a neighbour, it is a collision — read this before either is
implemented.** Its Modified Capabilities list, verified at
`openspec/changes/settings-window/proposal.md:69-77`, names `dashboard-loop`,
`responsive-layout`, **and** `mouse-input` — three of this change's own — and it plans the same
three mechanisms this change builds:

| `settings-window` plans | `help-overlay` lands |
|---|---|
| "an overlay **route**, and what keys mean while it is open" | an overlay **layer**; `Route` deliberately left at `List`/`Detail` (design.md → Decision 2) |
| "how the overlay sizes at the 100-column breakpoint and below" | `layout::help_band`, deliberately breakpoint-**independent** (design.md → Decision 3) |
| "clicking a row, and clicking outside to dismiss" | click-outside dismissal, as an ADDED precedence requirement (design.md → Decision 7) |

Both proposals cite `/` filter mode as their precedent, so the two were reasoned out from the
same starting point without either knowing about the other. **Whichever lands first owns the
machinery and the other adapts** — which this change's Sequencing already claimed, now with the
specific disagreements named. The route-versus-layer choice is the load-bearing one: if
`settings-window` ships a `Route::Settings` first, Decision 2 must be re-argued rather than
rebased, because a third `Route` variant would have to remember which route to return to and
this change's whole layer argument is that it should not.

`mouse-text-selection` also overlaps `mouse-input` — it proposes reconsidering
`EnableMouseCapture` entirely, which would change what every gesture in this change's precedence
table can receive. `header-progress-bar` and `heading-sections` plausibly overlap
`responsive-layout` and `detail-header`.
