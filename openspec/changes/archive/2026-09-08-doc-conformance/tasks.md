Every check below was run at `d1942bb` and its output recorded beside it. HEAD has since moved
to `f947ab2`, whose diff touches only four sibling changes' own proposal artifacts and no file
any check here reads (`git diff --name-only d1942bb..f947ab2`), so every result below still
holds. All six
leg checks are **RED at HEAD** — the documents are wrong today, so the corrections are the
GREEN half and no manufactured failure is needed. The sub-legs that are *green* at HEAD (the
directions where the documents already agree) get planted-defect negative controls in group 9.

**Parallelism was checked rather than assumed, and one group qualifies.** Groups 1–7 all write
`tests/doc_contract.rs`, one shared file, so they are strictly sequential. **Group 8 is marked
`parallel-after: 0`** — it depends on no earlier group, writes only `HANDOFF.md`, which no
other group touches, and none of its own tasks runs `cargo test` or `make check`, so a
half-written leg elsewhere cannot make its verification fail: all three criteria hold, and `0`
means it has no prerequisite group. Group 9 is **not** parallel: its plant-and-revert cycle
edits `src/lib.rs`, `SPEC.md`, `Cargo.toml`, `AGENTS.md`, and the `Makefile` — every file
groups 1–7 correct — and it needs their tests to exist.

There is no `## 0. Acceptance Test` group: this change renders nothing and wires no
collaborators together, so there is no end-to-end path to drive (design.md → Test Strategy).

## 1. Module map is bound to `src/lib.rs`
<!-- kind: behavior -->

Check, run at HEAD:

```sh
rows=$(awk '/^\*\*Module map:\*\*/{f=1;next} f&&/^\| /{print} f&&!/^\|/&&!/^$/{exit}' SPEC.md \
       | grep -v '^|---' | grep -v '^| Module ' | wc -l | tr -d ' ')
mods=$(grep -c '^pub mod ' src/lib.rs)
echo "map=$rows lib=$mods"; [ "$rows" = "$mods" ]
```

→ `map=12 lib=13`, **exit 1 (RED)**. The unmapped module is `open`.

- [x] 1.1 CHECK: Capture the base SHA this change starts from into `openspec/changes/doc-conformance/notes/baseline.md` — `git rev-parse HEAD` — so tasks 10.3 and 12.9 have a defined `<base>`.
- [x] 1.2 RED: In a new `tests/doc_contract.rs`, write `module_map_matches_lib_rs` plus the pure-parser unit tests `map_missing_row`, `map_orphan_row`, `absent_heading`, and `missing_document` — the parser is `module_map_names(&str) -> Result<BTreeSet<String>, String>`, the module set comes from `pub_mod_names(&str)` over `src/lib.rs`, and `read_doc(path) -> Result<String, String>` returns `Err` naming the path when the file does not exist (`missing_document` drives it with a path that does not exist). Confirm `module_map_matches_lib_rs` fails naming `open` as declared but unmapped.
- [x] 1.3 GREEN: Add the `open` row to `SPEC.md`'s Module map, between `refresh` and `ui`, describing it as the `open` and `open-tab` subcommands that open or focus the dashboard pane through `herdr plugin pane`, and the crate's third `HerdrCli` consumer. Re-run 1.2's tests — green.
- [x] 1.4 REFACTOR: Lift `read_doc` and `section(&str, heading) -> Result<&str, String>` into the shared helpers the later groups reuse, keeping the tests green.
- [x] 1.5 Run `cargo test --test doc_contract` — green, no regressions. Commit.

## 2. Every module is named in the tested-modules section
<!-- kind: behavior -->

Check, run at HEAD:

```sh
awk '/^### Unit-tested modules/{f=1;next} f&&/^### /{exit} f' SPEC.md > /tmp/utm.txt
for m in $(sed -n 's/^pub mod \([a-z_]*\);/\1/p' src/lib.rs); do
  grep -q "$m::" /tmp/utm.txt || echo "MISSING: $m"
done
```

→ `MISSING: config`, `MISSING: state`, `MISSING: launch`, `MISSING: open` — **four**, not the
two the audit named (design.md → Findings). `grep -c "launch::" SPEC.md` returns `0`.

