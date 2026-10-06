## Why

The repository is public and the plugin is feature-complete, but nothing about its front
door is written for a stranger. It is about to be listed in OpenSpec's community showcase
(`Fission-AI/OpenSpec` → `docs/community.md`), and a reader arriving from that link today
finds no screenshot of a TUI, a README that opens on internal roadmap status ("Status:
complete. All 21 changes…"), three session-handoff records at the repository root, and a
README that says "Licence: MIT" while no `LICENSE` file exists and `Cargo.toml` declares no
`license` — so nobody is actually licensed to use the code. `SPEC.md` → Build and
distribution also says prebuilt binaries arrive "before publishing", which this change
deliberately does not do.

This is unplanned post-roadmap work: `openspec/IMPLEMENTATION-ORDER.md` ends at the plugin
being built and never anticipated presenting it to outside readers.

## What Changes

- **Licence.** Add an MIT `LICENSE` file at the root, declare `license = "MIT"` (with
  `description` and `repository`) in `Cargo.toml`, and bind the three sites — `LICENSE`,
  `Cargo.toml`, and `README.md` → Licence — to each other inside `cargo test`, per this
  project's rule that a claim with a computable second site is checked rather than re-read.
- **Demo.** Add a small, self-contained demo OpenSpec repository under `docs/demo/` (a few
  active changes at different progress points, an archived one, specs) and a checked-in
  `vhs` tape that drives the dashboard against it and renders the README's animated
  screenshot. The rendered GIF is committed; the tape makes it reproducible when the UI
  changes. A contract test proves the demo still loads through the crate's own file loader,
  so the fixture cannot rot silently.
- **README rewrite.** Restructure `README.md` for a first-time reader: a one-line hook, the
  demo GIF, what you get, requirements, a quick start, the Keys table in plainer words, the
  configuration table, a short factual "Built with OpenSpec" section, and Development. The
  `## Keys` and `## Development` sections and the action titles that `tests/doc_contract.rs`
  and `tests/manifest.rs` already bind keep their headings and their bound facts.
- **Root cleanup.** Delete `HANDOFF.md`, `HANDOFF-AUDIT.md`, and `NIGHT.md`. Git history
  keeps them. **This reverses `doc-conformance`'s decision to keep `HANDOFF-AUDIT.md`.** That
  decision rested on three deferred items recorded nowhere else (the display-width
  primitives' placement, the `nosleep.sh` gaps, three load-sensitive tests), all still open,
  so they are moved to `docs/known-issues.md` before the file goes.
- **SPEC.md correction.** Build and distribution stops calling the prebuilt-binary fast path
  a precondition of publishing; it becomes planned future work.
- **Publication hand-off.** Prepare, not perform, the outward steps: the GitHub repository
  description and topics as a ready-to-run `gh repo edit` command, and the one-line
  `docs/community.md` entry plus pull request text. The user pushes and opens the PR.

No change touches the plugin manifest, the configuration format, any keybinding, or any code
under `src/`. Nothing here is **BREAKING**.

## Non-Goals

- **Prebuilt release binaries** and the `scripts/build.sh` fast path. That needs its own change.
  Until then the README states plainly that installing needs a Rust toolchain.
- **Herdr's plugin registry** (`assets.herdr.dev/plugins/index.json`). It is a separate
  submission, best made once binaries exist.
- **`worktree-agents`**, still in flight. The README stops making claims ("complete", a change
  count) that its landing would falsify.
- **Fixing the three known issues.** This change only moves them somewhere durable.
- **Any dashboard behaviour.** No rendered state, key, or degraded path changes, so the PRD
  non-goals (no editing of OpenSpec files, no orchestration, no authoring, no Windows) are
  untouched. The demo fixture is read-only input. The plugin writes nothing new anywhere.
- **Pushing, opening pull requests, or editing the GitHub repository.** These are outward
  actions the user performs.

## Capabilities

### New Capabilities

- `publish-surface`: the repository's outward-facing surface and the checks binding it.
  `LICENSE`, `Cargo.toml`'s `license`, and `README.md` → Licence agree. The README embeds a
  demo image that exists, and the tape renders to that same path. The demo repository loads
  through `changes::from_files` with at least one active and one archived change. It
  deliberately does not assert that the root has no handoff records, because the
  overnight-session workflow writes `NIGHT.md` there on purpose, and that would turn
  `make check` red mid-session.

### Modified Capabilities

None. `doc-conformance` and `plugin-manifest` keep their requirements unchanged. The README
rewrite must preserve what they already bind, which is a constraint on the rewrite, not a new
requirement.

## Impact

- **New files:** `LICENSE`, `docs/demo/` (fixture repository and `demo.tape`), the rendered
  `docs/demo.gif`, `docs/known-issues.md`, and `tests/publish_surface.rs`.
- **Edited:** `README.md`, `Cargo.toml` (package metadata only, no dependency change),
  `SPEC.md` → Build and distribution, and `AGENTS.md` (the contract tier gains a fourth
  member).
- **Deleted:** `HANDOFF.md`, `HANDOFF-AUDIT.md`, `NIGHT.md`. Historical citations of
  `HANDOFF.md` in archived specs (`quality-gates`, `dashboard-loop`) and in one
  `tests/manifest.rs` comment describe what was true when they were written, and stay.
- **Tooling:** `vhs` (Charm) is needed once, on the author's machine, to render the GIF. It is
  not a build, test, or CI dependency.
- **External:** the `Fission-AI/OpenSpec` showcase PR and the GitHub repository metadata, both
  performed by the user.
