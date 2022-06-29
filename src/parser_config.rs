use venum::venum::Value;

use super::transform_sanitize_token::*;

#[derive(Debug, PartialEq)]
pub struct TypeColumnEntry {
    pub header: Option<String>,
    pub target_type: Value,
    pub chrono_pattern: Option<String>,
}

impl TypeColumnEntry {
    pub fn new(header: Option<String>, target_type: Value) -> Self {
        Self {
            header,
            target_type,
            chrono_pattern: None,
        }
    }

    pub fn new_with_chrono_pattern(
        header: Option<String>,
        target_type: Value,
        chrono_pattern: String,
    ) -> Self {
        Self {
            header,
            target_type,
            chrono_pattern: Some(chrono_pattern),
        }
    }
}

pub type TransformSanitizeTokens = Vec<Box<dyn TransformSanitizeToken>>;
