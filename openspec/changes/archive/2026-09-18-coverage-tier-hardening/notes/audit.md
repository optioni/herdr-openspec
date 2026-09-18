# The `covers` audit — all 74 ranges across 59 rows, and the 79 they became

Task 3.2's mandate: for each range, read the row's `condition`, `why`, and `proof`, then
confirm the range names code that **runs when that condition holds**. No rule reaches this
question, and this change's new structural rule explicitly does not — it asks what a range
*contains*, never what it is *about*.

## Method

Four subagents audited disjoint batches, each given the row blocks verbatim and told to read
the range, widen to the enclosing function, and propose a precise replacement where the
binding failed. They edited nothing. Every finding acted on below was re-verified in the
applying session by reading the source, and every proposed range was checked against the new
structural rule before it was written into the map. Where an agent's reading of a line's
content was wrong, the source decided: `src/changes.rs:1794` is `archived,`, not
`active: file_active,` — the substance of the finding survived, the detail did not.

## Headline: 24 of the 59 rows were mis-bound, replacing 28 ranges with 33

The archived `settings-window` review scoped this at "~10 rows in the same vacuous shape"
(`notes/change-review.md` -> W7), and the plan named two by path. The real yield is
**twenty-four rows**. Part of the difference is a defect class neither the review nor this
change's plan anticipated.

### Five classes, and the gate catches one

Counted by **row** first, because a row is what the map binds; the range column counts the
individual ranges replaced, and the two measures agree on the ordering.

| Class | Rows | Ranges | What it is | Caught by `covers-check`? |
|---|---|---|---|---|
| **Aboutness** | 12 | 13 | real, instrumented, hot code with nothing to do with the row | no, and no rule can |
| **Drift** | 7 | 9 | a binding that was **correct when written** and silently slid | no — a drifted range still holds statements |
| **Opposite branch** | 3 | 4 | the range names the path taken when the condition does *not* hold | no |
| **Shape** | 1 | 1 | the range only *names* code — a bare signature, a struct's fields | **yes**, this is the rule's whole subject |
| **Pattern field** | 1 | 1 | a field of a destructuring `let` pattern — shape, one step outside the rule | no, see below |
| | **24** | **28** | | |

**Drift was on nobody's list.** Aboutness is the larger class and the plan expected it; drift
is second, and the plan did not name it at all. All eight `src/open.rs`
ranges drifted in commit `3ce2d50 test(tab-open-focus): act on the change review's
findings` — a commit of 38 added lines that are **mostly doc comments**. Eight insertions
above `run`'s body and two above `context` moved every binding in the file. Verified by
reading `git show 3ce2d50^:src/open.rs`, where each stale range lands exactly on the code
its row describes:

- `open.rs:411-417`, bound to "focus fails with a **usage error**", is the focus **success**
  arm at HEAD — the one path on which the condition cannot hold.
- `open.rs:454-456`, bound to "**post-open** focus fails", is the post-open listing *call*.
- `open.rs:437-441`, bound to "focus domain error, or `pane open` itself fails", is
  `open_ctx`'s construction, which runs identically on success and failure.

`src/launch.rs:158-162` is the same class at two lines: its last two lines are
`Decision::Go(Request::Launch {` / `change: change.to_string(),`, the branch taken exactly
when the derived agent name is **not** already live — the negation of its row.

This is worth stating plainly rather than leaving implied: **a documentation-only commit
broke eight coverage bindings and nothing noticed.** `make coverage`'s hotness check could
not see it, because the drifted-onto code is hot; the old structural rule could not see it,
because the drifted-onto code is real; and the new structural rule cannot see it either.
What would catch it is an anchor that survives an edit above it — a function name, a
`// COVERS:` marker — and that is a different change to propose, not a silent widening of
this one.

### The rule's own blind spot, found by the audit

`src/changes.rs:1794-1794` was `archived,` — a **field of a destructuring pattern**
(`let ChangeSet { active: file_active, archived, .. } = files;`). It passes the new rule,
because the rule recognises a struct *declaration*'s extent and a pattern is not one. It is
exactly the vacuous shape the change exists to reject, one syntactic step outside the rule's
reach. Recorded here rather than widened into the rule: extending the extent scan to `let`
patterns is a change with its own controls to write.

## Every range's verdict

