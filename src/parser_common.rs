use std::collections::{HashMap, VecDeque};

use venum_tds::data_cell::DataCell;
use venum_tds::data_cell_row::DataCellRow;

use crate::errors::{PattiCsvError, Result, SanitizeError};

use super::parser_config::{TransformSanitizeTokens, TypeColumnEntry};

pub fn build_layout_template(
    header_tokens: Option<&VecDeque<String>>,
    column_typing: &[TypeColumnEntry],
) -> Result<DataCellRow> {
    let mut csv_cell_templ_row = DataCellRow::new(); // our return value

    match header_tokens {
        // If we do not have header tokens (i.e. from the parsed column header line), we only have the column_typings info that is either provided
        // through the user config, or auto-generated as everything-is-string + index-numbers-as-column headers.
        // However, since the header name is optional in the typings, we still have to fill in with index-as-header-name, if the header name is not
        // provided.
        None => {
            for (idx, tce) in column_typing.iter().enumerate() {
                csv_cell_templ_row.push(DataCell::new_without_data(
                    tce.target_type.clone(),
                    tce.header.as_ref().unwrap_or(&idx.to_string()).clone(), // fallback to index-as-header, if no real header name is given
                    idx,
                ));
            }
        }
        // If we're here, we have header lines AND column typings (either real ones, or auto generated index-as-header-name ones. In this
        // case the type would also be ValueName::String. Either way, we know that the column typings info is complete, length / # column wise.)
        // From a logical standpoint, the column typings have precedence over the header lines, because they are used to actually override
        // given header values.
        Some(header_tokens) => {
            for (idx, tce) in column_typing.iter().enumerate() {
                csv_cell_templ_row.push(DataCell::new_without_data(
                    tce.target_type.clone(),
                    tce.header
                        .as_ref()
                        .or_else(|| header_tokens.get(idx)) // ok returns the column-typing header, else returns the header-header
                        .ok_or(PattiCsvError::Generic {
                            msg: format!("No header provided for column#{}", idx), // we don't fall back to indexes, like above, because something is wrong, when we don't have a header from the header line
                        })?
                        .clone(),
                    idx,
                ));
            }
        }
    }
    Ok(csv_cell_templ_row)
}

pub fn sanitize_token(
    token: String,
    column_sanitizers: &HashMap<Option<usize>, TransformSanitizeTokens>,
    line_num: usize, // for error context
    col_num: usize,  // used internally AND for error context
) -> Result<String> {
    // If we have sanitizers for index=None, that means, we have global sanitizers, not bound to any index. I.e. they will always be applied.
    // Note that this strongly differs from getting None as a result of a .get on the HashMap!
    let token = match column_sanitizers.get(&None) {
        Some(tst) => tst.iter().try_fold(token, |acc, transitizer| {
            transitizer
                .transitize(&acc) // apply filter, then yield
                // Supply more error context
                .map_err(|e| {
                    if let PattiCsvError::Sanitize(se) = e {
                        PattiCsvError::Sanitize(SanitizeError::extend(
                            se,
                            Some(format!(
                                " Error in/from global sanitizer: {}.",
                                &transitizer.get_info()
                            )),
                            Some(line_num),
                            None,
                        ))
                    } else {
                        panic!("If we end up here, we mixed errors!");
                    }
                })
        }),
        None => Ok(token), // no global sanitizers. move on.
    }?;

    // "local" (aka indexed) column sanitizers
    match column_sanitizers.get(&Some(col_num)) {
        // We don't have a local sanitizer for the specific "column", return token as is
        None => Ok(token),
        // Apply all sanitizers and return the sanitized token in the end
        Some(tst) => tst.iter().try_fold(token, |acc, transitizer| {
            transitizer
                .transitize(&acc)
                // Supply more error context
                .map_err(|e| {
                    if let PattiCsvError::Sanitize(se) = e {
                        PattiCsvError::Sanitize(SanitizeError::extend(
                            se,
                            Some(format!(
                                " Error in/from local sanitizer: {}.",
                                &transitizer.get_info(),
                            )),
                            Some(line_num),
                            Some(col_num),
                        ))
                    } else {
                        panic!("If we end up here, we mixed errors!");
                    }
                })
        }),
    }
}

pub fn sanitize_tokenizer_iter_res(
    line_number: usize,
    line_tokens: VecDeque<String>,
    column_transitizers: &Option<HashMap<Option<usize>, TransformSanitizeTokens>>,
) -> Result<VecDeque<String>> {
    match column_transitizers {
        None => Ok(line_tokens),
        Some(ct) => {
            let mut ret: VecDeque<String> = VecDeque::with_capacity(line_tokens.len());
            // Apply sanitization and escaping / enclosure
            for (i, token) in line_tokens.into_iter().enumerate() {
                // On first glance, borrowing and transforming inplace is smarter, however, all the transformations
                // we use, allocate a new String anyway, so it doesn't make much sense.
                ret.push_back(sanitize_token(token, ct, line_number, i)?);
            }
            Ok(ret)
        }
    }
}

#[cfg(test)]
mod tests {
    use venum::value::Value;
    use venum::value_type::ValueType;

    use crate::transform_sanitize_token::*;

    use super::*;

