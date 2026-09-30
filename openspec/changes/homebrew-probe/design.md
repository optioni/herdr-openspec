## Context

`resolve::openspec_bin` probes four steps: the configured `openspec_bin`, `PATH`, the nvm
version trees, and `<npm prefix -g>/bin/openspec` (`src/resolve.rs:248`). A Herdr pane does
not run through a login shell, so the `PATH` it inherits may lack `/opt/homebrew/bin`. Step
4's production binding runs a bare `npm` (`cli::npm_prefix`, `src/cli.rs:435-437`), which is
looked up through that same `PATH`. When step 2 misses Homebrew, step 4 misses it too.

Measured on the reference machine at planning time:

- `ls -la /opt/homebrew/bin/openspec` → `-> ../Cellar/openspec/1.13.2/bin/openspec`
- `brew info openspec` → `openspec: stable 1.13.2 (bottled)`, `From: …/homebrew-core/…/openspec.rb`
- `file /opt/homebrew/Cellar/openspec/1.13.2/bin/openspec` → `a /opt/homebrew/opt/node/bin/node script text executable`
- `env -i PATH=/usr/bin:/bin /opt/homebrew/bin/openspec --version` → `1.13.2`, exit 0; `npm`
  is not found on that same `PATH`
- `ls ~/.nvm/versions/node/*/bin/openspec` → no match, across four installed versions

So Homebrew's own `openspec` formula is the only install on the reference machine. Its shim
is still a link to `@fission-ai/openspec/bin/openspec.js`, but the formula rewrites its
shebang to an absolute interpreter path, so it needs no `PATH` to exec. Once this change
lands, `openspec/config.yaml`'s injected `context` ("the `openspec` CLI is nvm-installed")
is false here.

## Goals / Non-Goals

**Goals:**

- A Homebrew-installed `openspec` is resolved when no earlier step finds anything, and the
  settings panel reports `Homebrew` as the step that won.
- No test can consult the machine's real Homebrew installation. The list is injected on the
  npm hook's terms.
- Every machine where steps 1–4 already resolve behaves byte-identically.

**Non-Goals:** as proposal.md: no `HOMEBREW_PREFIX` read, no `brew --prefix` spawn, no
login-shell `PATH` recovery, no other version managers, no reordering of steps 1–4.

## Boundaries

- **`src/resolve.rs`**, the pure side. It gains:
  - `pub const HOMEBREW_PREFIXES: &[&str]`;
  - a `BinSource::Homebrew` variant;
  - a pure `pub(crate) fn homebrew_candidates(prefixes) -> Vec<PathBuf>`, on
    `path_candidates`' terms, which drops empty, blank and non-absolute entries;
  - a private `step5_homebrew` beside `step4_npm_prefix`;
  - a fourth parameter on `openspec_bin`.

  The step reuses `is_usable_binary` and names no process API or environment lookup, so
  `src/resolve.rs` stays clear of the "names no process API" clause.
  `openspec_bin_from_env` passes `HOMEBREW_PREFIXES`. That function has **no production
  caller**: its only caller, `cli::worker_cli_from_env` (`src/cli.rs:525`), is itself unused.
  The live binding is `ui::run`, which `WIRED` covers. It is kept correct because it is the
  crate's documented one-line composition.
- **`src/settings.rs`** gains one arm in `Provenance::label`'s exhaustive `match` on
  `BinSource`. The compiler forces it, because the match has no wildcard.
- **`src/ui/mod.rs`**, the composition root:
  - `Startup` gains `homebrew_prefixes: &'a [&'a str]` beside `npm_hook`. It stays flat, so
    no existing field is renamed.
  - A new `pub struct ProbeBindings<'a> { env, npm_hook, homebrew_prefixes }` replaces
    `start_collaborators`' separate `env` and `npm_hook` parameters, taking it from 7
    parameters to 6 (Decision 7).
  - `run_wired` builds the bundle from its `Startup`. `start_collaborators` hands
    `probe.homebrew_prefixes` to `resolve::openspec_bin`, and reads `probe.env` for the `PATH`
    overlay exactly as it read `env`.
  - `run` passes `crate::resolve::HOMEBREW_PREFIXES`, following `degraded-states`' Decision
    14 for `env`/`npm_hook`. `start_collaborators` names no constant, only the bundle.
