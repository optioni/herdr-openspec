## Context

`src/open.rs` today ends at the open call: it lists panes, focuses an existing dashboard or
opens a new one, and returns. `--focus` is on the open argument vector for both placements,
and for a split that is enough. For a tab it is not — measured live against Herdr 0.9.0, the
new tab is created with its pane focused *inside* it while the workspace keeps the tab it was
already showing, so `OpenSpec: dashboard (tab)` from the action menu appears to do nothing.

Two further measurements, taken in this repository's own Herdr session before this design was
written, decide the shape of the fix:

- `herdr plugin pane focus <pane id>` **does** switch the active tab. Focusing pane `wG:p0`
  flipped tab `wG:tB` from `"focused":false` to `"focused":true` in the next `herdr tab list`.
  So no new Herdr subcommand is needed — `open.rs` already issues exactly this call on its
  already-open path, which is therefore already correct and is why the bug only shows on a
  *first* open.
- Pane creation is **synchronous** with respect to the listing API. `herdr pane split wJ:p1
  --direction right` returned `wJ:p2`, and `wJ:p2` was present in the very next `herdr pane
  list` (the pane was closed again immediately). One Herdr server serves both calls over one
  socket, so re-listing after an open is race-free and needs no retry, no sleep, and no clock
  — which matters, because a clock read is exactly what this project forbids on collaborator
  paths.

`herdr pane list` also already carries `tab_id` on every pane, which is what makes the
tempting alternative — read the open response, or call `herdr tab focus` — unnecessary.

## Goals / Non-Goals

**Goals:**

- `open-tab` raises the tab it creates, and `open` keeps behaving exactly as it does today.
- No new Herdr JSON envelope enters the crate; `pane list` stays the only payload parsed.
- Every post-open failure degrades to a warning. Having opened a pane and failed to raise it
  is a success with a caveat, never a failure.
- The three-part dashboard-pane test exists in exactly one place after this change, not two.

**Non-Goals:**

- `herdr tab focus`, `tab_id`, and the `tab list` payload. Pane focus is measured sufficient
  and adds no new shape.
- Reading the `plugin pane open` response.
- Changing any argument vector, `--focus` included.
- Retrying, polling, or waiting for the pane to appear — measured unnecessary above.
- Anything the dashboard renders once raised.

## Boundaries

| Piece | Module | Pattern it follows |
|---|---|---|
| `run`'s new post-open steps | `src/open.rs` | The existing list-then-focus sequence in the same function; still `&dyn cli::HerdrCli` only |
| `dashboard_panes` (new, pure) | `src/open.rs` | `existing_pane`'s own parse-and-match body, generalised from first-match to all-matches |
| `opened_pane` (new, pure) | `src/open.rs` | `split_target`/`placement_for` — pure, total, no I/O, unit-tested directly |
| `existing_pane` (re-expressed) | `src/open.rs` | Unchanged signature; body becomes `dashboard_panes(..)?.into_iter().next()` |

One caveat on the pattern cited above: `split_target` is production code (`src/open.rs:152`)
governed by **no** requirement — `grep -rn 'split_target' openspec/specs/ openspec/changes/`
matches only this file — and it substitutes a live pane from the listing where the live
requirement "`open` splits the pane the action was invoked from" still mandates the context's
own `focused_pane_id`. It is cited here only as a shape to copy (pure, total, no I/O), never as
a spec precedent, and closing that divergence is out of this change's scope.

No module is added and no module boundary moves. **No process spawn is added outside
`cli`** — `open.rs` reaches Herdr only through the `HerdrCli` trait object it already holds,
and issues two more calls through that same object. `src/open.rs` stays the fifth file on the
seam gates' `ALLOWED` list; the count does not change. No view is touched, so no I/O is added
to `src/ui/`, and `open` still starts no thread and so stays off `NOBLOCK`'s seam-module list.
The `Change` type is not altered, so `from_files`/`from_cli` agreement is not in question.

## Contracts

The one consumer is Herdr's action menu, which invokes the binary and reads its exit status
and stderr. The contract is **additive**: exit 0 keeps meaning "the dashboard is open", exit 1
keeps meaning "it is not", and the only new observable is that stderr may now carry a warning
on a path that previously carried none while still exiting 0 — which the existing warning
contract already permits ("A failed listing warns and still opens" does exactly this today).
No manifest, config, keybinding, or argument vector changes, so nothing is **BREAKING**.

## Persistence and Rollout