- [x] 2.1 RED: Write `tested_modules_names_every_module` over the real files, plus `tested_modules_missing` and `tested_modules_scoped_to_section` over in-test `&str` inputs (the second feeds a source naming `open::` before the heading and not inside the section). Confirm the real-file test fails listing all four modules.
- [x] 2.2 GREEN: Add a bullet to `SPEC.md` → § Unit-tested modules for `launch`, `open`, `config`, and `state`, each naming what is tested and against which collaborator, in the voice of the bullets already there. `launch` names the pane-split/agent-start/prompt sequence and its `Outcome`; `open` names `open::context` and the subcommand's pane resolution; `config` and `state` name their injected-environment lookups.
- [x] 2.3 REFACTOR: State that no refactor was needed, or lift the section-slicing shared with group 1 if a duplicate appeared.
- [x] 2.4 Run `cargo test --test doc_contract` — green, no regressions. Commit.

## 3. The worker-thread count is bound to the production thread sites
<!-- kind: behavior -->

Check, run at HEAD:

```sh
grep -n "worker threads" SPEC.md
for f in src/*.rs; do
  t=$(grep -n '^#\[cfg(test)\]' "$f" | head -1 | cut -d: -f1)
  head -n "${t:-999999}" "$f" | grep -q 'thread::spawn' && echo "$f"
done
```

→ `SPEC.md:821: crate's two worker threads` — **unemphasised**, no `**` around `two`; the
production slices naming `thread::spawn` are `src/agents.rs`, `src/launch.rs`,
`src/refresh.rs` — **three**. **RED**, failing on the count. `src/cli.rs`'s four occurrences
are all below its `#[cfg(test)]` at line 317 and are correctly excluded.

- [x] 3.1 RED: Write `worker_threads_match_sources` over the real files, plus `worker_count_stale`, `worker_count_grows`, and `worker_claim_absent` over synthetic sources. The claim parser matches `crate's <word> worker threads` for `one`–`six` with the word's `**` emphasis **optional** (design.md → Decision 3); the production slice is the text before the first `#[cfg(test)]` line, matching `NOBLOCK` leg 3. Confirm the real-file test fails reporting documented 2 against computed 3 — not "claim not located", which would mean the emphasis was made mandatory.
- [x] 3.2 GREEN: Correct `SPEC.md:820-821` to `one of the crate's **three** worker threads`, adding the emphasis as well as the number so the phrase reads in the house style. Re-run — green.
- [x] 3.3 REFACTOR: State that no refactor was needed, or lift the number-word mapping if a second leg needs it.
- [x] 3.4 Run `cargo test --test doc_contract` — green, no regressions. Commit.

## 4. The documented Rust version is bound to `Cargo.toml`
<!-- kind: behavior -->

Check, run at HEAD:

```sh
v=$(sed -n 's/^rust-version = "\(.*\)"/\1/p' Cargo.toml)
echo "msrv=$v"; grep -c -- "$v" AGENTS.md README.md
```

→ `msrv=1.88`, `AGENTS.md:0`, `README.md:0`. **RED** — the crate's actual minimum appears in
no prose anywhere.

- [x] 4.1 RED: Write `msrv_is_documented` over the real files (parsing `rust-version` from `Cargo.toml` with the `toml` crate, never `cargo metadata`) and `msrv_bump_is_caught` over synthetic doc text. The search is scoped to `AGENTS.md` → Environment and `README.md` → Development and requires a non-version character on each side of the match, so `11.887` does not satisfy it. Confirm the real-file test fails naming both documents.
- [x] 4.2 GREEN: Rewrite `AGENTS.md` → Environment's Rust bullet to state the supported floor (`Cargo.toml`'s `rust-version = "1.88"`) and, separately, the version the reference machine runs, so `1.91` stops reading as a requirement. Add the floor to `README.md` → Development.
- [x] 4.3 REFACTOR: State that no refactor was needed.
- [x] 4.4 Run `cargo test --test doc_contract` — green, no regressions. Commit.

## 5. Every program the gate path invokes is documented
<!-- kind: behavior -->

Check, run at HEAD:

```sh
grep -c 'python3' README.md AGENTS.md
grep -l 'python3' scripts/gates/* | wc -l
```

