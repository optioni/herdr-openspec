## MODIFIED Requirements

### Requirement: Both producers of a change build every field, enforced at compile time

The plugin SHALL keep `changes::from_files` and `changes::from_cli` producing the same
`Change` by three mechanisms, none of which is a comment:

1. **No `Default`.** None of `Change`, `ChangeSet`, `ArtifactRef`, or `Origin` derives or
   implements `Default` **anywhere in the crate** — `impl Default for Change` is legal in
   any file, so the rule is stated over `src/`, not over one module — and no expression
   anywhere in `src/changes.rs` uses a `..` functional update or a `..` rest pattern in a
   literal or a pattern for any of those four types. Every construction site therefore
   names every field, and a new field is a compile error (`E0063`) at each of them.

   The `..` half is stated separately from the `Default` half because it is **not implied**
   by it: `Change { name, ..other }` compiles with no `Default` anywhere in the crate, and
   a check that looked only for `Default` would pass over it. Both halves are checked.

2. **A shared conformance function that destructures exhaustively.** A test-only
   `changes::conformance::assert_invariants(&Change)` SHALL open with an exhaustive `let
   Change { … }` pattern carrying **no** `..` rest pattern, so adding a field makes that one
   function fail to compile (`E0027`). Because it is the function *both* producers' test
   suites call, whoever adds a field is forced to state what the new field means for both
   sources rather than for the one they were working on.

3. **Every producer's tests call it.** Every `Change` a producer's test builds SHALL be
   passed through `assert_invariants`, which checks the source-independent invariants: a
   non-empty `name`; a non-empty `schema`; a `dir` whose final component equals `name` for
   an `Active` change and ends with `name` for an `Archived` one; an `ArtifactRef` with a
   non-empty `id` for every entry; and `problems` holding no empty string, so it is used
   only for human-readable text and never to carry data a field should hold. It SHALL NOT
   assert that artifact ids are unique — the landed `schema-artifacts` capability requires
   that a schema's artifact list is "never de-duplicated", so a schema declaring the same
   id twice legitimately produces two `ArtifactRef`s with one id.

Mechanisms 1 and 2 SHALL both be checked, and each SHALL be shown to catch what the other
misses. Neither alone is sufficient and neither substitutes for the other.

The set of construction sites the mechanisms bind is **every** site that builds a `Change`,
not only the two named producers. `changes::merge` builds merged `Change` values from a
pair of producer values and is a third such site; a field it filled by default would drift
exactly as a producer's would.

The plugin SHALL NOT satisfy this requirement with a `#[non_exhaustive]` attribute, which
constrains only other crates, nor by making fields private, which constrains nothing within
one module.

#### Scenario: Adding a field to `Change` fails to compile in the shared conformance function

- **WHEN** a field is added to `Change` and only `from_files`' construction site is updated
- **THEN** `cargo test --all-features` fails to compile, reporting that the pattern in
  `conformance::assert_invariants` does not mention the new field
- **AND** the error names the field, so the omission cannot be resolved by adding a rest
  pattern without deliberately deleting the enforcement

#### Scenario: Adding a field breaks both mechanisms, and each catches what the other misses

- **WHEN** a field is added to `Change` in a throwaway copy of the crate and nothing else
  is changed
- **THEN** the build fails with **both** `E0027` in `conformance::assert_invariants` and
  `E0063` at every construction site, which after `changes-from-cli` includes
  `from_files`', `from_cli`'s, and `merge`'s
- **AND** when mechanism 1 is then fully defeated in that copy — `Change` derives
  `Default` and every `Change` literal is filled from it, so **no `E0063` remains** — the
  build still fails with `E0027`, proving mechanism 2 catches what mechanism 1 misses
- **AND** when instead mechanism 2 is defeated by adding a `..` rest pattern to
  `assert_invariants`' pattern and no `Default` is added, so **no `E0027` remains**, the
  build still fails with `E0063`, proving mechanism 1 catches what mechanism 2 misses
- **AND** the assertions are on the specific error codes present **and absent**, not on
  "the build failed": a copy broken for an unrelated reason also fails, and would
  otherwise be read as evidence

#### Scenario: `Change` gaining a `Default` is caught by a source check

- **WHEN** the guarded source check for a `Default` implementation is run against a copy of
  `src/` in which, in turn, `Change` derives `Default` across a multi-line `#[derive(…)]`,
  `Change` carries a hand-written `impl std::default::Default for Change`, and
  `src/state.rs` — not `src/changes.rs` — carries an `impl Default for
  crate::changes::Origin`
- **THEN** the check exits non-zero in all three cases, because it searches every file
  under `src/` and matches a path-qualified `Default` as well as a bare one
- **AND** the same check exits non-zero when `src/changes.rs` does not exist, when fewer
  than eight `.rs` files are searched, and when the four types are not declared where it
  expects them, rather than passing on a missing or wrong file

#### Scenario: A rest pattern is caught even when a comment separates it from the comma

- **WHEN** the guarded source check for a `..` functional update or rest pattern is run
  against a copy of `src/changes.rs` carrying, in turn, `Change { name, ..other }` on one
  line; the same spread across lines; `let Change { name, .. }`; a functional update with a
  `// line comment` between the comma and the `..`; and one with a `/* block comment */`
  between them
- **THEN** the check exits non-zero in all five cases. The last two are the ones a
  comment-naive scan misses, and they compile and are formatter-clean
- **AND** the check exits zero against the real `src/changes.rs`, which contains a
  `segment[..star]` slice index it must not fire on, so the green run is itself
  discriminating rather than merely permissive

#### Scenario: Every value a producer builds satisfies the shared invariants

- **WHEN** `from_files` produces a `ChangeSet` from a repository holding two active and
  three archived changes
- **THEN** `assert_invariants` passes for all five values
- **AND** the same function is the one `changes-from-cli` calls for every value
  `from_cli` builds and every value `merge` returns, with no second copy of the invariants
  written for either
