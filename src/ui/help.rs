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
//!
//! This module owns the inventory, the overlay's interior row grammar, and
//! the band's own row grammar — a top rule row, a scrolled window onto
//! [`rows`], and a bottom rule row carrying the position indicator when the
//! content does not fit. `layout::help_band` (the band's rectangle) and its
//! wiring into `ui::view::render` (deciding *whether* and *where* to call
//! [`render`] at all) live elsewhere — `help-overlay`'s later task group 9.
//! [`render`] here is total over every `Rect`, degenerate ones included, and
//! never panics.

use crate::ui::app::{Action, SelectPhase, Target};
use crate::ui::layout::{columns, truncate_columns};
use crate::ui::palette::{self, Role};
use ratatui::Frame;
use ratatui::layout::Rect;

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
/// written in. Six groups, thirty-two bindings; the test module's own shape
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
                description: "Fold/unfold the list section.",
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
                description: "Scroll the content down, or walk the sections.",
                action: Action::Next,
            },
            Binding {
                input: "k / ↑",
                description: "Scroll the content up, or walk the sections.",
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
                description: "Fold/unfold the content section.",
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
                description: "Launch an agent to apply the change.",
                action: Action::LaunchApply,
            },
            Binding {
                input: "c",
                description: "Launch an agent to create the next artifact.",
                action: Action::LaunchContinue,
            },
            Binding {
                input: "s",
                description: "Launch an agent to archive the change.",
                action: Action::LaunchArchive,
            },
            Binding {
                input: "g",
                description: "Focus the agent already running for it.",
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
                description: "Delete the last character; other keys type.",
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
                description: "Move the selection down.",
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
                description: "Select a row, fold a section, or pick a line.",
                action: Action::Click(Target::Change(0)),
            },
            Binding {
                input: "Click",
                description: "Switch to the clicked artifact tab.",
                action: Action::SelectTab(0),
            },
            Binding {
                input: "Drag",
                description: "Select text in the artifact area: drag a span, or press twice for a word and three times for the row.",
                action: Action::Select(SelectPhase::Begin { line: 0, column: 0 }),
            },
        ],
    },
];

/// One row of the overlay's interior grammar: one or more `(text, role)`
/// segments, each already fitted (truncated or padded) to its own share of
/// the row's width, so a caller draws every cell the row covers. Styling a
/// segment is this module's job here, never a render call site's — every
/// segment names a `palette::Role`, on `ui::view`'s own terms.
pub struct Row {
    pub segments: Vec<(String, Role)>,
}

/// The number of rows [`rows`] produces, independent of any particular
/// width: one heading row and one binding row per binding, per group, plus
/// one blank row after every group but the last. Thirty-two bindings, six
/// headings, five blanks: **43**.
pub fn content_rows() -> usize {
    let bindings: usize = INVENTORY.iter().map(|group| group.bindings.len()).sum();
    let headings = INVENTORY.len();
    let blanks = INVENTORY.len().saturating_sub(1);
    bindings + headings + blanks
}

/// The widest `input` in the whole inventory, measured in display columns
/// by [`columns`] and never by a `char` count — every group's descriptions
/// align to this one number. Shared by [`rows`] and by the test module's
/// own key-column assertions, so it is computed in exactly one place.
fn key_column() -> usize {
    INVENTORY
        .iter()
        .flat_map(|group| group.bindings.iter())
        .map(|binding| columns(binding.input))
        .max()
        .unwrap_or(0)
}

/// The longest prefix of `text` that fits in `width` display columns,
/// padded with spaces to exactly `width` columns when it is shorter —
/// never sliced by byte or by `char`. The crate's one column-aware
/// pad-or-truncate; both halves it uses, [`truncate_columns`] and
/// [`columns`], are `ui::layout`'s.
fn fit(text: &str, width: usize) -> String {
    let truncated = truncate_columns(text, width);
    let used = columns(truncated);
    let mut out = truncated.to_string();
    out.push_str(&" ".repeat(width.saturating_sub(used)));
    out
}

/// `title`, and, when `scope` is not [`Scope::Any`], a space and the
/// scope's label in parentheses — `Changes (list route)`, or `Agents` with
/// no suffix at all.
fn heading_text(group: &Group) -> String {
    match group.scope.label() {
        Some(label) => format!("{} ({label})", group.title),
        None => group.title.to_string(),
    }
}

