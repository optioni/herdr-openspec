# Gate floors — measured at group 7, on the tree groups 1–6 produced

One floor moves in this change: `NODEFAULT-UI`'s first type set gains `Sections`, the
plain-data collapse-state struct `list-sections` adds to `src/ui/app.rs`. Every other leg's
floor is untouched, and none was re-measured, because no other type set's construction sites
changed.

`NODEFAULT-UI` is the one **multi-subject** gate in `make gates`: it scans five distinct type
sets, and one shared `SCAN_MIN` would either pass vacuously for the smallest set or fail
legitimately for the largest. Its five floors are therefore the only ones that live on the
`Makefile` recipe line rather than in the script's own default, exactly as
`degraded-states/notes/gate-floors.md` recorded when it extracted them.

## The measurement

All commands run from the repository root, at commit `7c406c8` (group 6's last).

| | Command | Result |
|---|---|---|
| Before, without `Sections` | `SCAN_MIN=135 /bin/sh scripts/gates/nodefault-ui.sh` | `150 literal/pattern spans scanned (>= 135)`, exit 0 |
| After, with `Sections` | `SCAN_MIN=135 TYPES='Dashboard Filter Detail Sections' /bin/sh scripts/gates/nodefault-ui.sh` | `206 literal/pattern spans scanned (>= 135)`, exit 0 |

**Delta: +56 spans**, every one a `Sections { … }` construction site. The count without
`Sections` is unchanged from the 150 `tasks.md` check B recorded at planning time, which is the
expected result: adding a field to `Dashboard` does not add a construction *span*, and this
change added no new `Dashboard`, `Filter`, or `Detail` construction site of its own.

## The floor chosen

`SCAN_MIN=206` — the exact re-measured count, following `degraded-states`' own rule that a
floor is set at the figure measured on the tree `make gates` actually runs against, not at a
rounded-down margin. A floor below the measurement would let a future change delete
construction sites without the gate noticing, which is the drift the floor exists to catch.

    SCAN_MIN=206 TYPES='Dashboard Filter Detail Sections' /bin/sh scripts/gates/nodefault-ui.sh

## Negative control (task 7.4)

`SCAN_MIN=207` on the same line, so that the floor exceeds what the tree can produce:

    NODEFAULT-UI FAIL: half B found only 206 literal/pattern spans (expected >= 207) - the scan is vacuous

exit 1, as required. Restoring `206` returns the gate to `OK` and `make gates` to silence. The
control proves the floor is load-bearing rather than decorative: the gate can still fail.
