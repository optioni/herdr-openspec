## Context

`ui::markdown::lines(source, width)` is a pure, total transformation from markdown source to
`Vec<Line>` of faced `Segment`s, 2,646 lines with 34 inline tests, on the pure side of the
render seam. Two of its behaviours are being reversed and four of its glyphs replaced.

The measurement that started this: `openspec/changes/pane-chrome/proposal.md` is hard-wrapped
at 89–92 display columns — lines 37, 65 and 67 measure 92 — and renders at the detail
region's 78, producing alternating full-width and orphan lines for its whole length. `glow` 3.0.0 produces the same raggedness on the same file, so the cause is per-source-line
wrapping, not this implementation. Measured at a matched content width (`-w 82`; glow spends
two columns of margin per side), with its margin and trailing pad stripped, the two renderings
are identical for their first 32 lines and 48 of 130 overall — the first divergence being the
bullet glyph, `•` against `-`, which this change adopts. Comparing at `-w 78` instead shows
glow breaking one word earlier, which is the margin, not a different wrapping rule.

Internally the cause is one line. `Block` holds `groups: Vec<Vec<Run>>` — one group per source
line, accumulated by `Formatter::group_break` (`src/ui/markdown.rs:291`) — and the emitter
wraps **each group independently** (`src/ui/markdown.rs:760`). `Event::SoftBreak` and
`Event::HardBreak` share one arm (`src/ui/markdown.rs:695`), so the two are indistinguishable
downstream.

The corpus measurements the reversals rest on, with the commands that produced them, recorded
here rather than in the specs because a spec outlives the snapshot:

- Task-list items outside `tasks.md` in the corpus this pane renders:
  `grep -rlE '^\s*[-*] \[[ xX]\]' openspec/ --include='*.md' | grep -v '/tasks\.md$' | wc -l`
  → **0**. Unchanged from what `markdown-viewer` measured.
- Display-width class of every glyph this change introduces:
  `python3 -c "import unicodedata as u; print([(c, u.east_asian_width(c)) for c in '•│─├┼┤✓'])"`
  → `A` for all but `✓`, which is `N`. The characters they replace — `-`, `>`, `|` — are `Na`.

## Goals / Non-Goals

**Goals:**

- A paragraph reflows at the region width; an author's **explicit** break still breaks.
- The pane's own chrome glyphs are the ones a reader of rendered markdown expects.
- The markdown path and `ui::tasks` render a checkbox with the **same** glyph, asserted.
- Every glyph's one-column width is asserted, not assumed.

**Non-Goals:**

- Replacing `ui::markdown` with `glow` or `tui-markdown` (Decision 5).
- Syntax highlighting.
- Any new dependency or any change to `Cargo.toml`. One gate line changes — `MD_MIN`'s
  default — because the test count it floors does; no gate's *logic* changes and no
  exemption is added to any of them.
- Any colour or face change. `ui::palette` is not touched and no `Face` field is added.
- Compensating for the CJK-locale two-column paint (Decision 4).

## Boundaries

| Module | What changes | Pattern followed |
|---|---|---|
| `src/ui/markdown.rs` | `Event::SoftBreak` arm; four glyph sites; `Options`; a new `Event::TaskListMarker` arm | unchanged — pure, total, `pulldown_cmark` confined here |
| `src/ui/tasks.rs` | the item glyph string only | unchanged — builds `markdown::Line` values directly |
| `src/ui/view.rs` | **test re-baseline only** — 10 tests, including `tasks_tab_shows_checkboxes` (`:4818`, `:4836`), `tasks_tab_chosen_by_flag` (`:4873`, `:4881`), `no_marked_artifact_renders_markdown` | unchanged — `TestBackend` render tests |
| `src/ui/mod.rs` | **test re-baseline only** — 5 tests, reached as `ui::tests::*`, including the rendered-glyph assertion at `:1289` | unchanged |
| `src/ui/app.rs` | **test re-baseline only** — 4 tests (`scroll::*`, `keys::*`), all on the shared `- line-NN` fixture | unchanged |
| `src/ui/driver.rs` | **test re-baseline only** — 3 tests, whose fixtures spell rendered bullets literally (`:904`, `:1223`, `:1341`) | unchanged |
| `SPEC.md` | degraded-states row; the § Detail view markdown-grammar paragraph; the Ambiguous-width paragraph | prose, bound by `tests/doc_contract.rs` and `tests/degraded_coverage.rs` |
| `scripts/gates/mdwidths.sh` | `MD_MIN` default 34 → 35 | unchanged — a gate's floor is its own script default |
| `tests/degraded-coverage.toml` | one row's `condition` and `why` | unchanged binding mechanism |

