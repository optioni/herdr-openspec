//! Contract tier: binds documented claims in this repository's own documents to the
//! second sites that determine their truth — the crate's module list, its production
//! worker threads, its declared MSRV, the programs `make check` invokes, the plugin
//! manifest, and the project context injected into every OpenSpec agent's prompt. See
//! `openspec/changes/doc-conformance/specs/doc-conformance/spec.md` for the full contract
//! this file implements, and `design.md` -> Decision 1 for why this lives here rather than
//! as a `scripts/gates/` shell script.
//!
//! Follows `tests/manifest.rs`'s discipline exactly: parse the second site, parse the
//! document, compare, name both sides on failure, and assert nothing that has no second
//! site. Every leg is two functions — a pure parser over `&str` with its own unit tests
//! over hand-written (including malformed) inputs, and a thin `#[test]` that feeds it the
//! real file — the same shape `tests/degraded_coverage.rs`'s `parse_spec_conditions` uses:
//! a pure function returning `Result`, never a panic.
//!
//! No process is spawned here: no `openspec`, no `herdr`, no `cargo metadata`, no `git`.
//! Repository files are located from `CARGO_MANIFEST_DIR`, read via the compile-time
//! `env!` macro, never `std::env::var` at run time.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A scratch directory under `std::env::temp_dir()`, removed recursively on drop —
/// `testutil::ScratchDir`'s shape reimplemented here for the same reason `tests/cli.rs`,
/// `tests/gate_controls.rs`, and `tests/spec_purposes.rs` each carry their own copy: this is
/// a separate integration-test crate and cannot reach the library's `pub(crate)` items.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new(label: &str) -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "herdr-openspec-doc-contract-{}-{counter}-{label}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("create scratch dir");
        Self { path }
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Read a document's full text. `Err` names the path when the file does not exist (or is
/// otherwise unreadable), rather than treating an unreadable document as agreeing with
/// whatever it was compared against.
fn read_doc(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("could not read {}: {e}", path.display()))
}

/// Slice the section of `text` introduced by `heading` (a literal line or inline marker,
/// matched as a substring) up to the next line starting with the same leading marker
/// (`#`-prefixed headings stop at the next `#`-prefixed line of equal or higher level;
/// non-heading markers such as `**Module map:**` stop at the next blank-line-delimited
/// heading-like boundary). Concretely: find `heading`'s first occurrence; the section runs
/// from there to the end of the text, or, when `heading` starts with one or more `#`
/// characters, up to (excluding) the next line that starts with `#` at the same or a
/// shallower depth.
///
/// `Err` names the heading when it cannot be found at all — "the section is gone" is a
/// distinct failure from "the section disagrees with the second site".
fn section<'a>(text: &'a str, heading: &str) -> Result<&'a str, String> {
    let start = text
        .find(heading)
        .ok_or_else(|| format!("heading {heading:?} was not found"))?;
    let after_heading = &text[start..];

    // Determine the heading's own `#` depth, if any, so the section stops at the next
    // heading of equal-or-shallower depth rather than running to end of file.
    let hashes = heading.chars().take_while(|&c| c == '#').count();

    let mut end = text.len();
    if hashes > 0 {
        // Skip past the heading line itself before scanning for the next boundary.
        let after_heading_line = after_heading.find('\n').map(|i| start + i + 1);
        if let Some(scan_from) = after_heading_line {
            let mut offset = scan_from;
            for line in text[scan_from..].lines() {
                let line_hashes = line.chars().take_while(|&c| c == '#').count();
                if line_hashes > 0 && line_hashes <= hashes {
                    end = offset;
                    break;
                }
                offset += line.len() + 1;
            }
        }
    }

    Ok(&text[start..end])
}

/// Extract the module names from `src/lib.rs`'s `pub mod <name>;` declarations. Anchored to
/// the start of a line (after trimming leading whitespace) so a `pub mod` occurring inside a
/// comment or a string literal is not matched.
fn pub_mod_names(lib_rs: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for line in lib_rs.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("pub mod ")
            && rest.contains(';')
        {
            // Cut at the FIRST `;` rather than requiring it to be the trimmed line's last
            // character — a trailing line comment (`pub mod a; // note`) puts characters
            // after the semicolon, and a suffix-only check silently drops the module.
            let name = rest.split(';').next().unwrap_or("").trim();
            if !name.is_empty() {
                names.insert(name.to_string());
            }
        }
    }
    names
}

/// Parse the data rows of the table introduced by `**Module map:**` in `SPEC.md`: skip the
/// header row and the `|---|---|` separator, take each remaining row's first cell, strip
/// backticks. Stops at the first blank line after the table starts (a Markdown table ends
/// where its own row syntax stops).
///
/// `Err` when the heading itself cannot be found — this must not silently return an empty
/// set, which would compare unequal to a non-empty module set and look like a real defect
/// rather than a parser that lost its anchor.
fn module_map_names(spec_md: &str) -> Result<BTreeSet<String>, String> {
    let table_section = section(spec_md, "**Module map:**")?;

    let mut names = BTreeSet::new();
    let mut in_table = false;
    for line in table_section.lines().skip(1) {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            if in_table {
                break;
            }
            continue;
        }
        in_table = true;

        let is_separator = trimmed
            .trim_matches('|')
            .split('|')
            .all(|cell| !cell.is_empty() && cell.chars().all(|c| c == '-' || c == ':'));
        if is_separator {
            continue;
        }

        let first_cell = trimmed
            .trim_start_matches('|')
            .split('|')
            .next()
            .unwrap_or("")
            .trim();
        if first_cell == "Module" {
            continue;
        }
        let stripped = first_cell.trim_matches('`').to_string();
        if !stripped.is_empty() {
            names.insert(stripped);
        }
    }
    Ok(names)
}

/// Report a `BTreeSet` difference in both directions, or `None` when the sets agree.
fn set_diff_message(
    label_a: &str,
    a: &BTreeSet<String>,
    label_b: &str,
    b: &BTreeSet<String>,
) -> Option<String> {
    let only_a: Vec<_> = a.difference(b).cloned().collect();
    let only_b: Vec<_> = b.difference(a).cloned().collect();
    if only_a.is_empty() && only_b.is_empty() {
        return None;
    }
    let mut msg = String::new();
    if !only_a.is_empty() {
        msg.push_str(&format!("in {label_a} but not {label_b}: {only_a:?}; "));
    }
    if !only_b.is_empty() {
        msg.push_str(&format!("in {label_b} but not {label_a}: {only_b:?}"));
    }
    Some(msg)
}

#[test]
fn the_overlay_submodule_is_invisible_to_both_map_checks() {
    // `help-overlay` -> `doc-conformance`: "The submodule is invisible to both map
    // checks, and that is asserted rather than assumed".
    //
    // `src/ui/help.rs` is a submodule. `pub_mod_names` reads `src/lib.rs`'s
    // TOP-LEVEL `pub mod` declarations, and `SPEC.md`'s Module map carries a single
    // `ui` row for every file under `src/ui/`, so adding the overlay module failed
    // neither map check — exactly as adding `src/ui/palette.rs` failed neither.
    //
    // An earlier draft of that requirement claimed both checks WOULD fail until the
    // documents named `ui::help`, which is false and would have made its own
    // scenario unfalsifiable. This test is the executable form of the true claim,
    // so a future change that extends either check to submodule granularity fails
    // HERE and is told to update the requirement, rather than discovering the
    // granularity by surprise.
    let lib_rs = read_doc(&manifest_dir().join("src/lib.rs")).expect("read src/lib.rs");
    let names = pub_mod_names(&lib_rs);
    assert!(
        names.contains("ui"),
        "src/lib.rs declares no top-level `pub mod ui`: {names:?}"
    );
    for absent in ["help", "ui::help"] {
        assert!(
            !names.contains(absent),
            "pub_mod_names now yields `{absent}`, so it has gained submodule \
             granularity - update `specs/doc-conformance`'s invisibility scenario \
             rather than deleting this assertion: {names:?}"
        );
    }
}

#[test]
fn pub_mod_names_handles_comments_attributes_and_indentation() {
    let lib_rs = "\
pub mod a; // a brand-new module
#[cfg(feature = \"x\")]
pub mod b;
    pub mod c;
// pub mod d;
pub mod e;
";
    let names = pub_mod_names(lib_rs);
    let expected: BTreeSet<String> = ["a", "b", "c", "e"].iter().map(|s| s.to_string()).collect();
    assert_eq!(
        names, expected,
        "a trailing line comment, an attributed declaration, indentation, and a `pub mod` \
         occurring inside a `//` comment must each be handled correctly: {names:?}"
    );
}

#[test]
fn module_map_matches_lib_rs() {
    let lib_rs = read_doc(&manifest_dir().join("src/lib.rs")).expect("read src/lib.rs");
    let spec_md = read_doc(&manifest_dir().join("SPEC.md")).expect("read SPEC.md");

    let declared = pub_mod_names(&lib_rs);
    let mapped = module_map_names(&spec_md).expect("parse SPEC.md's module map");

    if let Some(diff) = set_diff_message(
        "src/lib.rs's pub mod set",
        &declared,
        "SPEC.md's Module map",
        &mapped,
    ) {
        panic!("SPEC.md's Module map disagrees with src/lib.rs's declared modules: {diff}");
    }
}

#[test]
fn map_missing_row() {
    let declared: BTreeSet<String> = ["config", "state", "open"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let spec_md = "\
**Module map:**

| Module | Responsibility |
|---|---|
| `config` | resolve config |
| `state` | resolve state |
";
    let mapped = module_map_names(spec_md).expect("parse synthetic module map");
    let diff = set_diff_message("declared", &declared, "mapped", &mapped)
        .expect("expected a difference: open is declared but unmapped");
    assert!(
        diff.contains("open"),
        "diff should name the missing module `open`: {diff}"
    );
}

#[test]
fn map_orphan_row() {
    let declared: BTreeSet<String> = ["config", "state"].iter().map(|s| s.to_string()).collect();
    let spec_md = "\
**Module map:**

| Module | Responsibility |
|---|---|
| `config` | resolve config |
| `state` | resolve state |
| `ghost` | no longer exists |
";
    let mapped = module_map_names(spec_md).expect("parse synthetic module map");
    let diff = set_diff_message("declared", &declared, "mapped", &mapped)
        .expect("expected a difference: ghost is mapped but undeclared");
    assert!(
        diff.contains("ghost"),
        "diff should name the orphan row `ghost`: {diff}"
    );
}

#[test]
fn absent_heading() {
    let spec_md = "# SPEC\n\nNo module map heading here at all.\n";
    let result = module_map_names(spec_md);
    assert!(
        result.is_err(),
        "module_map_names must fail loudly when the heading is absent, not return an empty set"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("Module map"),
        "error should name the heading it searched for: {err}"
    );
}

/// Return the subset of `declared` that does not appear in `section_text` as a `<name>::`
/// token. A bare mention of the module name with no trailing `::` does not satisfy the leg
/// (`specs/doc-conformance/spec.md` -> "Every public module is named in the tested-modules
/// list"). Returned as a `BTreeSet` so a failure names every missing module, sorted, rather
/// than only the first one a `grep -c`-shaped check would happen to count.
fn missing_tested_modules(section_text: &str, declared: &BTreeSet<String>) -> BTreeSet<String> {
    declared
        .iter()
        .filter(|name| {
            let token = format!("{name}::");
            // Left-boundary-only: a `<name>::` token must not be preceded by an identifier
            // character (`reopen::` must not satisfy a search for `open::`), but nothing
            // needs to be true of the character AFTER the token — `open::context` is exactly
            // the shape every real mention takes, and a right-boundary check would reject it.
            !bounded_mention_asym(
                section_text,
                &token,
                |c| c.is_ascii_alphanumeric() || c == b'_',
                |_| false,
            )
        })
        .cloned()
        .collect()
}

#[test]
fn tested_modules_names_every_module() {
    let lib_rs = read_doc(&manifest_dir().join("src/lib.rs")).expect("read src/lib.rs");
    let spec_md = read_doc(&manifest_dir().join("SPEC.md")).expect("read SPEC.md");

    let declared = pub_mod_names(&lib_rs);
    let tested_section = section(&spec_md, "### Unit-tested modules")
        .expect("find the '### Unit-tested modules' section");

    let missing = missing_tested_modules(tested_section, &declared);
    assert!(
        missing.is_empty(),
        "modules declared `pub mod` in src/lib.rs but not named as `<name>::` in SPEC.md's \
         '### Unit-tested modules' section: {missing:?}"
    );
}

#[test]
fn tested_modules_missing() {
    let declared: BTreeSet<String> = ["config", "state", "launch"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let section_text = "\
### Unit-tested modules

- `config::load` reads configuration
- `state::read` reads the mapping

### Next section
";
    let missing = missing_tested_modules(section_text, &declared);
    let expected: BTreeSet<String> = ["launch".to_string()].into_iter().collect();
    assert_eq!(
        missing, expected,
        "expected only `launch` to be reported missing"
    );
}

#[test]
fn tested_modules_rejects_identifier_prefixed_token() {
    let declared: BTreeSet<String> = ["open"].iter().map(|s| s.to_string()).collect();
    // The section names `reopen::`, whose trailing "open::" is a substring of a longer
    // identifier — an unbounded `contains` would wrongly treat this as satisfying `open`.
    let section_text = "\
### Unit-tested modules

- `reopen::run` reopens a change
";
    let missing = missing_tested_modules(section_text, &declared);
    let expected: BTreeSet<String> = ["open".to_string()].into_iter().collect();
    assert_eq!(
        missing, expected,
        "`reopen::` must not satisfy a search for `open::`: {missing:?}"
    );
}

#[test]
fn tested_modules_scoped_to_section() {
    let declared: BTreeSet<String> = ["open"].iter().map(|s| s.to_string()).collect();
    // `open::` is named above the heading and again after the next heading, but never
    // inside the tested-modules section itself. A whole-file grep would wrongly pass this.
    let text = "\
## Somewhere else entirely

`open::context` reads the invocation context.

### Unit-tested modules

- `config::load` reads configuration

### Next section

`open::run` drives the whole subcommand.
";
    let tested_section = section(text, "### Unit-tested modules")
        .expect("find the '### Unit-tested modules' section");
    let missing = missing_tested_modules(tested_section, &declared);
    let expected: BTreeSet<String> = ["open".to_string()].into_iter().collect();
    assert_eq!(
        missing, expected,
        "a mention outside the section must not satisfy the leg"
    );
}

#[test]
fn missing_document() {
    let path = manifest_dir().join("this-file-does-not-exist-doc-contract.md");
    let result = read_doc(&path);
    assert!(result.is_err(), "read_doc must fail on a missing path");
    let err = result.unwrap_err();
    assert!(
        err.contains("this-file-does-not-exist-doc-contract.md"),
        "error should name the missing path: {err}"
    );
}

/// Whether `version` appears in `text` delimited by a non-version character on the LEFT (an
/// ASCII digit or `.`, so `2.1.88` does not satisfy a search for `1.88`) and by a non-digit
/// character on the RIGHT ONLY. The two boundaries are deliberately asymmetric: `.` is not
/// rejected on the right, so `1.88.0` (a patch-version suffix) and `the floor is 1.88.` (a
/// sentence-ending period) both satisfy a search for `1.88`, while `1.889` still does not (a
/// digit immediately to the right). A version occurring at the very start or end of `text`
/// counts as delimited on that side. Shares its boundary-scanning core, `bounded_mention_asym`
/// (defined below, alongside the gate-programs leg that also needs a boundary-checked
/// document match), with `program_mentioned`'s symmetric `bounded_mention`.
fn msrv_mentions(text: &str, version: &str) -> bool {
    bounded_mention_asym(
        text,
        version,
        |c| c.is_ascii_digit() || c == b'.',
        |c: u8| c.is_ascii_digit(),
    )
}

/// Read `rust-version` from a `Cargo.toml`-shaped TOML document's `[package]` table, via the
/// `toml` crate — never by spawning `cargo metadata`. `Err` names what is wrong: unparseable
/// TOML, or a missing/non-string `rust-version` key.
fn manifest_rust_version(cargo_toml: &str) -> Result<String, String> {
    let table: toml::Table = cargo_toml
        .parse()
        .map_err(|e| format!("Cargo.toml is not valid TOML: {e}"))?;
    table
        .get("package")
        .and_then(|pkg| pkg.get("rust-version"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| "Cargo.toml's [package] has no string rust-version key".to_string())
}

#[test]
fn msrv_is_documented() {
    let cargo_toml = read_doc(&manifest_dir().join("Cargo.toml")).expect("read Cargo.toml");
    let version = manifest_rust_version(&cargo_toml).expect("parse Cargo.toml's rust-version");

    let agents_md = read_doc(&manifest_dir().join("AGENTS.md")).expect("read AGENTS.md");
    let readme_md = read_doc(&manifest_dir().join("README.md")).expect("read README.md");

    let agents_section =
        section(&agents_md, "## Environment").expect("find AGENTS.md's Environment section");
    let readme_section =
        section(&readme_md, "## Development").expect("find README.md's Development section");

    let mut missing = Vec::new();
    if !msrv_mentions(agents_section, &version) {
        missing.push("AGENTS.md's Environment section");
    }
    if !msrv_mentions(readme_section, &version) {
        missing.push("README.md's Development section");
    }
    assert!(
        missing.is_empty(),
        "Cargo.toml's rust-version ({version:?}) is not documented in: {missing:?}"
    );
}

#[test]
fn msrv_bump_is_caught() {
    let bumped_cargo_toml = "[package]\nname = \"x\"\nrust-version = \"1.92\"\n";
    let version = manifest_rust_version(bumped_cargo_toml).expect("parse synthetic Cargo.toml");

    // Doc text unchanged from before the bump: still names the old floor, not the new one.
    let doc_section = "## Environment\n\n- **Rust** stable, floor 1.88 today.\n";
    assert!(
        !msrv_mentions(doc_section, &version),
        "a bumped rust-version ({version:?}) must not be satisfied by unchanged doc text"
    );
}

#[test]
fn msrv_mentions_left_boundary_control() {
    // "11.88" contains "1.88" as a substring, but with the digit `1` immediately to its
    // left — must not count as a delimited mention.
    assert!(
        !msrv_mentions("supports Rust 11.88 and later", "1.88"),
        "a digit immediately to the left of the match must not satisfy the leg"
    );
}

#[test]
fn msrv_mentions_right_boundary_control() {
    // "1.889" contains "1.88" as a substring, but with the digit `9` immediately to its
    // right — must not count as a delimited mention.
    assert!(
        !msrv_mentions("supports Rust 1.889 and later", "1.88"),
        "a digit immediately to the right of the match must not satisfy the leg"
    );
}

#[test]
fn msrv_mentions_sentence_ending_period_satisfies() {
    // "." is not an ASCII digit, so it must satisfy the RIGHT boundary — ordinary prose
    // ending a sentence right after the version must not be reported undocumented.
    assert!(
        msrv_mentions("the floor is 1.88.", "1.88"),
        "a sentence-ending period immediately after the version must satisfy the leg"
    );
}

#[test]
fn msrv_mentions_patch_version_suffix_satisfies() {
    // "1.88.0" contains "1.88" followed by ".", not a digit, so it satisfies the (relaxed)
    // right boundary even though the full token is a patch version rather than bare "1.88".
    assert!(
        msrv_mentions("Rust 1.88.0 or newer", "1.88"),
        "\"1.88.0\" must satisfy a search for \"1.88\" now that `.` is not a right-boundary char"
    );
}

#[test]
fn msrv_mentions_left_boundary_still_rejects_dotted_prefix() {
    // "2.1.88" has "1.88" preceded by ".", which the LEFT boundary must still reject —
    // only the right-hand boundary set drops `.`.
    assert!(
        !msrv_mentions("2.1.88", "1.88"),
        "a `.` immediately to the left of the match must still be rejected"
    );
}

#[test]
fn msrv_mentions_delimited_match() {
    assert!(msrv_mentions("rust-version = \"1.88\"", "1.88"));
    assert!(msrv_mentions("the floor is 1.88 for now", "1.88"));
}

#[test]
fn manifest_rust_version_missing_key() {
    let cargo_toml = "[package]\nname = \"x\"\n";
    let result = manifest_rust_version(cargo_toml);
    assert!(
        result.is_err(),
        "manifest_rust_version must fail loudly when rust-version is absent, not panic or \
         return an empty string"
    );
}

// --- Gate-path programs leg -------------------------------------------------------------
//
// See `specs/doc-conformance/spec.md` -> "Every non-cargo program `make check` invokes is
// documented as a prerequisite", and `design.md` -> Decision 7 for why the extraction rule
// below is stated against the two real recipe shapes the `Makefile` contains (a shell guard
// block, and a quoted assignment value) rather than a naive "first token per line" rule.

/// Whether `needle` occurs in `text` delimited by a character the corresponding side's
/// predicate rejects — the LEFT and RIGHT boundary character sets need not agree. A version
/// number's boundaries are asymmetric: `2.1.88` must still reject `1.88` on its dotted LEFT
/// side, but `1.88.0` (a patch-version suffix) and `1.88.` (a sentence-ending period) must
/// both satisfy a search for `1.88` on the RIGHT. `bounded_mention` below is the symmetric
/// case, used everywhere the two sides agree.
fn bounded_mention_asym(
    text: &str,
    needle: &str,
    is_left_boundary_char: impl Fn(u8) -> bool,
    is_right_boundary_char: impl Fn(u8) -> bool,
) -> bool {
    if needle.is_empty() {
        return false;
    }
    let bytes = text.as_bytes();
    let nbytes = needle.as_bytes();
    let mut search_start = 0;
    while let Some(rel) = text[search_start..].find(needle) {
        let idx = search_start + rel;
        let left_ok = idx == 0 || !is_left_boundary_char(bytes[idx - 1]);
        let end = idx + nbytes.len();
        let right_ok = end == bytes.len() || !is_right_boundary_char(bytes[end]);
        if left_ok && right_ok {
            return true;
        }
        search_start = idx + 1;
    }
    false
}

/// Whether `word` occurs in `text` delimited by a non-word character (anything but an ASCII
/// alphanumeric or `_`) on both sides, so a program name occurring as part of a longer
/// identifier does not satisfy a search for it. Shared with `msrv_mentions`, which uses the
/// asymmetric core directly since its two boundary sides differ.
fn bounded_mention(text: &str, needle: &str, is_boundary_char: impl Fn(u8) -> bool) -> bool {
    bounded_mention_asym(text, needle, &is_boundary_char, &is_boundary_char)
}

/// Whether `program` occurs in `text` as a whole word: not immediately adjacent to another
/// alphanumeric character or `_` on either side.
fn program_mentioned(text: &str, program: &str) -> bool {
    bounded_mention(text, program, |c| c.is_ascii_alphanumeric() || c == b'_')
}

/// The shell control-flow keywords and builtins step 6 of the extraction rule discards. A
/// recipe line whose first remaining token is one of these names no external program.
const SHELL_KEYWORDS: &[&str] = &[
    "if", "then", "else", "elif", "fi", "for", "do", "done", "while", "case", "esac", "echo",
    "exit", "test", "[", ":", "cd", "set", "true", "false",
];

/// Whether `token` is a `NAME=value` assignment: a leading run of ASCII letters/digits/`_`
/// starting with a letter or `_`, followed by `=`.
fn is_assignment_token(token: &str) -> bool {
    let Some(eq_idx) = token.find('=') else {
        return false;
    };
    let name = &token[..eq_idx];
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        _ => false,
    }
}

/// Split a recipe line into tokens, quote-aware: a single- or double-quoted run becomes part
/// of the token it occurs in (quote characters themselves are dropped), so a value such as
/// `ENTRY='pub fn run_from_env\('` never splits into several tokens because of the spaces its
/// quotes protect. This is step 3 of the extraction rule.
fn tokenize_quote_aware(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_token = false;
    let mut quote: Option<char> = None;
    for c in line.chars() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            } else {
                current.push(c);
            }
            continue;
        }
        match c {
            '\'' | '"' => {
                quote = Some(c);
                in_token = true;
            }
            c if c.is_whitespace() => {
                if in_token {
                    tokens.push(std::mem::take(&mut current));
                    in_token = false;
                }
            }
            c => {
                current.push(c);
                in_token = true;
            }
        }
    }
    if in_token {
        tokens.push(current);
    }
    tokens
}

