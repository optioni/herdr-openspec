<!-- Sequencing finding: groups 1, 2, 3, 4 and 6 all edit `src/ui/markdown.rs`, and one file
     is shared mutable state, so none of them may run in parallel. Group 5 edits
     `src/ui/tasks.rs` instead, but its cross-check assertion needs group 4's task-list
     rendering to exist; group 7's `cargo test --test degraded_coverage` needs group 4 for
     the same reason AND group 5, via the `ui::tasks::lines` cross-check the delta adds at
     `specs/markdown-render/spec.md`. Beyond the shared file and those dependencies, every
     group verifies with `cargo test --lib ui::`, which compiles the whole lib — so no pair
     could satisfy the third condition either, a failure staying attributable to its own
     group. The plan is sequential throughout and carries no `parallel-after` marker. -->

<!-- Recorded HEAD for every check below is `8f069d0`. The tree has since advanced to
     `68b345e` by one docs-only commit (`docs(pane-chrome)`), and
     `git diff --stat 8f069d0..68b345e -- src/ tests/ Cargo.toml` is empty, so every
     source-level measurement here still holds.

     Every check below was run against HEAD at planning time; its exit status and the
     relevant line of output are recorded with it. Six behavior checks (`c1`–`c6`) cover
     groups 1–4 and are RED, as they must be; groups 5 and 6 carry their own recorded checks
     of their own kinds. The two invariant checks are green with negative controls recorded.

     Every width literal in the snippets below is written unsuffixed. `scripts/gates/mdwidths.sh`
     requires each `#[test]` in `src/ui/markdown.rs` to name both 58 and 78 and finds them with
     `\b(\d+)\b`, which sees `78` but not `58u16` — measured:
     `python3 -c "import re;print(re.findall(r'\b(\d+)\b','[58u16, 78]'))"` -> `['78']`.
     A snippet lifted in with a suffix fails `make gates`.

     Baseline caveat: `cargo test --lib` is NOT green at HEAD. `ui::tests::wiring::*` fails
     5 of 29 on one run, 7 on the next, and 1 of 29 under `-- --test-threads=1` — timing
     dependent, pre-existing, and out of this change's scope (it touches no collaborator,
     thread or clock). Every "no regressions" step below means no new failure OUTSIDE
     `ui::tests::wiring::*`; re-run, or run that module serially, before treating one as
     yours. See design.md -> Test Strategy. -->

## 1. Soft breaks fold into the paragraph
<!-- kind: behavior -->

- [ ] 1.1 RED: Write failing tests for `A soft break starts a new line rather than being folded` — the fold at 58 and 78, the two-trailing-space hard-break carve-out still producing two lines, the sixty-word one-word-per-line source producing fewer lines at 78 than at 58, and a fenced block of `alpha`/`bravo` still producing two lines.

  Check, run at HEAD:

  ```rust
  // tests/zz_planning_probe.rs
  use herdr_openspec::ui::markdown::lines;
  fn t(s: &str, w: u16) -> Vec<String> { lines(s, w).iter().map(|l| l.text()).collect() }
  #[test] fn c1_softbreak_folds() {
      for w in [58, 78] {
          assert_eq!(t("first line\nsecond line", w), vec!["first line second line".to_string()], "w={w}");
      }
  }
  ```

  `cargo test --test zz_planning_probe` → exit **101**, `left: ["first line", "second line"] / right: ["first line second line"]`. RED because the behavior is missing, not because the harness is wrong: the same harness reports the current two-line output.

- [ ] 1.2 GREEN: Split `Event::SoftBreak | Event::HardBreak => f.group_break()` (`src/ui/markdown.rs:695`) into two arms — `SoftBreak` pushes a single-space `Run` into the current group, `HardBreak` keeps calling `group_break()` (per design.md → Decision 1). Nothing else moves; `groups` stays a `Vec<Vec<Run>>`.
- [ ] 1.3 GREEN: Re-baseline the one test the fold breaks. Measured with Decision 1 planted at HEAD: `cargo test --lib` → 1221 passed, **1** failed, `ui::markdown::tests::a_soft_break_starts_a_new_line`. Any *other* failure in this group is a real regression, not a baseline — and that heuristic is scoped to this group: `src/ui/driver.rs:904` builds its fixture as `(0..20).map(|i| format!("- line-{i:02}\n"))` and group 2 re-baselines it legitimately.
- [ ] 1.4 REFACTOR: Rename `groups`/`group_break` if "group" now reads as "source line" at its use sites; skip if it already reads as "hard-broken run" and say so.
- [ ] 1.5 Run `cargo test --lib ui::` — no regressions. (549 tests at HEAD by `cargo test --lib ui:: -- --list`) Every behavior group in this plan verifies with this one filter rather than a per-module list: the `Dashboard` tests that assert rendered markdown live in `src/ui/view.rs` and `src/ui/mod.rs` (reached as `ui::tests`), and a narrower filter silently skips them.

