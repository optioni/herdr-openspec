//! Markdown task-file parsing: which lines are tasks, whether each is
//! checked, and how they group under headings.
//!
//! See `openspec/changes/task-parsing/design.md` for the full contract.

/// A checkbox count: how many task lines a document holds and how many are
/// checked. The shape both sources of a change's task progress must produce
/// — `changes-from-files` obtains it by counting checkboxes, `changes-from-cli`
/// by reading `completedTasks`/`totalTasks` out of `openspec list --json` — so
/// the two can be compared with `==` and summed with no conversion and no
/// third state. See `openspec/changes/task-parsing/design.md` -> Contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub completed: usize,
    pub total: usize,
}

impl Progress {
    /// True only when there is at least one task and every task is checked.
    /// A change with no tasks is deliberately not "complete" — matching the
    /// CLI's own three-way "No tasks" / "n/m tasks" / "✓ Complete" split.
    pub fn is_complete(&self) -> bool {
        self.total > 0 && self.completed == self.total
    }
}

impl std::ops::Add for Progress {
    type Output = Progress;

    fn add(self, rhs: Progress) -> Progress {
        Progress {
            completed: self.completed + rhs.completed,
            total: self.total + rhs.total,
        }
    }
}

impl std::ops::AddAssign for Progress {
    fn add_assign(&mut self, rhs: Progress) {
        self.completed += rhs.completed;
        self.total += rhs.total;
    }
}

/// The flat checkbox count for `text`. Deliberately identical to the
/// OpenSpec CLI's own `countTasksFromContent` (`dist/utils/task-progress.js`,
/// `@fission-ai/openspec` 1.11.0): a `-` or `*` bullet at any indent carrying
/// a one-character `[ ]` / `[x]` / `[X]` box, with no code-fence, comment, or
/// block-quote exemption. See design.md -> Decisions 1 for why the rule is
/// copied rather than invented.
pub fn count(text: &str) -> Progress {
    let mut progress = Progress {
        completed: 0,
        total: 0,
    };
    for raw_line in text.split('\n') {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if let Some(parts) = task_line(line) {
            progress.total += 1;
            if parts.checked {
                progress.completed += 1;
            }
        }
    }
    progress
}

/// An ATX heading: `level` is the number of `#` characters (1 to 6) and
/// `text` is the remainder of the line, trimmed, kept verbatim — no closing
/// `#` sequence is stripped and no numbering prefix is interpreted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: u8,
    pub text: String,
}

/// One task line: its checked state, its trimmed text, and its indent —
/// the number of whitespace characters preceding its bullet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub checked: bool,
    pub text: String,
    pub indent: usize,
}

/// Which third of a testing lifecycle a task's leading label names:
/// evidence first, then the change, then the confirmation. Four roles
/// rather than one per keyword — `VERIFY`, `THEN`, and `ASSERT` are one
/// position under three conventions, and colouring them separately would be
/// a rainbow nobody learns. `Other` is the role a recognised but
/// unclassified run falls back to, and is not a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelRole {
    Evidence,
    Change,
    Confirm,
    Other,
}

/// Where an item's leading label starts, how many bytes it covers, and the
/// role it classifies to. `start` skips any task number; `len` covers the
/// label token itself and never the number, because a reader wants `1.1` to
/// stay legible and styling it with the label would make the whole row's
/// leading third one colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Label {
    pub start: usize,
    pub len: usize,
    pub role: LabelRole,
}

/// `text`'s leading label, or `None` when it carries none. Pure and
/// **total**: every `&str` returns a value, and both `start` and
/// `start + len` are always character boundaries of `text`, so a caller may
/// `split_at` at either without checking. See
/// `openspec/changes/tasks-emphasis/specs/task-labels/spec.md`.
///
/// Applied to an [`Item`]'s already-trimmed text, in five steps:
///
/// 1. skip an optional task number — a non-empty run of ASCII digits and
///    `.`, optionally one ASCII lowercase letter, then exactly one space;
/// 2. take the leading run of **two or more** ASCII uppercase letters, which
///    is what keeps `Commit: …` and `Run \`cargo test\` …` unlabelled;
/// 3. require that run to be a whole word — the byte after it absent, or
///    neither an ASCII letter nor an ASCII digit — so `REDdish` is not a
///    label and `RED-by-addition:` is;
/// 4. require a `:` somewhere at or after the run's end, which separates a
///    labelled task from prose opening with an acronym;
/// 5. cover the run plus one **immediately following** `:` where there is
///    one.
///
/// Step 4 is a colon test rather than a keyword allow-list because the
/// rule's errors are then misses and never wrong colours: measured over the
/// archive, it labels 2269 of 2563 items and gives a label to **zero** items
/// that are not one. An allow-list would need extending for every workflow
/// and would still have missed `DEFERRED`.
pub fn label_of(text: &str) -> Option<Label> {
    let bytes = text.as_bytes();
    let start = skip_task_number(bytes);

    let mut end = start;
    while end < bytes.len() && bytes[end].is_ascii_uppercase() {
        end += 1;
    }
    if end - start < 2 {
        return None;
    }

    // Whole word: the run ends the text, or the byte after it is neither a
    // letter nor a digit. Every byte examined here is ASCII, and a
    // multi-byte lead byte is neither, so the test is correct on any UTF-8.
    if let Some(&next) = bytes.get(end)
        && next.is_ascii_alphanumeric()
    {
        return None;
    }

    if !text[end..].contains(':') {
        return None;
    }

    let len = if bytes.get(end) == Some(&b':') {
        end - start + 1
    } else {
        end - start
    };

    Some(Label {
        start,
        len,
        role: role_of(&text[start..end]),
    })
}