→ `README.md:0`, `AGENTS.md:1`; **10** files under `scripts/gates/` name `python3`
(`deps.sh detailwidths.sh gate-mech1.py listwidths.sh mdwidths.sh nodefault-ui.sh nosleep.sh
taskseam.sh taskwidths.sh widths.sh`). **RED** on `README.md`.

- [x] 5.1 RED: Write `gate_programs_are_documented` and `gate_script_interpreters_are_documented` over the real files, plus `guard_block_yields_no_program`, `quoted_assignment_is_one_token`, `new_gate_program_is_caught`, and `empty_program_set_fails` over synthetic `Makefile` inputs. The extractor implements design.md → Decision 7's six steps exactly; the first two synthetic inputs carry the real shapes from the `lint:`/`coverage:` `@if … fi` guards and from `gates:` line 35's quoted `ENTRY=` value. Confirm the real-file tests fail naming `python3` and `README.md`, and that neither `if`, `fi`, nor `fn` is ever reported as a program.
- [x] 5.2 GREEN: Add `python3` to `README.md` → Development as a prerequisite and name the hygiene-gate tier in its description of `make check`. In `SPEC.md`, add `python3` as a **new sentence after** the "Two one-time setup steps" paragraph (`SPEC.md:911-913`), leaving that paragraph's own two-step count intact and editing nothing else in § Gates (design.md → Boundaries).
- [x] 5.3 CHECK: Confirm this group edited no line of § Gates' table or gate count: `git diff -U0 SPEC.md | grep -c '^[+-].*| Format \|^[+-].*all four in order\|^[+-].*| Coverage '` returns `0`.
- [x] 5.4 REFACTOR: State that no refactor was needed, or lift the word-boundary document matcher shared with group 4.
- [x] 5.5 Run `cargo test --test doc_contract` — green, no regressions. Commit.

## 6. `SPEC.md`'s manifest transcription is bound to the manifest
<!-- kind: behavior -->

Check, run at HEAD:

```sh
grep -o '^\[\[[a-z]*\]\]' herdr-plugin.toml | tr '\n' ' '; echo '<- manifest'
awk '/^```toml$/{f=1;next} f&&/^```$/{exit} f' SPEC.md | grep -o '^\[\[[a-z]*\]\]' | tr '\n' ' '
```

→ manifest `[[build]] [[panes]] [[panes]] [[actions]] [[actions]]`; SPEC
`[[build]] [[actions]] [[actions]] [[panes]] [[panes]]`. Order **RED**; parsed values already
agree, so value equality is green at HEAD and takes a planted defect in group 9.

- [x] 6.1 RED: Write `spec_manifest_block_matches` (parse both with `toml`, compare values) and `spec_manifest_block_order_matches` as **separate** named tests, so a failure says which half fired, plus `manifest_block_order` and `manifest_block_absent` over synthetic sources. Confirm the order test fails naming the two orderings and the value test passes.
- [x] 6.2 GREEN: Reorder `SPEC.md`'s fenced manifest block to `[[build]]`, both `[[panes]]`, then both `[[actions]]`, matching the file. **Do not edit `herdr-plugin.toml`** — the correction direction is file → document.
- [x] 6.3 CHECK: Contract gate — `git diff --name-only | grep -q herdr-plugin.toml` returns non-zero, and `cargo test --test manifest` stays green, confirming no consumer-facing manifest value moved.
- [x] 6.4 REFACTOR: State that no refactor was needed.
- [x] 6.5 Run `cargo test --test doc_contract` — green, no regressions. Commit.

## 7. The injected project context is bound to the repository
<!-- kind: behavior -->

Check, run at HEAD:

```sh
grep -c 'fixture repositories' openspec/config.yaml
find tests/fixtures -type d -name openspec | wc -l
grep -c 'make gates' openspec/config.yaml
```

→ `1` occurrence of the phrase, `0` fixture repositories on disk, `0` mentions of `make gates`.
**RED on both legs.**

