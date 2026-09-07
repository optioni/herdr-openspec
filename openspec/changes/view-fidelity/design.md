## Context

`ui::view::render` draws every line through `ratatui::buffer::Buffer::set_string`. That
function walks the string's **grapheme clusters**, drops clusters holding a control
character, and consumes `cluster.cell_width()` cells for each — one for most, two for a CJK
ideograph or an emoji — and it clips at the **buffer's** right edge, never at the region's.
Every width computation in `src/ui/` counts `char`s instead. For ASCII the two agree exactly,
which is why 200,000 randomised sources and ~1,500 adversarial dashboards found no panic and
no overflow: the fixtures were ASCII, and against ASCII the arithmetic is right.

Against anything else it is not, and the failure is silent and destructive. A twenty-column
name budgeted at ten columns runs ten columns past its region, over the neighbouring block's
border and into the neighbouring region's content. In the detail pane the guard that is meant
to stop this — `if x >= last_col { break }` in the per-segment loop — advances `x` by
`chars().count()`, so it under-counts by exactly the amount that would have made it fire.

This repository cannot see it. The distinct non-ASCII set over every tracked file under
`openspec/` is exactly thirty codepoints —
`— → … – ─ ░ █ │ ┌ − × ↔ ┐ ✳ └ ┘ ∪ ¶ ⁴ ✓ ≤ ≥ ⇒ ☑ ☐ ● ○ ∈ ∅ §` — every one measured at width 1
under `unicode-width 0.2.2`, and every change directory name is ASCII. (An earlier draft put a
file count on that claim; the count was stale within a day of being written, and the set is the
part that carries the argument.) The pane renders whatever repository it is pointed at.

Three smaller findings sit in the same family. Two literals, `No content yet` and
`No tasks yet`, are pushed as bare `String`s while every neighbouring line goes through
`pad_or_truncate_right`, so at frames of 15 and 13 and narrower — content areas of 13 and 11,
the frame less the region's two border columns — they eat the region's right border — a range the `DETAILWIDTHS` gate's mandated 58 and 78 cannot reach. `Action::OpenDetail`
resets `detail.scroll` unconditionally, so at any width above the 100-column breakpoint —
where both regions are always drawn — `Enter` moves no route and its only effect is to throw
away the reader's scroll position. And `ui::app::matches` folds case with
`to_ascii_lowercase`, so `/Ä` does not find `änderung`.

Constraints this design works inside, all of them landed: views are pure functions from state
to a ratatui frame and do no I/O; `pulldown_cmark` is named only in `src/ui/markdown.rs` and
that module names no `ratatui` type; nothing under `src/ui/` blocks or reads a clock; the list
region's mandated interiors are 38 and 58 and the detail region's are 78 and 58; the pure view
set is exactly eight files, a count `dashboard-loop` owns; `scripts/gates/deps.sh` asserts
exactly six direct dependencies, a claim `plugin-build` and `quality-gates` own and this change
does not touch.

## Goals / Non-Goals

**Goals:**

- One measure of a rendered length in the crate, in terminal display columns, agreeing with
  what `Buffer::set_string` consumes — by construction, not by coincidence.
- No line the view produces ever exceeds the region it is drawn into, at any width, for any
  input, including the two narrow-frame literals no mandated-width test can see.
- The panic-freedom the audit measured is preserved and re-proved, not disturbed.
- The pane's Unicode promise written down: what it measures, what it does not, and which way
  it errs when a terminal disagrees with ratatui.
- `Enter` at the detail route stops discarding scroll; the filter finds what the list shows.

**Non-Goals:**

- No new panic-hardening. The audit found nothing to harden.
- No change to the mandated interior widths (38/58, 78/58) or to any gate's mandated pair.
- No new dependency, no seam moved, no I/O in a view, no clock in a view.
- No bidirectional reordering, no Unicode normalisation, no locale-tailored case mapping, no
  terminal capability probing.
- No change to the `Change` type; neither `from_files` nor `from_cli` is touched.

## Boundaries

