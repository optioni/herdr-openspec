## MODIFIED Requirements

### Requirement: `Dashboard::sync_detail` resolves the selected tab's content

`ui::app::Dashboard::sync_detail(&mut self, read: ArtifactReader)` SHALL, in this order:

0. Take and clear `refresh.reload`, holding the taken value as `force` for the rest of the
   call. Taking it here — rather than reading it later — is what makes one flag produce
   exactly one re-read, on every path out of the function including the empty-list one.
1. When `visible()` is empty, clear `detail.source` and `detail.problems`, set `detail.tab`
   and `detail.scroll` to `0`, set `detail.loaded` to `None`, and return.
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
4. Otherwise re-read: set `detail.source` to the concatenation of `read(path)` over the
   selected artifact's `paths` **in the order `changes::from_files` resolved them**,
   inserting a `\n` between two files when the preceding one does not already end in one, so
   two spec files cannot run together on one line. Every `Err(e)` contributes no text and
   appends the problem `"<path>: <e>"` to `detail.problems`, which is cleared first. Set
   `detail.scroll` to `0` **exactly when the key changed**, and set `detail.loaded` to the
   key.

Resetting the scroll on a key change and not on a forced reload is the whole point of the
split: a tab or change move starts at the top, while an agent's save re-renders the document
under a reader who is halfway down it without throwing them back to line one.
`layout::scroll_offset` and `Dashboard::normalise_scroll` still clamp the preserved offset
against the new content's length, so a document that shrank is corrected within one frame.

`sync_detail` SHALL be called by `ui::driver::run_loop` once per iteration, **before** the
draw and **after** the loop's live tier has adopted any result waiting for it, so the very
first frame shows content rather than a blank region that fills in on the second, and a
corrected change set is read in the same iteration that adopted it. It SHALL NOT be called
by any view.

`sync_detail` SHALL be total: it SHALL NOT panic for any dashboard state, any artifact list,
any `detail.tab`, any value of `refresh.reload`, or any reader behaviour including one that
fails on every path.

#### Scenario: The selected tab's file is read once and reused

- **WHEN** a `Dashboard` whose selected change carries artifacts `proposal` →
  `[/repo/p.md]` and `design` → `[/repo/d.md]`, with `detail.tab: 0` and `refresh.reload`
  false, is given `sync_detail` with a recording reader returning `# proposal` for any path,
  three times in a row
- **THEN** `detail.source` is `# proposal`, `detail.problems` is empty, and
  `detail.loaded` is `Some((change dir, 0))`
- **AND** the reader recorded exactly **one** call, for `/repo/p.md`, so an unchanged
  selection re-reads nothing

#### Scenario: A forced reload re-reads the same key and keeps the scroll

- **WHEN** the same dashboard, already synced once and scrolled to `detail.scroll: 6`, has
  `refresh.reload` set and is synced again with a reader now returning `# proposal (edited)`
- **THEN** the reader recorded a **second** call for `/repo/p.md`
- **AND** `detail.source` is `# proposal (edited)`
- **AND** `detail.scroll` is still `6`, because the key did not change
- **AND** `refresh.reload` is false afterwards, and a third sync with the flag still clear
  records no further call

#### Scenario: A tab move under a forced reload still resets the scroll

- **WHEN** a `Dashboard` scrolled to `6` has both `detail.tab` moved from `0` to `1` and
  `refresh.reload` set, and is then synced
- **THEN** the reader was called with `/repo/d.md` and `detail.scroll` is `0`
- **AND** the reset is attributable to the key change rather than to the flag, because the
  previous scenario proves a forced reload alone preserves the offset

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
- **AND** the same holds across a forced reload: a `refresh.reload` that re-reads an
  unchanged key clears `detail.problems` first, so a file whose permissions were repaired
  stops being reported the moment the next refresh arrives

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
