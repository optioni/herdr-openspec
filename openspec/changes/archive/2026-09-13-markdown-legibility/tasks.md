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

     Baseline: `cargo test --lib` at HEAD is 1222 passed / 0 failed, and
     `cargo test --lib ui::` is 549 — but only on an unloaded machine. Under a competing
     cargo run the wall-clock-deadlined tests in `ui::tests::wiring::*` fail
     non-deterministically (measured here: 7, 11 and 10 of 29 on three runs taking 35-51s,
     against 3.8s idle).

     How to tell a flake from your own regression: a deadline flake is confined to
     `ui::tests::wiring::*` AND its message asserts a count against 0 with an EMPTY log
     (`left: 0 / right: 4`, `calls: []`) — the run was cut short, not mis-wired. Re-run
     `cargo test --lib ui::tests::wiring` alone (~4s idle) to settle it. Do not chase one
     into a re-baseline; that is exactly the "corrected a literal that was right" failure
     mode 8.1 watches for. Pre-existing and out of scope. See design.md -> Test Strategy. -->

## 1. Soft breaks fold into the paragraph
<!-- kind: behavior -->

- [x] 1.1 RED: Write failing tests for `A soft break starts a new line rather than being folded` — the fold at 58 and 78, the two-trailing-space hard-break carve-out still producing two lines, the sixty-word one-word-per-line source producing fewer lines at 78 than at 58, and a fenced block of `alpha`/`bravo` still producing two lines.

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

- [x] 1.2 GREEN: Split `Event::SoftBreak | Event::HardBreak => f.group_break()` (`src/ui/markdown.rs:695`) into two arms — `SoftBreak` pushes a single-space `Run` into the current group, `HardBreak` keeps calling `group_break()` (per design.md → Decision 1). Nothing else moves; `groups` stays a `Vec<Vec<Run>>`.
- [x] 1.3 GREEN: Re-baseline the one test the fold breaks. **Measured against this tree, not the plan's HEAD:** the fold breaks **two**, not one — `ui::markdown::tests::a_soft_break_starts_a_new_line` (rewritten in 1.1) and `ui::driver::tests::a_body_click_moves_the_cursor_and_folds_nothing`, which `foldable-spec-sections` added after this plan was measured. Its fixture is the only `format!("line-{i:02}\n")` in the crate without a bullet — a five-line paragraph, which now folds to one rendered line — and it is re-baselined by making it a bullet list, the shape every sibling fixture already uses, which restores its five body lines and leaves every asserted index unchanged. Measured with Decision 1 planted at HEAD: `cargo test --lib` → 1221 passed, **1** failed, `ui::markdown::tests::a_soft_break_starts_a_new_line`. Any *other* failure in this group is a real regression, not a baseline — and that heuristic is scoped to this group: `src/ui/driver.rs:904` builds its fixture as `(0..20).map(|i| format!("- line-{i:02}\n"))` and group 2 re-baselines it legitimately.
- [x] 1.4 REFACTOR: Rename `groups`/`group_break` if "group" now reads as "source line" at its use sites; skip if it already reads as "hard-broken run" and say so. **Done as a comment correction, not a rename:** the names already read as "hard-broken run" at every use site (`push_verbatim`, `finish`, and the new `HardBreak` arm), but `Block`'s doc comment and `group_break`'s both named the soft break as the boundary and were false. Both corrected in place.
- [x] 1.5 Run `cargo test --lib ui::` — no regressions. (549 tests at HEAD by `cargo test --lib ui:: -- --list`) Every behavior group in this plan verifies with this one filter rather than a per-module list: the `Dashboard` tests that assert rendered markdown live in `src/ui/view.rs` and `src/ui/mod.rs` (reached as `ui::tests`), and a narrower filter silently skips them.

## 2. Bullet, quote, and thematic-break glyphs
<!-- kind: behavior -->

