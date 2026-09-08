## ADDED Requirements

### Requirement: Every cell of the row grammar is measured in display columns

Every field of every row this capability defines — the name field, an archived row's date
field, the badge cell, the progress cell, the `! `-prefixed problem row, the `Message` rows,
and the no-repository block's shortened `searched_from` — SHALL be laid out, padded, and
truncated in **display columns** as `responsive-layout` defines them, never in `char`s and
never in bytes. `ui::list` SHALL reach that measure only through `layout::columns` and
`layout::truncate_columns`.

`ui::list::pad_or_truncate_right(text, width)` — the crate's one right-truncation
implementation, shared with `ui::detail`'s header, tab bar, and problem-line grammar — SHALL
return a string measuring **exactly `width` display columns** in both of its arms:

- when `layout::columns(text) <= width`, `text` followed by `width - layout::columns(text)`
  spaces;
- when it is longer and `width` is at least 1, `layout::truncate_columns(text, width - 1)`,
  then `…`, then **as many further spaces as are needed to reach exactly `width`
  columns**. That trailing pad is new and load-bearing: `truncate_columns` drops a grapheme
  cluster whole, so the prefix measures `width - 1` or `width - 2`, and without the pad a row
  whose name ends in a wide cluster would be one column short of the interior and leave a
  stale cell behind it;
- when `width` is `0`, the empty string.

`ui::list::shorten_left` — the keep-the-tail rule the no-repository block and
`responsive-layout`'s header share — SHALL likewise keep the longest **suffix ending on a
grapheme-cluster boundary** that measures at most `width - 1` columns, prefixed with `…`. It
SHALL NOT pad, exactly as it does not today; the header right-aligns it within its own
remaining space. `ui::list::shorten_left_row`, the padded form the no-repository block's
third row uses, SHALL pad the result back to exactly `width` display columns — a third
measuring site, named here because it is easy to miss beside its un-padded sibling.

The cell-drop order is unchanged and is now evaluated in columns: the badge cell and its
space go first, then the progress cell and its space, then an archived row's date field and
its space, each when the name field would otherwise fall below **one column**. The progress
cell (`[<completed>/<total>]` or `[-]`) and the badge (one ASCII column) are ASCII by
construction and measure exactly their character counts; the date field is ten ASCII columns.
Only the name field and the message rows can carry non-ASCII content, and only they can
change width under this rule.

The two mandated interior widths SHALL remain **38** and **58**, and every row-grammar test
SHALL continue to assert both. What changes is the unit each assertion is stated in: a row's
length SHALL be asserted as `layout::columns(row.text) == width`, not as
`row.text.chars().count() == width`.

No row SHALL write past the interior's last column at any width, for any change name, in any
repository — including a name holding wide characters, emoji, combining marks, or a
zero-width-joiner sequence.

