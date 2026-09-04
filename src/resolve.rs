//! Repository discovery and `openspec` binary resolution.
//!
//! See `openspec/changes/repo-resolution/design.md` for the full contract.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// The outcome of walking up from a starting path for an `openspec` directory.
/// See `openspec/changes/repo-resolution/design.md` -> Contracts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepoSearch {
    /// The nearest ancestor (including the starting path itself) holding an
    /// `openspec` directory child.
    Found { root: PathBuf },
    /// No ancestor up to the filesystem root holds one. Names the directory
    /// the search began from, so the empty state can print it.
    NotFound { searched_from: PathBuf },
}

/// Is `path` a directory, following symbolic links? Every `Err` from the
/// filesystem — an unreadable or non-existent path — means "no", never a
/// panic.
fn is_dir_through_symlinks(path: &Path) -> bool {
    std::fs::metadata(path).map(|m| m.is_dir()).unwrap_or(false)
}

/// Locate the OpenSpec repository by walking `start` and its ancestors,
/// stopping at the first one whose `openspec` child is a directory (through
/// symbolic links), and at the filesystem root. See
/// `openspec/changes/repo-resolution/design.md` -> Contracts and
/// `openspec/changes/repo-resolution/specs/repo-discovery/spec.md`.
pub fn find_repo(start: &Path) -> RepoSearch {
    match std::fs::canonicalize(start) {
        Ok(canonical_start) => {
            for ancestor in canonical_start.ancestors() {
                if is_dir_through_symlinks(&ancestor.join("openspec")) {
                    return RepoSearch::Found {
                        root: ancestor.to_path_buf(),
                    };
                }
            }
            RepoSearch::NotFound {
                searched_from: canonical_start,
            }
        }
        Err(_) => {
            // `start` cannot be resolved — most commonly because it does not
            // exist. Walk the path as given, but skip the *empty* path that
            // `Path::ancestors` yields at the end of a relative chain: joining
            // `openspec` to it would produce a bare relative `openspec`,
            // resolved against the process's own working directory rather
            // than against anything the caller named.
            for ancestor in start.ancestors() {
                if ancestor.as_os_str().is_empty() {
                    continue;
                }
                if is_dir_through_symlinks(&ancestor.join("openspec")) {
                    return RepoSearch::Found {
                        root: ancestor.to_path_buf(),
                    };
                }
            }
            RepoSearch::NotFound {
                searched_from: start.to_path_buf(),
            }
        }
    }
}

/// Which step of the probe chain produced a binary. Part of the contract,
/// not a debugging aid: two steps can legitimately produce the same path (on
/// the reference machine, the nvm step and the npm-prefix step do), so path
/// equality alone cannot distinguish which one actually ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinSource {
    Configured,
    Path,
    Nvm,
    NpmPrefix,
}

/// A usable `openspec` binary and the step that found it. The path is
/// returned exactly as the chain constructed it — never canonicalized — so a
/// caller that later spawns it uses the same stable, upgrade-surviving name
/// (a symbolic link, on the reference machine).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundBin {
    pub path: PathBuf,
    pub source: BinSource,
}

/// The outcome of probing for the `openspec` binary. `problems` has exactly
/// one producer today — a configured `openspec_bin` that could not be used —
/// so an empty `problems` with `found: None` means "no CLI installed" and a
/// non-empty one means "the user configured something that does not work".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinResolution {
    pub found: Option<FoundBin>,
    pub problems: Vec<String>,
}

/// Is `path` a usable `openspec` candidate: an executable regular file,
/// reached through symbolic links? `fs::metadata` follows symlinks —
/// `symlink_metadata` does not, and the real install on the reference
/// machine is a symlink to a `.js` file. A directory carries the execute
/// bit, so the regular-file clause is load-bearing on its own; "any execute
/// bit" rather than "owner execute" because a binary installed by another
/// user with mode `0711` is still runnable. Every filesystem `Err` means
/// "not usable", never a panic.
fn is_usable_binary(path: &Path) -> bool {
    match std::fs::metadata(path) {
        Ok(metadata) => metadata.is_file() && metadata.permissions().mode() & 0o111 != 0,
        Err(_) => false,
    }
}

