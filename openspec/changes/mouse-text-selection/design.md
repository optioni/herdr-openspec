## Context

`mouse-input` enabled mouse capture at start-up and never turned it off again. A terminal
with reporting on routes drags to the application, so the pane gained click, wheel, fold, and
tab-switch and lost the terminal's own text selection.

This change's investigation is finished and recorded in `notes/measurements.md`: two runs of
`notes/probe.sh`, Ghostty inside a Herdr pane and Ghostty bare, identical row for row. What
they establish, and what this design is built on:

- Plain drag-selection is suppressed under **every** mode set and works only with reporting
  fully off. No mode set and no code in this crate changes that — it is the terminal's
  decision.
- `Shift`+drag restores selection under every mode set. `Option`+drag does **not**, which
  falsifies a sentence already in `specs/mouse-input/spec.md`.
- Narrowing the enabled DEC modes buys nothing for selection, while `?1000 ?1006` alone was
  measured to deliver click and wheel intact.
- Herdr is not involved.

So the only way to give the reader plain drag-selection is to stop reporting, and the only way
to keep the bindings is to start again afterwards. That is a toggle, and the seam it needs
already exists: `TerminalOps` has both `enable_mouse` and `disable_mouse`, and `TerminalGuard`
already treats a refused capture as a stored, non-fatal problem.

## Goals / Non-Goals

**Goals:**

- A key that releases mouse capture for as long as the reader wants, and restores it.
- The pane says so while capture is released, at every width and at every route.
- The panic-restore guarantee survives a toggle without the hook gaining state to consult.
- The falsified `Option` claim is corrected where it is written down.

**Non-Goals:**

- Selection, a clipboard, or a copy buffer inside the TUI. The terminal already does this.
- Narrowing the capture mode set. Measured to buy nothing here; a separate change with a
  separate justification if anyone wants the dead modes gone.
- Changing the default capture state, or any existing binding.
- Claiming the `Shift` bypass on terminals it was never measured on.

## Boundaries

| Piece | Where it lands | Pattern it follows |
|---|---|---|
| `TerminalGuard::set_mouse(bool)` | `src/ui/terminal.rs` | the file's existing rule: it is the only one naming a crossterm terminal-mode function, and the method calls `TerminalOps`, never crossterm directly |
| the injected capture seam | `src/ui/mod.rs` | **exactly** `ui::read_artifact`: one production binding in `mod.rs`, threaded into `run_loop` as a `&dyn Fn`, never named by a view |
| `Action::ToggleMouse`, `mouse_capture` | `src/ui/app.rs` | a pure action over pure state, on `Action::Refresh`'s terms |
| `m`'s inventory row | `src/ui/help.rs` | a `Binding` in the `Pane` group reaching a real `Action` — no `EXEMPT_ACTIONS` entry |
| `mouse off (m)` badge | `src/ui/view.rs` footer | the footer's existing conditional hints (`a/c/s launch`, `g focus`), which appear and vanish on state |
| `Role::MouseOff` | `src/ui/palette.rs` | the only file naming `ratatui::style::Color`; every call site asks for the role |

`src/ui/driver.rs` is **not** touched. Whether a gesture arrives is the terminal's decision
and the loop's; the resolver stays a pure, total function of event and frame with no capture
parameter.

## Contracts

The pane is the only consumer. `Action` and `Dashboard` are crate-internal; no Herdr surface,
no `config.toml` key, no manifest field, and no CLI argument changes.

**Additive, not breaking.** Capture is entered at start-up exactly as today and
`mouse_capture` initialises to `true`, so a reader who never presses `m` sees a pane
byte-identical to the current one — the specs assert that byte-identity rather than asserting
it in prose.

Error surface: the seam answers `Result<(), String>`, carrying the `TerminalError`'s own
`Display` text. A failure is rendered as a `!`-marked problem row and changes no state.

## Persistence and Rollout

- **migration** — none. **backfill** — none. **seeding** — none.
- **cache invalidation** — none. The `(change directory, tab)` artifact cache is untouched;
  capture state is not part of its key and does not invalidate it.
- **index rebuild** — none. **authorization** — none; the pane has no actors.
- **observability** — a refused capture change becomes a rendered problem row and a new
  `SPEC.md` degraded-states row bound to a named test in `tests/degraded-coverage.toml`.
