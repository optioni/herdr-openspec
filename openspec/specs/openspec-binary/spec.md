# openspec-binary Specification

## Purpose
Finding a usable `openspec` executable for the CLI path: a four-step probe chain over
the configured `openspec_bin`, the `PATH`, the nvm version tree, and the npm global
prefix, reporting which step won and caching the outcome for the session in a value
the caller owns. An absent binary is a supported state that degrades the view to file
mode rather than a fault; a configured path that cannot be used falls through and is
reported. Probing spawns no process and writes nothing.

## Requirements

### Requirement: A usable `openspec` is an executable regular file reached through symbolic links

Every step of the probe chain SHALL accept a candidate path only when, following
symbolic links, it names a **regular file** carrying at least one execute permission
bit. A directory SHALL NOT qualify, because a directory named `openspec` carries the
execute bit and would otherwise be returned as a binary. A regular file with no execute
bit SHALL NOT qualify. A symbolic link resolving to a qualifying file SHALL qualify —
the installed CLI on the reference machine is a symbolic link to a `.js` file — and a
symbolic link whose target does not exist SHALL NOT.

The path returned SHALL be the candidate path as the chain constructed it, **not** its
canonicalized target, because the returned path is the one a later change will execute
and the symbolic link is the stable name.

#### Scenario: An executable regular file is usable

- **WHEN** the chain probes a `PATH` entry `D` containing `D/openspec` as a regular file
  with mode `0755`, with nothing configured and no other source present
- **THEN** the resolved binary is `D/openspec` and its source is the `PATH` step

#### Scenario: A file without an execute bit is skipped

- **WHEN** `PATH` is `A:B`, `A/openspec` is a regular file with mode `0644`, and
  `B/openspec` is a regular file with mode `0755`
- **THEN** the resolved binary is `B/openspec`, so mere existence is not the test

#### Scenario: A directory named `openspec` is skipped

- **WHEN** `PATH` is `A:B`, `A/openspec` is a **directory** with mode `0755`, and
  `B/openspec` is an executable regular file
- **THEN** the resolved binary is `B/openspec`

#### Scenario: A symbolic link to an executable file is usable and is returned unresolved

- **WHEN** `PATH` is `A`, `A/openspec` is a symbolic link to `T/openspec.js` elsewhere in
  the scratch tree, and `T/openspec.js` is a regular file with mode `0755`
- **THEN** the resolved binary is `A/openspec` — the link path — and not `T/openspec.js`

#### Scenario: A dangling symbolic link is skipped

- **WHEN** `PATH` is `A:B`, `A/openspec` is a symbolic link to a path that does not
  exist, and `B/openspec` is an executable regular file
- **THEN** the resolved binary is `B/openspec`

### Requirement: The binary is probed in four ordered steps and the winning step is reported

The plugin SHALL probe for the `openspec` binary in exactly this order, taking the first
usable candidate and probing no further:

1. the `openspec_bin` value read from `config.toml` by `plugin-config`;
2. each `PATH` entry in order, joined with `openspec`;
3. `<nvm root>/versions/node/<version>/bin/openspec`, where the nvm root is `NVM_DIR`
   when it is set to a value that is neither empty nor whitespace-only, and
   `$HOME/.nvm` otherwise;
4. `<npm prefix>/bin/openspec`, where the npm prefix comes from the injected hook
   described below.

The result SHALL report **which** step produced the binary, so that the chain's order is
observable rather than inferred from a path that two steps could both have produced.

A `PATH` entry that is empty or whitespace-only SHALL be skipped rather than interpreted
as the current directory: a plugin pane starts in a directory the user chose for other
reasons, and resolving an executable from it is not a behaviour this plugin offers. An
absent `PATH` SHALL contribute no candidates rather than being an error. Because that
rule is invisible in the resolved path — the current directory is frequently a
repository, whose `openspec` child is a *directory* and would be rejected for an
unrelated reason — step 2's candidate list SHALL be observable in its own right, as a
pure function of the `PATH` string, so the skipping is asserted rather than inferred.

