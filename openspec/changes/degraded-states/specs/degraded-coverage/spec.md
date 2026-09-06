## Purpose
Binds `SPEC.md`'s degraded-states table to executable proof: every row of that table names
the test that proves it and the tier that test runs at, checked inside `cargo test` so a row
added or reworded without a proof fails `make check`. Covers the coverage map, its parser and
failure conditions, and the audit verdict each row carries.

## ADDED Requirements

### Requirement: Every row of the degraded-states table is bound to a named proving test

`SPEC.md` → Degraded states is the plugin's never-fail-closed contract, and it is the last
claim in the repository with nothing forcing it to stay true. A checked-in coverage map at
`tests/degraded-coverage.toml` SHALL bind **every** row of that table to at least one named
test, and `tests/degraded_coverage.rs` — an ordinary `cargo test` target, and therefore part
of `make check` — SHALL check the binding on every run.

The map SHALL be a TOML array of tables named `row`, each carrying exactly five keys:

- `condition` — the row's **first column, verbatim**, including its backticks and its
  Markdown emphasis, so a reworded row breaks the binding rather than silently keeping it;
- `tier` — one of `view`, `outer`, `unit`, or `integration`;
- `proof` — a non-empty array of test function names, each a bare Rust function identifier;
- `verdict` — one of `confirmed`, `unproven`, `spec-corrected`, `repaired`, or `implemented`,
  the audit's own finding for that row;
- `why` — one sentence naming what the proof observes, so a reader can tell a proof that
  watches the rendered state from one that watches a value nothing renders.

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

### Requirement: Every row carries an audit verdict, and a repair names who owed it

An audit that changes nothing leaves no trace, and a later reader cannot tell a row that was
checked from one that was never looked at. Every `[[row]]` entry SHALL therefore carry a
`verdict`, and `tests/degraded_coverage.rs` SHALL fail on any entry whose `verdict` is not one
of the five, on any row whose entry is missing, and on any `repaired` entry whose `why` does
not name an earlier change.

The narrative half — the production code implementing each row, its deciding expression, and
the measurement behind each correction — SHALL live in the change's own
`notes/audit.md`. That file is prose for a reader, not a checked artifact, precisely because
archiving moves it.

An entry whose verdict is `repaired` SHALL name **which earlier change should have shipped
the behaviour**, so a defect this change fixes is visibly a defect an earlier change left
rather than scope this change invented.

#### Scenario: Every row carries one of the five verdicts

- **WHEN** `tests/degraded_coverage.rs` runs over `tests/degraded-coverage.toml`
- **THEN** every entry's `verdict` is one of `confirmed`, `unproven`, `spec-corrected`,
  `repaired`, or `implemented`
- **AND** an entry whose `verdict` is `repaired` carries a `why` naming an archived change
  directory that exists under `openspec/changes/archive/`
- **AND** changing one entry's `verdict` to `probably-fine` fails, naming that entry

### Requirement: The rows the audit found unproven at their own tier gain a proof at that tier

Twenty rows of `SPEC.md` → Degraded states describe behaviour that is real and correct in
the shipped plugin but is proved only **below** the tier the row's own wording claims: a launch
failure the row says renders as a problem row is proved only inside `src/launch.rs`; a schema
error the row says is named is proved only as a string on a vector. This requirement adds no
behaviour. It binds each such row to a proof at the tier its wording names, and the
requirements that define the behaviour stay where they are — in `schema-cli-fallback`,
`change-artifacts`, `agent-launch`, `agent-attribution`, `change-merge`, `markdown-render`, and
`pane-open`. What is new is only that the table's claim is now executable.

Every proof at the `view` tier SHALL render into a `TestBackend` at **both** mandated widths,
and SHALL be written so that it fails when the degraded state is not rendered — proved by
planting the absence, not asserted. A test that would pass against a pane rendering nothing is
not a proof of a degraded state.

#### Scenario: A schema the CLI rejects falls back per change and names the reason

- **WHEN** a `Dashboard` holding two active changes — `alpha` on a schema the CLI accepts and
  `learning-tool` declaring `outside-in-tdd`, whose `problems` carries the CLI's own
  `Unknown schema` reason — is rendered at the detail route with `learning-tool` selected, at
  120x20 and again at 60x20