- **`scripts/gates/wired.sh`** gains a body-scoped **leg 8** (Decision 8): `run`'s body names
  `HOMEBREW_PREFIXES`, and `start_collaborators`' body does not. It also gains a positive
  control anchored on `^pub const HOMEBREW_PREFIXES` in `src/resolve.rs`, on Guard A's terms
  beside `npm_probe_hook`'s. Leg 1's list and its "fourteen names" message do not change.
  **`tests/gate-controls.toml`** gains three `[[control]]` plants: the list emptied in `run`,
  the list moved into `start_collaborators`, and the constant renamed.
- **Stale doc comments** are rewritten alongside the code they describe:
  - `openspec_bin`'s, which enumerates four steps (`src/resolve.rs:235-247`);
  - `openspec_bin_from_env`'s, which says "and nothing else" of three inputs (`:322-327`);
  - the overlay helper's "all four" (`src/ui/mod.rs:165`);
  - `start_collaborators`', which names `env` and `npm_hook` (`:190-192`);
  - `launch`'s "the plugin's four-step probe" (`src/launch.rs:288`).
- **No process spawn is added anywhere.** Step 5 is `fs::metadata` on at most three paths.
  `src/cli.rs` changes only in its tests, at two `openspec_bin` call sites.
- **No view changes.** The settings panel already renders `Provenance::label()` and is
  unchanged. `Homebrew` (8 columns) is narrower than the widest existing probe label,
  `npm prefix -g` (13), so no width test moves.
- **The `Change` type is untouched.** `from_files` and `from_cli` are not involved. A
  resolved binary changes only whether the CLI tier runs, which is existing behaviour.

## Contracts

All crate-internal. None is consumed outside this repository.

| Interface | Change | Consumers |
|---|---|---|
| `resolve::openspec_bin(configured, env, npm_prefix)` | gains `homebrew_prefixes: &[&str]` as a 4th parameter | `ui::start_collaborators`, `resolve::openspec_bin_from_env`, 29 `super::openspec_bin(` test calls in `src/resolve.rs`, 2 `crate::resolve::openspec_bin(` in `src/cli.rs` tests (`grep -c` for each) |
| `resolve::BinSource` | gains `Homebrew` | `settings::Provenance::label`, the one exhaustive match. Elsewhere, `grep -rn "BinSource::" src \| grep -v src/resolve.rs` shows constructions, doc comments, and one `==` comparison (`src/settings.rs:192`), none of which a new variant moves |
| `resolve::homebrew_candidates` | new, `pub(crate)` | `step5_homebrew` and its own tests |
| `ui::Startup` | gains `homebrew_prefixes` | `run`'s literal (`src/ui/mod.rs:471`) passes the constant; `run_wired_probed`'s literal (`:3361`) forwards `p.homebrew_prefixes`; the 5 other test `Startup {` literals (`:3295`, `:3681`, `:3913`, `:5230`, `:5394`) pass `&[]`. `grep -n "Startup {" src/ui/mod.rs` gives 14 lines: these 7 plus 7 `ProbedStartup {` |
| test-only `ProbedStartup` | gains `homebrew_prefixes` | its 7 existing literals (`:5434`, `:5463`, `:5728`, `:5775`, `:5822`, `:5944`, `:6016`) pass `&[]` |
| `ui::start_collaborators(repo, config, herdr, state_dir, env, npm_hook, git)` | becomes `(repo, config, herdr, state_dir, probe: ProbeBindings<'_>, git)` | `run_wired` (`:317`) builds the bundle from `startup`; the 2 test calls (`:5868`, `:6374`) build it with `homebrew_prefixes: &[]` |
| `ui::ProbeBindings` | new | the same 3 call sites |

The change is additive in behaviour and breaking only to these in-crate signatures. It
touches no manifest, `config.toml` key, keybinding, or rendered grammar.

## Persistence and Rollout

