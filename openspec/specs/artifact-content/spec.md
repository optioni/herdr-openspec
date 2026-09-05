# artifact-content Specification

## Purpose
TBD - created by archiving change detail-view. Update Purpose after archive.

## Requirements

### Requirement: The artifact read is an injected collaborator, confined to one binding

Reading an artifact file is **not** a view's job. That no view file names a filesystem API is
`dashboard-loop`'s requirement — its pure set grows from six files to seven with
`src/ui/detail.rs` added — and is not restated here. What **this** requirement adds is where
the read does live, and that there is exactly one of it.

The read SHALL arrive as an injected closure, in the same shape by which `resolve` takes the
`npm prefix -g` probe and `config` takes the process environment:

```rust
pub type ArtifactReader<'a> = &'a dyn Fn(&std::path::Path) -> Result<String, String>;
```

`ui::driver::run_loop` SHALL take one as a parameter and pass it to
`Dashboard::sync_detail`. The crate SHALL hold exactly one production binding,
`ui::read_artifact(path: &Path) -> Result<String, String>` in `src/ui/mod.rs`, which calls
`std::fs::read_to_string` and maps its error to the error's `Display` text. `ui::run` SHALL
be the only caller that passes it.

`read_to_string` SHALL appear under `src/ui/` only in `src/ui/mod.rs`, checked tree-wide with
a positive control asserting that `src/ui/mod.rs` does name it, so an exclusion that protects
nothing fails rather than passing.

`read_artifact` SHALL carry its own assertions rather than being covered only by the
composition that calls it, on the same terms `config::env_lookup` does: it is a one-line
binding to the real world, and a binding nothing asserts is untested residue.

#### Scenario: The artifact read has exactly one binding under `src/ui/`

- **WHEN** every `*.rs` file under `src/ui/` is searched for `read_to_string`
- **THEN** every match is in `src/ui/mod.rs` and there is at least one
- **AND** the check is proven able to fail: run against a copy carrying a
  `std::fs::read_to_string` call in `src/ui/driver.rs`, it reports it
- **AND** it is proven able to fail the other way too: run against a copy whose
  `src/ui/mod.rs` names no `read_to_string`, it reports the exclusion as vacuous rather than
  reporting a clean tree

#### Scenario: `read_artifact` agrees with the standard library and names its failure

- **WHEN** `read_artifact` is called on a file written into a `ScratchDir` holding the text
  `# proposal\n`, and again on a path inside that directory that does not exist
- **THEN** the first returns `Ok` with a string equal to `std::fs::read_to_string` of the
  same path
- **AND** the second returns `Err` whose text is non-empty, and does not panic

### Requirement: `Dashboard::sync_detail` resolves the selected tab's content

`ui::app::Dashboard::sync_detail(&mut self, read: ArtifactReader)` SHALL, in this order:

1. When `visible()` is empty, clear `detail.source` and `detail.problems`, set `detail.tab`
   and `detail.scroll` to `0`, set `detail.loaded` to `None`, and return.
2. Clamp `detail.tab` against the selected change's `artifacts`: `0` when the list is empty,
   otherwise at most `artifacts.len() - 1`. This is the one place the `(change, tab)`
   invariant is restored, so a `/` filter edit that moves the selection to a change with
   fewer artifacts cannot leave the tab out of range for a frame.
3. Build the key `(selected change's `dir`, `detail.tab`)` and return unchanged when it
   equals `detail.loaded`. The key is the change's **directory**, never its name and never
   its index: an active `add-auth` and an archived `add-auth` share a name, and a filter edit
   changes which change an index addresses without changing the index.
4. Otherwise re-read: set `detail.source` to the concatenation of `read(path)` over the
   selected artifact's `paths` **in the order `changes::from_files` resolved them**,
   inserting a `\n` between two files when the preceding one does not already end in one, so
   two spec files cannot run together on one line. Every `Err(e)` contributes no text and
   appends the problem `"<path>: <e>"` to `detail.problems`, which is cleared first. Set
   `detail.scroll` to `0` and `detail.loaded` to the key.

`sync_detail` SHALL be called by `ui::driver::run_loop` once per iteration, **before** the
draw, so the very first frame shows content rather than a blank region that fills in on the
second. It SHALL NOT be called by any view.

