## Context

The dashboard's frame was designed in `tui-shell` against a full-screen terminal. The pane it
actually occupies is a Herdr split, and measured live at 60 columns that frame spends three of
its twenty rows and two of its sixty columns on chrome that says nothing — four columns is the
**wide** layout's figure, two regions at two borders each, and this paragraph previously
carried it into a 60-column measurement where it is not true: a header row whose
literal `OpenSpec` restates the pane title Herdr already draws one row above it, and a
bordered block titled `Changes` drawn inside a pane that is already a box. The rest of the
header row carries an absolute repository path too long to read at that width, of which only
the last component identifies anything.

Three smaller faults were reported alongside it: a selected collapsed section reads
`> > archived (30)` because the fold glyph and the cursor marker are the same character; the
detail region's change header, tab bar, and the artifact's first markdown line sit on three
adjacent rows, so the bar reads as content; and the divider between the two wide-layout
regions, once borders are gone, has no space around it.

This is unplanned work. `openspec/IMPLEMENTATION-ORDER.md` ends at Phase 6 (`degraded-states`),
and every decision this change revisits predates the pane being used in a split.

The geometry was settled against a **column-exact HTML prototype** rather than in prose —
`https://claude.ai/code/artifact/bae73f03-0c24-48d5-b382-bb67c1899039` — which models the
frame from the same arithmetic specified here and renders the current and proposed chromes
side by side at both mandated widths. Three decisions below changed as a direct result of
looking at it (D2, D5, D6).

## Goals / Non-Goals

**Goals:**

- Remove every drawn glyph that restates something the reader can already see.
- Give the divider, the tab bar, and the region headings room, without spending a mandated
  interior width.
- Make the fold state unmistakable at a glance.
- Keep both mandated interior widths — 38/58 for the list, 78/58 for the detail — **exactly**
  where they are, so no landed row-grammar, markdown, tasks, or detail expectation moves.

**Non-Goals:**

- No change to what the pane reads, writes, or spawns.
- No new keybinding, no new mouse gesture, and no key removed. Not **BREAKING**: the plugin
  manifest, the plugin config format, and every binding are unchanged.
- Not the false `is not vendored` problem row. It reverses a written decision in
  `change-merge` and is proposed separately.
- Not the dimming of artifact tabs whose file is not written yet. That needs a `present` flag
  on `ArtifactRef`, which is a change to what the pane **knows** rather than how it is drawn;
  it is proposed separately as `artifact-presence`.

## Boundaries

Every module this change touches is on the **pure** side of both architecture seams.

| Module | What changes | Pattern followed |
|---|---|---|
| `src/ui/layout.rs` | `split_frame` loses the header rect; `interior` takes a `Gutters` and reserves two rows; `split_body` gains the divider column; `split_detail` returns `(tabs, rule, content)`; `zone` follows | already the crate's one geometry module |
| `src/ui/view.rs` | `render_header` deleted; `render_region` draws a heading not a `Block`; `render_body` draws the divider; the detail draw path gains the rule | already the crate's only `Frame` consumer |
| `src/ui/list.rs` | `section_row_text`'s glyph pair; the list heading's own text | the existing row-grammar helpers |
| `src/ui/palette.rs` | five roles removed, three added | already the crate's only `Color` |
| `src/ui/app.rs` | destructures `split_body`'s pair at `:624`; `normalise_scroll` (`:628-637`) feeds `content.width` into `content_lines`, so the scroll clamp follows the interior | a compile error, not a design choice |
| `src/ui/driver.rs` | destructures `split_body`'s pair at `:3516` and `:3526`; two landed assertions move — `:3646` expects `(10,0)` to be `Ignore` where row 0 is now a region heading, and `:4303` expects the tab bar at `y == 3` where it becomes 2 | the hit test follows the geometry |
| `src/ui/mod.rs` | destructures `split_body`'s pair at `:758`; holds the "fourteen-row content area" expectations at `:843` and `:1132` | a compile error, not a design choice |
| `src/ui/detail.rs`, `src/ui/tasks.rs`, `src/ui/markdown.rs` | **nothing** — both mandated widths are unchanged, which is the point of the gutter arithmetic | — |

No file gains a filesystem, process, environment, network, or standard-I/O API, so
`NOIO-VIEW`'s nine-file `PURE` list is unchanged and needs no exemption. **No process spawn is
added anywhere**, inside `cli` or out; `NOSPAWN-GREP` and `LAUNCHSEAM` are untouched. No view
gains I/O: every new drawing decision is a pure function of `Dashboard` and a `Rect`.

`ui::layout::columns`/`truncate_columns` stay the crate's only width measure, so `COLWIDTH`'s
sweep is unaffected — the new `▾`/`▸` glyphs are measured through it like every other cell.

## Contracts

`ui::layout` is consumed by `ui::view`, `ui::app`, `ui::driver` and `ui::mod`, all inside this
crate. **Four** signatures change — the earlier draft of this table said three and omitted
`split_body`, which is the one whose call sites reach furthest — and every change is a
**compile error at each call site**, which is the intended enforcement:

| Signature | Before | After |
|---|---|---|
| `split_frame(Rect)` | `(header, body, footer)` | `(body, footer)` |
| `interior(Rect)` | `Rect` | `interior(Rect, Gutters) -> Rect` |
| `split_detail(Rect)` | `(header, tabs, content)` | `(tabs, rule, content)` |
| `split_body(Rect, Route)` | `(Option<Rect>, Option<Rect>)` | `(Option<Rect>, Option<u16>, Option<Rect>)` — the divider column between them |

`split_body`'s call sites outside `ui::layout` are `src/ui/view.rs:35`, `:513` and `:560`,
`src/ui/mod.rs:758`, `src/ui/driver.rs:3516` and `:3526`, and `src/ui/app.rs:624`. Three of
those files are not otherwise part of this change and appear in Boundaries for that reason
alone.

`ui::palette::Role` loses `HeaderTitle`, `HeaderPath`, `DetailHeader`, `RegionBorder`, and
`RegionBorderFocused`, and gains `RegionHeading`, `RegionHeadingFocused`, and `RegionRule`.
The exhaustive `match` in `palette::style` makes each removal a compile error at every site
that named one — the same mechanism `view-palette` already relies on for additions.