/// Step 2's candidate list, as a pure function of the `PATH` string — no
/// filesystem access. Split by hand on `':'` rather than
/// `std::env::split_paths`, which takes an `OsStr` while the injected lookup
/// is `String`-typed and, on Unix, yields empty entries that would still
/// need filtering. An entry that is empty or whitespace-only is skipped
/// rather than treated as the current directory: POSIX says an empty `PATH`
/// entry means the current directory, but a plugin pane starts in whatever
/// directory the user's workspace is rooted at, and resolving an executable
/// from it is not a behaviour this plugin offers. Reuses `config::non_blank`
/// for the identical "first non-blank value wins" rule the rest of the crate
/// already applies to environment values.
pub(crate) fn path_candidates(path_value: &str) -> Vec<PathBuf> {
    path_value
        .split(':')
        .filter(|entry| crate::config::non_blank(Some((*entry).to_string())).is_some())
        .map(|entry| Path::new(entry).join("openspec"))
        .collect()
}

/// Probe for the `openspec` binary: step 1, the configured path, then step
/// 2, each `PATH` entry in order. (Steps 3 and 4 arrive in later groups.) A
/// mis-configured `configured` path falls through to the remaining steps
/// rather than winning or ending the chain, and records exactly one
/// problem naming it — silently substituting a different binary would hide
/// a user's mistake, and refusing to look further would fail closed, which
/// `SPEC.md` forbids. See
/// `openspec/changes/repo-resolution/design.md` -> Contracts.
pub fn openspec_bin(
    configured: Option<&Path>,
    env: &dyn Fn(&str) -> Option<String>,
) -> BinResolution {
    let mut problems = Vec::new();

    if let Some(configured) = configured {
        if is_usable_binary(configured) {
            return BinResolution {
                found: Some(FoundBin {
                    path: configured.to_path_buf(),
                    source: BinSource::Configured,
                }),
                problems,
            };
        }
        problems.push(format!(
            "configured openspec_bin is not usable: {}",
            configured.display()
        ));
    }

    if let Some(path_value) = env("PATH") {
        for candidate in path_candidates(&path_value) {
            if is_usable_binary(&candidate) {
                return BinResolution {
                    found: Some(FoundBin {
                        path: candidate,
                        source: BinSource::Path,
                    }),
                    problems,
                };
            }
        }
    }

    BinResolution {
        found: None,
        problems,
    }
}

#[cfg(test)]
mod tests {
    use crate::testutil::{ScratchDir, canonical, symlink, write_with_mode};
    use std::collections::BTreeMap;
    use std::path::Path;

