## Context

The tracked-tasks tab renders every row with equal weight. A change is mostly *done* by the
time anyone reads it, and the done part is the part the reader no longer cares about — but a
completed `VERIFY:` row is exactly as bright as the unstarted `RED:` row below it. Three
signals already in the data reach no pixel: each group's own checkbox count
(`tasks::Group::progress()`, computed and discarded), the lifecycle label at the head of 2272
of the archive's 2563 task items, and the group's `kind` marker.

Two facts about the current tree shape the whole design, and neither was true when the
proposal was drafted:

1. **`heading-sections` archived on 2026-09-14.** A real `tasks.md` carries headings, so its
   tab is now **foldable**: group headings are `artifact-folds` **fold header rows**, not
   `Face { heading }` lines emitted by `ui::tasks::lines`. Per-group progress therefore lands
   on `ArtifactSection` and on the header row grammar, which is an `artifact-folds` delta the
   proposal did not anticipate. `ui::tasks::lines` is reached only for a task file with no
   heading at all.
2. **`Face` is the crate's one carrier of "what this run of text is"**, and its shape is
   pinned by `markdown-render`. Saying "this row is finished" and "this token is a lifecycle
   label" means adding to `Face`, which makes `markdown-render` a second unanticipated delta.
   `markdown-legibility` set the precedent when it added `strikethrough` the same way.

The proposal's three Open Questions were answered before a line of spec was written, and the
answers are recorded in `proposal.md` → *Resolved before specs were written*. This document
does not re-argue them; it records the mechanisms they imply.

## Goals / Non-Goals

**Goals:**

- A completed task row reads as finished at a glance, its label included.
- An unfinished row's lifecycle label is distinguishable by position — evidence, change,
  confirmation — not by memorising seven words.
- A folded group still says how far along it is.
- The top progress bar shows where the group boundaries fall, without spending columns on
  separators.
- Every one of the above is added with **no** character of rendered text moved: the change
  splits rows into segments, faces them, and substitutes glyphs.

**Non-Goals:**

- Editing or toggling a task. The plugin writes nothing inside `openspec/`.
- Folding, which is `heading-sections`, already archived.
- The group `kind` marker, dropped with its `task-groups` delta.
- A schema-specific keyword list. `task-labels` reads no schema at all.
- A per-word colour. Four roles, not seven-plus.
- A new file under `src/ui/`, which would move the pure-view count two capability specs bind.

## Boundaries

| Module | What changes | Pattern it follows |
|---|---|---|
| `src/tasks.rs` | `LabelRole`, `Label`, `label_of` — pure, total, no view type | the module's existing `parse`/`count` shape: a pure function over `&str` |
| `src/ui/markdown.rs` | `Face` gains `muted: bool` and `label: Option<crate::tasks::LabelRole>`; `lines` sets neither | `markdown-legibility`'s `strikethrough`, added the same way |
| `src/ui/tasks.rs` | item rows split into faced segments; `progress_bar`/`bar_lines` take a group slice; the segmented gauge | unchanged: plain data, width-parameterised, no `ratatui` type |
| `src/ui/palette.rs` | five roles: `Muted`, `TaskEvidence`, `TaskChange`, `TaskConfirm`, `TaskLabel` | the existing role table; colour literals stay in its own tests |
| `src/ui/view.rs` | `style_for` grows from seven composing roles to nine | the existing fold-with-`patch` order |
| `src/ui/app.rs` | `ArtifactSection` gains `progress: Option<Progress>`, set only for a split tracked-tasks file | `heading-sections`' own `label: Option<String>` / `depth` addition |
| `src/ui/detail.rs` | the header row gains a right-aligned progress cell, dropped whole; `content_lines`' foldable branch feeds the gauge its group slice | `tasks-progress-bar`'s and `detail-header`'s drop-whole order |
| `src/ui/view.rs` (tests) | three tracked-tasks fold-header fixtures and their shared `expected_header_at` helper gain the progress cell | the fixture-amendment discipline every row-grammar change here has used |

**No process spawn is added anywhere.** No file this change touches names `process::Command`,
`Stdio`, `OpenspecCli`, or `HerdrCli`; `src/cli.rs` is untouched. **No I/O is added to a
view.** Every function added or changed under `src/ui/` is a pure function of its arguments:
`label_of` takes a `&str`, the gauge takes a `&[Progress]`, and the only filesystem reach in
the neighbourhood — `Dashboard::sync_detail`'s injected reader — is unchanged in signature and
in call count. `ui::markdown` naming `crate::tasks::LabelRole` widens neither `MDSEAM` (which
confines `pulldown_cmark`, and greps for `ratatui|Modifier|Style|Span`) nor `NOIO-VIEW` (which
names `tasks::read`, not `tasks::`): `LabelRole` is a plain enum that reaches nothing.

