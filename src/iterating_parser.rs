use std::collections::HashMap;
use std::io::Read;
use std::marker::PhantomData;

use venum::venum::Value;
use venum_tds::cell::DataCell;
use venum_tds::row::DataCellRow;

use crate::errors::{PattiCsvError, Result};
use crate::line_tokenizer::DelimitedLineTokenizer;

use super::line_tokenizer::DelimitedLineTokenizerStats;
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
    dlt_iter: DelimitedLineTokenizer<'rd, R>,
}

impl<'rd, R: Read> PattiCsvParser<'rd, R> {
    pub fn builder() -> PattiCsvParserBuilder<R> {
        PattiCsvParserBuilder::new()
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
    column_transitizers: Option<HashMap<Option<usize>, TransformSanitizeTokens>>,
    mandatory_column_typings: bool,
    column_typings: Vec<TypeColumnEntry>,
}

impl<'rd, R: Read> PattiCsvParserBuilder<R> {
    pub fn new() -> Self {
        Self {
            separator_char: ',',
            enclosure_char: Some('"'),
            first_line_is_header: true,
            skip_take_lines_fns: None,
            column_transitizers: None,
            mandatory_column_typings: false,
            column_typings: Vec::new(),
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
        self.column_typings = t;
        self
    }
    pub fn build(&mut self, input_raw_data: &'rd mut R) -> Result<PattiCsvParser<'rd, R>> {
        if self.mandatory_column_typings && self.column_typings.len() == 0 {
            return Err(PattiCsvError::Generic {
                msg: String::from("Column typings have been flagged mandatory but are not set!"),
            });
        }
        Ok(PattiCsvParser {
            first_line_is_header: self.first_line_is_header,
            column_transitizers: std::mem::take(&mut self.column_transitizers),
            column_typings: std::mem::take(&mut self.column_typings),
            dlt_iter: DelimitedLineTokenizer::new(
                input_raw_data,
                self.separator_char,
                self.enclosure_char,
                std::mem::take(&mut self.skip_take_lines_fns),
            ),
        })
    }
    // TODO: default_to_string_on_no_typings = true
}

pub struct PattiCsvParserIterator<'rd, R: Read> {
    patti_csv_parser: PattiCsvParser<'rd, R>,
    col_layout_template: Option<DataCellRow>,
}

impl<'rd, 'cfg, R: Read> Iterator for PattiCsvParserIterator<'rd, R> {
    type Item = Result<(DataCellRow, DelimitedLineTokenizerStats)>;

    fn next(&mut self) -> Option<Self::Item> {
        // .next() returns a: Option<Result<(Vec<String>, DelimitedLineTokenizerStats)>>
        // We early "return" a None through the ?, then we check for an error inside the Some(Result)
        let (dlt_iter_res_vec, dlt_iter_res_stats) = match self.patti_csv_parser.dlt_iter.next()? {
            Err(e) => return Some(Err(e)),
            Ok(dlt_iter_res) => dlt_iter_res,
        };

        // Special case for the first line, which might be a header line and must be treated differently.
        if dlt_iter_res_stats.is_at_first_line_to_parse() {
            // If we don't have type info for the columns, default to String for everything, as this is a common
            // usecase when typings are not actually needed, e.g. when we just want to skip certain things, etc.
            if self.patti_csv_parser.column_typings.len() == 0 {
                for _ in 0..dlt_iter_res_vec.len() {
                    self.patti_csv_parser
                        .column_typings
                        .push(TypeColumnEntry::new(None, Value::string_default()));
                }
            }

            // If this is the case, we need to set the correct headers in our template, then return
            // the data as the first line.
            if self.patti_csv_parser.first_line_is_header {
                self.col_layout_template = match build_layout_template(
                    Some(&dlt_iter_res_vec),
                    &self.patti_csv_parser.column_typings,
                ) {
                    Ok(v) => Some(v),
                    Err(e) => return Some(Err(e)),
                };

                // Special case for the header line, where our datatype is always, hardcoded, a string.
                // Also, we need to use the correct header names that may come from the typings, or the
                // headerline, or are defaulted to indices, in this order!
                let mut csv_header_data_cell_row: DataCellRow = DataCellRow::new();
                dlt_iter_res_vec.into_iter().enumerate().for_each(|(i, _)| {
                    // We have set the correct header-name above anyway, we can just use it here!
                    // All we really care about here is, that we default the type to String.
                    let header_name = &self
                        .col_layout_template
                        .as_ref()
                        .unwrap() // This is set above, no risk in calling unwrap here!
                        .0
                        .get(i)
                        .unwrap()
                        .name;

                    // TODO: do we want transitization on the headers!?

                    let new_csv_cell = DataCell::new(
                        Value::string_default(),
                        header_name.clone(),
                        i,
                        Some(header_name.clone().into()),
                    );
                    csv_header_data_cell_row.0.push(new_csv_cell);
                });
                return Some(Ok((csv_header_data_cell_row, dlt_iter_res_stats)));
            } else {
                // In this case, the first line is actual data, meaning, we first need to build the
                // structure, without parsing and setting the headers.
                // We do not(!) return this immediately as the first line, since we must first sanitize
                // and then type the data.
                self.col_layout_template =
                    match build_layout_template(None, &self.patti_csv_parser.column_typings) {
                        Ok(v) => Some(v),
                        Err(e) => return Some(Err(e)),
                    };
            }
        }

        // Shared logic for all data, or non-header lines
        let mut row_data: DataCellRow = match self.col_layout_template.clone() {
            Some(v) => v,
            None => {
                return Some(Err(PattiCsvError::Generic {
                    msg: "Error! No structure template available, but expected one.".into(),
                }))
            }
        };

        let sanitized_tokens = match sanitize_tokenizer_iter_res(
            dlt_iter_res_stats.curr_line_num,
            dlt_iter_res_vec,
            &self.patti_csv_parser.column_transitizers,
        ) {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };

        let mut col_iter = row_data.0.iter_mut().enumerate();
        while let Some((i, cell)) = col_iter.next() {
            let curr_token = sanitized_tokens.get(i).unwrap();
            let typings = self.patti_csv_parser.column_typings.get(i).unwrap(); // TODO: I think this is save, as the col iter index shouldn't be larger than the typings, but need to check again!

            if cell.type_info.is_some_date_type() && typings.chrono_pattern.is_some() {
                cell.data = match Value::datetype_from_string_with_templ_and_chrono_pattern(
                    curr_token,
                    &cell.type_info,
                    typings.chrono_pattern.as_ref().unwrap(), // we already checked above
                ) {
                    Ok(v) => v,
                    Err(e) => return Some(Err(e.into())),
                };
            } else {
                // Will still attempt to construct date-(time) types from the token, but only tries the specified default patterns.
                cell.data = match Value::from_string_with_templ(curr_token, &cell.type_info) {
                    Ok(v) => v,
                    Err(e) => return Some(Err(e.into())),
                };
            }
        }
        Some(Ok((row_data, dlt_iter_res_stats)))
    }
}

