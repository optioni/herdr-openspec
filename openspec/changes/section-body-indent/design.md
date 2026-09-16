## Context

`ui::detail::content_lines` draws a section header at `"  ".repeat(depth)` (`src/ui/detail.rs`
→ `header`, reached at the section walk) and then draws that section's body at the full
`width`, starting at column zero. The body therefore hangs left of its own header, and left of
every ancestor header above it. Measured over this repository's 236 delta spec files: 3,830
section headers, of which **2,872 are at depth 3** — six columns of misalignment is the common
case, not the corner.

`artifact-folds` states the unindented body deliberately, protecting the 58-column narrow
interior's text column. This change keeps that protection and narrows its scope to the width
that actually needs it.

## Goals / Non-Goals

**Goals.** A body sits flush beneath its own header at the wide layout. The narrow layout keeps
every column it has today. One decision per render, so the tab's left edge is never half
aligned.

**Non-Goals.** No header row moves. No fold level, keybinding, cursor behaviour, or
`section_at` lookup changes. No change to markdown wrapping itself. The tracked-tasks tab is
out of scope by construction — its sections are at depth 0.

## Decisions

**1. Indent the body, wrap it narrower — not prefix a full-width row.** An indented body row is
`indent` spaces followed by what `ui::markdown::lines` produced at `width - indent_cols`.
Prefixing a row already wrapped at `width` would push every long line `indent_cols` past the
interior, which the region does not clip and `responsive-layout` forbids. This is the same
shape the item grammar already uses for its hanging indent.

Three row kinds are **not** a section's body and are not indented. `ui::tasks::bar_lines`'
progress rows lead the whole body, owned by no section and hidden by no fold, so they have no
header to align beneath. The blank `separator_row` is already `pad_or_truncate_right("",
width)` — exactly `width` blank columns — so prefixing it would make it the one row that
exceeds the region, and it is blank, so the two readings are visually identical and the defect
would be silent. A `None`-labelled **preamble**, by contrast, *is* a body and *is* indented,
by its own `depth`, which is `base` and not necessarily 0: for a glob artifact the file section
is depth 0 and the preamble beneath it is depth 1.

**2. One decision per render, from the tab's deepest body-bearing section.** The alternative —
deciding per section, so a depth-1 body indents where a depth-3 body cannot — produces a tab
whose left edge steps in and out with no rule a reader can infer. A single decision makes the
tab either consistently aligned or consistently flush, and both are legible. It also means the
rule is evaluated once, not once per row.

**Body-bearing** means exactly `!section.text.is_empty()` — not "renders at least one line",
which is a different and narrower predicate (`seven_section_detail`'s depth-1 section carries
`"\n"`, which is non-empty and renders nothing). The looser rule can only raise the floor,
never lower it, so it is safe; it is stated as the byte test because a fixture carrying a
whitespace-only body would otherwise move a tab's floor by two columns for a reason no reader
could see. It counts every section that could draw a body, not only the ones open on the frame
being drawn. Reading fold state instead would make
the text column widen as a deep section is collapsed and narrow again as it is opened, so
`Space` would reflow the prose of every sibling that stayed open. The indent is a property of
the tab, not of the cursor's fold history, and computing it from `detail.sections` rather than
from `visible_sections` is what makes that true.

**3. The floor is `width.saturating_sub(2 * max_depth) >= 64`, and 64 is a measured trade, not
a derivation.** The subtraction must saturate: `width` is a `u16`, `depth` a `usize` (so the
expression needs one explicit conversion), and this capability's existing width sweeps run from
`0`, reaching every width below `2 * max_depth` — an unguarded `-` panics there in the debug
build `cargo test` uses. The *other* subtraction, `width - indent_cols`, needs no guard of its
own but only because of this one: the floor leaves at least 64 columns, so an indented body's
wrap width is never zero and `markdown-render`'s width-0 empty-vector path is unreachable.
Stated plainly because a constant that pretends to be principled is worse than one that shows
its working:

| Tab's max depth | Indent cost | Text column at 78 | Text column at 58 | Floor | Indents at |
|---|---|---|---|---|---|
| 1 | 2 | 76 | 56 | 66 | width ≥ 66 |
| 2 | 4 | 74 | 54 | 68 | width ≥ 68 |
| 3 (the archive's deepest) | 6 | **72** | **52** | 70 | width ≥ 70 |

64 is placed so that the archive's deepest spec — depth 3 — indents at the 78-column wide
interior (72 ≥ 64) and does not at the 58-column narrow one (52 < 64). That is precisely the
trade this change was asked to make, and the table is the record of how the constant was
chosen.

Depth 3 is the archive's deepest but not its only shape: a change touching one capability
resolves `specs` to a single path, so `base` is 0 and its `max_depth` is 2, floor 68 — still
indenting at the 78-column interior, so no behavioural difference, but it is the reason the
floor is not simply written as the constant 70.

The floor reads the **tab's own** maximum depth rather than being a fixed width, so a shallow
tab indents at a narrower pane than a deep one. That falls out of the rule rather than being a
second mechanism, and the scenario "A shallower tab indents at a narrower width" is what keeps
it from silently collapsing into a constant.

**4. One rule for every tab: the tracked-tasks tab indents too, and gets no exemption.** An
earlier draft of this design said the tasks tab could not regress because every tracked-tasks
section is at depth 0. That is false. `base` is `usize::from(paths.len() > 1)`
(`src/ui/app.rs`), so a tracked-tasks artifact resolving to more than one path starts at depth
1; and `depth` is `base + (level - min_level)`, so a task file opening with a level-1 title
puts every `## ` group at depth 1 — the shape 17 of this repository's 44 task files have today.
At depth 1 the floor is 66, the 78-column interior clears it, and the items shift two columns.

Because the premise was false, the exemption was never really chosen; it is now chosen
deliberately, and against. `tasks-checklist`'s items are a section's body like any other, and
an exemption would make the indent depend on the selected change's artifact list rather than on
`(sections, width)` alone — a second discriminator to keep in step with the body-grammar one.
Two scenarios fix this from both sides: a genuinely depth-0 tab is unmoved *by its depth*, and
a depth-1 tab indents its items two columns at 78 and not at 58. The first asserts
byte-identity rather than reasoning about it, because `task-item-bodies` is rewriting that same
grammar and a regression there would otherwise be attributed to the wrong change.

**5. The copied indent is accepted, not trimmed.** `span_text` reads each covered row's
rendered text, taking `from = 0` for every interior line and only `trim_end`ing it, so an
indented body row is copied with its leading spaces. That is kept, because a **header** row's
own `"  " * depth` already copies the same way: trimming one and not the other would make the
selection's output depend on which kind of row it crossed. The indent is rendered content, not
the trailing padding `text-selection` drops. The consequence that a double click inside the
indent selects nothing follows from that capability's own existing whitespace rule, and needs
no new decision. Nothing today would catch either — no existing selection test uses a fixture
with a section deeper than 0 — so this change adds the scenario that does.

**6. The previous rule's rationale is kept in the spec, not deleted.** The sentence about
reducing the text column stays, reworded as the reason the floor exists. A future reader who
finds only the new rule would have no way to know the narrow layout was considered and
deliberately excluded; this repository's specs carry that kind of reasoning on purpose.

## Risks / Trade-offs

| Risk | Mitigation |
|---|---|
| The constant `64` drifts out of meaning as spec depth grows | The floor reads `max_depth` at render time, so a depth-4 tab gets a floor of 72 with no edit; and the derivation table above is in the spec, not only here |
| A width near the floor flickers between layouts as the pane is resized | Accepted: one transition width per tab, asserted as exactly one in "The indent is all-or-nothing across one render". A hysteresis band would need state in a pure view |
| `task-item-bodies` and this change both delta the same requirement | Sequenced explicitly in the proposal's Impact: this lands and archives first, then that change's delta is refreshed. Stated there rather than here because it is a scheduling fact, not a design one |

## Migration Plan

None. No persisted state, no config, no manifest, no keybinding.

## Modules touched

One production site — the section walk in `content_lines` in `src/ui/detail.rs`, which already
holds each section's `depth` and the content width. **Three** existing tests move with it,
because they pin the behaviour being reversed: `src/ui/view.rs`'s
`a_body_row_is_never_indented_by_its_sections_depth` (`artifact-content`'s proving test), and
in `src/ui/detail.rs`, `a_badged_header_row_is_still_addressed_by_its_own_section_index`
(depth-1 fixture, exact body-text equality) and
`the_tracked_tasks_tab_concatenates_rather_than_folding` (two-path tracked-tasks fixture, its
groups at depth 1). That set is measured, not predicted: a minimal implementation planted at
planning time gave `cargo test --lib` → `1445 passed; 3 failed`, and these are the three. The maximum depth is computed from
`detail.sections` in the same pass that already builds `visible_sections`.

**No process spawn is added** — `src/ui/detail.rs` names none and gains none. **No view gains
I/O**: `content_lines` stays a pure function of `Detail` and a width, and `NOIO-VIEW`'s
ten-file pure set is unchanged in membership. **The `Change` type does not move**, so
`changes::from_files` and `changes::from_cli` need no reconciliation.

## Test Strategy

**No acceptance/outer-loop group.** `content_lines` is a pure function from a `Detail` and a
width to a row list; there is no end-to-end wiring whose assembly is the risk. The change moves
rows sideways and, wherever a body wraps, **adds** rows, since the body is re-wrapped at
`width - indent_cols` rather than prefixed — so `rows.len()` moves too. Every consumer
re-derives the list from the same call at the same width — `ui::view::render`
(`src/ui/view.rs`, `content.width`), `normalise_scroll` (`src/ui/driver.rs`,
`detail.drawn_width`), `detail_cell` and `clamp_to_content` (both `content.width`) — so the
drawn frame, the scroll clamp, and the pointer resolvers cannot disagree about the indent
decision or the row count. Had any of them derived its own width, an outer loop would have been
mandatory. Every scenario is asserted against `content_lines`' returned rows
directly, at the tier the matrix below names.

## Test Boundaries

| Collaborator | Treatment | Why |
|---|---|---|
| Filesystem | **Absent** | `content_lines` takes a `Detail` built in the test; no scenario reads a file |
| `openspec` binary | **Not reached** | No CLI path is touched |
| Herdr socket | **Not reached** | No agent, launch, or pane surface is touched |
| Terminal | **Replaced** by `ratatui::backend::TestBackend` where a scenario renders a frame; the indent scenarios themselves assert `content_lines`' rows directly | `cargo test` spawns this binary, so no test may reach a real terminal |
| Clock | **Not reached** | No timing behaviour changes |
| `ui::markdown::lines` | **Real**, called at the reduced `width - indent_cols` | Decision 1's correctness is precisely that the body is *wrapped* narrower rather than prefixed; a replaced wrapper could not show that |
| `ui::tasks::items` | **Real**, called at the same reduced width | The tracked-tasks tab takes the same path under Decision 4, and the depth-1 scenario asserts the narrower wrap |

## Verification Matrix

Tier key: **U** unit over `ui::detail`'s pure rows, **V** view test into a `TestBackend`.
`DETAILWIDTHS` requires every `#[test]` in `src/ui/detail.rs` to name both `58` and `78` as
unsuffixed literals. Three of the rows below do not name both on their own — the wide-interior
scenario names 78, the narrow one 58, the sweep runs `0..=120`, and the shallower-tab one
asserts at 67/66/65 — so each adds both literals explicitly (tasks.md 1.4). The gate carries no
exemption list, and this change asks for none.

| Scenario | Capability | Tier | Collaborators | Command |
|---|---|---|---|---|
| A spec tab's bodies align under their headers at the wide interior | artifact-folds | U | `ui::markdown::lines` real | `cargo test --lib ui::detail` |
| The same tab draws its bodies at column zero at the narrow interior | artifact-folds | U | `ui::markdown::lines` real | `cargo test --lib ui::detail` |
| The indent is all-or-nothing across one render | artifact-folds | U | none | `cargo test --lib ui::detail` |
| A shallower tab indents at a narrower width | artifact-folds | U | none | `cargo test --lib ui::detail` |
| A depth-0 tracked-tasks tab is unmoved at every width | artifact-folds | U | `ui::tasks::items` real | `cargo test --lib ui::detail` |
| A depth-1 tracked-tasks tab indents its items like any other tab | artifact-folds | U | `ui::tasks::items` real | `cargo test --lib ui::detail` |
| A selection over an indented body row copies the indent | artifact-folds | U | none | `cargo test --lib ui::detail` |
| A body row is indented by its section's depth exactly when the floor allows | artifact-content | V | TestBackend | `cargo test --lib ui::view` |
| The sixteen scenarios carried unchanged by the artifact-folds MODIFIED block | artifact-folds | U + V | TestBackend | `cargo test --lib -- ui::detail ui::view` |

Every command above was run at HEAD and its selection counted, because a filter naming no test
exits clean: `cargo test --lib ui::detail` selects **63**, `cargo test --lib ui::view` selects
**156**, and `cargo test --lib -- ui::detail ui::view` selects **219**. Note the `--`: `cargo
test` takes one positional `TESTNAME`, so a second filter without it is rejected outright and
runs nothing.

The sixteen carried scenarios keep the tests they already have and are re-run as regression:
every one of them draws body rows, and this change moves body rows. One of them does **not**
stay green unchanged — `a_badged_header_row_is_still_addressed_by_its_own_section_index`
compares a depth-1 body row's text exactly (`r.text() == "Beta text."`), and at 78 that row
becomes `"  Beta text."`. Nor does `the_tracked_tasks_tab_concatenates_rather_than_folding`,
whose two-path tracked-tasks fixture puts its groups at depth 1 so its items gain `"  "` at 78.
Both were found by planting a minimal implementation and running `cargo test --lib` at planning
time — `1445 passed; 3 failed`, the third being `src/ui/view.rs`'s. Its assertion is updated to the indented text at 78 and the flush text
at 58; the scenario it proves is unchanged, only the fixture's expectation moves.

## Gates

`make check` is the single gate.

- **`DETAILWIDTHS`** — satisfied by adding both literals to each new test, as above; not by
  construction, which an earlier draft of this design wrongly claimed.
- **`COLWIDTH`** — the indent is measured through `ui::layout::columns`, never a `.chars()`
  count. `"  ".repeat(depth)` is ASCII and measures its own character count, but the
  *remaining* width arithmetic is column arithmetic and is written as such.
- **`NOIO-VIEW`** — `src/ui/detail.rs` stays in the ten-file pure set with no new API named.

Coverage stays at the 80% line floor with no exclusion added.