## 2. Bullet, quote, and thematic-break glyphs
<!-- kind: behavior -->

- [ ] 2.1 RED: Write failing tests for `Bullet items carry their marker and wrap under their text column`, `A nested list indents two columns per level`, `A block quote prefixes every one of its lines`, and `A thematic break fills the interior at both widths`.

  Checks, run at HEAD:

  ```rust
  #[test] fn c2_bullet_glyph() { for w in [58, 78] { assert!(t("- alpha", w)[0].starts_with("• ")); } }
  #[test] fn c3_quote_glyph()  { for w in [58, 78] { assert!(t("> quoted", w)[0].starts_with("│ ")); } }
  #[test] fn c4_rule_glyph()   { for w in [58, 78] { assert_eq!(t("a\n\n---\n\nb", w)[2], "─".repeat(w as usize)); } }
  ```

  All three → exit **101**. `c4` reports `left: "------…" / right: "──────…"` at w=58; `c2` and `c3` fail on the `starts_with`.

- [ ] 2.2 GREEN: Replace the bullet marker with `• `, the quote prefix with `│ ` and the nested one with `│ │ `, and the thematic break's fill character with `─`. The prefix widths are unchanged, so no wrap arithmetic moves.
- [ ] 2.3 REFACTOR: Lift the four glyphs to named `const`s in `src/ui/markdown.rs` if they are otherwise written as bare literals at their use sites; state that no refactor was needed if they already sit behind one name each.
- [ ] 2.4 GREEN: Re-baseline the **25** tests this group's two substitutions break, measured by planting them at HEAD and running `cargo test --lib`: `ui::view` 10, `ui::markdown` 6, `ui::app` 4, `ui::driver` 3, `ui::tests::detail` 2. (A raw run shows 28; three are `ui::tests::wiring::*` flakes — see the baseline caveat above.) Most share **one** cause: a detail fixture written `format!("- line-{i:02}\n")` and repeated across `src/ui/driver.rs`, `src/ui/app.rs`, `src/ui/mod.rs` and `src/ui/view.rs`, so the bullet change fails every site asserting it at once. The rest are the checklist literals at `src/ui/view.rs:4836` and `:4881`.
- [ ] 2.5 Run `cargo test --lib ui::` — no regressions; no refactor was needed beyond 2.3.

## 3. Table separators become box-drawing
<!-- kind: behavior -->

- [ ] 3.1 RED: Write failing tests for `A table that fits renders as aligned columns at both mandated widths`, `A ragged table renders every declared column and drops no header column`, and `A region too narrow for the pipe grammar renders one cell per line`.

  Check, run at HEAD:

  ```rust
  #[test] fn c5_table_glyph() {
      for w in [58, 78] {
          let l = t("| a | b |\n|---|---|\n| c | d |", w);
          assert!(l[0].starts_with('│') && l[1].starts_with('├'), "w={w} got {l:?}");
      }
  }
  ```

  → exit **101**; the row line still starts `|` and the delimiter line still starts `|`.