Within step 3, the nvm root SHALL be `NVM_DIR` when non-blank and `$HOME/.nvm`
otherwise; when neither is available the step SHALL contribute no candidates rather than
panicking. Version directories SHALL be tried newest first, ordered **numerically** by
the `MAJOR.MINOR.PATCH` triple in a leading-`v` name, so that `v10.0.0` precedes
`v9.99.99`; a directory whose name does not parse as such a version SHALL still be
eligible, ordered after every parsed version and among its own kind by name descending.
A version directory with no usable `bin/openspec` SHALL be skipped rather than ending
the step.

When no step produces a usable binary, the result SHALL report no binary and SHALL NOT
record a problem: an absent CLI is a supported state that degrades the view to file
mode, not a fault.

#### Scenario: The configured path wins over every other source

- **WHEN** `openspec_bin` is a scratch executable `C/openspec`, `PATH` contains a
  *different* executable `D/openspec`, and the nvm tree contains a third
- **THEN** the resolved binary is `C/openspec` and its source is the configured step
- **AND** the result would be `D/openspec` if step 1 were skipped, which is what makes
  the assertion on the reported source load-bearing
- **AND** the reported problems are empty, because a configured path that *works* is not
  a fallback and must not produce a message the header would render

#### Scenario: `PATH` wins when nothing is configured

- **WHEN** `openspec_bin` is absent, `PATH` contains an executable `D/openspec`, and the
  nvm tree contains a different executable
- **THEN** the resolved binary is `D/openspec` with the `PATH` step as its source

#### Scenario: `PATH` entries are searched left to right

- **WHEN** `PATH` is `A:B` and both `A/openspec` and `B/openspec` are executable regular
  files
- **THEN** the resolved binary is `A/openspec`

#### Scenario: An empty or blank `PATH` entry is not the current directory

- **WHEN** the candidate list is built from the `PATH` string `:D:   :` — a leading empty
  entry, a directory `D`, a trailing empty entry, and a whitespace-only entry
- **THEN** the candidate list is exactly `[D/openspec]`: one entry, absolute, with no
  bare relative `openspec` and no candidate under a directory of spaces
- **AND** resolving with that same `PATH`, where `D/openspec` is an executable regular
  file, yields the absolute `D/openspec`
- **AND** the candidate-list assertion is the load-bearing half: an implementation
  honouring the POSIX "empty entry means the current directory" rule produces the same
  *resolved* path here, because the working directory during a test run is the crate
  root, whose `openspec` child is a directory and is rejected for an unrelated reason

#### Scenario: An absent or blank `PATH` contributes nothing

- **WHEN** the chain runs three times with `PATH` absent from the lookup, then set to
  `""`, then set to `"   "`, in each case with nothing configured, no nvm tree, and an
  npm prefix holding an executable `openspec`
- **THEN** all three resolve to the npm-prefix binary with the npm-prefix step as their
  source, so a blank `PATH` fell through rather than erroring or matching

#### Scenario: The nvm tree is searched when `PATH` has nothing

- **WHEN** `openspec_bin` is absent, `PATH` is a directory with no `openspec` in it, and
  `HOME` names a scratch home containing
  `.nvm/versions/node/v24.20.0/bin/openspec` as an executable regular file
- **THEN** that path is resolved with the nvm step as its source

#### Scenario: Node versions are ordered numerically, not lexically

- **WHEN** the nvm tree holds executable `openspec` binaries under both `v9.99.99` and
  `v10.0.0`
- **THEN** the resolved binary is the one under `v10.0.0`, which a lexical ordering would
  have ranked below `v9.99.99`

#### Scenario: A version directory without a usable binary is skipped

- **WHEN** the nvm tree holds `v22.0.0/` with no `bin/` at all, `v21.0.0/bin/openspec`
  as a non-executable file, and `v20.0.0/bin/openspec` as an executable file
- **THEN** the resolved binary is the one under `v20.0.0`, so neither earlier version
  ended the step

