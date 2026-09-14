## Reviewed Artifacts

- `proposal.md`
- `specs/quality-gates/spec.md`
- `design.md`
- `tasks.md`

## Reviewed Against

- This repository HEAD: `9c37085`
- Sibling repository HEAD: Not applicable
- Working tree: clean apart from this change's own directory, which is untracked

Two reviewers, neither of which wrote the package, each given one merged pair of slices:
**A+C** (capability coverage, scenario quality, cross-artifact contradictions; task alignment
and lifecycle discipline) and **B+D** (design completeness, test boundaries, whether each
proposed check could fail at all; factual verification). The schema slices four ways; two was
proportionate to a package of two requirements and five task groups, and the merge kept each
reviewer's scope small without four reports rediscovering each other. Slice D was kept intact
deliberately — a change about a wrong number cannot afford a wrong number in its own plan, and
that is exactly what it found.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | `specs/quality-gates`, `proposal.md` | **Found independently by both reviewers, from different slices.** The change committed the error it exists to eliminate. The extracted-gate figure was moved from twenty-six to twenty-nine by **arithmetic** (31 − 2) without re-counting the enumeration of gate names that the figure summarises, while `proposal.md:21-22` claimed it was "re-measured rather than adjusted by arithmetic". The list is short by three: mapping every enumerated name onto `scripts/gates/` leaves `colwidth.sh`, `palette.sh` and `helpwidths.sh` named by no entry. So the sentence enumerated twenty-five items, called them twenty-nine, and derived thirty-one from them — a chain visibly broken in the one requirement a future gate-extractor is told to consult, and one the change's own new assertion cannot catch, because nothing binds the list. Slice D dated each omission: `ae0bd9d` (`degraded-states`) left 28, then `c4d497d` added `colwidth.sh`, `5b77782` `palette.sh`, and `dc02ec3` `helpwidths.sh` — one per change, never propagated. | `twenty-five` → `twenty-eight`; the three names added; chain re-derived from the list and confirmed to close: 28 + `OPENSPEC-UNTOUCHED` = 29, + `deps.sh` + `build-graph.sh` = 31 = `ls scripts/gates/ \| wc -l`. Re-verified mechanically by slice A after the repair, matching gate names **backtick-delimited** rather than by substring, since `WIDTHS` is a substring of five other gate names and a naive test would report `widths.sh` covered whatever the list said. `design.md` → Decisions 2 records the general rule. | `specs/quality-gates` (enumeration), `proposal.md` → Why, What Changes; `design.md` → Decisions |
| CRITICAL | `proposal.md`, `specs/quality-gates` | The proposal's central claim — "the count is the last figure in the quality-gates tier not bound to the thing it describes" — was false. `NODEFAULT-UI` is documented as scanning "**five** type sets" with "**five** `SCAN_MIN` values"; `grep -c SCAN_MIN Makefile` returns **7**. `foldable-spec-sections` added the sixth (`b52767d`) and `help-overlay` the seventh (`dc02ec3`), neither propagating to the prose — and `tests/gate-controls.toml:425`, written during `help-overlay`, already names "the Makefile's **seventh** line", so two checked-in files have disagreed since the previous change and nothing notices (`grep -rn five tests/*.rs \| grep -i 'nodefault\|scan_min'` returns nothing). Both false sentences sit **inside requirement blocks this delta reproduces**. | Corrected to **seven** at both delta sites and at `AGENTS.md:266,270`, which carry the same figure. The deciding argument is the reviewer's: a MODIFIED block **re-lands its content as current** at archive time, so passing a known-false figure through one converts a stale sentence into a freshly asserted one, and the "not re-auditing every numeral" Non-Goal cannot shield that. The figure is corrected but **not bound** — binding needs a second mechanism against a second subject (the recipe's `SCAN_MIN` lines, not a directory listing) and would make this two changes; that is now `design.md` → Decisions 1 and an explicit Non-Goal. | `specs/quality-gates` (two sites), `proposal.md` → Why, What Changes, Non-Goals, Impact; `design.md` → Decisions; tasks 2.4, 3.1 |
| WARNING | `specs/quality-gates`, `design.md` | The new scenario claimed the test asserts the directory count equals "the figure this requirement's first scenario states". A Rust literal reads no figure. The surviving failure mode the wording hid: a developer adds a gate, sees red, bumps the literal to 32, and leaves the spec at thirty-one — today's drift minus one red run. `design.md` justified the literal as avoiding a parser, which answered an argument nobody made: `tests/ci_workflow.rs` already carries `read_spec_md()` and `spec_md_gates_section_names_every_check_prerequisite_by_name` already asserts document content, so `spec.contains("— **thirty-one** in all")` was available and is no parser. | The literal stands, for the reason the reviewer identified as unaddressed rather than the one given. Verified: `openspec/specs/` is written **only** by `openspec archive` — every edit to it across the whole of `help-overlay` lands in one commit, `9c37085`, and the live spec still reads "twenty-eight in all". A content assertion would go red the moment it landed and stay red for the entire apply phase, with `make check` gating every commit between. The scenario now states what the test does, says the directory is bound by machine and the sentence by the **failure message**, and requires that message to name the file, the scenario and both counts — since it is the only thing standing against the surviving failure mode. | `specs/quality-gates` (new scenario), `design.md` → Test Boundaries |
| WARNING | `tasks.md` | Task 2.2 read "Apply the delta in `specs/quality-gates/spec.md`", which reads as an instruction to edit the **live** spec during apply. Doing so would put the two copies in disagreement in the opposite direction and pre-empt the archive. | Rewritten to name the delta path explicitly and forbid editing `openspec/specs/quality-gates/spec.md`, with the archive-boundary reason attached. Line references are marked as pointing into the live spec, which is where the figures are read from. | tasks 2.2 |

