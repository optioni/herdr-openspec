# AGENTS.md

## Project overview

`herdr-openspec` is a [Herdr](https://herdr.dev) plugin: a read-only OpenSpec
dashboard rendered in a Herdr pane, written in Rust. It lists active and archived
changes, renders their artifacts as schema-driven tabs, shows task progress, maps
live Herdr agents onto changes, and launches an agent onto a change on request.

The durable sources of truth:

- **`PRD.md`** — requirements, goals, non-goals, risks.
- **`SPEC.md`** — the design contract every change implements.
- **`openspec/IMPLEMENTATION-ORDER.md`** — the roadmap and dependency graph.
- **`openspec/changes/<change>/`** — artifacts for an in-flight change.

Where prose and `SPEC.md` disagree, `SPEC.md` wins. When a change reveals that the
spec is wrong, update the spec as part of that change rather than letting the two drift.

## Current repo state

Early. There is no `Cargo.toml` yet — the design documents and the OpenSpec
scaffolding are the whole repository. Do not assume build, lint, or test commands
exist until `repo-foundation` has landed.

Important files:

- `SPEC.md` — architecture and design contract.
- `PRD.md` — product requirements.
- `openspec/IMPLEMENTATION-ORDER.md` — phased roadmap with a Mermaid dependency graph.
- `openspec/config.yaml` — project context and per-artifact rules.
- `openspec/schemas/tdd/schema.yaml` — the active schema (vendored by graft).
- `.claude/agents/` — OpenSpec orchestration agents (vendored by graft).
- `graft.toml` / `graft.lock` — what is vendored, and at which commit.

## Environment

- **Rust** stable (1.91+ at time of writing). Two one-time components:
  `rustup component add clippy` and `cargo install cargo-llvm-cov`.
- **Herdr** 0.7.0 or later, for the plugin manifest format and the `plugin`,
  `agent`, and `pane` CLI surfaces.
- **OpenSpec CLI** (`@fission-ai/openspec`) — optional for the plugin at runtime,
  required for the workflow below. Installed under nvm here, so it is not always on
  the `PATH` a non-login shell inherits.
- **Platforms:** macOS and Linux. Windows is out of scope.

## Vendored files — do not edit in place

`openspec/schemas/tdd/` and `.claude/agents/` are vendored from
`github.com/optioni/openspec-schemas` by [graft](https://github.com/optioni/graft)
and are listed in `graft.lock`. Editing them here is pointless: the next
`graft sync` overwrites the change silently. Edit the source repository and re-sync.

Repository-specific guidance belongs in `openspec/config.yaml` under `context` and
`rules`, not in the vendored schema.

## OpenSpec workflow

The active schema is `tdd`:

```text
proposal -> specs -> design -> tasks -> planning-review
```

Non-trivial work goes through a change proposal before implementation. Bug fixes,
typos, and small refactors do not.

```sh
openspec list
openspec status --change "<change>" --json
openspec instructions <artifact-id> --change "<change>" --json
openspec validate --strict
```

Run `openspec validate --strict` before presenting a proposal as ready. After an
approved change is implemented and verified, run `openspec archive` so the specs
stay in sync.

## Quality gates

All four are enforced in CI and available locally behind one target, so the two
cannot diverge:

```sh
make check
```

| Gate | Command |
|---|---|
| Format | `cargo fmt --all -- --check` |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings` |
| Test | `cargo test --all-features` |
| Coverage | `cargo llvm-cov --fail-under-lines 80` |

Coverage is a floor that catches drift, not the mechanism that produces tests — the
`tdd` schema drives RED → GREEN → REFACTOR, so tests come first by construction.

## Architecture rules

Two boundaries carry the design. Respect them, or the coverage target becomes
unreachable and the tests become integration tests by accident.

- **Nothing spawns a process outside `cli`.** `OpenspecCli` and `HerdrCli` are
  traits whose real implementations do nothing but spawn and return stdout. Parsing,
  merging, and decisions live on the testable side of that seam.
- **Views do no I/O.** They are pure functions from state to a ratatui frame, tested
  by rendering into a `TestBackend` buffer at 60 and 120 columns.

Two further invariants from `SPEC.md`:

- **Never fail closed.** A missing OpenSpec CLI, an unknown schema, or an
  unreachable Herdr socket degrades the view. It never replaces it with an error screen.
- **Never write to OpenSpec files.** An agent may be editing `tasks.md` in another
  pane. The dashboard reads.

Do not attribute an agent to a change on weak evidence. A terminal title is a
summary, not a change id. Unattributable agents are reported as a count, not guessed at.

## Development

```sh
herdr plugin link .    # build and load the working tree as a plugin
make check             # every gate
```

## Conventions

- **Commits:** Conventional Commits — `type(scope): description`. Commit after each
  task or logical task group while implementing a change.
- **Branching:** committing straight to `main` is fine unless the change itself
  calls for isolation.
- **Language:** English for code, comments, commits, and documentation.
- **Dependencies:** check the current stable version before adding one; do not rely
  on remembered version numbers.