| # | Condition | `covers` at HEAD of this change | Verdict |
|---|---|---|---|
| 1 | No `openspec/` found while walking up | `src/ui/list.rs:348-368`, `src/ui/list.rs:380-381` | **CORRECTED (aboutness)** — was `list.rs:300-322`, `fold_glyph`; now `no_repo_rows`' body and the guard that reaches it |
| 2 | `openspec` binary not found | `src/ui/view.rs:596-598`, `src/ui/view.rs:609-616` | **CORRECTED (aboutness)** — was `view.rs:400-421`, drag-select spans + `detail_row_role`; now the badge's drop-whole decision and its right-aligned draw |
| 3 | Schema unknown to the CLI | `src/changes.rs:1430-1474` | **confirmed** — `resolve_cli_schema_uncached`; the CLI-rejection arm at 1461-1464 is inside it. Over-wide — see Reported and left |
| 4 | Schema not vendored (no `openspec/schemas/<name>/schema.y... | `src/changes.rs:993-995` | **confirmed** — `schema_load_problem`'s `NotVendored` arm; keyed on the condition by construction |
| 5 | Schema unreadable or invalid (I/O error, or bytes that ar... | `src/changes.rs:996-1001` | **confirmed** — `schema_load_problem`'s `Unreadable` and `Invalid` arms. The I/O-error half is unproven by the named proof — see Reported and left |
| 6 | Schema loads with no tasks artifact (`apply.tracks` match... | `src/changes.rs:808-810` | **confirmed** — `change_progress`'s `tasks.md` fallback, the arm taken exactly when no tasks artifact resolves |
| 7 | No active changes | `src/ui/list.rs:536-542` | **CORRECTED (aboutness)** — was `list.rs:405-412`, unconditional count arithmetic; now **both** empty-state branches: the `No active changes` row that is the SPEC row's own behaviour ("archived changes remain browsable") and the `No changes yet` branch the named proof actually drives, which positively asserts the former's absence. Change Review WARNING 1: binding only the proof's branch left the row's own wording uncovered |
| 8 | A `/` filter matches no change | `src/ui/list.rs:544-555` | **CORRECTED (aboutness)** — was `list.rs:413-426`, a comment plus more unconditional arithmetic; now the `No changes match` branch |
| 9 | Artifact file missing | `src/ui/detail.rs:843-845` | **CORRECTED (aboutness)** — was `detail.rs:512-522`, `bodies_are_indented`; now the `out.is_empty()` -> `no_content_yet_row` fallback |
| 10 | An artifact file exists and cannot be read (permission er... | `src/ui/app.rs:2218-2222`, `src/ui/detail.rs:672-672` | **CORRECTED (aboutness)** — was `detail.rs:525-535`, `indented_line`; now `sync_detail`'s read `Err` arm and the problem-row extend |
| 11 | No change is selected (an empty visible list, or a `/` fi... | `src/ui/view.rs:128-130` | **confirmed** — `render_detail`'s `let ... else { return; }` guard — the blank interior itself |
| 12 | Markdown source holds a construct the parser does not mod... | `src/ui/markdown.rs:809-811` | **CORRECTED (aboutness)** — was `markdown.rs:786-786`, `Folder::end`'s unconditional `self.blocks`; now the parser option set. No condition-exclusive code exists — see Reported and left |
| 13 | A tasks file exists and yields no task **items** | `src/ui/tasks.rs:635-639` | **CORRECTED (aboutness)** — was `tasks.rs:591-597`, inside a per-item loop that does not run with zero items; now the `No tasks yet` site |
| 14 | A marked tab's artifact resolves to no file while the cha... | `src/ui/detail.rs:843-845`, `src/ui/detail.rs:674-679` | **CORRECTED (aboutness)** — was the same two indent-geometry ranges as `Artifact file missing`, copied; now the `No content yet` fallback and `header_row`'s `progress.total > 0` branch, which builds the counted pair the row's `why` names. Change Review WARNING 2: the first repair bound `tracked_tasks_progress`, whose value is read only inside `if detail.foldable()` and is therefore discarded on this row's own path |
| 15 | `openspec/changes/` or its `archive/` exists and cannot b... | `src/ui/list.rs:432-439` | **confirmed** — `push_problem`'s body, reached by the `changes.problems` loop. Shared by five sources — see Reported and left |
| 16 | An artifact's `generates` pattern falls outside the file ... | `src/changes.rs:748-751` | **confirmed** — `resolve_artifact`'s unsupported-glob `Err` arm, where the named reason is produced |
| 17 | A `generates` glob crosses a symlinked directory whose ta... | `src/changes.rs:693-694`, `src/changes.rs:697-697`, `src/changes.rs:703-705` | **CORRECTED (opposite branch)** — `697-699`/`701-705` held the real-directory recursion, the opposite of the condition; now the `is_dir` branch guard and the regular-file rejection a symlink falls through to |
| 18 | A tasks file exists but cannot be read (a directory where... | `src/tasks.rs:551-554` | **CORRECTED (shape)** — was `tasks.rs:193-196`, three doc-comment lines and a bare `pub fn` signature; now `read`'s catch-all `Err` arm. The one range the new structural rule catches |
| 19 | Herdr socket unreachable | `src/ui/list.rs:456-458` | **CORRECTED (opposite branch)** — `456-460`'s only statement is the push the proof never reaches — it sets `stalled: false`; now the guard itself, which is what the spec-correction records |
| 20 | A launch's `pane split` call fails | `src/launch.rs:399-405` | **confirmed** — the split-failure `Err` arm |
| 21 | A launch's `agent start` call fails | `src/launch.rs:424-435` | **confirmed** — the `agent start` failure body, whose pane/agent-naming format is unique to this row |
| 22 | A launch's `agent prompt` call fails | `src/launch.rs:449-451` | **confirmed** — the prompt argv build and the push that runs only on a failed `agent prompt` |
| 23 | A launch's derived agent name is already live in the session | `src/launch.rs:156-160` | **CORRECTED (drift)** — `158-162` straddled into `Decision::Go`, which runs exactly when the name is NOT live; now the `live_names` guard and its whole refusal |
| 24 | `state::record` fails after a successful `agent start` | `src/launch.rs:441-444` | **confirmed** — the record-failure push, reached only past the `agent start` success path |
| 25 | `g` on a change with no attributed agent | `src/launch.rs:124-131` | **confirmed** — `decide`'s `Focus` branch; `g` with no pane lands on `Decision::Nothing` |
| 26 | A `pane split` payload is not the measured envelope (a He... | `src/launch.rs:201-209` | **confirmed** — `pane_id`'s two `ok_or_else` closures, which run exactly for a mis-shaped envelope. Three further legs sit outside — see Reported and left |
| 27 | Pane narrower than 100 columns | `src/ui/layout.rs:27-30` | **confirmed** — the whole body of `layout::mode` — the one-region switch |
| 28 | `config.toml` malformed, unreadable, or a key of the wron... | `src/config.rs:134-139`, `src/config.rs:146-153` | **confirmed** — the unreadable and malformed arms. The wrong-type leg has no range — see Reported and left |
| 29 | `agent-names.toml` unusable (malformed, unreadable, or an... | `src/state.rs:165-170`, `src/state.rs:177-184` | **confirmed** — the unreadable and malformed arms. The bad-entry leg has no range — see Reported and left |
| 30 | A live agent's `cwd` is absent, or outside the resolved r... | `src/agents.rs:137-143` | **CORRECTED (aboutness)** — was `agents.rs:120-126`, `attribute`'s accumulator prologue; now the `in_scope` containment check and its `continue` |
| 31 | An agent works in a **linked worktree** of this repository | `src/agents.rs:137-143` | **CORRECTED (aboutness)** — same prologue, same repair — the row's own `why` says it fails the same containment check, and that check was not in the range |
| 32 | A configured `openspec_bin` that does not name a usable b... | `src/resolve.rs:265-268` | **confirmed** — the not-usable push, reached only when a configured binary failed step 1 |
| 33 | A schema declares the same artifact id at two positions | `src/schema.rs:263-268` | **CORRECTED (opposite branch)** — `258-263` was mostly the EMPTY-sequence early return; now the loop that keys artifacts by position and pushes both duplicates |
| 34 | `openspec list --json` reports a repository root other th... | `src/changes.rs:1643-1660` | **confirmed** — the `root_agrees` guard and the empty-`active` return that leaves the file numbers standing |
| 35 | An `openspec` command exits non-zero | `src/changes.rs:1621-1628`, `src/changes.rs:1396-1411` | **confirmed** — the failed-`list` arm and `cli_error_problem`'s `Failed` variant |
| 36 | A `herdr agent list` call exits non-zero, or the program ... | `src/agents.rs:371-376` | **confirmed** — `poll_once`'s mapping of every failing path to an empty, unreachable snapshot |
| 37 | The Herdr agent poller's outstanding `agent list` call pa... | `src/agents.rs:517-532` | **confirmed** — `drain_at`'s stall report |
| 38 | The filesystem watcher will not start (`notify` refuses t... | `src/watch.rs:284-288`, `src/watch.rs:294-298` | **confirmed** — both `Err` arms. The row's `why` names them in the opposite order — see Reported and left |
| 39 | `openspec/` is removed while the watcher runs | `src/watch.rs:243-250` | **confirmed (weak)** — `drain`'s debounce tail; it runs when the condition holds but also on every frame — see Reported and left |
| 40 | The refresh worker stops answering (its worker thread has... | `src/refresh.rs:147-148`, `src/refresh.rs:150-154`, `src/refresh.rs:176-179` | **confirmed** — the post-`Stopped` silence, the `pending_death` consumption, and the channel-disconnect arm |
| 41 | A CLI cycle fails after the worker already sent its file-... | `src/changes.rs:1623-1628`, `src/changes.rs:1830-1830` | **CORRECTED (aboutness)** — `1821-1822` sat in the arm taken when the CLI DID answer; now the failed-`list` return and the `None => merged.push(file_change)` arm that keeps the file numbers |
| 42 | A touched path is classified to the wrong change, or cons... | `src/watch.rs:156-168` | **confirmed** — `invalidate`'s fold, holding the over-broad-vs-narrow decision |
| 43 | `open`/`open-tab` invoked with no Herdr context at all (n... | `src/open.rs:72-76` | **CORRECTED (drift)** — +2 lines in `3ce2d50`; now the `ok_or_else` fatal arm including its message |
| 44 | `open`/`open-tab`'s `pane list` call fails or its payload... | `src/open.rs:391-407` | **CORRECTED (drift)** — +8 lines in `3ce2d50`; now the whole first listing match, both `Err` halves |
| 45 | `open`/`open-tab`'s `plugin pane focus` call fails with a... | `src/open.rs:419-425` | **CORRECTED (drift)** — +8; the stale range was the focus SUCCESS arm. Now the `code: Some(2)` guard and its warning push |
| 46 | `open`/`open-tab`'s `plugin pane focus` call fails with a... | `src/open.rs:426-430`, `src/open.rs:445-450` | **CORRECTED (drift)** — +8; the stale ranges were the `is_usage_error` guard and `open_ctx`'s construction. Now the domain-error return and the `pane open` failure return |
| 47 | `open`/`open-tab`'s **post-open** `pane list` call fails,... | `src/open.rs:466-471`, `src/open.rs:473-475` | **CORRECTED (drift)** — +8; now the no-pane-identified arm and the unparseable/list-failed arms |
| 48 | `open`/`open-tab`'s **post-open** `plugin pane focus` cal... | `src/open.rs:462-464` | **CORRECTED (drift)** — +8; the stale range was the post-open listing call. Now the focus failure's own warning push |
| 49 | The dashboard process opened by `open`/`open-tab` finds n... | `src/ui/mod.rs:417-422` | **confirmed (weak)** — `startup_cwd`'s body; it produces the `None` but runs identically when a cwd is found — see Reported and left |
| 50 | The terminal refuses mouse capture (`enable_mouse` fails) | `src/ui/mod.rs:341-343`, `src/ui/terminal.rs:83-83`, `src/ui/terminal.rs:91-93` | **confirmed** — the `mouse_problem` push and the `enable_mouse` error capture. The accessor range is non-discriminating — see Reported and left |
| 51 | The clipboard write fails locally (`TerminalOps::write_cl... | `src/ui/driver.rs:398-400` | **confirmed** — the failing write and the recording of its reason onto `Selection::problem` |
| 52 | A change is archived between the worker's `list --json` c... | `src/changes.rs:1902-1918`, `src/changes.rs:1834-1835`, `src/changes.rs:1842-1847` | **CORRECTED (vacuous shape)** — `1794-1794` was `archived,` — a field of `merge`'s destructuring PATTERN, which passes the new rule because the rule recognises struct declarations, not struct patterns; now the archived file walk that puts the change in the archived tier |
| 53 | `herdr integration status` cannot be read (non-zero exit,... | `src/launch.rs:893-909` | **confirmed** — `resolve_kind`'s `integration status` match, including its `Err` arm |
| 54 | Two or more agent integrations installed, with no `agent_... | `src/launch.rs:796-816` | **confirmed** — the `Choice::Ambiguous` arm that stops the launch with one problem |
| 55 | No agent integration installed at all, with no `agent_kin... | `src/integration.rs:203-215` | **confirmed** — `resolve`'s `0 =>` arm and its last-resort problem. Boundary lines belong to neighbouring arms — see Reported and left |
| 56 | The chosen agent kind's own integration is not installed ... | `src/integration.rs:174-185` | **confirmed** — the not-installed guard's push and the return that still yields the kind |
| 57 | No installed integration leaves the settings panel's `age... | `src/settings.rs:258-261` | **confirmed** — the `Editable::No { reason: NoIntegration }` branch |
| 58 | The settings panel's commit to `settings.toml` fails (the... | `src/state.rs:367-376` | **confirmed** — `record_kind`'s resolve guard and the failing `create_dir_all`. The write-error leg has no range — see Reported and left |
| 59 | `a`, `c`, or `s` pressed in file mode | `src/launch.rs:136-142` | **confirmed** — `decide`'s file-mode guard, after the `Focus` early return that exempts `g` |

Rows: 59. Ranges: 79 (74 before this change).

## Reported and left

Task 3.2's own rule: a row whose `proof` watches something other than its `why` claims is
reported and left, not silently re-aimed. The same discipline applies to a range that is
genuinely executed but under-covers its row, or that is on-topic but non-discriminating.
None of the following is a mis-binding; each is a smaller, separate piece of work.

**Rows whose condition has a leg no range covers.** Adding a range here would be
strengthening a binding rather than correcting one, and each new range must also be *hot* or
`make coverage` fails — so each carries a test question this change did not open.

| Row | Uncovered leg | Where it lives |
|---|---|---|
| `config.toml` malformed, unreadable, or a key of the wrong type | "a key of the wrong type" | `src/config.rs:159-161`, the `openspec_bin is not a string` push |
| `agent-names.toml` unusable | "an entry Herdr would reject" | `src/state.rs:205-216`, the per-entry match |
| The settings panel's commit to `settings.toml` fails | "or the write itself errors" | `src/state.rs:395-403`, the rename failure's cleanup and `Err` |
| Schema unreadable or invalid | the I/O-error half; the named proof only drives the invalid-bytes half | `src/changes.rs:996-998` is bound, but no sub-case makes the file unreadable |
| A `pane split` payload is not the measured envelope | invalid JSON, non-object, and Herdr's own error envelope | `src/launch.rs:176-193` |

**Ranges that are on-topic but discriminate nothing** — they run when the condition holds
*and* when it does not, so they prove execution rather than the degraded path. Left because
each is the value's genuine production site and the alternative is a range that is narrower
but no more honest:

- `src/ui/mod.rs:417-422` — `startup_cwd`'s body produces the `None` but runs identically
  when a cwd is found. The discriminating site is the `match startup_cwd(env)` fallback.
- `src/ui/terminal.rs:91-93` — the `mouse_problem()` accessor, called unconditionally by
  `run`; kept because it is the value's sole exit route.
- `src/watch.rs:243-250` — `drain`'s debounce tail runs on every frame.
- `src/changes.rs:1842-1847` — `merge`'s return value; harmless beside the range that does
  discriminate, and proves nothing alone.
- `src/ui/list.rs:432-439` — `push_problem`'s body, shared by five problem sources.
- `src/changes.rs:1430-1474` — the whole of `resolve_cli_schema_uncached`; the CLI-rejection
  arm is `1461-1464`.
- `src/integration.rs:203-215` — straddles one line into each neighbouring `match` arm;
  `205-214` is exact.
- `src/launch.rs:124-131` — two lines wider than the `Focus` branch it is about.

**One row may be genuinely unattributable.** "Markdown source holds a construct the parser
does not model (a footnote)" has no condition-exclusive code **by design**: with
`ENABLE_FOOTNOTES` off, pulldown-cmark emits a footnote's syntax as ordinary `Event::Text`,
so there is no footnote branch to point at — the absence *is* the implementation. It was
bound to `Folder::end`'s unconditional `self.blocks`, which is vacuous; it is now bound to
the `Parser::new_ext` option set, which is the decision that makes a footnote degrade to
literal source. That is the honest binding available, and it is still not exclusive to the
condition. Recorded rather than papered over.

**One prose error, no range change.** The watcher row's `why` names its two ranges in the
opposite order to the code: `src/watch.rs:284-288` is the **constructor's** `Err` arm and
`294-298` is the `.watch()` arm, not the reverse. Left for the row's own change, per the
rule above — this change corrects where rows point, never what they say.

## What this leaves standing

`make covers-check` exits 0. All 79 ranges resolve, are in bounds, and hold a statement, and
all 79 are instrumented and hot, confirmed by a full `make coverage` run — production slice
96.48%, up from 96.23%, because the corrected ranges point at better-covered code than the
arithmetic and geometry they replaced.

What it does **not** leave standing is the reading this change exists to prevent: the gate
being green does not mean the bindings are right. One of the three defect classes above is
machine-caught. The other four were found by reading 74 ranges against 59 conditions, and
they will come back — drift in particular, on the next commit that inserts a line above a
bound range.
