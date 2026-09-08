## Context

The pane is monochrome. `grep -rn "Color::" src/` returns nothing: every distinction it draws
is carried by four modifiers, and `DIM` alone already means *archived date*, *inline code*,
*block quote*, *the `file mode` badge*, and *an agent badge*. The two signals worth picking
out of a frame — "something is wrong" and "an agent is working here" — are drawn exactly like
the rows around them.

The constraint that shapes the whole design is the render seam. `src/ui/`'s pure files are
pure functions from state to a `ratatui` frame: no filesystem, no process, **no environment**,
no clock. That last one decides the shape before anything else does — a palette cannot read
`NO_COLOR`, cannot probe the terminal, and cannot branch on a capability, because `NOIO-VIEW`
forbids `std::env` in a view file and the crate's own convention forbids `std::env::var`
outside one injected binding. So the palette is a **constant table**, not a resolver.

The second constraint is the grammar/style split this repository already enforces with gates.
`src/ui/detail.rs` and `src/ui/list.rs` are plain-data grammar modules — `NOTABSEAM` fails the
build if `detail.rs` so much as names `Style` — so neither can decide what a tab chip or a
badge cell looks like. They can only say **where** it is; `ui::view` says what colour it is.

`view-fidelity` and `gate-integrity` are both archived, so the render functions this change
rewrites are settled and the shape a provably-falsifiable gate script must take is settled
too.

## Goals / Non-Goals

**Goals:**

- One table, in one module, from a semantic role to a `ratatui::style::Style`, confined by a
  gate on the same terms `pulldown_cmark` is confined to `src/ui/markdown.rs`.
- Colour exactly where a modifier cannot carry the distinction: problem rows, the `file mode`
  badge, the five agent statuses, heading level, inline code, links, and the artifact tab
  chips.
- The artifact tab bar reads as a row of tabs rather than a sentence: padded chips on a
  painted background, one unpainted spacer column between them, no leading digits.
- Every existing modifier assertion in the crate still passes, unedited, so a monochrome
  terminal loses nothing.

**Non-Goals:**

- No user-configurable theme and no new `config.toml` key.
- No terminal capability probing, no `NO_COLOR` read, no runtime branch on colour support.
- No measurement, layout, or row-grammar change outside the tab bar. `Row::text` stays
  byte-identical at every width; only a non-text field is added beside it.
- No new markdown construct and no strikethrough face — `markdown-constructs` owns that.
- No new dependency: `ratatui::style::Color` is already reachable.

## Boundaries

| Piece | Where | Pattern it follows |
|---|---|---|
| `ui::palette` (new) | `src/ui/palette.rs` | `src/ui/markdown.rs`: one module, one job, one confinement gate, no I/O |
| `ui::view::style_for` | `src/ui/view.rs` | unchanged signature; its body becomes a fold over palette roles |
| `ui::view` render functions | `src/ui/view.rs` | unchanged; each `Style::default().add_modifier(…)` becomes `palette::style(role)` |
| `ui::detail::tab_bar` | `src/ui/detail.rs` | unchanged signature and return type; the cell text and the separator width change |
| `ui::list::Row` | `src/ui/list.rs` | plain data, no `ratatui` type; gains one field the same way `Row::kind` is carried |
| `PALETTE` gate (new) | `scripts/gates/palette.sh` | `scripts/gates/mdseam.sh`, structurally: exclusion by path, positive control first, file-count guard; plus a third leg asserting the new module is in `NOIO-VIEW`'s and `COLWIDTH`'s hard-coded `PURE` lists |
| Gate control (new) | `tests/gate-controls.toml` | one `[[control]]` row per script, executed by `tests/gate_controls.rs` |

Modules **not** touched: `cli`, `changes`, `schema`, `tasks`, `agents`, `launch`, `watch`,
`refresh`, `resolve`, `config`, `state`, `open`, `ui::app`, `ui::layout`, `ui::markdown`,
`ui::tasks`, `ui::driver`, `ui::terminal`, `ui::event`.

