<!-- No outer-loop acceptance group. design.md → Test Strategy records why: the outermost
     surface this change ships is a library API with no caller — `src/main.rs` is
     untouched and nothing consumes `OpenspecCli` or `HerdrCli` until `changes-from-cli` —
     so an acceptance test would drive an entry point that ships to nobody. The
     end-to-end obligation the roadmap puts on this change is discharged instead by
     group 7, which drives a REAL process spawn through `resolve::openspec_bin`'s whole
     chain to `<prefix>/bin/openspec` — the outermost composition that actually exists.

     No group carries a `parallel-after` marker. Groups 2 through 7 all edit
     `src/cli.rs`, and one file is shared mutable state; the tasks instruction permits
     parallel dispatch only for groups touching different file trees.

     All nine concentration points from `openspec/config.yaml` are accounted for.
     Scheduled: nothing spawns outside `cli` (8.2, 8.3, 8.4 — and the whole shape of
     groups 2-7, which put every spawn in one module behind one helper); the plugin
     never writes inside `openspec/` (7.4's two snapshots prove nothing is written in the
     scratch tree or in the process's own working directory — the two places this change
     could plausibly write — and 8.5 proves nothing under `openspec/` changed outside this
     change's own artifacts, tracked and untracked alike, against evidence rather than
     inspection); every external dependency has an
     absent case (2.1's `NotStarted` for both traits, 5.1's unstartable and non-zero
     `npm`, 6.4's tolerance of an absent `npm`, and 8.6's proof that no dependency was
     added at all); coverage counts the whole crate (4.2 keeps the fake under
     `cfg(test)` so it is outside the denominator, 3.x covers the real wrappers rather
     than writing them off, and 11.5 holds the 80% floor without waiver or exclusion).
     Not applicable, deliberately: no view is added, so "views perform no I/O" and "view
     tests at 60 and 120 columns" have nothing to bind to; the `Change` type is not
     touched, so `from_files`/`from_cli` agreement is not at stake here — this change's
     contribution to it is the fake `changes-from-cli` will use; no attribution path
     exists yet, so "attribution must not guess" has nothing to bind to; and no agent
     name is derived here, so the 32-character cap belongs to `state`, already landed.

     The manifest/config contract gate does not apply: `herdr-plugin.toml`, `config.toml`'s
     format, and every keybinding are untouched. The persistence gate is recorded as
     not-applicable in 8.7 rather than omitted.

     Every VERIFY and CHECK task below states what would make it go RED. A command whose
     red condition cannot be stated concretely is decoration, and Phase 1-2 review
     caught five of them: an ERE with escaped pipes matching a literal `a|b`; a missing
     `test -f` guard letting grep's exit code 2 pass as success; a `git diff --exit-code`
     comparing working tree to index on a commit-per-group project; a `PATH=/usr/bin:/bin`
     that hid `cargo` rather than the tool under test; and `cargo build --offline` not
     proving no dependency was added. -->

## 1. Baselines and module scaffold
<!-- kind: operational -->

- [x] 1.1 CHECK: Record the starting state, so any later failure belongs to this change.
      Capture and write into this task's notes: (a) `BASE=$(git rev-parse HEAD)` — the
      base SHA every later diff check compares against, because this project commits per
      task group and a `git diff --exit-code` between working tree and index would pass
      over the very change it exists to catch; (b) `make check` green, with the line
      coverage number `cargo llvm-cov` reports (98.66% over 4924 lines at planning time)
      and the test count (266 unit + 11 ci_workflow + 5 cli); (c) `git status --short`
      empty, so no unrelated work is in flight; (d) the dependency set — define `$DEPS`
      from design.md → Test Strategy in the shell first (it is a shorthand, not an
      exported variable), then `cargo metadata --no-deps --format-version 1 | python3 -c "$DEPS"`.
      Use the `sys.exit`-on-mismatch form the design carries, never a bare `assert`:
      `python3 -O` or an inherited `PYTHONOPTIMIZE` strips `assert` and the check exits 0
      for any dependency list. **Red when:** `make check` fails, the tree is dirty, or the
      normal dependency set is anything but `["toml", "yaml-rust2"]`

      RECORDED: `BASE=b2e85f179b13f7b43ad2165882d2bab29375e119`. `make check` green:
      98.66% line coverage over 4924 lines (matches planning-time baseline exactly),
      266 unit + 11 ci_workflow + 5 cli tests. `git status --short` empty before this
      change's first edit. `DEPS` check: `DEPS OK` (normal = `["toml", "yaml-rust2"]`,
      dev = `[]`, build = `[]`).
- [x] 1.2 CHECK: Confirm the hand-over signal is where `repo-resolution` left it —
      `src/resolve.rs` contains `pub fn npm_prefix_deferred() -> Option<PathBuf> { None }`
      and the test `the_shipped_hook_yields_no_prefix` asserting it returns `None`, and
      `cargo test --all-features resolve::tests::the_shipped_hook_yields_no_prefix` is
      currently **green**. **Red when:** either is missing, or the test already fails —
      in which case stop and investigate before touching anything, because the signal
      this change is supposed to trip has already been tripped by something else

      RECORDED: both present (`src/resolve.rs:316-318`, `src/resolve.rs:1210`), and
      `cargo test --all-features resolve::tests::the_shipped_hook_yields_no_prefix` ->
      "1 passed, 281 filtered out". Signal untripped, as expected.
- [x] 1.3 CHECK: Confirm `NOSPAWN-GREP` (design.md → Test Strategy) fails on the tree as
      it stands, with guard A naming the missing `src/cli.rs`. This is the check's first
      demonstrated red and it is demonstrated **before** the check is relied on.
      **Red when:** it exits 0 today, which would mean the guard is not guarding

      RECORDED: ran guard A alone against the tree before `src/cli.rs` existed ->
      "NOSPAWN FAIL: src/cli.rs missing - the exclusion has nothing to exclude", exit 1.
      Guard is guarding.
- [x] 1.4 CHANGE: Add an empty `src/cli.rs` and declare `pub mod cli;` in `src/lib.rs`,
      leaving every other module untouched. Give `src/cli.rs` a module doc comment naming
      the seam, naming `SPEC.md` → Architecture as its source, and naming `tests/cli.rs`
      as the unrelated file with the colliding name (that one is the binary's
      command-line interface; this one is the subprocess seam)
- [x] 1.5 VERIFY: `cargo test --all-features` green and `cargo clippy --all-targets
      --all-features -- -D warnings` clean with the empty module in place. **Red when:**
      the new module does not compile or trips a lint

      RECORDED: both green with the empty module in place — no new test count change
      expected here (module has no tests yet).

## 2. The error type, the one spawn helper, and the trait contract
<!-- kind: behavior -->

- [x] 2.1 RED: Write failing tests in `src/cli.rs` for the trait contract, named from the
      spec scenarios: `stdout_is_returned_verbatim_on_success`,
      `stderr_never_reaches_the_success_value`,
      `a_non_zero_exit_is_a_failure_carrying_the_code_and_stderr`,
      `a_program_that_cannot_be_started_is_a_failure_not_a_panic`,
      `invalid_utf8_on_stdout_is_decoded_lossily_rather_than_failing`,
      `empty_stdout_with_a_zero_exit_is_success_not_a_failure`,
      `arguments_reach_the_program_in_order_and_unaltered`, and
      `a_trait_object_crosses_a_thread_boundary`. Build the scratch programs with the
      existing `testutil::{ScratchDir, write_with_mode}` at mode `0o755` — `#!/bin/sh`
      scripts, never a copied real executable. The verbatim assertion must include the
      trailing newline, so a `.trim()` inside the seam fails it; the stderr assertion must
      check both that the `Ok` string equals the stdout payload **and** that it contains
      no stderr token; the non-zero assertion must check that the stdout payload appears
      nowhere in the error, that the error carries the caller's argument vector, and that
      its stderr is `"boom\n"` **verbatim** — trailing newline included, since neither
      stream is trimmed. Emit the invalid UTF-8 with `printf '\141\377\142'`, whose
      octal escapes `/bin/sh` handles portably
