# list-filtering Specification

## Purpose
Describes the `/` query layer over the change list: entering it from either route, the modal
keymap in which every printable character — `q`, `j`, `k`, `/`, `r`, the digits, and the action
keys alike — types itself rather than commanding, leaving only `Ctrl-C` to quit,
`Backspace` to delete, `Enter` to accept and `Esc` to cancel. The matching rule is deliberately
narrow: an ASCII-case-insensitive substring test against `Change::name` alone, applied to the
active and archived tiers alike, never against a path, schema, artifact id or problem text. It
also owns the footer while a query exists — a `/query_` prompt that replaces the hint row
outright while filtering and keeps its tail when it overruns the width, and a leading `/query`
hint once accepted.

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

`Char(' ')` — `list-sections`' `ToggleSection` key — types itself too, with no exception
carved out for it and no new table row: a space arrives as `KeyCode::Char(' ')` with no
modifiers, which the first row above already maps to `FilterPush(' ')`. Change names contain
no spaces, so a space in a query matches nothing; typing one is nonetheless what the reader
expects from a text field, and folding a section from inside a query would be acting on a
list the query has already forced open.

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

#### Scenario: `Space` types into the query rather than folding a section

- **WHEN** a `Dashboard` with three active changes, two archived changes and the archived
  section collapsed enters filter mode with `/` and is then given Presses of `Char('a')`,
  `Char(' ')`, and `Char('d')`
- **THEN** `filter.query` is `a d` and `filter.active` is still true
- **AND** `sections.collapsed` is unchanged by all three, so no section was folded or unfolded
  by the key itself
- **AND** the footer at 60x20 and at 120x20 is exactly `/a d_` followed by spaces
- **AND** the same three keys with `filter.active` false leave `filter.query` empty and fold
  a section on the second, so the typing is the filter mode and not the key losing its binding

### Requirement: The query is a case-insensitive substring match on the change name

A change SHALL be visible when its `name`, lowercased, contains the query, lowercased. An
empty query SHALL match every change. The rule SHALL apply to the active and the archived
tier alike, and SHALL read only `Change::name` — never the directory path, the schema, the
artifact ids, or the problems.

**The lowercasing SHALL be Unicode's, not ASCII's.** `ui::app::matches` SHALL fold both sides
with `str::to_lowercase`, which applies the Unicode default case-conversion mapping, and SHALL
NOT use `str::to_ascii_lowercase`, which leaves every codepoint above U+007F untouched. Under
the ASCII fold a query of `Ä` did not match a change named `änderung`, while `A` matched
`add` — the pane folded case for half its input and not the other half, in the same
non-ASCII blind spot `responsive-layout`'s display-column measure closes one layer down. A
filter that cannot find what the list is displaying is the failure being fixed; the previous
behaviour was documented rather than accidental, which is why it is corrected here in the
spec and not only in the code.

The fold SHALL be **default** case conversion, not locale-tailored and not full case
folding: `str::to_lowercase` is the whole of the rule, including its documented behaviours —
a final sigma `Σ` lowercases to `ς` in final position, and `İ` (U+0130) lowercases to two
codepoints. The pane SHALL NOT tailor the mapping to a locale, and SHALL NOT normalise
either side: `ä` written as U+00E4 and `ä` written as `a` plus U+0308 are different names to
this rule, because they are different bytes on disk and the list shows what the directory is
called.

The match is on **codepoints after folding**, not on display columns; the display-column
measure `responsive-layout` introduces governs how a matched row is drawn, not whether it
matches.

`change-rows`' emission order is unchanged by filtering: each **section header** is emitted
only when that section's count — the number of changes in its tier the query matches — is
greater than zero, and the `No active changes` message replaces the active rows when the
active count is zero while the archived count is not. `list-sections` replaced the single
separator with two headers; the emission rule is the same rule, now applied twice.

#### Scenario: A query narrows both tiers at both widths

- **WHEN** a `Dashboard` with active changes `add-token-refresh` at 4 of 9,
  `fix-empty-basket` at 7 of 7, and `migrate-ai-sdk-v7` at 0 of 0, archived changes
  `add-auth` dated `2026-08-14` at 7 of 7 and `legacy-cleanup` undated at 3 of 3,
  `selected` **1** — target 0 is the active section header — and an accepted query `add`, is
  rendered at 120x20 and at 60x20