`sync_detail` SHALL be total: it SHALL NOT panic for any dashboard state, any artifact list,
any `detail.tab`, or any reader behaviour including one that fails on every path.

#### Scenario: The selected tab's file is read once and reused

- **WHEN** a `Dashboard` whose selected change carries artifacts `proposal` →
  `[/repo/p.md]` and `design` → `[/repo/d.md]`, with `detail.tab: 0`, is given
  `sync_detail` with a recording reader returning `# proposal` for any path, three times in
  a row
- **THEN** `detail.source` is `# proposal`, `detail.problems` is empty, and
  `detail.loaded` is `Some((change dir, 0))`
- **AND** the reader recorded exactly **one** call, for `/repo/p.md`, so an unchanged
  selection re-reads nothing

#### Scenario: Switching the tab re-reads, and so does switching the change

- **WHEN** the same dashboard is synced, then given `NextTab`, then synced again, then its
  `selected` is moved to a second change and it is synced a third time
- **THEN** the reader recorded three calls, for `/repo/p.md`, `/repo/d.md`, and the second
  change's first artifact path, in that order
- **AND** `detail.source` holds the second change's text at the end, and `detail.scroll` is
  `0` after each re-read

#### Scenario: Two changes with the same name are distinguished by directory

- **WHEN** a `Dashboard` holding an active change named `add-auth` at
  `/repo/openspec/changes/add-auth` and an archived change also named `add-auth` at
  `/repo/openspec/changes/archive/2026-08-14-add-auth`, each with one artifact resolving to
  a distinct path, is synced with `selected: 0`, then `selected` is set to `1` and it is
  synced again
- **THEN** the reader recorded two calls, one per directory
- **AND** `detail.source` holds the archived change's text after the second sync, so keying
  on the name alone would have shown the wrong artifact

#### Scenario: A multi-file artifact is concatenated in path order with a separating newline

- **WHEN** a `Dashboard` whose selected artifact resolves to `[/repo/specs/a/spec.md,
  /repo/specs/b/spec.md]` is synced with a reader returning `# a` for the first (no trailing
  newline) and `# b\n` for the second
- **THEN** `detail.source` is `# a\n# b\n`
- **AND** with a reader returning `# a\n` for the first instead, `detail.source` is
  `# a\n# b\n` as well, so a file that already ends in a newline gains no second one

#### Scenario: An unreadable file names its reason and does not lose its siblings

- **WHEN** a `Dashboard` whose selected artifact resolves to two paths is synced with a
  reader that returns `Err("permission denied")` for the first and `Ok("# b\n")` for the
  second
- **THEN** `detail.source` is `# b\n`
- **AND** `detail.problems` holds exactly one entry, which contains the failing path and the
  text `permission denied`
- **AND** a second sync after a tab move and back clears the previous `problems` before
  recording again, so a transient failure does not accumulate

#### Scenario: An artifact with no resolved paths reads nothing at all

- **WHEN** a `Dashboard` whose selected artifact has an **empty** `paths` list is synced with
  a recording reader
- **THEN** `detail.source` and `detail.problems` are both empty and `detail.loaded` is
  `Some((change dir, tab))`
- **AND** the reader recorded **zero** calls, so "no content yet" costs no filesystem access

#### Scenario: A tab out of range for the newly selected change is clamped before the read

- **WHEN** a `Dashboard` whose selected change carries five artifacts, with `detail.tab: 4`,
  has its `filter.query` set so that `visible()` becomes a single different change carrying
  two artifacts, and is then synced
- **THEN** `detail.tab` is `1` and the reader was called with that change's second artifact's
  path
- **AND** no panic occurs, and `detail.loaded` names the new change's directory

#### Scenario: An empty visible list clears the detail

- **WHEN** a `Dashboard` holding one change is synced, and then its `filter.query` is set to
  a string matching no change and it is synced again
- **THEN** after the second sync `detail.source` and `detail.problems` are empty,
  `detail.tab` and `detail.scroll` are `0`, and `detail.loaded` is `None`
- **AND** the same holds for a `Dashboard` built over `changes::empty_set()`, which is synced
  without the reader being called at all

