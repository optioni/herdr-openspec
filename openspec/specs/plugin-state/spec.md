# plugin-state Specification

## Purpose
TBD - created by archiving change plugin-config. Update Purpose after archive.

## Requirements

### Requirement: The state directory is resolved from the process environment

The plugin SHALL determine its state directory by reading the process environment, and
SHALL NOT spawn any process to obtain it. `HERDR_PLUGIN_STATE_DIR`, which Herdr sets on
the plugin processes it starts — pane processes and action processes alike — is
authoritative when it is set to a value that is neither empty nor whitespace-only. When
it is unset, empty, or whitespace-only the directory SHALL fall back to
`$XDG_STATE_HOME/herdr/plugins/herdr-openspec` when `XDG_STATE_HOME` is likewise set to
a non-blank value, and otherwise to `$HOME/.local/state/herdr/plugins/herdr-openspec` —
the path Herdr itself supplies. When none of the three variables is available there
SHALL be no state directory, and every read SHALL yield an empty mapping while every
write that would otherwise create a file SHALL report that it could not be performed.

State and configuration are separate directories and SHALL NOT be conflated: the
mapping file is written, so it must not live in a directory the user hand-edits.

#### Scenario: Herdr supplies the state directory

- **WHEN** the directory is resolved with `HERDR_PLUGIN_STATE_DIR` set to
  `/tmp/state-fixture`, `XDG_STATE_HOME` set to `/xdg`, and `HOME` set to
  `/home/someone`
- **THEN** the resolved directory is `/tmp/state-fixture`
- **AND** it differs from the configuration directory resolved from the same lookup

#### Scenario: `XDG_STATE_HOME` is honoured before `HOME`

- **WHEN** the directory is resolved with `HERDR_PLUGIN_STATE_DIR` absent,
  `XDG_STATE_HOME` set to `/xdg`, and `HOME` set to `/home/someone`
- **THEN** the resolved directory is `/xdg/herdr/plugins/herdr-openspec`

#### Scenario: Run outside a Herdr pane with no XDG setting

- **WHEN** the directory is resolved with `HERDR_PLUGIN_STATE_DIR` and `XDG_STATE_HOME`
  absent and `HOME` set to `/home/someone`
- **THEN** the resolved directory is
  `/home/someone/.local/state/herdr/plugins/herdr-openspec`
- **AND** an `XDG_STATE_HOME` set to the empty string, and one set to `"   "`, each
  produce the same result

#### Scenario: No directory can be resolved at all

- **WHEN** the directory is resolved with all three variables absent
- **THEN** no directory is resolved
- **AND** reading the mapping yields an empty mapping with no problem reported
- **AND** recording a mapping whose derived name differs from the change name — for
  example agent `c-2fa-support` for change `2fa-support` — returns an error and creates
  nothing
- **AND** recording where the agent name equals the change name returns success without
  creating anything, because the "nothing to record" test runs before the directory is
  needed

### Requirement: A change name is converted to a Herdr-legal agent name

Herdr requires an agent name matching `[a-z][a-z0-9_-]{0,31}`. The plugin SHALL convert
any change name into such a name by a pure, total, deterministic function of the change
name alone, in this order:

1. ASCII-lowercase the name.
2. Replace every character outside `[a-z0-9_-]` with `-`.
3. Collapse runs of `-` to a single `-`, then trim leading and trailing `-` and `_`.
4. If the result is empty, use `change`.
5. If the first character is not in `[a-z]`, prefix `c-`.
6. If the result is longer than 32 characters, take its first 27 characters, trim any
   trailing `-` or `_`, and append `-` followed by a four-digit suffix: the 32-bit
   FNV-1a hash of the **original** change name, reduced modulo 36⁴, rendered in
   lowercase base-36 and left-padded with `0` to exactly four digits. (A `u32` needs
   seven base-36 digits, so the reduction is what makes the suffix four digits; padding
   applies to the small remainders that would otherwise render shorter.)

Step 6 makes a truncated name depend on the whole change name, so the same change always
produces the same agent name across processes and machines, and two changes sharing a
27-character prefix are overwhelmingly unlikely to collide. A collision remains
possible — four base-36 digits is 1,679,616 values, and step 4 maps every unusable name
to `change` — and its handling is fixed by the recording requirement below. The function
SHALL NOT consult the recorded mapping, the filesystem, or Herdr's live agent list;
uniqueness among *live* agents is Herdr's check at `agent start` and belongs to
`agent-launch`.

#### Scenario: A short kebab-case change name is unchanged

- **WHEN** the agent name for the change `add-token-refresh` is derived
- **THEN** it is `add-token-refresh`
- **AND** it matches `^[a-z][a-z0-9_-]{0,31}$`

