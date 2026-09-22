## MODIFIED Requirements

### Requirement: `Dashboard::sync_detail` resolves the selected tab's content

`ui::app::Dashboard::sync_detail(&mut self, read: ArtifactReader)` SHALL, in this order:

0. Take and clear `refresh.reload`, holding the taken value as `force` for the rest of the
   call. Taking it here — rather than reading it later — is what makes one flag produce
   exactly one re-read, on every path out of the function including the empty-list one.
1. When `visible()` is empty, clear `detail.sections`, `detail.problems`, and
   `detail.expanded`, set `detail.tab` and `detail.scroll` to `0`, set `detail.loaded` to
   `None`, and return.
2. Clamp `detail.tab` against the selected change's `artifacts`: `0` when the list is empty,
   otherwise at most `artifacts.len() - 1`. This is the one place the `(change, tab)`
   invariant is restored, so a `/` filter edit that moves the selection to a change with
   fewer artifacts cannot leave the tab out of range for a frame.
3. Build the key `(selected change's `dir`, `detail.tab`)` and return unchanged when it
   equals `detail.loaded` **and `force` is false**. The key is the change's **directory**,
   never its name and never its index: an active `add-auth` and an archived `add-auth` share
   a name, and a filter edit changes which change an index addresses without changing the
   index. `force` is the second exit condition because the key does not change when a
   **file's content** does — an agent saving the very artifact on screen leaves the key
   identical, and without `force` the pane would keep showing the bytes it read at startup.
4. Otherwise re-read: clear `detail.problems`, then for each path of the selected artifact's
   `paths` **in the order `changes::from_files` resolved them**, call `read(path)`. Every
   `Err(e)` contributes no section and appends the problem `"<path>: <e>"`. Every `Ok(text)`
   contributes one or more `ArtifactSection` values by `artifact-folds`' derivation: an
   optional file section, up to two optional `None`-labelled sections — the preamble, and the
   body of a **title heading** demoted on a tracked-tasks artifact — and one section per
   heading `ui::app::split_headings` returned **that was not demoted**, each carrying its own
   `depth` — or, when the file does not split, exactly one section carrying the whole text,
   which is every prose artifact and is unchanged from before this change.
5. Set `detail.scroll` to `0` and clear `detail.expanded` **exactly when the key changed**,
   and on that same condition **seed** `detail.expanded` per `artifact-folds` when the
   selected `ArtifactRef` carries `tracks_tasks == true` — every section whose subtree is
   incomplete, and no other. Then set `detail.loaded` to the key.

The split is decided from the text the reader returned and from `tracks_tasks`, and from
nothing else: no path, no artifact id, and no schema lookup. `artifact-folds` states the
gate. `sync_detail` SHALL call `ui::app::split_headings` at most once per successfully read path
per re-read — the row list is derived on every draw, but the section list is not.

Step 4 does not concatenate. The separating `\n` this requirement once inserted between two
files that would otherwise run together on one line is unnecessary once each file is its own
section: a section's body is rendered on its own lines under its own header, and a file that
ends without a newline can no longer collide with the next file's first line. An unsplit
section's `text` is therefore the reader's bytes unmodified, and a split file's sections
partition those same bytes, less the heading lines that became labels — which is what lets
`artifact-folds`' label rule and `ui::markdown` each see exactly what is on disk.

Resetting the scroll and the fold state on a key change and not on a forced reload is the
whole point of the split: a tab or change move starts at the top with every section
collapsed — or, on the tasks tab, with the unfinished groups open — while an agent's save
re-renders the document under a reader who is halfway down it, and inside a section they
opened, without throwing them back to line one, folding it shut, or re-seeding the folds
around a task that was just checked off. `layout::scroll_offset`, `layout::viewport`, and
`Dashboard::normalise_scroll` still clamp the preserved position against the new content's
length, so a document that shrank is corrected within one frame.

