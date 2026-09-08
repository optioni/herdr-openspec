# herdr-openspec — Design Specification

**Status:** Draft
**Date:** 2026-09-03
**Requirements:** see `PRD.md`

## Overview

`herdr-openspec` is a Herdr plugin providing a read-only dashboard for OpenSpec
state, plus the ability to launch an agent onto a change. It ships a single Rust
binary invoked two ways by the manifest: as a pane process rendering the TUI, and
as an action process that opens or focuses that pane.

**Stack:** Rust, `ratatui` (TUI) — `crossterm` is reached through `ratatui`'s own
re-export and is not itself a declared dependency, so a backend type and an event
type can never come from two different `crossterm` releases — `notify` 8.2.0
(filesystem watching; `default-features = false, features = ["macos_fsevent"]` —
`default-features = false` alone does not compile on macOS, since `notify`'s
FSEvents backend is gated on the *absence* of `macos_kqueue`, not the presence of
`macos_fsevent`, and FSEvents recurses in the kernel while the kqueue backend opens
one file descriptor per watched entry; the argument, and why no
`notify-debouncer-*` crate is used, is in `live-refresh`'s design.md — a later
change needing a watcher should not re-open it), `serde_json`, `yaml-rust2` (YAML —
the choice is argued in `schema-model`'s design.md; a later change needing YAML
should not re-open it), `pulldown-cmark` (markdown — the version and the
`default-features = false` choice are argued in `markdown-viewer`'s design.md; a
later change needing markdown should not re-open it), `toml` (plugin configuration
and state, both TOML). Exact versions are pinned to current stable releases at
implementation time, not from memory.

## Architecture

Two boundaries define the design. Everything interesting lives between them.

**The subprocess seam.** `cli` is the only module in this crate permitted to
spawn a process. Two traits carry the two programs it wraps directly:

```rust
pub trait OpenspecCli: Send + Sync { fn run(&self, args: &[&str]) -> Result<String, CliError>; }
pub trait HerdrCli:    Send + Sync { fn run(&self, args: &[&str]) -> Result<String, CliError>; }
```

Both require `Send + Sync`: `live-refresh` runs CLI calls on a worker thread and
`agent-polling` polls on another, so a trait that could not cross a thread boundary
would have to be redesigned by the first change that used one. Each has exactly one
real implementation that spawns a process and returns stdout, and a fake used by
tests. No parsing, merging, or decision-making happens inside either. A third spawn
lives behind the same seam: the one-shot `npm prefix -g` probe that
`resolve::openspec_bin`'s fourth step needs. It is neither `openspec` nor `herdr`, so
it is not one of the two traits above — but it is still a process spawn, and `cli` is
still where it lives. `resolve` itself never spawns it: the probe arrives as an
injected `&dyn Fn() -> Option<PathBuf>`, the same shape by which `resolve` and
`config` take the process environment as a lookup closure, so `resolve` stays pure and
the seam still exists before anything crosses it. This is what makes the coverage
target reachable: the wrappers and the probe are covered by tests run against
scratch `#!/bin/sh` programs, and the untestable residue is the one-line `npm_prefix()`
program binding plus `main`. (`config::env_lookup` is a second one-line binding to the
real world — the crate's single call to `std::env::var` — but it is not untestable
residue: it carries its own assertions, comparing its result against `std::env::var`
directly for a variable known to be present and for one nothing sets, rather than
being covered only by the composition that calls it.)