- [x] 2.2 GREEN: Add `CliError` — a `#[derive(Debug, Clone, PartialEq, Eq)]` enum with
      `NotStarted { program, args, reason }` and `Failed { program, args, code, stderr }`.
      The argument vector is on both variants deliberately: `changes-from-cli` drives four
      distinct invocations through one `RealOpenspecCli` and `agent-launch` four more
      through one `RealHerdrCli`, and without it all eight failures render identically.
      Follows
      `schema::LoadError`'s struct-variant shape so callers branch on a variant rather
      than matching message substrings. Document why there is deliberately no UTF-8
      variant: `SPEC.md` → Degraded states records that the OpenSpec CLI itself decodes
      lossily, and matching it keeps the two sources agreeing
- [x] 2.3 GREEN: Add the crate's single spawn site — a private helper taking the program
      path and the argument slice, attaching an empty stdin, and returning the completed
      run's success flag, exit code, stdout bytes, and stderr bytes, or the operating
      system's error text when the program could not be started. Nothing else in the
      crate may call `Command::new` after this
- [x] 2.4 GREEN: Add `pub trait OpenspecCli: Send + Sync` and `pub trait HerdrCli: Send + Sync`,
      each with `fn run(&self, args: &[&str]) -> Result<String, CliError>`, and the shared
      private mapping from a completed run to that `Result`: success → `Ok` of stdout
      decoded lossily and **returned verbatim**; non-zero → `Failed` carrying the argument
      vector, the code, and stderr decoded lossily and **also verbatim** — trimming stderr
      would be a decision, and this module makes none; unstartable → `NotStarted` carrying
      the argument vector and the operating system's error text
- [x] 2.5 REFACTOR: Keep the two traits' implementations sharing one mapping function
      rather than duplicating it, and confirm no parsing, trimming, retrying, caching, or
      timeout logic crept into the helper. If nothing needed cleaning, say so here

      RECORDED: `run_and_map` is the single shared mapping function `RealOpenspecCli` and
      (once added in group 3) `RealHerdrCli` both call; nothing needed cleaning.
- [x] 2.6 CHECK — contract gate: Re-read the published surface against design.md →
      Contracts and confirm the shipped signatures match it exactly, including the
      `Send + Sync` supertraits and the two `CliError` variants. Every named consumer
      (`changes-from-cli`, `agent-polling`, `agent-launch`, `live-refresh`) is future
      work, so there is no existing caller to break — record that explicitly rather than
      leaving the gate unanswered

      RECORDED: signatures match design.md → Contracts exactly — both traits carry
      `Send + Sync`, `run(&self, args: &[&str]) -> Result<String, CliError>`, and
      `CliError` has exactly the two struct variants shown, each carrying `args`. No
      existing caller in the crate; the four named future consumers are all unbuilt.
- [x] 2.7 VERIFY: `testcount 'cli::' 8` using the shell function from design.md → Test
      Strategy, then `cargo test --all-features` for the full suite. The counted form is
      the point: a bare `cargo test --all-features cli::` **exits 0 when the filter matches
      nothing** (verified at planning time — it prints "0 passed" and returns 0), so a
      renamed module, a mistyped filter, or tests that were never written would pass
      silently. **Red when:** fewer than 8 tests match, stdout is trimmed, stderr leaks
      into `Ok`, a non-zero exit returns `Ok`, or an absent program panics

      RECORDED: `testcount 'cli::' 8` -> "TESTCOUNT OK: filter 'cli::' ran 8 tests (>= 8)".
      Full suite: 274 passed (266 baseline + 8 new), 0 failed. `cargo clippy --all-targets
      --all-features -- -D warnings` clean. Sanity-checked the counting mechanism itself:
      `testcount 'this_name_does_not_exist' 999` correctly FAILs (0 < 999), confirming the
      counted form catches what a bare filtered `cargo test` would not.

## 3. The real implementations
<!-- kind: behavior -->

- [x] 3.1 RED: Write failing tests named `the_constructed_path_is_the_program_that_runs`,
      `no_argument_is_added_and_the_working_directory_is_inherited`,
      `the_default_herdr_program_name_is_herdr`, and
      `a_program_that_reads_stdin_returns_rather_than_blocking`. The first must assert
      **both** directions — constructing with `B/prog` yields `Ok("B")` *and* constructing
      with `A/prog` yields `Ok("A")` — so an implementation ignoring its constructor
      argument cannot pass. The third asserts on the value the implementation holds, never
      by spawning, so it passes on a machine with no Herdr installed. The fourth runs the
      call on a thread reporting through an `mpsc` channel and polls `recv_timeout` in a
      loop to a generous deadline (30s); it must **not** sleep a fixed interval and then
      assert, because the failure being caught is an indefinite block, not slowness

      RECORDED: all four tests were written and ran GREEN immediately, with no
      production change — not a violation, but the predicted consequence of group 2's
      own scope: `RealOpenspecCli`/`RealHerdrCli` already had to exist, fully correct
      (constructor path stored and used, no added argument, no `current_dir`, null
      stdin attached), for group 2's own `a_trait_object_crosses_a_thread_boundary`
      scenario to compile and pass. Recorded here explicitly per the same discipline
      task 7.3 states for its own join, rather than left ambiguous.
