# Planning review — settings-window

Reviewed against HEAD `e9e00b9`, working tree clean outside this change's own directory.

Four `planning-reviewer` subagents were dispatched simultaneously, one slice each, none of
them the session that wrote the package: **A** capability coverage, delta fidelity, scenario
quality, cross-artifact contradictions; **B** design completeness, test boundaries, and the
falsifiability audit; **C** task alignment, lifecycle discipline, ordering; **D** factual
verification of every empirical claim. They reported findings and edited nothing. This
session merged them, repaired the owning artifact, and wrote this log.

Two of them (A and D) were then sent back over the **repaired** files, which caught four more
defects the first round could not have seen — three introduced by the repairs themselves.

**Outcome: 14 CRITICAL, 22 WARNING, 8 SUGGESTION. All CRITICALs fixed. All WARNINGs fixed or
consciously accepted with a reason below. `openspec validate settings-window --strict` passes;
123 spec scenarios bind to 123 matrix rows in both directions.**

## The three findings that changed the design

### 1. The panel had no data path and no state home (A, B, and D, independently)

`Dashboard`'s sixteen fields hold no `Config`, no `BinResolution`, and no resolved agent
kind. The resolved kind is a stack local inside `worker_body` (`src/launch.rs:608`) with no
accessor; `Launcher` exposes only `request`/`drain`; `Outcome` was `{named, problems}`. And
`agent-client-choice` resolves the kind **lazily on the first `a`/`c`/`s`, never at startup**,
so `,` pressed on a fresh pane had no kind at all. `setting-provenance` claimed the
composition root "already reads" all three inputs. It reads none of them at panel-open time.

Neither obvious fix was available: resolving on panel open puts a blocking `integration
status` on the render path, and resolving at startup reverses a landed decision.

**Repaired** by specifying the carrier end to end: a seventeenth `Dashboard` field
`settings: settings::PanelState { rows, cursor }`; `Request::Resolve` out and
`Outcome::resolution` back (taking `Outcome` from two fields to four); `Provenance::Pending`
as a specified state for the frames before the worker answers. `ui::load` already receives the
`&Config` it ignores as `_config`, so half the plumbing existed. `design.md` gained a
Contracts subsection, "How the panel's three inputs reach a pure view", and Decision 13.

### 2. `Choice::Ambiguous` was unrepresentable — and it is the headline entry path (B)

`integration::Choice` is `Use { kind, source } | Ambiguous { installed }`, so `Source` exists
**only** on `Use` and the specified total `From<Source>` could never produce the ambiguous
state. `agent-launch` opens this panel on exactly that outcome, so the most common way in was
undefined: no artifact said what the `agent_kind` value row or source row read.

**Repaired**: `Provenance::Ambiguous` is its own variant, `Editable` is `Kind { shortlist }`
carrying both candidates — so the panel can resolve the very refusal that opened it — and two
scenarios were added.

### 3. Checks that could not fail (B, measured)

- **Three commands selected zero tests and exited 0.** `cargo test ui::run_wired` matches
  nothing (the real names are `ui::tests::wiring::run_wired_*`); tasks 2.5 and 9.5 combined
  `--test doc_contract` with filters matching nothing in that binary — and `--test` also
  excluded the lib target the task meant to run. B reproduced the zero-match exit-0 behaviour
  in a scratch crate.
- **Thirty-three commands were cargo hard errors.** `cargo test a b` → `error: unexpected
  argument`; `[TESTNAME]` takes one value. Rewritten as `cargo test -- a b`.
- **Task 10.1's "deterministic failing check" was green.** B rsync'd the tree, added a pure
  `src/ui/settings.rs`, and measured **26/26 gates passing**: both `PURE` lists are fixed
  literals, so an unlisted file is simply unswept. Restated as a negative control that plants
  the violation first.
- **`set_kind`'s non-blocking claim rested on `NOBLOCK`**, whose `BLOCK3_RE` names no `Mutex`,
  `RwLock`, or `lock()`, and whose lock leg is scoped to `src/ui/`. Green either way. Replaced
  with a FIFO-ordering scenario that deadlocks against an implementation holding the lock
  across `agent start`, plus a stated hold discipline (Decision 14).

### 4. The winning probe step has no reader, so `Provenance::Probe` had no data (D, second round)

`design.md` claimed the composition root "already holds the `BinResolution` it derived
`file_mode` from". It does not. `run_at` computes `resolve::openspec_bin(...)` and then
**moves** the resolution into `cli::worker_cli`, which takes it by value and keeps only
`found.path`. What survives is `resolved_bin: Option<PathBuf>` and `file_mode: bool`;
`found.source` — the `BinSource` this change wanted to render — has **zero production readers
anywhere in the crate**. The scenario "Every probe step is named by the step that won" was
unimplementable as the tasks stood.

**Repaired**: `run_at` retains the `FoundBin` before the move and passes it to `ui::load` as a
further parameter, with an explicit prohibition on probing twice. Specified in
`setting-provenance`, `dashboard-loop`, `design.md` → Contracts, and task 4.2.

This is the same class of error as finding 1 and was missed by the same reasoning: I checked
that a value was *computed* at startup without checking that it *survived*.

### 5. The repairs left three stale lists behind (A, second round)

The seventeen-fields block copies text from the live sixteen-fields requirement, and three
carried passages did not move with it:

- the `Default`-prohibition list and the scenario's search list still named `Help` while the
  positive control in the same block had been changed to `Overlay` — so every search leg would
  look for a name that no longer exists while the control passed, which is exactly the failure
  the anchoring paragraph says it prevents;