    /// Build an environment lookup closure over a fixture map — never
    /// `std::env::set_var`, which is `unsafe` in edition 2024 and races
    /// parallel tests. `pairs` maps variable name to value; a name absent
    /// from `pairs` is `None`.
    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        let map: BTreeMap<&str, &str> = pairs.iter().copied().collect();
        move |name| map.get(name).map(|s| s.to_string())
    }

    fn mkdir(path: &Path) {
        std::fs::create_dir_all(path).expect("create fixture directory");
    }

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            mkdir(parent);
        }
        std::fs::write(path, b"not a repository marker").expect("write fixture file");
    }

    #[test]
    fn the_starting_directory_is_itself_the_repository() {
        let scratch = ScratchDir::new();
        let r = scratch.path();
        mkdir(&r.join("openspec"));

        let result = super::find_repo(r);
        assert_eq!(result, super::RepoSearch::Found { root: canonical(r) });
        assert_ne!(
            result,
            super::RepoSearch::Found {
                root: canonical(&r.join("openspec"))
            }
        );
    }

    #[test]
    fn the_repository_is_an_ancestor_several_levels_up() {
        let scratch = ScratchDir::new();
        let r = scratch.path();
        mkdir(&r.join("openspec"));
        let start = r.join("a").join("b").join("c");
        mkdir(&start);

        let result = super::find_repo(&start);
        assert_eq!(result, super::RepoSearch::Found { root: canonical(r) });
    }

    #[test]
    fn the_innermost_repository_wins() {
        let scratch = ScratchDir::new();
        let r = scratch.path();
        mkdir(&r.join("openspec"));
        mkdir(&r.join("inner").join("openspec"));
        let start = r.join("inner").join("deep");
        mkdir(&start);

        let result = super::find_repo(&start);
        let inner_root = super::RepoSearch::Found {
            root: canonical(&r.join("inner")),
        };
        let outer_root = super::RepoSearch::Found { root: canonical(r) };
        assert_eq!(result, inner_root);
        // Discriminating: a walk that runs to the outermost match would produce
        // `outer_root` here, so this assertion must fail against it.
        assert_ne!(result, outer_root);
    }

    #[test]
    fn a_regular_file_named_openspec_is_not_a_repository() {
        let scratch = ScratchDir::new();
        let root = scratch.path();
        let r = root.join("r");
        mkdir(&r);
        touch(&r.join("openspec"));
        // A real directory in the parent, so an `exists()`-based walk (which
        // would accept the file at `r`) is distinguishable from the correct
        // answer rather than accidentally agreeing with it.
        mkdir(&root.join("openspec"));

        let result = super::find_repo(&r);
        assert_eq!(
            result,
            super::RepoSearch::Found {
                root: canonical(root)
            }
        );
    }

    #[test]
    fn a_symbolic_link_to_a_directory_is_a_repository() {
        let scratch = ScratchDir::new();
        let r = scratch.path();
        let target = r.join("elsewhere").join("real-dir");
        mkdir(&target);
        symlink(&target, &r.join("openspec"));

        let result = super::find_repo(r);
        assert_eq!(result, super::RepoSearch::Found { root: canonical(r) });
    }

    #[test]
    fn a_dangling_symbolic_link_named_openspec_is_not_a_repository() {
        let scratch = ScratchDir::new();
        let root = scratch.path();
        let r = root.join("r");
        mkdir(&r);
        symlink(&root.join("does-not-exist"), &r.join("openspec"));
        mkdir(&root.join("openspec"));

        let result = super::find_repo(&r);
        assert_eq!(
            result,
            super::RepoSearch::Found {
                root: canonical(root)
            }
        );
    }

    #[test]
    fn a_starting_path_containing_dotdot_is_resolved_before_the_walk() {
        let scratch = ScratchDir::new();
        let r = scratch.path();
        mkdir(&r.join("openspec"));
        mkdir(&r.join("a"));
        let start = r.join("a").join("..").join("a");

        let result = super::find_repo(&start);
        let expected_root = canonical(r);
        assert_eq!(
            result,
            super::RepoSearch::Found {
                root: expected_root.clone()
            }
        );
        if let super::RepoSearch::Found { root } = result {
            assert!(
                !root.to_string_lossy().contains(".."),
                "root retained a '..' component: {}",
                root.display()
            );
        }
    }

    #[test]
    fn a_starting_path_naming_a_regular_file_is_walked_from_its_parent() {
        let scratch = ScratchDir::new();
        let r = scratch.path();
        mkdir(&r.join("openspec"));
        let file = r.join("a").join("notes.md");
        touch(&file);

        let result = super::find_repo(&file);
        assert_eq!(result, super::RepoSearch::Found { root: canonical(r) });
    }

    #[test]
    fn a_starting_directory_that_does_not_exist_is_not_an_error() {
        let scratch = ScratchDir::new();
        let s = scratch.path();
        let nope = s.join("nope");
        assert!(!nope.exists());

        let result = super::find_repo(&nope);
        assert_eq!(
            result,
            super::RepoSearch::NotFound {
                searched_from: nope.clone()
            }
        );
        assert!(!nope.exists());
    }

    #[test]
    fn a_relative_starting_path_that_cannot_be_resolved_does_not_reach_the_process_working_directory()
     {
        // No fixture: this asserts that a relative, non-existent path never
        // resolves against the process's own working directory, which under
        // `cargo test` is the crate root — itself a repository.
        let start = Path::new("nope/deeper");
        assert!(!start.exists());

        let result = super::find_repo(start);
        assert_eq!(
            result,
            super::RepoSearch::NotFound {
                searched_from: start.to_path_buf()
            }
        );
    }

    #[test]
    fn no_repository_anywhere_up_to_the_filesystem_root() {
        let scratch = ScratchDir::new();
        let s = scratch.path();

        // Precondition: no ancestor of `s`, up to and including `/`, holds an
        // `openspec` directory. A stray `/openspec` or `/tmp/openspec` on some
        // machine must fail this loudly rather than let the test silently pass.
        let canonical_s = canonical(s);
        let mut ancestor = Some(canonical_s.as_path());
        while let Some(a) = ancestor {
            assert!(
                !a.join("openspec").is_dir(),
                "ancestor {} unexpectedly holds an openspec directory; \
                 the fixture assumption for this test is violated on this machine",
                a.display()
            );
            ancestor = a.parent();
        }

        let result = super::find_repo(s);
        assert_eq!(
            result,
            super::RepoSearch::NotFound {
                searched_from: canonical_s
            }
        );
    }

    #[test]
    fn a_repository_tree_is_byte_identical_after_discovery() {
        use crate::testutil::snapshot;

        let scratch = ScratchDir::new();
        let r = scratch.path();
        let x = r.join("openspec").join("changes").join("x");
        mkdir(&x);
        std::fs::write(x.join("tasks.md"), b"- [ ] 1 do it\n").expect("write tasks.md");
        mkdir(&r.join("openspec").join("specs"));
        std::fs::write(r.join("README.md"), b"# Fixture\n").expect("write README.md");

        assert!(
            x.exists(),
            "fixture directory must exist before the call under test"
        );

        let before = snapshot(r);
        let _ = super::find_repo(&x);
        let _ = super::find_repo(&x);
        let after = snapshot(r);
        assert_eq!(before, after);
    }

    #[test]
    fn a_missing_starting_directory_is_not_created() {
        let scratch = ScratchDir::new();
        let s = scratch.path();
        let missing = s.join("a").join("b").join("nope");

        let listing_before: Vec<_> = std::fs::read_dir(s)
            .expect("read scratch dir")
            .map(|e| e.expect("entry").file_name())
            .collect();

        let _ = super::find_repo(&missing);

        assert!(!missing.exists());
        assert!(!s.join("a").exists());
        let listing_after: Vec<_> = std::fs::read_dir(s)
            .expect("read scratch dir")
            .map(|e| e.expect("entry").file_name())
            .collect();
        assert_eq!(listing_before, listing_after);
    }

    // --- group 3: candidate usability, result types, and the PATH step -----

    #[test]
    fn an_executable_regular_file_is_usable() {
        let scratch = ScratchDir::new();
        let d = scratch.path().join("d");
        write_with_mode(&d.join("openspec"), b"#!/bin/sh\n", 0o755);

        let result = super::openspec_bin(None, &env(&[("PATH", &d.display().to_string())]));
        assert_eq!(
            result.found,
            Some(super::FoundBin {
                path: d.join("openspec"),
                source: super::BinSource::Path,
            })
        );
    }

    #[test]
    fn a_file_without_an_execute_bit_is_skipped() {
        let scratch = ScratchDir::new();
        let a = scratch.path().join("a");
        let b = scratch.path().join("b");
        write_with_mode(&a.join("openspec"), b"not executable", 0o644);
        write_with_mode(&b.join("openspec"), b"#!/bin/sh\n", 0o755);

        let path_value = format!("{}:{}", a.display(), b.display());
        let result = super::openspec_bin(None, &env(&[("PATH", &path_value)]));
        assert_eq!(
            result.found,
            Some(super::FoundBin {
                path: b.join("openspec"),
                source: super::BinSource::Path,
            })
        );
    }

    #[test]
    fn a_directory_named_openspec_is_skipped() {
        let scratch = ScratchDir::new();
        let a = scratch.path().join("a");
        let b = scratch.path().join("b");
        mkdir(&a.join("openspec")); // a directory, which carries the execute bit
        write_with_mode(&b.join("openspec"), b"#!/bin/sh\n", 0o755);

        let path_value = format!("{}:{}", a.display(), b.display());
        let result = super::openspec_bin(None, &env(&[("PATH", &path_value)]));
        assert_eq!(
            result.found,
            Some(super::FoundBin {
                path: b.join("openspec"),
                source: super::BinSource::Path,
            })
        );
    }

    #[test]
    fn a_symbolic_link_to_an_executable_file_is_usable_and_is_returned_unresolved() {
        let scratch = ScratchDir::new();
        let a = scratch.path().join("a");
        let t = scratch.path().join("t");
        write_with_mode(&t.join("openspec.js"), b"#!/usr/bin/env node\n", 0o755);
        symlink(&t.join("openspec.js"), &a.join("openspec"));

        let result = super::openspec_bin(None, &env(&[("PATH", &a.display().to_string())]));
        let found = result.found.expect("a binary should be found");
        assert_eq!(found.path, a.join("openspec"));
        assert_ne!(found.path, t.join("openspec.js"));
        assert_eq!(found.source, super::BinSource::Path);
    }

    #[test]
    fn a_dangling_symbolic_link_is_skipped() {
        let scratch = ScratchDir::new();
        let a = scratch.path().join("a");
        let b = scratch.path().join("b");
        symlink(&scratch.path().join("does-not-exist"), &a.join("openspec"));
        write_with_mode(&b.join("openspec"), b"#!/bin/sh\n", 0o755);

        let path_value = format!("{}:{}", a.display(), b.display());
        let result = super::openspec_bin(None, &env(&[("PATH", &path_value)]));
        assert_eq!(
            result.found,
            Some(super::FoundBin {
                path: b.join("openspec"),
                source: super::BinSource::Path,
            })
        );
    }

    #[test]
    fn path_entries_are_searched_left_to_right() {
        let scratch = ScratchDir::new();
        let a = scratch.path().join("a");
        let b = scratch.path().join("b");
        write_with_mode(&a.join("openspec"), b"#!/bin/sh\n", 0o755);
        write_with_mode(&b.join("openspec"), b"#!/bin/sh\n", 0o755);

        let ab = format!("{}:{}", a.display(), b.display());
        let result_ab = super::openspec_bin(None, &env(&[("PATH", &ab)]));
        assert_eq!(result_ab.found.map(|f| f.path), Some(a.join("openspec")));

        // Re-run with the two directories swapped, so the test cannot pass by
        // accident of directory-creation order.
        let ba = format!("{}:{}", b.display(), a.display());
        let result_ba = super::openspec_bin(None, &env(&[("PATH", &ba)]));
        assert_eq!(result_ba.found.map(|f| f.path), Some(b.join("openspec")));
    }

    #[test]
    fn an_empty_or_blank_path_entry_is_not_the_current_directory() {
        let scratch = ScratchDir::new();
        let d = scratch.path().join("d");
        write_with_mode(&d.join("openspec"), b"#!/bin/sh\n", 0o755);

        let path_value = format!(":{}:   :", d.display());

        // The candidate-list half is the load-bearing assertion: exactly one
        // entry, absolute, with no bare relative `openspec` and no candidate
        // under a directory of spaces.
        let candidates = super::path_candidates(&path_value);
        assert_eq!(candidates, vec![d.join("openspec")]);

        let result = super::openspec_bin(None, &env(&[("PATH", &path_value)]));
        assert_eq!(
            result.found,
            Some(super::FoundBin {
                path: d.join("openspec"),
                source: super::BinSource::Path,
            })
        );
    }

    #[test]
    fn a_configured_path_that_does_not_exist_falls_through_to_path() {
        let scratch = ScratchDir::new();
        let g = scratch.path().join("g").join("openspec");
        let d = scratch.path().join("d");
        write_with_mode(&d.join("openspec"), b"#!/bin/sh\n", 0o755);
        assert!(
            !g.exists(),
            "the configured path must not exist for this fixture"
        );

        let result = super::openspec_bin(Some(&g), &env(&[("PATH", &d.display().to_string())]));
        assert_eq!(
            result.found,
            Some(super::FoundBin {
                path: d.join("openspec"),
                source: super::BinSource::Path,
            })
        );
        assert_eq!(result.problems.len(), 1);
        assert!(result.problems[0].contains(&g.display().to_string()));
        assert!(!g.exists());
    }

    #[test]
    fn a_configured_path_that_is_not_executable_falls_through() {
        let scratch = ScratchDir::new();
        let configured = scratch.path().join("configured-openspec");
        write_with_mode(&configured, b"not executable", 0o644);
        let d = scratch.path().join("d");
        write_with_mode(&d.join("openspec"), b"#!/bin/sh\n", 0o755);

        let result = super::openspec_bin(
            Some(&configured),
            &env(&[("PATH", &d.display().to_string())]),
        );
        assert_eq!(
            result.found,
            Some(super::FoundBin {
                path: d.join("openspec"),
                source: super::BinSource::Path,
            })
        );
        assert_eq!(result.problems.len(), 1);
        assert!(result.problems[0].contains(&configured.display().to_string()));
    }

    #[test]
    fn a_configured_path_fails_and_nothing_else_is_found() {
        let scratch = ScratchDir::new();
        let configured = scratch.path().join("nowhere").join("openspec");

        let result = super::openspec_bin(Some(&configured), &env(&[]));
        assert_eq!(result.found, None);
        assert_eq!(result.problems.len(), 1);
        assert!(result.problems[0].contains(&configured.display().to_string()));
    }
}
