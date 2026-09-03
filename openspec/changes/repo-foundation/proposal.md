## Why

The repository is documentation only — there is no `Cargo.toml`, no build, and no
quality gates. Nothing else on the roadmap can be written, tested, or seen running
until a crate exists, `make check` enforces the four gates, and Herdr can link the
working tree. This is the first row of Phase 1 and every other change depends on it.

## What Changes

- **Cargo scaffold.** Package and binary named `herdr-openspec`, edition 2024, with a
  library holding the logic and a thin `main`. No third-party dependencies yet.
- **The `ui` subcommand.** Prints a placeholder banner and holds the process open
  until stdin reaches EOF, so a linked pane stays visible. Any other argument list —
  unknown subcommand, no subcommand, or `ui` with a trailing argument — prints usage on
  stderr and exits 2, which is what lets `plugin-actions` add `open` safely later. The
  real dashboard lands in `tui-shell`.
- **`rustfmt.toml`** at the repository root, declaring the crate's edition so a bare
  `rustfmt` agrees with `cargo fmt`.
- **`Makefile`** with `fmt`, `fmt-check`, `lint`, `test`, `coverage`, and `build`
  targets, and a `check` target composing the four gates in order, so local and CI
  cannot diverge. The lint and coverage targets fail with a message naming the one-time
  install when `clippy` or `cargo-llvm-cov` is absent.
- **`scripts/build.sh`.** POSIX `sh`, sources `~/.cargo/env` when `cargo` is not
  already resolvable, fails with a clear message when it still is not, then builds
  release. Herdr may launch without `~/.cargo/bin` on `PATH`.
- **`herdr-plugin.toml`.** Minimal: id, name, version, `min_herdr_version`,
  `platforms`, the `[[build]]` step, and the single `dashboard` pane, so
  `herdr plugin link .` works from day one. Not **BREAKING** — the manifest is
  introduced here; `plugin-actions` completes it.

## Non-Goals

- No TUI, no OpenSpec reading, no Herdr agent integration, no subprocess seam.
- No GitHub Actions workflow — that is `ci-pipeline`.
- No `[[actions]]` entries and no `dashboard-tab` pane — that is `plugin-actions`.
- No prebuilt-binary fast path in `scripts/build.sh` and no registry submission.
- No lowering, waiving, or disabling the 80% line-coverage floor.
- Nothing that writes inside `openspec/`; no change authoring, no orchestration across
  changes, no Windows support.

## Capabilities

### New Capabilities
- `plugin-build`: the release binary — how it is produced, where it lands, and how it
  behaves when invoked.
- `quality-gates`: the single `make check` entry point and the four gates behind it.
- `plugin-manifest`: `herdr-plugin.toml` and what Herdr does with it.

### Modified Capabilities
- None. `openspec/specs/` is empty; this change creates the first specs.

## Impact

- **New files:** `Cargo.toml`, `Cargo.lock`, `rustfmt.toml`, `Makefile`,
  `scripts/build.sh`, `herdr-plugin.toml`, `src/lib.rs`, `src/main.rs`, `tests/cli.rs`.
- **Modified:** `AGENTS.md` → "Current repo state", which currently states no
  `Cargo.toml` exists and tells agents not to assume build commands work; and
  `README.md` → Install, which advertises action-menu entries that arrive with
  `plugin-actions`.
- **Roadmap:** Phase 1, row `repo-foundation`. Depends on nothing; unblocks
  `ci-pipeline`, `plugin-config`, `schema-model`, `task-parsing`, and `subprocess-seam`.
- **External:** two one-time developer setup steps, `rustup component add clippy` and
  `cargo install cargo-llvm-cov`. Herdr 0.7.0 or later is needed only for the manual
  link check; no gate depends on Herdr being installed.
- **No data model, background job, external service, or sibling repository is affected.**
