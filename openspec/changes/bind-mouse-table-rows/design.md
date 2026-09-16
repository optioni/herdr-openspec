## Context

`tests/doc_contract.rs` holds thirteen claims, each binding a documented statement to the
repository file that determines it. Twelve are strong. One — the mouse bindings — is a set
equality over backticked `Action::` names lifted out of two *texts*. Names matching on both
sides says nothing about whether any row describes what the pane does.

`mouse-text-selection` proved the gap costs real errors: four false statements in that one
table survived a green `make check`.

Most of the machinery to close it exists in the same file. `sweep_mouse_actions` executes
`mouse_action` at every cell of three frames across fourteen `MouseEventKind` values — then
reduces the result to a `BTreeSet<String>` of action names.

**A first draft of this design keyed the check on `(gesture, zone, action-name)` and was
wrong.** Planning review measured the resulting set: 104 distinct triples, 18 of them
non-`Ignore`. Under that key the row `mouse-text-selection` actually deleted claims
`(Down(Left), DetailRow, Click)` — a triple that **is** observed, produced by the
artifact-section-header row beside it — so the row is not vacuous and the check passes. Of
the four false statements, only one would have been caught. The claim below is built to the
measurement rather than to the intuition.

## Goals / Non-Goals

**Goals**

- A table row describing a binding the pane no longer has fails `cargo test`.
- A binding the pane has that no row describes fails `cargo test`.
- Both failures name both sides of the disagreement, as every other claim in this file does.

**Non-Goals**

- No behaviour change. `mouse_action`, `Action`, `Zone`, and every binding are untouched.
- Not the key table — already bound from three sides through `ui::help::INVENTORY`.
- No fourteenth claim. `CLAIM_COUNT` stays 13.
- The catch-all row's prose is not parsed, so two of the four motivating false statements stay
  out of reach. Stated, not glossed.
- No semantic parsing of English prose.

## Boundaries

Modules touched: **`tests/doc_contract.rs`** only, plus `SPEC.md` § Keys and
§ Testing and quality gates, and `AGENTS.md`, as prose. No file under `src/` changes.

**No process spawn is added anywhere.** `mouse_action` is a pure function of
`(&Dashboard, Rect, &MouseEvent)`.

**No view is added and no I/O is added to a view.** The change adds no code under `src/ui/`.
The only I/O is the existing `read_doc` of `SPEC.md` and `src/ui/driver.rs` inside a test.

The **`Change` type is not altered**, so keeping `from_files` and `from_cli` in agreement
does not arise.

| New piece | Existing pattern it follows |
|---|---|
| Claim-retaining sweep | `sweep_mouse_actions`' own three-frame × fourteen-kind loop, unchanged except in what it accumulates and in gaining a fixture axis |
| Row extraction | `documented_mouse_actions`' table-bounded scan and its `Err`-on-absence rule |
| Zone and payload tokens in a row | `backticked_action_variants`, already the file's way of lifting a typed name out of prose |
| Planted-defect controls | the file's **synthetic input** idiom — verified in review: `module_map_names`, `manifest_rust_version` and `check_programs` are each a pure `&str → Result` parser with one thin test feeding it the real file and several feeding it hand-written and malformed inputs. The file's own module doc states the rule |
| Pinned closed lists | `EXEMPT_ACTIONS` and the two-entry alias table, both asserted by length |

## Contracts

No interface a separate consumer depends on changes. `swept_mouse_action_names` keeps its
signature and its `BTreeSet<String>` return, now derived from the richer set. It has **three
direct callers**, measured (`grep -n "swept_mouse_action_names(" tests/doc_contract.rs`):

- `swept_action_names` (the helper at :2485-2486, through which
  `sweep_finds_the_bound_actions_and_exactly_two_exemptions` reaches it indirectly),
- `select_is_the_only_mouse_only_action_by_name_and_count` (:2836-2837),
- `the_sweep_covers_the_mouse_under_both_overlay_states` (:2897, :2918), which asserts the
  closed and open name sets **by equality** and is therefore the test most likely to go red if
  the projection is written wrong.

The change is **additive**. The plugin manifest, config format, and every keybinding are
unchanged, so nothing is **BREAKING**.

## Persistence and Rollout

