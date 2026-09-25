<!-- Checks below were run at planning time against HEAD `2363e09`, whose `src/` and `tests/`
     are identical to `0d44237`. `cargo test --lib -q -- --list` lists 1638 tests; the module
     filters used below select 224 (`changes::`), 56 (`cli::`), 14 (`refresh::`), 53
     (`ui::list`), and 0 for `worktrees::` and `branched_header`, so every behavior group starts
     RED by absence. Site counts are from `grep -rnE '(^|[^A-Za-z0-9_])ChangeSet[[:space:]]*\{' src
     tests`, `grep -rn 'Startup {' src tests`, and `grep -rn 'worker_for_test(' src`. -->

**Ordering.** Groups 0 through 13 are sequential, citing the standing veto in
`openspec/config.yaml` → `rules.tasks`; no `parallel-after` marker is set.

## 0. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

- [x] 0.1 Build the harness named in design.md → Test Boundaries in `src/ui/mod.rs`'s test
      module: a `ScratchDir` base repository with active `x` at 0 of 3, a scratch member tree
      whose `x` counts 2 of 3, a scratch `#!/bin/sh` `git` answering the four `worktree-overlay`
      commands for that member and logging each call, and scratch `openspec` and `herdr`
      programs as the existing `run_wired` tests use. Add the inert plumbing the test needs to
      compile: `Startup::git`, set at the 6 `Startup {` literals and the 7 `ProbedStartup {`
      harness literals, and a `git: &Path` parameter on `start_collaborators`, unused until group
      8, passed at its 2 test calls (`src/ui/mod.rs:5669`, `:6169`).
- [x] 0.2 RED: Write `run_wired_shows_a_worktree_copy_with_its_marker`, driving `run_wired`
      through the existing `UntilReady` harness with the predicate "the scratch `git` log records
      a `status` call", then asserting a frame's list interior holds a row ending ` @ [2/3]`.
- [x] 0.3 Confirm it compiles and fails on the missing row — no `status` call is ever logged —
      not on the harness. Record the failure line here.
      - Confirmed: `cargo test --lib ui::tests::wiring::run_wired_shows_a_worktree_copy_with_its_marker`
        compiles and fails (after the `UntilReady` 30s deadline, since the predicate — a
        `status` line in the scratch git log — never goes true) at `src/ui/mod.rs:3701`:
        `panicked at src/ui/mod.rs:3701:13: a frame's list interior must hold a row ending
        ' @ [2/3]' for the worktree copy of change x: [...]` — the rendered rows show only the
        base repository's own `x [0/3]`, never the member's `@ [2/3]` row, and the scratch git
        log stays empty throughout (the preceding assertion, that no `status` call was ever
        logged, passes), confirming `Startup::git` is plumbed but unread.

## 1. The worktree family on the change set
<!-- kind: behavior -->

- [x] 1.1 RED: Write `the_worktree_family_travels_with_the_set_and_nowhere_else` in
      `src/changes.rs`'s tests, and in `src/worktrees.rs`'s tests the three `member_of`
      scenarios of `worktree-overlay`.
- [x] 1.2 GREEN: Add `src/worktrees.rs` (the seventeenth `pub mod` in `src/lib.rs`) holding
      `Worktree { root: PathBuf, label: String }` and `member_of`, per design.md → D11; in the
      same task add its `SPEC.md` Module-map row and a `worktrees::` entry under
      § Unit-tested modules, which `tests/doc_contract.rs`'s `module_map_matches_lib_rs` and
      `tested_modules_names_every_module` require.
