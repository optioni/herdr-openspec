//! The settings panel's row grammar and render function. See
//! `openspec/changes/settings-window/specs/settings-window/spec.md` -> "The
//! panel renders three settings, each as a value row and a source row", and
//! `design.md` -> Boundaries: `src/ui/help.rs` is the pattern this file
//! follows — a pure view drawing a band, its own `*widths` gate.
//!
//! Pure and total, on exactly `ui::help`'s terms: no filesystem, process,
//! environment, network, or standard-I/O API, no clock, no global mutable
//! state, and no panic for any input. `Dashboard::settings` — the field that
//! will hold the rows this module renders — does not exist yet
//! (`settings-window`'s later task group 4 adds it), so [`render`] takes the
//! rows and the row cursor as parameters rather than reading a `Dashboard`
//! field. That is the signature group 4 calls this with: `&dashboard.settings.rows`
//! and `dashboard.settings.cursor`.
//!
//! [`render`]'s own band grammar, on `ui::help`'s terms: a top rule row, the
//! panel's own interior — a heading row reading `Settings`, then a value row
//! and an indented source row per setting — and a bottom rule row. Unlike the
//! help band, neither rule row carries an embedded title: the settings
//! band's heading is a row of the interior instead, because
//! `settings-window`'s row-grammar requirement lists the heading row and the
//! top rule row as two separate elements. `─` is the only chrome glyph
//! either rule row draws.

use crate::settings::{Editable, Reason, Setting};
use crate::ui::layout::{columns, truncate_columns};
use crate::ui::palette::{self, Role};
use ratatui::Frame;
use ratatui::layout::Rect;

/// One row of the panel's interior grammar — `ui::help::Row`'s own shape,
/// duplicated here rather than shared, because the two modules' rows carry
/// unrelated content and sharing the type would couple them for no reason
/// beyond the coincidence that both are `Vec<(String, Role)>`.
pub struct Row {
    pub segments: Vec<(String, Role)>,
}

/// The number of rows the band's interior carries: one heading row, plus a
/// value row and a source row per entry in `settings` — `1 + settings.len() *
/// 2`, total over any length rather than fixed at three, so a caller handing
/// an empty or a longer slice still gets a coherent count.
/// `setting-provenance` always hands exactly three, and
/// `content_rows(&[a, b, c])` is `7` (design.md -> "Both panels are centred
/// by the one function").
pub fn content_rows(settings: &[Setting]) -> usize {
    1 + settings.len() * 2
}

/// The longest prefix of `text` that fits in `width` display columns, padded
/// with spaces to exactly `width` columns when it is shorter —
/// `ui::help::fit`'s own rule, duplicated for the reason [`Row`] is: the two
/// modules' grammars share no state. Never sliced by `char`: both halves it
/// uses, [`truncate_columns`] and [`columns`], are `ui::layout`'s.
fn fit(text: &str, width: usize) -> String {
    let truncated = truncate_columns(text, width);
    let used = columns(truncated);
    let mut out = truncated.to_string();
    out.push_str(&" ".repeat(width.saturating_sub(used)));
    out
}

/// The value row's own text: `key: value`, or `key: <value>` when the
/// setting is editable — the affordance `settings-window`'s row-grammar
/// requirement names, two ASCII characters bracketing the value alone.
fn value_text(setting: &Setting) -> String {
    match &setting.editable {
        Editable::Kind { .. } => format!("{}: <{}>", setting.key, setting.value),
        Editable::No { .. } => format!("{}: {}", setting.key, setting.value),
    }
}

/// The source row's own text: the provenance's own label, with `, read-only`
/// appended for a set-once setting and a `config.toml` pointer appended for
/// one refused for want of an installed integration — the two reasons this
/// group's own scenarios name. `Reason::Configured`'s row needs no further
/// text: `Provenance::Configured`'s own label is already `config.toml`, and
/// `Reason::Resolving`'s row needs none either: `Provenance::Pending`'s own
/// label is already `resolving`.
fn source_text(setting: &Setting) -> String {
    match &setting.editable {
        Editable::No {
            reason: Reason::SetOnce,
        } => format!("{}, read-only", setting.provenance.label()),
        Editable::No {
            reason: Reason::NoIntegration,
        } => format!("{}; set it in config.toml", setting.provenance.label()),
        _ => setting.provenance.label(),
    }
}

