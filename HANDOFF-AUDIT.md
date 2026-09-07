# Handoff — audit remediation proposals

**Written:** 2026-09-07, at the 5h session limit (11% left, reset 12:59pm).
**Commit:** `994a6d1` — five proposals, WIP. No `src/` changes. Nothing implemented.

This is a *separate* file from `HANDOFF.md` on purpose: that one is a historical
record of the original build, it is stale (audit finding D8), and the
`doc-conformance` change owns the decision about its fate. Do not merge these.

---

## Where things stand

A five-track parallel audit of the plugin produced **40 findings**, published at
https://claude.ai/code/artifact/beec2a4f-da5e-4723-8af7-64003f601727 — read that
first, it is the source for every proposal below and carries the full evidence.

`make check` was green at audit time (all five gates, 96.71% reported coverage).
The defects cluster in what was never exercised: other people's repositories,
non-ASCII input, and the gates that guard the gates.

Five change proposals were drafted in parallel, one per domain. **None has been
reviewed or approved. None may be implemented until it is.**

## Per-change state

| Change | Findings | State |
|---|---|---|
| `gate-integrity` | G1–G8, D1–D3 | **Complete**, `Change 'gate-integrity' is valid` |
| `seam-resilience` | S1–S9, U3, S10 | **Complete**, `Change 'seam-resilience' is valid` |
| `view-fidelity` | U1, U2, U4, U5 | Drafting in progress — no `planning-review.md` yet |
| `cli-parity` | C1–C8 | **Complete**, `Change 'cli-parity' is valid` |
| `doc-conformance` | D4–D10 | **Complete**, `Change 'doc-conformance' is valid` |

Run `openspec validate --strict` on each before treating any as ready.
`seam-resilience`, `cli-parity` and `doc-conformance` are confirmed passing.
Two remain unfinished: **`gate-integrity`** and **`view-fidelity`**.

### What the two finished ones decided

`cli-parity` closes C1, C2, C3, C5, C7, C8 and documents C4 and C6 as
degraded-states rows (`MIN_ROWS` 44 → 46). `doc-conformance` adds one new
capability of the same name, whose anti-drift mechanism is `tests/doc_contract.rs`
— an ordinary `cargo test` target (so it runs inside `make check`) with six legs,
each binding a documented claim to a computable second site. It recommends keeping
`HANDOFF.md`, deleting its open-work sections and marking it closed, because it holds
the only copy of several transferable findings.

`openspec` is not on a non-login `PATH` here — it is at
`~/.nvm/versions/node/v24.18.0/bin/openspec` (v1.12.0). Source nvm first.

## S10 — RESOLVED, recorded for review

`seam-resilience`'s S10 revision **was applied and validates**. Nothing is owed here;
this section is kept because the reviewer should understand what changed and why.

**The defect (S10), measured and reproduced, not hypothesised:**

1. `~/.nvm/.../bin/openspec` is a symlink to `openspec.js`, whose first line is
   `#!/usr/bin/env node`.
2. `env -i PATH=/usr/bin:/bin <nvmbin>/openspec list --json` gives exactly
   `exit=127`, `stdout=[]`, `stderr=[env: node: No such file or directory]`.
3. Reachability is **structural**: `openspec`'s symlink sits in the *same* nvm bin
   directory as `node`. Whenever that directory is off `PATH`, `resolve::step2_path`
   misses **and** `step3_nvm` succeeds — the probe chain lands on the shim precisely
   in the condition that guarantees the child cannot exec. `src/cli.rs:75` spawns
   with no `.env()`, inheriting that node-less `PATH`. AGENTS.md itself notes nvm's
   bin "is not always on the `PATH` a non-login shell inherits", which is the normal
   case for a GUI-launched Herdr.

**Why it matters:** S7's premise is "the root guard discards the payload" — that
assumes the child *runs*. Where the probe reached step 3 or 4 there is no payload
at all, and setting `current_dir` fixes nothing. Same symptom, earlier layer.

**Three things the revision must do:**

- (a) Add the exec failure as its **own** requirement, not folded into S7.
- (b) Extend task group 1's measurement to record a **real spawn's exit code and
  stderr**, not only `current_dir()` and `startup_cwd`. Without it a 127 in the
  field reads as "S7's fix didn't work".
- (c) Resolve a live spec collision. `specs/subprocess-seam/spec.md:7-8` re-affirms
  "SHALL NOT ... **alter or clear the environment**" — which a `PATH` fix violates by
  name. Worse, `:35-37` mandates using "the path the probe chain constructed and
  never its canonicalized target", i.e. the shim, by requirement. Suggested shape,
  mirroring the `current_dir` relaxation already in the same paragraph:
  `RealOpenspecCli` SHALL additionally be constructible with an environment overlay;
  SHALL set exactly what it was given and nothing else; SHALL NOT derive, validate or
  extend it; when none was given it SHALL set none. The caller decides, so the seam
  stays decision-free and `RealHerdrCli` keeps both prohibitions. A different fix is
  acceptable *with an argument*; leaving the clause re-affirmed while the defect
  stands is not.

`cli_error_problem` is **not** part of this — `cli-parity` owns carrying stderr on
the `Failed` arm (the child *did* start, so this is `CliError::Failed`, not
`NotStarted`).