- Migration: none. Backfill: none. Seeding: none. Index rebuild: none.
- Cache invalidation: the `OnceLock` caches inside `swept_mouse_action_names` are per-process
  test caches; they gain a dimension (fixture × overlay state) and keep working as they are.
- Authorization: none. Observability: none.
- Deployment impact: none. No shipped artifact changes; `make check` gains assertions and wall
  time (see Risks).

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| Filesystem (`SPEC.md`, `src/ui/driver.rs`) | real — `read_doc` of the checked-in files | replaced — synthetic `&str` documents passed to the extractors |
| `ui::driver::mouse_action` | real — executed at every cell of three frames, over every fixture and overlay state | real for the agree-at-HEAD and clamp rows; replaced by a hand-built claim set for the planted-defect rows |
| `Dashboard` fixtures | real values, explicitly listed and pinned by count | real values; planted rows use hand-built claims instead |
| `ui::help::INVENTORY` | real — a `'static` constant | real |
| Terminal | not touched — `mouse_action` renders nothing and enters no mode; no `TestBackend` and no `TerminalOps` double | not touched |
| `openspec` binary | not touched | not touched |
| Herdr socket / `HerdrCli` | not touched | not touched |
| Filesystem watcher, refresh worker, agent poller, launcher | not touched | not touched |

The comparison is factored into a **pure function over `(rows, claims)`** precisely so the
right-hand column is possible: a planted defect on either side is a hand-built argument, not a
doctored tree.

## Test Strategy

One tier matters here: the **contract tier** inside `cargo test` (`tests/doc_contract.rs`),
run by `cargo test --test doc_contract` and therefore by `make check`'s Test gate.

**This change takes no outer-loop acceptance test**, because its subject *is* a test. The
"acceptance" of a claim in this file is the claim running green against the real checked-in
tree — the agree-at-HEAD rows below. There is no user journey beneath it to drive through
`run_loop`.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| dc: A row describing a removed binding fails as vacuous | Comparator called with real claims and a row set carrying the planted `Target::DetailLine` row; asserts `Err` naming the row and the claims at its zone, and that leg 1 passes | contract | real sweep, synthetic rows | `cargo test --test doc_contract` |
| dc: Two rows that differ only in their payload are told apart | Comparator with the section-header row mis-naming `Target::Change`; asserts `Err`, and a companion assertion that the bare-variant key would have passed | contract | real sweep, synthetic rows | same |
| dc: A binding added with no row fails as undocumented | Comparator with a planted `(Down(Middle), closed, List, ToggleHelp)` claim; asserts `Err` naming all four axes | contract | synthetic claims, real rows | same |
| dc: The table and the pane agree at HEAD | The claim itself, all legs green; asserts 14 kinds and 6 zones plus the overlay axis appear, and that the new overlay-wheel row is covered | contract | real filesystem, real sweep | same |
| dc: The clamp is observed rather than reported vacuous | Sweep run with the selection-present fixture; asserts `(Drag(Left), closed, List, Select(Extend))` present, and absent with the selection-absent fixture alone | contract | real sweep, real fixtures | same |
| dc: An unrecognised gesture phrase is an error | Row extractor on a synthetic table with `Left quadruple-click`; asserts `Err` naming row and phrase | contract | synthetic document | same |
| dc: A mistyped zone token is named | Row extractor on a synthetic row naming `Zone::DetailRows`; asserts `Err` names the token, and is not the vacuity message | contract | synthetic document | same |
| dc: Zone-less rows are rejected, and a second catch-all is an error | Row extractor on a zone-less `SelectTab` row, then on a table with two zone-less `Ignore`-only rows; asserts `Err` both times, naming both rows in the second | contract | synthetic document | same |
| dc: A gutted table fails as a broken control | Row extractor on a header-plus-separator table and on one with the header deleted; asserts `Err` both times | contract | synthetic document | same |
| dc: The overlay axis keeps the two passes apart | Comparator with the dismissing-click row marked overlay-closed; asserts both a vacuous-row error and uncovered overlay claims | contract | real sweep, synthetic rows | same |
| dc: A third row joining a known collision fails | Comparator with a third row claiming `Click(Change)` at `ListRow`; asserts `Err` naming the pinned count | contract | real sweep, synthetic rows | same |
| dc: A binding added to the driver and not to the docs fails `make check` | Unchanged existing test | contract | real `action_for`, real `INVENTORY` | same |
| dc: The documented key set and the inventory agree at HEAD | Unchanged existing test | contract | real filesystem, real `INVENTORY` | same |
| dc: A gutted document fails as a broken control rather than a clean tree | Unchanged existing test | contract | synthetic document | same |
| dc: The key legs are unaffected by the mouse table's stronger binding | The three key legs re-run unchanged; asserts the `Mouse` group stays excluded | contract | real filesystem, real `INVENTORY` | same |
| mi: The documented bindings match the resolver | Covered by the agree-at-HEAD row above; no separate test | contract | real filesystem, real sweep | same |
| mi: The comparison is executed, not parsed | Same assertion as the vacuity row above, stated from `mouse-input`'s side | contract | real sweep, synthetic rows | same |
| mi: The documented confined set matches the gate | Unchanged existing test (`osc52`/`NORAW` claim) | contract | real filesystem | same |
| mi: The documented bypass names what was measured | Unchanged existing test (`the_documented_bypass_names_what_was_measured`) | contract | real filesystem | same |
| bi: An action added without a help row fails `cargo test` | Unchanged existing test | contract | real `action_for`, real `INVENTORY` | same |
| bi: A binding removed from the driver and left in the help fails | Unchanged existing test | contract | real sweep, real `INVENTORY` | same |
| bi: The sweep's totals and the exemption set are pinned | Unchanged existing test; re-run to confirm the projection preserves the twenty-five-name union | contract | real sweep | same |
| bi: The sweep covers the mouse under both overlay states | `the_sweep_covers_the_mouse_under_both_overlay_states`, unedited — the projection must reproduce its two exact name sets | contract | real sweep | same |
| bi: (its remaining carried scenario) | Unchanged existing test | contract | real `INVENTORY` | same |