/// The band's interior grammar at `width` display columns: a heading row
/// reading `Settings`, then a value row and an indented source row per entry
/// in `settings` — [`content_rows`] rows in total, regardless of `width`.
/// This is the interior alone: no top or bottom rule row, on exactly
/// `ui::help::rows`' own division of labour.
pub fn rows(width: u16, settings: &[Setting]) -> Vec<Row> {
    let w = width as usize;
    let mut out = Vec::with_capacity(content_rows(settings));
    out.push(Row {
        segments: vec![(fit("Settings", w), Role::RegionHeadingFocused)],
    });
    for setting in settings {
        out.push(Row {
            segments: vec![(fit(&value_text(setting), w), Role::ListRow)],
        });
        let source = format!("  {}", source_text(setting));
        out.push(Row {
            segments: vec![(fit(&source, w), Role::Muted)],
        });
    }
    out
}

/// A plain rule row: `─` repeated to `width` — the band's own top and bottom
/// rows, on exactly `ui::help::bottom_rule_row`'s undecorated form. Unlike
/// the help band's top rule, neither of this band's rule rows embeds a
/// title.
fn rule_row(width: usize) -> Row {
    Row {
        segments: vec![("─".repeat(width), Role::RegionRule)],
    }
}

/// Draw one already-fitted [`Row`] at `(x, y)`, cell by cell, advancing by
/// each segment's own [`columns`] width — `ui::help::draw_row`'s own rule,
/// duplicated for the reason [`Row`] itself is.
fn draw_row(buf: &mut ratatui::buffer::Buffer, x: u16, y: u16, row: &Row) {
    let mut cx = x;
    for (text, role) in &row.segments {
        buf.set_string(cx, y, text, palette::style(*role));
        cx = cx.saturating_add(columns(text) as u16);
    }
}

/// Draw the settings panel into `body`: [`crate::ui::layout::overlay_band`]'s
/// own rectangle, a top rule row, a window onto [`rows`] centred on `cursor`
/// through [`crate::ui::layout::viewport`], and a bottom rule row. Total over
/// every `Rect`, degenerate ones included, and never panics — on exactly
/// `ui::help::render`'s own terms.
///
/// `settings` and `cursor` are parameters rather than fields read from a
/// `Dashboard`, because `Dashboard::settings` does not exist yet
/// (`settings-window`'s later task group 4 adds it); that group calls this
/// with `&dashboard.settings.rows` and `dashboard.settings.cursor`. `cursor`
/// indexes `settings`, not the rendered row list — it is translated to the
/// row grammar's own coordinates (the cursor's own setting's value row,
/// `1 + 2 * cursor`, the heading taking row `0`) before being handed to
/// `viewport`, on exactly the translation `settings-window`'s row-cursor
/// requirement states. An empty `settings` slice clamps the cursor to the
/// heading row rather than indexing out of bounds.
pub fn render(frame: &mut Frame, body: Rect, settings: &[Setting], cursor: usize) {
    let total = content_rows(settings);
    let area = crate::ui::layout::overlay_band(body, total);
    if area.width == 0 || area.height == 0 {
        return;
    }
    let width = area.width as usize;
    let buf = frame.buffer_mut();
    draw_row(buf, area.x, area.y, &rule_row(width));
    if area.height < 2 {
        return;
    }
    let interior_height = (area.height - 2) as usize;
    let cursor_row = if settings.is_empty() {
        0
    } else {
        1 + 2 * cursor.min(settings.len() - 1)
    };
    let offset = crate::ui::layout::viewport(total, cursor_row, interior_height as u16);
    for (i, row) in rows(area.width, settings)
        .iter()
        .enumerate()
        .skip(offset)
        .take(interior_height)
    {
        let y = area.y + 1 + (i - offset) as u16;
        draw_row(buf, area.x, y, row);
    }
    let bottom_y = area.y + area.height - 1;
    draw_row(buf, area.x, bottom_y, &rule_row(width));
}

#[cfg(test)]
mod tests {
    use super::{Row, content_rows, fit, render, rows, source_text, value_text};
    use crate::config;
    use crate::integration::{Choice, Source};
    use crate::resolve;
    use crate::settings::{self, KindResolution};
    use crate::testutil::{cell, row_text};
    use crate::ui::layout::{columns, overlay_band, split_frame};
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;
    use std::path::PathBuf;

