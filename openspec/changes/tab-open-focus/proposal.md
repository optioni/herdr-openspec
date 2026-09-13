## Why

`herdr-openspec open-tab` opens the dashboard in a tab that stays in the **background**:
the user invokes the action from Herdr's menu and nothing visibly happens. Measured live
against Herdr 0.9.0, `herdr plugin pane open --placement tab --focus` creates the tab and
focuses the pane *within* it, but does not make that tab the workspace's active tab. The
same measurement shows `herdr plugin pane focus <pane_id>` **does** switch the active tab —
focusing pane `wG:p0` flipped tab `wG:tB` from `focused: false` to `focused: true`. So the
already-open path in `open::run` is correct today and only the newly-opened path is wrong.

Unplanned work past Phase 6: a defect in `plugin-actions`' shipped behaviour, which the
roadmap could not anticipate because `--focus` was reasonably assumed to focus the tab it
creates.

## What Changes

- After a **successful** `plugin pane open`, `open::run` re-issues `pane list` and focuses
  the pane it just opened, using the `focus_args` call it already has.
- The just-opened pane is identified by **difference**: the dashboard pane ids present in
  the pre-open listing are remembered, and the pane matched afterwards that is not among
  them is the new one. A first-match fallback covers the case where no id is new.
- Applies to **both** placements, with no placement branch — a split that already takes
  focus is unharmed by a second focus, and one code path is cheaper to spec and test than
  two.
- Every step after the successful open degrades to a **warning**: a failed or unparseable
  re-listing, no matched pane, or a failed focus never turns a successful open into a
  failure.
- Not **BREAKING**: no manifest, config, or keybinding change.

## Non-Goals

- Reading the `plugin pane open` **response**. The standing rule that `open` parses only
  `pane list`'s output is preserved deliberately — no new JSON envelope enters the crate.
- Using `herdr tab focus`, or reading `tab_id` at all. Pane focus is measured sufficient.
- Any change to `open`/`open-tab` argument vectors, including `--focus`, which stays.
- Anything the opened dashboard then renders.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `pane-open`: the open path gains a mandated focus-after-open step, and the "a successful
  open is silent" and warning-degradation requirements extend to cover it.

## Impact

- `src/open.rs` — `run`, plus two new pure functions for the new-pane decision.
- `tests/cli.rs` — the scratch `herdr` stub gains a per-invocation counter, and one
  acceptance test is added; the existing routing test's call count is corrected.
- `SPEC.md` — two degraded-states rows, and the § Herdr integration prose that currently
  implies `--focus` is sufficient.
- `tests/degraded-coverage.toml` — the contract-tier binding for those two rows.
  `tests/degraded_coverage.rs` runs inside `make check`, so this is not incidental churn.
- `AGENTS.md` — one clause on why the open path re-lists rather than reading the response.
- `openspec/specs/pane-open/spec.md` — via this change's delta.
- No dependency, manifest, or gate-script change.
