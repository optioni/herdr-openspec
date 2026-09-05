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

#[cfg(test)]
mod tests {
    use super::progress_bar;
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
}
