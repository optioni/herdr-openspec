## REMOVED Requirements

### Requirement: An ambiguous agent kind stops the launch before any pane is split

**Reason**: The header states the whole behaviour, and the behaviour changes: the launch still
stops before any pane is split, but the refusal now opens the settings panel on the
`agent_kind` row instead of leaving a problem row as the only answer. The landed requirement
says so itself — "`settings-window` upgrades this row to an interactive picker; until then the
row is the answer" — and this is that change.

**Migration**: Replaced by the ADDED requirement below, which carries every sentence and every
scenario forward and adds the `picker` field, the panel-opening rule, and the rule that no
other outcome touches the overlay. The refusal's wording, its front-loading, its single-entry
`problems`, the absent `pane split`, and the in-flight clearing are all unchanged.

## ADDED Requirements

### Requirement: An ambiguous agent kind opens the settings panel instead of picking one

`integration::resolve` returns `Choice::Ambiguous` when nothing is configured, nothing is
recorded, and **two or more** integrations are installed. It produces no kind, so there is no
`--kind` value for `agent start` to carry, and the launch SHALL stop rather than pick one.

The worker SHALL end the request at that point, before `pane split`, and report an `Outcome`
with:

- `named` `None` — no agent was started, so nothing may reach `Dashboard::agent_names`, on
  exactly the failed-split path's terms;
- `problems` holding **exactly one** entry, naming every installed kind in Herdr's own printed
  order and telling the reader to set `agent_kind`;
- `picker` **set** — the signal that this refusal has an answer the reader can act on without
  leaving the pane;
- `resolution` carrying the `KindResolution` the worker just computed, so the panel it opens
  can render the ambiguity and offer both candidates rather than showing `Pending`.

`Outcome` SHALL therefore carry exactly **four** fields — `named`, `problems`, `picker`, and
`resolution` — where `agent-client-choice` left it at two. Never `Default`, anywhere in the
crate; every construction and destructuring names all four, with no `..` rest, on exactly the
terms the two-field version held to.

The entry SHALL be **front-loaded**: the key to set and the kinds to choose between come
first, before any explanation. This row is the whole answer to the key the reader just
pressed, and `pad_or_truncate_right` cuts it to the list region's **38**-column interior at
the wide layout — an explanation-first wording put both candidate kinds past that cut, so the
row said nothing the reader could act on at either mandated width. Both kinds and the key
SHALL survive the cut at 38 and at 58 for a two-kind fixture.

No Herdr call beyond `integration status` SHALL be issued: no pane is created, so none can be
left behind. `Dashboard::launch.in_flight` SHALL be cleared by the same `drain` that adopts the
outcome, on the established lifecycle — an ambiguous resolution SHALL NOT lock the launch keys
for the rest of the session.

This is the one resolution outcome that stops a launch, and it is not a contradiction of
"never fail closed": the evidence exists and points two ways at once, and choosing between two
clients the reader has both set up is exactly the guess `agent-attribution` refuses to make
about a terminal title. Where there is no evidence at all, `LastResort` launches `claude` and
warns. The `drain` that adopts an outcome whose `picker` is set SHALL open the **settings panel** on
the `agent_kind` row: `overlay.panel` becomes `Some(Panel::Settings)` and the row cursor moves
to `agent_kind`'s index. This is the upgrade the landed wording anticipated — step 4 of the
precedence *is* this panel — and it is what makes one modal in the crate rather than a picker
beside a window.

The problem row SHALL still be recorded, unchanged and front-loaded. The panel is drawn over
the body and the reader may dismiss it without choosing; the row is what remains behind when
they do, so the answer to the key they pressed does not vanish with the overlay.

Opening a panel from a **worker's** result is the one place in the crate where a background
answer moves the overlay, and it is deliberate: the reader pressed `a`, the pane could not
answer it, and the panel is the answer. It SHALL NOT happen on any other outcome — a
successful launch, a failed call, and a refusal that is not `Ambiguous` all leave
`overlay.panel` exactly as they found it.

If the reader has an overlay open when the outcome arrives, the settings panel SHALL replace
whatever panel was open, on the same swap terms `ToggleSettings` follows, and
`overlay.scroll` SHALL reset to `0`.

The other three keys SHALL be unaffected: `g` still focuses, and the list, filter, and detail
routes are untouched — a launch that could not choose a client is not a broken pane.

#### Scenario: Two installed integrations stop the launch with one problem and no pane

- **WHEN** a launch runs against a scratch `herdr` program whose `integration status` reports
  both `claude` and `codex` as installed, with no `agent_kind` configured and no recorded kind
- **THEN** the invocation log holds exactly one entry, `integration status`
- **AND** no `pane split` entry appears, so no pane is created and none can be orphaned
- **AND** `outcome.named` is `None` and `outcome.problems` holds exactly one entry naming
  `claude`, `codex`, and `agent_kind`
