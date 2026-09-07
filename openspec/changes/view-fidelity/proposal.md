## Why

An adversarial audit of the view layer — 200,000 randomised markdown and task sources across
widths 0–129, plus ~1,500 dashboards at 28 geometries from 1x1 to 300x60, all in debug mode
with `usize`/`u16` overflow checks live — found **zero** reachable panics, underflows, or
out-of-bounds accesses. The geometry and the `&str` slicing are clean.

What it did find is that **every width computation under `src/ui/` counts `char`s while the
terminal, and ratatui's own `Buffer::set_string`, count display columns.** `set_string`
truncates at the *buffer's* right edge, never at the region's, so a character wider than one
column renders past its budget and escapes the region it was drawn into. At 120x10 an active
change named `日本語の変更名前です` runs a list row from column 1 to about column 48: it
erases the list's right border at 39, the detail region's left border at 40, and overwrites
the detail header row, while its own `[4/9]` progress cell is pushed off-screen. At 60
columns an `emoji-🎉-change` row writes `[4/9]` over the right border. In the detail pane —
where the source is arbitrary user content — `ui::view::render`'s `x += chars().count()`
under-counts, so the `x >= last_col` guard that is supposed to stop the row never fires.

This is latent in *this* repository and live in any other. All 269 files under
`openspec/changes/` and `openspec/specs/` use only width-1 codepoints and every change
directory name is ASCII — but the pane renders whatever repository it is pointed at, so one
emoji in someone else's `proposal.md`, or a CJK change name, makes it real immediately.

Three smaller findings ride along, all in the same "characters the arithmetic does not model"
family: two literals that bypass the padding every neighbouring line goes through and eat a
border at narrow widths, an `Enter` that silently resets the detail scroll when it moves no
route, and a filter that folds only the ASCII half of the case table.

**This is unplanned work.** `openspec/IMPLEMENTATION-ORDER.md` ends at `degraded-states` and
the project is marked complete. The roadmap did not anticipate it because every view change
measured itself against ASCII fixtures at two mandated widths, and no row of the plan ever
asked what a column *is* — the mandated-width discipline made the arithmetic consistent with
itself rather than with the terminal.

## What Changes

- **Display columns replace `char` counts at every measuring and truncating site under
  `src/ui/`** — roughly twenty-five production sites across `list.rs`, `markdown.rs`,
  `tasks.rs`, `detail.rs`, and `view.rs`. Two new pure primitives in `ui::layout`,
  `columns(&str) -> usize` and `truncate_columns(&str, usize) -> &str`, become the crate's
  only measurement, checked by a source sweep the way `pulldown_cmark`'s confinement already
  is.
- **The measurement is ratatui's own, reached through its public API** —
  `Span::styled_graphemes` for the grapheme split and control-character filter, and the
  `CellWidth` trait for each cluster's width. This is byte-for-byte what `Buffer::set_string`
  consumes. **No dependency is added** (design.md → Decision 1).
- **The pane's Unicode promise is stated**: a grapheme cluster occupies the columns ratatui
  gives it, no line ever exceeds its region, and a terminal that renders a ZWJ sequence
  narrower than ratatui measures it leaves trailing blanks rather than overflowing.
- **`No content yet` and `No tasks yet` go through the same padding every neighbouring line
  uses**, and both bodies gain a width sweep proving no produced line exceeds its width at
  *any* width, not only at the two mandated ones.
- **`Enter` at the detail route is a no-op**, guarded on `route != Route::Detail` the way
  every other scroll-reset site in `apply` is guarded — it no longer throws away forty lines
  of scroll to move no route.
- **The filter folds case for the whole of Unicode**, not the ASCII half, so `/Ä` finds
  `änderung`.
- Not **BREAKING**: no plugin manifest, config-format, or keybinding change. `Enter`'s
  binding is unchanged; only its no-op case stops having a side effect.

## Non-Goals

- **No new panic-hardening.** The audit found none to do. Every change here is additive to
  the arithmetic and must leave the total, never-panicking character of these functions
  intact; the sweeps are written to prove that, not to assume it.
- **No change to the mandated interior widths.** 38/58 for the list and 78/58 for the detail
  region stay exactly as they are, and no gate's mandated pair grows a narrow case
  (design.md → Decision 4). The narrow bug is caught by an all-widths sweep instead.
- **No bidirectional text, no locale-tailored case folding, no terminal capability probing.**
  The pane measures; it does not reorder, and it does not ask the terminal what it can draw.
- No dependency added, no seam moved, no I/O introduced into a view, nothing written inside
  `openspec/`, no change authored, no orchestration across changes, no Windows support.

## Capabilities

### New Capabilities

None. The display-width primitives would naturally be their own capability, but the crate's
existing spec boundaries put `ui::layout` under `responsive-layout`, and that is where they
land.

### Modified Capabilities

- `responsive-layout`: the crate's display-width primitives, their confinement, the Unicode
  promise they carry, and the header's left-shortening measured in columns.
- `change-rows`: every cell of the row grammar — the name field, the date field, the badge,
  the progress cell, the problem and message rows — measured and truncated in columns.
- `markdown-render`: wrapping, hard-splitting, and the hanging indent measured in columns;
  the "no line exceeds `width`" promise restated in columns.
- `artifact-content`: the detail draw loop advances `x` by the columns `set_string` consumed;
  `No content yet` is padded; a sweep proves every `content_lines` line fits at every width.
- `tasks-checklist`: the checklist's line grammar in columns; `No tasks yet` is padded; the
  same sweep over `ui::tasks::lines`.
- `tasks-progress-bar`: the bar's length and its cell-dropping order counted in columns.
- `detail-header`: the header row's three cells measured in columns.
- `detail-scroll`: `Action::OpenDetail` resets the scroll only when it moves the route.
- `list-filtering`: the query match folds case for the whole of Unicode.

## Impact

`src/ui/layout.rs` (two new pure functions), `src/ui/list.rs`, `src/ui/markdown.rs`,
`src/ui/tasks.rs`, `src/ui/detail.rs`, `src/ui/view.rs`, `src/ui/app.rs`, and a new
`scripts/gates/colwidth.sh` composed into `make gates` alongside `tests/ci_workflow.rs`'s
recipe-vs-directory check. `SPEC.md` and `AGENTS.md` gain the Unicode promise. No manifest,
no config format, no keybinding, no dependency, no data model, no external service, no
sibling repository.

**One coordination item, stated rather than assumed:** nothing here changes the dependency
set, so `plugin-build`'s and `quality-gates`' "exactly six" requirements are untouched. That
is a direct consequence of Decision 1 and is the reason it was decided that way.
