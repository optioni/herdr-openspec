//! The tracked-tasks tab's grammar: plain data, on exactly the terms
//! `ui::detail` and `ui::markdown` use — no styling type, no I/O API, every
//! public function parameterised by an interior width. See
//! `openspec/changes/tasks-tab/design.md` -> Boundaries and Contracts.

/// `<gauge> <count cell> <percent cell>`, at most `width` chars, dropping
/// whole fields as it narrows. Empty string at width 0.
///
/// The count cell is [`crate::ui::list::progress_cell`] — not a second
/// `format!` of the same pair — so the bar, the detail header, and the
/// list row can never disagree about a change's progress. The percent
/// cell truncates rather than rounds and is absent at `total == 0`, when
/// the whole line is the count cell alone (`[-]`) with no gauge.
///
/// Degradation is drop-whole in a fixed order: the percent cell first,
/// reclaiming its separating space; then the gauge, reclaiming its own;
/// leaving the count cell alone; and, when even that does not fit, the
/// empty string. Width arithmetic is done in `i64`, the same way the view
/// already avoids a `u16` subtraction underflowing.
pub fn progress_bar(progress: &crate::tasks::Progress, width: u16) -> String {
    if width == 0 {
        return String::new();
    }
    let w = i64::from(width);
    let count_cell = crate::ui::list::progress_cell(progress);
    let count_len = count_cell.chars().count() as i64;

    if progress.total == 0 {
        return if w >= count_len {
            count_cell
        } else {
            String::new()
        };
    }

    let percent = percent_of(progress);
    let percent_cell = format!("{percent}%");
    let percent_len = percent_cell.chars().count() as i64;

    // Full form: gauge + ' ' + count cell + ' ' + percent cell.
    let full_gauge_len = w - count_len - percent_len - 2;
    if full_gauge_len >= 1 {
        let gauge = gauge_of(progress, full_gauge_len as u16);
        return format!("{gauge} {count_cell} {percent_cell}");
    }

    // Drop the percent cell and its separating space.
    let no_percent_gauge_len = w - count_len - 1;
    if no_percent_gauge_len >= 1 {
        let gauge = gauge_of(progress, no_percent_gauge_len as u16);
        return format!("{gauge} {count_cell}");
    }

    // Drop the gauge too: the count cell alone, when it fits.
    if w >= count_len {
        return count_cell;
    }

    String::new()
}

/// `completed * 100 / total`, truncating, with a saturating multiply so no
/// `Progress` value can overflow it. `total == 0` is never passed here —
/// `progress_bar` returns before reaching this for that case.
fn percent_of(progress: &crate::tasks::Progress) -> u64 {
    let completed = progress.completed as u64;
    let total = progress.total as u64;
    completed.saturating_mul(100) / total
}

/// A bare run of exactly `g` characters: `filled` of `█` (U+2588) followed
/// by `g - filled` of `░` (U+2591), where `filled = g * completed / total`
/// in integer arithmetic with a saturating multiply. `filled == g` holds
/// iff `progress.is_complete()`, and `filled == 0` holds whenever
/// `completed == 0` — both properties of plain integer truncation given
/// `completed <= total`.
fn gauge_of(progress: &crate::tasks::Progress, g: u16) -> String {
    let g = u64::from(g);
    let completed = progress.completed as u64;
    let total = progress.total as u64;
    let filled = completed.saturating_mul(g) / total;
    let filled = filled.min(g) as usize;
    let g = g as usize;
    let mut out = String::with_capacity(g * "█".len());
    for _ in 0..filled {
        out.push('█');
    }
    for _ in filled..g {
        out.push('░');
    }
    out
}

/// A blank separator line — an empty segment list, the same shape
/// `ui::markdown::lines` uses between blocks.
fn blank_line() -> crate::ui::markdown::Line {
    crate::ui::markdown::Line {
        segments: Vec::new(),
    }
}

