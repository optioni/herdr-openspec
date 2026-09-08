# Planted defects

Each entry: the plant, the command run, the verbatim red output, and confirmation the
plant was reverted (`git diff --quiet -- <path>` returning 0, checked after every revert).
Follows `degraded-states`' `notes/planted-defects.md` in shape.

Groups 1–8 of `doc-conformance` each began RED by construction (the leg's own first test
was written failing). This group plants a real defect for every direction that was green
at HEAD and had not yet been exercised against the real files — the controls below.

One plant was live at a time throughout; nothing here left the tree dirty.

## (a) The orphan direction — `module_map_matches_lib_rs`

Added a row to `SPEC.md`'s Module map with no matching `pub mod` in `src/lib.rs`:

```diff
 | `cli` | The two subprocess traits and their real implementations |
+| `zzz` | placeholder |
```

Command: `cargo test --test doc_contract module_map_matches_lib_rs -- --exact`

Red:

```
thread 'module_map_matches_lib_rs' panicked at tests/doc_contract.rs:175:9:
SPEC.md's Module map disagrees with src/lib.rs's declared modules: in SPEC.md's Module map
but not src/lib.rs's pub mod set: ["zzz"]
```

Reverted; `git diff --quiet -- SPEC.md` → 0; `module_map_matches_lib_rs` green afterwards.
Direction confirmed: "mapped but undeclared" is reported on the correct side.

## (b) The manifest value half — `spec_manifest_block_matches` / `spec_manifest_block_order_matches`

Changed one `command` value inside `SPEC.md`'s fenced manifest block (the `open-tab` action):

```diff
-command = ["./target/release/herdr-openspec", "open-tab"]
+command = ["./target/release/herdr-openspec", "open-tab-WRONG"]
```

Command: `cargo test --test doc_contract spec_manifest_block_matches`

Red:

```
thread 'spec_manifest_block_matches' panicked at tests/doc_contract.rs:986:5:
assertion `left == right` failed: SPEC.md's transcription of the plugin manifest does not
match herdr-plugin.toml
  left: {..., "open-tab-WRONG", ...}
 right: {..., "open-tab", ...}
```

`spec_manifest_block_order_matches` (`cargo test --test doc_contract
spec_manifest_block_order_matches`): **stayed green** — `test spec_manifest_block_order_matches
... ok` — confirming the value leg and the order leg are independent, as designed.

Reverted; `git diff --quiet -- SPEC.md` → 0; both tests green afterwards.

## (c) The MSRV manifest side — `msrv_is_documented`

The plan named `1.92` for this control. Not used: the installed toolchain is `rustc
1.91.1`, so `1.92` would make `cargo build` refuse to build the crate at all, and the
control would redden on a build failure rather than on the assertion under test. Used
`1.87` instead — below the installed compiler, so the crate still builds, and different
from the documented `1.88`:

```diff
-rust-version = "1.88"
+rust-version = "1.87"
```

Confirmed the crate still builds under the plant: `cargo build` → `Finished` (no error).

Command: `cargo test --test doc_contract msrv_is_documented -- --exact`

Red:

```
thread 'msrv_is_documented' panicked at tests/doc_contract.rs:376:5:
Cargo.toml's rust-version ("1.87") is not documented in: ["AGENTS.md's Environment
section", "README.md's Development section"]
```

Reverted; `git diff --quiet -- Cargo.toml` → 0; `msrv_is_documented` green afterwards
(confirmed the crate still builds and matches HEAD, `rust-version = "1.88"`).

## (d1) The gate-script sub-leg, README-only half — `gate_programs_are_documented` / `gate_script_interpreters_are_documented`

The plan said "delete the `python3` line from the `Makefile`'s `gates:` recipe" — stale:
`gate-integrity` added a second `python3` site, so the `Makefile` now names it twice
(line 29, `coverage:`'s `python3 scripts/coverage-prod.py …`; line 65, `gates:`'s `python3
scripts/gates/gate-mech1.py`), and `README.md`/`AGENTS.md` both already document it. This
step removes `python3` from `README.md` → § Development only, leaving both `Makefile`
lines and `AGENTS.md` untouched:

```diff
 Requires Rust — `Cargo.toml`'s `rust-version = "1.88"` is the supported floor
-— and `python3`, which the hygiene-gate tier's own scripts and the coverage
-gate's production-floor script invoke, plus two one-time components:
+— plus two one-time components:
```

Command: `cargo test --test doc_contract gate_`

Red (both legs, both naming only `README.md`):

```
thread 'gate_programs_are_documented' panicked at tests/doc_contract.rs:718:5:
programs invoked by make check's path are undocumented: ["python3 missing from
[\"README.md's Development section\"]"]

thread 'gate_script_interpreters_are_documented' panicked at tests/doc_contract.rs:758:5:
python3 is invoked under scripts/gates/ but not documented in: ["README.md's Development
section"]
```

Reverted; `git diff --quiet -- README.md` → 0; both tests green afterwards.

## (d2) The gate-script sub-leg's independence — `gate_programs_are_documented` / `gate_script_interpreters_are_documented`

Removed **both** `Makefile` `python3` lines (29 and 65) **and** removed `python3` from
`README.md` → § Development (the same edit as d1), together:

```diff
 	cargo llvm-cov report --summary-only
-	python3 scripts/coverage-prod.py target/llvm-cov.json tests/degraded-coverage.toml
```
```diff
 	/bin/sh scripts/gates/widths.sh
-	python3 scripts/gates/gate-mech1.py
```
plus the same README.md edit as (d1).

Command: `cargo test --test doc_contract gate_`

Red — the two legs failed for **different reasons**, proving their independence:

```
thread 'gate_programs_are_documented' panicked at tests/doc_contract.rs:696:46:
extract programs from Makefile's check: path: "the extraction rule found no external
program in check:'s prerequisite recipes"

thread 'gate_script_interpreters_are_documented' panicked at tests/doc_contract.rs:758:5:
python3 is invoked under scripts/gates/ but not documented in: ["README.md's Development
section"]
```

`gate_programs_are_documented` did **not** fail on "python3 missing from [...]" — with
both `Makefile` `python3` lines gone, `check_programs`'s extraction over `check:`'s
prerequisite recipes yields an **empty** set (the remaining recipe lines are all either
`cargo …` or `/bin/sh scripts/gates/*.sh`, both excluded by the extraction rule), and
`check_programs`'s own non-emptiness assertion fires first, so the failure is an `.expect()`
panic on `Err`, not the "undocumented programs" assertion. `gate_script_interpreters_are_documented`
meanwhile still failed on its own terms, still naming `python3`, sourced independently from
the ten files under `scripts/gates/` that invoke it (`nosleep.sh`, `taskseam.sh`,
`nodefault-ui.sh`, `mdwidths.sh`, `listwidths.sh`, `deps.sh`, `taskwidths.sh`,
`gate-mech1.py`, `detailwidths.sh`, `widths.sh`) — confirming the spec scenario "a gate
script's own interpreter is undocumented" fires independently of whatever the `Makefile`'s
own `check:` path happens to invoke.

Reverted (`Makefile` and `README.md` both); `git diff --quiet -- Makefile README.md` → 0;
`gate_` tests green afterwards (4 passed).

## 9.2 The `AGENTS.md` direction of leg 5 — `gate_programs_are_documented`

Removed `python3` from `AGENTS.md` → § Environment (the bullet naming `scripts/gates/deps.sh`):

```diff
-- **python3** — required by `make check`: `scripts/gates/deps.sh` parses `cargo
-  metadata`'s JSON through it. Present on both GitHub runners and on the reference
-  machine; no crate is added to do this instead.
```

Command: `cargo test --test doc_contract gate_programs_are_documented -- --exact`

Red:

```
thread 'gate_programs_are_documented' panicked at tests/doc_contract.rs:718:5:
programs invoked by make check's path are undocumented: ["python3 missing from
[\"AGENTS.md's Environment section\"]"]
```