- [x] 2.1 RED: Write failing tests for `Bullet items carry their marker and wrap under their text column`, `A nested list indents two columns per level`, `A block quote prefixes every one of its lines`, and `A thematic break fills the interior at both widths`.

  Checks, run at HEAD:

  ```rust
  #[test] fn c2_bullet_glyph() { for w in [58, 78] { assert!(t("- alpha", w)[0].starts_with("• ")); } }
  #[test] fn c3_quote_glyph()  { for w in [58, 78] { assert!(t("> quoted", w)[0].starts_with("│ ")); } }
  #[test] fn c4_rule_glyph()   { for w in [58, 78] { assert_eq!(t("a\n\n---\n\nb", w)[2], "─".repeat(w as usize)); } }
  ```

  All three → exit **101**. `c4` reports `left: "------…" / right: "──────…"` at w=58; `c2` and `c3` fail on the `starts_with`.

- [x] 2.2 GREEN: Replace the bullet marker with `• `, the quote prefix with `│ ` and the nested one with `│ │ `, and the thematic break's fill character with `─`. The prefix widths are unchanged, so no wrap arithmetic moves.
- [x] 2.3 REFACTOR: Lift the four glyphs to named `const`s in `src/ui/markdown.rs` if they are otherwise written as bare literals at their use sites; state that no refactor was needed if they already sit behind one name each.
- [x] 2.4 GREEN: **Measured 22 against this tree, not 25:** `ui::view` 10, `ui::markdown` 5, `ui::app` 4, `ui::driver` 3 (one of them re-baselined in group 1 already), `ui::tests::detail` 2 — the plan's count was taken before `foldable-spec-sections` landed. The shared `format!("- line-{i:02}\n")` fixture is left alone: it is *source*, and only the rendered literals asserted against it move. Two further sites the plan assigned elsewhere are touched here because this group's substitutions reach them first: `unmodelled_constructs_render_as_source`'s task-list entry gains an explicit expected-lines column (group 4 narrows the entry out entirely), and the four rendered checkbox literals in `ui::view::tests::*` become `• [x]`/`• [ ]` (group 4 makes them `[✓]`/`[ ]`), exactly the double-touch 4.5 already anticipates. Originally: Re-baseline the **25** tests this group's two substitutions break, measured by planting them at HEAD and running `cargo test --lib`: `ui::view` 10, `ui::markdown` 6, `ui::app` 4, `ui::driver` 3, `ui::tests::detail` 2. (A raw run shows 28; three are `ui::tests::wiring::*` flakes — see the baseline caveat above.) Most share **one** cause: a detail fixture written `format!("- line-{i:02}\n")` and repeated across `src/ui/driver.rs`, `src/ui/app.rs`, `src/ui/mod.rs` and `src/ui/view.rs`, so the bullet change fails every site asserting it at once. The rest are the checklist literals at `src/ui/view.rs:4836` and `:4881`.
- [x] 2.5 Run `cargo test --lib ui::` — no regressions; no refactor was needed beyond 2.3.

## 3. Table separators become box-drawing
<!-- kind: behavior -->

- [x] 3.1 RED: Write failing tests for `A table that fits renders as aligned columns at both mandated widths`, `A ragged table renders every declared column and drops no header column`, and `A region too narrow for the pipe grammar renders one cell per line`.

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

