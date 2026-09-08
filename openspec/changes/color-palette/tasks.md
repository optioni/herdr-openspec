Every number below names the command that produced it. Every check is written out in full and
was run against HEAD (`3450086`) while this plan was written; its exit status and the relevant
line of its output are recorded beside it.

**Group ordering.** Groups 2, 3, and 4 all edit `src/ui/view.rs`, and one file is shared
mutable state, so none of them is marked parallel. Groups 3 and 4 are otherwise independent of
each other — different grammar modules, different scenarios — and would qualify but for that
file. No `parallel-after` marker appears in this plan.

## 0. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

The one claim of this change no unit test can make is a claim about every file that does not
exist yet: that a `Color` named anywhere but the palette fails the build. Its harness is the
existing planted-defect tier (`tests/gate-controls.toml` + `tests/gate_controls.rs`), and no
collaborator is replaced — the test copies the real tree.

- [x] 0.1 Add `scripts/gates/palette.sh` with exactly this content, and add
      `/bin/sh scripts/gates/palette.sh` to the `Makefile`'s `gates:` recipe in alphabetical
      position (after `openspec-untouched.sh`, before `readonly-ui.sh`):

      ```sh
      # PALETTE — the colour table is confined to ONE module. src/ui/palette.rs is the only
      # file in the crate that may name a ratatui Color, on exactly MDSEAM's terms: excluded
      # BY PATH so a future src/palette.rs is still searched, with the positive control
      # checked BEFORE the sweep so a gutted table is reported as vacuous rather than clean.
      PAL="${PAL:-src/ui/palette.rs}"
      MIN="${MIN:-25}"
      fail() { echo "PALETTE FAIL: $1" >&2; exit 1; }

      [ -d src ] || fail "no src directory"
      [ -f "$PAL" ] || fail "$PAL missing - the exclusion has nothing to exclude"

      # Guard B - positive control: the allowed file must actually name a Color.
      grep -qE 'Color::' "$PAL" || fail "$PAL names no Color - exclusion is vacuous"

      # Guard C - the searched set is real.
      n=$(find src -name '*.rs' ! -path "$PAL" | wc -l | tr -d ' ')
      [ "$n" -ge "$MIN" ] || fail "searched only $n files (expected >= $MIN)"

      hits=$(find src -name '*.rs' ! -path "$PAL" -print0 \
             | xargs -0 -I{} grep -nE 'Color::|ratatui::style::Color' {} /dev/null 2>&1 || true)
      [ -z "$hits" ] || { echo "PALETTE FAIL: a ratatui Color is named outside $PAL:" >&2
                          echo "$hits" >&2; exit 1; }

      # Second leg - named ANSI indices only.
      r=$(grep -nE 'Color::Rgb|Color::Indexed|Color::Reset' "$PAL" || true)
      [ -z "$r" ] || { echo "PALETTE FAIL: $PAL names a non-ANSI colour:" >&2
                       echo "$r" >&2; exit 1; }

      # Third leg - the two standing view gates actually sweep the new module. Both
      # hard-code their PURE lists and check only that the files they name exist, so
      # without this a forgotten edit leaves palette.rs unswept while both print OK.
      for g in scripts/gates/noio-view.sh scripts/gates/colwidth.sh; do
        [ -f "$g" ] || fail "$g missing"
        grep -q "$PAL" "$g" || fail "$g does not list $PAL in its PURE set"
      done
      echo "PALETTE OK: $n files searched (>= $MIN), Color only in $PAL, named ANSI indices only, swept by NOIO-VIEW and COLWIDTH"
      ```

      `MIN=25` is the measured floor:
      `find src -name '*.rs' ! -path src/ui/palette.rs | wc -l` → **25** once `palette.rs`
      exists (26 files less the excluded one); it is **25** at HEAD too, where the file is
      absent. Recorded planning-time runs, all five in a scratch copy of `src/`:

      | Condition | Exit | Output line |
      |---|---|---|
      | HEAD, no `palette.rs` | 1 | `PALETTE FAIL: src/ui/palette.rs missing - the exclusion has nothing to exclude` |
      | stub `palette.rs`, nothing planted | 0 | `PALETTE OK: 25 files searched (>= 25), …` |
      | `use ratatui::style::Color;` planted in `src/ui/view.rs` | 1 | `PALETTE FAIL: a ratatui Color is named outside src/ui/palette.rs: src/ui/view.rs:4285:…` (`wc -l src/ui/view.rs` → 4284, so the appended line is 4285) |
      | plant removed | 0 | `PALETTE OK: 25 files searched (>= 25), …` |
      | `Color::Rgb(1,2,3)` planted in `palette.rs` | 1 | `PALETTE FAIL: src/ui/palette.rs names a non-ANSI colour:` |
      | `palette.rs` gutted of every `Color` | 1 | `PALETTE FAIL: src/ui/palette.rs names no Color - exclusion is vacuous` |
      | third leg, `PURE` lists not yet edited | 1 | `PALETTE FAIL: scripts/gates/noio-view.sh does not list src/ui/palette.rs in its PURE set` |
      | third leg, both `PURE` lists edited | 0 | `PALETTE OK: … swept by NOIO-VIEW and COLWIDTH` |

