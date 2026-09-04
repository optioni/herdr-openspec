## ADDED Requirements

### Requirement: The `npm prefix -g` step is an injected hook bound to a real probe in `cli`

Step 4 needs the output of `npm prefix -g`, which requires spawning a process. No module
in this crate may spawn one outside `cli`. The chain SHALL therefore continue to take the
npm prefix as an injected `&dyn Fn() -> Option<PathBuf>`, following the same pattern by
which configuration takes the process environment as an injected lookup — that injection
is what keeps `resolve` pure and unit-testable, and it survives the seam landing rather
than being replaced by it.

What changes is the binding. The production binding SHALL be the real probe published by
the `subprocess-seam` capability, which starts the npm program with the arguments `prefix`
and `-g`, reads its **stdout only**, trimmed, and reports no prefix when the program could
not be started, exited non-zero, or produced empty output. `resolve::openspec_bin_from_env`
SHALL pass that binding and no other.

`resolve` SHALL expose no npm binding of its own. The placeholder that always returned
nothing SHALL be removed rather than left beside the real one.

`src/resolve.rs` SHALL name no process API at all — no `std::process`, no `Command`, no
`Stdio` — including in its comments, because the check reads source text and cannot tell a
comment from a call. This clause is scoped to `src/resolve.rs` deliberately; the tree-wide
form of the rule, with `src/cli.rs` excluded as the one module permitted to spawn, is a
requirement of the `subprocess-seam` capability rather than of this one.

#### Scenario: The hook is still injected, so the chain stays pure

- **WHEN** `openspec_bin` is called with nothing configured, a `PATH` holding no
  `openspec`, no nvm tree, and a fourth-step hook that is an ordinary closure returning a
  scratch directory `N` holding an executable `N/bin/openspec`
- **THEN** `N/bin/openspec` is resolved with the npm-prefix step as its source
- **AND** the call spawns nothing, because the collaborator is a closure and `resolve` has
  no other way to reach a process

#### Scenario: The production binding is the real probe

- **WHEN** `src/resolve.rs` is searched for the identifier the composition passes as its
  fourth-step hook
- **THEN** it names `cli::npm_prefix`, and searching all of `src/` for
  `npm_prefix_deferred` yields no match
- **AND** both halves are needed: the absence check alone passes for a hand-over in which
  the placeholder was renamed and still returns nothing, and every other check in this
  change — the chain tests, the no-spawn greps, the no-tools suite run, and the binding's
  own smoke test — stays green for that implementation, because no prefix is a legitimate
  answer

#### Scenario: Resolution spawns no process

- **WHEN** the crate's own tests are run on a `PATH` from which every directory
  containing `npm`, `node`, or `openspec` has been removed, having first asserted that
  all three are unresolvable on it and that `cargo` and `rustc` still are
- **THEN** every resolution test still passes
- **AND** `src/resolve.rs` names no process API at all — no `std::process`, no `Command`,
  no `Stdio` — including in its comments, because the check reads source text and cannot
  tell a comment from a call
- **AND** the whole suite passes on that `PATH` too, including `cli`'s own tests, because
  every spawning test names an absolute scratch program path

## REMOVED Requirements

### Requirement: The `npm prefix -g` step is an injected hook, deferred until the subprocess seam exists

**Reason**: The deferral it describes has ended. `subprocess-seam` builds `cli` and binds
step 4 to a real `npm prefix -g` probe, so the requirement's central clause — "the binding
this change ships SHALL return no prefix, so step 4 contributes nothing in production" —
is now false by design, and its scenario "The shipped hook yields no prefix" is the
hand-over signal `repo-resolution` planted to go red at exactly this moment. The
requirement's still-live content is carried forward, unweakened, by "The `npm prefix -g`
step is an injected hook bound to a real probe in `cli`" above: the hook stays injected,
`src/resolve.rs` still names no process API, and the no-tools suite run still holds. The
removed requirement's own note anticipated this, recording that its tree-wide grep was a
task-level check "that `subprocess-seam` will rescope".

**Migration**: No consumer migration is needed — nothing outside the crate depended on the
placeholder. Inside the crate, `resolve::npm_prefix_deferred` is deleted and
`resolve::openspec_bin_from_env` passes `cli`'s real probe instead; callers that injected
their own hook, which is every test of the chain, are unaffected because the injection
point is unchanged. The tree-wide no-spawn grep moves to the `subprocess-seam`
capability, where it is stated with `src/cli.rs` excluded and with guards that fail when
the exclusion is vacuous.
