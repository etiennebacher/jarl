use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnMode {
    Implicit,
    Explicit,
}

/// <!-- docs: start -->
/// The `mode` option takes a single value (`"implicit"` or `"explicit"`)
/// indicating the preferred return style in functions. If `mode = "implicit"`,
/// then any `return()` call in a terminal position of a function will be
/// reported. If `mode = "explicit"`, then any terminal expression that isn't
/// wrapped in `return()` (or in a function that stops execution, such as
/// `stop()`) will be reported.
///
/// ### Default values
///
/// ```toml
/// mode = "implicit"
/// ```
///
/// ### TOML settings
///
/// ```toml
/// [lint]
/// ...
///
/// [lint.function_return]
/// # Require explicit `return()` calls instead
/// mode = "explicit"
/// ```
/// <!-- docs: end -->
#[derive(Clone, Debug, PartialEq, Eq, Default, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct FunctionReturnOptions {
    pub mode: Option<String>,
}

/// Resolved options for the `function_return` rule.
#[derive(Clone, Debug)]
pub struct ResolvedFunctionReturnOptions {
    pub mode: ReturnMode,
}

impl ResolvedFunctionReturnOptions {
    pub fn resolve(options: Option<&FunctionReturnOptions>) -> anyhow::Result<Self> {
        let mode = match options.and_then(|opts| opts.mode.as_deref()) {
            Some("implicit") | None => ReturnMode::Implicit,
            Some("explicit") => ReturnMode::Explicit,
            Some(other) => {
                return Err(anyhow::anyhow!(
                    "Invalid value for `mode` in `[lint.function_return]`: \"{other}\". \
                     Expected \"implicit\" or \"explicit\"."
                ));
            }
        };

        Ok(Self { mode })
    }
}