    // Supply both, header tokens and info via typings. Typings must get precedence.
    #[test]
    fn test_build_layout_template_w_typings_precedence() {
        let header_tokens: &VecDeque<String> =
            &VecDeque::from(vec![String::from("header1-from-header-tokens")]); // second prio for header name
        let column_typing: &Vec<TypeColumnEntry> = &vec![TypeColumnEntry::new(
            Some(String::from("header1-from-column-typings")), // first prio for header name (used here!)
            ValueType::String,
        )];
        let res = build_layout_template(Some(header_tokens), column_typing).unwrap();

        let mut exp = DataCellRow::new();
        exp.push(DataCell::new(
            ValueType::String,
            "header1-from-column-typings".into(),
            0,
            Value::None,
        ));

        assert_eq!(exp, res);
    }

    // Supply info via header line only.
    #[test]
    fn test_build_layout_template_w_header_from_header_tokens() {
        let header_tokens: &VecDeque<String> =
            &VecDeque::from(vec![String::from("header1-from-header-tokens")]); // second prio for header name (used here!)
        let column_typing: &Vec<TypeColumnEntry> = &vec![TypeColumnEntry::new(
            None, // first prio for header name
            ValueType::String,
        )];
        let res = build_layout_template(Some(header_tokens), column_typing).unwrap();

        let mut exp = DataCellRow::new();
        exp.push(DataCell::new(
            ValueType::String,
            "header1-from-header-tokens".into(),
            0,
            Value::None,
        ));

        assert_eq!(exp, res);
    }

    #[test]
    fn test_build_layout_template_w_header_from_typings() {
        let column_typing: &Vec<TypeColumnEntry> = &vec![TypeColumnEntry::new(
            Some(String::from("header1-from-column-typings")), // first prio for header name (used here!)
            ValueType::String,
        )];
        let res = build_layout_template(None, column_typing).unwrap();

        let mut exp = DataCellRow::new();
        exp.push(DataCell::new(
            ValueType::String,
            "header1-from-column-typings".into(),
            0,
            Value::None,
        ));

        assert_eq!(exp, res);
    }

    #[test]
    #[should_panic(expected = "No header provided for column#0")]
    fn test_build_layout_template_w_header_err_no_header_info() {
        let header_tokens: &VecDeque<String> = &VecDeque::new(); // second prio for header name
        let column_typing: &Vec<TypeColumnEntry> = &vec![TypeColumnEntry::new(
            None, // first prio for header name
            ValueType::String,
        )];
        build_layout_template(Some(header_tokens), column_typing).unwrap();
        // errors
    }

    // Neither header tokens, nor headers via typings are supplied. Fallback to indices.
    #[test]
    fn test_build_layout_template_no_info_fallback_to_index() {
        let column_typing: &Vec<TypeColumnEntry> = &vec![TypeColumnEntry::new(
            None, // first prio for header name
            ValueType::String,
        )];
        let res = build_layout_template(None, column_typing).unwrap();

        let mut exp = DataCellRow::new();
        exp.push(DataCell::new(ValueType::String, "0".into(), 0, Value::None)); // fallback to index as header "name" (used here!)

        assert_eq!(exp, res);
    }

    #[test]
    fn test_sanitize_token_global() {
        let mut san_hm: HashMap<Option<usize>, TransformSanitizeTokens> = HashMap::with_capacity(1);
        san_hm.insert(
            None,
            vec![
                Box::new(TrimTrailing),
                Box::new(ReplaceWith {
                    from: String::from("o"),
                    to: String::from("u"),
                }),
                Box::new(ToUppercase),
            ],
        );

        let res = sanitize_token(String::from("foobar   "), &san_hm, 112, 3).unwrap();
        assert_eq!(String::from("FUUBAR"), res);
    }

    #[test]
    fn test_sanitize_token_local() {
        let mut san_hm: HashMap<Option<usize>, TransformSanitizeTokens> = HashMap::with_capacity(1);
        san_hm.insert(
            Some(0),
            vec![Box::new(RegexTake::new("(\\d+\\.\\d+).*").unwrap())],
        );

        let res = sanitize_token(String::from("10.00 (CHF)"), &san_hm, 112, 0).unwrap();
        assert_eq!(String::from("10.00"), res);
    }

    #[test]
    #[should_panic(
        expected = "Sanitize(SanitizeError { msg: \"No captures, but we need exactly one. Error in/from global sanitizer: RegexTake { regex: "
    )]
    fn test_sanitize_token_global_err() {
        let mut san_hm: HashMap<Option<usize>, TransformSanitizeTokens> = HashMap::with_capacity(1);
        san_hm.insert(
            None,
            vec![Box::new(RegexTake::new("(\\d+\\.\\d+).*").unwrap())],
        );

        sanitize_token(String::from("10 (CHF)"), &san_hm, 112, 3).unwrap();
    }

    #[test]
    #[should_panic(
        expected = "Sanitize(SanitizeError { msg: \"No captures, but we need exactly one. Error in/from local sanitizer: RegexTake { regex: "
    )]
    fn test_sanitize_token_local_err() {
        let mut san_hm: HashMap<Option<usize>, TransformSanitizeTokens> = HashMap::with_capacity(1);
        san_hm.insert(
            Some(0),
            vec![Box::new(RegexTake::new("(\\d+\\.\\d+).*").unwrap())],
        );

        sanitize_token(String::from("10 (CHF)"), &san_hm, 112, 0).unwrap();
    }
}
