Ordering: sequential. The standing parallelism veto in `openspec/config.yaml` → rules → tasks
applies (one crate, one compile, and every gate sweeps the whole tree). The first criterion
to fire is also a shared file: groups 1, 2, 4, and 5 all edit `tests/publish_surface.rs`, and
groups 1, 4, and 5 all edit `README.md`.

No outer-loop group 0 (design.md → Test Strategy): each claim's real-file test is its own
acceptance test, and it goes RED for the real reason, a missing file or key.

Planning-time evidence at HEAD `e146943` was recorded with the command beside each value:
- `test -f LICENSE` returned exit 1.
- `grep -c '^license\|^repository\|^description' Cargo.toml` returned 0.
- `test -d docs` returned exit 1.
- `grep -c '!\[' README.md` returned 1, which is the CI badge, an absolute URL and not under
  `docs/`.

## 1. Licence agreement
<!-- kind: behavior -->

- [ ] 1.1 RED: Create `tests/publish_surface.rs` with a pure `check_licence(licence: Option<&str>, cargo_toml: &str, readme: &str) -> Result<(), Vec<String>>`. Add one planted-input test per failure scenario (`licence_file_missing_is_named`, `cargo_without_license_is_named`, `readme_naming_another_licence_is_named`, `readme_without_licence_section_fails`) and `licence_sites_agree` over the real files. Run `cargo test --test publish_surface licence`: 5 tests should execute, the four plants passing and `licence_sites_agree` RED on `LICENSE` missing.
- [ ] 1.2 GREEN: Add `LICENSE` (the standard MIT text, `Copyright (c) 2026 Juuso Piikkilä`) and `license = "MIT"` under `[package]` in `Cargo.toml`, and make `README.md` → `## Licence` read `[MIT](LICENSE)`. Run `cargo test --test publish_surface licence`: 5 executed, 5 passed.
- [ ] 1.3 REFACTOR: Extract any README-section helper that group 2 will reuse. If the checker is already minimal, record that no refactor was needed.
- [ ] 1.4 Run `cargo test --test publish_surface --test manifest --test doc_contract` and confirm no regressions. Commit `feat(publish): license the crate under MIT and bind the three sites`.

## 2. Repository metadata agreement
<!-- kind: behavior -->

