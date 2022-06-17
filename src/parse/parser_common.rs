use std::collections::HashMap;

use crate::{
    data::{csv_column::CsvColumn, csv_data_columns::CsvDataColumns},
    errors::{PattiCsvError, Result, SanitizeError},
};

use super::parser_config::{TransformSanitizeTokens, TypeColumnEntry};

pub fn build_csv_data_skeleton_w_header(
    header_tokens: &Vec<String>,
    column_typing: &Vec<TypeColumnEntry>,
) -> Result<CsvDataColumns> {
    let mut csv_data = CsvDataColumns::new(); // our return value

    let mut header_map = HashMap::<usize, String>::new();
    for (i, token) in header_tokens.into_iter().enumerate() {
        header_map.insert(i, token.clone());
    }

    for (idx, tce) in column_typing.into_iter().enumerate() {
        csv_data.columns.push(CsvColumn::new(
            tce.target_type.clone(),
            // Either we have a header name from the typings, or the headerline.
            // If we have no header from the typings (which is ok) and also NO
            // header from the headerline (not ok), then we need to error.
            tce.header
                .as_ref()
                .or(header_map.get(&idx))
                .ok_or(PattiCsvError::Generic {
                    msg: format!("No header provided for column#{}", idx),
                })?
                .clone(),
            idx,
        ));
    }
    Ok(csv_data)
}

pub fn build_csv_data_skeleton(column_typing: &Vec<TypeColumnEntry>) -> CsvDataColumns {
    let mut csv_data = CsvDataColumns::new(); // our return value

    for (idx, tce) in column_typing.iter().enumerate() {
        csv_data.columns.push(CsvColumn::new(
            tce.target_type.clone(),
            tce.header.as_ref().unwrap_or(&idx.to_string()).clone(), // fallback to indices as header, if no real header name is given
            idx,
        ));
    }
    csv_data
}

pub fn sanitize_token(
    token: String,
    sanitizers: &Option<HashMap<Option<usize>, TransformSanitizeTokens>>,
    line_num: usize, // for error context
    col_num: usize,  // used internally AND for error context
) -> Result<String> {
    match sanitizers {
        // We have no sanitizers, return token as is
        None => Ok(token),
        // We have sanitizers...
        Some(ref column_sanitizers) => {
            // If we have sanitizers for index=None, that means, we have global sanitizers, not bound to any index. I.e. they will always be applied.
            // Note that this strongly differs from getting None as a result of a .get on the HashMap!
            let token = match column_sanitizers.get(&None) {
                Some(tst) => tst.iter().try_fold(token, |acc, transitizer| {
                    transitizer
                        .transitize(acc) // apply filter, then yield
                        // Supply more error context
                        .map_err(|e| {
                            if let PattiCsvError::Sanitize(se) = e {
                                PattiCsvError::Sanitize(SanitizeError::extend(
                                    se,
                                    Some(String::from(" Error during global sanitization.")), // TODO: better debug/err info here about the sanitizer/type
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
                        .transitize(acc)
                        // Supply more error context
                        .map_err(|e| {
                            if let PattiCsvError::Sanitize(se) = e {
                                PattiCsvError::Sanitize(SanitizeError::extend(
                                    se,
                                    Some(" Error in local sanitizer".to_string()), // TODO: better debug/err info here about the sanitizer/type
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
    }
}

pub fn sanitize_tokenizer_iter_res(
    line_number: usize,
    line_tokens: Vec<String>,
    column_transitizers: &Option<HashMap<Option<usize>, TransformSanitizeTokens>>,
) -> Result<Vec<String>> {
    let mut ret: Vec<String> = Vec::with_capacity(line_tokens.len());

    // Apply sanitization and escaping / enclosure
    for (i, token) in line_tokens.into_iter().enumerate() {
        let sanitized_token = sanitize_token(token, &column_transitizers, line_number, i)?;
        ret.push(sanitized_token);
    }
    Ok(ret)
}

#[cfg(test)]
mod tests {

    use crate::data::csv_value::CsvValue;

    use super::*;
    #[test]
    fn test_build_csv_data_skeleton_w_header_from_header_tokens() {
        let header_tokens: &Vec<String> = &vec![String::from("header1-from-header-tokens")]; // second prio for header name
        let column_typing: &Vec<TypeColumnEntry> = &vec![TypeColumnEntry {
            header: Some(String::from("header1-from-column-typings")), // first prio for header name (used here!)
            target_type: CsvValue::string_default(),
        }];
        let res = build_csv_data_skeleton_w_header(header_tokens, column_typing).unwrap();

        let mut exp = CsvDataColumns::new();
        exp.add_col(CsvColumn::new(
            CsvValue::string_default(),
            "header1-from-column-typings".into(),
            0,
        ));

        assert_eq!(exp, res);
    }

    #[test]
    fn test_build_csv_data_skeleton_w_header_from_column_typings() {
        let header_tokens: &Vec<String> = &vec![String::from("header1-from-header-tokens")]; // second prio for header name (used here!)
        let column_typing: &Vec<TypeColumnEntry> = &vec![TypeColumnEntry {
            header: None, // first prio for header name
            target_type: CsvValue::string_default(),
        }];
        let res = build_csv_data_skeleton_w_header(header_tokens, column_typing).unwrap();

        let mut exp = CsvDataColumns::new();
        exp.add_col(CsvColumn::new(
            CsvValue::string_default(),
            "header1-from-header-tokens".into(),
            0,
        ));

        assert_eq!(exp, res);
    }

    #[test]
    #[should_panic(expected = "No header provided for column#0")]
    fn test_build_csv_data_skeleton_w_header_err_no_header_info() {
        let header_tokens: &Vec<String> = &vec![]; // second prio for header name
        let column_typing: &Vec<TypeColumnEntry> = &vec![TypeColumnEntry {
            header: None, // first prio for header name
            target_type: CsvValue::string_default(),
        }];
        build_csv_data_skeleton_w_header(header_tokens, column_typing).unwrap();
        // errors
    }

    #[test]
    fn test_build_csv_data_skeleton_with_headers() {
        let column_typing: &Vec<TypeColumnEntry> = &vec![TypeColumnEntry {
            header: Some(String::from("header1-from-column-typings")), // first prio for header name (used here!)
            target_type: CsvValue::string_default(),
        }];
        let res = build_csv_data_skeleton(column_typing);

        let mut exp = CsvDataColumns::new();
        exp.add_col(CsvColumn::new(
            CsvValue::string_default(),
            "header1-from-column-typings".into(),
            0,
        ));

        assert_eq!(exp, res);
    }

    #[test]
    fn test_build_csv_data_skeleton_without_headers() {
        let column_typing: &Vec<TypeColumnEntry> = &vec![TypeColumnEntry {
            header: None, // first prio for header name
            target_type: CsvValue::string_default(),
        }];
        let res = build_csv_data_skeleton(column_typing);

        let mut exp = CsvDataColumns::new();
        exp.add_col(CsvColumn::new(CsvValue::string_default(), "0".into(), 0)); // fallback to index as header "name" (used here!)

        assert_eq!(exp, res);
    }
}
