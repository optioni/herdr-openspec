## 1. Body rows indent to their section's depth

<!-- kind: behavior -->

- [ ] 1.1 CHARACTERIZE: Pin the two baselines that are green at HEAD and must stay green — `the_same_tab_draws_its_bodies_at_column_zero_at_the_narrow_interior` and `a_depth_0_tracked_tasks_tab_is_unmoved_at_every_width`. Assert the row list **byte-identical** to HEAD's rather than "begins at column zero": `ui::markdown`'s own hanging indent already starts some body rows with spaces, so the weaker phrasing is ambiguous where byte-identity is not.
- [ ] 1.2 RED: Write failing tests for `a_spec_tabs_bodies_align_under_their_headers_at_the_wide_interior`, `the_indent_is_all_or_nothing_across_one_render`, `a_shallower_tab_indents_at_a_narrower_width`, `a_depth_1_tracked_tasks_tab_indents_its_items_like_any_other_tab`, and `a_selection_over_an_indented_body_row_copies_the_indent`, plus `artifact-content`'s view test at its rewritten two-sided form. Confirm each fails because body rows sit at column zero — the probe under "Checks run at planning time" reproduces exactly that at HEAD.
- [ ] 1.3 Two fixtures do not exist and must be built, or the tests fail for reasons unrelated to the code: a **max-depth-1** `Detail` for the shallower-tab test (`three_spec_detail` and `preamble_detail` are all depth 0; `seven_section_detail` is depths 0–3, giving floor 70 and failing the 67/66/65 assertions), and a **depth-1 tracked-tasks** `Change`+`Detail` pair. Reuse `seven_section_detail` for the wide-interior test — its file sections at depth 0 and headings at 1, 2, 3 are exactly the glob shape that scenario now names.
- [ ] 1.4 CHECK: Every `#[test]` in `src/ui/detail.rs` must name both `58` and `78` as unsuffixed literals or `DETAILWIDTHS` fails. The shallower-tab test asserts at `67`/`66`/`65` and the sweep runs `0..=120`, so both must additionally assert at `58` and `78`. Verify with `/bin/sh scripts/gates/detailwidths.sh` before GREEN.
- [ ] 1.5 GREEN: In `content_lines`, compute `max_depth` as the greatest `depth` over `detail.sections` where `!text.is_empty()`, indent each section's body by `2 * section.depth` when `width.saturating_sub(2 * max_depth) >= 64`, and wrap that body at `width - indent_cols` before prepending a plain-faced indent segment (design.md → Decisions 1–4). `ui::markdown::lines` and `ui::tasks::items` both take the reduced width; `ui::tasks::bar_lines` and `separator_row` do not.
- [ ] 1.6 CHANGE: Update the **three** existing tests that pin the reversed behaviour. All three were found by planting a minimal implementation at planning time and running `cargo test --lib` (`1445 passed; 3 failed`), so this is the measured set, not a predicted one: `src/ui/view.rs`'s `a_body_row_is_never_indented_by_its_sections_depth` (split to match its two rewritten `artifact-content` scenarios); `src/ui/detail.rs`'s `a_badged_header_row_is_still_addressed_by_its_own_section_index`, whose `r.text() == "Beta text."` becomes `"  Beta text."` at 78 and is unchanged at 58; and `src/ui/detail.rs`'s `the_tracked_tasks_tab_concatenates_rather_than_folding`, a two-path tracked-tasks fixture whose groups sit at depth 1 and whose items gain `"  "` at 78 — the live proof that the tasks tab is not exempt.
- [ ] 1.7 REFACTOR: Extract the floor into one named helper over `(&[ArtifactSection], u16)` so `64` and the `2 *` unit appear once, or record that the section walk reads clearly without it.
- [ ] 1.8 VERIFY: Run `cargo test --lib -- ui::detail ui::view ui::tasks` — 260 tests at HEAD, plus the ones this group adds, all green.

## 2. Change Review

<!-- kind: operational -->