- [ ] 3.2 GREEN: Emit `│` at every row-line separator, and build the delimiter line as the row line's own shape with spaces and content columns replaced by `─` and the separators by `├`/`┼`/`┤` (per design.md → Decision 3). `total = 3n + 1 + sum(w)` and the `width < 4n + 1` fallback threshold do not change.
- [ ] 3.3 GREEN: Rewrite the two test helpers the separator change invalidates before touching any literal. `allocated_widths` (`src/ui/markdown.rs:2034`) does `.trim_matches('|').split('|')` and `columns(run) - 2`, neither valid against a `├─┼─┤` delimiter — trim `├`/`┤` and split on `┼`. `field`'s doc comment (`:2042-2045`) names "the `|` that closes it" and goes stale with it. `grep -c 'allocated_widths(' src/ui/markdown.rs` → **9** (one definition, eight call sites).
- [ ] 3.4 GREEN: Re-baseline every table literal, not only the worked example: the eight `allocated_widths` call sites, plus `a_narrow_region_renders_one_cell_per_line` and `unmodelled_constructs_render_as_source`, which assert a rendered `| Gate   | Runner    |` row directly. Assert `layout::columns` on the delimiter line equals that of a row line.
- [ ] 3.5 GREEN: Re-baseline `a_table_inside_a_container_carries_the_prefix_on_every_line` (`src/ui/markdown.rs:2526`), which group 2's quote prefix and this group's separators both hit. It backs the table requirement's "A table inside a container" paragraph, which carries no scenario of its own and so appears in no matrix row.
- [ ] 3.6 Run `cargo test --lib ui::` — no regressions; no refactor was needed beyond 3.3's helper rewrite.

## 4. Task-list items are modelled
<!-- kind: behavior -->

- [ ] 4.1 RED: Write failing tests for `A table renders as its literal source text, one row per line` in its narrowed form — the footnote still literal, and the task-list item, the table and the strikethrough span as discriminating controls.

  Check, run at HEAD:

  ```rust
  #[test] fn c6_tasklist_glyph() {
      for w in [58, 78] { assert_eq!(t("- [x] done", w), vec!["[✓] done".to_string()], "w={w}"); }
  }
  ```

  → exit **101**, `left: ["- [x] done"] / right: ["[✓] done"]`.

- [ ] 4.2 GREEN: Add `Options::ENABLE_TASKLISTS` **and** the `Event::TaskListMarker(checked)` arm in the same edit, rendering `[✓] `/`[ ] ` in place of the item's bullet marker, and **recompute `cont_prefix` from the new marker's `columns`**. `start_item` (`src/ui/markdown.rs:372-390`) derives `cont_prefix` from `columns(&marker)` before the checkbox marker is known, so an arm that rewrites `first_prefix` alone leaves continuations hanging at two columns instead of four. Landing the option without the arm instead makes the checkbox vanish into `fold`'s `_ => {}` wildcard, which 4.1's check catches.
- [ ] 4.3 GREEN: Update all four sites naming the option set, found by `grep -n 'ENABLE_TABLES' src/ui/markdown.rs` → `:630`, `:634`, `:642`, `:2311`. `:642` is production and `:2311` is inside `a_ragged_table_keeps_its_declared_columns`, whose event-stream assertion design.md → Test Boundaries relies on; leaving `:2311` behind makes that test parse under a different option set than production, with nothing reporting it.
- [ ] 4.4 CHECK: Confirm `DEPS` still passes — parser options are runtime values, not Cargo features, so `pulldown-cmark`'s `default-features = false` and `features = []` are untouched. Run `/bin/sh scripts/gates/deps.sh`; expect the leg 2b and leg 2d lines to report OK.
- [ ] 4.5 Run `cargo test --lib ui::` — no regressions; no refactor was needed. This group breaks `src/ui/view.rs:4836` and `:4881` a second time — group 2 made them `• [x] …`, this group makes them `[✓] …`.

## 5. The checklist adopts the shared glyph
<!-- kind: behavior -->

- [ ] 5.0 RED: Record group 5's check. `cargo test --lib ui::view::tests::tasks_tab_shows_checkboxes` at HEAD → exit **0** (the test asserts today's `[x]`); after 5.1 rewrites its literals to `[✓]` it must be RED before 5.2 lands the glyph. This group's evidence is the rewritten assertion going red against unchanged production code, not a new test.
- [ ] 5.1 RED: **Rewrite the existing tests**, do not write new ones, for `The tasks tab shows checkboxes and its siblings show markdown` and `The tab is chosen by tracks_tasks, not by its id` — they are `ui::view::tests::tasks_tab_shows_checkboxes` and `ui::view::tests::tasks_tab_chosen_by_flag` in `src/ui/view.rs`, and `ui::tests`' assertion at `src/ui/mod.rs:1289` moves with them. Then write failing tests for `A schema naming no tasks artifact leaves every tab as markdown`, `Groups, headings, items, and separators at both mandated widths`, `A nested item reproduces its own indent`, `The indent is dropped whole as the width collapses`, `A headingless leading group renders without a heading line`, and `A checklist of wide-character items fits at both mandated widths`. The first three assert the two paths render the **same** glyph and that the absent progress-bar row is what now names the path taken.
- [ ] 5.2 GREEN: Change the item glyph in `src/ui/tasks.rs` from `[x]` to `[✓]` (per design.md → Decision 6). `✓` is one column, so `prefix_len` stays 4 and no wrap or indent arithmetic changes.
- [ ] 5.3 GREEN: Re-baseline the four rendered-glyph literals outside `ui::tasks` — `src/ui/view.rs:4818`, `:4836`, `:4873`, `:4881` and the `starts_with` assertion at `src/ui/mod.rs:1289`. Found by `grep -rn '\[x\]' src/ tests/ | grep -E '"\[x\]|\[x\] '`, which lists seventeen sites of which these five and the eight in `src/ui/tasks.rs` are rendered output; the `src/ui/list.rs` hits are a `text[x]` format string.
- [ ] 5.4 Run `cargo test --lib ui::` — no regressions; no refactor was needed. (549 tests at HEAD by `cargo test --lib ui:: -- --list`)

