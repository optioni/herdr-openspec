# doc-conformance Specification

## Purpose
Binds the project's own documents to the code they describe. Every documented claim that
has a computable *second site* — the crate's module list, its production worker threads,
its declared MSRV, the programs `make check` invokes, the plugin manifest, and the project
context injected into every OpenSpec agent's prompt — is checked inside `cargo test`, so a
claim that drifts fails `make check` rather than surviving until an audit finds it.

## Requirements

### Requirement: Documented claims with a second site are checked inside `make check`

`SPEC.md` is this project's precedence authority — where prose disagrees with it, it wins —
so a stale claim in it is a wrong contract, not a typo. The project already has two tests of
exactly this shape: `tests/manifest.rs` binds `herdr-plugin.toml`, `README.md`, and the
binary name to each other, and `tests/degraded_coverage.rs` binds `SPEC.md`'s degraded-states
table to named proving tests. This capability extends that tier.

A checked-in test target at `tests/doc_contract.rs` — an ordinary `cargo test` target, and
therefore part of `make check` — SHALL check every claim named in the requirements below on
every run. It SHALL be confined to claims with a **second site**: a value or set the
repository itself determines, whose disagreement with the prose is otherwise silent. It
SHALL NOT assert on prose that has no such site.

Every failure SHALL name the document, the claim, and both sides of the disagreement, so the
reader is told what to correct rather than only that something is wrong.

The check SHALL read only files inside the repository and SHALL spawn no process: no
`openspec` binary, no `herdr`, no `cargo metadata`. `openspec` and `herdr` are optional at
runtime and absent from both CI runners, so a check depending on either could not live in
`make check` at all — the same reasoning that put `tests/spec_purposes.rs` inside
`cargo test` rather than behind `openspec validate --specs --strict`.

#### Scenario: A document is missing entirely
- **WHEN** the check runs against a repository in which `SPEC.md` does not exist
- **THEN** the check fails naming the missing file by path
- **AND** it does not pass vacuously by treating an unreadable document as agreeing

#### Scenario: A bound section's heading no longer exists
- **WHEN** `SPEC.md` exists but the heading a leg parses from (for example `**Module map:**`)
  has been renamed or deleted
- **THEN** that leg fails naming the heading it searched for
- **AND** the failure distinguishes "the section is gone" from "the section disagrees"

#### Scenario: Every leg is proved able to fail
- **WHEN** a defect is planted in the second site or in the document for each leg in turn —
  a module added to `src/lib.rs` and not to the map, a wrong MSRV, a reordered manifest block
- **THEN** the corresponding leg fails on each planted defect
- **AND** the defect is reverted, leaving the repository green

### Requirement: The module map names exactly the crate's public modules

`SPEC.md`'s **Module map** table is what a reader consults to find a module. It SHALL name
exactly the set of modules `src/lib.rs` declares `pub mod`, no more and no fewer.

The check SHALL parse the first column of every data row of the table introduced by
`**Module map:**` in `SPEC.md`, strip its backticks, and compare that set against the module
names in `src/lib.rs`'s `pub mod <name>;` declarations. It SHALL report a set difference in
both directions.

Today `src/lib.rs` declares thirteen public modules and the table has twelve rows: `open` —
which owns the `open` and `open-tab` subcommands, is the third `HerdrCli` consumer, and holds
two of the crate's three binary entry points — is described elsewhere in `SPEC.md` but never
reaches the map. The change adds its row.

#### Scenario: A module exists with no map row
- **WHEN** `src/lib.rs` declares `pub mod open;` and `SPEC.md`'s module map has no `open` row
- **THEN** the check fails naming `open` as declared but unmapped

#### Scenario: A map row names no module
- **WHEN** the module map carries a row for a module `src/lib.rs` no longer declares
- **THEN** the check fails naming that row as mapped but undeclared

#### Scenario: The map and the crate agree
- **WHEN** every `pub mod` name has exactly one row and every row names a declared module
- **THEN** the leg passes

### Requirement: Every public module is named in the tested-modules list

`SPEC.md` → Testing and quality gates → **Unit-tested modules** is the reader's index of what
is tested and how. Every module `src/lib.rs` declares SHALL be named in that section as a
`<module>::` token.

The check SHALL take the section between the `### Unit-tested modules` heading and the next
`### ` heading, and SHALL fail for every declared module name that does not appear in it as
`<name>::`.

Today **four** of the thirteen declared modules fail this: `launch::` appears **nowhere** in
`SPEC.md` — a 1,612-line module with no entry — and `open::`, `config::`, and `state::` appear
only outside this section. The audit named `launch` and `open`; writing the check found
`config` and `state` as well, which is the mechanism doing its job before it has shipped. The
change adds a bullet for each of the four.

#### Scenario: A module has no bullet
- **WHEN** `src/lib.rs` declares `pub mod launch;` and the tested-modules section contains no
  `launch::` token
- **THEN** the check fails naming `launch` and the section it searched

#### Scenario: A mention outside the section does not satisfy the leg
- **WHEN** `open::context` is named in `SPEC.md` § Opening the pane but not in the
  tested-modules section
- **THEN** the check still fails for `open`, because the leg is scoped to the section

### Requirement: The documented worker-thread count equals the crate's production thread sites

`SPEC.md` states the count of the crate's worker threads inline. That count SHALL equal the
number of files under `src/` whose **production slice** names both `std::thread::spawn` and
`mpsc`.

**Corrected during this change's own implementation, superseding the rule as originally
planned.** The rule was first stated as "names `std::thread::spawn`" alone, with
`src/cli.rs` carved out by a separate sentence ("`src/cli.rs` names `thread::spawn` only in
its test slice and so SHALL NOT be counted"). `seam-resilience` (commit `137d21b`) landed
after that text was written and added two per-invocation stdout/stderr pipe-drain threads to
`src/cli.rs`'s production slice, above its first `#[cfg(test)]`. Measurement, not the plan,
won: the two clauses could no longer both be true, because `src/cli.rs` now names
`thread::spawn` in production too. Re-stating the carve-out as "`src/cli.rs` is an
exception" would have made the rule ad hoc; instead the rule itself is corrected to the
discriminator that was true all along for the files that matter — a worker thread answers
over a channel, a fire-and-forget pipe pump does not.

The production slice of a source file is the text before its first line equal to
`#[cfg(test)]` — the same cut `NOBLOCK`'s leg 3 already uses, so this leg and that gate agree
on what "production" means. `thread::spawn` is matched anchored per line (no `/` character
before it on that line), excluding a comment or doc-comment mention. `mpsc` is matched as a
plain, unanchored substring of the production slice — deliberately: this is not a new
discriminator invented for this check, it is `scripts/gates/noblock.sh`'s own Guard A, its
positive control for leg 1 (`prod src/refresh.rs | grep -qE 'mpsc'` alongside `grep -qE
'thread::spawn'`), reused on the same reasoning that a worker thread answers over a channel.

`src/cli.rs`'s production slice spawns two threads (a stdout-drain and a stderr-drain, each
joined via `JoinHandle::join()` before the invocation returns) but names no `mpsc` — they are
per-invocation pipe pumps, not workers that answer the render loop over a channel — and is
correctly excluded. `src/watch.rs` names `mpsc` (the type the debounce classifier's caller
receives from) but spawns no thread of its own — `notify` spawns its background thread — and
is correctly excluded from the other direction.