**The render seam.** Views are pure functions from a `Dashboard` state value to a
ratatui frame. They perform no I/O, so they are tested by rendering into a
`TestBackend` buffer at fixed widths. `ui::read_artifact` is the crate's third
one-line binding to the real world, alongside `resolve`'s `npm prefix -g`
hook and `config::env_lookup`: the loop's injected `&dyn Fn(&Path) ->
Result<String, String>` reader, whose one production binding lives in
`src/ui/mod.rs` and is the only place under `src/ui/` naming
`read_to_string` (`detail-view`). The fourth is `watch::RealFsEvents::drain`'s
one `Instant::now()` call (`live-refresh`): the debounce it drives takes `now`
as a parameter rather than reading the clock itself, so no view test can
reach it — `NOBLOCK`'s leg 2 makes that a checked fact, not merely a claim,
by forbidding every file under `src/ui/` (tests included) from naming
`Instant::now`. `src/agents.rs`'s `RealAgentPoll::drain` (`agent-polling`)
reads a clock the same way, for the same reason: the schedule lives on the
render side of the poller seam so the loop can shorten its wait for a poll
without reading a clock itself, which makes it the crate's **second** clock
binding, not its only one — `NOBLOCK` leg 2 covers it identically.

**Module map:**

| Module | Responsibility |
|---|---|
| `config` | Resolve the plugin's configuration directory from the process environment (no process spawn) and read `config.toml` into `openspec_bin`, `agent_kind`, `archived_count` |
| `state` | Resolve the plugin's state directory from the process environment; derive a Herdr-legal agent name from a change name; record and read back the mapping |
| `resolve` | Locate the repository root and the `openspec` binary |
| `schema` | Parse `schema.yaml` into an ordered artifact list; identify the tasks artifact |
| `changes` | Build `Change` values from files and from CLI JSON |
| `tasks` | Parse markdown checkboxes into groups, items, and counts |
| `agents` | Poll `herdr agent list`, parse its envelope into agent values, and attribute live Herdr agents to changes |
| `launch` | Split a pane, start an agent, send the `/opsx:*` prompt |
| `watch` | The recursive `notify` watch, the debounce, and classifying a touched path to a per-change `Selection` |
| `refresh` | The worker thread and the non-blocking `Refresher` seam it answers through |
| `open` | The `open` and `open-tab` subcommands that open or focus the dashboard pane through `herdr plugin pane`; the crate's third `HerdrCli` consumer |
| `ui` | Views (the change-row grammar, the detail region's header/tab-bar/content grammar, markdown rendering, and `ui::tasks`' checklist-and-progress-bar grammar for the tracked-tasks tab), layout, the dashboard's own state (selection, the `/` filter, the detail scroll offset, the selected artifact tab, the live tier's refresh flag and standing problems, and the injected artifact-read binding), key handling, terminal lifecycle, and the event loop |
| `cli` | The two subprocess traits and their real implementations |

## Data layer

### Dual-source model

The two sources differ in latency and authority, and the design uses both rather
than treating one as a fallback for the other:

- **Files render immediately.** Walking `openspec/changes/` and parsing checkboxes
  costs well under a millisecond. The pane paints on open.
- **The CLI arrives and corrects.** `openspec` is a Node binary costing 200–400ms
  per invocation. It runs on a worker thread; when its JSON lands, the view is
  upgraded to schema-correct state.

The consequence is that an absent CLI degrades correctness slightly rather than
breaking the pane, and the file path is exercised on every launch rather than
being untested fallback code.

The two sources must also agree on the *same* number, or the pane would
flicker between two counts for a change nobody touched. `tasks::count` and
`tasks::parse` therefore reproduce the OpenSpec CLI's own checkbox rule
verbatim — a `-`/`*` bullet carrying a one-character `[ ]`/`[x]`/`[X]` box,
counted even inside a fence or an HTML comment, on purpose — rather than a
hand-designed one. Re-verify it against `dist/utils/task-progress.js`'s
`TASK_LINE_PATTERN` (`@fission-ai/openspec` 1.11.0 on the reference machine)
if the two sources are ever suspected of disagreeing. `Progress::is_complete`
mirrors the CLI's three-way split too: `total > 0 && completed == total`.

On the CLI side, the count comes from `openspec list --json`'s
`completedTasks`/`totalTasks` pair — **never** from
`openspec instructions apply --change <n> --json`'s own `progress` field. The
two are different computations: `list` resolves the tracked-tasks artifact's
`generates` through the same glob expansion the file path uses and sums every
matched file, falling back to `<changeDir>/tasks.md`; apply resolves
`apply.tracks` as a single path with no globbing. They agree only when
`apply.tracks` names a plain filename, so `changes::from_cli` reads
`list --json`'s pair and discards apply's `progress` entirely.

### Resolution chain

**Repository.** Start from the invocation context's workspace working directory
and walk up looking for `openspec/`, stopping at the innermost ancestor that
holds it — an `openspec/` several levels up is not preferred over one closer in.
A regular file named `openspec` does not count; only a directory (including one
reached through a symbolic link) does. The starting path is canonicalized
before the walk when the filesystem can resolve it, so a `..` component or a
symlinked working directory does not leak into the root the empty state
prints or a later change joins onto. If none is found, render an empty state
naming the directory searched.

**Schema.** Determine the schema name by consulting, in order: a change's own
`.openspec.yaml` → `schema:`, then the repository's `openspec/config.yaml` →
`schema:`, then the default `spec-driven` when neither declares one. Search for
`openspec/schemas/<name>/schema.yaml` in three tiers: the project's own
`openspec/schemas/` — graft places a vendored schema there — then
`$XDG_DATA_HOME`, then the CLI package's own built-ins; the third tier is why a CLI
fallback exists at all, since `spec-driven` is not vendored in any repository. When
the first tier misses, ask `openspec schema which <name> --json`, which returns
`{"name","source","path","shadows"}` where `path` is the schema's *directory*
(stdout is clean JSON; an "experimental" note goes to stderr) — read the same
`schema.yaml` from that directory. The `artifacts` list becomes the tab order. The
tasks artifact is the one whose `generates` equals the schema's top-level
`apply.tracks`, falling back to the artifact with id `tasks` when no `apply` block
declares what it tracks; a *present* `tracks` value matching nothing — a wrong-typed
one included — yields no tasks artifact rather than falling back, because that is
what the OpenSpec CLI does and the dual-source model depends on the two agreeing.
There is no `role` key anywhere in the schema format.

**Changes.** `openspec list --json` yields the envelope
`{"changes": [ … ], "root": {"path", "source"}}` — **not** a bare array. Each
element of `changes` carries `{name, completedTasks, totalTasks, lastModified,
status}`. The file path enumerates subdirectories of `openspec/changes/`
excluding `archive/`. Active changes are ordered name-ascending in byte order —
a contract, not a preference, because both producers of `Change` must agree on
it: the CLI's own default order is most-recently-modified first, and its own
`--sort name` flag sorts with `localeCompare` — a locale collation, not byte
order — so it cannot be used to obtain the shared order; the CLI path instead
re-sorts its own result by name, in byte order, after the fact. `Change`
carries no `status` and no `lastModified` field; the CLI derives the first
from the completed/total pair (`Progress::is_complete` mirrors the same split)
and no view renders the second.

**Archived changes.** Directories under `openspec/changes/archive/` named
`YYYY-MM-DD-<name>`. Strip the date prefix; entries are ordered dated-newest-
first, with same-date entries broken by name descending, and every undated
entry ordered after all dated ones, among itself by name descending; show the
five most recent (`archived_count` in plugin configuration). Archived changes
are permanently file-sourced — the CLI has no way to address one — so this
ordering is the plugin's own rather than copied from a CLI default.

A change's `artifacts` are joined between the file and CLI producers by
**position** in the schema's declared order, never by path or by id:
`schema-artifacts` requires a schema's artifact list to be kept verbatim and
never de-duplicated, so an id is not a key, and the file producer does not
canonicalize its paths while the CLI's `contextFiles` does, so the path
strings legitimately differ.

**Artifact files.** The file path resolves each schema artifact's `generates`
value against the change directory — never a path constructed from the
artifact's `id`. `<id>.md` for a file artifact and `<id>/` for a directory
artifact is what `generates` happens to reduce to for the vendored `tdd`
schema, where every artifact's filename is its id; it is not the rule, and a
schema declaring `id: plan` with `generates: implementation-plan.md` resolves
to `implementation-plan.md`, not `plan.md`.

`openspec instructions apply --change <name> --json` returns `contextFiles`,
mapping an artifact id to an **array** of absolute paths — preferable to
guessing filenames once the CLI path is available. An artifact matching
nothing is **omitted** from `contextFiles` entirely, rather than present with
an empty array (`dist/commands/workflow/instructions.js:270-277`,
`dist/commands/workflow/shared.d.ts:24`, `@fission-ai/openspec@1.11.0`).

The plugin runs `instructions apply` and deliberately **not**
`openspec status --change <n> --json`, even though `status` also reports
per-artifact paths: both compute them through the same
`resolveArtifactOutputs` function, so `status` supplies nothing `apply`
doesn't, at the cost of a second Node start per change. `status`'s own
`artifacts` array is additionally sorted in **topological build order**
rather than the schema's declared order, so it would not even be a correct
source for the positional join above.

**The `openspec` binary.** Probed in order, and cached for the session, taking
the first usable candidate and probing no further:

1. `openspec_bin` from plugin configuration — `config.toml` in the directory
   Herdr injects as `HERDR_PLUGIN_CONFIG_DIR` into every plugin process it
   starts, pane or action alike, no subprocess required. That directory is
   the same one `herdr plugin config-dir herdr-openspec` reports for a human,
   and is also the fallback path the plugin computes for itself when run
   outside a Herdr-started process. `config.toml` also holds `agent_kind` and
   `archived_count`. The agent-name mapping lives separately, under
   `HERDR_PLUGIN_STATE_DIR` — see Herdr integration → Attributing an agent —
   because the plugin writes it and must not write into the directory the
   user hand-edits
2. `openspec` on `PATH`
3. `<nvm root>/versions/node/<version>/bin/openspec`, where the nvm root is
   `NVM_DIR` when it is set to a non-blank value and `$HOME/.nvm` otherwise,
   and version directories are tried newest first, ordered numerically (so
   `v10.0.0` precedes `v9.99.99`) with an unparseable name kept and sorted
   after every parsed version
4. `$(npm prefix -g)/bin/openspec`

A step matches only a candidate that is, following symbolic links, an
executable regular file — a directory named `openspec` does not qualify, and
neither does a file with no execute bit. The winning path is returned exactly
as the chain constructed it, never canonicalized, since it is the name a
later change spawns and a symbolic link is the stable, upgrade-surviving
form. A configured `openspec_bin` that is not usable does not win and does
not end the chain: the remaining steps still run, and the fallback is
recorded as a problem naming the configured path, so a user's typo degrades
visibly rather than either silently substituting a different binary or
failing closed.

Steps 3 and 4 exist because the binary is commonly installed under a Node version
manager, and a plugin pane command does not run through a login shell.
`resolve::openspec_bin` takes the npm prefix as an injected `&dyn Fn() -> Option<PathBuf>`
hook, the same shape by which it and `config` take the process environment as a
lookup closure — that injection is what keeps `resolve` pure, and it survives the
subprocess seam landing rather than being replaced by it. The hook's production
binding is `cli::npm_prefix`: it starts the npm program with the arguments `prefix`
and `-g`, reads its **stdout only**, trimmed — on the reference machine `npm` writes
unrelated shell-plugin noise to stderr — and reports no prefix when the program could
not be started, exited non-zero, or produced empty output.

### Refresh

One recursive `notify` watch on `<repo>/openspec` — `<repo>` the same resolved
repository root `start_collaborators` also hands the `openspec` child as its own
working directory, alongside a one-entry `PATH` overlay prepending the resolved
binary's own parent directory (`seam-resilience` → Decisions 2 and 8) — opened
once at startup and held for the pane's lifetime; `watch::start` itself takes
the already-joined path as a plain argument and stays ignorant of the
repository's own layout. Every
touched path it reports is folded into a **debounce**: a pure
state machine (`watch::Debounce`) that takes `now` as a parameter rather than reading
the clock itself, so its window-boundary behaviour is asserted directly
(`take_due(t0 + 149ms)` is `None`, `take_due(t0 + 150ms)` is `Some(_)`) instead of
through a real sleep. The window is 150ms, capped at one second of total deferral
(`DEBOUNCE_MAX`) so a writer saving more often than that — an agent editing
`tasks.md`, then a spec, then a design doc — cannot defer a batch forever. A real
`Instant::now()` call lives in `watch::RealFsEvents::drain`, which captures it
once per call and reuses it for both the debounce and the `pending_in` value the
next frame's wait consults; `src/agents.rs`'s `RealAgentPoll::drain`
(`agent-polling`) reads a second, on the same terms, for its own one-second
poll schedule.

A batch of touched paths is classified to a `changes::Selection`: a path under a
specific `openspec/changes/<name>/` narrows the selection to that change alone; a
touch to the `changes/` directory itself, to `archive/`, or to anything the
classifier does not recognise widens it to every change (conservative by design — a
wrong "every change" costs one extra CLI cycle that was already going to happen on
the next `list --json`; a wrong "just this one" would silently stop the pane
noticing a change elsewhere). A worker thread confined to `src/refresh.rs` takes a
`Selection` request and answers it twice: first the file-sourced `ChangeSet`
(sub-millisecond, since it is a directory walk), then the CLI-merged one
200–400ms later. The loop applies whichever result is ready on every frame
without ever waiting for either, which is what makes "files paint, the CLI
corrects" a property of two successive frames rather than a synchronous read.
`src/agents.rs` (`agent-polling`) confines the crate's **second** worker
thread the same way, answering an `AgentSnapshot` request instead of a
`Selection` one — `src/refresh.rs` and `src/agents.rs` are the crate's only
two threads, not the one.

Pressing `r` sets the same one-shot request the pane issues automatically at
startup — `Selection::All`, so the CLI corrects every change's numbers once,
whether or not anything was ever touched. The event loop's own wait shortens to
whatever is left of the debounce window (`watch::poll_timeout`) rather than always
sleeping the full 250ms tick, so a pending batch is noticed close to the moment it
becomes due; the tick itself is unchanged.

## User interface

### Responsive layout

A Herdr split pane is frequently 40–60 columns, where a fixed two-column layout is
unusable.

- **100 columns or wider:** two columns — change list left (`Length(40)`), artifact
  detail right (`Min(0)`), so every column gained beyond 100 goes to detail. At
  this width `Enter` and `Esc` still move the route; they select which region is
  emphasised (bold border) rather than which is visible.
- **Narrower than 100 columns:** single column. The list is the root view; `Enter`
  opens detail and `Esc` returns. At this width the route selects which region is
  visible, not merely emphasised.

`Enter` and `Esc` move the route at **every** width when neither the filter mode
nor a filter query is active — the difference is only what moving it does to the
frame. While filtering, `Enter` accepts the query and `Esc` cancels it without
touching the route, and `/` itself moves the route to the list at every width;
`list-view` → Keys states the full layering. Both the `Length(40)`/`Min(0)`
constraints and the underlying every-width route rule are frozen here for
`list-view`, `detail-view`, and `agent-attribution` to inherit.

The two constraints above produce four mandated interiors, one pair per region,
each **16 rows** at the mandated 20-row frame: the **list** region is 38 columns
wide at the wide layout's `Length(40)` column (less two border columns) and 58 at
the narrow layout's 60-column frame (`list-view`); the **detail** region is 78
columns wide at the wide layout's `Min(0)` column — a property of the mandated
120-column frame rather than a constant, since every column gained beyond 120
also goes to it — and 58 at the narrow layout's 60-column frame in the detail
route (`markdown-viewer`, frozen here for `detail-view` and `tasks-tab` to
inherit). The detail region's sixteen rows are further divided by
`layout::split_detail` (`detail-view`): row one the change header, row two the
artifact tab bar, and the remaining **fourteen** rows the content area, whose
own height — not the interior's — is what the scroll clamp is computed
against.

**The Unicode promise, and its limit.** Every width and truncation in this section — the
mandated interiors above, the list row grammar and the detail region's header, tab bar,
and content below — is exact **as ratatui measures it**, not an independent guarantee of
what a terminal paints. `ui::layout::columns`/`truncate_columns` are the crate's one
measure of a rendered length: they split into grapheme clusters and sum each cluster's
cell width through the same public ratatui API `Buffer::set_string` itself calls, so the
pane's arithmetic and the buffer it writes into agree by construction, not by coincidence.
No line the view produces ever exceeds — as ratatui measures it — the region it is drawn
into.

That qualifier is the promise's edge, not a hedge, and it is crossed in exactly two named,
accepted cases, neither detectable from a process writing bytes to a pty and neither
compensated for. A terminal that does **not** compose a zero-width-joiner emoji sequence
paints each constituent emoji separately — six columns for a three-person family — and the
row overruns; a terminal that **does** compose it already agrees with ratatui, which
measures the whole sequence at two columns, so there is no over-reservation to spend
against the non-composing case. And East Asian **Ambiguous** characters — a class that
includes several box-drawing and arrow characters this repository's own artifacts already
use — are painted at two columns by a CJK-locale terminal, where `unicode-width`'s default,
and therefore ratatui's own, says one. Any compensation for either case would be a guess
that breaks the terminal it did not guess wrong for, so the pane makes none.

### List view

One row per active change, then a separator, then the archived changes
(`archived_count` in plugin configuration, five by default) — the real
rendering against the wide layout's 38-column list-region interior
(`Length(40)` less two border columns; the narrow layout's 60-column frame
leaves 58), a `2fa-support` row carrying `w` shown for scale:

```
> 2fa-support                  w [4/9]
  fix-empty-basket               [7/7]
  migrate-ai-sdk-v7                [-]
  -- archived ------------------------
  2026-08-14 add-auth            [7/7]
