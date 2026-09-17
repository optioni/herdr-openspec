## Why

Configuration is invisible and, after `agent-client-choice`, no longer simple. That change
turns `agent_kind` into a **five-level precedence** — an explicit `config.toml` value, then a
recorded choice in the state directory, then a single installed integration, then a prompt,
then `claude` as a last resort. When the pane launches the "wrong" client there is nothing on
screen that says which level decided it.

The settings the plugin already has are equally hidden: `openspec_bin`, `agent_kind` and
`archived_count` live in `~/.config/herdr/plugins/config/herdr-openspec/config.toml`, a file
most users never create — **it does not exist on the reference machine** — and every key has a
silent default.

Two arguments this was not worth building before, and why they have flipped:

- *"It is one setting, not a preferences system."* It is now three keys plus per-kind prompt
  overrides plus a five-level resolution.
- *"A modal is new machinery for one job."* `agent-client-choice` needs a client picker
  regardless. One window that covers the picker costs less than a picker plus, later, a window.

## What Changes

- **A settings window**, opened by a key from either route **at any time**, closed by `Esc`. It
  is the crate's first overlay: `Route` is `List`/`Detail` today and no modal or overlay concept
  exists.
- **Every editable setting stays editable for the life of the install.** This is not a first-run
  wizard. Once a choice is recorded, `agent-client-choice`'s step 4 never fires again, so this
  window is the **only** way to change the client without hand-editing `config.toml` — which
  makes it the re-entry path, not a one-time prompt.
- **It shows every setting, its effective value, and the level that decided it** — this is the
  primary job. "`agent_kind` = `codex`, from the single installed integration" answers the
  question a user actually has.
- **It subsumes the client picker.** `agent-client-choice`'s step 4 is this window opened on the
  `agent_kind` row, so there is one modal in the crate rather than two.
- **Exactly one setting is editable in place**, written to `HERDR_PLUGIN_STATE_DIR`:
  `agent_kind`. Everything else is **read-only** with its source shown.
- **The `agent_kind` row is a shortlist of the installed integrations, and nothing else.**
  This **reverses** what this proposal first argued. The earlier text said any kind could be
  entered, because `integration status` is not an availability check and filtering by it would
  be the capability-check misuse `agent-client-choice` rules out. That argument is sound and is
  accepted as a **cost**, not refuted: the panel gains no text field, no cursor, and no
  character-level editing — machinery the crate has only in `/` filter mode — and the reader who
  wants an uninstalled kind hand-edits `config.toml`, which still accepts any kind and still
  outranks this panel. The boundary case shortlist-only creates is specified rather than
  ignored: with **no** installed integration the row is not editable and says so, naming
  `config.toml`.
- **`archived_count` is not shown at all.** `list-sections` made the key *accepted and inert* —
  nothing consumes it, `change-enumeration` no longer truncates the archived tier, and the
  archived section's fold decides what the list shows. A panel whose job is to say what is
  deciding each setting has nothing to say about a value that decides nothing, and a stepper for
  it would invite an edit with no visible effect. The key stays readable in `config.toml`,
  documented there as inert, which is where a reader who has set it will look.
- **`config.toml` still wins.** An explicit key there outranks anything set here, and the
  window says so on any row where that is happening, rather than silently accepting an edit
  that will not take effect.
- Not **BREAKING**: no manifest or config-format change, and one new keybinding.

## Non-Goals

- **Writing `config.toml`.** The plugin's writes are scoped to its state directory — never the
  repository, never the configuration directory the user hand-edits. A program rewriting that
  file would destroy comments and formatting and race with the editor that owns it. This is a
  standing rule and this change does not relax it.
- **Editing `openspec_bin` or the per-kind prompts in the window.** A filesystem path and a
  multi-line prompt are miserable to type in a TUI and are set-once values. Shown, not edited.
- A general key-value editor, or anything resembling a form framework.
- Reading or writing Herdr's own configuration.
- Editing anything inside `openspec/`.

## Capabilities

### New Capabilities

- `settings-window`: the overlay — when it opens, what it renders, which keys it takes, how it
  closes, and how it behaves at the narrow layout.
- `setting-provenance`: for each setting, the effective value and which precedence level
  produced it, as a value the view renders rather than derives.

### Modified Capabilities