**No process spawn is added anywhere.** Nothing in this change names `process::Command`,
`Command::new`, or `Stdio`; `src/cli.rs` remains the crate's only spawner and no new
`OpenspecCli`/`HerdrCli` consumer appears. `NOSPAWN-GREP` and `LAUNCHSEAM` are untouched.

**No I/O is added to a view.** `ui::markdown` and `ui::tasks` stay in `noio-view.sh`'s `PURE`
set and gain no filesystem, process, environment, network, or standard-I/O call. `NOIO-VIEW`,
`MDSEAM`, `PALETTE`, and `COLWIDTH` all continue to pass without an exclusion being added.

**The blast radius is measured, not read.** Planting only group 2's two glyph substitutions
— `src/ui/markdown.rs:156` `"> "` → `"│ "` and `:383` `"- "` → `"• "`, the smallest of the four
behaviour changes — and running `cargo test --lib` produces **25** deterministic failures
across five modules: `ui::view` 10, `ui::markdown` 6, `ui::app` 4, `ui::driver` 3, and
`ui::tests::detail` 2. (A raw run reports 28; three are `ui::tests::wiring::*` flakes from the
red baseline below, and separating them is why the number is stated as deterministic.)
Nineteen of the 25 sit outside the two production files this change edits and are pure
re-baselines. This
is why every behaviour group verifies with `cargo test --lib ui::` rather than a module filter,
and why the four test-only files above are named here: a plan that lists two files sends the
implementer to two files.

Most of those nineteen share one cause rather than nineteen: a single detail fixture built as
`format!("- line-{i:02}\n")`, repeated at `src/ui/driver.rs:815, 904, 905, 1223, 1261, 3625,
4370`; `src/ui/app.rs:1781, 1813, 1848, 3569, 3587, 3993, 4011`; `src/ui/mod.rs:798, 828, 831,
979, 1121, 1124`; and `src/ui/view.rs:3251, 3430, 3431, 3443`. The bullet glyph changes its rendered form once and every
site asserting it fails together, so the re-baseline is mechanical — which is the reason it is
safe to do in bulk, and the reason a reviewer must still check each one rather than trusting
that.

Group 1 is the opposite shape and is sized separately: the soft-break fold breaks **exactly
one** test, `ui::markdown::tests::a_soft_break_starts_a_new_line` (measured — 1221 passed, 1
failed with Decision 1 planted). Group 1's re-baseline is one literal, not a sweep.

The delta also carries `markdown-render`'s first requirement — "Markdown source becomes
plain-data lines, parameterised by the interior width" — because its two sweep scenarios must
gain task-list inputs. The new `Event::TaskListMarker` arm carries a four-column prefix and can
underflow `width.saturating_sub(prefix)` exactly as every other prefixed block can, and a
fixture asserting only 58 and 78 cannot reach that case.

**The `Change` type is not altered.** No field changes shape, so `changes::from_files` and
`changes::from_cli` need no reconciliation and `changes::merge` is untouched. This change is
entirely below the `Change` level, in how an artifact's already-read text is laid out.

## Contracts

`ui::markdown::lines` is crate-internal; its only consumers are `ui::detail::content_lines`
and `ui::tasks` (which reuses its `Line`/`Segment`/`Face` types, not its wrapper). The
**signature** is unchanged — additive in neither direction, because nothing is added: the same
function returns different `Line` values for the same input. `Line`, `Segment`, and `Face`
keep their exact shape, so `ui::view::style_for` needs no new arm and no new `Role` is added
to `ui::palette`.

No error surface: `lines` is total and returns no `Result`. No pagination, no streaming.

There is no external consumer. The plugin manifest, the config format, and every keybinding
are unchanged, so nothing here is **BREAKING** under this repository's definition.

## Persistence and Rollout

