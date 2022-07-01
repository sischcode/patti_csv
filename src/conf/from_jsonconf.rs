use std::{collections::HashMap, io::Read};

use venum::venum::Value;

use crate::{
    conf::jsonconf::{self, *},
    errors::{PattiCsvError, Result},
    iterating_parser::{PattiCsvParser, PattiCsvParserBuilder},
    parser_config::{TransformSanitizeTokens, TypeColumnEntry},
    skip_take_lines::*,
    transform_sanitize_token::*,
};

impl From<&VenumValueVariantNames> for Value {
    fn from(vvvn: &VenumValueVariantNames) -> Self {
        match vvvn {
            VenumValueVariantNames::Char => Value::char_default(),
            VenumValueVariantNames::String => Value::string_default(),
            VenumValueVariantNames::Int8 => Value::int8_default(),
            VenumValueVariantNames::Int16 => Value::int16_default(),
            VenumValueVariantNames::Int32 => Value::int32_default(),
            VenumValueVariantNames::Int64 => Value::int64_default(),
            VenumValueVariantNames::Int128 => Value::int128_default(),
            VenumValueVariantNames::UInt8 => Value::uint8_default(),
            VenumValueVariantNames::UInt16 => Value::uint16_default(),
            VenumValueVariantNames::UInt32 => Value::uint32_default(),
            VenumValueVariantNames::UInt64 => Value::uint64_default(),
            VenumValueVariantNames::UInt128 => Value::uint128_default(),
            VenumValueVariantNames::Float32 => Value::float32_default(),
            VenumValueVariantNames::Float64 => Value::float64_default(),
            VenumValueVariantNames::Bool => Value::bool_default(),
            VenumValueVariantNames::Decimal => Value::decimal_default(),
            VenumValueVariantNames::NaiveDate => Value::naive_date_default(),
            VenumValueVariantNames::NaiveDateTime => Value::naive_date_time_default(),
            VenumValueVariantNames::DateTime => Value::date_time_default(),
        }
    }
}

impl From<&mut SanitizeColumnsEntry> for (Option<usize>, TransformSanitizeTokens) {
    fn from(entry: &mut SanitizeColumnsEntry) -> (Option<usize>, TransformSanitizeTokens) {
        let vec_tst = entry
            .sanitizers
            .iter_mut()
            .map(|entry_elem| -> TransformSanitizeTokens {
                match entry_elem {
                    jsonconf::SanitizeColumnOpts::Trim { spec } => match spec {
                        TrimOpts::All => vec![Box::new(TrimAll)],
                        TrimOpts::Leading => vec![Box::new(TrimLeading)],
                        TrimOpts::Trailing => vec![Box::new(TrimTrailing)],
                    },
                    jsonconf::SanitizeColumnOpts::Casing { spec } => match spec {
                        CasingOpts::ToLower => vec![Box::new(ToLowercase)],
                        CasingOpts::ToUpper => vec![Box::new(ToUppercase)],
                    },
                    jsonconf::SanitizeColumnOpts::Eradicate { spec } => spec
                        .iter_mut()
                        .map(|er| -> Box<dyn TransformSanitizeToken> {
                            Box::new(Eradicate {
                                eradicate: std::mem::take(er),
                            })
                        })
                        .collect(),
                    jsonconf::SanitizeColumnOpts::Replace { spec } => spec
                        .iter_mut()
                        .map(|re| -> Box<dyn TransformSanitizeToken> {
                            Box::new(ReplaceWith {
                                from: std::mem::take(&mut re.from),
                                to: std::mem::take(&mut re.to),
                            })
                        })
                        .collect(),
                    jsonconf::SanitizeColumnOpts::RegexTake { spec } => {
                        vec![Box::new(RegexTake::new(spec.clone()))]
                    }
                }
            })
            .flatten()
            .collect::<TransformSanitizeTokens>();

        (entry.idx, vec_tst)
    }
}

impl From<&mut TypeColumnsEntry> for TypeColumnEntry {
    fn from(tce: &mut TypeColumnsEntry) -> Self {
        match tce.src_pattern {
            Some(ref mut pattern) => TypeColumnEntry::new_with_chrono_pattern(
                std::mem::take(&mut tce.header),
                Value::from(&tce.target_type),
                std::mem::take(pattern),
            ),
            None => TypeColumnEntry::new(tce.header.clone(), Value::from(&tce.target_type)),
        }
    }
}