| Module | What changes | Pattern it follows |
|---|---|---|
| `src/ui/layout.rs` | Gains `columns` and `truncate_columns`, the crate's only display-width measure. | The module already holds the crate's pure geometry (`split_frame`, `interior`, `scroll_offset`) and already names a `ratatui` type. |
| `src/ui/list.rs` | `pad_or_truncate_right`, `shorten_left`, `shorten_left_row`, `row` assembly, the problem and message rows, and the no-repository block measure in columns. `pad_or_truncate_right` gains a trailing pad after truncation. | Unchanged shape: still returns plain `String`s, still the crate's one right-truncation implementation shared with `ui::detail`. |
| `src/ui/markdown.rs` | Wrap, hard-split, prefix budget, and hanging indent measure in columns through `layout::`. | Still names no `ratatui` type and no `char` count; `layout::` is the seam, exactly as `ui::list::pad_or_truncate_right` already is for `ui::detail`. |
| `src/ui/tasks.rs` | Item wrap, heading line, and `No tasks yet` measure and truncate in columns; `No tasks yet` goes through `pad_or_truncate_right`. | Same plain-data return, same `Face::plain()` single segment. |
| `src/ui/detail.rs` | `header_row`'s three cells and their band boundaries in columns; `content_lines`' `No content yet` goes through `pad_or_truncate_right`. | The literal now takes the path every neighbouring line already takes. |
| `src/ui/view.rs` | The per-segment draw loop advances by `layout::columns` and clamps each segment with `layout::truncate_columns`; the header's `A` arithmetic and footer hint budget in columns. | The loop keeps its `x >= last_col` guard; the clamp is belt to its braces. |
| `src/ui/app.rs` | `Action::OpenDetail` guards its scroll reset on `route != Route::Detail`; `matches` folds with `to_lowercase`. | The guard is the `before != after` shape `Back` and `FilterStart` already use. |
| `scripts/gates/colwidth.sh` (new) | Sweeps the seven non-`layout` pure files for `char`-count measurement, with `src/ui/layout.rs` as its positive control. | The tree-wide-grep-with-a-positive-control shape of `NOSPAWN-GREP`, `NORAW-GREP`, `MDSEAM`, and `READSEAM`. |
| `Makefile`, `tests/ci_workflow.rs` | `colwidth.sh` joins the `gates:` recipe, run bare with its own script default. | Every gate's floor is its own script default; `ci_workflow.rs` already asserts recipe-versus-directory both ways. |

Not touched: `src/cli.rs`, `src/agents.rs`, `src/launch.rs`, `src/open.rs`, `src/watch.rs`,
`src/refresh.rs`, `src/changes.rs`, `src/schema.rs`, `src/tasks.rs`, `src/state.rs`,
`Cargo.toml`, `herdr-plugin.toml`, `tests/fixtures/build-graph.txt`.

## Contracts

Every signature this change touches is `pub(crate)` or private. `ui::layout::columns` and
`ui::layout::truncate_columns` are new `pub(crate)` functions. `pad_or_truncate_right`,
`shorten_left`, `shorten_left_row`, `header_row`, `content_lines`, `progress_bar`, `ui::tasks::lines`,
`ui::markdown::lines`, `ui::list::rows`, and `Dashboard::apply` keep their signatures exactly.

The binary's own contract — `herdr-openspec ui`, `open`, `open-tab`, the manifest, the config
format, the keybindings — is **unchanged**. `Enter` keeps its binding and its meaning; only
the side effect it had when it moved nothing is removed. **Additive, not breaking.** No
separate consumer exists: the plugin manifest is the only external surface and it is not
edited.

## Persistence and Rollout

- **Migration:** none. No persisted format changes.
- **Backfill:** none.
- **Seeding:** none.
- **Cache invalidation:** none. `Dashboard::sync_detail`'s `(change directory, tab)` cache key
  is untouched; the change alters how a cached source is *rendered*, not when it is re-read.
- **Index rebuild:** none.
- **Authorization:** none. The pane is read-only and this change adds no write; the plugin's
  writes remain exactly `agent-names.toml` under `HERDR_PLUGIN_STATE_DIR`.
- **Observability:** none beyond the existing problem rows. No new problem is produced: a
  wide character is not a degraded state, it is content.
- **Deployment:** `make build` produces the same single binary; the pane picks it up on next
  open. No manifest edit, so `herdr plugin link .` need not be re-run.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| Terminal (raw mode, alternate screen) | **replaced** — `ratatui::backend::TestBackend`; the real `src/ui/terminal.rs` is never reached, because `cargo test` spawns this binary and a real mode switch corrupts the developer's session | replaced (same) |
| ratatui `Buffer` / `set_string` | **real** — this is the collaborator under test; `columns` is asserted against what a real `Buffer` consumed | real |
| Filesystem (artifact read) | **replaced** — the injected `&dyn Fn(&Path) -> Result<String, String>` reader, per `artifact-content` | replaced |
| Filesystem (repository walk) | **replaced** — `Dashboard` values are constructed directly from `changes::fixture`; no scenario here needs a tree | replaced |
| `openspec` binary | **replaced** — no scenario reaches `OpenspecCli`; every `Dashboard` is built in memory | replaced |
| Herdr socket / `herdr` binary | **replaced** — no scenario reaches `HerdrCli`; badged rows use an in-memory `AgentSnapshot` | replaced |
| Process environment | **replaced** — the injected `&dyn Fn(&str) -> Option<String>` lookup; no test reads or writes the real environment | replaced |
| Clock | **not used** — nothing here reads one, and `NOBLOCK` keeps it that way | not used |
| Filesystem watcher (`notify`) | **replaced** — `FsEvents` trait object; no scenario drives one | replaced |
| Refresh worker (`refresh::Refresher`) | **replaced** — trait object in `run_loop`'s `Live`; the loop-tier `Enter` scenario supplies one that answers nothing | replaced |
| Agent poller (`agents::AgentPoll`) | **replaced** — trait object in `Live`; badged-row scenarios supply an in-memory `AgentSnapshot` rather than reaching `HerdrCli` | replaced |
| Launcher (`launch::Launcher`) | **replaced** — trait object in `Live`. Named explicitly because it is the one collaborator that blocks up to thirty seconds (`herdr agent start`); no scenario here presses `a`, `c`, `s`, or `g`, and none may | replaced |
| Event source (`EventSource`) | **replaced** — the scripted press sequence the loop-tier row's Collaborators column names; the real crossterm source is never constructed | replaced |
| Rust source tree (gate scans) | **real** — `colwidth.sh` and `tests/ci_workflow.rs` read the checked-in files, which is the point of a source sweep | real |
| `Makefile` / `scripts/gates/` | **real** — read by `tests/ci_workflow.rs` | real |

