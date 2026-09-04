<!-- No outer-loop acceptance group: design.md → Test Strategy records why. This change's
     outermost surface is a library API — `src/main.rs` is untouched and no caller consumes
     a `Tasks` or a `Progress` until `changes-from-files` — so an acceptance test would drive
     an entry point that ships to nobody. The unit tier reaches every collaborator these
     functions have: a `&str`, and a real filesystem for `read`.

     No group carries a `parallel-after` marker. Groups 1 through 5 all edit `src/tasks.rs`;
     the tasks instruction permits parallel dispatch only for groups touching different file
     trees with no shared mutable state, and one file is shared mutable state.

     All nine concentration points from `openspec/config.yaml` are accounted for, and the
     three that were padded or mis-cited in the first draft are named here as such rather
     than left to look satisfied.
     Scheduled: **nothing spawns outside `cli`** — 6.2 for the module source (guarded, and
     with the alternation typed as bare `|`; see design.md → Test Strategy for the version
     of that command that could not fail) and 6.3 for the build graph, which is the half a
     source grep cannot see. **Nothing writes inside `openspec/`** — 5.3, asserting
     directory entries, bytes, and modification times rather than a file listing;
     `testutil::snapshot` already records directories, so no extension is needed.
     **An absent case for every external dependency** — 5.1's absent file is the real one
     (the filesystem is the only external dependency this change has); 6.1 and 6.3 confirm
     no dependency was added at all. 1.6's empty document is an absent *input*, not an
     absent dependency, and is not counted here. **`from_files` and `from_cli` produce the
     same type** — 1.7's final test is the actual agreement site: it constructs a
     `Progress` from the pair a CLI response supplies and compares it with `count`'s
     result. 1.10 is the gate that keeps the type free of derived state, and group 4 is a
     different property (the two *file-path* entry points agreeing with each other), not
     this one. **Coverage counts the whole crate** — 9.5 holds the floor; 5.5 keeps the
     filesystem edge to a shape a test can reach; and 9.1 checks that `src/main.rs` and
     `src/lib.rs` gained no logic beyond the one `pub mod` line, since that is where
     untestable residue would accumulate. Not applicable, deliberately: no view is added,
     so "views perform no I/O" and "view tests at 60 and 120 columns" have nothing to bind
     to; no attribution path exists, so "attribution must not guess" is
     `agent-attribution`'s; no agent is named, so the 32-character Herdr cap does not apply.

     Contract gate: three fire. `Progress` is the type both producers of `Change` must
     build, so 1.10 gates it at the moment it is frozen rather than after the rest of the
     module is piled on top. 3.9 gates `Tasks`, `Group`, `Item`, and `Heading` — that is
     `tasks-tab`'s entire surface, frozen in group 3, and waiting for 5.6 would gate it two
     groups after the fact. 5.6 gates the complete public surface once `read` closes it.
     This change consumes no published contract of another module — the path arrives as an
     argument — and alters no plugin manifest key, no `config.toml` key, and no keybinding,
     so the project rule's manifest/config gate has nothing to bind to.

     Persistence gate: none applies, and design.md → Persistence and Rollout says so per
     item. Nothing is stored, migrated, indexed, or cached. 5.7 is the task that records why
     no memoisation is introduced, and it runs after 5.5's GREEN so that a `OnceLock`
     introduced by the very code it inspects is something it can actually see.

     Labels are honest about which tasks can fail. Group 4's checks and 5.3's containment
     test are properties over code an earlier task in the same or a previous group has
     already written; they carry `CHECK` and `CHARACTERIZE` rather than `RED`, because the
     schema requires that label for evidence tasks that cannot honestly produce a red
     result, and dressing them as RED is exactly the "verification that cannot fail"
     failure mode this repository has already shipped twice.

     None of the command checks in group 6 may be committed as a Rust test. A test that reads
     `Cargo.toml` and asserts two dependencies proves only that the string was typed twice;
     the command that consumes the manifest is the check. -->

## 1. `Progress` and the flat line rule
<!-- kind: behavior -->

Covers the `task-checkboxes` scenarios "The four canonical shapes are task lines",
"Whitespace around the bullet and the box is optional", "A `+` bullet and an ordered
marker are not task lines", "A malformed box is not a task line", "A bullet with no
checkbox and prose that mentions one are not task lines", "Both letter cases count as
done", "A space, a tab, and a non-breaking space are all empty boxes", "A checkbox inside
a fenced code block counts", "A checkbox inside an HTML comment counts", "A block-quoted
checkbox does not count, because the quote marker is not a bullet", "A CRLF document
counts and reads the same as its LF twin" (the `count` half), "The last line counts
without a trailing newline", "An empty document has no tasks and is not an error" (the
`count` half), "A fully checked file is complete", "A file with no tasks is not
complete", "Progress values from several files sum", and "A file-derived count and a
CLI-shaped count compare equal".

