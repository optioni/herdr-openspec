## Context

`tasks-emphasis` chose alternating shade for group boundaries because separators were measured
too expensive — at the archive's worst case, 22 groups, 21 separator columns would eat 21 of
the narrow interior's 45 gauge columns. That reasoning was correct and is untouched here. What
it did not weigh is that the fill boundary is itself drawn on the same scale as the boundaries
it was adding, so the new signal and the existing one became indistinguishable.

## Goals / Non-Goals

**Goals.** The fill boundary is the highest-contrast edge in the bar. Group boundaries stay
visible in both halves. Every property `tasks-emphasis` established holds by construction, not
by re-assertion.

**Non-Goals.** No change to fill arithmetic, `gauge_of`, `detail-header`, the legibility floor,
the partition rule, or the no-separator decision. Not a fix for the width exposure.

## Decisions

**1. Two character families, not one lightness scale.** This is the whole change. Blocks mean
filled, braille dots mean empty, and the reader learns one categorical rule instead of ranking
four shades. It follows that both halves can carry boundaries: `█`/`▒` in the filled half is a
50-point step, but it competes with nothing, because the empty half contains no shades for `▒`
to be compared against.

**2. `▒` is the filled partner, not `▊` or `▓`.** Three candidates were rendered at both
mandated widths against real data:

| Filled pair | Objection |
|---|---|
| `█`/`▓` | `▓` is a shade — visually "partly filled" — used to mean "fully filled, other group" |
| `█`/`▉` | some fonts round seven-eighths to full and the alternation disappears entirely |
| `█`/`▊` | unambiguous, but the ribbing is low-contrast and the filled half's boundaries nearly vanish |
| **`█`/`▒`** | highest contrast of the three; the shade-means-partial objection does not apply once the empty half is not a shade |

`▒` was chosen after rendering all of them in the target terminal. The objection to it —
that a 50% shade might read as unfilled — was raised during exploration and does not survive
Decision 1: there is no lightness scale in the empty half for it to sit on.

**3. `⢕` and `⠌`, not `⣿`/`⠿` or a denser pair.** The empty half must read as clearly lighter
than the filled half, or the fill edge weakens again. `⢕` is 4 dots of 8 and `⠌` is 2 of 8,
against a solid block — a wide margin. `⣿` (8/8) and `⠿` (6/8) were rendered and rejected: they
make the empty half read as heavy and put the bar back where it started.

`⢕` and `⠌` also differ in **shape, not only in density** — `⢕` is a four-dot checkerboard
(dots 1,3,5,8) and `⠌` a two-dot diagonal stroke (dots 3,4) — so the boundary between two empty
groups shifts both the count and the arrangement of the dots. That is a second signal at no
cost. An earlier draft said `⢕` occupies the right-hand dot column and `⠌` the left; that is
false — decoded against the 8-dot layout (1 4 / 2 5 / 3 6 / 7 8) both straddle both columns —
and it is recorded here because a later glyph swap reasoning from the column claim would pick
wrong.

**4. In phase.** Even-indexed groups get `█` and `⢕`; odd-indexed get `▒` and `⠌`. A group
straddling the fill boundary therefore keeps the "even" or "odd" identity on both sides of it.
Out of phase, such a group would change glyph family *and* parity at one position and read as
two groups — and a straddling group is the common case, since exactly one group is usually
in progress.

**5. `gauge_of` does not move, so `detail-header` does not either.** The header's gauge is
passed no groups and is segmented by nothing; it is the unsegmented form by definition, and
`█`/`░` is exactly right for it. This leaves two vocabularies in the crate, which would be
drift if they overlapped — they do not: **blocks are the unsegmented vocabulary and braille is
the boundary vocabulary**, and braille appears exactly where boundaries are being marked. An
earlier draft of this design moved `gauge_of` to `█`/`⢕` for uniformity; it was dropped because
it churns three `detail-header` requirements and dozens of scenario lines naming `░`, to make
a gauge that has no boundaries use the boundary vocabulary.

**6. The substitution stays a substitution.** `segmented_gauge` keeps taking `gauge_of`'s run
and rewriting glyphs position by position. Today it rewrites only odd-group positions; it will
now rewrite three of the four cases (`█`→`▒` odd-filled, `░`→`⢕` even-empty, `░`→`⠌`
odd-empty). The count of rewritten cases is not the property that matters — that no position
changes *whether it is filled* is, and that is preserved because the match reads the incoming
glyph to decide.

**7. The width exposure is recorded, not fixed.** Measured against UCD 16.0.0: `█` and `▒` are
Ambiguous, `⢕` and `⠌` are Neutral. In a CJK-locale terminal the filled half may paint two
columns per position and the empty half one, so the bar is **mis-proportioned** — 53% reads as
roughly 69%.

