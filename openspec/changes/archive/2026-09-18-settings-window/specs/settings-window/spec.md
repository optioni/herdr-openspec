## ADDED Requirements

### Requirement: `,` opens and closes a settings overlay that is the crate's second panel

The pane SHALL bind `,` (`KeyCode::Char(',')`, `KeyModifiers::NONE`) to a new
`Action::ToggleSettings`, which opens the settings overlay when it is closed and closes it
when it is open. `,` SHALL act at **both** routes, `List` and `Detail`, on exactly `?`'s
terms.

The settings overlay SHALL be a **panel of the existing overlay layer**, not a second
mechanism. `help-overlay` established the layer — a band over the body, a dispatch that
takes precedence over the route and filter dispatches, and `Esc` closing it first — and this
change SHALL generalise that one layer to carry either panel rather than add a parallel one.
`Dashboard::route` SHALL remain an enum of exactly `List` and `Detail`, and opening either
panel SHALL NOT change it.

The two panels SHALL be **mutually exclusive by construction**, not by convention: the layer
carries `panel: Option<Panel>` where `Panel` is `Help` or `Settings`, so there is no
representable state in which both are open. `ToggleHelp` while the settings panel is open
SHALL swap the panel to `Help`, and `ToggleSettings` while the help panel is open SHALL swap
it to `Settings`; swapping SHALL reset the layer's scroll to `0`, because the two panels
have unrelated row counts and carrying a position between them would land the reader at an
arbitrary row.

While `filter.active` is set, `,` SHALL type into the query like every other printable
character and SHALL NOT open the overlay, on exactly `?`'s terms: `list-filtering`'s rule is
that only `Ctrl-C` keeps a command meaning inside the filter, and a change name may contain
a comma.

`,` is chosen over a letter because every unshifted letter that reads as "settings" is taken
or adjacent to one that is: `s` launches an archive agent and `S` is one slipped shift away
from it, which would start a process instead of opening a window.

Opening the panel SHALL send a non-blocking `launch::Request::Resolve` to the launcher's
worker, so the agent kind resolves without the render path ever calling `integration status`.
The panel SHALL draw on the very next frame whether or not the worker has answered: until it
does, `agent_kind` reads `Provenance::Pending` and is not editable, while `openspec_bin` and
`prompts` are already complete (`setting-provenance`). `ToggleSettings` SHALL NOT wait for the
answer. The request SHALL be sent only when the key **opens** the panel; closing it sends
nothing, and neither does swapping to the help panel.

#### Scenario: `,` toggles the panel and its near misses do not

- **WHEN** `action_for` is called with `filtering` false and Presses of `Char(',')` with
  `KeyModifiers::NONE`, `Char(',')` with `SHIFT`, `Char(',')` with `CONTROL`, `Char('<')`
  with `SHIFT`, and a `Release` and a `Repeat` of `Char(',')`
- **THEN** the first returns `ToggleSettings` and the other five all return `Ignore` — a
  terminal reporting releases cannot toggle twice, and `<` is not `,`
- **AND** with `filtering` true, `Char(',')` with `NONE` returns `FilterPush(',')`, so the
  key types into the query and the overlay does not open
- **AND** every other key's mapping is unchanged under both modes, so `,` gaining a meaning
  moved no existing key

#### Scenario: The panels swap rather than stacking

- **WHEN** a `Dashboard` at `Route::Detail` with the help panel open and `overlay.scroll` `5`
  is given `ToggleSettings`
- **THEN** `overlay.panel` is `Some(Panel::Settings)` and `overlay.scroll` is `0`
- **AND** given `ToggleHelp` next, `overlay.panel` is `Some(Panel::Help)` and
  `overlay.scroll` is `0` again
- **AND** given `ToggleSettings` then `ToggleSettings`, `overlay.panel` is `None`
- **AND** `route` is `Detail` throughout, and `changes`, `filter`, `quit`, `refresh`,
  `agents`, `agent_names`, `launch`, `selected`, `detail`, and `sections` are unchanged
  field for field after every one of them

#### Scenario: `Esc` closes the settings panel before any other layer