**No process spawn is added anywhere.** `src/cli.rs` is untouched, and neither
`src/ui/palette.rs` nor any file this change edits names `process::Command`, `Command::new`,
or `Stdio`. `NOSPAWN-GREP` and `LAUNCHSEAM` are unaffected.

**No I/O is added to a view.** `ui::palette` is a pure total function of its argument. It is
added to `NOIO-VIEW`'s `PURE` list (eight files → nine) and to `COLWIDTH`'s (seven → eight)
rather than exempted from either.

**The `Change` type is not altered.** `changes::from_files` and `changes::from_cli` are
untouched, so there is nothing to keep in agreement. The one type that changes is
`ui::list::Row`, which neither producer constructs.

## Contracts

Three in-crate interfaces change. There is no external consumer: the crate is a binary plugin
with no published library surface, and the plugin manifest, the config format, and every
keybinding are untouched.

1. **`ui::palette::style(Role) -> Style` — new, additive.** Consumers: `ui::view` only.
   Total: every `Role` value, including `Heading(0)` and `Heading(255)`, returns a `Style`.
   No error surface — there is nothing to fail.

2. **`ui::list::Row` — additive.** Gains `badge: Option<BadgeCell>`, and `BadgeCell` is new.
   Consumers: `ui::view::render_list` and `ui::list`'s own tests. Every construction site is
   in `src/ui/list.rs`; `NODEFAULT-UI`'s half B (no field elided at a construction or
   destructuring site) applies to `Dashboard`/`Filter`/`Detail`, not to `Row`, so no gate
   invocation changes. **Breaking for any exhaustive `Row { .. }` pattern**, of which the
   crate has none outside `src/ui/list.rs`'s tests.

3. **`ui::detail::Tab` — same shape, different values.** `text` now carries the chip's two
   padding spaces and no leading digit; `x` reflects a one-column separator instead of two.
   Consumers: `ui::view::render_detail_tabs` and `ui::detail`'s own tests. The struct
   definition is unchanged, so this is a **behavioural** break rather than a compile break —
   every landed `assert_eq!(tab.text, "1 proposal")` fails, which is the intended RED.

   The `mouse-input` change proposal hit-tests exactly `[tab.x, tab.x + columns(tab.text))`.
   That span is what this change paints, and settling the padding here is why `mouse-input`
   depends on this change rather than the reverse.

## Persistence and Rollout

- **Migration:** none. No stored data, no schema, no file format.
- **Backfill:** none.
- **Seeding:** none.
- **Cache invalidation:** none. The `artifact-content` cache is keyed on `(change directory,
  tab)` and this change touches neither key nor value.
- **Index rebuild:** none.
- **Authorization:** none. The pane is read-only and this change adds no key and no action.
- **Observability:** none beyond the pane itself. No logging, no metrics, no new problem row
  source.
- **Deployment:** `make build` produces the same binary at the same path; `herdr plugin link`
  is unaffected. A running pane picks the change up on its next launch.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| Terminal (raw mode, alternate screen) | replaced — `ratatui::backend::TestBackend`, never a real terminal | replaced — same |
| Filesystem (`openspec/` tree) | replaced — no scenario in this change reaches it; `NOIO-VIEW` sweeps `src/ui/view.rs` whole-file, so a view test there cannot open a directory | replaced — `Dashboard` values built in-memory |
| Artifact reader (`&dyn Fn(&Path) -> Result<String, String>`) | replaced — closure over an in-memory string | replaced — same |
| `openspec` binary (`OpenspecCli`) | replaced — not reached; no scenario in this change runs the CLI path | replaced — same |
| Herdr socket (`HerdrCli`, agent poll) | replaced — `Dashboard::agents` set directly to an `AgentSnapshot` value | replaced — same |
| Launcher (`Launcher`, `launch::start`) | replaced — `Dashboard::launch.problems` set directly | replaced — same |
| Filesystem watcher (`notify`) | replaced — `Dashboard::refresh` set directly | replaced — same |
| Process environment | replaced — never read; the palette is a constant table and `NOIO-VIEW` proves no view file names `std::env` | replaced — same |
| Clock | replaced — never read | replaced — same |
| The repository tree itself (for the gate) | real — `tests/gate_controls.rs` copies the tree to a scratch directory and plants a defect | n/a |