- **THEN** the 120-column buffer's interior rows 2, 3, 4, and 5 at columns 1 through 38 spell
  `  v active (1)` padded,
  `> add-token-refresh              [4/9]`,
  `  v archived (1)` padded, and
  `  2026-08-14 add-auth            [7/7]`
- **AND** the 60-column buffer's same four rows at columns 1 through 58 spell the same four,
  padded to 58
- **AND** neither buffer contains the string `-- archived`
- **AND** in both buffers the strings `fix-empty-basket`, `migrate-ai-sdk-v7`, and
  `legacy-cleanup` appear nowhere

#### Scenario: Matching ignores case

- **WHEN** the same dashboard is rendered at 120x20 and at 60x20 with the query `ADD`, and
  again with the query `Add`
- **THEN** all four buffers hold exactly the rows the query `add` produced
- **AND** a dashboard whose active change is named `ADD-TOKEN-REFRESH` and whose query is
  `add` also matches, so the lowercasing is applied to both sides

#### Scenario: Matching ignores case outside ASCII

- **WHEN** a `Dashboard` whose active changes are `änderung-der-api`, `ÜBERSICHT`, and
  `add-token-refresh` is rendered at 120x20 and at 60x20 with the query `Ä`, then with `ä`,
  then with `ÜBER`, then with `über`
- **THEN** the `Ä` and `ä` buffers each show `änderung-der-api` and neither of the other two
- **AND** the `ÜBER` and `über` buffers each show `ÜBERSICHT` and neither of the other two,
  so the fold is applied to the change name as well as to the query
- **AND** `matches("änderung", "Ä")` is true and `matches("ÄNDERUNG", "ä")` is true, while
  `matches("add-token-refresh", "Ä")` is false — the fold widened what matches without
  matching everything
- **AND** the query `add` still shows `add-token-refresh` alone in both buffers, so the ASCII
  behaviour every landed scenario asserts is unchanged

#### Scenario: A query matching only an archived change

- **WHEN** the same dashboard with the query `auth` is rendered at 120x20 and at 60x20
- **THEN** in both buffers the first interior row begins `No active changes`, the second is
  exactly `  v archived (1)` padded to the interior width, and the third is the `add-auth`
  row carrying the `>` marker, because `selected` **1** addresses the first visible change
  and the only visible change is archived — `selected` 0 would put the marker on the archived
  header, which is target 0 here since the active section's count is zero and it emits no
  header
- **AND** neither buffer contains `No changes match` or `No changes yet`

#### Scenario: A query matching nothing names itself

- **WHEN** the same dashboard with the query `zzz` is rendered at 120x20 and at 60x20
- **THEN** in both buffers the first interior row begins `No changes match` and the second
  begins `/zzz`
- **AND** neither buffer contains `-- archived`, any change name, or a `>` marker in the
  interior's first column

#### Scenario: The fold is total and its documented edge cases hold

- **WHEN** `matches` is called with each of the empty string, a 200-character name, a name
  holding a NUL, a name of only combining marks, the name `ΟΔΟΣ` against a query of
  `οδο` followed by U+03C2 FINAL SIGMA and again against `οδο` followed by U+03C3 SIGMA, the
  name `İstanbul` (U+0130) against the query `i` followed by U+0307 and `stanbul`, and a
  family emoji name against an emoji query
- **THEN** no call panics, and an empty query matches every one of them
- **AND** the FINAL SIGMA query matches `ΟΔΟΣ` while the SIGMA query does not, because
  `str::to_lowercase` maps a trailing sigma to U+03C2 by position — the documented behaviour
  of the default mapping, recorded here so it reads as a known consequence and not a defect
- **AND** the `İstanbul` query matches, because `str::to_lowercase` expands U+0130 to two
  codepoints and the query already spells that expansion
- **AND** neither `ä` written as U+00E4 nor `ä` written as `a` plus U+0308 matches the other,
  because the rule normalises neither side

#### Scenario: Shrinking the visible list clamps the selection

- **WHEN** the same dashboard with an empty query, both sections open, and `selected` **6** —
  addressing `legacy-cleanup`, the last of seven targets: two headers and five changes — is
  given the actions for `/`, then Presses of `Char('a')`, `Char('d')`, and `Char('d')`
