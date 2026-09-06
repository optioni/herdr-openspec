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
