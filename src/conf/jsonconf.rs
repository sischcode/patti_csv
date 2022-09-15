use serde::Deserialize;
use venum::value_type::ValueType;

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigRoot {
    pub comment: Option<String>,
    pub parser_opts: ParserOpts,
    pub sanitize_columns: Option<Vec<SanitizeColumnsEntry>>,
    pub type_columns: Option<Vec<TypeColumnsEntry>>,
}

/// If skip and take options are present, the take filter overrules the skip filter.
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "Lines", rename_all = "camelCase")]
pub struct ParserOptLines {
    pub comment: Option<String>,
    pub skip_lines_from_start: Option<usize>,
    pub skip_lines_by_startswith: Option<Vec<String>>,
    pub skip_lines_by_regex: Option<Vec<String>>,
    pub skip_empty_lines: Option<bool>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "ParserOpts", rename_all = "camelCase")]
pub struct ParserOpts {
    pub comment: Option<String>,
    pub separator_char: char,
    pub enclosure_char: Option<char>,
    pub lines: Option<ParserOptLines>,
    pub first_line_is_header: bool,
    pub save_skipped_lines: bool,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TrimOpts {
    All,
    Leading,
    Trailing,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CasingOpts {
    ToLower,
    ToUpper,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceColumnSanitizerEntry {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SanitizeColumnOpts {
    Trim {
        spec: TrimOpts,
    },
    Casing {
        spec: CasingOpts,
    },
    Eradicate {
        spec: Vec<String>,
    },
    Replace {
        spec: Vec<ReplaceColumnSanitizerEntry>,
    },
    RegexTake {
        spec: String,
    },
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SanitizeColumnsEntry {
    pub comment: Option<String>,
    pub idxs: Option<Vec<usize>>,
    pub sanitizers: Vec<SanitizeColumnOpts>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TypeColumnsEntry {
    pub header: Option<String>,
    pub comment: Option<String>,
    pub target_type: ValueType,
    pub src_pattern: Option<String>,
    pub map_to_none: Option<Vec<String>>,
}

impl TypeColumnsEntry {
    pub fn new(target_type: ValueType) -> Self {
        Self {
            header: None,
            comment: None,
            target_type,
            src_pattern: None,
            map_to_none: None,
        }
    }
    pub fn builder() -> TypeColumnsEntryBuilder {
        TypeColumnsEntryBuilder::new()
    }
}

pub struct TypeColumnsEntryBuilder {
    pub header: Option<String>,
    pub comment: Option<String>,
    pub target_type: Option<ValueType>, // mandatory!
    pub src_pattern: Option<String>,
    pub map_to_none: Option<Vec<String>>,
}
impl TypeColumnsEntryBuilder {
    pub fn new() -> Self {
        Self {
            header: None,
            comment: None,
            target_type: None,
            src_pattern: None,
            map_to_none: None,
        }
    }
    pub fn with_header(&mut self, header: &str) -> &mut Self {
        self.header = Some(String::from(header));
        self
    }
    pub fn with_comment(&mut self, comment: &str) -> &mut Self {
        self.comment = Some(String::from(comment));
        self
    }
    pub fn with_datetype_src_pattern(&mut self, pattern: &str) -> &mut Self {
        self.src_pattern = Some(String::from(pattern));
        self
    }
    pub fn with_map_to_none(&mut self, map_to_none: Vec<String>) -> &mut Self {
        self.map_to_none = Some(map_to_none);
        self
    }
    pub fn build_with_target_type(&mut self, target_type: ValueType) -> TypeColumnsEntry {
        TypeColumnsEntry {
            header: std::mem::take(&mut self.header),
            comment: std::mem::take(&mut self.comment),
            target_type,
            src_pattern: std::mem::take(&mut self.src_pattern),
            map_to_none: std::mem::take(&mut self.map_to_none),
        }
    }
}

impl Default for TypeColumnsEntryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deser_parser_opt_lines() {
        let data = r#"
        {
            "comment": "We do this, because...",
            "skipLinesFromStart": 1,
            "skipLinesFromEnd": 1,
            "skipLinesByStartswith": ["foo", "-"],
            "skipLinesByRegex": ["bar.*"],
            "skipEmptyLines": true
        }
        "#;
        assert_eq!(
            ParserOptLines {
                comment: Some("We do this, because...".to_string()),
                skip_lines_from_start: Some(1),
                skip_lines_by_startswith: Some(vec!["foo".to_string(), "-".to_string()]),
                skip_lines_by_regex: Some(vec!["bar.*".to_string()]),
                skip_empty_lines: Some(true),
            },
            serde_json::from_str(data).expect("could not deserialize ")
        )
    }

    #[test]
    fn deser_col_sanitize_config_trim() {
        // Trailing
        let data = r#"
        {
            "type": "trim",
            "spec": "trailing"
        }
        "#;
        assert_eq!(
            SanitizeColumnOpts::Trim {
                spec: TrimOpts::Trailing
            },
            serde_json::from_str(data).expect("could not deserialize ")
        );

        // Leading
        let data = r#"
        {
            "type": "trim",
            "spec": "leading"
        }
        "#;
        assert_eq!(
            SanitizeColumnOpts::Trim {
                spec: TrimOpts::Leading
            },
            serde_json::from_str(data).expect("could not deserialize ")
        );

        // All
        let data = r#"
        {
            "type": "trim",
            "spec": "all"
        }
        "#;
        assert_eq!(
            SanitizeColumnOpts::Trim {
                spec: TrimOpts::All
            },
            serde_json::from_str(data).expect("could not deserialize ")
        );
    }

    #[test]
    fn deser_col_sanitize_config_casing() {
        // ToLower
        let data = r#"
        {
            "type": "casing",
            "spec": "toLower"
        }
        "#;
        assert_eq!(
            SanitizeColumnOpts::Casing {
                spec: CasingOpts::ToLower
            },
            serde_json::from_str(data).expect("could not deserialize ")
        );

        // Leading
        let data = r#"
        {
            "type": "casing",
            "spec": "toUpper"
        }
        "#;
        assert_eq!(
            SanitizeColumnOpts::Casing {
                spec: CasingOpts::ToUpper
            },
            serde_json::from_str(data).expect("could not deserialize ")
        );
    }

    #[test]
    fn deser_col_sanitize_config_eradicate() {
        let data = r#"
        {
            "type": "eradicate",
            "spec": ["."]
        }
        "#;
        assert_eq!(
            SanitizeColumnOpts::Eradicate {
                spec: vec![".".to_string()]
            },
            serde_json::from_str(data).expect("could not deserialize ")
        );
    }

    #[test]
    fn deser_col_sanitize_config_regex_take() {
        let data = r#"
        {
            "type": "regexTake",
            "spec": "(\\d+\\.\\d+).*"
        }
        "#;
        assert_eq!(
            SanitizeColumnOpts::RegexTake {
                spec: "(\\d+\\.\\d+).*".to_string()
            },
            serde_json::from_str(data).expect("could not deserialize ")
        );
    }

    #[test]
    fn deser_col_sanitize_config_replace() {
        let data = r#"
        {
            "type": "replace",
            "spec": [{
                "from": "foo",
                "to": "bar"
            }]
        }
        "#;
        assert_eq!(
            SanitizeColumnOpts::Replace {
                spec: vec![ReplaceColumnSanitizerEntry {
                    from: "foo".to_string(),
                    to: "bar".to_string()
                }]
            },
            serde_json::from_str(data).expect("could not deserialize ")
        );
    }

    #[test]
    fn deser_type_columns_entry_bool() {
        let data = r#"
        {
            "comment": "0",
            "header": "fooheader",
            "targetType": "Bool"
        }
        "#;
        assert_eq!(
            TypeColumnsEntry::builder()
                .with_comment("0")
                .with_header("fooheader")
                .build_with_target_type(ValueType::Bool),
            serde_json::from_str(data).expect("could not deserialize ")
        );
    }

    #[test]
    fn deser_type_columns_entry_naive_date_default() {
        let data = r#"
        {
            "comment": "0",
            "header": "fooheader",
            "targetType": "NaiveDate"
        }
        "#;
        assert_eq!(
            TypeColumnsEntry::builder()
                .with_comment("0")
                .with_header("fooheader")
                .build_with_target_type(ValueType::NaiveDate),
            serde_json::from_str(data).expect("could not deserialize ")
        );
    }

    #[test]
    fn deser_type_columns_entry_naive_date_pattern() {
        let data = r#"
        {
            "comment": "0",
            "header": "fooheader",
            "targetType": "NaiveDate",
            "srcPattern": "%Y-%m-%d"
        }
        "#;
        assert_eq!(
            TypeColumnsEntry::builder()
                .with_comment("0")
                .with_header("fooheader")
                .with_datetype_src_pattern("%Y-%m-%d")
                .build_with_target_type(ValueType::NaiveDate),
            serde_json::from_str(data).expect("could not deserialize ")
        );
    }

    #[test]
    fn deser_conf() {
        let cfg_str = r###"
        {
            "comment": "Some optional explanation",
            "parserOpts": {
                "comment": "Some optional explanation",
                "separatorChar": ",",
                "enclosureChar": "\"",
                "lines": {
                    "comment": "Some optional explanation",
                    "skipLinesFromStart": 1,
                    "skipLinesByStartswith": ["#", "-"],
                    "skipEmptyLines": true
                },
                "saveSkippedLines": false,
                "firstLineIsHeader": true
            },
            "sanitizeColumns": [{
                "comment": "Some optional explanation",
                "sanitizers": [{
                    "type": "trim",
                    "spec": "all"
                }]
            }, {
                "comment": "Some optional explanation",
                "idxs": [0],
                "sanitizers": [{
                    "type": "casing",
                    "spec": "toLower"
                }]
            }, {
                "comment": "Some optional explanation",
                "idxs": [1],
                "sanitizers": [{
                    "type": "casing",
                    "spec": "toUpper"
                }]
            }],
            "typeColumns": [
                { "comment": "0", "header": "Header-1", "targetType": "Char" },
                { "comment": "1", "header": "Header-2", "targetType": "String" },
                { "comment": "2", "header": "Header-3", "targetType": "Int8" },
                { "comment": "3", "header": "Header-4", "targetType": "DateTime", "srcPattern": "%FT%T%:z"}
            ]
        }
        "###;

        let cfg = ConfigRoot {
            comment: Some(String::from("Some optional explanation")),
            parser_opts: ParserOpts {
                comment: Some(String::from("Some optional explanation")),
                separator_char: ',',
                enclosure_char: Some('"'),
                lines: Some(ParserOptLines {
                    comment: Some(String::from("Some optional explanation")),
                    skip_lines_from_start: Some(1 as usize),
                    skip_empty_lines: Some(true),
                    skip_lines_by_startswith: Some(vec![String::from("#"), String::from("-")]),
                    skip_lines_by_regex: None,
                }),
                first_line_is_header: true,
                save_skipped_lines: false,
            },
            sanitize_columns: Some(vec![
                SanitizeColumnsEntry {
                    comment: Some(String::from("Some optional explanation")),
                    idxs: None,
                    sanitizers: vec![SanitizeColumnOpts::Trim {
                        spec: TrimOpts::All,
                    }],
                },
                SanitizeColumnsEntry {
                    comment: Some(String::from("Some optional explanation")),
                    idxs: Some(vec![0_usize]),
                    sanitizers: vec![SanitizeColumnOpts::Casing {
                        spec: CasingOpts::ToLower,
                    }],
                },
                SanitizeColumnsEntry {
                    comment: Some(String::from("Some optional explanation")),
                    idxs: Some(vec![1_usize]),
                    sanitizers: vec![SanitizeColumnOpts::Casing {
                        spec: CasingOpts::ToUpper,
                    }],
                },
            ]),
            type_columns: Some(vec![
                TypeColumnsEntry::builder()
                    .with_comment("0")
                    .with_header("Header-1")
                    .build_with_target_type(ValueType::Char),
                TypeColumnsEntry::builder()
                    .with_comment("1")
                    .with_header("Header-2")
                    .build_with_target_type(ValueType::String),
                TypeColumnsEntry::builder()
                    .with_comment("2")
                    .with_header("Header-3")
                    .build_with_target_type(ValueType::Int8),
                TypeColumnsEntry::builder()
                    .with_comment("3")
                    .with_header("Header-4")
                    .with_datetype_src_pattern("%FT%T%:z")
                    .build_with_target_type(ValueType::DateTime),
            ]),
        };

        assert_eq!(
            cfg,
            serde_json::from_str(cfg_str).expect("could not deserialize ")
        );
    }
}