Three of those scenarios ("The four canonical shapes are task lines", "Whitespace around
the bullet and the box is optional", "A space, a tab, and a non-breaking space are all
empty boxes") carry a `parse` clause as well as a `count` clause. `parse` does not exist
until group 3, so this group takes only the `count` half of each and group 3 picks up the
other half explicitly — see 3.2.

This group implements the whitespace test as `char::is_whitespace`, which is the
straightforward thing that passes its own tests: every code point any group-1 scenario
names is whitespace to it. Group 2 discovers that the CLI's alphabet differs at two code
points and corrects it. That is an ordinary incremental step, not a deliberate defect
planted to manufacture a red — see 2.1 for what to do if the two turn out to agree.

- [ ] 1.1 CHECK: Before the first commit of this change, capture the two baselines group 6
      compares against, because both of its checks are green by construction without them —
      this project commits after every task group, so by group 6 any `Cargo.toml` edit is
      already in `HEAD` and `git diff` against the working tree sees nothing. Record
      `git rev-parse HEAD` as the base SHA and save `cargo tree --edges normal > <baseline>`
      to a path outside the repository. Write both values into this task when you run it,
      so group 6 does not have to guess them
- [ ] 1.2 Create `src/tasks.rs` with a module doc comment naming the module's job in one
      sentence and pointing at `openspec/changes/task-parsing/design.md`, matching the
      opening of `src/schema.rs` and `src/resolve.rs`; register it with one
      `pub mod tasks;` line in `src/lib.rs` beside the existing four. Add nothing else to
      `src/lib.rs` and nothing at all to `src/main.rs` — logic in either is coverage's blind
      spot, which is the reason the crate keeps them thin. Confirm `cargo build` succeeds
      with an empty module before writing a test, so a later failure is never "the module is
      not registered"
- [ ] 1.3 RED: Write failing unit tests in `src/tasks.rs` for `count`, one per spec
      scenario listed above and named after it. Build every input as a Rust string
      literal in the test — no fixture file, no filesystem, no `include_str!` (group 4
      owns the corpus). Where a scenario names a specific code point, write it as an
      explicit escape (`\u{a0}`, `\u{feff}`, `\u{85}`) rather than pasting the character,
      so an editor normalising the file cannot silently disarm the test
- [ ] 1.4 RED: Make the negative tests discriminating rather than merely green-able, each
      against the specific wrong implementation it exists to reject. "A `+` bullet and an
      ordered marker are not task lines" rejects a bullet set widened to CommonMark's
      `[-+*]` and to ordered items — the CLI's pattern is `[-*]` and a wider one reports a
      total the CLI does not. "A malformed box is not a task line" must include `- [] empty`
      (a box of zero characters) and `- [xx] two` (a box of two), because a `\[.*?\]`-shaped
      rule accepts both. "A bullet with no checkbox and prose that mentions one are not task
      lines" must include `text - [x] mid-line`, which a rule anchored anywhere but the start
      of the line accepts, and `-- [x] two dashes`, which a rule consuming a *run* of bullet
      characters accepts. Each of these three tests must place a genuine task line in the
      same document and assert the exact total, so an implementation that matches nothing at
      all cannot pass them by returning zero
- [ ] 1.5 RED: Write the fence, comment, and block-quote tests in this same pass, and be
      honest in a comment about what they are — **guards pinning a deliberate non-feature**,
      not drivers of new behaviour. They are red now because `count` does not exist; once it
      does, the only implementation they go red against is one that grows fence, comment, or
      block-quote state. That is exactly the "improvement" a future reader is most likely to
      make, and design.md → Decisions 1 and 6 record why it would break agreement with the
      CLI. The block-quote test needs no rule of its own: `>` is not in the bullet alphabet,
      so a `> - [x] quoted` line already fails to match, and the test asserts that the
      agreement holds without a block-quote rule existing
- [ ] 1.6 RED: Write the boundary tests — an empty string, a document of blank lines and
      prose with no checkbox, a single task line with no trailing newline, and a document
      whose only checkbox is on the last line after a trailing newline. Assert
      `Progress { completed: 0, total: 0 }` for the first two and the exact non-zero pair
      for the others. These are the absent-input cases; a `split('\n')` that drops the final
      element passes the first two and fails the third
- [ ] 1.7 RED: Write the `Progress` tests — `is_complete` true at 3/3 and **false at 0/0**
      (the CLI distinguishes "No tasks" from "✓ Complete" and an implementation testing only
      `completed == total` reports every taskless change as done), `+` and `+=` summing
      `(1,3)` and `(2,2)` to `(3,5)`, and equality between a `count` result and a `Progress`
      built directly from the pair a CLI response would supply. That last one is the
      agreement site's test: it is what makes `Progress` a shape `changes-from-cli` can
      construct without conversion
- [ ] 1.8 Confirm every failure is the missing function or type rather than a broken test —
      run `cargo test --all-features tasks::` and check that the new test names appear and
      fail for the stated reason, not that the file failed to compile for an unrelated one
- [ ] 1.9 GREEN: Implement `Progress` (`Debug, Clone, Copy, PartialEq, Eq`, plus `Add` and
      `AddAssign`), `is_complete`, and `count(text: &str) -> Progress`. Split `text` on
      `'\n'`, strip **one** trailing `'\r'` from each line, and apply the line rule as a
      single left-to-right scan: skip leading whitespace, require exactly one `-` or `*`,
      skip whitespace, require `[`, take exactly one character, require `]`. Treat `x` and
      `X` as checked and every other admitted character as unchecked. Use
      `char::is_whitespace` for every whitespace test: it is the standard-library predicate
      and it satisfies every scenario this group owns. Do not reach ahead to group 2's
      alphabet — not in order to manufacture a failure there, but because a rule adopted
      before its own scenario exists is a rule nothing in the suite constrains. No fence
      state, no comment state, no block-quote state, no allocation per line
- [ ] 1.10 CHECK: Contract gate for `Progress`. Re-read design.md → Contracts and
      `SPEC.md` → Data layer → Dual-source model as they stand at implementation time, and
      confirm that the value this group froze is the one `changes-from-cli` can build from
      `completedTasks` and `totalTasks` with no conversion and no third state: two `usize`
      counts, no ratio, no formatted string, no "unknown" variant. If a sentinel value looks
      tempting for "this change has no tasks artifact", record here that it is
      `changes-from-files`' `Option<Progress>`, not a `Progress` variant
- [ ] 1.11 REFACTOR: If the scan has grown a duplicated "skip whitespace from index i"
      fragment, extract it into one named helper — group 2 rewrites exactly that predicate
      and a single call site is what makes group 2 a two-line change rather than a
      four-site edit. Otherwise state that no refactor was needed
- [ ] 1.12 Run the group tests — `cargo test --all-features` green, with no regressions to
      the existing `config`, `resolve`, `schema`, `state`, or invocation tests

## 2. The whitespace alphabet
<!-- kind: behavior -->

Covers "A byte-order mark before the first bullet does not hide the task", "A next-line
character before a bullet is not indentation", and "A next-line character inside the box
is not an empty box".

- [ ] 2.1 RED: Write the three failing tests named after those scenarios, then confirm
      each is red **against group 1's shipped implementation** rather than against a
      missing function — this group's whole content is that `char::is_whitespace` is the
      wrong predicate. The BOM test fails because `char::is_whitespace` rejects U+FEFF, so
      the first task of a BOM-prefixed document goes uncounted; the two U+0085 tests fail
      because `char::is_whitespace` accepts it and the CLI does not. If any of the three is
      green on arrival, group 1 already implemented this group's rule: **keep the correct
      code**, record here which test could not be made red and why, and treat it as a
      planning note rather than as a reason to regress working code into a failure. A test
      that passes because the behaviour is already right is a fact to record, not a defect
      to create
- [ ] 2.2 RED: Give the BOM test a second task line after the BOM-prefixed one and assert
      the exact pair `(completed: 1, total: 2)`. A test asserting only "the total is not
      zero" passes against an implementation that counts the second line and drops the
      first, which is precisely the bug
- [ ] 2.3 GREEN: Replace the whitespace predicate with one named helper carrying a doc
      comment stating the rule and its provenance in two lines: the set is the CLI's `\s` —
      Unicode `White_Space` plus U+FEFF, minus U+0085 — and both differences from
      `char::is_whitespace` are resolved in the CLI's favour because the counts must agree.
      Apply it at every whitespace test in the scan: leading indent, between bullet and
      box, inside the box, and after the box
- [ ] 2.4 REFACTOR: Confirm there is exactly one place in the module that decides what
      whitespace is, and that `trim` on the item text is either the same rule or explicitly
      documented as `str::trim`'s. `str::trim` uses `char::is_whitespace`, so a text ending
      in U+FEFF keeps it while a text ending in U+0085 loses it — record which way you went
      and why it cannot change a count, since trimming happens after the line has already
      been counted
- [ ] 2.5 Run the group tests — `cargo test --all-features` green, no regressions

## 3. Headings, groups, and items
<!-- kind: behavior -->

Covers the `task-groups` scenarios "Two headings yield two groups holding their own
items", "Items before the first heading form an unnamed leading group", "A file opening
with a heading has no empty leading group", "Indented, over-long, and unspaced hashes are
not headings", "A deeper heading closes the group above rather than nesting", "Repeated
and empty headings are all kept", "Prose between items is dropped and does not split a
group", "A nested sub-task is a sibling item carrying its indent", "Indent does not affect
membership of the preceding group", "A checked item's state survives grouping", "A closing
hash sequence is kept, not stripped", "A CRLF document counts and reads the same as its LF
twin" (the `parse` half), and "An empty document has no tasks and is not an error" (the
`parse` half). It also picks up the `parse` half of three `task-checkboxes` scenarios
group 1 could only take the `count` half of — "The four canonical shapes are task lines",
"Whitespace around the bullet and the box is optional", and "A space, a tab, and a
non-breaking space are all empty boxes" — plus "A numbering prefix and inline markup are
kept verbatim in the text", which is a `parse`-only scenario throughout.