- [ ] 2.1 CHECK: Dispatch an independent reviewer (not a fork of the implementing session) against proposal.md, both delta specs, design.md, and tasks.md with the diff. Concentrate on: a width sweep that selects zero rows and passes vacuously; a body wrapped at `width` and then prefixed, which overflows an interior the region does not clip; the separator row or the progress bar wrongly indented; and either CHARACTERIZE baseline having been edited rather than kept.
- [ ] 2.2 CHANGE: Fix every CRITICAL, resolve or accept each WARNING with a one-line reason, note SUGGESTIONs, and re-run affected tests.
- [ ] 2.3 VERIFY: Confirm no blocking or unowned finding remains.

## 3. Documentation

<!-- kind: operational -->

- [ ] 3.1 CHECK: Confirm both prose sites still state the reversed rule — `grep -n "no depth indent" SPEC.md` matches at line 601, and `grep -n "bodies drawn" openspec/specs/artifact-folds/spec.md` matches in the capability's `## Purpose`. Both are green-to-red checks: they must match now and match nothing after 3.2.
- [ ] 3.2 CHANGE: Rewrite both (audience: anyone implementing against the design contract). `SPEC.md` line 601 — "bodies carry no depth indent, so the narrow interior spends its columns on text" — states the floor instead, keeping the narrow-layout reasoning as the reason the floor exists. `openspec/specs/artifact-folds/spec.md` → `## Purpose` — "with bodies drawn unindented at the full content width" — likewise. The Purpose is edited **in place** because a `## MODIFIED Requirements` delta cannot reach a Purpose and `tests/spec_purposes.rs` only checks it is non-empty, so nothing else would ever catch it. Net change is one clause replaced in each, not added.
- [ ] 3.3 VERIFY: Both greps from 3.1 now return nothing, and no requirement body in `openspec/specs/` was touched — the deltas fold in at archive time, not now.

## 4. Lint & Verify

<!-- kind: operational -->

- [ ] 4.1 CHECK: Inspect the intended verification commands and affected tiers — `ui::detail` and `ui::view` unit tests, and the `DETAILWIDTHS`, `COLWIDTH`, and `NOIO-VIEW` gate scripts.
- [ ] 4.2 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 4.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.
- [ ] 4.4 VERIFY: `make gates` — every script exits 0, `DETAILWIDTHS` reporting at least 70 tests all naming 58 and 78 (63 at HEAD plus the 7 this change adds to `src/ui/detail.rs`).
- [ ] 4.5 VERIFY: `cargo test --all-features` — green.
- [ ] 4.6 VERIFY: `make coverage` — both floors hold with no exclusion added.
- [ ] 4.7 VERIFY: `openspec validate section-body-indent --strict` — passes.

## Ordering

Sequential. `openspec/config.yaml:81` records the standing parallelism veto — one crate, one compile, every gate sweeping the whole tree — so criterion 3 fails for every pair in every change here and the pair walk is skipped per that rule. Group 1 is in any case the only group that edits code, and groups 2–4 each depend on its result.

## Checks run at planning time

**RED probe — group 1.2.** Planted a temporary test in `src/ui/detail.rs` over the existing `seven_section_detail` fixture (sections at depths `0,1,2,3,2,0,0`), all expanded, asserting that the depth-3 scenario's body rows begin with six spaces at width `78`; reverted after running, and `git status --porcelain` confirmed clean.

```
cargo test --lib ui::detail::tests::probe_red -- --nocapture
```

Exit non-zero. **1 test executed, 2 body rows selected** (`PROBE selected 2 scenario body rows` — a sweep selecting none would have failed its own assertion). Failure: `depth-3 body row not indented six columns: "• WHEN a"`, against the header printed three rows above it as `"      ▾ Scenario: A works"`. The behaviour is missing, and the six-column misalignment the proposal describes is reproduced verbatim.

