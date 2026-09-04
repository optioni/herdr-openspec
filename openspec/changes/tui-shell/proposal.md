## Why

`herdr-openspec ui` prints a banner and blocks on stdin. Everything the dashboard needs
to paint already exists — `changes::from_files` returns a complete `ChangeSet` with no
`openspec` binary present — but there is no terminal, no frame, and no key handling to
put it on screen. This is the Phase 4 `tui-shell` row of
`openspec/IMPLEMENTATION-ORDER.md`: the shell that `list-view`, `markdown-viewer`,
`agent-polling`, and everything after them render into.

## What Changes

- **`ui::terminal`** — crossterm setup and teardown behind a `TerminalOps` seam: raw mode
  and the alternate screen entered in a fixed order and left in the exact reverse order,
  by an RAII guard whose `Drop` restores on a normal return, on an error return, and on an
  unwinding panic. A panic hook restores *before* the panic message is printed, so the
  message lands on a cooked terminal instead of a raw one.
- **`ui::driver`** — the ratatui event loop: draw first, then wait. The first frame is on
  screen before any event is read, which is what `SPEC.md` → Dual-source model means by
  "the pane paints on open".
- **`ui::app`** — the `Dashboard` state value and pure key handling. `q` and `Ctrl-C` quit;
  `Enter` opens the detail route and `Esc` returns; every other key is ignored rather than
  guessed at.
- **`ui::layout` and `ui::view`** — the 100-column breakpoint (`SPEC.md` → Responsive
  layout) and the shell chrome it decides: a header row, a footer key-hint row, and a body
  that is two bordered regions side by side at 100 columns or wider and one region below
  it. Views are pure functions of state and perform no I/O.
- **`ui::load`** — startup state: `resolve::find_repo` then `changes::from_files`. **No CLI
  path.** The dashboard is usable and fully tested with no `openspec` binary present at
  all; that is the dependency edge the roadmap keeps deliberate.
- **A `TestBackend` harness in `crate::testutil`** — `render_at(width, height, &Dashboard)`
  returning a `ratatui::buffer::Buffer`, plus row- and cell-reading helpers. Every view
  scenario in this change and in every later view change renders at **both 60 and 120
  columns** and asserts on named cells, not on "something was drawn".
- **`herdr-openspec ui` refuses to start when stdout is not a terminal**, exiting 3 with a
  message naming the reason. A TUI cannot render into a pipe, and — load-bearing for this
  repository — it keeps `cargo test`, which spawns this binary, from putting the
  developer's own terminal into raw mode.
- **Dependency:** add `ratatui` 0.30.2 (`default-features = false`, `features =
  ["crossterm"]`), used through its own `ratatui::crossterm` re-export rather than a second
  direct `crossterm` declaration. This raises the crate's `rust-version` from `1.85` to
  `1.88` and introduces the first proc-macro crates into the normal build graph; both are
  argued in design.md and both change `plugin-build`'s enforced requirements.
- **`changes::empty_set()`** — a one-line total constructor for the "no repository" case,
  placed in `src/changes.rs` so `change-model`'s existing no-`Default`/no-`..` gate covers
  it like every other construction site.

Nothing spawns a process outside `cli`; nothing in `src/ui/` names the CLI seam at all.
Not **BREAKING**: no plugin-manifest change, no config-format change, no keybinding
removed or repurposed. `Ctrl-C` is added to the key table, which is additive.

## Non-Goals

- **No change list, no rows, no selection, no filtering, no artifact tabs, no markdown.**
  The body regions render as empty bordered frames in this change. `list-view`,
  `markdown-viewer`, and `detail-view` fill them. The "no repository" and "no changes"
  **empty states** are `list-view`'s, not this change's; the shell only marks a missing
  repository in its header.
- **No CLI, no worker thread, no watcher, no debounce, no refresh key.** `live-refresh`
  owns those. `src/ui/` naming `from_cli`, `OpenspecCli`, or `HerdrCli` is a check
  failure here, not a design choice.
- **No agent column, no action keys, no Herdr socket.** Phase 5.
- **No runtime write anywhere under `openspec/`** — PRD non-goal. A check proves the tree
  is unchanged against a base commit, covering untracked files as well as tracked ones.
- **No orchestration, no change authoring, no editing of OpenSpec files, no Windows** —
  the four PRD non-goals are untouched: this change reads and renders only.

## Capabilities

### New Capabilities
- `terminal-lifecycle`: entering and leaving raw mode and the alternate screen, restoring
  on every exit path including a panic, and refusing to start without a terminal.
- `dashboard-loop`: the draw-then-wait event loop, quit handling, route switching,
  ignored input, resize, and the startup load from files.
- `responsive-layout`: the 100-column breakpoint, the regions drawn on each side of it,
  the header and footer, and behaviour at degenerate sizes.

### Modified Capabilities
- `plugin-build`: the argued dependency set gains `ratatui`; the absolute
  "no proc-macro crate in the normal build graph" rule becomes an enumerated allowlist,
  because `ratatui-widgets` cannot be built without one; the declared MSRV moves to
  `1.88`; the per-triple build graph is no longer identical across the four supported
  triples; and the `ui` invocation runs the dashboard instead of printing a placeholder.
- `quality-gates`: the view-test tier becomes a stated requirement — `TestBackend`, both
  widths, assertions on named cells — rather than a sentence in `SPEC.md`.

## Impact

New: `src/ui/mod.rs`, `src/ui/app.rs`, `src/ui/layout.rs`, `src/ui/view.rs`,
`src/ui/terminal.rs`, `src/ui/event.rs`, `src/ui/driver.rs`,
`tests/fixtures/build-graph.txt`. Modified: `src/lib.rs` (module declaration, harness
helpers in `testutil`), `src/main.rs` (dispatch `ui` into `ui::run`), `src/changes.rs`
(`empty_set`), `Cargo.toml`, `Cargo.lock`, `tests/cli.rs` (the banner tests are replaced
by the non-terminal tests). `SPEC.md` gains the corrections listed in planning-review.md.
`AGENTS.md`'s repo-state section is rewritten in place. No manifest change, no config-format
change, no external service, no stored data, no background job.
