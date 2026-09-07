## Why

This project is built spec-first, and its own precedence rule is that `SPEC.md` wins
where prose disagrees — so a stale `SPEC.md` is not cosmetic, it is a wrong contract
that later changes are built against. An audit found seven findings across eight
passages where the documents no longer describe the code: a module missing from the map a
reader uses to find modules, a worker-thread count that says two when there are three,
**four** modules absent from the tested-modules list (the audit named two; running the
proposed check's own rule found `config` and `state` as well), and — worst — a false claim
inside
`openspec/config.yaml`'s `context` block, which is injected verbatim into every
OpenSpec agent's prompt, so that one error propagates into every future change.

## What Changes

- Correct the eight drifted passages in `SPEC.md`, `openspec/config.yaml`, `README.md`,
  `AGENTS.md`, and `HANDOFF.md` (enumerated in design.md → Findings; seven audit findings,
  one of which — D5 — covers two distinct passages).
- Add a **doc-conformance test tier**: `tests/doc_contract.rs`, run by `cargo test` and
  therefore by `make check`, binding each doc claim that has a *second site* to the
  site that determines its truth — modules to `src/lib.rs`, the worker-thread count to
  the production `thread::spawn` sites, the MSRV to `Cargo.toml`'s `rust-version`, the
  documented toolchain to the programs `make check` actually invokes, `SPEC.md`'s
  transcription of the manifest to `herdr-plugin.toml`, and `openspec/config.yaml`'s
  injected context to the repository it describes. This is the same shape as the
  existing `tests/manifest.rs` and `tests/degraded_coverage.rs`.
- Reduce `HANDOFF.md` to a closed historical record: delete its residual open-work
  list (all three items landed), correct its stale environment claims, and state at the
  top that open work lives in OpenSpec changes, not in this file.
- Not **BREAKING**: no manifest value, config-format key, or keybinding changes.
  `SPEC.md`'s manifest transcription is reordered to match the file, which is a
  documentation edit, not a manifest edit.

## Non-Goals

- **No behaviour change.** No file under `src/` is edited; the binary is unchanged.
- **No new gate script.** The mechanism is a Rust test, not a `scripts/gates/` entry —
  `make gates`' recipe and `tests/ci_workflow.rs`'s recipe/directory correspondence are
  untouched.
- **Not a general docs rewrite.** Only the seven audited claims and the mechanism.
- **Not the siblings' passages.** Eight other changes are in flight against these same
  documents; the passages explicitly assigned elsewhere — `SPEC.md`'s § Gates gate count
  and table (`gate-integrity`), `AGENTS.md`'s "joined by position" sentence and the
  glob-symlink degraded row (`cli-parity`), and `SPEC.md:263`'s watch-scope statement
  (`seam-resilience`) — are left to their owning changes. See design.md → Boundaries.
- Crosses no PRD non-goal: no OpenSpec file is written outside this change's own
  directory (`openspec/config.yaml` excepted, and argued in design.md), no
  orchestration, no change authoring, no Windows.

## Capabilities

### New Capabilities
- `doc-conformance`: `make check` fails when a documented claim that has a computable
  second site disagrees with that site.

### Modified Capabilities

None. The seven corrections are prose; the only new observable behaviour is the
`doc-conformance` capability above.

## Impact

- **Roadmap:** unplanned work. `openspec/IMPLEMENTATION-ORDER.md` ends at Phase 6 and
  the roadmap is complete; it never anticipated this because doc accuracy was governed
  by a per-change rule ("a doc claim is fixed by the change that ships the behaviour"),
  which `HANDOFF.md` itself records as having a blind spot at the end of a roadmap.
  This change replaces that rule with a mechanism for the claims that can carry one.
- **Code:** one new file, `tests/doc_contract.rs`. Nothing under `src/`.
- **Docs:** `SPEC.md`, `README.md`, `AGENTS.md`, `HANDOFF.md`, `openspec/config.yaml`.
- **Agents:** correcting `openspec/config.yaml`'s `context` changes the prompt every
  future OpenSpec agent receives in this repository.
- **No external service, dependency, data model, or manifest value changes.**