- **THEN** the detail region's first content row is `! `-marked and names `outside-in-tdd`
- **AND** `learning-tool`'s artifact tabs are the ones its **file-sourced** schema produced, so
  the fall-back is per change: `alpha`'s tabs are unaffected in the same buffer
- **AND** clearing `learning-tool`'s `problems` removes the line and leaves the rest of both
  buffers byte-identical

#### Scenario: A schema that is not vendored and one that will not parse both empty the tab bar

- **WHEN** a change whose schema resolves to no local `schema.yaml` is selected and rendered at
  both widths, and separately a change whose `schema.yaml` holds bytes that are not a usable
  schema
- **THEN** the detail region's tab bar row spells `no artifacts` in both cases
- **AND** both cases show a `! `-marked content row naming the reason — the not-vendored
  path and the unparseable path both push onto `Change::problems`, which `content_lines`
  renders uniformly regardless of which of the two produced it; the two rows of the table
  are distinguished by their own wording, not by one of them staying silent
- **AND** both changes still appear in the list with their progress pair, so an unusable schema
  degrades the detail region alone

  Corrected during Change Review: the first draft of this scenario claimed the not-vendored
  case shows no problem row at all. Measured against the actual implementation
  (`an_unusable_schema_renders_no_artifacts`, `src/ui/detail.rs`): both sub-cases populate
  `Change::problems`, and group 5 renders that vector wholesale with no case-by-case
  filtering — there is no mechanism that would suppress one and not the other.

#### Scenario: A schema with no tasks artifact renders every tab as markdown and still counts

- **WHEN** a change whose schema declares three artifacts, none matching `apply.tracks` and none
  with id `tasks`, and whose directory holds a `tasks.md` counting 3 of 7, is rendered at both
  widths with each of the three tabs selected in turn
- **THEN** none of the three renders the checklist grammar: no `[x]` glyph and no progress bar
  appears in any of the six buffers
- **AND** the change's list row ends `[3/7]` in all six, so the `tasks.md` fallback count is
  live while no artifact is marked
- **AND** exactly three tabs are shown — none added, removed, or hidden

#### Scenario: An unsupported `generates` glob empties one artifact and names why

- **WHEN** a change whose schema declares two artifacts, one with `generates: "**/*.md"` and one
  with `generates: "proposal.md"`, is selected and rendered at both widths
- **THEN** the detail region's first content row is `! `-marked and names the unsupported glob
- **AND** the second artifact's tab still resolves and renders its file, so one artifact's
  failure does not empty the others
- **AND** the same dashboard with the glob replaced by `*.md` shows no `! `-marked row, which is
  the discriminating control

#### Scenario: A tasks file that cannot be read is zero tasks with a named reason

- **WHEN** a change whose tasks artifact resolves to a **directory** where a file was expected
  is rendered at both widths, and separately one whose tasks file holds invalid UTF-8
- **THEN** both list rows read `[0/0]` and both detail regions carry a `! `-marked row naming
  the path and the reason
- **AND** the invalid-UTF-8 case's reason is distinguishable from the directory case's, so the
  row's four named causes are not collapsed into one message
- **AND** neither pane refuses to draw: every other change in the list renders normally

#### Scenario: Each of the six launch failures renders as a leading problem row

- **WHEN** `run_wired` is driven at 120x20 and again at 60x20 against a scratch `herdr` program
  that fails, in six separate runs, at (a) `pane split` with a domain error, (b) `pane split`
  with a payload that is not the measured envelope, (c) `agent start`, (d) `agent prompt`,
  (e) `state::record` after a successful start, and (f) a derived name already live in the
  session — with a real `a` keypress driving each
- **THEN** each run's list region shows the reason as its **first** interior row, `! `-marked,
  above every change row and above any watcher or change-set problem in the same buffer
- **AND** run (f)'s row reads `` `c-2fa-support` is already running for this change - press g to
  focus it `` in full at 120 columns, the exact sentence `agent-launch` specifies, truncated at
  60
- **AND** run (f)'s scratch `herdr` log is **empty**: the refusal is made before any Herdr call
- **AND** each run's log holds exactly the calls its row's table entry names, so the six are
  distinguishable in what they left behind as well as in what they rendered

#### Scenario: `g` with no attributed agent changes nothing the pane shows

