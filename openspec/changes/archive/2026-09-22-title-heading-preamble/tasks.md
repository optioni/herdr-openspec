<!-- Checks below were run against HEAD `8ccd255` (the parent of the planning commit) and
     their exit status and selection count recorded beside them. Where a count moved at
     `6386318` — this change's own tasks.md joined the corpus it measures — both figures
     are given. -->

**Ordering.** Groups 1 through 5 are sequential, and the criterion is the standing
repository-wide veto `openspec/config.yaml` → `rules.tasks` already records: one crate, one
compile, and every gate sweeps the whole tree, so any group's gate run reaches every other
group's half-written files. No `parallel-after` marker is set. Group 2 would fail criterion 2
in any case — its guard drives the derivation group 1 writes.

## 1. The title rule, and what it renders as
<!-- kind: behavior -->

- [x] 1.1 RED: Write failing tests in `src/ui/app.rs`'s test module named for the spec
      scenarios *A document title heading is demoted to an unlabelled section*, *A title
      heading with no prose under it does not split the file*, *A whitespace-only title body
      contributes no section*, *A leading heading holding its own items is a group, not a
      title*, *Two headings at the file's shallowest level are both groups*, *A preamble and a
      demoted title are two unlabelled sections*, and *A spec tab's lone operation heading
      keeps its header row*. Assert `(label, depth)` through the existing `shape_of` helper,
      which returns **pairs** (`src/ui/app.rs:3313`), and read each section's `progress` and
      `operation` directly off `d.detail.sections[i]` — the demoted title's `progress`
      becoming `None` is the headline `SHALL` and `shape_of` cannot see it. The
      `ToggleSection` assertion needs `route: Route::Detail` and `detail.drawn_width:
      Some(78)`, without which `apply` never reaches the detail cursor and the check passes
      vacuously.

      Evidence at HEAD, from a probe reproducing `sync_detail`'s own formula over the public
      `split_headings` for the fixture
      `# drift — tasks\n\nIntro prose.\n\n## 1. Setup\n\n- [x] 1.1 a\n\n## 2. Build\n\n- [ ] 2.1 b\n`:

      ```
      cargo test --test zz_probe          # 2 executed, 2 failed, exit 101
      red_title_is_demoted
        left:  [(Some("drift — tasks"), 0), (Some("1. Setup"), 1), (Some("2. Build"), 1)]
        right: [(None, 0), (Some("1. Setup"), 0), (Some("2. Build"), 0)]
      red_titled_file_without_prose_does_not_split
        today the file contributes 2 sections and therefore splits, losing one of its
        two heading lines
      ```

      Both are RED because the behaviour is absent, not because the fixture is wrong: the
      `left` value is today's real derivation. The probe was deleted after measuring.

- [x] 1.2 RED: Write the render half in `src/ui/view.rs`, whose `WIDTHS` gate requires every
      `#[test]` there to name both `60` and `120`: for *A document title heading is demoted*,
      exactly two header rows at 120x40 and 60x40, both at column zero, with
      `ui::detail::section_at` resolving none of the first entry's rows; for *A title heading
      with no prose under it*, rows reading `# drift — tasks` and `## 1. Setup` both present
      at 120x20 and 60x20. Confirm both fail for the missing behaviour.

- [x] 1.3 GREEN: Add the private pure helper
      `fn title_heading(headings: &[HeadingSection], tracks_tasks: bool) -> Option<usize>` to
      `src/ui/app.rs`, returning `Some(0)` when the three clauses of
      `specs/artifact-folds` hold and `None` otherwise, with unit tests covering each clause
      failing alone. The body-emptiness predicate is **trimmed**, per design.md → Decisions D8.

- [x] 1.4 GREEN: Use it in `sync_detail`: exclude the title from `min_level` (falling back to
      `0` when demotion leaves no labelled heading), push a `None`-labelled section carrying
      its `body` at `base` depth when that body is non-empty after trimming, and skip its
      labelled push — inside the existing `for heading in headings` loop, so
      `current_operation` still advances over every heading (design.md → Decisions D6).

