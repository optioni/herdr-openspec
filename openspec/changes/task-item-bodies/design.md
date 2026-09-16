## Context

`tasks::parse` is the *counter's* model doing the *renderer's* job. It keeps ATX headings and
checkbox bullets and discards everything else (`src/tasks.rs:279-291`), which is correct for
`count` — that rule copies the OpenSpec CLI byte-for-byte and must stay line-based — and wrong
for the tab that exists to show the file. Measured over this repository's 44 `tasks.md` files:
4,791 lines kept, **18,684 non-blank lines dropped**, 159 fenced blocks rendered as nothing,
and 1,924 of 2,842 items (67.7%) truncated at their first physical line.

This change gives `parse` a second output — retained content — without moving `count` at all.

## Goals / Non-Goals

**Goals.** Every non-blank line of a `tasks.md` reaches the screen exactly once. Fenced code,
tables, and block quotes render with the grammar the other tabs already use. Item text is
faced. The counting rule, and therefore every number in the pane, is provably unmoved.

**Non-Goals.** No second fold level (an item's body folds with its section). No change to
`count`, `task-checkboxes`, or CLI parity. No change to section-body indentation —
`artifact-folds` argues that separately and this change does not reopen it. No new dependency
and no wider parser option set.

## Decisions

**1. Two retention fields, not one.** `Item.body` alone does not cover the corpus and
`Group.blocks` alone does not either: measured, **47** fenced blocks open indented under an
item's bullet and **112** open at group level between items. A single field would have to pick
one of those to render wrongly. Two fields, each with an unambiguous owner, render both.

**1a. A leading group carrying blocks but no items is emitted.** The pre-retention rule
suppressed the headingless leading group unless it held an item, which was right when the only
thing a group could hold *was* items. Retention makes it wrong: measured, **27 of 44** corpus
files open with non-blank content above their first heading — **736** lines — and every one is
retained into a group that is then thrown away, so "Every retained line appears exactly once"
cannot hold. The group is emitted when it carries at least one item **or** at least one block.

This is the single shape in which grouping is not byte-identical to the pre-retention
grouping, and the cost is bounded and named: the extra group carries `Progress { 0, 0 }`, so
no count moves, and on the non-foldable path it contributes one more zero-width span to
`tasks-progress-bar`'s gauge segmentation. The foldable path — which is the one every real
`tasks.md` takes — builds its slice from `detail.sections`, not from `parse`, and is
unaffected.

**2. The continuation rule is indentation, and a checkbox always wins.** A line continues the
preceding item when it is blank or indented strictly past that item's own `indent`; the body
ends at the first heading, the first checkbox line at any indent, or the first non-blank line
at or below the item's indent. Trailing blanks are stripped.

A checkbox line is **never** body text. Two things force this. A nested sub-task is indented
past its parent and would otherwise be swallowed by the parent's body, contradicting
`task-groups`' standing requirement that nested items stay siblings. And a checkbox inside a
fenced block must stay an item, because `count` has no fence exemption — deliberately, copying
the CLI — so a fence that sheltered a checkbox from `parse` but not from `count` would put the
two entry points back into the disagreement the whole dual-source model exists to prevent.
Measured: **0** checkbox lines appear inside a fence in the archive today, so this rule costs
nothing now and is written down because the day it costs something is the day it matters.

**3. Item text goes through `inline`, bodies and blocks through `lines`.** An item's text is a
**fragment**; a body and a block are **documents**. Handing a fragment to the block parser
reinterprets it — `- [ ] # not a heading` loses its `#` and gains a heading face,
`- [ ] 1. first` becomes an ordered list — restyled, re-segmented or mis-faced rather than
*deleted*, which an earlier draft of this decision got wrong: measured, `lines` keeps the `#`
and sets a heading face, replaces `- ` with `• `, and returns **zero rows** for a leading
`` ``` ``. Measured, **0 of 2,842** item texts would be reinterpreted today, so this is a
structural argument and is labelled as one in the spec rather than dressed up as a live
defect. It earns its place on a second count: a single-paragraph
render returns one flat run list, which is what makes Decision 6's label lookup tractable.

**4. `inline` escapes the leading block marker rather than post-filtering events, and the
escape goes where the block opens — not at the front.** `pulldown-cmark` has no inline-only
entry point, and flattening block events after the fact loses the marker that opened the block
— by then `# foo` has already become `Text("foo")`. So `inline` inserts a CommonMark backslash
escape immediately before the character that would open the block, parses the result, and the
parser yields a paragraph whose text is the fragment verbatim.

**Two rules decide where the escape goes and whether it goes in at all**, and both were
measured against the pinned `pulldown-cmark` 0.13.4 rather than reasoned about:

- **Placement.** A backslash escapes only ASCII **punctuation**. Prepending one to a digit does
  not escape — it is emitted: `\1. first` parses to `Text("\\1. first")`, a **visible
  backslash**, while `1\. first` parses to `Text("1")` + `Text(". first")`, which concatenates
  to the fragment verbatim. So for an ordered-list start the escape goes **after** the digit
  run. Every item text in this repository's archive — **2,842 of 2,842** — opens with a digit
  run, so a prepended escape would put a stray `\` on every item row in the pane, and would
  shift `tasks::label_of`'s byte offsets by one so that Decision 6's label slice reads
  `" CHEC"` instead of `"CHECK:"`. That is a **wrong colour**, the one error direction
  `task-labels` forbids.
- **Trigger.** The escape fires only on a run that genuinely opens a **block**, which is
  narrower than "the first character is one of these": `#` ×1–6 followed by a space or the end
  of the input; `-`, `*`, or `+` **followed by a space**; `>`; a whole-line run of `-`, `=`,
  `_`, or `*`; a fence opening of three or more `` ` `` or `~`; and a digit run followed by `.`
  or `)` **and then a space or the end of the input**. A bare leading emphasis marker gets no
  escape at all, because `**RED**: …` and `*stressed* opening` are already paragraphs.

The narrow trigger is not tidiness — the wide one **destroys** the inline faces this function
exists to set. Measured: `\*stressed* opening` parses to three plain `Text` events with the
emphasis gone, and `\**RED**: write the failing test` parses to `Text("*")` + `Emphasis("RED")`
+ `Text("*")`, downgrading strong to emphasis and leaving a literal asterisk. That falsifies
`markdown-render`'s own "Inline constructs SHALL be recognised exactly as `lines` recognises
them inside a paragraph" and falsifies `tasks-checklist`'s scenario "An emphasised label
degrades to unlabelled rather than mis-coloured", whose second **THEN** requires the leading
segment to carry `face.strong`. Two archived items open with `**` today
(`archive/2026-09-10-foldable-spec-sections/tasks.md:219` and
`archive/2026-09-13-markdown-legibility/tasks.md:190`), so this is live, not hypothetical.

The **backtick fence** is the marker that most needs the escape and the one a
first-character-based set would most easily omit: `lines("```rust let x = 1;", 200)` returns a
**zero-element** vector — the fragment vanishes whole rather than degrading to literal text. An
item's `text` is already trimmed by `parse`, so a four-space indented-code opening cannot occur
and is not in the set.

The escape is asserted invisible: the scenario "A leading block marker is literal text, not a
block" compares against the marker character for character, and requires `lines` to differ on
at least four of the five inputs so it cannot pass vacuously. That comparison is over
**`Vec<Line>`**, not `Line::text()` — measured at HEAD, `lines` differs from the literal
fragment in **text** for only 2 of the 5 (`- ` becomes `• `, `> ` becomes `│ `), and in
segments-and-faces for 4 of the 5 (`# not a heading` keeps its `#` but gains
`heading: Some(1)`; `1. not an ordered list` splits into two segments; `--- not a rule` is
byte-identical and is the one that does not differ). Four is the floor with no margin, so the
comparison basis is part of the requirement rather than a detail of the test.

