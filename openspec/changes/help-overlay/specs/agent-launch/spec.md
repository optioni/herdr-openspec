## MODIFIED Requirements

### Requirement: The launch keys are offered only when the socket is reachable, and type themselves while filtering

The footer SHALL carry two further hints, `a/c/s launch` and `g focus`, in that order,
immediately after `Esc back` and **before** the `<n> unattributed` count, present exactly when
`Dashboard::agents.reachable` is `true`. The condition SHALL be read from `Dashboard::agents` and
from nowhere else — never from `ChangeSet::problems`, which `Dashboard::adopt` replaces wholesale
on every refresh.

A single compound hint is used rather than four separate ones because four do not fit the
mandated 60-column footer: with `help-overlay`'s leading `? help`,
`? help  q quit  Enter detail  Esc back  a apply  c continue  s archive`
is 70 columns, so `fit_hints` would drop `s archive`, `c continue`, and `g focus` and offer the
reader an arbitrary subset of the action keys at the narrow width. The compound form is kept
for that reason, unchanged.

`help-overlay` does move one measured outcome: the reachable footer is now **61** columns, so
`g focus` no longer fits the mandated 60-column frame and is dropped there. That is accepted
rather than worked around. The original objection to dropping an action hint was that the
reader would be left with "no indication that the other two exist", and `? help` is now the
first hint on the row: the overlay it opens lists `g` under `Agents` with its own description,
so a hint dropped at a narrow width is a hint the reader can still find. The footer is the
always-visible minimum; the overlay is the full list.

While filtering, `a`, `c`, `s`, and `g` SHALL type themselves into the query like every other
printable key, and the footer SHALL show the filter prompt alone — the action hints are dropped
with the rest.

#### Scenario: The action hints appear at both mandated widths when the socket is reachable

- **WHEN** a `Dashboard` with `agents.reachable` `true`, no filter, and no unattributed agents is
  rendered at 120x20 and at 60x20
- **THEN** the footer row at 120 reads exactly
  `? help  q quit  Enter detail  Esc back  a/c/s launch  g focus`, padded to the frame width
- **AND** that row is **61** columns of text, so at 60 the footer reads exactly
  `? help  q quit  Enter detail  Esc back  a/c/s launch` followed by eight spaces: `a/c/s
  launch` fits whole and `g focus` is dropped whole, never cut to `g focu`
- **AND** the reachable pane therefore still names an action key at the mandated narrow width,
  which is what the compound hint exists to guarantee

#### Scenario: An unreachable socket hides both hints at both widths

- **WHEN** the same `Dashboard` with `agents.reachable` `false` is rendered at 120x20 and 60x20
- **THEN** the footer row reads exactly `? help  q quit  Enter detail  Esc back`, padded, at
  both
- **AND** neither `a/c/s launch` nor `g focus` appears anywhere in either buffer, so a pane
  with no socket is offered no action key

#### Scenario: The count is dropped before the action hints as the width falls

- **WHEN** a `Dashboard` with `agents.reachable` `true` and one unattributed agent is rendered at
  120x20 and at 60x20
- **THEN** at 120 the footer reads
  `? help  q quit  Enter detail  Esc back  a/c/s launch  g focus  1 unattributed` — **77**
  columns
- **AND** at 60 it reads `? help  q quit  Enter detail  Esc back  a/c/s launch` — the full row
  is 77 columns, so `fit_hints` drops the count whole and then `g focus` whole, in that order,
  rather than cutting either
- **AND** with `agents.reachable` `false` at 60 the count reappears, reading
  `? help  q quit  Enter detail  Esc back  1 unattributed` — **54** columns — because the two
  hints it was competing with are absent

#### Scenario: The action keys type into the query while filtering

- **WHEN** the filter is active with an empty query and `a`, then `c`, then `s`, then `g` are
  pressed
- **THEN** each maps to `Action::FilterPush` of its own character and the query becomes `acsg`
- **AND** no launch request is produced by any of the four presses
- **AND** the footer shows `/acsg_` and neither action hint, at both mandated widths
