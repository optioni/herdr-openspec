# Baseline

**BASE SHA:** `31fc7f59cf5357c4ba8ee7c0ab6ba5957be8ff36`

Captured once at task 0.1, before any implementation edit in this apply session. The plan's
own design/tasks docs were written against `89cb3b2`; `31fc7f59cf5357c4ba8ee7c0ab6ba5957be8ff36` is the commit that added those
planning docs themselves (`docs(degraded-states): plan the final change`), so it is the
correct base for this change's own diff. Every later `OPENSPEC-UNTOUCHED` run reads this
recorded value and first confirms it still resolves (`git cat-file -e $BASE^{commit}`) rather
than re-deriving `BASE` as the current `HEAD`.
