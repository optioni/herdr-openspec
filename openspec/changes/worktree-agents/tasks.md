<!-- Checks below were run at planning time against HEAD `2363e09`, before `worktree-changes` is
     implemented; `src/` there is identical to `0d44237`. This change applies only after
     `worktree-changes`: `ChangeSet::worktrees`, `changes::fixture::with_worktrees`, and
     `worktrees::member_of` do not exist until then. `cargo test --lib -q -- --list` lists 1638
     tests at `0d44237`, none naming a worktree scope or a launch root; the module filters select
     58 (`agents::`), 74 (`launch::`), and 201 (`ui::app`). -->

**Ordering.** Groups 0 through 5 are sequential, citing the standing veto in
`openspec/config.yaml` → `rules.tasks`; groups 1 and 2 would fail criterion 1 anyway — both edit
`src/ui/app.rs` (`attribution` at `:1957`, the launch `Go` branch at `:1743`).

## 0. Precondition
<!-- kind: operational -->

- [ ] 0.1 CHECK: `openspec list --json` shows no active `worktree-changes`;
      `grep -c 'pub worktrees' src/changes.rs` → 1; `grep -c 'fn with_worktrees' src/changes.rs`
      → 1; `grep -c 'pub fn member_of' src/worktrees.rs` → 1. At planning time all three greps
      return 0 and the change is active, so this check fails today as it should. If any fails,
      stop.
- [ ] 0.2 CHANGE: Re-extract this change's four carried MODIFIED blocks from `openspec/specs/`
      and diff them against the deltas, repairing any drift from a change archived since
      planning, before writing code.
- [ ] 0.3 VERIFY: `openspec validate worktree-agents --strict` — valid.

## 1. Attribution admits member worktrees
<!-- kind: behavior -->

- [ ] 1.1 RED: Write in `src/agents.rs`'s tests *An agent in a member worktree is in scope and
      placed by the ordinary tiers*, *A member's OpenSpec root bounds its scope*, and the guard
      *Without a family a worktree-shaped path stays out of scope*; in `src/ui/list.rs` (38/58)
      *The dashboard passes the family it holds*, named `a_member_worktree_agent_badges_its_row`;
      and in `src/ui/view.rs` (60/120) *The footer counts an unplaced agent in a member worktree*.
      Confirm the first two and the two dashboard tests fail on an assertion against a stub that
      only adds the parameter.
- [ ] 1.2 GREEN: Add `worktrees: &[&Path]` to `agents::attribute` and widen its scope test per the
      `agent-attribution` delta and design.md → D2/D3; pass `&[]` at the 40 existing test calls,
      and the roots of `changes.worktrees` from `Dashboard::attribution`.
- [ ] 1.3 Run `cargo test --lib agents::` (58 at `0d44237`, plus this group's), `… ui::list`, and
      `… ui::view` — green; no refactor needed, the scope test being one expression.

## 2. Launch splits in the row's checkout
<!-- kind: behavior -->

- [ ] 2.1 RED: Write in `src/launch.rs`'s tests *A launch onto a worktree copy splits in that
      worktree*, through `handle` with a root-parameterised `split_ok`, and the guard *`decide`
      never fills the root*; in `src/ui/app.rs` *The worktree's root is filled from the selected
      row*, *A pane inside a nested worktree launches its own rows in place*, and *A refusal is
      unchanged by the root, and `g` reaches the worktree agent*. Every `Request::Launch` literal
      names all four fields (`NODEFAULT-UI`).
- [ ] 2.2 GREEN: Add `root: Option<PathBuf>` to `Request::Launch`, build it `None` in `decide`, fill
      it in `Dashboard`'s `Go` branch through `worktrees::member_of` per design.md → D4 with a full
      four-field destructure, and split at `root.unwrap_or(&settings.repo)` in `handle`; the
      compiler names the 34 `Request::Launch {` sites (`grep -rn 'Request::Launch {' src`).
- [ ] 2.3 CHECK: Re-inspect `Request::Launch` and `agents::attribute` against design.md →
      Contracts; neither has a consumer outside the crate.
- [ ] 2.4 CHECK: Ask the user before touching their Herdr session; with their go-ahead, in a
      throwaway workspace, run `herdr pane split --cwd <a directory that does not exist>
      --direction right --no-focus` and record the exit status and whether a pane appeared in
      design.md → Risks. If Herdr silently falls back, add a degraded-states row and a refusal
      when `root` no longer exists, as a follow-up task here.
- [ ] 2.5 Run `cargo test --lib launch::` (74 at `0d44237`) and `cargo test --lib ui::app` (201) —
      green; no refactor needed, the root being one field and one expression.

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

- [ ] 4.1 CHECK: Record the stale sites, each non-zero before the change:
      `grep -n 'outside the resolved repository root' SPEC.md` (the scope paragraph and the
      out-of-scope degraded row); `grep -n 'linked worktree' SPEC.md` (the paragraph after tier 3
      and its degraded row, as `worktree-changes` left them); `grep -n '\-\-cwd <repo>' SPEC.md`;
      `grep -n 'agents::attribute' SPEC.md` (the Unit-tested modules bullet);
      `grep -c 'out_of_scope_and_worktree_agents_are_invisible' tests/degraded-coverage.toml` → 2.
- [ ] 4.2 CHANGE: Rewrite in `SPEC.md` (audience: anyone implementing against the contract), in
      place: the attribution scope paragraph; the degraded row whose condition reads "A live
      agent's `cwd` is absent, or outside the resolved repository root" → "…outside the resolved
      repository root and every member of its worktree family", moving its toml `condition` in the
      same edit; the row "An agent works in a **linked worktree** of this repository" — description
      only, whatever `worktree-changes` left there; the linked-worktree paragraph after tier 3;
      the launch flow's `--cwd` line; and the `agents::attribute` tested-modules bullet, which
      gains the worktree roots.
- [ ] 4.3 CHANGE: Rewrite in `AGENTS.md` (audience: every agent session) the sentence "every
      attribution tier is scoped to the resolved repository" to name member worktrees, since a
      future change touching scope would otherwise re-narrow it, and add `worktree-agents` to the
      "Current repo state" landed list. Net change under five lines.
- [ ] 4.4 CHANGE: In `tests/degraded-coverage.toml`, point the linked-worktree row's proof at
      `a_member_worktree_agent_badges_its_row` with `tier = "unit"` (it calls `rows()`, not a
      render) and a new `why`; re-measure `covers` for both it and the out-of-scope row, whose
      `src/agents.rs:137-143` range the new parameter shifts.
- [ ] 4.5 VERIFY: The 4.1 greps show the new wording; the toml shows the linked-worktree row's new
      proof; `make covers-check` — green.

## 5. Lint & Verify
<!-- kind: operational -->

- [ ] 5.1 CHECK: The tiers reached are unit and view (`cargo test --lib`) and the degraded-states
      contract (`make covers-check`); no gate script or doc-contract claim moves.
- [ ] 5.2 VERIFY: `make lint` — 0 warnings.
- [ ] 5.3 VERIFY: `make fmt-check` — clean.
- [ ] 5.4 VERIFY: `make gates` — every gate OK, `NODEFAULT-UI`'s `Launch` leg included.
- [ ] 5.5 VERIFY: `make test` — green.
- [ ] 5.6 VERIFY: `make coverage` — both floors met.
- [ ] 5.7 VERIFY: `openspec validate worktree-agents --strict` — valid.
- [ ] 5.8 VERIFY: `make check` as the single gate; if it fails, name the failing sub-command.
