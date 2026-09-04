//! Pure invocation classification for the `herdr-openspec` binary.
//!
//! Nothing here performs I/O: `parse` turns argument strings into an
//! [`Invocation`], and `usage`, `banner`, and `rejection_text` produce the
//! strings `src/main.rs` writes to stdout or stderr. Keeping this logic here
//! rather than in `main` is what makes it unit-testable — see design.md ->
//! Decisions ("Library plus thin `main`").

pub mod config;
pub mod state;

/// Test-only helpers shared by `config` and `state`'s unit tests.
#[cfg(test)]
pub(crate) mod testutil {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A uniquely named directory under `std::env::temp_dir()`, created on
    /// construction and removed (recursively) on drop. Named
    /// `herdr-openspec-test-<pid>-<counter>` — predictable, so a test can assert a
    /// path's absence up front and a leak-check can search for the prefix if a
    /// panicking test ever leaves one behind. No `tempfile` dependency: see
    /// `openspec/changes/plugin-config/design.md` -> Decisions.
    pub(crate) struct ScratchDir {
        path: PathBuf,
    }

    impl ScratchDir {
        pub(crate) fn new() -> Self {
            static COUNTER: AtomicUsize = AtomicUsize::new(0);
            let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "herdr-openspec-test-{}-{counter}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).expect("create scratch dir");
            Self { path }
        }

        pub(crate) fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

/// The classified shape of an invocation of the binary.
#[derive(Debug, PartialEq, Eq)]
pub enum Invocation {
    /// `ui` alone: render the placeholder dashboard pane.
    Ui,
    /// Anything else. Carries the offending token, when there is one — the
    /// unrecognised first argument, or the trailing argument after `ui`.
    /// `None` means the argument list was empty.
    Reject(Option<String>),
}

/// Classify a program's arguments (excluding argv[0]).
pub fn parse(args: &[&str]) -> Invocation {
    match args {
        [] => Invocation::Reject(None),
        [first] if *first == "ui" => Invocation::Ui,
        ["ui", rest, ..] => Invocation::Reject(Some((*rest).to_string())),
        [first, ..] => Invocation::Reject(Some((*first).to_string())),
    }
}

/// Usage text printed to stderr for any rejected invocation.
pub fn usage() -> &'static str {
    "usage: herdr-openspec <ui>\n\nCommands:\n  ui    Run the OpenSpec dashboard pane\n"
}

/// The placeholder banner printed to stdout on `ui`.
pub fn banner() -> String {
    format!(
        "herdr-openspec {}\nThe OpenSpec dashboard is not implemented yet.\n",
        env!("CARGO_PKG_VERSION")
    )
}

/// The full stderr text for a rejected invocation: an optional line naming
/// the offending token, followed by usage.
pub fn rejection_text(token: Option<&str>) -> String {
    match token {
        Some(token) => format!("error: unrecognized argument: {token}\n\n{}", usage()),
        None => usage().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_alone_classifies_as_ui() {
        assert_eq!(parse(&["ui"]), Invocation::Ui);
    }

    #[test]
    fn unrecognised_first_argument_is_a_rejection_carrying_the_token() {
        assert_eq!(parse(&["wat"]), Invocation::Reject(Some("wat".to_string())));
    }

    #[test]
    fn no_arguments_is_a_rejection_with_no_token() {
        assert_eq!(parse(&[]), Invocation::Reject(None));
    }

    #[test]
    fn ui_followed_by_an_argument_is_a_rejection_carrying_that_argument() {
        assert_eq!(
            parse(&["ui", "--tab"]),
            Invocation::Reject(Some("--tab".to_string()))
        );
    }

    #[test]
    fn banner_names_the_plugin_id_version_and_not_implemented_line() {
        let banner = banner();
        assert!(banner.contains("herdr-openspec"));
        assert!(banner.contains(env!("CARGO_PKG_VERSION")));
        assert!(banner.to_lowercase().contains("not implemented"));
    }

    #[test]
    fn usage_lists_ui() {
        assert!(usage().contains("ui"));
    }
}