## Test Strategy

Four tiers, all inside `cargo test` except the gate recipe:

- **unit** — pure functions asserted directly (`ui::palette::style`, `ui::view::style_for`,
  `ui::detail::tab_bar`, `ui::list::rows`). `cargo test --lib`.
- **view** — a `Dashboard` rendered into a `TestBackend` at **120x20 and 60x20**, asserting
  cell contents *and* `Cell::style()`. `cargo test --lib`.
- **gate** — `scripts/gates/palette.sh` run by `make gates`, and its planted-defect control
  run by `cargo test --test gate_controls`.
- **contract** — `tests/degraded_coverage.rs` and `tests/ci_workflow.rs`, unchanged in
  content but re-run because the gate list grows.

**This change takes the outer-loop acceptance test.** It is
`tests/gate_controls.rs::gate_controls_catch_their_plants` running the new `palette-outside`
control — the planted `use ratatui::style::Color;` in `src/ui/view.rs` that the new gate must
reject. That is the one claim of this change that no unit test can make, because it is a claim
about *every file that does not exist yet*.

The test **name** matters, because controls are table rows in `tests/gate-controls.toml`
iterated by a single `#[test]` rather than one test function each. No test name will ever
contain `palette`, so `cargo test --test gate_controls palette` runs **zero** tests and exits
0 — a filter that vouches for nothing. Every task below names
`cargo test --test gate_controls` or `... catch_their_plants` instead.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| The palette answers every role with a `Style` | `ui::palette` unit test over every variant | unit | none | `cargo test --lib ui::palette::` |
| The confinement gate catches a `Color` named outside the palette | `tests/gate-controls.toml` entry + green run | gate | real tree copy | `cargo test --test gate_controls` |
| The palette module reaches no I/O and measures no width | `NOIO-VIEW` and `COLWIDTH` output strings | gate | none | `make gates` |
| No RGB, indexed, or reset colour is named | second leg of `scripts/gates/palette.sh` | gate | none | `/bin/sh scripts/gates/palette.sh` |
| The palette reads nothing from the environment | `NOIO-VIEW` with `src/ui/palette.rs` in `PURE` | gate | none | `make gates` |
| Each role's modifier set is exactly the table above | table-driven unit test | unit | none | `cargo test --lib ui::palette::` |
| A monochrome reading of the frame is unchanged | render at both sizes, assert modifiers only | view | TestBackend | `cargo test --lib ui::view::` |
| The coloured set is exactly the table above | table-driven unit test over `fg`/`bg` | unit | none | `cargo test --lib ui::palette::` |
| An out-of-range heading level does not panic | unit test on `Heading(0)`, `(7)`, `(255)` | unit | none | `cargo test --lib ui::palette::` |
| Faces reach the buffer as coloured styles at both mandated widths | render at 120x20 and 60x20 | view | TestBackend | `cargo test --lib ui::view::` |
| Heading foreground wins over a code span inside it | `style_for` unit test | unit | none | `cargo test --lib ui::view::` |
| A plain face is the default style | `style_for` unit test + render | unit + view | TestBackend | `cargo test --lib ui::view::` |
| The five tdd artifacts become five numbered tabs at both mandated widths | `tab_bar` unit test at 78 and 58 | unit | none | `cargo test --lib ui::detail::` |
| A tenth artifact is labelled without a digit | `tab_bar` unit test, twelve artifacts | unit | none | `cargo test --lib ui::detail::` |
| Duplicate artifact ids remain two separately addressable tabs | `tab_bar` unit test | unit | none | `cargo test --lib ui::detail::` |
| No artifacts is a single placeholder cell, not an empty bar | `tab_bar` unit test at 78, 58, 8 | unit | none | `cargo test --lib ui::detail::` |
| A zero-width bar is empty and does not panic | `tab_bar` unit test at width 0 | unit | none | `cargo test --lib ui::detail::` |
| A twelve-artifact bar windows to keep the selected tab visible | `tab_bar` unit test, six calls | unit | none | `cargo test --lib ui::detail::` |
| The window slides back when the selection moves left again | `tab_bar` unit test, four calls | unit | none | `cargo test --lib ui::detail::` |
| A selected cell wider than the whole bar is truncated rather than dropped | `tab_bar` unit test, 200-char id | unit | none | `cargo test --lib ui::detail::` |
| A selected index past the end of the list does not panic | `tab_bar` unit test, `selected: 7` | unit | none | `cargo test --lib ui::detail::` |
| The tab bar reaches the buffer at both mandated widths | render at 120x20 and 60x20, assert text and `bg` | view | TestBackend | `cargo test --lib ui::view::` |
| The tab bar never overwrites a border or the rows around it | render, assert border cells and absent `bg` | view | TestBackend | `cargo test --lib ui::view::` |
| `split_detail` is exact at its degenerate heights | existing `ui::layout` unit test, re-run unchanged | unit | none | `cargo test --lib ui::layout::` |
| The digit keys select tabs and near misses do not | existing `ui::app::action_for` unit test, re-run unchanged | unit | none | `cargo test --lib ui::app::` |
| The bracket keys step one tab and near misses do not | existing `ui::app::action_for` unit test, re-run unchanged | unit | none | `cargo test --lib ui::app::` |
| Stepping is clamped at both ends and does not wrap | existing `Dashboard::apply` unit test, re-run unchanged | unit | none | `cargo test --lib ui::app::` |
| An out-of-range digit is inert | existing `Dashboard::apply` unit test, re-run unchanged | unit | none | `cargo test --lib ui::app::` |
| Switching tabs resets the scroll and staying put does not | existing `Dashboard::apply` unit test, re-run unchanged | unit | none | `cargo test --lib ui::app::` |
| Moving the selection resets the tab and the scroll, and a clamped move does not | existing `Dashboard::apply` unit test, re-run unchanged | unit | none | `cargo test --lib ui::app::` |
| Tab keys act at both routes | render at 120x20, assert the third chip carries the active style | view | TestBackend | `cargo test --lib ui::view::` |
| The routed region's border takes its style from the palette at both widths | render at 120x20 both routes and 60x20 | view | TestBackend | `cargo test --lib ui::view::` |
| A badged active row reports the column its badge occupies, at both mandated widths | `ui::list::rows` unit test at 38 and 58 | unit | none | `cargo test --lib ui::list::` |
| A badged archived row reports the column its badge occupies | `ui::list::rows` unit test at 38 and 58 | unit | none | `cargo test --lib ui::list::` |
| A dropped badge cell reports no badge | `ui::list::rows` unit test at six widths | unit | none | `cargo test --lib ui::list::` |
| No non-change row carries a badge | `ui::list::rows` unit test over every row kind | unit | none | `cargo test --lib ui::list::` |
| The badge cell reaches the buffer coloured and the rest of the row does not | render at 120x20 and 60x20 | view | TestBackend | `cargo test --lib ui::view::` |
| Problem rows are red and change rows are not, at both mandated widths | render at 120x20 and 60x20 | view | TestBackend | `cargo test --lib ui::view::` |
| An empty-state message row is not a problem row | render at 120x20 and 60x20, two dashboards | view | TestBackend | `cargo test --lib ui::view::` |
| The selected row is bold and uncoloured at both mandated widths | render an in-memory `Dashboard` at 120x20 and 60x20 | view | TestBackend | `cargo test --lib ui::view::` |
| A badged selected row keeps its bold under the badge colour | render at 120x20 and 60x20 | view | TestBackend | `cargo test --lib ui::view::` |
| The detail header is bold and uncoloured at both mandated widths | render at 120x20 and 60x20 | view | TestBackend | `cargo test --lib ui::view::` |
| The document fills the detail interior at both mandated widths | existing `ui::view` render test, re-run unchanged | view | TestBackend | `cargo test --lib ui::view::` |
| Faces reach the buffer as styles at both widths | render, assert every landed modifier plus the three new colours | view | TestBackend | `cargo test --lib ui::view::` |
| An empty source leaves the detail interior blank at both widths | existing render test, re-run unchanged | view | TestBackend | `cargo test --lib ui::view::` |
| Content never overwrites the detail region's border | existing render test, re-run unchanged | view | TestBackend | `cargo test --lib ui::view::` |
| A degenerate detail interior draws nothing and does not panic | existing render test, re-run unchanged | view | TestBackend | `cargo test --lib ui::view::` |
| A path that fits is right-aligned whole at both widths | existing render test, re-run unchanged | view | TestBackend | `cargo test --lib ui::view::` |
| A path too long for the narrow header is shortened from the left | existing render test, re-run unchanged | view | TestBackend | `cargo test --lib ui::view::` |
| A header too narrow for any path shows only the label | existing render test, re-run unchanged | view | TestBackend | `cargo test --lib ui::view::` |
| No repository found is named in the header at both widths | existing render test, re-run unchanged | view | TestBackend | `cargo test --lib ui::view::` |
| The badge is drawn dim after the label at both widths | render, assert `DIM` **and** `fg: Yellow` | view | TestBackend | `cargo test --lib ui::view::` |
| A false flag renders the header that landed before this change | render, assert no `fg` anywhere in row 0 | view | TestBackend | `cargo test --lib ui::view::` |
| The badge takes its columns from the path, not from the label | existing render test, re-run unchanged | view | TestBackend | `cargo test --lib ui::view::` |
| A header too narrow for the badge drops it whole | existing render test, re-run unchanged | view | TestBackend | `cargo test --lib ui::view::` |
| A wide-character path is shortened by columns and stays inside the header | existing render test, re-run unchanged | view | TestBackend | `cargo test --lib ui::view::` |