```

Each row is a selection marker (`>` for the selected change, a space
otherwise), a space, a name field, a space, and a progress cell right-aligned
so its final character occupies the interior's last column:
`[<completed>/<total>]`, or the three characters `[-]` when the change has no
tasks — its cell still ends in the same column as one that has them. An
archived row additionally carries a ten-column date field (`YYYY-MM-DD`, or
ten spaces when the entry is undated) between the marker and the name field.

**The third column is the agent badge**, `agent-attribution`'s addition: one
ASCII character, present only when a live agent is attributed to that row —
`w` `Working`, `i` `Idle`, `b` `Blocked`, `d` `Done`, `?` `Unknown` — placed
between the name field and the progress cell, separated from each by one
space. A row with no attributed agent carries no badge cell and no
separating space, so it renders byte-identically to a pane with no agents at
all. `degraded-states` resolves the collision this paragraph once recorded — a per-change
problem indicator cannot also occupy this same third column — by placing it elsewhere
entirely: the selected change's own problems are named in the **detail region**, above the
tab's own, rather than as a marker in this one-column cell. Below 100 columns the list and
the detail region are separate routes, so reaching a per-change reason costs one keypress;
that is the accepted trade-off of naming the reason at all, against a one-column marker
that would have named nothing.

A cell too narrow for the interior is dropped **whole**, never cut short, in
a fixed order: the badge cell first (reclaiming its separating space too),
then the progress cell (reclaiming its separating space too), then an
archived row's date field (reclaiming its separating space), and then the
row degenerates to the marker-plus-name grammar an active row always has. A
name too long for its field is truncated with a trailing `…`.

**The footer's last hint is `<n> unattributed`**, `agent-attribution`'s other
addition: present only when at least one in-scope agent could not be placed
on any row, dropped first as the width falls since it is the last of the
footer's hints. It is never a per-agent listing and never a `!`-marked
problem row — a shared Herdr session commonly has agents this pane cannot
place, and that is a normal state, not a fault.

Progress comes from the CLI when available and from checkbox counts
otherwise; the rendered list is **file-sourced at every width** until
`live-refresh` wires the CLI correction in, since `list-view` depends only on
`changes-from-files`. The dual-source model above is still the design — this
is what the pane can reach before that later change lands.

### Detail view

The detail region's interior is split into three rows-groups (`detail-view`,
`layout::split_detail`): a header carrying change name, schema, and progress
in the interior's first row; a tab bar built from the schema's artifact list
in the second row, with `1`–`9` / `[` / `]` switching between tabs; and
content below, in the remaining rows, resolved for whichever artifact the
selected tab names.

The tab bar addresses artifacts by **position**, never by id, in the
schema's declared order: `1`–`9` select the first nine positions directly; a
tenth position and beyond carry no digit in their label and are reached only
with `[` and `]`, which step one tab at a time and clamp at both ends. An
artifact list with no entries renders a single `no artifacts` cell rather
than an empty bar. The detail region's interior is blank — no header, no tab
bar, no content — exactly when the visible change list is empty (no
repository, no changes, or a `/` filter matching none); whenever a change
**is** selected, the header and tab bar are always drawn and the content
area always holds at least one line (`detail-view`).

The **tracked-tasks tab** — identified by **position**, from the schema
artifact `ArtifactRef::tracks_tasks` marks, never by id or filename —
renders `ui::tasks`' grammar: a progress bar showing the change's own
`progress` (never a second count of the source), then task groups under
their headings with a `[x]`/`[ ]` glyph per item (`tasks-tab`). Every other
tab is rendered by
`markdown-viewer`'s markdown viewer, whose rendering grammar is: a heading
keeps its `#` markers rather than being distinguished by colour; a paragraph
word-wraps to the interior width, with a soft break starting a new rendered
line rather than being folded into a space; a bullet or ordered list item
carries its marker — numbered from the list's own start value, not from 1 —
and a hanging indent of two columns per nesting level; a fenced or indented
code block, and a raw HTML block, are reproduced verbatim and hard-split at
the interior width rather than word-wrapped or clipped, so a long line never
silently loses its tail; a block quote prefixes every one of its lines,
continuations included, with `> `; a thematic break fills the interior width;
emphasis, strong, inline code, and links become faces on the affected text,
with a link's destination never printed and an image rendering its alt text
in its place; and a construct the parser does not model — a table, a
footnote, strikethrough, a task-list item — renders as its literal source
text rather than being dropped or mangled (see Degraded states). The content
scrolls with `j` / `k` and the arrows at the detail route (`markdown-viewer`).