/// Join Make recipe continuation lines (a line ending in `\`) with the line that follows, so a
/// multi-line shell construct becomes one logical line. This is step 1 of the extraction rule.
/// Every other line (including target-definition lines) passes through unchanged.
fn join_continuations(makefile: &str) -> Vec<String> {
    let mut logical = Vec::new();
    let mut current = String::new();
    let mut joining = false;
    for line in makefile.lines() {
        if let Some(stripped) = line.strip_suffix('\\') {
            current.push_str(stripped);
            current.push(' ');
            joining = true;
        } else {
            current.push_str(line);
            logical.push(std::mem::take(&mut current));
            joining = false;
        }
    }
    if joining {
        logical.push(current);
    }
    logical
}

/// Find the `check:` target's own logical line among `logical_lines` and return its
/// prerequisite target names, in order. `Err` when no such line exists — a `Makefile` with no
/// `check:` target cannot yield a program set at all.
fn check_prerequisites(logical_lines: &[String]) -> Result<Vec<String>, String> {
    for line in logical_lines {
        if !line.starts_with('\t')
            && let Some(rest) = line.strip_prefix("check:")
        {
            return Ok(rest.split_whitespace().map(str::to_string).collect());
        }
    }
    Err("no `check:` target found in Makefile".to_string())
}

/// Collect `target`'s own recipe lines (the tab-indented lines immediately following its
/// `target:` definition line) from `logical_lines`.
fn target_recipe_lines<'a>(logical_lines: &'a [String], target: &str) -> Vec<&'a str> {
    let target_prefix = format!("{target}:");
    let mut lines = Vec::new();
    let mut found = false;
    for line in logical_lines {
        if !found {
            if !line.starts_with('\t') && line.starts_with(&target_prefix) {
                found = true;
            }
            continue;
        }
        if line.starts_with('\t') {
            lines.push(line.as_str());
        } else if line.trim().is_empty() {
            continue;
        } else {
            break;
        }
    }
    lines
}

/// Apply steps 2-6 of the extraction rule to one already-continuation-joined recipe line,
/// returning the external program it names, or `None` when the line names no external
/// program (a shell guard keyword, `cargo`, or a `/bin/sh <scripts/ path>` invocation).
///
/// Examines only the FIRST token of the line: a compound line such as `cargo x && jq y` or
/// `python3 a.py | jq` would hide its second program. This is faithful to spec step 5, which
/// is itself stated against "the recipe line's first remaining token" — not a divergence this
/// leg introduces, but worth naming here since nothing else in the file says so.
fn program_from_recipe_line(line: &str) -> Option<String> {
    let stripped = line.trim_start_matches('\t');
    let stripped = stripped.trim_start_matches(['@', '-']);
    let tokens = tokenize_quote_aware(stripped);

    let mut i = 0;
    loop {
        if i >= tokens.len() {
            return None;
        }
        if is_assignment_token(&tokens[i]) {
            i += 1;
            continue;
        }
        if tokens[i] == "env" {
            i += 1;
            loop {
                if i >= tokens.len() {
                    break;
                }
                if is_assignment_token(&tokens[i]) {
                    i += 1;
                } else if let Some(flag) = tokens[i].strip_prefix('-') {
                    let takes_arg = matches!(flag, "u" | "C" | "S" | "P");
                    i += 1;
                    if takes_arg {
                        i += 1;
                    }
                } else {
                    break;
                }
            }
            continue;
        }
        break;
    }

    let candidate = tokens.get(i)?;
    if candidate == "cargo" {
        return None;
    }
    if candidate == "/bin/sh" || candidate == "sh" {
        let next_is_script = tokens
            .get(i + 1)
            .is_some_and(|next| next.starts_with("scripts/"));
        if next_is_script {
            return None;
        }
    }
    if SHELL_KEYWORDS.contains(&candidate.as_str()) {
        return None;
    }
    Some(candidate.clone())
}

/// Extract the set of external programs `make check`'s path invokes, by following `check:`'s
/// prerequisite targets' own recipes and applying the six-step extraction rule to each recipe
/// line. `Err` both when the `Makefile` cannot be followed at all (no `check:` target, or no
/// prerequisites) and when the extraction runs to completion but yields an empty set — a
/// broken extractor must fail loudly, never pass because it found nothing to check.
fn check_programs(makefile: &str) -> Result<BTreeSet<String>, String> {
    let logical_lines = join_continuations(makefile);
    let prerequisites = check_prerequisites(&logical_lines)?;
    if prerequisites.is_empty() {
        return Err("check:'s target names no prerequisites".to_string());
    }

    let mut programs = BTreeSet::new();
    for target in &prerequisites {
        for recipe_line in target_recipe_lines(&logical_lines, target) {
            if let Some(program) = program_from_recipe_line(recipe_line) {
                programs.insert(program);
            }
        }
    }

    if programs.is_empty() {
        return Err(
            "the extraction rule found no external program in check:'s prerequisite recipes"
                .to_string(),
        );
    }
    Ok(programs)
}

/// Whether any of `gate_scripts`' contents names `word` as a whole word — the second sub-leg,
/// which requires a program named anywhere under `scripts/gates/` to be documented on the same
/// terms as one the `Makefile` itself names, even when no `Makefile` line names it.
fn gate_scripts_require(gate_scripts: &[String], word: &str) -> bool {
    gate_scripts
        .iter()
        .any(|content| program_mentioned(content, word))
}

#[test]
fn gate_programs_are_documented() {
    let makefile = read_doc(&manifest_dir().join("Makefile")).expect("read Makefile");
    let programs = check_programs(&makefile).expect("extract programs from Makefile's check: path");

    let readme_md = read_doc(&manifest_dir().join("README.md")).expect("read README.md");
    let agents_md = read_doc(&manifest_dir().join("AGENTS.md")).expect("read AGENTS.md");
    let readme_section =
        section(&readme_md, "## Development").expect("find README.md's Development section");
    let agents_section =
        section(&agents_md, "## Environment").expect("find AGENTS.md's Environment section");

    let mut undocumented = Vec::new();
    for program in &programs {
        let mut missing = Vec::new();
        if !program_mentioned(readme_section, program) {
            missing.push("README.md's Development section");
        }
        if !program_mentioned(agents_section, program) {
            missing.push("AGENTS.md's Environment section");
        }
        if !missing.is_empty() {
            undocumented.push(format!("{program} missing from {missing:?}"));
        }
    }
    assert!(
        undocumented.is_empty(),
        "programs invoked by make check's path are undocumented: {undocumented:?}"
    );
}

#[test]
fn gate_script_interpreters_are_documented() {
    let gates_dir = manifest_dir().join("scripts/gates");
    let mut gate_scripts = Vec::new();
    for entry in std::fs::read_dir(&gates_dir).expect("read scripts/gates directory") {
        let entry = entry.expect("read scripts/gates directory entry");
        let path = entry.path();
        if path.is_file() {
            gate_scripts.push(read_doc(&path).unwrap_or_else(|e| panic!("{e}")));
        }
    }
    assert!(
        !gate_scripts.is_empty(),
        "scripts/gates must contain at least one file to scan"
    );

    if !gate_scripts_require(&gate_scripts, "python3") {
        return;
    }

    let readme_md = read_doc(&manifest_dir().join("README.md")).expect("read README.md");
    let agents_md = read_doc(&manifest_dir().join("AGENTS.md")).expect("read AGENTS.md");
    let readme_section =
        section(&readme_md, "## Development").expect("find README.md's Development section");
    let agents_section =
        section(&agents_md, "## Environment").expect("find AGENTS.md's Environment section");

    let mut missing = Vec::new();
    if !program_mentioned(readme_section, "python3") {
        missing.push("README.md's Development section");
    }
    if !program_mentioned(agents_section, "python3") {
        missing.push("AGENTS.md's Environment section");
    }
    assert!(
        missing.is_empty(),
        "python3 is invoked under scripts/gates/ but not documented in: {missing:?}"
    );
}

#[test]
fn guard_block_yields_no_program() {
    // The real `lint:`/`coverage:` guard joins into one logical line whose only examined
    // token is `if`, so it alone proves the real shape. The lines after it individually drive
    // every other step-6 keyword as the first token of its own line, so no single alternative
    // in that list goes unchecked by an actual assertion (a `grep -c` over several keywords at
    // once would hide exactly this).
    let makefile = "\
check: lint

lint:
\t@if ! cargo clippy --version >/dev/null 2>&1; then \\
\t\techo \"error: clippy not found\" 1>&2; \\
\t\texit 1; \\
\tfi
\tcargo clippy --all-targets --all-features -- -D warnings
\tif something
\tthen something
\telse something
\telif something
\tfi something
\tfor something
\tdo something
\tdone
\twhile something
\tcase something
\tesac
\techo something
\texit 1
\ttest -f foo
\t[ -f foo ]
\t: noop
\tcd /tmp
\tset -e
\ttrue
\tfalse
\tpython3 scripts/gates/gate-mech1.py
";
    let programs = check_programs(makefile).expect("extract from synthetic guard-block Makefile");
    let expected: BTreeSet<String> = ["python3".to_string()].into_iter().collect();
    assert_eq!(
        programs, expected,
        "a shell guard block's own keywords and `cargo` must never be reported as programs"
    );
    for keyword in [
        "if", "then", "else", "elif", "fi", "for", "do", "done", "while", "case", "esac", "echo",
        "exit", "test", "[", ":", "cd", "set", "true", "false", "cargo",
    ] {
        assert!(
            !programs.contains(keyword),
            "`{keyword}` must never be reported as a program: {programs:?}"
        );
    }
}

#[test]
fn quoted_assignment_is_one_token() {
    let makefile = "\
check: gates

gates:
\tenv -u GRAPH_WRITE /bin/sh scripts/gates/build-graph.sh
\tLAUNCH=src/open.rs ENTRY='pub fn run_from_env\\(' /bin/sh scripts/gates/launchseam.sh
\tpython3 scripts/gates/gate-mech1.py
";
    let programs =
        check_programs(makefile).expect("extract from synthetic quoted-assignment Makefile");
    let expected: BTreeSet<String> = ["python3".to_string()].into_iter().collect();
    assert_eq!(
        programs, expected,
        "a whitespace-split tokenizer would report `fn`, and a naive env-stripper would report \
         `GRAPH_WRITE`; the quote-aware, env-option-aware extractor must report neither"
    );
    for stray in ["fn", "GRAPH_WRITE", "env", "-u", "/bin/sh", "sh"] {
        assert!(
            !programs.contains(stray),
            "`{stray}` must never be reported as a program: {programs:?}"
        );
    }
}

#[test]
fn new_gate_program_is_caught() {
    let makefile = "\
check: gates

gates:
\t/bin/sh scripts/gates/deps.sh
\tjq -r '.foo' target/out.json
\tpython3 scripts/gates/gate-mech1.py
";
    let programs = check_programs(makefile).expect("extract from synthetic new-tool Makefile");
    assert!(
        programs.contains("jq"),
        "a future recipe line invoking an undocumented program must be caught: {programs:?}"
    );
}

#[test]
fn empty_program_set_fails() {
    let makefile = "\
check: gates

gates:
\t/bin/sh scripts/gates/deps.sh
\tcargo build
";
    let result = check_programs(makefile);
    assert!(
        result.is_err(),
        "an extraction yielding no external program must fail loudly, not pass vacuously"
    );
}

/// Locate the fenced ` ```toml ` block in `spec_md` that contains `id = "herdr-openspec"` —
/// the plugin manifest transcription — and return its inner text (the lines between the
/// fences). Anchored to that marker rather than to "the first ` ```toml ` fence", which would
/// silently pick up an unrelated fenced example. `Err` when no such fence exists: "the
/// transcription could not be located" is a distinct failure from "the transcription
/// disagrees", per `specs/doc-conformance/spec.md` -> "The transcription is absent".
fn manifest_block(spec_md: &str) -> Result<&str, String> {
    const OPEN_FENCE: &str = "```toml";
    const CLOSE_FENCE: &str = "\n```";
    const MARKER: &str = "id = \"herdr-openspec\"";

    let mut search_from = 0;
    while let Some(rel_open) = spec_md[search_from..].find(OPEN_FENCE) {
        let fence_start = search_from + rel_open;
        let after_fence_line = spec_md[fence_start..]
            .find('\n')
            .map(|i| fence_start + i + 1)
            .unwrap_or(spec_md.len());

        let Some(rel_close) = spec_md[after_fence_line..].find(CLOSE_FENCE) else {
            // Unterminated fence: nothing more to find from here.
            break;
        };
        let body_end = after_fence_line + rel_close;
        let body = &spec_md[after_fence_line..body_end];

        if body.contains(MARKER) {
            return Ok(body);
        }
        search_from = body_end + CLOSE_FENCE.len();
    }
    Err(format!(
        "no fenced ```toml block in the document contains {MARKER:?} (the plugin manifest \
         transcription)"
    ))
}

/// The ordered sequence of `[[header]]` lines in `source`, as text — e.g. `["[[build]]",
/// "[[panes]]", "[[panes]]", "[[actions]]", "[[actions]]"]`. A parsed TOML `Value` is keyed,
/// so nothing in a value comparison can see this ordering; it is why table order needs its
/// own, purely textual, comparison.
fn table_header_order(source: &str) -> Vec<String> {
    source
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("[[") && line.ends_with("]]"))
        .map(str::to_string)
        .collect()
}

#[test]
fn manifest_block_absent() {
    let spec_md = "Some prose.\n\n```toml\nid = \"some-other-plugin\"\n```\n\nMore prose.\n";
    let result = manifest_block(spec_md);
    assert!(
        result.is_err(),
        "a fenced toml block that does not carry id = \"herdr-openspec\" must not be mistaken \
         for the transcription: {result:?}"
    );
}

#[test]
fn manifest_block_order() {
    let a = "\
[[build]]
command = [\"x\"]

[[actions]]
id = \"open\"
";
    let b = "\
[[actions]]
id = \"open\"

[[build]]
command = [\"x\"]
";
    let value_a: toml::Table = a.parse().expect("parse synthetic source a");
    let value_b: toml::Table = b.parse().expect("parse synthetic source b");
    assert_eq!(
        value_a, value_b,
        "these two synthetic sources must parse equal for this test to prove anything about \
         order alone"
    );

    let order_a = table_header_order(a);
    let order_b = table_header_order(b);
    assert_ne!(
        order_a, order_b,
        "two sources presenting [[…]] tables in a different order must be distinguishable: \
         {order_a:?} vs {order_b:?}"
    );
}

#[test]
fn spec_manifest_block_matches() {
    let spec_md = read_doc(&manifest_dir().join("SPEC.md")).expect("read SPEC.md");
    let manifest_text =
        read_doc(&manifest_dir().join("herdr-plugin.toml")).expect("read herdr-plugin.toml");

    let block = manifest_block(&spec_md).expect("locate SPEC.md's manifest transcription");
    let spec_value: toml::Table = block
        .parse()
        .expect("parse SPEC.md's transcription as TOML");
    let manifest_value: toml::Table = manifest_text
        .parse()
        .expect("parse herdr-plugin.toml as TOML");

    assert_eq!(
        spec_value, manifest_value,
        "SPEC.md's transcription of the plugin manifest does not match herdr-plugin.toml"
    );
}

#[test]
fn spec_manifest_block_order_matches() {
    let spec_md = read_doc(&manifest_dir().join("SPEC.md")).expect("read SPEC.md");
    let manifest_text =
        read_doc(&manifest_dir().join("herdr-plugin.toml")).expect("read herdr-plugin.toml");

    let block = manifest_block(&spec_md).expect("locate SPEC.md's manifest transcription");
    let spec_order = table_header_order(block);
    let manifest_order = table_header_order(&manifest_text);

    assert_eq!(
        spec_order, manifest_order,
        "SPEC.md's block orders [[…]] tables as {spec_order:?}, herdr-plugin.toml as \
         {manifest_order:?}"
    );
}

// --- Injected project context: fixture claim + gate-tier coverage ---------------------------
//
// `openspec/config.yaml`'s `context` block is injected verbatim into every OpenSpec agent's
// prompt in this repository (design.md -> Decision 6). It is read as text, never parsed as
// YAML: the block is sliced between the `context: |` marker and the next top-level key (a
// line matching `^[a-z_]+:` at column 0), failing loudly when that boundary cannot be found so
// this leg never silently searches `rules:` or `operations:` too.

/// Whether `line` opens a new top-level YAML key: no leading whitespace, and its text before
/// the first `:` is one or more lowercase ASCII letters or underscores.
fn is_top_level_key_line(line: &str) -> bool {
    if line.starts_with(' ') || line.starts_with('\t') || line.is_empty() {
        return false;
    }
    match line.split_once(':') {
        Some((key, _)) => {
            !key.is_empty() && key.chars().all(|c| c.is_ascii_lowercase() || c == '_')
        }
        None => false,
    }
}

/// Slice the `context: |` block scalar out of `config_yaml`'s text: everything between the
/// `context: |` marker and the next top-level key. `Err` names the missing marker rather than
/// falling back to searching the whole file.
fn context_block(config_yaml: &str) -> Result<&str, String> {
    let marker = "context: |";
    let marker_at = config_yaml
        .find(marker)
        .ok_or_else(|| "no `context: |` block found in openspec/config.yaml".to_string())?;
    let after_marker = marker_at + marker.len();

    let mut offset = after_marker;
    for line in config_yaml[after_marker..].lines() {
        if is_top_level_key_line(line) {
            return Ok(&config_yaml[after_marker..offset]);
        }
        offset += line.len() + 1;
    }
    Ok(&config_yaml[after_marker..])
}

/// Recursively collect the basenames of every directory under `root`. Real-filesystem edge
/// for the fixture-repository check below; absent or unreadable directories yield an empty
/// list rather than a panic.
fn collect_dir_basenames(root: &std::path::Path) -> Vec<String> {
    let mut names = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    names.push(name.to_string());
                }
                stack.push(path);
            }
        }
    }
    names
}

/// A fixture repository is a directory under `tests/fixtures/` containing an `openspec/`
/// subdirectory — i.e. some directory named `openspec` appears anywhere in the recursive
/// listing of basenames under `tests/fixtures/`.
fn has_fixture_repository(dir_basenames: &[String]) -> bool {
    dir_basenames.iter().any(|name| name == "openspec")
}

/// Whether `phrase` occurs in `text` as a whole phrase — the character immediately following a
/// match is not alphanumeric, `-`, or `_` — so `fixture repositories` does not fire on some
/// future `fixture repositories-ish` coinage, and `make gates` does not fire on `make
/// gates-full`.
fn phrase_present(text: &str, phrase: &str) -> bool {
    let mut search_from = 0;
    while let Some(pos) = text[search_from..].find(phrase) {
        let abs = search_from + pos;
        let end = abs + phrase.len();
        let boundary_ok = text
            .as_bytes()
            .get(end)
            .is_none_or(|&b| !(b.is_ascii_alphanumeric() || b == b'-' || b == b'_'));
        if boundary_ok {
            return true;
        }
        search_from = abs + 1;
    }
    false
}

/// Clause 1: the block SHALL NOT claim checked-in fixture repositories while none exist.
/// Returns a failure message naming the phrase when the claim is false, `None` when the claim
/// is either true or not made at all.
fn fixture_claim_violation(context_text: &str, fixture_repo_exists: bool) -> Option<String> {
    if phrase_present(context_text, "fixture repositories") && !fixture_repo_exists {
        Some(
            "openspec/config.yaml's context block claims \"fixture repositories\" but no \
             directory under tests/fixtures/ contains an openspec/ subdirectory"
                .to_string(),
        )
    } else {
        None
    }
}

/// Clause 2: for every prerequisite target of the `Makefile`'s `check:` target, the context
/// block SHALL name either `make <target>` or that target's own recipe command — and a target
/// whose recipe is more than one command line SHALL be satisfied only by `make <target>`.
/// Returns the prerequisite targets left unrepresented, in `check:`'s own order. `Err` when the
/// `Makefile` cannot be followed at all (reusing the same extraction rule as
/// `check_programs` above: continuation-joining, then per-target recipe lines).
fn unrepresented_check_targets(context_text: &str, makefile: &str) -> Result<Vec<String>, String> {
    let logical_lines = join_continuations(makefile);
    let prerequisites = check_prerequisites(&logical_lines)?;
    if prerequisites.is_empty() {
        return Err("check:'s target names no prerequisites".to_string());
    }

    let mut missing = Vec::new();
    for target in &prerequisites {
        let make_mention = format!("make {target}");
        let named_by_make = phrase_present(context_text, &make_mention);

        let recipe_lines = target_recipe_lines(&logical_lines, target);
        let named_by_own_command = recipe_lines.len() == 1
            && recipe_lines.first().is_some_and(|line| {
                let command = line.trim_start_matches('\t').trim();
                !command.is_empty() && context_text.contains(command)
            });

        if !(named_by_make || named_by_own_command) {
            missing.push(target.clone());
        }
    }
    Ok(missing)
}

#[test]
fn context_block_absent() {
    let config_yaml = "schema: tdd\n\nrules:\n  proposal:\n    - a\n";
    assert!(
        context_block(config_yaml).is_err(),
        "a config with no `context: |` marker must fail loudly rather than pretend an empty \
         block"
    );
}

#[test]
fn context_slice_excludes_content_below_the_block() {
    // The only occurrence of `make gates` sits inside `rules:`, below the block. If the
    // slicer's boundary is wrong (e.g. it returns the whole file), clause 2 would wrongly
    // see it and pass — proving the slice is non-vacuous requires this to still fail.
    let config_yaml = "schema: tdd\n\ncontext: |\n  Some context text naming no gate tier.\n\n\
rules:\n  note: 'run make gates before anything else'\n";

    let context_text = context_block(config_yaml).expect("slice context block");
    assert!(
        !context_text.contains("make gates"),
        "the slice leaked past the block boundary into `rules:`: {context_text:?}"
    );

    let makefile = "check: gates\n\ngates:\n\tstep one\n\tstep two\n";
    let missing =
        unrepresented_check_targets(context_text, makefile).expect("extract check: targets");
    assert_eq!(
        missing,
        vec!["gates".to_string()],
        "clause 2 must still fail naming `gates` when `make gates` sits only below the block"
    );
}

#[test]
fn fixture_claim_flags_when_no_repo_exists() {
    let context_text = "fixture repositories under `tests/fixtures/`.";
    assert!(
        fixture_claim_violation(context_text, false).is_some(),
        "the phrase is present and no fixture repository exists — must be flagged"
    );
}

#[test]
fn fixture_repo_makes_claim_true() {
    let context_text = "fixture repositories under `tests/fixtures/`.";
    assert!(
        fixture_claim_violation(context_text, true).is_none(),
        "once a fixture repository exists, the same phrase is no longer a false claim"
    );
}

