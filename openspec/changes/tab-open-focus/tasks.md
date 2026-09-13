# Tasks — tab-open-focus

**Parallelism: none.** Groups 1, 2, and 3 all write `src/open.rs`, and groups 5 and 7 both
write `SPEC.md`; one file is shared mutable state. Group 5's check (`tests/degraded_coverage.rs`)
also needs group 3's proof tests to exist and pass. No pair qualifies, so nothing is marked
`parallel-after`.

**Counts in this plan, and the commands that produced them.**

```
grep -c '^#### Scenario:' openspec/changes/tab-open-focus/specs/pane-open/spec.md   -> 24  (8 + 1 added in planning review, then 7, then 8 in the MODIFIED block)
grep -c '#\[test\]' src/open.rs                                                     -> 37
grep -n 'existing_pane' src/open.rs | awk -F: '$1<424' | wc -l                      -> 4  (production: 1 doc line, 1 definition, 1 doc reference, 1 call site)
grep -rn 'existing_pane' src/ tests/ | grep -vc 'src/open.rs'                        -> 0  (no caller outside this module)
```

## 0. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

The end-to-end wiring is precisely what failed here: every unit test passed while the real
binary never issued the call. This group takes the outer loop for that reason
(design.md → Test Strategy).

- [x] 0.1 Add `stub_herdr_sequenced` to `tests/cli.rs` beside `stub_herdr`, answering the
      first `pane list` of a process with `{"result":{"panes":[]}}` and later ones with
      `{"pane_id":"w8:pG","label":"OpenSpec","workspace_id":"w8"}`, keyed on a counter file in
      its own scratch dir. `stub_herdr` is left untouched, and no reset affordance is built —
      no test consumes one (design.md → Decision 4).
- [x] 0.2 RED: Write `main_focuses_the_pane_it_just_opened` in `tests/cli.rs`, driving the
      real built binary for `open-tab` against that stub and asserting `argv.log` ends with
      `plugin pane focus w8:pG`. It covers the acceptance row of "A tab open is followed by a
      listing and a focus". Build the spawn from `scrubbed()`, never `bin()`, and pipe or null
      stdout: `every_run_pipes_and_scrubs_herdr` (`tests/cli.rs:460`) enforces that on any spawn
      site naming `open`/`open-tab`. Its `MIN_SPAWN_SITES = 9` is a `>=` floor, so adding a site
      cannot break it.
- [x] 0.3 Confirm it fails because the focus call is missing, not because the stub is
      misconfigured — the shell reproduction below is the same check outside Rust and was run
      at HEAD.

```sh
# Run at HEAD: exit 0 from the binary, and NO post-open focus in the log.
S=$(mktemp -d); mkdir -p "$S/bin"
cat > "$S/bin/herdr" <<'EOF'
#!/bin/sh
D="$(dirname "$0")/.."
echo "$@" >> "$D/argv.log"
case "$*" in
  "pane list")
    if [ -f "$D/seen" ]; then
      printf '{"result":{"panes":[{"pane_id":"w8:pG","label":"OpenSpec","workspace_id":"w8"}]}}'
    else : > "$D/seen"; printf '{"result":{"panes":[]}}'; fi ;;
  *) printf '' ;;
esac
exit 0
EOF
chmod 755 "$S/bin/herdr"
PATH="$S/bin:$PATH" HERDR_WORKSPACE_ID=w8 \
  HERDR_PLUGIN_CONTEXT_JSON='{"workspace_id":"w8","focused_pane_id":"w8:p1"}' \
  ./target/debug/herdr-openspec open-tab
grep -q "plugin pane focus" "$S/argv.log"
```

**HEAD result: RED as required.** The binary exited 0 and `argv.log` held exactly two lines —
`pane list`, then `plugin pane open … --placement tab --workspace w8 --focus`. The final
`grep` exited 1: no post-open focus is issued today.

## 1. The two pure matchers
<!-- kind: behavior -->

