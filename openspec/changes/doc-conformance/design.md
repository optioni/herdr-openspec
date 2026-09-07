## Context

An audit of this repository's own documents against its code found seven drifts. The
project's precedence rule is that `SPEC.md` wins where prose disagrees, so these are wrong
contracts rather than typos, and one of them — `openspec/config.yaml`'s `context` block — is
injected verbatim into every OpenSpec agent's prompt, which means it propagates into future
changes rather than sitting inert.

The project's standing rule for doc accuracy has been *"a doc claim is fixed by the change
that ships the behaviour"* (`HANDOFF.md` → "Rule: a doc claim is fixed by the change that
makes it true"). `HANDOFF.md` also records that rule's blind spot: it assumes the change
making a claim true comes after the claim was written, an ordering that inverts at the end of
a roadmap. Every one of the seven findings is a claim that was true when written and was
falsified by a *later* change that had no reason to look at it — exactly the case the rule
cannot cover. So the durable half of this change is a mechanism, not a set of edits.

### Findings this change owns

| # | Severity | Site | Drift |
|---|---|---|---|
| D4 | MEDIUM | `SPEC.md` Module map | 12 rows against 13 `pub mod` declarations; `open` is missing |
| D5a | MEDIUM | `SPEC.md` § Unit-tested modules | `refresh::start` called "one of the crate's **two** worker threads"; there are three |
| D5b | MEDIUM | `SPEC.md` § Unit-tested modules | no bullet for `launch`, `open`, `config`, or `state`; `launch::` appears nowhere in the file |
| D6 | MEDIUM | `openspec/config.yaml` `context` | claims "fixture repositories under `tests/fixtures/`" (none exist); the gate list omits `make gates` |
| D7 | LOW | `README.md`, `SPEC.md` | omit `python3`, which `make check` requires, and omit the hygiene-gate tier |
| D8 | MEDIUM | `HANDOFF.md` | a residual open-work list whose three items all landed, and a stale Environment section, under a header saying "nothing is outstanding" |
| D9 | LOW | `AGENTS.md` | "Rust stable (1.91+)" reads as a requirement; `Cargo.toml` declares `rust-version = "1.88"` and that value appears in no prose |
| D10 | LOW | `SPEC.md` manifest block | `[[…]]` tables in a different order from `herdr-plugin.toml`; key/value content identical |