#[test]
fn fixture_claim_absent_is_not_a_violation() {
    let context_text = "run-time ScratchDir trees and include_str! corpora.";
    assert!(
        fixture_claim_violation(context_text, false).is_none(),
        "a block that never makes the claim has nothing to be false"
    );
}

#[test]
fn missing_multiline_target_is_named() {
    let makefile = "check: fmt-check gates\n\nfmt-check:\n\tcargo fmt --all -- --check\n\n\
gates:\n\tstep one\n\tstep two\n";
    let context_text = "commands: `cargo fmt --all -- --check`.";
    let missing =
        unrepresented_check_targets(context_text, makefile).expect("extract check: targets");
    assert_eq!(
        missing,
        vec!["gates".to_string()],
        "a multi-command-line target with no `make gates` mention must be named, even though \
         `fmt-check` is satisfied by its own recipe command"
    );
}

#[test]
fn single_line_recipe_satisfied_by_own_command_text() {
    let makefile = "check: fmt-check\n\nfmt-check:\n\tcargo fmt --all -- --check\n";
    let context_text = "run `cargo fmt --all -- --check` first";
    let missing =
        unrepresented_check_targets(context_text, makefile).expect("extract check: targets");
    assert!(
        missing.is_empty(),
        "a single-command-line target's own recipe command must satisfy clause 2: {missing:?}"
    );
}

#[test]
fn single_line_recipe_not_satisfied_by_partial_mention() {
    let makefile = "check: fmt-check\n\nfmt-check:\n\tcargo fmt --all -- --check\n";
    let context_text = "run cargo fmt sometimes";
    let missing =
        unrepresented_check_targets(context_text, makefile).expect("extract check: targets");
    assert_eq!(
        missing,
        vec!["fmt-check".to_string()],
        "a partial mention of the recipe command must not satisfy clause 2"
    );
}

#[test]
fn context_fixture_claim() {
    let config_yaml =
        read_doc(&manifest_dir().join("openspec/config.yaml")).expect("read config.yaml");
    let context_text = context_block(&config_yaml).expect("slice context block");

    let basenames = collect_dir_basenames(&manifest_dir().join("tests/fixtures"));
    let fixture_repo_exists = has_fixture_repository(&basenames);

    if let Some(message) = fixture_claim_violation(context_text, fixture_repo_exists) {
        panic!("{message}");
    }
}

#[test]
fn context_names_every_gate_tier() {
    let config_yaml =
        read_doc(&manifest_dir().join("openspec/config.yaml")).expect("read config.yaml");
    let context_text = context_block(&config_yaml).expect("slice context block");
    let makefile = read_doc(&manifest_dir().join("Makefile")).expect("read Makefile");

    let missing = unrepresented_check_targets(context_text, &makefile)
        .expect("extract check:'s prerequisite targets from the Makefile");
    assert!(
        missing.is_empty(),
        "openspec/config.yaml's context block does not represent check: prerequisite \
         target(s) {missing:?} — each needs `make <target>`, or (for a single-command-line \
         recipe only) that command verbatim"
    );
}

// --- Worker-thread count leg ------------------------------------------------------------
//
// See `specs/doc-conformance/spec.md` -> "The documented worker-thread count equals the
// crate's production thread sites". The rule below is NOT the one that spec originally
// stated. `seam-resilience` (commit 137d21b) added two per-invocation pipe-drain threads to
// `src/cli.rs`'s production slice, above its first `#[cfg(test)]`. The original rule — "a
// file's production slice names `std::thread::spawn`" — now computes four files
// (`agents.rs`, `cli.rs`, `launch.rs`, `refresh.rs`), not the true three. `src/cli.rs`'s two
// threads are per-invocation pipe pumps joined by `JoinHandle::join()`; they answer no one,
// so they are not worker threads.
//
// The corrected second site: a file's production slice names BOTH `std::thread::spawn` AND
// `mpsc`. This is not invented for this test — it is `scripts/gates/noblock.sh`'s own Guard
// A, its positive control for leg 1: `prod src/refresh.rs | grep -qE 'mpsc'` alongside
// `grep -qE 'thread::spawn'`, on the reasoning that a worker thread answers over a channel.
// Measured at HEAD: `agents.rs`, `launch.rs`, `refresh.rs` name both and are counted;
// `cli.rs` names `thread::spawn` but no `mpsc` and is excluded; `watch.rs` names `mpsc` but
// spawns no thread of its own (`notify` spawns it) and is excluded from the other direction.
//
// LIMIT, STATED PLAINLY: this discriminator does not generalise to every possible worker
// thread. A future worker thread that answers over something other than `mpsc` — a
// `Mutex`/`Condvar` pair, or a channel type from a crate this crate does not currently
// depend on — would not be counted by this rule. That is acceptable today only because the
// crate has six dependencies and none of them provides a channel type. `NOBLOCK` leg 3
// constrains the four NAMED seam modules' (`watch`, `refresh`, `agents`, `launch`) own
// blocking behaviour — it cannot see a fifth worker module anywhere, named or not, so this
// leg's `mpsc` requirement is the ONLY thing standing between a new worker thread and a
// silently stale count. A reader who adds such a thread must find this sentence rather than
// rediscover the gap by tracing a stale count back through git blame.
//
// `thread::spawn` is matched ANCHORED per line — no `/` character anywhere before it on that
// line — the same rule `NOBLOCK`'s own thread::spawn checks use (`^[^/]*thread::spawn`), so
// a doc comment reading "the worker is started by a single `thread::spawn`" does not count.
// `mpsc` is matched UNANCHORED, a plain whole-file substring, deliberately: that is exactly
// Guard A's own shape (`grep -qE 'mpsc'`, no anchor), reused rather than tightened, so this
// leg and that gate can never disagree about which files carry a channel.

/// Recursively collect every `.rs` file under `root`, paired with its content. The returned
/// name is the file's path RELATIVE TO `root`, joined with `/` regardless of platform, so a
/// file at `root/newdir/worker.rs` is reported as `newdir/worker.rs` rather than colliding
/// with a same-named `root/worker.rs` in a failure message. `src/` is walked in full — the
/// worker-thread claim this leg checks is about the crate's whole production tree, not only
/// `src/`'s top level, and a new worker module under a new subdirectory (e.g. `src/ui/`, or
/// any future `src/<other>/`) must be just as visible to it as one at the top level.
fn collect_rs_files(root: &std::path::Path) -> Vec<(String, String)> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace(std::path::MAIN_SEPARATOR, "/");
                let content = read_doc(&path).unwrap_or_else(|e| panic!("{e}"));
                files.push((rel, content));
            }
        }
    }
    files
}

/// The text of `src` before its first line equal to `#[cfg(test)]` — the same cut
/// `scripts/gates/noblock.sh`'s `prod()` uses. A file with no such line is entirely
/// production.
fn production_slice(src: &str) -> &str {
    let mut offset = 0;
    for line in src.lines() {
        if line == "#[cfg(test)]" {
            return &src[..offset];
        }
        offset += line.len() + 1;
    }
    src
}

/// Whether `prod`'s production slice names `thread::spawn` on some line with no `/`
/// character before it on that line — excluding a `//`, `///`, or `//!` comment mention,
/// including a module doc comment.
fn spawns_thread_in_production(prod: &str) -> bool {
    prod.lines().any(|line| match line.find("thread::spawn") {
        Some(idx) => !line[..idx].contains('/'),
        None => false,
    })
}

/// Whether `prod` names `mpsc` anywhere at all — unanchored, comments included. Deliberately
/// the same shape as `noblock.sh`'s Guard A (`grep -qE 'mpsc'`), not the stricter anchored
/// form used for `thread::spawn` above: see the leg header for why the two are asymmetric.
fn names_mpsc(prod: &str) -> bool {
    prod.contains("mpsc")
}

/// Whether `src`'s production slice makes it a worker-thread module under the corrected
/// rule: both `thread::spawn` (anchored) and `mpsc` (unanchored) in the production slice.
fn is_worker_thread_source(src: &str) -> bool {
    let prod = production_slice(src);
    spawns_thread_in_production(prod) && names_mpsc(prod)
}

/// The names of every `(name, content)` pair whose content is a worker-thread source under
/// `is_worker_thread_source`, as a `BTreeSet` so a failure names every counted file, sorted,
/// rather than only a count.
fn worker_thread_files<'a>(
    sources: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> BTreeSet<String> {
    sources
        .into_iter()
        .filter(|(_, content)| is_worker_thread_source(content))
        .map(|(name, _)| name.to_string())
        .collect()
}

/// The number words the claim parser accepts, `one` through `six`, in ascending order.
const WORKER_COUNT_WORDS: [(&str, usize); 6] = [
    ("one", 1),
    ("two", 2),
    ("three", 3),
    ("four", 4),
    ("five", 5),
    ("six", 6),
];

/// Parse every occurrence of `crate's <number-word> worker threads` in `text`, where the
/// number word may optionally be wrapped in `**` emphasis (the phrase itself never is —
/// `design.md` -> Decision 3 measured that at HEAD the phrase carries no markup at all).
/// Matches the PLURAL `worker threads` only: `SPEC.md`'s singular ordinal phrasing ("the
/// crate's third worker thread") has no trailing `s` and so never matches this marker.
///
/// `Err` when no occurrence exists ("the claim could not be located" — a distinct failure
/// from "the count disagrees") and when two or more occurrences state different numbers (a
/// contradiction is never silently resolved to one of them). `Ok` with the shared value
/// when every occurrence agrees.
fn worker_thread_claim(text: &str) -> Result<usize, String> {
    const MARKER: &str = "worker threads";
    let mut found = Vec::new();
    let mut search_from = 0;
    while let Some(rel) = text[search_from..].find(MARKER) {
        let idx = search_from + rel;
        let before = text[..idx].trim_end();
        let before = before.strip_suffix("**").unwrap_or(before);
        for (word, value) in WORKER_COUNT_WORDS {
            if let Some(rest) = before.strip_suffix(word) {
                let rest = rest.strip_suffix("**").unwrap_or(rest);
                if rest.trim_end().ends_with("crate's") {
                    found.push(value);
                    break;
                }
            }
        }
        search_from = idx + MARKER.len();
    }

    match found.as_slice() {
        [] => {
            Err("no occurrence of \"crate's <number-word> worker threads\" was found".to_string())
        }
        [only, rest @ ..] if rest.iter().all(|v| v == only) => Ok(*only),
        multiple => Err(format!(
            "conflicting worker-thread counts found in the same document: {multiple:?}"
        )),
    }
}

#[test]
fn production_slice_cuts_before_cfg_test() {
    let src = "fn a() {}\n\n#[cfg(test)]\nmod tests {\n    fn b() {}\n}\n";
    assert_eq!(production_slice(src), "fn a() {}\n\n");
}

#[test]
fn production_slice_whole_file_when_no_cfg_test() {
    let src = "fn a() {}\nfn b() {}\n";
    assert_eq!(production_slice(src), src);
}

#[test]
fn thread_spawn_in_comment_not_counted() {
    let prod = "// the worker is started by a single thread::spawn in start\nfn start() {}\n";
    assert!(
        !spawns_thread_in_production(prod),
        "a comment mentioning thread::spawn must not count, the same rule NOBLOCK applies"
    );
}

#[test]
fn thread_spawn_in_code_is_counted() {
    let prod = "fn start() {\n    thread::spawn(move || {});\n}\n";
    assert!(spawns_thread_in_production(prod));
}

#[test]
fn mpsc_in_comment_counts_like_noblock_guard_a() {
    // Guard A's own shape is unanchored (`grep -qE 'mpsc'`), so a doc-comment mention of
    // `mpsc` counts here exactly as it would count for that gate — deliberately, not an
    // oversight; see the leg header for the asymmetry with `thread::spawn`.
    let prod = "//! answers over an mpsc channel\nfn start() {}\n";
    assert!(names_mpsc(prod));
}

#[test]
fn cli_rs_excluded_without_mpsc() {
    // The load-bearing exclusion: two production thread::spawn sites, no mpsc.
    let src = "fn drain_stdout() {\n    thread::spawn(move || {});\n}\n\
               fn drain_stderr() {\n    thread::spawn(move || {});\n}\n";
    assert!(
        !is_worker_thread_source(src),
        "a production slice with thread::spawn but no mpsc must not count as a worker module"
    );
}

#[test]
fn watch_rs_excluded_without_thread_spawn() {
    // notify spawns watch.rs's background thread, not watch.rs itself: mpsc with no spawn.
    let src = "use std::sync::mpsc;\nfn start() -> mpsc::Receiver<()> { todo!() }\n";
    assert!(
        !is_worker_thread_source(src),
        "a production slice with mpsc but no thread::spawn of its own must not count"
    );
}

#[test]
fn worker_claim_absent() {
    let text = "This document never states the worker-thread count in the bound form.";
    let result = worker_thread_claim(text);
    assert!(
        result.is_err(),
        "an absent claim must Err, never pass vacuously"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("could not be located") || err.contains("no occurrence"),
        "the error should say the claim could not be located: {err}"
    );
}

#[test]
fn worker_claim_singular_ordinal_is_not_matched() {
    // SPEC.md line 886's real phrasing: singular, an ordinal, not the bound plural phrase.
    let text = "and the worker — the crate's third worker thread, confined to this module";
    assert!(
        worker_thread_claim(text).is_err(),
        "the singular ordinal phrase must not satisfy the plural-phrase parser"
    );
}

#[test]
fn worker_claim_unemphasised_matches() {
    assert_eq!(
        worker_thread_claim("one of the crate's two worker threads, tested through"),
        Ok(2)
    );
}

#[test]
fn worker_claim_emphasised_matches() {
    assert_eq!(
        worker_thread_claim("one of the crate's **three** worker threads, tested through"),
        Ok(3)
    );
}

#[test]
fn worker_claim_disagreement_is_a_contradiction() {
    let text = "first it says crate's two worker threads, then crate's **three** worker \
                threads";
    let result = worker_thread_claim(text);
    assert!(
        result.is_err(),
        "two disagreeing occurrences must be reported as a contradiction, not resolved to \
         either value: {result:?}"
    );
}

#[test]
fn worker_count_stale() {
    // Synthetic sources computing three, against a documented claim of two — the "documented
    // count is stale" scenario, stated without touching the real tree.
    let sources: Vec<(&str, &str)> = vec![
        (
            "agents.rs",
            "use std::sync::mpsc;\nfn start() { thread::spawn(move || {}); }\n",
        ),
        (
            "launch.rs",
            "use std::sync::mpsc;\nfn start() { thread::spawn(move || {}); }\n",
        ),
        (
            "refresh.rs",
            "use std::sync::mpsc;\nfn start() { thread::spawn(move || {}); }\n",
        ),
        ("cli.rs", "fn drain() { thread::spawn(move || {}); }\n"),
        (
            "watch.rs",
            "use std::sync::mpsc;\nfn poll() -> mpsc::Receiver<()> { todo!() }\n",
        ),
    ];
    let counted = worker_thread_files(sources);
    let expected: BTreeSet<String> = ["agents.rs", "launch.rs", "refresh.rs"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(counted, expected);

    let documented = worker_thread_claim("crate's two worker threads").expect("parse claim");
    assert_ne!(
        documented,
        counted.len(),
        "documented {documented} must disagree with computed {} ({counted:?}) for this to \
         prove the stale-count scenario",
        counted.len()
    );
}

#[test]
fn worker_count_grows() {
    // A fifth synthetic module whose production slice names both thread::spawn and mpsc:
    // computed grows from three to four while the document still says three.
    let sources: Vec<(&str, &str)> = vec![
        (
            "agents.rs",
            "use std::sync::mpsc;\nfn start() { thread::spawn(move || {}); }\n",
        ),
        (
            "launch.rs",
            "use std::sync::mpsc;\nfn start() { thread::spawn(move || {}); }\n",
        ),
        (
            "refresh.rs",
            "use std::sync::mpsc;\nfn start() { thread::spawn(move || {}); }\n",
        ),
        (
            "poll5.rs",
            "use std::sync::mpsc;\nfn start() { thread::spawn(move || {}); }\n",
        ),
    ];
    let counted = worker_thread_files(sources);
    assert_eq!(
        counted.len(),
        4,
        "the fifth planted module must be counted: {counted:?}"
    );

    let documented = worker_thread_claim("crate's three worker threads").expect("parse claim");
    assert_ne!(
        documented,
        counted.len(),
        "documented {documented} must disagree with computed {} once a fourth worker module \
         is added",
        counted.len()
    );
}

#[test]
fn collect_rs_files_walks_recursively() {
    let scratch = ScratchDir::new("collect-rs-files");
    let root = scratch.path();
    std::fs::write(root.join("top.rs"), "fn a() {}\n").expect("write top.rs");
    std::fs::create_dir_all(root.join("sub")).expect("create sub/");
    std::fs::write(root.join("sub/nested.rs"), "fn b() {}\n").expect("write sub/nested.rs");
    std::fs::write(root.join("notes.txt"), "not rust\n").expect("write notes.txt");

    let files = collect_rs_files(root);
    let names: BTreeSet<String> = files.iter().map(|(name, _)| name.clone()).collect();
    let expected: BTreeSet<String> = ["top.rs".to_string(), "sub/nested.rs".to_string()]
        .into_iter()
        .collect();
    assert_eq!(
        names, expected,
        "the walk must find both the top-level and the nested .rs file, using `/` as the \
         separator, and skip the non-.rs file: {names:?}"
    );
}

#[test]
fn worker_threads_match_sources() {
    let src_dir = manifest_dir().join("src");
    let sources = collect_rs_files(&src_dir);
    assert!(
        !sources.is_empty(),
        "src/ must contain at least one .rs file to scan"
    );

    let borrowed: Vec<(&str, &str)> = sources
        .iter()
        .map(|(name, content)| (name.as_str(), content.as_str()))
        .collect();
    let counted = worker_thread_files(borrowed);
    let computed = counted.len();

    let spec_md = read_doc(&manifest_dir().join("SPEC.md")).expect("read SPEC.md");
    let documented = worker_thread_claim(&spec_md)
        .unwrap_or_else(|e| panic!("SPEC.md's worker-thread claim could not be parsed: {e}"));

    assert_eq!(
        documented, computed,
        "SPEC.md documents {documented} worker thread(s) but {computed} file(s) under src/ \
         have a production slice naming both thread::spawn and mpsc: {counted:?}"
    );
}

// ---------------------------------------------------------------------------
// `mouse-input`: the documented mouse bindings, and the documented confined
// terminal-seam names.
// ---------------------------------------------------------------------------

/// Every backticked `Action::<Variant>` named in `SPEC.md` -> Keys' mouse table,
/// as the bare variant names.
///
/// The table is located by its own header row, `| Gesture | Action |`, inside the
/// `### Keys` section — never by position, and never by scanning the whole
/// document, whose key table names no `Action` at all. `Err` names what was
/// missing, so a deleted table fails loudly rather than comparing an empty set
/// against an empty set and passing vacuously.
///
/// The extraction rule is stated in
/// `openspec/changes/mouse-input/specs/mouse-input/spec.md`: **each row carries
/// its `Action` variant in backticks**. A row that names none contributes
/// nothing, which is why the header row and the separator row are harmless.
/// **Why this lenient parse survives beside [`documented_mouse_rows`].** The
/// two answer different questions and the difference is load-bearing. This one
/// wants the table's row-free **vocabulary**, and leg 1 exists to report a plain
/// vocabulary mismatch in its own terms *before* the stricter legs run
/// (`design.md` -> Decision 8). Folding it into a projection of the strict parse
/// would invert that order: a missing `Zone` token or an unlisted gesture phrase
/// would then surface as leg 1's failure, which is not what leg 1 is about. The
/// duplication that did exist — locating the table — is gone: both parses share
/// [`mouse_table_lines`], so there is one answer to "where is the table" and two
/// to "what does a row mean".
fn documented_mouse_actions(spec_md: &str) -> Result<BTreeSet<String>, String> {
    let names: BTreeSet<String> = mouse_table_lines(spec_md)?
        .iter()
        .flat_map(|line| backticked_action_variants(line))
        .collect();
    if names.is_empty() {
        return Err(
            "SPEC.md -> Keys' mouse table names no `Action::` variant in backticks - the \
             extraction rule this check depends on is not being followed"
                .to_string(),
        );
    }
    Ok(names)
}

/// Every `Action::<Variant>` named in `text`, as the bare variant names. Used on
/// one markdown table row (where the names are backticked, and the backticks
/// simply fall outside the match) and on `mouse_action`'s own body alike.
fn backticked_action_variants(text: &str) -> Vec<String> {
    qualified_names(text, "Action::")
}

/// Every `<prefix><Name>` in `text`, as the bare names — the file's one way of
/// lifting a typed name out of prose, now serving `Action::`, `Zone::`,
/// `Target::` and `SelectPhase::` alike rather than four near-copies of one
/// scan.
fn qualified_names(text: &str, prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(idx) = rest.find(prefix) {
        rest = &rest[idx + prefix.len()..];
        let end = rest
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .unwrap_or(rest.len());
        if end > 0 {
            out.push(rest[..end].to_string());
        }
        rest = &rest[end..];
    }
    out
}

/// The six `Zone` variant names, pinned and closed. A backticked `Zone` token
/// outside this list is a typo, and is reported as one rather than left to
/// surface two legs later as an unexplained vacuity
/// (`doc-conformance` -> "The documented set").
const ZONE_VARIANTS: [&str; 6] = [
    "ListRow",
    "List",
    "DetailTab",
    "DetailRow",
    "Detail",
    "Outside",
];

/// The token a row carries to say it describes the **overlay-open** pass, where
/// `mouse_action` returns before consulting `ui::layout::zone` at all and there
/// is no zone for the row to name (`design.md` -> Decision 6).
const OVERLAY_TOKEN: &str = "`help.open`";

/// The **closed** gesture vocabulary: a leading phrase of the Gesture cell
/// mapped to the `MouseEventKind` values it stands for (`design.md` ->
/// Decision 5). Pinned by length, and a phrase outside it is a hard error
/// naming the row — semantic parsing of the cell's English is not attempted,
/// because its failure mode is silent.
const MOUSE_GESTURES: [(&str, &[MouseEventKind]); 5] = [
    ("Wheel down", &[MouseEventKind::ScrollDown]),
    ("Wheel up", &[MouseEventKind::ScrollUp]),
    ("Left click", &[MouseEventKind::Down(MouseButton::Left)]),
    ("Left drag", &[MouseEventKind::Drag(MouseButton::Left)]),
    ("Anything else", &MOUSE_KINDS),
];

/// One row of `SPEC.md` -> Keys' mouse table, on the same four axes a [`Claim`]
/// carries, plus its own source text so a failure can quote the row back.
///
/// A row's claims are the **cross product** of its gestures, its zones and its
/// outcomes. The catch-all is the exception: it names no zone, its only outcome
/// is `Ignore`, and it claims the remainder rather than a cross product.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MouseRow {
    /// The row's own line, quoted back in a failure message.
    text: String,
    /// Axis 1: the gesture kinds the row's Gesture cell resolves to, as
    /// [`kind_name`] spells them.
    kinds: Vec<&'static str>,
    /// Axis 2: whether the row describes the overlay-open pass.
    help_open: bool,
    /// Axis 3: the `Zone` variants the row covers. Empty only for the catch-all
    /// and for an overlay-open row.
    zones: Vec<&'static str>,
    /// Axis 4: the outcomes, in `Click(Change)` form.
    outcomes: Vec<String>,
    /// Whether this is **the** catch-all: zone-less, overlay-agnostic, and
    /// `Ignore`-only.
    catch_all: bool,
}

