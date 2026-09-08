<!-- No outer-loop acceptance group. The one genuinely end-to-end fact this change turns on
     — what working directory Herdr gives a pane process — cannot be driven from
     `cargo test` at all, so it is group 1's manual measurement rather than an automated
     acceptance test (design.md -> Test Strategy). -->

## 1. S7/S10 measurement — where does the pane process stand, and does the child even exec?
<!-- kind: operational -->

Nothing else in this change may start until this group is done and recorded. The fix in
group 3 is specified for either outcome (design.md -> Decision 1); what is not yet known is
which one is true. S10's own facts are already measured (design.md -> Decision 1b); what this
group adds is confirming they hold **inside a pane process**, and telling the two defects
apart by exit code.

- [x] 1.1 CHECK: Confirm the claim is currently unmeasured — `grep -rn 'current_dir' src/open.rs src/ui/mod.rs | grep -v '^.*tests'` and confirm no production line reads the pane process's own directory for this purpose. At HEAD, `grep -c '\.current_dir(' src/cli.rs` → `0` (exit 1), so no spawn sets one either.
- [x] 1.2 CHANGE: Add two temporary first statements to `ui::run` — `eprintln!("PANE CWD: {:?}", std::env::current_dir());` and an `eprintln!` of `startup_cwd`'s result — then `make build` and `herdr plugin link .`.
- [x] 1.3 CHANGE: Add a third temporary statement printing `resolve::openspec_bin`'s resolved path, and a fourth that spawns that path with `["list","--json"]` through the seam and prints its exit code, stdout, and stderr. This is the half that separates S10 from S7.
- [x] 1.4 CHECK: Open a Herdr workspace rooted at a **different** OpenSpec repository and open the dashboard from the plugin's action menu (`open` / `open-tab`, not a hand-run binary). Read all four printed values from the pane's stderr.
- [x] 1.5 CHECK: Record what the pane rendered — whether the change list was entirely file-sourced, and the exact text of any root-disagreement problem row.
- [x] 1.6 CHECK: Classify the result. Exit `127` with `env: node: No such file or directory` is S10 and not S7 — the child never ran, so no working directory would have helped. A payload plus a root-disagreement row is S7. Both can be true at once; record which.
- [x] 1.7 CHANGE: Write every observation verbatim into design.md -> **S7/S10 measurement (recorded)**, then remove the temporary lines and rebuild.
- [x] 1.8 VERIFY: `git diff --stat` shows no `src/` change remaining from this group, and `make check` is green.

## 2. The subprocess seam: a working directory, an environment overlay, and a deadline
<!-- kind: behavior -->

- [x] 2.1 RED: Write failing tests for: `A constructed working directory is the child's working directory`, `A working directory that does not exist fails rather than falling back`, `A child that never exits times out with a named reason`, `A fast child is unaffected by the deadline`, `A child that writes a large payload and exits is read in full`, `The deadline is a named constant and is asserted`, `A given overlay sets exactly those variables and disturbs no others`, `No overlay leaves the child's environment byte-identical`, `An interpreter-shim program fails without an overlay and succeeds with one`, and rework `No argument is added and the working directory is inherited` to name the no-directory constructor. All nine new ones are RED at HEAD: `grep -c '\.current_dir(' src/cli.rs` → `0` (exit 1), `grep -c '\.env(' src/cli.rs` → `0` (exit 1), and `grep -c 'RUN_DEADLINE' src/cli.rs` → `0` (exit 1), so none of the three behaviors exists.
- [x] 2.2 GREEN: Give `RealOpenspecCli` an optional working directory and pass it to the spawn; leave `RealHerdrCli` without one (design.md -> Decision 2).
- [x] 2.2b GREEN: Give `RealOpenspecCli` an environment overlay and apply it with `Command::env` per entry — never `env_clear`, never `env_remove`. The seam derives nothing: it sets what it was handed (design.md -> Decision 2).
- [x] 2.3 GREEN: Replace `Command::output()` with spawn plus a bounded wait, add `cli::RUN_DEADLINE` and `CliError::TimedOut { args, after }`, and kill the child on expiry. Read stdout and stderr so a full pipe cannot deadlock against the deadline.
- [x] 2.4 CHECK: Contract gate — every `match` on `CliError` in the crate compiles with the new variant, and each renders a reason naming the command. `grep -rn 'CliError::' src/ | wc -l` → `42` sites at HEAD to review.
- [x] 2.5 CHECK: The seam still parses nothing and decides nothing — `grep -c 'serde_json' src/cli.rs` → `0` (exit 1) and `grep -cE 'env_clear|env_remove' src/cli.rs` → `0` (exit 1). Confirm no **production** line constructs a `PATH` value: `grep -n 'PATH' src/cli.rs` returns `8` hits at HEAD, six of them doc comments and two test literals (`:1056`, `:1087`), and the overlay must add no ninth outside a test — the seam applies what it is handed and joins nothing. `make gates` passes `NOSPAWN-GREP` unchanged.
- [x] 2.6 REFACTOR: Fold the two spawn paths into one helper if the bounded wait duplicated the outcome mapping; otherwise state that none was needed.
- [x] 2.7 CHECK: `NOSLEEP` passes with the seam's bounded wait — `src/cli.rs` is under neither leg 2 nor leg 2b, so leg 1's deadline-bounded-poll shape governs it. Confirm the wait polls to a deadline rather than sleeping a fixed interval.
- [x] 2.8 Run `cargo test --all-features cli` — no regressions.

## 3. The CLI runs at all, and answers about the repository on screen
<!-- kind: behavior -->

- [x] 3.1 RED: Write failing tests for: `The CLI is constructed with the resolved repository root`, `A CLI answering about the resolved repository is merged, not discarded`, `A CLI answering about another repository is still discarded, with its reason`, `The overlay prepends the resolved binary's own directory to PATH`, `An absent inherited PATH yields the directory alone`, `A binary already on PATH gets the same overlay, harmlessly`, `A shim whose interpreter is unreachable exits 127 and renders a problem row`. RED at HEAD — `worker_cli` takes one argument today (`grep -n 'pub fn worker_cli' -A 3 src/cli.rs`).
- [x] 3.2 GREEN: Add the working-directory and overlay parameters to `cli::worker_cli`, pass both to `RealOpenspecCli`, and have `ui::start_collaborators` supply the root it already resolved. `worker_cli_from_env` passes `None` and an empty overlay.
- [x] 3.2b GREEN: Build the one-entry overlay in `ui::start_collaborators` — `PATH` set to the resolved binary's `parent()` joined by the platform separator to the inherited `PATH`, read through the injected environment lookup; the parent alone when no `PATH` is inherited. Applied for every probe step (design.md -> Decision 2).
- [x] 3.3 CHECK: Confirm the root-disagreement guard in `changes::from_cli_cached` is unchanged — the fix stops the disagreement arising, it does not start trusting a CLI that reports another repository.
- [x] 3.3b CHECK: `ui::start_collaborators` names no `std::env::var` — `grep -c 'env::var' src/ui/mod.rs` → `0` (exit 1) at HEAD, and must stay `0`, since the inherited `PATH` comes through the injected lookup and `cargo test` runs in parallel threads of one process.
- [x] 3.4 Run `cargo test --all-features` for `cli`, `changes`, and `ui::mod` — no regressions.

## 4. The watch covers `openspec/`, not the repository
<!-- kind: behavior -->

- [x] 4.1 RED: Write failing tests for: `The composition root watches openspec/, not the repository root` and `A write outside openspec/ produces no batch`. RED at HEAD: `grep -n 'watch::start(root)' src/ui/mod.rs` → line 169 passes the root itself.
- [x] 4.2 GREEN: Pass `root.join("openspec")` from `start_collaborators`. `watch::start` is unchanged (design.md -> Decision 8).
- [x] 4.3 CHECK: The three landed `watch::start` call sites — `grep -rn 'watch::start(' src/ | wc -l` → `3` — still pass a path that exists in their own fixture, so no landed watcher test starts watching a directory it did not create.
- [x] 4.4 Run `cargo test --all-features watch ui::mod` — no regressions.

## 5. The poller reports a stall and compares canonical paths
<!-- kind: behavior -->
<!-- parallel-after: 2 -->

- [x] 5.1 RED: Write failing tests for: `A poll outstanding past the stall threshold is announced once`, `A poll answering normally never reports a stall`, `A stalled socket that recovers restores the badges`, `An herdr that answers with an error is not a stall`, `The stall threshold is a named constant and is asserted`, `A symlinked repository path still badges its agents`, `An unresolvable working directory is kept verbatim, not dropped`. RED at HEAD: `grep -c 'STALL_AFTER\|stalled' src/agents.rs` → `0` (exit 1).
- [x] 5.2 GREEN: Add `agents::STALL_AFTER` (`5 * POLL_INTERVAL`) and `AgentSnapshot::stalled`, and report a stalled poll once per episode from `RealAgentPoll::drain`'s existing single clock read.
- [x] 5.3 GREEN: Canonicalize each agent's `cwd` through an injected `&dyn Fn(&Path) -> Option<PathBuf>` before the snapshot leaves the poller, keeping an unresolvable path verbatim. `attribute` is untouched (design.md -> Decision 5).
- [x] 5.4 CHECK: Contract gate — `AgentSnapshot` gains a field, so every literal and pattern must name it. `grep -rn 'AgentSnapshot {' src/ tests/ | wc -l` → `78` sites at HEAD; the compile is the check, since the type implements no `Default` and no site may use `..`. Two landed loop scenarios carry `AgentSnapshot` literals under `src/ui/` and are updated by the same compile: `An agent snapshot reaches the frame that consumed it and survives a refresh` and `An unreachable socket never becomes a problem row`.
- [x] 5.5 CHECK: `make gates` — `NODEFAULT-UI`'s `src/agents.rs` leg still passes with its own `SCAN_MIN`, and the count has grown rather than shrunk.
- [x] 5.6 Run `cargo test --all-features agents` — no regressions.

## 6. The panic hook restores only on the render thread
<!-- kind: behavior -->
<!-- parallel-after: 1 -->

- [x] 6.1 RED: Write failing tests for: `A panic on a worker thread restores nothing` and `A panic on the render thread still restores`. RED at HEAD: `grep -c 'thread::current' src/ui/terminal.rs` → `0` (exit 1), so the hook cannot tell the threads apart.
- [x] 6.2 GREEN: Capture the installing thread's `ThreadId` in `install_panic_hook` and extract the compare-then-restore-or-delegate decision into a pure function the tests drive, on `restore_then`'s existing terms.
- [x] 6.3 CHECK: `make gates` — the crossterm terminal-mode confinement check still names `src/ui/terminal.rs` alone, and its negative control still fires when a mode call is planted elsewhere.
- [x] 6.4 Run `cargo test --all-features ui::terminal` — no regressions.

## 7. The launcher reports its own death and can be settled
<!-- kind: behavior -->
<!-- parallel-after: 2 -->

- [x] 7.1 RED: Write failing tests for: `A second press while a launch is in flight is refused, not queued`, `Focus still works while a launch is in flight`, `Every combination is total`, `A dead launcher worker is reported once and then stops being reported`, `The settle budget is a named constant and is asserted`, and the extended `The inert launcher answers nothing and starts nothing`. RED at HEAD: `grep -c 'in_flight' src/launch.rs` → `0` (exit 1) and `grep -c 'SETTLE_BUDGET\|pub fn settle' src/launch.rs` → `0` (exit 1).
- [x] 7.2 GREEN: Add `decide`'s sixth argument and the in-flight refusal, ordered before the live-name refusal.
- [x] 7.3 GREEN: Latch a `dead` flag on the first `SendError` or `Disconnected` and report it once as `Outcome { named: None, problems: [reason] }`, copying `agents::RealAgentPoll`'s shape.
- [x] 7.4 GREEN: Add `launch::SETTLE_BUDGET` and `launch::settle`, declared **below** the module's single `thread::spawn` so `NOBLOCK` leg 3's cut already excludes it (design.md -> Decision 6).
- [x] 7.5 CHECK: Contract gate — `decide` gains a parameter, so its four carried-over scenarios (`An unreachable socket makes every action key inert`, `No selected change means no launch, and no agent means no focus`, `Each launch intent carries its own change and derived name`, `A derived name already live in the session is refused before any Herdr call`) each gain the sixth argument. `grep -rn 'decide(' src/ | wc -l` → `13` sites at HEAD; confirm every caller passes a real value rather than a literal `false`.
- [x] 7.6 CHECK: `make gates` — `NOBLOCK` leg 3's `src/launch.rs` leg passes unchanged, and its negative control (a `recv_timeout` planted above the spawn) still fires. Record both halves.
- [x] 7.7 CHECK: `NOSLEEP` passes with `settle`'s deadline-bounded poll. `src/launch.rs` is named by neither leg 2 (`src/ui` only) nor leg 2b (`watch`/`refresh`/`agents` only), so leg 1's shape rule governs it; confirm the sleep sits inside `while Instant::now() < deadline`. Plant a bare `thread::sleep` outside the loop, show leg 1 fires, remove it, show it goes quiet.
- [x] 7.8 Run `cargo test --all-features launch` — no regressions.

## 8. The dashboard and the loop: startup rows, in-flight, and the two new problem sources
<!-- kind: behavior -->

- [x] 8.1 RED: Write failing tests for: `A watcher error does not erase the startup problems`, `A forced refresh does not repopulate or duplicate the startup problems`, `A batch that invalidates nothing issues no refresh`, `A stalled poller becomes a problem row and withdraws the badges`, `A stopped refresh worker becomes a problem row and keeps the list`, `All five problem sources render in their specified order`, `A non-stalled agent problem draws no row`, `The flag is set on hand-over and cleared on outcome`, `A focus request never sets the flag`, `A second launch key while in flight renders a problem row and issues nothing`, `A dead launcher clears the flag`, `A launch in flight at quit is settled rather than orphaned`, `Quitting with nothing in flight pays no budget`, `A held key causes no read`, `A tab switch and an adopted refresh each cause exactly one read`. RED at HEAD: `grep -rn 'refresh\.startup' src/ | wc -l` → `0` and `grep -c 'in_flight' src/ui/app.rs` → `0` (exit 1), so neither field exists. (`grep -c 'startup' src/ui/app.rs` alone returns `3` — all three are prose in doc comments, which is why the field-qualified form is the check.)
- [x] 8.2 RED: Add the refresh worker's own failing tests here too, since they are driven through the same loop doubles: `A dead refresh worker is reported once and then stops being reported`, `A refresh outstanding does not queue further selections`, `A forced refresh outstanding behind a narrower one is not lost`. Extend `The inert refresher answers nothing and starts no thread` with its no-`Stopped` clause in the same pass. RED at HEAD: `grep -c 'Stopped' src/refresh.rs` → `0` (exit 1).
- [x] 8.3 GREEN: Add `RefreshResult::Stopped`, the dead-worker latch, and the at-most-one-cycle-outstanding rule to `src/refresh.rs`, copying `agents::RealAgentPoll`'s shape.
- [x] 8.4 GREEN: Add `Refresh::startup` and `Launch::in_flight` to `src/ui/app.rs`, and have `run_wired` seed `startup` from `Collaborators::problems` instead of `problems`.
- [x] 8.5 GREEN: Update the loop's steps 1, 3, 4, and 6 in `src/ui/driver.rs` — set and clear `in_flight`, short-circuit an empty selection, handle `Stopped` without adopting.
- [x] 8.6 GREEN: Emit the two new leading problem sources from `ui::list::rows` in the specified order, and call `launch::settle` from `run_wired` when `in_flight` is set.
- [x] 8.7 CHECK: Contract gate — `Refresh` and `Launch` each gain a field. `grep -rn 'Refresh {' src/ | wc -l` → `54` and `grep -rn 'Launch {' src/ | wc -l` → `82` at HEAD (the gate's own span floors, on the `Makefile` recipe lines, are `SCAN_MIN=53` for `Refresh` and `SCAN_MIN=81` for `Launch`; both are minima, so adding sites keeps them green); the compile-time destructuring companions must be updated to name four and the new field respectively.
- [x] 8.8 CHECK: `make gates` — `NODEFAULT-UI`'s five legs each still pass with their own `SCAN_MIN`, and `NOBLOCK` legs 1 and 2 still find no clock or blocking wait under `src/ui/` (`launch::SETTLE_BUDGET` is a `Duration` constant, not a clock read). Plant a `Instant::now()` in `src/ui/mod.rs`, show leg 2 fires, remove it, show it goes quiet.
- [x] 8.9 REFACTOR: Collapse any duplication between the five problem-row sources in `ui::list::rows` while the view tests stay green; otherwise state that none was needed.
- [x] 8.10 Run `cargo test --all-features` in full — no regressions.

## 9. SPEC.md and the degraded-states coverage map
<!-- kind: operational -->

- [x] 9.1 CHECK: Determine whether the `degraded-coverage` sibling change has landed — `git log --oneline -20 -- tests/degraded-coverage.toml SPEC.md`. The answer decides between 9.2 and 9.3 (design.md -> Coordination).
- [x] 9.2 CHANGE (only if it has landed): Correct the three now-wrong rows of `SPEC.md`'s degraded-states table — the root-disagreement row, the recursive-watch sentence under Refresh, and the watcher-error row's problem ordering — and add two rows for a stopped worker and a stalled poller, with a proving test named for each in `tests/degraded-coverage.toml`.
- [x] 9.3 CHANGE (otherwise): *(not taken — 9.1 resolved to the has-landed branch, so 9.2 applies.)* Write the same five edits verbatim into planning-review.md as a hand-off, naming for each the row's current text, its replacement, and the test that proves it. Touch neither file.
- [x] 9.4 VERIFY: `make check` green either way, and `cargo test --test degraded_coverage` green.

## 10. Change Review
<!-- kind: operational -->

- [x] 10.1 CHECK: Dispatch an independent reviewer — not a fork of this session — against proposal.md, all eight spec deltas, design.md, and the diff. Ask specifically about: whether every new test would go red if its behavior were deleted; whether the S7 measurement's recorded outcome matches what group 3 actually implemented; whether any task invented a boundary design.md -> Test Boundaries does not name; and whether the seam's relaxed `current_dir` prohibition leaked beyond `RealOpenspecCli`.
- [x] 10.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a one-line reason, note SUGGESTIONs, and re-run affected tests.
- [x] 10.3 VERIFY: Confirm no blocking or unowned finding remains, and that no spec delta outside this change's eight capabilities was touched.

## 11. Documentation
<!-- kind: operational -->

- [ ] 11.1 Rewrite in `AGENTS.md`: the "Nothing spawns a process outside `cli`" bullet (audience: every future session). The sentence "never sets `current_dir`" is now false for `RealOpenspecCli` and must be corrected in place, not appended to — replace it with the narrow rule (a caller-supplied working directory for `openspec` only, because the CLI resolves its root from the process cwd and has no root flag). Net effect: ~2 lines rewritten, none added.
- [ ] 11.2 Rewrite in `AGENTS.md`: the "The render path blocks on nothing but the terminal" bullet (audience: every future session). Add one sentence naming the per-frame artifact read as the one accepted exception and the cache that bounds it (design.md -> Decision 10), so a later reader does not "fix" it. Net: +2 lines.
- [ ] 11.3 Rewrite in `SPEC.md` -> Refresh: correct "one recursive watch on `openspec/`" to say what the composition root passes, and record that the `openspec` child runs with the repository root as its working directory. Replaces the current text rather than adding beside it.
- [ ] 11.4 VERIFY: `cargo test --test manifest` green — the README/manifest contract is untouched by these edits.

## 12. Lint & Verify
<!-- kind: operational -->

- [ ] 12.1 CHECK: Inspect the intended verification commands and affected tiers — every gate below plus `make gates-full`, which this change does not otherwise reach.
- [ ] 12.2 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors
- [ ] 12.3 VERIFY: `cargo fmt --all -- --check` — clean
- [ ] 12.4 VERIFY: `make gates` — every hygiene gate green, none run with a `MIN`/`SCAN_MIN` override that was not already on the recipe line
- [ ] 12.5 VERIFY: `cargo test --all-features` — green
- [ ] 12.6 VERIFY: `cargo llvm-cov --fail-under-lines 80` — at or above the floor, with no exclusion added
- [ ] 12.7 VERIFY: `openspec validate seam-resilience --strict` — no errors

<!-- Parallelism, examined: groups 5, 6, and 7 each edit exactly one module nobody else in
     this change touches (`src/agents.rs`, `src/ui/terminal.rs`, `src/launch.rs`) and need
     nothing from each other, so they are marked parallel — 5 and 7 after group 2, whose
     `RUN_DEADLINE` their own constants are asserted against, and 6 after group 1, which
     it does not depend on at all beyond the measurement gate. Everything else is
     sequential and the reason is shared files, not narrative: groups 2 and 3 both edit
     `src/cli.rs`; groups 3, 4, and 8 all edit `src/ui/mod.rs`; group 8 additionally needs
     the types groups 5 and 7 introduce. -->
