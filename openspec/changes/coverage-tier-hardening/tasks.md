# Tasks — coverage-tier-hardening

Baselines measured at HEAD `fbfae77`, each with the command that produced it:

| Fact | Value | Command |
|---|---|---|
| `check:`'s prerequisites | `fmt-check lint gates test coverage` | `grep -n '^check:' Makefile` |
| `covers-check` occurrences in `Makefile` | 0 | `grep -c 'covers-check' Makefile` |
| Rows in the coverage map | 59 | `grep -c '^\[\[row\]\]' tests/degraded-coverage.toml` |
| `covers` ranges | 74 | `grep -o '"[^"]*:[0-9]*-[0-9]*"' tests/degraded-coverage.toml \| wc -l` |
| Ranges the new rule rejects | 1 (`src/tasks.rs:193-196`) | the enclosing-item sweep in task 1.1, run at planning time |
| `degraded_coverage` tests | 10 passed, 1 ignored | `cargo test --all-features --test degraded_coverage` |
| Scripts under `scripts/gates/` | 32 — must not move | `ls scripts/gates/ \| wc -l` |
| CI steps invoking `make` | 6 (`fmt-check`, `lint`, `gates`, `test`, `gates-full`, `coverage`) | `grep -n 'make ' .github/workflows/ci.yml` |
| Today's range rule | `tests/degraded_coverage.rs:386` | `grep -n 'holds_code' tests/degraded_coverage.rs` |
| `SPEC.md` → Gates says | "composes **five** gates" | `SPEC.md:1387-1396` |
| `ci-workflow` spec says | "**All four** gates run on each runner", "slowest of the **five** gates" | `openspec/specs/ci-workflow/spec.md:82,171` |
| `openspec/config.yaml` lists | five targets, no `covers-check` | `sed -n '49,51p' openspec/config.yaml` |
| `AGENTS.md` says | "**Five** are enforced in CI" | `AGENTS.md:278` |

**Ordering: sequential, no `parallel-after` on any group.** Cited rather than walked, per the
project rules' standing parallelism veto: one crate, one compile, and `make gates`/`make lint`/
the whole `--lib` suite sweep the entire tree, so criterion 3 fails for every pair here.

No acceptance-test group. This change adds no runtime behaviour — nothing the pane renders or
does moves — so there is no client-visible wiring for an outer loop to drive. Its outer loop is
`make check`, driven for real by group 2's checks. See design.md → Test Strategy.

## 1. The structural range rule
<!-- kind: behavior -->

