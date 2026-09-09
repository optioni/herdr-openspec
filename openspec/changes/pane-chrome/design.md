## Context

The dashboard's frame was designed in `tui-shell` against a full-screen terminal. The pane it
actually occupies is a Herdr split, and measured live at 60 columns that frame spends four of
its twenty rows and four of its sixty columns on chrome that says nothing: a header row whose
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
| `src/ui/detail.rs`, `src/ui/tasks.rs`, `src/ui/markdown.rs` | **nothing** — both mandated widths are unchanged, which is the point of the gutter arithmetic | — |

No file gains a filesystem, process, environment, network, or standard-I/O API, so
`NOIO-VIEW`'s nine-file `PURE` list is unchanged and needs no exemption. **No process spawn is
added anywhere**, inside `cli` or out; `NOSPAWN-GREP` and `LAUNCHSEAM` are untouched. No view
gains I/O: every new drawing decision is a pure function of `Dashboard` and a `Rect`.

`ui::layout::columns`/`truncate_columns` stay the crate's only width measure, so `COLWIDTH`'s
sweep is unaffected — the new `▾`/`▸` glyphs are measured through it like every other cell.

## Contracts

`ui::layout` is consumed only by `ui::view` and `ui::app`, both inside this crate. Three
signatures change and every change is a **compile error at each call site**, which is the
intended enforcement:

| Signature | Before | After |
|---|---|---|
| `split_frame(Rect)` | `(header, body, footer)` | `(body, footer)` |
| `interior(Rect)` | `Rect` | `interior(Rect, Gutters) -> Rect` |
| `split_detail(Rect)` | `(header, tabs, content)` | `(tabs, rule, content)` |

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

Two documentation sites are bound inside `cargo test` and must be updated with the code, or
`make check` goes red naming both sides: `SPEC.md`'s degraded-states row for the `file mode`
badge (`tests/degraded-coverage.toml`) and `SPEC.md`'s documented mouse bindings and confined
terminal-seam names (`tests/doc_contract.rs`).

### Verification matrix