The count SHALL be written in `SPEC.md` in a form the check can find: `crate's <number-word>
worker threads`, where the number word may optionally be wrapped in `**` emphasis. The
emphasis is optional deliberately — at HEAD the phrase reads `crate's two worker threads`
with no markup, so a parser requiring `**` would report "the claim could not be located"
rather than "the count is wrong", which is the correct failure for the wrong reason. The
check SHALL fail when the number disagrees, when no occurrence of the phrase exists, and when
two occurrences state different numbers.

Today the production slice of `src/refresh.rs`, `src/agents.rs`, and `src/launch.rs` each
names both `thread::spawn` and `mpsc` — **three** — while `SPEC.md` said two before this
change fixed it. `AGENTS.md` and `openspec/IMPLEMENTATION-ORDER.md` already said three.

#### Scenario: The documented count is stale
- **WHEN** three production files name `thread::spawn` and `SPEC.md` says "two worker threads"
- **THEN** the check fails reporting documented 2 against computed 3, naming the three files

#### Scenario: A fourth worker thread is added
- **WHEN** a new module's production slice names `thread::spawn` and `SPEC.md` is not updated
- **THEN** the check fails reporting documented 3 against computed 4

#### Scenario: The bound phrasing is removed
- **WHEN** `SPEC.md` no longer states the count in the form the check parses
- **THEN** the check fails saying the claim could not be located, rather than passing

### Requirement: The documented minimum Rust version is the manifest's

The crate's supported floor is `Cargo.toml`'s `rust-version`. Wherever a document states a
required Rust version, that document SHALL name the manifest's value.

The check SHALL read `rust-version` from `Cargo.toml` by parsing the file as TOML — never by
spawning `cargo metadata` — and SHALL require that string to appear in `AGENTS.md`'s
Environment section and in `README.md`'s Development section. It SHALL search **within those
sections**, not the whole file, and SHALL require the version to be delimited by a
non-version character on each side, so a match inside an unrelated number (`11.887`) does not
satisfy it.

Today `Cargo.toml` declares `rust-version = "1.88"` and `AGENTS.md` says "**Rust** stable
(1.91+ at time of writing)", which reads as a requirement; the crate's actual minimum appears
in no prose anywhere. The change states both: the supported floor, and the version the
reference machine runs.

#### Scenario: The MSRV is documented nowhere
- **WHEN** `Cargo.toml` declares `rust-version = "1.88"` and neither `AGENTS.md` nor
  `README.md` contains `1.88`
- **THEN** the check fails naming the document that omits it and the value it must name

#### Scenario: The MSRV is raised without a doc edit
- **WHEN** `Cargo.toml`'s `rust-version` becomes `1.92` and the documents still say `1.88`
- **THEN** the check fails reporting the manifest value against the documented one

### Requirement: Every non-cargo program `make check` invokes is documented as a prerequisite

A contributor who reads only `README.md` must be able to run `make check`. Every external
program the `make check` path invokes that is not `cargo` and not a repository script SHALL
be named in `README.md`'s Development section and `AGENTS.md`'s Environment section.

The check SHALL parse the `Makefile`'s `check:` target and follow its prerequisite targets. A
real `Makefile` recipe is not a list of bare commands, so the extraction rule SHALL be stated
exactly and SHALL be driven by the shapes this `Makefile` actually contains:

1. Join continuation lines (a recipe line ending in `\`) with the line that follows, so a
   multi-line shell construct is one logical line.
2. Strip a leading `@` (Make's echo-suppression prefix) and a leading `-`.
3. Split the logical line into tokens **quote-aware**: a single- or double-quoted run is one
   token, so `ENTRY='pub fn run_from_env\('` does not become four tokens.
4. Strip leading assignments matching `^[A-Za-z_][A-Za-z0-9_]*=` and a leading `env` with its
   own options and assignments.
5. Take the first remaining token as a candidate program.
6. Discard `cargo`; discard any `/bin/sh` (or `sh`) invocation whose next token is a path under
   `scripts/`; and discard shell control-flow keywords and builtins — `if`, `then`, `else`,
   `elif`, `fi`, `for`, `do`, `done`, `while`, `case`, `esac`, `echo`, `exit`, `test`, `[`,
   `:`, `cd`, `set`, `true`, `false`.

Step 6's keyword list is not decoration: the `lint:` and `coverage:` recipes each open with an
`@if ! cargo … --version >/dev/null 2>&1; then echo …; exit 1; fi` guard, and without it those
recipes would yield `if`, `echo`, `exit`, and `fi` as programs a reader would be told to
document.

It SHALL additionally require that `python3`, if named anywhere under `scripts/gates/`, is
documented on the same terms — a gate script may invoke an interpreter no `Makefile` line
names.

The check SHALL match a documented program as a **whole word** inside the relevant document's
own section, never as a bare substring anywhere in the file, so a program name occurring
incidentally in English prose does not satisfy it.

Today the computed set is exactly `{python3}`: the `gates:` recipe ends with
`python3 scripts/gates/gate-mech1.py`, and ten files under `scripts/gates/` reach for
`python3` internally (`grep -l python3 scripts/gates/* | wc -l` → 10). `AGENTS.md` documents
it; `README.md` does not. The change adds it to `README.md`, alongside the hygiene-gate tier
that file's gate list omits.

#### Scenario: An invoked interpreter is undocumented
- **WHEN** `make check` reaches `python3 scripts/gates/gate-mech1.py` and `README.md` does not
  contain `python3`
- **THEN** the check fails naming `python3` and `README.md`

#### Scenario: A gate script's own interpreter is undocumented
- **WHEN** the `Makefile` line naming `python3` is removed but `scripts/gates/deps.sh` still
  invokes it, and `README.md` does not name it
- **THEN** the check still fails, because the second sub-leg scans the gate scripts

#### Scenario: A shell guard block yields no program
- **WHEN** the extractor reaches the `lint:` recipe's `@if ! cargo clippy --version
  >/dev/null 2>&1; then echo "error: …" 1>&2; exit 1; fi` guard
- **THEN** it yields no candidate program from those lines
- **AND** `if`, `echo`, `exit`, and `fi` are never reported as undocumented prerequisites

#### Scenario: A quoted assignment value is one token
- **WHEN** the extractor reaches
  `LAUNCH=src/open.rs ENTRY='pub fn run_from_env\(' /bin/sh scripts/gates/launchseam.sh`
- **THEN** it discards the line as a `/bin/sh` invocation of a `scripts/` path
- **AND** it never reports `fn` as a program, which a whitespace-split tokenizer would

#### Scenario: A new external tool joins the gate path
- **WHEN** a future recipe line invokes a program that neither document names
- **THEN** the check fails naming that program

#### Scenario: The extractor yields nothing
- **WHEN** the extraction rule computes an empty program set from the `Makefile`
- **THEN** the check fails saying the extractor found no program, rather than passing
  vacuously because there was nothing to look for

### Requirement: `SPEC.md`'s transcription of the plugin manifest matches the manifest

`SPEC.md` reproduces `herdr-plugin.toml` in a fenced TOML block. That transcription SHALL
parse to the same TOML document as the manifest file, and SHALL present its `[[…]]` tables in
the same order.

The check SHALL locate the fenced `toml` block in `SPEC.md` that contains `id = "herdr-openspec"`,
parse it and `herdr-plugin.toml` with the crate's existing `toml` dependency, and compare the
parsed values for equality; separately it SHALL compare the ordered sequence of `[[header]]`
lines in the two sources.

Value equality is the load-bearing half: a divergent `command`, `id`, `title`, `placement`, or
`contexts` in the transcription would send a reader to build the wrong thing, and a TOML
`Value` is keyed, so nothing in the value comparison can see cross-table ordering.

Order is bound by a **separately named** test, so a failure says plainly which half fired and
the remedy is unambiguous: move one block in `SPEC.md`. TOML array-of-table order is not
semantic, so an order-only difference is cosmetic; binding it is a deliberate,
consciously-accepted strictness (design.md → Decision 4) whose whole cost is that one block
move, and whose benefit is that no future audit re-derives whether the difference matters.
Today the two agree byte-for-byte on key/value content and
differ only in order: `SPEC.md` shows `[[build]]`, both `[[actions]]`, then both `[[panes]]`;
the manifest has `[[build]]`, both `[[panes]]`, then both `[[actions]]`. The change reorders
the `SPEC.md` block; **the manifest file is not edited**.

This leg SHALL NOT duplicate `tests/manifest.rs`, which binds the manifest to `README.md` and
to the Cargo binary name. This leg binds `SPEC.md`'s copy of the manifest to the manifest.

#### Scenario: A transcribed value diverges
- **WHEN** `herdr-plugin.toml` gains a `[[panes]]` entry and `SPEC.md`'s block does not
- **THEN** the check fails reporting the value present in one source and absent from the other

#### Scenario: The blocks differ only in table order
- **WHEN** the two sources parse equal but present `[[actions]]` and `[[panes]]` in a
  different order
- **THEN** the check fails naming the two orderings

#### Scenario: The transcription is absent
- **WHEN** `SPEC.md` contains no fenced `toml` block carrying `id = "herdr-openspec"`
- **THEN** the check fails saying the transcription could not be located

### Requirement: The injected project context describes the repository it ships with

`openspec/config.yaml`'s `context` block is injected verbatim into every OpenSpec agent's
prompt in this repository, so a false claim in it propagates into every future change rather
than sitting inert in a document nobody reads. Its claims about this repository SHALL be true
of this repository.

Two claims have a second site and SHALL be checked:

1. **Fixtures.** The block SHALL NOT describe checked-in fixture repositories while none
   exist. The check SHALL treat a fixture repository as a directory under `tests/fixtures/`
   containing an `openspec/` subdirectory, and SHALL fail if the block contains the phrase
   `fixture repositories` while no such directory exists. Today `tests/fixtures/` holds five
   markdown task files and `build-graph.txt`, and `SPEC.md` § Fixtures states outright "Three
   mechanisms, **not** checked-in fixture repositories" — the config block contradicts the
   design contract it is supposed to summarise.

2. **The gate set.** For every prerequisite target of the `Makefile`'s `check:` target, the
   block SHALL name either `make <target>` or that target's own recipe command. A target
   whose recipe is more than one command line — `gates`, with more than thirty — SHALL be
   satisfied **only** by `make <target>`: naming one of its lines would let a future author
   satisfy the letter of the check while leaving the other thirty scripts unrepresented in
   the injected prompt. Today the block lists four commands and omits `make gates`, the
   hygiene-gate tier, entirely.

#### Scenario: The context claims a fixture tier that does not exist
- **WHEN** no directory under `tests/fixtures/` contains an `openspec/` subdirectory and
  `openspec/config.yaml` says "fixture repositories under `tests/fixtures/`"
- **THEN** the check fails naming the phrase and reporting that no fixture repository was found

#### Scenario: A gate tier is missing from the injected context
- **WHEN** `check:` lists `gates` among its prerequisites and the context block names neither
  `make gates` nor the `gates` recipe
- **THEN** the check fails naming the unrepresented target

#### Scenario: A fixture repository is later added
- **WHEN** a change adds `tests/fixtures/<name>/openspec/` and updates the context block to
  describe it
- **THEN** the fixtures leg passes, because the claim is now true

### Requirement: The dashboard's runtime boundaries are unaffected

This capability adds a test target and corrects prose. It SHALL NOT change any rendered
state, any keybinding, any manifest value, any configuration key, or any code under `src/`.

The dashboard boundary cases this project's spec rules enumerate — among them no `openspec/`
directory, no active changes, a missing artifact file, a schema the CLI rejects, an
unreachable Herdr socket, and a pane narrower than 100 columns — are untouched by this
change, as are the other rows of `SPEC.md`'s 44-row degraded-states table, which is
unchanged. The argument is structural rather than an enumeration: no file under `src/` is
edited, so no rendered state can move. The `tests/degraded_coverage.rs` binding
of that table SHALL continue to pass unmodified.

#### Scenario: The dashboard is unchanged
- **WHEN** the change is implemented and `make check` runs
- **THEN** every pre-existing test passes with no assertion edited
- **AND** `git diff` reports no file under `src/` modified

#### Scenario: The degraded-states binding still holds
- **WHEN** `tests/degraded_coverage.rs` runs after the change
- **THEN** it passes with `tests/degraded-coverage.toml` unmodified

### Requirement: `SPEC.md`'s mouse table is bound by executing `mouse_action`, row by row

`documented_mouse_actions` parses `SPEC.md` → Keys' mouse table for backticked `Action::`
names, `mouse_action_body` parses `src/ui/driver.rs` for the same, and
`mouse_bindings_match_spec_md` asserts the two **sets** are equal. That check cannot see a row
whose *prose* is wrong, because prose is not in either set. `mouse-text-selection` left four
false statements in the table past a green `make check` (`git show 9beadc2 -- SPEC.md`).

The information needed is largely computed and then discarded. `sweep_mouse_actions` executes
`mouse_action` at **every cell** of three frames — the wide frame at `Route::List`, the narrow
frame at both routes — across **fourteen** `MouseEventKind` values, and reduces the result to
a `BTreeSet<String>` of action names, discarding everything that distinguishes one documented
row from another.

`tests/doc_contract.rs` SHALL retain that discarded detail and bind the table to it.

**The claim.** The sweep SHALL produce a set of **claims**. A claim carries four axes:

1. the `MouseEventKind` dispatched;
2. the **overlay state**, as an explicit axis — not a value written into the zone slot;
3. the `Zone` the cell resolved to, as its **variant name**, payload discarded;
4. the **outcome**: the `Action` variant name **together with its payload's constructor
   name** where it has one — `Click(Change)`, `Click(Section)`, `Click(DetailHeader)`,
   `Select(Begin)`, `Select(Extend)`, `SelectTab`, `Ignore`.

Axis 4 is load-bearing and SHALL NOT be reduced to the bare variant name. `action_name`
collapses every `Click(_)` to `"Click"`, and under that collapse three separate documented
rows — a click on a change row, a second click on the row already selected, and a click on a
section header — all reduce to one claim, as do the artifact-section-header click and the
content-row press that `mouse-text-selection` confused. Measured at HEAD, the bare-name form
yields 104 distinct triples of which 18 are non-`Ignore`; the stale row that change actually
deleted claims one of them, so under the bare-name form it is **not** vacuous and the check
passes. The payload discriminant is what makes the vacuity direction able to fire at all.

**Where the zone comes from.** `mouse_action` returns an `Action` and never surfaces the
`Zone` it resolved, so the sweep SHALL recover it by calling `ui::layout::zone` on the same
arguments. That recomputation SHALL mirror `mouse_action`'s own precedence: a point outside the
frame SHALL be `Zone::Outside`, and while the overlay is open the zone SHALL NOT be consulted
at all, because `mouse_action` returns before reaching it. A recomputation that disagrees with
that precedence would attribute real behaviour to the wrong zone and is the one way this axis
can be silently wrong.

**The dashboard-fixture axis.** `mouse_action` is a total function of a `Dashboard`, a `Rect`
and a `MouseEvent` — the `Dashboard` included. Its `Drag(MouseButton::Left)` arm returns
`Action::Select` from `Zone::ListRow`, `DetailTab`, `List`, `Detail` and `Outside` when
`dashboard.selection.is_some()`, and `Action::Ignore` otherwise; today's single fixture pins
`selection: None`, so the clamp behaviour the table documents is never observed and a row
describing it would be reported vacuous and be right to. The sweep SHALL therefore run over an
**explicitly listed set of dashboard fixtures**, at minimum one with no selection and one with
a selection present, and the set's length SHALL be asserted in the check's own source on the
terms `EXEMPT_ACTIONS` is pinned at two. A row resting on a dashboard precondition no fixture
varies SHALL NOT be tolerated silently: either the fixture set gains it, or the row is named in
a pinned, counted exemption list.

**The documented set.** Each row SHALL name, in backticks, **one or more** `Zone` variants —
the wheel rows legitimately span several, `Zone::List` and `Zone::ListRow` for the list region
and `Zone::Detail`, `Zone::DetailTab` and `Zone::DetailRow` for the detail region — together
with the outcome in axis-4 form, naming the payload constructor where the `Action` has one. A
row's claims are the cross product of its gestures, its zones and its outcomes. A backticked
`Zone` token that names no real variant SHALL be an error naming the row and the token, never
left to surface as an unexplained vacuity.

A row's **gesture** SHALL be derived from a **closed, explicitly-listed** vocabulary mapping a
leading phrase of the Gesture cell to one or more `MouseEventKind` values. The vocabulary SHALL
cover the phrases the real table uses — `Wheel down`, `Wheel up`, `Left click`, `Left drag`,
`Anything else` — SHALL be pinned by length, and SHALL error by row and phrase on anything
unlisted. Semantic parsing of the cell's English SHALL NOT be attempted.

**The catch-all row.** **Exactly one** row SHALL be the catch-all: the row that names no
`Zone` and whose only outcome is `Ignore`. It SHALL claim exactly those claims no other row
claims, it SHALL claim at least one, and a **second** zone-less `Ignore`-only row SHALL be an
error rather than a silently accepted duplicate. The catch-all's own prose is **not** parsed,
which is the reason two of `mouse-text-selection`'s four false statements — both inside that
row — are out of this check's reach.

**The assertion.** The check SHALL hold in **both** directions:

1. every claim the sweep observed is covered by at least one row; one covered by none SHALL
   fail as **undocumented**, naming the gesture, the overlay state, the zone and the outcome;
2. every row covers at least one observed claim; a row covering none SHALL fail as
   **vacuous**, naming the row's own text and the claims actually observed at that row's zones,
   so the reader is told what the row should have said.

Both directions SHALL be reported when both fail, rather than the first masking the second.

**The admitted limitation.** Two rows may legitimately share one claim, and the vacuity
direction cannot tell them apart. At HEAD the one such pair is the click on a change row and
the second click on the row already selected: both produce `Click(Change)`, because "a second
click opens the detail" is decided in `Dashboard::apply`, not in `mouse_action`. Such pairs
SHALL be listed by name in the check's own source and pinned by count, so a third row joining
the collision fails rather than passing unnoticed.

The existing name-set equality SHALL be kept as the check's **first** leg, so a plain
vocabulary mismatch is reported in its existing terms before the stricter legs run.

`CLAIM_COUNT` SHALL remain **13**: this strengthens an existing claim rather than adding a new
one, so the three sites pinned to that count do not move.

#### Scenario: A row describing a removed binding fails as vacuous

- **WHEN** the check runs against a copy of the tree in which the mouse table carries a row
  reading `` | Left click on any other row of a foldable artifact's content | `Action::Click`
  naming `Target::DetailLine` in `Zone::DetailRow` | `` — the row `mouse-text-selection`
  actually deleted — while `mouse_action` produces `Select(Begin)` there
- **THEN** the check fails naming that row as covering no observed claim
- **AND** the failure names the claims actually observed at `Zone::DetailRow` under a left
  press — `Click(DetailHeader)` and `Select(Begin)` — so the reader is told what the row
  should have said
- **AND** leg 1, the name-set equality, is **not** what fails: `Action::Click` is still
  produced elsewhere in the table and in `mouse_action` alike, which is exactly why the
  shipped check passed this row

#### Scenario: Two rows that differ only in their payload are told apart

- **WHEN** the check runs against a copy of the tree in which the row documenting the click on
  a section header names `Target::Change` instead of `Target::Section`
- **THEN** the check fails: no observed claim pairs `Zone::ListRow` with `Click(Section)` any
  longer from that row, and the row claiming `Click(Change)` twice does not cover it
- **AND** this scenario fails under the axis-4 form and passes under the bare-variant form,
  which is the difference the payload discriminant exists to make

#### Scenario: A binding added with no row fails as undocumented

- **WHEN** the check runs against a copy of the tree in which the observed claim set carries
  `(Down(Middle), overlay closed, Zone::List, ToggleHelp)`, and no table row covers it
- **THEN** the check fails naming the gesture, the overlay state, the zone, and `ToggleHelp`
- **AND** the catch-all row does not absorb it, because the catch-all covers only claims whose
  outcome is `Ignore`

#### Scenario: The table and the pane agree at HEAD

- **WHEN** the check runs against the tree at the end of this change
- **THEN** every leg passes
- **AND** all fourteen `MouseEventKind` values appear in the observed claim set
- **AND** all six `Zone` variants appear in it, and the overlay-open axis is represented
- **AND** the **two** wheel-over-an-open-overlay rows added by this change cover
  `(ScrollDown, overlay open, ScrollDown)` and `(ScrollUp, overlay open, ScrollUp)`, which
  before this change were bound by no row at all. They are two rather than one because the
  gesture vocabulary is closed and maps `Wheel down` and `Wheel up` to one `MouseEventKind`
  each: a single row would claim `(ScrollDown, overlay open, ScrollUp)`, which is not
  observed, and leave `(ScrollUp, overlay open, ScrollUp)` uncovered
- **AND** no row is vacuous and no claim is uncovered, and the catch-all covers at least one

#### Scenario: The clamp is observed rather than reported vacuous

- **WHEN** the check runs at HEAD with the dashboard-fixture axis in place
- **THEN** the fixture whose `selection` is present yields
  `(Drag(Left), overlay closed, Zone::List, Select(Extend))` among its claims
- **AND** the table's drag row, which documents the clamp, covers it and is not vacuous
- **AND** with the selection-absent fixture alone that claim is absent, which is the state the
  check would have shipped in without this axis

#### Scenario: An unrecognised gesture phrase is an error, not a silent skip

- **WHEN** the check runs against a copy of the tree in which a Gesture cell begins
  `Left quadruple-click`, a phrase the vocabulary does not list
- **THEN** the check fails naming the row and the unrecognised phrase
- **AND** the row is not skipped, so a typo cannot quietly remove that row from both
  directions of the assertion

#### Scenario: A mistyped zone token is named rather than left as an unexplained vacuity

- **WHEN** the check runs against a copy of the tree in which a row names `` `Zone::DetailRows` ``
- **THEN** the check fails naming the row and the token, and states that it matches no `Zone`
  variant
- **AND** the message is not the vacuity message, which would point the reader at the wrong
  problem

#### Scenario: Zone-less rows are rejected, and a second catch-all is an error

- **WHEN** the check runs against a copy of the tree in which a row naming `Action::SelectTab`
  carries no backticked `Zone`
- **THEN** the check fails naming that row as missing its zone, and states that only the single
  `Ignore`-only catch-all may omit one
- **AND** when a second zone-less `Ignore`-only row is added, the check fails naming both rows,
  rather than accepting one and ignoring the other

#### Scenario: A gutted table fails as a broken control

- **WHEN** the check runs against a copy of the tree in which the mouse table is reduced to its
  header row and separator row, and again against one in which the `| Gesture | Action |`
  header is deleted outright
- **THEN** both runs fail with a message naming the empty table or the missing header
- **AND** neither reports a pass, on exactly the terms `documented_mouse_actions` already holds
  to — an empty documented set is never compared against an empty observed set

#### Scenario: The overlay axis keeps the two passes apart

- **WHEN** the check runs against a copy of the tree in which the row documenting the dismissing
  click outside the help band names the overlay-closed state instead of overlay-open
- **THEN** the check fails, because no closed-overlay cell produces `ToggleHelp` and the
  open-overlay claims are left uncovered
- **AND** both failures are reported — the row as vacuous and the overlay claims as
  undocumented — rather than one masking the other

#### Scenario: An empty catch-all fails rather than passing silently

- **WHEN** the comparator is called with a hand-built claim set containing no `Ignore` claim
  at all, against the real row set
- **THEN** the check fails naming the catch-all row as covering nothing
- **AND** this control is driven with a synthetic claim set rather than the real tree, where
  the great majority of claims are `Ignore` and the rule could never fire — an assertion that
  cannot go red is not a guard

#### Scenario: A third row joining a known collision fails

- **WHEN** the check runs against a copy of the tree in which a third row also claims
  `(Down(Left), overlay closed, Zone::ListRow, Click(Change))`, beyond the two listed in the
  pinned collision list
- **THEN** the check fails naming the collision list's pinned count and the row that exceeded it
- **AND** the two listed rows continue to pass, so the limitation is bounded rather than open

### Requirement: The documented bindings are bound to the functions that produce them

`SPEC.md` → Keys and `README.md` → Keys both list the pane's bindings in prose, and
`ui::help::INVENTORY` now lists them a third time as data the pane renders. Three lists of
one thing is three places to go stale, and this repository's rule is that a documented
claim with a computable second site is bound to that site inside `cargo test`. This
requirement binds all three.

`tests/doc_contract.rs` SHALL carry a check that:

1. derives the set of bound actions and compares it against `ui::help::INVENTORY`, exactly as
   `binding-inventory` requires. That requirement owns the sweep, the exemption set, and the
   failure directions; this one neither restates nor weakens them, and legs 2 and 3 below are
   what `doc-conformance` adds on top;
2. requires `SPEC.md` → Keys' **key** table to name every key `INVENTORY` holds, and to name
   no key `INVENTORY` does not;
3. requires `README.md` → Keys to name the same set.

Legs 2 and 3 compare **key atoms**, not `input` strings verbatim, and the normalisation SHALL
be stated here rather than left to the implementer — an earlier draft said "every `input` string
`INVENTORY` holds", which is unsatisfiable against either document as written and was caught by
running the comparison rather than by re-reading it. The rule is:

- The **`Mouse`** group is excluded. Its gestures are bound by the mouse table's own check
  against `SPEC.md` → Keys' **mouse** table, which is a different table with a different
  grammar, and `README.md` documents no gesture at all.
- An `INVENTORY` `input` is split on `" / "` into atoms, so `j / ↓` contributes `j` and `↓`.
  This is what lets one overlay row stand for a key and its arrow synonym without forcing the
  prose to split into two rows.
- A document's atoms are the **backticked spans in the Key column** of its Keys table, the
  table bounded by its own contiguous `|` rows — never a scan of the whole document, which
  picks up every backticked identifier in it.
- Exactly **two** normalisations are permitted, and they SHALL be named in the check's own
  source: the bare word `arrows` in a Key cell contributes `↑` and `↓`; and a backticked pair
  joined by an en-dash, `` `1`–`9` ``, contributes the single range atom `1–9` rather than the
  two endpoints. No third alias SHALL be added — a future binding whose prose spelling does not
  atomise to its `INVENTORY` spelling SHALL be made to agree by editing the document, not by
  growing this list. The check SHALL assert the alias table's length is two, on the same terms
  leg 1's exemption set is pinned at two.

Under that rule the two documents SHALL gain four things, which are real gaps rather than
artifacts of the comparison: `?` in both (it is this change's own binding); `Space` in
`README.md`, which omits it while `SPEC.md` carries it; and `Backspace` in **both**, which each
document mentions only inside the `/` row's prose and neither names as a key of its own,
though `INVENTORY`'s `While filtering` group binds it to `FilterPop`.

Legs 2 and 3 compare the prose against `INVENTORY`, not against the swept functions, and
that is deliberate: leg 1 already binds `INVENTORY` to the functions, so binding the prose
to `INVENTORY` makes the inventory the single hinge every other list turns on, and a
binding added to the driver fails leg 1 before it can reach legs 2 and 3 at all.

Each of the three legs SHALL fail loudly rather than vacuously. A missing `### Keys`
section, a missing table, or a table from which every row has been deleted SHALL be an
error naming what was missing, on exactly the terms `documented_mouse_actions` already
holds to — it returns `Err` naming the absence rather than comparing an empty set against
an empty set and passing.

`SPEC.md` → Keys' **mouse** table is bound separately, and that binding is **executed**
rather than parsed: it is stated in full by "`SPEC.md`'s mouse table is bound by executing
`mouse_action`, row by row" above. The earlier framing here — that the mouse leg "parses
source text for `Action::` variants" while the key leg "executes the function", and that the
disagreement between the two styles is "itself informative" — is **superseded**. Parsing
source text was the weaker of the two and let four false statements survive a green
`make check`; the name-set comparison survives only as the first leg of the executed check,
where it reports a plain vocabulary mismatch in its existing terms before the stricter legs
run. Both tables are now bound by executing the function that produces the behaviour.

#### Scenario: A binding added to the driver and not to the docs fails `make check`

- **WHEN** a key is bound in `action_for` to an existing `Action` for which `INVENTORY`
  holds no row
- **THEN** the check fails naming that action, the swept function it came from, and the
  inventory that omitted it
- **AND** when the inventory row is added but `SPEC.md` → Keys is not, leg 2 fails naming
  the `input` string and the document
- **AND** when `SPEC.md` is updated but `README.md` is not, leg 3 fails on the same terms

#### Scenario: The documented key set and the inventory agree at HEAD

- **WHEN** the check is run against the tree at the end of this change
- **THEN** all three legs pass
- **AND** leg 2's extraction finds `SPEC.md` → Keys' key table by its own header row, not
  by position, and reports at least one row
- **AND** leg 3's extraction finds `README.md` → Keys the same way
- **AND** the atom sets agree exactly, in both directions, with the alias table at length two —
  no residual difference is tolerated and none is exempted
- **AND** the extraction is bounded to the table's own contiguous `|` rows: a check that
  scanned the whole of `SPEC.md` would collect every backticked identifier in it — sixty-odd
  role names, CLI fragments and paths — and pass vacuously in the doc-has-extra direction,
  which is the failure this bullet exists to forbid

#### Scenario: A gutted document fails as a broken control rather than a clean tree

- **WHEN** the check is run against a copy of the tree in which `SPEC.md`'s `### Keys`
  section has been deleted, and again against one in which its key table has been reduced
  to a header row and a separator row
- **THEN** both runs fail with a message naming the missing section or the empty table
- **AND** neither reports a pass, so a document that stopped documenting is a failure and
  not an agreement between two empty sets

#### Scenario: The key legs are unaffected by the mouse table's stronger binding

- **WHEN** the mouse table's executed check is added and the key legs are re-run unchanged
- **THEN** legs 1, 2 and 3 pass exactly as before
- **AND** the `Mouse` group remains excluded from leg 2's and leg 3's atom comparison, since
  `README.md` still documents no gesture

### Requirement: The new module's documentation is bound where a binding exists, and hand-written where none does

`src/ui/help.rs` is a **submodule**, and the two existing map checks do not see submodules.
`tests/doc_contract.rs`'s `pub_mod_names` reads `src/lib.rs`'s top-level `pub mod`
declarations, and `SPEC.md`'s Module map carries a single `ui` row for all thirteen files
under `src/ui/`; § Unit-tested modules is bound to "every **declared** module" on the same
terms. Adding `src/ui/help.rs` therefore fails **neither** check, exactly as adding
`src/ui/palette.rs` failed neither.

This requirement states that plainly rather than claiming a binding that does not exist. An
earlier draft of it asserted both checks would fail until the documents named `ui::help`,
which is false and would have made its own scenario unfalsifiable — a guard that is believed
and cannot fire is worse than no guard, which is this capability's own standing rule.

What SHALL be machine-bound is what has a computable second site:

- `scripts/gates/noio-view.sh`'s `PURE` list SHALL hold **ten** entries and
  `scripts/gates/colwidth.sh`'s **nine**, both naming `src/ui/help.rs`, and both gates SHALL
  fail when it is absent — `view-palette` owns that leg and this change extends it.
- **`AGENTS.md`'s** "the pure set is **nine** files" SHALL read **ten** and SHALL list
  `src/ui/help.rs`. The gate output is the second site: a prose count that disagrees with
  `NOIO-VIEW OK: 10 pure files` is a drift a reader can settle in one command. `SPEC.md` is
  **not** named here, and the omission is deliberate: it carries no pure-set count and no
  pure-view file list at all. Its § Architecture render-seam passage states the property in
  prose — "Views are pure functions … They perform no I/O" — and enumerates nothing, so there
  is no sentence in it to move from nine to ten. `AGENTS.md:339` is the crate's only prose
  copy of that list.
- **`AGENTS.md`'s** "`tests/doc_contract.rs` carries **nine** further claims" SHALL read
  **ten**, since this change adds the inventory/key-table binding to that file. `SPEC.md`
  § Doc-conformance checks carries the same nine claims as an unnumbered **bullet list** and
  no count, so what it SHALL gain is a **tenth bullet** naming the new binding — not a
  changed numeral.

Naming `SPEC.md` in either bullet would have mandated an edit to a sentence that does not
exist, which is the unfalsifiable guard this capability's own standing rule forbids and which
the paragraph above already refuses once. It was caught by executing the greps rather than by
re-reading the bullet.

What SHALL be hand-written, with no check claimed for it: `SPEC.md`'s Module map `ui` row
and § Unit-tested modules SHALL mention the overlay and the inventory in prose, as
documentation. Extending the two checks to submodule granularity is **not** in this change's
scope — the `ui` row is a prose cell describing responsibilities, not a file list, and
binding it to file names would be a fragile check written to satisfy a sentence.

#### Scenario: The submodule is invisible to both map checks, and that is asserted rather than assumed

- **WHEN** `src/ui/help.rs` is added and neither `SPEC.md`'s Module map nor § Unit-tested
  modules is touched
- **THEN** `tests/doc_contract.rs`'s module-map and tested-modules checks both still **pass**,
  because both read `src/lib.rs`'s top-level `pub mod` set and `ui` is already in it
- **AND** a test asserts exactly that — `pub_mod_names(src/lib.rs)` contains `ui` and does not
  contain `help` or `ui::help` — so a future change that extends either check to submodules
  fails this scenario and is told to update this requirement rather than discovering the
  granularity by surprise
- **AND** the checks that **do** fire for this module are `NOIO-VIEW` and `COLWIDTH`, covered
  by the scenario below

#### Scenario: The gate lists and the prose counts agree

- **WHEN** `scripts/gates/noio-view.sh` and `scripts/gates/colwidth.sh` are run against
  the tree at the end of this change
- **THEN** the first reports ten pure files and the second nine
- **AND** both include `src/ui/help.rs`, and both fail when it is removed from their
  `PURE` list rather than reporting a clean tree over the remaining files

### Requirement: A non-view pure module's freedom from I/O is checked inside `cargo test`

`tests/doc_contract.rs` SHALL carry an **eleventh** claim: that the production slice of
`src/specs.rs` names no filesystem, process, environment, network, or standard-I/O API, and no
schema-reading name.

The check SHALL read the file's production slice — the text above its first line-anchored
`#[cfg(test)]`, by the same helper `production_slice_cuts_before_cfg_test` and
`production_slice_whole_file_when_no_cfg_test` already prove and that `scripts/coverage-prod.py`
already uses — and SHALL fail naming both the needle and the line when one occurs.

It SHALL live in `tests/doc_contract.rs` and **not** in `src/specs.rs`, because a check written
inside the file it sweeps contains its own needles and can never pass. This is the same
substitution `terminal_seam_names_match_the_gate` already makes: binding needle names to source
text from inside `cargo test` rather than from a gate script.

It SHALL NOT be a `scripts/gates/` script, and SHALL NOT be folded into `NOIO-VIEW`. Both costs
are stated rather than assumed:

- `NOIO-VIEW`'s `PURE` list is the **render seam's**, and `src/specs.rs` is not a view. Adding
  it would move the "ten pure files" figure that `AGENTS.md`, `SPEC.md`, `view-palette`, and
  `responsive-layout` each carry in prose — the five-site cost that putting the module outside
  `src/ui/` exists to avoid, so folding in would defeat the decision it is meant to support.
- A thirty-second script under `scripts/gates/` would move the count
  `openspec/specs/quality-gates/spec.md` states in seven places, whose owning requirement is 201
  lines and would therefore need a full-content `MODIFIED` rewrite carrying no behaviour.

The claim SHALL be falsifiable through this tier's standing mechanism rather than asserted:
planting a `use std::fs;` above the `#[cfg(test)]` line SHALL make it fail, and removing the
plant SHALL make it pass.

This requirement generalises deliberately. The subject is "a pure module outside `src/ui/` that
a `NOIO-VIEW` file calls into" — `src/specs.rs` is the first, and a second such module SHALL
join this check rather than acquire one of its own, because the reason the property matters is
the same in both cases: I/O one call away from a swept file, with every gate green.

#### Scenario: The production slice of `src/specs.rs` carries no I/O or schema name

- **WHEN** `cargo test --test doc_contract` runs on a tree whose `src/specs.rs` reaches no I/O
- **THEN** the claim passes, having read the slice above the file's first `#[cfg(test)]` line
- **AND** the slice is non-empty, so the check cannot pass vacuously against a file it failed to
  read or cut at the wrong place

#### Scenario: An I/O name planted in the production slice fails the claim

- **WHEN** `use std::fs;` is inserted above `src/specs.rs`'s `#[cfg(test)]` line and
  `cargo test --test doc_contract` runs
- **THEN** the claim fails, and its message names both the needle `std::fs` and the line number
- **AND** removing the plant returns the claim to passing, which is the negative control this
  tier requires of a check that is green at HEAD

#### Scenario: An I/O name in the test module alone does not fail the claim

- **WHEN** `src/specs.rs`'s own `#[cfg(test)] mod tests` names `read_to_string` — as a test
  reading a fixture would — and the production slice does not
- **THEN** the claim passes
- **AND** the slice boundary is therefore load-bearing rather than an exemption: it is what lets
  the needles be searched for at all without the searching test matching itself

### Requirement: The clipboard write's confinement is bound inside `cargo test`

`tests/doc_contract.rs` SHALL carry a **twelfth** claim: that the OSC 52 introducer appears
in `src/ui/terminal.rs` and nowhere else in the crate, `tests/` included. It joins the eleven
already there on the same terms — a documented claim with a computable second site is bound
to that site inside `cargo test`, not left to a human re-reading it.

The claim SHALL fail when its exclusion goes **vacuous**: when `src/ui/terminal.rs` is absent,
or is present and names no OSC 52 sequence. A confinement check that passes because the
confined thing has disappeared is worse than none, because it is believed.

`AGENTS.md`'s "eleven further claims" and `SPEC.md` → § Doc-conformance checks SHALL both move
to twelve, and the count SHALL be bound to the number of claims the test file actually carries
rather than left as prose. Neither site is machine-bound today, which is why both have drifted
before; this change binds them.

#### Scenario: The twelfth claim holds and is falsifiable

- **WHEN** `cargo test --test doc_contract` runs against the tree at the end of this change
- **THEN** the OSC 52 confinement claim passes
- **AND** planting the OSC 52 introducer in any other file under `src/` or `tests/` fails it
- **AND** removing it from `src/ui/terminal.rs` fails it too, rather than passing vacuously

#### Scenario: The documented claim count matches the file

- **WHEN** the claim count stated in `AGENTS.md` and in `SPEC.md` is compared against the
  number of claims `tests/doc_contract.rs` carries
- **THEN** all three agree at twelve
- **AND** the comparison is executed by a test, so a thirteenth claim added without moving the
  prose fails `cargo test` rather than drifting unnoticed

### Requirement: `src/settings.rs`' freedom from I/O is the sixteenth claim

`tests/doc_contract.rs` SHALL carry a **sixteenth** claim: that the production slice of
`src/settings.rs` names no filesystem, process, environment, network, or standard-I/O API, no
clock, and no `ratatui` type.

It is owed for exactly the reason the eleventh (`src/specs.rs`) and the fourteenth
(`src/integration.rs`) are owed. `src/settings.rs` is a pure module deliberately placed
**outside** `src/ui/`, so that adding it moves neither `NOIO-VIEW`'s pure-file count nor
`COLWIDTH`'s. The cost of that placement is that **no `make gates` script sweeps it at all**,
and this claim is what replaces the sweep.

The `ratatui` needle is this claim's own addition, beyond what the eleventh requires: the
module produces the values a view renders, so a `Span`, a `Style`, or a `Line` reaching it
would put rendering decisions on the wrong side of the seam that `PALETTE` and `MDSEAM`
already police from the other direction. `src/integration.rs`' claim forbids the render
crate's own types on the same reasoning.

The check SHALL read the file's production slice — the text above its first line-anchored
`#[cfg(test)]`, by the same helper the eleventh claim uses — and SHALL fail naming both the
needle and the line when one occurs.

Adding this claim moves **four** sites, and `cargo test --test doc_contract` stays red until
all four agree: `CLAIM_COUNT` in `tests/doc_contract.rs` from 15 to 16; `CLAIM_COUNT_WORDS`,
today a fixed `[(&str, usize); 6]` topping out at `("fifteen", 15)`, extended to seven entries
with `("sixteen", 16)`; `AGENTS.md`'s "fifteen further claims" and its enumerated list; and a
new bullet under `SPEC.md` → `### Doc-conformance checks`. That section holds one more bullet
than it has claims — the last, "A claim with no second site is argued in review, not checked",
is a meta-statement — so the new bullet is inserted **above** it.

#### Scenario: A planted I/O name fails the claim

- **WHEN** `use std::fs;` is added above `src/settings.rs`' first line-anchored `#[cfg(test)]`
- **THEN** `cargo test --test doc_contract` fails, naming both the needle and the line
- **AND** removing it makes the claim pass again
- **AND** the same holds for a planted `ratatui::style::Style` and a planted `Instant::now()`

#### Scenario: An I/O name in the test module alone does not fail the claim

- **WHEN** `src/settings.rs`' own `#[cfg(test)] mod tests` names `read_to_string` — as a test
  reading a fixture would — and the production slice does not
- **THEN** the claim passes
- **AND** the slice boundary is therefore load-bearing rather than an exemption

#### Scenario: The claim count is bound at all four sites

- **WHEN** `CLAIM_COUNT` is raised to 16 but `CLAIM_COUNT_WORDS` is left at six entries
- **THEN** `agents_md_claim_count` returns `Err` on the word "sixteen" rather than passing
- **AND** raising `CLAIM_COUNT` while leaving `AGENTS.md` at "fifteen further claims" fails
  with a message naming both counts
- **AND** adding the `SPEC.md` bullet below the trailing meta-statement rather than above it
  leaves that statement no longer last, which the section's own ordering assertion catches

### Requirement: `src/worktrees.rs`' freedom from I/O is the seventeenth claim

`tests/doc_contract.rs` SHALL carry a **seventeenth** claim: that the production slice of
`src/worktrees.rs` names none of these needles — the eleventh claim's I/O set (`std::fs`,
`std::io`, `std::env`, `std::process`, `std::net`, `File::`, `read_to_string`, `Command`); the
path methods that reach the filesystem without naming `std::fs` (`canonicalize`, `.exists(`,
`.is_dir(`, `metadata(`, `read_dir`); the clocks (`Instant`, `SystemTime`); `ratatui`; and the
CLI handles (`GitCli`, `OpenspecCli`, `HerdrCli`, `git_cli_via`, `crate::cli`).

It is owed for exactly the reason the eleventh (`src/specs.rs`), the fourteenth
(`src/integration.rs`), and the sixteenth (`src/settings.rs`) are owed. `src/worktrees.rs` is a
pure module placed **outside** `src/ui/`, so adding it moves neither `NOIO-VIEW`'s pure-file
count nor `COLWIDTH`'s, and **no `make gates` script sweeps it**; this claim replaces the sweep.
The CLI-handle needles are this claim's own addition, on `src/integration.rs`' model: the module
parses the stdout of `git` commands the refresh worker runs, and a handle reaching it would move
a blocking call into code the design keeps pure. Canonicalizing a worktree path is a filesystem
call and SHALL stay in the worker, which passes the canonical paths in.

The check SHALL read the file's production slice — the text above its first line-anchored
`#[cfg(test)]`, by the same helper the eleventh claim uses — and SHALL fail naming both the
needle and the line when one occurs.

Adding this claim moves the same **four** sites the sixteenth moved, **in one commit**, and
`cargo test --test doc_contract` stays red until all four agree: `CLAIM_COUNT` from 16 to 17;
`CLAIM_COUNT_WORDS` from a `[(&str, usize); 7]` to an 8-entry array ending
`("seventeen", 17)`; `AGENTS.md`'s "sixteen further claims" and its enumerated list; and a new
bullet under `SPEC.md` → `### Doc-conformance checks`, inserted **above** the trailing
meta-statement. Its negative controls SHALL be in-file tests over string literals, as the
sixteenth claim's `settings_rs_production_slice_check_*` tests are, so they run on every
`cargo test` rather than once in a scratch copy.

#### Scenario: A planted I/O name or CLI handle fails the claim

- **WHEN** `use std::fs;` is added above `src/worktrees.rs`' first line-anchored `#[cfg(test)]`
- **THEN** `cargo test --test doc_contract` fails, naming both the needle and the line
- **AND** removing it makes the claim pass again
- **AND** the same holds for a planted `root.canonicalize()` — the spelling that names no
  `std::fs` — a planted `Instant::now()`, and a planted `use crate::cli::GitCli;`

#### Scenario: An I/O name in the test module alone does not fail the claim

- **WHEN** `src/worktrees.rs`' own `#[cfg(test)] mod tests` names `read_to_string` and the
  production slice does not
- **THEN** the claim passes

#### Scenario: The claim count is bound at all four sites

- **WHEN** `CLAIM_COUNT` is raised to 17 while `AGENTS.md` still reads "sixteen further claims"
- **THEN** `cargo test --test doc_contract` fails with a message naming both counts
- **AND** raising it while `CLAIM_COUNT_WORDS` lacks `("seventeen", 17)` makes
  `agents_md_claim_count` return `Err` on that word rather than pass