- **AND** the scratch state directory is byte-identical to before the launch

#### Scenario: The ambiguous refusal clears the in-flight flag and leaves `g` working

- **WHEN** the outcome above is adopted by a `Dashboard` whose `launch.in_flight` was `true`
- **THEN** `launch.in_flight` is `false` afterwards
- **AND** a subsequent `g` with an attributed agent still returns
  `Decision::Go(Request::Focus { .. })`
- **AND** a subsequent `a` is not refused by the in-flight guard, so the launch keys are not
  locked for the rest of the session. It answers with the **same** refusal, re-derived from
  the cached `Choice`, **unless** a kind has since been committed in the settings panel, which
  invalidates that cache — see "A committed kind takes effect without restarting the pane"

#### Scenario: The ambiguous stop renders as one leading problem row at both widths

- **WHEN** that `Dashboard` is rendered at 120x20 and 60x20
- **THEN** the list region's interior row 0 begins `! ` and names both installed kinds and
  `agent_kind` at **both** widths — which is what the front-loading above is for, since the
  38-column interior cuts everything after roughly the first thirty-six columns
- **AND** the row is exactly the interior width — 38 at 120 and 58 at 60 — on the same
  `pad_or_truncate_right` terms as every other row
- **AND** the change list is drawn below it, so the pane stays usable

#### Scenario: A configured kind suppresses the stop entirely

- **WHEN** the same two-installed status is answered with `agent_kind = "codex"` configured
- **THEN** the launch proceeds and the log holds all four entries
- **AND** `outcome.problems` is empty, so the stop is caused by the absence of a choice and by
  nothing else in the fixture

#### Scenario: The ambiguous outcome opens the settings panel on `agent_kind`

- **WHEN** the two-installed ambiguous outcome above is adopted by a `Dashboard` whose
  `overlay.panel` is `None`
- **THEN** `overlay.panel` is `Some(Panel::Settings)` afterwards and the row cursor is on
  `agent_kind`
- **AND** `overlay.scroll` is `0` and no edit is in progress
- **AND** `launch.problems` still holds exactly the one front-loaded entry naming both kinds
  and `agent_kind`, so dismissing the panel leaves the answer on screen
- **AND** `route`, `selected`, `detail`, and `sections` are unchanged, so the panel opened over
  the frame rather than moving it

#### Scenario: No other outcome touches the overlay

- **WHEN** a successful launch outcome, a failed `pane split` outcome, an in-flight refusal,
  and a `LastResort` outcome are each adopted by a `Dashboard` whose `overlay.panel` is `None`,
  and again by one whose panel is `Some(Panel::Help)`
- **THEN** `overlay.panel` is unchanged in all eight cases
- **AND** only the `Ambiguous` outcome's `picker` field is set, across all five outcome kinds

### Requirement: A committed kind takes effect without restarting the pane

`Launcher` SHALL gain one method, and `launch::Request` SHALL gain one variant. Both SHALL be
**non-blocking** on exactly the trait's established terms — they return without waiting on the
worker, perform no I/O, spawn no process, and read no clock:

```rust
fn set_kind(&mut self, kind: String);

pub enum Request {
    Launch { change: String, agent: String, intent: Intent },
    Focus { pane_id: String },
    Resolve,                      // `settings-window`: resolve the kind and answer, nothing else
}
```

`Request::Resolve` SHALL be sent when `,` **opens** the settings panel. The worker SHALL answer
with an `Outcome` whose `named` is `None`, whose `problems` carries whatever the resolution
itself reported, whose `picker` is **unset** — the panel is already open, so nothing needs to
open it — and whose `resolution` is `Some`. When the worker's session cache is already
populated it SHALL answer from the cache and issue **no** `integration status` call, so
opening the panel repeatedly costs one call per session at most.