No **external** consumer is affected. `herdr-plugin.toml`, the `[[panes]]` and `[[actions]]`
entries, the `open`/`open-tab` subcommands, and every documented key are untouched.

The `Change` type is **not** altered, so `from_files` and `from_cli` need no new agreement
mechanism and `changes::conformance::assert_invariants` is unchanged. (`artifact-presence`,
proposed separately, is the change that would alter it.)

## Persistence and Rollout

- **migration:** none — nothing is persisted by this change.
- **backfill:** none.
- **seeding:** none.
- **cache invalidation:** none. The `artifact-content` cache is keyed on `(change directory,
  tab)`, neither of which this change touches.
- **index rebuild:** none.
- **authorization:** none.
- **observability:** none.
- **deployment:** `make build` then reload the pane. The plugin manifest is unchanged, so
  `herdr plugin link` need not be re-run.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| The terminal (`crossterm` raw mode, alternate screen, mouse capture) | replaced — `ratatui::backend::TestBackend`, never a real terminal | replaced — `TestBackend`. `cargo test` spawns this binary, so a real terminal would corrupt the developer's session |
| The filesystem (`openspec/` tree, artifact files) | replaced — the artifact read is the injected `&dyn Fn(&Path) -> Result<String, String>` | replaced — same injection; `ui::read_artifact` is never called |
| The `openspec` binary | absent — no `OpenspecCli` is constructed; `Dashboard` values are built directly | absent |
| The Herdr socket (`herdr agent list`, `herdr agent start`) | replaced — `AgentPoll`/`Launcher` trait objects returning fixtures | replaced — same |
| The filesystem watcher (`notify`) | replaced — `FsEvents` trait object | replaced — same |
| The refresh worker thread | replaced — `Refresher` trait object | replaced — same |
| The process environment | replaced — the injected `&dyn Fn(&str) -> Option<String>` | replaced — same |
| The clock | absent — no view reads one, and `NOBLOCK` forbids it under `src/ui/` | absent |
| The repository tree itself (gate scripts) | real — a scratch copy under `tests/gate_controls.rs` | n/a |

No dependency is left unstated. Nothing in this change is real that was replaced before, and
nothing replaced that was real.

## Test Strategy

This change takes **no outer-loop acceptance test**, and the reason is structural rather than
convenience: the behaviour being changed is *what a frame's cells contain*, and the fastest
tier that can observe that is a `TestBackend` render inside `src/ui/`. There is no layer above
it that would see anything a render test does not — `run_loop` draws through the very function
under test, and the only thing an integration-tier test could add is a real terminal, which
`terminal-lifecycle` forbids because `cargo test` spawns this binary.

The repository's tiers, and what this change puts in each:

