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

The structural rule in 4c is checked inside `cargo test` **and** by `make covers-check`,
which reads the map alone and needs no coverage report. That duplication is deliberate: the
hotness half of the contract lives in the coverage run, which a red suite prevents from ever
starting, so the half that can be checked without one SHALL NOT be gated behind the half that
cannot. See `quality-gates` -> "`make check` is the single gate and runs every check".

`covers` is new, and it exists because a `proof` alone proved less than it appeared to.
Measured when `covers` was introduced: `src/watch.rs`'s `Err` arms — returning the inert
watcher and the "filesystem watch unavailable" problem, a row of this very table — were
**uncovered by the coverage run**, while that row's `proof` resolved, its tier check passed,
and the map reported green. The repair landed and the row now binds
`src/watch.rs:284-288` and `src/watch.rs:294-298`, which are those arms at HEAD. A name that resolves somewhere under `src/` or `tests/` is not evidence that the
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
   **statement**. A line SHALL be treated as a declaration when it is blank, a comment, an
   item declaration (`fn`, `struct`, `enum`, `impl`, `trait`, `mod`, `use`, `const`, `static`,
   `type`) together with its generics and parameter list, an attribute, a lone delimiter, or a struct
   field or enum variant. A range SHALL be rejected when it holds no line that is none of
   these. A multi-line signature SHALL be ended by a lone delimiter that closes it: a
   bracketed item (`const STATUSES: [AgentStatus; 5] = [`) closes on `];`, which is itself a
   lone delimiter, so a scanner clearing the signature state only on a brace latches and
   swallows every following line until one happens to carry one.

   A field or variant SHALL be recognised by its **enclosing item**, not by the shape of the
   line alone. An item's extent SHALL run from its declaration line to the closing brace at
   the **same indentation** — the rule `tests/degraded_coverage.rs` already uses to find a
   function's body for the `tier = "view"` check — and `name: …` SHALL be treated as a field
   only inside a `struct` or `enum` extent. A running brace **counter** SHALL NOT be used: a
   `{` inside a string literal desyncs it for the remainder of the file, and its failure mode
   is **vacuous acceptance** — fields stop being recognised and the very range this rule
   exists to reject passes again. `scripts/coverage-prod.py` solves that on its own side by
   masking literals before counting, and its docstring records that this crate's tests hold
   `r#"{"schemaName":…}"#` fixtures; the indentation rule needs no masking because it never
   counts. A fixture holding a brace inside a string literal SHALL be among the tests, and
   **each** clause of the declaration list SHALL carry a range that only that clause rejects.
   Measured during this change's own review: the attribute, lone-delimiter and
   multi-line-signature clauses could each be deleted with the whole test binary still green,
   although every such deletion **widens** what the rule accepts. A clause nothing falsifies is
   the vacuous check this requirement exists to forbid, wearing the rule's own clothes.
   A line-local rule cannot do this and must not be written: `name: Type,` and
   `name: expr,` are the same shape, so a line-local test also classifies every struct-literal
   initialiser and every `Enum::Variant => expr,` match arm as a declaration. Both forms were
   measured against `target/llvm-cov.json`, counting a line as instrumented exactly as
   `scripts/coverage-prod.py`'s own `build_line_counts` does — a segment with `hasCount` set.
   Of 45,157 instrumented lines under `src/`, the line-local form classifies **12,114** as
   declarations (26.8%), of which 5,856 are ordinary expressions it mistakes for fields; the
   enclosing-item form classifies **6,258** (13.9%), and every one is a lone delimiter (3,642),
   an item declaration (2,597 `pub fn …`), or a signature continuation (19) — **not one field,
   attribute, or expression**, because a field declaration is not instrumented in the first
   place. The difference is the
   whole of why the rule is specified by enclosing item. The residual 16.9% is not
   misclassification: llvm-cov instruments a function's `fn` line and its closing brace, and
   those are declarations by this rule's own definition. Measured across the map as it stood
   when this rule was written — 74 ranges — the enclosing-item form rejected exactly **one**;
   the audit that followed replaced 28 of them with 33 and left 79, none rejected. Both shapes this rule exists
   to reject were live at HEAD: `src/tasks.rs:193-196` is three doc-comment lines plus
   `pub fn task_number_len(text: &str) -> usize {` — a function that reads no file — bound to
   "a tasks file exists but cannot be read"; and `src/ui/app.rs:830-843`, bound to "Herdr
   socket unreachable", was `pub struct Dashboard`'s field declarations and their doc
   comments. The earlier rule asked only for one line that was non-empty and did not begin
   with `//`, which both satisfy. This rule is **not** an approximation of what llvm-cov
   instruments, and must not be justified as one: measured against the real report,
   `src/tasks.rs:196` — a bare `pub fn` line — is instrumented with a count of 35,050, and
   `src/changes.rs:1794` with 69. The rule's subject is different and narrower. A declaration
   is where code is *named*; the row claims something about where it *runs*, so a range
   consisting only of names is answering a different question from the one its row asks. That
   catches two shapes the hotness check cannot, for two different reasons: a field-declaration
   range is uninstrumented, so `make coverage` does catch it — but only when `make coverage`
   can run, which is the scheduling half of this change; and a signature-only range is
   instrumented **and hot**, so `make coverage` never catches it at all;
