//! The one colour table: a semantic role to a `ratatui::style::Style`, and
//! nothing else. See `openspec/changes/color-palette/specs/view-palette/spec.md`.
//!
//! This is the only file in `src/` permitted to name a ratatui colour, on exactly
//! the terms the markdown parser is confined to `src/ui/markdown.rs` and a process
//! spawn to `src/cli.rs` — enforced by `scripts/gates/palette.sh`, which greps the
//! whole of `src/` with a positive control so a gutted table reads as a broken
//! exclusion rather than a clean tree. Every other file styles a span by asking
//! for a role.
//!
//! Pure and total: no filesystem, process, environment, network, or standard-I/O
//! API, no clock, no global state, and no input that panics. The palette is a
//! **constant table, not a resolver** — `NOIO-VIEW` forbids an environment read in
//! a pure view file, so `NO_COLOR` is not read and the terminal is never probed.
//! The guarantee offered instead is that colour is added strictly *beside* the
//! modifier a role already carried, so a monochrome reading of the frame loses
//! nothing (design.md -> Decision 3 and Decision 10).
//!
//! Both sentences above are worded around the names their own gates search for:
//! `MDSEAM` and `NOIO-VIEW` grep this file whole, so naming the parser crate or an
//! environment API even in prose fails them. That is the known limit those checks
//! already carry and state, and rewording is its stated repair.
//!
//! Only **named** ANSI indices appear: the reader's own terminal theme decides
//! what `Red` looks like, a 16-colour terminal renders the pane correctly, and the
//! pane does not fight the theme of the Herdr panes beside it (design.md ->
//! Decision 9).

use ratatui::style::{Color, Modifier, Style};

use crate::agents::AgentStatus;

/// A semantic role a drawn span carries. Not a colour and not a widget: the role
/// says *what the span means*, and this module alone decides what that looks like.
///
/// `AgentBadge` and `Heading` are parameterised rather than expanded into one
/// variant per value (design.md -> Decision 1), which is what lets `Heading(255)`
/// be answered with a value rather than a lookup miss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    FileMode,
    Footer,
    RegionHeading,
    RegionHeadingFocused,
    RegionRule,
    ListRow,
    ListRowSelected,
    ListProblem,
    ListSeparator,
    ListMessage,
    AgentBadge(AgentStatus),
    TabActive,
    TabInactive,
    Heading(u8),
    Strong,
    Emphasis,
    Code,
    Link,
    Quoted,
    Strikethrough,
}