- [ ] 3.1 RED: Write failing unit tests for `parse`, one per scenario above, named after
      it. Compare **whole values** with `assert_eq!` wherever the expected `Tasks` is small
      enough to write out — the types derive `PartialEq` for exactly this, as
      `resolve::BinResolution` and `schema::Schema` already do — rather than asserting a
      field at a time, which lets an extra fabricated group slip through unnoticed
- [ ] 3.2 RED: Write the four text-fidelity tests this group inherits, which are the only
      place `Item::text` is asserted against hostile input. "Whitespace around the bullet
      and the box is optional" asserts the texts are exactly `done` and `todo` for
      `-[x]done` and `   *   [ ]   todo   `; "A numbering prefix and inline markup are kept
      verbatim in the text" asserts both texts byte for byte, proving no numbering field is
      parsed out and no backtick or asterisk is resolved; "A space, a tab, and a
      non-breaking space are all empty boxes" asserts `checked == false` on each of the
      three parsed items rather than only on the aggregate count; "The four canonical shapes
      are task lines" asserts the four texts in order. Together with 2.4's decision about
      which trim applies, these are what stop the text field being whatever the
      implementation happened to produce
- [ ] 3.3 RED: Make the heading tests discriminating. "Indented, over-long, and unspaced
      hashes are not headings" must assert the group count is **exactly one** and that its
      heading is `None`, so an implementation trimming the line before counting hashes
      fabricates three groups and fails. "A file opening with a heading has no empty
      leading group" must assert the group count is exactly two **and** that no group in
      the result has `heading == None`, so an implementation that always emits a leading
      group fails; pair it with "Items before the first heading form an unnamed leading
      group", which fails the opposite implementation — one that never emits one. Neither
      test alone constrains the rule. "A closing hash sequence is kept, not stripped"
      asserts the heading text is exactly `Group ##`, rejecting a CommonMark-faithful
      implementation that strips it — which would also mangle a heading like `## C# ##`