/// The interior grammar at `width` display columns: for every group, in
/// order, a heading row, one row per binding, and a blank row after every
/// group but the last — [`content_rows`] rows in total, regardless of
/// `width`. Every row's segments sum to exactly `width` columns whenever
/// `width` is at least as wide as the key column plus its four surrounding
/// spaces, so a caller painting them leaves no gap for whatever was drawn
/// underneath to show through.
///
/// This is the interior alone: no top or bottom rule row, and no
/// scrolling window onto it — both are `help-overlay`'s later groups.
pub fn rows(width: u16) -> Vec<Row> {
    let key_col = key_column();
    let w = width as usize;
    let mut out = Vec::with_capacity(content_rows());
    let last = INVENTORY.len().saturating_sub(1);
    for (i, group) in INVENTORY.iter().enumerate() {
        out.push(Row {
            segments: vec![(fit(&heading_text(group), w), Role::RegionHeadingFocused)],
        });
        for binding in group.bindings {
            let key_field = format!("  {}  ", fit(binding.input, key_col));
            let key_field_width = columns(&key_field);
            let description = fit(binding.description, w.saturating_sub(key_field_width));
            out.push(Row {
                segments: vec![(key_field, Role::Strong), (description, Role::ListRow)],
            });
        }
        if i != last {
            out.push(Row {
                segments: vec![(" ".repeat(w), Role::ListRow)],
            });
        }
    }
    out
}

/// `─ Help ` followed by `─` to `width`'s right edge — the band's fixed top
/// row. `Help` is [`Role::RegionHeadingFocused`]; every `─`, and the two
/// plain spaces the literal prefix carries, are [`Role::RegionRule`]. A
/// `width` narrower than the seven-column prefix is truncated by
/// [`truncate_columns`] as a single undivided segment, since at that width
/// there is no whole `Help` left to colour differently — see
/// `specs/help-overlay/spec.md` -> "The overlay degrades rather than
/// panicking at any frame size".
fn top_rule_row(width: usize) -> Row {
    const PREFIX: &str = "─ Help ";
    let prefix_cols = columns(PREFIX);
    if width < prefix_cols {
        return Row {
            segments: vec![(
                truncate_columns(PREFIX, width).to_string(),
                Role::RegionRule,
            )],
        };
    }
    Row {
        segments: vec![
            ("─ ".to_string(), Role::RegionRule),
            ("Help".to_string(), Role::RegionHeadingFocused),
            (
                format!(" {}", "─".repeat(width - prefix_cols)),
                Role::RegionRule,
            ),
        ],
    }
}

/// `─` repeated to `width` — the band's fixed bottom row — carrying the
/// `<first>-<last>/<total>` position indicator in [`Role::ListSeparator`]
/// when `content_rows` exceeds `interior_height` and `width` is wide enough
/// to hold it (its own display width plus two; narrower, the rule is drawn
/// whole rather than clipping the indicator). `interior_height` of `0`
/// always draws the plain rule, since an empty window names nothing to
/// report on — this is what lets [`render`]'s two-row degenerate case share
/// this function rather than branch around it. The indicator's own final
/// character lands exactly one column in from `width`'s right edge. See
/// `specs/help-overlay/spec.md` -> "The overlay scrolls when the body
/// cannot hold it".
fn bottom_rule_row(
    width: usize,
    content_rows: usize,
    interior_height: usize,
    offset: usize,
) -> Row {
    if interior_height == 0 || content_rows <= interior_height {
        return Row {
            segments: vec![("─".repeat(width), Role::RegionRule)],
        };
    }
    let first = offset + 1;
    let last = first + interior_height - 1;
    let indicator = format!("{first}-{last}/{content_rows}");
    let indicator_cols = columns(&indicator);
    if width < indicator_cols + 2 {
        return Row {
            segments: vec![("─".repeat(width), Role::RegionRule)],
        };
    }
    let left = width - indicator_cols - 1;
    Row {
        segments: vec![
            ("─".repeat(left), Role::RegionRule),
            (indicator, Role::ListSeparator),
            ("─".to_string(), Role::RegionRule),
        ],
    }
}

