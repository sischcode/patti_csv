use std::collections::HashMap;

use crate::{
    conf::jsonconf::{self, *},
    errors::{PattiCsvError, Result},
    iterating_parser::{PattiCsvParser, PattiCsvParserBuilder},
    parser_config::{TransformSanitizeTokens, TypeColumnEntry},
    skip_take_lines::*,
    transform_sanitize_token::*,
};

impl TryFrom<&mut SanitizeColumnOpts> for TransformSanitizeTokens {
    type Error = PattiCsvError;

    fn try_from(entry_elem: &mut SanitizeColumnOpts) -> Result<TransformSanitizeTokens> {
        match entry_elem {
            jsonconf::SanitizeColumnOpts::Trim { spec } => match spec {
                TrimOpts::All => Ok(vec![Box::new(TrimAll)]),
                TrimOpts::Leading => Ok(vec![Box::new(TrimLeading)]),
                TrimOpts::Trailing => Ok(vec![Box::new(TrimTrailing)]),
            },

            jsonconf::SanitizeColumnOpts::Casing { spec } => match spec {
                CasingOpts::ToLower => Ok(vec![Box::new(ToLowercase)]),
                CasingOpts::ToUpper => Ok(vec![Box::new(ToUppercase)]),
            },

            jsonconf::SanitizeColumnOpts::Eradicate { spec } => Ok(spec
                .iter_mut()
                .map(|er| -> Box<dyn TransformSanitizeToken> {
                    Box::new(Eradicate {
                        eradicate: std::mem::take(er),
                    })
                })
                .collect::<TransformSanitizeTokens>()),

            jsonconf::SanitizeColumnOpts::Replace { spec } => Ok(spec
                .iter_mut()
                .map(|re| -> Box<dyn TransformSanitizeToken> {
                    Box::new(ReplaceWith {
                        from: std::mem::take(&mut re.from),
                        to: std::mem::take(&mut re.to),
                    })
                })
                .collect::<TransformSanitizeTokens>()),

            jsonconf::SanitizeColumnOpts::RegexTake { spec } => {
                let re = RegexTake::new(spec)?; // <--- this is why we do all this...
                Ok(vec![Box::new(re)])
            }
        }
    }
}

impl TryFrom<&mut SanitizeColumnsEntry> for (Option<usize>, TransformSanitizeTokens) {
    type Error = PattiCsvError;

    fn try_from(
        entry: &mut SanitizeColumnsEntry,
    ) -> Result<(Option<usize>, TransformSanitizeTokens)> {
        let vec_tst = entry
            .sanitizers
            .iter_mut()
            .map(|entry_elem| -> Result<TransformSanitizeTokens> { entry_elem.try_into() })
            // I really didn't get how I needed to use flatten + collect in this context, so I did it manually, in the end.
            // Essentially we want this: [Result<TransformSanitizeTokens>, Result<TransformSanitizeTokens>, ...] -> Result<TransformSanitizeTokens>
            // However, this means the first error will always end up in the Err part of the Result.
            .reduce(|acc, mut e| match acc {
                Ok(mut acc_v) => match e {
                    Ok(ref mut new_v) => {
                        acc_v.append(new_v);
                        Ok(acc_v)
                    }
                    Err(err) => Err(err), // if we have an error, pass it through...
                },
                Err(err) => Err(err), // if we had an error before, pass it through...
            });

        // Reduce however wraps it in an Option, since the iterator could be empty. We need to get rid of this here.
        // In our case, if that happens, we return an empty vec (for this index)
        match vec_tst {
            Some(v) => match v {
                Err(err) => Err(err),
                Ok(v) => Ok((entry.idx, v)),
            },
            None => Ok((entry.idx, Vec::<Box<dyn TransformSanitizeToken>>::new())),
        }
    }
}

impl From<&mut TypeColumnsEntry> for TypeColumnEntry {
    fn from(tce: &mut TypeColumnsEntry) -> Self {
        let src_chrono_pattern_opt = tce.src_pattern.as_mut();
        let map_to_none_opt = tce.map_to_none.as_mut();

        match (src_chrono_pattern_opt, map_to_none_opt) {
            (None, None) => TypeColumnEntry::new(
                std::mem::take(&mut tce.header),
                std::mem::take(&mut tce.target_type),
            ),
            (None, Some(map_to_none)) => TypeColumnEntry::new_with_map_to_none(
                std::mem::take(&mut tce.header),
                std::mem::take(&mut tce.target_type),
                std::mem::take(map_to_none),
            ),
            (Some(src_pattern), None) => TypeColumnEntry::new_with_chrono_pattern(
                std::mem::take(&mut tce.header),
                std::mem::take(&mut tce.target_type),
                std::mem::take(src_pattern),
            ),
            (Some(src_pattern), Some(map_to_none)) => {
                TypeColumnEntry::new_with_chrono_pattern_with_map_to_none(
                    std::mem::take(&mut tce.header),
                    std::mem::take(&mut tce.target_type),
                    std::mem::take(src_pattern),
                    std::mem::take(map_to_none),
                )
            }
        }
    }
}

