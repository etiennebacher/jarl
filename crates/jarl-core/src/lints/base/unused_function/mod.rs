pub(crate) mod options;
pub(crate) mod unused_function;

#[cfg(test)]
mod tests {
    use crate::namespace::Namespace;

    // ── Namespace ─────────────────────────────────────────

    #[test]
    fn test_parse_simple_exports() {
        let content = "export(foo)\nexport(bar)\n";
        let exports = Namespace::parse(content).exported([]);
        assert!(exports.contains("foo"));
        assert!(exports.contains("bar"));
        assert_eq!(exports.len(), 2);
    }

    #[test]
    fn test_parse_multi_export() {
        let content = "export(foo, bar, baz)\n";
        let exports = Namespace::parse(content).exported([]);
        assert!(exports.contains("foo"));
        assert!(exports.contains("bar"));
        assert!(exports.contains("baz"));
        assert_eq!(exports.len(), 3);
    }

    #[test]
    fn test_parse_export_pattern() {
        let content = "exportPattern(\"^[[:alpha:]]\")\n";
        let all_names = ["foo", "bar", ".hidden"];
        let exports = Namespace::parse(content).exported(all_names.iter().copied());
        assert!(exports.contains("foo"));
        assert!(exports.contains("bar"));
        assert!(!exports.contains(".hidden"));
    }

    #[test]
    fn test_parse_s3method() {
        let content = "S3method(print, myclass)\n";
        let exports = Namespace::parse(content).exported([]);
        assert!(exports.contains("print.myclass"));
    }

    #[test]
    fn test_parse_comments_ignored() {
        let content = "# export(secret)\nexport(public)\n";
        let exports = Namespace::parse(content).exported([]);
        assert!(!exports.contains("secret"));
        assert!(exports.contains("public"));
    }

    #[test]
    fn test_parse_s3method_namespaced_generic() {
        let content = "S3method(pkg::record_print, tinytable)\n";
        let exports = Namespace::parse(content).exported([]);
        assert!(exports.contains("record_print.tinytable"));
    }

    #[test]
    fn test_parse_s3method_triple_colon() {
        let content = "S3method(pkg:::record_print, tinytable)\n";
        let exports = Namespace::parse(content).exported([]);
        assert!(exports.contains("record_print.tinytable"));
    }

    #[test]
    fn test_parse_s3method_with_explicit_method() {
        let content = "S3method(print, myclass, print_myclass_impl)\n";
        let exports = Namespace::parse(content).exported([]);
        assert!(exports.contains("print_myclass_impl"));
        assert!(!exports.contains("print.myclass"));
    }

    #[test]
    fn test_parse_s3method_with_if_guard() {
        let content = "if (getRversion() >= \"4.4.0\") S3method(sort_by, data.table)\n";
        let exports = Namespace::parse(content).exported([]);
        assert!(
            exports.contains("sort_by.data.table"),
            "expected sort_by.data.table, got: {:?}",
            exports
        );
    }

    #[test]
    fn test_parse_export_with_if_guard() {
        let content = "if (getRversion() >= \"4.4.0\") export(sort_by)\n";
        let exports = Namespace::parse(content).exported([]);
        assert!(exports.contains("sort_by"));
    }

    // ── ResolvedUnusedFunctionOptions ────────────────────────────────────

    use crate::lints::base::unused_function::options::{
        ResolvedUnusedFunctionOptions, UnusedFunctionOptions,
    };

    #[test]
    fn test_threshold_ignore_default_is_50() {
        let resolved = ResolvedUnusedFunctionOptions::resolve(None).unwrap();
        assert_eq!(resolved.threshold_ignore, 50);
    }

    #[test]
    fn test_threshold_ignore_custom_value() {
        let opts = UnusedFunctionOptions { threshold_ignore: Some(10), ..Default::default() };
        let resolved = ResolvedUnusedFunctionOptions::resolve(Some(&opts)).unwrap();
        assert_eq!(resolved.threshold_ignore, 10);
    }

    #[test]
    fn test_threshold_ignore_none_uses_default() {
        let opts = UnusedFunctionOptions { threshold_ignore: None, ..Default::default() };
        let resolved = ResolvedUnusedFunctionOptions::resolve(Some(&opts)).unwrap();
        assert_eq!(resolved.threshold_ignore, 50);
    }

    #[test]
    fn test_threshold_ignore_zero() {
        let opts = UnusedFunctionOptions { threshold_ignore: Some(0), ..Default::default() };
        let resolved = ResolvedUnusedFunctionOptions::resolve(Some(&opts)).unwrap();
        assert_eq!(resolved.threshold_ignore, 0);
    }

    // ── skipped_functions ──────────────────────────────────────────────

    #[test]
    fn test_skipped_functions_invalid_regex() {
        let opts = UnusedFunctionOptions {
            skipped_functions: Some(vec!["^pl__".to_string(), "[invalid".to_string()]),
            ..Default::default()
        };
        let err = ResolvedUnusedFunctionOptions::resolve(Some(&opts)).unwrap_err();
        assert!(
            err.to_string().contains("[invalid"),
            "error should mention the bad pattern: {err}"
        );
    }
}