**One thing no tier in this plan can observe, stated rather than left implicit.** Every render
assertion reads a `TestBackend` buffer filled by the same `Buffer::set_string` the measure is
defined against, and `columns` agrees with that function by construction. So the plan can
prove the pane never exceeds a region *as ratatui measures it*, and cannot prove anything
about a terminal that shapes a cluster differently from ratatui — the accepted limit
`responsive-layout`'s Unicode promise now names, and the reason Decision 7 stops where it
does. There is no tier that would buy that evidence: it would need a real terminal emulator
with a real font, which is neither testable in `cargo test` nor stable across machines.

**This change takes no outer-loop acceptance test that spawns a process or touches a real
terminal.** Every behaviour it specifies is observable in a `TestBackend` buffer or in a pure
function's return value, and the two seams that would justify an outer loop — the `openspec`
binary and the Herdr socket — are not on any path it touches. `ui::driver::run_loop` is
exercised where a scenario names it (the `Enter` sequences), driven by a scripted event source
against a `TestBackend`, which is the tier `dashboard-loop` already established for it.

## Test Strategy

Tiers, per the project's own: **unit** = `cargo test --lib` over a pure function; **view** =
`cargo test --lib` rendering into a `TestBackend` at the mandated widths; **loop** =
`cargo test --lib` driving `run_loop` with a scripted event source; **gate** = a script under
`scripts/gates/` invoked by `make gates`, plus `tests/ci_workflow.rs` inside `cargo test`.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| `columns` agrees with what the buffer consumed | `layout::columns` vs. a real `Buffer` write, seven strings | unit | real `Buffer` | `cargo test --lib ui::layout::tests::columns` |
| `truncate_columns` never splits a cluster and never overruns | sweep over two strings at every `max` | unit | none | `cargo test --lib ui::layout::tests::truncate` |
| Nothing under `src/ui/` measures in characters except the primitives | `scripts/gates/colwidth.sh`, plus `tests/ci_workflow.rs` proving it is wired | gate | real source tree | `make gates` / `cargo test --test ci_workflow` |
| A path that fits is right-aligned whole at both widths | header render at 60 and 120 | view | `TestBackend` | `cargo test --lib ui::view::tests::header` |
| A path too long for the narrow header is shortened from the left | header render at 60 and 120 | view | `TestBackend` | `cargo test --lib ui::view::tests::header` |
| A header too narrow for any path shows only the label | header render at 16, 60, 120 | view | `TestBackend` | `cargo test --lib ui::view::tests::header` |
| No repository found is named in the header at both widths | header render at 60 and 120 | view | `TestBackend` | `cargo test --lib ui::view::tests::header` |
| The badge is drawn dim after the label at both widths | header render + style read at 60 and 120 | view | `TestBackend` | `cargo test --lib ui::view::tests::header` |
| A false flag renders the header that landed before this change | byte-identical buffer comparison | view | `TestBackend` | `cargo test --lib ui::view::tests::header` |
| The badge takes its columns from the path, not from the label | header render at 60 and 120 | view | `TestBackend` | `cargo test --lib ui::view::tests::header` |
| A header too narrow for the badge drops it whole | header render at 17, 18, 60 | view | `TestBackend` | `cargo test --lib ui::view::tests::header` |
| A wide-character path is shortened by columns and stays inside the header | header render at 1, 16, 18, 19, 60, 120, badged and not | view | `TestBackend` | `cargo test --lib ui::view::tests::header_wide` |
| A CJK change name stays inside the list region at both mandated widths | full-frame render at 60 and 120, border cells asserted | view | `TestBackend` | `cargo test --lib ui::list::tests` |
| An emoji change name at 58 columns does not overwrite the border | full-frame render at 60 and 120, badged and not | view | `TestBackend` | `cargo test --lib ui::list::tests` |
| A wide name is truncated whole and padded back to the full width | `ui::list::rows` at 38 and 58 | unit | none | `cargo test --lib ui::list::tests` |
| Rows are total over adversarial names at every width | sweep 0..=130 over six names | unit | none | `cargo test --lib ui::list::tests::total` |
| The no-repository block shortens its search path by columns | full-frame render at 60 and 120 | view | `TestBackend` | `cargo test --lib ui::list::tests` |
| A paragraph is word-wrapped, differently at the two mandated widths | `markdown::lines` at 58 and 78 | unit | none | `cargo test --lib ui::markdown::tests` |
| An empty source and a zero width each produce no lines | four calls | unit | none | `cargo test --lib ui::markdown::tests` |
| No line exceeds the width it was given | sweep over eight widths | unit | none | `cargo test --lib ui::markdown::tests` |
| A wide-character document wraps by columns at both mandated widths | `markdown::lines` at 58 and 78, byte-offset re-slice | unit | none | `cargo test --lib ui::markdown::tests::wide` |
| Rendering is total over arbitrary input | ten sources at widths 1, 2, 58, 78 | unit | none | `cargo test --lib ui::markdown::tests::total` |
| A missing artifact still shows its tab and reads `No content yet` | full-frame render at 60 and 120 | view | `TestBackend`, injected reader | `cargo test --lib ui::detail::tests` |
| `No content yet` does not eat the border at a narrow frame | full-frame render at 1, 2, 13, 14, 15 | view | `TestBackend`, injected reader | `cargo test --lib ui::detail::tests::narrow` |
| A read failure is named above the content at both widths | full-frame render at 60 and 120 | view | `TestBackend`, injected reader | `cargo test --lib ui::detail::tests` |
| The rendered markdown fills the content area, not the whole interior | full-frame render at 60 and 120 | view | `TestBackend`, injected reader | `cargo test --lib ui::detail::tests` |
| The tracked-tasks tab renders the checklist body instead | full-frame render at 60 and 120 | view | `TestBackend`, injected reader | `cargo test --lib ui::detail::tests` |
| A wide-character document stays inside the detail region | full-frame render at 60 and 120, border cells asserted | view | `TestBackend`, injected reader | `cargo test --lib ui::detail::tests::wide` |
| `content_lines` is total and width-parameterised | matrix of seven `Detail` x four `change` at 58 and 78 | unit | none | `cargo test --lib ui::detail::tests::total` |
| No `content_lines` line exceeds its width at any width | sweep 0..=130 over the same matrix | unit | none | `cargo test --lib ui::detail::tests::sweep` |
| A checklist of wide-character items fits at both mandated widths | `ui::tasks::lines` at 58 and 78, byte-offset re-slice | unit | none | `cargo test --lib ui::tasks::tests::wide` |
| No checklist line exceeds its width at any width | sweep 0..=130 over five sources x two `Progress` | unit | none | `cargo test --lib ui::tasks::tests::sweep` |
| A prose-only tasks file reads `No tasks yet` | full-frame render at 60 and 120 | view | `TestBackend`, injected reader | `cargo test --lib ui::tasks::tests` |
| `No tasks yet` does not eat the border at a narrow frame | full-frame render at 1, 2, 13, 14, 15 | view | `TestBackend`, injected reader | `cargo test --lib ui::tasks::tests::narrow` |
| A missing tasks artifact still reads `No content yet` | full-frame render at 60 and 120 | view | `TestBackend`, injected reader | `cargo test --lib ui::tasks::tests` |
| A read failure on the tasks tab names its reason and renders no checklist | full-frame render at 60 and 120 | view | `TestBackend`, injected reader | `cargo test --lib ui::tasks::tests` |
| The full grammar at both mandated interior widths | `progress_bar` at 58 and 78 | unit | none | `cargo test --lib ui::tasks::tests::bar` |
| The bar reaches the buffer at both mandated frame widths | full-frame render at 60 and 120 | view | `TestBackend`, injected reader | `cargo test --lib ui::tasks::tests::bar` |
| The percentage truncates rather than rounds | `progress_bar` at 58 and 78, four `Progress` | unit | none | `cargo test --lib ui::tasks::tests::bar` |
| The bar measures at most its width at every width | sweep 0..=130 over four `Progress` | unit | none | `cargo test --lib ui::tasks::tests::bar_sweep` |
| A CJK change name keeps the header inside its region at both mandated widths | `header_row` at 58 and 78 | unit | none | `cargo test --lib ui::detail::tests::header` |
| The header reaches the buffer without crossing the region border | full-frame render at 60 and 120, border cells asserted | view | `TestBackend` | `cargo test --lib ui::detail::tests::header` |
| The header is total over adversarial names at every width | sweep 0..=130 over five names | unit | none | `cargo test --lib ui::detail::tests::header_sweep` |
| At the detail route the content scrolls by one line at both widths | `apply` + render at 60 and 120 | view | `TestBackend` | `cargo test --lib ui::app::tests::scroll` |
| At the list route the same actions still move the selection | `apply` + render at 60 and 120 | view | `TestBackend` | `cargo test --lib ui::app::tests::scroll` |
| Scrolling stops at the top | `apply` x4 + render at 60 and 120 | view | `TestBackend` | `cargo test --lib ui::app::tests::scroll` |
| While filtering, `j` and `k` still type into the query | `action_for` + `apply` | unit | none | `cargo test --lib ui::app::tests::filter` |
| Every route move resets the scroll | `apply` sequences + render at 60 and 120 | view | `TestBackend` | `cargo test --lib ui::app::tests::route` |
| `Enter` at the detail route moves nothing and keeps the scroll | `apply` x3 + byte-identical buffers at 60 and 120 | view | `TestBackend` | `cargo test --lib ui::app::tests::enter_noop` |
| `Enter` at the detail route moves nothing and keeps the scroll | scripted `run_loop` press sequence at 120x20 | loop | `TestBackend`, scripted events | `cargo test --lib ui::driver::tests::enter_noop` |
| `Enter` from the list route still opens at the top | `apply` + render at 60 and 120 | view | `TestBackend` | `cargo test --lib ui::app::tests::enter_opens` |
| `Enter` while filtering still dismisses the filter and resets nothing | `apply` at both routes | unit | none | `cargo test --lib ui::app::tests::enter_filter` |
| A query narrows both tiers at both widths | full-frame render at 60 and 120 | view | `TestBackend` | `cargo test --lib ui::app::tests::matches` |
| Matching ignores case | four renders at 60 and 120 | view | `TestBackend` | `cargo test --lib ui::app::tests::matches` |
| Matching ignores case outside ASCII | four renders at 60 and 120, plus direct `matches` calls | view | `TestBackend` | `cargo test --lib ui::app::tests::matches_unicode` |
| A query matching only an archived change | full-frame render at 60 and 120 | view | `TestBackend` | `cargo test --lib ui::app::tests::matches` |
| A query matching nothing names itself | full-frame render at 60 and 120 | view | `TestBackend` | `cargo test --lib ui::app::tests::matches` |
| The fold is total and its documented edge cases hold | seven direct `matches` calls | unit | none | `cargo test --lib ui::app::tests::matches_total` |
| Shrinking the visible list clamps the selection | `apply` sequence + render at 60 and 120 | view | `TestBackend` | `cargo test --lib ui::app::tests::matches` |
| The separator and archived rows render at both mandated widths | carried-forward render at 38 and 58, assertions restated in columns | view | `TestBackend` | `cargo test --lib ui::list::tests` |
| An archived change carries a badge in the same column as an active one | carried-forward render at 38 and 58 | view | `TestBackend` | `cargo test --lib ui::list::tests` |
| An archived row drops the progress cell, then the date, as the width falls | carried-forward drop-order sweep, restated in columns | unit | none | `cargo test --lib ui::list::tests` |
| No archived changes means no separator | carried-forward render at 38 and 58 | view | `TestBackend` | `cargo test --lib ui::list::tests` |
| No repository names the directory searched, at both widths | carried-forward render at 38 and 58 | view | `TestBackend` | `cargo test --lib ui::list::tests` |
| A repository with no changes at all | carried-forward render at 38 and 58 | view | `TestBackend` | `cargo test --lib ui::list::tests` |
| No active changes with archived ones still browsable | carried-forward render at 38 and 58 | view | `TestBackend` | `cargo test --lib ui::list::tests` |
| Repository-level problems are named above the rows | carried-forward render at 38 and 58 | view | `TestBackend` | `cargo test --lib ui::list::tests` |
| The full header grammar at both mandated interior widths | carried-forward `header_row` at 58 and 78, assertions restated in columns | unit | none | `cargo test --lib ui::detail::tests::header` |
| A change with no tasks still ends its row in the same column | carried-forward `header_row` at 58 and 78 | unit | none | `cargo test --lib ui::detail::tests::header` |
| A long name is truncated with an ellipsis, never overflowing the row | carried-forward `header_row` at 58 and 78 | unit | none | `cargo test --lib ui::detail::tests::header` |
| The cells are dropped whole in order as the row narrows | carried-forward band sweep at nine widths | unit | none | `cargo test --lib ui::detail::tests::header` |
| An empty schema name is a cell of two characters, not an absent one | carried-forward `header_row` at 58 and 78 | unit | none | `cargo test --lib ui::detail::tests::header` |
| A fenced code block's lines are reproduced verbatim | carried-forward `markdown::lines` at 58 and 78 | unit | none | `cargo test --lib ui::markdown::tests` |
| A code line longer than the interior is hard-split rather than word-wrapped | carried-forward hard-split, now cut at a cluster boundary in columns | unit | none | `cargo test --lib ui::markdown::tests` |
| An indented code block renders the same as a fenced one | carried-forward `markdown::lines` at 58 and 78 | unit | none | `cargo test --lib ui::markdown::tests` |
| A raw HTML block renders verbatim rather than being dropped | carried-forward `markdown::lines` at 58 and 78 | unit | none | `cargo test --lib ui::markdown::tests` |
| Groups, headings, items, and separators at both mandated widths | carried-forward `ui::tasks::lines` at 58 and 78, assertions restated in columns | unit | none | `cargo test --lib ui::tasks::tests` |
| A nested item reproduces its own indent | carried-forward `ui::tasks::lines` at 58 and 78 | unit | none | `cargo test --lib ui::tasks::tests` |
| A long item wraps with a hanging indent at both widths | carried-forward wrap, now measured in columns | unit | none | `cargo test --lib ui::tasks::tests` |
| An unbreakable word is hard-split rather than lost | carried-forward hard-split, now cut at a cluster boundary | unit | none | `cargo test --lib ui::tasks::tests` |
| The indent is dropped whole as the width collapses | carried-forward drop-order sweep | unit | none | `cargo test --lib ui::tasks::tests` |
| A heading with no items still renders its heading | carried-forward `ui::tasks::lines` at 58 and 78 | unit | none | `cargo test --lib ui::tasks::tests` |
| A headingless leading group renders without a heading line | carried-forward `ui::tasks::lines` at 58 and 78 | unit | none | `cargo test --lib ui::tasks::tests` |


