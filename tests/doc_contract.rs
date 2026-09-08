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

// --- Gate-path programs leg -------------------------------------------------------------
//
// See `specs/doc-conformance/spec.md` -> "Every non-cargo program `make check` invokes is
// documented as a prerequisite", and `design.md` -> Decision 7 for why the extraction rule
// below is stated against the two real recipe shapes the `Makefile` contains (a shell guard
// block, and a quoted assignment value) rather than a naive "first token per line" rule.

/// Whether `word` occurs in `text` delimited by a non-word character (anything but an ASCII
/// alphanumeric or `_`) on both sides, so a program name occurring as part of a longer
/// identifier does not satisfy a search for it. Shared with `msrv_mentions`, which is the same
/// shape parameterised on a different boundary-character set (digits and `.` rather than
/// alphanumerics and `_`), since a version number's own characters are not word characters.
fn bounded_mention(text: &str, needle: &str, is_boundary_char: impl Fn(u8) -> bool) -> bool {
    if needle.is_empty() {
        return false;
    }
    let bytes = text.as_bytes();
    let nbytes = needle.as_bytes();
    let mut search_start = 0;
    while let Some(rel) = text[search_start..].find(needle) {
        let idx = search_start + rel;
        let left_ok = idx == 0 || !is_boundary_char(bytes[idx - 1]);
        let end = idx + nbytes.len();
        let right_ok = end == bytes.len() || !is_boundary_char(bytes[end]);
        if left_ok && right_ok {
            return true;
        }
        search_start = idx + 1;
    }
    false
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
    let programs =
        check_programs(&makefile).expect("extract programs from Makefile's check: path");

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
    let programs =
        check_programs(makefile).expect("extract from synthetic guard-block Makefile");
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
    let programs =
        check_programs(makefile).expect("extract from synthetic new-tool Makefile");
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