- `dashboard-loop`: `Action` gains `ToggleSettings`, and `Dashboard::help: Help` becomes
  `overlay: Overlay` carrying `panel: Option<Panel>` — still sixteen fields.
- `help-overlay`: the layer it introduced now carries either panel; the help panel answers
  eight actions instead of seven.
- `binding-inventory`: the `,` row, and a seventh group for the keys the settings panel
  reinterprets.
- `plugin-state`: `settings.toml` gains its first writer, beside `agent-names.toml`.
- `responsive-layout`: `layout::help_band` becomes `layout::overlay_band`, serving both panels.
- `mouse-input`: clicking a setting row, and clicking outside to dismiss — which becomes
  `Action::Back` rather than `Action::ToggleHelp`, since with two panels the latter swaps
  rather than closes.
- `agent-launch`: step 4 of the kind precedence opens this panel, and a committed kind
  invalidates the launcher's session cache so it takes effect without a restart.
- `quality-gates`: the gate-script count moves from thirty-one to thirty-two with
  `settingswidths.sh`.
- `doc-conformance`: `src/settings.rs`' freedom from I/O becomes the sixteenth checked claim.

The last two were added during planning review, which found both capabilities being changed
with no delta — and `tasks.md` directing an edit to `openspec/specs/` in place, which
`OPENSPEC-UNTOUCHED`'s tracked-diff leg exists to refuse.

`plugin-config` is **not** modified. This proposal first put provenance "beside each value" in
`src/config.rs`; it lands in the new `src/settings.rs` instead, because `Config` is the parse
result of one file and cannot know about the four other levels that outrank or underwrite it.
Absence of a key in `Config` is the only provenance signal `config.toml` can honestly give.

## Impact

- `src/ui/` — a new pure view file, the overlay route, key and mouse handling. No I/O: the view
  receives settings and provenance as state, exactly as it receives changes.
- `src/settings.rs` — **new**: the settings list and the provenance enum. Not `src/config.rs`:
  see the note under Capabilities.
- `src/launch.rs` — `Request::Resolve`, `Outcome`'s two new fields, and `Launcher::set_kind`.
- `src/ui/driver.rs` — `Target::Setting`, and the click-outside action correction.
- `src/state.rs` — recording the editable settings.
- `README.md`, `SPEC.md` — the keybinding, the precedence table, and the write boundary restated
  where someone proposing "just save config.toml" will read it.
- View tests at 60 and 120 columns. No dependency, no spawn, no seam change.

## Dependency

Depends on `agent-client-choice`, which defines the precedence this window displays and the
picker it absorbs. Until this lands, that change's step 4 degrades to a problem row naming the
installed integrations and telling the user to set `agent_kind` — honest and non-blocking, and
the fallback this window upgrades.

**`help-overlay` landed first, as this proposal preferred.** It owns the overlay — a band over
the body, a dispatch that takes precedence over the route and filter dispatches, and `Esc`
closing it before any other layer — so this change is a **second panel**, not a second
mechanism, exactly as the sentence above anticipated. `agent-client-choice` has landed too, so
the precedence this panel displays and the picker it absorbs both exist.

## Resolved Decisions

The five questions this proposal opened are answered. Two were settled by work that landed
after it was written; three were decided by the author before specs.

1. **Is provenance display the real product?** No — it is half of it. The panel shows all three
   settings with their levels, and `agent_kind` is editable, because `settings.toml` had no
   writer at all: step 2 of the precedence could never fire, and an ambiguous refusal was
   fixable only by hand-editing `config.toml` and restarting the pane.
2. **Overlay or third route?** **Overlay** — settled by `help-overlay`. `Route` stays `List` and
   `Detail`; the layer carries `panel: Option<Panel>`, so two panels open at once is
   unrepresentable rather than merely avoided.
3. **`config.toml` overriding an edited row?** **Refuse the edit**, naming the file. An edit that
   is written, persisted, and visibly has no effect is the more confusing outcome; a refusal
   answers the same question before anything is written.
4. **Free-text or shortlist?** **Shortlist only**, reversing this proposal's own argument — see
   What Changes above for the cost that buys and the boundary case it creates.
5. **Which key?** **`,`**, the near-universal settings convention. `S` was rejected for sitting
   one slipped shift from `s`, which launches an archive agent — a mis-press that starts a
   process is worse than one that opens nothing.