Reverted; `git diff --quiet -- AGENTS.md` → 0; `gate_programs_are_documented` green
afterwards. Confirms the leg names `AGENTS.md` specifically, not only `README.md` (which
groups 1–8 never drove — README.md was the only side any earlier group's RED touched).

## Extra control 1 — the worker-thread leg against the real document, not just synthetic sources

Group 3 proved `worker_threads_match_sources`'s discriminator (`thread::spawn` + `mpsc` in
the production slice) with synthetic in-memory sources (`worker_count_stale`,
`worker_count_grows`), never a real file edit — and `src/cli.rs` is under `src/` and must
not be touched by this group. Planted from the document side instead: changed `SPEC.md`'s
claim from three to four worker threads, leaving every `src/*.rs` file untouched:

```diff
 - `refresh::start`, `refresh::none`, and the worker body — one of the
-  crate's **three** worker threads, tested through a `#[cfg(test)]` constructor
+  crate's **four** worker threads, tested through a `#[cfg(test)]` constructor
```

Command: `cargo test --test doc_contract worker_threads_match_sources -- --exact`

Red:

```
thread 'worker_threads_match_sources' panicked at tests/doc_contract.rs:1641:5:
assertion `left == right` failed: SPEC.md documents 4 worker thread(s) but 3 file(s) under
src/ have a production slice naming both thread::spawn and mpsc: {"agents.rs", "launch.rs",
"refresh.rs"}
  left: 4
 right: 3
```

Reverted; `git diff --quiet -- SPEC.md` → 0; `worker_threads_match_sources` green
afterwards.

## Extra control 2 — the tested-modules leg's section scoping, against the real file

Group 3's `tested_modules_scoped_to_section` proved the section-boundary rule with a
synthetic `&str`, never against `SPEC.md` itself. Moved the `launch` module's `<name>::`
mentions out of `### Unit-tested modules` — stripped the `launch::` prefixes from that
bullet's four function names and its `Outcome` mention — while leaving a `launch::` mention
elsewhere in the document (the Module map row, outside the section) so the module is still
named in the file, just not inside the bound section:

```diff
-- `launch::decide`, `launch::pane_id`, `launch::run_request`, and `launch::settle` — the pure
+- `decide`, `pane_id`, `run_request`, and `settle` — the pure
   launch policy ...
-  `Launcher` trait's `RealLauncher`/`NoLauncher` implementations. `launch::Outcome` carries at
+  `Launcher` trait's `RealLauncher`/`NoLauncher` implementations. `Outcome` carries at
```
```diff
-| `launch` | Split a pane, start an agent, send the `/opsx:*` prompt |
+| `launch` | Split a pane, start an agent, send the `/opsx:*` prompt (`launch::decide`) |
```

Command: `cargo test --test doc_contract tested_modules_names_every_module -- --exact`

Red:

```
thread 'tested_modules_names_every_module' panicked at tests/doc_contract.rs:261:5:
modules declared `pub mod` in src/lib.rs but not named as `<name>::` in SPEC.md's '###
Unit-tested modules' section: {"launch"}
```

Reverted; `git diff --quiet -- SPEC.md` → 0; `tested_modules_names_every_module` green
afterwards. Confirms the section-scoping rule holds against the real document: a `launch::`
mention elsewhere in `SPEC.md` does not satisfy the leg once it is absent from the bound
section.

## Verification (task 9.4)

- `git status --porcelain` — clean of every plant throughout and at the end of this group;
  each plant was reverted (`git diff --quiet -- <path>` → 0) before the next was made, and
  no two plants were ever live at once.
- `git diff --name-only 47353bf..HEAD` — lists only this change's own files (`SPEC.md`
  unchanged by this group's real work; this group's one addition is
  `openspec/changes/doc-conformance/notes/planted-defects.md`) plus the peer session's own
  directories (`openspec/changes/list-sections/`, `color-palette/`, `markdown-constructs/`,
  `mouse-input/`), never `src/`.
- `cargo test --test doc_contract` — 49 passed, 0 failed.
