## ADDED Requirements

### Requirement: View behaviour is verified against a `TestBackend` buffer at both widths

Every test of a `ui` view SHALL render into a `ratatui::backend::TestBackend` buffer and
SHALL assert on the content of named cells — an exact character at an exact column and row,
an exact substring spanning named columns of a named row, or an exact `Style` on a named
cell. A test that renders and asserts only that no panic occurred, or that the buffer is
non-empty, does not satisfy this requirement: it stays green with the behaviour deleted.

A **view scenario**, for the purpose of this requirement, is one whose assertions are on a
`Buffer` produced by `ui::view::render`. A scenario that renders only to prove loop
sequencing — the `dashboard-loop` driver scenarios, whose subject is frame and poll
counting — is a unit scenario and is out of scope here.

Every view scenario SHALL be exercised at **both 60 and 120 columns**, one below the
100-column breakpoint and one above it. A single-width test does not cover the breakpoint,
and the breakpoint is the reason the layout is not a fixed split: a Herdr side pane is
frequently 40 to 60 columns wide. Where a scenario is specifically about a boundary or a
degenerate size — 1, 16, 18, 20, 99, 100, or 101 columns — that width is exercised **in
addition to**, never instead of, both of the two. There is no exemption list: a scenario
whose subject is inherently one side of the breakpoint still renders at the other side as
its contrasting control, which is the assertion that proves the behaviour is a width branch
rather than the feature being absent.

The harness SHALL live in `crate::testutil`, beside the crate's existing `ScratchDir` and
`snapshot` helpers, and SHALL offer at minimum: rendering a `Dashboard` at a given width
and height into a `Buffer`; reading a whole row as a `String`; and reading a named cell's
symbol and `Style`. It SHALL be `#[cfg(test)]` and SHALL add nothing to the shipped binary.

No view test SHALL touch the filesystem, the process environment, a subprocess, or a real
terminal. A view test that needs a real directory is the signal that logic leaked out of
the pure side of the render seam and into the view.

#### Scenario: The harness renders a state value with no repository on disk

- **WHEN** a view test builds a `Dashboard` value directly in memory — `repo: None`,
  `changes: changes::empty_set()` — and renders it at 60x20 and at 120x20
- **THEN** each buffer's row 0 spells `OpenSpec` in columns 0 through 7 and each buffer's
  last row begins `q quit` at column 0, so the scenario fails if rendering is deleted
- **AND** a `testutil::ScratchDir` the test creates for the purpose is byte-identical
  across both renders, compared with `testutil::snapshot` over that directory alone —
  **not** over `std::env::temp_dir()`, in which this crate's other tests create and destroy
  scratch directories concurrently under `cargo test`'s parallel threads
- **AND** the stronger claim, that no view file can perform I/O at all, is
  `dashboard-loop`'s `NOIO-VIEW` grep rather than a runtime observation

#### Scenario: Both widths are exercised for every view scenario

- **WHEN** the `ui` view tests are inventoried against `responsive-layout`'s scenarios
- **THEN** each scenario's test renders at 60 columns and at 120 columns
- **AND** a source check confirms that every `#[test]` in `src/ui/view.rs` names both
  literals, with a floor on the number of tests found so a gutted file fails rather than
  passing with nothing to check
- **AND** that check is a floor, not a proof: it cannot see whether an assertion is
  meaningful, and a `60` in a comment would satisfy it. The proof that the tests exist and
  run is a separate filtered run gated on a counted minimum, because `cargo test` exits 0
  when a filter matches nothing

#### Scenario: The coverage floor is unchanged by the new module

- **WHEN** `cargo llvm-cov --fail-under-lines 80` is run after `ui` lands
- **THEN** it passes at the same 80% floor, with no exclusion, no `#[coverage(off)]`, and
  no adjustment to the threshold
- **AND** the code that cannot be covered without a real terminal is confined to
  `ui::terminal::CrosstermOps`, `ui::terminal::install_panic_hook`, `ui::event::CrosstermEvents`,
  and the body of `ui::run` after its terminal check — each of them a wiring binding with
  no branch of its own, which is the reason for keeping them separate from the logic
- **AND** two further uncovered regions are expected and named rather than discovered:
  `src/main.rs`'s exit-1 arm, which no test can reach because `StartError::Terminal` and
  `StartError::Io` arise only from a real terminal, and the `Backend` methods a test double
  implements but `Terminal` never calls, since `cargo llvm-cov` instruments `#[cfg(test)]`
  code too