Verified at HEAD: `src/lib.rs:9-21` declares 13 modules; production `thread::spawn` (before
each file's first `#[cfg(test)]`) appears in `src/refresh.rs:89`, `src/agents.rs:467`, and
`src/launch.rs:387` — three, with `src/cli.rs`'s four occurrences all inside its test slice;
`grep -c "launch::" SPEC.md` returns 0, and `open::`, `config::`, and `state::` appear only
outside § Unit-tested modules (`SPEC.md:666`, `:702`, `:57`, `:67`, `:749`); `tests/fixtures/`
holds `tasks/` and `build-graph.txt` and no `openspec/` subdirectory anywhere.

**The audit undercounted D5b, and running the check's rule found it.** The audit named
`launch` and `open` as missing from § Unit-tested modules; the rule reported four — `config`
and `state` are absent from that section too. Recorded here rather than quietly folded in,
because it is the first evidence that this mechanism finds what a careful reading does not,
which is the argument for building it.

### Sibling changes — passages this change must not touch

**Eight** other changes are being proposed in parallel, measured with `openspec list` at
planning time: `cli-parity`, `color-palette`, `gate-integrity`, `list-sections`,
`markdown-constructs`, `mouse-input`, `seam-resilience`, and `view-fidelity`. Every one of
them names `SPEC.md`; six also name `AGENTS.md`; three name
`openspec/IMPLEMENTATION-ORDER.md`. Three carry passages explicitly assigned away from this
change:

| Passage | Owner |
|---|---|
| `SPEC.md` § Gates — the gate count and its table; the `quality-gates`, `ci-workflow`, and `degraded-coverage` capability specs | `gate-integrity` |
| `AGENTS.md`'s "joined by position" merge sentence; `SPEC.md`'s degraded-states row on glob symlink divergence | `cli-parity` |
| `SPEC.md:263`'s watch-scope statement | `seam-resilience` |

The other five are not assigned passages here, and this change does not attempt to enumerate
what they will edit. What protects the boundary is not the enumeration but the scoping rule
below: this change's checks assert against `README.md` and `AGENTS.md` for the toolchain leg,
and against four *named sections* of `SPEC.md` — the Module map table, § Unit-tested modules,
the `crate's <N> worker threads` phrase, and the fenced manifest block. A sibling editing any
other part of `SPEC.md` cannot make a leg here go red.

**A sibling that adds a module or a worker thread will find these legs red, and that is the
mechanism working, not a collision.** `view-fidelity`, `list-sections`, and others may add
code; if one adds a `pub mod`, the module-map leg correctly requires a map row for it. Whoever
lands second pays that cost, which is the point of building the check.

The near-miss is D7. `SPEC.md:911-913`'s *"Two one-time setup steps are required for local
development"* paragraph sits **inside** § Gates, the section `gate-integrity` is rewriting,
and its subject is two *installable* components (`rustup component add clippy`,
`cargo install cargo-llvm-cov`) — `python3` is neither, so folding it in would break that
paragraph's own count. This change therefore adds `python3` to `SPEC.md` as **its own
sentence immediately after** that paragraph, leaving the paragraph's two-step claim intact,
and edits nothing else in § Gates: not the table, not the gate count, not the "all four in
order" sentence. Task 5.3 greps the diff to prove it.

Correspondingly, the toolchain-prerequisite check asserts against `README.md` and `AGENTS.md`
only, never against any part of `SPEC.md`. The two changes therefore cannot land
contradictory checks, and if `gate-integrity` rewrites § Gates around the added sentence —
or adds `python3` itself — nothing here goes red.

## Goals / Non-Goals

**Goals:**

- Make the eight drifted passages true.
- Add a doc-conformance test tier that fails `make check` when a documented claim with a
  computable second site disagrees with that site.
- Leave `HANDOFF.md` in a state a cold reader cannot mistake for live work.

**Non-Goals:**

- No change to any file under `src/`, any rendered state, keybinding, manifest value, or
  configuration key. Not **BREAKING**.
- No new `scripts/gates/` script and no edit to the `Makefile`'s `gates:` recipe (see
  Decision 1).
- No general documentation rewrite; only the audited claims.
- No check on prose that has no second site. A claim the repository cannot recompute is
  argued in text and left to review — inventing a check for it would be a check that cannot
  fail.
- The four sibling-owned passages above.

## Boundaries

**Modules touched:** none. `src/` is not edited.

**New file:** `tests/doc_contract.rs`, an ordinary `cargo test` integration target. It joins
the repository's existing **contract tier**, which already has two members and one
established shape:

- `tests/manifest.rs` — binds `herdr-plugin.toml`, `README.md`'s advertised action titles,
  and the Cargo binary name to each other, and confines itself to *values with a second
  site*, deliberately not asserting `version` or `platforms` because a wrong value there is
  rejected loudly elsewhere.
- `tests/degraded_coverage.rs` — binds `SPEC.md`'s degraded-states table to named proving
  tests through a checked-in map.

`tests/doc_contract.rs` follows `tests/manifest.rs`'s discipline exactly: parse the second
site, parse the document, compare, name both sides on failure, and assert nothing that has no
second site.

**Documents edited:** `SPEC.md`, `README.md`, `AGENTS.md`, `HANDOFF.md`,
`openspec/config.yaml`.

