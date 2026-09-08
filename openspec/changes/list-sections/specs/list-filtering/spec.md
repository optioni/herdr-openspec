## MODIFIED Requirements

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

## ADDED Requirements

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
  `archived_total` 2, and `sections.collapsed` holding `SectionKey::Archived` is rendered at
  120x20 and at 60x20 with the accepted query `auth`
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
