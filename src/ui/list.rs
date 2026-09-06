//! The row grammar: every function here is a pure total transformation,
//! parameterised by an interior width, with no ratatui styling. See
//! `openspec/changes/list-view/specs/change-rows/spec.md` and design.md ->
//! Decisions ("The row grammar is fixed-field and right-aligned").

use crate::ui::app::{Dashboard, matches};

/// What kind of thing a `Row` represents, so a caller can tell a change row
/// from a separator, a message, or a repository-level problem without
/// parsing its text. `Item`'s `index` is into the *visible* list `rows`
/// emits — active then archived, filtered — never into `ChangeSet` itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    Problem,
    Item { index: usize },
    Separator,
    Message,
}

/// One drawn row: its text, exactly `width` characters; its kind; and
/// whether it carries the selection marker. `list.rs` never styles a row —
/// `ui::view` applies `Modifier::BOLD` to the selected one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub text: String,
    pub kind: RowKind,
    pub selected: bool,
}

/// The progress cell: `[<completed>/<total>]`, or the three characters
/// `[-]` when `total` is zero, so a change with no tasks still ends its row
/// in the same column as one that has them. `pub(crate)` rather than
/// private: `ui::detail::header_row` calls this rather than copying it, so
/// the crate keeps exactly one progress-cell implementation.
pub(crate) fn progress_cell(progress: &crate::tasks::Progress) -> String {
    if progress.total == 0 {
        "[-]".to_string()
    } else {
        format!("[{}/{}]", progress.completed, progress.total)
    }
}

/// Pad `text` with trailing spaces to `width` when it fits; otherwise
/// truncate to `width - 1` characters and append `…` — the crate's one
/// right-truncation implementation, shared by the name field, the problem
/// row, and the message rows. `width == 0` truncates to the empty string:
/// there is no room even for the ellipsis. `pub(crate)` rather than
/// private: `ui::detail`'s header, tab bar, and problem-line grammar calls
/// this rather than copying it.
pub(crate) fn pad_or_truncate_right(text: &str, width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= width {
        let mut s: String = chars.into_iter().collect();
        s.push_str(&" ".repeat(width - s.chars().count()));
        return s;
    }
    if width == 0 {
        return String::new();
    }
    let mut s: String = chars[..width - 1].iter().collect();
    s.push('…');
    s
}

/// The keep-the-tail truncation `change-rows`' no-repository block and
/// `responsive-layout`'s header share: whole when `text` fits in `width`
/// characters, else `…` followed by its last `width - 1` characters, empty
/// at `width == 0`. Does **not** pad — callers that need a full-width row
/// pad the result themselves, since the header uses this un-padded (it
/// right-aligns within its own remaining space).
pub(crate) fn shorten_left(text: &str, width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= width {
        return chars.into_iter().collect();
    }
    if width == 0 {
        return String::new();
    }
    let tail: String = chars[chars.len() - (width - 1)..].iter().collect();
    format!("…{tail}")
}

/// `shorten_left`, then padded with trailing spaces to `width` — the
/// no-repository block's third row needs a full-width row like every other
/// kind, unlike the header's un-padded, right-aligned use of the same rule.
fn shorten_left_row(text: &str, width: usize) -> String {
    let shortened = shorten_left(text, width);
    let len = shortened.chars().count();
    if len >= width {
        shortened
    } else {
        format!("{shortened}{}", " ".repeat(width - len))
    }
}

/// The active-change grammar: `[marker][space][name field][space][badge]
/// [space][progress]`, with the badge cell offered only together with the
/// progress cell and dropped whole first (reclaiming its own separating
/// space) as soon as the name field would fall below one column with it;
/// the progress cell (and its separating space) drops next on the same
/// condition; and the row degenerates to the first `width` characters of
/// `"{marker} "` below two columns. `badge` `None` reproduces exactly
/// today's `[marker][space][name field][space][progress]` grammar, byte
/// for byte — see `agent-attribution` -> Decisions 3. Shared,
/// unparameterised by date, by both the active row and the final
/// degenerate branch of the archived row.
fn active_style_row(
    marker: char,
    name: &str,
    progress: Option<&str>,
    badge: Option<char>,
    width: u16,
) -> String {
    let w = i64::from(width);
    if w < 2 {
        let head = format!("{marker} ");
        return head.chars().take(w.max(0) as usize).collect();
    }
    if let Some(progress) = progress {
        let progress_len = progress.chars().count() as i64;
        if let Some(badge) = badge {
            let name_field_w = w - 2 - 1 - 1 - 1 - progress_len;
            if name_field_w >= 1 {
                let name_field = pad_or_truncate_right(name, name_field_w as usize);
                return format!("{marker} {name_field} {badge} {progress}");
            }
        }
        let name_field_w = w - 2 - 1 - progress_len;
        if name_field_w >= 1 {
            let name_field = pad_or_truncate_right(name, name_field_w as usize);
            return format!("{marker} {name_field} {progress}");
        }
    }
    let name_field_w = (w - 2) as usize;
    let name_field = pad_or_truncate_right(name, name_field_w);
    format!("{marker} {name_field}")
}

/// The archived-change grammar: `[marker][space][date field: 10][space]`
/// then the active grammar's `[name field][space][badge][space][progress]`,
/// with the date field ten spaces when `date` is `None`. Dropped whole, in
/// order — the badge cell (and its separating space) first, then the
/// progress cell (and its separating space), then the date field (and its
/// separating space), then degenerating to `active_style_row` with neither
/// a badge nor a progress cell ever offered — exactly as `change-rows`'
/// "Archived changes sit below a separator" requirement states.
fn archived_row_text(
    marker: char,
    date: Option<&str>,
    name: &str,
    progress: &str,
    badge: Option<char>,
    width: u16,
) -> String {
    let w = i64::from(width);
    let date_field = date.map_or_else(|| " ".repeat(10), str::to_string);
    let progress_len = progress.chars().count() as i64;

    // Full form with the badge: marker + space + date(10) + space + name +
    // space + badge + space + progress.
    if let Some(badge) = badge {
        let name_field_w = w - 14 - 1 - 1 - progress_len;
        if name_field_w >= 1 {
            let name_field = pad_or_truncate_right(name, name_field_w as usize);
            return format!("{marker} {date_field} {name_field} {badge} {progress}");
        }
    }

    // Full form without the badge (dropped, or never offered): marker +
    // space + date(10) + space + name + space + progress.
    let name_field_full = w - 14 - progress_len;
    if name_field_full >= 1 {
        let name_field = pad_or_truncate_right(name, name_field_full as usize);
        return format!("{marker} {date_field} {name_field} {progress}");
    }

    // Drop the progress cell and its separating space: marker + space +
    // date(10) + space + name.
    let name_field_no_progress = w - 13;
    if name_field_no_progress >= 1 {
        let name_field = pad_or_truncate_right(name, name_field_no_progress as usize);
        return format!("{marker} {date_field} {name_field}");
    }

    // Drop the date field too: degenerate to the active grammar, with
    // neither a badge nor a progress cell ever offered.
    active_style_row(marker, name, None, None, width)
}

