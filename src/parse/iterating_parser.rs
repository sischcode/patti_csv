use std::collections::HashMap;
// use log::trace;
use std::io::Read;
use std::marker::PhantomData;

use crate::data::data::CsvData;
use crate::data::column::Column;
use crate::data::value::Value;
use crate::errors::{PattiCsvError, Result};
use crate::parse::line_tokenizer::DelimitedLineTokenizer;

use super::line_tokenizer::DelimitedLineTokenizerStats;
use super::parser_common::{build_csv_data_skeleton_w_header, build_csv_data_skeleton, sanitize_tokenizer_iter_res};
use super::parser_config::{TransformSanitizeTokens, TypeColumnEntry};
use super::skip_take_lines::SkipTakeLines;


pub struct PattiCsvParser<'rd, R>
where
    R: Read,
{
    first_line_is_header: bool,
    column_transitizers: Option<HashMap<Option<usize>, TransformSanitizeTokens>>,
    column_typings: Vec<TypeColumnEntry>,
    dlt_iter: DelimitedLineTokenizer<'rd, R>,
}

impl<'rd, R: Read> PattiCsvParser<'rd, R> {
    pub fn builder() -> PattiCsvParserBuilder<R>  {
        PattiCsvParserBuilder::new()
    }
}

pub struct PattiCsvParserBuilder<R> where R: Read {
    phantom: PhantomData<R>,
    separator_char: char,
    enclosure_char: Option<char>,
    first_line_is_header: bool,
    skip_take_lines_fns: Option<Vec<Box<dyn SkipTakeLines>>>,
    column_transitizers: Option<HashMap<Option<usize>, TransformSanitizeTokens>>,
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
            column_typings: Vec::new(),
            phantom: PhantomData::default()
        }
    }
    pub fn separator_char(mut self, c: char) -> PattiCsvParserBuilder<R> {
        self.separator_char = c;
        self
    }
    pub fn enclosure_char(mut self, c: Option<char>) -> PattiCsvParserBuilder<R>  {
        self.enclosure_char = c;
        self
    }
    pub fn first_line_is_header(mut self, b: bool) -> PattiCsvParserBuilder<R>  {
        self.first_line_is_header = b;
        self
    }
    pub fn skip_take_lines_fns(mut self, s: Vec<Box<dyn SkipTakeLines>>) -> PattiCsvParserBuilder<R>  {
        self.skip_take_lines_fns = Some(s);
        self
    }
    pub fn column_transitizers(mut self, t: HashMap<Option<usize>, TransformSanitizeTokens>) -> PattiCsvParserBuilder<R>  {
        self.column_transitizers = Some(t);
        self
    }
    pub fn column_typings(mut self, t: Vec<TypeColumnEntry>) -> PattiCsvParserBuilder<R>  {
        self.column_typings = t;
        self
    }
    pub fn build(mut self, input_raw_data: &'rd mut R) -> PattiCsvParser<'rd, R> {
        PattiCsvParser {
            first_line_is_header: self.first_line_is_header,
            column_transitizers: self.column_transitizers,
            column_typings: self.column_typings,
            dlt_iter: DelimitedLineTokenizer::new(
                input_raw_data,
                self.separator_char,
                self.enclosure_char,
                std::mem::take(&mut self.skip_take_lines_fns)
            ),
        }
    }
}

pub struct PattiCsvParserIterator<'rd, R: Read> {
    patti_csv_parser: PattiCsvParser<'rd, R>,
    col_layout_template: Option<CsvData>,
}

impl<'rd, 'cfg, R: Read> Iterator for PattiCsvParserIterator<'rd, R> {
    type Item = Result<(usize, CsvData, DelimitedLineTokenizerStats)>;

    fn next(&mut self) -> Option<Self::Item> {
        // .next() returns a: Option<Result<(Vec<String>, DelimitedLineTokenizerStats)>>
        // We early "return" a None through the ?, then we check for an error inside the Some(Result)
        let (dlt_iter_res_vec, dlt_iter_res_stats) = match self.patti_csv_parser.dlt_iter.next()? {
            Err(e) => return Some(Err(e)),
            Ok(dlt_iter_res) => dlt_iter_res,
        };

        let mut csv_data = CsvData::new();
        match dlt_iter_res_stats.is_at_header_line() {
            // Special case for first line. We create a skeleton with or without supplied headers
            true => {
                if self.patti_csv_parser.first_line_is_header {
                    self.col_layout_template = match build_csv_data_skeleton_w_header(
                        &dlt_iter_res_vec,
                        &self.patti_csv_parser.column_typings,
                    ) {
                        Ok(v) => Some(v),
                        Err(e) => return Some(Err(e)),
                    }
                } else {
                    self.col_layout_template =
                        Some(build_csv_data_skeleton(&self.patti_csv_parser.column_typings));
                }

                dlt_iter_res_vec.into_iter().enumerate().for_each(|(i, v)| {
                    let mut new_col = Column::new(Value::string_default(), v.clone(), i);
                    new_col.push(Some(v.into()));
                    csv_data.add_col(new_col);
                });
            }
            false => {
                csv_data = match self.col_layout_template.clone() {
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

                let mut col_iter = csv_data.columns.iter_mut().enumerate();
                while let Some((i, col)) = col_iter.next() {
                    let curr_token = sanitized_tokens.get(i).unwrap();
                    col.push(
                        match Value::from_string_with_templ(curr_token.clone(), &col.type_info) {
                            Ok(v) => v,
                            Err(e) => return Some(Err(e)),
                        },
                    );
                }
            }
        }
        Some(Ok((0, csv_data, dlt_iter_res_stats)))
    }
}

impl<'rd, 'cfg, R: Read> IntoIterator for PattiCsvParser<'rd, R> {
    type Item = Result<(usize, CsvData, DelimitedLineTokenizerStats)>;
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
    use super::*;

    #[test]
    fn test_builder() {
        let mut test_data_cursor = std::io::Cursor::new("foo;'bar';baz");

        let parser = PattiCsvParserBuilder::new()
            .separator_char(';')
            .enclosure_char(Some('\''))
            .first_line_is_header(false)
            .column_typings(vec![
                TypeColumnEntry{ header: None, target_type: Value::string_default()},
                TypeColumnEntry{ header: None, target_type: Value::string_default()},
                TypeColumnEntry{ header: None, target_type: Value::string_default()},
            ])
            .build(&mut test_data_cursor);
            
        assert_eq!(parser.first_line_is_header, false);
        assert_eq!(parser.column_transitizers.is_none(), true);
        assert_eq!(parser.column_typings, vec![]);
        assert_eq!(parser.dlt_iter.delim_char, ';');
        
        let mut iter = parser.into_iter();
        println!("{:?}", iter.next());
    }
}