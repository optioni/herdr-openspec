//! Which coding-agent client the pane launches, decided from evidence rather
//! than from a constant: the `herdr integration status` parser and the
//! five-step kind precedence that reads it.
//!
//! Pure and total on exactly `crate::specs`' terms — no filesystem, process,
//! environment, network, or standard-I/O API, no clock, no global mutable
//! state, no `ratatui` type, no handle on the Herdr half of the subprocess
//! seam, and no panic for any input, the empty string included. The
//! `herdr integration status` call itself lives in `src/launch.rs`'s worker
//! body, which reaches the program through the seam trait it already holds;
//! this module only classifies the text that call returns.
//!
//! It lives outside `src/ui/` deliberately, for the reason `crate::specs`
//! does: a new pure-view file would move `NOIO-VIEW`'s "ten pure files" and
//! `COLWIDTH`'s "nine pure view files", two counts four documents carry. The
//! cost is that no `make gates` script sweeps this file at all, so its
//! freedom from I/O is a `tests/doc_contract.rs` claim over its production
//! slice instead. See `specs/integration-status/spec.md`.

/// One line of `herdr integration status`: an integration Herdr knows about,
/// the status text it printed for it, and whether the user has set it up.
///
/// `status` carries the parsed text verbatim, and exists so [`parse`]'s split
/// rule is **observable**: with only `kind` and `installed`, a first-` (`
/// split and a last-` (` split agree on every line Herdr actually prints, so
/// no assertion over the real corpus can tell a correct parser from
/// `split_once(" (")`. See `specs/integration-status/spec.md` and design.md
/// -> Decisions 14.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Integration {
    pub kind: String,
    pub status: String,
    pub installed: bool,
}

/// The status text Herdr prints for an integration the user has not set up.
/// `installed` is `false` when and only when the status is exactly this:
/// every other status, `current (v9)` and any future `outdated (v7)` alike,
/// is a kind the user set up, because this module asks whether the
/// integration exists rather than which version it is.
const NOT_INSTALLED: &str = "not installed";

/// The kind launched when no evidence is available at all — step 5 of the
/// precedence below. It lives here rather than in `src/config.rs` or under
/// `src/ui/`: it is the last resort of a resolution, not a configuration
/// default, and `WIRED`'s leg 6 forbids the literal in any production slice
/// under `src/ui/`.
pub const LAST_RESORT: &str = "claude";

/// Parse `herdr integration status`' plain text — there is no `--json` form —
/// into one [`Integration`] per non-blank line, in Herdr's own printed order,
/// plus one problem naming each line that could not be read.
///
/// Each line has the shape `<kind>: <status> (<path>)`. The kind is the text
/// before the **first** `: `, trimmed; the status is everything after it up to
/// the **last** ` (` on the line, trimmed — not the first, because
/// `current (v9) (/path)` carries a parenthesised version before the
/// parenthesised path and a first-match split would yield the status
/// `current`.
///
/// A line with no `: `, an empty kind, or no ` (` is skipped with one problem
/// naming it, while every well-formed line in the same output still comes
/// back — `state::read`'s established per-entry rule. A blank read is not a
/// malformed read: the empty string and a run of blank lines both yield
/// nothing and report nothing, because reporting a status that could not be
/// obtained is the caller's job, not this one's.
pub fn parse(text: &str) -> (Vec<Integration>, Vec<String>) {
    let mut integrations = Vec::new();
    let mut problems = Vec::new();

    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Some(integration) = parse_line(line) else {
            problems.push(format!(
                "herdr integration status: could not read the line {line:?}"
            ));
            continue;
        };
        integrations.push(integration);
    }

    (integrations, problems)
}

/// One line, or `None` when it does not have the measured shape. Split out so
/// the two failure conditions read as one `?` chain rather than as a nest.
fn parse_line(line: &str) -> Option<Integration> {
    let (kind, rest) = line.split_once(": ")?;
    let kind = kind.trim();
    if kind.is_empty() {
        return None;
    }
    let (status, _path) = rest.rsplit_once(" (")?;
    let status = status.trim();
    Some(Integration {
        kind: kind.to_string(),
        status: status.to_string(),
        installed: status != NOT_INSTALLED,
    })
}

