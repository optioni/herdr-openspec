## MODIFIED Requirements

### Requirement: Every row of the degraded-states table is bound to a named proving test

`SPEC.md` → Degraded states is the plugin's never-fail-closed contract, and it is the last
claim in the repository with nothing forcing it to stay true. A checked-in coverage map at
`tests/degraded-coverage.toml` SHALL bind **every** row of that table to at least one named
test, and `tests/degraded_coverage.rs` — an ordinary `cargo test` target, and therefore part
of `make check` — SHALL check the binding on every run.

The map SHALL be a TOML array of tables named `row`, each carrying exactly six keys:

- `condition` — the row's **first column, verbatim**, including its backticks and its
  Markdown emphasis, so a reworded row breaks the binding rather than silently keeping it;
- `tier` — one of `view`, `outer`, `unit`, or `integration`;
- `proof` — a non-empty array of test function names, each a bare Rust function identifier;
- `verdict` — one of `confirmed`, `unproven`, `spec-corrected`, `repaired`, or `implemented`,
  the audit's own finding for that row;
- `why` — one sentence naming what the proof observes, so a reader can tell a proof that
  watches the rendered state from one that watches a value nothing renders;
- `covers` — a non-empty array of `path:first-last` line ranges naming the **production
  code** that implements the row, so the map says where the behaviour lives and not only
  which test claims to watch it.

`covers` is new, and it exists because a `proof` alone proved less than it appeared to.
Measured at HEAD: `src/watch.rs:264-268` — the `Err` arm that returns the inert watcher and
the "filesystem watch unavailable" problem, a row of this very table — is **uncovered by the
coverage run**, while that row's `proof` resolves, its tier check passes, and the map reports
green. A name that resolves somewhere under `src/` or `tests/` is not evidence that the
degraded path ran.

The `verdict` lives **here**, in `tests/`, rather than only in the change's `notes/audit.md`.
`openspec archive` relocates a change's directory, so a check reading a path under
`openspec/changes/<name>/` would break permanently the moment this change archives — and it
is inside `make check`. The narrative audit stays in the change's notes; the machine-checked
half lives where it will still resolve in a year.

The parser SHALL read the table between the `## Degraded states` heading and the
`### No terminal is not a degraded state` heading that follows it, SHALL skip the header and
separator rows, and SHALL take each remaining line's first pipe-delimited cell, trimmed.

The check SHALL fail when any of the following is true, naming the offending row or entry:

1. a table row has no `row` entry whose `condition` equals it;
2. a `row` entry's `condition` matches no table row (an orphan left by a reworded row);
3. two `row` entries carry the same `condition`;
4. a named `proof` function is not defined anywhere under `src/` or `tests/` — searched as a
   line-anchored `fn <name>(` so a name that appears only in a comment does not satisfy it;
4b. a named `proof` function is not **a test**. It SHALL carry a `#[test]` attribute on the
   line above its `fn` line, or — for a shared helper several tests call — be named in the
   body of at least one function that does. Today the check accepts any `fn <name>(`
   whatsoever, so a `proof` may name a production function, a closure-free helper nothing
   calls, or a test that was `#[ignore]`d, and the binding still reports green;
4c. a `covers` range does not resolve: its path does not exist under `src/`, its `first` and
   `last` are not a `first <= last` pair within that file's line count, or the range holds no
   line of code at all;
5. a `tier = "view"` or `tier = "outer"` entry names a function whose **own body** does not
   name `TestBackend` — the body being the text from its `fn <name>(` line to the closing
   brace at that function's own indentation. File granularity would prove nothing: only three
   of the eleven files under `src/ui/` name `TestBackend` at all, and within those three a
   file-level check passes for every function in the file. A file requirement would also be
   unsatisfiable for a proof that belongs in `src/ui/detail.rs` or `src/ui/markdown.rs`, which
   `NOTABSEAM` and `MDSEAM` forbid from naming a `ratatui` type — such a proof lives in
   `src/ui/view.rs`, and this rule says so by checking the body rather than the path;
6. the table holds fewer than **44** rows, or the map fewer than **44** entries — a floor,
   not an equality, so adding a degraded state is allowed and dropping the whole table is not.

`tier` SHALL record the tier at which the row's **own wording** is observable, not the
cheapest tier that could be written. A row that names something rendered is `view`; a row
whose rendering is only reachable by driving the real `ui::run_wired` is `outer`; a row that
names a value the plugin records but nothing renders is `unit`; a row about `open`/`open-tab`
is `integration`, because those are one-shot commands with nothing to render — `SPEC.md`'s own
carve-out immediately below the table.

`SPEC.md`'s roadmap row for this change asks for "a view test" per state. Three tiers other
than `view` appear above, and that is a stated deviation rather than a shortcut: five rows
describe one-shot commands with nothing to render, and two describe values the plugin records
deliberately without rendering. Writing a `view` proof for those would be writing a test that
watches the wrong thing.

#### Scenario: The map covers the table at HEAD

- **WHEN** `cargo test --all-features` runs `degraded_coverage` on the repository at HEAD
- **THEN** the test passes, having parsed **at least 44** rows out of `SPEC.md` and matched
  every one to exactly one entry of `tests/degraded-coverage.toml`
- **AND** every `proof` name it read resolves to a line-anchored `fn <name>(` under `src/` or
  `tests/`
- **AND** every `tier = "view"` proof resolves to a file under `src/ui/` whose text names
  `TestBackend`

#### Scenario: A row added to SPEC without a proof fails the build