5. a `tier = "view"` or `tier = "outer"` entry names a function whose **own body** does not
   name `TestBackend` — the body being the text from its `fn <name>(` line to the closing
   brace at that function's own indentation. File granularity would prove nothing: only a
   minority of the files under `src/ui/` name `TestBackend` at all — five of fourteen at HEAD — and within those a
   file-level check passes for every function in the file. A file requirement would also be
   unsatisfiable for a proof that belongs in `src/ui/detail.rs` or `src/ui/markdown.rs`, which
   `NOTABSEAM` and `MDSEAM` forbid from naming a `ratatui` type — such a proof lives in
   `src/ui/view.rs`, and this rule says so by checking the body rather than the path;
6. the table holds fewer rows than `MIN_ROWS`, or the map fewer entries — a floor, not an
   equality, so adding a degraded state is allowed and dropping the whole table is not.
   `MIN_ROWS` is **46** at HEAD (`tests/degraded_coverage.rs:17`), against a table of 59 rows.
   This requirement states the floor as the constant rather than as a literal precisely so two
   changes in flight cannot fight over the number: whichever lands second adopts the higher
   count, and neither lowers it;
6b. the map holds fewer `covers` **ranges**, summed across every row, than `MIN_RANGES`.
   `MIN_ROWS` does not bound this and never could: a map may keep every row and collapse each
   multi-range row to a single range, losing a fifth of what it names while the row floor stays
   satisfied. The only range floor before this one lived in `scripts/coverage-prod.py`, which
   requires ranges to be at least rows — so the map could fall from 79 ranges to 59 without
   either check firing, and `coverage-prod.py` would not have been run to fire: it needs a
   green suite, which is the scheduling defect this change exists to correct. `MIN_RANGES` is
   **70** at HEAD against 79 ranges, stated as the constant for the same reason `MIN_ROWS` is.

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
- **THEN** the test passes, having parsed **at least `MIN_ROWS`** rows out of `SPEC.md` and
  matched every one to exactly one entry of `tests/degraded-coverage.toml`
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
- **THEN** the test exits non-zero on the `MIN_ROWS` floor rather than reporting full coverage
  of zero rows
- **AND** the same holds when the `## Degraded states` heading is absent entirely

#### Scenario: A `proof` that is not a test fails the binding

- **WHEN** one `proof` entry is repointed at a production function that the existing
  resolver does match. The obvious candidate does **not** work: every `start` in the crate is
  written `pub fn start(` (`src/watch.rs:266`, `src/agents.rs:579`, `src/refresh.rs:187`,
  `src/launch.rs:642`), and condition 4 matches a line-anchored `fn <name>(`, so that plant
  would fail as "not defined" rather than as "not a test" — passing for the wrong reason. The
  plant is `is_usable_binary` in `src/resolve.rs`, a production function declared `fn` without
  `pub`, which the existing resolver does match and which carries no `#[test]`
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
- **AND** an entry with no `covers` key at all fails on the six-key rule, so the `unproven`
  rows cannot keep their verdict without saying where the behaviour lives
- **AND** a `covers` range naming a trivially hot line (`src/changes.rs:1-1`) satisfies the
  resolver, which is a **stated limit** rather than a hidden one: the machine checks that the
  range resolves and ran, and the `why` sentence beside it is what a reviewer reads to catch a
  range that resolves but proves nothing

#### Scenario: `unproven` gains a consequence rather than a new verdict

- **WHEN** the map is read at HEAD, where 21 of its 59 entries carry
  `verdict = "unproven"` — counts asserted against the map itself rather than restated here,
  so a row added or re-verdicted does not falsify this scenario
