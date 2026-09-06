# Baseline

**BASE SHA:** `31fc7f59cf5357c4ba8ee7c0ab6ba5957be8ff36`

Captured once at task 0.1, before any implementation edit in this apply session. The plan's
own design/tasks docs were written against `89cb3b2`; `31fc7f59cf5357c4ba8ee7c0ab6ba5957be8ff36` is the commit that added those
planning docs themselves (`docs(degraded-states): plan the final change`), so it is the
correct base for this change's own diff. Every later `OPENSPEC-UNTOUCHED` run reads this
recorded value and first confirms it still resolves (`git cat-file -e $BASE^{commit}`) rather
than re-deriving `BASE` as the current `HEAD`.

## 0.2 — coverage baseline

`cargo llvm-cov --fail-under-lines 80 2>&1 | tail -3`:

```
TOTAL                            41572              1273    96.94%        2116                79    96.27%       25674               757    97.05%           0                 0         -
```

**97.05% of 25,674 lines.** The floor is never lowered and no exclusion is ever added.

## 0.3 — `WIRED` reproduced red

`sh $C/WIRED.sh` (`$C` = `openspec/changes/archive/2026-09-06-spec-purposes/notes/extracted-gates`):

```
WIRED FAIL (leg 2): 'pub fn run()' holds a branch or a loop:
5:    let cwd = match startup_cwd(&crate::config::env_lookup()) {
everything with a decision in it belongs in run_wired, which is tested
```

Exit 1, reproduced exactly as design.md -> Context records it.
