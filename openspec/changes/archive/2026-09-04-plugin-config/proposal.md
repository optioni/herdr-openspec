## Why

Every later change needs to know three things a user is allowed to decide: where the
`openspec` binary lives, which agent kind to launch, and how many archived changes to
list. `repo-resolution` cannot write its probe chain against a real first step until
that configuration is readable, and `agent-launch` cannot name an agent after a change
longer than 32 characters without somewhere to record the mapping. This is the third
row of Phase 1 in `openspec/IMPLEMENTATION-ORDER.md`.

## What Changes

- **The plugin's own directories are read from the environment, not from a
  subprocess.** Herdr injects `HERDR_PLUGIN_CONFIG_DIR` and `HERDR_PLUGIN_STATE_DIR`
  into every plugin pane and action process — verified against the installed Herdr
  0.8.2, and the mechanism three shipping Herdr plugins already use. Running outside a
  Herdr pane falls back to the same paths Herdr would have supplied. Nothing spawns
  `herdr plugin config-dir`, so this change does not wait on `subprocess-seam` and adds
  no process spawn outside `cli`.
- **`config.toml` is read into a `Config` value** carrying `openspec_bin`,
  `agent_kind`, and `archived_count`. An absent directory, an absent file, an absent
  key, a malformed file, or a key of the wrong type each yields the documented default
  for that key rather than an error. `~` and `$HOME` in `openspec_bin` are expanded.
- **A plugin-local state file records truncated agent-name mappings.** A change name
  longer than 32 characters, or one Herdr's `[a-z][a-z0-9_-]{0,31}` rule rejects, is
  converted to a legal agent name; the mapping from that name back to the change is
  recorded under `HERDR_PLUGIN_STATE_DIR` and read back on the next launch. Writes are
  atomic, so two dashboards cannot leave a half-written file behind.
- **`toml` becomes the crate's first third-party dependency**, pinned, with default
  features off. It builds six transitive crates and no proc macro. Hand-rolling a TOML
  reader would silently ignore valid configuration a user wrote, which is the opposite
  of degrading honestly.
- **`SPEC.md` is corrected** where it says the config directory comes from `herdr
  plugin config-dir`, where its stack list omits a TOML parser, where its module map has
  no row for the modules this change adds, where its degraded-states table lacks a row
  for either new degraded read, and where agent-name derivation is described as
  truncation alone. `AGENTS.md`'s two architecture rules — nothing spawns outside `cli`,
  and nothing writes to OpenSpec files — are rewritten in place to carry the corollaries
  this change establishes, and `openspec/IMPLEMENTATION-ORDER.md`'s own `plugin-config`
  row is corrected for the same reason `SPEC.md` is.
- Not **BREAKING**: the config format is introduced here, not altered. No manifest key
  and no keybinding moves.

## Non-Goals

- **No probe chain.** Consuming `openspec_bin` as step 1 of the four-step search is
  `repo-resolution`'s work. This change hands over a value and validates nothing about
  the path it names.
- **No launching.** Deriving and recording an agent name is here; splitting a pane and
  starting an agent is `agent-launch`.
- **No `herdr` or `openspec` invocation**, and therefore no `cli` module. That seam
  lands in `subprocess-seam`.
- **No config writing.** The plugin reads `config.toml`; only the user edits it.
- **No reloading.** Configuration is read once per process. `live-refresh` owns
  watching, and nothing here needs to change for it to.
- **No agent-kind validation.** Checking a kind against Herdr's list would need a
  subprocess; an unknown kind is Herdr's error to report at launch time.

## Capabilities

### New Capabilities

- `plugin-config`: locating the plugin's configuration directory without a subprocess,
  reading `config.toml`, and the defaults every absent or malformed input falls back to.
- `plugin-state`: locating the plugin's state directory, deriving a Herdr-legal agent
  name from a change name, and recording and reading back that mapping atomically.

### Modified Capabilities

- `plugin-build`: the requirement "The crate produces one binary, with no third-party
  dependencies" asserts `Cargo.lock` holds exactly one package. That was true of the
  scaffold and is now false. The requirement narrows to naming the dependency set
  explicitly, asserting it against the resolved build graph rather than against the text
  of `Cargo.toml`, and forbidding a proc-macro crate in that graph — so each future
  addition is argued rather than assumed.
- `ci-workflow`: the requirement "CI invokes every gate through `make`" justifies its
  single-line `run:` rule with "the crate may take no dependency, so the guard is
  std-only string matching". This change makes that premise false. The rule is right and
  stays; only its stated reason is rewritten, to the one that survives — a YAML parser
  as a dev-dependency to read four lines is not a trade worth making. No scenario
  changes.

## Impact

- **Code:** new `src/config.rs` and `src/state.rs`, both pure transformations plus a
  thin filesystem edge; `src/lib.rs` gains the module declarations and a test-only
  scratch-directory helper. No change to `src/main.rs` — nothing reads configuration
  until `repo-resolution`.
- **Build:** `Cargo.toml` gains `toml`; `Cargo.lock` is regenerated and committed.
- **Docs:** `SPEC.md` → Overview (stack), Architecture (module map), Data layer →
  Resolution chain, Degraded states, and Herdr integration → Attributing an agent;
  `README.md` → Configuration; `AGENTS.md` → Architecture rules (both the spawn bullet
  and the never-write bullet, rewritten in place), Conventions, and Current repo state;
  `openspec/IMPLEMENTATION-ORDER.md` → the Phase 1 `plugin-config` row.
- **External:** none. No network, no registry submission, no sibling repository. The
  files written live under `HERDR_PLUGIN_STATE_DIR`, never inside `openspec/`.
