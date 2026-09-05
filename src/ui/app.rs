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

/// The nine outcomes a terminal event can map to, under either filter mode.
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

/// The dashboard's whole state. Carries no width, no layout mode, no column
/// count, no terminal handle, and no frame — those are derived from the
/// frame area on every draw, never stored here. Deliberately implements no
/// `Default`, anywhere in the crate: every construction and every
/// destructuring names all eight fields, so a field added later fails to
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
    /// The index into the *visible* list — `visible()`'s output, active
    /// changes then archived, with the filter query applied. See
    /// `specs/list-selection/spec.md`.
    pub selected: usize,
    /// The `/` filter's mode and query. See `specs/list-filtering/spec.md`.
    pub filter: Filter,
    /// The detail region's markdown source and scroll offset. Nothing reads
    /// or writes it in this change beyond startup, which leaves it empty and
    /// unscrolled. See `specs/detail-scroll/spec.md`.
    pub detail: Detail,
}

impl Dashboard {
    /// Apply `action`, mutating only the field(s) it names. `Ignore` changes
    /// nothing.
    pub fn apply(&mut self, action: Action) {
        match action {
            Action::Quit => self.quit = true,
            Action::OpenDetail => {
                if self.filter.active {
                    self.filter.active = false;
                } else {
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
                    self.selected = self.selected.saturating_add(1);
                    self.clamp_selection();
                }
                Route::Detail => {
                    self.detail.scroll = self.detail.scroll.saturating_add(1);
                }
            },
            Action::Prev => match self.route {
                Route::List => {
                    self.selected = self.selected.saturating_sub(1);
                    self.clamp_selection();
                }
                Route::Detail => {
                    self.detail.scroll = self.detail.scroll.saturating_sub(1);
                }
            },
            // Nothing yet: group 7 gives these their effect. Kept as
            // explicit no-op arms rather than folded into a wildcard, so
            // `apply`'s match stays exhaustive without `_` and a later
            // variant added to `Action` fails to compile here instead of
            // silently falling through.
            Action::SelectTab(_) => {}
            Action::NextTab => {}
            Action::PrevTab => {}
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
            Action::Ignore => {}
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
        let total = crate::ui::markdown::lines(&self.detail.source, interior.width).len();
        self.detail.scroll =
            crate::ui::layout::scroll_offset(total, self.detail.scroll, interior.height);
    }

    /// Active-then-archived, in `ChangeSet`'s own order, with `list-filtering`'s
    /// case-insensitive substring query applied to `Change::name`. What
    /// `change-rows`' `rows` also emits, and what `selected` indexes into.
    pub fn visible(&self) -> Vec<&Change> {
        self.changes
            .active
            .iter()
            .chain(self.changes.archived.iter())
            .filter(|c| matches(&c.name, &self.filter.query))
            .collect()
    }

    /// `visible().len()`, without building the intermediate `Vec` twice at
    /// call sites that only need the count.
    pub fn visible_len(&self) -> usize {
        self.visible().len()
    }

    /// Keep `selected` addressing a change that is actually shown: `0` when
    /// the visible list is empty, otherwise clamped to its last index. Never
    /// *raises* `selected` — clamping shrinks the index and never restores
    /// it once the list grows back.
    fn clamp_selection(&mut self) {
        let len = self.visible_len();
        if len == 0 {
            self.selected = 0;
        } else if self.selected >= len {
            self.selected = len - 1;
        }
    }
}

/// A change is visible when its name, ASCII-lowercased, contains `query`,
/// ASCII-lowercased. An empty query matches every change. See
/// `specs/list-filtering/spec.md` -> "The query is a case-insensitive
/// substring match on the change name".
pub fn matches(name: &str, query: &str) -> bool {
    name.to_ascii_lowercase()
        .contains(query.to_ascii_lowercase().as_str())
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
        (KeyCode::Enter, KeyModifiers::NONE) => Action::OpenDetail,
        (KeyCode::Esc, KeyModifiers::NONE) => Action::Back,
        _ => Action::Ignore,
    }
}

#[cfg(test)]
mod tests {
    mod keys {
        use ratatui::crossterm::event::{
            Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
        };

        use crate::changes::empty_set;
        use crate::changes::fixture;
        use crate::ui::app::{Action, Dashboard, Detail, Filter, Route, action_for};

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
            }
        }

        fn press(code: KeyCode, modifiers: KeyModifiers) -> Event {
            Event::Key(KeyEvent::new(code, modifiers))
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
                action_for(&press(KeyCode::Char('c'), KeyModifiers::NONE), false),
                Action::Ignore
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
            d.apply(Action::Back);
            assert_eq!(d.route, Route::List);
            assert!(!d.quit);
            assert_eq!(d.detail.scroll, 0);
            d.apply(Action::Back);
            assert_eq!(d.route, Route::List);
            assert!(!d.quit);
            assert_eq!(d.detail.scroll, 0);
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
            d.changes.archived.clear();
            for _ in 0..4 {
                d.apply(Action::Next);
            }
            assert_eq!(d.selected, 2);
            for _ in 0..4 {
                d.apply(Action::Prev);
            }
            assert_eq!(d.selected, 0);
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
            d.detail = twenty_line_detail();
            d.route = Route::List;
            d.apply(Action::Next);
            d.apply(Action::Next);
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
            };
            d3.apply(Action::Back);
            assert_eq!(d3.detail.scroll, 3);
        }

        #[test]
        fn normalise_scroll_clamps_against_the_frame() {
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
            };
            d.normalise_scroll(ratatui::layout::Rect::new(0, 0, 120, 20));
            assert_eq!(d.detail.scroll, 4);

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
            };
            d2.normalise_scroll(ratatui::layout::Rect::new(0, 0, 60, 20));
            assert_eq!(d2.detail.scroll, 4);
        }

        #[test]
        fn normalise_scroll_is_inert_when_the_detail_region_is_not_drawn() {
            let mut d = Dashboard {
                detail: Detail {
                    source: twenty_line_detail().source,
                    scroll: 7,
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
            };
            d.normalise_scroll(ratatui::layout::Rect::new(0, 0, 60, 20));
            assert_eq!(
                d.detail.scroll, 7,
                "narrow list route: not drawn, unchanged"
            );
            d.normalise_scroll(ratatui::layout::Rect::new(0, 0, 120, 20));
            assert_eq!(
                d.detail.scroll, 4,
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
            assert_eq!(d.selected, 1);
            for _ in 0..3 {
                d.apply(Action::FilterPop);
            }
            assert_eq!(d.visible_len(), 5);
            assert_eq!(d.selected, 1);
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
        fn dashboard_destructures_into_exactly_eight_fields() {
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
        }
    }
}
