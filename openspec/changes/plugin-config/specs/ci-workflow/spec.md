## MODIFIED Requirements

### Requirement: CI invokes every gate through `make`, so no command is written twice

Every gate the workflow runs SHALL be invoked as `make <target>` against the
repository's root `Makefile`. No `run:` step SHALL contain the string `cargo`, and no
`run:` step SHALL restate a gate's command line, its flags, or the coverage threshold.
The workflow SHALL declare no `env:` mapping at any level — workflow, job, or step —
because an environment variable such as `RUSTFLAGS` changes what a gate does without
appearing in any `run:` body, which is the same second definition reached through a door
no `run:`-level rule watches. Every `run:` step SHALL be written as a single line rather
than a block scalar, so the parity guard can read the workflow without a YAML parser.
The guard is deliberately std-only string matching: adding a YAML parser as a
dev-dependency to read four lines is not a trade this repository makes. The workflow
SHALL NOT invoke the composite `make check` target, because coverage runs on only one of
the two runners.

This is what makes `quality-gates`' claim that "local runs and CI invoke identical
commands" enforceable rather than aspirational.

#### Scenario: Every `run:` step is a make invocation of a declared target

- **WHEN** every `run:` step body in `.github/workflows/ci.yml` is collected
- **THEN** each one either begins with `make ` or is the aggregate job's `exit 1`
- **AND** every target named after `make ` appears in the root `Makefile`'s `.PHONY`
  list, so a renamed or misspelled target fails the test rather than failing CI
- **AND** none of them contains the string `cargo`

#### Scenario: No environment mapping redefines what a gate does

- **WHEN** the workflow is searched for an `env:` key at any indentation
- **THEN** there is no match
- **AND** a `RUSTFLAGS: "-A warnings"` that would neuter `make lint` while leaving every
  `run:` body untouched therefore cannot be added without failing the guard

#### Scenario: The coverage threshold appears only in the Makefile

- **WHEN** `.github/workflows/ci.yml` is searched for `--fail-under-lines`
- **THEN** there is no match, so the 80% floor cannot be raised, lowered, or
  contradicted per platform from inside the workflow
- **AND** the Makefile's `coverage` recipe still carries `--fail-under-lines 80`, so
  the absence in the workflow is a relocation of the threshold and not a deletion of it

#### Scenario: The composite target is not used

- **WHEN** the workflow is searched for `make check`
- **THEN** there is no match
- **AND** `make fmt-check`, `make lint`, `make test`, and `make coverage` each appear
  at least once, so every gate `make check` composes is still covered by the workflow

#### Scenario: Every `run:` step is a single line

- **WHEN** the workflow is searched for a block-scalar `run:` in either the pipe or the
  folded form
- **THEN** there is no match, so each step's command is one readable, greppable line
  and the parity guard needs no YAML parser to read it