/// Where a resolved kind came from. Carried on [`Choice::Use`] so a caller can
/// tell an explicit choice from an inferred one without re-deriving it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Configured,
    Recorded,
    SoleIntegration,
    LastResort,
}

/// What [`resolve`] decided: a kind to launch, or a refusal to guess between
/// the kinds the reader has set up. There is no third shape, and no `Use`
/// carries a blank kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Choice {
    Use { kind: String, source: Source },
    Ambiguous { installed: Vec<String> },
}

/// [`resolve`]'s whole answer: the choice, and every reason the reader is
/// owed. Never `Default` — a defaulted `Choice` would be a guess, which is
/// what this module exists not to make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub choice: Choice,
    pub problems: Vec<String>,
}

/// The five-step precedence, as a pure total function of its three arguments.
/// Performs no I/O of any kind and never panics.
///
/// 1. `configured` non-blank — a hand-edited `config.toml` choice always wins,
///    and no further evidence is consulted.
/// 2. `recorded` non-blank — what `settings-window` will write under
///    `HERDR_PLUGIN_STATE_DIR`; this change reads it and never writes it.
/// 3. Exactly one installed integration — not a guess between candidates; it
///    is the only candidate.
/// 4. Two or more installed — [`Choice::Ambiguous`], carrying their kinds in
///    Herdr's own order. The caller answers with a problem row.
/// 5. None installed — [`LAST_RESORT`] under [`Source::LastResort`], with one
///    problem saying so.
///
/// The asymmetry between steps 4 and 5 is deliberate: at step 4 the evidence
/// exists and points two ways at once, and picking one would be exactly the
/// guess `agent-attribution` refuses to make about a terminal title; at step 5
/// there is no evidence at all, and refusing would fail closed, which
/// `SPEC.md` -> Never fail closed forbids.
///
/// A kind that steps 1 or 2 produced and that is **not** among the installed
/// integrations carries one further problem: it launches fine, but every agent
/// started under it reports status `unknown`. Steps 3 and 5 cannot reach that
/// warning — the first by construction, the second because it already carries
/// its own.
pub fn resolve(
    configured: Option<&str>,
    recorded: Option<&str>,
    integrations: &[Integration],
) -> Resolved {
    // `config::non_blank`'s rule, reused rather than restated, so blankness
    // means the same thing at every step that tests for it.
    let chosen = crate::config::non_blank(configured.map(str::to_string))
        .map(|kind| (kind, Source::Configured))
        .or_else(|| {
            crate::config::non_blank(recorded.map(str::to_string))
                .map(|kind| (kind, Source::Recorded))
        });

    if let Some((kind, source)) = chosen {
        let kind = kind.trim().to_string();
        let mut problems = Vec::new();
        if !integrations.iter().any(|i| i.installed && i.kind == kind) {
            problems.push(format!(
                "no herdr integration is installed for agent kind {kind} - \
                 every agent launched under it will report its status as unknown"
            ));
        }
        return Resolved {
            choice: Choice::Use { kind, source },
            problems,
        };
    }

    let installed: Vec<String> = integrations
        .iter()
        .filter(|i| i.installed)
        .map(|i| i.kind.clone())
        .collect();

    match installed.len() {
        1 => Resolved {
            choice: Choice::Use {
                kind: installed
                    .into_iter()
                    .next()
                    .expect("a one-element vector has a first element"),
                source: Source::SoleIntegration,
            },
            problems: Vec::new(),
        },
        0 => Resolved {
            choice: Choice::Use {
                kind: LAST_RESORT.to_string(),
                source: Source::LastResort,
            },
            problems: vec![format!(
                "no herdr agent integration is installed and no agent_kind is configured - \
                 launching {LAST_RESORT} as a last resort"
            )],
        },
        _ => Resolved {
            choice: Choice::Ambiguous { installed },
            problems: Vec::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The seventeen-line `herdr integration status` output measured on the
    /// reference machine, `include_str!`-embedded at compile time — never a
    /// run-time path, so the corpus travels with the binary and a missing
    /// fixture is a compile error rather than a skipped test. `crate::tasks`'
    /// own corpora established the pattern.
    const MEASURED: &str = include_str!("../tests/fixtures/integration-status.txt");

    /// The measured corpus, parsed — the fixture every `resolve` test below
    /// that names "the seventeen measured integrations" uses.
    fn measured() -> Vec<Integration> {
        let (integrations, problems) = parse(MEASURED);
        assert!(problems.is_empty(), "the measured corpus parses whole");
        integrations
    }

    /// The measured corpus with `kinds` installed and every other integration
    /// absent — Herdr's own shape, with only the installed set moved.
    fn installed_only(kinds: &[&str]) -> Vec<Integration> {
        measured()
            .into_iter()
            .map(|i| {
                let installed = kinds.contains(&i.kind.as_str());
                Integration {
                    status: if installed {
                        "current (v9)".to_string()
                    } else {
                        NOT_INSTALLED.to_string()
                    },
                    installed,
                    ..i
                }
            })
            .collect()
    }

    mod parse {
        use super::*;

        #[test]
        fn the_measured_seventeen_line_status_parses_whole() {
            let (integrations, problems) = parse(MEASURED);

            assert_eq!(integrations.len(), 17, "{integrations:?}");
            assert_eq!(
                integrations[0],
                Integration {
                    kind: "pi".to_string(),
                    status: "not installed".to_string(),
                    installed: false,
                },
                "the first entry, in Herdr's own order"
            );

            let installed: Vec<&str> = integrations
                .iter()
                .filter(|i| i.installed)
                .map(|i| i.kind.as_str())
                .collect();
            assert_eq!(installed, vec!["claude", "codex"]);

            assert!(problems.is_empty(), "{problems:?}");
        }

        /// The assertion on `status` is what discriminates the two split
        /// rules: a first-` (` split yields `current`, which fails it, while
        /// both splits agree on `installed` and so prove nothing on their own.
        #[test]
        fn a_version_in_the_status_does_not_become_the_status() {
            let (integrations, problems) =
                parse("claude: current (v9) (/Users/x/.claude/hooks/herdr-agent-state.sh)\n");

            assert_eq!(
                integrations,
                vec![Integration {
                    kind: "claude".to_string(),
                    status: "current (v9)".to_string(),
                    installed: true,
                }]
            );
            assert!(problems.is_empty());
        }

        /// The one line where the two split rules disagree on `installed`
        /// itself — a case no line Herdr actually prints can reach, because
        /// every measured `not installed` line carries no parenthesised
        /// version.
        #[test]
        fn a_parenthesised_version_on_an_absent_integration_discriminates_the_split() {
            let (integrations, problems) = parse("codex: not installed (v1) (/p/hook.sh)\n");

            assert_eq!(
                integrations,
                vec![Integration {
                    kind: "codex".to_string(),
                    status: "not installed (v1)".to_string(),
                    installed: true,
                }],
                "a first-` (` split would yield `not installed` and `installed` false"
            );
            assert!(problems.is_empty());
        }

        #[test]
        fn an_unrecognised_status_is_treated_as_installed() {
            let (integrations, problems) =
                parse("codex: outdated (v3) (/Users/x/.codex/herdr-agent-state.sh)\n");

            assert_eq!(integrations.len(), 1);
            assert!(integrations[0].installed, "{integrations:?}");
            assert!(
                problems.is_empty(),
                "an out-of-date integration is one the user set up: {problems:?}"
            );
        }

        #[test]
        fn a_malformed_line_is_skipped_and_the_rest_survive() {
            let (integrations, problems) = parse(
                "claude: current (v9) (/Users/x/.claude/hooks/herdr-agent-state.sh)\n\
                 this is not an integration line\n\
                 codex: current (v8) (/Users/x/.codex/herdr-agent-state.sh)\n",
            );

            let kinds: Vec<&str> = integrations.iter().map(|i| i.kind.as_str()).collect();
            assert_eq!(kinds, vec!["claude", "codex"]);
            assert_eq!(problems.len(), 1, "{problems:?}");
            assert!(
                problems[0].contains("this is not an integration line"),
                "the problem must name the skipped line: {problems:?}"
            );
        }

        #[test]
        fn empty_and_blank_input_yield_nothing_and_report_nothing() {
            for text in ["", "\n   \n\t\n"] {
                let (integrations, problems) = parse(text);
                assert!(integrations.is_empty(), "{text:?}: {integrations:?}");
                assert!(
                    problems.is_empty(),
                    "{text:?}: a blank read is not a malformed read: {problems:?}"
                );
            }
        }

        /// Totality, including the two shapes `parse_line`'s `?` chain rejects
        /// and the empty-kind case the chain tests for by hand.
        #[test]
        fn every_malformed_shape_is_skipped_without_panicking() {
            for line in [
                "no separator at all (/p)",
                ": no kind (/p)",
                "   : blank kind (/p)",
                "kind: no open paren",
                ":",
                " (",
            ] {
                let (integrations, problems) = parse(line);
                assert!(integrations.is_empty(), "{line:?}: {integrations:?}");
                assert_eq!(problems.len(), 1, "{line:?}: {problems:?}");
            }
        }
    }

    mod resolve {
        use super::*;

        #[test]
        fn a_configured_kind_beats_every_other_source() {
            let resolved = resolve(Some("codex"), Some("gemini"), &installed_only(&["claude"]));

            assert_eq!(
                resolved.choice,
                Choice::Use {
                    kind: "codex".to_string(),
                    source: Source::Configured,
                }
            );
        }

        #[test]
        fn a_recorded_choice_beats_the_evidence_but_not_the_configuration() {
            let integrations = installed_only(&["claude", "codex"]);

            let resolved = resolve(None, Some("gemini"), &integrations);
            assert_eq!(
                resolved.choice,
                Choice::Use {
                    kind: "gemini".to_string(),
                    source: Source::Recorded,
                }
            );

            // The control: the same fixture with nothing recorded refuses, so
            // the recorded value is what suppressed the refusal and nothing
            // else in the fixture is.
            let control = resolve(None, None, &integrations);
            assert_eq!(
                control.choice,
                Choice::Ambiguous {
                    installed: vec!["claude".to_string(), "codex".to_string()],
                }
            );
        }

        #[test]
        fn a_sole_installed_integration_is_used_without_asking() {
            let resolved = resolve(None, None, &installed_only(&["codex"]));

            assert_eq!(
                resolved.choice,
                Choice::Use {
                    kind: "codex".to_string(),
                    source: Source::SoleIntegration,
                }
            );
            assert!(resolved.problems.is_empty(), "{:?}", resolved.problems);
        }

        #[test]
        fn two_installed_integrations_are_refused_not_guessed_between() {
            let resolved = resolve(None, None, &measured());

            assert_eq!(
                resolved.choice,
                Choice::Ambiguous {
                    installed: vec!["claude".to_string(), "codex".to_string()],
                },
                "in Herdr's own printed order"
            );
            assert!(
                resolved.problems.is_empty(),
                "turning Ambiguous into a reason the reader sees is agent-launch's job: {:?}",
                resolved.problems
            );
        }

        #[test]
        fn nothing_installed_and_nothing_configured_reaches_claude_and_says_so() {
            let resolved = resolve(None, None, &installed_only(&[]));

            assert_eq!(
                resolved.choice,
                Choice::Use {
                    kind: "claude".to_string(),
                    source: Source::LastResort,
                }
            );
            assert_eq!(resolved.problems.len(), 1, "{:?}", resolved.problems);
            assert!(
                resolved.problems[0].contains("agent_kind")
                    && resolved.problems[0].contains("claude"),
                "{:?}",
                resolved.problems
            );
        }

        #[test]
        fn a_blank_configured_or_recorded_value_is_not_a_value() {
            let sole = installed_only(&["codex"]);
            let expected = Choice::Use {
                kind: "codex".to_string(),
                source: Source::SoleIntegration,
            };

            for blank in ["", "   ", "\t"] {
                assert_eq!(
                    resolve(Some(blank), None, &sole).choice,
                    expected,
                    "configured {blank:?}"
                );
                assert_eq!(
                    resolve(None, Some(blank), &sole).choice,
                    expected,
                    "recorded {blank:?}"
                );
            }

            assert_eq!(
                resolve(Some("  codex  "), None, &sole).choice,
                Choice::Use {
                    kind: "codex".to_string(),
                    source: Source::Configured,
                },
                "trimmed, not rejected"
            );
        }

        #[test]
        fn every_combination_is_total() {
            let fixtures = [
                installed_only(&[]),
                installed_only(&["codex"]),
                installed_only(&["claude", "codex"]),
            ];
            let mut calls = 0usize;
            for configured in [None, Some(""), Some("x")] {
                for recorded in [None, Some(""), Some("x")] {
                    for integrations in &fixtures {
                        let resolved = resolve(configured, recorded, integrations);
                        calls += 1;
                        match resolved.choice {
                            Choice::Use { kind, .. } => assert!(
                                !kind.trim().is_empty(),
                                "{configured:?}/{recorded:?}: a Use must carry a non-blank kind"
                            ),
                            Choice::Ambiguous { installed } => assert!(
                                installed.len() >= 2,
                                "{configured:?}/{recorded:?}: {installed:?}"
                            ),
                        }
                    }
                }
            }
            assert_eq!(calls, 27);
        }

        /// `cursor` is listed by Herdr and reads `not installed` on the
        /// reference machine — the case this warning is about. The fixture is
        /// the measured corpus itself, so the absence is Herdr's own rather
        /// than one this test invented.
        #[test]
        fn a_configured_kind_with_no_integration_warns_and_still_resolves() {
            let integrations = measured();

            let resolved = resolve(Some("cursor"), None, &integrations);
            assert_eq!(
                resolved.choice,
                Choice::Use {
                    kind: "cursor".to_string(),
                    source: Source::Configured,
                }
            );
            assert_eq!(resolved.problems.len(), 1, "{:?}", resolved.problems);
            assert!(
                resolved.problems[0].contains("cursor") && resolved.problems[0].contains("unknown"),
                "{:?}",
                resolved.problems
            );

            // The control: a kind that *is* installed carries no warning, so
            // the absence causes it and nothing else in the fixture does.
            let control = resolve(Some("codex"), None, &integrations);
            assert_eq!(
                control.choice,
                Choice::Use {
                    kind: "codex".to_string(),
                    source: Source::Configured,
                }
            );
            assert!(control.problems.is_empty(), "{:?}", control.problems);
        }

        #[test]
        fn a_kind_herdr_never_listed_is_warned_about_on_the_same_terms() {
            let resolved = resolve(Some("not-a-kind"), None, &measured());

            assert_eq!(
                resolved.choice,
                Choice::Use {
                    kind: "not-a-kind".to_string(),
                    source: Source::Configured,
                }
            );
            assert_eq!(resolved.problems.len(), 1, "{:?}", resolved.problems);
            assert!(resolved.problems[0].contains("not-a-kind"));
        }

        #[test]
        fn a_sole_integration_and_a_last_resort_carry_no_absence_warning() {
            let sole = resolve(None, None, &installed_only(&["codex"]));
            assert!(sole.problems.is_empty(), "{:?}", sole.problems);

            let last = resolve(None, None, &installed_only(&[]));
            assert_eq!(
                last.problems.len(),
                1,
                "the nothing-installed warning and no second one about claude's own \
                 integration being absent: {:?}",
                last.problems
            );
        }

        /// A recorded kind reaches the absence warning on the same terms a
        /// configured one does — the second of the two sources this warning
        /// is stated for.
        #[test]
        fn a_recorded_kind_with_no_integration_warns_too() {
            let resolved = resolve(None, Some("cursor"), &measured());

            assert_eq!(
                resolved.choice,
                Choice::Use {
                    kind: "cursor".to_string(),
                    source: Source::Recorded,
                }
            );
            assert_eq!(resolved.problems.len(), 1, "{:?}", resolved.problems);
        }
    }
}
