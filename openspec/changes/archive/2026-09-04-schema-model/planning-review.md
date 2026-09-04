## Reviewed Artifacts

- `proposal.md`
- `specs/schema-selection/spec.md`
- `specs/schema-artifacts/spec.md`
- `specs/plugin-build/spec.md`
- `design.md`
- `tasks.md`

The finding pass was delegated to three independent reviewers, none of which wrote the
planning package and none of which was a fork of the planning session. Each received the
change directory, the repository's durable documents, the archived `repo-resolution` change
as the quality bar, the installed OpenSpec CLI's own source, and one slice of the review
list: capability coverage and scenario quality; design completeness, test boundaries and
test strategy — with an explicit instruction to **run** every shell command and try to make
each one fail; and task alignment, contradictions, and repository invariants. Each was given
a scratchpad path and told to append findings as it discovered them rather than only in a
final message — the mitigation `HANDOFF.md` asks for after two review rounds were lost to
`529 Overloaded`. All three reports survived and are the source of the table below. The
reviewers edited nothing; `git status --short` was empty at `fb52758` after each. Every
repair was made in this session, in the artifact that owns it.

They returned **5 CRITICAL, 26 WARNING, 21 SUGGESTION** across 52 findings. Every CRITICAL
and every WARNING is repaired below. Every SUGGESTION is applied except two that were
answered rather than adopted, both recorded at the end.

## Reviewed Against

- This repository HEAD: `fb52758ca3cb4b29d13050527a041188b6c42810`
  (`docs(schema-model): add proposal, specs, design, and TDD task list`) — the proposal, all
  three delta specs, design.md and tasks.md were committed at that point, before the
  reviewers were dispatched, so all three read a fixed tree.
- Sibling repository HEAD: Not applicable. `openspec`, `herdr`, `node` and `npm` are
  consumed as installed binaries, not as source siblings. The graft source
  `optioni/openspec-schemas` is vendored, read-only, and unmodified.
- Working tree: clean apart from this change's own planning directory. No source file is
  touched by this change yet; `src/schema.rs` does not exist.
- Repairs were committed as `87a300e` (`docs(schema-model): repair planning-review
  findings`), which touches only this change's five planning artifacts.

Environment facts confirmed during planning and re-confirmed independently during review,
on this machine (macOS 27.0.0, Rust 1.91.1, Herdr 0.8.2):

- **`make check` at HEAD: exit 0, 97.51% of 1806 lines**, matching the number `tasks.md` 1.1
  carries as its baseline. The floor is 80% and is not at risk.
