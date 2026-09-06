# Gate floors — measured at BASE (`31fc7f5`), re-verified at task 0.4/0.5/0.6

All commands run from the repository root. `$C` =
`openspec/changes/archive/2026-09-06-spec-purposes/notes/extracted-gates`.

| Gate | Floor variable (default) | Measured at BASE | Invocation |
|---|---|---|---|
| `NOSPAWN-GREP` | `MIN` (20) | 24 | `MIN=24 sh $C/NOSPAWN-GREP.sh` |
| `NOLIT-CHANGE` | `MIN` (20) | 24 | `MIN=24 sh $C/NOLIT-CHANGE.sh` |
| `MDSEAM` | `MIN` (16) | 24 | `MIN=24 sh $C/MDSEAM.sh` |
| `WATCHSEAM` | `MIN` (20) | 28 | `MIN=27 sh $C/WATCHSEAM.sh` -> "28 files searched" |
| `AGENTSEAM` | `MIN` (20), `ALLOWED` (3 files) | 24, 5 files | `MIN=23 ALLOWED='src/cli.rs src/agents.rs src/ui/mod.rs src/launch.rs src/open.rs' sh $C/AGENTSEAM.sh` |
| `LAUNCHSEAM` (`src/launch.rs`) | `MIN` (22), `ALLOWED` (4 files) | 24, 5 files | as above, `ENTRY='pub fn start\('` |
| `LAUNCHSEAM` (`src/open.rs`) | `MIN`, `LAUNCH`, `ENTRY` | 24 | as above, `LAUNCH=src/open.rs ENTRY='pub fn run_from_env\('` |
| `NOCLI-SHELL` | `UI_MIN` | 11 | `UI_MIN=11 sh $C/NOCLI-SHELL.sh` |
| `READSEAM` | `UI_MIN` (7) | 10 | `UI_MIN=10 sh $C/READSEAM.sh` |
| `NOBLOCK` | `UI_MIN` | 11 | `UI_MIN=11 sh $C/NOBLOCK.sh` |
| `READONLY-UI` | `UI_MIN` (10), `EXTRA` (empty) | 11, `EXTRA='src/watch.rs src/refresh.rs src/agents.rs src/launch.rs src/open.rs'` | bare runs `EXTRA` empty, weaker than `plugin-actions` ran it |
| `NORAW-GREP` | none exposed (16 hardcoded inside) | 28 files searched | `sh $C/NORAW-GREP.sh` |
| `NOSLEEP` | `MIN` (20), `SLEEP_MIN` (3) | 28, **6** | `SLEEP_MIN=6 MIN=28 sh $C/NOSLEEP.sh` |
| `NODEFAULT-UI` (Dashboard/Filter/Detail set, `HOMEFILE=src/ui/app.rs` default `TYPES`) | `SCAN_MIN` (60) | 126 | `sh $C/NODEFAULT-UI.sh` |
| `NODEFAULT-UI` (`TYPES='Refresh'`, same `HOMEFILE`) | `SCAN_MIN` (60) | **51 — below default** | `TYPES='Refresh' SCAN_MIN=51 sh $C/NODEFAULT-UI.sh` |
| `NODEFAULT-UI` (`TYPES='Launch'`, same `HOMEFILE`) | `SCAN_MIN` (60) | 78 | `TYPES='Launch' SCAN_MIN=78 sh $C/NODEFAULT-UI.sh` |
| `NODEFAULT-UI` (`HOMEFILE=src/agents.rs`, `TYPES='Agent Listed AgentSnapshot Attribution'`) | `SCAN_MIN` (60) | 103 | `HOMEFILE=src/agents.rs TYPES='Agent Listed AgentSnapshot Attribution' SCAN_MIN=103 sh $C/NODEFAULT-UI.sh` |
| `NODEFAULT-UI` (`HOMEFILE=src/launch.rs`, `TYPES='Outcome'`) | `SCAN_MIN` (60) | **23 — below default, and re-measured after group 6 widens `Outcome`** | `HOMEFILE=src/launch.rs TYPES='Outcome' SCAN_MIN=<remeasured> sh $C/NODEFAULT-UI.sh` |
| `GATE-MECH1` | none | 25 files, 84 constructions | `python3 $C/GATE-MECH1.py` |
| `WIDTHS` | `WIDTHS_MIN` (16) | 94 | `WIDTHS_MIN=94 sh $C/WIDTHS.sh` |
| `LISTWIDTHS` | `LIST_MIN` (17) | 29 | `sh $C/LISTWIDTHS.sh` |
| `MDWIDTHS` | `MD_MIN` (23) | 24 | `sh $C/MDWIDTHS.sh` |
| `TASKWIDTHS` | `TASK_MIN` (14) | 16 | `sh $C/TASKWIDTHS.sh` |
| `DETAILWIDTHS` | `DETAIL_MIN` | 23 | `DETAIL_MIN=23 sh $C/DETAILWIDTHS.sh` |
| `NOJSON-SEAM`, `NOTABSEAM`, `TASKSEAM`, `NOWAIVER` | none | structural | bare |