- [ ] 3.4 RED: "Repeated and empty headings are all kept" uses two headings of identical
      text, one bare `##` with no text at all, and one trailing heading with no items, and
      asserts four groups in document order with the exact heading texts. It rejects three
      separate wrong implementations at once: one that deduplicates by heading text, one
      that treats a heading with no following item as absent, and one that drops a heading
      whose text is empty
- [ ] 3.5 RED: "A nested sub-task is a sibling item carrying its indent" asserts the
      indents `[0, 2, 1]` for a document whose third line is tab-indented, so an
      implementation expanding a tab to a column width fails, and asserts the group
      progress is `(1, 3)`, so an implementation treating a nested item as a child of the
      one above it — and therefore not a task — fails. "Indent does not affect membership
      of the preceding group" places an eight-space item between two headings and asserts
      it belongs to the first
- [ ] 3.6 Confirm the failures are the missing `parse` and its types rather than a broken
      expectation — check that `cargo test --all-features tasks::` names the new tests and
      that the group-1 and group-2 `count` tests are still green and untouched
- [ ] 3.7 GREEN: Implement `Heading`, `Item`, `Group`, and `Tasks`, each deriving
      `Debug, Clone, PartialEq, Eq`; `Group::progress` and `Tasks::progress` — and **no**
      `Tasks::items` iterator: design.md → Contracts records that it has no consumer, no
      scenario, and no test, and an uncovered `pub fn` under an 80% line floor is a cost
      with no buyer; then `parse(text: &str) -> Tasks`. One pass over the same split-and-
      strip-`\r` lines as `count`: a line starting at column zero with one to six `#`
      followed by a space or end of line closes the current group and opens a new one; a
      line matching the task rule appends an item carrying its checked state, its trimmed
      text, and the count of whitespace characters before its bullet; every other line is
      discarded. Emit the leading group only when it holds at least one item. Never sort,
      merge, deduplicate, or nest
- [ ] 3.8 REFACTOR: Extract the shared "is this line a task, and what are its parts"
      decision so `count` and `parse` call one function rather than carrying two copies of
      the rule. This is what design.md → Decisions 3 means by "two entry points, one rule":
      the split is in what each does with the result, not in the rule itself. Return the
      parts borrowed (`&str`) rather than owned, so `count` can use the same function
      without allocating a `String` per line and `parse` allocates only when it builds an
      `Item`. If that shape proves impossible, do not force it — say so here and let group
      4's equality test be what keeps the two in step
- [ ] 3.9 CHECK: Contract gate for the document model. This group freezes `Tasks`, `Group`,
      `Item`, and `Heading` — the whole surface `tasks-tab` consumes, per design.md →
      Contracts. Re-read that table and `SPEC.md` → User interface → Detail view as they
      stand now, and confirm the tab can be drawn from what this group emits: groups in
      document order with their heading level and text, items with checked state, text, and
      indent, and a per-group and whole-file progress for the bar. Name anything the view
      would need and cannot get, and fix it here rather than after 5.6 gates the module two
      groups later
- [ ] 3.10 Run the group tests — `cargo test --all-features` green, no regressions

## 4. Count and parse agree, on a real corpus
<!-- kind: refactor -->

Covers "A real change's task file counts the same both ways" and "Text with no heading
still agrees".

This group is classified `refactor`, not `behavior`, and the reason is the honesty rule
rather than a technicality. Both scenarios are **properties over code groups 1–3 already
wrote**, so a correct implementation makes them green the moment they compile; a group
claiming RED → GREEN here would be manufacturing a red result that cannot exist. The
refactor lifecycle is the one that fits: characterize the existing behaviour, change
structure (or, contingently, fix a genuine disagreement the characterization exposes),
verify the characterization still holds. What these tests genuinely protect against is
future drift between the two entry points, and — through 4.3's absolute counts — an
implementation that agrees with itself by counting nothing.

- [ ] 4.1 CHARACTERIZE: Copy one real, complete `tasks.md` from this repository's `openspec/changes/archive/`
      into `tests/fixtures/tasks/archived-change.md`, **by copying the file**, not by
      hand-transcribing it. Add the four smaller documents beside it —
      `crlf.md` (CRLF endings throughout), `fenced-and-commented.md` (checkboxes inside a
      code fence and inside an HTML comment, plus real ones), `no-heading.md` (five task
      lines, **two of them checked**, and no heading), and `empty.md` (zero bytes). The
      mixed state of `no-heading.md` is load-bearing: every archived `tasks.md` in this
      repository is 100% complete, so without one mixed fixture a hardcoded `checked = true`
      satisfies the entire corpus. Record in a one-line comment at the top of the copied
      file which change it came from and that it is a frozen copy, so a reader does not try
      to keep it in sync