#### Scenario: A version directory whose name is not a version is still eligible, and sorts last

- **WHEN** the only entry in the nvm tree is `system/bin/openspec`, an executable regular
  file
- **THEN** it is resolved with the nvm step as its source, so an unparseable name is kept
  rather than dropped
- **AND** re-running against a tree holding **both** `vnightly/bin/openspec` and
  `v20.0.0/bin/openspec`, each executable, resolves the `v20.0.0` one, because a parsed
  version outranks an unparseable name. `vnightly` is the fixture rather than `system`
  because plain name-descending sorting puts `vnightly` **above** `v20.0.0` and `system`
  **below** it — only the first of those two discriminates against the wrong ordering

#### Scenario: `NVM_DIR` overrides the default nvm root

- **WHEN** `NVM_DIR` names a scratch directory holding
  `versions/node/v20.0.0/bin/openspec`, and `HOME` names a *different* scratch home
  holding `.nvm/versions/node/v20.0.0/bin/openspec`, both executable
- **THEN** the binary under `NVM_DIR` is resolved
- **AND** re-running with `NVM_DIR` set to `"   "` resolves the one under `HOME`, so a
  blank value falls through to the default rather than naming a directory of spaces
- **AND** re-running with **both** `NVM_DIR` and `HOME` absent from the lookup yields no
  binary and no panic, rather than unwrapping an absent `HOME`

#### Scenario: The nvm tree outranks the npm prefix

- **WHEN** nothing is configured, `PATH` holds no `openspec`, the nvm tree holds an
  executable `H/.nvm/versions/node/v20.0.0/bin/openspec`, and the injected hook returns a
  **different** directory `N` holding an executable `N/bin/openspec`
- **THEN** the resolved binary is the nvm one, with the nvm step as its source
- **AND** this is the one adjacent pair in the chain that no other scenario separates:
  on the reference machine `npm prefix -g` prints the nvm version directory, so the two
  steps resolve to the same path there and a swapped chain would be invisible

#### Scenario: The npm prefix is the last resort

- **WHEN** nothing is configured, `PATH` holds no `openspec`, the nvm tree is absent, and
  the injected npm-prefix hook returns a scratch directory `N` holding an executable
  `N/bin/openspec`
- **THEN** `N/bin/openspec` is resolved with the npm-prefix step as its source

#### Scenario: An npm prefix without a usable binary yields nothing

- **WHEN** the injected hook returns a directory holding no `bin/openspec`, and no
  earlier step produced a candidate
- **THEN** no binary is resolved and no problem is recorded

#### Scenario: Nothing anywhere is a supported state, not a fault

- **WHEN** nothing is configured, `PATH`, `NVM_DIR`, and `HOME` are all absent from the
  lookup, and the hook returns nothing
- **THEN** no binary is resolved
- **AND** the reported problems are empty, which is what distinguishes this from a
  configured path that could not be used

### Requirement: A configured path that cannot be used falls through and is reported

When `openspec_bin` is set but does not name a usable binary — it does not exist, it is
not a regular file, or it carries no execute bit — the chain SHALL continue with steps 2
through 4 rather than resolving to nothing, and SHALL record exactly one problem naming
the configured path. Silently substituting a different binary would hide the user's
mistake; refusing to look further would fail closed, which `SPEC.md` forbids.

The problem SHALL be reported whether or not a later step succeeds, so that "not
configured" and "configured wrong" are distinguishable by the caller.

#### Scenario: A configured path that does not exist falls through to `PATH`

- **WHEN** `openspec_bin` is `G/openspec`, which was never created, and `PATH` contains
  an executable `D/openspec`
- **THEN** the resolved binary is `D/openspec` with the `PATH` step as its source
- **AND** exactly one problem is reported, and its text contains `G/openspec`
- **AND** `G/openspec` still does not exist afterwards

#### Scenario: A configured path that is not executable falls through

- **WHEN** `openspec_bin` names a regular file with mode `0644` and `PATH` contains an
  executable `D/openspec`