- Migration: none. Nothing is stored.
- Backfill: none.
- Seeding: none.
- Cache invalidation: **one item.** `artifact-content` caches rendered detail keyed on
  `(change directory, tab)`, not on width, and `Dashboard::sync_detail` re-renders on a width
  change already. No cache key changes; a running pane picks up the new rendering on its next
  draw after the binary is replaced, which is a restart.
- Index rebuild: none.
- Authorization: none. The pane is read-only and this change writes nothing.
- Observability: none. No new problem row, no new degraded state — one is **removed**.
- Deployment: `make build` and the pane's next start. No manifest change, so no
  `herdr plugin link` re-run.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| Filesystem | **mixed** — replaced for everything this change adds (`lines` takes `&str`; detail-route tests inject a `&dyn Fn(&Path) -> Result<String, String>` reader over an in-memory map), but **real** via `ScratchDir` in one carried `degraded-coverage` row, *A tasks file that cannot be read* | replaced, same injection |
| `openspec` binary | not reached — this change is below the `Change` level; the delta's `degraded-coverage` scenarios that do reach it use the existing `OpenspecCli` stub | replaced (stub `OpenspecCli`) |
| Herdr socket / `herdr` binary | **reached, real** — one carried `degraded-coverage` row, *The one-shot commands' degrades*, spawns this crate's own binary as a real process with a stub `herdr` first on `PATH`; and `ui::tests::wiring` reaches a real scratch `herdr` (observed under the `[✓]` plant, e.g. `agent start c-2fa-support --kind gemini --pane wD:pJ`). Nothing this change *adds* reaches it | replaced (stub `HerdrCli`) in the launch scenarios carried unchanged in the `degraded-coverage` delta |
| Terminal | replaced — `ratatui::backend::TestBackend` at 120x20 and 60x20 for every `Dashboard` render; `ui::markdown::lines` reaches no backend at all | replaced, same |
| `pulldown_cmark` | **real** — it is the unit under test for `ENABLE_TASKLISTS`; the ragged-table scenario asserts its event stream directly | real |
| `notify` watcher / refresh worker / agent poller / launcher | **not reached by anything this change adds**; one carried `degraded-coverage` row, *A watcher failure and a mid-run removal*, uses a stub `FsEvents` | replaced (stub `FsEvents`) in that one carried row, otherwise not reached |
| Clock | not reached | not reached |
| `HERDR_PLUGIN_STATE_DIR` and the process environment | not reached — no environment lookup is added | not reached |

## Test Strategy

Tiers, fastest first: **unit** (`cargo test --lib`, pure functions over `&str` and `u16`),
**view** (`cargo test --lib`, rendering into a `TestBackend` at 120x20 and 60x20), and
**contract** (`cargo test --test <name>`, binding a documented claim to its computable site).

Every `ui::markdown` and `ui::tasks` scenario asserts at **both** mandated interior widths —
58 and 78 — as this repository's width rule requires; the `Dashboard` scenarios assert at
120x20 and 60x20.

**The baseline is not green, and this change does not make it so.** Measured at HEAD:
`cargo test --lib ui::tests::wiring` fails 5 of 29 on one run and 7 of 29 on the next, over
the same test set — `a_keypress_launches_an_agent`, `a_polled_agent_reaches_a_rendered_badge`,
`a_failed_cli_cycle_keeps_the_file_numbers` and others in `ui::tests::wiring::*`. Run serially
with `-- --test-threads=1` the same set fails **1** of 29, in 173s against ~55s parallel. The
variance across identical runs, and the collapse to one failure under serialisation, is what
makes them timing-dependent rather than broken: these are the tests driving the four
worker-thread collaborators, and they are contending under parallel execution. They are **pre-existing and out of scope**:
this change touches no collaborator, no thread, and no clock. Every "no regressions" step
below therefore means *no new failure outside `ui::tests::wiring::*`*, and an implementer who
sees one of those go red should re-run before treating it as theirs. Fixing them is a separate
change; recorded here rather than silently absorbed, because a plan whose green is undefined
cannot report a regression.