- [ ] 4.2 CHARACTERIZE: Obtain the copied document's expected `completed` and `total` from
      an **oracle outside this crate**, not from the code under test: run the CLI's own
      `TASK_LINE_PATTERN` over the same bytes in `node`, or read the pair out of
      `openspec list --json` for the change it came from. Record the pair, the method, and
      the observed package version in this task. An expected value derived by running the
      Rust implementation is not evidence of anything
- [ ] 4.3 CHARACTERIZE: Write the corpus test: `include_str!` each of the five fixtures —
      never a run-time path, so no test resolves a location in the live repository — and
      assert `parse(text).progress() == count(text)` for every one. In the same test,
      assert the copied archived document's **absolute** `completed` and `total` against
      4.2's oracle pair. Without that second assertion the test passes against an
      implementation where both sides return zero, which is the one failure mode an
      equality-only property test cannot see
- [ ] 4.4 CHARACTERIZE: Write "Text with no heading still agrees" against `no-heading.md`,
      asserting `Progress { completed: 2, total: 5 }` from both entry points — the
      `completed` half as well as the `total` — and that `parse` returns exactly one group
      whose heading is `None`
- [ ] 4.5 CHANGE (contingency, fires only if 4.3 or 4.4 is red): the two entry points
      already disagree, which is a real defect rather than a missing feature. Reconcile
      whichever one is wrong and add a scenario-shaped unit test for the specific line that
      differed, so the corpus test is not the only thing standing between the two rules. Do
      **not** commit a deliberately broken implementation in order to watch a test fail
- [ ] 4.6 REFACTOR: If 4.5 fired and produced a third copy of the line rule, collapse it
      into the one from 3.8; otherwise state that no refactor was needed
- [ ] 4.7 VERIFY: Run the group tests — `cargo test --all-features` green, no regressions —
      and state plainly which outcome 4.3 and 4.4 had. Green on arrival is the expected
      result for a correct implementation and is not a problem; a *silent* green is, so say
      it out loud either way

## 5. The filesystem edge, and what it must not touch
<!-- kind: behavior -->

Covers "An absent file is zero tasks and no problem", "A directory where a file was
expected is one named problem", "A file of invalid UTF-8 is one named problem, not a parse
result", "A readable file is parsed exactly as its text would be", and "A task tree is
byte-identical after reading".

- [ ] 5.1 RED: Write the four `read` tests, named after their scenarios, building every
      fixture under a `crate::testutil::ScratchDir` — never a path in the real repository.
      The absent-file test asserts no groups, `Progress { completed: 0, total: 0 }`, **and**
      `problems.is_empty()`; the directory and invalid-UTF-8 tests each assert no groups and
      **exactly one** problem whose text contains the path. Those two directions together
      are what make `problems` load-bearing: an implementation recording a problem for every
      failed read passes the second pair alone, and one that never records passes the first
      alone
- [ ] 5.2 RED: Use a **directory** as the unreadable case, not a file with mode `0000`. A
      read of a directory fails with `EISDIR` on both macOS and Linux and for the root user
      as well, while a mode-`0000` file is readable by root and would pass silently in a
      container that runs as one. It is also the realistic shape: a schema whose tasks
      artifact `generates` a directory glob gives `changes-from-files` a directory path to
      hand over. Write the invalid-UTF-8 fixture as explicit bytes (`0xFF 0xFE`) with
      `std::fs::write`, not as a string. Have "A readable file is parsed exactly as its text
      would be" assert `read(path) == parse(text)` on the whole value, problems included, so
      the edge cannot quietly differ from the pure path
- [ ] 5.3 CHARACTERIZE: Write the containment test in the same pass as 5.1 and 5.2, before
      any `read` exists — build a fixture tree holding `openspec/changes/x/tasks.md`, an
      **empty** `openspec/specs/` directory, and `README.md`; take a `testutil::snapshot`;
      call `read` on the `tasks.md` path, on the `openspec/specs` directory path, and on a
      path that does not exist; take a second snapshot and assert equality; then assert the
      non-existent path and each of its intermediate directories are still absent. Assert
      bytes and modification times, not only the listing — a rewrite of identical length
      passes a listing-only check — and read modification times through `std::fs::Metadata`,
      never by shelling out to `stat`, whose flags differ between BSD and GNU. Label it
      `CHARACTERIZE`, not `RED`, and be honest about why: before `read` exists it fails to
      compile, which is not the same as failing for the behaviour, and once `read` exists a
      correct implementation makes it green immediately. What it genuinely goes red against
      is an implementation that creates a parent directory, opens the target in a mode that
      updates its modification time, or writes a lock file beside it
- [ ] 5.4 Confirm 5.1's failures are the missing `read` rather than a broken fixture —
      assert inside at least one test that the fixture file exists and holds the expected
      bytes before the call under test, and check that `cargo test --all-features tasks::`
      names the new tests rather than failing to compile for an unrelated reason