Every landed test in the touched modules is re-run unchanged except for the unit its width
assertions are stated in: `assert_eq!(row.text.chars().count(), width)` becomes
`assert_eq!(layout::columns(&row.text), width)`. For ASCII fixtures the two are equal, so
that rewrite must not change a single pass/fail — a rewrite that does has found a second bug
and is reported, not absorbed.

## Decisions

### 1. Measure through ratatui's own public API. No new dependency.

`ui::layout::columns` splits with `ratatui::text::Span::styled_graphemes` — public, and the
exact iterator `Buffer::set_stringn` uses, including its `!g.contains(char::is_control)`
filter — and sums each cluster through the public `ratatui::buffer::CellWidth` trait, whose
`impl CellWidth for str` is what `set_stringn` calls per cluster. The property bought is not
"a correct Unicode width"; it is **agreement with the buffer**, and this is the only way to
get it by construction.

*Alternative A — add `unicode-width` as a seventh direct dependency.* Genuinely cheap on its
face: `unicode-width v0.2.2` is already in `Cargo.lock` and in all four triples of
`tests/fixtures/build-graph.txt`, pulled in by ratatui, so declaring it adds zero crates and
the `GRAPH-SNAP` snapshot would not change by one line. But it is *not* what `set_stringn`
calls. ratatui's `str::cell_width` is `UnicodeWidthStr::width` **plus** a
terminal-compatibility adjustment for halfwidth katakana dakuten and handakuten (U+FF9E,
U+FF9F), and `set_stringn` additionally drops control clusters. Calling `unicode-width`
directly would therefore disagree with the buffer on exactly the inputs this change exists
to get right, and would agree only as long as ratatui's version and ours resolved to the same
node. It would also require deltas to `plugin-build`'s and `quality-gates`' "exactly six
dependencies" requirements and edits to `deps.sh` legs 2a, 2b, and 5 — and those two
capabilities are owned by other changes. Rejected on correctness first, cost second.

