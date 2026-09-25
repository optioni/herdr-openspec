//! The worktree family a change's directory may belong to, and the
//! functions that decide which member owns it.
//!
//! Pure and total on exactly `crate::integration`'s terms — no filesystem,
//! process, environment, network, or standard-I/O API, no clock, no global
//! mutable state, and no view-layer type. It lives outside `src/ui/`
//! deliberately, for the reason `crate::specs` and `crate::integration` do: a
//! new pure-view file would move `NOIO-VIEW`'s "eleven pure files" and
//! `COLWIDTH`'s "ten pure view files", two counts four documents carry.
//!
//! This module holds the shared value type ([`Worktree`]), the one
//! derivation `worktree-overlay` names as the crate's only membership test
//! ([`member_of`]), parsing of `git worktree list --porcelain -z`'s own
//! stdout ([`Record`], [`parse_list`], [`label`]), the family-selection rule
//! choosing a base and its members ([`family_with_tops`]), and classifying which
//! change directories a member touched ([`Touched`], [`touched`]). All of it
//! stays pure: [`family`] takes canonical paths its caller already resolved rather than
//! calling into the filesystem itself (design.md -> D15) — the refresh
//! worker is where canonicalization and the `git` calls themselves happen.
//! See `specs/worktree-overlay/spec.md`.

use std::collections::BTreeSet;
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

/// One record from `git worktree list --porcelain -z`'s stdout: the raw
/// (not yet made canonical) top-level path it names, its `HEAD` and
/// `branch` fields when present, and its state flags. A record with no
/// `worktree` field is never represented here — [`parse_list`] skips it
/// entirely, so `path` is always present on anything this module hands
/// back. See specs/worktree-overlay/spec.md -> "The worktree family is read
/// from `git worktree list`, and only live members join it".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub path: String,
    pub head: Option<String>,
    pub branch: Option<String>,
    pub detached: bool,
    pub bare: bool,
    pub locked: bool,
    pub prunable: bool,
}

/// Parses `git worktree list --porcelain -z`'s stdout into one [`Record`]
/// per worktree. Pure and total: records are separated by an empty
/// NUL-terminated field (i.e. `\0\0` in the raw stream), and within a
/// record each `\0`-terminated line is either `worktree <path>`,
/// `HEAD <sha>`, `branch <ref>`, or one of the bare words `detached`,
/// `bare`, `locked`, `prunable` — each optionally followed by ` <reason>`,
/// which is ignored. Any other line is ignored, so a newer git that adds a
/// field never empties the family. A record with no `worktree` line is
/// skipped and never appears in the result.
pub fn parse_list(stdout: &str) -> Vec<Record> {
    stdout
        .split("\0\0")
        .filter(|chunk| !chunk.is_empty())
        .filter_map(parse_record)
        .collect()
}

