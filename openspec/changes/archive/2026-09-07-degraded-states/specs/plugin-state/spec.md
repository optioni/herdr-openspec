## ADDED Requirements

### Requirement: Recording rewrites the mapping from the entries the read recovered, and says so

`SPEC.md` → Degraded states claims of an unusable `agent-names.toml`: "Empty or partial
mapping; attribution falls back to the name-equality tier, and nothing already on disk is
lost." The first two clauses are true. The third is true only of the **read**, and this
change SHALL make the distinction explicit rather than leaving it to be discovered.

`state::read` SHALL remain non-destructive: it opens the file, recovers what it can, records
its own reason on `Mapping::problems`, and writes nothing. Nothing is lost by reading.

`state::record` SHALL be documented and tested as what it is: it re-reads the mapping, inserts
the new pair, and rewrites the file **from the recovered entries alone**. Bytes the parse
could not recover therefore do not survive the next successful record. This SHALL NOT be
changed to a refusal: a `record` that declined to write whenever the file was slightly
malformed would leave a freshly launched agent unattributable for the rest of the session,
which is a worse failure than losing entries the plugin itself wrote and can write again.

The behaviour SHALL be pinned by a test rather than left as an emergent property of
`read`-then-serialise, so a later change that makes `record` merge unparsed bytes, or refuse,
is a deliberate edit to a failing test rather than a silent reversal.

#### Scenario: Reading an unusable mapping leaves the file byte-identical

- **WHEN** a scratch state directory holding an `agent-names.toml` whose contents are
  `[names]\nok = "alpha"\nthis is not toml\n` is snapshotted, `state::read` is called over it,
  and it is snapshotted again
- **THEN** the two snapshots are identical — the path, its mode, and its bytes
- **AND** the returned `Mapping::problems` is non-empty and names the file
- **AND** the returned `Mapping::names` is whatever the parse recovered, empty or partial,
  with no entry invented

#### Scenario: Recording after an unusable read drops what the parse could not recover

- **WHEN** the same directory is used, `state::record(dir, "c-beta", "beta")` is called, and
  the file is read back as raw bytes
- **THEN** the call returns `Ok(())` — the record is not refused
- **AND** the file's `[names]` table holds `c-beta = "beta"` together with exactly the entries
  `state::read` recovered, and nothing else
- **AND** the unrecoverable line `this is not toml` is **absent** from the rewritten file, which
  is the clause `SPEC.md` is corrected to state

#### Scenario: Recording over a clean mapping preserves every entry

- **WHEN** a scratch state directory holding a well-formed `agent-names.toml` with three pairs
  receives `state::record(dir, "c-delta", "delta")`
- **THEN** the file holds all four pairs afterwards
- **AND** the three pre-existing pairs are byte-for-byte the values they held, so the drop in
  the scenario above is the parse failure's consequence and not the rewrite's
