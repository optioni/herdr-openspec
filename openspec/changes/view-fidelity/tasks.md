<!-- No outer-loop acceptance group. design.md → Test Strategy states why: every behaviour
     here is observable in a TestBackend buffer or a pure return value, and neither seam that
     would justify an outer loop (the openspec binary, the Herdr socket) is on any path this
     change touches. -->

<!-- PARALLELISM: none of groups 1-8 is marked parallel, and the dependency shape is
     1 -> {2,3,4,5,6,7} -> 8, not a flat chain. Groups 2-7 each consume group 1's
     `columns`/`truncate_columns` and nothing else of each other's; group 8's gate cannot pass
     until 2-7 have landed. So 2 through 7 are independent by dependency and touch seven
     different files. What fails is the third condition: `cargo build` and `cargo test --lib`
     compile the whole crate, so a half-written `src/ui/markdown.rs` reddens every other
     group's run and a failure is not attributable to its own group. Sequential by toolchain,
     not by design — recorded so a future worktree-isolated run knows the real edge. -->

<!-- NUMBERS IN THIS PLAN, and the commands that produced them.
     29 char-count measurement sites in production code, by file:
       for f in src/ui/app.rs src/ui/detail.rs src/ui/list.rs src/ui/markdown.rs \
                src/ui/tasks.rs src/ui/view.rs src/ui/driver.rs; do
         n=$(awk '/^#\[cfg\(test\)\]/{exit} {print}' "$f" \
             | grep -cE '\.chars\(\)\.count\(\)|\.chars\(\)\.take\(|Vec<char>'); echo "$f $n"; done
     -> app 0, detail 4, list 10, markdown 5, tasks 5, view 5, driver 0.

     Width assertions inside each TEST module — the number each REFACTOR task restates:
       for f in list markdown tasks detail view app; do
         printf "%s %s\n" "$f" "$(awk 'f{print} /^#\[cfg\(test\)\]/{f=1}' src/ui/$f.rs \
             | grep -cE '\.chars\(\)\.count\(\)')"; done
     -> list 8, markdown 11, tasks 11, detail 13, view 18, app 0.

     `#[test]` counts (`grep -cE '^[[:space:]]*#\[test\][[:space:]]*$' src/ui/<f>.rs`), which
     are the width gates' floors and NOT the assertion counts above:
       layout 15, list 32, markdown 25, tasks 16, detail 32, view 98, app 72, driver 35.
     Current gate defaults (`grep -h 'MIN:-' scripts/gates/*widths.sh`): LIST_MIN 32,
     MD_MIN 25, TASK_MIN 16, DETAIL_MIN 32, WIDTHS_MIN 98 — each equal to its true count today,
     which is why 8.6 re-measures rather than guessing.

     `to_ascii_lowercase` call sites (`grep -rn to_ascii_lowercase src/`): src/ui/app.rs:575
     and :576 (this change's), src/state.rs:84 (deliberately untouched, design.md → 6). -->

<!-- THE GATE, WRITTEN OUT AND RUN AT PLANNING TIME. The script does not exist at HEAD — task
     8.2 creates it — so the body below was written to a scratch path and run from the repo
     root. It exits **1** with
       COLWIDTH FAIL: positive control - src/ui/layout.rs names no cell_width/styled_graphemes
     RED, and red on the control before it reaches the sweep. With the control satisfied
     against a scratch copy of src/, the sweep reports exactly the 5 files and 29 lines in the
     breakdown above. The sweep self-test below was added after review found the original
     control tested a different pattern from the sweep (design.md → Decision 9); it fails the
     script when the sweep's own regex stops matching a line that holds all three forms.

     #!/bin/sh
     set -u
     PURE="src/ui/app.rs src/ui/detail.rs src/ui/list.rs src/ui/markdown.rs src/ui/tasks.rs src/ui/view.rs src/ui/driver.rs"
     PAT='\.chars\(\)\.count\(\)|\.chars\(\)\.take\(|Vec<char>'
     # Control 1: the sweep's OWN pattern must match a line holding all three forms.
     probe='let v: Vec<char> = s.chars().collect(); s.chars().count(); s.chars().take(1);'
     [ "$(printf '%s\n' "$probe" | grep -cE "$PAT")" = "1" ] \
       || { echo "COLWIDTH FAIL: self-test - the sweep pattern matches nothing" >&2; exit 1; }
     # Control 2: the measure being protected exists, so the exemption is not vacuous.
     [ -f src/ui/layout.rs ] || { echo "COLWIDTH FAIL: src/ui/layout.rs missing" >&2; exit 1; }
     grep -q 'cell_width' src/ui/layout.rs && grep -q 'styled_graphemes' src/ui/layout.rs \
       || { echo "COLWIDTH FAIL: positive control - src/ui/layout.rs names no cell_width/styled_graphemes" >&2; exit 1; }
     bad=0
     for f in $PURE; do
       [ -f "$f" ] || { echo "COLWIDTH FAIL: $f missing" >&2; exit 1; }
       n=$(awk '/^#\[cfg\(test\)\]/{exit} {print}' "$f" | grep -cE "$PAT")
       if [ "$n" != "0" ]; then
         echo "COLWIDTH FAIL: $f has $n char-count measurement(s) in production code" >&2
         awk '/^#\[cfg\(test\)\]/{exit} {print NR": "$0}' "$f" | grep -E "$PAT" >&2
         bad=$((bad+1))
       fi
     done
     [ "$bad" = "0" ] || exit 1
     echo "COLWIDTH OK: no char-count measurement in the seven pure view files"
-->

<!-- EVERY NEW TEST NAMES ITS MODULE'S MANDATED PAIR. LISTWIDTHS (38/58), MDWIDTHS,
     TASKWIDTHS and DETAILWIDTHS (58/78), and WIDTHS (60/120) require every `#[test]` in their
     file to name both literals bare and unsuffixed, and each states it has NO exemption list.
     A sweep over 0..=130 names neither, so each new test asserts explicitly at its mandated
     pair among the swept widths — the remedy taskwidths.sh's own header records for the
     existing sweep. design.md → Decision 4 carries the reasoning. -->

## 1. Display-width primitives in `ui::layout`
<!-- kind: behavior -->

- [x] 1.1 RED: Write failing tests for `columns` agrees with what the buffer consumed and `truncate_columns` never splits a cluster and never overruns. The oracle is the `x` that `Buffer::set_stringn(0, 0, s, usize::MAX, Style::default())` **returns**, never the first blank cell — `set_stringn` resets the trailing cells of a wide cluster, so a first-blank scan reports 1 for `日本語`.
- [x] 1.2 GREEN: Add `pub(crate) fn columns(&str) -> usize` to `src/ui/layout.rs`, summing `cell_width()` over `Span::raw(text).styled_graphemes(Style::default())` per design.md → Decision 1. Both tests pass.
- [x] 1.3 GREEN: Add `pub(crate) fn truncate_columns(&str, usize) -> &str`, deriving each symbol's cut point from its byte offset within the original `&str` rather than from a running sum of symbol lengths. The `ab`+BEL+`日本語` case in 1.1 passes without panicking.
- [x] 1.4 REFACTOR: Share the grapheme walk between the two functions if it removes duplication; otherwise record that none was needed.
- [x] 1.5 Run `cargo test --lib ui::layout` — green, and `cargo test --lib` shows no regression elsewhere.

## 2. The row grammar measures in columns
<!-- kind: behavior -->

- [x] 2.1 RED: Write failing tests for A CJK change name stays inside the list region at both mandated widths, An emoji change name at 58 columns does not overwrite the border, A wide name is truncated whole and padded back to the full width, Rows are total over adversarial names at every width, and The no-repository block shortens its search path by columns. Every new test names 38 and 58 bare.
- [x] 2.2 GREEN: Rewrite `pad_or_truncate_right` to measure with `layout::columns`, truncate with `layout::truncate_columns`, and pad the truncating arm back to exactly `width` columns per design.md → Decision 3.
- [x] 2.3 GREEN: Rewrite `shorten_left`, `shorten_left_row`, and the row/problem/message/no-repository assembly in `src/ui/list.rs` to measure in columns. `awk '/^#\[cfg\(test\)\]/{exit} {print}' src/ui/list.rs | grep -cE '\.chars\(\)\.count\(\)|\.chars\(\)\.take\(|Vec<char>'` reports 0, down from 10.
- [x] 2.4 REFACTOR: Restate the 8 existing `.chars().count()` width assertions in `src/ui/list.rs`'s test module as `layout::columns(...)`. No pass/fail may move — every fixture there is ASCII, where the two measures are equal.
- [x] 2.5 Run `cargo test --lib ui::list`, `cargo test --lib ui::view`, and `sh scripts/gates/listwidths.sh` — all green, no regressions.

## 3. Markdown wrapping measures in columns
<!-- kind: behavior -->

- [x] 3.1 RED: Write failing tests for A wide-character document wraps by columns at both mandated widths, and extend No line exceeds the width it was given and Rendering is total over arbitrary input with the CJK and ZWJ sources and widths 1 and 2 the specs name. Every new test names 58 and 78 bare.
- [x] 3.2 GREEN: Rewrite the wrap, the hard-split, `split_at_char`'s `char_indices` cut, the prefix budget, and the hanging indent in `src/ui/markdown.rs` to measure through `layout::` and cut at grapheme boundaries. The same `awk | grep -c` reports 0, down from 5.
- [x] 3.3 GREEN: Drop a single cluster wider than the whole region rather than emitting an over-wide line, per `markdown-render`'s carve-out for code blocks. The width-1 case in 3.1 passes.
- [x] 3.4 REFACTOR: Restate the 11 existing `.chars().count()` width assertions in `src/ui/markdown.rs`'s test module in columns; no pass/fail may move.
- [x] 3.5 Run `cargo test --lib ui::markdown`, `sh scripts/gates/mdwidths.sh`, and `sh scripts/gates/mdseam.sh` — all green, and `grep -c ratatui src/ui/markdown.rs` reports 0.

## 4. The checklist and the progress bar measure in columns
<!-- kind: behavior -->

- [x] 4.1 RED: Write failing tests for A checklist of wide-character items fits at both mandated widths, No checklist line exceeds its width at any width, `No tasks yet` does not eat the border at a narrow frame, and The bar measures at most its width at every width. Every new test names 58 and 78 bare.
- [x] 4.2 GREEN: Route `No tasks yet` through `ui::list::pad_or_truncate_right` at `width` in `src/ui/tasks.rs:266`, and rewrite the item wrap, heading line, and bar arithmetic to measure in columns. The `awk | grep -c` reports 0, down from 5.
- [x] 4.3 REFACTOR: Restate the 11 existing `.chars().count()` width assertions in `src/ui/tasks.rs`'s test module in columns; no pass/fail may move.
- [x] 4.4 Run `cargo test --lib ui::tasks` and `sh scripts/gates/taskwidths.sh` — green, and A prose-only tasks file reads `No tasks yet`, A missing tasks artifact still reads `No content yet`, and A read failure on the tasks tab still pass unchanged.

## 5. The detail header and content measure in columns
<!-- kind: behavior -->

- [x] 5.1 RED: Write failing tests for A CJK change name keeps the header inside its region at both mandated widths, The header reaches the buffer without crossing the region border, The header is total over adversarial names at every width, `No content yet` does not eat the border at a narrow frame, A wide-character document stays inside the detail region, and No `content_lines` line exceeds its width at any width. Every new test names 58 and 78 bare.
- [x] 5.2 GREEN: Route `No content yet` through `ui::list::pad_or_truncate_right` at `width` in `src/ui/detail.rs:225`, and rewrite `header_row`'s cell widths, name-field budget, and band boundaries to measure in columns. The `awk | grep -c` reports 0, down from 4.
- [x] 5.3 REFACTOR: Restate the 13 existing `.chars().count()` width assertions in `src/ui/detail.rs`'s test module in columns; no pass/fail may move.
- [x] 5.4 Run `cargo test --lib ui::detail` and `sh scripts/gates/detailwidths.sh` — green, its mandated 58/78 pair unchanged.

## 6. The draw loop advances by consumed columns
<!-- kind: behavior -->

- [x] 6.1 RED: Write failing tests for A wide-character path is shortened by columns and stays inside the header, and the border-cell assertions of A CJK change name stays inside the list region and A wide-character document stays inside the detail region driven through a full `ui::view::render`. Every new test names 60 and 120 bare.
- [x] 6.2 GREEN: Change `src/ui/view.rs:131` to advance `x` by `layout::columns(&segment.text)` and to draw `layout::truncate_columns(&segment.text, (last_col - x) as usize)`, keeping the `x >= last_col` guard, per design.md → Decision 8.
- [x] 6.3 GREEN: Rewrite the header's `A` arithmetic and the footer's hint budget and prompt tail-truncation in `src/ui/view.rs` to measure in columns. The `awk | grep -c` reports 0, down from 5.
- [x] 6.4 REFACTOR: Restate the 18 existing `.chars().count()` width assertions in `src/ui/view.rs`'s test module in columns; no pass/fail may move.
- [x] 6.5 Run `cargo test --lib ui::view` and `sh scripts/gates/widths.sh` — green, the mandated 60/120 pair unchanged.

## 7. `Enter` guards its reset, and the filter folds all of Unicode
<!-- kind: behavior -->

- [x] 7.1 RED: Write failing tests for `Enter` at the detail route moves nothing and keeps the scroll (both the `apply` and the scripted `run_loop` rows of the matrix), `Enter` from the list route still opens at the top, `Enter` while filtering still dismisses the filter and resets nothing, Matching ignores case outside ASCII, and The fold is total and its documented edge cases hold.
- [x] 7.2 GREEN: Guard `Action::OpenDetail`'s `self.route = Route::Detail; self.detail.scroll = 0;` on `self.route != Route::Detail` in `src/ui/app.rs:212`, per design.md → Decision 5.
- [x] 7.3 GREEN: Change `matches` at `src/ui/app.rs:575` to fold both sides with `str::to_lowercase`, and update its doc comment. `src/state.rs:84` stays `to_ascii_lowercase` (design.md → Decision 6).
- [x] 7.4 REFACTOR: Fold the three `Enter` scenarios' shared `Dashboard` setup into one helper if it removes duplication; otherwise record that none was needed.
- [x] 7.5 Run `cargo test --lib ui::app` and `cargo test --lib ui::driver` — green, and Every route move resets the scroll still passes unchanged.

## 8. The `COLWIDTH` gate and the re-measured width floors
<!-- kind: operational -->

- [x] 8.1 CHECK: Confirm the RED evidence recorded in the header block still holds — run the script body from a scratch path outside the repo against this working tree, before 8.2 creates the file. It must exit 1 on the control.
- [x] 8.2 CHANGE: Add the script verbatim from the header block above as `scripts/gates/colwidth.sh` and add `/bin/sh scripts/gates/colwidth.sh` to the `Makefile`'s `gates:` recipe, run bare with no `MIN` override.
- [x] 8.3 VERIFY: Negative controls, three of them. Corrupt the sweep regex and confirm the self-test fires; delete `styled_graphemes` from `src/ui/layout.rs` and confirm control 2 fires; reintroduce one `.chars().count()` into `src/ui/list.rs` production code and confirm the sweep fires naming that file and line. Restore after each and confirm exit 0.
- [x] 8.4 VERIFY: `cargo test --test ci_workflow` — green, so the recipe names every script under `scripts/gates/` and vice versa with `colwidth.sh` present.
- [x] 8.5 VERIFY: `make gates` — green.
- [x] 8.6 CHANGE: Re-measure each width gate's `#[test]` count with `grep -cE '^[[:space:]]*#\[test\][[:space:]]*$' src/ui/<f>.rs` and raise `LIST_MIN`, `MD_MIN`, `TASK_MIN`, `DETAIL_MIN`, and `WIDTHS_MIN` to the new true counts in their own scripts. Re-run `make gates` — green.

## 9. Change Review
<!-- kind: operational -->

- [x] 9.1 CHECK: Dispatch an independent reviewer — not a fork of this session — against proposal.md, all nine delta specs, design.md, and the diff.
- [x] 9.2 CHANGE: Fix every CRITICAL, resolve or accept each WARNING with a one-line reason, note SUGGESTIONs, and re-run affected tests.
- [x] 9.3 VERIFY: Confirm every one of the 82 spec scenarios in design.md → Test Strategy has a named test that would go red if its behaviour were deleted, and that no finding is left unowned. An exact-title diff of the nine deltas against `openspec/specs/` gives 25 new scenarios and 57 carried forward from landed specs (25 + 57 = 82); the 57 carried-forward scenarios are covered by their group's REFACTOR task. (Corrected from an earlier draft's "51 scenarios carried forward": the 6-scenario gap is scenarios whose title is unchanged but whose body was reworded, which the earlier count missed.) Check that the clauses **reworded** in the deltas — `tasks-progress-bar`'s byte-identical assertion and `artifact-content`'s "measures exactly the interior width" — have assertions of their own.