- **Migration:** none.
- **Backfill:** none.
- **Seeding:** none.
- **Cache invalidation:** none. The probe runs once per process because
  `start_collaborators` runs once, and a new process probes afresh. `resolve::BinCache` has no
  production user.
- **Index rebuild:** none.
- **Authorization:** none. See Risks for the trust question step 5 raises.
- **Observability:** the settings panel's `openspec_bin` row reads `Homebrew` when step 5
  wins. That label is the only diagnostic, as it already is for every other step.
- **Deployment:** none beyond the usual `make build`. Existing panes pick it up on restart.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| Filesystem | real: `ScratchDir` trees holding scratch `bin/openspec` files | real: `ScratchDir` trees |
| Machine's Homebrew installation (`/opt/homebrew`, `/home/linuxbrew/.linuxbrew`, `/usr/local`) | replaced: `homebrew_prefixes` names a scratch prefix, or is empty | replaced: the `homebrew_prefixes` argument names scratch prefixes, or is empty. Never consulted: the order test reads the constant's value and stats nothing, and the source-text test reads source |
| `openspec` binary | replaced: a scratch `#!/bin/sh` stand-in written by `openspec_script`, which logs each invocation | not spawned: step 5 only stats it |
| `npm` program (step 4) | replaced: `npm_hook` closure returning `None` or a scratch prefix | replaced: closure |
| Process environment | replaced: fixture-map lookup through `Startup::env` | replaced: fixture-map lookup |
| Herdr socket | replaced: a non-existent `herdr` path (`does-not-exist-herdr`), as the probe wiring tests already pass. Not exercised by this change | not reached |
| Terminal | replaced: `TestBackend` at 120×20 and 60×20 | not reached |
| `git` | replaced: a non-existent `git` path (`does-not-exist-git`), on the same terms. Not exercised by this change | not reached |

## Test Strategy

This change **writes an acceptance test but opens no group 0**. The headline is observable
only at the composition root: a pane whose only `openspec` sits under a Homebrew prefix runs
in merged mode, not file mode. The wiring the defect lives in is also at the root: `run` →
`Startup` → `ProbeBindings` → `start_collaborators` → `resolve::openspec_bin`. The
acceptance test, `a_homebrew_only_install_leaves_file_mode`, drives `run_wired` at 120×20 and
60×20 with the prefix list naming a scratch prefix `B`. It asserts:

- `!dashboard.file_mode`;
- at least one line in the stand-in's invocation log, so `B/bin/openspec` is the binary that
  ran — on the reference machine a leak of the real list would also clear file mode, and
  this assertion is what tells the two apart;
- no `file mode` badge in the 120-column header, which is the symptom a user sees;
- that the same run with `homebrew_prefixes: &[]` reports file mode.

It cannot be written first. The crate is one compilation unit, and a test naming a
`Startup` field that does not exist yet fails to *compile*. That would stop every group-1
test from building, not just itself. So it is group 2's first RED, and group 2 takes it
through a **behavioural** RED before GREEN:

1. Add the field and `ProbeBindings`, while `start_collaborators` still hands group 1's `&[]`
   placeholder to the probe. The test now compiles, and fails on `!dashboard.file_mode`.
2. Forward the bundle's list. The test goes green.

The value `run` passes is unreachable by any test (`run` needs a real terminal), so `WIRED`
leg 8 binds it.

The source-text test `the_composition_passes_the_production_homebrew_prefix_list` cuts
`openspec_bin_from_env`'s body out of `include_str!("resolve.rs")`, from
`pub fn openspec_bin_from_env(` to the next line that is exactly `}`. It strips `//` comments
and asserts that `HOMEBREW_PREFIXES` is named. So the constant's own definition, and a
comment, cannot satisfy it. Task 1.5 plants `&[]` in its place and sees it fail.