**Crate dependencies:** none added. `toml` is already a dependency and is what
`tests/manifest.rs` uses to parse the manifest; the same parser reads `Cargo.toml` and
`SPEC.md`'s transcribed block. `openspec/config.yaml` is read as **text**, not parsed as
YAML — the crate has `yaml-rust2` for schema files, but every claim checked in the context
block is a substring or a phrase, and text matching keeps the check from depending on how the
block is folded.

**Architecture rules:** untouched. No process is spawned (the check reads files only, which
is also why it can live in `make check` at all — `openspec` and `herdr` are optional at
runtime and absent from both CI runners). No view gains I/O. Nothing under `src/ui/` is
edited. `NOSPAWN-GREP`, `NOBLOCK`, `NOIO-VIEW`, and the width gates are unaffected.

**Writing inside `openspec/`:** this change edits `openspec/config.yaml`, which the PRD
non-goal and `OPENSPEC-UNTOUCHED` do **not** forbid. That non-goal governs *the plugin
process* — the dashboard must not write OpenSpec files while an agent edits them — and the
gate that mechanises it (`scripts/gates/openspec-untouched.sh`) checks for **untracked** files
appearing under `openspec/`. `openspec/config.yaml` is a tracked project-configuration file
edited by its authors, exactly as `openspec/IMPLEMENTATION-ORDER.md` has been edited by
several archived changes. Editing it leaves the gate green.

## Contracts

No interface with a separate consumer changes. Specifically:

- **`herdr-plugin.toml`** is not edited; `SPEC.md`'s copy of it is brought into line with the
  file, in the direction file → document. Herdr sees nothing new.
- **The plugin binary** is unchanged: no subcommand, argument, exit status, or key.
- **`config.toml`'s format** is unchanged.
- **`openspec/config.yaml`'s `context`** is consumed by OpenSpec agents. The edit is
  additive and corrective — a false sentence replaced by a true one, and `make gates` added
  to a command list. No `rules` or `operations` key changes, so no schema behaviour changes.

This change is **additive**. Nothing is removed from a consumer's surface.

## Persistence and Rollout

- **Migration:** none — no data, no schema, no on-disk format.
- **Backfill:** none.
- **Seeding:** none.
- **Cache invalidation:** none. The plugin's only write, `agent-names.toml` under
  `HERDR_PLUGIN_STATE_DIR`, is untouched.
- **Index rebuild:** none.
- **Authorization:** none — no privilege boundary exists in this crate.
- **Observability:** the new test target's failure messages are the only new output. Each
  names the document, the claim, and both sides of the disagreement.
- **Deployment:** none. No release, no registry submission, no push. The release binary is
  byte-identical, since `tests/` is not compiled into it.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| The repository's own files (`SPEC.md`, `README.md`, `AGENTS.md`, `Cargo.toml`, `Makefile`, `herdr-plugin.toml`, `openspec/config.yaml`, `src/lib.rs`, `src/*.rs`, `scripts/gates/*`, `tests/fixtures/`) | **real**, read from `CARGO_MANIFEST_DIR` — they are the subject | **real**, same |
| The parsing of a document (table rows, section slicing, phrase extraction) | exercised through the real files | **replaced**: driven by in-test `&str` inputs, so a malformed document can be tested without creating one |
| `toml` parser | real | real |
| The `openspec` binary | **not used** — never spawned, never required | not used |
| The `herdr` binary / Herdr socket | **not used** | not used |
| The terminal | **not used** — no test constructs a `ratatui` terminal or enters raw mode | not used |
| The process environment | **not used** — `CARGO_MANIFEST_DIR` is read via `env!` at compile time, never `std::env::var` at run time | not used |
| The filesystem outside the repository | **not used** — no `ScratchDir`, no `std::env::temp_dir()` | not used |
| `git` | **not used** inside `tests/doc_contract.rs`. Used only as task-level verification: the diff greps at 5.3, 6.3 and 7.3, the plant-and-revert cycle at 9.1–9.3, the scope check at 10.3, and the `src/` diff at 12.9 | not used |
| The network | **not used** | not used |