Tasks are **read-only by design**. Writing a checkbox from the pane would race the
agent editing `tasks.md` in another pane.

### Keys

| Key | Action |
|---|---|
| `j` / `k`, arrows | At the list route: move the list selection, clamped at both ends rather than wrapping; the list scrolls to keep it visible (`list-view`). At the detail route: scroll the detail content by one line, clamped so the stored offset cannot run away (`markdown-viewer`). While filtering, the arrows still navigate whichever the current route uses them for, but `j` and `k` type themselves into the query instead |
| `Enter` | Open change detail, or — while filtering — accept the query without opening detail |
| `Esc` | Dismiss one layer: filter mode with its query when active, else a non-empty query alone, else back to list, else nothing |
| `1`–`9`, `[`, `]` | Switch artifact tab, at **both** routes — the wide layout draws the detail region at the list route too, so a tab press there is immediately visible (`detail-view`). `0` is inert: tab addressing is 1-based. While filtering, all of them type themselves into the query like any other printable key |
| `/` | Start filter mode from either route, moving to the list: printable keys type into the query, `Backspace` deletes, `Enter` accepts, `Esc` cancels, and `Ctrl-C` still quits |
| `r` | Force a full refresh: re-read every change from files, and re-ask the CLI about every one. While filtering, `r` types itself into the query instead, like every other printable key |
| `a` | Launch an agent with `/opsx:apply`. Inert — no call, no problem — with no change selected; refused with a reason when the derived name is already running for this change (see Launch flow, Degraded states) |
| `c` | Launch an agent with `/opsx:continue`, on the same terms as `a` |
| `s` | Launch an agent with `/opsx:archive`, on the same terms as `a` |
| `g` | Focus the running agent for this change (`herdr agent focus`); a change with no attributed agent leaves `g` inert — no call, no problem |
| `q` | Quit — except while filtering, where it types a `q` instead |
| `Ctrl-C` | Quit |

While filtering, `a`, `c`, `s`, and `g` each type themselves into the query
instead of launching or focusing, like every other printable key.

`Esc` at the list root, with no detail open, no filter active, and no query set,
is inert rather than a quit — the layered dismissal above is what determines
whether there is a layer left to dismiss. Only `q` (outside filter mode) and
`Ctrl-C` close the pane.

Action keys, and their footer hints (`a/c/s launch  g focus`), are hidden when
the Herdr socket is unreachable.

## Herdr integration

### Agent status by polling

`herdr agent list` is polled at roughly one-second intervals — **no `--json`
flag**: Herdr 0.8.2 rejects one with exit status 2 and `usage: herdr agent
list` on stderr, even though the command's only output format is already
JSON. The payload is an **envelope**, not a bare array:

```json
{"id":"cli:agent:list","result":{"agents":[ … ],"type":"agent_list"}}
```

Each entry in `result.agents` carries `agent` (the agent **kind**, e.g.
`"claude"`) separately from `name` (the name a user or this plugin gave the
agent, **omitted entirely** when unset) — the two are not interchangeable,
and attribution below keys on `name`, never on `agent`. Herdr's own schema
marks seven fields required — `pane_id`, `tab_id`, `workspace_id`,
`terminal_id`, `focused`, `revision`, and `agent_status` — and omits every
other field when its value is null or default, so `agent`, `cwd`, `name`,
and `terminal_title` are routinely absent from a real payload. This plugin
reads `agent`, `agent_status`, `cwd`, `pane_id`, `tab_id`, `workspace_id`,
`name`, and `terminal_title`; of those, only `pane_id`, `tab_id`, and
`workspace_id` are required for an entry to parse at all, and an
unrecognised `agent_status` decodes to `Unknown` rather than being treated
as a fault.

Polling is chosen over an event hook deliberately. The call is a Unix-socket
round trip costing milliseconds — unlike the Node CLI — and needs no manifest
hook. The confirmed plugin event names are `pane.created`, `pane.closed`,
`pane.exited`, `pane.focused`, the `tab.*` and `workspace.*` families, and
`worktree.created` / `worktree.opened`; agent-status events appear in the binary
but are not confirmed as valid hook targets. An event hook remains available later
as a pure optimisation.

### Attributing an agent to a change

`herdr agent list` is **session-global**: measured live against Herdr 0.8.2, it returns
byte-identical output from three different working directories, including from `/`, and
lists agents whose `cwd` lies in a repository other than the current one. **Every tier
below is therefore scoped to the resolved repository, not tier 3 alone**: an agent is
placed in scope only when its `cwd` is present and is inside (or equal to) the repository
root, by Rust's component-wise `starts_with`, never a textual prefix. An agent carrying no
`cwd` at all — `cwd` is not one of Herdr's seven required fields — cannot be shown to be in
the repository and is out of scope on the same terms as one reported elsewhere. An
out-of-scope agent is invisible to the pane: neither attributed nor counted.

An in-scope agent is then decided by exactly three tiers, evaluated in this order and no
other, and the design refuses to guess beyond them:

1. **Launched by the plugin.** `herdr agent start` takes a name positionally, but a
   change name is not always a legal Herdr agent name
   (`[a-z][a-z0-9_-]{0,31}`) — it may start with a digit, carry illegal
   characters, or simply run past 32 characters. The plugin derives an agent
   name from the change name by a pure, total function: ASCII-lowercase;
   replace every character outside `[a-z0-9_-]` with `-`; collapse runs of
   `-` and trim leading/trailing `-`/`_`; fall back to `change` if nothing is
   left; prefix `c-` if the result cannot legally start an agent name; and,
   past 32 characters, keep the first 27 characters (trimmed of any trailing
   separator) plus `-` and a four-digit lowercase base-36 suffix derived from
   an FNV-1a hash of the whole original change name, so the same change
   always derives the same agent name on every machine. A mapping from the
   derived agent name back to the change name is recorded in plugin-local
   state (under `HERDR_PLUGIN_STATE_DIR`, never beside `config.toml`)
   whenever the derived name **differs from the change name at all** — not
   only when it was truncated. `2fa-support` is only 13 characters but still
   becomes `c-2fa-support` and still needs the mapping to be attributable. A
   derived name already bound to a different change is rebound rather than
   rejected: the most recent launch is the live one. Launch flow's
   `herdr agent start <change>` and `herdr agent prompt <change>` below refer
   to this *derived* name, not the raw change name; correcting those two
   lines to say so explicitly is `agent-launch`'s work, planned from this
   paragraph. Tier 1 is consulted **before** tier 2, and falls through to it
   when the mapping names a change that is no longer in the visible window
   (archived out of range, renamed, or deleted since the launch) — a stale
   record never strands an agent the weaker tier can still place, and never
   invents a change.
2. **Named manually.** Any live agent whose `name` — never its `agent` field, which is
   the agent *kind* (`"claude"` on every live agent measured) — equals a change name,
   byte for byte, is attributed, making `herdr agent rename` a deliberate way to opt in.
3. **Everything else.** An in-scope agent no tier could place is *not* attributed to any
   row. It is reported as a count in the footer — never guessed at, never assigned.

Linked worktrees are a known limitation: Herdr places one at
`<repo-parent>/.worktrees/<repo>-<branch>`, **outside** the repository root, so an agent
working in a worktree of this repository fails the containment test in tier 1 above and is
invisible to the pane. Recorded here rather than fixed, pending a change that reads
`herdr worktree list` deliberately.

