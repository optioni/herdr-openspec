## Purpose
Binds the project's own documents to the code they describe. Every documented claim that
has a computable *second site* — the crate's module list, its production worker threads,
its declared MSRV, the programs `make check` invokes, the plugin manifest, and the project
context injected into every OpenSpec agent's prompt — is checked inside `cargo test`, so a
claim that drifts fails `make check` rather than surviving until an audit finds it.

## ADDED Requirements

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
