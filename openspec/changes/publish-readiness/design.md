## Context

The plugin is built and the repository is public, but its outward surface was never written
for an outside reader (see `proposal.md` → Why). Four facts constrain how it can be fixed:

- **`README.md` is already partly contract-bound.** `tests/manifest.rs` requires both action
  titles (`OpenSpec: dashboard`, `OpenSpec: dashboard (tab)`) to appear, and forbids the old
  deferral sentence. `tests/doc_contract.rs` requires the `## Keys` table's key atoms to equal
  `ui::help::INVENTORY`, and requires `## Development` to name `Cargo.toml`'s `rust-version` and
  every external program the `make check` path invokes (`python3`, `git`, and the two
  one-time components). A rewrite may reword and reorder freely, provided those headings and
  facts survive.
- **The project rule:** a documented claim with a computable second site is checked inside
  `cargo test`. The licence and the repository slug both become such claims here.
- **`tests/gate_controls.rs` copies the whole tree** (excluding `.git` and `target`) once per
  planted control, more than sixty times per suite run. Every committed byte is paid for many
  times over in every `make check`, which is why the demo image carries a size cap.
- **The file loader resolves a schema only from the repository's own `openspec/schemas/`**
  when no `SchemaLocations` is supplied. A demo that names a bundled schema (`spec-driven`)
  without vendoring it would carry a problem in file mode.

## Goals / Non-Goals

**Goals:** a licensed repository; a README that leads with what the plugin does and shows it
working; a reproducible demo recording; a root free of session records, without losing the
deferred work they hold; and the outward steps prepared for the user to run.

**Non-Goals:** everything in `proposal.md` → Non-Goals, which covers release binaries, Herdr's
registry, `worktree-agents`, fixing the known issues, any dashboard behaviour, and every
outward action.

## Boundaries

- **`src/`: untouched.** No module changes, no process spawn is added anywhere, and the
  `Change` type is not altered, so `from_files`/`from_cli` agreement is unaffected.
- **`tests/publish_surface.rs` (new)** is the fourth member of the contract tier, beside
  `tests/manifest.rs`, `tests/degraded_coverage.rs`, and `tests/doc_contract.rs`. It follows
  `doc_contract.rs`'s shape: each claim is a **pure checker** over strings or bytes, returning
  `Result<(), Vec<String>>` whose messages name each disagreeing site. One test runs each
  checker on the real files, and further tests feed it planted inputs, one per failure
  scenario. It names no spawn API (`NOSPAWN-GREP` sweeps `tests/`), and constructs no
  `Change`/`ChangeSet` literal (`NOLIT-CHANGE`), reading those values by field access only.
  It parses TOML with the existing `toml` dev-path dependency, so no dependency is added.
- **The demo check** calls `herdr_openspec::changes::from_files`, the crate's public, total,
  read-only loader, exactly as the dashboard does in file mode.
- **`docs/demo/`** is input data only. Nothing in the crate reads it at runtime, and the
  `OPENSPEC-UNTOUCHED` gate is unaffected, because it sweeps `:(top)openspec/` alone.

## Contracts

- **README → install slug and Cargo `repository`:** additive. They are now bound to each other.
- **`Cargo.toml` `[package]`:** gains `license`, `description`, and `repository`. Additive. Nothing
  consumes the crate as a library outside this repository.
- **Plugin manifest, config format, keybindings:** unchanged. Not breaking.

## Persistence and Rollout

Migration: none. Backfill: none. Seeding: none. Cache invalidation: none. Index rebuild:
none. Authorization: none. Observability: none. Deployment: none. The user pushes to `main`
after review, and the showcase PR and repository metadata are user actions performed after
that.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| Filesystem (repository files: `LICENSE`, `Cargo.toml`, `README.md`, `docs/`) | real, read-only | replaced by in-memory strings and bytes fed to the pure checkers |
| Filesystem (demo fixture `docs/demo/`) | real, read-only through `changes::from_files` | a scratch copy under `env!("CARGO_TARGET_TMPDIR")`, mutated per scenario, never the tracked fixture |
| `openspec` binary | not used: the demo check is file mode by construction, with no `SchemaLocations` | not used |
| Herdr socket | not used | not used |
| Terminal | not used: no test renders or enters raw mode | not used |
| `vhs` | not used: an author-only tool, run by hand to render `docs/demo.gif` | not used |
| `git` | not used | not used |
| GitHub (`gh repo edit`, showcase PR) | not used: prepared as text, performed by the user | not used |

