## Why

The `specs` artifact's `generates` is a glob (`specs/**/*.md`), so `Dashboard::sync_detail`
concatenates every capability's spec into one flat `detail.source` joined by a newline and
nothing else. OpenSpec spec files open at `## MODIFIED Requirements`, carrying no capability
name — the name lives only in the directory, which is never rendered. The `specs` tab of
`markdown-legibility` is therefore three indistinguishable `## MODIFIED Requirements` blocks
run together, and a reader cannot see which capabilities a change touches, let alone reach
one.

## What Changes

- A multi-file artifact's content becomes a list of **foldable per-file sections**, each
  under a header row naming the file (for `specs/<capability>/spec.md`, the capability).
  **All sections start collapsed**, so the `specs` tab opens as a list of capability names.
- An artifact resolving to **one** path is unaffected: no header row, no fold state, no
  behavioural change. `proposal`, `design`, `tasks`, and `planning-review` render as today.
- **BREAKING** — `Space` becomes route-dependent. It folds the list section at
  `Route::List` and the artifact section at `Route::Detail`, mirroring `Next`/`Prev`'s
  existing split. Today it folds a list section at either route.
- **BREAKING** — at a *foldable* tab the detail region gains a **line cursor**: `j`/`k`,
  the arrows, and the wheel move it and the viewport follows through `layout::viewport`,
  the helper the list region already uses, instead of moving `detail.scroll` through
  `layout::scroll_offset`. Without a cursor `Space` cannot address a section at all —
  `scroll_offset` clamps the offset to `0` whenever the content fits the region, so three
  collapsed headers in a tall pane would leave only the first reachable. A non-foldable tab
  keeps today's offset scrolling unchanged.
- A left click on a section header folds it, on the same terms a click on a list section
  header already does.

## Non-Goals

- Changing markdown rendering itself. Soft-break reflow and glyph choices are the in-flight
  `markdown-legibility` change; this one is the join *above* the renderer.
- Editing, reordering, or filtering spec files. The pane stays read-only.
- Persisting fold state across changes, tabs, or sessions.
- Unifying the two regions on one cursor model for every tab. Considered in `design.md`.

## Capabilities

### New Capabilities
- `artifact-folds`: a multi-file artifact's content as named, foldable sections — the
  section list, the header row, the collapsed-by-default rule, the toggle, and the reset.

### Modified Capabilities
- `artifact-content`: `sync_detail` produces per-file sections rather than one concatenated
  string, and `content_lines` renders them.
- `detail-scroll`: the detail region carries a cursor at a foldable tab and derives its
  offset from it.
- `list-selection`: `Space` toggles a *list* section only at `Route::List`.
- `mouse-input`: a left click in the detail content area can toggle a section.
- `view-palette`: two roles join the enum, the modifier table, and the uncoloured list.
- `responsive-layout`: `Zone` gains a content-area row variant, and `Detail` narrows by it.

## Impact

`src/ui/app.rs` (`Detail`, `sync_detail`, `apply`, `scroll_by`, `normalise_scroll`,
`apply_click`, `Target`), `src/ui/detail.rs` (`content_lines`, a new `section_at`),
`src/ui/view.rs`, `src/ui/driver.rs` (`mouse_action`), `src/ui/layout.rs` (`Zone`), and
`src/ui/palette.rs` (two roles). `Detail` and `Dashboard` are on the `NODEFAULT-UI` gate's
type list, so a new field must be named at every construction site, and `ArtifactSection` joins that
list. `SPEC.md`'s key and mouse binding tables and `tests/doc_contract.rs`'s binding lists
change with them. No manifest, no config format, no dependency, and no process spawned.

## Roadmap position

Unplanned post-roadmap work, like `doc-conformance` before it.
`openspec/IMPLEMENTATION-ORDER.md` planned `detail-view` and `markdown-viewer` against
single-file artifacts and never anticipated that one tab's `generates` is a glob, so no row
covers this. It violates no PRD non-goal: the pane still writes nothing, orchestrates
nothing, authors nothing, and adds no Windows surface.
