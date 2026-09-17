# `NODEFAULT-UI`'s eighth scanned set: `launch::Settings`

`AGENTS.md` records the standing rule: **a gate's floor is its own script default**, and
every recipe line runs the script bare — with one stated exception, the multi-subject
`NODEFAULT-UI`, whose seven (now eight) lines each carry an explicit `SCAN_MIN` because
one shared default would pass vacuously for the smallest set and fail legitimately for the
largest. `degraded-states` established both the exception and the practice of recording
each floor's measurement.

## Why `Settings` joins the gate at all

Unlike `config.prompts`' nested `BTreeMap` — which needs no scanned set, having no field
whose default could be silently wrong — `Settings` is constructed at exactly one site and
carries a `repo: PathBuf` and an `openspec_bin: Option<PathBuf>` whose defaults **would**
be wrong there: a defaulted `repo` is `""` and a defaulted `openspec_bin` is the file-mode
value. That is precisely the failure this gate exists to catch (design.md -> Decisions 10).

## The measurement

Run against the tree at the end of group 10, with `HOMEFILE=src/launch.rs TYPES='Settings'`:

```
$ SCAN_MIN=1 HOMEFILE=src/launch.rs TYPES='Settings' /bin/sh scripts/gates/nodefault-ui.sh
NODEFAULT-UI OK (half B): 4 literal/pattern spans scanned (>= 1), none elides a field
NODEFAULT-UI OK: no Default for [Settings], no elided field; positive controls matched

$ SCAN_MIN=4 ...   -> OK: 4 spans scanned (>= 4)
$ SCAN_MIN=5 ...   -> FAIL: half B found only 4 literal/pattern spans (expected >= 5)
                      - the scan is vacuous     (exit 1)
```

**The true measured floor is 4**, and `5` is its negative control: the floor is pinned at
the value the tree actually produces, not one below it, so a future change that deletes a
construction site makes the gate go vacuous loudly rather than passing on three.

The four spans are the type's own declaration and the three construction sites: the
production root in `src/ui/mod.rs`, and the two test fixtures (`settings_for_test` in
`launch::tests::seam`, `settings` in `launch::tests::resolution`).

## Recipe line

```make
SCAN_MIN=4 HOMEFILE=src/launch.rs TYPES='Settings' /bin/sh scripts/gates/nodefault-ui.sh
```

Added as the **eighth** `nodefault-ui.sh` line (`grep -c 'nodefault-ui.sh' Makefile` read
**7** before this change and reads **8** after it).