Whole-suite verification is `make check`, which runs format, lint, `make gates`, `cargo test
--all-features`, and coverage at both floors.

## Decisions

**Decision 1 — the palette is a `fn(Role) -> Style`, not a `struct` of `Style` fields or a
`const` table.**
A function keeps `Role::AgentBadge(AgentStatus)` and `Role::Heading(u8)` expressible as
parameterised roles rather than as twelve separate fields, and it makes `Heading(255)`
answerable without a lookup miss. *Alternatives:* a `pub const PALETTE: Palette` struct
— a genuine option, since `Style::new`, `fg`, `bg`, and `add_modifier` **are** `const fn`
(`ratatui-core-0.1.2/src/style.rs:300`, `:335`, `:352`, `:408`; only `patch` at `:471` is not),
so no `LazyLock` would be needed — rejected because it would need one field per
`(status, level)` pair and would answer `Heading(255)` with a lookup miss rather than a value;
a `HashMap` — rejected as a runtime allocation for a fixed, tiny, total mapping.

**Decision 2 — the confinement gate is on `Color`, not on `Style` or `Modifier`.**
`Style` and `Modifier` are named all over `src/ui/view.rs` and in every view test that asserts
one, so a gate on them would be red on the unmodified tree and the only repairs would be
exemptions — which is how a confinement check rots into a rubber stamp (`NOTABSEAM`'s own
comment makes the same argument about `src/ui/list.rs`). `Color` is named **nowhere** in the
crate today, which is what makes the gate green from its first run and falsifiable from its
first plant. *Alternative:* gate `Modifier` too, scoped to production slices only — rejected;
the awk production-slice split is exactly the kind of exemption the previous sentence warns
about, and the modifiers are already frozen by Decision 3's assertion.