- the `NOIO-VIEW` enumeration still listed **ten** pure view files and omitted
  `src/ui/settings.rs`, contradicting this change's own `responsive-layout` delta and
  `design.md`, both of which say ten → eleven;
- a carried sentence still argued that "this requirement pins `Dashboard` at sixteen fields, so
  a dedicated field is not available", refuted by the header of the block containing it.

All three repaired, and the type count moved from thirteen to seventeen with it.

## Repairs by artifact

| Artifact | Repairs |
|---|---|
| `proposal.md` | Impact named `src/config.rs` for provenance (refuted by its own Capabilities note) → `src/settings.rs`, plus the omitted `src/launch.rs` and `src/ui/driver.rs`; `quality-gates` and `doc-conformance` added as modified capabilities; a stale "chosen as two" clause removed |
| `specs/setting-provenance/` | `resolve::Step`/`resolve::Resolved` do not exist → `BinSource`/`BinResolution`; `Pending` and `Ambiguous` added; the `NODEFAULT-UI` subject corrected to the module's **structs** only |
| `specs/settings-window/` | a requirement added for the panel's **inert action set** (nothing specified it — `a` could have launched an agent from inside the panel); the commit's write site moved out of `apply` explicitly; the affordance named as ASCII `<`/`>`; the band-scroll scenario given concrete 120x8 and 60x8 frames; the stale `src/config.rs` sentence fixed |
| `specs/dashboard-loop/` | `Dashboard` sixteen → **seventeen** via REMOVED+ADDED (a header count cannot move in a MODIFIED block); a `ToggleSettings` dispatch bullet added; `Back`'s layer order extended with the edit layer; eleven `help.open`/`help.scroll` references renamed; the `NODEFAULT-UI` paragraph corrected from six runs / `SCAN_MIN` 206→308 to **eight** runs / 333 with `Selection`; the positive control moved from `struct Help` to `struct Overlay` |
| `specs/mouse-input/` | eight stale field references; "index among the four" → three; the precedence quote restored to the live header (`…or arms a selection`); the wheel specified as `Ignore` during an edit |
| `specs/help-overlay/` | the inert enumeration listed **seventeen** names while claiming eighteen — `Select` was missing since `text-selection`; repaired rather than reproduced |
| `specs/responsive-layout/` | "four settings" and `content_rows` 11 recomputed for three settings (band height 9, `y` 15 at 120x40 and `y` 5 at 60x20, verified against the formula) |
| `specs/agent-launch/` | `Request::Resolve`, `Outcome` at four fields, the lock's hold discipline, the falsifiable `set_kind` scenario, a resolve-once scenario |
| `specs/plugin-state/` | `record_kind(None, kind)` specified |
| `specs/quality-gates/` | **new delta** — thirty-one → thirty-two |
| `specs/doc-conformance/` | **new delta** — the sixteenth claim and its four bound sites |
| `design.md` | the Contracts subsection; Decisions 13 and 14; the config-directory row in Test Boundaries; every command corrected; matrix regenerated at 122 rows |
| `tasks.md` | commands corrected; the `pub mod settings;` doc sites (three machine-bound); `NODEFAULT-UI`'s `TYPES` and the `Makefile:46` rename; the `quality-gates` figure moved to the delta; a persistence gate in group 7; the `AGENTS.md` sites enumerated to **nine**; a task for the one uncovered scenario |

Second round also corrected: `NoLauncher` is a **second production implementation** (what file
mode gets), not a test double, and there are **five** `impl Launcher` sites across **two**
files — none in `src/ui/mod.rs`, where tasks 7.3/7.4 had placed two of them. `set_kind` and
`Request::Resolve` are now specified as inert there, with a file-mode scenario.

## Two claims the reviewers got wrong

- **C counted 15 bullets under `SPEC.md` → Doc-conformance checks; there are 16.** The
  sixteenth is a meta-statement that must stay last, so the new claim inserts above it. Task
  13.6 and the `doc-conformance` delta both say so.
- **C reported task 13.5 should correct "the `Action` count" in a maintained document.** It
  then verified that no such prose site exists (`grep -n 'Action\b' AGENTS.md` → nothing); the
  count is pinned only by `tests/doc_contract.rs`. Removed from 13.5 rather than sent to an
  implementer to hunt for.

## Accepted, not fixed

- **`cargo test settings::` also selects `ui::settings::` tests by substring.** The matrix's
  collaborator column says so rather than contriving an exact-match filter; the extra tests are
  a superset and cannot mask a failure.
- **The four earlier-generation reviewer agents visible in this session's agent list**
  (`reviewA-coverage` … `reviewD-facts`, 16h old) belong to a different change and were not
  consulted.

## Ordering

Unchanged and sequential. The project rules record a standing parallelism veto for this
repository — one crate, one compile, and `make gates`/`make lint`/the whole `--lib` suite sweep
the entire tree, so criterion 3 fails for every pair in every change here. C confirmed the veto
is genuinely recorded, so it is cited rather than re-walked. No group carries `parallel-after`.

## Verification

- `openspec validate settings-window --strict` → **valid**.
- 123 spec scenarios ↔ 123 matrix rows, names matching in both directions (set comparison, not
  a count).
- `cargo test --lib` at HEAD → **1562 passed, 0 failed**, the baseline group 14 compares against.
- `scripts/gates/noio-view.sh` negative control reproduced: exit 0 → planted `use std::fs;` →
  exit 1 naming `src/ui/app.rs:1` → removed → exit 0. Tree restored clean.

## Open questions

None. The proposal's five are resolved and recorded there; the two that arose while writing
the specs — `archived_count`'s inertness and the closed shortlist — were decided by the author
before design.md was written, and both carry their cost in writing.
