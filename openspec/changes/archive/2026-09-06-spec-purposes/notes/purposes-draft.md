# Planning-time Purpose drafts

Derived at ff time by reading each capability's own requirements. **Reference, not
authority**: task group 2 re-reads the capability before writing, and may improve on the
draft. Only the 26 capabilities carrying the `TBD - created by archiving change`
placeholder are to be edited; the other 14 already have written Purposes and are left alone.

# Drafted Purposes — capabilities 1–20 (alphabetical)

Note: seven of these twenty (agent-attribution, agent-launch, agent-list, agent-poller,
change-artifacts, change-enumeration, change-model) already carry a real, non-placeholder
Purpose in the repo. Drafts for them are given below as condensed alternatives only; they
are NOT TBD and need no edit unless the parent wants a shorter form.

### agent-attribution
Maps each live Herdr agent onto the change it is working on, through exactly three tiers —
the plugin-local launch mapping, byte-exact equality of an agent's `name` against a change
name, and a plain count of everything else — with every tier first scoped to the resolved
repository so a session-global `herdr agent list` cannot badge this repo's rows with another
repo's agents. It also fixes what happens when several agents land on one change (a single
badge chosen by `Blocked` > `Working` > `Idle` > `Done` > `Unknown`, with the winning agent's
pane carried alongside it for `g`), and keeps the whole derivation pure, recomputed per frame
and keyed by change name so nothing drifts when a refresh reorders the list. Fetching the
agents belongs to `agent-list` and `agent-poller`; acting on them belongs to `agent-launch`.

Requirements it covers: Attribution is a pure, total function that refuses to guess; Every tier is scoped to the resolved repository; Tier 1 resolves an agent name through the plugin-local mapping; Tier 2 matches an agent name against a change name exactly; A change's badge is the highest-precedence status among its agents; Attribution is derived per frame, keyed by name, and stored nowhere


### agent-launch
The only place in the plugin where a keypress causes an outward, side-effecting action:
`a`/`c`/`s` start a coding agent on the selected change with `/opsx:apply`, `/opsx:continue`
or `/opsx:archive`, and `g` focuses the pane of the agent already badged on it. A launch it
allows is exactly three sequential `herdr` calls — `pane split` for a pane id, `agent start`
under a derived Herdr-legal name capped at 32 characters, then `agent prompt` carrying the
real change name — each fed by the previous call's output, run on the crate's third worker
thread behind the `HerdrCli` seam so `agent start`'s readiness wait never blocks the draw
loop. Refusals and failures are named rather than silent: the decision function declines with
a reason before it ever reaches Herdr, and a failed call stops the launch there and renders
Herdr's own message as a leading problem row.

Requirements it covers: The launch decision is a pure, total function that refuses before it reaches Herdr; The launched agent's name is Herdr-legal, capped at 32 characters, and recorded whenever it differs; A launch is exactly three Herdr calls, in order, the second and third carrying the first's output; The pane id is read from `pane split`'s envelope, and an unusable payload stops the launch; A failed call stops the launch at that call and carries Herdr's own reason; `g` focuses the pane of the agent whose status the badge shows; The launcher seam is a trait whose every method is non-blocking, and the worker is the crate's third thread; The launch keys are offered only when the socket is reachable, and type themselves while filtering; A launch failure renders as a leading problem row and is replaced, never grown; A real keypress reaches the three Herdr calls in the shipped composition root


### agent-list
Covers one question asked of Herdr and the decoding of its answer: `herdr agent list`, spawned
through the `HerdrCli` seam with exactly `["agent", "list"]` and no `--json` flag, and its
`{"id": …, "result": {"agents": […]}}` envelope turned into `Agent` values. The degradation
rules live here too — absent optional fields survive, an entry with no identity is skipped, an
unusable payload names its reason and yields no agents, and a failed run is an unreachable
socket carrying the program's own text, a standing state rather than an error. When and how
often the question is asked is `agent-poller`'s business, and what the answers mean for a
change is `agent-attribution`'s.

Requirements it covers: The agent list is asked for with exactly two arguments and no JSON flag; A successful list is an envelope, not a bare array; Absent optional fields degrade, and a missing identity skips one entry; An unusable payload names its reason and yields no agents; A failed run is an unreachable socket carrying the program's own reason


### agent-poller
Supplies the cadence around `agent-list`: a non-blocking `AgentPoll` seam, a poll at roughly
one-second intervals that reports its own deadline so the render loop can wake for it, the
crate's second worker thread reaching Herdr only through that seam, and the `Dashboard` field
that holds the latest `AgentSnapshot`. Only the plumbing is specified — nothing here renders
the snapshot or interprets it — and the composition root that wires it is driven by a test
rather than left to a reviewer's reading.

Requirements it covers: The poller seam is a trait whose every method is non-blocking; The poll fires at roughly one second and reports its own deadline; The worker is the crate's second thread and reaches Herdr only through the seam; The dashboard carries the latest snapshot and nothing renders it; The composition root is driven by a test, not only read by a reviewer