- [x] 0.2 RED: Add **three** `[[control]]` rows to `tests/gate-controls.toml`, one per failure
      mode the gate has — several controls per script is supported, as `scripts/gates/wired.sh`
      already shows. `palette-outside`: `plant_file = "src/ui/view.rs"`, a `plant_find` of that
      file's first doc-comment line, a `plant_replace` appending `// use ratatui::style::Color;`,
      `expect = "PALETTE FAIL: a ratatui Color is named outside"`. `palette-rgb`:
      `plant_file = "src/ui/palette.rs"`, appending `// Color::Rgb(1,2,3)`,
      `expect = "names a non-ANSI colour"`. `palette-unswept`:
      `plant_file = "scripts/gates/noio-view.sh"`, removing `src/ui/palette.rs` from its `PURE`
      line, `expect = "does not list src/ui/palette.rs in its PURE set"`. All three take
      `script = "palette.sh"` and an empty `env`. Verify with
      `cargo test --test gate_controls catch_their_plants` — RED at HEAD, because
      `gate_controls_catch_their_plants` requires each control's unplanted baseline run to
      exit 0 and this one exits 1 with `src/ui/palette.rs missing`. **Not**
      `cargo test --test gate_controls palette`: controls are table rows iterated by one
      `#[test]`, so no test name contains `palette` and that filter runs zero tests
      (`cargo test --test gate_controls palette -- --list` at HEAD → `0 tests`, exit 0).
- [x] 0.3 Confirm the failure is the missing module and not a misconfigured harness: the
      baseline-run assertion names `the exclusion has nothing to exclude`, and
      `cargo test --test ci_workflow` is **green**, so the recipe/script correspondence is
      already satisfied.

## 1. `ui::palette` — the role table
<!-- kind: behavior -->

- [ ] 1.1 RED: Write failing tests in `src/ui/palette.rs`, iterating a role list built from an
      **exhaustive** `match role { … }` so a `Role` added later fails to compile until it is
      covered, for: *The palette answers every role
      with a `Style`*, *Each role's modifier set is exactly the table above*, *The coloured set
      is exactly the table above*, and *An out-of-range heading level does not panic*. Each
      enumerates every `Role` variant, the five `AgentStatus` values, and heading levels 1
      through 6, so no arm is asserted by a hand-listed subset.
- [ ] 1.2 GREEN: Add `src/ui/palette.rs` — `Role`, `BadgeCell`-free, and
      `pub fn style(Role) -> Style` — per `specs/view-palette/spec.md`'s three tables, and
      declare `pub mod palette;` in `src/ui/mod.rs`. `Heading(l)` for `l` outside `1..=6`
      returns `Heading(6)`'s style.
