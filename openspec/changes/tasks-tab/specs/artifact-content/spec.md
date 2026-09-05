## MODIFIED Requirements

### Requirement: The artifact read is an injected collaborator, confined to one binding

Reading an artifact file is **not** a view's job. That no view file names a filesystem API is
`dashboard-loop`'s requirement — its pure set grows to **eight** files with `src/ui/tasks.rs`
added, and the patterns it searches for grow by `tasks::read` — and is not restated here.
What **this** requirement adds is where the read does live, and that there is exactly one of
it.

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

The tracked-tasks tab SHALL add no second read and no second binding. `ui::tasks::lines`
receives the bytes `sync_detail` already placed in `detail.source` and parses them in memory;
`tasks::read`, the filesystem edge of the same module, SHALL be named nowhere under
`src/ui/`.

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

### Requirement: The content area renders the artifact, its problems, or `No content yet`

`ui::detail::content_lines(detail: &Detail, change: Option<&Change>, width: u16) ->
Vec<markdown::Line>` SHALL be the single line list both `ui::view::render` and
`Dashboard::normalise_scroll` derive the detail content from, so the drawn slice and the
clamp can never disagree. It SHALL return:

- one line per entry of `detail.problems`, each the text `"! <problem>"` passed through
  `ui::list::pad_or_truncate_right` at `width`, followed by
- the selected tab's **body**, which is:
  - `ui::tasks::lines(&detail.source, &change.progress, width)` — `tasks-checklist`'s
    grammar and `tasks-progress-bar`'s leading line — when `change` is `Some` and the
    `ArtifactRef` at `detail.tab` carries `tracks_tasks == true`, and
  - `ui::markdown::lines(&detail.source, width)`, unchanged, in every other case,
    including a `None` change, a `detail.tab` past the end of the artifact list, and a
    change carrying no artifacts at all;
- and, when `detail.problems` is empty **and** `detail.source` is empty, exactly one line
  reading `No content yet` — the state `SPEC.md`'s degraded-states table names for a missing
  artifact file.

The `change` argument is the **only** reason the tracked-tasks decision is made once rather
than at each of the two call sites; both callers SHALL pass `Dashboard::selected_change()`
and SHALL NOT decide the grammar themselves.

Both bodies SHALL return nothing for an empty `detail.source`, so the `No content yet` rule
above is unaffected by which body was selected: an artifact file that does not exist reads
`No content yet` whether or not it is the tracked-tasks artifact, and never reads
`tasks-checklist`'s `No tasks yet`.

When `detail.problems` is non-empty and `detail.source` is empty, the problems alone SHALL be
returned and `No content yet` SHALL NOT appear: the reason is known, and reporting both would
say two contradictory things about the same tab.

Every line SHALL carry plain faces except those `markdown::lines` and `ui::tasks::lines`
produce; `content_lines` SHALL name no `ratatui` type, on the same terms `ui::markdown`,
`ui::list`, and `ui::tasks` do not.

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
- **AND** the same holds with `detail.tab` set to `3`, the tracked-tasks position: a missing
  tasks artifact reads `No content yet` and shows no progress bar

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

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change carries one artifact,
  **not** marked `tracks_tasks`, and whose `detail.source` is twenty lines reading
  `- line-00` through `- line-19` is rendered at 120x20 and at 60x20
- **THEN** in each buffer the header row and the tab row are unchanged and the content
  area's first row holds `- line-00`
- **AND** the content area's last drawn row is `- line-13`, because the 16-row interior gives
  the content area 14 rows, so the header and the tab bar took two rows from the markdown
  rather than being drawn over it

#### Scenario: The tracked-tasks tab renders the checklist body instead

- **WHEN** the same `Dashboard` has its one artifact marked `tracks_tasks == true`, a
  `progress` of `Progress { completed: 0, total: 0 }`, and is rendered at 120x20 and at
  60x20
- **THEN** in each buffer the content area's first row holds the progress bar `[-]`, its
  second row is blank, and its third row reads `No tasks yet` — the twenty `- line-NN`
  bullets hold no task lines
- **AND** the header row and the tab row are byte-identical to the markdown render, so only
  the body changed

#### Scenario: `content_lines` is total and width-parameterised

- **WHEN** `content_lines` is called at width `78` and at width `58`, and with `change` set
  to `None`, to a change with no artifacts, to a change whose artifact at `detail.tab` is
  marked, and to one whose is not, over: an empty `Detail`; one with problems only; one with
  a source only; one with both; one whose source is a 200-character paragraph; and one whose
  `tab` is past the end of the artifact list
- **THEN** no call panics at either width for any combination
- **AND** the wrapped paragraph produces strictly more lines at `58` than at `78`, so the
  width genuinely reaches both bodies
- **AND** no returned line's text exceeds the width it was called with
