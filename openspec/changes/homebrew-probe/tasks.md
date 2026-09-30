Ordering: sequential. Groups 1 → 2 → 3 each need the previous group's code to exist. Group 2
forwards the parameter group 1 adds, and group 3's gate names the value group 2 passes, so
criterion 2 fires for every adjacent pair. Any pair it does not rule out is ruled out by the
standing parallelism veto in `openspec/config.yaml` → rules → tasks: one crate, one compile.
There is no group 0. See design.md → Test Strategy for why the acceptance test is group 2's
first RED.

The change directory is committed at planning time (`docs(openspec): propose homebrew-probe`),
so `OPENSPEC-UNTOUCHED`, which fails on any untracked file under `openspec/`, is green from
group 1 on.

Planning-time selection counts below come from
`cargo test --all-features --lib -- --list 2>/dev/null > "$T/list.txt"`, run at `c261940`
(1700 lines). `$T` is any scratch directory outside the repository.

## 1. The fifth probe step
<!-- kind: behavior -->

- [x] 1.1 RED setup: Regenerate `$T/list.txt` with the command above, then count every filter
      in design.md → Test Strategy:
      `grep -o 'cargo test --lib [^ \`]*' openspec/changes/homebrew-probe/design.md | awk '{print $4}' | sort -u | while read f; do echo "$(grep -c -- "$f" "$T/list.txt") $f"; done`.
      At planning time, every existing filter printed `1`, and every filter for a new test
      printed `0`. An existing filter printing `0`, or any filter printing `2`, is a failed
      check.
- [x] 1.2 RED: In `src/resolve.rs`'s tests, write these new tests:
      - `a_homebrew_prefix_is_searched_when_nothing_earlier_resolves`, including the empty-list
        arm and the `HOMEBREW_PREFIX`-in-env arm;
      - `the_npm_prefix_outranks_the_homebrew_prefixes`;
      - `homebrew_prefixes_are_tried_in_list_order_skipping_one_without_a_binary`, with its
        directory, non-executable and dangling-link arms;
      - `homebrew_candidates_drop_empty_blank_and_relative_entries`;
      - `the_production_prefix_list_puts_apple_silicon_first_and_intel_last`;
      - `the_composition_passes_the_production_homebrew_prefix_list`, cut and stripped as in
        design.md → Test Strategy.

      Change these existing tests:
      - extend `nothing_anywhere_is_a_supported_state_not_a_fault` with the
        empty-plus-never-created re-run;
      - rework `a_full_probe_leaves_the_filesystem_byte_identical` so steps 1–4 all miss and
        step 5 resolves `B` through `[E, B]`;
      - extend `every_probe_step_is_named_by_the_step_that_won` in `src/settings.rs` with
        `BinSource::Homebrew` and `assert_eq!(…label(), "Homebrew")`.

      Expect a compile failure naming `openspec_bin`'s arity, `BinSource::Homebrew`,
      `homebrew_candidates`, and `HOMEBREW_PREFIXES`: the missing interface.
- [x] 1.3 GREEN: Add these to `src/resolve.rs`, per design.md → Boundaries and Decisions 1–5:
      - `pub const HOMEBREW_PREFIXES: &[&str]`;
      - `BinSource::Homebrew`;
      - `homebrew_candidates`;
      - `step5_homebrew`;
      - `openspec_bin`'s fourth parameter `homebrew_prefixes: &[&str]`, probed after step 4.

      Pass `HOMEBREW_PREFIXES` from `openspec_bin_from_env`. Add the `Homebrew` arm to
      `Provenance::label`. Pass `&[]` at every other call site:
      - the 29 `super::openspec_bin(` calls (`grep -c "super::openspec_bin(" src/resolve.rs` → 29);
      - the 2 calls in `src/cli.rs`'s tests (`grep -c "crate::resolve::openspec_bin(" src/cli.rs` → 2);
      - `start_collaborators`' one call, which group 2 replaces.

      Rewrite the two `src/resolve.rs` doc comments design.md → Boundaries lists, and
      `src/launch.rs:288`'s "four-step".