- **deployment** — `make build` and the existing Herdr pane; no manifest change, so no
  `herdr plugin link` re-run is required.
- **The plugin's own writes are unchanged**: still exactly `agent-names.toml` under
  `HERDR_PLUGIN_STATE_DIR`. Capture state is per-session and is deliberately **not**
  persisted — see Decision 6.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| The real terminal (raw mode, alternate screen, capture) | **replaced** — never real, at any tier. `cargo test` spawns this binary, so a test reaching `CrosstermOps` corrupts the developer's own session | **replaced** — the existing `Recorder` double implementing `TerminalOps` |
| `TerminalOps` | replaced (`Recorder`) | replaced (`Recorder`) |
| The capture seam `&dyn Fn(bool) -> Result<(), String>` | replaced — a closure recording calls and answering `Ok`/`Err` on demand | replaced — same |
| ratatui backend | **real** `TestBackend`, in memory, at 120x20 and 60x20 | real `TestBackend` |
| The artifact reader `&dyn Fn(&Path) -> Result<String, String>` | replaced — the injected reader already used by every loop test | replaced |
| Filesystem (`openspec/` tree) | real, a scratch directory, only where a test already builds one | not reached |
| `openspec` CLI (`OpenspecCli`) | **not touched** by this change; replaced wherever an existing test already reaches it | not touched |
| Herdr CLI (`HerdrCli`) | **not touched**; replaced wherever an existing test reaches it | not touched |
| `notify` watcher, refresh worker, agent poller, launcher | **not touched**; already behind non-blocking trait objects and replaced in every loop test | not touched |
| The process environment | replaced — the injected `&dyn Fn(&str) -> Option<String>`, never `std::env::var` | replaced |

## Test Strategy

Tiers in this repository: **unit** (inline `#[cfg(test)]` modules, `cargo test`), **contract**
(`tests/*.rs`, also `cargo test`), and **gates** (`make gates`). There is no containerised or
network tier and nothing here adds one.

