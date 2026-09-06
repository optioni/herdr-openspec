# list-filtering Specification

## Purpose
TBD - created by archiving change list-view. Update Purpose after archive.

## Requirements

### Requirement: `/` opens a filter mode in which printable keys type rather than command

`ui::app::Dashboard` SHALL carry a `filter: Filter` field, where
`ui::app::Filter { query: String, active: bool }`. `ui::load` SHALL start it with an empty
query and `active` false. `Filter` SHALL NOT implement `Default` — neither derived nor
hand-written, anywhere in the crate — and every construction and destructuring SHALL name
both fields with no `..` rest, on the same terms `dashboard-loop` already states for
`Dashboard`.

`ui::app::action_for(event: &Event, filtering: bool) -> Action` SHALL take the filter mode
as its second argument and SHALL remain total: every `Event` value maps to an `Action` and
none panics. `ui::driver::run_loop` SHALL pass `dashboard.filter.active`.

While `filtering` is **false**, `Char('/')` with no modifiers SHALL map to
`Action::FilterStart`. `Dashboard::apply` SHALL then set `filter.active` true and set
`route` to `Route::List`, so `/` reaches the list from either route and at either width.

While `filtering` is **true** the mapping SHALL be:

| Input | Action |
|---|---|
| `Char(c)` with `KeyModifiers::NONE` or `KeyModifiers::SHIFT` | `FilterPush(c)` |
| `Char('c')` with `KeyModifiers::CONTROL` | `Quit` |
| `Backspace` with no modifiers | `FilterPop` |
| `Enter` with no modifiers | `OpenDetail` |
| `Esc` with no modifiers | `Back` |
| `Up` / `Down` with no modifiers | `SelectPrev` / `SelectNext` |
| anything else | `Ignore` |

`Char('q')`, `Char('j')`, `Char('k')`, and `Char('/')` therefore type themselves while
filtering: only `Ctrl-C` closes the pane from inside filter mode. `Backspace` on an empty
query SHALL leave the query empty and filter mode active rather than cancelling.

`Dashboard::apply` SHALL push a character on `FilterPush`, pop the last character on
`FilterPop`, and clamp `selected` after each, per `list-selection`.

#### Scenario: `/` starts filter mode from either route

- **WHEN** a `Dashboard` at `Route::Detail` with `filter.active` false is given the action
  for a Press of `Char('/')`
- **THEN** `filter.active` is true, `filter.query` is empty, and `route` is `Route::List`
- **AND** rendering at 120x20 and at 60x20 shows the `Changes` region in both, so `/` is
  usable at either width

#### Scenario: `q` types a character while filtering and does not quit

- **WHEN** a `Dashboard` with `filter.active` true is given the actions for a Press of
  `Char('q')`, then `Char('j')`, then `Char('/')`
- **THEN** `filter.query` is `qj/` and `quit` is false after all three
- **AND** giving the same dashboard a Press of `Char('c')` with `KeyModifiers::CONTROL`
  sets `quit` true, so exactly one key still closes the pane from filter mode

#### Scenario: Backspace deletes, and on an empty query is inert

- **WHEN** a `Dashboard` whose `filter.query` is `ad` and whose `filter.active` is true is
  given a Press of `Backspace`, then another, then a third
- **THEN** `filter.query` is `a`, then empty, then still empty
- **AND** `filter.active` is true after all three, so an over-run backspace does not leave
  filter mode

#### Scenario: `Esc` cancels the filter and `Enter` accepts it

- **WHEN** a `Dashboard` at `Route::List` whose `filter.query` is `add` and whose
  `filter.active` is true is given a Press of `Enter`
- **THEN** `filter.active` is false, `filter.query` is still `add`, and `route` is still
  `Route::List` — accepting a filter does not open a detail
- **AND** giving a second such dashboard a Press of `Esc` instead leaves `filter.active`
  false **and** `filter.query` empty, and `quit` false

### Requirement: The query is a case-insensitive substring match on the change name

A change SHALL be visible when its `name`, ASCII-lowercased, contains the query,
ASCII-lowercased. An empty query SHALL match every change. The rule SHALL apply to the
active and the archived tier alike, and SHALL read only `Change::name` — never the
directory path, the schema, the artifact ids, or the problems.