- [x] 7.1 RED: Write `context_fixture_claim` and `context_names_every_gate_tier` over the real files, plus `fixture_repo_makes_claim_true` over a synthetic listing. The context block is sliced as the text between `context: |` and the next top-level key, failing loudly if that boundary is not found (design.md → Decision 6); `check:`'s prerequisites are parsed from the `Makefile`, and a target whose recipe has more than one command line is satisfied only by `make <target>`. Confirm both real-file tests fail.
- [x] 7.2 GREEN: In `openspec/config.yaml`'s `context`, replace "fixture repositories under `tests/fixtures/`" with the three mechanisms `SPEC.md` § Fixtures actually names, and add `make gates` to the command list so all five `check:` prerequisites are represented.
- [x] 7.3 CHECK: Contract gate — this block is injected into every OpenSpec agent's prompt. Confirm `rules:` and `operations:` are byte-unchanged: `git diff -U0 openspec/config.yaml | grep -c '^[+-]' ` counts only lines inside the `context` block.
- [x] 7.4 VERIFY: `/bin/sh scripts/gates/openspec-untouched.sh` — exit 0, confirming the edit left no untracked file under `openspec/`.
- [x] 7.5 REFACTOR: State that no refactor was needed.
- [x] 7.6 Run `cargo test --test doc_contract` — green, no regressions. Commit.

## 8. `HANDOFF.md` becomes a closed historical record
<!-- kind: operational -->
<!-- parallel-after: 0 -->

No test: "does this file read as a live to-do list" has no second site, and this change does
not invent a check for it (design.md → Decision 5, which also records why the file is
corrected rather than deleted).

- [x] 8.1 CHECK: Confirm all three items in § "Known-deferred doc fixes" have landed — `grep -c 'Cargo.toml' AGENTS.md` is non-zero and no "no Cargo.toml" claim remains; `grep -c '^\[\[actions\]\]' herdr-plugin.toml` is `2`; `grep -c 'not here' openspec/IMPLEMENTATION-ORDER.md` is non-zero. Record each result.
- [x] 8.2 CHECK: Confirm the Environment section's claim is false — `cargo clippy --version` and `cargo llvm-cov --version` both exit 0.
- [x] 8.3 CHANGE: Delete § "Known-deferred doc fixes" and fold § "Deferred: sync README's keymap at the end of Phase 4" into the historical record, since Phase 4 closed and the keymap was synced.
- [x] 8.4 CHANGE: Correct § Environment so the clippy and `cargo-llvm-cov` line reads as fresh-machine setup rather than as current state.
- [x] 8.5 CHANGE: Add one line under the title stating the file is a closed record of a completed project, that open work lives in OpenSpec changes and is reported by `openspec list`, and that a section describing something as deferred is describing the past.
- [x] 8.6 VERIFY: `grep -n '^## ' HANDOFF.md` lists no heading that promises outstanding work, and the transferable findings (fork-is-not-a-sandbox, the per-subject `SCAN_MIN` floors, the outer-test finding, the `%G?` signing trap, the `gh` push procedure) are all still present. Commit.

## 9. Negative controls for the legs that were green at HEAD
<!-- kind: operational -->

Groups 1–7 each began RED, which proves those legs can fail **in the direction they were red
in**. The sub-legs and directions that were green at HEAD prove nothing without a planted
defect, and each control below names a direction no group already exercised against real
files.

- [x] 9.1 CHECK: Plant each defect in the real files in turn, observe the named test go red, revert, observe green — (a) the **orphan direction**: add a `| \`zzz\` | … |` row to `SPEC.md`'s Module map with no `pub mod zzz;` in `src/lib.rs` → `module_map_matches_lib_rs` red reporting the row as mapped but undeclared; (b) change one `command` value inside `SPEC.md`'s manifest block → `spec_manifest_block_matches` red on **value** equality while `spec_manifest_block_order_matches` stays green; (c) change `Cargo.toml`'s `rust-version` to `1.92` → `msrv_is_documented` red; (d) delete the `python3` line from the `Makefile`'s `gates:` recipe → `gate_script_interpreters_are_documented` stays red via the `scripts/gates/` sub-leg.
- [x] 9.2 CHECK: Plant the **`AGENTS.md` direction** of leg 5, which no group drove — remove `python3` from `AGENTS.md` → Environment → `gate_programs_are_documented` red naming `AGENTS.md`, not only `README.md`. Revert.
- [x] 9.3 CHANGE: Record each planted defect, the exact failing assertion line, and the revert in `openspec/changes/doc-conformance/notes/planted-defects.md`, following `degraded-states`' file of the same name.
- [x] 9.4 VERIFY: `git status --porcelain` is clean of every plant, `git diff` against the baseline SHA from task 1.1 shows only intended edits, and `cargo test --test doc_contract` is green. Commit.

