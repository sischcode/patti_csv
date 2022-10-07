use std::collections::HashMap;

use crate::{
    conf::jsonconf::{self, *},
    errors::{PattiCsvError, Result},
    iterating_parser::{PattiCsvParser, PattiCsvParserBuilder},
    parser_config::{TypeColumnEntry, VecOfTokenTransitizers},
    skip_take_lines::*,
    transform_sanitize_token::*,
};

fn resolve_sanitize_column_opts_entry(
    entry: &SanitizeColumnOpts,
) -> Result<VecOfTokenTransitizers> {
    match entry {
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
            .iter()
            .map(|er| -> Box<dyn TransformSanitizeToken + Send + Sync> {
                Box::new(Eradicate::new(er))
            })
            .collect::<VecOfTokenTransitizers>()),

        jsonconf::SanitizeColumnOpts::Replace { spec } => Ok(spec
            .iter()
            .map(|re| -> Box<dyn TransformSanitizeToken + Send + Sync> {
                Box::new(ReplaceWith::new(&re.from, &re.to))
            })
            .collect::<VecOfTokenTransitizers>()),

        jsonconf::SanitizeColumnOpts::RegexTake { spec } => {
            let re = RegexTake::new(spec)?; // <--- this is why we do all this...
            Ok(vec![Box::new(re)])
        }
    }
}

fn resolve_sanitize_columns_entry(
    entry: &SanitizeColumnsEntry,
) -> Result<Vec<(Option<usize>, VecOfTokenTransitizers)>> {
    // inner resolve helper
    fn mk_token_transitizers_for(entry: &SanitizeColumnsEntry) -> Result<VecOfTokenTransitizers> {
        let tmp_accum: Result<VecOfTokenTransitizers> =
            Ok(Vec::with_capacity(entry.sanitizers.len())); // This wont be the correct length, but more of a lower bound

        return entry
            .sanitizers
            .iter()
            .map(|san| -> Result<VecOfTokenTransitizers> {
                resolve_sanitize_column_opts_entry(san)
            })
            // I really didn't get how I needed to use flatten + collect in this context, so I did it manually, in the end.
            // Essentially we want this: [Result<TransformSanitizeTokens>, Result<TransformSanitizeTokens>, ...] -> Result<TransformSanitizeTokens>
            // However, this means the first error will always end up in the Err part of the Result.
            .fold(tmp_accum, |acc, mut curr| match acc {
                Ok(mut acc) => match curr {
                    Ok(ref mut curr) => {
                        acc.append(curr);
                        Ok(acc)
                    }
                    Err(err) => Err(err), // if we have an error, pass it through...
                },
                Err(err) => Err(err), // if we had an error before, pass it through...
            });
    }

    if let Some(idxs) = &entry.idxs {
        let mut res: Vec<(Option<usize>, VecOfTokenTransitizers)> =
            Vec::with_capacity(idxs.len() * entry.sanitizers.len()); // again, capacity is more of a lower bound

        for &i in idxs {
            let r = mk_token_transitizers_for(entry)?;
            res.push((Some(i), r));
        }
        Ok(res)
    } else {
        match mk_token_transitizers_for(entry) {
            Ok(rt) => Ok(vec![(None, rt)]),
            Err(e) => Err(e),
        }
    }
}

impl From<&TypeColumnsEntry> for TypeColumnEntry {
    fn from(entry: &TypeColumnsEntry) -> Self {
        match (&entry.src_pattern, &entry.map_to_none) {
            (None, None) => TypeColumnEntry::new(entry.header.clone(), entry.target_type.clone()),
            (None, Some(map_to_none)) => TypeColumnEntry::new_with_map_to_none(
                entry.header.clone(),
                entry.target_type.clone(),
                map_to_none.clone(),
            ),
            (Some(src_pattern), None) => TypeColumnEntry::new_with_chrono_pattern(
                entry.header.clone(),
                entry.target_type.clone(),
                src_pattern.clone(),
            ),
            (Some(src_pattern), Some(map_to_none)) => {
                TypeColumnEntry::new_with_chrono_pattern_with_map_to_none(
                    entry.header.clone(),
                    entry.target_type.clone(),
                    src_pattern.clone(),
                    map_to_none.clone(),
                )
            }
        }
    }
}

