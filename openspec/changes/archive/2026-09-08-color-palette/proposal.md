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

The artifact tab bar is where the shortage shows most, because it is the one row the pane
draws as a set of peers and asks the reader to pick the current one out of. Today it is
`1 proposal  2 specs  3 design  4 tasks  5 planning-review` — five labels two spaces apart,
the current one `BOLD`, which is also what the detail header directly above it is. A row of
bare labels distinguished by weight alone reads as a sentence, not as tabs.

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
- **The artifact tab bar becomes a row of chips.** Every tab is padded one column per side
  and painted a palette background — one for an inactive tab, a brighter one for the active
  tab — with a single unpainted spacer column between chips, so two inactive chips have a
  visible edge rather than one continuous field. The leading `1 `–`9 ` digits are dropped
  from the labels. Measured against this repository's five-artifact `tdd` schema the bar
  goes from **57 columns to 53**: the padding adds 10, the dropped digits save 10, and the
  separator halves from 8 columns to 4. It therefore fits both mandated interior widths (78
  and 58) with more slack than it has today, not less. `1`–`9` still select a tab —
  `ui::app::action_for` is untouched — but the bar no longer advertises them; `mouse-input`
  restores a direct affordance by making the chip clickable, and the chip's painted span is
  exactly the span that change will hit-test.
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
- **No measurement, layout, or row-grammar change outside the tab bar.** `view-fidelity`
  owns display-column arithmetic and this change adds no measuring site. The one cell that
  moves is the artifact tab, whose padding and dropped digit are inseparable from giving it
  a background: an unpadded chip paints tight against its glyphs and a numbered chip is
  wider than the bar needs to be. No list row, header, or content line moves.
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

- `change-rows`: `Row` gains a `badge: Option<BadgeCell>` field naming the column its agent
  badge occupies and the status it carries — no cell moves and every row's `text` stays
  byte-identical — so the view can paint that one column; and the problem and separator rows
  take their colour from the palette. Added to this list during planning: colouring the five
  agent statuses is impossible without telling the view where the badge sits, and that column
  is `change-rows`' grammar, not `list-selection`'s.
- `detail-scroll`: the `Face`-to-`Style` mapping stated there (`heading`/`strong` → `BOLD`,
  `emphasis` → `ITALIC`, `code` → `DIM`, `link` → `UNDERLINED`, `quoted` → `DIM`) is
  restated as a palette lookup that still yields those modifiers.
- `list-selection`: the selected row's style comes from the palette.
- `artifact-tabs`: the cell grammar becomes a padded chip with a one-column spacer and no
  leading digit, the active and inactive chip styles come from the palette, and the
  mandated 78- and 58-column width assertions are restated against the new grammar. The
  `1`-`9`/`[`/`]` requirement is restated unchanged apart from its final scenario, which
  asserted a `3 ` label the chip grammar removes.
- `detail-header`: the header row's style comes from the palette.
- `responsive-layout`: the `OpenSpec` label and the `file mode` badge take palette roles,
  the badge's colour being the one new distinction; and a region's border takes a palette role
  too, added during planning because `ui::view::render_region` is rewritten and the
  requirement that owns border styling lives here.

## Impact

- **Code:** `src/ui/view.rs` (`style_for` and every render function's style source),
  `src/ui/detail.rs` (`tab_bar`'s cell construction and separator width — the numbering
  branch that treats the tenth artifact differently from the first nine goes away with the
  digits), one new palette module under `src/ui/`, one new `scripts/gates/` confinement
  script composed into `make gates` — which `tests/ci_workflow.rs` already requires to
  correspond one-to-one with the recipe.
- **Docs:** `SPEC.md` → User interface gains the palette table; `AGENTS.md` → Architecture
  rules gains the confinement rule beside `pulldown_cmark`'s.
- **Depends on `view-fidelity`** (rewrites the same render functions) and is best written
  **after `gate-integrity`**, whose repairs decide what a new gate script must look like to
  be provably able to fail. **`mouse-input` depends on this one** if both land: its
  click-to-switch hit target is the chip's painted span, so the padding must be settled
  first or that change hit-tests a rectangle this one then moves.
- No manifest, no config format, no dependency, no data model, no external service.