`change-rows`' emission order is unchanged by filtering: the separator is emitted only when
at least one archived change matches, and the `No active changes` message replaces the
active rows when none matches while at least one archived one does.

#### Scenario: A query narrows both tiers at both widths

- **WHEN** a `Dashboard` with active changes `add-token-refresh` at 4 of 9,
  `fix-empty-basket` at 7 of 7, and `migrate-ai-sdk-v7` at 0 of 0, archived changes
  `add-auth` dated `2026-08-14` at 7 of 7 and `legacy-cleanup` undated at 3 of 3,
  `selected` 0, and an accepted query `add`, is rendered at 120x20 and at 60x20
- **THEN** the 120-column buffer's interior rows 2, 3, and 4 at columns 1 through 38 spell
  `> add-token-refresh              [4/9]`,
  `  -- archived ------------------------`, and
  `  2026-08-14 add-auth            [7/7]`
- **AND** the 60-column buffer's interior rows 2, 3, and 4 at columns 1 through 58 spell
  `> add-token-refresh                                  [4/9]`,
  `  -- archived --------------------------------------------`, and
  `  2026-08-14 add-auth                                [7/7]`
- **AND** in both buffers the strings `fix-empty-basket`, `migrate-ai-sdk-v7`, and
  `legacy-cleanup` appear nowhere

#### Scenario: Matching ignores case

- **WHEN** the same dashboard is rendered at 120x20 and at 60x20 with the query `ADD`, and
  again with the query `Add`
- **THEN** all four buffers hold exactly the rows the query `add` produced
- **AND** a dashboard whose active change is named `ADD-TOKEN-REFRESH` and whose query is
  `add` also matches, so the lowercasing is applied to both sides

#### Scenario: A query matching only an archived change

- **WHEN** the same dashboard with the query `auth` is rendered at 120x20 and at 60x20
- **THEN** in both buffers the first interior row begins `No active changes`, the second is
  the archived separator, and the third is the `add-auth` row carrying the `>` marker,
  because `selected` 0 addresses the first **visible** change and the only visible change
  is archived
- **AND** neither buffer contains `No changes match` or `No changes yet`

#### Scenario: A query matching nothing names itself

- **WHEN** the same dashboard with the query `zzz` is rendered at 120x20 and at 60x20
- **THEN** in both buffers the first interior row begins `No changes match` and the second
  begins `/zzz`
- **AND** neither buffer contains `-- archived`, any change name, or a `>` marker in the
  interior's first column

#### Scenario: Shrinking the visible list clamps the selection

- **WHEN** the same dashboard with an empty query and `selected` 4 — addressing
  `legacy-cleanup`, the fifth of five visible changes — is given the actions for `/`, then
  Presses of `Char('a')`, `Char('d')`, and `Char('d')`
- **THEN** `selected` is 1 after the three characters, because only two changes match `add`
- **AND** rendering at 120x20 and at 60x20 puts the `>` marker on the `add-auth` row in
  both, and no interior row is drawn with a marker for a change that is not shown
- **AND** a subsequent `Backspace`, `Backspace`, `Backspace` restores five visible changes
  with `selected` still 1, so clamping shrinks the index and never restores it

### Requirement: The footer shows the filter prompt while filtering and the query after

While `dashboard.filter.active` is true, the footer row SHALL render, starting at column 0,
`/` followed by the query followed by `_`, and nothing else — the key hints are replaced,
not appended to, and `agent-launch`'s two action hints and `agent-attribution`'s unattributed
count are replaced along with them.
When the prompt is longer than the footer width it SHALL keep its **tail**, so the characters
just typed remain visible.

While `filter.active` is false and `filter.query` is non-empty, the footer's hint list
SHALL be `/` followed by the query, then `q quit`, then `Enter detail`, then `Esc back`, then
— when `Dashboard::agents.reachable` is true — `a/c/s launch` and `g focus`, and
then — when `Dashboard::attribution().unattributed` is greater than zero —
`<n> unattributed`, joined by two spaces and dropped from the **end** by the existing rule.
The query is therefore the last hint to be dropped and the count is the first, which is the
whole reason each sits where it does: the query is the state the reader just typed, the action
hints say which keys are live right now, and the count is the piece they can act on later.

