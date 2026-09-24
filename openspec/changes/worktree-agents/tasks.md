<!-- Checks below were run at planning time against HEAD `5c24298`, before `worktree-changes` is
     implemented. This change applies only after it: `ChangeSet::worktrees` and
     `changes::fixture::with_worktrees` do not exist until that change's group 1 lands, so every
     behavior group below is RED by absence — `cargo test --lib -q -- --list` lists 1638 tests
     at `0d44237`, none of them naming a worktree scope or a launch root. Re-run 0.1 first. -->

**Ordering.** Groups 0 through 5 are sequential, citing the standing veto in
`openspec/config.yaml` → `rules.tasks`; groups 1 and 2 would fail criterion 1 anyway — both edit
`src/ui/app.rs`.

## 0. Precondition
<!-- kind: operational -->

- [ ] 0.1 CHECK: `openspec list --json` shows `worktree-changes` archived (absent from the active
      list), and `grep -n 'pub worktrees' src/changes.rs` matches once. If either fails, stop:
      this change cannot be applied yet.
- [ ] 0.2 CHANGE: Re-extract this change's three carried MODIFIED blocks from `openspec/specs/`
      and diff them against the deltas, since `worktree-changes`' archive moved the live specs;
      repair any drift in the delta before writing code.
- [ ] 0.3 VERIFY: `openspec validate worktree-agents --strict` — valid.

## 1. Attribution admits member worktrees
<!-- kind: behavior -->

- [ ] 1.1 RED: Write in `src/agents.rs`'s tests *An agent in a member worktree is in scope and
      placed by the ordinary tiers*, *Without a family a worktree-shaped path stays out of scope*,
      and *A member's OpenSpec root bounds its scope*; and in `src/ui/list.rs` (38/58 columns) *The
      dashboard passes the family it holds*, named `a_member_worktree_agent_badges_its_row` for the
      degraded-states table.
- [ ] 1.2 GREEN: Add `worktrees: &[&Path]` to `agents::attribute` and widen its scope test per
      the `agent-attribution` delta and design.md → D2/D3; pass `&[]` at the 40 existing test
      calls (`grep -c 'attribute(' src/agents.rs` → 41 including the definition), and the roots of
      `changes.worktrees` from `Dashboard::attribution`.
- [ ] 1.3 REFACTOR: None expected — the scope test is one expression; record it.
- [ ] 1.4 Run `cargo test --lib agents` (58 at `0d44237`) and `cargo test --lib ui::list` —
      green, every carried attribution test unchanged but for its `&[]`.

## 2. Launch splits in the row's checkout
<!-- kind: behavior -->

- [ ] 2.1 RED: Write in `src/launch.rs`'s tests *`decide` never fills the root* and *A launch onto
      a worktree copy splits in that worktree* (through the landed scratch-`herdr` worker harness),
      and in `src/ui/app.rs` *The worktree's root is filled from the selected row* and *A refusal is
      unchanged by the root*.
- [ ] 2.2 GREEN: Add `root: Option<PathBuf>` to `Request::Launch`, build it `None` in `decide`,
      fill it in `Dashboard`'s `Go` branch per design.md → D4, and split at
      `root.unwrap_or(settings.repo)`; the compiler names the 34 `Request::Launch {` sites
      (`grep -rn 'Request::Launch {' src`).
- [ ] 2.3 CHECK: Re-inspect `Request::Launch` and `agents::attribute` against design.md →
      Contracts; neither has a consumer outside the crate.
- [ ] 2.4 Run `cargo test --lib launch` (74 at `0d44237`) and `cargo test --lib ui::app` (201) —
      green.

## 3. Change Review
<!-- kind: operational -->

- [ ] 3.1 CHECK: Dispatch the `outside-in-tdd-reviewer` subagent — not a fork of this session —
      against `proposal.md`, both delta specs, `design.md`, `tasks.md`, `planning-review.md`, and
      the diff.
- [ ] 3.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a one-line
      reason, note each SUGGESTION, and re-run the affected tests.
- [ ] 3.3 VERIFY: Confirm no blocking or unowned finding remains.

## 4. Documentation
<!-- kind: operational -->

- [ ] 4.1 CHECK: Rewrite `SPEC.md` row 30's description (its condition column, the toml key,
      stays) and confirm `make covers-check` still passes on the old proof — then point the proof
      at `a_member_worktree_agent_badges_its_row` and confirm it passes on the new one.
- [ ] 4.2 Rewrite in `SPEC.md` (audience: anyone implementing against the contract): the
      attribution scope paragraph ("scoped to the resolved repository" → "or a member worktree"),
      the linked-worktree paragraph after tier 3 (no longer a limitation), row 30's description,
      and the launch flow's `--cwd <repo>` line. In place; nothing appended.
- [ ] 4.3 Rewrite in `AGENTS.md` (audience: every agent session) the sentence "every attribution
      tier is scoped to the resolved repository" to name member worktrees, since a future change
      touching scope would otherwise re-narrow it. Net change: one sentence.
- [ ] 4.4 Rewrite `tests/degraded-coverage.toml` row 30: proof, verdict, `why`, and `covers`
      measured after the code lands.
- [ ] 4.5 VERIFY: `make covers-check` — green.

## 5. Lint & Verify
<!-- kind: operational -->

- [ ] 5.1 CHECK: The tiers reached are unit and view (`cargo test --lib`) and the degraded-states
      contract (`make covers-check`); no gate script or doc-contract claim moves.
- [ ] 5.2 VERIFY: `make lint` — 0 warnings.
- [ ] 5.3 VERIFY: `make fmt-check` — clean.
- [ ] 5.4 VERIFY: `make gates` — every gate OK.
- [ ] 5.5 VERIFY: `make test` — green.
- [ ] 5.6 VERIFY: `make coverage` — both floors met.
- [ ] 5.7 VERIFY: `openspec validate worktree-agents --strict` — valid.
- [ ] 5.8 VERIFY: `make check` as the single gate; if it fails, name the failing sub-command.