- **WHEN** a `Dashboard` at `Route::Detail` whose `filter.query` is `add`, whose
  `filter.active` is true, and whose `overlay.panel` is `Some(Panel::Settings)` with no edit
  in progress is given four consecutive `Back` actions
- **THEN** after the first, `overlay.panel` is `None` and `filter.active` is **still true**
  with the query still `add`
- **AND** after the second, `filter.active` is false; after the third, `route` is `List`;
  after the fourth, nothing has changed
- **AND** `quit` is false after all four

### Requirement: The panel renders three settings, each as a value row and a source row

`ui::settings::render(frame, band, dashboard)` SHALL draw the settings panel, and it SHALL
be a pure function of the dashboard state it is given: no filesystem, process, environment,
network, or standard-I/O API, no clock, and no panic for any input, on exactly `ui::help`'s
terms.

The panel SHALL render exactly the three settings `setting-provenance` produces, in this
fixed order — `openspec_bin`, `agent_kind`, `prompts` — each as **two** rows:

- a **value row**: the setting's key, its effective value, and, for an editable setting, the
  affordance `<` and `>` bracketing the value — two **ASCII** characters, chosen so the panel
  adds no glyph to `SPEC.md`'s recorded East Asian Ambiguous set, and so the rule below that
  `─` is the only *chrome* glyph stays true (a bracket around a value is content, not chrome);
- a **source row**, indented beneath it: the human-readable name of the precedence level
  that produced the value, and `read-only` for a setting this change does not make editable.

The panel SHALL NOT derive any of this. It SHALL render the `Setting` values
`setting-provenance` hands it — held on `Dashboard::settings.rows` and recomputed outside the
render path — which is what keeps the view pure while the precedence that produced them lives
in `src/settings.rs`.

Every row SHALL be padded or truncated to the band's own width through
`ui::layout::truncate_columns`, which SHALL remain the crate's only display-width measure;
`src/ui/settings.rs` SHALL use it for the key column and for every truncation.

`archived_count` SHALL NOT appear. `list-sections` made the key **accepted and inert** —
nothing consumes it, `change-enumeration` no longer truncates the archived tier, and the
archived section's fold is what decides how much of the archive is shown. A settings panel
exists to answer "what is deciding this", and a row for a value that decides nothing answers
a question the reader did not have while inviting an edit that would change nothing on
screen. The key stays readable in `config.toml`, documented there as inert, which is where a
reader who has set it will look.

The band SHALL carry a heading row reading `Settings`, a top rule row, the six setting
rows, and a bottom rule row, on `ui::help`'s band grammar. `─` SHALL be the only chrome
glyph it draws, for the reason `help-overlay` already argued: the box-drawing corners and
both arrow glyphs are East Asian **Ambiguous**, and the band spends none of them.

#### Scenario: The panel renders every setting at both mandated widths

- **WHEN** a `Dashboard` whose config sets `archived_count = 9` and no other key, whose
  `agent_kind` resolved to `codex` from a sole installed integration, and whose
  `openspec_bin` resolved at probe step 2, is rendered with the settings panel open at
  120x40 and at 60x20
- **THEN** both frames hold a value row for each of `openspec_bin`, `agent_kind`, and
  `prompts`, in that order
- **AND** each value row is followed by a source row naming, respectively, the probe step,
  the sole installed integration, and the default
- **AND** the `openspec_bin` and `prompts` source rows both carry `read-only` at both widths
- **AND** no row of either frame names `archived_count`, although the fixture sets it

#### Scenario: The panel draws before the kind has resolved

- **WHEN** `,` is pressed on a freshly started pane at which no launch key has yet been
  pressed, and the frame is drawn at 120x40 and 60x20 before the launcher's worker answers
- **THEN** the panel is drawn at both widths on the first frame after the keypress
- **AND** the `agent_kind` source row reads the `Pending` label and its value row says the
  kind is still resolving
- **AND** the `openspec_bin` and `prompts` rows are already complete, with their real
  provenance
- **AND** a `launch::Request::Resolve` was sent exactly once, and `integration status` was not
  called from the render thread
- **AND** when the worker's answer is adopted, the same frame redraws with `agent_kind`
  carrying its resolved value and provenance, and the row cursor has not moved
- **AND** every row is exactly the band's width, and the band's width equals the body's at
  both, so the panel spans both regions and the divider column at 120

