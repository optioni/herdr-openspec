## Why

`herdr-openspec ui` reads the repository once, at startup, from files only. Nothing it shows
ever changes again: an agent finishing a task group in the pane beside it moves `[4/9]` to
`[5/9]` on disk and the dashboard keeps saying `[4/9]` until the user quits and reopens it.
And the `openspec` binary — authoritative for schema, artifacts, and progress — is never
consulted at all, in any change that has landed. `SPEC.md` → Dual-source model describes two
sources of a change, fast and authoritative; the pane has only ever had the fast one.

This is the Phase 4 `live-refresh` row of `openspec/IMPLEMENTATION-ORDER.md`, the last of
the phase. Both its dependencies have landed: `detail-view` (archived 2026-09-05) supplies
the loop and the detail region, and `changes-from-cli` (archived 2026-09-04) supplies
`changes::from_cli` and `changes::merge` — written, fully tested, and never once called at
runtime. This change is what finally runs them.

## What Changes

- **A `notify` watcher on `<repo>/openspec/`, debounced at 150ms.** One recursive watch. The
  debounce is a pure state machine taking `now` as a **parameter**, so it is proven with
  fabricated instants rather than by sleeping.
- **Per-change invalidation, not a full reload.** A touched path is classified to a
  `Selection`: a file under `openspec/changes/<name>/` invalidates that change and no other.
  `changes::from_cli_cached` reruns `openspec instructions apply` only for the selected
  names, reusing a cache for the rest; `openspec list --json` still runs every cycle, so
  every change's progress is fresh even when its artifacts are cached.
- **A worker thread running every CLI call off the render path**, reached through the
  `OpenspecCli` trait — it does not spawn. It answers one request with **two** results:
  the file-sourced `ChangeSet` first (sub-millisecond), the CLI-merged one second
  (200–400ms). That is the dual-source model made mechanical rather than described.
- **`r` forces a full refresh.** `SPEC.md` → Keys already reserves the key; it was
  `Action::Ignore` until now. Not **BREAKING**: no existing binding moves, and while
  filtering `r` still types itself, by `list-filtering`'s unchanged rule.
- **`Dashboard` gains a ninth field, `refresh: Refresh`** — `requested`, `reload`, and
  `problems`. `Detail` still carries exactly five fields.
- **Two new modules outside the render seam**: `src/watch.rs` (the only file naming
  `notify`) and `src/refresh.rs` (the only file spawning a thread). Neither is under
  `src/ui/`, so `NOCLI-SHELL` and `NOBLOCK` legs 1–2 prove the CLI and the worker are named
  nowhere on the render path, and `NOBLOCK` leg 3 proves the two functions the loop calls on
  every frame — `FsEvents::drain` and `Refresher::take_result` — never block. Legs 1 and 2
  alone would not: a `drain` written as `rx.recv_timeout(150ms)` satisfies both while
  delaying every draw, which is why leg 3 searches inside the two seam modules themselves.
- **A new dependency, `notify` 8.2.0**, `default-features = false, features =
  ["macos_fsevent"]`. MSRV 1.77, below the crate's 1.88 floor; +5 packages per macOS triple
  and +6 per Linux triple; no proc-macro. `notify-debouncer-mini` was measured and rejected:
  it costs five further packages including `tempfile` as a normal dependency of the shipped
  binary, for a coalescing loop this change writes in twenty lines.
- **Every degraded path is a test, not a note**: no `openspec` binary at all, a CLI that
  errors, a CLI that returns garbage, a watcher that will not start, and `openspec/` deleted
  while running. Files still paint in every one.
- **No manifest change, no configuration change, no key removed, no write anywhere inside
  `openspec/`.**

## Non-Goals

- **Any Phase 5 work**: no `herdr agent list` poll, no attribution, no agent badge, no
  launch keys. This change adds the first background worker; the second poller is
  `agent-polling`'s.
- **A second refresh trigger.** No inotify-free polling fallback, no `pane.focused` hook, no
  periodic re-read on a timer. The watcher and `r` are the only two triggers.
- **Rewriting `changes::from_cli` or `changes::merge`.** `from_cli` is *redefined* as
  `from_cli_cached` with `Selection::All` and an empty cache, so every landed `cli-changes`
  and `change-merge` scenario stays green unmodified.
- **Editing a task checkbox, or any other write inside `openspec/`.** The watcher makes the
  read-only guarantee harder to keep, not weaker.
- **A visible refresh indicator** — no spinner, no `file mode` badge, no "last updated"
  line. `degraded-states` owns the badge; a failed watch is a `!`-marked problem row, which
  `change-rows` already draws.
- **Lowering `ui::driver::TICK`.** It stays 250ms; the loop's wake-up shortens to the
  debounce deadline instead, which is a pure function of two `Duration`s.

