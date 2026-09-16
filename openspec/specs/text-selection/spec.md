# text-selection Specification

## Purpose
Makes the detail region's content area drag-selectable, so a reader can get rendered text
back out of the pane. It owns the `Selection` value, the `Action::Select` phase pair that
produces it, the reverse-video painting of the selected span, and the OSC 52 write that
copies it on release.

It exists because the pane renders documents a reader wants to quote, and until now the
only way out was the terminal's own bypass — Shift+drag, which `notes/measurements.md`
proved works on Ghostty/macOS and which Option+drag was measured *not* to do. One terminal
family, one platform: too thin a thing to tell a reader to rely on, and it selects the
frame's painted cells rather than the artifact's own rows.

`Selection` is **one** field carrying its own granularity rather than a mode plus a span,
and it is a sibling of `detail` rather than living inside it, so that a reload must
*deliberately* clear it instead of silently discarding it — every site that invalidates the
rendered content (a scroll, a click, a fold, a tab switch, an adopted refresh) names the
clear. `Action::Select` carries the phase — a `Begin` for a press, an `Extend` for a drag —
rather than costing two variants, because every variant costs a `Mouse` row in
`ui::help::INVENTORY` and moves two other capabilities' pinned counts, where one row reading
"select text in the artifact area" is also what a reader needs. Granularity widens by
counting consecutive presses at one cell, never by a double-click timer, because nothing
under `src/ui/` may read a clock.

It fixes three things about how the selection behaves at its edges. The highlight
**composes** with whatever role already styles the cell rather than replacing it, so a
selected heading is still a heading. Selection does not inherit `detail_row_click`'s
`!foldable()` short-circuit — a single-section artifact is a whole rendered document, and is
exactly what a reader wants to copy. And a drag past the content area's edge **clamps**
rather than scrolling, because auto-scroll during a drag is a second motion model the pane
has no frame budget for.

The pane claims nothing about the clipboard. OSC 52 is a write into the terminal with no
reply, so a successful write is reported as nothing at all; only a *failed* write — the
terminal seam returning an error — becomes a `Selection::problem` row. `Action::Select` is
also the pane's one **mouse-only** action, the single exemption `mouse-input` has ever
granted to its rule that every gesture has a key above it: selecting a span is a pointing
gesture with no keyboard path, and getting text out of the pane was not a function of the
pane before this capability added it.

## Requirements

### Requirement: The frame is partitioned into clickable and selectable regions

The pane SHALL decide where a drag may begin **by region**, not by disambiguating a press
from a drag after the fact. `ui::layout::zone` already resolves any point in the frame to
exactly one `Zone`, and that resolution SHALL be the only thing consulted.

A left drag SHALL begin only on a `Zone::DetailRow` whose row is **not** a section header.
This SHALL hold whether or not the artifact is foldable: `detail_row_click` short-circuits on
`!detail.foldable()` and returns `Action::Ignore` today, and selection SHALL NOT inherit that
short-circuit — a single-section artifact is a whole rendered document, and is exactly what a
reader wants to copy out of.
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

#### Scenario: A single press with no motion selects nothing

- **WHEN** a left press lands on a non-header `Zone::DetailRow` and is followed by a release
  with no intervening drag event
- **THEN** no cell is highlighted and nothing is written to the clipboard
- **AND** the press is nonetheless recorded, so a second press at that cell can select the
  word — see the click requirement below

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

A selection is **completed** by a drag's finishing phase, by the second press that selects a
word, and by the third that widens to a row. A first press completes nothing. On completion the pane SHALL extract the selected
text from the `ContentRow` values
`ui::detail::content_lines` already returns, and SHALL pass it to `TerminalOps` for an OSC 52
write. No `ratatui::buffer::Buffer` SHALL be read back: the rows are already data, produced
by a pure function, and reading the rendered buffer would make the copy depend on the draw.

`src/ui/terminal.rs` SHALL remain the only file in the crate naming a terminal escape, and
the write SHALL reach it through `TerminalOps` rather than a second seam.

**The call site is an injected writer, named here because the render loop has none today.**
`ui::driver::run_loop` takes no `TerminalOps` handle — the guard is built in `ui::run` and
never travels inward — so the pane SHALL thread a `ClipboardWriter<'a> = &'a dyn Fn(&str) ->
Result<(), String>` into `run_loop` **beside `ArtifactReader`**, bound in `src/ui/mod.rs` to
the guard's `write_clipboard`. This is the crate's established shape for exactly this problem:
`read` is already injected that way, for the same reason, and a test drives both with a
closure. No view file SHALL name the writer.

