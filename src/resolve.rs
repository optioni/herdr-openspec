//! Repository discovery and `openspec` binary resolution.
//!
//! See `openspec/changes/repo-resolution/design.md` for the full contract.

#[cfg(test)]
mod tests {
    use crate::testutil::{ScratchDir, canonical, symlink};
    use std::path::Path;

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
}
