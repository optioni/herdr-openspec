//! The subprocess seam: `cli` is the only module in this crate permitted to
//! spawn a process. See `SPEC.md` -> Architecture ("The subprocess seam") for
//! the contract this module implements, and
//! `openspec/changes/subprocess-seam/design.md` for the full design.
//!
//! Not to be confused with `tests/cli.rs`, which is a different file with the
//! same base name and an unrelated meaning: it exercises this crate's own
//! binary's command-line interface (`herdr-openspec ui`, `herdr-openspec
//! wat`, and so on) by spawning `env!("CARGO_BIN_EXE_herdr-openspec")`. This
//! module is the seam through which the crate calls the external `openspec`
//! and `herdr` programs.