- [x] 1.4 REFACTOR: If steps 4 and 5 both spell `prefix.join("bin").join("openspec")`, extract
      one private helper that both call. Otherwise record that no refactor was needed.
- [x] 1.5 Run the group's tests and prove the source-text test can fail:
      - `cargo test --all-features --lib resolve::tests` → 47 passed. That is 41 at planning
        time (`grep -c '^resolve::tests::' "$T/list.txt"`) plus six new tests.
      - `cargo test --all-features --lib settings::tests` → 9 passed, unchanged.
      - Plant `&[]` in place of `HOMEBREW_PREFIXES` in `openspec_bin_from_env`. See
        `the_composition_passes_the_production_homebrew_prefix_list` fail, revert, and see it
        pass.
      - `cargo test --all-features --lib` is green, and `make lint` reports 0 warnings.
- [x] 1.6 Commit: `feat(resolve): probe Homebrew's default prefixes as a fifth step`.

## 2. The composition root injects the Homebrew prefix list
<!-- kind: behavior -->

- [ ] 2.1 RED: In `ui::tests::wiring`, write the acceptance test
      `a_homebrew_only_install_leaves_file_mode` at 120×20 and 60×20, with the four assertions
      in design.md → Test Strategy. Also write
      `collaborators_overlay_a_binary_found_under_a_homebrew_prefix`, which asserts that the
      overlay `PATH` is `B/bin:/usr/bin:/bin`. Expect a compile failure on the missing
      `homebrew_prefixes` field and `ProbeBindings`.
- [ ] 2.2 RED (behavioural): Add `ProbeBindings`, `Startup::homebrew_prefixes` and
      `ProbedStartup::homebrew_prefixes`. Reshape `start_collaborators` to
      `(repo, config, herdr, state_dir, probe, git)`, still handing `&[]` to `openspec_bin`.
      Fill every literal and call site exactly as design.md → Contracts lists them. `run`
      passes `crate::resolve::HOMEBREW_PREFIXES`, and `run_wired_probed`'s literal forwards
      `p.homebrew_prefixes`. Run
      `cargo test --all-features --lib ui::tests::wiring::a_homebrew_only_install` and record
      that it **fails on `!dashboard.file_mode`**, 1 test selected.
- [ ] 2.3 GREEN: In `start_collaborators`, hand `probe.homebrew_prefixes` to
      `resolve::openspec_bin`, and read `probe.env` for the overlay. Rewrite the two
      `src/ui/mod.rs` doc comments that design.md → Boundaries lists.
- [ ] 2.4 CHECK: Contract gate. Run
      `git diff -U0 c261940 -- src | grep -E '^[-+].*(fn openspec_bin\(|fn start_collaborators\(|homebrew_prefixes|ProbeBindings|Homebrew)'`
      and confirm that every changed signature is one design.md → Contracts names, and that
      each named consumer compiles against it.
- [ ] 2.5 CHECK: No test slice outside `src/resolve.rs` names the real list. Run
      `for f in $(grep -rl HOMEBREW_PREFIXES src); do [ "$f" = src/resolve.rs ] && continue; c=$(grep -n '^#\[cfg(test)\]' "$f" | head -1 | cut -d: -f1); [ -n "$c" ] && echo "$(awk -v c="$c" 'NR>c && !/^[[:space:]]*\/\//' "$f" | grep -c HOMEBREW_PREFIXES) $f"; done`.
      It prints only `0 …` lines. Negative control: add
      `let _ = crate::resolve::HOMEBREW_PREFIXES;` to one test in `src/ui/mod.rs`, see `1
      src/ui/mod.rs`, then revert.
- [ ] 2.6 REFACTOR: none expected, since the bundle mirrors the parameters it replaces field for
      field. Record that, or the cleanup made.
