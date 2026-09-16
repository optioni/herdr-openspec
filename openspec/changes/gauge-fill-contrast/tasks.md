<!-- Ordering, with the file each group writes:

       0  (no file — measurement only)
       1  src/ui/tasks.rs, src/ui/view.rs
       2  (no file — review)
       3  SPEC.md, AGENTS.md
       4  (no file — verification)

     No pair is marked `parallel-after`. The pair walk is skipped and the standing veto in
     `openspec/config.yaml` cited instead, exactly as that rule directs: this is one crate,
     one compile, and `make check` sweeps the whole tree, so criterion 3 fails for every pair
     in every change here and re-deriving it pair by pair says nothing about these groups.
     Groups 1 and 3 would fail criterion 3 on that standing fact alone; there is in any case
     no second implementation group for it to run beside.

     Group 1 writes two files rather than one; design.md -> Modules touched and -> Test Strategy
     carry the reason. In short: the view row names the retiring glyphs at six sites and goes red
     the moment the match arms move, so splitting it out would leave `cargo test` red at a group
     boundary. -->

<!-- No group 0 acceptance test — design.md -> Test Strategy. The change binds no key and
     alters no state transition; the view tier is the outermost tier that can fail here, and
     its one row lands in group 1 beside the unit rows. -->

## 0. Measurements

<!-- kind: operational -->

Every figure this plan uses, with the command that produced it, run at HEAD on 2026-09-16
with this change's proposal, specs, and design committed.

| Figure | Command | Result |
|---|---|---|
| `▓`/`▒` sites in `src/` | `grep -n '▓\|▒' src/ui/tasks.rs src/ui/view.rs` | **24** lines — 18 `src/ui/tasks.rs`, 6 `src/ui/view.rs` |
| …split at `#[cfg(test)]` | `python3` scan for the `#[cfg(test)]` line per file | `src/ui/tasks.rs` (marker at `:587`): **3 production** — `:133` doc comment, `:201` and `:202` match arms — and **15** test lines. `src/ui/view.rs` (marker at `:717`): **0 production**, **6** test lines, all inside one test |
| Tests that must be rewritten | the 21 test lines above, resolved to their enclosing `#[test]` | **7** — six in `src/ui/tasks.rs` (`two_groups_of_unequal_size_get_spans_proportional_to_their_item_counts` `:1188`, `an_empty_group_contributes_no_span_and_consumes_no_index` `:1228`, `segmentation_is_skipped_below_the_legibility_floor` `:1263`, `a_single_group_is_never_segmented` `:1308`, `segmentation_is_total_and_partitions_the_run_exactly` `:1328`, `groups_headings_items` `:1473`) and one in `src/ui/view.rs` (`a_real_tasks_tab_renders_a_segmented_gauge_into_the_frame` `:5161`) |
| …plus the two shared helpers | `grep -n 'fn gauge_run\|fn filled_count' src/ui/tasks.rs` | `gauge_run` `:1174` (its class is the literal `"█░▓▒"`) and `filled_count` `:1178` (filled is `█` or `▓`). Every rewritten test reads a run through these two |
| `#[test]` count, `src/ui/tasks.rs` | `grep -c '#\[test\]' src/ui/tasks.rs` | **38**, and `TASKWIDTHS` reports "all 38 tasks tests name both 58 and 78" |
| East Asian Width class of every glyph in play | `python3 -c "import unicodedata as u; print(u.unidata_version); [print(hex(ord(c)), u.east_asian_width(c)) for c in '█░▓▒⢕⠌']"` | UCD **16.0.0**: `█` **A**, `░` **N**, `▓` **A**, `▒` **A**, `⢕` **N**, `⠌` **N** — so the standing "all four are Ambiguous" claim is false at three sites, per design.md -> Decision 8 |
| Where `unicode-width` enters the build | `cargo tree -i unicode-width` | `unicode-width v0.2.2` under three parents — `ratatui-core`, `ratatui-widgets`, `unicode-truncate` — every one of them reached through `ratatui`; the crate declares **six** direct dependencies and this is not one of them, which is why Decision 8 declines to bind the width table in `tests/doc_contract.rs` |
| Whether any test binds the glyph claims today | `grep -n 'gauge\|Ambiguous\|█\|░\|▓\|▒' tests/*.rs` | **0** lines selected — the claim could go wrong unnoticed because nothing executable reads it |
| The legibility floor's measured worst case | a `python3` sweep over `openspec/changes/archive/*/tasks.md` counting **contributing** groups — `^## ` headings holding at least one `- [ ]`/`- [x]` item — and ranking by that, not by heading count | `archive/2026-09-07-degraded-states`, **18** contributing groups and **115** items, so cells `[115/115]` and `100%`: `g = 58 - 9 - 4 - 2 = 43` against a floor of `2 * 18 = 36` — **seven** columns of headroom |
| …and why the previous figure was wrong | `grep -c '^## '` on `archive/2026-09-06-agent-launch` → 22, but five of those headings hold zero items | `segmented_gauge` filters to `total > 0` (`src/ui/tasks.rs:178`), so that change's real `n` is **17**, floor 34, headroom 11. `grep -c '^## '` counts headings, which is not what the floor uses; the delta spec's own paragraph is corrected in this change |
| The three prose sites carrying the false claim | `grep -n '▓' SPEC.md AGENTS.md openspec/specs/tasks-progress-bar/spec.md` | `SPEC.md:406` (paragraph `404-410`), `AGENTS.md:142` (paragraph `141-145`; `CLAUDE.md` is a symlink to it), `openspec/specs/tasks-progress-bar/spec.md:66` (paragraph `65-70`, in the **grammar** requirement) |

