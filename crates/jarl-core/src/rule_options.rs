use std::collections::HashSet;

use crate::toml::LinterTomlOptions;

/// Resolve a pair of `field` / `extend-field` options against a set of defaults.
///
/// - If both are `Some`, returns an error.
/// - If `base` is `Some`, uses it as the full replacement.
/// - If `extend` is `Some`, merges it with the defaults.
/// - If neither is set, returns the defaults.
///
/// `rule_section` and `field_name` are used for the error message, e.g.
/// `"duplicated_arguments"` and `"skipped-functions"`.
pub fn resolve_with_extend(
    base: Option<&Vec<String>>,
    extend: Option<&Vec<String>>,
    defaults: &[&str],
    rule_section: &str,
    field_name: &str,
) -> anyhow::Result<HashSet<String>> {
    if base.is_some() && extend.is_some() {
        return Err(anyhow::anyhow!(
            "Cannot specify both `{field_name}` and `extend-{field_name}` \
             in `[lint.{rule_section}]`."
        ));
    }

    let default_set: HashSet<String> = defaults.iter().map(|s| (*s).to_string()).collect();

    if let Some(values) = base {
        Ok(values.iter().cloned().collect())
    } else if let Some(values) = extend {
        let mut set = default_set;
        set.extend(values.iter().cloned());
        Ok(set)
    } else {
        Ok(default_set)
    }
}

/// Declare the per-rule options that jarl resolves from `[lint.<rule>]`.
///
/// Each entry names the rule's directory (`<group>::<rule>`) and its resolved
/// options type, which must live in `lints/<group>/<rule>/options.rs` and
/// expose `resolve(Option<&T>) -> anyhow::Result<Self>`, where `T` is the type
/// of the matching `LinterTomlOptions` field.
///
/// From that single line the macro generates the [ResolvedRuleOptions] field,
/// its resolution from the parsed TOML, and the [Default] impl.
macro_rules! declare_rule_options {
    ($($group:ident :: $rule:ident => $resolved:ident),* $(,)?) => {
        /// Resolved per-rule options, ready for use during linting.
        ///
        /// To add options for a new rule:
        /// 1. Create `lints/<group>/<rule_name>/options.rs` with the TOML and
        ///    resolved types, and declare `pub(crate) mod options;` in the
        ///    rule's `mod.rs`.
        /// 2. Add the TOML field (named after the rule) to `LinterTomlOptions`
        ///    in `toml.rs`.
        /// 3. Add a line to the `declare_rule_options!` invocation in this
        ///    file.
        #[derive(Clone, Debug)]
        pub struct ResolvedRuleOptions {
            $(
                pub $rule: crate::lints::$group::$rule::options::$resolved,
            )*
        }

        impl ResolvedRuleOptions {
            /// Resolve every `[lint.<rule>]` table, filling in defaults for the
            /// ones the user didn't set.
            pub fn resolve(options: &LinterTomlOptions) -> anyhow::Result<Self> {
                Ok(Self {
                    $(
                        $rule: crate::lints::$group::$rule::options::$resolved::resolve(
                            options.$rule.as_ref(),
                        )?,
                    )*
                })
            }
        }

        impl Default for ResolvedRuleOptions {
            fn default() -> Self {
                Self::resolve(&LinterTomlOptions::default())
                    .expect("default rule options should always resolve")
            }
        }
    };
}

declare_rule_options! {
    base::assignment => ResolvedAssignmentOptions,
    base::duplicated_arguments => ResolvedDuplicatedArgumentsOptions,
    base::if_not_else => ResolvedIfNotElseOptions,
    base::implicit_assignment => ResolvedImplicitAssignmentOptions,
    base::missing_argument => ResolvedMissingArgumentOptions,
    base::nested_pipe => ResolvedNestedPipeOptions,
    base::pipe_consistency => ResolvedPipeConsistencyOptions,
    base::quotes => ResolvedQuotesOptions,
    base::true_false_symbol => ResolvedTrueFalseSymbolOptions,
    base::undesirable_function => ResolvedUndesirableFunctionOptions,
    base::unreachable_code => ResolvedUnreachableCodeOptions,
    base::unused_function => ResolvedUnusedFunctionOptions,
    base::unused_object => ResolvedUnusedObjectOptions,
}