/// A `Problem` row: `! `, then the text, truncated with `…` when the width
/// falls below three columns to the first `width` characters of `"! "`
/// itself, exactly as `change-rows`' empty-state requirement states.
fn problem_row_text(text: &str, width: u16) -> String {
    let w = width as usize;
    if w < 3 {
        return "! ".chars().take(w).collect();
    }
    format!("! {}", pad_or_truncate_right(text, w - 2))
}

/// A `Message` row: the whole width, no prefix, padded or truncated by the
/// shared right-truncation rule.
fn message_row_text(text: &str, width: u16) -> String {
    pad_or_truncate_right(text, width as usize)
}

fn separator_row_text(width: u16) -> String {
    const PREFIX: &str = "  -- archived ";
    let w = width as usize;
    let prefix_len = PREFIX.chars().count();
    if w <= prefix_len {
        return PREFIX.chars().take(w).collect();
    }
    format!("{PREFIX}{}", "-".repeat(w - prefix_len))
}

fn no_repo_rows(searched_from: &std::path::Path, width: u16) -> Vec<Row> {
    let path_text = searched_from.display().to_string();
    vec![
        Row {
            text: message_row_text("No OpenSpec repository found", width),
            kind: RowKind::Message,
            selected: false,
        },
        Row {
            text: message_row_text("searched from:", width),
            kind: RowKind::Message,
            selected: false,
        },
        Row {
            text: shorten_left_row(&path_text, width as usize),
            kind: RowKind::Message,
            selected: false,
        },
    ]
}

/// The list region's content: every row, in the order `change-rows` states
/// — problem rows, then the visible active changes (or the empty-state
/// message for the state the dashboard is in), then the separator when at
/// least one visible archived change follows, then the visible archived
/// changes. Pure and total over every `Dashboard` value and every `u16`
/// width, including `0`: never panics, performs no I/O, reads no clock and
/// no global state. See `specs/change-rows/spec.md`.
pub fn rows(dashboard: &Dashboard, width: u16) -> Vec<Row> {
    if dashboard.repo.is_none() {
        return no_repo_rows(&dashboard.searched_from, width);
    }

    let attribution = dashboard.attribution();
    let query = dashboard.filter.query.as_str();
    let active: Vec<&crate::changes::Change> = dashboard
        .changes
        .active
        .iter()
        .filter(|c| matches(&c.name, query))
        .collect();
    let archived: Vec<&crate::changes::Change> = dashboard
        .changes
        .archived
        .iter()
        .filter(|c| matches(&c.name, query))
        .collect();

    let mut out = Vec::new();
    // `agent-launch`: launch problems lead the whole list, ahead of even refresh problems —
    // they are the only rows that answer a key the reader has just pressed, and burying the
    // reply under a standing condition is how a reader concludes the key did nothing.
    // `launch.problems` holds at most **two** entries (`degraded-states`' repair of row 23: a
    // recording failure and a prompt failure can co-occur) and is replaced wholesale, so this
    // costs at most two rows.
    for problem in &dashboard.launch.problems {
        out.push(Row {
            text: problem_row_text(problem, width),
            kind: RowKind::Problem,
            selected: false,
        });
    }
    // `live-refresh`: refresh problems (a watcher that would not start, or a
    // drain error) lead the list, ahead of change-set problems — they are
    // the ones that outlive a reload, while a `ChangeSet` problem is
    // re-derived every cycle and may vanish on the next one. Same grammar,
    // same row kind: `change-rows` requires exactly one problem kind, so
    // neither is addressable by `selected` and `ui::view` styles both alike.
    for problem in &dashboard.refresh.problems {
        out.push(Row {
            text: problem_row_text(problem, width),
            kind: RowKind::Problem,
            selected: false,
        });
    }
    for problem in &dashboard.changes.problems {
        out.push(Row {
            text: problem_row_text(problem, width),
            kind: RowKind::Problem,
            selected: false,
        });
    }

    if active.is_empty() && archived.is_empty() {
        if query.is_empty() {
            out.push(Row {
                text: message_row_text("No changes yet", width),
                kind: RowKind::Message,
                selected: false,
            });
        } else {
            out.push(Row {
                text: message_row_text("No changes match", width),
                kind: RowKind::Message,
                selected: false,
            });
            out.push(Row {
                text: message_row_text(&format!("/{query}"), width),
                kind: RowKind::Message,
                selected: false,
            });
        }
        return out;
    }

    let mut index = 0usize;
    if active.is_empty() {
        out.push(Row {
            text: message_row_text("No active changes", width),
            kind: RowKind::Message,
            selected: false,
        });
    } else {
        for change in &active {
            let selected = index == dashboard.selected;
            let marker = if selected { '>' } else { ' ' };
            let badge = attribution
                .badges
                .get(&change.name)
                .copied()
                .map(badge_char);
            out.push(Row {
                text: active_style_row(
                    marker,
                    &change.name,
                    Some(&progress_cell(&change.progress)),
                    badge,
                    width,
                ),
                kind: RowKind::Item { index },
                selected,
            });
            index += 1;
        }
    }

    if !archived.is_empty() {
        out.push(Row {
            text: separator_row_text(width),
            kind: RowKind::Separator,
            selected: false,
        });
        for change in &archived {
            let selected = index == dashboard.selected;
            let marker = if selected { '>' } else { ' ' };
            let date = match &change.origin {
                crate::changes::Origin::Archived { date } => date.as_deref(),
                crate::changes::Origin::Active => None,
            };
            let badge = attribution
                .badges
                .get(&change.name)
                .copied()
                .map(badge_char);
            out.push(Row {
                text: archived_row_text(
                    marker,
                    date,
                    &change.name,
                    &progress_cell(&change.progress),
                    badge,
                    width,
                ),
                kind: RowKind::Item { index },
                selected,
            });
            index += 1;
        }
    }

    out
}

