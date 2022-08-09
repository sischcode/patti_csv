use std::collections::HashMap;
use std::io::Read;
use std::marker::PhantomData;

use venum::value::Value;
use venum::value_type::ValueType;
use venum_tds::data_cell::DataCell;
use venum_tds::data_cell_row::DataCellRow;

use crate::errors::{PattiCsvError, Result};
use crate::line_tokenizer::{
    DelimitedLineTokenizer, DelimitedLineTokenizerIter, DelimitedLineTokenizerStats,
};

use super::parser_common::{build_layout_template, sanitize_tokenizer_iter_res};
use super::parser_config::{TransformSanitizeTokens, TypeColumnEntry};
use super::skip_take_lines::SkipTakeLines;

pub struct PattiCsvParser<'rd, R>
where
    R: Read,
{
    first_line_is_header: bool,
    // This means:
    // a) if the first Option is None, we simply don't have transitizers.
    // b) if the second Option is None, this means we have transitizers that apply to all columns,
    //    not just a specific one. (i.e. this is the "global" option. Everything is applied "globally")
    column_transitizers: Option<HashMap<Option<usize>, TransformSanitizeTokens>>,
    column_typings: Vec<TypeColumnEntry>,
    dlt_iter: DelimitedLineTokenizerIter<'rd, R>,
}

impl<'rd, R: Read> PattiCsvParser<'rd, R> {
    pub fn builder() -> PattiCsvParserBuilder<R> {
        PattiCsvParserBuilder::new()
    }
    pub fn get_skipped_lines(&self) -> &Vec<(usize, String)> {
        self.dlt_iter.get_skipped_lines()
    }
    pub fn first_line_is_header(&self) -> bool {
        self.first_line_is_header
    }
    pub fn get_delim_char(&self) -> char {
        self.dlt_iter.get_delim_char()
    }
    pub fn get_encl_char(&self) -> Option<char> {
        self.dlt_iter.get_encl_char()
    }
}

pub struct PattiCsvParserBuilder<R>
where
    R: Read,
{
    phantom: PhantomData<R>,
    separator_char: char,
    enclosure_char: Option<char>,
    first_line_is_header: bool,
    skip_take_lines_fns: Option<Vec<Box<dyn SkipTakeLines>>>,
    save_skipped_lines: bool,
    column_transitizers: Option<HashMap<Option<usize>, TransformSanitizeTokens>>,
    mandatory_column_typings: bool,
    column_typings: Option<Vec<TypeColumnEntry>>,
}

impl<'rd, R: Read> PattiCsvParserBuilder<R> {
    pub fn new() -> Self {
        Self {
            separator_char: ',',
            enclosure_char: Some('"'),
            first_line_is_header: true,
            save_skipped_lines: false,
            skip_take_lines_fns: None,
            column_transitizers: None,
            mandatory_column_typings: false,
            column_typings: None,
            phantom: PhantomData::default(),
        }
    }
    pub fn separator_char(&mut self, c: char) -> &mut PattiCsvParserBuilder<R> {
        self.separator_char = c;
        self
    }
    pub fn enclosure_char(&mut self, c: Option<char>) -> &mut PattiCsvParserBuilder<R> {
        self.enclosure_char = c;
        self
    }
    pub fn first_line_is_header(&mut self, b: bool) -> &mut PattiCsvParserBuilder<R> {
        self.first_line_is_header = b;
        self
    }
    pub fn save_skipped_lines(&mut self, b: bool) -> &mut PattiCsvParserBuilder<R> {
        self.save_skipped_lines = b;
        self
    }
    pub fn skip_take_lines_fns(
        &mut self,
        s: Vec<Box<dyn SkipTakeLines>>,
    ) -> &mut PattiCsvParserBuilder<R> {
        self.skip_take_lines_fns = Some(s);
        self
    }
    pub fn column_transitizers(
        &mut self,
        t: HashMap<Option<usize>, TransformSanitizeTokens>,
    ) -> &mut PattiCsvParserBuilder<R> {
        self.column_transitizers = Some(t);
        self
    }
    pub fn mandatory_column_typings(&mut self, b: bool) -> &mut PattiCsvParserBuilder<R> {
        self.mandatory_column_typings = b;
        self
    }
    pub fn column_typings(&mut self, t: Vec<TypeColumnEntry>) -> &mut PattiCsvParserBuilder<R> {
        self.column_typings = Some(t);
        self
    }
    /// For simplicity sake we consume the builder. We also want the input / csv-source file here already.
    /// We accept this for know, since we have to create a new parser for every parsing action anyway since
    /// we...consume the config during creation of the parser.
    pub fn build(&mut self, input_raw_data: &'rd mut R) -> Result<PattiCsvParser<'rd, R>> {
        if self.mandatory_column_typings && self.column_typings.is_none() {
            return Err(PattiCsvError::Generic {
                msg: String::from("Column typings have been flagged mandatory but are not set!"),
            });
        }

        let column_typings: Vec<TypeColumnEntry> = if self.column_typings.is_none() {
            Vec::with_capacity(15) // we need to start somewhere, and since this is just one line of data, I'd rather over-allocate
        } else {
            std::mem::take(&mut self.column_typings).unwrap() // checked above!
        };

        Ok(PattiCsvParser {
            first_line_is_header: self.first_line_is_header,
            column_transitizers: std::mem::take(&mut self.column_transitizers),
            column_typings,
            dlt_iter: DelimitedLineTokenizer::new(
                input_raw_data,
                self.separator_char,
                self.enclosure_char,
                std::mem::take(&mut self.skip_take_lines_fns),
                self.save_skipped_lines,
            )
            .into_iter(),
        })
    }
}