## Decisions

**Decision 1 — Retain the sweep's claims; reject row-count-versus-arm-count.**
The middle ground originally floated was to diff the table's row count against
`mouse_action`'s match-arm count. It does not hold: `mouse_action` is a **nested** match —
kind, then zone, then row kind — so its arms have no correspondence to table rows, and any
number derived from them would be arbitrary.

**Decision 2 — The outcome axis keeps the payload's constructor name; only geometry is
discarded.** This is the decision the first draft got wrong, and the measurement that settles
it: `action_name` maps every `Click(_)` to `"Click"`, and under that collapse the three list
click rows share one claim, as do the artifact-section-header click and the content-row press.
The outcome is therefore `Click(Change)`, `Click(Section)`, `Click(DetailHeader)`,
`Select(Begin)`, `Select(Extend)`, `SelectTab`, `Ignore` — the `Action` variant plus its
payload's constructor, with the geometry inside (`line`, `column`, `Rect`) dropped. It is a
**projection of the value the function already returned**, not a second computation that could
disagree with it, and it is the vocabulary the rows already use in prose. Keying on the row
kind `row_at`/`detail_cell` resolved would work equally but re-derives inside the test
something `mouse_action` has already decided.

**Decision 3 — The comparison is a pure function over `(rows, claims)`.**
Both failure directions must be provable and a defect must be plantable on either side. A test
that could only drive the real tree would have to doctor `SPEC.md` or `mouse_action` on disk.
Passing both sides as arguments follows the file's established synthetic-input idiom.

**Decision 4 — A row names a *set* of zones, and its outcome names the payload constructor;
both visible in the table.** The wheel rows genuinely span several zones — `List` and
`ListRow` for the list region, `Detail`, `DetailTab` and `DetailRow` for the detail region, a
span `SPEC.md` already states in prose — so one token per row would leave six active claims
uncovered. Alternative considered: an HTML comment per row carrying a machine key. Rejected —
a key the reader cannot see can disagree with the prose beside it unnoticed, re-creating the
exact failure this change exists to remove. Visible tokens also improve the table: they say
which region and which target each row is about, where several rows only imply it.

**Decision 5 — A closed gesture vocabulary, erroring on anything unlisted.**
Alternative considered: parse the Gesture cell's English. Rejected — ambiguous English yields
false negatives, and the failure mode is silent. The vocabulary is a short
`[(&str, &[MouseEventKind])]` table in the check's own source, pinned by length, covering the
five phrases the real table uses (`Wheel down`, `Wheel up`, `Left click`, `Left drag`,
`Anything else`), with an unmatched phrase a **hard error naming the row**.

