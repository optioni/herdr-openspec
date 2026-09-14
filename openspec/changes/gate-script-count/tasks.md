<!-- Groups 1 and 2 are sequential: group 2's spec figure must match the literal group 1
     asserts, and a mismatch between them is the defect this change exists to prevent.
     Group 3 (Documentation) shares no file with either and could run beside them, but is
     small enough that the ordering costs nothing. -->

## 1. Bind the gate-script count to the directory
<!-- kind: operational -->

The new assertion pins an invariant that already holds — `scripts/gates/` holds 31 files
today — so it is green on arrival and proves nothing without its negative control. Both
directions of that control are run below, per design.md → Test Boundaries.

- [ ] 1.1 CHECK: Record the gap this change closes. Plant a 32nd script and its `gates:`
  recipe line, run `cargo test --test ci_workflow`, and confirm it **passes** — the count is
  bound to nothing. Then delete an existing script with its recipe line and confirm it passes
  again. Remove both plants and confirm `git status --short -- Makefile scripts/gates` is empty.
  **Run at HEAD `9c37085`:** plant → `test result: ok. 21 passed; 0 failed`; deletion →
  `test result: ok. 21 passed; 0 failed`; tree clean after both. The existing
  `assert!(on_disk.len() >= 25, …)` floor fires in neither direction.
- [ ] 1.2 CHANGE: In `tests/ci_workflow.rs`'s
  `every_gate_script_the_recipe_names_exists_and_every_script_is_named`, replace the `>= 25`
  floor with an equality against the figure `specs/quality-gates` states. The message names
  both counts and the spec sentence to edit, so a reader who adds a gate is told which
  sentence moved rather than only that a number did.
- [ ] 1.3 VERIFY: Negative control, both directions. Re-plant the 32nd script with its recipe
  line and confirm the suite now **fails**, naming 32 against 31; remove it and confirm green.
  Delete a script with its recipe line and confirm it fails naming 30 against 31; restore it
  and confirm green. Record all four runs.
- [ ] 1.4 VERIFY: `cargo test --test ci_workflow` — **21** passing, no regressions. Twenty-one,
  not twenty-two: 1.2 replaces an assertion inside an existing test rather than adding a new
  one, so the count does not move. **Run at HEAD `9c37085`:** `21 passed; 0 failed`.

## 2. Correct the figures, and attribute the historical ones
<!-- kind: operational -->

- [ ] 2.1 CHECK: Confirm the three current figures and the one historical figure are still
  where this change expects them, and that no concurrent change has moved them.
  **Run at HEAD `9c37085`:** `grep -nE 'twenty-eight|twenty-six'
  openspec/specs/quality-gates/spec.md` → `:49`, `:71`, `:127`, `:139`, `:140`, `:187`,
  `:210`, `:755`. `ls scripts/gates/ | wc -l` → `31`.
- [ ] 2.2 CHANGE: Edit the **delta** at `openspec/changes/gate-script-count/specs/quality-gates/
  spec.md`, never `openspec/specs/quality-gates/spec.md` — the live spec is written only by
  `openspec archive`, and editing it during apply would make the two disagree in the opposite
  direction. Line references below are into the live spec, which is where the figures are read
  from: `:140`, `:187`, `:210` from twenty-eight to thirty-one, `:139`/`:187`'s non-`cargo`
  count from twenty-six to twenty-nine, and the requirement's new paragraph stating that the
  count is asserted rather than written down.
- [ ] 2.3 CHECK: Verify the non-`cargo` figure by measurement, not arithmetic. `grep -ln cargo
  scripts/gates/*` returns five files, of which `colwidth.sh`, `noio-view.sh` and `wired.sh`
  name `cargo` only inside comments; `deps.sh` and `build-graph.sh` are the two that invoke
  it, so the figure is 29 of 31. **Run at HEAD `9c37085`:** five hits, three comment-only,
  confirmed by reading each line.
- [ ] 2.4 CHANGE: Correct `NODEFAULT-UI`'s subject-set count from five to **seven** at both
  sites the delta carries — the multi-subject sentence and the `SCAN_MIN`-inventory bullet.
  **Run at HEAD `9c37085`:** `grep -c SCAN_MIN Makefile` → `7`; `grep -rn five tests/*.rs |
  grep -i 'nodefault\|scan_min'` → no output, so nothing binds it; `grep -n seventh
  tests/gate-controls.toml` → `:425`, which already says seven. In scope per proposal → Why: a
  MODIFIED block re-lands its content as current at archive time.
- [ ] 2.5 CHANGE: Re-count the extracted-gate enumeration against the directory rather than by
  arithmetic, and confirm `list + OPENSPEC-UNTOUCHED + deps.sh + build-graph.sh` equals the
  directory count. **Run at HEAD `9c37085`:** the enumeration omitted `COLWIDTH`, `PALETTE` and
  `HELPWIDTHS`, so it read twenty-five against a directory of twenty-eight; with them the chain
  is 28 + 1 = 29 and 29 + 2 = 31.
- [ ] 2.6 CHANGE: Attribute the two unattributed historical figures — `:127`'s "left
  twenty-eight outside" and `:755`'s "every one of the twenty-eight gate scripts", the latter
  naming `gate-integrity` (archived `2026-09-07`). `:49` and `:71` already name
  `degraded-states` and are left alone.
- [ ] 2.7 VERIFY: `grep -n 'twenty-eight' openspec/specs/quality-gates/spec.md` — every
  surviving occurrence is either an attributed historical figure or part of the paragraph
  describing the drift itself. No unattributed current figure remains.

## 3. Documentation
<!-- kind: operational -->

- [ ] 3.1 Rewrite in `AGENTS.md`: `NODEFAULT-UI`'s subject-set count at `:266` and `:270`
  (audience: every future session) — both read "five", and the `Makefile` carries seven. This
  is the same figure task 2.4 corrects in the spec; leaving `AGENTS.md` behind would recreate
  the two-files-disagree state this change exists to end.
- [ ] 3.2 Rewrite in `AGENTS.md`: the `tests/ci_workflow.rs` sentence under Quality gates
  (audience: every future session) — it currently says that file "proves a narrower thing
  beside it: the recipe names every script under `scripts/gates/` and vice versa". That is now
  incomplete: it also pins the count. One clause, naming what breaks when a gate is added, so
  the next session extracting a gate knows two files move and not one.

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
