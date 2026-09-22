<!-- Checks below were run at HEAD 8ccd255 and their exit status and selection count
     recorded beside them. -->

**Ordering.** Groups 1 through 6 are sequential, and the criterion is the standing
repository-wide veto `openspec/config.yaml` → `rules.tasks` already records: one crate, one
compile, and every gate sweeps the whole tree, so any group's gate run reaches every other
group's half-written files. No `parallel-after` marker is set. Groups 1 and 2 would fail
criterion 1 in any case — both edit `src/ui/app.rs`.

## 1. The title rule in `sync_detail`
<!-- kind: behavior -->

- [ ] 1.1 RED: Write failing tests in `src/ui/app.rs`'s test module named for the spec
      scenarios *A document title heading is demoted to an unlabelled section*, *A leading
      heading holding its own items is a group, not a title*, *Two headings at the file's
      shallowest level are both groups*, *A preamble and a demoted title are two unlabelled
      sections*, and *A spec tab's lone operation heading keeps its header row*, each
      asserting the scenario's `(label, depth, progress)` triples through the existing
      `shape_of` helper and the existing `RecordingReader`. Run
      `cargo test --lib ui::app::tests::` and record the executed count and the failures.

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

- [ ] 1.2 GREEN: Add the private pure helper
      `fn title_heading(headings: &[HeadingSection], tracks_tasks: bool) -> Option<usize>` to
      `src/ui/app.rs`, returning `Some(0)` when the three clauses of
      `specs/artifact-folds` hold and `None` otherwise. Its own unit tests cover each clause
      failing alone.

- [ ] 1.3 GREEN: Use it in `sync_detail`: exclude the title from `min_level`, push a
      `None`-labelled section carrying its `body` at `base` depth when that body is non-empty,
      and skip its labelled push — inside the existing `for heading in headings` loop, so
      `current_operation` still advances over every heading (per design.md → Decisions D6).

- [ ] 1.4 GREEN: Change the `splits` gate's contribution count to count the sections the
      derivation yields rather than the headings — `has_preamble` plus a non-empty title body
      plus the remaining headings (per design.md → Decisions D4). The existing test
      `a_single_heading_task_file_is_not_split_and_keeps_its_heading` must stay green.

- [ ] 1.5 REFACTOR: Clean up while the tests stay green, or state that none was needed.

- [ ] 1.6 Run `cargo test --lib ui::app` — no regressions, and record the executed count.

## 2. What the demoted title renders as
<!-- kind: behavior -->

- [ ] 2.1 RED: Write failing render tests for the scenarios' rendering halves — *A document
      title heading is demoted to an unlabelled section* (exactly two header rows, both at
      column zero, `ui::detail::section_at` resolving none of the first entry's rows) and *A
      title heading with no prose under it does not split the file* (rows reading
      `# drift — tasks` and `## 1. Setup` both present). Site them in `src/ui/view.rs`, whose
      `WIDTHS` gate requires every test there to name both `60` and `120`, and render at
      120x40 and 60x40 (120x20 and 60x20 for the second).

- [ ] 2.2 GREEN: No production change is expected — `content_lines`, `visible_sections` and
      `bodies_are_indented` already handle a `None` label. If one is needed, it belongs in
      `src/ui/detail.rs` and its tests must name both `58` and `78` per `DETAILWIDTHS`.

- [ ] 2.3 VERIFY: Run `cargo test --lib ui::view` and `cargo test --lib ui::detail` — green,
      with the executed counts recorded.

## 3. The corpus guard
<!-- kind: behavior -->

- [ ] 3.1 RED: Add `tests/title_corpus.rs` asserting that every `openspec/changes/**/tasks.md`
      in this repository which the title rule demotes still yields **more than one** section,
      so no file's split decision changes. It lives in `tests/` and not under `src/ui/`
      because `NOIO-VIEW` forbids a pure view file from naming a filesystem API
      (design.md → Boundaries). The test SHALL print the number of files scanned and fail
      when that number is zero.

      Evidence at HEAD, from the same survey run as a script:

      ```
      scanned 48 files; offenders at HEAD: []
      scanned 49 files with the plant; offenders: ['PLANTED/tasks.md']
      ```

      The guard is green at HEAD, so it is proven by its negative control: planting one
      synthetic `# T — tasks\n## 1. G\n\n- [ ] 1.1 x\n` makes it fire and removing it makes it
      quiet again. 48 files scanned, not zero.

- [ ] 3.2 VERIFY: Run `cargo test --test title_corpus` — green, and confirm the printed scan
      count is 48 or more.

## 4. Change Review
<!-- kind: operational -->

- [ ] 4.1 CHECK: Dispatch the `outside-in-tdd-reviewer` subagent — not a fork of this session
      — against `proposal.md`, both delta specs, `design.md`, `tasks.md`, and the diff.
- [ ] 4.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
      one-line reason, note each SUGGESTION, and re-run the affected tests.
- [ ] 4.3 VERIFY: Confirm no blocking or unowned finding remains.

## 5. Documentation
<!-- kind: operational -->

- [ ] 5.1 Rewrite in `SPEC.md`: the paragraph describing how an artifact splits into sections
      (audience: anyone implementing against the design contract). It states that a file
      splits at every ATX heading and normalises depth against the shallowest; that is now
      false for a tracked-tasks file's document title, so the sentence is corrected in place
      rather than a second one added beside it.
- [ ] 5.2 Rewrite in `AGENTS.md`: the same claim in the "Current repo state" paragraph
      describing the detail region's foldable sections (audience: every agent session, which
      loads this file). Correct the existing sentence; do not append. Net addition to
      `AGENTS.md` must stay under ten lines — the title rule replaces text rather than
      extending it.
- [ ] 5.3 VERIFY: Run `cargo test --all-features --test doc_contract` — green, since several
      of those claims are machine-bound to the files that determine them.

## 6. Lint & Verify
<!-- kind: operational -->

- [ ] 6.1 CHECK: Inspect the intended verification commands and the tiers they reach — unit
      and view under `cargo test --lib`, corpus under `cargo test --test title_corpus`, the
      contract tier under `cargo test --all-features`, and coverage at both floors.
- [ ] 6.2 VERIFY: Run `make lint` — 0 warnings (`clippy -D warnings`).
- [ ] 6.3 VERIFY: Run `make fmt-check` — clean.
- [ ] 6.4 VERIFY: Run `make gates` — every hygiene gate OK.
- [ ] 6.5 VERIFY: Run `make covers-check` — the degraded-states table's proofs all pass, and
      repoint any `tests/degraded-coverage.toml` line range the edits to `src/ui/app.rs`
      shifted.
- [ ] 6.6 VERIFY: Run `make test` — green.
- [ ] 6.7 VERIFY: Run `make coverage` — both floors met; add tests rather than lowering either.
- [ ] 6.8 VERIFY: Run `openspec validate title-heading-preamble --strict` — valid.
- [ ] 6.9 VERIFY: Run `make check` as the single gate. If it fails, name the failing
      sub-command rather than summarising.
