# Manual verifications — tasks 2.10 and 2.11

Two scenarios in this change cannot be driven from inside `cargo test`: `make check`'s
`test` target **is** `cargo test --all-features`, so a test that ran real `make check` on a
tree copy would re-enter the whole suite in the copy, which would run `ci_workflow` again,
which would copy again — unbounded (design.md → Test Strategy). They are therefore verified
once, by hand, at apply time, and recorded here so the evidence outlives the session.

What stays permanently checked is the **ordering**, asserted against the `Makefile`'s parsed
prerequisite list by `tests/ci_workflow.rs::check_composes_gates_third` — text, not
execution, and design.md → Risks records that residue rather than papering over it.

## Task 2.10 — the structural half reports while the suite is red

Plant: `assert_eq!(1 + 1, 3);` as the first statement of
`src/integration.rs`'s `the_measured_seventeen_line_status_parses_whole`. The shape matters:
`assert!(false)` trips `clippy::assertions-on-constants`, which is `-D warnings` here, so
`make check` would exit at **lint** two steps before `covers-check` ever ran and the
scenario would prove nothing. Measured: the `assert_eq!` shape passes clippy and fails the
test.

**Red suite alone.** `make check`:

```
cargo fmt --all -- --check                      # passed
cargo clippy --all-targets --all-features -- -D warnings   # passed
… 33 gate invocations …                          # passed
cargo test --all-features --test degraded_coverage
    test result: ok. 15 passed; 0 failed; 1 ignored          <- covers-check PASSED
cargo test --all-features
    integration::tests::parse::the_measured_seventeen_line_status_parses_whole panicked
    make: *** [test] Error 101
EXIT=2
```

`covers-check` ran and reported before the suite that was going to fail.

**Red suite plus a mis-bound range.** Same plant, plus one row's `covers` set to
`src/ui/app.rs:820-846` (`pub struct Dashboard`'s field declarations — the
`settings-window` shape). `make check`:

```
cargo test --all-features --test degraded_coverage
    row "A tasks file exists but cannot be read (…)": covers entry src/ui/app.rs:820-846
    holds no statement — every line is blank, a comment, an item declaration, a struct
    field or enum variant, an attribute, or a lone delimiter, so the range names where the
    code is declared rather than where it runs
    test result: FAILED. 13 passed; 2 failed; 1 ignored
make: *** [covers-check] Error 101
EXIT=2
```

It exits at `covers-check` rather than at `test`, naming the row's `condition`, with the
suite still red. Both plants were removed; `git status` clean.

## Task 2.11 — the floors are not moved ahead of the suite

A clean tree has both targets exiting 0, which discriminates nothing. The scenario names a
**below-floor** tree, so that is what was built: `git ls-files` copied to a scratch
directory, then the test modules of `src/specs.rs`, `src/ui/help.rs`, `src/ui/palette.rs`
and `src/cli.rs` marked `#[ignore]`. None of those four files carries a `covers` range, so
the run fails on the floor rather than on a cold range. Measured production slice: **95.66%
(6170/6450)**, below the 96% floor.

On that same tree:

```
$ rm -f target/llvm-cov.json && make covers-check
covers-check EXIT=0
report ABSENT

$ make coverage
COVERAGE-PROD FAIL: production coverage 95.66% (6170/6450) < floor 96%
  (line hasCount/max-count rule); most uncovered production lines:
  src/ui/terminal.rs (57), src/cli.rs (40), src/ui/markdown.rs (28),
  src/ui/mod.rs (23), src/ui/app.rs (21)
make: *** [coverage] Error 1
EXIT=2
```

`covers-check` exits 0 and leaves no report, because line percentages are not its subject
and it reads none. The floor is enforced in exactly one place, and `covers-check` has not
become a second, weaker definition of it.