**How it was resolved.** The agent reproduced all three facts, then measured the
one data point the brief lacked: `env -i PATH=<nvmbin>:/usr/bin:/bin <nvmbin>/openspec
--version` → `exit=0`, `1.12.0`. That is what let it choose a mechanism instead of
arguing three in the abstract. It took the environment overlay — `RealOpenspecCli`
sets exactly the variables it is handed, clears nothing, derives nothing — and both
relaxations now sit in one paragraph under one shared rationale, since `current_dir`
assumed the two directories coincide and the environment clause assumed an executable
is self-contained, and measurement falsified both. The blanket line now reads "SHALL
NOT add an argument of its own, **clear** the environment, retry, cache, or inspect
the output." Two alternatives were argued down in `design.md` → Decision 2: a resolved
interpreter path (needs a second probe chain for `node`, which the seam refuses to
build even for `herdr`, plus shebang parsing, and picks the wrong `node` when a
different one resolves) and a self-contained binary (none exists — the npm package
ships a JS entry with an `env node` shebang and nothing else). The rule is one `PATH`
entry, the resolved binary's parent prepended to the inherited value, applied for
every probe step so neither the seam nor the composition root need know which won.

S10 is its own requirement in `refresh-worker`, and task group 1 now classifies:
a 127 with `env: node:` is S10 and not S7; a payload plus a root-disagreement row is
S7; both can be true, and it records which. 93 scenarios, up from 86.

## Facts already verified — do not re-derive

- `changes::merge` joins changes **by name**, not by position (AGENTS.md says
  "joined by position" and is wrong — that is finding C8, owned by `cli-parity`).
- `openspec list` has **no root flag** (v1.12.0: only `--specs`, `--changes`,
  `--sort`, `--json`, `--store <id>`; a store is a *registered* repo, not a path).
  So the child's working directory is the only lever that exists.
- `unicode-width` is **not** being added. `view-fidelity` measures through ratatui's
  own public API instead — `ratatui::text::Span::styled_graphemes` +
  `ratatui::buffer::CellWidth`, both confirmed public in 0.30.2 and re-exported at
  `ratatui/src/lib.rs:480` and `:517`. `set_stringn` itself calls `symbol.cell_width()`
  (`ratatui-core-0.1.2/src/buffer/buffer.rs:352`), so "measured the way the buffer
  measures" is literal. Dependency count stays at six.
- **CORRECTED — the 80% coverage floor CAN fail.** The audit claimed production
  coverage could reach zero with the gate still green. Wrong twice over: it cut each
  file at its *first* line-anchored `#[cfg(test)]` (misclassifying ~6,281 production
  lines as test — `src/changes.rs` has three such attributes, the first at line 77 on
  a `mod conformance` test helper, the real `mod tests` at 1818), and it used raw file
  lines as a proxy for `llvm-cov`'s counted lines, which are only executable regions.
  Under correct brace extents, production-at-zero reports 66.65% and the floor fires.
  **The real finding: the floor does not fire until production coverage falls below
  43.68%**, tolerating 56% of production uncovered. Still serious, still justifies
  `gate-integrity`; the headline claim did not survive. The published audit artifact
  has been corrected in place at the same URL.

## Corrections the drafting agents made to the audit itself

Three findings were wrong or understated as briefed. The proposals carry the
corrected versions; the audit artifact does not.

- **C4 was inverted.** Measured against openspec 1.12.0 in scratch repos: for a
  symlink resolving *outside* the change directory the CLI does **not** over-list —
  it fails closed (`Path is outside the allowed directory`, surfacing as an empty
  `list --json`). The divergence exists only for a link resolving *inside* the
  change directory. `cli-parity`'s spec, row and `1/5` constant are written around
  that narrower case, with the outside case given its own scenario stating it is
  explicitly not the row's subject.
- **D5b was understated.** The audit named `launch` and `open` as absent from
  `SPEC.md`'s § Unit-tested modules. Running the check's rule at HEAD reports
  **four** — `config` and `state` are missing too.
- **D7's python3 count was low.** Ten gate scripts invoke `python3`, not seven —
  `taskseam.sh`, `taskwidths.sh` and `widths.sh` also do.

Also note `cli-parity`'s C1 **reverses a divergence a previous change stated
deliberately**: `schema-artifacts` currently argues the id-fallback is more useful.
That argument ignored the count — the fallback selects a possibly-glob `generates`
and sums files the CLI never counts, which is what the dual-source model forbids.
Worth a second opinion at review time, since it overrides a recorded decision.

## Recommended order

**`gate-integrity` first.** Every other finding is one the gates were supposed to
catch, so the rest should land on gates that work.

## Coordination — a peer session is active

A second Claude session (`herdr-openspec-ea`) is scoping post-audit refinements:
mouse support, colour, collapsible list sections, markdown constructs, and demoting
the CLI-failure problem row into the header badge slot. It has agreed to **hold until
these five land**. It contributed S10 and the C3 `Failed`-arm correction. It has made
no edits; the tree is otherwise clean.

## Resume prompt

> Read `HANDOFF-AUDIT.md` in this repo and continue from there. First check whether
> `seam-resilience`'s outstanding S10 revision was applied; if not, apply it. Then
> finish and validate the remaining proposals (`gate-integrity`, `view-fidelity`,
> `cli-parity`, `doc-conformance`) with `openspec validate --strict`, and report
> which are ready for review. Do not implement anything.