## Test Strategy

The outer-loop acceptance test is the real-file run of all four checkers. It is written
first and goes RED for real reasons: there is no `LICENSE`, no Cargo `license`/`repository`,
no image, and no demo. Each failure scenario is then driven by a planted input. Everything
runs in `cargo test`, inside `make check`'s Test gate. The known-issues requirement is the one
exception: it is verified at review time (see its spec text).

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| All three sites agree | licence checker over the real files | contract | real fs, read-only | `cargo test --test publish_surface licence` |
| The LICENSE file is missing | checker fed `None` for `LICENSE`; error names `LICENSE` | contract | in-memory | `cargo test --test publish_surface licence` |
| Cargo.toml declares no licence | checker fed TOML without `license`; error names it | contract | in-memory | `cargo test --test publish_surface licence` |
| The README names a different licence | README section reading `Apache-2.0`; error names both | contract | in-memory | `cargo test --test publish_surface licence` |
| The README has no Licence section | README with no `## Licence`; error names the section | contract | in-memory | `cargo test --test publish_surface licence` |
| The slugs agree | slug checker over the real files | contract | real fs, read-only | `cargo test --test publish_surface repository` |
| The repository moved but the README did not | mismatched slugs; error names both | contract | in-memory | `cargo test --test publish_surface repository` |
| Metadata is absent | three plants (no `description`, empty `description`, no `repository`) | contract | in-memory | `cargo test --test publish_surface repository` |
| The image, the file, and the tape agree | image checker over the real README, GIF bytes, and tape | contract | real fs, read-only | `cargo test --test publish_surface demo_image` |
| The image is referenced but was never rendered | checker fed `None` for the file bytes | contract | in-memory | `cargo test --test publish_surface demo_image` |
| The file is not a GIF | two plants: empty bytes, PNG signature | contract | in-memory | `cargo test --test publish_surface demo_image` |
| The image outgrows its budget | `GIF89a` header plus padding to 2 MiB + 1 byte | contract | in-memory | `cargo test --test publish_surface demo_image` |
| The tape renders somewhere else | tape `Output demo.gif` against README `docs/demo.gif` | contract | in-memory | `cargo test --test publish_surface demo_image` |
| The README carries no image | two plants: no image; image below `## Install` | contract | in-memory | `cargo test --test publish_surface demo_image` |
| The demo loads with no problems | `from_files(docs/demo, Full)` plus the four assertions | contract | real fs, read-only | `cargo test --test publish_surface demo_repo` |
| The demo's schema is removed | scratch copy with `openspec/schemas/` deleted; the check reports `from_files`' own problem | contract | scratch fs | `cargo test --test publish_surface demo_repo` |
| A demo change loses its tasks file | scratch copy, one active change's `tasks.md` deleted | contract | scratch fs | `cargo test --test publish_surface demo_repo` |
| The demo has no partial progress to show | scratch copy, every `- [ ]` in active changes rewritten to `- [x]` | contract | scratch fs | `cargo test --test publish_surface demo_repo` |
| The fixture is left untouched | path+bytes digest of `docs/demo/` and top-level `openspec/` taken before and after the real-file demo check | contract | real fs, read-only | `cargo test --test publish_surface demo_repo` |
| The audit's three deferred items are carried over | review: each item re-verified open at base (`grep` for `fn columns` in `src/ui/layout.rs`; `src/launch.rs` absent from `nosleep.sh`; the three test names exist) and present in `docs/known-issues.md`; `HANDOFF-AUDIT.md` absent | review | real fs | `rg -n 'truncate_columns\|nosleep\|g_focuses_the_agent' docs/known-issues.md` |
| A resolved item is not resurrected | review: `NIGHT.md`'s items (the two SPEC.md-documented limitations, and the `openspec-schemas` fix, which is in tag `v0.6.0`) are verified resolved and absent from `docs/known-issues.md` | review | real fs | `git -C ~/Code/openspec-schemas tag --contains 7e31a8f` |

## Decisions

