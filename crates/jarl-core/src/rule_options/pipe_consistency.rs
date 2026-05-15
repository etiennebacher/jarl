use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreferredPipe {
    Base,
    Magrittr,
}

/// TOML options for `[lint.pipe_consistency]`.
///
/// Use `pipe` to specify which pipe operator to enforce. Valid values
/// are `"|>"` (the default) and `"%>%"`.
#[derive(Clone, Debug, PartialEq, Eq, Default, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PipeConsistencyOptions {
    pub pipe: Option<String>,
}

/// Resolved options for the `pipe_consistency` rule.
#[derive(Clone, Debug)]
pub struct ResolvedPipeConsistencyOptions {
    pub pipe: PreferredPipe,
}

impl ResolvedPipeConsistencyOptions {
    pub fn resolve(options: Option<&PipeConsistencyOptions>) -> anyhow::Result<Self> {
        let pipe = match options {
            Some(opts) => match opts.pipe.as_deref() {
                Some("|>") | None => PreferredPipe::Base,
                Some("%>%") => PreferredPipe::Magrittr,
                Some(other) => {
                    return Err(anyhow::anyhow!(
                        "Invalid value for `pipe` in `[lint.pipe_consistency]`: \"{other}\". \
                         Expected \"|>\" or \"%>%\"."
                    ));
                }
            },
            None => PreferredPipe::Base,
        };

        Ok(Self { pipe })
    }
}