/// The mouse table's own contiguous `|` lines, header and separator included.
///
/// The table is located by its own header row, `| Gesture | Action |`, inside
/// the `### Keys` section — never by position, and never by scanning the whole
/// document. `Err` names what was missing, so a deleted or gutted table fails
/// loudly rather than yielding an empty set that compares equal to another one.
fn mouse_table_lines(spec_md: &str) -> Result<Vec<&str>, String> {
    let keys = section(spec_md, "### Keys")?;
    let start = keys.find("| Gesture | Action |").ok_or_else(|| {
        "SPEC.md -> Keys holds no mouse table (no `| Gesture | Action |` header row)".to_string()
    })?;
    let lines: Vec<&str> = keys[start..]
        .lines()
        .take_while(|line| line.starts_with('|'))
        .collect();
    if lines.len() < 3 {
        return Err(format!(
            "SPEC.md -> Keys' mouse table has {} row(s) - a header, a separator, and at \
             least one binding are the minimum",
            lines.len()
        ));
    }
    Ok(lines)
}

/// One binding row, parsed onto the four claim axes.
fn parse_mouse_row(line: &str) -> Result<MouseRow, String> {
    let cells: Vec<&str> = line.split('|').collect();
    if cells.len() < 4 {
        return Err(format!(
            "SPEC.md -> Keys' mouse table row {line:?} does not carry a Gesture cell and an \
             Action cell"
        ));
    }
    let gesture = cells[1].trim();
    let kinds = MOUSE_GESTURES
        .iter()
        .find(|(phrase, _)| gesture.starts_with(phrase))
        .map(|(_, kinds)| kinds.iter().copied().map(kind_name).collect::<Vec<_>>())
        .ok_or_else(|| {
            let listed: Vec<&str> = MOUSE_GESTURES.iter().map(|(phrase, _)| *phrase).collect();
            format!(
                "SPEC.md -> Keys' mouse table row {line:?} opens with a gesture phrase the \
                 closed vocabulary does not list: {gesture:?}. The five listed phrases are \
                 {listed:?} - add the phrase to MOUSE_GESTURES or reword the row, never \
                 leave it unmatched, which would drop the row from both directions of the \
                 assertion"
            )
        })?;

    let mut zones: Vec<&'static str> = Vec::new();
    for token in qualified_names(line, "Zone::") {
        let variant = ZONE_VARIANTS
            .iter()
            .find(|known| **known == token)
            .ok_or_else(|| {
                format!(
                    "SPEC.md -> Keys' mouse table row {line:?} names `Zone::{token}`, which is \
                     no `Zone` variant - the six are {ZONE_VARIANTS:?}"
                )
            })?;
        if !zones.contains(variant) {
            zones.push(variant);
        }
    }

    let variants = backticked_action_variants(line);
    if variants.is_empty() {
        return Err(format!(
            "SPEC.md -> Keys' mouse table row {line:?} names no `Action::` variant in \
             backticks - the extraction rule this check depends on is not being followed"
        ));
    }
    let mut outcomes: Vec<String> = Vec::new();
    for variant in variants {
        let prefix = match variant.as_str() {
            "Click" => Some("Target::"),
            "Select" => Some("SelectPhase::"),
            _ => None,
        };
        match prefix {
            None => {
                if !outcomes.contains(&variant) {
                    outcomes.push(variant);
                }
            }
            Some(prefix) => {
                let payloads = qualified_names(line, prefix);
                if payloads.is_empty() {
                    return Err(format!(
                        "SPEC.md -> Keys' mouse table row {line:?} names `Action::{variant}` \
                         and no `{prefix}` constructor - one `{variant}` row is told from \
                         another only by the payload constructor it carries"
                    ));
                }
                for payload in payloads {
                    let outcome = format!("{variant}({payload})");
                    if !outcomes.contains(&outcome) {
                        outcomes.push(outcome);
                    }
                }
            }
        }
    }

    let help_open = line.contains(OVERLAY_TOKEN);
    if help_open && !zones.is_empty() {
        return Err(format!(
            "SPEC.md -> Keys' mouse table row {line:?} carries {OVERLAY_TOKEN} and names \
             {zones:?} as well - while the overlay is open `mouse_action` returns before \
             consulting `ui::layout::zone` at all, so there is no zone for such a row to \
             name, and its zones would be silently discarded"
        ));
    }
    let catch_all = zones.is_empty() && !help_open && outcomes == ["Ignore"];
    Ok(MouseRow {
        text: line.to_string(),
        kinds,
        help_open,
        zones,
        outcomes,
        catch_all,
    })
}

/// `SPEC.md` -> Keys' mouse table, as rows on the claim's own four axes.
///
/// Added **beside** [`documented_mouse_actions`] rather than replacing it: this
/// parse is strict where that one is lenient, and the two answer different
/// questions — leg 1 wants the row-free vocabulary of the whole table.
fn documented_mouse_rows(spec_md: &str) -> Result<Vec<MouseRow>, String> {
    let lines = mouse_table_lines(spec_md)?;
    let rows: Vec<MouseRow> = lines
        .into_iter()
        .skip(2)
        .map(parse_mouse_row)
        .collect::<Result<_, _>>()?;

    let catch_alls: Vec<&str> = rows
        .iter()
        .filter(|row| row.catch_all)
        .map(|row| row.text.as_str())
        .collect();
    if catch_alls.len() != 1 {
        return Err(format!(
            "SPEC.md -> Keys' mouse table must carry exactly one catch-all row - the one \
             naming no `Zone` and whose only outcome is `Action::Ignore` - and carries {}: \
             {catch_alls:?}",
            catch_alls.len()
        ));
    }

    for row in &rows {
        if row.zones.is_empty() && !row.catch_all && !row.help_open {
            return Err(format!(
                "SPEC.md -> Keys' mouse table row {:?} names no `Zone` - only the single \
                 `Ignore`-only catch-all, and a row naming the overlay-open state with \
                 {OVERLAY_TOKEN}, may omit one",
                row.text
            ));
        }
    }
    Ok(rows)
}

/// `mouse_action`'s own body, cut from `src/ui/driver.rs`'s **production slice**:
/// from the line beginning `pub fn mouse_action(` up to and including the next
/// line that is exactly `}` at column zero.
///
/// The production slice matters: the file's inline `#[cfg(test)]` module names
/// every `Action` variant this crate has, so a whole-file scan would compare the
/// documented set against the enum rather than against what the resolver
/// produces, and would pass on a resolver with swapped arms.
fn mouse_action_body(driver_rs: &str) -> Result<&str, String> {
    let prod = production_slice(driver_rs);
    let mut offset = 0usize;
    let mut start = None;
    for line in prod.lines() {
        if line.starts_with("pub fn mouse_action(") {
            start = Some(offset);
            break;
        }
        offset += line.len() + 1;
    }
    let start = start.ok_or_else(|| {
        "src/ui/driver.rs's production slice defines no `pub fn mouse_action(`".to_string()
    })?;
    let body = &prod[start..];
    // Up to and including the next line that is exactly `}` at column zero.
    let mut end = None;
    let mut at = 0usize;
    for line in body.lines() {
        if at > 0 && line == "}" {
            end = Some(at);
            break;
        }
        at += line.len() + 1;
    }
    let end = end
        .ok_or_else(|| "`pub fn mouse_action(` has no closing brace at column zero".to_string())?;
    Ok(&body[..end])
}

/// The six names `scripts/gates/noraw-grep.sh`'s `RAW_RE` searches for.
fn gate_raw_names(script: &str) -> Result<BTreeSet<String>, String> {
    let line = script
        .lines()
        .find(|l| l.starts_with("RAW_RE="))
        .ok_or_else(|| "scripts/gates/noraw-grep.sh defines no RAW_RE".to_string())?;
    let open = line.find('\'').ok_or_else(|| {
        "scripts/gates/noraw-grep.sh's RAW_RE is not a single-quoted literal".to_string()
    })?;
    let rest = &line[open + 1..];
    let close = rest
        .find('\'')
        .ok_or_else(|| "scripts/gates/noraw-grep.sh's RAW_RE is unterminated".to_string())?;
    Ok(rest[..close]
        .split('|')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

/// The confined terminal-seam function names `AGENTS.md` claims, read from the
/// parenthetical that immediately follows its own marker sentence.
///
/// Only backticked **identifiers** are taken, so a change name like
/// `mouse-input` mentioned nearby could never be read as a function name.
fn documented_seam_names(agents_md: &str) -> Result<BTreeSet<String>, String> {
    const MARKER: &str = "permitted to name a crossterm terminal-mode function";
    let idx = agents_md
        .find(MARKER)
        .ok_or_else(|| format!("AGENTS.md holds no {MARKER:?} sentence"))?;
    let rest = &agents_md[idx + MARKER.len()..];
    let open = rest
        .find('(')
        .ok_or_else(|| "AGENTS.md's terminal-seam rule names no parenthesised list".to_string())?;
    let close = rest[open..]
        .find(')')
        .ok_or_else(|| "AGENTS.md's terminal-seam parenthetical is unterminated".to_string())?;
    let list = &rest[open + 1..open + close];
    let names: BTreeSet<String> = list
        .split('`')
        .map(str::trim)
        .filter(|s| {
            !s.is_empty()
                && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && s.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        })
        .map(str::to_string)
        .collect();
    if names.is_empty() {
        return Err(
            "AGENTS.md's terminal-seam parenthetical names no backticked function".to_string(),
        );
    }
    Ok(names)
}

#[test]
fn mouse_bindings_match_spec_md() {
    let spec_md = read_doc(&manifest_dir().join("SPEC.md")).expect("read SPEC.md");
    let driver = read_doc(&manifest_dir().join("src/ui/driver.rs")).expect("read src/ui/driver.rs");

    // Leg 1, unchanged: the set of backticked `Action::` names on each side.
    // Kept as the **first** leg so a plain vocabulary mismatch is still reported
    // in its existing terms, before the stricter legs run on a table whose
    // vocabulary already agrees (`design.md` -> Decision 8).
    let documented = documented_mouse_actions(&spec_md).expect("SPEC.md -> Keys' mouse table");
    let body = mouse_action_body(&driver).expect("mouse_action's own body");
    let implemented: BTreeSet<String> = backticked_action_variants(body).into_iter().collect();

    assert_eq!(
        documented, implemented,
        "SPEC.md -> Keys' mouse table documents {documented:?} while \
         ui::driver::mouse_action produces {implemented:?}"
    );

    // Leg 2: the table parses onto the claim's four axes at all — a gesture
    // phrase outside the closed vocabulary, a `Zone` token naming no variant, a
    // zone-less binding row, a missing or duplicated catch-all, and a gutted
    // table are each an error here rather than a silent weakening.
    let rows = documented_mouse_rows(&spec_md).expect("SPEC.md -> Keys' mouse table, by row");

    // Leg 3: the two-way coverage, against `mouse_action` **executed** at every
    // cell of the swept frames rather than against its source text.
    let claims = all_mouse_claims();
    if let Err(report) = compare_mouse_claims(&rows, &claims) {
        panic!("SPEC.md -> Keys' mouse table and ui::driver::mouse_action disagree:\n  {report}");
    }

    // What the sweep must have reached for legs 2 and 3 to mean anything. A
    // fixture or a frame that stopped reaching a gesture or a zone would make
    // the rows resting on it vacuous; these say so directly instead.
    let kinds: BTreeSet<&str> = claims.iter().map(|c| c.kind).collect();
    assert_eq!(
        kinds.len(),
        MOUSE_KINDS.len(),
        "the sweep reaches {} of the {} MouseEventKind values: {kinds:?}",
        kinds.len(),
        MOUSE_KINDS.len()
    );
    let zones: BTreeSet<&str> = claims.iter().filter_map(|c| c.zone).collect();
    assert_eq!(
        zones.len(),
        ZONE_VARIANTS.len(),
        "the sweep reaches {} of the six Zone variants: {zones:?}",
        zones.len()
    );
    assert!(
        claims.iter().any(|c| c.help_open),
        "the overlay-open pass is represented in the claim set"
    );

    // The two rows this change added, for a binding the pane has had since
    // `help-overlay` and the table documented only in the paragraph after it.
    assert!(claims.contains(&claim("ScrollDown", true, None, "ScrollDown")));
    assert!(claims.contains(&claim("ScrollUp", true, None, "ScrollUp")));

    assert_eq!(
        KNOWN_MOUSE_COLLISIONS.len(),
        1,
        "the collision list is pinned by length on EXEMPT_ACTIONS' terms, so the \
         vacuity direction's blindness cannot grow unnoticed: {KNOWN_MOUSE_COLLISIONS:?}"
    );
}

/// Claims two rows may legitimately share, listed by name and pinned by count.
///
/// The vacuity direction cannot tell such rows apart: each covers the claim, so
/// neither is vacuous. Rather than weaken the assertion, the pairs are written
/// down here and the list's length asserted, so a **third** row joining a
/// collision fails rather than passing unnoticed
/// (`design.md` -> Decision 10).
///
/// The one at HEAD is the click on a change row and the second click on the row
/// already selected: both produce `Click(Change)`, because "a second click opens
/// the detail" is decided in `Dashboard::apply`, not in `mouse_action`.
const KNOWN_MOUSE_COLLISIONS: [(&str, bool, &str, &str, usize); 1] =
    [("Down(Left)", false, "ListRow", "Click(Change)", 2)];

/// One row's claims: the **cross product** of its gestures, its zones and its
/// outcomes. An overlay-open row has one zone slot, `None`, because
/// `mouse_action` resolves no zone at all under that state; a row that is
/// neither the catch-all nor an overlay row and names no zone has an **empty**
/// cross product and is therefore vacuous, which is the right answer.
///
/// The catch-all is not computed this way: it claims the remainder.
fn row_claims(row: &MouseRow) -> BTreeSet<(&'static str, bool, Option<&'static str>, &str)> {
    let slots: Vec<Option<&'static str>> = if row.help_open {
        vec![None]
    } else {
        row.zones.iter().map(|zone| Some(*zone)).collect()
    };
    let mut out = BTreeSet::new();
    for kind in &row.kinds {
        for zone in &slots {
            for outcome in &row.outcomes {
                out.insert((*kind, row.help_open, *zone, outcome.as_str()));
            }
        }
    }
    out
}

fn claim_key(c: &Claim) -> (&'static str, bool, Option<&'static str>, &'static str) {
    (c.kind, c.help_open, c.zone, c.outcome)
}

fn describe(kind: &str, help_open: bool, zone: Option<&str>, outcome: &str) -> String {
    let overlay = if help_open {
        "overlay open"
    } else {
        "overlay closed"
    };
    let zone = zone.map_or_else(|| "no zone".to_string(), |z| format!("Zone::{z}"));
    format!("({kind}, {overlay}, {zone}, {outcome})")
}

/// The two-way comparison, as a **pure function over both sides** so a defect
/// can be planted on either without doctoring the tree
/// (`design.md` -> Decision 3).
///
/// 1. every claim the sweep observed is covered by at least one row; one covered
///    by none fails as **undocumented**, naming all four axes;
/// 2. every row covers at least one observed claim; a row covering none fails as
///    **vacuous**, naming the row's own text and the claims actually observed at
///    that row's own zones, so the reader is told what the row should have said.
///
/// Both directions are reported when both fail, rather than the first masking
/// the second. The catch-all claims the remainder — but only claims whose
/// outcome is `Ignore`, so a new **active** binding can never hide in it — and
/// must claim at least one.
fn compare_mouse_claims(rows: &[MouseRow], claims: &BTreeSet<Claim>) -> Result<(), String> {
    let catch_alls: Vec<&MouseRow> = rows.iter().filter(|row| row.catch_all).collect();
    if catch_alls.len() != 1 {
        return Err(format!(
            "the mouse table must carry exactly one catch-all row and carries {}",
            catch_alls.len()
        ));
    }

    let observed: BTreeSet<(&str, bool, Option<&str>, &str)> =
        claims.iter().map(claim_key).collect();

    let mut problems: Vec<String> = Vec::new();

    // Direction 2, and the coverage tally direction 1 and the collision check
    // both read.
    let mut cover_count: BTreeMap<(&str, bool, Option<&str>, &str), Vec<&str>> = BTreeMap::new();
    for row in rows.iter().filter(|row| !row.catch_all) {
        let mine = row_claims(row);
        let hits: Vec<_> = mine.iter().filter(|c| observed.contains(*c)).collect();
        if hits.is_empty() {
            let near: Vec<String> = claims
                .iter()
                .filter(|c| {
                    c.help_open == row.help_open
                        && row.kinds.contains(&c.kind)
                        && c.zone.is_some_and(|z| row.zones.contains(&z))
                })
                .map(|c| describe(c.kind, c.help_open, c.zone, c.outcome))
                .collect();
            problems.push(format!(
                "vacuous row - it covers no observed claim: {:?}\n    it claims {:?}\n    \
                 what is actually observed at its own gestures and zones is {near:?}",
                row.text,
                mine.iter()
                    .map(|(k, h, z, o)| describe(k, *h, *z, o))
                    .collect::<Vec<_>>()
            ));
        }
        for hit in hits {
            cover_count.entry(*hit).or_default().push(row.text.as_str());
        }
    }

    // Direction 1. The catch-all takes the remainder, `Ignore` only.
    let mut catch_all_claims = 0usize;
    let mut undocumented: Vec<String> = Vec::new();
    for key in &observed {
        if cover_count.contains_key(key) {
            continue;
        }
        if key.3 == "Ignore" {
            catch_all_claims += 1;
        } else {
            undocumented.push(describe(key.0, key.1, key.2, key.3));
        }
    }
    if !undocumented.is_empty() {
        problems.push(format!(
            "undocumented - the pane produces these and no row covers them: {undocumented:?}. \
             The catch-all does not absorb them: it covers only claims whose outcome is \
             `Ignore`, so a new active binding can never hide in it"
        ));
    }
    if catch_all_claims == 0 {
        problems.push(format!(
            "the catch-all row covers nothing - every observed `Ignore` claim is already \
             claimed by another row: {:?}",
            catch_alls[0].text
        ));
    }

    // The bounded blindness: a claim shared by more than one row must be listed.
    for (key, texts) in &cover_count {
        if texts.len() < 2 {
            continue;
        }
        let pinned = KNOWN_MOUSE_COLLISIONS
            .iter()
            .find(|(kind, help_open, zone, outcome, _)| {
                (*kind, *help_open, Some(*zone), *outcome) == *key
            })
            .map(|(_, _, _, _, rows)| *rows);
        match pinned {
            Some(count) if count == texts.len() => {}
            Some(count) => problems.push(format!(
                "{} rows claim {} - KNOWN_MOUSE_COLLISIONS pins that collision at {count}. \
                 The rows are {texts:?}",
                texts.len(),
                describe(key.0, key.1, key.2, key.3)
            )),
            None => problems.push(format!(
                "{} rows claim {}, which KNOWN_MOUSE_COLLISIONS does not list. Two rows may \
                 legitimately share one claim, but the vacuity direction cannot tell them \
                 apart, so every such pair is listed by name and pinned by count. The rows \
                 are {texts:?}",
                texts.len(),
                describe(key.0, key.1, key.2, key.3)
            )),
        }
    }

    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n  "))
    }
}

/// The real table's rows, for the comparator's controls: one real side, one
/// hand-built (`design.md` -> Test Boundaries).
fn real_mouse_rows() -> Vec<MouseRow> {
    let spec_md = read_doc(&manifest_dir().join("SPEC.md")).expect("read SPEC.md");
    documented_mouse_rows(&spec_md).expect("SPEC.md -> Keys' mouse table")
}

/// Every claim the sweep observes, both overlay states over every fixture.
fn all_mouse_claims() -> BTreeSet<Claim> {
    let mut claims = mouse_claims(false);
    claims.extend(mouse_claims(true));
    claims
}

fn row_index(rows: &[MouseRow], needle: &str) -> usize {
    rows.iter()
        .position(|row| row.text.contains(needle))
        .unwrap_or_else(|| panic!("SPEC.md -> Keys' mouse table has no row containing {needle:?}"))
}

#[test]
fn a_row_describing_a_removed_binding_fails_as_vacuous() {
    // The regression this whole change exists for. `mouse-text-selection`
    // deleted the row describing `Target::DetailLine` and the shipped check
    // could not have noticed if it had not: a set equality over backticked
    // `Action::` names never reads a row's prose.
    let mut rows = real_mouse_rows();
    let at = row_index(
        &rows,
        "Left click on any other row of the detail region's content area",
    );
    let before = rows[at].outcomes.clone();
    rows[at].outcomes = vec!["Click(DetailLine)".to_string()];
    assert_ne!(before, rows[at].outcomes);

    let err = compare_mouse_claims(&rows, &all_mouse_claims())
        .expect_err("a row covering no observed claim is vacuous");
    assert!(err.contains("vacuous"), "{err}");
    assert!(err.contains("Click(DetailLine)"), "{err}");
    // Task 5.3: the reader is told what the row should have said.
    assert!(err.contains("Click(DetailHeader)"), "{err}");
    assert!(err.contains("Select(Begin)"), "{err}");

    // Leg 1 is **not** what fails: the row still names `Action::Click`, which
    // `mouse_action` still produces elsewhere, which is exactly why the shipped
    // check passed this row.
    assert!(rows[at].outcomes.iter().all(|o| o.starts_with("Click")));
}

#[test]
fn two_rows_differing_only_in_payload_are_told_apart() {
    let mut rows = real_mouse_rows();
    let at = row_index(&rows, "Left click on a section header");
    rows[at].outcomes = vec!["Click(Change)".to_string()];

    let err = compare_mouse_claims(&rows, &all_mouse_claims())
        .expect_err("Click(Section) is left uncovered");
    assert!(err.contains("Click(Section)"), "{err}");
    assert!(err.contains("ListRow"), "{err}");

    // The companion assertion, and the reason axis 4 keeps the payload
    // constructor: under the **bare** variant key the same mutation is
    // invisible. `Click(Section)` and `Click(Change)` both collapse to `Click`,
    // the mutated row still covers the claim, and nothing is left uncovered.
    let bare = |outcome: &str| -> String {
        outcome
            .split_once('(')
            .map_or(outcome.to_string(), |(variant, _)| variant.to_string())
    };
    let observed: BTreeSet<(&str, bool, Option<&str>, String)> = all_mouse_claims()
        .iter()
        .map(|c| (c.kind, c.help_open, c.zone, bare(c.outcome)))
        .collect();
    let mut covered = BTreeSet::new();
    for row in &rows {
        if row.catch_all {
            continue;
        }
        for kind in &row.kinds {
            let slots: Vec<Option<&str>> = if row.help_open {
                vec![None]
            } else {
                row.zones.iter().map(|z| Some(*z)).collect()
            };
            for zone in slots {
                for outcome in &row.outcomes {
                    covered.insert((*kind, row.help_open, zone, bare(outcome)));
                }
            }
        }
    }
    let uncovered: Vec<_> = observed
        .iter()
        .filter(|c| c.3 != "Ignore" && !covered.contains(*c))
        .collect();
    assert!(
        uncovered.is_empty(),
        "under the bare-variant key the mutation leaves nothing uncovered, which \
         is the difference the payload discriminant exists to make: {uncovered:?}"
    );
}