Every chain rule is a `src/resolve.rs` unit test over scratch trees. The label is a
`src/settings.rs` unit test. Tier commands are `cargo test --all-features --lib <filter>` for
unit and wiring tests, `cargo test --all-features --test <target>` for the contract tier, and
`/bin/sh scripts/gates/wired.sh` or `make gates` for the gate tier.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| openspec-binary: The configured path wins over every other source | existing `the_configured_path_wins_over_every_other_source`, gains `&[]` | unit | scratch fs, fixture env, closure | `cargo test --lib resolve::tests::the_configured_path_wins` |
| openspec-binary: `PATH` wins when nothing is configured | existing `path_wins_when_nothing_is_configured` | unit | scratch fs | `cargo test --lib resolve::tests::path_wins` |
| openspec-binary: `PATH` entries are searched left to right | existing `path_entries_are_searched_left_to_right` | unit | scratch fs | `cargo test --lib resolve::tests::path_entries` |
| openspec-binary: An empty or blank `PATH` entry is not the current directory | existing `an_empty_or_blank_path_entry_is_not_the_current_directory` | unit | scratch fs | `cargo test --lib resolve::tests::an_empty_or_blank` |
| openspec-binary: An absent or blank `PATH` contributes nothing | existing `an_absent_or_blank_path_contributes_nothing` | unit | scratch fs, closure | `cargo test --lib resolve::tests::an_absent_or_blank` |
| openspec-binary: The nvm tree is searched when `PATH` has nothing | existing `the_nvm_tree_is_searched_when_path_has_nothing` | unit | scratch fs | `cargo test --lib resolve::tests::the_nvm_tree_is_searched` |
| openspec-binary: Node versions are ordered numerically, not lexically | existing `node_versions_are_ordered_numerically_not_lexically` | unit | scratch fs | `cargo test --lib resolve::tests::node_versions` |
| openspec-binary: A version directory without a usable binary is skipped | existing `a_version_directory_without_a_usable_binary_is_skipped` | unit | scratch fs | `cargo test --lib resolve::tests::a_version_directory_without` |
| openspec-binary: A version directory whose name is not a version is still eligible, and sorts last | existing `a_version_directory_whose_name_is_not_a_version_is_still_eligible_and_sorts_last` | unit | scratch fs | `cargo test --lib resolve::tests::a_version_directory_whose` |
| openspec-binary: `NVM_DIR` overrides the default nvm root | existing `nvm_dir_overrides_the_default_nvm_root` | unit | scratch fs | `cargo test --lib resolve::tests::nvm_dir_overrides` |
| openspec-binary: The nvm tree outranks the npm prefix | existing `the_nvm_tree_outranks_the_npm_prefix` | unit | scratch fs, closure | `cargo test --lib resolve::tests::the_nvm_tree_outranks` |
| openspec-binary: The npm prefix is the last resort | existing `the_npm_prefix_is_the_last_resort`, passing `&[]` explicitly | unit | scratch fs, closure | `cargo test --lib resolve::tests::the_npm_prefix_is_the_last_resort` |
| openspec-binary: An npm prefix without a usable binary yields nothing | existing `an_npm_prefix_without_a_usable_binary_yields_nothing` | unit | scratch fs, closure | `cargo test --lib resolve::tests::an_npm_prefix_without` |
| openspec-binary: Nothing anywhere is a supported state, not a fault | existing `nothing_anywhere_is_a_supported_state_not_a_fault`, extended with the empty-plus-never-created prefixes re-run | unit | scratch fs, closure | `cargo test --lib resolve::tests::nothing_anywhere` |
| openspec-binary: A Homebrew prefix is searched when nothing earlier resolves | new `a_homebrew_prefix_is_searched_when_nothing_earlier_resolves`, including the `HOMEBREW_PREFIX`-in-env arm | unit | scratch fs, fixture env, closure | `cargo test --lib resolve::tests::a_homebrew_prefix_is_searched` |
| openspec-binary: The npm prefix outranks the Homebrew prefixes | new `the_npm_prefix_outranks_the_homebrew_prefixes` | unit | scratch fs, closure | `cargo test --lib resolve::tests::the_npm_prefix_outranks` |
| openspec-binary: Homebrew prefixes are tried in list order, skipping one without a binary | new `homebrew_prefixes_are_tried_in_list_order_skipping_one_without_a_binary` (resolution arms) | unit | scratch fs | `cargo test --lib resolve::tests::homebrew_prefixes_are_tried` |
| openspec-binary: Homebrew prefixes are tried in list order, skipping one without a binary (candidate list) | new `homebrew_candidates_drop_empty_blank_and_relative_entries` | unit | none | `cargo test --lib resolve::tests::homebrew_candidates_drop` |
| openspec-binary: The production prefix list puts Apple Silicon first and Intel last | new `the_production_prefix_list_puts_apple_silicon_first_and_intel_last` (see Decision 3 for why a constant earns a test) | unit | none | `cargo test --lib resolve::tests::the_production_prefix_list` |
| openspec-binary: A configured path that does not exist falls through to `PATH` | existing `a_configured_path_that_does_not_exist_falls_through_to_path` | unit | scratch fs | `cargo test --lib resolve::tests::a_configured_path_that_does_not` |
| openspec-binary: A configured path that is not executable falls through | existing `a_configured_path_that_is_not_executable_falls_through` | unit | scratch fs | `cargo test --lib resolve::tests::a_configured_path_that_is_not` |
| openspec-binary: A configured path fails and nothing else is found | existing `a_configured_path_fails_and_nothing_else_is_found` | unit | scratch fs | `cargo test --lib resolve::tests::a_configured_path_fails` |
| openspec-binary: The composition honours a configured binary | existing `the_composition_honours_a_configured_binary` | unit | scratch fs, real env | `cargo test --lib resolve::tests::the_composition_honours` |
| openspec-binary: The composition passes the production Homebrew prefix list | new `the_composition_passes_the_production_homebrew_prefix_list`, body-cut and comment-stripped as above, with a planted `&[]` (task 1.5) | unit | source text | `cargo test --lib resolve::tests::the_composition_passes` |
| openspec-binary: An outer test drives a failing probe without touching the machine | existing `run_wired_probes_through_the_injected_hook` (`src/ui/mod.rs:5427`), whose `ProbedStartup` gains `homebrew_prefixes: &[]`, keeps the failing and npm-resolving arms | wiring | scratch fs, fixture env, closure, `TestBackend`, non-existent herdr/git | `cargo test --lib ui::tests::wiring::run_wired_probes_through_the_injected_hook` |
| openspec-binary: An outer test drives a failing probe without touching the machine (Homebrew-list control) | new acceptance test `a_homebrew_only_install_leaves_file_mode` at 120×20 and 60×20. This is the scenario's Homebrew AND clause; it is not duplicated as an arm of the test above | wiring | as above, plus an `openspec_script` stand-in under `B/bin` | `cargo test --lib ui::tests::wiring::a_homebrew_only_install` |
| openspec-binary: The composition root is the one place the real bindings are named | existing `WIRED` leg 1 for `config::env_lookup(` and `npm_probe_hook`; new leg 8 for `HOMEBREW_PREFIXES`, with 3 recorded plants (task 3.3) | gate / contract | source text | `/bin/sh scripts/gates/wired.sh`; `cargo test --all-features --test gate_controls` |
| openspec-binary: A full probe leaves the filesystem byte-identical | existing `a_full_probe_leaves_the_filesystem_byte_identical`, reworked so steps 1–4 all miss and step 5 resolves through `[E, B]`, asserting `BinSource::Homebrew` | unit | scratch fs | `cargo test --lib resolve::tests::a_full_probe` |
| setting-provenance: Every `integration::Source` maps to a `Provenance` and back to one label | existing `every_integration_source_maps_to_a_provenance_and_back_to_one_label` | unit | none | `cargo test --lib settings::tests::every_integration_source` |
| setting-provenance: Every probe step is named by the step that won | existing `every_probe_step_is_named_by_the_step_that_won` (`src/settings.rs:436`), gains `Homebrew` and its exact label | unit | none | `cargo test --lib settings::tests::every_probe_step` |
| setting-provenance: `Unresolved` is file mode's own label and is not an error | existing `unresolved_is_file_modes_own_label_and_is_not_an_error` | unit | none | `cargo test --lib settings::tests::unresolved_is` |
| setting-provenance: A pending kind renders as resolving and edits nothing | existing `a_pending_kind_renders_as_resolving_and_edits_nothing` | unit | none | `cargo test --lib settings::tests::a_pending_kind` |
| setting-provenance: An ambiguous kind has no effective value and offers both candidates | existing `an_ambiguous_kind_has_no_effective_value_and_offers_both_candidates` | unit | none | `cargo test --lib settings::tests::an_ambiguous_kind` |
| refresh-worker: The overlay prepends the resolved binary's own directory to `PATH` | existing `collaborators_overlay_prepends_the_resolved_binarys_own_directory_to_path` (`src/ui/mod.rs:5706`) | wiring | fixture env, scratch fs | `cargo test --lib collaborators_overlay_prepends` |
| refresh-worker: An absent inherited `PATH` yields the directory alone | existing `collaborators_overlay_is_the_binarys_directory_alone_when_path_is_absent` | wiring | fixture env, scratch fs | `cargo test --lib collaborators_overlay_is_the_binarys` |
| refresh-worker: A binary already on `PATH` gets the same overlay, harmlessly | existing `collaborators_overlay_a_binary_already_on_path_harmlessly` | wiring | fixture env, scratch fs | `cargo test --lib collaborators_overlay_a_binary_already` |
| refresh-worker: A binary found under a Homebrew prefix gets the same overlay | new `collaborators_overlay_a_binary_found_under_a_homebrew_prefix` | wiring | fixture env, scratch fs, closure | `cargo test --lib collaborators_overlay_a_binary_found_under` |
| refresh-worker: A shim whose interpreter is unreachable exits 127 and renders a problem row | existing `a_shim_whose_interpreter_is_unreachable_exits_127_and_renders_a_problem_row` (`src/refresh.rs:1122`) | unit | scratch stand-in | `cargo test --lib a_shim_whose_interpreter` |
| agent-prompts: The prompt carries the probe's own path, not a bare command | existing `each_intent_produces_its_own_text_against_the_same_binary_and_change` (`src/launch.rs:1406`), which pins the exact text | unit | none | `cargo test --lib each_intent_produces_its_own_text` |
| agent-prompts: No production file still produces an `/opsx:` prompt | existing `no_production_file_still_produces_an_opsx_prompt` (`tests/doc_contract.rs:4827`) | contract | source text | `cargo test --test doc_contract no_production_file_still_produces` |
| agent-prompts: File mode carries no path and builds no prompt | existing `file_mode_leaves_a_refusing_and_g_working_in_the_shipped_root` (`src/ui/mod.rs:5139`), gains `homebrew_prefixes: &[]` so a machine with Homebrew cannot leave file mode | wiring | scratch fs, fixture env, closure, stand-ins | `cargo test --lib file_mode_leaves_a_refusing` |
| agent-prompts: `g` still works with no binary | existing `g_still_works_with_no_binary` (`src/launch.rs:2542`) | unit | fake CLI | `cargo test --lib g_still_works_with_no_binary` |
| quality-gates: A name hidden in a block comment no longer satisfies the gate | existing controls `wired-launch-comment` and `wired-stripper-identity` | contract | tree copy | `cargo test --all-features --test gate_controls` |
| quality-gates: Deleting the panic-hook call fails the gate | existing control `wired-panic-hook-deleted` | contract | tree copy | `cargo test --all-features --test gate_controls` |
| quality-gates: Hardcoding the mouse-capture reason in `run` fails the gate | existing controls `wired-mouse-problem-hardcoded` and `wired-mouse-problem-renamed` | contract | tree copy | `cargo test --all-features --test gate_controls` |
| quality-gates: A renamed definition fails in the defining file | existing positive controls in `wired.sh`, and `wired-stripper-identity` | gate / contract | tree copy | `/bin/sh scripts/gates/wired.sh`; `cargo test --all-features --test gate_controls` |
| quality-gates: Emptying or relocating the Homebrew prefix list fails leg 8 | new controls `wired-homebrew-prefixes-emptied`, `wired-homebrew-prefixes-relocated` and `wired-homebrew-prefixes-renamed` | contract | tree copy | `cargo test --all-features --test gate_controls` |

