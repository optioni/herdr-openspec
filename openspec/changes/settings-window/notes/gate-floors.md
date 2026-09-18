# `NODEFAULT-UI`'s ninth scanned set: `settings::{Setting, KindResolution, PanelState}`

`AGENTS.md` records the standing rule: **a gate's floor is its own script default**, and
every recipe line runs the script bare — with one stated exception, the multi-subject
`NODEFAULT-UI`, whose eight (now nine) lines each carry an explicit `SCAN_MIN` because one
shared default would pass vacuously for the smallest set and fail legitimately for the
largest. `degraded-states` established both the exception and the practice of recording each
floor's measurement; `agent-client-choice` did the same for its own (eighth) addition.

## Why this set joins the gate at all

`Setting`, `KindResolution`, and `PanelState` are all **structs** declared in `src/settings.rs`
(verified: `grep -n '^pub struct' src/settings.rs` names all three). The gate's positive
control is `grep -qE "struct[[:space:]]+$T[[:space:]]*\{"`, so only a struct can join —
`Provenance`, `Editable`, `Reason` (enums in the same file) and `Panel` (an enum in
`src/ui/app.rs`) cannot, and stay covered by exhaustive `match` instead, on the same terms the
task line states.

## The measurement

Run against the tree at the end of group 10, with
`HOMEFILE=src/settings.rs TYPES='Setting KindResolution PanelState'`:

```
$ SCAN_MIN=1 HOMEFILE=src/settings.rs TYPES='Setting KindResolution PanelState' /bin/sh scripts/gates/nodefault-ui.sh
NODEFAULT-UI OK (half B): 105 literal/pattern spans scanned (>= 1), none elides a field
NODEFAULT-UI OK: no Default for [Setting KindResolution PanelState], no elided field; positive controls matched

$ SCAN_MIN=105 ...   -> OK: 105 spans scanned (>= 105)
$ SCAN_MIN=106 ...   -> FAIL: half B found only 105 literal/pattern spans (expected >= 106)
                        - the scan is vacuous     (exit 1)
```

**The true measured floor is 105**, and `106` is its negative control: the floor is pinned at
the value the tree actually produces, not one below it, so a future change that deletes a
construction site makes the gate go vacuous loudly rather than passing silently.

## Recipe line

```make
SCAN_MIN=105 HOMEFILE=src/settings.rs TYPES='Setting KindResolution PanelState' /bin/sh scripts/gates/nodefault-ui.sh
```

Added as the **ninth** `nodefault-ui.sh` line (`grep -c 'nodefault-ui.sh' Makefile` read **8**
before this group and reads **9** after it).

## Finding: the first `NODEFAULT-UI` line's `SCAN_MIN` was stale, not 350

Task 10.5 only asked to verify the existing first `NODEFAULT-UI` line
(`TYPES='Dashboard Filter Detail Sections Overlay Selection Edit'`, `Makefile:46`) is still
green after group 1's `Help` -> `Overlay` rename — the dispatch note (B1) stated its `SCAN_MIN`
was re-measured to 350 by group 4 and asked this group to *verify* that figure rather than
re-derive it from scratch.

Re-measuring it here at HEAD (`ca04eaa`, after group 9's own work) found the **true** count is
**367**, not 350:

```
$ SCAN_MIN=350 TYPES='Dashboard Filter Detail Sections Overlay Selection Edit' /bin/sh scripts/gates/nodefault-ui.sh
NODEFAULT-UI OK (half B): 367 literal/pattern spans scanned (>= 350), none elides a field
NODEFAULT-UI OK: ...

$ SCAN_MIN=367 ...   -> OK: 367 spans scanned (>= 367)
$ SCAN_MIN=368 ...   -> FAIL: half B found only 367 literal/pattern spans (expected >= 368)
                        - the scan is vacuous     (exit 1)
```

So the gate was still green at 350 (367 >= 350), but the floor itself had drifted 17 spans
below the tree's true count some time between group 4's measurement and group 9's own new
tests — a violation of "a gate's floor is its own script default: the true measured floor,"
left un-noticed because a floor below the true count still passes. Since this group is the
one instructed to re-measure this line rather than merely restate it, `Makefile`'s first
`NODEFAULT-UI` line is corrected here: `SCAN_MIN=350` -> `SCAN_MIN=367`. The `TYPES` list
itself is unchanged from B1's description (`Dashboard Filter Detail Sections Overlay Selection
Edit`), confirming task 10.5's rename requirement was indeed already satisfied by group 1.
