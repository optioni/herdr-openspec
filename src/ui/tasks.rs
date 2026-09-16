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

use crate::ui::layout::columns;

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
/// [`segmented_gauge`] uses to mark the group boundaries by alternating glyph.
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

/// [`gauge_of`]'s run with each position's **glyph** chosen by which group it
/// falls in: `█`/`⢕` in an even-indexed contributing group and `▒`/`⠌` in an
/// odd-indexed one. A glyph **substitution**, never a second fill computation —
/// each position keeps whether it is filled or empty and changes only which
/// glyph it is drawn with, so `filled == g` iff complete, `filled == 0`
/// whenever `completed == 0`, and the `u128` arithmetic all hold unchanged and
/// by construction. `gauge_of` itself does not move, so `detail-header`'s
/// twelve-column gauge is untouched.
///
/// The filled half and the empty half come from **different character
/// families** — block elements when filled, braille dot patterns when empty —
/// and that, not a difference in lightness, is what marks where the fill ends.
/// Under the previous table the fill boundary was a one-step shade change,
/// exactly the weight of the group boundaries beside it, so every edge in the
/// bar competed equally and the one a reader actually wants at a glance was the
/// hardest to find. A block-to-dots transition cannot be confused with a
/// boundary between two blocks or between two dot patterns, which is why both
/// halves can now carry boundaries without either blurring the fill edge: `▒`
/// is not read as partly filled, because nothing in the empty half is a shade
/// for it to sit on a scale with.
///
/// The two alternations are **in phase** — even-indexed is `█` and `⢕`,
/// odd-indexed `▒` and `⠌` — so a group straddling the fill boundary keeps one
/// identity on both sides of it rather than reading as two groups. That is the
/// common case: exactly one group is usually in progress.
///
/// `░` survives; the second block shade this table used to hold does not. The
/// rule that results: **block shades are the unsegmented vocabulary and braille
/// is the boundary vocabulary**, so braille appears exactly where there are
/// boundaries to mark and nowhere else in the crate (design.md -> Decision 5).
/// The retired glyph is deliberately not named here: `gauge-fill-contrast`'s
/// own check that it is gone from the crate is a tree-wide grep for it, which a
/// mention in this comment would defeat.
///
/// A **contributing group** is one whose `total` is greater than zero. An empty
/// group contributes no stretch and **consumes no index**, so two groups left
/// adjacent after empty ones are dropped still alternate — an empty group
/// taking an index would give two neighbours the same glyph and erase the
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
/// one-column stretch cannot be read as a glyph run, so a gauge that cannot give
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
            ('█', true) => '▒',
            ('░', false) => '⢕',
            ('░', true) => '⠌',
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
/// remainder of that leading segment's text — each omitted rather than
/// emitted empty, so an item whose text is exactly its label produces one
/// fewer segment than one with a remainder after it — followed by `row`'s
/// own later segments untouched.
///
/// `label` is looked up against `item.text`, never against the rendered
/// row: `tasks::label_of` returns byte offsets into the item's **plain**
/// text, which no longer address the row once inline spans fold. The label
/// is therefore applied only when `row`'s leading segment is
/// `Face::plain()` and long enough to hold `start + len` on character
/// boundaries — otherwise `None`, and the item renders unlabelled, carrying
/// the faces `ui::markdown::inline` returned (design.md -> Decision 6). An
/// emphasised label (`- [ ] **RED**: …`) is the reachable case: its leading
/// segment is `strong`-faced, so it degrades to unlabelled — a miss, never
/// a wrong colour.
fn labelled_line(
    prefix: &str,
    row: &crate::ui::markdown::Line,
    label: crate::tasks::Label,
) -> Option<crate::ui::markdown::Line> {
    let first = row.segments.first()?;
    if first.face != crate::ui::markdown::Face::plain() {
        return None;
    }
    let text = &first.text;
    let end = label.start.checked_add(label.len)?;
    if end > text.len() || !text.is_char_boundary(label.start) || !text.is_char_boundary(end) {
        return None;
    }
    let plain = crate::ui::markdown::Face::plain();
    let faced = crate::ui::markdown::Face {
        label: Some(label.role),
        ..plain
    };
    let mut segments = Vec::with_capacity(row.segments.len() + 2);
    for (t, face) in [
        (format!("{prefix}{}", &text[..label.start]), plain),
        (text[label.start..end].to_string(), faced),
        (text[end..].to_string(), plain),
    ] {
        if !t.is_empty() {
            segments.push(crate::ui::markdown::Segment { text: t, face });
        }
    }
    segments.extend(row.segments[1..].iter().cloned());
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

/// Prepend `prefix` to `line`, merging it into the leading segment — under
/// whatever face that segment already carries — when there is one, and
/// inserting a fresh `Face::plain()` segment carrying `prefix` alone
/// otherwise. Used at a hanging indent, where the prefix is bare whitespace
/// and must read as a literal continuation of whatever the row's own first
/// character is faced as: a fenced block's body row stays wholly
/// `face.code`, hang included (`tasks-checklist`'s own "every segment of
/// that row carries `face.code`" scenario), rather than splitting into a
/// plain indent segment beside a code one.
fn hang_line(prefix: &str, mut line: crate::ui::markdown::Line) -> crate::ui::markdown::Line {
    if prefix.is_empty() {
        return line;
    }
    if let Some(first) = line.segments.first_mut() {
        first.text = format!("{prefix}{}", first.text);
        return line;
    }
    crate::ui::markdown::Line {
        segments: vec![crate::ui::markdown::Segment {
            text: prefix.to_string(),
            face: crate::ui::markdown::Face::plain(),
        }],
    }
}

/// One `tasks::Item`'s rendered line(s): its own text rows, faced through
/// `ui::markdown::inline` and labelled at most once, followed by its
/// body's rows, faced through `ui::markdown::lines` — both at a hanging
/// indent of `prefix_len + tasks::task_number_len(&item.text)`
/// (design.md -> Decision 9a). The indent is dropped whole when the full
/// prefix would leave no text column, then the separating space and
/// glyph-only prefix is tried, and when even that leaves no text column,
/// the glyph alone — truncated by `ui::list::pad_or_truncate_right` at
/// `width` — is the whole line, with the item's text and its body alike
/// discarded rather than wrapped into zero columns (design.md ->
/// Decision 9).
fn item_lines(item: &crate::tasks::Item, width: u16) -> Vec<crate::ui::markdown::Line> {
    let w = width as usize;
    // The same three-column glyph `ui::markdown` renders for a task-list
    // item, so the two checkbox renderers agree by construction and a future
    // divergence is a failing test rather than a silent inconsistency
    // (design.md -> Decision 6). `✓` (U+2713) is East Asian Neutral and one
    // column, so the prefix is three columns exactly as `[x]` was and no
    // wrap or indent arithmetic moves.
    let glyph = if item.checked { "[✓]" } else { "[ ]" };

    // The third element is whether the prefix survived *whole*. A prefix
    // that has degraded to the glyph alone has no column left to hang a
    // body from, so the body is dropped with it rather than wrapped into
    // one or two columns (design.md -> Decision 9); at width 5 the body
    // was measured rendering fourteen one-character rows, which is the
    // outcome drop-whole exists to avoid.
    let full_prefix_len = item.indent + 4;
    let prefix = if full_prefix_len < w {
        Some((
            format!("{}{glyph} ", " ".repeat(item.indent)),
            full_prefix_len,
            true,
        ))
    } else if 4 < w {
        Some((format!("{glyph} "), 4, false))
    } else {
        None
    };

    let Some((prefix, prefix_len, prefix_whole)) = prefix else {
        let degraded = crate::ui::list::pad_or_truncate_right(glyph, w);
        return vec![if item.checked {
            muted_line(degraded)
        } else {
            plain_line(degraded)
        }];
    };

    // The number hang is dropped whole, back to the prefix alone, before
    // the prefix's own degradation applies (design.md -> Decision 9a): a
    // width that can hold the glyph and some text never loses the text to
    // the number's own indent.
    let number_len = crate::tasks::task_number_len(&item.text);
    let hang = if prefix_len + number_len < w {
        prefix_len + number_len
    } else {
        prefix_len
    };
    let col = (w - hang) as u16;
    let hang_spaces = " ".repeat(hang);

    // A fragment, not a document: `item.text` is one sentence, so it goes
    // through `inline` rather than `lines` (design.md -> Decision 3).
    let rows = crate::ui::markdown::inline(&item.text, col);

    // An unchecked item's label, when it has one. `label_of` reads
    // `item.text`; `labelled_line` re-checks its offsets against the
    // rendered row's own leading segment, so a label whose plain run does
    // not survive facing degrades to unlabelled instead (design.md ->
    // Decision 6).
    let label = if item.checked {
        None
    } else {
        crate::tasks::label_of(&item.text)
    };

    let mut out = Vec::with_capacity(rows.len());
    for (i, line) in rows.into_iter().enumerate() {
        if item.checked {
            // Every row of a checked item, its first and its continuations
            // alike: one de-emphasised segment, never split at its label or
            // at any inline face (design.md -> Decision 7).
            let text = if i == 0 {
                format!("{prefix}{}", line.text())
            } else {
                format!("{hang_spaces}{}", line.text())
            };
            out.push(muted_line(text));
            continue;
        }
        if i == 0 {
            // A label appears once, on the row it was written on.
            if let Some(label) = label
                && let Some(labelled) = labelled_line(&prefix, &line, label)
            {
                out.push(labelled);
            } else {
                out.push(hang_line(&prefix, line));
            }
        } else {
            out.push(hang_line(&hang_spaces, line));
        }
    }

    // The item's body, at the same hanging indent as its own continuation
    // rows, immediately after its own rows and before the next item or
    // block. An empty body contributes no row (design.md -> Decision 9).
    if prefix_whole && !item.body.is_empty() {
        for line in crate::ui::markdown::lines(&item.body, col) {
            if item.checked {
                out.push(muted_line(format!("{hang_spaces}{}", line.text())));
            } else {
                out.push(hang_line(&hang_spaces, line));
            }
        }
    }

    out
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

/// Every block `group` records at position `after`, each drawn through
/// `ui::markdown::lines` at the group's own full `width` — no hanging
/// indent, a block belonging to the group rather than to the item above it
/// — and separated from the rows around it by one blank row either side
/// (design.md -> Decision 10). A group carrying no block at that position
/// contributes nothing, which is what keeps a blockless group's rows
/// byte-identical to the group it was before blocks existed.
fn push_blocks(
    out: &mut Vec<crate::ui::markdown::Line>,
    group: &crate::tasks::Group,
    after: usize,
    width: u16,
) {
    for block in group.blocks.iter().filter(|b| b.after == after) {
        out.push(blank_line());
        out.extend(crate::ui::markdown::lines(&block.text, width));
        out.push(blank_line());
    }
}

/// Every row `group` contributes below its own heading, in document order:
/// its items, each item's own body rows, and its blocks interleaved at the
/// positions `task-groups` records for them — no progress bar, no heading
/// line, no blank separator between groups.
///
/// Takes a **parsed group** rather than a source string: [`lines`] already
/// holds `tasks::Group` values and would have to re-serialise each one to
/// call a string-taking form, which is the duplication this extraction
/// exists to remove. A folded tab reaches it through
/// `tasks::parse(&section.text)` on a section body that carries no heading
/// of its own, that heading having become the fold header.
///
/// This function was named `items` and took `&[crate::tasks::Item]`. It is
/// renamed because its subject changed: a group's rows are no longer only
/// its items, and a function called `items` that also draws fenced blocks
/// would be a name that lies (design.md -> Decision 5).
pub(crate) fn group_body(
    group: &crate::tasks::Group,
    width: u16,
) -> Vec<crate::ui::markdown::Line> {
    if width == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    push_blocks(&mut out, group, 0, width);
    for (i, item) in group.items.iter().enumerate() {
        out.extend(item_lines(item, width));
        push_blocks(&mut out, group, i + 1, width);
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
        // A document with zero items may still hold retained prose — a
        // leading group carrying a block and no items, per `task-groups`'
        // Decision 1a — and that prose is drawn beneath the row rather than
        // discarded with the items it does not have. `group_body` with an
        // empty `items` slice draws exactly that group's own blocks.
        for group in &tasks.groups {
            out.extend(group_body(group, width));
        }
        return out;
    }

    let last_index = tasks.groups.len().saturating_sub(1);
    for (index, group) in tasks.groups.iter().enumerate() {
        if let Some(heading) = &group.heading {
            out.push(heading_line(heading, width));
        }
        out.extend(group_body(group, width));
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
        let group = &parsed.groups[0];
        for width in [78, 58] {
            let out = super::group_body(group, width);
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
            let out = super::group_body(&checked.groups[0], width);
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
            let twin = super::group_body(&unchecked.groups[0], width);
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
        let group = &parsed.groups[0];
        let items = &group.items;

        // Measured against `group_body`: the hang is now `prefix_len +
        // task_number_len` (eight columns here, "1.1 " being the number),
        // not `prefix_len` alone, so both the continuation indent and the
        // wrap column moved from `task-labels`' own recorded literals.
        let recorded: [(u16, &[&str]); 2] = [
            (
                78,
                &[
                    "[ ] 1.1 GREEN: abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh",
                    "        abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh",
                    "        abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh",
                ],
            ),
            (
                58,
                &[
                    "[ ] 1.1 GREEN: abcdefgh abcdefgh abcdefgh abcdefgh",
                    "        abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh",
                    "        abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh",
                    "        abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh",
                    "        abcdefgh",
                ],
            ),
        ];

        for (width, expected) in recorded {
            let out = super::group_body(group, width);
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
            super::group_body(group, 58).len() > super::group_body(group, 78).len(),
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
        let group = &parsed.groups[0];

        // Measured against `group_body`: the hang is eight columns here too
        // (the item carries the same `1.1 ` number), so the wrap column —
        // `width - 8`, not `width - 4` — and the continuation indent both
        // moved from `task-labels`' own recorded literals; the number now
        // sometimes fills the whole of row 0 on its own, at 16.
        let recorded: [(u16, &[&str]); 3] = [
            (
                16,
                &[
                    "[ ] 1.1",
                    "        CHARACTE",
                    "        RIZE:",
                    "        record",
                    "        the",
                    "        baseline",
                ],
            ),
            (
                14,
                &[
                    "[ ] 1.1",
                    "        CHARAC",
                    "        TERIZE",
                    "        :",
                    "        record",
                    "        the",
                    "        baseli",
                    "        ne",
                ],
            ),
            (
                12,
                &[
                    "[ ] 1.1",
                    "        CHAR",
                    "        ACTE",
                    "        RIZE",
                    "        :",
                    "        reco",
                    "        rd",
                    "        the",
                    "        base",
                    "        line",
                ],
            ),
        ];
        for (width, expected) in recorded {
            let out = super::group_body(group, width);
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
            let out = super::group_body(group, width);
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

    /// The five glyphs the gauge can draw — the four of the segmentation
    /// table plus the `░` the unsegmented run keeps — and the counts every
    /// test below reads a run through.
    fn gauge_run(bar: &str) -> String {
        bar.chars().take_while(|c| "█░▒⢕⠌".contains(*c)).collect()
    }

    fn filled_count(bar: &str) -> usize {
        gauge_run(bar)
            .chars()
            .filter(|&c| c == '█' || c == '▒')
            .count()
    }

    /// The two character families the fill boundary separates: blocks when
    /// filled, braille dot patterns when empty.
    fn is_block(c: char) -> bool {
        c == '█' || c == '▒'
    }

    fn is_braille(c: char) -> bool {
        c == '⢕' || c == '⠌'
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
            let even = run.chars().filter(|&c| c == '█' || c == '⢕').count();
            let odd = run.chars().filter(|&c| c == '▒' || c == '⠌').count();
            assert_eq!(even + odd, g, "width {width}: every position is drawn");
            assert_eq!(even, g * 9 / 12, "width {width}: the first span");
            assert_eq!(odd, g - g * 9 / 12, "width {width}: the second span");
            // The first `floor(g * 9 / 12)` positions are the `█`/`⢕` pair and
            // the remainder the `▒`/`⠌` pair, in that order.
            assert!(
                run.chars().take(even).all(|c| c == '█' || c == '⢕'),
                "width {width}: {run:?}"
            );
            assert!(
                run.chars().skip(even).all(|c| c == '▒' || c == '⠌'),
                "width {width}: {run:?}"
            );

            // The fill is unchanged: the same count the unsegmented gauge
            // produces.
            assert_eq!(filled_count(&bar), g * 3 / 12, "width {width}");

            // And the empty-slice call holds only `█` and `░`, with the same
            // filled count and no braille anywhere. Mapping the segmented line
            // back through the inverse substitution reproduces it exactly,
            // which is what pins that segmentation moved neither cell nor the
            // gauge's length — not only its glyphs.
            let plain = progress_bar(&progress, &[], width);
            assert_eq!(
                bar.replace('▒', "█").replace(['⢕', '⠌'], "░"),
                plain,
                "width {width}: segmentation moved something other than a glyph"
            );
            let plain_run = gauge_run(&plain);
            assert!(
                plain_run.chars().all(|c| c == '█' || c == '░'),
                "width {width}: {plain_run:?}"
            );
            assert!(
                !plain_run.chars().any(is_braille),
                "width {width}: the unsegmented run drew braille"
            );
            assert_eq!(
                filled_count(&plain),
                g * 3 / 12,
                "width {width}: the unsegmented fill count"
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
            let first = run.chars().filter(|&c| c == '⢕').count();
            let second = run.chars().filter(|&c| c == '⠌').count();
            assert_eq!(first + second, g, "width {width}: every position is drawn");
            assert_eq!(first, g / 2, "width {width}: the first half");
            assert!(
                run.chars().take(first).all(|c| c == '⢕'),
                "width {width}: {run:?}"
            );
            assert!(
                run.chars().skip(first).all(|c| c == '⠌'),
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
            let marked = run.chars().any(|c| c == '▒' || is_braille(c));
            if g < 44 {
                below += 1;
                assert!(!marked, "width {width}: g {g} is below the floor: {run:?}");
                assert!(
                    run.chars().all(|c| c == '█' || c == '░'),
                    "width {width}: g {g} is below the floor: {run:?}"
                );
            } else {
                above += 1;
                assert!(
                    run.chars().any(|c| c == '█' || c == '⢕')
                        && run.chars().any(|c| c == '▒' || c == '⠌'),
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
                run.chars().any(|c| c == '▒' || is_braille(c)),
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
                !bar.contains('▒') && !bar.chars().any(is_braille),
                "width {width}: one group has no boundary to mark: {bar:?}"
            );
            assert!(
                gauge_run(&bar).chars().all(|c| c == '█' || c == '░'),
                "width {width}: {bar:?}"
            );
            assert_eq!(
                bar,
                progress_bar(&progress, &[], width),
                "width {width}: byte-identical to the empty-slice call"
            );
        }
    }

    /// `tasks-progress-bar` :: "The fill boundary is the only change of
    /// character family".
    ///
    /// The fixture is this repository's own `mouse-text-selection` at the
    /// moment this change was written — eleven groups, 25 of 47 items — chosen
    /// because its straddling group's index is **odd**, which under the
    /// previous table put the fill boundary at the weakest edge in the bar.
    #[test]
    fn the_fill_boundary_is_the_only_change_of_character_family() {
        let progress = p(25, 47);
        let groups = [
            p(4, 4),
            p(4, 4),
            p(3, 3),
            p(5, 5),
            p(8, 8),
            p(1, 3),
            p(0, 4),
            p(0, 3),
            p(0, 4),
            p(0, 3),
            p(0, 6),
        ];
        // The widths are written unsuffixed: `TASKWIDTHS` scans `\b(\d+)\b`
        // and does not see `78u16`.
        for (width, expected_g) in [(78, 66), (58, 46)] {
            let bar = progress_bar(&progress, &groups, width);
            let run = gauge_run(&bar);
            let chars: Vec<char> = run.chars().collect();
            let g = chars.len();
            assert_eq!(g, expected_g, "width {width}: the gauge run's width");

            // Exactly one block-followed-by-braille adjacency, and none the
            // other way round.
            let forward = chars
                .windows(2)
                .filter(|w| is_block(w[0]) && is_braille(w[1]))
                .count();
            let backward = chars
                .windows(2)
                .filter(|w| is_braille(w[0]) && is_block(w[1]))
                .count();
            assert_eq!(forward, 1, "width {width}: {run:?}");
            assert_eq!(backward, 0, "width {width}: {run:?}");

            // The **first braille position** is `floor(g * 25 / 47)`, so the
            // adjacency above sits one before it. This assertion reads the
            // first braille position, which is the one of the two the spec
            // names as the index.
            let first_braille = chars.iter().position(|&c| is_braille(c)).unwrap();
            assert_eq!(first_braille, g * 25 / 47, "width {width}: {run:?}");
            assert_eq!(
                chars
                    .windows(2)
                    .position(|w| is_block(w[0]) && is_braille(w[1]))
                    .unwrap(),
                g * 25 / 47 - 1,
                "width {width}: the adjacency sits one before it"
            );

            // And it is the same index at which `█` becomes `░` unsegmented.
            let plain = gauge_run(&progress_bar(&progress, &[], width));
            assert_eq!(
                plain.chars().position(|c| c == '░').unwrap(),
                first_braille,
                "width {width}: the fill boundary moved"
            );

            // All four glyphs are actually reached.
            for glyph in ['█', '▒', '⢕', '⠌'] {
                assert!(
                    chars.contains(&glyph),
                    "width {width}: {glyph:?} never drawn: {run:?}"
                );
            }
        }
    }

    /// `tasks-progress-bar` :: "A group straddling the fill boundary keeps one
    /// identity".
    #[test]
    fn a_group_straddling_the_fill_boundary_keeps_one_identity() {
        let progress = p(1, 4);
        let groups = [p(1, 2), p(0, 2)];
        for width in [78, 58] {
            let bar = progress_bar(&progress, &groups, width);
            let run = gauge_run(&bar);
            let chars: Vec<char> = run.chars().collect();
            let g = chars.len();
            // Group 0 owns `[0, floor(g/2))`; group 1 the remainder.
            let split = g / 2;
            let (first, second) = chars.split_at(split);

            // Group 0 straddles the fill boundary at `floor(g/4)`: `█` before
            // it and `⢕` after, both the even-indexed pair. The straddle guard
            // runs **first**, so a regression that moved the fill past the
            // group boundary reports that rather than panicking on the slice.
            let fill = g / 4;
            assert!(
                fill > 0 && fill < split,
                "width {width}: g {g} must straddle"
            );
            assert!(
                first[..fill].iter().all(|&c| c == '█'),
                "width {width}: {run:?}"
            );
            assert!(
                first[fill..].iter().all(|&c| c == '⢕'),
                "width {width}: {run:?}"
            );

            // Group 1 is wholly empty and wholly odd.
            assert!(second.iter().all(|&c| c == '⠌'), "width {width}: {run:?}");

            // No position of group 0 carries an odd-indexed glyph, so the two
            // alternations are in phase and it does not read as two groups.
            assert!(
                !first.iter().any(|&c| c == '▒' || c == '⠌'),
                "width {width}: group 0 changed identity at the fill boundary: {run:?}"
            );
        }
    }

    /// `tasks-progress-bar` :: "Every glyph the bar can draw measures one
    /// column".
    ///
    /// The glyph-presence clause is what makes this red before the
    /// substitution lands: `layout::columns` measures a `char` literal whether
    /// or not the crate ever draws it, so the width assertions alone would
    /// pass against any build.
    #[test]
    fn every_glyph_the_bar_can_draw_measures_one_column() {
        for glyph in ['█', '░', '▒', '⢕', '⠌'] {
            assert_eq!(columns(&glyph.to_string()), 1, "{glyph:?}");
        }

        let progress = p(25, 47);
        let groups = [
            p(4, 4),
            p(4, 4),
            p(3, 3),
            p(5, 5),
            p(8, 8),
            p(1, 3),
            p(0, 20),
        ];
        for width in [78, 58] {
            let bar = progress_bar(&progress, &groups, width);
            let run = gauge_run(&bar);
            assert_eq!(
                columns(&run),
                run.chars().count(),
                "width {width}: a gauge glyph is not one column: {run:?}"
            );
            assert_eq!(columns(&bar), width as usize, "width {width}: {bar:?}");

            // The segmented run reaches all four segmentation glyphs, and the
            // empty-slice call reaches `░`, so all five are measured rather
            // than the scenario passing on a run holding only two of them.
            for glyph in ['█', '▒', '⢕', '⠌'] {
                assert!(
                    run.contains(glyph),
                    "width {width}: {glyph:?} never drawn: {run:?}"
                );
            }
            assert!(
                gauge_run(&progress_bar(&progress, &[], width)).contains('░'),
                "width {width}: the unsegmented run drew no `░`"
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
                // Five glyphs rather than the segmentation table's four: this
                // sweep deliberately reaches widths and slices where
                // segmentation is **skipped**, and there the run is the `█`/`░`
                // pair `gauge_of` returns, so a sum over the table alone is
                // short by every `░` on that path.
                let counts = ['█', '░', '▒', '⢕', '⠌']
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
                // The two families never interleave: every braille position
                // lies at or after every block one, so the fill boundary is a
                // single transition rather than a scatter. (No "and no
                // position is both" clause: `is_block` and `is_braille` are
                // disjoint by construction, so asserting it could not fail,
                // and this repository does not keep tests that cannot.)
                let chars: Vec<char> = run.chars().collect();
                let last_block = chars.iter().rposition(|&c| is_block(c));
                let first_braille = chars.iter().position(|&c| is_braille(c));
                if let (Some(last_block), Some(first_braille)) = (last_block, first_braille) {
                    assert!(
                        first_braille > last_block,
                        "case {index} width {width}: the families interleave: {run:?}"
                    );
                }
            }
        }

        // The saturating input is not reintroduced by segmentation: every
        // position is a filled glyph and the percent cell reads 100%.
        let groups = [p(half, half), p(half, half)];
        for width in [78, 58] {
            let bar = progress_bar(&p(usize::MAX, usize::MAX), &groups, width);
            let run = gauge_run(&bar);
            assert!(run.chars().all(is_block), "width {width}: {run:?}");
            assert!(bar.ends_with("100%"), "width {width}: {bar:?}");
        }
    }

    /// `tasks-checklist` :: "A folded group and an unfolded one render the
    /// same item lines". Every expectation here is a **literal**: asserting
    /// that `group_body`'s output equals a slice of `lines`' output could not
    /// fail once `lines` calls `group_body`, and this repository does not
    /// keep tests that cannot fail.
    #[test]
    fn a_folded_group_and_an_unfolded_one_render_the_same_item_lines() {
        let progress = Progress {
            completed: 1,
            total: 2,
        };
        let parsed = crate::tasks::parse("- [x] 1.1 first\n- [ ] 1.2 second\n");
        let group = &parsed.groups[0];

        for width in [78, 58] {
            // `group_body` alone: the item rows and nothing else.
            let out = super::group_body(group, width);
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

    // --- group 4: item bodies and blocks -------------------------------

    /// `tasks-checklist` :: "An item's body is drawn under it at its
    /// hanging indent".
    #[test]
    fn an_items_body_is_drawn_under_it_at_its_hanging_indent() {
        let parsed = crate::tasks::parse(
            "- [ ] 2.2 GREEN: add the method\n      writing the OSC 52 sequence, and\n      the arm.\n",
        );
        let group = &parsed.groups[0];
        assert_eq!(
            group.items[0].body,
            "writing the OSC 52 sequence, and\nthe arm."
        );

        let mut body_row_counts = std::collections::HashMap::new();
        for width in [78, 58] {
            let out = super::group_body(group, width);
            assert!(
                out[0].text().starts_with("[ ] 2.2 "),
                "width {width}: {:?}",
                out[0].text()
            );
            let label_seg = out[0]
                .segments
                .iter()
                .find(|s| s.text == "GREEN:")
                .unwrap_or_else(|| panic!("width {width}: no GREEN: segment in {:?}", out[0]));
            assert_eq!(
                label_seg.face,
                crate::ui::markdown::Face {
                    label: Some(crate::tasks::LabelRole::Change),
                    ..crate::ui::markdown::Face::plain()
                },
                "width {width}"
            );

            let body_rows = &out[1..];
            assert!(!body_rows.is_empty(), "width {width}: the body must render");
            let mut reflowed = String::new();
            for row in body_rows {
                let text = row.text();
                assert!(
                    text.starts_with("        "),
                    "width {width}: {text:?} does not hang at eight spaces"
                );
                if !reflowed.is_empty() {
                    reflowed.push(' ');
                }
                reflowed.push_str(text.trim_start());
            }
            assert_eq!(
                reflowed, "writing the OSC 52 sequence, and the arm.",
                "width {width}"
            );
            body_row_counts.insert(width, body_rows.len());
        }
        // Measured, not predicted: at `width - 8` the body's forty-one
        // reflowed characters fit one row at both 58 (a 50-column body) and
        // 78 (a 70-column one), so this exact fixture does not itself reach
        // a width where 58 wraps more than 78. A longer body, over the same
        // item, is what demonstrates the body genuinely reflows rather than
        // being reproduced line for line.
        assert_eq!(body_row_counts[&58], 1, "{body_row_counts:?}");
        assert_eq!(body_row_counts[&78], 1, "{body_row_counts:?}");

        let long_body = long_paragraph(200);
        let long_group = crate::tasks::Group {
            heading: None,
            items: vec![crate::tasks::Item {
                checked: false,
                text: "2.2 GREEN: add the method".to_string(),
                indent: 0,
                body: long_body,
            }],
            blocks: Vec::new(),
        };
        let mut long_row_counts = std::collections::HashMap::new();
        for width in [78, 58] {
            let out = super::group_body(&long_group, width);
            long_row_counts.insert(width, out.len() - 1);
        }
        assert!(
            long_row_counts[&58] > long_row_counts[&78],
            "{long_row_counts:?}: 58 must produce strictly more body rows"
        );
    }

    /// `tasks-checklist` :: "The hanging indent falls after the task
    /// number".
    #[test]
    fn the_hanging_indent_falls_after_the_task_number() {
        let words = long_paragraph(200);
        let item_of = |text: String| crate::tasks::Item {
            checked: false,
            text,
            indent: 0,
            body: String::new(),
        };
        let group_of = |item: crate::tasks::Item| crate::tasks::Group {
            heading: None,
            items: vec![item],
            blocks: Vec::new(),
        };

        for width in [78, 58] {
            for (text, hang) in [
                (format!("1.1 {words}"), 8usize),
                (format!("10.11a {words}"), 11usize),
                (words.clone(), 4usize),
            ] {
                let number: Option<String> = if hang > 4 {
                    Some(text.split(' ').next().unwrap().to_string())
                } else {
                    None
                };
                let group = group_of(item_of(text));
                let out = super::group_body(&group, width);
                assert!(
                    out.len() > 1,
                    "width {width} hang {hang}: the item must wrap"
                );

                for (i, line) in out.iter().enumerate().skip(1) {
                    let text = line.text();
                    let indent: String = text.chars().take_while(|&c| c == ' ').collect();
                    assert_eq!(
                        indent.len(),
                        hang,
                        "width {width} hang {hang} row {i}: {text:?}"
                    );
                }

                // The text column is continuous: the first row's character
                // at column `hang` is the character the continuation rows
                // begin with.
                let first = out[0].text();
                let second = out[1].text();
                assert_eq!(
                    first.chars().nth(hang),
                    second.chars().nth(hang),
                    "width {width} hang {hang}: {first:?} vs {second:?}"
                );

                // The number appears only on the item's first row.
                if let Some(number) = &number {
                    for line in &out[1..] {
                        assert!(
                            !line.text().trim_start().starts_with(number.as_str()),
                            "width {width} hang {hang}: {:?}",
                            line.text()
                        );
                    }
                }
            }
        }
    }

    /// `tasks-checklist` :: "The number hang is dropped before the prefix
    /// is".
    #[test]
    fn the_number_hang_is_dropped_before_the_prefix_is() {
        let words = long_paragraph(40);
        let item = crate::tasks::Item {
            checked: false,
            text: format!("1.1 {words}"),
            indent: 0,
            body: String::new(),
        };
        let group = crate::tasks::Group {
            heading: None,
            items: vec![item],
            blocks: Vec::new(),
        };

        for width in [78, 58, 20, 12, 10, 8, 6, 5, 4, 0] {
            let out = super::group_body(&group, width);
            for line in &out {
                assert!(
                    columns(&line.text()) <= width as usize,
                    "width {width}: {:?} exceeds its width",
                    line.text()
                );
            }
            if width == 0 {
                assert!(out.is_empty(), "width {width}");
                continue;
            }
            let w = width as usize;
            if 8 < w {
                // `[ ] 1.1 ` leaves a text column: the hang is the prefix
                // plus the number.
                for line in &out[1..] {
                    let indent: String = line.text().chars().take_while(|&c| c == ' ').collect();
                    assert_eq!(indent.len(), 8, "width {width}: {:?}", line.text());
                }
            } else if 4 < w {
                // The number hang was dropped whole; the prefix alone
                // still leaves a text column and the text is still
                // rendered.
                assert!(
                    out.len() > 1,
                    "width {width}: the item's text must still render"
                );
                for line in &out[1..] {
                    let indent: String = line.text().chars().take_while(|&c| c == ' ').collect();
                    assert_eq!(indent.len(), 4, "width {width}: {:?}", line.text());
                }
            }
        }
    }

    /// `tasks-checklist` :: "A fenced block in an item's body renders as
    /// code, not as vanished text".
    #[test]
    fn a_fenced_block_in_an_items_body_renders_as_code_not_as_vanished_text() {
        let parsed =
            crate::tasks::parse("- [ ] run the gate\n      ```\n      make check\n      ```\n");
        let group = &parsed.groups[0];
        assert_eq!(group.items[0].body, "```\nmake check\n```");

        for width in [78, 58] {
            let out = super::group_body(group, width);
            let code_row = out
                .iter()
                .find(|l| l.text().trim_start() == "make check")
                .unwrap_or_else(|| panic!("width {width}: no `make check` row in {out:?}"));
            assert!(
                code_row.segments.iter().all(|s| s.face.code),
                "width {width}: {code_row:?}"
            );
            assert!(!code_row.text().contains('`'), "width {width}");

            // The item carries no task number ("run the gate"), so its hang
            // is the prefix alone — four columns. The body, handed directly
            // to `ui::markdown::lines` at `width - 4`, produces the same row
            // texts with the hang stripped, so the body path and the
            // ordinary markdown path cannot drift.
            let direct = crate::ui::markdown::lines(&group.items[0].body, width - 4);
            let direct_texts: Vec<String> = direct.iter().map(|l| l.text()).collect();
            let via_item: Vec<String> = out[1..]
                .iter()
                .map(|l| l.text().trim_start_matches(' ').to_string())
                .collect();
            assert_eq!(via_item, direct_texts, "width {width}");
        }
    }

    /// `tasks-checklist` :: "A group's block renders between the items it
    /// sits between".
    #[test]
    fn a_groups_block_renders_between_the_items_it_sits_between() {
        let with_block = crate::tasks::parse("- [ ] a\n```\nmake check\n```\n- [ ] b\n");
        let group = &with_block.groups[0];
        assert_eq!(group.blocks.len(), 1, "one block between the two items");
        assert_eq!(group.blocks[0].after, 1, "after the first item");

        let without_block = crate::tasks::parse("- [ ] a\n- [ ] b\n");
        let plain_group = &without_block.groups[0];
        assert!(plain_group.blocks.is_empty());

        for width in [78, 58] {
            let out = super::group_body(group, width);
            let texts: Vec<String> = out.iter().map(|l| l.text()).collect();
            assert_eq!(texts[0], "[ ] a", "width {width}");
            assert_eq!(texts.last().unwrap(), "[ ] b", "width {width}");
            assert_eq!(texts[1], "", "width {width}: a blank row opens the block");
            let code_idx = texts
                .iter()
                .position(|t| t.trim_start() == "make check")
                .unwrap_or_else(|| panic!("width {width}: no `make check` row: {texts:?}"));
            assert_eq!(
                texts[code_idx], "make check",
                "width {width}: a block carries no hanging indent"
            );
            assert!(
                out[code_idx].segments.iter().all(|s| s.face.code),
                "width {width}: {:?}",
                out[code_idx]
            );
            // A blank row closes the block, immediately before the next
            // item — the block's own rendering may itself trail a further
            // blank, a `ui::markdown::lines` property of the fenced
            // content, not something this grammar adds.
            assert_eq!(
                texts[texts.len() - 2],
                "",
                "width {width}: a blank row precedes the next item"
            );

            let plain = super::group_body(plain_group, width);
            let plain_texts: Vec<String> = plain.iter().map(|l| l.text()).collect();
            assert_eq!(
                plain_texts,
                vec!["[ ] a".to_string(), "[ ] b".to_string()],
                "width {width}: no block, no blank row"
            );
        }
    }

    /// `tasks-checklist` :: "An item's inline markdown is faced rather than
    /// shown as markers".
    #[test]
    fn an_items_inline_markdown_is_faced_rather_than_shown_as_markers() {
        let parsed =
            crate::tasks::parse("- [ ] 1.1 RED: add the `Recorder` arm and **assert** it\n");
        let group = &parsed.groups[0];
        for width in [78, 58] {
            let out = super::group_body(group, width);
            assert_eq!(out.len(), 1, "width {width}: fits on one row");
            let row = &out[0];
            let code_seg = row
                .segments
                .iter()
                .find(|s| s.text == "Recorder")
                .unwrap_or_else(|| panic!("width {width}: no Recorder segment: {row:?}"));
            assert!(code_seg.face.code, "width {width}");
            let strong_seg = row
                .segments
                .iter()
                .find(|s| s.text == "assert")
                .unwrap_or_else(|| panic!("width {width}: no assert segment: {row:?}"));
            assert!(strong_seg.face.strong, "width {width}");
            assert!(!row.text().contains('`'), "width {width}: {:?}", row.text());
            assert!(!row.text().contains('*'), "width {width}: {:?}", row.text());

            let label_seg = row
                .segments
                .iter()
                .find(|s| s.text == "RED:")
                .unwrap_or_else(|| panic!("width {width}: no RED: segment: {row:?}"));
            assert_eq!(
                label_seg.face,
                crate::ui::markdown::Face {
                    label: Some(crate::tasks::LabelRole::Evidence),
                    ..crate::ui::markdown::Face::plain()
                },
                "width {width}"
            );
        }
    }

    /// `tasks-checklist` :: "An emphasised label degrades to unlabelled
    /// rather than mis-coloured".
    #[test]
    fn an_emphasised_label_degrades_to_unlabelled_rather_than_mis_coloured() {
        let emphasised = crate::tasks::parse("- [ ] **RED**: write the failing test\n");
        let plain = crate::tasks::parse("- [ ] RED: write the failing test\n");
        for width in [78, 58] {
            let out = super::group_body(&emphasised.groups[0], width);
            assert_eq!(out.len(), 1, "width {width}: fits on one row");
            assert!(
                out[0].segments.iter().all(|s| s.face.label.is_none()),
                "width {width}: {:?}",
                out[0]
            );
            // The prefix ("[ ] ") merges into this leading segment rather
            // than starting a plain segment of its own, since it is not the
            // leading *plain* segment the label rule looks for
            // (design.md -> Decision 6) — so its text is `"[ ] RED"`, not
            // `"RED"` alone.
            let bold_seg = out[0]
                .segments
                .iter()
                .find(|s| s.text.ends_with("RED"))
                .unwrap_or_else(|| panic!("width {width}: no RED segment: {:?}", out[0]));
            assert!(bold_seg.face.strong, "width {width}");

            let out_plain = super::group_body(&plain.groups[0], width);
            assert!(
                out_plain[0].segments.iter().any(|s| s.face.label.is_some()),
                "width {width}: {:?}",
                out_plain[0]
            );
        }
    }

    /// `tasks-checklist` :: "A body is dropped whole with the prefix it
    /// hangs from".
    #[test]
    fn a_body_is_dropped_whole_with_the_prefix_it_hangs_from() {
        let source = "      - [x] alpha\n        a body line\n";
        let progress = Progress {
            completed: 1,
            total: 1,
        };
        for width in [78, 58, 12, 6, 5, 4, 3, 2, 1, 0] {
            let out = lines(source, &progress, width);
            for line in &out {
                assert!(
                    columns(&line.text()) <= width as usize,
                    "width {width}: {:?} exceeds its width",
                    line.text()
                );
            }
            if width == 0 {
                assert!(out.is_empty(), "width {width}");
            }
        }

        for width in [78, 58] {
            let out = lines(source, &progress, width);
            let start = first_content_index(&progress, width);
            assert!(
                out[start].text().starts_with("      [✓] alpha"),
                "width {width}: {:?}",
                out[start].text()
            );
            let body_row = out
                .get(start + 1)
                .unwrap_or_else(|| panic!("width {width}: no body row"));
            assert_eq!(
                body_row.text().trim_start(),
                "a body line",
                "width {width}: {:?}",
                body_row.text()
            );
            assert!(
                body_row.text().starts_with(' '),
                "width {width}: {:?}",
                body_row.text()
            );
        }

        // At a width where even `[✓] ` leaves no text column, the item
        // contributes exactly one row — the body dropped whole with the
        // prefix rather than wrapped into zero columns.
        for width in [4, 3, 2, 1] {
            let out = lines(source, &progress, width);
            let start = first_content_index(&progress, width);
            let item_rows = &out[start..];
            assert_eq!(item_rows.len(), 1, "width {width}: {item_rows:?}");
        }

        // The **glyph-only** band, where the six-column indent is gone but
        // `[✓] ` still fits: the body is dropped here too, this being the
        // other half of Decision 9's "glyph-only or truncated-glyph" rule.
        // Measured before the fix, width 5 rendered the body as fourteen
        // one-character rows — verbatim the outcome drop-whole exists to
        // avoid — so a row count is asserted rather than a mere absence.
        for width in [9, 8, 7, 6, 5] {
            let out = lines(source, &progress, width);
            let start = first_content_index(&progress, width);
            let item_rows = &out[start..];
            assert!(
                item_rows[0].text().starts_with("[✓]"),
                "width {width}: the glyph-only prefix did not survive: {:?}",
                item_rows[0].text()
            );
            // The exact claim: at a glyph-only prefix the item renders
            // what the *same item with no body at all* renders. Comparing
            // against a bodiless twin rather than searching for the body's
            // own words is what survives the narrow widths, where the item
            // text itself wraps to three rows and any word of the body
            // would be split across rows before a substring search saw it.
            let bodiless = lines("      - [x] alpha\n", &progress, width);
            let twin = &bodiless[first_content_index(&progress, width)..];
            assert_eq!(
                item_rows, twin,
                "width {width}: a glyph-only prefix kept body rows its bodiless twin does not"
            );
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
                bar.contains('▒') || bar.chars().any(is_braille),
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
            // `task-item-bodies`: "some prose" is group `1. Empty`'s own
            // block, not discarded content, and it draws a row beneath that
            // heading rather than vanishing (design.md -> Decision 10).
            assert!(
                texts.iter().any(|t| t == "some prose"),
                "width {width}: {texts:?}"
            );
            let empty_idx = texts.iter().position(|t| t == "## 1. Empty").unwrap();
            let full_idx = texts.iter().position(|t| t == "## 2. Full").unwrap();
            let prose_idx = texts.iter().position(|t| t == "some prose").unwrap();
            assert!(
                empty_idx < prose_idx && prose_idx < full_idx,
                "width {width}: the prose sits between the two headings — {texts:?}"
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

    /// `task-item-bodies` -> design.md -> Decision 1a and tasks.md 5.3a: a
    /// prose-only document parses to a leading group carrying a block and
    /// no items, so `progress().total == 0` still short-circuits to `No
    /// tasks yet`, but the retained prose is drawn beneath that row rather
    /// than discarded — the whole point of retention is that a line, once
    /// kept by `parse`, is not then dropped by a renderer.
    #[test]
    fn a_prose_only_document_draws_its_blocks_beneath_no_tasks_yet() {
        let source = "Nothing checkable here.\n";
        let progress = Progress {
            completed: 0,
            total: 0,
        };
        for width in [78, 58] {
            let out = lines(source, &progress, width);
            let texts: Vec<String> = out.iter().map(|l| l.text()).collect();
            let no_tasks_idx = texts
                .iter()
                .position(|t| t.trim_end() == "No tasks yet")
                .unwrap_or_else(|| panic!("width {width}: no `No tasks yet` row in {texts:?}"));
            let block_idx = texts
                .iter()
                .position(|t| t == "Nothing checkable here.")
                .unwrap_or_else(|| panic!("width {width}: block not drawn: {texts:?}"));
            assert!(
                no_tasks_idx < block_idx,
                "width {width}: the block must draw beneath the row: {texts:?}"
            );
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