- **THEN** `selected` is 3 after the three characters, because only two changes match `add`
  and the four surviving targets are the active header, `add-token-refresh`, the archived
  header, and `add-auth`
- **AND** rendering at 120x20 and at 60x20 puts the `>` marker on the `add-auth` row in
  both, and no interior row is drawn with a marker for a change that is not shown
- **AND** a subsequent `Backspace`, `Backspace`, `Backspace` restores seven targets with
  `selected` still 3 — now addressing the archived header — so clamping shrinks the index and
  never restores it

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

### Requirement: A non-empty query forces every section open

While `dashboard.filter.query` is non-empty, **every** section SHALL be treated as open, for
`change-rows`' row emission, for `list-selection`'s `visible()` and `targets()`, and for
`Dashboard::archived_scope()` alike, regardless of what `sections.collapsed` holds. A filter
that silently hides matches is worse than a long list: a reader who searches for `add` and is
shown nothing, because the only match is inside a folded archive, has been told the change
does not exist.

The force-open SHALL be **derived, never stored**. `Dashboard::section_open(key)` SHALL be
`!self.filter.query.is_empty() || !self.sections.collapsed.contains(&key)`, and no code path
SHALL write to `sections` on account of a query. The reader's own collapse state is therefore
restored the instant the query becomes empty again — by `Backspace`, by `Esc`, or by any
other means — with nothing to save and nothing to restore.

The rule keys off `filter.query`, not `filter.active`: an **accepted** query (`Enter`, which
leaves `active` false and the query in place) filters the list and SHALL force the sections
open on exactly the same terms as a query still being typed.

Forcing the archived section open makes `Dashboard::archived_scope()` `ArchivedScope::Full`,
which — through `list-selection`'s `needs_archived_refresh()` rule on `Dashboard::apply` —
requests the refresh that resolves the tier. The first character of a query therefore costs
one refresh cycle when the archive is unresolved, and no further character costs another,
because the predicate is false once the tier is resolved.

#### Scenario: A query reaches a match inside a folded archive

- **WHEN** a `Dashboard` with active changes `fix-empty-basket` and `migrate-ai-sdk-v7`,
  resolved archived changes `add-auth` dated `2026-08-14` and `legacy-cleanup` undated,
  `archived_total` 2, `sections.collapsed` holding `SectionKey::Archived`, and `selected`
  **1** — target 0 is the archived section header, since the active section's count is zero
  and it emits none — is rendered at 120x20 and at 60x20 with the accepted query `auth`
- **THEN** in both buffers the first interior row is `No active changes`, the second is
  exactly `  v archived (1)` padded to the interior width, and the third is the `add-auth`
  row carrying the `>` marker
- **AND** the archived header's glyph is `v` although `sections.collapsed` still holds
  `SectionKey::Archived`, so the force-open is derived rather than written
- **AND** clearing the query and rendering again shows `  > archived (2)` and no archived
  name, so the reader's own fold came back with nothing having been saved

#### Scenario: The archived count under a query is the matched count

- **WHEN** the same dashboard with both sections open and the accepted query `add` is
  rendered at 120x20 and at 60x20
- **THEN** in both buffers the archived header is exactly `  v archived (1)` padded to the
  interior width, because one of the two archived changes matches
- **AND** no active header is emitted at all, and the `No active changes` row stands in its
  place, because a section whose count is zero emits no header
- **AND** clearing the query renders `  v active (2)` and `  v archived (2)`, so the counts
  follow the query when the tier is resolved and the true total when it is not

#### Scenario: The first character of a query requests the archive it needs

- **WHEN** a `Dashboard` whose `changes.archived` is empty, whose `archived_total` is 22,
  whose archived section is collapsed, and whose `refresh.requested` is false is given
  `FilterStart` and then a Press of `Char('a')`, then `Char('d')`, then `Char('d')`
- **THEN** `refresh.requested` is true and `archived_scope()` is `ArchivedScope::Full` after
  the first character
- **AND** setting `refresh.requested` back to false and giving the second and third characters
  leaves it true again — the predicate is still unsatisfied while the tier is unresolved —
  and giving the same two characters against a dashboard whose twenty-two archived changes are
  present leaves it false, so a resolved tier costs no further cycle
- **AND** `Backspace` back to an empty query leaves `archived_scope()` `ArchivedScope::Names`
  and `refresh.requested` untouched, because a fold needs no data
