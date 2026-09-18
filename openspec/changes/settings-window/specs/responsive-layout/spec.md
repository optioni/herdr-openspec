## REMOVED Requirements

### Requirement: The help overlay is a full-body-width band the layout computes

**Reason**: The requirement's subject is no longer the help overlay in particular. This change
gives the layer a second panel and renames `layout::help_band` to `layout::overlay_band`, which
both panels use; a header naming one of the two would leave the live spec describing a function
that no longer exists under a name that covers half its callers.

**Migration**: Replaced by the ADDED requirement below, which carries every sentence and every
scenario forward with `help_band` renamed to `overlay_band` and the pure-file counts raised for
`src/ui/settings.rs`. The geometry itself is unchanged — same `x`, `width`, `height`, and `y`,
same totality guarantees, same breakpoint independence — so `src/ui/help.rs` needs no edit
beyond the call's new name.

## ADDED Requirements

### Requirement: The overlay is a full-body-width band the layout computes

`ui::layout` SHALL carry one function, and it SHALL be the only place **either** overlay
panel's geometry is decided. `help-overlay` introduced it as `help_band`; this change renames
it to `overlay_band` and adds no second geometry, because a settings panel with a band of its
own would be two rectangles to keep centred, two sets of degenerate-frame tests, and two
places for the footer-row rule to be got wrong:

```rust
pub fn overlay_band(body: Rect, content_rows: usize) -> Rect;
```

It SHALL be pure and total: no I/O, no clock, no panic for any `Rect` including a
zero-sized one and one at `u16::MAX`, and no arithmetic that can overflow or underflow —
every subtraction saturating, on exactly the terms `interior` and `scroll_offset` already
hold to.

It SHALL return a rectangle whose `x` and `width` are the **body's own**, whose `height`
is `min(content_rows + 2, body.height)`, and whose `y` is
`body.y + (body.height - height) / 2`. The band therefore runs the full width of the body,
is vertically centred in it, and gives any odd remaining row to the space below it.

Both panels SHALL use it, and `content_rows` SHALL be the only argument that differs between
them: the help panel passes the rows its inventory grammar produces, the settings panel the
rows `setting-provenance`' three settings produce. A panel SHALL NOT adjust `x`, `width`, or
`y` for itself.

The overlay SHALL NOT participate in the 100-column breakpoint. `split_body` decides one
region or two and the overlay covers whichever it produced; there is no wide form and no
narrow form of the band, and `LayoutMode` is not consulted. That is a deliberate
simplification with a stated cost: at 120 columns the band's descriptions sit in a single
column with the right half of the row empty, where a two-column reflow would have halved
its height. Single-column is chosen because the band is scrollable — `help-overlay`
requires it — so height is not the constraint the reflow would have relieved, and a
breakpoint-dependent overlay would need its own geometry tests at both widths for a
layout no other requirement in this capability asks for.

The band SHALL cover the **body only**. `ui::view::render` splits the frame into a body
and a footer row, and the overlay is drawn into the body after the regions are; the footer
row SHALL render its hints unchanged while the overlay is open, so `? help` and `q quit`
are both visible from inside it.

`ui::layout::columns` and `ui::layout::truncate_columns` SHALL remain the crate's only
display-width measure, and `src/ui/help.rs` and `src/ui/settings.rs` SHALL use them for their
key columns and for every truncation. `COLWIDTH`'s `PURE` list SHALL gain
`src/ui/settings.rs`, taking it from nine files to **ten**, and `NOIO-VIEW`'s from ten to
**eleven**. `src/settings.rs` — the pure module `setting-provenance` adds — sits outside
`src/ui/` and moves neither count, on exactly `src/specs.rs`' and `src/integration.rs`' terms;
the cost of that placement is that no `make gates` script sweeps it, so its freedom from I/O
is a `tests/doc_contract.rs` claim over its production slice instead.

`ui::help`'s own both-widths rule SHALL be **mechanized**, not merely mandated. Every other
view module in the crate has a width gate of its own — `detailwidths.sh`, `listwidths.sh`,
`mdwidths.sh`, `taskwidths.sh`, and `widths.sh` (hard-coded to `src/ui/view.rs`) at
`Makefile:39,40,42,64,66` — and without one, `ui::help` would be the only module carrying a
"60 and 120, both widths, every time" mandate with nothing counting whether it is kept. A
`scripts/gates/helpwidths.sh` SHALL be added on `detailwidths.sh`'s **script** pattern — the
same `#[test]`-splitting scan, the same doc-comment stripping, the same unsuffixed-literal
limit — composed into the `gates:` recipe, its floor measured when the module's tests are
written rather than guessed, and bound to its own planted defect in `tests/gate-controls.toml`
like every other gate.

