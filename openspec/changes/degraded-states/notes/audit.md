# Audit — every row of `SPEC.md` → Degraded states

Re-derived per task group 1. Row count re-parsed with the exact script task 1.1 specifies
(`tests/degraded_coverage.rs` implements the same parse in group 11):

```
$ python3 - <<'PY'
import re
t = open('SPEC.md').read()
s = t.index('## Degraded states'); e = t.index('### No terminal is not a degraded state')
rows = [l for l in t[s:e].splitlines() if l.startswith('|')]
data = [l for l in rows
        if not re.match(r'^\|[\s\-:|]*$', l) and not l.startswith('| Condition')]
print(len(data))
PY
44
```

**44**, matching proposal.md, design.md, and `specs/degraded-coverage/spec.md`'s claim
(task 1.2 — no correction needed).

Legal verdicts (exactly these five, no others): `confirmed`, `unproven`, `spec-corrected`,
`repaired`, `implemented`.

This audit **re-checks** design.md's pre-planning partition against the tree rather than
transcribing it (task 1.3). Every disagreement or nuance found is called out in its own row
and summarised again in "Findings" below (task 1.4).

| # | Condition (verbatim) | Production location | Proving test today | Verdict |
|---|---|---|---|---|
| 1 | No `openspec/` found while walking up | `src/resolve.rs::find_repo` (`RepoSearch::NotFound`); `src/ui/list.rs::no_repo_rows` | `resolve.rs` `NotFound` tests; `list.rs` no-repo row tests | confirmed |
| 2 | `openspec` binary not found | none — no `file_mode` field on `Dashboard`, no badge anywhere in `src/ui/view.rs` (grepped for `file_mode`/`file mode`/badge: zero hits before this change) | none | implemented (this change, groups 3–4) |
| 3 | Schema unknown to the CLI | `src/changes.rs` schema-rejection fallback (CLI-tier merge path) | unit-tier only | unproven — proof exists below view/outer tier; group 7 (`a_cli_rejected_schema_names_its_reason_per_change`) raises it |
| 4 | Schema not vendored (no `openspec/schemas/<name>/schema.yaml` locally) | `src/schema.rs` load path, "is not vendored" branch; `src/changes.rs` artifact-list emptying | unit-tier only | unproven — group 7 (`an_unusable_schema_renders_no_artifacts`) raises it |
| 5 | Schema unreadable or invalid (I/O error, or bytes that are not a usable schema) | same load path, distinct error branch | unit-tier only | unproven — same group-7 test, second sub-case |
| 6 | Schema loads with no tasks artifact (`apply.tracks` matches nothing, and no artifact has id `tasks`) | `src/schema.rs` `apply.tracks` fallback / id-fallback logic | unit tests exist for the fallback selection itself; no view test yet proves *every tab* renders as markdown and the count still falls back correctly | unproven, not confirmed — design.md's "confirmed" placement is optimistic: task 9.1 (`no_tasks_artifact_renders_every_tab_as_markdown`) does not exist yet. Recorded as a disagreement with design.md; see Findings |
| 7 | No active changes | `src/ui/list.rs` `"No changes yet"` | `list.rs` empty-state tests | confirmed |
| 8 | A `/` filter matches no change | `src/ui/list.rs` `"No changes match"` | `list.rs` filter-empty tests | confirmed |
| 9 | Artifact file missing | `src/ui/detail.rs` `"No content yet"` | `detail.rs` missing-artifact tests | confirmed |
| 10 | An artifact file exists and cannot be read (permission error, I/O error) | `src/ui/detail.rs` problems-then-no-content-suppressed logic | `detail.rs::problems_only_are_returned_with_no_no_content_yet_line`, `::both_problems_and_source_are_returned_with_problems_first` | confirmed |
| 11 | No change is selected (an empty visible list, or a `/` filter matching none) | `src/ui/detail.rs::content_lines` with `change: None` | `detail.rs` blank-region tests | confirmed |
| 12 | Markdown source holds a construct the parser does not model (a table, a footnote, strikethrough, a task-list item), on a tab other than the tracked-tasks one | `src/ui/markdown.rs` parser fallback-to-source-line | `markdown.rs` has unit tests for the general fallback; no test names footnote/strikethrough/GFM-table/task-list specifically at both widths | unproven — group 9 (`unmodelled_constructs_render_as_source`) raises it |
| 13 | A tasks file exists and yields no task items | `src/ui/tasks.rs` `"No tasks yet"` | `tasks.rs` empty-tasks tests | confirmed |
| 14 | A marked tab's artifact resolves to no file while the change's `progress` is non-zero (the `tasks.md` fallback above counted a file the artifact itself did not) | `src/ui/tasks.rs` / `src/ui/detail.rs` header-vs-content disagreement path | `tasks.rs` header/content-disagreement tests | confirmed |
| 15 | `openspec/changes/` or its `archive/` exists and cannot be read | `src/changes.rs` (`problems.push` on the parent/archive read) | `changes.rs` unreadable-directory tests; rendered via `list.rs`'s leading `!` row | confirmed |
| 16 | An artifact's `generates` pattern falls outside the file path's supported glob subset | `src/changes.rs` (`problems.push(format!("artifact {:?}: {reason}"...` for the unsupported-glob case) | unit-tier only; no view test names the reason on screen for this specific case | unproven — group 9 (`an_unsupported_glob_names_its_reason_and_spares_the_others`) raises it |
| 17 | A tasks file exists but cannot be read (a directory where a file was expected, a permission error, an I/O error, or invalid UTF-8) | `src/tasks.rs` `Tasks::problems` | unit-tier only | unproven — group 7 (`an_unreadable_tasks_file_is_zero_with_a_named_reason`) raises it |
| 18 | Herdr socket unreachable | `src/agents.rs::poll_once` (`reachable: false` path returns `agents: Vec::new()`); `src/ui/list.rs` (agent column) | view tests hiding the agent column at both widths | spec-corrected — wording is fine for THIS row on its own (row-18's own text is not the one Decision 12 corrects); the correction Decision 12 #5 makes is about the **column being empty because an unreachable snapshot carries no agents, not because the view applies a hiding rule of its own** — a wording precision, not a behaviour gap. Kept as `spec-corrected` since the row's phrasing changes even though behaviour is already right |
| 19 | A launch's `pane split` call fails | `src/launch.rs` (`run_request`, split-failure branch) | `launch.rs` unit tests | unproven — group 8 (`every_launch_failure_renders_as_a_leading_row`, case a) raises it to outer+view |
| 20 | A launch's `agent start` call fails | `src/launch.rs` (start-failure branch) | `launch.rs` unit tests | unproven — group 8, case b |
| 21 | A launch's `agent prompt` call fails | `src/launch.rs` (prompt-failure branch) | `launch.rs` unit tests | unproven — group 8, case c |
| 22 | A launch's derived agent name is already live in the session | `src/launch.rs` / `src/ui/app.rs` (`Decision::Refuse`) | `launch.rs`/`app.rs` unit tests | unproven — group 8, case d |
| 23 | `state::record` fails after a successful `agent start` | `src/launch.rs` — **repaired by `51318d3`**: `Outcome::problems: Vec<String>` now accumulates the record failure then the prompt failure, in occurrence order | `launch.rs::a_failed_record_and_a_failed_prompt_are_both_reported` (unit) and `ui::list::tests::a_failed_record_and_a_failed_prompt_are_both_reported` (view) — both pass | repaired |
| 24 | `g` on a change with no attributed agent | `src/ui/app.rs` `FocusAgent` no-op path | `app.rs` unit tests | unproven — group 8 (`g_with_no_agent_renders_the_same_buffer`) raises it to a whole-buffer-equality outer+view proof |
| 25 | A `pane split` payload is not the measured envelope (a Herdr older than the manifest's `min_herdr_version` floor, `0.7.0`, might return one) | `src/launch.rs` JSON-decode-failure branch | `launch.rs` unit tests | unproven — group 8, case e |
| 26 | Pane narrower than 100 columns | `src/ui/layout.rs` (`WIDE_MIN_WIDTH`, `split_body`/`split_frame`) | `layout.rs` view tests at both widths | confirmed |
| 27 | `config.toml` malformed, unreadable, or a key of the wrong type | `src/config.rs::load` — `Config::problems` | `config.rs` unit tests prove the values are correct; **nothing renders `Config::problems` today** — `start_collaborators` never read it before this change | unproven — same "true but unobservable" defect class as row 31 (design.md → Context); group 3/5 make it renderable |
| 28 | `agent-names.toml` unusable (malformed, unreadable, or an entry Herdr would reject) | `src/state.rs::read` (non-destructive) / `src/state.rs::record` (rewrites from recovered entries only) | `state.rs` unit tests | spec-corrected — "nothing already on disk is lost" is true of `read` but false of `record`, which rewrites the file from the entries the parse recovered; an entry the parse could not recover is gone after the next `record` |
| 29 | A live agent's `cwd` is absent, or outside the resolved repository root | `src/agents.rs` containment check (`poll_once`) | `agents.rs::a_cwd_outside_the_repo_is_still_parsed` + attribution tests | confirmed |
| 30 | An agent works in a linked worktree of this repository | same containment check as row 29 — there is no separate worktree-specific code path; this is a consequence of the same `cwd` check, not a distinct mechanism | none dedicated (relies on row 29's containment test) | unproven — and SPEC.md's own forward reference ("recorded pending a change that reads `herdr worktree list`") is stale, since no such change exists on the roadmap; task 14.2 restates it as a standing, accepted limitation rather than a forward reference |
| 31 | A configured `openspec_bin` that does not name a usable binary | `src/resolve.rs::openspec_bin`, `step1_configured` — pushes the problem string | `resolve.rs` unit tests prove the value lands correctly in `BinResolution::problems` | unproven — **same defect class as row 27**: `cli::worker_cli` (pre-change) drops `resolution.problems` entirely, so nothing downstream — not even a log — ever reads it. Group 3 repairs this (`cli::worker_cli` now surrenders the problems alongside the handle) and group 5 renders it |
| 32 | A schema declares the same artifact id at two positions | `src/schema.rs` parser (accepts the duplicate, per `schema-artifacts`) | `schema.rs` unit tests parse the duplicate correctly | unproven — no test proves the *rendered* consequence (permanently file-mode); group 9 (`a_duplicate_artifact_id_parses_and_renders`) raises it |
| 33 | `openspec list --json` reports a repository root other than the one this plugin resolved | `src/changes.rs` (`root_agrees`/`same_directory`, CLI-merge discard path) | `changes.rs::a_symlinked_repository_root_is_not_a_disagreement`, `::an_envelope_with_no_root_is_treated_as_a_disagreement` (unit) | unproven — group 8 (`a_failed_cli_cycle_keeps_the_file_numbers`) raises it to outer+view |
| 34 | An `openspec` command exits non-zero | `src/changes.rs` (`cli_error_problem`) | `changes.rs` unit tests | unproven — group 8, same test as row 33 (second sub-case) |
| 35 | A `herdr agent list` call exits non-zero, or the program cannot be started at all | `src/agents.rs` (`herdr_error_problem`; `AgentSnapshot::problem`) | `agents.rs` unit tests; poller reads it every cycle | confirmed |
| 36 | The filesystem watcher will not start (`notify` refuses the watch, or the repository root cannot be watched) | `src/watch.rs::start` (both `notify::Watcher::new` and `.watch()` failure branches); row order now set by `src/ui/list.rs::rows` (launch, then refresh, then change-set problems) | `watch.rs::start` failure-path unit test; `list.rs` ordering tests | spec-corrected — "above every other problem row" is measurably false now that `agent-launch`'s launch-problem row leads (and did already, before this change); this change's own row-23 repair does not change that ordering, only the launch row's own content |
| 37 | `openspec/` is removed while the watcher runs | same ordering as row 36 | same | spec-corrected — identical correction and reason as row 36 |
| 38 | A CLI cycle fails after the worker already sent its file-sourced result | `src/refresh.rs` worker body (keeps the prior `ChangeSet` on a later error) | `refresh.rs` unit tests | unproven — group 8 (`a_failed_cli_cycle_keeps_the_file_numbers`) raises it |
| 39 | A touched path is classified to the wrong change, or conservatively to every change | `src/watch.rs::classify` | `watch.rs` unit tests | confirmed |
| 40 | `open`/`open-tab` invoked with no Herdr context at all (no workspace id) | `src/open.rs` (`HERDR_WORKSPACE_ID` message, exit 1) | `open.rs` unit tests; `tests/cli.rs` integration spawns the real binary for this case already | confirmed — proof already reaches the integration tier; group 9 (task 9.4) strengthens the *assertion*, not the tier (splits a tautological substring check into two real assertions and adds the empty-argv-log check) |
| 41 | `open`/`open-tab`'s `pane list` call fails or its payload is unparseable | `src/open.rs` (warn-and-continue branch) | `open.rs` unit tests | confirmed |
| 42 | `open`/`open-tab`'s `plugin pane focus` call fails with a usage error (code 2) | `src/open.rs` (usage-error branch: warn, fall through) | `open.rs` unit tests | confirmed |
| 43 | `open`/`open-tab`'s `plugin pane focus` call fails with a domain error, or `plugin pane open` itself fails | `src/open.rs::herdr_reason` — prefixes `"herdr exited with code {n}: "`, never Herdr's raw message verbatim | `open.rs` unit tests; `tests/cli.rs` integration | spec-corrected — "carried verbatim from Herdr" is false; the reason is `herdr_reason`'s own formatted text, not Herdr's stderr byte-for-byte |
| 44 | The dashboard process opened by `open`/`open-tab` finds no workspace cwd in its own injected Herdr context (`ui::startup_cwd` returns `None`) | `src/ui/mod.rs::startup_cwd` / `::startup_dir` (group 2, already committed) | `ui::mod.rs::tests::startup_dir` — all three cases (prefers workspace cwd, propagates a failing fallback, contextless-environment-as-fallback), unit tier | confirmed — group 2's `startup_dir` repair (this change) raises this from "unproven" at planning time to confirmed: both arms are now driven by a named, unit-tested function rather than residing untested in `run()`'s own body |

## Findings (task 1.4)

**Row 23 is the one `repaired` verdict**, and it is `agent-launch`'s gap, not a later
change's. The archived spec
(`openspec/changes/archive/2026-09-06-agent-launch/specs/agent-launch/spec.md`) defines
`Outcome` with a single `problem: Option<String>` (lines 419–421) and carries **two
separate** scenarios that each exercise one of the two failure sources that collide in that
one slot:

- "A failed prompt leaves a running, un-prompted agent that is still attributable" (line
  339): "the outcome reports **both**: `named` is `Some(...)` ... **and** `problem` names
  `agent_blocked`. This is the one path on which both fields are `Some`" — describes a
  **prompt** failure alone.
- "A failed recording does not undo a successful start" (line 352): "the outcome's problem
  names the state directory path and the I/O reason" — describes a **record** failure
  alone.

Neither scenario, nor any other in that spec, drives both failures **together** in one
run. Since `problem` is a single `Option<String>`, the second failure a run hits always
overwrites (in `src/launch.rs`'s pre-repair mapping, the `match` even discarded the record
failure outright whenever the prompt also failed) rather than accumulates. The gap is
structural — visible directly in the `Outcome` type's own signature — and `agent-launch`'s
own two scenarios are the exact pair that, run together, expose it. No later change ever
claimed this behaviour as its own; `degraded-states` repairs it here only because there is
no later change on the roadmap (design.md → Context) — `agent-launch` is the change that
owed it, and `openspec/IMPLEMENTATION-ORDER.md`'s `agent-launch` row is corrected to say so
(see this change's `proposal.md` and the `IMPLEMENTATION-ORDER.md` edit accompanying this
audit).

**Row 2 is the one `implemented` verdict**, confirmed genuinely unimplemented before this
change: grepping `file_mode`, `file mode`, and badge-drawing code across `src/ui/view.rs`
and `src/ui/app.rs` at the base commit returns zero hits outside the unrelated
agent-status badge `agent-attribution` already ships.

**Two disagreements with design.md's pre-planning partition are recorded above rather than
silently folded in, per task 1.4's instruction to report a finding before folding it in:**

1. **Row 6** — design.md lists it among the 16 `confirmed` rows. Re-checked against the
   tree: the fallback-selection logic is unit-tested, but no test yet proves the row's own
   claim ("every tab renders as markdown ... and the task count still falls back") through
   a view. This is not a behavioural gap — group 9's task 9.1
   (`no_tasks_artifact_renders_every_tab_as_markdown`) is exactly the task that raises it —
   but it is not `confirmed` *today*, at planning time, on the tier the row's own wording
   claims. Recorded as `unproven` here; group 9 confirms it for real.
2. **Row 44** — design.md's Context section (written before group 2 landed) implicitly
   treats this row as still needing work, since `startup_cwd`/`startup_dir` had not yet been
   extracted from `run()`'s untested body. Group 2 (this change, already committed) resolved
   it: `startup_dir` is now a named, unit-tested function. Recorded as `confirmed` here,
   reflecting the tree as it stands now rather than as it stood when design.md was written.

No other row's re-derived verdict disagrees with design.md's partition. Every `unproven`
row's proof is unit-tier-only for a reason: the collaborator whose problems it is proving
either never reached the render path before this change (rows 27, 31), or the proof
predates this change's own new view/outer scenarios (every row raised by groups 7–9,
enumerated above beside each row).

## Tally (cross-checked against task 1.5)

- **confirmed (17):** 1, 7, 8, 9, 10, 11, 13, 14, 15, 26, 29, 35, 39, 40, 41, 42, 44
- **unproven (20):** 3, 4, 5, 6, 12, 16, 17, 19, 20, 21, 22, 24, 25, 27, 30, 31, 32, 33, 34, 38
- **spec-corrected (5):** 18, 28, 36, 37, 43
- **repaired (1):** 23
- **implemented (1):** 2

17 + 20 + 5 + 1 + 1 = 44. This differs from design.md's planning-time tally (16
confirmed / 21 unproven) by exactly the two reclassifications in "Findings" above: row 44
moves from (implicitly) unproven to confirmed because group 2 already landed, and row 6
moves from confirmed to unproven because its view-tier proof does not exist yet. Every
parsed condition appears exactly once above with exactly one of the five legal verdicts

## Addendum — row 6 after group 5

Group 5 wired `Change::problems` into `ui::detail::content_lines` wholesale, rendering
every entry the schema-and-artifact resolution pipeline produces — not a curated subset.
`schema::id_fallback`'s "no tasks artifact: no apply.tracks value and no artifact has id
\"tasks\"\"" message (row 6's own condition) is one of those entries, folded into the same
`Change::problems` vector `build_change`'s doc comment already documents as accumulating
"selection, then load, then artifacts, then tasks" in one ordered list. So row 6 now
renders a leading `!` line too, alongside rows 3/4/5/16/27/31.

This was not separately called out as its own "reason is named" clause in `SPEC.md`'s row
6 text at planning time — that row's wording speaks only to the tab bar ("every tab
renders as markdown ... no tab is added, removed, or hidden") and the task count fallback,
and is silent on whether a reason line appears. Rendering it is consistent with, not
contradicted by, that wording, and with design.md's own instruction to make
`Change::problems` "read by something" — there is no carve-out recorded anywhere for this
one entry among the others sharing the same vector. Recorded here so group 9's own row-6
proof (`no_tasks_artifact_renders_every_tab_as_markdown`) is written expecting a leading
problem row, rather than being surprised by one.
(task 1.5).