/// Helper method. Fills a given transitizer map with VecOfTokenTransitizers for the given entry.
fn add_transitizers_from(
    entry: &SanitizeColumnsEntry,
    transitizers: &mut HashMap<Option<usize>, VecOfTokenTransitizers>,
) -> Result<()> {
    let sanitizers_for_columns = resolve_sanitize_columns_entry(entry)?;
    sanitizers_for_columns
        .into_iter()
        .for_each(|(col_idx, mut new_transitizers)| {
            // This distinction is between the "global" (None) and local (Some()) sanitizers.
            match col_idx {
                // GLOBAL
                None => match transitizers.get_mut(&None) {
                    // ADD/INIT
                    None => {
                        transitizers.insert(None, new_transitizers);
                    }
                    // APPEND
                    Some(ex_tr) => {
                        ex_tr.append(&mut new_transitizers);
                    }
                },
                // LOCAL
                Some(idx) => match transitizers.get_mut(&Some(idx)) {
                    // ADD/INIT
                    None => {
                        transitizers.insert(Some(idx), new_transitizers);
                    }
                    // APPEND
                    Some(ex_tr) => {
                        ex_tr.append(&mut new_transitizers);
                    }
                },
            };
        });
    Ok(())
}

/// A ref to ConfigRoot would actually be sufficient, but we want the ConfigRoot to be dropped.
impl TryFrom<ConfigRoot> for PattiCsvParser {
    type Error = PattiCsvError;