The consequence, which is a constraint on **tests** and not only on production code: this
crate's view tests are inline `#[cfg(test)]` modules inside `src/ui/`, so the gate searches
them too. No test outside `src/ui/palette.rs` may name a `Color` literal; a render test
asserts a cell's colour by comparing it against `palette::style(role)`, and the literal table
is asserted once, in the palette's own tests. That is not a weakness of the confinement — it
is the same discipline applied one level up: a test naming `Color::Green` beside a palette
saying `Green` is two declarations of one fact, and the comparison form has only one.

The obvious objection is that `assert_eq!(cell.style().fg, palette::style(Role::X).fg)` is
tautological. It is not, because it is half a pair. It falsifies "the view applied the role"
— a view that forgot the role gives `None` against a `Some` — while the palette's own
table-driven test falsifies "the role carries the right colour", against literals, in the one
file allowed to write them. Neither half alone is sufficient and the spec requires both. The
rejected alternative is a production-slice `awk` cut in `palette.sh` (`colwidth.sh`'s
precedent), which would let a view test name `Color::Green` directly: cheaper, but it puts a
second copy of the colour table in the test module, where it goes stale silently.

**Decision 3 — this change adds, removes, and alters no modifier anywhere.**
Colour is added strictly beside the existing modifier. The payoff is a falsifiable claim
rather than a hope: every landed `Modifier::` assertion in the crate stays unedited and green,
so "a monochrome terminal loses nothing" is proved by the suite rather than argued in review.
The one stated exception is the tab-bar row, which `artifact-tabs` moves and relabels
wholesale; its narrower guarantee — the selected chip stays that row's only `BOLD` span — is
in `specs/view-palette/spec.md`.
The exception is the tab-bar row, and it is a change to *where* a modifier lands rather than
to the mapping: `artifact-tabs` relabels and moves every chip, so the two landed tests that
assert `BOLD` at fixed tab-bar columns (`the_five_tab_bars_exact_string_with_the_selected_tab_bold`
at `src/ui/view.rs:3320` and `a_select_tab_at_route_list_is_visible_in_the_tab_row_at_120` at
`:3370`) are rewritten by group 4. Everywhere else, no modifier expectation moves.
*Alternative:* rebalance modifiers now that colour carries some of the load — for example
dropping `DIM` from inline code since it is yellow anyway — rejected, because it would make
the monochrome regression untestable and it is a separate argument with its own trade-off.