fn parse_record(chunk: &str) -> Option<Record> {
    let mut path = None;
    let mut head = None;
    let mut branch = None;
    let mut detached = false;
    let mut bare = false;
    let mut locked = false;
    let mut prunable = false;

    for line in chunk.split('\0') {
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("worktree ") {
            path = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("HEAD ") {
            head = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("branch ") {
            branch = Some(rest.to_string());
        } else if line == "detached" || line.starts_with("detached ") {
            detached = true;
        } else if line == "bare" || line.starts_with("bare ") {
            bare = true;
        } else if line == "locked" || line.starts_with("locked ") {
            locked = true;
        } else if line == "prunable" || line.starts_with("prunable ") {
            prunable = true;
        }
        // A field this requirement does not name is ignored.
    }

    path.map(|path| Record {
        path,
        head,
        branch,
        detached,
        bare,
        locked,
        prunable,
    })
}

/// A member's label: its branch with a leading `refs/heads/` removed, the
/// full ref when it has another prefix (e.g. `refs/remotes/origin/x`), or
/// the first seven characters of its `HEAD` when it is `detached`. Neither
/// a branch nor a detached `HEAD` yields an empty label — this only arises
/// for a record `family` would not have turned into a member in the first
/// place (bare, prunable, or unresolvable).
pub fn label(record: &Record) -> String {
    if let Some(branch) = &record.branch {
        branch
            .strip_prefix("refs/heads/")
            .map(str::to_string)
            .unwrap_or_else(|| branch.clone())
    } else if record.detached {
        record
            .head
            .as_deref()
            .map(|head| head.chars().take(7).collect())
            .unwrap_or_default()
    } else {
        String::new()
    }
}

/// Selects the worktree family for the pane's own canonical root, given the
/// parsed records and their canonical top levels in the same order (`None`
/// where the worker could not resolve a record's canonical path). Per design.md
/// -> D10, the **base** is the record whose canonical top level is the
/// **longest** one that is equal to, or an ancestor of, `pane_root`; the
/// pane's root relative to that top level is the OpenSpec prefix, empty
/// when the two are equal. Every other record joins the family as a member
/// unless it is `bare`, carries `prunable`, or has no canonical path —
/// each skipped without any problem being recorded, since this module has
/// no such concept. A member's [`Worktree::root`] (its OpenSpec root) is
/// its own canonical top level joined to the OpenSpec prefix, matching
/// [`member_of`]'s expectation that `root` already points at the directory
/// whose `openspec/changes` subdirectory holds the member's changes.
///
/// Pure per design.md -> D15: `canonical` arrives already resolved
/// by the caller (the refresh worker), and this function never touches the
/// filesystem.
pub fn family(
    records: &[Record],
    canonical: &[Option<PathBuf>],
    pane_root: &Path,
) -> Vec<Worktree> {
    family_with_tops(records, canonical, pane_root)
        .1
        .into_iter()
        .map(|(_, worktree)| worktree)
        .collect()
}

/// [`family`]'s own selection, additionally handing back the OpenSpec prefix
/// (returned once, since every member shares the one prefix the base's own record
/// determines) and, paired with each [`Worktree`], the member's own canonical top
/// level — the path *before* the prefix was joined onto it to make `Worktree::root`.
/// `family` is a thin wrapper over this that keeps its existing signature and drops
/// both, so the two can never disagree about which records are members.
///
/// The refresh worker needs the top level, not the OpenSpec root, for `-C`: git's
/// `merge-base`/`diff-tree`/`status` are run against a member's checkout, and
/// `diff-tree`/`status` report paths relative to that checkout's own top, never
/// relative to `-C`'s directory — so a pathspec and a `touched` prefix must be the
/// OpenSpec prefix joined to `openspec/changes`, not `openspec/changes` alone, or
/// every path a nonempty prefix produces silently fails to match. See
/// specs/worktree-overlay/spec.md -> "A member owns exactly the changes it touched
/// since it forked from the base".
pub fn family_with_tops(
    records: &[Record],
    canonical: &[Option<PathBuf>],
    pane_root: &Path,
) -> (PathBuf, Vec<(PathBuf, Worktree)>) {
    let base_index = canonical
        .iter()
        .enumerate()
        .filter_map(|(index, top_level)| top_level.as_ref().map(|path| (index, path)))
        .filter(|(_, top_level)| pane_root.starts_with(top_level.as_path()))
        .max_by_key(|(_, top_level)| top_level.as_os_str().len())
        .map(|(index, _)| index);

    let prefix = base_index
        .and_then(|index| canonical[index].as_ref())
        .and_then(|base_root| pane_root.strip_prefix(base_root).ok())
        .map(Path::to_path_buf)
        .unwrap_or_default();

    let members = records
        .iter()
        .zip(canonical.iter())
        .enumerate()
        .filter(|(index, _)| Some(*index) != base_index)
        .filter_map(|(_, (record, top_level))| {
            if record.bare || record.prunable {
                return None;
            }
            let top_level = top_level.as_ref()?;
            let root = if prefix.as_os_str().is_empty() {
                top_level.clone()
            } else {
                top_level.join(&prefix)
            };
            Some((
                top_level.clone(),
                Worktree {
                    root,
                    label: label(record),
                },
            ))
        })
        .collect();

    (prefix, members)
}

/// Which change directories a member touched since it forked from the base:
/// the active change names and the archived change directory names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Touched {
    pub active: BTreeSet<String>,
    pub archived: BTreeSet<String>,
}

/// Reads every path out of `diff_tree` (bare NUL-terminated paths, as
/// `git diff-tree --no-renames -z --name-only` prints them) and `status` (a
/// `git status --no-renames -z` entry: two status characters, a space, then
/// the path), keeps those beginning with `<changes>/`, and classifies each
/// by what follows that prefix: `archive/<dir>/…` touches the archived
/// directory `<dir>`, and `<name>/…` with `<name>` not `archive` touches the
/// active change `<name>`. A path naming a file directly inside
/// `openspec/changes/` or directly inside `archive/` touches nothing. Pure,
/// total, and never panics on an empty or unterminated input.
pub fn touched(diff_tree: &str, status: &str, changes: &str) -> Touched {
    let mut result = Touched::default();
    let prefix = format!("{changes}/");

    for path in diff_tree.split('\0').filter(|line| !line.is_empty()) {
        classify(path, &prefix, &mut result);
    }

    for entry in status.split('\0').filter(|line| !line.is_empty()) {
        if let Some(path) = entry.get(3..) {
            classify(path, &prefix, &mut result);
        }
    }

    result
}

fn classify(path: &str, prefix: &str, result: &mut Touched) {
    let Some(rest) = path.strip_prefix(prefix) else {
        return;
    };
    if let Some(after_archive) = rest.strip_prefix("archive/") {
        if let Some((dir, _)) = after_archive.split_once('/') {
            result.archived.insert(dir.to_string());
        }
    } else if let Some((name, _)) = rest.split_once('/') {
        result.active.insert(name.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Record, Touched, Worktree, family_with_tops, label, member_of, parse_list, touched,
    };
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

    /// Change Review follow-up (`worktree-changes` task 11.2, item 1):
    /// `specs/worktree-overlay/spec.md` says "A listing with no record
    /// containing the root SHALL yield empty `worktrees`" — with no record
    /// an ancestor of (or equal to) `pane_root`, `family_with_tops` must
    /// return `None` rather than treating every resolvable, non-bare record
    /// as a member. Before this fix the pure rule disagreed with the spec;
    /// only `refresh::derive_family`'s own duplicated early return made the
    /// spec's promise hold in practice.
    #[test]
    fn no_base_yields_no_family() {
        let records = vec![Record {
            path: "/w/feat".to_string(),
            head: Some("bbbb".to_string()),
            branch: Some("refs/heads/feat".to_string()),
            detached: false,
            bare: false,
            locked: false,
            prunable: false,
        }];
        let canonical = vec![Some(PathBuf::from("/w/feat"))];

        assert_eq!(
            family_with_tops(&records, &canonical, &PathBuf::from("/nowhere")),
            None
        );
    }

    /// worktree-overlay, "A porcelain listing with a main checkout, a
    /// branch, a detached head, and a prunable entry": four records parse in
    /// order, the last carrying `prunable`; with the pane's root `/r`, the
    /// family selection yields `/r` as the base (empty prefix) and members
    /// exactly `[(/w/feat, "feat"), (/w/det, "ccccccc")]`, the prunable
    /// record skipped with no problem recorded.
    #[test]
    fn a_porcelain_listing_with_a_main_checkout_a_branch_a_detached_head_and_a_prunable_entry() {
        let stdout = "worktree /r\0HEAD aaaa\0branch refs/heads/main\0\0\
worktree /w/feat\0HEAD bbbb\0branch refs/heads/feat\0\0\
worktree /w/det\0HEAD cccccccccc\0detached\0\0\
worktree /w/gone\0HEAD dddd\0detached\0prunable gitdir file points to non-existent location\0\0";

        let records = parse_list(stdout);
        assert_eq!(records.len(), 4);
        assert!(records[3].prunable);

        let canonical: Vec<Option<PathBuf>> = records
            .iter()
            .map(|record| Some(PathBuf::from(&record.path)))
            .collect();
        let members = family(&records, &canonical, &PathBuf::from("/r"));

        assert_eq!(
            members,
            vec![worktree("/w/feat", "feat"), worktree("/w/det", "ccccccc")]
        );
    }

    /// worktree-overlay, "An unknown field and a record with no path are
    /// tolerated": an extra `future-field value` line is ignored, a record
    /// holding only `HEAD eeee` is skipped entirely, and `parse_list("")`
    /// never panics.
    #[test]
    fn an_unknown_field_and_a_record_with_no_path_are_tolerated() {
        let stdout = "worktree /r\0HEAD aaaa\0future-field value\0\0HEAD eeee\0\0";

        let records = parse_list(stdout);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].path, "/r");
        assert_eq!(records[0].head.as_deref(), Some("aaaa"));

        assert_eq!(parse_list(""), Vec::new());
    }

    /// worktree-overlay, "The OpenSpec root inside a member follows the
    /// pane's own prefix": with the pane's root `/r/sub`, base top level
    /// `/r`, and a member at `/w/feat`, the OpenSpec prefix is `sub` and the
    /// member's OpenSpec root is `/w/feat/sub`.
    #[test]
    fn the_openspec_root_inside_a_member_follows_the_panes_own_prefix() {
        let records = vec![
            Record {
                path: "/r".to_string(),
                head: Some("aaaa".to_string()),
                branch: Some("refs/heads/main".to_string()),
                detached: false,
                bare: false,
                locked: false,
                prunable: false,
            },
            Record {
                path: "/w/feat".to_string(),
                head: Some("bbbb".to_string()),
                branch: Some("refs/heads/feat".to_string()),
                detached: false,
                bare: false,
                locked: false,
                prunable: false,
            },
        ];
        let canonical = vec![Some(PathBuf::from("/r")), Some(PathBuf::from("/w/feat"))];

        let members = family(&records, &canonical, &PathBuf::from("/r/sub"));

        assert_eq!(members, vec![worktree("/w/feat/sub", "feat")]);
    }

    /// worktree-overlay, "A pane opened inside a linked worktree treats the
    /// main checkout as a member": with the pane's root `/w/feat` and git
    /// listing `/r` (branch `main`) first, `/w/feat` (branch `feat`) is the
    /// base and the members are `[(/r, "main")]`.
    #[test]
    fn a_pane_opened_inside_a_linked_worktree_treats_the_main_checkout_as_a_member() {
        let records = vec![
            Record {
                path: "/r".to_string(),
                head: Some("aaaa".to_string()),
                branch: Some("refs/heads/main".to_string()),
                detached: false,
                bare: false,
                locked: false,
                prunable: false,
            },
            Record {
                path: "/w/feat".to_string(),
                head: Some("bbbb".to_string()),
                branch: Some("refs/heads/feat".to_string()),
                detached: false,
                bare: false,
                locked: false,
                prunable: false,
            },
        ];
        let canonical = vec![Some(PathBuf::from("/r")), Some(PathBuf::from("/w/feat"))];

        let members = family(&records, &canonical, &PathBuf::from("/w/feat"));

        assert_eq!(members, vec![worktree("/r", "main")]);
    }

    /// worktree-overlay, "A worktree nested inside the main checkout is the
    /// base when the pane is in it": with the pane's root
    /// `/r/.worktrees/feat` and git listing `/r` (branch `main`) then
    /// `/r/.worktrees/feat` (branch `feat`) — both ancestors of or equal to
    /// the pane's root — the base is the **longer** match,
    /// `/r/.worktrees/feat`, and the members are `[(/r, "main")]`. An
    /// implementation that took the first containing record would wrongly
    /// pick `/r` as the base.
    #[test]
    fn a_worktree_nested_inside_the_main_checkout_is_the_base_when_the_pane_is_in_it() {
        let records = vec![
            Record {
                path: "/r".to_string(),
                head: Some("aaaa".to_string()),
                branch: Some("refs/heads/main".to_string()),
                detached: false,
                bare: false,
                locked: false,
                prunable: false,
            },
            Record {
                path: "/r/.worktrees/feat".to_string(),
                head: Some("bbbb".to_string()),
                branch: Some("refs/heads/feat".to_string()),
                detached: false,
                bare: false,
                locked: false,
                prunable: false,
            },
        ];
        let canonical = vec![
            Some(PathBuf::from("/r")),
            Some(PathBuf::from("/r/.worktrees/feat")),
        ];

        let members = family(&records, &canonical, &PathBuf::from("/r/.worktrees/feat"));

        assert_eq!(members, vec![worktree("/r", "main")]);
    }

    /// worktree-overlay, "Bare, unresolvable, and oddly named records": a
    /// `bare` record and a record whose path cannot be canonicalized
    /// (simulated with `None`) are skipped with no problem recorded; a
    /// detached record and one on `branch refs/remotes/origin/x` become
    /// members labelled with the first seven characters of `HEAD` and the
    /// full ref respectively — `label` only strips a leading
    /// `refs/heads/` prefix.
    #[test]
    fn bare_unresolvable_and_oddly_named_records() {
        let records = vec![
            Record {
                path: "/bare".to_string(),
                head: Some("aaaa".to_string()),
                branch: None,
                detached: false,
                bare: true,
                locked: false,
                prunable: false,
            },
            Record {
                path: "/gone".to_string(),
                head: Some("bbbb".to_string()),
                branch: Some("refs/heads/gone".to_string()),
                detached: false,
                bare: false,
                locked: false,
                prunable: false,
            },
            Record {
                path: "/det".to_string(),
                head: Some("cccccccccc".to_string()),
                branch: None,
                detached: true,
                bare: false,
                locked: false,
                prunable: false,
            },
            Record {
                path: "/remote".to_string(),
                head: Some("dddd".to_string()),
                branch: Some("refs/remotes/origin/x".to_string()),
                detached: false,
                bare: false,
                locked: false,
                prunable: false,
            },
        ];
        let canonical = vec![
            Some(PathBuf::from("/bare")),
            None,
            Some(PathBuf::from("/det")),
            Some(PathBuf::from("/remote")),
        ];

        // No record here is an ancestor of this pane root, so none is
        // selected as the base — the point of this scenario is which
        // records are skipped as members, not base selection.
        let members = family(&records, &canonical, &PathBuf::from("/nowhere"));

        assert_eq!(
            members,
            vec![
                worktree("/det", "ccccccc"),
                worktree("/remote", "refs/remotes/origin/x")
            ]
        );

        assert_eq!(label(&records[3]), "refs/remotes/origin/x");
    }

    /// worktree-overlay, "A committed edit, a committed archive, and
    /// uncommitted work are all owned": a member's `diff-tree` output names
    /// a committed edit to an active change and a committed archive, its
    /// `status` output names an uncommitted edit and an untracked file, and
    /// `touched` returns the active names `{x, y, z}` and the archived
    /// directory `{2026-09-24-y}` — `y` appearing in both is correct here,
    /// `touched` does no cross-checking against the base.
    #[test]
    fn a_committed_edit_a_committed_archive_and_uncommitted_work_are_all_owned() {
        let diff_tree =
            "openspec/changes/archive/2026-09-24-y/tasks.md\0openspec/changes/y/tasks.md\0";
        let status = " M openspec/changes/x/tasks.md\0?? openspec/changes/z/proposal.md\0";

        let result = touched(diff_tree, status, "openspec/changes");

        assert_eq!(
            result,
            Touched {
                active: ["x", "y", "z"].into_iter().map(String::from).collect(),
                archived: ["2026-09-24-y"].into_iter().map(String::from).collect(),
            }
        );
    }

    /// worktree-overlay, "Paths outside a change directory touch nothing":
    /// a file directly inside `openspec/changes/`, a file directly inside
    /// `archive/`, a path outside `openspec/changes` entirely, and a path
    /// only nested under it all touch nothing; an empty or unterminated
    /// input never panics.
    #[test]
    fn paths_outside_a_change_directory_touch_nothing() {
        let diff_tree = "openspec/changes/README.md\0openspec/changes/archive/notes.md\0openspec/specs/a/spec.md\0sub/openspec/changes/x/tasks.md";

        let result = touched(diff_tree, "", "openspec/changes");

        assert_eq!(result, Touched::default());
        assert_eq!(touched("", "", "openspec/changes"), Touched::default());
    }
}