/// One line carrying `text` as a single `Face::plain()` segment.
fn plain_line(text: String) -> crate::ui::markdown::Line {
    crate::ui::markdown::Line {
        segments: vec![crate::ui::markdown::Segment {
            text,
            face: crate::ui::markdown::Face::plain(),
        }],
    }
}

/// A heading line: `level` `#` markers, a space, and `text` verbatim,
/// carrying `Face { heading: Some(level), .. }` spelled out field by
/// field so the view bolds it through the existing `heading` mapping,
/// with no new mapping of its own.
fn heading_line(heading: &crate::tasks::Heading, width: u16) -> crate::ui::markdown::Line {
    let text = format!("{} {}", "#".repeat(heading.level as usize), heading.text);
    // Truncated, never wrapped: the requirement is one line whose text is
    // the heading verbatim, but "no line's text exceeds width" still
    // applies, or a long heading overwrites the detail region's border
    // exactly the way `ui::markdown`'s own long-heading test guards
    // against. Only reached when the text is already too long, so this
    // never pads a heading that already fits.
    let w = width as usize;
    let text = if text.chars().count() > w {
        crate::ui::list::pad_or_truncate_right(&text, w)
    } else {
        text
    };
    crate::ui::markdown::Line {
        segments: vec![crate::ui::markdown::Segment {
            text,
            face: crate::ui::markdown::Face {
                heading: Some(heading.level),
                strong: false,
                emphasis: false,
                code: false,
                link: false,
                quoted: false,
            },
        }],
    }
}

/// Word-wrap `text` to `col` columns: wrap at spaces, hard-split a word
/// longer than `col`, and never lose a tail. Private to this module —
/// reusing `ui::markdown::wrap_prose` would mean making its internal
/// `Run` and folder shapes public for a caller that carries no faces at
/// all, widening `MDSEAM`'s confined module for no reason. `col == 0` is
/// never reached: every caller in this module has already fallen back to
/// the truncated-glyph line before wrapping would be attempted with no
/// column to wrap into.
fn wrap_plain(text: &str, col: usize) -> Vec<String> {
    if col == 0 {
        return vec![String::new()];
    }
    let mut lines: Vec<String> = Vec::new();
    let mut current: Vec<char> = Vec::new();
    for word in text.split(' ').filter(|w| !w.is_empty()) {
        let mut remaining: Vec<char> = word.chars().collect();
        loop {
            if current.is_empty() {
                if remaining.len() <= col {
                    current = remaining;
                    break;
                }
                let (head, tail) = remaining.split_at(col);
                lines.push(head.iter().collect());
                remaining = tail.to_vec();
                continue;
            }
            if current.len() + 1 + remaining.len() <= col {
                current.push(' ');
                current.extend(remaining);
                break;
            }
            lines.push(current.iter().collect());
            current = Vec::new();
        }
    }
    lines.push(current.iter().collect());
    lines
}

/// One `tasks::Item`'s rendered line(s): the prefix — `item.indent`
/// spaces, the three-character glyph, one space — followed by the
/// word-wrapped text at a hanging indent of the prefix's own width. The
/// indent is dropped whole when the full prefix would leave no text
/// column, then the separating space and glyph-only prefix is tried, and
/// when even that leaves no text column, the glyph alone — truncated by
/// `ui::list::pad_or_truncate_right` at `width` — is the whole line, with
/// the item's text discarded rather than wrapped into zero columns.
fn item_lines(item: &crate::tasks::Item, width: u16) -> Vec<crate::ui::markdown::Line> {
    let w = width as usize;
    let glyph = if item.checked { "[x]" } else { "[ ]" };

    let full_prefix_len = item.indent + 4;
    let prefix = if full_prefix_len < w {
        Some((
            format!("{}{glyph} ", " ".repeat(item.indent)),
            full_prefix_len,
        ))
    } else if 4 < w {
        Some((format!("{glyph} "), 4))
    } else {
        None
    };

    let Some((prefix, prefix_len)) = prefix else {
        return vec![plain_line(crate::ui::list::pad_or_truncate_right(glyph, w))];
    };

    let col = w - prefix_len;
    let wrapped = wrap_plain(&item.text, col);
    let indent = " ".repeat(prefix_len);
    wrapped
        .into_iter()
        .enumerate()
        .map(|(i, text)| {
            if i == 0 {
                plain_line(format!("{prefix}{text}"))
            } else {
                plain_line(format!("{indent}{text}"))
            }
        })
        .collect()
}