/// The role table, transcribed from `specs/view-palette/spec.md`'s modifier table
/// and colour table. Total: every `Role` value returns a `Style` and none panics —
/// `Heading(l)` for an `l` outside `1..=6` answers with `Heading(6)`'s style rather
/// than panicking on a level `markdown-render`'s parser cannot emit.
///
/// Colour sits **beside** the modifier a role already carried, never in place of
/// it. Two pairs share a style deliberately: `FileMode` with `Code` (both `DIM` +
/// `Yellow`, and they cannot meet — one is drawn in the list region's heading
/// row, the other
/// only inside the detail region's content area), and `AgentBadge(Unknown)` with
/// `ListSeparator` (both `DarkGray`, this palette's one "no information" grey, and
/// an unknown status and a divider rule are both exactly that).
pub fn style(role: Role) -> Style {
    match role {
        Role::FileMode => Style::default()
            .add_modifier(Modifier::DIM)
            .fg(Color::Yellow),
        Role::Footer => Style::default(),
        Role::RegionHeading => Style::default().add_modifier(Modifier::DIM),
        Role::RegionHeadingFocused => Style::default().add_modifier(Modifier::BOLD),
        Role::RegionRule => Style::default().add_modifier(Modifier::DIM),
        Role::ListRow => Style::default(),
        Role::ListRowSelected => Style::default().add_modifier(Modifier::BOLD),
        Role::ListProblem => Style::default().fg(Color::Red),
        Role::ListSeparator => Style::default().fg(Color::DarkGray),
        Role::ListMessage => Style::default(),
        Role::AgentBadge(status) => Style::default().fg(match status {
            AgentStatus::Working => Color::Green,
            AgentStatus::Idle => Color::Cyan,
            AgentStatus::Blocked => Color::LightRed,
            AgentStatus::Done => Color::Blue,
            AgentStatus::Unknown => Color::DarkGray,
        }),
        Role::TabActive => Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::Black)
            .bg(Color::Cyan),
        Role::TabInactive => Style::default().bg(Color::DarkGray),
        Role::Heading(level) => Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(match level {
                1 => Color::Magenta,
                2 => Color::Cyan,
                3 => Color::Blue,
                4 => Color::Green,
                5 => Color::Yellow,
                // 6 and every level the parser cannot emit.
                _ => Color::DarkGray,
            }),
        Role::Strong => Style::default().add_modifier(Modifier::BOLD),
        Role::Emphasis => Style::default().add_modifier(Modifier::ITALIC),
        Role::Code => Style::default()
            .add_modifier(Modifier::DIM)
            .fg(Color::Yellow),
        Role::Link => Style::default()
            .add_modifier(Modifier::UNDERLINED)
            .fg(Color::Blue),
        Role::Quoted => Style::default().add_modifier(Modifier::DIM),
        // `CROSSED_OUT` rather than a colour: it is the terminal's own
        // rendering of exactly this meaning, it costs no columns, and a
        // terminal that does not support it drops the attribute and still
        // shows the text — the right failure for a construct whose whole point
        // is that the text is still there (design.md -> Decision 9).
        Role::Strikethrough => Style::default().add_modifier(Modifier::CROSSED_OUT),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the spec's three tables say a role carries: the modifier from
    /// "Every role keeps the modifier the crate applied before this change",
    /// and the foreground and background from "Colour is added only where it
    /// carries a distinction a modifier cannot".
    ///
    /// This is the one place in the crate that writes a colour literal down —
    /// every other test compares a rendered cell against `palette::style(role)`
    /// instead (design.md -> Decision 2). It is a second, independent
    /// transcription of the same two tables `style` transcribes, which is what
    /// makes the pair falsifiable rather than tautological.
    struct Expect {
        role: Role,
        modifier: Modifier,
        fg: Option<Color>,
        bg: Option<Color>,
    }

    const NONE: Modifier = Modifier::empty();

    fn row(role: Role, modifier: Modifier, fg: Option<Color>, bg: Option<Color>) -> Expect {
        Expect {
            role,
            modifier,
            fg,
            bg,
        }
    }

    /// Every `Role` variant, the five `AgentStatus` values, and heading levels 1
    /// through 6 — twenty-nine rows, so no arm of `style` is asserted by a
    /// hand-listed subset of the enum.
    fn table() -> Vec<Expect> {
        vec![
            row(Role::FileMode, Modifier::DIM, Some(Color::Yellow), None),
            row(Role::Footer, NONE, None, None),
            row(Role::RegionHeading, Modifier::DIM, None, None),
            row(Role::RegionHeadingFocused, Modifier::BOLD, None, None),
            row(Role::RegionRule, Modifier::DIM, None, None),
            row(Role::ListRow, NONE, None, None),
            row(Role::ListRowSelected, Modifier::BOLD, None, None),
            row(Role::ListProblem, NONE, Some(Color::Red), None),
            row(Role::ListSeparator, NONE, Some(Color::DarkGray), None),
            row(Role::ListMessage, NONE, None, None),
            row(
                Role::AgentBadge(AgentStatus::Working),
                NONE,
                Some(Color::Green),
                None,
            ),
            row(
                Role::AgentBadge(AgentStatus::Idle),
                NONE,
                Some(Color::Cyan),
                None,
            ),
            row(
                Role::AgentBadge(AgentStatus::Blocked),
                NONE,
                Some(Color::LightRed),
                None,
            ),
            row(
                Role::AgentBadge(AgentStatus::Done),
                NONE,
                Some(Color::Blue),
                None,
            ),
            row(
                Role::AgentBadge(AgentStatus::Unknown),
                NONE,
                Some(Color::DarkGray),
                None,
            ),
            row(
                Role::TabActive,
                Modifier::BOLD,
                Some(Color::Black),
                Some(Color::Cyan),
            ),
            row(Role::TabInactive, NONE, None, Some(Color::DarkGray)),
            row(Role::Heading(1), Modifier::BOLD, Some(Color::Magenta), None),
            row(Role::Heading(2), Modifier::BOLD, Some(Color::Cyan), None),
            row(Role::Heading(3), Modifier::BOLD, Some(Color::Blue), None),
            row(Role::Heading(4), Modifier::BOLD, Some(Color::Green), None),
            row(Role::Heading(5), Modifier::BOLD, Some(Color::Yellow), None),
            row(
                Role::Heading(6),
                Modifier::BOLD,
                Some(Color::DarkGray),
                None,
            ),
            row(Role::Strong, Modifier::BOLD, None, None),
            row(Role::Emphasis, Modifier::ITALIC, None, None),
            row(Role::Code, Modifier::DIM, Some(Color::Yellow), None),
            row(Role::Link, Modifier::UNDERLINED, Some(Color::Blue), None),
            row(Role::Quoted, Modifier::DIM, None, None),
            row(Role::Strikethrough, Modifier::CROSSED_OUT, None, None),
        ]
    }

    /// The compile-time half of "no arm is asserted by a hand-listed subset": an
    /// **exhaustive** `match`, so a `Role` variant added later fails to compile
    /// here until it is named — and the developer repairing it meets [`table`]
    /// immediately above. Alive rather than decorative: every assertion below
    /// reports through it.
    fn label(role: Role) -> String {
        match role {
            Role::FileMode => "FileMode".to_string(),
            Role::Footer => "Footer".to_string(),
            Role::RegionHeading => "RegionHeading".to_string(),
            Role::RegionHeadingFocused => "RegionHeadingFocused".to_string(),
            Role::RegionRule => "RegionRule".to_string(),
            Role::ListRow => "ListRow".to_string(),
            Role::ListRowSelected => "ListRowSelected".to_string(),
            Role::ListProblem => "ListProblem".to_string(),
            Role::ListSeparator => "ListSeparator".to_string(),
            Role::ListMessage => "ListMessage".to_string(),
            Role::AgentBadge(status) => format!("AgentBadge({status:?})"),
            Role::TabActive => "TabActive".to_string(),
            Role::TabInactive => "TabInactive".to_string(),
            Role::Heading(level) => format!("Heading({level})"),
            Role::Strong => "Strong".to_string(),
            Role::Emphasis => "Emphasis".to_string(),
            Role::Code => "Code".to_string(),
            Role::Link => "Link".to_string(),
            Role::Quoted => "Quoted".to_string(),
            Role::Strikethrough => "Strikethrough".to_string(),
        }
    }

    const STATUSES: [AgentStatus; 5] = [
        AgentStatus::Working,
        AgentStatus::Idle,
        AgentStatus::Blocked,
        AgentStatus::Done,
        AgentStatus::Unknown,
    ];

    /// The sixteen named ANSI indices — the only colours the palette may name.
    /// `Rgb`, `Indexed`, and `Reset` are absent deliberately (design.md ->
    /// Decision 9); `scripts/gates/palette.sh`'s second leg is the mechanical half
    /// of the same claim.
    const NAMED_ANSI: [Color; 16] = [
        Color::Black,
        Color::Red,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
        Color::Gray,
        Color::DarkGray,
        Color::LightRed,
        Color::LightGreen,
        Color::LightYellow,
        Color::LightBlue,
        Color::LightMagenta,
        Color::LightCyan,
        Color::White,
    ];

    /// `view-palette` :: "The palette answers every role with a `Style`".
    #[test]
    fn the_palette_answers_every_role_with_a_style() {
        for expect in table() {
            // A `Style` came back and nothing panicked on the way.
            let _ = style(expect.role);
        }
        // Every variant is reachable through the exhaustive `label` match, whose
        // non-exhaustiveness is the compile error a future `Role` must repair.
        for expect in table() {
            assert!(!label(expect.role).is_empty());
        }

        // Each of these carries a distinction the reader must actually draw, so no
        // two may collapse onto one another.
        let mut distinct: Vec<(String, Style)> = vec![
            (label(Role::ListProblem), style(Role::ListProblem)),
            (label(Role::FileMode), style(Role::FileMode)),
            (label(Role::TabActive), style(Role::TabActive)),
            (label(Role::TabInactive), style(Role::TabInactive)),
        ];
        for status in STATUSES {
            let role = Role::AgentBadge(status);
            distinct.push((label(role), style(role)));
        }
        for i in 0..distinct.len() {
            for j in (i + 1)..distinct.len() {
                assert_ne!(
                    distinct[i].1, distinct[j].1,
                    "{} and {} must not share a style",
                    distinct[i].0, distinct[j].0
                );
            }
        }

        // The two pairs that DO share a style share it deliberately, so the sharing
        // is recorded rather than a gap the loop above happens to step around.
        assert_eq!(
            style(Role::FileMode),
            style(Role::Code),
            "FileMode and Code are deliberately the same DIM yellow: they cannot meet"
        );
        assert_eq!(
            style(Role::AgentBadge(AgentStatus::Unknown)),
            style(Role::ListSeparator),
            "DarkGray is this palette's one 'no information' grey"
        );

        // `markdown-constructs`: `Strikethrough` joins neither of those pairs by
        // accident — it is a third style equal to no other role's, so the shared
        // set stays exactly the two pairs above.
        for expect in table() {
            if expect.role == Role::Strikethrough {
                continue;
            }
            assert_ne!(
                style(Role::Strikethrough),
                style(expect.role),
                "Strikethrough must not share a style with {}",
                label(expect.role)
            );
        }
    }

    /// `view-palette` :: "Each role's modifier set is exactly the table above".
    #[test]
    fn each_role_s_modifier_set_is_exactly_the_table_above() {
        for expect in table() {
            assert_eq!(
                style(expect.role).add_modifier,
                expect.modifier,
                "{}: wrong modifier set",
                label(expect.role)
            );
        }

        // The seven roles the spec names as carrying no modifier at all.
        for role in [
            Role::Footer,
            Role::ListRow,
            Role::ListProblem,
            Role::ListSeparator,
            Role::ListMessage,
            Role::AgentBadge(AgentStatus::Working),
            Role::TabInactive,
        ] {
            assert_eq!(
                style(role).add_modifier,
                NONE,
                "{} must carry no modifier",
                label(role)
            );
        }

        // The assertion discriminates rather than passing on any modifier at all.
        assert!(
            style(Role::Emphasis)
                .add_modifier
                .contains(Modifier::ITALIC)
        );
        assert!(!style(Role::Emphasis).add_modifier.contains(Modifier::BOLD));
        assert!(
            style(Role::Strikethrough)
                .add_modifier
                .contains(Modifier::CROSSED_OUT)
        );
        assert!(
            !style(Role::Strikethrough)
                .add_modifier
                .contains(Modifier::DIM)
        );
    }

    /// `view-palette` :: "The coloured set is exactly the table above", and
    /// "No RGB, indexed, or reset colour is named" — the value half of the claim
    /// whose textual half is `scripts/gates/palette.sh`'s second leg.
    #[test]
    fn the_coloured_set_is_exactly_the_table_and_every_colour_is_a_named_ansi_index() {
        for expect in table() {
            let got = style(expect.role);
            assert_eq!(
                got.fg,
                expect.fg,
                "{}: wrong foreground",
                label(expect.role)
            );
            assert_eq!(
                got.bg,
                expect.bg,
                "{}: wrong background",
                label(expect.role)
            );
            for colour in [got.fg, got.bg].into_iter().flatten() {
                assert!(
                    NAMED_ANSI.contains(&colour),
                    "{}: {colour:?} is not one of the sixteen named ANSI indices",
                    label(expect.role)
                );
            }
        }

        // `markdown-constructs`: `CROSSED_OUT` already says the whole of what the
        // face means, and the obvious candidate colour — `DarkGray` — is this
        // palette's one "no information" grey, which struck text is not.
        assert_eq!(style(Role::Strikethrough).fg, None);
        assert_eq!(style(Role::Strikethrough).bg, None);

        // Every role the table leaves uncoloured reports neither, so the coloured
        // set is exactly the table rather than merely a subset of it.
        for expect in table() {
            if expect.fg.is_none() && expect.bg.is_none() {
                let got = style(expect.role);
                assert_eq!(got.fg, None, "{}: unexpected fg", label(expect.role));
                assert_eq!(got.bg, None, "{}: unexpected bg", label(expect.role));
            }
        }
    }

    /// `view-palette` :: "An out-of-range heading level does not panic".
    #[test]
    fn an_out_of_range_heading_level_falls_back_to_level_six() {
        let six = style(Role::Heading(6));
        for level in [0u8, 7, 8, 99, 255] {
            assert_eq!(
                style(Role::Heading(level)),
                six,
                "{} must answer with Heading(6)'s style",
                label(Role::Heading(level))
            );
        }
    }
}
