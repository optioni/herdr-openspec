## Why

Nothing in the crate can yet answer the two questions every later change starts from:
*which directory is the OpenSpec repository*, and *where is the `openspec` binary*.
`changes-from-files` enumerates `openspec/changes/` and cannot begin without the first;
`changes-from-cli` invokes a binary that a Herdr pane — started without a login shell —
will usually not find on `PATH`. This is the first row of Phase 2 in
`openspec/IMPLEMENTATION-ORDER.md`, and the last piece of the resolution chain that is
still a pure transformation.

## What Changes

- **A `resolve` module locates the repository** by walking up from a starting directory
  for a `openspec/` **directory** (a *file* of that name is not a repository), stopping
  at the innermost match and at the filesystem root. When nothing is found, the result
  names the directory the walk began from, so the empty state `SPEC.md` promises has
  something concrete to print.
- **A four-step probe locates the `openspec` binary** — the configured
  `Config::openspec_bin`, then `PATH`, then the nvm version directories, then
  `npm prefix -g` — returning the winning path *and which step won it*, so the chain's
  order is observable rather than inferred. Every step requires an **executable regular
  file**, resolved through symlinks: the real install on this machine is a symlink to a
  `.js` file, and a directory named `openspec` carries the execute bit.
- **Step 4 is an injected hook, not a subprocess.** `npm prefix -g` needs a process
  spawn, and the `cli` seam does not exist until Phase 3. `resolve` therefore takes the
  npm prefix as a `&dyn Fn() -> Option<PathBuf>`, exactly as `config` takes the
  environment as a lookup closure. The binding this change ships returns `None`;
  `subprocess-seam` replaces it, and a test asserts today's `None` so that hand-over goes
  red rather than silent.
- **Resolution is cached in a value, not a global.** A `BinCache` the caller owns caches
  the whole outcome, negative results included, so a re-render never re-walks `PATH`. A
  `static` would be shared by every parallel test in the process.
- **`SPEC.md` is corrected** where it says all process spawning sits behind
  `OpenspecCli` and `HerdrCli` — step 4 spawns `npm`, which is neither — where the
  binary chain leaves executability, symlinks, nvm version ordering, and a mis-configured
  `openspec_bin` unspecified, and where the degraded-states table has no row for a
  configured path that does not resolve. `openspec/IMPLEMENTATION-ORDER.md`'s
  `subprocess-seam` row gains the `npm prefix -g` probe it must wire.
- Not **BREAKING**: no manifest key, no config key, and no keybinding moves. `config.toml`
  is read, not extended.

## Non-Goals

- **No process spawn, and no `cli` module.** Step 4's real implementation is
  `subprocess-seam`'s work. Nothing in `src/` gains a process API here.
- **No change enumeration.** Finding `openspec/` is here; reading what is inside it is
  `changes-from-files`.
- **No schema reading.** `openspec/config.yaml` is `schema-model`'s file, not this one's.
- **No invocation context.** The *starting* directory is an argument. Deriving it from
  Herdr's injected workspace variables belongs to whichever change first renders a pane.
- **No `openspec` execution.** This change finds a path and stats it. It never runs it,
  and never asks it for its version.
- **No `PATH` search for anything else.** No `herdr`, no `node`. One binary, one chain.
- **No watching or invalidation.** The cache lives for the process; `live-refresh` owns
  re-reading.

## Capabilities

### New Capabilities

- `repo-discovery`: walking up from a starting directory to the nearest ancestor holding
  an `openspec/` directory, and the not-found result that names where the search began.
- `openspec-binary`: the four-step probe chain, what counts as a usable binary at each
  step, the deferred `npm prefix -g` hook, the recorded problems for a configured path
  that does not resolve, and session caching.

### Modified Capabilities

None. `plugin-config` hands over `openspec_bin` unchanged and its requirements are
untouched; `plugin-build`'s dependency-set requirement stays true because this change
adds no dependency — it is re-verified rather than amended.

## Impact

- **Code:** new `src/resolve.rs`; `src/lib.rs` gains `pub mod resolve;`, an
  executable-file builder and a symlink helper for fixtures, and one correction to the
  existing `testutil::snapshot`, which records only non-directory entries today and so
  cannot see a created empty directory — the write both containment guards exist to catch. `src/main.rs` is untouched — nothing
  consumes resolution until `changes-from-files`.
- **Build:** none. No dependency is added; `Cargo.toml` and `Cargo.lock` are unchanged,
  and that is itself checked.
- **Docs:** `SPEC.md` → Architecture (the subprocess seam), Data layer → Resolution chain
  (Repository and *The `openspec` binary*), Degraded states, and Testing → Unit-tested
  modules; `openspec/IMPLEMENTATION-ORDER.md` → the Phase 3 `subprocess-seam` row and
  the dependency graph, which gains the `repo-resolution --> subprocess-seam` edge that
  handing over the hook creates; `AGENTS.md` → Current repo state.
- **External:** none. No network, no sibling repository, no registry. Nothing is written
  anywhere — this change's production code never creates a file or a directory.