impl<'rd, 'cfg, R: Read> IntoIterator for PattiCsvParser<'rd, R> {
    type Item = Result<(DataCellRow, DelimitedLineTokenizerStats)>;
    type IntoIter = PattiCsvParserIterator<'rd, R>;

    fn into_iter(self) -> Self::IntoIter {
        PattiCsvParserIterator {
            patti_csv_parser: self,
            col_layout_template: None,
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
                TypeColumnEntry::new(None, Value::int32_default()),
                TypeColumnEntry::new(None, Value::string_default()),
                TypeColumnEntry::new(None, Value::bool_default()),
            ])
            .column_transitizers(transitizers)
            .build(&mut test_data_cursor)
            .unwrap();

        assert_eq!(parser.dlt_iter.delim_char, ';');
        assert_eq!(parser.dlt_iter.encl_char, Some('\''));
        assert_eq!(parser.first_line_is_header, false);
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

        assert_eq!(parser.dlt_iter.delim_char, ',');
        assert_eq!(parser.dlt_iter.encl_char, Some('"'));
        assert_eq!(parser.first_line_is_header, true);
        assert_eq!(parser.column_typings.len(), 0);
        assert_eq!(parser.column_transitizers.is_none(), true);
    }

    #[test]
    fn test_parser_01() {
        let mut test_data_cursor = std::io::Cursor::new("c1;c2;c3;c4\n 1 ;'BaR';true;");

        let mut transitizers: HashMap<Option<usize>, TransformSanitizeTokens> = HashMap::new();
        transitizers.insert(None, vec![Box::new(ToLowercase)]);
        transitizers.insert(Some(0), vec![Box::new(TrimAll)]);

        let parser = PattiCsvParserBuilder::new()
            .separator_char(';')
            .enclosure_char(Some('\''))
            .first_line_is_header(true)
            .column_typings(vec![
                TypeColumnEntry::new(None, Value::int32_default()),
                TypeColumnEntry::new(Some(String::from("col2")), Value::string_default()),
                TypeColumnEntry::new(Some(String::from("col3")), Value::bool_default()),
                TypeColumnEntry::new(Some(String::from("col4")), Value::string_default()),
            ])
            .column_transitizers(transitizers)
            .build(&mut test_data_cursor)
            .unwrap();

        let mut iter = parser.into_iter();
        let headers = iter.next();
        let line_1 = iter.next();

        println!("{:?}", headers);
        println!("{:?}", line_1);
    }

    #[test]
    fn test_parser_02() {
        let mut test_data_cursor = std::io::Cursor::new("c1,c2,c3,c4\n 1 ,\"BaR\",true,");

        let mut transitizers: HashMap<Option<usize>, TransformSanitizeTokens> = HashMap::new();
        transitizers.insert(None, vec![Box::new(ToLowercase)]);
        transitizers.insert(Some(0), vec![Box::new(TrimAll)]);

        let parser = PattiCsvParserBuilder::new()
            .first_line_is_header(true)
            .column_transitizers(transitizers)
            .build(&mut test_data_cursor)
            .unwrap();

        let mut iter = parser.into_iter();
        let headers = iter.next();
        let line_1 = iter.next();

        println!("{:?}", headers);
        println!("{:?}", line_1);
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
        println!("{:?}", iter.next()); // headers
        println!("{:?}", iter.next()); // line 1
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
                TypeColumnEntry::new(Some(String::from("col1")), Value::naive_date_default()),
                TypeColumnEntry::new(Some(String::from("col2")), Value::naive_date_time_default()),
                TypeColumnEntry::new(Some(String::from("col3")), Value::date_time_default()),
            ])
            .column_transitizers(transitizers)
            .build(&mut test_data_cursor)
            .unwrap();

        let mut iter = parser.into_iter();
        let headers = iter.next();
        let line_1 = iter.next();

        println!("{:?}", headers);
        println!("{:?}", line_1);
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
                    Value::naive_date_default(),
                    String::from("%d.%m.%Y"),
                ),
                TypeColumnEntry::new_with_chrono_pattern(
                    Some(String::from("col2")),
                    Value::naive_date_time_default(),
                    String::from("%d.%m.%Y %H_%M_%S"),
                ),
                TypeColumnEntry::new_with_chrono_pattern(
                    Some(String::from("col3")),
                    Value::date_time_default(),
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
    }
}
