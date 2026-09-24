//! The worktree family a change's directory may belong to, and the one
//! function that decides which member owns it.
//!
//! Pure and total on exactly `crate::integration`'s terms — no filesystem,
//! process, environment, network, or standard-I/O API, no clock, no global
//! mutable state, and no view-layer type. It lives outside `src/ui/`
//! deliberately, for the reason `crate::specs` and `crate::integration` do: a
//! new pure-view file would move `NOIO-VIEW`'s "eleven pure files" and
//! `COLWIDTH`'s "ten pure view files", two counts four documents carry.
//!
//! This module currently holds only the shared value type and the one
//! derivation `worktree-overlay` names as the crate's only one: [`member_of`].
//! Parsing `git worktree list`'s own stdout, deciding which member a touched
//! path names, and the family-selection rule are a later change's addition
//! to this file, not this group's. See `specs/worktree-overlay/spec.md`.

use std::path::{Path, PathBuf};

/// One member of the worktree family the pane was launched against: an
/// OpenSpec root and the label `git worktree list` gave it (its branch name,
/// or a short hash when detached). Never includes the pane's own root —
/// `worktree-overlay` owns how the list itself is derived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worktree {
    pub root: PathBuf,
    pub label: String,
}

/// The one derivation of "which member's copy is this change" in the crate:
/// the member whose `<root>/openspec/changes` directory `dir` lies under,
/// component-wise (`Path::starts_with`), or `None` when there is none.
///
/// The test is against `<root>/openspec/changes`, never against `<root>`
/// alone, because a member's root can be an **ancestor** of the pane's own
/// root: with the pane inside `/r/.worktrees/feat` and the main checkout `/r`
/// a member, every one of the pane's own changes lies under `/r`, but none
/// lies under `/r/openspec/changes`. Two members' changes directories cannot
/// nest, so at most one member matches. See `specs/worktree-overlay/spec.md`
/// -> "A change's worktree is found by its changes directory, in one place".
pub fn member_of<'a>(members: &'a [Worktree], dir: &Path) -> Option<&'a Worktree> {
    members
        .iter()
        .find(|member| dir.starts_with(member.root.join("openspec").join("changes")))
}

#[cfg(test)]
mod tests {
    use super::{Worktree, member_of};
    use std::path::PathBuf;

    fn worktree(root: &str, label: &str) -> Worktree {
        Worktree {
            root: PathBuf::from(root),
            label: label.to_string(),
        }
    }

    /// worktree-overlay, "A member's change is found and the pane's own is
    /// not": given one member `/w/feat`, both an active and an archived
    /// change under its `openspec/changes` resolve to it, and a change under
    /// a different root resolves to `None`.
    #[test]
    fn a_members_change_is_found_and_the_panes_own_is_not() {
        let members = vec![worktree("/w/feat", "feat")];

        assert_eq!(
            member_of(&members, &PathBuf::from("/w/feat/openspec/changes/x")),
            Some(&members[0])
        );
        assert_eq!(
            member_of(
                &members,
                &PathBuf::from("/w/feat/openspec/changes/archive/2026-09-24-y"),
            ),
            Some(&members[0])
        );
        assert_eq!(
            member_of(&members, &PathBuf::from("/r/openspec/changes/z")),
            None
        );
    }

    /// worktree-overlay, "A nested layout does not claim the pane's own
    /// rows": the main checkout `/r` is a member, and a change under
    /// `/r/.worktrees/feat/openspec/changes` — nested under `/r` but not
    /// under `/r/openspec/changes` — must not resolve to it, although the
    /// pane's own `/r/openspec/changes/y` does.
    #[test]
    fn a_nested_layout_does_not_claim_the_panes_own_rows() {
        let members = vec![worktree("/r", "main")];

        assert_eq!(
            member_of(
                &members,
                &PathBuf::from("/r/.worktrees/feat/openspec/changes/x"),
            ),
            None,
            "a directory nested under /r but not under /r/openspec/changes must not match"
        );
        assert_eq!(
            member_of(&members, &PathBuf::from("/r/openspec/changes/y")),
            Some(&members[0])
        );
    }

    /// worktree-overlay, "Two nested members resolve to the right one": with
    /// both `/r` and its own nested `/r/.worktrees/b` as members, a change
    /// under the nested member's own changes directory resolves to that
    /// member, not the outer one.
    #[test]
    fn two_nested_members_resolve_to_the_right_one() {
        let members = vec![worktree("/r", "main"), worktree("/r/.worktrees/b", "b")];

        let found = member_of(
            &members,
            &PathBuf::from("/r/.worktrees/b/openspec/changes/y"),
        );
        assert_eq!(found.map(|w| w.label.as_str()), Some("b"));
    }
}