- [x] 3.2 GREEN: Emit `│` at every row-line separator, and build the delimiter line as the row line's own shape with spaces and content columns replaced by `─` and the separators by `├`/`┼`/`┤` (per design.md → Decision 3). `total = 3n + 1 + sum(w)` and the `width < 4n + 1` fallback threshold do not change.
- [x] 3.3 GREEN: Rewrite the two test helpers the separator change invalidates before touching any literal. `allocated_widths` (`src/ui/markdown.rs:2034`) does `.trim_matches('|').split('|')` and `columns(run) - 2`, neither valid against a `├─┼─┤` delimiter — trim `├`/`┤` and split on `┼`. `field`'s doc comment (`:2042-2045`) names "the `|` that closes it" and goes stale with it. `grep -c 'allocated_widths(' src/ui/markdown.rs` → **9** (one definition, eight call sites).
- [x] 3.4 GREEN: **Two further sites the plan did not name**, both in `src/ui/view.rs` and both found by running rather than by grepping: `a_table_reaches_the_buffer_aligned`'s `pipe_offsets` helper and its delimiter-line predicate, and `content_never_overwrites_the_detail_region_s_border`'s three `contains('|')` probes. `pipe_offsets` now matches all four separator glyphs, which is what keeps the delimiter-to-row-line offset equality the alignment assertion. Originally: Re-baseline every table literal, not only the worked example: the eight `allocated_widths` call sites, plus `a_narrow_region_renders_one_cell_per_line` and `unmodelled_constructs_render_as_source`, which assert a rendered `| Gate   | Runner    |` row directly. Assert `layout::columns` on the delimiter line equals that of a row line.
- [x] 3.5 GREEN: Re-baseline `a_table_inside_a_container_carries_the_prefix_on_every_line` (`src/ui/markdown.rs:2526`), which group 2's quote prefix and this group's separators both hit. It backs the table requirement's "A table inside a container" paragraph, which carries no scenario of its own and so appears in no matrix row.
- [x] 3.6 Run `cargo test --lib ui::` — no regressions; no refactor was needed beyond 3.3's helper rewrite.

## 4. Task-list items are modelled
<!-- kind: behavior -->

- [x] 4.1 RED: Write failing tests for `A table renders as its literal source text, one row per line` in its narrowed form — the footnote still literal, and the task-list item, the table and the strikethrough span as discriminating controls.

  Check, run at HEAD:

  ```rust
  #[test] fn c6_tasklist_glyph() {
      for w in [58, 78] { assert_eq!(t("- [x] done", w), vec!["[✓] done".to_string()], "w={w}"); }
  }
  ```

  → exit **101**, `left: ["- [x] done"] / right: ["[✓] done"]`.

- [x] 4.2 GREEN: Add `Options::ENABLE_TASKLISTS` **and** the `Event::TaskListMarker(checked)` arm in the same edit, rendering `[✓] `/`[ ] ` in place of the item's bullet marker, and **recompute `cont_prefix` from the new marker's `columns`**. `start_item` (`src/ui/markdown.rs:372-390`) derives `cont_prefix` from `columns(&marker)` before the checkbox marker is known, so an arm that rewrites `first_prefix` alone leaves continuations hanging at two columns instead of four. Landing the option without the arm instead makes the checkbox vanish into `fold`'s `_ => {}` wildcard, which 4.1's check catches.
- [x] 4.3 GREEN: Update all four sites naming the option set, found by `grep -n 'ENABLE_TABLES' src/ui/markdown.rs` → `:630`, `:634`, `:642`, `:2311`. `:642` is production and `:2311` is inside `a_ragged_table_keeps_its_declared_columns`, whose event-stream assertion design.md → Test Boundaries relies on; leaving `:2311` behind makes that test parse under a different option set than production, with nothing reporting it.
- [x] 4.4 CHECK: Confirm `DEPS` still passes — parser options are runtime values, not Cargo features, so `pulldown-cmark`'s `default-features = false` and `features = []` are untouched. Run `/bin/sh scripts/gates/deps.sh`; expect the leg 2b and leg 2d lines to report OK.
- [x] 4.5 Run `cargo test --lib ui::` — no regressions; no refactor was needed. This group breaks `src/ui/view.rs:4836` and `:4881` a second time — group 2 made them `• [x] …`, this group makes them `[✓] …`. **Groups 4 and 5 are committed together.** The narrowing scenario's own cross-check — the same task-list source through `ui::tasks::lines`, asserted to render the **same** `[✓]` — is the structural answer to the drift `markdown-render` names, and it cannot pass until group 5 has moved `ui::tasks`' glyph. Splitting the commit would leave one assertion red in between.

## 5. The checklist adopts the shared glyph
<!-- kind: behavior -->

- [x] 5.0 RED: Record group 5's check. `cargo test --lib ui::view::tests::tasks_tab_shows_checkboxes` at HEAD → exit **0** (the test asserts today's `[x]`); after 5.1 rewrites its literals to `[✓]` it must be RED before 5.2 lands the glyph. This group's evidence is the rewritten assertion going red against unchanged production code, not a new test.
- [x] 5.1 RED: **Rewrite the existing tests**, do not write new ones, for `The tasks tab shows checkboxes and its siblings show markdown` and `The tab is chosen by tracks_tasks, not by its id` — they are `ui::view::tests::tasks_tab_shows_checkboxes` and `ui::view::tests::tasks_tab_chosen_by_flag` in `src/ui/view.rs`, and `ui::tests`' assertion at `src/ui/mod.rs:1289` moves with them. Then write failing tests for `A schema naming no tasks artifact leaves every tab as markdown`, `Groups, headings, items, and separators at both mandated widths`, `A nested item reproduces its own indent`, `The indent is dropped whole as the width collapses`, `A headingless leading group renders without a heading line`, and `A checklist of wide-character items fits at both mandated widths`. The first three assert the two paths render the **same** glyph and that the absent progress-bar row is what now names the path taken.
- [x] 5.2 GREEN: Change the item glyph in `src/ui/tasks.rs` from `[x]` to `[✓]` (per design.md → Decision 6). `✓` is one column, so `prefix_len` stays 4 and no wrap or indent arithmetic changes.
- [x] 5.3 GREEN: **Two `tests/degraded-coverage.toml` `covers` pointers moved too**, a mechanical consequence of the two production edits: `src/ui/markdown.rs:684` -> `:695` (the option-set line, pushed down by group 4's doc-comment rewrite) and `src/ui/tasks.rs:318-324` -> `:324-330` (the `No tasks yet` guard, pushed down by 5.2's comment). Both are line-number corrections, not the row narrowing 7.2 makes. Originally: Re-baseline the four rendered-glyph literals outside `ui::tasks` — `src/ui/view.rs:4818`, `:4836`, `:4873`, `:4881` and the `starts_with` assertion at `src/ui/mod.rs:1289`. Found by `grep -rn '\[x\]' src/ tests/ | grep -E '"\[x\]|\[x\] '`, which lists seventeen sites of which these five and the eight in `src/ui/tasks.rs` are rendered output; the `src/ui/list.rs` hits are a `text[x]` format string.
- [x] 5.4 Run `cargo test --lib ui::` — no regressions; no refactor was needed. (549 tests at HEAD by `cargo test --lib ui:: -- --list`)

