# Change Review — heading-sections

Dispatched at task 8.1 to an `outside-in-tdd-reviewer` subagent with fresh context, given
only the planning artifacts and the diff `857163a..881f6bd`. Not a fork of the implementing
session, and not told which arguments had already been had.

## Verdict

Ready to archive once the CRITICAL below was fixed. It was, in `a1c412e`.

## Findings and triage

| Severity | Finding | Disposition |
|---|---|---|
| **CRITICAL** | A file passing the split gate but yielding exactly one section — one resolved path, empty preamble, one heading — had its heading consumed into `ArtifactSection::label`. `Detail::foldable()` is then `false`, so `content_lines` drew the section *text* with no header row, and **the heading vanished from the screen**. Probed: a one-group `tasks.md` drew `["[-]", "", "[✓] 1.1 first", "[ ] 1.2 second"]` with no `1. Setup` row, and a single-`### Requirement:` spec drew only `["Alpha text."]`. Against three normative sentences of this change's own deltas, most plainly `tasks-checklist`' "never both, and **never neither**". | **Fixed** in 8.2, `sync_detail` only. Re-probed after: `["[-]", "", "## 1. Setup", …]` and `["### Requirement: Alpha", "", "Alpha text."]`, with a two-group file still folding. |
| **WARNING** | `content_lines`' doc comment (`src/ui/detail.rs:391–401`) still states the pre-reversal rule and cites Decision 8 as live, twelve lines above a `foldable()` dispatch that does the opposite. Named by no task: 9.2–9.5 cover `openspec/specs/`, `AGENTS.md` and `SPEC.md`, but no source doc comment. | **Routed to group 9** as 9.7, not accepted. |
| **WARNING** | `preamble_len`'s doc block (`src/ui/app.rs:321–348`) runs into `seed_expanded`'s with no break, so the compiler binds the whole run to `seed_expanded` and `preamble_len` is undocumented. The casualty is the backwards-walk argument, the non-obvious part of the derivation. | **Routed to group 9** as 9.8. |
| **SUGGESTION** | `SPEC.md:96` still calls the stored state "the detail region's per-file sections and its fold set". A second site made stale by this change, covered by none of group 9's edits. | **Folded into 9.4**, which now names two sites. |
| **SUGGESTION** | The two-path tracked-tasks derivation and `seed_expanded`'s subtree walk at non-zero `base` were asserted by transcription in `ui::detail` and executed by nothing — every `sync_detail` tracked-tasks fixture resolved to one path. | **Promoted to task 8.4**, not merely noted. See below. |

## Why the second suggestion was promoted rather than noted

The reviewer re-derived the transcribed fixture by hand and confirmed it correct, so this was a
gap and not a defect, and 8.2's letter only requires noting a SUGGESTION. It was promoted anyway
because it is **the same species of gap the CRITICAL survived on**: a production path no fixture
could reach. GAP-07 made one fixture headingless and GAP-09 moved another from one group to two;
each repair was right for its own scenario, and between them they removed the last fixture that
could reach the one-section path. Having just paid for that once, leaving its twin in place was
not defensible. `a_two_path_tracked_tasks_artifact_nests_its_groups_and_seeds_them_all` now
executes it, proved falsifiable against two planted defects.

## Checked and clean

The reviewer found no departure from D1–D11, no architecture-rule violation in substance rather
than merely as the greps see it (purity, the spawn seam, the confined artifact read,
`pulldown_cmark` and `Color` confinement, display-column width, a render path that gained no wait
and no clock), and no tautological assertion among group 7's four tests — whose falsifiability it
re-checked plant by plant. It independently confirmed six of the eight implementation-time spec
repairs as straightforwardly right, including that GAP-13 resolved in the correct direction and
that production was not bent to the prose.