**The `Change` type is not altered**, so `changes::from_files` and `changes::from_cli` need no
reconciliation. Every number this change renders is derived from values those two already
agree on: the bar's `progress` is `Change::progress` exactly as before, and a group's own
count comes from `tasks::parse` over text the reader already returned, which `task-groups`
pins to the CLI's counting rule.

## Contracts

Five signatures move. All five are **internal** to the crate — nothing is published, no
serialized form exists, and no consumer outside `src/` names any of them.

| Item | Before | After | Consumers |
|---|---|---|---|
| `markdown::Face` | seven fields | nine | `ui::view::style_for`, `ui::tasks::heading_line` (the one site that spells fields out), `ui::markdown`, and every test constructing a `Face` |
| `ui::tasks::progress_bar` | `(&Progress, u16)` | `(&Progress, &[Progress], u16)` | `ui::tasks::bar_lines` only |
| `ui::tasks::bar_lines` | `(&Progress, u16)` | `(&Progress, &[Progress], u16)` | `ui::tasks::lines`, `ui::detail::content_lines` |
| `app::ArtifactSection` | three fields | four | `sync_detail`, `content_lines`, `Detail::foldable`, test fixtures |
| `ui::view::style_for` | composes seven roles | nine | `ui::view` only |

Three are **additive at the call site** because `Face` derives `Default` and `Face::plain()`
stays the zero value: a construction written `..Face::plain()` compiles unchanged. The two
slice parameters are **breaking at the call site and additive in behaviour** — an empty slice
reproduces the previous output byte for byte, at every width and for every `Progress`.

**Both production callers pass a populated slice.** `ui::tasks::lines` derives it from its own
parse; `ui::detail::content_lines`' foldable branch passes `detail.sections`' `progress`
values, skipping `None`. The empty slice is what the crate's **test** call sites pass and what
a future caller with no group data would pass — not what the pane passes. Planning review found
an earlier draft of this paragraph saying `content_lines` passed the empty slice, which would
have made the segmented gauge unreachable in every frame while all five of its scenarios
passed; the render-tier scenario `tasks-progress-bar` now carries is the repair's own check.

`ui::tasks::gauge_of` does **not** move. It keeps the signature and output `header-progress-bar`
gave it, so `detail-header`'s twelve-column gauge is untouched in every respect.

Error surface: none of the five can fail. `label_of` returns `Option`, never `Result`, and is
total. There is no pagination, no streaming, and no backwards-compatibility surface.

## Persistence and Rollout

- **Migration**: none. No stored format changes.
- **Backfill**: none.
- **Seeding**: none. Fold seeding is `artifact-folds`', unchanged — a group whose subtree
  still holds incomplete work still opens.
- **Cache invalidation**: none. The `(change directory, tab)` artifact-content cache key is
  untouched; a group's progress is derived from text already in the cached section.
- **Index rebuild**: none.
- **Authorization**: none. The tab stays read-only; no key added, no key changed.
- **Observability**: none. The crate logs nothing.
- **Deployment**: `make build` and the pane picks it up. No manifest change, no config key, no
  new dependency, no MSRV move.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| Terminal (crossterm raw mode, alternate screen) | **replaced** — `ratatui::backend::TestBackend` at 120 and 60 columns; the real seam stays confined to `src/ui/terminal.rs` and is never reached | **replaced** (same) |
| Artifact reader (`&dyn Fn(&Path) -> Result<String, String>`) | **replaced** — a closure returning fixture text | **replaced** (same) |
| Filesystem | **not reached** — every fixture in this change is an in-memory `&str` or an in-memory `Dashboard` | **not reached**, with the one exception below |
| Working tree, read by `tests/doc_contract.rs` | **real**, read-only — task 10.4 runs `--test doc_contract`, which reads `SPEC.md` and `AGENTS.md` off the tree to bind their claims to the files that determine them | not reached |
| `openspec` binary (`OpenspecCli`) | **not reached** — no code path this change touches spawns it | **not reached** |
| Herdr socket (`HerdrCli`, agent poller, launcher) | **not reached** — no file under `src/ui/` may name `HerdrCli` | **not reached** |
| Filesystem watcher (`notify`, `FsEvents`) | **inert** — the loop tier passes a live tier that yields nothing | **not reached** |
| Refresh worker, launcher worker | **inert** — same | **not reached** |
| Event source (key and mouse input) | **replaced** — a scripted `EventSource`; this change binds no key, so it is reached only to drive a frame | **replaced** (same) |
| Clock | **not reached** — no file under `src/ui/` names `Instant::now()`, and this change adds none | **not reached** |
| `crate::tasks::parse` / `label_of` | **real** — both are pure functions over `&str` and there is nothing to replace | **real** |
| `ui::list::progress_cell` / `ui::tasks::gauge_of` | **real** — sharing them is the property under test; replacing either would make the agreement assertions vacuous | **real** |
| Gate scripts (`/bin/sh scripts/gates/*.sh`, `make gates`) | **real**, out of process, over the real working tree | not reached |
| `tests/gate_controls.rs` planted defects | **not reached** — this change adds no gate script | not reached |