## 6. The glyph set's width is asserted
<!-- kind: behavior -->

- [x] 6.0 CHECK: Extend the two sweep scenarios with task-list inputs — `No line exceeds the width it was given` gains a checked and an unchecked task-list item and one nested in a block quote; `Rendering is total over arbitrary input` gains a 500-character task-list token, a bare `- [x]` with no text, and an item nested three quote levels deep. Labelled CHECK, not RED: the fixture at `src/ui/markdown.rs:1246-1256` already sweeps a nested bullet whose prefix is also four columns, so the `saturating_sub` path these inputs take already passes. They widen the input set over a guarded path rather than reaching a new one.
- [x] 6.1 CHECK: Write the test for `Each emitted glyph measures one column, and the set is complete` — `layout::columns` returns 1 for each of `•│─├┼┤✓`, no line of an all-constructs document exceeds 58 or 78, and the set of non-space characters present in the rendering but absent from its source is exactly those seven.
- [x] 6.2 GREEN: Extend the existing `composite_fixture()` (`src/ui/markdown.rs:1211`) with a nested block quote and a checked and unchecked task-list item, rather than adding a second all-constructs document beside it. It already covers the heading, paragraph, bullet list, nested list, fenced code, block quote, thematic break, link and three-column table this scenario needs, and its comment records why its widths were chosen. No production code changes in this group.
- [x] 6.3 CHECK: **Both plants measured.** Plant A (`BULLET` -> `🎉 `) fires the width leg first (`emitted '🎉': left 2 / right 1`) and, with that leg removed, the set-difference leg (`{…, '🎉'} != {'•', …}`) — the width leg exists because it measures what is actually **emitted**, not only the seven names the test declares, which is what makes a glyph added later without an assertion fail here. Plant B (`set_task_marker` rewriting `first_prefix` alone) fires **only** the wrapped-item clause (`continuation "  india …" is not four columns in`) while `no_line_exceeds_the_width_it_was_given`, `lines_is_total_over_arbitrary_input` and the glyph test all stay green — which is the point: the sweeps could not have caught it. Both plants removed and each confirmed quiet. Originally: Give 6.0 and 6.1 their negative controls — both are green on first run when reached in plan order, because groups 1–4 have already landed what they pin. For 6.1, plant a two-column glyph (replace the bullet `•` with `🎉`) and confirm failure on both the `layout::columns == 1` leg and the set-difference leg. For 6.0, plant a `cont_prefix` that is not recomputed from the checkbox marker and confirm the wrapped-item clause goes red while both sweeps stay green — which is the point: it demonstrates that the sweeps could not have caught it. Remove both plants and confirm each goes quiet.
- [x] 6.4 CHANGE: Raise `MD_MIN`'s default in `scripts/gates/mdwidths.sh:21` from 34 to 35. `grep -c '#\[test\]' src/ui/markdown.rs` → **34** at HEAD and the default is exactly 34, so the floor sits at its true measured value; this group adds the 35th. Per CLAUDE.md a gate's floor is its own script default, and nothing fails if it is left behind.
- [x] 6.5 VERIFY: Confirm every new and re-baselined `#[test]` in `src/ui/markdown.rs` names both `58` and `78` unsuffixed. `/bin/sh scripts/gates/mdwidths.sh` → expect `MDWIDTHS OK: all 35 markdown tests name both 58 and 78`.
- [x] 6.6 Run `cargo test --lib ui::` — no regressions; no refactor was needed.