- [x] 1.1 RED: Write failing unit tests in `src/open.rs` for the seven Requirement-2
      scenarios: `a_pane_absent_before_and_present_after_is_chosen`,
      `a_pre_existing_dashboard_is_not_chosen_when_a_new_one_appears`,
      `two_new_matches_choose_the_first_in_order`,
      `every_match_already_present_falls_back_to_the_first`,
      `a_vanished_pre_open_id_is_ignored`, `no_post_open_match_chooses_nothing`, and
      `an_unparseable_listing_is_the_same_error_from_both_matchers`.
- [x] 1.2 GREEN: Add `dashboard_panes(listing, workspace_id) -> Result<Vec<String>, String>`,
      lifting `existing_pane`'s three-part test to yield every match in listing order and
      keeping its two error reasons verbatim. Its `pane_id` clause requires a **non-empty**
      string, which `existing_pane` does not enforce at HEAD (`src/open.rs:116`).
- [x] 1.3 GREEN: Add `opened_pane(before: &[String], after: &[String]) -> Option<String>`,
      total over both lists: the first `after` id absent from `before`, else `after`'s first,
      else `None`.
- [x] 1.4 REFACTOR: Clean up while green, or state that none was needed.
- [x] 1.5 Run `cargo test --all-features --lib open::tests` — green, and the 37 tests already in
      `src/open.rs` still pass.

## 2. `existing_pane` re-expressed through the extractor
<!-- kind: refactor -->

- [ ] 2.1 CHARACTERIZE: Confirm the **five** test functions that call `existing_pane` are green
      and leave their *matcher* assertions unedited — they are the contract this refactor must
      preserve (`awk '/#\[test\]/{t=NR} /existing_pane\(/{if(t)print t}' src/open.rs | sort -u
      | wc -l` → 5; `cargo test --all-features --lib open::tests` at HEAD → 37 passed).
      This does **not** extend to the `run`-level call counts inside `no_match_opens_instead`,
      which group 3.4 must change; only its three `existing_pane(..)` assertions are frozen.
- [ ] 2.2 REFACTOR: Reduce `existing_pane`'s body to the first element of
      `dashboard_panes(..)`, keeping its signature and its one production call site, so the
      three-part test exists once (design.md → Decision 6).
- [ ] 2.3 VERIFY: Run `cargo test --all-features --lib open::tests` — the unchanged characterization
      tests still pass.

## 3. `run`'s post-open listing and focus
<!-- kind: behavior -->

- [ ] 3.1 RED: Write failing tests for the nine Requirement-1 scenarios over `cli::FakeCli`:
      `a_tab_open_lists_then_focuses`, `a_split_open_takes_the_same_path`,
      `a_failed_second_listing_warns_and_succeeds`,
      `an_unparseable_second_listing_warns_and_succeeds`,
      `an_empty_second_listing_warns_rather_than_focusing_nothing`,
      `a_post_open_focus_failure_warns_on_every_code`,
      `the_post_open_focus_names_the_newly_opened_pane`,
      `a_failed_open_makes_no_post_open_call`, and `the_already_open_path_is_untouched`.
      Register `["pane","list"]` twice on the fake — its per-argv `VecDeque` returns them in
      order and repeats the last (design.md → Test Boundaries).
- [ ] 3.2 RED: Assert every one of those on `fake.calls()` — the recorded argument vectors —
      not on `outcome` alone. `FakeCli` panics only on an *unregistered* call, so a test that
      checks `outcome == Ok` stays green with the whole post-open step deleted.
      `the_post_open_focus_names_the_newly_opened_pane` is the single test that falsifies an
      implementation passing a hardcoded empty pre-open id list; every other scenario either
      starts from an empty listing or leaves `before` and `after` equal.
- [ ] 3.3 GREEN: Collect the pre-open dashboard ids from the first listing, and after a
      successful `plugin pane open` issue a second `pane list`, choose the pane with
      `opened_pane`, and focus it — every failure after the open pushing a warning and leaving
      `outcome` as `Ok`.
