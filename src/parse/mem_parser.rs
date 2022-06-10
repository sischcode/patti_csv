// use std::io::BufRead;

// use super::{
//     config::DsvParserConfig,
//     line_tokenizer::{DelimitedLineTokenizer, DelimitedLineTokenizerStats},
//     parser_common::{build_imf_skeleton, build_imf_skeleton_w_header, sanitize_tokenizer_iter_res},
// };
// use crate::data::{imf::IMF, imf_value::Value};
// use crate::errors::{PattiCsvError, Result};

// pub struct DsvMemParser {}

// impl DsvMemParser {
//     pub fn parse_all<R>(
//         config: &DsvParserConfig,
//         input: &mut R,
//     ) -> Result<(IMF, DelimitedLineTokenizerStats)>
//     where
//         R: BufRead,
//     {
//         let mut imf_result;

//         let mut dlt_iter = DelimitedLineTokenizer::custom(
//             &config.parser_opts.lines,
//             input,
//             config.parser_opts.separator_char,
//             config.parser_opts.enclosure_char,
//         )
//         .into_iter();

//         // Special case for first line. We create a skeleton with or without supplied headers
//         if let Some((_line_number, first_result)) = dlt_iter.next() {
//             let (first_line_tokens, _stats) = first_result?;
//             if config.parser_opts.first_line_is_header {
//                 imf_result = build_imf_skeleton_w_header(&first_line_tokens, &config.type_columns)?;
//             } else {
//                 imf_result = build_imf_skeleton(&config.type_columns);
//             }
//         } else {
//             return Err(PattiCsvError::Generic {
//                 msg: "there was nothing to parse (i.e. no line)".into(),
//             });
//         }

//         let mut ret_stats: DelimitedLineTokenizerStats = DelimitedLineTokenizerStats::new();
//         while let Some((line_number, line_tokens_result)) = dlt_iter.next() {
//             let (line_tokens_result_vec, stats) = line_tokens_result?;
//             ret_stats = stats;
//             let sanitized_tokens =
//                 sanitize_tokenizer_iter_res(config, (line_number, line_tokens_result_vec))?;

//             let mut iter = imf_result.columns.iter_mut().enumerate();
//             while let Some((i, col)) = iter.next() {
//                 let curr_token = sanitized_tokens.get(i).unwrap();
//                 col.push(Value::from_string_with_templ(
//                     curr_token.clone(),
//                     &col.imf_type,
//                 )?);
//             }
//         }
//         Ok((imf_result, ret_stats))
//     }
// }