**This change does not take an outer-loop acceptance test in the usual sense, and the reason
is structural rather than a shortcut:** the only true end-to-end proof would drive a real
terminal, and this repository forbids that at every tier because `cargo test` spawns this
binary. The outermost honest test is `run_loop` driven over a `TestBackend` with every
collaborator replaced — which is what the loop scenarios below are — and that is the same
outer loop every landed change in this crate has used.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| Releasing and re-entering records exactly the two operations | new `#[test]` in `src/ui/terminal.rs` extending the `Recorder` suite | unit | `TerminalOps` replaced | `cargo test ui::terminal` |
| Teardown is unconditional whatever state capture is left in | new `#[test]`, normal drop and `catch_unwind` drop | unit | `TerminalOps` replaced | `cargo test ui::terminal` |
| A start-up refusal is not overwritten by a later success | new `#[test]` asserting `mouse_problem()` after a successful `set_mouse(true)` | unit | `TerminalOps` replaced | `cargo test ui::terminal` |
| A refused release is a problem row and no state change | new `#[test]` in `src/ui/mod.rs`'s loop suite, seam answering `Err` | unit | seam + backend replaced | `cargo test ui::` |
| The documented bindings match the resolver | existing `tests/doc_contract.rs` leg, unchanged — regression | contract | files on disk | `cargo test --test doc_contract` |
| The documented confined set matches the gate | existing `tests/doc_contract.rs` leg, unchanged — regression | contract | files on disk | `cargo test --test doc_contract` |
| The documented bypass names what was measured | new `tests/doc_contract.rs` leg reading `SPEC.md`, with a recorded negative control | contract | `SPEC.md` | `cargo test --test doc_contract` |
| The resolver gains no capture parameter | `src/ui/driver.rs` compiles unchanged; its existing suite passes | unit | none | `cargo test ui::driver` |
| A released capture withdraws the gestures and nothing else | render both dashboards, compare buffers cell by cell at 120x20 and 60x20 | unit | `TestBackend` real | `cargo test ui::view` |
| `Dashboard` has no `Default` and no site elides a field | existing sweep + compile-time companion — regression, count moves to sixteen | unit | none | `cargo test ui::app` |
| The pure view files name no I/O API | existing `NOIO-VIEW` gate — regression | gates | files on disk | `make gates` |
| The shell never names the CLI seam | existing `NOCLI-SHELL` gate — regression | gates | files on disk | `make gates` |
| Change literals live only in the gated file | existing gate — regression | gates | files on disk | `make gates` |
| The render path names no channel, thread, lock, or clock | existing `NOBLOCK` gate — regression; the seam is a `Fn`, not a channel | gates | files on disk | `make gates` |
| No test sleeps and then asserts something has already happened | existing gate — regression | gates | files on disk | `make gates` |
| `file_mode` is set by the composition root and by nothing else | existing `#[test]` — regression | unit | replaced | `cargo test ui::` |
| Every field is named at every construction site | existing sweep and companion, updated to sixteen | unit | none | `cargo test ui::app` |
| `mouse_capture` starts true and only the toggle moves it | new `#[test]` over `ui::load` and `run_wired` | unit | replaced | `cargo test ui::` |
| `m` toggles at both routes and types while filtering | new `#[test]` table-driving `action_for` over both routes and both filter modes | unit | none | `cargo test ui::app` |
| A refused release leaves the state it failed to leave | new `#[test]`, seam answering `Err`, asserting the flag and the problem row | unit | seam replaced | `cargo test ui::` |
| The keyboard is unaffected by a released capture | new `#[test]` stepping both dashboards through one key sequence and comparing | unit | seam replaced | `cargo test ui::app` |
| The inventory's shape is asserted, not described | existing `#[test]`, counts updated to 5,7,4,5,5,6 summing to 32 | unit | none | `cargo test ui::help` |
| `Space` and `Esc` each appear under their route | existing `#[test]` — regression | unit | none | `cargo test ui::help` |
| Both quit keys have a row | existing `#[test]` — regression | unit | none | `cargo test ui::help` |
| `m` has a row in the `Pane` group | new `#[test]`, plus the existing executed binding sweep in `tests/doc_contract.rs` | unit + contract | none | `cargo test` |
| The badge is drawn first and is additive | new render `#[test]` at 120x20 and 60x20, asserting columns and `DIM` | unit | `TestBackend` real | `cargo test ui::view` |
| The badge survives a width that drops every hint | new render `#[test]` at 14x20, 13x20, 12x20 | unit | `TestBackend` real | `cargo test ui::view` |
| A released capture drops `g focus` at the mandated narrow width | new render `#[test]` at 60x20, reachable socket | unit | `TestBackend` real | `cargo test ui::view` |
| Both badges can be on screen at once and are distinguishable | new render `#[test]` with `file_mode` and released capture both set | unit | `TestBackend` real | `cargo test ui::view` |

## Decisions

**Decision 1 — The capture change is an injected `&dyn Fn(bool) -> Result<(), String>`, not a
`&TerminalGuard` parameter.** `run_loop` already takes the artifact reader this way, so the
pattern is the crate's own rather than a new one, and a test drives the toggle with a closure
instead of constructing a guard. *Alternative considered:* pass `&TerminalGuard` into
`run_loop`. Rejected — it puts a terminal-owning type in the loop's signature, and
`NOCLI-SHELL`-style confinement is easier to keep when the loop names a function type rather
than a struct that owns real terminal state.

**Decision 2 — `mouse_capture` is a `Dashboard` field, making sixteen.** The footer renders it
and every view is a pure function of `Dashboard`, so it cannot live anywhere the views cannot
see. *Alternative considered:* a local in `run_loop`. Rejected — the footer could not read it.
*Alternative considered:* a fourth `Refresh` field. Rejected for the reason `file_mode` is a
`Dashboard` field: `Refresh`'s fields are one-shot flags the loop consumes or are replaced
wholesale on any iteration, and a toggle's state must survive both.

**Decision 3 — The badge goes in the footer, not the list region's heading row.** Below the
100-column breakpoint at `Route::Detail` the list region is not drawn at all, and the detail
route is exactly where a reader releases capture to copy a requirement out of a spec — a
heading-row badge would be invisible at the moment it is most needed. The footer is drawn at
every width and route. *Alternative considered:* the heading row beside `file mode`, with a
combined drop-whole rule. Rejected for the invisibility above, and because two right-aligned
badges competing for one row needs an ordering rule that buys nothing.