- [ ] 3.4 GREEN: Update the **five** existing tests the new call sequence invalidates. Every
      test that reaches a *successful* `plugin pane open` is affected, because `FakeCli`
      repeats its last response per argv and so re-answers the second `pane list` with the
      first listing:
      - `focus_usage_error_warns_and_opens_once` — call count becomes "first three calls are".
      - `listing_failure_warns_and_still_opens` — reaching the spec's "the post-open listing's
        first match is the pane focused" clause needs a **second** `Ok(labelled)` registered
        for `["pane","list"]`; with only the `Err` registered, repeat-last makes the post-open
        listing fail too and no focus follows.
      - `successful_open_is_silent` (`src/open.rs:1153`) — assert `warnings` is empty **and**
        four calls whose fourth is the focus.
      - `no_match_opens_instead` (`src/open.rs:522`) — asserts `calls.len() == 2`; becomes 3
        (list → open → list, with no focus since neither listing carries a dashboard pane).
      - `a_dead_focused_pane_opens_against_a_live_one` (`src/open.rs:1102`) — asserts
        `warnings: Vec::new()`; its `pane_listing` helper labels entries `"zsh"`
        (`src/open.rs:592`), not `DASHBOARD_LABEL`, so the post-open listing identifies no
        dashboard pane and warns per Requirement 1's table row 3. Repair it by registering a
        **second** `pane list` answer carrying a labelled pane, so `warnings` stays empty and
        the focus is exercised. Do **not** weaken the `warnings: Vec::new()` assertion — it is
        the only thing that test says about silence.
- [ ] 3.5 CHECK: Confirm no spawn API entered `src/open.rs` and the `HerdrCli` handle is still
      confined to five files — `make gates`, whose `NOSPAWN` leg is the one that fires and
      whose `LAUNCHSEAM` leg re-runs with `LAUNCH=src/open.rs`.
      **Negative control, run at planning time:** appending
      `fn _planted() { let _ = std::process::Command::new("x"); }` to `src/open.rs` turned the
      `NOSPAWN` line from `NOSPAWN OK: 25 files checked under src (>= 25), only src/cli.rs may
      spawn` into `NOSPAWN FAIL: spawn API outside src/cli.rs:`; restoring the file returned it
      to `OK`. `LAUNCHSEAM` stayed `OK` for that plant **only because it landed at EOF**, below
      the test module: its `prod()` slice truncates at the first `#[cfg(test)]`
      (`src/open.rs:423`). Re-planted above that line it gives `LAUNCHSEAM FAIL (leg 1):
      src/open.rs spawns a process`. Both legs guard the production slice this change writes
      into. `tests/gate_controls.rs` holds the standing version of this control.
- [ ] 3.6 REFACTOR: Clean up while green — or state explicitly that none was needed.
- [ ] 3.7 Run `cargo test --all-features --lib open::tests` — green, no regressions.

## 4. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [ ] 4.1 GREEN: Correct `main_routes_each_subcommand_to_its_own_placement`'s
      `assert_eq!(lines.len(), 4)` to 6 — with the plain stub each subcommand now lists, opens,
      and lists again, finding nothing. Its two `plugin pane open` assertions are unaffected. Correct the stale comment above it
      (`tests/cli.rs:361`, "so four lines total") in the same edit.
- [ ] 4.2 VERIFY: `cargo test --test cli` — green, including
      `main_focuses_the_pane_it_just_opened` from group 0.
      (HEAD: `cargo test --test cli main_routes_each_subcommand_to_its_own_placement` exits 0,
      `1 passed`, so the 4 → 6 edit is a real change and not a pre-broken test.)
- [ ] 4.3 REFACTOR: Factor any duplication between `stub_herdr` and `stub_herdr_sequenced`
      into a shared writer, or state that the two are small enough that none was warranted.

## 5. Degraded-states contract
<!-- kind: operational -->

- [ ] 5.1 CHECK: Run `cargo test --test degraded_coverage` at HEAD and record it green, so a
      later failure names this change's own rows rather than inherited drift.
- [ ] 5.2 CHANGE: Add two rows to `SPEC.md`'s degraded-states table, beside the four `open`
      rows it already has (design.md → Decision 5): the post-open `pane list` failing, being
      unparseable, or naming no dashboard pane; and the post-open `plugin pane focus` failing
      on any code. Both state that the process still exits 0 with the reason on stderr.
