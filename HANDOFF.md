# Handoff — Phase 1, paused after `repo-foundation` planning

Paused 2026-09-04 ~00:35 EEST because the 5-hour session limit was exhausted
(96% used, resetting 2026-09-03T22:30Z / 01:30 EEST). Working tree clean.

## State

| Change | State |
|---|---|
| `repo-foundation` | Artifacts complete, `--strict` valid, committed. **Not implemented.** |
| `ci-pipeline` | Untouched — no artifacts |
| `plugin-config` | Untouched — no artifacts |

## Next action

Run the apply phase for `repo-foundation`. Artifacts already exist at
`openspec/changes/repo-foundation/`, so **skip `ff-change`** and start at apply.
`tasks.md` is the live checklist: 8 groups, all unchecked.

Then `ci-pipeline` and `plugin-config` (both depend only on `repo-foundation`,
so either order) need the full ff → apply → archive loop.

## Environment

The `openspec` CLI is under nvm and not on the default PATH. Every shell needs:

```bash
export PATH="$HOME/.nvm/versions/node/v24.20.0/bin:$PATH"
```

One-time setup `repo-foundation`'s apply phase requires, neither yet installed:
`rustup component add clippy` and `cargo install cargo-llvm-cov`.

## Budget note

`ff-change` alone for this Phase 1 change cost ~25% of a 5-hour session — it
spent three rounds of reviewer subagents, two of which died on `529 Overloaded`
and lost their reports. Assume a full ff → apply → archive loop needs 40–50%,
and do not start a change below roughly half a session remaining.

## Deferred, already scheduled as tasks in `repo-foundation/tasks.md`

- `AGENTS.md` → "Current repo state" claims no `Cargo.toml` exists.
- `README.md` → Install advertises action-menu entries that only arrive with
  Phase 6's `plugin-actions`.

Deferred to archive time per `openspec/config.yaml` → `operations.archive`:
`IMPLEMENTATION-ORDER.md` Phase 6 credits `plugin-actions` with
`min_herdr_version` and `platforms`, which `repo-foundation` actually ships.
Its prose also still names a `test-plan` artifact the `tdd` schema does not
have — the artifacts are `proposal → specs → design → tasks → planning-review`.
