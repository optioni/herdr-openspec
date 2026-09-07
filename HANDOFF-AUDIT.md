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
| `gate-integrity` | G1–G8, D1–D3 | Drafting in progress — no `planning-review.md` yet |
| `seam-resilience` | S1–S9, U3, **S10** | Complete + validates, **but has an outstanding revision — see below** |
| `view-fidelity` | U1, U2, U4, U5 | Drafting in progress — no `planning-review.md` yet |
| `cli-parity` | C1–C8 | Drafting in progress — no `planning-review.md` yet |
| `doc-conformance` | D4–D10 | `planning-review.md` present; completion unconfirmed |

Run `openspec validate --strict` on each before treating any as ready. Only
`seam-resilience` is confirmed to have passed (`Change 'seam-resilience' is valid`).

`openspec` is not on a non-login `PATH` here — it is at
`~/.nvm/versions/node/v24.18.0/bin/openspec` (v1.12.0). Source nvm first.

## THE ONE OUTSTANDING ACTION

`seam-resilience` was sent back for a revision that had **not** been applied when
the session ended. The agent may or may not have completed it — check for the
strings `127`, `shim`, `shebang` or `overlay` in
`openspec/changes/seam-resilience/`; if they are absent, the revision is still owed.

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
- The 80% coverage floor cannot fail: 33,101 of 40,500 lines in `src/` are inside
  `#[cfg(test)]` modules and ~96% covered, so test code alone clears 83%.

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