**5. `items` becomes `group_body` and takes a `&Group`.** Blocks interleave with items by
position, so one function must see both; a function still called `items` that draws fenced
blocks would be a name that lies. The rename moves two call sites — `ui::tasks::lines` and
`ui::detail::content_lines` — and `artifact-content`'s walk stops flattening the parse to an
item slice, which is the line that discarded every body and block.

**6. The label is applied to the leading plain segment, or not at all.** `tasks::label_of`
returns byte offsets into **plain** text; once inline spans fold, those offsets no longer
address the rendered row. The label is applied only when the first row's leading segment is
`Face::plain()` and long enough to hold `start + len` on character boundaries. An emphasised
label (`- [ ] **RED**: …`) has a `strong`-faced leading segment and degrades to unlabelled — a
**miss, never a wrong colour**, the error direction `task-labels` already chose. Measured:
2,531 plain labels in the archive, **0** emphasised. This is the same shape as the existing
"a label split across a wrap degrades to unlabelled" rule, not a new kind of concession.

**7. A checked item is muted whole, inline faces dropped.** Every row of a completed item — its
text rows and its body rows — is one `muted` segment, with code spans and emphasis discarded
rather than dimmed. This is the existing label rule extended, not a new policy: `tasks-emphasis`
already dropped the label's own role on a completed item rather than dimming it, for the reason
that a finished task is not where the eye belongs. Keeping the rule at "one segment per row"
also keeps the muted path as cheap to assert as it is today.