- [x] 1.5 GREEN: Change the `splits` gate's contribution count to count the sections the
      derivation yields rather than the headings — `has_preamble` plus a non-empty-after-
      trimming title body plus the remaining headings (design.md → Decisions D4). The existing
      `a_single_heading_task_file_is_not_split_and_keeps_its_heading` (`src/ui/app.rs:3493`)
      must stay green; its fixture's one heading holds two items, so clause 3 fails for it.

- [x] 1.6 REFACTOR: Clean up while the tests stay green, or state that none was needed.
      *None was needed: the walk was already one branch inside the heading loop, and
      `title_heading` already had the `preamble_len` doc-comment shape.*

- [x] 1.7 Run `cargo test --lib` — no regressions, recording the executed count. Not
      `ui::app` alone: `sync_detail` is driven from `src/ui/view.rs`, `src/ui/detail.rs` and
      `src/ui/driver.rs` as well, and a regression there would otherwise stay hidden until
      group 5.

## 2. The corpus guard
<!-- kind: operational -->

- [x] 2.1 CHECK: Record the survey at HEAD and its negative control. The guard asserts that
      every `openspec/changes/**/tasks.md` in this repository which the title rule demotes
      still yields **more than one** section, so no file's split decision changes.

      ```
      scanned 48 files; offenders at HEAD: []                     # at 8ccd255
      scanned 49 files with the plant; offenders: ['PLANTED/tasks.md']
      ```

      Green at HEAD, so it is proven by its negative control: planting one synthetic
      `# T — tasks\n## 1. G\n\n- [ ] 1.1 x\n` makes it fire and removing it makes it quiet
      again. 48 files scanned, not zero; 49 at `6386318`, this change's own `tasks.md` having
      joined the corpus. The minimum post-demotion contribution count in the tree is 7.

- [x] 2.2 CHANGE: Add `tests/title_corpus.rs`. It reads each `tasks.md` itself, builds a
      one-artifact `Dashboard` with `tracks_tasks: true`, calls `Dashboard::sync_detail` with
      a closure returning those bytes, and asserts `detail.sections.len() > 1` — driving the
      real derivation and the real split gate rather than re-deriving the rule, which would
      stay green with `title_heading` absent (design.md → Boundaries). It finds the tree
      through `env!("CARGO_MANIFEST_DIR")`, prints the number of files scanned, and fails when
      that number is zero. It lives in `tests/` because `noio-view.sh`'s `PURE` list covers
      `src/ui/app.rs` and its grep is not `#[cfg(test)]`-stripped.
      *Clarified by the Change Review (W2, `7751748`): `sections.len() > 1` over every file
      with items was stronger than the rule guarded — an untitled one-group `tasks.md` failed
      it while blaming this change. The guard now asserts the split decision is **unchanged**,
      comparing the real post-change `sync_detail` against the pre-change contribution count
      re-derived from the public `split_headings` (the old formula, not the rule under guard).
      Both plants re-run: the titled single-group plant fires, the untitled one stays quiet.*

- [x] 2.3 VERIFY: Run `cargo test --test title_corpus` — green, with the printed scan count
      at 49 or more.

## 3. Change Review
<!-- kind: operational -->

- [x] 3.1 CHECK: Dispatch the `outside-in-tdd-reviewer` subagent — not a fork of this session
      — against `proposal.md`, both delta specs, `design.md`, `tasks.md`, `planning-review.md`,
      and the diff.
- [x] 3.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
      one-line reason, note each SUGGESTION, and re-run the affected tests.
      *0 CRITICAL, 3 WARNING, 8 SUGGESTION. Every WARNING fixed, each proved by a mutation
      that the new assertion catches (`4acd4b8`, `7751748`): W1 clause 1 of the title rule had
      no test; W2 the corpus guard asserted a stronger rule (see 2.2); W3 the view test's
      `section_at` loop could never fail and discarded the drawn buffer. SUGGESTIONS taken:
      a D6 operation-walk test, a positive `ToggleSection` control, a `min_level`-0 fallback
      shape test, a no-zero-row assertion, and change-qualified design citations. Routed to
      group 4 (4.3): the stale `ArtifactSection` doc at `src/ui/app.rs:230-233`, the
      "a `None`-labelled section is a preamble" comments in `src/ui/detail.rs`, and the
      `artifact-content` delta's "no header is selected" sentence, false at `base > 0` before
      this change. Task 1.6's missing note is added above.*
