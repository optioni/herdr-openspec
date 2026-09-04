## ADDED Requirements

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

### Requirement: The `npm prefix -g` step is an injected hook, deferred until the subprocess seam exists

Step 4 needs the output of `npm prefix -g`, which requires spawning a process. No module
in this crate may spawn one outside `cli`, and `cli` does not exist until
`subprocess-seam`. The chain SHALL therefore take the npm prefix as an injected
`&dyn Fn() -> Option<PathBuf>`, following the same pattern by which configuration takes
the process environment as an injected lookup.

The binding this change ships SHALL return no prefix, so step 4 contributes nothing in
production and no code path spawns anything. `subprocess-seam` SHALL replace that
binding with one that runs `npm prefix -g` behind the seam. A test SHALL pin today's
empty result, so that hand-over turns a test red rather than passing silently.

#### Scenario: The shipped hook yields no prefix

- **WHEN** the binding this change provides is called
- **THEN** it returns no prefix
- **AND** the test asserting this names `subprocess-seam` as the change that will make it
  fail, because going red is the intended signal rather than a regression

#### Scenario: Resolution spawns no process

- **WHEN** the crate's own tests are run on a `PATH` from which every directory
  containing `npm`, `node`, or `openspec` has been removed, having first asserted that
  all three are unresolvable on it and that `cargo` and `rustc` still are
- **THEN** every resolution test still passes
- **AND** `src/resolve.rs` names no process API at all — no `std::process`, no `Command`,
  no `Stdio` — including in its comments, because the check reads source text and cannot
  tell a comment from a call

The normative clause is scoped to `src/resolve.rs` deliberately. A tree-wide "no file
under `src/` names `Command`" claim would be a live requirement that `subprocess-seam`'s
`src/cli.rs` is *required* to falsify — the defect `plugin-config`'s review repaired in
its own version of this scenario. The tree-wide grep is still run, as a task-level check
that `subprocess-seam` will rescope; it is not frozen here as a requirement.

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
