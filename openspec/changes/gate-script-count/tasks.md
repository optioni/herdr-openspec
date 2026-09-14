<!-- Groups 1 and 2 are sequential: group 2's spec figure must match the literal group 1
     asserts, and a mismatch between them is the defect this change exists to prevent.
     Group 3 (Documentation) shares no file with either and could run beside them, but is
     small enough that the ordering costs nothing. -->

## 1. Bind the gate-script count to the directory
<!-- kind: operational -->

The new assertion pins an invariant that already holds — `scripts/gates/` holds 31 files
today — so it is green on arrival and proves nothing without its negative control. Both
directions of that control are run below, per design.md → Test Boundaries.

- [x] 1.1 CHECK: Record the gap this change closes. Plant a 32nd script and its `gates:`
  recipe line, run `cargo test --test ci_workflow`, and confirm it **passes** — the count is
  bound to nothing. Then delete an existing script with its recipe line and confirm it passes
  again. Remove both plants and confirm `git status --short -- Makefile scripts/gates` is empty.
  **Run at HEAD `9c37085`:** plant → `test result: ok. 21 passed; 0 failed`; deletion →
  `test result: ok. 21 passed; 0 failed`; tree clean after both. The existing
  `assert!(on_disk.len() >= 25, …)` floor fires in neither direction.
  **Re-confirmed at HEAD `db5ebc0`** (docs-only commits since): plant → `ok. 21 passed;
  0 failed`; deletion → `ok. 21 passed; 0 failed`; tree clean after both.
- [x] 1.2 CHANGE: In `tests/ci_workflow.rs`'s
  `every_gate_script_the_recipe_names_exists_and_every_script_is_named`, replace the `>= 25`
  floor with an equality against the figure `specs/quality-gates` states. The message names
  both counts and the spec sentence to edit, so a reader who adds a gate is told which
  sentence moved rather than only that a number did.
- [x] 1.3 VERIFY: Negative control, both directions. Re-plant the 32nd script with its recipe
  line and confirm the suite now **fails**, naming 32 against 31; remove it and confirm green.
  Delete a script with its recipe line and confirm it fails naming 30 against 31; restore it
  and confirm green. Record all four runs.
  **Run at HEAD `db5ebc0`** (`plant32.sh` planted with its recipe line; `nosleep.sh` the one
  deleted): plant → `FAILED. 20 passed; 1 failed`, `left: 32 / right: 31`, message naming
  `openspec/specs/quality-gates/spec.md` and both scenarios; removed → `ok. 21 passed`;
  deletion → `FAILED. 20 passed; 1 failed`, `left: 30 / right: 31`; restored → `ok. 21
  passed`. `git status --short -- Makefile scripts/gates` empty after each restore.
- [x] 1.4 VERIFY: `cargo test --test ci_workflow` — **21** passing, no regressions. Twenty-one,
  not twenty-two: 1.2 replaces an assertion inside an existing test rather than adding a new
  one, so the count does not move. **Run at HEAD `9c37085`:** `21 passed; 0 failed`.
  **Re-run after 1.2 at HEAD `db5ebc0`:** `21 passed; 0 failed`.

## 2. Correct the figures, and attribute the historical ones
<!-- kind: operational -->

- [x] 2.1 CHECK: Confirm the three current figures and the one historical figure are still
  where this change expects them, and that no concurrent change has moved them.
  **Run at HEAD `9c37085`:** `grep -nE 'twenty-eight|twenty-six'
  openspec/specs/quality-gates/spec.md` → `:49`, `:71`, `:127`, `:139`, `:140`, `:187`,
  `:210`, `:755`. `ls scripts/gates/ | wc -l` → `31`.