No task may invent a boundary this table does not name.

## Test Strategy

This repository's tiers: unit tests over the pure modules; view tests rendering into a
`TestBackend` at **60 and 120 columns**; `run_loop` tests over a scripted event source;
run-time `ScratchDir` trees; and the contract tier in `tests/`. Commands: `make check`, or
individually `make fmt-check`, `make lint`, `make gates`, `make test`, `make coverage`.

**This change does not take the outer-loop acceptance test, and the reason is that it binds no
key and changes no state transition.** The `run_loop` tier exists to prove that an event
reaches a state change that reaches a frame. Every behaviour here is a function of a
`Dashboard` that is already synced: given the same state, the same frame comes out with
different faces and different glyphs. A `run_loop` row would drive a key that this change does
not touch in order to observe a rendering the view tier observes directly, which is a slower
restatement rather than an outer loop. The **view tier is the outermost tier that can fail
here**, and every rendered claim below lands there.

Two width conventions the tasks below must respect, because a gate enforces each:

- **`TASKWIDTHS`** requires every `#[test]` in `src/ui/tasks.rs` to name both `58` and `78`,
  with no exemption list. A test whose interesting widths are elsewhere — the wrap-split
  degradation at 12/14/16, the 0..=130 sweeps — must still name both, which is why the
  scenarios above list them as swept or contrasted values rather than leaving them implied.
- **`DETAILWIDTHS`** requires the same of `src/ui/detail.rs`. A width-free helper's tests
  belong in a file that gate does not sweep.

Every scenario appears at least once. Rows marked *carried* reproduce an existing requirement
whose scenario already has a passing test; the obligation there is that it **stays** green
byte-for-byte, which is what proves this change moved no rendered text.

