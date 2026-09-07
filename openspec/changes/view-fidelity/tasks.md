<!-- No outer-loop acceptance group. design.md → Test Strategy states why: every behaviour
     here is observable in a TestBackend buffer or a pure return value, and neither seam that
     would justify an outer loop (the openspec binary, the Herdr socket) is on any path this
     change touches. -->

<!-- PARALLELISM: none of groups 1-8 is marked parallel. They touch seven different files and
     several pairs are genuinely independent by dependency — group 3 (`ui::markdown`) needs
     nothing group 2 wrote, and neither does group 7 (`ui::app`). What fails is the third
     condition: `cargo build` and `cargo test --lib` compile the whole crate, so a half-written
     `src/ui/markdown.rs` reddens every other group's run and a failure is not attributable to
     its own group. Sequential by toolchain, not by design. -->

<!-- NUMBERS IN THIS PLAN, and the commands that produced them.
     29 char-count measurement sites in production code, by file:
       for f in src/ui/app.rs src/ui/detail.rs src/ui/list.rs src/ui/markdown.rs \
                src/ui/tasks.rs src/ui/view.rs src/ui/driver.rs; do
         n=$(awk '/^#\[cfg\(test\)\]/{exit} {print}' "$f" \
             | grep -cE '\.chars\(\)\.count\(\)|\.chars\(\)\.take\(|Vec<char>'); echo "$f $n"; done
     -> app.rs 0, detail.rs 4, list.rs 10, markdown.rs 5, tasks.rs 5, view.rs 5, driver.rs 0.
     Existing #[test] counts (`grep -c '^\s*#\[test\]' <file>`): layout 15, list 32,
     markdown 25, tasks 46, detail 32, view 98, app 72, driver 35.
     `to_ascii_lowercase` call sites (`grep -rn to_ascii_lowercase src/`): src/ui/app.rs:575
     and :576 (this change's), src/state.rs:84 (deliberately untouched, design.md → 6). -->

<!-- THE GATE, WRITTEN OUT AND RUN AT HEAD. `sh scripts/gates/colwidth.sh` at HEAD exits **1**
     with `COLWIDTH FAIL: positive control - src/ui/layout.rs names no
     cell_width/styled_graphemes` — RED, and red on the control before it reaches the sweep,
     which is what proves the control is load-bearing. With the control satisfied the sweep
     reports the 29 sites above. Script body, verbatim, as group 8 lands it:

     #!/bin/sh
     set -u
     PURE="src/ui/app.rs src/ui/detail.rs src/ui/list.rs src/ui/markdown.rs src/ui/tasks.rs src/ui/view.rs src/ui/driver.rs"
     [ -f src/ui/layout.rs ] || { echo "COLWIDTH FAIL: src/ui/layout.rs missing" >&2; exit 1; }
     grep -q 'cell_width' src/ui/layout.rs && grep -q 'styled_graphemes' src/ui/layout.rs \
       || { echo "COLWIDTH FAIL: positive control - src/ui/layout.rs names no cell_width/styled_graphemes" >&2; exit 1; }
     bad=0
     for f in $PURE; do
       [ -f "$f" ] || { echo "COLWIDTH FAIL: $f missing" >&2; exit 1; }
       n=$(awk '/^#\[cfg\(test\)\]/{exit} {print}' "$f" \
           | grep -cE '\.chars\(\)\.count\(\)|\.chars\(\)\.take\(|Vec<char>')
       if [ "$n" != "0" ]; then
         echo "COLWIDTH FAIL: $f has $n char-count measurement(s) in production code" >&2
         awk '/^#\[cfg\(test\)\]/{exit} {print NR": "$0}' "$f" \
           | grep -E '\.chars\(\)\.count\(\)|\.chars\(\)\.take\(|Vec<char>' >&2
         bad=$((bad+1))
       fi
     done
     [ "$bad" = "0" ] || exit 1
     echo "COLWIDTH OK: no char-count measurement in the seven pure view files"
-->

## 1. Display-width primitives in `ui::layout`
<!-- kind: behavior -->

- [ ] 1.1 RED: Write failing tests for `columns` agrees with what the buffer consumed and `truncate_columns` never splits a cluster and never overruns. The first writes each of the seven strings into a fresh 40x1 `ratatui::buffer::Buffer` with `set_string` and asserts `columns` equals the index of the first reset cell.
- [ ] 1.2 GREEN: Add `pub(crate) fn columns(&str) -> usize` to `src/ui/layout.rs`, summing `cell_width()` over `Span::raw(text).styled_graphemes(Style::default())` per design.md → Decision 1. Both tests pass.
- [ ] 1.3 GREEN: Add `pub(crate) fn truncate_columns(&str, usize) -> &str`, returning the longest grapheme-boundary prefix fitting `max`. The sweep test over `日本語の変更` and `abc🎉def` passes at every `max`.
- [ ] 1.4 REFACTOR: Share the grapheme iterator between the two functions if it removes duplication; otherwise record that none was needed.
- [ ] 1.5 Run `cargo test --lib ui::layout` — green, and `cargo test --lib` shows no regression elsewhere.

## 2. The row grammar measures in columns
<!-- kind: behavior -->

- [ ] 2.1 RED: Write failing tests for A CJK change name stays inside the list region at both mandated widths, An emoji change name at 58 columns does not overwrite the border, A wide name is truncated whole and padded back to the full width, Rows are total over adversarial names at every width, and The no-repository block shortens its search path by columns.
- [ ] 2.2 GREEN: Rewrite `pad_or_truncate_right` to measure with `layout::columns`, truncate with `layout::truncate_columns`, and pad the truncating arm back to exactly `width` columns per design.md → Decision 3.
- [ ] 2.3 GREEN: Rewrite `truncate_left` and the row/problem/message/no-repository assembly in `src/ui/list.rs` to measure in columns. `awk '/^#\[cfg\(test\)\]/{exit} {print}' src/ui/list.rs | grep -cE '\.chars\(\)\.count\(\)|\.chars\(\)\.take\(|Vec<char>'` reports 0, down from 10.
- [ ] 2.4 REFACTOR: Restate the 32 existing `ui::list` width assertions as `layout::columns(...)`. No pass/fail may move — every fixture there is ASCII, where the two measures are equal.
- [ ] 2.5 Run `cargo test --lib ui::list` and `cargo test --lib ui::view` — green, no regressions.

## 3. Markdown wrapping measures in columns
<!-- kind: behavior -->

- [ ] 3.1 RED: Write failing tests for A wide-character document wraps by columns at both mandated widths, and extend No line exceeds the width it was given and Rendering is total over arbitrary input with the CJK and ZWJ sources and widths 1 and 2 the specs name.
- [ ] 3.2 GREEN: Rewrite the wrap, hard-split, prefix budget, and hanging indent in `src/ui/markdown.rs` to measure through `layout::columns`/`layout::truncate_columns`, splitting at grapheme boundaries. The same `awk | grep -c` reports 0, down from 5.
- [ ] 3.3 GREEN: Drop a single cluster wider than the whole region rather than emitting an over-wide line, per `markdown-render`'s rule. The width-1 case in 3.1 passes.
- [ ] 3.4 CHECK: `grep -c ratatui src/ui/markdown.rs` reports 0 and `sh scripts/gates/mdseam.sh` exits 0 — the module reached the new measure through `layout::` without naming a ratatui type.
- [ ] 3.5 Run `cargo test --lib ui::markdown` — green, and the 25 existing tests still pass with their assertions restated in columns.

## 4. The checklist and the progress bar measure in columns
<!-- kind: behavior -->

- [ ] 4.1 RED: Write failing tests for A checklist of wide-character items fits at both mandated widths, No checklist line exceeds its width at any width, `No tasks yet` does not eat the border at a narrow frame, and The bar measures at most its width at every width.
- [ ] 4.2 GREEN: Route `No tasks yet` through `ui::list::pad_or_truncate_right` at `width` in `src/ui/tasks.rs:266`, and rewrite the item wrap, heading line, and bar arithmetic to measure in columns. The `awk | grep -c` reports 0, down from 5.
- [ ] 4.3 REFACTOR: Restate the 46 existing `ui::tasks` width assertions in columns; no pass/fail may move.
- [ ] 4.4 Run `cargo test --lib ui::tasks` — green, and A prose-only tasks file reads `No tasks yet`, A missing tasks artifact still reads `No content yet`, and A read failure on the tasks tab still pass unchanged.

## 5. The detail header and content measure in columns
<!-- kind: behavior -->

- [ ] 5.1 RED: Write failing tests for A CJK change name keeps the header inside its region at both mandated widths, The header reaches the buffer without crossing the region border, The header is total over adversarial names at every width, `No content yet` does not eat the border at a narrow frame, A wide-character document stays inside the detail region, and No `content_lines` line exceeds its width at any width.
- [ ] 5.2 GREEN: Route `No content yet` through `ui::list::pad_or_truncate_right` at `width` in `src/ui/detail.rs:225`, and rewrite `header_row`'s cell widths, name-field budget, and band boundaries to measure in columns. The `awk | grep -c` reports 0, down from 4.
- [ ] 5.3 REFACTOR: Restate the 32 existing `ui::detail` width assertions in columns; no pass/fail may move.
- [ ] 5.4 Run `cargo test --lib ui::detail` — green, and `sh scripts/gates/detailwidths.sh` exits 0 with its mandated 58/78 pair unchanged.

## 6. The draw loop advances by consumed columns
<!-- kind: behavior -->

- [ ] 6.1 RED: Write failing tests for A wide-character path is shortened by columns and stays inside the header, and the border-cell assertions of A CJK change name stays inside the list region and A wide-character document stays inside the detail region driven through a full `ui::view::render` at 60 and 120.
- [ ] 6.2 GREEN: Change `src/ui/view.rs:131` to advance `x` by `layout::columns(&segment.text)` and to draw `layout::truncate_columns(&segment.text, (last_col - x) as usize)`, keeping the `x >= last_col` guard, per design.md → Decision 8.
- [ ] 6.3 GREEN: Rewrite the header's `A` arithmetic and the footer's hint budget in `src/ui/view.rs` to measure in columns. The `awk | grep -c` reports 0, down from 5.
- [ ] 6.4 REFACTOR: Restate the 98 existing `ui::view` width assertions in columns; no pass/fail may move.
- [ ] 6.5 Run `cargo test --lib ui::view` and `sh scripts/gates/widths.sh` — green, the mandated 60/120 pair unchanged.

## 7. `Enter` guards its reset, and the filter folds all of Unicode
<!-- kind: behavior -->

- [ ] 7.1 RED: Write failing tests for `Enter` at the detail route moves nothing and keeps the scroll (both the `apply` and the scripted `run_loop` rows of the matrix), `Enter` from the list route still opens at the top, `Enter` while filtering still dismisses the filter and resets nothing, Matching ignores case outside ASCII, and The fold is total and allocates no surprise.
- [ ] 7.2 GREEN: Guard `Action::OpenDetail`'s `self.route = Route::Detail; self.detail.scroll = 0;` on `self.route != Route::Detail` in `src/ui/app.rs:212`, per design.md → Decision 5.
- [ ] 7.3 GREEN: Change `matches` at `src/ui/app.rs:575` to fold both sides with `str::to_lowercase`, and update its doc comment. `src/state.rs:84` stays `to_ascii_lowercase` (design.md → Decision 6).
- [ ] 7.4 Run `cargo test --lib ui::app` and `cargo test --lib ui::driver` — green, and Every route move resets the scroll still passes unchanged.

## 8. The `COLWIDTH` gate
<!-- kind: operational -->

- [ ] 8.1 CHECK: `sh scripts/gates/colwidth.sh` — at HEAD this exits **1** on the positive control; after groups 1-7 it must exit 0. Record both.
- [ ] 8.2 CHANGE: Add the script verbatim from the header block above as `scripts/gates/colwidth.sh` and add `/bin/sh scripts/gates/colwidth.sh` to the `Makefile`'s `gates:` recipe, run bare with no `MIN` override.
- [ ] 8.3 VERIFY: Negative control — delete `styled_graphemes` from `src/ui/layout.rs` and confirm the gate exits 1 on the control; restore it and confirm exit 0. Then reintroduce one `.chars().count()` into `src/ui/list.rs` production code, confirm exit 1 naming that file and line, and remove it.
- [ ] 8.4 VERIFY: `cargo test --test ci_workflow` — green, so the recipe names every script under `scripts/gates/` and vice versa with `colwidth.sh` present.
- [ ] 8.5 VERIFY: `make gates` — green.

## 9. Change Review
<!-- kind: operational -->

- [ ] 9.1 CHECK: Dispatch an independent reviewer — not a fork of this session — against proposal.md, all nine delta specs, design.md, and the diff.
- [ ] 9.2 CHANGE: Fix every CRITICAL, resolve or accept each WARNING with a one-line reason, note SUGGESTIONs, and re-run affected tests.
- [ ] 9.3 VERIFY: Confirm every one of the 58 spec scenarios in design.md → Test Strategy has a named test that would go red if its behaviour were deleted, and that no finding is left unowned.

## 10. Documentation
<!-- kind: operational -->

- [ ] 10.1 Rewrite in `AGENTS.md`: Architecture rules (audience: agents working in this crate). Replace nothing; add one rule — every width computation under `src/ui/` is in display columns through `ui::layout::columns`/`truncate_columns`, which measure what `Buffer::set_string` consumes, and `COLWIDTH` enforces it. Four lines, and it is durable: a `chars().count()` written by a future change is silently correct against ASCII fixtures.
- [ ] 10.2 Rewrite in `SPEC.md`: the view-layer section (audience: this project's maintainers). State the Unicode promise — grapheme clusters, over-reserving for ZWJ rather than overflowing, no normalisation, no locale tailoring — and correct any sentence there that counts characters. This replaces prose that is now false, so the section does not only grow.
- [ ] 10.3 Rewrite in `AGENTS.md`: the "Current repo state" paragraph naming `make gates`' gate inventory, so `COLWIDTH` is listed with the rest rather than appended as a second inventory.

## 11. Lint & Verify
<!-- kind: operational -->

- [ ] 11.1 CHECK: Inspect the intended verification commands and affected tiers — `ui::layout`, `ui::list`, `ui::markdown`, `ui::tasks`, `ui::detail`, `ui::view`, `ui::app`, `ui::driver`, plus `make gates` and `tests/ci_workflow.rs`.
- [ ] 11.2 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.
- [ ] 11.3 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 11.4 VERIFY: `make gates` — green, `COLWIDTH` included.
- [ ] 11.5 VERIFY: `cargo test --all-features` — green.
- [ ] 11.6 VERIFY: `cargo llvm-cov --fail-under-lines 80` — at or above the floor. Never lower it; if it falls short, add tests.
- [ ] 11.7 VERIFY: `openspec validate view-fidelity --strict` — valid.