*Alternative B — hand-roll the width function.* Rejected. It means vendoring East Asian Width
tables that must be re-derived on each Unicode revision, in a crate whose stated discipline is
that one module owns one external concern. An approximation ("CJK ranges only") is silently
wrong for emoji, and the audit's own live example is an emoji.

*Alternative C — `Span::width()`.* Public and one line, but it is `UnicodeWidthStr::width`
of the whole string — Alternative A's measure with an extra allocation-free wrapper, without
the per-cluster filter or the katakana adjustment. Rejected for the same reason as A.

**Cost of the chosen route, stated:** `columns` allocates nothing but iterates graphemes, so
it is O(n) per call where a `char` count was O(n) too. `pad_or_truncate_right` calls it once
or twice per row. The list draws at most `height` rows and the detail at most `height` lines
per frame, and the loop is draw-then-wait — there is no hot path here to protect.

### 2. The primitives live in `ui::layout`, not in a new `ui::width` module.

A new file would make the pure view set **nine** files. That count is `dashboard-loop`'s
requirement and is asserted by `scripts/gates/noio-view.sh`'s `PURE` list, and this change
does not own `dashboard-loop`. `ui::layout` already holds the crate's pure geometry, already
names a `ratatui` type, and is already owned by `responsive-layout`, which this change does
own. The placement is a consequence of that ownership boundary and of the file count — not a
claim that measuring text is `Rect` geometry. Recorded here so a later change knows it may
move freely if it also updates `dashboard-loop`.