    fn try_from(cfg: ConfigRoot) -> Result<Self> {
        let mut builder = PattiCsvParserBuilder::new()
            .enclosure_char(cfg.parser_opts.enclosure_char)
            .separator_char(cfg.parser_opts.separator_char)
            .first_data_line_is_header(cfg.parser_opts.first_line_is_header);

        if let Some(vec_san_col_entry) = &cfg.sanitize_columns {
            let mut transitizers: HashMap<Option<usize>, VecOfTokenTransitizers> =
                HashMap::with_capacity(vec_san_col_entry.len()); // only correct for idx(1)<-->sanitizer(1) relationships

            vec_san_col_entry.iter().try_for_each(|san_col_entry| {
                add_transitizers_from(san_col_entry, &mut transitizers)
            })?;

            if !transitizers.is_empty() {
                builder = builder.column_transitizers(transitizers);
            }
        }

        if let Some(skip_take_lines_cfg) = &cfg.parser_opts.lines {
            let mut skip_take_lines: Vec<Box<dyn SkipTakeLines + Send + Sync>> = Vec::new();

            if let Some(true) = skip_take_lines_cfg.skip_empty_lines {
                skip_take_lines.push(Box::new(SkipEmptyLines {}));
            }
            if let Some(v) = skip_take_lines_cfg.skip_lines_from_start {
                skip_take_lines.push(Box::new(SkipLinesFromStart::new(v)));
            }
            if let Some(v) = &skip_take_lines_cfg.skip_lines_by_startswith {
                v.iter()
                    .for_each(|e| skip_take_lines.push(Box::new(SkipLinesStartingWith::new(e))));
            }
            if let Some(v) = &skip_take_lines_cfg.skip_lines_by_regex {
                for c in v.iter() {
                    let tmp = SkipLinesByRegex::new(c)?;
                    skip_take_lines.push(Box::new(tmp))
                }
            }

            if !skip_take_lines.is_empty() {
                builder = builder.skip_take_lines_fns(skip_take_lines);
            }
        }

        if let Some(col_typings_cfg) = &cfg.type_columns {
            let col_typings = col_typings_cfg.iter().map(TypeColumnEntry::from).collect();
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

    pub mod resolve_sanitize_column_opts_entry {
        use super::*;

        #[test]
        fn resolve_trim_all() -> Result<()> {
            let test_setup_val = SanitizeColumnOpts::Trim {
                spec: TrimOpts::All,
            };
            let exp = vec![Box::new(TrimAll)];
            let test_val = resolve_sanitize_column_opts_entry(&test_setup_val)?;

            assert_eq!(
                exp.get(0).unwrap().get_self_info(),
                test_val.get(0).unwrap().get_self_info()
            );
            Ok(())
        }

        #[test]
        fn resolve_trim_leading() -> Result<()> {
            let test_setup_val = SanitizeColumnOpts::Trim {
                spec: TrimOpts::Leading,
            };
            let exp = vec![Box::new(TrimLeading)];
            let test_val = resolve_sanitize_column_opts_entry(&test_setup_val)?;

            assert_eq!(
                exp.get(0).unwrap().get_self_info(),
                test_val.get(0).unwrap().get_self_info()
            );
            Ok(())
        }

        #[test]
        fn resolve_trim_trailing() -> Result<()> {
            let test_setup_val = SanitizeColumnOpts::Trim {
                spec: TrimOpts::Trailing,
            };
            let exp = vec![Box::new(TrimTrailing)];
            let test_val = resolve_sanitize_column_opts_entry(&test_setup_val)?;

            assert_eq!(
                exp.get(0).unwrap().get_self_info(),
                test_val.get(0).unwrap().get_self_info()
            );
            Ok(())
        }

        #[test]
        fn resolve_casing_to_lower() -> Result<()> {
            let test_setup_val = SanitizeColumnOpts::Casing {
                spec: CasingOpts::ToLower,
            };
            let exp = vec![Box::new(ToLowercase)];
            let test_val = resolve_sanitize_column_opts_entry(&test_setup_val)?;

            assert_eq!(
                exp.get(0).unwrap().get_self_info(),
                test_val.get(0).unwrap().get_self_info()
            );
            Ok(())
        }

        #[test]
        fn resolve_casing_to_upper() -> Result<()> {
            let test_setup_val = SanitizeColumnOpts::Casing {
                spec: CasingOpts::ToUpper,
            };
            let exp = vec![Box::new(ToUppercase)];
            let test_val = resolve_sanitize_column_opts_entry(&test_setup_val)?;

            assert_eq!(
                exp.get(0).unwrap().get_self_info(),
                test_val.get(0).unwrap().get_self_info()
            );
            Ok(())
        }

        #[test]
        fn resolve_eradicate() -> Result<()> {
            let test_setup_val = SanitizeColumnOpts::Eradicate {
                spec: vec![String::from("foo"), String::from("bar")],
            };
            let exp = vec![
                Box::new(Eradicate::new("foo")),
                Box::new(Eradicate::new("bar")),
            ];
            let test_val = resolve_sanitize_column_opts_entry(&test_setup_val)?;

            assert_eq!(
                exp.get(0).unwrap().get_self_info(),
                test_val.get(0).unwrap().get_self_info()
            );
            assert_eq!(
                exp.get(1).unwrap().get_self_info(),
                test_val.get(1).unwrap().get_self_info()
            );
            Ok(())
        }

        #[test]
        fn resolve_replace() -> Result<()> {
            let test_setup_val = SanitizeColumnOpts::Replace {
                spec: vec![
                    ReplaceColumnSanitizerEntry {
                        from: String::from("aaa"),
                        to: String::from("bbb"),
                    },
                    ReplaceColumnSanitizerEntry {
                        from: String::from("ccc"),
                        to: String::from("ddd"),
                    },
                ],
            };
            let exp = vec![
                Box::new(ReplaceWith::new("aaa", "bbb")),
                Box::new(ReplaceWith::new("ccc", "ddd")),
            ];
            let test_val = resolve_sanitize_column_opts_entry(&test_setup_val)?;

            assert_eq!(
                exp.get(0).unwrap().get_self_info(),
                test_val.get(0).unwrap().get_self_info()
            );
            assert_eq!(
                exp.get(1).unwrap().get_self_info(),
                test_val.get(1).unwrap().get_self_info()
            );
            Ok(())
        }
    }

    #[test]
    fn add_transitizers_from_succ() -> Result<()> {
        let sce = SanitizeColumnsEntry {
            comment: None,
            idxs: Some(vec![0_usize, 1]),
            sanitizers: vec![
                SanitizeColumnOpts::Trim {
                    spec: TrimOpts::All,
                },
                SanitizeColumnOpts::Casing {
                    spec: CasingOpts::ToLower,
                },
            ],
        };

        let mut transitizers_map: HashMap<Option<usize>, VecOfTokenTransitizers> =
            HashMap::with_capacity(4);

        add_transitizers_from(&sce, &mut transitizers_map)?;

        assert_eq!(2, transitizers_map.len());
        assert_eq!(2, transitizers_map.get(&Some(0)).unwrap().len());
        assert_eq!(2, transitizers_map.get(&Some(1)).unwrap().len());

        println!("{:?}", &transitizers_map);

        Ok(())
    }

    #[test]
    fn from_sanitize_column_entry_for_global_and_trans_san_token_tuple_trim_all() {
        let sce = SanitizeColumnsEntry {
            comment: None,
            idxs: None,
            sanitizers: vec![SanitizeColumnOpts::Trim {
                spec: TrimOpts::All,
            }],
        };

        let res = resolve_sanitize_columns_entry(&sce).unwrap();
        assert_eq!(1, res.len());

        let res_first = res.first().unwrap();
        assert_eq!(None, res_first.0);

        let exp: VecOfTokenTransitizers = vec![Box::new(TrimAll)];
        assert_eq!(
            exp.get(0).unwrap().get_self_info(),
            res_first.1.get(0).unwrap().get_self_info()
        );
    }

    #[test]
    fn from_sanitize_column_entry_for_idx_and_trans_san_token_tuple_replace_with() {
        let sce = SanitizeColumnsEntry {
            comment: None,
            idxs: Some(vec![1_usize]),
            sanitizers: vec![SanitizeColumnOpts::Replace {
                spec: vec![ReplaceColumnSanitizerEntry {
                    from: String::from("foo"),
                    to: String::from("bar"),
                }],
            }],
        };

        let res = resolve_sanitize_columns_entry(&sce).unwrap();
        assert_eq!(1, res.len());

        let res_first = res.first().unwrap();
        assert_eq!(Some(1), res_first.0);

        let exp: VecOfTokenTransitizers = vec![Box::new(ReplaceWith::new("foo", "bar"))];
        assert_eq!(
            exp.get(0).unwrap().get_self_info(),
            res_first.1.get(0).unwrap().get_self_info()
        );
    }

    #[test]
    fn from_type_columns_entry_for_type_column_entry_no_date_type() {
        let exp = TypeColumnEntry::new(Some(String::from("header-1")), ValueType::Char);
        let test = TypeColumnsEntry::builder()
            .with_header("header-1")
            .build_with_target_type(ValueType::Char);
        let res = TypeColumnEntry::from(&test);
        assert_eq!(exp, res);
    }

    #[test]
    fn from_type_columns_entry_for_type_column_entry_date_type() {
        let exp = TypeColumnEntry::new(Some(String::from("header-1")), ValueType::DateTime);
        let test = TypeColumnsEntry::builder()
            .with_header("header-1")
            .build_with_target_type(ValueType::DateTime);
        let res = TypeColumnEntry::from(&test);
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
                    idxs: None,
                    sanitizers: vec![SanitizeColumnOpts::Trim {
                        spec: TrimOpts::All,
                    }],
                },
                SanitizeColumnsEntry {
                    comment: None,
                    idxs: Some(vec![0_usize]),
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