### Launch flow

Pressing `a`, `c`, or `s` on a change issues exactly three Herdr calls, in this
order, stopping at the first failure:

```
herdr pane split --cwd <repo> --direction right --no-focus
herdr agent start <derived-name> --kind <kind> --pane <pane_id>
herdr agent prompt <derived-name> "/opsx:apply <change>"
```

`--direction` is **required**, not optional, and no pane argument is given: the
split always targets the currently focused pane. The call's response is an
**envelope**, not a bare pane id:

```json
{"id":"cli:pane:split","result":{"pane":{"pane_id":"…", …}}}
```

`result.pane.pane_id` is what the second and third calls use; a payload that is
not valid JSON, not an object, or missing `result.pane.pane_id` stops the launch
before any agent is started (see Degraded states). `<derived-name>` is the
*derived* agent name from "Attributing an agent to a change" above, never the
raw change name — a change name is not always a legal Herdr agent name — and it
is what both `agent start` and `agent prompt` name **positionally**, as the
agent to act on. The **prompt text itself** names `<change>`, the real change
name, not the derived name: `/opsx:apply <change>` (or `/opsx:continue`/
`/opsx:archive` for `c`/`s`) is an OpenSpec slash-command the launched agent
reads, and OpenSpec knows the change by its real name, never by a Herdr-legal
agent identifier that may be truncated or hashed. The prompt is one
**positional** argument, passed to the program directly with no shell and no
`--wait` flag: the plugin does not wait for the agent to act on it. A failed
call at any of the three carries Herdr's own reported reason (its stderr JSON
error envelope's `code` and `message`, on the same terms "Agent status by
polling" above already documents) rather than a generic failure.

`<kind>` comes from plugin configuration (`agent_kind`, default `claude`);
Herdr supports more than twenty agent kinds. `g` focuses the pane an
attributed agent already runs in — `herdr agent focus <pane_id>` — one call,
resolving `<pane_id>` from the same three-tier attribution above, never a
launch of its own.

### Manifest

`min_herdr_version` stays `0.7.0`: Herdr's own changelog records manifest-declared
actions, managed plugin panes, plugin pane placement, and plugin invocation-context and
environment injection as 0.7.0 additions. `herdr plugin pane focus`, which `open`/
`open-tab` use to avoid duplicating an already-open dashboard, is confirmed working on the
installed 0.8.2 but appears **nowhere** in Herdr's changelog, so its true first version is
unknown; `herdr-file-viewer` (also `min_herdr_version = 0.7.0`, installed locally) still
works around focus with a `pane zoom --on`/`--off` cycle rather than `plugin pane focus`,
suggesting the subcommand may postdate 0.7.0. This plugin does not raise its floor on that
suspicion alone — see "Focus can fail two ways" above for how a Herdr without the
subcommand degrades, which is bounded to one extra dashboard rather than a refusal. A
floor bump, if one is ever measured necessary, belongs to whichever change first measures
a real failure on an older Herdr.

```toml
id = "herdr-openspec"
name = "OpenSpec"
version = "0.1.0"
min_herdr_version = "0.7.0"
platforms = ["macos", "linux"]

[[build]]
command = ["/bin/sh", "scripts/build.sh"]

[[panes]]
id = "dashboard"
title = "OpenSpec"
placement = "split"
command = ["./target/release/herdr-openspec", "ui"]

[[panes]]
id = "dashboard-tab"
title = "OpenSpec"
placement = "tab"
command = ["./target/release/herdr-openspec", "ui"]

[[actions]]
id = "open"
title = "OpenSpec: dashboard"
contexts = ["workspace"]
command = ["./target/release/herdr-openspec", "open"]

[[actions]]
id = "open-tab"
title = "OpenSpec: dashboard (tab)"
contexts = ["workspace"]
command = ["./target/release/herdr-openspec", "open-tab"]
```

The tab action's command is the flat `open-tab` subcommand, not `["open", "--tab"]`: `parse`
stays a total function over a flat token list with no flag state, and the landed `ui --tab`
rejection keeps its meaning rather than becoming an exception to a flag grammar introduced
for one boolean (`plugin-actions` design.md → Decision 1).

### Opening the pane

`open` and `open-tab` are one-shot binary subcommands, not part of the render path — they
never enter raw mode, the alternate screen, or construct a `ratatui` terminal. Everything
below is measured live against the installed Herdr **0.8.2**.

**Invocation context.** An action's process working directory is the **plugin root**, not
the workspace's. The invocation context arrives as `HERDR_PLUGIN_CONTEXT_JSON` — an object
carrying `workspace_id`, `workspace_cwd`, `tab_id`, `focused_pane_id`, and
`focused_pane_cwd` — alongside the discrete `HERDR_PLUGIN_ID`, `HERDR_WORKSPACE_ID`, and
`HERDR_PANE_ID` variables, read purely through one injected lookup (`open::context`). Only
an absent workspace id is fatal; every other field falls back in turn and a context this
plugin cannot read (unset, not valid JSON, or valid JSON that is not an object) is treated
as absent rather than a failure.

**Open or focus.** `herdr plugin pane open` is **not** idempotent — a second call with the
same `--plugin`/`--entrypoint` opens a second pane — so both subcommands run
`herdr pane list` first. `pane list` exposes no plugin ownership field; a plugin pane is
distinguished only by a `label` carrying the manifest pane's `title`, which both `[[panes]]`
entries deliberately share (`OpenSpec`), so `open` and `open-tab` focus each other's pane —
"show me the dashboard" has one answer per workspace. A listed pane counts as this
workspace's dashboard when it carries a string `pane_id`, its `label` matches, and its
`workspace_id` matches; the first match in list order wins.

**The two argument vectors.** A split targets an **existing** pane — the focused one by
default, or `--target-pane <id>` — and fails `invalid_params` with `--workspace` unless
that workspace happens to be focused; `--direction` is optional here, unlike
`herdr pane split`, which requires it. A tab needs no target and passes `--workspace`
instead.

```
open:      plugin pane open --plugin <id> --entrypoint dashboard
                            --placement split --direction right
                            --target-pane <focused pane id> --focus
open-tab:  plugin pane open --plugin <id> --entrypoint dashboard-tab
                            --placement tab --workspace <workspace id> --focus
```

