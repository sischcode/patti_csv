use super::transform_sanitize_token::*;
use crate::data::csv_value::CsvValue;

#[derive(Debug, PartialEq)]
pub struct TypeColumnEntry {
    pub header: Option<String>,
    pub target_type: CsvValue,
}

impl TypeColumnEntry {
    pub fn new(header: Option<String>, target_type: CsvValue) -> Self {
        Self {
            header,
            target_type,
        }
    }
}

pub type TransformSanitizeTokens = Vec<Box<dyn TransformSanitizeToken>>;