| Tier | Command | This change's use |
|---|---|---|
| unit — inline `#[cfg(test)]` under `src/ui/` | `cargo test --all-features <module>` | every rendered and every pure-geometry scenario |
| contract — `tests/*.rs` | `cargo test --all-features --test <name>` | `tests/doc_contract.rs` (the confined-seam names and the module map) and `tests/degraded_coverage.rs` (`SPEC.md`'s file-mode row) |
| hygiene gates | `make gates` | `PALETTE`, `NOIO-VIEW`, `COLWIDTH`, `MDSEAM`, `DETAILWIDTHS`, `MDWIDTHS`, `TASKWIDTHS` — all unchanged in intent, and all must still pass |
| coverage | `cargo llvm-cov --fail-under-lines 80` | the production-slice floor, unchanged |

**One** documentation site is bound inside `cargo test` tightly enough to fail on this change,
and three more drift in silence. The distinction matters because the earlier draft of this
paragraph claimed all of them "fail loudly and name both sides", and a claim of enforcement
that does not hold is worse than no claim: it is why the tasks list stopped naming them.

- **Fails loudly:** `SPEC.md`'s degraded-states row for the `file mode` badge, via
  `tests/degraded-coverage.toml`. That row's two proof names, its `why`, and its `covers`
  range all go stale — one proof is `file_mode_badge_is_dim_after_the_label`, named for the
  `OpenSpec` label this change deletes, and `covers` is `src/ui/view.rs:293-301`, which spans
  the deleted `render_header`.
- **Drifts silently:** `SPEC.md`'s mouse table. `tests/doc_contract.rs` binds it by the set of
  backticked `Action::` variants it names, cross-checked against `mouse_action`'s body
  (`:1837-1875`); this change adds no `Action` variant, so the table's *prose* — which lists
  "the header row" and "a border" among the ignored gestures — goes false with nothing
  failing.
- **Drifts silently:** `SPEC.md` § List view (`:388-407`), whose three example blocks and
  whose sentence "its glyph is `v` when open and `>` when collapsed" are falsified by D6.
  Nothing binds fold glyphs.
- **Drifts silently:** the `## Purpose` of `responsive-layout`, `detail-header`,
  `artifact-tabs` and `view-palette` in `openspec/specs/`. `tests/spec_purposes.rs` checks
  only that a Purpose exists and is not the archive placeholder (`:11`, `:120-128`), never
  that it is true.

The **confined terminal-seam names** are *not* affected: this change adds and removes no
crossterm terminal-mode function, so `doc_contract`'s `terminal_seam_names_match_the_gate` has
no subject here. The earlier tasks list named that leg and not the four above.

### Verification matrix

One row per spec scenario, 149 in total, and every row names **the test that proves it** and
a command that **runs exactly that test**.

Both halves are load-bearing, and the first draft of this matrix had neither. Its commands
passed two or three bare filters — `cargo test --all-features ui::detail ui::view` — and
`cargo test` accepts one `TESTNAME`: two bare filters exit 1, and the quoted form exits **0**
having run nothing (`0 passed; 1222 filtered out`). 123 of these 149 rows carried such a
command. This repository already wrote the rule down after the same defect was caught in
`mouse-input` at three rows: *every filtered command in the matrix must run more than zero
tests, because a row whose filter matches nothing is a green row that proves nothing.*

A bare test name is a valid single filter and matches on the full test path, so the module a
test lives in is not part of the command and cannot be got wrong here.

**The names in the Verification column are the contract, not a suggestion.** Where a landed
test already proves a row, it is renamed to the name given here; where the row's RED task
writes it, that is the name it takes. A name that exists nowhere is not a green row — it is a
filter matching nothing, which exits 0 — so the matrix is only worth anything alongside the
conformance check in tasks.md → group 9, which runs every command in this table and fails on
any that runs zero tests. That check is what makes this table falsifiable; without it the
table is prose.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| artifact-content → A missing artifact still shows its tab and reads `No content yet` | `a_missing_artifact_still_shows_its_tab_and_reads_no_content_yet` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_missing_artifact_still_shows_its_tab_and_reads_no_content_yet` |
| artifact-content → `No content yet` does not eat the border at a narrow frame | `no_content_yet_does_not_eat_the_border_at_a_narrow_frame` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib no_content_yet_does_not_eat_the_border_at_a_narrow_frame` |
| artifact-content → A read failure is named above the content at both widths | `a_read_failure_is_named_above_the_content_at_both_widths` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_read_failure_is_named_above_the_content_at_both_widths` |
| artifact-content → The rendered markdown fills the content area, not the whole interior | `the_twenty_item_list_fills_the_content_area_rows_5_through_18` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_twenty_item_list_fills_the_content_area_rows_5_through_18` |
| artifact-content → The tracked-tasks tab renders the checklist body instead | `marked_tab_renders_checklist_body` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib marked_tab_renders_checklist_body` |
| artifact-content → A wide-character document stays inside the detail region | `a_wide_character_document_stays_inside_the_detail_region` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_wide_character_document_stays_inside_the_detail_region` |
| artifact-content → `content_lines` is total and width-parameterised | `content_lines_total` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib content_lines_total` |
| artifact-content → No `content_lines` line exceeds its width at any width | `no_content_lines_line_exceeds_its_width_at_any_width` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib no_content_lines_line_exceeds_its_width_at_any_width` |
| artifact-tabs → The tab bar reaches the buffer at both mandated widths | `the_tab_bar_reaches_the_buffer_at_both_mandated_widths` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_tab_bar_reaches_the_buffer_at_both_mandated_widths` |
| artifact-tabs → The bar, the rule, and the content never leave the interior | `the_bar_the_rule_and_the_content_never_leave_the_interior` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_bar_the_rule_and_the_content_never_leave_the_interior` |
| artifact-tabs → `split_detail` is exact at its degenerate heights | `split_detail_is_exact_at_its_degenerate_heights` | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features --lib split_detail_is_exact_at_its_degenerate_heights` |
| change-rows → Active rows render at both mandated widths | `list_rows_render_at_60_and_120` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib list_rows_render_at_60_and_120` |
| change-rows → A badged row carries its status between the name and the progress cell | `badged_rows_render_at_both_widths` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib badged_rows_render_at_both_widths` |
| change-rows → An unattributed agent badges nothing | `an_unattributed_agent_badges_nothing` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib an_unattributed_agent_badges_nothing` |
| change-rows → A watch problem leads the list, above a change-set problem | `refresh_and_change_problems_in_order` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib refresh_and_change_problems_in_order` |
| change-rows → The row grammar places the marker, the name, and the progress cell | `active_row_grammar_at_38_and_58` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib active_row_grammar_at_38_and_58` |
| change-rows → A name too long for the field is truncated with an ellipsis | `a_long_name_is_truncated_with_an_ellipsis` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_long_name_is_truncated_with_an_ellipsis` |
| change-rows → A field too narrow for both drops the progress cell whole | `a_badged_row_drops_the_badge_first` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_badged_row_drops_the_badge_first` |
| change-rows → No repository names the directory searched, at both widths | `no_repository_names_the_directory_searched` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib no_repository_names_the_directory_searched` |
| change-rows → A repository with no changes at all | `a_repository_with_no_changes_says_so` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_repository_with_no_changes_says_so` |
| change-rows → No active changes with archived ones still browsable | `no_active_changes_with_archived_ones_still_browsable` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib no_active_changes_with_archived_ones_still_browsable` |
| change-rows → A collapsed but non-empty archive is not "no changes yet" | `a_collapsed_but_non_empty_archive_is_not_no_changes_yet` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_collapsed_but_non_empty_archive_is_not_no_changes_yet` |
| change-rows → A collapsed active section shows its header and no message row | `a_collapsed_active_section_shows_its_header_and_no_message_row` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_collapsed_active_section_shows_its_header_and_no_message_row` |
| change-rows → Repository-level problems are named above the rows | `repository_problems_are_named_above_the_rows` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib repository_problems_are_named_above_the_rows` |
| change-rows → The detail region stays blank while the list fills | `the_detail_region_stays_blank_while_the_list_fills` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_detail_region_stays_blank_while_the_list_fills` |
| change-rows → The narrow detail route draws no rows | `the_narrow_detail_route_draws_no_rows` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_narrow_detail_route_draws_no_rows` |
| change-rows → More changes than rows do not overflow the region | `more_changes_than_rows_do_not_overflow` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib more_changes_than_rows_do_not_overflow` |
| change-rows → A CJK change name stays inside the list region at both mandated widths | `a_cjk_change_name_stays_inside_the_list_region_at_both_mandated_widths` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_cjk_change_name_stays_inside_the_list_region_at_both_mandated_widths` |
| change-rows → An emoji change name at 58 columns does not overwrite the border | `an_emoji_change_name_at_58_columns_does_not_overwrite_the_border` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib an_emoji_change_name_at_58_columns_does_not_overwrite_the_border` |
| change-rows → A wide name is truncated whole and padded back to the full width | `a_wide_name_is_truncated_whole_and_padded_back_to_the_full_width` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_wide_name_is_truncated_whole_and_padded_back_to_the_full_width` |
| change-rows → Rows are total over adversarial names at every width | `rows_are_total_over_adversarial_names_at_every_width` | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features --lib rows_are_total_over_adversarial_names_at_every_width` |
| change-rows → The no-repository block shortens its search path by columns | `the_no_repository_block_shortens_its_search_path_by_columns` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_no_repository_block_shortens_its_search_path_by_columns` |
| change-rows → The section header and archived rows render at both mandated widths | `the_section_header_and_archived_rows_render_at_both_mandated_widths` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_section_header_and_archived_rows_render_at_both_mandated_widths` |
| change-rows → An archived change carries a badge in the same column as an active one | `an_archived_change_carries_a_badge_in_the_same_column_as_an_active_one` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib an_archived_change_carries_a_badge_in_the_same_column_as_an_active_one` |
| change-rows → A query against an unresolved archive counts from `archived_total` | `a_query_against_an_unresolved_archive_counts_from_archived_total` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_query_against_an_unresolved_archive_counts_from_archived_total` |
| change-rows → A collapsed archived section shows its count and no rows | `a_collapsed_archived_section_shows_its_count_and_no_rows` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_collapsed_archived_section_shows_its_count_and_no_rows` |
| change-rows → An expanded but unresolved archived section shows its header alone | `an_expanded_but_unresolved_archived_section_shows_its_header_alone` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib an_expanded_but_unresolved_archived_section_shows_its_header_alone` |
| change-rows → An archived row drops the progress cell, then the date, as the width falls | `an_archived_row_drops_the_progress_cell_then_the_date_as_the_width_falls` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib an_archived_row_drops_the_progress_cell_then_the_date_as_the_width_falls` |
| change-rows → A section header degrades by truncation at every width | `a_section_header_degrades_by_truncation_at_every_width` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_section_header_degrades_by_truncation_at_every_width` |
| change-rows → No archived changes means no archived header | `no_archived_changes_means_no_archived_header` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib no_archived_changes_means_no_archived_header` |
| detail-header → The header names the selected change at both mandated widths | `the_header_names_the_selected_change_at_both_mandated_widths` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_header_names_the_selected_change_at_both_mandated_widths` |
| detail-header → Moving the selection moves the header | `moving_the_selection_moves_the_header` | unit (`src/ui/view.rs` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib moving_the_selection_moves_the_header` |
| detail-header → An archived change's header carries its stripped name and its own schema | `an_archived_change_s_header_carries_its_stripped_name_and_its_own_schema` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib an_archived_change_s_header_carries_its_stripped_name_and_its_own_schema` |
| detail-header → An empty visible list leaves the whole detail interior blank | `an_empty_visible_list_leaves_the_whole_detail_interior_blank` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib an_empty_visible_list_leaves_the_whole_detail_interior_blank` |
| detail-header → A CJK change name keeps the header inside its region at both mandated widths | `a_cjk_change_name_keeps_the_header_inside_its_region_at_both_mandated_widths` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_cjk_change_name_keeps_the_header_inside_its_region_at_both_mandated_widths` |
| detail-header → The header reaches the buffer without crossing the region border | `the_header_reaches_the_buffer_without_crossing_the_region_border` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_header_reaches_the_buffer_without_crossing_the_region_border` |
| detail-header → The header is total over adversarial names at every width | `header_row_is_total_over_adversarial_names_at_every_width` | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features --lib header_row_is_total_over_adversarial_names_at_every_width` |
| detail-header → The detail header is bold and uncoloured at both mandated widths | `the_detail_header_is_bold_and_uncoloured_at_both_mandated_widths` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_detail_header_is_bold_and_uncoloured_at_both_mandated_widths` |
| detail-scroll → The document fills the detail interior at both mandated widths | `the_detail_document_fills_the_interior_at_60_and_120` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_detail_document_fills_the_interior_at_60_and_120` |
| detail-scroll → Faces reach the buffer as styles at both widths | `faces_reach_the_buffer_as_styles` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib faces_reach_the_buffer_as_styles` |
| detail-scroll → A table reaches the buffer aligned and inside the region | `a_table_reaches_the_buffer_aligned` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_table_reaches_the_buffer_aligned` |
| detail-scroll → An empty source leaves the detail interior blank at both widths | `an_empty_detail_source_leaves_the_interior_blank` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib an_empty_detail_source_leaves_the_interior_blank` |
| detail-scroll → Content never overwrites the detail region's border | `content_never_overwrites_the_detail_region_s_border` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib content_never_overwrites_the_detail_region_s_border` |
| detail-scroll → A degenerate detail interior draws nothing and does not panic | `a_degenerate_detail_interior_draws_nothing_and_does_not_panic` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_degenerate_detail_interior_draws_nothing_and_does_not_panic` |
| detail-scroll → `scroll_offset` is exact at its boundaries | `scroll_offset_is_exact_at_its_boundaries` | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features --lib scroll_offset_is_exact_at_its_boundaries` |
| detail-scroll → `interior` agrees with a bordered block's own inner rectangle | `interior_agrees_with_a_bordered_block_s_own_inner_rectangle` | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features --lib interior_agrees_with_a_bordered_block_s_own_inner_rectangle` |
| detail-scroll → A scroll offset past the end still draws the last screenful | `a_scroll_past_the_end_draws_the_last_screenful` | unit (`src/ui/view.rs` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib a_scroll_past_the_end_draws_the_last_screenful` |
| detail-scroll → At the detail route the content scrolls by one line at both widths | `next_and_prev_scroll_at_the_detail_route` | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib next_and_prev_scroll_at_the_detail_route` |
| detail-scroll → At the list route the same actions still move the selection | `next_and_prev_select_at_the_list_route` | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib next_and_prev_select_at_the_list_route` |
| detail-scroll → Scrolling stops at the top | `scroll_stops_at_the_top` | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib scroll_stops_at_the_top` |
| detail-scroll → While filtering, `j` and `k` still type into the query | `filter_mode_types_j_and_k_while_arrows_scroll` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib filter_mode_types_j_and_k_while_arrows_scroll` |
| detail-scroll → Every route move resets the scroll | `every_route_move_resets_the_scroll` | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib every_route_move_resets_the_scroll` |
| detail-scroll → `Enter` at the detail route moves nothing and keeps the scroll | `enter_at_the_detail_route_is_a_noop` | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib enter_at_the_detail_route_is_a_noop` |
| detail-scroll → `Enter` from the list route still opens at the top | `enter_from_list_route_still_opens_at_the_top` | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib enter_from_list_route_still_opens_at_the_top` |
| detail-scroll → `Enter` while filtering still dismisses the filter and resets nothing | `enter_while_filtering_still_dismisses_and_resets_nothing` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib enter_while_filtering_still_dismisses_and_resets_nothing` |
| list-filtering → A query narrows both tiers at both widths | `a_query_narrows_both_tiers` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_query_narrows_both_tiers` |
| list-filtering → Matching ignores case | `matching_ignores_case` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib matching_ignores_case` |
| list-filtering → Matching ignores case outside ASCII | `matches_ignores_case_outside_ascii` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib matches_ignores_case_outside_ascii` |
| list-filtering → A query matching only an archived change | `a_query_matching_only_an_archived_change` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_query_matching_only_an_archived_change` |
| list-filtering → A query matching nothing names itself | `a_query_matching_nothing_names_itself` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_query_matching_nothing_names_itself` |
| list-filtering → The fold is total and its documented edge cases hold | `the_fold_is_total_and_its_documented_edge_cases_hold` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_fold_is_total_and_its_documented_edge_cases_hold` |
| list-filtering → Shrinking the visible list clamps the selection | `selection_clamps_when_the_filter_shrinks_the_list` | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib selection_clamps_when_the_filter_shrinks_the_list` |
| list-filtering → A query reaches a match inside a folded archive | `a_query_reaches_a_match_inside_a_folded_archive` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_query_reaches_a_match_inside_a_folded_archive` |
| list-filtering → The archived count under a query is the matched count | `the_archived_count_under_a_query_is_the_matched_count` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_archived_count_under_a_query_is_the_matched_count` |
| list-filtering → The first character of a query requests the archive it needs | `the_first_character_of_a_query_requests_the_archive_it_needs` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_first_character_of_a_query_requests_the_archive_it_needs` |
| list-selection → A selection past the interior scrolls the slice at both widths | `a_selection_past_the_interior_scrolls_the_slice` | unit (`src/ui/view.rs` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib a_selection_past_the_interior_scrolls_the_slice` |
| list-selection → The last change is reachable and the slice stops at the end | `the_last_change_is_reachable` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_last_change_is_reachable` |
| list-selection → The viewport is exact at its boundaries | `viewport_boundaries_are_exact` | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features --lib viewport_boundaries_are_exact` |
| list-selection → A resize changes the slice on the next frame | `resizing_changes_the_slice_on_the_next_frame` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib resizing_changes_the_slice_on_the_next_frame` |
| list-selection → `Space` on a header folds and unfolds that section | `space_on_a_header_folds_and_unfolds_that_section` | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib space_on_a_header_folds_and_unfolds_that_section` |
| list-selection → `Space` inside a section folds it and moves the cursor to its header | `space_inside_a_section_folds_it_and_moves_the_cursor_to_its_header` | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib space_inside_a_section_folds_it_and_moves_the_cursor_to_its_header` |
| list-selection → An empty list makes `Space` inert | `an_empty_list_makes_space_inert` | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib an_empty_list_makes_space_inert` |
| list-selection → Opening an unresolved archive requests a refresh | `opening_an_unresolved_archive_requests_a_refresh` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib opening_an_unresolved_archive_requests_a_refresh` |
| list-selection → A refresh does not undo a fold | `a_refresh_does_not_undo_a_fold` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_refresh_does_not_undo_a_fold` |
| list-selection → A refresh keeps the cursor on the same change | `a_refresh_keeps_the_cursor_on_the_same_change` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_refresh_keeps_the_cursor_on_the_same_change` |
| mouse-input → The two regions scroll independently at 120 columns | `the_two_regions_scroll_independently` | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib the_two_regions_scroll_independently` |
| mouse-input → The wheel acts over a border and not over the chrome | `the_wheel_acts_over_a_border_and_not_over_the_chrome` | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib the_wheel_acts_over_a_border_and_not_over_the_chrome` |
| mouse-input → At 60 columns only the routed region answers the wheel | `at_60_columns_only_the_routed_region_answers` | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib at_60_columns_only_the_routed_region_answers` |
| mouse-input → Horizontal wheel events do nothing | `horizontal_wheel_events_do_nothing` | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib horizontal_wheel_events_do_nothing` |
| mouse-input → A click selects a change row and a second click opens it | `a_click_selects_and_a_second_click_opens` | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib a_click_selects_and_a_second_click_opens` |
| mouse-input → A click on a section header folds it exactly as `Space` does | `a_header_click_equals_space` | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib a_header_click_equals_space` |
| mouse-input → A click on an archived header opens an unresolved archive and requests its refresh | `a_header_click_requests_the_archive_refresh` | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib a_header_click_requests_the_archive_refresh` |
| mouse-input → A click on a tab cell switches to that artifact | `a_tab_click_switches_the_tab` | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib a_tab_click_switches_the_tab` |
| mouse-input → Clicks that address nothing are inert | `clicks_that_address_nothing_are_inert` | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib clicks_that_address_nothing_are_inert` |
| mouse-input → The other buttons and the non-press kinds are inert | `the_other_buttons_are_inert` | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib the_other_buttons_are_inert` |
| responsive-layout → Body and footer occupy their rows at both widths | `body_and_footer_occupy_their_rows_at_both_widths` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib body_and_footer_occupy_their_rows_at_both_widths` |
| responsive-layout → The action hints follow `Esc back` when the socket is reachable | `the_action_hints_follow_esc_back_when_reachable` | unit (`src/ui/view.rs` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib the_action_hints_follow_esc_back_when_reachable` |
| responsive-layout → The action hints are dropped whole, `g focus` first | `the_action_hints_are_dropped_whole_g_focus_first` | unit (`src/ui/view.rs` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features --lib the_action_hints_are_dropped_whole_g_focus_first` |
| responsive-layout → A one-row frame renders the body's heading row and nothing else | `a_one_row_frame_renders_the_body_s_heading_row_and_nothing_else` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_one_row_frame_renders_the_body_s_heading_row_and_nothing_else` |
| responsive-layout → A two-row frame renders one body row and the footer | `a_two_row_frame_renders_one_body_row_and_the_footer` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_two_row_frame_renders_one_body_row_and_the_footer` |
| responsive-layout → A one-column frame renders without panicking | `a_one_column_frame_renders_without_panicking` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_one_column_frame_renders_without_panicking` |
| responsive-layout → The footer drops whole hints rather than truncating one | `footer_drops_whole_hints` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib footer_drops_whole_hints` |
| responsive-layout → The unattributed count is the footer's last hint at both widths | `the_unattributed_count_is_the_last_hint` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_unattributed_count_is_the_last_hint` |
| responsive-layout → The count is reported with an empty change list | `the_count_is_reported_with_an_empty_list` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_count_is_reported_with_an_empty_list` |
| responsive-layout → The count is dropped whole before the three key hints | `the_count_drops_before_the_key_hints` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_count_drops_before_the_key_hints` |
| responsive-layout → The filter prompt replaces the count along with the hints | `the_count_survives_a_filter` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_count_survives_a_filter` |
| responsive-layout → A region draws a heading, a blank row, and no border at both widths | `a_region_draws_a_heading_a_blank_row_and_no_border_at_both_widths` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_region_draws_a_heading_a_blank_row_and_no_border_at_both_widths` |
| responsive-layout → `interior` reserves two rows and the gutters its `Gutters` names | `interior_reserves_two_rows_and_the_gutters_its_gutters_names` | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features --lib interior_reserves_two_rows_and_the_gutters_its_gutters_names` |
| responsive-layout → The divider has a blank column on each side at 120 columns | `the_divider_has_a_blank_column_on_each_side_at_120_columns` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_divider_has_a_blank_column_on_each_side_at_120_columns` |
| responsive-layout → The divider column is a width branch, not a constant | `the_detail_interior_is_58_columns_at_the_100_column_breakpoint` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_detail_interior_is_58_columns_at_the_100_column_breakpoint` |
| responsive-layout → A one-, two-, and three-column frame degenerates without drawing over a gutter | `a_one_two_and_three_column_frame_degenerates_without_drawing_over_a` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_one_two_and_three_column_frame_degenerates_without_drawing_over_a` |
| responsive-layout → The heading names the directory, not the path, at both widths | `the_heading_names_the_directory_not_the_path_at_both_widths` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_heading_names_the_directory_not_the_path_at_both_widths` |
| responsive-layout → A name longer than the heading row keeps its tail | `a_name_longer_than_the_heading_row_keeps_its_tail` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_name_longer_than_the_heading_row_keeps_its_tail` |
| responsive-layout → The badge is right-aligned and dropped whole | `the_badge_is_right_aligned_and_dropped_whole` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_badge_is_right_aligned_and_dropped_whole` |
| responsive-layout → No repository names itself in the heading | `no_repository_names_itself_in_the_heading` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib no_repository_names_itself_in_the_heading` |
| responsive-layout → The routed region's heading is bold and the other's is dim | `the_routed_region_s_heading_is_bold_and_the_other_s_is_dim` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_routed_region_s_heading_is_bold_and_the_other_s_is_dim` |
| responsive-layout → No heading or rule cell carries a colour | `no_heading_or_rule_cell_carries_a_colour` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib no_heading_or_rule_cell_carries_a_colour` |
| responsive-layout → Every colour literal still lives in the palette module alone | the gate exits 0 on the tree; `tests/gate_controls.rs` proves it exits non-zero on its planted control | hygiene gate (`make gates`) + `tests/gate_controls.rs` | filesystem real (scratch copy), no process | `bash scripts/gates/palette.sh` **and** `cargo test --all-features --test gate_controls` |
| responsive-layout → At 120 columns both regions are drawn with the divider at column 40 | `at_120_columns_both_regions_are_drawn_with_the_divider_at_column_40` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib at_120_columns_both_regions_are_drawn_with_the_divider_at_column_40` |
| responsive-layout → At 60 columns only the routed region is drawn | `at_60_columns_only_the_routed_region_is_drawn` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib at_60_columns_only_the_routed_region_is_drawn` |
| responsive-layout → At 60 columns the detail route replaces the list region | `at_60_columns_the_detail_route_replaces_the_list_region` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib at_60_columns_the_detail_route_replaces_the_list_region` |
| responsive-layout → The breakpoint is exact at 99, 100, and 101 columns | `the_breakpoint_is_exact_at_99_100_and_101_columns` | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features --lib the_breakpoint_is_exact_at_99_100_and_101_columns` |
| responsive-layout → The mode follows the current frame, not the startup size | `the_mode_follows_the_current_frame_not_the_startup_size` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_mode_follows_the_current_frame_not_the_startup_size` |
| responsive-layout → The routed region's border is bold and the other's is not | `the_routed_region_s_border_is_bold_and_the_other_s_is_not` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_routed_region_s_border_is_bold_and_the_other_s_is_not` |
| responsive-layout → Interiors are blank at both widths | `region_interiors_are_blank` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib region_interiors_are_blank` |
| responsive-layout → Rows do not overwrite the borders at either width | `rows_do_not_overwrite_the_borders` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib rows_do_not_overwrite_the_borders` |
| responsive-layout → The detail interior is 78 columns at 120 and 58 at 60 | `the_detail_interior_is_78_columns_at_120_and_58_at_60` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_detail_interior_is_78_columns_at_120_and_58_at_60` |
| responsive-layout → Every markdown test names both of its two interior widths | the gate exits 0 on the tree; `tests/gate_controls.rs` proves it exits non-zero on its planted control | hygiene gate (`make gates`) + `tests/gate_controls.rs` | filesystem real (scratch copy), no process | `bash scripts/gates/mdwidths.sh` **and** `cargo test --all-features --test gate_controls` |
| responsive-layout → The zones tile the frame at 120 columns | `the_zones_tile_the_frame_at_120_columns` | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features --lib the_zones_tile_the_frame_at_120_columns` |
| responsive-layout → Below the breakpoint only the routed region has zones | `below_the_breakpoint_only_the_routed_region_has_zones` | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features --lib below_the_breakpoint_only_the_routed_region_has_zones` |
| responsive-layout → The breakpoint is exact for the hit test too | `the_breakpoint_is_exact_for_the_hit_test_too` | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features --lib the_breakpoint_is_exact_for_the_hit_test_too` |
| responsive-layout → Degenerate frames resolve without panicking | `degenerate_frames_resolve_without_panicking` | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features --lib degenerate_frames_resolve_without_panicking` |
| responsive-layout → The hit test agrees with what was drawn | `the_hit_test_agrees_with_what_was_drawn` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_hit_test_agrees_with_what_was_drawn` |
| tasks-checklist → The tasks tab shows checkboxes and its siblings show markdown | `tasks_tab_shows_checkboxes` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib tasks_tab_shows_checkboxes` |
| tasks-checklist → The tab is chosen by `tracks_tasks`, not by its id | `tasks_tab_chosen_by_flag` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib tasks_tab_chosen_by_flag` |
| tasks-checklist → A schema naming no tasks artifact leaves every tab as markdown | `no_marked_artifact_renders_markdown` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib no_marked_artifact_renders_markdown` |
| tasks-checklist → A `detail.tab` past the end of the artifact list renders no checklist | `tab_past_the_end` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib tab_past_the_end` |
| tasks-checklist → A prose-only tasks file reads `No tasks yet` | `prose_only_reads_no_tasks_yet` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib prose_only_reads_no_tasks_yet` |
| tasks-checklist → `No tasks yet` does not eat the border at a narrow frame | `no_tasks_yet_does_not_eat_the_border_at_a_narrow_frame` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib no_tasks_yet_does_not_eat_the_border_at_a_narrow_frame` |
| tasks-checklist → A missing tasks artifact still reads `No content yet` | `missing_tasks_artifact_no_content_yet` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib missing_tasks_artifact_no_content_yet` |
| tasks-checklist → A read failure on the tasks tab names its reason and renders no checklist | `tasks_tab_read_failure` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib tasks_tab_read_failure` |
| view-palette → The palette answers every role with a `Style` | `the_palette_answers_every_role_with_a_style` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib the_palette_answers_every_role_with_a_style` |
| view-palette → The confinement gate catches a `Color` named outside the palette | the gate exits 0 on the tree; `tests/gate_controls.rs` proves it exits non-zero on its planted control | hygiene gate (`make gates`) + `tests/gate_controls.rs` | filesystem real (scratch copy), no process | `bash scripts/gates/palette.sh` **and** `cargo test --all-features --test gate_controls` |
| view-palette → The palette module reaches no I/O and measures no width | the gate exits 0 on the tree; `tests/gate_controls.rs` proves it exits non-zero on its planted control | hygiene gate (`make gates`) + `tests/gate_controls.rs` | filesystem real (scratch copy), no process | `bash scripts/gates/noio-view.sh` and `bash scripts/gates/colwidth.sh` **and** `cargo test --all-features --test gate_controls` |
| view-palette → Each role's modifier set is exactly the table above | `each_role_s_modifier_set_is_exactly_the_table_above` | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features --lib each_role_s_modifier_set_is_exactly_the_table_above` |
| view-palette → A monochrome reading of the frame is unchanged | `a_monochrome_reading_of_the_frame_is_unchanged` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_monochrome_reading_of_the_frame_is_unchanged` |
| view-palette → Faces reach the buffer as coloured styles at both mandated widths | `faces_reach_the_buffer_as_coloured_styles` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib faces_reach_the_buffer_as_coloured_styles` |
| view-palette → Heading foreground wins over a code span inside it | `heading_foreground_wins_over_a_code_span_inside_it` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib heading_foreground_wins_over_a_code_span_inside_it` |
| view-palette → A plain face is the default style | `a_plain_face_is_the_default_style` | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features --lib a_plain_face_is_the_default_style` |

## Decisions

**D1 — Gutters, not narrower interiors.** A bordered region's interior was its area less two
border columns. A borderless region's interior is its area less two **gutter** columns, which
is the same arithmetic, so 38/58 and 78/58 do not move and roughly 145 hand-computed test
expectations stay true. *Alternative considered:* draw content flush to the region's edges and
let the interiors widen by two. Rejected — every row-grammar, markdown, tasks and detail
expectation in the tree is written against the current widths, and three `scripts/gates/`
scripts hard-code `78`.

**D2 — Every region is heading row, padding row, interior.** The list region wants a blank row
under the repository's name; the detail region wants one under the change's header. They are
the same row at the same offset, so they are one part of one shape rather than two special
cases in two draw paths. *This decision came from the prototype*: the first draft put a spacer
only in the detail region, and looking at the two side by side made the asymmetry obvious. The
happy consequence is arithmetic rather than aesthetic — two rows are freed (the frame header
and the bottom border) and one is spent, so the interior still **begins at buffer row 2**,
exactly where the bordered one did, and `change-rows`' scenarios move only their last index.
*Alternative considered:* no padding row, interiors two rows taller. Rejected on looking at it.

**D3 — `Gutters::Both` / `Gutters::LeftOnly`, not two functions.** `interior` stays one
function with one clamp and one saturating subtraction. *Alternative considered:*
`interior` plus `interior_flush_right`. Rejected — two functions is two places for the origin
clamp to be got wrong, and that clamp is the one thing `detail-scroll` calls out as easy to get
wrong. There is no `RightOnly` and no `Neither`: nothing constructs them, and a variant nothing
constructs is a variant nothing tests.

**D4 — The divider is a `Length(1)` part of the body, owned by neither region.** It is drawn by
`render_body`, the one place that knows both rectangles. *Alternative considered:* draw it into
one region's gutter. Rejected — that is exactly the arrangement that leaves it flush against one
side's content, which is the fault D5 exists to fix.

**D5 — A blank column on **both** sides of the divider, paid for by the detail region's trailing
gutter.** `38 + 78` leaves four chrome columns at a 120-column frame and a spaced divider needs
five, so one outer gutter cannot be had. The **leading** one is kept because column 0 carries the
list's selection marker on every row, where a flush edge reads as part of the grammar; the
trailing edge is reached only by a right-aligned cell. *Alternatives considered and measured:*
(a) take the column from the detail interior, making it 77 — rejected, `78` appears 162 times
under `src/ui/` and in three gate scripts, so a column of whitespace would cost roughly **98**
recomputed expectations (`detail` 42 + `markdown` 34 + `tasks` 22; the 145 figure D1 uses is the
whole hand-computed set including `list`'s 47, which name 38 and 58 rather than 78 and so do not
move for a detail-only change); (b) drop the leading gutter instead — rejected on looking at the prototype, the
marker column is the more noticeable edge; (c) no divider at all, two blank columns — the only
arrangement compromising nothing, kept in the prototype as a comparison and rejected because the
two regions read as one field without it.

**D6 — `▾`/`▸` over `+`/`-` and over keeping `v`/`>`.** The landed pair made the fold glyph and
the cursor marker the same character two columns apart, which reads as one repeated thing. Both
replacements are one display column and unambiguous-width in Unicode's East Asian Width table, so
`layout::columns` measures them as one and no terminal's ambiguous-width setting can widen them.
*Alternative considered:* `+`/`-`. Rejected — `-` already leads a `[-]` progress cell in this same
list. *Alternative considered:* keep `v`/`>` and move the cursor to a different glyph. Rejected —
that changes every row of the list to fix a collision on two of them.

**D7 — `RegionHeading` carries `DIM`, which is the first modifier `view-palette`'s table has
ever changed.** An unfocused **heading** is text a reader can mistake for content, where an
unfocused border was a line nobody read. `RegionHeadingFocused` keeps `RegionBorderFocused`'s
`BOLD` unchanged, and no other row of that table moves. `DetailHeader` is removed rather than
kept beside `RegionHeadingFocused`: two roles identical in everything but name would let the two
regions' headings drift for no reason a reader could see.

**D8 — No acceptance test.** Stated in Test Strategy above rather than left implicit.

## Risks / Trade-offs

- **The wide detail interior now runs to the frame's last column, so a right-aligned cell touches
  the pane edge.** → Accepted deliberately (D5) and pinned: `responsive-layout`'s
  "Rows do not overwrite the borders at either width" asserts that column 119 *does* carry content
  at the wide layout, so a future change that quietly re-adds a trailing gutter fails there.
- **The narrow region keeps both gutters while the wide detail region does not, so list content
  shifts by one column across the 100-column breakpoint.** → Accepted. Each layout is internally
  consistent, the breakpoint is rarely crossed mid-session, and the alternative is an inconsistency
  that costs a mandated width instead.
- **`▾` and `▸` are not ASCII, and this crate's fixtures are otherwise all ASCII.** → `COLWIDTH`
  already forbids `char`-count measurement across the pure view set, and the glyphs go through
  `layout::columns` like every other cell; `change-rows`' own display-column requirement covers
  them.
- **The divider `│` and the rule `─` *are* East Asian Ambiguous, and this change is the first
  to write them from its own code.** → Accepted, and named here because D6's width argument is
  about the fold glyphs only and reads as though it covered every glyph the change adds. It does
  not: `U+2502` and `U+2500` are `A`, where `▾`/`▸` are `N`. `SPEC.md` → § List view names
  Ambiguous box-drawing characters as this project's standing uncompensated exposure, and the
  exposure is not **new** — `Block::bordered()` drew the same two code points at the same cells
  — but it moves from ratatui's widget into this crate's own draw calls, so this change owns it.
  No compensation is added, for `SPEC.md`'s stated reason: any would break the terminal it
  guessed wrong for.
- **Eleven capability specs change at once, so a half-applied implementation leaves the tree
  self-contradictory.** → The three signature changes in `ui::layout` are compile errors at every
  call site, so a partial application does not build. The task order below finishes the geometry
  before anything reads it.
- **Four documentation sites go stale the moment the first task lands, and only one of them
  fails.** → `tests/degraded_coverage.rs` fails loudly and names both sides; `SPEC.md`'s mouse
  table prose, `SPEC.md` § List view's fold glyphs, and four capabilities' `## Purpose` drift
  with nothing watching, per Test Strategy above. They are therefore carried as **named task
  lines** rather than trusted to a gate, which is the only mechanism available for a claim no
  test binds.
- **`scripts/gates/widths.sh` has eight tests of headroom and this change spends most of it.**
  → It requires *every* `#[test]` in `src/ui/view.rs` to name both `60` and `120` (`:32`) above
  a floor of 114, and the file holds 122 today. Task 3.2 deletes the header tests and task 3.5
  adds a divider test that is naturally 120-only. Both legs are pinned in tasks.md → 3.6, and
  the divider test must assert at 60 as well as 120 or the gate fails on arrival.

## Migration Plan

None. Nothing is persisted, no data shape changes, and the plugin manifest is untouched, so the
deploy is `make build` and a pane reload. Rollback is `git revert` — there is no state written in
the new shape that the old code could not read, because there is no state.

## Visual Design

This change modifies a user-facing view and a design source exists for it: the column-exact
prototype named in Context. It is **not** imported into `design/` as an asset, because it is not
markup this change ships — it is a model of a terminal character grid, and the artifact it drives
is `ui::view`'s buffer rather than a rendered page. The spec deltas are the imported form of it:
every cell position the prototype settles is written down as a scenario assertion.

**Design ↔ contract reconciliation.** The prototype diverges from the specified contract in three
places, and the specs win in all three:

| Prototype | Contract | Resolution |
|---|---|---|
| Renders a fixed five-tab bar and a fixed change list | The real bar is windowed by `artifact-tabs` and the real list is sliced by `list-selection` | Prototype is illustrative; the windowing and slicing rules are unchanged by this change and untouched by the deltas |
| Offers four divider modes and two padding toggles | Exactly one arrangement is specified | The alternatives are recorded in D5 and D2 as rejected, not shipped |
| Dims artifact tabs whose file is unwritten | No such distinction exists in `ArtifactRef` | Out of scope; proposed separately as `artifact-presence`, and named as a Non-Goal above |

The prototype discloses nothing security-sensitive: it renders this repository's own public change
names and a fabricated `/tmp/demo-repo` path.

## Open Questions

None. The three that were open — how much vertical separation the detail region gets, which side
of the divider gets a blank column, and whether the leading or trailing gutter survives — were
each resolved by looking at the prototype and are recorded as D2 and D5.