/// The badge glyph for `status` — one ASCII column, collision-free against
/// every other glyph this row grammar uses (`>`, `!`, `…`, `-`, `[`). See
/// `specs/change-rows/spec.md` -> "The agent badge".
fn badge_char(status: crate::agents::AgentStatus) -> char {
    match status {
        crate::agents::AgentStatus::Working => 'w',
        crate::agents::AgentStatus::Idle => 'i',
        crate::agents::AgentStatus::Blocked => 'b',
        crate::agents::AgentStatus::Done => 'd',
        crate::agents::AgentStatus::Unknown => '?',
    }
}

#[cfg(test)]
mod tests {
    use crate::agents::{Agent, AgentStatus};
    use crate::changes::fixture;
    use crate::ui::app::{Dashboard, Detail, Filter, Route};
    use crate::ui::list::{Row, RowKind, problem_row_text, rows};

    /// An in-scope `Agent` named `name` at `/tmp/demo-repo` — this module's fixture
    /// dashboards' repository root — carrying `status`. `agent-attribution`'s own
    /// tests build agents this way rather than through a `Change` literal, on
    /// exactly `NOLIT-CHANGE`'s terms.
    fn agent_at(name: &str, status: AgentStatus) -> Agent {
        Agent {
            name: Some(name.to_string()),
            kind: None,
            status,
            cwd: Some(std::path::PathBuf::from("/tmp/demo-repo")),
            pane_id: "p".to_string(),
            tab_id: "t".to_string(),
            workspace_id: "w".to_string(),
            terminal_title: None,
        }
    }

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

    fn dashboard_with(
        active: Vec<crate::changes::Change>,
        archived: Vec<crate::changes::Change>,
        problems: Vec<String>,
        selected: usize,
    ) -> Dashboard {
        Dashboard {
            repo: Some(std::path::PathBuf::from("/tmp/demo-repo")),
            searched_from: std::path::PathBuf::from("/tmp/demo-repo"),
            changes: fixture::set(active, archived, problems),
            route: Route::List,
            quit: false,
            selected,
            filter: empty_filter(),
            detail: empty_detail(),
            refresh: crate::ui::app::Refresh {
                requested: false,
                reload: false,
                problems: Vec::new(),
            },
            agents: crate::agents::AgentSnapshot {
                agents: Vec::new(),
                reachable: false,
                problem: None,
            },
            agent_names: crate::state::Mapping::default(),
            launch: crate::ui::app::Launch {
                pending: None,
                problems: Vec::new(),
            },
        }
    }

    fn three_active() -> Dashboard {
        dashboard_with(
            vec![
                fixture::active("add-token-refresh", 4, 9),
                fixture::active("fix-empty-basket", 7, 7),
                fixture::active("migrate-ai-sdk-v7", 0, 0),
            ],
            Vec::new(),
            Vec::new(),
            0,
        )
    }

    /// Every dashboard fixture this module's tests build, for
    /// `rows_never_panic_at_any_width` and `every_row_is_exactly_the_requested_width`.
    fn every_fixture() -> Vec<Dashboard> {
        vec![
            three_active(),
            dashboard_with(
                vec![fixture::active("alpha", 4, 9)],
                Vec::new(),
                Vec::new(),
                0,
            ),
            dashboard_with(
                Vec::new(),
                vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
                Vec::new(),
                0,
            ),
            dashboard_with(
                vec![fixture::active("fix-empty-basket", 7, 7)],
                vec![
                    fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                    fixture::archived(None, "legacy-cleanup", 3, 3),
                ],
                Vec::new(),
                0,
            ),
            dashboard_with(Vec::new(), Vec::new(), Vec::new(), 0),
            {
                let mut d = dashboard_with(Vec::new(), Vec::new(), Vec::new(), 0);
                d.filter.query = "zzz".to_string();
                d
            },
            dashboard_with(
                Vec::new(),
                vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
                Vec::new(),
                0,
            ),
            dashboard_with(
                vec![fixture::active("fix-empty-basket", 7, 7)],
                Vec::new(),
                vec!["openspec/changes: Permission denied (os error 13)".to_string()],
                0,
            ),
            Dashboard {
                repo: None,
                searched_from: std::path::PathBuf::from(
                    "/home/dev/workspaces/openspec-demos/a-rather-long-repository-name-here",
                ),
                changes: fixture::set(Vec::new(), Vec::new(), Vec::new()),
                route: Route::List,
                quit: false,
                selected: 0,
                filter: empty_filter(),
                detail: empty_detail(),
                refresh: crate::ui::app::Refresh {
                    requested: false,
                    reload: false,
                    problems: Vec::new(),
                },
                agents: crate::agents::AgentSnapshot {
                    agents: Vec::new(),
                    reachable: false,
                    problem: None,
                },
                agent_names: crate::state::Mapping::default(),
                launch: crate::ui::app::Launch {
                    pending: None,
                    problems: Vec::new(),
                },
            },
        ]
    }

    #[test]
    fn active_row_grammar_at_38_and_58() {
        let d = three_active();
        for (width, expected) in [
            (
                38,
                [
                    "> add-token-refresh              [4/9]",
                    "  fix-empty-basket               [7/7]",
                    "  migrate-ai-sdk-v7                [-]",
                ],
            ),
            (
                58,
                [
                    "> add-token-refresh                                  [4/9]",
                    "  fix-empty-basket                                   [7/7]",
                    "  migrate-ai-sdk-v7                                    [-]",
                ],
            ),
        ] {
            let rows = rows(&d, width);
            assert_eq!(rows.len(), 3);
            for (row, expect) in rows.iter().zip(expected.iter()) {
                assert_eq!(&row.text, expect, "width {width}");
            }
        }

        // `agent-attribution`: the identical dashboard, now carrying agents, still
        // renders every row byte-identically when none of them is in scope.
        let mut unattributed = three_active();
        unattributed.agents.agents = vec![agent_at("scratch-work", AgentStatus::Working)];
        for width in [38, 58] {
            let badgeless = rows(&d, width);
            let still_unbadged = rows(&unattributed, width);
            assert_eq!(
                badgeless, still_unbadged,
                "width {width}: an unmatched agent must badge nothing"
            );
        }
    }

