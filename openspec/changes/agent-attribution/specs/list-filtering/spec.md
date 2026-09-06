## MODIFIED Requirements

### Requirement: The footer shows the filter prompt while filtering and the query after

While `dashboard.filter.active` is true, the footer row SHALL render, starting at column 0,
`/` followed by the query followed by `_`, and nothing else — the key hints are replaced,
not appended to, and `agent-attribution`'s unattributed count is replaced along with them.
When the prompt is longer than the footer width it SHALL keep its **tail**, so the characters
just typed remain visible.

While `filter.active` is false and `filter.query` is non-empty, the footer's hint list
SHALL be `/` followed by the query, then `q quit`, then `Enter detail`, then `Esc back`, and
then — when `Dashboard::attribution().unattributed` is greater than zero —
`<n> unattributed`, joined by two spaces and dropped from the **end** by the existing rule.
The query is therefore the last hint to be dropped and the count is the first, which is the
whole reason each sits where it does: the query is the state the reader just typed, and the
count is the piece they can act on later.

While `filter.active` is false and `filter.query` is empty, the footer SHALL be exactly
what `responsive-layout` already specifies, unchanged — including that capability's trailing
count, which owns the row and which this requirement is read subject to.

#### Scenario: The prompt replaces the hints while filtering, at both widths

- **WHEN** a `Dashboard` whose `filter.active` is true and whose `filter.query` is `add` is
  rendered at 120x20 and at 60x20
- **THEN** the 60-column buffer's row 19 is exactly `/add_` followed by fifty-five spaces
- **AND** the 120-column buffer's row 19 is exactly `/add_` followed by one hundred and
  fifteen spaces
- **AND** neither buffer's row 19 contains `q quit`, `Enter detail`, or `Esc back`
- **AND** the same holds with two in-scope unattributable agents on the dashboard: neither
  buffer's row 19 contains `unattributed` either, so the prompt replaces the whole row

#### Scenario: An accepted query leads the hint list, at both widths

- **WHEN** a `Dashboard` whose `filter.active` is false and whose `filter.query` is `add`
  is rendered at 120x20 and at 60x20
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

#### Scenario: A prompt longer than the footer keeps its tail

- **WHEN** a `Dashboard` whose `filter.active` is true and whose `filter.query` is the
  seventy-character string `aaaaaaaaaa` repeated seven times is rendered at 60x20 and at
  120x20
- **THEN** the 60-column buffer's row 19 is exactly sixty characters ending in `_`, and its
  first character is `a`, not `/` — the head was dropped so the cursor stays visible
- **AND** the 120-column buffer's row 19 is exactly `/` followed by the seventy `a`s,
  then `_`, then forty-eight spaces, so the drop at 60 columns is a width branch
