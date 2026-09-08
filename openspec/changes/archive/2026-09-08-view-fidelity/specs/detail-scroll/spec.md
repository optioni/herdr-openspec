## MODIFIED Requirements

### Requirement: `j`, `k`, and the arrows scroll the detail content at the detail route

`Action::Next` and `Action::Prev` — renamed from `SelectNext` and `SelectPrev`, because the
action is route-agnostic and only its effect is not — SHALL be interpreted by
`Dashboard::apply` according to the current route: at `Route::List` they move and clamp
`selected`, per `list-selection`; at `Route::Detail` they move `detail.scroll` by one line,
with `Prev` saturating at `0` and `Next` saturating at `usize::MAX`, the upper bound being
enforced by the draw-time clamp and the frame normalisation below.

`action_for` SHALL be unchanged in shape: it maps `Char('j')` and `Down` to `Next` and
`Char('k')` and `Up` to `Prev` while `filtering` is false, and while `filtering` is true
`j` and `k` still type themselves into the query and only the arrows navigate — the layering
`list-filtering` states is untouched.

**Every action that moves `route`** SHALL reset `detail.scroll` to `0`, so a change opened
twice opens at the top both times. That is three arms of `apply`: `OpenDetail` when it sets
`Route::Detail`, `Back` when it returns to `Route::List`, and `FilterStart`, which
`list-filtering` also defines as moving the route to `List`. Dismissing a filter layer is
**not** a route move and SHALL leave `detail.scroll` alone.

**An action that does not move `route` SHALL NOT reset `detail.scroll`, and each of the three
arms above SHALL guard on that.** `OpenDetail` SHALL reset the scroll only when
`self.route != Route::Detail` — that is, only when it is about to change the route — matching
`Back`'s and `FilterStart`'s existing `before != after` guards, which is the rule this
sentence generalises rather than a new one. `OpenDetail` alone did not guard: it assigned the
route and zeroed the scroll unconditionally whenever the filter was inactive.

That was observable, not theoretical. In the wide layout — every width at or above the
100-column breakpoint — `responsive-layout` draws **both** regions regardless of route, so at
`Route::Detail` an `Enter` moves nothing on screen and its only visible effect was to throw
the reader back to line one of a `design.md` they were forty lines into. The requirement's own
wording already said "every action that moves `route`"; the implementation fired on an action
that moved none, and this clause is what closes that gap. The intended behaviour, stated
plainly: **`Enter` at the detail route is a no-op.** It is not rebound, it is not given a new
meaning such as "reload" or "scroll to top", and nothing else about the key changes — a
keybinding change would be **BREAKING** and this is deliberately not one.

`Back` at `Route::List` with no filter layer to dismiss remains a no-op that resets nothing,
which the same guard already gives it.

#### Scenario: At the detail route the content scrolls by one line at both widths

- **WHEN** a `Dashboard` whose `detail.source` is the twenty-item list, whose `route` is
  `Route::Detail`, and whose `detail.scroll` is `0` is given a `Next` action, then a second
  `Next`
- **THEN** `detail.scroll` is `1`, then `2`, and `selected` is unchanged throughout
- **AND** rendering after the second action at 120x20 puts `- line-02` at row 2, columns 41
  through 49, and `- line-17` at row 17
- **AND** rendering after the second action at 60x20 puts `- line-02` at row 2, columns 1
  through 9, and `- line-17` at row 17

#### Scenario: At the list route the same actions still move the selection

- **WHEN** a `Dashboard` with three active changes, a non-empty `detail.source`, `route` of
  `Route::List`, and `detail.scroll` of `0` is given two `Next` actions
- **THEN** `selected` is `2` and `detail.scroll` is still `0`
- **AND** rendering at 120x20 and at 60x20 puts the `>` marker on the third list row in
  both, so the contrast with the detail route is observed on screen and not only in state

#### Scenario: Scrolling stops at the top

- **WHEN** a `Dashboard` at `Route::Detail` whose `detail.source` is the twenty-item list and
  whose `detail.scroll` is `0` is given four consecutive `Prev` actions
- **THEN** `detail.scroll` is `0` after each, and nothing panics
- **AND** rendering at 120x20 and at 60x20 still puts `- line-00` on the interior's first
  row in both

#### Scenario: While filtering, `j` and `k` still type into the query

- **WHEN** `action_for` is called with `filtering` true and Presses of `Char('j')`,
  `Char('k')`, `Down`, and `Up`
- **THEN** it returns `FilterPush('j')`, `FilterPush('k')`, `Next`, and `Prev`
- **AND** applying those four to a `Dashboard` at `Route::Detail` with an active filter
  leaves `filter.query` as `jk` and moves `detail.scroll` to `1` and back to `0`, so the
  filter layer still wins over the scroll layer for printable keys

#### Scenario: Every route move resets the scroll

- **WHEN** a `Dashboard` at `Route::Detail` whose `detail.source` is the twenty-item list and
  whose `detail.scroll` is `3` is given a `Back` action, and then an `OpenDetail` action
- **THEN** `detail.scroll` is `0` after the `Back` and still `0` after the `OpenDetail`
- **AND** a second `Dashboard` in the same state given a `FilterStart` action instead has
  `route` `List` and `detail.scroll` `0`, while a third at `Route::Detail` with
  `filter.active` true and `detail.scroll` `3` given a `Back` — which dismisses the filter
  layer and not the route — still has `detail.scroll` `3`
- **AND** rendering the first dashboard at 120x20 and at 60x20 after the `OpenDetail` puts
  `- line-00` on the interior's first row in both

#### Scenario: `Enter` at the detail route moves nothing and keeps the scroll

- **WHEN** a `Dashboard` at `Route::Detail` with `filter.active` false, whose `detail.source`
  is the twenty-item list and whose `detail.scroll` is `7`, is given an `OpenDetail` action,
  and then a second and a third
- **THEN** `route` is `Route::Detail` and `detail.scroll` is `7` after each of the three
- **AND** `selected`, `filter`, and every other field of the `Dashboard` are unchanged, so
  the action is a no-op in state and not only in the scroll field
- **AND** rendering at 120x20 before and after the three actions produces byte-identical
  buffers, which is the width band where both regions are drawn and the route move was
  invisible; rendering at 60x20 likewise produces byte-identical buffers, with `- line-07`
  on the content area's first row in each

#### Scenario: `Enter` from the list route still opens at the top

- **WHEN** a `Dashboard` at `Route::List` with `filter.active` false, whose `detail.source`
  is the twenty-item list and whose `detail.scroll` is `7` — a value left behind by an
  earlier session at the detail route — is given an `OpenDetail` action
- **THEN** `route` is `Route::Detail` and `detail.scroll` is `0`, because the action moved
  the route
- **AND** rendering at 120x20 and at 60x20 puts `- line-00` on the content area's first row
  in both, so the guard narrowed the reset to real route moves and did not remove it

#### Scenario: `Enter` while filtering still dismisses the filter and resets nothing

- **WHEN** a `Dashboard` at `Route::Detail` with `filter.active` true, a query of `add`, and
  `detail.scroll` of `7` is given an `OpenDetail` action
- **THEN** `filter.active` is false, `filter.query` is still `add`, `route` is still
  `Route::Detail`, and `detail.scroll` is still `7`
- **AND** the same dashboard at `Route::List` given the same action has `filter.active`
  false, `route` still `Route::List`, and `detail.scroll` still `7`, so accepting a filter
  is not a route move at either route