**Neither ever passes `--cwd`.** The first draft of this design did, to make the pane
follow the workspace's own repository rather than the plugin root. Measured live: Herdr
resolves the manifest's *relative* pane `command` against `--cwd` too, not only against the
plugin root, so a workspace directory holding no `target/release/herdr-openspec` binary of
its own makes the whole call fail (`plugin_pane_open_failed`, "Unable to spawn
`<dir>/./target/release/herdr-openspec` because it does not exist") — for every real
workspace, which is exactly the case `--cwd` existed to serve. `ui::run` instead prefers
the workspace cwd from its own injected context (`ui::startup_cwd`, reading the same
`open::context`) over `std::env::current_dir()`, since every process Herdr starts for a
plugin — a `[[panes]]` entry no less than an `[[actions]]` one — receives this same
injected context.

**Focus can fail two ways.** `herdr plugin pane focus <pane_id>` reaches the server in
0.8.2 but appears nowhere in Herdr's changelog, so its first supported version is unknown.
A usage error (`CliError::Failed { code: Some(2), .. }` — plain text on stderr, what a
Herdr without this subcommand produces) warns and falls through to opening once, rather
than refusing on a Herdr the manifest's own `min_herdr_version` still declares supported.
Any other focus failure — including the recoverable case of a pane that closed between the
listing and the focus call — stops the command; invoking the action again recovers it, once
the pane is no longer listed.

**Nothing about the open response is parsed.** Its envelope
(`result.plugin_pane.pane.pane_id`) differs from `pane split`'s (`result.pane.pane_id`);
exit status alone carries success or failure, on the same domain-error-vs-usage-error split
(exit 1 JSON on stderr, exit 2 plain text) every other `plugin`/`pane`/`agent` call in this
crate already carries.

## Degraded states

Every condition renders usable content rather than an error screen:

| Condition | Behaviour |
|---|---|
| No `openspec/` found while walking up | Empty state naming the directory searched |
| `openspec` binary not found | File mode, with a dim `file mode` badge in the header, immediately after the `OpenSpec` label; dropped whole (never truncated) below 18 columns |
| Schema unknown to the CLI | Per-change fall back to file mode. This is real: `learning-tool` declares schema `outside-in-tdd`, which the installed CLI rejects |
| Schema not vendored (no `openspec/schemas/<name>/schema.yaml` locally) | Artifact list empty until the CLI tier supplies it; distinct from the row above, which is the CLI rejecting a schema the plugin already read — both can be true at once for a schema like `outside-in-tdd`. The detail region's tab bar renders `no artifacts` for such a change (`detail-view`) |
| Schema unreadable or invalid (I/O error, or bytes that are not a usable schema) | Artifact list empty, and the reason is named. The detail region's tab bar renders `no artifacts` for such a change (`detail-view`) |
| Schema loads with no tasks artifact (`apply.tracks` matches nothing and no artifact has id `tasks`, or `apply.tracks` is present but wrong-typed even when an artifact has id `tasks`) | No artifact is marked, so every tab renders as markdown and none renders the checklist; no tab is added, removed, or hidden. The task **count** is not deferred: it falls back to counting `<change dir>/tasks.md` directly, matching `openspec list --json`'s own behaviour for such a change, so the pane never shows a pair the CLI would immediately correct |
| No active changes | Empty state; archived changes remain browsable |
| A `/` filter matches no change | Two rows: `No changes match`, then `/` and the query, so the filter that produced the empty state stays visible |
| Artifact file missing | Tab is still shown and renders "No content yet" (`detail-view`) |
| An artifact file exists and cannot be read (permission error, I/O error) | Tab is still shown; a `!`-marked problem line naming the path and the reason is rendered above the content, and "No content yet" is not also shown — the reason is known, and showing both would say two contradictory things about the same tab (`detail-view`) |
| No change is selected (an empty visible list, or a `/` filter matching none) | The whole detail region is blank; the list region already names the empty state, and duplicating it in the detail region would say the same thing twice (`detail-view`) |
| Markdown source holds a construct the parser does not model (a table, a footnote, strikethrough, a task-list item), on a tab **other** than the tracked-tasks one | Renders as its literal source text, one line per source line, rather than being dropped or mangled (`markdown-viewer`) |
| A tasks file exists and yields no task **items** | The tracked-tasks tab renders `No tasks yet`, distinct from `No content yet`, with no heading line even where the source carries headings (`tasks-tab`) |
| A marked tab's artifact resolves to no file while the change's `progress` is non-zero (the `tasks.md` fallback above counted a file the artifact itself did not) | The tab reads `No content yet` and shows no progress bar, while the header one row up still shows the counted pair — the one place the tab's content and the header legitimately disagree (`tasks-tab`) |
| `openspec/changes/` or its `archive/` exists and cannot be read | Empty list for the affected tier, with the reason named on `ChangeSet::problems` and rendered as a leading `!`-marked row of the list, above the change rows; the archive walk is not attempted when the parent read already failed, so a permission error is never reported twice for the same fault |
| An artifact's `generates` pattern falls outside the file path's supported glob subset | That artifact's list is empty and the reason is named; every other artifact on the change still resolves normally, and the CLI tier supplies the correct list when it arrives |
| A `generates` glob crosses a symlinked directory whose target resolves inside the change directory | The plugin does not descend into it, so a file reached only through the link is absent from that artifact's list — a knowing divergence from the CLI, which follows it (`followSymbolicLinks: true`, `dist/core/artifact-graph/outputs.js:93`) and lists the files behind it. The divergence is bounded to the artifact list, never to a change's `progress`, and the CLI tier corrects the list when it arrives. A link whose target resolves **outside** the change directory is a different case, and not this divergence: the CLI fails closed there instead of over-listing, so the plugin quietly skipping it is the more useful of the two |
| A tasks file exists but cannot be read (a directory where a file was expected, a permission error, an I/O error, or invalid UTF-8) | Reported as zero tasks, named in `Tasks::problems`; the CLI's count corrects the pane when it arrives. Invalid UTF-8 is one of two cases where the file path knowingly disagrees with `openspec list --json` — the other is the inside-resolving symlink row above — and here the CLI decodes lossily and still reports a count; every other read failure already agrees with the CLI, which records the same failure as zero tasks too |
| Herdr socket unreachable | Runs as a standalone TUI; the badge column is empty because an unreachable snapshot carries no agents — the view applies no hiding rule of its own — and the action keys and their footer hints are withheld |
| A launch's `pane split` call fails | The launch stops before an agent is started; the reason is recorded and rendered as the list's leading problem row |
| A launch's `agent start` call fails | The pane `pane split` opened is left in place, un-closed — `agent start`'s failure set includes the readiness *timeout*, in which an agent may in fact be starting — and the reason names the agent and the pane |
| A launch's `agent prompt` call fails | The agent is already recorded as running for this change (the derived name is bound before the prompt is sent), so `g` can still focus it; the reason is recorded as the list's leading problem row |
| A launch's derived agent name is already live in the session | Refused before any Herdr call is made: "`<name>` is already running for this change - press g to focus it" |
| `state::record` fails after a successful `agent start` | The start is not undone and the prompt is still sent; the record failure is recorded as its own problem alongside the successful launch |
| `g` on a change with no attributed agent | Nothing happens — no Herdr call, no problem recorded |
| A `pane split` payload is not the measured envelope (a Herdr older than the manifest's `min_herdr_version` floor, `0.7.0`, might return one) | The launch stops before an agent is started, on the same terms as the JSON-decode and error-envelope failures above |
| Pane narrower than 100 columns | Single-column list and detail |
| `config.toml` malformed, unreadable, or a key of the wrong type | The affected key falls back to its documented default while every other key that parsed correctly is still honoured; `Config::problems` names each fallback |
| `agent-names.toml` unusable (malformed, unreadable, or an entry Herdr would reject) | Empty or partial mapping; attribution falls back to the name-equality tier. The read itself is non-destructive — nothing on disk is touched by reading it — but the next successful recording rewrites the file from the entries the read recovered alone, so an entry the parse could not recover does not survive that later write |
| A live agent's `cwd` is absent, or outside the resolved repository root | Neither badged nor counted, at any tier — `herdr agent list` is session-global, so an agent this pane cannot place in its own repository must not badge or count toward it |
| An agent works in a **linked worktree** of this repository | Herdr places a linked worktree at `<repo-parent>/.worktrees/<repo>-<branch>`, outside the repository root, so the agent fails the containment test above and is invisible to the pane — a standing, accepted limitation, not a silent mis-count and not pending any future change |
| A configured `openspec_bin` that does not name a usable binary | Falls through to the remaining probe steps rather than winning or ending the chain; the fallback is named in `BinResolution::problems` rather than being silent |
| A schema declares the same artifact id at two positions | The CLI rejects such a schema outright (`Duplicate artifact ID`), so a change using it is permanently file-mode — this crate's own parser accepts the duplicate, as `schema-artifacts` requires, so the plugin's "usable" is strictly wider than the CLI's |
| `openspec list --json` reports a repository root other than the one this plugin resolved | The whole CLI result is discarded, not merged: `ui::start_collaborators` gives the `openspec` child the same resolved repository root as its own working directory (`seam-resilience` → Decision 2), so the CLI's own upward walk from that directory ordinarily agrees with what the plugin already resolved — a disagreement here is no longer an accepted consequence of a `current_dir`-less seam, but a signal that something else is wrong (a configured `openspec_bin` answering for a different tree, among other causes), and the guard stays in place as a safety net rather than being removed |
| An `openspec` command exits non-zero | openspec's own diagnostic goes to stdout and is unavailable; a stderr line, when present, is carried — including a shim whose `#!/usr/bin/env node` interpreter cannot be found or cannot run, which exits non-zero (127, or another code on a signal death) on exactly this row, not a row of its own |
| A `herdr agent list` call exits non-zero, or the program cannot be started at all | The reverse of the row above: Herdr writes its diagnostic to **stderr** as a JSON error envelope (`{"id":…,"error":{"code":…,"message":…}}`), with an empty stdout, so the reason **is** available and is recorded on `AgentSnapshot::problem` — never rendered as a `!`-marked row, since an unreachable socket is the silent "runs as a standalone TUI" state above, not an error |
| The Herdr agent poller's outstanding `agent list` call passes the stall threshold (`agents::STALL_AFTER`, five poll intervals) with no answer | Distinct from the row above: the badges and the `reachable` flag are withdrawn — a socket that has stopped answering entirely is not the silent "runs as a standalone TUI" state — and the reason is named once as a leading `!`-marked row directly below a launch problem row and above any preserved startup row (design.md → Decision 4); the action keys, offered only while reachable, withdraw along with the badges so a press cannot queue behind the stuck call. A later answer restores both |
| The filesystem watcher will not start (`notify` refuses the watch, or the repository root cannot be watched) | The pane runs unwatched rather than refusing to start: the reason is named as a leading `!`-marked row of the list, ordered below a launch problem row, a stalled-poller row, and any preserved startup problem when present, and above every change-set problem row (`ui::list::rows`' five problem sources, in order: launch, stall, startup, live, change-set — `seam-resilience`), and `r` still forces a full refresh — the one path to a corrected list on a machine where watching does not work |
| `openspec/` is removed while the watcher runs | The next filesystem read reports an empty change set with no problem of its own — a missing `openspec/` directory means "not an OpenSpec repository", the same as it always has. The watcher's own read failure (the event channel disconnecting) is what the pane actually shows, as the same leading `!`-marked row above (ordered below a launch, stalled-poller, or preserved-startup problem row when present, above every change-set problem row), and the loop keeps drawing regardless — a watch failure is never treated as a reason to stop |
| The refresh worker stops answering (its worker thread has exited) | The change set already on screen — the last known-good result — stays exactly as it was: the reason is named once as a leading `!`-marked row (the same `live`-tier position as the two watcher rows above: below a launch, stalled-poller, or preserved-startup row, above every change-set problem row), and every drain after that returns nothing rather than repeating the row or emptying the list; `r` still sends a request, which the dead worker silently discards |
| A CLI cycle fails after the worker already sent its file-sourced result | The pane keeps the numbers the file read produced; the failure is not silently dropped, but nothing overwrites what is already on screen with a blanker state |
| A touched path is classified to the wrong change, or conservatively to every change | Cosmetic only: a change's **progress** always comes from the same fresh `openspec list --json` call every cycle makes regardless of selection, never from the per-change cache, so a mis-classified path costs at most one cycle of stale **artifact** content — never a stale progress pair — and `r` corrects the rest immediately. This is stated so a later change does not "fix" the cache by making progress come from it, which would reintroduce exactly the staleness this design avoids |
| `open`/`open-tab` invoked with no Herdr context at all (no workspace id) | Exits 1 immediately, naming `HERDR_WORKSPACE_ID` and that the command must be invoked from Herdr; no Herdr call is made |
| `open`/`open-tab`'s `pane list` call fails or its payload is unparseable | Warns (stderr) and still attempts to open a pane, rather than refusing — the plugin cannot know whether a dashboard already exists, and a socket too broken to list is one whose open call will report its own reason |
| `open`/`open-tab`'s `plugin pane focus` call fails with a usage error (code 2) | Warns and falls through to opening once — refusing would fail closed on a Herdr the manifest's `min_herdr_version` still declares supported |
| `open`/`open-tab`'s `plugin pane focus` call fails with a domain error, or `plugin pane open` itself fails | The command stops; the reason — `herdr exited with code <n>: <stderr>`, `herdr_reason`'s own formatted prefix, never Herdr's raw message byte-for-byte — goes to stderr and the process exits 1 |
| The dashboard process opened by `open`/`open-tab` finds no workspace cwd in its own injected Herdr context (`ui::startup_cwd` returns `None`) | Falls back to `std::env::current_dir()` — the plugin root — and renders whatever `openspec/` (if any) is found there, exactly as a `--cwd`-less direct `herdr plugin pane open` always has; not a refusal |
| A change is archived between the worker's `list --json` call and its file walk | The merged `ChangeSet` holds it in both `active` (the CLI's stale answer) and `archived` (the fresh file walk) for exactly one refresh cycle, with no problem recorded — `merge` does not consult `files.archived` when deciding whether a CLI change is CLI-only, because doing so would invert the dual-source model's rule that the CLI corrects the files, for a state that self-corrects once the next `list --json` no longer names the change (design.md → Decision D6) |

### No terminal is not a degraded state

`ui` refuses to start at all when stdout is not a terminal, exiting status 3 with a
message naming `herdr-openspec` and the words `not a terminal`. `open` and `open-tab` are
not covered by this either: they are one-shot commands with nothing to render, so their
degrade is a named reason and an exit status, never usable content — the same as every row
above, on different terms than `ui`'s. This is deliberately
outside the table above: every row there is a precondition the pane still renders
*something* for, but a TUI has nothing to render into a pipe — there is no content
to degrade to. Treating it as a table row would make the section's opening sentence
("every condition renders usable content rather than an error screen") false, and
`PRD.md` → Success criteria, which repeats the same claim, false with it.

## Testing and quality gates

### Unit-tested modules

Each is a pure transformation, tested without a TUI. `cli` is the one exception: it
is tested against scratch `#!/bin/sh` programs rather than the real `openspec`,
`herdr`, or `npm`, because performing a spawn is the one thing it exists to do.

- `cli::OpenspecCli`, `cli::HerdrCli`, and the `npm prefix -g` probe — the traits'
  contract (stdout returned verbatim, stderr excluded, a non-zero exit or an
  absent program becomes an error rather than a panic) and the probe's
  trim/decode rules, proved against scratch `#!/bin/sh` programs built under
  `std::env::temp_dir()` so the suite passes with `openspec`, `herdr`, and `npm`
  all unresolvable
- `changes::from_files` and `changes::from_cli` — both produce the same `Change`
  type, from fixture trees and fixture JSON respectively
- `schema::select` and `schema::parse` — which schema name applies (a change's
  own override, the project's, or the default), and `schema.yaml` to ordered
  tabs plus the `apply.tracks`-then-id-`tasks` rule for the tasks artifact
- `tasks::count`, `tasks::parse`, and `tasks::read` — markdown checkboxes to
  flat counts and to grouped items, both by the OpenSpec CLI's own counting
  rule; `read` is the filesystem edge, tested against a scratch directory
  tree, not a faked filesystem layer
- `agents::attribute` — live agents, the repository root, the change names, and the
  plugin-local mapping to per-change badges and one unattributed count, covering all
  three tiers including the deliberate non-attribution case
- `resolve::find_repo` and `resolve::openspec_bin` — the upward walk for
  `openspec/` and the four-step binary probe chain, both tested against a
  purpose-built scratch directory tree under `std::env::temp_dir()`, not a
  faked filesystem layer
- `watch::classify`, `watch::invalidate`, `watch::Debounce`, and
  `watch::poll_timeout` — pure functions over values and an **injected**
  instant, never the real clock; `watch::start` and `RealFsEvents` are the
  one filesystem edge, tested against a real `ScratchDir` and against a path
  that does not exist
- `refresh::start`, `refresh::none`, and the worker body — one of the
  crate's **three** worker threads, tested through a `#[cfg(test)]` constructor
  (`worker_for_test`) that hands the test the worker's own result and exit
  channels directly, with every assertion made **after** a `recv_timeout`
  returned an item, never after a fixed sleep
- `agents::parse_list`, `agents::poll_once`, and the `AgentPoll` seam
  (`agent-polling`) — the reference envelope and every unusable-shape
  reason for the parse; the four-way mapping from a CLI outcome to an
  `AgentSnapshot` for the poll; and the worker — the crate's second
  thread, confined to `src/agents.rs` and reached only through
  `cli::HerdrCli` — tested through its own `#[cfg(test)]` constructor
  (`poller_for_test`), on `worker_for_test`'s terms
- `ui::layout`, `ui::app`, `ui::list`, `ui::detail`, `ui::markdown`,
  `ui::tasks`, `ui::view`, `ui::driver`, `ui::terminal`, and `ui::mod`'s
  `load` and `read_artifact` functions — the breakpoint and frame split,
  `Dashboard` and key handling, the change-row grammar, the detail
  region's header/tab-bar/content grammar (`ui::detail` — plain data, no
  I/O, parameterised by width; see Architecture rules), markdown source to
  plain-data lines of faced segments (`ui::markdown` — confined to one
  module and checked for it; see Architecture rules), the tracked-tasks
  tab's checklist-and-progress-bar grammar (`ui::tasks` — plain data, no
  I/O, parameterised by width, on the same terms as `ui::detail` and
  `ui::markdown`; the tab is chosen by `ArtifactRef::tracks_tasks`, never
  an id or filename, and the bar renders `Change::progress` rather than
  recounting the source), the render seam proper, the draw-then-wait event
  loop, startup state from files, and the one artifact-read binding.
  `ratatui::backend::TestBackend` stands in for the rendering surface and
  a recording `TerminalOps` double stands in for the terminal; no test
  constructs the real terminal implementation
- `launch::decide`, `launch::pane_id`, `launch::run_request`, and `launch::settle` — the pure
  launch policy (no `Dashboard`, no `ChangeSet`, no fixture), the `pane split` envelope parse,
  the three-call `pane split` → `agent start` → `agent prompt` sequence (with `state::record`
  run between the last two) driven against a `FakeCli`, and the bounded wait for a launch
  still in flight when the loop exits; and the worker — the crate's third worker thread,
  confined to this module and reached only through `cli::HerdrCli` — tested through the
  `Launcher` trait's `RealLauncher`/`NoLauncher` implementations. `launch::Outcome` carries at
  most two problems (a `state::record` failure and an `agent prompt` failure), never silently
  dropping either
- `open::context`, `open::existing_pane`, `open::open_args`/`open::focus_args`, `open::run`,
  and `open::placement_for`/`open::report_output` — the invocation context read through one
  injected environment lookup, the pane-list match against `DASHBOARD_LABEL` and the
  workspace id, the exact `plugin pane open`/`plugin pane focus` argument vectors, the
  list-then-focus-or-open driver run against a `FakeCli`, and the two pure decisions
  `src/main.rs` would otherwise make itself
- `config::config_dir` and `config::load` — the configuration directory resolved from an
  injected environment lookup (`&dyn Fn(&str) -> Option<String>`, never `std::env::var`
  directly), and `config.toml` read into a `Config` with a documented default for every key,
  so a missing, empty, or malformed file never fails the caller
- `state::state_dir`, `state::agent_name`, and `state::read`/`state::record` — the state
  directory resolved from the same kind of injected environment lookup as `config::config_dir`,
  the six-step derivation of a Herdr-legal agent name from a change name, and the
  read-then-rewrite of `agent-names.toml` that never fails, panics, or loses a well-formed
  neighbour when one entry is malformed

### View tests

Views render into a ratatui `TestBackend` and assert on the resulting buffer, at
both 60 and 120 columns so the responsive breakpoint is genuinely covered.
Three width pairs matter, at three different tiers, and each is asserted
directly rather than left implied by the frame pair alone: the frame itself
at 60 and 120; the list region's interior at 38 and 58 (`ui::list`); and the
detail region's interior at 58 and 78 (`ui::markdown`, `ui::tasks`,
`ui::view`, and `ui::detail`, whose header, tab-bar, and content-line
grammar is asserted at both widths directly, with no exemption). No `ui::`
test starts a thread, opens a filesystem watch, or reads the system clock:
the live tier's two collaborators (`watch::FsEvents`, `refresh::Refresher`)
are always replaced by the two synchronous, thread-free doubles in
`crate::testutil` (`ScriptedFs`, `RecordingRefresher`), except for the one
acceptance scenario that deliberately opens a real watch and asserts only
byte-identity, never a timing-sensitive claim.