**Every "byte-identical" assertion compares against a string literal recorded from HEAD and
written into the test**, never against a fresh call of the function under test. The second form
cannot fail and would silently convert the change's central claim — that no character of
rendered text moved — into a tautology. Tasks 4.5, 7.7 and 8.4 carry that wording explicitly,
and the Change Review group asks its reviewer to check exactly this.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| task-labels — The plain and compound label forms are both recognised | `tasks` unit test over `&str` literals | unit (pure) | none | `cargo test tasks::` |
| task-labels — A task number is skipped and does not become part of the label | `tasks` unit test over `&str` literals | unit (pure) | none | `cargo test tasks::` |
| task-labels — Unlabelled tasks are recognised as unlabelled | `tasks` unit test over `&str` literals | unit (pure) | none | `cargo test tasks::` |
| task-labels — The recognition is total over degenerate input | `tasks` unit test sweeping the degenerate set and calling `split_at` at every returned offset | unit (pure) | none | `cargo test tasks::` |
| task-labels — Every token in the table classifies to its own role | `tasks` unit test over all thirteen tokens | unit (pure) | none | `cargo test tasks::` |
| task-labels — An unrecognised run is a generic label, not a miss | `tasks` unit test over `&str` literals | unit (pure) | none | `cargo test tasks::` |
| task-labels — Matching is case-sensitive and whole-run | `tasks` unit test over `&str` literals | unit (pure) | none | `cargo test tasks::` |
| task-labels — The classification reads nothing outside its argument | `grep` over `src/tasks.rs` for the schema-reading names, plus a compile-time test constructing no `Schema` | unit (pure) + deterministic evidence | the working tree | `cargo test tasks::` and `grep -nE 'schema::\|Schema\|config\.yaml\|\.openspec\.yaml' src/tasks.rs` |
| markdown-render — The markdown path sets neither new face field | `ui::markdown` unit test at 58 and 78 asserting every segment's two new fields | unit (pure) | none | `cargo test ui::markdown` |
| markdown-render — A paragraph is word-wrapped, differently at the two mandated widths *(carried)* | existing test stays green | unit (pure) | none | `cargo test ui::markdown` |
| markdown-render — An empty source and a zero width each produce no lines *(carried)* | existing test stays green | unit (pure) | none | `cargo test ui::markdown` |
| markdown-render — No line exceeds the width it was given *(carried)* | existing test stays green | unit (pure) | none | `cargo test ui::markdown` |
| markdown-render — A wide-character document wraps by columns at both mandated widths *(carried)* | existing test stays green | unit (pure) | none | `cargo test ui::markdown` |
| markdown-render — Rendering is total over arbitrary input *(carried)* | existing test stays green | unit (pure) | none | `cargo test ui::markdown` |
| tasks-checklist — A labelled unchecked item splits into three segments | `ui::tasks` unit test at 58 and 78 asserting segment count, texts, and faces | unit (pure) | `tasks::parse` and `label_of` **real** | `cargo test ui::tasks` |
| tasks-checklist — A labelled unchecked item splits into three segments | assert each row's `Line::text()` equals the pre-change literal | unit (pure) | none | `cargo test ui::tasks` |
| tasks-checklist — A checked item is de-emphasised whole, label included | `ui::tasks` unit test at 58 and 78 comparing the `[x]` and `[ ]` forms against each other | unit (pure) | `tasks::parse` and `label_of` **real** | `cargo test ui::tasks` |
| tasks-checklist — A wrapped labelled item labels only its first row | `ui::tasks` unit test at 58, contrasted against 78 where it does not wrap | unit (pure) | `tasks::parse` and `label_of` **real** | `cargo test ui::tasks` |
| tasks-checklist — A label split across a wrap degrades to unlabelled | `ui::tasks` unit test at 12, 14, 16, contrasted against 58 and 78 | unit (pure) | `tasks::parse` and `label_of` **real** | `cargo test ui::tasks` |
| tasks-checklist — A folded group and an unfolded one render the same item lines *(amended)* | existing test, its face assertions widened to the two new fields | unit (pure) | `tasks::parse` **real** | `cargo test ui::tasks` |
| tasks-checklist — Groups, headings, items, and separators at both mandated widths *(amended)* | existing test, its face assertions widened | unit (pure) | `tasks::parse` **real** | `cargo test ui::tasks` |
| tasks-checklist — A foldable tasks tab draws its groups as fold headers *(carried)* | existing view test stays green apart from the new progress cells | view | terminal **replaced**; reader **replaced** | `cargo test ui::view` |
| tasks-checklist — A nested item reproduces its own indent *(carried)* | existing test stays green | unit (pure) | `tasks::parse` **real** | `cargo test ui::tasks` |
| tasks-checklist — A long item wraps with a hanging indent at both widths *(carried)* | existing test stays green | unit (pure) | `tasks::parse` **real** | `cargo test ui::tasks` |
| tasks-checklist — An unbreakable word is hard-split rather than lost *(carried)* | existing test stays green | unit (pure) | `tasks::parse` **real** | `cargo test ui::tasks` |
| tasks-checklist — The indent is dropped whole as the width collapses *(carried)* | existing test stays green | unit (pure) | `tasks::parse` **real** | `cargo test ui::tasks` |
| tasks-checklist — A heading with no items still renders its heading *(carried)* | existing test stays green | unit (pure) | `tasks::parse` **real** | `cargo test ui::tasks` |
| tasks-checklist — A headingless leading group renders without a heading line *(carried)* | existing test stays green | unit (pure) | `tasks::parse` **real** | `cargo test ui::tasks` |
| tasks-progress-bar — Two groups of unequal size get spans proportional to their item counts | `ui::tasks` unit test at 58 and 78 counting each glyph | unit (pure) | `gauge_of` **real** | `cargo test ui::tasks` |
| tasks-progress-bar — An empty group contributes no span and consumes no index | `ui::tasks` unit test at 58 and 78 comparing the three-group and two-group forms | unit (pure) | `gauge_of` **real** | `cargo test ui::tasks` |
| tasks-progress-bar — Segmentation is skipped below the legibility floor | `ui::tasks` unit test sweeping 0..=130 with 22 groups, asserting a crossing in both directions | unit (pure) | `gauge_of` **real** | `cargo test ui::tasks` |
| tasks-progress-bar — A single group is never segmented | `ui::tasks` unit test at 58 and 78 against the empty-slice form | unit (pure) | `gauge_of` **real** | `cargo test ui::tasks` |
| tasks-progress-bar — Segmentation is total and partitions the run exactly | `ui::tasks` unit test sweeping 0..=130 over five adversarial group sets, summing glyph counts to `g` | unit (pure) | `gauge_of` **real** | `cargo test ui::tasks` |
| tasks-progress-bar — The full grammar at both mandated interior widths *(carried)* | existing test stays green with an empty slice | unit (pure) | `progress_cell` **real** | `cargo test ui::tasks` |
| tasks-progress-bar — The bar reaches the buffer at both mandated frame widths *(carried)* | existing view test stays green | view | terminal **replaced**; reader **replaced** | `cargo test ui::view` |
| tasks-progress-bar — The percentage truncates rather than rounds *(carried)* | existing test stays green | unit (pure) | none | `cargo test ui::tasks` |
| tasks-progress-bar — The bar measures at most its width at every width *(carried)* | existing sweep stays green, now also with a populated slice | unit (pure) | `gauge_of` **real** | `cargo test ui::tasks` |
| tasks-progress-bar — A saturating `Progress` renders a full gauge and a full percentage *(carried)* | existing test stays green, now also with a two-group slice | unit (pure) | `gauge_of` **real** | `cargo test ui::tasks` |
| artifact-folds — A tracked-tasks tab's group headers carry their own progress | `ui::app` unit test on `detail.sections`, plus a `TestBackend` render at 120 and 60 | unit (pure) + view | reader **replaced**; terminal **replaced**; `progress_cell` **real** | `cargo test ui::app` and `cargo test ui::view` |
| artifact-folds — Every other artifact's section headers carry no progress cell | the existing three-section fixture's view tests stay green with their assertions **unmodified** | view | terminal **replaced**; reader **replaced** | `cargo test ui::view` |
| artifact-folds — The progress cell is dropped whole rather than truncated | `ui::detail` unit test sweeping content widths 0..=40, naming 58 and 78 as the contrasted pair | unit (pure) | `progress_cell` **real** | `cargo test ui::detail` |
| artifact-folds — A group holding no items still gets a header and a counted cell | `ui::app` unit test on `detail.sections`, plus a `TestBackend` render at 120 | unit (pure) + view | reader **replaced**; terminal **replaced** | `cargo test ui::app` and `cargo test ui::view` |
| artifact-folds — The three spec files of a change become three labelled sections *(amended)* | existing test, its `ArtifactSection` literals gaining `progress: None` | unit (pure) | reader **replaced** | `cargo test ui::app` |
| artifact-folds — A spec glob nests requirements under their capability *(amended)* | existing test, same amendment | unit (pure) | reader **replaced** | `cargo test ui::app` |
| artifact-folds — A preamble becomes an unlabelled section *(carried)* | existing view test stays green | view | terminal **replaced**; reader **replaced** | `cargo test ui::view` |
| artifact-folds — A single-file artifact is one section and is not foldable *(carried)* | existing view test stays green | view | terminal **replaced**; reader **replaced** | `cargo test ui::view` |
| artifact-folds — An artifact with no resolved paths has no sections *(carried)* | existing test stays green | unit (pure) | reader **replaced** | `cargo test ui::app` |
| artifact-folds — An unreadable file drops its section and keeps its siblings *(carried)* | existing test stays green | unit (pure) | reader **replaced** | `cargo test ui::app` |
| artifact-folds — The label derivation is total over adversarial paths *(carried)* | existing test stays green | unit (pure) | reader **replaced** | `cargo test ui::app` |
| artifact-folds — Folding one section shows its body and leaves its siblings shut *(carried)* | existing view test stays green | view | terminal **replaced**; reader **replaced** | `cargo test ui::view` |
| artifact-folds — A fold hides a whole subtree *(carried)* | existing view test stays green | view | terminal **replaced**; reader **replaced** | `cargo test ui::view` |
| artifact-folds — The cursor's section header is the emphasised one *(carried)* | existing view test stays green | view | terminal **replaced**; reader **replaced** | `cargo test ui::view` |
| artifact-folds — A narrow pane truncates the label and keeps the glyph *(carried)* | existing `ui::detail` test stays green | unit (pure) | none | `cargo test ui::detail` |
| view-palette — The five new roles leave every existing cell's modifier where it was | `TestBackend` render at 120 and 60, modifier-only comparison against the pre-change buffer | view | terminal **replaced**; reader **replaced** | `cargo test ui::view` |
| view-palette — Every shared style is licensed, and the unshared roles stay unshared | `ui::palette` unit test grouping every role's `Style` by equality | unit (pure) | none | `cargo test ui::palette` |
| view-palette — A task label and a problem row are distinguishable in one frame | `TestBackend` render at 120 and 60 comparing two foregrounds against `palette::style` | view | terminal **replaced**; reader **replaced** | `cargo test ui::view` |
| view-palette — The two new face fields compose in their stated positions | `ui::view` unit test calling `style_for` on four `Face` values | unit (pure) | `palette::style` **real** | `cargo test ui::view` |
| view-palette — A checklist row reaches the buffer with its label coloured | `TestBackend` render at 120 and 60 | view | terminal **replaced**; reader **replaced** | `cargo test ui::view` |
| view-palette — Each role's modifier set is exactly the table above *(amended)* | existing test, its table gaining five rows and its no-modifier count moving 7 → 11 | unit (pure) | none | `cargo test ui::palette` |
| view-palette — The coloured set is exactly the table above *(amended)* | existing test, its table gaining four rows, plus the `LightRed`-not-`Red` assertion | unit (pure) | none | `cargo test ui::palette` |
| view-palette — A monochrome reading of the frame is unchanged *(carried)* | existing view test stays green | view | terminal **replaced**; reader **replaced** | `cargo test ui::view` |
| view-palette — An out-of-range heading level does not panic *(carried)* | existing test stays green | unit (pure) | none | `cargo test ui::palette` |
| view-palette — Faces reach the buffer as coloured styles at both mandated widths *(carried)* | existing view test stays green | view | terminal **replaced**; reader **replaced** | `cargo test ui::view` |
| view-palette — A section header's role is selected by its kind, not by its face *(carried)* | existing view test stays green | view | terminal **replaced**; reader **replaced** | `cargo test ui::view` |
| view-palette — Heading foreground wins over a code span inside it *(carried)* | existing view test stays green | view | terminal **replaced**; reader **replaced** | `cargo test ui::view` |
| tasks-progress-bar — A real tasks tab renders a segmented gauge into the frame | `TestBackend` render at 120x20 and 60x20 asserting `▓`/`▒` in the bar row | view | terminal **replaced**; reader **replaced** | `cargo test ui::view` |
| detail-scroll — The `ArtifactSection` companion names the fourth field | compile-time destructuring test, plus the `NODEFAULT-UI` invocation | unit (pure) + gate | the working tree | `cargo test ui::app` and `SCAN_MIN=25 TYPES='ArtifactSection' /bin/sh scripts/gates/nodefault-ui.sh` |
| detail-scroll — Startup leaves the detail empty and unscrolled *(carried)* | existing `ui::load` test over a `ScratchDir` stays green | run-time (scratch tree) | filesystem **real** (`crate::testutil::ScratchDir`) | `cargo test ui::tests` |
| detail-scroll — `Detail` has no `Default` and no site elides a field *(carried)* | existing test and gate stay green | unit (pure) + gate | the working tree | `cargo test` and `make gates` |
| view-palette — The enum's membership is exactly this list | `ui::palette` unit test with an exhaustive `match` over `Role` | unit (pure) | none | `cargo test ui::palette` |
| view-palette — The palette answers every role with a `Style` *(amended)* | existing test, its shared-pair assertions widened from two pairs to five groups | unit (pure) | none | `cargo test ui::palette` |
| view-palette — The confinement gate catches a `Color` named outside the palette *(carried)* | existing gate-control stays green | gate | the working tree, copied to a scratch directory | `cargo test --test gate_controls` |
| view-palette — The palette module reaches no I/O and measures no width *(carried)* | existing gate stays green | gate | the working tree | `/bin/sh scripts/gates/noio-view.sh` |
| view-palette — A plain face is the default style *(carried)* | existing test stays green with the two new fields at their zero | unit (pure) | none | `cargo test ui::view` |

