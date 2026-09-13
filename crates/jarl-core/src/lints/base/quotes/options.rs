use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreferredQuote {
    Double,
    Single,
}

impl PreferredQuote {
    pub const fn as_char(self) -> char {
        match self {
            Self::Double => '"',
            Self::Single => '\'',
        }
    }
}

/// <!-- docs: start -->
/// This takes a single value (`"single"` or `"double"`) indicating the preferred
/// quote style in the files to check. If `quote = "double"` and if the `"quotes"`
/// rule is enabled, then any use of single quotes `'` will be reported, and
/// vice-versa.
/// 
/// Default: `double`
/// 
/// ```toml
/// [lint]
/// ...
/// 
/// [lint.quotes]
/// quote = "single" # or "double"
/// ```
/// <!-- docs: end -->
#[derive(Clone, Debug, PartialEq, Eq, Default, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct QuotesOptions {
    pub quote: Option<String>,
}

/// Resolved options for the `quotes` rule, ready for use during linting.
#[derive(Clone, Debug)]
pub struct ResolvedQuotesOptions {
    pub preferred_delimiter: PreferredQuote,
}

impl ResolvedQuotesOptions {
    pub fn resolve(options: Option<&QuotesOptions>) -> anyhow::Result<Self> {
        let preferred_delimiter = match options {
            Some(opts) => match opts.quote.as_deref() {
                Some("double") | None => PreferredQuote::Double,
                Some("single") => PreferredQuote::Single,
                Some(other) => {
                    return Err(anyhow::anyhow!(
                        "Invalid value for `quote` in `[lint.quotes]`: \"{other}\". \
                         Expected \"double\" or \"single\"."
                    ));
                }
            },
            None => PreferredQuote::Double,
        };

        Ok(Self { preferred_delimiter })
    }
}