### artifact-content
Governs what the detail region's body actually holds: `Dashboard::sync_detail` resolves the
selected `(change directory, tab)` pair into text by reading the artifact's files through a
single injected reader closure — one production binding, `ui::read_artifact`, so no view file
ever names a filesystem API — concatenating multiple paths in resolution order, caching on the
key so an unchanged selection re-reads nothing, and re-reading when a live refresh forces it
without throwing a mid-document reader back to line one. It also fixes what the content area
shows: problem rows for files that failed to read, then either the markdown body or, for the
schema's tracked-tasks artifact, the checklist body, and `No content yet` only when there is
neither content nor a reason. Which tabs exist is `artifact-tabs`', how far the body scrolls is
`detail-scroll`'s, and the two bodies' own grammars belong to `markdown-render` and
`tasks-checklist`.

Requirements it covers: The artifact read is an injected collaborator, confined to one binding; `Dashboard::sync_detail` resolves the selected tab's content; The content area renders the artifact, its problems, or `No content yet`


### artifact-tabs
Owns the row of tabs across the top of the detail region: one cell per entry of
`Change::artifacts` in the schema's declared order, addressed strictly by position so
duplicate artifact ids stay two separately selectable tabs, labelled `"<n> <id>"` for the nine
positions a digit key can reach and by bare id beyond that, windowed to whole cells that always
keep the selected tab on screen, and reduced to a single `no artifacts` placeholder when a
change declares none. It also defines where the bar sits — the second interior row of the
detail region, via `split_detail`'s header/tab-bar/content split — and the keys that move it:
`1`–`9`, `[` and `]` outside filter mode, inert rather than clamped on an out-of-range digit,
resetting the scroll only when the tab actually changed. Reading the selected tab's file is
`artifact-content`'s.

Requirements it covers: The tab bar is built from the schema's declared artifact order, addressed by position; The tab bar is a window of whole cells that always contains the selected tab; The tab bar is drawn into the detail region's second interior row; `1`–`9`, `[`, and `]` switch the artifact tab


### change-artifacts
Turns one change directory into the artifact paths a tab renders and the task count a list row
shows, joining four upstream capabilities: the schema that applies to this change, the artifact
list that schema declares, the checkbox rule, and the task-file parser. The join itself is what
is specified — resolving each artifact's `generates` value (literal or glob, a glob exactly when
it contains `*`, `?` or `[`) against the change's own directory, in ascending path order, and
summing the tasks artifact's files into one `Progress` by the CLI's own rule with a `tasks.md`
fallback, so the row's number does not move when the CLI result lands. Resolution and counting
read the filesystem and never write to it.

Requirements it covers: Each change resolves its own schema, and the project file is read once; Artifact paths come from `generates`, never from the artifact id; A `generates` value is a glob exactly when it contains `*`, `?`, or `[`; A supported glob resolves to every matching regular file, in ascending path order; An unsupported glob shape records a problem and resolves to no files; Task progress sums the tasks artifact's files and falls back to `tasks.md`; Artifact resolution and counting read and never write; The tracked-tasks artifact is marked at its schema position


### change-enumeration
Decides which directories on disk are changes at all: active ones from `openspec/changes/`
(everything but `archive`) ordered by name in byte order, archived ones from
`openspec/changes/archive/` with their date prefix split off the name and ordered newest first
with undated entries last, truncated to the configured window after ordering. The active rule
is copied from `openspec list --json` so the file path and the CLI path agree on which rows
exist; the archived ordering has no CLI equivalent to copy and is the plugin's own, written
down here. A missing or unreadable changes directory degrades to an empty list rather than
failing, and enumeration never writes.

Requirements it covers: An active change is any directory under `openspec/changes/` other than `archive`; Active changes are ordered by name ascending, in byte order; An archived directory's date prefix is split off its name; Archived changes are ordered newest first, with undated entries last; `archived_count` truncates the archived list after it is ordered; A missing or unreadable changes directory degrades rather than failing; Enumeration reads and never writes


### change-merge
Reconciles the two producers of a change set into the one the pane renders: `changes::merge`
pairs active changes by name, layers the CLI's schema, progress and artifacts over the file
tier's while keeping the file tier's `dir`, passes archived changes through untouched, and
re-sorts the union so a CLI-only change lands in name order rather than at the end. Its hardest
rule is the artifact join, which is positional — never by path, which the two producers
canonicalize differently, and never by id, which a schema may legally repeat — with six stated
outcomes for empty, mismatched-length, and disagreeing lists, and the `tracks_tasks` flag
travelling with whichever whole list survives. Problems from both sides are kept in full rather
than dropped when a field is superseded. Producing either input is `change-enumeration`'s and
`cli-changes`' work; this is only the join.

Requirements it covers: The merge layers CLI results over the file result, by name; Artifact lists are joined by position, never by path and never by id; Merged problems are kept in full, never dropped; The merge is a third construction site, bound by the same gate; The positional join carries the tracked-tasks flag with its artifact