- [x] 1.1 CHECK: Run the structural sweep over all 74 ranges and record which it flags, as the
      audit's input. **One** flagged at HEAD; the number is evidence, not a target.
      ```sh
      python3 - <<'EOF'
      import re
      # A field is recognised by its enclosing struct/enum extent - closing brace at the
      # SAME indentation - never by the shape of the line, which cannot tell
      # `name: Type,` from `name: expr,`. No brace counter: a `{` in a string literal
      # would desync it and the failure mode is vacuous acceptance.
      ITEM=re.compile(r'^\s*(pub(\([^)]*\))?\s+)?(unsafe\s+|async\s+)*'
                      r'(fn|struct|enum|impl|trait|mod|use|const|static|type)\b')
      SE=re.compile(r'^\s*(pub(\([^)]*\))?\s+)?(struct|enum)\b')
      ATTR=re.compile(r'^\s*#!?\[')
      DELIM=re.compile(r'^\s*[\)\}\]{,;]+\s*$')
      def decls(path):
          src=open(path).read().splitlines(); out=set(); se=None; pend=False
          for i,l in enumerate(src,1):
              t=l.strip(); ind=len(l)-len(l.lstrip())
              if se is not None and t.startswith('}') and ind==se:
                  se=None; out.add(i); continue
              if not t or t.startswith('//') or ATTR.match(l) or DELIM.match(l): out.add(i)
              elif ITEM.match(l):
                  out.add(i)
                  if SE.match(l) and '{' in l: se=ind
                  elif '{' not in l and not l.rstrip().endswith((';','}')): pend=True
              elif pend:
                  out.add(i)
                  if '{' in l or l.rstrip().endswith(';'): pend=False
              elif se is not None: out.add(i)
          return out
      txt=open("tests/degraded-coverage.toml").read(); cache={}; n=0; total=0
      for blk in txt.split("[[row]]")[1:]:
          cond=re.search(r'condition\s*=\s*"((?:[^"\\]|\\.)*)"',blk)
          for rg in re.findall(r'"([^"]+:\d+-\d+)"',blk):
              total+=1; f,span=rg.rsplit(":",1); a,b=(int(x) for x in span.split("-"))
              cache.setdefault(f,decls(f))
              if all(ln in cache[f] for ln in range(a,b+1)):
                  n+=1; print("REJECT",rg,"|",cond.group(1)[:70])
      print("ranges=%d rejected=%d" % (total,n))
      EOF
      ```
      Run at planning time with enclosing-item recognition: exit 0, **1 flagged** —
      `src/tasks.rs:193-196`. It also rejects `src/ui/app.rs:820-846` (the `settings-window`
      shape, still `Dashboard`'s fields at HEAD and wholly uninstrumented), and accepts
      `src/ui/list.rs:300-322` and `src/ui/view.rs:400-421`, both mis-bound for a reason no rule
      can see — see 3.2. A line-local field test instead flags 3 and misclassifies 26.8% of
      instrumented lines; that is why the spec mandates the extent rule.
- [x] 1.2 RED: Add three tests to `tests/degraded_coverage.rs`, each asserting the failure
      **message names the row's `condition`** — an error for an unrelated reason otherwise reads
      as green. Fixtures are ranges of the real tree, not synthetic files: `validate_covers`
      rejects paths outside `src/` and resolves against `manifest_dir()`, and
      `crate::testutil::ScratchDir` is `#[cfg(test)] pub(crate)` and unreachable from `tests/`.
      (a) signature-only — `src/tasks.rs:193-196`; (b) fields-only — `src/ui/app.rs:820-846` at
      HEAD; (c) the scanner control — a range inside a file holding a `{` in a string literal,
      asserting field recognition has not desynced, since that failure mode is **vacuous
      acceptance** of (b). All three MUST fail at HEAD. Verified with the current rule:
      ```sh
      python3 -c "
      lines=open('src/tasks.rs').read().splitlines()[192:196]
      print('accepted today:', any(l.strip() and not l.strip().startswith('//') for l in lines))"
      ```
      → `accepted today: True`, and the same for the `app.rs` shape (exit 0, both `True`).
- [x] 1.2b RED: Retain the old predicate as `legacy_holds_code` and assert it **accepts** both
      (a) and (b), so the strengthening is shown to be what catches them. Task 1.3 replaces the
      rule's use, not this predicate — deleting it outright leaves the control with nothing to
      assert against. It must pass before and after 1.3.
- [x] 1.3 GREEN: Replace `tests/degraded_coverage.rs:386`'s `holds_code` with the rule in
      `specs/degraded-coverage/spec.md` → failure condition 4c: a line is non-executable when it
      is blank, a comment, an item declaration, a struct field or enum variant, an attribute, or
      a lone delimiter; a range holding none other is rejected. Both 1.2 tests pass and the
      other 10 still do — `cargo test --all-features --test degraded_coverage`.
- [x] 1.4 GREEN: Add the acceptance half — a range starting at a `fn` line but including that
      function's body is accepted, and the `fn` line alone is rejected, so the rule turns on
      content rather than on the first line. Use `src/ui/view.rs:128-130` (`render_detail`'s
      `let ... else { return; }`), which the sweep in 1.1 does **not** flag.
- [x] 1.5 VERIFY: `cargo test --all-features --test degraded_coverage` — green, and the count is
      at or above 13 (10 at HEAD plus the three above).

## 2. `covers-check` as a named gate
<!-- kind: operational -->

- [x] 2.1 CHECK: Confirm `check:`'s prerequisite list and CI's per-target steps are what the
      baseline table records, so the two edits below are made against the measured state:
      `grep -n '^check:' Makefile` and `grep -n 'make ' .github/workflows/ci.yml`.
- [x] 2.2 CHANGE: Add a `covers-check` phony target running
      `cargo test --all-features --test degraded_coverage`, and compose it into `check` between
      `gates` and `test` (per design.md → Decision 1: this runs the existing binary rather than
      reimplementing the rule). `make covers-check` exits 0 on a clean tree.
- [x] 2.3 CHANGE: Update `tests/ci_workflow.rs`'s `check_composes_gates_third`, which asserts
      `parse_check_prereqs` **by equality** against
      `vec!["fmt-check", "lint", "gates", "test", "coverage"]` (`:741-742`). It goes red the
      moment 2.2 lands; `covers-check` belongs in that vector between `gates` and `test`.
- [x] 2.3b CHANGE: Add the matching CI step on both runners, between Gates and Test.
      `tests/ci_workflow.rs` asserts every `check:` prerequisite has a CI step and that Gates
      sits between Lint and Test; it goes red until both this and 2.2 land.
- [x] 2.6 CHANGE: `SPEC.md` → Gates — "composes five gates" → six, plus the `covers-check` row.
      `tests/ci_workflow.rs` requires a row named exactly `covers-check` once 2.2 lands.
- [x] 2.7 CHANGE: `AGENTS.md:278` "Five are enforced in CI" → six, plus its gate-table row.
      `CLAUDE.md` is a symlink to `AGENTS.md`, so this is one file.
- [x] 2.8 CHANGE: `README.md:123-124`'s prose enumeration — add `covers-check`.
- [x] 2.9 CHANGE: `openspec/config.yaml`'s context block — add `make covers-check`.
      `unrepresented_check_targets` reads `check:`'s prerequisites from the `Makefile`.
- [ ] 2.10 VERIFY: Prove the gate reports while the suite is red. Plant a failing unit test that
      clippy accepts — `assert_eq!(1 + 1, 3);`, measured; `assert!(false)` trips
      `clippy::assertions-on-constants` under `-D warnings` and would abort `make check` at lint
      two steps early — run
      `make check`, and confirm it exits non-zero at `cargo test` **with `covers-check` already
      passed**. This is a manual, one-off verification recorded in the commit message: the
      composite cannot run inside `cargo test` without unbounded recursion (design.md → Test
      Strategy), and it is run only after 2.6-2.9, since before them `make check` cannot be
      green for unrelated reasons; then additionally plant a struct-fields range and confirm it exits non-zero at
      `covers-check` instead. Remove both and confirm `make check` returns to exit 0.
- [ ] 2.11 VERIFY: Prove the floors did not move, on a tree that can tell the two apart. Build a
      scratch tree whose production-slice coverage is **below** its floor (delete a test that
      drives a well-covered module), then run both targets: `make covers-check` exits 0 and
      leaves `target/llvm-cov.json` absent, `make coverage` exits non-zero naming the floor. On
      a clean tree both exit 0, which discriminates nothing and is why the below-floor tree is
      the one the scenario names.
- [x] 2.12 VERIFY: `ls scripts/gates/ | wc -l` → **32**, unchanged. `covers-check` is not a
      hygiene gate and must not move the count `openspec/specs/quality-gates/spec.md` pins by
      equality.

## 3. The audit and the rebinding
<!-- kind: operational -->

- [x] 3.1 CHANGE: *(landed in group 1: task 1.3's "the other 10 still do" and task 1.5's green
      `degraded_coverage` binary are unreachable while the checked-in map still holds the range
      the new rule rejects, so the rebinding is the GREEN half of 1.3 rather than a later step.)*
      Rebind the one range `make covers-check` rejects — `src/tasks.rs:193-196`,
      under "a tasks file exists but cannot be read" — to code that runs when that condition
      holds. `make covers-check` exits 0 when this is done.
- [ ] 3.2 CHANGE: Audit the remaining 73 ranges for **aboutness**, which no rule reaches (design.md → Risks). For each, read the row's `condition`, `why`, and `proof`, then confirm the range
      names code that runs when that condition holds. The archived `settings-window` review scoped this at
      "~10 rows in the same vacuous shape" (`notes/change-review.md` → W7); two are verified
      here, so treat ~10 as the expected yield and record the difference rather than stopping at
      two. Both known-wrong rows pass
      the rule: `src/ui/list.rs:300-322` is `fold_glyph` under "No `openspec/` found", and
      `src/ui/view.rs:400-421` is `detail_row_role` under "`openspec` binary not found". Record
      each row's verdict in `notes/audit.md` so the judgement is reviewable as a whole; a row
      whose `proof` watches something other than its `why` claims is reported and left, not
      silently re-aimed.
- [ ] 3.3 CHANGE: Add a **range** floor to `tests/degraded_coverage.rs` beside `MIN_ROWS`, so
      `covers-check` carries it. Today the only in-suite floor counts rows (`MIN_ROWS = 46`
      against 59, thirteen rows of slack) and the range floor lives solely in
      `scripts/coverage-prod.py`, where it fires only below the row count — so 74 ranges could
      fall to 59 unnoticed by `cargo test`.
- [ ] 3.4 VERIFY: Name the outcome, not the tally. `make covers-check` exits 0; `notes/audit.md`
      carries a verdict for all 74 ranges; and **both** aboutness defects the plan names by path
      are repaired: `src/ui/list.rs:300-322` (`fold_glyph`, under "No `openspec/` found while
      walking up") and `src/ui/view.rs:400-421` (`detail_row_role`, under "`openspec` binary not
      found") each now name code that executes when its row's condition holds.
- [ ] 3.5 VERIFY: `git diff --stat tests/degraded-coverage.toml` shows changes beyond 3.1's single
      range — or `notes/audit.md` records explicitly that no further range moved and why. Every
      other check in this group is green at HEAD before the audit runs: `covers-check` exits 0
      once 3.1 lands, the range count is already 74, and a verdict file is self-authored prose.
      An audit that writes 74 lines of "looks fine" must not pass this group.

## 4. Documentation
<!-- kind: operational -->

- [ ] 4.1 CHECK: Confirm the two prose sites still say five, so the edits below are corrections
      rather than additions: `SPEC.md:1387` and `AGENTS.md:278`.
- [ ] 4.2 CHANGE: `SPEC.md` → Gates — "composes five gates" → six, and add the `covers-check`
      row to its table in the `Makefile`'s own order. `tests/ci_workflow.rs` binds this table to
      `check:`'s prerequisites **by name**, so it is red until this lands.
- [ ] 4.3 CHANGE: `AGENTS.md:278` "Five are enforced in CI" → six, and add the row to its gate
      table. `CLAUDE.md` is a symlink to `AGENTS.md`, so this is one file.
- [ ] 4.3b CHANGE: `README.md:123-124` enumerates the gates in prose — "format, lint, the
      hygiene-gate tier, tests, and the 80% coverage floor". Add `covers-check` to it.
- [ ] 4.4 CHANGE: `openspec/config.yaml` — add `make covers-check` to the commands its context
      block lists. `tests/doc_contract.rs`' `unrepresented_check_targets` reads `check:`'s
      prerequisites from the `Makefile` and requires each to be named there, so this goes red
      inside `cargo test` the moment 2.2 lands. This is the change's only edit inside
      `openspec/` outside its own directory.
- [ ] 4.5 CHANGE: `.github/workflows/ci.yml`'s `check` job — the delta for `ci-workflow` retitles
      its requirement and reworders its step list; confirm the workflow matches the ADDED
      requirement's scenario, `make covers-check` between `make gates` and `make test` on both
      runners.
- [ ] 4.6 VERIFY: `cargo test --all-features --test ci_workflow --test doc_contract` — green,
      including the leg requiring every `check:` prerequisite to be named in `SPEC.md` → Gates
      and the leg requiring it in `openspec/config.yaml`'s context block.

## 5. Change Review
<!-- kind: operational -->

- [ ] 5.1 CHECK: Dispatch an independent `outside-in-tdd-reviewer` — not a fork of the applying
      session — against proposal.md, both delta specs, design.md, and the diff. Point it at the
      rebound ranges in `tests/degraded-coverage.toml` specifically: this change's whole subject
      is bindings that pass while proving nothing, and a rebinding that is merely *plausible* is
      the same defect in a new place.
- [ ] 5.2 CHANGE: Fix every CRITICAL. Resolve or consciously accept each WARNING with a one-line
      reason recorded in `notes/change-review.md`.
- [ ] 5.3 VERIFY: Confirm no blocking or unowned finding remains.

## 6. Lint & Verify
<!-- kind: operational -->

- [ ] 6.1 CHECK: Inspect the intended verification commands and the tiers they reach, so a
      command that selects nothing is caught before it is trusted.
- [ ] 6.2 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 6.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.
- [ ] 6.4 VERIFY: `make gates` — every gate green, and the script count still 32.
- [ ] 6.5 VERIFY: `make covers-check` — exits 0. Anti-vacuity is 3.3's range floor, not the
      target's output: the binary prints test names, `check_coverage` returns a **row** count,
      and cargo captures a passing test's stdout, so "names the number of ranges" describes
      nothing that exists.
- [ ] 6.6 VERIFY: `cargo test --all-features` — green, and the count is at or above HEAD's
      **1818** across its ten binaries. 1624 is the `--lib` binary alone and is the wrong floor.
- [ ] 6.7 VERIFY: `cargo llvm-cov --fail-under-lines 80` plus `scripts/coverage-prod.py` — both
      floors hold at or above 80% total and 96% production slice.
- [ ] 6.8 VERIFY: `make check` as the single gate, naming the failing sub-command if it fails.
- [ ] 6.9 VERIFY: `openspec validate coverage-tier-hardening --strict`.
