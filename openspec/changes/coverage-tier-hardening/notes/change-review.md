# Change review — coverage-tier-hardening

One independent `outside-in-tdd-reviewer`, not a fork of the applying session, against
`proposal.md`, the three delta specs, `design.md`, and `git diff fbfae77..HEAD`. Pointed
specifically at the rebound ranges, on the grounds that this change's whole subject is
bindings that pass while proving nothing — so a rebinding that is merely *plausible* is the
same defect in a new place. It edited nothing. Every finding below was reproduced in this
session before it was acted on; none was taken on report.

**Outcome: 0 CRITICAL, 4 WARNING, 4 SUGGESTION. All eight fixed.** The review found one
defect inside this change's own gate that would have shipped a rule three of whose clauses
nothing checked.

## The finding that mattered

**Three branches of the new rule were load-bearing for nothing (WARNING 3).** Deleting
`is_attribute`, `is_lone_delimiter`, or the multi-line-signature branch each left the binary
**green** — and each deletion *widens* what the rule accepts, which is the failure mode this
change exists to prevent. Reproduced by ablation before acting: 16 passed, three times over.

The multi-line-signature gap was the sharpest, because "a range that is only a signature" is
this change's headline shape and the only signature fixture it shipped —
`src/tasks.rs:193-196` — is a **single-line** one that `opens_an_item` alone already catches.

The repair is `each_declaration_branch_has_a_range_only_it_rejects`, one control range per
clause: `src/changes.rs:1981-1982` (two attribute lines), `src/ui/view.rs:107-109` (three
closing braces), `src/ui/detail.rs:45-48` (`header_row`'s parameter list). All **six**
branches of `declaration_lines` were then re-ablated, including the latch repair below —
every one now turns the suite red. `specs/degraded-coverage/spec.md`'s failure condition 4c
now requires a per-clause control, so a clause added later without one fails review against a
written rule rather than a reviewer's memory.

## Two rebindings that were plausible and wrong

Exactly what the reviewer was pointed at, and the reason it was pointed there.

- **"No active changes" (WARNING 1).** The map bound only `src/ui/list.rs:536-542`, the
  `No changes yet` branch the named proof drives — and that proof *positively asserts the
  row's own message is absent* (`src/ui/view.rs:3139`). The row's SPEC behaviour is "Empty
  state; archived changes remain browsable", implemented at `525-535`. Now binds both.
- **"A marked tab's artifact resolves to no file" (WARNING 2).** The map bound
  `src/ui/detail.rs:674-679`, `tracked_tasks_progress` — whose value is read only inside
  `if detail.foldable()` (`:689`), and on this row's own path `detail.sections` is empty, so
  it is computed and discarded. Now `src/ui/detail.rs:64-70`, `header_row`'s
  `progress.total > 0` branch, which builds the counted pair the row's `why` names.

## The tallies were wrong (WARNING 4)

Recomputed here against `git show fbfae77:tests/degraded-coverage.toml`: **24 rows changed,
28 ranges replaced by 33, 74 -> 79**. The applying session had written "20 rows" and a class
split of 1 shape / 11 aboutness / 16 drift that reconciled with nothing. By row the split is
aboutness 12, drift 7, opposite branch 3, shape 1, pattern field 1; by replaced range,
13 / 9 / 4 / 1 / 1.

So **aboutness is the largest class, not drift** — and `proposal.md` and `design.md` had both
built "drift is the largest class, and it was not on anyone's list" on the wrong half of that
sentence. The half that survives is the second: drift was on nobody's list. Both documents
now say that and only that.

## Suggestions, all acted on

- **The spec's keyword list omitted `type`**, which `ITEM_KEYWORDS` includes. The spec is the
  contract and the code was one keyword wider than it; the spec now lists `type`.
- **`in_signature` latched.** A lone delimiter reaches the first branch and never the clearing
  arm, so an item whose signature closes on a bracket — `const STATUSES: [AgentStatus; 5] = [`
  — stayed latched through everything that followed. Five such runs under `src/`, about a
  hundred lines, the longest `src/ui/palette.rs:400-435`, where `for expect in table() {`
  classified as a declaration. The direction of error is conservative (a latched run only
  *adds* declarations, so it could reject a range and never accept one), which puts it inside
  `design.md` -> Decision 2 rather than against it — but `covers-check`'s answer to a false
  rejection is to widen the range, which would have written the quirk into the map. Repaired,
  with `a_lone_delimiter_ends_a_bracketed_item_signature` pinning both halves: the latch
  clears, and a `fn` signature — which closes on a brace, never on `;` or `]` — is still
  recognised.
- **Five prose sites restated the live range count** and every one went stale when the review
  added two ranges. Swept to 79. The reviewer's own observation is the durable part and is now
  in `design.md` -> Risks: this is the same drift class the audit found in `SPEC.md` ->
  Gates' "five", reproduced inside the change that was correcting it. Left unbound
  deliberately — a floor's slack is the point, and an equality would defeat it.
- **The range-floor test's assertion was satisfiable by an unrelated error**
  (`err.contains("covers") && err.contains("70")` — `"covers"` appears in several other
  messages and `"70"` is a short numeric substring). Now asserted against the message's own
  distinguishing phrase and the floor it names.

## Nothing blocking or unowned remains

Every WARNING is repaired and every SUGGESTION acted on. The four items the reviewer was told
were known and accepted — the manual `make check` ordering verification, the destructuring
pattern the rule does not reach, drift going uncaught, and the non-discriminating ranges in
`notes/audit.md` -> Reported and left — it correctly did not re-report.
