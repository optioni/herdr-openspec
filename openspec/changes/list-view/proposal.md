## Why

`herdr-openspec ui` opens a dashboard whose body is two empty bordered frames. The pane
already knows every change in the repository — `ui::load` reads them from files on
startup, with no `openspec` binary needed — but shows none of them. `PRD.md` → Goal 2
("see progress at a glance") and the first two Reading stories are unmet until the list
region holds rows.

This is the Phase 4 `list-view` row of `openspec/IMPLEMENTATION-ORDER.md`, whose single
dependency is `tui-shell` (archived 2026-09-04).

## What Changes

- **Change rows fill the list region.** One row per active change carrying a selection
  marker, the change name, and its task progress; then a separator; then the archived
  changes below it, each additionally carrying its `YYYY-MM-DD` date. The row grammar is
  written against the **38-column interior** the wide layout's `Length(40)` list column
  leaves — `tui-shell` froze that constraint and recorded that `SPEC.md`'s List view mock,
  at 48 characters, illustrates content rather than width. This change makes the width
  decision and corrects the mock.
- **Selection and navigation.** `j` / `k` and the Up / Down arrows move a selection across
  active and archived rows, clamped at both ends rather than wrapping. The list scrolls to
  keep the selection visible, from a viewport derived on every draw rather than stored.
- **`/` filtering.** `/` enters a filter mode in which printable keys type into a query
  instead of acting as commands, `Backspace` deletes, `Enter` accepts, and `Esc` cancels.
  The query is a case-insensitive substring match on the change name, applied to both
  tiers. `Ctrl-C` still quits while filtering; **`q` does not**.
- **Empty and degraded body states.** No repository (naming the directory searched), no
  changes at all, no *active* changes with archived ones still browsable, a filter that
  matches nothing, and repository-level `ChangeSet::problems`.
- `Dashboard` gains `selected` and `filter`; `Action` gains six variants;
  `action_for` gains a `filtering` parameter; `Esc` becomes a layered dismissal.
- New pure module `src/ui/list.rs` holds the row grammar; the scroll viewport joins
  `src/ui/layout.rs`'s geometry and the filter predicate joins `src/ui/app.rs`'s state, so
  every function in `list.rs` is width-parameterised and its width check needs no
  exemption list. `src/ui/view.rs` draws what `rows` returns.
- **No new dependency.** No **BREAKING** change: the plugin manifest, `config.toml`'s
  format, and every key already documented in `SPEC.md` → Keys are unchanged. `/`, `j`,
  `k`, and the arrows are documented there already and are bound here for the first time.

## Non-Goals

- **The detail side.** No artifact tab bar, no markdown rendering, no "No content yet" —
  `markdown-viewer`, `detail-view`, and `tasks-tab`.
- **Agent badges and the footer's unattributed count.** `SPEC.md`'s mock reserves a third
  column for them; this change reserves **no** width for it and lets the name field flex,
  so `agent-attribution` re-flexes rather than inheriting a blank gutter.
- **Per-change `Change::problems`**, which shares that third column. Repository-level
  `ChangeSet::problems` *is* rendered here, because nothing else would ever show it.
- **The CLI path, the watcher, and `r`** — `changes-from-cli` is already landed but
  `list-view` does not acquire a dependency on it; `live-refresh` wires it in.
- **Editing.** The list is read-only. Nothing writes inside `openspec/`.
- No orchestration across changes, no change authoring, no Windows support.

## Capabilities

### New Capabilities
- `change-rows`: the row grammar for an active and an archived change, the archived
  separator, ordering, truncation at the mandated interiors, and every empty or degraded
  body state.
- `list-selection`: the selection index, `j` / `k` / arrow navigation, clamping, and the
  scroll viewport that keeps the selection visible.
- `list-filtering`: `/` filter mode, its key semantics, the matching rule, and the
  interaction between the query and the selection.

### Modified Capabilities
- `dashboard-loop`: `Dashboard` grows from five fields to seven; `action_for` takes the
  filter mode; `Action` gains `SelectNext`, `SelectPrev`, `FilterStart`, `FilterPush`,
  `FilterPop`, and a renamed layered `Back`; `Dashboard::apply` becomes a small state
  machine. The no-`Default`/name-every-field gate extends to the new `Filter` type.
- `responsive-layout`: the list region's interior is no longer blank, so the
  "Interiors are blank at both widths" requirement is replaced with one that names which
  region each change owns; the footer renders the filter prompt while filtering; and the
  header requirement's "no repository" scenario narrows its assertion from the whole
  buffer to row 0, because the body now legitimately names the searched directory.

## Impact

- **Code:** `src/ui/list.rs` (new), `src/ui/layout.rs` (`viewport`), `src/ui/view.rs`,
  `src/ui/app.rs`, `src/ui/driver.rs` (one call site), `src/ui/mod.rs` (`load` names the
  two new fields), and `src/changes.rs` (a `#[cfg(test)]` fixture module — the crate's
  `Change`/`ChangeSet` literals stay in that one file, which is what `change-model`'s gate
  searches; they deliberately do **not** go in `src/lib.rs`'s `testutil`). No change to
  `src/cli.rs`, `Cargo.toml`, or `Cargo.lock`.
- **Test churn:** adding two `Dashboard` fields with `..` forbidden breaks every
  `Dashboard` literal in the crate — six sites across `src/ui/app.rs`, `src/ui/mod.rs`,
  `src/ui/view.rs`, `src/ui/driver.rs`, and `src/lib.rs` — so roughly thirty landed tests
  fail to compile until they are updated in the same task group.
- **`Change` type:** unaltered. No field added, removed, or retyped, so the
  `from_files` / `from_cli` agreement is untouched.
- **Checks:** `NOIO-VIEW`'s pure set gains `src/ui/list.rs`; `NOCLI-SHELL`'s and
  `NOSPAWN-GREP`'s file-count floors rise by one; `NODEFAULT-UI` extends to `Filter`;
  `WIDTHS`'s floor rises; `NOLIT-CHANGE` and `LISTWIDTHS` are new. `NORAW-GREP`,
  `NOWAIVER`, `GATE-MECH1`, `NOJSON-SEAM`, and `GRAPH-SNAP` are carried forward unchanged.
  `DEPS` is carried forward **with the four edits `tui-shell` recorded only as prose**
  applied and written out, because the archived block fails on today's tree without them.
- **Documentation:** `SPEC.md` → List view, Keys, and Degraded states are corrected as
  part of this change; `AGENTS.md` → Current repo state is updated.
- **No sibling repository, deployment manifest, background job, or external service is
  affected.** A linked plugin needs `make build` to see the rows; no re-link.
