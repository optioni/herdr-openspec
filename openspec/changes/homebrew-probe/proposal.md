## Why

`openspec` is now packaged in homebrew-core (`brew install openspec`), and that is how the
reference machine has it: `/opt/homebrew/bin/openspec`, a link into
`Cellar/openspec/<version>/`, with no copy under any nvm version tree. The probe chain has
no step for it. A Herdr started outside a login shell hands the pane a `PATH` without
`/opt/homebrew/bin`, so step 2 misses; step 3 finds nothing under nvm; and step 4 cannot
rescue it, because `npm prefix -g` is spawned as a bare `npm` resolved through that same
`PATH`. The pane drops to **file mode** on a machine with a working, current CLI — the
degraded view is correct for "no CLI", and wrong here, and it gives the reader no hint why.

## What Changes

- The binary probe chain gains a **fifth, last-resort step**: `<prefix>/bin/openspec` for
  each of Homebrew's three default prefixes, in order — `/opt/homebrew` (Apple Silicon),
  `/home/linuxbrew/.linuxbrew` (Linux), `/usr/local` (Intel macOS). It is a filesystem
  check on fixed paths: it spawns nothing and reads no environment variable.
- `resolve::BinSource` gains a `Homebrew` variant, so the settings panel's `openspec_bin`
  row can say which step resolved the binary (label `Homebrew`), on the same terms as the
  four existing variants.
- The step's prefix list is **injected**, exactly as the npm-prefix hook is:
  `resolve::openspec_bin` takes it as a parameter and `ui::Startup` carries it as a field,
  and `ui::run` and `resolve::openspec_bin_from_env` are the only places that pass the
  real list. `start_collaborators` takes its three probe inputs as one bundle rather than
  gaining an eighth parameter, which clippy's `too_many_arguments` rejects. Without this, every test that drives "nothing resolves" would find the
  reference machine's own `/opt/homebrew/bin/openspec` and stop proving anything.
- `SPEC.md`, `AGENTS.md`, and `openspec/config.yaml`'s injected `context` are updated.
  The chain grows from four steps to five. The claim that the CLI is nvm-installed is no
  longer true on the reference machine, and every OpenSpec agent reads that claim as a
  working instruction.

## Non-Goals

- **No `HOMEBREW_PREFIX` lookup and no `brew --prefix` spawn.** An environment that sets
  `HOMEBREW_PREFIX` already has `brew shellenv`'s `PATH` and resolves at step 2. A
  non-default prefix is what `openspec_bin` in `config.toml` is for.
- **No login-shell `PATH` recovery.** Spawning `$SHELL -lc` to learn the user's real
  `PATH` would fix every manager at once, but it runs arbitrary rc files at pane startup.
  That is a separate decision with its own risks.
- **No other version managers** (volta, fnm, asdf, mise). The same argument applies to
  them. None of them is measured on the reference machine, so none is added on speculation.
- **No reordering of the existing four steps**, and no change to the `PATH` overlay rule.
  Homebrew's `openspec` formula pins its interpreter by absolute shebang
  (`#!/opt/homebrew/opt/node/bin/node`), and the overlay's "prepend the binary's own
  directory" is harmless for it.
- Nothing here touches the PRD's non-goals. It writes nothing, orchestrates nothing,
  authors nothing, and adds no Windows path.

This is **unplanned post-roadmap work**. The roadmap's `repo-resolution` row fixed the
chain when the only install anyone had measured was npm-under-nvm. Homebrew packaging
arrived later.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `openspec-binary`:
  - The ordered-steps requirement is **renamed** to drop its count ("…probed in ordered
    steps…") and becomes five steps, with the Homebrew step last.
  - "A configured path that cannot be used" falls through steps 2–5.
  - The two composition requirements inject and bind the prefix list. Their stale
    `cli::npm_prefix`, `worker_cli_from_env`, and "deferred" wording is corrected.
  - The read-never-writes scenario now actually reaches all five steps.
- `setting-provenance`: `Provenance::Probe(BinSource::Homebrew)` has a reader-facing label,
  and the label scenario covers five variants.
- `refresh-worker`: the `PATH`-overlay requirement is uniform across five probe steps, not
  four. Its shim paragraph distinguishes npm's `#!/usr/bin/env node` from Homebrew's
  absolute shebang. It gains one scenario for a Homebrew-resolved binary. The rule itself
  does not move.
- `quality-gates`: `WIRED` gains a body-scoped leg 8 binding `HOMEBREW_PREFIXES` to `run`.
  The requirement's required-name count is corrected to the fourteen the script already
  checks.
- `agent-prompts`: two requirements that enumerate the probe's steps ("configuration,
  `PATH`, nvm, `npm prefix -g`"; "any of the probe's four steps") name the fifth.

## Impact

- **Code:** `src/resolve.rs` gains the step, the variant, the prefix constant, and the
  new parameter, and every `openspec_bin` call site in its tests passes an empty list.
  `src/settings.rs` gains the label arm. `src/ui/mod.rs` gains a `ProbeBindings` bundle and
  changes `Startup`, `start_collaborators` (7 parameters become 6), `run`, and every
  test-built `Startup`/`ProbedStartup`. `src/cli.rs` updates its
  two direct `openspec_bin` calls in tests.
- **Gates:** `scripts/gates/wired.sh` gains leg 8, which is body-scoped: `run` names the
  constant and `start_collaborators` does not. It also gains a positive control anchored on
  `src/resolve.rs`. `tests/gate-controls.toml` gains three `[[control]]` plants for them. No
  new gate script is added.
- **Docs:** `SPEC.md` (the probe chain section and the two "four-step" mentions),
  `AGENTS.md` (Environment, and the Startup paragraph), and `openspec/config.yaml`'s
  `context`. `tests/doc_contract.rs` binds that block's fixture claim and its `make`
  commands, not its install sentence, so the edit is free of that test as long as those
  two are left intact.
- **Behaviour:** a machine that previously landed in file mode with Homebrew's `openspec`
  installed now resolves it, and the merged CLI view replaces file mode. Every machine
  where an earlier step already wins behaves byte-identically.
- **No new dependency**, no manifest or `config.toml` format change, and no keybinding
  change, so nothing here is **BREAKING**.
- **Concurrent work:** `worktree-agents` is active and touches `agent-attribution` and
  `agent-launch` only, so it shares no capability. Both changes edit `SPEC.md` and
  `AGENTS.md`, in different paragraphs. That is a merge-order concern, not a conflict of
  contract.
