//! The tracked-tasks tab's grammar: plain data, on exactly the terms
//! `ui::detail` and `ui::markdown` use — no styling type, no I/O API, every
//! public function parameterised by an interior width. See
//! `openspec/changes/tasks-tab/design.md` -> Boundaries and Contracts.
//!
//! Every measurement and truncation here reaches the crate's one display-
//! width measure, `crate::ui::layout::columns`/`truncate_columns`, and
//! nowhere counts `char`s. See
//! `openspec/changes/view-fidelity/specs/responsive-layout/spec.md` ->
//! "Display width is measured in terminal columns by one pair of
//! primitives".

use crate::ui::layout::{columns, truncate_columns};

/// `<gauge> <count cell> <percent cell>`, at most `width` display columns,
/// dropping whole fields as it narrows. Empty string at width 0.
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
///
/// `groups` is one `Progress` per task group in document order, which
/// [`segmented_gauge`] uses to mark the group boundaries by alternating shade.
/// It affects **only** which glyph each position is drawn with — never `g`,
/// never `filled`, never either cell, and never the drop-whole order — so an
/// **empty slice** produces a byte-identical line to the one this function
/// produced before `tasks-emphasis`, at every width and for every `Progress`.
/// That is what the detail header and the non-foldable path both rely on.
pub fn progress_bar(
    progress: &crate::tasks::Progress,
    groups: &[crate::tasks::Progress],
    width: u16,
) -> String {
    if width == 0 {
        return String::new();
    }
    let w = i64::from(width);
    let count_cell = crate::ui::list::progress_cell(progress);
    let count_len = columns(&count_cell) as i64;

    if progress.total == 0 {
        return if w >= count_len {
            count_cell
        } else {
            String::new()
        };
    }

    let percent = percent_of(progress);
    let percent_cell = format!("{percent}%");
    let percent_len = columns(&percent_cell) as i64;

    // Full form: gauge + ' ' + count cell + ' ' + percent cell.
    let full_gauge_len = w - count_len - percent_len - 2;
    if full_gauge_len >= 1 {
        let gauge = segmented_gauge(progress, groups, full_gauge_len as u16);
        return format!("{gauge} {count_cell} {percent_cell}");
    }

    // Drop the percent cell and its separating space.
    let no_percent_gauge_len = w - count_len - 1;
    if no_percent_gauge_len >= 1 {
        let gauge = segmented_gauge(progress, groups, no_percent_gauge_len as u16);
        return format!("{gauge} {count_cell}");
    }

    // Drop the gauge too: the count cell alone, when it fits.
    if w >= count_len {
        return count_cell;
    }

    String::new()
}

/// `completed * 100 / total`, truncating, computed in `u128` so
/// `usize::MAX * 100` cannot overflow for any `Progress`. `total == 0` is
/// never passed here — `progress_bar` returns before reaching this for
/// that case.
fn percent_of(progress: &crate::tasks::Progress) -> u64 {
    let completed = progress.completed as u128;
    let total = progress.total as u128;
    (completed * 100 / total) as u64
}

/// A bare run of exactly `g` characters: `filled` of `█` (U+2588) followed
/// by `g - filled` of `░` (U+2591), where `filled = g * completed / total`
/// in integer arithmetic computed in `u128`, so `usize::MAX * g` cannot
/// overflow for any `Progress` and no `u16` `g`. `filled == g` holds iff
/// `progress.is_complete()`, and `filled == 0` holds whenever
/// `completed == 0` — both properties of plain integer truncation, and
/// both now hold for **every** `Progress`, with no saturation regime
/// excepted: a saturating `u64` multiply previously produced the wrong
/// *quotient* at `Progress { completed: usize::MAX, total: usize::MAX }`
/// (`u64::MAX / u64::MAX == 1`), not merely a clamped magnitude, which is
/// why widening rather than re-clamping is what repairs it.
///
/// Total: returns the empty string at `g == 0` — an existing property
/// rather than a new one, since `filled` was already `0` there and both
/// push loops were already empty — and at `progress.total == 0`, which
/// removes a division by zero. Neither guard is reachable from either
/// production call site: `progress_bar` returns before reaching this at
/// `total == 0` and draws no zero-width gauge, and `ui::detail::header_row`
/// draws no gauge cell at all at `total == 0`. The guard exists for a
/// `pub(crate)` caller this module does not control.
pub(crate) fn gauge_of(progress: &crate::tasks::Progress, g: u16) -> String {
    if g == 0 || progress.total == 0 {
        return String::new();
    }
    let g = u128::from(g);
    let completed = progress.completed as u128;
    let total = progress.total as u128;
    let filled = (completed * g / total).min(g) as usize;
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

/// [`gauge_of`]'s run with each position's **shade** chosen by which group it
/// falls in: `█`/`░` in an even-indexed contributing group and `▓`/`▒` in an
/// odd-indexed one. A glyph **substitution**, never a second fill computation —
/// each position keeps whether it is filled or empty and changes only which of
/// two shades it is drawn with, so `filled == g` iff complete, `filled == 0`
/// whenever `completed == 0`, and the `u128` arithmetic all hold unchanged and
/// by construction. `gauge_of` itself does not move, so `detail-header`'s
/// twelve-column gauge is untouched.
///
/// A **contributing group** is one whose `total` is greater than zero. An empty
/// group contributes no stretch and **consumes no index**, so two groups left
/// adjacent after empty ones are dropped still alternate — an empty group
/// taking an index would give two neighbours the same shade and erase the
/// boundary between them.
///
/// Each group's stretch is proportional to its item count, by **cumulative flooring**: with
/// contributing totals `t_0 … t_{n-1}` summing to `T`, group `i` owns
/// `[e_{i-1}, e_i)` where `e_i = floor(g * (t_0 + … + t_i) / T)` in `u128` and
/// `e_{-1} = 0`. Those stretches partition the `g` positions exactly, with no
/// position unassigned and none assigned twice, for every `g` and every set of
/// totals — a property of cumulative flooring, with no rounding residue to
/// distribute and no tie-break rule to specify (design.md -> Decision 4).
/// `T` is the **sum of the slice**, never `progress.total`: the bar's own
/// progress is the `Change`'s field, which `change-merge` may have replaced
/// with the CLI's count, and the two are allowed to disagree. A disagreement
/// moves no rendered output, because this chooses glyph variants and never how
/// many positions are filled.
///
/// (The word *stretch* is used throughout rather than the obvious one: this
/// module's own seam check greps the whole file, prose included, for the
/// drawing crate's type names, and one of them is a substring of it. That is
/// the known limit that check already carries, and rewording is its stated
/// repair.)
///
/// Segmentation is **skipped entirely** — the plain `█`/`░` run — when `groups`
/// holds fewer than two contributing groups, when `T` is zero, or when
/// `g < 2 * n`. The last is the legibility floor, stated in columns: a
/// one-column stretch cannot be read as a shade run, so a gauge that cannot give
/// every contributing group two columns shows no boundaries rather than
/// unreliable ones (design.md -> Decision 5).
fn segmented_gauge(
    progress: &crate::tasks::Progress,
    groups: &[crate::tasks::Progress],
    g: u16,
) -> String {
    let run = gauge_of(progress, g);
    let totals: Vec<usize> = groups.iter().map(|p| p.total).filter(|&t| t > 0).collect();
    let n = totals.len();
    let sum: u128 = totals.iter().map(|&t| t as u128).sum();
    if n < 2 || sum == 0 || usize::from(g) < 2 * n {
        return run;
    }

    // `e_i` for each contributing group, in order; `e_{n-1}` is exactly `g`.
    let mut ends = Vec::with_capacity(n);
    let mut cumulative: u128 = 0;
    for &total in &totals {
        cumulative += total as u128;
        ends.push((u128::from(g) * cumulative / sum) as usize);
    }

    let mut out = String::with_capacity(run.len());
    let mut group = 0usize;
    for (position, glyph) in run.chars().enumerate() {
        while group + 1 < n && position >= ends[group] {
            group += 1;
        }
        let odd = group % 2 == 1;
        out.push(match (glyph, odd) {
            ('█', true) => '▓',
            ('░', true) => '▒',
            (g, _) => g,
        });
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

/// One line carrying `text` as a single de-emphasised segment — a completed
/// task item's row, prefix and text alike, its label included. The label's own
/// role is **dropped** rather than dimmed alongside it: leaving a bright
/// `VERIFY:` on a finished task is the exact complaint this change exists to
/// answer (design.md -> Decision 8).
fn muted_line(text: String) -> crate::ui::markdown::Line {
    crate::ui::markdown::Line {
        segments: vec![crate::ui::markdown::Segment {
            text,
            face: crate::ui::markdown::Face {
                muted: true,
                ..crate::ui::markdown::Face::plain()
            },
        }],
    }
}

/// An unchecked item's **first** row, split at its label: the prefix
/// concatenated with everything before the label, the label itself, and the
/// remainder of that row's text — each omitted rather than emitted empty, so an
/// item whose text is exactly its label produces two segments rather than
/// three.
///
/// `label` is looked up against `item.text`, never against the rendered row, so
/// a wrap falling inside `CHARACTERIZE:` cannot half-style it. The caller has
/// already established that `start + len` lies within `row` and on its
/// boundaries; this function re-checks both rather than slicing on trust, so it
/// is total for any `Label` and any `row`.
fn labelled_line(
    prefix: &str,
    row: &str,
    label: crate::tasks::Label,
) -> Option<crate::ui::markdown::Line> {
    let end = label.start.checked_add(label.len)?;
    if end > row.len() || !row.is_char_boundary(label.start) || !row.is_char_boundary(end) {
        return None;
    }
    let plain = crate::ui::markdown::Face::plain();
    let faced = crate::ui::markdown::Face {
        label: Some(label.role),
        ..plain
    };
    let mut segments = Vec::with_capacity(3);
    for (text, face) in [
        (format!("{prefix}{}", &row[..label.start]), plain),
        (row[label.start..end].to_string(), faced),
        (row[end..].to_string(), plain),
    ] {
        if !text.is_empty() {
            segments.push(crate::ui::markdown::Segment { text, face });
        }
    }
    Some(crate::ui::markdown::Line { segments })
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
    let text = if columns(&text) > w {
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
                strikethrough: false,
                // A heading line is neither muted nor labelled: it is the
                // group's own name, not a task item. It carries no delta
                // badge either — that is `ui::detail`'s field to set, on a
                // requirement section's badge segment, never a tracked-tasks
                // group's own heading.
                muted: false,
                label: None,
                delta: None,
            },
        }],
    }
}