**Decision 6 — Overlay state is its own axis, not a value in the zone slot.**
While `help.open`, `mouse_action` returns before consulting `ui::layout::zone` at all, so there
is no zone to record. The first draft wrote a sentinel into the zone slot; making it a separate
axis keeps the two passes disjoint without overloading a field, and lets a row name the overlay
state independently of its zones.

**Decision 7 — Exactly one catch-all row, required non-empty, its prose unparsed.**
Requiring the catch-all to enumerate every inert zone would be unreadable. It claims the
remainder — but only claims whose outcome is `Ignore`, so a new *active* binding can never hide
in it — and must claim at least one. A second zone-less `Ignore`-only row is an error rather
than a silent duplicate. **The admitted cost is precise:** two of the four false statements
`mouse-text-selection` left behind ("any drag", "the detail content area when the selected
artifact is not foldable") lived inside this row's prose, and this check does not reach them.
It catches the other two.

**Decision 8 — Strengthen the existing claim; keep the name-set equality as leg 1.**
Adding a fourteenth claim would move `CLAIM_COUNT` and the three sites pinned to it for no
gain. Keeping the name-set comparison as the first leg preserves its existing failure message
for a plain vocabulary mismatch, and means the stricter legs only run on a table whose
vocabulary already agrees.

**Decision 9 — The sweep gains a pinned dashboard-fixture axis.**
`mouse_action` is a function of the whole `Dashboard`, which `binding-inventory`'s own spec
states. Its `Drag(Left)` arm returns `Select(Extend)` from five zones when
`selection.is_some()`, and `sweep_dashboard` pins `selection: None` — so the clamp the table
documents is unobservable, and the two-way check would report that row vacuous and be right to.
The fixture set is explicit and asserted by length, at minimum selection-absent and
selection-present. Alternative considered: exempt such rows by name. Rejected as the default —
an exemption list is where this kind of hole goes to hide — but retained as the escape hatch
for a precondition no fixture can reasonably carry, pinned and counted like `EXEMPT_ACTIONS`.

**Decision 10 — Known claim collisions are listed by name and pinned by count.**
Two rows can legitimately share one claim: the click on a change row and the second click on
the row already selected both produce `Click(Change)`, because "a second click opens the
detail" is decided in `Dashboard::apply`, not in `mouse_action`. The vacuity direction cannot
tell them apart. Rather than weaken the assertion, the pair is listed in the check's source and
the list's length asserted, so a third row joining the collision fails.

## Risks / Trade-offs

- **The vacuity direction is blind to rows sharing a claim.** → Decision 10 bounds it: the one
  known pair is listed and the count pinned, so the blindness cannot silently grow.
- **The catch-all's prose is unparsed, and two of the four motivating defects lived there.** →
  Stated in the proposal's Non-Goals, in the spec, and in Decision 7. This change is a real
  improvement, not a complete one, and says which half it is.
- **Sweep wall time.** Review measured a triple-retaining dump at **88.5 s** in a debug build
  over six passes (~201,600 `mouse_action` + `zone` calls); the fixture axis doubles the cell
  work again. → The result is cached per `(fixture, overlay)` in `OnceLock`s and paid once per
  process, and `doc_contract` already runs ~43 s. The REFACTOR task measures the real figure;
  if the binary crosses roughly three minutes, trimming the second fixture to the wide frame
  alone is the lever, recorded there with its measurement rather than assumed here.
- **The closed gesture vocabulary is a maintenance point.** → Deliberate: pinned by length and
  erroring loudly, so it fails toward "a human must look", never toward a silent pass.
- **A `Zone` variant unreachable in the swept frames would be unclaimable.** → Not hypothetical
  in general — the clamp was exactly this, and Decision 9 fixes that instance. For any future
  one the escape hatch is the pinned exemption, never silence.
- **`SPEC.md` and the spec file can still drift from each other**, since `openspec/specs/` is
  written only by `openspec archive`. → Unchanged from every other claim in this file.

## Migration Plan

None required. The change is a test and prose: no deploy order, no backfill, no rollback step
beyond reverting the commit. `make check` is green before and after.

## Open Questions

None. The one genuine fork — what the claim key must be — was settled by measurement during
planning review rather than by argument, and is recorded in Decision 2.

## Visual Design

Not applicable. This change modifies no user-facing view and no email template. No design
source exists or is needed.
