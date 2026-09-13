## Context

Most of the pane's bindings are undiscoverable. The footer holds at most five hints and drops
them from the end, so `r`, `Space`, `/`, `1`–`9`, `[`, `]` and every mouse gesture are never
named on screen at any width. Against that: twenty-two bound actions reachable from eighteen
keys and five gestures.

Two constraints shape the answer more than the feature does.

**This repository writes every list down twice and binds the two.** `SPEC.md`'s mouse table is
already bound to `mouse_action`'s real `Action` variants inside `cargo test`. A help overlay
that hand-lists bindings would be a third list with no binding at all — and the one most likely
to rot, because nothing fails when it does. So the overlay's content is not the feature; the
**derivation** is.

**The overlay is the crate's first modal.** `Route` is `List`/`Detail` and no layer concept
exists above them. `settings-window`, queued separately, needs the same machinery *and* adds
editing, state writes, and provenance. Building the layer here — where it is read-only — is the
cheaper order, and it is why this design is careful about precedence rules that a second panel
will inherit.

The change is unplanned post-roadmap work, like `doc-conformance` and `foldable-spec-sections`
before it. `openspec/IMPLEMENTATION-ORDER.md` ends at Phase 6 and never anticipated a modal; it
assumed the footer was sufficient, which was true at five bindings and stopped being true at
twenty-two.

## Goals / Non-Goals

**Goals:**

- `?` opens a read-only overlay listing every key and gesture, grouped, with the route each
  applies at.
- The overlay's rows are **bound to the functions that produce the bindings**, so a binding
  added without a help row fails `cargo test`.
- The overlay is the layer mechanism `settings-window` can add a panel to.
- `? help` reaches the footer, so the key that reveals every other key is itself discoverable.

**Non-Goals:**

- Acting on anything from the overlay. No rebinding, no jump-to-feature, no side effect but its
  own scroll.
- Documenting OpenSpec, the schema, or the workflow. This is the pane's own controls.
- Replacing the footer. The footer stays the always-visible minimum.
- Context-sensitive help about the selected change or artifact.
- A second overlay mechanism. If `settings-window` lands first, this shrinks to a panel.
- Any change to the `Change` type. `changes::from_files` and `changes::from_cli` are untouched,
  so the question of keeping them in agreement does not arise (design rule 5).

## Boundaries

**Modules touched.** One new file, `src/ui/help.rs`, holding `INVENTORY` and the overlay's
renderer; it follows `src/ui/palette.rs`'s pattern exactly — a pure `'static` table plus the
functions that read it, swept by `NOIO-VIEW` and `COLWIDTH`. Edits to `src/ui/app.rs`
(`Action::ToggleHelp`, `Help`, `apply`'s overlay branch, and `normalise_help_scroll` — which
sits beside `normalise_scroll` at `src/ui/app.rs:865`, not in `src/ui/driver.rs`),
`src/ui/driver.rs` (`mouse_action`'s overlay branch, and the `run_loop` call site for the
second normaliser),
`src/ui/layout.rs` (`help_band`), `src/ui/view.rs`
(`FOOTER_HINTS`, the post-body overlay draw), **`src/ui/mod.rs`** (four byte-exact footer
assertions in `mod wiring` that the prepend breaks — lines 3544, 3546, 3646, 4423; the file's
production code is untouched), `tests/doc_contract.rs`, two gate scripts,
`tests/gate-controls.toml`, `SPEC.md`, `README.md`, and `AGENTS.md`.

**No process spawn is added anywhere.** `src/cli.rs` is untouched, and `src/ui/help.rs` names
no `HerdrCli`, no `OpenspecCli`, and no spawn API. `NOSPAWN-GREP`, `LAUNCHSEAM`, and
`NOCLI-SHELL` all stay green with no exemption added (design rule 1).

**The new view does no I/O.** `ui::help::render` is a pure function of a `Rect` and a `&Help`,
reading a `'static` slice. It performs no filesystem, process, environment, network, or
standard-I/O call, reads no clock, and joins `NOIO-VIEW`'s `PURE` list as its **tenth** entry
and `COLWIDTH`'s as its **ninth** (design rule 2). No exemption is requested and none is needed.

**Collaborators.** None added. The watcher, refresh worker, agent poller, and launcher are
untouched; the overlay reaches none of them, which is the whole of its read-only claim.

## Contracts

The pane's keyboard and mouse surface is the interface a separate consumer — the reader —
depends on, and `SPEC.md` → Keys is its written form.

- **Additive**, not breaking, for every existing binding: `?` was `Ignore` outside filter mode
  and `FilterPush('?')` inside it, and it stays `FilterPush('?')` inside it. No landed key
  changes meaning while the overlay is closed.
- **Conditionally suppressive** while the overlay is open: seventeen actions become inert.
  This is new behaviour reachable only after the reader presses `?`, so no existing workflow
  changes without an explicit act.
- **One measured regression**, named in full under Decisions: the reachable footer goes from 53
  to 61 columns, so `g focus` is dropped at the mandated 60-column frame.
- `Action` goes from twenty-three variants to twenty-four and `Dashboard` from fourteen fields
  to fifteen. Both are compile-enforced at every site — neither type has a `Default` — so no
  consumer can silently miss the addition.
- `herdr-plugin.toml`, `config.toml`, and the state file format are **unchanged**. `? help` is
  a new keybinding and the proposal marks it as such; it is not BREAKING under the project's
  rule, which reserves that for a manifest, config, or *changed* keybinding.

## Persistence and Rollout