impl<R: Read> Default for PattiCsvParserBuilder<R> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PattiCsvParserIterator<'rd, R: Read> {
    pcp: PattiCsvParser<'rd, R>,
    column_layout_template: DataCellRow,
}

impl<'rd, R: Read> PattiCsvParserIterator<'rd, R> {
    // For now, we clone everytime this gets called. This is not optimal, but usually only done, once, at the end.
    pub fn get_skipped_lines(&self) -> &Vec<(usize, String)> {
        self.pcp.get_skipped_lines()
    }
    pub fn get_stats(&self) -> &DelimitedLineTokenizerStats {
        self.pcp.dlt_iter.get_stats()
    }
    pub fn save_skipped_lines(&self) -> bool {
        self.pcp.dlt_iter.save_skipped_lines()
    }
    pub fn first_line_is_header(&self) -> bool {
        self.pcp.first_line_is_header()
    }
}

impl<'rd, R: Read> Iterator for PattiCsvParserIterator<'rd, R> {
    type Item = Result<DataCellRow>;

    fn next(&mut self) -> Option<Self::Item> {
        // .next() yields "Option<Result<(Vec<String>, DelimitedLineTokenizerStats)>>".
        // We early "return" a None (i.e. end of parsing) through the ?, then we check for an error inside the Some(Result)
        let dlt_iter_res_vec = match self.pcp.dlt_iter.next()? {
            // returns a: Option<Result<(Vec<String>, DelimitedLineTokenizerStats)>>
            Err(e) => return Some(Err(e)),
            Ok(dlt_iter_res) => dlt_iter_res,
        };

        // Special case for the first line, which might be a header line and must be treated differently either way. This is only run once!
        if self.pcp.dlt_iter.get_stats().is_at_first_line_to_parse() {
            // If we don't have type info for the columns, default to String for everything and also set the header name to the index.
            let len_typings = self.pcp.column_typings.len();
            let len_data = dlt_iter_res_vec.len();

            if len_typings == 0 {
                for _ in 0..len_data {
                    self.pcp
                        .column_typings
                        .push(TypeColumnEntry::new(None, ValueType::String));
                }
            } else if len_typings != len_data {
                return Some(Err(PattiCsvError::ConfigError { msg: format!("Column typings provided, but length {} differs from actual length of data with num columns {}", len_typings, len_data) }));
            }

            // Set the correct headers in our template, i.e. make a column layout template, then return the data as the first line.
            if self.pcp.first_line_is_header {
                self.column_layout_template = match build_layout_template(
                    Some(&dlt_iter_res_vec),
                    &self.pcp.column_typings,
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

                    let new_csv_cell = DataCell::new(
                        ValueType::String,
                        header_name.clone(),
                        i,
                        header_name.clone().into(),
                    );
                    csv_header_data_cell_row.push(new_csv_cell);
                });
                return Some(Ok(csv_header_data_cell_row));
            } else {
                // In this case, the first line is actual data, meaning, we first need to build the structure, without parsing and setting the headers.
                // We do not(!) return this immediately as the first line, since we must first sanitize and then type the data.
                self.column_layout_template =
                    match build_layout_template(None, &self.pcp.column_typings) {
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
            self.pcp.dlt_iter.get_stats().curr_line_num,
            dlt_iter_res_vec,
            &self.pcp.column_transitizers,
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
            let curr_typing = self.pcp.column_typings.get(i).unwrap();

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
                    curr_typing.chrono_pattern.as_ref().map(|e| e.as_str()),
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
                                &self.pcp.dlt_iter.get_stats().curr_line_num,
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

impl<'rd, R: Read> IntoIterator for PattiCsvParser<'rd, R> {
    type Item = Result<DataCellRow>;
    type IntoIter = PattiCsvParserIterator<'rd, R>;

    fn into_iter(self) -> Self::IntoIter {
        PattiCsvParserIterator {
            pcp: self,
            column_layout_template: DataCellRow::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{skip_take_lines::*, transform_sanitize_token::*};

    use super::*;

    #[test]
    fn test_iterating_parser_builder_all_opts() {
        let mut test_data_cursor = std::io::Cursor::new("");

        let mut transitizers: HashMap<Option<usize>, TransformSanitizeTokens> =
            HashMap::with_capacity(2);
        transitizers.insert(None, vec![Box::new(ToLowercase)]);
        transitizers.insert(Some(0), vec![Box::new(TrimAll)]);

        let parser = PattiCsvParserBuilder::new()
            .separator_char(';')
            .enclosure_char(Some('\''))
            .first_line_is_header(false)
            .mandatory_column_typings(true)
            .column_typings(vec![
                TypeColumnEntry::new(None, ValueType::Int32),
                TypeColumnEntry::new(None, ValueType::String),
                TypeColumnEntry::new(None, ValueType::Bool),
            ])
            .column_transitizers(transitizers)
            .build(&mut test_data_cursor)
            .unwrap();

        assert_eq!(parser.get_delim_char(), ';');
        assert_eq!(parser.get_encl_char(), Some('\''));
        assert_eq!(parser.first_line_is_header(), false);
        assert_eq!(parser.column_typings.len(), 3);
        assert_eq!(parser.column_transitizers.is_none(), false);
        assert_eq!(parser.column_transitizers.unwrap().len(), 2);
    }

    #[test]
    fn test_iterating_parser_builder_defaults() {
        let mut test_data_cursor = std::io::Cursor::new("");
        let parser = PattiCsvParserBuilder::new()
            .build(&mut test_data_cursor)
            .unwrap();

        assert_eq!(parser.get_delim_char(), ',');
        assert_eq!(parser.get_encl_char(), Some('"'));
        assert_eq!(parser.first_line_is_header(), true);
        assert_eq!(parser.column_typings.len(), 0);
        assert_eq!(parser.column_transitizers.is_none(), true);
    }

    #[test]
    fn test_parser_01() {
        let mut test_data_cursor = std::io::Cursor::new("c1;c2;c3;c4;c5\n 1 ;'BaR';true;null;");

        let mut transitizers: HashMap<Option<usize>, TransformSanitizeTokens> = HashMap::new();
        transitizers.insert(None, vec![Box::new(ToLowercase)]);
        transitizers.insert(Some(0), vec![Box::new(TrimAll)]);

        let parser = PattiCsvParserBuilder::new()
            .separator_char(';')
            .enclosure_char(Some('\''))
            .first_line_is_header(true)
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
            .build(&mut test_data_cursor)
            .unwrap();

        let mut iter = parser.into_iter();
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
    fn test_parser_02() {
        let mut test_data_cursor = std::io::Cursor::new("c1,c2,c3,c4,c5\n 1 ,\"BaR\",true,null,");

        let mut transitizers: HashMap<Option<usize>, TransformSanitizeTokens> = HashMap::new();
        transitizers.insert(None, vec![Box::new(ToLowercase)]);
        transitizers.insert(Some(0), vec![Box::new(TrimAll)]);

        let parser = PattiCsvParserBuilder::new()
            .first_line_is_header(true)
            .column_transitizers(transitizers)
            .build(&mut test_data_cursor)
            .unwrap();

        let mut iter = parser.into_iter();
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

    #[test]
    fn test_parser_skip_comments_and_summation_lines() {
        let mut test_data_cursor = std::io::Cursor::new("# shitty comment line!\n# shitty comment line 2\nc1,c2,c3,c4\n 1 ,\"BaR\",true,\na, shitty, summation, line");

        let mut transitizers: HashMap<Option<usize>, TransformSanitizeTokens> = HashMap::new();
        transitizers.insert(None, vec![Box::new(ToLowercase)]);
        transitizers.insert(Some(0), vec![Box::new(TrimAll)]);

        let parser = PattiCsvParserBuilder::new()
            .first_line_is_header(true)
            .mandatory_column_typings(false)
            .skip_take_lines_fns(vec![
                Box::new(SkipLinesStartingWith {
                    starts_with: String::from("#"),
                }),
                Box::new(SkipLinesFromEnd {
                    skip_num_lines: 1,
                    lines_total: 5,
                }),
            ])
            .column_transitizers(transitizers)
            .build(&mut test_data_cursor)
            .unwrap();

        let mut iter = parser.into_iter();
        let headers = iter.next();
        let line_1 = iter.next();

        println!("{:?}", headers);
        println!("{:?}", line_1);
        // TODO
    }

    #[test]
    fn test_parser_skip_comments_and_summation_lines_save_skipped() {
        let mut test_data_cursor = std::io::Cursor::new("# shitty comment line!\n# shitty comment line 2\nc1,c2,c3,c4\n 1 ,\"BaR\",true,\na, shitty, summation, line");

        let mut transitizers: HashMap<Option<usize>, TransformSanitizeTokens> = HashMap::new();
        transitizers.insert(None, vec![Box::new(ToLowercase)]);
        transitizers.insert(Some(0), vec![Box::new(TrimAll)]);

        let parser = PattiCsvParserBuilder::new()
            .first_line_is_header(true)
            .mandatory_column_typings(false)
            .skip_take_lines_fns(vec![
                Box::new(SkipLinesStartingWith {
                    starts_with: String::from("#"),
                }),
                Box::new(SkipLinesFromEnd {
                    skip_num_lines: 1,
                    lines_total: 5,
                }),
            ])
            .save_skipped_lines(true)
            .column_transitizers(transitizers)
            .build(&mut test_data_cursor)
            .unwrap();

        let mut parser_iter = parser.into_iter();

        while let Some(_) = parser_iter.next() {}
        println!("{:?}", &parser_iter.get_skipped_lines());
        // TODO real test
    }

    #[test]
    fn test_parser_date_default_patterns() {
        let mut test_data_cursor = std::io::Cursor::new(
            "c1,c2,c3\n2022-01-01,2022-02-02 12:00:00,2022-12-31T06:00:00+05:00",
        );

        let mut transitizers: HashMap<Option<usize>, TransformSanitizeTokens> = HashMap::new();
        transitizers.insert(None, vec![Box::new(ToLowercase)]);
        transitizers.insert(Some(0), vec![Box::new(TrimAll)]);

        let parser = PattiCsvParserBuilder::new()
            .first_line_is_header(true)
            .column_typings(vec![
                TypeColumnEntry::new(Some(String::from("col1")), ValueType::NaiveDate),
                TypeColumnEntry::new(Some(String::from("col2")), ValueType::NaiveDateTime),
                TypeColumnEntry::new(Some(String::from("col3")), ValueType::DateTime),
            ])
            .column_transitizers(transitizers)
            .build(&mut test_data_cursor)
            .unwrap();

        let mut parser_iter = parser.into_iter();
        let headers = parser_iter.next();
        let line_1 = parser_iter.next();

        println!("{:?}", headers);
        println!("{:?}", line_1);
        // TODO
    }

    #[test]
    fn test_parser_date_manual_chrono_patterns() {
        let mut test_data_cursor = std::io::Cursor::new(
            "c1,c2,c3\n20.01.2022,20.01.2022 12_00_00,20.1.2022 8:00 am +0000",
        );

        let mut transitizers: HashMap<Option<usize>, TransformSanitizeTokens> = HashMap::new();
        transitizers.insert(None, vec![Box::new(ToLowercase)]);
        transitizers.insert(Some(0), vec![Box::new(TrimAll)]);

        let parser = PattiCsvParserBuilder::new()
            .first_line_is_header(true)
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
            .column_transitizers(transitizers)
            .build(&mut test_data_cursor)
            .unwrap();

        let mut iter = parser.into_iter();
        let headers = iter.next();
        let line_1 = iter.next();

        println!("{:?}", headers);
        println!("{:?}", line_1);
        // TODO
    }
}
