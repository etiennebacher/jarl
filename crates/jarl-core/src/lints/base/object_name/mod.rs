pub(crate) mod object_name;
pub(crate) mod options;

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::lints::base::object_name::options::{ObjectNameOptions, ResolvedObjectNameOptions};
    use crate::rule_options::ResolvedRuleOptions;
    use crate::settings::{LinterSettings, Settings};
    use crate::utils_test::check_code;

    fn reported_names(code: &str, settings: Option<Settings>) -> Vec<String> {
        let diagnostics =
            crate::utils_test::check_code_with_settings(code, "object_name", None, settings);
        diagnostics
            .iter()
            .map(|diagnostic| {
                let start: usize = diagnostic.range.start().into();
                let end: usize = diagnostic.range.end().into();
                code[start..end].to_string()
            })
            .collect()
    }

    fn settings_with_options(options: ObjectNameOptions) -> Settings {
        Settings {
            linter: LinterSettings {
                rule_options: ResolvedRuleOptions {
                    object_name: ResolvedObjectNameOptions::resolve(Some(&options)).unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    #[test]
    fn reports_assignment_targets_and_formals() {
        let code = "badName <- 1\nf <- function(badArg, good_name, ...) badArg";
        assert_eq!(reported_names(code, None), ["badName", "badArg"]);
    }

    #[test]
    fn reports_extract_assignment_roots() {
        let code = "badName$member <- 1\nbadName@slot <- 1\n1 -> badName$member";
        assert_eq!(
            reported_names(code, None),
            ["badName", "badName", "badName"]
        );
    }

    #[test]
    fn distinguishes_named_arguments() {
        assert!(reported_names("call(badName = 1)", None).is_empty());
    }

    #[test]
    fn supports_custom_styles_and_regexes() {
        let styles = ObjectNameOptions {
            styles: Some(vec!["CamelCase".to_string()]),
            ..Default::default()
        };
        assert_eq!(
            reported_names(
                "GoodName <- 1\nbadName <- 1",
                Some(settings_with_options(styles))
            ),
            ["badName"]
        );

        let regexes = ObjectNameOptions {
            styles: Some(Vec::new()),
            regexes: Some(BTreeMap::from([(
                "prefixed".to_string(),
                r"^x_[a-z]+$".to_string(),
            )])),
            ..Default::default()
        };
        assert_eq!(
            reported_names(
                "x_good <- 1\ngood_name <- 1",
                Some(settings_with_options(regexes)),
            ),
            ["good_name"]
        );
    }

    #[test]
    fn accepts_default_symbol_names() {
        assert!(check_code("%>% <- function(x) x", "object_name", None).is_empty());
    }

    #[test]
    fn accepts_default_special_names() {
        let code = concat!(
            ".onLoad <- function(...) TRUE\n",
            ".onAttach <- function(...) TRUE\n",
            ".onUnload <- function(...) TRUE\n",
            ".onDetach <- function(...) TRUE\n",
            ".Last.lib <- function(...) TRUE\n",
            ".First <- function(...) TRUE\n",
            ".Last <- function(...) TRUE\n",
            "badName <- 1",
        );

        assert_eq!(reported_names(code, None), ["badName"]);
    }

    #[test]
    fn extends_special_names() {
        let options = ObjectNameOptions {
            extend_special_names: Some(vec!["mySpecial".to_string()]),
            ..Default::default()
        };

        let code = ".onLoad <- function(...) TRUE\nmySpecial <- 1\nbadName <- 1";

        assert_eq!(
            reported_names(code, Some(settings_with_options(options))),
            ["badName"]
        );
    }

    #[test]
    fn replaces_special_names() {
        let options = ObjectNameOptions {
            special_names: Some(vec!["mySpecial".to_string()]),
            ..Default::default()
        };

        let code = ".onLoad <- function(...) TRUE\nmySpecial <- 1";

        assert_eq!(
            reported_names(code, Some(settings_with_options(options))),
            [".onLoad"]
        );
    }

    #[test]
    fn rejects_both_special_name_lists() {
        let options = ObjectNameOptions {
            special_names: Some(vec!["mySpecial".to_string()]),
            extend_special_names: Some(vec!["anotherSpecial".to_string()]),
            ..Default::default()
        };

        let error = ResolvedObjectNameOptions::resolve(Some(&options)).unwrap_err();

        assert_eq!(
            error.to_string(),
            "Cannot specify both `special-names` and `extend-special-names` in `[lint.object_name]`."
        );
    }
}