**Two of the planned tests are green at HEAD, which is why they are CHARACTERIZE and not RED (1.1).** At HEAD the section walk ends in `out.extend(body.into_iter().map(body_row))` with no prefix, so every body row already begins at column zero and is already wrapped at the full width. A task instructing "confirm it fails" for those would be unsatisfiable, and an implementer taking it literally would distort the fixture until it did.

**DETAILWIDTHS — groups 1.4, 4.4.** `/bin/sh scripts/gates/detailwidths.sh` exits 0: `DETAILWIDTHS OK: all 63 detail tests name both 58 and 78`. **63 tests scanned** against a floor of `DETAIL_MIN=38`, so the tests this change adds keep it satisfied. Green at HEAD and therefore no evidence on its own; its negative control is `tests/gate-controls.toml` → `script = "detailwidths.sh"`, exercised by `cargo test --test gate_controls` (`5 passed`), which copies the tree, plants a violation, and requires the script to exit non-zero.

**COLWIDTH and NOIO-VIEW — group 4.4.** Both exit 0 at HEAD: `COLWIDTH OK: no char-count measurement in the nine pure view files` and `NOIO-VIEW OK: 10 pure files carry no I/O API; positive control matched`. Both are invariants that already hold, and both carry negative controls in the same `gate_controls` run (`colwidth-sweep-hit`, `colwidth-sweep-take`, `colwidth-sweep-vec-char`, `colwidth-sweep-help`, `noio-view-hit`, `noio-view-help-hit`); `DETAILWIDTHS`' own control is `detailwidths-missing`.

**Documentation checks — group 3.1.** Both match at HEAD, which is what makes 3.3 falsifiable: `grep -n "no depth indent" SPEC.md` → **1 line** (601); `grep -n "bodies drawn" openspec/specs/artifact-folds/spec.md` → **1 line** (22, inside `## Purpose`).

**Numbers used in this plan.**

| Number | Command | Result |
|---|---|---|
| 63 existing `ui::detail` tests | `cargo test --lib ui::detail::tests::` | `63 passed` |
| 156 existing `ui::view` tests | `cargo test --lib ui::view` | `156 passed` |
| 260 across all three filters | `cargo test --lib -- ui::detail ui::view ui::tasks` | `260 passed` |
| `DETAILWIDTHS` floor of 38 | `grep DETAIL_MIN scripts/gates/detailwidths.sh` | `DETAIL_MIN:-38` |
| 16 artifact-folds scenarios carried unchanged | `grep -c '^#### Scenario:' …/specs/artifact-folds/spec.md` | `23`, less the 7 this change adds |
| 13 artifact-content scenarios carried unchanged | `grep -c '^#### Scenario:' …/specs/artifact-content/spec.md` | `14`, less the 1 this change rewrites |
| Deepest spec section is depth 3 | `cat openspec/changes/*/specs/*/spec.md openspec/changes/archive/*/specs/*/spec.md \| grep -oE '^#{1,8} ' \| sort \| uniq -c` | `306 ##`, `652 ###`, `2872 ####`, nothing deeper — so no section exceeds depth 3, and `max_depth` is 3 with floor 70 |
| The deep case is the common case | the rule itself re-run over all 236 files — per-file `min_level`, per-change `base = usize::from(paths.len() > 1)` | **2,776 of 3,830** sections are at `depth` 3 (72.5%); by `depth`, `{0: 7, 1: 316, 2: 731, 3: 2776}` |
| 17 of 44 task files start at depth 1 | `grep -l '^# [^#]' openspec/changes/*/tasks.md openspec/changes/archive/*/tasks.md \| wc -l` vs `ls … \| wc -l` | `17` of `44` — why the tracked-tasks tab is not exempt |

`2,872` is the count of `####` **headings**, which is not the same as `depth` 3: five changes carry exactly one spec file, so their `base` is 0 and their `####` sections land at depth 2. The depth figure above is the rule re-run, not the heading count — the two differ by 96, and it is the depth that the floor reads.

`cargo test` takes one positional `TESTNAME`; a second filter must follow `--`, or the command is rejected and runs nothing. Every multi-filter command above is written in the `--` form for that reason.