The split that makes this testable: every leg is **two functions** — a pure parser over
`&str` (`module_map_names(spec_md)`, `worker_thread_claim(spec_md)`,
`check_prerequisite_targets(makefile)`, …) with its own unit tests over hand-written inputs
including malformed ones, and a thin `#[test]` that feeds it the real file. This is the same
shape `tests/degraded_coverage.rs` uses (`parse_spec_conditions` is a pure function returning
`Result`, never a panic) and it is what lets the "heading is gone" and "document is missing"
scenarios be driven without deleting a real file.

## Test Strategy

Tiers used, and why no other tier applies:

- **contract** — a `#[test]` in `tests/doc_contract.rs` reading real repository files.
  Command: `cargo test --test doc_contract`.
- **unit** — a `#[test]` in `tests/doc_contract.rs` over an in-test `&str` or a synthetic
  input. Same command.
- **planted defect** — a real defect written into the repository, the check observed failing,
  the defect reverted, and the transcript recorded in
  `openspec/changes/doc-conformance/notes/planted-defects.md`. This is the project's
  established way of proving a check can fail (`degraded-states`' `notes/planted-defects.md`).
- **review** — a claim with no second site, resolved by argument in this document.

No **view** tier and no **outer-loop acceptance test**: this change renders nothing and adds
no wiring between collaborators. The outer-loop test exists in this project to prove
composition — `HANDOFF.md`'s `ui::run` wiring bug is the canonical case — and there is no
composition here. The nearest equivalent, "does this check actually run inside `make check`",
is covered by the fact that `cargo test` discovers `tests/*.rs` automatically and `check:`
lists `test` among its prerequisites; task group 8 runs the whole `make check` and observes
the new target in its output rather than asserting it in a test.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| A document is missing entirely | `read_doc` returns `Err` naming the path; driven with a path that does not exist | unit | real filesystem (a nonexistent path) | `cargo test --test doc_contract missing_document` |
| A bound section's heading no longer exists | each parser returns `Err` naming the heading it searched for, driven with an in-test `&str` lacking it | unit | none (pure `&str`) | `cargo test --test doc_contract absent_heading` |
| Every leg is proved able to fail | one planted defect per leg, observed red, reverted; transcript recorded | planted defect | real repository files, `git` | `make check`, then `git checkout -- <file>` |
| A module exists with no map row | `module_map_names` vs a synthetic `pub mod` set omitting one row | unit | none (pure `&str`) | `cargo test --test doc_contract map_missing_row` |
| A map row names no module | same, in the other direction | unit | none (pure `&str`) | `cargo test --test doc_contract map_orphan_row` |
| The map and the crate agree | real `SPEC.md` vs real `src/lib.rs` | contract | real files | `cargo test --test doc_contract module_map_matches_lib_rs` |
| A module has no bullet | tested-modules section vs a synthetic module set, one absent | unit | none (pure `&str`) | `cargo test --test doc_contract tested_modules_missing` |
| A mention outside the section does not satisfy the leg | an `&str` naming `open::` before the section heading and not inside it | unit | none (pure `&str`) | `cargo test --test doc_contract tested_modules_scoped_to_section` |
| The documented count is stale | `worker_thread_claim` vs a synthetic production-slice count | unit | none (pure `&str`) | `cargo test --test doc_contract worker_count_stale` |
| A fourth worker thread is added | `production_thread_files` over synthetic sources, one added | unit | none (pure `&str`) | `cargo test --test doc_contract worker_count_grows` |
| The bound phrasing is removed | `worker_thread_claim` returns `Err` on an `&str` with no bound phrase | unit | none (pure `&str`) | `cargo test --test doc_contract worker_claim_absent` |
| (same, real state) | real `SPEC.md` vs the real production slices of `src/*.rs` | contract | real files | `cargo test --test doc_contract worker_threads_match_sources` |
| The MSRV is documented nowhere | real `Cargo.toml` `rust-version` present in real `AGENTS.md` and `README.md` | contract | real files | `cargo test --test doc_contract msrv_is_documented` |
| The MSRV is raised without a doc edit | `msrv_mentions` over synthetic doc text and a bumped version | unit | none (pure `&str`) | `cargo test --test doc_contract msrv_bump_is_caught` |
| An invoked interpreter is undocumented | `check_programs(makefile)` over the real `Makefile`, each result required in real `README.md` and `AGENTS.md` | contract | real files | `cargo test --test doc_contract gate_programs_are_documented` |
| A gate script's own interpreter is undocumented | `python3` grep over real `scripts/gates/*`, required in both documents | contract | real files | `cargo test --test doc_contract gate_script_interpreters_are_documented` |
| A shell guard block yields no program | `check_programs` over a synthetic `Makefile` carrying the real `@if ! cargo … fi` guard shape | unit | none (pure `&str`) | `cargo test --test doc_contract guard_block_yields_no_program` |
| A quoted assignment value is one token | `check_programs` over the real `gates:` line 35 shape as a synthetic input | unit | none (pure `&str`) | `cargo test --test doc_contract quoted_assignment_is_one_token` |
| A new external tool joins the gate path | `check_programs` over a synthetic `Makefile` naming an extra program | unit | none (pure `&str`) | `cargo test --test doc_contract new_gate_program_is_caught` |
| The extractor yields nothing | `check_programs` over a synthetic `Makefile` with no recipes; the leg fails rather than passing | unit | none (pure `&str`) | `cargo test --test doc_contract empty_program_set_fails` |
| A transcribed value diverges | real `SPEC.md` block parsed and compared to real `herdr-plugin.toml` | contract | real files, `toml` | `cargo test --test doc_contract spec_manifest_block_matches` |
| The blocks differ only in table order | `table_header_order` over two synthetic TOML sources that parse equal | unit | `toml` | `cargo test --test doc_contract manifest_block_order` |
| The transcription is absent | `manifest_block(spec_md)` returns `Err` on an `&str` with no such fence | unit | none (pure `&str`) | `cargo test --test doc_contract manifest_block_absent` |
| The context claims a fixture tier that does not exist | real `openspec/config.yaml` vs a real scan of `tests/fixtures/` for an `openspec/` subdirectory | contract | real files | `cargo test --test doc_contract context_fixture_claim` |
| A gate tier is missing from the injected context | real `check:` prerequisites vs the real context block | contract | real files | `cargo test --test doc_contract context_names_every_gate_tier` |
| A fixture repository is later added | `fixture_repositories(dir)` over a synthetic directory listing containing `openspec/` | unit | none (pure input) | `cargo test --test doc_contract fixture_repo_makes_claim_true` |
| The dashboard is unchanged | `git diff --name-only <base>..HEAD` lists no path under `src/` | review | `git` | `git diff --name-only $BASE..HEAD -- src/` |
| The degraded-states binding still holds | `tests/degraded_coverage.rs` passes with `tests/degraded-coverage.toml` unmodified | contract | real files | `cargo test --test degraded_coverage` |

## Decisions

### 1. A Rust test target, not a `scripts/gates/` shell gate

**Chosen:** `tests/doc_contract.rs`, discovered by `cargo test` and reached by `make check`
through its `test` prerequisite.

**Why:** three reasons, in order of weight.

1. The second sites are *structured*. Comparing two parsed TOML documents, extracting
   `pub mod` names, and slicing a Markdown section are all things the crate's existing `toml`
   dependency and Rust's string handling do precisely, and that a `grep`-and-`sed` gate would
   do approximately. Every shell gate in this repository checks a *presence or absence* fact;
   none of them compares two structures.
2. `tests/ci_workflow.rs` binds the `gates:` recipe and the `scripts/gates/` directory to each
   other in both directions, and `gate-integrity` — a change being proposed in parallel — owns
   the `ci-workflow` capability. Adding a script under `scripts/gates/` would mean editing the
   recipe *and* landing inside a sibling's capability. A `tests/` file touches neither.
3. Precedent. `tests/manifest.rs`'s own header records why it is a Rust test inside
   `make check` rather than a per-change shell command: *"every named gate that lives outside
   `make check` has rotted at least once"*. This is the same kind of contract, so it takes the
   same form.

**Alternative considered:** a `scripts/gates/docs.sh`. Rejected on all three counts above.
**Alternative considered:** extending `tests/manifest.rs` instead of adding a file. Rejected —
that file's stated scope is the manifest/README/binary-name triangle; six unrelated legs would
blur a boundary its own header defines carefully.

### 2. No coverage-map TOML

`tests/degraded_coverage.rs` needs `tests/degraded-coverage.toml` because its subject is 44
rows of prose that no computation can recover; the map is where a human records which test
proves which row. Here every leg recomputes its second site directly from the repository, so a
map would be indirection with nothing to hold. Each leg is code.

**Alternative considered:** `tests/doc-claims.toml` listing `{document, claim, second site}`
rows for symmetry with the degraded-coverage tier. Rejected: it would encode in data what the
six legs already say in code, and a row whose "second site" is a free-text pointer is not
mechanically checked at all — it would look like the degraded-coverage mechanism while
providing none of its force.

### 3. The worker-thread count is bound by a fixed phrase, with optional emphasis

The check parses `crate's <number-word> worker threads`, accepting `one` through `six`, with
the number word **optionally** wrapped in `**`. Every occurrence must agree, and at least one
must exist — an absent phrase fails rather than passing vacuously, which is the scenario "The
bound phrasing is removed".

**The emphasis must be optional, and this was measured.** At HEAD `SPEC.md:820-821` reads
`crate's two worker threads` with **no** markup. A parser requiring `**` would fire the
"claim could not be located" branch at HEAD instead of the "count is stale" branch — still
red, but for the wrong reason and with a message that would send the implementer to add
markup rather than fix the number. Accepting both forms makes the failure say what is
actually wrong.

**Why a fixed phrase and not free-form number detection:** searching `SPEC.md` for any numeral
near the word "thread" would fire on unrelated prose (`agent-polling`'s "second thread",
`NOBLOCK`'s "third file") and would be a check nobody could predict. A named phrase is a
contract a future author can see in the file and keep.

**"Production" is defined as the text before a file's first `#[cfg(test)]` line** — the same
cut `NOBLOCK` leg 3 uses to slice `src/refresh.rs`, `src/agents.rs`, and `src/launch.rs`.
Reusing it means this leg and that gate can never disagree about which `thread::spawn` counts;
`src/cli.rs`'s four occurrences are all below its `#[cfg(test)]` at line 317 and are correctly
excluded.

### 4. `SPEC.md`'s manifest block is reordered rather than declared a non-issue (D10)

Key/value content is byte-identical and TOML array-of-table order is not semantic, so nothing
is broken today. Two reasons to fix it anyway: it costs one block move in a document, and
leaving it means every future audit re-reports it as a possible defect and re-derives that it
is not one. Binding the order also makes the *value* comparison — which is load-bearing —
cheap to state as "these two sources agree", with no carve-out a reader has to remember.

**Alternative considered:** declare it a non-issue in `SPEC.md` with a note. Rejected: a note
saying "the order differs and that is fine" is itself a claim that will need maintaining.

### 5. `HANDOFF.md` becomes a closed historical record — corrected, not deleted

**Recommendation: keep the file, delete its open-work sections, and mark it closed.**
Concretely:

- Delete § **"Known-deferred doc fixes"** outright. All three items have landed: `AGENTS.md`'s
  "Current repo state" is current and no longer claims `Cargo.toml` is absent; `plugin-actions`
  shipped and both `[[actions]]` are in `herdr-plugin.toml`; `IMPLEMENTATION-ORDER.md`'s
  Phase 6 row now reads "(`min_herdr_version` and `platforms` ship in `repo-foundation`, not
  here.)".
- Correct § **Environment**: `clippy` and `cargo-llvm-cov` have been installed since
  `repo-foundation` task 4.1, so "are **not** installed" is false; the sentence becomes a
  record of what to install on a fresh machine.
- Fold § **"Deferred: sync README's keymap at the end of Phase 4"** into the historical
  record: Phase 4 closed and the keymap was synced, so a section headed "Deferred" is a
  false signal.
- Add one line under the title stating that the file is a closed record of a completed
  project, that open work lives in OpenSpec changes, and that a section describing something
  as deferred is describing the past.

**Why not delete the file:** it is the only place holding findings the project says should
transfer beyond it — "a fork is not a sandbox", the multi-subject `SCAN_MIN` floor, the outer
test that dodges its own scenario, the `%G?` signing trap, and the SSH/`gh` push workaround
that is still the working procedure. Deleting it would discard live operational knowledge to
fix a stale to-do list.

**Why not leave it and only correct the facts:** the defect a cold reader hits is *structural*,
not factual. A file headed "Project complete — nothing is outstanding" that contains sections
headed "Known-deferred doc fixes" and "Deferred: …" reads as self-contradiction regardless of
whether the individual sentences inside them are accurate. Removing the open-work framing is
the fix; correcting the facts alone is not.

**This decision has no mechanical check, and this change does not invent one.** "Does this
file read as a live to-do list?" has no second site. A check forbidding a heading beginning
"Deferred" would be a lint on English, would forbid a legitimate future use of the file, and
would not catch the next instance if it were worded differently. The structural mitigation
is that open work in this repository lives in OpenSpec changes and is reported by
`openspec list`, so `HANDOFF.md` has no reason to carry a to-do list again.

### 6. The context block is read as text, not parsed as YAML

`openspec/config.yaml`'s `context` is a folded scalar. Every claim checked in it is a phrase
(`fixture repositories`, `make gates`), so a substring search over the file is sufficient and
is immune to how the block is folded, indented, or re-wrapped. Parsing YAML would add a
dependency edge from a test to `yaml-rust2` for no additional precision.

**Trade-off accepted:** a substring search cannot tell the `context` block from the `rules`
block. The check scopes itself to the text between `context: |` and the next top-level key
(`rules:`) to keep the claim honest, and fails naming that boundary if it cannot find it.

### 7. The `Makefile` extractor is specified against the recipes that exist, not against an idealised one

A first draft of leg 5 said "take each recipe line, strip leading `env …` and `VAR=value`
assignments, take the first remaining token". Run by hand against the real `Makefile`, that
rule produces garbage on two shapes it contains:

- `lint:` and `coverage:` each open with `@if ! cargo <tool> --version >/dev/null 2>&1; then
  echo "error: …" 1>&2; exit 1; fi` spread over continuation lines. The naive rule yields
  `@if`, `echo`, `exit`, and `fi` as "programs a reader must document" — and worse, `if` and
  `fi` occur incidentally as English words in `README.md`, so depending on how "documented"
  is matched the leg could equally *pass vacuously* on them. Both outcomes are defects.
- `gates:` line 35 is `LAUNCH=src/open.rs ENTRY='pub fn run_from_env\(' /bin/sh
  scripts/gates/launchseam.sh`. A whitespace-split tokenizer splits the quoted value, the
  assignment-strip stops at `fn`, and the leg reports **`fn`** as an undocumented program.

The rule in the spec is therefore stated in six explicit steps — join continuations, strip
`@`/`-`, tokenize quote-aware, strip `^[A-Za-z_][A-Za-z0-9_]*=` assignments and `env`, take
the first remaining token, then discard `cargo`, `/bin/sh <scripts/ path>`, and a named list
of shell keywords and builtins. Two of leg 5's scenarios drive exactly these two real shapes
as synthetic `Makefile` inputs, so the rule is tested against the formatting the codebase
actually uses rather than against a tidy imagined one — the project's standing rule that a
check must *see* what it guards.

**Alternative considered:** hardcode the checked set as `{python3}` and skip extraction
entirely. Rejected: it would go green forever after a future gate adds `node` or `jq`, which
is the drift this leg exists to prevent. The extractor's own emptiness is guarded by the
"extractor yields nothing" scenario.

**Whole-word, section-scoped matching** applies to every leg that asks "is X named in this
document": a bare substring search over a whole file is how `if` would have passed.

### 8. Which claims get a check, and which get an argument

Bound (six legs): the module map, the tested-modules list, the worker-thread count, the MSRV,
the gate-path programs, the manifest transcription, and the two context claims. Each has a
value or set the repository determines.

Not bound, deliberately: `HANDOFF.md`'s disposition (Decision 5), `openspec/config.yaml`'s
nvm node path (a machine fact, not a repository fact — the path it names,
`$HOME/.nvm/versions/node/v24.20.0/bin`, was verified to exist and is left alone), and the
wording of any individual prose sentence. `tests/manifest.rs`'s header states the principle this follows —
confine the check to values with a second site, because a check on anything else either
cannot fail or fails for reasons that are not defects.

## Risks / Trade-offs

- **Eight sibling changes edit the same documents concurrently**, and all eight name
  `SPEC.md` → this change's checks assert against `README.md`, `AGENTS.md`, `Cargo.toml`,
  `Makefile`, `herdr-plugin.toml`, `openspec/config.yaml`, and **four named sections** of
  `SPEC.md`; a sibling editing any other part of that file cannot turn a leg red. A merge
  conflict in the prose is possible; a contradictory check is not. Group 12 re-runs
  `make check` in full.
- **A parser bound to a heading is itself a drift surface** — renaming `### Unit-tested
  modules` breaks the leg → each parser fails loudly naming the heading it searched for,
  never silently, and that behaviour is its own scenario. A failure that says "the section is
  gone" is a correct outcome, not a false positive: the section is where the contract lives.
- **The gate-program leg could pass vacuously** if the extraction rule yields an empty set →
  the leg asserts the computed set is non-empty (it contains `python3` today) before checking
  documentation, so a broken extractor fails rather than passing.
- **Correcting `openspec/config.yaml` changes every future agent's prompt** → the edit is one
  false sentence replaced by a true one and one command added to a list; the `rules` and
  `operations` blocks the schema behaviour depends on are untouched.
- **Coverage floor** — `tests/` files are integration targets and are not counted against the
  crate's line coverage; the 80% floor is unaffected because no `src/` line is added. If
  `cargo llvm-cov` reports a shift, the response is to add tests, never to lower the floor.
- **Binding order in the manifest transcription is stricter than TOML requires** → accepted
  deliberately (Decision 4); the cost is one block move whenever the manifest's table order
  changes, which has happened once in the project's life.

## Migration Plan

None required. No data, no format, no deployment. The release binary is unchanged — `tests/`
is not compiled into it — so nothing needs rebuilding, reinstalling, or relinking in Herdr.

**Rollback** is `git revert` of this change's commits: the new test file disappears and the
documents return to their previous text. Nothing else observes the change.

**Order within the change:** the checks are written before the corrections (RED), so each leg
is observed failing against the real, uncorrected documents before the corrections turn it
green. That is the project's TDD schema, and here it doubles as the proof that each leg can
fail — the planted-defect tier then covers the legs that were already green when written.

## Open Questions

None. Two judgment calls are recorded as decisions rather than questions: `HANDOFF.md`'s
disposition (Decision 5 — corrected and closed, not deleted) and D10's manifest ordering
(Decision 4 — fixed in `SPEC.md`, not declared a non-issue). Both are reversible in a single
commit if the user prefers otherwise.

## Visual Design

Not applicable. This change modifies no user-facing view and no email template; it adds a
test target and corrects prose. No design source exists or is needed.
