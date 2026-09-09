## MODIFIED Requirements

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
- **THEN** none of the three renders the checklist grammar: no progress bar appears in any of
  the six buffers, and no line begins with a checkbox glyph. The discriminator is stated as the
  absent progress bar rather than an absent `[x]`, which after this change no path emits; the
  bound test already asserts the stronger `!text.contains('[')`
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

The scenario's name is kept verbatim because a delta's scenario headers are its merge key and
OpenSpec has no scenario-level rename — `openspec validate --strict` refuses a MODIFIED block
that drops one. Its subject narrows again, to the **one** construct that stays literal, and
the three that have left the set — strikethrough, the table, and now the task-list item —
appear below as the **discriminating controls**, so the scenario fails if the narrowing did
not actually happen.

The degraded-states row this scenario proves is reworded by the same change, from "a footnote,
a task-list item" to "a footnote"; `tests/degraded-coverage.toml`'s `condition` for it is
updated to match, and its `proof` continues to name this scenario's test.

- **WHEN** a non-tracked tab whose source is exactly `See it here[^1].` / `` / `[^1]: The
  note.` — the same three lines, blank line included, that `markdown-render`'s own scenario
  names — is rendered at 78 and at 58 columns. The blank line is load-bearing rather than
  incidental: with soft breaks now folding, two adjacent source lines become one paragraph
  and the line-count assertion below would fail for a reason the scenario is not about
- **THEN** each construct appears as its own literal source text, one rendered line per source
  line, with no character dropped and none reinterpreted
- **AND** the rendered line count equals the source line count for that region — an assertion
  the reflow this change also lands would break for any source that is **not** literal, which
  is what makes it a live check rather than a tautology
- **AND** a task-list item in the same fixture renders as `[✓]`/`[ ]` rather than as its
  literal `- [x]`/`- [ ]` source, so the scenario fails if the task-list narrowing is not real
- **AND** the same source on the **tracked-tasks** tab renders the checklist grammar instead,
  which is the row's own "on a tab other than the tracked-tasks one" carve-out
- **AND** a strikethrough span and a GFM table in the same fixture render as a struck face and
  as aligned columns rather than as literal text, which discriminates the narrowing from a
  reword that changed nothing

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