*Alternative — put them next to `pad_or_truncate_right` in `ui::list`.* Rejected: it would
make `ui::markdown` and `ui::tasks` import from the list module, which reads as a dependency
on the list's grammar rather than on a shared measure.

### 3. `pad_or_truncate_right` pads **after** truncating.

`truncate_columns` drops a grapheme cluster whole, so a truncation at `width - 1` can land at
`width - 2` when the last cluster is two columns wide. The landed contract is that the
function returns exactly `width`, and `change-rows`, `detail-header`, and `artifact-content`
all rely on it. Without a trailing pad a row whose name ends in a wide cluster would be one
column short of the interior and leave whatever the previous frame drew in that cell. So the
truncating arm is `truncate_columns(text, width - 1)` + `…` + pad-to-exactly-`width`.
`shorten_left` deliberately does **not** pad, exactly as it does not today — the header
right-aligns it inside its own remaining space — and `shorten_left_row` is the padded form
the no-repository block's third row uses. Both measure and cut in columns; the pair is named
here because the padded sibling is a third measuring site that reads like a wrapper.

### 4. The mandated widths do not change. All-widths sweeps close the narrow gap instead.

The brief asked whether `DETAILWIDTHS` should gain a narrow case. **No.** The pairs 38/58 and
78/58 appear in the prose of nine capability specs and in the assertions of every row-grammar
test; adding a third mandated width would ripple through all of them, and a per-gate exemption
list is how a width check rots into a rubber stamp. The bug it would have caught is better
caught by a **property**: every line `content_lines` and `ui::tasks::lines` produce measures at
most `width` at every width from 0 to 130. That subsumes the 13-, 14-, and 15-column cases,
subsumes the next literal someone adds, and needs no change to any gate's mandated pair.
`tasks-progress-bar` already carries a sweep of exactly this shape, so the pattern is the
repository's own.

