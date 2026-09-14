## Why

`openspec/specs/quality-gates/spec.md` states that `scripts/gates/` holds **twenty-eight**
files. It holds **31**. The count was already wrong by two before `help-overlay` added
`helpwidths.sh` and made it three, and nobody noticed across three changes — because nothing
binds the numeral. `tests/ci_workflow.rs`'s
`every_gate_script_the_recipe_names_exists_and_every_script_is_named` proves *correspondence*
between the directory and the `gates:` recipe in both directions, and asserts only
`on_disk.len() >= 25` — a floor that 31 satisfies as comfortably as 28 did.

That is the exact failure this repository has a standing rule against: a documented claim with
a computable second site, left to a human re-reading it.

Planning review found a **second** figure of the same shape, one requirement away:
`NODEFAULT-UI` is described as scanning "five type sets" with "five `SCAN_MIN` values", and the
`Makefile` carries **seven** (`grep -c SCAN_MIN Makefile`). `foldable-spec-sections` added the
sixth and `help-overlay` the seventh, neither propagating to the prose — and
`tests/gate-controls.toml:425` already names "the Makefile's seventh line", so two checked-in
files disagree today and nothing notices. It is in scope because a **MODIFIED block re-lands
its content as current at archive time**: carrying a known-false figure through one is worse
than leaving it, since it converts a stale sentence into a freshly asserted one.

## What Changes

- A new assertion binds `scripts/gates/`'s own file count to the figure the spec states, so the
  next extracted gate either corrects the prose or fails `cargo test`. Replacing the existing
  `>= 25` floor with an exact count is the mechanism; the floor stays as-is for nothing.
- The **current** figures in `quality-gates`' spec go from 28 to 31, and every count derived
  from them is re-measured against the directory rather than adjusted by arithmetic. Planning
  review caught exactly that error in this change's own first draft: the extracted-gate
  enumeration lists gates by name, and `COLWIDTH`, `PALETTE` and `HELPWIDTHS` were in none of
  them, so the list read twenty-five where the directory says twenty-eight.
- `NODEFAULT-UI`'s subject-set count goes from five to **seven** at both sites, and
  `AGENTS.md`'s two copies of the same figure with it.
- The four **historical** figures are left as figures but gain the commit or change that makes
  each true, so the next sweep can tell a superseded number from a deliberate one. This is the
  convention `help-overlay`'s Change Review named after nearly "repairing" a correct sentence:
  `specs/dashboard-loop:774`'s "six times today" verifies only against that change's base.

## Non-Goals

- **Not** re-auditing every numeral in `quality-gates`' 17 requirements. Scope is the two
  figures that are both **false today** and **inside a requirement this change already
  modifies** — the gate-script count and `NODEFAULT-UI`'s subject-set count. A false numeral
  elsewhere in the capability stays for its own change.
- **Not** changing which gates exist, what they check, their floors, or the `gates:` recipe.
- **Not** extracting `EXTENDED` or `TESTCOUNT`, which are deliberately unextracted.
- **Not** generalising to other prose counts (the pure-view set, the worker-thread count, the
  `doc_contract` claim count). Those already have second sites; these two did not.
- **Not** binding `NODEFAULT-UI`'s subject-set count to the `Makefile` the way the gate-script
  count is bound to its directory. Correcting it is in scope because a MODIFIED block re-lands
  it; binding it is a second mechanism against a second subject, and belongs to whichever
  change next touches that gate.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `quality-gates`: the scenario stating what `scripts/gates/` holds gains an exact,
  machine-bound count, and the requirement stating it says the count is asserted rather than
  written down. The historical-figure convention is stated where those figures live.

## Impact

- `openspec/specs/quality-gates/spec.md` — one requirement and its first scenario; four
  sentences carrying the figure.
- `tests/ci_workflow.rs` — `every_gate_script_the_recipe_names_exists_and_every_script_is_named`
  has its `>= 25` floor replaced by the count assertion. No new test: the file stays at 21.
- No production code. No gate script, no `Makefile` recipe line, no CI job.
- `AGENTS.md` — two copies of `NODEFAULT-UI`'s five/seven figure (`:266`, `:270`). It names no
  gate-script *count* and is unaffected by that half — verified: `grep -c 'twenty-eight'
  AGENTS.md` = 0.