- [ ] 1.3 CHANGE: Add `src/ui/palette.rs` to `PURE` in `scripts/gates/noio-view.sh` (its
      closing message becomes `9 pure files`, and its header comment saying "the set is EIGHT
      files rather than seven" is rewritten) and to `PURE` in `scripts/gates/colwidth.sh` (its
      closing message becomes `eight pure view files`). This gate edit sits here rather than in
      group 6 because `palette.sh`'s third leg fails until it lands, which would leave
      `make gates` red across groups 2 to 5. Verify with `make gates` — both lines report the
      new counts.
- [ ] 1.4 VERIFY: `/bin/sh scripts/gates/palette.sh` now exits 0, and
      `cargo test --test gate_controls` is green — all four of its tests, the
      `gate_controls_catch_their_plants` loop included. The outer-loop RED from group 0 is
      closed by the module existing, which is what makes it an outer loop.
- [ ] 1.5 REFACTOR: None expected — the module is one `match`. State so explicitly if nothing
      is extracted.
- [ ] 1.6 Run the group tests — `cargo test --lib ui::palette::` and `make gates` — no
      regressions.

## 2. `ui::view` takes every style from the palette
<!-- kind: behavior -->

Scope: `src/ui/view.rs` only. `Color::` is named **nowhere** in the crate at HEAD
(`grep -rnE 'Color::|ratatui::style::Color' src tests` → exit 1, no output), so every colour
assertion written here is RED by construction.

- [ ] 2.0 CHECK: Every colour assertion outside `src/ui/palette.rs` compares against
      `palette::style(role)`, never a `Color` literal — the gate from group 0 searches `src/`
      and this crate's view tests are inline `#[cfg(test)]` modules there. Verify by running
      `/bin/sh scripts/gates/palette.sh` after each RED task in groups 2, 3, and 4; it must
      stay at exit 0.
- [ ] 2.1 RED: Write failing tests for: *Faces reach the buffer as coloured styles at both
      mandated widths*, *Heading foreground wins over a code span inside it*, *A plain face is
      the default style*, *Faces reach the buffer as styles at both widths*, *The badge is
      drawn dim after the label at both widths*, *A false flag renders the header that landed
      before this change*, *The detail header is bold and uncoloured at both mandated widths*,
      *The routed region's border takes its style from the palette at both widths*,
      *The selected row is bold and uncoloured at both mandated widths*, and *A monochrome
      reading of the frame is unchanged*. Every render test runs at 120x20 **and** 60x20, and
      the two pure `style_for` tests must **also** name 60 and 120 — `WIDTHS` requires it of
      every `#[test]` in `src/ui/view.rs` with no exemption, and a comment satisfies its scan.
      Do not rename `file_mode_badge_is_dim_after_the_label` or
      `badge_drops_whole_below_eighteen_columns`: `tests/degraded-coverage.toml:18` names both
      as its proof, and a rename fails `cargo test --test degraded_coverage`.
- [ ] 2.2 GREEN: Rewrite `style_for` as a `Style::patch` fold over the palette in the order
      `Quoted, Link, Code, Emphasis, Strong, Heading` (per design.md → Decision 8).
- [ ] 2.3 GREEN: Replace every `Style` constructed in `render_header`, `render_region`,
      `render_list`, `render_detail_header`, `render_detail_tabs`, and `render_footer` — the
      last writes a bare `Style::default()` at `src/ui/view.rs:334` and takes `Role::Footer` —
      with
      `palette::style(role)` for the role `specs/view-palette/spec.md` names for that span.
      `ui::view` constructs no `Style` of its own afterwards except by `patch`. `render_region`
      passes the role's style to `Block::border_style`, never `Block::style`, so a blank
      interior's cells still equal `Cell::default().style()`.
