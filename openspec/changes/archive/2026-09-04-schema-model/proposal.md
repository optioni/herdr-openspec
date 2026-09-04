## Why

`repo-resolution` can find the repository; nothing yet can say what is *inside* a change.
The detail view's tab bar is the schema's artifact list in schema order, and the tasks tab
is one particular artifact of that list — so `changes-from-files` cannot build a `Change`,
and `detail-view` cannot render a tab, until something reads the schema. This is the
second row of Phase 2 in `openspec/IMPLEMENTATION-ORDER.md`.

It is also the row whose specification is wrong. `SPEC.md` says the tasks artifact "is the
one whose entry carries `role: tasks`". **There is no `role` key in the OpenSpec schema
format** — not in the vendored `openspec/schemas/tdd/schema.yaml`, not in the CLI's own
`spec-driven` schema, and not in the CLI's Zod definition of an artifact. Reading the real
format before writing the model is the point of this change.

## What Changes

- **A `schema` module resolves which schema applies** — a change's own
  `.openspec.yaml`, then the repository's `openspec/config.yaml`, then the CLI's default
  `spec-driven`. It reports *which* of the three answered, so the ordering is observable
  rather than inferred.
- **It loads `openspec/schemas/<name>/schema.yaml` and produces the ordered artifact
  list** — id and `generates` per artifact, in file order, which is the tab order.
- **It identifies the tasks artifact by the real rule, which `SPEC.md` gets wrong.** The
  artifact whose `generates` equals the schema's top-level `apply.tracks`, falling back to
  the artifact with id `tasks` when no `apply` block declares what it tracks. This is the
  CLI's own `findTrackedTasksArtifact`. On this repository both rules pick `tasks`, which
  is exactly why the scenarios use a schema where they disagree.
- **A YAML dependency is added: `yaml-rust2`.** Argued in `design.md` → Decisions against
  hand-parsing and against the four alternative crates. `serde_yaml` — which `SPEC.md`'s
  stack still names — is deprecated and unmaintained; `saphyr` pulls a proc macro into the
  normal build graph, which the live `plugin-build` requirement forbids.
- **Every failure degrades and is named.** A schema that is not vendored, a `schema.yaml`
  that cannot be read or understood, an artifact entry missing `id` or `generates`, and a
  schema with no tasks artifact each produce a usable value plus a problem string, in the
  shape `Config::problems` and `BinResolution::problems` already established.