**8. `Line::text()` byte-identity is retired, deliberately.** `tasks-emphasis` guaranteed that
splitting rows into segments moved no character. Facing cannot preserve that — ``add the
`Recorder` arm`` renders without its backticks. The guarantee is given up rather than worked
around because keeping it *is* the defect: an item's text and its body are one sentence in the
source, and rendering the first row with literal backticks beside a body row with a styled code
span shows one sentence two ways. The spec says this in the requirement body so a later reader
finds the reasoning where the rule used to be.

**9. A body is dropped whole with the prefix it hangs from.** The existing grammar drops the
indent whole, then falls back to the glyph alone. A body hangs at `prefix_len`; where the prefix
has degraded to the glyph-only or truncated-glyph form there is no column for it, so the body
contributes no rows at all rather than one character per row.

**9a. The hanging indent falls after the task number.** A wrapped item's continuation rows and
its body rows hang at `prefix_len + tasks::task_number_len(&item.text)`. The number column then
stays clear down the whole item, which is what makes a group scannable by number. Measured,
**2,842 of 2,842** archived items carry a number — width 4 (2,168), 5 (663), 6 (11) — so the
hang is 8 columns for three items in four and never exceeds 10, costing at most 6 further
columns of the 58-column interior.

This departs from the source file's own convention, where a continuation line is indented to
six columns and lands *under* the number. That convention is right for the source, whose prefix
is `- [x] `; the rendered prefix is `[x] `, a different width, so reproducing the source column
would align with nothing on screen. The renderer aligns with what it draws.

The skip rule is **not copied**. `tasks::label_of` already skips the number to find where a
label starts; `task_number_len` exposes that same skip through one shared helper, on the terms
`specs::clause_of` calls `tasks::role_of` rather than restating its token table. The two are
bound by a corpus scenario asserting `Label.start == task_number_len(text)` for every labelled
item in the archive, so a divergent second copy could not pass.

The number hang is **dropped whole before the prefix is**: where `prefix_len + number_len`
leaves no text column the hang falls back to `prefix_len`, and only then does the existing
prefix chain (indent, then glyph-only, then truncated glyph) apply. So a width that can hold
the glyph and some text never loses the text to the number's indent.

**10. Blocks draw at column zero, separated from the rows around them by exactly one blank
row — and by none where there is no row to separate from.** A block belongs to the group, not
to the item above it, so it carries no hanging indent. The blank separators belong to the
block: a group with no blocks renders byte-identically to the group it was, blank rows included,
which is what keeps every existing `tasks-checklist` scenario passing unchanged.

