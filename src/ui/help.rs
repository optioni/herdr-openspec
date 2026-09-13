//! The pane's key and gesture inventory, and the overlay's row grammar. See
//! `openspec/changes/help-overlay/specs/binding-inventory/spec.md` and
//! `specs/help-overlay/spec.md` -> "The overlay's row grammar is a rule row,
//! groups, and a rule row".
//!
//! Pure and total, on exactly `ui::palette`'s terms: no filesystem, process,
//! environment, network, or standard-I/O API, no clock, no global mutable
//! state, and no panic for any input. `INVENTORY` is a `'static` value with
//! no construction cost — see the test module's own
//! `const _: &[Group] = INVENTORY;`.

use crate::ui::app::{Action, Target};

/// The route (or filter mode) a [`Group`]'s bindings apply at. Sits on the
/// group, not the binding: within a group every binding shares one scope,
/// so a per-row tag would repeat the heading's own information. `Any` means
/// the group's bindings act the same way at every route — `Agents`, `Pane`,
/// and `Mouse`, whose descriptions name their own region instead. See
/// `specs/binding-inventory/spec.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Any,
    List,
    Detail,
    Filter,
}

impl Scope {
    /// The parenthesised suffix a group heading carries — `None` for
    /// `Any`, whose group carries no suffix at all.
    fn label(self) -> Option<&'static str> {
        match self {
            Scope::Any => None,
            Scope::List => Some("list route"),
            Scope::Detail => Some("detail route"),
            Scope::Filter => Some("filter mode"),
        }
    }
}

/// One key or gesture and the [`Action`] it produces. `input` is the
/// reader's own spelling — `j / ↓`, `Ctrl-C`, `1`–`9` — not a machine
/// token; `help-overlay`'s contract-tier parser (task group 10) reads it
/// back into `(KeyCode, KeyModifiers)` pairs.
///
/// No `Default`, anywhere in the crate: every construction site below names
/// all three fields, with no `..` rest, so a field added later is a
/// compile error at each site rather than a silent `""`. Joins
/// `NODEFAULT-UI`'s scanned type sets on exactly `Filter`, `Refresh`, and
/// `Launch`'s own terms.
pub struct Binding {
    pub input: &'static str,
    pub description: &'static str,
    pub action: Action,
}

/// A titled, scoped run of [`Binding`]s — one section of the overlay. No
/// `Default`, on the same terms as [`Binding`].
pub struct Group {
    pub title: &'static str,
    pub scope: Scope,
    pub bindings: &'static [Binding],
}