- [x] 3.3 VERIFY: Confirm no blocking or unowned finding remains.
      *None: `cargo test --lib` 1638 passed, `title_corpus` scanned 49, tree clean at
      `1c45b6e`; the remaining three findings are owned by 4.3.*

## 4. Documentation
<!-- kind: operational -->

- [x] 4.1 Rewrite in `SPEC.md`, **three** sites (audience: anyone implementing against the
      design contract), each verified to carry the claim it is named for:
      `:99`, the `| ui |` module-map row — "one section per resolved file and, inside a
      spec-shaped or tracked task file, one per heading";
      `:640-642` — "is split again at its own ATX headings, one section per heading, labelled
      with the heading's own text";
      and `:647-648` — "Text before a split file's first heading is a section with no label",
      which is now only one of the two shapes that produce one.
      `SPEC.md` says nothing about normalising depth against the shallowest heading — the words
      "shallowest", "normalis" and `min_level` appear nowhere in it — so no such sentence is to
      be hunted for. Correct all three in place; do not append beside any of them.
- [x] 4.2 Rewrite in `AGENTS.md`: the same claim in the "Current repo state" paragraph
      describing the detail region's foldable sections (audience: every agent session, which
      loads this file). Correct the existing sentence rather than appending. Net addition must
      stay under ten lines — the title rule replaces text rather than extending it.
- [x] 4.3 Rewrite the doc comment at `src/ui/detail.rs:4749-4751`, which asserts "17 of this
      repository's own 44 task files" put their groups at depth 1. The denominator is stale and
      the claim itself is falsified by this change; the test below it hand-builds its depths,
      so nothing fails on its own. *Widened by the Change Review* to the three further sites it
      found: `ArtifactSection`'s `progress` doc at `src/ui/app.rs:230-233` (its list of `None`
      cases omits the demoted title), the comments in `src/ui/detail.rs` near `:540`, `:698-702`
      and `:725` that call every `None`-labelled section a preamble, and
      `specs/artifact-content/spec.md:255-257` in this change's own delta, whose "no header is
      selected on a `None`-labelled row" holds only at `base` 0.
- [x] 4.4 VERIFY: Run `cargo test --all-features --test doc_contract` — green. This is a plain
      regression check and **not** evidence for 4.1–4.3: nothing in `tests/doc_contract.rs`
      reads either document's section-derivation prose, so it passes whether or not the
      rewrites happened.

## 5. Lint & Verify
<!-- kind: operational -->

- [x] 5.1 CHECK: Inspect the intended verification commands and the tiers they reach — unit
      and view under `cargo test --lib`, the corpus guard under
      `cargo test --test title_corpus`, the contract tier under `cargo test --all-features`,
      and coverage at both floors.
- [x] 5.2 VERIFY: Run `make lint` — 0 warnings (`clippy -D warnings`).
- [x] 5.3 VERIFY: Run `make fmt-check` — clean.
- [x] 5.4 VERIFY: Run `make gates` — every hygiene gate OK.
- [x] 5.5 VERIFY: Run `make covers-check` — the degraded-states table's proofs all pass, and
      repoint `tests/degraded-coverage.toml:84` (`src/ui/app.rs:2218-2222`, immediately after
      `sync_detail`'s heading loop), whose line range this change's edits will shift.
- [x] 5.6 VERIFY: Run `make test` — green.
- [x] 5.7 VERIFY: Run `make coverage` — both floors met; add tests rather than lowering either.
- [x] 5.8 VERIFY: Run `openspec validate title-heading-preamble --strict` — valid.
- [x] 5.9 VERIFY: Run `make check` as the single gate. If it fails, name the failing
      sub-command rather than summarising.