`WIRED` reproduced red at BASE (0.3): leg 2, `'pub fn run()' holds a branch or a loop`.

Note on `NODEFAULT-UI`'s per-type-set floors: the four sets (`Dashboard`/`Filter`/`Detail`,
`Refresh`, the `src/agents.rs` set, the `src/launch.rs` set) are measured separately per
design.md -> Decision 6 (amended by the planning review's CRITICAL finding) because one shared
`SCAN_MIN` either passes vacuously for the smaller sets or fails legitimately for the larger
one. `Refresh` (51) and `src/launch.rs`'s `Outcome` (23) both measure below the script's
default of 60 at BASE. The `Outcome` set is re-measured at group 12 (after group 6 widens
`Outcome::problem` to `Outcome::problems: Vec<String>`, which changes its construction-site
literal count) rather than frozen at the BASE figure, since the floor must reflect the tree
`make gates` actually runs against.

`READONLY-UI`'s `EXTRA` default becomes the five-file list above once extracted (task 12.4) —
the script does not report a numeric count for `EXTRA`; it is the fixed set itself that is
recorded as the new default, each existence-checked by the script's own guard.

## Final floors, re-measured at group 12 extraction (after groups 1–11's own new tests)

Every gate below is now a repository file under `scripts/gates/`, invoked bare in the
`Makefile`'s `gates:` recipe (task 12.7) except `LAUNCHSEAM`'s two subjects and
`NODEFAULT-UI`'s five type sets, each of which carries its own `SCAN_MIN` on the recipe
line per design.md -> Decision 6.

| Gate | Floor variable | Re-measured | Delta from BASE |
|---|---|---|---|
| `NOSPAWN-GREP` | `MIN` | 24 | unchanged |
| `NOLIT-CHANGE` | `MIN` | 24 | unchanged |
| `MDSEAM` | `MIN` | 24 | unchanged |
| `WATCHSEAM` | `MIN` | 29 | +1 (`tests/degraded_coverage.rs`) |
| `AGENTSEAM` | `MIN`, `ALLOWED` | 25, 5 files | +1 file searched; `ALLOWED` now the 5-file list by default |
| `LAUNCHSEAM` (both subjects) | `MIN`, `ALLOWED` | 25, 5 files | same |
| `NOCLI-SHELL` | `UI_MIN` | 11 | unchanged |
| `READSEAM` | `UI_MIN` | 10 | unchanged |
| `NOBLOCK` | `UI_MIN` | 11 | unchanged |
| `READONLY-UI` | `UI_MIN`, `EXTRA` | 11, the 5-file list | `EXTRA` now defaults to the list, not empty |
| `NORAW-GREP` | hardcoded (fixed, not exposed) | 29 | was hardcoded at 16 (stale since list-view); fixed to 29 |
| `NOSLEEP` | `MIN`, `SLEEP_MIN` | 29, 6 | `MIN` +1 |
| `NODEFAULT-UI` (Dashboard/Filter/Detail) | `SCAN_MIN` | 135 | +9 (`file_mode` field added at every site) |
| `NODEFAULT-UI` (`Refresh`) | `SCAN_MIN` | 53 | +2 |
| `NODEFAULT-UI` (`Launch`, `ui::app::Launch`) | `SCAN_MIN` | 81 | +3 |
| `NODEFAULT-UI` (`src/agents.rs` set) | `SCAN_MIN` | 111 | +8 — **this set was never actually recorded as one of the "four" in the BASE table above; found and closed here.** |
| `NODEFAULT-UI` (`src/launch.rs` `Outcome`) | `SCAN_MIN` | 26 | +3 — re-measured post-group-6's `problem` → `problems: Vec<String>` widening, exactly as flagged |
| `WIDTHS` | `WIDTHS_MIN` | 98 | +4 new view tests (group 4) |
| `LISTWIDTHS` | `LIST_MIN` | 32 | +3 new tests (group 8) |
| `MDWIDTHS` | `MD_MIN` | 25 | +1 new test (group 9) |
| `TASKWIDTHS` | `TASK_MIN` | 16 | unchanged (no group touched `src/ui/tasks.rs`) |
| `DETAILWIDTHS` | `DETAIL_MIN` | 32 | +9 new tests (groups 5, 7, 9) |
| `GATE-MECH1` | none | 25 files, 87 constructions | +3 constructions |

**Finding on `NODEFAULT-UI`'s `src/agents.rs` set:** design.md -> Context and task 0.5 both
describe "four type sets", but `notes/gate-floors.md`'s own BASE table (above) recorded only
three (`Dashboard`/`Filter`/`Detail`, `Refresh`, `Launch`/`ui::app::Launch`) plus
`src/launch.rs`'s `Outcome` — omitting the `src/agents.rs` set entirely from the recorded
BASE measurement, even though task 0.5's own text names it. Re-measured here at 111 and
added as its own `Makefile` recipe line (five `NODEFAULT-UI` lines total, not four), so the
gap does not silently persist into the extracted gate set.

`AGENTSEAM`/`LAUNCHSEAM` verified they do **not** regress from `spec-purposes`' original
"5 instances of a gate run without its floor" defect: run bare (no `ALLOWED` override) both
failed with `src/open.rs` and (for `LAUNCHSEAM`) `src/launch.rs` reported as reaching the
Herdr handle "outside" the allowed set — exactly the class of failure this whole change
exists to close, confirmed pre-existing (not a regression from groups 1–11's own work) and
closed by task 12.4's `ALLOWED` default.

Two incidental gate violations found and fixed during extraction, both self-inflicted by
this change's own new test/doc-comment text rather than by groups 1–11's actual production
changes: `NOCLI-SHELL` tripped on a doc comment naming `CliChanges` in a `mod.rs` test
(reworded); `READSEAM` tripped on a `src/ui/detail.rs` test that read an artifact file back
through `ui::read_artifact` to satisfy `NOIO-VIEW` — `READSEAM` confines that binding to
`src/ui/mod.rs` alone, so the test was rewritten to spell the known file contents directly
rather than reading them back at all.

The three dependency-gate clauses `spec-purposes` parked (task 12.6): `notify`'s
`default-features = false` was **already closed** — `scripts/gates/deps.sh`'s leg 2a's
`want` dict already includes `"notify":["macos_fsevent"]` and its generic
`uses_default_features is False` assertion already covers it (closed by `spec-purposes`
itself when it added `notify` to leg 2a for the DEPS repair; the roadmap's "parked" list was
stale on this one clause). `kqueue`/`kqueue-sys` and `notify-debouncer-*` were genuinely
open and are now closed: `scripts/gates/build-graph.sh` gained both as named absences,
verified absent from the real resolved graph and verified the check fires on planted data
(a synthetic snapshot line for each — see `notes/planted-defects.md`).