### Fixtures

Three mechanisms, not checked-in fixture repositories: no change in the roadmap
has a use for one. `changes::from_files` needs an empty directory, a
symbolic link (dangling and not), and a directory at mode `0o000`, none of
which git can store faithfully; and every view **test** performs no I/O at
all, building a `ChangeSet` or `Config` value directly rather than opening a
repository — `render` itself still takes a value, never a path, so the render
seam stays pure. The one exception, by design rather than by drift: an
outer-loop composition test per change may open a real directory through
`ui::load`, because that is the one path no unit test crosses; `list-view`'s
own such test lives in `ui::tests::load::`, never in a view module.

- **Run-time `crate::testutil::ScratchDir` trees**, built fresh under
  `std::env::temp_dir()` for every test that needs a real filesystem edge —
  `config`, `state`, `resolve`, `schema`, `tasks`, and `changes` all use this,
  and it is how `changes::from_files`' unreadable-directory and symbolic-link
  scenarios are reached at all.
- **`include_str!` corpora** for pure parsers whose input is bytes, not a
  directory tree — `tests/fixtures/tasks/` holds the markdown fixtures
  `tasks::count` and `tasks::parse` are proven to agree on.
- **A committed tool-output snapshot**, `tests/fixtures/build-graph.txt` — the
  resolved dependency graph, one `<name> <version>` line per package per
  supported triple, compared against a live `cargo tree` run. Neither a
  `ScratchDir` tree nor an `include_str!` corpus: it pins what `cargo`, not this
  crate, produces.

