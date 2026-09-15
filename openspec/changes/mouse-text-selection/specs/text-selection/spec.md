## ADDED Requirements

### Requirement: The frame is partitioned into clickable and selectable regions

The pane SHALL decide where a drag may begin **by region**, not by disambiguating a press
from a drag after the fact. `ui::layout::zone` already resolves any point in the frame to
exactly one `Zone`, and that resolution SHALL be the only thing consulted.

A left drag SHALL begin only on a `Zone::DetailRow` whose row is **not** a section header.
In every other zone — `Zone::ListRow`, `Zone::DetailTab`, a `Zone::DetailRow` that is a
section header, `Zone::List`, `Zone::Detail`, and `Zone::Outside` — a left press SHALL
produce exactly the action it produces today, and a left drag SHALL produce
`Action::Ignore`.

Because the selectable region and the clickable regions do not overlap, **no existing
binding changes its dispatch timing**: a click still acts on press, and nothing waits for a
release to learn whether it was a drag.

#### Scenario: A drag begins only in the detail content area

- **WHEN** `ui::driver::mouse_action` is called with `MouseEventKind::Drag(MouseButton::Left)`
  at a point in each of the six zones, on a frame at 120x20 at `Route::Detail` showing a
  foldable artifact
- **THEN** only the point resolving to a non-header `Zone::DetailRow` produces a selection
  action
- **AND** every other zone produces `Action::Ignore`
- **AND** the same six points with `MouseEventKind::Down(MouseButton::Left)` produce exactly
  the actions they produce before this change, byte for byte

#### Scenario: A section header stays clickable and is never selectable

- **WHEN** a left press, and then a left drag, land on a `Zone::DetailRow` that
  `ui::detail::section_at` reports is a section header
- **THEN** the press produces `Action::Click(Target::DetailHeader { .. })`, unchanged
- **AND** the drag produces `Action::Ignore`, so folding a section can never begin a
  selection

### Requirement: A drag selects a span of the rendered artifact

`Dashboard` SHALL carry the selection as a single field holding an anchor and a focus, each
a line index into `ui::detail::content_lines` and a display column. The anchor SHALL be set
where the drag begins and SHALL NOT move; the focus SHALL follow the pointer.

`Action::Select` SHALL carry the phase — begin, extend, or finish — so that the pane adds
**one** `Action` variant rather than three. A variant costs a `Mouse` row in
`ui::help::INVENTORY`, and every such row moves `binding-inventory`'s pinned counts and
`help-overlay`'s row arithmetic; one row reading "select text in the artifact area" is also
what a reader needs, where three phase rows would describe an implementation.

The focus SHALL be **clamped to the content area**. A drag that leaves the area vertically
or horizontally SHALL extend the selection to that edge and no further, and SHALL NOT
scroll the content: a selection covers what is drawn.

#### Scenario: Anchor holds while the focus follows

- **WHEN** a drag begins at content line 4 column 10, extends to line 6 column 2, and then
  back to line 5 column 30
- **THEN** the anchor is line 4 column 10 throughout
- **AND** the focus is line 6 column 2, then line 5 column 30
- **AND** the selected span is computed from the pair in either order, so dragging upward
  selects the same text as dragging downward across the same two points

#### Scenario: A drag past the edge clamps and does not scroll

- **WHEN** a drag begins inside the content area and extends to a row above the area's first
  row, and separately to a row below its last
- **THEN** the focus is the area's first and last content line respectively
- **AND** `detail.scroll` is unchanged in both cases, so the content did not move
- **AND** no line outside the drawn window is included in the span

#### Scenario: A press with no motion selects the word under it

- **WHEN** a left press lands on a non-header `Zone::DetailRow` over the text `content_lines`
  and is followed by a release with no intervening drag event
- **THEN** the word under the press is selected, per the click requirement below
- **AND** a press landing on whitespace leaves no selection

### Requirement: The selection is painted from a palette role

Selected cells SHALL be drawn with `palette::style(Role::Selected)` and SHALL NOT have a
style written at the render call site. The role SHALL be distinguishable from the selected
list row's own role, because a reader can have a selected change and a text selection on
screen at once and they mean unrelated things.

The highlight SHALL survive the release that completes the drag, and SHALL be cleared by the
next selection, any click, any key that changes the detail content, a tab switch, a scroll,
or an adopted refresh that reloads the content. Persisting it is the pane's only honest
feedback about what was copied — see the clipboard requirement below.

#### Scenario: The span is highlighted at both mandated widths

- **WHEN** a dashboard with a selection from line 2 column 4 to line 3 column 8 is rendered
  at 120x20 and at 60x20
- **THEN** every cell inside the span reports `palette::style(Role::Selected)`
- **AND** no cell outside the span differs from the same frame rendered with no selection
- **AND** `palette::style(Role::Selected)` is not equal to `palette::style(Role::SelectedRow)`

#### Scenario: The highlight persists after release and clears on the next interaction

