use std::{collections::HashMap, io::Read};

use venum::{value::Value, value_type::ValueType};
use venum_tds::{data_cell::DataCell, data_cell_row::DataCellRow};

use crate::{
    errors::{PattiCsvError, Result},
    line_tokenizer::{
        DelimitedLineTokenizer, DelimitedLineTokenizerIter, DelimitedLineTokenizerStats,
    },
    parser_common::{build_layout_template, sanitize_tokenizer_iter_res},
    parser_config::{TypeColumnEntry, VecOfTokenTransitizers},
    skip_take_lines::SkipTakeLines,
};

#[derive(Debug)]
pub struct PattiCsvParser {
    pub first_data_line_is_header: bool,
    dlt: DelimitedLineTokenizer,
    // This means:
    // a) if the first Option is None, we simply don't have transitizers.
    // b) if the second Option is None, this means we have transitizers that apply to all columns,
    //    not just a specific one. (i.e. this is the "global" option. Everything is applied "globally")
    column_transitizers: Option<HashMap<Option<usize>, VecOfTokenTransitizers>>,
    column_typings: Vec<TypeColumnEntry>,
}

impl PattiCsvParser {
    pub fn builder() -> PattiCsvParserBuilder {
        PattiCsvParserBuilder::new()
    }
    pub fn parse_iter<'pars, 'rd, R: Read>(
        &'pars self,
        data: &'rd mut R,
    ) -> PattiCsvParserIterator<'pars, 'rd, R> {
        PattiCsvParserIterator::new(self, self.dlt.tokenize_iter(data))
    }
}

pub struct PattiCsvParserBuilder {
    separator_char: Option<char>,
    enclosure_char: Option<char>,
    first_data_line_is_header: bool,
    skip_take_lines_fns: Option<Vec<Box<dyn SkipTakeLines + Send + Sync>>>,
    save_skipped_lines: bool,
    column_transitizers: Option<HashMap<Option<usize>, VecOfTokenTransitizers>>,
    column_typings: Option<Vec<TypeColumnEntry>>,
}

impl PattiCsvParserBuilder {
    pub fn new() -> Self {
        Self {
            separator_char: None,
            enclosure_char: Some('"'),
            first_data_line_is_header: true,
            save_skipped_lines: false,
            skip_take_lines_fns: None,
            column_transitizers: None,
            column_typings: None,
        }
    }

    pub fn csv() -> Self {
        Self {
            separator_char: Some(','),
            enclosure_char: Some('"'),
            first_data_line_is_header: true,
            save_skipped_lines: false,
            skip_take_lines_fns: None,
            column_transitizers: None,
            column_typings: None,
        }
    }

    pub fn tsv() -> Self {
        Self {
            separator_char: Some('\t'),
            enclosure_char: None,
            first_data_line_is_header: false,
            save_skipped_lines: false,
            skip_take_lines_fns: None,
            column_transitizers: None,
            column_typings: None,
        }
    }

    pub fn separator_char(mut self, c: char) -> PattiCsvParserBuilder {
        self.separator_char = Some(c);
        self
    }

    pub fn enclosure_char(mut self, c: Option<char>) -> PattiCsvParserBuilder {
        self.enclosure_char = c;
        self
    }

    pub fn first_data_line_is_header(mut self, b: bool) -> PattiCsvParserBuilder {
        self.first_data_line_is_header = b;
        self
    }

    pub fn skip_take_lines_fns(
        mut self,
        s: Vec<Box<dyn SkipTakeLines + Send + Sync>>,
    ) -> PattiCsvParserBuilder {
        self.skip_take_lines_fns = Some(s);
        self
    }

    pub fn save_skipped_lines(mut self, b: bool) -> PattiCsvParserBuilder {
        self.save_skipped_lines = b;
        self
    }

    pub fn column_transitizers(
        mut self,
        t: HashMap<Option<usize>, VecOfTokenTransitizers>,
    ) -> PattiCsvParserBuilder {
        self.column_transitizers = Some(t);
        self
    }

    pub fn column_typings(mut self, t: Vec<TypeColumnEntry>) -> PattiCsvParserBuilder {
        self.column_typings = Some(t);
        self
    }

