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

/// The root nvm's version trees live under: `NVM_DIR` when it is set to a
/// value that is neither empty nor whitespace-only, and `$HOME/.nvm`
/// otherwise. `None` when neither is available, so the step contributes no
/// candidates rather than panicking on an absent `HOME`.
fn nvm_root(env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    if let Some(dir) = crate::config::non_blank(env("NVM_DIR")) {
        return Some(PathBuf::from(dir));
    }
    let home = crate::config::non_blank(env("HOME"))?;
    Some(PathBuf::from(home).join(".nvm"))
}

/// Parse a directory name as a `vMAJOR.MINOR.PATCH` node version, for
/// ordering purposes only. Anything else — `system`, `iojs-v3.3.1`,
/// `vnightly` — does not parse and is still eligible as a candidate; see
/// `nvm_candidates`.
fn parse_node_version(name: &str) -> Option<(u64, u64, u64)> {
    let rest = name.strip_prefix('v')?;
    let mut parts = rest.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// Step 3's candidate list: `<nvm root>/versions/node/<version>/bin/openspec`
/// for every version directory found, newest first. Parsed versions sort
/// numerically descending (`v10.0.0` before `v9.99.99`, unlike a lexical
/// sort); every unparseable name sorts after every parsed version, and among
/// themselves by name descending — an `nvm alias`-heavy setup should still
/// resolve something rather than nothing. An `Err` reading the versions
/// directory (root cannot be resolved, or does not exist) yields no
/// candidates rather than a panic.
fn nvm_candidates(env: &dyn Fn(&str) -> Option<String>) -> Vec<PathBuf> {
    let Some(root) = nvm_root(env) else {
        return Vec::new();
    };
    let versions_dir = root.join("versions").join("node");
    let Ok(read_dir) = std::fs::read_dir(&versions_dir) else {
        return Vec::new();
    };

    let mut names: Vec<String> = read_dir
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();

    names.sort_by(
        |a, b| match (parse_node_version(a), parse_node_version(b)) {
            (Some(va), Some(vb)) => vb.cmp(&va),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => b.cmp(a),
        },
    );

    names
        .into_iter()
        .map(|name| versions_dir.join(name).join("bin").join("openspec"))
        .collect()
}

/// Step 1's candidate: the configured path, if it is usable. Records no
/// problem itself — only the caller knows whether an unusable configured
/// path here means "fall through and report it" or "not configured at all".
fn step1_configured(configured: Option<&Path>) -> Option<PathBuf> {
    configured
        .filter(|p| is_usable_binary(p))
        .map(|p| p.to_path_buf())
}

/// Step 2's candidate: the first usable `PATH` entry, left to right.
fn step2_path(env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let path_value = env("PATH")?;
    path_candidates(&path_value)
        .into_iter()
        .find(|c| is_usable_binary(c))
}

/// Step 3's candidate: the first usable nvm version-tree binary, newest
/// first.
fn step3_nvm(env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    nvm_candidates(env)
        .into_iter()
        .find(|c| is_usable_binary(c))
}

/// Step 4's candidate: `<npm prefix>/bin/openspec`, if the hook returns a
/// prefix and the result is usable.
fn step4_npm_prefix(npm_prefix: &dyn Fn() -> Option<PathBuf>) -> Option<PathBuf> {
    let prefix = npm_prefix()?;
    let candidate = prefix.join("bin").join("openspec");
    is_usable_binary(&candidate).then_some(candidate)
}

/// Probe for the `openspec` binary in exactly this order, taking the first
/// usable candidate and probing no further: step 1, the configured path;
/// step 2, each `PATH` entry in order; step 3, the nvm version trees, newest
/// first; step 4, `<npm prefix>/bin/openspec`, where the prefix comes from
/// the injected `npm_prefix` hook. A mis-configured `configured` path falls
/// through to the remaining steps rather than winning or ending the chain,
/// and records exactly one problem naming it — silently substituting a
/// different binary would hide a user's mistake, and refusing to look
/// further would fail closed, which `SPEC.md` forbids. When no step
/// produces a usable binary, `found` is `None` and `problems` carries
/// whatever step 1 already produced — never a synthesised "not found"
/// problem, because an absent CLI is a supported state, not a fault. See
/// `openspec/changes/repo-resolution/design.md` -> Contracts.
pub fn openspec_bin(
    configured: Option<&Path>,
    env: &dyn Fn(&str) -> Option<String>,
    npm_prefix: &dyn Fn() -> Option<PathBuf>,
) -> BinResolution {
    let mut problems = Vec::new();

    if let Some(configured) = configured {
        if let Some(path) = step1_configured(Some(configured)) {
            return BinResolution {
                found: Some(FoundBin {
                    path,
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

    if let Some(path) = step2_path(env) {
        return BinResolution {
            found: Some(FoundBin {
                path,
                source: BinSource::Path,
            }),
            problems,
        };
    }

    if let Some(path) = step3_nvm(env) {
        return BinResolution {
            found: Some(FoundBin {
                path,
                source: BinSource::Nvm,
            }),
            problems,
        };
    }

    if let Some(path) = step4_npm_prefix(npm_prefix) {
        return BinResolution {
            found: Some(FoundBin {
                path,
                source: BinSource::NpmPrefix,
            }),
            problems,
        };
    }

    BinResolution {
        found: None,
        problems,
    }
}

/// Step 4's collaborator. Needs the output of `npm prefix -g`, which requires
/// spawning a process; no module in this crate may spawn one outside `cli`,
/// and `cli` does not exist until `subprocess-seam`. This binding therefore
/// returns no prefix, so step 4 contributes nothing in production and no
/// code path here spawns anything. `subprocess-seam` replaces this binding
/// with one that runs `npm prefix -g` behind the seam — and must read its
/// **stdout only**, trimmed: on the reference machine `npm` writes unrelated
/// zsh-plugin noise to stderr, and a non-zero exit or empty output means no
/// prefix.
pub fn npm_prefix_deferred() -> Option<PathBuf> {
    None
}

/// A session-lifetime cache of a single `BinResolution`, owned by the
/// caller rather than process-global: `cargo test` runs the suite in
/// parallel threads of one process, so a `static` cache would be shared by
/// every test and the first probe would decide the answer for all of them.
/// Caches the whole outcome, negative results included, so a re-render
/// never re-walks `PATH`.
#[derive(Default)]
pub struct BinCache(std::sync::OnceLock<BinResolution>);

impl BinCache {
    /// Return the cached resolution, running `probe` only on the first call.
    pub fn get_or_probe(&self, probe: impl FnOnce() -> BinResolution) -> &BinResolution {
        self.0.get_or_init(probe)
    }
}

/// The single composition against the real process environment: the
/// configured value, `config::env_lookup`, and the deferred npm hook, and
/// nothing else — the crate's untestable residue does not grow past this
/// one line. See `openspec/changes/repo-resolution/design.md` -> Contracts.
pub fn openspec_bin_from_env(config: &crate::config::Config) -> BinResolution {
    let env = crate::config::env_lookup();
    openspec_bin(config.openspec_bin.as_deref(), &env, &npm_prefix_deferred)
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

        let result = super::openspec_bin(
            None,
            &env(&[("PATH", &d.display().to_string())]),
            &no_prefix,
        );
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
        let result = super::openspec_bin(None, &env(&[("PATH", &path_value)]), &no_prefix);
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
        let result = super::openspec_bin(None, &env(&[("PATH", &path_value)]), &no_prefix);
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

        let result = super::openspec_bin(
            None,
            &env(&[("PATH", &a.display().to_string())]),
            &no_prefix,
        );
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
        let result = super::openspec_bin(None, &env(&[("PATH", &path_value)]), &no_prefix);
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
        let result_ab = super::openspec_bin(None, &env(&[("PATH", &ab)]), &no_prefix);
        assert_eq!(result_ab.found.map(|f| f.path), Some(a.join("openspec")));

        // Re-run with the two directories swapped, so the test cannot pass by
        // accident of directory-creation order.
        let ba = format!("{}:{}", b.display(), a.display());
        let result_ba = super::openspec_bin(None, &env(&[("PATH", &ba)]), &no_prefix);
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

        let result = super::openspec_bin(None, &env(&[("PATH", &path_value)]), &no_prefix);
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

        let result = super::openspec_bin(
            Some(&g),
            &env(&[("PATH", &d.display().to_string())]),
            &no_prefix,
        );
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
            &no_prefix,
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

        let result = super::openspec_bin(Some(&configured), &env(&[]), &no_prefix);
        assert_eq!(result.found, None);
        assert_eq!(result.problems.len(), 1);
        assert!(result.problems[0].contains(&configured.display().to_string()));
    }

    // --- group 4: the nvm step ----------------------------------------------

    fn nvm_bin(home: &Path, version: &str) -> std::path::PathBuf {
        home.join(".nvm")
            .join("versions")
            .join("node")
            .join(version)
            .join("bin")
            .join("openspec")
    }

    #[test]
    fn the_nvm_tree_is_searched_when_path_has_nothing() {
        let scratch = ScratchDir::new();
        let path_only = scratch.path().join("path-only");
        mkdir(&path_only); // no openspec in it
        let home = scratch.path().join("home");
        let bin = nvm_bin(&home, "v24.20.0");
        write_with_mode(&bin, b"#!/bin/sh\n", 0o755);

        let result = super::openspec_bin(
            None,
            &env(&[
                ("PATH", &path_only.display().to_string()),
                ("HOME", &home.display().to_string()),
            ]),
            &no_prefix,
        );
        assert_eq!(
            result.found,
            Some(super::FoundBin {
                path: bin,
                source: super::BinSource::Nvm,
            })
        );
    }

    #[test]
    fn node_versions_are_ordered_numerically_not_lexically() {
        let scratch = ScratchDir::new();
        let home = scratch.path().join("home");
        write_with_mode(&nvm_bin(&home, "v9.99.99"), b"#!/bin/sh\n", 0o755);
        write_with_mode(&nvm_bin(&home, "v10.0.0"), b"#!/bin/sh\n", 0o755);

        let result = super::openspec_bin(
            None,
            &env(&[("HOME", &home.display().to_string())]),
            &no_prefix,
        );
        assert_eq!(
            result.found.map(|f| f.path),
            Some(nvm_bin(&home, "v10.0.0"))
        );
    }

    #[test]
    fn a_version_directory_without_a_usable_binary_is_skipped() {
        let scratch = ScratchDir::new();
        let home = scratch.path().join("home");
        mkdir(
            &home
                .join(".nvm")
                .join("versions")
                .join("node")
                .join("v22.0.0"),
        ); // no bin/ at all
        write_with_mode(&nvm_bin(&home, "v21.0.0"), b"not executable", 0o644);
        write_with_mode(&nvm_bin(&home, "v20.0.0"), b"#!/bin/sh\n", 0o755);

        let result = super::openspec_bin(
            None,
            &env(&[("HOME", &home.display().to_string())]),
            &no_prefix,
        );
        assert_eq!(
            result.found.map(|f| f.path),
            Some(nvm_bin(&home, "v20.0.0"))
        );
    }

    #[test]
    fn a_version_directory_whose_name_is_not_a_version_is_still_eligible_and_sorts_last() {
        let scratch1 = ScratchDir::new();
        let home1 = scratch1.path().join("home");
        write_with_mode(&nvm_bin(&home1, "system"), b"#!/bin/sh\n", 0o755);

        let result1 = super::openspec_bin(
            None,
            &env(&[("HOME", &home1.display().to_string())]),
            &no_prefix,
        );
        assert_eq!(
            result1.found,
            Some(super::FoundBin {
                path: nvm_bin(&home1, "system"),
                source: super::BinSource::Nvm,
            })
        );

        // A parsed version outranks an unparseable name. `vnightly` is the
        // fixture, not `system`: plain name-descending sorts `vnightly` above
        // `v20.0.0` and `system` below it, so only `vnightly` discriminates
        // against the wrong ordering.
        let scratch2 = ScratchDir::new();
        let home2 = scratch2.path().join("home");
        write_with_mode(&nvm_bin(&home2, "vnightly"), b"#!/bin/sh\n", 0o755);
        write_with_mode(&nvm_bin(&home2, "v20.0.0"), b"#!/bin/sh\n", 0o755);

        let result2 = super::openspec_bin(
            None,
            &env(&[("HOME", &home2.display().to_string())]),
            &no_prefix,
        );
        assert_eq!(
            result2.found.map(|f| f.path),
            Some(nvm_bin(&home2, "v20.0.0"))
        );
    }

    #[test]
    fn nvm_dir_overrides_the_default_nvm_root() {
        let scratch = ScratchDir::new();
        let nvm_dir = scratch.path().join("nvmdir");
        let nvm_dir_bin = nvm_dir
            .join("versions")
            .join("node")
            .join("v20.0.0")
            .join("bin")
            .join("openspec");
        write_with_mode(&nvm_dir_bin, b"#!/bin/sh\n", 0o755);

        let home = scratch.path().join("home");
        let home_bin = nvm_bin(&home, "v20.0.0");
        write_with_mode(&home_bin, b"#!/bin/sh\n", 0o755);

        let nvm_dir_str = nvm_dir.display().to_string();
        let home_str = home.display().to_string();

        let both_pairs = [
            ("NVM_DIR", nvm_dir_str.as_str()),
            ("HOME", home_str.as_str()),
        ];
        let with_both = env(&both_pairs);
        let result = super::openspec_bin(None, &with_both, &no_prefix);
        assert_eq!(result.found.map(|f| f.path), Some(nvm_dir_bin.clone()));

        // A blank NVM_DIR falls through to the default HOME-based root.
        let blank_pairs = [("NVM_DIR", "   "), ("HOME", home_str.as_str())];
        let with_blank = env(&blank_pairs);
        let result = super::openspec_bin(None, &with_blank, &no_prefix);
        assert_eq!(result.found.map(|f| f.path), Some(home_bin));

        // Neither variable available: no binary and no panic.
        let with_neither = env(&[]);
        let result = super::openspec_bin(None, &with_neither, &no_prefix);
        assert_eq!(result.found, None);
    }

    // --- group 5: the npm-prefix step, the deferred hook, and the chain ----

    fn no_prefix() -> Option<std::path::PathBuf> {
        None
    }

    #[test]
    fn the_configured_path_wins_over_every_other_source() {
        let scratch = ScratchDir::new();
        let c = scratch.path().join("c");
        let configured = c.join("openspec");
        write_with_mode(&configured, b"#!/bin/sh\n", 0o755);

        let d = scratch.path().join("d");
        write_with_mode(&d.join("openspec"), b"#!/bin/sh\n", 0o755);

        let home = scratch.path().join("home");
        write_with_mode(&nvm_bin(&home, "v20.0.0"), b"#!/bin/sh\n", 0o755);

        let d_str = d.display().to_string();
        let home_str = home.display().to_string();
        let pairs = [("PATH", d_str.as_str()), ("HOME", home_str.as_str())];
        let lookup = env(&pairs);

        let result = super::openspec_bin(Some(&configured), &lookup, &no_prefix);
        assert_eq!(
            result.found,
            Some(super::FoundBin {
                path: configured.clone(),
                source: super::BinSource::Configured,
            })
        );
        assert!(result.problems.is_empty());

        // Dropping the configured argument yields the PATH answer, so the
        // ordering itself is what is under test.
        let dropped = super::openspec_bin(None, &lookup, &no_prefix);
        assert_eq!(
            dropped.found,
            Some(super::FoundBin {
                path: d.join("openspec"),
                source: super::BinSource::Path,
            })
        );
    }

    #[test]
    fn path_wins_when_nothing_is_configured() {
        let scratch = ScratchDir::new();
        let d = scratch.path().join("d");
        write_with_mode(&d.join("openspec"), b"#!/bin/sh\n", 0o755);
        let home = scratch.path().join("home");
        write_with_mode(&nvm_bin(&home, "v20.0.0"), b"#!/bin/sh\n", 0o755);

        let d_str = d.display().to_string();
        let home_str = home.display().to_string();
        let pairs = [("PATH", d_str.as_str()), ("HOME", home_str.as_str())];
        let lookup = env(&pairs);
        let result = super::openspec_bin(None, &lookup, &no_prefix);
        assert_eq!(
            result.found,
            Some(super::FoundBin {
                path: d.join("openspec"),
                source: super::BinSource::Path,
            })
        );
    }

    #[test]
    fn an_absent_or_blank_path_contributes_nothing() {
        let scratch = ScratchDir::new();
        let n = scratch.path().join("n");
        write_with_mode(&n.join("bin").join("openspec"), b"#!/bin/sh\n", 0o755);
        let hook = || Some(n.clone());

        for path_value in [None, Some(""), Some("   ")] {
            let pairs: Vec<(&str, &str)> = match path_value {
                Some(v) => vec![("PATH", v)],
                None => vec![],
            };
            let lookup = env(&pairs);
            let result = super::openspec_bin(None, &lookup, &hook);
            assert_eq!(
                result.found,
                Some(super::FoundBin {
                    path: n.join("bin").join("openspec"),
                    source: super::BinSource::NpmPrefix,
                }),
                "PATH value {path_value:?} should have fallen through to the npm prefix"
            );
        }
    }

    #[test]
    fn the_nvm_tree_outranks_the_npm_prefix() {
        let scratch = ScratchDir::new();
        let path_only = scratch.path().join("path-only");
        mkdir(&path_only);
        let home = scratch.path().join("home");
        write_with_mode(&nvm_bin(&home, "v20.0.0"), b"#!/bin/sh\n", 0o755);
        let n = scratch.path().join("n");
        write_with_mode(&n.join("bin").join("openspec"), b"#!/bin/sh\n", 0o755);
        let hook = || Some(n.clone());

        let path_only_str = path_only.display().to_string();
        let home_str = home.display().to_string();
        let pairs = [
            ("PATH", path_only_str.as_str()),
            ("HOME", home_str.as_str()),
        ];
        let lookup = env(&pairs);
        let result = super::openspec_bin(None, &lookup, &hook);
        assert_eq!(
            result.found,
            Some(super::FoundBin {
                path: nvm_bin(&home, "v20.0.0"),
                source: super::BinSource::Nvm,
            })
        );
    }

    #[test]
    fn the_npm_prefix_is_the_last_resort() {
        let scratch = ScratchDir::new();
        let path_only = scratch.path().join("path-only");
        mkdir(&path_only);
        let n = scratch.path().join("n");
        write_with_mode(&n.join("bin").join("openspec"), b"#!/bin/sh\n", 0o755);
        let hook = || Some(n.clone());

        // No configured, no PATH match, and — deliberately — no HOME/NVM_DIR
        // at all, so the nvm step contributes nothing.
        let path_only_str = path_only.display().to_string();
        let pairs = [("PATH", path_only_str.as_str())];
        let lookup = env(&pairs);
        let result = super::openspec_bin(None, &lookup, &hook);
        assert_eq!(
            result.found,
            Some(super::FoundBin {
                path: n.join("bin").join("openspec"),
                source: super::BinSource::NpmPrefix,
            })
        );
    }

    #[test]
    fn an_npm_prefix_without_a_usable_binary_yields_nothing() {
        let scratch = ScratchDir::new();
        let n = scratch.path().join("n");
        mkdir(&n); // no bin/openspec inside
        let hook = || Some(n.clone());

        let lookup = env(&[]);
        let result = super::openspec_bin(None, &lookup, &hook);
        assert_eq!(result.found, None);
        assert!(result.problems.is_empty());
    }

    #[test]
    fn nothing_anywhere_is_a_supported_state_not_a_fault() {
        let lookup = env(&[]);
        let result = super::openspec_bin(None, &lookup, &no_prefix);
        assert_eq!(result.found, None);
        assert!(result.problems.is_empty());
    }

    #[test]
    fn the_shipped_hook_yields_no_prefix() {
        assert_eq!(
            super::npm_prefix_deferred(),
            None,
            "this is expected to go RED the day subprocess-seam wires npm prefix -g \
             through this hook — that failure is the intended hand-over signal, not \
             a regression"
        );
    }

    // --- group 6: session cache and the one real-environment composition ---

    #[test]
    fn a_second_lookup_does_not_re_probe() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let counter = AtomicUsize::new(0);
        let scratch = ScratchDir::new();
        let bin = scratch.path().join("openspec");
        write_with_mode(&bin, b"#!/bin/sh\n", 0o755);
        let bin_for_probe = bin.clone();

        let probe = || {
            counter.fetch_add(1, Ordering::Relaxed);
            super::BinResolution {
                found: Some(super::FoundBin {
                    path: bin_for_probe.clone(),
                    source: super::BinSource::Path,
                }),
                problems: Vec::new(),
            }
        };

        let cache = super::BinCache::default();
        let first = cache.get_or_probe(probe).clone();
        let second = cache.get_or_probe(probe).clone();

        assert_eq!(counter.load(Ordering::Relaxed), 1);
        assert_eq!(first, second);
    }

    #[test]
    fn a_negative_result_is_cached_too() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let counter = AtomicUsize::new(0);
        let probe = || {
            counter.fetch_add(1, Ordering::Relaxed);
            super::BinResolution {
                found: None,
                problems: Vec::new(),
            }
        };

        let cache = super::BinCache::default();
        let first = cache.get_or_probe(probe);
        assert_eq!(first.found, None);
        let second = cache.get_or_probe(probe);
        assert_eq!(second.found, None);

        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn two_caches_are_independent() {
        // A `static` cache would fail this: both values would share one
        // underlying `OnceLock`, and the first probe would decide the answer
        // for both. This is the reason the cache is a value, not global.
        let a = super::BinCache::default();
        let b = super::BinCache::default();

        let resolution_a = super::BinResolution {
            found: Some(super::FoundBin {
                path: std::path::PathBuf::from("/a/openspec"),
                source: super::BinSource::Path,
            }),
            problems: Vec::new(),
        };
        let resolution_b = super::BinResolution {
            found: Some(super::FoundBin {
                path: std::path::PathBuf::from("/b/openspec"),
                source: super::BinSource::Path,
            }),
            problems: Vec::new(),
        };

        let got_a = a.get_or_probe(|| resolution_a.clone());
        let got_b = b.get_or_probe(|| resolution_b.clone());

        assert_eq!(got_a, &resolution_a);
        assert_eq!(got_b, &resolution_b);
        assert_ne!(got_a, got_b);
    }

    #[test]
    fn the_composition_honours_a_configured_binary() {
        let scratch = ScratchDir::new();
        let configured = scratch.path().join("openspec");
        write_with_mode(&configured, b"#!/bin/sh\n", 0o755);

        let config = crate::config::Config {
            openspec_bin: Some(configured.clone()),
            ..crate::config::Config::default()
        };

        let result = super::openspec_bin_from_env(&config);
        assert_eq!(
            result.found,
            Some(super::FoundBin {
                path: configured,
                source: super::BinSource::Configured,
            })
        );
    }
}