#### Scenario: A long path is truncated rather than wrapped or overflowing

- **WHEN** the panel is rendered at 60x20 with an `openspec_bin` whose resolved path is 200
  characters long
- **THEN** the `openspec_bin` value row is exactly 60 columns and no row of the frame beneath
  the band shows through
- **AND** no row of the panel exceeds the band's width, measured by
  `ui::layout::columns`, so a multi-byte path cannot push a row past the edge

#### Scenario: The panel degrades rather than panicking at any frame size

- **WHEN** a dashboard with the settings panel open is rendered at 1x1, 60x1, 60x2, 120x3,
  and 0x0
- **THEN** no render panics, and at each size the band is the rectangle
  `layout::overlay_band` returns for that body
- **AND** a zero-height band draws nothing at all
- **AND** the reader is never trapped: `Back` and `Quit` are answered at every one of those
  sizes, so a frame too small to draw the panel is still a frame the reader can leave

### Requirement: The settings panel answers nine actions and every other one is inert

While `overlay.panel` is `Some(Panel::Settings)`, `Dashboard::apply` SHALL dispatch as follows,
and this dispatch SHALL take precedence over the route dispatch and the filter dispatch alike,
on exactly the help panel's terms:

| Action | Effect while the settings panel is open |
|---|---|
| `Quit` | quit, exactly as when it is closed |
| `ToggleSettings` | close the panel, clearing any edit and resetting `overlay.scroll` to `0` |
| `ToggleHelp` | swap the panel to `Help`, clearing any edit and resetting `overlay.scroll` to `0` |
| `Back` | cancel the edit in progress; else close the panel |
| `OpenDetail` | begin the edit on the cursor's setting; else commit the edit in progress |
| `Next`, `ScrollDown` | step the candidate while editing; else move the row cursor down |
| `Prev`, `ScrollUp` | step the candidate while editing; else move the row cursor up |
| `Click(Target::Setting(i))` | move the row cursor to `i` and nothing else |
| every other action | change nothing at all |

**Nine** answer — `Quit`, `ToggleSettings`, `ToggleHelp`, `Back`, `OpenDetail`, `Next`, `Prev`,
`ScrollDown`, `ScrollUp` — with `Click` answered only for the `Target::Setting` payload. The
closed remainder SHALL be inert: `SelectTab`, `NextTab`, `PrevTab`, `FilterStart`,
`FilterPush`, `FilterPop`, `Refresh`, `LaunchApply`, `LaunchContinue`, `LaunchArchive`,
`FocusAgent`, `ToggleSection`, `SelectNext`, `SelectPrev`, `Select`, `Click` with any other
target, and `Ignore`.

`LaunchApply`, `LaunchContinue`, `LaunchArchive`, and `FocusAgent` being inert is load-bearing
here for a sharper reason than it is in the help panel. This panel is the one
`agent-launch` **opens on an ambiguous refusal**, so the reader arrives at it having just
pressed `a`. If `a` still launched from inside it, the key that could not choose a client would
start an agent the moment the reader pressed it again while reading the very row that explains
why it could not. `Refresh` is inert on the same argument one step down.

`Quit` is the one exception, on exactly the help panel's terms: a modal that traps the reader
is a worse failure than one that lets a quit through.

The blanket rule `apply` runs after every action — setting `refresh.requested` when
`Dashboard::needs_archived_refresh()` holds — SHALL continue to run unchanged, as it does
under the help panel.

#### Scenario: The agent keys launch nothing from inside the settings panel

- **WHEN** a `Dashboard` with `agents.reachable` true, a selected change `add-auth`, and the
  settings panel open is given `LaunchApply`, `LaunchContinue`, `LaunchArchive`, `FocusAgent`,
  and `Refresh` in turn
- **THEN** `launch.pending` is `None` and `launch.problems` is unchanged after all five
- **AND** `refresh.requested` is false after all five, unless `needs_archived_refresh()` holds
- **AND** the dashboard but for that one flag is equal, field for field, to the one before
- **AND** the same holds when the panel was opened by an ambiguous refusal, which is the case
  this rule exists for: the reader pressed `a`, got the panel, and pressing `a` again inside it
  starts nothing

