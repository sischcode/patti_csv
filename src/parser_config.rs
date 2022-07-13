use venum::venum::ValueType;

use super::transform_sanitize_token::*;

#[derive(Debug, PartialEq)]
pub struct TypeColumnEntry {
    pub header: Option<String>,
    pub target_type: ValueType,
    pub chrono_pattern: Option<String>,
}

impl TypeColumnEntry {
    pub fn new(header: Option<String>, target_type: ValueType) -> Self {
        Self {
            header,
            target_type,
            chrono_pattern: None,
        }
    }

    pub fn new_with_chrono_pattern(
        header: Option<String>,
        target_type: ValueType,
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