- [x] 1.3 GREEN: Add `ChangeSet::worktrees` at `src/changes.rs`'s 7 literals (`:414, :1842,
      :1856, :1920, :3979, :7645, :7734`) and 2 destructures (`:166, :1792`), carry it through
      `merge`, and set it at the two `tests/` literals (`tests/title_corpus.rs:155`,
      `tests/doc_contract.rs:3192`). Extend `assert_set_invariants`' destructure with the
      no-duplicate-root invariant, and add `fixture::with_worktrees(set, &[(root, label)])`.
- [x] 1.4 CHECK: Re-inspect every `ChangeSet` consumer the compiler names; none reads
      `worktrees` yet — the field is additive.
- [x] 1.5 Run `cargo test --no-run`, `cargo test --lib changes::` (224 at HEAD, plus this
      group's), `cargo test --lib worktrees::`, and `cargo test --test doc_contract` — green; no
      refactor needed.

## 2. The worktree marker in the list row
<!-- kind: behavior -->

- [x] 2.1 RED: Write in `src/ui/list.rs` (whose `LISTWIDTHS` gate requires `38` and `58` in every
      test): *A worktree row carries its marker after the badge*, *The worktree marker is dropped
      before the badge*, *A pane inside a nested worktree marks none of its own rows*, *A change
      archived in a worktree carries the marker on its archived row*, and the width-22/21
      worktree arm of *An archived row drops the progress cell, then the date, as the width
      falls*, with the strings the `change-rows` delta gives.
- [x] 2.2 GREEN: In `ui::list::rows`, mark a row when `worktrees::member_of(&changes.worktrees,
      &change.dir)` is `Some`, insert the `@` cell after the badge, and drop it first, per
      design.md → D12.
- [x] 2.3 REFACTOR: Fold the marker into the badge's cell-list arithmetic if the two duplicate
      each other, or record that no refactor was needed.
      - No refactor needed: `active_style_row` and `archived_row_text` gained a `worktree: bool`
        parameter and a fullest-to-degenerate cascade (badge+marker, badge alone, marker alone,
        neither) mirroring the existing drop structure one-for-one; `BadgeCell::x` arithmetic is
        untouched.
- [x] 2.4 Run `cargo test --lib ui::list` — green (53 at HEAD, plus this group's), every carried
      `change-rows` test unchanged.
      - Confirmed: `cargo test --lib ui::list::` → 58 passed, 0 failed (53 + 5 new).

## 3. The branch in the detail header
<!-- kind: behavior -->

- [x] 3.1 RED: Write in `src/ui/detail.rs` (`DETAILWIDTHS`: `58` and `78` in every test) the
      three `branched_header_row` scenarios, and in `src/ui/view.rs` (`60` and `120`) *The view
      draws the branch only for a worktree copy*, nested arm included.
- [x] 3.2 GREEN: Add `ui::detail::branched_header_row` composing `header_row` and cutting the
      label by `ui::list::truncate_right`'s rule, per design.md → D13, and choose it at
      `src/ui/view.rs:164` through `member_of`.
- [x] 3.3 Run `cargo test --lib ui::detail` and `cargo test --lib ui::view` — green, every
      existing `header_row` test unchanged (17 references at HEAD); no refactor needed.
      - Confirmed: `cargo test --lib ui::detail::` → 77 passed; `cargo test --lib ui::view::` →
        167 passed, including the new `the_view_draws_the_branch_only_for_a_worktree_copy`
        scenario.

## 4. Parsing git's answers
<!-- kind: behavior -->

- [x] 4.1 RED: Write in `src/worktrees.rs`'s tests the six scenarios of the family requirement
      and the two pure `touched` scenarios.
- [x] 4.2 GREEN: Implement `parse_list`, `label`, `touched`, and the family selection — taking
      the pane's canonical root and each record's canonical path or `None`, choosing the longest
      containing top level as the base — per design.md → D10/D15. No I/O, no CLI handle.
- [x] 4.3 Run `cargo test --lib worktrees::` — green, a non-zero count; no refactor needed.

## 5. The overlay
<!-- kind: behavior -->

- [x] 5.1 RED: Write in `src/changes.rs`'s tests the six overlay scenarios, the two conflict
      scenarios (the first named `two_owners_show_the_first_and_name_both`), and a
      `from_files_owned` test proving it opens nothing beneath an unowned change through the
      existing thread-local path recorder; and in `src/ui/view.rs` (60/120) the rendered half of
      *Two worktrees touching one proposal*.
- [x] 5.2 GREEN: Add `from_files_owned(root, &Touched, ArchivedScope)` over the private
      `build_change`, "holds" as membership in the member's own archived enumeration, and
      `overlay(base, base_archive_dirs, members)`, per `worktree-overlay` and design.md → D4/D5.
- [x] 5.3 CHECK: Confirm the artifact cache, keyed on `(change directory, tab)`, re-reads when a
      row's copy moves between base and member — or name the existing test that proves it.
- [x] 5.4 Run `cargo test --lib changes::` and `cargo test --lib ui::view` — green, with
      `assert_set_invariants` called on every set the new tests build; no refactor needed.

## 6. The git seam
<!-- kind: behavior -->

- [x] 6.1 RED: Write in `src/cli.rs`'s tests *The binding spawns the program it was given* (the
      nine-argument vector), *The default program name is written down once*, the `GitCli` arm of
      *A trait object crosses a thread boundary*, and the `GitCli`-side arms of *An `openspec`
      call is not answered from a `herdr` registration*.
- [x] 6.2 GREEN: Add `GitCli`, `RealGitCli`, `GIT_PROGRAM`, and `git_cli_via` on `RealHerdrCli`'s
      exact shape, and `Program::Git` with `impl GitCli for FakeCli`.
- [x] 6.3 Run `cargo test --lib cli::` — green (56 at HEAD, plus this group's); no refactor needed.

## 7. The gates see the git seam
<!-- kind: operational -->

- [x] 7.1 CHECK: At HEAD nothing catches a stray git handle. In a scratch copy of the tree:

      ```
      /bin/sh scripts/gates/nocli-shell.sh      # + 'use crate::cli::GitCli;' in src/ui/list.rs → exit 0
      # + 'fn _p(p:&Path){ let _ = crate::cli::git_cli_via(p); }' in src/ui/list.rs:
      /bin/sh scripts/gates/nocli-shell.sh      # → exit 0
      /bin/sh scripts/gates/launchseam.sh       # → exit 0
      /bin/sh scripts/gates/wired.sh            # → exit 0
      # control: + 'use crate::cli::HerdrCli;'  # nocli-shell → "NOCLI-SHELL FAIL … src/ui/list.rs:1"
      ```

- [x] 7.2 CHANGE: Add `GitCli` to `nocli-shell.sh`'s `CLI_RE`; make `launchseam.sh`'s
      `HANDLE_RE` an environment parameter defaulting to its current value and add a third
      `gates:` line with `LAUNCH=src/refresh.rs`, the git names, and `ALLOWED='src/cli.rs
      src/refresh.rs src/ui/mod.rs'`; extend `wired.sh` to require `git_cli_via` in
      `start_collaborators` and `cli::GIT_PROGRAM` in `run` and to reject a `"git"` literal there;
      add a plant for each to `tests/gate-controls.toml`.
