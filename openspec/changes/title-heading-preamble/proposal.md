## Why

A `tasks.md` that opens with a document title — `# drift-window — tasks`, then prose, then
`## 1. …` groups — renders as if that title were a task group. Measured live against
`~/Code/slot-car-racing`'s `drift-window` change: the tracked-tasks tab draws a fold header
`▾ drift-window — tasks` carrying `[-]`, every real group sits one level under it and is
indented two columns, and roughly 140 rows of the title's own prose stand between the
progress bar and the first group. The `[-]` is the sharpest part: the header claims the
section was counted and found empty while `seed_expanded` — reading the same section's
**subtree** — has already decided it holds incomplete work and opened it.

Nothing is wrong with the file. `artifact-folds` normalises depth against a file's own
shallowest heading and gives every heading a header row, which is right for a delta spec and
wrong for a document title: on a tracked-tasks tab a heading section **is** a task group, and
a lone top-level heading holding no items of its own is not one.

## What Changes

- On a **tracked-tasks** artifact only, a split file's leading heading is recognised as a
  **document title** when it is the file's first heading, the only heading at the file's
  shallowest level, and its own body holds no task items.
- A title heading contributes **no header row and no fold**: its body becomes a
  `None`-labelled section on exactly the terms a preamble already is — always open, never a
  fold target, `progress` `None`. A file may therefore contribute two such sections (a real
  preamble and a title body) where it could previously contribute one.
- Depth is normalised against the file's remaining **labelled** headings, so the real groups
  return to depth 0 and lose their two-column indent.
- The split gate counts the title's body as a contribution rather than the title heading, so
  a file whose title is followed by a single group and no prose stops splitting and renders
  through `ui::tasks::lines` — which keeps both heading lines — instead of losing one.
- A spec tab is untouched: a delta spec whose only `##` heading is `## ADDED Requirements`
  keeps that header row, because the rule is gated on `tracks_tasks`.

Not **BREAKING**: no manifest, config-format, or keybinding change. `Space` on the demoted
section becomes inert where it used to fold, which is the existing preamble behaviour rather
than a new one.

## Non-Goals

- **No change to what a heading section's `progress` counts.** A `## 7.` group with `###`
  sub-headings still reports its own body's items, not its subtree's. That disagreement with
  `seed_expanded` is real and is left standing deliberately; it is a separate argument about
  the cell, not about the title.
- **No new fold target, and no "collapse all".** Removing the title's header row removes the
  one gesture that collapsed the whole document. Replacing it is a different change.
- **No rendering of the title's text.** The detail region's own header already names the
  change; the title line is dropped rather than drawn somewhere new.
- **No change to the progress bar, the gauge, or its segment slice**, which reads `Some`
  values only and is unaffected by a section moving from `Some({0,0})` to `None`.
- No change to `tasks::parse`, to the checkbox-counting rule, or to what the CLI reports.
- No editing of OpenSpec files, no orchestration across changes, no change authoring, no
  Windows support — the PRD non-goals are untouched.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `artifact-folds`: the section-derivation rule gains the title-heading case; the "at most one
  `None`-labelled section per file" sentence, the `min_level` normalisation, and the
  `progress`-is-`Some`-for-every-heading-section sentence each need the carve-out.
- `artifact-content`: its description of the section list ("one section per heading") stops
  being true, and the body-grammar walk needs to say what a second `None`-labelled section
  draws on a tracked-tasks tab.

## Impact

- `src/ui/app.rs` — `Dashboard::sync_detail`'s per-path section walk (the `splits` gate, the
  `has_preamble`/`contributions` computation, `min_level`, and the heading loop). No new I/O:
  the walk already holds the file's text and its `HeadingSection` list.
- `src/ui/detail.rs` — no behaviour change expected; `visible_sections`, `bodies_are_indented`
  and `content_lines` already treat a `None` label as always-open and un-foldable. Its width
  sweeps and the `58`/`78` rule apply to any test added there.
- `SPEC.md` and `AGENTS.md` — both describe the fold derivation in prose and must move with
  it; `tests/doc_contract.rs` binds several such claims to their files.
- No change to `Cargo.toml`, the plugin manifest, `herdr-plugin.toml`, `scripts/gates/`, or
  any process spawn: nothing here reaches `cli`, the Herdr socket, or the terminal.
- **Roadmap:** unplanned work. `openspec/IMPLEMENTATION-ORDER.md` has no row for it.
  The shape is not rare and was not confined to another repository: measured over both trees,
  **17 of this repository's own 48 `tasks.md` files** and 2 of `slot-car-racing`'s 9 satisfy
  all three clauses of the title rule below. `artifact-folds` shipped a derivation that is
  right for a spec file and was never argued for a task file that opens with a title; the
  defect has been on screen here since, and went unremarked rather than unrendered.
