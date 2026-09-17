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
//! RED: task 3.1. The production items this test module names —
//! [`render`], [`rows`], [`content_rows`], [`Row`], and the two row-text
//! helpers — do not exist yet; task 3.2 adds them.

#[cfg(test)]
mod tests {
    use super::{Row, content_rows, fit, render, rows, source_text, value_text};
    use crate::integration::{Choice, Source};
    use crate::resolve;
    use crate::settings::{self, KindResolution};
    use crate::testutil::{cell, row_text};
    use crate::ui::layout::{columns, overlay_band, split_frame};
    use crate::config;
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
