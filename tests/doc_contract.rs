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
        .filter(|name| !section_text.contains(&format!("{name}::")))
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