- **WHEN** a copy of `SPEC.md` gains a forty-fifth degraded-states row,
  `| A planted condition | A planted behaviour |`, and the map is left unchanged
- **THEN** the test exits non-zero naming `A planted condition` as uncovered
- **AND** removing the planted row returns the test to green

#### Scenario: A renamed test fails the binding rather than passing vacuously

- **WHEN** one `proof` entry is changed to `a_function_that_does_not_exist`
- **THEN** the test exits non-zero naming that identifier and the condition it was bound to
- **AND** the same happens when the identifier is left valid but its `condition` is reworded
  by one character, which the orphan check reports separately from the uncovered check, so
  the two failure modes are distinguishable in the message

#### Scenario: A `view` tier pointing at a test that renders nothing fails

- **WHEN** one `tier = "view"` entry is repointed at a unit test in `src/changes.rs` whose own
  body does not name `TestBackend`
- **THEN** the test exits non-zero naming that identifier and that its body renders nothing
- **AND** repointing it instead at a function in `src/ui/view.rs` that does **not** itself
  render — one whose body names no `TestBackend`, in a file where other functions do — fails
  on the same rule, which is what makes the check function-granular rather than file-granular

#### Scenario: An empty table is a failure, not a vacuous pass

- **WHEN** the parser is run against a `SPEC.md` copy whose degraded-states table holds only
  its header and separator rows
- **THEN** the test exits non-zero on the 44-row floor rather than reporting full coverage of
  zero rows
- **AND** the same holds when the `## Degraded states` heading is absent entirely

#### Scenario: A `proof` that is not a test fails the binding

- **WHEN** one `proof` entry is repointed at a production function — `fn start(` in
  `src/watch.rs`, which resolves under the old rule and carries no `#[test]`
- **THEN** the test exits non-zero naming that identifier and that it is not a test
- **AND** the same happens when the entry names a real test function whose `#[test]`
  attribute has been replaced with `#[ignore]`, since a test nothing runs proves nothing
- **AND** an entry naming a shared helper still passes, provided at least one `#[test]`
  function's body names it, so the rule does not force every proof to be a test itself

#### Scenario: A `covers` range that does not resolve fails the binding

- **WHEN** one entry's `covers` names `src/watch.rs:9000-9001`, a range past the end of the
  file
- **THEN** the test exits non-zero naming that entry and the unresolvable range
- **AND** the same happens for a path that does not exist under `src/`, for a reversed
  `last-first` pair, and for a range holding only blank lines and comments
- **AND** an entry with no `covers` key at all fails on the six-key rule, so the twenty
  `unproven` rows cannot keep their verdict without saying where the behaviour lives

#### Scenario: `unproven` gains a consequence rather than a new verdict

- **WHEN** the map is read at HEAD, where twenty of the forty-four entries carry
  `verdict = "unproven"`
- **THEN** each of those twenty satisfies the same `proof`-is-a-test and `covers`-resolves
  rules as every other entry, so `unproven` records what the audit found at audit time and
  no longer names a row with weaker machinery behind it
- **AND** no sixth verdict is introduced: the verdict list stays the five
  `degraded-states` established, because the defect was that `LEGAL_VERDICTS` accepted a
  value with no consequence attached, not that a value was missing


## ADDED Requirements

### Requirement: The code a degraded-states row covers is executed by the test suite

A `proof` that resolves and a `covers` range that resolves still leave the question the table
actually asks: did the degraded path **run**? The production-coverage checker introduced by
`quality-gates` reads a per-line `cargo llvm-cov` report, and it SHALL additionally require
every line of every `covers` range in `tests/degraded-coverage.toml` to be covered.

This is the one place the two halves of the repair meet, and it is why the range form is
`path:first-last` rather than a bare path: file granularity would report `src/watch.rs` as
94.74% covered and say nothing about the five uncovered lines that are the row's entire
subject.

The check SHALL name the row's `condition` in its failure message, not only the line
range, so the reader is told which degraded state is unproven rather than which lines are
cold. It runs under `make coverage` and not inside `cargo test`, because only the coverage
run produces per-line data — a stated split, so a reader does not look for it in
`tests/degraded_coverage.rs` and conclude it is missing.

#### Scenario: An uncovered degraded path fails the coverage run

- **WHEN** `make coverage` is run at HEAD after this change
- **THEN** the checker reports, for each `covers` range, whether every line in it was
  executed
- **AND** it exits non-zero for `src/watch.rs:264-268` — the `Err` arm returning the inert
  watcher — naming the row `` `notify` cannot watch the tree `` alongside the range, unless a
  test has been added that drives that arm
- **AND** adding a test that drives it returns the run to exit 0, which is the repair this
  scenario is written to force rather than to describe

#### Scenario: Deleting the test that drives a degraded path is caught

- **WHEN** the test that drives an existing, currently-covered `covers` range is removed and
  `make coverage` is run
- **THEN** the checker exits non-zero naming that row's `condition` and the now-cold range
- **AND** `cargo test --all-features` still passes on the same tree, and
  `tests/degraded_coverage.rs` still reports its binding green, which is the gap this
  requirement closes

#### Scenario: The range check cannot pass vacuously

- **WHEN** the checker is run against a map whose every `covers` array is empty, and
  separately against a report that names none of the files the ranges point at
- **THEN** it exits non-zero in both cases rather than reporting that all ranges are covered
- **AND** it exits non-zero when the map holds fewer `covers` ranges than there are rows, so
  dropping a range is a failure rather than a silent reduction in what is proved