**This change takes no outer-loop acceptance test.** The outer loop here is a real terminal,
and `ui` refuses to start when stdout is not a terminal (exit 3) precisely so `cargo test`
never drives one. The view tier over `TestBackend` is the outermost tier this repository has,
and every scenario below reaches it or a tier beneath it.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| A paragraph is word-wrapped, differently at the two mandated widths | unchanged existing test — `paragraph_wraps_at_58_and_78`'s fixture (`src/ui/markdown.rs:1179`) is one source line with no soft break, so the fold leaves it byte-identical | unit | none | `cargo test --lib ui::markdown` |
| An empty source and a zero width each produce no lines | unchanged existing test | unit | none | `cargo test --lib ui::markdown` |
| No line exceeds the width it was given | `ui::markdown` sweep over widths 0,1,2,3,10,58,78,200, fixture gaining a checked and an unchecked task-list item and one nested in a quote — 1, 2 and 3 sit under the item's own four-column prefix, so the `saturating_sub` underflow is measured | unit | none | `cargo test --lib ui::markdown` |
| A wide-character document wraps by columns at both mandated widths | unchanged existing test | unit | none | `cargo test --lib ui::markdown` |
| Rendering is total over arbitrary input | `ui::markdown` test, three task-list inputs added: a 500-character token, a bare `- [x]` with no text, and one nested three quote levels deep | unit | none | `cargo test --lib ui::markdown` |
| Bullet items carry their marker and wrap under their text column | `ui::markdown` test, `• ` marker and 2-space hanging indent at 58/78 | unit | none | `cargo test --lib ui::markdown` |
| An ordered list numbers from its own start value | unchanged existing test, re-run as a regression check that the glyph change did not reach ordered markers | unit | none | `cargo test --lib ui::markdown` |
| A nested list indents two columns per level | `ui::markdown` test, `• outer`/`  • inner`/`    • deepest` | unit | none | `cargo test --lib ui::markdown` |
| A table that fits renders as aligned columns at both mandated widths | `ui::markdown` test asserting the four box-drawing literals and `layout::columns == 25` on each | unit | none | `cargo test --lib ui::markdown` |
| A cell wider than its column wraps rather than being truncated | unchanged existing test; separators substituted | unit | none | `cargo test --lib ui::markdown` |
| A table too wide for the region spends its columns on the narrow ones | unchanged existing test — allocation is untouched | unit | none | `cargo test --lib ui::markdown` |
| Alignment markers pad the cell on the side they name | unchanged existing test | unit | none | `cargo test --lib ui::markdown` |
| A ragged table renders every declared column and drops no header column | `ui::markdown` test counting four separators per line **and** asserting the parser's own `TableCell` event stream | unit | `pulldown_cmark` real | `cargo test --lib ui::markdown` |
| A region too narrow for the pipe grammar renders one cell per line | `ui::markdown` sweep over widths 0–10 plus 58/78, asserting no `│` below the threshold | unit | none | `cargo test --lib ui::markdown` |
| A block quote prefixes every one of its lines | `ui::markdown` test, `│ ` and `│ │ ` prefixes, 57 columns at 58 | unit | none | `cargo test --lib ui::markdown` |
| A thematic break fills the interior at both widths | `ui::markdown` test, 58/78 `─` and `layout::columns` equal to the width | unit | none | `cargo test --lib ui::markdown` |
| A table renders as its literal source text, one row per line | `ui::markdown` test: footnote literal; task-list, table and strikethrough as discriminating controls; plus the `ui::tasks::lines` cross-check | unit | `pulldown_cmark` real | `cargo test --lib ui::markdown` |
| A soft break starts a new line rather than being folded | `ui::markdown` test: fold, hard-break carve-out, the sixty-word width-driven check, and the code-block leak check | unit | none | `cargo test --lib ui::markdown` |
| Exactly one blank line separates blocks, with none leading or trailing | unchanged existing test — the block-separation half is not changed | unit | none | `cargo test --lib ui::markdown` |
| Each emitted glyph measures one column, and the set is complete | `ui::markdown` test: `layout::columns` on each of the seven glyphs, plus the set-difference check over the scenario's enumerated fixture — which deliberately excludes an ordered list and an image, both of which emit a non-source character that is not a glyph | unit | none | `cargo test --lib ui::markdown` |
| The tasks tab shows checkboxes and its siblings show markdown | `Dashboard` render test `ui::view::tests::tasks_tab_shows_checkboxes`; the discriminator becomes the absent progress-bar row, and both paths' `[✓]` are compared | view | `TestBackend`, injected reader | `cargo test --lib ui::view` |
| The tab is chosen by `tracks_tasks`, not by its id | `Dashboard` render test `ui::view::tests::tasks_tab_chosen_by_flag` at both sizes | view | `TestBackend`, injected reader | `cargo test --lib ui::view` |
| A schema naming no tasks artifact leaves every tab as markdown | `Dashboard` render test `ui::view::tests::no_marked_artifact_renders_markdown`, `[ ] a` on every tab | view | `TestBackend`, injected reader | `cargo test --lib ui::view` |
| A `detail.tab` past the end of the artifact list renders no checklist | unchanged existing test `ui::detail::tests::tab_past_the_end` (`src/ui/detail.rs:1484`) | view | `TestBackend`, injected reader | `cargo test --lib ui::detail` |
| Groups, headings, items, and separators at both mandated widths | `ui::tasks` test with `[✓]` | unit | none | `cargo test --lib ui::tasks` |
| A nested item reproduces its own indent | `ui::tasks` test with `[✓]` | unit | none | `cargo test --lib ui::tasks` |
| A long item wraps with a hanging indent at both widths | unchanged existing test | unit | none | `cargo test --lib ui::tasks` |
| An unbreakable word is hard-split rather than lost | `ui::tasks::tests::unbreakable_word_hard_split`, re-baselined — it does `t.strip_prefix("[x] ")` at `src/ui/tasks.rs:751` | unit | none | `cargo test --lib ui::tasks` |
| The indent is dropped whole as the width collapses | `ui::tasks` test with `[✓]` at column zero | unit | none | `cargo test --lib ui::tasks` |
| A heading with no items still renders its heading | unchanged existing test | unit | none | `cargo test --lib ui::tasks` |
| A headingless leading group renders without a heading line | `ui::tasks` test with `[✓] loose` | unit | none | `cargo test --lib ui::tasks` |
| A checklist of wide-character items fits at both mandated widths | `ui::tasks` test; the `[✓]` glyph's interior column offsets asserted unchanged from the ASCII case | unit | none | `cargo test --lib ui::tasks` |
| No checklist line exceeds its width at any width | unchanged existing sweep over every width | unit | none | `cargo test --lib ui::tasks` |
| A footnote, strikethrough, and a table each render as literal source | test named by `tests/degraded-coverage.toml`, re-pointed at the narrowed condition, with the task-list item added as a fourth control | unit + contract | `pulldown_cmark` real | `cargo test --lib ui::markdown && cargo test --test degraded_coverage` |
| A schema the CLI rejects falls back per change and names the reason | unchanged existing proof | unit | stub `OpenspecCli` | `cargo test --test degraded_coverage` |
| A schema that is not vendored and one that will not parse both empty the tab bar | unchanged existing proof | unit | stub `OpenspecCli` | `cargo test --test degraded_coverage` |
| A schema with no tasks artifact renders every tab as markdown and still counts | unchanged existing proof | view | `TestBackend` | `cargo test --test degraded_coverage` |
| An unsupported `generates` glob empties one artifact and names why | unchanged existing proof | unit | none | `cargo test --test degraded_coverage` |
| A tasks file that cannot be read is zero tasks with a named reason | unchanged existing proof | unit | `ScratchDir` real | `cargo test --test degraded_coverage` |
| Each of the six launch failures renders as a leading problem row | unchanged existing proof | view | stub `HerdrCli` | `cargo test --test degraded_coverage` |
| `g` with no attributed agent changes nothing the pane shows | unchanged existing proof | view | stub `HerdrCli` | `cargo test --test degraded_coverage` |
| A CLI root disagreement and a non-zero exit both leave the file numbers standing | unchanged existing proof | unit | stub `OpenspecCli` | `cargo test --test degraded_coverage` |
| A watcher failure and a mid-run removal both keep the loop drawing | unchanged existing proof | view | stub `FsEvents` | `cargo test --test degraded_coverage` |
| An out-of-scope agent and a worktree agent are both invisible | unchanged existing proof | unit | stub `HerdrCli` | `cargo test --test degraded_coverage` |
| A duplicate artifact id is accepted by this crate and stays file-mode | unchanged existing proof | unit | none | `cargo test --test degraded_coverage` |
| The one-shot commands' degrades are proved by exit status and stderr | unchanged existing proof | contract | real process, stub `herdr` on `PATH` | `cargo test --test degraded_coverage` |