- [ ] 2.1 RED: Add `check_repository(cargo_toml: &str, readme: &str)` with plants `moved_repository_names_both_slugs`, `missing_description_is_named`, `empty_description_is_named`, `missing_repository_is_named`, plus the real-file `repository_slugs_agree`. Run `cargo test --test publish_surface repository`: 5 executed, with `repository_slugs_agree` RED on the missing `repository`.
- [ ] 2.2 GREEN: Add `description` (one sentence matching the README hook) and `repository = "https://github.com/optioni/herdr-openspec"` to `Cargo.toml` `[package]`. Run `cargo test --test publish_surface repository`: 5 passed.
- [ ] 2.3 Run `cargo build` to confirm the manifest still parses (the build is the check for the metadata, per the schema's plumbing rule). Then run `cargo test --test publish_surface` and confirm no regressions. Commit `feat(publish): declare package metadata and bind the repository slug`.

## 3. Demo repository
<!-- kind: behavior -->

- [ ] 3.1 RED: Add `check_demo(set: &ChangeSet) -> Result<(), Vec<String>>` (field access only, no literals, per design.md → Boundaries), a recursive `copy_tree` into `env!("CARGO_TARGET_TMPDIR")`, and a path-and-bytes `digest`. Add tests `demo_loads_with_no_problems` (real `docs/demo`), `demo_without_schema_quotes_the_problem`, `demo_change_without_tasks_is_named`, `demo_without_partial_progress_fails` (scratch copies), and `demo_check_leaves_fixtures_untouched` (digests `docs/demo/` and top-level `openspec/` before and after). Run `cargo test --test publish_surface demo_repo`: 5 executed, all RED because `docs/demo` does not exist.
- [ ] 3.2 GREEN: Write `docs/demo/openspec/`: `config.yaml` (`schema: spec-driven`), the vendored `schemas/spec-driven/schema.yaml` with its source and licence header (design.md → Decision 2), and a fictional todo-CLI project per Decision 3. It needs two active changes (one mid-way, one proposed with all-unchecked tasks), one archived complete change, and specs for at least one capability. Run `cargo test --test publish_surface demo_repo`: 5 passed.
- [ ] 3.3 CHECK: Run `/bin/sh scripts/gates/openspec-untouched.sh` and confirm it prints `OPENSPEC-UNTOUCHED OK` with the demo in place, proving that `docs/demo/openspec/` is outside the gate's `:(top)openspec/` sweep.
- [ ] 3.4 Run `cargo test --test publish_surface` and confirm no regressions. Commit `test(publish): add the demo OpenSpec repository and prove it loads`.

## 4. README rewrite
<!-- kind: operational -->

- [ ] 4.1 CHECK: Run `cargo test --test doc_contract --test manifest` before editing and record the executed and passed counts. These are the characterization of every README fact already bound.
- [ ] 4.2 CHANGE: Rewrite `README.md` in this order: hook, image placeholder line (embedded in group 5), what you get, requirements, Install, a quick start, `## Keys` (plainer wording with the same key atoms), Configuration, "Built with OpenSpec" (no counts, per design.md → Decision 5), `## Development`, `## Licence`. Drop "Status: complete" and the change count. Keep both action titles and every Development fact the contract tests bind.
- [ ] 4.3 VERIFY: Re-run `cargo test --test doc_contract --test manifest --test publish_surface` and confirm the same counts as 4.1 plus publish_surface's, all passed. Commit `docs(readme): rewrite the README for first-time readers`.

## 5. Demo image
<!-- kind: behavior -->

- [ ] 5.1 RED: Add `check_demo_image(readme: &str, gif: Option<&[u8]>, tape: &str)` with plants `unrendered_image_is_named`, `empty_file_is_not_a_gif`, `png_is_not_a_gif`, `oversized_gif_states_size_and_cap`, `tape_output_mismatch_names_both`, `readme_without_image_fails`, `image_below_install_fails`, plus the real-file `demo_image_agrees`. Run `cargo test --test publish_surface demo_image`: 8 executed, with `demo_image_agrees` RED.
- [ ] 5.2 GREEN: Write `docs/demo/demo.tape`. It hides the setup, then runs `../../target/release/herdr-openspec ui` from `docs/demo`, and walks the list, opens a change, switches tabs, folds a section, and quits, with `Output docs/demo.gif`. Run `make build` and then `vhs docs/demo/demo.tape` from the repository root (if `vhs` is missing, ask the user to `brew install vhs`; see design.md → Open Questions). Then embed `docs/demo.gif` above `## Install`.
- [ ] 5.3 CHECK: Run `ls -l docs/demo.gif` and confirm it is at most 2 MiB (2097152 bytes). If not, reduce the tape's dimensions, framerate, or length and re-render (design.md → Decision 8).
- [ ] 5.4 Run `cargo test --test publish_surface`: all checks pass. Commit `docs(readme): add the recorded demo and bind it to its tape`.

## 6. Root cleanup
<!-- kind: operational -->

- [ ] 6.1 CHECK: Re-verify that each deferred item is still open, with `grep -c 'fn columns\|fn truncate_columns' src/ui/layout.rs` (4 at HEAD), `grep -c 'src/launch.rs' scripts/gates/nosleep.sh` (0 at HEAD), and `grep -c 'fn g_focuses_the_agent_the_launch_started\|fn a_failed_cli_cycle_keeps_the_file_numbers\|fn the_real_wiring_polls_a_scratch_herdr' src/ui/mod.rs` (3 at HEAD). Confirm `NIGHT.md`'s items are resolved with `git -C ~/Code/openspec-schemas tag --contains 7e31a8f` (prints `v0.6.0`).
- [ ] 6.2 CHANGE: Write `docs/known-issues.md`, one entry per open item with its statement, evidence, and deferral reason (carried from `HANDOFF-AUDIT.md` → "Three deferred items"). Then `git rm HANDOFF.md HANDOFF-AUDIT.md NIGHT.md`.
- [ ] 6.3 VERIFY: Run `grep -c 'truncate_columns\|nosleep.sh\|g_focuses_the_agent_the_launch_started' docs/known-issues.md` (at least 3), and `ls HANDOFF.md HANDOFF-AUDIT.md NIGHT.md`, where all three should be absent. Run `cargo test --test manifest` (its comment cites `HANDOFF.md`, design.md → Decision 7). Commit `chore: retire the session handoff records into docs/known-issues.md`.

## 7. Change Review
<!-- kind: operational -->

- [ ] 7.1 CHECK: Dispatch `outside-in-tdd-reviewer` against proposal, specs, design, tasks, and the diff `e146943..HEAD`. Concentration points: every planted test can fail; the README still satisfies every bound fact; the demo check really is file mode; and the GIF is a real render of the current UI.
- [ ] 7.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a one-line reason, and re-run the affected tests.
- [ ] 7.3 VERIFY: Confirm no blocking or unowned finding remains.

## 8. Documentation
<!-- kind: operational -->

- [ ] 8.1 Rewrite in `SPEC.md` → Build and distribution (audience: future implementers): replace "Before publishing, that script gains a fast path" with the same fast path stated as planned, separate work. This corrects a precondition this change deliberately does not meet. Net size about 0.
- [ ] 8.2 Rewrite in `AGENTS.md` → Quality gates, contract-tier paragraph (audience: every agent session): "three members" becomes four, adding `tests/publish_surface.rs` (licence, repository slug, demo image, demo repository) in one clause. This is durable because a future README or `Cargo.toml` edit will otherwise not know why `make check` went red. It adds at most 3 lines.
- [ ] 8.3 Add the publication hand-off at `openspec/changes/publish-readiness/publish-handoff.md` (audience: the user): the `gh repo edit optioni/herdr-openspec --description … --add-topic …` command, the showcase line for `Fission-AI/OpenSpec` → `docs/community.md`, and a PR body. Nothing is run (design.md → Decision 9).

## 9. Lint & Verify
<!-- kind: operational -->

- [ ] 9.1 CHECK: The affected tiers are the contract tier (`tests/publish_surface.rs`, `tests/doc_contract.rs`, `tests/manifest.rs`) and the hygiene gates, because the tree gained files. No production code changed, so coverage moves only by its denominator.
- [ ] 9.2 VERIFY: Run `make fmt-check`: clean.
- [ ] 9.3 VERIFY: Run `make lint`: 0 warnings.
- [ ] 9.4 VERIFY: Run `make gates`: every gate OK.
- [ ] 9.5 VERIFY: Run `make test`: green, with `publish_surface` showing 23 tests executed (5 + 5 + 5 + 8, counted from groups 1, 2, 3, and 5).
- [ ] 9.6 VERIFY: Run `make coverage`: both floors hold.
- [ ] 9.7 VERIFY: Run `make check` as the single gate, and name the failing sub-command if it fails.
- [ ] 9.8 VERIFY: Run `openspec validate publish-readiness --strict`.