#### Scenario: A name of exactly 32 characters is not truncated

- **WHEN** the agent name for `add-really-long-change-name-that`, which is exactly 32
  characters, is derived
- **THEN** it is `add-really-long-change-name-that`, unchanged
- **AND** the agent name for `add-really-long-change-name-thatx`, one character longer,
  is `add-really-long-change-name-mmky` — at most 32 characters, different from the
  first, and carrying the four-digit suffix

#### Scenario: A long change name is truncated with a suffix from the whole name

- **WHEN** the agent names for `add-really-long-change-name-that-overflows-alpha` and
  `add-really-long-change-name-that-overflows-beta` are derived
- **THEN** they are `add-really-long-change-name-8jqt` and
  `add-really-long-change-name-alft`
- **AND** both are at most 32 characters and match `^[a-z][a-z0-9_-]{0,31}$`
- **AND** they differ from each other, even though their first 27 characters are equal
- **AND** deriving either one a second time yields the identical string

#### Scenario: A truncated name may be shorter than 32 characters

- **WHEN** the agent name for `abcdefghijklmnopqrstuvwxyz-abcdefg`, 34 characters whose
  27th character is `-`, is derived
- **THEN** it is `abcdefghijklmnopqrstuvwxyz-lhun`, 31 characters, because step 6 trims
  the trailing separator from the 27-character prefix before appending the suffix
- **AND** the rule is "at most 32", not "exactly 32"

#### Scenario: Illegal characters and casing are normalised

- **WHEN** the agent name for `Add Token Refresh (v2)!` is derived
- **THEN** it is `add-token-refresh-v2`
- **AND** it does not begin or end with `-`, and contains no run of two `-`

#### Scenario: A name that cannot begin an agent name is prefixed

- **WHEN** the agent name for `2fa-support` is derived
- **THEN** it is `c-2fa-support`
- **AND** the agent name for `-leading-dash` is `leading-dash`, the dash having been
  trimmed rather than prefixed

#### Scenario: A name with nothing usable in it

- **WHEN** the agent name for the empty string is derived
- **THEN** it is `change`
- **AND** the agent name for `!!!` and for `---` is also `change`, which is why the
  recording rule below must define what happens when one agent name is claimed twice

### Requirement: A derived name is recorded only when it differs from the change name

The plugin SHALL record the mapping from derived agent name to change name in
`agent-names.toml` inside the state directory, under a top-level `[names]` table whose
keys are agent names and whose values are change names. It SHALL record a mapping only
when the derived agent name differs from the change name: an identical name is already
attributable by the rule that an agent whose name equals a change name belongs to that
change, so writing it would add a file for no information. That test SHALL run before
the state directory is consulted, so an unchanged name succeeds even when no state
directory can be resolved. Recording SHALL be idempotent — recording a pair already
present SHALL leave the file byte-identical. When the agent name is already recorded
against a **different** change, the new change SHALL replace the old one: the most
recent launch is the live one, and a stale mapping would attribute a running agent to a
change it is not working on. The replacement SHALL NOT be reported as a problem; it is
the specified outcome, not a degradation.

#### Scenario: A truncated name is recorded

- **WHEN** the agent name for a 48-character change name is derived and recorded into an
  empty state directory
- **THEN** `agent-names.toml` exists and its `[names]` table maps the derived name to
  the full change name
- **AND** reading the mapping back returns that one pair

#### Scenario: An unchanged name is not recorded

- **WHEN** the agent name for `add-token-refresh` is derived and recorded into an empty
  state directory
- **THEN** no `agent-names.toml` is created
- **AND** the state directory is not created either

#### Scenario: Recording the same pair twice changes nothing

- **WHEN** the same derived name and change name are recorded twice in succession
- **THEN** the second call reports success
- **AND** the file's bytes are identical to those after the first call

#### Scenario: A second mapping is added beside the first

- **WHEN** a mapping is recorded into a state directory whose `agent-names.toml`
  already holds a different pair
- **THEN** the file holds both pairs
- **AND** reading it back returns both

#### Scenario: The same agent name recorded for a different change replaces it

- **WHEN** `agent-names.toml` maps `change` to `!!!` and a third unrelated pair, and
  `change` is recorded again for `---`
- **THEN** the file maps `change` to `---`
- **AND** the unrelated pair is unchanged
- **AND** no problem is reported, and the call returns success

### Requirement: An unusable mapping file yields an empty mapping