- **THEN** each of those entries satisfies the same `proof`-is-a-test and `covers`-resolves
  rules as every other entry, so `unproven` records what the audit found at audit time and
  no longer names a row with weaker machinery behind it
- **AND** no sixth verdict is introduced: the verdict list stays the five
  `degraded-states` established, because the defect was that `LEGAL_VERDICTS` accepted a
  value with no consequence attached, not that a value was missing

#### Scenario: A range that is only a signature or only struct fields is rejected

- **WHEN** a row's `covers` is set to `src/tasks.rs:193-196` — three doc-comment lines and
  `pub fn task_number_len(text: &str) -> usize {` — and `cargo test --all-features` is run
- **THEN** `tests/degraded_coverage.rs` exits non-zero naming that row's `condition` and the
  range, and saying that the range holds no statement
- **AND** `make covers-check` exits non-zero on the same tree with no coverage report present
- **AND** widening the range to include the function's body returns both to exit 0
- **AND** setting a row's `covers` to `src/ui/app.rs:830-843` at `bfe7e62` — `pub struct
  Dashboard`'s field declarations and their doc comments — is rejected for the same reason,
  which is the binding that stayed broken for seven task groups during `settings-window`
- **AND** both ranges passed the always-on **structural** check before this change, which is
  what the scenario is written to prevent. The struct-field range additionally failed the
  **hotness** check — uninstrumented lines are reported as "instruments no line in this range"
  — but only once `make coverage` was finally reachable, seven task groups late, which is the
  scheduling half of the same defect

#### Scenario: Dropping `covers` ranges fails a floor of their own

- **WHEN** every row's `covers` array is truncated to its first entry, so all 59 rows survive
  and `MIN_ROWS` stays satisfied, and `cargo test --all-features` runs `degraded_coverage`
- **THEN** it exits non-zero naming the **range** floor and the count it fell to, not the row
  floor
- **AND** the failure is reachable without a coverage report, so `make covers-check` reports
  it on a red suite — which is the whole reason the floor is duplicated here rather than left
  to `scripts/coverage-prod.py`'s ranges-at-least-rows rule

#### Scenario: A range of real statements is accepted whatever it starts with

- **WHEN** a row's `covers` names a range whose first line is `fn render_detail(` and whose
  remaining lines are that function's own `let ... else { return; }` guard
- **THEN** the check exits 0, because the range holds statements
- **AND** a range consisting of that `fn` line alone exits non-zero, so the rule turns on what
  the range contains rather than on which line it begins at

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
cold. The **hotness** half runs under `make coverage` and not inside `cargo test`, because
only the coverage run produces per-line data — a stated split, so a reader does not look for
it in `tests/degraded_coverage.rs` and conclude it is missing.

The **structural** half — that a range resolves, is in bounds, and holds statements — SHALL
additionally be reachable without a coverage report, as `make covers-check`, composed into
`make check` ahead of `test`. The two halves answer different questions and fail for
different reasons, and binding them into one command that needs a green suite made both
unreachable together: measured during `settings-window`, the `Herdr socket unreachable` row
pointed at `pub struct Dashboard`'s field declarations for seven task groups without being
reported, because a red acceptance test aborted `make check` before the coverage tier every
time it was run.

#### Scenario: An uncovered degraded path fails the coverage run

- **WHEN** `make coverage` is run at HEAD
- **THEN** the checker reports, for each `covers` range, whether every line in it was
  executed, and exits 0 — every range is hot, which is the post-repair state rather than the
  state that motivated the check
- **AND** planting a cold range — a row's `covers` pointed at an `Err` arm no test drives,
  `src/watch.rs:284-288` being the arm this rule was written against — makes it exit non-zero,
  naming that row's `condition` alongside the range
- **AND** restoring the range returns the run to exit 0, so the check is shown to fire rather
  than asserted to

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

#### Scenario: The structural half reports while the suite is red

- **WHEN** a unit test is edited to fail, a `covers` range is edited to name a struct's field
  declarations, and `make check` is run
- **THEN** it exits non-zero at `covers-check`, before `cargo test --all-features` runs,
  naming that row's `condition`
- **AND** the hotness half does not run at all, because no coverage report was produced
- **AND** restoring the range but leaving the test red makes `make check` exit non-zero at
  `test` instead, with `covers-check` having passed
