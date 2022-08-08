use venum::value_type::ValueType;

use super::transform_sanitize_token::*;

#[derive(Debug, PartialEq)]
pub struct TypeColumnEntry {
    pub header: Option<String>,
    pub target_type: ValueType,
    pub chrono_pattern: Option<String>,
    pub map_to_none: Option<Vec<String>>,
}

impl TypeColumnEntry {
    pub fn new(header: Option<String>, target_type: ValueType) -> Self {
        Self {
            header,
            target_type,
            chrono_pattern: None,
            map_to_none: None,
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
            map_to_none: None,
        }
    }

    pub fn new_with_map_to_none(
        header: Option<String>,
        target_type: ValueType,
        map_to_none: Vec<String>,
    ) -> Self {
        Self {
            header,
            target_type,
            chrono_pattern: None,
            map_to_none: Some(map_to_none),
        }
    }

    pub fn new_with_chrono_pattern_with_map_to_none(
        header: Option<String>,
        target_type: ValueType,
        chrono_pattern: String,
        map_to_none: Vec<String>,
    ) -> Self {
        Self {
            header,
            target_type,
            chrono_pattern: Some(chrono_pattern),
            map_to_none: Some(map_to_none),
        }
    }
}

pub type TransformSanitizeTokens = Vec<Box<dyn TransformSanitizeToken>>;
