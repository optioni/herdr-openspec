<!-- Checks below were run at planning time against HEAD `0d44237`. Counts name the command
     that produced them. `cargo test --lib -q -- --list` lists 1638 tests at HEAD; of those,
     the filters `worktrees::` and `branched_header` select 0, so every behavior group below
     starts RED by absence. -->

**Ordering.** Groups 0 through 13 are sequential. The criterion is the standing
repository-wide veto `openspec/config.yaml` → `rules.tasks` records — one crate, one compile,
and every gate sweeps the whole tree — so no `parallel-after` marker is set. Groups 1, 5, and 8
would fail criterion 1 regardless: all three edit `src/changes.rs`.

## 0. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

- [ ] 0.1 Build the harness named in design.md → Test Boundaries in `src/ui/mod.rs`'s test
      module: a `ScratchDir` base repository with active `x` at 0 of 3, a scratch member tree
      whose `x` counts 2 of 3, a scratch `#!/bin/sh` `git` answering the four
      `worktree-overlay` commands for that member (porcelain listing, a merge-base, empty
      `diff-tree`, and a `status` naming `openspec/changes/x/tasks.md`), and scratch `openspec`
      and `herdr` programs as the existing `run_wired` tests use.
- [ ] 0.2 RED: Write `run_wired_shows_a_worktree_copy_with_its_marker`, driving `run_wired`
      with `Startup::git` pointing at the scratch `git` until a frame's list interior holds a
      row ending ` @ [2/3]`, with a 10 s deadline-bounded poll of the `TestBackend`.
- [ ] 0.3 Confirm it fails to compile only on the missing `Startup::git` field, then — once
      group 9 adds the field with the worker not yet overlaying — fails on the missing row,
      not on the harness. Record both failure lines here.

## 1. The worktree family on the change set
<!-- kind: behavior -->

- [ ] 1.1 RED: Write `the_worktree_family_travels_with_the_set_and_nowhere_else` in
      `src/changes.rs`'s tests, covering `from_files`, `merge`, `empty_set`, and
      `assert_set_invariants` rejecting a duplicate root.
- [ ] 1.2 GREEN: Add `src/worktrees.rs` (declared in `src/lib.rs`) holding
      `Worktree { root: PathBuf, label: String }` deriving `Debug, Clone, PartialEq, Eq`; add
      `ChangeSet::worktrees` and set it at all 14 `ChangeSet {` sites in `src/changes.rs`
      (`grep -c 'ChangeSet {' src/changes.rs` → 14), carrying it through `merge`.
- [ ] 1.3 GREEN: Extend `conformance::assert_set_invariants`' exhaustive destructure with
      `worktrees` and the no-duplicate-root invariant; add `fixture::with_worktrees(set,
      &[(root, label)])` so tests outside `src/changes.rs` never write a `ChangeSet` literal
      (`NOLIT-CHANGE`).
- [ ] 1.4 CHECK: Re-inspect every `ChangeSet` consumer the compiler names and confirm none
      reads `worktrees` yet — the field is additive.
- [ ] 1.5 Run `cargo test --lib changes::` — green, recording the executed count (219 at HEAD).

## 2. The worktree marker in the list row
<!-- kind: behavior -->

- [ ] 2.1 RED: Write in `src/ui/list.rs` (whose `LISTWIDTHS` gate requires both `38` and
      `58` in every test): *A worktree row carries its marker after the badge*, *The worktree
      marker is dropped before the badge*, *A change archived in a worktree carries the marker
      on its archived row*, and the width-22/21 worktree arm of *An archived row drops the
      progress cell, then the date, as the width falls*, using the exact strings the
      `change-rows` delta gives.
- [ ] 2.2 GREEN: In `ui::list::rows`, derive "under a worktree root" from
      `change.dir.starts_with(&w.root)` over `dashboard.changes.worktrees`, insert the `@` cell
      after the badge, and drop it first, per design.md → D11.