    /// The **body** a full dashboard render at `total` would hand to
    /// [`render`] — `layout::split_frame`'s own body region, the footer row
    /// excluded, on exactly `ui::help`'s own `body_for` helper.
    fn body_for(total: Rect) -> Rect {
        let (body, _) = split_frame(total);
        body
    }

    /// The three settings `setting-provenance` produces for a dashboard
    /// whose `config.toml` sets `archived_count = 9` and no other key,
    /// whose `agent_kind` resolved to `codex` from the sole installed
    /// integration, and whose `openspec_bin` resolved at probe step 2 (the
    /// `PATH` step) — the fixture `settings-window` :: "The panel renders
    /// every setting at both mandated widths" names.
    fn fixture_rows() -> Vec<settings::Setting> {
        let cfg = config::Config {
            archived_count: 9,
            ..config::Config::default()
        };
        let binary = resolve::BinResolution {
            found: Some(resolve::FoundBin {
                path: PathBuf::from("/usr/bin/openspec"),
                source: resolve::BinSource::Path,
            }),
            problems: Vec::new(),
        };
        let kind = KindResolution {
            choice: Choice::Use {
                kind: "codex".to_string(),
                source: Source::SoleIntegration,
            },
            installed: vec!["codex".to_string()],
        };
        settings::settings(&cfg, &binary, Some(&kind))
    }