- [ ] 5.3 CHANGE: Bind each new row in `tests/degraded-coverage.toml` with **all six** keys
      `openspec/specs/degraded-coverage/spec.md` requires — `condition` (the table's first
      column verbatim), `tier = "unit"`, `proof` naming the group-3 tests,
      `verdict = "implemented"` (new behaviour this change adds, not an audit confirmation of
      existing behaviour), `why`, and a `covers` range in `src/open.rs`. A row missing
      `verdict` fails the parse at `tests/degraded_coverage.rs:126`, so task 5.4 would reject it.
      `unit` matches the three existing `open` rows; the capability spec's prose prefers
      `integration` for one-shot commands, so the `why` states that these proofs render nothing.
- [ ] 5.3a CHANGE: Re-anchor the `covers` ranges of the four existing `open` rows while the
      file is open. They are already wrong at HEAD — `covers = ["src/open.rs:268-280"]` lands
      on `placement_for` and the `Report` doc comment, not the listing block, and `run` begins
      at `src/open.rs:332` — and this change inserts lines that shift them further.
      `validate_covers` (`tests/degraded_coverage.rs:352-392`) only checks that a range names
      an existing path, is non-reversed, is in range, and holds a line of code, so it cannot
      catch this drift and will not catch it after the change either.
- [ ] 5.4 VERIFY: `cargo test --test degraded_coverage` — green, each new row bound to a named
      passing test.

## 6. Change Review
<!-- kind: operational -->

- [ ] 6.1 CHECK: Dispatch `outside-in-tdd-reviewer` against proposal, specs, design, and tasks
      with the diff only. Concentration points for this change: that the post-open focus is
      asserted on a *recorded argv* rather than on the fake's own return value; that no test
      asserts the `plugin pane open` response shape; and that the four post-open failure paths
      each have a test that would go red if the warning were dropped.
- [ ] 6.2 CHANGE: Fix every CRITICAL, resolve or accept each WARNING with a one-line reason,
      note SUGGESTIONs, and re-run affected tests.
- [ ] 6.3 VERIFY: Confirm no blocking or unowned finding remains.

## 7. Documentation
<!-- kind: operational -->

- [ ] 7.1 Rewrite in `SPEC.md`: § Herdr integration → **Open or focus** and **Focus can fail
      two ways** (audience: anyone changing `open.rs`). Add that a successful open is followed
      by a second `pane list` and a `plugin pane focus`, and scope the existing exit-code split
      to the *pre-open* focus. Replaces the current text's implication that `--focus` is
      sufficient, which the measurement disproves.
- [ ] 7.2 Rewrite in `AGENTS.md`: the paragraph beginning "Two more binary subcommands, `open`
      and `open-tab`" (audience: every session). Add, in one clause, that the open path
      re-lists and focuses rather than reading the open response — the durable reason being
      that a future change will otherwise reach for `result.plugin_pane.pane.pane_id` and
      quietly admit a second Herdr envelope shape into the crate. Net addition: ~3 lines.

## 8. Lint & Verify
<!-- kind: operational -->

- [ ] 8.1 CHECK: Affected tiers are the `src/open.rs` unit tests, `tests/cli.rs`, and
      `tests/degraded_coverage.rs`; no view, schema, or coverage-sensitive module is touched
      beyond `src/open.rs`.
- [ ] 8.2 VERIFY: `make check` — green. If it fails, name the failing sub-command
      (`make fmt-check`, `make lint`, `make gates`, `make test`, `make coverage`) rather than
      re-running the composite. Commit the change's own artifacts before running it: measured
      at planning time, `make gates` exits 1 with `OPENSPEC-UNTOUCHED FAIL: an untracked file
      exists inside openspec/` while this change's five files are untracked, which is that
      gate working, not a defect to chase.
- [ ] 8.3 VERIFY: `openspec validate tab-open-focus --strict` — valid. The `openspec` binary is
      nvm-installed; prepend the active node's bin directory to `PATH` first.