/// How many bytes of `bytes` a leading task number occupies, including its
/// single trailing space — `0` when there is none. `1.1 `, `10.2a ` and `7 `
/// are skipped; `1.1` with no following space is not, and neither is `a1 `.
///
/// The bounded lookahead here and the uppercase run [`label_of`] scans next
/// cover disjoint byte ranges, so the recognition is one left-to-right pass
/// over the text's prefix rather than two (tasks.md 1.5).
fn skip_task_number(bytes: &[u8]) -> usize {
    let mut i = 0;
    while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
        i += 1;
    }
    if i == 0 {
        return 0;
    }
    if bytes.get(i).is_some_and(|b| b.is_ascii_lowercase()) {
        i += 1;
    }
    if bytes.get(i) == Some(&b' ') {
        i + 1
    } else {
        0
    }
}

/// The lifecycle position `run` names, by **exact, case-sensitive** match,
/// with `Other` as the fallback arm rather than a lookup miss. A general
/// testing vocabulary, not one workflow's task prefixes: GIVEN/WHEN/THEN and
/// ARRANGE/ACT/ASSERT are the same three positions under two further
/// conventions and are styled identically. Nothing outside `run` is read.
fn role_of(run: &str) -> LabelRole {
    match run {
        "RED" | "CHARACTERIZE" | "CHECK" | "GIVEN" | "ARRANGE" => LabelRole::Evidence,
        "GREEN" | "REFACTOR" | "CHANGE" | "WHEN" | "ACT" => LabelRole::Change,
        "VERIFY" | "THEN" | "ASSERT" => LabelRole::Confirm,
        _ => LabelRole::Other,
    }
}

/// The task lines under one heading (or, for the leading group, under no
/// heading at all), in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub heading: Option<Heading>,
    pub items: Vec<Item>,
}

impl Group {
    /// This group's checkbox count.
    pub fn progress(&self) -> Progress {
        let mut progress = Progress {
            completed: 0,
            total: 0,
        };
        for item in &self.items {
            progress.total += 1;
            if item.checked {
                progress.completed += 1;
            }
        }
        progress
    }
}

/// A parsed task document: groups in document order, plus any problems
/// encountered reading it (always empty for [`parse`]; [`read`] populates
/// it for a filesystem edge).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tasks {
    pub groups: Vec<Group>,
    pub problems: Vec<String>,
}

impl Tasks {
    /// The whole document's checkbox count: the sum of every group's.
    /// Equal to `count` applied to the same text — see the group-4 corpus
    /// test that pins this agreement.
    pub fn progress(&self) -> Progress {
        let mut progress = Progress {
            completed: 0,
            total: 0,
        };
        for group in &self.groups {
            progress += group.progress();
        }
        progress
    }
}

/// Arrange `text`'s task lines into groups under the ATX headings above
/// them. One pass over the same split-and-strip-`\r` lines as [`count`]: a
/// line starting at column zero with one to six `#` followed by a space or
/// the end of the line closes the current group and opens a new one; a
/// line matching the task rule appends an item; every other line — prose —
/// is discarded. The leading (headingless) group is emitted only when it
/// holds at least one item; every group with a heading is emitted
/// regardless, including one with no items and one whose heading text is
/// empty. Never sorts, merges, deduplicates, or nests.
pub fn parse(text: &str) -> Tasks {
    let mut groups = Vec::new();
    let mut heading: Option<Heading> = None;
    let mut items: Vec<Item> = Vec::new();

    for raw_line in text.split('\n') {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if let Some(new_heading) = heading_line(line) {
            close_group(&mut groups, heading.take(), std::mem::take(&mut items));
            heading = Some(new_heading);
        } else if let Some(parts) = task_line(line) {
            items.push(Item {
                checked: parts.checked,
                text: parts.text.trim().to_string(),
                indent: parts.indent,
            });
        }
    }
    close_group(&mut groups, heading, items);

    Tasks {
        groups,
        problems: Vec::new(),
    }
}

/// Read a task file at `path`, without writing anything. Never returns a
/// `Result`, never panics, never `unwrap`s: a missing file — the CLI's own
/// treatment of a change whose tasks artifact is not written yet — is an
/// empty [`Tasks`] with **no** problem; every other read failure, including
/// a directory where a file was expected, a permission error, or an
/// invalid-UTF-8 decode failure, is an empty `Tasks` carrying exactly one
/// problem naming `path`.
///
/// Invalid UTF-8 is deliberately **not** decoded lossily to preserve the
/// CLI's count: the CLI reads with replacement characters and so reports a
/// count for such a file, and this module reports none plus a named
/// problem instead — a knowing divergence recorded in
/// `openspec/changes/task-parsing/design.md` -> Decisions 8 and in
/// `specs/task-groups`. The dual-source model is what resolves it: the
/// CLI's count arrives and corrects the pane.
pub fn read(path: &std::path::Path) -> Tasks {
    #[cfg(test)]
    READ_PATHS.with(|paths| paths.borrow_mut().push(path.to_path_buf()));

    match std::fs::read_to_string(path) {
        Ok(text) => parse(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Tasks {
            groups: Vec::new(),
            problems: Vec::new(),
        },
        Err(e) => Tasks {
            groups: Vec::new(),
            problems: vec![format!("{} could not be read: {e}", path.display())],
        },
    }
}

/// Push the group being closed, unless it is the leading (headingless)
/// group and holds no items — the one case `parse`'s doc comment names as
/// suppressed.
fn close_group(groups: &mut Vec<Group>, heading: Option<Heading>, items: Vec<Item>) {
    if heading.is_some() || !items.is_empty() {
        groups.push(Group { heading, items });
    }
}

/// If `line` is an ATX heading — one to six `#` characters at column zero,
/// followed by a space or the end of the line — the heading it opens.
/// `None` for seven or more `#` characters, a `#` immediately followed by a
/// non-space character, or any leading whitespace before the first `#`
/// (deliberately stricter than CommonMark's three-space allowance — see
/// design.md -> Decisions 5).
fn heading_line(line: &str) -> Option<Heading> {
    let hashes = line.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &line[hashes..];
    match rest.chars().next() {
        None => Some(Heading {
            level: hashes as u8,
            text: String::new(),
        }),
        Some(' ') => Some(Heading {
            level: hashes as u8,
            text: rest.trim().to_string(),
        }),
        Some(_) => None,
    }
}

/// The parts of one task line, borrowed rather than owned: `count` uses
/// this without allocating anything, and `parse` allocates only when it
/// builds an [`Item`] from the result. Two entry points, one rule — see
/// design.md -> Decisions 3.
struct TaskLineParts<'a> {
    checked: bool,
    /// Everything after the closing `]`, not yet trimmed. `count` ignores
    /// this; `parse` trims it with `str::trim` (`char::is_whitespace`-based,
    /// not [`is_task_whitespace`] — trimming happens after the line has
    /// already been counted, so which whitespace rule it uses cannot change
    /// a count).
    text: &'a str,
    /// The number of [`is_task_whitespace`] characters preceding the
    /// bullet, counted as characters rather than columns.
    indent: usize,
}