    fn draw(total: Rect, settings_rows: &[settings::Setting]) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(total.width, total.height);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        let body = body_for(total);
        terminal
            .draw(|frame| render(frame, body, settings_rows, 0))
            .expect("draw a frame");
        terminal.backend().buffer().clone()
    }

    fn row_plain_text(row: &Row) -> String {
        row.segments.iter().map(|(text, _)| text.as_str()).collect()
    }

    /// Every rendered row of the band, top rule through bottom rule, equals
    /// [`rows`]'s own text at the band's width — the render path never
    /// disagrees with the grammar it is built from.
    fn assert_panel_renders(total: Rect, settings_rows: &[settings::Setting]) {
        let body = body_for(total);
        let band = overlay_band(body, content_rows(settings_rows));
        let buffer = draw(total, settings_rows);

        assert_eq!(
            row_text(&buffer, band.y),
            "─".repeat(band.width as usize),
            "top rule at {}x{}",
            total.width,
            total.height
        );

        let content = rows(band.width, settings_rows);
        assert_eq!(content.len(), content_rows(settings_rows));
        for (i, row) in content.iter().enumerate() {
            let y = band.y + 1 + i as u16;
            assert_eq!(
                row_text(&buffer, y),
                row_plain_text(row),
                "interior row {i} at {}x{}",
                total.width,
                total.height
            );
        }

        let bottom_y = band.y + band.height - 1;
        assert_eq!(
            row_text(&buffer, bottom_y),
            "─".repeat(band.width as usize),
            "bottom rule at {}x{}",
            total.width,
            total.height
        );
    }

    /// `settings-window` :: "The panel renders every setting at both
    /// mandated widths".
    #[test]
    fn the_panel_renders_every_setting_at_both_mandated_widths() {
        let settings_rows = fixture_rows();
        for total in [Rect::new(0, 0, 120, 40), Rect::new(0, 0, 60, 20)] {
            assert_panel_renders(total, &settings_rows);
        }

        assert_eq!(settings_rows.len(), 3);
        let openspec_bin = &settings_rows[0];
        let agent_kind = &settings_rows[1];
        let prompts = &settings_rows[2];
        assert_eq!(openspec_bin.key, "openspec_bin");
        assert_eq!(agent_kind.key, "agent_kind");
        assert_eq!(prompts.key, "prompts");

        assert!(value_text(openspec_bin).contains("/usr/bin/openspec"));
        assert!(
            source_text(openspec_bin).contains("PATH"),
            "openspec_bin's source names the probe step: {}",
            source_text(openspec_bin)
        );
        assert!(source_text(openspec_bin).contains("read-only"));

        assert!(
            value_text(agent_kind).contains("<codex>"),
            "an editable setting's value is bracketed: {}",
            value_text(agent_kind)
        );
        assert!(
            source_text(agent_kind).contains("the sole installed integration"),
            "agent_kind's source names the sole installed integration: {}",
            source_text(agent_kind)
        );

        assert!(value_text(prompts).contains("no per-kind overrides"));
        assert!(
            source_text(prompts).contains("the default"),
            "prompts' source names the default: {}",
            source_text(prompts)
        );
        assert!(source_text(prompts).contains("read-only"));

        for setting in &settings_rows {
            assert_ne!(setting.key, "archived_count");
            assert!(!value_text(setting).contains("archived_count"));
            assert!(!source_text(setting).contains("archived_count"));
        }
    }

    /// `settings-window` :: "A long path is truncated rather than wrapped or
    /// overflowing". Task 3.1 asserts at both mandated widths even though
    /// the spec's own scenario names 60x20 alone.
    #[test]
    fn a_long_path_is_truncated_rather_than_wrapped_or_overflowing() {
        let long_path = "x".repeat(200);
        let cfg = config::Config::default();
        let binary = resolve::BinResolution {
            found: Some(resolve::FoundBin {
                path: PathBuf::from(&long_path),
                source: resolve::BinSource::Path,
            }),
            problems: Vec::new(),
        };
        let kind = KindResolution {
            choice: Choice::Use {
                kind: "codex".to_string(),
                source: Source::SoleIntegration,
            },
            installed: vec!["codex".to_string()],
        };
        let settings_rows = settings::settings(&cfg, &binary, Some(&kind));

        for total in [Rect::new(0, 0, 60, 20), Rect::new(0, 0, 120, 40)] {
            let body = body_for(total);
            let band = overlay_band(body, content_rows(&settings_rows));
            let buffer = draw(total, &settings_rows);

            // The openspec_bin value row is the band's second interior row:
            // one row below the top rule for the heading, one more for the
            // value row itself.
            let value_row_y = band.y + 2;
            let text = row_text(&buffer, value_row_y);
            assert_eq!(
                columns(&text),
                band.width as usize,
                "the value row is exactly the band's width at {}x{}",
                total.width,
                total.height
            );
            assert_eq!(
                text,
                fit(&value_text(&settings_rows[0]), band.width as usize),
                "the value row is the fitted (truncated or padded) value text"
            );

            // No row of the panel exceeds the band's own width, measured by
            // `columns` rather than by byte or `char` length.
            for row in rows(band.width, &settings_rows) {
                let total_cols: usize = row.segments.iter().map(|(t, _)| columns(t)).sum();
                assert!(
                    total_cols <= band.width as usize,
                    "a row exceeded the band's width at {}x{}",
                    total.width,
                    total.height
                );
            }
        }
    }

    /// `settings-window` :: "The panel degrades rather than panicking at any
    /// frame size".
    #[test]
    fn the_panel_degrades_rather_than_panicking_at_any_frame_size() {
        let settings_rows = fixture_rows();
        let backend = TestBackend::new(130, 60);
        let mut terminal = ratatui::Terminal::new(backend).expect("construct terminal");
        for (w, h) in [(1u16, 1u16), (60, 1), (60, 2), (120, 3), (0u16, 0u16)] {
            let total = Rect::new(0, 0, w, h);
            let body = body_for(total);
            let band = overlay_band(body, content_rows(&settings_rows));
            terminal
                .draw(|frame| render(frame, body, &settings_rows, 0))
                .unwrap_or_else(|e| panic!("draw at {w}x{h} failed: {e}"));
            let buffer = terminal.backend().buffer().clone();
            let band_row = |y: u16| -> String {
                (0..band.width)
                    .map(|x| cell(&buffer, x, y).symbol())
                    .collect()
            };
            match (w, h) {
                (60, 1) | (60, 2) => {
                    assert_eq!(band.height, 1, "{w}x{h}: body one row -> band height one");
                    assert_eq!(
                        band_row(band.y),
                        "─".repeat(60),
                        "{w}x{h}: the top rule alone"
                    );
                }
                (120, 3) => {
                    assert_eq!(band.height, 2, "120x3: body two rows -> band height two");
                    assert_eq!(band_row(band.y), "─".repeat(120));
                    assert_eq!(
                        band_row(band.y + 1),
                        "─".repeat(120),
                        "the bottom rule, no interior"
                    );
                }
                (1, 1) => {
                    assert_eq!(band_row(band.y), "─");
                }
                (0, 0) => {
                    assert_eq!(band.height, 0, "a zero-height band draws nothing at all");
                }
                _ => {}
            }
        }
    }
}