#[test]
fn a_binding_with_no_row_fails_as_undocumented() {
    let rows = real_mouse_rows();
    let mut claims = all_mouse_claims();
    claims.insert(claim("Down(Middle)", false, Some("List"), "ToggleHelp"));

    let err = compare_mouse_claims(&rows, &claims).expect_err("a bound gesture with no row");
    assert!(err.contains("undocumented"), "{err}");
    assert!(err.contains("Down(Middle)"), "{err}");
    assert!(err.contains("List"), "{err}");
    assert!(err.contains("ToggleHelp"), "{err}");
    // The catch-all does not absorb it: it covers only `Ignore` claims, so a new
    // **active** binding can never hide in it. That is what `undocumented`
    // asserted above already proves - the claim reached the uncovered list
    // rather than the catch-all's tally - so there is nothing weaker to add.
    assert!(
        err.contains("overlay closed"),
        "all four axes are named: {err}"
    );
}

#[test]
fn the_overlay_axis_keeps_the_two_passes_apart() {
    let mut rows = real_mouse_rows();
    let at = row_index(&rows, "Left click outside the help overlay's band");
    rows[at].help_open = false;

    let err = compare_mouse_claims(&rows, &all_mouse_claims())
        .expect_err("the overlay axis is load-bearing");
    // Both failures, rather than one masking the other.
    assert!(err.contains("vacuous"), "{err}");
    assert!(err.contains("undocumented"), "{err}");
    assert!(err.contains("ToggleHelp"), "{err}");
}

#[test]
fn an_empty_catch_all_fails() {
    // Driven with a synthetic claim set rather than the real tree, and
    // deliberately: at HEAD the great majority of claims are `Ignore`, so
    // against the real tree this rule could never fire — and an assertion that
    // cannot go red is not a guard.
    let rows = real_mouse_rows();
    let active: BTreeSet<Claim> = all_mouse_claims()
        .into_iter()
        .filter(|c| c.outcome != "Ignore")
        .collect();
    assert!(!active.is_empty());

    let err = compare_mouse_claims(&rows, &active).expect_err("a catch-all covering nothing");
    assert!(err.contains("catch-all row covers nothing"), "{err}");
    assert!(err.contains("Anything else"), "the row is named: {err}");
    // Nothing else fails: every active claim is still covered and no row is
    // vacuous, so this is the empty catch-all alone rather than a side effect.
    assert!(!err.contains("vacuous"), "{err}");
    assert!(!err.contains("undocumented"), "{err}");
}

#[test]
fn a_third_row_joining_a_known_collision_fails() {
    let mut rows = real_mouse_rows();
    let at = row_index(&rows, "Left click on a change row");
    let mut third = rows[at].clone();
    third.text = "| Left click on a change row, a third time | `Action::Click` naming \
                  `Target::Change` in `Zone::ListRow` |"
        .to_string();
    rows.push(third);

    let err = compare_mouse_claims(&rows, &all_mouse_claims())
        .expect_err("a third row joining the collision");
    assert!(err.contains("a third time"), "{err}");
    assert!(err.contains("Click(Change)"), "{err}");
    assert!(err.contains('2'), "the pinned count is named: {err}");
}

#[test]
fn terminal_seam_names_match_the_gate() {
    let agents_md = read_doc(&manifest_dir().join("AGENTS.md")).expect("read AGENTS.md");
    let script = read_doc(&manifest_dir().join("scripts/gates/noraw-grep.sh"))
        .expect("read scripts/gates/noraw-grep.sh");

    let documented = documented_seam_names(&agents_md).expect("AGENTS.md's confined set");
    let searched = gate_raw_names(&script).expect("noraw-grep.sh's RAW_RE");

    assert_eq!(
        documented, searched,
        "AGENTS.md names {documented:?} as the confined terminal-seam set while \
         noraw-grep.sh's RAW_RE searches for {searched:?}"
    );
    // The size, not a third hardcoded list of the names themselves: `NORAW-GREP`
    // sweeps `tests/` as well as `src/`, so spelling the six confined names in
    // this file would break the very invariant this leg checks. `terminal-lifecycle`
    // states six; the two sites above are what say *which* six, and
    // `noraw-grep.sh`'s own per-name positive control is what binds that list to
    // `src/ui/terminal.rs`.
    assert_eq!(
        documented.len(),
        6,
        "terminal-lifecycle states six confined terminal-mode functions, not \
         {}: {documented:?}",
        documented.len()
    );
}

#[test]
fn documented_mouse_actions_fails_on_a_missing_table() {
    // The parser control: a `### Keys` section with no mouse table must be an
    // `Err` naming the absent table, never an empty set that compares equal to
    // an empty set.
    let no_table = "### Keys\n\n| Key | Action |\n|---|---|\n| `q` | Quit |\n\n### Next\n";
    let err = documented_mouse_actions(no_table).expect_err("no mouse table is an error");
    assert!(err.contains("no mouse table"), "{err}");

    let no_section = "## Overview\n\nnothing here\n";
    assert!(documented_mouse_actions(no_section).is_err());

    // A table whose rows carry no backticked variant is an error too, so a
    // reworded table cannot silently empty the documented set.
    let unnamed = "### Keys\n\n| Gesture | Action |\n|---|---|\n| Wheel down | scrolls |\n";
    let err = documented_mouse_actions(unnamed).expect_err("no variant named is an error");
    assert!(err.contains("names no `Action::` variant"), "{err}");
}

/// A synthetic `### Keys` section carrying `rows` as its mouse table, for the
/// row extractor's own controls. The file's established idiom: every parser here
/// is a pure `&str -> Result` fed hand-written and malformed inputs, so a
/// planted defect is an argument rather than a doctored tree.
fn mouse_table(rows: &[&str]) -> String {
    let mut out = String::from("### Keys\n\n| Gesture | Action |\n|---|---|\n");
    for row in rows {
        out.push_str(row);
        out.push('\n');
    }
    out.push_str("\nProse after the table.\n\n### Next\n");
    out
}

#[test]
fn an_unrecognised_gesture_phrase_is_an_error() {
    // `design.md` -> Decision 5: the gesture vocabulary is closed, and a phrase
    // outside it is a hard error naming the row. A row silently skipped would
    // drop out of **both** directions of the assertion, so a typo would weaken
    // the check rather than fail it.
    let doc = mouse_table(&[
        "| Left quadruple-click on a change row | `Action::Click` naming `Target::Change` in \
         `Zone::ListRow` |",
        "| Anything else | `Action::Ignore` |",
    ]);
    let err = documented_mouse_rows(&doc).expect_err("an unlisted phrase is an error");
    assert!(err.contains("Left quadruple-click"), "{err}");
    assert!(err.contains("gesture"), "{err}");
}

#[test]
fn a_mistyped_zone_token_is_named() {
    let doc = mouse_table(&[
        "| Left click on a change row | `Action::Click` naming `Target::Change` in \
         `Zone::DetailRows` |",
        "| Anything else | `Action::Ignore` |",
    ]);
    let err = documented_mouse_rows(&doc).expect_err("a mistyped Zone token is an error");
    assert!(err.contains("DetailRows"), "{err}");
    assert!(err.contains("no `Zone` variant"), "{err}");
    // Not the vacuity message, which would point the reader at the wrong problem.
    assert!(!err.contains("vacuous"), "{err}");
}

#[test]
fn a_zone_less_row_is_rejected_unless_it_is_the_catch_all() {
    let doc = mouse_table(&[
        "| Left click on an artifact tab cell | `Action::SelectTab` for that cell's own \
         position |",
        "| Anything else | `Action::Ignore` |",
    ]);
    let err = documented_mouse_rows(&doc).expect_err("a zone-less binding row is an error");
    assert!(err.contains("artifact tab cell"), "{err}");
    assert!(err.contains("catch-all"), "{err}");

    // The two rows that may omit a zone: the catch-all, and a row that names the
    // overlay-open state instead, where `mouse_action` resolves no zone at all.
    let ok = mouse_table(&[
        "| Left click on an artifact tab cell | `Action::SelectTab` in `Zone::DetailTab` |",
        "| Left click outside the band while `help.open` | `Action::ToggleHelp` |",
        "| Anything else | `Action::Ignore` |",
    ]);
    let rows = documented_mouse_rows(&ok).expect("both omissions are legal");
    assert_eq!(rows.len(), 3, "{rows:?}");
    assert!(
        rows[1].help_open,
        "the overlay row carries the overlay axis"
    );
    assert!(!rows[0].help_open && !rows[2].help_open);
}

#[test]
fn an_overlay_row_naming_a_zone_is_an_error() {
    // While the overlay is open `mouse_action` resolves no zone, so `row_claims`
    // gives such a row one `None` slot and would silently discard whatever it
    // named. Reported on exactly the terms the mistyped-token check already
    // holds to: never left to surface as an unexplained vacuity.
    let doc = mouse_table(&[
        "| Left click outside the band while `help.open` (`Zone::List`) | `Action::ToggleHelp` |",
        "| Anything else | `Action::Ignore` |",
    ]);
    let err = documented_mouse_rows(&doc).expect_err("an overlay row may not name a zone");
    assert!(err.contains("help.open"), "{err}");
    assert!(err.contains("List"), "{err}");
    assert!(!err.contains("vacuous"), "{err}");
}

#[test]
fn a_second_catch_all_is_an_error() {
    let two = mouse_table(&[
        "| Left click on a change row | `Action::Click` naming `Target::Change` in \
         `Zone::ListRow` |",
        "| Anything else - a right press | `Action::Ignore` |",
        "| Anything else - a middle press | `Action::Ignore` |",
    ]);
    let err = documented_mouse_rows(&two).expect_err("a second catch-all is an error");
    assert!(err.contains("a right press"), "naming both rows: {err}");
    assert!(err.contains("a middle press"), "naming both rows: {err}");

    // And none at all is an error too: the catch-all is what absorbs the
    // `Ignore` claims, so a table without one cannot cover the observed set.
    let none = mouse_table(&[
        "| Left click on a change row | `Action::Click` naming `Target::Change` in \
         `Zone::ListRow` |",
    ]);
    let err = documented_mouse_rows(&none).expect_err("no catch-all is an error");
    assert!(err.contains("catch-all"), "{err}");
}

#[test]
fn a_gutted_mouse_table_fails_as_a_broken_control() {
    // Exactly `documented_mouse_actions`' own rule: an empty documented set is
    // never compared against an observed one and allowed to pass.
    let empty = "### Keys\n\n| Gesture | Action |\n|---|---|\n\nProse.\n";
    let err = documented_mouse_rows(empty).expect_err("header and separator only");
    assert!(err.contains("row"), "{err}");

    let no_header = "### Keys\n\n| Key | Action |\n|---|---|\n| `q` | Quit |\n";
    let err = documented_mouse_rows(no_header).expect_err("no mouse table");
    assert!(err.contains("no mouse table"), "{err}");

    assert!(documented_mouse_rows("## Overview\n\nnothing here\n").is_err());
}

#[test]
fn the_gesture_vocabulary_is_closed_and_covers_the_real_table() {
    // `design.md` -> Decision 5: pinned by length, so a sixth phrase is a
    // deliberate edit. The five are the ones `SPEC.md`'s own table uses.
    assert_eq!(MOUSE_GESTURES.len(), 5, "{MOUSE_GESTURES:?}");
    let phrases: BTreeSet<&str> = MOUSE_GESTURES.iter().map(|(p, _)| *p).collect();
    assert_eq!(
        phrases,
        BTreeSet::from([
            "Wheel down",
            "Wheel up",
            "Left click",
            "Left drag",
            "Anything else"
        ])
    );
}

#[test]
fn mouse_action_body_is_cut_from_the_production_slice() {
    // The parser control for the other side: the cut stops at the function's own
    // closing brace, and never reaches the inline test module — which names
    // every `Action` variant the crate has.
    let src = "pub fn mouse_action(a: u8) -> Action {\n    Action::Ignore\n}\n\n\
               pub fn other() {\n    Action::Quit\n}\n\n\
               #[cfg(test)]\nmod tests {\n    fn t() { Action::Refresh; }\n}\n";
    let body = mouse_action_body(src).expect("cut the body");
    let found: BTreeSet<String> = backticked_action_variants(body).into_iter().collect();
    assert_eq!(found, ["Ignore".to_string()].into_iter().collect());

    assert!(mouse_action_body("fn nothing() {}\n").is_err());
}

#[test]
fn gate_raw_names_parses_and_fails_loudly() {
    let script = "RAW_RE='a|b|c'\n";
    assert_eq!(
        gate_raw_names(script).expect("parse"),
        ["a", "b", "c"].into_iter().map(str::to_string).collect()
    );
    assert!(gate_raw_names("# no RAW_RE here\n").is_err());
}

#[test]
fn documented_seam_names_takes_identifiers_only() {
    // Fabricated names, never the real six: `NORAW-GREP` sweeps `tests/` too, so a
    // fixture spelling them here would fail the confinement this file's own
    // `terminal_seam_names_match_the_gate` leg exists to check.
    let text = "`src/ui/terminal.rs` is the only file in the crate \
                permitted to name a crossterm terminal-mode function\n  \
                (`alpha_mode`, `BetaScreen`) — two names since `some-change`\n";
    let names = documented_seam_names(text).expect("parse");
    assert_eq!(
        names,
        ["BetaScreen", "alpha_mode"]
            .into_iter()
            .map(str::to_string)
            .collect(),
        "a hyphenated change name is not an identifier and is not taken"
    );
    assert!(documented_seam_names("nothing here\n").is_err());
}

// ---------------------------------------------------------------------------
// The tenth claim: the pane's bindings, bound to the functions that produce
// them. Three legs — the action sweep against `ui::help::INVENTORY`, and
// `INVENTORY` against each of `SPEC.md` -> Keys and `README.md` -> Keys. See
// `openspec/changes/help-overlay/specs/doc-conformance/spec.md` (legs 2 and 3,
// and the normalisation they compare under) and
// `.../specs/binding-inventory/spec.md` (the sweep, the exemption set, and the
// per-row parser).
//
// Unlike every other leg in this file, legs 1 and the per-row check parse no
// source text at all: `action_for` and `mouse_action` are pure total functions,
// so the set each produces is computable by **calling** it, and a derivation
// that calls the function cannot disagree with the function the way one that
// parses it can. Still no process is spawned.
// ---------------------------------------------------------------------------

use std::collections::BTreeMap;

use ratatui::crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Rect;

use herdr_openspec::agents::AgentSnapshot;
use herdr_openspec::changes::{ArtifactRef, Change, ChangeSet, Origin};
use herdr_openspec::state::Mapping;
use herdr_openspec::tasks::Progress;
use herdr_openspec::ui::app::{
    Action, ArtifactSection, Dashboard, Detail, Filter, Granularity, Launch, Overlay, Panel,
    Refresh, Route, Sections, SelectPhase, Selection, Target, action_for,
};
use herdr_openspec::ui::driver::mouse_action;
use herdr_openspec::ui::help::{INVENTORY, Scope};
use herdr_openspec::ui::layout::Zone;

/// Every `Action` mapped to a stable name by an **exhaustive** `match` with no
/// wildcard arm: a variant added later is a compile error here, in this test,
/// before it is a missing help row.
fn action_name(action: Action) -> &'static str {
    match action {
        Action::Quit => "Quit",
        Action::OpenDetail => "OpenDetail",
        Action::Back => "Back",
        Action::Next => "Next",
        Action::Prev => "Prev",
        Action::SelectTab(_) => "SelectTab",
        Action::NextTab => "NextTab",
        Action::PrevTab => "PrevTab",
        Action::FilterStart => "FilterStart",
        Action::FilterPush(_) => "FilterPush",
        Action::FilterPop => "FilterPop",
        Action::Refresh => "Refresh",
        Action::LaunchApply => "LaunchApply",
        Action::LaunchContinue => "LaunchContinue",
        Action::LaunchArchive => "LaunchArchive",
        Action::FocusAgent => "FocusAgent",
        Action::ToggleSection => "ToggleSection",
        Action::ToggleHelp => "ToggleHelp",
        Action::SelectNext => "SelectNext",
        Action::SelectPrev => "SelectPrev",
        Action::ScrollDown => "ScrollDown",
        Action::ScrollUp => "ScrollUp",
        Action::Click(_) => "Click",
        Action::Select(_) => "Select",
        Action::Ignore => "Ignore",
    }
}

/// The **closed** exemption set, named rather than predicated: `FilterPush` is
/// typing rather than a binding, and `Ignore` is the absence of one. A third
/// exemption costs a spec change — see `binding-inventory`.
const EXEMPT_ACTIONS: [&str; 2] = ["FilterPush", "Ignore"];

/// The **only two** normalisations legs 2 and 3 compare under, named here in
/// the check's own source as `doc-conformance` requires. A future binding whose
/// prose spelling does not atomise to its `INVENTORY` spelling is made to agree
/// by editing the document, never by growing this list.
///
/// The first is the bare word `arrows` in a Key cell, which stands for the two
/// arrow keys `INVENTORY` spells `↑` and `↓`. The second is a backticked pair
/// joined by an en-dash, which is one range atom rather than its two endpoints.
const KEY_ALIASES: [(&str, &[&str]); 2] = [("arrows", &["↑", "↓"]), ("`1`–`9`", &["1–9"])];

fn press(code: KeyCode, modifiers: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, modifiers))
}

/// Step 2 of the derivation: every printable ASCII `Char`, plus the named keys
/// a terminal can send that this pane might bind, each under four modifier
/// values and both filter modes.
///
/// Computed once per test binary and cloned thereafter. Seven tests below want
/// a swept set and the sweeps are the expensive part of this file — the mouse
/// one resolves a `Down(Left)` at every cell of two frames at two routes, and
/// each such cell inside the detail region renders the fixture's markdown. The
/// cache changes nothing about what is swept; it only stops the same total
/// function being asked the same question fourteen times.
fn swept_key_action_names() -> BTreeSet<String> {
    static CACHE: std::sync::OnceLock<BTreeSet<String>> = std::sync::OnceLock::new();
    CACHE.get_or_init(sweep_key_actions).clone()
}

fn sweep_key_actions() -> BTreeSet<String> {
    let mut codes: Vec<KeyCode> = (' '..='~').map(KeyCode::Char).collect();
    codes.extend([
        KeyCode::Backspace,
        KeyCode::Enter,
        KeyCode::Esc,
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Tab,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::Delete,
    ]);

    let mut names = BTreeSet::new();
    for code in codes {
        for modifiers in [
            KeyModifiers::NONE,
            KeyModifiers::SHIFT,
            KeyModifiers::CONTROL,
            KeyModifiers::ALT,
        ] {
            for filtering in [false, true] {
                names.insert(
                    action_name(action_for(&press(code, modifiers), filtering)).to_string(),
                );
            }
        }
    }
    names
}

/// One change carrying `tabs` artifacts. `tabs` is not decoration: a dashboard
/// whose selected change carries no artifacts makes `Zone::DetailTab` resolve
/// to `Action::Ignore` and silently removes the mouse's tab-switching coverage
/// from the whole sweep.
fn sweep_change(name: &str, origin: Origin, tabs: usize) -> Change {
    Change {
        name: name.to_string(),
        dir: PathBuf::from(format!("/repo/openspec/changes/{name}")),
        origin,
        schema: "tdd".to_string(),
        artifacts: (0..tabs)
            .map(|i| ArtifactRef {
                id: format!("artifact-{i}"),
                paths: vec![PathBuf::from(format!(
                    "/repo/openspec/changes/{name}/artifact-{i}.md"
                ))],
                tracks_tasks: false,
            })
            .collect(),
        progress: Progress {
            completed: 1,
            total: 3,
        },
        problems: Vec::new(),
    }
}

/// The fixture step 3 mandates: active changes, archived changes, a selected
/// change with several artifact tabs, and a **foldable** artifact (more than one
/// resolved section), so `Zone::DetailRow`'s fold path is reachable too.
///
/// `text-selection` widens it once more, per the same rule the `SelectTab`
/// note below already states: `detail.expanded` holds section `0`, so that
/// section's own body row is drawn and `Action::Select` is reachable at all.
/// Both sections collapsed would let every drawn content row be a header,
/// silently dropping the mouse's selection coverage from the whole sweep —
/// the widen-the-fixture rule `the_sweep_covers_the_mouse_under_both_overlay_states`
/// already states for the tab-switching case applies here identically.
fn sweep_dashboard(route: Route, help_open: bool, fixture: SweepFixture) -> Dashboard {
    Dashboard {
        selection: match fixture {
            SweepFixture::SelectionAbsent => None,
            // A drag already in progress, anchored at the content's first cell.
            // `mouse_action`'s clamp arm reads `selection.is_some()` and nothing
            // else about it, so the span's own extent is not part of the fixture.
            SweepFixture::SelectionPresent => Some(Selection {
                anchor: (0, 0),
                focus: (0, 0),
                granularity: Granularity::Span,
                problem: None,
            }),
        },
        repo: Some(PathBuf::from("/repo")),
        searched_from: PathBuf::from("/repo"),
        changes: ChangeSet {
            active: vec![
                sweep_change("alpha", Origin::Active, 4),
                sweep_change("beta", Origin::Active, 4),
            ],
            archived: vec![sweep_change(
                "gamma",
                Origin::Archived {
                    date: Some("2026-01-01".to_string()),
                },
                2,
            )],
            problems: Vec::new(),
            archived_total: 1,
        },
        route,
        quit: false,
        // `targets()` is [Section(Active), Change(0), Change(1),
        // Section(Archived), Change(2)] with both sections open, so index 1 is
        // a change and `selected_change()` is `Some`.
        selected: 1,
        filter: Filter {
            query: String::new(),
            active: false,
        },
        detail: Detail {
            sections: vec![
                ArtifactSection {
                    label: Some("one".to_string()),
                    text: "# One\n\nbody one\n".to_string(),
                    depth: 0,
                    progress: None,
                    operation: None,
                },
                ArtifactSection {
                    label: Some("two".to_string()),
                    text: "# Two\n\nbody two\n".to_string(),
                    depth: 0,
                    progress: None,
                    operation: None,
                },
            ],
            scroll: 0,
            tab: 0,
            problems: Vec::new(),
            loaded: None,
            expanded: BTreeSet::from([0]),
            drawn_width: None,
        },
        refresh: Refresh {
            requested: false,
            reload: false,
            startup: Vec::new(),
            problems: Vec::new(),
        },
        agents: AgentSnapshot {
            agents: Vec::new(),
            reachable: true,
            stalled: false,
            problem: None,
        },
        agent_names: Mapping {
            names: BTreeMap::new(),
            problems: Vec::new(),
        },
        launch: Launch {
            pending: None,
            problems: Vec::new(),
            in_flight: false,
        },
        sections: Sections {
            collapsed: BTreeSet::new(),
        },
        file_mode: false,
        overlay: Overlay {
            panel: if help_open { Some(Panel::Help) } else { None },
            scroll: 0,
            edit: None,
        },
    }
}

/// The **dashboard fixtures** the mouse sweep runs over, listed explicitly and
/// pinned by length, on exactly the terms [`EXEMPT_ACTIONS`] is pinned at two.
///
/// `mouse_action` is a total function of a `Dashboard`, a `Rect` and a
/// `MouseEvent` — the `Dashboard` included — so a binding that branches on
/// dashboard state is unobservable under a fixture that pins that state. A row
/// resting on such a precondition would be reported vacuous and the check would
/// be right to. See `doc-conformance` -> "The dashboard-fixture axis".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SweepFixture {
    /// `selection: None` — the state the sweep has always run in.
    SelectionAbsent,
    /// `selection: Some(..)` — a selection already in progress, which is the
    /// only state `mouse_action`'s clamp arm is reachable from.
    SelectionPresent,
}