## 7. The degraded-states row narrows
<!-- kind: operational -->

- [x] 7.1 CHECK: Confirm every stale `SPEC.md` claim before editing, with a check that does not miss a line-wrapped one. `grep -c 'task-list item' SPEC.md` → **2** at HEAD (`:509` inside the § Detail view paragraph, where the phrase is split across a newline and so is invisible to a grep for the whole phrase, and `:897` in the degraded-states table). `grep -n 'a footnote, a task-list item' SPEC.md tests/degraded-coverage.toml` → exit **0** but only two matches, `SPEC.md:897` and `tests/degraded-coverage.toml:95`, which is why the first form is the one to use. After 7.2 and 7.3 both must find nothing.
- [x] 7.2 CHANGE: Reword the row in `SPEC.md` and the `condition` in `tests/degraded-coverage.toml` from "a footnote, a task-list item" to "a footnote", and update that entry's `why` to name the task-list item as a control rather than as a literal-rendering case.
- [x] 7.3 CHANGE: Rewrite `SPEC.md` § Detail view's markdown-grammar paragraph (`:485-510`) for the five claims this change reverses: the `[x]`/`[ ]` glyph (`:487`), the soft break starting a new rendered line (`:491`), the block quote's `> ` prefix (`:498`), the table's leading and closing `|` (`:500-503`), and the task-list item rendering as literal source (`:508-509`). `CLAUDE.md` makes `SPEC.md` the winner where prose and spec disagree, and no gate binds this paragraph, so leaving it stale would leave the authority asserting the opposite of three delta specs.
- [x] 7.4 CHANGE: Add the widened Ambiguous-width exposure to `SPEC.md` beside its existing paragraph on the class (`:372`), naming the seven glyphs and that no compensation is made (per design.md → Decision 4).
- [x] 7.5 CHANGE: **Done at archive time, as specified.** All three phrases are gone: `grep -n "soft breaks that preserve\|footnotes, task-list items\|one \`[x]\`" openspec/specs/markdown-render/spec.md openspec/specs/tasks-checklist/spec.md` -> no match. `markdown-render`'s Purpose now states the fold-plus-hard-break rule, the modelled task-list item and the one-column glyph assertion; `tasks-checklist`'s states `[✓]` and that the two renderers agree by construction. `cargo test --test spec_purposes` green. Originally: **DO THIS AS PART OF `openspec archive`, NOT BEFORE — and nothing will catch it if the archive step forgets** (Change Review WARNING). `tests/spec_purposes.rs` rejects only an empty or placeholder Purpose, so all three phrases below pass every gate while being false. Grep for them verbatim rather than re-reading the sections:

  | File | Line | Stale phrase | Now reads |
  |---|---|---|---|
  | `openspec/specs/markdown-render/spec.md` | 11 | `soft breaks that preserve the author's line structure` | a soft break folds into a space; a hard break still breaks |
  | `openspec/specs/markdown-render/spec.md` | 15 | `footnotes, task-list items` | only footnotes remain unmodelled |
  | `openspec/specs/tasks-checklist/spec.md` | 7 | `one \`[x]\`/\`[ ]\` glyph line per item` | `[✓]`/`[ ]` |

  **Deferred to the archive step by its own wording** — the two `## Purpose` sections live in `openspec/specs/`, which `openspec archive` never rewrites from a delta (a delta carries no Purpose), and editing them before the change is archived would put the main specs ahead of the archive. Do this as part of running `openspec archive`, not before. Rewrite the stale `## Purpose` in `openspec/specs/markdown-render/spec.md` ("soft breaks that preserve the author's line structure"; "footnotes, task-list items — render as their literal source") and in `openspec/specs/tasks-checklist/spec.md` ("one `[x]`/`[ ]` glyph line per item") at archive time. Delta specs carry no Purpose and `tests/spec_purposes.rs` rejects only an empty or placeholder one, so a stale Purpose passes every gate.