/// Draw one already-fitted [`Row`] at `(x, y)`, cell by cell, advancing by
/// each segment's own [`columns`] width — the one place both [`render`] and
/// its rule rows paint a row, so painting never disagrees with measuring.
fn draw_row(buf: &mut ratatui::buffer::Buffer, x: u16, y: u16, row: &Row) {
    let mut cx = x;
    for (text, role) in &row.segments {
        buf.set_string(cx, y, text, palette::style(*role));
        cx = cx.saturating_add(columns(text) as u16);
    }
}

/// Draw the whole band into `area`: [`top_rule_row`], a window onto
/// [`rows`] at the offset [`layout::scroll_offset`] clamps `scroll` to, and
/// [`bottom_rule_row`]. Total over every `Rect`, degenerate ones included,
/// and never panics — see `specs/help-overlay/spec.md` -> "The overlay
/// degrades rather than panicking at any frame size".
///
/// The two-row case (a top rule and a bottom rule with no interior) needs
/// no branch of its own: at `interior_height` `0`,
/// `layout::scroll_offset` returns `0`, the interior loop's `take(0)` draws
/// nothing, and [`bottom_rule_row`]'s own `interior_height == 0` guard
/// already omits the indicator. Only the fully empty case (zero width or
/// height) and the one-row case (the top rule alone, no bottom rule) are
/// distinct enough to need their own early return.
///
/// `scroll` is the caller's stored offset — `help.scroll`, on
/// `detail.scroll`'s own terms — never resolved or clamped here; deciding
/// *whether* to call this at all, and normalising `scroll` beforehand, are
/// `Dashboard`'s job (`normalise_help_scroll`) and `ui::view::render`'s
/// (`help-overlay`'s later task group 9).
pub fn render(frame: &mut Frame, body: Rect, help: &crate::ui::app::Help) {
    // The band is derived here, from the body, rather than taken as an
    // already-computed rectangle. `help_band` is total and cheap, and owning
    // the derivation is what makes the parameter impossible to get wrong: a
    // caller that passed the body where a band was wanted would otherwise
    // draw a full-height overlay with no error anywhere, which is precisely
    // the silent-wrong-rectangle failure `scroll_offset`'s own argument-order
    // note warns about one level down. `specs/help-overlay/spec.md` states
    // this signature.
    let area = crate::ui::layout::help_band(body, content_rows());
    let scroll = help.scroll;
    if area.width == 0 || area.height == 0 {
        return;
    }
    let width = area.width as usize;
    let buf = frame.buffer_mut();
    draw_row(buf, area.x, area.y, &top_rule_row(width));
    if area.height < 2 {
        return;
    }
    let total = content_rows();
    let interior_height = (area.height - 2) as usize;
    let offset = crate::ui::layout::scroll_offset(total, scroll, interior_height as u16);
    for (i, row) in rows(area.width)
        .iter()
        .enumerate()
        .skip(offset)
        .take(interior_height)
    {
        let y = area.y + 1 + (i - offset) as u16;
        draw_row(buf, area.x, y, row);
    }
    let bottom_y = area.y + area.height - 1;
    draw_row(
        buf,
        area.x,
        bottom_y,
        &bottom_rule_row(width, total, interior_height, offset),
    );
}

#[cfg(test)]
mod tests {
    use super::{
        Binding, Group, INVENTORY, Row, Scope, content_rows, fit, key_column, render, rows,
    };
    use crate::testutil::{cell, row_text};
    use crate::ui::app::Action;
    use crate::ui::layout::{columns, help_band, split_frame};
    use crate::ui::palette::{self, Role};
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;
    use std::collections::BTreeSet;

    /// The **body** a full dashboard render at `total` would hand to
    /// [`render`] — `layout::split_frame`'s own body region, the footer row
    /// excluded. [`render`] derives the band from it with `help_band`, so
    /// these tests exercise that derivation rather than bypassing it;
    /// `band_for` below is what they compare the drawn rectangle against.
    /// `ui::view::render`'s own wiring of this pair is `help-overlay`'s
    /// later task group 9, and needs no `help_band` call of its own.
    fn body_for(total: Rect) -> Rect {
        let (body, _) = split_frame(total);
        body
    }

    /// A closed-at-the-top [`Help`] — `scroll` `0`. [`render`] never reads
    /// `open`: deciding *whether* to draw the overlay belongs to
    /// `ui::view::render` (task group 9), so a scenario that is only about
    /// the grammar or the geometry passes this and says so.
    fn closed_at_top() -> crate::ui::app::Help {
        crate::ui::app::Help {
            open: false,
            scroll: 0,
        }
    }

