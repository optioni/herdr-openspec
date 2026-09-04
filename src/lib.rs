//! Pure invocation classification for the `herdr-openspec` binary.
//!
//! Nothing here performs I/O: `parse` turns argument strings into an
//! [`Invocation`], and `usage`, `banner`, and `rejection_text` produce the
//! strings `src/main.rs` writes to stdout or stderr. Keeping this logic here
//! rather than in `main` is what makes it unit-testable — see design.md ->
//! Decisions ("Library plus thin `main`").

pub mod config;
pub mod resolve;
pub mod schema;
pub mod state;
pub mod tasks;

/// The current process id. Exists so `state::record`'s temporary-file name
/// can include it without `src/state.rs` itself naming the standard-library
/// process module: that module is checked by the stricter, module-scoped
/// form of the "resolution spawns nothing" gate that applies to
/// `src/config.rs` and `src/state.rs` only, forbidding any process API there
/// at all — spawning or not. See `openspec/changes/plugin-config/design.md`
/// -> Boundaries and Test Strategy for the exact check (deliberately not
/// reproduced here, so this comment cannot itself trip the tree-wide half of
/// that same check). Reading the current process id is not a spawn.
pub(crate) fn pid() -> u32 {
    std::process::id()
}

/// Test-only helpers shared by `config` and `state`'s unit tests.
#[cfg(test)]
pub(crate) mod testutil {
    use std::os::unix::fs::PermissionsExt;
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
            let path = std::env::temp_dir()
                .join(format!("herdr-openspec-test-{}-{counter}", crate::pid()));
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

    /// A snapshot of a directory's listing, every file's bytes, and every file's
    /// modification time — read through [`std::fs::Metadata`], never by shelling
    /// out to `stat`, whose flags differ between BSD and GNU. Two snapshots taken
    /// around an operation and compared for equality is how "the tree is
    /// untouched" is proven, rather than by checking the listing alone.
    ///
    /// Directory entries are recorded too (path, empty bytes, own modification
    /// time), not only files: `collect` originally pushed an entry only for a
    /// non-directory, so an empty directory contributed nothing and a
    /// `create_dir_all` — the likeliest accidental write — was invisible to a
    /// comparison of two snapshots. `config` and `state`'s existing snapshot
    /// assertions are equality comparisons over two snapshots taken with the
    /// same (extended) function, so they stay green with directory entries added
    /// to both sides.
    #[derive(Debug, PartialEq, Eq)]
    pub(crate) struct Snapshot(Vec<(PathBuf, Vec<u8>, std::time::SystemTime)>);

    pub(crate) fn snapshot(dir: &Path) -> Snapshot {
        let mut entries = Vec::new();
        collect(dir, &mut entries);
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        Snapshot(entries)
    }

    fn collect(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>, std::time::SystemTime)>) {
        let Ok(read_dir) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Ok(metadata) = entry.metadata() {
                    let mtime = metadata.modified().expect("modified time");
                    out.push((path.clone(), Vec::new(), mtime));
                }
                collect(&path, out);
            } else if let Ok(metadata) = entry.metadata() {
                let bytes = std::fs::read(&path).unwrap_or_default();
                let mtime = metadata.modified().expect("modified time");
                out.push((path, bytes, mtime));
            }
        }
    }

    /// Write `contents` to `path`, creating parent directories as needed, and
    /// set its permission bits explicitly to `mode`. Never relies on the
    /// process umask, which differs between an interactive shell and a CI
    /// runner and would make "mode 0644" mean something else depending on who
    /// runs the suite. The fixture "binary" this repository's tests build is
    /// always a short text file, never a copied real executable.
    pub(crate) fn write_with_mode(path: &Path, contents: &[u8], mode: u32) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create fixture parent directory");
        }
        std::fs::write(path, contents).expect("write fixture file");
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
            .expect("set fixture file mode");
    }

    /// Create a symbolic link at `link` pointing to `original`, creating
    /// `link`'s parent directories as needed. `original` need not exist —
    /// several scenarios need a dangling link.
    pub(crate) fn symlink(original: &Path, link: &Path) {
        if let Some(parent) = link.parent() {
            std::fs::create_dir_all(parent).expect("create fixture parent directory");
        }
        std::os::unix::fs::symlink(original, link).expect("create fixture symlink");
    }

    /// Canonicalize `path`, on the assumption that it exists. Every discovery
    /// assertion compares against `canonicalize(expected)` rather than a raw
    /// `ScratchDir` path: on macOS `std::env::temp_dir()` sits under
    /// `/var/folders/...`, and `/var` is a symlink to `/private/var`, so an
    /// assertion against the raw path passes on Linux and fails on macOS.
    pub(crate) fn canonical(path: &Path) -> PathBuf {
        std::fs::canonicalize(path).expect("canonicalize expected path")
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