- [ ] 2.4 CHECK: Contract gate — re-inspect `palette::style`'s signature and its consumers.
      `ui::view` is the only one; confirm `grep -rn 'palette::' src | grep -v '^src/ui/view.rs'`
      returns only the `pub mod palette;` declaration.
- [ ] 2.5 VERIFY: Every landed `Modifier::` **assertion** still passes unedited. The 32 lines
      `grep -rn 'Modifier::' src tests | wc -l` reports at HEAD split into 13 production lines
      and 19 inside `src/`'s test modules, with `tests/` contributing none:
      `for f in $(find src -name '*.rs'); do awk '/^#\[cfg\(test\)\]/{exit} /Modifier::/{print}' $f; done | wc -l`
      → **13**. Twelve of those 13 are the `add_modifier` call sites in `src/ui/view.rs` that
      task 2.3 replaces by construction; the invariant is over the **19 test-module lines**,
      which may grow but of which none may change. A line count cannot see an *edited* line, so
      the check is a diff: `git diff -U0 <BASE> -- src tests | grep '^[-+].*Modifier::'` must
      show only additions, plus the group-4 rewrites design.md → Decision 3 names. This is that
      decision's falsifiable half.
- [ ] 2.6 REFACTOR: Fold the six render functions' repeated role lookups if a shape emerges;
      otherwise state that no refactor was needed.
- [ ] 2.7 Run the group tests — `cargo test --lib ui::view::` — no regressions.

## 3. The agent badge cell and the problem row
<!-- kind: behavior -->

- [ ] 3.1 RED: Write failing tests in `src/ui/list.rs` for: *A badged active row reports the
      column its badge occupies, at both mandated widths*, *A badged archived row reports the
      column its badge occupies*, *A dropped badge cell reports no badge*, and *No non-change
      row carries a badge*. Every one names both **38** and **58**, which `LISTWIDTHS`
      requires of every `#[test]` in that file.
- [ ] 3.2 RED: Write failing tests in `src/ui/view.rs` for: *The badge cell reaches the buffer
      coloured and the rest of the row does not*, *Problem rows are red and change rows are
      not, at both mandated widths*, *An empty-state message row is not a problem row*, and
      *A badged selected row keeps its bold under the badge colour*.
- [ ] 3.3 GREEN: Add `BadgeCell { x, status }` and `Row::badge` in `src/ui/list.rs`, and have
      `active_style_row` and `archived_row_text` return the badge's column alongside their
      text. `Row::text` is byte-identical at every width — assert that against the landed
      row-grammar tests, unedited.
- [ ] 3.4 GREEN: In `render_list`, draw the row, then re-write the single cell at
      `interior.x + badge.x` with the row's own style patched by
      `palette::style(Role::AgentBadge(status))`, skipping it when `badge.x` is not less than
      the interior width.
- [ ] 3.5 CHECK: Contract gate — `Row` gained a field. Confirm every construction and
      exhaustive pattern is inside `src/ui/list.rs`:
      `grep -rn 'Row *{' src | grep -v '^src/ui/list.rs'` returns nothing.
- [ ] 3.6 REFACTOR: Collapse the two badge-column computations into one helper if they read
      as duplication; otherwise state that no refactor was needed.
- [ ] 3.7 Run the group tests — `cargo test --lib ui::list:: ui::view::` and `make gates` — no
      regressions.

## 4. The artifact tab bar becomes a row of chips
<!-- kind: behavior -->

Measured with
`python3 -c "ids=['proposal','specs','design','tasks','planning-review']; w=[len(i)+2 for i in ids]; print(sum(w)+2*4, sum(w)+1*4)"`
→ **57 53**: the bar falls from 57 columns to 53, so it fits the 78- and 58-column interiors
with more slack than today. The x offsets come from a second command,
`python3 -c "ids=['proposal','specs','design','tasks','planning-review']; x=0; out=[]\nfor i in ids:\n out.append(x); x+=len(i)+3\nprint(out, out[-1]+len(ids[-1])+1)"`
→ `[0, 11, 19, 28, 36] 52`.