- **THEN** the resolved binary is `D/openspec` and exactly one problem names the
  configured path

#### Scenario: A configured path fails and nothing else is found

- **WHEN** `openspec_bin` names a path that does not exist and no other step produces a
  candidate
- **THEN** no binary is resolved
- **AND** exactly one problem still names the configured path, so the caller can tell
  this apart from the case where nothing was configured at all

### Requirement: The `npm prefix -g` step is an injected hook bound to a real probe in `cli`

Step 4 needs the output of `npm prefix -g`, which requires spawning a process. No module
in this crate may spawn one outside `cli`. The chain SHALL therefore continue to take the
npm prefix as an injected `&dyn Fn() -> Option<PathBuf>`, following the same pattern by
which configuration takes the process environment as an injected lookup — that injection
is what keeps `resolve` pure and unit-testable, and it survives the seam landing rather
than being replaced by it.

What changes is the binding. The production binding SHALL be the real probe published by
the `subprocess-seam` capability, which starts the npm program with the arguments `prefix`
and `-g`, reads its **stdout only**, trimmed, and reports no prefix when the program could
not be started, exited non-zero, or produced empty output. `resolve::openspec_bin_from_env`
SHALL pass that binding and no other.

`resolve` SHALL expose no npm binding of its own. The placeholder that always returned
nothing SHALL be removed rather than left beside the real one.

`src/resolve.rs` SHALL name no process API at all — no `std::process`, no `Command`, no
`Stdio` — including in its comments, because the check reads source text and cannot tell a
comment from a call. This clause is scoped to `src/resolve.rs` deliberately; the tree-wide
form of the rule, with `src/cli.rs` excluded as the one module permitted to spawn, is a
requirement of the `subprocess-seam` capability rather than of this one.

#### Scenario: The hook is still injected, so the chain stays pure

- **WHEN** `openspec_bin` is called with nothing configured, a `PATH` holding no
  `openspec`, no nvm tree, and a fourth-step hook that is an ordinary closure returning a
  scratch directory `N` holding an executable `N/bin/openspec`
- **THEN** `N/bin/openspec` is resolved with the npm-prefix step as its source
- **AND** the call spawns nothing, because the collaborator is a closure and `resolve` has
  no other way to reach a process

#### Scenario: The production binding is the real probe

- **WHEN** `src/resolve.rs` is searched for the identifier the composition passes as its
  fourth-step hook
- **THEN** it names `cli::npm_prefix`, and searching all of `src/` for
  `npm_prefix_deferred` yields no match
- **AND** both halves are needed: the absence check alone passes for a hand-over in which
  the placeholder was renamed and still returns nothing, and every other check in this
  change — the chain tests, the no-spawn greps, the no-tools suite run, and the binding's
  own smoke test — stays green for that implementation, because no prefix is a legitimate
  answer

#### Scenario: Resolution spawns no process

- **WHEN** the crate's own tests are run on a `PATH` from which every directory
  containing `npm`, `node`, or `openspec` has been removed, having first asserted that
  all three are unresolvable on it and that `cargo` and `rustc` still are
- **THEN** every resolution test still passes
- **AND** `src/resolve.rs` names no process API at all — no `std::process`, no `Command`,
  no `Stdio` — including in its comments, because the check reads source text and cannot
  tell a comment from a call
- **AND** the whole suite passes on that `PATH` too, including `cli`'s own tests, because
  every spawning test names an absolute scratch program path

### Requirement: A resolved binary is cached for the session in a value the caller owns

Probing walks `PATH` and reads directories, and a dashboard re-renders often, so the
outcome SHALL be cached. The cache SHALL be an ordinary value the caller holds for the
life of the process, not process-global state: `cargo test` runs the suite in parallel
threads of one process, and a global cache would leak one test's fixture into another's.

The cache SHALL store the whole outcome, including the case where no binary was found,
so that an absent CLI costs one probe per process rather than one per render.

#### Scenario: A second lookup does not re-probe