### change-model
Defines `Change` and `ChangeSet`, the value every consumer of this plugin reads: one unit of
OpenSpec work reduced to a name, a location, active-or-archived status, the applicable schema,
each artifact's path on disk, task progress, and the problems collected while producing those
answers — with no derived and no source-specific state. It exists because the value has two
producers that will never merge, `changes::from_files` painting the pane immediately and
`changes::from_cli` correcting it moments later, so it also specifies the mechanism that makes
a field one producer fills and the other silently defaults fail to compile rather than change
a number under a reader's eyes. Reconciling the two producers' output is `change-merge`'s job,
not this one's.

Requirements it covers: A change is one total value carrying identity, artifacts, and progress; `Change` carries no derived and no source-specific state; A change's artifacts are the schema's, in the schema's order, resolved to paths; Both producers of a change build every field, enforced at compile time; Archived changes are file-sourced only, and the two lists stay separate; Producing changes never fails and never panics


### change-rows
Fixes the text of every line the Changes region can hold and the order they appear in: launch
problems, then refresh problems, then change-set problems, then the active changes, then an
archived separator emitted only when archived rows follow it, then those rows. The row grammar
itself lives here — the selection marker, the padded-or-ellipsised name field, an archived row's
ten-column date field, the optional one-character agent badge, and the right-aligned `[n/m]` or
`[-]` progress cell — together with the fixed order in which whole cells are dropped as the
width falls, and the exact wording of every empty and degraded body state, from
`No changes yet` to the three-row no-repository block. Rows are pure and never re-order what
the `ChangeSet` gave them; which slice of them is on screen is `list-selection`'s and which
survive the query is `list-filtering`'s.

Requirements it covers: The list region's interior holds one row per change, in the order the `ChangeSet` gives; Archived changes sit below a separator and carry their date; The list region names every empty and degraded body state; Rows are confined to the list region


### ci-workflow
Specifies the repository's single GitHub Actions workflow, `.github/workflows/ci.yml`: its
three triggers with no path filters, a ref-keyed concurrency group that spares `main`, a
`check` matrix over `ubuntu-latest` and `macos-latest` with `fail-fast` off, a Linux-only
`coverage` job, per-job toolchain installs and caching that can only change a run's duration,
and one aggregate `ci` job that every other job feeds so branch protection has a stable status
name. Its central constraint is that CI defines no command of its own — every gate is invoked
as `make <target>`, no `run:` step may mention `cargo` or restate a threshold, and no `env:`
mapping may alter a gate through a door the guard cannot read — which is what makes
`quality-gates`' claim of local/CI parity checkable rather than aspirational. The gates
themselves are defined in the Makefile, not here.

Requirements it covers: A single workflow runs on pushes to `main` and on pull requests; Both supported platforms run the format, lint, and test gates; CI invokes every gate through `make`, so no command is written twice; Coverage runs exactly once, on Linux, at the Makefile's floor; Each job installs the toolchain it needs rather than inheriting one; Caching speeds a run up and never changes its outcome; One aggregate status check reports the whole run; The workflow is read-only and uses no secrets


### cli-changes
Describes the second producer of a change set, `changes::from_cli`: the exact two argument
vectors it may run through the `OpenspecCli` seam (`list --json` once, then
`instructions apply --change <n> --json` per change), what it takes from each payload —
progress from the list envelope rather than apply's differently-computed figure, artifacts
placed at their schema positions from `contextFiles`, all seven `Change` fields named
explicitly — and its own byte-order re-sort rather than the CLI's locale-collated `--sort`.
Everything else here is refusal and degradation: no mutating or `status` vector is ever run,
an envelope reporting a different repository root is discarded whole, and every failure from an
absent binary to one change's malformed payload leaves the affected changes out of the result
with a named problem instead of failing closed. How this result meets the file-sourced one is
`change-merge`'s.

Requirements it covers: Two commands produce the CLI's view of the active changes; The list payload is an envelope, and progress comes from it; The CLI's active list is re-sorted by name in byte order; A change's artifacts are placed by schema position from `contextFiles`; Every one of `Change`'s seven fields comes from CLI data; Every CLI failure degrades to the file result and names itself; A CLI answering for a different repository is discarded whole; Producing changes from the CLI writes nothing; The CLI producer marks the same tracked-tasks artifact


### dashboard-loop
Holds the render seam itself: `Dashboard`, the plain twelve-field state value that carries no
geometry, no terminal handle, no watcher, no thread and no `Default`, so every view is a pure
function of it and every construction site fails to compile when a field is added; `action_for`,
the total mapping from a terminal event and the filter mode to one of seventeen actions;
`run_loop`, which on each iteration hands off a pending launch, drains the four non-blocking
live collaborators, syncs the detail, draws, normalises the scroll and only then waits; and
`ui::load`, which builds the startup dashboard from files alone so the pane opens with a full
change list on a machine that has no `openspec` binary. It is also where the purity boundary is
enforced by name — eight files under `src/ui/` that may name no filesystem, process,
environment, network or standard-I/O API. What each action then means to a region belongs to
that region's capability; this specifies only the state, the dispatch, and the order.