The twelve `degraded-coverage` rows marked "unchanged existing proof" are carried into the
delta because OpenSpec's MODIFIED operation requires the whole requirement block; only the
footnote row's scenario changes. They are listed because the matrix admits no silence, and
re-running them is the check that the requirement was copied rather than rewritten.

## Decisions

**Decision 1 — Fold `SoftBreak`, keep `HardBreak` breaking, at the one shared arm.**
`Event::SoftBreak | Event::HardBreak => f.group_break()` (`src/ui/markdown.rs:695`) splits into
two arms: `SoftBreak` pushes a single-space `Run` into the current group, `HardBreak` keeps
calling `group_break()`. Nothing else changes — `groups` stays a `Vec<Vec<Run>>` and the
emitter keeps wrapping each group independently, because a group now means "a hard-broken
run of prose" rather than "a source line".
*Alternative considered:* collapse `groups` to a single `Vec<Run>` and delete the group
machinery. Rejected: hard breaks, table cells, and the verbatim path all still need the
grouping, so the deletion would have to be re-added.
*Alternative considered:* fold in a pre-pass over the source string. Rejected: it cannot tell
a paragraph newline from one inside a fenced block without re-implementing the parser.

**Decision 2 — Verbatim blocks need no guard.** Measured against pulldown-cmark 0.13.4, a
fenced or indented code block arrives as `Event::Text` carrying its own newlines, never as
`Event::SoftBreak`, so the fold cannot reach it. The spec asserts this as a scenario clause
rather than trusting the measurement, because it is a property of the parser and not of this
module.