/// Pinned by length on exactly [`EXEMPT_ACTIONS`]' terms: a fixture added or
/// dropped is a deliberate edit here, never a silent narrowing of what the
/// sweep can observe.
const SWEEP_FIXTURES: [SweepFixture; 2] = [
    SweepFixture::SelectionAbsent,
    SweepFixture::SelectionPresent,
];

/// One cell's worth of what `mouse_action` decided, on the four axes a
/// documented row makes a statement about (`doc-conformance` -> "The claim").
///
/// Axis 4 keeps the payload's **constructor name**, never the bare `Action`
/// variant: `action_name` collapses every `Click(_)` to `"Click"`, and under
/// that collapse three separate documented rows reduce to one claim and the
/// vacuity direction cannot fire at all.
///
/// Every axis is a `&'static str` drawn from a closed vocabulary rather than an
/// owned `String`, and deliberately: the sweep inserts one claim per cell of
/// every frame, kind, overlay state and fixture — hundreds of thousands of them
/// — and an allocation per axis per cell is the whole of the difference between
/// a sweep that costs seconds and one that costs minutes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Claim {
    /// The `MouseEventKind` dispatched, as [`kind_name`] spells it.
    kind: &'static str,
    /// The overlay state, as its own axis rather than a sentinel in `zone`.
    help_open: bool,
    /// The `Zone` the cell resolved to, payload discarded — `None` while the
    /// overlay is open, where `mouse_action` returns before consulting
    /// `ui::layout::zone` at all (`design.md` -> Decision 6 and Decision 11).
    zone: Option<&'static str>,
    /// The `Action` variant together with its payload's constructor name where
    /// it has one: `Click(Change)`, `Select(Begin)`, `SelectTab`, `Ignore`.
    outcome: &'static str,
}

/// A [`Claim`] from its four axes, so a test and the comparator spell one the
/// same way.
fn claim(
    kind: &'static str,
    help_open: bool,
    zone: Option<&'static str>,
    outcome: &'static str,
) -> Claim {
    Claim {
        kind,
        help_open,
        zone,
        outcome,
    }
}

/// Every `MouseEventKind` the crate can receive — all fourteen inhabited values
/// of an eight-variant enum over three buttons.
const MOUSE_KINDS: [MouseEventKind; 14] = [
    MouseEventKind::Down(MouseButton::Left),
    MouseEventKind::Down(MouseButton::Right),
    MouseEventKind::Down(MouseButton::Middle),
    MouseEventKind::Up(MouseButton::Left),
    MouseEventKind::Up(MouseButton::Right),
    MouseEventKind::Up(MouseButton::Middle),
    MouseEventKind::Drag(MouseButton::Left),
    MouseEventKind::Drag(MouseButton::Right),
    MouseEventKind::Drag(MouseButton::Middle),
    MouseEventKind::Moved,
    MouseEventKind::ScrollDown,
    MouseEventKind::ScrollUp,
    MouseEventKind::ScrollLeft,
    MouseEventKind::ScrollRight,
];

/// Axis 1's spelling, by an **exhaustive** match: a `MouseEventKind` variant or
/// a `MouseButton` added later is a compile error here before it is an
/// unclaimable gesture.
fn kind_name(kind: MouseEventKind) -> &'static str {
    match kind {
        MouseEventKind::Down(MouseButton::Left) => "Down(Left)",
        MouseEventKind::Down(MouseButton::Right) => "Down(Right)",
        MouseEventKind::Down(MouseButton::Middle) => "Down(Middle)",
        MouseEventKind::Up(MouseButton::Left) => "Up(Left)",
        MouseEventKind::Up(MouseButton::Right) => "Up(Right)",
        MouseEventKind::Up(MouseButton::Middle) => "Up(Middle)",
        MouseEventKind::Drag(MouseButton::Left) => "Drag(Left)",
        MouseEventKind::Drag(MouseButton::Right) => "Drag(Right)",
        MouseEventKind::Drag(MouseButton::Middle) => "Drag(Middle)",
        MouseEventKind::Moved => "Moved",
        MouseEventKind::ScrollDown => "ScrollDown",
        MouseEventKind::ScrollUp => "ScrollUp",
        MouseEventKind::ScrollLeft => "ScrollLeft",
        MouseEventKind::ScrollRight => "ScrollRight",
    }
}

/// Axis 3's spelling, by an **exhaustive** match: a `Zone` variant added later
/// is a compile error here.
fn zone_name(zone: &Zone) -> &'static str {
    match zone {
        Zone::ListRow { .. } => "ListRow",
        Zone::List => "List",
        Zone::DetailTab { .. } => "DetailTab",
        Zone::DetailRow { .. } => "DetailRow",
        Zone::Detail => "Detail",
        Zone::Outside => "Outside",
    }
}

/// Axis 4's spelling: the `Action` variant plus its payload's constructor name
/// where it has one, the geometry inside (`line`, `column`, `Rect`, the index)
/// discarded. A **projection** of the value `mouse_action` already returned,
/// never a second computation that could disagree with it.
///
/// The fall-through delegates to [`action_name`], whose match is exhaustive, so
/// a new `Action` variant is still a compile error — there, rather than here.
fn outcome_name(action: Action) -> &'static str {
    match action {
        Action::Click(Target::Section(_)) => "Click(Section)",
        Action::Click(Target::Change(_)) => "Click(Change)",
        Action::Click(Target::DetailHeader { .. }) => "Click(DetailHeader)",
        Action::Select(SelectPhase::Begin { .. }) => "Select(Begin)",
        Action::Select(SelectPhase::Extend { .. }) => "Select(Extend)",
        other => action_name(other),
    }
}

/// Step 3: every [`MOUSE_KINDS`] value at every cell of a 120x40 frame and of a
/// 60x20 frame, under both overlay states and over every [`SWEEP_FIXTURES`]
/// entry — retaining a [`Claim`] per cell rather than only the action's name.
///
/// **The route dimension is this check's own addition, not the spec's.**
/// `binding-inventory` mandates the two frames, the kinds, the fixture and the
/// two overlay states, and names no route at all; sweeping the narrow frame at
/// both routes is what reaches the detail region there, since `split_body`'s
/// `Narrow` arm draws exactly one region and `route` is what picks which.
///
/// The **wide** frame is swept once, route-free, and that is not a narrowed
/// sweep: `split_body`'s `Wide` arm ignores `route` entirely, `zone` passes
/// `route` nowhere else, and nothing downstream of `zone` reads it — so the two
/// route passes at 120x40 were byte-identical work, not a second set of cells.
/// [`the_wide_layout_resolves_every_cell_route_free`] is the executable licence
/// for that: it asserts the property per cell rather than trusting the layout
/// function's shape.
///
/// **Where the zone comes from.** `mouse_action` returns an `Action` and never
/// surfaces the `Zone` it resolved, so the claim's zone is recovered here by
/// calling `ui::layout::zone` on the same arguments. That recomputation must
/// mirror `mouse_action`'s own precedence, and the two rules are written out
/// rather than left to be rediscovered (`design.md` -> Decision 11): a point
/// outside `area` is `Zone::Outside`, and while the overlay is open the zone is
/// **not consulted at all**, because `mouse_action` returns before reaching it.
/// The frames one fixture is swept over.
///
/// The wide frame once (route-free, licensed by
/// [`the_wide_layout_resolves_every_cell_route_free`]); the narrow frame at both
/// routes, where `route` genuinely selects which single region exists.
///
/// **The second fixture is swept over the wide frame alone**, which is
/// `design.md` -> Risks' own stated lever, applied because the measurement
/// crossed its threshold: three passes per fixture put
/// `cargo test --test doc_contract` at 182 s, past the roughly three minutes
/// recorded there.
///
/// Trimming those passes costs no coverage, and that is **asserted rather than
/// assumed**, because a comment claiming it would be a documented claim with a
/// computable second site and no binding to it — the rule this whole file
/// exists to keep. The two assertions are
/// [`every_zone_is_reached_by_every_fixture`], which compares the fixtures'
/// zone sets, and [`the_trimmed_passes_could_contribute_no_claim`], which
/// observes that every claim the selection-present fixture adds is a
/// `Drag(Left)` claim — `mouse_action` reads `dashboard.selection` in that one
/// arm — and that the wide frame already resolves all six `Zone` variants under
/// that kind, so a narrow pass could only re-observe a pair the wide pass has
/// already contributed.
fn passes_for(fixture: SweepFixture) -> Vec<(Route, u16, u16)> {
    match fixture {
        SweepFixture::SelectionAbsent => vec![
            (Route::List, 120, 40),
            (Route::List, 60, 20),
            (Route::Detail, 60, 20),
        ],
        SweepFixture::SelectionPresent => vec![(Route::List, 120, 40)],
    }
}

fn sweep_mouse_claims(fixture: SweepFixture, help_open: bool) -> BTreeSet<Claim> {
    let mut claims = BTreeSet::new();
    for (route, width, height) in passes_for(fixture) {
        let dashboard = sweep_dashboard(route, help_open, fixture);
        let area = Rect::new(0, 0, width, height);
        for kind in MOUSE_KINDS {
            for row in 0..height {
                for column in 0..width {
                    let mouse = MouseEvent {
                        kind,
                        column,
                        row,
                        modifiers: KeyModifiers::NONE,
                    };
                    let zone = (!help_open).then(|| {
                        zone_name(&herdr_openspec::ui::layout::zone(area, route, column, row))
                    });
                    claims.insert(Claim {
                        kind: kind_name(kind),
                        help_open,
                        zone,
                        outcome: outcome_name(mouse_action(&dashboard, area, &mouse)),
                    });
                }
            }
        }
    }
    claims
}

/// [`sweep_mouse_claims`], computed once per test binary and cloned thereafter,
/// on exactly [`swept_key_action_names`]' terms. Every `(fixture, overlay)`
/// combination is filled on the first touch: the full check wants all of them,
/// and one table keeps the expensive sweep to a single pass per combination.
fn swept_mouse_claims(fixture: SweepFixture, help_open: bool) -> BTreeSet<Claim> {
    static CACHE: std::sync::OnceLock<BTreeMap<(SweepFixture, bool), BTreeSet<Claim>>> =
        std::sync::OnceLock::new();
    CACHE
        .get_or_init(|| {
            let mut table = BTreeMap::new();
            for entry in SWEEP_FIXTURES {
                for open in [false, true] {
                    table.insert((entry, open), sweep_mouse_claims(entry, open));
                }
            }
            table
        })
        .get(&(fixture, help_open))
        .expect("SWEEP_FIXTURES covers every fixture the sweep is asked for")
        .clone()
}

/// The claims observed at one overlay state, over **every** fixture — the
/// left-hand side of the two-way comparison.
fn mouse_claims(help_open: bool) -> BTreeSet<Claim> {
    let mut all = BTreeSet::new();
    for fixture in SWEEP_FIXTURES {
        all.extend(swept_mouse_claims(fixture, help_open));
    }
    all
}

/// The bare action names, as a **projection** of [`mouse_claims`] rather than a
/// second sweep, so the two cannot disagree. Keeps its signature and its
/// `BTreeSet<String>` return: three callers want a name-set view of this data.
fn swept_mouse_action_names(help_open: bool) -> BTreeSet<String> {
    mouse_claims(help_open)
        .into_iter()
        .map(|c| match c.outcome.split_once('(') {
            Some((variant, _)) => variant.to_string(),
            None => c.outcome.to_string(),
        })
        .collect()
}

/// Step 4's left-hand side: the swept union less the closed exemption set.
fn bound_action_names() -> BTreeSet<String> {
    let mut union = swept_action_names();
    for name in EXEMPT_ACTIONS {
        union.remove(name);
    }
    union
}

/// The swept union itself, exemptions still in it — what the shape assertion
/// counts, and the shared setup both sweeps' callers want.
fn swept_action_names() -> BTreeSet<String> {
    let mut union = swept_key_action_names();
    union.extend(swept_mouse_action_names(false));
    union.extend(swept_mouse_action_names(true));
    union
}

/// Step 4's right-hand side: `action_name(binding.action)` over every binding.
fn inventory_action_names() -> BTreeSet<String> {
    INVENTORY
        .iter()
        .flat_map(|group| group.bindings.iter())
        .map(|binding| action_name(binding.action).to_string())
        .collect()
}

/// Leg 1's comparison, failing in **both** directions and naming both sides in
/// each, so the reader is never left to work out which of the two moved.
fn compare_bound_and_documented(
    bound: &BTreeSet<String>,
    documented: &BTreeSet<String>,
) -> Result<(), String> {
    let only_bound: Vec<_> = bound.difference(documented).cloned().collect();
    let only_documented: Vec<_> = documented.difference(bound).cloned().collect();
    if only_bound.is_empty() && only_documented.is_empty() {
        return Ok(());
    }
    let mut msg = String::new();
    if !only_bound.is_empty() {
        msg.push_str(&format!(
            "bound but not documented: {only_bound:?} - produced by ui::app::action_for or \
             ui::driver::mouse_action, named by no row in ui::help::INVENTORY; "
        ));
    }
    if !only_documented.is_empty() {
        msg.push_str(&format!(
            "documented but unreachable: {only_documented:?} - named by a row in \
             ui::help::INVENTORY, produced by no call to ui::app::action_for or \
             ui::driver::mouse_action"
        ));
    }
    Err(msg)
}

/// The atoms one Key cell contributes: the two aliases first (each removed from
/// the cell as it is honoured, so its own backticks cannot be read twice), then
/// every remaining backticked span.
fn key_atoms(cell: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = cell.to_string();
    for (pattern, atoms) in KEY_ALIASES {
        while rest.contains(pattern) {
            rest = rest.replacen(pattern, " ", 1);
            out.extend(atoms.iter().map(|atom| (*atom).to_string()));
        }
    }
    for (index, span) in rest.split('`').enumerate() {
        if index % 2 == 1 {
            let span = span.trim();
            if !span.is_empty() {
                out.push(span.to_string());
            }
        }
    }
    out
}

/// A document's Keys table, reduced to its key atoms, with the row count beside
/// them. The table is located by its own `| Key | Action |` header row inside
/// the named section and bounded to its own **contiguous** `|` rows — never a
/// scan of the whole document, which would collect every backticked identifier
/// in it and pass vacuously in the doc-has-extra direction.
///
/// `Err` names what was missing: a gutted document is a broken control, never
/// an agreement between two empty sets.
fn documented_key_atoms(text: &str, heading: &str) -> Result<(BTreeSet<String>, usize), String> {
    let keys = section(text, heading)?;
    let start = keys.find("| Key | Action |").ok_or_else(|| {
        format!("{heading} holds no key table (no `| Key | Action |` header row)")
    })?;
    let table = &keys[start..];

    let mut atoms = BTreeSet::new();
    let mut rows = 0usize;
    for line in table.lines() {
        if !line.starts_with('|') {
            break;
        }
        rows += 1;
        let cell = line.trim_start_matches('|').split('|').next().unwrap_or("");
        atoms.extend(key_atoms(cell));
    }
    if rows < 3 {
        return Err(format!(
            "{heading}'s key table has {rows} row(s) - a header, a separator, and at least one \
             binding are the minimum"
        ));
    }
    if atoms.is_empty() {
        return Err(format!(
            "{heading}'s key table names no key in backticks - the extraction rule this check \
             depends on is not being followed"
        ));
    }
    Ok((atoms, rows))
}

/// `INVENTORY`'s own key atoms: every non-`Mouse` group's `input`, split on
/// `" / "`, so one overlay row stands for a key and its arrow synonym without
/// forcing the prose to split into two rows.
fn inventory_key_atoms() -> BTreeSet<String> {
    let mut atoms = BTreeSet::new();
    for group in INVENTORY {
        if group.title == "Mouse" {
            continue;
        }
        for binding in group.bindings {
            for atom in binding.input.split(" / ") {
                let atom = atom.trim();
                if !atom.is_empty() {
                    atoms.insert(atom.to_string());
                }
            }
        }
    }
    atoms
}

/// The `input` string an atom came from, so leg 2's and leg 3's failure names
/// the row and not only the atom.
fn inventory_input_for(atom: &str) -> Option<&'static str> {
    INVENTORY
        .iter()
        .filter(|group| group.title != "Mouse")
        .flat_map(|group| group.bindings.iter())
        .find(|binding| binding.input.split(" / ").any(|a| a.trim() == atom))
        .map(|binding| binding.input)
}

/// Legs 2 and 3's comparison. No residual difference is tolerated and none is
/// exempted: the alias table above is the whole of the permitted normalisation.
fn compare_key_atoms(
    document: &str,
    documented: &BTreeSet<String>,
    inventory: &BTreeSet<String>,
) -> Result<(), String> {
    let missing: Vec<_> = inventory.difference(documented).cloned().collect();
    let extra: Vec<_> = documented.difference(inventory).cloned().collect();
    if missing.is_empty() && extra.is_empty() {
        return Ok(());
    }
    let mut msg = format!("{document} disagrees with ui::help::INVENTORY: ");
    if !missing.is_empty() {
        let rows: Vec<String> = missing
            .iter()
            .map(|atom| format!("{atom:?} (INVENTORY input {:?})", inventory_input_for(atom)))
            .collect();
        msg.push_str(&format!(
            "named by the inventory and not by {document}: {}; ",
            rows.join(", ")
        ));
    }
    if !extra.is_empty() {
        msg.push_str(&format!(
            "named by {document} and not by the inventory: {extra:?}"
        ));
    }
    Err(msg)
}

/// Read one `input` back into the `(KeyCode, KeyModifiers)` pairs a reader
/// would press. Small on purpose, and not a second key table: a single
/// character, an `X / Y` pair, a `Ctrl-<c>` form, an `<a>`–`<b>` digit range,
/// and the named keys `Enter`, `Esc`, `Space`, `Backspace`, `↑`, and `↓`.
/// Anything else is an error, not a guess — a silently skipped row is the same
/// unfalsifiable guard this check exists to remove, one level down.
fn parse_input(input: &str) -> Result<Vec<(KeyCode, KeyModifiers)>, String> {
    let unrecognised = || format!("{input:?} is not a recognised key spelling");

    if let Some((left, right)) = input.split_once(" / ") {
        let mut pairs = parse_input(left.trim())?;
        pairs.extend(parse_input(right.trim())?);
        return Ok(pairs);
    }

    if let Some(rest) = input.strip_prefix("Ctrl-") {
        let mut chars = rest.chars();
        return match (chars.next(), chars.next()) {
            (Some(c), None) => Ok(vec![(
                KeyCode::Char(c.to_ascii_lowercase()),
                KeyModifiers::CONTROL,
            )]),
            _ => Err(unrecognised()),
        };
    }

    if let Some((low, high)) = input.split_once('–') {
        let bounds = (one_char(low), one_char(high));
        return match bounds {
            (Some(low), Some(high))
                if low.is_ascii_digit() && high.is_ascii_digit() && low <= high =>
            {
                Ok((low..=high)
                    .map(|c| (KeyCode::Char(c), KeyModifiers::NONE))
                    .collect())
            }
            _ => Err(unrecognised()),
        };
    }

    match input {
        "Enter" => Ok(vec![(KeyCode::Enter, KeyModifiers::NONE)]),
        "Esc" => Ok(vec![(KeyCode::Esc, KeyModifiers::NONE)]),
        "Space" => Ok(vec![(KeyCode::Char(' '), KeyModifiers::NONE)]),
        "Backspace" => Ok(vec![(KeyCode::Backspace, KeyModifiers::NONE)]),
        "↑" => Ok(vec![(KeyCode::Up, KeyModifiers::NONE)]),
        "↓" => Ok(vec![(KeyCode::Down, KeyModifiers::NONE)]),
        _ => match one_char(input) {
            Some(c) => Ok(vec![(KeyCode::Char(c), KeyModifiers::NONE)]),
            None => Err(unrecognised()),
        },
    }
}

/// `Some(c)` when `text` is exactly one `char`, `None` otherwise.
fn one_char(text: &str) -> Option<char> {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    }
}

/// One row's own assertion: every pair its `input` parses to, evaluated under
/// its group's filter mode, returns the action the row claims — compared by the
/// **variant name**, because `1`–`9` parses to nine pairs whose actions differ
/// in their payload and no single `binding.action` can equal all nine.
fn check_binding_row(
    group: &str,
    input: &str,
    filtering: bool,
    expected: &str,
) -> Result<(), String> {
    let pairs = parse_input(input).map_err(|e| format!("{group} -> {e}"))?;
    if pairs.is_empty() {
        return Err(format!(
            "{group} -> {input:?} parsed to no (KeyCode, KeyModifiers) pair"
        ));
    }
    for (code, modifiers) in pairs {
        let got = action_name(action_for(&press(code, modifiers), filtering));
        if got != expected {
            return Err(format!(
                "{group} -> {input:?} claims {expected} but ui::app::action_for returns {got} \
                 for {code:?} with {modifiers:?} (filtering: {filtering})"
            ));
        }
    }
    Ok(())
}

#[test]
fn an_action_added_without_a_help_row_fails() {
    // `action_name`'s exhaustive `match` is what catches a *new variant* — that
    // is a compile error, before any assertion runs, and cannot be exercised at
    // run time. What this test drives is the assertion one step later: the
    // variant is named but `INVENTORY` still holds no row for it.
    let mut bound = bound_action_names();
    bound.insert("Fabricated".to_string());
    let documented = inventory_action_names();

    let err = compare_bound_and_documented(&bound, &documented)
        .expect_err("an action with no inventory row must fail");
    assert!(err.contains("Fabricated"), "{err}");
    assert!(err.contains("bound but not documented"), "{err}");
    assert!(err.contains("ui::help::INVENTORY"), "{err}");
}

#[test]
fn a_binding_removed_from_the_driver_and_left_in_the_help_fails() {
    // The other direction: `INVENTORY` names `Refresh` and no swept call
    // produces it — what a deleted `action_for` arm looks like from here.
    let mut bound = bound_action_names();
    assert!(bound.remove("Refresh"), "the sweep must reach `r` at HEAD");
    let documented = inventory_action_names();

    let err = compare_bound_and_documented(&bound, &documented)
        .expect_err("an inventory row no key reaches must fail");
    assert!(err.contains("Refresh"), "{err}");
    assert!(err.contains("documented but unreachable"), "{err}");
    // Both sides named, so the reader is not left to work out which moved.
    assert!(err.contains("ui::help::INVENTORY"), "{err}");
    assert!(err.contains("ui::app::action_for"), "{err}");
    assert!(err.contains("ui::driver::mouse_action"), "{err}");
}

#[test]
fn sweep_finds_the_bound_actions_and_exactly_two_exemptions() {
    // The substantive claim first, and deliberately: a planted defect in either
    // direction — a deleted `action_for` arm, a removed `Binding` — must report
    // the *action* that moved and which side moved it, not a count that happens
    // to be one short. The bookkeeping below is what pins the numbers.
    let bound = bound_action_names();
    let documented = inventory_action_names();
    compare_bound_and_documented(&bound, &documented).expect("the tree at HEAD agrees");

    // The exemption set is asserted by name and by length, never by a predicate:
    // a third exemption must cost a spec change.
    assert_eq!(EXEMPT_ACTIONS.len(), 2);
    assert_eq!(EXEMPT_ACTIONS, ["FilterPush", "Ignore"]);

    let union = swept_action_names();
    for name in EXEMPT_ACTIONS {
        assert!(union.contains(name), "the sweep must produce {name}");
    }
    assert_eq!(
        union.len(),
        25,
        "the swept union is `Action`'s full membership after `Select`: {union:?}"
    );
    assert_eq!(bound.len(), 23, "{bound:?}");
}