- [ ] 5.5 GREEN: Implement `read(path: &Path) -> Tasks`. Read the file as UTF-8 text; on
      success return `parse` of it. Map `ErrorKind::NotFound` to an empty `Tasks` with **no**
      problem — the CLI treats a missing tasks file as zero tasks, and a change whose tasks
      artifact is not written yet is an ordinary state. Map every other error, including an
      invalid-UTF-8 decode failure, to an empty `Tasks` carrying exactly one problem naming
      the path. Do **not** decode lossily to keep the CLI's count: design.md → Decisions 8
      records that this is a knowing divergence, that the CLI reports a count for such a
      file and this module reports none plus a problem, and why that is the better failure.
      Never return a `Result`, never panic, never `unwrap`
- [ ] 5.6 CHECK: Contract gate for the module's complete public surface. `read` closes it,
      and design.md → Contracts names five consumers — `changes-from-files`,
      `changes-from-cli`, `tasks-tab`, `live-refresh`, and `degraded-states`. Re-read that
      table against the code as it now stands and confirm each row is still satisfiable:
      every type a consumer names is `pub`, every field it reads is `pub`, nothing returns a
      `Result` the caller must handle, and `Tasks::problems` is the only channel by which a
      degradation is reported. Run `cargo doc --no-deps` and read the generated `tasks`
      page as a consumer would, to catch a type that is public but whose field is not
- [ ] 5.7 CHECK: Persistence gate — confirm and record that none of migration, backfill,
      seeding, cache invalidation, index rebuild, authorization, observability, or
      deployment applies, matching design.md → Persistence and Rollout item by item. In
      particular confirm no memoisation was introduced: `read` recomputes on every call
      because `live-refresh` owns invalidation and a cache here would have to be invalidated
      by a watcher this change cannot see. If a `OnceLock` or a lazily-initialised static
      appeared during groups 1–5, this is where it is caught
- [ ] 5.8 REFACTOR: If `read` has grown a nested match on `ErrorKind`, flatten it to the
      one-line "NotFound is silent, everything else is one problem" shape the doc comment
      claims; otherwise state that no refactor was needed
- [ ] 5.9 Run the group tests — `cargo test --all-features` green, no regressions

## 6. No-spawn and dependency confirmation
<!-- kind: operational -->

Covers "The module names no process API", and the three cross-cutting rows of design.md →
Test Strategy. None of these is a standing gate: `make check` is `fmt-check lint test
coverage` and CI runs exactly those, so each runs once, here. Be plain about what pins
things afterwards, because the first draft of this paragraph was wrong: the committed
`Cargo.lock` pins the dependency set, and **nothing** pins the no-spawn rule between
changes — no tree-wide scan is wired into `make check` or CI, and `repo-resolution`'s
equivalent scan was likewise a one-off run during that change. Making one standing would
change the `quality-gates` capability and is deliberately not this change's work.

- [ ] 6.1 CHECK: `git diff --exit-code "$BASE"..HEAD -- Cargo.toml Cargo.lock` — exits 0,
      proving this change added no dependency, direct or transitive, where `$BASE` is the
      SHA task 1.1 captured. The base SHA is not optional: `git diff --exit-code` alone
      compares the working tree to the index, and this project commits after every task
      group, so by the time this runs a `Cargo.toml` edit made in group 1 is already in
      `HEAD` and the bare form passes green over the very change it exists to catch. This is
      the check, not a Rust test asserting that two dependencies are listed: such a test
      proves only that the same string was typed twice, goes red when the value changes for
      a good reason, and stays green when the value is wrong. If it fails, the design
      decision to add no dependency has been broken and design.md → Decisions 2 must be
      reopened rather than amended after the fact
- [ ] 6.2 CHECK: Scan the new module for any process API —
      `test -f src/tasks.rs && ! grep -nE 'std::process|Command|spawn|\boutput\(|\bstatus\(' src/tasks.rs`
      Run it as a check whose exit status decides the outcome, not as a report you read, and
      keep both halves: the `test -f` guard, because `grep` exits 2 on a missing file and a
      bare `!` turns that into a pass; and the alternation typed as **bare `|` characters**,
      because `grep -E 'a\|b'` matches the literal string `a|b`, matches nothing in any real
      source file, and yields a check that can never fail. Confirm afterwards that the
      crate's only process reference is still `crate::pid`'s single documented call in
      `src/lib.rs`
- [ ] 6.3 CHECK: `diff <(cargo tree --edges normal) "$TREE_BASELINE"` — exits 0 against the
      baseline task 1.1 captured, confirming the normal build graph still contains only
      `toml`, `yaml-rust2`, and their existing transitive dependencies. A `cargo tree` run
      with nothing to compare against is a printed report, not a check. This is the half a
      manifest diff cannot see: a dependency arriving through a feature flag adds no line to
      `Cargo.toml`
- [ ] 6.4 CHECK: Re-read `dist/utils/task-progress.js` in the installed
      `@fission-ai/openspec` package **at implementation time**, not from this plan, and
      compare `TASK_LINE_PATTERN` character by character against the rule groups 1 and 2
      implemented. Record the package version observed. If the pattern has changed since
      this design was written, stop and reopen design.md → Decisions 1 rather than
      implementing to a rule the installed CLI no longer applies — the whole point of the
      copy is agreement with the program that is actually installed. Locate the package
      without hardcoding an nvm version, for example
      `dirname "$(readlink -f "$(command -v openspec)")"`
