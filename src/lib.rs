// Group 2 replaces this body wholesale with the invocation classifier.

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