**Decision 4 — The badge is `mouse off (m)` and carries its own key.** `mouse off` names a
state and offers no way out of it. The reader who pressed `m` by accident needs the way back
where they are already looking. *Alternative considered:* `mouse off` alone, nine columns to
match `file mode`. Rejected — the symmetry is pleasing and the omission is a trap.

**Decision 5 — The key is `m`.** Free, mnemonic, and harmless when mis-keyed. Its neighbours
in the `Pane` and `Agents` groups — `a`, `c`, `s`, `g` — all start or focus an agent, so a
slip there starts a process; a slip onto `m` releases capture and pressing it again undoes it.
*Alternatives considered:* `M` (shifted keys are used nowhere in the pane), `Ctrl-M` (which is
`Enter`, and would be a genuine bug).

**Decision 6 — Capture state is per-session and is not persisted.** It is not written to
`agent-names.toml`, not added to `config.toml`, and not remembered across restarts. The pane's
one write stays what `SPEC.md` says it is. A reader who wants capture off permanently is
asking for a configuration key, which is a different change with a different argument.
*Alternative considered:* a `config.toml` key for the start-up state. Rejected as scope, and
because the measured problem is momentary — select, copy, resume.

**Decision 7 — `restore_then` keeps calling `disable_mouse` unconditionally.** This is the
proposal's Open Question 3, and the answer is that nothing has to change: the hook already
consults no state, which is exactly what makes it safe across a toggle. Making it conditional
on live capture state would require reading that state from a panicking thread, which is the
hazard the unconditional call was chosen to avoid in `mouse-input`. Writing a disable sequence
to a terminal that is not reporting is inert.

**Decision 8 — A refused change leaves the old state.** No optimistic flip. A pane claiming
capture is released while the terminal is still reporting would show the badge while the mouse
keeps being captured — the worst of both, and unfalsifiable from the reader's seat.

**Decision 9 — `Role::MouseOff` is a new palette role, not a reuse of `Role::FileMode`.** Both
badges can be on screen at once. Two badges sharing a full style while meaning unrelated
things is the collision Decision 3's placement exists to avoid, and reusing the role would
reintroduce it one layer down. Both are `DIM`, because both name a mode rather than a fault.

**Decision 10 — The mode set is not narrowed.** Measured to buy nothing for selection. Left
alone deliberately so that a future change removing `?1002`/`?1003`/`?1015` has to make its
own argument — dead-cost removal — rather than inheriting this change's.

## Risks / Trade-offs

- **A redraw mid-selection makes the copied text stale.** The loop still draws when a refresh
  or an agent poll is adopted, and a terminal's selection is a region of the screen, not of
  the text. → Not mitigated in code, and deliberately: pausing refreshes while capture is
  released would make the pane lie about the repository to protect a selection, and the window
  is a second or two. Recorded here so a later change does not "fix" it that way.
- **Footer width pressure.** Fifteen columns pushes the reachable footer to 76, dropping
  `g focus` at the mandated 60-column frame. → Accepted on the terms the capability already
  accepts dropping `s archive`; the keys stay in `SPEC.md`, `README.md`, and the help overlay,
  and the badge is only present while capture is released.
- **One terminal family measured.** Ghostty, macOS. → The documentation says exactly that and
  claims nothing about iTerm2, Terminal.app, or Linux. The toggle itself is terminal-agnostic:
  it works by not reporting, which every terminal understands.
- **Three pinned counts move at once** — `INVENTORY`'s group counts (to 5,7,4,5,5,6 = 32), the
  overlay's row count (32 bindings + 6 headings + 5 blanks = **43**, from 42), and
  `Dashboard`'s field count (to sixteen). → All three are asserted in `cargo test`, and the
  failure names both sides; the task list moves them in one group so the tree is never half-way.
- **`m` is not reachable while filtering.** A reader filtering the list cannot release capture
  without leaving filter mode first. → Accepted: `m` must type there, as every printable key
  does, and `Esc` is one key away.

## Migration Plan

No migration, no backfill, no rollback procedure. The change is additive and per-session: it
adds a field initialised to today's behaviour, a key, and a conditional footer badge. Reverting
is deleting them. Deploy order is a single `make build`; the manifest is untouched, so an
already-linked plugin picks it up.

## Open Questions

None. The proposal's three are closed: the mode-set question by the measurement (Decision 10),
the badge's home by Decision 3, and the panic-safety question by Decision 7.