## Decisions

**Decision 1 — `label_of` lives in `src/tasks.rs`, not under `src/ui/`.**
Recognising a label is a fact about a task's text, not about how a frame is painted, and the
function takes a `&str` and returns plain data. Putting it under `src/ui/` would add an
eleventh pure-view file, which moves a count that `view-palette` and `responsive-layout` both
bind and which `noio-view.sh`, `colwidth.sh`, and `palette.sh` each carry as a `PURE` list —
five sites for a function that needs none of them. *Alternative considered:* a new
`src/ui/labels.rs`, rejected for that cost. *Alternative considered:* inlining the rule into
`ui::tasks::item_lines`, rejected because the recognition is the part with the interesting
edge cases and it deserves its own tests at its own tier.

**Decision 2 — `Face` gains two fields rather than a new segment type or a `ContentKind`.**
`Face` is the crate's one carrier of "what this run of text is", and both new facts are facts
about a run of text. The test of where a distinction belongs is already written down in
`view-palette`: a *row-wide* distinction the renderer knows and the text does not is a
`ContentKind`; a distinction about a run of text is a `Face`. `muted` happens to cover every
run on its row, but nothing about the rule turns on that. *Alternative considered:* a
`ContentKind::TaskItem { checked }`, rejected because the label is emphatically not row-wide —
it is one token inside the row — so the label would have needed a `Face` anyway and the change
would have had two mechanisms. *Alternative considered:* a parallel `Vec<Role>` beside the
segments, rejected because it can fall out of step with the segments it describes and nothing
would catch it; the struct field is a compile-time forcing site, which is the same reason
`markdown-legibility` gave for `strikethrough`.