**Decision 3 — The delimiter line is the row line's own shape, not a second arithmetic.**
Rather than computing `─` runs from `w[j] + 2`, the delimiter line is specified as a row line
with every space and content column replaced by `─` and the separators replaced by `├`/`┼`/`┤`.
Alignment then holds by construction. The existing implementation already builds it from the
same widths, so this is a specification simplification the code can follow directly.
*Alternative considered:* glow's borderless form — no leading or trailing separator, interior
`│` only. Rejected: it changes `total` from `3n + 1 + sum(w)`, invalidating the narrow-fallback
threshold `width < 4n + 1` and every width literal in four scenarios, for no legibility gain
at 58.

**Decision 4 — The Ambiguous-width exposure is accepted and written down, not compensated.**
Six of the seven new glyphs are East Asian Ambiguous and a CJK-locale terminal paints them at
two columns while `unicode-width` — and therefore `layout::columns`, and therefore
`Buffer::set_string` — says one. `SPEC.md` already names this class as an accepted,
uncompensated overrun for content the artifacts carry; this change extends it to the pane's
own chrome and says so in `SPEC.md` rather than leaving the widened scope implicit.
*Alternative considered:* measure with an Ambiguous-aware width table behind a config flag.
Rejected on `SPEC.md`'s standing rule — any compensation is a guess that breaks the terminal it
guessed wrong for — and because it would put a second width measure in a crate whose
`COLWIDTH` gate exists to keep exactly one.
*Alternative considered:* keep ASCII throughout. Rejected by the user after this trade-off was
presented in full; the legibility of rendered markdown was judged worth the widened exposure.

**Decision 5 — `glow` and `tui-markdown` are rejected as the renderer. Recorded so it is not
re-litigated.** Both were evaluated against this repository's own corpus at width 78.
`glow` 3.0.0 is a Go binary and therefore a subprocess: under the standing rule that only
`src/cli.rs` may spawn, it would need a third CLI trait plus a collaborator outside `src/ui/`
with its own thread and a `(change, tab, width)` cache, since a per-frame spawn is barred by
the no-blocking-render-path rule. It emits OSC-8 hyperlink escapes that `ansi-to-tui` does not
parse, hardcodes a **two-column margin on each side** and pads every row to full width — four
of the detail region's 58 interior columns spent on nothing, measured: `glow -w 20` yields
`'  aaaa bbbb cccc  '` and `glow -w 58` yields 54 content columns — allocates table columns equal-width rather than
content-fitted (worse at 58), and its colours arrive as raw SGR, bypassing `ui::palette` and
breaking every render test that asserts a cell against `palette::style(role)`. Under the
never-fail-closed rule it would have to be kept **alongside** `ui::markdown` as the fallback for
a machine without it, so it adds code rather than removing it. Measured on this repository's own hard-wrapped artifacts at a matched content width it
produces the same raggedness as today's renderer — identical for 32 consecutive lines — so it
does not even fix the defect that prompted the comparison. `tui-markdown` 0.3.9 is better shaped — in-process,
ratatui-native, MSRV 1.88 exactly at this crate's floor — but still bypasses `ui::palette`,
still offers no width contract this repository can assert at 58 and 78, and would trade 2,646
tested lines for a dependency. Its `syntect` and `ansi-to-tui` deps ride on the **default**
`highlight-code` feature, so under this crate's uniform `default-features = false` it is one
crate rather than three — a smaller cost than first stated, and still not the deciding one. Syntax highlighting is the only
real gain either offers and is not worth that.