The separator is emitted only where one is **missing**, which is one rule rather than three
special cases. Emitting unconditionally was measured to produce all three of: a leading blank
at the top of a section body, where nothing sits above the block at all — which falsifies
`artifact-folds`' amended "`Space` on a preamble row is inert", whose preamble row set is
exactly "the progress-bar row, its blank line, and the row `Intro prose.` itself now draws";
a **double** blank between two blocks sharing an `after`, each contributing one; and a double
blank after a fenced block, `ui::markdown::lines` already ending one with a blank of its own —
which is what made "A group's block renders between the items it sits between" render six rows
where it states five.

The rule is relative to the rows of the **group** being filled, which is what a section body
is. A caller appending a group's rows beneath a row of its own cannot be seen from there, so
where such a row needs separating the caller pushes it: `ui::tasks::lines` does exactly this
for the `No tasks yet` row, which is a status row rather than content and would otherwise read
as the retained prose's own first line.

**11. No second fold level.** Folding an item's body is the better end state — items carry
4.72 body lines each here, so a 12-item group grows from ~12 rows to ~68 — but the detail cursor
addresses sections, not items, so it is a `detail-scroll` change as well. Deferred, and the spec
says so: body rows carry `ContentKind::Body`, are never fold targets, and no item row gains a
glyph.

## Risks / Trade-offs

| Risk | Mitigation |
|---|---|
| Retention silently changes a count | "Retention leaves every count in the archive unmoved" asserts every file's pair against a **committed fixture** of the pre-change pairs, not against a second run of the same code |
| A line is claimed by both an item body and a group block, or by neither | "Every retained line appears exactly once" sweeps the whole archive and asserts a partition in both directions |
| The `inline` escape leaks into rendered text | Asserted character for character, with a `lines` comparison that must differ so the test cannot pass vacuously |
| Tab rows grow ~4.9x and the tab becomes unscannable | Accepted for now, and named in Decision 11 as the reason a second fold level is the next change rather than a nice-to-have. Decision 9a's number-column hang is the partial mitigation: more rows, but a column a reader can run an eye down |
| The number hang costs 4-6 more columns at the 58-column interior | Dropped whole before the prefix is, and asserted by "The number hang is dropped before the prefix is" across ten widths |

## Migration Plan

None. No persisted state, no config format, no manifest entry, and no keybinding changes; the
plugin's own writes stay exactly `agent-names.toml` under `HERDR_PLUGIN_STATE_DIR`.

## Modules touched

- `src/tasks.rs` — `parse`, plus `task_number_len` exposing the skip `label_of` already
  performs through one shared helper. `count`, `Progress`, `label_of`'s own behaviour, and
  `read` are untouched.
- `src/ui/markdown.rs` — the new `inline` entry point. The `pulldown-cmark` option set does not
  move, so `MDSEAM`'s subject does not move either.
- `src/ui/tasks.rs` — `items` → `group_body`; item text through `inline`; body and block rows.
- `src/ui/detail.rs` — the tracked-tasks branch of the section walk stops flattening the parse.

**No process spawn is added anywhere.** No file outside `src/cli.rs` gains a spawn API, and
none of the four modules above names one. **No view gains I/O**: all four remain pure functions
over `&str` and parsed values, and `NOIO-VIEW`'s ten-file pure set is unchanged in membership.

**The `Change` type does not move.** No field is added, removed, or retyped, so
`changes::from_files` and `changes::from_cli` need no reconciliation and `change-merge` is
untouched. This change is entirely downstream of `Change`, operating on the artifact text a
tab already reads.

## Test Boundaries