impl<'rd, R: Read> TryFrom<(&'rd mut R, ConfigRoot)> for PattiCsvParser<'rd, R> {
    type Error = PattiCsvError;

    fn try_from(data_config_tuple: (&'rd mut R, ConfigRoot)) -> Result<Self> {
        let (src, cfg) = data_config_tuple;

        let mut builder = PattiCsvParserBuilder::new();
        builder
            .enclosure_char(cfg.parser_opts.enclosure_char)
            .separator_char(cfg.parser_opts.separator_char)
            .first_line_is_header(cfg.parser_opts.first_line_is_header);

        if let Some(mut san) = cfg.sanitize_columns {
            let mut transitizers: HashMap<Option<usize>, TransformSanitizeTokens> = HashMap::new();

            san.iter_mut().for_each(|s| {
                let c: (Option<usize>, TransformSanitizeTokens) = s.into();
                transitizers.insert(c.0, c.1);
            });

            if !transitizers.is_empty() {
                builder.column_transitizers(transitizers);
            }
        }

        if let Some(skip_take_lines_cfg) = cfg.parser_opts.lines {
            let mut skip_take_lines: Vec<Box<dyn SkipTakeLines>> = Vec::new();

            if let Some(v) = skip_take_lines_cfg.skip_empty_lines {
                if v {
                    skip_take_lines.push(Box::new(SkipEmptyLines {}))
                }
            }
            if let Some(v) = skip_take_lines_cfg.skip_lines_from_start {
                skip_take_lines.push(Box::new(SkipLinesFromStart { skip_num_lines: v }))
            }
            if let Some(v) = skip_take_lines_cfg.skip_lines_from_end {
                // let reader = BufReader::new(src);
                // let lines_total = reader.lines().count();

                skip_take_lines.push(Box::new(SkipLinesFromEnd {
                    skip_num_lines: v,
                    lines_total: 0, // TODO: !!!
                }))
            }
            if let Some(mut v) = skip_take_lines_cfg.skip_lines_by_startswith {
                v.iter_mut().for_each(|e| {
                    skip_take_lines.push(Box::new(SkipLinesStartingWith {
                        starts_with: std::mem::take(e),
                    }))
                })
            }
            if let Some(mut v) = skip_take_lines_cfg.take_lines_by_startswith {
                v.iter_mut().for_each(|e| {
                    skip_take_lines.push(Box::new(TakeLinesStartingWith {
                        starts_with: std::mem::take(e),
                    }))
                })
            }

            if !skip_take_lines.is_empty() {
                builder.skip_take_lines_fns(skip_take_lines);
            }
        }

        if let Some(mut col_typings_cfg) = cfg.type_columns {
            let col_typings = col_typings_cfg
                .iter_mut()
                .map(|ct| TypeColumnEntry::from(ct))
                .collect();
            builder.column_typings(col_typings);
        }

        builder.build(src)
    }
}

#[cfg(test)]
mod tests {
    use venum_tds::{cell::DataCell, row::DataCellRow};

    use super::*;

    #[test]
    fn test() {
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
                "idx": 0,
                "sanitizers": [{
                    "type": "casing",
                    "spec": "toLower"
                }]
            }],
            "typeColumns": [
                { "comment": "0", "header": "Header-1", "targetType": "Char" },
                { "comment": "1", "header": "Header-2", "targetType": "String" },
                { "comment": "2", "header": "Header-3", "targetType": "Int8" },
                { "comment": "3", "header": "Header-4", "targetType": "NaiveDate", "srcPattern": "%Y-%m-%d"}
            ]
        }
        "###;

        let cfg: ConfigRoot = serde_json::from_str(cfg_str).expect("could not deserialize config");

        let data_str =
            "# some bullshit\n\n-some bullshit again\na,b,c,d\n A, BEE , 1 , 2022-01-01 ";
        let mut test_data_cursor = std::io::Cursor::new(data_str);

        let parser = PattiCsvParser::try_from((&mut test_data_cursor, cfg)).unwrap();
        let mut iter = parser.into_iter();

        let res_header = iter.next().unwrap().unwrap();
        let res_line01 = iter.next().unwrap().unwrap();

        assert_eq!(
            DataCellRow {
                0: vec![
                    DataCell::new(
                        Value::string_default(),
                        String::from("Header-1"),
                        0,
                        Some(Value::String(String::from("Header-1")))
                    ),
                    DataCell::new(
                        Value::string_default(),
                        String::from("Header-2"),
                        1,
                        Some(Value::String(String::from("Header-2")))
                    ),
                    DataCell::new(
                        Value::string_default(),
                        String::from("Header-3"),
                        2,
                        Some(Value::String(String::from("Header-3")))
                    ),
                    DataCell::new(
                        Value::string_default(),
                        String::from("Header-4"),
                        3,
                        Some(Value::String(String::from("Header-4")))
                    ),
                ]
            },
            res_header.0
        );

        assert_eq!(
            DataCellRow {
                0: vec![
                    DataCell::new(
                        Value::char_default(),
                        String::from("Header-1"),
                        0,
                        Some(Value::Char('a'))
                    ),
                    DataCell::new(
                        Value::string_default(),
                        String::from("Header-2"),
                        1,
                        Some(Value::String(String::from("BEE")))
                    ),
                    DataCell::new(
                        Value::int8_default(),
                        String::from("Header-3"),
                        2,
                        Some(Value::Int8(1))
                    ),
                    DataCell::new(
                        Value::naive_date_default(),
                        String::from("Header-4"),
                        3,
                        Some(Value::parse_naive_date_from_str_iso8601_ymd("2022-01-01").unwrap())
                    ),
                ]
            },
            res_line01.0
        );
    }
}
