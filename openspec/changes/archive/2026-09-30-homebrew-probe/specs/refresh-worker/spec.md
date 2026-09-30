## MODIFIED Requirements

### Requirement: The resolved `openspec` binary is spawned in an environment where its interpreter resolves

`openspec` is not a self-contained executable. The path the probe chain resolves is a
symbolic link to `@fission-ai/openspec/bin/openspec.js`. As npm installs it — the case steps
3 and 4 reach — that file's first line is `#!/usr/bin/env node`, so the spawned child must
find `node` on **its own inherited `PATH`** before any of this crate's logic runs. Homebrew's
`openspec` formula, which step 5 reaches, rewrites that line to an absolute interpreter path
(`#!/opt/homebrew/opt/node/bin/node`), so a Homebrew-installed binary needs no `PATH` to exec
and the rule below is merely harmless for it.

The failure is structural rather than incidental, and the probe chain reaches it by
construction: `openspec` is installed in the **same** directory as the `node` that runs it
(both under `<nvm>/versions/node/<version>/bin/`). So whenever that directory is off the
process's `PATH`, `resolve::step2_path` misses **and** `step3_nvm` or `step4_npm_prefix`
succeeds — the chain reaches steps 3 and 4 in precisely the condition that guarantees the
child cannot exec. `AGENTS.md` already records that the nvm bin directory "is not always on
the `PATH` a non-login shell inherits", which is the ordinary case for a GUI-launched Herdr.
Measured on the reference machine:
`env -i PATH=/usr/bin:/bin <nvm-bin>/openspec list --json` gives exit **127**, empty stdout,
and `env: node: No such file or directory` on stderr; the same command with the binary's own
directory prepended to `PATH` gives exit **0**.

This is a **different mechanism from the working-directory defect above with the same
symptom**, and the two must not be conflated: there, the child runs and reports a root the
plugin rejects; here, the child never runs at all and there is no payload to reject. Setting
a working directory fixes nothing on a machine whose probe reached step 3 or 4.

The composition root SHALL therefore supply an environment overlay alongside the working
directory: `cli::worker_cli(resolution, cwd, env_overlay)` SHALL pass the overlay to
`RealOpenspecCli` (`subprocess-seam` → "The real implementations spawn and return stdout and
do nothing else"), and `ui::start_collaborators` SHALL build it as exactly one entry — `PATH`
set to the resolved binary's **own parent directory**, followed by the path separator,
followed by the inherited `PATH` value.

The rule SHALL be that one entry and no more:

- The parent directory of the resolved path is used because that is where a Node
  distribution puts an installed package's shim **and** its `node`, which is the join the
  defect turns on. It is not a search: it is the one directory already known to exist,
  because a binary was resolved from it.
- The inherited `PATH` is **prepended to**, never replaced, so a machine that was already
  working keeps working and every other program the child may exec stays reachable.
- The inherited value SHALL be read through the injected environment lookup the composition
  root already holds, never `std::env::var` directly, on the crate's established terms.
- When the inherited `PATH` is absent, the overlay SHALL be the parent directory alone.
- The overlay SHALL be supplied for **every** resolved binary, not only for steps 3 and 4.
  Prepending a directory that already holds the resolved binary is a no-op for a binary found
  on `PATH`, and a rule that applied only to some probe steps would need the seam or the
  composition root to know which step won — a coupling neither has today.

The plugin SHALL NOT resolve `node` itself, SHALL NOT read the shim's shebang line, and SHALL
NOT build a second probe chain for an interpreter. `subprocess-seam` refuses to build a
resolution chain even for `herdr`, and one for `node` would be strictly worse: the correct
interpreter for a Node package is whichever one its own installation directory names, which
is what prepending that directory selects.

A 127 that still happens — a binary resolved from a directory holding no `node` — SHALL
remain a supported degraded state and SHALL NOT become an error screen: it is an
`openspec` command exiting non-zero, which the landed rules already turn into a problem row
naming the command and its exit code. Carrying the child's stderr on that row is
`cli-changes`' own change and SHALL NOT be duplicated here.

#### Scenario: The overlay prepends the resolved binary's own directory to `PATH`

- **WHEN** `ui::start_collaborators` is driven with an injected environment lookup reporting
  `PATH` as `/usr/bin:/bin` and a probe that resolves `openspec` at
  `/nvm/versions/node/v24.18.0/bin/openspec`, and the `RealOpenspecCli` it builds is
  inspected
- **THEN** its overlay is exactly one entry, `PATH` →
  `/nvm/versions/node/v24.18.0/bin:/usr/bin:/bin`, asserted as a string equality
- **AND** no other variable appears in the overlay, so nothing else about the child's
  environment is decided by this plugin

#### Scenario: An absent inherited `PATH` yields the directory alone

- **WHEN** the same drive is performed with an injected lookup that reports no `PATH` at all
- **THEN** the overlay's single entry is `PATH` → `/nvm/versions/node/v24.18.0/bin`, with no
  trailing separator
- **AND** nothing panics and no `unwrap` is reached, so a stripped environment degrades
  rather than failing the pane

#### Scenario: A binary already on `PATH` gets the same overlay, harmlessly

- **WHEN** the probe resolves `openspec` at `/usr/local/bin/openspec` with an inherited
  `PATH` of `/usr/local/bin:/usr/bin`
- **THEN** the overlay is `PATH` → `/usr/local/bin:/usr/local/bin:/usr/bin`
- **AND** the rule is therefore uniform across all five probe steps, so neither the seam nor
  the composition root needs to know which step resolved the binary

#### Scenario: A binary found under a Homebrew prefix gets the same overlay

- **WHEN** `ui::start_collaborators` is driven with an injected environment lookup reporting
  `PATH` as `/usr/bin:/bin`, a `Config` naming no `openspec_bin`, an `npm` hook returning
  `None`, and a Homebrew prefix list naming a scratch prefix `B` whose `B/bin/openspec` is an
  executable regular file
- **THEN** the overlay is exactly one entry, `PATH` → `B/bin:/usr/bin:/bin`
- **AND** Homebrew's own `openspec` formula names its interpreter by absolute path in its
  shebang, so the overlay is unneeded there and harmless, which is why step 5 needs no rule of
  its own

#### Scenario: A shim whose interpreter is unreachable exits 127 and renders a problem row

- **WHEN** a refresh cycle runs against an `openspec` stand-in that exits `127` with an empty
  stdout — the shape a missing interpreter produces — and the worker answers
- **THEN** the `Merged` result carries no changes from the CLI and `ChangeSet::problems`
  holds one entry naming `openspec list --json` and the exit code `127`
- **AND** the file-sourced change set is still what the pane renders, so a 127 degrades the
  pane to file mode rather than emptying it
- **AND** no panic, no error screen, and no `LoopError` results