/// If `line` is a task line, its parts. `None` when the line is not a task
/// line at all. A single left-to-right scan: skip leading whitespace,
/// require exactly one `-` or `*`, skip whitespace, require `[`, take
/// exactly one character, require `]`. No fence, comment, or block-quote
/// state — see design.md -> Decisions 6.
fn task_line(line: &str) -> Option<TaskLineParts<'_>> {
    let mut it = line.char_indices().peekable();
    let indent = skip_task_whitespace(&mut it);

    match it.next() {
        Some((_, '-')) | Some((_, '*')) => {}
        _ => return None,
    }

    skip_task_whitespace(&mut it);

    match it.next() {
        Some((_, '[')) => {}
        _ => return None,
    }

    let box_char = it.next().map(|(_, c)| c)?;
    if !(is_task_whitespace(box_char) || box_char == 'x' || box_char == 'X') {
        return None;
    }

    match it.next() {
        Some((_, ']')) => {}
        _ => return None,
    }

    let text = match it.peek() {
        Some(&(byte_idx, _)) => &line[byte_idx..],
        None => "",
    };

    Some(TaskLineParts {
        checked: box_char == 'x' || box_char == 'X',
        text,
        indent,
    })
}

/// Advance `it` past any run of [`is_task_whitespace`] characters at its
/// front, returning how many were skipped.
fn skip_task_whitespace(it: &mut std::iter::Peekable<std::str::CharIndices<'_>>) -> usize {
    let mut skipped = 0;
    while let Some(&(_, c)) = it.peek() {
        if is_task_whitespace(c) {
            skipped += 1;
            it.next();
        } else {
            break;
        }
    }
    skipped
}

/// The whitespace alphabet the OpenSpec CLI's `\s` matches: Unicode
/// `White_Space` plus U+FEFF (byte-order mark), minus U+0085 (next line).
/// Both differences from `char::is_whitespace` are resolved in the CLI's
/// favour, because the counts must agree: a UTF-8 BOM sits immediately
/// before a file's first character, so without U+FEFF here the first task
/// line of a BOM-prefixed file would be invisible to this module and
/// visible to the CLI; U+0085 is whitespace to `char::is_whitespace` and
/// not to the CLI, so it is admitted by neither rule here. Used at every
/// whitespace test in the scan: leading indent, between bullet and box,
/// inside the box, and after the box.
fn is_task_whitespace(c: char) -> bool {
    (c.is_whitespace() || c == '\u{feff}') && c != '\u{85}'
}