- [ ] 2.7 Run the group's tests and lint:
      - `cargo test --all-features --lib ui::tests::wiring` → 41 passed. That is 39 at
        planning time (`grep -c '^ui::tests::wiring::' "$T/list.txt"`) plus two new tests.
      - `cargo test --all-features --lib` is green.
      - `make lint` reports 0 warnings, so `start_collaborators` is at 6 parameters.
- [ ] 2.8 Commit: `feat(ui): inject the Homebrew prefix list through Startup`.

## 3. `WIRED` leg 8 binds the value `run` passes
<!-- kind: operational -->

- [ ] 3.1 CHECK: Against the *unextended* gate, confirm it misses both defects:
      - plant 1: in `run`, replace `homebrew_prefixes: crate::resolve::HOMEBREW_PREFIXES,` with
        `homebrew_prefixes: &[],`;
      - plant 2: restore `run`, and make `start_collaborators` pass
        `crate::resolve::HOMEBREW_PREFIXES` to `openspec_bin` in place of
        `probe.homebrew_prefixes`.

      Run `/bin/sh scripts/gates/wired.sh` after each and record both exit statuses. Expect
      exit **0** for both, since no leg names `HOMEBREW_PREFIXES` yet: that is the gap leg 8
      closes. Revert both. This cannot run at planning time, because the planted line only
      exists after group 2.