- [x] 2.2 CHANGE: Edit the **delta** at `openspec/changes/gate-script-count/specs/quality-gates/
  spec.md`, never `openspec/specs/quality-gates/spec.md` — the live spec is written only by
  `openspec archive`, and editing it during apply would make the two disagree in the opposite
  direction. Line references below are into the live spec, which is where the figures are read
  from: `:140`, `:187`, `:210` from twenty-eight to thirty-one, `:139`/`:187`'s non-`cargo`
  count from twenty-six to twenty-nine, and the requirement's new paragraph stating that the
  count is asserted rather than written down.
  **Already landed:** the planning-review repair (commit `db5ebc0`) wrote these into the delta
  when it fixed the enumeration CRITICAL, since the delta *is* a planning artifact. Verified at
  HEAD `9d40d19` by diffing each MODIFIED requirement block against the live spec: the only
  differences are exactly the edits tasks 2.2, 2.4, 2.5 and 2.6 describe, and nothing else.
- [x] 2.3 CHECK: Verify the non-`cargo` figure by measurement, not arithmetic. `grep -ln cargo
  scripts/gates/*` returns five files, of which `colwidth.sh`, `noio-view.sh` and `wired.sh`
  name `cargo` only inside comments; `deps.sh` and `build-graph.sh` are the two that invoke
  it, so the figure is 29 of 31. **Run at HEAD `9c37085`:** five hits, three comment-only,
  confirmed by reading each line.
  **Re-run at HEAD `9d40d19`:** `grep -ln cargo scripts/gates/*` → `build-graph.sh`,
  `colwidth.sh`, `deps.sh`, `noio-view.sh`, `wired.sh`; the hits in `colwidth.sh:10`,
  `wired.sh:209` and `noio-view.sh:31` are each on a `#`-prefixed comment line. 29 of 31
  confirmed by measurement.
- [x] 2.4 CHANGE: Correct `NODEFAULT-UI`'s subject-set count from five to **seven** at both
  sites the delta carries — the multi-subject sentence and the `SCAN_MIN`-inventory bullet.
  **Run at HEAD `9c37085`:** `grep -c SCAN_MIN Makefile` → `7`; `grep -rn five tests/*.rs |
  grep -i 'nodefault\|scan_min'` → no output, so nothing binds it; `grep -n seventh
  tests/gate-controls.toml` → `:425`, which already says seven. In scope per proposal → Why: a
  MODIFIED block re-lands its content as current at archive time.
  **Already landed in the delta** (see 2.2). Re-confirmed at HEAD `9d40d19`: all three
  measurements identical, and the delta reads "seven type sets" and "seven `SCAN_MIN` values"
  at both sites.
- [x] 2.5 CHANGE: Re-count the extracted-gate enumeration against the directory rather than by
  arithmetic, and confirm `list + OPENSPEC-UNTOUCHED + deps.sh + build-graph.sh` equals the
  directory count. **Run at HEAD `9c37085`:** the enumeration omitted `COLWIDTH`, `PALETTE` and
  `HELPWIDTHS`, so it read twenty-five against a directory of twenty-eight; with them the chain
  is 28 + 1 = 29 and 29 + 2 = 31.
  **Already landed in the delta** (see 2.2). Re-verified at HEAD `9d40d19` mechanically, by
  mapping every backtick-delimited gate name in the enumeration onto a file under
  `scripts/gates/`: 28 names → 28 distinct files, none unmapped; the three files no name covers
  are exactly `openspec-untouched.sh`, `deps.sh` and `build-graph.sh`; 28 + 1 = 29, 29 + 2 = 31
  = `ls scripts/gates/ | wc -l`. The list is the measurement and the figure is derived from it,
  per design.md → Decisions 2.
- [x] 2.6 CHANGE: Attribute the two unattributed historical figures — `:127`'s "left
  twenty-eight outside" and `:755`'s "every one of the twenty-eight gate scripts", the latter
  naming `gate-integrity` (archived `2026-09-07`). `:49` and `:71` already name
  `degraded-states` and are left alone.
  **Already landed in the delta** (see 2.2): `:12` of the delta carries "(the count at
  `spec-purposes`' own base; see the historical-figure convention below)" and `:207` carries
  "measured by `gate-integrity` (archived `2026-09-07`)".
