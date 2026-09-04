//! `Dashboard`, `Route`, and pure key handling. See
//! `openspec/changes/tui-shell/design.md` -> Boundaries and Contracts.

use std::path::PathBuf;

use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::changes::ChangeSet;

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

/// The four outcomes a terminal event can map to. `action_for` is total over
/// every `Event`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    OpenDetail,
    BackToList,
    Ignore,
}

/// The dashboard's whole state. Carries no width, no layout mode, no column
/// count, no terminal handle, and no frame — those are derived from the
/// frame area on every draw, never stored here. Deliberately implements no
/// `Default`, anywhere in the crate: every construction and every
/// destructuring names all five fields, so a field added later fails to
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
}

impl Dashboard {
    /// Apply `action`, mutating only the field(s) it names. `Ignore` changes
    /// nothing. `BackToList` while already on `Route::List` is a no-op, so a
    /// stray `Esc` at the root cannot close the pane.
    pub fn apply(&mut self, action: Action) {
        match action {
            Action::Quit => self.quit = true,
            Action::OpenDetail => self.route = Route::Detail,
            Action::BackToList => self.route = Route::List,
            Action::Ignore => {}
        }
    }
}

/// Map a terminal event to one of the four actions. Total: every `Event`
/// value maps to something, and nothing panics. Acts only on key events
/// whose `kind` is `KeyEventKind::Press` — a `Repeat` or `Release` maps to
/// `Ignore`, so a terminal that reports release events does not quit twice
/// or navigate on the release of a key already handled on its press.
pub fn action_for(event: &Event) -> Action {
    let Event::Key(key) = event else {
        return Action::Ignore;
    };
    if key.kind != KeyEventKind::Press {
        return Action::Ignore;
    }
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), KeyModifiers::NONE) => Action::Quit,
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Action::Quit,
        (KeyCode::Enter, KeyModifiers::NONE) => Action::OpenDetail,
        (KeyCode::Esc, KeyModifiers::NONE) => Action::BackToList,
        _ => Action::Ignore,
    }
}

#[cfg(test)]
mod tests {
    mod keys {
        use ratatui::crossterm::event::{
            Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
        };

        use crate::changes::ChangeSet;
        use crate::ui::app::{Action, Dashboard, Route, action_for};

        fn empty_changes() -> ChangeSet {
            ChangeSet {
                active: Vec::new(),
                archived: Vec::new(),
                problems: Vec::new(),
            }
        }

        fn dashboard_at(route: Route) -> Dashboard {
            Dashboard {
                repo: None,
                searched_from: std::path::PathBuf::from("/tmp/does-not-matter"),
                changes: empty_changes(),
                route,
                quit: false,
            }
        }

        fn press(code: KeyCode, modifiers: KeyModifiers) -> Event {
            Event::Key(KeyEvent::new(code, modifiers))
        }

        #[test]
        fn quit_keys_and_their_near_misses() {
            assert_eq!(
                action_for(&press(KeyCode::Char('q'), KeyModifiers::NONE)),
                Action::Quit
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('c'), KeyModifiers::CONTROL)),
                Action::Quit
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('Q'), KeyModifiers::SHIFT)),
                Action::Ignore
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('q'), KeyModifiers::CONTROL)),
                Action::Ignore
            );
            assert_eq!(
                action_for(&press(KeyCode::Char('c'), KeyModifiers::NONE)),
                Action::Ignore
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
            assert_eq!(action_for(&released), Action::Ignore);
            assert_eq!(action_for(&repeated), Action::Ignore);
            assert_eq!(action_for(&pressed), Action::Quit);
        }

        #[test]
        fn enter_and_esc_map_to_routes() {
            assert_eq!(
                action_for(&press(KeyCode::Enter, KeyModifiers::NONE)),
                Action::OpenDetail
            );
            assert_eq!(
                action_for(&press(KeyCode::Esc, KeyModifiers::NONE)),
                Action::BackToList
            );
            assert_eq!(
                action_for(&press(KeyCode::Enter, KeyModifiers::CONTROL)),
                Action::Ignore
            );
        }

        #[test]
        fn non_key_events_are_ignored() {
            assert_eq!(action_for(&Event::Resize(60, 20)), Action::Ignore);
            assert_eq!(action_for(&Event::FocusGained), Action::Ignore);
            assert_eq!(action_for(&Event::FocusLost), Action::Ignore);
            assert_eq!(
                action_for(&Event::Paste("q".to_string())),
                Action::Ignore,
                "a paste of the single character q must not quit"
            );
            let mouse = Event::Mouse(MouseEvent {
                kind: MouseEventKind::Moved,
                column: 0,
                row: 0,
                modifiers: KeyModifiers::NONE,
            });
            assert_eq!(action_for(&mouse), Action::Ignore);
        }

        #[test]
        fn action_for_is_total_over_a_keycode_sweep() {
            let mapped = [KeyCode::Enter, KeyCode::Esc, KeyCode::Char('q')];
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
                let action = action_for(&press(code, KeyModifiers::NONE));
                if !mapped.contains(&code) {
                    assert_eq!(action, Action::Ignore, "{code:?} should be ignored");
                }
            }
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
            dashboard.apply(Action::BackToList);
            assert_eq!(dashboard.route, Route::List);
            assert!(!dashboard.quit);
        }

        #[test]
        fn back_to_list_at_list_is_a_no_op() {
            let mut dashboard = dashboard_at(Route::List);
            dashboard.apply(Action::BackToList);
            assert_eq!(dashboard.route, Route::List);
            assert!(!dashboard.quit);
            dashboard.apply(Action::BackToList);
            assert_eq!(dashboard.route, Route::List);
            assert!(!dashboard.quit);
        }

        #[test]
        fn dashboard_destructures_into_exactly_five_fields() {
            let d = dashboard_at(Route::List);
            let Dashboard {
                repo,
                searched_from,
                changes,
                route,
                quit,
            } = &d;
            assert_eq!(*repo, None);
            assert_eq!(
                searched_from,
                &std::path::PathBuf::from("/tmp/does-not-matter")
            );
            assert_eq!(changes.active.len(), 0);
            assert_eq!(*route, Route::List);
            assert!(!*quit);
        }
    }
}