- [ ] 4.1 RED: Rewrite the tab-bar tests in `src/ui/detail.rs` for the chip grammar, **and**
      every landed assertion in `src/ui/view.rs` that spells a numbered label:
      `grep -c '1 proposal\|3 gamma' src/ui/view.rs` at HEAD → **12** lines, at `:2981`,
      `:2990`, `:2999`, `:3008`, `:3046`, `:3053`, `:3060`, `:3067`, `:3340`, `:3347`,
      `:3363`, and `:3742`, across `a_degenerate_detail_interior_draws_nothing`, the tab-bar
      render test, and `no_marked_artifact_renders_markdown`. The `ui::detail` names are: *The five
      tdd artifacts become five numbered tabs at both mandated widths*, *A tenth artifact is
      labelled without a digit*, *Duplicate artifact ids remain two separately addressable
      tabs*, *No artifacts is a single placeholder cell, not an empty bar*, *A zero-width bar
      is empty and does not panic*, *A twelve-artifact bar windows to keep the selected tab
      visible*, *The window slides back when the selection moves left again*, *A selected cell
      wider than the whole bar is truncated rather than dropped*, and *A selected index past
      the end of the list does not panic*. Every one names both **78** and **58**
      (`DETAILWIDTHS`).
- [ ] 4.2 RED: Rewrite the two landed tab-bar tests that assert `Modifier::BOLD` at fixed
      columns — `the_five_tab_bars_exact_string_with_the_selected_tab_bold` (`:3320`, assertion
      at `:3353`) and `a_select_tab_at_route_list_is_visible_in_the_tab_row_at_120` (`:3370`,
      assertion at `:3390`) — for the chip grammar, and write *The tab bar reaches the buffer at
      both mandated widths* and *The tab bar never overwrites a border or the rows around it*,
      asserting the chip **backgrounds** and that the separating column carries none.
- [ ] 4.3 GREEN: In `tab_bar`, build each cell as `" {id} "`, change `joined_width`'s
      separator from 2 to 1, advance `x` by `cell_len + 1`, and delete the `i < 9` numbering
      branch. The `start`/`end` window loops stay byte-identical apart from that constant.
- [ ] 4.4 GREEN: In `render_detail_tabs`, style each cell `TabActive` when `cell.selected` and
      `TabInactive` otherwise, so every column of the chip including its padding is painted.
- [ ] 4.5 VERIFY: `cargo test --lib ui::app::` — the seven `1`-`9`/`[`/`]` scenarios still
      pass unedited; `action_for` and `Dashboard::apply` are untouched by the chip grammar.
      Then add the one render assertion *Tab keys act at both routes* needs: the third chip
      carries the active style at 120x20.
- [ ] 4.6 VERIFY: `cargo test --lib ui::layout::` — *`split_detail` is exact at its degenerate
      heights* still passes unedited; the split is untouched by the chip grammar.
- [ ] 4.7 REFACTOR: Remove the now-dead numbering branch and any helper it alone needed;
      otherwise state that no refactor was needed.
- [ ] 4.8 Run the group tests — `cargo test --lib ui::detail:: ui::view:: ui::app::` and
      `make gates` — no regressions.

## 5. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [ ] 5.1 VERIFY: `cargo test --test gate_controls` passes end to end — for control
      `palette-outside`, the unplanted baseline exits 0 and the planted `Color` in
      `src/ui/view.rs` exits non-zero naming that file. Confirm the control ran rather than
      being filtered out: `gate_controls_every_script_has_a_control` fails if
      `scripts/gates/palette.sh` has no row.
- [ ] 5.2 REFACTOR: Clean up the control's plant text if it drifted from
      `src/ui/view.rs`'s current first doc-comment line; otherwise state that none was needed.