`sync_detail` SHALL be called by `ui::driver::run_loop` once per iteration, **before** the
draw and **after** the loop's live tier has adopted any result waiting for it, so the very
first frame shows content rather than a blank region that fills in on the second, and a
corrected change set is read in the same iteration that adopted it. It SHALL NOT be called
by any view.

`sync_detail` SHALL be total: it SHALL NOT panic for any dashboard state, any artifact list,
any `detail.tab`, any value of `refresh.reload`, any reader behaviour including one that
fails on every path, and any file content including one that is a single unterminated fence.

#### Scenario: The selected tab's file is read once and reused

- **WHEN** a `Dashboard` whose selected change carries artifacts `proposal` →
  `[/repo/p.md]` and `design` → `[/repo/d.md]`, with `detail.tab: 0` and `refresh.reload`
  false, is given `sync_detail` with a recording reader returning `# proposal` for any path,
  three times in a row
- **THEN** `detail.sections` holds one entry whose `text` is `# proposal`,
  `detail.problems` is empty, and `detail.loaded` is `Some((change dir, 0))`
- **AND** the reader recorded exactly **one** call, for `/repo/p.md`, so an unchanged
  selection re-reads nothing

#### Scenario: A forced reload re-reads the same key and keeps the scroll

- **WHEN** the same dashboard, already synced once and scrolled to `detail.scroll: 6`, has
  `refresh.reload` set and is synced again with a reader now returning `# proposal (edited)`
- **THEN** the reader recorded a **second** call for `/repo/p.md`
- **AND** `detail.sections` holds one entry whose `text` is `# proposal (edited)`
- **AND** `detail.scroll` is still `6` and `detail.expanded` is unchanged, because the key
  did not change
- **AND** `refresh.reload` is false afterwards, and a third sync with the flag still clear
  records no further call

#### Scenario: A tab move under a forced reload still resets the scroll

- **WHEN** a `Dashboard` scrolled to `6`, with `detail.expanded` holding `1`, has both
  `detail.tab` moved from `0` to `1` and `refresh.reload` set, and is then synced
- **THEN** the reader was called with `/repo/d.md`, `detail.scroll` is `0`, and
  `detail.expanded` is empty
- **AND** the reset is attributable to the key change rather than to the flag, because the
  previous scenario proves a forced reload alone preserves both

#### Scenario: Switching the tab re-reads, and so does switching the change

- **WHEN** the same dashboard is synced, then given `NextTab`, then synced again, then its
  `selected` is moved to a second change and it is synced a third time
- **THEN** the reader recorded three calls, for `/repo/p.md`, `/repo/d.md`, and the second
  change's first artifact path, in that order
- **AND** `detail.sections` holds the second change's text at the end, and `detail.scroll`
  is `0` and `detail.expanded` empty after each re-read

#### Scenario: Two changes with the same name are distinguished by directory

- **WHEN** a `Dashboard` holding an active change named `add-auth` at
  `/repo/openspec/changes/add-auth` and an archived change also named `add-auth` at
  `/repo/openspec/changes/archive/2026-08-14-add-auth`, each with one artifact resolving to
  a distinct path, is synced with `selected: 0`, then `selected` is set to `1` and it is
  synced again
- **THEN** the reader recorded two calls, one per directory
- **AND** `detail.sections` holds the archived change's text after the second sync, so
  keying on the name alone would have shown the wrong artifact

#### Scenario: A multi-file artifact is concatenated in path order with a separating newline

The scenario's name is kept verbatim because a delta's scenario headers are its merge key. It
pins the **absence** of a separator, which is what the section split replaced it with.

- **WHEN** a `Dashboard` whose selected artifact resolves to `[<dir>/specs/a/spec.md,
  <dir>/specs/b/spec.md]` is synced with a reader returning `# a` for the first (no trailing
  newline) and `# b\n` for the second
- **THEN** `detail.sections` holds two entries, labelled `Some("a")` and `Some("b")` in that
  order, both at `depth` `0`, whose `text` values are `# a` and `# b\n` — the reader's bytes
  verbatim, with no separator inserted and no newline added, and neither file splitting,
  because neither carries a `### Requirement:` heading