**What this does cost, corrected from an earlier draft that claimed it cost nothing.**
`LISTWIDTHS`, `MDWIDTHS`, `TASKWIDTHS`, `DETAILWIDTHS`, and `WIDTHS` require **every**
`#[test]` in their file to name both mandated widths as bare, unsuffixed literals, and each
script states in its own header that it has **no exemption list**. A sweep test over
`0..=130` names neither literal, so roughly eight of the new tests would fail their gate on
arrival. The remedy is the one `taskwidths.sh`'s own header already records for the existing
`0..=120` sweep: the sweep's spec scenario names the mandated pair explicitly among its swept
values, and the test names them as bare literals in the assertion it makes at those widths.
That is why each RED task below says every new test names its module's mandated pair, and why
groups 2, 3, and 4 now run their own width gate rather than discovering the collision at
`make gates` three to six groups later. No gate's floor is lowered, no exemption is added,
and the mandated pair is untouched.

**Gate floors move up, because they are floors.** `LIST_MIN` 32, `MD_MIN` 25, `TASK_MIN` 16,
`DETAIL_MIN` 32, and `WIDTHS_MIN` 98 are the current true measured counts. This change only
adds tests, so nothing goes red — but AGENTS.md's rule is that a gate's default *is* its true
measured floor, not a value that happens to pass, so each is re-measured and re-defaulted once
the tests land (task 8.6).

### 5. `Enter` at the detail route is a **no-op**. This is a behaviour decision, and it is stated.

Three readings were available: keep the reset (status quo), guard it, or give `Enter` a second
meaning at the detail route. The guard wins, and not only because it matches the neighbouring
arms. `detail-scroll`'s landed requirement already says "**every action that moves `route`**
SHALL reset `detail.scroll`" — the implementation fired on an action that moved none, so the
requirement was already right and the code was already wrong against it. Giving `Enter` a new
meaning would be a keybinding change, which the proposal rules mark **BREAKING**, and there is
no demand for one. So: `Enter` at the detail route changes nothing, in state or on screen.

### 6. The filter folds with `str::to_lowercase`, and stops there.

`to_lowercase` is std, allocates two `String`s per call — which `matches` already did — and
applies the Unicode **default** mapping. The pane does not normalise (`ä` as U+00E4 and as
`a`+U+0308 are different directory names, and the list shows what the directory is called),
does not case-**fold** (`str::to_lowercase` is not `CaseFold`; the difference shows on Greek
final sigma and on `İ`, both recorded as scenarios so they read as known consequences), and
does not tailor to a locale. The documented ASCII promise is replaced rather than kept: a
filter that cannot find what the list is displaying is the failure, and the list will display
non-ASCII names correctly the moment Decision 1 lands.

The crate's **other** `to_ascii_lowercase`, at `src/state.rs:84`, is deliberately left alone.
That one derives a Herdr agent name, which `plugin-state` requires to match
`[a-z][a-z0-9_-]{0,31}` — an ASCII-only target where a Unicode fold would produce a name
Herdr rejects. Two call sites, two different jobs; only the filter's changes.

### 7. The promise is scoped to ratatui's measure, and the residual divergence is named, not compensated.

An earlier draft of this decision had the ZWJ case backwards, and the correction matters
because it changes what the pane can promise. Measured against ratatui 0.30.2:
`unicode_segmentation::graphemes(true)` treats a zero-width-joiner emoji sequence as **one**
cluster, and ratatui measures it at **2** columns — `🏳️‍🌈`, `👨‍👩‍👧‍👦`, and `🧑‍🚀` all
return 2. So ratatui already agrees with a terminal that composes the sequence. There is no
over-reservation to trade on.