## 6. Gate floors
<!-- kind: operational -->

A gate's floor is its own script default, kept at the gate's true measured floor.

- [ ] 6.1 CHECK: Re-measure the three **test-count** floors this change raises, with the
      commands that produced the current defaults:
      `awk '/^#\[cfg\(test\)\]/{t=1} t&&/#\[test\]/{c++} END{print c}' src/ui/list.rs` → **39**
      at HEAD (`LIST_MIN` default 39); the same over `src/ui/detail.rs` → **38**
      (`DETAIL_MIN` 38) and over `src/ui/view.rs` → **101** (`WIDTHS_MIN` 101).
- [ ] 6.2 CHECK: Re-measure the seven **file-count** floors that `src/ui/palette.rs` moves.
      Each is at its exact measured value today and would be one short after the file lands:
      `find src -name '*.rs' | wc -l` → **25** and `find src/ui -name '*.rs' | wc -l` → **11**
      at HEAD, giving `mdseam.sh MIN=24`, `nospawn-grep.sh MIN=24`, `nolit-change.sh MIN=24`
      (each `25 − 1` excluded file), `noblock.sh`, `nocli-shell.sh`, `readonly-ui.sh`
      `UI_MIN=11`, and `readseam.sh UI_MIN=10`. Each rises by one.
- [ ] 6.3 CHANGE: Raise all ten **script defaults** to the newly measured counts. Do not pass
      an override on a `Makefile` line. `scripts/gates/palette.sh`'s own `MIN=25` already
      accounts for the new file and does not move.
- [ ] 6.4 CHECK: Re-run `make coverage` and read `scripts/coverage-prod.py`'s reported
      production-slice percentage against its `DEFAULT_PROD_MIN = 96`
      (`grep -n DEFAULT_PROD_MIN scripts/coverage-prod.py` → line 84). Raise the default to
      the newly measured floor if the fully-covered palette module lifts it past 97; leave it
      alone otherwise. Never lower it.
- [ ] 6.5 VERIFY: `make gates` — every gate reports OK and each floor equals its measured
      count, so a deleted test or a deleted file fails the next run.

## 7. Change Review
<!-- kind: operational -->

- [ ] 7.1 CHECK: Dispatch `outside-in-tdd-reviewer` against proposal.md, all seven spec files,
      design.md, and the diff — not this session's reasoning. Concentration points, in
      addition to the standing ones: whether any colour assertion could pass with the palette
      deleted; whether `Row::text` is genuinely byte-identical at every width rather than
      merely at the two mandated ones; whether the chip window rule changed anywhere beyond
      the separator constant; and whether `make gates`' new line is reached in CI.
- [ ] 7.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
      one-line reason, note SUGGESTIONs, and re-run affected tests.
- [ ] 7.3 VERIFY: Confirm no blocking or unowned finding remains.

## 8. Documentation
<!-- kind: operational -->

- [ ] 8.0 CHECK: Read every sentence this change makes false against the tree it produces —
      `AGENTS.md:305` and `:340`, `SPEC.md:434-437`, `SPEC.md`'s `ui` module-map row, and the
      `covers` ranges in `tests/degraded-coverage.toml` — and confirm which are bound by a test
      and which only by a reader. Decide `SPEC.md:757` explicitly: its degraded-states row calls
      the badge "dim", which stays true, and the row's text is the `condition` key
      `tests/degraded-coverage.toml` binds it by — so reword it only together with that file, or
      record that it is deliberately left as it is.
- [ ] 8.1 CHANGE — rewrite in `AGENTS.md`: § Architecture rules, the `pulldown_cmark` bullet (audience:
      every future session). Extend it in place to name `ratatui::style::Color`'s confinement
      to `src/ui/palette.rs` beside the parser's to `src/ui/markdown.rs` — one bullet stating
      one rule about two replaceable-by-editing-one-file seams, rather than a second bullet
      repeating the argument. Durable because the next change adding a colour will otherwise
      add it at the render call site.