Requirements it covers: `Dashboard` is a plain state value with no rendering and no I/O; Key handling is a pure, total function over events; The loop draws before it waits and stops when quit is set; Startup state is read from files only


### detail-header
Specifies the one bold row at the top of the detail region and its grammar: the selected
change's name in a padded-or-ellipsised field, its schema in parentheses, and the same
right-aligned progress cell the list rows use — one implementation, so the header and the row
can never disagree about a change's progress — with the schema cell and then the progress cell
dropped whole at named width boundaries so the name is always what survives. It also fixes when
the row is drawn at all: whenever the detail region is, and never when the visible list is
empty, where the whole interior is left blank rather than repeating the empty state the list
region already names. An archived change's header shows its stripped name and no date field;
the date belongs to `change-rows`.

Requirements it covers: The detail region's first row names the selected change; The header row is drawn into the detail region's first interior row


### detail-scroll
Answers where in a long artifact the reader is and which slice of it reaches the buffer: the
`detail.scroll` offset as a user-controlled position, `j`/`k`/arrows moving it by a line at the
detail route while the same actions move the list selection at the list route, every route move
resetting it to the top, and `layout::scroll_offset` deriving the drawn window on every frame
from the current content area rather than from anything stored. It also fixes the drawing side —
lines painted below the header and tab-bar rows, each segment styled by the one `Face`-to-`Style`
mapping in `ui::view`, never past a border, never panicking at a one-row interior — and requires
`normalise_scroll` to clamp the stored offset against the same `content_lines` the draw used, so
a switch between the markdown and checklist bodies cannot leave an offset valid for one applied
to the other. The line lists themselves come from `markdown-render` and `tasks-checklist`.

Requirements it covers: The detail region draws the rendered markdown document; Switching to and from the tracked-tasks tab renormalises the scroll; `Dashboard::detail` carries the markdown source and the scroll offset; The drawn slice is derived on every draw from the current interior; `j`, `k`, and the arrows scroll the detail content at the detail route; The stored scroll offset is normalised against the frame just drawn


### list-filtering
Describes the `/` query layer over the change list: entering it from either route, the modal
keymap in which every printable character — `q`, `j`, `k`, `/`, `r`, the digits, and the four
action keys alike — types itself rather than commanding, leaving only `Ctrl-C` to quit,
`Backspace` to delete, `Enter` to accept and `Esc` to cancel. The matching rule is deliberately
narrow: an ASCII-case-insensitive substring test against `Change::name` alone, applied to the
active and archived tiers alike, never against a path, schema, artifact id or problem text. It
also owns the footer while a query exists — a `/query_` prompt that replaces the hint row
outright while filtering and keeps its tail when it overruns the width, and a leading `/query`
hint once accepted.

Requirements it covers: `/` opens a filter mode in which printable keys type rather than command; The query is a case-insensitive substring match on the change name; The footer shows the filter prompt while filtering and the query after


### list-selection
Fixes what "the selected change" means: a single `usize` index into the *visible* list — active
then archived, with the filter applied — clamped on every action that could change either the
index or the list, so it can never address a row that is not shown, and never lands on a
separator, problem or message row, which are not selectable at all. `Next` and `Prev` move it
only at the list route, clamping at both ends rather than wrapping; at the detail route the same
keys scroll instead. The second half is the viewport: `layout::viewport` derives the first drawn
row from the row count, the cursor's row, and the current interior height on every frame, so the
selection is always on screen and no scroll offset is stored on `Dashboard` where a resize could
stale it.

Requirements it covers: One change is selected, addressed by index into the visible list; The visible slice follows the selection


### live-updates
Covers the dashboard side of keeping the pane current: the three-field `Refresh` state, the `r`
key that only sets a flag the loop later turns into a request, `Dashboard::adopt` preserving the
reader's selection by change **name** across a refresh that reorders the list, and the forced
reload that re-reads the artifact on screen without throwing a scrolled reader back to line one.
Its central obligation is that none of this may ever make the draw wait: the loop's six live
steps run in a fixed order before the frame, every collaborator method is non-blocking, and that
claim is enforced structurally inside the seam modules rather than by any elapsed-time test.
It also fixes that the whole live tier writes nothing under `openspec/`, and that refresh
problems render as leading `!` rows between launch problems and change-set problems. The
watcher, the worker and the poller themselves are `watch-invalidation`'s, `refresh-worker`'s
and `agent-poller`'s.

Requirements it covers: `Dashboard::refresh` carries the live tier's state; `r` forces a full refresh and types itself while filtering; Adopting a change set preserves the selection by name; A forced reload re-reads without losing the scroll; The loop drives the live tier without ever waiting on it; The live tier writes nothing inside the repository; The list region's leading rows name refresh problems first


