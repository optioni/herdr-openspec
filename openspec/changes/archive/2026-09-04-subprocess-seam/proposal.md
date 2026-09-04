## Why

Everything the dashboard needs from the OpenSpec CLI and from Herdr arrives through a
process spawn, and `SPEC.md` → Architecture makes `cli` the only module in this crate
permitted to perform one. Nothing has crossed that boundary yet, so the boundary does
not exist: `resolve::openspec_bin`'s fourth probe step ships as an injected hook whose
binding returns nothing, and `changes-from-cli`, `agent-polling`, and `agent-launch` all
queue behind a seam that has never been built. Building it now — before the first caller
— is what keeps the untestable residue to a few lines and keeps every later change's
tests free of a real `openspec` or `herdr` binary.

## What Changes

- Add `src/cli.rs`: the `OpenspecCli` and `HerdrCli` traits, one real spawn-and-return-
  stdout implementation each, a shared error type, and the recording fake every later
  change's tests use. No parsing, merging, or decision-making lives in this module.
- Add the real `npm prefix -g` probe behind the same seam, reading **stdout only,
  trimmed**, and treating a spawn failure, a non-zero exit, or empty output as no prefix.
- **BREAKING (internal)**: remove `resolve::npm_prefix_deferred` and bind
  `resolve::openspec_bin_from_env`'s fourth step to the real probe in `cli`. The test
  pinning today's empty result is expected to go red at that moment; that is the
  hand-over signal `repo-resolution` planted, and it is replaced rather than adjusted.
- Prove the seam end to end: a scenario drives the real spawn through to
  `<prefix>/bin/openspec`, a join exercised only by fixture closures until now.
- Replace `plugin-config`'s program-name-literal no-spawn check, which false-positived on
  legitimate `.join("herdr")` path code and was recorded as not-run, with an
  architectural check that distinguishes a spawn from a path join and cannot pass on an
  empty or missing input.
- Correct `SPEC.md` where this change proves it wrong (the trait signatures, and the
  paragraphs describing step 4 as unwired) and `AGENTS.md`'s repo-state paragraph.

## Non-Goals

- **No parsing.** `openspec list --json`, `herdr agent list`, and `contextFiles` belong
  to `changes-from-cli`, `agent-polling`, and `agent-launch`. This change returns stdout.
- **No caller.** Nothing in the crate calls `OpenspecCli` or `HerdrCli` after this
  change except the tests that prove them; `resolve`'s npm hook is the one real binding.
- **No dashboard, no view, no async.** No worker thread, no polling loop, no debounce —
  `live-refresh` and `agent-polling` own those.
- **No `herdr` binary resolution chain.** `openspec` has one because `plugin-config` and
  `repo-resolution` built it; `herdr` gets a program path supplied by its constructor.
- No new dependency, no manifest change, no config-format change, no keybinding.
- Untouched PRD non-goals: nothing here edits OpenSpec files, orchestrates across
  changes, authors a change, or assumes Windows.

## Capabilities

### New Capabilities

- `subprocess-seam`: the two traits and their contract, the shared error type, the real
  spawn implementations, the recording fake, the `npm prefix -g` probe, and the
  architectural rule that `cli` is the only module in the crate that spawns a process.

### Modified Capabilities

- `openspec-binary`: the requirement "The `npm prefix -g` step is an injected hook,
  deferred until the subprocess seam exists" is superseded — the hook stays injected, but
  its production binding now runs the real probe, and the scenario pinning the empty
  result is replaced by one driving a real spawn through to `<prefix>/bin/openspec`. The
  module-scoped claim that `src/resolve.rs` names no process API stays normative and
  stays true; the tree-wide grep this change was always going to rescope moves into
  `subprocess-seam` as an excluded-module check.

## Impact

- **Roadmap row:** Phase 3, `subprocess-seam` — `openspec/IMPLEMENTATION-ORDER.md`.
  Depends on `repo-foundation` and `repo-resolution`, both landed. Unblocks
  `changes-from-cli` and `agent-polling`.
- **Code:** new `src/cli.rs`; `src/lib.rs` gains `pub mod cli;`; `src/resolve.rs` loses
  `npm_prefix_deferred` and rebinds `openspec_bin_from_env`; `src/resolve.rs`'s group-5
  tests lose the pinning test and gain its successor.
- **Docs:** `SPEC.md` (Architecture → The subprocess seam; Data layer → Resolution chain,
  step 4; Testing and quality gates), `AGENTS.md` (Current repo state).
- **Not affected:** `Cargo.toml`, `herdr-plugin.toml`, `Makefile`, `.github/workflows/`,
  `config.toml`'s format, and every file under `openspec/` other than this change's own.