- [x] 0.1 CHECK: Re-run every command above and confirm each figure still holds. A figure that
      moved sends you somewhere specific — the site split and the seven test names to 1.2 and
      1.3, the helper line numbers to 1.2, the three prose sites to group 3.
- [x] 0.2 CHECK: Confirm the baseline is green before any edit. Measured at HEAD on 2026-09-16:
      `cargo test --all-features` exits **0** with **1624 passed, 0 failed, 1 ignored** across
      ten test binaries — lib 1445, main 0, `ci_workflow` 21, `cli` 10, `coverage_prod` 19,
      `degraded_coverage` 10 (+1 ignored), `doc_contract` 107, `gate_controls` 5, `manifest` 4,
      `spec_purposes` 3. Redirect to a file rather than piping into `grep`: a pipeline drops the
      trailing binaries' result lines and undercounts.
      **`make gates` needs the change directory tracked first.** It exits **2** on an untouched
      checkout of this plan — `OPENSPEC-UNTOUCHED FAIL: an untracked file exists inside
      openspec/: openspec/changes/gauge-fill-contrast/tasks.md`, since
      `scripts/gates/openspec-untouched.sh` sweeps `git ls-files --others --exclude-standard`.
      Run `git add openspec/changes/gauge-fill-contrast/` before `make gates`, after which it
      exits **0** with **58** OK lines. (The failing run prints only 47 — it stops at
      `OPENSPEC-UNTOUCHED` and the eleven later gates never run, so 47 is a truncation artefact
      and not the figure to compare against.) The same applies to 4.4 and 4.8.
      **Known flake:** `gate_controls_catch_their_plants` fails with "the real working tree
      changed while running the gate controls" if anything touches this checkout during the
      run. Re-run in a quiet tree before attributing it to a change.
- [x] 0.3 CHECK: Re-run the two RED checks. Both were run at HEAD on 2026-09-16 and neither
      describes the post-change tree, so the behaviour is provably absent before group 1.
      `grep -rl '▓' src/ --include='*.rs' | wc -l` → **2** (`src/ui/tasks.rs`,
      `src/ui/view.rs`); it must read **0** after group 1.
      `grep -rl '⢕\|⠌' src/ --include='*.rs' | wc -l` → **0**; it must read **2** after group 1,
      naming `src/ui/tasks.rs` **and** `src/ui/view.rs`. Two, not one: the view test asserts the
      per-span pairs `█`/`⢕` and `▒`/`⠌` by name, so a correct implementation puts braille in
      both files, and a plan expecting one would be satisfied only by gutting that test.
      The second selects nothing, so its positive control is recorded beside it:
      `grep -rlE '⢕|⠌|█' src/ --include='*.rs' | wc -l` → **3**, proving the pattern and the
      path reach the files that will hold braille rather than the check being aimed at nothing.
