use super::transform_sanitize_token::*;
use crate::data::value::Value;

#[derive(Debug, PartialEq)]
pub struct TypeColumnEntry {
    pub header: Option<String>,
    pub target_type: Value,
}

impl TypeColumnEntry {
    pub fn new(header: Option<String>, target_type: Value) -> Self {
        Self {
            header,
            target_type,
        }
    }
}

pub type TransformSanitizeTokens = Vec<Box<dyn TransformSanitizeToken>>;