- **Migration**: none. No persisted format changes.
- **Backfill**: none.
- **Seeding**: none.
- **Cache invalidation**: none. `Detail::loaded`, the `(change directory, tab)` artifact cache,
  is untouched; the overlay reads no artifact and opening it invalidates nothing.
- **Index rebuild**: none.
- **Authorization**: none. The pane has no notion of a caller; the overlay is read-only and
  reaches no collaborator, so there is no privileged path to guard.
- **Observability**: none added. The plugin logs nothing today and this change does not start.
- **Deployment**: `make build` and the existing `[[build]]` step, unchanged. The plugin's own
  writes stay exactly `agent-names.toml` under `HERDR_PLUGIN_STATE_DIR`; the overlay writes
  nothing at all, and nothing is written inside `openspec/`.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| Terminal (crossterm raw mode, alternate screen) | **replaced** — `ratatui::backend::TestBackend`. No test reaches `src/ui/terminal.rs`; `cargo test` spawns this binary and a real raw-mode entry would corrupt the developer's session | replaced, same |
| Filesystem (`openspec/`, artifact reads) | **not reached** — the overlay reads no file; `Dashboard` values are built in memory | not reached |
| Filesystem (gate scripts' subject tree) | **real** — `tests/gate_controls.rs` copies `src/` to a `testutil::ScratchDir` and plants a defect | real (scratch copy only) |
| Filesystem (`SPEC.md`, `README.md`, `AGENTS.md`, gate scripts) | **real, read-only** — `tests/doc_contract.rs` reads the repository's own files as text | real, read-only |
| `openspec` binary (`OpenspecCli`) | **not reached** — no CLI call is added and none is removed | not reached |
| Herdr socket (`HerdrCli`) | **not reached** — the overlay reaches no collaborator; `agents.reachable` is a plain `bool` on an in-memory `AgentSnapshot` | not reached |
| Herdr agent poller / launcher / refresh worker / filesystem watcher | **not reached** — `run_loop`'s live tier is unchanged; no scenario here drives it | not reached |
| Process environment (`HERDR_PLUGIN_*`) | **not reached** — no new environment read; `env_lookup` is untouched | not reached |
| Clock | **not reached** — no scenario reads one, and `src/ui/` may not name one | not reached |
| `ui::app::action_for`, `ui::driver::mouse_action` | **real** — `tests/doc_contract.rs` executes them over a swept input space rather than parsing their source | real |
| `ui::help::INVENTORY` | **real** — a `'static` slice, compared against the swept set | real |

Three rows are the load-bearing ones. The terminal is replaced everywhere, with no exception.
The `openspec` binary and the Herdr socket are **not reached at all** — that is a stronger
statement than "replaced", and it is what makes every scenario here a `make test` scenario. And
`action_for`/`mouse_action` are **real** in the contract tier by design: the anti-drift claim is
worth nothing if the check runs against a double.

## Test Strategy

Every scenario lands in `make test`. **No scenario needs the outer-loop acceptance tier**, and
the reason is the Test Boundaries table above rather than a preference: this change reaches no
process, no socket, and no terminal, so there is no integration a slower tier could exercise
that a `TestBackend` render and a swept pure function do not.

Tiers used, in this repository's own terms:

- **unit** — `#[cfg(test)]` modules in `src/ui/app.rs`, `src/ui/driver.rs`, `src/ui/layout.rs`.
  Pure functions over values built in memory.
- **view** — `#[cfg(test)]` modules in `src/ui/help.rs` and `src/ui/view.rs`, rendering into a
  `TestBackend` at **60 and 120 columns**, both widths, every time (tasks rule 7).
- **contract** — `tests/doc_contract.rs`, which links the crate and reads the repository's own
  documents.
- **gate** — `scripts/gates/*.sh` run by `make gates`, each bound to a planted defect in
  `tests/gate-controls.toml` and executed by `tests/gate_controls.rs`.
- **wiring** — `ui::tests::wiring` in `src/ui/mod.rs`. The footer prepend breaks **four**
  assertions there across `agent-poller` and `agent-launch` fixtures, one of them a byte-exact
  `assert_eq!` on the full 120-column row and two of them 60-column assertions that lose
  `g focus`. Group 8 owns the repair; `grep -n 'q quit\|g focus' src/ui/mod.rs` finds the five
  sites, of which four move.

Commands: `make test` for the first five tiers' assertions, `make gates` for the gate scripts
themselves, and `make check` as the single gate before any group is called complete.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| `help-overlay` / `?` toggles the overlay and its near misses do not | `ui::app` unit over `apply` | unit | `Dashboard` in memory; none replaced | `make test` |
| `help-overlay` / The overlay opens and closes without moving the route | `ui::app` unit over `apply` | unit | `Dashboard` in memory; none replaced | `make test` |
| `help-overlay` / `Esc` closes the overlay before any other layer | `ui::app` unit over `apply` | unit | `Dashboard` in memory; none replaced | `make test` |
| `help-overlay` / The agent keys launch nothing while the overlay is open | `ui::app` unit over `apply` | unit | `Dashboard` in memory; none replaced | `make test` |
| `help-overlay` / Both quit keys still quit from inside the overlay | `ui::app` unit over `apply` | unit | `Dashboard` in memory; none replaced | `make test` |
| `help-overlay` / The overlay swallows the seventeen inert actions | `ui::app` unit over `apply` | unit | `Dashboard` in memory; none replaced | `make test` |
| `help-overlay` / `j` and `k` scroll the overlay rather than the frame beneath | `ui::app` unit over `apply` | unit | `Dashboard` in memory; none replaced | `make test` |
| `help-overlay` / The band's geometry at both mandated widths | `ui::help::tests` buffer assertion | view | `TestBackend` 60 and 120; no real terminal | `make test` |
| `help-overlay` / The band paints every cell it covers | `ui::help::tests` buffer assertion | view | `TestBackend` 60 and 120; no real terminal | `make test` |
| `help-overlay` / The frame beneath is unchanged when the overlay closes | `ui::help::tests` buffer assertion | view | `TestBackend` 60 and 120; no real terminal | `make test` |
| `help-overlay` / The overlay lists the agent keys when the socket is unreachable | `ui::help::tests` two-render byte-identity assertion | view | `TestBackend` 60 and 120; `AgentSnapshot` in memory, socket not reached | `make test` |
| `help-overlay` / The grammar renders at 120 columns | `ui::help::tests` buffer assertion | view | `TestBackend` 60 and 120; no real terminal | `make test` |
| `help-overlay` / The grammar renders at 60 columns | `ui::help::tests` buffer assertion | view | `TestBackend` 60 and 120; no real terminal | `make test` |
| `help-overlay` / The key column is measured in display columns | `ui::help::tests` buffer assertion | view | `TestBackend` 60 and 120; no real terminal | `make test` |
| `help-overlay` / The overlay scrolls at both mandated sizes | `ui::help::tests` buffer assertion | view | `TestBackend` 60 and 120; no real terminal | `make test` |
| `help-overlay` / A held key cannot run the window off the end | `ui::help::tests` buffer assertion | view | `TestBackend` 60 and 120; no real terminal | `make test` |
| `help-overlay` / No indicator when the content fits | `ui::help::tests` buffer assertion | view | `TestBackend` 60 and 120; no real terminal | `make test` |
| `help-overlay` / Degenerate frames render without panicking | `ui::help::tests` buffer assertion | view | `TestBackend` 60 and 120; no real terminal | `make test` |
| `help-overlay` / The reader is never trapped in a degenerate frame | `ui::app` unit over `apply` | unit | `Dashboard` in memory; none replaced | `make test` |
| `binding-inventory` / The inventory is a pure `'static` value with no construction cost | `tests/doc_contract.rs` | contract | crate linked; documents read as text | `make test` |
| `binding-inventory` / Every binding names a field explicitly | `tests/doc_contract.rs` | contract | crate linked; documents read as text | `make test` |
| `binding-inventory` / An action added without a help row fails `cargo test` | `tests/doc_contract.rs` | contract | crate linked; documents read as text | `make test` |
| `binding-inventory` / A binding removed from the driver and left in the help fails | `tests/doc_contract.rs` | contract | crate linked; documents read as text | `make test` |
| `binding-inventory` / The sweep finds the twenty-two bound actions and exactly two exemptions | `tests/doc_contract.rs` | contract | crate linked; documents read as text | `make test` |
| `binding-inventory` / The sweep covers the mouse under both overlay states | `tests/doc_contract.rs` | contract | crate linked; documents read as text | `make test` |
| `binding-inventory` / The inventory's shape is asserted, not described | `tests/doc_contract.rs` | contract | crate linked; documents read as text | `make test` |
| `binding-inventory` / `Space` and `Esc` each appear under their route | `tests/doc_contract.rs` | contract | crate linked; documents read as text | `make test` |
| `binding-inventory` / Both quit keys have a row | `tests/doc_contract.rs` | contract | crate linked; documents read as text | `make test` |
| `dashboard-loop` / The four action keys map, and their near misses do not | `ui::app::tests` unit | unit | `Dashboard` in memory; none replaced | `make test` |
| `dashboard-loop` / A launch action reaches no collaborator and starts no work | `ui::app::tests` unit | unit | `Dashboard` in memory; none replaced | `make test` |
| `dashboard-loop` / A refused launch records the reason and produces no request | `ui::app::tests` unit | unit | `Dashboard` in memory; none replaced | `make test` |
| `dashboard-loop` / An unreachable socket makes the four keys change nothing | `ui::app::tests` unit | unit | `Dashboard` in memory; none replaced | `make test` |
| `dashboard-loop` / Both quit keys quit and neither near-miss does | `ui::app::tests` unit | unit | `Dashboard` in memory; none replaced | `make test` |
| `dashboard-loop` / A released quit key does not quit | `ui::app::tests` unit | unit | `Dashboard` in memory; none replaced | `make test` |
| `dashboard-loop` / Enter and Esc move between the two routes | `ui::app::tests` unit | unit | `Dashboard` in memory; none replaced | `make test` |
| `dashboard-loop` / `Esc` dismisses one layer at a time | `ui::app::tests` unit | unit | `Dashboard` in memory; none replaced | `make test` |
| `dashboard-loop` / Navigation and filter keys are distinguished from near misses | `ui::app::tests` unit | unit | `Dashboard` in memory; none replaced | `make test` |
| `dashboard-loop` / Non-key events are ignored without panicking | `ui::app::tests` unit | unit | `Dashboard` in memory; none replaced | `make test` |
| `dashboard-loop` / `Space` maps to `ToggleSection` outside filter mode and types inside it | `ui::app::tests` unit | unit | `Dashboard` in memory; none replaced | `make test` |
| `dashboard-loop` / A pending launch request is handed over exactly once | `ui::driver::tests` one-iteration drive | unit | inert `Live` doubles; `TestBackend` | `make test` |
| `dashboard-loop` / A launch outcome updates the mapping and replaces the problem | `ui::driver::tests` one-iteration drive | unit | inert `Live` doubles; `TestBackend` | `make test` |
| `dashboard-loop` / A quit on the same event as a launch dispatches nothing | `ui::driver::tests` one-iteration drive | unit | inert `Live` doubles; `TestBackend` | `make test` |
| `dashboard-loop` / The first frame is on screen before the first event is read | `ui::driver::tests` `LoopSummary` assertion | unit | inert `Live` doubles; `TestBackend` | `make test` |
| `dashboard-loop` / Timeouts are not events and do not end the loop | `ui::driver::tests` scripted `EventSource` | unit | replaced `EventSource`; `TestBackend` | `make test` |
| `dashboard-loop` / A backend draw failure ends the loop rather than spinning | `ui::driver::tests` failing backend | unit | replaced backend; inert `Live` | `make test` |
| `dashboard-loop` / Ctrl-C ends the loop | `ui::driver::tests` scripted `EventSource` | unit | replaced `EventSource`; `TestBackend` | `make test` |
| `dashboard-loop` / An ignored key redraws and keeps waiting | `ui::driver::tests` `LoopSummary` assertion | unit | replaced `EventSource`; `TestBackend` | `make test` |
| `dashboard-loop` / A route change is visible in the next frame | `ui::driver::tests` buffer assertion | unit | replaced `EventSource`; `TestBackend` | `make test` |
| `dashboard-loop` / `Live` cannot be built without naming the poller | compile-time: `Live` has no `Default` | unit | none | `make test` |
| `dashboard-loop` / The overlay's scroll is clamped where the detail region's is not | `ui::driver::tests` one-iteration drive at 60x20 `Route::List` | unit | inert `Live` doubles; `TestBackend` | `make test` |
| `dashboard-loop` / `?` maps to `ToggleHelp` outside filter mode and types inside it | `ui::app::tests` unit | unit | `Dashboard` in memory; none replaced | `make test` |
| `dashboard-loop` / The overlay layer suppresses every action but seven | `ui::app::tests` unit | unit | `Dashboard` in memory; none replaced | `make test` |
| `dashboard-loop` / The overlay's seven live actions act and nothing else moves | `ui::app::tests` unit | unit | `Dashboard` in memory; none replaced | `make test` |
| `dashboard-loop` / `Dashboard` has no `Default` and no site elides a field | `scripts/gates/*.sh` + `tests/gate_controls.rs` plant | gate | scratch tree copy of the repo | `make gates` / `make test` |
| `dashboard-loop` / The pure view files name no I/O API | `scripts/gates/*.sh` + `tests/gate_controls.rs` plant | gate | scratch tree copy of the repo | `make gates` / `make test` |
| `dashboard-loop` / The shell never names the CLI seam | `ui::app::tests` unit | unit | `Dashboard` in memory; none replaced | `make test` |
| `dashboard-loop` / Change literals live only in the gated file | `scripts/gates/*.sh` + `tests/gate_controls.rs` plant | gate | scratch tree copy of the repo | `make gates` / `make test` |
| `dashboard-loop` / The render path names no channel, thread, lock, or clock | `ui::app::tests` unit | unit | `Dashboard` in memory; none replaced | `make test` |
| `dashboard-loop` / No test sleeps and then asserts something has already happened | `ui::app::tests` unit | unit | `Dashboard` in memory; none replaced | `make test` |
| `dashboard-loop` / `file_mode` is set by the composition root and by nothing else | `ui::app::tests` unit | unit | `Dashboard` in memory; none replaced | `make test` |
| `dashboard-loop` / The fifteenth field is named at every construction site | `ui::app::tests` unit | unit | `Dashboard` in memory; none replaced | `make test` |
| `responsive-layout` / The band's rectangle at both mandated widths | `ui::layout::tests` unit over `help_band` | unit | `Rect` values in memory | `make test` |
| `responsive-layout` / The band is total over degenerate and extreme rectangles | `ui::layout::tests` unit over `help_band` | unit | `Rect` values in memory | `make test` |
| `responsive-layout` / The overlay does not move the breakpoint | `ui::layout::tests` unit over `help_band` | unit | `Rect` values in memory | `make test` |
| `responsive-layout` / Body and footer occupy their rows at both widths | `ui::view::tests` buffer assertion | view | `TestBackend` 60 and 120; no real terminal | `make test` |
| `responsive-layout` / The action hints follow `Esc back` when the socket is reachable | `ui::view::tests` buffer assertion | view | `TestBackend` 60 and 120; no real terminal | `make test` |
| `responsive-layout` / The action hints are dropped whole, `g focus` first | `ui::view::tests` buffer assertion | view | `TestBackend` 60 and 120; no real terminal | `make test` |
| `responsive-layout` / A one-row frame renders the body's heading row and nothing else | `ui::view::tests` buffer assertion | view | `TestBackend` 60 and 120; no real terminal | `make test` |
| `responsive-layout` / A two-row frame renders one body row and the footer | `ui::view::tests` buffer assertion | view | `TestBackend` 60 and 120; no real terminal | `make test` |
| `responsive-layout` / A one-column frame renders without panicking | `ui::view::tests` buffer assertion | view | `TestBackend` 60 and 120; no real terminal | `make test` |
| `responsive-layout` / The footer drops whole hints rather than truncating one | `ui::view::tests` buffer assertion | view | `TestBackend` 60 and 120; no real terminal | `make test` |
| `responsive-layout` / The unattributed count is the footer's last hint at both widths | `ui::view::tests` buffer assertion | view | `TestBackend` 60 and 120; no real terminal | `make test` |
| `responsive-layout` / The count is reported with an empty change list | `ui::view::tests` buffer assertion | view | `TestBackend` 60 and 120; no real terminal | `make test` |
| `responsive-layout` / The count is dropped whole before the three key hints | `ui::view::tests` buffer assertion | view | `TestBackend` 60 and 120; no real terminal | `make test` |
| `responsive-layout` / The filter prompt replaces the count along with the hints | `ui::view::tests` buffer assertion | view | `TestBackend` 60 and 120; no real terminal | `make test` |
| `mouse-input` / The wheel scrolls the overlay from every region | `ui::driver::tests` unit over `mouse_action` | unit | `Dashboard` + `Rect` in memory | `make test` |
| `mouse-input` / A click inside the band does nothing | `ui::driver::tests` unit over `mouse_action` | unit | `Dashboard` + `Rect` in memory | `make test` |
| `mouse-input` / A click outside the band dismisses it and selects nothing | `ui::driver::tests` unit over `mouse_action` | unit | `Dashboard` + `Rect` in memory | `make test` |
| `mouse-input` / The band's edges are inside it | `ui::driver::tests` unit over `mouse_action` | unit | `Dashboard` + `Rect` in memory | `make test` |
| `mouse-input` / Motion still costs no frame while the overlay is open | `ui::driver::tests` unit over `mouse_action` | unit | `Dashboard` + `Rect` in memory | `make test` |
| `mouse-input` / Nothing in the overlay is mouse-only | `ui::driver::tests` unit over `mouse_action` | unit | `Dashboard` + `Rect` in memory | `make test` |
| `doc-conformance` / A binding added to the driver and not to the docs fails `make check` | `tests/doc_contract.rs` | contract | crate linked; documents read as text | `make test` |
| `doc-conformance` / The documented key set and the inventory agree at HEAD | `tests/doc_contract.rs` | contract | crate linked; documents read as text | `make test` |
| `doc-conformance` / A gutted document fails as a broken control rather than a clean tree | `scripts/gates/*.sh` + `tests/gate_controls.rs` plant | gate | scratch tree copy of the repo | `make gates` / `make test` |
| `doc-conformance` / The submodule is invisible to both map checks, and that is asserted rather than assumed | `tests/doc_contract.rs` assertion on `pub_mod_names(src/lib.rs)` | contract | `src/lib.rs` read as text | `make test` |
| `doc-conformance` / The gate lists and the prose counts agree | `scripts/gates/*.sh` + `tests/gate_controls.rs` plant | gate | scratch tree copy of the repo | `make gates` / `make test` |
| `view-palette` / The palette answers every role with a `Style` | `scripts/gates/palette.sh` + `tests/gate_controls.rs` plant | gate | scratch tree copy of `src/` | `make gates` / `make test` |
| `view-palette` / The confinement gate catches a `Color` named outside the palette | `scripts/gates/*.sh` + `tests/gate_controls.rs` plant | gate | scratch tree copy of the repo | `make gates` / `make test` |
| `view-palette` / The palette module reaches no I/O and measures no width | `scripts/gates/*.sh` + `tests/gate_controls.rs` plant | gate | scratch tree copy of the repo | `make gates` / `make test` |
| `list-filtering` / The prompt replaces the hints while filtering, at both widths | `ui::view::tests` buffer assertion | view | `TestBackend` 60 and 120 | `make test` |
| `list-filtering` / An accepted query leads the hint list, at both widths | `ui::view::tests` buffer assertion | view | `TestBackend` 60 and 120 | `make test` |
| `list-filtering` / The action keys type into the query rather than launching | `ui::view::tests` buffer assertion | view | `TestBackend` 60 and 120 | `make test` |
| `list-filtering` / A prompt longer than the footer keeps its tail | `ui::view::tests` buffer assertion | view | `TestBackend` 60 and 120 | `make test` |
| `agent-launch` / The action hints appear at both mandated widths when the socket is reachable | `ui::view::tests` buffer assertion | view | `TestBackend`; `AgentSnapshot` in memory | `make test` |
| `agent-launch` / An unreachable socket hides both hints at both widths | `ui::view::tests` buffer assertion | view | `TestBackend`; `AgentSnapshot` in memory | `make test` |
| `agent-launch` / The count is dropped before the action hints as the width falls | `ui::view::tests` buffer assertion | view | `TestBackend`; `AgentSnapshot` in memory | `make test` |
| `agent-launch` / The action keys type into the query while filtering | `ui::view::tests` buffer assertion | view | `TestBackend`; `AgentSnapshot` in memory | `make test` |
| `agent-poller` / The real wiring polls a scratch Herdr and adopts what it says | `ui::tests::wiring` composition-root test | wiring | scratch `#!/bin/sh` `herdr`; `TestBackend` | `make test` |
| `agent-poller` / A polled agent reaches a rendered badge and a rendered count | `ui::tests::wiring` composition-root test | wiring | scratch `#!/bin/sh` `herdr`; `TestBackend` | `make test` |
| `agent-poller` / The wiring test fails when the poller is replaced by the inert double | `ui::tests::wiring` composition-root test | wiring | scratch `#!/bin/sh` `herdr`; `TestBackend` | `make test` |
| `agent-poller` / An unreachable scratch Herdr leaves the pane a working standalone TUI | `ui::tests::wiring` composition-root test | wiring | scratch `#!/bin/sh` `herdr`; `TestBackend` | `make test` |
| `agent-poller` / A pane with no OpenSpec repository still polls for agents | `ui::tests::wiring` composition-root test | wiring | scratch `#!/bin/sh` `herdr`; `TestBackend` | `make test` |
| `agent-poller` / The residue left in `run` is small enough to read | `ui::tests::wiring` composition-root test | wiring | scratch `#!/bin/sh` `herdr`; `TestBackend` | `make test` |
| `agent-poller` / `startup_dir` prefers the workspace cwd and falls back when there is none | `ui::tests::wiring` composition-root test | wiring | scratch `#!/bin/sh` `herdr`; `TestBackend` | `make test` |
| `agent-poller` / `startup_dir` propagates a failing fallback rather than panicking | `scripts/gates/*.sh` + `tests/gate_controls.rs` plant | gate | scratch tree copy of the repo | `make gates` / `make test` |
| `agent-poller` / A Herdr context with no workspace cwd is the fallback case, not a failure | `ui::tests::wiring` composition-root test | wiring | scratch `#!/bin/sh` `herdr`; `TestBackend` | `make test` |
| `agent-attribution` / A refresh that reorders the list moves the badge with its change | `ui::app::tests` + `ui::view::tests` | unit / view | `TestBackend`; agents in memory | `make test` |
| `agent-attribution` / An unreachable socket yields no badge, no count, and no problem | `ui::app::tests` + `ui::view::tests` | unit / view | `TestBackend`; agents in memory | `make test` |
| `agent-attribution` / The `/` filter hides rows without changing the count | `ui::app::tests` + `ui::view::tests` | unit / view | `TestBackend`; agents in memory | `make test` |
| `quality-gates` / The harness renders a state value with no repository on disk | `ui::view::tests` buffer assertion | view | `TestBackend`; `ScratchDir` | `make test` |
| `quality-gates` / Both widths are exercised for every view scenario | `ui::view::tests` buffer assertion | view | `TestBackend`; `ScratchDir` | `make test` |
| `quality-gates` / The coverage floor is unchanged by the new module | `ui::view::tests` buffer assertion | view | `TestBackend`; `ScratchDir` | `make test` |

## Decisions

### Decision 1 — The inventory is bound by **executing** the driver, not by parsing it

`action_for(&Event, bool) -> Action` and `mouse_action(&Dashboard, Rect, &MouseEvent) -> Action`
are pure and total. The set of actions the pane binds is therefore **computable by calling
them**: sweep every `KeyCode` × modifier × filter mode, and every `MouseEventKind` × cell ×
overlay state, and collect the results. `tests/doc_contract.rs` compares that set against
`INVENTORY`'s.

A hand-written `fn action_name(Action) -> &'static str` with an **exhaustive** `match` and no
wildcard arm is what makes the enumeration honest: a variant added later is a compile error in
the test before it is a missing help row. Two names are exempt, asserted by name *and* by list
length so the exemption cannot grow silently: `FilterPush`, which is typing, and `Ignore`, which
is the absence of a binding.

*Alternatives considered.* A **procedural macro** deriving the inventory from the `match` arms:
the strongest possible binding and far too much machinery for one table, adding a `syn`/`quote`
dependency to a crate that argues over each of its six. **Parsing `src/ui/app.rs` as text**, the
way `documented_mouse_actions` parses `SPEC.md`: workable, and strictly weaker — a text parse
can disagree with the function it parses, where a call cannot. **A hand-written list with a
review checklist**: the status quo the change exists to end.

The check runs in **both directions**. An action bound and undocumented fails; a name documented
and unreachable fails too. The second direction is the one that catches a binding deleted from
`action_for` and left in the help, which is the likelier rot.

### Decision 2 — A layer, not a `Route` variant

`help.open: bool` sits beside `filter.active`, and `Route` stays `List`/`Detail`.

A third `Route` variant would have to remember which route to return to, duplicating state that
already exists. `filter.active` is the settled precedent for a mode that changes what keys mean
without being a route, and the overlay is exactly that mode one layer further out. `Esc`'s
existing "dismiss exactly one layer" rule extends by one entry at the front rather than being
rewritten.

*Alternatives considered.* `Route::Help` — rejected above. A `Vec<Layer>` **stack** — the
general shape, and premature: two layers do not need a stack, and `settings-window` will have
the evidence to justify one if it ever does.

### Decision 3 — A full-width band, not a centred box

The overlay spans the body's full width, is vertically centred, and is bounded by a top rule row
and a bottom rule row. No sides, no corners.

Two arguments, both already made in this repository. `pane-chrome` removed every border in the
pane and replaced each with a heading row and a rule; a boxed overlay reintroduces the one
construct that change removed. And a box costs four corner glyphs — `┌ ┐ └ ┘` — every one East
Asian **Ambiguous**, widening the uncompensated CJK-locale exposure `SPEC.md` records for the
seven glyphs already drawn. A band spends `─` alone, already in that set as the thematic break,
and adds no glyph class.

Full-width also removes a question rather than answering it: there is no overlay width to make
responsive, no second geometry to test at both breakpoints, and no horizontal centring
arithmetic to get wrong. The cost is stated plainly: at 120 columns the right half of each row
is empty where a two-column reflow would have halved the height. That is acceptable because the
band scrolls, so height was never the binding constraint.

*Alternatives considered.* A **centred box** — rejected above. **Two-column reflow above the
breakpoint** — the option the user was offered and declined; it needs its own geometry at both
widths for a layout no other requirement asks for.

### Decision 4 — It scrolls, and the indicator is digits rather than arrows

Thirty-nine content rows against a body that is 39 rows at 120x40 and 19 at 60x20: the overlay
is scrollable at **both** mandated sizes, which is a feature of the test surface rather than a
regret — every render test exercises the windowing path.

`help.scroll` is a line offset, clamped every frame by `layout::scroll_offset`, on exactly
`detail.scroll`'s terms. Reusing that function rather than writing a second clamp is what keeps
"a held key cannot run the content away" true by construction in a second place.

The indicator is `<first>-<last>/<total>`, right-inset one column in the bottom rule. Not
`▲ n/m ▼`: those two glyphs are Ambiguous, and they carry strictly less information than the
numbers that would sit between them.

*Alternatives considered.* **Clip with a count** — simplest, and it leaves the hidden bindings
undiscoverable, which is the problem the change exists to fix. **A scrollbar column** — a fifth
new glyph class for one bit of information.

### Decision 5 — `q` and `Ctrl-C` still quit from inside the overlay

Seventeen actions are inert while the overlay is open and one is not. A modal that traps the
reader is a worse failure than one that lets a quit through, and both keys have a row in the
inventory's `Pane` group saying so. This is also what makes the degenerate-frame scenarios safe:
at a 1x1 frame the view can draw almost nothing, but `apply` never receives a `Rect`, so every
exit key still works.

The four launch actions being inert is the load-bearing half of "read-only". A reader who opens
the help to find out what `a` does must be able to press `a` without spawning an agent.

### Decision 6 — `? help` is prepended, and `g focus` is dropped at 60 columns

Hints drop from the **end**, so the one key that reveals every other key must be first or it is
first lost. Prepending costs eight columns and takes the reachable footer from 53 to **61**,
which no longer fits the mandated 60-column frame: `g focus` is dropped there.

This is accepted, and it **repairs** `agent-launch`'s rationale rather than contradicting it.
That requirement rejected four separate action hints because dropping two would leave the reader
with "no indication that the other two exist" — an argument that held only while nothing on
screen pointed at the full list. `? help` is now that pointer. The footer is the always-visible
minimum; the overlay is the full list. The compound `a/c/s launch` is kept unchanged, so the
narrow footer still names an action key.

*Alternatives considered.* **Appending `? help`** — it would be the first hint dropped, which
inverts the priority. **Shrinking the footer to `q quit  ? help`** — frees 40 columns and makes
`a`/`c`/`s`/`g` invisible until the overlay is opened; it rewrites every landed footer
assertion for a worse resting state. **Compressing to `a/c/s/g agents`** — fits, and rewrites
what the action hints say to dodge a consequence better stated than hidden.

### Decision 7 — `mouse-input` is extended by an ADDED precedence requirement, not by MODIFYING two landed tables

The open overlay captures the wheel and the click. Both landed gesture requirements resolve the
pointer to a **region**, and while a modal covers the body there is no region to resolve to — so
the honest form is a new requirement that states its own precedence, conditional on
`help.open`.

The alternative was copying the 60-line wheel requirement and the 160-line click requirement
into `MODIFIED` blocks. `openspec/IMPLEMENTATION-ORDER.md` records what that costs with several
changes in flight: a `MODIFIED` block carries the **whole** requirement, so two changes
modifying one requirement silently discard the earlier's edits at archive time, and git reports
nothing. `mouse-text-selection` is in flight and overlaps `mouse-input`. One ADDED requirement
with an explicit precedence clause is both more accurate and a smaller merge target.

The same reasoning governs `responsive-layout`, which takes one ADDED requirement for
`help_band` and one MODIFIED for the footer — the footer's count and strings are literally false
otherwise, so there it is unavoidable.

### Decision 8 — No new palette `Role`

The overlay reuses six: `RegionRule` for its rules, `RegionHeadingFocused` for the `Help` title
and the group headings, `Strong` for `input`, `ListRow` for `description`, and `ListSeparator`
for the indicator. Minting `HelpTitle`, `HelpGroup`, `HelpKey`, and `HelpText` would add four
roles identical in every respect but their names to four existing ones, which is exactly the
drift `pane-chrome` removed `DetailHeader` to prevent. `view-palette`'s own rule — colour is
added only where it carries a distinction a modifier cannot — applies to roles as well as to
colours.

### Decision 9 — Two landed drifts are repaired in passing, and named

Both sit inside blocks this change must copy anyway, and leaving a known-false sentence in a
block being rewritten is worse than the scope it adds.

1. `dashboard-loop`'s pure-view file list reads **eight** and omits `src/ui/palette.rs`.
   `view-palette` corrected the count in its own spec and never carried it back. Repaired to
   **ten**, with both missing files named.
2. `quality-gates`' `TestBackend` scenario asserts row 0 spells `OpenSpec`. `pane-chrome`
   deleted that row. The claim is **removed** rather than updated; the footer half of the same
   assertion is the half that still discriminates, and it moves from `q quit` to `? help`.

### Decision 10 — A second normaliser, not a wider first one

`Dashboard::normalise_help_scroll(frame_area)` is a new function called beside
`normalise_scroll` in `run_loop`'s step 9, rather than a clamp added inside it.

`normalise_scroll` returns early when the detail region is not drawn (`src/ui/app.rs:868`,
`let Some(area) = detail_area else { return; }`), and `detail-scroll` requires exactly that of
it in as many words. The detail region is not drawn at `Route::List` below the breakpoint —
which is precisely the 60x20 fixture this change's own held-key scenario uses. Folding the
overlay's clamp in would have left `help.scroll` unbounded in the one case the scenario exists
to pin; widening `normalise_scroll` to not return early would need a `detail-scroll` delta
contradicting a landed requirement.

The two also read different geometry, which is the structural argument and the better one: the
band is computed from the **body**, the detail clamp from the detail region's content area.
Sharing one function was conflating two rectangles.

*Alternatives considered.* **Clamping inside `ui::help::render`** — impossible, views are pure
and cannot mutate `Dashboard`. **A `detail-scroll` delta widening `normalise_scroll`** — a
thirteenth delta on a capability `foldable-spec-sections` already touches, to make a landed
requirement less true.

**A note on `scroll_offset`'s signature.** It is `scroll_offset(lines, scroll, height)`
(`src/ui/layout.rs:121`), not `(scroll, lines, height)`. The two `usize` parameters are adjacent
and interchangeable to the compiler, so a transposed call compiles and silently clamps against
the wrong bound. Every call site in this change writes `lines` first.

### Decision 11 — Group 7 is not parallel, and the marker is withdrawn

An earlier draft marked the gate-script group `parallel-after: 4`. It fails all three
independence conditions and the marker is withdrawn.

Its planted controls land in `src/ui/help.rs`, which is the file groups 4, 6 and 9 all edit —
so the pair shares a file. `..Default::default()` in a `Binding` literal is a hard compile error
once `Binding` has no `Default`, so while that plant is in place every concurrent group's
`cargo test` fails on group 7's plant rather than on its own work — a failure that is not
attributable. And `make gates` sweeps the whole tree, so group 7's own runs would read the other
lane's half-written file.

The repair is two-part: the marker goes, and the plants move into `tests/gate-controls.toml`
where `tests/gate_controls.rs` applies them to a `testutil::ScratchDir` copy. The second half
matters independently of parallelism — planting in the real tree contradicts this design's own
Test Boundaries row, which says the gate scripts' subject tree is a scratch copy.

**No group in this change is parallel.** That is the finding, not an omission: groups 2-3 share
`src/ui/app.rs`, groups 8-9 share `src/ui/view.rs`, and groups 4-6 are a dependency chain.

## Risks / Trade-offs

- **The action sweep passes vacuously if the swept input space is too narrow** → the sweep is
  specified concretely (printable ASCII `' '..='~'`, thirteen named `KeyCode`s, four modifier
  values, both filter modes; every `MouseEventKind` at every cell of two frames under both
  overlay states) and the check asserts the swept set has the expected membership — twenty-four
  names — rather than only that it is a superset of the inventory.
- **The exemption list becomes an escape hatch** → closed at two names, asserted by name and by
  length. A third exemption requires a spec change.
- **Six capabilities carry byte-exact footer strings and seven changes are in flight** → the
  archive-order hazard is recorded in the proposal's Sequencing section with the mitigation
  `IMPLEMENTATION-ORDER.md` prescribes: re-extract each requirement block from
  `openspec/specs/<capability>/spec.md` before archiving, and compare by **phrase**, never by
  line, since a later archive may have re-wrapped it.
- **`g focus` disappears from the narrow footer** → stated as a decision rather than discovered
  as a bug, and answered by the overlay itself, which lists `g` under `Agents`.
- **Thirty-nine content rows make the overlay scrollable even at 120x40** → accepted; it means
  the windowing path is exercised at both mandated widths rather than only at the narrow one.
  If the inventory later shrinks below the body height, the "No indicator when the content
  fits" scenario is the one that keeps the non-scrolling path honest.
- **`?` is reported differently by different terminals** → matched under both
  `KeyModifiers::NONE` and `SHIFT`, with a scenario pinning both and pinning that `Char('/')`
  with `SHIFT` is *not* it.
- **A tenth pure view file is added and a gate list is not updated** → `view-palette`'s existing
  gate already fails when a pure-view module is missing from either `PURE` list; this change
  extends that leg to cover `src/ui/help.rs`, and adds its own planted control.
- **`scroll_offset`'s two `usize` parameters transpose silently** → a transposed call compiles
  and clamps against the wrong bound with no error. The scrolling scenarios assert the exact
  clamped value — `2` at 120x40 and `22` at 60x20 — rather than only that the window stayed in
  range, which is what makes a transposition red rather than plausible.
- **The overlay swallows a key the reader expected to work** → every suppressed action is
  enumerated in the spec as a closed list of seventeen, and a scenario applies all seventeen and
  asserts the dashboard is unchanged field for field.

## Migration Plan

None needed, and the reasons are worth stating rather than omitting. No persisted format
changes, so there is nothing to migrate or back-fill. No deploy order: the plugin is a single
binary Herdr launches, and the manifest is untouched, so `make build` and a pane restart is the
whole of it. Rollback is `git revert` of the change's commits — the overlay holds no state that
outlives the process, writes nothing, and leaves no artefact behind.

## Open Questions

None. The proposal's four are answered in it and implemented by the specs:

1. The inventory is derived by **executing** `action_for` and `mouse_action`, not by parsing
   them (Decision 1).
2. The overlay **scrolls**, with a digits-only position indicator (Decision 4).
3. `?` does **not** open the overlay while filtering; it types into the query.
4. The footer **keeps** its hints and gains `? help` first, at the measured cost of `g focus`
   at 60 columns (Decision 6).

## Visual Design

Not applicable. This change builds a terminal view, not an HTML view or an email template, and
no design source exists for it. The overlay's appearance is specified as a row grammar in
`specs/help-overlay/spec.md` and asserted against a `TestBackend` buffer at both mandated
widths, which is this repository's equivalent and is stronger than a static mock.