## 6. The glyph set's width is asserted
<!-- kind: behavior -->

- [ ] 6.0 CHECK: Extend the two sweep scenarios with task-list inputs — `No line exceeds the width it was given` gains a checked and an unchecked task-list item and one nested in a block quote; `Rendering is total over arbitrary input` gains a 500-character task-list token, a bare `- [x]` with no text, and an item nested three quote levels deep. Labelled CHECK, not RED: the fixture at `src/ui/markdown.rs:1246-1256` already sweeps a nested bullet whose prefix is also four columns, so the `saturating_sub` path these inputs take already passes. They widen the input set over a guarded path rather than reaching a new one.
- [ ] 6.1 CHECK: Write the test for `Each emitted glyph measures one column, and the set is complete` — `layout::columns` returns 1 for each of `•│─├┼┤✓`, no line of an all-constructs document exceeds 58 or 78, and the set of non-space characters present in the rendering but absent from its source is exactly those seven.
- [ ] 6.2 GREEN: Extend the existing `composite_fixture()` (`src/ui/markdown.rs:1211`) with a nested block quote and a checked and unchecked task-list item, rather than adding a second all-constructs document beside it. It already covers the heading, paragraph, bullet list, nested list, fenced code, block quote, thematic break, link and three-column table this scenario needs, and its comment records why its widths were chosen. No production code changes in this group.
- [ ] 6.3 CHECK: Give 6.0 and 6.1 their negative controls — both are green on first run when reached in plan order, because groups 1–4 have already landed what they pin. For 6.1, plant a two-column glyph (replace the bullet `•` with `🎉`) and confirm failure on both the `layout::columns == 1` leg and the set-difference leg. For 6.0, plant a `cont_prefix` that is not recomputed from the checkbox marker and confirm the wrapped-item clause goes red while both sweeps stay green — which is the point: it demonstrates that the sweeps could not have caught it. Remove both plants and confirm each goes quiet.
- [ ] 6.4 CHANGE: Raise `MD_MIN`'s default in `scripts/gates/mdwidths.sh:21` from 34 to 35. `grep -c '#\[test\]' src/ui/markdown.rs` → **34** at HEAD and the default is exactly 34, so the floor sits at its true measured value; this group adds the 35th. Per CLAUDE.md a gate's floor is its own script default, and nothing fails if it is left behind.
- [ ] 6.5 VERIFY: Confirm every new and re-baselined `#[test]` in `src/ui/markdown.rs` names both `58` and `78` unsuffixed. `/bin/sh scripts/gates/mdwidths.sh` → expect `MDWIDTHS OK: all 35 markdown tests name both 58 and 78`.
- [ ] 6.6 Run `cargo test --lib ui::` — no regressions; no refactor was needed.

## 7. The degraded-states row narrows
<!-- kind: operational -->

- [ ] 7.1 CHECK: Confirm every stale `SPEC.md` claim before editing, with a check that does not miss a line-wrapped one. `grep -c 'task-list item' SPEC.md` → **2** at HEAD (`:509` inside the § Detail view paragraph, where the phrase is split across a newline and so is invisible to a grep for the whole phrase, and `:897` in the degraded-states table). `grep -n 'a footnote, a task-list item' SPEC.md tests/degraded-coverage.toml` → exit **0** but only two matches, `SPEC.md:897` and `tests/degraded-coverage.toml:95`, which is why the first form is the one to use. After 7.2 and 7.3 both must find nothing.
- [ ] 7.2 CHANGE: Reword the row in `SPEC.md` and the `condition` in `tests/degraded-coverage.toml` from "a footnote, a task-list item" to "a footnote", and update that entry's `why` to name the task-list item as a control rather than as a literal-rendering case.
- [ ] 7.3 CHANGE: Rewrite `SPEC.md` § Detail view's markdown-grammar paragraph (`:485-510`) for the five claims this change reverses: the `[x]`/`[ ]` glyph (`:487`), the soft break starting a new rendered line (`:491`), the block quote's `> ` prefix (`:498`), the table's leading and closing `|` (`:500-503`), and the task-list item rendering as literal source (`:508-509`). `CLAUDE.md` makes `SPEC.md` the winner where prose and spec disagree, and no gate binds this paragraph, so leaving it stale would leave the authority asserting the opposite of three delta specs.
- [ ] 7.4 CHANGE: Add the widened Ambiguous-width exposure to `SPEC.md` beside its existing paragraph on the class (`:372`), naming the seven glyphs and that no compensation is made (per design.md → Decision 4).
- [ ] 7.5 CHANGE: Rewrite the stale `## Purpose` in `openspec/specs/markdown-render/spec.md` ("soft breaks that preserve the author's line structure"; "footnotes, task-list items — render as their literal source") and in `openspec/specs/tasks-checklist/spec.md` ("one `[x]`/`[ ]` glyph line per item") at archive time. Delta specs carry no Purpose and `tests/spec_purposes.rs` rejects only an empty or placeholder one, so a stale Purpose passes every gate.
- [ ] 7.6 VERIFY: Run `cargo test --test degraded_coverage && cargo test --test doc_contract` — both green. These bind the prose to its computable site, so a row edited in one file and not the other fails here. Note the limit of that cover: `doc_contract` binds the module map, tested-modules list, worker-thread count, MSRV, gate paths, manifest transcription, injected context, mouse bindings and terminal-seam names — **not** § Detail view's grammar paragraph, so 7.3 is verified by review, not by a gate.