- [ ] 3.2 CHANGE: In `scripts/gates/wired.sh`, add `RESOLVE=src/resolve.rs` beside `$CLI`, and
      a positive control `grep -qE '^pub const HOMEBREW_PREFIXES' "$RESOLVE" || fail "…"`
      beside `npm_probe_hook`'s. Add **leg 8** on leg 7's terms:
      - `printf '%s\n' "$body" | grep -q HOMEBREW_PREFIXES` must succeed;
      - `start_collaborators`' body, cut the way `run`'s is, must not name `HOMEBREW_PREFIXES`;
      - each failure message begins `WIRED FAIL: leg 8`.

      Append `run names HOMEBREW_PREFIXES (leg 8)` to the `WIRED OK` line. In
      `tests/gate-controls.toml`, add `wired-homebrew-prefixes-emptied` (plant 1, expect
      `WIRED FAIL: leg 8`), `wired-homebrew-prefixes-relocated` (plant 2, expect
      `WIRED FAIL: leg 8`), and `wired-homebrew-prefixes-renamed` (rename the constant in
      `src/resolve.rs` only, expect the positive control's message). Each gets a `why`, per
      design.md → Decision 8.
- [ ] 3.3 VERIFY:
      - `/bin/sh scripts/gates/wired.sh` exits 0.
      - Plants 1 and 2 each exit 1 naming leg 8, and the rename exits 1 naming
        `src/resolve.rs`. Revert each.
      - `make gates` exits 0.
      - `cargo test --all-features --test gate_controls` is green. Run it with the tree quiet
        for its ~78 s: it fails on any concurrent edit.
- [ ] 3.4 Commit: `test(gates): bind the Homebrew prefix list to run in WIRED leg 8`.

## 4. Change Review
<!-- kind: operational -->

- [ ] 4.1 CHECK: Dispatch an `outside-in-tdd-reviewer`, giving it only this change's artifacts
      and `git diff c261940..HEAD`, never this session's reasoning. It reports CRITICAL /
      WARNING / SUGGESTION findings against the proposal, all five delta specs, the design and
      the tasks.
- [ ] 4.2 CHANGE: Fix every CRITICAL. Resolve each WARNING, or accept it with a one-line reason
      recorded here. Note SUGGESTIONs, and re-run the affected groups' tests.
- [ ] 4.3 VERIFY: Confirm that no blocking or unowned finding remains, and commit any fixes.
- [ ] 4.4 VERIFY: Live check on the reference machine, whose only `openspec` is
      `/opt/homebrew/bin/openspec`. Run `make build`. Then, in a real terminal at the repository
      root, run
      `env -i HOME="$HOME" TERM="$TERM" PATH=/usr/bin:/bin ./target/release/herdr-openspec ui`.
      The header shows no `file mode` badge, and the settings panel (key listed under `?`)
      shows the `openspec_bin` row reading `/opt/homebrew/bin/openspec` with provenance
      `Homebrew`. Record what was seen. This is the one step no automated test reaches.

## 5. Documentation
<!-- kind: operational -->

- [ ] 5.1 Rewrite in `SPEC.md` → the binary probe chain (audience: implementers):
      - Append step 5 to the list, which ends at step 4 on line 303.
      - Extend the paragraph "Steps 3 and 4 exist because…" (line 316) to say why step 5
        exists and why it is a fixed, injected list.
      - Change "four-step" to "five-step" at lines 1008 and 1315
        (`grep -n "four-step" SPEC.md`).
      - Beside "nvm is lazy-loaded" (line 1011), add the Homebrew reason: `brew shellenv` runs
        only from `.zprofile`.

      Durable because `SPEC.md` is the design contract and states the chain's shape. Net
      about +8 lines. Nothing else there goes stale.
- [ ] 5.2 Rewrite in `AGENTS.md` (the file `CLAUDE.md` links to; audience: every agent
      session):
      - In Environment's "OpenSpec CLI" bullet, replace "Installed under nvm here" with the
        Homebrew install at `/opt/homebrew/bin/openspec`.
      - In the "Current repo state" sentence about what arrives on `Startup`, name the
        Homebrew prefix list beside the npm hook, as the third binding a test must inject.

      Durable because a future change that reaches the real machine in a test would otherwise
      miss it. Net 0 to +2 lines, replacing false text. `worktree-agents` edits other
      paragraphs of this file, so merge carefully.
- [ ] 5.3 Rewrite in `openspec/config.yaml` → `context` the sentence saying the `openspec` CLI
      is nvm-installed (audience: every OpenSpec agent, which receives it verbatim). It is
      Homebrew-installed at `/opt/homebrew/bin/openspec`; prepend `/opt/homebrew/bin` when it
      is not on `PATH`. Leave the `make` command names and the fixture wording intact, since
      `tests/doc_contract.rs` binds them. Net 0 lines.
- [ ] 5.4 VERIFY: `cargo test --all-features --test doc_contract` is green.
      `grep -n "nvm-installed\|Installed under nvm" AGENTS.md openspec/config.yaml` prints
      nothing. Commit: `docs: record the Homebrew probe step and the Homebrew-installed CLI`.

## 6. Lint & Verify
<!-- kind: operational -->

- [ ] 6.1 CHECK: The affected tiers are:
      - unit (`resolve`, `settings`);
      - wiring (`ui::tests::wiring`);
      - gate (`WIRED`);
      - contract (`doc_contract`, `gate_controls`).

      All of them run inside `make check`. Run it with no other session editing this tree.
- [ ] 6.2 VERIFY: `make lint` → 0 warnings.
- [ ] 6.3 VERIFY: `make fmt-check` → clean.
- [ ] 6.4 VERIFY: `make gates` → exit 0.
- [ ] 6.5 VERIFY: `make test` → green.
- [ ] 6.6 VERIFY: `make coverage` → both floors hold.
- [ ] 6.7 VERIFY: `openspec validate homebrew-probe --strict` → valid.
- [ ] 6.8 VERIFY: `make check` as the single gate → exit 0. If it fails, name the failing
      sub-command here.
- [ ] 6.9 After `openspec archive homebrew-probe`, rewrite `openspec/specs/openspec-binary/spec.md`
      → `## Purpose` to name five steps and the Homebrew prefixes. `grep -n "four-step"
      openspec/specs/openspec-binary/spec.md` must print nothing. Commit it with the archive.