- **AND** rendering at 120x20 and at 60x20 in the detail route puts `> a` on the content
  area's first row and `> b` on its second, so the two files cannot run together on one line
  even though the first ends without a newline

#### Scenario: A split file is partitioned rather than copied

- **WHEN** a `Dashboard` whose selected artifact resolves to one path is synced with a reader
  returning
  `## ADDED Requirements\n\n### Requirement: Alpha\nAlpha text.\n\n#### Scenario: A works\n- **WHEN** a\n\n### Requirement: Beta\nBeta text.\n`
- **THEN** `detail.sections` holds four entries, and reassembling them — each section's
  heading line rebuilt from its `depth` and `label`, followed by its `text` — reproduces the
  reader's bytes exactly
- **AND** no section's `text` contains a heading line, and no two sections' `text` values
  overlap
- **AND** the reader recorded exactly one call, and a second `sync_detail` on the unchanged
  key records none and re-splits nothing

#### Scenario: The tasks tab seeds its folds once, on the key change

- **WHEN** a `Dashboard` whose selected artifact carries `tracks_tasks == true` and resolves
  to one path is synced with a reader returning
  `## 1. Done\n\n- [x] a\n\n## 2. Doing\n\n- [ ] b\n`
- **THEN** `detail.expanded` holds exactly `1`
- **AND** a second sync under `refresh.reload`, with the reader now returning the same file
  with `b` checked, leaves `detail.expanded` holding `1`
- **AND** moving `detail.tab` away and back and syncing twice more leaves `detail.expanded`
  empty, because the re-seed found no incomplete subtree
- **AND** the same dashboard whose artifact does **not** carry `tracks_tasks` has an empty
  `detail.expanded` after every one of those syncs

#### Scenario: An unreadable file names its reason and does not lose its siblings

- **WHEN** a `Dashboard` whose selected artifact resolves to two paths is synced with a
  reader that returns `Err("permission denied")` for the first and `Ok("# b\n")` for the
  second
- **THEN** `detail.sections` holds exactly one entry, labelled for the second path, whose
  `text` is `# b\n`
- **AND** `detail.problems` holds exactly one entry, which contains the failing path and the
  text `permission denied`
- **AND** the artifact is **not** foldable, because one section remains, so no header row is
  drawn for the file that did survive
- **AND** a second sync after a tab move and back clears the previous `problems` before
  recording again, so a transient failure does not accumulate
- **AND** the same holds across a forced reload: a `refresh.reload` that re-reads an
  unchanged key clears `detail.problems` first, so a file whose permissions were repaired
  stops being reported the moment the next refresh arrives

#### Scenario: An artifact with no resolved paths reads nothing at all

- **WHEN** a `Dashboard` whose selected artifact has an **empty** `paths` list is synced with
  a recording reader
- **THEN** `detail.sections`, `detail.problems`, and `detail.expanded` are all empty and
  `detail.loaded` is `Some((change dir, tab))`
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
- **THEN** after the second sync `detail.sections`, `detail.problems`, and `detail.expanded`
  are empty, `detail.tab` and `detail.scroll` are `0`, and `detail.loaded` is `None`
- **AND** the same holds for a `Dashboard` built over `changes::empty_set()`, which is synced
  without the reader being called at all
- **AND** a `refresh.reload` set on such a dashboard is still cleared by the sync, so the flag
  cannot survive an empty list and force a spurious re-read on the next frame

#### Scenario: The loop syncs before it draws

- **WHEN** `run_loop` is driven over a `Terminal<TestBackend>` at 120x20 with a dashboard
  whose selected change's first artifact reads as `# proposal\n`, an event script of a single
  `q` press, a recording reader, and an inert live tier
- **THEN** it returns `Ok(LoopSummary { frames: 1, polls: 1 })`
- **AND** the buffer's detail content area holds `# proposal` on its first row, so the very
  first frame carried the content rather than a blank region
- **AND** the reader recorded exactly one call