## Independently Verified at HEAD

Run by this session while merging, not reported by a reviewer:

- **MODIFIED-block completeness, re-run after the repairs.** The reviewers' diff was against the
  pre-repair delta and its ranges had shifted, so the guarantee was re-established against what
  will actually archive: both blocks extracted by their own `### Requirement:` boundaries and
  `diff -u`'d against the merge target. Requirement 1 differs only by additions and numeral
  edits, with the new scenario appended after the last existing one and all four originals
  present in order; requirement 2's scenario headings are byte-identical and its only residual
  is a trailing blank line at the file boundary. **No deletion hunk in either.** This is the one
  failure that cannot be detected after archive, which is why it was re-run rather than assumed.
- `scripts/gates/` holds **31** files, 30 `.sh` and one `.py`; it held **30** before
  `help-overlay` (`git show 0745e70^:scripts/gates`).
- Exactly **two** gate scripts invoke `cargo`. Five name it; `colwidth.sh`, `noio-view.sh` and
  `wired.sh` name it only inside comments — two of those comments written by this session during
  `help-overlay` — so the non-`cargo` figure is 29 of 31, measured rather than derived.
- Both negative controls recorded in task 1.1, run at HEAD: planting a 32nd script with its
  recipe line leaves `cargo test --test ci_workflow` at `21 passed; 0 failed`, and so does
  deleting one with its line. The existing `on_disk.len() >= 25` floor fires in neither
  direction. Tree confirmed clean after both.

## No Remaining Implementation-Blocking Gaps

None. Both CRITICALs are repaired and re-verified; both WARNINGs are resolved in the artifact
that owns them.

One judgement is recorded rather than resolved, and is not blocking: the count is bound to the
test by machine and to the spec sentence only by the failure message. The archive boundary makes
a stronger binding impossible without a check that is red for a whole apply phase. If a
mechanism exists that survives that boundary, it belongs to a later change and should cite
`design.md` → Test Boundaries.

## Deferred Non-Blocking Notes

- **`NODEFAULT-UI`'s subject-set count is corrected but unbound.** It will drift again the next
  time a type set is added. The resolution point is recorded: whichever change next touches that
  gate, with the argument already in `design.md` → Decisions 1 and half-stated by
  `tests/gate-controls.toml:425`.
- **`quality-gates` numerals outside the two modified requirements were not audited.** The
  Non-Goal is scoped to figures that are both false today and inside a requirement this change
  already modifies. Slice A checked for others and found none in the modified blocks; the rest of
  the capability is untested ground and stays so deliberately.