    #[test]
    fn progress_cell_is_dash_when_total_is_zero() {
        let d = three_active();
        for width in [38, 58] {
            let rows = rows(&d, width);
            assert!(rows[0].text.ends_with(']'));
            assert!(rows[2].text.ends_with(']'));
            // migrate-ai-sdk-v7 is the 0-of-0 fixture: its cell is the dash
            // form, not a numeric one — found in Change Review, whose test
            // only compared the two rows' last characters and would have
            // passed a `[0/0]` cell too.
            assert!(rows[2].text.contains("[-]"), "width {width}");
            assert_eq!(
                rows[0].text.chars().last(),
                rows[2].text.chars().last(),
                "width {width}"
            );
        }
    }

    #[test]
    fn every_row_is_exactly_the_requested_width() {
        for d in every_fixture() {
            for width in [38, 58] {
                for row in rows(&d, width) {
                    assert_eq!(row.text.chars().count(), width as usize);
                }
            }
        }

        // `agent-attribution`: a badged fixture is exactly the requested width too.
        let mut badged = three_active();
        badged.agents.agents = vec![agent_at("add-token-refresh", AgentStatus::Working)];
        for width in [38, 58] {
            for row in rows(&badged, width) {
                assert_eq!(row.text.chars().count(), width as usize, "badged row");
            }
        }
    }

    #[test]
    fn a_long_name_is_truncated_with_an_ellipsis() {
        let d = dashboard_with(
            vec![fixture::active(
                "a-very-long-change-name-that-will-not-fit-here",
                2,
                5,
            )],
            Vec::new(),
            Vec::new(),
            usize::MAX,
        );
        let rows38 = rows(&d, 38);
        assert_eq!(rows38[0].text, "  a-very-long-change-name-that-… [2/5]");
        let rows58 = rows(&d, 58);
        assert_eq!(
            rows58[0].text,
            "  a-very-long-change-name-that-will-not-fit-here     [2/5]"
        );
        assert!(!rows58[0].text.contains('…'));

        // `agent-attribution`: the 46-character name is unreachable by Herdr's own
        // 32-character cap, so it is badged only through the mapping tier — and doing
        // so truncates the name two columns earlier to make room for the badge.
        let long_name = "a-very-long-change-name-that-will-not-fit-here";
        let derived = crate::state::agent_name(long_name);
        let mut badged = d;
        badged.agents.agents = vec![agent_at(&derived, AgentStatus::Working)];
        badged.agent_names.names =
            std::collections::BTreeMap::from([(derived, long_name.to_string())]);
        let rows38_badged = rows(&badged, 38);
        assert_eq!(
            rows38_badged[0].text,
            "  a-very-long-change-name-tha… w [2/5]"
        );
    }

    #[test]
    fn a_narrow_width_drops_the_progress_cell_whole() {
        let d = dashboard_with(
            vec![fixture::active("alpha", 4, 9)],
            Vec::new(),
            Vec::new(),
            0,
        );
        assert_eq!(rows(&d, 10)[0].text, "> a… [4/9]");
        assert_eq!(rows(&d, 9)[0].text, "> … [4/9]");
        assert_eq!(rows(&d, 8)[0].text, "> alpha ");
        assert_eq!(rows(&d, 3)[0].text, "> …");
        assert_eq!(rows(&d, 2)[0].text, "> ");
        assert_eq!(rows(&d, 1)[0].text, ">");
        assert_eq!(rows(&d, 0)[0].text, "");
        assert!(rows(&d, 38)[0].text.contains("[4/9]"));
        assert!(rows(&d, 58)[0].text.contains("[4/9]"));
    }