- [x] 7.3 VERIFY: `make gates` green; `cargo test --test gate_controls` green with every new plant
      executed. Keep the tree quiet while it runs.

## 8. The refresh worker overlays and re-checks
<!-- kind: behavior -->

- [x] 8.1 RED: Write in `src/refresh.rs`'s tests, through `worker_for_test(repo, cli, git,
      recheck)` with a 5 ms `recheck`: *A worktree copy reaches the merged result first and the
      file result after*; the six idle re-check scenarios; *No git binary*
      (`no_git_binary_leaves_the_set_unoverlaid`); *Not a git repository, a timeout, or no record
      for the root*; *A git too old for the listing is named once*
      (`a_git_too_old_for_the_listing_is_named_once`); *One member's query fails and the other
      still overlays* (`a_failing_member_contributes_nothing_and_is_named`); *A member with
      unrelated history owns nothing and is not a problem*; *A prunable record and an
      unresolvable path record no problem through the worker*
      (`a_prunable_record_and_an_unresolvable_path_record_no_problem`); *A member with no
      OpenSpec tree owns nothing*; *Only the four commands are run*; and the extended *No binary
      means no worker* (`file_mode_reads_no_worktree_family`).
- [x] 8.2 RED: Write the two real-`git` tests — *A full cycle over a real repository and worktree
      leaves git's files untouched* and *A worktree that forked before the base moved on owns
      nothing it did not touch* — building the repository through
      `cli::git_cli_via(cli::GIT_PROGRAM)` with the settings design.md → Test Boundaries names.
- [x] 8.3 GREEN: Change `refresh::start` and `worker_for_test` to take the git handle; in
      `start_collaborators` build `cli::git_cli_via(git)` and pass it (`src/ui/mod.rs:244`); give
      each of the 6 existing `worker_for_test` callers a `GitCli` registration (`worktree list` →
      `NotStarted`), since `FakeCli` panics on an unregistered call; implement the cycle and the
      re-check per the `refresh-worker` delta and design.md → D6–D8, with `WORKTREE_RECHECK`
      declared above the single `thread::spawn`.
- [x] 8.4 CHECK: Re-inspect `ui::Startup`, `start_collaborators`, and `refresh::start` against
      design.md → Contracts; no consumer outside the crate exists.
- [x] 8.5 REFACTOR: Share one "derive family and ownership" function between step 2 and the
      re-check, or record why none was needed.
- [x] 8.6 Run `cargo test --lib refresh::` (14 at HEAD, plus this group's) and `cargo test --lib
      ui::` — green; `make gates` for `NOBLOCK`, `NOSLEEP`, `LAUNCHSEAM`, and `WIRED`.

## 9. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [x] 9.1 VERIFY: `cargo test --lib run_wired_shows_a_worktree_copy_with_its_marker` — 1
      executed, passing.
- [x] 9.2 REFACTOR: Fold the scratch-`git` builder into the existing scratch-program helpers if
      they duplicate, or record that none was needed. — None needed: `git_script` already
      builds through the shared `write_script` helper.

## 10. The seventeenth doc-contract claim
<!-- kind: operational -->

- [ ] 10.1 CHECK: `grep -n 'CLAIM_COUNT: usize' tests/doc_contract.rs` → `5000: … = 16` at HEAD,
      and no claim reads `src/worktrees.rs`, so a `use std::fs;` or `root.canonicalize()` in its
      production slice passes `cargo test --test doc_contract` today.
- [ ] 10.2 CHANGE: In one commit, add the claim with the needle set the `doc-conformance` delta
      enumerates and its in-file negative controls over string literals, and move all four count
      sites: `CLAIM_COUNT` to 17; `CLAIM_COUNT_WORDS` to an 8-entry array ending
      `("seventeen", 17)`; `AGENTS.md`'s "sixteen further claims" and its list; the `SPEC.md` →
      Doc-conformance checks bullet above the trailing meta-statement.
- [ ] 10.3 VERIFY: `cargo test --test doc_contract` — green, the new negative controls among the
      executed tests.

## 11. Change Review
<!-- kind: operational -->

- [ ] 11.1 CHECK: Dispatch the `outside-in-tdd-reviewer` subagent — not a fork of this session —
      against `proposal.md`, every delta spec, `design.md`, `tasks.md`, `planning-review.md`, and
      the diff.
- [ ] 11.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a one-line
      reason, note each SUGGESTION, and re-run the affected tests.
- [ ] 11.3 VERIFY: Confirm no blocking or unowned finding remains.

## 12. Documentation
<!-- kind: operational -->

- [ ] 12.1 CHECK: `make covers-check` is green at the start of this group, and
      `grep -n 'Two traits carry\|badge cell first\|is \*\*first\*\* in the header\|answers it twice\|cli::OpenspecCli' SPEC.md`
      (5 lines at HEAD: `:36, :345, :532, :566, :1220`) and
      `grep -n "that side's pure\|it is first in the header's" AGENTS.md` (2 lines: `:101, :403`)
      find the stale sites named below.
- [ ] 12.2 CHANGE: Rewrite in `SPEC.md` (audience: anyone implementing against the contract), in
      place: Architecture's "Two traits carry the two programs" and its code block; the module
      map's `cli` and `refresh` rows; § Refresh's "answers it twice" (the idle re-check); List
      view's drop order (the marker first); Detail view's "the gauge is **first** in the header's
      drop-whole order" (the branch cell now is); the Unit-tested modules `cli` bullet (`GitCli`);
      the linked-worktree paragraph after attribution tier 3 and the description column of its
      degraded row (the change half is shown; the agent half waits on `worktree-agents`).
- [ ] 12.3 CHANGE: Add to `SPEC.md`'s degraded-states table six rows — *`git` absent, or the
      repository is not a git repository*, *A worktree's git query fails*, *A worktree's directory
      is gone*, *Two worktrees modify one change*, *`git` too old for `worktree list -z`*, and
      *Worktree changes in file mode* — and the matching `tests/degraded-coverage.toml` rows,
      proofs `no_git_binary_leaves_the_set_unoverlaid`,
      `a_failing_member_contributes_nothing_and_is_named`,
      `a_prunable_record_and_an_unresolvable_path_record_no_problem`,
      `two_owners_show_the_first_and_name_both`, `a_git_too_old_for_the_listing_is_named_once`,
      and `file_mode_reads_no_worktree_family`, with `covers` measured now.
- [ ] 12.4 CHANGE: Rewrite in `AGENTS.md` (audience: every agent session), in place: "Current repo
      state" gains `worktree-changes` and one sentence on the overlay, and its gauge-first
      sentence names the branch cell; the pure-classifiers sentence names `src/worktrees.rs` and
      claim seventeen; the subprocess rule says three traits and names the git handle's three
      files; Environment names `git` (measured on 2.48.1). Net addition under ten lines.
- [ ] 12.5 CHANGE: Rewrite in `README.md` → Development: name `git` beside `python3` as a
      `make check` prerequisite (audience: a contributor running the gates).
- [ ] 12.6 VERIFY: `make covers-check` and `cargo test --test doc_contract` — green.

## 13. Lint & Verify
<!-- kind: operational -->

- [ ] 13.1 CHECK: The tiers this change reaches are unit and view (`cargo test --lib`), the
      real-git unit tests, contract (`cargo test --test doc_contract`), gate controls
      (`cargo test --test gate_controls`), and coverage at both floors.
- [ ] 13.2 VERIFY: `make lint` — 0 warnings.
- [ ] 13.3 VERIFY: `make fmt-check` — clean.
- [ ] 13.4 VERIFY: `make gates` — every gate OK.
- [ ] 13.5 VERIFY: `make covers-check` — green.
- [ ] 13.6 VERIFY: `make test` — green.
- [ ] 13.7 VERIFY: `make coverage` — both floors met; add tests rather than lowering either.
- [ ] 13.8 VERIFY: `openspec validate worktree-changes --strict` — valid.
- [ ] 13.9 VERIFY: `make check` as the single gate; if it fails, name the failing sub-command.