**Decision 6 — `tasks-checklist` moves to `[✓]` in the same change.** The reversal being made
in `markdown-render` was argued against on the grounds that "a second checkbox renderer beside
`ui::tasks`' own … is how two renderers drift apart". That risk is real and is answered
structurally rather than dismissed: both paths render the same three-column glyph, and the
`tasks-checklist` scenario "The tasks tab shows checkboxes and its siblings show markdown"
asserts the two agree. A future divergence is then a failing test.
*Alternative considered:* leave `ui::tasks` on `[x]`. Rejected — it is precisely the drift the
original argument predicted, arriving on day one.

**Decision 7 — the checkbox glyph replaces the bullet marker rather than following it, and the
hanging indent moves with it.** `[✓] ` is the item's whole marker, so a task-list item's prefix
is four columns where a `• ` item's is two. The consequence is the one that bites: `start_item`
(`src/ui/markdown.rs:372-390`) builds `first_prefix` and `cont_prefix` as two separate strings,
`cont_prefix` from `columns(&marker)`, and it runs **before** `TaskListMarker` is seen. An arm
that sets only the first line's prefix leaves continuations hanging at two columns, and no
width or totality assertion can detect it — a shorter indent never overruns. The spec therefore
carries an explicit wrapped-item clause and 4.2 an explicit `cont_prefix` instruction.
*Alternative considered:* render `• [✓] ` — keep the bullet and prepend the checkbox. Rejected:
it costs six prefix columns at a 58-column interior, and it renders a glyph pair no markdown
reader shows.

## Risks / Trade-offs

- A CJK-locale terminal paints six of the seven glyphs at two columns, overrunning every line
  that carries one → Accepted, not mitigated (Decision 4). Written into `SPEC.md` so the next
  reader finds the decision rather than the symptom.
- Reflow destroys an author's deliberate line shape where they used a soft break rather than a
  hard one → Mitigated by the hard-break carve-out, and bounded: verbatim blocks are untouched
  and pipe tables are modelled, so the two shapes the original argument named are safe.
- `ENABLE_TASKLISTS` turned on without its render arm makes the checkbox **vanish** into
  `fold`'s `_ => {}` wildcard, invisibly to every gate → Mitigated by landing the option and
  the arm in one task, and by the delta's scenario asserting the rendered glyph rather than
  the option's presence.
- Re-baselining 34 tests risks a literal being "corrected" to match a wrong implementation →
  Mitigated by the RED-first order in `tasks.md`: each changed assertion is written and seen to
  fail before the implementation moves.
- `SPEC.md` prose drifts from the code → `tests/doc_contract.rs` and `tests/degraded_coverage.rs`
  already bind it; the degraded-states row edit must be made in both `SPEC.md` and
  `tests/degraded-coverage.toml` or `cargo test` fails, which is the intended behaviour.

## Migration Plan

None needed. No stored data, no schema, no manifest, no config format, and no keybinding
changes; the pane is read-only. Deploy is `make build` and the next pane start. Rollback is
`git revert` of the change's commits — there is no state to unwind.

## Open Questions

None. The one live question — whether to accept the Ambiguous-width exposure for the glyph set
or stay ASCII — was put to the user with the measurement in hand and answered in favour of the
glyphs; it is recorded as Decision 4 rather than left open.

## Visual Design

Not applicable. This change modifies a terminal view, and no HTML or image design source
exists for it. The rendered grammars are specified as literal text in the delta specs, which
is this repository's equivalent and is where the visual contract lives.