- [x] 0.4 CHECK: Confirm the ▓-retirement check is tree-wide rather than pinned to the two
      files that hold the glyph today. Negative control run at HEAD: appending
      `// planted: ▓` to `src/ui/detail.rs` made the check name **3** files including
      `src/ui/detail.rs`; `git checkout -- src/ui/detail.rs` returned it to **2** and
      `git status --porcelain` printed nothing. Re-run it if group 1 changes the check's form.
- [x] 0.5 CHECK: Confirm both greps above are absence-of-string checks, not
      absence-of-behaviour ones. The behavioural RED is 1.1 and 1.2; these are recorded as a
      starting condition, because a grep cannot fail for the right reason.

## 1. The glyph table: blocks filled, braille empty

<!-- kind: behavior -->

Writes `src/ui/tasks.rs` and `src/ui/view.rs`, for the reason the ordering note gives.
`TASKWIDTHS` carries no exemption list, so every test added here names both `58` and `78`.

Five of the MODIFIED grammar requirement's six scenarios are **carried unchanged** and get no
task: *The full grammar at both mandated interior widths*, *The bar reaches the buffer at both
mandated frame widths*, *The percentage truncates rather than rounds*, *The bar measures at most
its width at every width*, and *A saturating `Progress` renders a full gauge and a full
percentage*. Each passes an empty `groups` slice, so no glyph in it moves; their tests
(`src/ui/tasks.rs:621`, `:2046`, `:2085`, `:2229`, `src/ui/view.rs:7627`) are regression rows
re-run by 1.9 and 4.5, not rewrites.

- [x] 1.1 RED: Write three failing `ui::tasks` tests named for their scenarios —
      `the_fill_boundary_is_the_only_change_of_character_family`,
      `a_group_straddling_the_fill_boundary_keeps_one_identity`, and
      `every_glyph_the_bar_can_draw_measures_one_column`. The first two belong to the
      segmentation requirement, the third to the grammar requirement.
      The third is RED **only** because of its glyph-presence clause — that the segmented run
      holds at least one `█`, `▒`, `⢕` and `⠌`, and the empty-slice call at least one `░`. Its
      width assertions alone pass at HEAD, since `layout::columns` measures a `char` literal the
      crate never draws. Write that clause first and confirm it fails, or the test is a
      decoration in a group whose marker promises a failing start.
- [x] 1.2 RED: Rewrite the two shared helpers to the new glyph set — `gauge_run`'s class
      becomes `"█░▒⢕⠌"` and `filled_count` counts `█` and `▒` — then rewrite the six
      `ui::tasks` tests and the one `ui::view` test that 0.1 names. Six of the seven are
      regression rows that only change which glyph they count.
- [x] 1.3 RED: The seventh is not. `segmentation_is_total_and_partitions_the_run_exactly`
      gains genuinely new coverage: its glyph-count sum moves from four glyphs to five, and it
      gains the non-interleaving assertion — no position drawn as both families, every braille
      position at or after every block one — over its full `0..=130` × five-fixture sweep. The
      new fixture-specific test in 1.1 asserts that property for one slice at two widths; this
      is the row that asserts it everywhere.
- [x] 1.4 RED: Confirm the failures are the missing substitution rather than damaged fixtures:
      run `cargo test --all-features` and check that every failure names a test from 1.1 or
      1.2 and none names a test outside them. Record the failing count. **8 failures**, all
      inside the set: the three of 1.1, four of 1.2's six (`a_single_group_is_never_segmented`
      and `groups_headings_items` pass at HEAD, both being regression rows whose assertions
      the old glyph set still satisfies), and 1.3's. No test outside the set failed.