/// The tracked-tasks tab's body: the bar, a blank line, then
/// `tasks::parse`'s groups. Empty vector at width 0, matching
/// `ui::markdown::lines`.
pub fn lines(
    source: &str,
    progress: &crate::tasks::Progress,
    width: u16,
) -> Vec<crate::ui::markdown::Line> {
    if width == 0 {
        return Vec::new();
    }

    // An empty source is `artifact-content`'s "No content yet" state, not
    // this capability's "No tasks yet" — the two SHALL NOT be conflated.
    // Both bodies return nothing for an empty `detail.source`, matching
    // `ui::markdown::lines`, so `content_lines`' outer `No content yet`
    // fallback fires regardless of which body was selected.
    if source.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    let bar = progress_bar(progress, width);
    if !bar.is_empty() {
        out.push(plain_line(bar));
        out.push(blank_line());
    }

    let tasks = crate::tasks::parse(source);

    // The trigger is items, not groups: `task-groups` requires `parse` to
    // emit a group for every heading it recognises, including one holding
    // no items, so a prose file opening with a heading returns one group
    // and zero items. A document with zero items renders no heading line
    // at all, because a heading with nothing under it anywhere is not a
    // section.
    if tasks.progress().total == 0 {
        out.push(plain_line("No tasks yet".to_string()));
        return out;
    }

    let last_index = tasks.groups.len().saturating_sub(1);
    for (index, group) in tasks.groups.iter().enumerate() {
        if let Some(heading) = &group.heading {
            out.push(heading_line(heading, width));
        }
        for item in &group.items {
            out.extend(item_lines(item, width));
        }
        if index != last_index {
            out.push(blank_line());
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::{lines, progress_bar};
    use crate::tasks::Progress;

    #[test]
    fn bar_full_grammar() {
        let progress = Progress {
            completed: 4,
            total: 9,
        };
        for width in [78, 58] {
            let bar = progress_bar(&progress, width);
            assert_eq!(bar.chars().count(), width as usize, "width {width}");
            assert!(bar.ends_with("[4/9] 44%"), "width {width}: {bar:?}");
            let gauge: String = bar
                .chars()
                .take(bar.chars().count() - " [4/9] 44%".len())
                .collect();
            assert!(
                gauge.chars().all(|c| c == '█' || c == '░'),
                "width {width}: {gauge:?}"
            );
        }
        let bar78 = progress_bar(&progress, 78);
        let gauge78: String = bar78.chars().take(68).collect();
        assert_eq!(gauge78.matches('█').count(), 30, "78-col gauge fill");
        assert_eq!(gauge78.matches('░').count(), 38, "78-col gauge fill");
        let bar58 = progress_bar(&progress, 58);
        let gauge58: String = bar58.chars().take(48).collect();
        assert_eq!(gauge58.matches('█').count(), 21, "58-col gauge fill");
        assert_eq!(gauge58.matches('░').count(), 27, "58-col gauge fill");
    }

    #[test]
    fn percent_truncates() {
        for width in [78, 58] {
            let cases = [
                (
                    Progress {
                        completed: 2,
                        total: 3,
                    },
                    "66%",
                ),
                (
                    Progress {
                        completed: 1,
                        total: 3,
                    },
                    "33%",
                ),
                (
                    Progress {
                        completed: 0,
                        total: 7,
                    },
                    "0%",
                ),
                (
                    Progress {
                        completed: 7,
                        total: 7,
                    },
                    "100%",
                ),
            ];
            for (progress, want) in cases {
                let bar = progress_bar(&progress, width);
                assert!(
                    bar.ends_with(want),
                    "width {width}: {progress:?} -> {bar:?}, want ending {want:?}"
                );
                assert_eq!(bar.chars().count(), width as usize, "width {width}");
            }
        }
    }

    #[test]
    fn gauge_full_only_when_complete() {
        for width in [78, 58] {
            let almost = progress_bar(
                &Progress {
                    completed: 99,
                    total: 100,
                },
                width,
            );
            assert!(almost.contains('░'), "width {width}: {almost:?}");

            let complete = progress_bar(
                &Progress {
                    completed: 100,
                    total: 100,
                },
                width,
            );
            assert!(!complete.contains('░'), "width {width}: {complete:?}");
            assert!(complete.ends_with("100%"), "width {width}: {complete:?}");

            let untouched = progress_bar(
                &Progress {
                    completed: 0,
                    total: 100,
                },
                width,
            );
            assert!(!untouched.contains('█'), "width {width}: {untouched:?}");
            assert!(untouched.ends_with("0%"), "width {width}: {untouched:?}");
        }
    }

    #[test]
    fn gauge_property_sweep() {
        let cases = [
            Progress {
                completed: 0,
                total: 0,
            },
            Progress {
                completed: 0,
                total: 1,
            },
            Progress {
                completed: 1,
                total: 1,
            },
            Progress {
                completed: 1,
                total: 2,
            },
            Progress {
                completed: 3,
                total: 7,
            },
            Progress {
                completed: 999,
                total: 1000,
            },
        ];
        for progress in cases {
            let cell = crate::ui::list::progress_cell(&progress);
            let cell_len = cell.chars().count();
            for width in 0..=120 {
                let bar = progress_bar(&progress, width);
                assert!(
                    bar.chars().count() <= width as usize,
                    "{progress:?} width {width}: {bar:?} exceeds its width"
                );
                if bar.chars().any(|c| c == '█' || c == '░') {
                    assert_eq!(
                        !bar.contains('░'),
                        progress.is_complete(),
                        "{progress:?} width {width}: {bar:?}"
                    );
                }
                // The count cell is dropped only when it does not fit —
                // a bar that stays empty at every width that could hold
                // it pins nothing.
                if width as usize >= cell_len {
                    assert!(
                        bar.contains(cell.as_str()),
                        "{progress:?} width {width}: {bar:?} does not carry {cell:?}"
                    );
                }
            }
            // The mandated pair, asserted explicitly by this scenario too.
            for width in [58, 78] {
                let bar = progress_bar(&progress, width);
                assert!(bar.chars().count() <= width as usize, "width {width}");
                assert!(bar.contains(cell.as_str()), "width {width}: {bar:?}");
            }
        }
    }

    #[test]
    fn bar_drops_fields_whole() {
        let progress = Progress {
            completed: 4,
            total: 9,
        };
        for width in [78, 58, 11] {
            let bar = progress_bar(&progress, width);
            assert!(
                bar.chars().any(|c| c == '█' || c == '░'),
                "width {width}: {bar:?}"
            );
            assert!(bar.contains("[4/9]"), "width {width}: {bar:?}");
            assert!(bar.ends_with('%'), "width {width}: {bar:?}");
        }
        for width in [10, 7] {
            let bar = progress_bar(&progress, width);
            assert!(
                bar.chars().any(|c| c == '█' || c == '░'),
                "width {width}: {bar:?}"
            );
            assert!(bar.contains("[4/9]"), "width {width}: {bar:?}");
            assert!(!bar.contains('%'), "width {width}: {bar:?}");
        }
        for width in [6, 5] {
            let bar = progress_bar(&progress, width);
            assert_eq!(bar, "[4/9]", "width {width}");
        }
        for width in [4, 1, 0] {
            let bar = progress_bar(&progress, width);
            assert_eq!(bar, "", "width {width}");
        }
    }

    #[test]
    fn bar_never_partial() {
        let progress = Progress {
            completed: 4,
            total: 9,
        };
        let widths: Vec<u16> = (0..=30).chain([58, 78]).collect();
        for width in widths {
            let bar = progress_bar(&progress, width);
            assert!(
                bar.contains("[4/9]") || !bar.contains('['),
                "width {width}: {bar:?}"
            );
            if let Some(pct) = bar.strip_suffix('%') {
                let digits = pct
                    .rsplit(|c: char| !c.is_ascii_digit())
                    .next()
                    .unwrap_or("");
                assert!(!digits.is_empty(), "width {width}: {bar:?}");
            }
            assert!(!bar.contains("  "), "width {width}: {bar:?}");
            // "[4/9]" is five characters; a width of at least that many
            // must carry it, or the assertions above pass vacuously on a
            // bar that is always empty.
            if width as usize >= 5 {
                assert!(bar.contains("[4/9]"), "width {width}: {bar:?}");
            }
        }
    }

    #[test]
    fn no_tasks_bar_is_the_count_cell() {
        let progress = Progress {
            completed: 0,
            total: 0,
        };
        for width in [78, 58, 4, 3] {
            assert_eq!(progress_bar(&progress, width), "[-]", "width {width}");
        }
        for width in [2, 1, 0] {
            assert_eq!(progress_bar(&progress, width), "", "width {width}");
        }
    }

    // --- group 5: the checklist grammar --------------------------------

    fn long_paragraph(target_chars: usize) -> String {
        let mut s = String::new();
        let mut i = 0usize;
        while s.chars().count() < target_chars {
            if !s.is_empty() {
                s.push(' ');
            }
            s.push_str(&format!("word{i}"));
            i += 1;
        }
        s
    }

    /// The first item line's index in `out`, after the bar and its blank
    /// line — omitted entirely when the bar renders as the empty string at
    /// `width`, which happens well before the checklist's own text runs out
    /// of room.
    fn first_content_index(progress: &Progress, width: u16) -> usize {
        if progress_bar(progress, width).is_empty() {
            0
        } else {
            2
        }
    }

    #[test]
    fn groups_headings_items() {
        let source = "## 1. Setup\n\n- [x] 1.1 first\n- [ ] 1.2 second\n\n\
                       ## 2. Build\n\n- [ ] 2.1 third\n";
        let progress = Progress {
            completed: 1,
            total: 3,
        };
        for width in [78, 58] {
            let out = lines(source, &progress, width);
            let bar = progress_bar(&progress, width);
            let texts: Vec<String> = out.iter().map(|l| l.text()).collect();
            assert_eq!(
                texts,
                vec![
                    bar,
                    String::new(),
                    "## 1. Setup".to_string(),
                    "[x] 1.1 first".to_string(),
                    "[ ] 1.2 second".to_string(),
                    String::new(),
                    "## 2. Build".to_string(),
                    "[ ] 2.1 third".to_string(),
                ],
                "width {width}"
            );
            let heading_idxs = [2usize, 6usize];
            for (i, line) in out.iter().enumerate() {
                let face = line.segments.first().map(|s| s.face).unwrap_or_default();
                if heading_idxs.contains(&i) {
                    assert_eq!(face.heading, Some(2), "width {width} line {i}");
                } else if !line.segments.is_empty() {
                    assert_eq!(
                        face,
                        crate::ui::markdown::Face::plain(),
                        "width {width} line {i}"
                    );
                }
            }
        }
    }

    #[test]
    fn nested_indent() {
        let source = "- [ ] parent\n  - [x] child\n    - [ ] grandchild\n";
        let progress = Progress {
            completed: 1,
            total: 3,
        };
        for width in [78, 58] {
            let out = lines(source, &progress, width);
            let texts: Vec<String> = out.iter().map(|l| l.text()).collect();
            assert_eq!(
                texts,
                vec![
                    progress_bar(&progress, width),
                    String::new(),
                    "[ ] parent".to_string(),
                    "  [x] child".to_string(),
                    "    [ ] grandchild".to_string(),
                ],
                "width {width}: flat, in order, no heading line"
            );
        }
    }

    #[test]
    fn long_item_hanging_indent() {
        let text = long_paragraph(200);
        let source = format!("- [ ] {text}\n");
        let progress = Progress {
            completed: 0,
            total: 1,
        };
        let mut line_counts = std::collections::HashMap::new();
        for width in [78, 58] {
            let out = lines(&source, &progress, width);
            let start = first_content_index(&progress, width);
            let item_lines = &out[start..];
            assert!(!item_lines.is_empty(), "width {width}");
            for (i, line) in item_lines.iter().enumerate() {
                let t = line.text();
                assert!(
                    t.chars().count() <= width as usize,
                    "width {width} line {i}: {t:?}"
                );
                if i == 0 {
                    assert!(t.starts_with("[ ] "), "width {width}: {t:?}");
                } else {
                    assert!(t.starts_with("    "), "width {width} line {i}: {t:?}");
                }
            }
            line_counts.insert(width, item_lines.len());
        }
        assert!(
            line_counts[&58] > line_counts[&78],
            "{line_counts:?}: the 58-column call must produce strictly more lines"
        );
    }

    #[test]
    fn unbreakable_word_hard_split() {
        let text: String = (0..300)
            .map(|i| char::from(b'a' + (i % 26) as u8))
            .collect();
        let source = format!("- [x] {text}\n");
        let progress = Progress {
            completed: 1,
            total: 1,
        };
        for width in [78, 58] {
            let out = lines(&source, &progress, width);
            let start = first_content_index(&progress, width);
            let item_lines = &out[start..];
            let mut reassembled = String::new();
            for (i, line) in item_lines.iter().enumerate() {
                let t = line.text();
                assert!(
                    t.chars().count() <= width as usize,
                    "width {width} line {i}: {t:?}"
                );
                let stripped = if i == 0 {
                    t.strip_prefix("[x] ").unwrap_or(&t).to_string()
                } else {
                    t.trim_start_matches(' ').to_string()
                };
                reassembled.push_str(&stripped);
            }
            assert_eq!(reassembled, text, "width {width}");
        }
    }

    #[test]
    fn indent_dropped_whole() {
        let source = "      - [x] alpha\n";
        let progress = Progress {
            completed: 1,
            total: 1,
        };
        for width in [78, 58, 12, 6, 5, 4, 3, 2, 1, 0] {
            let out = lines(source, &progress, width);
            for line in &out {
                assert!(
                    line.text().chars().count() <= width as usize,
                    "width {width}: {:?}",
                    line.text()
                );
            }
        }
        for width in [78, 58, 12] {
            let out = lines(source, &progress, width);
            let start = first_content_index(&progress, width);
            let item = out[start].text();
            assert!(item.starts_with("      [x]"), "width {width}: {item:?}");
        }
        for width in [6, 5, 4, 3] {
            let out = lines(source, &progress, width);
            let start = first_content_index(&progress, width);
            let item = out[start].text();
            assert!(item.starts_with("[x]"), "width {width}: {item:?}");
            assert!(!item.starts_with(' '), "width {width}: {item:?}");
        }
        let out0 = lines(source, &progress, 0);
        assert!(out0.is_empty());
    }

    #[test]
    fn empty_group_keeps_heading() {
        let source = "## 1. Empty\n\nsome prose\n\n## 2. Full\n\n- [ ] only\n";
        let progress = Progress {
            completed: 0,
            total: 1,
        };
        for width in [78, 58] {
            let out = lines(source, &progress, width);
            let texts: Vec<String> = out.iter().map(|l| l.text()).collect();
            assert!(
                texts.contains(&"## 1. Empty".to_string()),
                "width {width}: {texts:?}"
            );
            assert!(
                texts.contains(&"## 2. Full".to_string()),
                "width {width}: {texts:?}"
            );
            assert!(
                texts.contains(&"[ ] only".to_string()),
                "width {width}: {texts:?}"
            );
            assert!(
                !texts.iter().any(|t| t.contains("prose")),
                "width {width}: {texts:?}"
            );
            let empty_idx = texts.iter().position(|t| t == "## 1. Empty").unwrap();
            let full_idx = texts.iter().position(|t| t == "## 2. Full").unwrap();
            assert_eq!(
                full_idx - empty_idx,
                2,
                "width {width}: heading, blank, heading — {texts:?}"
            );
        }
    }

    #[test]
    fn headingless_leading_group() {
        let source = "- [x] loose\n\n## 1. Later\n\n- [ ] grouped\n";
        let progress = Progress {
            completed: 1,
            total: 2,
        };
        for width in [78, 58] {
            let out = lines(source, &progress, width);
            let texts: Vec<String> = out.iter().map(|l| l.text()).collect();
            let loose_idx = texts.iter().position(|t| t == "[x] loose").unwrap();
            let later_idx = texts.iter().position(|t| t == "## 1. Later").unwrap();
            assert!(loose_idx < later_idx, "width {width}: {texts:?}");
            assert_eq!(
                later_idx - loose_idx,
                2,
                "width {width}: exactly one blank line between groups — {texts:?}"
            );
            assert!(
                texts.contains(&"[ ] grouped".to_string()),
                "width {width}: {texts:?}"
            );
        }
    }

    /// Caught by `ui::view::tests::a_missing_artifact_still_shows_its_tab_and_reads_no_content_yet`
    /// (group 7): a marked artifact whose `paths` resolve to no file has a
    /// non-zero `progress` (`change-artifacts`' `tasks.md` fallback
    /// counted a file the marked artifact itself did not resolve to), and
    /// an unguarded `lines` rendered the bar plus `No tasks yet` for the
    /// empty source, instead of the empty vector `content_lines`'
    /// `No content yet` fallback needs. An empty source is a different
    /// state from one that exists and holds no items, and the two SHALL
    /// NOT be conflated.
    #[test]
    fn empty_source_returns_nothing_regardless_of_progress() {
        for width in [78, 58] {
            let out = lines(
                "",
                &Progress {
                    completed: 4,
                    total: 9,
                },
                width,
            );
            assert!(out.is_empty(), "width {width}: {out:?}");
        }
    }

    /// Found in Change Review: `heading_line` took no `width` and emitted
    /// a heading verbatim, so a heading longer than the interior
    /// overwrote the detail region's border — the buffer-level control
    /// (`ui::view::tests::detail_content_never_overwrites_the_border`)
    /// reproduces the same claim end to end.
    #[test]
    fn a_long_heading_is_truncated_not_wrapped() {
        let heading_text = "x".repeat(200);
        let source = format!("## {heading_text}\n\n- [ ] a\n");
        let progress = Progress {
            completed: 0,
            total: 1,
        };
        for width in [78, 58] {
            let out = lines(&source, &progress, width);
            for line in &out {
                assert!(
                    line.text().chars().count() <= width as usize,
                    "width {width}: {:?} exceeds its width",
                    line.text()
                );
            }
            let heading_line = out
                .iter()
                .find(|l| l.text().starts_with("##"))
                .expect("a heading line");
            assert!(
                heading_line.text().starts_with("## xxx"),
                "width {width}: {:?}",
                heading_line.text()
            );
            // Exactly one heading line — truncated, not wrapped into more.
            assert_eq!(
                out.iter().filter(|l| l.text().starts_with("##")).count(),
                1,
                "width {width}"
            );
        }
    }
}
