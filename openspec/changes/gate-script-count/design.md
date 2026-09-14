## Context

`openspec/specs/quality-gates/spec.md` states that `scripts/gates/` holds twenty-eight files.
It holds thirty-one. Measured at the tree that archived `help-overlay`:

```
$ ls scripts/gates/ | wc -l
31
$ git show 0745e70^:scripts/gates | tail -n +3 | grep -c .
30
```

So the figure was already wrong by two before `help-overlay` added `helpwidths.sh`. It drifted
across at least three changes without anyone noticing, which is the part worth fixing — the
number itself is a one-line edit.

Nothing binds it. `tests/ci_workflow.rs`'s
`every_gate_script_the_recipe_names_exists_and_every_script_is_named` is thorough about
*correspondence*: every file on disk must be named in the `gates:` recipe, every
`scripts/gates/<name>` the recipe names must exist, and the two sets must be equal. Its only
cardinality assertion is:

```rust
assert!(on_disk.len() >= 25, "expected at least 25 files under scripts/gates/, found {}", …);
```

A floor. Adding a gate with its recipe line keeps every correspondence assertion green and
clears the floor by six, so the prose drifts silently. That is precisely the failure mode
`AGENTS.md` has a standing rule against: *a documented claim with a computable second site is
bound to that site inside `cargo test`, not left to a human re-reading it.*

## Goals / Non-Goals

**Goals.** Bind the gate-script count. Correct every figure that is both false today and inside
a requirement this change already modifies — the gate-script count, the enumeration it
summarises, and `NODEFAULT-UI`'s subject-set count (see Decisions 1 and 2). Attribute the
historical figures so the next sweep can tell a superseded number from a deliberate one.

**Non-Goals.** Not re-auditing `quality-gates`' numerals outside the modified requirements; not
binding `NODEFAULT-UI`'s count, only correcting it; not changing which gates exist, what they
check, their floors, or the recipe; not extracting `EXTENDED` or `TESTCOUNT`; not generalising
to other prose counts, which already have second sites.

## Boundaries

One test file and one spec. `tests/ci_workflow.rs` already reads `scripts/gates/` with
`fs::read_dir` and already owns the recipe/directory correspondence, so the count belongs
beside it rather than in a new file or a new gate script — a gate that checked a spec numeral
would be a gate checking prose, which is the contract tier's job and not `make gates`'.

This follows the existing contract-tier pattern exactly: `tests/manifest.rs` binds the
manifest/README/binary-name triangle, `tests/degraded_coverage.rs` binds `SPEC.md`'s table to
named tests, `tests/doc_contract.rs` binds ten documented claims to their computing sites. This
change adds an eleventh claim of the same shape to the file that already computes it.

**Which count to assert.** Files under `scripts/gates/`, all extensions, matching what
`on_disk` already collects — thirty-one, of which thirty are `.sh` and one (`gate-mech1.py`) is
Python. Not "recipe lines": the recipe names `nodefault-ui.sh` seven times and
`deps.sh` twice, so line count and file count are different numbers, and the spec's sentence is
about the directory.

## Contracts

None. No interface any separate consumer depends on. Additive.

## Persistence and Rollout

Migration: none. Backfill: none. Seeding: none. Cache invalidation: none. Index rebuild: none.
Authorization: none. Observability: none. Deployment: none. The change is one assertion and
four sentences.

## Test Boundaries

- **The filesystem is real.** `tests/ci_workflow.rs` reads the actual `scripts/gates/`
  directory through `fs::read_dir`, as it already does. No fixture, no scratch tree, no
  temporary directory: the assertion's subject *is* the repository's own tree, and a fixture
  would assert something about the fixture.
- **The spec file is not read, and the reason is the archive boundary — not parsing.** The
  asserted figure is a literal in the test. An earlier draft justified that by saying a parser
  would make the test's own failure mode a regex bug; planning review rightly rejected the
  framing, because the middle option is not a parser. `tests/ci_workflow.rs` already carries
  `read_spec_md()` and already asserts document content elsewhere, so
  `assert!(spec.contains("— **thirty-one** in all, counting"))` was available and is no parser
  at all.

  The real obstacle is **when `openspec/specs/` changes**. It is written only by `openspec
  archive`: verified on the change that precedes this one — every edit to `openspec/specs/`
  across the whole of `help-overlay` lands in its single archive commit `9c37085`, and the live
  `quality-gates` spec still reads "twenty-eight in all" today. So a test asserting the live
  spec's content would go **red the moment this change's assertion lands and stay red for the
  entire apply phase**, until archive — and `make check` gates every commit in between. A check
  that must be red for a dozen commits is not a check.

  What the literal therefore buys, honestly stated: the directory is bound to the test, and the
  test is bound to the spec sentence by the **failure message** naming it, not by machine. The
  surviving failure mode is real and worth writing down — a developer adds a gate, sees red,
  bumps the literal to 32, and leaves the spec at thirty-one. The message is the only thing
  standing against that, so it must name the file, the sentence, and both counts. This is a
  **deliberate asymmetry with `doc_contract`**, which does read its documents: those live in the
  repository root and change in the same commit as the code, where `openspec/specs/` does not.
- **No process is spawned.** No `cargo`, no `make`. The test reads a directory.
- **The negative control is a planted file**, not an attestation: the scenario requires that
  adding a thirty-second script *with* its recipe line — so the correspondence assertions stay
  green — fails the new assertion, and that deleting one fails it from the other side. Both
  are to be run and recorded, on this repository's standing rule that a gate is executed
  against a planted defect rather than believed.

## Decisions

**1. `NODEFAULT-UI`'s count is corrected but not bound.** Planning review found a second false
figure one requirement away: the spec says `NODEFAULT-UI` scans "five type sets" with "five
`SCAN_MIN` values" and the `Makefile` carries seven. It is corrected here, not deferred,
because both sites sit inside requirement blocks this delta reproduces — and a MODIFIED block
**re-lands its content as current** at archive time, converting a stale sentence into a freshly
asserted one. It is not *bound* here: that needs a second mechanism against a second subject
(the recipe's `SCAN_MIN` lines, not a directory listing), and bundling it would make this
change two changes. Whoever next touches that gate inherits the argument, which
`tests/gate-controls.toml:425` already half-states by naming "the Makefile's seventh line".

**2. Counts derived from an enumeration are re-counted, never adjusted.** This change's own
first draft moved the extracted-gate figure from twenty-six to twenty-nine by arithmetic —
31 − 2 — without re-counting the list of gate names that figure summarises. The list was
missing `COLWIDTH`, `PALETTE` and `HELPWIDTHS`, so the corrected sentence enumerated
twenty-five items and called them twenty-nine. Planning review caught it. The rule this yields
is worth more than the fix: **where a figure summarises a list, the list is the measurement and
the figure is derived from it** — re-deriving the figure from a different source, even a
correct one, leaves the two disagreeing.

## Open Questions

None.

## Risks

**The assertion becomes a tax on adding a gate.** Every future gate now edits a test literal
and a spec sentence. That is the intended cost — it is the same cost `NOIO-VIEW`'s `PURE` list
and `doc_contract`'s claim count already impose, and it is what makes the prose trustworthy.
The failure message names both counts and the file to edit, so the tax is one obvious edit and
not a hunt.

**The historical figures could be "corrected" by a later sweep.** Four figures in this
capability describe an earlier tree. Two (`:49`, `:71`) already name `degraded-states`; this
change attributes the other two. A sweep that changes an attributed figure is then visibly
wrong rather than plausibly right — which is the outcome `help-overlay`'s Change Review asked
for after nearly turning a true sentence false.