### markdown-render
Turns an artifact's markdown text into plain-data lines and segments sized for a given column
width — a pure, total transformation that carries a `Face` per segment and never a `ratatui`
type, leaving the mapping from face to style to `ui::view`. It settles every rendering question
a spec-reading pane raises: greedy word wrap with a hard split for tokens that cannot fit,
headings that keep their `#` markers and wrap under their own text column, composing inline
faces with link destinations dropped and images shown as alt text, list markers with a hanging
indent, code and raw HTML reproduced verbatim and split rather than reflowed, quotes and
thematic breaks, soft breaks that preserve the author's line structure, and one blank line
between blocks. Constructs the parser is deliberately not configured for — tables, footnotes,
strikethrough — render as their literal source rather than vanishing, and `pulldown_cmark` is
confined to this one module.

Requirements it covers: Markdown source becomes plain-data lines, parameterised by the interior width; Headings carry their level and their marker; Emphasis, strong, inline code, and links become faces on segments; Lists carry markers, numbering, and a hanging indent; Code blocks and raw HTML are reproduced verbatim and hard-split; Block quotes, thematic breaks, and constructs the parser does not model; Blocks are separated by one blank line and source line structure is preserved; The markdown parser is confined to one module and imports no view type


# Drafted Purposes — capabilities 21–40 (alphabetical)

Planning-time drafts only. No repository file was edited.

### openspec-binary
Locates a usable `openspec` executable for the CLI path, probing four sources in a fixed order — the configured `openspec_bin`, each `PATH` entry, the nvm version tree newest-first, and the npm global prefix reached through an injected hook bound to `cli::npm_prefix` — and reporting which step won so the ordering is observable. It defines what counts as usable (an executable regular file, symlinks followed but returned unresolved), treats an absent binary as a supported state that degrades the dashboard to file mode while a configured-but-unusable path falls through with a recorded problem, and caches the whole outcome in a caller-owned value. Probing itself spawns nothing and writes nothing; how the resolved binary is then run belongs to `subprocess-seam`.

Requirements it covers: A usable `openspec` is an executable regular file reached through symbolic links, The binary is probed in four ordered steps and the winning step is reported, A configured path that cannot be used falls through and is reported, The `npm prefix -g` step is an injected hook bound to a real probe in `cli`, A resolved binary is cached for the session in a value the caller owns, One composition binds resolution to the real environment, Binary resolution reads and never writes

### pane-open
Covers the `open` and `open-tab` subcommands that Herdr's action menu invokes to put the dashboard on screen: reading the invocation context out of Herdr's injected environment through one injected lookup, listing panes first so an existing dashboard in the workspace is focused rather than duplicated, and issuing the exact `herdr plugin pane open` argument vectors for a split placement targeting the invoking pane and for a tab placement in the invoking workspace — never passing `--cwd`, which would break the manifest's relative command. It also fixes how Herdr's own failure reasons are carried verbatim, which failures stop the command and which degrade to opening anyway, and that this path renders nothing and needs no terminal, leaving everything the opened pane then draws to the dashboard capabilities.

Requirements it covers: `open` and `open-tab` are subcommands of their own with no flag grammar, The invocation context is read from Herdr's injected environment through one injected lookup, An already-open dashboard is focused never duplicated, `open` splits the pane the action was invoked from, `open`/`open-tab` never pass `--cwd`, `open-tab` opens a tab in the invoking workspace, Herdr's own reason is carried verbatim and a failed listing degrades to opening, The process's output and exit status are computed by pure functions, The open family needs no terminal and renders nothing, `src/open.rs` reaches the `herdr` program only through the trait object

### plugin-build
Governs how the crate is turned into the single `target/release/herdr-openspec` artifact and what that artifact is made of: `scripts/build.sh` as the POSIX-shell `[[build]]` step Herdr runs (including its `~/.cargo/env` fallback and its refusal to build when `cargo` is unreachable), the binary's three-subcommand argument surface with its stable exit statuses, `ui::run`'s preference for the Herdr workspace cwd over the process working directory, and the argued dependency set — six direct crates with defaults off, a pinned build-graph snapshot, a proc-macro allowlist, and an MSRV floor. It is about what gets built and how it starts, not about how the manifest declares it to Herdr, which `plugin-manifest` owns.

Requirements it covers: The build script produces the release binary, The `ui` invocation runs the dashboard and needs a terminal, The dashboard's own starting directory prefers the workspace context over the process cwd, The crate produces one binary from an argued dependency set

### plugin-config
Reads the plugin's own `config.toml` into the three settings the dashboard is tuned by — `openspec_bin`, `agent_kind`, and `archived_count` — each with a documented default, after resolving the configuration directory from an injected environment lookup (`HERDR_PLUGIN_CONFIG_DIR`, else the `$HOME/.config/herdr/...` path Herdr itself prints, deliberately ignoring `XDG_CONFIG_HOME`). Loading never fails: malformed TOML, wrong-typed keys, and a negative count each degrade to the default while recording a human-readable problem, and a configured `openspec_bin` is tilde/`$HOME`-expanded but never tested for existence — probing it belongs to `openspec-binary`. Reading configuration spawns nothing and writes nothing; the writing half of the plugin's own directories is `plugin-state`'s.