A row naming an *existing* test means either that the scenario is carried unchanged, or that
only an input in its fixture moves. Task 1.1 re-counts every filter before editing, because a
filter selecting zero tests is a failed check.

## Decisions

1. **Step 5 goes last, after the npm prefix.** The alternatives were to place it between
   `PATH` and nvm, or between nvm and the npm prefix. Both are rejected:
   - A user with both an nvm-installed and a Homebrew-installed `openspec` gets the nvm one
     in their own shell. `brew shellenv` runs in `.zprofile`, and nvm prepends later from
     `.zshrc` (measured on the reference machine), so nvm ahead of Homebrew mirrors the shell.
   - `npm prefix -g` names whichever toolchain's `npm` is actually on `PATH`. That is a
     statement about the user's active setup, which a hardcoded prefix list is not. When the
     two agree (`/opt/homebrew/bin/npm prefix -g` → `/opt/homebrew`), they resolve the same
     path.
   - Appending keeps step 4 "the fourth step", as `AGENTS.md`, `cli.rs`'s doc comments and
     `subprocess-seam` call it (`grep -rn "fourth-step\|fourth probe"`), and as `SPEC.md`
     calls it at line 59 ("fourth step").
   - Cost is not a reason to move it earlier. The probe runs once per process, and a missing
     `npm` fails to start at once.
