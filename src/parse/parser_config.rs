use super::{transform_sanitize_token::*};
use crate::data::value::Value;

#[derive(Debug, PartialEq)]
pub struct TypeColumnEntry {
    pub header: Option<String>,
    pub target_type: Value,
}

pub struct TransformSanitizeTokens {
    pub transitizers: Vec<Box<dyn TransformSanitizeToken>>,
}