Requirements it covers: The configuration directory is resolved from the process environment, `config.toml` yields three values each with a documented default, Malformed configuration degrades to defaults and reports what it ignored, `openspec_bin` is expanded and never probed, Reading configuration writes nothing

### plugin-manifest
Pins the contents of `herdr-plugin.toml` — the one contract this crate has with a consumer outside itself: the required top-level keys, the single `[[build]]` step, the two `[[panes]]` entries (`dashboard` split and `dashboard-tab` tab, both titled `OpenSpec`), and the two workspace `[[actions]]` that invoke `open` and `open-tab`. It also holds the agreement checks that keep the manifest from drifting from its second sites — the Cargo binary name, `open::DASHBOARD_LABEL` and the entrypoint/placement literals `open_args` emits, the subcommand tokens `parse` accepts, and the action titles named in `README.md` — deliberately leaving `version`, `min_herdr_version`, and `platforms` unpinned because they have no second site.

Requirements it covers: A minimal manifest Herdr can link from the working tree, The manifest path and the Cargo binary name agree

### plugin-state
Owns the plugin's writable state directory and the one file it keeps there: `agent-names.toml`, mapping each Herdr-legal agent name back to the change it was launched for. It fixes how the directory is resolved from the environment (`HERDR_PLUGIN_STATE_DIR`, then `XDG_STATE_HOME`, then `$HOME/.local/state`, deliberately separate from the hand-edited config directory), the pure deterministic change-name-to-agent-name derivation including truncation with an FNV-1a base-36 suffix, when a mapping is worth recording at all, how an unusable mapping file degrades to an empty mapping, and that recording is atomic by rename and confined to the state directory. Consuming the mapping to attribute a live agent to a change belongs to `agent-attribution`, and checking a name against Herdr's live agents to `agent-launch`.

Requirements it covers: The state directory is resolved from the process environment, A change name is converted to a Herdr-legal agent name, A derived name is recorded only when it differs from the change name, An unusable mapping file yields an empty mapping, Recording is atomic and creates only what it needs, Nothing is written outside the state directory

### quality-gates
Defines the verification contract for the repository: the `Makefile` targets behind `make check` (format check, clippy with warnings denied, tests, and `cargo llvm-cov` at an 80% line floor, in that order, first failure stopping the run), the guards that name the one-time install command when clippy or `cargo-llvm-cov` is missing, the checked-in `rustfmt.toml` that keeps a bare `rustfmt` on the crate's edition, and the rule that no gate may touch Herdr. It also carries the testing discipline the `ui` layer is held to — every view scenario asserted against named cells of a `TestBackend` buffer at both 60 and 120 columns, with test doubles that are synchronous and touch no filesystem, clock, thread, or terminal.

Requirements it covers: `make check` is the single gate and runs all four checks, The coverage floor is 80% of lines enforced and never waived, Missing one-time tools fail with the install command named, Formatting configuration is checked in and the tree is formatted, The gates do not depend on Herdr, View behaviour is verified against a `TestBackend` buffer at both widths

### refresh-worker
The background tier that keeps the dashboard's data fresh without ever making a frame wait: a `Selection` naming which changes the expensive `openspec instructions apply` call must be re-asked about, a `CliCache` that reuses schema and artifact lists for unselected changes while always taking progress from the fresh `list --json`, and a `Refresher` trait whose `request`/`take_result` pair is non-blocking by contract. Its worker owns the crate's only extra thread, coalesces queued requests by union, and answers each request twice — the cheap file-sourced set first, then the CLI-merged one — so the render loop adopts whichever is ready. Deciding *when* to request a refresh from filesystem events is `watch-invalidation`'s job; adopting the results into the view is `live-updates`'.

Requirements it covers: A selection names which changes the CLI must be re-asked about, The CLI producer re-asks only about the selected changes, The refresh seam is a trait whose every method is non-blocking, The worker answers one request with the file result then the merged one

### repo-discovery
Locating the OpenSpec repository the pane is looking at: walking up from a caller-supplied starting directory to the nearest ancestor holding an `openspec/` directory — a real directory or a link to one, never a file or a dangling link — canonicalizing the start when the filesystem can resolve it and otherwise stopping short of the empty ancestor so a relative path is never answered with the process's own working directory. When nothing qualifies it names the directory the search began from, so the empty state can print it; a starting path that does not exist is a supported result rather than an error, and the whole walk creates and modifies nothing.

Requirements it covers: The repository is the nearest ancestor holding an `openspec/` directory, Repository discovery reads and never writes