- [ ] 6.5 VERIFY: Confirm each of 6.1 through 6.4 was run as a command with a pass/fail
      outcome and record what each returned. A check reported as "looks fine" is a check
      that cannot fail

## 7. Change Review
<!-- kind: operational -->

- [ ] 7.1 CHECK: Dispatch an independent reviewer — not a fork of the implementing session
      — against proposal.md, both spec files, design.md, and tasks.md plus the diff. Point
      it first at the concentration points that bite here: that group 2's three tests were
      genuinely red against group 1's shipped implementation rather than green on arrival;
      that the fence, comment, and block-quote guards are labelled as guards and assert an
      exact non-zero total rather than merely "not zero"; that the corpus test carries the
      absolute-count assertion without which an all-zero implementation passes it; that the
      three negative line-rule tests each place a real task line in the same document; that
      `is_complete` is asserted false at 0/0 and not only true at n/n; that the absent-file
      and unreadable-file tests assert `problems` in **both** directions; that the
      unreadable case is a directory rather than a mode-`0000` file; that the containment
      test asserts bytes and modification times rather than a listing, and includes the
      non-existent-path run; that `count` and `parse` share one line rule or are pinned
      equal by group 4; and that no test resolves a path inside the live repository. Three
      more, all from this change's own planning review, because each is a check that was
      green by construction in the first draft: that 6.1's manifest diff really names task
      1.1's base SHA rather than comparing the working tree to the index; that 6.2's scan
      keeps both the `test -f` guard and bare `|` alternation; and that 4.3's absolute
      counts came from an oracle outside this crate rather than from the implementation.
      Also confirm `Tasks::items` was not reintroduced. Give
      the reviewer the **planned group-8 document rewrites**, quoted from this file
      alongside the current text of `SPEC.md` and `AGENTS.md`: those edits land after this
      review and are otherwise never seen by fresh eyes, which is exactly the defect
      `repo-resolution`'s planning review caught. Give the reviewer a scratchpad path and
      require it to append findings as it goes rather than only in a final message —
      `HANDOFF.md` records two review rounds lost to `529 Overloaded`
- [ ] 7.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
      one-line reason, note SUGGESTIONs, and re-run every affected check
- [ ] 7.3 VERIFY: Confirm no blocking or unowned finding remains, and that every artifact a
      repair touched was updated in place rather than annotated afterwards

## 8. Documentation
<!-- kind: operational -->

- [ ] 8.1 CHECK: Re-read, at implementation time rather than from memory of this plan, the
      exact current wording of every passage rewritten below: `SPEC.md` → Data layer →
      Dual-source model, → Degraded states (the table), → Testing and quality gates →
      Unit-tested modules (the `tasks::parse` entry); `openspec/IMPLEMENTATION-ORDER.md` →
      the Phase 2 `task-parsing` row; and `AGENTS.md` → Current repo state. Record what each
      says now, so a correction another change already made is neither duplicated nor
      reverted
- [ ] 8.2 CHANGE — add to `SPEC.md`: Data layer → Dual-source model, three or four lines
      (audience: `changes-from-files`, `changes-from-cli`, and `tasks-tab`, all planned
      from this section). The section says files and the CLI both produce progress and says
      nothing about the two producing the *same* number, which is the property the whole
      dual-source model rests on. State the counting rule — a `-` or `*` bullet at any
      indent carrying a one-character `[ ]`/`[x]`/`[X]` box — that it is the OpenSpec CLI's
      own rule rather than this crate's, that a checkbox inside a fence or an HTML comment
      is counted deliberately so the two agree, and where the rule can be re-verified
      (`dist/utils/task-progress.js` in the installed package). Name the package version
      observed in 6.4, so a future reader can tell whether the agreement is still current
- [ ] 8.3 CHANGE — rewrite in `SPEC.md`: Testing and quality gates → Unit-tested modules,
      the `tasks::parse` entry (audience: future changes choosing a tier). It reads
      "`tasks::parse` — markdown checkboxes to grouped items and counts", which names one
      of the three entry points this change ships. Widen it in place to name `count` and
      `read` beside `parse` and to say the counting rule is the CLI's — this is a rewrite
      of the existing line, not a second bullet beside it
- [ ] 8.4 CHANGE — add to `SPEC.md`: Degraded states, one table row (audience:
      `degraded-states`, which is planned from this table and is told to confirm the table
      rather than to discover behaviour). The table's nearest row covers a *missing*
      artifact file, which this change deliberately treats as zero tasks and not a problem.
      The state it has no row for is a tasks file that exists and cannot be read — a
      directory where a file was expected, a permission error, an I/O error, or bytes that
      are not valid UTF-8 — where the change reports zero tasks, names the file in
      `Tasks::problems`, and lets the CLI's count correct the pane when it arrives. Say in
      the same row that this is the one place the file path knowingly disagrees with
      `openspec list --json`, which decodes lossily and still reports a count.
      `plugin-config` and `repo-resolution` each added their own row for the same reason:
      Phase 6 must be able to confirm the table, not extend it