    /// Where [`render`] will actually draw, given the same `total`. Kept
    /// beside [`body_for`] so a test can assert against the band's own
    /// geometry — including the vertical centring, which `body_for` alone
    /// would not reach — without recomputing the rule twice.
    fn band_for(total: Rect) -> Rect {
        help_band(body_for(total), content_rows())
    }

    /// A `Dashboard` with `help` set and every other field at its emptiest
    /// — no repository, no changes, `Route::List` — since none of this
    /// module's own scenarios reach past `apply`/`normalise_help_scroll`
    /// into anything that field would otherwise matter to. No `Default`
    /// exists for `Dashboard`, so every field is still named here, on the
    /// same terms as every other test module's own fixture.
    fn minimal_dashboard(help: crate::ui::app::Help) -> crate::ui::app::Dashboard {
        crate::ui::app::Dashboard {
            selection: None,
            help,
            repo: None,
            searched_from: std::path::PathBuf::new(),
            changes: crate::changes::empty_set(),
            route: crate::ui::app::Route::List,
            quit: false,
            selected: 0,
            filter: crate::ui::app::Filter {
                query: String::new(),
                active: false,
            },
            detail: crate::ui::app::Detail {
                sections: Vec::new(),
                scroll: 0,
                tab: 0,
                problems: Vec::new(),
                loaded: None,
                expanded: BTreeSet::new(),
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
            sections: crate::ui::app::Sections {
                collapsed: BTreeSet::new(),
            },
            file_mode: false,
        }
    }

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
        assert_eq!(counts, vec![5, 7, 4, 4, 5, 7]);
        assert_eq!(counts.iter().sum::<usize>(), 32);

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

    /// A `TestBackend` buffer's own resting style before anything is drawn
    /// into it — `ui::view`'s `uncoloured()` is the same helper for the same
    /// reason. Ratatui fills an uncoloured cell with its own reset colour
    /// rather than leaving the field empty, so "carries no colour" is
    /// equality with this rather than with `Style::default()`, whose fields
    /// are `None`. Read from `Cell::default()` because this file, like every
    /// file but `src/ui/palette.rs`, may not name a colour at all — the
    /// `PALETTE` gate sweeps inline test modules too, comments included.
    /// `Buffer::set_string` also **patches** onto whatever a cell already
    /// carried rather than replacing it outright, which is why the
    /// comparisons below patch the expected role onto this.
    fn uncoloured() -> ratatui::style::Style {
        ratatui::buffer::Cell::default().style()
    }

    fn assert_grammar_renders_correctly(width: u16) {
        let content = rows(width);
        assert_eq!(content.len(), content_rows());
        // The band's own height is the interior plus its two rule rows —
        // `content.len()` rows are visible with nothing left to scroll to,
        // so `scroll` `0` shows all of them and `bottom_rule_row` draws no
        // indicator (`content_rows <= interior_height`).
        let height = content.len() as u16 + 2;
        // Passed as the **body**: `help_band` over a body of exactly
        // `content_rows + 2` returns that same rectangle — `min(45, 45)` with
        // no remainder to centre — so the band fills it and the row indices
        // below are the band's own.
        let body = Rect::new(0, 0, width, height);
        let backend = TestBackend::new(width, height);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        terminal
            .draw(|frame| render(frame, body, &closed_at_top()))
            .expect("draw a frame");
        let buffer = terminal.backend().buffer().clone();

        // Row 0: the band's top rule — `─ Help ` then `─` to the right
        // edge. `Help` is styled `RegionHeadingFocused`; the surrounding
        // dashes and spaces are `RegionRule`. Both widths tested here are
        // well past the seven-column prefix, so no truncation branch is
        // exercised — `degenerate_frames_render_without_panicking` covers
        // that one.
        let top_prefix = ["─", " ", "H", "e", "l", "p", " "];
        for (x, want) in top_prefix.iter().enumerate() {
            assert_eq!(
                cell(&buffer, x as u16, 0).symbol(),
                *want,
                "top rule column {x}"
            );
        }
        assert_eq!(
            cell(&buffer, 2, 0).style(),
            uncoloured().patch(palette::style(Role::RegionHeadingFocused)),
            "the H of Help is styled RegionHeadingFocused"
        );
        for x in top_prefix.len()..width as usize {
            assert_eq!(
                cell(&buffer, x as u16, 0).symbol(),
                "─",
                "top rule fill at column {x}"
            );
        }

        // Row 1: the `Changes` heading, padded to the full width, bold —
        // the interior's own first row, one below the top rule.
        let changes_heading = format!("Changes ({})", Scope::List.label().unwrap());
        assert_eq!(row_text(&buffer, 1), fit(&changes_heading, width as usize));
        assert_eq!(
            cell(&buffer, 0, 1).style(),
            uncoloured().patch(palette::style(Role::RegionHeadingFocused))
        );

        // The next five rows: the `Changes` group's own bindings, each
        // carrying its own `input` and `description`, both beginning at
        // the same column as every other binding row.
        let key_col = key_column();
        let desc_col = 2 + key_col + 2;
        for (i, binding) in INVENTORY[0].bindings.iter().enumerate() {
            let y = 2 + i as u16;
            // Sliced by **column**, one cell at a time, never by `char` over
            // the joined row: `row_text` concatenates cell symbols, and a cell
            // symbol is a grapheme that need not be one `char`. Counting chars
            // here would be silently correct against an ASCII fixture and wrong
            // against the `↑`/`↓` this very inventory already carries — the
            // exact drift `COLWIDTH` exists to forbid, in a test whose whole
            // subject is display-column alignment. `COLWIDTH` stops at the
            // first `#[cfg(test)]` and would not have caught it.
            let columns = |from: usize, to: usize| -> String {
                (from..to.min(width as usize))
                    .map(|x| cell(&buffer, x as u16, y).symbol())
                    .collect()
            };
            assert_eq!(columns(0, 2), "  ", "row {y} starts with two spaces");
            let key_field = columns(2, desc_col - 2);
            assert_eq!(
                key_field.trim_end(),
                binding.input,
                "row {y}'s key column holds its own input"
            );
            let description = columns(desc_col, width as usize);
            let expected_description = fit(binding.description, width as usize - desc_col);
            assert_eq!(
                description, expected_description,
                "row {y}'s description should begin at column {desc_col}, \
                 truncated or padded to the band's own width"
            );
            assert_eq!(
                cell(&buffer, 2, y).style(),
                uncoloured().patch(palette::style(Role::Strong)),
                "row {y}'s input is styled Strong"
            );
            assert_eq!(
                cell(&buffer, desc_col as u16, y).style(),
                uncoloured().patch(palette::style(Role::ListRow)),
                "row {y}'s description is styled ListRow"
            );
        }

        // A blank row separates the `Changes` group from the `Artifact` group.
        let blank_y = 2 + INVENTORY[0].bindings.len() as u16;
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

        // The band's last row is the bottom rule, drawn whole: all
        // `content_rows()` rows fit the interior with none left over, so no
        // indicator is due.
        assert_eq!(
            row_text(&buffer, height - 1),
            "─".repeat(width as usize),
            "the bottom rule carries no indicator when the content fits"
        );
    }

    #[test]
    fn the_grammar_renders_at_120_columns() {
        assert_grammar_renders_correctly(120);
    }

    #[test]
    fn the_grammar_renders_at_60_columns() {
        assert_grammar_renders_correctly(60);
    }

    /// `specs/help-overlay/spec.md` -> "The overlay scrolls when the body
    /// cannot hold it" -> "The overlay scrolls at both mandated sizes".
    #[test]
    fn the_overlay_scrolls_at_both_mandated_sizes() {
        // 120x40: body 39 rows, band 39 (clamped against 45 wanted),
        // interior 37 against 43 content rows.
        let total = Rect::new(0, 0, 120, 40);
        let body = body_for(total);
        let band = band_for(total);
        assert_eq!(
            band.height, 39,
            "120x40's band is clamped to the body's 39 rows"
        );
        let mut dashboard = minimal_dashboard(crate::ui::app::Help {
            open: true,
            scroll: 0,
        });
        let backend = TestBackend::new(120, 40);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        terminal
            .draw(|f| render(f, body, &dashboard.help))
            .expect("draw the unscrolled band");
        let buffer = terminal.backend().buffer().clone();
        let bottom_y = band.y + band.height - 1;
        assert_indicator(&buffer, band, bottom_y, "1-37/43");

        for _ in 0..10 {
            dashboard.apply(Action::Next);
        }
        dashboard.normalise_help_scroll(total);
        assert_eq!(dashboard.help.scroll, 6, "clamped to 43 - 37");
        terminal
            .draw(|f| render(f, body, &dashboard.help))
            .expect("draw the scrolled band");
        let buffer = terminal.backend().buffer().clone();
        assert_indicator(&buffer, band, bottom_y, "7-43/43");

        // 60x20: body 19 rows, band 19, interior 17 against 43 content
        // rows.
        let total = Rect::new(0, 0, 60, 20);
        let body = body_for(total);
        let band = band_for(total);
        assert_eq!(band.height, 19);
        let help = crate::ui::app::Help {
            open: true,
            scroll: 0,
        };
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        terminal
            .draw(|f| render(f, body, &help))
            .expect("draw the 60x20 band");
        let buffer = terminal.backend().buffer().clone();
        assert_indicator(&buffer, band, band.y + band.height - 1, "1-17/43");
    }

    /// Read the trailing `text.len()` columns of row `y`, cell by cell —
    /// never by slicing a joined `String` by byte, since `─` is a
    /// multi-byte glyph and a byte offset would not agree with a column
    /// offset. Asserts the indicator's own final character sits exactly one
    /// column in from `band`'s right edge, and that the one column after it
    /// is still a plain rule.
    fn assert_indicator(buffer: &ratatui::buffer::Buffer, band: Rect, y: u16, want: &str) {
        let indicator_cols = columns(want);
        let start = band.x as usize + band.width as usize - indicator_cols - 1;
        let got: String = (start..start + indicator_cols)
            .map(|x| cell(buffer, x as u16, y).symbol())
            .collect();
        assert_eq!(got, want, "indicator at row {y}");
        assert_eq!(
            cell(buffer, band.x + band.width - 1, y).symbol(),
            "─",
            "the indicator's final character is one column in from the right edge"
        );
        assert_eq!(
            cell(buffer, (start - 1) as u16, y).symbol(),
            "─",
            "a rule column precedes the indicator"
        );
    }

    /// `specs/help-overlay/spec.md` -> "A held key cannot run the window
    /// off the end".
    #[test]
    fn a_held_key_cannot_run_the_window_off_the_end() {
        let total = Rect::new(0, 0, 60, 20);
        let body = body_for(total);
        let band = band_for(total);
        let mut dashboard = minimal_dashboard(crate::ui::app::Help {
            open: true,
            scroll: 0,
        });
        let backend = TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");

        for _ in 0..200 {
            dashboard.apply(Action::Next);
            dashboard.normalise_help_scroll(total);
            terminal
                .draw(|f| render(f, body, &dashboard.help))
                .expect("redraw after Next");
        }
        assert_eq!(dashboard.help.scroll, 26, "clamped to 43 - 17");
        let buffer = terminal.backend().buffer().clone();
        let last_content_row = &rows(band.width)[content_rows() - 1];
        let interior_last_y = band.y + band.height - 2;
        assert_eq!(
            row_text(&buffer, interior_last_y),
            row_plain_text(last_content_row),
            "the window's last visible row is always content row 43, never blank"
        );

        for _ in 0..200 {
            dashboard.apply(Action::Prev);
            dashboard.normalise_help_scroll(total);
            terminal
                .draw(|f| render(f, body, &dashboard.help))
                .expect("redraw after Prev");
        }
        assert_eq!(
            dashboard.help.scroll, 0,
            "a held Prev saturates rather than underflowing"
        );
        let buffer = terminal.backend().buffer().clone();
        let first_content_row = &rows(band.width)[0];
        assert_eq!(
            row_text(&buffer, band.y + 1),
            row_plain_text(first_content_row),
            "two hundred Prev actions return the window to content row 1"
        );
    }

    /// `specs/help-overlay/spec.md` -> "No indicator when the content
    /// fits".
    #[test]
    fn no_indicator_when_the_content_fits() {
        // 120x60: body 59 rows, band 45 (43 content rows + 2 rule rows,
        // well under the body), interior 43 — exactly `content_rows()`, so
        // every row is visible in one frame and no indicator is due.
        let total = Rect::new(0, 0, 120, 60);
        let body = body_for(total);
        let band = band_for(total);
        assert_eq!(band.height, 45);
        let help = crate::ui::app::Help {
            open: true,
            scroll: 0,
        };
        let backend = TestBackend::new(120, 60);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        terminal
            .draw(|f| render(f, body, &help))
            .expect("draw the tall band");
        let buffer = terminal.backend().buffer().clone();
        assert_eq!(
            row_text(&buffer, band.y + band.height - 1),
            "─".repeat(band.width as usize),
            "the bottom rule carries no digits"
        );
        for (i, row) in rows(band.width).iter().enumerate() {
            assert_eq!(
                row_text(&buffer, band.y + 1 + i as u16),
                row_plain_text(row),
                "content row {i} is visible in the one frame"
            );
        }
    }

    /// `specs/help-overlay/spec.md` -> "The overlay degrades rather than
    /// panicking at any frame size" -> "Degenerate frames render without
    /// panicking". Drawing the footer row is `ui::view::render`'s own job
    /// (`help-overlay`'s later task group 9, not wired yet), so the
    /// `with the footer on row 1` half of that scenario is not asserted
    /// here — only what `render` itself is responsible for.
    #[test]
    fn degenerate_frames_render_without_panicking() {
        let backend = TestBackend::new(130, 60);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        for (w, h) in [
            (1u16, 1u16),
            (1, 2),
            (2, 1),
            (0, 0),
            (120, 1),
            (120, 2),
            (120, 3),
            (5, 10),
        ] {
            let total = Rect::new(0, 0, w, h);
            let body = body_for(total);
            let band = band_for(total);
            terminal
                .draw(|f| render(f, body, &closed_at_top()))
                .unwrap_or_else(|e| panic!("draw at {w}x{h} failed: {e}"));
            let buffer = terminal.backend().buffer().clone();
            // Read exactly `band.width` columns, cell by cell — the
            // backend is a fixed 130 columns wide so every band here is
            // narrower than it, and `row_text` reads the whole backend
            // row, trailing blanks included.
            let band_row = |y: u16| -> String {
                (0..band.width)
                    .map(|x| cell(&buffer, x, y).symbol())
                    .collect()
            };
            match (w, h) {
                (120, 2) => {
                    // body 1 row -> band height 1: the top rule alone.
                    assert_eq!(band.height, 1);
                    let expected = format!("─ Help {}", "─".repeat(120 - 7));
                    assert_eq!(band_row(band.y), expected);
                }
                (120, 3) => {
                    // body 2 rows -> band height 2: top and bottom rule,
                    // no interior, no indicator.
                    assert_eq!(band.height, 2);
                    let expected = format!("─ Help {}", "─".repeat(120 - 7));
                    assert_eq!(band_row(band.y), expected);
                    assert_eq!(band_row(band.y + 1), "─".repeat(120));
                }
                (5, 10) => {
                    assert_eq!(
                        band_row(band.y),
                        "─ Hel",
                        "the first five columns of ─ Help "
                    );
                }
                _ => {}
            }
        }
    }

    /// `specs/help-overlay/spec.md` -> "The overlay degrades rather than
    /// panicking at any frame size" -> "The reader is never trapped in a
    /// degenerate frame". `Dashboard::apply` never receives a `Rect` at
    /// all, so the "1x1 frame" the scenario names is flavour rather than a
    /// fixture this test needs to build — what it actually pins is that
    /// `Quit`, `Back`, and `ToggleHelp` still act while `help.open` is
    /// true, independent of anything the view ever computed. This test
    /// lives here rather than in `ui::app`'s own test module because this
    /// group's file manifest does not extend to `src/ui/app.rs` beyond
    /// `normalise_help_scroll`; `Dashboard::apply` is `pub` and reachable
    /// from any module that needs it.
    #[test]
    fn the_reader_is_never_trapped_in_a_degenerate_frame() {
        let mut quitter = minimal_dashboard(crate::ui::app::Help {
            open: true,
            scroll: 0,
        });
        quitter.apply(Action::Quit);
        assert!(quitter.quit);

        let mut backer = minimal_dashboard(crate::ui::app::Help {
            open: true,
            scroll: 0,
        });
        backer.apply(Action::Back);
        assert!(!backer.help.open);

        let mut toggler = minimal_dashboard(crate::ui::app::Help {
            open: true,
            scroll: 0,
        });
        toggler.apply(Action::ToggleHelp);
        assert!(!toggler.help.open);
    }
}