| Collaborator | Treatment | Why |
|---|---|---|
| Filesystem | **Real** in `tests/` corpus sweeps, which walk `openspec/changes/**/tasks.md`; **absent** in every `src/` unit test, which takes `&str` | The archive-wide partition and count-invariance claims are about real files; the grammar is not. The sweeps live in `tests/` precisely so no `src/ui/` file gains a read |
| `openspec` binary | **Not reached** | This change touches no CLI path; `count`/`from_cli` parity is asserted against the existing fixture, not by spawning |
| Herdr socket | **Not reached** | No agent, launch, or pane surface is touched |
| Artifact reader | **Replaced** by the injected `&dyn Fn(&Path) -> Result<String, String>` every `src/ui/` test already passes to `Dashboard::sync_detail`, never `ui::read_artifact` itself | Group 5's two view scenarios drive the section walk through a synced `Detail`, which reaches the reader; calling the real one would give a view test a filesystem and break `NOIO-VIEW` |
| Terminal | **Replaced** by `ratatui::backend::TestBackend` at 60 and 120 columns for every view scenario | `cargo test` spawns this binary; a real terminal would corrupt the developer's session |
| Clock | **Not reached** | No render-path timing changes; the `artifact-content` cache keyed on `(change directory, tab)` is untouched |

## Verification Matrix

Tier key: **U** unit over a pure module (`cargo test --lib`), **V** view test into a
`TestBackend` at 60 and 120 columns, **C** corpus sweep in `tests/`.

### `task-groups`

| Scenario | Tier | Collaborators | Command |
|---|---|---|---|
| A deeper heading closes the group above rather than nesting | U | none | `cargo test --lib tasks::` |
| Repeated and empty headings are all kept | U | none | `cargo test --lib tasks::` |
| Prose between items is dropped and does not split a group | U | none | `cargo test --lib tasks::` |
| Prose between items is retained as a block | U | none | `cargo test --lib tasks::` |
| A real change's task file counts the same both ways | U | `include_str!` corpus | `cargo test --lib tasks::` |
| Text with no heading still agrees | U | none | `cargo test --lib tasks::` |
| Retention leaves every count in the archive unmoved | C | real filesystem, committed pair fixture | `cargo test --test task_corpus` |
| A checkbox inside a fenced block is an item, not body text | U | none | `cargo test --lib tasks::` |
| An item's continuation lines are retained as its body | U | none | `cargo test --lib tasks::` |
| A nested sub-task is a sibling item and takes its own body | U | none | `cargo test --lib tasks::` |
| A fenced block indented under an item is that item's body | U | none | `cargo test --lib tasks::` |
| A dedented line ends the body and is not attributed to the item | U | none | `cargo test --lib tasks::` |
| An item with nothing after it carries an empty body | U | none | `cargo test --lib tasks::` |
| A group's lifecycle marker is retained as a block before its first item | U | none | `cargo test --lib tasks::` |
| A fenced block at group level is retained between the items it sits between | U | none | `cargo test --lib tasks::` |
| Every retained line appears exactly once | C | real filesystem | `cargo test --test task_corpus` |
| A group with no interstitial content carries no blocks | U | none | `cargo test --lib tasks::` |

### `task-labels`

| Scenario | Tier | Collaborators | Command |
|---|---|---|---|
| A numbered item reports its number's width | U | none | `cargo test --lib tasks::` |
| An unnumbered item reports zero | U | none | `cargo test --lib tasks::` |
| The exposed skip agrees with the one `label_of` performs | C | real filesystem | `cargo test --test task_corpus` |
| A malformed number is not a number | U | none | `cargo test --lib tasks::` |

### `markdown-render`

| Scenario | Tier | Collaborators | Command |
|---|---|---|---|
| A fragment's inline faces are set and its text is unchanged | U | none | `cargo test --lib ui::markdown` |
| A leading block marker is literal text, not a block | U | none | `cargo test --lib ui::markdown` |
| A fragment wraps and hard-splits exactly as a paragraph does | U | none | `cargo test --lib ui::markdown` |
| Empty, whitespace, and zero-width inputs return nothing | U | none | `cargo test --lib ui::markdown` |

### `tasks-checklist`

Every row runs at widths `78` and `58`, which `TASKWIDTHS` requires of every `#[test]` in
`src/ui/tasks.rs`.