    pub fn stringly_type_columns(mut self, num_columns: usize) -> PattiCsvParserBuilder {
        self.column_typings = Some(
            (0..num_columns)
                .into_iter()
                .map(|_| TypeColumnEntry::new(None, ValueType::String))
                .collect(),
        );
        self
    }

    pub fn build(mut self) -> Result<PattiCsvParser> {
        if self.column_typings.is_none() {
            return Err(PattiCsvError::Generic {
                msg: String::from("mandatory 'column typings' are not set! (None)"),
            });
        }
        if self.column_typings.is_some() && self.column_typings.as_ref().unwrap().is_empty() {
            return Err(PattiCsvError::Generic {
                msg: String::from("mandatory 'column typings' are not set! (Empty vec)"),
            });
        }
        if self.separator_char.is_none() {
            return Err(PattiCsvError::Generic {
                msg: String::from("mandatory 'separator character' is not set! (use the convenience functions '::csv()' or '::tsv()' or set the separator character manually)"),
            });
        }

        Ok(PattiCsvParser {
            first_data_line_is_header: self.first_data_line_is_header,
            column_transitizers: std::mem::take(&mut self.column_transitizers),
            column_typings: std::mem::take(&mut self.column_typings.unwrap()), // checked above!
            dlt: DelimitedLineTokenizer::new(
                self.separator_char.unwrap(), // checked above!
                self.enclosure_char,
                std::mem::take(&mut self.skip_take_lines_fns),
                self.save_skipped_lines,
            ),
        })
    }
}

pub struct PattiCsvParserIterator<'pars, 'rd, R: Read> {
    parser: &'pars PattiCsvParser,
    dlt_iter: DelimitedLineTokenizerIter<'pars, 'rd, R>,
    column_layout_template: DataCellRow,
}

impl<'pars, 'rd, R: Read> PattiCsvParserIterator<'pars, 'rd, R> {
    fn new(
        parser: &'pars PattiCsvParser,
        dlt_iter: DelimitedLineTokenizerIter<'pars, 'rd, R>,
    ) -> Self {
        Self {
            parser,
            dlt_iter,
            column_layout_template: DataCellRow::default(),
        }
    }
    pub fn get_stats(&self) -> &DelimitedLineTokenizerStats {
        self.dlt_iter.get_stats()
    }
}

impl<'pars, 'rd, R: Read> Iterator for PattiCsvParserIterator<'pars, 'rd, R> {
    type Item = Result<DataCellRow>;