2. **Fixed paths, no environment variable, no spawn.** `HOMEBREW_PREFIX` is set by the same
   `brew shellenv` that puts Homebrew on `PATH`, so an environment carrying it already
   resolves at step 2. `brew --prefix` would add a second spawned program just to find a
   directory we can stat directly. A non-default prefix is `openspec_bin`'s job.
3. **The list is `/opt/homebrew`, `/home/linuxbrew/.linuxbrew`, `/usr/local`, in that order,
   and the order earns a test.** These are Homebrew's documented defaults for Apple Silicon,
   Linux, and Intel macOS (`/opt/homebrew/docs/Installation.md`, `FAQ.md`). Apple Silicon
   leads because a migrated Mac can keep a stale Rosetta Homebrew under `/usr/local`, and a
   swapped order would silently resolve the stale binary. Nothing else observes that order,
   because every behavioural test injects its own list. The constant is otherwise plumbing,
   but this one value is load-bearing and fails silently, so it gets the one assertion.
   `/usr/local/bin` may also hold another installer's global npm packages; step 5 labels
   those `Homebrew` too, which the spec records and Risks accepts.
4. **Injected as `&[&str]`, not `&[&Path]` or a hook.** `Path::new` cannot be called in a
   `const` on this toolchain. Measured: `const P: &[&Path] = &[Path::new("/opt/homebrew")];`
   gives `error[E0658]: cannot call conditionally-const associated function` on rustc 1.91.1.
   A `&[&str]` const needs no conversion in `run`, whose body `WIRED` leg 2 keeps free of
   logic. A `Fn() -> Vec<PathBuf>` hook would mirror `npm_hook`'s shape, but would add a
   closure where a value suffices. The list is data, not a computation.