**Decision 4 — `Row` gains a `badge` field; the view does not re-derive the badge's column.**
The badge is one column inside a row the grammar has already assembled into a `String`, and
its offset depends on the name field's width, which depends on whether the progress cell and
the date field survived the drop order. Re-deriving that in `ui::view` would be a second copy
of `change-rows`' arithmetic — the exact duplication `progress_cell` being `pub(crate)` exists
to prevent. *Alternatives:* return the row as faced segments the way `ui::markdown::Line`
does — rejected as a far larger grammar change than colouring one cell justifies, and it would
force `NOTABSEAM`-style questions onto `src/ui/list.rs`; scan the rendered text for the badge
glyph — rejected, because `w`, `i`, `b`, `d`, and `?` all occur in change names.

**Decision 5 — the chip's padding is part of `Tab::text`, not a view-side inset.**
`ui::view` writes `tab.text` at `tab.x` with one `set_string`; if the padding lived in the
view, the painted span and the span `ui::detail` reports would be two different rectangles
computed in two places, and `mouse-input` would hit-test the wrong one. Putting the spaces in
the text makes the reported cell and the painted cell the same object by construction.
*Alternative:* keep `text` bare and add `pad: u16` — rejected as the same fact stored twice.

**Decision 6 — the separator between chips falls from two columns to one.**
Two chips with backgrounds and a two-column gap read as two chips with a gap; with a
one-column gap they read as a segmented control, which is what a tab bar is. One column is
also the minimum that still shows an edge — zero would merge two inactive chips into one
continuous field. Combined with the dropped digits this is what takes the `tdd` bar from 57
columns to 53, so the change buys slack rather than spending it.

**Decision 7 — the `no artifacts` placeholder is drawn as an inactive chip.**
It occupies the tab bar's position and the reader should read it as "the tab bar, which is
empty". *Alternative:* a third role, `TabEmpty`, drawn dim and unpainted — rejected as a role
that exists for one string in one degenerate state, and it would need its own padding branch
in `tab_bar`.