    #[test]
    fn archived_narrow_widths_drop_progress_then_date() {
        let d = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
            Vec::new(),
            1,
        );
        let cases: [(u16, &str); 7] = [
            (20, "> 2026-08-14 … [7/7]"),
            (19, "> 2026-08-14 add-a…"),
            (14, "> 2026-08-14 …"),
            (13, "> add-auth   "),
            (3, "> …"),
            (1, ">"),
            (0, ""),
        ];
        for (width, expected) in cases {
            let row = &rows(&d, width)[2];
            assert_eq!(row.text, expected, "width {width}");
            assert_eq!(row.text.chars().count(), width as usize);
        }
        assert!(rows(&d, 38)[2].text.contains("2026-08-14"));
        assert!(rows(&d, 38)[2].text.contains("[7/7]"));
        assert!(rows(&d, 58)[2].text.contains("2026-08-14"));
        assert!(rows(&d, 58)[2].text.contains("[7/7]"));
    }

    /// `change-rows`: "A badged row carries its status between the name and the
    /// progress cell" — exact strings at 38, the badge's 0-based column index at 58.
    #[test]
    fn a_badged_active_row_at_both_widths() {
        let mut d = three_active();
        d.agents.agents = vec![
            agent_at("add-token-refresh", AgentStatus::Working),
            agent_at("fix-empty-basket", AgentStatus::Blocked),
            agent_at("migrate-ai-sdk-v7", AgentStatus::Unknown),
        ];

        let rows38 = rows(&d, 38);
        assert_eq!(rows38[0].text, "> add-token-refresh            w [4/9]");
        assert_eq!(rows38[1].text, "  fix-empty-basket             b [7/7]");
        assert_eq!(rows38[2].text, "  migrate-ai-sdk-v7              ? [-]");

        let rows58 = rows(&d, 58);
        for row in &rows58 {
            assert_eq!(row.text.chars().count(), 58);
        }
        assert_eq!(rows58[0].text.chars().nth(51), Some('w'));
        assert_eq!(rows58[1].text.chars().nth(51), Some('b'));
        assert_eq!(rows58[2].text.chars().nth(53), Some('?'));
        for (row, idx) in [(&rows58[0], 51), (&rows58[1], 51), (&rows58[2], 53)] {
            assert_eq!(
                row.text.chars().nth(idx - 1),
                Some(' '),
                "column left of the badge"
            );
            assert_eq!(
                row.text.chars().nth(idx + 1),
                Some(' '),
                "column right of the badge"
            );
        }
        assert!(rows58[0].text.ends_with("[4/9]"));
        assert!(rows58[1].text.ends_with("[7/7]"));
        assert!(rows58[2].text.ends_with("[-]"));
    }

    /// `change-rows`: "An archived change carries a badge in the same column as an
    /// active one".
    #[test]
    fn a_badged_archived_row_at_both_widths() {
        let unbadged = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            vec![
                fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                fixture::archived(None, "legacy-cleanup", 3, 3),
            ],
            Vec::new(),
            0,
        );
        let mut d = unbadged.clone();
        d.agents.agents = vec![agent_at("add-auth", AgentStatus::Blocked)];

        let rows38 = rows(&d, 38);
        assert_eq!(rows38[0].text, "> fix-empty-basket               [7/7]");
        assert_eq!(rows38[2].text, "  2026-08-14 add-auth          b [7/7]");
        assert_eq!(rows38[3].text, "             legacy-cleanup      [3/3]");

        let rows58 = rows(&d, 58);
        for row in &rows58 {
            assert_eq!(row.text.chars().count(), 58);
        }
        assert_eq!(rows58[2].text.chars().nth(51), Some('b'));
        assert_eq!(rows58[2].text.chars().nth(50), Some(' '));
        assert_eq!(rows58[2].text.chars().nth(52), Some(' '));
        assert_eq!(
            rows58[3].text,
            rows(&unbadged, 58)[3].text,
            "the separator and every unbadged row must be byte-identical"
        );
    }

    /// `change-rows`: "An unattributed agent badges nothing" — the agentless
    /// rendering, byte-identical, then the discriminating half: the same agent
    /// renamed to a change does badge.
    #[test]
    fn an_unattributed_agent_badges_nothing() {
        let agentless = three_active();
        let mut with_unattributed = three_active();
        with_unattributed.agents.agents = vec![
            agent_at("scratch", AgentStatus::Working),
            Agent {
                name: None,
                kind: None,
                status: AgentStatus::Working,
                cwd: Some(std::path::PathBuf::from("/tmp/demo-repo")),
                pane_id: "p".to_string(),
                tab_id: "t".to_string(),
                workspace_id: "w".to_string(),
                terminal_title: None,
            },
        ];
        for width in [38, 58] {
            assert_eq!(
                rows(&agentless, width),
                rows(&with_unattributed, width),
                "width {width}: an unmatched or unnamed agent must badge nothing"
            );
        }

        // Discriminating half: the second agent renamed to a change does badge.
        let mut renamed = three_active();
        renamed.agents.agents = vec![
            agent_at("scratch", AgentStatus::Working),
            agent_at("migrate-ai-sdk-v7", AgentStatus::Working),
        ];
        for width in [38, 58] {
            assert_ne!(
                rows(&agentless, width),
                rows(&renamed, width),
                "width {width}: renaming the agent to a change must badge that row"
            );
            assert!(
                rows(&renamed, width)[2].text.contains(" w ["),
                "width {width}: migrate-ai-sdk-v7's row must carry the working badge: {:?}",
                rows(&renamed, width)[2].text
            );
        }
    }

    /// `change-rows`: "A field too narrow for both drops the progress cell whole" —
    /// the badged form.
    #[test]
    fn a_badged_row_drops_the_badge_first() {
        let mut d = dashboard_with(
            vec![fixture::active("alpha", 4, 9)],
            Vec::new(),
            Vec::new(),
            0,
        );
        d.agents.agents = vec![agent_at("alpha", AgentStatus::Working)];

        let cases: [(u16, &str); 5] = [
            (12, "> a… w [4/9]"),
            (11, "> … w [4/9]"),
            (10, "> a… [4/9]"),
            (9, "> … [4/9]"),
            (8, "> alpha "),
        ];
        for (width, expected) in cases {
            assert_eq!(rows(&d, width)[0].text, expected, "width {width}");
        }
        assert_eq!(rows(&d, 1)[0].text, ">");
        assert_eq!(rows(&d, 0)[0].text, "");
        assert!(rows(&d, 38)[0].text.contains(" w ["));
        assert!(rows(&d, 58)[0].text.contains(" w ["));
    }

    /// `change-rows`: "An archived row drops the progress cell, then the date, as
    /// the width falls" — the badged form.
    #[test]
    fn a_badged_archived_row_drops_the_badge_first() {
        let mut d = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
            Vec::new(),
            1,
        );
        d.agents.agents = vec![agent_at("add-auth", AgentStatus::Blocked)];

        let cases: [(u16, &str); 9] = [
            (22, "> 2026-08-14 … b [7/7]"),
            (21, "> 2026-08-14 a… [7/7]"),
            (20, "> 2026-08-14 … [7/7]"),
            (19, "> 2026-08-14 add-a…"),
            (14, "> 2026-08-14 …"),
            (13, "> add-auth   "),
            (3, "> …"),
            (1, ">"),
            (0, ""),
        ];
        for (width, expected) in cases {
            assert_eq!(rows(&d, width)[2].text, expected, "width {width}");
        }
        assert!(rows(&d, 38)[2].text.contains("2026-08-14"));
        assert!(rows(&d, 38)[2].text.contains(" b ["));
        assert!(rows(&d, 58)[2].text.contains("2026-08-14"));
        assert!(rows(&d, 58)[2].text.contains(" b ["));
    }

    #[test]
    fn archived_rows_carry_a_ten_column_date_field() {
        let d = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
            Vec::new(),
            usize::MAX,
        );
        assert_eq!(
            rows(&d, 38)[2].text,
            "  2026-08-14 add-auth            [7/7]"
        );
        assert_eq!(
            rows(&d, 58)[2].text,
            "  2026-08-14 add-auth                                [7/7]"
        );
    }

    #[test]
    fn an_undated_archived_row_uses_ten_spaces() {
        let d = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            vec![fixture::archived(None, "legacy-cleanup", 3, 3)],
            Vec::new(),
            usize::MAX,
        );
        assert_eq!(
            rows(&d, 38)[2].text,
            "             legacy-cleanup      [3/3]"
        );
        assert_eq!(
            rows(&d, 58)[2].text,
            "             legacy-cleanup                          [3/3]"
        );

        let dated = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            vec![fixture::archived(
                Some("2026-08-14"),
                "legacy-cleanup",
                3,
                3,
            )],
            Vec::new(),
            usize::MAX,
        );
        for width in [38, 58] {
            let undated_start = rows(&d, width)[2].text.find('l');
            let dated_start = rows(&dated, width)[2].text.find('l');
            assert_eq!(undated_start, dated_start, "width {width}");
        }
    }

    #[test]
    fn the_separator_fills_the_width() {
        assert_eq!(
            separator_for_test(38),
            "  -- archived ------------------------"
        );
        assert_eq!(
            separator_for_test(58),
            "  -- archived --------------------------------------------"
        );
        assert_eq!(separator_for_test(6), "  -- a");
    }

    fn separator_for_test(width: u16) -> String {
        let d = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
            Vec::new(),
            0,
        );
        rows(&d, width)
            .into_iter()
            .find(|r| r.kind == RowKind::Separator)
            .expect("a separator row")
            .text
    }

    #[test]
    fn the_separator_is_emitted_only_when_archived_rows_follow() {
        let no_archived = three_active();
        for width in [38, 58] {
            assert!(
                !rows(&no_archived, width)
                    .iter()
                    .any(|r| r.kind == RowKind::Separator)
            );
        }
        let with_archived = dashboard_with(
            vec![fixture::active("fix-empty-basket", 7, 7)],
            vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
            Vec::new(),
            0,
        );
        for width in [38, 58] {
            let count = rows(&with_archived, width)
                .iter()
                .filter(|r| r.kind == RowKind::Separator)
                .count();
            assert_eq!(count, 1);
        }
    }

    #[test]
    fn row_order_is_problems_then_active_then_separator_then_archived() {
        let mut d = dashboard_with(
            vec![
                fixture::active("add-token-refresh", 4, 9),
                fixture::active("fix-empty-basket", 7, 7),
            ],
            vec![
                fixture::archived(Some("2026-08-14"), "add-auth", 7, 7),
                fixture::archived(None, "legacy-cleanup", 3, 3),
            ],
            vec!["openspec/changes: broken".to_string()],
            0,
        );
        // `agent-launch`: a launch problem leads even the refresh and change-set problems.
        d.launch.problems = vec!["launch failed".to_string()];
        for width in [38, 58] {
            let kinds: Vec<RowKind> = rows(&d, width).into_iter().map(|r| r.kind).collect();
            assert_eq!(
                kinds,
                vec![
                    RowKind::Problem,
                    RowKind::Problem,
                    RowKind::Item { index: 0 },
                    RowKind::Item { index: 1 },
                    RowKind::Separator,
                    RowKind::Item { index: 2 },
                    RowKind::Item { index: 3 },
                ],
                "width {width}"
            );
        }
    }

    #[test]
    fn rows_preserve_the_change_set_order() {
        let d = dashboard_with(
            vec![
                fixture::active("zeta", 1, 2),
                fixture::active("alpha", 1, 2),
                fixture::active("mid", 1, 2),
            ],
            Vec::new(),
            Vec::new(),
            usize::MAX,
        );
        for width in [38, 58] {
            let rows = rows(&d, width);
            assert!(rows[0].text.contains("zeta"));
            assert!(rows[1].text.contains("alpha"));
            assert!(rows[2].text.contains("mid"));
        }
    }

    #[test]
    fn the_selected_flag_marks_exactly_the_selected_change() {
        let d = dashboard_with(
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
            3,
        );
        for width in [38, 58] {
            let rows = rows(&d, width);
            let selected: Vec<&Row> = rows.iter().filter(|r| r.selected).collect();
            assert_eq!(selected.len(), 1, "width {width}");
            assert_eq!(selected[0].kind, RowKind::Item { index: 3 });
            for r in &rows {
                if r.kind == RowKind::Separator {
                    assert!(!r.selected);
                }
            }
        }
    }

    #[test]
    fn the_no_repository_block_is_three_rows() {
        let d = Dashboard {
            repo: None,
            searched_from: std::path::PathBuf::from(
                "/home/dev/workspaces/openspec-demos/a-rather-long-repository-name-here",
            ),
            changes: fixture::set(Vec::new(), Vec::new(), Vec::new()),
            route: Route::List,
            quit: false,
            selected: 0,
            filter: empty_filter(),
            detail: empty_detail(),
            refresh: crate::ui::app::Refresh {
                requested: false,
                reload: false,
                problems: Vec::new(),
            },
            agents: crate::agents::AgentSnapshot {
                agents: Vec::new(),
                reachable: false,
                problem: None,
            },
            agent_names: crate::state::Mapping::default(),
            launch: crate::ui::app::Launch {
                pending: None,
                problems: Vec::new(),
            },
        };
        let rows38 = rows(&d, 38);
        assert_eq!(rows38.len(), 3);
        assert!(rows38[0].text.starts_with("No OpenSpec repository found"));
        assert!(rows38[1].text.starts_with("searched from:"));
        assert!(
            rows38[2]
                .text
                .starts_with("…os/a-rather-long-repository-name-here")
        );
        assert!(
            !rows38
                .iter()
                .any(|r| matches!(r.kind, RowKind::Item { .. }))
        );

        let rows58 = rows(&d, 58);
        assert!(
            rows58[2]
                .text
                .starts_with("…kspaces/openspec-demos/a-rather-long-repository-name-here")
        );
        assert!(
            !rows58
                .iter()
                .any(|r| matches!(r.kind, RowKind::Item { .. }))
        );

        // The no-repository block replaces EVERY other row, problem rows
        // included — `ui::load` cannot build this value (it only ever
        // constructs `changes::empty_set()` alongside `repo: None`), but
        // `rows` is a pure total function over every `Dashboard` a test can
        // construct, and this precedence is stated for exactly that case.
        // Found in Change Review: no scenario exercised it, so a guard
        // narrowed to `repo.is_none() && problems.is_empty()` would have
        // passed the whole suite.
        let with_problems = Dashboard {
            repo: None,
            searched_from: d.searched_from.clone(),
            changes: fixture::set(Vec::new(), Vec::new(), vec!["broken".to_string()]),
            route: Route::List,
            quit: false,
            selected: 0,
            filter: empty_filter(),
            detail: empty_detail(),
            refresh: crate::ui::app::Refresh {
                requested: false,
                reload: false,
                problems: Vec::new(),
            },
            agents: crate::agents::AgentSnapshot {
                agents: Vec::new(),
                reachable: false,
                problem: None,
            },
            agent_names: crate::state::Mapping::default(),
            launch: crate::ui::app::Launch {
                pending: None,
                problems: Vec::new(),
            },
        };
        let rows_with_problems = rows(&with_problems, 38);
        assert_eq!(rows_with_problems.len(), 3);
        assert!(
            !rows_with_problems
                .iter()
                .any(|r| r.kind == RowKind::Problem)
        );
    }

    #[test]
    fn the_four_message_states_are_distinct() {
        for width in [38, 58] {
            let empty = dashboard_with(Vec::new(), Vec::new(), Vec::new(), 0);
            let rows_empty = rows(&empty, width);
            assert!(rows_empty[0].text.starts_with("No changes yet"));

            // `agent-attribution`: two in-scope, unattributable agents change no pixel
            // of the message row — a `Message` row is never badged.
            let mut empty_with_agents = dashboard_with(Vec::new(), Vec::new(), Vec::new(), 0);
            empty_with_agents.agents.agents = vec![
                agent_at("nothing-like-a-change", AgentStatus::Working),
                agent_at("also-nothing", AgentStatus::Idle),
            ];
            assert_eq!(
                rows(&empty_with_agents, width),
                rows_empty,
                "width {width}: an unattributable agent must badge no message row"
            );

            let archived_only = dashboard_with(
                Vec::new(),
                vec![fixture::archived(Some("2026-08-14"), "add-auth", 7, 7)],
                Vec::new(),
                0,
            );
            let rows_archived_only = rows(&archived_only, width);
            assert!(rows_archived_only[0].text.starts_with("No active changes"));
            assert_eq!(rows_archived_only[1].kind, RowKind::Separator);
            assert_eq!(rows_archived_only[2].kind, RowKind::Item { index: 0 });

            let mut no_match = dashboard_with(
                vec![fixture::active("fix-empty-basket", 7, 7)],
                Vec::new(),
                Vec::new(),
                0,
            );
            no_match.filter.query = "zzz".to_string();
            let rows_no_match = rows(&no_match, width);
            assert!(rows_no_match[0].text.starts_with("No changes match"));
            assert!(rows_no_match[1].text.starts_with("/zzz"));

            let mut matched = dashboard_with(
                vec![fixture::active("fix-empty-basket", 7, 7)],
                Vec::new(),
                Vec::new(),
                0,
            );
            matched.filter.query = "fix".to_string();
            let rows_matched = rows(&matched, width);
            assert!(!rows_matched.iter().any(|r| r.kind == RowKind::Message));
        }
    }

    #[test]
    fn problem_rows_are_truncated_to_the_width() {
        let d = dashboard_with(
            Vec::new(),
            Vec::new(),
            vec!["openspec/changes: Permission denied (os error 13)".to_string()],
            0,
        );
        let row38 = &rows(&d, 38)[0];
        assert_eq!(row38.text, "! openspec/changes: Permission denied…");
        let row58 = &rows(&d, 58)[0];
        assert_eq!(
            row58.text,
            "! openspec/changes: Permission denied (os error 13)       "
        );
    }

    #[test]
    fn rows_never_panic_at_any_width() {
        for d in every_fixture() {
            for width in [0u16, 1, 2, 8, 9, 10, 13, 14, 38, 58, 120, u16::MAX] {
                for row in rows(&d, width) {
                    assert_eq!(row.text.chars().count(), width as usize, "width {width}");
                }
            }
        }
    }

    // `live-refresh` -> "The list region's leading rows name refresh
    // problems first". See `specs/live-updates/spec.md`.

    #[test]
    fn refresh_problems_lead_the_rows() {
        let mut d = dashboard_with(
            vec![fixture::active("add-token-refresh", 4, 9)],
            Vec::new(),
            Vec::new(),
            0,
        );
        d.refresh.problems = vec!["filesystem watch unavailable for /r".to_string()];
        for width in [38, 58] {
            let all = rows(&d, width);
            assert_eq!(all[0].kind, RowKind::Problem);
            assert_eq!(
                all[0].text,
                problem_row_text("filesystem watch unavailable for /r", width),
                "width {width}"
            );
            assert_eq!(all[1].kind, RowKind::Item { index: 0 }, "width {width}");
        }

        // The no-problem control: with `refresh.problems` empty, the same
        // dashboard's rows are unchanged.
        let mut without = d.clone();
        without.refresh.problems = Vec::new();
        for width in [38, 58] {
            assert_eq!(rows(&without, width)[0].kind, RowKind::Item { index: 0 });
        }

        // `agent-launch`: the same holds for `launch.problems` — a pane that has launched
        // nothing renders byte-identically to one with no launch tier at all.
        assert!(without.launch.problems.is_empty());
        let mut with_launch_failure = without.clone();
        with_launch_failure.launch.problems = vec!["launch failed".to_string()];
        for width in [38, 58] {
            assert_eq!(
                rows(&with_launch_failure, width)[0].kind,
                RowKind::Problem,
                "width {width}"
            );
            assert_eq!(rows(&without, width)[0].kind, RowKind::Item { index: 0 });
        }
    }

    #[test]
    fn refresh_and_change_problems_in_order() {
        let mut d = dashboard_with(
            vec![fixture::active("add-token-refresh", 4, 9)],
            Vec::new(),
            vec!["openspec/changes unreadable".to_string()],
            0,
        );
        d.refresh.problems = vec!["watch failed".to_string()];
        // `agent-launch`: a launch problem leads even the refresh problem.
        d.launch.problems = vec!["launch failed".to_string()];
        for width in [38, 58] {
            let kinds: Vec<RowKind> = rows(&d, width).into_iter().map(|r| r.kind).collect();
            assert_eq!(
                kinds,
                vec![
                    RowKind::Problem,
                    RowKind::Problem,
                    RowKind::Problem,
                    RowKind::Item { index: 0 },
                ],
                "width {width}"
            );
            let texts: Vec<String> = rows(&d, width).into_iter().map(|r| r.text).collect();
            assert_eq!(
                texts[0],
                problem_row_text("launch failed", width),
                "the launch problem must come first, width {width}"
            );
            assert_eq!(
                texts[1],
                problem_row_text("watch failed", width),
                "the refresh problem must come second, width {width}"
            );
            assert_eq!(
                texts[2],
                problem_row_text("openspec/changes unreadable", width),
                "width {width}"
            );
        }

        // The empty control: with `launch.problems` emptied, the rows are byte-identical to
        // the two-problem list this scenario specified before `agent-launch` existed.
        let mut without_launch = d.clone();
        without_launch.launch.problems = Vec::new();
        for width in [38, 58] {
            let kinds: Vec<RowKind> = rows(&without_launch, width)
                .into_iter()
                .map(|r| r.kind)
                .collect();
            assert_eq!(
                kinds,
                vec![
                    RowKind::Problem,
                    RowKind::Problem,
                    RowKind::Item { index: 0 },
                ],
                "width {width}"
            );
        }

        // `agent-attribution`: a badged change below the three problem rows carries its
        // badge, and none of the problem rows above it ever carries one.
        d.agents.agents = vec![agent_at("add-token-refresh", AgentStatus::Blocked)];
        for width in [38, 58] {
            let rows = rows(&d, width);
            assert!(
                !rows[0].text.contains(" b ["),
                "width {width}: a problem row is never badged"
            );
            assert!(
                !rows[1].text.contains(" b ["),
                "width {width}: a problem row is never badged"
            );
            assert!(
                !rows[2].text.contains(" b ["),
                "width {width}: a problem row is never badged"
            );
            assert!(
                rows[3].text.contains(" b ["),
                "width {width}: the change row below must carry its badge"
            );
        }
    }

    /// `agent-launch`: "A refused launch renders one row at both widths."
    #[test]
    fn a_launch_problem_is_the_lists_first_row() {
        let mut d = dashboard_with(
            vec![fixture::active("2fa-support", 1, 2)],
            Vec::new(),
            Vec::new(),
            0,
        );
        d.launch.problems = vec![
            "c-2fa-support is already running for this change - press g to focus it".to_string(),
        ];
        for width in [38, 58] {
            let all = rows(&d, width);
            assert_eq!(all[0].kind, RowKind::Problem, "width {width}");
            assert!(
                all[0].text.starts_with("! "),
                "width {width}: {}",
                all[0].text
            );
            assert!(all[0].text.contains("c-2fa-support"), "width {width}");
            assert!(all[0].text.contains('g'), "width {width}");
            assert_eq!(
                all[0].text.chars().count(),
                width as usize,
                "width {width}: padded or truncated to the interior width exactly"
            );
            assert_eq!(all[1].kind, RowKind::Item { index: 0 }, "width {width}");

            // Pressing `a` twice more (replacing the entry wholesale, never growing it)
            // leaves exactly one such row.
            let count = rows(&d, width)
                .into_iter()
                .filter(|r| r.kind == RowKind::Problem)
                .count();
            assert_eq!(count, 1, "width {width}");
        }
    }

    /// `degraded-states`' repair of row 23 (task 6.2): the two reasons — a recording failure
    /// and a prompt failure — render as the list's first two `!`-marked interior rows, in
    /// occurrence order, at both widths. RED at `main`: `launch::Outcome::problem` could not
    /// hold both, so `Dashboard::launch.problems` never carried more than one entry.
    #[test]
    fn a_failed_record_and_a_failed_prompt_are_both_reported() {
        let mut d = dashboard_with(
            vec![fixture::active("2fa-support", 1, 2)],
            Vec::new(),
            Vec::new(),
            0,
        );
        d.launch.problems = vec![
            "/state/dir: not a directory".to_string(),
            "agent_blocked: agent is blocked".to_string(),
        ];
        for width in [38, 58] {
            let all = rows(&d, width);
            assert_eq!(all[0].kind, RowKind::Problem, "width {width}");
            assert_eq!(all[1].kind, RowKind::Problem, "width {width}");
            assert!(all[0].text.starts_with("! "), "width {width}");
            assert!(all[1].text.starts_with("! "), "width {width}");
            assert!(all[0].text.contains("/state/dir"), "width {width}");
            assert!(all[1].text.contains("agent_blocked"), "width {width}");
            assert_eq!(all[2].kind, RowKind::Item { index: 0 }, "width {width}");
        }
    }

    /// `agent-launch`: "A launch problem leads the refresh and change-set problems."
    #[test]
    fn launch_refresh_and_change_problems_in_order() {
        let mut d = dashboard_with(
            vec![fixture::active("alpha", 1, 2)],
            Vec::new(),
            vec!["openspec/changes unreadable".to_string()],
            0,
        );
        d.refresh.problems = vec!["watch failed".to_string()];
        d.launch.problems = vec!["launch failed".to_string()];
        for width in [38, 58] {
            let all = rows(&d, width);
            assert_eq!(all[0].kind, RowKind::Problem, "width {width}");
            assert_eq!(all[1].kind, RowKind::Problem, "width {width}");
            assert_eq!(all[2].kind, RowKind::Problem, "width {width}");
            assert_eq!(all[0].text, problem_row_text("launch failed", width));
            assert_eq!(all[1].text, problem_row_text("watch failed", width));
            assert_eq!(
                all[2].text,
                problem_row_text("openspec/changes unreadable", width)
            );
            assert!(!all[0].selected && !all[1].selected && !all[2].selected);
        }
    }

    /// `agent-launch`: "No refresh problem draws no extra row" restated for
    /// `launch.problems` — a pane that has launched nothing renders exactly as before.
    #[test]
    fn no_launch_problem_draws_no_extra_row() {
        let d = dashboard_with(
            vec![fixture::active("alpha", 1, 2)],
            Vec::new(),
            Vec::new(),
            0,
        );
        assert!(d.launch.problems.is_empty());
        for width in [38, 58] {
            assert_eq!(
                rows(&d, width)[0].kind,
                RowKind::Item { index: 0 },
                "width {width}"
            );
        }
    }

    /// `agent-launch`: a launch problem row carries no badge, whatever `attribution().badges`
    /// holds — only a change row is badged.
    #[test]
    fn a_launch_problem_row_carries_no_badge() {
        let mut d = dashboard_with(
            vec![fixture::active("alpha", 1, 2)],
            Vec::new(),
            Vec::new(),
            0,
        );
        d.launch.problems = vec!["launch failed".to_string()];
        d.agents.agents = vec![agent_at("alpha", AgentStatus::Working)];
        for width in [38, 58] {
            let all = rows(&d, width);
            assert_eq!(all[0].kind, RowKind::Problem, "width {width}");
            assert!(
                !all[0].text.contains(" w ["),
                "width {width}: a launch problem row must carry no badge"
            );
            assert!(
                all[1].text.contains(" w ["),
                "width {width}: the change row below must still carry its badge"
            );
        }
    }

    #[test]
    fn a_refresh_problem_row_degrades_at_narrow_widths() {
        let mut d = dashboard_with(Vec::new(), Vec::new(), Vec::new(), 0);
        d.refresh.problems = vec!["watch failed entirely".to_string()];
        for width in [38, 58] {
            assert_eq!(rows(&d, width)[0].kind, RowKind::Problem, "width {width}");
            assert_eq!(
                rows(&d, width)[0].text.chars().count(),
                width as usize,
                "width {width}"
            );
        }
        // The degenerate widths this scenario is named for: the row falls
        // back to the first `width` characters of `"! "` itself.
        assert_eq!(rows(&d, 1)[0].text, "!");
        assert_eq!(rows(&d, 0)[0].text, "");
    }
}