5. **`BinSource::Homebrew`, labelled `Homebrew`.** The label names where the step looked, on
   the same terms as `nvm`. It does not name the directory the step found: `label` takes a
   `BinSource`, which carries no path, and widening it would move `setting-provenance`'s
   one-enum rule.
6. **`openspec-binary`'s ordered-steps requirement is renamed to drop its count.** It becomes
   "The binary is probed in ordered steps…", so the next step added moves no heading.
   OpenSpec refuses to drop or rename a scenario inside a MODIFIED block (confirmed in
   `specs-apply.js`, where RENAMED is applied first and MODIFIED must name the new header).
   So "The npm prefix is the last resort" keeps its title, and an AND clause records that it
   is the last step the *environment* can steer.
7. **`start_collaborators` takes a `ProbeBindings` bundle rather than an eighth parameter.**
   At 7 parameters, an eighth trips `clippy::too_many_arguments` under `-D warnings`. Measured
   on rustc 1.91.1: `error: this function has too many arguments (8/7)`. The repository
   carries no `#[allow(` anywhere in `src/`, and `ProbedStartup` exists for exactly this
   reason. Alternatives considered:
   - Passing `&Startup` whole. Rejected: the two direct test callers would have to invent a
     `cwd` and a `mouse_problem` they do not use.
   - Nesting the bundle inside `Startup`. Rejected: it renames `env` and `npm_hook` in all 14
     literals for no behavioural gain.
   
   The bundle is exactly the spec's "three bindings the probe needs". It derives nothing,
   `Default` included.