While `filter.active` is false and `filter.query` is empty, the footer SHALL be exactly
what `responsive-layout` already specifies, unchanged — including that capability's action
hints and trailing count, which owns the row and which this requirement is read subject to.

The four action keys themselves — `a`, `c`, `s`, and `g` — SHALL type into the query while
`filter.active` is true, like every other printable character and with no exception carved out
for any of them, on exactly `r`'s and the digits' terms. That is not an incidental consequence
of `action_for`'s filter-mode table: it is the rule that makes the filter usable, since four of
the twenty-six lower-case letters would otherwise be unavailable in a query, and `2fa-support`,
`add-auth`, and `agent-launch` all contain at least one of them.

#### Scenario: The prompt replaces the hints while filtering, at both widths

- **WHEN** a `Dashboard` whose `filter.active` is true and whose `filter.query` is `add` is
  rendered at 120x20 and at 60x20
- **THEN** the 60-column buffer's row 19 is exactly `/add_` followed by fifty-five spaces
- **AND** the 120-column buffer's row 19 is exactly `/add_` followed by one hundred and
  fifteen spaces
- **AND** neither buffer's row 19 contains `q quit`, `Enter detail`, or `Esc back`
- **AND** the same holds with two in-scope unattributable agents on the dashboard: neither
  buffer's row 19 contains `unattributed` either, so the prompt replaces the whole row
- **AND** the same holds with `agents.reachable` set to `true`: neither buffer's row 19 contains
  `a/c/s launch` or `g focus`, so the prompt replaces the action hints too

#### Scenario: An accepted query leads the hint list, at both widths

- **WHEN** a `Dashboard` whose `filter.active` is false, whose `filter.query` is `add`, and
  whose `agents.reachable` is false is rendered at 120x20 and at 60x20
- **THEN** the 60-column buffer's row 19 is exactly
  `/add  q quit  Enter detail  Esc back` — thirty-six characters — followed by twenty-four
  spaces
- **AND** the 120-column buffer's row 19 is the same thirty-six characters followed by
  eighty-four spaces
- **AND** the same dashboard with an empty query renders row 19 as exactly
  `q quit  Enter detail  Esc back` followed by spaces at both widths, so the leading hint
  is present only when a query is
- **AND** the same dashboard with one in-scope unattributable agent added renders row 19 as
  exactly `/add  q quit  Enter detail  Esc back  1 unattributed` — fifty-two characters — at
  both widths, so the query leads the list and the count trails it
- **AND** the same dashboard with `agents.reachable` set to `true` and that one agent renders
  row 19 as exactly
  `/add  q quit  Enter detail  Esc back  a/c/s launch  g focus  1 unattributed` — seventy-five
  characters — at 120, and as exactly
  `/add  q quit  Enter detail  Esc back  a/c/s launch  g focus` — fifty-nine characters —
  followed by one space at 60: the action hints sit between `Esc back` and the count, and at
  the narrow width the count is the one dropped

#### Scenario: The action keys type into the query rather than launching

- **WHEN** a `Dashboard` whose `agents.reachable` is true and whose visible list holds one
  change enters filter mode with `/` and is then given Presses of `a`, `c`, `s`, and `g`
- **THEN** `filter.query` is `acsg` and `filter.active` is still true
- **AND** `launch.pending` is `None` and `launch.problems` is empty after all four, so not one
  of them produced a request
- **AND** the footer at 60x20 and at 120x20 is exactly `/acsg_` followed by spaces, carrying
  neither action hint
- **AND** `Ctrl-C` still quits from that state, and `Esc` still cancels the query, so the two
  keys that escape filter mode are unchanged

#### Scenario: A prompt longer than the footer keeps its tail

- **WHEN** a `Dashboard` whose `filter.active` is true and whose `filter.query` is the
  seventy-character string `aaaaaaaaaa` repeated seven times is rendered at 60x20 and at
  120x20
- **THEN** the 60-column buffer's row 19 is exactly sixty characters ending in `_`, and its
  first character is `a`, not `/` — the head was dropped so the cursor stays visible
- **AND** the 120-column buffer's row 19 is exactly `/` followed by the seventy `a`s,
  then `_`, then forty-eight spaces, so the drop at 60 columns is a width branch
- **AND** the same holds with `agents.reachable` `true`, because the prompt form has no hint
  list to grow