impl TryFrom<ConfigRoot> for PattiCsvParser {
    type Error = PattiCsvError;

    fn try_from(cfg: ConfigRoot) -> Result<Self> {
        let mut builder = PattiCsvParserBuilder::new()
            .enclosure_char(cfg.parser_opts.enclosure_char)
            .separator_char(cfg.parser_opts.separator_char)
            .first_data_line_is_header(cfg.parser_opts.first_line_is_header);

        if let Some(mut san) = cfg.sanitize_columns {
            let mut transitizers: HashMap<Option<usize>, TransformSanitizeTokens> =
                HashMap::with_capacity(san.len());

            san.iter_mut().try_for_each(|e| -> Result<()> {
                let (opt_line_num, ts_tokens): (Option<usize>, TransformSanitizeTokens) =
                    e.try_into()?;
                transitizers.insert(opt_line_num, ts_tokens);
                Ok(())
            })?;

            if !transitizers.is_empty() {
                builder = builder.column_transitizers(transitizers);
            }
        }

        if let Some(skip_take_lines_cfg) = cfg.parser_opts.lines {
            let mut skip_take_lines: Vec<Box<dyn SkipTakeLines>> = Vec::new();

            if let Some(true) = skip_take_lines_cfg.skip_empty_lines {
                skip_take_lines.push(Box::new(SkipEmptyLines {}));
            }
            if let Some(v) = skip_take_lines_cfg.skip_lines_from_start {
                skip_take_lines.push(Box::new(SkipLinesFromStart { skip_num_lines: v }));
            }
            if let Some(mut v) = skip_take_lines_cfg.skip_lines_by_startswith {
                v.iter_mut().for_each(|e| {
                    skip_take_lines.push(Box::new(SkipLinesStartingWith {
                        starts_with: std::mem::take(e),
                    }))
                });
            }
            if let Some(v) = skip_take_lines_cfg.skip_lines_by_regex {
                for c in v.iter() {
                    let tmp = SkipLinesByRegex::new(c)?;
                    skip_take_lines.push(Box::new(tmp))
                }
            }

            if !skip_take_lines.is_empty() {
                builder = builder.skip_take_lines_fns(skip_take_lines);
            }
        }

        if let Some(mut col_typings_cfg) = cfg.type_columns {
            let col_typings = col_typings_cfg
                .iter_mut()
                .map(TypeColumnEntry::from)
                .collect();
            builder = builder.column_typings(col_typings);
        }

        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use venum::value::Value;
    use venum::value_type::ValueType;
    use venum_tds::{data_cell::DataCell, data_cell_row::DataCellRow};

    use super::*;

    #[test]
    fn from_sanitize_column_entry_for_idx_and_trans_san_token_tuple_trim_all() {
        let mut sce = SanitizeColumnsEntry {
            comment: None,
            idx: None,
            sanitizers: vec![SanitizeColumnOpts::Trim {
                spec: TrimOpts::All,
            }],
        };
        let res_tuple: (Option<usize>, TransformSanitizeTokens) = (&mut sce).try_into().unwrap();
        assert_eq!(None, res_tuple.0);

        let exp: Vec<Box<dyn TransformSanitizeToken>> = vec![Box::new(TrimAll)];
        assert_eq!(
            exp.get(0).unwrap().get_info(),
            res_tuple.1.get(0).unwrap().get_info()
        );
    }

    #[test]
    fn from_sanitize_column_entry_for_idx_and_trans_san_token_tuple_replace_with() {
        let mut sce = SanitizeColumnsEntry {
            comment: None,
            idx: Some(1),
            sanitizers: vec![SanitizeColumnOpts::Replace {
                spec: vec![ReplaceColumnSanitizerEntry {
                    from: String::from("foo"),
                    to: String::from("bar"),
                }],
            }],
        };
        let res_tuple: (Option<usize>, TransformSanitizeTokens) = (&mut sce).try_into().unwrap();
        assert_eq!(Some(1), res_tuple.0);

        let exp: Vec<Box<dyn TransformSanitizeToken>> = vec![Box::new(ReplaceWith {
            from: String::from("foo"),
            to: String::from("bar"),
        })];
        assert_eq!(
            exp.get(0).unwrap().get_info(),
            res_tuple.1.get(0).unwrap().get_info()
        );
        // println!("{:}", res_tuple.1.get(0).unwrap().get_info());
    }

    #[test]
    fn from_type_columns_entry_for_type_column_entry_no_date_type() {
        let exp = TypeColumnEntry::new(Some(String::from("header-1")), ValueType::Char);
        let mut test = TypeColumnsEntry::builder()
            .with_header("header-1")
            .build_with_target_type(ValueType::Char);
        let res = TypeColumnEntry::from(&mut test);
        assert_eq!(exp, res);
    }

    #[test]
    fn from_type_columns_entry_for_type_column_entry_date_type() {
        let exp = TypeColumnEntry::new(Some(String::from("header-1")), ValueType::DateTime);
        let mut test = TypeColumnsEntry::builder()
            .with_header("header-1")
            .build_with_target_type(ValueType::DateTime);
        let res = TypeColumnEntry::from(&mut test);
        assert_eq!(exp, res);
    }

    #[test]
    fn try_from_data_cfg_root_tuple_for_patti_csv_parser_1() {
        let cfg = ConfigRoot {
            comment: None,
            parser_opts: ParserOpts {
                comment: None,
                separator_char: ',',
                enclosure_char: Some('"'),
                lines: Some(ParserOptLines {
                    comment: None,
                    skip_lines_from_start: Some(1_usize),
                    skip_empty_lines: Some(true),
                    skip_lines_by_startswith: Some(vec![String::from("#"), String::from("-")]),
                    skip_lines_by_regex: None,
                }),
                first_line_is_header: true,
                save_skipped_lines: false,
            },
            sanitize_columns: Some(vec![
                SanitizeColumnsEntry {
                    comment: None,
                    idx: None,
                    sanitizers: vec![SanitizeColumnOpts::Trim {
                        spec: TrimOpts::All,
                    }],
                },
                SanitizeColumnsEntry {
                    comment: None,
                    idx: Some(0 as usize),
                    sanitizers: vec![SanitizeColumnOpts::Casing {
                        spec: CasingOpts::ToLower,
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
                    .with_datetype_src_pattern("%F")
                    .build_with_target_type(ValueType::NaiveDate),
            ]),
        };

        let data_str =
            "# some bullshit\n\n-some bullshit again\na,b,c,d\n A, BEE , 1 , 2022-01-01 ";
        let mut test_data_cursor = std::io::Cursor::new(data_str);

        let parser = PattiCsvParser::try_from(cfg).unwrap();
        let mut iter = parser.parse_iter(&mut test_data_cursor);

        let res_header = iter.next().unwrap().unwrap(); // first unwrap is from the iter, second one is our result
        let res_line01 = iter.next().unwrap().unwrap(); // first unwrap is from the iter, second one is our result

        // Data is:
        // =========================
        // # some bullshit
        //
        // - some bullshit
        // a,b,c,d
        //  A, BEE , 1 , 2022-01-01
        // =========================

        // Data we want:
        // ==================================================
        // Header-1,Header-2,Header-3,Header-4
        // char(a),String(BEE),Int8(1),NaiveDate(2022-01-01)
        // ==================================================

        assert_eq!(
            DataCellRow {
                0: vec![
                    DataCell::new(
                        String::from("Header-1"),
                        0,
                        Value::String(String::from("Header-1"))
                    )
                    .unwrap(),
                    DataCell::new(
                        String::from("Header-2"),
                        1,
                        Value::String(String::from("Header-2"))
                    )
                    .unwrap(),
                    DataCell::new(
                        String::from("Header-3"),
                        2,
                        Value::String(String::from("Header-3"))
                    )
                    .unwrap(),
                    DataCell::new(
                        String::from("Header-4"),
                        3,
                        Value::String(String::from("Header-4"))
                    )
                    .unwrap(),
                ]
            },
            res_header
        );

        assert_eq!(
            DataCellRow {
                0: vec![
                    DataCell::new(String::from("Header-1"), 0, Value::Char('a')).unwrap(),
                    DataCell::new(
                        String::from("Header-2"),
                        1,
                        Value::String(String::from("BEE"))
                    )
                    .unwrap(),
                    DataCell::new(String::from("Header-3"), 2, Value::Int8(1)).unwrap(),
                    DataCell::new(
                        String::from("Header-4"),
                        3,
                        Value::parse_naive_date_from_str_iso8601_ymd("2022-01-01").unwrap()
                    )
                    .unwrap(),
                ]
            },
            res_line01
        );
    }
}