8. **`WIRED` binds the list in a body-scoped leg 8, not in leg 1.** Leg 1 searches the whole
   production slice of `src/ui/mod.rs`. Because the search is case-sensitive and the field is
   `homebrew_prefixes`, leg 1 would catch a `run` passing `&[]`. It would *not* catch the
   constant moving into `start_collaborators`, whose body sits in the same slice. That move
   makes the injection decorative, since every test drives `start_collaborators` with its own
   list. So leg 8 is scoped like leg 7 (`GIT_PROGRAM`): `run`'s body names the constant, and
   `start_collaborators`' body does not. The plants are recorded in `tests/gate-controls.toml`
   on `wired-git-literal`'s precedent, so a leg 8 reduced to `true` fails `cargo test`.

## Risks / Trade-offs

- **Risk:** `/usr/local/bin/openspec` from a non-Homebrew install is labelled `Homebrew`.
  **Mitigation:** accepted. The binary is still the right one to run. Only the settings
  panel's provenance word is imprecise, and `openspec_bin` overrides it.
- **Risk:** A stale `openspec` under a Homebrew prefix wins where file mode used to be
  shown. **Mitigation:** this can only happen when steps 1–4 find nothing, where the old
  answer was no CLI at all. A failing CLI already degrades to a problem row and file-sourced
  changes (`cli-changes`).
- **Risk:** Step 5 executes a binary from a directory outside the pane's `PATH`.
  **Mitigation:** each prefix is a system location that needs administrator rights to
  create. Once Homebrew has claimed it, it is writable by the user who installed Homebrew,
  which is no more than step 3 already trusts in `~/.nvm` and step 2 in `PATH`. Step 5 only
  executes the one file at `<prefix>/bin/openspec` and never searches.
- **Risk:** An outer test passes on CI while resolving the developer's Homebrew.
  **Mitigation:** `Startup`, `ProbedStartup` and `ProbeBindings` have no `Default` and are
  built by struct literal, so omitting the field does not compile. The only way to reach the
  real list is to name `HOMEBREW_PREFIXES`, and task 2.5's sweep confirms that no test slice
  outside `src/resolve.rs`'s two source-reading tests does.
- **Risk:** Delta blocks carry large live requirements for small edits, notably
  `refresh-worker`'s overlay requirement and `quality-gates`' `WIRED` requirement.
  **Mitigation:** a block archived stale reverts whatever landed in between. `worktree-agents`,
  the one other active change, touches none of these five capabilities. It does edit other
  paragraphs of `SPEC.md` and `AGENTS.md`, which is a merge-order concern only. Re-diff every
  MODIFIED block at archive if anything else archives first.
- **Risk:** Some specs outside this change still list the probe's first four inputs, and
  were deliberately left alone. **Mitigation:** each stays satisfiable, because `Startup`
  forces the new field and they never say "four":
  - `agent-launch` "File mode leaves `a` refusing…", which `worktree-agents` holds;
  - `dashboard-loop` "File mode opens the archive…";
  - `agent-poller`'s account of `Startup`'s fields, already stale at HEAD.
  
  They are recorded here so a later reviewer does not re-raise them.

## Migration Plan

No data migration, and no rollback steps beyond `git revert`. After `openspec archive
homebrew-probe` has written the specs, task 6.9 rewrites `openspec-binary`'s `## Purpose` from
"a four-step probe chain over …" to name five steps and the Homebrew prefixes. A delta cannot
carry a Purpose (the CLI ignores one for an existing spec), and `tests/spec_purposes.rs`
checks only that a Purpose is present, not that it is accurate.

## Open Questions

None.