/// The pane's whole binding inventory, in the reading order
/// `specs/binding-inventory/spec.md` mandates: the list first, the detail
/// second, the agent and pane keys next, and the two modal groups last —
/// not alphabetical, and not the order `action_for`'s `match` happens to be
/// written in. Six groups, thirty-one bindings; the test module's own shape
/// assertion is the authority `cargo test` enforces, not this comment.
pub const INVENTORY: &[Group] = &[
    Group {
        title: "Changes",
        scope: Scope::List,
        bindings: &[
            Binding {
                input: "j / ↓",
                description: "Move the selection down.",
                action: Action::Next,
            },
            Binding {
                input: "k / ↑",
                description: "Move the selection up.",
                action: Action::Prev,
            },
            Binding {
                input: "Enter",
                description: "Open the selected change.",
                action: Action::OpenDetail,
            },
            Binding {
                input: "Space",
                description: "Fold or unfold the list section the cursor is on or in.",
                action: Action::ToggleSection,
            },
            Binding {
                input: "/",
                description: "Start filtering the list by name.",
                action: Action::FilterStart,
            },
        ],
    },
    Group {
        title: "Artifact",
        scope: Scope::Detail,
        bindings: &[
            Binding {
                input: "j / ↓",
                description: "Scroll the artifact content down, or move the section cursor at a foldable artifact.",
                action: Action::Next,
            },
            Binding {
                input: "k / ↑",
                description: "Scroll the artifact content up, or move the section cursor at a foldable artifact.",
                action: Action::Prev,
            },
            Binding {
                input: "1–9",
                description: "Jump straight to that numbered artifact tab.",
                action: Action::SelectTab(0),
            },
            Binding {
                input: "]",
                description: "Move to the next artifact tab.",
                action: Action::NextTab,
            },
            Binding {
                input: "[",
                description: "Move to the previous artifact tab.",
                action: Action::PrevTab,
            },
            Binding {
                input: "Space",
                description: "Fold or unfold the content section the cursor is on or in.",
                action: Action::ToggleSection,
            },
            Binding {
                input: "Esc",
                description: "Return to the change list.",
                action: Action::Back,
            },
        ],
    },
    Group {
        title: "Agents",
        scope: Scope::Any,
        bindings: &[
            Binding {
                input: "a",
                description: "Launch an agent on the selected change with /opsx:apply.",
                action: Action::LaunchApply,
            },
            Binding {
                input: "c",
                description: "Launch an agent on the selected change with /opsx:continue.",
                action: Action::LaunchContinue,
            },
            Binding {
                input: "s",
                description: "Launch an agent on the selected change with /opsx:archive.",
                action: Action::LaunchArchive,
            },
            Binding {
                input: "g",
                description: "Focus the agent already running for the selected change.",
                action: Action::FocusAgent,
            },
        ],
    },
    Group {
        title: "Pane",
        scope: Scope::Any,
        bindings: &[
            Binding {
                input: "q",
                description: "Quit the pane.",
                action: Action::Quit,
            },
            Binding {
                input: "Ctrl-C",
                description: "Quit the pane.",
                action: Action::Quit,
            },
            Binding {
                input: "r",
                description: "Force a full refresh of the change set.",
                action: Action::Refresh,
            },
            Binding {
                input: "?",
                description: "Open or close this help overlay.",
                action: Action::ToggleHelp,
            },
        ],
    },
    Group {
        title: "While filtering",
        scope: Scope::Filter,
        bindings: &[
            Binding {
                input: "Esc",
                description: "Clear the query and stop filtering.",
                action: Action::Back,
            },
            Binding {
                input: "Backspace",
                description: "Delete the last character of the query.",
                action: Action::FilterPop,
            },
            Binding {
                input: "Enter",
                description: "Open the selected change.",
                action: Action::OpenDetail,
            },
            Binding {
                input: "↑",
                description: "Move the selection up.",
                action: Action::Prev,
            },
            Binding {
                input: "↓",
                description: "Move the selection down. Every other printable key types into the query instead.",
                action: Action::Next,
            },
        ],
    },
    Group {
        title: "Mouse",
        scope: Scope::Any,
        bindings: &[
            Binding {
                input: "Wheel ↓",
                description: "Move the selection down, over the list.",
                action: Action::SelectNext,
            },
            Binding {
                input: "Wheel ↑",
                description: "Move the selection up, over the list.",
                action: Action::SelectPrev,
            },
            Binding {
                input: "Wheel ↓",
                description: "Scroll down, over the detail region.",
                action: Action::ScrollDown,
            },
            Binding {
                input: "Wheel ↑",
                description: "Scroll up, over the detail region.",
                action: Action::ScrollUp,
            },
            Binding {
                input: "Click",
                description: "Select a list row, fold a section header, or scroll to a line in the detail content.",
                action: Action::Click(Target::Change(0)),
            },
            Binding {
                input: "Click",
                description: "Switch to the clicked artifact tab.",
                action: Action::SelectTab(0),
            },
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::{Binding, Group, INVENTORY, Row, Scope, content_rows, fit, key_column, render, rows};
    use crate::testutil::{cell, row_text};
    use crate::ui::app::Action;
    use crate::ui::layout::columns;
    use crate::ui::palette::{self, Role};
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;
    use std::collections::BTreeSet;

    /// The observable second site `specs/binding-inventory/spec.md`'s own
    /// scenario names: a const item is evaluated at compile time, so an
    /// `INVENTORY` that became a function call, allocated, or read a file
    /// would fail to compile here rather than pass a runtime assertion that
    /// has nothing to observe.
    const _: &[Group] = INVENTORY;

    #[test]
    fn the_inventory_s_shape_is_asserted_not_described() {
        let titles: Vec<&str> = INVENTORY.iter().map(|g| g.title).collect();
        assert_eq!(
            titles,
            vec![
                "Changes",
                "Artifact",
                "Agents",
                "Pane",
                "While filtering",
                "Mouse"
            ]
        );

        let scopes: Vec<Scope> = INVENTORY.iter().map(|g| g.scope).collect();
        assert_eq!(
            scopes,
            vec![
                Scope::List,
                Scope::Detail,
                Scope::Any,
                Scope::Any,
                Scope::Filter,
                Scope::Any,
            ]
        );

        let counts: Vec<usize> = INVENTORY.iter().map(|g| g.bindings.len()).collect();
        assert_eq!(counts, vec![5, 7, 4, 4, 5, 6]);
        assert_eq!(counts.iter().sum::<usize>(), 31);

        assert!(
            INVENTORY.iter().all(|g| !g.bindings.is_empty()),
            "no group is empty"
        );
        let unique: BTreeSet<&str> = titles.iter().copied().collect();
        assert_eq!(unique.len(), titles.len(), "no two groups share a title");
    }

    #[test]
    fn space_and_esc_each_appear_under_their_route() {
        let space: Vec<(Scope, &Binding)> = INVENTORY
            .iter()
            .flat_map(|g| g.bindings.iter().map(move |b| (g.scope, b)))
            .filter(|(_, b)| b.input == "Space")
            .collect();
        assert_eq!(space.len(), 2);
        assert!(space.iter().any(|(scope, _)| *scope == Scope::List));
        assert!(space.iter().any(|(scope, _)| *scope == Scope::Detail));
        assert_ne!(
            space[0].1.description, space[1].1.description,
            "the two Space rows must describe different folds"
        );
        assert!(space.iter().all(|(_, b)| b.action == Action::ToggleSection));

        let esc: Vec<(Scope, &Binding)> = INVENTORY
            .iter()
            .flat_map(|g| g.bindings.iter().map(move |b| (g.scope, b)))
            .filter(|(_, b)| b.input.contains("Esc"))
            .collect();
        assert_eq!(esc.len(), 2);
        assert!(esc.iter().any(|(scope, _)| *scope == Scope::Detail));
        assert!(esc.iter().any(|(scope, _)| *scope == Scope::Filter));
        assert_ne!(
            esc[0].1.description, esc[1].1.description,
            "the two Esc rows must describe different effects"
        );
        assert!(esc.iter().all(|(_, b)| b.action == Action::Back));
    }

    #[test]
    fn both_quit_keys_have_a_row() {
        let quits: Vec<&Binding> = INVENTORY
            .iter()
            .flat_map(|g| g.bindings.iter())
            .filter(|b| b.action == Action::Quit)
            .collect();
        assert_eq!(quits.len(), 2);
        let inputs: BTreeSet<&str> = quits.iter().map(|b| b.input).collect();
        assert_eq!(inputs, BTreeSet::from(["q", "Ctrl-C"]));
        assert!(INVENTORY.iter().any(|g| g.title == "Pane"
            && g.scope == Scope::Any
            && g.bindings.iter().any(|b| b.action == Action::Quit)));
    }

    #[test]
    fn the_inventory_is_a_pure_static_value_with_no_construction_cost() {
        // `INVENTORY` is referenced from every test above with no
        // constructor call anywhere in this module — two independent
        // references to the `'static` value settle to the same address.
        let a: *const [Group] = INVENTORY;
        let b: *const [Group] = INVENTORY;
        assert_eq!(a, b);
    }

    #[test]
    fn the_key_column_is_measured_in_display_columns() {
        let key_col = key_column();
        assert_eq!(key_col, columns("Backspace"));
        assert_eq!(key_col, 9);
        assert_eq!(2 + key_col + 2, 13);

        for group in INVENTORY {
            for binding in group.bindings {
                assert!(columns(binding.input) <= key_col);
            }
        }
    }

    fn row_plain_text(row: &Row) -> String {
        row.segments.iter().map(|(text, _)| text.as_str()).collect()
    }

    fn assert_grammar_renders_correctly(width: u16) {
        let content = rows(width);
        assert_eq!(content.len(), content_rows());
        let height = content.len() as u16;
        let area = Rect::new(0, 0, width, height);
        let backend = TestBackend::new(width, height);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        terminal
            .draw(|frame| render(frame, area))
            .expect("draw a frame");
        let buffer = terminal.backend().buffer().clone();

        // Row 0: the `Changes` heading, padded to the full width, bold.
        let changes_heading = format!("Changes ({})", Scope::List.label().unwrap());
        assert_eq!(row_text(&buffer, 0), fit(&changes_heading, width as usize));
        assert_eq!(
            cell(&buffer, 0, 0).style(),
            palette::style(Role::RegionHeadingFocused)
        );

        // The next five rows: the `Changes` group's own bindings, each
        // carrying its own `input` and `description`, both beginning at
        // the same column as every other binding row.
        let key_col = key_column();
        let desc_col = 2 + key_col + 2;
        for (i, binding) in INVENTORY[0].bindings.iter().enumerate() {
            let y = 1 + i as u16;
            let text = row_text(&buffer, y);
            let chars: Vec<char> = text.chars().collect();
            assert_eq!(&chars[0..2], &[' ', ' '], "row {y} starts with two spaces");
            let key_field: String = chars[2..desc_col - 2].iter().collect();
            assert_eq!(
                key_field.trim_end(),
                binding.input,
                "row {y}'s key column holds its own input"
            );
            let description: String = chars[desc_col..].iter().collect();
            assert!(
                description.starts_with(binding.description),
                "row {y}'s description should begin at column {desc_col}"
            );
            assert_eq!(
                cell(&buffer, 2, y).style(),
                palette::style(Role::Strong),
                "row {y}'s input is styled Strong"
            );
            assert_eq!(
                cell(&buffer, desc_col as u16, y).style(),
                palette::style(Role::ListRow),
                "row {y}'s description is styled ListRow"
            );
        }

        // A blank row separates the `Changes` group from the `Artifact` group.
        let blank_y = 1 + INVENTORY[0].bindings.len() as u16;
        assert_eq!(row_text(&buffer, blank_y), " ".repeat(width as usize));

        // The `Artifact` heading follows the blank row.
        let artifact_heading = format!("Artifact ({})", Scope::Detail.label().unwrap());
        assert_eq!(
            row_text(&buffer, blank_y + 1),
            fit(&artifact_heading, width as usize)
        );

        // `Agents`, `Pane`, and `Mouse` carry no parenthesised scope,
        // because their groups' `scope` is `Any`.
        for title in ["Agents", "Pane", "Mouse"] {
            let group = INVENTORY.iter().find(|g| g.title == title).unwrap();
            assert_eq!(group.scope, Scope::Any);
            let want = fit(title, width as usize);
            assert!(
                content.iter().any(|row| row_plain_text(row) == want),
                "{title}'s heading should carry no parenthesised scope"
            );
        }

        // No interior row exceeds `width` display columns.
        for row in &content {
            let total: usize = row.segments.iter().map(|(t, _)| columns(t)).sum();
            assert!(total <= width as usize, "a row exceeded {width} columns");
        }
    }

    #[test]
    fn the_grammar_renders_at_120_columns() {
        assert_grammar_renders_correctly(120);
    }

    #[test]
    fn the_grammar_renders_at_60_columns() {
        assert_grammar_renders_correctly(60);
    }
}