It SHALL be a **count against a floor**, and SHALL NOT require every `#[test]` in the file to
name both widths the way `detailwidths.sh` requires it of `src/ui/detail.rs`. That gate can be
exemption-free because every public function in `src/ui/detail.rs` is parameterised by a width;
`src/ui/help.rs` is not that module. Four of its tests are about `INVENTORY` as **data** — its
group count, its scopes, its two `Space` rows, its `const`-evaluability — and name no width
because there is no width in what they assert; `The grammar renders at 120 columns` and
`The grammar renders at 60 columns` are a deliberate pair, one width each, because the spec
states them as two scenarios; and `The reader is never trapped in a degenerate frame` drives
`apply` and renders nothing at all. Requiring all twelve to name both widths would mean either
deleting those assertions or padding them with a width they do not use, and a width written
into a test that does not measure it is precisely the rubber stamp `detailwidths.sh`'s own
comment warns an exemption list becomes.

The floor SHALL therefore be the measured number of `src/ui/help.rs` tests that assert at both
60 and 120, and the gate SHALL also fail when the file is missing or holds no `#[test]` at all
— the vacuity leg every other width gate carries.

#### Scenario: The overlay's both-widths rule is counted, not just stated

- **WHEN** `scripts/gates/helpwidths.sh` is run against the tree at the end of this change
- **THEN** it exits zero and reports the number of `src/ui/help.rs` tests asserting at both 60
  and 120 columns, against a floor measured from that tree
- **AND** it exits non-zero against a copy in which a test that asserted at both widths is
  **narrowed** to 120 alone — which is what drops the count below the floor — and against one
  in which the module's tests are removed, the vacuity leg every other width gate carries.
  Narrowed rather than *added*: this gate counts the tests that assert at both widths against a
  floor, so a new single-width test raises the total without lowering that count and is not the
  defect the floor exists to catch
- **AND** `tests/gate-controls.toml` binds it to a planted defect, so a `helpwidths.sh` neutered
  to `exit 0` fails `cargo test` rather than passing `make gates` quietly

#### Scenario: The band's rectangle at both mandated widths

- **WHEN** `overlay_band` is called with the body a 120x40 frame produces — `x` 0, `y` 0,
  `width` 120, `height` 39 — and `content_rows` of 42
- **THEN** it returns `x` 0, `width` 120, `height` 39, and `y` 0: the content needs 44
  rows and the body holds 39, so the band fills it
- **AND** with the body a 60x20 frame produces — `height` 19 — and the same
  `content_rows`, it returns `x` 0, `width` 60, `height` 19, and `y` 0
- **AND** with a 120x60 frame's body — `height` 59 — and the same `content_rows`, it
  returns `height` 44 and `y` 7, so the band is centred with the odd row below it

#### Scenario: The band is total over degenerate and extreme rectangles

- **WHEN** `overlay_band` is called with a zero-width body, a zero-height body, a 1x1 body, a
  120x1 body, a 120x2 body, a body at `u16::MAX` width and height, and `content_rows` of
  `0`, `1`, `42`, and `usize::MAX` against each
- **THEN** no call panics, overflows, or underflows
- **AND** every returned rectangle lies entirely inside the body it was given: its `x` and
  `y` are at least the body's, and its right and bottom edges do not exceed the body's
- **AND** a zero-height body returns a zero-height rectangle, which `ui::help::render`
  answers by drawing nothing

#### Scenario: The overlay does not move the breakpoint

- **WHEN** a dashboard with `overlay.panel` `Some(Panel::Help)` is rendered at 120x40 and at 60x20, and
  `split_body` is called for each
- **THEN** `split_body` returns the two-region result at 120 and the one-region result at
  60, byte-identical to what it returns for the same dashboard with `overlay.panel` `None`
- **AND** the band's `x` and `width` equal the body's at both, so the overlay spans both
  regions and the divider column at 120 rather than sitting inside one of them

#### Scenario: Both panels are centred by the one function

- **WHEN** `overlay_band` is called with a 120x40 frame's body — `x` 0, `y` 0, `width` 120,
  `height` 39 — and `content_rows` of `42` (a scenario-local figure exceeding the body's own
  height, to prove the overflow leg below — not the help panel's own inventory count, which
  is `50`), and again with
  `content_rows` of `7` (the settings panel's heading row plus its **three** settings' six
  rows)
- **THEN** the first returns `height` 39 and `y` 0, filling the body it cannot fit in
- **AND** the second returns `height` 9 and `y` 15, centred with the odd row below it
- **AND** both return `x` 0 and `width` 120, so neither panel narrows the band for itself
- **AND** at 60x20, where the body's `height` is 19, the same two calls return `height` 19 /
  `y` 0 and `height` 9 / `y` 5

#### Scenario: The settings panel gets its own both-widths gate

- **WHEN** `scripts/gates/settingswidths.sh` is run against the tree at the end of this change
- **THEN** it exits zero and reports the number of `src/ui/settings.rs` tests asserting at both
  60 and 120 columns, against a floor measured from that tree
- **AND** it exits non-zero against a copy in which a test asserting at both widths is narrowed
  to 120 alone, and against one in which the module's tests are removed
- **AND** `tests/gate-controls.toml` binds it to a planted defect, so a `settingswidths.sh`
  neutered to `exit 0` fails `cargo test` rather than passing `make gates` quietly
- **AND** `scripts/gates/` holds one more file than before, and both
  `openspec/specs/quality-gates/spec.md`'s stated count and `tests/ci_workflow.rs`' equality
  against it are raised by one in the same change

