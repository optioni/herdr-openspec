## MODIFIED Requirements

### Requirement: Every tier is scoped to the resolved repository

`herdr agent list` is **session-global**: measured against Herdr 0.8.2, it returned
byte-identical output from three different working directories and listed agents whose `cwd` lay
in a repository other than the current one. `SPEC.md` scopes only tier 3 to the repository;
every tier SHALL be scoped to it, or an agent renamed `add-auth` in an unrelated repository
would badge this repository's `add-auth` row.

An agent SHALL be **in scope** exactly when `repo` is `Some(root)` and the agent's `cwd` is
`Some(path)` and `path.starts_with(root)` — Rust's component-wise prefix test, which is true for
an equal path and false for a sibling whose textual prefix matches. An agent that is not in
scope SHALL be neither badged nor counted: it contributes nothing to `badges` and nothing to
`unattributed`, and is invisible to the pane.

An agent carrying no `cwd` SHALL be out of scope. `cwd` is not one of the seven fields Herdr's
own schema marks required, so its absence is an ordinary payload, not a fault; but an agent
whose working directory is unknown cannot be shown to be in this repository, and counting it
would be a guess.

Both sides of the comparison SHALL be canonical by the time they reach `attribute`, and
`attribute` SHALL NOT canonicalize either: it performs no filesystem I/O, so it cannot.
`resolve::find_repo` already canonicalizes the root it returns. Herdr, however, reports the
working directory **verbatim as the pane process was given it**, which is not canonical
whenever the repository is reached through a symbolic link — `/tmp` resolving to
`/private/tmp` on macOS, a symlinked home directory, a path under `/Volumes`. In that state
the component-wise prefix test fails for **every** agent, and because the scope test precedes
the unattributed count, the agents are not even reported as a number: every badge and the
footer count vanish with no problem row and no other signal.

Canonicalization SHALL therefore happen once, on the poller's own side of the seam, before a
snapshot reaches `attribute`: `agents::AgentSnapshot`'s agents SHALL carry a `cwd` that has
been passed through the same canonicalizing step `resolve::find_repo` applies to the root.
The step SHALL be an injected `&dyn Fn(&Path) -> Option<PathBuf>` on exactly
`config::env_lookup`'s and `resolve::openspec_bin`'s npm-hook terms, so the rule is driven by
a test without a real symlink, and its one production binding SHALL live outside `src/ui/`
alongside the poller. A path that cannot be canonicalized — it no longer exists, or the
process cannot resolve it — SHALL be kept **verbatim** rather than dropped: a stale directory
is still better evidence than none, and dropping it would reintroduce the same silent
disappearance from the other direction.

`attribute` SHALL remain a pure, total function performing no filesystem I/O, and its
signature SHALL be unchanged: the canonicalization is a property of the values it is handed,
never of the function.

#### Scenario: An agent in another repository is neither badged nor counted

- **WHEN** `attribute` is called with two agents, both `Working`: the first with `cwd`
  `/other/repo` and `name` `Some("add-auth")`, the second with `cwd` `/repo` and `name`
  `Some("nothing-like-a-change")`; `repo` `Some("/repo")`; `change_names` `["add-auth"]`; and an
  empty mapping
- **THEN** `badges` is empty — the out-of-scope agent's name matched a change and was still not
  attributed
- **AND** `unattributed` is exactly `1`, counting the in-scope agent alone and not the
  out-of-scope one
- **AND** removing the scope test is still discriminating here, though not through the count:
  this fixture's out-of-scope agent shares its name with the one change on screen, so without
  the scope test it would be placed by the **name tier** rather than counted — `badges` would
  become `{"add-auth": Working}` instead of staying empty, and `unattributed` would stay `1`.
  The count-moves-to-`2` direction is proven by `agent-poller`'s outer-loop scenario below,
  whose out-of-scope agent is deliberately named to match **no** change on screen for exactly
  this reason — a repair recorded in planning-review.md after task 8.3 found the same collision
  in that scenario's own fixture

#### Scenario: A subdirectory is inside the repository and a sibling prefix is not

- **WHEN** `attribute` is called with `repo` `Some("/repo")`, an empty mapping, `change_names`
  `[]`, and three `Idle` agents whose `cwd` values are `/repo`, `/repo/openspec/changes/alpha`,
  and `/repo-other`
- **THEN** `unattributed` is exactly `2`: the agent at the root and the agent in the
  subdirectory are in scope, and the sibling `/repo-other` is not
- **AND** the sibling's exclusion is a component-wise comparison rather than a string prefix,
  which a textual `starts_with` on the display string would have admitted

#### Scenario: No repository means no badges and no count

- **WHEN** `attribute` is called with `repo` `None`, three agents whose `cwd` values are all
  `/repo`, `change_names` `["add-auth"]`, and a mapping holding `c-add-auth -> add-auth`
- **THEN** `badges` is empty and `unattributed` is `0`
- **AND** nothing panics: a pane that never found a repository has no scope to attribute within,
  and its list has no rows to badge

#### Scenario: A symlinked repository path still badges its agents

- **WHEN** the poller's canonicalizing hook is a test closure mapping `/tmp/repo` to
  `/private/tmp/repo` and every other path to itself, Herdr reports one `Working` agent whose
  `cwd` is `/tmp/repo` and whose `name` is `Some("add-auth")`, and the resolved root is the
  canonical `/private/tmp/repo` with `change_names` `["add-auth"]`
- **THEN** the snapshot reaching `attribute` carries `cwd` `/private/tmp/repo`, and `badges` is
  `{"add-auth": Working}`
- **AND** without the canonicalizing step the same fixture yields empty `badges` **and**
  `unattributed` `0` — the whole-session disappearance this scenario exists to catch — so the
  test is discriminating in both directions
- **AND** the hook is an injected closure, so no symbolic link is created on disk and the
  scenario runs identically on both supported platforms

#### Scenario: An unresolvable working directory is kept verbatim, not dropped

- **WHEN** the canonicalizing hook returns `None` for an agent's `cwd` `/repo/gone`, the
  resolved root is `/repo`, and `change_names` is `[]`
- **THEN** the snapshot carries `cwd` `/repo/gone` unchanged and the agent is counted:
  `unattributed` is `1`
- **AND** an agent whose directory has since been deleted is therefore still reported as a
  number rather than silently vanishing, which is the same failure this requirement's
  canonicalization exists to remove