- **WHEN** a cache is asked for a resolution twice, given a probe closure that increments
  an atomic counter and returns a resolution naming `D/openspec`
- **THEN** the counter reads exactly 1
- **AND** both calls return the same resolution

#### Scenario: A negative result is cached too

- **WHEN** a cache is asked twice, given a probe closure that increments a counter and
  returns a resolution with no binary and no problems
- **THEN** the counter reads exactly 1, and both calls report no binary

#### Scenario: Two caches are independent

- **WHEN** two separate cache values are each asked once, with probe closures returning
  different binaries
- **THEN** each returns its own binary, proving the cache is per-value rather than
  process-global

### Requirement: One composition binds resolution to the real environment

The crate SHALL expose exactly one function that resolves the binary against the real
process environment, composing the configured value, `config::env_lookup`, and the
deferred npm hook, and doing nothing else. Keeping it to a composition is what stops the
crate's untestable residue from growing; the environment lookup it uses already carries
its own assertions.

#### Scenario: The composition honours a configured binary

- **WHEN** the composition is called with a `Config` whose `openspec_bin` is a scratch
  executable `C/openspec`
- **THEN** it resolves `C/openspec` with the configured step as its source, on the real
  process environment
- **AND** the assertion is on a path no other step could have produced, so a composition
  that ignored its argument and returned nothing would fail
- **AND** the result is deterministic on any machine, because step 1 wins before `PATH`,
  `NVM_DIR`, or `HOME` are consulted — a property the chain-ordering requirement above
  already pins with its own tests

### Requirement: The resolution's problems and its outcome reach the dashboard

`resolve::BinResolution` carries `problems`, and `SPEC.md` → Degraded states relies on it in
two rows: a configured `openspec_bin` that does not name a usable binary is "named in
`BinResolution::problems` rather than being silent", and an absent binary puts the pane in
"file mode, with a dim `file mode` badge in the header". Neither is true in the shipped
binary today: `cli::worker_cli` takes the resolution **by value** and reads `.found` alone, so
`problems` is dropped on the floor and nothing anywhere learns that no binary was found.

The production composition that builds the worker's CLI handle SHALL therefore surrender both
halves of the resolution to its caller rather than consuming them: the resolved handle (or its
absence) **and** the `problems` the probe accumulated. The composition SHALL remain the only
place binding resolution to the real environment — `openspec-binary`'s existing requirement is
unchanged — and SHALL still spawn nothing beyond the `npm prefix -g` probe the fourth step
already owns.

`ui::start_collaborators` SHALL fold the returned `problems` into the `problems` vector it
already hands to `run_wired`, so they reach `Dashboard::refresh.problems` on exactly the terms
a watcher that would not start already does, and SHALL report whether a usable binary was
found, so `responsive-layout`'s `file mode` badge has a fact to render.

A probe that resolves a binary cleanly SHALL contribute **no** problem: `BinResolution::problems`
is empty on the happy path and the pane renders exactly as it did.

#### Scenario: A configured path that cannot be used reaches the list as a problem row

- **WHEN** `run_wired` is driven at 120x20 and again at 60x20 over a scratch repository with a
  `Config` whose `openspec_bin` names a path that exists but is not executable, an environment
  in which no later probe step resolves a binary either, and an event source that presses `q`
- **THEN** the returned dashboard's `refresh.problems` holds an entry naming that path and the
  reason the probe rejected it
- **AND** the list region's first interior row, at both widths, begins `! ` and names that path
- **AND** the same run with an `openspec_bin` naming a **usable** scratch `#!/bin/sh` program
  yields an empty `refresh.problems` and a list whose first interior row is a change row, so
  the problem is the configuration's and not a constant

#### Scenario: No binary anywhere is reported as file mode rather than as an error

- **WHEN** the same run is driven with a `Config` naming no `openspec_bin` and an environment
  in which every probe step fails
- **THEN** `run_wired` returns `Ok(dashboard)`, never `Err`: an absent binary is a supported
  state