- [x] 7.6 VERIFY: Run `cargo test --test degraded_coverage && cargo test --test doc_contract` — both green. These bind the prose to its computable site, so a row edited in one file and not the other fails here. Note the limit of that cover: `doc_contract` binds the module map, tested-modules list, worker-thread count, MSRV, gate paths, manifest transcription, injected context, mouse bindings and terminal-seam names — **not** § Detail view's grammar paragraph, so 7.3 is verified by review, not by a gate.

## 8. Change Review
<!-- kind: operational -->

- [x] 8.1 CHECK: Dispatch an independent reviewer — not a fork of the implementing session — against proposal.md, all three delta specs, design.md, and the diff. Concentration points for this change: that each re-baselined literal was changed because the behavior changed and not to match a wrong implementation; that `ENABLE_TASKLISTS` and its render arm landed together; that the two checkbox renderers are asserted to agree rather than assumed to; and that no test asserts a glyph constant the implementation also declares.
- [x] 8.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a one-line reason, note SUGGESTIONs, and re-run affected tests.
- [x] 8.3 VERIFY: Confirm no blocking or unowned finding remains.

  **Outcome: 1 CRITICAL, 1 WARNING, 6 SUGGESTIONs. All eight closed; none deferred unowned.**
  The reviewer was an independent `outside-in-tdd-reviewer`, not a fork of this session, and re-ran every gate, all six contract binaries, `cargo llvm-cov` (exit 0, 95.86% lines) and `openspec validate --strict` against `fca4403`. It verified all four concentration points and every recorded plan deviation.

  - **CRITICAL — the `tasks-checklist` delta would have reverted four `foldable-spec-sections` corrections on archive.** It was written against `43de01e` and never rebased: three `whose \`detail.source\` is` (there is no `source` field — `Detail` carries `sections: Vec<ArtifactSection>`) and one `the tab bar in row 3` (the archived spec and the bound test both say row 2). Confirmed by `git diff 43de01e..HEAD -- openspec/specs/tasks-checklist/spec.md`, which shows commit `92c0b6b` making exactly those four edits. Fixed in the delta; the requirement now differs from the archived one only where this change actually changes it. The reviewer bounded the blast radius: the other two deltas have an empty `git diff 43de01e..HEAD` against their capabilities and needed no rebase.
  - **WARNING — 7.5 is invisible to every gate.** Accepted as-is (the deferral reasoning is correct) and mitigated: 7.5 now carries a table of the three stale phrases quoted verbatim with their file and line, so the archive step greps rather than re-reads.
  - **SUGGESTION 1** — the delta's ADDED requirement said "six of the eight"; `│` is enumerated twice. Reworded to say so and to give the count as seven.
  - **SUGGESTION 2** — `start_item`'s doc comment still named `- ` as the marker. Now names [`BULLET`].
  - **SUGGESTION 3** — `set_task_marker`'s `category != Item` early return is an untested path whose effect would be the exact silent vanish this change prevents. The reason it cannot fire — `start_paragraph` deliberately preserves `Category::Item` — is now written at the guard rather than inferred from another function.
  - **SUGGESTION 4** — `seps(text, ['│', '│', '│'])` read as a copy-paste error. The closure takes `&[char]`; the row-line call passes `&['│']`.
  - **SUGGESTION 5** — `design.md`'s matrix promised the wide-character checklist asserts the glyph's interior column offsets against the ASCII case, and nothing did (the clause was unasserted before this change too). **Fixed by asserting it rather than by correcting the design row**, which also closes a real gap in 5.1. The first attempt was a check that could not fail — both fixtures were unindented, so every offset was 0 — and was replaced: both sources now carry the same three indents, and the assertion pins the column the glyph begins at **and** the column its text begins at, so a wider glyph moves the second. Negative control measured: planting `[🎉]` for `[✓]` gives `left: [(2, 6), (4, 8)] / right: [(0, 4), (2, 6), (4, 8)]`; removing it goes quiet.
  - **SUGGESTION 6** — `tests/degraded-coverage.toml`'s `why` said "the same fixture's" for what are three separate fixtures. Reworded.