- migration: none · backfill: none · seeding: none · cache invalidation: none ·
  index rebuild: none · authorization: none (the plugin acts as the invoking user through
  Herdr's own socket, unchanged) · observability: two more lines may appear on stderr, which
  Herdr surfaces from a failed action · deployment: none beyond `make build`.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| The Herdr socket / `herdr` binary | replaced — a `#!/bin/sh` stub named `herdr` on a scratch `PATH`, recording its argv and answering `pane list` from a per-invocation counter | replaced — `cli::FakeCli`, which already queues per-argv responses and repeats the last |
| The `openspec` binary | not touched by this change — `open`/`open-tab` never invoke it | not touched |
| The filesystem | real — `ScratchDir` for the stub program and its `argv.log` | not touched; every new function is pure over strings |
| The terminal | not touched — `open`/`open-tab` never enter raw mode or construct a ratatui terminal, and this change adds nothing that could | not touched |
| The process environment | real in the acceptance test — `scrubbed()` plus explicit `HERDR_*` values on the child | replaced — the existing injected `&dyn Fn(&str) -> Option<String>` lookup |
| Wall-clock time | not touched — no sleep, retry, or `Instant::now()` is added, per Context | not touched |
| The repository tree itself (`SPEC.md`, `tests/degraded-coverage.toml`, `src/open.rs`) | real, read-only — `tests/degraded_coverage.rs` in group 5 parses the live files; it writes nothing | not touched |
| `make gates` / the `scripts/gates/` programs | real — task 3.4 shells out to the actual gate scripts over the actual tree | not touched |

`cli::FakeCli` needs **no change**: its responses are a `VecDeque` per argument vector whose
last entry repeats, so registering `["pane","list"]` twice gives the pre-open answer then the
post-open one. The scratch `herdr` stub does need one, because a shell script is stateless —
see Decision 4.

## Test Strategy

Everything sits in the fastest tier its dependencies allow. Twenty-three of the twenty-four
scenarios are unit tests in `src/open.rs`'s own `#[cfg(test)]` module over `cli::FakeCli`,
because every collaborator is already injected there. This change **does** take an outer-loop
acceptance test, in `tests/cli.rs` against the real built binary: the defect being fixed is
precisely a wiring assumption that held in every unit test and failed in the real product, so
one end-to-end proof that the focus call actually reaches `herdr` is the point.

Commands: `cargo test --all-features --lib open::tests` for the unit rows, `cargo test --test cli` for
the binary rows, `make check` for the whole gate set.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| A tab open is followed by a listing and a focus | `run` over `FakeCli`, asserting the four-call argv sequence | unit | Herdr replaced (`FakeCli`) | `cargo test --all-features --lib open::tests` |
| A tab open is followed by a listing and a focus | Built binary against the scratch `herdr` stub; `argv.log` shows `plugin pane focus w8:pG` | acceptance | Herdr replaced (shell stub), filesystem real, environment real | `cargo test --test cli` |
| A split open takes the identical path, with no placement branch | `run` with `Placement::Split`, same four-call assertion | unit | Herdr replaced | `cargo test --all-features --lib open::tests` |
| A failed second listing warns and still reports success | `FakeCli` queues `Ok(listing)` then `Err(CliError::Failed{code:Some(1),..})` for `pane list` | unit | Herdr replaced | `cargo test --all-features --lib open::tests` |
| An unparseable second listing warns and still reports success | Second `pane list` answers `not json`, then a no-`result.panes` payload | unit | Herdr replaced | `cargo test --all-features --lib open::tests` |
| An empty second listing warns rather than focusing nothing | Second `pane list` answers `{"result":{"panes":[]}}`; assert no `plugin pane focus` in the recorded argv | unit | Herdr replaced | `cargo test --all-features --lib open::tests` |
| A post-open focus failure warns, on every code, including the one that stops the pre-open focus | Loop over `Failed{code:Some(1)}`, `Failed{code:Some(2)}`, `NotStarted`; in each, assert `fake.calls()` records `["plugin","pane","focus","w8:pG"]`, `outcome` is `Ok`, and `warnings` carries that variant's reason. Asserting `outcome == Ok` alone stays green with the whole post-open step deleted | unit | Herdr replaced | `cargo test --all-features --lib open::tests` |
| A failed open makes no post-open call at all | `plugin pane open` errors; assert exactly two recorded calls | unit | Herdr replaced | `cargo test --all-features --lib open::tests` |
| The already-open path is untouched | Existing `open_focuses_an_existing_pane`-shaped test, two-call assertion unchanged | unit | Herdr replaced | `cargo test --all-features --lib open::tests` |
| A pane absent before and present after is the one chosen | `opened_pane(&[], &["w8:pG"])` | unit | none — pure | `cargo test --all-features --lib open::tests` |
| A pre-existing dashboard is not chosen when a new one appears | `opened_pane(&["w8:pG"], &["w8:pG","w8:pH"])` | unit | none — pure | `cargo test --all-features --lib open::tests` |
| Two new matches choose the first in post-open order | `opened_pane(&[], &["w8:pG","w8:pH"])` | unit | none — pure | `cargo test --all-features --lib open::tests` |
| Every match already present falls back to the first | `opened_pane(&["w8:pG"], &["w8:pG"])` | unit | none — pure | `cargo test --all-features --lib open::tests` |
| A pre-open id that has since vanished is ignored | `opened_pane(&["w8:pZ"], &["w8:pG"])` | unit | none — pure | `cargo test --all-features --lib open::tests` |
| No post-open match chooses nothing | `dashboard_panes` over the four non-matching listings, plus `opened_pane(.., &[])` | unit | none — pure | `cargo test --all-features --lib open::tests` |
| An unparseable listing is an error both matchers report identically | `dashboard_panes` and `existing_pane` over the same two bad payloads, comparing the reasons | unit | none — pure | `cargo test --all-features --lib open::tests` |
| A domain error is carried verbatim and stops the command | Existing test, unchanged | unit | Herdr replaced | `cargo test --all-features --lib open::tests` |
| A usage error on the open call is treated identically, without being parsed | Existing test, unchanged | unit | Herdr replaced | `cargo test --all-features --lib open::tests` |
| An unstartable `herdr` names the program | Existing test, unchanged | unit | Herdr replaced | `cargo test --all-features --lib open::tests` |
| A domain-error focus stops the command and opens nothing | Existing test, unchanged | unit | Herdr replaced | `cargo test --all-features --lib open::tests` |
| A usage-error focus warns and opens once | Existing test, call-count assertion rewritten to "first three calls are" plus the post-open pair | unit | Herdr replaced | `cargo test --all-features --lib open::tests` |
| A failed listing warns and still opens | Existing test, extended with the post-open focus that the empty pre-open id list produces | unit | Herdr replaced | `cargo test --all-features --lib open::tests` |
| An unparseable listing warns and still opens | Existing test, unchanged | unit | Herdr replaced | `cargo test --all-features --lib open::tests` |
| A successful open is silent | `run` with both listings answering and the focus succeeding; assert `warnings` is empty **and** `fake.calls()` holds exactly four calls whose fourth is the focus. The empty-`warnings` half alone passes at HEAD unchanged | unit | Herdr replaced | `cargo test --all-features --lib open::tests` |
| The post-open focus names the newly opened pane, not the one already listed | `run` with a pre-open listing carrying `w8:pG`, a focus failing at exit 2, and a post-open listing carrying `w8:pG` then `w8:pH`; assert the fifth call is `focus w8:pH`. The only row that falsifies a hardcoded empty `before` | unit | Herdr replaced | `cargo test --all-features --lib open::tests` |

Two rows of `SPEC.md`'s degraded-states table are added by this change (Decision 5), and the
contract tier binds each to a named passing test through `tests/degraded-coverage.toml` —
`tests/degraded_coverage.rs` fails `make check` otherwise.

## Decisions

**Decision 1 — Re-list rather than read the open response.** The `plugin pane open` response
carries `result.plugin_pane.pane.pane_id`, which would give the pane id directly and in one
fewer call. Rejected: `specs/pane-open/spec.md` already states that `open` parses only `pane
list`'s output, precisely so that envelope's shape cannot break this command, and that
envelope is measured to differ from `pane split`'s. A second `pane list` reuses a payload
shape the crate already parses and already degrades on. The cost is one extra socket call in a
one-shot process that then exits.

**Decision 2 — Focus the pane, not the tab.** `herdr tab focus <tab_id>` exists and works, and
`pane list` carries `tab_id`. Rejected in favour of `plugin pane focus`, which is measured to
switch the active tab anyway, is already the call `open.rs` makes on its other path, and needs
no new field read. One focus mechanism, not two.

**Decision 3 — One path for both placements.** Branching on `Placement` would skip the two
extra calls for a split, where `--focus` already suffices. Rejected: a placement branch is a
second path to test and a second place for the two to drift, the split case is idempotent
(focusing an already-focused pane is inert), and a split invoked while the workspace is
showing a *different* tab is then raised correctly too. The cost is two socket calls per
split invocation, in a process that exits immediately afterwards.

**Decision 4 — A second stub helper, not a counter-reset on the existing one.** `stub_herdr`
answers every `pane list` with `{"result":{"panes":[]}}`, which would make the post-open
listing find nothing and prove only that a second listing happened. Rather than teach that one
helper to sequence, the change adds a **separate** `stub_herdr_sequenced` whose `pane list`
answers empty on the first call of a process and with a labelled `w8` pane afterwards, keyed
on a counter file in its own scratch directory.

Two helpers rather than one with a reset, corrected during planning review: the only test that
runs two binary invocations against one scratch directory is
`main_routes_each_subcommand_to_its_own_placement`, and it must keep the plain stub — a
sequenced one would let `open-tab`'s *first* listing find `open`'s dashboard and take the focus
path, destroying the routing assertion that test exists for. The one new test drives a single
invocation. So no caller ever needs a reset, and building the affordance would leave dead code
for `cargo clippy -D warnings` to reject. The existing routing test keeps the plain stub and has
its `lines.len() == 4` assertion corrected to 6 (each subcommand now lists, opens, and lists
again, finding nothing).

**Decision 5 — Two new degraded-states rows, not one and not four.** The post-open surface
degrades in four ways (listing fails, listing unparseable, no pane identified, focus fails)
but in exactly two *kinds*: "could not work out which pane to raise" and "could not raise it".
`SPEC.md`'s table is a behaviour contract, not an enumeration of error variants, and its
existing `open` rows are already grouped this way ("fails or its payload is unparseable" is
one row today). Four rows would also mean four `degraded-coverage.toml` bindings asserting one
observable outcome between them.

**Decision 6 — Re-express `existing_pane` through the extractor rather than writing a second
matcher.** The post-open step needs every match, not the first. Copying the three-part test
would put the pre-open and post-open definitions of "this workspace's dashboard" in two places
that a later change could move apart. `existing_pane` keeps its signature and callers.

*Accepted deviation, recorded at Change Review.* `existing_pane` kept its signature but lost
its **production** call site: `run` needs the pre-open listing's whole id list for
`opened_pane` as well as its first element, and calling both matchers would parse the same
payload twice, so `run` calls `dashboard_panes` and takes `.first()` itself. `existing_pane`
remains as the named single-match accessor this change's Requirement-2 scenarios exercise by
name, so deleting it would require a spec edit. Recorded in its own doc comment.

**Decision 7 — The single matcher requires a non-empty `pane_id`, which closes a latent gap
rather than only avoiding a new one.** `existing_pane` accepts `"pane_id":""` today
(`src/open.rs:116` calls `as_str()` with no emptiness check), while the live
`specs/pane-open/spec.md` already promises "no focus call is made with an empty or non-string
pane id" — a promise with no proving fixture at HEAD. Decision 6 makes the extractor the one
matcher, so the guard belongs there and one test satisfies both the existing promise and the
post-open site that would otherwise issue `plugin pane focus ""`. This slightly widens the
change beyond the tab-focus defect; the alternative is knowingly routing a second call site
through a matcher whose own spec it violates.

## Risks / Trade-offs

- **A future Herdr makes `--focus` raise the tab, leaving this change issuing a redundant
  focus** → Harmless and inert; the extra call costs one socket round trip in a process that
  exits. The spec records the measurement and the Herdr version it was taken on, so a later
  change can remove it deliberately rather than discovering it.
- **The extra `pane list` widens the window in which another Herdr client closes the pane
  between the open and the focus** → The focus then fails with a domain error, which this
  change defines as a warning, and the process still exits 0. The pane genuinely is gone;
  reporting failure would be no more accurate.
- **Warning noise on a path that used to be silent** → Bounded: the warning fires only when a
  listing or focus actually fails, and Herdr surfaces action stderr only on demand. The
  acceptance stub's plain variant will emit one, which is why its assertions check exit status
  and argv rather than stderr emptiness.
- **`open-tab` and `open` still share the `OpenSpec` label, so each focuses the other's pane**
  → Unchanged by this design and deliberate; `specs/pane-open/spec.md` already records it as
  intended ("show me the dashboard" has one answer per workspace).

## Migration Plan

None needed. The change is confined to one module's behaviour inside a one-shot subcommand,
with no persisted state, no schema, and no format. Rollback is reverting the commit; a stale
binary and a new one differ only in whether the tab is raised. Deploy order is `make build`,
which the plugin's `[[build]]` step already runs.

## Visual Design

Not applicable: this change is non-visual. `open`/`open-tab` render nothing and never
construct a terminal, and no view file is touched.

## Open Questions

None.
