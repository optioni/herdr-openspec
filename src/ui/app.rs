//! `Dashboard`, `Route`, and pure key handling. See
//! `openspec/changes/tui-shell/design.md` -> Boundaries and Contracts.

use std::path::PathBuf;

use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::changes::{Change, ChangeSet};

/// The artifact read, injected rather than imported: the same shape by
/// which `resolve::openspec_bin`'s `npm prefix -g` probe and
/// `config::env_lookup` take their environment-dependent hook. The crate's
/// one production binding for this lives in `src/ui/mod.rs`, beside
/// `ui::load`; this file names no I/O API and never calls that binding by
/// name. See `openspec/changes/detail-view/design.md` -> Contracts and
/// Decisions.
pub type ArtifactReader<'a> = &'a dyn Fn(&std::path::Path) -> Result<String, String>;

/// The clipboard write, injected on exactly `ArtifactReader`'s terms and threaded into
/// `run_loop` beside it: `run_loop` has no `TerminalOps` handle today, so a completing
/// selection had no reachable call site without this (design.md -> Decision 11). `String`
/// rather than `TerminalError` — the seam speaks the same shape `ArtifactReader` does — and
/// the crate's one production binding, in `src/ui/mod.rs`'s `run`, maps a `TerminalGuard`'s
/// `write_clipboard` (which does return `TerminalError`) down to its `Display` text. No view
/// file names it: `apply`/`apply_select` stay pure, and the write happens only in `run_loop`,
/// on the completing phase (`text-selection`'s design.md -> Decision 12).
pub type ClipboardWriter<'a> = &'a dyn Fn(&str) -> Result<(), String>;

/// `settings-window`'s addition, on exactly `ClipboardWriter`'s terms: the settings panel's
/// commit write, injected into `run_loop` rather than named directly — this file and
/// `src/ui/driver.rs` are both swept for a filesystem API and neither may name one. The
/// crate's one production binding, built in `src/ui/mod.rs`'s composition root, closes over
/// the resolved state directory and maps the underlying `io::Error` down to its `Display`
/// text, on exactly `ClipboardWriter`'s own mapping. `Ok` records nothing further; `Err`
/// carries the operating system's own reason, which `run_loop` turns into a problem row on
/// the same `!`-row channel a failed clipboard write already uses.
pub type KindRecorder<'a> = &'a dyn Fn(&str) -> Result<(), String>;

/// `settings-window`'s addition: `run_loop`'s three synchronous, injected seams to the
/// outside world — bundled for the same reason [`crate::ui::driver::Live`] bundles the loop's
/// three background collaborators (its own doc comment states the rule this follows):
/// `record_kind` joining `read` and `write` as a fourth trailing parameter took `run_loop` to
/// eight arguments, which is measured, on this crate and toolchain, to be exactly where
/// clippy's `too_many_arguments` fires — cohesion is the real reason for the struct on both
/// sides of that threshold, on exactly `Startup`'s own precedent (`src/ui/mod.rs`). Every
/// field keeps its own type's name and its own doc comment; this struct adds no behaviour of
/// its own.
pub struct Seams<'a> {
    pub read: ArtifactReader<'a>,
    pub write: ClipboardWriter<'a>,
    pub record_kind: KindRecorder<'a>,
}

// `Route` is pulled forward from this group (3) into group 2's commit: `ui::layout`'s
// `split_body` needs it for the narrow-mode single-region case, and layout.rs is built
// before this file's `Dashboard`/`Action`/`action_for` content. Recorded as a deliberate,
// minimal exception in tasks.md group 2 and group 3 — `Route` still lives here, exactly
// where design.md -> Boundaries assigns it; only its *arrival time* moved.
/// The two panes the dashboard can show. `list-view`, `markdown-viewer`, and
/// `detail-view` fill their content; this change only routes between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    List,
    Detail,
}

/// The twenty-six outcomes a terminal event can map to, under either filter mode. The
/// count has moved seven times since this comment was last true: to thirteen with
/// `live-refresh`'s `Refresh`, to seventeen with `agent-launch`'s `LaunchApply`,
/// `LaunchContinue`, `LaunchArchive`, and `FocusAgent`, to eighteen with `list-sections`'s
/// `ToggleSection`, to twenty-three with `mouse-input`'s `SelectNext`, `SelectPrev`,
/// `ScrollDown`, `ScrollUp`, and `Click`, to twenty-four with `help-overlay`'s
/// `ToggleHelp`, to twenty-five with `text-selection`'s `Select`, and to twenty-six
/// with `settings-window`'s `ToggleSettings`.
/// `action_for` is total over every `Event`. `Back` replaces the earlier
/// `BackToList`: it now dismisses one of several layers rather than only
/// ever returning to the list route. `Next` and `Prev` are renamed from
/// `SelectNext` and `SelectPrev`: the action is route-agnostic — the list
/// selection at `Route::List`, the detail scroll at `Route::Detail` — and
/// a name asserting one of the two would be false half the time. See
/// `specs/dashboard-loop/spec.md`, `specs/detail-scroll/spec.md`, and
/// `specs/list-filtering/spec.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    OpenDetail,
    Back,
    Next,
    Prev,
    SelectTab(usize),
    NextTab,
    PrevTab,
    FilterStart,
    FilterPush(char),
    FilterPop,
    /// `live-refresh`'s addition: request a full refresh. Route-agnostic in
    /// a stronger sense than `Next`/`Prev` — it names no region at all.
    Refresh,
    /// `agent-launch`'s additions: launch an agent onto the selected change
    /// to apply it, to create its next artifact, or to archive it, or focus
    /// the agent already attributed to it. What each sends is `launch`'s own
    /// CLI-driven prompt for that intent, built from the resolved `openspec`
    /// path — never a client's own slash command. Four flat variants rather than one
    /// carrying an `Intent`, so a hand-written enumeration cannot silently
    /// omit one. Route-agnostic in the same stronger sense as `Refresh`:
    /// they act on the selected change, which is the same change at either
    /// route.
    LaunchApply,
    LaunchContinue,
    LaunchArchive,
    FocusAgent,
    /// `list-sections`'s addition: fold or unfold the section the cursor is
    /// on or in. Mapped from `Char(' ')` outside filter mode; inside it, a
    /// space types into the query on the same terms as every other
    /// printable character.
    ToggleSection,
    /// `help-overlay`'s addition: open the help overlay when it is closed and
    /// close it when it is open. Mapped from `Char('?')` outside filter mode,
    /// under both `KeyModifiers::NONE` and `KeyModifiers::SHIFT` — the one key
    /// with two accepted modifier values, because terminals disagree about
    /// whether `Shift` is reported alongside a shifted character like `?`.
    /// Inside filter mode, `?` types into the query on the same terms as
    /// every other printable character. Route-agnostic in the same stronger
    /// sense as `Refresh`: it opens a layer over whichever route is current
    /// and leaves `route` alone.
    ToggleHelp,
    /// `mouse-input`'s five. `action_for` maps **no key** to any of these
    /// *names*: they exist because a mouse event names the region or the row
    /// it landed on, where a key does not, and `ui::driver::mouse_action` is
    /// their only producer. That is not the same as being mouse-**only** —
    /// every one of them has a key above reaching an identical `Dashboard`
    /// under a different name, which is what keeps the pane usable with no
    /// pointer at all, and `tests/doc_contract.rs` pins the five by name for
    /// exactly that reason. `Select`, declared after them, is the enum's one
    /// variant with no keyboard path of any name (`text-selection`), and it
    /// carries a phase rather than being flat.
    ///
    /// `SelectNext`/`SelectPrev` move the list selection and
    /// `ScrollDown`/`ScrollUp` move the detail offset **at either route**,
    /// unlike `Next`/`Prev`, which do one or the other depending on it — which
    /// is what lets the wide layout's two regions scroll independently.
    ///
    /// Five flat variants rather than two carrying a direction or a region, for
    /// the same reason the four launch variants are flat: an enumerate-by-hand
    /// test must enumerate the same things the exhaustive `match` does.
    /// `Click` carries a `Target` because a click's subject is genuinely data —
    /// the row — not a fixed choice from a small set (design.md -> Decision 3).
    SelectNext,
    SelectPrev,
    ScrollDown,
    ScrollUp,
    Click(Target),
    /// `text-selection`'s addition, and the pane's one mouse-only action
    /// (`specs/mouse-input/spec.md`'s pinned exemption): a left press or drag
    /// over a non-header row of the detail content area. Carries the phase
    /// rather than costing three `Action` variants — a `Begin` for a press
    /// and an `Extend` for a drag — per design.md -> Decision 5: each
    /// variant costs a `Mouse` row in `ui::help::INVENTORY`, and every such
    /// row moves `binding-inventory`'s pinned counts and `help-overlay`'s row
    /// arithmetic, where one row reading "select text in the artifact area"
    /// is also what a reader needs. `ui::driver::mouse_action` is the only
    /// producer; `Dashboard::apply_select` counts consecutive presses at one
    /// cell through `Selection::granularity` alone, with no clock
    /// (design.md -> Decision 2).
    Select(SelectPhase),
    /// `settings-window`'s addition: open the settings panel when no panel is
    /// open, swap to it from the help panel, and close it when it is already
    /// open — `,`'s own three-way toggle, on exactly `ToggleHelp`'s terms but
    /// for the second panel (design.md -> Decisions 1 and 2). Mapped from
    /// `Char(',')` outside filter mode, under `KeyModifiers::NONE` only —
    /// unlike `?`, `,` carries no shifted-character ambiguity. Inside filter
    /// mode, `,` types into the query on the same terms as every other
    /// printable character. Route-agnostic in the same stronger sense as
    /// `Refresh` and `ToggleHelp`: it opens a layer over whichever route is
    /// current and leaves `route` alone.
    ToggleSettings,
    Ignore,
}

/// The phase [`Action::Select`] carries. Both variants carry the press or
/// drag's own content-line index and display column, already resolved
/// against the frame just drawn by `ui::driver::mouse_action` — `Dashboard`
/// has neither the width nor the row list to recompute either, on the same
/// terms `Target::DetailHeader` carries its own resolved indices (design.md
/// -> Decision 7's note, carried forward by Decision 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectPhase {
    /// A left press. `Dashboard::apply_select` decides, from the standing
    /// `Selection` alone, whether this is a first press at this cell (arms),
    /// a second at the same cell (widens to the word), or a third or later
    /// (widens to the row) — see `specs/text-selection/spec.md` -> "A press
    /// arms, a second selects the word, a third selects the row".
    Begin { line: usize, column: u16 },
    /// A left drag: the focus follows to `line`/`column` while the anchor
    /// holds. See `specs/text-selection/spec.md` -> "A drag selects a span
    /// of the rendered artifact".
    Extend { line: usize, column: u16 },
}

/// The `/` filter's mode and query. Deliberately implements no `Default`,
/// anywhere in the crate, on the same terms as `Dashboard`: every
/// construction and destructuring names both fields, with no `..` rest. See
/// `specs/list-filtering/spec.md` and the `NODEFAULT-UI` check, whose type
/// list now covers this type too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    pub query: String,
    pub active: bool,
}

/// `artifact-folds`' addition: one file of a multi-file artifact, once the
/// injected reader has read it. `label` names the file — for
/// `specs/<capability>/spec.md`, the capability directory — and `text` is
/// the reader's bytes **verbatim**: no separator inserted between two
/// sections and no newline added, unlike the single concatenated `source`
/// string this type replaces. Labels are not required to be unique;
/// sections are addressed by index everywhere, never by label. Joins the
/// `NODEFAULT-UI` type list, so every construction site names all three
/// fields.
///
/// `heading-sections` widened it: `label` is an `Option` so that a split
/// file's text before its first heading can carry no label at all, and
/// `depth` is that section's nesting level within the flat, index-addressed
/// list — `0` for a whole file, deeper for a heading inside one. Every site
/// this change touched passes `Some(..)` and `0`; the producers that supply
/// anything else arrive with the splitter.
/// See `specs/artifact-folds/spec.md` -> "A multi-file artifact's content
/// is a list of named sections" and design.md -> Decision 1 and Decision 2.
///
/// `tasks-emphasis` adds a fourth field. `progress` is `Some` for exactly the
/// **heading sections of a split tracked-tasks file** and `None` everywhere
/// else — on every file section, every preamble, every section of an unsplit
/// file, and every section of every other artifact — so a reader can fold a
/// completed task group without losing how far along it was. It is an
/// `Option<Progress>` rather than a `Progress` defaulting to `{0, 0}` because
/// the two mean different things on a header row: a `[-]` cell would claim the
/// section was counted and found empty, when a spec file's section is not a
/// task group at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactSection {
    pub label: Option<String>,
    pub text: String,
    pub depth: usize,
    pub progress: Option<crate::tasks::Progress>,
    pub operation: Option<crate::specs::DeltaOp>,
}

/// The label `artifact_section_label` derives for `path`, relative to
/// `change_dir`: when the file name is `spec.md` and the relative path has
/// at least one parent component, the label is that parent's final
/// component; otherwise the label is the file name. Total over every path,
/// including one not under `change_dir`, an empty path, and one whose
/// relevant path component is not valid UTF-8 — each of those falls back to
/// the file name or the empty string rather than panicking. See
/// `specs/artifact-folds/spec.md` -> "The label derivation is total over
/// adversarial paths" and design.md -> Decision 5.
fn artifact_section_label(change_dir: &std::path::Path, path: &std::path::Path) -> String {
    let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
        return String::new();
    };
    if file_name == "spec.md"
        && let Ok(relative) = path.strip_prefix(change_dir)
    {
        let components: Vec<_> = relative.components().collect();
        if components.len() > 1
            && let std::path::Component::Normal(parent) = components[components.len() - 2]
        {
            return match parent.to_str() {
                Some(s) => s.to_string(),
                None => String::new(),
            };
        }
    }
    file_name.to_string()
}

/// `heading-sections`' addition: one heading and the text beneath it, as
/// [`split_headings`] derives it. `level` is the count of `#` characters,
/// `label` the heading's remainder with surrounding whitespace trimmed, and
/// `body` the verbatim bytes between that heading's line and the next heading
/// line at **any** level. See `specs/artifact-folds/spec.md` -> "Headings
/// split a file into nested sections".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadingSection {
    pub level: u8,
    pub label: String,
    pub body: String,
}

/// The crate's **one** rule for deriving sections from a document's own
/// headings — the same function for a task file and for a spec file, because
/// two implementations of one rule drift.
///
/// A line is a heading exactly when, **outside a fenced code block**, it
/// begins with at most three spaces of indent, then one to six `#`
/// characters, then at least one space, then a non-empty remainder after
/// trimming. Fences are tracked: a trimmed line beginning with three or more
/// backticks or three or more tildes opens one, and the next trimmed line
/// beginning with at least as many of the **same** character closes it. Every
/// line inside an open fence, the fence lines included, is body text — this
/// repository's own `tasks.md` files carry well over a thousand `# comment`
/// lines inside `sh` fences, each of which would otherwise become a level-one
/// section (design.md -> Decision 4).
///
/// The returned sections **partition** the text: `body` is the verbatim bytes
/// between a heading's line and the next heading line at any level, so
/// reassembling the heading lines and bodies reproduces the input, less the
/// preamble before the first heading, which is no section (design.md ->
/// Decision 6). Pure and total: no I/O, no panic for any input, no `ratatui`
/// type, and no display-width measurement at all — truncation is the header
/// row's job, and measuring no width is why this is sited here rather than in
/// `src/ui/detail.rs` (design.md -> Decision 11).
///
/// See `specs/artifact-folds/spec.md` -> "Headings split a file into nested
/// sections".
pub fn split_headings(text: &str) -> Vec<HeadingSection> {
    let mut sections = Vec::new();
    // The heading whose body is still open: its level, its label, and the byte
    // offset its body starts at.
    let mut open: Option<(u8, String, usize)> = None;
    // The fence currently open: its character and the length of its run.
    let mut fence: Option<(char, usize)> = None;
    let mut start = 0;

    for line in text.split_inclusive('\n') {
        let end = start + line.len();
        let trimmed = line.trim();

        if let Some((ch, run)) = fence {
            if marker_run(trimmed, ch) >= run {
                fence = None;
            }
        } else if let Some(opener) = fence_opener(trimmed) {
            fence = Some(opener);
        } else if let Some((level, label)) = heading_of(line) {
            if let Some((prev_level, prev_label, body_start)) = open.take() {
                sections.push(HeadingSection {
                    level: prev_level,
                    label: prev_label,
                    body: text[body_start..start].to_string(),
                });
            }
            open = Some((level, label, end));
        }

        start = end;
    }

    if let Some((level, label, body_start)) = open {
        sections.push(HeadingSection {
            level,
            label,
            body: text[body_start..].to_string(),
        });
    }
    sections
}

/// The length of the run of `ch` at the start of `trimmed`. Both fence
/// characters and `#` are one byte, so this byte arithmetic counts them
/// exactly — and it is not a display-width measurement, which this file is
/// swept for and which a run of markers does not need.
fn marker_run(trimmed: &str, ch: char) -> usize {
    trimmed.len() - trimmed.trim_start_matches(ch).len()
}

/// The fence `trimmed` opens, if any: its character and the length of its run.
fn fence_opener(trimmed: &str) -> Option<(char, usize)> {
    ['`', '~'].into_iter().find_map(|ch| {
        let run = marker_run(trimmed, ch);
        (run >= 3).then_some((ch, run))
    })
}

/// The `(level, label)` pair `line` is a heading for, if it is one. `line` may
/// carry its own line terminator. Fences are the caller's business: this
/// answers only the ATX shape.
fn heading_of(line: &str) -> Option<(u8, String)> {
    let line = line.strip_suffix('\n').unwrap_or(line);
    let line = line.strip_suffix('\r').unwrap_or(line);

    let unindented = line.trim_start_matches(' ');
    if line.len() - unindented.len() > 3 {
        return None;
    }

    let after = unindented.trim_start_matches('#');
    let level = u8::try_from(unindented.len() - after.len()).ok()?;
    if !(1..=6).contains(&level) || !after.starts_with(' ') {
        return None;
    }

    let label = after.trim();
    (!label.is_empty()).then(|| (level, label.to_string()))
}

/// Whether `text` is a specification delta: it carries at least one level-3
/// heading whose label begins `Requirement:`.
///
/// Asks [`split_headings`] rather than scanning the text a second time, which
/// is what makes a `### Requirement:` quoted inside a fence — as this
/// repository's own planning documents do constantly — not count. Two scans
/// of one rule would drift, and this one would drift in the direction of
/// folding prose. See `specs/artifact-folds/spec.md` -> "A file splits at its
/// headings only when it is a spec or a tracked task file".
pub fn is_spec_shaped(text: &str) -> bool {
    has_requirement_heading(&split_headings(text))
}

/// A level-3 heading labelled `Requirement:` — the crate's one predicate for
/// "this heading opens a requirement", shared by [`has_requirement_heading`]
/// and, since `spec-emphasis`, `Dashboard::sync_detail`'s operation walk
/// (design.md -> Decision 4). Written once so the two never drift apart.
fn is_requirement_heading(section: &HeadingSection) -> bool {
    section.level == 3 && section.label.starts_with("Requirement:")
}

/// The spec half of the split gate, over sections already derived.
/// `Dashboard::sync_detail` asks this rather than [`is_spec_shaped`] because
/// it holds the sections already and `artifact-content` allows it exactly one
/// `split_headings` call per successfully read path; writing the predicate
/// twice would be the drift `is_spec_shaped`'s own doc comment warns about.
fn has_requirement_heading(sections: &[HeadingSection]) -> bool {
    sections.iter().any(is_requirement_heading)
}

/// The indices of `sections` a tracked-tasks tab opens with:
/// `artifact-folds`' seed, every section whose **subtree** is incomplete and
/// no other. A section's subtree is itself and every following section of
/// strictly greater `depth`, up to the first section of `depth` at or below
/// its own; it is incomplete when `tasks::count` over the concatenation of
/// that subtree's `text` values reports `completed < total`.
///
/// The subtree, not the section's own text (design.md -> D5): a group's own
/// `text` holds only the lines before its first child heading, so a `tasks.md`
/// with `###` sub-headings would otherwise report the parent itemless and
/// leave it collapsed over incomplete work. A subtree holding no items at all
/// is **not** seeded — `completed < total` is false at `total == 0` — because
/// opening it would show nothing.
///
/// Called by `sync_detail` on the key change and nowhere else, which is what
/// keeps an agent checking a task off in another pane from folding a group
/// shut under the reader.
fn seed_expanded(sections: &[ArtifactSection]) -> std::collections::BTreeSet<usize> {
    let mut seeded = std::collections::BTreeSet::new();
    for (index, section) in sections.iter().enumerate() {
        let end = sections[index + 1..]
            .iter()
            .position(|s| s.depth <= section.depth)
            .map_or(sections.len(), |offset| index + 1 + offset);
        let subtree: String = sections[index..end]
            .iter()
            .map(|s| s.text.as_str())
            .collect();
        let progress = crate::tasks::count(&subtree);
        if progress.completed < progress.total {
            seeded.insert(index);
        }
    }
    seeded
}

/// The byte length of `text`'s preamble — everything before the first heading
/// line `split_headings` recognised — derived from `sections` themselves
/// rather than from a second, fence-aware scan of the text, which would be
/// the second implementation of one rule this change exists to avoid.
///
/// Walks backwards: the last section's `body` is a suffix of `text`, so the
/// heading line above it ends where that body starts, and stepping over that
/// line lands on the end of the previous section's body. After every section
/// the offset is the start of the first heading line. Total: an offset that
/// does not land on a character boundary — which the partition rules out —
/// falls back to the whole text rather than panicking.
fn preamble_len(text: &str, sections: &[HeadingSection]) -> usize {
    let mut pos = text.len();
    for section in sections.iter().rev() {
        pos = pos.saturating_sub(section.body.len());
        let Some(head) = text.get(..pos) else {
            return text.len();
        };
        let line = head.strip_suffix('\n').unwrap_or(head);
        pos = line.rfind('\n').map_or(0, |i| i + 1);
    }
    pos
}

/// The index into `headings` of the document's **title heading**, per
/// `specs/artifact-folds/spec.md`'s three clauses — recognised on a
/// **tracked-tasks** artifact only, never a spec, per `title-heading-preamble`'s
/// design.md -> D2:
///
/// 1. it is the first heading `split_headings` returned;
/// 2. it is the only heading at the file's smallest returned `level`;
/// 3. `tasks::count` over its own `body` reports a `total` of zero.
///
/// `None` when `tracks_tasks` is `false`, `headings` is empty, or any
/// clause fails; `Some(0)` otherwise, a title being by construction always
/// the first heading in document order. Pure and total, sited beside
/// `preamble_len` for the same reason: it measures no display width, so
/// `COLWIDTH` and the `*WIDTHS` gates are unaffected. Recognition alone —
/// whether a **non-empty-after-trimming** body then contributes a section
/// is a separate question the caller decides, per `title-heading-preamble`'s
/// design.md -> D8.
fn title_heading(headings: &[HeadingSection], tracks_tasks: bool) -> Option<usize> {
    if !tracks_tasks {
        return None;
    }
    let first = headings.first()?;
    let min_level = headings.iter().map(|h| h.level).min()?;
    if first.level != min_level {
        return None;
    }
    if headings.iter().filter(|h| h.level == min_level).count() != 1 {
        return None;
    }
    if crate::tasks::count(&first.body).total != 0 {
        return None;
    }
    Some(0)
}

/// The detail region's content and scroll offset, plus `detail-view`'s three
/// additions. `sections` is set by `Dashboard::sync_detail`, driven once per
/// loop iteration by the injected `ArtifactReader`; `ui::load` still starts
/// it empty. `artifact-folds`' addition: a path the reader failed on
/// contributes no entry, so `sections.len()` is not `paths.len()` when a read
/// failed. `scroll` is a user-controlled position, the detail region's
/// counterpart to `Dashboard::selected`, not derived geometry: the offset
/// actually drawn is still recomputed on every draw by `layout::scroll_offset`
/// (or, at a foldable artifact, `layout::viewport`) against the current
/// content area's height. `tab` is the selected artifact's position in the
/// selected change's `artifacts`; `problems` names each artifact file that
/// could not be read; `loaded` is the `(change directory, tab)` key whose
/// content `sections` currently holds — `sync_detail`'s cache key, and the
/// reason an unchanged selection re-reads nothing. `expanded` is
/// `artifact-folds`' addition: the indices of `sections` that are **open**,
/// inverting `Sections { collapsed }` on purpose (design.md -> Decision 4) —
/// the empty set means every section is collapsed, so no construction site
/// needs to seed it. `sync_detail` clears it on exactly the condition that
/// resets `scroll` — the `(change directory, tab)` key changed — and never
/// on a forced-but-unchanged-key reload; `adopt` does not touch it at all.
/// Deliberately implements no `Default`, anywhere in the crate, on the same
/// terms as `Dashboard` and `Filter`: every construction and destructuring
/// names all six fields, with no `..` rest. See `specs/detail-scroll/spec.md`,
/// `specs/artifact-tabs/spec.md`, `specs/artifact-content/spec.md`,
/// `specs/artifact-folds/spec.md`, and the `NODEFAULT-UI` check, whose type
/// list covers this type too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detail {
    pub sections: Vec<ArtifactSection>,
    pub scroll: usize,
    pub tab: usize,
    pub problems: Vec<String>,
    pub loaded: Option<(PathBuf, usize)>,
    pub expanded: std::collections::BTreeSet<usize>,
    /// The **content area's own width at the frame last drawn**, recorded by
    /// [`Dashboard::normalise_scroll`], and `None` until a first frame exists.
    ///
    /// Derived geometry, deliberately cached, and the one exception to
    /// `Dashboard`'s "carries no width" rule (design.md -> Decision 13).
    /// `content_lines`' row list is width-dependent — `ui::markdown` word-wraps
    /// at the content width, and this crate's own width property asserts the row
    /// count is strictly greater at 58 than at 78 — while `Space` at
    /// `Route::Detail` must fold the section the cursor is *on or in*, resolving
    /// `scroll` through that same list. Resolving at any other width can name a
    /// different section than the one the reader sees emphasised.
    ///
    /// `ui::view::render` never reads it: the render path has the real width in
    /// hand, so this is only ever what a **keypress between frames** resolves
    /// against, never a source of what is drawn.
    pub drawn_width: Option<u16>,
}

impl Detail {
    /// Whether the selected artifact is **foldable**: more than one resolved
    /// section. Derived, never stored (design.md -> Decision 3), and named here
    /// so it is decided in **one** place — `ui::detail` asks it to decide
    /// whether to emit header rows, `ui::view` to choose between
    /// `layout::viewport` and `layout::scroll_offset`, and `Dashboard::apply`
    /// and `normalise_scroll` to decide whether `Space` and the clamp act on a
    /// cursor at all. Four inline `sections.len() > 1` tests would be four
    /// places for the answer to drift.
    ///
    /// A single-section artifact and one with no sections are both **not**
    /// foldable, which is what keeps a one-file artifact byte-identical to its
    /// pre-change rendering.
    pub fn foldable(&self) -> bool {
        self.sections.len() > 1
    }
}

/// The loop's live tier state: `requested` and `reload` are one-shot flags
/// the loop consumes, `problems` is text that outlives a reload — none of it
/// is derived geometry, and none of it is a watcher, a worker handle, a
/// channel, or the clock. `requested` is set by `Action::Refresh` and by
/// `ui::load` at startup, and cleared by `run_loop` once it has asked the
/// refresher for a full reload; `Dashboard::apply` never reaches a
/// collaborator, so setting it stays a pure state change. `reload` is set by
/// `Dashboard::adopt` and consumed by `Dashboard::sync_detail`: it forces a
/// re-read of an unchanged `(change directory, tab)` key, which is what
/// makes an edit to the artifact currently on screen visible — the cache
/// key `sync_detail` compares does not change when a file's *content* does.
/// `problems` holds the **live** tier's own standing conditions: a
/// `FsEvents::drain` error, and a refresh worker reported stopped. Replaced
/// wholesale by each new one, never grown.
///
/// `seam-resilience`'s addition: `startup` — the standing conditions the
/// pane learns **once**, at startup, in causal order (the configuration's
/// own fallbacks, then the `openspec` binary probe's, then the watcher's
/// failure to start). Written exactly once, by `run_wired` from
/// `Collaborators::problems`, and never written again by anything — not by
/// `r`, not by a refresh, not by a watcher error. Splitting it from
/// `problems` is this change's correction of a real, reachable defect:
/// `degraded-states` seeded all three startup sources into `problems`, and
/// the loop's wholesale replacement on the first `FsEvents::drain` error
/// erased the rows naming a missing `openspec` binary and every
/// configuration fallback, for the rest of the session. See
/// `specs/live-updates/spec.md`.
///
/// Deliberately implements no `Default`, anywhere in the crate, on the same
/// terms as `Dashboard`, `Filter`, and `Detail`: every construction and
/// destructuring names all four fields, with no `..` rest. See the
/// `NODEFAULT-UI` check, whose type list covers this type too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refresh {
    pub requested: bool,
    pub reload: bool,
    pub startup: Vec<String>,
    pub problems: Vec<String>,
}

/// `agent-launch`'s addition: the launch tier's state. `pending` is the one-shot request
/// `Dashboard::apply` produced and the loop has not yet handed to the launcher; `problems`
/// holds the last outcome's failure or the last refusal, replaced wholesale and never grown, at
/// most one entry. A sibling of `Refresh` for the same reason `agents` is: a launch's answer
/// put anywhere `adopt` or a watcher error can replace wholesale would vanish before the reader
/// saw it. `pending` carries plain data — a `launch::Request` names no trait, no handle, and no
/// thread — so the state value stays `Clone`, `PartialEq`, and constructible in a test.
/// `seam-resilience`'s addition: `in_flight`, the single source of `launch::decide`'s
/// sixth argument. Set to `true` at the moment the loop hands `pending`'s `Request::Launch`
/// to `Launcher::request` — never when `Dashboard::apply` sets `pending`, so the flag
/// describes work actually handed over rather than work merely intended; left `false` for a
/// `Request::Focus`, which starts no agent and splits no pane and therefore cannot leak one;
/// cleared to `false` when `Launcher::drain` yields an `Outcome`, in the same step that folds
/// the outcome's `named` pair and problems into the dashboard — including when that
/// `Outcome` reports the launcher's own worker dead, since no answer will ever come. Never
/// derived from `pending` (a one-shot slot emptied on the very next iteration) or from
/// `agents.agents` (which lags the launch by up to a poll interval plus the launch's own
/// duration) — the two sources whose gap is exactly the window this flag closes.
///
/// Deliberately implements no `Default`, anywhere in the crate, on the same terms as
/// `Dashboard`, `Filter`, `Detail`, and `Refresh`: every construction and destructuring names
/// all three fields, with no `..` rest. See `specs/agent-launch/spec.md` and the
/// `NODEFAULT-UI` check, whose type list covers this type too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    pub pending: Option<crate::launch::Request>,
    pub problems: Vec<String>,
    pub in_flight: bool,
}

/// `list-sections`' addition: the two sections the list is split into. A
/// section is identified by a key rather than by a bare boolean so a later
/// change can nest a date grouping under `Archived` without reworking the
/// collapse state, the cursor index, or the `/` force-open rule — see
/// design.md -> Decision 8 and -> Non-Goals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SectionKey {
    Active,
    Archived,
}

/// `list-sections`' addition: the reader's own collapse state, one entry per
/// folded section. A section is collapsed exactly when its key is in this
/// set, so a key added later — a nested date group, say — defaults to open
/// with no migration. Deliberately implements no `Default`, anywhere in the
/// crate, on the same terms as `Filter` and `Detail`: every construction and
/// destructuring names its one field, with no `..` rest. See
/// `specs/list-selection/spec.md` and the `NODEFAULT-UI` check, whose type
/// list now covers this type too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sections {
    pub collapsed: std::collections::BTreeSet<SectionKey>,
}

/// `list-sections`' addition: one addressable row a click can land on.
/// `Section` and `Change` address the **list** region — a section header or
/// a change, the latter as an index into `Dashboard::visible()`.
/// `Dashboard::targets()` returns them in emission order, and
/// `Dashboard::selected` indexes into that vector rather than into
/// `visible()` directly, because a collapsed section's header is its only
/// row and must stay reachable. See design.md -> Decision 2.
///
/// `DetailHeader` is `foldable-spec-sections`' addition and addresses the
/// **detail** region instead; see its own doc comment and design.md ->
/// Decision 7 for why it carries resolved indices rather than a bare row
/// offset, and why `targets()` never returns it. `text-selection` removed
/// this enum's other former variant, which addressed an ordinary content
/// row: a press there now arms a selection (`Action::Select`) instead of
/// moving the detail cursor, per `specs/text-selection/spec.md` and
/// design.md -> Decision 9.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Section(SectionKey),
    /// An index into `Dashboard::visible()`.
    Change(usize),
    /// `settings-window`'s addition: a setting's row inside the settings
    /// panel's band — an index into `Dashboard::settings.rows`, addressed by
    /// either of its two rendered rows, the value row or the source row.
    /// Produced only by `ui::driver::mouse_action`, only while
    /// `overlay.panel` is `Some(Panel::Settings)`, through a band-relative
    /// hit test that is the exact inverse of `ui::settings::render`'s own
    /// row grammar (design.md -> Boundaries). Not returned by `targets()`,
    /// which addresses the list region alone, on exactly `DetailHeader`'s
    /// own terms below.
    Setting(usize),
    /// A section-header row of the detail region's content: its own
    /// content-line index beside its section index, both already resolved
    /// against the frame just drawn — `mouse_action` has the width to call
    /// `ui::detail::content_lines` and `ui::detail::section_at`;
    /// `Dashboard::apply` does not (design.md -> Decision 7). Not returned by
    /// `targets()`.
    DetailHeader {
        line: usize,
        section: usize,
    },
}

/// `help-overlay`'s addition, generalised by `settings-window`: the overlay
/// layer's own state — which panel is open, if any, the scroll position
/// within its own row sequence, and (`settings-window`'s addition) the
/// in-progress edit of a setting's value, if any. A sibling of `Filter` on
/// `filter.active`'s own terms: the overlay is a layer over whichever route
/// is current rather than a third `Route` variant, and closing it must
/// return the reader to that route, which a `Route::Help` (or a
/// `Route::Settings`) would have to remember separately.
///
/// `panel` replaces `help-overlay`'s own `open: bool` with `Option<Panel>`
/// rather than adding a second boolean beside it: two independent flags
/// would make "both panels open" a representable state that every dispatch
/// and every render would have to refuse, where `Option<Panel>` makes it
/// unrepresentable by construction (design.md -> Decision 1). `scroll` is a
/// user-controlled position on exactly `detail.scroll`'s terms, not derived
/// geometry, clamped on every draw by `ui::layout::scroll_offset`, and
/// belongs to the panel `panel` names — closing one panel and opening the
/// other starts that scroll at the top again, on the same terms
/// `apply_help_action`'s own `ToggleHelp` arm already reset it on open.
///
/// Deliberately implements no `Default`, anywhere in the crate, on the same
/// terms as `Dashboard`, `Filter`, `Detail`, `Refresh`, and `Launch`: every
/// construction and destructuring names all three fields, with no `..`
/// rest. See `specs/help-overlay/spec.md`, `specs/dashboard-loop/spec.md`,
/// and the `NODEFAULT-UI` check, whose type list now covers this type
/// (renamed from `Help`) and `Edit` too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overlay {
    pub panel: Option<Panel>,
    pub scroll: usize,
    pub edit: Option<Edit>,
}

/// `settings-window`'s addition: which of the two panels `Overlay` carries,
/// when one is open at all. Exhaustively matched wherever a panel's identity
/// decides behaviour — `Dashboard::apply_help_action` does not yet branch on
/// it (task group 1 is a rename, not new behaviour), but a `match` written
/// with no wildcard arm here is what makes a third panel added later a
/// compile error at every such site rather than a silently-inert branch. Not
/// named in `NODEFAULT-UI`'s scanned type sets: that gate's positive control
/// greps for `struct $T {`, and an enum has no such declaration to find
/// (task 10.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Help,
    Settings,
}

/// `settings-window`'s addition: an in-progress edit of one setting's value,
/// begun on `OpenDetail` and committed on `OpenDetail`, cancelled on `Back`
/// (task group 5). `setting` is the index into `settings::PanelState::rows`
/// of the row being edited; `candidate` is the index into that row's own
/// shortlist of the value currently highlighted, which `Next`/`Prev` cycle
/// with wrap while an edit is in progress. Both are plain `usize` rather
/// than a resolved value: the candidate is rendered instead of the
/// committed value while editing (task 5.3), so the edit's own state is
/// nothing more than "which row, which position in its shortlist" and needs
/// no copy of the value itself. Deliberately implements no `Default`, on the
/// same terms as `Overlay` and every other state type in this module: every
/// construction and destructuring names both fields, with no `..` rest. See
/// `specs/dashboard-loop/spec.md` and the `NODEFAULT-UI` check, whose type
/// list now covers this type too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edit {
    pub setting: usize,
    pub candidate: usize,
}

/// `text-selection`'s addition: the drag-selection state. `None` when nothing is
/// selected. A sibling of `detail` rather than an eighth `Detail` field:
/// `sync_detail` reloads `Detail` wholesale on a tab switch, a selection change, or
/// an adopted refresh, and a selection living inside it would be silently discarded
/// by a reload rather than deliberately cleared by one (design.md -> Decision 4).
/// Deliberately implements no `Default`, anywhere in the crate, on the same terms as
/// `Dashboard`, `Filter`, `Detail`, `Refresh`, `Launch`, and `Help`: every
/// construction and destructuring names every field, with no `..` rest. See
/// `specs/text-selection/spec.md`, design.md -> Decision 3 and Decision 12, and the
/// `NODEFAULT-UI` check, whose type list now covers this type too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    /// The line index into `ui::detail::content_lines` and the display column where
    /// the selection began. Held even after `focus` moves past it, so a drag back
    /// the other way still selects the same span.
    pub anchor: (usize, u16),
    /// The line index and display column the selection currently extends to.
    pub focus: (usize, u16),
    /// What a first press, a second, or a third has widened the span to. See
    /// design.md -> Decision 2 and Decision 3.
    pub granularity: Granularity,
    /// The reason the completing clipboard write failed, rendered as a
    /// detail-region problem row beside `detail.problems`. `None` before a write is
    /// attempted and after one succeeds — design.md -> Decision 6 and Decision 12.
    /// It lives here rather than on any of the five existing `!`-marked lists
    /// because each of those is replaced wholesale on its own producer's cadence
    /// and would drop the reason before the reader saw it.
    pub problem: Option<String>,
}

/// What a [`Selection`] currently covers. No clock names any of these: consecutive
/// presses at one cell count through this field alone (design.md -> Decision 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Granularity {
    /// A first press with no motion: `anchor` and `focus` sit at the same cell, and
    /// nothing is drawn.
    Armed,
    /// A second press at the same cell: the span widens to the whole word under it.
    Word,
    /// A third press at the same cell: the span widens to the whole row.
    Row,
    /// A drag: the span runs from `anchor` to wherever `focus` last moved to.
    Span,
}

/// The dashboard's whole state. Carries no layout mode, no terminal handle, and
/// no frame — those are derived from the frame area on every draw, never stored
/// here. **One exception, and only one:** `Detail::drawn_width` records the
/// content width of the frame last drawn, because a keypress taken *between*
/// frames has to resolve against what the last one did and the render path is
/// pure and cannot tell it (design.md -> Decision 13). It is never a source of
/// what is drawn. Deliberately implements no
/// `Default`, anywhere in the crate: every construction and every
/// destructuring names all seventeen fields, so a field added later fails to
/// compile at each site rather than defaulting silently. See
/// `specs/dashboard-loop/spec.md` and the `NODEFAULT-UI` check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dashboard {
    /// The repository root, when one was found.
    pub repo: Option<PathBuf>,
    /// The directory the walk began at — kept for `list-view`'s empty state.
    pub searched_from: PathBuf,
    pub changes: ChangeSet,
    pub route: Route,
    pub quit: bool,
    /// The index into the **visible targets** — `targets()`'s output,
    /// section headers and visible changes in emission order — not into
    /// `visible()` directly. `list-sections`' change to what this index
    /// addresses: see `specs/list-selection/spec.md`.
    pub selected: usize,
    /// The `/` filter's mode and query. See `specs/list-filtering/spec.md`.
    pub filter: Filter,
    /// The detail region's markdown source and scroll offset. Nothing reads
    /// or writes it in this change beyond startup, which leaves it empty and
    /// unscrolled. See `specs/detail-scroll/spec.md`.
    pub detail: Detail,
    /// The live tier's state: the startup/`r`-triggered refresh request, the
    /// forced-reload flag, and standing watcher problems. See
    /// `specs/live-updates/spec.md`.
    pub refresh: Refresh,
    /// The most recent agent poll's outcome. Replaced wholesale, never
    /// merged: a poll that found no agents means there are no agents.
    /// `ui::list` and `ui::view` read it, but only through `attribution()`;
    /// see `specs/agent-poller/spec.md`.
    pub agents: crate::agents::AgentSnapshot,
    /// The plugin-local agent-name mapping, read once by `ui::load` from the
    /// state directory `Startup` carries. `agent-attribution`'s first tier
    /// resolves an agent's name through it. See `specs/dashboard-loop/spec.md`.
    pub agent_names: crate::state::Mapping,
    /// `agent-launch`'s addition: the launch tier's one-shot request and last reported
    /// problem. See `specs/agent-launch/spec.md`.
    pub launch: Launch,
    /// `list-sections`' addition: the reader's own per-session fold state. Survives every
    /// `adopt`, which never touches it. See `specs/list-selection/spec.md`.
    pub sections: Sections,
    /// `degraded-states`' addition: whether the binary probe resolved nothing. Set only by
    /// the composition root (`run_wired`, via `start_collaborators`'s real probe) -- `ui::load`
    /// sets `false` on both branches, since it never probes for a binary. Read only by the
    /// header badge (`ui::view::render_header`). See `specs/responsive-layout/spec.md`.
    pub file_mode: bool,
    /// `help-overlay`'s addition, generalised by `settings-window`: the overlay
    /// layer's state, carrying whichever panel is open (if any) rather than only
    /// the help panel. See `specs/help-overlay/spec.md` and
    /// `specs/dashboard-loop/spec.md`.
    pub overlay: Overlay,
    /// `text-selection`'s addition: the drag-selection state, `None` when nothing is
    /// selected. Cleared by `sync_detail` exactly when the detail content reloads —
    /// never by living inside `Detail` and being silently discarded by one. See
    /// `specs/text-selection/spec.md` and design.md -> Decision 3 and Decision 4.
    pub selection: Option<Selection>,
    /// `settings-window`'s addition, the seventeenth field: the settings panel's own
    /// rows and row cursor. `rows` is produced by `settings::settings` outside the
    /// render path — at startup, on an adopted kind resolution, and on a commit — and
    /// the view derives nothing from it. `cursor` is a sibling of `overlay` rather than
    /// a member of it: `overlay.scroll` already means the help band's first visible
    /// row, and the settings panel needs a selected **setting** instead, windowed by
    /// `ui::layout::viewport`; living beside the rows it indexes also lets it outlive
    /// the panel, so reopening `,` returns the reader to the setting they left. See
    /// `specs/dashboard-loop/spec.md` and design.md -> Decision 13.
    pub settings: crate::settings::PanelState,
}

impl Dashboard {
    /// Apply `action`, mutating only the field(s) it names. `Ignore` changes
    /// nothing. `help-overlay`'s overlay layer (`apply_help_action`) takes
    /// precedence over the ordinary route dispatch (`apply_route_action`)
    /// whenever `overlay.panel` is `Some` — see design.md -> Decision 2. After
    /// either, `list-sections`' one blanket rule runs for **every** action:
    /// `refresh.requested` is set to `true` when `needs_archived_refresh()`
    /// holds, so a future key that opens a section cannot forget to trigger
    /// its resolution. See design.md -> Decision 6.
    pub fn apply(&mut self, action: Action) {
        match self.overlay.panel {
            // `help-overlay`: this branch takes precedence over the route
            // dispatch and the filter dispatch alike, and over every other
            // capability that owns an action's semantics. See
            // `apply_help_action` and design.md -> Decision 2 and Decision 5.
            Some(Panel::Help) => self.apply_help_action(action),
            // `settings-window`: added on exactly the help panel's terms — see
            // `apply_settings_action` and design.md -> Decision 2.
            Some(Panel::Settings) => self.apply_settings_action(action),
            None => self.apply_route_action(action),
        }
        // `list-sections`' one blanket rule, run after every action rather than a named
        // subset: see design.md -> Decision 6. It never clears the flag. `help-overlay`
        // leaves it running unconditionally while the overlay is open too: it is not an
        // action's effect and the overlay does not suppress it.
        if self.needs_archived_refresh() {
            self.refresh.requested = true;
        }
    }

    /// The route dispatch: every action's ordinary effect, reached only while
    /// `overlay.panel` is `None`. Pulled out of `apply` so the overlay's own
    /// precedence branch can sit ahead of it without duplicating the blanket
    /// `needs_archived_refresh()` rule that follows both. See
    /// `specs/dashboard-loop/spec.md`.
    fn apply_route_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.quit = true,
            // `list-sections`: inert while the cursor addresses a section header —
            // `selected_change()` is `None` there, so there is nothing for the detail
            // region to show, and moving to an empty one would be a worse answer to
            // `Enter` than doing nothing. Accepting an open filter is not opening a
            // detail, so that arm is unconditional on the target.
            Action::OpenDetail => {
                if self.filter.active {
                    self.filter.active = false;
                } else if self.route != Route::Detail
                    && !matches!(self.targets().get(self.selected), Some(Target::Section(_)))
                {
                    self.route = Route::Detail;
                    self.detail.scroll = 0;
                }
            }
            // Dismiss exactly one layer: filter mode with its query, when
            // active; else a non-empty query alone; else the route, back to
            // List; else nothing — so a stray `Esc` at the root cannot close
            // the pane. Only the first two layers can shrink the visible
            // list, so only they clamp `selected`; only the route layer
            // resets `detail.scroll` — dismissing the filter is not a route
            // move.
            Action::Back => {
                if self.filter.active {
                    self.filter.active = false;
                    self.filter.query.clear();
                    self.clamp_selection();
                } else if !self.filter.query.is_empty() {
                    self.filter.query.clear();
                    self.clamp_selection();
                } else if self.route == Route::Detail {
                    self.route = Route::List;
                    self.detail.scroll = 0;
                }
            }
            // Route-dependent: the list selection at `Route::List`, the
            // detail scroll at `Route::Detail`, never both. `mouse-input`
            // design.md -> Decision 4: these two arms are exactly a route
            // dispatch over the four region-explicit actions below and hold no
            // arithmetic of their own, so a key and a wheel over the same
            // region can never disagree about what one line means.
            Action::Next => match self.route {
                Route::List => self.select_by(1),
                Route::Detail => self.scroll_by(1),
            },
            Action::Prev => match self.route {
                Route::List => self.select_by(-1),
                Route::Detail => self.scroll_by(-1),
            },
            // `mouse-input`: the wheel names its region rather than inheriting
            // the route, so these four act at **either** route.
            Action::SelectNext => self.select_by(1),
            Action::SelectPrev => self.select_by(-1),
            Action::ScrollDown => self.scroll_by(1),
            Action::ScrollUp => self.scroll_by(-1),
            // `mouse-input`: a click names the row it landed on. Inert when that
            // target is absent from `targets()` — never clamped to a neighbour,
            // since moving the cursor somewhere the reader did not click is
            // worse than ignoring a click whose row is gone.
            Action::Click(target) => self.apply_click(target),
            // `sync_detail` (group 8) owns clamping `detail.tab` against the
            // selected change's artifact count; these arms perform only the
            // validity check `artifact-tabs` -> Decisions 4 and 5 state.
            Action::SelectTab(i) => {
                if let Some(change) = self.selected_change()
                    && i < change.artifacts.len()
                {
                    let before = self.detail.tab;
                    self.detail.tab = i;
                    if self.detail.tab != before {
                        self.detail.scroll = 0;
                    }
                }
            }
            Action::NextTab => {
                if let Some(change) = self.selected_change()
                    && !change.artifacts.is_empty()
                {
                    let last = change.artifacts.len() - 1;
                    let before = self.detail.tab;
                    self.detail.tab = (self.detail.tab + 1).min(last);
                    if self.detail.tab != before {
                        self.detail.scroll = 0;
                    }
                }
            }
            Action::PrevTab => {
                let before = self.detail.tab;
                self.detail.tab = self.detail.tab.saturating_sub(1);
                if self.detail.tab != before {
                    self.detail.scroll = 0;
                }
            }
            Action::FilterStart => {
                self.filter.active = true;
                self.route = Route::List;
                self.detail.scroll = 0;
            }
            Action::FilterPush(c) => {
                self.filter.query.push(c);
                self.clamp_selection();
            }
            Action::FilterPop => {
                self.filter.query.pop();
                self.clamp_selection();
            }
            // `live-refresh`: request a full refresh. Reaches no
            // collaborator and changes nothing else — `run_loop`'s live
            // tier is what turns the flag into a request. See
            // `specs/live-updates/spec.md` -> "`r` forces a full refresh".
            Action::Refresh => self.refresh.requested = true,
            // `agent-launch`: map the action to an `Intent`, decide, and write the result.
            // Reaches no collaborator, spawns nothing, touches no filesystem, reads no clock —
            // `apply` stays a pure function of `&mut self` and its argument; `run_loop` is what
            // turns `launch.pending` into a request to a collaborator outside `src/ui/`
            // entirely. See `specs/agent-launch/spec.md` and design.md -> Decisions 6.
            Action::LaunchApply
            | Action::LaunchContinue
            | Action::LaunchArchive
            | Action::FocusAgent => self.apply_launch_action(action),
            // `list-sections` / `artifact-folds`: route-dependent, exactly as `Next`
            // and `Prev` already are and for the same reason (design.md ->
            // Decision 6) — at `Route::List` fold or unfold exactly one **list**
            // section, moving `selected` to that section's header; at
            // `Route::Detail` fold or unfold exactly one **artifact** section,
            // moving `detail.scroll` to that section's header row. Neither arm
            // reaches a collaborator, spawns a process, touches the filesystem,
            // or reads a clock. See `specs/list-selection/spec.md` -> "`Space`
            // toggles the section the cursor is on or in" and
            // `specs/artifact-folds/spec.md` -> "`Space` toggles the artifact
            // section the cursor is on or in".
            Action::ToggleSection => match self.route {
                Route::List => self.apply_toggle_section(),
                Route::Detail => self.apply_toggle_detail_section(),
            },
            // `help-overlay`: reached only while the overlay is closed —
            // `apply` dispatches to `apply_help_action` instead while it is
            // open, and that branch closes it rather than reopening it. Open
            // it and reset `overlay.scroll`, changing nothing else.
            Action::ToggleHelp => {
                self.overlay.panel = Some(Panel::Help);
                self.overlay.scroll = 0;
            }
            // `settings-window`: reached only while no panel is open — `apply` dispatches to
            // `apply_settings_action` instead while the settings panel is open, and that
            // branch closes it rather than reopening it, on exactly `ToggleHelp`'s terms. Open
            // it and reset `overlay.scroll`, changing nothing else. Sending
            // `launch::Request::Resolve` on this transition is `agent-client-choice`'s later
            // group; this group leaves `agent_kind` at `Provenance::Pending` until it lands.
            Action::ToggleSettings => {
                self.overlay.panel = Some(Panel::Settings);
                self.overlay.scroll = 0;
            }
            // `text-selection`: a press or a drag over a non-header detail
            // content row. See `apply_select` for the press-count and
            // clamping rules (design.md -> Decision 2 and Decision 3).
            Action::Select(phase) => self.apply_select(phase),
            Action::Ignore => {}
        }
    }

    /// `text-selection`: apply a press or a drag to `self.selection`.
    ///
    /// A `Begin` counts consecutive presses at one cell through
    /// `Selection::granularity` alone, with no clock (design.md -> Decision
    /// 2): a first press at a new cell, or one whose standing selection is a
    /// dragged `Granularity::Span`, **arms** — anchor and focus both at that
    /// cell, drawn as nothing; a second press at the same cell **armed**
    /// widens to the word; a third or later at the same cell, already
    /// **word** or **row**, widens to (or stays at) the row. Widening never
    /// applies to a dragged span, which is exactly what routes a press at a
    /// span's own anchor cell back to the first arm rather than the second
    /// (`specs/text-selection/spec.md` -> "A dragged span arms rather than
    /// widening").
    ///
    /// An `Extend` follows the focus to the given cell and sets the
    /// granularity to `Span`, holding the anchor — the standing selection's
    /// when one exists, or the drag's own starting cell otherwise.
    ///
    /// Neither arm writes to the clipboard: `Selection::problem` is always
    /// `None` after either, since only the completing write a later change
    /// adds can set it, and a fresh gesture supersedes whatever the last one
    /// reported.
    fn apply_select(&mut self, phase: SelectPhase) {
        match phase {
            SelectPhase::Begin { line, column } => {
                let cell = (line, column);
                let granularity = match &self.selection {
                    Some(selection) if selection.anchor == cell => match selection.granularity {
                        Granularity::Armed => Granularity::Word,
                        Granularity::Word | Granularity::Row => Granularity::Row,
                        Granularity::Span => Granularity::Armed,
                    },
                    _ => Granularity::Armed,
                };
                self.selection = Some(Selection {
                    anchor: cell,
                    focus: cell,
                    granularity,
                    problem: None,
                });
            }
            SelectPhase::Extend { line, column } => {
                let anchor = self.selection.as_ref().map_or((line, column), |s| s.anchor);
                self.selection = Some(Selection {
                    anchor,
                    focus: (line, column),
                    granularity: Granularity::Span,
                    problem: None,
                });
            }
        }
    }

    /// `help-overlay`'s overlay layer: while `overlay.panel` is `Some`, this table
    /// takes precedence over `apply_route_action` and over every capability
    /// that owns an action's semantics elsewhere — `detail-scroll`,
    /// `artifact-tabs`, `artifact-folds`, `list-selection`, `live-updates`,
    /// and `agent-launch` among them. Seven actions act; the other eighteen
    /// — the closed remainder `specs/help-overlay/spec.md` names — change
    /// nothing at all. Written as an exhaustive match with no wildcard arm,
    /// on the same terms `ui::help`'s own action sweep uses, so an `Action`
    /// added later is a compile error here rather than a silently-inert
    /// nineteenth. `Quit` is the one exception that still acts: a modal that
    /// traps the reader is a worse failure than one that lets a quit through
    /// (design.md -> Decision 5).
    fn apply_help_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.quit = true,
            Action::ToggleHelp | Action::Back => {
                self.overlay.panel = None;
                self.overlay.scroll = 0;
            }
            // `settings-window`: the eighth answered action — swap rather than close or
            // stack, on exactly the requirement's terms ("`?` and `,` swap panels rather
            // than stacking them"). `overlay.scroll` resets because the two panels have
            // unrelated row counts and a carried position would land the reader at an
            // arbitrary row.
            Action::ToggleSettings => {
                self.overlay.panel = Some(Panel::Settings);
                self.overlay.scroll = 0;
            }
            Action::Next | Action::ScrollDown => {
                self.overlay.scroll = self.overlay.scroll.saturating_add(1);
            }
            Action::Prev | Action::ScrollUp => {
                self.overlay.scroll = self.overlay.scroll.saturating_sub(1);
            }
            Action::OpenDetail
            | Action::SelectTab(_)
            | Action::NextTab
            | Action::PrevTab
            | Action::FilterStart
            | Action::FilterPush(_)
            | Action::FilterPop
            | Action::Refresh
            | Action::LaunchApply
            | Action::LaunchContinue
            | Action::LaunchArchive
            | Action::FocusAgent
            | Action::ToggleSection
            | Action::SelectNext
            | Action::SelectPrev
            | Action::Click(_)
            // `text-selection`: inert while the overlay is open — the mouse
            // resolver never produces it there anyway, since a drag over the
            // band already resolves to `Action::Ignore`
            // (`specs/help-overlay/spec.md` -> "`Action::Select` is inert
            // while the overlay is open").
            | Action::Select(_)
            | Action::Ignore => {}
        }
    }

    /// `settings-window`'s addition: the `agent_kind` row's current committed value, read out
    /// of `dashboard.settings.rows` rather than re-derived. `run_loop` (`src/ui/driver.rs`)
    /// calls this immediately before and after every `apply`, so a change between the two
    /// calls means exactly a commit. `apply_settings_open_detail`'s commit branch is **not**
    /// the only place in the crate that changes a row's `value` after `load` constructs it —
    /// `Dashboard::adopt_launch_outcome` also replaces `agent_kind` wholesale, on every
    /// adopted `Outcome` whose `resolution` is `Some` — but `run_loop` calls `drive_live_tier`,
    /// `adopt_launch_outcome`'s one caller, at the top of each iteration, strictly before the
    /// "before" call this doc comment describes, so any such replacement is already folded
    /// into `before` by the time the "after" call could see it as a change. `None` only when
    /// the row itself is absent, which `settings::settings`
    /// never produces (`agent_kind` is always one of its three rows).
    pub(crate) fn agent_kind_value(&self) -> Option<String> {
        self.settings
            .rows
            .iter()
            .find(|row| row.key == "agent_kind")
            .map(|row| row.value.clone())
    }

    /// `settings-window`'s addition: the settings panel's own dispatch layer, reached only
    /// while `overlay.panel` is `Some(Panel::Settings)` — `apply` dispatches here instead of
    /// `apply_help_action` on exactly that branch's terms (design.md -> Decision 2). Nine
    /// actions act; the other seventeen — the closed remainder
    /// `specs/settings-window/spec.md` names — change nothing at all.
    ///
    /// `OpenDetail` begins an edit on the cursor's setting when none is in progress, and
    /// commits the one in progress otherwise — see `apply_settings_open_detail`. `Back`
    /// cancels an edit in progress and leaves the panel open; only when no edit is in
    /// progress does it close the panel, on exactly the requirement's own "innermost layer"
    /// rule. `Next`/`ScrollDown` and `Prev`/`ScrollUp` step the edit's candidate while one is
    /// in progress and move `settings.cursor` through `ui::layout::viewport` otherwise —
    /// `settings.cursor`, not `overlay.scroll`, which stays unread while this panel is open —
    /// see `apply_settings_step` and `move_settings_cursor`.
    fn apply_settings_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.quit = true,
            Action::ToggleSettings => {
                self.overlay.panel = None;
                self.overlay.scroll = 0;
                self.overlay.edit = None;
            }
            // `settings-window`: an edit in progress is the innermost layer — cancel it and
            // leave the panel open; only with no edit in progress does `Back` close the panel,
            // on exactly `ToggleSettings`'s terms for the panel itself.
            Action::Back => {
                if self.overlay.edit.take().is_none() {
                    self.overlay.panel = None;
                    self.overlay.scroll = 0;
                }
            }
            // `settings-window`: swap rather than close or stack, symmetric with
            // `apply_help_action`'s own `ToggleSettings` arm.
            Action::ToggleHelp => {
                self.overlay.panel = Some(Panel::Help);
                self.overlay.scroll = 0;
                self.overlay.edit = None;
            }
            Action::OpenDetail => self.apply_settings_open_detail(),
            Action::Next | Action::ScrollDown => self.apply_settings_step(1),
            Action::Prev | Action::ScrollUp => self.apply_settings_step(-1),
            // `mouse-input` :: "A click on a setting row selects it and begins no edit" —
            // `settings-window`'s own dispatch table: "move the row cursor to `i` and nothing
            // else". `ui::driver::mouse_action` only ever produces an in-range `i`, but the
            // bounds check is kept anyway, on exactly `apply_click`'s `Target::Change` terms:
            // a stale target changes nothing rather than panicking.
            Action::Click(Target::Setting(i)) if i < self.settings.rows.len() => {
                self.settings.cursor = i;
            }
            Action::SelectTab(_)
            | Action::NextTab
            | Action::PrevTab
            | Action::FilterStart
            | Action::FilterPush(_)
            | Action::FilterPop
            | Action::Refresh
            | Action::LaunchApply
            | Action::LaunchContinue
            | Action::LaunchArchive
            | Action::FocusAgent
            | Action::ToggleSection
            | Action::SelectNext
            | Action::SelectPrev
            | Action::Click(_)
            | Action::Select(_)
            | Action::Ignore => {}
        }
    }

    /// `settings-window`'s addition: `OpenDetail` inside the settings panel — begin an edit
    /// on the cursor's setting when none is in progress, or commit the one in progress
    /// otherwise.
    ///
    /// Beginning consults `settings.rows[settings.cursor].editable` directly rather than
    /// re-deriving any precedence rule: `Editable::No` (whatever the `Reason`) begins nothing,
    /// and `Editable::Kind { shortlist }` begins on the committed value's own position in the
    /// shortlist when it is there, and on the first entry otherwise (`settings-window` ::
    /// "A committed kind outside the shortlist starts the edit at the first entry"). An empty
    /// shortlist begins nothing either, though `setting-provenance` never hands one as `Kind`.
    ///
    /// Committing writes the candidate's own shortlist entry back to `Setting::value` — the
    /// in-memory half `settings-window`'s later groups build on: writing `settings.toml`
    /// (group 6) and replacing the launcher's session cache (group 7) are reached from
    /// `run_loop`, never from here, on exactly `NOIO-VIEW`'s terms — and ends the edit. The
    /// row cursor itself is untouched by either half.
    fn apply_settings_open_detail(&mut self) {
        match self.overlay.edit {
            Some(edit) => {
                if let Some(shortlist) = self.settings_shortlist(edit.setting)
                    && let Some(candidate) = shortlist.get(edit.candidate).cloned()
                    && let Some(row) = self.settings.rows.get_mut(edit.setting)
                {
                    row.value = candidate;
                }
                self.overlay.edit = None;
            }
            None => {
                let cursor = self.settings.cursor;
                if let Some(shortlist) = self.settings_shortlist(cursor)
                    && !shortlist.is_empty()
                {
                    let committed = &self.settings.rows[cursor].value;
                    let candidate = shortlist.iter().position(|k| k == committed).unwrap_or(0);
                    self.overlay.edit = Some(Edit {
                        setting: cursor,
                        candidate,
                    });
                }
            }
        }
    }

    /// The shortlist of `settings.rows[index]`, when that row is in bounds and
    /// `Editable::Kind` — the one place `apply_settings_open_detail` and
    /// `apply_settings_step` reach into a row's own shortlist, so the two
    /// cannot disagree about what counts as editable.
    fn settings_shortlist(&self, index: usize) -> Option<&[String]> {
        match self
            .settings
            .rows
            .get(index)
            .map(|setting| &setting.editable)
        {
            Some(crate::settings::Editable::Kind { shortlist }) => Some(shortlist),
            _ => None,
        }
    }

    /// `settings-window`'s addition: `Next`/`ScrollDown` (`step` `1`) and `Prev`/`ScrollUp`
    /// (`step` `-1`) inside the settings panel — step the edit's candidate through its own
    /// shortlist, wrapping at both ends, while an edit is in progress, and move the row
    /// cursor via `move_settings_cursor` otherwise (`settings-window` :: "The shortlist is
    /// the installed kinds, in Herdr's order, and wraps").
    fn apply_settings_step(&mut self, step: isize) {
        let Some(edit) = self.overlay.edit else {
            self.move_settings_cursor(step);
            return;
        };
        let Some(shortlist) = self.settings_shortlist(edit.setting) else {
            return;
        };
        let len = shortlist.len();
        if len == 0 {
            return;
        }
        let candidate = if step.is_negative() {
            (edit.candidate + len - 1) % len
        } else {
            (edit.candidate + 1) % len
        };
        self.overlay.edit = Some(Edit {
            setting: edit.setting,
            candidate,
        });
    }

    /// Move `settings.cursor` by `step` (`1` or `-1`) over `settings.rows`, saturating at
    /// both ends rather than wrapping or underflowing — `select_by`'s own saturating rule,
    /// on `list-selection`'s terms. A row count of zero leaves the cursor at `0`.
    fn move_settings_cursor(&mut self, step: isize) {
        let len = self.settings.rows.len();
        if len == 0 {
            self.settings.cursor = 0;
            return;
        }
        let last = len - 1;
        self.settings.cursor = if step.is_negative() {
            self.settings.cursor.saturating_sub(step.unsigned_abs())
        } else {
            self.settings.cursor.saturating_add(step as usize).min(last)
        };
    }

    /// Move `selected` by `step` (`1` or `-1`), clamp it, and reset
    /// `detail.tab` and `detail.scroll` to zero exactly when it changed value.
    /// The one implementation `Next`/`Prev` at `Route::List` and
    /// `SelectNext`/`SelectPrev` at either route share, so the clamp-and-reset
    /// rule exists once (design.md -> Decision 4).
    fn select_by(&mut self, step: i8) {
        let before = self.selected;
        self.selected = if step > 0 {
            self.selected.saturating_add(1)
        } else {
            self.selected.saturating_sub(1)
        };
        self.clamp_selection();
        if self.selected != before {
            self.detail.tab = 0;
            self.detail.scroll = 0;
        }
    }

    /// Move `detail.scroll` by one line, saturating at zero. The upper bound
    /// stays where `detail-scroll` already puts it — the draw-time clamp in
    /// `render_detail` and the per-frame `normalise_scroll` — not here, so a
    /// wheel held down cannot run the stored offset arbitrarily far ahead any
    /// more than a held `j` can. The one implementation `Next`/`Prev` at
    /// `Route::Detail` and `ScrollDown`/`ScrollUp` at either route share —
    /// and therefore the one site that clears a standing selection on a
    /// scroll (`specs/text-selection/spec.md` -> "The highlight persists
    /// after release and clears on the next interaction"): a scroll moves
    /// what is drawn under the span without changing `sync_detail`'s `(dir,
    /// tab)` key, so nothing else would clear it.
    fn scroll_by(&mut self, step: i8) {
        self.selection = None;
        self.detail.scroll = if step > 0 {
            self.detail.scroll.saturating_add(1)
        } else {
            self.detail.scroll.saturating_sub(1)
        };
    }

    /// `mouse-input`: act on the row a click landed on.
    ///
    /// The `targets()` membership guard applies only in the `Target::Section`
    /// and `Target::Change` arms — the two **list** targets `targets()`
    /// yields — and does nothing at all when the target is absent there: a
    /// change filtered away, or a section folded, between the draw and the
    /// event. A `Target::Section` moves the cursor to that header and toggles
    /// it through `apply_toggle_section`, the very code `Action::ToggleSection`
    /// runs, so a click and a `Space` on the same header can never diverge. A
    /// `Target::Change` moves the cursor to that row when it is not already
    /// there, and otherwise — at `Route::List` — opens the detail, which is
    /// what makes a second click on a selected row an `Enter`.
    ///
    /// `Target::DetailHeader` is exempt from that guard (design.md ->
    /// Decision 7): it carries indices `mouse_action` already resolved
    /// against the frame just drawn, not an index into a list that may have
    /// changed since — re-validating against `targets()`, which does not
    /// even contain it, would be both impossible and pointless. A
    /// top-of-function guard would return before this arm is reached,
    /// silently discarding every detail click; it remains gated only by
    /// `Detail::foldable()`, on the same terms `mouse_action` does not emit
    /// it for a non-foldable artifact and `apply` checks anyway rather than
    /// trusting it.
    fn apply_click(&mut self, target: Target) {
        // `text-selection`: every click clears a standing selection, whichever
        // of the three `Target` forms it addresses and whether or not the
        // membership guard below finds it — a click that lands on nothing is
        // still a click, and a stale target does not make the click not have
        // happened. See `specs/text-selection/spec.md` -> "The highlight
        // persists after release and clears on the next interaction".
        self.selection = None;
        match target {
            Target::Section(_) => {
                let Some(index) = self.targets().iter().position(|t| *t == target) else {
                    return;
                };
                self.selected = index;
                self.apply_toggle_section();
            }
            Target::Change(_) => {
                let Some(index) = self.targets().iter().position(|t| *t == target) else {
                    return;
                };
                if self.selected == index {
                    if self.route == Route::List {
                        self.route = Route::Detail;
                        self.detail.scroll = 0;
                    }
                } else {
                    self.selected = index;
                    self.detail.tab = 0;
                    self.detail.scroll = 0;
                }
            }
            Target::DetailHeader { line, section } => {
                if self.detail.foldable() {
                    self.detail.scroll = line;
                    // The carried `section`, not a re-derivation of it:
                    // `mouse_action` resolved it against the frame just drawn
                    // and `specs/mouse-input/spec.md` forbids `apply`
                    // recomputing it.
                    self.toggle_detail_section(section);
                }
            }
            // Structurally unreachable, on `DetailHeader`'s own terms above:
            // `mouse_action` produces `Target::Setting` only while
            // `overlay.panel` is `Some(Panel::Settings)`, and `apply` routes
            // that case to `apply_settings_action` instead of here. The arm
            // exists only because this match is over `Target` as a whole.
            Target::Setting(_) => {}
        }
    }

    /// The section `Action::ToggleSection` acts on: the one `selected` already
    /// addresses when it is a header, and otherwise the tier the addressed
    /// change is drawn from — active for a change from `changes.active`,
    /// archived for one from `changes.archived`. `None` when the target list
    /// is empty, which is what makes an empty list inert.
    fn target_section(&self) -> Option<SectionKey> {
        match self.targets().get(self.selected)? {
            // `targets()` never emits `DetailHeader` — it addresses the
            // detail region, not the list — so this arm is structurally
            // unreachable; it exists only because the match is over `Target`
            // as a whole (design.md -> Decision 7's own note on
            // `list-selection/spec.md`).
            Target::DetailHeader { .. } => None,
            // `targets()` never emits `Setting` either, on the same terms:
            // it addresses the settings panel's band, not the list.
            Target::Setting(_) => None,
            Target::Section(key) => Some(*key),
            Target::Change(i) => {
                let active_visible = if self.section_open(SectionKey::Active) {
                    self.changes
                        .active
                        .iter()
                        .filter(|c| matches(&c.name, &self.filter.query))
                        .count()
                } else {
                    0
                };
                if *i < active_visible {
                    Some(SectionKey::Active)
                } else {
                    Some(SectionKey::Archived)
                }
            }
        }
    }

    /// Fold `target_section()`'s section if it is open, unfold it otherwise,
    /// and move `selected` to that section's header — collapsing a section
    /// the cursor was inside would otherwise leave it addressing a change
    /// that is no longer shown. A no-op when the target list is empty.
    fn apply_toggle_section(&mut self) {
        let Some(key) = self.target_section() else {
            return;
        };
        if !self.sections.collapsed.remove(&key) {
            self.sections.collapsed.insert(key);
        }
        if let Some(idx) = self
            .targets()
            .iter()
            .position(|t| *t == Target::Section(key))
        {
            self.selected = idx;
        }
    }

    /// `artifact-folds`: `Space`'s detail-route counterpart to
    /// `apply_toggle_section` — folds or unfolds exactly one **artifact**
    /// section, the one `detail.scroll` is on or in, and moves
    /// `detail.scroll` to that section's own header row, recomputed against
    /// the line list the fold just produced. Touches only `detail.expanded`
    /// and `detail.scroll`; leaves `sections`, `selected`, and
    /// `refresh.requested` untouched. Inert, with no problem recorded, when
    /// the selected artifact is not foldable (`Detail::foldable()` — the
    /// one site, per design.md -> Decision 3) or when `detail.scroll`
    /// addresses a problem row rather than a section.
    fn apply_toggle_detail_section(&mut self) {
        if !self.detail.foldable() {
            return;
        }
        let Some(section) = self.detail_cursor_section() else {
            return;
        };
        self.toggle_detail_section(section);
    }

    /// Fold `section` if it is open, unfold it if it is collapsed, and move the
    /// detail cursor to its header row — `artifact-folds`' cursor-to-header
    /// rule.
    ///
    /// Takes the section **already resolved**, which is what lets a click and a
    /// `Space` on the same header run the very same code without the click
    /// re-deriving an index it was already handed:
    /// `specs/mouse-input/spec.md` requires that `Target::DetailHeader`'s two
    /// indices arrive resolved against the frame just drawn and that
    /// `Dashboard::apply` SHALL NOT recompute either, and design.md ->
    /// Decision 7 states that carrying `section` beside `line` is exactly why
    /// the variant has two fields.
    ///
    /// `text-selection`: a fold changes which rows `ui::detail::content_lines`
    /// produces for this **same** `(dir, tab)` — a line index a standing
    /// selection carries can point at different text the instant the fold
    /// takes effect, with no reload to catch it: `sync_detail`'s cache key
    /// does not see `detail.expanded` at all. `apply_click`'s
    /// `Target::DetailHeader` arm already reaches this function through a
    /// call site that clears first; clearing here too is what makes the
    /// keyboard path (`Action::ToggleSection` at `Route::Detail`) agree
    /// with it rather than relying on that caller to remember to.
    fn toggle_detail_section(&mut self, section: usize) {
        self.selection = None;
        if !self.detail.expanded.remove(&section) {
            self.detail.expanded.insert(section);
        }
        // A **second** derivation of the row list, deliberately: this one runs
        // after the mutation above, and `artifact-folds` requires the cursor to
        // land on the toggled section's header row "recomputed against the line
        // list the fold just produced". Merging the two calls would reintroduce
        // the bug that clause exists to prevent (Change Review, group 10, which
        // raised the duplication and then withdrew it for this reason).
        if let Some(header_row) = self.detail_section_header_row(section) {
            self.detail.scroll = header_row;
        }
    }

    /// The section `detail.scroll` is currently on or in, per
    /// `ui::detail::content_lines`' own `SectionHeader { selected }` flag —
    /// a **lookup** into that row list, never a second derivation of it
    /// (design.md -> Decision 12's own reasoning, carried to the keyboard
    /// side of the fold). `None` when `detail.scroll` addresses a problem
    /// row or the artifact carries no sections at all.
    fn detail_cursor_section(&self) -> Option<usize> {
        // `None` before a first frame: the fold is inert rather than resolving
        // against a guessed width (design.md -> Decision 13).
        let width = self.detail.drawn_width?;
        crate::ui::detail::content_lines(&self.detail, self.selected_change(), width)
            .iter()
            .find_map(|row| match row.kind {
                crate::ui::detail::ContentKind::SectionHeader {
                    section,
                    selected: true,
                } => Some(section),
                _ => None,
            })
    }

    /// `section`'s own header row index in the **current** row list — used
    /// to move `detail.scroll` onto the header a fold just toggled,
    /// recomputed against the line list the fold produced rather than
    /// reused from before it.
    fn detail_section_header_row(&self, section: usize) -> Option<usize> {
        let width = self.detail.drawn_width?;
        crate::ui::detail::content_lines(&self.detail, self.selected_change(), width)
            .iter()
            .position(|row| {
                matches!(
                    row.kind,
                    crate::ui::detail::ContentKind::SectionHeader { section: s, .. } if s == section
                )
            })
    }

    /// Whether the next refresh cycle should resolve the archived tier: the
    /// archived section is open, `changes.archived` is empty, and
    /// `changes.archived_total` is greater than zero. Self-clearing, because
    /// a `Full` cycle always yields `archived.len() == archived_total`. See
    /// design.md -> Decision 6.
    pub fn needs_archived_refresh(&self) -> bool {
        self.section_open(SectionKey::Archived)
            && self.changes.archived.is_empty()
            && self.changes.archived_total > 0
    }

    /// The one place all four launch actions gather the same six values — the selected
    /// change's name, the focus pane `attribution().panes` holds for it, whether the socket is
    /// reachable, the live agents' names, whether a launch is already in flight, and whether
    /// the pane is in file mode — and hand them to `launch::decide`.
    /// `Decision::Nothing` changes nothing; `Decision::Refuse` replaces `launch.problems` with
    /// the one reason and leaves `launch.pending` alone; `Decision::Go` sets `launch.pending`
    /// and clears `launch.problems`.
    fn apply_launch_action(&mut self, action: Action) {
        let intent = match action {
            Action::LaunchApply => crate::launch::Intent::Apply,
            Action::LaunchContinue => crate::launch::Intent::Continue,
            Action::LaunchArchive => crate::launch::Intent::Archive,
            Action::FocusAgent => crate::launch::Intent::Focus,
            _ => unreachable!("apply_launch_action is called only for the four launch actions"),
        };
        let change = self.selected_change().map(|c| c.name.clone());
        let attribution = self.attribution();
        let pane = change
            .as_deref()
            .and_then(|name| attribution.panes.get(name))
            .cloned();
        let live_names: Vec<&str> = self
            .agents
            .agents
            .iter()
            .filter_map(|a| a.name.as_deref())
            .collect();
        match crate::launch::decide(
            intent,
            change.as_deref(),
            pane.as_deref(),
            self.agents.reachable,
            &live_names,
            self.launch.in_flight,
            // `agent-client-choice`: the seventh and last argument, so `in_flight` stays the
            // sixth. `file_mode` is the composition root's own fact — the binary probe
            // resolved nothing — and it is the same one the header badges and the footer
            // reads, so the badge, the dropped hint and this refusal all follow one value.
            self.file_mode,
        ) {
            crate::launch::Decision::Nothing => {}
            crate::launch::Decision::Refuse(reason) => {
                self.launch.problems = vec![reason];
            }
            crate::launch::Decision::Go(request) => {
                self.launch.pending = Some(request);
                self.launch.problems.clear();
            }
        }
    }

    /// Recompute the detail region from `frame_area` — through
    /// `layout::split_frame`, `layout::split_body`, and `layout::interior`
    /// — and clamp the stored `detail.scroll` against the line count the
    /// markdown at that interior's width actually produces. A pure total
    /// function of `&mut self` and `frame_area`: it changes nothing when
    /// the detail region is not drawn (the narrow list route) or its
    /// interior has zero width or zero height. Called once per loop
    /// iteration by `ui::driver::run_loop`, against the frame just drawn,
    /// so a held key cannot leave the offset arbitrarily far past the end.
    pub fn normalise_scroll(&mut self, frame_area: ratatui::layout::Rect) {
        let (body, _) = crate::ui::layout::split_frame(frame_area);
        let (_, divider, detail_area) = crate::ui::layout::split_body(body, self.route);
        let Some(area) = detail_area else {
            return;
        };
        let interior =
            crate::ui::layout::interior(area, crate::ui::layout::detail_gutters(divider));
        if interior.width == 0 || interior.height == 0 {
            return;
        }
        let (_, _, content) = crate::ui::layout::split_detail(interior);
        if content.width == 0 || content.height == 0 {
            return;
        }
        // The one place the drawn content width is recorded, on the one call
        // already made once per frame with it in hand (design.md -> Decision 13).
        self.detail.drawn_width = Some(content.width);
        let total =
            crate::ui::detail::content_lines(&self.detail, self.selected_change(), content.width)
                .len();
        // `detail-scroll` -> "The stored scroll offset is normalised against the
        // frame just drawn": a foldable artifact clamps the cursor to the last
        // **line**, so every section header stays reachable however tall the
        // pane is (design.md -> Decision 2); a non-foldable one keeps today's
        // screenful clamp. Both leave `detail.scroll` at `0` when `total` is
        // `0`, since `total.saturating_sub(1)` is itself `0` there.
        self.detail.scroll = if self.detail.foldable() {
            self.detail.scroll.min(total.saturating_sub(1))
        } else {
            crate::ui::layout::scroll_offset(total, self.detail.scroll, content.height)
        };
    }

    /// The overlay's own per-frame clamp — a **second** normaliser beside
    /// [`Self::normalise_scroll`], called once per frame by
    /// `ui::driver::run_loop` right after it. Changes nothing while
    /// `overlay.panel` is `None`; while it is `Some`, clamps `overlay.scroll`
    /// against the band's own interior height, derived from `frame_area`
    /// through `layout::split_frame`'s body and `layout::overlay_band` —
    /// never through the detail region `normalise_scroll` reads.
    ///
    /// It is not folded into `normalise_scroll`, and the reason is a
    /// measured contradiction rather than tidiness: `normalise_scroll`
    /// returns early when the detail region is not drawn, which
    /// `detail-scroll` requires of it in as many words, and the detail
    /// region is not drawn at `Route::List` below the breakpoint — exactly
    /// the 60x20 `Route::List` fixture this change's own held-key scenario
    /// uses. Sharing the function would leave `overlay.scroll` unclamped in
    /// the one case that scenario exists to pin (design.md -> Decision 10).
    ///
    /// `ui::layout::scroll_offset`'s parameter order is `(lines, scroll,
    /// height)` — `content_rows` first, on the same terms every other call
    /// site in this change writes it.
    pub fn normalise_help_scroll(&mut self, frame_area: ratatui::layout::Rect) {
        if self.overlay.panel.is_none() {
            return;
        }
        let (body, _) = crate::ui::layout::split_frame(frame_area);
        let band = crate::ui::layout::overlay_band(body, crate::ui::help::content_rows());
        let interior_height = band.height.saturating_sub(2);
        self.overlay.scroll = crate::ui::layout::scroll_offset(
            crate::ui::help::content_rows(),
            self.overlay.scroll,
            interior_height,
        );
    }

    /// Whether `key`'s rows are shown: a non-empty `/` query forces every section
    /// open for as long as it is non-empty, regardless of the reader's own fold —
    /// a filter that silently hid a match behind a fold would be worse than the
    /// long list it replaces. Otherwise a section is open exactly when its key is
    /// **not** in `sections.collapsed`. Nothing is ever written to `sections`
    /// because of a query, so "restoring the reader's own collapse state when the
    /// query clears" needs no save and no restore. See design.md -> Decision 5.
    pub fn section_open(&self, key: SectionKey) -> bool {
        !self.filter.query.is_empty() || !self.sections.collapsed.contains(&key)
    }

    /// The number of matched entries `key`'s header reports: the matched count
    /// when the tier is actually resolved, and `changes.archived_total` when the
    /// archived tier is open but not yet resolved (`changes.archived` empty with
    /// `archived_total` greater than zero) — with no query the two coincide. See
    /// design.md -> Decision 10.
    fn section_count(&self, key: SectionKey) -> usize {
        match key {
            SectionKey::Active => self
                .changes
                .active
                .iter()
                .filter(|c| matches(&c.name, &self.filter.query))
                .count(),
            SectionKey::Archived => {
                if self.changes.archived.is_empty() && self.changes.archived_total > 0 {
                    self.changes.archived_total
                } else {
                    self.changes
                        .archived
                        .iter()
                        .filter(|c| matches(&c.name, &self.filter.query))
                        .count()
                }
            }
        }
    }

    /// The addressable targets, in exactly the order `change-rows` emits their
    /// rows: the active section header when its count is greater than zero, then
    /// the visible active changes when that section is open, then the archived
    /// header on the same condition, then the visible archived changes when it is
    /// open. Pure and total over `&self`, stored nowhere. See
    /// `specs/list-selection/spec.md` and design.md -> Decision 2.
    pub fn targets(&self) -> Vec<Target> {
        let mut targets = Vec::new();
        if self.section_count(SectionKey::Active) > 0 {
            targets.push(Target::Section(SectionKey::Active));
        }
        let mut index = 0usize;
        if self.section_open(SectionKey::Active) {
            for _ in self
                .changes
                .active
                .iter()
                .filter(|c| matches(&c.name, &self.filter.query))
            {
                targets.push(Target::Change(index));
                index += 1;
            }
        }
        if self.section_count(SectionKey::Archived) > 0 {
            targets.push(Target::Section(SectionKey::Archived));
        }
        if self.section_open(SectionKey::Archived) {
            for _ in self
                .changes
                .archived
                .iter()
                .filter(|c| matches(&c.name, &self.filter.query))
            {
                targets.push(Target::Change(index));
                index += 1;
            }
        }
        targets
    }

    /// The changes actually **shown**: the query-matching entries of
    /// `changes.active` when the active section is open, followed by the
    /// query-matching entries of `changes.archived` when the archived section is
    /// open. A closed section's changes are absent, so folding a section that is
    /// already resolved removes its entries from this list on the same frame
    /// rather than waiting for a refresh. What `change-rows`' `rows` also emits,
    /// and what `Target::Change`'s index addresses.
    pub fn visible(&self) -> Vec<&Change> {
        let mut visible = Vec::new();
        if self.section_open(SectionKey::Active) {
            visible.extend(
                self.changes
                    .active
                    .iter()
                    .filter(|c| matches(&c.name, &self.filter.query)),
            );
        }
        if self.section_open(SectionKey::Archived) {
            visible.extend(
                self.changes
                    .archived
                    .iter()
                    .filter(|c| matches(&c.name, &self.filter.query)),
            );
        }
        visible
    }

    /// `visible().len()`, without building the intermediate `Vec` twice at
    /// call sites that only need the count.
    pub fn visible_len(&self) -> usize {
        self.visible().len()
    }

    /// The scope a refresh should resolve the archived tier under: `Full` when
    /// the archived section is open, `Names` when it is collapsed. Cheap either
    /// way — this reads only `sections` and `filter.query`, never `changes`. See
    /// design.md -> Decision 1 and -> Decision 13.
    pub fn archived_scope(&self) -> crate::changes::ArchivedScope {
        if self.section_open(SectionKey::Archived) {
            crate::changes::ArchivedScope::Full
        } else {
            crate::changes::ArchivedScope::Names
        }
    }

    /// Derive the current attribution from state this dashboard already
    /// carries — `repo`, `changes`, `agents.agents`, and `agent_names` —
    /// beside `visible()`, recomputed on every call and stored nowhere. The
    /// change-name slice is built from `changes.active` and `changes.archived`
    /// in full, never the filtered `visible()`, so the `/` filter hides rows
    /// without changing the count. See `specs/agent-attribution/spec.md`.
    pub fn attribution(&self) -> crate::agents::Attribution {
        let change_names: Vec<&str> = self
            .changes
            .active
            .iter()
            .chain(self.changes.archived.iter())
            .map(|c| c.name.as_str())
            .collect();
        crate::agents::attribute(
            &self.agents.agents,
            self.repo.as_deref(),
            &change_names,
            &self.agent_names.names,
        )
    }

    /// The change `selected` addresses, or `None` when it addresses a section
    /// header, the target list is empty, or `selected` is somehow out of range.
    /// What `sync_detail`, `SelectTab`, `NextTab`, and `agent-launch`'s four
    /// action keys all resolve their target change through, so a header cursor
    /// makes every one of them inert with no rule of its own.
    pub fn selected_change(&self) -> Option<&Change> {
        match self.targets().get(self.selected)? {
            Target::Change(i) => self.visible().get(*i).copied(),
            Target::Section(_) => None,
            // Structurally unreachable — see `target_section`'s own note.
            Target::DetailHeader { .. } => None,
            // Structurally unreachable, on the same terms.
            Target::Setting(_) => None,
        }
    }

    /// Replace `changes` with a freshly produced set, preserving the
    /// selection by the previously selected change's **name** rather than
    /// its index: a refresh can add a change alphabetically above the
    /// selected one, and an index preserved across that shift would
    /// silently move the reader to a different change mid-read. The name is
    /// resolved against the **new** `visible()` list, not `changes.active`,
    /// because `selected` indexes the visible list and a `/` filter may be
    /// active — and then resolved a second time through `targets()`, because
    /// `selected` indexes *targets*, not `visible()`, and a `visible()`
    /// position found here is one or two short of the `targets()` index that
    /// actually addresses it whenever a section header precedes it (design.md
    /// -> Decision 14). Pure: no I/O, no clock. Sets `refresh.reload`, which is
    /// what makes `sync_detail` re-read the (possibly unchanged) selection on
    /// the very next call. Does **not** reset `detail.tab` or `detail.scroll`,
    /// and does **not** touch `sections`: a refresh is neither a selection move
    /// nor a fold. See `specs/live-updates/spec.md` -> "Adopting a change set
    /// preserves the selection by name" and `specs/list-selection/spec.md`.
    pub fn adopt(&mut self, changes: ChangeSet) {
        let previous_name = self.selected_change().map(|c| c.name.clone());
        self.changes = changes;
        if let Some(name) = previous_name
            && let Some(pos) = self.visible().iter().position(|c| c.name == name)
            && let Some(target_index) = self
                .targets()
                .iter()
                .position(|t| *t == Target::Change(pos))
        {
            self.selected = target_index;
        }
        self.clamp_selection();
        self.refresh.reload = true;
    }

    /// `agent-launch`'s addition, generalised by `settings-window`: fold a `Launcher::drain`
    /// answer into the dashboard, on `adopt`'s own "one pure method, called from the loop"
    /// terms — `named` into `agent_names.names`, `problems` replacing `launch.problems`
    /// wholesale, and `in_flight` cleared, all in the one step `ui::driver::drive_live_tier`
    /// used to perform inline.
    ///
    /// `settings-window`'s addition: an outcome whose `picker` is set opens the settings
    /// panel on the `agent_kind` row — `overlay.panel` becomes `Some(Panel::Settings)`,
    /// `overlay.scroll` resets to `0`, and any edit in progress is cancelled, on exactly
    /// `ToggleSettings`'s own swap terms, whichever panel (if any) was open before. Every
    /// other outcome — a success, a failed call, a plain refusal, and `Request::Resolve`'s
    /// own answer, whose `picker` is always unset — leaves `overlay` untouched entirely: not
    /// only `panel`, but `scroll` and `edit` too. See `specs/agent-launch/spec.md` -> "An
    /// ambiguous agent kind opens the settings panel instead of picking one".
    ///
    /// `settings-window`'s further addition: whenever `resolution` is `Some` — a `,`-triggered
    /// `Request::Resolve`'s answer no less than an ambiguous launch refusal's — the
    /// `agent_kind` row is replaced in place with `settings::agent_kind_setting`'s own
    /// rendering of it, the second of the three moments `settings::PanelState::rows`'s doc
    /// comment names beside startup and a commit. `openspec_bin` and `prompts` are untouched:
    /// neither depends on `kind`. Done before the `picker` branch so a picker outcome finds the
    /// freshly-resolved row already in place when it repositions `settings.cursor` onto it.
    pub fn adopt_launch_outcome(&mut self, outcome: crate::launch::Outcome) {
        self.launch.in_flight = false;
        if let Some((agent, change)) = outcome.named {
            self.agent_names.names.insert(agent, change);
        }
        self.launch.problems = outcome.problems;
        if let Some(resolution) = &outcome.resolution
            && let Some(row) = self
                .settings
                .rows
                .iter_mut()
                .find(|row| row.key == "agent_kind")
        {
            *row = crate::settings::agent_kind_setting(Some(resolution));
        }
        if outcome.picker {
            self.overlay.panel = Some(Panel::Settings);
            self.overlay.scroll = 0;
            self.overlay.edit = None;
            if let Some(index) = self
                .settings
                .rows
                .iter()
                .position(|row| row.key == "agent_kind")
            {
                self.settings.cursor = index;
            }
        }
    }

    /// Resolve the selected tab's content through the injected reader,
    /// re-reading when the `(change directory, tab)` key has changed **or**
    /// `refresh.reload` was set. Total: never panics for any dashboard
    /// state, any artifact list, any `detail.tab`, or any reader behaviour,
    /// including one that fails on every path. Called by
    /// `ui::driver::run_loop` once per iteration, before the draw — never by
    /// a view.
    ///
    /// `refresh.reload` is taken (and cleared) at the very start of every
    /// call, on every branch, including the two early returns below — one
    /// flag drives at most one re-read. `detail.scroll` is reset exactly
    /// when the **key** changed, never merely because a reload was forced:
    /// that split is what lets an agent's save re-render the document a
    /// reader is halfway down without throwing them back to line one, while
    /// a tab or change move still starts at the top. See
    /// `specs/live-updates/spec.md` -> "A forced reload re-reads without
    /// losing the scroll".
    ///
    /// The borrow checker forbids interleaving these steps with the borrow
    /// `selected_change()` holds over `*self`, so everything the read needs
    /// is copied out in one expression and the borrow ends there; `tab` is
    /// clamped inside that expression and assigned after it. See
    /// `openspec/changes/detail-view/design.md` -> Contracts.
    pub fn sync_detail(&mut self, read: ArtifactReader<'_>) {
        let forced = std::mem::take(&mut self.refresh.reload);
        let Some((dir, tab, paths, tracks_tasks)) = self.selected_change().map(|change| {
            let count = change.artifacts.len();
            let tab = self.detail.tab.min(count.saturating_sub(1));
            let artifact = change.artifacts.get(tab);
            let paths = artifact.map(|a| a.paths.clone()).unwrap_or_default();
            let tracks_tasks = artifact.is_some_and(|a| a.tracks_tasks);
            (change.dir.clone(), tab, paths, tracks_tasks)
        }) else {
            // Step 1: nothing selected.
            self.detail.sections.clear();
            self.detail.problems.clear();
            self.detail.tab = 0;
            self.detail.scroll = 0;
            self.detail.loaded = None;
            self.detail.expanded.clear();
            // `text-selection`: no change is selected, so there is nothing left to
            // have selected text from — design.md -> Decision 4.
            self.selection = None;
            return;
        };
        self.detail.tab = tab; // step 2
        let key = (dir, tab);
        let key_changed = self.detail.loaded.as_ref() != Some(&key);
        if !key_changed && !forced {
            return; // step 3
        }
        // `text-selection`: the detail content is about to reload, deliberately —
        // never a side effect of `Detail` being overwritten below. A selection
        // pointing at the old content would be stale, so it is cleared here rather
        // than carried forward. See design.md -> Decision 4 and the scenario
        // "`selection` starts empty and is cleared rather than reloaded".
        self.selection = None;
        // Step 4: re-read every path, one or more `ArtifactSection` values
        // per successful read, in resolution order — nothing concatenated,
        // no separator inserted and no newline added, per `artifact-folds`.
        // A failing path contributes no section, only a problem.
        self.detail.problems.clear();
        self.detail.sections.clear();
        // A file section is contributed only when the artifact resolved to
        // more than one path, and it is what pushes that file's own headings
        // down one level.
        let base = usize::from(paths.len() > 1);
        for path in &paths {
            match read(path) {
                Ok(text) => {
                    // Exactly one `split_headings` call per successfully read
                    // path, which is why the spec half of the gate is asked of
                    // the sections rather than of the text.
                    let headings = split_headings(&text);
                    let preamble_end = preamble_len(&text, &headings);
                    // `title-heading-preamble`: recognised from the heading
                    // list alone, before the split decision reads it — never
                    // the other way, which would be circular
                    // (`specs/artifact-folds/spec.md` -> "The order is").
                    let title_index = title_heading(&headings, tracks_tasks);
                    // A title heading's own body contributes a section only
                    // when it is non-empty **after trimming whitespace** —
                    // deliberately not the preamble's own byte-emptiness
                    // predicate (`title-heading-preamble`'s design.md -> D8).
                    let title_contributes =
                        title_index.is_some_and(|i| !headings[i].body.trim().is_empty());
                    let remaining_headings = headings.len() - usize::from(title_index.is_some());
                    // What this file would contribute if it split: its
                    // preamble, when non-empty, plus the demoted title's own
                    // body, when it contributes, plus one section per
                    // remaining heading. A file that would contribute
                    // exactly one section is not split at all, unless a file
                    // section already precedes it. Splitting it would
                    // consume its one heading into a `label` that
                    // `content_lines`' non-foldable branch never draws,
                    // losing the heading row off the screen — so refusing
                    // the split keeps `artifact-folds`' byte-identity
                    // sentence true by construction rather than by a second
                    // exemption inside `content_lines`, which is the
                    // argument `heading-sections`' design.md -> D3 already
                    // makes for the gate's `total > 0` half. With a file
                    // section ahead of it the heading does draw as a header
                    // row, so the fallback is not wanted there. Counting
                    // contributions **after** demotion, rather than counting
                    // headings, is what keeps a titled single-group file
                    // unsplit (`title-heading-preamble`'s design.md -> D4).
                    let has_preamble = !text.get(..preamble_end).unwrap_or_default().is_empty();
                    let contributions = usize::from(has_preamble)
                        + usize::from(title_contributes)
                        + remaining_headings;
                    let splits = ((tracks_tasks && crate::tasks::count(&text).total > 0)
                        || has_requirement_heading(&headings))
                        && (base > 0 || contributions > 1);
                    let label = Some(artifact_section_label(&key.0, path));
                    if !splits {
                        // Today's behaviour, unchanged: one section carrying
                        // the reader's bytes, which is every prose artifact.
                        self.detail.sections.push(ArtifactSection {
                            label,
                            text,
                            depth: 0,
                            progress: None,
                            operation: None,
                        });
                        continue;
                    }
                    if base > 0 {
                        self.detail.sections.push(ArtifactSection {
                            label,
                            text: String::new(),
                            depth: 0,
                            progress: None,
                            operation: None,
                        });
                    }
                    let preamble = text.get(..preamble_end).unwrap_or_default();
                    if !preamble.is_empty() {
                        self.detail.sections.push(ArtifactSection {
                            label: None,
                            text: preamble.to_string(),
                            depth: base,
                            progress: None,
                            operation: None,
                        });
                    }
                    // Normalised against this file's own shallowest heading,
                    // so a delta spec starting at `##` and an archived spec
                    // starting at `#` both open flush at the left. The
                    // demoted title is excluded — it owns no header row to
                    // level against — which is the half that un-indents
                    // every group (`title-heading-preamble`'s design.md ->
                    // D5); `unwrap_or(0)` is reached when demotion leaves no
                    // labelled heading at all.
                    let min_level = headings
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| Some(*i) != title_index)
                        .map(|(_, h)| h.level)
                        .min()
                        .unwrap_or(0);
                    // `spec-emphasis`: the operation walk, a single forward
                    // pass over this file's own headings, in the order
                    // `heading-sections` produced them, holding the most
                    // recent recognised operation and attributing it to the
                    // requirement sections that follow — reset per file, and
                    // computed beside `progress` above for the same reason
                    // `tasks-emphasis` computed that field here: both are
                    // derived once per key change over the section list
                    // already in hand.
                    let mut current_operation: Option<crate::specs::DeltaOp> = None;
                    for (index, heading) in headings.into_iter().enumerate() {
                        // A level-2 heading that classifies **resets** the
                        // walk rather than nesting: the sections after a
                        // later operation heading carry that heading's
                        // operation even where an earlier one appeared in the
                        // same file. The operation heading itself is
                        // deliberately unbadged — attribution below asks
                        // `is_requirement_heading`, the same predicate
                        // `has_requirement_heading` applies, never restated —
                        // and a requirement under no operation heading yet
                        // carries `None`. This runs **inside** the loop, at
                        // every heading including the demoted title, so the
                        // walk still advances over every heading in
                        // document order regardless of demotion
                        // (`title-heading-preamble`'s design.md -> D6): a
                        // single branch, not a title skipped ahead of it.
                        if let Some(op) =
                            crate::specs::operation_of_heading(heading.level, &heading.label)
                        {
                            current_operation = Some(op);
                        }
                        if Some(index) == title_index {
                            // The demoted title contributes no header row,
                            // and only where its own body is non-empty
                            // after trimming (`title-heading-preamble`'s
                            // design.md -> D8).
                            if !heading.body.trim().is_empty() {
                                self.detail.sections.push(ArtifactSection {
                                    label: None,
                                    text: heading.body,
                                    depth: base,
                                    progress: None,
                                    operation: None,
                                });
                            }
                            continue;
                        }
                        // `tasks-emphasis`: a **heading section of a split
                        // tracked-tasks file** carries its own group's count,
                        // which is what lets a reader fold a completed group
                        // without losing how far along it was. Every other
                        // section — the file section, the preamble, every
                        // section of an unsplit file, and every section of
                        // every other artifact — carries `None`. The value is
                        // `task-groups`' own count of that group's items, so
                        // it agrees with the whole file's by that capability's
                        // summation property.
                        let progress =
                            tracks_tasks.then(|| crate::tasks::parse(&heading.body).progress());
                        let operation = is_requirement_heading(&heading)
                            .then_some(current_operation)
                            .flatten();
                        self.detail.sections.push(ArtifactSection {
                            label: Some(heading.label),
                            text: heading.body,
                            depth: base + usize::from(heading.level.saturating_sub(min_level)),
                            progress,
                            operation,
                        });
                    }
                }
                Err(e) => {
                    self.detail
                        .problems
                        .push(format!("{}: {e}", path.display()));
                }
            }
        }
        // Step 5: the scroll and the fold state reset on a key change only,
        // never on a forced-but-unchanged-key reload — `artifact-folds`'
        // "A tab move forgets the fold, a forced reload does not" — and, on
        // that same condition and only there, the tracked-tasks seed.
        if key_changed {
            self.detail.scroll = 0;
            self.detail.expanded.clear();
            if tracks_tasks {
                self.detail.expanded = seed_expanded(&self.detail.sections);
            }
        }
        self.detail.loaded = Some(key);
    }

    /// Keep `selected` addressing a target that actually exists: `0` when the
    /// target list is empty, otherwise clamped to its last index. Never *raises*
    /// `selected` — clamping shrinks the index and never restores it once the
    /// list grows back. Clamps against `targets().len()`, not `visible_len()`:
    /// `selected` indexes targets, and a change-count clamp would leave it
    /// addressing the wrong row by one or two targets whenever a section header
    /// precedes the last valid position. See design.md -> Decision 14.
    fn clamp_selection(&mut self) {
        let len = self.targets().len();
        if len == 0 {
            self.selected = 0;
        } else if self.selected >= len {
            self.selected = len - 1;
        }
    }
}

/// A change is visible when its name, Unicode-lowercased, contains `query`,
/// Unicode-lowercased. An empty query matches every change. `str::to_lowercase`
/// applies the Unicode **default** case-conversion mapping — not locale-tailored,
/// not full case folding, and not normalising either side — so `to_ascii_lowercase`'s
/// blind spot above U+007F is closed without introducing a different one. See
/// `specs/list-filtering/spec.md` -> "The query is a case-insensitive substring
/// match on the change name" and design.md -> Decision 6.
pub fn matches(name: &str, query: &str) -> bool {
    name.to_lowercase().contains(query.to_lowercase().as_str())
}

/// Map a terminal event and the current filter mode to one of the twenty-six
/// actions. Total: every `Event` value maps to something, under either
/// mode, and nothing panics. Acts only on key events whose `kind` is
/// `KeyEventKind::Press` — a `Repeat` or `Release` maps to `Ignore` under
/// either mode, so a terminal that reports release events does not quit
/// twice, navigate, or type a character on the release of a key already
/// handled on its press.
pub fn action_for(event: &Event, filtering: bool) -> Action {
    let Event::Key(key) = event else {
        return Action::Ignore;
    };
    if key.kind != KeyEventKind::Press {
        return Action::Ignore;
    }
    if filtering {
        return match (key.code, key.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Action::Quit,
            (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                Action::FilterPush(c)
            }
            (KeyCode::Backspace, KeyModifiers::NONE) => Action::FilterPop,
            (KeyCode::Enter, KeyModifiers::NONE) => Action::OpenDetail,
            (KeyCode::Esc, KeyModifiers::NONE) => Action::Back,
            (KeyCode::Up, KeyModifiers::NONE) => Action::Prev,
            (KeyCode::Down, KeyModifiers::NONE) => Action::Next,
            _ => Action::Ignore,
        };
    }
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), KeyModifiers::NONE) => Action::Quit,
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Action::Quit,
        (KeyCode::Char('j'), KeyModifiers::NONE) | (KeyCode::Down, KeyModifiers::NONE) => {
            Action::Next
        }
        (KeyCode::Char('k'), KeyModifiers::NONE) | (KeyCode::Up, KeyModifiers::NONE) => {
            Action::Prev
        }
        (KeyCode::Char('/'), KeyModifiers::NONE) => Action::FilterStart,
        (KeyCode::Char('r'), KeyModifiers::NONE) => Action::Refresh,
        // `agent-launch`: `a`/`c`/`s` launch, `g` focuses. `Char('c')` with `CONTROL` is
        // matched above and still quits; bare `c` is this arm.
        (KeyCode::Char('a'), KeyModifiers::NONE) => Action::LaunchApply,
        (KeyCode::Char('c'), KeyModifiers::NONE) => Action::LaunchContinue,
        (KeyCode::Char('s'), KeyModifiers::NONE) => Action::LaunchArchive,
        (KeyCode::Char('g'), KeyModifiers::NONE) => Action::FocusAgent,
        (KeyCode::Char(c @ '1'..='9'), KeyModifiers::NONE) => {
            Action::SelectTab((c as u8 - b'1') as usize)
        }
        (KeyCode::Char(']'), KeyModifiers::NONE) => Action::NextTab,
        (KeyCode::Char('['), KeyModifiers::NONE) => Action::PrevTab,
        // `list-sections`: the one new row in the `filtering` false table. While
        // filtering, `' '` already falls through to the generic `Char(c)` arm above,
        // which types it — no row is added to that table.
        (KeyCode::Char(' '), KeyModifiers::NONE) => Action::ToggleSection,
        // `help-overlay`: the one row with two accepted modifier values — terminals
        // disagree about whether `Shift` is reported alongside a shifted character like
        // `?`, so matching only `NONE` would make the key work on some terminals and not
        // others. While filtering, `'?'` already falls through to the generic `Char(c)`
        // arm above, which types it under either modifier — no row is added there.
        (KeyCode::Char('?'), KeyModifiers::NONE) | (KeyCode::Char('?'), KeyModifiers::SHIFT) => {
            Action::ToggleHelp
        }
        // `settings-window`: `,` carries no shifted-character ambiguity (unlike `?`), so this
        // row matches only `NONE`. While filtering, `','` already falls through to the generic
        // `Char(c)` arm above, which types it — no row is added there.
        (KeyCode::Char(','), KeyModifiers::NONE) => Action::ToggleSettings,
        (KeyCode::Enter, KeyModifiers::NONE) => Action::OpenDetail,
        (KeyCode::Esc, KeyModifiers::NONE) => Action::Back,
        _ => Action::Ignore,
    }
}

#[cfg(test)]
mod tests {
    // `agent-attribution`'s own tests live directly under `mod tests`, not nested in
    // `mod keys` below, so `testcount`'s `ui::app::tests::attribution_` and
    // `ui::app::tests::dashboard_names_agent_names_at_every_site` filters — which name no
    // submodule — actually match their full test paths.
    use crate::agents::{Agent, AgentStatus};
    use crate::changes::fixture;
    use crate::ui::app::{
        Action, ArtifactSection, Dashboard, Detail, Filter, Panel, Refresh, Route, Sections,
        action_for, artifact_section_label, is_spec_shaped, split_headings,
    };
    use std::collections::BTreeMap;

    fn agent(name: Option<&str>, status: AgentStatus, cwd: Option<&str>) -> Agent {
        Agent {
            name: name.map(str::to_string),
            kind: None,
            status,
            cwd: cwd.map(std::path::PathBuf::from),
            pane_id: "p".to_string(),
            tab_id: "t".to_string(),
            workspace_id: "w".to_string(),
            terminal_title: None,
        }
    }

    /// A `Dashboard` rooted at `/repo`, over `active`/`archived` changes, carrying
    /// `agents` and `agent_names` — this module's own fixture, since `mod keys`'s
    /// `dashboard_at` and `five_change_dashboard` are private to it.
    fn dashboard_for_attribution(
        active: Vec<crate::changes::Change>,
        archived: Vec<crate::changes::Change>,
        selected: usize,
        agents: Vec<Agent>,
        agent_names: BTreeMap<String, String>,
    ) -> Dashboard {
        Dashboard {
            settings: crate::settings::PanelState {
                rows: Vec::new(),
                cursor: 0,
            },
            selection: None,
            repo: Some(std::path::PathBuf::from("/repo")),
            searched_from: std::path::PathBuf::from("/repo"),
            changes: fixture::set(active, archived, Vec::new()),
            route: Route::List,
            quit: false,
            selected,
            filter: Filter {
                query: String::new(),
                active: false,
            },
            detail: Detail {
                sections: Vec::new(),
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
                expanded: std::collections::BTreeSet::new(),
                drawn_width: None,
            },
            refresh: Refresh {
                requested: false,
                reload: false,
                startup: Vec::new(),
                problems: Vec::new(),
            },
            agents: crate::agents::AgentSnapshot {
                agents,
                reachable: true,
                stalled: false,
                problem: None,
            },
            agent_names: crate::state::Mapping {
                names: agent_names,
                problems: Vec::new(),
            },
            launch: crate::ui::app::Launch {
                pending: None,
                in_flight: false,
                problems: Vec::new(),
            },
            sections: Sections {
                collapsed: std::collections::BTreeSet::new(),
            },
            file_mode: false,
            overlay: crate::ui::app::Overlay {
                panel: None,
                scroll: 0,
                edit: None,
            },
        }
    }

    /// `dashboard-loop`'s compile-time companion, on `agent-attribution`'s own terms:
    /// an exhaustive destructuring naming all eleven fields with no `..` rest, so a
    /// field added later fails to compile here rather than defaulting silently. Beside
    /// (not a replacement for) `mod keys`'s landed
    /// `dashboard_destructures_into_exactly_ten_fields`, which stays where it is.
    #[test]
    fn dashboard_names_agent_names_at_every_site() {
        let d = dashboard_for_attribution(Vec::new(), Vec::new(), 0, Vec::new(), BTreeMap::new());
        let Dashboard {
            repo,
            searched_from,
            changes,
            route,
            quit,
            selected,
            filter,
            detail,
            refresh,
            agents,
            agent_names,
            launch,
            sections: _,
            file_mode: _,
            overlay: _,
            selection: _,
            settings: _,
        } = &d;
        assert_eq!(*repo, Some(std::path::PathBuf::from("/repo")));
        assert_eq!(searched_from, &std::path::PathBuf::from("/repo"));
        assert_eq!(changes.active.len(), 0);
        assert_eq!(*route, Route::List);
        assert!(!*quit);
        assert_eq!(*selected, 0);
        assert_eq!(
            filter,
            &Filter {
                query: String::new(),
                active: false
            }
        );
        assert_eq!(
            detail,
            &Detail {
                sections: Vec::new(),
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
                expanded: std::collections::BTreeSet::new(),
                drawn_width: None,
            }
        );
        assert!(!refresh.requested);
        assert!(!refresh.reload);
        assert!(refresh.startup.is_empty());
        assert!(refresh.problems.is_empty());
        assert!(agents.agents.is_empty());
        assert!(agents.reachable);
        assert!(agents.problem.is_none());
        assert!(agent_names.names.is_empty());
        assert_eq!(launch.pending, None);
        assert!(launch.problems.is_empty());
        assert!(!launch.in_flight);
    }

    /// Drives all three tiers through `Dashboard::attribution()` rather than through
    /// `agents::attribute` directly: the mapping tier (`c-alpha` -> `alpha`), the name
    /// tier (an agent named exactly the archived change `legacy`), and the count (an
    /// agent matching neither).
    #[test]
    fn attribution_derives_the_three_tiers() {
        let names = BTreeMap::from([("c-alpha".to_string(), "alpha".to_string())]);
        let via_mapping = agent(Some("c-alpha"), AgentStatus::Working, Some("/repo"));
        let via_name = agent(Some("legacy"), AgentStatus::Idle, Some("/repo"));
        let unmatched = agent(Some("scratch"), AgentStatus::Blocked, Some("/repo"));
        let d = dashboard_for_attribution(
            vec![
                fixture::active("alpha", 1, 2),
                fixture::active("beta", 0, 1),
            ],
            vec![fixture::archived(Some("2026-08-14"), "legacy", 3, 3)],
            0,
            vec![via_mapping, via_name, unmatched],
            names,
        );

        let attribution = d.attribution();
        assert_eq!(
            attribution.badges,
            BTreeMap::from([
                ("alpha".to_string(), AgentStatus::Working),
                ("legacy".to_string(), AgentStatus::Idle),
            ])
        );
        assert_eq!(attribution.unattributed, 1);
    }

    /// `agent-attribution`'s "A refresh that reorders the list moves the badge with its
    /// change": `adopt` preserves the selection by name, and the badge — keyed by name,
    /// never by index — must follow it.
    #[test]
    fn attribution_follows_adopt_by_name() {
        let mut d = dashboard_for_attribution(
            vec![
                fixture::active("beta", 0, 1),
                fixture::active("gamma", 0, 1),
            ],
            Vec::new(),
            // `list-sections`: 2, not 1 — the active header is target 0, so
            // `gamma` (`visible()` position 1) sits at target 2.
            2,
            vec![agent(Some("gamma"), AgentStatus::Working, Some("/repo"))],
            BTreeMap::new(),
        );
        assert_eq!(d.selected_change().unwrap().name, "gamma");

        d.adopt(fixture::set(
            vec![
                fixture::active("alpha", 0, 1),
                fixture::active("beta", 0, 1),
                fixture::active("gamma", 0, 1),
            ],
            Vec::new(),
            Vec::new(),
        ));

        assert_eq!(
            d.attribution().badges,
            BTreeMap::from([("gamma".to_string(), AgentStatus::Working)])
        );
        assert_eq!(
            d.selected_change().unwrap().name,
            "gamma",
            "the selection and the badge must agree because both are resolved by name"
        );
    }

    /// `agent-attribution`'s "The `/` filter hides rows without changing the count":
    /// `attribution()` is built from the full change list, never the filtered
    /// `visible()`, so setting a query changes nothing about it.
    #[test]
    fn attribution_ignores_the_filter() {
        let mut d = dashboard_for_attribution(
            vec![
                fixture::active("alpha", 0, 1),
                fixture::active("beta", 0, 1),
            ],
            Vec::new(),
            0,
            vec![
                agent(Some("alpha"), AgentStatus::Working, Some("/repo")),
                agent(Some("scratch-one"), AgentStatus::Idle, Some("/repo")),
                agent(Some("scratch-two"), AgentStatus::Idle, Some("/repo")),
            ],
            BTreeMap::new(),
        );
        let before = d.attribution();
        assert_eq!(
            before.badges,
            BTreeMap::from([("alpha".to_string(), AgentStatus::Working)])
        );
        assert_eq!(before.unattributed, 2);

        d.filter.query = "beta".to_string();
        let after = d.attribution();
        assert_eq!(before, after, "the filter must not change the attribution");
    }

    /// `dashboard-loop`: "The four action keys map, and their near misses do not."
    #[test]
    fn the_four_action_keys_map_and_their_near_misses_do_not() {
        use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

        fn press(code: KeyCode, modifiers: KeyModifiers) -> Event {
            Event::Key(KeyEvent::new(code, modifiers))
        }

        assert_eq!(
            action_for(&press(KeyCode::Char('a'), KeyModifiers::NONE), false),
            Action::LaunchApply
        );
        assert_eq!(
            action_for(&press(KeyCode::Char('c'), KeyModifiers::NONE), false),
            Action::LaunchContinue
        );
        assert_eq!(
            action_for(&press(KeyCode::Char('s'), KeyModifiers::NONE), false),
            Action::LaunchArchive
        );
        assert_eq!(
            action_for(&press(KeyCode::Char('g'), KeyModifiers::NONE), false),
            Action::FocusAgent
        );
        assert_eq!(
            action_for(&press(KeyCode::Char('c'), KeyModifiers::CONTROL), false),
            Action::Quit
        );
        assert_eq!(
            action_for(&press(KeyCode::Char('A'), KeyModifiers::SHIFT), false),
            Action::Ignore
        );
        assert_eq!(
            action_for(&press(KeyCode::Char('G'), KeyModifiers::SHIFT), false),
            Action::Ignore
        );
        assert_eq!(
            action_for(&press(KeyCode::Char('a'), KeyModifiers::CONTROL), false),
            Action::Ignore
        );
        assert_eq!(
            action_for(&press(KeyCode::Char('s'), KeyModifiers::ALT), false),
            Action::Ignore
        );

        assert_eq!(
            action_for(&press(KeyCode::Char('a'), KeyModifiers::NONE), true),
            Action::FilterPush('a')
        );
        assert_eq!(
            action_for(&press(KeyCode::Char('c'), KeyModifiers::NONE), true),
            Action::FilterPush('c')
        );
        assert_eq!(
            action_for(&press(KeyCode::Char('s'), KeyModifiers::NONE), true),
            Action::FilterPush('s')
        );
        assert_eq!(
            action_for(&press(KeyCode::Char('g'), KeyModifiers::NONE), true),
            Action::FilterPush('g')
        );
        assert_eq!(
            action_for(&press(KeyCode::Char('c'), KeyModifiers::CONTROL), true),
            Action::Quit
        );
        assert_eq!(
            action_for(&press(KeyCode::Char('A'), KeyModifiers::SHIFT), true),
            Action::FilterPush('A')
        );
        assert_eq!(
            action_for(&press(KeyCode::Char('G'), KeyModifiers::SHIFT), true),
            Action::FilterPush('G')
        );
        assert_eq!(
            action_for(&press(KeyCode::Char('a'), KeyModifiers::CONTROL), true),
            Action::Ignore
        );
        assert_eq!(
            action_for(&press(KeyCode::Char('s'), KeyModifiers::ALT), true),
            Action::Ignore
        );

        for code in [
            KeyCode::Char('a'),
            KeyCode::Char('c'),
            KeyCode::Char('s'),
            KeyCode::Char('g'),
        ] {
            let released = Event::Key(KeyEvent::new_with_kind(
                code,
                KeyModifiers::NONE,
                KeyEventKind::Release,
            ));
            let repeated = Event::Key(KeyEvent::new_with_kind(
                code,
                KeyModifiers::NONE,
                KeyEventKind::Repeat,
            ));
            for filtering in [false, true] {
                assert_eq!(action_for(&released, filtering), Action::Ignore, "{code:?}");
                assert_eq!(action_for(&repeated, filtering), Action::Ignore, "{code:?}");
            }
        }
    }

    /// `dashboard-loop`: "A launch action reaches no collaborator and starts no work."
    #[test]
    fn a_launch_action_reaches_no_collaborator_and_starts_no_work() {
        let mut d = dashboard_for_attribution(
            vec![fixture::active("add-auth", 1, 2)],
            Vec::new(),
            // `list-sections`: 1, not 0 — target 0 is the active header.
            1,
            Vec::new(),
            BTreeMap::new(),
        );
        let before = d.clone();
        d.apply(Action::LaunchApply);

        assert_eq!(
            d.launch.pending,
            Some(crate::launch::Request::Launch {
                change: "add-auth".to_string(),
                agent: "add-auth".to_string(),
                intent: crate::launch::Intent::Apply,
            })
        );
        assert!(d.launch.problems.is_empty());
        assert_eq!(d.changes, before.changes);
        assert_eq!(d.selected, before.selected);
        assert_eq!(d.route, before.route);
        assert_eq!(d.detail, before.detail);
        assert_eq!(d.filter, before.filter);
        assert_eq!(d.quit, before.quit);
        assert_eq!(d.refresh, before.refresh);
        assert_eq!(d.agents, before.agents);
        assert_eq!(d.agent_names, before.agent_names);
    }

    /// `dashboard-loop`: "A refused launch records the reason and produces no request."
    #[test]
    fn a_refused_launch_records_the_reason_and_produces_no_request() {
        let mut d = dashboard_for_attribution(
            vec![fixture::active("2fa-support", 1, 2)],
            Vec::new(),
            // `list-sections`: 1, not 0 — target 0 is the active header.
            1,
            vec![agent(
                Some("c-2fa-support"),
                AgentStatus::Working,
                Some("/repo"),
            )],
            BTreeMap::new(),
        );
        for _ in 0..3 {
            d.apply(Action::LaunchApply);
        }
        assert_eq!(d.launch.pending, None);
        assert_eq!(
            d.launch.problems.len(),
            1,
            "replaced wholesale, never grown"
        );
        assert!(d.launch.problems[0].contains("c-2fa-support"));
        assert!(d.launch.problems[0].contains('g'));
    }

    /// `dashboard-loop`: "An unreachable socket makes the four keys change nothing."
    #[test]
    fn an_unreachable_socket_makes_the_four_keys_change_nothing() {
        let mut d = dashboard_for_attribution(
            vec![fixture::active("add-auth", 1, 2)],
            Vec::new(),
            0,
            Vec::new(),
            BTreeMap::new(),
        );
        d.agents.reachable = false;
        let before = d.clone();
        for action in [
            Action::LaunchApply,
            Action::LaunchContinue,
            Action::LaunchArchive,
            Action::FocusAgent,
        ] {
            d.apply(action);
        }
        assert_eq!(d, before);
    }

    /// `dashboard-loop`: "Dashboard has no Default and no site elides a field" — a third
    /// exhaustive destructuring naming `launch` explicitly, beside
    /// `dashboard_names_agent_names_at_every_site` and `mod keys`'s own companion.
    #[test]
    fn dashboard_names_launch_at_every_site() {
        let d = dashboard_for_attribution(Vec::new(), Vec::new(), 0, Vec::new(), BTreeMap::new());
        let Dashboard {
            repo: _,
            searched_from: _,
            changes: _,
            route: _,
            quit: _,
            selected: _,
            filter: _,
            detail: _,
            refresh: _,
            agents: _,
            agent_names: _,
            launch,
            sections: _,
            file_mode: _,
            overlay: _,
            selection: _,
            settings: _,
        } = &d;
        assert_eq!(launch.pending, None);
        assert!(launch.problems.is_empty());
        assert!(!launch.in_flight);
    }

    /// `agent-launch`: "`g` focuses an agent attributed through the mapping" and "`g` focuses
    /// an agent attributed by name" — both tiers `Attribution::panes` follows.
    #[test]
    fn focus_resolves_the_pane_for_each_attribution_tier() {
        let names = BTreeMap::from([("c-2fa-support".to_string(), "2fa-support".to_string())]);
        let mut mapped = dashboard_for_attribution(
            vec![fixture::active("2fa-support", 1, 2)],
            Vec::new(),
            // `list-sections`: 1, not 0 — target 0 is the active header.
            1,
            vec![agent(
                Some("c-2fa-support"),
                AgentStatus::Working,
                Some("/repo"),
            )],
            names,
        );
        mapped.apply(Action::FocusAgent);
        assert_eq!(
            mapped.launch.pending,
            Some(crate::launch::Request::Focus {
                pane_id: "p".to_string()
            })
        );

        let mut named = dashboard_for_attribution(
            vec![fixture::active("add-auth", 1, 2)],
            Vec::new(),
            // `list-sections`: 1, not 0 — target 0 is the active header.
            1,
            vec![agent(Some("add-auth"), AgentStatus::Idle, Some("/repo"))],
            BTreeMap::new(),
        );
        named.apply(Action::FocusAgent);
        assert_eq!(
            named.launch.pending,
            Some(crate::launch::Request::Focus {
                pane_id: "p".to_string()
            })
        );
    }

    /// `agent-launch`: "`g` on a change no tier could attribute does nothing" — including the
    /// empty-list half.
    #[test]
    fn g_on_an_unattributed_change_does_nothing() {
        let mut d = dashboard_for_attribution(
            vec![fixture::active("add-auth", 1, 2)],
            Vec::new(),
            0,
            vec![agent(
                Some("scratch-work"),
                AgentStatus::Working,
                Some("/repo"),
            )],
            BTreeMap::new(),
        );
        let before = d.clone();
        d.apply(Action::FocusAgent);
        assert_eq!(d, before);

        let mut empty =
            dashboard_for_attribution(Vec::new(), Vec::new(), 0, Vec::new(), BTreeMap::new());
        let before_empty = empty.clone();
        empty.apply(Action::FocusAgent);
        assert_eq!(empty, before_empty);
    }

    /// `agent-launch`: "An archived change launches on the same terms as an active one" —
    /// restated in terms `Dashboard::apply` can observe: `launch.pending` is field-for-field
    /// identical to what the same press produces on an active change of that name.
    #[test]
    fn an_archived_change_launches_on_the_same_terms() {
        let mut d = dashboard_for_attribution(
            Vec::new(),
            vec![fixture::archived(Some("2026-08-14"), "add-auth", 1, 2)],
            // `list-sections`: 1, not 0 — with no active changes, target 0 is
            // the archived header.
            1,
            Vec::new(),
            BTreeMap::new(),
        );
        d.apply(Action::LaunchApply);
        assert_eq!(
            d.launch.pending,
            Some(crate::launch::Request::Launch {
                change: "add-auth".to_string(),
                agent: "add-auth".to_string(),
                intent: crate::launch::Intent::Apply,
            })
        );
    }

    /// `agent-launch`: each launch action maps to its own `Intent`, one to one.
    #[test]
    fn launch_intent_maps_one_to_one_from_the_action() {
        for (action, intent) in [
            (Action::LaunchApply, crate::launch::Intent::Apply),
            (Action::LaunchContinue, crate::launch::Intent::Continue),
            (Action::LaunchArchive, crate::launch::Intent::Archive),
        ] {
            let mut d = dashboard_for_attribution(
                vec![fixture::active("add-auth", 1, 2)],
                Vec::new(),
                // `list-sections`: 1, not 0 — target 0 is the active header.
                1,
                Vec::new(),
                BTreeMap::new(),
            );
            d.apply(action);
            match d.launch.pending {
                Some(crate::launch::Request::Launch {
                    change: _,
                    agent: _,
                    intent: got,
                }) => {
                    assert_eq!(got, intent, "{action:?}")
                }
                other => panic!("expected Go(Launch), got {other:?}"),
            }
        }
    }

    /// The three-row fixture `adopt_launch_outcome`'s own tests need, with `agent_kind` at
    /// index `1` — `settings::settings`'s own fixed order — so
    /// `adopt_launch_outcome`'s cursor-repositioning has a row to find.
    fn three_settings_for_launch_tests() -> Vec<crate::settings::Setting> {
        vec![
            crate::settings::Setting {
                key: "openspec_bin",
                value: "openspec_bin-value".to_string(),
                provenance: crate::settings::Provenance::Default,
                editable: crate::settings::Editable::No {
                    reason: crate::settings::Reason::SetOnce,
                },
            },
            crate::settings::Setting {
                key: "agent_kind",
                value: "the agent kind is still resolving".to_string(),
                provenance: crate::settings::Provenance::Pending,
                editable: crate::settings::Editable::No {
                    reason: crate::settings::Reason::Resolving,
                },
            },
            crate::settings::Setting {
                key: "prompts",
                value: "prompts-value".to_string(),
                provenance: crate::settings::Provenance::Default,
                editable: crate::settings::Editable::No {
                    reason: crate::settings::Reason::SetOnce,
                },
            },
        ]
    }

    /// `agent-launch` :: "The ambiguous outcome opens the settings panel on `agent_kind`".
    #[test]
    fn the_ambiguous_outcome_opens_the_settings_panel_on_agent_kind() {
        let mut d = dashboard_for_attribution(
            vec![fixture::active("add-auth", 1, 2)],
            Vec::new(),
            1,
            Vec::new(),
            BTreeMap::new(),
        );
        d.settings.rows = three_settings_for_launch_tests();
        d.settings.cursor = 0;
        d.launch.in_flight = true;
        d.overlay.panel = None;
        let before_route = d.route;
        let before_selected = d.selected;
        let before_detail = d.detail.clone();
        let before_sections = d.sections.clone();

        let outcome = crate::launch::Outcome {
            named: None,
            problems: vec![
                "set agent_kind (claude, codex): more than one agent integration is installed"
                    .to_string(),
            ],
            picker: true,
            resolution: Some(crate::settings::KindResolution {
                choice: crate::integration::Choice::Ambiguous {
                    installed: vec!["claude".to_string(), "codex".to_string()],
                },
                installed: vec!["claude".to_string(), "codex".to_string()],
            }),
        };
        d.adopt_launch_outcome(outcome);

        assert_eq!(d.overlay.panel, Some(Panel::Settings));
        assert_eq!(d.settings.cursor, 1, "the agent_kind row's own index");
        assert_eq!(d.overlay.scroll, 0);
        assert_eq!(d.overlay.edit, None);
        assert_eq!(
            d.launch.problems,
            vec![
                "set agent_kind (claude, codex): more than one agent integration is installed"
                    .to_string()
            ]
        );
        assert!(
            !d.launch.in_flight,
            "cleared on every outcome, this one included"
        );
        assert_eq!(d.route, before_route);
        assert_eq!(d.selected, before_selected);
        assert_eq!(d.detail, before_detail);
        assert_eq!(d.sections, before_sections);
    }

    /// `agent-launch` :: "Opening the panel resolves the kind once and launches nothing" — the
    /// `Dashboard` half of that scenario, beside
    /// `launch::tests::opening_the_panel_resolves_the_kind_once_and_launches_nothing`, which
    /// proves only the `Outcome` shape. `settings::PanelState::rows`'s own doc comment states
    /// three moments `settings::settings` runs at: startup, every adopted `resolution`, and
    /// every commit — this is the middle one. A `Request::Resolve` answer carries `picker`
    /// unset (only `Choice::Ambiguous` reached through a launch refusal sets it) and
    /// `resolution` `Some`; adopting it must replace the `agent_kind` row in place with what
    /// `settings::agent_kind_setting` derives from that resolution — from `Pending`/
    /// `Editable::No { reason: Resolving }` to `Provenance::Ambiguous`/`Editable::Kind` here —
    /// so the very next `Enter` can begin an edit rather than finding the row still
    /// resolving. `openspec_bin` and `prompts` are untouched: only `agent_kind`'s own entry
    /// depends on `kind`.
    #[test]
    fn a_resolved_kind_updates_the_agent_kind_row_without_opening_the_panel() {
        let mut d = dashboard_for_attribution(
            vec![fixture::active("add-auth", 1, 2)],
            Vec::new(),
            1,
            Vec::new(),
            BTreeMap::new(),
        );
        d.settings.rows = three_settings_for_launch_tests();
        let before_openspec_bin = d.settings.rows[0].clone();
        let before_prompts = d.settings.rows[2].clone();
        d.overlay.panel = None;

        let outcome = crate::launch::Outcome {
            named: None,
            problems: Vec::new(),
            picker: false,
            resolution: Some(crate::settings::KindResolution {
                choice: crate::integration::Choice::Ambiguous {
                    installed: vec!["claude".to_string(), "codex".to_string()],
                },
                installed: vec!["claude".to_string(), "codex".to_string()],
            }),
        };
        d.adopt_launch_outcome(outcome);

        assert_eq!(
            d.overlay.panel, None,
            "picker unset: this Resolve answer must not open the panel"
        );
        let agent_kind = &d.settings.rows[1];
        assert_eq!(agent_kind.key, "agent_kind");
        assert_eq!(
            agent_kind.provenance,
            crate::settings::Provenance::Ambiguous
        );
        match &agent_kind.editable {
            crate::settings::Editable::Kind { shortlist } => {
                assert_eq!(shortlist, &vec!["claude".to_string(), "codex".to_string()]);
            }
            other => panic!("expected Editable::Kind, got {other:?}"),
        }
        assert_eq!(
            d.settings.rows[0], before_openspec_bin,
            "openspec_bin does not depend on kind"
        );
        assert_eq!(
            d.settings.rows[2], before_prompts,
            "prompts does not depend on kind"
        );
    }

    /// `agent-launch` :: "No other outcome touches the overlay" — a successful launch, a
    /// failed call, a dead-worker-style refusal, and a `LastResort` warning, each adopted
    /// against a `Dashboard` whose overlay is closed and again against one whose overlay
    /// shows the help panel.
    #[test]
    fn no_other_outcome_touches_the_overlay() {
        let outcomes = [
            crate::launch::Outcome {
                named: Some(("c-add-auth".to_string(), "add-auth".to_string())),
                problems: Vec::new(),
                picker: false,
                resolution: None,
            },
            crate::launch::Outcome {
                named: None,
                problems: vec!["herdr pane split exited with code 1: no space".to_string()],
                picker: false,
                resolution: None,
            },
            crate::launch::Outcome {
                named: None,
                problems: vec!["the launcher's worker has stopped answering".to_string()],
                picker: false,
                resolution: None,
            },
            crate::launch::Outcome {
                named: Some(("c-add-auth".to_string(), "add-auth".to_string())),
                problems: vec![
                    "no herdr agent integration is installed and no agent_kind is configured - \
                     launching claude as a last resort"
                        .to_string(),
                ],
                picker: false,
                resolution: None,
            },
        ];
        for outcome in outcomes {
            assert!(!outcome.picker, "only Choice::Ambiguous sets picker");
            for panel in [None, Some(Panel::Help)] {
                let mut d = dashboard_for_attribution(
                    vec![fixture::active("add-auth", 1, 2)],
                    Vec::new(),
                    1,
                    Vec::new(),
                    BTreeMap::new(),
                );
                d.settings.rows = three_settings_for_launch_tests();
                d.overlay.panel = panel;
                let before_overlay = d.overlay.clone();
                d.adopt_launch_outcome(outcome.clone());
                assert_eq!(
                    d.overlay, before_overlay,
                    "outcome {outcome:?} must not touch the overlay when it started at {panel:?}"
                );
            }
        }
    }

    /// `artifact-folds`: "The label derivation is total over adversarial
    /// paths" — the scenario's own six paths against one change directory,
    /// none of which panics.
    #[test]
    fn the_label_derivation_is_total_over_adversarial_paths() {
        let change_dir = std::path::Path::new("/repo/openspec/changes/c");
        let cases: [(&str, &str); 6] = [
            ("/repo/openspec/changes/c/specs/a/spec.md", "a"),
            ("/repo/openspec/changes/c/specs/a/b/spec.md", "b"),
            ("/repo/openspec/changes/c/specs/notes.md", "notes.md"),
            ("/repo/openspec/changes/c/spec.md", "spec.md"),
            ("/elsewhere/spec.md", "spec.md"),
            ("", ""),
        ];
        for (path, want) in cases {
            assert_eq!(
                artifact_section_label(change_dir, std::path::Path::new(path)),
                want,
                "path {path:?}"
            );
        }
    }

    /// `artifact-folds`: "The three spec files of a change become three
    /// labelled sections".
    #[test]
    fn the_three_spec_files_of_a_change_become_three_labelled_sections() {
        let change = fixture::with_artifacts(
            fixture::active("c", 0, 0),
            &[(
                "specs",
                &[
                    "/repo/openspec/changes/c/specs/degraded-coverage/spec.md",
                    "/repo/openspec/changes/c/specs/markdown-render/spec.md",
                    "/repo/openspec/changes/c/specs/tasks-checklist/spec.md",
                ],
            )],
        );
        let mut d =
            dashboard_for_attribution(vec![change], Vec::new(), 1, Vec::new(), BTreeMap::new());
        let recorder =
            crate::testutil::RecordingReader::always(Ok("## MODIFIED Requirements\n".to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert_eq!(d.detail.sections.len(), 3);
        assert_eq!(
            d.detail.sections[0].label.as_deref(),
            Some("degraded-coverage")
        );
        assert_eq!(
            d.detail.sections[1].label.as_deref(),
            Some("markdown-render")
        );
        assert_eq!(
            d.detail.sections[2].label.as_deref(),
            Some("tasks-checklist")
        );
        for section in &d.detail.sections {
            assert_eq!(section.text, "## MODIFIED Requirements\n");
        }
        assert!(d.detail.foldable(), "foldable: sections.len() is 3");
    }

    /// `artifact-folds`: "An artifact with no resolved paths has no
    /// sections".
    #[test]
    fn an_artifact_with_no_resolved_paths_has_no_sections() {
        let change = fixture::with_artifacts(fixture::active("c", 0, 0), &[("specs", &[])]);
        let mut d =
            dashboard_for_attribution(vec![change], Vec::new(), 1, Vec::new(), BTreeMap::new());
        let recorder =
            crate::testutil::RecordingReader::always(Err("should not be called".to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert!(d.detail.sections.is_empty());
        assert!(d.detail.sections.len() <= 1, "not foldable");
        assert_eq!(recorder.calls(), 0);
        let lines = crate::ui::detail::content_lines(&d.detail, None, 78);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].text().starts_with("No content yet"));
    }

    /// `artifact-folds`: "An unreadable file drops its section and keeps
    /// its siblings".
    #[test]
    fn an_unreadable_file_drops_its_section_and_keeps_its_siblings() {
        let change = fixture::with_artifacts(
            fixture::active("c", 0, 0),
            &[(
                "specs",
                &[
                    "/repo/openspec/changes/c/specs/a/spec.md",
                    "/repo/openspec/changes/c/specs/b/spec.md",
                    "/repo/openspec/changes/c/specs/d/spec.md",
                ],
            )],
        );
        let mut d =
            dashboard_for_attribution(vec![change], Vec::new(), 1, Vec::new(), BTreeMap::new());
        let recorder = crate::testutil::RecordingReader::new(
            vec![
                (
                    std::path::PathBuf::from("/repo/openspec/changes/c/specs/a/spec.md"),
                    Ok("# ok\n".to_string()),
                ),
                (
                    std::path::PathBuf::from("/repo/openspec/changes/c/specs/b/spec.md"),
                    Err("permission denied".to_string()),
                ),
                (
                    std::path::PathBuf::from("/repo/openspec/changes/c/specs/d/spec.md"),
                    Ok("# ok\n".to_string()),
                ),
            ],
            Err("unexpected".to_string()),
        );
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert_eq!(d.detail.sections.len(), 2);
        assert_eq!(d.detail.sections[0].label.as_deref(), Some("a"));
        assert_eq!(d.detail.sections[1].label.as_deref(), Some("d"));
        assert_eq!(d.detail.problems.len(), 1);
        assert!(d.detail.problems[0].contains("/repo/openspec/changes/c/specs/b/spec.md"));
        assert!(d.detail.problems[0].contains("permission denied"));
        assert!(d.detail.foldable(), "still foldable");
    }

    // `heading-sections`, group 3: the split gate and the section derivation,
    // every one of them driven through `sync_detail` with a closure reader
    // (design.md -> Test Boundaries), never the real filesystem.

    /// A prose artifact, whose `##` and `###` headings are not navigation
    /// targets: `specs/artifact-folds/spec.md` -> "A prose artifact with
    /// headings does not split".
    const PROSE_WITH_HEADINGS: &str = "# Why\n\n## What Changes\n\n### A sub-heading\ntext\n";

    /// One change's dashboard over `artifacts`, with `marked` — when `Some` —
    /// the position whose `tracks_tasks` is `true`, selected at the one change
    /// row these scenarios have.
    fn dashboard_over(artifacts: &[(&str, &[&str])], marked: Option<usize>) -> Dashboard {
        let change = fixture::with_artifacts(fixture::active("c", 0, 0), artifacts);
        let change = match marked {
            Some(index) => fixture::track_tasks_at(change, index),
            None => change,
        };
        dashboard_for_attribution(vec![change], Vec::new(), 1, Vec::new(), BTreeMap::new())
    }

    /// Every section's `(label, depth)` pair, in order.
    fn shape_of(detail: &Detail) -> Vec<(Option<&str>, usize)> {
        detail
            .sections
            .iter()
            .map(|s| (s.label.as_deref(), s.depth))
            .collect()
    }

    /// `artifact-folds`: "A prose artifact with headings does not split" — the
    /// change's answer to the Non-Goal "folding arbitrary markdown headings in
    /// prose artifacts".
    #[test]
    fn a_prose_artifact_with_headings_does_not_split() {
        let mut d = dashboard_over(
            &[("proposal", &["/repo/openspec/changes/c/proposal.md"])],
            None,
        );
        let recorder =
            crate::testutil::RecordingReader::always(Ok(PROSE_WITH_HEADINGS.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert_eq!(shape_of(&d.detail), vec![(Some("proposal.md"), 0)]);
        assert_eq!(d.detail.sections[0].text, PROSE_WITH_HEADINGS);
        assert!(!d.detail.foldable(), "one section is not foldable");
    }

    /// `artifact-folds`: "A spec file splits and a task file splits" — the two
    /// halves of the gate, one dashboard each.
    #[test]
    fn a_spec_file_splits_and_a_task_file_splits() {
        let mut spec = dashboard_over(
            &[("specs", &["/repo/openspec/changes/c/specs/a/spec.md"])],
            None,
        );
        let spec_recorder = crate::testutil::RecordingReader::always(Ok(DELTA_SPEC.to_string()));
        let read_spec = |p: &std::path::Path| spec_recorder.read(p);

        spec.sync_detail(&read_spec);

        assert_eq!(
            shape_of(&spec.detail),
            vec![
                (Some("ADDED Requirements"), 0),
                (Some("Requirement: Alpha"), 1),
                (Some("Scenario: A works"), 2),
                (Some("Requirement: Beta"), 1),
            ],
            "four sections, normalised against the file's own `##`"
        );
        assert!(spec.detail.foldable());

        let mut tasks = dashboard_over(
            &[("tasks", &["/repo/openspec/changes/c/tasks.md"])],
            Some(0),
        );
        let task_recorder = crate::testutil::RecordingReader::always(Ok(TASK_FILE.to_string()));
        let read_tasks = |p: &std::path::Path| task_recorder.read(p);

        tasks.sync_detail(&read_tasks);

        assert_eq!(
            shape_of(&tasks.detail),
            vec![(None, 0), (Some("1. Setup"), 0), (Some("2. Build"), 0)],
            "three: the preamble the splitter does not return, then its two groups"
        );
        assert!(
            tasks.detail.foldable(),
            "foldable because `tasks::count` reported three items"
        );
    }

    /// `artifact-folds`: "A spec file whose `### Requirement:` sits inside a
    /// fence does not split" — `is_spec_shaped` asks the splitter, which never
    /// returns a heading from inside a fence.
    #[test]
    fn a_spec_file_whose_requirement_sits_inside_a_fence_does_not_split() {
        const QUOTED: &str = "# Doc\n\n```md\n### Requirement: quoted\n```\n";
        let mut d = dashboard_over(
            &[("specs", &["/repo/openspec/changes/c/specs/a/spec.md"])],
            None,
        );
        let recorder = crate::testutil::RecordingReader::always(Ok(QUOTED.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert_eq!(shape_of(&d.detail), vec![(Some("a"), 0)]);
        assert_eq!(d.detail.sections[0].text, QUOTED);
        assert!(!d.detail.foldable());
    }

    /// `artifact-folds`: "A spec glob nests requirements under their
    /// capability" — a file section at `0`, its own headings under it from
    /// `1`, and the two unsplit siblings back at `0`.
    #[test]
    fn a_spec_glob_nests_requirements_under_their_capability() {
        let mut d = dashboard_over(
            &[(
                "specs",
                &[
                    "/repo/openspec/changes/c/specs/degraded-coverage/spec.md",
                    "/repo/openspec/changes/c/specs/markdown-render/spec.md",
                    "/repo/openspec/changes/c/specs/tasks-checklist/spec.md",
                ],
            )],
            None,
        );
        let recorder = crate::testutil::RecordingReader::new(
            vec![(
                std::path::PathBuf::from(
                    "/repo/openspec/changes/c/specs/degraded-coverage/spec.md",
                ),
                Ok(DELTA_SPEC.to_string()),
            )],
            Ok("## MODIFIED Requirements\n".to_string()),
        );
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert_eq!(
            shape_of(&d.detail),
            vec![
                (Some("degraded-coverage"), 0),
                (Some("ADDED Requirements"), 1),
                (Some("Requirement: Alpha"), 2),
                (Some("Scenario: A works"), 3),
                (Some("Requirement: Beta"), 2),
                (Some("markdown-render"), 0),
                (Some("tasks-checklist"), 0),
            ]
        );
        assert_eq!(
            d.detail.sections[0].text, "",
            "a split file's text lives in its heading sections"
        );
        assert!(
            !d.detail.sections.iter().any(|s| s.label.is_none()),
            "the delta spec's own preamble is empty, so no `None` section appears"
        );
        assert_eq!(recorder.calls(), 3);
    }

    /// `artifact-folds`: "A preamble becomes an unlabelled section" — the text
    /// before a split file's first heading owns no header row and is no fold
    /// target (design.md -> Decision 2).
    #[test]
    fn a_preamble_becomes_an_unlabelled_section() {
        let mut d = dashboard_over(
            &[("tasks", &["/repo/openspec/changes/c/tasks.md"])],
            Some(0),
        );
        let recorder = crate::testutil::RecordingReader::always(Ok(TASK_FILE.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert_eq!(
            shape_of(&d.detail),
            vec![(None, 0), (Some("1. Setup"), 0), (Some("2. Build"), 0)]
        );
        assert_eq!(d.detail.sections[0].text, "Intro prose.\n\n");
        assert_eq!(
            d.detail.sections[0].label, None,
            "the preamble carries no label at all"
        );
    }

    /// One resolved path, an empty preamble, exactly one heading. Splitting
    /// such a file consumes its heading into a `label` that
    /// `content_lines`' **non-foldable** branch never draws, so the heading
    /// row vanishes from the screen — which `artifact-folds` forbids ("SHALL
    /// render the single section's `text` exactly as it rendered it before
    /// this change") and `tasks-checklist` forbids twice over (a group
    /// heading is rendered as a `Face { heading }` line or as a fold header,
    /// "never both, and **never neither**"). So `sync_detail` adopts a
    /// file's split only when it would yield more than one section.
    #[test]
    fn a_single_heading_task_file_is_not_split_and_keeps_its_heading() {
        const ONE_GROUP: &str = "## 1. Setup\n\n- [x] 1.1 first\n- [ ] 1.2 second\n";
        let mut d = dashboard_over(
            &[("tasks", &["/repo/openspec/changes/c/tasks.md"])],
            Some(0),
        );
        let recorder = crate::testutil::RecordingReader::always(Ok(ONE_GROUP.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert_eq!(
            shape_of(&d.detail),
            vec![(Some("tasks.md"), 0)],
            "one unsplit section carrying the file's own label"
        );
        assert_eq!(
            d.detail.sections[0].text, ONE_GROUP,
            "the reader's bytes verbatim"
        );
        assert!(!d.detail.foldable(), "one section is not foldable");

        for width in [78, 58] {
            let change = d.selected_change().expect("a change is selected");
            let rows = crate::ui::detail::content_lines(&d.detail, Some(change), width);
            let want = crate::ui::tasks::lines(ONE_GROUP, &change.progress, width);
            assert_eq!(rows.len(), want.len(), "width {width}");
            for (row, line) in rows.iter().zip(want.iter()) {
                assert_eq!(&row.line, line, "width {width}");
            }
            assert!(
                rows.iter().any(|r| r.text().contains("1. Setup")),
                "width {width}: the heading row is gone: {:?}",
                rows.iter()
                    .map(crate::ui::detail::ContentRow::text)
                    .collect::<Vec<_>>()
            );
        }
    }

    // `title-heading-preamble`, group 1: a document title heading, recognised
    // on a tracked-tasks artifact alone, is demoted to an unlabelled section
    // rather than drawn as a task group with a stray progress cell.
    // `specs/artifact-folds/spec.md` -> the seven scenarios below, verbatim.

    /// `specs/artifact-folds/spec.md` -> "A document title heading is
    /// demoted to an unlabelled section".
    #[test]
    fn a_document_title_heading_is_demoted_to_an_unlabelled_section() {
        const SOURCE: &str = "# drift — tasks\n\nIntro prose.\n\n## 1. Setup\n\n- [x] 1.1 a\n\n## 2. Build\n\n- [ ] 2.1 b\n";
        let mut d = dashboard_over(
            &[("tasks", &["/repo/openspec/changes/c/tasks.md"])],
            Some(0),
        );
        let recorder = crate::testutil::RecordingReader::always(Ok(SOURCE.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert_eq!(
            shape_of(&d.detail),
            vec![(None, 0), (Some("1. Setup"), 0), (Some("2. Build"), 0)]
        );
        assert_eq!(d.detail.sections[0].text, "\nIntro prose.\n\n");
        assert!(
            !d.detail
                .sections
                .iter()
                .any(|s| s.label.as_deref() == Some("drift — tasks")),
            "the title's own text never becomes a section label"
        );
        assert!(
            !d.detail.sections.iter().any(|s| s.depth == 1),
            "excluding the demoted title from min_level un-indents every group"
        );
        assert_eq!(d.detail.sections[0].progress, None);
        assert_eq!(
            d.detail.sections[1].progress,
            Some(crate::tasks::Progress {
                completed: 1,
                total: 1
            })
        );
        assert_eq!(
            d.detail.sections[2].progress,
            Some(crate::tasks::Progress {
                completed: 0,
                total: 1
            })
        );
    }

    /// `specs/artifact-folds/spec.md` -> "A title heading with no prose
    /// under it does not split the file".
    #[test]
    fn a_title_heading_with_no_prose_under_it_does_not_split_the_file() {
        const SOURCE: &str = "# drift — tasks\n## 1. Setup\n\n- [x] 1.1 a\n";
        let mut d = dashboard_over(
            &[("tasks", &["/repo/openspec/changes/c/tasks.md"])],
            Some(0),
        );
        let recorder = crate::testutil::RecordingReader::always(Ok(SOURCE.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert_eq!(
            shape_of(&d.detail),
            vec![(Some("tasks.md"), 0)],
            "one unsplit section carrying the file's own label"
        );
        assert_eq!(
            d.detail.sections[0].text, SOURCE,
            "the reader's bytes verbatim"
        );
        assert!(!d.detail.foldable(), "one section is not foldable");
    }

    /// `specs/artifact-folds/spec.md` -> "A whitespace-only title body
    /// contributes no section".
    #[test]
    fn a_whitespace_only_title_body_contributes_no_section() {
        const SOURCE: &str =
            "# drift — tasks\n\n## 1. Setup\n\n- [x] 1.1 a\n\n## 2. Build\n\n- [ ] 2.1 b\n";
        let mut d = dashboard_over(
            &[("tasks", &["/repo/openspec/changes/c/tasks.md"])],
            Some(0),
        );
        let recorder = crate::testutil::RecordingReader::always(Ok(SOURCE.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert_eq!(
            shape_of(&d.detail),
            vec![(Some("1. Setup"), 0), (Some("2. Build"), 0)]
        );
        assert!(
            !d.detail.sections.iter().any(|s| s.label.is_none()),
            "the title body is whitespace-only, so no unlabelled section is contributed"
        );
        assert!(
            d.detail.foldable(),
            "the file still splits, its contribution count being two"
        );
        let change = d.selected_change().expect("a change is selected");
        let rows = crate::ui::detail::content_lines(&d.detail, Some(change), 78);
        for (index, section) in d.detail.sections.iter().enumerate() {
            assert!(
                rows.iter().any(|r| matches!(
                    r.kind,
                    crate::ui::detail::ContentKind::SectionHeader { section: s, .. } if s == index
                )),
                "section {index} ({:?}) contributes no header row in content_lines",
                section.label
            );
        }
    }

    /// The `min_level` fallback when demotion leaves no labelled heading at
    /// all — `title-heading-preamble`'s design.md -> D5's `unwrap_or(0)`.
    /// The file's only heading is the recognised title, so `headings.iter()`
    /// filtered to exclude `title_index` is empty and `.min()` would panic
    /// without the fallback; both the preamble and the demoted title's own
    /// body land as `None`-labelled sections at depth `0`.
    #[test]
    fn a_file_with_no_labelled_heading_after_demotion_falls_back_to_depth_zero() {
        const SOURCE: &str = "- [ ] a\n\n# T\n\nprose\n";
        let mut d = dashboard_over(
            &[("tasks", &["/repo/openspec/changes/c/tasks.md"])],
            Some(0),
        );
        let recorder = crate::testutil::RecordingReader::always(Ok(SOURCE.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert_eq!(
            shape_of(&d.detail),
            vec![(None, 0), (None, 0)],
            "the preamble and the demoted title's own body, no labelled section at all"
        );
        assert_eq!(d.detail.sections[0].text, "- [ ] a\n\n");
        assert_eq!(d.detail.sections[1].text, "\nprose\n");
    }

    /// `specs/artifact-folds/spec.md` -> "A leading heading holding its own
    /// items is a group, not a title".
    #[test]
    fn a_leading_heading_holding_its_own_items_is_a_group_not_a_title() {
        const SOURCE: &str = "# drift — tasks\n\n- [ ] 0.1 a\n\n## 1. Setup\n\n- [x] 1.1 b\n";
        let mut d = dashboard_over(
            &[("tasks", &["/repo/openspec/changes/c/tasks.md"])],
            Some(0),
        );
        let recorder = crate::testutil::RecordingReader::always(Ok(SOURCE.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert_eq!(
            shape_of(&d.detail),
            vec![(Some("drift — tasks"), 0), (Some("1. Setup"), 1)],
            "clause 3 of the title rule fails, so this is the pre-change derivation"
        );
        assert_eq!(
            d.detail.sections[0].progress,
            Some(crate::tasks::Progress {
                completed: 0,
                total: 1
            })
        );
    }

    /// `specs/artifact-folds/spec.md` -> "Two headings at the file's
    /// shallowest level are both groups".
    #[test]
    fn two_headings_at_the_files_shallowest_level_are_both_groups() {
        const SOURCE: &str = "# A\n\nprose\n\n# B\n\n- [ ] x\n";
        let mut d = dashboard_over(
            &[("tasks", &["/repo/openspec/changes/c/tasks.md"])],
            Some(0),
        );
        let recorder = crate::testutil::RecordingReader::always(Ok(SOURCE.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert_eq!(
            shape_of(&d.detail),
            vec![(Some("A"), 0), (Some("B"), 0)],
            "clause 2 of the title rule fails: two headings share the shallowest level"
        );
        assert!(!d.detail.sections.iter().any(|s| s.label.is_none()));
    }

    /// Clause 1 of the title rule: the title heading must be the **first**
    /// heading in document order, not merely a heading at the file's
    /// shallowest level. `## Notes` sits at level two and comes first; `#
    /// Plan` sits at level one, later, and is alone at that level — clauses 2
    /// and 3 both pass for `## Notes` taken on its own, so only clause 1 (the
    /// `first.level != min_level` check in `title_heading`) tells the two
    /// apart and keeps `## Notes` an ordinary group.
    #[test]
    fn a_first_heading_deeper_than_the_files_shallowest_level_is_not_a_title() {
        const SOURCE: &str = "## Notes\n\nprose\n\n# Plan\n\n- [ ] x\n";
        let mut d = dashboard_over(
            &[("tasks", &["/repo/openspec/changes/c/tasks.md"])],
            Some(0),
        );
        let recorder = crate::testutil::RecordingReader::always(Ok(SOURCE.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert_eq!(
            shape_of(&d.detail),
            vec![(Some("Notes"), 1), (Some("Plan"), 0)],
            "clause 1 of the title rule fails: the first heading is not the file's shallowest"
        );
    }

    /// `specs/artifact-folds/spec.md` -> "A preamble and a demoted title are
    /// two unlabelled sections".
    #[test]
    fn a_preamble_and_a_demoted_title_are_two_unlabelled_sections() {
        const SOURCE: &str = "Intro.\n\n# drift — tasks\n\nMore prose.\n\n## 1. Setup\n\n- [ ] x\n";
        let mut d = dashboard_over(
            &[("tasks", &["/repo/openspec/changes/c/tasks.md"])],
            Some(0),
        );
        let recorder = crate::testutil::RecordingReader::always(Ok(SOURCE.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert_eq!(
            shape_of(&d.detail),
            vec![(None, 0), (None, 0), (Some("1. Setup"), 0)]
        );
        assert_eq!(d.detail.sections[0].text, "Intro.\n\n");
        assert_eq!(d.detail.sections[1].text, "\nMore prose.\n\n");

        // `route: Route::Detail` and `detail.drawn_width: Some(78)` are both
        // required, or `apply` never reaches the detail cursor at all and the
        // check below would pass vacuously.
        d.route = Route::Detail;
        d.detail.drawn_width = Some(78);
        let rows = crate::ui::detail::content_lines(&d.detail, d.selected_change(), 78);
        for needle in ["Intro", "More prose"] {
            let scroll = rows
                .iter()
                .position(|r| r.text().contains(needle))
                .unwrap_or_else(|| panic!("no row contains {needle:?}: {rows:?}"));
            d.detail.scroll = scroll;
            let before = d.detail.expanded.clone();
            for _ in 0..10 {
                d.apply(Action::ToggleSection);
            }
            assert_eq!(
                d.detail.expanded, before,
                "neither unlabelled section owns a header row, so Space stays inert"
            );
        }

        // Positive control: a labelled header row (`1. Setup`) does still
        // toggle, proving the two inert checks above actually exercised
        // `Action::ToggleSection`'s section lookup at the detail cursor
        // rather than passing vacuously for some unrelated reason (the
        // cursor never reaching `apply`, or `ToggleSection` failing to look
        // up a section at all).
        let setup_row = rows
            .iter()
            .position(|r| r.text().contains("1. Setup"))
            .unwrap_or_else(|| panic!("no row contains \"1. Setup\": {rows:?}"));
        d.detail.scroll = setup_row;
        let before = d.detail.expanded.clone();
        d.apply(Action::ToggleSection);
        assert_ne!(
            d.detail.expanded, before,
            "the labelled header row does toggle"
        );
    }

    /// `title-heading-preamble`'s design.md -> D6: the operation walk
    /// advances over **every** heading in document order, the demoted title
    /// included, so a `## ADDED Requirements` title still classifies the
    /// requirements that follow it even though the title itself is demoted
    /// and draws no header row of its own on a tracked-tasks artifact.
    #[test]
    fn the_operation_walk_advances_over_a_demoted_title_heading() {
        const SOURCE: &str = "## ADDED Requirements\n\n### Requirement: X\n\n- [ ] 1.1 x\n\n### Requirement: Y\n\n- [ ] 1.2 y\n";
        let mut d = dashboard_over(
            &[("tasks", &["/repo/openspec/changes/c/tasks.md"])],
            Some(0),
        );
        let recorder = crate::testutil::RecordingReader::always(Ok(SOURCE.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert_eq!(
            shape_of(&d.detail),
            vec![(Some("Requirement: X"), 0), (Some("Requirement: Y"), 0)],
            "the title is demoted (its own body is empty) and contributes no section"
        );
        for section in &d.detail.sections {
            assert_eq!(
                section.operation,
                Some(crate::specs::DeltaOp::Added),
                "{:?}: the operation walk must reach every requirement past the demoted title",
                section.label
            );
        }
    }

    /// `specs/artifact-folds/spec.md` -> "A spec tab's lone operation
    /// heading keeps its header row".
    #[test]
    fn a_spec_tabs_lone_operation_heading_keeps_its_header_row() {
        const SOURCE: &str =
            "## ADDED Requirements\n\n### Requirement: Alpha\n\n#### Scenario: A works\n";
        let mut d = dashboard_over(
            &[("specs", &["/repo/openspec/changes/c/specs/a/spec.md"])],
            None,
        );
        let recorder = crate::testutil::RecordingReader::always(Ok(SOURCE.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert_eq!(
            shape_of(&d.detail),
            vec![
                (Some("ADDED Requirements"), 0),
                (Some("Requirement: Alpha"), 1),
                (Some("Scenario: A works"), 2),
            ],
            "the title rule is gated on tracks_tasks, which this artifact does not carry"
        );
        assert_eq!(
            d.detail.sections[1].operation,
            Some(crate::specs::DeltaOp::Added)
        );
    }

    /// The two-path counterpart of the test above, and the derivation that
    /// `ui::detail`'s `the_tracked_tasks_tab_concatenates_rather_than_folding`
    /// transcribes by hand rather than executing. Two things run nowhere
    /// else: the derivation of a **multi-path** tracked-tasks artifact — one
    /// file section at `depth` `0` per path with that file's own groups
    /// nested beneath it at `depth` `1` — and `seed_expanded`'s subtree walk
    /// at a **non-zero** `base`, where a group sits under a file section
    /// rather than at the root. Each file here carries a single heading, so
    /// this is also the far side of the split gate's
    /// `base > 0 || contributions > 1`: with a file section ahead of it that
    /// heading does draw as a header row, so the file splits where the
    /// one-path fixture above refuses to.
    ///
    /// Two path pairs, because a file section's label keys on the file name:
    /// `tasks.md` labels by file name — twice over, since labels are not
    /// required to be unique — while `spec.md` labels by its parent
    /// directory and so reproduces the `a`/`b` labels `ui::detail`
    /// transcribes, binding the two shapes literally.
    #[test]
    fn a_two_path_tracked_tasks_artifact_nests_its_groups_and_seeds_them_all() {
        const SETUP: &str = "## 1. Setup\n\n- [ ] a\n";
        const BUILD: &str = "## 2. Build\n\n- [ ] b\n";

        for (first, second, labels) in [
            (
                "/repo/openspec/changes/c/a/tasks.md",
                "/repo/openspec/changes/c/b/tasks.md",
                ["tasks.md", "tasks.md"],
            ),
            (
                "/repo/openspec/changes/c/a/spec.md",
                "/repo/openspec/changes/c/b/spec.md",
                ["a", "b"],
            ),
        ] {
            let mut d = dashboard_over(&[("tasks", &[first, second])], Some(0));
            let recorder = crate::testutil::RecordingReader::new(
                vec![
                    (std::path::PathBuf::from(first), Ok(SETUP.to_string())),
                    (std::path::PathBuf::from(second), Ok(BUILD.to_string())),
                ],
                Err("unexpected".to_string()),
            );
            let read = |p: &std::path::Path| recorder.read(p);

            d.sync_detail(&read);

            assert_eq!(
                shape_of(&d.detail),
                vec![
                    (Some(labels[0]), 0),
                    (Some("1. Setup"), 1),
                    (Some(labels[1]), 0),
                    (Some("2. Build"), 1),
                ],
                "{first}: a file section per path, each file's group beneath it"
            );
            assert_eq!(
                (
                    d.detail.sections[0].text.as_str(),
                    d.detail.sections[1].text.as_str(),
                    d.detail.sections[2].text.as_str(),
                    d.detail.sections[3].text.as_str(),
                ),
                // The group's body is the **verbatim** bytes between its own
                // heading line and the next heading line (design.md -> D6),
                // so the blank line after the heading belongs to the body:
                // heading line + body reassembles the file exactly.
                ("", "\n- [ ] a\n", "", "\n- [ ] b\n"),
                "{first}: a split file's text lives in its heading sections"
            );
            assert_eq!(
                d.detail.expanded,
                std::collections::BTreeSet::from([0, 1, 2, 3]),
                "{first}: every subtree is incomplete, so every index is seeded"
            );
            assert!(d.detail.foldable(), "{first}: four sections are foldable");
        }
    }

    /// The same defect on the spec axis: a delta carrying exactly one
    /// `### Requirement:` heading and no preamble lost that requirement's
    /// own name off the screen.
    #[test]
    fn a_single_requirement_spec_file_is_not_split_and_keeps_its_heading() {
        const ONE_REQUIREMENT: &str = "### Requirement: Alpha\nAlpha text.\n";
        let mut d = dashboard_over(
            &[("specs", &["/repo/openspec/changes/c/specs/a/spec.md"])],
            None,
        );
        let recorder = crate::testutil::RecordingReader::always(Ok(ONE_REQUIREMENT.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert_eq!(
            shape_of(&d.detail),
            vec![(Some("a"), 0)],
            "one unsplit section carrying the file's own label"
        );
        assert_eq!(
            d.detail.sections[0].text, ONE_REQUIREMENT,
            "the reader's bytes verbatim"
        );
        assert!(!d.detail.foldable(), "one section is not foldable");

        for width in [78, 58] {
            let change = d.selected_change().expect("a change is selected");
            let rows = crate::ui::detail::content_lines(&d.detail, Some(change), width);
            let want = crate::ui::markdown::lines(ONE_REQUIREMENT, width);
            assert_eq!(rows.len(), want.len(), "width {width}");
            for (row, line) in rows.iter().zip(want.iter()) {
                assert_eq!(&row.line, line, "width {width}");
            }
            assert!(
                rows.iter().any(|r| r.text().contains("Requirement: Alpha")),
                "width {width}: the heading row is gone: {:?}",
                rows.iter()
                    .map(crate::ui::detail::ContentRow::text)
                    .collect::<Vec<_>>()
            );
        }
    }

    /// `artifact-folds` / design.md -> Decision 6: sections partition the file
    /// rather than copying it. Reassembling every section's `text` with the
    /// heading line each labelled section's `(depth, label)` names reproduces
    /// the reader's bytes exactly — over a spec file whose preamble is empty
    /// and a task file whose preamble is not.
    #[test]
    fn a_split_file_is_partitioned_rather_than_copied() {
        // Both fixtures start at `##` and resolve to one path, so a section's
        // own heading level is `2 + depth`.
        for (artifact, marked, source) in [
            (
                ("specs", ["/repo/openspec/changes/c/specs/a/spec.md"]),
                None,
                DELTA_SPEC,
            ),
            (
                ("tasks", ["/repo/openspec/changes/c/tasks.md"]),
                Some(0),
                TASK_FILE,
            ),
        ] {
            let mut d = dashboard_over(&[(artifact.0, &artifact.1)], marked);
            let recorder = crate::testutil::RecordingReader::always(Ok(source.to_string()));
            let read = |p: &std::path::Path| recorder.read(p);

            d.sync_detail(&read);

            let parts: Vec<String> = d
                .detail
                .sections
                .iter()
                .map(|s| match &s.label {
                    Some(label) => format!("{} {label}\n{}", "#".repeat(2 + s.depth), s.text),
                    None => s.text.clone(),
                })
                .collect();
            assert_eq!(parts.concat(), source, "round trip over {artifact:?}");
        }
    }

    /// A `Route::Detail` dashboard over one active, selected change, carrying
    /// `detail` verbatim. The common shape group 7's `Space`-at-the-detail-
    /// route scenarios build on, mirroring `ui::view::tests::dashboard_with_detail`.
    fn dashboard_with_detail(detail: Detail) -> Dashboard {
        let change =
            fixture::with_artifacts(fixture::active("detail-view", 4, 9), &[("proposal", &[])]);
        let mut d =
            dashboard_for_attribution(vec![change], Vec::new(), 1, Vec::new(), BTreeMap::new());
        d.route = Route::Detail;
        d.detail = detail;
        d
    }

    /// Mirrors `ui::view::tests::three_spec_dashboard` /
    /// `ui::detail::tests::three_spec_detail`: three short, non-wrapping
    /// sections, so a scenario about *which* section `Space` acts on does
    /// not also have to reason about `ui::markdown`'s own wrapping. Labels
    /// and bodies match the ones
    /// `the_three_spec_files_of_a_change_become_three_labelled_sections` and
    /// `three_spec_dashboard` already fix for the same three-file shape.
    fn three_spec_detail(expanded: std::collections::BTreeSet<usize>, scroll: usize) -> Detail {
        Detail {
            sections: vec![
                ArtifactSection {
                    label: Some("degraded-coverage".to_string()),
                    text: "one\n".to_string(),
                    depth: 0,
                    progress: None,
                    operation: None,
                },
                ArtifactSection {
                    label: Some("markdown-render".to_string()),
                    text: "two\n".to_string(),
                    depth: 0,
                    progress: None,
                    operation: None,
                },
                ArtifactSection {
                    label: Some("tasks-checklist".to_string()),
                    text: "three\n".to_string(),
                    depth: 0,
                    progress: None,
                    operation: None,
                },
            ],
            scroll,
            tab: 0,
            problems: Vec::new(),
            loaded: None,
            expanded,
            // 78 columns: the mandated wide detail interior. These bodies
            // are single words, so no wrap -- the row list is the same at
            // any width, and the fold resolves rather than going inert.
            drawn_width: Some(78),
        }
    }

    /// `dashboard_with_detail(three_spec_detail(expanded, scroll))` — the
    /// shape most of group 7's `artifact-folds` and `detail-scroll`
    /// scenarios need.
    fn foldable_dashboard(expanded: std::collections::BTreeSet<usize>, scroll: usize) -> Dashboard {
        dashboard_with_detail(three_spec_detail(expanded, scroll))
    }

    /// `<glyph> <label>` padded to `width` — built the same way
    /// `ui::detail::header` builds it, from `ui::list::fold_glyph` and
    /// `ui::list::pad_or_truncate_right`, so a test never writes its own
    /// copy of the glyph pair. Mirrors `ui::view::tests::expected_header`.
    fn expected_header(label: &str, collapsed: bool, width: u16) -> String {
        let glyph = crate::ui::list::fold_glyph(collapsed);
        crate::ui::list::pad_or_truncate_right(&format!("{glyph} {label}"), width as usize)
    }

    /// The detail content area's row `y`, across its own mandated interior
    /// width — 78 at 120 columns, 58 at 60 — the same band
    /// `ui::view::tests::detail_interior_cols` measures.
    fn detail_interior_row(buf: &ratatui::buffer::Buffer, y: u16) -> String {
        let (from, width) = if buf.area.width == 60 {
            (1, 58)
        } else {
            (42, 78)
        };
        crate::testutil::row_text(buf, y)
            .chars()
            .skip(from)
            .take(width)
            .collect()
    }

    /// `artifact-folds`: "Space opens the section under the cursor and
    /// leaves its siblings shut".
    #[test]
    fn space_opens_the_section_under_the_cursor_and_leaves_its_siblings_shut() {
        let mut d = foldable_dashboard(std::collections::BTreeSet::new(), 1);
        let before = d.clone();

        d.apply(Action::ToggleSection);

        assert_eq!(d.detail.expanded, std::collections::BTreeSet::from([1]));
        assert_eq!(
            d.detail.scroll, 1,
            "did not move: nothing above it changed height"
        );
        assert_eq!(d.sections.collapsed, before.sections.collapsed);
        assert_eq!(d.selected, before.selected);
        assert_eq!(d.refresh.requested, before.refresh.requested);
        assert_eq!(d.route, before.route);
        assert_eq!(d.filter, before.filter);
        assert_eq!(d.changes, before.changes);

        for (width, height) in [(120u16, 40u16), (60, 40)] {
            let buf = crate::testutil::render_at(width, height, &d);
            let interior = if width == 60 { 58 } else { 78 };
            assert_eq!(
                detail_interior_row(&buf, 5),
                expected_header("degraded-coverage", true, interior),
                "width {width}: first header, collapsed"
            );
            assert_eq!(
                detail_interior_row(&buf, 6),
                expected_header("markdown-render", false, interior),
                "width {width}: second header, opened"
            );
            let body = crate::ui::markdown::lines("two\n", interior);
            assert_eq!(body.len(), 1, "width {width}: the fixture body is one line");
            assert_eq!(
                detail_interior_row(&buf, 7),
                crate::ui::list::pad_or_truncate_right(&body[0].text(), interior as usize),
                "width {width}: the opened section's own body"
            );
            // `heading-sections`: a blank separator row follows a non-empty
            // open body that a further visible section follows.
            assert_eq!(
                detail_interior_row(&buf, 8).trim(),
                "",
                "width {width}: the separator below the opened body"
            );
            assert_eq!(
                detail_interior_row(&buf, 9),
                expected_header("tasks-checklist", true, interior),
                "width {width}: third header, still collapsed"
            );
        }
    }

    /// `artifact-folds`: "Space inside an open section folds it and moves
    /// the cursor to its header".
    #[test]
    fn space_inside_an_open_section_folds_it_and_moves_the_cursor_to_its_header() {
        // Section 0 open (one body line), cursor at row 1 — inside its body.
        let mut d = foldable_dashboard(std::collections::BTreeSet::from([0]), 1);

        d.apply(Action::ToggleSection);

        assert!(d.detail.expanded.is_empty());
        assert_eq!(d.detail.scroll, 0, "the folded section's own header row");
        for (width, height) in [(120u16, 40u16), (60, 40)] {
            let interior = if width == 60 { 58 } else { 78 };
            let rows = crate::ui::detail::content_lines(&d.detail, d.selected_change(), interior);
            assert_eq!(rows.len(), 3, "width {width}: three collapsed sections");
            assert!(
                rows.iter().all(|r| matches!(
                    r.kind,
                    crate::ui::detail::ContentKind::SectionHeader { .. }
                )),
                "width {width}: every row is a header"
            );

            let buf = crate::testutil::render_at(width, height, &d);
            for (row, label) in [
                (5u16, "degraded-coverage"),
                (6, "markdown-render"),
                (7, "tasks-checklist"),
            ] {
                assert_eq!(
                    detail_interior_row(&buf, row),
                    expected_header(label, true, interior),
                    "width {width} row {row}: collapsed"
                );
            }
        }

        // Section 0 still open, cursor now on the third header row — the row
        // index depends on the one body line the open first section
        // contributes **and** on the blank separator row `heading-sections`
        // emits after it, so the third header sits at row 4 rather than row 3.
        let mut third = foldable_dashboard(std::collections::BTreeSet::from([0]), 4);
        third.apply(Action::ToggleSection);
        assert_eq!(
            third.detail.expanded,
            std::collections::BTreeSet::from([0, 2]),
            "the third section opened and the first stayed open"
        );
    }

    /// `artifact-folds`: "Space opens the section under the cursor" — the
    /// regression test for the Change Review's second CRITICAL.
    ///
    /// A section body that **wraps** at the drawn width is what separates
    /// resolving against `drawn_width` from resolving at `u16::MAX`
    /// (design.md -> Decision 13). At 40 columns this body occupies several
    /// rows; unwrapped it occupies one. A cursor parked inside the *second*
    /// section's wrapped body therefore resolves to section 1 against the real
    /// width and to something else against the unwrapped list — which is the
    /// defect exactly: the reader folds a section they are not looking at.
    ///
    /// Fails against the `u16::MAX` implementation and passes against the
    /// recorded-width one, which is what makes the repair falsifiable rather
    /// than asserted.
    #[test]
    fn a_wrapping_body_folds_the_section_the_cursor_is_actually_in() {
        let long = "alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima\n";
        let section = |label: &str| ArtifactSection {
            label: Some(label.to_string()),
            text: long.to_string(),
            depth: 0,
            progress: None,
            operation: None,
        };
        let mut d = dashboard_with_detail(Detail {
            sections: vec![section("first"), section("second"), section("third")],
            scroll: 0,
            tab: 0,
            problems: Vec::new(),
            loaded: None,
            expanded: std::collections::BTreeSet::from([0, 1, 2]),
            drawn_width: Some(40),
        });
        d.route = Route::Detail;

        let at_40 = crate::ui::detail::content_lines(&d.detail, d.selected_change(), 40);
        let unwrapped = crate::ui::detail::content_lines(&d.detail, d.selected_change(), u16::MAX);

        // The cursor sits on the **last** row of section 0's wrapped body.
        let header_1 = at_40
            .iter()
            .position(|r| {
                matches!(
                    r.kind,
                    crate::ui::detail::ContentKind::SectionHeader { section: 1, .. }
                )
            })
            .expect("section 1 has a header row at 40");
        let inside_0 = header_1 - 1;

        // The premise that makes this discriminate: at the drawn width that row
        // is inside section 0, while the same index in the unwrapped list is a
        // different section entirely. Asserted, not assumed -- if a future
        // change stops the body wrapping, this test says so instead of quietly
        // passing for the wrong reason.
        let section_of = |rows: &[crate::ui::detail::ContentRow], row: usize| {
            rows[..=row]
                .iter()
                .rev()
                .find_map(|r| match r.kind {
                    crate::ui::detail::ContentKind::SectionHeader { section, .. } => Some(section),
                    _ => None,
                })
                .expect("a header precedes every body row")
        };
        assert_eq!(
            section_of(&at_40, inside_0),
            0,
            "at 40 the row is in section 0"
        );
        assert!(
            inside_0 < unwrapped.len(),
            "the row must exist in the unwrapped list too, or the comparison is vacuous"
        );
        assert_ne!(
            section_of(&unwrapped, inside_0),
            0,
            "unwrapped, the same index names a different section -- that gap IS the defect"
        );

        d.detail.scroll = inside_0;
        d.apply(Action::ToggleSection);

        assert_eq!(
            d.detail.expanded,
            std::collections::BTreeSet::from([1, 2]),
            "the cursor was inside section 0's wrapped body, so that is the section that folds"
        );
    }

    /// `artifact-folds`: "Space on a problem row is inert".
    #[test]
    fn space_on_a_problem_row_is_inert() {
        let mut d = dashboard_with_detail(Detail {
            sections: vec![
                ArtifactSection {
                    label: Some("a".to_string()),
                    text: "one\n".to_string(),
                    depth: 0,
                    progress: None,
                    operation: None,
                },
                ArtifactSection {
                    label: Some("b".to_string()),
                    text: "two\n".to_string(),
                    depth: 0,
                    progress: None,
                    operation: None,
                },
            ],
            scroll: 0,
            tab: 0,
            problems: vec!["permission denied".to_string()],
            loaded: None,
            expanded: std::collections::BTreeSet::new(),
            // A real width, not `None`: with no drawn width the fold is inert
            // whatever else is true, so this assertion would pass even
            // with the guard it is testing removed.
            drawn_width: Some(78),
        });
        let before = d.clone();
        for _ in 0..10 {
            d.apply(Action::ToggleSection);
        }
        assert_eq!(d, before, "no field changed, and it did not panic");

        d.detail.scroll = 1;
        d.apply(Action::ToggleSection);
        assert_eq!(
            d.detail.expanded,
            std::collections::BTreeSet::from([0]),
            "moving off the problem row lets the same action toggle the first section"
        );
    }

    /// `artifact-folds`: "Space is inert on a non-foldable artifact".
    #[test]
    fn space_is_inert_on_a_non_foldable_artifact() {
        let mut one = dashboard_with_detail(Detail {
            sections: vec![ArtifactSection {
                label: Some("a".to_string()),
                text: "one\n".to_string(),
                depth: 0,
                progress: None,
                operation: None,
            }],
            scroll: 0,
            tab: 0,
            problems: Vec::new(),
            loaded: None,
            expanded: std::collections::BTreeSet::new(),
            // A real width, not `None`: with no drawn width the fold is inert
            // whatever else is true, so this assertion would pass even
            // with the guard it is testing removed.
            drawn_width: Some(78),
        });
        let before_one = one.clone();
        for _ in 0..10 {
            one.apply(Action::ToggleSection);
        }
        assert_eq!(one, before_one);

        let mut none = dashboard_with_detail(Detail {
            sections: Vec::new(),
            scroll: 0,
            tab: 0,
            problems: Vec::new(),
            loaded: None,
            expanded: std::collections::BTreeSet::new(),
            drawn_width: None,
        });
        let before_none = none.clone();
        for _ in 0..10 {
            none.apply(Action::ToggleSection);
        }
        assert_eq!(none, before_none);
    }

    /// `list-selection`: "Space at the detail route leaves the list alone".
    #[test]
    fn space_at_the_detail_route_leaves_the_list_alone() {
        let mut d = foldable_dashboard(std::collections::BTreeSet::new(), 1);
        let before = d.clone();
        let buf_before = crate::testutil::render_at(120, 20, &d);

        d.apply(Action::ToggleSection);

        assert_eq!(d.sections.collapsed, before.sections.collapsed);
        assert_eq!(d.selected, before.selected);
        assert_eq!(d.refresh.requested, before.refresh.requested);
        assert_eq!(d.detail.expanded, std::collections::BTreeSet::from([1]));

        let buf_after = crate::testutil::render_at(120, 20, &d);
        for y in 0..20u16 {
            let before_row: String = crate::testutil::row_text(&buf_before, y)
                .chars()
                .take(40)
                .collect();
            let after_row: String = crate::testutil::row_text(&buf_after, y)
                .chars()
                .take(40)
                .collect();
            assert_eq!(
                before_row, after_row,
                "row {y}: the list region did not move"
            );
        }
    }

    /// `list-selection`: "Space at the detail route is inert on a
    /// non-foldable artifact".
    #[test]
    fn space_at_the_detail_route_is_inert_on_a_non_foldable_artifact() {
        let mut d = dashboard_with_detail(Detail {
            sections: vec![ArtifactSection {
                label: Some("a".to_string()),
                text: "one\n".to_string(),
                depth: 0,
                progress: None,
                operation: None,
            }],
            scroll: 0,
            tab: 0,
            problems: Vec::new(),
            loaded: None,
            expanded: std::collections::BTreeSet::new(),
            // A real width, not `None`: with no drawn width the fold is inert
            // whatever else is true, so this assertion would pass even
            // with the guard it is testing removed.
            drawn_width: Some(78),
        });
        let before = d.clone();
        for _ in 0..10 {
            d.apply(Action::ToggleSection);
        }
        assert_eq!(d, before);
    }

    /// The three-section task dashboard `artifact-folds`' preamble scenario
    /// names: `TASK_FILE` on a `tracks_tasks` artifact, split by `sync_detail`
    /// into a `None`-labelled preamble and its two groups. The change's own
    /// progress is `1/3` rather than the `0/0` `dashboard_over` fixes, because
    /// a change with no tasks draws no gauge at all — and the progress-bar row
    /// is exactly the preamble row this scenario is about.
    fn task_preamble_dashboard() -> Dashboard {
        let change = fixture::track_tasks_at(
            fixture::with_artifacts(
                fixture::active("c", 1, 3),
                &[("tasks", &["/repo/openspec/changes/c/tasks.md"])],
            ),
            0,
        );
        let mut d =
            dashboard_for_attribution(vec![change], Vec::new(), 1, Vec::new(), BTreeMap::new());
        d.route = Route::Detail;
        let recorder = crate::testutil::RecordingReader::always(Ok(TASK_FILE.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);
        d.sync_detail(&read);
        // The mandated wide detail interior, so the fold resolves rather than
        // going inert for want of a drawn width (design.md -> Decision 13).
        d.detail.drawn_width = Some(78);
        d
    }

    /// A `tracks_tasks` dashboard whose one path reads `text`, synced.
    /// `progress` is the change's own field, which the bar renders and which
    /// the per-group cells are deliberately independent of.
    fn synced_tracked_tasks_dashboard(text: &str, completed: usize, total: usize) -> Dashboard {
        let change = fixture::track_tasks_at(
            fixture::with_artifacts(
                fixture::active("c", completed, total),
                &[("tasks", &["/repo/openspec/changes/c/tasks.md"])],
            ),
            0,
        );
        let mut d =
            dashboard_for_attribution(vec![change], Vec::new(), 1, Vec::new(), BTreeMap::new());
        d.route = Route::Detail;
        let recorder = crate::testutil::RecordingReader::always(Ok(text.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);
        d.sync_detail(&read);
        d.detail.drawn_width = Some(78);
        d
    }

    fn progress_of(detail: &Detail) -> Vec<Option<crate::tasks::Progress>> {
        detail.sections.iter().map(|s| s.progress).collect()
    }

    fn progress(completed: usize, total: usize) -> Option<crate::tasks::Progress> {
        Some(crate::tasks::Progress { completed, total })
    }

    /// `artifact-folds` :: "A tracked-tasks tab's group headers carry their own
    /// progress" — the unit half. The view half is `ui::view`'s test of the
    /// same name, which is what shows the cell reaching a buffer.
    #[test]
    fn a_tracked_tasks_tabs_group_headers_carry_their_own_progress() {
        let d = synced_tracked_tasks_dashboard(
            "## 1. Setup\n\n- [x] 1.1 first\n- [ ] 1.2 second\n\n## 2. Build\n\n- [ ] 2.1 third\n",
            1,
            3,
        );
        assert_eq!(
            shape_of(&d.detail),
            vec![(Some("1. Setup"), 0), (Some("2. Build"), 0)],
            "two heading sections and no preamble"
        );
        assert_eq!(
            progress_of(&d.detail),
            vec![progress(1, 2), progress(0, 1)],
            "each section carries its own group's count"
        );

        // The preamble and the file section of the same shape carry `None`, so
        // only heading sections gain a cell.
        let with_preamble = synced_tracked_tasks_dashboard(TASK_FILE, 1, 3);
        assert_eq!(
            progress_of(&with_preamble.detail),
            vec![None, progress(1, 2), progress(0, 1)],
            "the `None`-labelled preamble is not a task group"
        );
    }

    /// `artifact-folds` :: "A group holding no items still gets a header and a
    /// counted cell" — the unit half.
    #[test]
    fn a_group_holding_no_items_still_gets_a_header_and_a_counted_cell() {
        let d = synced_tracked_tasks_dashboard(
            "## 1. Notes\n\nprose only\n\n## 2. Build\n\n- [ ] 2.1 third\n",
            0,
            1,
        );
        assert_eq!(
            shape_of(&d.detail),
            vec![(Some("1. Notes"), 0), (Some("2. Build"), 0)],
        );
        assert_eq!(
            progress_of(&d.detail),
            vec![progress(0, 0), progress(0, 1)],
            "a prose group is counted and found empty, not left uncounted"
        );
        // `[-]` and `[0/1]` are different cells, which is the distinction a
        // `Progress` defaulting to `{0, 0}` on every section would have erased.
        assert_ne!(
            crate::ui::list::progress_cell(&d.detail.sections[0].progress.unwrap()),
            crate::ui::list::progress_cell(&d.detail.sections[1].progress.unwrap()),
        );
    }

    /// Every other artifact's sections carry no progress at all — the unit half
    /// of `artifact-folds` :: "Every other artifact's section headers carry no
    /// progress cell".
    #[test]
    fn a_non_tracked_tasks_artifacts_sections_carry_no_progress() {
        let change = fixture::with_artifacts(
            fixture::active("c", 1, 3),
            &[(
                "specs",
                &[
                    "/repo/openspec/changes/c/specs/a/spec.md",
                    "/repo/openspec/changes/c/specs/b/spec.md",
                ],
            )],
        );
        let mut d =
            dashboard_for_attribution(vec![change], Vec::new(), 1, Vec::new(), BTreeMap::new());
        d.route = Route::Detail;
        let recorder = crate::testutil::RecordingReader::always(Ok(
            "## MODIFIED Requirements\n\n- [ ] not a task file\n".to_string(),
        ));
        let read = |p: &std::path::Path| recorder.read(p);
        d.sync_detail(&read);
        assert!(
            d.detail.sections.iter().all(|s| s.progress.is_none()),
            "no section of a non-tracked-tasks artifact carries a count"
        );
    }

    /// `spec-emphasis` / `artifact-folds` :: the attribution walk. Every
    /// section's `operation`, in order.
    fn operation_of(detail: &Detail) -> Vec<Option<crate::specs::DeltaOp>> {
        detail.sections.iter().map(|s| s.operation).collect()
    }

    /// `artifact-folds` :: "A delta spec's requirement sections carry their
    /// operation and nothing else does" — a level-2 operation heading resets
    /// the walk rather than nesting, and the operation heading itself, the
    /// file section ahead of it, and every non-requirement section stay
    /// unbadged.
    #[test]
    fn a_delta_specs_requirement_sections_carry_their_operation_and_nothing_else_does() {
        let mut d = dashboard_over(
            &[(
                "specs",
                &[
                    "/repo/openspec/changes/c/specs/degraded-coverage/spec.md",
                    "/repo/openspec/changes/c/specs/markdown-render/spec.md",
                ],
            )],
            None,
        );
        const FIVE_HEADINGS: &str = "## ADDED Requirements\n\n### Requirement: A\nA text.\n\n#### Scenario: a1\n- **WHEN** a\n- **THEN** b\n\n## REMOVED Requirements\n\n### Requirement: B\nB text.\n";
        let recorder = crate::testutil::RecordingReader::new(
            vec![(
                std::path::PathBuf::from(
                    "/repo/openspec/changes/c/specs/degraded-coverage/spec.md",
                ),
                Ok(FIVE_HEADINGS.to_string()),
            )],
            Ok("## MODIFIED Requirements\n".to_string()),
        );
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert_eq!(
            shape_of(&d.detail)[..6],
            [
                (Some("degraded-coverage"), 0),
                (Some("ADDED Requirements"), 1),
                (Some("Requirement: A"), 2),
                (Some("Scenario: a1"), 3),
                (Some("REMOVED Requirements"), 1),
                (Some("Requirement: B"), 2),
            ],
            "the file section and the five headings, in order"
        );
        assert_eq!(
            operation_of(&d.detail)[..6],
            [
                None,
                None,
                Some(crate::specs::DeltaOp::Added),
                None,
                None,
                Some(crate::specs::DeltaOp::Removed),
            ],
            "`Requirement: B` carries `Removed` and not `Added` — the second \
             operation heading reset the walk rather than nesting"
        );
        assert!(
            d.detail.sections[..6].iter().all(|s| s.progress.is_none()),
            "the two derived fields are independent; a spec section is not \
             mistaken for a task group"
        );
    }

    /// `artifact-folds` :: "A requirement above every operation heading
    /// carries none" — a requirement with no preceding operation heading is
    /// unbadged rather than inheriting from one that comes later.
    #[test]
    fn a_requirement_above_every_operation_heading_carries_none() {
        let mut d = dashboard_over(
            &[("specs", &["/repo/openspec/changes/c/specs/a/spec.md"])],
            None,
        );
        const TEXT: &str = "## Purpose\n\nPurpose text.\n\n### Requirement: A\nA text.\n\n## ADDED Requirements\n\n### Requirement: B\nB text.\n";
        let recorder = crate::testutil::RecordingReader::always(Ok(TEXT.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert_eq!(
            shape_of(&d.detail),
            vec![
                (Some("Purpose"), 0),
                (Some("Requirement: A"), 1),
                (Some("ADDED Requirements"), 0),
                (Some("Requirement: B"), 1),
            ]
        );
        assert_eq!(
            operation_of(&d.detail),
            vec![None, None, None, Some(crate::specs::DeltaOp::Added)],
            "`Requirement: A` is unbadged, having no operation heading before it"
        );
    }

    /// `artifact-folds` :: "A main spec's requirements are entirely
    /// unbadged" — `Purpose` and `Requirements` are not operation headings,
    /// so a badge distinguishes a delta spec from a main spec rather than
    /// marking every requirement in the tree.
    #[test]
    fn a_main_specs_requirements_are_entirely_unbadged() {
        let mut d = dashboard_over(
            &[("specs", &["/repo/openspec/changes/c/specs/a/spec.md"])],
            None,
        );
        const MAIN_SPEC: &str = "## Purpose\n\nPurpose text.\n\n## Requirements\n\n### Requirement: A\nA text.\n\n### Requirement: B\nB text.\n";
        let recorder = crate::testutil::RecordingReader::always(Ok(MAIN_SPEC.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert!(
            d.detail.sections.iter().all(|s| s.operation.is_none()),
            "every section of a main spec carries `operation: None`"
        );
    }

    /// `artifact-folds` :: "Only a level-3 `Requirement:` heading is
    /// attributed" — a level-4 `Requirement:` heading and a level-3 heading
    /// merely starting with `Requirements` both decline.
    #[test]
    fn only_a_level_3_requirement_heading_is_attributed() {
        let mut d = dashboard_over(
            &[("specs", &["/repo/openspec/changes/c/specs/a/spec.md"])],
            None,
        );
        const TEXT: &str = "## ADDED Requirements\n\n### Requirement: A\nA text.\n\n### Requirements overview\nOverview text.\n\n#### Requirement: B\nB text.\n\n### Requirement:\nBare text.\n";
        let recorder = crate::testutil::RecordingReader::always(Ok(TEXT.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert_eq!(
            shape_of(&d.detail)[1..],
            [
                (Some("Requirement: A"), 1),
                (Some("Requirements overview"), 1),
                (Some("Requirement: B"), 2),
                (Some("Requirement:"), 1),
            ]
        );
        assert_eq!(
            operation_of(&d.detail)[1..],
            [
                Some(crate::specs::DeltaOp::Added),
                None,
                None,
                Some(crate::specs::DeltaOp::Added),
            ],
            "`Requirements overview` is declined for its label and \
             `Requirement: B` for its level"
        );
    }

    /// `artifact-folds` :: "A non-spec artifact is attributed nothing" — a
    /// `design.md`-shaped file with a level-2 `## ADDED Requirements` written
    /// as prose and no level-3 `Requirement:` heading does not split at all,
    /// so no badge is reachable.
    #[test]
    fn a_non_spec_artifact_is_attributed_nothing() {
        let mut d = dashboard_over(&[("design", &["/repo/openspec/changes/c/design.md"])], None);
        const DESIGN: &str = "# Design\n\n## ADDED Requirements\n\nProse discussing what was added, not a heading structure.\n";
        let recorder = crate::testutil::RecordingReader::always(Ok(DESIGN.to_string()));
        let read = |p: &std::path::Path| recorder.read(p);

        d.sync_detail(&read);

        assert_eq!(
            d.detail.sections.len(),
            1,
            "not spec-shaped, so the file does not split"
        );
        assert_eq!(d.detail.sections[0].operation, None);
    }

    /// `artifact-folds` :: "A tracked-tasks tab's sections carry progress and
    /// no operation" — the badge column and the progress cell never compete
    /// for the same header row.
    #[test]
    fn a_tracked_tasks_tabs_sections_carry_progress_and_no_operation() {
        let d = synced_tracked_tasks_dashboard(
            "## 1. Setup\n\n- [x] 1.1 first\n- [ ] 1.2 second\n\n## 2. Build\n\n- [ ] 2.1 third\n",
            1,
            3,
        );
        assert!(
            operation_of(&d.detail).iter().all(Option::is_none),
            "every section's operation is None on a tracked-tasks tab"
        );
        assert_eq!(
            progress_of(&d.detail),
            vec![progress(1, 2), progress(0, 1)],
            "the group headers' progress is `Some`"
        );
    }

    /// `artifact-folds`: "`Space` on a preamble row is inert".
    ///
    /// No branch of `apply`'s own answers this: `detail_cursor_section` finds
    /// no header row at or before a cursor above the first one, and a
    /// `None`-labelled preamble emits no header row at all (design.md -> D2).
    /// The test exists to hold that property, not a guard.
    #[test]
    fn space_on_a_preamble_row_is_inert() {
        let mut d = task_preamble_dashboard();
        assert_eq!(
            shape_of(&d.detail),
            vec![(None, 0), (Some("1. Setup"), 0), (Some("2. Build"), 0)],
            "three sections, the first of them the unlabelled preamble"
        );

        let rows = crate::ui::detail::content_lines(&d.detail, d.selected_change(), 78);
        let first_header = rows
            .iter()
            .position(|r| matches!(r.kind, crate::ui::detail::ContentKind::SectionHeader { .. }))
            .expect("the fixture is foldable, so it draws header rows");
        // `task-item-bodies`: a `None`-labelled section's body now goes
        // through `ui::tasks::group_body`, which draws that section's own
        // blocks and item bodies beside its items rather than items alone —
        // so the preamble's prose, retained as its leading group's
        // position-0 block, now draws a row of its own: bar, blank, `Intro
        // prose.`, the block's own closing blank, and the fold's separator.
        //
        // Five, not six. A position-0 block opens a section body with
        // nothing above it to be separated from, and the separator rule
        // emits a blank only where one is missing — so the preamble rows
        // are exactly the three the amended scenario names, "the
        // progress-bar row, its blank line, and the row `Intro prose.`
        // itself now draws". An unconditional leading separator put a
        // fourth, empty row above the prose and made that sentence false.
        assert_eq!(
            first_header, 5,
            "the progress-bar row, its blank line, and the preamble's block-wrapped prose \
             precede every header"
        );
        assert!(
            rows[..first_header]
                .iter()
                .any(|r| r.line.text() == "Intro prose."),
            "the preamble's prose is now drawn, as its leading group's own block: {:?}",
            rows[..first_header]
                .iter()
                .map(|r| r.line.text())
                .collect::<Vec<_>>()
        );

        for row in 0..first_header {
            d.detail.scroll = row;
            let before = d.clone();
            for _ in 0..10 {
                d.apply(Action::ToggleSection);
            }
            assert_eq!(
                d, before,
                "row {row}: no field changed, and it did not panic"
            );
        }

        // Moving onto the `1. Setup` header and repeating the action toggles
        // that section, so the inertness was attributable to the row.
        assert_eq!(
            d.detail.expanded,
            std::collections::BTreeSet::from([1, 2]),
            "the seed opened both incomplete groups"
        );
        d.detail.scroll = first_header;
        d.apply(Action::ToggleSection);
        assert_eq!(
            d.detail.expanded,
            std::collections::BTreeSet::from([2]),
            "`1. Setup` folded"
        );
    }

    /// The seven-section spec-glob dashboard `artifact-folds`' ancestor
    /// scenario names: three capability sections at depth `0`, the first of
    /// them carrying a requirement with a nested scenario and a second
    /// requirement, the second carrying one requirement. Every body is a
    /// single word, so no width wraps it and the row list is the same at 78
    /// and at 58.
    fn seven_section_dashboard(expanded: std::collections::BTreeSet<usize>) -> Dashboard {
        let section = |label: &str, depth: usize| ArtifactSection {
            label: Some(label.to_string()),
            text: "one\n".to_string(),
            depth,
            progress: None,
            operation: None,
        };
        dashboard_with_detail(Detail {
            sections: vec![
                section("degraded-coverage", 0),
                section("Requirement: Alpha", 1),
                section("Scenario: A works", 2),
                section("Requirement: Beta", 1),
                section("markdown-render", 0),
                section("Requirement: Gamma", 1),
                section("tasks-checklist", 0),
            ],
            scroll: 0,
            tab: 0,
            problems: Vec::new(),
            loaded: None,
            expanded,
            drawn_width: Some(78),
        })
    }

    /// `artifact-folds`: "Closing an ancestor preserves its subtree's folds".
    ///
    /// The flat encoding is what makes this free (design.md -> D1): a closed
    /// ancestor hides its descendants at render time, and their membership of
    /// `detail.expanded` is never touched.
    #[test]
    fn closing_an_ancestor_preserves_its_subtrees_folds() {
        let mut d = seven_section_dashboard(std::collections::BTreeSet::from([0, 1, 2]));
        // The cursor on the `degraded-coverage` header row, which is row 0.
        d.detail.scroll = 0;
        let before = [(120u16, 40u16), (60, 40)]
            .map(|(width, height)| crate::testutil::render_at(width, height, &d));

        d.apply(Action::ToggleSection);

        assert_eq!(
            d.detail.expanded,
            std::collections::BTreeSet::from([1, 2]),
            "the descendants kept their membership"
        );
        assert_eq!(d.detail.scroll, 0, "the cursor is on the folded header");
        for (width, height) in [(120u16, 40u16), (60, 40)] {
            let interior = if width == 60 { 58 } else { 78 };
            let rows = crate::ui::detail::content_lines(&d.detail, d.selected_change(), interior);
            assert_eq!(
                rows.len(),
                3,
                "width {width}: three collapsed capability headers alone"
            );
            for (row, label) in [
                (0usize, "degraded-coverage"),
                (1, "markdown-render"),
                (2, "tasks-checklist"),
            ] {
                assert_eq!(
                    rows[row].line.text(),
                    expected_header(label, true, interior),
                    "width {width} row {row}"
                );
            }
            let buf = crate::testutil::render_at(width, height, &d);
            for (row, label) in [
                (5u16, "degraded-coverage"),
                (6, "markdown-render"),
                (7, "tasks-checklist"),
            ] {
                assert_eq!(
                    detail_interior_row(&buf, row),
                    expected_header(label, true, interior),
                    "width {width} drawn row {row}"
                );
            }
        }

        d.apply(Action::ToggleSection);

        assert_eq!(
            d.detail.expanded,
            std::collections::BTreeSet::from([0, 1, 2]),
            "reopening the ancestor restores exactly the subtree it had"
        );
        for (index, (width, height)) in [(120u16, 40u16), (60, 40)].into_iter().enumerate() {
            let after = crate::testutil::render_at(width, height, &d);
            for y in 0..height {
                assert_eq!(
                    crate::testutil::row_text(&before[index], y),
                    crate::testutil::row_text(&after, y),
                    "{width}x{height} row {y}: byte-identical to before the two actions"
                );
            }
        }
    }

    /// `detail-scroll`: "At a collapsed foldable tab the same keys walk the
    /// section list". Named `ui::view::tests::…` in design.md's Test
    /// Strategy table, but the analogous existing test for this exact
    /// shape — `the_wheel_moves_the_cursor_at_a_foldable_tab` below — is
    /// itself `ui::app::tests::…` despite being a "view" tier row; this one
    /// follows that precedent instead of the table's literal path.
    #[test]
    fn at_a_collapsed_foldable_tab_the_same_keys_walk_the_section_list() {
        let mut d = foldable_dashboard(std::collections::BTreeSet::new(), 0);
        for (want_scroll, want_row) in [(1usize, 6u16), (2, 7)] {
            d.apply(Action::Next);
            assert_eq!(d.detail.scroll, want_scroll);
            for (width, height) in [(120u16, 40u16), (60, 40)] {
                let buf = crate::testutil::render_at(width, height, &d);
                let interior = if width == 60 { 58 } else { 78 };
                for (row, label) in [
                    (5u16, "degraded-coverage"),
                    (6, "markdown-render"),
                    (7, "tasks-checklist"),
                ] {
                    assert_eq!(
                        detail_interior_row(&buf, row),
                        expected_header(label, true, interior),
                        "width {width} row {row}: still collapsed, three headers stay drawn"
                    );
                }
                let from = if width == 60 { 1 } else { 42 };
                let uncoloured = ratatui::buffer::Cell::default().style();
                assert_eq!(
                    crate::testutil::cell(&buf, from, want_row).style(),
                    uncoloured.patch(crate::ui::palette::style(
                        crate::ui::palette::Role::DetailSectionSelected
                    )),
                    "width {width}: header at row {want_row} carries the selected role"
                );
            }
        }
        d.apply(Action::Next);
        d.normalise_scroll(ratatui::layout::Rect::new(0, 0, 120, 40));
        assert_eq!(
            d.detail.scroll, 2,
            "clamped to the last line, not the last screenful"
        );
    }

    /// `detail-scroll`: "The wheel moves the cursor at a foldable tab".
    #[test]
    fn the_wheel_moves_the_cursor_at_a_foldable_tab() {
        let mut d = foldable_dashboard(std::collections::BTreeSet::new(), 0);
        d.route = Route::List;

        d.apply(Action::ScrollDown);
        d.apply(Action::ScrollDown);
        assert_eq!(d.detail.scroll, 2);
        assert!(d.detail.expanded.is_empty());
        let rows = crate::ui::detail::content_lines(&d.detail, d.selected_change(), 78);
        assert!(matches!(
            rows[2].kind,
            crate::ui::detail::ContentKind::SectionHeader { selected: true, .. }
        ));

        d.apply(Action::ScrollUp);
        d.apply(Action::ScrollUp);
        assert_eq!(d.detail.scroll, 0);
        let rows = crate::ui::detail::content_lines(&d.detail, d.selected_change(), 78);
        assert!(matches!(
            rows[0].kind,
            crate::ui::detail::ContentKind::SectionHeader { selected: true, .. }
        ));
    }

    // `heading-sections`: the heading splitter's own fixtures and its six tests.
    // Every one of them is width-free, which is why the splitter is sited in this
    // file and not in `src/ui/detail.rs` — `DETAILWIDTHS` requires every `#[test]`
    // there to name both `58` and `78` and carries no exemption list (design.md
    // -> Decision 11).

    /// `specs/artifact-folds/spec.md` -> "A delta spec splits into operations,
    /// requirements, and scenarios", verbatim.
    const DELTA_SPEC: &str = "## ADDED Requirements\n\n### Requirement: Alpha\nAlpha text.\n\n#### Scenario: A works\n- **WHEN** a\n- **THEN** b\n\n### Requirement: Beta\nBeta text.\n";

    /// `specs/artifact-folds/spec.md` -> "A task file splits into its groups",
    /// verbatim. Its `Intro prose.` is the preamble, which is no section.
    const TASK_FILE: &str = "Intro prose.\n\n## 1. Setup\n\n- [x] 1.1 first\n- [ ] 1.2 second\n\n## 2. Build\n\n- [ ] 2.1 third\n";

    /// The fence fixture, in its three forms: three backticks, three tildes, and
    /// a four-backtick fence closed by four backticks.
    const FENCED: &str = "## Real\n\n```sh\n# not a heading\nmake check\n```\n\n### Also real\nx\n";
    const TILDE_FENCED: &str =
        "## Real\n\n~~~sh\n# not a heading\nmake check\n~~~\n\n### Also real\nx\n";
    const QUAD_FENCED: &str =
        "## Real\n\n````sh\n# not a heading\nmake check\n````\n\n### Also real\nx\n";

    /// A fence opened with three backticks and closed with three tildes is not
    /// closed: everything after it stays body.
    const MISMATCHED_FENCE: &str = "## Real\n\n```sh\n# not a heading\n~~~\n\n### Also real\nx\n";

    /// `specs/artifact-folds/spec.md` -> "Near-headings are not headings",
    /// verbatim: no space after the run, seven hashes, four spaces of indent,
    /// three spaces of indent (the one real heading), a bare `#`, and a `#`
    /// whose remainder trims to nothing.
    const NEAR_HEADINGS: &str =
        "#Nospace\n####### Seven hashes\n    # Indented four\n   ### Indented three\n#\n# \n";

    /// A file that is one unterminated fence.
    const UNTERMINATED_FENCE: &str = "```sh\n# one\n## two\n";

    /// Reconstruct each section's heading line from its `level` and `label` and
    /// concatenate the result with its `body`. `concat()` rather than the bare
    /// zero-argument `.join()`, which `NOBLOCK` greps this file for (design.md
    /// -> Boundaries).
    fn reassemble(text: &str) -> String {
        let parts: Vec<String> = split_headings(text)
            .iter()
            .map(|s| {
                format!(
                    "{} {}\n{}",
                    "#".repeat(usize::from(s.level)),
                    s.label,
                    s.body
                )
            })
            .collect();
        parts.concat()
    }

    /// The `(level, label)` pairs `split_headings` returns for `text`.
    fn shape(text: &str) -> Vec<(u8, String)> {
        split_headings(text)
            .into_iter()
            .map(|s| (s.level, s.label))
            .collect()
    }

    /// `specs/artifact-folds/spec.md` -> "A delta spec splits into operations,
    /// requirements, and scenarios".
    #[test]
    fn a_delta_spec_splits_into_operations_requirements_and_scenarios() {
        assert_eq!(
            shape(DELTA_SPEC),
            vec![
                (2, "ADDED Requirements".to_string()),
                (3, "Requirement: Alpha".to_string()),
                (4, "Scenario: A works".to_string()),
                (3, "Requirement: Beta".to_string()),
            ]
        );
        let sections = split_headings(DELTA_SPEC);
        assert_eq!(sections[0].body, "\n");
        assert_eq!(sections[1].body, "Alpha text.\n\n");
        assert_eq!(sections[2].body, "- **WHEN** a\n- **THEN** b\n\n");
        assert_eq!(sections[3].body, "Beta text.\n");
        assert_eq!(reassemble(DELTA_SPEC), DELTA_SPEC);
    }

    /// `specs/artifact-folds/spec.md` -> "A task file splits into its groups".
    #[test]
    fn a_task_file_splits_into_its_groups() {
        assert_eq!(
            shape(TASK_FILE),
            vec![(2, "1. Setup".to_string()), (2, "2. Build".to_string())]
        );
        let sections = split_headings(TASK_FILE);
        for section in &sections {
            assert!(
                !section.body.contains("Intro prose."),
                "the preamble precedes the first heading and is no section's body"
            );
        }
        assert!(sections[0].body.contains("- [x] 1.1 first"));
        assert!(sections[0].body.contains("- [ ] 1.2 second"));
        assert!(!sections[0].body.contains("2.1 third"));
        assert!(sections[1].body.contains("- [ ] 2.1 third"));
    }

    /// `specs/artifact-folds/spec.md` -> "A heading inside a fence is body text".
    #[test]
    fn a_heading_inside_a_fence_is_body_text() {
        for input in [FENCED, TILDE_FENCED, QUAD_FENCED] {
            assert_eq!(
                shape(input),
                vec![(2, "Real".to_string()), (3, "Also real".to_string())],
                "fence fixture: {input:?}"
            );
            let sections = split_headings(input);
            let fence = input
                .strip_prefix("## Real\n")
                .and_then(|rest| rest.strip_suffix("### Also real\nx\n"))
                .expect("fixture shape");
            assert_eq!(sections[0].body, fence, "the fence is body, verbatim");
            assert!(sections[0].body.contains("# not a heading"));
            assert_eq!(sections[1].body, "x\n");
        }

        // A fence opened with three backticks is not closed by three tildes.
        let sections = split_headings(MISMATCHED_FENCE);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].label, "Real");
        assert_eq!(
            sections[0].body,
            "\n```sh\n# not a heading\n~~~\n\n### Also real\nx\n"
        );
    }

    /// `specs/artifact-folds/spec.md` -> "Near-headings are not headings".
    #[test]
    fn near_headings_are_not_headings() {
        assert_eq!(
            shape(NEAR_HEADINGS),
            vec![(3, "Indented three".to_string())]
        );
        assert_eq!(split_headings(NEAR_HEADINGS)[0].body, "#\n# \n");
    }

    /// `specs/artifact-folds/spec.md` -> "The splitter is total over degenerate
    /// input".
    #[test]
    fn the_splitter_is_total_over_degenerate_input() {
        assert!(split_headings("").is_empty());
        assert!(split_headings("\n\n\n").is_empty());

        let one = split_headings("## a");
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].level, 2);
        assert_eq!(one[0].label, "a");
        assert_eq!(one[0].body, "");

        // 2,000 CJK code points, each two display columns wide: a 4,000-column
        // label, which the splitter returns whole because it measures no width.
        let wide = "漢".repeat(2000);
        let cjk = split_headings(&format!("### {wide}\n"));
        assert_eq!(cjk.len(), 1);
        assert_eq!(cjk[0].level, 3);
        assert_eq!(cjk[0].label, wide);

        assert!(split_headings(UNTERMINATED_FENCE).is_empty());
    }

    /// `heading-sections` task 1.4: reassembling each section's heading line and
    /// `body` reproduces the input less its preamble, over all five scenario
    /// fixtures and over a file with no trailing newline. Two of them document
    /// where the canonical heading line is not the input's own bytes: a legally
    /// indented heading loses its indent, and a file with no final newline gains
    /// one.
    #[test]
    fn reassembling_the_sections_reproduces_the_input_less_its_preamble() {
        for input in [DELTA_SPEC, FENCED, TILDE_FENCED, QUAD_FENCED] {
            assert_eq!(reassemble(input), input, "no preamble: {input:?}");
        }

        assert_eq!(
            reassemble(TASK_FILE),
            TASK_FILE
                .strip_prefix("Intro prose.\n\n")
                .expect("preamble")
        );

        let tail = NEAR_HEADINGS
            .strip_prefix("#Nospace\n####### Seven hashes\n    # Indented four\n")
            .expect("preamble");
        assert_eq!(reassemble(NEAR_HEADINGS), tail.trim_start_matches(' '));

        assert_eq!(reassemble(""), "");
        assert_eq!(reassemble("\n\n\n"), "");
        assert_eq!(reassemble(UNTERMINATED_FENCE), "");
        assert_eq!(reassemble("## a"), "## a\n");
    }

    /// `specs/artifact-folds/spec.md` -> "A file splits at its headings only
    /// when it is a spec or a tracked task file": the `is_spec_shaped` half.
    /// It asks `split_headings` rather than scanning the text a second time,
    /// which is why a `### Requirement:` inside a fence does not count.
    #[test]
    fn is_spec_shaped_asks_the_splitter_for_a_level_three_requirement() {
        assert!(is_spec_shaped(DELTA_SPEC));
        assert!(!is_spec_shaped(TASK_FILE));
        assert!(!is_spec_shaped(""));
        assert!(!is_spec_shaped(
            "# Doc\n\n```md\n### Requirement: quoted\n```\n"
        ));
        assert!(!is_spec_shaped("## Requirement: too shallow\n"));
        assert!(!is_spec_shaped("#### Requirement: too deep\n"));
        assert!(!is_spec_shaped("### Requirements\n"));
    }

    /// `mouse-input`: the wheel and click actions `Dashboard::apply` gains.
    /// `list-selection`'s ten scenarios live here; `detail-scroll`'s seven live
    /// in `mod scroll` beside it.
    mod click {
        use super::dashboard_for_attribution;
        use crate::changes::fixture;
        use crate::ui::app::{Action, Dashboard, Route, SectionKey, Target};
        use std::collections::BTreeMap;

        /// `active` active changes and `archived` archived ones, cursor on
        /// target `selected`, at `Route::List`.
        pub(super) fn dashboard(active: usize, archived: usize, selected: usize) -> Dashboard {
            let a: Vec<_> = (0..active)
                .map(|i| fixture::active(&format!("a{i}"), 0, 1))
                .collect();
            let z: Vec<_> = (0..archived)
                .map(|i| fixture::archived(Some("2026-01-01"), &format!("z{i}"), 1, 1))
                .collect();
            dashboard_for_attribution(a, z, selected, Vec::new(), BTreeMap::new())
        }

        /// The `targets()` index that addresses `target`.
        fn index_of(d: &Dashboard, target: Target) -> usize {
            d.targets()
                .iter()
                .position(|t| *t == target)
                .unwrap_or_else(|| panic!("{target:?} is not among {:?}", d.targets()))
        }

        #[test]
        fn moves_the_cursor_and_resets_the_tab() {
            let mut d = dashboard(4, 0, 0);
            d.selected = index_of(&d, Target::Change(0));
            d.detail.tab = 3;
            d.detail.scroll = 9;
            let before = d.clone();

            d.apply(Action::Click(Target::Change(2)));

            assert_eq!(d.selected, index_of(&d, Target::Change(2)));
            assert_eq!(d.detail.tab, 0);
            assert_eq!(d.detail.scroll, 0);
            assert_eq!(d.route, before.route);
            assert_eq!(d.filter, before.filter);
            assert_eq!(d.changes, before.changes);
            assert_eq!(d.agents, before.agents);
            assert_eq!(d.agent_names, before.agent_names);
            assert_eq!(d.launch, before.launch);
        }

        #[test]
        fn the_selected_change_resets_nothing() {
            let mut d = dashboard(4, 0, 0);
            d.selected = index_of(&d, Target::Change(2));
            d.detail.tab = 3;
            d.detail.scroll = 9;
            d.route = Route::Detail;
            let before = d.clone();

            d.apply(Action::Click(Target::Change(2)));

            assert_eq!(d.selected, before.selected);
            assert_eq!(d.detail.tab, before.detail.tab);
            assert_eq!(d.detail.scroll, before.detail.scroll);
            assert_eq!(d, before, "the whole dashboard is unchanged");
        }

        #[test]
        fn an_absent_target_changes_nothing() {
            // Three active changes plus their header: four targets.
            let mut d = dashboard(3, 0, 1);
            assert_eq!(d.targets().len(), 4);
            let before = d.clone();
            d.apply(Action::Click(Target::Change(7)));
            assert_eq!(d, before);

            let mut no_archive = dashboard(3, 0, 1);
            let before = no_archive.clone();
            no_archive.apply(Action::Click(Target::Section(SectionKey::Archived)));
            assert_eq!(no_archive, before);
        }

        #[test]
        fn a_collapsed_section_hides_its_rows_when_clicked() {
            let mut d = dashboard(3, 3, 1);
            let archived_target = Target::Change(3);
            assert!(d.targets().contains(&archived_target), "open, it is drawn");

            d.sections.collapsed.insert(SectionKey::Archived);
            let before = d.clone();
            d.apply(Action::Click(archived_target));
            assert_eq!(d, before, "a collapsed section hides its rows from a click");

            d.sections.collapsed.remove(&SectionKey::Archived);
            d.apply(Action::Click(archived_target));
            assert_eq!(d.selected, index_of(&d, archived_target));
        }

        #[test]
        fn click_and_space_produce_equal_dashboards() {
            for key in [SectionKey::Active, SectionKey::Archived] {
                let mut clicked = dashboard(3, 3, 0);
                let mut spaced = dashboard(3, 3, 0);

                clicked.apply(Action::Click(Target::Section(key)));
                spaced.selected = index_of(&spaced, Target::Section(key));
                spaced.apply(Action::ToggleSection);
                assert_eq!(clicked, spaced, "{key:?}: the fold");

                clicked.apply(Action::Click(Target::Section(key)));
                spaced.selected = index_of(&spaced, Target::Section(key));
                spaced.apply(Action::ToggleSection);
                assert_eq!(clicked, spaced, "{key:?}: the unfold");
            }
        }

        #[test]
        fn clicking_open_an_unresolved_archive_requests_a_refresh() {
            let mut d = dashboard(3, 0, 0);
            d.changes.archived_total = 22;
            d.sections.collapsed.insert(SectionKey::Archived);
            assert!(d.changes.archived.is_empty());

            d.apply(Action::Click(Target::Section(SectionKey::Archived)));
            assert!(!d.sections.collapsed.contains(&SectionKey::Archived));
            assert_eq!(
                d.selected,
                index_of(&d, Target::Section(SectionKey::Archived))
            );
            assert!(d.refresh.requested);

            // The fold does not set it a second time.
            d.refresh.requested = false;
            d.apply(Action::Click(Target::Section(SectionKey::Archived)));
            assert!(d.sections.collapsed.contains(&SectionKey::Archived));
            assert!(!d.refresh.requested);
        }

        #[test]
        fn an_undrawn_header_is_inert() {
            let mut d = dashboard(3, 0, 1);
            let before = d.clone();
            d.apply(Action::Click(Target::Section(SectionKey::Archived)));
            assert_eq!(d, before);
            assert_eq!(d.sections.collapsed, before.sections.collapsed);
        }

        #[test]
        fn the_second_click_opens_and_the_third_does_nothing() {
            // The rule is the same above and below the breakpoint; `apply` sees
            // no width at all, which is what makes that structural.
            let mut d = dashboard(3, 0, 0);
            let target = Target::Change(1);

            d.apply(Action::Click(target));
            assert_eq!(d.selected, index_of(&d, target));
            assert_eq!(d.route, Route::List);

            d.apply(Action::Click(target));
            assert_eq!(d.route, Route::Detail);
            assert_eq!(d.detail.scroll, 0);

            let after_second = d.clone();
            d.apply(Action::Click(target));
            assert_eq!(d, after_second);
        }

        #[test]
        fn the_second_click_matches_enter() {
            let mut clicked = dashboard(3, 0, 0);
            clicked.selected = index_of(&clicked, Target::Change(1));
            let mut entered = clicked.clone();

            clicked.apply(Action::Click(Target::Change(1)));
            entered.apply(Action::OpenDetail);
            assert_eq!(clicked, entered);
        }

        #[test]
        fn two_header_clicks_fold_and_unfold() {
            let mut d = dashboard(3, 3, 0);
            d.apply(Action::Click(Target::Section(SectionKey::Active)));
            assert!(d.sections.collapsed.contains(&SectionKey::Active));
            assert_eq!(d.route, Route::List);

            d.apply(Action::Click(Target::Section(SectionKey::Active)));
            assert!(!d.sections.collapsed.contains(&SectionKey::Active));
            assert_eq!(d.route, Route::List);
        }
    }

    /// `mouse-input`: the four region-explicit wheel actions. `detail-scroll`'s
    /// seven scenarios.
    mod scroll {
        use crate::ui::app::{Action, ArtifactSection, Dashboard, Route, Target};

        use super::click::dashboard;

        /// A twenty-line markdown list — one rendered line per source line.
        fn lines(n: usize) -> String {
            (0..n).map(|i| format!("- line-{i:02}\n")).collect()
        }

        /// The `targets()` index that addresses `target`.
        fn index_of(d: &Dashboard, target: Target) -> usize {
            d.targets()
                .iter()
                .position(|t| *t == target)
                .expect("target is drawn")
        }

        #[test]
        fn scroll_down_scrolls_at_the_list_route() {
            let mut d = dashboard(3, 0, 0);
            d.selected = index_of(&d, Target::Change(0));
            d.detail.sections = vec![ArtifactSection {
                label: Some(String::new()),
                text: lines(40),
                depth: 0,
                progress: None,
                operation: None,
            }];
            let selected_before = d.selected;

            for _ in 0..3 {
                d.apply(Action::ScrollDown);
            }
            assert_eq!(d.detail.scroll, 3);
            assert_eq!(d.selected, selected_before);
            assert_eq!(d.route, Route::List);

            // The drawn detail region shows the content advanced by three lines
            // while the list region still shows the same selected row. The
            // content area's first row is buffer row 5 (the region's heading
            // row and its padding row sit two rows above the interior, and the
            // tab bar, the rule, and the interior's own padding row take three
            // more), and the interior's first column is 42, one gutter past the
            // divider at column 40.
            let buffer = crate::testutil::render_at(120, 40, &d);
            let content: String = (42..120)
                .map(|x| buffer[(x, 5)].symbol().to_string())
                .collect();
            assert!(
                content.starts_with("• line-03"),
                "the first content row is {content:?}"
            );
            assert_eq!(buffer[(1, 3)].symbol(), ">", "the selected row is unmoved");
        }

        #[test]
        fn scroll_up_stops_at_the_top() {
            for route in [Route::List, Route::Detail] {
                let mut d = dashboard(3, 0, 1);
                d.route = route;
                d.detail.sections = vec![ArtifactSection {
                    label: Some(String::new()),
                    text: lines(40),
                    depth: 0,
                    progress: None,
                    operation: None,
                }];
                let before = d.clone();
                d.apply(Action::ScrollUp);
                assert_eq!(d.detail.scroll, 0);
                assert_eq!(d, before, "{route:?}: nothing else changes either");
            }
        }

        #[test]
        fn a_held_wheel_is_clamped_by_the_frame() {
            let mut wheeled = dashboard(3, 0, 0);
            wheeled.selected = index_of(&wheeled, Target::Change(0));
            wheeled.detail.sections = vec![ArtifactSection {
                label: Some(String::new()),
                text: lines(12),
                depth: 0,
                progress: None,
                operation: None,
            }];
            for _ in 0..500 {
                wheeled.apply(Action::ScrollDown);
            }
            assert_eq!(wheeled.detail.scroll, 500, "`apply` does not clamp");

            let area = ratatui::layout::Rect::new(0, 0, 120, 40);
            let buffer = crate::testutil::render_at(120, 40, &wheeled);
            // The content area's first row is buffer row 5 and its first column is
            // 42, per the same geometry `scroll_down_scrolls_at_the_list_route`
            // documents.
            let first: String = (42..120)
                .map(|x| buffer[(x, 5)].symbol().to_string())
                .collect();
            assert!(
                first.starts_with("• line-00"),
                "twelve lines fit the 34-row content area whole, so the last screenful \
                 starts at line 0 rather than leaving a blank region: {first:?}"
            );

            wheeled.normalise_scroll(area);
            // The same value a held `j` at the detail route leaves behind.
            let mut held = dashboard(3, 0, 0);
            held.selected = index_of(&held, Target::Change(0));
            held.detail.sections = vec![ArtifactSection {
                label: Some(String::new()),
                text: lines(12),
                depth: 0,
                progress: None,
                operation: None,
            }];
            held.route = Route::Detail;
            for _ in 0..500 {
                held.apply(Action::Next);
            }
            held.normalise_scroll(area);
            assert_eq!(wheeled.detail.scroll, held.detail.scroll);
        }

        #[test]
        fn next_at_detail_equals_scroll_down() {
            let mut keyed = dashboard(3, 0, 1);
            keyed.route = Route::Detail;
            keyed.detail.sections = vec![ArtifactSection {
                label: Some(String::new()),
                text: lines(40),
                depth: 0,
                progress: None,
                operation: None,
            }];
            let mut wheeled = keyed.clone();
            keyed.apply(Action::Next);
            wheeled.apply(Action::ScrollDown);
            assert_eq!(keyed, wheeled);

            keyed.detail.scroll = 5;
            wheeled.detail.scroll = 5;
            keyed.apply(Action::Prev);
            wheeled.apply(Action::ScrollUp);
            assert_eq!(keyed, wheeled);

            // The same holds at a foldable tab, for both pairs.
            let mut keyed_f = super::foldable_dashboard(std::collections::BTreeSet::new(), 0);
            let mut wheeled_f = keyed_f.clone();
            keyed_f.apply(Action::Next);
            wheeled_f.apply(Action::ScrollDown);
            assert_eq!(keyed_f, wheeled_f);

            keyed_f.apply(Action::Prev);
            wheeled_f.apply(Action::ScrollUp);
            assert_eq!(keyed_f, wheeled_f);
        }

        #[test]
        fn select_next_moves_at_the_detail_route() {
            let mut d = dashboard(6, 0, 0);
            d.selected = index_of(&d, Target::Change(0));
            d.route = Route::Detail;
            d.detail.tab = 2;
            d.detail.scroll = 9;
            let before = d.selected;

            d.apply(Action::SelectNext);

            assert_eq!(d.selected, before + 1);
            assert_eq!(d.detail.tab, 0);
            assert_eq!(d.detail.scroll, 0);
            assert_eq!(d.route, Route::Detail);
        }

        #[test]
        fn a_clamped_select_resets_nothing() {
            let mut last = dashboard(6, 0, 0);
            last.selected = last.targets().len() - 1;
            last.detail.tab = 2;
            last.detail.scroll = 9;
            let before = last.clone();
            last.apply(Action::SelectNext);
            assert_eq!(last, before);

            let mut first = dashboard(6, 0, 0);
            first.selected = 0;
            first.detail.tab = 2;
            first.detail.scroll = 9;
            let before = first.clone();
            first.apply(Action::SelectPrev);
            assert_eq!(first, before);
        }

        #[test]
        fn next_at_list_equals_select_next() {
            let mut keyed = dashboard(6, 0, 1);
            keyed.detail.tab = 2;
            keyed.detail.scroll = 9;
            let mut wheeled = keyed.clone();
            keyed.apply(Action::Next);
            wheeled.apply(Action::SelectNext);
            assert_eq!(keyed, wheeled);

            keyed.apply(Action::Prev);
            wheeled.apply(Action::SelectPrev);
            assert_eq!(keyed, wheeled);
        }
    }

    mod keys {
        use ratatui::crossterm::event::{
            Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
        };

        use crate::changes::empty_set;
        use crate::changes::fixture;
        use crate::testutil::RecordingReader;
        use crate::ui::app::{
            Action, ArtifactSection, Dashboard, Detail, Edit, Filter, Granularity, Overlay, Panel,
            Route, SectionKey, Sections, SelectPhase, Selection, Target, action_for,
        };

        fn empty_filter() -> Filter {
            Filter {
                query: String::new(),
                active: false,
            }
        }

        fn empty_detail() -> Detail {
            Detail {
                sections: Vec::new(),
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
                expanded: std::collections::BTreeSet::new(),
                drawn_width: None,
            }
        }

        fn dashboard_at(route: Route) -> Dashboard {
            Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                repo: None,
                searched_from: std::path::PathBuf::from("/tmp/does-not-matter"),
                changes: empty_set(),
                route,
                quit: false,
                selected: 0,
                filter: empty_filter(),
                detail: empty_detail(),
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            }
        }

        /// Three active changes, two archived — the fixture `list.rs` and
        /// `view.rs`'s tests reuse verbatim.
        fn five_change_dashboard() -> Dashboard {
            Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
                searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
                changes: fixture::set(
                    vec![
                        fixture::active("add-token-refresh", 4, 9),
                        fixture::active("fix-empty-basket", 7, 7),
                        fixture::active("migrate-ai-sdk-v7", 0, 0),
                    ],
                    vec![
                        fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                        fixture::archived(None, "legacy-cleanup", 3, 3),
                    ],
                    Vec::new(),
                ),
                route: Route::List,
                quit: false,
                selected: 0,
                filter: empty_filter(),
                detail: empty_detail(),
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            }
        }

        fn press(code: KeyCode, modifiers: KeyModifiers) -> Event {
            Event::Key(KeyEvent::new(code, modifiers))
        }

        /// A `Route::Detail` dashboard whose one active, selected change
        /// carries `count` artifacts named `a0`, `a1`, ... — `artifact-tabs`'
        /// key-handling scenarios need a change with a known artifact count,
        /// not the row-grammar fixture's empty `artifacts`.
        fn dashboard_with_artifacts(count: usize, tab: usize, scroll: usize) -> Dashboard {
            let pairs: Vec<(&str, &[&str])> = (0..count).map(|_| ("a", &[][..])).collect();
            let change = fixture::with_artifacts(fixture::active("detail-view", 4, 9), &pairs);
            Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
                searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
                changes: fixture::set(vec![change], Vec::new(), Vec::new()),
                route: Route::Detail,
                quit: false,
                // `list-sections`: 1, not 0 — target 0 is the active header.
                selected: 1,
                filter: empty_filter(),
                detail: Detail {
                    sections: Vec::new(),
                    scroll,
                    tab,
                    problems: Vec::new(),
                    loaded: None,
                    expanded: std::collections::BTreeSet::new(),
                    drawn_width: None,
                },
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            }
        }

        #[test]
        fn matches_is_case_insensitive_substring_on_the_name() {
            use crate::ui::app::matches;
            assert!(matches("add-token-refresh", "add"));
            assert!(matches("ADD-TOKEN-REFRESH", "add"));
            assert!(matches("add-token-refresh", "ADD"));
            assert!(matches("add-token-refresh", ""));
            assert!(!matches("fix-empty-basket", "add"));
        }

        #[test]
        fn visible_applies_the_query_to_both_tiers() {
            let mut d = five_change_dashboard();

            d.filter.query = "add".to_string();
            let names: Vec<&str> = d.visible().iter().map(|c| c.name.as_str()).collect();
            assert_eq!(names, vec!["add-token-refresh", "add-auth"]);

            d.filter.query = String::new();
            let names: Vec<&str> = d.visible().iter().map(|c| c.name.as_str()).collect();
            assert_eq!(
                names,
                vec![
                    "add-token-refresh",
                    "fix-empty-basket",
                    "migrate-ai-sdk-v7",
                    "add-auth",
                    "legacy-cleanup",
                ]
            );

            d.filter.query = "zzz".to_string();
            assert!(d.visible().is_empty());
        }

        #[test]
        fn quit_keys_and_their_near_misses() {
            assert_eq!(
                action_for(&press(KeyCode::Char('q'), KeyModifiers::NONE), false),
                Action::Quit
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('c'), KeyModifiers::CONTROL), false),
                Action::Quit
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('Q'), KeyModifiers::SHIFT), false),
                Action::Ignore
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('q'), KeyModifiers::CONTROL), false),
                Action::Ignore
            );
            assert_eq!(
                // `agent-launch`: bare `c` is now `LaunchContinue` where it was `Ignore` —
                // `Ctrl-C` (asserted above) is the one that still quits.
                action_for(&press(KeyCode::Char('c'), KeyModifiers::NONE), false),
                Action::LaunchContinue
            );

            assert_eq!(
                action_for(&press(KeyCode::Char('q'), KeyModifiers::NONE), true),
                Action::FilterPush('q')
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('c'), KeyModifiers::CONTROL), true),
                Action::Quit
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('Q'), KeyModifiers::SHIFT), true),
                Action::FilterPush('Q')
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('q'), KeyModifiers::CONTROL), true),
                Action::Ignore
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('c'), KeyModifiers::NONE), true),
                Action::FilterPush('c')
            );
        }

        #[test]
        fn r_maps_to_refresh_outside_filter_mode() {
            // `live-refresh` -> "r forces a full refresh and types itself
            // while filtering".
            assert_eq!(
                action_for(&press(KeyCode::Char('r'), KeyModifiers::NONE), false),
                Action::Refresh
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('r'), KeyModifiers::NONE), true),
                Action::FilterPush('r')
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('r'), KeyModifiers::CONTROL), false),
                Action::Ignore
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('r'), KeyModifiers::CONTROL), true),
                Action::Ignore
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('R'), KeyModifiers::SHIFT), false),
                Action::Ignore
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('R'), KeyModifiers::SHIFT), true),
                Action::FilterPush('R')
            );

            // A terminal reporting releases and repeats must not refresh
            // twice for one press, under either mode.
            let released = Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('r'),
                KeyModifiers::NONE,
                KeyEventKind::Release,
            ));
            let repeated = Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('r'),
                KeyModifiers::NONE,
                KeyEventKind::Repeat,
            ));
            for filtering in [false, true] {
                assert_eq!(action_for(&released, filtering), Action::Ignore);
                assert_eq!(action_for(&repeated, filtering), Action::Ignore);
            }
        }

        #[test]
        fn only_press_kind_acts() {
            let released = Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('q'),
                KeyModifiers::NONE,
                KeyEventKind::Release,
            ));
            let repeated = Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('q'),
                KeyModifiers::NONE,
                KeyEventKind::Repeat,
            ));
            let pressed = Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('q'),
                KeyModifiers::NONE,
                KeyEventKind::Press,
            ));
            for filtering in [false, true] {
                assert_eq!(action_for(&released, filtering), Action::Ignore);
                assert_eq!(action_for(&repeated, filtering), Action::Ignore);
            }
            assert_eq!(action_for(&pressed, false), Action::Quit);
            assert_eq!(action_for(&pressed, true), Action::FilterPush('q'));

            // `live-refresh`: a released or repeated `r` must not quit,
            // refresh, or type — it must be ignored under either mode, the
            // same as the landed `q` case above.
            let r_released = Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('r'),
                KeyModifiers::NONE,
                KeyEventKind::Release,
            ));
            let r_repeated = Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('r'),
                KeyModifiers::NONE,
                KeyEventKind::Repeat,
            ));
            for filtering in [false, true] {
                assert_eq!(action_for(&r_released, filtering), Action::Ignore);
                assert_eq!(action_for(&r_repeated, filtering), Action::Ignore);
            }
        }

        #[test]
        fn enter_and_esc_map_to_routes() {
            assert_eq!(
                action_for(&press(KeyCode::Enter, KeyModifiers::NONE), false),
                Action::OpenDetail
            );
            assert_eq!(
                action_for(&press(KeyCode::Esc, KeyModifiers::NONE), false),
                Action::Back
            );
            assert_eq!(
                action_for(&press(KeyCode::Enter, KeyModifiers::CONTROL), false),
                Action::Ignore
            );

            // detail-scroll: "Enter and Esc move between the two routes" —
            // both route moves reset detail.scroll, whether or not it was
            // ever nonzero.
            let mut d = dashboard_at(Route::List);
            d.apply(Action::OpenDetail);
            assert_eq!(d.route, Route::Detail);
            assert!(!d.quit);
            assert_eq!(d.detail.scroll, 0);
            assert!(
                !d.refresh.requested,
                "a route move must not touch the refresh flag"
            );
            d.apply(Action::Back);
            assert_eq!(d.route, Route::List);
            assert!(!d.quit);
            assert_eq!(d.detail.scroll, 0);
            assert!(!d.refresh.requested);
            d.apply(Action::Back);
            assert_eq!(d.route, Route::List);
            assert!(!d.quit);
            assert_eq!(d.detail.scroll, 0);
            assert!(!d.refresh.requested);
        }

        #[test]
        fn non_key_events_are_ignored() {
            for filtering in [false, true] {
                assert_eq!(
                    action_for(&Event::Resize(60, 20), filtering),
                    Action::Ignore
                );
                assert_eq!(action_for(&Event::FocusGained, filtering), Action::Ignore);
                assert_eq!(action_for(&Event::FocusLost, filtering), Action::Ignore);
                assert_eq!(
                    action_for(&Event::Paste("q".to_string()), filtering),
                    Action::Ignore,
                    "a paste of the single character q must not quit or type, filtering={filtering}"
                );
                assert_eq!(
                    action_for(&Event::Paste("r".to_string()), filtering),
                    Action::Ignore,
                    "a paste of the single character r must not refresh or type, filtering={filtering}"
                );
                // `dashboard-loop` names these two explicitly, with their own AND
                // clauses — "a paste whose text is `1` does not switch tabs" and one
                // whose text is `a` "does not launch an agent". Added by
                // `mouse-input`'s Change Review: the scenario has asserted them since
                // `agent-launch` and the test never did.
                assert_eq!(
                    action_for(&Event::Paste("1".to_string()), filtering),
                    Action::Ignore,
                    "a paste of the single character 1 must not switch tabs, filtering={filtering}"
                );
                assert_eq!(
                    action_for(&Event::Paste("a".to_string()), filtering),
                    Action::Ignore,
                    "a paste of the single character a must not launch an agent, \
                     filtering={filtering}"
                );
                let mouse = Event::Mouse(MouseEvent {
                    kind: MouseEventKind::Moved,
                    column: 0,
                    row: 0,
                    modifiers: KeyModifiers::NONE,
                });
                assert_eq!(action_for(&mouse, filtering), Action::Ignore);
            }
        }

        #[test]
        fn action_for_is_total_over_a_keycode_sweep() {
            let mapped = [
                KeyCode::Enter,
                KeyCode::Esc,
                KeyCode::Char('q'),
                KeyCode::Up,
                KeyCode::Down,
                KeyCode::Char('/'),
                // detail-view: `1`-`9`, `[`, and `]` now map to `SelectTab`,
                // `PrevTab`, and `NextTab` rather than `Ignore`.
                KeyCode::Char('1'),
                KeyCode::Char('['),
                KeyCode::Char(']'),
                // `agent-launch`: `a` now maps to `LaunchApply` rather than `Ignore`.
                KeyCode::Char('a'),
            ];
            let codes = [
                KeyCode::Backspace,
                KeyCode::Left,
                KeyCode::Right,
                KeyCode::Up,
                KeyCode::Down,
                KeyCode::Home,
                KeyCode::End,
                KeyCode::PageUp,
                KeyCode::PageDown,
                KeyCode::Tab,
                KeyCode::BackTab,
                KeyCode::Delete,
                KeyCode::Insert,
                KeyCode::F(1),
                KeyCode::F(12),
                KeyCode::Char('a'),
                KeyCode::Char('1'),
                KeyCode::Char('/'),
                KeyCode::Char('['),
                KeyCode::Char(']'),
                KeyCode::Null,
            ];
            assert!(codes.len() >= 20, "sweep must cover at least twenty codes");
            for code in codes {
                let action = action_for(&press(code, KeyModifiers::NONE), false);
                if !mapped.contains(&code) {
                    assert_eq!(action, Action::Ignore, "{code:?} should be ignored");
                }
            }

            // Under filtering: true, every Char maps to FilterPush and every
            // non-Char outside the filter-mode table (Backspace, Up, Down —
            // Enter and Esc are covered above) maps to Ignore.
            let filter_table_non_char = [KeyCode::Backspace, KeyCode::Up, KeyCode::Down];
            for code in codes {
                let action = action_for(&press(code, KeyModifiers::NONE), true);
                match code {
                    KeyCode::Char(c) => assert_eq!(action, Action::FilterPush(c)),
                    other if filter_table_non_char.contains(&other) => {}
                    other => assert_eq!(
                        action,
                        Action::Ignore,
                        "{other:?} should be ignored while filtering"
                    ),
                }
            }
        }

        /// `tasks-checklist` -> "No action mutates a task item": every
        /// `Action` variant, applied to a `Dashboard` whose selected
        /// change carries a marked tracked-tasks artifact, leaves
        /// `changes` untouched. Rendering a checkbox makes toggling one
        /// look natural, which is exactly why this test exists.
        #[test]
        fn no_action_mutates_changes() {
            // An exhaustive match with no wildcard arm: a new `Action`
            // variant fails to compile here, which is the signal to add
            // it to `variants` below too — an enumerate-by-hand test
            // would silently miss it instead.
            fn assert_known_variant(a: &Action) {
                match a {
                    Action::Quit
                    | Action::OpenDetail
                    | Action::Back
                    | Action::Next
                    | Action::Prev
                    | Action::SelectTab(_)
                    | Action::NextTab
                    | Action::PrevTab
                    | Action::FilterStart
                    | Action::FilterPush(_)
                    | Action::FilterPop
                    | Action::Refresh
                    | Action::LaunchApply
                    | Action::LaunchContinue
                    | Action::LaunchArchive
                    | Action::FocusAgent
                    | Action::ToggleSection
                    | Action::ToggleHelp
                    | Action::SelectNext
                    | Action::SelectPrev
                    | Action::ScrollDown
                    | Action::ScrollUp
                    | Action::Click(_)
                    | Action::Select(_)
                    | Action::ToggleSettings
                    | Action::Ignore => {}
                }
            }

            let variants = [
                Action::Quit,
                Action::OpenDetail,
                Action::Back,
                Action::Next,
                Action::Prev,
                Action::SelectTab(0),
                Action::NextTab,
                Action::PrevTab,
                Action::FilterStart,
                Action::FilterPush('a'),
                Action::FilterPop,
                Action::Refresh,
                // `agent-launch`'s four, bumping the count from thirteen to seventeen — the
                // number moved deliberately, as this test's own subject, in the same commit
                // that gave the four launch actions their `apply` arms.
                Action::LaunchApply,
                Action::LaunchContinue,
                Action::LaunchArchive,
                Action::FocusAgent,
                // `list-sections`'s addition, bumping the count from seventeen to eighteen.
                Action::ToggleSection,
                // `mouse-input`'s five, bumping the count from eighteen to twenty-three.
                // `ui::driver::mouse_action` is their only producer — `action_for` maps no
                // key to any of them — so this array and `apply`'s exhaustive match are the
                // only two places that enumerate them, and they must agree.
                Action::SelectNext,
                Action::SelectPrev,
                Action::ScrollDown,
                Action::ScrollUp,
                Action::Click(crate::ui::app::Target::Change(0)),
                // `help-overlay`'s addition, bumping the count from twenty-three to
                // twenty-four. `apply`'s arm for it is still a no-op at this group's end —
                // group 3 gives it the overlay's real dispatch — so it trivially leaves
                // `changes` untouched here.
                Action::ToggleHelp,
                // `text-selection`'s addition, bumping the count from twenty-four to
                // twenty-five. `apply_select` only ever writes `self.selection`, never
                // `self.changes`, so this leaves `changes` untouched here too.
                Action::Select(SelectPhase::Begin { line: 0, column: 0 }),
                // `settings-window`'s addition, bumping the count from twenty-five to
                // twenty-six. `apply_settings_action`/`apply_help_action` only ever write
                // `self.overlay`/`self.settings`, never `self.changes`, so this leaves
                // `changes` untouched here too.
                Action::ToggleSettings,
                Action::Ignore,
            ];
            assert_eq!(
                variants.len(),
                26,
                "the twenty-six variants this crate specifies"
            );
            for v in &variants {
                assert_known_variant(v);
            }

            let change = crate::changes::fixture::track_tasks_at(
                crate::changes::fixture::with_artifacts(
                    crate::changes::fixture::active("x", 2, 5),
                    &[("tasks", &["/repo/tasks.md"])],
                ),
                0,
            );
            let dashboard = Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                detail: Detail {
                    sections: vec![ArtifactSection {
                        label: Some(String::new()),
                        text: "## 1. Setup\n- [x] a\n- [ ] b\n".to_string(),
                        depth: 0,
                        progress: None,
                        operation: None,
                    }],
                    scroll: 0,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
                    expanded: std::collections::BTreeSet::new(),
                    drawn_width: None,
                },
                // `agent-launch`: a real repository root and one in-scope, named agent, so
                // `attribution()` populates `panes` and the four launch arms genuinely reach
                // `launch::decide` — `g` finds a pane to focus, and `a`/`c`/`s` find the same
                // name already live and are refused, both real decisions rather than the
                // vacuous `Nothing` an unreachable socket or an empty agent list would produce.
                repo: Some(std::path::PathBuf::from("/repo")),
                searched_from: std::path::PathBuf::from("/tmp/does-not-matter"),
                changes: crate::changes::fixture::set(vec![change], Vec::new(), Vec::new()),
                route: Route::Detail,
                quit: false,
                // `list-sections`: 1, not 0 — target 0 is the active header.
                selected: 1,
                filter: empty_filter(),
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: vec![crate::agents::Agent {
                        name: Some("x".to_string()),
                        kind: None,
                        status: crate::agents::AgentStatus::Working,
                        cwd: Some(std::path::PathBuf::from("/repo")),
                        pane_id: "p1".to_string(),
                        tab_id: "t1".to_string(),
                        workspace_id: "w1".to_string(),
                        terminal_title: None,
                    }],
                    reachable: true,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            };

            for action in variants {
                let mut d = dashboard.clone();
                d.apply(action);
                assert_eq!(d.changes, dashboard.changes, "{action:?} mutated changes");
                if matches!(
                    action,
                    Action::LaunchApply
                        | Action::LaunchContinue
                        | Action::LaunchArchive
                        | Action::FocusAgent
                ) {
                    assert!(
                        d.launch.pending.is_some() || !d.launch.problems.is_empty(),
                        "{action:?} must reach the decision rather than satisfy the equality \
                         above vacuously through an unreachable socket"
                    );
                }
            }
        }

        #[test]
        fn navigation_and_filter_keys_are_distinguished() {
            assert_eq!(
                action_for(&press(KeyCode::Char('j'), KeyModifiers::NONE), false),
                Action::Next
            );
            assert_eq!(
                action_for(&press(KeyCode::Down, KeyModifiers::NONE), false),
                Action::Next
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('k'), KeyModifiers::NONE), false),
                Action::Prev
            );
            assert_eq!(
                action_for(&press(KeyCode::Up, KeyModifiers::NONE), false),
                Action::Prev
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('/'), KeyModifiers::NONE), false),
                Action::FilterStart
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('r'), KeyModifiers::NONE), false),
                Action::Refresh
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('R'), KeyModifiers::SHIFT), false),
                Action::Ignore
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('r'), KeyModifiers::CONTROL), false),
                Action::Ignore
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('J'), KeyModifiers::SHIFT), false),
                Action::Ignore
            );
            assert_eq!(
                action_for(&press(KeyCode::Down, KeyModifiers::CONTROL), false),
                Action::Ignore
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('/'), KeyModifiers::CONTROL), false),
                Action::Ignore
            );

            // While filtering, Up/Down still navigate (only Char keys type
            // into the query) — found in Change Review, whose planted
            // mutation (deleting both arms from the filter-mode table) left
            // the suite green without this pair.
            assert_eq!(
                action_for(&press(KeyCode::Up, KeyModifiers::NONE), true),
                Action::Prev
            );
            assert_eq!(
                action_for(&press(KeyCode::Down, KeyModifiers::NONE), true),
                Action::Next
            );
        }

        #[test]
        fn slash_starts_filter_mode_and_routes_to_list() {
            let mut d = dashboard_at(Route::Detail);
            let action = action_for(&press(KeyCode::Char('/'), KeyModifiers::NONE), false);
            assert_eq!(action, Action::FilterStart);
            d.apply(action);
            assert!(d.filter.active);
            assert_eq!(d.filter.query, "");
            assert_eq!(d.route, Route::List);
        }

        #[test]
        fn filter_mode_types_printable_characters() {
            let mut d = dashboard_at(Route::List);
            d.apply(Action::FilterStart);
            for c in ['q', 'j', '/'] {
                let action = action_for(&press(KeyCode::Char(c), KeyModifiers::NONE), true);
                d.apply(action);
            }
            assert_eq!(d.filter.query, "qj/");
            assert!(!d.quit);
        }

        #[test]
        fn ctrl_c_quits_from_filter_mode() {
            let mut d = dashboard_at(Route::List);
            d.apply(Action::FilterStart);
            let action = action_for(&press(KeyCode::Char('c'), KeyModifiers::CONTROL), true);
            assert_eq!(action, Action::Quit);
            d.apply(action);
            assert!(d.quit);
        }

        #[test]
        fn backspace_deletes_and_is_inert_on_empty() {
            let mut d = dashboard_at(Route::List);
            d.filter.active = true;
            d.filter.query = "ad".to_string();
            d.apply(Action::FilterPop);
            assert_eq!(d.filter.query, "a");
            assert!(d.filter.active);
            d.apply(Action::FilterPop);
            assert_eq!(d.filter.query, "");
            assert!(d.filter.active);
            d.apply(Action::FilterPop);
            assert_eq!(d.filter.query, "");
            assert!(d.filter.active);
        }

        #[test]
        fn enter_accepts_the_filter_without_opening_detail() {
            let mut d = dashboard_at(Route::List);
            d.filter.active = true;
            d.filter.query = "add".to_string();
            d.apply(Action::OpenDetail);
            assert!(!d.filter.active);
            assert_eq!(d.filter.query, "add");
            assert_eq!(d.route, Route::List);
        }

        #[test]
        fn esc_dismisses_one_layer_at_a_time() {
            let mut d = dashboard_at(Route::Detail);
            d.filter.active = true;
            d.filter.query = "add".to_string();
            d.detail.scroll = 3;

            d.apply(Action::Back);
            assert!(!d.filter.active);
            assert_eq!(d.filter.query, "");
            // Exactly one layer was dismissed: the route is untouched by
            // this first `Back` — found in Change Review, whose planted
            // mutation (unconditionally resetting the route on every
            // `Back`) left the suite green without this assertion.
            assert_eq!(d.route, Route::Detail);
            assert!(!d.quit);
            // detail-scroll: dismissing the filter layer is not a route
            // move, so the scroll survives it.
            assert_eq!(d.detail.scroll, 3);

            d.apply(Action::Back);
            assert_eq!(d.route, Route::List);
            assert!(!d.quit);
            // The second `Back` dismisses the route layer, which does
            // reset the scroll.
            assert_eq!(d.detail.scroll, 0);

            d.apply(Action::Back);
            assert_eq!(d.route, Route::List);
            assert!(!d.quit);

            d.apply(Action::Back);
            assert_eq!(d.route, Route::List);
            assert!(!d.quit);

            let mut d2 = dashboard_at(Route::List);
            d2.filter.active = false;
            d2.filter.query = "add".to_string();
            d2.apply(Action::Back);
            assert_eq!(d2.filter.query, "");
            assert!(!d2.filter.active);
            d2.apply(Action::Back);
            assert_eq!(d2.filter.query, "");
            assert_eq!(d2.route, Route::List);
        }

        /// Six active changes, `agents.reachable` `true` — `help-overlay`'s own
        /// fixture for the scenarios that need a real selected change under the
        /// overlay, reusing `super::dashboard_for_attribution` rather than a new
        /// literal.
        fn six_active_changes() -> Vec<crate::changes::Change> {
            (0..6)
                .map(|i| fixture::active(&format!("a{i}"), 0, 1))
                .collect()
        }

        /// The eighteen actions `help-overlay`'s spec names as inert while the
        /// overlay is open, in the order the dashboard-loop scenario applies
        /// them.
        fn eighteen_inert_actions() -> [Action; 18] {
            [
                Action::OpenDetail,
                Action::SelectTab(3),
                Action::NextTab,
                Action::PrevTab,
                Action::FilterStart,
                Action::FilterPush('a'),
                Action::FilterPop,
                Action::Refresh,
                Action::LaunchApply,
                Action::LaunchContinue,
                Action::LaunchArchive,
                Action::FocusAgent,
                Action::ToggleSection,
                Action::SelectNext,
                Action::SelectPrev,
                Action::Click(Target::Change(0)),
                // `text-selection`'s addition, bumping the count from seventeen to
                // eighteen.
                Action::Select(SelectPhase::Begin { line: 0, column: 0 }),
                Action::Ignore,
            ]
        }

        /// `help-overlay` -> "The overlay opens and closes without moving the
        /// route".
        #[test]
        fn overlay_opens_and_closes_without_moving_the_route() {
            let mut d = dashboard_at(Route::Detail);
            d.detail.tab = 2;
            d.detail.scroll = 7;
            d.selected = 3;
            let before = d.clone();

            d.apply(Action::ToggleHelp);
            assert!(d.overlay.panel.is_some());
            assert_eq!(d.overlay.scroll, 0);
            assert_eq!(d.route, Route::Detail);
            assert_eq!(d.detail.tab, 2);
            assert_eq!(d.detail.scroll, 7);
            assert_eq!(d.selected, 3);

            d.apply(Action::ToggleHelp);
            assert!(d.overlay.panel.is_none());
            assert_eq!(d.overlay.scroll, 0);
            assert_eq!(d.route, Route::Detail);
            assert_eq!(d.detail.tab, 2);
            assert_eq!(d.detail.scroll, 7);
            assert_eq!(d.selected, 3);

            assert_eq!(d.changes, before.changes);
            assert_eq!(d.filter, before.filter);
            assert_eq!(d.quit, before.quit);
            assert_eq!(d.refresh, before.refresh);
            assert_eq!(d.agents, before.agents);
            assert_eq!(d.agent_names, before.agent_names);
            assert_eq!(d.launch, before.launch);
            assert_eq!(d.sections, before.sections);
        }

        /// `help-overlay` -> "`Esc` closes the overlay before any other layer".
        #[test]
        fn esc_closes_the_overlay_before_any_other_layer() {
            let mut d = dashboard_at(Route::Detail);
            d.filter.active = true;
            d.filter.query = "add".to_string();
            d.overlay.panel = Some(Panel::Help);

            d.apply(Action::Back);
            assert!(
                d.overlay.panel.is_none(),
                "the overlay is the outermost layer"
            );
            assert!(
                d.filter.active,
                "dismissing the overlay is not a filter move"
            );
            assert_eq!(d.filter.query, "add");
            assert!(!d.quit);

            d.apply(Action::Back);
            assert!(!d.filter.active);
            assert_eq!(d.filter.query, "");
            assert!(!d.quit);

            d.apply(Action::Back);
            assert_eq!(d.route, Route::List);
            assert!(!d.quit);

            let before = d.clone();
            d.apply(Action::Back);
            assert_eq!(d, before, "the fourth Back changes nothing");
            assert!(!d.quit);
        }

        /// `dashboard-loop` -> "The overlay layer suppresses every action but
        /// eight".
        #[test]
        fn the_overlay_layer_suppresses_every_action_but_eight() {
            let mut open = super::dashboard_for_attribution(
                six_active_changes(),
                Vec::new(),
                2,
                Vec::new(),
                std::collections::BTreeMap::new(),
            );
            open.detail.tab = 1;
            open.overlay.panel = Some(Panel::Help);
            let before = open.clone();
            for action in eighteen_inert_actions() {
                open.apply(action);
            }
            assert_eq!(open.changes, before.changes);
            assert_eq!(open.route, before.route);
            assert_eq!(open.selected, before.selected);
            assert_eq!(open.detail, before.detail);
            assert_eq!(open.filter, before.filter);
            assert_eq!(open.quit, before.quit);
            assert_eq!(open.agents, before.agents);
            assert_eq!(open.agent_names, before.agent_names);
            assert_eq!(open.sections, before.sections);
            assert_eq!(open.overlay, before.overlay);
            assert_eq!(
                open.selection, before.selection,
                "`Select` is one of the eighteen — the overlay is closed to it too"
            );
            assert_eq!(open.launch.pending, None);
            assert!(open.launch.problems.is_empty());
            if before.needs_archived_refresh() {
                assert!(open.refresh.requested);
            } else {
                assert_eq!(open.refresh.requested, before.refresh.requested);
            }

            // The same eighteen actions, applied to the identical dashboard
            // with the overlay closed, do change it — so the suppression above
            // is the overlay's own and not a property of the dashboard.
            let mut closed = super::dashboard_for_attribution(
                six_active_changes(),
                Vec::new(),
                2,
                Vec::new(),
                std::collections::BTreeMap::new(),
            );
            closed.detail.tab = 1;
            let before_closed = closed.clone();
            for action in eighteen_inert_actions() {
                closed.apply(action);
            }
            assert_ne!(closed, before_closed);
        }

        /// `dashboard-loop` -> "The overlay's eight live actions act and
        /// nothing else moves".
        #[test]
        fn the_overlays_eight_live_actions_act_and_nothing_else_moves() {
            let mut d = dashboard_at(Route::Detail);
            d.detail.scroll = 4;
            d.selected = 1;
            d.overlay.panel = Some(Panel::Help);

            d.apply(Action::Next);
            assert_eq!(d.overlay.scroll, 1);
            d.apply(Action::ScrollDown);
            assert_eq!(d.overlay.scroll, 2);
            d.apply(Action::Prev);
            assert_eq!(d.overlay.scroll, 1);
            d.apply(Action::ScrollUp);
            assert_eq!(d.overlay.scroll, 0);
            d.apply(Action::Prev);
            assert_eq!(d.overlay.scroll, 0, "saturates rather than underflowing");

            assert_eq!(d.detail.scroll, 4);
            assert_eq!(d.selected, 1);

            // An eighth, `ToggleSettings`, applied to the same dashboard (before `Back`
            // closes it) — `settings-window`'s addition to this layer, and the reason
            // the count is eight rather than seven.
            let mut swapped = d.clone();
            swapped.apply(Action::ToggleSettings);
            assert_eq!(swapped.overlay.panel, Some(Panel::Settings));
            assert_eq!(swapped.overlay.scroll, 0);

            d.apply(Action::Back);
            assert!(d.overlay.panel.is_none());
            assert_eq!(d.overlay.scroll, 0);

            d.apply(Action::Quit);
            assert!(d.quit);
        }

        /// `help-overlay` -> "Both quit keys still quit from inside the
        /// overlay".
        #[test]
        fn both_quit_keys_still_quit_from_inside_the_overlay() {
            let mut d = dashboard_at(Route::List);
            d.overlay.panel = Some(Panel::Help);
            d.apply(Action::Quit);
            assert!(d.quit);

            let mut d2 = dashboard_at(Route::List);
            d2.overlay.panel = Some(Panel::Help);
            d2.filter.active = true;
            d2.apply(Action::Quit);
            assert!(d2.quit, "no combination of layers traps the reader");
        }

        /// `help-overlay` -> "The agent keys launch nothing while the overlay
        /// is open".
        #[test]
        fn the_agent_keys_launch_nothing_while_the_overlay_is_open() {
            let mut d = super::dashboard_for_attribution(
                vec![fixture::active("add-auth", 1, 2)],
                Vec::new(),
                // `list-sections`: 1, not 0 — target 0 is the active header.
                1,
                Vec::new(),
                std::collections::BTreeMap::new(),
            );
            d.overlay.panel = Some(Panel::Help);
            let before = d.clone();

            for action in [
                Action::LaunchApply,
                Action::LaunchContinue,
                Action::LaunchArchive,
                Action::FocusAgent,
                Action::Refresh,
            ] {
                d.apply(action);
            }

            assert_eq!(d.launch.pending, None);
            assert!(d.launch.problems.is_empty());
            if before.needs_archived_refresh() {
                assert!(d.refresh.requested);
            } else {
                assert!(!d.refresh.requested);
            }
            assert_eq!(d.changes, before.changes);
            assert_eq!(d.route, before.route);
            assert_eq!(d.selected, before.selected);
            assert_eq!(d.detail, before.detail);
            assert_eq!(d.filter, before.filter);
            assert_eq!(d.quit, before.quit);
            assert_eq!(d.agents, before.agents);
            assert_eq!(d.agent_names, before.agent_names);
            assert_eq!(d.sections, before.sections);
            assert_eq!(d.overlay, before.overlay);
        }

        /// `help-overlay` -> "The overlay swallows every inert action" — the
        /// scenario's own title dropped its number when `settings-window` moved
        /// `ToggleSettings` from the (would-be) inert set to the answered one,
        /// leaving the inert eighteen unchanged member for member.
        #[test]
        fn the_overlay_swallows_every_inert_action() {
            let mut d = super::dashboard_for_attribution(
                six_active_changes(),
                Vec::new(),
                2,
                Vec::new(),
                std::collections::BTreeMap::new(),
            );
            d.detail.tab = 1;
            d.overlay.panel = Some(Panel::Help);
            let sections_before = d.sections.clone();

            for action in [
                Action::OpenDetail,
                Action::SelectTab(3),
                Action::NextTab,
                Action::PrevTab,
                Action::FilterStart,
                Action::FilterPush('a'),
                Action::FilterPop,
                Action::ToggleSection,
                Action::SelectNext,
                Action::SelectPrev,
                Action::Click(Target::Change(0)),
            ] {
                d.apply(action);
            }

            assert_eq!(d.route, Route::List);
            assert_eq!(d.selected, 2);
            assert_eq!(d.detail.tab, 1);
            assert_eq!(d.filter.query, "");
            assert!(!d.filter.active);
            assert_eq!(d.sections, sections_before);
            assert!(d.overlay.panel.is_some());
            assert_eq!(d.overlay.scroll, 0);
        }

        /// `help-overlay` -> "`j` and `k` scroll the overlay rather than the
        /// frame beneath".
        #[test]
        fn j_and_k_scroll_the_overlay_rather_than_the_frame_beneath() {
            let mut d = dashboard_at(Route::Detail);
            d.detail.scroll = 4;
            d.selected = 1;
            d.overlay.panel = Some(Panel::Help);

            d.apply(Action::Next);
            d.apply(Action::Next);
            d.apply(Action::Next);
            assert_eq!(d.overlay.scroll, 3);
            d.apply(Action::Prev);
            assert_eq!(d.overlay.scroll, 2);

            assert_eq!(
                d.detail.scroll, 4,
                "the overlay's scroll is not the detail region's"
            );
            assert_eq!(d.selected, 1);

            let mut d2 = dashboard_at(Route::List);
            d2.detail.scroll = 4;
            d2.selected = 1;
            d2.overlay.panel = Some(Panel::Help);
            d2.apply(Action::Next);
            d2.apply(Action::Next);
            d2.apply(Action::Next);
            d2.apply(Action::Prev);
            assert_eq!(
                d2.overlay.scroll, 2,
                "route-agnostic where Next and Prev are not"
            );
            assert_eq!(d2.selected, 1);

            let mut d3 = dashboard_at(Route::Detail);
            d3.overlay.panel = Some(Panel::Help);
            d3.apply(Action::Prev);
            assert_eq!(d3.overlay.scroll, 0, "saturates rather than underflowing");
        }

        // `settings-window`: three fixture rows — `openspec_bin`, `agent_kind`,
        // `prompts` — for tests that need `Dashboard::settings.rows` populated
        // without depending on `settings::settings`'s own precedence rules.
        fn three_settings() -> Vec<crate::settings::Setting> {
            ["openspec_bin", "agent_kind", "prompts"]
                .iter()
                .map(|key| crate::settings::Setting {
                    key,
                    value: format!("{key}-value"),
                    provenance: crate::settings::Provenance::Default,
                    editable: crate::settings::Editable::No {
                        reason: crate::settings::Reason::SetOnce,
                    },
                })
                .collect()
        }

        /// `settings-window` -> "`,` toggles the panel and its near misses do
        /// not", `dashboard-loop` -> "`,` maps to `ToggleSettings` and moves no
        /// existing key".
        #[test]
        fn comma_toggles_the_panel_and_its_near_misses_do_not() {
            assert_eq!(
                action_for(&press(KeyCode::Char(','), KeyModifiers::NONE), false),
                Action::ToggleSettings
            );
            for modifiers in [KeyModifiers::SHIFT, KeyModifiers::CONTROL] {
                assert_eq!(
                    action_for(&press(KeyCode::Char(','), modifiers), false),
                    Action::Ignore,
                    "{modifiers:?} is a near miss"
                );
            }
            assert_eq!(
                action_for(&press(KeyCode::Char('<'), KeyModifiers::SHIFT), false),
                Action::Ignore,
                "`<` is not `,`"
            );
            let released = Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char(','),
                KeyModifiers::NONE,
                KeyEventKind::Release,
            ));
            let repeated = Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char(','),
                KeyModifiers::NONE,
                KeyEventKind::Repeat,
            ));
            assert_eq!(action_for(&released, false), Action::Ignore);
            assert_eq!(action_for(&repeated, false), Action::Ignore);

            assert_eq!(
                action_for(&press(KeyCode::Char(','), KeyModifiers::NONE), true),
                Action::FilterPush(','),
                "filtering types the key rather than opening the panel"
            );
        }

        /// `help-overlay` -> "`?` and `,` swap panels rather than stacking
        /// them".
        #[test]
        fn question_mark_and_comma_swap_panels_rather_than_stacking_them() {
            let mut d = dashboard_at(Route::Detail);
            d.overlay.panel = Some(Panel::Help);
            d.overlay.scroll = 6;

            d.apply(Action::ToggleSettings);
            assert_eq!(d.overlay.panel, Some(Panel::Settings));
            assert_eq!(d.overlay.scroll, 0);

            d.apply(Action::ToggleHelp);
            assert_eq!(d.overlay.panel, Some(Panel::Help));
            assert_eq!(d.overlay.scroll, 0);

            d.apply(Action::ToggleHelp);
            assert_eq!(d.overlay.panel, None);
        }

        /// `help-overlay` -> "`,` swaps to the settings panel from inside the
        /// help".
        #[test]
        fn comma_swaps_to_the_settings_panel_from_inside_the_help() {
            let mut d = dashboard_at(Route::Detail);
            d.overlay.panel = Some(Panel::Help);
            d.overlay.scroll = 9;
            let before = d.clone();

            d.apply(Action::ToggleSettings);
            assert_eq!(d.overlay.panel, Some(Panel::Settings));
            assert_eq!(d.overlay.scroll, 0);
            assert_eq!(d.route, before.route);
            assert_eq!(d.selected, before.selected);
            assert_eq!(d.detail, before.detail);
            assert_eq!(d.filter, before.filter);
            assert_eq!(d.sections, before.sections);
        }

        /// `settings-window` -> "The panels swap rather than stacking".
        #[test]
        fn the_panels_swap_rather_than_stacking() {
            let mut d = dashboard_at(Route::Detail);
            d.overlay.panel = Some(Panel::Help);
            d.overlay.scroll = 5;
            let before = d.clone();

            d.apply(Action::ToggleSettings);
            assert_eq!(d.overlay.panel, Some(Panel::Settings));
            assert_eq!(d.overlay.scroll, 0);

            d.apply(Action::ToggleHelp);
            assert_eq!(d.overlay.panel, Some(Panel::Help));
            assert_eq!(d.overlay.scroll, 0);

            d.apply(Action::ToggleSettings);
            d.apply(Action::ToggleSettings);
            assert_eq!(d.overlay.panel, None);

            assert_eq!(d.route, Route::Detail);
            assert_eq!(d.changes, before.changes);
            assert_eq!(d.filter, before.filter);
            assert_eq!(d.quit, before.quit);
            assert_eq!(d.refresh, before.refresh);
            assert_eq!(d.agents, before.agents);
            assert_eq!(d.agent_names, before.agent_names);
            assert_eq!(d.launch, before.launch);
            assert_eq!(d.selected, before.selected);
            assert_eq!(d.detail, before.detail);
            assert_eq!(d.sections, before.sections);
        }

        /// `settings-window` -> "`Esc` closes the settings panel before any
        /// other layer".
        #[test]
        fn esc_closes_the_settings_panel_before_any_other_layer() {
            let mut d = dashboard_at(Route::Detail);
            d.filter.active = true;
            d.filter.query = "add".to_string();
            d.overlay.panel = Some(Panel::Settings);

            d.apply(Action::Back);
            assert!(d.overlay.panel.is_none());
            assert!(d.filter.active, "dismissing the panel is not a filter move");
            assert_eq!(d.filter.query, "add");
            assert!(!d.quit);

            d.apply(Action::Back);
            assert!(!d.filter.active);
            assert_eq!(d.filter.query, "");
            assert!(!d.quit);

            d.apply(Action::Back);
            assert_eq!(d.route, Route::List);
            assert!(!d.quit);

            let before = d.clone();
            d.apply(Action::Back);
            assert_eq!(d, before, "the fourth Back changes nothing");
            assert!(!d.quit);
        }

        /// `settings-window` -> "The agent keys launch nothing from inside the
        /// settings panel".
        #[test]
        fn the_agent_keys_launch_nothing_from_inside_the_settings_panel() {
            let mut d = super::dashboard_for_attribution(
                vec![fixture::active("add-auth", 1, 2)],
                Vec::new(),
                // `list-sections`: 1, not 0 — target 0 is the active header.
                1,
                Vec::new(),
                std::collections::BTreeMap::new(),
            );
            d.overlay.panel = Some(Panel::Settings);
            let before = d.clone();

            for action in [
                Action::LaunchApply,
                Action::LaunchContinue,
                Action::LaunchArchive,
                Action::FocusAgent,
                Action::Refresh,
            ] {
                d.apply(action);
            }

            assert_eq!(d.launch.pending, None);
            assert_eq!(d.launch.problems, before.launch.problems);
            if before.needs_archived_refresh() {
                assert!(d.refresh.requested);
            } else {
                assert!(!d.refresh.requested);
            }
            assert_eq!(d.changes, before.changes);
            assert_eq!(d.route, before.route);
            assert_eq!(d.selected, before.selected);
            assert_eq!(d.detail, before.detail);
            assert_eq!(d.filter, before.filter);
            assert_eq!(d.quit, before.quit);
            assert_eq!(d.agents, before.agents);
            assert_eq!(d.agent_names, before.agent_names);
            assert_eq!(d.sections, before.sections);
            assert_eq!(d.overlay, before.overlay);
        }

        /// `settings-window` -> "The settings panel swallows every inert
        /// action".
        #[test]
        fn the_settings_panel_swallows_every_inert_action() {
            let mut d = super::dashboard_for_attribution(
                six_active_changes(),
                Vec::new(),
                2,
                Vec::new(),
                std::collections::BTreeMap::new(),
            );
            d.detail.tab = 1;
            d.overlay.panel = Some(Panel::Settings);
            d.settings.rows = three_settings();
            let sections_before = d.sections.clone();

            for action in [
                Action::SelectTab(3),
                Action::NextTab,
                Action::PrevTab,
                Action::FilterStart,
                Action::FilterPush('a'),
                Action::FilterPop,
                Action::ToggleSection,
                Action::SelectNext,
                Action::SelectPrev,
                Action::Select(SelectPhase::Begin { line: 0, column: 0 }),
                Action::Click(Target::Change(0)),
            ] {
                d.apply(action);
            }

            assert_eq!(d.route, Route::List);
            assert_eq!(d.selected, 2);
            assert_eq!(d.detail.tab, 1);
            assert_eq!(d.filter.query, "");
            assert!(!d.filter.active);
            assert_eq!(d.sections, sections_before);
            assert_eq!(d.overlay.panel, Some(Panel::Settings));
            assert_eq!(d.settings.cursor, 0, "the row cursor has not moved");
        }

        /// `settings-window` -> "Both quit keys still quit from inside the
        /// settings panel".
        #[test]
        fn both_quit_keys_still_quit_from_inside_the_settings_panel() {
            let mut d = dashboard_at(Route::List);
            d.overlay.panel = Some(Panel::Settings);
            d.apply(Action::Quit);
            assert!(d.quit);

            let mut d2 = dashboard_at(Route::List);
            d2.overlay.panel = Some(Panel::Settings);
            d2.filter.active = true;
            d2.apply(Action::Quit);
            assert!(d2.quit, "no combination of layers traps the reader");
        }

        /// `settings-window` -> "The cursor walks settings, not rendered rows,
        /// and saturates".
        #[test]
        fn the_cursor_walks_settings_not_rendered_rows_and_saturates() {
            let mut d = dashboard_at(Route::Detail);
            d.detail.scroll = 4;
            d.selected = 1;
            d.overlay.panel = Some(Panel::Settings);
            d.settings.rows = three_settings();
            d.settings.cursor = 0;

            d.apply(Action::Next);
            assert_eq!(d.settings.cursor, 1);
            d.apply(Action::Next);
            assert_eq!(d.settings.cursor, 2);
            d.apply(Action::Next);
            assert_eq!(d.settings.cursor, 2, "saturates at the last setting");
            d.apply(Action::Prev);
            assert_eq!(d.settings.cursor, 1);

            assert_eq!(d.detail.scroll, 4);
            assert_eq!(d.selected, 1);

            let mut zero = dashboard_at(Route::Detail);
            zero.overlay.panel = Some(Panel::Settings);
            zero.settings.rows = three_settings();
            zero.apply(Action::Prev);
            assert_eq!(
                zero.settings.cursor, 0,
                "saturates rather than underflowing"
            );
        }

        /// `settings-window`'s own three fixture rows, with `agent_kind` (row `1`)
        /// made `Editable::Kind` with the given committed value and shortlist —
        /// `three_settings()`'s own shape, for the group 5 edit tests that need an
        /// editable `agent_kind` row rather than three uniformly read-only ones.
        fn settings_with_agent_kind(
            value: &str,
            shortlist: Vec<&str>,
        ) -> Vec<crate::settings::Setting> {
            vec![
                crate::settings::Setting {
                    key: "openspec_bin",
                    value: "openspec_bin-value".to_string(),
                    provenance: crate::settings::Provenance::Default,
                    editable: crate::settings::Editable::No {
                        reason: crate::settings::Reason::SetOnce,
                    },
                },
                crate::settings::Setting {
                    key: "agent_kind",
                    value: value.to_string(),
                    provenance: crate::settings::Provenance::SoleIntegration,
                    editable: crate::settings::Editable::Kind {
                        shortlist: shortlist.iter().map(|s| s.to_string()).collect(),
                    },
                },
                crate::settings::Setting {
                    key: "prompts",
                    value: "prompts-value".to_string(),
                    provenance: crate::settings::Provenance::Default,
                    editable: crate::settings::Editable::No {
                        reason: crate::settings::Reason::SetOnce,
                    },
                },
            ]
        }

        /// `three_settings()`'s own shape, with `agent_kind` (row `1`) refused for
        /// `reason` rather than editable — for the group 5 tests that need a
        /// refused `agent_kind` row (`Reason::Configured` or
        /// `Reason::NoIntegration`).
        fn settings_with_agent_kind_refused(
            reason: crate::settings::Reason,
        ) -> Vec<crate::settings::Setting> {
            let provenance = if reason == crate::settings::Reason::Configured {
                crate::settings::Provenance::Configured
            } else {
                crate::settings::Provenance::LastResort
            };
            vec![
                crate::settings::Setting {
                    key: "openspec_bin",
                    value: "openspec_bin-value".to_string(),
                    provenance: crate::settings::Provenance::Default,
                    editable: crate::settings::Editable::No {
                        reason: crate::settings::Reason::SetOnce,
                    },
                },
                crate::settings::Setting {
                    key: "agent_kind",
                    value: "the-last-resort".to_string(),
                    provenance,
                    editable: crate::settings::Editable::No { reason },
                },
                crate::settings::Setting {
                    key: "prompts",
                    value: "prompts-value".to_string(),
                    provenance: crate::settings::Provenance::Default,
                    editable: crate::settings::Editable::No {
                        reason: crate::settings::Reason::SetOnce,
                    },
                },
            ]
        }

        /// `settings-window` -> "An edit begins, changes a candidate, and
        /// commits" (the in-memory half — writing `settings.toml` and calling
        /// `Launcher::set_kind` are later groups' work; see design.md -> Test
        /// Strategy).
        #[test]
        fn an_edit_begins_changes_a_candidate_and_commits() {
            let mut d = dashboard_at(Route::Detail);
            d.overlay.panel = Some(Panel::Settings);
            d.settings.rows = settings_with_agent_kind("claude", vec!["claude", "codex"]);
            d.settings.cursor = 1;

            d.apply(Action::OpenDetail);
            assert_eq!(
                d.overlay.edit,
                Some(Edit {
                    setting: 1,
                    candidate: 0
                }),
                "the edit begins on the committed kind's own position in the shortlist"
            );

            d.apply(Action::Next);
            assert_eq!(
                d.overlay.edit,
                Some(Edit {
                    setting: 1,
                    candidate: 1
                })
            );
            assert_eq!(
                d.settings.rows[1].value, "claude",
                "the committed value is untouched while the edit is in progress"
            );

            d.apply(Action::OpenDetail);
            assert!(d.overlay.edit.is_none(), "the third `OpenDetail` commits");
            assert_eq!(
                d.settings.rows[1].value, "codex",
                "the candidate becomes the committed value"
            );
            assert_eq!(d.settings.cursor, 1, "the cursor stays on agent_kind");
        }

        /// `settings-window` -> "`Esc` cancels the edit and a second `Esc`
        /// closes the panel".
        #[test]
        fn esc_cancels_the_edit_and_a_second_esc_closes_the_panel() {
            let mut d = dashboard_at(Route::Detail);
            d.overlay.panel = Some(Panel::Settings);
            d.settings.rows = settings_with_agent_kind("claude", vec!["claude", "codex"]);
            d.settings.cursor = 1;

            d.apply(Action::OpenDetail);
            d.apply(Action::Next);
            assert_eq!(
                d.overlay.edit,
                Some(Edit {
                    setting: 1,
                    candidate: 1
                })
            );

            d.apply(Action::Back);
            assert!(
                d.overlay.edit.is_none(),
                "the third `Back` cancels the edit"
            );
            assert_eq!(
                d.settings.rows[1].value, "claude",
                "a cancelled edit writes nothing to the committed value"
            );
            assert_eq!(
                d.overlay.panel,
                Some(Panel::Settings),
                "the panel is still open"
            );

            d.apply(Action::Back);
            assert_eq!(d.overlay.panel, None, "the fourth `Back` closes the panel");
        }

        /// `settings-window` -> "`Enter` on a read-only setting begins no
        /// edit".
        #[test]
        fn enter_on_a_read_only_setting_begins_no_edit() {
            let mut d = dashboard_at(Route::Detail);
            d.overlay.panel = Some(Panel::Settings);
            d.settings.rows = settings_with_agent_kind("claude", vec!["claude", "codex"]);

            for cursor in [0usize, 2usize] {
                d.settings.cursor = cursor;
                let before = d.clone();
                d.apply(Action::OpenDetail);
                assert_eq!(
                    d, before,
                    "row {cursor} is read-only, so OpenDetail changes nothing"
                );
            }
        }

        /// `settings-window` -> "The shortlist is the installed kinds, in
        /// Herdr's order, and wraps".
        #[test]
        fn the_shortlist_is_the_installed_kinds_in_herdrs_order_and_wraps() {
            let mut d = dashboard_at(Route::Detail);
            d.overlay.panel = Some(Panel::Settings);
            d.settings.rows = settings_with_agent_kind("codex", vec!["claude", "codex", "cursor"]);
            d.settings.cursor = 1;

            d.apply(Action::OpenDetail);
            assert_eq!(
                d.overlay.edit,
                Some(Edit {
                    setting: 1,
                    candidate: 1
                }),
                "the candidate begins at codex"
            );

            d.apply(Action::Next);
            d.apply(Action::Next);
            assert_eq!(
                d.overlay.edit,
                Some(Edit {
                    setting: 1,
                    candidate: 0
                }),
                "Next, Next moves to cursor then wraps to claude"
            );

            d.apply(Action::Prev);
            assert_eq!(
                d.overlay.edit,
                Some(Edit {
                    setting: 1,
                    candidate: 2
                }),
                "Prev from claude wraps to cursor"
            );
        }

        /// `settings-window` -> "A committed kind outside the shortlist starts
        /// the edit at the first entry".
        #[test]
        fn a_committed_kind_outside_the_shortlist_starts_the_edit_at_the_first_entry() {
            let mut d = dashboard_at(Route::Detail);
            d.overlay.panel = Some(Panel::Settings);
            d.settings.rows = settings_with_agent_kind("aider", vec!["claude", "codex"]);
            d.settings.cursor = 1;

            d.apply(Action::OpenDetail);
            assert_eq!(
                d.overlay.edit,
                Some(Edit {
                    setting: 1,
                    candidate: 0
                }),
                "the candidate begins at the first entry, Herdr's own order"
            );
            assert_eq!(
                d.settings.rows[1].value, "aider",
                "the committed value is unchanged by beginning an edit"
            );
        }

        /// `settings-window` -> "No installed integration makes the row
        /// non-editable with a reason".
        #[test]
        fn no_installed_integration_makes_the_row_non_editable_with_a_reason() {
            let mut d = dashboard_at(Route::Detail);
            d.overlay.panel = Some(Panel::Settings);
            d.settings.rows =
                settings_with_agent_kind_refused(crate::settings::Reason::NoIntegration);
            d.settings.cursor = 1;

            d.apply(Action::OpenDetail);
            assert!(
                d.overlay.edit.is_none(),
                "no installed integration begins no edit"
            );

            d.apply(Action::Back);
            assert_eq!(
                d.overlay.panel, None,
                "the panel is still open beforehand and `Back` still closes it"
            );
        }

        /// `settings-window` -> "A configured `agent_kind` refuses the edit
        /// and names the file".
        #[test]
        fn a_configured_agent_kind_refuses_the_edit_and_names_the_file() {
            let mut d = dashboard_at(Route::Detail);
            d.overlay.panel = Some(Panel::Settings);
            d.settings.rows = settings_with_agent_kind_refused(crate::settings::Reason::Configured);
            d.settings.cursor = 1;

            d.apply(Action::OpenDetail);
            assert!(
                d.overlay.edit.is_none(),
                "a configured agent_kind refuses the edit"
            );
        }

        /// `settings-window` -> "The refusal is per setting, not per panel".
        #[test]
        fn the_refusal_is_per_setting_not_per_panel() {
            let mut d = dashboard_at(Route::Detail);
            d.overlay.panel = Some(Panel::Settings);
            d.settings.rows = vec![
                crate::settings::Setting {
                    key: "openspec_bin",
                    value: "/usr/bin/openspec".to_string(),
                    provenance: crate::settings::Provenance::Configured,
                    editable: crate::settings::Editable::No {
                        reason: crate::settings::Reason::Configured,
                    },
                },
                crate::settings::Setting {
                    key: "agent_kind",
                    value: "claude".to_string(),
                    provenance: crate::settings::Provenance::SoleIntegration,
                    editable: crate::settings::Editable::Kind {
                        shortlist: vec!["claude".to_string(), "codex".to_string()],
                    },
                },
                crate::settings::Setting {
                    key: "prompts",
                    value: "prompts-value".to_string(),
                    provenance: crate::settings::Provenance::Default,
                    editable: crate::settings::Editable::No {
                        reason: crate::settings::Reason::SetOnce,
                    },
                },
            ];

            d.settings.cursor = 0;
            d.apply(Action::OpenDetail);
            assert!(
                d.overlay.edit.is_none(),
                "openspec_bin is owned by config.toml and begins no edit"
            );

            d.settings.cursor = 1;
            d.apply(Action::OpenDetail);
            assert_eq!(
                d.overlay.edit,
                Some(Edit {
                    setting: 1,
                    candidate: 0
                }),
                "agent_kind still begins one; one refused setting does not freeze the panel"
            );
        }

        /// `dashboard-loop` -> "Both panels open is unrepresentable".
        #[test]
        fn both_panels_open_is_unrepresentable() {
            let mut d = dashboard_at(Route::List);
            assert_eq!(d.overlay.panel, None);
            d.overlay.panel = Some(Panel::Help);
            assert_eq!(d.overlay.panel, Some(Panel::Help));
            d.overlay.panel = Some(Panel::Settings);
            assert_eq!(d.overlay.panel, Some(Panel::Settings));
            // `Option<Panel>` has exactly these three values: there is no
            // fourth to construct in which both panels are open.
        }

        /// `dashboard-loop` -> "`Dashboard` carries seventeen fields after the
        /// overlay is generalised".
        #[test]
        fn dashboard_carries_seventeen_fields_after_the_overlay_is_generalised() {
            let d = dashboard_at(Route::List);
            let Dashboard {
                repo: _,
                searched_from: _,
                changes: _,
                route: _,
                quit: _,
                selected: _,
                filter: _,
                detail: _,
                refresh: _,
                agents: _,
                agent_names: _,
                launch: _,
                sections: _,
                file_mode: _,
                overlay,
                selection: _,
                settings: _,
            } = &d;

            let Overlay {
                panel,
                scroll: _,
                edit: _,
            } = overlay;

            // An exhaustive match with no wildcard arm: a third `Panel` variant
            // added later fails to compile here rather than falling into a
            // wrong branch.
            fn assert_known_panel(p: &Panel) {
                match p {
                    Panel::Help | Panel::Settings => {}
                }
            }
            if let Some(p) = panel {
                assert_known_panel(p);
            }
            assert_known_panel(&Panel::Help);
            assert_known_panel(&Panel::Settings);
        }

        #[test]
        fn next_and_prev_clamp() {
            let mut d = five_change_dashboard();
            // `list-sections`: clear the total too, or the archived tier reads as
            // unresolved-but-nonempty and keeps its header — this test wants a
            // dashboard with no archived section at all.
            d.changes.archived.clear();
            d.changes.archived_total = 0;
            for _ in 0..4 {
                d.apply(Action::Next);
            }
            // `list-sections`: re-indexed — targets are the active header (0)
            // then the three active changes (1..=3), so the clamp lands on 3,
            // not the change-only index 2 this test asserted before.
            assert_eq!(d.selected, 3);
            for _ in 0..4 {
                d.apply(Action::Prev);
            }
            assert_eq!(d.selected, 0);
        }

        /// A `Route::List` dashboard over `active` and `archived`, both sections
        /// open, `selected` given explicitly — `list-sections`' own fixture for
        /// the scenarios that need control over both tiers at once, which
        /// neither `dashboard_at` nor `five_change_dashboard` gives.
        fn dashboard_with_archive(
            active: Vec<crate::changes::Change>,
            archived: Vec<crate::changes::Change>,
            selected: usize,
        ) -> Dashboard {
            Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
                searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
                changes: fixture::set(active, archived, Vec::new()),
                route: Route::List,
                quit: false,
                selected,
                filter: empty_filter(),
                detail: empty_detail(),
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            }
        }

        /// `list-selection` -> "The first target is selected on startup" — a
        /// dashboard with three active changes and no archive starts with
        /// `selected` 0 addressing the active header, not the first change.
        #[test]
        fn first_target_is_selected_on_startup() {
            let mut d = five_change_dashboard();
            d.changes.archived.clear();
            d.changes.archived_total = 0;
            assert_eq!(d.selected, 0);
            assert_eq!(d.selected_change(), None);
            assert_eq!(
                d.targets(),
                vec![
                    Target::Section(SectionKey::Active),
                    Target::Change(0),
                    Target::Change(1),
                    Target::Change(2),
                ]
            );
        }

        /// `list-selection` -> "`j`, `k`, and the arrows move the cursor over
        /// headers and changes".
        #[test]
        fn j_k_and_the_arrows_move_the_cursor_over_headers_and_changes() {
            let mut d = five_change_dashboard();
            d.changes.archived.clear();
            d.changes.archived_total = 0;
            d.route = Route::List;
            assert_eq!(d.selected, 0);

            d.apply(action_for(
                &press(KeyCode::Char('j'), KeyModifiers::NONE),
                false,
            ));
            assert_eq!(d.selected, 1);
            assert_eq!(
                d.selected_change().unwrap().name,
                "add-token-refresh",
                "the cursor stepped off the header onto the first change"
            );

            d.apply(action_for(&press(KeyCode::Down, KeyModifiers::NONE), false));
            assert_eq!(d.selected, 2);

            d.apply(action_for(
                &press(KeyCode::Char('k'), KeyModifiers::NONE),
                false,
            ));
            assert_eq!(d.selected, 1);

            d.apply(action_for(&press(KeyCode::Up, KeyModifiers::NONE), false));
            assert_eq!(d.selected, 0);
            assert_eq!(d.selected_change(), None);
            assert_eq!(
                d.detail.scroll, 0,
                "the list route's keys never touch the detail offset"
            );
        }

        /// `list-selection` -> "The cursor clamps at both ends rather than
        /// wrapping".
        #[test]
        fn the_cursor_clamps_at_both_ends_rather_than_wrapping() {
            let mut d = five_change_dashboard();
            d.changes.archived.clear();
            d.changes.archived_total = 0;
            d.route = Route::List;
            for _ in 0..5 {
                d.apply(Action::Next);
            }
            assert_eq!(d.selected, 3, "four targets: the header and three changes");
            for _ in 0..5 {
                d.apply(Action::Prev);
            }
            assert_eq!(d.selected, 0);
        }

        /// `list-selection` -> "The cursor crosses the archived header into the
        /// archived rows".
        #[test]
        fn the_cursor_crosses_the_archived_header_into_the_archived_rows() {
            let mut d = dashboard_with_archive(
                vec![fixture::active("fix-empty-basket", 7, 7)],
                vec![
                    fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                    fixture::archived(None, "legacy-cleanup", 3, 3),
                ],
                0,
            );
            for _ in 0..4 {
                d.apply(Action::Next);
            }
            assert_eq!(d.selected, 4);
            assert_eq!(d.targets()[4], Target::Change(2));
            assert_eq!(d.selected_change().unwrap().name, "legacy-cleanup");

            // The archived header is a stop, not a row stepped over.
            let on_header = dashboard_with_archive(
                vec![fixture::active("fix-empty-basket", 7, 7)],
                vec![
                    fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                    fixture::archived(None, "legacy-cleanup", 3, 3),
                ],
                2,
            );
            assert_eq!(
                on_header.targets()[2],
                Target::Section(SectionKey::Archived)
            );
            assert_eq!(on_header.selected_change(), None);
        }

        /// `list-selection` -> "A collapsed section's changes are neither
        /// visible nor addressable".
        #[test]
        fn a_collapsed_sections_changes_are_neither_visible_nor_addressable() {
            let mut d = dashboard_with_archive(
                vec![fixture::active("fix-empty-basket", 7, 7)],
                vec![
                    fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                    fixture::archived(None, "legacy-cleanup", 3, 3),
                ],
                4,
            );
            d.sections.collapsed.insert(SectionKey::Archived);
            d.clamp_selection();

            assert_eq!(
                d.visible()
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>(),
                vec!["fix-empty-basket"]
            );
            assert_eq!(
                d.targets(),
                vec![
                    Target::Section(SectionKey::Active),
                    Target::Change(0),
                    Target::Section(SectionKey::Archived),
                ]
            );
            assert!(d.selected <= 2);
            assert_eq!(
                d.targets()[d.selected],
                Target::Section(SectionKey::Archived)
            );
            assert_eq!(d.selected_change(), None);
        }

        /// `list-selection` -> "Navigation over an empty visible list is inert".
        #[test]
        fn navigation_over_an_empty_visible_list_is_inert() {
            let mut d = dashboard_at(Route::List);
            d.apply(Action::Next);
            assert_eq!(d.selected, 0);
            d.apply(Action::Prev);
            assert_eq!(d.selected, 0);
            assert!(d.targets().is_empty());
            assert_eq!(d.selected_change(), None);
        }

        /// `list-selection` -> "`Enter` on a section header does nothing".
        #[test]
        fn enter_on_a_section_header_does_nothing() {
            let mut d = five_change_dashboard();
            d.changes.archived.clear();
            d.changes.archived_total = 0;
            d.route = Route::List;
            assert_eq!(d.selected, 0);

            d.apply(Action::OpenDetail);
            assert_eq!(d.route, Route::List);
            assert_eq!(d.detail.tab, 0);
            assert_eq!(d.detail.scroll, 0);
            assert!(!d.quit);

            // The same dashboard at `selected` 1 — a real change — opens
            // normally, so the inertness above is the header, not the key
            // being unbound.
            let mut on_change = five_change_dashboard();
            on_change.changes.archived.clear();
            on_change.changes.archived_total = 0;
            on_change.route = Route::List;
            on_change.selected = 1;
            on_change.apply(Action::OpenDetail);
            assert_eq!(on_change.route, Route::Detail);

            // The four launch keys are already inert at the header:
            // `selected_change()` is `None` there, and `launch::decide`
            // answers `Decision::Nothing` with no new rule.
            d.agents.reachable = true;
            for action in [
                Action::LaunchApply,
                Action::LaunchContinue,
                Action::LaunchArchive,
                Action::FocusAgent,
            ] {
                d.apply(action);
            }
            assert_eq!(d.launch.pending, None);
            assert!(d.launch.problems.is_empty());
        }

        /// `list-selection` -> "A refresh keeps the cursor on the same change" —
        /// design.md -> Decision 14's proving assertion: `adopt` must resolve
        /// the reselected change through `targets()`, not leave it at the
        /// `visible()` position it found. Goes red against the off-by-headers
        /// form of `adopt`.
        #[test]
        fn a_refresh_keeps_the_cursor_on_the_same_change() {
            // `list-sections`: inlined rather than a shared local helper, on
            // `NOLIT-CHANGE`'s own terms — a helper whose return type names
            // `ChangeSet` trips that gate on its signature alone, the exact
            // defect Change Review found three of in a past change.
            let mut d = dashboard_with_archive(
                vec![
                    fixture::active("alpha", 1, 4),
                    fixture::active("beta", 2, 4),
                ],
                vec![
                    fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                    fixture::archived(None, "legacy-cleanup", 3, 3),
                ],
                4,
            );
            assert_eq!(
                d.selected_change().unwrap().name,
                "add-auth",
                "selected 4 must already address add-auth"
            );

            // `RefreshResult::Files`, then `RefreshResult::Merged` — both are
            // just an `adopt` call from `Dashboard`'s own point of view.
            d.adopt(fixture::set(
                vec![
                    fixture::active("aardvark", 0, 4),
                    fixture::active("alpha", 1, 4),
                    fixture::active("beta", 2, 4),
                ],
                vec![
                    fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                    fixture::archived(None, "legacy-cleanup", 3, 3),
                ],
                Vec::new(),
            ));
            assert_eq!(d.selected_change().unwrap().name, "add-auth");
            assert_eq!(
                d.selected, 5,
                "the targets() index of add-auth, not its visible() position of 3"
            );
            assert_eq!(
                d.visible().iter().position(|c| c.name == "add-auth"),
                Some(3)
            );

            d.adopt(fixture::set(
                vec![
                    fixture::active("aardvark", 0, 4),
                    fixture::active("alpha", 1, 4),
                    fixture::active("beta", 2, 4),
                ],
                vec![
                    fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                    fixture::archived(None, "legacy-cleanup", 3, 3),
                ],
                Vec::new(),
            ));
            assert_eq!(d.selected_change().unwrap().name, "add-auth");
            assert_eq!(d.selected, 5);

            // A selected change that has disappeared from the adopted set
            // leaves `selected` within `targets().len()`, with
            // `selected_change()` either `None` or a change actually shown.
            let mut gone = dashboard_with_archive(
                vec![fixture::active("alpha", 1, 4)],
                vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
                3,
            );
            assert_eq!(gone.selected_change().unwrap().name, "add-auth");
            gone.adopt(fixture::set(
                vec![fixture::active("alpha", 1, 4)],
                Vec::new(),
                Vec::new(),
            ));
            let gone_len = gone.targets().len();
            assert!(gone_len == 0 || gone.selected < gone_len);

            // Adopting with the archived section collapsed still leaves
            // `selected` addressing a target that exists.
            let mut collapsed = dashboard_with_archive(
                vec![
                    fixture::active("alpha", 1, 4),
                    fixture::active("beta", 2, 4),
                ],
                vec![
                    fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                    fixture::archived(None, "legacy-cleanup", 3, 3),
                ],
                4,
            );
            collapsed.sections.collapsed.insert(SectionKey::Archived);
            collapsed.clamp_selection();
            collapsed.adopt(fixture::set(
                vec![
                    fixture::active("aardvark", 0, 4),
                    fixture::active("alpha", 1, 4),
                    fixture::active("beta", 2, 4),
                ],
                vec![
                    fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                    fixture::archived(None, "legacy-cleanup", 3, 3),
                ],
                Vec::new(),
            ));
            assert!(collapsed.selected < collapsed.targets().len());
        }

        /// `dashboard-loop` -> "`Space` maps to `ToggleSection` outside filter
        /// mode and types inside it".
        #[test]
        fn space_maps_to_toggle_section_outside_filter_mode_and_types_inside_it() {
            assert_eq!(
                action_for(&press(KeyCode::Char(' '), KeyModifiers::NONE), false),
                Action::ToggleSection
            );
            assert_eq!(
                action_for(&press(KeyCode::Char(' '), KeyModifiers::NONE), true),
                Action::FilterPush(' ')
            );
            assert_eq!(
                action_for(&press(KeyCode::Char(' '), KeyModifiers::CONTROL), false),
                Action::Ignore
            );
            assert_eq!(
                action_for(&press(KeyCode::Char(' '), KeyModifiers::CONTROL), true),
                Action::Ignore
            );

            let released = Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char(' '),
                KeyModifiers::NONE,
                KeyEventKind::Release,
            ));
            let repeated = Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char(' '),
                KeyModifiers::NONE,
                KeyEventKind::Repeat,
            ));
            for filtering in [false, true] {
                assert_eq!(action_for(&released, filtering), Action::Ignore);
                assert_eq!(action_for(&repeated, filtering), Action::Ignore);
            }
        }

        /// `dashboard-loop` -> "`?` maps to `ToggleHelp` outside filter mode
        /// and types inside it".
        #[test]
        fn question_mark_maps_to_toggle_help_outside_filter_mode_and_types_inside_it() {
            assert_eq!(
                action_for(&press(KeyCode::Char('?'), KeyModifiers::NONE), false),
                Action::ToggleHelp
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('?'), KeyModifiers::SHIFT), false),
                Action::ToggleHelp
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('?'), KeyModifiers::NONE), true),
                Action::FilterPush('?')
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('?'), KeyModifiers::SHIFT), true),
                Action::FilterPush('?')
            );
        }

        /// `help-overlay` -> "`?` toggles the overlay and its near misses do
        /// not".
        #[test]
        fn question_mark_toggles_the_overlay_and_its_near_misses_do_not() {
            assert_eq!(
                action_for(&press(KeyCode::Char('?'), KeyModifiers::CONTROL), false),
                Action::Ignore
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('?'), KeyModifiers::ALT), false),
                Action::Ignore
            );
            // `Char('/')` with `SHIFT` is not `?` — the filter key carrying a
            // stray modifier falls to the wildcard rather than starting a
            // filter, and the two are distinguished by the `KeyCode` the
            // terminal reports rather than by the physical key.
            assert_eq!(
                action_for(&press(KeyCode::Char('/'), KeyModifiers::SHIFT), false),
                Action::Ignore
            );

            let released = Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('?'),
                KeyModifiers::NONE,
                KeyEventKind::Release,
            ));
            let repeated = Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('?'),
                KeyModifiers::NONE,
                KeyEventKind::Repeat,
            ));
            assert_eq!(action_for(&released, false), Action::Ignore);
            assert_eq!(action_for(&repeated, false), Action::Ignore);
        }

        /// `list-selection` -> "`Space` on a header folds and unfolds that
        /// section".
        #[test]
        fn space_on_a_header_folds_and_unfolds_that_section() {
            let mut d = dashboard_with_archive(
                vec![fixture::active("fix-empty-basket", 7, 7)],
                vec![
                    fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                    fixture::archived(None, "legacy-cleanup", 3, 3),
                ],
                2,
            );
            assert_eq!(d.targets()[2], Target::Section(SectionKey::Archived));
            // `list-selection`: `Space` is route-dependent since group 7 — pinned
            // here rather than assumed, since `ToggleSection` at `Route::Detail`
            // folds an artifact section instead.
            assert_eq!(d.route, Route::List);

            d.apply(Action::ToggleSection);
            assert_eq!(
                d.sections.collapsed,
                std::collections::BTreeSet::from([SectionKey::Archived])
            );
            assert_eq!(
                d.visible()
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>(),
                vec!["fix-empty-basket"]
            );
            assert_eq!(d.selected, 2, "still the archived header");

            d.apply(Action::ToggleSection);
            assert!(d.sections.collapsed.is_empty());
            assert_eq!(
                d.visible()
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>(),
                vec!["fix-empty-basket", "add-auth", "legacy-cleanup"]
            );
            assert_eq!(d.selected, 2);
        }

        /// `list-selection` -> "`Space` inside a section folds it and moves
        /// the cursor to its header".
        #[test]
        fn space_inside_a_section_folds_it_and_moves_the_cursor_to_its_header() {
            let mut d = dashboard_with_archive(
                vec![fixture::active("fix-empty-basket", 7, 7)],
                vec![
                    fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                    fixture::archived(None, "legacy-cleanup", 3, 3),
                ],
                4,
            );
            assert_eq!(d.selected_change().unwrap().name, "legacy-cleanup");
            // `list-selection`: pinned to `Route::List` — see the sibling test above.
            assert_eq!(d.route, Route::List);

            d.apply(Action::ToggleSection);
            assert_eq!(
                d.sections.collapsed,
                std::collections::BTreeSet::from([SectionKey::Archived])
            );
            assert_eq!(d.selected, 2);
            assert_eq!(d.targets()[2], Target::Section(SectionKey::Archived));
            assert_eq!(d.selected_change(), None);

            // The same action given at the active change collapses the
            // active section instead — the section acted on is the one the
            // cursor is in, not a fixed one.
            let mut on_active = dashboard_with_archive(
                vec![fixture::active("fix-empty-basket", 7, 7)],
                vec![
                    fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                    fixture::archived(None, "legacy-cleanup", 3, 3),
                ],
                1,
            );
            assert_eq!(on_active.route, Route::List);
            on_active.apply(Action::ToggleSection);
            assert_eq!(
                on_active.sections.collapsed,
                std::collections::BTreeSet::from([SectionKey::Active])
            );
            assert_eq!(on_active.selected, 0);
        }

        /// `list-selection` -> "An empty list makes `Space` inert".
        #[test]
        fn an_empty_list_makes_space_inert() {
            let mut d = dashboard_at(Route::List);
            let before = d.clone();
            for _ in 0..10 {
                d.apply(Action::ToggleSection);
            }
            assert_eq!(d, before, "no field changed, and it did not panic");
        }

        /// `list-selection` -> "Opening an unresolved archive requests a
        /// refresh".
        #[test]
        fn opening_an_unresolved_archive_requests_a_refresh() {
            let mut d = dashboard_with_archive(Vec::new(), Vec::new(), 0);
            d.changes.archived_total = 22;
            d.sections.collapsed.insert(SectionKey::Archived);
            d.refresh.requested = false;
            assert_eq!(d.targets(), vec![Target::Section(SectionKey::Archived)]);
            d.selected = 0;
            assert_eq!(d.route, Route::List);

            d.apply(Action::ToggleSection);
            assert!(d.sections.collapsed.is_empty());
            assert!(d.refresh.requested);
            assert_eq!(d.archived_scope(), crate::changes::ArchivedScope::Full);

            // The reverse toggle from a resolved, open archive costs
            // nothing, because a fold needs no data.
            let mut resolved = dashboard_with_archive(
                Vec::new(),
                (0..22)
                    .map(|i| fixture::archived(None, &format!("c{i}"), 0, 0))
                    .collect(),
                0,
            );
            resolved.refresh.requested = false;
            assert_eq!(resolved.route, Route::List);
            resolved.apply(Action::ToggleSection);
            assert!(!resolved.refresh.requested);
        }

        /// `list-selection` -> "A refresh does not undo a fold".
        #[test]
        fn a_refresh_does_not_undo_a_fold() {
            let mut d = dashboard_with_archive(
                vec![fixture::active("fix-empty-basket", 7, 7)],
                vec![
                    fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                    fixture::archived(None, "legacy-cleanup", 3, 3),
                ],
                0,
            );
            d.sections.collapsed.insert(SectionKey::Archived);
            d.clamp_selection();

            // `RefreshResult::Files`, then `RefreshResult::Merged` — both
            // are just an `adopt` call from `Dashboard`'s own point of view.
            d.adopt(fixture::set(
                vec![fixture::active("wholly-different", 0, 1)],
                vec![fixture::archived(None, "another-one", 0, 1)],
                Vec::new(),
            ));
            assert_eq!(
                d.sections.collapsed,
                std::collections::BTreeSet::from([SectionKey::Archived]),
                "a live update must never reopen what the reader folded"
            );

            d.adopt(fixture::set(
                vec![fixture::active("yet-another", 2, 2)],
                vec![fixture::archived(None, "and-another", 1, 1)],
                Vec::new(),
            ));
            assert_eq!(
                d.sections.collapsed,
                std::collections::BTreeSet::from([SectionKey::Archived])
            );
        }

        /// `list-filtering` -> "`Space` types into the query rather than
        /// folding a section". The footer render assertion belongs to
        /// `ui::view::tests::`, out of this group's file boundary; this test
        /// covers the app-state half.
        #[test]
        fn space_types_into_the_query_rather_than_folding_a_section() {
            let build = || {
                dashboard_with_archive(
                    vec![
                        fixture::active("aaa", 0, 1),
                        fixture::active("bbb", 0, 1),
                        fixture::active("ccc", 0, 1),
                    ],
                    vec![
                        fixture::archived(None, "ddd", 0, 1),
                        fixture::archived(None, "eee", 0, 1),
                    ],
                    0,
                )
            };

            let mut d = build();
            d.sections.collapsed.insert(SectionKey::Archived);
            d.apply(action_for(
                &press(KeyCode::Char('/'), KeyModifiers::NONE),
                false,
            ));
            for c in ['a', ' ', 'd'] {
                d.apply(action_for(
                    &press(KeyCode::Char(c), KeyModifiers::NONE),
                    true,
                ));
            }
            assert_eq!(d.filter.query, "a d");
            assert!(d.filter.active);
            assert_eq!(
                d.sections.collapsed,
                std::collections::BTreeSet::from([SectionKey::Archived]),
                "no section was folded or unfolded by the key itself"
            );

            // The same three keys with `filter.active` false leave
            // `filter.query` empty and fold a section on the second, so the
            // typing above is the filter mode's doing and not the key
            // losing its binding.
            let mut not_filtering = build();
            for c in ['a', ' ', 'd'] {
                not_filtering.apply(action_for(
                    &press(KeyCode::Char(c), KeyModifiers::NONE),
                    false,
                ));
            }
            assert_eq!(not_filtering.filter.query, "");
            assert_eq!(
                not_filtering.sections.collapsed,
                std::collections::BTreeSet::from([SectionKey::Active])
            );
        }

        /// `list-selection` -> "A query reaches a match inside a folded
        /// archive" (the `ui::app::tests::` half; the render half belongs to
        /// group 5, per tasks.md -> 5.1).
        #[test]
        fn a_query_reaches_a_match_inside_a_folded_archive() {
            let mut d = dashboard_with_archive(
                vec![
                    fixture::active("fix-empty-basket", 7, 7),
                    fixture::active("migrate-ai-sdk-v7", 0, 0),
                ],
                vec![
                    fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                    fixture::archived(None, "legacy-cleanup", 3, 3),
                ],
                0,
            );
            d.sections.collapsed.insert(SectionKey::Archived);
            d.filter.query = "auth".to_string();

            assert!(
                d.section_open(SectionKey::Archived),
                "the query forces the archived section open"
            );
            assert_eq!(
                d.visible()
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>(),
                vec!["add-auth"]
            );
            assert_eq!(
                d.targets(),
                vec![Target::Section(SectionKey::Archived), Target::Change(0)],
                "no active header — its matched count is zero"
            );
            assert_eq!(
                d.sections.collapsed,
                std::collections::BTreeSet::from([SectionKey::Archived]),
                "the force-open is derived rather than written"
            );

            // Clearing the query restores the reader's own fold, with
            // nothing having been saved.
            d.filter.query.clear();
            assert!(!d.section_open(SectionKey::Archived));
            assert!(
                d.visible()
                    .iter()
                    .all(|c| c.name != "add-auth" && c.name != "legacy-cleanup"),
                "the archived tier is folded again"
            );
        }

        /// `list-selection` -> "The archived count under a query is the
        /// matched count" (the `ui::app::tests::` half; the render half
        /// belongs to group 5, per tasks.md -> 5.1).
        #[test]
        fn the_archived_count_under_a_query_is_the_matched_count() {
            let mut d = dashboard_with_archive(
                vec![
                    fixture::active("fix-empty-basket", 7, 7),
                    fixture::active("migrate-ai-sdk-v7", 0, 0),
                ],
                vec![
                    fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                    fixture::archived(None, "legacy-cleanup", 3, 3),
                ],
                0,
            );
            d.filter.query = "add".to_string();

            assert_eq!(
                d.targets(),
                vec![Target::Section(SectionKey::Archived), Target::Change(0)],
                "no active header at all, because a section whose matched \
                 count is zero emits none"
            );
            assert_eq!(
                d.visible()
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>(),
                vec!["add-auth"]
            );

            d.filter.query.clear();
            assert_eq!(
                d.targets().len(),
                6,
                "both headers plus all four changes — the counts follow the \
                 query when resolved and the true totals when it is not"
            );
        }

        /// `list-selection` -> "The first character of a query requests the
        /// archive it needs".
        #[test]
        fn the_first_character_of_a_query_requests_the_archive_it_needs() {
            let mut d = dashboard_with_archive(Vec::new(), Vec::new(), 0);
            d.changes.archived_total = 22;
            d.sections.collapsed.insert(SectionKey::Archived);
            d.clamp_selection();
            d.refresh.requested = false;

            d.apply(Action::FilterStart);
            d.apply(Action::FilterPush('a'));
            assert!(d.refresh.requested);
            assert_eq!(d.archived_scope(), crate::changes::ArchivedScope::Full);

            d.refresh.requested = false;
            d.apply(Action::FilterPush('d'));
            d.apply(Action::FilterPush('d'));
            assert!(
                d.refresh.requested,
                "the predicate is still unsatisfied while the tier is unresolved"
            );

            // Against a dashboard whose twenty-two archived changes are
            // already present, the same two characters leave the flag
            // false: a resolved tier costs no further cycle.
            let mut resolved = dashboard_with_archive(
                Vec::new(),
                (0..22)
                    .map(|i| fixture::archived(None, &format!("c{i}"), 0, 0))
                    .collect(),
                0,
            );
            resolved.sections.collapsed.insert(SectionKey::Archived);
            resolved.clamp_selection();
            resolved.apply(Action::FilterStart);
            resolved.apply(Action::FilterPush('a'));
            resolved.refresh.requested = false;
            resolved.apply(Action::FilterPush('d'));
            resolved.apply(Action::FilterPush('d'));
            assert!(!resolved.refresh.requested);

            // `Backspace` back to an empty query leaves `archived_scope()`
            // `Names` and `refresh.requested` untouched, because a fold
            // needs no data.
            d.apply(Action::FilterPop);
            d.apply(Action::FilterPop);
            d.refresh.requested = false;
            d.apply(Action::FilterPop);
            assert_eq!(d.filter.query, "");
            assert_eq!(d.archived_scope(), crate::changes::ArchivedScope::Names);
            assert!(!d.refresh.requested);
        }

        /// `dashboard-loop` -> "Space gains a meaning" and design.md ->
        /// Decision 6's self-clearing claim.
        #[test]
        fn needs_archived_refresh_self_clears_once_resolved() {
            let mut d = dashboard_with_archive(Vec::new(), Vec::new(), 0);
            d.changes.archived_total = 3;
            assert!(
                d.needs_archived_refresh(),
                "open, empty, and a nonzero total"
            );

            d.changes.archived = vec![
                fixture::archived(None, "a", 0, 1),
                fixture::archived(None, "b", 0, 1),
                fixture::archived(None, "c", 0, 1),
            ];
            assert!(
                !d.needs_archived_refresh(),
                "archived.len() == archived_total"
            );

            d.sections.collapsed.insert(SectionKey::Archived);
            d.changes.archived.clear();
            assert!(
                !d.needs_archived_refresh(),
                "a collapsed section needs no data"
            );
        }

        fn twenty_line_detail() -> Detail {
            Detail {
                sections: vec![ArtifactSection {
                    label: Some(String::new()),
                    text: (0..20).map(|i| format!("- line-{i:02}\n")).collect(),
                    depth: 0,
                    progress: None,
                    operation: None,
                }],
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
                expanded: std::collections::BTreeSet::new(),
                drawn_width: None,
            }
        }

        #[test]
        fn detail_destructures_into_exactly_six_fields() {
            let d = twenty_line_detail();
            let Detail {
                sections,
                scroll,
                tab,
                problems,
                loaded,
                expanded,
                drawn_width,
            } = &d;
            assert_eq!(sections.len(), 1);
            assert!(sections[0].text.starts_with("- line-00"));
            assert_eq!(*scroll, 0);
            assert_eq!(*tab, 0);
            assert!(problems.is_empty());
            assert_eq!(*loaded, None);
            assert!(expanded.is_empty());
            // Named because `NODEFAULT-UI` requires the pattern to be
            // exhaustive, and asserted rather than discarded with `_`: a
            // fixture that never drew a frame has recorded no width, which is
            // what makes the fold inert before the first draw.
            assert_eq!(*drawn_width, None);
        }

        /// `detail-scroll` :: "The `ArtifactSection` companion names the fifth
        /// field".
        ///
        /// The compile-time half of `NODEFAULT-UI`'s textual scan: removing any
        /// one of the five from this pattern fails to compile, which is what
        /// makes the companion a check rather than a restatement. `progress` is
        /// `tasks-emphasis`' addition, `operation` `spec-emphasis`'s.
        #[test]
        fn artifact_section_destructures_into_exactly_five_fields() {
            let section = ArtifactSection {
                label: Some("1. Setup".to_string()),
                text: "- [x] 1.1 first\n".to_string(),
                depth: 1,
                progress: Some(crate::tasks::Progress {
                    completed: 1,
                    total: 1,
                }),
                operation: Some(crate::specs::DeltaOp::Added),
            };
            let ArtifactSection {
                label,
                text,
                depth,
                progress,
                operation,
            } = &section;
            assert_eq!(label.as_deref(), Some("1. Setup"));
            assert!(text.starts_with("- [x]"));
            assert_eq!(*depth, 1);
            assert_eq!(
                *progress,
                Some(crate::tasks::Progress {
                    completed: 1,
                    total: 1
                })
            );
            assert_eq!(*operation, Some(crate::specs::DeltaOp::Added));

            // And the two values with no default are really optional: a
            // section that is not a task group carries `None` progress, which
            // is not the same claim as a counted-and-empty `[-]`, and a
            // section that is not a delta requirement carries `None`
            // operation, which is not the same claim as an unbadged one.
            let plain = ArtifactSection {
                label: Some("proposal.md".to_string()),
                text: "# proposal\n".to_string(),
                depth: 0,
                progress: None,
                operation: None,
            };
            assert_eq!(plain.progress, None);
            assert_eq!(plain.operation, None);
        }

        #[test]
        fn next_and_prev_scroll_at_the_detail_route() {
            let mut d = Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                detail: twenty_line_detail(),
                repo: None,
                searched_from: std::path::PathBuf::from("/tmp/does-not-matter"),
                changes: empty_set(),
                route: Route::Detail,
                quit: false,
                selected: 0,
                filter: empty_filter(),
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            };
            d.apply(Action::Next);
            assert_eq!(d.detail.scroll, 1);
            assert_eq!(d.selected, 0);
            d.apply(Action::Next);
            assert_eq!(d.detail.scroll, 2);
            assert_eq!(d.selected, 0);
        }

        #[test]
        fn next_and_prev_select_at_the_list_route() {
            let mut d = five_change_dashboard();
            d.changes.archived.clear();
            d.changes.archived_total = 0;
            d.detail = twenty_line_detail();
            d.route = Route::List;
            d.apply(Action::Next);
            d.apply(Action::Next);
            // `list-sections`: still 2 — the active header shifts every target by
            // one, but starting from 0 and stepping twice lands on the same raw
            // index either way; it now addresses the second active change
            // (`fix-empty-basket`) rather than the third.
            assert_eq!(d.selected, 2);
            assert_eq!(d.detail.scroll, 0);
        }

        #[test]
        fn scroll_stops_at_the_top() {
            let mut d = Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                detail: twenty_line_detail(),
                repo: None,
                searched_from: std::path::PathBuf::from("/tmp/does-not-matter"),
                changes: empty_set(),
                route: Route::Detail,
                quit: false,
                selected: 0,
                filter: empty_filter(),
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            };
            for _ in 0..4 {
                d.apply(Action::Prev);
                assert_eq!(d.detail.scroll, 0);
            }

            // The same four actions against a foldable tab leave
            // `detail.scroll` at `0` and the first header row emphasised.
            let mut foldable = super::foldable_dashboard(std::collections::BTreeSet::new(), 0);
            for _ in 0..4 {
                foldable.apply(Action::Prev);
                assert_eq!(foldable.detail.scroll, 0);
            }
            let rows =
                crate::ui::detail::content_lines(&foldable.detail, foldable.selected_change(), 78);
            assert!(matches!(
                rows[0].kind,
                crate::ui::detail::ContentKind::SectionHeader { selected: true, .. }
            ));
        }

        #[test]
        fn filter_mode_types_j_and_k_while_arrows_scroll() {
            assert_eq!(
                action_for(&press(KeyCode::Char('j'), KeyModifiers::NONE), true),
                Action::FilterPush('j')
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('k'), KeyModifiers::NONE), true),
                Action::FilterPush('k')
            );
            assert_eq!(
                action_for(&press(KeyCode::Down, KeyModifiers::NONE), true),
                Action::Next
            );
            assert_eq!(
                action_for(&press(KeyCode::Up, KeyModifiers::NONE), true),
                Action::Prev
            );

            let mut d = Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                detail: twenty_line_detail(),
                repo: None,
                searched_from: std::path::PathBuf::from("/tmp/does-not-matter"),
                changes: empty_set(),
                route: Route::Detail,
                quit: false,
                selected: 0,
                filter: Filter {
                    query: String::new(),
                    active: true,
                },
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            };
            d.apply(Action::FilterPush('j'));
            d.apply(Action::FilterPush('k'));
            d.apply(Action::Next);
            d.apply(Action::Prev);
            assert_eq!(d.filter.query, "jk");
            assert_eq!(d.detail.scroll, 0);
        }

        #[test]
        fn every_route_move_resets_the_scroll() {
            let mut d = Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                detail: Detail {
                    sections: twenty_line_detail().sections,
                    scroll: 3,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
                    expanded: std::collections::BTreeSet::new(),
                    drawn_width: None,
                },
                repo: None,
                searched_from: std::path::PathBuf::from("/tmp/does-not-matter"),
                changes: empty_set(),
                route: Route::Detail,
                quit: false,
                selected: 0,
                filter: empty_filter(),
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            };
            d.apply(Action::Back);
            assert_eq!(d.detail.scroll, 0);
            d.apply(Action::OpenDetail);
            assert_eq!(d.detail.scroll, 0);

            let mut d2 = Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                detail: Detail {
                    sections: twenty_line_detail().sections,
                    scroll: 3,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
                    expanded: std::collections::BTreeSet::new(),
                    drawn_width: None,
                },
                repo: None,
                searched_from: std::path::PathBuf::from("/tmp/does-not-matter"),
                changes: empty_set(),
                route: Route::Detail,
                quit: false,
                selected: 0,
                filter: empty_filter(),
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            };
            d2.apply(Action::FilterStart);
            assert_eq!(d2.route, Route::List);
            assert_eq!(d2.detail.scroll, 0);

            let mut d3 = Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                detail: Detail {
                    sections: twenty_line_detail().sections,
                    scroll: 3,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
                    expanded: std::collections::BTreeSet::new(),
                    drawn_width: None,
                },
                repo: None,
                searched_from: std::path::PathBuf::from("/tmp/does-not-matter"),
                changes: empty_set(),
                route: Route::Detail,
                quit: false,
                selected: 0,
                filter: Filter {
                    query: String::new(),
                    active: true,
                },
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            };
            d3.apply(Action::Back);
            assert_eq!(d3.detail.scroll, 3);

            // A fourth dashboard, at a foldable tab with `detail.expanded`
            // holding `1`: none of the three route moves clears it.
            let mut d4 = super::foldable_dashboard(std::collections::BTreeSet::from([1]), 2);
            d4.apply(Action::Back);
            assert_eq!(
                d4.detail.expanded,
                std::collections::BTreeSet::from([1]),
                "Back"
            );
            d4.apply(Action::OpenDetail);
            assert_eq!(
                d4.detail.expanded,
                std::collections::BTreeSet::from([1]),
                "OpenDetail"
            );
            d4.apply(Action::FilterStart);
            assert_eq!(
                d4.detail.expanded,
                std::collections::BTreeSet::from([1]),
                "FilterStart"
            );
        }

        /// The `Dashboard` shape shared by this group's three `Enter`
        /// scenarios below, factored out in the REFACTOR step: the same
        /// twenty-line detail source as `twenty_line_detail`, an explicit
        /// `route`, `filter`, and `detail.scroll`. Carries one real,
        /// selected change — `ui::view::render_detail` draws nothing at all
        /// when `selected_change()` is `None`, so an empty `ChangeSet` would
        /// make every rendered-buffer assertion below vacuous regardless of
        /// `detail.sections` or `detail.scroll`.
        fn dashboard_for_enter(route: Route, filter: Filter, scroll: usize) -> Dashboard {
            Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                detail: Detail {
                    sections: twenty_line_detail().sections,
                    scroll,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
                    expanded: std::collections::BTreeSet::new(),
                    drawn_width: None,
                },
                repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
                searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
                changes: fixture::set(
                    vec![fixture::active("detail-view", 4, 9)],
                    Vec::new(),
                    Vec::new(),
                ),
                route,
                quit: false,
                // `list-sections`: 1, not 0 — target 0 is the active header.
                selected: 1,
                filter,
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            }
        }

        /// The detail content area's first row, at the column band the
        /// buffer's own width mandates — 1..=9 at 60 columns, 41..=49 at
        /// 120 — the same pair `ui::view`'s own (private) `detail_marker_cols`
        /// asserts against.
        fn detail_marker_cols(buf: &ratatui::buffer::Buffer, y: u16) -> String {
            let (from, len) = if buf.area.width == 60 {
                (1, 9)
            } else {
                (42, 9)
            };
            crate::testutil::row_text(buf, y)
                .chars()
                .skip(from)
                .take(len)
                .collect()
        }

        /// Whether any row of `buf` contains `needle` — `ui::view`'s own
        /// `buffer_contains`, repeated here because that one is private to
        /// `ui::view`'s own test module.
        fn buffer_contains(buf: &ratatui::buffer::Buffer, needle: &str) -> bool {
            (0..buf.area.height).any(|y| crate::testutil::row_text(buf, y).contains(needle))
        }

        /// `detail-scroll`: "Enter at the detail route moves nothing and
        /// keeps the scroll" — design.md -> Decision 5. The guard is
        /// `self.route != Route::Detail`, the same `before != after` shape
        /// `Back` and `FilterStart` already use.
        #[test]
        fn enter_at_the_detail_route_is_a_noop() {
            // A scroll safely inside `layout::scroll_offset`'s own render-time
            // clamp (content height 14 over the twenty-line source, so a
            // scroll above 6 is clamped regardless of this group's fix) — the
            // point here is `OpenDetail`'s guard, not that pre-existing clamp.
            let mut d = dashboard_for_enter(Route::Detail, empty_filter(), 3);
            let before = d.clone();
            let buf_before_120 = crate::testutil::render_at(120, 20, &d);
            let buf_before_60 = crate::testutil::render_at(60, 20, &d);

            d.apply(Action::OpenDetail);
            d.apply(Action::OpenDetail);
            d.apply(Action::OpenDetail);

            assert_eq!(
                d, before,
                "OpenDetail at the detail route must change nothing at all, not only the scroll"
            );

            let buf_after_120 = crate::testutil::render_at(120, 20, &d);
            assert_eq!(buf_after_120, buf_before_120);
            let buf_after_60 = crate::testutil::render_at(60, 20, &d);
            assert_eq!(buf_after_60, buf_before_60);
            assert_eq!(detail_marker_cols(&buf_after_60, 5), "• line-03");
        }

        /// `detail-scroll`: "Enter from the list route still opens at the
        /// top" — a scroll left behind by an earlier session at the detail
        /// route must not survive a real route move.
        #[test]
        fn enter_from_list_route_still_opens_at_the_top() {
            let mut d = dashboard_for_enter(Route::List, empty_filter(), 7);
            d.apply(Action::OpenDetail);
            assert_eq!(d.route, Route::Detail);
            assert_eq!(
                d.detail.scroll, 0,
                "the guard narrows the reset to real route moves, it does not remove it"
            );

            for (width, height) in [(120u16, 20u16), (60, 20)] {
                let buf = crate::testutil::render_at(width, height, &d);
                assert_eq!(detail_marker_cols(&buf, 5), "• line-00", "width {width}");
            }
        }

        /// `detail-scroll`: "Enter while filtering still dismisses the
        /// filter and resets nothing" — accepting a filter is not a route
        /// move at either route.
        #[test]
        fn enter_while_filtering_still_dismisses_and_resets_nothing() {
            let mut detail_d = dashboard_for_enter(
                Route::Detail,
                Filter {
                    query: "add".to_string(),
                    active: true,
                },
                7,
            );
            detail_d.apply(Action::OpenDetail);
            assert!(!detail_d.filter.active);
            assert_eq!(detail_d.filter.query, "add");
            assert_eq!(detail_d.route, Route::Detail);
            assert_eq!(detail_d.detail.scroll, 7);

            let mut list_d = dashboard_for_enter(
                Route::List,
                Filter {
                    query: "add".to_string(),
                    active: true,
                },
                7,
            );
            list_d.apply(Action::OpenDetail);
            assert!(!list_d.filter.active);
            assert_eq!(list_d.route, Route::List);
            assert_eq!(list_d.detail.scroll, 7);
        }

        /// `list-filtering`: "Matching ignores case outside ASCII" —
        /// `str::to_lowercase` folds both sides, where `to_ascii_lowercase`
        /// left every codepoint above U+007F untouched.
        #[test]
        fn matches_ignores_case_outside_ascii() {
            use crate::ui::app::matches;
            assert!(matches("änderung", "Ä"));
            assert!(matches("ÄNDERUNG", "ä"));
            assert!(!matches("add-token-refresh", "Ä"));
            assert!(matches("add-token-refresh", "add"));

            let mut d = five_change_dashboard();
            d.changes.active = vec![
                fixture::active("änderung-der-api", 1, 2),
                fixture::active("ÜBERSICHT", 1, 2),
                fixture::active("add-token-refresh", 4, 9),
            ];
            d.changes.archived = Vec::new();

            for query in ["Ä", "ä"] {
                d.filter.query = query.to_string();
                for (width, height) in [(120u16, 20u16), (60, 20)] {
                    let buf = crate::testutil::render_at(width, height, &d);
                    assert!(
                        buffer_contains(&buf, "änderung-der-api"),
                        "query {query} width {width}"
                    );
                    assert!(
                        !buffer_contains(&buf, "ÜBERSICHT"),
                        "query {query} width {width}"
                    );
                    assert!(
                        !buffer_contains(&buf, "add-token-refresh"),
                        "query {query} width {width}"
                    );
                }
            }

            for query in ["ÜBER", "über"] {
                d.filter.query = query.to_string();
                for (width, height) in [(120u16, 20u16), (60, 20)] {
                    let buf = crate::testutil::render_at(width, height, &d);
                    assert!(
                        buffer_contains(&buf, "ÜBERSICHT"),
                        "query {query} width {width}"
                    );
                    assert!(
                        !buffer_contains(&buf, "änderung-der-api"),
                        "query {query} width {width}"
                    );
                }
            }

            d.filter.query = "add".to_string();
            for (width, height) in [(120u16, 20u16), (60, 20)] {
                let buf = crate::testutil::render_at(width, height, &d);
                assert!(buffer_contains(&buf, "add-token-refresh"), "width {width}");
            }
        }

        /// `list-filtering`: "The fold is total and its documented edge
        /// cases hold" — seven direct calls, none of which may panic.
        #[test]
        fn the_fold_is_total_and_its_documented_edge_cases_hold() {
            use crate::ui::app::matches;

            let long_name = "a".repeat(200);
            let with_nul = format!("na{}me", '\u{0}');
            let combining_only = "\u{0301}\u{0302}\u{0303}".to_string();
            let family_emoji = "team-\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}";

            for name in [
                "",
                long_name.as_str(),
                with_nul.as_str(),
                combining_only.as_str(),
                "ΟΔΟΣ",
                "İstanbul",
                family_emoji,
            ] {
                assert!(matches(name, ""), "an empty query must match {name:?}");
            }

            // A trailing Σ lowercases to ς (U+03C2, final sigma) by
            // position, not to σ (U+03C3) — the default mapping's own
            // documented behaviour, not a defect.
            assert!(matches("ΟΔΟΣ", "οδο\u{03C2}"));
            assert!(!matches("ΟΔΟΣ", "οδο\u{03C3}"));

            // İ (U+0130) expands under to_lowercase to `i` plus a combining
            // dot above (U+0307); a query already spelling that expansion
            // matches, and so does the plain suffix that follows it.
            assert!(matches("İstanbul", "i\u{0307}"));
            assert!(matches("İstanbul", "stanbul"));
            // The combined query: both halves of the expansion in one query, not just
            // each half separately.
            assert!(matches("İstanbul", "i\u{0307}stanbul"));

            // No normalisation, either side: precomposed ä (U+00E4) and
            // decomposed a + combining diaeresis (U+0308) are different
            // codepoint sequences and neither matches the other.
            assert!(!matches("\u{00E4}", "a\u{0308}"));
            assert!(!matches("a\u{0308}", "\u{00E4}"));

            // A multi-codepoint ZWJ emoji sequence must not panic, and
            // matches itself.
            assert!(matches(family_emoji, family_emoji));
            assert!(!matches(family_emoji, "\u{0}"));
        }

        #[test]
        fn normalise_scroll_clamps_against_the_frame() {
            // detail-scroll: "A resize renormalises the offset on the next
            // frame" — three independent 99-scroll trials, each clamped
            // against the CONTENT area's height (not the whole interior):
            // 14 rows at 120x20 and at 60x20, 34 rows at 120x40, where the
            // twenty-line source fits entirely and the clamp is 0.
            let mut d = Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                detail: Detail {
                    sections: twenty_line_detail().sections,
                    scroll: 99,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
                    expanded: std::collections::BTreeSet::new(),
                    drawn_width: None,
                },
                repo: None,
                searched_from: std::path::PathBuf::from("/tmp/does-not-matter"),
                changes: empty_set(),
                route: Route::List,
                quit: false,
                selected: 0,
                filter: empty_filter(),
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            };
            d.normalise_scroll(ratatui::layout::Rect::new(0, 0, 120, 20));
            assert_eq!(d.detail.scroll, 6);

            let mut d2 = Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                detail: Detail {
                    sections: twenty_line_detail().sections,
                    scroll: 99,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
                    expanded: std::collections::BTreeSet::new(),
                    drawn_width: None,
                },
                repo: None,
                searched_from: std::path::PathBuf::from("/tmp/does-not-matter"),
                changes: empty_set(),
                route: Route::Detail,
                quit: false,
                selected: 0,
                filter: empty_filter(),
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            };
            d2.normalise_scroll(ratatui::layout::Rect::new(0, 0, 60, 20));
            assert_eq!(d2.detail.scroll, 6);

            let mut d3 = Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                detail: Detail {
                    sections: twenty_line_detail().sections,
                    scroll: 99,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
                    expanded: std::collections::BTreeSet::new(),
                    drawn_width: None,
                },
                repo: None,
                searched_from: std::path::PathBuf::from("/tmp/does-not-matter"),
                changes: empty_set(),
                route: Route::Detail,
                quit: false,
                selected: 0,
                filter: empty_filter(),
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            };
            d3.normalise_scroll(ratatui::layout::Rect::new(0, 0, 120, 40));
            assert_eq!(
                d3.detail.scroll, 0,
                "a 34-row content area holds every line"
            );
        }

        /// `detail-scroll`: "Switching to and from the tracked-tasks tab
        /// renormalises the scroll" — the checklist and the markdown body
        /// produce different line counts for the same source, so the
        /// clamp must differ between them too, or `normalise_scroll` is
        /// reading the wrong body's length.
        #[test]
        fn normalise_scroll_clamps_against_the_drawn_body() {
            let source: String = (0..20).map(|i| format!("- [ ] line-{i:02}\n")).collect();
            let progress = crate::tasks::Progress {
                completed: 0,
                total: 20,
            };

            fn dashboard_for(change: crate::changes::Change, source: String) -> Dashboard {
                Dashboard {
                    settings: crate::settings::PanelState {
                        rows: Vec::new(),
                        cursor: 0,
                    },
                    selection: None,
                    detail: Detail {
                        sections: vec![ArtifactSection {
                            label: Some(String::new()),
                            text: source,
                            depth: 0,
                            progress: None,
                            operation: None,
                        }],
                        scroll: 99,
                        tab: 0,
                        problems: Vec::new(),
                        loaded: None,
                        expanded: std::collections::BTreeSet::new(),
                        drawn_width: None,
                    },
                    repo: None,
                    searched_from: std::path::PathBuf::from("/tmp/does-not-matter"),
                    changes: crate::changes::fixture::set(vec![change], Vec::new(), Vec::new()),
                    route: Route::Detail,
                    quit: false,
                    // `list-sections`: 1, not 0 — target 0 is the active header.
                    selected: 1,
                    filter: empty_filter(),
                    refresh: crate::ui::app::Refresh {
                        requested: false,
                        reload: false,
                        startup: Vec::new(),
                        problems: Vec::new(),
                    },
                    agents: crate::agents::AgentSnapshot {
                        agents: Vec::new(),
                        reachable: false,
                        stalled: false,
                        problem: None,
                    },
                    agent_names: crate::state::Mapping::default(),
                    launch: crate::ui::app::Launch {
                        pending: None,
                        in_flight: false,
                        problems: Vec::new(),
                    },
                    sections: Sections {
                        collapsed: std::collections::BTreeSet::new(),
                    },
                    file_mode: false,
                    overlay: crate::ui::app::Overlay {
                        panel: None,
                        scroll: 0,
                        edit: None,
                    },
                }
            }

            let mut d_marked = dashboard_for(
                crate::changes::fixture::with_marked_artifacts(
                    &[("tasks", &[])],
                    Some(0),
                    progress,
                ),
                source.clone(),
            );
            d_marked.normalise_scroll(ratatui::layout::Rect::new(0, 0, 120, 20));

            let mut d_unmarked = dashboard_for(
                crate::changes::fixture::with_marked_artifacts(&[("tasks", &[])], None, progress),
                source,
            );
            d_unmarked.normalise_scroll(ratatui::layout::Rect::new(0, 0, 120, 20));

            assert_ne!(
                d_marked.detail.scroll, d_unmarked.detail.scroll,
                "the checklist (bar + blank + 20 items) and the markdown \
                 body (20 lines) differ in length, so their clamps must too"
            );
        }

        #[test]
        fn normalise_scroll_is_inert_when_the_detail_region_is_not_drawn() {
            let mut d = Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                detail: Detail {
                    sections: twenty_line_detail().sections,
                    scroll: 9,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
                    expanded: std::collections::BTreeSet::new(),
                    drawn_width: None,
                },
                repo: None,
                searched_from: std::path::PathBuf::from("/tmp/does-not-matter"),
                changes: empty_set(),
                route: Route::List,
                quit: false,
                selected: 0,
                filter: empty_filter(),
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            };
            d.normalise_scroll(ratatui::layout::Rect::new(0, 0, 60, 20));
            assert_eq!(
                d.detail.scroll, 9,
                "narrow list route: not drawn, unchanged"
            );
            d.normalise_scroll(ratatui::layout::Rect::new(0, 0, 120, 20));
            assert_eq!(
                d.detail.scroll, 6,
                "wide: drawn, so the early return is a real branch"
            );
        }

        /// `detail-scroll` -> "`ui::view::render` and `Dashboard::normalise_scroll`
        /// cannot derive different content heights from the same frame".
        /// `normalise_scroll` hardcoded `Gutters::Both` regardless of layout
        /// mode, while `render_body` derives `Gutters::LeftOnly` at the wide
        /// layout — the divider spends the detail region's trailing gutter
        /// there. The clamp was therefore computed against a 77-column
        /// content area while the draw path wraps at 78, so a document that
        /// fits the drawn 14-row content area entirely could still be left
        /// scrolled.
        ///
        /// A single 1092-character word hard-splits into exactly 14 full
        /// lines at 78 columns (14 * 78 = 1092) but into 15 at 77 (14 full
        /// lines of 77 plus a 14-character remainder) — the one-column
        /// difference between the two candidate widths is what turns "fits"
        /// into "overflows".
        #[test]
        fn normalise_scroll_agrees_with_render_about_the_wide_layouts_content_width() {
            let mut d = Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                detail: Detail {
                    sections: vec![ArtifactSection {
                        label: Some(String::new()),
                        text: "x".repeat(1092),
                        depth: 0,
                        progress: None,
                        operation: None,
                    }],
                    scroll: 99,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
                    expanded: std::collections::BTreeSet::new(),
                    drawn_width: None,
                },
                repo: None,
                searched_from: std::path::PathBuf::from("/tmp/does-not-matter"),
                changes: empty_set(),
                route: Route::Detail,
                quit: false,
                selected: 0,
                filter: empty_filter(),
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            };
            d.normalise_scroll(ratatui::layout::Rect::new(0, 0, 120, 20));
            assert_eq!(
                d.detail.scroll, 0,
                "a document that fits the drawn 78-column, 14-row content area \
                 must not be left scrolled"
            );
        }

        #[test]
        fn selection_clamps_when_the_filter_shrinks_the_list() {
            let mut d = five_change_dashboard();
            d.selected = 4;
            d.apply(Action::FilterStart);
            for c in ['a', 'd', 'd'] {
                d.apply(Action::FilterPush(c));
            }
            // `list-sections`: 3, not 1 — the query "add" matches one active
            // change and one archived change, so targets are the active
            // header, `add-token-refresh`, the archived header, and
            // `add-auth`; the clamp lands on the last of those four.
            assert_eq!(d.selected, 3);
            for _ in 0..3 {
                d.apply(Action::FilterPop);
            }
            assert_eq!(d.visible_len(), 5);
            assert_eq!(d.selected, 3);
        }

        #[test]
        fn apply_never_leaves_selected_out_of_range() {
            let all_actions = [
                Action::Quit,
                Action::OpenDetail,
                Action::Back,
                Action::Next,
                Action::Prev,
                Action::FilterStart,
                Action::FilterPush('x'),
                Action::FilterPop,
                Action::Ignore,
            ];
            for mut d in [dashboard_at(Route::List), five_change_dashboard()] {
                for action in all_actions {
                    d.apply(action);
                    assert!(d.selected == 0 || d.selected < d.visible_len());
                }
            }

            let mut empty = dashboard_at(Route::List);
            empty.apply(Action::Next);
            assert_eq!(empty.selected, 0);
            empty.apply(Action::Prev);
            assert_eq!(empty.selected, 0);
        }

        #[test]
        fn apply_quit_sets_the_flag() {
            let mut dashboard = dashboard_at(Route::List);
            assert!(!dashboard.quit);
            dashboard.apply(Action::Quit);
            assert!(dashboard.quit);
            assert_eq!(dashboard.route, Route::List);
        }

        #[test]
        fn apply_moves_between_routes() {
            let mut dashboard = dashboard_at(Route::List);
            dashboard.apply(Action::OpenDetail);
            assert_eq!(dashboard.route, Route::Detail);
            assert!(!dashboard.quit);
            dashboard.apply(Action::Back);
            assert_eq!(dashboard.route, Route::List);
            assert!(!dashboard.quit);
        }

        #[test]
        fn refresh_sets_requested_and_changes_nothing_else() {
            // `live-refresh` -> "Refresh sets the flag and touches nothing
            // else".
            let change_a = fixture::active("alpha", 1, 4);
            let change_b = fixture::active("beta", 2, 4);
            let mut dashboard = Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
                searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
                changes: fixture::set(vec![change_a, change_b], Vec::new(), Vec::new()),
                route: Route::Detail,
                quit: false,
                selected: 1,
                filter: Filter {
                    query: "b".to_string(),
                    active: false,
                },
                detail: Detail {
                    sections: vec![ArtifactSection {
                        label: Some(String::new()),
                        text: "stale".to_string(),
                        depth: 0,
                        progress: None,
                        operation: None,
                    }],
                    scroll: 5,
                    tab: 2,
                    problems: Vec::new(),
                    loaded: None,
                    expanded: std::collections::BTreeSet::new(),
                    drawn_width: None,
                },
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            };
            let before = dashboard.clone();

            dashboard.apply(Action::Refresh);

            assert!(dashboard.refresh.requested);
            assert_eq!(dashboard.route, before.route);
            assert_eq!(dashboard.selected, before.selected);
            assert_eq!(dashboard.detail.tab, before.detail.tab);
            assert_eq!(dashboard.detail.scroll, before.detail.scroll);
            assert_eq!(dashboard.filter, before.filter);
            assert_eq!(dashboard.quit, before.quit);
            assert_eq!(dashboard.changes, before.changes);

            // Applying it a second time leaves the flag true and still
            // changes nothing else.
            dashboard.apply(Action::Refresh);
            assert!(dashboard.refresh.requested);
            assert_eq!(dashboard.route, before.route);
            assert_eq!(dashboard.selected, before.selected);
            assert_eq!(dashboard.detail.tab, before.detail.tab);
            assert_eq!(dashboard.detail.scroll, before.detail.scroll);
            assert_eq!(dashboard.filter, before.filter);
            assert_eq!(dashboard.quit, before.quit);
            assert_eq!(dashboard.changes, before.changes);
        }

        /// A `Route::List` dashboard over `active`, with `selected` and
        /// `filter.query` set explicitly — `adopt`'s own tests need control
        /// over the visible list a plain `fixture::set` does not give
        /// `dashboard_at`.
        fn dashboard_with_active(
            active: Vec<crate::changes::Change>,
            selected: usize,
            query: &str,
        ) -> Dashboard {
            Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
                searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
                changes: fixture::set(active, Vec::new(), Vec::new()),
                route: Route::List,
                quit: false,
                selected,
                filter: Filter {
                    query: query.to_string(),
                    active: false,
                },
                detail: Detail {
                    sections: vec![ArtifactSection {
                        label: Some(String::new()),
                        text: "stale".to_string(),
                        depth: 0,
                        progress: None,
                        operation: None,
                    }],
                    scroll: 6,
                    tab: 2,
                    problems: Vec::new(),
                    loaded: None,
                    expanded: std::collections::BTreeSet::new(),
                    drawn_width: None,
                },
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            }
        }

        #[test]
        fn adopt_keeps_the_selection_by_name() {
            // `live-refresh` -> "Adopting a change set preserves the
            // selection by name" -> "The selection follows its change when
            // the list shifts".
            let mut d = dashboard_with_active(
                vec![
                    fixture::active("beta", 1, 4),
                    fixture::active("gamma", 2, 4),
                ],
                // `list-sections`: 2, not 1 — target 0 is the active header.
                2,
                "",
            );
            assert_eq!(d.selected_change().unwrap().name, "gamma");

            d.adopt(fixture::set(
                vec![
                    fixture::active("alpha", 0, 4),
                    fixture::active("beta", 1, 4),
                    fixture::active("gamma", 3, 4),
                ],
                Vec::new(),
                Vec::new(),
            ));

            // `list-sections`: 3, not 2 — the header shifts every target by one.
            assert_eq!(d.selected, 3, "selected must still address gamma");
            assert_eq!(d.selected_change().unwrap().name, "gamma");
            assert_eq!(d.detail.tab, 2, "adopt must not touch detail.tab");
            assert_eq!(d.detail.scroll, 6, "adopt must not touch detail.scroll");
            assert!(d.refresh.reload);
        }

        #[test]
        fn adopt_clamps_when_the_change_is_gone() {
            // `live-refresh` -> "The selection is clamped when its change is
            // gone".
            let mut d = dashboard_with_active(
                vec![
                    fixture::active("alpha", 0, 4),
                    fixture::active("beta", 1, 4),
                    fixture::active("gamma", 2, 4),
                ],
                // `list-sections`: 3, not 2 — target 0 is the active header.
                3,
                "",
            );
            assert_eq!(d.selected_change().unwrap().name, "gamma");

            d.adopt(fixture::set(
                vec![fixture::active("alpha", 0, 4)],
                Vec::new(),
                Vec::new(),
            ));
            // `list-sections`: 1, not 0 — the last target is the header (0)
            // then `alpha` (1), the last valid index.
            assert_eq!(d.selected, 1, "the last visible index, not past the end");
            assert_eq!(d.selected_change().unwrap().name, "alpha");

            // Adopting an empty set from there must not panic, and leaves
            // `selected` at 0 with nothing selected.
            d.adopt(crate::changes::empty_set());
            assert_eq!(d.selected, 0);
            assert!(d.selected_change().is_none());
        }

        #[test]
        fn adopt_resolves_the_name_against_the_filtered_list() {
            // `live-refresh` -> "A filter narrows what the name is resolved
            // against". Query "b" picks out names containing a 'b'; "xyz"
            // deliberately does not, so it is excluded from both visible
            // lists on the same terms the scenario describes.
            let mut d = dashboard_with_active(
                vec![
                    fixture::active("abc", 0, 4),
                    fixture::active("bcd", 1, 4),
                    fixture::active("xyz", 2, 4),
                ],
                // `list-sections`: 2, not 1 — target 0 is the active header,
                // and `xyz` is excluded by the query, so `bcd` is target 2.
                2,
                "b",
            );
            assert_eq!(
                d.visible()
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>(),
                vec!["abc", "bcd"]
            );
            assert_eq!(d.selected_change().unwrap().name, "bcd");

            d.adopt(fixture::set(
                vec![
                    fixture::active("abc", 0, 4),
                    fixture::active("bxx", 1, 4),
                    fixture::active("bcd", 2, 4),
                    fixture::active("xyz", 3, 4),
                ],
                Vec::new(),
                Vec::new(),
            ));

            assert_eq!(
                d.visible()
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>(),
                vec!["abc", "bxx", "bcd"],
                "xyz must stay excluded by the untouched filter"
            );
            // `list-sections`: 3, not 2 — the header shifts every target by
            // one.
            assert_eq!(
                d.selected, 3,
                "still addressing bcd in the new visible list"
            );
            assert_eq!(d.selected_change().unwrap().name, "bcd");
            assert_eq!(d.filter.query, "b", "adopt must not touch the filter query");
        }

        #[test]
        fn back_to_list_at_list_is_a_no_op() {
            let mut dashboard = dashboard_at(Route::List);
            dashboard.apply(Action::Back);
            assert_eq!(dashboard.route, Route::List);
            assert!(!dashboard.quit);
            dashboard.apply(Action::Back);
            assert_eq!(dashboard.route, Route::List);
            assert!(!dashboard.quit);
        }

        #[test]
        fn dashboard_destructures_into_exactly_seventeen_fields() {
            let d = dashboard_at(Route::List);
            let Dashboard {
                repo,
                searched_from,
                changes,
                route,
                quit,
                selected,
                filter,
                detail,
                refresh,
                agents,
                agent_names,
                launch,
                sections,
                file_mode: _,
                overlay,
                selection,
                settings,
            } = &d;
            assert_eq!(*repo, None);
            assert_eq!(
                searched_from,
                &std::path::PathBuf::from("/tmp/does-not-matter")
            );
            assert_eq!(changes.active.len(), 0);
            assert_eq!(*route, Route::List);
            assert!(!*quit);
            assert_eq!(*selected, 0);
            assert_eq!(filter, &empty_filter());
            assert_eq!(detail, &empty_detail());
            assert!(!refresh.requested);
            assert!(!refresh.reload);
            assert!(refresh.startup.is_empty());
            assert!(refresh.problems.is_empty());
            assert!(agents.agents.is_empty());
            assert!(!agents.reachable);
            assert!(agents.problem.is_none());
            assert!(agent_names.names.is_empty());
            assert_eq!(launch.pending, None);
            assert!(launch.problems.is_empty());
            assert!(!launch.in_flight);
            assert!(sections.collapsed.is_empty());
            assert!(overlay.panel.is_none());
            assert_eq!(overlay.scroll, 0);
            assert_eq!(*selection, None);
            assert!(settings.rows.is_empty());
            assert_eq!(settings.cursor, 0);
        }

        #[test]
        fn filter_destructures_into_exactly_two_fields() {
            let f = Filter {
                query: "add".to_string(),
                active: true,
            };
            let Filter { query, active } = &f;
            assert_eq!(query, "add");
            assert!(*active);

            // The fourth compile-time companion `design.md` -> Contracts
            // names: a `Refresh` destructured naming all four fields (three at
            // `live-refresh`, `startup` added by `seam-resilience`) and no `..`,
            // folded into this test rather than a fifth top-level one, since
            // group 1 adds no test to `ui::app::tests::`.
            let r = crate::ui::app::Refresh {
                requested: true,
                reload: false,
                startup: vec!["openspec binary not found".to_string()],
                problems: vec!["watch failed".to_string()],
            };
            let crate::ui::app::Refresh {
                requested,
                reload,
                startup,
                problems,
            } = &r;
            assert!(*requested);
            assert!(!*reload);
            assert_eq!(startup, &vec!["openspec binary not found".to_string()]);
            assert_eq!(problems, &vec!["watch failed".to_string()]);

            // `seam-resilience`'s own companion, on the same terms: a `Launch`
            // destructured naming all three fields (two at `agent-launch`,
            // `in_flight` added here) and no `..`.
            let l = crate::ui::app::Launch {
                pending: None,
                problems: vec!["launch failed".to_string()],
                in_flight: true,
            };
            let crate::ui::app::Launch {
                pending,
                problems,
                in_flight,
            } = &l;
            assert_eq!(*pending, None);
            assert_eq!(problems, &vec!["launch failed".to_string()]);
            assert!(*in_flight);
        }

        /// `agent-polling`'s three compile-time companions `design.md` -> Contracts
        /// names alongside `Dashboard`'s own: an `Agent` destructured naming all eight
        /// fields, a `Listed` naming both, and an `AgentSnapshot` naming all three — so
        /// a field added to any of the three fails to compile at this site rather than
        /// passing a source grep that never saw it.
        #[test]
        fn agent_destructures_into_exactly_eight_fields() {
            let agent = crate::agents::Agent {
                name: Some("agent-polling".to_string()),
                kind: Some("claude".to_string()),
                status: crate::agents::AgentStatus::Idle,
                cwd: Some(std::path::PathBuf::from("/repo")),
                pane_id: "w8:p1".to_string(),
                tab_id: "w8:t1".to_string(),
                workspace_id: "w8".to_string(),
                terminal_title: Some("a title".to_string()),
            };
            let crate::agents::Agent {
                name,
                kind,
                status,
                cwd,
                pane_id,
                tab_id,
                workspace_id,
                terminal_title,
            } = &agent;
            assert_eq!(name, &Some("agent-polling".to_string()));
            assert_eq!(kind, &Some("claude".to_string()));
            assert_eq!(*status, crate::agents::AgentStatus::Idle);
            assert_eq!(cwd, &Some(std::path::PathBuf::from("/repo")));
            assert_eq!(pane_id, "w8:p1");
            assert_eq!(tab_id, "w8:t1");
            assert_eq!(workspace_id, "w8");
            assert_eq!(terminal_title, &Some("a title".to_string()));
        }

        #[test]
        fn listed_destructures_into_exactly_two_fields() {
            let listed = crate::agents::Listed {
                agents: Vec::new(),
                problems: vec!["bad entry".to_string()],
            };
            let crate::agents::Listed { agents, problems } = &listed;
            assert!(agents.is_empty());
            assert_eq!(problems, &vec!["bad entry".to_string()]);
        }

        #[test]
        fn agent_snapshot_destructures_into_exactly_four_fields() {
            let snapshot = crate::agents::AgentSnapshot {
                agents: Vec::new(),
                reachable: true,
                stalled: false,
                problem: Some("partial".to_string()),
            };
            let crate::agents::AgentSnapshot {
                agents,
                reachable,
                stalled: _,
                problem,
            } = &snapshot;
            assert!(agents.is_empty());
            assert!(*reachable);
            assert_eq!(problem, &Some("partial".to_string()));
        }

        /// `Dashboard` still carries no thread, channel, or clock: `AgentSnapshot` is
        /// plain data, and the poller itself reaches the loop through
        /// `ui::driver::Live`, never through the state value.
        #[test]
        fn dashboard_is_clone_and_eq_with_agents() {
            let mut d = dashboard_at(Route::List);
            d.agents = crate::agents::AgentSnapshot {
                agents: vec![crate::agents::Agent {
                    name: None,
                    kind: Some("claude".to_string()),
                    status: crate::agents::AgentStatus::Working,
                    cwd: None,
                    pane_id: "w8:p1".to_string(),
                    tab_id: "w8:t1".to_string(),
                    workspace_id: "w8".to_string(),
                    terminal_title: None,
                }],
                reachable: true,
                stalled: false,
                problem: None,
            };
            d.agent_names = crate::state::Mapping {
                names: std::collections::BTreeMap::from([(
                    "c-2fa-support".to_string(),
                    "2fa-support".to_string(),
                )]),
                problems: Vec::new(),
            };
            let cloned = d.clone();
            assert_eq!(d, cloned);
            assert_eq!(d.agents.agents.len(), 1);
            assert_eq!(d.agent_names.names.len(), 1);
        }

        #[test]
        fn the_digit_keys_select_tabs_and_near_misses_do_not() {
            assert_eq!(
                action_for(&press(KeyCode::Char('1'), KeyModifiers::NONE), false),
                Action::SelectTab(0)
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('5'), KeyModifiers::NONE), false),
                Action::SelectTab(4)
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('9'), KeyModifiers::NONE), false),
                Action::SelectTab(8)
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('0'), KeyModifiers::NONE), false),
                Action::Ignore
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('1'), KeyModifiers::CONTROL), false),
                Action::Ignore
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('!'), KeyModifiers::SHIFT), false),
                Action::Ignore
            );

            assert_eq!(
                action_for(&press(KeyCode::Char('1'), KeyModifiers::NONE), true),
                Action::FilterPush('1')
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('5'), KeyModifiers::NONE), true),
                Action::FilterPush('5')
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('9'), KeyModifiers::NONE), true),
                Action::FilterPush('9')
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('0'), KeyModifiers::NONE), true),
                Action::FilterPush('0')
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('1'), KeyModifiers::CONTROL), true),
                Action::Ignore
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('!'), KeyModifiers::SHIFT), true),
                Action::FilterPush('!')
            );
        }

        #[test]
        fn the_bracket_keys_step_one_tab_and_near_misses_do_not() {
            assert_eq!(
                action_for(&press(KeyCode::Char(']'), KeyModifiers::NONE), false),
                Action::NextTab
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('['), KeyModifiers::NONE), false),
                Action::PrevTab
            );
            assert_eq!(
                action_for(&press(KeyCode::Char(']'), KeyModifiers::CONTROL), false),
                Action::Ignore
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('}'), KeyModifiers::SHIFT), false),
                Action::Ignore
            );

            assert_eq!(
                action_for(&press(KeyCode::Char(']'), KeyModifiers::NONE), true),
                Action::FilterPush(']')
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('['), KeyModifiers::NONE), true),
                Action::FilterPush('[')
            );
        }

        #[test]
        fn a_release_or_repeat_of_a_tab_key_is_ignored_under_both_modes() {
            for filtering in [false, true] {
                let released = Event::Key(KeyEvent::new_with_kind(
                    KeyCode::Char('1'),
                    KeyModifiers::NONE,
                    KeyEventKind::Release,
                ));
                let repeated = Event::Key(KeyEvent::new_with_kind(
                    KeyCode::Char('1'),
                    KeyModifiers::NONE,
                    KeyEventKind::Repeat,
                ));
                assert_eq!(action_for(&released, filtering), Action::Ignore);
                assert_eq!(action_for(&repeated, filtering), Action::Ignore);

                let released = Event::Key(KeyEvent::new_with_kind(
                    KeyCode::Char(']'),
                    KeyModifiers::NONE,
                    KeyEventKind::Release,
                ));
                let repeated = Event::Key(KeyEvent::new_with_kind(
                    KeyCode::Char(']'),
                    KeyModifiers::NONE,
                    KeyEventKind::Repeat,
                ));
                assert_eq!(action_for(&released, filtering), Action::Ignore);
                assert_eq!(action_for(&repeated, filtering), Action::Ignore);
            }
        }

        #[test]
        fn stepping_is_clamped_at_both_ends_and_does_not_wrap() {
            let mut d = dashboard_with_artifacts(3, 0, 0);
            d.apply(Action::PrevTab);
            assert_eq!(d.detail.tab, 0);
            for expect in [1, 2, 2, 2] {
                d.apply(Action::NextTab);
                assert_eq!(d.detail.tab, expect);
            }
            for expect in [1, 0, 0, 0] {
                d.apply(Action::PrevTab);
                assert_eq!(d.detail.tab, expect);
            }
        }

        #[test]
        fn an_out_of_range_digit_is_inert() {
            let mut d = dashboard_with_artifacts(3, 1, 5);
            d.apply(Action::SelectTab(6));
            assert_eq!(d.detail.tab, 1);
            assert_eq!(d.detail.scroll, 5);
            d.apply(Action::SelectTab(2));
            assert_eq!(d.detail.tab, 2);
            assert_eq!(d.detail.scroll, 0);
        }

        #[test]
        fn switching_tabs_resets_the_scroll_and_staying_put_does_not() {
            let mut d = dashboard_with_artifacts(3, 2, 7);
            d.apply(Action::NextTab);
            assert_eq!(d.detail.tab, 2);
            assert_eq!(d.detail.scroll, 7);

            let mut d2 = dashboard_with_artifacts(3, 2, 7);
            d2.apply(Action::PrevTab);
            assert_eq!(d2.detail.tab, 1);
            assert_eq!(d2.detail.scroll, 0);
        }

        #[test]
        fn moving_the_selection_resets_the_tab_and_the_scroll_and_a_clamped_move_does_not() {
            let mut d = Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
                searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
                changes: fixture::set(
                    vec![
                        fixture::active("a", 1, 2),
                        fixture::active("b", 1, 2),
                        fixture::active("c", 1, 2),
                    ],
                    Vec::new(),
                    Vec::new(),
                ),
                route: Route::List,
                quit: false,
                selected: 0,
                filter: empty_filter(),
                detail: Detail {
                    sections: Vec::new(),
                    scroll: 9,
                    tab: 2,
                    problems: Vec::new(),
                    loaded: None,
                    expanded: std::collections::BTreeSet::new(),
                    drawn_width: None,
                },
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            };
            d.apply(Action::Next);
            assert_eq!(d.selected, 1);
            assert_eq!(d.detail.tab, 0);
            assert_eq!(d.detail.scroll, 0);

            let mut d2 = Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
                searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
                changes: fixture::set(
                    vec![
                        fixture::active("a", 1, 2),
                        fixture::active("b", 1, 2),
                        fixture::active("c", 1, 2),
                    ],
                    Vec::new(),
                    Vec::new(),
                ),
                route: Route::List,
                quit: false,
                selected: 0,
                filter: empty_filter(),
                detail: Detail {
                    sections: Vec::new(),
                    scroll: 9,
                    tab: 2,
                    problems: Vec::new(),
                    loaded: None,
                    expanded: std::collections::BTreeSet::new(),
                    drawn_width: None,
                },
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            };
            d2.apply(Action::Prev);
            assert_eq!(d2.selected, 0);
            assert_eq!(d2.detail.tab, 2);
            assert_eq!(d2.detail.scroll, 9);
        }

        #[test]
        fn tab_keys_act_at_both_routes() {
            let mut list_route = dashboard_with_artifacts(3, 0, 0);
            list_route.route = Route::List;
            list_route.apply(Action::SelectTab(2));
            assert_eq!(list_route.detail.tab, 2);
            assert_eq!(list_route.route, Route::List);

            let mut detail_route = dashboard_with_artifacts(3, 0, 0);
            detail_route.apply(Action::SelectTab(2));
            assert_eq!(detail_route.detail.tab, 2);
            assert_eq!(detail_route.route, Route::Detail);
        }

        /// A `Route::Detail` dashboard over one selected active change,
        /// built through `fixture::with_artifacts`.
        fn dashboard_with_artifacts_named(name: &str, artifacts: &[(&str, &[&str])]) -> Dashboard {
            let change = fixture::with_artifacts(fixture::active(name, 4, 9), artifacts);
            Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
                searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
                changes: fixture::set(vec![change], Vec::new(), Vec::new()),
                route: Route::Detail,
                quit: false,
                // `list-sections`: 1, not 0 — target 0 is the active header.
                selected: 1,
                filter: empty_filter(),
                detail: Detail {
                    sections: Vec::new(),
                    scroll: 0,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
                    expanded: std::collections::BTreeSet::new(),
                    drawn_width: None,
                },
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            }
        }

        #[test]
        fn the_selected_tabs_file_is_read_once_and_reused() {
            let mut d = dashboard_with_artifacts_named(
                "x",
                &[("proposal", &["/repo/p.md"]), ("design", &["/repo/d.md"])],
            );
            let dir = d.changes.active[0].dir.clone();
            let recorder = RecordingReader::always(Ok("# proposal".to_string()));
            let read = |p: &std::path::Path| recorder.read(p);

            d.sync_detail(&read);
            assert!(
                !d.refresh.reload,
                "live-refresh: an unforced sync leaves the flag false"
            );
            d.sync_detail(&read);
            assert!(!d.refresh.reload);
            d.sync_detail(&read);
            assert!(!d.refresh.reload);

            assert_eq!(d.detail.sections.len(), 1);
            assert_eq!(d.detail.sections[0].text, "# proposal");
            assert!(d.detail.problems.is_empty());
            assert_eq!(d.detail.loaded, Some((dir, 0)));
            assert_eq!(recorder.calls(), 1);
            assert_eq!(
                recorder.paths(),
                vec![std::path::PathBuf::from("/repo/p.md")]
            );
        }

        #[test]
        fn a_forced_reload_keeps_the_scroll() {
            // `live-refresh` -> "A forced reload re-reads without losing the
            // scroll" -> "A forced reload re-reads the same tab and keeps
            // the offset".
            let mut d = dashboard_with_artifacts_named("x", &[("proposal", &["/repo/p.md"])]);
            let dir = d.changes.active[0].dir.clone();
            let first = RecordingReader::always(Ok("# proposal".to_string()));
            let read_first = |p: &std::path::Path| first.read(p);
            d.sync_detail(&read_first);
            d.detail.scroll = 6;
            d.detail.expanded = std::collections::BTreeSet::from([0]);
            assert_eq!(d.detail.loaded, Some((dir.clone(), 0)));

            d.refresh.reload = true;
            let second = RecordingReader::always(Ok("# a much longer proposal now".to_string()));
            let read_second = |p: &std::path::Path| second.read(p);
            d.sync_detail(&read_second);

            assert_eq!(
                second.calls(),
                1,
                "the unchanged key must not suppress the forced re-read"
            );
            assert_eq!(second.paths(), vec![std::path::PathBuf::from("/repo/p.md")]);
            assert_eq!(d.detail.sections.len(), 1);
            assert_eq!(d.detail.sections[0].text, "# a much longer proposal now");
            assert_eq!(
                d.detail.scroll, 6,
                "a forced reload must not move the scroll"
            );
            assert_eq!(
                d.detail.expanded,
                std::collections::BTreeSet::from([0]),
                "artifact-content: expanded is unchanged because the key did not change"
            );
            assert!(!d.refresh.reload, "one flag must drive exactly one re-read");
            assert_eq!(d.detail.loaded, Some((dir, 0)));
        }

        #[test]
        fn a_tab_move_under_a_forced_reload_resets_the_scroll() {
            // `live-refresh` -> "A tab move under a forced reload still
            // resets the scroll" — the reset is attributed to the KEY
            // change, not to the flag; the preceding test proves a forced
            // reload alone preserves the offset.
            let mut d = dashboard_with_artifacts_named(
                "x",
                &[("proposal", &["/repo/p.md"]), ("design", &["/repo/d.md"])],
            );
            let dir = d.changes.active[0].dir.clone();
            let first = RecordingReader::always(Ok("# proposal".to_string()));
            let read_first = |p: &std::path::Path| first.read(p);
            d.sync_detail(&read_first);
            d.detail.scroll = 6;
            d.detail.expanded = std::collections::BTreeSet::from([0]);

            d.detail.tab = 1;
            d.refresh.reload = true;
            let second = RecordingReader::always(Ok("# design".to_string()));
            let read_second = |p: &std::path::Path| second.read(p);
            d.sync_detail(&read_second);

            assert_eq!(second.calls(), 1);
            assert_eq!(second.paths(), vec![std::path::PathBuf::from("/repo/d.md")]);
            assert_eq!(d.detail.sections.len(), 1);
            assert_eq!(d.detail.sections[0].text, "# design");
            assert_eq!(
                d.detail.scroll, 0,
                "the tab move, not the forced flag, must reset the scroll"
            );
            assert!(
                d.detail.expanded.is_empty(),
                "artifact-content: the key change resets expanded, attributable to the \
                 move rather than the flag — the preceding test proves a forced reload \
                 alone preserves it"
            );
            assert!(!d.refresh.reload);
            assert_eq!(d.detail.loaded, Some((dir, 1)));
        }

        #[test]
        fn a_tab_move_forgets_the_fold_a_forced_reload_does_not() {
            // `artifact-folds` -> "A tab move forgets the fold, a forced
            // reload does not" — three dashboards, one per path, each
            // starting synced once with `expanded` holding `1`.
            let reader = RecordingReader::always(Ok("# text".to_string()));
            let read = |p: &std::path::Path| reader.read(p);
            let artifacts: &[(&str, &[&str])] =
                &[("proposal", &["/repo/p.md"]), ("design", &["/repo/d.md"])];

            // Path 1: a tab move away and back forgets the fold.
            let mut moved = dashboard_with_artifacts_named("x", artifacts);
            moved.sync_detail(&read);
            moved.detail.expanded = std::collections::BTreeSet::from([1]);
            moved.detail.tab = 1;
            moved.sync_detail(&read);
            moved.detail.tab = 0;
            moved.sync_detail(&read);
            assert!(
                moved.detail.expanded.is_empty(),
                "the tab reopened after the round trip must reopen collapsed"
            );

            // Path 2: a forced reload of the same key leaves the fold and
            // the scroll alone.
            let mut reloaded = dashboard_with_artifacts_named("x", artifacts);
            reloaded.sync_detail(&read);
            reloaded.detail.expanded = std::collections::BTreeSet::from([1]);
            reloaded.detail.scroll = 6;
            reloaded.refresh.reload = true;
            reloaded.sync_detail(&read);
            assert_eq!(
                reloaded.detail.expanded,
                std::collections::BTreeSet::from([1]),
                "a forced reload of an unchanged key must not fold anything shut"
            );
            assert_eq!(reloaded.detail.scroll, 6);

            // Path 3: an adopted `RefreshResult::Files` then
            // `RefreshResult::Merged` — both are just an `adopt` call — also
            // leave the fold alone.
            let mut adopted = dashboard_with_artifacts_named("x", artifacts);
            adopted.sync_detail(&read);
            adopted.detail.expanded = std::collections::BTreeSet::from([1]);
            adopted.adopt(fixture::set(
                vec![fixture::with_artifacts(
                    fixture::active("x", 4, 9),
                    artifacts,
                )],
                Vec::new(),
                Vec::new(),
            ));
            adopted.adopt(fixture::set(
                vec![fixture::with_artifacts(
                    fixture::active("x", 4, 9),
                    artifacts,
                )],
                Vec::new(),
                Vec::new(),
            ));
            assert_eq!(
                adopted.detail.expanded,
                std::collections::BTreeSet::from([1]),
                "adopt must never touch the fold state"
            );
        }

        /// A one-change dashboard whose `proposal` is unmarked and whose
        /// `tasks` artifact — position 1, the selected tab — carries
        /// `tracks_tasks`, unless `marked` is false.
        fn tasks_tab_dashboard(marked: bool) -> Dashboard {
            let artifacts: &[(&str, &[&str])] =
                &[("proposal", &["/repo/p.md"]), ("tasks", &["/repo/t.md"])];
            let mut d = dashboard_with_artifacts_named("x", artifacts);
            if marked {
                let change = fixture::track_tasks_at(
                    fixture::with_artifacts(fixture::active("x", 4, 9), artifacts),
                    1,
                );
                d.changes = fixture::set(vec![change], Vec::new(), Vec::new());
            }
            d.detail.tab = 1;
            d
        }

        /// `artifact-content` :: "The tasks tab seeds its folds once, on the
        /// key change".
        #[test]
        fn the_tasks_tab_seeds_its_folds_once_on_the_key_change() {
            const UNFINISHED: &str = "## 1. Done\n\n- [x] a\n\n## 2. Doing\n\n- [ ] b\n";
            const FINISHED: &str = "## 1. Done\n\n- [x] a\n\n## 2. Doing\n\n- [x] b\n";

            for marked in [true, false] {
                let source = std::cell::RefCell::new(UNFINISHED.to_string());
                let read =
                    |_: &std::path::Path| -> Result<String, String> { Ok(source.borrow().clone()) };
                let want_seed = if marked {
                    std::collections::BTreeSet::from([1])
                } else {
                    std::collections::BTreeSet::new()
                };

                let mut d = tasks_tab_dashboard(marked);
                d.sync_detail(&read);
                assert_eq!(
                    d.detail.expanded, want_seed,
                    "marked {marked}: the first sync seeds"
                );

                // A forced reload of the **same** key re-reads and re-splits
                // but must not re-seed, even though `b` is now checked.
                *source.borrow_mut() = FINISHED.to_string();
                d.refresh.reload = true;
                d.sync_detail(&read);
                assert_eq!(
                    d.detail.expanded, want_seed,
                    "marked {marked}: a forced reload does not re-seed"
                );

                // A tab move away and back re-seeds — and finds nothing.
                d.detail.tab = 0;
                d.sync_detail(&read);
                d.detail.tab = 1;
                d.sync_detail(&read);
                assert!(
                    d.detail.expanded.is_empty(),
                    "marked {marked}: the re-seed found no incomplete subtree"
                );
            }
        }

        /// `artifact-folds` :: "A completed group does not fold shut under the
        /// reader" — the whole reason the seed runs on the key change only.
        #[test]
        fn a_completed_group_does_not_fold_shut_under_the_reader() {
            const MOSTLY: &str = "## 1. Done\n\n- [x] a\n\n## 2. Doing\n\n- [x] b\n- [ ] c\n\n## 3. Later\n\n- [ ] d\n";
            const ALL_BUT_LAST: &str = "## 1. Done\n\n- [x] a\n\n## 2. Doing\n\n- [x] b\n- [x] c\n\n## 3. Later\n\n- [ ] d\n";

            let source = std::cell::RefCell::new(MOSTLY.to_string());
            let read =
                |_: &std::path::Path| -> Result<String, String> { Ok(source.borrow().clone()) };

            let mut d = tasks_tab_dashboard(true);
            d.sync_detail(&read);
            assert_eq!(
                d.detail.expanded,
                std::collections::BTreeSet::from([1, 2]),
                "the mostly-finished file opens at its first unfinished group"
            );

            // An agent in another pane checks `c` off: a forced reload on an
            // unchanged key.
            *source.borrow_mut() = ALL_BUT_LAST.to_string();
            d.refresh.reload = true;
            d.sync_detail(&read);
            assert_eq!(
                d.detail.expanded,
                std::collections::BTreeSet::from([1, 2]),
                "the group the reader is in must not fold shut under them"
            );

            // Moving the tab away and back re-seeds it to hold `2` alone.
            d.detail.tab = 0;
            d.sync_detail(&read);
            d.detail.tab = 1;
            d.sync_detail(&read);
            assert_eq!(
                d.detail.expanded,
                std::collections::BTreeSet::from([2]),
                "the re-seed found only the third group incomplete"
            );
        }

        #[test]
        fn switching_the_tab_rereads_and_so_does_switching_the_change() {
            let a = fixture::with_artifacts(
                fixture::active("a", 0, 0),
                &[
                    ("proposal", &["/repo/a-p.md"]),
                    ("design", &["/repo/a-d.md"]),
                ],
            );
            let b = fixture::with_artifacts(
                fixture::active("b", 0, 0),
                &[("proposal", &["/repo/b-p.md"])],
            );
            let mut d = Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
                searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
                changes: fixture::set(vec![a, b], Vec::new(), Vec::new()),
                route: Route::Detail,
                quit: false,
                // `list-sections`: 1, not 0 — target 0 is the active header,
                // so `a` is target 1.
                selected: 1,
                filter: empty_filter(),
                detail: Detail {
                    sections: Vec::new(),
                    scroll: 0,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
                    expanded: std::collections::BTreeSet::new(),
                    drawn_width: None,
                },
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            };
            let recorder = RecordingReader::always(Ok("text".to_string()));
            let read = |p: &std::path::Path| recorder.read(p);

            d.sync_detail(&read);
            d.apply(Action::NextTab);
            d.sync_detail(&read);
            // `list-sections`: 2, not 1 — target 1 is `a`, target 2 is `b`.
            d.selected = 2;
            d.sync_detail(&read);

            assert_eq!(
                recorder.paths(),
                vec![
                    std::path::PathBuf::from("/repo/a-p.md"),
                    std::path::PathBuf::from("/repo/a-d.md"),
                    std::path::PathBuf::from("/repo/b-p.md"),
                ]
            );
            assert_eq!(d.detail.sections.len(), 1);
            assert_eq!(d.detail.sections[0].text, "text");
            assert_eq!(d.detail.scroll, 0);
        }

        #[test]
        fn two_changes_with_the_same_name_are_distinguished_by_directory() {
            let active = fixture::with_artifacts(
                fixture::active("add-auth", 0, 0),
                &[("proposal", &["/repo/openspec/changes/add-auth/p.md"])],
            );
            let archived = fixture::with_artifacts(
                fixture::archived(Some("2026-08-14"), "add-auth", 0, 0),
                &[(
                    "proposal",
                    &["/repo/openspec/changes/archive/2026-08-14-add-auth/p.md"],
                )],
            );
            let mut d = Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
                searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
                changes: fixture::set(vec![active], vec![archived], Vec::new()),
                route: Route::Detail,
                quit: false,
                // `list-sections`: 1, not 0 — target 0 is the active header,
                // so the active `add-auth` is target 1.
                selected: 1,
                filter: empty_filter(),
                detail: Detail {
                    sections: Vec::new(),
                    scroll: 0,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
                    expanded: std::collections::BTreeSet::new(),
                    drawn_width: None,
                },
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            };
            let recorder = RecordingReader::new(
                vec![
                    (
                        std::path::PathBuf::from("/repo/openspec/changes/add-auth/p.md"),
                        Ok("ACTIVE".to_string()),
                    ),
                    (
                        std::path::PathBuf::from(
                            "/repo/openspec/changes/archive/2026-08-14-add-auth/p.md",
                        ),
                        Ok("ARCHIVED".to_string()),
                    ),
                ],
                Err("unexpected path".to_string()),
            );
            let read = |p: &std::path::Path| recorder.read(p);

            d.sync_detail(&read);
            // `list-sections`: 3, not 1 — targets are the active header (0),
            // the active `add-auth` (1), the archived header (2), and the
            // archived `add-auth` (3).
            d.selected = 3;
            d.sync_detail(&read);

            assert_eq!(recorder.calls(), 2);
            assert_eq!(d.detail.sections.len(), 1);
            assert_eq!(d.detail.sections[0].text, "ARCHIVED");
        }

        #[test]
        fn a_multi_file_artifact_is_concatenated_in_path_order_with_a_separating_newline() {
            // `artifact-folds`: rewritten per the spec's new body — a section's
            // `text` is the reader's bytes **verbatim**, with no separator
            // inserted and no newline added, even though the first file below
            // does not end with one. The paths sit under the change directory
            // (`fixture::active("x", ..)`'s own `/repo/openspec/changes/x`) so
            // the two sections also carry the labels `artifact-folds` derives.
            let mut d = dashboard_with_artifacts_named(
                "x",
                &[(
                    "specs",
                    &[
                        "/repo/openspec/changes/x/specs/a/spec.md",
                        "/repo/openspec/changes/x/specs/b/spec.md",
                    ],
                )],
            );
            let recorder = RecordingReader::new(
                vec![
                    (
                        std::path::PathBuf::from("/repo/openspec/changes/x/specs/a/spec.md"),
                        Ok("# a".to_string()),
                    ),
                    (
                        std::path::PathBuf::from("/repo/openspec/changes/x/specs/b/spec.md"),
                        Ok("# b\n".to_string()),
                    ),
                ],
                Err("unexpected".to_string()),
            );
            let read = |p: &std::path::Path| recorder.read(p);
            d.sync_detail(&read);

            assert_eq!(d.detail.sections.len(), 2);
            assert_eq!(d.detail.sections[0].label.as_deref(), Some("a"));
            assert_eq!(d.detail.sections[0].text, "# a");
            assert_eq!(d.detail.sections[1].label.as_deref(), Some("b"));
            assert_eq!(d.detail.sections[1].text, "# b\n");
        }

        #[test]
        fn an_unreadable_file_names_its_reason_and_does_not_lose_its_siblings() {
            let mut d = dashboard_with_artifacts_named(
                "x",
                &[("specs", &["/repo/specs/a/spec.md", "/repo/specs/b/spec.md"])],
            );
            let recorder = RecordingReader::new(
                vec![
                    (
                        std::path::PathBuf::from("/repo/specs/a/spec.md"),
                        Err("permission denied".to_string()),
                    ),
                    (
                        std::path::PathBuf::from("/repo/specs/b/spec.md"),
                        Ok("# b\n".to_string()),
                    ),
                ],
                Err("unexpected".to_string()),
            );
            let read = |p: &std::path::Path| recorder.read(p);
            d.sync_detail(&read);

            assert_eq!(d.detail.sections.len(), 1);
            assert_eq!(d.detail.sections[0].text, "# b\n");
            assert!(
                d.detail.sections.len() <= 1,
                "artifact-folds: one surviving section is not foldable"
            );
            assert_eq!(d.detail.problems.len(), 1);
            assert!(d.detail.problems[0].contains("/repo/specs/a/spec.md"));
            assert!(d.detail.problems[0].contains("permission denied"));

            // A transient failure does not accumulate: `live-refresh`'s
            // forced reload of the SAME (unchanged) key clears the previous
            // `problems` before recording again — the key never changes
            // here, so this exercises the "forced re-read of an unchanged
            // key" clause directly, rather than smuggling a re-read in
            // through a tab move.
            d.refresh.reload = true;
            let recorder2 = RecordingReader::always(Ok("# b\n".to_string()));
            let read2 = |p: &std::path::Path| recorder2.read(p);
            d.sync_detail(&read2);
            assert!(d.detail.problems.is_empty());
        }

        #[test]
        fn an_artifact_with_no_resolved_paths_reads_nothing_at_all() {
            let mut d = dashboard_with_artifacts_named("x", &[("proposal", &[])]);
            let recorder = RecordingReader::always(Err("should not be called".to_string()));
            let read = |p: &std::path::Path| recorder.read(p);
            let dir = d.changes.active[0].dir.clone();

            d.sync_detail(&read);

            assert!(d.detail.sections.is_empty());
            assert!(d.detail.problems.is_empty());
            assert!(
                d.detail.expanded.is_empty(),
                "artifact-content: no content yet costs no fold state either"
            );
            assert_eq!(d.detail.loaded, Some((dir, 0)));
            assert_eq!(recorder.calls(), 0);
        }

        #[test]
        fn a_tab_out_of_range_for_the_newly_selected_change_is_clamped_before_the_read() {
            let five = fixture::with_artifacts(
                fixture::active("five", 0, 0),
                &[
                    ("a0", &["/repo/five/a0.md"]),
                    ("a1", &["/repo/five/a1.md"]),
                    ("a2", &["/repo/five/a2.md"]),
                    ("a3", &["/repo/five/a3.md"]),
                    ("a4", &["/repo/five/a4.md"]),
                ],
            );
            let two = fixture::with_artifacts(
                fixture::active("two", 0, 0),
                &[("b0", &["/repo/two/b0.md"]), ("b1", &["/repo/two/b1.md"])],
            );
            let mut d = Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
                searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
                changes: fixture::set(vec![five, two], Vec::new(), Vec::new()),
                route: Route::Detail,
                quit: false,
                // `list-sections`: 1, not 0 — target 0 is the active header;
                // the query below narrows `visible()` to `two` alone, whose
                // only remaining target is index 1.
                selected: 1,
                filter: empty_filter(),
                detail: Detail {
                    sections: Vec::new(),
                    scroll: 0,
                    tab: 4,
                    problems: Vec::new(),
                    loaded: None,
                    expanded: std::collections::BTreeSet::new(),
                    drawn_width: None,
                },
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            };
            let recorder = RecordingReader::always(Ok("t".to_string()));
            let read = |p: &std::path::Path| recorder.read(p);
            d.filter.query = "two".to_string();

            d.sync_detail(&read);

            assert_eq!(d.detail.tab, 1);
            assert_eq!(
                recorder.paths(),
                vec![std::path::PathBuf::from("/repo/two/b1.md")]
            );
            assert_eq!(recorder.calls(), 1);
            assert_eq!(d.detail.loaded, Some((d.changes.active[1].dir.clone(), 1)));
        }

        #[test]
        fn an_empty_visible_list_clears_the_detail() {
            let mut d = dashboard_with_artifacts_named("x", &[("proposal", &["/repo/p.md"])]);
            let recorder = RecordingReader::always(Ok("# proposal".to_string()));
            let read = |p: &std::path::Path| recorder.read(p);
            d.sync_detail(&read);
            assert!(!d.detail.sections.is_empty());
            d.detail.expanded = std::collections::BTreeSet::from([0]);

            d.filter.query = "zzz".to_string();
            d.refresh.reload = true;
            d.sync_detail(&read);

            assert!(d.detail.sections.is_empty());
            assert!(d.detail.problems.is_empty());
            assert!(
                d.detail.expanded.is_empty(),
                "artifact-content: an empty visible list clears the fold state too"
            );
            assert_eq!(d.detail.tab, 0);
            assert_eq!(d.detail.scroll, 0);
            assert_eq!(d.detail.loaded, None);
            assert!(
                !d.refresh.reload,
                "the flag must still be cleared on the empty-list early return"
            );

            // The same holds for a Dashboard built over `changes::empty_set()`.
            let mut d2 = Dashboard {
                settings: crate::settings::PanelState {
                    rows: Vec::new(),
                    cursor: 0,
                },
                selection: None,
                repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
                searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
                changes: empty_set(),
                route: Route::Detail,
                quit: false,
                selected: 0,
                filter: empty_filter(),
                detail: Detail {
                    sections: vec![ArtifactSection {
                        label: Some(String::new()),
                        text: "stale".to_string(),
                        depth: 0,
                        progress: None,
                        operation: None,
                    }],
                    scroll: 3,
                    tab: 2,
                    problems: vec!["stale problem".to_string()],
                    loaded: Some((std::path::PathBuf::from("/repo/x"), 0)),
                    expanded: std::collections::BTreeSet::from([1]),
                    drawn_width: None,
                },
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: true,
                    startup: Vec::new(),
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    stalled: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    in_flight: false,
                    problems: Vec::new(),
                },
                sections: Sections {
                    collapsed: std::collections::BTreeSet::new(),
                },
                file_mode: false,
                overlay: crate::ui::app::Overlay {
                    panel: None,
                    scroll: 0,
                    edit: None,
                },
            };
            let recorder2 = RecordingReader::always(Err("must not be called".to_string()));
            let read2 = |p: &std::path::Path| recorder2.read(p);
            d2.sync_detail(&read2);
            assert!(d2.detail.sections.is_empty());
            assert!(d2.detail.problems.is_empty());
            assert!(
                d2.detail.expanded.is_empty(),
                "artifact-content: the stale fold state over changes::empty_set() is cleared too"
            );
            assert_eq!(d2.detail.tab, 0);
            assert_eq!(d2.detail.scroll, 0);
            assert_eq!(d2.detail.loaded, None);
            assert_eq!(recorder2.calls(), 0);
            assert!(!d2.refresh.reload);
        }

        /// `text-selection`: "`selection` starts empty and is cleared rather than
        /// reloaded" — a freshly built `Dashboard` selects nothing, a tab switch that
        /// actually reloads the detail content clears a standing selection, and a
        /// `sync_detail` call that hits the cache (nothing to reload) leaves a
        /// standing selection untouched — proving the clearing is a deliberate rule
        /// `sync_detail` states, not a side effect of every call replacing `Detail`.
        /// See design.md -> Decision 4.
        #[test]
        fn selection_starts_empty_and_is_cleared_by_a_reload() {
            let mut d = dashboard_with_artifacts_named(
                "x",
                &[("proposal", &["/repo/p.md"]), ("design", &["/repo/d.md"])],
            );
            assert_eq!(
                d.selection, None,
                "a freshly built Dashboard has selected nothing"
            );

            let recorder = RecordingReader::always(Ok("# heading\ntext".to_string()));
            let read = |p: &std::path::Path| recorder.read(p);
            d.sync_detail(&read); // loads tab 0, establishing `detail.loaded`

            d.selection = Some(Selection {
                anchor: (0, 0),
                focus: (0, 3),
                granularity: Granularity::Span,
                problem: None,
            });

            // A `sync_detail` call that changes nothing — same tab, no forced
            // reload — must not clear a standing selection: the clearing is tied to
            // an actual reload, not to every call.
            d.sync_detail(&read);
            assert!(
                d.selection.is_some(),
                "a cache hit reloads nothing and must leave the selection alone"
            );

            let loaded_before = d.detail.loaded.clone();
            d.apply(Action::SelectTab(1));
            d.sync_detail(&read);

            assert_eq!(
                d.selection, None,
                "a tab switch that reloads the detail content must clear the \
                 selection, not carry it forward"
            );
            assert_ne!(
                d.detail.loaded, loaded_before,
                "the detail content must have actually reloaded, not merely hit the \
                 cache, or this test proves nothing"
            );
        }
    }
}
