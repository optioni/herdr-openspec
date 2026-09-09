## MODIFIED Requirements

### Requirement: `WIRED` reads code, and the panic hook is one of the names it requires

`WIRED` strips `//` line comments before searching, then looks for twelve required names as
plain substrings. Two consequences, both measured:

- A name surviving inside a `/* … */` block comment satisfies the gate. Replacing the real
  call with `/* crate::launch::start( is gone */ crate::launch::begin(` made the script print
  `OK: twelve names present` on a tree whose launcher was unwired — the gate that proves
  wiring reporting that unwired code is wired.
- `terminal::install_panic_hook()` is not among the twelve. It is called from exactly one
  line, in `ui::run`'s uncovered body, and appears nowhere else in `src/`, `tests/`, or
  `scripts/`. Deleting the call leaves every gate and every test green, and a production panic
  then leaves the terminal raw on the alternate screen.

`WIRED`'s `code()` SHALL strip block comments — including multi-line ones — as well as line
comments, and SHALL carry a control proving the stripper strips. `install_panic_hook` SHALL
join the required-name list, making it thirteen, with a positive control anchored on
`^pub fn install_panic_hook\(` in `src/ui/terminal.rs` so a rename fails in the file that
defines it rather than leaving the leg hunting a name nobody defines.

`mouse-input`'s `mouse_problem` SHALL **not** join that list, and the required-name count
SHALL stay thirteen. Leg 1 searches the whole production slice of `src/ui/mod.rs`, and
`pub struct Startup<'a>` is declared in that slice — so the field's own declaration would
satisfy a leg-1 name while `ui::run` had stopped passing a value, which is precisely the
failure leg 5 documents for `state::read` and answers by scoping to `$body`.

`WIRED` SHALL instead carry a **body-scoped leg 5c**, on leg 5's exact terms: `pub fn run()`'s
own body SHALL name `mouse_problem(`, and SHALL NOT hardcode `mouse_problem: None`. A `run`
that stopped threading the guard's reason would otherwise ship a refused-capture problem row
that can never appear, with every test green — every test constructs its own `Startup` and
drives `run_wired` directly. This is the defect class `live-refresh` shipped and
`gate-integrity` closed for the panic hook, arriving at the one kind of name leg 1 cannot
see.

The **behaviour** of the hook — that it must be inert off the render thread — is out of scope
here and belongs to `seam-resilience`. This requirement makes the wiring provable, nothing
more.

#### Scenario: A name hidden in a block comment no longer satisfies the gate

- **WHEN** the real `crate::launch::start(` call in `src/ui/mod.rs` is replaced with
  `/* crate::launch::start( is gone */ crate::launch::begin(`, and `make gates` is run
- **THEN** `WIRED` exits non-zero naming leg 1 and `launch::start`
- **AND** the same plant written as a multi-line `/* … */` spanning three lines also fails,
  so the stripper is not line-scoped
- **AND** restoring the call returns `make gates` to exit 0

#### Scenario: Deleting the panic-hook call fails the gate

- **WHEN** the single `terminal::install_panic_hook();` line is removed from `ui::run` and
  `make gates` is run
- **THEN** `WIRED` exits non-zero naming `install_panic_hook` as absent from the production
  slice of `src/ui/mod.rs`
- **AND** before this change the same deletion left `make check` entirely green, which is why
  the name is added to the list rather than left to review

#### Scenario: Hardcoding the mouse-capture reason in `run` fails the gate

- **WHEN** `mouse_problem: guard.mouse_problem(),` in `ui::run`'s `Startup` construction is
  replaced with `mouse_problem: None,` and `make gates` is run
- **THEN** `WIRED` exits non-zero on leg 5c, naming `mouse_problem: None` and that the
  refused-capture row would be dead in the shipped binary
- **AND** `cargo test --all-features` alone stays green against that same tree, since every
  test constructs its own `Startup` and drives `run_wired` directly — which is why a gate is
  the answer and review is not
- **AND** leg **1** stays green against that same tree, because `pub struct Startup`'s own
  field declaration satisfies a slice-wide name search; that is why this is leg 5c and not a
  fourteenth entry on leg 1

#### Scenario: A renamed definition fails in the defining file

- **WHEN** `pub fn install_panic_hook` in `src/ui/terminal.rs` is renamed and every caller
  updated
- **THEN** `WIRED` exits non-zero on its positive control, naming `src/ui/terminal.rs`,
  rather than on leg 1 naming a call site
- **AND** the stripper's own control fails when `code()` is edited to strip nothing, so a
  stripper reduced to the identity function cannot pass
