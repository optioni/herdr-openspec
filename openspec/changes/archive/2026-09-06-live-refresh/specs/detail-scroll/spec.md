## MODIFIED Requirements

### Requirement: `Dashboard::detail` carries the markdown source and the scroll offset

`ui::app::Dashboard` SHALL carry a `detail: Detail` field, where

```rust
pub struct Detail {
    pub source: String,
    pub scroll: usize,
    pub tab: usize,
    pub problems: Vec<String>,
    pub loaded: Option<(std::path::PathBuf, usize)>,
}
```

`source` is the markdown the detail region shows. From `detail-view` onward it **is** set:
`ui::load` still starts it empty, and `Dashboard::sync_detail` — driven by
`ui::driver::run_loop` with `artifact-content`'s injected reader — fills it from the selected
change's selected artifact. `scroll` is the index of the first rendered line the region
draws, and is a **user-controlled position**, not derived geometry: it is the detail region's
counterpart to `list-selection`'s `selected`, not to `list-selection`'s derived `viewport`.
`tab`, `problems`, and `loaded` are `artifact-tabs`' and `artifact-content`'s, and are
specified there.

`Detail` SHALL carry exactly these **five** fields after `live-refresh` too. That change's
forced-reload flag deliberately lives on `ui::app::Refresh` rather than here: `Dashboard`
gains one field either way, and putting it on `Refresh` leaves `Detail`'s five — and every
`Detail { … }` literal in the crate — untouched. `live-updates` states the flag's contract
and `artifact-content` states what `sync_detail` does with it.

`Detail` SHALL NOT implement `Default` — neither derived nor hand-written, anywhere in the
crate — and every construction and every destructuring of it SHALL name **all five** fields,
with no `..` rest, on exactly the terms `dashboard-loop` states for `Dashboard`, `Filter`,
and (from `live-refresh`) `Refresh`.

#### Scenario: `Detail` has no `Default` and no site elides a field

- **WHEN** every `*.rs` file under `src/` is searched, for the type name `Detail`, for
  `impl Default for Detail`, for a `Default` inside the `#[derive(...)]` immediately
  preceding `struct Detail`, and for a `..` appearing inside a `Detail { … }` literal or
  pattern
- **THEN** there is no match
- **AND** the search is the same parameterised check that covers `Dashboard`, `Filter`, and
  `Refresh`, run over the type list `Dashboard Filter Detail Refresh`, rather than a second
  drifting check. `Refresh` is `live-refresh`'s addition to that list; the check's own
  `TYPES` parameter is what makes adding it a change to an invocation rather than to the
  check
- **AND** it is paired with a positive control asserting that `src/ui/app.rs` **does**
  contain `struct Detail {`, and the check is proven able to fail against a copy carrying
  `impl Default for Detail { … }` and against a copy carrying `let Detail { source, .. }`
- **AND** a compile-time companion exists: a test destructures a `Detail` with an
  exhaustive pattern naming all five fields and no `..`, the `Dashboard` companion
  continues to name all **nine** — eight before `live-refresh`, plus `refresh` — and a
  fourth companion destructures a `Refresh` naming all three

#### Scenario: Startup leaves the detail empty and unscrolled

- **WHEN** `ui::load` is called over a scratch repository holding one change
- **THEN** the returned `Dashboard`'s `detail.source` is empty, `detail.problems` is empty,
  and `detail.scroll`, `detail.tab`, and `detail.loaded` are `0`, `0`, and `None`
- **AND** the same holds when `ui::load` finds **no** `openspec/` directory above its
  starting path and takes its `RepoSearch::NotFound` arm, which is a second `Dashboard`
  construction site and therefore a second place the field can be got wrong
- **AND** `refresh.reload` is **false** at both construction sites, so nothing forces a
  re-read before the loop's first ordinary sync; `refresh.requested` is true at both, which
  is `dashboard-loop`'s clause and not `Detail`'s
- **AND** rendering the `NotFound` dashboard at 120x20 and at 60x20 leaves the detail
  interior blank, because nothing is selected
- **AND** rendering the loaded dashboard at 120x20 shows that change's header and tab bar
  with `No content yet` below them, because a change **is** selected and nothing has been
  read yet — which is exactly the state `run_loop`'s first `sync_detail` replaces
