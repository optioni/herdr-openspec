## MODIFIED Requirements

### Requirement: `Dashboard::refresh` carries the live tier's state

`ui::app::Refresh` SHALL carry exactly three fields:

- `requested: bool` — set by `Action::Refresh` and by `ui::load` at startup, cleared by
  `run_loop` once it has asked the refresher for a full reload. It is the counterpart of
  `quit`: a pure state value the loop observes, so `Dashboard::apply` stays a pure function
  that reaches no collaborator.
- `reload: bool` — set by `adopt`, consumed by `sync_detail`. It forces a re-read of an
  unchanged `(change directory, tab)` key, which is what makes an edit to the artifact
  currently on screen visible: the cache key `artifact-content` compares does not change when
  a file's **content** does.
- `problems: Vec<String>` — problems that outlive a reload. `live-refresh` populated it with
  one condition, a watcher that would not start; `degraded-states` adds the two other standing
  conditions the pane learns at startup, in this order: the configuration's key-by-key
  fallbacks (`plugin-config`), then the `openspec` binary probe's (`openspec-binary`), then the
  watcher's. All three share one lifetime — they are true for the whole session and no reload
  re-derives them — which is exactly what `ChangeSet::problems` is not: everything the CLI or
  the file walk reports rides there instead and is replaced wholesale on every adopt, which is
  why none of these three may live there.

`Refresh` SHALL carry exactly these three fields and no more. `degraded-states`' `file_mode`
flag is a **`Dashboard`** field rather than a fourth one here, and that is a deliberate
placement rather than an arbitrary one: `requested`, `reload`, and `problems` are all consumed
or replaced as the session runs, while `file_mode` is decided once, at startup, and never
moves. `dashboard-loop` carries its definition and the field count that follows from it.

`Detail` SHALL still carry exactly **five**: `reload` deliberately lives on `Refresh` rather
than on `Detail`, because `Dashboard` gains one field either way and putting it on `Refresh`
keeps `Detail`'s five unchanged.

The count of `Dashboard`'s own fields is **not** restated here. It was written as nine when
`live-refresh` landed and three later changes have added to it since without correcting the
sentence; `dashboard-loop`'s own requirement is the single place that number lives, and this
requirement now defers to it rather than carrying a second copy that drifts.

`Refresh` SHALL NOT implement `Default` — neither derived nor hand-written, anywhere in the
crate — and every construction and destructuring of it SHALL name all three fields with no
`..` rest, on exactly the terms `Dashboard`, `Filter`, and `Detail` are already bound.

#### Scenario: `Refresh` has no `Default` and no site elides a field

- **WHEN** every `*.rs` file under `src/` is searched for `impl Default for Refresh`, for a
  `Default` inside the `#[derive(...)]` immediately preceding `struct Refresh`, and for a `..`
  inside a `Refresh { … }` literal or pattern, brace-matched from the opening `{` to its
  partner so a multi-line elision rustfmt spread over several lines is seen
- **THEN** there is no match
- **AND** the same check runs over `Dashboard`, `Filter`, and `Detail` in the same invocation,
  and is judged against that type set's **own** counted minimum of literal or pattern spans —
  not a floor shared with the `Dashboard` set, which scans far more (`dashboard-loop`). The
  `Refresh` set's floor is measured at this change's base commit and written as that run's
  default when the gate becomes a repository file; the landed **90**, measured at **107**, is
  superseded by that measurement
- **AND** a compile-time companion destructures a `Dashboard` naming every one of its fields
  with no `..`, and a `Refresh` naming all **three**, so a field added to either breaks the
  build at that site rather than passing a source grep that never saw it. The `Dashboard` arm
  names **thirteen** fields after `degraded-states` adds `file_mode`