#### Scenario: A CJK change name stays inside the list region at both mandated widths

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo` and whose
  `changes.active` is the single change `日本語の変更名前です` — ten characters, twenty
  display columns — at 4 of 9 tasks, with no archived changes, no problems, an empty query,
  `selected` 0 and no agents, is rendered at 120x20 and at 60x20
- **THEN** in the 120-column buffer interior row 2 measures exactly 38 columns, spells the
  ten-character name from interior column 2, and ends with `[4/9]` in the interior's last
  five columns
- **AND** in the 120-column buffer the list block's right border at column 39, the detail
  block's left border at column 40, and every cell of the detail region's header row are
  exactly what the same dashboard renders with the ASCII name `add-token-refresh` — the
  overwrite the audit measured is gone
- **AND** in the 60-column buffer interior row 2 measures exactly 58 columns and its `[4/9]`
  cell ends in the interior's last column, with the frame's right border at column 59
  intact

#### Scenario: An emoji change name at 58 columns does not overwrite the border

- **WHEN** a `Dashboard` whose single active change is named `emoji-🎉-change` at 4 of 9 is
  rendered at 60x20 and at 120x20, once with no agent and once carrying an in-scope
  `Working` agent for it
- **THEN** in all four buffers the row measures exactly its interior width — 58 and 38 — the
  progress cell ends in the interior's last column, and the frame's border column is
  unchanged from the ASCII-named render
- **AND** in the badged renders the badge sits two columns left of the progress cell's first
  column, on exactly the landed rule, because the badge and progress cells are ASCII and
  their column budgets did not move

#### Scenario: A wide name is truncated whole and padded back to the full width

- **WHEN** `ui::list::rows` is called at widths 38 and 58 for a change named
  `日本語の変更名前です日本語の変更名前です日本語の変更名前です` — thirty characters, sixty
  display columns — at 4 of 9
- **THEN** at each width the row's `layout::columns` is exactly that width
- **AND** the name field ends with `…`, the character before the ellipsis is a whole CJK
  character rather than a cut one, and slicing the drawn name back out of the change's own
  name succeeds
- **AND** where the truncation landed one column short, the shortfall appears as a space
  between the ellipsis and the following separator rather than as a missing cell at the end
  of the row

#### Scenario: Rows are total over adversarial names at every width

- **WHEN** `ui::list::rows` is called at every width from `0` through `130` for a dashboard
  whose active changes are named, in order: a 200-column CJK string; a family emoji joined
  by two zero-width joiners; `e` followed by five combining accents; a lone U+FF9E halfwidth
  katakana sound mark; a name holding a NUL; and the empty string — each at 4 of 9, with an
  archived change carrying a date, and with one badged
- **THEN** no call panics at any width
- **AND** at every width every returned row's `layout::columns` is at most that width, and
  every non-`Separator` change row's is exactly that width
- **AND** at widths `0`, `1`, and `2` the rows are empty or a bare marker, and no row's text
  is longer than the width in columns

#### Scenario: The no-repository block shortens its search path by columns

- **WHEN** a `Dashboard` with no repository root whose `searched_from` is
  `/home/dev/workspaces/日本語のディレクトリ名前がとても長い場合の例` — forty-three
  characters and **sixty-five display columns**, chosen so that it exceeds the wider of the
  two interiors (58) and therefore shortens at both, which a shorter wide-character path
  would not — is rendered at 120x20 and at 60x20
- **THEN** in each buffer the three-row no-repository block is drawn, its shortened path row
  measures **exactly** the interior width in columns — 38 and 58 — and begins with `…`
- **AND** at each width the drawn suffix's own `layout::columns` is at most the interior less
  the ellipsis's one column, and slicing it back out of `searched_from` succeeds, so the
  keep-the-tail cut landed on a cluster boundary
- **AND** a `char`-counted shortening would have kept the last 37 and 57 **characters** —
  70 and 110 columns — so the two measures are distinguishable at both widths and the
  scenario discriminates between them
- **AND** neither buffer writes a cell past the list region's interior, and the block is
  still exactly three rows with no badge and no marker

## MODIFIED Requirements

### Requirement: Archived changes sit below a separator and carry their date

An archived change's row SHALL carry, after the marker and its following space, a
**ten-column date field** holding the `YYYY-MM-DD` string the archive directory name was
prefixed with, then one space, then the name field, the badge cell, and the progress cell
exactly as an active row has them. When the change's `Origin::Archived` carries `date: None`
the ten columns SHALL be spaces, so an undated entry's name still begins in the same column
as a dated one's.

An archived change is badged on exactly the same terms as an active one: a change can be
archived while an agent that was working on it is still alive, and `agent-attribution` builds
its `badges` from both tiers of the change list.

An archived row's fields SHALL be dropped **whole**, never cut short, in a fixed order as
the width falls: first the badge cell and its following space, when the name field would
otherwise fall below one column; then the progress cell and its following space, when the
name field would **still** fall below one column; then the ten-column date field and its
following space, on the same condition again; and only then does the row degenerate to the
active grammar's `[marker][space][name field]`, with no progress cell ever offered. Below two columns
the row is `> ` truncated to the first `width` **display columns**. Concretely, for an **unbadged** change named
`add-auth` at 7 of 7 dated `2026-08-14`: at width 20 the row is `> 2026-08-14 … [7/7]`; at 19
it is `> 2026-08-14 add-a…`, the progress cell dropped; at 14 it is `> 2026-08-14 …`; at 13
it is `> add-auth   `, the date dropped; at 3 it is `> …`; at 1 it is `>`; at 0 it is empty.
Every one of those rows SHALL be exactly its width in **display columns**, which for this
all-ASCII worked example is the same row it has always been. The **badged** form of the
same change needs two further columns: at width 22 the row is `> 2026-08-14 … b [7/7]`, and
at width 21 and below the badge is gone and every row above is reproduced unchanged.

The separator row SHALL be two spaces, then the literal `-- archived `, then `-`
characters filling the interior to its full width in **display columns**; when the interior
is narrower than the fourteen columns of `  -- archived ` it SHALL be that prefix truncated
to the width by `layout::truncate_columns` and padded back to exactly `width` columns.
The separator SHALL be emitted only when at least one archived row follows it, so a
repository with no archived changes — or a filter that matches none — shows no dangling
rule. A separator row is never badged.

#### Scenario: The separator and archived rows render at both mandated widths

- **WHEN** a `Dashboard` with one active change `fix-empty-basket` at 7 of 7, and archived
  changes `add-auth` dated `2026-08-14` at 7 of 7 followed by `legacy-cleanup` with
  `date: None` at 3 of 3, and no agents, is rendered at 120x20 and at 60x20
- **THEN** the 120-column buffer's interior rows read, in order at columns 1 through 38:
  `> fix-empty-basket               [7/7]`, then
  `  -- archived ------------------------`, then
  `  2026-08-14 add-auth            [7/7]`, then
  `             legacy-cleanup      [3/3]`
- **AND** the 60-column buffer's interior rows read, in order at columns 1 through 58:
  `> fix-empty-basket                                   [7/7]`, then
  `  -- archived --------------------------------------------`, then
  `  2026-08-14 add-auth                                [7/7]`, then
  `             legacy-cleanup                          [3/3]`
- **AND** in both buffers the undated row's name begins in the same interior column as the
  dated row's, so the absent date is ten spaces rather than a shift

#### Scenario: An archived change carries a badge in the same column as an active one

- **WHEN** the same dashboard is rendered at 120x20 and at 60x20 carrying one in-scope
  `Blocked` agent named `add-auth`
- **THEN** the 120-column buffer's archived `add-auth` row spells exactly
  `  2026-08-14 add-auth          b [7/7]`, and the `legacy-cleanup` row and the active
  `fix-empty-basket` row are byte-identical to the agentless rendering
- **AND** the 60-column buffer's `add-auth` row is exactly 58 characters with interior
  column 51 (0-based) holding `b` and interior columns 50 and 52 holding spaces — the same
  three columns a badged active row uses at that width
- **AND** the separator row is byte-identical at both widths, so no badge column was
  reserved on a row that cannot carry one

#### Scenario: An archived row drops the progress cell, then the date, as the width falls

- **WHEN** `ui::list::rows` is called for a single selected archived change `add-auth` at
  7 of 7 dated `2026-08-14` carrying a `Blocked` badge, at widths 22, 21, 20, 19, 14, 13, 3,
  1, and 0, and — as the contrasting controls at the mandated interiors — at 38 and 58
- **THEN** the rows are exactly `> 2026-08-14 … b [7/7]`, `> 2026-08-14 a… [7/7]`,
  `> 2026-08-14 … [7/7]`, `> 2026-08-14 add-a…`, `> 2026-08-14 …`, `> add-auth   `, `> …`,
  `>`, and the empty string, in that order
- **AND** the rows at widths 20 and below are character-for-character the rows the same
  widths produced for the same change before the badge existed, so the badge dropping first
  left the progress-cell and date-field boundaries exactly where they were
- **AND** each of those rows is exactly the requested width in characters, so no branch
  is off by one — every drop reclaims the separating space as well as the cell
- **AND** at 38 and 58 the row still carries the date field, the badge, and the `[7/7]`
  cell, so each drop is a width branch

#### Scenario: No archived changes means no separator

- **WHEN** the three-active-change dashboard above, whose `changes.archived` is empty, is
  rendered at 120x20 and at 60x20
- **THEN** the string `-- archived` appears in no row of either buffer
- **AND** adding a single archived change to the same dashboard and rendering again at both
  widths makes it appear, so the absence is the emission rule and not the string being
  unrenderable

### Requirement: The list region names every empty and degraded body state

When `dashboard.repo` is `None` the list region's interior SHALL hold exactly three rows
and no change rows: `No OpenSpec repository found`, then `searched from:`, then
`dashboard.searched_from`'s display path shortened to the interior width by the same
keep-the-tail rule the header uses — whole when it fits, otherwise `…` followed by the
longest suffix ending on a grapheme-cluster boundary that measures at most *width − 1*
**display columns**, then padded back to exactly `width` columns. This is `SPEC.md` →
Degraded states, row "No `openspec/` found while walking up".

The two functions that implement that rule are `ui::list::shorten_left` — un-padded, shared
with `responsive-layout`'s header, which right-aligns it within its own remaining space — and
`ui::list::shorten_left_row`, which is `shorten_left` padded to the full row. Both SHALL
measure and shorten in display columns.

When `dashboard.repo` is `Some`, the interior SHALL hold:

- exactly one `Message` row reading `No changes yet` when no change is visible and the
  filter query is empty;
- exactly one `Message` row reading `No changes match` followed by one row holding `/` and
  the query when no change is visible and the query is non-empty;
- one `Message` row reading `No active changes` in place of the active rows when no active
  change is visible but at least one archived change is, followed by the separator and the
  archived rows — `SPEC.md` → Degraded states, row "No active changes: empty state;
  archived changes remain browsable".

Every entry of `dashboard.changes.problems` SHALL be rendered as a leading row: `!`, a
space, then the problem text, truncated with `…` to the interior width. This is where the
degraded-states rows that record a reason on `ChangeSet::problems` — an unreadable
`openspec/changes/` or `archive/` — become visible; nothing else in the plugin renders
them.

A `Problem` row's text field is the interior width less its two-column `! ` prefix; a
`Message` row has no prefix and its field is the whole width. Both SHALL be padded with
spaces when short and truncated from the right with `…` when long, by the same rule the
name field uses, and both SHALL follow the same drop-whole rule the change rows do: below
three columns a `Problem` row is `! ` truncated to the first `width` **display columns**, and
at every width — including `0`, `1`, and `2` — every row of every kind, `Separator`,
`Problem`, and `Message` included, SHALL be **exactly** `width` **display columns**.

The indexing guarantee that sentence used to carry is narrowed here, deliberately, because
display columns are the honest unit and a `char` index is not one: a caller may take the
first `n` **columns** of a row with `layout::truncate_columns` for any `n <= width` and get a
whole prefix back, but SHALL NOT index a row by `char` or by byte and assume `width` of them
exist. For an all-ASCII row — every row this repository produces today — the two are the same
count, which is why no landed assertion moves.

The no-repository block SHALL replace **every** other row, problem rows included. That is
not a conflict to resolve at render time: `ui::load` produces `changes::empty_set()`
whenever `find_repo` reports `NotFound`, so a `Dashboard` with `repo: None` and a
non-empty `changes.problems` is not a value the composition root can build. Stating the
precedence anyway keeps `rows` total over every `Dashboard` value a test can construct.

`ui::view::render` SHALL still draw a bordered `Changes` region in every one of these
states. No state SHALL replace the frame with an error screen.

#### Scenario: No repository names the directory searched, at both widths

- **WHEN** a `Dashboard` whose `repo` is `None`, whose `searched_from` is
  `/home/dev/workspaces/openspec-demos/a-rather-long-repository-name-here` — seventy
  characters — and whose `changes` is `changes::empty_set()`, is rendered at 120x20 and at
  60x20
- **THEN** the 120-column buffer's interior rows 2, 3, and 4 at columns 1 through 38 begin
  `No OpenSpec repository found`, `searched from:`, and
  `…os/a-rather-long-repository-name-here` respectively
- **AND** the 60-column buffer's interior rows 2, 3, and 4 at columns 1 through 58 begin
  `No OpenSpec repository found`, `searched from:`, and
  `…kspaces/openspec-demos/a-rather-long-repository-name-here`
- **AND** in both buffers row 1 still holds `┌` at the region's first column and the title
  `Changes`, so the empty state renders inside the frame rather than replacing it

#### Scenario: A repository with no changes at all

- **WHEN** a `Dashboard` whose `repo` is `Some("/tmp/demo-repo")`, whose `changes` is
  `changes::empty_set()`, and whose filter query is empty is rendered at 120x20 and at
  60x20
- **THEN** in both buffers the first interior row begins `No changes yet`
- **AND** neither buffer contains `-- archived`, `No active changes`, or `No changes match`

#### Scenario: No active changes with archived ones still browsable

- **WHEN** a `Dashboard` whose `changes.active` is empty and whose `changes.archived` holds
  `add-auth` dated `2026-08-14` at 7 of 7 is rendered at 120x20 and at 60x20
- **THEN** in both buffers the first interior row begins `No active changes`, the second is
  the archived separator, and the third is the `add-auth` row
- **AND** neither buffer contains `No changes yet`, so the two empty states are
  distinguished rather than sharing one message

#### Scenario: Repository-level problems are named above the rows

- **WHEN** a `Dashboard` whose `changes.problems` holds the single entry
  `openspec/changes: Permission denied (os error 13)` and whose `changes.active` holds
  `fix-empty-basket` at 7 of 7 is rendered at 120x20 and at 60x20
- **THEN** in the 120-column buffer interior row 2 at columns 1 through 38 spells exactly
  `! openspec/changes: Permission denied…` — the two-character `! ` prefix, then the
  problem text's first thirty-five characters, then `…` — and interior row 3 is the
  `fix-empty-basket` row
- **AND** in the 60-column buffer interior row 2 at columns 1 through 58 spells
  `! openspec/changes: Permission denied (os error 13)` followed by spaces, with no `…`, so
  the truncation at 38 columns is a width branch
- **AND** in both buffers the region is still the bordered `Changes` block, not an error
  screen