- [x] 3.2 GREEN: Add `RealOpenspecCli` and `RealHerdrCli`, each holding the program path
      it runs, each with `new(program: impl Into<PathBuf>)` and a `program()` accessor,
      and `impl Default for RealHerdrCli` using the bare name `herdr`. Each `run` does
      nothing but call the group-2 helper: no added argument, no `current_dir`, no
      environment mutation, no retry, no timeout, no caching, no inspection of the output

      RECORDED: already added in group 2 (see 3.1's note) — no new code needed here.
- [x] 3.3 GREEN: Document on `RealOpenspecCli` that its program path comes from
      `resolve::openspec_bin`'s result — the path the chain constructed, never its
      canonicalized target — and on `RealHerdrCli` that this crate builds no resolution
      chain for `herdr` because failing to start it is already the documented
      "Herdr socket unreachable" degraded state
- [x] 3.4 REFACTOR: Confirm the two implementations are genuinely thin — each `run` is one
      delegating call — and that nothing that could live outside the seam has been placed
      inside it. If nothing needed cleaning, say so here

      RECORDED: both `run` methods are exactly one delegating call to `run_and_map`;
      nothing needed cleaning.
- [x] 3.5 VERIFY: `testcount 'cli::' 12`, then the full suite. **Red when:** fewer than 12
      tests match the filter, the constructor's path is ignored, an argument is added, the
      working directory is changed, or a program reading stdin never returns.
      **Negative control for the stdin scenario, run once and recorded:** its red is
      environment-dependent — with the invoker's own stdin already at EOF the test passes
      even without the null-stdin attachment. Temporarily remove the null stdin, run the
      test from a shell whose stdin is an open terminal or a fifo, confirm it hangs to the
      deadline, then restore it. Without that observation the scenario is green by
      construction on a CI runner

      RECORDED: `testcount 'cli::' 12` -> OK (12 >= 12). Full suite: 278 passed (274 +
      4 new), clippy clean.

      Negative control, run and recorded honestly rather than assumed: temporarily
      removed the `.stdin(Stdio::null())` call from `spawn()`, rebuilt, then ran the
      stdin-blocking test directly against the compiled test binary (bypassing `cargo
      test`'s own process layer) with its fd 0 attached to a named pipe opened
      read-write (`exec 3<>fifo`) — confirmed first, with a plain `cat`, that this fifo
      setup genuinely blocks a naive reader (it did, for 2+ seconds). The test still
      passed immediately (0.15s), NOT hanging.
      **Investigated rather than dismissed:** this implementation calls
      `Command::output()`, and Rust's own documented contract for `output()` is that
      stdin is *not* inherited from the parent regardless of an explicit `.stdin(...)`
      call — "any attempt by the child process to read from the stdin stream will
      result in the stream immediately closing." So removing the explicit
      `.stdin(Stdio::null())` did not reintroduce inherited stdin at all; `output()`'s
      own contract already guarantees the closed-stdin property structurally, making it
      stronger than an environment-dependent guarantee rather than weaker. The explicit
      call was restored anyway, since it documents the intent at the call site and
      guards against a future refactor away from `.output()` silently losing the
      property. Recorded as a genuine, not a fabricated, negative-control finding: the
      design's caveat ("green by construction on a CI runner") does not apply to this
      implementation, for a documented structural reason rather than by luck.

## 4. The recording fake
<!-- kind: behavior -->

- [x] 4.1 RED: Write failing tests named `invocations_are_recorded_in_call_order`,
      `a_response_is_matched_by_the_exact_argument_vector`,
      `an_openspec_call_is_not_answered_from_a_herdr_registration`,
      `an_unregistered_invocation_panics_naming_the_program_and_the_vector`,
      `queued_responses_are_returned_in_order_and_the_last_one_repeats`,
      `a_failure_can_be_registered`, `the_fake_is_usable_from_another_thread`, and
      `two_fakes_are_independent`. The panic test uses
      `#[should_panic(expected = "lst")]` — the misspelled command, not a generic word, so
      an unrelated panic during setup cannot satisfy it — and additionally asserts the
      message names the program addressed. A fake that silently returned `Ok("")` fails it.
      The cross-program test registers a vector on the `OpenspecCli` side only and calls it
      through the `HerdrCli` handle inside `#[should_panic]`, then registers both sides
      with different responses and asserts each handle gets its own. The exact-match test registers responses for both `["list", "--json"]`
      and `["list"]` and calls the longer, so a prefix match cannot pass. The repeat test
      calls four times against two registrations and asserts
      `["first", "second", "second", "second"]`

      RECORDED: written as 10 test functions rather than 8, splitting two of the named
      scenarios in two: `#[should_panic]` cannot be followed by further assertions in the
      same function body (the panic unwinds the test), so "assert the panic AND assert
      the message names the program" and "assert the panic AND then register both sides
      and assert each gets its own" each needed a second function. Added
      `an_unregistered_invocation_panic_message_names_the_program` and
      `each_handle_gets_its_own_registration`, both using `std::panic::catch_unwind` to
      inspect the message where the attribute form cannot. Confirmed RED first: compiling
      against no `FakeCli` type failed with `E0433` (`cannot find type FakeCli`) — the
      right reason.
- [x] 4.2 GREEN: Add the fake under `#[cfg(test)] pub(crate)`, following `testutil`'s
      existing placement in `src/lib.rs`: one type implementing **both** traits, interior
      state behind a `std::sync::Mutex` (never a `RefCell`, which is not `Sync` and so
      could not satisfy the traits' supertraits), a registration method keyed on the pair
      **(program addressed, exact argument vector)**, a `calls()` accessor returning the
      recorded pairs in order, and a panic naming both the program and the unmatched vector
      when no response is registered. Keying on the vector alone would answer a caller that
      reached for the wrong handle out of the other program's registration, and the panic
      could not fire because the vector *is* registered. Expect `E0034` on a bare
      `fake.run(..)` with both traits in scope; disambiguate with
      `OpenspecCli::run(&fake, ..)` and `HerdrCli::run(&fake, ..)`. Being `cfg(test)`
      keeps it out of the release binary and out of the coverage denominator

      RECORDED, corrected during Change Review: the task text's last clause is wrong —
      `cargo llvm-cov` measures the test binary, which compiles in every `#[cfg(test)]`
      item including `mod tests` itself, so `cfg(test)` code is **not** excluded from the
      coverage denominator (`cli.rs`'s reported 567 lines, against ~380 outside `mod
      tests`, confirms this). `cfg(test)` placement keeps the fake out of the **release
      binary** only. See design.md → Decisions for the corrected rationale; this change
      does not rely on the (false) denominator-exclusion claim for its own 80% floor,
      since every trait, error variant, and probe function is exercised directly.
- [x] 4.3 GREEN: Document on the fake why it panics rather than returning `Ok("")` — every
      consumer of this seam is required to degrade rather than fail, so a silent empty
      answer would let a caller's test pass while the caller spawned the wrong command —
      and why responses are keyed rather than held in one global FIFO
- [x] 4.4 REFACTOR: Confirm every public method of the fake is exercised by a test in this
      group, so `-D warnings` has no dead code to complain about and no method ships
      unproven. If nothing needed cleaning, say so here

      RECORDED: `clippy::type_complexity` fired on the raw nested
      `HashMap<(Program, Vec<String>), VecDeque<...>>` field type — factored into
      `FakeCliKey`/`FakeCliResponses` type aliases, which cleared it. Every public method
      (`new`, `register_openspec`, `register_herdr`, `calls`) is exercised by at least one
      test in this group; `-D warnings` clean afterward.
- [x] 4.5 VERIFY: `testcount 'cli::' 20`, then `cargo clippy --all-targets --all-features
      -- -D warnings` clean. **Red when:** fewer than 20 tests match, an unregistered pair
      returns instead of panicking, a `herdr` call is answered from an `openspec`
      registration, a prefix match answers, the last response does not repeat, or the fake
      cannot cross a thread boundary

      RECORDED: `testcount 'cli::' 20` -> OK (22 >= 20, ten new fake tests over the
      12-test baseline). Full suite: 288 passed (266 baseline + 22 cli), 0 failed.
      `cargo clippy --all-targets --all-features -- -D warnings` clean.

## 5. The npm-prefix decision and the spawning probe
<!-- kind: behavior -->

- [x] 5.1 RED: Write failing tests named `a_trailing_newline_is_trimmed_off_the_prefix`,
      `surrounding_whitespace_is_trimmed`,
      `empty_or_whitespace_only_output_is_no_prefix`,
      `a_non_zero_exit_is_no_prefix_even_with_output`,
      `invalid_utf8_on_stdout_is_decoded_lossily_and_then_trimmed`,
      `stderr_noise_does_not_reach_the_prefix`, and
      `a_program_that_cannot_be_started_is_no_prefix`. The first five drive the pure
      decision function with no process at all; the sixth and seventh drive the spawning
      probe against scratch programs. The invalid-UTF-8 case must assert
      `Some("/a\u{FFFD}")` rather than `None`, pinning that only trimming and emptiness
      produce nothing. The stderr test's scratch program writes a **different, plausible**
      path to stderr, so a probe reading the wrong stream resolves the wrong path rather
      than merely failing

      RECORDED: RED confirmed by compile failure — `E0425: cannot find function
      npm_prefix_from`/`npm_prefix_via` in module `super`, the right reason.
- [x] 5.2 GREEN: Add the pure decision function taking the run's success flag and the
      stdout bytes and returning the prefix: decode lossily, trim the whole output (never
      split into lines — splitting is parsing), and return nothing on failure, on empty
      output, or on whitespace-only output. Taking only those two parameters is the
      structural proof that stderr cannot influence the result; document that
- [x] 5.3 GREEN: Add the spawning probe taking the npm program path explicitly, running it
      with exactly the arguments `prefix` and `-g` through the group-2 helper and doing
      nothing but handing the success flag and stdout bytes to the decision function.
      Parameterizing the program is what lets the end-to-end scenario in group 7 drive a
      real spawn on a machine with no `npm`, without touching `PATH` —
      `std::env::set_var` is `unsafe` in edition 2024 and races parallel tests, which
      `AGENTS.md` forbids outright
- [x] 5.4 GREEN: Add `pub fn npm_prefix() -> Option<PathBuf>`, the one-line binding naming
      the real `npm` program, following `config::env_lookup`'s shape
- [x] 5.5 REFACTOR: Confirm the probe is a delegation and the decision function holds every
      rule, so each rule is provable without a process. If nothing needed cleaning, say so

      RECORDED: `npm_prefix_via` is a single `match` on `spawn`'s outcome delegating
      straight to `npm_prefix_from`; every trimming/emptiness/failure rule lives in
      `npm_prefix_from` alone, provable with no process. Nothing needed cleaning.
- [x] 5.6 VERIFY: `testcount 'cli::' 27`, then the full suite. **Red when:** fewer than 27
      tests match, the prefix is untrimmed, whitespace-only output yields a path, a failing
      run's output is kept, stderr reaches the result, or an unstartable program panics

      RECORDED: `testcount 'cli::' 27` -> OK (29 >= 27). Full suite: 295 passed (266
      baseline + 29 cli), 0 failed. `cargo clippy --all-targets --all-features -- -D
      warnings` clean.

## 6. The hand-over: repoint, observe the red, then replace
<!-- kind: behavior -->

- [x] 6.1 RED: Change `resolve::npm_prefix_deferred`'s **body** to call
      `crate::cli::npm_prefix()`, keeping its name and signature for this one step. Do not
      delete it yet — deleting it outright turns the hand-over into a compile error, which
      is a red of a sort but not the observation `repo-resolution` asked for
- [x] 6.2 RED — the hand-over observation, and the reason this group exists: run the
      `NPM-PATH` block from design.md → Test Strategy and record its **verbatim** output in
      this task's notes. Its precondition is **measured, not assumed**: it runs
      `$NPMBIN/npm prefix -g` itself and aborts unless that exits 0 and prints an absolute
      path. Only under that measured condition can the pinning test go red at all, and
      review corrected an earlier draft here — the first version asserted "npm is not on
      the PATH a plain shell inherits", which is **false** on this machine
      (`/bin/sh -c 'command -v npm'` prints `/opt/homebrew/bin/npm`). **The expected result
      is a FAILURE** of
      `cargo test --all-features resolve::tests::the_shipped_hook_yields_no_prefix` run
      with `$NPMBIN` on `PATH`, because that `npm prefix -g` exits 0 and prints the nvm
      version directory, so the hook now returns `Some(...)` where the test pins `None`.
      Then run the same test **without** `$NPMBIN` on `PATH` and record that output too —
      **informational only**, never a prediction: it currently stays green because the
      Homebrew node on this machine is broken (`dyld: Library not loaded:
      libllhttp.9.3.dylib`, exit 134, empty stdout), and a `brew reinstall node` would flip
      it. **If the run WITH the measured precondition satisfied is GREEN, stop: the binding
      was not actually repointed, or the hook is not reaching `cli`. Investigate. Do not
      adjust, relax, or delete the test to make it pass** — a pinning test that will not go
      red at its own hand-over is not doing its job, and that is the finding, not the
      inconvenience

      RECORDED — precondition measured: `PRECONDITION OK: npm prefix -g -> /Users/
      juusopiikkila/.nvm/versions/node/v24.20.0`.

      RECORDED — WITH `$NPMBIN` on `PATH` (expected FAILURE, verbatim):
      ```
      test resolve::tests::the_shipped_hook_yields_no_prefix ... FAILED
      thread 'resolve::tests::the_shipped_hook_yields_no_prefix' panicked at
      src/resolve.rs:1209:9:
      assertion `left == right` failed: this is expected to go RED the day
      subprocess-seam wires npm prefix -g through this hook — that failure is the
      intended hand-over signal, not a regression
        left: Some("/Users/juusopiikkila/.nvm/versions/node/v24.20.0")
       right: None
      test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 294 filtered out
      ```
      This is the hand-over signal: CONFIRMED RED, for the right reason (the hook now
      resolves a real prefix where the test still pinned `None`), not a harness error.

      RECORDED — WITHOUT `$NPMBIN` on `PATH` (informational only, verbatim):
      `test result: ok. 1 passed; 0 failed`. Matches the design's documented reason:
      Homebrew's `npm`/`node` on this machine are on the plain `PATH` but broken (dyld
      error), so `npm prefix -g` there still yields nothing and the test stayed green —
      consistent with, not contradicting, the measured-precondition red above.
- [x] 6.3 GREEN: Delete `resolve::npm_prefix_deferred` entirely and change
      `resolve::openspec_bin_from_env` to pass `&crate::cli::npm_prefix` as its
      fourth-step hook. Delete the now-satisfied pinning test
      `the_shipped_hook_yields_no_prefix` — it was written to die here, and its epitaph
      belongs in `planning-review.md`, not in a weakened assertion
- [x] 6.4 GREEN: Add two successors in `src/cli.rs`.
      `the_binding_delegates_to_the_probe_rather_than_answering_for_itself` asserts
      `cli::npm_prefix()` equals `cli::npm_prefix_via(Path::new("npm"))` — machine-
      independent, and, unlike an assertion on the value alone, **red for a hardcoded
      `None` body wherever a working `npm` exists**, which is the one wrong implementation
      every other check in this change would let through.
      `the_binding_yields_either_nothing_or_an_absolute_path` asserts the result is `None`
      or a path for which `is_absolute()` holds. Neither names a machine-specific value, so
      both pass where `npm` works, where it is broken, and on the no-tools `PATH` that 8.4
      runs the suite on
- [x] 6.5 CHECK: Confirm `src/resolve.rs` still names **no** process API at all — no
      `std::process`, no `Command`, no `Stdio` — including in its comments, which is a
      normative requirement of the `openspec-binary` capability. Run the module-scoped
      half of `NOSPAWN-GREP`. **Red when:** the rebinding pulled a process API name into
      `resolve.rs`, in code or in a doc comment

      RECORDED, corrected during Change Review: the original note here quoted
      `MODULE-SCOPED OK: resolve.rs names no process API` as if it were the check's own
      output — it was not. Design.md's `MODULE-SCOPED` block prints nothing on success
      (it only echoes on `FAIL`); that line was an ad-hoc echo from a simplified command
      run against `resolve.rs` alone rather than the verbatim block's actual behavior.
      Re-run properly (task 8.1's extracted, byte-identical `NOSPAWN-GREP.sh`, which
      contains both halves): `SRC=src sh NOSPAWN-GREP.sh` -> `NOSPAWN OK: 8 files checked
      under src, only src/cli.rs may spawn`, exit 0 — the tree-wide half's own success
      line, with the silent module-scoped half passing (confirmed by the exit code and
      by a direct `grep -nE 'std::process|Command|Stdio' src/resolve.rs` returning
      empty).
- [x] 6.6 CHECK: Run the `BINDING` block from design.md → Test Strategy. Its three guards
      are the only check in this change that can tell a completed hand-over from
      `pub fn npm_prefix() -> Option<PathBuf> { None }`: (a) `src/resolve.rs` names
      `cli::npm_prefix` where the composition passes its fourth-step hook; (b) no
      `npm_prefix_deferred` survives anywhere under `src/`; (c) `npm_prefix()`'s own body
      delegates to the probe rather than returning a literal. The `[ -f ]` / `[ -d ]`
      guards are load-bearing, because `grep` exits 2 on a missing path and a bare `!`
      would turn that into a pass. **Red when:** any of the three guards fails — and each
      is demonstrated red against a scratch copy in 8.3a, so none of them is taken on
      trust

      RECORDED: `BINDING OK` — but not on the first attempt, and the miss is worth
      recording. The deleted test's epitaph comment I wrote in `src/resolve.rs` quoted
      the literal identifier `npm_prefix_deferred()` in prose, which guard (b) — "no
      `npm_prefix_deferred` survives anywhere under `src/`" — correctly flagged:
      `BINDING FAIL: placeholder survives: src/resolve.rs:1200`. The guard is
      text-blind to intent, exactly as designed ("including in its comments"). Reworded
      the comment to describe the deleted function without spelling its identifier,
      re-ran, and got `BINDING OK`. This is the guard doing its job, not a false
      positive — recorded rather than silently fixed.
- [x] 6.7 CHECK — contract gate: `resolve::npm_prefix_deferred` is a **removal** from the
      crate's published surface. Confirm its only caller, `openspec_bin_from_env`, was
      updated, that no test injects it, and that the injection point itself — the
      `&dyn Fn() -> Option<PathBuf>` parameter — did not move, so every existing chain
      test is unaffected

      RECORDED: only caller (`openspec_bin_from_env`) updated to pass
      `&crate::cli::npm_prefix`; no test injects `npm_prefix_deferred` (it never existed
      as an injectable — every chain test injects its own closure); the parameter
      remains `npm_prefix: &dyn Fn() -> Option<PathBuf>` on both `openspec_bin` and
      `step4_npm_prefix`, unmoved. `resolve::` suite: 41 passed, 0 failed (255 filtered
      out of 296), confirming every existing chain test — closure-injected fourth step
      included — is unaffected.
- [x] 6.8 VERIFY: `cargo test --all-features` green in full — `resolve`'s chain tests,
      including the closure-driven fourth-step tests, must all still pass. **Red when:**
      removing the placeholder broke a caller, or the chain's fourth step stopped being
      injectable

      RECORDED: full suite 296 passed, 0 failed (265 non-cli unit + 31 cli + 11
      ci_workflow + 5 cli.rs binary-integration — one fewer non-cli unit test than the
      266 baseline, since the pinning test was deleted and replaced by two cli::
      successors already counted in the 31). `cargo clippy --all-targets --all-features
      -- -D warnings` clean.

## 7. The end-to-end join, and proof that nothing is written
<!-- kind: behavior -->

- [x] 7.1 RED: Write the failing test `a_real_spawn_resolves_the_npm_prefix_binary`: call
      `resolve::openspec_bin` with nothing configured, a `PATH` naming one scratch
      directory that holds no `openspec`, **no** `NVM_DIR` and **no** `HOME` in the
      lookup, and a fourth-step hook that is the real spawning probe pointed at a scratch
      `#!/bin/sh` program printing a scratch prefix `N` to stdout and unrelated noise to
      stderr, with `N/bin/openspec` written at mode `0o755`. Assert the resolved binary is
      `N/bin/openspec` **and** that its source is the npm-prefix step, so a chain that
      reached it by another route fails. This is the roadmap's end-to-end obligation:
      until now that join was exercised only by fixture closures returning a literal path
- [x] 7.2 RED: Write `a_failing_real_spawn_resolves_nothing` — the same call with a scratch
      program that prints the same prefix to stdout but exits `1`; assert no binary is
      resolved and no problem is recorded, because an absent CLI is a supported state
      rather than a fault
- [x] 7.3 GREEN: Make both pass. If groups 5 and 6 were built correctly this requires no
      production change; record that explicitly rather than leaving it ambiguous, since
      "the test passed without a code change" is only acceptable when it was predicted

      RECORDED: predicted and confirmed — both tests passed with no production change,
      since groups 5 (the probe) and 6 (the binding/injection point) were already
      correct. The only fix needed was in the *test* code itself (an `E0716` borrow
      error from an inline temporary `&[("PATH", ...)]` — fixed with a `let` binding),
      not in `src/cli.rs` or `src/resolve.rs` production code.
- [x] 7.4 RED then GREEN: Write `a_run_and_a_probe_leave_the_scratch_tree_byte_identical`
      — snapshot a scratch tree holding the scratch programs, a prefix directory with
      `bin/openspec`, and an **empty** directory using `testutil::snapshot`, then perform a
      successful run, a failing run, an unstartable run, and a full npm probe against it,
      then snapshot again and assert equality. The snapshot records directory entries and
      modification times, not just file listings, so a helper that created a missing
      directory while looking for one is caught. Take a **second** snapshot pair around the
      same four operations over `std::env::current_dir()`, so the requirement's "not in the
      working directory" clause is covered by evidence rather than implied by the scratch
      tree's result — review caught that overclaim

      DRIFT FOUND AND CORRECTED: written exactly as specified first — a second full
      recursive `testutil::snapshot` around `std::env::current_dir()`. It passed, but
      took **7.59s** for this one test alone: under `cargo test`, the real cwd is this
      crate's own repository root, whose `target/` directory holds ~42,400 files and
      ~468MB, all read byte-for-byte by `snapshot`'s recursive walk, on every test run,
      forever (`make check`, CI, every future `cargo test`). This is a genuine
      reliability/performance defect in the design's test-boundary choice, not a
      cosmetic one — caught here per the standing instruction to distrust a check that
      passes suspiciously easily, and per the operating hard rule "no timing-based test"
      cousin: a test whose cost scales with unrelated repository state is exactly the
      kind of check that becomes flaky or prohibitively slow without ever having been
      wrong.

      Fix: added `testutil::shallow_snapshot` (`src/lib.rs`) — a *non-recursive* listing
      of a directory's direct children (path, is-dir, mtime; never bytes, never
      descends) — and used it for the cwd half only; the scratch-tree half keeps the
      full recursive `snapshot`, since that tree is small and under this change's
      control. Verified the replacement (a) is fast: 0.10s vs 7.59s for the same test,
      and (b) still has genuine discriminating power: planted
      `std::fs::write(cwd.join("stray-test-artifact.txt"), b"oops")` after the
      operations under test, reran, and got a genuine `FAILED` with the exact stray file
      named in the diff; removed the plant and confirmed green again. Rationale this is
      still faithful evidence: nothing in `cli`'s implementation computes any path
      relative to the current directory (every path is either an absolute scratch path
      or the bare literal `"npm"`/`"herdr"`), so a stray write this seam caused would
      necessarily land as a new, removed, or modified **top-level** entry — exactly what
      `shallow_snapshot` catches.

      This is a material change to a test-boundary mechanism (design.md → Test
      Boundaries and → Decisions), so treated as drift: design.md's verification-matrix
      row for "A run and a probe leave the scratch tree byte-identical" (the
      cwd-snapshot half) is updated to name `shallow_snapshot` and the reason, and
      `planning-review.md` gets a repair-log row recording this before/after. See both
      files for the corrected text; `openspec validate subprocess-seam --strict` re-run
      clean after the edit (task 11.7 re-confirms at the end of the change).
- [x] 7.5 REFACTOR: Extract the scratch-program builders this group and groups 2, 3, and 5
      share into one local helper in `src/cli.rs`'s test module, rather than repeating the
      shebang and the mode in a dozen tests. If nothing needed cleaning, say so here

      RECORDED: already a single shared `script()` helper (added in group 2, task 2.1),
      used by every scratch-program test in groups 2, 3, 5, 6, and 7. Nothing needed
      extracting.
- [x] 7.6 VERIFY: `testcount 'cli::' 31`, then `cargo test --all-features` in full. **Red
      when:** fewer than 31 tests match, the real probe's output cannot be consumed by the
      chain, the npm-prefix source is misreported, a failing probe still resolves a binary,
      or anything in the seam writes to the filesystem or to the working directory

      RECORDED: `testcount 'cli::' 31` -> OK (34 >= 31). Full suite: 299 passed (265 +
      34), 0 failed, in 0.35s (versus what would have been ~7s+ with the unfixed
      cwd-snapshot). `cargo fmt --all -- --check` and `cargo clippy --all-targets
      --all-features -- -D warnings` both clean.

## 8. Architectural checks that can actually fail
<!-- kind: operational -->

- [x] 8.1 CHECK: Copy each command block out of design.md → Test Strategy into a scratch
      shell file **verbatim** and run the blocks from that file, rather than retyping a
      shortened form. Record the scratch file's path in this task's notes so the review in
      group 9 can diff it against the design. Every one of the defects Phase 1-2 review
      caught was a check shortened at the moment of running it, and "run it as written" is
      not a mechanism unless the thing that was run is recoverable afterwards

      RECORDED: extracted verbatim (via `sed` line ranges against design.md, not
      retyped) to
      `/private/tmp/claude-501/-Users-juusopiikkila-Code-herdr-openspec/d5be0e1c-512c-496f-bdf6-1ea630cd0644/scratchpad/subprocess-seam-checks/{NOSPAWN-GREP,NOSPAWN-RUN,NPM-PATH,BINDING,DEPS,OPENSPEC-UNTOUCHED,TESTCOUNT}.sh`.
      Only the Markdown code-fence lines (` ``` `) were stripped; every other character is
      as design.md has it.
- [x] 8.2 VERIFY: Run `NOSPAWN-GREP` against `src` — it must now **pass**, reporting at
      least 8 files checked. **Red when:** any `*.rs` file under `src/` other than
      `src/cli.rs` names `process::Command`, `Command::new`, or `Stdio`; when `src/cli.rs`
      is missing (guard A); when `src/cli.rs` names no spawn API, making the exclusion
      vacuous (guard B); or when fewer than 8 non-`cli` files are found, which is how an
      empty or wrong-directory run fails instead of passing (guard C)

      RECORDED: `NOSPAWN OK: 8 files checked under src, only src/cli.rs may spawn`.
- [x] 8.3 VERIFY — the negative controls, which are what make 8.2 evidence rather than
      decoration. Run `NOSPAWN-GREP` four more times with `SRC` pointed at scratch copies
      of `src/`: (a) a copy with `Command::new("openspec")` planted in a non-`cli` file —
      must fail, naming the file and line; (b) a copy with `cli.rs` deleted — must fail on
      guard A; (c) a copy with `cli.rs` emptied of its spawn API — must fail on guard B;
      (d) a copy with the spawn planted at `ui/cli.rs` — must fail, proving the exclusion
      is by **path** and not by base name. Control (d) exists because review demonstrated
      that a base-name exclusion silently exempts a future `src/ui/cli.rs` while the file
      count still reads 8. Record all four exit codes and messages. Also confirm that run
      (a)'s planted line is what fired and not one of the three legitimate `.join("herdr")`
      sites in `src/config.rs` and `src/state.rs`, which is exactly what `plugin-config`'s
      `grep -rn '"herdr"' src/` could not distinguish and why that check was recorded as
      not-run and is not inherited here. **Red when:** any of the four passes

      RECORDED — all four exit 1, none passed:
      (a) `Command::new("openspec")` planted in a copy's `schema.rs` (line 1500) ->
      `NOSPAWN FAIL: spawn API outside .../cli.rs: .../schema.rs:1500:fn planted() { ... }`
      — confirmed by separate `grep` that this fired line is distinct from the three real
      `.join("herdr")`/`.join("openspec")` sites at `config.rs:59`, `state.rs:34`,
      `state.rs:44`.
      (b) `cli.rs` deleted -> `NOSPAWN FAIL: .../cli.rs missing - the exclusion has
      nothing to exclude` (guard A).
      (c) `cli.rs` emptied -> `NOSPAWN FAIL: .../cli.rs names no spawn API - exclusion is
      vacuous` (guard B).
      (d) spawn planted at `ui/cli.rs` -> `NOSPAWN FAIL: spawn API outside .../cli.rs:
      .../ui/cli.rs:1:fn planted() { ... }` — proves the exclusion is by path, not base
      name. All four scratch copies removed afterward.
- [x] 8.3a VERIFY — the `BINDING` negative controls, for the same reason. Run `BINDING`
      against three scratch copies: one whose `resolve.rs` has the `cli::npm_prefix`
      reference stripped (guard a must fire), one still holding `npm_prefix_deferred`
      somewhere (guard b), and one whose `npm_prefix()` body is `None` (guard c). Record
      each exit code and which guard named itself. **Red when:** any of the three passes —
      which would mean the one check standing between this change and a hand-over that
      never happened is decoration

      RECORDED — baseline on the real tree: `BINDING OK`. All three negative controls
      exit 1: (a) `cli::npm_prefix` reference replaced with a dummy identifier ->
      `BINDING FAIL: src/resolve.rs does not name cli::npm_prefix` (guard a). (b) a
      `npm_prefix_deferred` function appended to a copy's `resolve.rs` -> `BINDING FAIL:
      placeholder survives: src/resolve.rs:1353:pub fn npm_prefix_deferred() -> ...`
      (guard b). (c) `npm_prefix()`'s body replaced with `{ None }` -> `BINDING FAIL:
      npm_prefix() does not delegate to the probe: pub fn npm_prefix() -> Option<PathBuf>
      { None }` (guard c). All three scratch copies removed afterward.
- [x] 8.4 VERIFY: Run `NOSPAWN-RUN` — the whole suite on a `PATH` from which every
      directory holding `npm`, `node`, or `openspec` has been removed, with the five
      preconditions running **first** and aborting. **Red when:** any of `npm`, `node`, or
      `openspec` is still resolvable on that `PATH` (precondition), when `cargo` or
      `rustc` is not (precondition), or when any test fails there — including this
      change's own, since every spawning test must name an absolute scratch path and
      `npm_prefix()`'s smoke test must tolerate an absent `npm`

      RECORDED: all five preconditions passed (npm/node/openspec confirmed unresolvable
      on the stripped `PATH`; cargo/rustc confirmed still resolvable), then
      `env PATH="$NOTOOLS" cargo test --all-features` ran the full suite: 299 passed, 0
      failed (lib) + 11 (ci_workflow) + 5 (cli.rs binary-integration), matching the
      normal-`PATH` run exactly. Confirms every spawning test in `cli::` names an
      absolute scratch path and `npm_prefix()`'s smoke test tolerates a fully absent
      `npm`.
- [x] 8.5 VERIFY: Run `OPENSPEC-UNTOUCHED` with `BASE` set to the SHA captured in 1.1, and
      demonstrate its red once by planting an untracked file under `openspec/specs/` and
      confirming the check names it — then remove it. Both halves matter: `git diff` lists
      **tracked** paths only, and a file written at runtime is untracked, so the diff alone
      was blind to the one violation this check exists to catch (review demonstrated the
      earlier draft reporting success over a planted file). **Red when:** `BASE` is unset,
      names no commit, or the repository root cannot be found (all abort), or when any path
      under `openspec/` outside `openspec/changes/subprocess-seam/` differs from the base
      commit or exists as an untracked file

      RECORDED, all three phases: clean tree -> `OPENSPEC-UNTOUCHED OK`. Planted
      untracked `openspec/specs/foo/cache.md` -> `FAIL: wrote inside openspec/ outside
      this change: openspec/specs/foo/cache.md`, exit 1 — demonstrating the red the
      untracked-files sweep exists to catch. Plant removed -> `OPENSPEC-UNTOUCHED OK`
      again; `git status --short openspec/` confirmed clean afterward.
- [x] 8.6 VERIFY: Run `DEPS`. **Red when:** the normal dependency set is anything but
      `["toml", "yaml-rust2"]`, or the dev or build sets are non-empty, or the input is not
      JSON. Dev and build are pinned empty because they are empty today and a test-support
      crate such as `assert_cmd` is the likeliest accidental addition for a change that
      starts spawning things — review demonstrated that a normal-deps-only form let a
      planted dev-dependency through. Checked with `cargo metadata` rather than
      `cargo build --offline`, which proves only that the cache is warm

      RECORDED: `DEPS OK` — normal deps still exactly `["toml", "yaml-rust2"]`, dev and
      build still empty; this change added no dependency of any kind.
- [x] 8.7 CHECK — persistence gate: record that none of migration, backfill, cache
      invalidation, or index rebuild applies. `resolve::BinCache` is unchanged and still
      caches at most one probe per owned value; no file format, no stored data, and no
      index exists in this change. Recorded rather than omitted, so the gate is answered

      RECORDED: confirmed — `resolve::BinCache` (a `std::sync::OnceLock<BinResolution>`
      wrapper) is untouched by this change; nothing in `src/cli.rs` introduces a file
      format, persisted state, migration, backfill, or index. Not applicable, answered
      rather than omitted.

## 9. Change Review
<!-- kind: operational -->

- [x] 9.1 CHECK: Dispatch an independent reviewer — an agent that did **not** write the
      implementation and is **not** a fork of the implementing session, per
      `openspec/config.yaml`'s planning-review rule — given only `proposal.md`, both spec
      files, `design.md`, `tasks.md`, and the diff against the SHA from 1.1. Ask it to
      spend its attention first on: every verification command in this file and whether
      each can concretely go red; whether each spec scenario's test would fail if the
      behavior were deleted; whether anything that parses, decides, or merges leaked into
      `src/cli.rs`; and whether the fake could let a caller's test pass while the caller
      spawned the wrong command. Require it to write findings to a scratchpad file
      incrementally as it goes rather than only in a final message

      RECORDED: dispatched the `outside-in-tdd-reviewer` agent (not a fork; fresh
      context) with the planning docs, tasks.md's full text (whose RECORDED notes it
      was told to treat as evidence, not decoration), and the diff against
      `b2e85f179b13f7b43ad2165882d2bab29375e119`. It wrote findings incrementally to
      `/private/tmp/claude-501/.../scratchpad/subprocess-seam-review-findings.md` and
      independently re-ran (not merely re-read) every check block, all four
      `NOSPAWN-GREP` negative controls, all three `BINDING` guards, `NOSPAWN-RUN`,
      `DEPS`, `OPENSPEC-UNTOUCHED`, and the `shallow_snapshot` claim — via its own
      mutation testing in each case, not by trusting the recorded text. Result: **0
      CRITICAL, 2 WARNING, 6 SUGGESTION.**
- [x] 9.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
      one-line reason, note each SUGGESTION, and re-run the affected tests

      RECORDED — 0 CRITICAL: none to fix.

      WARNING 1 — `a_non_zero_exit_is_a_failure_carrying_the_code_and_stderr` never
      asserted `CliError::Failed.program`; reviewer proved it by mutating `run_and_map`
      to hardcode `program: "MUTANT"` and the suite stayed green. **Fixed**: the test now
      clones the program path before construction and asserts `program` equals it, and
      the "stdout payload is nowhere in the error" clause is now checked against the
      whole error's `Debug` rendering (the old `!stderr.contains("partial")` assertion
      could never fail, since `stderr` was already asserted `== "boom\n"` two lines
      above — a second finding folded into the same fix).

      WARNING 2 — groups 10 (Documentation) and 11 (Lint & Verify) are not yet done,
      confirmed by the reviewer independently running task 10.8's `STALEDOC` block
      against the tree as it stood (correctly red, naming all six stale phrases).
      **Accepted, not a defect**: this is simply "the orchestrator had not reached those
      groups yet" at review time, per this schema's own group ordering — resolved by
      doing groups 10 and 11 next, in order, as planned.

      SUGGESTION 1 — `each_handle_gets_its_own_registration` wasn't independently
      discriminating: the reviewer collapsed the fake's key to the argument vector alone
      and the test stayed green by queue-order coincidence. **Fixed**: now calls the
      `HerdrCli` handle first (out of registration order) and additionally asserts
      `fake.calls()` names the two distinct `Program` values.

      SUGGESTION 2 — `RealOpenspecCli::program()` had no test (the only uncovered
      non-test code in the seam). **Fixed**: added
      `openspec_clis_program_accessor_reports_the_constructed_path`.

      SUGGESTION 3 — the thread-boundary test's `HerdrCli` half asserted only
      `inline == threaded`, which two `Err(NotStarted)` values would satisfy. **Fixed**:
      added `assert_eq!(inline, Ok("ok".to_string()))`.

      SUGGESTION 4 — task 6.5's RECORDED note quoted `MODULE-SCOPED OK: ...` as if it
      were the design block's own output; design.md's module-scoped block actually
      prints nothing on success. **Fixed**: reworded to say so and re-ran the real,
      byte-identical `NOSPAWN-GREP.sh` (which contains both halves) for accurate
      evidence.

      SUGGESTION 5 — design.md → Decisions claimed the fake's `#[cfg(test)]` placement
      "contributes nothing to the coverage denominator" — false: `cargo llvm-cov`
      instruments the test binary, which compiles in every `#[cfg(test)]` item including
      `mod tests` itself (`cli.rs` measures 567-569 lines against ~380 outside `mod
      tests`). **Fixed**: corrected design.md's Decisions entry and added a matching
      correction note to tasks.md 4.2, both stating the true, narrower claim
      (`cfg(test)` excludes the fake from the release binary only) and that this
      change's own 80% floor does not rely on the false claim.

      Re-ran after all fixes: `cargo test --all-features` -> 300 passed, 0 failed (265 +
      35 cli + 11 ci_workflow + 5 cli.rs, one more than group 8's 299 for the new
      `program()` test). `cargo clippy --all-targets --all-features -- -D warnings` and
      `cargo fmt --all -- --check` both clean. `cargo llvm-cov --fail-under-lines 80`:
      98.67% over 5499 lines (up from 98.61%/5477 at the reviewer's snapshot, and above
      the 98.66%/4924 baseline). `openspec validate subprocess-seam --strict`: valid.
- [x] 9.3 VERIFY: Confirm no blocking or unowned finding remains, and that any artifact a
      finding invalidated — `design.md`, a spec file, or this list — was updated rather
      than left to drift

      RECORDED: no CRITICAL and no unowned WARNING remains — WARNING 1 is fixed in
      code, WARNING 2 is owned by groups 10-11 (next). Both SUGGESTIONs touching
      design.md (the `shallow_snapshot` rationale had already been recorded correctly by
      this orchestrator, and the `cfg(test)`/coverage-denominator claim) are corrected in
      `design.md` itself, not left to drift; no spec file required a change from this
      review. This tasks.md file was updated throughout 9.1-9.2 rather than only
      afterward.

## 10. Documentation
<!-- kind: operational -->

- [x] 10.0 CHECK: Capture, verbatim and with line numbers, the stale phrases the tasks
      below must make cease to exist, so 10.8 can prove they are gone rather than that
      something was merely added beside them. Each phrase below was chosen because it lies
      **within a single line** of the wrapped source — a phrase spanning a line break
      cannot be matched by a line-oriented search, and a check that can never match is the
      defect this project keeps finding. Confirmed present at planning time:
      `-> Result<String>;` (`SPEC.md`:28 and :29, two occurrences — both trait sketches);
      `thin wrappers, the` (`SPEC.md`:42); `unwired until` (`SPEC.md`:201);
      `Each is a pure transformation, tested without a TUI or a subprocess`
      (`SPEC.md`:396); `always returns nothing until` (`AGENTS.md`:29); and
      `test ever spawns a real process` (`openspec/IMPLEMENTATION-ORDER.md`:21).
      **Red when:** any is absent, or its occurrence count differs from the one recorded
      here — either means the document moved under the plan and the rewrite targets need
      re-reading before anything is edited

      RECORDED: re-confirmed by `grep -n` immediately before any edit — all six phrases
      present at exactly the line numbers above, no drift since planning time.
- [x] 10.1 Rewrite in `SPEC.md`: Architecture → The subprocess seam, the trait snippet
      (audience: every future change that implements or consumes the seam) — it shows
      `fn run(&self, args: &[&str]) -> Result<String>;` with no error type and no thread
      bound. Replace with the shipped signature including `Send + Sync` and `CliError`,
      and one line on why the bound is there: `live-refresh` runs CLI calls on a worker
      thread and `agent-polling` polls on another, so a trait that cannot cross a thread
      would have to be redesigned by its first consumer. Record the before/after in
      `planning-review.md`

      RECORDED: the before/after was already captured in `planning-review.md`'s
      "SPEC.md and roadmap corrections" table row 1, written at planning time; the
      shipped rewrite matches it (`pub trait OpenspecCli: Send + Sync { fn run(&self,
      args: &[&str]) -> Result<String, CliError>; }` plus the `live-refresh`/
      `agent-polling` thread rationale). No further planning-review.md update needed.
- [x] 10.2 Rewrite in `SPEC.md`: Architecture → The subprocess seam, the residue sentence
      (audience: every future change reasoning about the coverage target) — "the
      untestable residue is two thin wrappers, the `npm prefix -g` binding, and `main`" is
      now wrong: the wrappers and the probe are covered by tests against scratch
      `#!/bin/sh` programs, and the residue is the one-line `npm_prefix()` program binding
      plus `main`. Rewrite in place; do not append a correction beside the stale claim

      RECORDED: rewritten in place (`planning-review.md` row 2). `grep -n 'thin
      wrappers, the' SPEC.md` -> no match afterward.
- [x] 10.3 Rewrite in `SPEC.md`: Data layer → Resolution chain, the paragraph beginning
      "Step 4 is unwired until `subprocess-seam` lands" (audience: every future change
      reading the binary probe chain) — entirely superseded. Replace with the landed
      state: the hook stays injected, which is what keeps `resolve` pure; its production
      binding is `cli`'s probe; and the stdout-only, trimmed rule with a non-zero exit or
      empty output meaning no prefix is now that binding's contract rather than an
      instruction to a future change. Removes more text than it adds

      RECORDED: rewritten in place (`planning-review.md` row 3). `grep -n 'unwired
      until' SPEC.md` -> no match afterward.
- [x] 10.4 Add in `SPEC.md`: Testing and quality gates → Unit-tested modules (audience:
      every future change adding a test to `cli`) — one entry for `cli`: the traits'
      contract and the `npm prefix -g` probe, tested against scratch `#!/bin/sh` programs
      built under `std::env::temp_dir()` rather than against the real `openspec`, `herdr`,
      or `npm`, so the suite passes with all three unresolvable. Three lines; it is the
      only place that records why spawning in a test here is not a breach of the rule

      RECORDED: added (`planning-review.md` row 4).
- [x] 10.4a Rewrite in `SPEC.md`: Testing and quality gates → Unit-tested modules, the
      **lead-in sentence** (audience: the same) — it reads "Each is a pure transformation,
      tested without a TUI or a subprocess:", which 10.4's new entry contradicts one line
      below it. Review caught this. Narrow it: these modules are pure transformations
      tested without a TUI, and `cli` is the one exception, tested against scratch programs
      because performing a spawn is what it exists to do. Rewrite in place — adding 10.4's
      entry without this leaves a self-contradicting paragraph

      RECORDED: rewritten in place, combined with 10.4's addition in one edit
      (`planning-review.md` row 5). `grep -nF 'Each is a pure transformation, tested
      without a TUI or a subprocess' SPEC.md` -> no match afterward.
- [x] 10.4b Rewrite in `openspec/IMPLEMENTATION-ORDER.md`: Ordering principles, the
      subprocess-seam bullet (audience: every future change reading the roadmap) — it says
      the seam lands before the first change that shells out "so no test ever spawns a real
      process", which this change falsifies with roughly twenty scratch `#!/bin/sh` spawns
      in its own tests. The principle's intent is sound and `tests/cli.rs` is an existing
      precedent for the narrower reading, but as written the sentence is now false. Narrow
      it to say what it means: no test spawns the `openspec` or `herdr` binaries. The
      `subprocess-seam` **row** itself needs no correction — 10.7 re-confirms that

      RECORDED: rewritten in place (`planning-review.md` row 8). `grep -nF 'test ever
      spawns a real process' openspec/IMPLEMENTATION-ORDER.md` -> no match afterward.
- [x] 10.5 Rewrite in `AGENTS.md`: Current repo state (audience: every future session) —
      the sentence "the binary chain's fourth probe step ships as an injected hook that
      always returns nothing until `subprocess-seam` wires it" is false once this lands.
      Rewrite that clause in place and add `subprocess-seam` to the list of landed
      changes; do not append a second paragraph beside the stale one. Net roughly neutral

      RECORDED: clause rewritten in place, `subprocess-seam` added to the landed-changes
      list in the same sentence (`planning-review.md` row 6). `grep -nF 'always returns
      nothing until' AGENTS.md` -> no match afterward.
- [x] 10.6 Rewrite in `AGENTS.md`: Architecture rules, the spawn bullet (audience: every
      future session) — it describes the seam in the future tense. Rewrite it to say the
      seam exists, that `src/cli.rs` is the one module permitted to name a process-spawn
      API, and that the check guarding it excludes exactly that file and fails when the
      exclusion is vacuous. Keep it to the rule plus the non-obvious reason; the
      verification block itself lives in this change's `design.md`, not in `AGENTS.md`

      RECORDED: rewritten in place (`planning-review.md` row 7) — names `src/cli.rs` as
      the one permitted module, and states the exclusion is by path (not base name) and
      fails when vacuous, without reproducing the verification block itself.
- [x] 10.7 CHECK: `openspec/IMPLEMENTATION-ORDER.md`'s `subprocess-seam` row was re-read
      at planning time and describes exactly what was built, including the end-to-end
      obligation. Re-read it once more against the landed change and correct it only if
      the work split, merged, or moved. **Red when:** the row no longer describes the
      change — in which case correct the row, and record the correction in
      `planning-review.md`

      RECORDED: re-read the row (`openspec/IMPLEMENTATION-ORDER.md`:63) against the
      landed change — it still describes exactly what was built: the two traits, one
      real implementation each, the fake, the hand-over replacing
      `resolve::npm_prefix_deferred`, the stdout-only/trimmed rule, and the end-to-end
      obligation of driving the real hook through to `<prefix>/bin/openspec`. No
      correction needed, as `proposal.md` and `planning-review.md` both anticipated.
- [x] 10.8 VERIFY: Every phrase captured in 10.0 is gone. Run from the repository root, and
      judge on output rather than on an exit code, because `grep` exits 1 for no-match and
      2 for a missing file and both would satisfy a bare `!`:

      ```sh
      for f in SPEC.md AGENTS.md openspec/IMPLEMENTATION-ORDER.md; do
        [ -f "$f" ] || { echo "STALEDOC FAIL: $f missing" >&2; exit 1; }
      done
      hits=$( { grep -nF -e '-> Result<String>;' -e 'thin wrappers, the' \
                        -e 'unwired until' \
                        -e 'Each is a pure transformation, tested without a TUI or a subprocess' \
                        SPEC.md
                grep -nF -e 'always returns nothing until' AGENTS.md
                grep -nF -e 'test ever spawns a real process' \
                        openspec/IMPLEMENTATION-ORDER.md; } || true)
      [ -z "$hits" ] || { echo "STALEDOC FAIL:" >&2; echo "$hits" >&2; exit 1; }
      echo "STALEDOC OK"
      ```

      `-F` rather than `-E` on purpose: these are literal phrases, and an escaped `|`
      inside an ERE matches a literal pipe — the exact defect Phase 1 review found.
      Demonstrate the red once before the rewrites land, by running the block against the
      tree as it stands and confirming it names all six phrases. **Red when:** any stale
      phrase survives — which is exactly what a task that appended a correction beside the
      old text, instead of rewriting it in place, would leave behind

      RECORDED: the red was demonstrated **after the fact**, against the last pre-edit
      commit (`496c73a`) via `git show 496c73a:<file> | grep`, rather than as a separate
      step before editing — task 10.0's individual per-phrase greps served as the
      pre-edit confirmation at the time, but the combined `STALEDOC` script itself was
      not run until after all six rewrites landed. Recorded honestly rather than
      implied otherwise: `git show 496c73a:SPEC.md/AGENTS.md/IMPLEMENTATION-ORDER.md`
      piped through the same `grep -nF` lines names all six phrases at their original
      line numbers, confirming the block's red is genuine and not a check that could
      never fail. Run against the current tree: `STALEDOC OK` — all six gone.

## 11. Lint & Verify
<!-- kind: operational -->

- [x] 11.1 CHECK: Inspect the intended verification commands and the tiers they touch —
      the unit tier (`cargo test --all-features`, covering `cli::` and `resolve::`), the
      binary-integration tier (`tests/cli.rs`, untouched by this change but re-run), and
      the command-level checks from group 8. There is no view tier here, because no view
      was added

      RECORDED: confirmed — unit tier (`cli::` 35 tests, `resolve::` chain tests
      unaffected), binary-integration tier (`tests/cli.rs`, 5 tests, byte-for-byte
      unchanged by this change and still passing), and group 8's seven command-level
      checks (all re-verified in this group via `make check` and a direct
      `openspec validate` run). No view tier, as designed.
- [x] 11.2 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.
      **Red when:** any lint fires, including dead code in the `cfg(test)` fake

      RECORDED: clean, 0 warnings, 0 errors.
- [x] 11.3 VERIFY: `cargo fmt --all -- --check` — clean. **Red when:** any file is
      unformatted

      RECORDED: clean, exit 0.
- [x] 11.4 VERIFY: `cargo test --all-features` — green, and record the total test count.
      Rust's type checker runs as part of every build here, so there is no separate
      type-check command; the compile that backs 11.2 and 11.4 is it. **Red when:** any
      test fails, or the total is not greater than the baseline recorded in 1.1 (266 unit +
      11 ci_workflow + 5 cli) — a suite that grew by fewer tests than the groups above
      wrote means tests were dropped or a filter silently matched nothing

      RECORDED: 300 unit + 11 ci_workflow + 5 cli.rs binary-integration = 316 total,
      versus the 282-test baseline (266 + 11 + 5) — grew by 34, all failures 0. 300 is
      strictly greater than 266, satisfying the red condition's inverse.
- [x] 11.5 VERIFY: `cargo llvm-cov --fail-under-lines 80` — the coverage gate, which this
      change's code is squarely inside. Record the reported percentage and total lines
      beside the baseline from 1.1 (98.66% over 4924 lines). **Red when:** line coverage
      falls below 80% — the enforced floor — **or** when it falls more than one percentage
      point below the baseline, which is the softer condition that catches a seam whose
      real implementations ended up unreachable while the hard floor still passes. Neither
      threshold is ever lowered, waived, or given an exclusion: if a line is genuinely
      unreachable the answer is to shrink the residue to one line, not to exempt it

      RECORDED: **98.67% over 5499 lines**, against the 98.66%-over-4924-lines baseline
      — 0.01 percentage points *above* baseline, not below it, and far above both the
      80% hard floor and the "no more than one point below baseline" soft condition.
      `cli.rs` itself: 98.42% line coverage (569 lines, 9 missed — the `RealOpenspecCli`
      constructor path not yet fully exercised beyond `program()`'s own test plus a
      couple of defensive branches). Neither threshold touched, waived, or excluded.
- [x] 11.6 VERIFY: `make check` as the single composite gate, and name the failing
      sub-command rather than a summary if it fails

      RECORDED: `make check` exit code 0 (captured directly, not inferred from tail
      output) — fmt-check, lint, test, and coverage all passed in sequence. Nothing to
      name as failing.
- [x] 11.7 VERIFY: `openspec validate subprocess-seam --strict`. **Red when:** the change's
      artifacts drift from the schema. Run it with the nvm directory on `PATH` — the
      `openspec` binary is nvm-installed here and is not on the `PATH` a plain shell
      inherits, the same fact that makes probe steps 3 and 4 exist

      RECORDED: `Change 'subprocess-seam' is valid`, exit 0, with
      `$HOME/.nvm/versions/node/v24.20.0/bin` prepended to `PATH`.
