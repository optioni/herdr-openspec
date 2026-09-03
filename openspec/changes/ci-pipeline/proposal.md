## Why

`make check` exists and passes, but nothing runs it except a human who remembers to.
A change can land on `main` unformatted, with a denied clippy lint, with a failing
test, or below the coverage floor, and nobody finds out until the next person runs the
gate locally — on one platform, the one they happen to use. macOS-only and Linux-only
breakage is the specific risk: the plugin targets both, and `scripts/build.sh` already
exists precisely because the two environments differ.

This is Phase 1, row `ci-pipeline` of `openspec/IMPLEMENTATION-ORDER.md`. It depends on
`repo-foundation`, which is implemented and archived.

## What Changes

- **`.github/workflows/ci.yml`.** One workflow, three jobs: a `check` job matrixed over
  `ubuntu-latest` and `macos-latest` running the format, lint, and test gates; a
  `coverage` job on Linux only; and a `ci` job that aggregates both into one stable
  status name for branch protection to require.
- **CI invokes `make`, never a raw cargo command.** `make fmt-check`, `make lint`,
  `make test`, `make coverage`. The command strings stay defined exactly once, in the
  `Makefile`, which is what makes SPEC.md's "local and CI cannot diverge" true rather
  than aspirational. CI does not call the composite `make check`, because coverage runs
  on only one of the two runners.
- **A parity test, `tests/ci_workflow.rs`.** Standard-library only, and scoped
  deliberately: it guards the values whose wrong version leaves CI green with a gate
  silently absent — a gate command restated instead of invoked through `make`, an `env:`
  mapping that neuters one, a `continue-on-error`, a `paths-ignore` that stops the
  workflow running at all, a deleted coverage step, a second workflow file, a job left
  out of the aggregate check. Values that fail loudly — a missing toolchain component,
  a missing `cargo-llvm-cov`, a bad cache key — are deliberately not asserted. Without
  the guard, the divergence this change exists to prevent could be reintroduced
  silently by a one-line workflow edit.
- **Explicit toolchain.** `dtolnay/rust-toolchain@stable` with the components each job
  needs, rather than whatever Rust the runner image happens to ship.
- **`Swatinem/rust-cache` per job**, with the coverage job on its own cache key because
  instrumented builds do not share artifacts with ordinary ones.
- **SPEC.md correction, and a matching delta on `quality-gates`.** SPEC.md → Testing and
  quality gates says the gates are enforced "behind a single `make check` target" and,
  later in the same section, that coverage "runs once, on Linux". Both cannot be true:
  `check` composes coverage. The sentence is corrected as part of this change and the
  correction logged in `planning-review.md`. The live `quality-gates` spec carries the
  same rationale clause word for word, so it gets a MODIFIED delta rather than being
  left to contradict the corrected SPEC.md.
- **Docs:** a CI badge and `.github/workflows/ci.yml` added to the file lists in
  `README.md` and `AGENTS.md`. Not **BREAKING** — no manifest key, config key, or
  keybinding is touched.

## Non-Goals

- **No new `Makefile` target and no change to the four gate commands.** CI adapts to the
  Makefile; the Makefile does not adapt to CI. The 80 in `--fail-under-lines 80` is not
  lowered, duplicated into the workflow, or overridden per platform.
- **No branch-protection configuration.** That is a GitHub repository setting, not a
  file in the tree. This change makes one stable status name available to require; a
  human turns it on.
- **No release, publish, tag, or registry-submission job.** No job holds a token beyond
  a read-only `contents: read`, and no secret is referenced.
- **No MSRV job, no beta/nightly job, no Windows runner, no cross-compilation.**
- **No third-party crate**, dev-dependency included: `plugin-build` requires
  `Cargo.lock` to hold exactly one package, so the parity test parses with `std`.
- **No dependency-update bot and no commit-SHA action pinning** — see design.md →
  Decisions for the argument.
- **Nothing that writes inside `openspec/`**; no change authoring, no orchestration
  across changes, no Windows support.

## Capabilities

### New Capabilities

- `ci-workflow`: the GitHub Actions workflow — when it runs, on which runners, which
  gates it invokes and through what, where coverage runs, and how a gate that cannot run
  is required to fail rather than be skipped.

### Modified Capabilities

- `quality-gates`: its first requirement justifies composing `check` from the four
  targets "so that no gate is defined twice and local runs and CI invoke identical
  commands" — the same claim SPEC.md is being corrected for, and one that stops being
  literally true here: CI never invokes `make check`, and never invokes `make coverage`
  on macOS. The delta keeps every SHALL, the command table, and all three scenarios
  unchanged, and rewrites only that rationale clause to say what is actually true —
  `check` is the single local entry point, CI invokes the same targets individually, and
  every command is still written in exactly one place.

## Impact

- **New files:** `.github/workflows/ci.yml`, `tests/ci_workflow.rs`.
- **Modified:** `SPEC.md` → Testing and quality gates → Gates (the contradiction above,
  plus a note that CI obtains `clippy` and `cargo-llvm-cov` from actions rather than the
  two local one-time setup steps); `README.md` (status badge); `AGENTS.md` (the "Quality
  gates" sentence, and the important-files list).
- **Unmodified on purpose:** `Makefile`, `Cargo.toml`, `Cargo.lock`, `src/`,
  `herdr-plugin.toml`, `scripts/build.sh`.
- **Roadmap:** Phase 1, row `ci-pipeline`. Depends on `repo-foundation`; blocks nothing —
  `plugin-config` and every Phase 2 change depend on `repo-foundation`, not on this.
- **External services:** GitHub Actions, and four marketplace actions —
  `actions/checkout`, `dtolnay/rust-toolchain`, `Swatinem/rust-cache`,
  `taiki-e/install-action`. Runner minutes are the only cost.
- **No data model, background job, sibling repository, or deployment manifest is
  affected.** No production code path changes; the crate's behaviour is identical
  before and after.
