## ADDED Requirements

### Requirement: The state directory is resolved from the process environment

The plugin SHALL determine its state directory by reading the process environment, and
SHALL NOT spawn any process to obtain it. `HERDR_PLUGIN_STATE_DIR`, which Herdr sets on
every plugin pane and action process, is authoritative when it is set to a non-empty
value. When it is unset or empty the directory SHALL fall back to
`$XDG_STATE_HOME/herdr/plugins/herdr-openspec` when `XDG_STATE_HOME` is set and
non-empty, and otherwise to `$HOME/.local/state/herdr/plugins/herdr-openspec` — the
path Herdr itself supplies on this platform. When none of the three variables is
available there SHALL be no state directory, and every read SHALL yield an empty
mapping while every write SHALL report that it could not be performed.

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
- **AND** an `XDG_STATE_HOME` set to the empty string produces the same result

#### Scenario: No directory can be resolved at all

- **WHEN** the directory is resolved with all three variables absent
- **THEN** no directory is resolved
- **AND** reading the mapping yields an empty mapping with no problem reported
- **AND** recording a mapping reports a failure to the caller and creates nothing

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
   trailing `-` or `_`, and append `-` followed by four base-36 digits of a 32-bit
   FNV-1a hash of the **original** change name, left-padded with `0`.

Step 6 makes a truncated name depend on the whole change name, so two changes sharing a
27-character prefix do not collide, and the same change always produces the same agent
name across processes and machines. The function SHALL NOT consult the recorded mapping,
the filesystem, or Herdr's live agent list; uniqueness among *live* agents is Herdr's
check at `agent start` and belongs to `agent-launch`.

#### Scenario: A short kebab-case change name is unchanged

- **WHEN** the agent name for the change `add-token-refresh` is derived
- **THEN** it is `add-token-refresh`
- **AND** it matches `^[a-z][a-z0-9_-]{0,31}$`

#### Scenario: A name of exactly 32 characters is not truncated

- **WHEN** the agent name for a 32-character kebab-case change name is derived
- **THEN** the result is that name unchanged, at 32 characters
- **AND** deriving the name for the same string with one character appended produces a
  different, 32-character result carrying the four-digit suffix

#### Scenario: A long change name is truncated with a suffix from the whole name

- **WHEN** the agent names for `add-really-long-change-name-that-overflows-alpha` and
  `add-really-long-change-name-that-overflows-beta` are derived
- **THEN** both are at most 32 characters and match `^[a-z][a-z0-9_-]{0,31}$`
- **AND** they differ from each other, even though their first 27 characters are equal
- **AND** deriving either one a second time yields the identical string

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
- **AND** the agent name for `!!!` and for `---` is also `change`

### Requirement: A derived name is recorded only when it differs from the change name

The plugin SHALL record the mapping from derived agent name to change name in
`agent-names.toml` inside the state directory, under a top-level `[names]` table whose
keys are agent names and whose values are change names. It SHALL record a mapping only
when the derived agent name differs from the change name: an identical name is already
attributable by the rule that an agent whose name equals a change name belongs to that
change, so writing it would add a file for no information. Recording SHALL be
idempotent — recording a pair already present SHALL leave the file byte-identical.

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
  `bad-agent = 7`, and `Bad_Name! = "another-change"`
- **THEN** the mapping contains exactly the `good-agent` pair
- **AND** two problems are reported, naming `bad-agent` and `Bad_Name!`

#### Scenario: `names` is present but is not a table

- **WHEN** `agent-names.toml` contains `names = "nope"`
- **THEN** the mapping is empty
- **AND** one problem is reported, naming `names`

### Requirement: Recording is atomic and creates only what it needs

Recording SHALL create the state directory when it is missing, write the complete new
file contents to a temporary file inside that same directory, and rename it over
`agent-names.toml`. A concurrent reader SHALL therefore observe either the previous file
or the new one, never a partial one. When the directory cannot be created, or the write
or rename fails, recording SHALL report the failure to the caller as an error rather
than panicking, and SHALL leave no temporary file behind. The caller treats a failed
recording as a degraded outcome and continues; attribution loses a mapping, and nothing
else.

#### Scenario: The state directory is created on first record

- **WHEN** a mapping is recorded and the state directory does not exist
- **THEN** the directory is created and `agent-names.toml` is written inside it
- **AND** after the call the directory contains exactly `agent-names.toml` — no
  temporary file remains

#### Scenario: The write is not visible until it is complete

- **WHEN** a mapping is recorded over an `agent-names.toml` that already holds a pair
- **THEN** at no point does `agent-names.toml` contain content that parses to neither
  the old mapping nor the new one, because the new content is renamed into place rather
  than written in place
- **AND** the temporary file used is created inside the state directory, so the rename
  cannot cross a filesystem boundary

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
  directory are both set to distinct existing scratch paths
- **THEN** the configuration directory's listing is unchanged