#### Scenario: The settings panel swallows every inert action

- **WHEN** a `Dashboard` at `Route::List` with six active changes, `selected` `2`, `detail.tab`
  `1`, and the settings panel open is given `SelectTab(3)`, `NextTab`, `PrevTab`,
  `FilterStart`, `FilterPush('a')`, `FilterPop`, `ToggleSection`, `SelectNext`, `SelectPrev`,
  `Select`, and `Click(Target::Change(0))` in turn
- **THEN** `route` is still `List`, `selected` is still `2`, `detail.tab` is still `1`,
  `filter.query` is still empty, `filter.active` is still false, and `sections` is unchanged
- **AND** `overlay.panel` is still `Some(Panel::Settings)` and the row cursor has not moved
- **AND** the answered set holds nine and the inert remainder seventeen, summing with nothing
  counted twice to the twenty-six `Action` carries

#### Scenario: Both quit keys still quit from inside the settings panel

- **WHEN** a `Dashboard` with the settings panel open is given `Quit`
- **THEN** `quit` is true
- **AND** the same holds with an edit in progress and with `filter.active` also true, so no
  combination of layers traps the reader

### Requirement: The row cursor moves with `j`/`k` and the band scrolls to follow it

While the settings panel is open, `Next`/`ScrollDown` and `Prev`/`ScrollUp` SHALL move a
**row cursor** over the three settings rather than scroll an offset, and the band SHALL
scroll only as far as it must to keep the cursor visible.

The cursor SHALL address a **setting**, not a rendered row: `j` from `openspec_bin` lands on
`agent_kind`, skipping its source row, because a source row is not a thing the reader can
act on. It SHALL saturate at both ends — `k` at the first setting and `j` at the last both
leave it where it is — on exactly `list-selection`'s terms.

The cursor SHALL live on `Dashboard::settings.cursor`, beside the rows it indexes, and
**not** on `Overlay`. `overlay.scroll` is the help panel's first-visible-row offset, clamped
on every draw by `ui::layout::scroll_offset`; the settings panel derives its window from the
cursor instead, with `ui::layout::viewport` — the primitive the foldable detail path already
uses — so the two panels do not share one field with two incompatible meanings, and this
change adds no second scrolling rule to the crate. `overlay.scroll` SHALL be unread while the
settings panel is open.

`Next` and `Prev` SHALL move the cursor and SHALL NOT move `selected`, `detail.scroll`, or
the route beneath, on exactly the help panel's terms.

#### Scenario: The cursor walks settings, not rendered rows, and saturates

- **WHEN** a `Dashboard` at `Route::Detail` with `detail.scroll` `4` and `selected` `1`, the
  settings panel open and the cursor at `0`, is given `Next`, `Next`, `Next`, then `Prev`
- **THEN** the cursor is `1`, `2`, `2`, then `1` after the four — three settings, so the
  third `Next` saturates rather than reaching a fourth
- **AND** `detail.scroll` is still `4` and `selected` is still `1` after all four
- **AND** `Prev` applied to a dashboard whose cursor is `0` leaves it `0` rather than
  underflowing

#### Scenario: The band scrolls only when the cursor would leave it

- **WHEN** the settings panel is rendered at **120x8** and at **60x8** — a body of 7 rows, so
  the band is `min(7 + 2, 7)` = 7 and cannot hold the heading, six setting rows, and two rules
  — and the cursor is moved from the first setting to the last
- **THEN** the band's first visible row is `0` at the first setting, whose value row is
  already the second row of the band
- **AND** the cursor's own value row is inside the window at every step, and the window
  never advances past the last one the content allows — `content_rows - interior_height`,
  which is `2` here — so the band cannot scroll past its own end
- **AND** the window at every step is **exactly** the one `ui::layout::viewport` returns for
  that cursor row and that height, asserted by equality against that function rather than by
  restating its rule, so the panel and the foldable detail path cannot disagree about
  scrolling. `viewport` keeps the cursor near the middle of the band rather than scrolling by
  the least possible amount; that is the primitive's established behaviour and this panel
  adopts it whole rather than adding the crate's second scrolling rule

### Requirement: `Enter` begins and commits an edit, `Esc` cancels it, and only `agent_kind` is editable