/// Split `s` at a grapheme-cluster boundary into a prefix whose [`columns`]
/// is at most `width`, and the remainder — never splitting a cluster and
/// never panicking. Mirrors `ui::markdown::split_at_columns` exactly, field
/// for field; duplicated rather than shared for the same reason
/// [`wrap_plain`]'s own doc comment gives for the whole wrap: a caller here
/// carries no faces at all, and importing `ui::markdown`'s private helper
/// would widen `MDSEAM`'s confined module for no reason.
///
/// When even the first cluster does not fit — it alone measures more than
/// `width` columns — it is **dropped** rather than emitted, per
/// `tasks-checklist`'s carve-out for an over-wide token: the returned
/// prefix is empty and the remainder skips the dropped cluster, so the
/// caller always makes progress rather than looping on it forever.
fn split_at_columns(s: &str, width: usize) -> (&str, &str) {
    let prefix = truncate_columns(s, width);
    if !prefix.is_empty() || s.is_empty() {
        return (prefix, &s[prefix.len()..]);
    }
    // `truncate_columns` returned empty on non-empty `s`: the first
    // grapheme cluster alone is wider than `width`. Find its byte length by
    // growing the budget one column at a time until something fits —
    // bounded by `s`'s own total columns, at which point `truncate_columns`
    // returns `s` whole, so this always terminates.
    let total = columns(s);
    let mut probe = width + 1;
    loop {
        let candidate = truncate_columns(s, probe);
        if !candidate.is_empty() {
            return ("", &s[candidate.len()..]);
        }
        if probe >= total {
            return ("", "");
        }
        probe += 1;
    }
}

/// Word-wrap `text` to `col` display columns: wrap at spaces, hard-split a
/// word longer than `col` at a grapheme-cluster boundary, and never lose a
/// tail. Private to this module — reusing `ui::markdown::wrap_prose` would
/// mean making its internal `Run` and folder shapes public for a caller
/// that carries no faces at all, widening `MDSEAM`'s confined module for no
/// reason. `col == 0` is never reached: every caller in this module has
/// already fallen back to the truncated-glyph line before wrapping would
/// be attempted with no column to wrap into.
fn wrap_plain(text: &str, col: usize) -> Vec<String> {
    if col == 0 {
        return vec![String::new()];
    }
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_cols = 0usize;
    for word in text.split(' ').filter(|w| !w.is_empty()) {
        let mut remaining = word;
        loop {
            let word_cols = columns(remaining);
            if current.is_empty() {
                if word_cols <= col {
                    current = remaining.to_string();
                    current_cols = word_cols;
                    break;
                }
                let (chunk, rest) = split_at_columns(remaining, col);
                lines.push(chunk.to_string());
                remaining = rest;
                if remaining.is_empty() {
                    break;
                }
                continue;
            }
            if current_cols + 1 + word_cols <= col {
                current.push(' ');
                current.push_str(remaining);
                current_cols += 1 + word_cols;
                break;
            }
            lines.push(std::mem::take(&mut current));
            current_cols = 0;
        }
    }
    lines.push(current);
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
    // The same three-column glyph `ui::markdown` renders for a task-list
    // item, so the two checkbox renderers agree by construction and a future
    // divergence is a failing test rather than a silent inconsistency
    // (design.md -> Decision 6). `✓` (U+2713) is East Asian Neutral and one
    // column, so the prefix is three columns exactly as `[x]` was and no
    // wrap or indent arithmetic moves.
    let glyph = if item.checked { "[✓]" } else { "[ ]" };

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
        let degraded = crate::ui::list::pad_or_truncate_right(glyph, w);
        return vec![if item.checked {
            muted_line(degraded)
        } else {
            plain_line(degraded)
        }];
    };

    let col = w - prefix_len;
    let wrapped = wrap_plain(&item.text, col);
    let indent = " ".repeat(prefix_len);

    // An unchecked item's label, when it has one and it fits on the first row
    // whole. `label_of` reads `item.text`; a label straddling the wrap would
    // otherwise produce a segment reading `CHARACT`, so the item degrades to
    // unlabelled instead (design.md -> Decision 9).
    let label = if item.checked {
        None
    } else {
        crate::tasks::label_of(&item.text)
    };

    wrapped
        .into_iter()
        .enumerate()
        .map(|(i, text)| {
            if item.checked {
                // Every row of a checked item, its first and its continuations
                // alike: one de-emphasised segment, never split at its label.
                return muted_line(if i == 0 {
                    format!("{prefix}{text}")
                } else {
                    format!("{indent}{text}")
                });
            }
            if i == 0 {
                // A label appears once, on the row it was written on.
                if let Some(label) = label
                    && let Some(line) = labelled_line(&prefix, &text, label)
                {
                    return line;
                }
                plain_line(format!("{prefix}{text}"))
            } else {
                plain_line(format!("{indent}{text}"))
            }
        })
        .collect()
}

/// The progress-bar line followed by one blank line — and the **empty
/// vector** when [`progress_bar`] renders as the empty string at `width`,
/// so a bar that does not fit costs no blank line either.
///
/// Extracted from [`lines`] so `artifact-content`'s foldable walk can draw
/// the same two rows above the first fold header that the flat tab draws
/// above its first heading. [`lines`] calls it; the two can therefore not
/// disagree.
pub(crate) fn bar_lines(
    progress: &crate::tasks::Progress,
    groups: &[crate::tasks::Progress],
    width: u16,
) -> Vec<crate::ui::markdown::Line> {
    let bar = progress_bar(progress, groups, width);
    if bar.is_empty() {
        return Vec::new();
    }
    vec![plain_line(bar), blank_line()]
}

