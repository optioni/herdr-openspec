# Group 10 measurements

## 10.2 — the three recorded plants

Each applied to a clean tree, the acceptance test run, the plant reverted. Every one
fails a **distinct** assertion set, which is what makes them evidence rather than three
ways of being red.

| Plant | First assertion that fires | Other scenarios |
|---|---|---|
| `launch::none()` in place of `launch::start` | the log: `0` non-`agent list` entries against `4` | — |
| `run_loop` never hands `launch.pending` over | `launch.pending` is `Some(Launch { change: "2fa-support", agent: "c-2fa-support", intent: Apply })` against `None` | — |
| `Settings::openspec_bin` hardcoded to `None` | the log: `0` entries against `4` — **no `integration status` and no `pane split`** | `file_mode_leaves_a_refusing_and_g_working_in_the_shipped_root` stays **green** |

The third is the one the design predicted precisely: `Collaborators::file_mode` comes from
`cli.is_none()` at the composition root, **not** from `Settings`, so `decide` still returns
`Go` under that plant and the request reaches the worker. The worker's own refusal of a
`Request::Launch` carrying `openspec_bin` `None` (per `agent-prompts`) is what stops it
before the status read, which is why both calls are missing rather than only the prompt
being wrong. The disagreement between the two runs — one red, one green — is the guard,
not a presence check.

**Repair made while measuring**: the first two plants originally failed the *same* first
assertion, because the call-log assertion preceded the `launch.pending` one. The
acceptance test now asserts `launch.pending` **first**, so a `run_loop` that never hands
the request over fails there while `launch::none()` (which takes the request and discards
it) passes it and fails on the log.

## 10.3 — wall time against `testutil::Stages`' shared deadline

`Stages::DEADLINE` is **30 s**, shared across *every* stage of one run
(`src/lib.rs`). This change takes "Pressing `a`" to four non-`agent list`
entries and "Pressing `g`" to five under that unchanged budget, so the margin is what
needed measuring.

| Test | Both widths | Per scenario (120x20 or 60x20) | Margin against 30 s |
|---|---|---|---|
| `a_keypress_launches_an_agent` | 3.17 s | ~1.6 s | ~28.4 s (≈19x headroom) |
| `g_focuses_the_agent_the_launch_started` | 4.93 s | ~2.5 s | ~27.5 s (≈12x headroom) |

Reference point from the planning package: `ui::tests::wiring` measured **43.4 s for 29
tests** at HEAD. It now runs 35 tests; `cargo test --lib ui::tests::` measured **55.6 s for
68 tests** after group 9.

The fourth and fifth entries cost roughly one process round trip each and the budget is
two orders of magnitude above that, so `SETTLE_BUDGET` and `DEADLINE` both stand
unchanged. Recorded rather than assumed, per design.md -> Risks.
