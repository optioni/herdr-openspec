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
- **A small set is editable in place**, written to `HERDR_PLUGIN_STATE_DIR`: `agent_kind` and
  `archived_count` (a number). Everything else is **read-only** with its source shown.
- **The `agent_kind` row offers installed integrations as a shortlist but does not restrict to
  them.** Any kind can be entered, because `integration status` is not an availability check — a
  kind launches fine without its integration, and filtering by it would be the capability-check
  misuse `agent-client-choice` explicitly rules out. Choosing a kind with no integration is
  allowed and carries the "status will read `unknown`" warning beside it. The shortlist also
  avoids hardcoding the 23-kind enum, which lives only in `--help` text.
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

- `dashboard-loop`: an overlay route, and what keys mean while it is open — `/` filter mode is
  the existing precedent for a mode that changes key meaning.
- `plugin-state`: the settings the window records, beside `agent-names.toml`.
- `plugin-config`: `Config` gains provenance alongside each value.
- `responsive-layout`: how the overlay sizes at the 100-column breakpoint and below.
- `mouse-input`: clicking a row, and clicking outside to dismiss.
- `agent-launch`: step 4 of the kind precedence becomes this window.

## Impact

- `src/ui/` — a new pure view file, the overlay route, key and mouse handling. No I/O: the view
  receives settings and provenance as state, exactly as it receives changes.
- `src/config.rs` — provenance beside each value.
- `src/state.rs` — recording the editable settings.
- `README.md`, `SPEC.md` — the keybinding, the precedence table, and the write boundary restated
  where someone proposing "just save config.toml" will read it.
- View tests at 60 and 120 columns. No dependency, no spawn, no seam change.

## Dependency

Depends on `agent-client-choice`, which defines the precedence this window displays and the
picker it absorbs. Until this lands, that change's step 4 degrades to a problem row naming the
installed integrations and telling the user to set `agent_kind` — honest and non-blocking, and
the fallback this window upgrades.

**Prefer landing `help-overlay` first.** Both need the crate's first overlay, and that change is
read-only — no editing, no state writes, no provenance — so it proves the machinery at a
fraction of the risk. Whichever lands first owns the overlay; if it is `help-overlay`, this
change becomes a second panel rather than a second mechanism.

## Open Questions for Review

1. **Is provenance display the real product here?** If so the editable set could start empty and
   the window would still earn its place — worth deciding before specs, because it changes what
   this is.
2. **Overlay or third route?** An overlay must compose with the existing two regions and the
   100-column breakpoint; a third route is simpler and loses the context behind it.
3. **What happens when `config.toml` overrides a row the user just edited?** Refusing the edit is
   honest; accepting it and showing it as shadowed is kinder and more confusing.
4. **Free-text entry or shortlist only?** Allowing any kind needs a text field, which the crate
   has only in `/` filter mode. A shortlist alone is far simpler and silently blocks the
   uninstalled-integration case.
5. **Which key opens it?** Single letters are nearly exhausted — `a c s g r q j k` and `1`–`9`,
   `[`, `]`, `/`, `Space`, `Enter`, `Esc` are taken.