The real divergence runs the other way: a terminal that does **not** compose the sequence
paints each constituent emoji, six columns for a three-person family, and the row overruns.
That is not detectable from a process writing bytes to a pty — it is a property of the
terminal's font and shaping — and any compensation would be a guess that breaks the composing
case. So the pane does not compensate. What it does instead is **state the boundary**:
`responsive-layout`'s Unicode promise now reads "never exceeds — as ratatui measures it — the
region it is drawn into", and names the non-composing terminal, along with the same limit's
other instance (East Asian **Ambiguous** characters, which a CJK-locale terminal paints at 2
where `unicode-width`'s default says 1 — and which includes several box-drawing and arrow
characters this repository's own artifacts already use).

This does not weaken the change. Before it, the pane overran its region for *every* wide
character in *every* terminal; after it, only for a cluster whose terminal disagrees with
ratatui's shaping. And the alternative — measure with something other than ratatui's own
function — makes the common case wrong to improve an uncommon one.

### 8. The draw loop keeps its guard **and** gains a per-segment clamp.

`x >= last_col { break }` is now correct, because `x` advances by consumed columns. The clamp
— drawing `truncate_columns(&segment.text, last_col - x)` — is added anyway, because the guard
fires *between* segments and a single segment wider than the space remaining would still cross
the border. Two mechanisms for one property, on the same terms the crate already asserts
`pulldown-cmark`'s defaults three ways: each alone is dodgeable.

### 9. One new gate, `COLWIDTH`, built the way every other gate here is built.

A tree-wide grep of the seven non-`layout` pure files' **production** lines for
`.chars().count()`, `.chars().take(`, and a `Vec<char>` `.chars().collect()`. The `Makefile`
runs it bare, per this repository's stated rule, and `tests/ci_workflow.rs` already asserts
the recipe names every script and vice versa, so it cannot silently drop out.

**The positive control runs the sweep's own pattern**, corrected from an earlier draft. That
draft controlled on `grep -q cell_width src/ui/layout.rs` — a *different* pattern from the
sweep's, which proves only that `cell_width` is spelled right and would let a corrupted sweep
regex print `COLWIDTH OK` over a tree full of violations. `NOSPAWN-GREP`, which AGENTS.md
names as the model, runs the *same* pattern against the file it excludes. Here there is no
file that must match once the change lands, so the control synthesises one: the script feeds a
line holding all three forms to its own pattern and fails if the pattern does not match all
three. The `layout.rs` check is kept beside it, demoted to what it actually proves — that the
measure being protected exists and the exemption is not vacuous.

It deliberately does **not** forbid `chars()` outright: `ui::markdown`'s per-character scanner
iterates characters for a purpose that is not measurement, and a gate that cannot tell those
apart would need an exemption list.

**Two measuring sites the pattern cannot see, recorded as a limit rather than papered over.**
`ui::markdown`'s `split_at_char` cuts a run at a `char_indices` offset — a column budget
expressed in characters that no `.chars()` pattern matches — and so would any future
`char_indices` cut. Widening the pattern to `char_indices` would catch legitimate
non-measuring uses and force the exemption list this gate exists without. What proves those
sites were rewritten is `markdown-render`'s wide-character and 500-column-CJK scenarios, named
in the verification matrix; the gate is a second line, not the only one.

## Risks / Trade-offs

- **[The `chars().count()` → `columns` rewrite silently changes an ASCII assertion.]** →
  Every touched test's fixtures are ASCII, where the two measures are equal, so no pass/fail
  may move. Task group 1 lands the primitives and their unit tests **before** any call site is
  rewritten, and each later group runs the full suite; a moved result is a second bug, reported
  rather than absorbed.
- **[A sweep from 0 to 130 at every width over a matrix of inputs is slow.]** → The sweeps run
  over pure functions returning `Vec<Line>`, not over renders; the widest matrix is 131 widths
  x 28 inputs. Measured before merging, and if a sweep exceeds a second it is narrowed by
  input, never by width — width is the axis that found the bug.
- **[`styled_graphemes` is ratatui-internal in spirit even though it is `pub`.]** → It is
  public API in `ratatui-core 0.1.2` and re-exported by `ratatui 0.30.2`, and it is confined
  to one function in one module. A future ratatui that removes it breaks one call site, which
  is exactly the blast radius the crate already accepts for `pulldown_cmark`.
- **[Over-reserving for ZWJ leaves visible trailing blanks on some terminals.]** → Accepted
  and written into the spec. The alternative is overwriting a border, which is the bug.
- **[Guarding `OpenDetail` could surprise a reader who presses `Enter` expecting a reset.]** →
  No such affordance was ever advertised; `r` already forces a full refresh and `Esc` then
  `Enter` still reopens at the top. The scenarios pin both.
- **[The dependency question is settled here but re-openable later.]** → Decision 1 records
  the exact reason `unicode-width` was rejected on *correctness*, not on cost, so a later
  change cannot reopen it by observing that the crate is already in the graph.

## Migration Plan

None needed. The change is source-only inside one binary: no persisted format, no manifest,
no config key, no keybinding, no dependency, no build-graph line. Deploy order is `make check`
then `make build`; the pane picks up the new binary on next open. Rollback is `git revert` of
the change's commits — nothing outside the repository is written and nothing outside it needs
undoing.

## Visual Design

Not applicable. The change modifies a terminal view, and no HTML or email design source exists
for it. The rendered grammars are specified cell-by-cell in the delta specs, which serve the
role a design asset would.

## Open Questions

None. The four findings each have a decided behaviour: display columns via ratatui's own API
(Decision 1), the two literals padded (Decision 4's sweep proves it at every width), `Enter` a
no-op at the detail route (Decision 5), and the filter folding all of Unicode (Decision 6). The
one thing deliberately left to a future change is whether the display-width primitives deserve
their own capability and their own module — recorded in Decision 2 with what it would cost.