    fn next(&mut self) -> Option<Self::Item> {
        // .next() yields "Option<Result<(Vec<String>, DelimitedLineTokenizerStats)>>".
        // We early "return" a None (i.e. end of parsing) through the ?, then we check for an error inside the Some(Result)
        let dlt_iter_res_vec = match self.dlt_iter.next()? {
            // returns a: Option<Result<(Vec<String>, DelimitedLineTokenizerStats)>>
            Err(e) => return Some(Err(e)),
            Ok(dlt_iter_res) => dlt_iter_res,
        };

        // Special case for the first line, which might be a header line and must be treated differently either way. This is only run once!
        if self
            .dlt_iter
            .get_stats()
            .is_at_first_unskipped_line_to_parse()
        {
            // Sanity check columns (lengths)
            let len_typings = self.parser.column_typings.len();
            let len_data = dlt_iter_res_vec.len();

            if len_typings != len_data {
                return Some(Err(PattiCsvError::ConfigError { msg: format!("Column typings provided, but length {} differs from actual length of data with num columns {}", len_typings, len_data) }));
            }

            // Set the correct headers in our template, i.e. make a column layout template, then return the data as the first line.
            if self.parser.first_data_line_is_header {
                self.column_layout_template = match build_layout_template(
                    Some(&dlt_iter_res_vec),
                    &self.parser.column_typings,
                ) {
                    Ok(v) => v,
                    Err(e) => return Some(Err(e)),
                };

                // We hardcode the datatype to ValueName::String for the header line.
                let mut csv_header_data_cell_row: DataCellRow =
                    DataCellRow::with_capacity(len_data);
                dlt_iter_res_vec.into_iter().enumerate().for_each(|(i, _)| {
                    // We have set the correct header-name above anyway, we can just use it here!
                    let header_name = &self
                        .column_layout_template
                        .0 // TODO: is there a way we don't need to rely on the underlying vec?
                        .get(i)
                        .unwrap() // we're sure we have something here! We set it above!
                        .name;

                    // TODO: do we want transitization on the headers!?

                    let new_csv_cell =
                        DataCell::new(header_name.clone(), i, header_name.clone().into())
                            .expect("data is never None, so the type_info can always be inferred from data correctly");
                    csv_header_data_cell_row.push(new_csv_cell);
                });
                return Some(Ok(csv_header_data_cell_row));
            } else {
                // In this case, the first line is actual data, meaning, we first need to build the structure, without parsing and setting the headers.
                // We do not(!) return this immediately as the first line, since we must first sanitize and then type the data.
                self.column_layout_template =
                    match build_layout_template(None, &self.parser.column_typings) {
                        Ok(v) => v,
                        Err(e) => return Some(Err(e)),
                    };
            }
        }

        // --------------------------------------------------------------------------------------------------------------------------------
        // ------------------------------------------------ Handle data rows --------------------------------------------------------------
        // --------------------------------------------------------------------------------------------------------------------------------
        let mut row_data: DataCellRow = self.column_layout_template.clone();

        let mut sanitized_tokens = match sanitize_tokenizer_iter_res(
            self.dlt_iter.get_stats().curr_line_num,
            dlt_iter_res_vec,
            &self.parser.column_transitizers,
        ) {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };

        let col_iter = row_data.0.iter_mut().enumerate(); // TODO: is there a way we don't need to rely on the underlying vec?
        for (i, cell) in col_iter {
            // We can safely unwrap here and be sure we won't have an illegal index access, because, above:
            // a) if we have no typings, we use the same length (from the tokens/data) to build them, and ...
            // b) if we have typings, we check against the length of the tokens/data, and...
            // ...subsequently we build the column layout template from the typings, AND this layout template is then used (as a clone) here, as the rows_data.
            // NOTE: Tried it with unsafe { ...get_unchecked(i) } but could not measure a significant speed improvement.
            let curr_token = sanitized_tokens.pop_front().unwrap();
            let curr_typing = self.parser.column_typings.get(i).unwrap();

            // Special short-cut cases for Empty Strings, and String -> String "conversion". I.e. we don't have to do anything.
            if curr_token.is_empty() {
                cell.data = Value::None;
            } else if curr_typing.target_type == ValueType::String
                && (curr_typing.map_to_none.is_none()
                    || curr_typing.map_to_none.as_ref().unwrap().is_empty())
            {
                cell.data = Value::String(curr_token);
            } else {
                cell.data = match Value::from_str_and_type_with_chrono_pattern_with_none_map(
                    &curr_token,
                    &cell.dtype,
                    curr_typing.chrono_pattern.as_deref(),
                    curr_typing
                        .map_to_none
                        .as_ref()
                        .map(|e| e.iter().map(|ie| ie.as_str()).collect()), // TODO we really should be using a Vec<&str> here?
                ) {
                    Ok(v) => v,
                    Err(e) => {
                        return Some(Err(PattiCsvError::Generic {
                            msg: format!(
                                "{:?}; line: {}; column: {}; header: {}",
                                e,
                                &self.dlt_iter.get_stats().curr_line_num,
                                &i,
                                &row_data.0.get(i).unwrap().get_name()
                            ),
                        }))
                    }
                };
            }
        }
        Some(Ok(row_data))
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;

    use super::*;

    use crate::{skip_take_lines::*, transform_sanitize_token::*};

    pub mod iterating_parser_builder {
        use super::*;

        #[test]
        fn test_iterating_parser_builder_all_opts() {
            let mut transitizers: HashMap<Option<usize>, VecOfTokenTransitizers> =
                HashMap::with_capacity(2);
            transitizers.insert(None, vec![Box::new(ToLowercase)]);
            transitizers.insert(Some(0), vec![Box::new(TrimAll)]);

            let parser_builder = PattiCsvParserBuilder::new()
                .separator_char(';')
                .enclosure_char(Some('\''))
                .first_data_line_is_header(false)
                .skip_take_lines_fns(vec![Box::new(SkipLinesStartingWith::new(""))])
                .save_skipped_lines(true)
                .column_typings(vec![
                    TypeColumnEntry::new(None, ValueType::Int32),
                    TypeColumnEntry::new(None, ValueType::String),
                    TypeColumnEntry::new(None, ValueType::Bool),
                ])
                .column_transitizers(transitizers);

            assert_eq!(Some(';'), parser_builder.separator_char);
            assert_eq!(Some('\''), parser_builder.enclosure_char);
            assert_eq!(false, parser_builder.first_data_line_is_header);
            assert_eq!(1, parser_builder.skip_take_lines_fns.unwrap().len());
            assert_eq!(true, parser_builder.save_skipped_lines);
            assert_eq!(3, parser_builder.column_typings.unwrap().len());
            assert_eq!(false, parser_builder.column_transitizers.is_none());
            assert_eq!(2, parser_builder.column_transitizers.unwrap().len());
        }

        #[test]
        fn test_iterating_parser_builder_defaults_csv() {
            let parser_builder = PattiCsvParserBuilder::csv().column_typings(vec![]);

            assert_eq!(Some(','), parser_builder.separator_char);
            assert_eq!(Some('"'), parser_builder.enclosure_char);
            assert_eq!(true, parser_builder.first_data_line_is_header);
            assert!(parser_builder.skip_take_lines_fns.is_none());
            assert_eq!(false, parser_builder.save_skipped_lines);
            assert!(parser_builder.column_transitizers.is_none());
        }

        #[test]
        fn test_iterating_parser_builder_defaults_tsv() {
            let parser_builder = PattiCsvParserBuilder::tsv().column_typings(vec![]);

            assert_eq!(Some('\t'), parser_builder.separator_char);
            assert_eq!(None, parser_builder.enclosure_char);
            assert_eq!(false, parser_builder.first_data_line_is_header);
            assert!(parser_builder.skip_take_lines_fns.is_none());
            assert_eq!(false, parser_builder.save_skipped_lines);
            assert!(parser_builder.column_transitizers.is_none());
        }

        #[test]
        #[should_panic(
            expected = "Generic { msg: \"mandatory 'column typings' are not set! (None)\" }"
        )]
        fn patti_csv_parser_from_patti_csv_parser_builder_err_no_column_typings() {
            PattiCsvParserBuilder::new()
                .separator_char(',')
                .enclosure_char(Some('"'))
                .first_data_line_is_header(true)
                .build()
                .unwrap();
        }

