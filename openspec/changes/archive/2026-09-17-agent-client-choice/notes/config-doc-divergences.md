# Configuration-format divergences recorded at task 3.6

The contract gate this repository's rules require for a `config.toml` format change:
every divergence between the new key set and the two documents that describe it,
recorded here at the group that changed the code and repaired in **group 13**.

Measured after group 3 landed, against `SPEC.md` and `README.md` at that point.

| # | Site | Says today | Owed |
|---|---|---|---|
| 1 | `README.md:88`, `agent_kind` row | Default `claude` | No default. Absent when unset; the launcher resolves the kind by `integration-status`' five-step precedence, and `claude` is only its last resort. A **blank** value now reports a problem |
| 2 | `README.md` configuration table | Three rows: `openspec_bin`, `agent_kind`, `archived_count` | A fourth: `[prompts.<kind>]`, with the `{openspec}`/`{change}` placeholders and the three intent keys `apply`, `continue`, `archive` |
| 3 | `SPEC.md:85`, module map's `config` row | reads `config.toml` into `openspec_bin`, `agent_kind`, `archived_count` | `prompts` beside them |
| 4 | `SPEC.md:236`, the probe's step 1 prose | "`config.toml` also holds `agent_kind` and `archived_count`" | `[prompts.<kind>]` named too, and `agent_kind` described as an override with no default rather than as a defaulted value |
| 5 | `SPEC.md:913`, the launch-flow prose | "`<kind>` comes from plugin configuration (`agent_kind`, default `claude`)" | `<kind>` comes from `integration::resolve`'s precedence over `config.toml`, `settings.toml`, and `herdr integration status` |
| 6 | `SPEC.md`, state-directory section | Names only `agent-names.toml` | `settings.toml`'s one **read** key, `agent_kind`; this change never writes that file |

Nothing here is a code defect: each is prose that group 3's and group 7's code
falsifies, owned by tasks 13.1, 13.2, and 13.3.