This is not a regression. The current table mixes Ambiguous `█`, `▓`, `▒` with **Neutral** `░`,
so that terminal already mis-proportions the bar *and* draws its empty half ragged. The change
removes the raggedness and keeps the mis-proportion. Closing it entirely needs every glyph in
one width class, which means an all-braille bar — whose filled half is dotted and no longer
reads as solid, which is the defect this change exists to repair. The exposure is therefore
written into the requirement, in the shape `markdown-render` already uses for its own glyphs.

**8. The "all four are Ambiguous" claim is corrected in the same pass, at all three sites.**
`SPEC.md:404-410`, the matching paragraph in `AGENTS.md:141-145` (`CLAUDE.md` is a symlink to
it), and `openspec/specs/tasks-progress-bar/spec.md:65-70` — the *grammar* requirement, not the
segmentation one — all say it. `░` U+2591 is Neutral (`python3 -c "import unicodedata;
print(unicodedata.east_asian_width(chr(0x2591)))"` → `N`, UCD 16.0.0), so the claim is false,
and it describes the failure as uniform doubling when the real failure is a **ragged** bar.
All three sentences name `▓`, which this change retires, so they must be rewritten regardless
— and rewriting a sentence into a knowingly false one is not an option.

The third site is why this change carries **two** MODIFIED requirements rather than one. The
grammar requirement owns the "every character the bar can hold is one column wide" claim for
the whole capability; leaving it to the archive would fold a retired glyph and a false width
statement into `openspec/specs/` under a requirement this change never touched, which is
exactly the spec drift the repository's rules forbid.

Binding each glyph's width property to a `tests/doc_contract.rs` claim would be the durable
fix for the *prose* and is **not** in this change: it needs a checked-in width table, since
the crate has no `unicode-width` dependency of its own, and that is an argument to have on its
own terms. What this change does instead is bind the property the prose is *about* — the
grammar requirement's new scenario asserts every glyph measures one column through
`layout::columns`, so the arithmetic claim is executable even while the East Asian Width claim
beside it stays prose. `grep -n 'gauge\|Ambiguous' tests/*.rs` selects **0** lines at HEAD,
which is why the claim could go wrong unnoticed in the first place.

## Risks / Trade-offs

| Risk | Mitigation |
|---|---|
| Braille renders as tofu in some terminal fonts | Rendered in the target terminal before choosing. Accepted for the pane's own font; noted here so a font change is a known trigger to re-check |
| A screen reader announces braille patterns as braille | Real, and unmitigated. The gauge is decorative — the `[25/47] 53%` cells beside it carry the same fact in text, and they are what a reader of the row actually parses |
| `▒` read as unfilled | Decision 1 and 2; asserted by "The fill boundary is the only change of character family", which requires exactly one block→braille transition and none the other way |
| The two halves drift out of phase in a later edit | Asserted by "A group straddling the fill boundary keeps one identity" |

## Migration Plan

None. No persisted state, no config, no manifest, no keybinding.

## Modules touched

`src/ui/tasks.rs` only **in production** — `segmented_gauge`'s match arms and its doc comment
(`grep -n '▓\|▒' src/ui/tasks.rs` selects 3 production lines: `:133`, `:201`, `:202`).
`src/ui/view.rs`'s test module moves with it: its one frame-rendering test names the retiring
glyphs at six sites and is rewritten in the same group, which is why tasks.md's group 1 writes
two files rather than one. **No process spawn is added**;
`src/ui/tasks.rs` names none and `TASKSEAM` sweeps it. **No view gains I/O**: the function
stays a pure `&str`-producing transformation and `NOIO-VIEW`'s pure set is unchanged in
membership. **The `Change` type does not move**, so `changes::from_files` and
`changes::from_cli` need no reconciliation.

## Test Strategy

**No outer-loop acceptance group.** The change binds no key, alters no state transition, and
adds no entry point: it rewrites the glyph a position is drawn with inside one pure function.
A `run_loop` row would have to drive an untouched key to observe a rendering the view tier
observes directly. The **view** tier is therefore the outermost tier that can fail here, and
`a_real_tasks_tab_renders_a_segmented_gauge_into_the_frame` in `src/ui/view.rs` is the row
that carries it — the one scenario in this package that reaches `progress_bar` through
`ui::detail::content_lines` rather than calling it directly, which is what stops every other
scenario passing against a build whose production caller hands over an empty slice.

## Test Boundaries

