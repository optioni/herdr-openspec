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

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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
            && let Some(name) = rest.trim_end().strip_suffix(';')
        {
            names.insert(name.trim().to_string());
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

/// Whether `version` appears in `text` delimited by a non-version character (anything but an
/// ASCII digit or `.`) on both sides, so a match inside an unrelated number (`11.887`,
/// `1.881`) does not satisfy it. A version occurring at the very start or end of `text`
/// counts as delimited on that side.
fn msrv_mentions(text: &str, version: &str) -> bool {
    if version.is_empty() {
        return false;
    }
    let is_version_char = |c: u8| c.is_ascii_digit() || c == b'.';
    let bytes = text.as_bytes();
    let vbytes = version.as_bytes();
    let mut search_start = 0;
    while let Some(rel) = text[search_start..].find(version) {
        let idx = search_start + rel;
        let left_ok = idx == 0 || !is_version_char(bytes[idx - 1]);
        let end = idx + vbytes.len();
        let right_ok = end == bytes.len() || !is_version_char(bytes[end]);
        if left_ok && right_ok {
            return true;
        }
        search_start = idx + 1;
    }
    false
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