Reading the mapping SHALL NOT fail, panic, or return an error to the caller. An absent
state directory, an absent file, and an empty file SHALL each yield an empty mapping
with no problem reported. A file that is not valid TOML, or one whose `[names]` table is
missing or is not a table, SHALL yield an empty mapping and report one problem. An entry
whose value is not a string SHALL be skipped, with a problem reported, while every
well-formed entry in the same file is still returned. A recorded agent name that does
not match `[a-z][a-z0-9_-]{0,31}` SHALL be skipped the same way, so a hand-edited file
cannot make the plugin hand Herdr a name it will reject.

#### Scenario: Absent file and absent directory are both empty, not faults

- **WHEN** the mapping is read from a state directory that does not exist
- **THEN** the mapping is empty and no problem is reported
- **AND** the same holds when the directory exists but holds no `agent-names.toml`, and
  when `agent-names.toml` exists and is zero bytes

#### Scenario: A malformed file is empty and reports one problem

- **WHEN** `agent-names.toml` contains `[names` and nothing else
- **THEN** the mapping is empty
- **AND** exactly one problem is reported, naming `agent-names.toml`

#### Scenario: A bad entry is skipped and its neighbours survive

- **WHEN** `agent-names.toml`'s `[names]` table contains `good-agent = "a-real-change"`,
  `bad-agent = 7`, `Bad_Name = "another-change"`, and `"has spaces!" = "third-change"` —
  the last key quoted, because `!` and a space are not permitted in a TOML bare key and
  an unquoted form would make the whole file a syntax error rather than a file with a
  bad entry
- **THEN** the mapping contains exactly the `good-agent` pair
- **AND** three problems are reported, naming `bad-agent` (value is not a string),
  `Bad_Name` (a legal TOML key that fails the agent-name pattern on its uppercase
  characters), and `has spaces!` (a legal quoted TOML key that fails the pattern on its
  space and `!`)

#### Scenario: `names` is present but is not a table

- **WHEN** `agent-names.toml` contains `names = "nope"`
- **THEN** the mapping is empty
- **AND** one problem is reported, naming `names`

### Requirement: Recording is atomic and creates only what it needs

Recording SHALL create the state directory when it is missing, write the complete new
file contents to a temporary file inside that same directory, and rename it over
`agent-names.toml`. Replacing the file by rename rather than by writing into it is what
makes a concurrent reader observe either the previous file or the new one and never a
partial one. When the directory cannot be created, or the write or rename fails,
recording SHALL report the failure to the caller as an error rather than panicking, and
SHALL leave no temporary file behind. The caller treats a failed recording as a degraded
outcome and continues; attribution loses a mapping, and nothing else.

#### Scenario: The state directory is created on first record

- **WHEN** a mapping is recorded and the state directory does not exist
- **THEN** the directory is created and `agent-names.toml` is written inside it
- **AND** after the call the directory contains exactly `agent-names.toml` — no
  temporary file remains

#### Scenario: The file is replaced by rename, not written in place

- **WHEN** a mapping is recorded over an `agent-names.toml` that already holds a pair,
  having first taken a hard link to that file at a second path in the same directory
- **THEN** `agent-names.toml` holds the new mapping
- **AND** the hard-linked path still holds the **previous** bytes, which is true of a
  rename and false of any implementation that opens the target and writes into it —
  including `std::fs::write`, whose truncating write leaves no distinguishing tail
- **AND** the temporary file the implementation used was created inside the state
  directory, so the rename could not cross a filesystem boundary; after the call the
  directory holds only `agent-names.toml` and the test's own hard link

#### Scenario: Recording fails without panicking

- **WHEN** a mapping is recorded to a state directory path that is an existing regular
  file, so the directory cannot be created
- **THEN** the call returns an error naming the path
- **AND** that regular file is unchanged, and no temporary file exists beside it

### Requirement: Nothing is written outside the state directory

Every write this capability performs SHALL be inside the resolved state directory. In
particular nothing SHALL be written inside any repository's `openspec/` directory, which
this capability never reads or names, and nothing SHALL be written into the
configuration directory, which the user owns.

#### Scenario: A repository tree is untouched by a recording

- **WHEN** a mapping is recorded with the state directory pointed at a scratch path and
  a fixture repository containing an `openspec/changes/` tree present on disk
- **THEN** the fixture tree's listing, file bytes, and modification times are unchanged
- **AND** the only path created is inside the scratch state directory

#### Scenario: The configuration directory is not written to

- **WHEN** a mapping is recorded while the configuration directory and the state
  directory are both set to distinct existing scratch paths, the configuration directory
  holding a `config.toml`
- **THEN** the configuration directory's listing, every file's bytes, and every file's
  modification time are unchanged — the same three assertions the repository-tree
  scenario makes, so an in-place rewrite of identical length could not pass
</content>
