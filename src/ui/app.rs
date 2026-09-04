//! `Dashboard`, `Route`, and pure key handling. See
//! `openspec/changes/tui-shell/design.md` -> Boundaries and Contracts.

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