/// The names `mouse_action` produces under a label no key ever produces, but
/// which `every_mouse_action_has_an_equal_key` (`src/ui/driver.rs`) proves
/// reach an identical `Dashboard` through a differently-named key action
/// instead: the four wheel outcomes, each equal to `Next`/`Prev` at the
/// matching route, and `Click`, whose three targets are equal to `Next`
/// (a change row), `OpenDetail` (a second click on the selected row), and
/// `ToggleSection` (a section header or an artifact-section header) in turn.
/// `SelectTab`, `ToggleHelp`, and `Ignore` need no entry here at all: a key
/// produces those same variants directly, so `Dashboard::apply` being a pure
/// function of the `Action` value already guarantees the same effect —
/// nothing to prove beyond the two sets sharing the name.
const PROVEN_EQUIVALENT_UNDER_A_DIFFERENT_NAME: [&str; 5] = [
    "SelectNext",
    "SelectPrev",
    "ScrollDown",
    "ScrollUp",
    "Click",
];

#[test]
fn select_is_the_only_mouse_only_action_by_name_and_count() {
    // `specs/mouse-input/spec.md` -> "`Action::Select` is the only mouse-only
    // action, by name and count". Two ways a mouse-produced name can fail to
    // be mouse-only: a key produces the identical variant (`SelectTab`,
    // `ToggleHelp`, `Ignore` — caught by `swept_key_action_names`), or a key
    // produces a *different* variant proven to reach the same `Dashboard`
    // (the five names above, proven in `src/ui/driver.rs`). What is left after
    // removing both SHALL be exactly one name, `Select`, asserted by name and
    // by length exactly as `EXEMPT_ACTIONS` is, so a second mouse-only action
    // fails here rather than passing silently.
    let mut mouse_only = swept_mouse_action_names(false);
    mouse_only.extend(swept_mouse_action_names(true));
    for name in swept_key_action_names() {
        mouse_only.remove(&name);
    }
    for name in PROVEN_EQUIVALENT_UNDER_A_DIFFERENT_NAME {
        mouse_only.remove(name);
    }
    assert_eq!(
        mouse_only,
        BTreeSet::from(["Select".to_string()]),
        "the mouse-only set must hold exactly one name, Select: {mouse_only:?}"
    );
}

#[test]
fn no_key_reaches_select_at_either_filter_mode() {
    // `specs/dashboard-loop/spec.md` -> "No key reaches `Select` at either
    // filter mode". `swept_key_action_names` already calls `action_for` over
    // every key this crate binds under both `filtering` states and merges the
    // two into one set (`sweep_key_actions`) - this is the assertion that was
    // missing: that `Select` never lands in it, so the mouse-only exemption is
    // real rather than merely asserted in prose.
    let names = swept_key_action_names();
    assert!(
        !names.contains("Select"),
        "no key may produce Action::Select: {names:?}"
    );
}

#[test]
fn the_wide_layout_resolves_every_cell_route_free() {
    // What this protects: `swept_mouse_action_names` sweeps the **wide** frame
    // once rather than once per route, on the ground that `route` cannot reach
    // the answer there. That is the whole of the licence, and it is asserted
    // per cell rather than inferred from `split_body`'s shape — a future change
    // that branched on `route` further down `zone`'s body would satisfy
    // `split_body(body, List) == split_body(body, Detail)` and still break the
    // sweep, which is why that weaker proxy is not what is written here.
    //
    // If this goes red, the fix is to restore the second route pass in
    // `sweep_mouse_actions` — roughly doubling that sweep's cost — and NOT to
    // delete or weaken this assertion. The cell coverage is the point; the
    // single pass is only an optimisation this property pays for.
    let area = Rect::new(0, 0, 120, 40);
    for row in 0..area.height {
        for column in 0..area.width {
            let at_list = herdr_openspec::ui::layout::zone(area, Route::List, column, row);
            let at_detail = herdr_openspec::ui::layout::zone(area, Route::Detail, column, row);
            assert_eq!(
                at_list, at_detail,
                "the wide layout became route-dependent at ({column}, {row}): \
                 Route::List resolves to {at_list:?} and Route::Detail to {at_detail:?} - \
                 restore the second route pass in sweep_mouse_actions"
            );
        }
    }
}