#### Scenario: The loop syncs before it draws

- **WHEN** `run_loop` is driven over a `Terminal<TestBackend>` at 120x20 with a dashboard
  whose selected change's first artifact reads as `# proposal\n`, an event script of a single
  `q` press, and a recording reader
- **THEN** it returns `Ok(LoopSummary { frames: 1, polls: 1 })`
- **AND** the buffer's detail content area holds `# proposal` on its first row, so the very
  first frame carried the content rather than a blank region
- **AND** the reader recorded exactly one call

### Requirement: The content area renders the artifact, its problems, or `No content yet`

`ui::detail::content_lines(detail: &Detail, width: u16) -> Vec<markdown::Line>` SHALL be the
single line list both `ui::view::render` and `Dashboard::normalise_scroll` derive the detail
content from, so the drawn slice and the clamp can never disagree. It SHALL return:

- one line per entry of `detail.problems`, each the text `"! <problem>"` passed through
  `ui::list::pad_or_truncate_right` at `width`, followed by
- `ui::markdown::lines(&detail.source, width)`, unchanged;
- and, when `detail.problems` is empty **and** `detail.source` is empty, exactly one line
  reading `No content yet` — the state `SPEC.md`'s degraded-states table names for a missing
  artifact file.

When `detail.problems` is non-empty and `detail.source` is empty, the problems alone SHALL be
returned and `No content yet` SHALL NOT appear: the reason is known, and reporting both would
say two contradictory things about the same tab.

Every line SHALL carry plain faces except those `markdown::lines` produces; `content_lines`
SHALL name no `ratatui` type, on the same terms `ui::markdown` and `ui::list` do not.

`ui::view::render` SHALL draw the slice of `content_lines` that starts at
`layout::scroll_offset(lines.len(), detail.scroll, content.height)` and runs for at most
`content.height` lines, into the content area `layout::split_detail` returns, starting at its
first row and first column, never writing past the interior's last column.

#### Scenario: A missing artifact still shows its tab and reads `No content yet`

- **WHEN** a `Dashboard` whose selected change carries the five tdd artifacts, whose
  `detail.tab` is `1`, whose `detail.source` and `detail.problems` are empty, is rendered at
  120x20 and at 60x20 at `Route::Detail`
- **THEN** in the 120-column buffer row 4, columns 41 onward, begins `No content yet`
- **AND** in the 60-column buffer row 4, columns 1 onward, begins `No content yet`
- **AND** in both buffers row 3 still holds the full five-tab bar with `2 specs` bold, so a
  missing file removes neither the tab nor the header

#### Scenario: A read failure is named above the content at both widths

- **WHEN** a `Dashboard` whose `detail.problems` is `["/repo/specs/a/spec.md: permission
  denied"]` and whose `detail.source` is `# b\n` is rendered at 120x20 and at 60x20 at
  `Route::Detail`
- **THEN** in each buffer the content area's first row begins `! /repo/specs/a/spec.md:` and
  its second row begins `# b`
- **AND** `No content yet` appears in neither buffer
- **AND** each problem row is exactly the interior width — 78 and 58 — so it neither
  overwrites the border nor leaves a partial cell

#### Scenario: The rendered markdown fills the content area, not the whole interior

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change carries one artifact and
  whose `detail.source` is twenty lines reading `- line-00` through `- line-19` is rendered
  at 120x20 and at 60x20
- **THEN** in each buffer the header row and the tab row are unchanged and the content
  area's first row holds `- line-00`
- **AND** the content area's last drawn row is `- line-13`, because the 16-row interior gives
  the content area 14 rows, so the header and the tab bar took two rows from the markdown
  rather than being drawn over it

#### Scenario: `content_lines` is total and width-parameterised

- **WHEN** `content_lines` is called at width `78` and at width `58` over: an empty `Detail`;
  one with problems only; one with a source only; one with both; and one whose source is a
  200-character paragraph
- **THEN** no call panics at either width
- **AND** the wrapped paragraph produces strictly more lines at `58` than at `78`, so the
  width genuinely reaches `markdown::lines`
- **AND** no returned line's text exceeds the width it was called with