While the settings panel is open, `OpenDetail` (the action `Enter` produces) SHALL begin an
edit on the setting under the cursor when no edit is in progress, and SHALL **commit** the
edit in progress when one is. `Back` (the action `Esc` produces) SHALL **cancel** an edit in
progress and SHALL leave the panel open; only when no edit is in progress SHALL `Back` close
the panel. An edit is therefore the innermost layer, one `Esc` inside the overlay, which is
itself one `Esc` inside the filter.

Exactly **one** setting SHALL be editable: `agent_kind`. `openspec_bin` and `prompts` SHALL
be **read-only** — a filesystem path and a multi-line prompt are miserable to type in a
terminal and are set-once values — and `Enter` on either SHALL begin no edit and change
nothing at all.

There SHALL be **no free-text entry anywhere in the panel**. `agent_kind` is chosen from a
shortlist, so the crate gains no text field outside `/` filter mode and no cursor, no
insertion point, and no character-level editing. The panel is therefore a picker, not a
form, and it grows no form machinery for a later setting to reuse — a later editable
setting is that change's problem to argue, not this one's to pre-build.

While an edit is in progress, `Next`/`ScrollDown` and `Prev`/`ScrollUp` SHALL change the
**candidate value** rather than move the row cursor, and the value row SHALL render the
candidate rather than the committed value, so the reader sees what they are about to commit.

A commit SHALL write the chosen kind to `settings.toml` through `plugin-state`'s recorder,
SHALL update the in-memory value the panel renders, SHALL call `Launcher::set_kind` so the
next `a`, `c`, or `s` launches under it without a restart (`agent-launch`), and SHALL leave
the cursor on `agent_kind`.

**`Dashboard::apply` SHALL do none of that itself.** It SHALL record the commit as a one-shot
request on the dashboard, exactly as it records `launch.pending`, and `run_loop` SHALL perform
the write and the `set_kind` call outside `src/ui/app.rs`. This is not a stylistic preference:
`scripts/gates/noio-view.sh`'s `IO_RE` lists `state::record`, which substring-matches
`state::record_kind`, so naming the recorder in any pure view file fails `NOIO-VIEW` — and it
should, because the write is I/O and `apply` is a pure total function of `&mut self` and an
`Action`. The in-memory update and the cursor position are `apply`'s; the file and the seam
call are the loop's. A cancel SHALL write nothing and SHALL
restore the rendered value to the committed one.

#### Scenario: An edit begins, changes a candidate, and commits

- **WHEN** a `Dashboard` with the settings panel open, the cursor on `agent_kind`, a
  shortlist of `claude` and `codex`, and a committed value of `claude` is given
  `OpenDetail`, then `Next`, then `OpenDetail`
- **THEN** after the first the edit is in progress with candidate `claude`
- **AND** after the second the candidate is `codex` and the value row renders `codex`
- **AND** after the third the edit is no longer in progress, the committed value is `codex`,
  the cursor is still on `agent_kind`, and `settings.toml` holds `agent_kind = "codex"`
- **AND** `Launcher::set_kind` was called exactly once, with `codex`

#### Scenario: `Esc` cancels the edit and a second `Esc` closes the panel

- **WHEN** that same dashboard is given `OpenDetail`, `Next`, then `Back`, then `Back`
- **THEN** after the third the edit is no longer in progress, the committed value is still
  `claude`, and the panel is **still open**
- **AND** after the fourth `overlay.panel` is `None`
- **AND** no `settings.toml` was created by any of the four, so a cancelled edit writes
  nothing

#### Scenario: `Enter` on a read-only setting begins no edit

- **WHEN** a `Dashboard` with the settings panel open and the cursor on `openspec_bin` is
  given `OpenDetail`, then the same with the cursor on `prompts`
- **THEN** no edit is in progress after either, and the dashboard is equal field for field
  to the one before each action
- **AND** no `settings.toml` is created by either

### Requirement: `agent_kind` is chosen from the installed integrations alone

The `agent_kind` edit SHALL offer a **shortlist**, and the shortlist SHALL be exactly the
kinds `integration status` reports as **installed**, in Herdr's own printed order. `Next`
and `Prev` SHALL cycle through it, wrapping at both ends, and the edit SHALL begin on the
committed kind when that kind is in the shortlist and on the first entry otherwise.