#[test]
fn the_sweep_covers_the_mouse_under_both_overlay_states() {
    let closed = swept_mouse_action_names(false);
    let expected_closed: BTreeSet<String> = [
        "SelectNext",
        "SelectPrev",
        "ScrollDown",
        "ScrollUp",
        "SelectTab",
        "Click",
        "Select",
        "Ignore",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    assert_eq!(
        closed, expected_closed,
        "a dashboard whose selected change carries no artifact tabs yields six \
         names and silently drops the mouse's tab-switching coverage - widen the \
         fixture, never narrow this set"
    );

    let open = swept_mouse_action_names(true);
    let expected_open: BTreeSet<String> = ["ScrollDown", "ScrollUp", "ToggleHelp", "Ignore"]
        .into_iter()
        .map(str::to_string)
        .collect();
    assert_eq!(open, expected_open);

    // The union names `ToggleHelp`, so the click-outside dismissal is bound here
    // and not only by its own scenarios.
    assert!(closed.union(&open).any(|n| n == "ToggleHelp"));
}

#[test]
fn the_sweep_records_all_four_claim_axes() {
    // What this protects: that the claim a cell contributes carries the zone it
    // resolved to and the outcome `mouse_action` returned, correlated with each
    // other. A sweep that recorded the right zones and the right outcomes but
    // paired them wrongly would satisfy any assertion written over the two
    // projections separately, which is why the third assertion below is an
    // **absence**: `Zone::ListRow` under the wheel is `SelectNext`, never
    // `ScrollDown`, and a zone/outcome mix-up says otherwise.
    let claims = swept_mouse_claims(SweepFixture::SelectionAbsent, false);

    assert!(
        claims.contains(&claim("ScrollDown", false, Some("ListRow"), "SelectNext")),
        "the wheel over a list row moves the list selection: {claims:?}"
    );
    assert!(
        claims.contains(&claim("Down(Left)", false, Some("DetailTab"), "SelectTab")),
        "a left press on a tab cell switches the tab: {claims:?}"
    );
    assert!(
        !claims.contains(&claim("ScrollDown", false, Some("ListRow"), "ScrollDown")),
        "the wheel over a list row must not be recorded as scrolling the detail \
         region - the zone and the outcome have been paired wrongly"
    );
}

#[test]
fn the_overlay_state_is_its_own_axis() {
    // `design.md` -> Decision 6: while the overlay is open `mouse_action`
    // returns before consulting `ui::layout::zone` at all, so an open-overlay
    // claim has no zone to carry. The axis is its own field rather than a
    // sentinel written into the zone slot, so the two passes stay disjoint.
    let open = swept_mouse_claims(SweepFixture::SelectionAbsent, true);
    let closed = swept_mouse_claims(SweepFixture::SelectionAbsent, false);

    assert!(
        open.iter().all(|c| c.help_open && c.zone.is_none()),
        "every open-overlay claim carries the overlay axis and no zone: {open:?}"
    );
    assert!(
        closed.iter().all(|c| !c.help_open && c.zone.is_some()),
        "every closed-overlay claim carries a zone: {closed:?}"
    );
    assert!(
        open.contains(&claim("Down(Left)", true, None, "ToggleHelp")),
        "the press outside the band dismisses the overlay: {open:?}"
    );
    assert!(
        !closed.iter().any(|c| c.outcome == "ToggleHelp"),
        "no closed-overlay cell produces ToggleHelp: {closed:?}"
    );
    assert!(
        open.contains(&claim("ScrollDown", true, None, "ScrollDown")),
        "the wheel scrolls the overlay from anywhere in the frame: {open:?}"
    );
}

#[test]
fn a_point_outside_the_frame_resolves_to_outside_under_both_overlay_states() {
    // `design.md` -> Decision 11's **second** precedence rule. The sweep cannot
    // exercise it: it visits `0..height` x `0..width` and so never leaves the
    // frame. `Zone::Outside` does appear in the claim set, but from in-frame
    // chrome - the footer row - so the sweep's six-zone assertion would go on
    // passing if this rule broke. This is the rule's own exercise, and it is
    // cheap: a handful of points rather than a fourth sweep.
    let area = Rect::new(0, 0, 120, 40);
    let outside = [(120u16, 0u16), (0, 40), (200, 200), (u16::MAX, u16::MAX)];
    for route in [Route::List, Route::Detail] {
        for (column, row) in outside {
            assert_eq!(
                herdr_openspec::ui::layout::zone(area, route, column, row),
                Zone::Outside,
                "({column}, {row}) is outside a 120x40 frame"
            );
            for (fixture, help_open) in [
                (SweepFixture::SelectionAbsent, false),
                (SweepFixture::SelectionAbsent, true),
                (SweepFixture::SelectionPresent, false),
                (SweepFixture::SelectionPresent, true),
            ] {
                let dashboard = sweep_dashboard(route, help_open, fixture);
                for kind in MOUSE_KINDS {
                    let mouse = MouseEvent {
                        kind,
                        column,
                        row,
                        modifiers: KeyModifiers::NONE,
                    };
                    // The one exception, and it is documented rather than
                    // excused: a left drag extending a selection already in
                    // progress clamps to the content area's nearest edge from
                    // anywhere, `Zone::Outside` included, which is why the
                    // table's drag row names that zone. Everything else outside
                    // the frame is not a gesture the pane received.
                    let clamps = !help_open
                        && fixture == SweepFixture::SelectionPresent
                        && kind == MouseEventKind::Drag(MouseButton::Left);
                    let expected = if clamps { "Select(Extend)" } else { "Ignore" };
                    assert_eq!(
                        outcome_name(mouse_action(&dashboard, area, &mouse)),
                        expected,
                        "{kind:?} at ({column}, {row}), overlay open = {help_open}, \
                         fixture = {fixture:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn the_trimmed_passes_could_contribute_no_claim() {
    // [`passes_for`]'s full licence. Sweeping the selection-present fixture over
    // the narrow frames as well yields three further claims - `Drag(Left)` over
    // `List`, `ListRow` and `Outside` resolving to `Ignore` - and the union
    // `all_mouse_claims()` is the same either way. This binds that rather than
    // asserting it in a comment, and does so out of sweeps already paid for.
    //
    // If this goes red, the fix is to restore the narrow passes in `passes_for`
    // and NOT to weaken the assertion.
    let absent = swept_mouse_claims(SweepFixture::SelectionAbsent, false);
    let present = swept_mouse_claims(SweepFixture::SelectionPresent, false);

    // `mouse_action` reads `dashboard.selection` in exactly one arm
    // (`src/ui/driver.rs`, the `Drag(MouseButton::Left)` arm), **observed** here
    // rather than taken on trust: every claim the second fixture adds is one.
    let added: Vec<&Claim> = present.difference(&absent).collect();
    assert!(!added.is_empty(), "the second fixture must add something");
    assert!(
        added.iter().all(|c| c.kind == "Drag(Left)"),
        "only the Drag(Left) arm reads dashboard.selection, so only Drag(Left) \
         claims can differ between the fixtures: {added:?}"
    );

    // And at the wide frame that fixture already resolves all six zones under
    // that one kind, so a narrow pass has no (kind, zone) pair left to reach.
    let dragged: BTreeSet<&str> = present
        .iter()
        .filter(|c| c.kind == "Drag(Left)")
        .filter_map(|c| c.zone)
        .collect();
    assert_eq!(
        dragged.len(),
        ZONE_VARIANTS.len(),
        "the wide frame reaches {} of the six Zone variants under Drag(Left): {dragged:?}",
        dragged.len()
    );
}

#[test]
fn the_clamp_is_observed_under_a_selection_fixture() {
    // `design.md` -> Decision 9. `mouse_action` is a total function of the whole
    // `Dashboard`: its `Drag(MouseButton::Left)` arm returns `Select(Extend)`
    // from every zone but `DetailRow` only while `dashboard.selection.is_some()`
    // (`src/ui/driver.rs`), and the sweep's original fixture pins
    // `selection: None`. So the clamp `SPEC.md`'s drag row documents is
    // unobservable under that fixture alone, the two-way check would report that
    // row vacuous, and it would be right to.
    let clamp = claim("Drag(Left)", false, Some("List"), "Select(Extend)");

    assert!(
        mouse_claims(false).contains(&clamp),
        "the clamp must be observable under some fixture the sweep runs"
    );
    assert!(
        !swept_mouse_claims(SweepFixture::SelectionAbsent, false).contains(&clamp),
        "the selection-absent fixture alone cannot reach the clamp - which is the \
         state this check would have shipped in without the fixture axis"
    );
    assert_eq!(
        SWEEP_FIXTURES.len(),
        2,
        "the fixture set is pinned by length on EXEMPT_ACTIONS' terms: at minimum \
         one with no selection and one with a selection present"
    );
}

#[test]
fn every_zone_is_reached_by_every_fixture() {
    // The licence for [`passes_for`]'s trim: the selection-present fixture is
    // swept over the wide frame alone, and that is sound only because the wide
    // frame already resolves every `Zone` variant the narrow passes would. This
    // asserts it per fixture rather than inferring it from the layout's shape.
    //
    // If this goes red, the fix is to restore the narrow passes for that fixture
    // — paying the wall time `design.md` -> Risks budgets for — and NOT to
    // weaken this assertion.
    let mut sets = Vec::new();
    for fixture in SWEEP_FIXTURES {
        let zones: BTreeSet<&str> = swept_mouse_claims(fixture, false)
            .iter()
            .filter_map(|c| c.zone)
            .collect();
        assert_eq!(
            zones.len(),
            6,
            "{fixture:?} reaches {} of the six Zone variants: {zones:?}",
            zones.len()
        );
        sets.push(zones);
    }
    assert!(
        sets.windows(2).all(|w| w[0] == w[1]),
        "every fixture must resolve the same six zones: {sets:?}"
    );
}

#[test]
fn a_binding_added_to_the_driver_and_not_to_the_docs_fails_make_check() {
    // Leg 1: bound, not documented.
    let mut bound = bound_action_names();
    bound.insert("Fabricated".to_string());
    let err = compare_bound_and_documented(&bound, &inventory_action_names())
        .expect_err("leg 1 fails first");
    assert!(
        err.contains("Fabricated") && err.contains("ui::app::action_for"),
        "{err}"
    );

    // Leg 2: the inventory row exists, `SPEC.md` -> Keys does not name the key.
    let inventory: BTreeSet<String> = ["q".to_string(), "F13".to_string()].into_iter().collect();
    let documented: BTreeSet<String> = ["q".to_string()].into_iter().collect();
    let err = compare_key_atoms("SPEC.md -> Keys", &documented, &inventory)
        .expect_err("leg 2 fails when the document omits a key");
    assert!(
        err.contains("F13") && err.contains("SPEC.md -> Keys"),
        "{err}"
    );

    // Leg 3: the same terms, against the other document.
    let err = compare_key_atoms("README.md -> Keys", &documented, &inventory)
        .expect_err("leg 3 fails on the same terms");
    assert!(
        err.contains("F13") && err.contains("README.md -> Keys"),
        "{err}"
    );
}

#[test]
fn the_documented_key_set_and_the_inventory_agree_at_head() {
    // Exactly two normalisations, named in this file's own source and pinned at
    // two on the same terms the exemption set is.
    assert_eq!(KEY_ALIASES.len(), 2);

    let spec_md = read_doc(&manifest_dir().join("SPEC.md")).expect("read SPEC.md");
    let readme = read_doc(&manifest_dir().join("README.md")).expect("read README.md");

    let inventory = inventory_key_atoms();
    assert_eq!(
        inventory.len(),
        20,
        "the non-mouse groups hold twenty distinct key atoms: {inventory:?}"
    );

    let (spec_atoms, spec_rows) =
        documented_key_atoms(&spec_md, "### Keys").expect("SPEC.md -> Keys' key table");
    assert!(
        spec_rows >= 1,
        "leg 2's extraction reports at least one row"
    );
    compare_key_atoms("SPEC.md -> Keys", &spec_atoms, &inventory).expect("leg 2 agrees at HEAD");

    let (readme_atoms, readme_rows) =
        documented_key_atoms(&readme, "## Keys").expect("README.md -> Keys' key table");
    assert!(
        readme_rows >= 1,
        "leg 3's extraction reports at least one row"
    );
    compare_key_atoms("README.md -> Keys", &readme_atoms, &inventory)
        .expect("leg 3 agrees at HEAD");
}

#[test]
fn the_key_legs_are_unaffected_by_the_mouse_tables_stronger_binding() {
    // `doc-conformance` -> "The key legs are unaffected by the mouse table's
    // stronger binding". The mouse table is now bound by executing
    // `mouse_action`; legs 2 and 3 still compare **key** atoms against
    // `INVENTORY`, and the `Mouse` group is still excluded from that comparison
    // because `README.md` documents no gesture at all.
    let spec_md = read_doc(&manifest_dir().join("SPEC.md")).expect("read SPEC.md");
    let readme = read_doc(&manifest_dir().join("README.md")).expect("read README.md");
    let inventory = inventory_key_atoms();

    let (spec_atoms, _) =
        documented_key_atoms(&spec_md, "### Keys").expect("SPEC.md -> Keys' key table");
    compare_key_atoms("SPEC.md -> Keys", &spec_atoms, &inventory).expect("leg 2 still agrees");
    let (readme_atoms, _) =
        documented_key_atoms(&readme, "## Keys").expect("README.md -> Keys' key table");
    compare_key_atoms("README.md -> Keys", &readme_atoms, &inventory).expect("leg 3 still agrees");

    // The exclusion, asserted directly rather than left to be inferred from the
    // pinned atom count in `the_documented_key_set_and_the_inventory_agree_at_head`.
    // Non-vacuous: the group exists and carries bindings.
    let mouse_inputs: Vec<&str> = INVENTORY
        .iter()
        .filter(|group| group.title == "Mouse")
        .flat_map(|group| group.bindings.iter())
        .map(|binding| binding.input)
        .collect();
    assert!(
        !mouse_inputs.is_empty(),
        "INVENTORY carries a `Mouse` group, so excluding it is not vacuous"
    );

    // And load-bearing: folding it back in makes both legs fail, which is what
    // says the exclusion is doing work rather than merely being harmless.
    let mut with_mouse = inventory.clone();
    for input in &mouse_inputs {
        for atom in input.split(" / ") {
            let atom = atom.trim();
            if !atom.is_empty() {
                with_mouse.insert(atom.to_string());
            }
        }
    }
    assert!(
        with_mouse.len() > inventory.len(),
        "the `Mouse` group contributes atoms of its own: {mouse_inputs:?}"
    );
    assert!(
        compare_key_atoms("SPEC.md -> Keys", &spec_atoms, &with_mouse).is_err(),
        "leg 2 compares key atoms only - the mouse table has its own, differently \
         grammared check"
    );
    assert!(
        compare_key_atoms("README.md -> Keys", &readme_atoms, &with_mouse).is_err(),
        "leg 3 compares key atoms only - README.md documents no gesture at all"
    );
}

#[test]
fn a_gutted_document_fails_as_a_broken_control_rather_than_a_clean_tree() {
    // The section is gone.
    let no_section = "## Overview\n\nnothing here\n";
    let err =
        documented_key_atoms(no_section, "### Keys").expect_err("a missing section is an error");
    assert!(err.contains("### Keys"), "{err}");

    // The section is there and the key table is not.
    let no_table = "### Keys\n\nsome prose, no table\n\n### Next\n";
    let err = documented_key_atoms(no_table, "### Keys").expect_err("a missing table is an error");
    assert!(err.contains("no key table"), "{err}");

    // The table is reduced to a header row and a separator row.
    let gutted = "### Keys\n\n| Key | Action |\n|---|---|\n\n### Next\n";
    let err = documented_key_atoms(gutted, "### Keys").expect_err("an emptied table is an error");
    assert!(err.contains("row(s)"), "{err}");

    // A table whose Key cells carry no backticked span at all.
    let unnamed = "### Keys\n\n| Key | Action |\n|---|---|\n| the any key | quits |\n";
    let err = documented_key_atoms(unnamed, "### Keys").expect_err("no atom is an error");
    assert!(err.contains("names no key"), "{err}");

    // And the bounding: a whole-document scan would collect every backticked
    // identifier below the table and pass vacuously in the doc-has-extra
    // direction. The extraction stops at the table's own last `|` row.
    let bounded = "### Keys\n\n| Key | Action |\n|---|---|\n| `q` | Quit |\n\n\
                   `ui::view` and `src/ui/help.rs` are named in this prose\n";
    let (atoms, rows) = documented_key_atoms(bounded, "### Keys").expect("parse");
    assert_eq!(atoms, ["q".to_string()].into_iter().collect());
    assert_eq!(rows, 3);
}

#[test]
fn a_row_naming_the_wrong_key_fails() {
    // The `r` row's `input` changed to `k`, its `action` left as `Refresh`.
    let err = check_binding_row("Pane", "k", false, "Refresh")
        .expect_err("a row naming the wrong key must fail");
    assert!(err.contains("Refresh"), "{err}");
    assert!(err.contains("Prev"), "{err}");
    assert!(err.contains('k'), "{err}");

    // And the action-set check still passes on that same tree, which is
    // precisely why this second check exists: `Refresh` is still named once.
    compare_bound_and_documented(&bound_action_names(), &inventory_action_names())
        .expect("the action-set check is blind to a wrong key");
}

#[test]
fn every_non_mouse_row_parses_and_agrees_at_head() {
    let mut checked = 0usize;
    for group in INVENTORY {
        if group.title == "Mouse" {
            continue;
        }
        let filtering = matches!(group.scope, Scope::Filter);
        for binding in group.bindings {
            let pairs = parse_input(binding.input)
                .unwrap_or_else(|e| panic!("{} -> {:?}: {e}", group.title, binding.input));
            assert!(
                !pairs.is_empty(),
                "{} -> {:?} parsed to no key",
                group.title,
                binding.input
            );
            check_binding_row(
                group.title,
                binding.input,
                filtering,
                action_name(binding.action),
            )
            .expect("every row agrees with the driver at HEAD");
            checked += 1;
        }
    }
    assert_eq!(checked, 25, "five non-mouse groups, 5 + 7 + 4 + 4 + 5 rows");
}

#[test]
fn an_unparseable_spelling_fails_rather_than_skipping() {
    let err = parse_input("the any key").expect_err("an unrecognised spelling is an error");
    assert!(err.contains("the any key"), "{err}");

    // And it reaches the per-row check as a failure, never as a skipped row or
    // a row treated as a mouse gesture.
    let err = check_binding_row("Pane", "the any key", false, "Refresh")
        .expect_err("an unparseable row fails the per-row check");
    assert!(err.contains("the any key"), "{err}");

    // A mouse spelling in a non-mouse group is unparseable too.
    assert!(parse_input("Wheel ↓").is_err());
    assert!(parse_input("Click").is_err());
}

/// The names `specs/doc-conformance/spec.md`'s Requirement forbids anywhere in
/// `src/specs.rs`'s production slice: filesystem, process, environment, network, and
/// standard-I/O names, plus every schema-reading name.
const SPECS_RS_FORBIDDEN_NEEDLES: [&str; 12] = [
    "std::fs",
    "std::io",
    "std::env",
    "std::process",
    "std::net",
    "File::",
    "read_to_string",
    "Command",
    "schema::",
    "Schema",
    "config.yaml",
    ".openspec.yaml",
];

/// Whether `src`'s production slice — the text above its first line-anchored
/// `#[cfg(test)]`, exactly as [`production_slice`] cuts it — names any of
/// [`SPECS_RS_FORBIDDEN_NEEDLES`]. `Err` names the first needle found and its 1-based
/// line number. The slice is asserted non-empty before it is searched, so the check
/// cannot pass vacuously against a file it failed to read or cut at the wrong place.
fn specs_rs_production_slice_is_io_free(src: &str) -> Result<(), String> {
    let prod = production_slice(src);
    if prod.is_empty() {
        return Err(
            "src/specs.rs's production slice is empty — cut at the wrong place, or the \
             file itself has none"
                .to_string(),
        );
    }
    for (idx, line) in prod.lines().enumerate() {
        for needle in SPECS_RS_FORBIDDEN_NEEDLES {
            if line.contains(needle) {
                return Err(format!(
                    "src/specs.rs's production slice names {needle:?} at line {}: {line}",
                    idx + 1
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn specs_rs_production_slice_check_passes_on_a_clean_slice() {
    let src =
        "//! docs\npub fn a() -> u8 { 1 }\n\n#[cfg(test)]\nmod tests {\n    use std::fs;\n}\n";
    specs_rs_production_slice_is_io_free(src).expect("no needle above the cut");
}

#[test]
fn specs_rs_production_slice_check_fails_naming_needle_and_line() {
    let src = "pub fn a() {}\nuse std::fs;\n\n#[cfg(test)]\nmod tests {}\n";
    let err = specs_rs_production_slice_is_io_free(src).expect_err("std::fs above the cut fails");
    assert!(err.contains("std::fs"), "{err}");
    assert!(err.contains("line 2"), "{err}");
}

#[test]
fn specs_rs_production_slice_check_ignores_a_needle_below_the_cut() {
    // The complement: an I/O name in the test module alone, with none above the cut,
    // must not fail the claim — the slice boundary is load-bearing, not an exemption.
    let src = "pub fn a() {}\n\n#[cfg(test)]\nmod tests {\n    use std::fs;\n    fn t() {\n        \
               let _ = std::fs::read_to_string(\"x\");\n    }\n}\n";
    specs_rs_production_slice_is_io_free(src).expect("a needle only below the cut must not fail");
}

#[test]
fn specs_rs_production_slice_check_rejects_an_empty_slice() {
    let err = specs_rs_production_slice_is_io_free("#[cfg(test)]\nmod tests {}\n")
        .expect_err("an empty production slice must fail rather than pass vacuously");
    assert!(err.contains("empty"), "{err}");
}

/// `specs-emphasis` :: "The production slice of `src/specs.rs` carries no I/O or
/// schema name" (`specs/doc-conformance/spec.md:39`) — the eleventh
/// `tests/doc_contract.rs` claim. Decision 1 in `design.md` puts `src/specs.rs` outside
/// `src/ui/`, where no `scripts/gates/` script sweeps it; this claim is the check the
/// property it rests on would otherwise have gone without.
#[test]
fn the_production_slice_of_src_specs_rs_carries_no_io_or_schema_name() {
    let src = read_doc(&manifest_dir().join("src/specs.rs")).expect("read src/specs.rs");
    specs_rs_production_slice_is_io_free(&src)
        .expect("src/specs.rs's production slice names no I/O or schema-reading API");
}

// ---------------------------------------------------------------------------
// `agent-client-choice` :: "The production slice of `src/integration.rs` carries no I/O
// or view name" — the **fourteenth** `tests/doc_contract.rs` claim, on exactly
// `src/specs.rs`' terms: design.md -> Boundaries puts `src/integration.rs` outside
// `src/ui/`, where **no** `scripts/gates/` script sweeps it at all. `LAUNCHSEAM` covers
// the Herdr handle and nothing else does, so `ratatui` joins the needle set here.
// ---------------------------------------------------------------------------

/// The names forbidden anywhere in `src/integration.rs`'s production slice: filesystem,
/// process, environment, network, and standard-I/O names, plus every `ratatui` type this
/// pure classifier must not reach for. `ratatui` is in the set because no `make gates`
/// script sweeps this file, unlike every pure file under `src/ui/`.
const INTEGRATION_RS_FORBIDDEN_NEEDLES: [&str; 13] = [
    "std::fs",
    "std::io",
    "std::env",
    "std::process",
    "std::net",
    "File::",
    "read_to_string",
    "Command",
    "ratatui",
    "Modifier",
    "Style",
    "Span",
    "Buffer",
];

/// Whether `src`'s production slice names any of [`INTEGRATION_RS_FORBIDDEN_NEEDLES`].
/// `Err` names the first needle found and its 1-based line number. The slice is asserted
/// non-empty before it is searched, so the check cannot pass vacuously against a file it
/// failed to read or cut at the wrong place.
fn integration_rs_production_slice_is_io_free(src: &str) -> Result<(), String> {
    let prod = production_slice(src);
    if prod.is_empty() {
        return Err(
            "src/integration.rs's production slice is empty — cut at the wrong place, or \
             the file itself has none"
                .to_string(),
        );
    }
    for (idx, line) in prod.lines().enumerate() {
        for needle in INTEGRATION_RS_FORBIDDEN_NEEDLES {
            if line.contains(needle) {
                return Err(format!(
                    "src/integration.rs's production slice names {needle:?} at line {}: {line}",
                    idx + 1
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn integration_rs_production_slice_check_passes_on_a_clean_slice() {
    let src = "//! docs
pub fn a() -> u8 { 1 }

#[cfg(test)]
mod tests {
    use std::fs;
}
";
    integration_rs_production_slice_is_io_free(src).expect("no needle above the cut");
}

#[test]
fn integration_rs_production_slice_check_fails_naming_needle_and_line() {
    let src = "pub fn a() {}
use std::fs;

#[cfg(test)]
mod tests {}
";
    let err =
        integration_rs_production_slice_is_io_free(src).expect_err("std::fs above the cut fails");
    assert!(err.contains("std::fs"), "{err}");
    assert!(err.contains("line 2"), "{err}");

    // The needle `ratatui` is load-bearing here and nowhere else: no gate script sweeps
    // this file, so a view type reaching it would otherwise go unnoticed.
    let src = "use ratatui::style::Style;

#[cfg(test)]
mod tests {}
";
    let err = integration_rs_production_slice_is_io_free(src)
        .expect_err("a ratatui name above the cut fails");
    assert!(err.contains("ratatui"), "{err}");
}

#[test]
fn integration_rs_production_slice_check_rejects_an_empty_slice() {
    let err = integration_rs_production_slice_is_io_free("#[cfg(test)]\nmod tests {}\n")
        .expect_err("an empty production slice must fail rather than pass vacuously");
    assert!(err.contains("empty"), "{err}");
}

#[test]
fn the_production_slice_of_src_integration_rs_carries_no_io_or_view_name() {
    let src =
        read_doc(&manifest_dir().join("src/integration.rs")).expect("read src/integration.rs");
    integration_rs_production_slice_is_io_free(&src)
        .expect("src/integration.rs's production slice names no I/O or ratatui API");
}

// ---------------------------------------------------------------------------
// `agent-prompts` :: "No production file still produces an `/opsx:` prompt" — the
// **fifteenth** claim. `agent-client-choice` removed the three Claude Code slash-command
// prompts; `grep -rn opsx scripts/gates/ tests/ Makefile` matched nothing before this
// claim, so no part of `make check` swept for them and the removal rested on a one-off
// shell command leaving no committed guard.
// ---------------------------------------------------------------------------

/// Whether any production slice under `src/` holds the literal `/opsx:`. `Err` names every
/// file and line that does. `files` is `(path, contents)` pairs, and an empty list is an
/// error: a sweep that reached no file proves nothing.
fn no_production_slice_names_opsx(files: &[(String, String)]) -> Result<(), String> {
    const NEEDLE: &str = "/opsx:";
    if files.is_empty() {
        return Err("the /opsx: sweep reached no file at all".to_string());
    }
    let mut hits = Vec::new();
    for (path, src) in files {
        for (idx, line) in production_slice(src).lines().enumerate() {
            if line.contains(NEEDLE) {
                hits.push(format!("{path}:{}: {}", idx + 1, line.trim()));
            }
        }
    }
    if hits.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} production slice(s) still name {NEEDLE:?}:\n{}",
            hits.len(),
            hits.join("\n")
        ))
    }
}

#[test]
fn the_opsx_sweep_finds_a_planted_literal_and_ignores_one_below_the_cut() {
    let clean = vec![(
        "src/a.rs".to_string(),
        "pub fn a() {}\n\n#[cfg(test)]\nmod tests {\n    const X: &str = \"/opsx:apply\";\n}\n"
            .to_string(),
    )];
    no_production_slice_names_opsx(&clean).expect("a literal below the cut is not a hit");

    let planted = vec![(
        "src/a.rs".to_string(),
        "pub const P: &str = \"/opsx:apply\";\n\n#[cfg(test)]\nmod tests {}\n".to_string(),
    )];
    let err = no_production_slice_names_opsx(&planted).expect_err("a planted literal fails");
    assert!(err.contains("src/a.rs:1"), "{err}");
    assert!(err.contains("/opsx:"), "{err}");

    let err = no_production_slice_names_opsx(&[]).expect_err("an empty sweep must fail");
    assert!(err.contains("no file at all"), "{err}");
}

#[test]
fn no_production_file_still_produces_an_opsx_prompt() {
    let files = collect_rs_files(&manifest_dir().join("src"));
    assert!(
        files.len() >= 25,
        "the sweep must reach the whole crate, found {}",
        files.len()
    );
    no_production_slice_names_opsx(&files)
        .expect("the three Claude Code slash-command prompts are gone from every production slice");
}

// ---------------------------------------------------------------------------
// `mouse-text-selection` :: "The clipboard write's confinement is bound inside
// `cargo test`" (`specs/doc-conformance/spec.md`) — the twelfth `tests/doc_contract.rs`
// claim. `TerminalOps::write_clipboard` (`src/ui/terminal.rs`) is the crate's only
// producer of the OSC 52 introducer `]52;`, and `NORAW-GREP`'s `make gates` script does
// not sweep for it — this leg is the check that property would otherwise have gone
// without, on the same terms the eleventh claim above states for `src/specs.rs`.
// ---------------------------------------------------------------------------

/// The single source of truth both `AGENTS.md` and `SPEC.md` are checked against below —
/// a documented number is never trusted on its own, only compared to this. Bump it, and
/// both prose sites, in the same commit that adds a sixteenth claim.
const CLAIM_COUNT: usize = 15;

/// The number words `agents_md_claim_count` accepts. `ten` is kept alongside the three
/// values this repository has actually used so the negative-control test below has a
/// fourth, distinct value to assert is parsed correctly without yet being correct.
const CLAIM_COUNT_WORDS: [(&str, usize); 6] = [
    ("ten", 10),
    ("eleven", 11),
    ("twelve", 12),
    ("thirteen", 13),
    ("fourteen", 14),
    ("fifteen", 15),
];

/// Parse `AGENTS.md`'s "(<number-word> further claims" marker — the sentence naming how
/// many claims `tests/doc_contract.rs` carries beside `tests/manifest.rs` and
/// `tests/degraded_coverage.rs`. `Err` when the marker is absent, or its number word is
/// not one this parser recognises.
fn agents_md_claim_count(agents_md: &str) -> Result<usize, String> {
    const MARKER: &str = "further claims";
    let idx = agents_md
        .find(MARKER)
        .ok_or_else(|| "AGENTS.md names no \"further claims\"".to_string())?;
    let before = agents_md[..idx].trim_end();
    for (word, value) in CLAIM_COUNT_WORDS {
        if before.ends_with(word) {
            return Ok(value);
        }
    }
    Err(format!(
        "AGENTS.md's \"further claims\" marker is preceded by an unrecognised number word: \
         {before:?}"
    ))
}

/// The number of claims `SPEC.md`'s "### Doc-conformance checks" section lists: one per
/// `- ` bullet up to the next `###` heading, excluding the closing "A claim with no second
/// site is argued in review, not checked." line, which is a statement about the section
/// rather than a claim it lists. `Err` when the heading itself is missing, so a renamed
/// or removed section fails loudly rather than comparing an empty count to `CLAIM_COUNT`.
fn spec_md_doc_conformance_claim_count(spec_md: &str) -> Result<usize, String> {
    const HEADING: &str = "### Doc-conformance checks";
    const META_LINE: &str = "A claim with no second site is argued in review, not checked.";
    let idx = spec_md
        .find(HEADING)
        .ok_or_else(|| "SPEC.md has no \"### Doc-conformance checks\" section".to_string())?;
    let mut count = 0;
    for line in spec_md[idx + HEADING.len()..].lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("###") {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("- ")
            && rest.trim() != META_LINE
        {
            count += 1;
        }
    }
    Ok(count)
}

#[test]
fn agents_md_claim_count_parses_and_fails_loudly() {
    assert_eq!(
        agents_md_claim_count("this crate carries (ten further claims — a, b, c)."),
        Ok(10)
    );
    assert_eq!(
        agents_md_claim_count("(twelve further claims — a, b, c)"),
        Ok(12)
    );
    let err = agents_md_claim_count("no such phrase here").expect_err("no marker is an error");
    assert!(err.contains("further claims"), "{err}");

    let err = agents_md_claim_count("(nine further claims)")
        .expect_err("an unrecognised number word is an error");
    assert!(err.contains("nine"), "{err}");
}

#[test]
fn spec_md_doc_conformance_claim_count_counts_bullets_and_skips_the_meta_line() {
    let doc = "### Doc-conformance checks\n\n\
               - First claim.\n\
               - Second claim.\n\
               - A claim with no second site is argued in review, not checked.\n\n\
               ### Gates\n\n\
               - Not counted at all.\n";
    assert_eq!(spec_md_doc_conformance_claim_count(doc), Ok(2));

    let err = spec_md_doc_conformance_claim_count("# SPEC\n\nno such section\n")
        .expect_err("a missing heading is an error");
    assert!(err.contains("Doc-conformance checks"), "{err}");
}

/// `mouse-text-selection` :: "The documented claim count matches the file" — `AGENTS.md`'s
/// "further claims" marker and `SPEC.md`'s "Doc-conformance checks" bullet list are each
/// bound to [`CLAIM_COUNT`], rather than trusted against one another or left as prose:
/// neither site was machine-bound before this change, which is why they had already
/// drifted (`SPEC.md`'s list was missing the eleventh claim, `src/specs.rs`'s purity,
/// entirely).
#[test]
fn documented_claim_count_matches_the_file() {
    let agents_md = read_doc(&manifest_dir().join("AGENTS.md")).expect("read AGENTS.md");
    let spec_md = read_doc(&manifest_dir().join("SPEC.md")).expect("read SPEC.md");

    let agents_count = agents_md_claim_count(&agents_md).expect("AGENTS.md's claim-count marker");
    let spec_count = spec_md_doc_conformance_claim_count(&spec_md)
        .expect("SPEC.md's Doc-conformance checks list");

    assert_eq!(
        agents_count, CLAIM_COUNT,
        "AGENTS.md states {agents_count} further claims, expected {CLAIM_COUNT}"
    );
    assert_eq!(
        spec_count, CLAIM_COUNT,
        "SPEC.md's Doc-conformance checks section lists {spec_count} claims, expected \
         {CLAIM_COUNT}"
    );
}

/// Every real `.rs` file under `src/` and `tests/`, found by walking the directories —
/// `tests/coverage_prod.rs`'s `all_src_rs_files` walk, extended to `tests/` because the
/// confinement this claim binds must hold there too (`NORAW-GREP` sweeps both for the
/// same reason).
fn all_src_and_test_rs_files() -> Vec<PathBuf> {
    fn walk(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(&manifest_dir().join("src"), &mut out);
    walk(&manifest_dir().join("tests"), &mut out);
    out.sort();
    out
}

/// The OSC 52 introducer `write_clipboard` writes — the escape this claim confines to one
/// file. A plain string, never a regex: the sequence starts with an ESC byte no source
/// file spells literally, so the printable remainder `]52;` is what every producer and
/// every searcher agree to look for.
const OSC52_INTRODUCER: &str = "]52;";

/// Paths, among `files` (each paired with its own text so the search is pure and testable
/// without touching a real filesystem), that name [`OSC52_INTRODUCER`]. `Err` when `files`
/// is empty — the vacuity guard for the walk itself, distinct from the vacuity guard for
/// the excluded file, which the `#[test]` below checks separately.
fn paths_naming_osc52(files: &[(PathBuf, String)]) -> Result<Vec<&PathBuf>, String> {
    if files.is_empty() {
        return Err(
            "no .rs files found under src/ or tests/ — the walk itself found nothing".to_string(),
        );
    }
    Ok(files
        .iter()
        .filter(|(_, text)| text.contains(OSC52_INTRODUCER))
        .map(|(path, _)| path)
        .collect())
}

#[test]
fn paths_naming_osc52_finds_every_occurrence_and_rejects_an_empty_walk() {
    let a = PathBuf::from("src/ui/terminal.rs");
    let b = PathBuf::from("src/ui/view.rs");
    let files = vec![
        (
            a.clone(),
            "write!(stdout, \"\\x1b]52;c;{}\\x07\", payload)".to_string(),
        ),
        (b.clone(), "no such escape here".to_string()),
    ];
    assert_eq!(paths_naming_osc52(&files), Ok(vec![&a]));

    let err = paths_naming_osc52(&[]).expect_err("an empty walk is an error, not a vacuous pass");
    assert!(err.contains("no .rs files"), "{err}");
}

/// `mouse-text-selection` :: "The clipboard write's confinement is bound inside `cargo
/// test`" — the twelfth `tests/doc_contract.rs` claim. `src/ui/terminal.rs` is the only
/// file permitted to name the OSC 52 introducer `]52;`, the escape
/// `TerminalOps::write_clipboard` writes; checked the same tree-wide-grep-with-a-
/// positive-control way as the subprocess and terminal-mode seams (`NOSPAWN-GREP`,
/// `NORAW-GREP`). The exclusion must not be vacuous: this leg fails just the same when
/// `src/ui/terminal.rs` is absent from the walk, or present but naming no OSC 52 sequence
/// at all — a confinement check that passes because the confined thing has disappeared is
/// worse than none, because it is believed.
#[test]
fn osc52_confinement_matches_the_gate() {
    let self_file = manifest_dir().join("tests/doc_contract.rs");
    let confined_to = manifest_dir().join("src/ui/terminal.rs");

    // Positive control — the excluded file must itself name the sequence, or the
    // confinement below would hold vacuously.
    let terminal_rs = read_doc(&confined_to).expect("read src/ui/terminal.rs");
    assert!(
        terminal_rs.contains(OSC52_INTRODUCER),
        "src/ui/terminal.rs must itself name the OSC 52 introducer {OSC52_INTRODUCER:?}, \
         or this confinement check passes vacuously"
    );

    // This file is excluded from the walk below for the same reason
    // `documented_seam_names_takes_identifiers_only` fabricates names rather than reusing
    // the real six: this claim's own doc comments and unit-test fixtures necessarily name
    // the introducer they check for, and that is not the violation this leg exists to
    // catch.
    let files: Vec<(PathBuf, String)> = all_src_and_test_rs_files()
        .into_iter()
        .filter(|path| path != &self_file)
        .filter_map(|path| read_doc(&path).ok().map(|text| (path, text)))
        .collect();
    let found = paths_naming_osc52(&files).expect("walk src/ and tests/ for the OSC 52 introducer");

    assert_eq!(
        found,
        vec![&confined_to],
        "the OSC 52 introducer {OSC52_INTRODUCER:?} must be named in exactly \
         src/ui/terminal.rs, found in {found:?}"
    );
}

// ---------------------------------------------------------------------------
// `mouse-text-selection` :: "The documented bypass names what was measured"
// (`specs/mouse-input/spec.md`) — the thirteenth `tests/doc_contract.rs` claim.
// `SPEC.md` once claimed the mouse-capture drag-to-select bypass "requires
// holding Option (macOS) or Shift (most Linux terminals)".
// `notes/measurements.md` found `Option` did **not** work in Ghostty on macOS
// under any capture mode set and `Shift` did, under every one, with every
// other terminal left unmeasured — so the sentence was corrected rather than
// generalised from one terminal's convention. This leg is the check that
// correction would otherwise have gone without: nothing else in `cargo test`
// reads this paragraph at all.
// ---------------------------------------------------------------------------

/// The marker opening `SPEC.md` -> Keys' drag-to-select paragraph, immediately
/// below the mouse table it explains.
const DRAG_BYPASS_MARKER: &str = "**Enabling mouse capture costs the terminal's own drag-to-select";

/// Extract the paragraph starting at [`DRAG_BYPASS_MARKER`]: everything up to
/// the next blank line, this document's own paragraph boundary. `Err` when the
/// marker itself is absent, so a rename or removal fails loudly rather than
/// comparing an empty string that trivially satisfies every assertion below.
fn drag_bypass_paragraph(spec_md: &str) -> Result<&str, String> {
    let start = spec_md
        .find(DRAG_BYPASS_MARKER)
        .ok_or_else(|| format!("SPEC.md names no {DRAG_BYPASS_MARKER:?} paragraph"))?;
    let rest = &spec_md[start..];
    let end = rest.find("\n\n").unwrap_or(rest.len());
    Ok(&rest[..end])
}

#[test]
fn drag_bypass_paragraph_extracts_up_to_the_blank_line_and_fails_loudly() {
    let doc = format!("intro\n\n{DRAG_BYPASS_MARKER} more.** text.\n\nnext section\n");
    let got = drag_bypass_paragraph(&doc).expect("marker present");
    assert!(got.starts_with(DRAG_BYPASS_MARKER), "{got:?}");
    assert!(!got.contains("next section"), "{got:?}");

    let err =
        drag_bypass_paragraph("no such marker here").expect_err("a missing marker is an error");
    assert!(err.contains("drag-to-select"), "{err}");
}

/// `mouse-text-selection` :: "The documented bypass names what was measured" —
/// the thirteenth `tests/doc_contract.rs` claim. Reads `SPEC.md` -> Keys' mouse
/// table and its drag-to-select paragraph and requires: the paragraph names
/// `Shift`, names the terminal it was measured on (`Ghostty`), and never
/// claims `Option` as a working bypass — the literal phrase (`holding
/// \`Option\``) this repository once wrote and then falsified against
/// `notes/measurements.md` — and the mouse table carries a drag row whose
/// action is `Action::Select`.
///
/// The `Option` check is deliberately narrower than "the word `Option` never
/// appears": the corrected paragraph itself names `Option` once, to say it was
/// measured **not** to work. What must never reappear is the specific
/// bypass-claiming phrase, `holding \`Option\``, which is what the falsified
/// sentence read.
#[test]
fn the_documented_bypass_names_what_was_measured() {
    let spec_md = read_doc(&manifest_dir().join("SPEC.md")).expect("read SPEC.md");

    let paragraph = drag_bypass_paragraph(&spec_md).expect("SPEC.md's drag-to-select paragraph");
    assert!(
        paragraph.contains("Shift"),
        "the drag-to-select paragraph must name Shift: {paragraph:?}"
    );
    assert!(
        paragraph.contains("Ghostty"),
        "the drag-to-select paragraph must name the terminal it was measured on: {paragraph:?}"
    );
    assert!(
        !paragraph.contains("holding `Option`"),
        "the drag-to-select paragraph must never claim Option as a working bypass: {paragraph:?}"
    );

    let documented = documented_mouse_actions(&spec_md).expect("SPEC.md -> Keys' mouse table");
    assert!(
        documented.contains("Select"),
        "SPEC.md -> Keys' mouse table must carry a drag row whose action is Action::Select, \
         found {documented:?}"
    );
}
