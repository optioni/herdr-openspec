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

/// The seventeen outcomes a terminal event can map to, under either filter mode. The count
/// has moved twice since this comment was last true: to thirteen with `live-refresh`'s
/// `Refresh`, and to seventeen with `agent-launch`'s `LaunchApply`, `LaunchContinue`,
/// `LaunchArchive`, and `FocusAgent`.
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
    /// `agent-launch`'s additions: launch the selected change with
    /// `/opsx:apply`, `/opsx:continue`, or `/opsx:archive`, or focus the
    /// agent already attributed to it. Four flat variants rather than one
    /// carrying an `Intent`, so a hand-written enumeration cannot silently
    /// omit one. Route-agnostic in the same stronger sense as `Refresh`:
    /// they act on the selected change, which is the same change at either
    /// route.
    LaunchApply,
    LaunchContinue,
    LaunchArchive,
    FocusAgent,
    Ignore,
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

/// The detail region's markdown source and scroll offset, plus
/// `detail-view`'s three additions. `source` is set by `Dashboard::sync_detail`,
/// driven once per loop iteration by the injected `ArtifactReader`; `ui::load`
/// still starts it empty. `scroll` is a user-controlled position, the detail
/// region's counterpart to `Dashboard::selected`, not derived geometry: the
/// offset actually drawn is still recomputed on every draw by
/// `layout::scroll_offset` against the current content area's height. `tab`
/// is the selected artifact's position in the selected change's `artifacts`;
/// `problems` names each artifact file that could not be read; `loaded` is
/// the `(change directory, tab)` key whose content `source` currently holds
/// — `sync_detail`'s cache key, and the reason an unchanged selection
/// re-reads nothing. Deliberately implements no `Default`, anywhere in the
/// crate, on the same terms as `Dashboard` and `Filter`: every construction
/// and destructuring names all five fields, with no `..` rest. See
/// `specs/detail-scroll/spec.md`, `specs/artifact-tabs/spec.md`,
/// `specs/artifact-content/spec.md`, and the `NODEFAULT-UI` check, whose type
/// list covers this type too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detail {
    pub source: String,
    pub scroll: usize,
    pub tab: usize,
    pub problems: Vec<String>,
    pub loaded: Option<(PathBuf, usize)>,
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

/// `list-sections`' addition: one addressable row the cursor can land on —
/// either a section header or a change, the latter as an index into
/// `Dashboard::visible()`. `Dashboard::targets()` returns them in emission
/// order, and `Dashboard::selected` indexes into that vector rather than
/// into `visible()` directly, because a collapsed section's header is its
/// only row and must stay reachable. See design.md -> Decision 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Section(SectionKey),
    /// An index into `Dashboard::visible()`.
    Change(usize),
}

/// The dashboard's whole state. Carries no width, no layout mode, no column
/// count, no terminal handle, and no frame — those are derived from the
/// frame area on every draw, never stored here. Deliberately implements no
/// `Default`, anywhere in the crate: every construction and every
/// destructuring names all fourteen fields, so a field added later fails to
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
}