## 9. Documentation
<!-- kind: operational -->

- [x] 9.1 Rewrite in `AGENTS.md`: the `pulldown_cmark` seam rule. **`CLAUDE.md` is a symlink to `AGENTS.md`** — edit the target with `Edit`, never `Write` through the link, which would replace the symlink with a regular file. (audience: any future session touching `src/ui/markdown.rs`). It currently states the option set is exactly `ENABLE_TABLES | ENABLE_STRIKETHROUGH` and warns that a further flag makes its construct vanish into `fold`'s wildcard. Correct the set to include `ENABLE_TASKLISTS` and keep the warning, which this change is the worked example of — it is why the option and its arm are one task.
- [x] 9.2 **The claim this task names is not in the file.** `grep -n 'source line\|soft break\|soft-break' AGENTS.md` -> no match: "Current repo state" never stated the soft-break rule, so there was nothing to replace. What it did state falsely was the `[x]`/`[ ]` glyph (`:100`). Corrected, and the section gains the fold-plus-hard-break rule, the four new glyphs, the two-renderers-agree point, and the Ambiguous caveat pointing at `SPEC.md` — an addition rather than the net-zero rewrite this task predicted, because the false claim was one word and the missing claims were four. `markdown-legibility` is added to the landed list. Originally: Rewrite in `AGENTS.md` (`:352` and its section): the "Current repo state" sentence describing markdown rendering (audience: same). Replace the claim that soft breaks preserve source line structure with the fold-plus-hard-break rule, and name the glyph set's Ambiguous-width caveat in one clause pointing at `SPEC.md`. Net effect is a rewrite, not an addition — nothing new is appended and two now-false claims are removed.

## 10. Lint & Verify
<!-- kind: operational -->