### responsive-layout
The dashboard's outer frame and how it reflows with terminal width: a one-row bold `OpenSpec` header carrying the repository root right-aligned and shortened from the left, a bordered body, and a footer of key hints dropped whole from the end rather than truncated, with explicit branches for degenerate heights and one-column frames. It owns the 100-column breakpoint that decides whether the body holds a 40-column `Changes` region beside a `Detail` one or a single region showing only the routed side, which region's border is emphasised, that content never bleeds across a border, and the two interior widths — 78 at a 120-column frame and 58 at a 60-column one — that the detail-side capabilities render into. What fills those interiors belongs to `change-rows`, `detail-header`, `artifact-tabs`, and `artifact-content`.

Requirements it covers: The frame is a header row a body and a footer row, The 100-column breakpoint decides one region or two, The routed region is emphasised and region interiors are left empty, The header names the repository root shortened from the left when narrow, The detail region's two mandated interior widths are 78 and 58

### schema-artifacts
Turning an already-chosen schema name into the model the dashboard renders: parsing `openspec/schemas/<name>/schema.yaml` as YAML into an ordered artifact list — file order, since that is the detail view's tab order — each entry carrying its `id` and `generates` verbatim, plus the one artifact holding the change's task checklist, picked by `apply.tracks` matching a `generates` value and falling back to the artifact whose id is `tasks`. Nothing fails closed: an unusable entry is skipped and named, and the three reasons a schema did not load at all — not vendored, unreadable, invalid — are kept apart because only the first is repairable by asking the CLI. Which name to load is `schema-selection`'s decision; asking the CLI where an unvendored schema lives is `schema-cli-fallback`'s.

Requirements it covers: A vendored schema is loaded from `openspec/schemas/<name>/schema.yaml`, The tasks artifact is the one `apply.tracks` selects else the one with id `tasks`, An unusable artifact entry is skipped and named not fatal, A `schema.yaml` that is not a usable schema yields the invalid reason, Schema loading reads and never writes and spawns no process

### schema-cli-fallback
Recovers the one schema-loading failure a disk-only read cannot fix: when `schema::load` reports the schema is simply not vendored in the repository, `from_cli` asks `openspec schema which <name> --json` for the directory it lives in — the user or CLI-package tier that holds `spec-driven` — and parses it with the same loader. It fixes that an unreadable or invalid vendored file is never re-asked of the CLI, that every way this tier can fail (unstartable binary, non-zero exit, unusable payload, a named directory with no or a broken `schema.yaml`) degrades that change alone with exactly one named problem, that the payload must be whole JSON with no tolerated leading noise, and that resolution is cached per name per call so twelve changes sharing a schema cost one invocation. Parsing the schema file itself remains `schema-artifacts`'.

Requirements it covers: A not-vendored schema is repaired through `openspec schema which`, Every failure of the fallback tier degrades and names itself, A schema is resolved at most once per name per call

### schema-selection
Deciding *which* OpenSpec schema name applies to a repository or to one change inside it: a change's own `.openspec.yaml`, then the repository's `openspec/config.yaml`, then the constant default `spec-driven` — reporting which of the three answered, because two sources routinely name the same schema and the name alone cannot prove the ordering. Selection is total and read-only: an unreadable file, invalid YAML, a wrong-typed or blank value, and a name that would escape `openspec/schemas/` each fall through with a named problem rather than winning or stopping the search, while an absent file or an absent `schema:` key is the silent normal case. It never opens the schema it names — that is `schema-artifacts`.

Requirements it covers: The schema name comes from the change then the project then the default, A declared name that would escape the schema directory is rejected, Schema selection reads and never writes

### subprocess-seam
The crate's one permitted spawn site: the `OpenspecCli` and `HerdrCli` traits, each `Send + Sync`, whose single real implementation starts the program it was constructed with and returns stdout verbatim on success — no added arguments, no trimming, no parsing, no retry, no timeout, no cache — with stderr, the exit code, and the argument vector carried on the two error variants instead. It also holds the recording fake that stands in for both traits in every other capability's tests and panics rather than guessing at an unregistered call, the real `npm prefix -g` probe behind a pure decision function over exit status and stdout, and the two architectural gates that make the seam real: no file under `src/` but `src/cli.rs` may name a process-spawn API, and no file under `src/ui/` may name the Herdr trait. Running a program writes nothing to disk.

Requirements it covers: Two traits carry the two programs and success is stdout, The real implementations spawn and return stdout and do nothing else, The recording fake answers by argument vector and refuses to guess, `npm prefix -g` is probed behind the seam from stdout only trimmed, The npm probe resolves an `openspec` binary end to end, One binding names the real `npm` program and `resolve` names no process API, `cli` is the only module in the crate that spawns a process, Running a program writes nothing, One binding names the real `herdr` program and `src/ui/` never names the trait

### task-checkboxes
The line-level rule that decides which lines of a task file are tasks and which of those are done, deliberately reproducing `@fission-ai/openspec`'s own `TASK_LINE_PATTERN` so the plugin's file path and the CLI path never report different `[completed/total]` numbers for one change: a `-` or `*` bullet, a one-character box holding whitespace, `x`, or `X`, and the trimmed remainder as the item's text. It fixes the exact edges that agreement demands — no `+` or ordered bullets, no context exemptions for fenced blocks or HTML comments, the CLI's whitespace alphabet extended with U+FEFF and excluding U+0085, CRLF-insensitive splitting — and defines `Progress`, the summable count both producers of a change emit. Arranging those items under headings is `task-groups`.

