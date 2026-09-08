## MODIFIED Requirements

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

- **WHEN** the same dashboard with an empty query and `selected` 4 — addressing
  `legacy-cleanup`, the fifth of five visible changes — is given the actions for `/`, then
  Presses of `Char('a')`, `Char('d')`, and `Char('d')`
- **THEN** `selected` is 1 after the three characters, because only two changes match `add`
- **AND** rendering at 120x20 and at 60x20 puts the `>` marker on the `add-auth` row in
  both, and no interior row is drawn with a marker for a change that is not shown
- **AND** a subsequent `Backspace`, `Backspace`, `Backspace` restores five visible changes
  with `selected` still 1, so clamping shrinks the index and never restores it