impl Dashboard {
    /// Apply `action`, mutating only the field(s) it names. `Ignore` changes
    /// nothing.
    pub fn apply(&mut self, action: Action) {
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
            // detail scroll at `Route::Detail`, never both. The scroll's
            // upper bound is enforced by the draw-time clamp
            // (`render_detail`) and the frame normalisation
            // (`normalise_scroll`), not here — `saturating_add` alone would
            // let a held key run the stored value arbitrarily far ahead.
            Action::Next => match self.route {
                Route::List => {
                    let before = self.selected;
                    self.selected = self.selected.saturating_add(1);
                    self.clamp_selection();
                    if self.selected != before {
                        self.detail.tab = 0;
                        self.detail.scroll = 0;
                    }
                }
                Route::Detail => {
                    self.detail.scroll = self.detail.scroll.saturating_add(1);
                }
            },
            Action::Prev => match self.route {
                Route::List => {
                    let before = self.selected;
                    self.selected = self.selected.saturating_sub(1);
                    self.clamp_selection();
                    if self.selected != before {
                        self.detail.tab = 0;
                        self.detail.scroll = 0;
                    }
                }
                Route::Detail => {
                    self.detail.scroll = self.detail.scroll.saturating_sub(1);
                }
            },
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
            Action::Ignore => {}
        }
    }

    /// The one place all four launch actions gather the same five values — the selected
    /// change's name, the focus pane `attribution().panes` holds for it, whether the socket is
    /// reachable, and the live agents' names — and hand them to `launch::decide`.
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
        let (_, body, _) = crate::ui::layout::split_frame(frame_area);
        let (_, detail_area) = crate::ui::layout::split_body(body, self.route);
        let Some(area) = detail_area else {
            return;
        };
        let interior = crate::ui::layout::interior(area);
        if interior.width == 0 || interior.height == 0 {
            return;
        }
        let (_, _, content) = crate::ui::layout::split_detail(interior);
        if content.width == 0 || content.height == 0 {
            return;
        }
        let total =
            crate::ui::detail::content_lines(&self.detail, self.selected_change(), content.width)
                .len();
        self.detail.scroll =
            crate::ui::layout::scroll_offset(total, self.detail.scroll, content.height);
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
        let Some((dir, tab, paths)) = self.selected_change().map(|change| {
            let count = change.artifacts.len();
            let tab = self.detail.tab.min(count.saturating_sub(1));
            let paths = change
                .artifacts
                .get(tab)
                .map(|a| a.paths.clone())
                .unwrap_or_default();
            (change.dir.clone(), tab, paths)
        }) else {
            // Step 1: nothing selected.
            self.detail.source.clear();
            self.detail.problems.clear();
            self.detail.tab = 0;
            self.detail.scroll = 0;
            self.detail.loaded = None;
            return;
        };
        self.detail.tab = tab; // step 2
        let key = (dir, tab);
        let key_changed = self.detail.loaded.as_ref() != Some(&key);
        if !key_changed && !forced {
            return; // step 3
        }
        // Step 4: re-read every path, concatenating in order.
        self.detail.problems.clear();
        self.detail.source.clear();
        for path in &paths {
            match read(path) {
                Ok(text) => {
                    if !self.detail.source.is_empty() && !self.detail.source.ends_with('\n') {
                        self.detail.source.push('\n');
                    }
                    self.detail.source.push_str(&text);
                }
                Err(e) => {
                    self.detail
                        .problems
                        .push(format!("{}: {e}", path.display()));
                }
            }
        }
        // Step 5: the scroll resets on a key change only, never on a
        // forced-but-unchanged-key reload.
        if key_changed {
            self.detail.scroll = 0;
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

/// Map a terminal event and the current filter mode to one of the nine
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
    use crate::ui::app::{Action, Dashboard, Detail, Filter, Refresh, Route, Sections, action_for};
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
                source: String::new(),
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
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
                source: String::new(),
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
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

    mod keys {
        use ratatui::crossterm::event::{
            Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
        };

        use crate::changes::empty_set;
        use crate::changes::fixture;
        use crate::testutil::RecordingReader;
        use crate::ui::app::{
            Action, Dashboard, Detail, Filter, Route, SectionKey, Sections, Target, action_for,
        };

        fn empty_filter() -> Filter {
            Filter {
                query: String::new(),
                active: false,
            }
        }

        fn empty_detail() -> Detail {
            Detail {
                source: String::new(),
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
            }
        }

        fn dashboard_at(route: Route) -> Dashboard {
            Dashboard {
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
            }
        }

        /// Three active changes, two archived — the fixture `list.rs` and
        /// `view.rs`'s tests reuse verbatim.
        fn five_change_dashboard() -> Dashboard {
            Dashboard {
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
                repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
                searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
                changes: fixture::set(vec![change], Vec::new(), Vec::new()),
                route: Route::Detail,
                quit: false,
                // `list-sections`: 1, not 0 — target 0 is the active header.
                selected: 1,
                filter: empty_filter(),
                detail: Detail {
                    source: String::new(),
                    scroll,
                    tab,
                    problems: Vec::new(),
                    loaded: None,
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
                Action::Ignore,
            ];
            assert_eq!(
                variants.len(),
                17,
                "the seventeen variants this crate specifies"
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
                detail: Detail {
                    source: "## 1. Setup\n- [x] a\n- [ ] b\n".to_string(),
                    scroll: 0,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
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

        fn twenty_line_detail() -> Detail {
            Detail {
                source: (0..20).map(|i| format!("- line-{i:02}\n")).collect(),
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
            }
        }

        #[test]
        fn detail_destructures_into_exactly_five_fields() {
            let d = twenty_line_detail();
            let Detail {
                source,
                scroll,
                tab,
                problems,
                loaded,
            } = &d;
            assert!(source.starts_with("- line-00"));
            assert_eq!(*scroll, 0);
            assert_eq!(*tab, 0);
            assert!(problems.is_empty());
            assert_eq!(*loaded, None);
        }

        #[test]
        fn next_and_prev_scroll_at_the_detail_route() {
            let mut d = Dashboard {
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
            };
            for _ in 0..4 {
                d.apply(Action::Prev);
                assert_eq!(d.detail.scroll, 0);
            }
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
                detail: Detail {
                    source: twenty_line_detail().source,
                    scroll: 3,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
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
            };
            d.apply(Action::Back);
            assert_eq!(d.detail.scroll, 0);
            d.apply(Action::OpenDetail);
            assert_eq!(d.detail.scroll, 0);

            let mut d2 = Dashboard {
                detail: Detail {
                    source: twenty_line_detail().source,
                    scroll: 3,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
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
            };
            d2.apply(Action::FilterStart);
            assert_eq!(d2.route, Route::List);
            assert_eq!(d2.detail.scroll, 0);

            let mut d3 = Dashboard {
                detail: Detail {
                    source: twenty_line_detail().source,
                    scroll: 3,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
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
            };
            d3.apply(Action::Back);
            assert_eq!(d3.detail.scroll, 3);
        }

        /// The `Dashboard` shape shared by this group's three `Enter`
        /// scenarios below, factored out in the REFACTOR step: the same
        /// twenty-line detail source as `twenty_line_detail`, an explicit
        /// `route`, `filter`, and `detail.scroll`. Carries one real,
        /// selected change — `ui::view::render_detail` draws nothing at all
        /// when `selected_change()` is `None`, so an empty `ChangeSet` would
        /// make every rendered-buffer assertion below vacuous regardless of
        /// `detail.source` or `detail.scroll`.
        fn dashboard_for_enter(route: Route, filter: Filter, scroll: usize) -> Dashboard {
            Dashboard {
                detail: Detail {
                    source: twenty_line_detail().source,
                    scroll,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
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
                (41, 9)
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
            assert_eq!(detail_marker_cols(&buf_after_60, 4), "- line-03");
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
                assert_eq!(detail_marker_cols(&buf, 4), "- line-00", "width {width}");
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
                detail: Detail {
                    source: twenty_line_detail().source,
                    scroll: 99,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
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
            };
            d.normalise_scroll(ratatui::layout::Rect::new(0, 0, 120, 20));
            assert_eq!(d.detail.scroll, 6);

            let mut d2 = Dashboard {
                detail: Detail {
                    source: twenty_line_detail().source,
                    scroll: 99,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
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
            };
            d2.normalise_scroll(ratatui::layout::Rect::new(0, 0, 60, 20));
            assert_eq!(d2.detail.scroll, 6);

            let mut d3 = Dashboard {
                detail: Detail {
                    source: twenty_line_detail().source,
                    scroll: 99,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
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
                    detail: Detail {
                        source,
                        scroll: 99,
                        tab: 0,
                        problems: Vec::new(),
                        loaded: None,
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
                detail: Detail {
                    source: twenty_line_detail().source,
                    scroll: 9,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
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
                    source: "stale".to_string(),
                    scroll: 5,
                    tab: 2,
                    problems: Vec::new(),
                    loaded: None,
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
                    source: "stale".to_string(),
                    scroll: 6,
                    tab: 2,
                    problems: Vec::new(),
                    loaded: None,
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
        fn dashboard_destructures_into_exactly_fourteen_fields() {
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
                    source: String::new(),
                    scroll: 9,
                    tab: 2,
                    problems: Vec::new(),
                    loaded: None,
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
            };
            d.apply(Action::Next);
            assert_eq!(d.selected, 1);
            assert_eq!(d.detail.tab, 0);
            assert_eq!(d.detail.scroll, 0);

            let mut d2 = Dashboard {
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
                    source: String::new(),
                    scroll: 9,
                    tab: 2,
                    problems: Vec::new(),
                    loaded: None,
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
                repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
                searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
                changes: fixture::set(vec![change], Vec::new(), Vec::new()),
                route: Route::Detail,
                quit: false,
                // `list-sections`: 1, not 0 — target 0 is the active header.
                selected: 1,
                filter: empty_filter(),
                detail: Detail {
                    source: String::new(),
                    scroll: 0,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
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

            assert_eq!(d.detail.source, "# proposal");
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
            assert_eq!(d.detail.source, "# a much longer proposal now");
            assert_eq!(
                d.detail.scroll, 6,
                "a forced reload must not move the scroll"
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

            d.detail.tab = 1;
            d.refresh.reload = true;
            let second = RecordingReader::always(Ok("# design".to_string()));
            let read_second = |p: &std::path::Path| second.read(p);
            d.sync_detail(&read_second);

            assert_eq!(second.calls(), 1);
            assert_eq!(second.paths(), vec![std::path::PathBuf::from("/repo/d.md")]);
            assert_eq!(d.detail.source, "# design");
            assert_eq!(
                d.detail.scroll, 0,
                "the tab move, not the forced flag, must reset the scroll"
            );
            assert!(!d.refresh.reload);
            assert_eq!(d.detail.loaded, Some((dir, 1)));
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
                    source: String::new(),
                    scroll: 0,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
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
            assert_eq!(d.detail.source, "text");
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
                    source: String::new(),
                    scroll: 0,
                    tab: 0,
                    problems: Vec::new(),
                    loaded: None,
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
            assert_eq!(d.detail.source, "ARCHIVED");
        }

        #[test]
        fn a_multi_file_artifact_is_concatenated_in_path_order_with_a_separating_newline() {
            let mut d = dashboard_with_artifacts_named(
                "x",
                &[("specs", &["/repo/specs/a/spec.md", "/repo/specs/b/spec.md"])],
            );
            let recorder = RecordingReader::new(
                vec![
                    (
                        std::path::PathBuf::from("/repo/specs/a/spec.md"),
                        Ok("# a".to_string()),
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
            assert_eq!(d.detail.source, "# a\n# b\n");

            // The preceding file already ending in a newline gains no
            // second one.
            let mut d2 = dashboard_with_artifacts_named(
                "y",
                &[("specs", &["/repo/specs/a/spec.md", "/repo/specs/b/spec.md"])],
            );
            let recorder2 = RecordingReader::new(
                vec![
                    (
                        std::path::PathBuf::from("/repo/specs/a/spec.md"),
                        Ok("# a\n".to_string()),
                    ),
                    (
                        std::path::PathBuf::from("/repo/specs/b/spec.md"),
                        Ok("# b\n".to_string()),
                    ),
                ],
                Err("unexpected".to_string()),
            );
            let read2 = |p: &std::path::Path| recorder2.read(p);
            d2.sync_detail(&read2);
            assert_eq!(d2.detail.source, "# a\n# b\n");
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

            assert_eq!(d.detail.source, "# b\n");
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

            assert!(d.detail.source.is_empty());
            assert!(d.detail.problems.is_empty());
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
                    source: String::new(),
                    scroll: 0,
                    tab: 4,
                    problems: Vec::new(),
                    loaded: None,
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
            assert!(!d.detail.source.is_empty());

            d.filter.query = "zzz".to_string();
            d.refresh.reload = true;
            d.sync_detail(&read);

            assert!(d.detail.source.is_empty());
            assert!(d.detail.problems.is_empty());
            assert_eq!(d.detail.tab, 0);
            assert_eq!(d.detail.scroll, 0);
            assert_eq!(d.detail.loaded, None);
            assert!(
                !d.refresh.reload,
                "the flag must still be cleared on the empty-list early return"
            );

            // The same holds for a Dashboard built over `changes::empty_set()`.
            let mut d2 = Dashboard {
                repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
                searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
                changes: empty_set(),
                route: Route::Detail,
                quit: false,
                selected: 0,
                filter: empty_filter(),
                detail: Detail {
                    source: "stale".to_string(),
                    scroll: 3,
                    tab: 2,
                    problems: vec!["stale problem".to_string()],
                    loaded: Some((std::path::PathBuf::from("/repo/x"), 0)),
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
            };
            let recorder2 = RecordingReader::always(Err("must not be called".to_string()));
            let read2 = |p: &std::path::Path| recorder2.read(p);
            d2.sync_detail(&read2);
            assert!(d2.detail.source.is_empty());
            assert!(d2.detail.problems.is_empty());
            assert_eq!(d2.detail.tab, 0);
            assert_eq!(d2.detail.scroll, 0);
            assert_eq!(d2.detail.loaded, None);
            assert_eq!(recorder2.calls(), 0);
            assert!(!d2.refresh.reload);
        }
    }
}