- **`SPEC.md` is corrected** on `role: tasks`, on `openspec schema` (**there is no such
  command** — the real fallback is `openspec schema which <name> --json`, which returns the
  schema's *directory*), on the deprecated `serde_yaml`, and on the schema search order and
  default that it omits entirely. `openspec/IMPLEMENTATION-ORDER.md`'s `schema-model` and
  `changes-from-cli` rows are corrected to match.
- Not **BREAKING**: no manifest key, no `config.toml` key, and no keybinding moves.

### Scope beyond the roadmap row, declared

The roadmap row says "parse `openspec/config.yaml` for `schema:`". This change also reads a
change's own `.openspec.yaml` override and applies the `spec-driven` default, because both
are real behaviour of the CLI this plugin mirrors and because `SPEC.md`'s own list view and
degraded-states table are already **per-change** ("`learning-tool` declares schema
`outside-in-tdd`"). Without the override, the dashboard would render the repository
schema's tabs for a change that declares a different one — silently wrong, with nothing to
notice it. The roadmap row is corrected rather than the behaviour dropped.

## Non-Goals

- **No process spawn, no `cli` module, no CLI fallback.** `openspec schema which` needs the
  Phase 3 seam. This change hands that obligation to `changes-from-cli` on the roadmap and
  ships nothing that spawns. Unlike `repo-resolution`'s step 4, no injected hook is added:
  there is no ordered chain here whose position needs pinning. What it ships instead is
  `load_dir(dir, name)`, which takes a schema *directory* rather than a repository root —
  `openspec schema which` reports an absolute path outside the repository for a CLI-shipped
  schema, so a repository-rooted signature alone would leave Phase 3 re-deriving this
  module's whole error mapping. `design.md` → Decisions argues the asymmetry rather than
  leaving it unexplained.
- **No change enumeration.** Which directories are changes is `changes-from-files`.
- **No artifact *file path* resolution.** `generates` is exposed; turning it into `<id>.md`
  or `<id>/` under a change directory is `changes-from-files`.
- **No task parsing.** Identifying the tasks *artifact* is here; reading checkboxes out of
  it is `task-parsing`.
- **No caching.** A schema read is one small file; `SPEC.md` specifies session caching for
  the binary probe and for nothing else, and `live-refresh` owns re-reading.
- **No rendering, no tab bar, no markdown.** `detail-view` consumes this list.
- **No general YAML surface.** The module exposes a `Schema`, never a YAML value; the
  parser crate is an implementation detail no consumer sees.
- **No writes.** Nothing in this change creates, modifies, or removes a file.

## Capabilities

### New Capabilities

- `schema-selection`: which schema name applies to a repository and to a single change —
  the change override, the project configuration, the default, the blank and wrong-type
  fallbacks, and the rejection of a name that would escape `openspec/schemas/`.
- `schema-artifacts`: loading a named schema from disk, the ordered artifact list, the
  tasks-artifact rule, and the degraded outcomes for a schema that is missing, unreadable,
  invalid, or partly unusable.

### Modified Capabilities

- `plugin-build`: the declared-dependency-set requirement names exactly one crate and pins
  the resolved normal build graph package-for-package. Adding `yaml-rust2` changes both,
  and the change is required to argue the dependency in `design.md` — which is what that
  requirement exists to force. The no-proc-macro clause and the MSRV clause are unchanged;
  the MSRV clause gains the scenario it never had, because `yaml-rust2`'s `1.85.0` sits
  exactly at this crate's own floor — tied with `hashbrown`, `hashlink`, `toml` and four
  more, so no single crate is uniquely the tightest and the check reports the set.

  The delta is written as **REMOVED plus ADDED** rather than MODIFIED. To be precise about
  why, because the difference matters: a MODIFIED block *would* validate, provided every
  original scenario heading were kept verbatim — `openspec validate --strict` rejects a
  MODIFIED requirement that omits any scenario name the live spec still carries, so what the
  tool forbids is **renaming a scenario**, not this change. It was rejected because one of
  those headings is "The declared dependency set is exactly one crate", and keeping it would
  archive a live spec whose scenario is named "exactly one crate" while its own THEN asserts
  two — the stale-spec-text defect this repository's reviews exist to catch. The replacement
  is named without a count, so it never needs this treatment again, and it carries every
  clause and every scenario of the removed requirement plus one for the MSRV clause that had
  prose and no verifier. `plugin-build` keeps its two other requirements, so no capability is
  retired and no `retire_capabilities` marker is owed. One cosmetic rename rides along —
  "The dependency is genuinely needed" becomes "Each dependency…" — which would have been a
  second forced rename under MODIFIED and is a readability change here.

## Impact

- **Code:** new `src/schema.rs`; `src/lib.rs` gains `pub mod schema;` and nothing else.
  `testutil` is reused unchanged — `ScratchDir` plus `std::fs::write` covers every fixture
  these scenarios need, and `snapshot` already records directory entries. If one turns out to
  want a builder it is added there rather than inside a test module (tasks.md 1.5).
  `src/main.rs`, `src/config.rs`, `src/state.rs`, and `src/resolve.rs` are untouched —
  nothing consumes the schema until `changes-from-files`.
- **Build:** `Cargo.toml` gains `yaml-rust2 = { version = "0.12.0", default-features =
  false, features = [] }` (defaults off drops `encoding_rs`), and `Cargo.lock` moves — by
  four new package blocks, `arraydeque`, `foldhash`, `hashlink`, `yaml-rust2`, plus a
  modification to `hashbrown`, which is already in the lock as an `indexmap`-only resolution
  cargo never builds. The normal build graph gains those four **and** `hashbrown` — no proc
  macro, no C.
- **Docs:** four documents, not three. `SPEC.md` → Overview (the stack line), Data layer →
  Resolution chain ("Schema"), Degraded states, and Testing → Unit-tested modules;
  `openspec/IMPLEMENTATION-ORDER.md` → the Phase 2 `schema-model` row and the Phase 3
  `changes-from-cli` row; `AGENTS.md` → Current repo state, whose "first third-party
  dependency (`toml`)" becomes false; and **`openspec/config.yaml` → `context`**, which
  carries both disproved claims — `role: tasks` and `serde_yaml` — and is injected verbatim
  into every future change's artifact instructions, so leaving it stale would author the next
  four changes from the exact falsehoods this one exists to kill. It is the
  repository-owned half of the schema stack, unlike the graft-vendored
  `openspec/schemas/tdd/`, so editing it is allowed.
- **External:** none. No network at runtime, no sibling repository, no registry. The
  graft-vendored `openspec/schemas/tdd/` is **read** by the tests and never edited.