- [ ] 2.3 REFACTOR: Fold the marker into the badge's existing cell-list arithmetic if the two
      duplicate each other, or record that no refactor was needed.
- [ ] 2.4 Run `cargo test --lib ui::list` — green (53 at HEAD, plus this group's), with every
      carried `change-rows` test unchanged.

## 3. The branch in the detail header
<!-- kind: behavior -->

- [ ] 3.1 RED: Write in `src/ui/detail.rs` (whose `DETAILWIDTHS` gate requires both `58` and
      `78` in every test) the three `branched_header_row` scenarios of the `detail-header`
      delta, and in `src/ui/view.rs` (which requires `60` and `120`) *The view draws the branch
      only for a worktree copy*.
- [ ] 3.2 GREEN: Add `ui::detail::branched_header_row` composing `header_row` per design.md
      → D12, and choose it at `src/ui/view.rs:164`'s one call site.
- [ ] 3.3 Run `cargo test --lib ui::detail` and `cargo test --lib ui::view` — green, every
      existing `header_row` test unchanged (`grep -c 'header_row(' -r src` → 20 at HEAD).

## 4. Parsing git's answers
<!-- kind: behavior -->

- [ ] 4.1 RED: Write in `src/worktrees.rs`'s tests the four scenarios of the family
      requirement and the two pure `touched` scenarios of `worktree-overlay`, plus
      `a_prunable_record_is_skipped_without_a_problem` (the degraded-states proof).
- [ ] 4.2 GREEN: Implement `parse_list`, `label`, `touched`, and the family selection taking
      the pane's canonical root and each record's canonical path (or `None`), per design.md →
      D14. No I/O, no CLI handle.
- [ ] 4.3 Run `cargo test --lib worktrees` — green, recording a non-zero executed count
      (0 at HEAD).

## 5. The overlay
<!-- kind: behavior -->

- [ ] 5.1 RED: Write in `src/changes.rs`'s tests the six overlay scenarios, the two conflict
      scenarios (naming the first `two_owners_show_the_first_and_name_both` for the
      degraded-states table), and a `from_files_owned` test proving it opens nothing beneath
      an unowned change, using the existing thread-local path recorder.
- [ ] 5.2 GREEN: Add `from_files_owned(root, &Touched, ArchivedScope)` over the private
      `build_change`, `archived_dir_names(repo)` over `archived_entries`, and `overlay(base,
      base_archive_dirs, members)`, per the `worktree-overlay` rules and design.md → D4/D5.
- [ ] 5.3 CHECK: Confirm the artifact cache, keyed on `(change directory, tab)`, re-reads when a
      row's copy moves between base and member — or record the existing test that proves it.
- [ ] 5.4 Run `cargo test --lib changes::` — green, with `assert_set_invariants` called on
      every set the new tests build.

## 6. The git seam
<!-- kind: behavior -->

- [ ] 6.1 RED: Write in `src/cli.rs`'s tests *The binding spawns the program it was given*,
      *The default program name is written down once*, the `GitCli` arm of *A trait object
      crosses a thread boundary*, and the `GitCli`-side arms of *An `openspec` call is not
      answered from a `herdr` registration*.
- [ ] 6.2 GREEN: Add `GitCli`, `RealGitCli`, `GIT_PROGRAM`, and `git_cli_via` on
      `RealHerdrCli`'s exact shape, and `Program::Git` with `impl GitCli for FakeCli`.
- [ ] 6.3 Run `cargo test --lib cli` — green (56 at HEAD, plus this group's).

## 7. `NOCLI-SHELL` sees the new trait
<!-- kind: operational -->

- [ ] 7.1 CHECK: At HEAD a planted `use crate::cli::GitCli;` at the top of `src/ui/list.rs`
      passes the gate, while the `HerdrCli` control fires. Run in a scratch copy of the tree:

      ```
      /bin/sh scripts/gates/nocli-shell.sh     # clean tree        → exit 0, 14 files
      # + 'use crate::cli::GitCli;'            # planted GitCli    → exit 0  (the gap)
      # + 'use crate::cli::HerdrCli;'          # control           → "NOCLI-SHELL FAIL … src/ui/list.rs:1"
      ```

- [ ] 7.2 CHANGE: Add `GitCli` to `CLI_RE` in `scripts/gates/nocli-shell.sh`, and add a
      `GitCli` plant beside `nocli-shell-hit` in `tests/gate-controls.toml`.
- [ ] 7.3 VERIFY: `make gates` green; `cargo test --test gate_controls` green with the new
      plant executed (it fails if the plant does not make the gate exit non-zero). Keep the
      tree quiet while it runs.

## 8. The refresh worker overlays and re-checks
<!-- kind: behavior -->

- [ ] 8.1 RED: Write in `src/refresh.rs`'s tests, through `worker_for_test(repo, cli, git,
      recheck)` with a 5 ms `recheck`: *A worktree copy reaches the merged result first and the
      file result after*, the four idle re-check scenarios, *No git binary* (named
      `no_git_binary_leaves_the_set_unoverlaid`), *Not a git repository*, *One member's query
      fails and the other still overlays* (named
      `a_failing_member_contributes_nothing_and_is_named`), *Only the four commands are run*,
      and the extended *No binary means no worker* (named
      `file_mode_reads_no_worktree_family`).
- [ ] 8.2 RED: Write the two real-`git` tests — *A full cycle over a real repository and
      worktree leaves git's files untouched* and *A worktree that forked before the base moved
      on owns nothing it did not touch* — building the repository through
      `cli::git_cli_via(cli::GIT_PROGRAM)` with the `-c` settings design.md → Risks names.
- [ ] 8.3 GREEN: Change `refresh::start` and `worker_for_test` to take the git handle (1 call
      site: `src/ui/mod.rs:244`; 7 `worker_for_test(` sites, `grep -rn 'worker_for_test(' src`),
      and implement the cycle and the re-check per the `refresh-worker` delta and design.md →
      D6–D8, with `WORKTREE_RECHECK` declared above the single `thread::spawn`.
- [ ] 8.4 REFACTOR: Share one "derive family and ownership" function between step 2 and the
      re-check, or record why none was needed.
- [ ] 8.5 Run `cargo test --lib refresh` — green (14 at HEAD, plus this group's), and
      `make gates` for `NOBLOCK` and `NOSLEEP`.

## 9. The composition root
<!-- kind: behavior -->

- [ ] 9.1 RED: Add `git: &Path` to every `Startup` literal the compiler names
      (`grep -rn 'Startup {' src tests` → 13 at HEAD) pointing at a scratch program, and
      confirm group 0's test now fails on the missing row rather than to compile.
- [ ] 9.2 GREEN: Add `Startup::git`; have `start_collaborators` build `cli::git_cli_via(git)`
      and pass it to `refresh::start`; have `run` pass `cli::GIT_PROGRAM`.
- [ ] 9.3 CHECK: Re-inspect `ui::Startup`, `start_collaborators`, and `refresh::start` against
      design.md → Contracts; no consumer outside the crate exists.
- [ ] 9.4 Run `cargo test --lib ui::` — green.

## 10. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [ ] 10.1 VERIFY: `cargo test --lib run_wired_shows_a_worktree_copy_with_its_marker` — 1
      executed, passing.
- [ ] 10.2 REFACTOR: Fold the scratch-`git` builder into the existing scratch-program helpers if
      they duplicate, or record that none was needed.

## 11. The seventeenth doc-contract claim
<!-- kind: operational -->

- [ ] 11.1 CHECK: `grep -n 'CLAIM_COUNT: usize' tests/doc_contract.rs` → `5000: … = 16` at
      HEAD, and no claim reads `src/worktrees.rs`; a `use std::fs;` planted in its production
      slice would pass `cargo test --test doc_contract` today.
- [ ] 11.2 CHANGE: Add the claim with the needle set the `doc-conformance` delta names, move
      `CLAIM_COUNT` to 17 and `CLAIM_COUNT_WORDS` to include `("seventeen", 17)`, and move the
      two prose sites in group 13.
- [ ] 11.3 VERIFY: `cargo test --test doc_contract` green; then, in a scratch copy, each plant
      the delta names makes it fail naming the needle and line.

## 12. Change Review
<!-- kind: operational -->

- [ ] 12.1 CHECK: Dispatch the `outside-in-tdd-reviewer` subagent — not a fork of this
      session — against `proposal.md`, every delta spec, `design.md`, `tasks.md`,
      `planning-review.md`, and the diff.
- [ ] 12.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
      one-line reason, note each SUGGESTION, and re-run the affected tests.
- [ ] 12.3 VERIFY: Confirm no blocking or unowned finding remains.

## 13. Documentation
<!-- kind: operational -->

- [ ] 13.1 CHECK: Add the five degraded-states rows below to `SPEC.md` and confirm
      `make covers-check` goes red naming them, before any `tests/degraded-coverage.toml` row
      exists.
- [ ] 13.2 Rewrite in `SPEC.md` (audience: anyone implementing against the contract): the module
      map (add `worktrees`; `cli` names three programs; `refresh` names the overlay and the
      re-check); the degraded-states table — add *`git` absent or not a git repository*, *A
      worktree's git query fails*, *A worktree's directory is gone*, *Two worktrees modify one
      change*, *Worktree changes in file mode*; the linked-worktree paragraph after attribution
      tier 3, which now says the change half is shown and the agent half waits on
      `worktree-agents` (row 30's description column likewise — its condition column is the
      toml key and stays); and the claim-seventeen bullet above the doc-conformance
      meta-statement. Rewrites in place; nothing is appended beside a stale sentence.
- [ ] 13.3 Add the five rows to `tests/degraded-coverage.toml`, each proof naming the test
      group 4, 5, or 8 named for it, with `covers` ranges measured after the code lands.
- [ ] 13.4 Rewrite in `AGENTS.md` (audience: every agent session): "Current repo state" gains
      `worktree-changes` in the landed list and one sentence on the overlay; the subprocess
      architecture rule says three traits and that the `GitCli` handle is confined to
      `src/cli.rs`, `src/refresh.rs`, and `src/ui/mod.rs`; "sixteen further claims" becomes
      seventeen with `src/worktrees.rs` in its list; Environment names `git` as a test
      prerequisite. Net addition under ten lines — each item edits an existing sentence.
- [ ] 13.5 Rewrite in `README.md` → Development: name `git` beside `python3` as a
      `make check` prerequisite (audience: a contributor running the gates).
- [ ] 13.6 VERIFY: `make covers-check` and `cargo test --test doc_contract` green.

## 14. Lint & Verify
<!-- kind: operational -->

- [ ] 14.1 CHECK: The tiers this change reaches are unit and view (`cargo test --lib`), the
      real-git unit tests, contract (`cargo test --test doc_contract`), gate controls
      (`cargo test --test gate_controls`), and coverage at both floors.
- [ ] 14.2 VERIFY: `make lint` — 0 warnings.
- [ ] 14.3 VERIFY: `make fmt-check` — clean.
- [ ] 14.4 VERIFY: `make gates` — every gate OK.
- [ ] 14.5 VERIFY: `make covers-check` — green.
- [ ] 14.6 VERIFY: `make test` — green.
- [ ] 14.7 VERIFY: `make coverage` — both floors met; add tests rather than lowering either.
- [ ] 14.8 VERIFY: `openspec validate worktree-changes --strict` — valid.
- [ ] 14.9 VERIFY: `make check` as the single gate; if it fails, name the failing sub-command.
