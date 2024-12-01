use crate::{lang::Signal, parser::*, session::SessionParserConfig};

pub type LineColResult = Result<Vec<(usize, usize)>, Signal>;
pub trait LocalizedParser: std::marker::Sync {
    fn parse_input_with(&self, input: &str, config: &SessionParserConfig) -> ParseResult;
    fn parse_input(&self, input: &str) -> ParseResult {
        self.parse_input_with(input, &SessionParserConfig::default())
    }
    fn parse_line_col(&self, input: &str) -> LineColResult;
}

#[cfg_attr(
    target_family = "wasm",
    wasm_bindgen::prelude::wasm_bindgen,
    derive(Serialize, Deserialize),
    serde(rename_all(serialize = "kebab-case", deserialize = "kebab-case"))
)]
#[derive(Debug, Copy, Clone, Default, PartialEq, strum::EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum Localization {
    // ISO-639 codes
    #[default]
    En,
}

impl LocalizedParser for Localization {
    fn parse_input_with(&self, input: &str, config: &SessionParserConfig) -> ParseResult {
        use Localization::*;
        match self {
            En => LocalizedParser::parse_input_with(&en::Parser, input, config),
        }
    }
    fn parse_line_col(&self, input: &str) -> LineColResult {
        use Localization::*;
        match self {
            En => LocalizedParser::parse_line_col(&en::Parser, input),
        }
    }
}

impl LocalizedParser for SessionParserConfig {
    fn parse_input_with(&self, _input: &str, _config: &SessionParserConfig) -> ParseResult {
        unimplemented!()
    }

    fn parse_input(&self, input: &str) -> ParseResult {
        use Localization::*;
        match self.locale {
            En => LocalizedParser::parse_input_with(&en::Parser, input, self),
        }
    }

    fn parse_line_col(&self, input: &str) -> LineColResult {
        use Localization::*;
        match self.locale {
            En => LocalizedParser::parse_line_col(&en::Parser, input),
        }
    }
}
