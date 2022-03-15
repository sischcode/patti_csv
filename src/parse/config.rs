use std::collections::HashMap;

use super::{skip_take_file_lines::*, transform_sanitize_token::*};
use crate::data::value::CsvValue;

pub struct ParserOptLines {
    pub transitizers: Vec<Box<dyn SkipTakeFileLines>>,
}

pub struct ParserOpts {
    pub separator_char: char,
    pub enclosure_char: Option<char>,
    pub lines: Option<ParserOptLines>,
    pub first_line_is_header: bool,
}

#[derive(Debug, PartialEq)]
pub struct TypeColumnEntry {
    pub header: Option<String>,
    pub target_type: CsvValue,
}

pub struct TransformSanitizeTokens {
    pub transitizers: Vec<Box<dyn TransformSanitizeToken>>,
}

pub struct DsvParserConfig {
    pub parser_opts: ParserOpts,
    pub sanitize_columns: Option<HashMap<Option<usize>, TransformSanitizeTokens>>,
    pub type_columns: Vec<TypeColumnEntry>,
}