## Capabilities

### New Capabilities

- `watch-invalidation`: the `notify` watcher confined to one module, the 150ms debounce as a
  pure state machine over an injected `now`, the classification of a touched path to a
  `Selection`, the loop's shortened poll timeout, and the degraded paths — a watcher that
  will not start, and `openspec/` removed underneath a running one.
- `refresh-worker`: the `Refresher` seam, `changes::Selection` and `changes::CliCache`, the
  per-change `changes::from_cli_cached`, the worker thread that answers a request with the
  file result then the merged one, and the no-binary and CLI-failure degraded paths.
- `live-updates`: the loop's live tier — `Dashboard::refresh`, `Action::Refresh` and the `r`
  key, `Dashboard::adopt` preserving the selection by name, `sync_detail`'s forced reload
  that keeps the scroll, and the rendering of `refresh.problems`. Files paint, the CLI
  corrects, and no frame waits for either.

### Modified Capabilities

- `dashboard-loop`: `Dashboard` carries **nine** fields; `action_for` maps to **thirteen**
  actions; `run_loop` takes a `Live` tier alongside its event source; `ui::load` starts with
  a refresh requested. The eight pure view files are unchanged as a set.
- `cli-changes`: `from_cli` is redefined in terms of `from_cli_cached(cli, repo,
  &Selection::All, &mut CliCache::default())`; a cached change keeps its schema and
  artifacts and takes its progress from the fresh `list --json`.
- `artifact-content`: `sync_detail` honours a forced reload — it re-reads an unchanged
  `(dir, tab)` key and resets the scroll only when the key itself changed.
- `change-rows`: the list's leading `!`-marked rows are `refresh.problems` followed by
  `ChangeSet::problems`.
- `plugin-build`: the argued dependency set gains `notify`; the graph snapshot grows by five
  lines per macOS triple and six per Linux triple, and the macOS/Linux delta becomes four
  named packages rather than one.
- `quality-gates`: the enumerated uncoverable residue gains only `ui::run`'s two live-tier
  wiring lines and one worker arm no deterministic test constructs; `src/watch.rs` and
  `src/refresh.rs` are otherwise covered by real tests and add none.
- `tasks-checklist`: `Action` holds **thirteen** variants rather than twelve, the read-only
  sweep covers `src/watch.rs` and `src/refresh.rs`, and the printable-key run now includes
  `r`.
- `detail-scroll`: `Detail` still carries **five** fields — the forced-reload flag lives on
  `Refresh` instead — while the `Dashboard` companion names **nine** and the no-`Default`
  check runs over four types.

## Impact

- **Code:** `src/watch.rs` and `src/refresh.rs` (both new, both outside `src/ui/`),
  `src/changes.rs` (`Selection`, `CliCache`, `from_cli_cached`), `src/ui/app.rs`
  (`Refresh`, `Action::Refresh`, `adopt`, `sync_detail`), `src/ui/driver.rs` (`Live` and the
  live tier), `src/ui/list.rs` (the problem rows), `src/ui/mod.rs` (`ui::load`'s startup
  request and `ui::run`'s wiring), `src/lib.rs` (`pub mod watch; pub mod refresh;` and two
  test doubles in `testutil`). `Dashboard`'s ninth field touches every `Dashboard { … }`
  literal, all compile-enforced by `NODEFAULT-UI`'s no-`Default`/no-`..` rule.
- **Dependency:** `notify = { version = "8.2.0", default-features = false, features =
  ["macos_fsevent"] }` in `Cargo.toml`; `Cargo.lock` and `tests/fixtures/build-graph.txt`
  regenerated (310 → 332 lines).
- **Docs:** `SPEC.md` → Refresh (rewritten), Architecture and Module map (a `refresh` row and
  the clock as a fourth one-line binding to the real world), Keys (`r`'s filter-mode
  behaviour), Degraded states (four rows added), Stack (`notify`'s version and features), and
  Testing → Unit-tested modules (the two new modules); `AGENTS.md` → Current repo state, the
  dependency count, and one new Architecture rule.
- **Checks:** twenty-six files in `$CHECKS`, of which twenty-five are gates and one
  (`TESTCOUNT.sh`) is a sourced helper. `WATCHSEAM`, `NOSLEEP`, and `NOBLOCK` are added, each
  with a planted-violation control; `READONLY-UI` gains an `EXTRA` search list, and `DEPS`,
  `GRAPH-SNAP`, and `OPENSPEC-UNTOUCHED` are edited **on disk** and re-run; the other nineteen
  are carried forward byte-identically and move only a floor in their invocation.
- **Data model, jobs, external services, sibling repositories:** none.
