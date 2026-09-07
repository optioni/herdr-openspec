## Why

The dashboard is monochrome. `grep -rn "Color::" src/` returns nothing: every distinction
the pane draws is carried by four modifiers — `BOLD`, `DIM`, `ITALIC`, `UNDERLINED` — and
they have run out. `DIM` alone currently means *archived date*, *inline code*, *block
quote*, *the `file mode` badge*, and *an agent badge*. `BOLD` means *selected row*, *the
`OpenSpec` label*, *a heading*, *strong text*, *the active tab*, and *the detail header*.

The consequence a reader meets is that the two things they most need to pick out of a
frame are the two least distinguishable: a `!`-marked problem row is styled exactly like
the change rows around it, and an agent's status badge is styled exactly like a date. In
a pane sitting beside working agents, "something is wrong" and "an agent is working here"
are the signals worth a colour, and neither has one.

This is **unplanned work**. The roadmap ends at Phase 6 (`degraded-states`). Every view
change was specified against a `TestBackend` buffer, where asserting a modifier is
straightforward and asserting an intent is not, so the plan never asked what the pane
should look like — only that it be correct.

## What Changes

- **A new `view-palette` capability: one table, in one module, mapping each semantic role
  to a `Style`.** It becomes the crate's only place naming a `Color`, confined by a gate
  on the terms `pulldown_cmark` and the process spawn already are — the palette stays
  replaceable by editing one file.
- **`ui::view::style_for` consults the palette** instead of hard-coding modifiers. It is
  already described as "the crate's only `Face`-to-`Style` mapping"; this change keeps
  that true and gives it a source.
- **Colour is added only where it carries a distinction a modifier cannot**: problem rows,
  the `file mode` badge, agent status badges (working / idle / blocked / done / unknown are
  five states currently drawn as one), heading level, inline code, and links.
- **Named ANSI indices, never RGB.** The reader's own terminal theme decides what "red" is,
  a 16-colour terminal renders correctly, and the pane does not fight the theme of the
  panes beside it.
- **Every role keeps its modifier.** Colour is added beside the existing modifier, never
  in place of it, so a monochrome terminal, a `NO_COLOR` environment, and a copy-pasted
  screenshot lose nothing that was there before this change. This is the "never fail
  closed" rule applied to styling.
- Not **BREAKING**: no keybinding, no manifest value, no config-format key.

## Non-Goals

- **No user-configurable theme, and no new `config.toml` key.** A palette that ships wrong
  is a bug to fix; a palette that ships configurable is a format to support forever. If a
  theme key is wanted it is a later change with its own argument.
- **No terminal capability probing.** The pane does not ask the terminal what it supports;
  it declares an ANSI index and lets the terminal answer.
- **No measurement, layout, or row-grammar change.** `view-fidelity` owns display-column
  arithmetic; this change adds no measuring site and moves no cell.
- **No new markdown construct.** A strikethrough face has no colour here because it has no
  parser support yet — `markdown-constructs` owns that, and adds its own role to this
  table when it lands.
- **No new dependency.** `ratatui::style::Color` is already reachable.
- Crosses no PRD non-goal: read-only, no change authoring, no orchestration, no Windows.

## Capabilities

### New Capabilities

- `view-palette`: the semantic-role-to-`Style` table, its confinement to one module, its
  named-index constraint, and the requirement that every role degrade to a modifier alone.

### Modified Capabilities

- `detail-scroll`: the `Face`-to-`Style` mapping stated there (`heading`/`strong` → `BOLD`,
  `emphasis` → `ITALIC`, `code` → `DIM`, `link` → `UNDERLINED`, `quoted` → `DIM`) is
  restated as a palette lookup that still yields those modifiers.
- `list-selection`: the selected row's style comes from the palette.
- `artifact-tabs`: the active tab's style comes from the palette.
- `detail-header`: the header row's style comes from the palette.
- `responsive-layout`: the `OpenSpec` label and the `file mode` badge take palette roles,
  the badge's colour being the one new distinction.

## Impact

- **Code:** `src/ui/view.rs` (`style_for` and every render function's style source), one
  new palette module under `src/ui/`, one new `scripts/gates/` confinement script composed
  into `make gates` — which `tests/ci_workflow.rs` already requires to correspond
  one-to-one with the recipe.
- **Docs:** `SPEC.md` → User interface gains the palette table; `AGENTS.md` → Architecture
  rules gains the confinement rule beside `pulldown_cmark`'s.
- **Depends on `view-fidelity`** (rewrites the same render functions) and is best written
  **after `gate-integrity`**, whose repairs decide what a new gate script must look like to
  be provably able to fail.
- No manifest, no config format, no dependency, no data model, no external service.