One row per spec scenario, 149 in total.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| artifact-content → A missing artifact still shows its tab and reads `No content yet` | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::detail ui::view` |
| artifact-content → `No content yet` does not eat the border at a narrow frame | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::detail ui::view` |
| artifact-content → A read failure is named above the content at both widths | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::detail ui::view` |
| artifact-content → The rendered markdown fills the content area, not the whole interior | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::detail ui::view` |
| artifact-content → The tracked-tasks tab renders the checklist body instead | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::detail ui::view` |
| artifact-content → A wide-character document stays inside the detail region | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::detail ui::view` |
| artifact-content → `content_lines` is total and width-parameterised | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::detail ui::view` |
| artifact-content → No `content_lines` line exceeds its width at any width | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::detail ui::view` |
| artifact-tabs → The tab bar reaches the buffer at both mandated widths | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::detail ui::layout` |
| artifact-tabs → The bar, the rule, and the content never leave the interior | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::detail ui::layout` |
| artifact-tabs → `split_detail` is exact at its degenerate heights | call the pure function over its argument grid and compare whole `Rect`/`Option` values | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features ui::detail ui::layout` |
| change-rows → Active rows render at both mandated widths | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → A badged row carries its status between the name and the progress cell | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → An unattributed agent badges nothing | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → A watch problem leads the list, above a change-set problem | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → The row grammar places the marker, the name, and the progress cell | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → A name too long for the field is truncated with an ellipsis | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → A field too narrow for both drops the progress cell whole | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → No repository names the directory searched, at both widths | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → A repository with no changes at all | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → No active changes with archived ones still browsable | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → A collapsed but non-empty archive is not "no changes yet" | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → A collapsed active section shows its header and no message row | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → Repository-level problems are named above the rows | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → The detail region stays blank while the list fills | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → The narrow detail route draws no rows | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → More changes than rows do not overflow the region | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → A CJK change name stays inside the list region at both mandated widths | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → An emoji change name at 58 columns does not overwrite the border | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → A wide name is truncated whole and padded back to the full width | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → Rows are total over adversarial names at every width | call the pure function over its argument grid and compare whole `Rect`/`Option` values | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features ui::list ui::view` |
| change-rows → The no-repository block shortens its search path by columns | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → The section header and archived rows render at both mandated widths | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → An archived change carries a badge in the same column as an active one | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → A query against an unresolved archive counts from `archived_total` | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → A collapsed archived section shows its count and no rows | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → An expanded but unresolved archived section shows its header alone | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → An archived row drops the progress cell, then the date, as the width falls | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → A section header degrades by truncation at every width | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| change-rows → No archived changes means no archived header | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::view` |
| detail-header → The header names the selected change at both mandated widths | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::detail ui::view` |
| detail-header → Moving the selection moves the header | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::detail ui::view` |
| detail-header → An archived change's header carries its stripped name and its own schema | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::detail ui::view` |
| detail-header → An empty visible list leaves the whole detail interior blank | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::detail ui::view` |
| detail-header → A CJK change name keeps the header inside its region at both mandated widths | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::detail ui::view` |
| detail-header → The header reaches the buffer without crossing the region border | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::detail ui::view` |
| detail-header → The header is total over adversarial names at every width | call the pure function over its argument grid and compare whole `Rect`/`Option` values | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features ui::detail ui::view` |
| detail-header → The detail header is bold and uncoloured at both mandated widths | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::detail ui::view` |
| detail-scroll → The document fills the detail interior at both mandated widths | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::view ui::layout ui::app` |
| detail-scroll → Faces reach the buffer as styles at both widths | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::view ui::layout ui::app` |
| detail-scroll → A table reaches the buffer aligned and inside the region | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::view ui::layout ui::app` |
| detail-scroll → An empty source leaves the detail interior blank at both widths | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::view ui::layout ui::app` |
| detail-scroll → Content never overwrites the detail region's border | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::view ui::layout ui::app` |
| detail-scroll → A degenerate detail interior draws nothing and does not panic | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::view ui::layout ui::app` |
| detail-scroll → `scroll_offset` is exact at its boundaries | call the pure function over its argument grid and compare whole `Rect`/`Option` values | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features ui::view ui::layout ui::app` |
| detail-scroll → `interior` agrees with a bordered block's own inner rectangle | call the pure function over its argument grid and compare whole `Rect`/`Option` values | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features ui::view ui::layout ui::app` |
| detail-scroll → A scroll offset past the end still draws the last screenful | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::view ui::layout ui::app` |
| detail-scroll → At the detail route the content scrolls by one line at both widths | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::view ui::layout ui::app` |
| detail-scroll → At the list route the same actions still move the selection | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::view ui::layout ui::app` |
| detail-scroll → Scrolling stops at the top | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::view ui::layout ui::app` |
| detail-scroll → While filtering, `j` and `k` still type into the query | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::view ui::layout ui::app` |
| detail-scroll → Every route move resets the scroll | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::view ui::layout ui::app` |
| detail-scroll → `Enter` at the detail route moves nothing and keeps the scroll | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::view ui::layout ui::app` |
| detail-scroll → `Enter` from the list route still opens at the top | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::view ui::layout ui::app` |
| detail-scroll → `Enter` while filtering still dismisses the filter and resets nothing | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::view ui::layout ui::app` |
| list-filtering → A query narrows both tiers at both widths | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list` |
| list-filtering → Matching ignores case | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list` |
| list-filtering → Matching ignores case outside ASCII | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list` |
| list-filtering → A query matching only an archived change | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list` |
| list-filtering → A query matching nothing names itself | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list` |
| list-filtering → The fold is total and its documented edge cases hold | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list` |
| list-filtering → Shrinking the visible list clamps the selection | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::list` |
| list-filtering → A query reaches a match inside a folded archive | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list` |
| list-filtering → The archived count under a query is the matched count | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list` |
| list-filtering → The first character of a query requests the archive it needs | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list` |
| list-selection → A selection past the interior scrolls the slice at both widths | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::list ui::app` |
| list-selection → The last change is reachable and the slice stops at the end | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::app` |
| list-selection → The viewport is exact at its boundaries | call the pure function over its argument grid and compare whole `Rect`/`Option` values | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features ui::list ui::app` |
| list-selection → A resize changes the slice on the next frame | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::app` |
| list-selection → `Space` on a header folds and unfolds that section | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::list ui::app` |
| list-selection → `Space` inside a section folds it and moves the cursor to its header | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::list ui::app` |
| list-selection → An empty list makes `Space` inert | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::list ui::app` |
| list-selection → Opening an unresolved archive requests a refresh | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::app` |
| list-selection → A refresh does not undo a fold | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::app` |
| list-selection → A refresh keeps the cursor on the same change | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::list ui::app` |
| mouse-input → The two regions scroll independently at 120 columns | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::app ui::layout` |
| mouse-input → The wheel acts over a border and not over the chrome | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::app ui::layout` |
| mouse-input → At 60 columns only the routed region answers the wheel | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::app ui::layout` |
| mouse-input → Horizontal wheel events do nothing | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::app ui::layout` |
| mouse-input → A click selects a change row and a second click opens it | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::app ui::layout` |
| mouse-input → A click on a section header folds it exactly as `Space` does | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::app ui::layout` |
| mouse-input → A click on an archived header opens an unresolved archive and requests its refresh | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::app ui::layout` |
| mouse-input → A click on a tab cell switches to that artifact | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::app ui::layout` |
| mouse-input → Clicks that address nothing are inert | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::app ui::layout` |
| mouse-input → The other buttons and the non-press kinds are inert | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::app ui::layout` |
| responsive-layout → Body and footer occupy their rows at both widths | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → The action hints follow `Esc back` when the socket is reachable | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → The action hints are dropped whole, `g focus` first | drive `action_for`/`apply` over a scripted event, then render and assert cells | unit (`src/ui/app.rs`, `src/ui/driver.rs` inline) | `TestBackend` real; filesystem, Herdr socket and terminal replaced | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → A one-row frame renders the body's heading row and nothing else | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → A two-row frame renders one body row and the footer | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → A one-column frame renders without panicking | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → The footer drops whole hints rather than truncating one | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → The unattributed count is the footer's last hint at both widths | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → The count is reported with an empty change list | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → The count is dropped whole before the three key hints | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → The filter prompt replaces the count along with the hints | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → A region draws a heading, a blank row, and no border at both widths | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → `interior` reserves two rows and the gutters its `Gutters` names | call the pure function over its argument grid and compare whole `Rect`/`Option` values | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → The divider has a blank column on each side at 120 columns | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → The divider column is a width branch, not a constant | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → A one-, two-, and three-column frame degenerates without drawing over a gutter | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → The heading names the directory, not the path, at both widths | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → A name longer than the heading row keeps its tail | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → The badge is right-aligned and dropped whole | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → No repository names itself in the heading | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → The routed region's heading is bold and the other's is dim | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → No heading or rule cell carries a colour | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → Every colour literal still lives in the palette module alone | run the gate script against the tree and against its planted control | hygiene gate (`make gates`) + `tests/gate_controls.rs` | filesystem real (scratch copy), no process | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → At 120 columns both regions are drawn with the divider at column 40 | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → At 60 columns only the routed region is drawn | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → At 60 columns the detail route replaces the list region | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → The breakpoint is exact at 99, 100, and 101 columns | call the pure function over its argument grid and compare whole `Rect`/`Option` values | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → The mode follows the current frame, not the startup size | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → The routed region's border is bold and the other's is not | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → Interiors are blank at both widths | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → Rows do not overwrite the borders at either width | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → The detail interior is 78 columns at 120 and 58 at 60 | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → Every markdown test names both of its two interior widths | run the gate script against the tree and against its planted control | hygiene gate (`make gates`) + `tests/gate_controls.rs` | filesystem real (scratch copy), no process | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → The zones tile the frame at 120 columns | call the pure function over its argument grid and compare whole `Rect`/`Option` values | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → Below the breakpoint only the routed region has zones | call the pure function over its argument grid and compare whole `Rect`/`Option` values | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → The breakpoint is exact for the hit test too | call the pure function over its argument grid and compare whole `Rect`/`Option` values | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → Degenerate frames resolve without panicking | call the pure function over its argument grid and compare whole `Rect`/`Option` values | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features ui::layout ui::view` |
| responsive-layout → The hit test agrees with what was drawn | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::layout ui::view` |
| tasks-checklist → The tasks tab shows checkboxes and its siblings show markdown | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::tasks` |
| tasks-checklist → The tab is chosen by `tracks_tasks`, not by its id | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::tasks` |
| tasks-checklist → A schema naming no tasks artifact leaves every tab as markdown | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::tasks` |
| tasks-checklist → A `detail.tab` past the end of the artifact list renders no checklist | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::tasks` |
| tasks-checklist → A prose-only tasks file reads `No tasks yet` | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::tasks` |
| tasks-checklist → `No tasks yet` does not eat the border at a narrow frame | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::tasks` |
| tasks-checklist → A missing tasks artifact still reads `No content yet` | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::tasks` |
| tasks-checklist → A read failure on the tasks tab names its reason and renders no checklist | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::tasks` |
| view-palette → The palette answers every role with a `Style` | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::palette` · `make gates` |
| view-palette → The confinement gate catches a `Color` named outside the palette | run the gate script against the tree and against its planted control | hygiene gate (`make gates`) + `tests/gate_controls.rs` | filesystem real (scratch copy), no process | `cargo test --all-features ui::palette` · `make gates` |
| view-palette → The palette module reaches no I/O and measures no width | run the gate script against the tree and against its planted control | hygiene gate (`make gates`) + `tests/gate_controls.rs` | filesystem real (scratch copy), no process | `cargo test --all-features ui::palette` · `make gates` |
| view-palette → Each role's modifier set is exactly the table above | call the pure function over its argument grid and compare whole `Rect`/`Option` values | unit (`src/ui/*` inline `#[cfg(test)]`) | none — no I/O, no terminal, no process | `cargo test --all-features ui::palette` · `make gates` |
| view-palette → A monochrome reading of the frame is unchanged | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::palette` · `make gates` |
| view-palette → Faces reach the buffer as coloured styles at both mandated widths | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::palette` · `make gates` |
| view-palette → Heading foreground wins over a code span inside it | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::palette` · `make gates` |
| view-palette → A plain face is the default style | render into `TestBackend` at 120x20 and 60x20 and assert the named cells | unit (`src/ui/*` inline `#[cfg(test)]`) | `TestBackend` real; filesystem, process and terminal absent | `cargo test --all-features ui::palette` · `make gates` |

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
under `src/ui/` and in three gate scripts, so a column of whitespace would cost ~145 recomputed
expectations; (b) drop the leading gutter instead — rejected on looking at the prototype, the
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
- **Eleven capability specs change at once, so a half-applied implementation leaves the tree
  self-contradictory.** → The three signature changes in `ui::layout` are compile errors at every
  call site, so a partial application does not build. The task order below finishes the geometry
  before anything reads it.
- **`SPEC.md`'s frame diagram and two machine-bound documentation claims go stale the moment the
  first task lands.** → `tests/doc_contract.rs` and `tests/degraded_coverage.rs` fail loudly and
  name both sides; the tasks list updates them in the same group as the code they describe.

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