## 8. Change Review
<!-- kind: operational -->

- [ ] 8.1 CHECK: Dispatch an independent reviewer — not a fork of the implementing session — against proposal.md, all three delta specs, design.md, and the diff. Concentration points for this change: that each re-baselined literal was changed because the behavior changed and not to match a wrong implementation; that `ENABLE_TASKLISTS` and its render arm landed together; that the two checkbox renderers are asserted to agree rather than assumed to; and that no test asserts a glyph constant the implementation also declares.
- [ ] 8.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a one-line reason, note SUGGESTIONs, and re-run affected tests.
- [ ] 8.3 VERIFY: Confirm no blocking or unowned finding remains.

## 9. Documentation
<!-- kind: operational -->

- [ ] 9.1 Rewrite in `AGENTS.md`: the `pulldown_cmark` seam rule. **`CLAUDE.md` is a symlink to `AGENTS.md`** — edit the target with `Edit`, never `Write` through the link, which would replace the symlink with a regular file. (audience: any future session touching `src/ui/markdown.rs`). It currently states the option set is exactly `ENABLE_TABLES | ENABLE_STRIKETHROUGH` and warns that a further flag makes its construct vanish into `fold`'s wildcard. Correct the set to include `ENABLE_TASKLISTS` and keep the warning, which this change is the worked example of — it is why the option and its arm are one task.
- [ ] 9.2 Rewrite in `AGENTS.md` (`:352` and its section): the "Current repo state" sentence describing markdown rendering (audience: same). Replace the claim that soft breaks preserve source line structure with the fold-plus-hard-break rule, and name the glyph set's Ambiguous-width caveat in one clause pointing at `SPEC.md`. Net effect is a rewrite, not an addition — nothing new is appended and two now-false claims are removed.

## 10. Lint & Verify
<!-- kind: operational -->

- [ ] 10.1 CHECK: Commit the change directory before running `make gates`. At HEAD `/bin/sh scripts/gates/openspec-untouched.sh` exits **1** on this change's own untracked files (`OPENSPEC-UNTOUCHED FAIL: an untracked file exists inside openspec/`), which is the gate working, not a defect.
- [ ] 10.2 VERIFY: Run `make check` as the single gate. If it fails, name the failing sub-command — `make fmt-check`, `make lint`, `make gates`, `make test`, or `make coverage` — rather than reporting the composite. Expect `ui::tests::wiring::*` failures that are not this change's: re-run to confirm they vary, and report them separately rather than folding them into this change's result.
- [ ] 10.3 VERIFY: Confirm `COLWIDTH` still passes and can still fail. `/bin/sh scripts/gates/colwidth.sh` at HEAD → exit **0**, `COLWIDTH OK: no char-count measurement in the eight pure view files`. Negative control, run at planning time: inserting `let _planted = source.chars().count();` into `lines` makes it exit **1** with `COLWIDTH FAIL: src/ui/markdown.rs has 1 char-count measurement(s) in production code`; removing the plant returns it to exit **0**.
- [ ] 10.4 VERIFY: Read 10.2's coverage output and confirm both floors held — the total and the production slice. `Makefile:71` is `check: fmt-check lint gates test coverage`, so `make check` already ran it; re-running it here would be duplicate work, and the assertion is on its output. This change adds tests and no untested production branch, so a fall means a re-baselined test stopped exercising a path.
- [ ] 10.5 VERIFY: Run `openspec validate markdown-legibility --strict` — valid.