        #[test]
        #[should_panic(
            expected = "Generic { msg: \"mandatory 'column typings' are not set! (Empty vec)\" }"
        )]
        fn patti_csv_parser_from_patti_csv_parser_builder_err_empty_column_typings() {
            PattiCsvParserBuilder::new()
                .separator_char(',')
                .column_typings(vec![])
                .build()
                .unwrap();
        }

        #[test]
        #[should_panic(
            expected = "Generic { msg: \"mandatory 'separator character' is not set! (use the convenience functions '::csv()' or '::tsv()' or set the separator character manually)\" }"
        )]
        fn patti_csv_parser_from_patti_csv_parser_builder_err_no_separator_char() {
            PattiCsvParserBuilder::new()
                .column_typings(vec![TypeColumnEntry::new(None, ValueType::Bool)])
                .build()
                .unwrap();
        }
    }

    #[test]
    fn parse_with_custom_parser() {
        let mut test_data_cursor = std::io::Cursor::new("c1;c2;c3;c4;c5\n 1 ;'BaR';true;null;");

        let mut transitizers: HashMap<Option<usize>, VecOfTokenTransitizers> = HashMap::new();
        transitizers.insert(None, vec![Box::new(ToLowercase)]);
        transitizers.insert(Some(0), vec![Box::new(TrimAll)]);

        let parser = PattiCsvParserBuilder::new()
            .separator_char(';')
            .enclosure_char(Some('\''))
            .first_data_line_is_header(true)
            .column_typings(vec![
                TypeColumnEntry::new(None, ValueType::Int32),
                TypeColumnEntry::new(Some(String::from("col2")), ValueType::String),
                TypeColumnEntry::new(Some(String::from("col3")), ValueType::Bool),
                TypeColumnEntry::new_with_map_to_none(
                    Some(String::from("col4")),
                    ValueType::String,
                    vec![String::from("null")],
                ),
                TypeColumnEntry::new(None, ValueType::Int32), // Empty String will automatically(!) be mapped to Value::None!
            ])
            .column_transitizers(transitizers)
            .build()
            .unwrap();

        let mut iter = parser.parse_iter(&mut test_data_cursor);
        let headers = iter.next().unwrap().unwrap();
        let line_1 = iter.next().unwrap().unwrap();

        // println!("{:?}", headers);
        // println!("{:?}", line_1);

        assert_eq!(
            DataCellRow {
                0: vec![
                    DataCell {
                        dtype: ValueType::String,
                        idx: 0,
                        name: String::from("c1"),
                        data: Value::String(String::from("c1"))
                    },
                    DataCell {
                        dtype: ValueType::String,
                        idx: 1,
                        name: String::from("col2"),
                        data: Value::String(String::from("col2"))
                    },
                    DataCell {
                        dtype: ValueType::String,
                        idx: 2,
                        name: String::from("col3"),
                        data: Value::String(String::from("col3"))
                    },
                    DataCell {
                        dtype: ValueType::String,
                        idx: 3,
                        name: String::from("col4"),
                        data: Value::String(String::from("col4"))
                    },
                    DataCell {
                        dtype: ValueType::String,
                        idx: 4,
                        name: String::from("c5"),
                        data: Value::String(String::from("c5"))
                    },
                ]
            },
            headers
        );

        assert_eq!(
            DataCellRow {
                0: vec![
                    DataCell {
                        dtype: ValueType::Int32,
                        idx: 0,
                        name: String::from("c1"),
                        data: Value::Int32(1)
                    },
                    DataCell {
                        dtype: ValueType::String,
                        idx: 1,
                        name: String::from("col2"),
                        data: Value::String(String::from("bar"))
                    },
                    DataCell {
                        dtype: ValueType::Bool,
                        idx: 2,
                        name: String::from("col3"),
                        data: Value::Bool(true)
                    },
                    DataCell {
                        dtype: ValueType::String,
                        idx: 3,
                        name: String::from("col4"),
                        data: Value::None
                    },
                    DataCell {
                        dtype: ValueType::Int32,
                        idx: 4,
                        name: String::from("c5"),
                        data: Value::None
                    },
                ]
            },
            line_1
        )
    }

    #[test]
    fn parse_with_csv_parser_stringly_typed() {
        // <header>
        //  1 -> "1", "BaR" -> "bar", true -> "true", null -> "null", <empty-string> -> <empty-string>

        let mut test_data_cursor = std::io::Cursor::new("c1,c2,c3,c4,c5\n 1 ,\"BaR\",true,null,");

        let mut transitizers: HashMap<Option<usize>, VecOfTokenTransitizers> = HashMap::new();
        transitizers.insert(None, vec![Box::new(ToLowercase)]);
        transitizers.insert(Some(0), vec![Box::new(TrimAll)]);

        let parser = PattiCsvParserBuilder::csv()
            .first_data_line_is_header(true)
            .stringly_type_columns(5)
            .column_transitizers(transitizers)
            .build()
            .unwrap();

        let mut iter = parser.parse_iter(&mut test_data_cursor);
        let headers = iter.next().unwrap().unwrap();
        let line_1 = iter.next().unwrap().unwrap();

        // println!("{:?}", headers);
        // println!("{:?}", line_1);

        assert_eq!(
            DataCellRow {
                0: vec![
                    DataCell {
                        dtype: ValueType::String,
                        idx: 0,
                        name: String::from("c1"),
                        data: Value::String(String::from("c1"))
                    },
                    DataCell {
                        dtype: ValueType::String,
                        idx: 1,
                        name: String::from("c2"),
                        data: Value::String(String::from("c2"))
                    },
                    DataCell {
                        dtype: ValueType::String,
                        idx: 2,
                        name: String::from("c3"),
                        data: Value::String(String::from("c3"))
                    },
                    DataCell {
                        dtype: ValueType::String,
                        idx: 3,
                        name: String::from("c4"),
                        data: Value::String(String::from("c4"))
                    },
                    DataCell {
                        dtype: ValueType::String,
                        idx: 4,
                        name: String::from("c5"),
                        data: Value::String(String::from("c5"))
                    },
                ]
            },
            headers
        );

        assert_eq!(
            DataCellRow {
                0: vec![
                    DataCell {
                        dtype: ValueType::String,
                        idx: 0,
                        name: String::from("c1"),
                        data: Value::String(String::from("1"))
                    },
                    DataCell {
                        dtype: ValueType::String,
                        idx: 1,
                        name: String::from("c2"),
                        data: Value::String(String::from("bar"))
                    },
                    DataCell {
                        dtype: ValueType::String,
                        idx: 2,
                        name: String::from("c3"),
                        data: Value::String(String::from("true"))
                    },
                    DataCell {
                        dtype: ValueType::String,
                        idx: 3,
                        name: String::from("c4"),
                        data: Value::String(String::from("null")) // we do NOT map "special" strings like "null" automatically
                    },
                    DataCell {
                        dtype: ValueType::String,
                        idx: 4,
                        name: String::from("c5"),
                        data: Value::None
                    },
                ]
            },
            line_1
        )
    }

    // TODO
    #[test]
    fn test_parser_skip_comments_and_summation_lines() {
        // <drop first two lines>
        // <header>
        //  1 -> "1", "BaR" -> "bar", true -> "true", <empty-string> -> <empty-string>
        // <drop last line>
        let mut test_data_cursor = std::io::Cursor::new("# shitty comment line!\n# shitty comment line 2\nc1,c2,c3,c4\n 1 ,\"BaR\",true,\na, shitty, summation, line");

        let mut transitizers: HashMap<Option<usize>, VecOfTokenTransitizers> = HashMap::new();
        transitizers.insert(None, vec![Box::new(ToLowercase)]);
        transitizers.insert(Some(0), vec![Box::new(TrimAll)]);

        let parser = PattiCsvParserBuilder::csv()
            .first_data_line_is_header(true)
            .stringly_type_columns(4)
            .skip_take_lines_fns(vec![
                Box::new(SkipLinesStartingWith::new("#")),
                Box::new(SkipLinesStartingWith::new("a, shitty")),
            ])
            .column_transitizers(transitizers)
            .build()
            .unwrap();

        let mut iter = parser.parse_iter(&mut test_data_cursor);
        let headers = iter.next().unwrap().unwrap();
        let line_1 = iter.next().unwrap().unwrap();

        let header_string = headers
            .into_iter()
            .map(|e| String::try_from(e.get_data()).unwrap())
            .collect::<Vec<_>>()
            .join(",");

        let line_1_string = line_1
            .into_iter()
            .map(|e| String::try_from(e.get_data()).unwrap())
            .collect::<Vec<_>>()
            .join(",");

        assert_eq!(String::from("c1,c2,c3,c4"), header_string);
        assert_eq!(String::from("1,bar,true,"), line_1_string);
        assert!(iter.next().is_none());
    }

    // TODO
    #[test]
    fn test_parser_skip_comments_and_summation_lines_save_skipped() {
        // <drop first two lines>
        // <header>
        //  1 -> "1", "BaR" -> "bar", true -> "true", <empty-string> -> <empty-string>
        // <drop last line>
        let mut test_data_cursor = std::io::Cursor::new("# shitty comment line!\n# shitty comment line 2\nc1,c2,c3,c4\n 1 ,\"BaR\",true,\na, shitty, summation, line");

        let mut transitizers: HashMap<Option<usize>, VecOfTokenTransitizers> = HashMap::new();
        transitizers.insert(None, vec![Box::new(ToLowercase)]);
        transitizers.insert(Some(0), vec![Box::new(TrimAll)]);

        let parser = PattiCsvParserBuilder::csv()
            .first_data_line_is_header(true)
            .stringly_type_columns(4)
            .skip_take_lines_fns(vec![
                Box::new(SkipLinesStartingWith::new("#")),
                Box::new(SkipLinesStartingWith::new("a, shitty")),
            ])
            .save_skipped_lines(true)
            .column_transitizers(transitizers)
            .build()
            .unwrap();

        let mut iter = parser.parse_iter(&mut test_data_cursor);

        while let Some(_) = iter.next() {}

        assert_eq!(2, *&iter.get_stats().num_lines_tokenized);
        assert_eq!(3, *&iter.get_stats().skipped_lines.len());
    }

    #[test]
    fn test_parser_date_default_patterns() {
        let mut test_data_cursor =
            std::io::Cursor::new("2022-01-01,2022-02-02T12:00:00,2022-12-31T06:00:00+05:00");

        let parser = PattiCsvParserBuilder::csv()
            .first_data_line_is_header(false)
            .column_typings(vec![
                TypeColumnEntry::new(Some(String::from("col1")), ValueType::NaiveDate),
                TypeColumnEntry::new(Some(String::from("col2")), ValueType::NaiveDateTime),
                TypeColumnEntry::new(Some(String::from("col3")), ValueType::DateTime),
            ])
            .build()
            .unwrap();

        let mut parser_iter = parser.parse_iter(&mut test_data_cursor);
        let line_1 = parser_iter.next().unwrap().unwrap();

        // println!("{:?}", &line_1);

        let naive_date_val = line_1
            .get_by_name("col1")
            .unwrap()
            .get_data()
            .try_convert_to(&ValueType::String)
            .unwrap();

        assert_eq!(
            String::from("2022-01-01"),
            String::try_from(naive_date_val).unwrap()
        );

        let naive_date_time_val = line_1
            .get_by_name("col2")
            .unwrap()
            .get_data()
            .try_convert_to(&ValueType::String)
            .unwrap();

        assert_eq!(
            String::from("2022-02-02T12:00:00.000"),
            String::try_from(naive_date_time_val).unwrap()
        );

        let date_time_val = line_1
            .get_by_name("col3")
            .unwrap()
            .get_data()
            .try_convert_to(&ValueType::String)
            .unwrap();

        assert_eq!(
            String::from("2022-12-31T06:00:00.000+05:00"),
            String::try_from(date_time_val).unwrap()
        );
    }

    #[test]
    fn test_parser_date_manual_chrono_patterns() {
        let mut test_data_cursor =
            std::io::Cursor::new("01.01.2022,02.02.2022 12_00_00,20.1.2022 8:00 am +0200");

        let parser = PattiCsvParserBuilder::csv()
            .first_data_line_is_header(false)
            .column_typings(vec![
                TypeColumnEntry::new_with_chrono_pattern(
                    Some(String::from("col1")),
                    ValueType::NaiveDate,
                    String::from("%d.%m.%Y"),
                ),
                TypeColumnEntry::new_with_chrono_pattern(
                    Some(String::from("col2")),
                    ValueType::NaiveDateTime,
                    String::from("%d.%m.%Y %H_%M_%S"),
                ),
                TypeColumnEntry::new_with_chrono_pattern(
                    Some(String::from("col3")),
                    ValueType::DateTime,
                    String::from("%d.%m.%Y %H:%M %P %z"),
                ),
            ])
            .build()
            .unwrap();

        let mut iter = parser.parse_iter(&mut test_data_cursor);
        let line_1 = iter.next().unwrap().unwrap();

        let naive_date_val = line_1
            .get_by_name("col1")
            .unwrap()
            .get_data()
            .try_convert_to(&ValueType::String)
            .unwrap();

        assert_eq!(
            String::from("2022-01-01"),
            String::try_from(naive_date_val).unwrap()
        );

        let naive_date_time_val = line_1
            .get_by_name("col2")
            .unwrap()
            .get_data()
            .try_convert_to(&ValueType::String)
            .unwrap();

        assert_eq!(
            String::from("2022-02-02T12:00:00.000"),
            String::try_from(naive_date_time_val).unwrap()
        );

        let date_time_val = line_1
            .get_by_name("col3")
            .unwrap()
            .get_data()
            .try_convert_to(&ValueType::String)
            .unwrap();

        assert_eq!(
            String::from("2022-01-20T08:00:00.000+02:00"),
            String::try_from(date_time_val).unwrap()
        );
    }
}