// `list-sections`' evidence that a collapsed archived section opens no
// file beneath it (design.md -> Decision 16): every path `read` reads is
// recorded here, on `schema::read_file`'s own recorder's terms —
// `thread_local!`, never a `static`, because the suite runs this crate's
// tests in parallel threads of one process, and declared at the bottom of
// the file, directly above `mod tests`, so a future production-slice
// sweep still finds every production line above it.
#[cfg(test)]
thread_local! {
    static READ_PATHS: std::cell::RefCell<Vec<std::path::PathBuf>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Every path [`read`] has read since the last [`take_recorded_reads`] call
/// (or since the thread started), and clears the record.
#[cfg(test)]
pub(crate) fn take_recorded_reads() -> Vec<std::path::PathBuf> {
    READ_PATHS.with(|paths| std::mem::take(&mut *paths.borrow_mut()))
}

#[cfg(test)]
mod tests {
    // Group 1: `Progress` and the flat line rule. Every test here drives
    // `count` only — `parse` does not exist until group 3, which picks up
    // the `parse` half of the three scenarios that carry both clauses (see
    // tasks.md group 1 preamble and 3.2).

    #[test]
    fn the_four_canonical_shapes_are_task_lines() {
        let text = "- [ ] a\n- [x] b\n* [ ] c\n* [x] d";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 2,
                total: 4
            }
        );
    }

    #[test]
    fn whitespace_around_the_bullet_and_the_box_is_optional() {
        let text = "-[x]done\n   *   [ ]   todo   ";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 1,
                total: 2
            }
        );
    }

    #[test]
    fn a_plus_bullet_and_an_ordered_marker_are_not_task_lines() {
        // A genuine task line alongside the three rejected shapes, so an
        // implementation matching nothing at all cannot pass by returning
        // zero (tasks.md 1.4).
        let text = "+ [x] plus\n1. [x] ordered\n1) [x] paren\n- [x] real";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 1,
                total: 1
            }
        );
    }

    #[test]
    fn a_malformed_box_is_not_a_task_line() {
        // Each line paired with a genuine task line so an implementation
        // matching nothing at all cannot pass by returning zero.
        let text = "- [] empty\n- [x] real1\n\
                     - [-] dash\n- [x] real2\n\
                     - [xx] two\n- [x] real3\n\
                     - [x extra\n- [x] real4\n\
                     - ( ) round\n- [x] real5";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 5,
                total: 5
            }
        );
    }

    #[test]
    fn a_bullet_with_no_checkbox_and_prose_that_mentions_one_are_not_task_lines() {
        let text = "- an ordinary list item\n- [x] real1\n\
                     text - [x] mid-line\n- [x] real2\n\
                     -- [x] two dashes\n- [x] real3";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 3,
                total: 3
            }
        );
    }

    #[test]
    fn both_letter_cases_count_as_done() {
        let text = "- [x] lower\n- [X] upper";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 2,
                total: 2
            }
        );
    }

    #[test]
    fn a_space_a_tab_and_a_non_breaking_space_are_all_empty_boxes() {
        let text = "- [\u{20}] space\n- [\u{9}] tab\n- [\u{a0}] nbsp";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 0,
                total: 3
            }
        );
    }

    #[test]
    fn a_checkbox_inside_a_fenced_code_block_counts() {
        let text = "```text\n- [x] example\n```\n- [ ] real";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 1,
                total: 2
            }
        );
    }

    #[test]
    fn a_checkbox_inside_an_html_comment_counts() {
        let text = "<!--\n- [x] commented out\n-->\n- [ ] real";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 1,
                total: 2
            }
        );
    }

    #[test]
    fn a_block_quoted_checkbox_does_not_count() {
        // `>` is not in the bullet alphabet: no block-quote rule exists to
        // go wrong, this is a guard pinning a non-feature, not a driver of
        // new behaviour.
        let text = "> - [x] quoted";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 0,
                total: 0
            }
        );
    }

    #[test]
    fn the_last_line_counts_without_a_trailing_newline() {
        let text = "- [x] only";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 1,
                total: 1
            }
        );
    }

    #[test]
    fn a_document_whose_only_checkbox_is_on_the_last_line_after_a_trailing_newline() {
        let text = "prose\n\n- [x] last\n";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 1,
                total: 1
            }
        );
    }

    #[test]
    fn a_crlf_document_counts_the_same_as_its_lf_twin() {
        // The `count` half of "A CRLF document counts and reads the same as
        // its LF twin" — group 3's a_crlf_document_parses_the_same_as_its_lf_twin
        // test covers the `parse` half. Found untested by the group-7 review.
        let crlf = "## G\r\n- [x] a\r\n- [ ] b\r\n";
        let lf = "## G\n- [x] a\n- [ ] b\n";
        let expected = super::Progress {
            completed: 1,
            total: 2,
        };
        assert_eq!(super::count(crlf), expected);
        assert_eq!(super::count(lf), expected);
    }

    #[test]
    fn an_empty_string_has_no_tasks() {
        let progress = super::count("");
        assert_eq!(
            progress,
            super::Progress {
                completed: 0,
                total: 0
            }
        );
    }

    #[test]
    fn a_document_of_blank_lines_and_prose_has_no_tasks() {
        let text = "\n\n   \nSome prose that mentions nothing checkbox-shaped.\n\n";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 0,
                total: 0
            }
        );
    }

    #[test]
    fn is_complete_is_true_at_three_of_three() {
        let progress = super::count("- [x] a\n- [x] b\n- [x] c");
        assert!(progress.is_complete());
    }

    #[test]
    fn is_complete_is_false_at_zero_of_zero() {
        let progress = super::count("no tasks here");
        assert!(!progress.is_complete());
    }

    #[test]
    fn progress_values_from_several_files_sum() {
        let a = super::Progress {
            completed: 1,
            total: 3,
        };
        let b = super::Progress {
            completed: 2,
            total: 2,
        };
        assert_eq!(
            a + b,
            super::Progress {
                completed: 3,
                total: 5
            }
        );

        let mut acc = super::Progress {
            completed: 0,
            total: 0,
        };
        acc += a;
        acc += b;
        assert_eq!(
            acc,
            super::Progress {
                completed: 3,
                total: 5
            }
        );
    }

    #[test]
    fn a_file_derived_count_and_a_cli_shaped_count_compare_equal() {
        let text = "- [x] a\n- [ ] b\n- [ ] c\n- [ ] d";
        let from_file = super::count(text);
        let from_cli = super::Progress {
            completed: 1,
            total: 4,
        };
        assert_eq!(from_file, from_cli);
    }

    // Group 2: the whitespace alphabet. `char::is_whitespace` differs from
    // the CLI's `\s` at exactly two code points — these three tests are red
    // against group 1's shipped `count` because it uses that predicate.

    #[test]
    fn a_byte_order_mark_before_the_first_bullet_does_not_hide_the_task() {
        let text = "\u{feff}- [x] first\n- [ ] second";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 1,
                total: 2
            }
        );
    }

    #[test]
    fn a_next_line_character_before_a_bullet_is_not_indentation() {
        let text = "\u{85}- [x] task";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 0,
                total: 0
            }
        );
    }

    #[test]
    fn a_next_line_character_inside_the_box_is_not_an_empty_box() {
        let text = "- [\u{85}] x";
        let progress = super::count(text);
        assert_eq!(
            progress,
            super::Progress {
                completed: 0,
                total: 0
            }
        );
    }

    // Group 3: headings, groups, and items. `assert_eq!` compares whole
    // `Tasks` values wherever the expected value is small enough to write
    // out, per tasks.md 3.1, so an extra fabricated group cannot slip
    // through unnoticed.

    fn item(checked: bool, text: &str, indent: usize) -> super::Item {
        super::Item {
            checked,
            text: text.to_string(),
            indent,
        }
    }

    fn heading(level: u8, text: &str) -> super::Heading {
        super::Heading {
            level,
            text: text.to_string(),
        }
    }

    #[test]
    fn two_headings_yield_two_groups_holding_their_own_items() {
        let text = "## 1. First\n- [x] a\n- [ ] b\n## 2. Second\n- [ ] c";
        let tasks = super::parse(text);
        assert_eq!(
            tasks,
            super::Tasks {
                groups: vec![
                    super::Group {
                        heading: Some(heading(2, "1. First")),
                        items: vec![item(true, "a", 0), item(false, "b", 0)],
                    },
                    super::Group {
                        heading: Some(heading(2, "2. Second")),
                        items: vec![item(false, "c", 0)],
                    },
                ],
                problems: vec![],
            }
        );
        assert_eq!(
            tasks.groups[0].progress(),
            super::Progress {
                completed: 1,
                total: 2
            }
        );
        assert_eq!(
            tasks.groups[1].progress(),
            super::Progress {
                completed: 0,
                total: 1
            }
        );
    }

    #[test]
    fn items_before_the_first_heading_form_an_unnamed_leading_group() {
        let text = "- [x] loose\n## Group\n- [ ] inside";
        let tasks = super::parse(text);
        assert_eq!(
            tasks,
            super::Tasks {
                groups: vec![
                    super::Group {
                        heading: None,
                        items: vec![item(true, "loose", 0)],
                    },
                    super::Group {
                        heading: Some(heading(2, "Group")),
                        items: vec![item(false, "inside", 0)],
                    },
                ],
                problems: vec![],
            }
        );
    }

    #[test]
    fn a_file_opening_with_a_heading_has_no_empty_leading_group() {
        let text = "# Implementation Tasks\n\n## 1. Group\n- [ ] a";
        let tasks = super::parse(text);
        assert_eq!(tasks.groups.len(), 2);
        assert!(tasks.groups.iter().all(|g| g.heading.is_some()));
        assert_eq!(
            tasks,
            super::Tasks {
                groups: vec![
                    super::Group {
                        heading: Some(heading(1, "Implementation Tasks")),
                        items: vec![],
                    },
                    super::Group {
                        heading: Some(heading(2, "1. Group")),
                        items: vec![item(false, "a", 0)],
                    },
                ],
                problems: vec![],
            }
        );
    }

    #[test]
    fn a_closing_hash_sequence_is_kept_not_stripped() {
        let text = "## Group ##\n- [ ] a";
        let tasks = super::parse(text);
        assert_eq!(tasks.groups.len(), 1);
        assert_eq!(tasks.groups[0].heading, Some(heading(2, "Group ##")));
    }

    #[test]
    fn indented_over_long_and_unspaced_hashes_are_not_headings() {
        let text = "   ## Indented\n- [ ] a\n####### Seven\n- [ ] a\n#NoSpace\n- [ ] a";
        let tasks = super::parse(text);
        assert_eq!(tasks.groups.len(), 1);
        assert_eq!(tasks.groups[0].heading, None);
        assert_eq!(tasks.groups[0].items.len(), 3);
        assert_eq!(
            tasks.progress(),
            super::Progress {
                completed: 0,
                total: 3
            }
        );
    }

    #[test]
    fn a_deeper_heading_closes_the_group_above_rather_than_nesting() {
        let text = "## Outer\n- [ ] a\n### Inner\n- [ ] b";
        let tasks = super::parse(text);
        assert_eq!(
            tasks,
            super::Tasks {
                groups: vec![
                    super::Group {
                        heading: Some(heading(2, "Outer")),
                        items: vec![item(false, "a", 0)],
                    },
                    super::Group {
                        heading: Some(heading(3, "Inner")),
                        items: vec![item(false, "b", 0)],
                    },
                ],
                problems: vec![],
            }
        );
    }

    #[test]
    fn repeated_and_empty_headings_are_all_kept() {
        let text = "## Same\n- [ ] a\n## Same\n- [ ] b\n##\n## Last";
        let tasks = super::parse(text);
        assert_eq!(tasks.groups.len(), 4);
        let heading_texts: Vec<&str> = tasks
            .groups
            .iter()
            .map(|g| g.heading.as_ref().unwrap().text.as_str())
            .collect();
        assert_eq!(heading_texts, vec!["Same", "Same", "", "Last"]);
        assert!(tasks.groups[2].items.is_empty());
        assert!(tasks.groups[3].items.is_empty());
    }

    #[test]
    fn prose_between_items_is_dropped_and_does_not_split_a_group() {
        let text = "## G\n- [ ] a\n\nSome explanatory prose.\n- [ ] b";
        let tasks = super::parse(text);
        assert_eq!(
            tasks,
            super::Tasks {
                groups: vec![super::Group {
                    heading: Some(heading(2, "G")),
                    items: vec![item(false, "a", 0), item(false, "b", 0)],
                }],
                problems: vec![],
            }
        );
    }

    #[test]
    fn a_nested_sub_task_is_a_sibling_item_carrying_its_indent() {
        let text = "## G\n- [x] parent\n  - [ ] child\n\t- [ ] tabbed";
        let tasks = super::parse(text);
        assert_eq!(
            tasks,
            super::Tasks {
                groups: vec![super::Group {
                    heading: Some(heading(2, "G")),
                    items: vec![
                        item(true, "parent", 0),
                        item(false, "child", 2),
                        item(false, "tabbed", 1),
                    ],
                }],
                problems: vec![],
            }
        );
        assert_eq!(
            tasks.groups[0].progress(),
            super::Progress {
                completed: 1,
                total: 3
            }
        );
    }

    #[test]
    fn indent_does_not_affect_membership_of_the_preceding_group() {
        let text = "## G\n        - [ ] deeply indented\n## H";
        let tasks = super::parse(text);
        assert_eq!(
            tasks,
            super::Tasks {
                groups: vec![
                    super::Group {
                        heading: Some(heading(2, "G")),
                        items: vec![item(false, "deeply indented", 8)],
                    },
                    super::Group {
                        heading: Some(heading(2, "H")),
                        items: vec![],
                    },
                ],
                problems: vec![],
            }
        );
    }

    #[test]
    fn a_checked_items_state_survives_grouping() {
        let text = "## Group\n- [X] done\n- [ ] todo";
        let tasks = super::parse(text);
        assert_eq!(
            tasks,
            super::Tasks {
                groups: vec![super::Group {
                    heading: Some(heading(2, "Group")),
                    items: vec![item(true, "done", 0), item(false, "todo", 0)],
                }],
                problems: vec![],
            }
        );
        assert_eq!(
            tasks.groups[0].progress(),
            super::Progress {
                completed: 1,
                total: 2
            }
        );
    }

    #[test]
    fn a_crlf_document_parses_the_same_as_its_lf_twin() {
        let crlf = "## G\r\n- [x] a\r\n- [ ] b\r\n";
        let lf = "## G\n- [x] a\n- [ ] b\n";
        let from_crlf = super::parse(crlf);
        let from_lf = super::parse(lf);
        assert_eq!(from_crlf, from_lf);
        assert_eq!(
            from_lf,
            super::Tasks {
                groups: vec![super::Group {
                    heading: Some(heading(2, "G")),
                    items: vec![item(true, "a", 0), item(false, "b", 0)],
                }],
                problems: vec![],
            }
        );
        assert_eq!(
            from_crlf.progress(),
            super::Progress {
                completed: 1,
                total: 2
            }
        );
        assert!(
            !from_crlf.groups[0]
                .heading
                .as_ref()
                .unwrap()
                .text
                .contains('\r')
        );
        for group in &from_crlf.groups {
            for item in &group.items {
                assert!(!item.text.contains('\r'));
            }
        }
    }

    #[test]
    fn an_empty_document_parses_to_no_tasks_and_no_problems() {
        let tasks = super::parse("");
        assert_eq!(
            tasks,
            super::Tasks {
                groups: vec![],
                problems: vec![],
            }
        );

        let prose_only =
            super::parse("\n\n   \nSome prose that mentions nothing checkbox-shaped.\n\n");
        assert_eq!(
            prose_only,
            super::Tasks {
                groups: vec![],
                problems: vec![],
            }
        );
    }

    #[test]
    fn the_four_canonical_shapes_are_task_lines_parse_half() {
        let text = "- [ ] a\n- [x] b\n* [ ] c\n* [x] d";
        let tasks = super::parse(text);
        assert_eq!(
            tasks,
            super::Tasks {
                groups: vec![super::Group {
                    heading: None,
                    items: vec![
                        item(false, "a", 0),
                        item(true, "b", 0),
                        item(false, "c", 0),
                        item(true, "d", 0),
                    ],
                }],
                problems: vec![],
            }
        );
    }

    #[test]
    fn whitespace_around_the_bullet_and_the_box_is_optional_parse_half() {
        let text = "-[x]done\n   *   [ ]   todo   ";
        let tasks = super::parse(text);
        assert_eq!(
            tasks,
            super::Tasks {
                groups: vec![super::Group {
                    heading: None,
                    items: vec![item(true, "done", 0), item(false, "todo", 3)],
                }],
                problems: vec![],
            }
        );
    }

    #[test]
    fn a_space_a_tab_and_a_non_breaking_space_are_all_empty_boxes_parse_half() {
        let text = "- [\u{20}] space\n- [\u{9}] tab\n- [\u{a0}] nbsp";
        let tasks = super::parse(text);
        assert_eq!(tasks.groups.len(), 1);
        assert_eq!(tasks.groups[0].items.len(), 3);
        for parsed_item in &tasks.groups[0].items {
            assert!(!parsed_item.checked);
        }
    }

    #[test]
    fn a_numbering_prefix_and_inline_markup_are_kept_verbatim_in_the_text() {
        let text = "- [x] 3.11 Run the group tests — `cargo test` green\n\
                     - [ ] 10.5a **VERIFY:** coverage";
        let tasks = super::parse(text);
        assert_eq!(
            tasks,
            super::Tasks {
                groups: vec![super::Group {
                    heading: None,
                    items: vec![
                        item(true, "3.11 Run the group tests — `cargo test` green", 0),
                        item(false, "10.5a **VERIFY:** coverage", 0),
                    ],
                }],
                problems: vec![],
            }
        );
    }

    // Group 4: count and parse agree, on a real corpus. Every fixture is
    // `include_str!`-embedded at compile time, never a run-time path, so no
    // test resolves a location in the live repository — see design.md ->
    // Decisions 11.

    const ARCHIVED_CHANGE: &str = include_str!("../tests/fixtures/tasks/archived-change.md");
    const CRLF: &str = include_str!("../tests/fixtures/tasks/crlf.md");
    const FENCED_AND_COMMENTED: &str =
        include_str!("../tests/fixtures/tasks/fenced-and-commented.md");
    const NO_HEADING: &str = include_str!("../tests/fixtures/tasks/no-heading.md");
    const EMPTY: &str = include_str!("../tests/fixtures/tasks/empty.md");

    #[test]
    fn a_real_changes_task_file_counts_the_same_both_ways() {
        for text in [
            ARCHIVED_CHANGE,
            CRLF,
            FENCED_AND_COMMENTED,
            NO_HEADING,
            EMPTY,
        ] {
            assert_eq!(super::parse(text).progress(), super::count(text));
        }

        // Oracle pair for `archived-change.md` (a frozen copy of
        // openspec/changes/archive/2026-09-04-schema-model/tasks.md), obtained
        // by running the CLI's own TASK_LINE_PATTERN over the same bytes in
        // node (@fission-ai/openspec 1.11.0), independently of this crate:
        //
        //   node -e '
        //     const fs = require("fs");
        //     const TASK_LINE_PATTERN = /^\s*[-*]\s*\[([\sxX])\]\s*(.*)/;
        //     const content = fs.readFileSync("tests/fixtures/tasks/archived-change.md", "utf-8");
        //     let total = 0, completed = 0;
        //     for (const line of content.split("\n")) {
        //       const m = line.match(TASK_LINE_PATTERN);
        //       if (m) { total++; if (m[1].toLowerCase() === "x") completed++; }
        //     }
        //     console.log({ total, completed });
        //   '
        //   => { total: 92, completed: 92 }
        //
        // Without this absolute-count assertion, an implementation that
        // returns zero from both `count` and `parse` would still pass the
        // equality loop above.
        assert_eq!(
            super::count(ARCHIVED_CHANGE),
            super::Progress {
                completed: 92,
                total: 92
            }
        );
    }

    #[test]
    fn text_with_no_heading_still_agrees() {
        let expected = super::Progress {
            completed: 2,
            total: 5,
        };
        assert_eq!(super::count(NO_HEADING), expected);

        let tasks = super::parse(NO_HEADING);
        assert_eq!(tasks.progress(), expected);
        assert_eq!(tasks.groups.len(), 1);
        assert_eq!(tasks.groups[0].heading, None);
    }

    // Group 5: the filesystem edge, and what it must not touch. Every
    // fixture is built under a `ScratchDir`, never a path in the real
    // repository.

    use crate::testutil::{ScratchDir, snapshot, write_with_mode};

    #[test]
    fn an_absent_file_is_zero_tasks_and_no_problem() {
        let scratch = ScratchDir::new();
        let path = scratch.path().join("tasks.md");
        assert!(!path.exists());

        let tasks = super::read(&path);
        assert_eq!(
            tasks,
            super::Tasks {
                groups: vec![],
                problems: vec![],
            }
        );
    }

    #[test]
    fn a_directory_where_a_file_was_expected_is_one_named_problem() {
        let scratch = ScratchDir::new();

        let tasks = super::read(scratch.path());
        assert!(tasks.groups.is_empty());
        assert_eq!(tasks.problems.len(), 1);
        assert!(tasks.problems[0].contains(&scratch.path().display().to_string()));
    }

    #[test]
    fn a_file_of_invalid_utf8_is_one_named_problem_not_a_parse_result() {
        let scratch = ScratchDir::new();
        let path = scratch.path().join("tasks.md");
        write_with_mode(&path, &[0xFF, 0xFE], 0o644);

        let tasks = super::read(&path);
        assert!(tasks.groups.is_empty());
        assert_eq!(tasks.problems.len(), 1);
        assert!(tasks.problems[0].contains(&path.display().to_string()));
    }

    #[test]
    fn a_readable_file_is_parsed_exactly_as_its_text_would_be() {
        let scratch = ScratchDir::new();
        let path = scratch.path().join("tasks.md");
        let text = "## G\n- [x] a\n- [ ] b\n";
        write_with_mode(&path, text.as_bytes(), 0o644);
        assert_eq!(
            std::fs::read(&path).expect("fixture file readable"),
            text.as_bytes()
        );

        assert_eq!(super::read(&path), super::parse(text));
    }

    #[test]
    fn a_task_tree_is_byte_identical_after_reading() {
        let scratch = ScratchDir::new();
        let root = scratch.path();

        let tasks_md = root
            .join("openspec")
            .join("changes")
            .join("x")
            .join("tasks.md");
        write_with_mode(&tasks_md, b"## G\n- [x] a\n", 0o644);
        let specs_dir = root.join("openspec").join("specs");
        std::fs::create_dir_all(&specs_dir).expect("create empty specs dir");
        let readme = root.join("README.md");
        write_with_mode(&readme, b"# Readme\n", 0o644);

        let nonexistent = root.join("nonexistent").join("nested").join("path.md");

        let before = snapshot(root);
        let _ = super::read(&tasks_md);
        let _ = super::read(&specs_dir);
        let _ = super::read(&nonexistent);
        let after = snapshot(root);

        assert_eq!(before, after);
        assert!(!nonexistent.exists());
        assert!(!nonexistent.parent().unwrap().exists());
        assert!(!root.join("nonexistent").exists());
    }

    // `task-labels`: the leading-label recognition and its three-position
    // vocabulary. Every fixture here is a `&str` literal and the function
    // under test is pure and total — there is no schema, no filesystem, and
    // no `Schema` value anywhere in scope.

    use super::{Label, LabelRole, label_of};

    /// `task-labels` :: "The plain and compound label forms are both recognised".
    #[test]
    fn the_plain_and_compound_label_forms_are_both_recognised() {
        let cases = [
            (
                "VERIFY: cargo test is green",
                0usize,
                7usize,
                LabelRole::Confirm,
            ),
            (
                "CHANGE — rewrite in `SPEC.md`: the module map",
                0,
                6,
                LabelRole::Change,
            ),
            ("RED then GREEN: write both", 0, 3, LabelRole::Evidence),
            ("RED-by-addition: add the row", 0, 3, LabelRole::Evidence),
            (
                "CHECK (contract gate): re-read design.md",
                0,
                5,
                LabelRole::Evidence,
            ),
        ];
        for (text, start, len, role) in cases {
            assert_eq!(label_of(text), Some(Label { start, len, role }), "{text:?}");
        }
        // The colon joins the label only where it immediately follows the run:
        // `VERIFY:` is 7 bytes and `CHANGE` is 6 because a space follows it.
        assert_eq!(&"VERIFY: cargo test is green"[0..7], "VERIFY:");
        assert_eq!(
            &"CHANGE — rewrite in `SPEC.md`: the module map"[0..6],
            "CHANGE"
        );
    }

    /// `task-labels` :: "A task number is skipped and does not become part of
    /// the label".
    #[test]
    fn a_task_number_is_skipped_and_does_not_become_part_of_the_label() {
        let cases = [
            (
                "1.1 RED: write the failing test",
                4usize,
                4usize,
                LabelRole::Evidence,
            ),
            ("10.2a GREEN: implement it", 6, 6, LabelRole::Change),
            ("7 VERIFY: make check", 2, 7, LabelRole::Confirm),
        ];
        for (text, start, len, role) in cases {
            assert_eq!(label_of(text), Some(Label { start, len, role }), "{text:?}");
            // `start` points at the run's first letter and never at a digit,
            // so a caller splitting there leaves the number unstyled.
            let first = text[start..].chars().next().unwrap();
            assert!(
                first.is_ascii_uppercase(),
                "{text:?}: start {start} points at {first:?}, not a letter"
            );
        }
        assert_eq!(&"1.1 RED: write the failing test"[4..8], "RED:");
        assert_eq!(&"10.2a GREEN: implement it"[6..12], "GREEN:");
        assert_eq!(&"7 VERIFY: make check"[2..9], "VERIFY:");
    }

    /// `task-labels` :: "Unlabelled tasks are recognised as unlabelled".
    #[test]
    fn unlabelled_tasks_are_recognised_as_unlabelled() {
        for text in [
            "Commit: the parser and its tests",
            "Run `cargo test --all-features`",
            "Rewrite in `SPEC.md`: the module map",
            "REDdish text",
            "A: short",
            "CI must stay green",
        ] {
            assert_eq!(label_of(text), None, "{text:?}");
        }
    }

    /// `task-labels` :: "The recognition is total over degenerate input".
    #[test]
    fn the_recognition_is_total_over_degenerate_input() {
        let long = format!("{}:", "A".repeat(10000));
        let cases: Vec<(&str, Option<Label>)> = vec![
            ("", None),
            ("   ", None),
            (":", None),
            ("::::", None),
            ("ABC", None),
            (
                "ABC:",
                Some(Label {
                    start: 0,
                    len: 4,
                    role: LabelRole::Other,
                }),
            ),
            ("1.1 ", None),
            ("1.1", None),
            (
                long.as_str(),
                Some(Label {
                    start: 0,
                    len: 10001,
                    role: LabelRole::Other,
                }),
            ),
            ("日本語: text", None),
        ];
        for (text, expected) in cases {
            let got = label_of(text);
            assert_eq!(got, expected, "{:?}", &text[..text.len().min(40)]);
            if let Some(label) = got {
                // Both offsets are character boundaries: `split_at` panics
                // otherwise, which is the assertion.
                let _ = text.split_at(label.start);
                let _ = text.split_at(label.start + label.len);
            }
        }
    }

    /// `task-labels` :: "Every token in the table classifies to its own role".
    #[test]
    fn every_token_in_the_table_classifies_to_its_own_role() {
        let evidence = ["RED", "CHARACTERIZE", "CHECK", "GIVEN", "ARRANGE"];
        let change = ["GREEN", "REFACTOR", "CHANGE", "WHEN", "ACT"];
        let confirm = ["VERIFY", "THEN", "ASSERT"];
        for (tokens, role) in [
            (&evidence[..], LabelRole::Evidence),
            (&change[..], LabelRole::Change),
            (&confirm[..], LabelRole::Confirm),
        ] {
            for token in tokens {
                let text = format!("{token}: do the thing");
                let label = label_of(&text).unwrap_or_else(|| panic!("{text:?} unrecognised"));
                assert_eq!(label.role, role, "{text:?}");
                assert_eq!(label.start, 0);
                assert_eq!(label.len, token.len() + 1);
            }
        }
        // The assertion discriminates: a table collapsing two roles could not
        // pass either of these.
        assert_eq!(
            label_of("RED: do the thing").unwrap().role,
            LabelRole::Evidence
        );
        assert_ne!(
            label_of("RED: do the thing").unwrap().role,
            LabelRole::Change
        );
        assert_eq!(
            label_of("CHANGE: do the thing").unwrap().role,
            LabelRole::Change
        );
        assert_ne!(
            label_of("CHANGE: do the thing").unwrap().role,
            LabelRole::Evidence
        );
    }

    /// `task-labels` :: "An unrecognised run is a generic label, not a miss".
    #[test]
    fn an_unrecognised_run_is_a_generic_label_not_a_miss() {
        for (text, len) in [
            ("NOTE: see design.md", 5usize),
            ("TODO: later", 5),
            ("HANDOFF: the next session picks this up", 8),
        ] {
            let label = label_of(text).unwrap_or_else(|| panic!("{text:?} returned None"));
            assert_eq!(label.role, LabelRole::Other, "{text:?}");
            assert_eq!(label.start, 0, "{text:?}");
            assert_eq!(label.len, len, "{text:?}");
        }
    }

    /// `task-labels` :: "Matching is case-sensitive and whole-run".
    #[test]
    fn matching_is_case_sensitive_and_whole_run() {
        assert_eq!(label_of("Red: lower"), None);
        assert_eq!(
            label_of("RE D: spaced"),
            Some(Label {
                start: 0,
                len: 2,
                role: LabelRole::Other
            })
        );
        assert_eq!(
            label_of("REDGREEN: joined"),
            Some(Label {
                start: 0,
                len: 9,
                role: LabelRole::Other
            })
        );
    }

    /// `task-labels` :: "The classification reads nothing outside its
    /// argument".
    ///
    /// The discriminating half is the **first** assertion, and it runs inside
    /// `cargo test` rather than being left to a human re-reading the file: this
    /// module's own production slice — everything above `mod tests`, with
    /// comment lines stripped — names none of the four workflow-reading
    /// identifiers. `include_str!` is a compile-time read of this very file, so
    /// the test performs no I/O of its own, and scoping it to the production
    /// slice is what keeps the needles below from matching themselves. A
    /// `use crate::schema::…` added to this module reddens it.
    ///
    /// The rest is the scenario's second clause: `label_of`'s signature admits
    /// a `&str` and nothing else, and no workflow value exists anywhere in
    /// this scope.
    #[test]
    fn the_classification_reads_nothing_outside_its_argument() {
        let whole = include_str!("tasks.rs");
        let production = whole
            .split_once("\nmod tests {")
            .map_or(whole, |(above, _)| above);
        let stripped: String = production
            .lines()
            .filter(|l| {
                let t = l.trim_start();
                !t.starts_with("//")
            })
            .collect::<Vec<_>>()
            .join("\n");
        // Split so the needles do not name themselves in a file this test reads.
        for needle in [
            concat!("schema", "::"),
            concat!("Sche", "ma"),
            concat!("config", ".yaml"),
            concat!(".opens", "pec.yaml"),
        ] {
            assert!(
                !stripped.contains(needle),
                "src/tasks.rs's production slice names {needle:?}: this module \
                 reads no workflow definition"
            );
        }
        // The control: the slice really was read and really does hold code, so
        // the four assertions above cannot pass vacuously against an empty
        // string.
        assert!(
            stripped.contains("pub fn label_of(text: &str) -> Option<Label>"),
            "the production slice was not found, so the scan above proved nothing"
        );

        let recognise: fn(&str) -> Option<Label> = label_of;
        assert_eq!(
            recognise("CHECK: nothing but the argument"),
            Some(Label {
                start: 0,
                len: 6,
                role: LabelRole::Evidence
            })
        );
        // The same text classifies identically however many times it is
        // called and in whatever order, so no global or ambient state is read.
        let first = label_of("GREEN: implement it");
        assert_eq!(label_of("RED: write it").unwrap().role, LabelRole::Evidence);
        assert_eq!(label_of("GREEN: implement it"), first);
    }
}