### Doc-conformance checks

- Module map ↔ `src/lib.rs`'s declared `pub mod` set.
- § Unit-tested modules ↔ every declared module named as a `<name>::` token.
- The worker-thread count ↔ `src/` files whose production slice names both `thread::spawn` and `mpsc`.
- The documented MSRV ↔ `Cargo.toml`'s `rust-version`.
- Every non-`cargo` program `make check`'s path invokes ↔ `README.md` and `AGENTS.md` (the `Makefile` walk and the `scripts/gates/` interpreter scan).
- `SPEC.md`'s fenced manifest transcription ↔ `herdr-plugin.toml` (values and `[[…]]` order).
- `openspec/config.yaml`'s injected `context` ↔ the repository (the fixture claim and every `check:` prerequisite target).
- A claim with no second site is argued in review, not checked.

### Gates

Every gate command is written once, in the `Makefile`. Locally, `make check` composes
five gates, in this order, and stops at the first failure:

| Target | Command |
|---|---|
| `fmt-check` | `cargo fmt --all -- --check` |
| `lint` | `cargo clippy --all-targets --all-features -- -D warnings` |
| `gates` | one invocation line per file under `scripts/gates/` |
| `test` | `cargo test --all-features` |
| `coverage` | `cargo llvm-cov --fail-under-lines 80`, plus a production-slice floor computed from its JSON export |

`gates-full` (`DEPS_FULL=1 /bin/sh scripts/gates/deps.sh`) is deliberately **not**
composed into `check`: it rebuilds the crate several times over — once for the release
binary, once per dependency removed — so it runs instead in its own CI job on every push,
and that cost never lands on a local `make check` or on the per-platform `check` runs.

`cargo-llvm-cov` is chosen over `tarpaulin`, which is Linux-first and unreliable on
Apple Silicon. CI invokes the same targets individually rather than the composite —
`make fmt-check`, `make lint`, and `make gates` and `make test` on both `ubuntu-latest`
and `macos-latest` with `Swatinem/rust-cache`, `make coverage` once on Linux, and
`make gates-full` in its own Linux-only job.

Two one-time setup steps are required for local development — `rustup component add
clippy` and `cargo install cargo-llvm-cov` — since CI obtains `clippy` from the
toolchain action and `cargo-llvm-cov` from `taiki-e/install-action`.

`python3` is required too, for the hygiene-gate tier's own scripts and for the
production-coverage floor script — no installable component, since it ships with the OS on
both CI runners and the reference machine.

## Build and distribution

Herdr installs a GitHub-managed plugin by cloning the repository at a resolved
commit and running the `[[build]]` step in place, after install confirmation and
before registering the plugin. There is no release process to satisfy:
`herdr plugin install <owner>/<repo>` is sufficient. `herdr plugin link .` — used
for local development — does **not** run build commands; the local author builds
the working tree themselves (`make build` or `scripts/build.sh`) before or after
linking.

The build step is `scripts/build.sh` rather than a bare `cargo build --release`,
because Herdr may be launched without `~/.cargo/bin` on `PATH` — a GUI or
login-less launch — in which case a direct cargo invocation fails even though Rust
is installed. The script sources `~/.cargo/env` when present, reports clearly if
cargo is still missing, and then builds.

Before publishing, that script gains a fast path: download a version-matched
prebuilt binary from GitHub Releases, verify its SHA-256, and fall back to
compiling on any miss. That removes the Rust toolchain from the install
requirements. Registry listing at `https://assets.herdr.dev/plugins/index.json` is
a separate submission, independent of releases.