- [x] 2.7 VERIFY: `grep -n 'twenty-eight' openspec/specs/quality-gates/spec.md` — every
  surviving occurrence is either an attributed historical figure or part of the paragraph
  describing the drift itself. No unattributed current figure remains.
  **Task clarification:** run against the **delta**, not the live spec. Task 2.2 forbids
  editing `openspec/specs/quality-gates/spec.md` during apply, so the live file still carries
  the uncorrected figures by design; the delta is what archive re-lands, and is therefore the
  file this check has a subject in. **Run at HEAD `9d40d19`** over
  `openspec/changes/gate-script-count/specs/quality-gates/spec.md` — six occurrences, each
  accounted for: `:12` attributed (`spec-purposes`' base), `:20` a **current** figure that is
  true (28 enumerated gates, verified in 2.5), `:94` and `:99` inside the paragraph describing
  the drift, `:102` quoting `:12` to illustrate the convention, `:207` attributed
  (`gate-integrity`). No unattributed stale figure remains.

## 3. Documentation
<!-- kind: operational -->

- [x] 3.1 Rewrite in `AGENTS.md`: `NODEFAULT-UI`'s subject-set count at `:266` and `:270`
  (audience: every future session) — both read "five", and the `Makefile` carries seven. This
  is the same figure task 2.4 corrects in the spec; leaving `AGENTS.md` behind would recreate
  the two-files-disagree state this change exists to end.
  **Done at HEAD `f258715`:** both sites read seven, and the parenthetical now enumerates all
  seven type sets rather than five — `ArtifactSection` (`foldable-spec-sections`' sixth) and
  `src/ui/help.rs`'s `Binding`/`Group` (`help-overlay`'s seventh) were the two missing, matching
  the `Makefile`'s seven `SCAN_MIN` lines one for one. `grep -n 'NODEFAULT-UI' AGENTS.md` now
  returns a single line, so no second copy of the figure survives.
- [x] 3.2 Rewrite in `AGENTS.md`: the `tests/ci_workflow.rs` sentence under Quality gates
  (audience: every future session) — it currently says that file "proves a narrower thing
  beside it: the recipe names every script under `scripts/gates/` and vice versa". That is now
  incomplete: it also pins the count. One clause, naming what breaks when a gate is added, so
  the next session extracting a gate knows two files move and not one.
  **Done at HEAD `f258715`:** the sentence now says `tests/ci_workflow.rs` proves *two* narrower
  things — the correspondence, and the file count against the spec's figure by equality rather
  than floor — and names the consequence: extracting a gate moves three sites (the script, the
  recipe line, and the count in both the test and the spec sentence the failure message names).
  `cargo test --all-features` green afterwards, so no `doc_contract` claim was disturbed.

## 4. Change Review
<!-- kind: operational -->

- [ ] 4.1 CHECK: Dispatch `outside-in-tdd-reviewer` against proposal, specs, design, tasks and
  the diff. Point it at the one question that matters here — whether the assertion can fail in
  both directions, and whether any *other* count in `quality-gates` is unbound in the same way
  this one was.
- [ ] 4.2 CHANGE: Fix every CRITICAL, resolve or accept each WARNING with a one-line reason,
  note SUGGESTIONs, re-run affected tests.
- [ ] 4.3 VERIFY: Confirm no blocking or unowned finding remains.

## 5. Lint & Verify
<!-- kind: operational -->

- [ ] 5.1 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.
- [ ] 5.2 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 5.3 VERIFY: `cargo test --all-features` — green, on a quiescent tree.
  `tests/gate_controls.rs` digests file mtimes repository-wide, so this is attributable only
  when nothing else is writing.
- [ ] 5.4 VERIFY: `make gates` — exit 0.
- [ ] 5.5 VERIFY: `make check` as the single gate, naming the failing sub-command if it fails.
- [ ] 5.6 VERIFY: `openspec validate gate-script-count --strict` — valid.
