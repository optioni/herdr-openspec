## Context

`ui::detail::content_lines` draws a section header at `"  ".repeat(depth)` (`src/ui/detail.rs`
→ `header`, reached at the section walk) and then draws that section's body at the full
`width`, starting at column zero. The body therefore hangs left of its own header, and left of
every ancestor header above it. Measured over this repository's 231 delta spec files: 3,757
section headers, of which **2,812 are at depth 3** — six columns of misalignment is the common
case, not the corner.

`artifact-folds` states the unindented body deliberately, protecting the 58-column narrow
interior's text column. This change keeps that protection and narrows its scope to the width
that actually needs it.

## Goals / Non-Goals

**Goals.** A body sits flush beneath its own header at the wide layout. The narrow layout keeps
every column it has today. One decision per render, so the tab's left edge is never half
aligned.

**Non-Goals.** No header row moves. No fold level, keybinding, cursor behaviour, or
`section_at` lookup changes. No change to markdown wrapping itself. The tracked-tasks tab is
out of scope by construction — its sections are at depth 0.

## Decisions

**1. Indent the body, wrap it narrower — not prefix a full-width row.** An indented body row is
`indent` spaces followed by what `ui::markdown::lines` produced at `width - indent_cols`.
Prefixing a row already wrapped at `width` would push every long line `indent_cols` past the
interior, which the region does not clip and `responsive-layout` forbids. This is the same
shape the item grammar already uses for its hanging indent.

**2. One decision per render, from the tab's deepest body-bearing section.** The alternative —
deciding per section, so a depth-1 body indents where a depth-3 body cannot — produces a tab
whose left edge steps in and out with no rule a reader can infer. A single decision makes the
tab either consistently aligned or consistently flush, and both are legible. It also means the
rule is evaluated once, not once per row.

**3. The floor is `width - 2 * max_depth >= 64`, and 64 is a measured trade, not a derivation.**
Stated plainly because a constant that pretends to be principled is worse than one that shows
its working:

| Tab's max depth | Indent cost | Text column at 78 | Text column at 58 | Floor | Indents at |
|---|---|---|---|---|---|
| 1 | 2 | 76 | 56 | 66 | width ≥ 66 |
| 2 | 4 | 74 | 54 | 68 | width ≥ 68 |
| 3 (the archive's deepest) | 6 | **72** | **52** | 70 | width ≥ 70 |

64 is placed so that the archive's deepest spec — depth 3 — indents at the 78-column wide
interior (72 ≥ 64) and does not at the 58-column narrow one (52 < 64). That is precisely the
trade this change was asked to make, and the table is the record of how the constant was
chosen.

The floor reads the **tab's own** maximum depth rather than being a fixed width, so a shallow
tab indents at a narrower pane than a deep one. That falls out of the rule rather than being a
second mechanism, and the scenario "A shallower tab indents at a narrower width" is what keeps
it from silently collapsing into a constant.

**4. Depth 0 is unaffected, so the tasks tab cannot regress.** `base` is 0 for a single-path
artifact, so every tracked-tasks section is at depth 0 and its indent is zero columns whether
the floor is met or not. The scenario "A tracked-tasks tab is unmoved at every width" asserts
byte-identity rather than reasoning about it, because `task-item-bodies` is rewriting that same
grammar and a regression there would otherwise be attributed to the wrong change.

**5. The previous rule's rationale is kept in the spec, not deleted.** The sentence about
reducing the text column stays, reworded as the reason the floor exists. A future reader who
finds only the new rule would have no way to know the narrow layout was considered and
deliberately excluded; this repository's specs carry that kind of reasoning on purpose.

## Risks / Trade-offs

| Risk | Mitigation |
|---|---|
| The constant `64` drifts out of meaning as spec depth grows | The floor reads `max_depth` at render time, so a depth-4 tab gets a floor of 72 with no edit; and the derivation table above is in the spec, not only here |
| A width near the floor flickers between layouts as the pane is resized | Accepted: one transition width per tab, asserted as exactly one in "The indent is all-or-nothing across one render". A hysteresis band would need state in a pure view |
| `task-item-bodies` and this change both delta the same requirement | Sequenced explicitly in the proposal's Impact: this lands and archives first, then that change's delta is refreshed. Stated there rather than here because it is a scheduling fact, not a design one |

## Migration Plan

None. No persisted state, no config, no manifest, no keybinding.

## Modules touched

`src/ui/detail.rs` only — the section walk in `content_lines`, which already holds each
section's `depth` and the content width. The maximum depth is computed from
`detail.sections` in the same pass that already builds `visible_sections`.

**No process spawn is added** — `src/ui/detail.rs` names none and gains none. **No view gains
I/O**: `content_lines` stays a pure function of `Detail` and a width, and `NOIO-VIEW`'s
ten-file pure set is unchanged in membership. **The `Change` type does not move**, so
`changes::from_files` and `changes::from_cli` need no reconciliation.

## Test Boundaries

| Collaborator | Treatment | Why |
|---|---|---|
| Filesystem | **Absent** | `content_lines` takes a `Detail` built in the test; no scenario reads a file |
| `openspec` binary | **Not reached** | No CLI path is touched |
| Herdr socket | **Not reached** | No agent, launch, or pane surface is touched |
| Terminal | **Replaced** by `ratatui::backend::TestBackend` where a scenario renders a frame; the indent scenarios themselves assert `content_lines`' rows directly | `cargo test` spawns this binary, so no test may reach a real terminal |
| Clock | **Not reached** | No timing behaviour changes |

## Verification Matrix

Tier key: **U** unit over `ui::detail`'s pure rows, **V** view test into a `TestBackend`.
`DETAILWIDTHS` requires every `#[test]` in `src/ui/detail.rs` to name both `58` and `78`; the
two sides of the floor are those two widths, so every row below satisfies it without an
exemption.

| Scenario | Tier | Collaborators | Command |
|---|---|---|---|
| A spec tab's bodies align under their headers at the wide interior | U | none | `cargo test --lib ui::detail` |
| The same tab draws its bodies at column zero at the narrow interior | U | none | `cargo test --lib ui::detail` |
| The indent is all-or-nothing across one render | U | none | `cargo test --lib ui::detail` |
| A shallower tab indents at a narrower width | U | none | `cargo test --lib ui::detail` |
| A tracked-tasks tab is unmoved at every width | U | none | `cargo test --lib ui::detail` |
| The sixteen scenarios carried unchanged by the MODIFIED block | U + V | TestBackend | `cargo test --lib ui::detail ui::view` |

The sixteen carried scenarios keep the tests they already have and are re-run as regression:
every one of them draws body rows, and this change moves body rows.

## Gates

`make check` is the single gate.

- **`DETAILWIDTHS`** — satisfied by construction, as above.
- **`COLWIDTH`** — the indent is measured through `ui::layout::columns`, never a `.chars()`
  count. `"  ".repeat(depth)` is ASCII and measures its own character count, but the
  *remaining* width arithmetic is column arithmetic and is written as such.
- **`NOIO-VIEW`** — `src/ui/detail.rs` stays in the ten-file pure set with no new API named.

Coverage stays at the 80% line floor with no exclusion added.