## 10. Change Review
<!-- kind: operational -->

- [x] 10.1 CHECK: Dispatch an independent reviewer — not a fork of the implementing session — against proposal.md, every spec scenario, design.md, and tasks.md, with the diff. Point it first at concentration point 1 (a leg that cannot fail) and at whether any leg asserts on prose with no second site.
- [x] 10.2 CHANGE: Fix every CRITICAL, resolve or accept each WARNING with a one-line reason, note SUGGESTIONs, re-run affected tests.
- [x] 10.3 VERIFY: Confirm no blocking or unowned finding remains, and that no sibling-owned passage (design.md → Boundaries) was edited: `git diff --stat <base>..HEAD`, with `<base>` from task 1.1, names only `SPEC.md`, `README.md`, `AGENTS.md`, `HANDOFF.md`, `openspec/config.yaml`, `openspec/IMPLEMENTATION-ORDER.md`, `tests/doc_contract.rs`, and this change's own directory. Commit any fixes.

## 11. Documentation
<!-- kind: operational -->

- [x] 11.1 Rewrite in `AGENTS.md`: § Quality gates (audience: every future agent in this repo) — the paragraph describing `tests/manifest.rs` becomes a description of the **contract tier** and its three members (`tests/manifest.rs`, `tests/degraded_coverage.rs`, `tests/doc_contract.rs`), naming the rule they share: a documented claim with a computable second site is bound to that site inside `cargo test`. Durable because a future change adding a module, a worker thread, or a gate program will otherwise not know why its `make check` went red. Net effect is a rewrite, not an addition — the existing `tests/manifest.rs` sentence is absorbed.
- [x] 11.2 Add in `SPEC.md`: § Testing and quality gates, a `### Doc-conformance checks` subsection after § Fixtures (audience: anyone editing this repository's documents) — the six bound claims, one line each, and the standing rule that a claim with no second site is argued in review rather than checked. Under 10 lines; it replaces nothing because the tier is new.
- [x] 11.3 Add in `openspec/IMPLEMENTATION-ORDER.md`: a Phase 6 row for `doc-conformance` (audience: anyone reading the roadmap) naming it as unplanned post-roadmap work and what it added, following the row `degraded-states` carries. Durable because the roadmap is how a reader reconstructs what shipped and why; without a row this change is invisible there. One row, replaces nothing.
- [x] 11.4 VERIFY: `cargo test --test doc_contract` still green after all three edits — 11.2 adds `open::`, `launch::`, and `config::` mentions to `SPEC.md` and must not be relied on to satisfy group 2's section-scoped leg. Commit.

## 12. Lint & Verify
<!-- kind: operational -->

- [x] 12.1 CHECK: Inspect the intended verification commands and affected tiers — the contract tier is new, `src/` is untouched, and the coverage floor is unaffected because no `src/` line is added.
- [x] 12.2 VERIFY: `cargo fmt --all -- --check` — clean.
- [x] 12.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings.
- [x] 12.4 VERIFY: `make gates` — every hygiene gate exit 0.
- [x] 12.5 VERIFY: `cargo test --all-features` — green, including `doc_contract`, `manifest`, `degraded_coverage`, `spec_purposes`, and `ci_workflow`.
- [x] 12.6 VERIFY: `cargo llvm-cov --fail-under-lines 80` — at or above the floor. If short, add tests; never lower the floor.
- [x] 12.7 VERIFY: `make check` — exit 0 as a single gate; name the failing sub-command if it fails.
- [x] 12.8 VERIFY: `openspec validate doc-conformance --strict` — passes.
- [x] 12.9 VERIFY: `git diff --name-only <base>..HEAD -- src/`, with `<base>` from task 1.1's `notes/baseline.md`, lists nothing — confirming the spec scenario "The dashboard is unchanged". Commit.