**Decision 3 — segmentation is a glyph substitution over the run `gauge_of` already returns.**
The gauge's fill count is computed exactly as it is today, and segmentation decides only which
of two shades each position is drawn with. This is the decision that keeps the change small:
`filled == g` iff complete, `filled == 0` when `completed == 0`, and the `u128` arithmetic
`header-progress-bar` repaired all hold **by construction** rather than by a second assertion,
and `gauge_of` itself does not move, so `detail-header`'s twelve-column gauge is untouched.
Planning review pushed back on "by construction" and it now carries an assertion beside it:
the `0..=130` sweep compares the filled-glyph count against the same call made with an empty
slice, so the preservation is checked rather than argued. What the phrase does and does not
cover: it covers the
three **fill** properties, which are properties of `gauge_of`'s output and are untouched by a
substitution that preserves each position's filled/empty state. It does **not** cover the
substitution's own correctness — that the spans partition the run exactly, that the shades
alternate, and that the skip rule fires — each of which has its own scenario and its own
sweep. *Alternative considered:* compute each group's fill independently and concatenate,
rejected because the per-segment fills do not sum to the global formula — a change at 7/31 would have
rendered a gauge disagreeing with its own count cell — and because it would have created the
crate's second fill computation, which is exactly what `header-progress-bar` spent a change
removing.