Requirements it covers: A task line is a `-` or `*` bullet carrying a one-character checkbox, `x` or `X` marks an item done every other legal box character does not, Counting has no context exemptions, The checkbox whitespace alphabet is the CLI's plus the byte-order mark, Line endings and trailing carriage returns never change a count, `Progress` is the counting shape both sources of a change produce

### task-groups
Arranges the task lines a file contains into the document model the detail view renders: column-zero ATX headings open groups, items keep document order inside them, each item carries its checked state, trimmed text, and indent, and every group plus the file as a whole carries its own `Progress`. The structure is deliberately flat — a deeper heading closes the group above rather than nesting — and grouping is required never to change what `task-checkboxes` would count flatly, asserted directly because the list row and the detail tab read the same file through different entry points. It also owns the single filesystem edge here: reading a task file from a caller-supplied path, treating an absent file as zero tasks and every other read failure as one named problem, writing nothing and spawning nothing.

Requirements it covers: An ATX heading at the start of a line opens a group, Groups and items preserve document order and no heading is discarded, An item carries its checked state its text and its indent, Grouping never changes what is counted, Reading a task file degrades rather than failing, Task parsing and reading write nothing

### tasks-checklist
The detail tab that renders a change's task file as a checklist instead of markdown: the single decision — made from the selected artifact's `tracks_tasks` flag, never from its id or filename — that swaps `ui::markdown::lines` for `ui::tasks::lines`, and the line grammar that results, with heading lines reproduced from their level, one `[x]`/`[ ]` glyph line per item preserving the parse's own indent, hanging-indent wrapping, whole-indent dropping as the width collapses, and blank separators between groups. It fixes `No tasks yet` for a source that holds no items, kept distinct from `artifact-content`'s `No content yet` for a source that does not exist, and it carries the read-only guarantee: no key toggles an item, no action mutates a `Change`, and no module the dashboard reaches names a filesystem write API. The bar that leads the tab is `tasks-progress-bar`'s.

Requirements it covers: The tracked-tasks tab renders a checklist and every other tab does not, The checklist's line grammar, A source holding no task lines renders `No tasks yet`, The tasks tab is read-only and writes nothing

### tasks-progress-bar
The single line that leads the tracked-tasks tab: a bare `█`/`░` gauge, then `ui::list::progress_cell`'s `[<completed>/<total>]`, then a truncated integer percentage, separated by one space each and filling exactly the interior width. It fixes that the gauge is full if and only if the change is complete and empty whenever nothing is done, that the fields degrade by being dropped whole in a fixed order — percentage, then gauge, then the whole line — as the width narrows, and that a change with no tasks shows `[-]` alone rather than an invented `0%`. The number it renders is the change's own `progress` field, shared with the list row and the detail header, never a recount of the source the checklist beneath it parses.

Requirements it covers: The progress bar's grammar, The gauge is full exactly when the change is complete, Cells are dropped whole as the bar narrows, A change with no tasks renders the count cell alone

### terminal-lifecycle
Owns the process's terminal mode: a four-operation `TerminalOps` seam whose only real implementation calls the crossterm functions and nothing else, a `TerminalGuard` that enters raw mode then the alternate screen and leaves them in exactly the mirrored order, unwinding a partial entry when the second step fails and swallowing teardown errors rather than panicking in `Drop`, and a panic hook that restores before delegating to the hook it replaces. It also holds the pure decision that refuses to start the dashboard when stdout is not a terminal without touching a single terminal operation. The exit status and message that refusal produces belong to `plugin-build`.

Requirements it covers: Terminal setup and teardown sit behind an injected seam, The guard enters and leaves in a fixed mirrored order, A panic restores the terminal, The dashboard refuses to start when stdout is not a terminal

### watch-invalidation
Turns filesystem activity under `openspec/` into the refresh requests the worker acts on: classifying a touched path to the one change it invalidates, to the repository as a whole, to the archive tier, or to nothing outside the tree — conservatively, so an unrecognised path costs a reload rather than silence — and folding a batch of them into a `Selection`. It owns the debounce that coalesces a burst of saves, written as a pure state machine over an injected `Instant` with a 150ms sliding window capped at one second so a continuously-writing agent cannot defer a batch forever, the arithmetic that shortens the event loop's wake-up to the nearest pending deadline, and the non-blocking `FsEvents` seam whose one real implementation confines `notify` to `src/watch.rs` and holds the crate's single binding to the real clock. Acting on the resulting selection is `refresh-worker`'s.

Requirements it covers: The filesystem-watch crate is confined to one module, A touched path is classified to the changes it invalidates, The debounce is a pure state machine over an injected instant, The loop's wake-up shortens to the debounce deadline, The watcher seam is a trait with one real implementation