Committing an `agent_kind` edit in the settings panel SHALL call it. It SHALL replace the
launcher's session cache of the resolved kind with `Choice::Use { kind, source:
Source::Recorded }`, so the **next** `a`, `c`, or `s` launches under the committed kind, issues
no `integration status` call, and produces no ambiguous refusal.

This is the one thing that makes the panel worth building rather than a viewer: before it,
`agent-client-choice` read the kind once per process and deliberately never invalidated it, so
acting on an ambiguous row meant editing `config.toml` and restarting the pane. A picker that
still required a restart would answer the reader's question and then refuse to act on it.

It SHALL NOT re-resolve the precedence. The committed kind is step 2's value and `config.toml`
is step 1; a setting that `config.toml` owns is refused before an edit can begin
(`settings-window`), so a commit can never install a value that the precedence would have
overruled.

`NoLauncher` — the inert implementation `start_collaborators` uses when no repository was
found, and a second **production** impl rather than a test double — SHALL answer both
additions inertly: `set_kind` SHALL do nothing and `Request::Resolve` SHALL produce no
`Outcome`, on exactly the terms its existing `request` discards and its `drain` returns
`None`. In **file mode** the launch keys are refused anyway, so there is no kind to commit;
the settings panel still opens, still shows `openspec_bin` and `prompts` with their real
provenance, and leaves `agent_kind` at `Provenance::Pending` for the life of the session
rather than resolving something no launch will use.

`set_kind` SHALL NOT reach the worker through the request channel. It SHALL store into a
cache the worker shares, so that a launch already queued behind it cannot be resolved under
the old kind.

The lock that cache needs SHALL live **inside** `src/launch.rs` and SHALL NOT be named from
`src/ui/`. Its **hold discipline** is the property that makes `set_kind` non-blocking, and it
SHALL be stated rather than assumed: the worker SHALL acquire the lock only to **read or
replace the cached `Choice`**, and SHALL NOT hold it across any `HerdrCli` call, any
`integration status`, or any `agent start`. The critical section is therefore bounded by a
clone and a move, and a caller can never wait behind a thirty-second `agent start`.

`NOBLOCK` does **not** check this. Its `BLOCK3_RE` names `recv`, `recv_timeout`, `join`,
`park_timeout`, `thread::park`, and channel iteration — no `Mutex`, `RwLock`, or `lock()` — and
its lock-naming leg is scoped to `src/ui/`. So the gate is green whether or not `set_kind`
blocks, and it SHALL NOT be cited as evidence for this requirement. The scenario below is the
evidence, and it is written to fail if the discipline is broken.

#### Scenario: The next launch uses the committed kind and issues no status call

- **WHEN** a launch against a scratch `herdr` whose `integration status` reports `claude` and
  `codex` installed refuses as ambiguous, the reader commits `codex` in the settings panel, and
  `a` is pressed again
- **THEN** the second launch's invocation log holds `pane split`, `agent start`, and
  `agent prompt` and **no** second `integration status`
- **AND** `agent start` carries `--kind codex`
- **AND** `outcome.problems` is empty and `outcome.picker` is unset

#### Scenario: A cancelled edit changes nothing the launcher sees

- **WHEN** the same ambiguous refusal is followed by an edit that is begun, stepped to `codex`,
  and then cancelled with `Esc`
- **THEN** `set_kind` was never called
- **AND** the next `a` answers with the same ambiguous refusal and sets `picker` again
- **AND** no `settings.toml` exists in the scratch state directory

#### Scenario: Opening the panel resolves the kind once and launches nothing

- **WHEN** `,` opens the settings panel on a freshly started pane against a scratch `herdr`
  whose `integration status` reports `claude` and `codex` installed, and the panel is then
  closed and opened twice more
- **THEN** the invocation log holds exactly **one** `integration status` entry and no
  `pane split`, `agent start`, or `agent prompt` entry at all
- **AND** the first answer's `Outcome` has `named` `None`, `picker` **unset**, and `resolution`
  `Some` carrying `Choice::Ambiguous` with both kinds
- **AND** the two later opens are answered from the worker's session cache, which is what keeps
  the count at one
- **AND** the scratch state directory is byte-identical throughout, since opening a panel
  writes nothing

#### Scenario: File mode answers both additions inertly

- **WHEN** a dashboard in **file mode** — no repository found, so the launcher is `NoLauncher`
  — opens the settings panel with `,` and the frame is drawn at 120x40 and 60x20
- **THEN** no `Outcome` is ever produced, `overlay.panel` is `Some(Panel::Settings)`, and the
  panel renders at both widths
- **AND** the `agent_kind` row stays at the `Pending` label rather than resolving or erroring,
  and is not editable
- **AND** `openspec_bin` and `prompts` carry their real values and provenance, so the panel is
  still useful in the mode where the dashboard has no CLI at all
- **AND** `set_kind` called against `NoLauncher` changes nothing and does not panic

#### Scenario: `set_kind` returns while the worker is blocked mid-launch

- **WHEN** the real `Launcher`'s worker is parked inside a scratch `herdr agent start` that
  blocks on a test-controlled FIFO, and `set_kind("codex")` is then called from the calling
  thread
- **THEN** `set_kind` returns **before** the FIFO is released, which is the observable ordering
  that distinguishes a bounded critical section from one held across the child process
- **AND** the test asserts that ordering by sequence — the return is observed, then the FIFO is
  released — rather than by elapsed time, so it needs no clock, which `NOBLOCK` forbids under
  `src/ui/` and which design.md → Test Boundaries records as unused
- **AND** a deliberately broken implementation that holds the lock across the `agent start`
  call deadlocks or fails this ordering, so the scenario can go red
- **AND** no file under `src/ui/` names the lock type