- [x] 1.5 GREEN: Rewrite `segmented_gauge`'s match arms to the four-case table — `('█', true)
      => '▒'`, `('░', false) => '⢕'`, `('░', true) => '⠌'`, everything else unchanged. The
      match still reads the incoming glyph to decide, which is what keeps the fill count fixed
      (design.md -> Decision 6).
- [x] 1.6 GREEN: Rewrite `segmented_gauge`'s doc comment (`:133` onward) and `progress_bar`'s
      reference to "alternating shade" so the prose names two character families rather than
      one lightness scale. Leave `gauge_of`'s doc comment alone — it does not move.
      Keep the word **stretch**: `TASKSEAM` greps this file's prose for `Span`, case-sensitively,
      so a sentence lifted from the spec beginning "Spans are proportional…" turns `make gates`
      red. The existing comment records this at `:159-164`; do not delete that aside.
- [x] 1.7 CHECK: Confirm `gauge_of` is untouched in signature and output, so `detail-header`'s
      twelve-column gauge cannot have moved: `git diff src/ui/tasks.rs` shows no hunk inside
      `gauge_of`, and `cargo test --all-features --lib ui::detail` is green.
- [x] 1.8 CHECK: Run the two 0.3 greps again and confirm they have flipped —
      `grep -rl '▓' src/ --include='*.rs' | wc -l` → 0, and
      `grep -rl '⢕\|⠌' src/ --include='*.rs' | wc -l` → **2**, naming exactly `src/ui/tasks.rs`
      and `src/ui/view.rs` and no other file. The count is the view test's own assertions; the
      "no other file" half is the claim worth checking, since braille is the boundary vocabulary
      and `src/ui/detail.rs` draws an unsegmented gauge that must not gain any.
- [x] 1.9 CHECK: Confirm `░` survived, with a check that can fail:
      `grep -rl '░' src/ --include='*.rs' | wc -l` → **3**, naming `src/ui/tasks.rs`,
      `src/ui/detail.rs` and `src/ui/view.rs`. Use `-rl`, never `-rc`: `grep -rc` prints a
      `path:0` line for every file it scans, so it names all three whatever the answer is.
- [x] 1.10 REFACTOR: Fold the four match arms into one table if a second copy of the
      even/odd decision emerged, or record that none was needed. **None was needed** — the
      even/odd decision is computed once (`let odd = group % 2 == 1;`) and read by one match;
      no second copy emerged, and the four arms are already the table.
- [x] 1.11 VERIFY: `cargo test --all-features` — green — and `/bin/sh scripts/gates/taskwidths.sh`,
      `taskseam.sh`, `colwidth.sh` and `noio-view.sh` each exit 0. The whole suite, not a module
      filter: this group edits `src/ui/view.rs`, which a `ui::tasks` filter would not run.

## 2. Change Review

<!-- kind: operational -->

- [x] 2.1 CHECK: Dispatch an independent reviewer — not a fork of this session — against
      proposal.md, both MODIFIED requirements, design.md, and the diff. Point it first at the
      two concentration points this change can actually fail on: a rewritten assertion that
      counts a glyph the fixture never reaches, and a scenario that passes against a
      production caller handing `progress_bar` an empty slice.
- [x] 2.2 CHANGE: Fix every CRITICAL, resolve or accept each WARNING with a one-line reason,
      note SUGGESTIONs, and re-run the affected tests.
      **No CRITICAL and no WARNING was found.** All three SUGGESTIONs were taken:
      (a) the "no position is drawn with both families" clause in
      `segmentation_is_total_and_partitions_the_run_exactly` was a tautology — `is_block` and
      `is_braille` are disjoint sets of `char`s — so it was dropped, keeping the
      non-interleaving half that does fail on a scatter, and the scenario's own bullet was
      reworded to say so rather than leaving the code and the spec disagreeing;
      (b) the whole-line identity `two_groups_of_unequal_size…` lost in the reglyphing was
      restored as the three-way inverse map `▒`→`█`, `⢕`/`⠌`→`░`, which pins that
      segmentation moved neither cell nor the gauge's length, and the scenario gained an AND
      clause for it; (c) `a_group_straddling_the_fill_boundary_keeps_one_identity` sliced
      before its straddle guard ran, so a regression would have surfaced as a slice panic
      instead of the message written for it — the guard moved above the slices.
      `cargo test --lib ui::tasks` — 41 passed.
- [ ] 2.3 VERIFY: Confirm no blocking or unowned finding remains, and that any artifact a
      finding changed was updated rather than only the code.

## 3. Documentation

<!-- kind: operational -->

Both tasks rewrite an existing paragraph in place and neither adds a rule; the net change to
each document is within a line or two. The third site carrying the same claim,
`openspec/specs/tasks-progress-bar/spec.md:65-70`, is **not** edited here — it is owned by
this change's own MODIFIED grammar requirement and lands when `openspec archive` runs.

- [ ] 3.1 CHANGE: Rewrite in `SPEC.md`: the paragraph at `404-410` beginning "`tasks-emphasis` widens
      the same exposure once more" (audience: future implementers) — it names the retiring `▓`
      and states that the gauge's glyphs are uniformly Ambiguous, which 0.1 measures as false.
      Say instead that the filled half is Ambiguous and the empty half Neutral, so a CJK-locale
      terminal mis-proportions the bar rather than doubling it. The markdown renderer's
      "seven glyphs, six Ambiguous" count above it does **not** move.
- [ ] 3.2 CHANGE: Rewrite in `AGENTS.md`: the paragraph at `141-145` beginning "The progress gauge's
      own glyphs join that exposure" (audience: every agent session) — it repeats `SPEC.md`'s
      claim and must say the same thing after 3.1. Rewrite in place; do not append beside it.
      `CLAUDE.md` is a symlink to this file, so there is one edit, not two.
      **Watch the tail of `:145`:** that line ends the gauge paragraph and then starts an
      unrelated claim on the same physical line — "`ui` refuses to start with exit status 3…" —
      which `tests/doc_contract.rs` binds. Rewrite the sentence, keep the tail.
- [ ] 3.3 VERIFY: Confirm the prose edits actually landed, with checks that can fail:
      `grep -n '▓' SPEC.md AGENTS.md` selects **0** lines (both sites named the retiring glyph
      before this group), and `grep -n 'all four' SPEC.md AGENTS.md` selects **0** lines. Both
      select non-zero at HEAD, so each is red before the group and green after — which is the
      only executable evidence this group has.
- [ ] 3.4 VERIFY: `cargo test --all-features --test doc_contract` — green — confirming no
      documented claim this change touched drifted from the file that determines it. 0.1 records
      that this suite binds none of the glyph claims, so a green run here is evidence about the
      claims it does bind and not about 3.1 or 3.2. It takes about three minutes on its own.

## 4. Lint & Verify

<!-- kind: operational -->

- [ ] 4.1 CHECK: Inspect the intended verification commands and affected tiers — `make check`
      is the single gate and runs format, lint, gates, test, and coverage.
- [ ] 4.2 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 4.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors. This
      is the crate's type checker for this purpose; `cargo check` is subsumed by it.
- [ ] 4.4 VERIFY: `make gates` — exits 0, every script included. Requires the change directory
      to be tracked (0.2): `OPENSPEC-UNTOUCHED` fails on any untracked file under `openspec/`.
- [ ] 4.5 VERIFY: `cargo test --all-features` — green, at or above the 1624 passing rows 0.2
      recorded plus the three tests 1.1 adds.
- [ ] 4.6 VERIFY: `cargo llvm-cov --fail-under-lines 80` — passes, and the production-slice
      floor from `scripts/coverage-prod.py` passes with it.
- [ ] 4.7 VERIFY: `openspec validate gauge-fill-contrast --strict` — valid.
- [ ] 4.8 VERIFY: `make check` — exits 0 as a whole. If it fails, name the failing
      sub-command here rather than the composite.