1. **One new test file rather than more claims in `doc_contract.rs`.** Adding claims there moves
   `CLAIM_COUNT`, `CLAIM_COUNT_WORDS`, and `AGENTS.md`'s "seventeen further claims" in one
   commit, and `worktree-agents` (in flight) may move the same counters. A separate file keeps
   the two changes from colliding. *Alternative:* fold into `tests/manifest.rs`. Rejected,
   because its `plugin-manifest` spec is about `herdr-plugin.toml`, and the licence is not
   part of it.
2. **The demo vendors a copy of OpenSpec's bundled `spec-driven` `schema.yaml`** under
   `docs/demo/openspec/schemas/spec-driven/`, copied from the installed package
   (`@fission-ai/openspec` 1.13.2 → `schemas/spec-driven/schema.yaml`). A header comment names
   the source and OpenSpec's MIT licence, as that licence requires. That makes the demo load
   problem-free in file mode, and it shows the stock schema most showcase readers use. *Alternative:* vendor `tdd`.
   Rejected, because it is a graft-managed copy, and a second, untracked copy would drift
   silently. *Alternative:* a hand-written toy schema. Rejected, because it would not look
   like a real OpenSpec project.
3. **The demo is a small fictional project** (a todo CLI), with at least two active changes
   (one mid-way, one just proposed) and one archived, complete change. That gives the gauge,
   the sections, and the archived fold something real to show.
4. **The recording runs outside Herdr,** so it shows no agent badges, and the action keys are
   hidden while the socket is unreachable (`README.md` → Keys already documents this). The
   README describes attribution and launching in text. A still screenshot taken inside Herdr
   with live agents is a possible later addition, not part of this change. *Alternative:*
   script Herdr inside `vhs`. Rejected as fragile and non-reproducible.
5. **No counts in the README.** "Built with OpenSpec" points at `openspec/specs/` and
   `openspec/changes/archive/` rather than stating "55 capabilities" or "21 changes". A count
   is a claim with a second site that would need binding, and it would go stale with every
   change.
6. **Delete all three handoff records,** reversing `doc-conformance`'s keep decision for
   `HANDOFF-AUDIT.md`, on the user's call. The three still-open deferred items move to
   `docs/known-issues.md` first, and everything else stays in git history.
7. **Historical citations stay.** `quality-gates` and `dashboard-loop` cite `HANDOFF.md` as the
   place something *was* recorded, and so does one `tests/manifest.rs` comment. Those are true
   statements about the past. Rewording them would cost two MODIFIED deltas, each carrying a
   whole oversized requirement, which is the revert risk the schema warns about, and it
   would buy nothing.
8. **The image cap is 2 MiB.** The `vhs` output is tuned (dimensions, framerate, a short script)
   to fit. *Alternative:* store the image outside the tree. Rejected, because the README must
   render on GitHub and on the showcase without an external host.
9. **Outward steps are text, not actions.** `openspec/config.yaml` forbids pushing and opening
   PRs. The final task writes the `gh repo edit` command and the showcase line and PR body
   into this change's directory, for the user to run.

## Risks / Trade-offs

- [A new `tests/*.rs` file moves a gate's scanned-file count] → Checked at planning time:
  `LAUNCHSEAM`'s file count is a floor (`[ "$n" -ge "$MIN" ]`, `scripts/gates/launchseam.sh:102`),
  and `grep -ln '"$n" -eq' scripts/gates/*.sh` matches 0 files. Planning review re-confirms
  this for every gate.
- [`vhs` output exceeds 2 MiB] → Shrink the dimensions, framerate, and script length. The cap
  is enforced, so this cannot slip through.
- [The README rewrite breaks a bound fact] → The existing contract tests catch it in the same
  `make check`, and the rewrite task runs `cargo test --test doc_contract --test manifest`
  before committing.
- [The GIF goes stale as the UI evolves] → It is reproducible from `docs/demo/demo.tape`. The
  demo-repo check keeps the fixture loading, but it cannot detect visual staleness. That is
  accepted.
- [The new spec's `## Purpose` is a placeholder at archive time and `tests/spec_purposes.rs`
  fails] → The archive step writes the purpose, as every prior new capability did.

## Migration Plan

None needed. The change adds and deletes documentation and test files and changes no runtime
behaviour. Rollback is `git revert`.

## Open Questions

None blocking. Two inputs come from the user during apply: running `vhs` once to render
`docs/demo.gif` (`brew install vhs`), and choosing the GitHub topics (a default list is
proposed in the hand-off text).

## Visual Design

There is no design source. The only visual is the recorded GIF of the existing UI.