**Decision 4 — spans are proportional by cumulative flooring, not by largest remainder.**
With `e_i = floor(g * cumulative_i / T)`, the spans partition the `g` positions exactly for
every `g` and every set of totals, with no rounding residue to distribute and no tie-break
rule to specify. It is one expression, it is monotone, and "no position unassigned or assigned
twice" is a property that can be asserted over a sweep rather than checked per fixture.
*Alternative considered:* largest-remainder apportionment, which distributes the residue more
evenly across groups but needs a deterministic tie-break and a second pass, for a difference
of at most one column per group. *Alternative considered:* equal spans, rejected as the answer
to the proposal's Open Question 2 — a one-item group and a nine-item group would look
identical and the filled fraction would stop tracking the count cell beside it.

**Decision 5 — the legibility floor is `g >= 2 * n`, stated in columns.**
A one-column span cannot be read as a shade run, so a gauge that cannot give every contributing
group two columns shows no boundaries rather than unreliable ones. The figure is not
arbitrary: the archive's worst case is 22 groups, the narrow interior gives the gauge ~47
columns, and `2 * 22 = 44 <= 47`, so the pane segments at its own narrow mandated width and
the floor bites only below it. *Alternative considered:* a fixed minimum frame width, rejected
because the gauge's width is `width - 11` and a rule stated in frame columns would have to
restate that arithmetic in a second place.

**Decision 6 — `TaskEvidence` takes `LightRed`, not `Red`.**
This is the proposal's Open Question 1. `Role::ListProblem` is `Red` and is drawn in the
**detail** region — the same region a task label is drawn in — so `Red` would have been a
share with no licence. `LightRed` collides instead with `AgentBadge(Blocked)`, which is drawn
only in the list region, so the two can never meet. *Alternative considered:* `Red` with the
overlap accepted, on the grounds that a full `!`-marked problem row and a short leading token
are distinguishable by shape. Rejected: shape is a weaker signal than colour in a region the
reader scans for exactly one red thing.

**Decision 7 — `view-palette`'s "exactly two pairs share a style" enumeration is replaced by
the two licences it was an instance of.** Five new roles would have grown an enumeration with
no principle to grow it by. The licences — *the roles cannot meet*, or *they mean the same
thing* — are both already the justifications the requirement gave for its two pairs, so this
is a restatement rather than a relaxation, and it makes the new shares checkable: a test can
group every role by `Style` equality and require each group of size greater than one to be one
of the enumerated consequences. *Alternative considered:* leaving the sentence and appending
four more pairs, rejected because the next change would face the same edit with no more
guidance than this one had.

**Decision 8 — a checked item is one muted segment carrying `label: None`, not a dimmed
coloured label.** The proposal's complaint is a bright `VERIFY:` on a finished row; dimming it
while keeping its hue answers half of it. Dropping the role entirely is also the simpler
assertion — "no cell of this row carries `TaskConfirm`'s foreground" — and it is what makes the
checked and unchecked forms of the same text a usable pair of fixtures. *Alternative
considered:* `DIM` patched over the label colour, rejected on both counts.

**Decision 9 — a label split across a wrap degrades to unlabelled.**
The label is looked up against `item.text`, not against the rendered row, so a wrap falling
inside `CHARACTERIZE:` would otherwise produce a label segment reading `CHARACT`. When
`start + len` exceeds the first row's own text length the item renders as one plain segment.
*Alternative considered:* styling the partial token, rejected as visibly wrong.
*Alternative considered:* forcing the label onto its own row, rejected because it moves
rendered text, which this change otherwise never does.

