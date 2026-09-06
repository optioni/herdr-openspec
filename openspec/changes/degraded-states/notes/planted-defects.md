# Planted defects

Each entry: the gate, the plant, the red output, and confirmation the plant was reverted
(`git diff --name-only` / `git status --porcelain` afterwards, or a direct `diff` against the
pre-plant backup).

## Group 2 — `scripts/gates/wired.sh`

**Plant 1 (leg 2 negative control, task 2.5):** reinstated the `match` in `pub fn run()`:

```rust
let cwd = match startup_cwd(&crate::config::env_lookup()) {
    Some(cwd) => cwd,
    None => std::env::current_dir()?,
};
```

Red:

```
WIRED FAIL (leg 2): 'pub fn run()' holds a branch or a loop:
5:    let cwd = match startup_cwd(&crate::config::env_lookup()) {
everything with a decision in it belongs in run_wired, which is tested
```

Reverted; `sh scripts/gates/wired.sh` green afterwards.

**Plant 2 (leg 5b negative control, task 2.6):** deleted the `startup_dir(` call and inlined
`std::env::current_dir()?` directly (no branch, no loop — leg 2 alone would pass):

```rust
let cwd = std::env::current_dir()?;
```

Red:

```
WIRED FAIL: leg 5b: 'pub fn run()' does not name startup_dir( - the cwd-resolution decision may have been reinlined
```

Reverted; `diff` against the pre-plant backup of `src/ui/mod.rs` reports no difference.

## Group 3 — `scripts/gates/wired.sh`'s leg 1, extended

**Plant 3 (leg 1's new `npm_probe_hook` name, task 3.7):** replaced `run()`'s
`npm_hook: &crate::cli::npm_probe_hook,` with `npm_hook: &|| None,` — a body that still
compiles, still passes every test, and still names no branch or loop (leg 2 stays green),
but silently drops the real fourth-step binding.

Red:

```
WIRED FAIL: leg 1: src/ui/mod.rs's production slice does not name npm_probe_hook - the name
is gone; leg 1 sees names, not values, so a call whose result is dropped still passes here
and is the acceptance test's job
```

Reverted; `diff` against the pre-plant backup of `src/ui/mod.rs` reports no difference,
`sh scripts/gates/wired.sh` green afterwards. (`config::env_lookup(` was not planted
separately — it is unconditionally present via `startup_dir`'s own call and `state_dir`'s
resolution, both already covered by legs 5/5b, so a leg 1 plant on that name would test
nothing leg 5 doesn't already.)

## Group 7 — one plant per proof test (task 7.2)

Each plant is a one-line edit inline in the test (before its final assertion loop), run,
observed red, then the file was restored from a pre-plant backup (`diff` against the
backup afterwards reports no difference in every case).

- **`a_cli_rejected_schema_names_its_reason_per_change`** (`src/changes.rs`) — presence
  plant: `let learning_tool = &{ let mut c = learning_tool.clone(); c.problems.clear(); c };`
  before the width loop. Red: `width 78: "No content yet"` (the leading `!` row vanished).
- **`an_unusable_schema_renders_no_artifacts`** (`src/ui/detail.rs`) — absence plant (per
  the task's own instruction: "the plant must add what must not be there"): fabricated an
  artifact via `fixture::with_artifacts` on the not-vendored change before the tab-bar
  loop. Red: `left: "1 fake"`, `right: "no artifacts"`.
- **`no_tasks_artifact_renders_every_tab_as_markdown`** (`src/ui/detail.rs`) — absence
  plant: `fixture::track_tasks_at(change.clone(), 0)` before the per-tab loop, forcing tab
  0 into the checklist dispatch. Red: the rendered lines included a progress bar
  (`"██…[1/3] 33%"`), tripping the "no `[` glyph" assertion.
- **`an_unsupported_glob_names_its_reason_and_spares_the_others`** (`src/ui/detail.rs`) —
  presence plant: `fixture::with_problems(change.clone(), Vec::new())` before the render
  loop. Red: `["No content yet"]`.
- **`an_unreadable_tasks_file_is_zero_with_a_named_reason`** (`src/ui/detail.rs`) —
  presence plant: cleared both `change` and `change2`'s problems via
  `fixture::with_problems(..., Vec::new())` before the final loop. Red: `["No content
  yet"]`.

All five reverted; `cargo test --all-features --lib` green afterwards at 972 tests, and
`git status --porcelain` / `diff` against each pre-plant backup empty.
