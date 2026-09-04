<!-- No outer-loop acceptance group: design.md → Test Strategy records why. The change's
     outermost surface is a library API — `src/main.rs` is untouched and no caller
     consumes `Config` until `repo-resolution` — so an acceptance test would drive an
     entry point that ships to nobody. The unit tier reaches every collaborator these
     functions have; the one it cannot reach, Herdr's injected environment, is the live
     check in group 6.

     Three concentration points from openspec/config.yaml do not apply to this change and
     are recorded here so their absence is deliberate, not overlooked: no view is added,
     so "views perform no I/O" and "view tests run at 60 and 120 columns" have nothing to
     bind to; and the `Change` type does not exist yet, so `from_files`/`from_cli`
     agreement is not at stake. The four that do apply are scheduled: no spawn outside
     `cli` (6.2, 6.3), nothing written inside `openspec/` (5.2, 5.9), an absent case for
     every external dependency (2.1, 3.1, 5.1), and the 32-character agent-name cap
     (4.1).

     None of the command checks in this file may be committed as a Rust test. A test
     that reads `Cargo.toml` and asserts `toml` is listed proves only that the string was
     typed twice. -->

## 1. Dependency and module skeleton
<!-- kind: operational -->

- [ ] 1.1 CHECK: Confirm the crate has no third-party dependency today — `cargo metadata --no-deps --format-version 1` reports an empty dependency list and `Cargo.lock` holds exactly one `[[package]]` — and that `make check` is green at HEAD, so any later failure belongs to this change
- [ ] 1.2 CHECK: Read the current stable `toml` version from the registry rather than from this file — `cargo info toml` — and record what it reports. `1.1.5` (MSRV 1.85, matching `Cargo.toml`'s `rust-version`) is what design.md argued; if the registry has moved, take the newer patch or minor and note it here rather than silently pinning the old number
- [ ] 1.3 CHANGE: Add to `[dependencies]`: `toml = { version = "<recorded version>", default-features = false, features = ["std", "parse", "display", "serde"] }`. All four features are load-bearing — `parse` reads, `display` writes `agent-names.toml`, `serde` gates `toml::Table` itself, `std` gates the rest — and defaults are off so what compiles is what design.md argued
- [ ] 1.4 CHANGE: Add empty `src/config.rs` and `src/state.rs` and declare `pub mod config;` and `pub mod state;` in `src/lib.rs`, leaving the existing invocation-parsing code untouched
- [ ] 1.5 CHANGE: Add a `#[cfg(test)]` test helper (in `src/lib.rs` or a `testutil` module) that creates a uniquely named directory under `std::env::temp_dir()` and removes it on drop. Derive the name from the process id and a `static AtomicUsize`, not from the clock and not from a random source, so a test can predict the paths it will see. No `tempfile` dependency — design.md → Decisions argues why
- [ ] 1.6 VERIFY: `cargo build --locked` exits 0 with the regenerated `Cargo.lock` committed, and the normal-kind dependency list is exactly `["toml"]` — spec scenario "The declared dependency set is exactly one crate". Filter it, do not eyeball it: `cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; p=json.load(sys.stdin)["packages"][0]; print(sorted(d["name"] for d in p["dependencies"] if d.get("kind") is None))'`
- [ ] 1.7 VERIFY: Record the real build graph with `cargo tree -e normal` and confirm no proc-macro crate is in it — no `syn`, `quote`, `proc-macro2`, or `serde_derive`. Expected transitive set: `serde_core`, `serde_spanned`, `toml_datetime`, `toml_parser`, `toml_writer`, `winnow`. `Cargo.lock` will hold more entries than that; those are optional resolutions cargo never builds, which is why no check counts lock entries
- [ ] 1.8 VERIFY: Exactly one target of kind `bin`, named `herdr-openspec`, and `/bin/sh scripts/build.sh` leaves it executable at `target/release/herdr-openspec` — spec scenario "Exactly one binary target is produced at the release path". `cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; t=[x for p in json.load(sys.stdin)["packages"] for x in p["targets"] if "bin" in x["kind"]]; assert [x["name"] for x in t]==["herdr-openspec"], t'`

## 2. Configuration directory resolution
<!-- kind: behavior -->

- [ ] 2.1 RED: Write failing unit tests in `src/config.rs` for `config_dir`, taking a `&dyn Fn(&str) -> Option<String>` over a fixture map — never `std::env::set_var`, which is `unsafe` in edition 2024 and races parallel tests. Cover spec scenarios "Herdr supplies the directory" (variable wins; identical result with `HOME` absent), "Run outside a Herdr pane" (`$HOME/.config/herdr/plugins/config/herdr-openspec`), "An empty environment variable is not a directory" (`""` falls through rather than yielding an empty or relative path), and "Neither variable is available" (`None`)
- [ ] 2.2 Confirm the failures come from the missing function, not a broken fixture — `cargo test --all-features config::` names those tests and none fails to compile the harness for an unrelated reason
- [ ] 2.3 GREEN: Implement `config::config_dir`. Read `HERDR_PLUGIN_CONFIG_DIR` first, treating an empty or whitespace-only value as unset; otherwise join `HOME`; otherwise `None`. No filesystem access, no `std::process::Command`
- [ ] 2.4 GREEN: Implement `config::load_from_env` as the single binding of the lookup to `std::env::var`, and nothing else. Keep it to one expression so the untestable residue stays one line
- [ ] 2.5 RED then GREEN: Add the test that `load_from_env` agrees with `load(config_dir(&lookup), &lookup)` for a lookup built from the real environment, so the one wrapper is not the one thing nothing exercises
- [ ] 2.6 REFACTOR: Extract the shared "read a variable, treat empty as unset" helper if `state` will want it too; if nothing warrants changing, say so in 2.7
- [ ] 2.7 Run the group tests — `cargo test --all-features` green, no regressions to the existing invocation tests

## 3. Reading `config.toml`
<!-- kind: behavior -->

- [ ] 3.1 RED: Write failing unit tests in `src/config.rs` for `load(dir, env)` against a scratch directory, one per spec scenario, with names derived from them: "Every key is set", "The file does not exist", "The directory does not exist", "An empty file is not a malformed file", "Only one key is set", "Unrecognised keys are ignored", "The file is not valid TOML", "One key has the wrong type, the rest survive", "A negative count is not a count", "The file cannot be read", "A tilde path is expanded", "A `$HOME` path is expanded and a bare tilde is the home directory", "An unexpandable value is returned verbatim", "An empty value is an absent value", "A path that does not exist is still returned", "A configuration read leaves the tree byte-identical", "A missing configuration directory stays missing". The empty-file and malformed-file tests must assert on `problems` as well as on the values — that assertion is the only thing distinguishing them, and without it both are green whichever way the code goes
- [ ] 3.2 RED: For "The file cannot be read", create `config.toml` as a *directory*; for the byte-identical test, snapshot the listing, each file's bytes, and each file's modification time before and after two loads and assert no entry, no byte, and no mtime moved and no temporary file appeared
- [ ] 3.3 Confirm every failure is the missing behaviour rather than a scratch-directory helper that never created anything — assert the fixture exists inside at least one test before the call under test
- [ ] 3.4 GREEN: Implement the `Config` type — `openspec_bin: Option<PathBuf>`, `agent_kind: String`, `archived_count: usize`, `problems: Vec<String>` — with a `Default` giving absent, `claude`, `5`, and no problems
- [ ] 3.5 GREEN: Implement `load`. Return `Config::default()` for `None`, a missing directory, or a missing file, with no problem recorded. Read the file; a read error or a TOML parse error yields full defaults plus one problem naming `config.toml`. Per key, a value of the wrong TOML type — and a negative `archived_count` — yields that key's default plus one problem naming the key, while the keys that parsed stay honoured. Ignore unknown keys and tables silently. Never return `Result`, never panic
- [ ] 3.6 GREEN: Implement `openspec_bin` expansion: trim, treat empty as absent, expand a leading `~/` and a leading `$HOME/` and a bare `~` using the same injected lookup, return the value verbatim when `HOME` is unavailable or the value is a `~user` form, and never touch the filesystem to check the result
- [ ] 3.7 CHECK: Contract gate for the config format — re-read `SPEC.md` → Data layer → Resolution chain and `README.md` → Configuration and confirm the three key names, their types, and their defaults still match what `load` implements. They currently do not agree with the implementation on where the directory comes from; group 8 owns that correction, and this gate confirms nothing *else* drifted
- [ ] 3.8 REFACTOR: Collapse the three per-key readers into one typed accessor if that removes real duplication while the tests stay green; otherwise state that none was needed
- [ ] 3.9 Run the group tests — `cargo test --all-features` green, no regressions

## 4. State directory and agent-name derivation
<!-- kind: behavior -->

- [ ] 4.1 RED: Write failing unit tests in `src/state.rs` for `state_dir` and `agent_name`, named after the spec scenarios: "Herdr supplies the state directory" (and that it differs from `config_dir` on the same lookup), "`XDG_STATE_HOME` is honoured before `HOME`", "Run outside a Herdr pane with no XDG setting" (including `XDG_STATE_HOME=""`), "No directory can be resolved at all", "A short kebab-case change name is unchanged", "A name of exactly 32 characters is not truncated", "A long change name is truncated with a suffix from the whole name", "Illegal characters and casing are normalised", "A name that cannot begin an agent name is prefixed", "A name with nothing usable in it". Every derivation test asserts the result matches `^[a-z][a-z0-9_-]{0,31}$` — the 32-character cap is Herdr's, and a name that exceeds it is rejected at `agent start`, not here
- [ ] 4.2 Confirm the failures are the missing functions, not the regex or the fixture lookup
- [ ] 4.3 GREEN: Implement `state::state_dir` — `HERDR_PLUGIN_STATE_DIR`, else `$XDG_STATE_HOME/herdr/plugins/herdr-openspec`, else `$HOME/.local/state/herdr/plugins/herdr-openspec`, else `None`; empty values treated as unset throughout
- [ ] 4.4 GREEN: Implement `state::agent_name` as a pure, total function of the change name alone, in the order design.md and the spec fix: ASCII-lowercase; every character outside `[a-z0-9_-]` becomes `-`; runs of `-` collapse and leading and trailing `-` and `_` are trimmed; an empty result becomes `change`; a first character outside `[a-z]` gains a `c-` prefix; a result longer than 32 characters becomes its first 27 characters with trailing separators trimmed, `-`, and four base-36 digits of FNV-1a-32 over the **original** input, zero-padded
- [ ] 4.5 GREEN: Implement the FNV-1a-32 hash inline. Do not use `DefaultHasher`: its output is explicitly not stable across Rust releases and this value is written to disk and compared across processes
- [ ] 4.6 REFACTOR: Remove duplication between `config_dir` and `state_dir`'s "first non-empty variable wins" logic if group 2 did not already; otherwise state that none was needed
- [ ] 4.7 Run the group tests — `cargo test --all-features` green, no regressions

## 5. Reading and recording the mapping file
<!-- kind: behavior -->

- [ ] 5.1 RED: Write failing unit tests in `src/state.rs` for `read` and `record` against scratch directories, named after the spec scenarios: "Absent file and absent directory are both empty, not faults" (all three sub-cases), "A malformed file is empty and reports one problem", "A bad entry is skipped and its neighbours survive", "`names` is present but is not a table", "A truncated name is recorded", "An unchanged name is not recorded", "Recording the same pair twice changes nothing", "A second mapping is added beside the first", "The state directory is created on first record", "The write is not visible until it is complete", "Recording fails without panicking"
- [ ] 5.2 RED: Write the two containment tests — "A repository tree is untouched by a recording" (build a scratch fixture holding `openspec/changes/x/tasks.md`, snapshot listing, bytes, and mtimes, record into a *separate* scratch state directory, assert nothing in the fixture moved) and "The configuration directory is not written to"
- [ ] 5.3 RED: Make "The write is not visible until it is complete" able to fail. Assert the state directory contains no `.tmp-` entry after the call, and prove the replacement is a rename rather than an in-place truncate — write a longer previous file and assert the new content is not the new bytes followed by the tail of the old ones. An assertion that the file merely parses would pass against a truncating implementation
- [ ] 5.4 Confirm each failure comes from the missing behaviour, not from a scratch directory the helper failed to create
- [ ] 5.5 GREEN: Implement `state::read` — `None`, a missing directory, a missing file, and a zero-byte file each give an empty `Mapping` with no problem. A parse failure, or a `names` key that is not a table, gives an empty mapping plus one problem. Per entry, a non-string value or a key that fails `^[a-z][a-z0-9_-]{0,31}$` is skipped with one problem, and the well-formed entries in the same file are still returned. Never `Result`, never panic
- [ ] 5.6 GREEN: Implement `state::record(dir, agent, change)` — return `Ok(())` without creating anything when `agent == change`; otherwise read the existing mapping, insert, and write. Serialise through `toml::Table` and its `Display`, not by formatting strings by hand, so a change name needing quoting cannot corrupt the file
- [ ] 5.7 GREEN: Make the write atomic — create the state directory, write the full contents to `agent-names.toml.tmp-<pid>-<counter>` **inside that same directory** so the rename cannot cross a filesystem boundary, then `std::fs::rename` over the target. On any failure return `Err` naming the path and remove the temporary file; never panic
- [ ] 5.8 CHECK: Persistence gate — `agent-names.toml` is the change's only stored data. Confirm no migration, backfill, seeding, cache invalidation, or index rebuild applies: there is no prior format, an absent file is a legitimate empty mapping, and nothing caches the read. Record that here rather than omitting it
- [ ] 5.9 CHECK: Contract gate — re-inspect the published surface (`Config`, `Mapping`, `config_dir`, `state_dir`, `load`, `load_from_env`, `agent_name`, `read`, `record`) against design.md → Contracts and confirm each named consumer still gets what that section promises it: `repo-resolution`, `changes-from-files`, `agent-launch`, `agent-attribution`, `degraded-states`. Nothing consumes them yet, so a drift here is silent until Phase 2
- [ ] 5.10 REFACTOR: Share the "read a TOML table, collect problems" shape between `config::load` and `state::read` if that is real duplication rather than coincidence; otherwise state that it is coincidence and leave them apart
- [ ] 5.11 Run the group tests — `cargo test --all-features` green, no regressions

## 6. Environment and dependency confirmation
<!-- kind: operational -->

- [ ] 6.1 CHECK: Confirm the live-Herdr premise before trusting it further. Create a throwaway plugin outside the repository (a `herdr-plugin.toml` with `id`, `name`, `version`, `min_herdr_version`, `platforms`, and one `[[panes]]` entry whose command is `/bin/sh -c 'env | sort > <scratch>/env.txt; sleep 2'`), `herdr plugin link` it, `herdr plugin pane open --plugin <id> --entrypoint <id> --placement split --direction down --no-focus`, then `herdr plugin unlink <id>` and delete the config and state directories Herdr created for it. Assert `HERDR_PLUGIN_CONFIG_DIR` and `HERDR_PLUGIN_STATE_DIR` are both present in `env.txt` and that the first is character-identical to `herdr plugin config-dir <id>`. Do this in a scratch directory, never by pointing the repository's own `dashboard` pane somewhere else. **If this fails, stop:** the whole design rests on it, and the fallback paths become the only path — record the failure here and in planning-review.md rather than proceeding
- [ ] 6.2 VERIFY: Nothing spawns. `! grep -rnE 'process::Command|Stdio|(^|[^-])herdr[[:space:]]' src/` finds no match, confirming production code neither spawns nor invokes `herdr`, and that no `cli` module was created early — spec scenario "Resolution spawns nothing"
- [ ] 6.3 VERIFY: The suite passes with `herdr` unreachable — `env PATH="/usr/bin:/bin" cargo test --all-features` is green, or where cargo itself is not on that `PATH`, a `PATH` built to resolve `cargo` and not `herdr`. Same spec scenario; record which form was used and confirm `herdr` was genuinely unresolvable on it
- [ ] 6.4 VERIFY: The dependency is load-bearing — copy `Cargo.toml` aside to a scratch path, delete the `toml` line, confirm `cargo build` fails, restore from the copy, and confirm `cargo test --all-features` is green again — spec scenario "The dependency is genuinely needed rather than incidental". Restore from the copy, never `git checkout --`: this change's own work may not be committed yet
- [ ] 6.5 VERIFY: No scratch state leaked — `HERDR_PLUGIN_STATE_DIR` for `herdr-openspec` and the real `~/.config/herdr/plugins/config/herdr-openspec` are both untouched by the test run, and no `herdr-openspec` temporary file exists under `std::env::temp_dir()` afterwards

## 7. Change Review
<!-- kind: operational -->

- [ ] 7.1 CHECK: Dispatch an independent reviewer — not a fork of the implementing session — against proposal.md, all three spec files, design.md, and tasks.md plus the diff. Point it first at the concentration points that bite here: a test asserting a fixture back at itself rather than the behaviour; the empty-file and malformed-file cases being distinguishable only by `problems`; the atomic-write test being able to fail against a truncating implementation; and the containment tests actually asserting mtimes rather than existence
- [ ] 7.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a one-line reason, note SUGGESTIONs, and re-run every affected check
- [ ] 7.3 VERIFY: Confirm no blocking or unowned finding remains, and that every artifact a repair touched was updated in place rather than annotated afterwards

## 8. Documentation
<!-- kind: operational -->

- [ ] 8.1 Rewrite in `SPEC.md`: Data layer → Resolution chain, step 1 of "The `openspec` binary" (audience: every future change reading plugin configuration) — it says the directory is "reported by `herdr plugin config-dir herdr-openspec`", which no plugin process can call without a spawn. Replace with `HERDR_PLUGIN_CONFIG_DIR`, naming the command as the equivalent a human runs and the computed path as the fallback outside a pane, and name `HERDR_PLUGIN_STATE_DIR` as the home of the truncated-name mapping — the current text puts that mapping in the config directory, which the plugin must not write to
- [ ] 8.2 Add to `SPEC.md`: Overview → stack list (audience: future changes choosing a parser) — `toml`, which is missing although the configuration format the same document specifies is TOML
- [ ] 8.3 Add to `SPEC.md`: Architecture → module map (audience: same) — one row each for `config` and `state`, the two modules this change adds, so the map stays the index it claims to be
- [ ] 8.4 Add to `README.md`: Configuration (audience: users) — one sentence that the same directory is supplied to the plugin as `HERDR_PLUGIN_CONFIG_DIR`, and which path is used when the binary runs outside a Herdr pane. `herdr plugin config-dir herdr-openspec` stays as the way a user finds the directory; it is correct and is not what the plugin calls
- [ ] 8.5 Rewrite in `AGENTS.md`: Architecture rules (audience: every future session) — the existing "Nothing spawns a process outside `cli`" bullet gains the corollary that makes it survivable here: Herdr injects `HERDR_PLUGIN_CONFIG_DIR`, `HERDR_PLUGIN_STATE_DIR`, `HERDR_PLUGIN_ROOT`, and the workspace, tab, and pane ids into every plugin process, so plugin context is read from the environment rather than from `herdr plugin ...`. Rewrite the bullet, do not append a second one
- [ ] 8.6 Add to `AGENTS.md`: Conventions (audience: same) — two lines: environment-dependent code takes an injected lookup because `std::env::set_var` is `unsafe` in edition 2024 and races parallel tests; and `openspec/` is never written to, while `HERDR_PLUGIN_STATE_DIR` is where this plugin's own writes go
- [ ] 8.7 VERIFY: Net size — group 8 adds roughly a dozen lines across four documents and rewrites two entries in place. Confirm no entry it touched now says something false, in particular that no document still claims the crate has no third-party dependencies or that the config directory comes from a subprocess

## 9. Lint & Verify
<!-- kind: operational -->

- [ ] 9.1 CHECK: Inspect the intended verification commands and affected tiers — the unit tier (`cargo test --all-features`, `src/config.rs` and `src/state.rs`), the command checks in groups 1 and 6, and the live check in 6.1. Coverage is affected: this change roughly triples the crate's line count
- [ ] 9.2 VERIFY: `cargo fmt --all -- --check` — clean
- [ ] 9.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors. Probe with `cargo clippy --version`, not `command -v cargo-clippy`: rustup installs that shim unconditionally, so the shim resolves even when the component is absent
- [ ] 9.4 VERIFY: `cargo test --all-features` — green. Rust has no separate type-check step; `cargo test` and `cargo clippy` cover it
- [ ] 9.5 VERIFY: `cargo llvm-cov --fail-under-lines 80` — at or above the floor. If it falls short, add tests; never lower the threshold and never add an exclusion flag
- [ ] 9.6 VERIFY: `make check` — the single composite gate, exit 0. If it fails, name the failing sub-command rather than reporting a summary
- [ ] 9.7 VERIFY: `openspec validate plugin-config --strict` reports the change valid, with nvm ahead on `PATH` — `export PATH="$HOME/.nvm/versions/node/v24.20.0/bin:$PATH"` — since `openspec` is not on the `PATH` a non-login shell inherits here