**Decision 10 — `Muted` is its own role rather than a reuse of `Quoted`.**
Both are plain `DIM` and the palette does not police plain-modifier equality, so the table is
unaffected either way. The role exists so that a reader tracing why a row is dim lands on a
role that says "this is finished" rather than one that says "this is a block quote".

**Decision 11 — the `kind` marker is dropped, and `task-groups` with it.**
The proposal's Open Question 3. The fold-header row already gains a progress cell, and a badge
beside it did not earn its columns at the 58-column interior. Retaining a marker in
`tasks::parse` that nothing renders would be spec'ing dead data, so the `task-groups` delta is
not written at all. It stays a real signal a later change may take.

**Decision 12 — the recognition rule requires a whole-word run and a colon somewhere, not a
colon immediately after the run.** Measured against the archive: the strict
immediate-colon rule catches 2196 of 2272 labelled items; the whole-word-plus-colon rule
catches all 2272, and produces **zero** false positives over 2563 items. The 76 it adds are
the compound forms a reader would obviously want coloured — `CHANGE — rewrite in \`SPEC.md\`:`,
`RED then GREEN:`, `RED-by-addition:`. *Alternative considered:* a keyword allow-list, which
would have had to be extended for every schema and would have contradicted the non-goal of
reading the schema at all. *Alternative considered:* the strict rule, rejected for the 76.

## Risks / Trade-offs

**`spec-emphasis` modifies `markdown-render` and `view-palette` too, and whichever change
archives second silently discards the first's edits.** → This is the hazard
`IMPLEMENTATION-ORDER.md` records three times, and it is live: `spec-emphasis`'s proposal
names both capabilities. A `MODIFIED` block carries the whole requirement, and `git` reports
nothing when one overwrites another. **Before archiving this change**, re-extract the
`markdown-render` "Markdown source becomes plain-data lines" block and all three
`view-palette` blocks from `openspec/specs/<capability>/spec.md` rather than trusting the
copies made here, and compare by **phrase**, never by line. Whoever writes `spec-emphasis`'s
specs after this change archives must do the same in the other direction.

**`TASKWIDTHS` has no exemption list, so a new `ui::tasks` test whose interesting widths are
12/14/16 fails the gate.** → Every scenario in this change that exercises a width other than
58 or 78 names both as a contrasted or swept value, and the tasks below carry that as an
explicit Red-when. The gate is a floor of 22 tests; this change raises the count, so the floor
still passes without being moved.

**`ArtifactSection` gains a field, and `NODEFAULT-UI`'s `SCAN_MIN=25` for that type set is a
floor measured before it.** → Adding a field adds construction sites, so the count rises and
the floor still passes. No `Makefile` edit is needed. If a task finds otherwise, the floor
moves up, never down — `notes/gate-floors.md` records that discipline.

**`Face` gaining two fields touches every construction site that spells its fields out.** →
`markdown-render` already records that there is exactly **one** such site in the crate,
`ui::tasks::heading_line`, and that it is a compile-time forcing site. Every other site writes
`..Face::plain()`. The compiler names any site this estimate missed.

**Four gauge glyphs are East Asian Ambiguous where there were two.** → This widens the
accepted, uncompensated CJK-locale exposure `SPEC.md` already records; it creates no new class
of failure, because a terminal resolving Ambiguous to two columns already painted this gauge
at twice its width. `SPEC.md`'s sentence is updated to say four rather than two.

**The bar's `progress` and the sum of its group slice may disagree**, because the first is
`Change::progress` — which `change-merge` may have replaced with the CLI's count — and the
second comes from parsing the file. → Segmentation uses the **slice's** sum for span
arithmetic and the `Change`'s own progress for the fill, so a disagreement moves no rendered
output: the gauge is filled by the authoritative number and divided by the parsed one. This is
stated in the spec rather than left to be discovered.

**A frame could in principle hold both a label face and a heading face**, which share
foregrounds (`TaskChange`/`Heading(4)`, `TaskConfirm`/`Heading(3)`). → It cannot: a heading
face reaches the detail content area only on the markdown path or on a **non-foldable**
tracked-tasks tab, and a non-foldable tracked-tasks tab is by `artifact-folds`' own definition
one whose file carries no heading. The argument is written into `view-palette`'s delta so a
later change that makes a tasks tab foldable-with-heading-lines is forced to re-examine it.

## Migration Plan

None is needed. No persisted format, no configuration key, no manifest entry, and no
dependency changes; the pane picks the new rendering up on the next `make build`. Rollback is
`git revert` of the change's commits — there is no state written anywhere that a revert would
leave behind, because the plugin's only write is `agent-names.toml` and this change does not
touch it.

## Open Questions

None remain. The proposal's three are answered and recorded in `proposal.md` → *Resolved
before specs were written*, and the fourth that fell out of them — that the change adds five
palette roles rather than four — is recorded there too.