- [ ] 8.5 CHANGE — rewrite in `openspec/IMPLEMENTATION-ORDER.md`: the Phase 2
      `task-parsing` row (audience: whoever reads the roadmap next, and the archive step,
      which is told to confirm the row still describes what was built). Two corrections.
      Its **Spec refs** cell reads "User interface → Detail view", which is where the parsed
      result is eventually *rendered* by `tasks-tab` in Phase 4; the contract this change
      actually implements — that file counts and CLI counts must agree — lives in Data layer
      → Dual-source model, and a planner reading only the cited section would not find it.
      Cite both. Its **Depends on** cell reads `repo-foundation`, which is true of the
      module but understates the test surface: the containment and scratch-tree tests use
      `testutil::ScratchDir` and `testutil::snapshot`, which `plugin-config` introduced and
      `repo-resolution` extended to record directories. Both have landed, so this is a note
      in the row rather than a new edge in the Mermaid graph — and record the decision not
      to add the edge, so a later reader does not read its absence as an oversight
- [ ] 8.6 CHANGE — rewrite in `AGENTS.md`: Current repo state (audience: a fresh session's
      first paragraph). Two clauses become false: the list of landed changes omits
      `task-parsing`, and "`task-parsing` is next; `changes-from-files` still needs it" is
      now wrong in both halves. Replace with what is true after this change — the crate
      parses a task file into groups, items, and counts that agree with the CLI's, and
      `changes-from-files` is next with all three of its dependencies landed. Check the
      Phase 2 table before writing the name rather than assuming roadmap order equals
      dependency order. In the same pass, decide whether the Important-files list should
      name `src/tasks.rs`; it names no other source file individually, so the answer is
      probably no — record which way you went
- [ ] 8.7 CHANGE: Make, or record the decision not to make, one `AGENTS.md` → Architecture
      rules edit. The candidate durable rule is the one a future change would otherwise get
      wrong: *checkbox counting follows the OpenSpec CLI's rule exactly, fenced and
      commented checkboxes included, because the file path and the CLI path must report the
      same progress*. That is a constraint no code comment in another module carries and
      that `changes-from-cli` and `tasks-tab` both sit on. If it goes in, it goes in as at
      most two lines under the existing invariants — `AGENTS.md` is a fixed attention
      budget, not an archive. Prefer editing a neighbouring rule over adding a third bullet
- [ ] 8.8 VERIFY: Net size — this group adds roughly fourteen lines across three documents
      (`SPEC.md`, `openspec/IMPLEMENTATION-ORDER.md`, `AGENTS.md`) and rewrites three
      entries in place. Because that is more than ten lines in `SPEC.md` alone, say what it
      replaces there: the Dual-source model section gains the counting rule it never stated,
      the Degraded states table gains the unreadable-tasks-file row it was missing, and the
      Unit-tested-modules entry is rewritten rather than duplicated. Confirm no entry it
      touched now says something false: that `tasks::parse` is the module's only function,
      that `task-parsing` is still next, that `changes-from-files` is still waiting on it,
      or that the roadmap row's only spec reference is the detail view. Then read each
      rewritten passage against its surrounding text as a reader who has not seen this
      change would — group 7's reviewer never saw these edits, because they land after it

## 9. Lint & Verify
<!-- kind: operational -->

- [ ] 9.1 CHECK: Inspect the intended verification commands and affected tiers — the unit
      tier (`cargo test --all-features tasks::` for one group at a time, plus the whole
      suite for regressions; a cargo test-name filter matching nothing exits 0, so the
      filtered form is a convenience and 9.4 is the gate), the four command checks in group
      6, and coverage, which is affected: this change adds a module and a corpus fixture.
      Record explicitly that none of group 6's checks is a standing gate, and that wiring
      them into `make check` would change the `quality-gates` capability and is deliberately
      not this change's work. In the same pass confirm the change added no untestable
      residue: `git diff "$BASE"..HEAD -- src/lib.rs src/main.rs` shows one `pub mod tasks;`
      line and nothing else. Logic in either file is what coverage cannot reach, which is
      the reason the crate keeps them thin
- [ ] 9.2 VERIFY: `cargo fmt --all -- --check` — clean
- [ ] 9.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.
      Probe with `cargo clippy --version`, not `command -v cargo-clippy`: rustup installs
      that shim unconditionally, so the shim resolves even when the component is absent
- [ ] 9.4 VERIFY: `cargo test --all-features` — green, including the unchanged `config`,
      `resolve`, `schema`, and `state` unit tests and the `tests/cli.rs` and
      `tests/ci_workflow.rs` integration tests, none of which this change touches. Rust has
      no separate type-check step; `cargo test` and `cargo clippy` cover it
- [ ] 9.5 VERIFY: `cargo llvm-cov --fail-under-lines 80` — at or above the floor, and record
      the resulting percentage against the pre-change baseline. If it falls short, add
      tests; never lower the threshold, never add an exclusion flag, and never mark a
      function `#[cfg(not(test))]` to hide it from the count
- [ ] 9.6 VERIFY: `make check` — the single composite gate, exit 0. If it fails, name the
      failing sub-command rather than reporting a summary
- [ ] 9.7 VERIFY: `openspec validate task-parsing --strict` reports the change valid, with
      nvm ahead on `PATH`, since `openspec` is not on the `PATH` a non-login shell inherits
      here. Resolve the version rather than hardcoding it, so an nvm upgrade does not
      silently make this task unrunnable — for example
      `export PATH="$(dirname "$(ls -d "$HOME"/.nvm/versions/node/*/bin/openspec | tail -1)"):$PATH"`,
      then confirm `openspec list` works before relying on it