| Scenario | Tier | Collaborators | Command |
|---|---|---|---|
| A folded group and an unfolded one render the same item lines | U | none | `cargo test --lib ui::tasks` |
| An item's body is drawn under it at its hanging indent | U | none | `cargo test --lib ui::tasks` |
| The hanging indent falls after the task number | U | none | `cargo test --lib ui::tasks` |
| The number hang is dropped before the prefix is | U | none | `cargo test --lib ui::tasks` |
| A fenced block in an item's body renders as code, not as vanished text | U | none | `cargo test --lib ui::tasks` |
| A group's block renders between the items it sits between | U | none | `cargo test --lib ui::tasks` |
| An item's inline markdown is faced rather than shown as markers | U | none | `cargo test --lib ui::tasks` |
| An emphasised label degrades to unlabelled rather than mis-coloured | U | none | `cargo test --lib ui::tasks` |
| A foldable tasks tab draws its groups as fold headers | V | TestBackend | `cargo test --lib ui::view` |
| Groups, headings, items, and separators at both mandated widths | U | none | `cargo test --lib ui::tasks` |
| A nested item reproduces its own indent | U | none | `cargo test --lib ui::tasks` |
| A long item wraps with a hanging indent at both widths | U | none | `cargo test --lib ui::tasks` |
| An unbreakable word is hard-split rather than lost | U | none | `cargo test --lib ui::tasks` |
| The indent is dropped whole as the width collapses | U | none | `cargo test --lib ui::tasks` |
| A body is dropped whole with the prefix it hangs from | U | none | `cargo test --lib ui::tasks` |
| A heading with no items still renders its heading | U | none | `cargo test --lib ui::tasks` |
| A headingless leading group renders without a heading line | U | none | `cargo test --lib ui::tasks` |
| A labelled unchecked item splits into three segments | U | none | `cargo test --lib ui::tasks` |
| A checked item is de-emphasised whole, label included | U | none | `cargo test --lib ui::tasks` |
| A wrapped labelled item labels only its first row | U | none | `cargo test --lib ui::tasks` |
| A label split across a wrap degrades to unlabelled | U | none | `cargo test --lib ui::tasks` |

### `artifact-folds`

Two scenarios are new and two are amended; the other **twenty-one** are carried unchanged by
the MODIFIED block and keep the tests they already have
(`cargo test --lib -- ui::detail ui::view`, 229 selected at HEAD), re-run as regression because
the tracked-tasks branch of the section walk moves beneath them. The live requirement holds 23
scenarios and the delta holds 25; `grep -c '^#### Scenario:'` over both is what produced those
figures.

| Scenario | Tier | Collaborators | Command |
|---|---|---|---|
| An open tracked-tasks section draws item bodies and group blocks | V | TestBackend | `cargo test --lib ui::view` |
| A depth-0 tracked-tasks tab is unmoved at every width (amended) | V | TestBackend | `cargo test --lib ui::view` |
| A depth-1 tracked-tasks tab indents its items (amended) | V | TestBackend | `cargo test --lib ui::view` |
| `Space` on a preamble row is inert (amended) | U | none | `cargo test --lib ui::app` |
| Collapsing a tracked-tasks section hides its bodies and blocks with its items | V | TestBackend | `cargo test --lib ui::view` |
| The twenty-one carried scenarios | U + V | TestBackend | `cargo test --lib -- ui::detail ui::view` |

## Gates

`make check` is the single gate. Three of its members have something specific to say here:

- **`TASKWIDTHS`** requires every `#[test]` in `src/ui/tasks.rs` to name both `58` and `78`.
  Every new body and block test is width-parameterised and satisfies it without an exemption.
- **`COLWIDTH`** sweeps `src/ui/tasks.rs` and `src/ui/markdown.rs` for `.chars()`-based
  measurement. The body's hanging indent is built from `prefix_len`, which is already a column
  count, and `inline`'s wrapping reaches `layout::columns` through the paragraph path it shares
  with `lines`.
- **`TASKSEAM`** forbids `ratatui` types and filesystem edges in `src/ui/tasks.rs`. Calling
  `ui::markdown::inline` and `ui::markdown::lines` trips neither leg; the parse leg still finds
  its `tasks::parse(` call.

Coverage stays at the 80% line floor with no exclusion added.