- **WHEN** a drag completes and the frame is redrawn with no further input
- **THEN** the span is still highlighted
- **AND** applying any of a tab switch, a scroll, a click, or an adopted refresh that reloads
  the detail content leaves no selection and no highlighted cell

### Requirement: The selected text is copied through the terminal seam

A selection is **completed** by a drag's finishing phase, by a click that selects a word, and
by a second click that widens to a line. On completion the pane SHALL extract the selected
text from the `ContentRow` values
`ui::detail::content_lines` already returns, and SHALL pass it to `TerminalOps` for an OSC 52
write. No `ratatui::buffer::Buffer` SHALL be read back: the rows are already data, produced
by a pure function, and reading the rendered buffer would make the copy depend on the draw.

`src/ui/terminal.rs` SHALL remain the only file in the crate naming a terminal escape. The
write SHALL go through the existing injected `TerminalOps` seam rather than a second one.

**Every completed selection copies, including one made by a single click.** The accepted
cost is that clicking around the content area overwrites the system clipboard, and it is
accepted because the region has no other click meaning — `Target::DetailLine`, which moved
the detail cursor, is what this capability replaces — and because requiring a second key to
copy would defeat the point. The persisting highlight is what tells the reader what the
clipboard now holds.

A multi-line selection SHALL join its lines with a single `\n` and SHALL NOT carry the
padding a rendered row ends with: what is copied is the text, not the cells.

#### Scenario: A single-line selection copies exactly the selected columns

- **WHEN** content line 3 renders as `  - **WHEN** the reader presses q` and a selection runs
  from column 4 to column 12
- **THEN** the text handed to the seam is exactly the display columns 4 through 12 of that
  line
- **AND** no trailing padding is included

#### Scenario: A multi-line selection joins with newlines and drops padding

- **WHEN** a selection spans three content lines of differing rendered length
- **THEN** the text handed to the seam holds exactly two `\n` characters
- **AND** no line in it ends with the spaces the rendered row was padded to

#### Scenario: The clipboard write cannot be confirmed, and the pane claims nothing

- **WHEN** the seam's clipboard write returns `Ok` on a terminal that silently ignores OSC 52
- **THEN** the pane renders no confirmation message and no problem row, because it has no
  evidence either way
- **AND** the highlight remains, which is the only claim the pane makes: this is what was
  selected, not this is what reached your clipboard
- **AND** when the seam returns `Err`, the reason is recorded and rendered as a `!`-marked
  row, because that failure **is** observable

### Requirement: A click selects a word and a second click widens it to the line

A left press on a non-header `Zone::DetailRow` SHALL select the **word** under the pointer.
A word is a maximal run of non-whitespace display columns in that rendered row, so an
identifier, a path, or a backticked span selects whole — `ui::layout::zone` is one word, not
three.

A second press **at the same cell**, while that word is the current selection, SHALL widen
the selection to the whole rendered line. A third press at the same cell SHALL change
nothing. A press at any other cell SHALL start again, selecting the word there.

**This is deliberately not a timed double-click.** Crossterm synthesises no double-click
event, so detecting one means timing two presses, and `NOBLOCK` forbids `src/ui/` from naming
a clock at all — `Instant::now`, `SystemTime::now`, and `.elapsed()` are all swept. The
widening is therefore driven by **state**, exactly as `list-selection`'s "A second click on
the selected change opens its detail" already is: that rule fires when "`selected` already
addresses that target", with no clock, and its third click deliberately does nothing. The
accepted consequence is the same one that rule already accepts — a *slow* second press at the
same cell still widens.

`Selection` SHALL carry the granularity — word, line, or span — rather than `Dashboard`
carrying a further field, because the granularity is meaningless without the selection it
describes and every read of one reads the other. A drag SHALL set it to span, and a span
SHALL NOT widen on a later press at its anchor: widening belongs to a selection the reader
made with a single press.

A press on whitespace SHALL clear any existing selection and select nothing, so there is a
way to dismiss a highlight with the mouse alone.

#### Scenario: A click selects the whole token under it

- **WHEN** content line 5 renders as `  the `ui::layout::zone` call` and a press lands on any
  column inside `ui::layout::zone`
- **THEN** the selection covers exactly that run of non-whitespace columns
- **AND** the granularity is word
- **AND** a press on the space before it leaves no selection

#### Scenario: The second click widens and the third does nothing

- **WHEN** a dashboard receives a press at one cell three times in a row, over a word
- **THEN** after the first the selection is that word
- **AND** after the second it is the whole rendered line, with granularity line
- **AND** after the third nothing has changed since the second, mirroring
  `list-selection`'s own third-click rule

#### Scenario: A press elsewhere restarts rather than widening

- **WHEN** a word is selected and the next press lands on a different cell holding another
  word
- **THEN** the selection is that second word at granularity word
- **AND** the first word is no longer highlighted

#### Scenario: A dragged span does not widen

- **WHEN** a drag completes, leaving a span selection, and a press then lands on that span's
  anchor cell
- **THEN** the selection becomes the word at that cell rather than the whole line
- **AND** the granularity is word, so widening never applies to a span the reader dragged
