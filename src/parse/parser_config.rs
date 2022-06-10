use std::collections::HashMap;

use super::{skip_take_lines::SkipTakeLines, transform_sanitize_token::*};
use crate::data::value::Value;

pub struct ParserOpts {
    pub separator_char: char,
    pub enclosure_char: Option<char>,
    pub skip_take_lines: Option<Vec<Box<dyn SkipTakeLines>>>,
    pub first_line_is_header: bool,
}

#[derive(Debug, PartialEq)]
pub struct TypeColumnEntry {
    pub header: Option<String>,
    pub target_type: Value,
}

pub struct TransformSanitizeTokens {
    pub transitizers: Vec<Box<dyn TransformSanitizeToken>>,
}

pub struct ParserConfig {
    pub parser_opts: ParserOpts,
    pub sanitize_columns: Option<HashMap<Option<usize>, TransformSanitizeTokens>>,
    pub type_columns: Vec<TypeColumnEntry>,
}