- [x] 10.1 CHECK: **Already satisfied** — the change directory was committed before this session began (`git ls-files openspec/changes/markdown-legibility` lists all eight files), so `/bin/sh scripts/gates/openspec-untouched.sh` exits **0** throughout: `OPENSPEC-UNTOUCHED OK (tree-only legs): no untracked file inside openspec/`. Originally: Commit the change directory before running `make gates`. At HEAD `/bin/sh scripts/gates/openspec-untouched.sh` exits **1** on this change's own untracked files (`OPENSPEC-UNTOUCHED FAIL: an untracked file exists inside openspec/`), which is the gate working, not a defect.
- [x] 10.2 VERIFY: **`make check` fails at `make test`, and only there, on the pre-existing `ui::tests::wiring::*` deadline flakes. Every other sub-command passes.**

  | Sub-command | Result |
  |---|---|
  | `make fmt-check` | OK |
  | `make lint` | OK (0 warnings) |
  | `make gates` | OK — every gate, `MDWIDTHS OK: all 35 markdown tests name both 58 and 78` among them |
  | `make test` | 1251 passed / 8 failed — **all eight inside `ui::tests::wiring::*`** |
  | `make coverage` | OK, both floors (see 10.4) |

  `cargo test --lib` with the wiring module filtered out is **fully green**, and every one of the seven integration binaries passes (`ci_workflow` 21, `cli` 9, `coverage_prod` 19, `degraded_coverage` 10, `doc_contract` 61, `manifest` 4, `spec_purposes` 3).

  **The flake is not this change's, measured rather than asserted.** `cargo test --lib ui::tests::wiring` run twice at this change's HEAD and twice at the pre-change commit `3f98b29`, back to back on the same machine at load average ~3 with no cargo competing:

  | Commit | Run 1 | Run 2 |
  |---|---|---|
  | `3f98b29` (pre-change) | **11** failed of 29, 36.3s | **9** failed, 33.2s |
  | this change | **8** failed of 29, 33.8s | **9** failed, 36.6s |

  The pre-change commit fails *more* often than the change does, and every failure carries the documented signature — a count asserted against `0` with an empty log (`left: 0 / right: 3`, `calls: []`). This machine simply runs these 29 wall-clock-deadlined tests at ~35s where `design.md`'s reference measurement recorded ~3.8s, so the 5-second `DEADLINE` in `crate::testutil::Stages` expires routinely here. Pre-existing, out of scope, and deserving its own change exactly as `planning-review.md` records. Originally: Run `make check` as the single gate. If it fails, name the failing sub-command — `make fmt-check`, `make lint`, `make gates`, `make test`, or `make coverage` — rather than reporting the composite. Run it on an otherwise idle machine. `ui::tests::wiring::*` failures under load are deadline flakes, not this change's — confirm by the empty-log signature and a clean isolated re-run, and report them separately rather than folding them into this change's result.
- [x] 10.3 VERIFY: **Reproduced end to end.** `/bin/sh scripts/gates/colwidth.sh` -> exit **0**, `COLWIDTH OK: no char-count measurement in the eight pure view files`. Planting `let _planted = source.chars().count();` into `lines` -> exit **1**, `COLWIDTH FAIL: src/ui/markdown.rs has 1 char-count measurement(s) in production code` naming line 1230; plant removed, exit **0** again. Originally: Confirm `COLWIDTH` still passes and can still fail. `/bin/sh scripts/gates/colwidth.sh` at HEAD → exit **0**, `COLWIDTH OK: no char-count measurement in the eight pure view files`. Negative control, run at planning time: inserting `let _planted = source.chars().count();` into `lines` makes it exit **1** with `COLWIDTH FAIL: src/ui/markdown.rs has 1 char-count measurement(s) in production code`; removing the plant returns it to exit **0**.
- [x] 10.4 VERIFY: **Both floors held.** `make check` never reached `coverage` (it aborts at `test`), so `make coverage` was run on its own — the assertion is still on a real run's output, not a re-derivation:

  - total: `cargo llvm-cov --fail-under-lines 80` exit **0**
  - production slice: `COVERAGE-PROD OK: production 96.04% (4609/4799) >= floor 96%; test-module 95.22% (26168/27481); 26 file(s) under src/`

  The production slice is the falsifiable one and it sits 0.04 points above its floor, so this change added no untested production branch. **One real defect was caught here rather than by review**: the Change Review's SUGGESTION-3 fix added eight comment lines above `set_task_marker`'s guard, which pushed the option-set line from 695 to 703 and left `tests/degraded-coverage.toml`'s `covers` pointer naming a comment. `make coverage` failed with `covers entry src/ui/markdown.rs:695-695 holds no line of code`; the pointer was corrected to `:703-703`. A `covers` pointer is a line number and moves whenever anything above it does — worth running `cargo test --test degraded_coverage` after any edit to a covered file, not only after a re-baseline. Originally: Read 10.2's coverage output and confirm both floors held — the total and the production slice. `Makefile:71` is `check: fmt-check lint gates test coverage`, so `make check` already ran it; re-running it here would be duplicate work, and the assertion is on its output. This change adds tests and no untested production branch, so a fall means a re-baselined test stopped exercising a path.
- [x] 10.5 VERIFY: `openspec validate markdown-legibility --strict` -> `Change 'markdown-legibility' is valid`. (The binary is under nvm and not on a non-login shell's `PATH`; reached as `$HOME/.nvm/versions/node/v24.18.0/bin/openspec`.)