- [ ] 8.2 CHANGE — rewrite in `AGENTS.md`: § Architecture rules, the two sentences at `:305`
      and `:340` giving the pure view set as **eight** files and `COLWIDTH`'s sweep as the
      **other seven** (audience: every future session). Both are false once `palette.rs`
      lands, and `tests/doc_contract.rs` does not bind either number.
- [ ] 8.3 CHANGE — add in `SPEC.md`: § User interface, the semantic-role table — role, modifier,
      colour — and one sentence naming `src/ui/palette.rs` as its only home (audience: anyone
      implementing a view). This is net-new, ~25 lines; it replaces nothing because `SPEC.md`
      has no styling section today, and it is what makes a future "which colour means what"
      question answerable without reading the match arm.
- [ ] 8.4 CHANGE — rewrite in `SPEC.md`: § User interface, the sentence at `SPEC.md:434-437` reading
      "`1`-`9` select the first nine positions directly; a tenth position and beyond carry no
      digit in their label" (audience: anyone implementing a view). Its contrast is false once
      no label carries a digit. Nothing binds it — `tests/doc_contract.rs`'s legs are the
      module map, § Unit-tested modules, the worker-thread count, the MSRV, the gate-path
      programs, the manifest transcription, and the injected context — so a human must.
- [ ] 8.5 CHANGE — rewrite in `SPEC.md`: the `ui` row of the module map (audience: same) — name the
      palette among that module's responsibilities. `tests/doc_contract.rs` binds the module
      map to `src/lib.rs`'s `pub mod` set, which `ui::palette` does not join, so this row is
      the only place the new module is discoverable.
- [ ] 8.6 CHANGE: Re-point the `covers` line ranges in `tests/degraded-coverage.toml` that
      name `src/ui/view.rs`, `src/ui/list.rs`, or `src/ui/detail.rs` (nine entries;
      `grep -n 'covers' tests/degraded-coverage.toml`). `validate_covers` checks only path
      existence, in-bounds range, and one non-comment line, so groups 2 to 4 shifting those
      lines fails nothing while the map stops pointing at the code it names.
- [ ] 8.7 VERIFY: `cargo test --test doc_contract` and `cargo test --test degraded_coverage` —
      both green.

## 9. Lint & Verify
<!-- kind: operational -->

- [ ] 9.1 CHECK: Inspect the intended verification commands and affected tiers — `make check`
      runs format, lint, `make gates`, `cargo test --all-features`, and coverage at both
      floors; every tier this change touches is inside it.
- [ ] 9.2 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 9.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.
- [ ] 9.4 VERIFY: `make gates` — every gate OK, `PALETTE` among them.
- [ ] 9.5 VERIFY: `cargo test --all-features` — green.
- [ ] 9.6 VERIFY: `make coverage` — it is what exports the JSON `scripts/coverage-prod.py`
      reads, which a bare `cargo llvm-cov --fail-under-lines 80` does not. Both floors pass; `src/ui/palette.rs` is fully covered by its table-driven tests.
- [ ] 9.7 VERIFY: `make check` as the single gate; name the failing sub-command if it fails.
- [ ] 9.8 VERIFY: `openspec validate color-palette --strict` — valid.

**After `openspec archive` — not apply work, and deliberately not a checkbox.** Archiving
writes `openspec/specs/view-palette/spec.md`, and `tests/spec_purposes.rs` requires every
capability spec to carry a `## Purpose`; a new capability arrives without one. The same step
should correct `openspec/specs/artifact-tabs/spec.md`'s `## Purpose`, which still describes
tabs "labelled `"<n> <id>"`". Neither can be done during apply: the tracked-diff leg of
`OPENSPEC-UNTOUCHED` fails any change that edits `openspec/specs/` from an apply session.