**A failed write is reported on `Selection`, not on any existing problem list.** `Selection`
SHALL carry `problem: Option<String>`, set when the write returns `Err` and `None` otherwise.
Every existing `!`-marked list — `launch.problems`, `refresh.problems`, `changes.problems`,
`refresh.startup`, `agents.problem` — is replaced wholesale on its own producer's cadence, so
a reason put in one would vanish before the reader saw it; and `Dashboard` is pinned at
sixteen fields, so a dedicated field is not available. `Selection` is the honest home: it is
created and cleared at exactly the moments the reason is and stops being true, and it is
already this change's own type. The reason SHALL render as a **detail-region** problem row
beside `detail.problems`, because the gesture that produced it is a detail-region gesture.

**A single press copies nothing**, so clicking around the content area never disturbs the
system clipboard. Only a gesture that highlights something — a drag, a double press, a triple
press — writes to it, and the persisting highlight is what tells the reader what the
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

### Requirement: A press arms, a second selects the word, a third selects the row

On a non-header `Zone::DetailRow` the pane SHALL count **consecutive presses at the same
cell**:

| press at that cell | selection afterwards | highlighted | copied |
|---|---|---|---|
| first | the cell is recorded, nothing more | no | no |
| second | the **word** under it | yes | yes |
| third | the **whole rendered row** | yes | yes |
| fourth and after | unchanged from the third | yes | no |

A press at any **other** cell SHALL restart the count at one, recording that cell and
selecting nothing. A word is a maximal run of non-whitespace display columns in the rendered
row, so an identifier, a path, or a backticked span selects whole — `ui::layout::zone` is one
word, not three. A second press whose cell holds only whitespace SHALL select nothing and
SHALL leave the count at two, so a third press there still selects the row.

**This is deliberately not a timed double-click, and the behaviour is the same.** Crossterm
synthesises no double-click event, so detecting one means timing two presses, and `NOBLOCK`
forbids `src/ui/` from naming a clock at all — `Instant::now`, `SystemTime::now` and
`.elapsed()` are all swept. Counting consecutive presses at one cell produces the same
gestures with no clock, exactly as `list-selection`'s "A second click on the selected change
opens its detail" already fires on state rather than time, and its third click deliberately
does nothing. The accepted consequence is the one that rule already accepts: presses
separated by a long pause still count as consecutive.

`Selection` SHALL carry the granularity — armed, word, row, or span — and the failure
reason above — rather than `Dashboard`
carrying a further field. **Armed** is the state after a first press: an anchor and focus at
one cell, drawn as nothing. It is what lets a single field express "a press landed here and
selected nothing" without a second field to forget to clear. A drag SHALL set the granularity
to span directly, and a span SHALL NOT widen on a later press at its anchor — that press is a
first press, and arms.

#### Scenario: One press arms, two select a word, three select the row

- **WHEN** a dashboard receives a left press at one cell over the word `zone` in content row
  5, four times in a row
- **THEN** after the first, no cell is highlighted and nothing was copied
- **AND** after the second, exactly the columns of `zone` are highlighted and that text was
  copied
- **AND** after the third, every column of rendered row 5 is highlighted and the row's text
  was copied
- **AND** after the fourth, the buffer is identical to after the third and nothing further
  was copied

#### Scenario: A press elsewhere restarts the count

- **WHEN** two presses select a word, and the next press lands on a different cell
- **THEN** nothing is highlighted afterwards
- **AND** a second press at that new cell selects the word there, so the count restarted at
  one rather than continuing to three

#### Scenario: A click selects the whole token, not a fragment

- **WHEN** content row 5 renders as ``  the `ui::layout::zone` call`` and two presses land on
  any column inside `ui::layout::zone`
- **THEN** the selection covers exactly that run of non-whitespace columns
- **AND** the granularity is word

#### Scenario: A dragged span arms rather than widening

- **WHEN** a drag completes, leaving a span selection, and a press then lands on that span's
  anchor cell
- **THEN** nothing is highlighted and the granularity is armed
- **AND** a second press at that cell selects the word there, so widening never applies to a
  span the reader dragged