- **WHEN** `run_wired` is driven at both widths with a reachable scratch `herdr`, a change with
  no attributed agent selected, and an event source pressing `g` and then `q`
- **THEN** the buffer is byte-identical to the same run pressing only `q`
- **AND** the scratch `herdr` log holds only `agent list` runs — no `agent focus`
- **AND** `launch.problems` is empty: `g` on an unattributed change records nothing, which is
  what distinguishes it from a refusal

#### Scenario: A CLI root disagreement and a non-zero exit both leave the file numbers standing

- **WHEN** the worker's `openspec list --json` reports a repository root other than the one the
  plugin resolved, and separately exits non-zero after the file-sourced result was already
  adopted
- **THEN** in both cases the list still shows every change with the progress the file read
  produced, at both widths
- **AND** the non-zero case shows a `! `-marked row naming the command and its exit code, and
  **not** the CLI's own diagnostic, which goes to stdout and is unavailable
- **AND** the root-disagreement case discards the whole CLI result rather than merging part of
  it: no change's schema, progress, or artifact list changes in the buffer

#### Scenario: A watcher failure and a mid-run removal both keep the loop drawing

- **WHEN** `run_wired` is driven at both widths over a repository whose root cannot be watched,
  and separately over one whose `openspec/` is removed while the loop runs
- **THEN** each shows the watcher's own reason as a `! `-marked row above every change-set
  problem row, and below a launch problem row when one is present
- **AND** the removal case's change set is empty with **no** problem of its own — a missing
  `openspec/` means "not an OpenSpec repository" — while the watcher's disconnect reason is
  what the row names
- **AND** `r` still forces a full refresh in the watcher-failure case, proved by the scratch
  `openspec` program's log gaining an invocation after the keypress

#### Scenario: An out-of-scope agent and a worktree agent are both invisible

- **WHEN** a `Dashboard` is rendered at both widths with three polled agents: one whose `cwd` is
  absent, one whose `cwd` is `/tmp/some-other-repo`, and one whose `cwd` is
  `<repo-parent>/.worktrees/<repo>-feature` — the shape Herdr places a linked worktree at
- **THEN** no row carries a badge cell and the footer reports **no** unattributed count
- **AND** the buffer is byte-identical to the same dashboard with no agents at all
- **AND** a fourth agent whose `cwd` is the repository root **does** badge its change in the
  same fixture, which is the discriminating control that the emptiness is containment's and not
  the fixture's

#### Scenario: A duplicate artifact id is accepted by this crate and stays file-mode

- **WHEN** `schema::parse` is given a `schema.yaml` whose YAML declares the id `tasks` at two
  positions
- **THEN** it returns a usable schema carrying both, in declaration order, rather than an error
- **AND** a `Dashboard` built from it renders both tabs at both widths, addressed by position
- **AND** the parse is driven from **YAML bytes**, not from a hand-built `Schema` value, so the
  claim is about the parser the CLI disagrees with

#### Scenario: A footnote, strikethrough, and a table each render as literal source

- **WHEN** a non-tracked tab whose source holds, on separate lines, a footnote reference and
  definition, a strikethrough span, a GFM table row, and a task-list item is rendered at 78 and
  at 58 columns
- **THEN** each construct appears as its own literal source text, one rendered line per source
  line, with no character dropped and none reinterpreted
- **AND** the rendered line count equals the source line count for that region
- **AND** the same source on the **tracked-tasks** tab renders the checklist grammar instead,
  which is the row's own "on a tab other than the tracked-tasks one" carve-out

#### Scenario: The one-shot commands' degrades are proved by exit status and stderr

- **WHEN** `herdr-openspec open` is run as a real process with no `HERDR_WORKSPACE_ID` in its
  environment, with a stub `herdr` first on `PATH` whose every invocation is logged
- **THEN** it exits `1`, its stderr names `HERDR_WORKSPACE_ID` **and** states that the command
  must be invoked from Herdr — two separate assertions, because the first substring is
  satisfied by the variable name alone and would pass on a message that said nothing else
- **AND** the stub's log is **empty**: no Herdr call is made, which the row claims and nothing
  asserted before
- **AND** the same command with the variable set, a `pane list` that fails, and a `plugin pane
  focus` that exits 2 warns on stderr and still opens exactly once, while a `focus` failing with
  a domain error stops with exit 1 and the reason on stderr