## 10. Documentation
<!-- kind: operational -->

- [ ] 10.1 CHECK: Read `AGENTS.md` → Architecture rules and "Current repo state", and `SPEC.md`'s view-layer section, and list every sentence there that states a rendered length in characters or enumerates the `make gates` inventory. That list is what 10.2–10.4 rewrite.
- [ ] 10.2 CHANGE: Rewrite in `AGENTS.md` → Architecture rules (audience: agents working in this crate) — every width computation under `src/ui/` is in display columns through `ui::layout::columns`/`truncate_columns`, which measure what `Buffer::set_string` consumes, enforced by `COLWIDTH`. Four lines; replaces nothing existing, and the durable reason is that a `chars().count()` written later is silently correct against ASCII fixtures.
- [ ] 10.3 CHANGE: Rewrite in `SPEC.md`'s view-layer section (audience: this project's maintainers) — the Unicode promise and its stated limit, correcting the sentences 10.1 listed. This replaces prose that is now false rather than only adding.
- [ ] 10.4 CHANGE: Rewrite `AGENTS.md`'s "Current repo state" gate inventory in place so `COLWIDTH` sits with the rest rather than being appended as a second list.
- [ ] 10.5 VERIFY: `cargo test --test spec_purposes` and `cargo test --test degraded_coverage` — green, so the `SPEC.md` edit did not break either file's binding to it.

## 11. Lint & Verify
<!-- kind: operational -->

- [ ] 11.1 CHECK: Inspect the intended verification commands and affected tiers — `ui::layout`, `ui::list`, `ui::markdown`, `ui::tasks`, `ui::detail`, `ui::view`, `ui::app`, `ui::driver`, plus `make gates` and `tests/ci_workflow.rs`. Time the 0..=130 sweeps; if any exceeds one second, narrow it by input and never by width (design.md → Risks).
- [ ] 11.2 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.
- [ ] 11.3 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 11.4 VERIFY: `make gates` — green, `COLWIDTH` included.
- [ ] 11.5 VERIFY: `cargo test --all-features` — green.
- [ ] 11.6 VERIFY: `cargo llvm-cov --fail-under-lines 80` — at or above the floor. Never lower it; if it falls short, add tests.
- [ ] 11.7 VERIFY: `openspec validate view-fidelity --strict` — valid.