**Decision 8 — foreground precedence is heading over code over link.**
`Face` fields compose, so a span can carry several coloured faces at once and exactly one
foreground reaches the buffer. Folding with `Style::patch` in a fixed order makes the winner a
stated rule rather than an artifact of the `if` order; heading is folded last because a
heading line reading as one colour is the entire reason to colour headings. Modifiers still
accumulate across the fold, so nothing is lost. *Alternative:* let the innermost inline face
win — rejected: it makes a heading's colour depend on its contents, which is the opposite of
what colouring a heading is for.

**Decision 9 — named ANSI indices, never RGB.**
The reader's terminal theme decides what `Red` is, a 16-colour terminal renders correctly, and
the pane does not fight the theme of the Herdr panes beside it. *Alternative:* a fixed RGB
palette — rejected: it would look wrong on a light background and would be the first thing in
this crate that assumes anything about the terminal.

**Decision 10 — the palette is a constant, and `NO_COLOR` is not read.**
Not a preference: `NOIO-VIEW` forbids `std::env` in a pure view file, and the crate's
convention confines the one real environment read to a single injected binding. Honouring
`NO_COLOR` would mean threading an environment lookup into `Dashboard` and through every
render function, for a variable neither `crossterm` nor `ratatui` observes either. The
guarantee offered instead is Decision 3's: removing colour loses nothing.

## Risks / Trade-offs

- **A terminal theme maps `DarkGray` to something invisible against its own background, so the
  inactive chips and the separator row vanish.** → Every such role also keeps a readable
  glyph: the separator row is still `-- archived ----`, the inactive chip still spells its id
  in the default foreground, and only the *background* is `DarkGray`. Nothing becomes
  unreadable, only unstyled.
- **`TabActive` sets an explicit `Black` foreground on a `Cyan` background, which is the one
  place this change assumes a background's brightness.** → It is assumed for a *named* colour
  whose contrast against black holds in every standard 16-colour scheme, and it is confined to
  one row of one table. Changing it is a one-line edit the gate protects.
- **Rewriting `tab_bar`'s cell text turns every landed tab-bar assertion red at once,
  including the ones about windowing that this change does not mean to alter.** → That is
  intended and is the RED of group 5, but it also means a windowing regression could hide
  inside a bulk test rewrite. Mitigated by keeping `joined_width`'s call sites and the
  `start`/`end` loops **byte-identical** except for the separator constant, and by the
  scenario that asserts every 58-column window holds four chips and every 78-column window
  five — a claim about the windowing rule, not about the labels.
- **A future view file names `Color` in a doc comment and fails the gate for a non-defect.** →
  Accepted, and it is the same known limit `MDSEAM` and `NOTABSEAM` already carry and state.
  The repair is to reword the comment, exactly as `src/ui/detail.rs`'s doc comment already had
  to be worded to say "the view" rather than naming a ratatui type.
- **Coverage: `src/ui/palette.rs` is a large `match` and the production-slice floor is the
  falsifiable one.** → Every arm is reached by the table-driven tests that enumerate every
  `Role` variant, so the module lands at full line coverage rather than diluting the slice.

## Migration Plan

None needed. No stored state, no file format, no protocol, and no key binding changes; the
plugin is a single binary rebuilt by `make build`. Rollback is `git revert` of the change's
commits — nothing outside the repository has to be undone.

Deploy order within the change is the task order: the palette module must exist before
`ui::view` can consult it, and the new gate script must be written before the module so its
first run is a recorded RED (`scripts/gates/palette.sh` failing "the exclusion has nothing to
exclude") rather than an untested green.

## Open Questions

None. Two questions were resolved during planning rather than deferred:

- *Does the tab bar still fit both mandated interiors after padding?* Measured: 53 columns
  against 78 and 58, against 57 today. It fits with more slack, not less.
- *Where does the agent badge's colour come from if `ui::list` may not name a `Style`?* From
  `ui::view`, via the badge's column and status carried as plain data on `Row` — Decision 4.