/// One or more lines per item, in order — no progress bar, no heading
/// line, no blank separator.
///
/// Takes **parsed items** rather than a source string: [`lines`] already
/// holds `tasks::Group` values and would have to re-serialise each group to
/// call a string-taking form, which is the duplication this extraction
/// exists to remove. A folded tab reaches it through
/// `tasks::parse(&section.text)` on a section body whose own heading has
/// become the fold header.
pub(crate) fn items(items: &[crate::tasks::Item], width: u16) -> Vec<crate::ui::markdown::Line> {
    let mut out = Vec::new();
    for item in items {
        out.extend(item_lines(item, width));
    }
    out
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
    // Both bodies return nothing for an empty `detail.sections`, matching
    // `ui::markdown::lines`, so `content_lines`' outer `No content yet`
    // fallback fires regardless of which body was selected.
    if source.is_empty() {
        return Vec::new();
    }

    let tasks = crate::tasks::parse(source);

    // One `Progress` per parsed group, in document order — this path's own
    // derivation of the slice. The foldable path, which is the one every real
    // `tasks.md` takes, passes `detail.sections`' own values instead and does
    // not re-parse: a second derivation is a second number that can disagree
    // with the header cells drawn beside it.
    let per_group: Vec<crate::tasks::Progress> =
        tasks.groups.iter().map(|g| g.progress()).collect();

    let mut out = bar_lines(progress, &per_group, width);

    // The trigger is items, not groups: `task-groups` requires `parse` to
    // emit a group for every heading it recognises, including one holding
    // no items, so a prose file opening with a heading returns one group
    // and zero items. A document with zero items renders no heading line
    // at all, because a heading with nothing under it anywhere is not a
    // section.
    if tasks.progress().total == 0 {
        out.push(plain_line(crate::ui::list::pad_or_truncate_right(
            "No tasks yet",
            width as usize,
        )));
        return out;
    }

    let last_index = tasks.groups.len().saturating_sub(1);
    for (index, group) in tasks.groups.iter().enumerate() {
        if let Some(heading) = &group.heading {
            out.push(heading_line(heading, width));
        }
        out.extend(items(&group.items, width));
        if index != last_index {
            out.push(blank_line());
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::{columns, gauge_of, lines, progress_bar};
    use crate::tasks::Progress;

    #[test]
    fn bar_full_grammar() {
        let progress = Progress {
            completed: 4,
            total: 9,
        };
        for width in [78, 58] {
            let bar = progress_bar(&progress, &[], width);
            assert_eq!(columns(&bar), width as usize, "width {width}");
            assert!(bar.ends_with("[4/9] 44%"), "width {width}: {bar:?}");
            let gauge: String = bar
                .chars()
                .take(columns(&bar) - " [4/9] 44%".len())
                .collect();
            assert!(
                gauge.chars().all(|c| c == '█' || c == '░'),
                "width {width}: {gauge:?}"
            );
        }
        let bar78 = progress_bar(&progress, &[], 78);
        let gauge78: String = bar78.chars().take(68).collect();
        assert_eq!(gauge78.matches('█').count(), 30, "78-col gauge fill");
        assert_eq!(gauge78.matches('░').count(), 38, "78-col gauge fill");
        let bar58 = progress_bar(&progress, &[], 58);
        let gauge58: String = bar58.chars().take(48).collect();
        assert_eq!(gauge58.matches('█').count(), 21, "58-col gauge fill");
        assert_eq!(gauge58.matches('░').count(), 27, "58-col gauge fill");
    }

    /// `tasks-progress-bar` :: "The percentage truncates rather than rounds".
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
                let bar = progress_bar(&progress, &[], width);
                assert!(
                    bar.ends_with(want),
                    "width {width}: {progress:?} -> {bar:?}, want ending {want:?}"
                );
                assert_eq!(columns(&bar), width as usize, "width {width}");
            }
        }
    }

    /// `tasks-progress-bar` :: "A one-task-short change never renders a full
    /// gauge".
    #[test]
    fn gauge_full_only_when_complete() {
        for width in [78, 58] {
            let almost = progress_bar(
                &Progress {
                    completed: 99,
                    total: 100,
                },
                &[],
                width,
            );
            assert!(almost.contains('░'), "width {width}: {almost:?}");

            let complete = progress_bar(
                &Progress {
                    completed: 100,
                    total: 100,
                },
                &[],
                width,
            );
            assert!(!complete.contains('░'), "width {width}: {complete:?}");
            assert!(complete.ends_with("100%"), "width {width}: {complete:?}");

            let untouched = progress_bar(
                &Progress {
                    completed: 0,
                    total: 100,
                },
                &[],
                width,
            );
            assert!(!untouched.contains('█'), "width {width}: {untouched:?}");
            assert!(untouched.ends_with("0%"), "width {width}: {untouched:?}");
        }
    }

    /// `tasks-progress-bar` :: "The property holds across a swept range of
    /// gauge widths".
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
            let cell_len = columns(&cell);
            for width in 0..=120 {
                let bar = progress_bar(&progress, &[], width);
                assert!(
                    columns(&bar) <= width as usize,
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
                let bar = progress_bar(&progress, &[], width);
                assert!(columns(&bar) <= width as usize, "width {width}");
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
            let bar = progress_bar(&progress, &[], width);
            assert!(
                bar.chars().any(|c| c == '█' || c == '░'),
                "width {width}: {bar:?}"
            );
            assert!(bar.contains("[4/9]"), "width {width}: {bar:?}");
            assert!(bar.ends_with('%'), "width {width}: {bar:?}");
        }
        for width in [10, 7] {
            let bar = progress_bar(&progress, &[], width);
            assert!(
                bar.chars().any(|c| c == '█' || c == '░'),
                "width {width}: {bar:?}"
            );
            assert!(bar.contains("[4/9]"), "width {width}: {bar:?}");
            assert!(!bar.contains('%'), "width {width}: {bar:?}");
        }
        for width in [6, 5] {
            let bar = progress_bar(&progress, &[], width);
            assert_eq!(bar, "[4/9]", "width {width}");
        }
        for width in [4, 1, 0] {
            let bar = progress_bar(&progress, &[], width);
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
            let bar = progress_bar(&progress, &[], width);
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
            assert_eq!(progress_bar(&progress, &[], width), "[-]", "width {width}");
        }
        for width in [2, 1, 0] {
            assert_eq!(progress_bar(&progress, &[], width), "", "width {width}");
        }
    }

    // --- group 5: the checklist grammar --------------------------------

    fn long_paragraph(target_chars: usize) -> String {
        let mut s = String::new();
        let mut i = 0usize;
        while columns(&s) < target_chars {
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
        if progress_bar(progress, &[], width).is_empty() {
            0
        } else {
            2
        }
    }

    /// The prefix a labelled item's first plain segment carries, and the two
    /// helpers every test below reads a row through.
    fn segment_texts(line: &crate::ui::markdown::Line) -> Vec<String> {
        line.segments.iter().map(|s| s.text.clone()).collect()
    }

    fn segment_faces(line: &crate::ui::markdown::Line) -> Vec<crate::ui::markdown::Face> {
        line.segments.iter().map(|s| s.face).collect()
    }

    fn labelled(role: crate::tasks::LabelRole) -> crate::ui::markdown::Face {
        crate::ui::markdown::Face {
            label: Some(role),
            ..crate::ui::markdown::Face::plain()
        }
    }

    fn muted() -> crate::ui::markdown::Face {
        crate::ui::markdown::Face {
            muted: true,
            ..crate::ui::markdown::Face::plain()
        }
    }

    /// `tasks-checklist` :: "A labelled unchecked item splits into three
    /// segments".
    ///
    /// The byte-identical leg compares against literals recorded from HEAD
    /// before this change (`notes/head-output.md`, task 0.5), never against a
    /// fresh call of the function under test — the second form could not fail
    /// and would turn this change's central claim, that it moved no character,
    /// into a tautology.
    #[test]
    fn a_labelled_unchecked_item_splits_into_three_segments() {
        let parsed = crate::tasks::parse(
            "- [ ] 1.1 RED: write the failing test\n- [ ] Commit: the parser\n",
        );
        let items = &parsed.groups[0].items;
        for width in [78, 58] {
            let out = super::items(items, width);
            assert_eq!(out.len(), 2, "width {width}: neither item wraps here");

            assert_eq!(
                segment_texts(&out[0]),
                vec!["[ ] 1.1 ", "RED:", " write the failing test"],
                "width {width}: three segments, split at the label"
            );
            assert_eq!(
                segment_faces(&out[0]),
                vec![
                    crate::ui::markdown::Face::plain(),
                    labelled(crate::tasks::LabelRole::Evidence),
                    crate::ui::markdown::Face::plain(),
                ],
                "width {width}: only the middle segment is faced"
            );

            // `Commit:` is a one-letter uppercase run and so no label at all.
            assert_eq!(
                segment_texts(&out[1]),
                vec!["[ ] Commit: the parser"],
                "width {width}: one plain segment"
            );
            assert_eq!(
                segment_faces(&out[1]),
                vec![crate::ui::markdown::Face::plain()],
                "width {width}"
            );

            // Recorded at HEAD, at both of these widths.
            assert_eq!(
                out.iter().map(|l| l.text()).collect::<Vec<_>>(),
                vec![
                    "[ ] 1.1 RED: write the failing test".to_string(),
                    "[ ] Commit: the parser".to_string(),
                ],
                "width {width}: the split moved a character"
            );
        }
    }

    /// `tasks-checklist` :: "A checked item is de-emphasised whole, label
    /// included".
    #[test]
    fn a_checked_item_is_de_emphasised_whole_label_included() {
        let checked = crate::tasks::parse("- [x] 1.1 VERIFY: make check is green\n");
        let unchecked = crate::tasks::parse("- [ ] 1.1 VERIFY: make check is green\n");
        for width in [78, 58] {
            let out = super::items(&checked.groups[0].items, width);
            assert_eq!(out.len(), 1, "width {width}");
            assert_eq!(
                segment_texts(&out[0]),
                vec!["[✓] 1.1 VERIFY: make check is green"],
                "width {width}: one segment carrying the whole row"
            );
            assert_eq!(segment_faces(&out[0]), vec![muted()], "width {width}");
            // The de-emphasis is not a dimmed `VERIFY:` but no `VERIFY:` role
            // at all.
            assert_eq!(out[0].segments[0].face.label, None, "width {width}");

            // The same text unchecked: three segments with the label role on
            // the middle one. Asserting the pair against each other is what a
            // rule muting both, or neither, could not pass.
            let twin = super::items(&unchecked.groups[0].items, width);
            assert_eq!(
                segment_texts(&twin[0]),
                vec!["[ ] 1.1 ", "VERIFY:", " make check is green"],
                "width {width}"
            );
            assert_eq!(
                segment_faces(&twin[0]),
                vec![
                    crate::ui::markdown::Face::plain(),
                    labelled(crate::tasks::LabelRole::Confirm),
                    crate::ui::markdown::Face::plain(),
                ],
                "width {width}"
            );

            // Recorded at HEAD, at both widths.
            assert_eq!(out[0].text(), "[✓] 1.1 VERIFY: make check is green");
            assert_eq!(twin[0].text(), "[ ] 1.1 VERIFY: make check is green");
        }
    }

    /// `tasks-checklist` :: "A wrapped labelled item labels only its first
    /// row".
    #[test]
    fn a_wrapped_labelled_item_labels_only_its_first_row() {
        let words = ["abcdefgh"; 20].join(" ");
        let source = format!("- [ ] 1.1 GREEN: {words}\n");
        let parsed = crate::tasks::parse(&source);
        let items = &parsed.groups[0].items;

        // Recorded at HEAD, at both mandated interior widths.
        let recorded: [(u16, &[&str]); 2] = [
            (
                78,
                &[
                    "[ ] 1.1 GREEN: abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh",
                    "    abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh",
                    "    abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh",
                ],
            ),
            (
                58,
                &[
                    "[ ] 1.1 GREEN: abcdefgh abcdefgh abcdefgh abcdefgh",
                    "    abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh",
                    "    abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh",
                    "    abcdefgh abcdefgh abcdefgh abcdefgh",
                ],
            ),
        ];

        for (width, expected) in recorded {
            let out = super::items(items, width);
            assert_eq!(
                out.iter().map(|l| l.text()).collect::<Vec<_>>(),
                expected.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                "width {width}: the split moved a character"
            );
            assert!(out.len() > 1, "width {width}: the item must wrap");

            // The first row splits at its label; the tail is that row's own
            // remaining text, which is longer at 78 than at 58.
            let first = &expected[0]["[ ] 1.1 GREEN:".len()..];
            assert_eq!(
                segment_texts(&out[0]),
                vec!["[ ] 1.1 ", "GREEN:", first],
                "width {width}: three segments, split at the label"
            );
            assert_eq!(
                segment_faces(&out[0]),
                vec![
                    crate::ui::markdown::Face::plain(),
                    labelled(crate::tasks::LabelRole::Change),
                    crate::ui::markdown::Face::plain(),
                ],
                "width {width}"
            );

            // A label appears once, on the row it was written on.
            for (i, line) in out.iter().enumerate().skip(1) {
                assert_eq!(
                    segment_faces(line),
                    vec![crate::ui::markdown::Face::plain()],
                    "width {width} continuation row {i}"
                );
                assert_eq!(line.segments[0].face.label, None, "width {width} row {i}");
                assert!(!line.segments[0].face.muted, "width {width} row {i}");
            }

            // No character was lost to the split: strip the prefix from the
            // first row and the hanging indent from the rest, and the item's
            // whole text comes back.
            let mut rebuilt = out[0].text()["[ ] ".len()..].to_string();
            for line in out.iter().skip(1) {
                rebuilt.push(' ');
                rebuilt.push_str(line.text().trim_start());
            }
            assert_eq!(rebuilt, items[0].text, "width {width}");
        }

        assert!(
            super::items(items, 58).len() > super::items(items, 78).len(),
            "58 must wrap more than 78, or the width does not reach the wrap"
        );
    }

    /// `tasks-checklist` :: "A label split across a wrap degrades to
    /// unlabelled".
    ///
    /// The three narrow widths are where the first row's text column ends
    /// before `CHARACTERIZE:` is whole; 58 and 78 are the contrast, and are
    /// named here for `TASKWIDTHS`, which carries no exemption list.
    #[test]
    fn a_label_split_across_a_wrap_degrades_to_unlabelled() {
        let parsed = crate::tasks::parse("- [ ] 1.1 CHARACTERIZE: record the baseline\n");
        let items = &parsed.groups[0].items;

        // Recorded at HEAD, at each of the three narrow widths.
        let recorded: [(u16, &[&str]); 3] = [
            (
                16,
                &[
                    "[ ] 1.1",
                    "    CHARACTERIZE",
                    "    : record the",
                    "    baseline",
                ],
            ),
            (
                14,
                &[
                    "[ ] 1.1",
                    "    CHARACTERI",
                    "    ZE: record",
                    "    the",
                    "    baseline",
                ],
            ),
            (
                12,
                &[
                    "[ ] 1.1",
                    "    CHARACTE",
                    "    RIZE:",
                    "    record",
                    "    the",
                    "    baseline",
                ],
            ),
        ];
        for (width, expected) in recorded {
            let out = super::items(items, width);
            assert_eq!(
                out.iter().map(|l| l.text()).collect::<Vec<_>>(),
                expected.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                "width {width}: the degradation moved a character"
            );
            for (i, line) in out.iter().enumerate() {
                assert_eq!(
                    segment_faces(line),
                    vec![crate::ui::markdown::Face::plain()],
                    "width {width} row {i}: one plain segment, no partial label"
                );
            }
        }

        // The contrast: where the label fits the first row whole, the same item
        // does split, so the degradation is width-driven rather than
        // unconditional.
        for width in [58, 78] {
            let out = super::items(items, width);
            assert_eq!(
                segment_texts(&out[0]),
                vec!["[ ] 1.1 ", "CHARACTERIZE:", " record the baseline"],
                "width {width}"
            );
            assert_eq!(
                segment_faces(&out[0])[1],
                labelled(crate::tasks::LabelRole::Evidence),
                "width {width}"
            );
            assert_eq!(
                out[0].text(),
                "[ ] 1.1 CHARACTERIZE: record the baseline",
                "width {width}: recorded at HEAD"
            );
        }
    }

    fn p(completed: usize, total: usize) -> Progress {
        Progress { completed, total }
    }

    /// The four glyphs the segmented gauge can draw, and the two counts every
    /// test below reads a run through.
    fn gauge_run(bar: &str) -> String {
        bar.chars().take_while(|c| "█░▓▒".contains(*c)).collect()
    }

    fn filled_count(bar: &str) -> usize {
        gauge_run(bar)
            .chars()
            .filter(|&c| c == '█' || c == '▓')
            .count()
    }

    /// `tasks-progress-bar` :: "Two groups of unequal size get spans
    /// proportional to their item counts".
    #[test]
    fn two_groups_of_unequal_size_get_spans_proportional_to_their_item_counts() {
        let progress = p(3, 12);
        let groups = [p(3, 9), p(0, 3)];
        for width in [78, 58] {
            let bar = progress_bar(&progress, &groups, width);
            let run = gauge_run(&bar);
            let g = run.chars().count();
            let even = run.chars().filter(|&c| c == '█' || c == '░').count();
            let odd = run.chars().filter(|&c| c == '▓' || c == '▒').count();
            assert_eq!(even + odd, g, "width {width}: every position is drawn");
            assert_eq!(even, g * 9 / 12, "width {width}: the first span");
            assert_eq!(odd, g - g * 9 / 12, "width {width}: the second span");
            // The first `floor(g * 9 / 12)` positions are the `█`/`░` pair and
            // the remainder the `▓`/`▒` pair, in that order.
            assert!(
                run.chars().take(even).all(|c| c == '█' || c == '░'),
                "width {width}: {run:?}"
            );
            assert!(
                run.chars().skip(even).all(|c| c == '▓' || c == '▒'),
                "width {width}: {run:?}"
            );

            // The fill is unchanged: the same count the unsegmented gauge
            // produces.
            assert_eq!(filled_count(&bar), g * 3 / 12, "width {width}");

            // And the empty-slice call differs only in the shades.
            let plain = progress_bar(&progress, &[], width);
            assert_eq!(
                bar.replace('▓', "█").replace('▒', "░"),
                plain,
                "width {width}: segmentation moved something other than a glyph"
            );
        }
    }

    /// `tasks-progress-bar` :: "An empty group contributes no span and consumes
    /// no index".
    #[test]
    fn an_empty_group_contributes_no_span_and_consumes_no_index() {
        let progress = p(0, 4);
        let with_empty = [p(0, 2), p(0, 0), p(0, 2)];
        let without = [p(0, 2), p(0, 2)];
        for width in [78, 58] {
            let bar = progress_bar(&progress, &with_empty, width);
            let run = gauge_run(&bar);
            let g = run.chars().count();
            let first = run.chars().filter(|&c| c == '░').count();
            let second = run.chars().filter(|&c| c == '▒').count();
            assert_eq!(first + second, g, "width {width}: every position is drawn");
            assert_eq!(first, g / 2, "width {width}: the first half");
            assert!(
                run.chars().take(first).all(|c| c == '░'),
                "width {width}: {run:?}"
            );
            assert!(
                run.chars().skip(first).all(|c| c == '▒'),
                "width {width}: {run:?}"
            );

            // The empty group took no index: the two contributing groups are
            // `0` and `1`, so dropping it is what the rule does rather than
            // merely what it permits.
            assert_eq!(
                bar,
                progress_bar(&progress, &without, width),
                "width {width}: byte-identical with the empty group removed"
            );
        }
    }

    /// `tasks-progress-bar` :: "Segmentation is skipped below the legibility
    /// floor". 58 and 78 are inside the sweep and are named for `TASKWIDTHS`.
    #[test]
    fn segmentation_is_skipped_below_the_legibility_floor() {
        let progress = p(5, 22);
        let groups: Vec<Progress> = (0..22).map(|_| p(0, 1)).collect();
        let mut below = 0usize;
        let mut above = 0usize;
        for width in 0..=130u16 {
            let bar = progress_bar(&progress, &groups, width);
            let run = gauge_run(&bar);
            let g = run.chars().count();
            if g == 0 {
                continue;
            }
            let shaded = run.chars().any(|c| c == '▓' || c == '▒');
            if g < 44 {
                below += 1;
                assert!(!shaded, "width {width}: g {g} is below the floor: {run:?}");
            } else {
                above += 1;
                assert!(
                    run.chars().any(|c| c == '█' || c == '░')
                        && run.chars().any(|c| c == '▓' || c == '▒'),
                    "width {width}: g {g} is above the floor and holds one pair only: {run:?}"
                );
            }
        }
        assert!(
            below > 0 && above > 0,
            "the sweep must cross the floor in both directions (below {below}, above {above})"
        );

        // At 58 — the narrow mandated interior, the measured worst case the
        // floor was chosen against — `g` is at least 44 and the bar segments.
        // 78 is the wide one, comfortably above it.
        for width in [58, 78] {
            let run = gauge_run(&progress_bar(&progress, &groups, width));
            assert!(run.chars().count() >= 44, "width {width}");
            assert!(
                run.chars().any(|c| c == '▓' || c == '▒'),
                "width {width}: {run:?}"
            );
        }
    }

    /// `tasks-progress-bar` :: "A single group is never segmented".
    #[test]
    fn a_single_group_is_never_segmented() {
        let progress = p(1, 2);
        for width in [78, 58] {
            let bar = progress_bar(&progress, &[p(1, 2)], width);
            assert!(
                !bar.contains('▓') && !bar.contains('▒'),
                "width {width}: one group has no boundary to mark: {bar:?}"
            );
            assert_eq!(
                bar,
                progress_bar(&progress, &[], width),
                "width {width}: byte-identical to the empty-slice call"
            );
        }
    }

    /// `tasks-progress-bar` :: "Segmentation is total and partitions the run
    /// exactly". 58 and 78 are inside the sweep and are named for
    /// `TASKWIDTHS`.
    #[test]
    fn segmentation_is_total_and_partitions_the_run_exactly() {
        let half = usize::MAX / 2;
        let cases: Vec<(Progress, Vec<Progress>)> = vec![
            (p(0, 0), Vec::new()),
            (
                p(usize::MAX, usize::MAX),
                vec![p(half, half), p(half, half)],
            ),
            (p(7, 40), (0..40).map(|_| p(0, 1)).collect()),
            (p(1, 2), vec![p(0, usize::MAX), p(1, 1)]),
            (p(0, 3), (0..100).map(|_| p(0, 0)).collect()),
        ];
        for (index, (progress, groups)) in cases.iter().enumerate() {
            for width in 0..=130u16 {
                let bar = progress_bar(progress, groups, width);
                assert!(
                    columns(&bar) <= width as usize,
                    "case {index} width {width}: {bar:?} exceeds it"
                );
                let run = gauge_run(&bar);
                let g = run.chars().count();
                let counts = ['█', '░', '▓', '▒']
                    .iter()
                    .map(|&glyph| run.chars().filter(|&c| c == glyph).count())
                    .sum::<usize>();
                assert_eq!(
                    counts, g,
                    "case {index} width {width}: a position is unassigned or assigned twice"
                );
                // The substitution provably preserves the fill rather than
                // being asserted to by construction.
                assert_eq!(
                    filled_count(&bar),
                    filled_count(&progress_bar(progress, &[], width)),
                    "case {index} width {width}: the fill moved"
                );
            }
        }

        // The saturating input is not reintroduced by segmentation: every
        // position is a filled glyph and the percent cell reads 100%.
        let groups = [p(half, half), p(half, half)];
        for width in [78, 58] {
            let bar = progress_bar(&p(usize::MAX, usize::MAX), &groups, width);
            let run = gauge_run(&bar);
            assert!(
                run.chars().all(|c| c == '█' || c == '▓'),
                "width {width}: {run:?}"
            );
            assert!(bar.ends_with("100%"), "width {width}: {bar:?}");
        }
    }

    /// `tasks-checklist` :: "A folded group and an unfolded one render the
    /// same item lines". Every expectation here is a **literal**: asserting
    /// that `items`' output equals a slice of `lines`' output could not fail
    /// once `lines` calls `items`, and this repository does not keep tests
    /// that cannot fail.
    #[test]
    fn a_folded_group_and_an_unfolded_one_render_the_same_item_lines() {
        let progress = Progress {
            completed: 1,
            total: 2,
        };
        let parsed = crate::tasks::parse("- [x] 1.1 first\n- [ ] 1.2 second\n");
        let group = &parsed.groups[0];

        for width in [78, 58] {
            // `items` alone: the item rows and nothing else.
            let out = super::items(&group.items, width);
            let texts: Vec<String> = out.iter().map(|l| l.text()).collect();
            assert_eq!(
                texts,
                vec!["[✓] 1.1 first".to_string(), "[ ] 1.2 second".to_string()],
                "width {width}: no bar, no heading, no blank separator"
            );
            // `tasks-emphasis` widened this to the two new fields: the
            // checked item is one de-emphasised segment, label included, and
            // the unchecked one is plain — neither text here holding a label.
            assert_eq!(
                out[0].segments.len(),
                1,
                "width {width}: a checked item is one segment"
            );
            assert_eq!(
                out[0].segments[0].face,
                crate::ui::markdown::Face {
                    muted: true,
                    ..crate::ui::markdown::Face::plain()
                },
                "width {width}: the checked row"
            );
            assert_eq!(
                out[0].segments[0].face.label, None,
                "width {width}: a completed row carries no label role"
            );
            assert_eq!(
                out[1].segments.first().map(|s| s.face),
                Some(crate::ui::markdown::Face::plain()),
                "width {width}: the unchecked row"
            );

            // `bar_lines` alone: the bar row and one blank.
            let bar_out = super::bar_lines(&progress, &[], width);
            let bar_texts: Vec<String> = bar_out.iter().map(|l| l.text()).collect();
            assert_eq!(
                bar_texts,
                vec![progress_bar(&progress, &[], width), String::new()],
                "width {width}: the bar and one blank line"
            );

            // The whole-tab grammar, against literals. One group, so `lines`'
            // own slice segments nothing and the bar is the unsegmented form.
            let whole = lines(
                "## 1. Setup\n\n- [x] 1.1 first\n- [ ] 1.2 second\n",
                &progress,
                width,
            );
            let whole_texts: Vec<String> = whole.iter().map(|l| l.text()).collect();
            assert_eq!(
                whole_texts,
                vec![
                    progress_bar(&progress, &[], width),
                    String::new(),
                    "## 1. Setup".to_string(),
                    "[✓] 1.1 first".to_string(),
                    "[ ] 1.2 second".to_string(),
                ],
                "width {width}"
            );
            assert_eq!(
                whole[2].segments.first().map(|s| s.face.heading),
                Some(Some(2)),
                "width {width}: the heading line"
            );
        }

        // A width where the bar renders as the empty string: `[1/2]` alone
        // measures five columns, so four leaves the bar nothing to draw and
        // `bar_lines` contributes no blank line either.
        assert!(progress_bar(&progress, &[], 4).is_empty());
        assert!(super::bar_lines(&progress, &[], 4).is_empty());
    }

    #[test]
    fn groups_headings_items() {
        let source = "## 1. Setup\n\n- [x] 1.1 first\n- [ ] 1.2 second\n\n\
                       ## 2. Build\n\n- [ ] 2.1 third\n";
        let progress = Progress {
            completed: 1,
            total: 3,
        };
        // `lines` derives its own group slice from its own parse, so the bar it
        // draws is segmented — two groups of two and one item. Naming the
        // slice here rather than passing `&[]` is what keeps this assertion
        // against the grammar `lines` actually produces.
        let per_group = [
            Progress {
                completed: 1,
                total: 2,
            },
            Progress {
                completed: 0,
                total: 1,
            },
        ];
        for width in [78, 58] {
            let out = lines(source, &progress, width);
            let bar = progress_bar(&progress, &per_group, width);
            assert!(
                bar.contains('▓') || bar.contains('▒'),
                "width {width}: two groups, so the bar segments"
            );
            let texts: Vec<String> = out.iter().map(|l| l.text()).collect();
            assert_eq!(
                texts,
                vec![
                    bar,
                    String::new(),
                    "## 1. Setup".to_string(),
                    "[✓] 1.1 first".to_string(),
                    "[ ] 1.2 second".to_string(),
                    String::new(),
                    "## 2. Build".to_string(),
                    "[ ] 2.1 third".to_string(),
                ],
                "width {width}"
            );
            let heading_idxs = [2usize, 6usize];
            // `tasks-emphasis`: line 3 is the one checked item and carries
            // `muted: true`; every other faced row here is plain, no item text
            // in this fixture holding a label.
            let muted_idxs = [3usize];
            for (i, line) in out.iter().enumerate() {
                let face = line.segments.first().map(|s| s.face).unwrap_or_default();
                if heading_idxs.contains(&i) {
                    assert_eq!(face.heading, Some(2), "width {width} line {i}");
                    assert!(
                        !face.muted,
                        "width {width} line {i}: a heading is not muted"
                    );
                    assert_eq!(face.label, None, "width {width} line {i}");
                } else if muted_idxs.contains(&i) {
                    assert_eq!(
                        face,
                        crate::ui::markdown::Face {
                            muted: true,
                            ..crate::ui::markdown::Face::plain()
                        },
                        "width {width} line {i}"
                    );
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
                    progress_bar(&progress, &[], width),
                    String::new(),
                    "[ ] parent".to_string(),
                    "  [✓] child".to_string(),
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
                    columns(&t) <= width as usize,
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
                    columns(&t) <= width as usize,
                    "width {width} line {i}: {t:?}"
                );
                let stripped = if i == 0 {
                    t.strip_prefix("[✓] ").unwrap_or(&t).to_string()
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
                let t = line.text();
                assert!(columns(&t) <= width as usize, "width {width}: {t:?}");
            }
        }
        for width in [78, 58, 12] {
            let out = lines(source, &progress, width);
            let start = first_content_index(&progress, width);
            let item = out[start].text();
            assert!(item.starts_with("      [✓]"), "width {width}: {item:?}");
        }
        for width in [6, 5, 4, 3] {
            let out = lines(source, &progress, width);
            let start = first_content_index(&progress, width);
            let item = out[start].text();
            assert!(item.starts_with("[✓]"), "width {width}: {item:?}");
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
            let loose_idx = texts.iter().position(|t| t == "[✓] loose").unwrap();
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
                let t = line.text();
                assert!(
                    columns(&t) <= width as usize,
                    "width {width}: {t:?} exceeds its width"
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

    // --- group 4: measuring in columns -------------------------------

    /// `tasks-checklist` :: "A checklist of wide-character items fits at both
    /// mandated widths" — the hard-split half. Mirrors
    /// `unbreakable_word_hard_split` above but with a wide-character word, so
    /// a wrap that measures in `chars` instead of columns would pack roughly
    /// twice as many characters per line as the region can hold and this
    /// test's `columns(t) <= width` assertion goes red.
    #[test]
    fn wide_character_item_hard_split_at_mandated_widths() {
        let text: String = "日本語のタスク".repeat(30);
        let source = format!("- [x] {text}\n");
        let progress = Progress {
            completed: 1,
            total: 1,
        };
        for width in [78, 58] {
            let out = lines(&source, &progress, width);
            let start = first_content_index(&progress, width);
            let item_lines = &out[start..];
            assert!(!item_lines.is_empty(), "width {width}");
            let mut reassembled = String::new();
            for (i, line) in item_lines.iter().enumerate() {
                let t = line.text();
                assert!(
                    columns(&t) <= width as usize,
                    "width {width} line {i}: {t:?} exceeds its width"
                );
                let stripped = if i == 0 {
                    t.strip_prefix("[✓] ").unwrap_or(&t).to_string()
                } else {
                    t.trim_start_matches(' ').to_string()
                };
                reassembled.push_str(&stripped);
            }
            assert_eq!(reassembled, text, "width {width}");
            assert!(
                item_lines.iter().any(|l| {
                    let c = columns(&l.text());
                    c == width as usize || c + 1 == width as usize
                }),
                "width {width}: no wrapped line reaches near the region width"
            );
        }
    }

    /// `tasks-checklist` :: "A checklist of wide-character items fits at both
    /// mandated widths" — a heading and several wide items (a repeated CJK
    /// word, an emoji, and a family emoji joined by two zero-width joiners)
    /// together, none of them producing a line wider than the region.
    #[test]
    fn wide_character_checklist_lines_stay_within_width() {
        let family_emoji = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}";
        let source = format!(
            "## 日本語の見出し\n\n- [x] 日本語のタスク\n  - [ ] 🎉 celebrate\n\
             \u{20}   - [ ] {family_emoji} family\n"
        );
        let progress = Progress {
            completed: 1,
            total: 3,
        };
        for width in [78, 58] {
            let out = lines(&source, &progress, width);
            for line in &out {
                let t = line.text();
                assert!(
                    columns(&t) <= width as usize,
                    "width {width}: {t:?} exceeds its width"
                );
            }
            assert!(
                out.iter().any(|l| l.text() == "## 日本語の見出し"),
                "width {width}: heading missing: {:?}",
                out.iter().map(|l| l.text()).collect::<Vec<_>>()
            );
            assert!(
                out.iter().any(|l| l.text().contains('🎉')),
                "width {width}: emoji item missing"
            );
            assert!(
                out.iter().any(|l| l.text().contains(family_emoji)),
                "width {width}: family-emoji item missing"
            );

            // The glyph budget did not move: `✓` (U+2713) is East Asian
            // Neutral and one column, so `[✓]` is three columns exactly as
            // `[x]` was. Asserted against an ASCII checklist of the same
            // *shape* — the same three indents — rather than against a
            // remembered offset, and on BOTH the column the glyph begins at
            // and the column its text begins at: the first pins the indent
            // rule, the second pins the glyph's own width, and a
            // wider-than-one-column glyph moves only the second.
            let ascii = lines(
                "## heading\n\n- [x] alpha\n  - [ ] bravo\n    - [ ] charlie\n",
                &progress,
                width,
            );
            let offsets = |rendered: &[crate::ui::markdown::Line]| -> Vec<(usize, usize)> {
                rendered
                    .iter()
                    .filter_map(|l| {
                        let t = l.text();
                        let glyph = ["[✓] ", "[ ] "]
                            .into_iter()
                            .find_map(|g| t.find(g).map(|byte| (byte, g)));
                        glyph.map(|(byte, g)| (columns(&t[..byte]), columns(&t[..byte + g.len()])))
                    })
                    .collect()
            };
            assert_eq!(
                offsets(&out),
                offsets(&ascii),
                "width {width}: the glyph's interior column offsets moved"
            );
            assert_eq!(
                offsets(&out),
                vec![(0, 4), (2, 6), (4, 8)],
                "width {width}: the three indents and the four-column prefix"
            );
        }
    }

    /// `tasks-checklist` :: "No checklist line exceeds its width at any
    /// width" — a sweep of `0..=130` over five sources (the wide-character
    /// source above, a twenty-item ASCII checklist, a heading with no items,
    /// the empty string, and a source holding a NUL character) against two
    /// `Progress` values, plus the mandated pair named explicitly.
    #[test]
    fn no_checklist_line_exceeds_its_width_at_any_width() {
        let family_emoji = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}";
        let wide_source = format!(
            "## 日本語の見出し\n\n- [x] 日本語のタスク\n- [ ] 🎉 celebrate\n\
             - [ ] {family_emoji} family\n"
        );
        let ascii_source: String = (0..20)
            .map(|i| format!("- [ ] item {i} with some words in it here\n"))
            .collect();
        let heading_only = "## 1. Setup\n\nno items here\n".to_string();
        let empty = String::new();
        let nul_source = "- [ ] a\u{0}b\n".to_string();
        let sources = [
            &wide_source,
            &ascii_source,
            &heading_only,
            &empty,
            &nul_source,
        ];
        let progresses = [
            Progress {
                completed: 4,
                total: 9,
            },
            Progress {
                completed: 0,
                total: 0,
            },
        ];

        for source in sources {
            for progress in &progresses {
                for width in 0u16..=130 {
                    let out = lines(source, progress, width);
                    for line in &out {
                        assert!(
                            columns(&line.text()) <= width as usize,
                            "source {source:?} progress {progress:?} width {width}: {:?} \
                             exceeds its width",
                            line.text()
                        );
                    }
                }
                // The mandated pair, asserted explicitly by this scenario too.
                for width in [78, 58] {
                    let out = lines(source, progress, width);
                    for line in &out {
                        assert!(
                            columns(&line.text()) <= width as usize,
                            "source {source:?} progress {progress:?} width {width}"
                        );
                    }
                }
            }
        }

        // The zero-progress runs include widths 0 through 12, the range in
        // which "No tasks yet" is longer than the region.
        let zero = Progress {
            completed: 0,
            total: 0,
        };
        for width in 0u16..=12 {
            let out = lines(&heading_only, &zero, width);
            for line in &out {
                assert!(columns(&line.text()) <= width as usize, "width {width}");
            }
        }
    }

    /// `tasks-checklist` :: "`No tasks yet` does not eat the border at a
    /// narrow frame" — exercised directly on `ui::tasks::lines`, at the
    /// **content-area** widths a narrow frame produces: a frame of 13 has a
    /// content area of 11 (first truncation), 14 has 12 (fits whole, no
    /// ellipsis), 15 has 13 (fits whole, one padding space), and frames 1
    /// and 2 both collapse to a content area of 0.
    #[test]
    fn no_tasks_yet_does_not_eat_the_border_at_a_narrow_frame() {
        let source = "# Plan\n\nNothing checkable here.\n";
        let progress = Progress {
            completed: 0,
            total: 0,
        };

        let out = lines(source, &progress, 11);
        let texts: Vec<String> = out.iter().map(|l| l.text()).collect();
        assert!(
            texts.contains(&"No tasks y…".to_string()),
            "content area 11: {texts:?}"
        );
        for t in &texts {
            assert!(columns(t) <= 11, "{t:?}");
        }

        let out = lines(source, &progress, 12);
        let texts: Vec<String> = out.iter().map(|l| l.text()).collect();
        assert!(
            texts.contains(&"No tasks yet".to_string()),
            "content area 12: {texts:?}"
        );

        let out = lines(source, &progress, 13);
        let texts: Vec<String> = out.iter().map(|l| l.text()).collect();
        assert!(
            texts.contains(&"No tasks yet ".to_string()),
            "content area 13: {texts:?}"
        );

        // A frame of 1 or 2 columns both produce a content area of 0.
        let out = lines(source, &progress, 0);
        assert!(out.is_empty(), "content area 0: {out:?}");

        // The mandated pair: the literal fits whole and is padded to the
        // full interior at both.
        for width in [78, 58] {
            let out = lines(source, &progress, width);
            let texts: Vec<String> = out.iter().map(|l| l.text()).collect();
            let want = format!("No tasks yet{}", " ".repeat(width as usize - 12));
            assert!(texts.contains(&want), "width {width}: {texts:?}");
        }
    }

    /// `tasks-progress-bar` :: "The bar measures at most its width at every
    /// width" — a sweep of `0..=130`, including the two `usize::MAX`
    /// `Progress` fixtures the spec adds beyond the existing sweeps.
    #[test]
    fn bar_measures_at_most_its_width_at_every_width() {
        let cases = [
            Progress {
                completed: 4,
                total: 9,
            },
            Progress {
                completed: 0,
                total: 0,
            },
            Progress {
                completed: 0,
                total: usize::MAX,
            },
            Progress {
                completed: usize::MAX,
                total: usize::MAX,
            },
        ];
        for progress in cases {
            for width in 0u16..=130 {
                let bar = progress_bar(&progress, &[], width);
                let cols = columns(&bar);
                assert!(
                    cols <= width as usize,
                    "{progress:?} width {width}: {bar:?} measures {cols}, exceeds its width"
                );
            }
            for width in [78, 58] {
                let bar = progress_bar(&progress, &[], width);
                assert!(columns(&bar) <= width as usize, "width {width}");
            }
        }
    }

    /// `tasks-progress-bar` :: "The full grammar at both mandated interior
    /// widths" — the discriminating clause: the result is byte-identical to
    /// what this requirement produced before display-column measurement.
    /// The expected strings are built independently of `progress_bar` (from
    /// the gauge-fill counts `bar_full_grammar` above already pins: 30/38 at
    /// 78, 21/27 at 58) rather than by calling the function under test, so
    /// this cannot pass by construction.
    #[test]
    fn full_grammar_is_byte_identical_to_pre_change_output() {
        let progress = Progress {
            completed: 4,
            total: 9,
        };
        let want78 = format!("{}{} [4/9] 44%", "█".repeat(30), "░".repeat(38));
        let want58 = format!("{}{} [4/9] 44%", "█".repeat(21), "░".repeat(27));
        assert_eq!(progress_bar(&progress, &[], 78), want78);
        assert_eq!(progress_bar(&progress, &[], 58), want58);
    }

    /// `tasks-progress-bar` :: "The gauge is full exactly when the change is
    /// complete, at the header's width too" — `gauge_of` at `g == 12`, the
    /// detail header's own budget, plus `progress_bar` at the mandated `78`
    /// and `58`. The `usize::MAX` clause is the one that fails against the
    /// shipped saturating arithmetic, which returns a single `█` followed
    /// by `g - 1` `░` at each of `g`'s 12, 48 and 68.
    #[test]
    fn the_gauge_is_full_exactly_when_the_change_is_complete() {
        let cases = [
            Progress {
                completed: 11,
                total: 12,
            },
            Progress {
                completed: 12,
                total: 12,
            },
            Progress {
                completed: 0,
                total: 12,
            },
            Progress {
                completed: 99,
                total: 100,
            },
            Progress {
                completed: 100,
                total: 100,
            },
        ];
        for progress in cases {
            let g12 = gauge_of(&progress, 12);
            assert_eq!(g12.chars().count(), 12, "{progress:?}: {g12:?}");
            assert_eq!(
                !g12.contains('░'),
                progress.is_complete(),
                "{progress:?}: {g12:?}"
            );
            for width in [78, 58] {
                let bar = progress_bar(&progress, &[], width);
                assert_eq!(
                    !bar.contains('░'),
                    progress.is_complete(),
                    "width {width} {progress:?}: {bar:?}"
                );
            }
        }
        let zero_fill = gauge_of(
            &Progress {
                completed: 0,
                total: 12,
            },
            12,
        );
        assert!(!zero_fill.contains('█'), "{zero_fill:?}");

        let saturating = Progress {
            completed: usize::MAX,
            total: usize::MAX,
        };
        for g in [12, 48, 68] {
            let run = gauge_of(&saturating, g);
            assert!(!run.contains('░'), "g {g}: {run:?}");
        }
    }

    /// `tasks-progress-bar` :: "The promoted function is total at both
    /// guard values" — `gauge_of(&Progress { completed: 4, total: 9 }, 0)`
    /// already returns the empty string today, so that half is a
    /// characterization; `gauge_of(&Progress { completed: 0, total: 0 },
    /// 12)` is the behavior change, and `progress_bar`'s own `total == 0`
    /// path (`[-]` at widths `78` and `58`, and below) stays untouched
    /// beside it.
    #[test]
    fn the_promoted_function_is_total_at_both_guard_values() {
        let ordinary = Progress {
            completed: 4,
            total: 9,
        };
        assert_eq!(gauge_of(&ordinary, 0), "");

        let empty = Progress {
            completed: 0,
            total: 0,
        };
        assert_eq!(gauge_of(&empty, 12), "");

        for width in [78, 58] {
            assert_eq!(progress_bar(&empty, &[], width), "[-]", "width {width}");
        }
        assert_eq!(progress_bar(&empty, &[], 3), "[-]");
        assert_eq!(progress_bar(&empty, &[], 2), "");
    }

    /// `tasks-progress-bar` :: "The completeness property holds at the
    /// saturation boundary" — `gauge_of` at `g` of `12`, `48` and `68`, the
    /// detail header's budget and the bar's own two mandated gauges, each
    /// reached again through `progress_bar` at `78` and `58`, so the test
    /// names both interiors and needs no `TASKWIDTHS` exemption.
    #[test]
    fn the_completeness_property_holds_at_the_saturation_boundary() {
        let saturating = Progress {
            completed: usize::MAX,
            total: usize::MAX,
        };
        for g in [12u16, 48, 68] {
            let run = gauge_of(&saturating, g);
            assert_eq!(run.chars().count(), g as usize, "g {g}: {run:?}");
            assert!(!run.contains('░'), "g {g}: {run:?}");
        }
        for width in [78, 58] {
            let bar = progress_bar(&saturating, &[], width);
            assert!(!bar.contains('░'), "width {width}: {bar:?}");
        }

        let zero_fill = gauge_of(
            &Progress {
                completed: 0,
                total: usize::MAX,
            },
            12,
        );
        assert!(!zero_fill.contains('█'), "{zero_fill:?}");
    }

    /// `tasks-progress-bar` :: "A saturating `Progress` renders a full
    /// gauge and a full percentage" — at `Progress { completed: usize::MAX,
    /// total: usize::MAX }` the gauge holds no `░` and the percent cell
    /// reads `100%`, at widths `78` and `58`, where the shipped saturating
    /// arithmetic produced a single `█` and `1%`.
    /// `Progress { completed: 4, total: 9 }` at the same two widths is
    /// byte-identical to before, so the widening moved exactly the
    /// saturating input and nothing else.
    #[test]
    fn a_saturating_progress_renders_a_full_gauge_and_a_full_percentage() {
        let saturating = Progress {
            completed: usize::MAX,
            total: usize::MAX,
        };
        for width in [78, 58] {
            let bar = progress_bar(&saturating, &[], width);
            assert!(!bar.contains('░'), "width {width}: {bar:?}");
            assert!(bar.ends_with("100%"), "width {width}: {bar:?}");
        }

        let unmoved = Progress {
            completed: 4,
            total: 9,
        };
        let want78 = format!("{}{} [4/9] 44%", "█".repeat(30), "░".repeat(38));
        let want58 = format!("{}{} [4/9] 44%", "█".repeat(21), "░".repeat(27));
        assert_eq!(progress_bar(&unmoved, &[], 78), want78);
        assert_eq!(progress_bar(&unmoved, &[], 58), want58);
    }

    /// `tasks-progress-bar` :: "The bar's rendered output does not move" —
    /// `progress_bar` over widths `0..=130` for the five `Progress` values
    /// this scenario names, characterizing that nothing panics and every
    /// result fits its width, plus the literal `78`- and `58`-column
    /// expectations for `{ completed: 4, total: 9 }` — built independently
    /// of `progress_bar`, on the same terms
    /// `full_grammar_is_byte_identical_to_pre_change_output` states, so the
    /// claim is that the bar did not move and not merely that it still
    /// runs. The saturating `Progress` in this sweep is the one excepted
    /// input whose *content* changes; this test pins no literal for it —
    /// `a_saturating_progress_renders_a_full_gauge_and_a_full_percentage`
    /// asserts its new value instead, which is why this test is green both
    /// before and after the widening.
    #[test]
    fn the_bar_s_rendered_output_does_not_move() {
        let cases = [
            Progress {
                completed: 4,
                total: 9,
            },
            Progress {
                completed: 0,
                total: 0,
            },
            Progress {
                completed: 2,
                total: 3,
            },
            Progress {
                completed: 0,
                total: usize::MAX,
            },
            Progress {
                completed: usize::MAX,
                total: usize::MAX,
            },
        ];
        for progress in cases {
            for width in 0u16..=130 {
                let bar = progress_bar(&progress, &[], width);
                assert!(
                    columns(&bar) <= width as usize,
                    "{progress:?} width {width}: {bar:?} exceeds its width"
                );
            }
        }

        let unmoved = Progress {
            completed: 4,
            total: 9,
        };
        let want78 = format!("{}{} [4/9] 44%", "█".repeat(30), "░".repeat(38));
        let want58 = format!("{}{} [4/9] 44%", "█".repeat(21), "░".repeat(27));
        assert_eq!(progress_bar(&unmoved, &[], 78), want78);
        assert_eq!(progress_bar(&unmoved, &[], 58), want58);
    }

    /// `tasks-progress-bar` :: "The header's gauge and the bar's gauge agree
    /// about the same change" — `gauge_of(p, 12)` appears, space-bounded, in
    /// `ui::detail::header_row`'s own output at both mandated widths, for
    /// 4-of-9, a complete change, and an untouched one. The header and the
    /// tracked-tasks tab's own bar share exactly this one `gauge_of` call,
    /// so the two can never disagree about how full a change is.
    #[test]
    fn the_header_s_gauge_and_the_bar_s_gauge_agree() {
        let cases = [
            Progress {
                completed: 4,
                total: 9,
            },
            Progress {
                completed: 7,
                total: 7,
            },
            Progress {
                completed: 0,
                total: 7,
            },
        ];
        for progress in cases {
            let gauge = gauge_of(&progress, 12);
            let needle = format!(" {gauge} ");
            for width in [78, 58] {
                let header =
                    crate::ui::detail::header_row("add-token-refresh", "tdd", &progress, width);
                assert!(
                    header.contains(&needle),
                    "progress {progress:?} width {width}: {header:?} missing {needle:?}"
                );
            }
        }
    }
}