- Three OpenSpec CLIs are installed under nvm: **1.11.0** (`v24.20.0`, the one on the
  workflow's `PATH`), 1.12.0 (`v24.18.0`) and 1.9.0 (`v24.19.0`). A reviewer re-verified
  every CLI fact in design.md → Context against 1.12.0 and found **no drift**, so none of
  them is a quirk of one release. design.md now says so.
- **No `role` key exists** in `ArtifactSchema`, `ApplyPhaseSchema`, or `SchemaYamlSchema`; in
  the vendored `tdd` schema; or in the CLI's bundled `spec-driven` schema. All three
  reviewers confirmed this independently.
- `findTrackedTasksArtifact` is byte-for-byte
  `tracks != null ? artifacts.find(a => a.generates === tracks) : artifacts.find(a => a.id === 'tasks')`.
- `openspec schema` has subcommands `which`, `validate`, `fork`, `init` and **no dump
  command**; `openspec schema which <name> --json` puts a four-key object on **stdout** and
  its "experimental" note on **stderr**, and its `path` is a schema *directory* — for
  `spec-driven`, `/…/node_modules/@fission-ai/openspec/schemas/spec-driven`, outside any
  repository.
- `Cargo.lock` at HEAD **already contains `hashbrown 0.17.1`** as an `indexmap`-only
  resolution cargo never builds.
- Every empirical claim in design.md → Decisions about `yaml-rust2` and the four rejected
  crates was reproduced in scratch crates: `saphyr` really does pull
  `thiserror → thiserror-impl (proc-macro) → syn/quote/proc-macro2` into the **normal**
  graph; `serde_yaml_ng` and `serde_norway` each drag eight crates including
  `unsafe-libyaml`; `yaml-rust2` with `default-features = false` drops `encoding_rs`. All
  fourteen adversarial parse cases behaved as design.md states.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | design.md Contracts; specs/schema-selection | **The most common real input had no contract branch, no scenario, and no matrix row**: a config file that exists, parses, and simply omits `schema:` — the ordinary shape of a repository that pins nothing. `schema_key`'s documented contract routed it to `Err`, so an implementer following it emits a spurious problem for the normal case. Empirically easy to fall into: `yaml-rust2` yields `Yaml::BadValue` for a missing key, whose `as_str()` is `None` — **identical through a string accessor to `schema: [a]`**, which must record a problem. | The `schema_key` doc gains the branch and states the discriminator explicitly: the wrong-type case is keyed on `!is_badvalue()`, never on `as_str().is_none()`. New scenario "A file that declares other keys but no `schema:` records no problem", with a matrix row and task 2.3a. The requirement prose now says the two must be distinguished and that the distinction is possible but not automatic. | design.md Contracts; specs/schema-selection; design.md matrix; tasks.md 2.1, 2.3a |
| CRITICAL | design.md matrix; specs/schema-artifacts | **Four scenarios asserted "exactly one problem is recorded" against `load` and `parse`, whose `Err` variants carry no `problems` field at all** — problems exist only on `ParsedSchema` and `SchemaResolution`, and only `resolve` turns a `LoadError` into one. No matrix row called `resolve`. Four spec clauses had no verifier, and a fifth conflated `LoadError::Invalid.reason` with a problem entry. | Every problem-count clause is now a two-part assertion — the `LoadError` variant from `load`, then `resolve(...).problems.len() == 1` and its text — and the pure-`parse` scenarios were reworded to assert the *reason*, which is what that function returns. Task 6.3 states the rule for the whole group. | design.md matrix; specs/schema-artifacts (invalid + not-vendored + unreadable requirements); tasks.md 6.3, 6.6 |
| CRITICAL | design.md Decisions, Open Questions, Contracts | **The no-hook argument's fallback position was impossible as specified.** It said Phase 3 "will call `load` again against the directory `openspec schema which` reports" — but that command returns an **absolute directory outside the repository** with no `openspec/` segment, and `load(repo, name)` builds `<repo>/openspec/schemas/<name>/schema.yaml`. Worse than a flat no: the `$XDG_DATA_HOME` tier *would* fit that signature and the package tier would not, so `changes-from-cli` would have got a function silently covering one of the two CLI tiers, and would have re-derived the `NotFound → NotVendored` / `IsADirectory → Unreadable` / `parse-Err → Invalid` mapping this change is the home for. | `load_dir(dir, name)` becomes the primitive and `load(repo, name)` one `join` onto it. Zero new scenarios — every existing `load` scenario exercises it — and the no-hook argument gets *stronger*: the seam is the signature rather than a closure. Recorded in Contracts, Decisions, Non-Goals, Open Questions, proposal.md, and task 6.10, which states why the directory is the parameter. | design.md Contracts/Decisions/Non-Goals/Open Questions; proposal.md; tasks.md 6.10 |
| CRITICAL | tasks.md 4.5, 4.6, 4.9, 4.11 | **Group 4 could not go green.** Three of its RED tasks asserted `Schema::tasks` while 4.9's GREEN said "read no other key except the top-level `name:`" — the tasks rule is 5.7's. So 4.11 ("run the group tests — green") was unsatisfiable, and both escapes were bad: pull group 5's rule into group 4 (dissolving the split and making 5.1–5.5 non-RED), or drop the assertions (losing 4.6's whole point, that a line scanner reads `tracks: fake.md` out of the block scalar). | The split is kept and the assertions moved. Group 4's preamble and 4.1 now restrict every test to `schema.name`, `schema.artifacts` and `problems`; new task **5.1a** opens group 5 with a second pass extending the flow-style and block-scalar tests to assert `Schema::tasks`. Three matrix rows lost their `tasks` clause; the header comment explains the split so it does not read as an oversight. | tasks.md header, group 4 preamble, 4.1, 4.5, 4.6, 5.1a; design.md matrix (3 rows) |
| CRITICAL | tasks.md 7.4; design.md Test Boundaries | **Task 7.4 destroyed `Cargo.lock`.** It removed a dependency from `Cargo.toml` and ran `cargo build` in the working tree, restoring only the manifest — but `cargo build` **rewrites the lock during resolution, before reaching the compile error the task waits for**. Verified in a scratch copy: removing `toml` deletes **fourteen** package blocks. Restoring only the manifest leaves `cargo build --locked` failing; repairing it re-resolves against the current registry, silently discarding what 1.6 verified, after which 7.5 re-runs `GRAPH`/`$DEPS`/`$MSRV` against a lock the change never inspected. An interrupt mid-task leaves the tree broken with no recovery step, and `git checkout`/`git restore` are forbidden because the tree holds other uncommitted work. A second defect compounded it: `git show HEAD:Cargo.toml` restores the *pre-change* manifest whenever group 1 is uncommitted, so the `yaml-rust2` leg becomes a no-op that still fails the build and records a pass for a step never performed. | A written `NEEDED` block that copies the crate to `mktemp -d` and does the whole experiment there, with a `grep -q "^$dep = "` precondition on each leg so a no-op removal is a failure rather than a pass. Run against this crate at HEAD: `"ok: removing toml breaks the build"`, then `"FAIL: yaml-rust2 is not declared; nothing to remove"`, exit 1, `git status --short` still empty. 7.4 now also asserts `git diff --quiet -- Cargo.lock` afterwards. The Test Boundaries row, which called the repository tree read-only while the old task wrote to it, now says so. | design.md Test Strategy (`NEEDED`), Test Boundaries, matrix; tasks.md 7.4; specs/plugin-build |
| WARNING | design.md `SPAWN` | The tree-wide half **exits 0 when `src/` is missing** — verified with the real BSD grep, which prints "No such file or directory" and returns 0 under `!`. The block's own comment warns about exactly this trap and guarded only the *other* half. | Three positive controls first: `[ -d src ]`, `grep -q 'pub mod schema' src/lib.rs`, `[ -f src/schema.rs ]`. Verified: the block now **fails at HEAD** and fails when run from a directory with no `src/`. | design.md `SPAWN`; tasks.md 7.2 |
| WARNING | design.md `SPAWN` | Two bare `! grep` lines with no `set -e`. As one block whose single exit status is inspected — which is how the design defines a command check, and what the matrix row cited — **only the last line's status survives**, so a tree-wide violation is masked by a clean module. `set -e` does **not** fix this: POSIX says it ignores a `!`-prefixed command (confirmed by running it). | Rewritten as `if grep ...; then exit 1; fi`. Verified five ways, including the specific masking case: a planted spawn in `src/state.rs` with a clean `src/schema.rs` now exits 1, where the old form exited 0. | design.md `SPAWN`; tasks.md 7.2 |
| WARNING | design.md `MSRV` | `FLOOR` was a hardcoded `(1, 85, 0)` — a second, unlinked copy of the crate's `rust-version` — while the scenario claims a *relationship* between the two. Lowering `rust-version` would leave the check silently over-strict; the failure message also hardcoded `> 1.85`, so the design's own negative control read wrong. Matching was by package **name** against a full-platform `cargo metadata` resolve, so a dev-dependency resolving a second version of a normal-graph package would be checked as though it were in the build. | Floor read from the root package's `rust_version`, `(name, version)` pair matching, the four supported triples, the floor formatted into the message, and two vacuity guards. Verified: prints `msrv ok: floor 1.85 from Cargo.toml; 12 …`; with the floor forced to 1.60 it exits 1 naming `hashbrown 0.17.1 declares rust-version 1.85.0 > 1.60`. | design.md `MSRV`; specs/plugin-build; tasks.md 1.8 |
| WARNING | design.md `GRAPH`; specs/plugin-build | `cargo tree` resolves the **host target only**, so "no proc macro anywhere in the graph" was stronger than the evidence — a reviewer built a probe with a `cfg(windows)` `thiserror` dependency and GRAPH passed. The practical hole is a Linux-only transitive dependency invisible on a macOS run. | The block loops over all four supported triples. **Not** the reviewer's proposed `--target all`: verified that on this graph it additionally lists `syn`, `quote`, `proc-macro2`, `serde_derive` and `unicode-ident` through `serde_core`'s optional `derive` feature, none of which is ever built, making an exact-list check unsatisfiable. A Windows-only dependency still passes — correctly, since `platforms = ["macos", "linux"]` — and that edge is now stated rather than implied. | design.md `GRAPH`; specs/plugin-build; tasks.md 1.7 |
| WARNING | design.md Risks | Two mitigations promised a gate that does not exist: "the next bump fails a gate" and "resolution point: whenever `plugin-build`'s graph check fails after a `cargo update`". `make check` is `fmt-check lint test coverage`, CI runs exactly those, and the six blocks run once, here. A `cargo update` fires nothing. | Both bullets reworded to the truth, plus a paragraph under "Where each check stops" saying none of the six is a standing gate and that the committed `Cargo.lock` is what pins the resolution. The resolution point is now "the next change that alters the dependency set", with the honest alternative named — a `deps-check` target inside `make check`, which is a `quality-gates` change and out of scope here. Task 10.1 records the same. | design.md Risks, Test Strategy; tasks.md 10.1 |
| WARNING | design.md Contracts/Decisions | The escape hatch justifying the `select`/`load` split was unreachable from the caller it names: `read_file` was declared with no visibility marker, so `changes-from-files` could not build a `FileText` and could not call `declared_name` — leaving it with `select`, which re-reads both files on every call, which is the cost the Risks section concedes. | `read_file` is `pub(crate)`, alongside `FileText` and `declared_name`, with the reason in the doc comment: `changes-from-files` is in this crate and is exactly the caller that wants to read `openspec/config.yaml` once. Task 6.11 checks it. | design.md Contracts; tasks.md 6.11 |
| WARNING | design.md Contracts/Decisions | `Schema::tasks` as a cloned `Artifact` was argued only against an index — the weakest of the three options — and never against a `tasks_id: Option<String>` key, which is immune to filtering *and* reordering. The clone also has its own desync in the other direction: after a consumer filters `artifacts`, `tasks` can hold a value no longer in the list, and no invariant was stated. | The invariant is stated — `tasks`, when `Some`, always equals one element of `artifacts`, and a consumer that filters must re-derive or clear it — and the id-key alternative is named and rejected on its actual cost: every scenario here asserts a whole `Artifact` with `assert_eq!`, and a bare id pushes a lookup into each of them. | design.md Contracts, Decisions |
| WARNING | design.md Test Strategy | Three matrix rows cited commands never written, under a heading claiming every command had been written and run — including the one that mutates `Cargo.toml`, the most in need of an exact recipe. | `BUILD` and `NEEDED` written out and run; the proc-macro/`encoding_rs` grep folded into `GRAPH` with `grep -qxE` (whole-field match, so a package merely containing one of those names is not a false failure) rather than left as prose whose polarity is easy to invert. | design.md Test Strategy; tasks.md 1.7, 7.3, 7.4 |
| WARNING | specs/schema-selection; design.md matrix; tasks.md 3.2 | The "No change directory is supplied at all" scenario's discriminating clause discriminated nothing, three ways: no file was actually planted in the WHEN; "beside the repository root" is a sibling directory no implementation — correct or buggy — reads, while the plausible bug is `change_dir = repo`; and the planted value was unspecified, so a fixture repeating the project's name would be green against everything. | The fixture plants `schema: planted` — a third name appearing nowhere else — in **both** `<scratch>/repo/.openspec.yaml` and `<scratch>/.openspec.yaml`, asserts the whole `Selection` equals the project's, and never `planted`. Both plants sit inside the `ScratchDir`, so drop still removes them, which the old "beside" wording did not guarantee. | specs/schema-selection; design.md matrix; tasks.md 3.2 |
| WARNING | specs/schema-selection | Every selection scenario asserted **exactly one** problem, always from one file. Nothing made both files unusable at once, so first-wins, last-wins, and an `Option<String>` in place of the `Vec` the requirement invokes all passed the whole capability. | New scenario "A fallback at both sources records both problems, in order", with a matrix row and task 2.3b. The requirement prose now says problems are appended in source order. | specs/schema-selection; design.md matrix; tasks.md 2.3b |
| WARNING | specs/schema-artifacts | Nothing pinned that selection's problems survive into the composed result. An implementation whose `resolve` rebuilt `problems` from the load step alone was green across all 47 scenarios, and the user silently lost the "your `.openspec.yaml` is broken" message that is the whole reason selection records one. | New scenario "A composed resolution carries the selection's problems as well as the load's" — invalid change file plus a project schema that is not vendored, asserting two problems with the selection's first — plus a clause in the requirement prose and task 6.4. | specs/schema-artifacts; design.md matrix; tasks.md 6.1, 6.4 |
| WARNING | specs/schema-artifacts | "Artifact order is the file's order" was green against two of the three transformations its own requirement forbids **by name**: a `requires` topological sort (the fixture had no edges, so any stable sort reproduces input order) and an insertion-ordered de-duplication (no id repeated). The vendored `tdd` schema really does carry `requires` edges. | The fixture gains edges that disagree with file order (`zeta` requires `middle`, `alpha` requires `zeta`) and a fourth entry repeating the id `zeta`; the THEN names all four rejected outputs including the topological order. | specs/schema-artifacts; design.md matrix; tasks.md 4.2 |
| WARNING | specs/schema-artifacts | The only test against a real schema asserted "contains", so a parser dropping `design` or `planning-review` — the two artifacts with the longest `instruction:` block scalars, exactly what a block-scalar defect eats — passed every clause. | The assertion is now the **exact** list `[proposal, specs, design, tasks, planning-review]` in order, plus the tasks artifact selected through the file's real `apply.tracks` (the only place the tracks rule meets a real schema). design.md → Risks records that a graft update adding an artifact turns this red **on purpose**, and that this is a one-line fixture update with a real question behind it. | specs/schema-artifacts; design.md matrix and Risks; tasks.md 6.5 |
| WARNING | specs/schema-artifacts; proposal.md | Two rules were asserted as **CLI parity** where the CLI does the opposite: a wrong-typed `tracks` yields *no* tasks artifact in the CLI (and is rejected at parse time), not an id fallback; and `description` and `template` are **required** in `ArtifactSchema`, so the plugin's leniency about them accepts schemas the CLI rejects. Both are good design; the parity claim was the defect, and it is the same class of error — a durable document stating a false fact about the CLI — that this change exists to fix. | Both requirements now state the divergence explicitly, say why the plugin degrades instead, and record the consequence: the plugin's *usable* is strictly wider than the CLI's, so `changes-from-cli` must not assume that the plugin having loaded a schema means the CLI will accept it. | specs/schema-artifacts (both requirements) |
| WARNING | proposal.md; tasks.md group 9; `openspec/config.yaml` | **The highest-leverage document was not scheduled.** `openspec/config.yaml` → `context` carries *both* claims this change disproves — "the artifact with `role: tasks` holds the task checklist" and `serde_yaml` in the stack line — and that block is injected verbatim into **every future change's artifact instructions**. Correcting `SPEC.md` while leaving it would author `changes-from-files`, `task-parsing`, `detail-view` and `tasks-tab` from the exact falsehoods being fixed. | New task **9.7a** rewrites both clauses, with a note that this file is the repository-owned half of the schema stack and editing it is allowed where editing `openspec/schemas/tdd/` is not. 9.1's re-read list and 9.10's falsehood checklist both extend to four documents, and 9.1 gains two greps that mechanically prove nothing was missed. Added to proposal.md → Impact. | proposal.md; tasks.md 9.1, 9.7a, 9.10 |
| WARNING | tasks.md 1.6 | "adds exactly the five packages … plus their checksums" is wrong. Verified by generating the lock in a scratch copy: **four** new package blocks (`arraydeque`, `foldhash`, `hashlink`, `yaml-rust2`); `hashbrown` is already in the lock as an `indexmap`-only resolution and is only *modified*, gaining `dependencies = ["foldhash"]` and no new checksum. An implementer checking this literally reports a false failure or forces a version bump the change does not need. | 1.6 now names four, explains why `hashbrown` differs, and gives two mechanical commands in place of an eyeball check: no `-` line in the lock diff, and `+name = ` lines exactly those four. proposal.md → Impact corrected to match. | tasks.md 1.6; proposal.md |
| WARNING | tasks.md 6.7 (old numbering) | The persistence gate ran **before** the GREEN that would introduce the thing it forbids. `load` and `resolve` are precisely where a `OnceLock` would appear — `repo-resolution`'s `BinCache` is that shape one change over — so the gate passed vacuously and nothing later in the group could catch it. | Moved after the GREEN and the REFACTOR (now 6.12) and given a command that a `OnceLock` added in 6.10 actually fires on: `grep -nE 'static \|OnceLock\|LazyLock\|RefCell\|Cell<\|Mutex' src/schema.rs` must be empty. The recorded reasoning about `select`/`load` is kept. | tasks.md 6.12; header comment |
| WARNING | design.md Test Boundaries | The table called this repository's tree "real, read-only" while task 7.4 edited `Cargo.toml` and caused `cargo` to rewrite `Cargo.lock`, and the matrix row for the same scenario said "reverted in the working tree" — the design said both things. This is the one "no task invents a boundary the table omits" violation in the change. | The row now states exactly what each check touches: `NEEDED` runs in a copy under `std::env::temp_dir()`, `BUILD` writes only to the gitignored `target/`, and group 1's manifest edit is the change rather than a check. | design.md Test Boundaries, matrix |
| WARNING | tasks.md 7.3 | Never checked `scripts/build.sh`'s **exit status**, and `target/release/herdr-openspec` survives from any earlier build — so on every run after the first, the assertion is satisfied by a leftover regardless of what the script did, and `build.sh`'s real failure path (`error: cargo not found`, `exit 1`) sails past. | The `BUILD` block deletes the binary first and asserts the script exits 0. The clause is carried into the spec scenario's THEN too, where it was also absent. | design.md `BUILD`; specs/plugin-build; tasks.md 7.3 |
| WARNING | tasks.md header | The concentration-point audit attributed "coverage counts the whole crate" to 6.7, the persistence gate, which says nothing about keeping a composition thin. The task that does is 6.8 (now 6.10), supported by the public-surface check. This is a verbatim repeat of a finding the archived `repo-resolution` review made about the same header. | Attribution corrected to the GREEN task and the REFACTOR that checks it. | tasks.md header comment |
| WARNING | specs/plugin-build; design.md Boundaries; tasks.md 1.4 | The requirement demands "an explicit feature list"; 1.4 said "declare no feature list" and then contradicted itself inside the same sentence with "an empty list is the explicit statement". Two different manifest texts, and `cargo metadata` reports `[]` for both — so no gate would ever catch a live requirement the manifest does not satisfy on its face. | One answer everywhere: `features = []` written out. The spec scenario adds a clause saying the check for it is reading `Cargo.toml`, precisely because `cargo metadata` cannot tell the two apart. | specs/plugin-build; design.md Boundaries; tasks.md 1.4; proposal.md |
| WARNING | proposal.md; design.md Boundaries; tasks.md 1.5 | Three artifacts disagreed on whether `testutil` gains a helper: the proposal said it does, the design's Boundaries row named a new piece and then said nothing is added, and 1.5 said no helper is needed. `repo-resolution`'s review repaired the mirror of this. | 1.5's answer — reuse unchanged, with a well-placed escape hatch — is adopted everywhere. The Boundaries row is now a `testutil` row naming the five existing helpers and the one place a new one may be added. | proposal.md; design.md Boundaries; tasks.md 1.5 |
| WARNING | tasks.md 9.5 | After 9.5's rewrite, `SPEC.md` would still name `schema::artifacts`, a function this change does not create. 9.5 said to correct the *mechanism*, never the identifier, and every sibling entry in that list is a real identifier — so this would have become the only stale name in it. 9.10's checklist did not cover it either. | 9.5 now names `schema::select` and `schema::parse` and says why; 9.10 gains the corresponding item. | tasks.md 9.5, 9.10 |
| WARNING | design.md Contracts | Behaviour for an **absent or non-string `name:`** key was undefined, while roughly a dozen `parse` fixtures omit the key and assert "no problem is recorded". If absence counted as a mismatch, half the suite would go red; if not, the mismatch rule silently only fires when the key is present. The design left the implementer to pick, and half the suite depended on the pick. | Stated in Contracts and in the requirement: only a **present, non-blank string** that differs is a disagreement. The mismatch scenario gains three companion fixtures — no key, blank, mapping-valued — each loading with no problem, and task 6.7 says why they are not padding. | design.md Contracts; specs/schema-artifacts; design.md matrix; tasks.md 6.7 |
| SUGGESTION | specs/schema-selection; specs/schema-artifacts | No scenario covered "no `openspec/` directory at all", which `openspec/config.yaml` → `rules.specs` names as a required boundary. | One clause in each capability: the default is still reached for a root with no `openspec/`, and a repository with no `openspec/schemas/` yields **not vendored** rather than *unreadable*. | specs/schema-selection; specs/schema-artifacts; design.md matrix; tasks.md 2.3, 6.2 |
| SUGGESTION | specs/schema-artifacts | An `apply:` value that is not a mapping was unspecified — and indexing a scalar node is exactly the shape the sibling requirement worries about. | A third fixture in the null-versus-absent scenario: `apply:` as a bare scalar also falls back to the id, without panicking. | specs/schema-artifacts; design.md matrix; tasks.md 5.4 |
| SUGGESTION | specs/schema-selection | A scenario clause asserted something the API cannot observe — "no path outside the repository is read or stat-ed", justified by restating the previous THEN. The archived review removed three clauses of exactly this shape. | Dropped, with the reason recorded in the matrix row so it is not reintroduced. | specs/schema-selection; design.md matrix |
| SUGGESTION | specs/schema-selection | The separator set was undefined for `\`, which is a legal filename character on both supported platforms. | Named explicitly: `/` and `\` both rejected, mirroring the CLI's `[\\/]+` split, with the CLI's drive-letter rule correctly out of scope. `a\b` added to the rejected list. | specs/schema-selection; design.md matrix; tasks.md 2.6, 2.9 |
| SUGGESTION | design.md matrix; tasks.md 2.6 | The NUL fixture would not have worked: verified that a **literal** NUL in a plain YAML scalar is silently truncated, so `schema: a\0b` yields the perfectly legal name `a`, the test goes red for an unrelated reason, and the likeliest repair is to weaken the assertion. | The exact fixture text is named — the YAML double-quoted escape `schema: "a\0b"`, i.e. the Rust literal `"schema: \"a\\0b\"\n"` — in both the matrix row and the task. | design.md matrix; specs/schema-selection; tasks.md 2.6 |
| SUGGESTION | design.md Contracts | `pub fn parse` returns `Err(String)` — the shape the three-variant `LoadError` argument rejects two paragraphs later. | Stated: `parse`'s `Err` is a display string that only ever becomes `LoadError::Invalid { reason }` and is never to be matched on; the variants are what a consumer branches on. | design.md Contracts |
| SUGGESTION | design.md Risks | The duplicate-key risk named only `schema.yaml`, while the same parser reads the two files a user actually hand-edits, where the effect is a *silent fall-through* rather than a visible `Invalid`. | Extended to `openspec/config.yaml` and `.openspec.yaml`, naming the different degrade. | design.md Risks |
| SUGGESTION | design.md Decisions | The one alternative that removes the dependency entirely — `openspec status --change <n> --json` already returns the ordered artifact list per change — sat in the Context table as a fact and was never evaluated, while Decisions attacked hand-rolling and four rival crates. | A paragraph dispatches it on `SPEC.md`'s dual-source model: the CLI costs 200–400ms, does not exist until Phase 3, and may be absent entirely, so the file tier is the one that must exist. It also sharpens what the CLI tier is *for* — corrector as well as fallback. | design.md Decisions |
| SUGGESTION | design.md Contracts | "unreadable — the path exists but the bytes could not be obtained" points an implementer at *invalid* for a non-UTF-8 file, whose bytes **were** obtained; only the decode failed. The two have different `changes-from-cli` consequences. | Redefined as "could not be read as UTF-8 text, whether because of an I/O error or a decode failure", with the reason stated. | specs/schema-artifacts |
| SUGGESTION | specs/plugin-build | "`yaml-rust2`'s `1.85.0` is the tightest of them" is unverified by the check and is in fact a **tie** — nine packages sit at 1.85. | The clause now reports the set at the floor rather than naming one crate, and the `MSRV` script prints it. The Risks bullet is corrected the same way. | specs/plugin-build; design.md Risks |
| SUGGESTION | proposal.md | The REMOVED+ADDED argument read as "MODIFIED was impossible". A reviewer reproduced it: MODIFIED **would** validate if both original scenario headings were kept, so the tool forbids *renaming a scenario*, not this change — and one of the two forced renames was a cosmetic one riding along. | Rewritten to say exactly that, and to rest the decision on the single genuinely stale heading rather than on a tool rejection. | proposal.md |
| SUGGESTION | specs/plugin-build | REMOVED+ADDED moves the requirement to the **end** of the archived capability file, which a later reader will see as a large block move on top of the rename. | One line in the REMOVED block's Migration note, so the move is expected rather than read as a merge artifact. | specs/plugin-build |
| SUGGESTION | tasks.md 9.8 | 9.8 named two false `AGENTS.md` sentences; a third false clause sits inside the second one — "`changes-from-files` still needs both `schema-model` and `task-parsing`, neither of which has landed". | Named in 9.8, so it is not caught only by luck. (9.8's *replacement* text was independently verified correct against the Phase 2 table: `task-parsing` is genuinely next.) | tasks.md 9.8 |
| SUGGESTION | tasks.md group 6 | The group lands the module's whole public surface with five named consumers and had no contract gate — the schema's rule is broader than the project's, which only names the manifest and the config format. `repo-resolution` added gates for the same situation. | New **6.8 CHECK**, placed *before* the GREEN that freezes the shape, re-reading `SPEC.md` → Resolution chain → "Schema" against the API — the direction that matters here, because group 9 rewrites that paragraph *from* this API. Recorded as forward-looking rather than a break check. | tasks.md 6.8; header comment |
| SUGGESTION | tasks.md 1.7 | The proc-macro half of the graph check was prose only, in a change where every other command is spelled out precisely because the spelling is where these checks go wrong. | Written into the `GRAPH` block with `grep -qxE` and the `if`-form exit semantics; 1.7 now says to run the block verbatim and names the two spellings that must not be "simplified". | design.md `GRAPH`; tasks.md 1.7 |
| SUGGESTION | tasks.md groups 8 and 9 | Group 8's reviewer sees a diff that does not yet contain group 9's document rewrites — structurally the same surface where the archived review found its one wrong-replacement-text defect. | 8.1's brief now includes the planned rewrites quoted alongside the current text of all four documents, and 9.10 asks for a fresh-eyes re-read of each rewritten entry. | tasks.md 8.1, 9.10 |
| SUGGESTION | tasks.md 7.6 | The leak check can fail for a directory left by an *earlier* session, reading as this change's fault. | It now lists the offenders and checks the embedded pid against this session's, and also confirms `NEEDED`'s own `mktemp -d` tree was removed. | tasks.md 7.6 |
| SUGGESTION | tasks.md 10.7 | The `PATH` export hardcodes nvm `v24.20.0`, which will rot. | Resolve the version rather than hardcoding it, and confirm `openspec list` works before relying on it. | tasks.md 10.7 |

**Confirmed clean, with no repair needed** — recorded so a later reader knows these were
checked rather than skipped:

- **Every factual claim about the OpenSpec CLI holds.** All three reviewers verified them
  independently, two by reading `dist/` and one by running the binary with the streams split;
  one also re-ran the whole set against a second installed version (1.12.0) and found no
  drift. No CRITICAL arose from this section, which was the largest risk in the change.
- **The central design decision survived a direct attack.** Every empirical claim behind
  `yaml-rust2` was reproduced in scratch crates and a scratch Rust program, including the
  `saphyr` proc-macro constraint, the `encoding_rs` drop, the transitive counts, the MSRVs,
  and all fourteen adversarial parse cases. Nothing in Decisions was refuted.
- **Capability coverage.** Both new capabilities have delta files at the right paths; the one
  modified capability does too; nothing appears only in design.md or tasks.md. All six live
  specs were read in full to test "Modified Capabilities: none else" — `quality-gates` and
  `ci-workflow` genuinely owe no delta, since neither names the dependency set.
- **Scenario form.** 50 scenarios after the repairs; every requirement has at least one; every
  scenario heading is exactly four hashes; SHALL/MUST throughout. The REMOVED block correctly
  has none.
- **The verification matrix is 1:1 with the scenarios**, script-verified in both directions
  before and after the repairs — now 50 rows against 50 scenarios, no duplicates, no orphans —
  and every scenario is named in a task, also script-verified.
- **The REMOVED + ADDED mechanics are sound.** The REMOVED block names the live requirement by
  its byte-identical heading; the ADDED requirement carries every clause and every scenario of
  the removed one plus the new MSRV scenario; archiving a scratch copy confirms `plugin-build`
  keeps three requirements, is not retired, and needs no `retire_capabilities` marker.
- **Group structure**: exactly one valid kind marker per group; behavior groups preserve
  RED → GREEN → REFACTOR with no implementation before a failing test; operational groups put
  their CHECK first; no RED task for plumbing; no `parallel-after` marker, correctly, since
  groups 2–6 share one file.
- **Sequencing holds after the group 4/5 repair.** No group-5 test is green after group 4 — but
  narrowly: two of them assert `tasks.is_none()`, which is trivially true of group 4's code,
  and stay red only because they also require a problem. Tasks 5.3 and 5.5 now say the
  problem-count clause is what keeps them red, so it is not later trimmed as noise.
- **None of this repository's catalogued verification traps is reintroduced**: no
  `env PATH=… cargo`, no `paste -sd:`, no `printf '%s'` dropping a line, no bare python
  `assert` (verified immune to `PYTHONOPTIMIZE=1`), no `cargo build --offline` standing in for
  `--locked`, no `stat -c`, no `sed -i` without an argument, no `grep -P`, and no
  `! grep -rn '"openspec"' src/` — correctly declined, and confirmed false at HEAD. Two *new*
  traps of the same family were found and fixed: the unguarded tree-wide grep and the
  `!`-prefixed command that `set -e` ignores.
- **No timing-shaped test.** Nothing sleeps, polls, or reads the clock; the containment guards
  compare two `std::fs::Metadata` snapshots, which is a value comparison.
- **Repository invariants**: no production write anywhere, let alone inside `openspec/`; no
  process spawn and no early `cli` module; the 80% coverage floor neither lowered, waived, nor
  excluded; no task edits the graft-vendored `openspec/schemas/tdd/` or `.claude/agents/` —
  one test *reads* the former and task 7.7 proves neither moved; no PRD non-goal crossed;
  correctly not marked **BREAKING**.
- **Every group-9 claim about what a document currently says was verified line by line** against
  `SPEC.md`, `AGENTS.md`, `openspec/IMPLEMENTATION-ORDER.md` and `openspec/config.yaml`, and
  all are accurate — including that the Mermaid graph already carries
  `schema-model --> changes-from-files --> changes-from-cli`, so 9.7's decision *not* to add a
  redundant direct edge is right. The one gap was a missing document, repaired above.
- **`testutil::snapshot` already records directory entries** (`repo-resolution` added that), so
  the two containment guards can see a `create_dir_all` and no extension is owed here.
- **`make check` at HEAD is 97.51% of 1806 lines**, matching what tasks.md 1.1 carries.

## No Remaining Implementation-Blocking Gaps

None remain. All five CRITICALs and all twenty-six WARNINGs are repaired in the artifact that
owns each, and nineteen of twenty-one SUGGESTIONs are applied. Every repaired shell command
was executed after the repair, with a negative control:

- `SPAWN` was verified five ways — it now fails at HEAD, passes on a simulated post-change
  tree, fails on a plant in `src/schema.rs`, fails on a plant in `src/state.rs` with a clean
  `src/schema.rs` (the masking case the old form got wrong), and fails when run from a
  directory with no `src/`.
- `GRAPH` matches on all four supported triples against a probe crate carrying both
  dependencies, and mismatches against this crate at HEAD naming the difference.
- `MSRV` prints its count line and exits 0; with the floor forced to 1.60 it exits 1 naming
  `hashbrown 0.17.1`.
- `DEPS` exits 0 against the probe, exits 1 with `unexpected normal deps: ['toml']` against
  this crate at HEAD, and still exits 1 under `PYTHONOPTIMIZE=1`.
- `NEEDED` reports `ok: removing toml breaks the build`, then
  `FAIL: yaml-rust2 is not declared; nothing to remove`, exits 1, and leaves
  `git status --short` empty.

Two repairs were made *against* a reviewer's proposal rather than by adopting it, both because
the proposal was checked and found wrong:

1. **`cargo tree --target all` was rejected** as the fix for the host-target hole. Verified
   that on this graph it additionally reports `syn`, `quote`, `proc-macro2`, `serde_derive` and
   `unicode-ident` — optional resolutions cargo never builds, reached through `serde_core`'s
   `derive` feature — which would make the exact-list check unsatisfiable. Looping the four
   supported triples gives the same coverage with an exact list, and all four produce an
   identical set.
2. **`set -e` was rejected** as the fix for the two-line `SPAWN` block. POSIX defines `set -e`
   to ignore a command prefixed with `!`, confirmed by running it: the masking case still
   exited 0 with `set -e` added. The `if grep …; then exit 1; fi` form is what actually fixes
   it.

## Deferred Non-Blocking Notes

Three, all with their resolution point already recorded:

1. **None of the six command checks is a standing gate.** `make check` is
   `fmt-check lint test coverage` and CI runs exactly those, so `DEPS`, `GRAPH`, `MSRV`,
   `BUILD`, `NEEDED` and `SPAWN` run once, in this change's task list. What pins the resolution
   afterwards is the committed `Cargo.lock`. Wiring them into `make check` would change the
   `quality-gates` capability, which this change otherwise leaves alone. Recorded in design.md
   → Test Strategy and Risks and in task 10.1; resolution point is the next change that alters
   the dependency set, or a deliberate `quality-gates` change if that proves too weak.
2. **The vendored-schema test is coupled to a graft-vendored file on purpose.** Its exact
   artifact-list assertion will go red if `optioni/openspec-schemas` adds or renames a `tdd`
   artifact. That is the intended coupling — it is the only test proving the model matches the
   file the plugin actually meets — and design.md → Risks says so, so a future red is read as
   a one-line fixture update with a real question behind it rather than as a flake.
3. **The plugin's *usable* is strictly wider than the CLI's.** A schema whose artifacts omit
   `description`/`template`, or whose `apply.tracks` is wrong-typed, loads here and is rejected
   there. Both divergences are deliberate — degrading beats failing closed for a read-only
   viewer — and both are now stated in `specs/schema-artifacts` rather than implied.
   `changes-from-cli` is the change that must not assume the reverse, and the requirement says
   so where its implementer will read it.
