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

`ui::list::truncate_left` — the keep-the-tail rule the no-repository block and
`responsive-layout`'s header share — SHALL likewise keep the longest **suffix ending on a
grapheme-cluster boundary** that measures at most `width - 1` columns, prefixed with `…`. It
SHALL NOT pad, exactly as it does not today; the two callers that owe a full-width row pad
the result themselves.

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
  `/home/dev/日本語のディレクトリ名前です/deeper` is rendered at 120x20 and at 60x20
- **THEN** in each buffer the three-row no-repository block is drawn, its shortened path row
  measures at most the interior width in columns — 38 and 58 — and begins with `…`
- **AND** neither buffer writes a cell past the list region's interior, and the block is
  still exactly three rows with no badge and no marker