- **AND** the dashboard reports file mode, which `responsive-layout` renders as the header
  badge
- **AND** the change list is still complete, sourced from files, so the pane degrades rather
  than emptying

#### Scenario: A resolved binary contributes nothing

- **WHEN** the probe resolves a usable binary at its first step
- **THEN** `problems` is empty, the dashboard does not report file mode, and the header carries
  no badge
- **AND** the buffers are byte-identical to the ones the same dashboard produced before this
  change existed

### Requirement: The composition root injects the probe's environment, so an outer test never consults the machine

`resolve::openspec_bin_from_env` reads the real process environment and calls the real
`cli::npm_prefix`. `ui::start_collaborators` reaches it through `cli::worker_cli_from_env`,
which means an outer-loop test driving `ui::run_wired` on a tree where the first probe step
does **not** win runs the remaining steps against the developer's own `PATH`, spawns the real
`npm`, and can resolve whichever global `openspec` happens to be installed. Every landed outer
test avoids this by naming a **usable** `openspec_bin`, so the chain stops at step one; the
scenarios this change adds deliberately exercise the case where nothing resolves, and cannot.

`ui::Startup` SHALL therefore carry the two bindings the probe needs — the environment lookup
and the `npm prefix -g` hook — and `start_collaborators` SHALL reach the probe through them
rather than through `openspec_bin_from_env`. `ui::run` SHALL be the one caller that passes
`config::env_lookup()` and `cli::npm_prefix`, exactly as it is already the one caller that
passes `cli::HERDR_PROGRAM` and `read_artifact`.

`start_collaborators` SHALL still name none of `OpenspecCli`, `HerdrCli`, `from_cli`,
`CliChanges`, or `npm_prefix`: it names `resolve::openspec_bin` and `cli::worker_cli`, whose
own signatures carry those types, which is what keeps `NOCLI-SHELL` green.

This is the same rule `AGENTS.md` already states for the environment — "a function that needs
the process environment takes a `&dyn Fn(&str) -> Option<String>` rather than calling
`std::env::var` directly" — applied one level further out, to the composition that assembles
the probe.

#### Scenario: An outer test drives a failing probe without touching the machine

- **WHEN** `run_wired` is driven at 120x20 and again at 60x20 with a `Startup` whose
  environment lookup is a fixture map holding no `PATH` and no `HOME`, whose `npm` hook returns
  `None`, and a `Config` naming no `openspec_bin`
- **THEN** the returned dashboard reports file mode, and the run spawns **no** process at all
- **AND** the same run with the `npm` hook returning a scratch prefix under which a usable
  `openspec` exists resolves that binary and does not report file mode, which is the
  discriminating control that the hook is consulted rather than ignored
- **AND** no test in this change calls `std::env::set_var`, changes the process working
  directory, or spawns `npm`

#### Scenario: The composition root is the one place the real bindings are named

- **WHEN** `src/ui/mod.rs`'s production slice is searched
- **THEN** `pub fn run()`'s own body names `config::env_lookup(` and `cli::npm_prefix`
- **AND** `start_collaborators`'s body names neither, and names no `npm_prefix` at all, so
  `NOCLI-SHELL` and `WIRED` stay green
- **AND** removing either binding from `run`'s body fails `WIRED`'s name legs, which is the
  check that the injection is real rather than decorative

### Requirement: Binary resolution reads and never writes

Probing SHALL create no file and no directory anywhere — not the configured path, not a
`PATH` entry, not an nvm version directory, and not the npm prefix.

#### Scenario: A full probe leaves the filesystem byte-identical

- **WHEN** a scratch tree holding a `PATH` directory, an nvm tree, an npm prefix, and an
  **empty** directory is snapshotted — every path, **including every directory**, every
  file's bytes, and every entry's modification time — and a full four-step probe is then
  run twice against it
- **THEN** a second snapshot equals the first exactly, with no entry added, removed, or
  modified
- **AND** the snapshot records directories, not only files, so a probe that created a
  missing `bin/` while looking for one is caught