| Collaborator | Treatment | Why |
|---|---|---|
| Filesystem | **Absent** | No test reaches a real directory. Every scenario passes a hand-built `Progress` and slice, except the one frame render — see the row below |
| Artifact reader | **Replaced** by `testutil::RecordingReader::always`, injected into `sync_detail` as the `&dyn Fn(&Path) -> Result<String, String>` seam (`src/ui/view.rs:5032-5034`) | This is the package's one replaced collaborator and the reason the frame scenario is worth its cost: the reader is what drives the **production** path to populate `groups` from `detail.sections`, so that scenario alone would go red against a build whose production caller handed `progress_bar` an empty slice. Every other scenario hand-builds the slice and would pass against it |
| `openspec` binary | **Not reached** | No CLI path is touched |
| Herdr socket | **Not reached** | No agent, launch, or pane surface is touched |
| Terminal | **Replaced** by `ratatui::backend::TestBackend` at 60 and 120 columns for the one frame-rendering scenario | `cargo test` spawns this binary |
| Clock | **Not reached** | No timing behaviour changes |

## Verification Matrix

Tier key: **U** unit over `ui::tasks` (`cargo test --lib ui::tasks`), **V** view test into a
`TestBackend`. `TASKWIDTHS` requires every `#[test]` in `src/ui/tasks.rs` to name both `58` and
`78`; every row below is called at both.

| Scenario | Tier | Collaborators | Command |
|---|---|---|---|
| Two groups of unequal size get spans proportional to their item counts | U | none | `cargo test --lib ui::tasks` |
| An empty group contributes no span and consumes no index | U | none | `cargo test --lib ui::tasks` |
| Segmentation is skipped below the legibility floor | U | none | `cargo test --lib ui::tasks` |
| A single group is never segmented | U | none | `cargo test --lib ui::tasks` |
| Segmentation is total and partitions the run exactly | U | none | `cargo test --lib ui::tasks` |
| **The fill boundary is the only change of character family** | U | none | `cargo test --lib ui::tasks` |
| **A group straddling the fill boundary keeps one identity** | U | none | `cargo test --lib ui::tasks` |
| A real tasks tab renders a segmented gauge into the frame | V | TestBackend | `cargo test --lib ui::view` |
| **Every glyph the bar can draw measures one column** | U | none | `cargo test --lib ui::tasks` |
| The full grammar at both mandated interior widths | U | none | `cargo test --lib ui::tasks` |
| The percentage truncates rather than rounds | U | none | `cargo test --lib ui::tasks` |
| The bar measures at most its width at every width | U | none | `cargo test --lib ui::tasks` |
| A saturating `Progress` renders a full gauge and a full percentage | U | none | `cargo test --lib ui::tasks` |
| The bar reaches the buffer at both mandated frame widths | V | TestBackend, artifact reader | `cargo test --lib ui::view` |

The three bold rows are new. The six beneath the first group are carried by the **segmentation**
block with their assertions rewritten to the new glyphs, and are re-run as regression because
every one of them counts glyphs this change replaces.

The five unbold rows at the foot belong to the **grammar** block and are a different case:
they are carried **unchanged**. Each passes an empty `groups` slice, so no glyph in them moves
and no assertion is rewritten — `grep -n '▓\|▒'` selects none of their bodies. They are listed
because the grammar requirement is MODIFIED in full and a matrix that named only its one new
scenario could not be told apart from a matrix that had forgotten the other five. Their tests
are `src/ui/tasks.rs:621`, `:2046`, `:2085`, `:2229` and `src/ui/view.rs:7627`.

The matrix header's "every row below is called at both" holds of the U rows; the two V rows are
frame renders at 120x20 and 60x20 rather than interior widths 78 and 58.

## Gates

`make check` is the single gate.

- **`TASKWIDTHS`** — satisfied by construction; both mandated widths appear in every test.
- **`COLWIDTH`** — the gauge is built by pushing `char`s into a `String` and its length is
  asserted through `ui::layout::columns`, never `.chars().count()`. Unchanged by this edit.
- **`TASKSEAM`** — no `ratatui` type and no filesystem edge is added; the `tasks::parse(` leg
  is untouched. **The limit that bites here:** `scripts/gates/taskseam.sh` greps the whole of
  `src/ui/tasks.rs` — prose and doc comments included — for `ratatui|Modifier|Style|Span|Rect|Frame|Buffer`,
  case-sensitively. `segmented_gauge`'s doc comment says *stretch* rather than the obvious word
  for exactly this reason and records why at `:159-164`, while this change's own spec and design
  prose say "span" throughout. A doc-comment rewrite that lifts a sentence beginning "Spans are
  proportional…" turns `make gates` red on the capital `S`.
- **Contract tier** — `tests/doc_contract.rs` does not currently bind the gauge glyphs, which
  is why the `SPEC.md` claim could go wrong unnoticed. Decision 8 records that and declines to
  add the binding here.

Coverage stays at the 80% line floor with no exclusion added.