No kind outside the shortlist SHALL be reachable from this panel. This is a **deliberate
narrowing** of what `proposal.md` first argued, and it carries a stated cost: a kind that is
not installed launches perfectly well — `integration status` is not an availability check —
and the panel can no longer reach one. The reader who wants such a kind SHALL be told to
hand-edit `config.toml`, which continues to accept any kind and continues to outrank
anything this panel writes.

When the shortlist is **empty** — no integration is installed at all — `agent_kind` SHALL
NOT be editable. `Enter` on it SHALL begin no edit, and its source row SHALL carry a reason
naming `config.toml` as the way to set it, rather than opening an edit that can commit
nothing. This is the boundary case shortlist-only creates, and it degrades to a stated
reason rather than to a silently inert key.

#### Scenario: The shortlist is the installed kinds, in Herdr's order, and wraps

- **WHEN** an edit begins on `agent_kind` with `integration status` reporting `claude`,
  `codex`, and `cursor` installed in that order and `codex` committed
- **THEN** the candidate begins at `codex`
- **AND** `Next`, `Next` moves it to `cursor` then wraps to `claude`
- **AND** `Prev` from `claude` wraps to `cursor`
- **AND** a kind reported **not** installed never appears as a candidate at any step

#### Scenario: A committed kind outside the shortlist starts the edit at the first entry

- **WHEN** an edit begins on `agent_kind` whose committed value is `aider`, which is not
  among the installed `claude` and `codex`
- **THEN** the candidate begins at `claude`, the first entry in Herdr's order
- **AND** the committed value is still `aider` until the edit is committed, so beginning an
  edit changes nothing on its own

#### Scenario: No installed integration makes the row non-editable with a reason

- **WHEN** a `Dashboard` whose `integration status` reports no installed integration has the
  settings panel open with the cursor on `agent_kind` and is given `OpenDetail`
- **THEN** no edit is in progress afterwards and no `settings.toml` is created
- **AND** the `agent_kind` source row names `config.toml` as the way to set it, at both 120
  and 60 columns
- **AND** the panel is still open and `Back` still closes it, so the reader is not stuck

### Requirement: A setting `config.toml` sets is refused rather than shadowed

When an editable setting's effective value came from `config.toml` — `Provenance::Configured`
— the panel SHALL **refuse the edit**. `Enter` on that setting SHALL begin no edit, and its
source row SHALL say that `config.toml` is deciding it.

Refusing is chosen over accepting the edit and marking it shadowed. `config.toml` outranks
`settings.toml` at every step of `integration-status`' precedence and this change does not
relax that. An edit that is written, persisted, and visibly has no effect is the more
confusing of the two outcomes: the reader would have committed a value, seen the row keep
its old one, and had to read a badge to learn why. A refusal that names the file answers the
same question before anything is written.

The panel SHALL NOT write `config.toml` under any circumstance. The plugin's writes remain
scoped to its state directory: the configuration directory is the user's to hand-edit, and a
program rewriting that file would destroy its comments and formatting and race with the
editor that owns it.

#### Scenario: A configured `agent_kind` refuses the edit and names the file

- **WHEN** a `Dashboard` whose `config.toml` sets `agent_kind = "codex"` has the settings
  panel open with the cursor on `agent_kind` and is given `OpenDetail`
- **THEN** no edit is in progress afterwards and no `settings.toml` is created
- **AND** the `agent_kind` source row names `config.toml` at both 120 and 60 columns
- **AND** the configuration directory is byte-identical afterwards

#### Scenario: The refusal is per setting, not per panel

- **WHEN** a `Dashboard` whose `config.toml` sets `openspec_bin` and no other key, with two
  installed integrations, has the settings panel open
- **THEN** `OpenDetail` with the cursor on `openspec_bin` begins no edit, and its source row
  names `config.toml`
- **AND** `OpenDetail` with the cursor on `agent_kind` **does** begin one, so one setting
  being owned by `config.toml` does not freeze the panel
- **AND** committing that edit writes `settings.toml` holding only `agent_kind`, and leaves
  `config.toml` byte-identical
