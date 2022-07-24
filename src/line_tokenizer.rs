use compact_str::CompactString;
use std::io::{BufRead, BufReader, Read};

use crate::errors::{PattiCsvError, Result, TokenizerError};

use super::skip_take_lines::SkipTakeLines;

enum State {
    Start, // same as Scan, but we need the distinction, so that we can apply special treatment to scan at the end of tokenizing.
    Scan, // decide whether to go to Field or QuotedField, or just add an empty field, if we encounter the delimiter character
    Field, // regular, unenclosed field. We stay here until the field is finished
    QuotedField, // enclosed field start
    QuoteInQuotedField, // we need this to do proper escape checking of the enclosure character
}

#[derive(Debug, Clone, PartialEq)]
pub struct DelimitedLineTokenizerStats {
    pub curr_line_num: usize, // needed for internal state while iterating
    pub lines_parsed: usize,  // needed for internal state while iterating
    pub skipped_lines: Vec<usize>,
    pub bytes_read: usize,
}

impl DelimitedLineTokenizerStats {
    pub fn new() -> Self {
        Self {
            curr_line_num: 0,
            lines_parsed: 0,
            skipped_lines: Vec::<usize>::new(),
            bytes_read: 0,
        }
    }
    pub fn is_at_first_line_to_parse(&self) -> bool {
        self.lines_parsed == 1
    }
}

impl Default for DelimitedLineTokenizerStats {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DelimitedLineTokenizer<'rd, R: Read> {
    num_tokens_exp_set: bool, // used as internal info to indicate if the num_tokens_exp has already been set. This is only done once!
    num_tokens_exp: usize, // number of tokens in a line/row, we expect, based on the first line (usually the header), is has parsed.
    max_inline_str_size: usize, // helper for compact string. Since we're not that concerned with space limitations, we allocate the max a compact string can allocate on the stack
    buf_raw_data: BufReader<&'rd mut R>,
    pub delim_char: char,
    pub encl_char: Option<char>,
    pub skip_take_lines_fns: Option<Vec<Box<dyn SkipTakeLines>>>, // needed here to skip lines while iterating
    pub stats: DelimitedLineTokenizerStats,
}

/// Mostly written with the csv rfc (https://tools.ietf.org/html/rfc4180) in mind, and compliant with that,
/// but works also for lines, delimited by other characters (e.g. tab or pipe), when it's simple enough.
/// Delimiter-Character and Enclosure-Character can be set, but a set of standard stuff is provided via
/// different "constructors".
impl<'rd, R: Read> DelimitedLineTokenizer<'rd, R> {
    pub fn new(
        raw_data: &'rd mut R,
        delim: char,
        enclc: Option<char>,
        skip_take_lines_fns: Option<Vec<Box<dyn SkipTakeLines>>>,
    ) -> Self {
        DelimitedLineTokenizer {
            num_tokens_exp_set: false,
            num_tokens_exp: 15, // we set the initial value to 15, because...we gotta start with something ¯\_(ツ)_/¯
            max_inline_str_size: std::mem::size_of::<String>(),
            buf_raw_data: BufReader::new(raw_data),
            delim_char: delim,
            encl_char: enclc,
            skip_take_lines_fns,
            stats: DelimitedLineTokenizerStats::new(),
        }
    }
    pub fn csv(raw_data: &'rd mut R, skip_take_lines: Option<Vec<Box<dyn SkipTakeLines>>>) -> Self {
        DelimitedLineTokenizer::new(raw_data, ',', Some('"'), skip_take_lines)
    }
    pub fn tab(raw_data: &'rd mut R, skip_take_lines: Option<Vec<Box<dyn SkipTakeLines>>>) -> Self {
        DelimitedLineTokenizer::new(raw_data, '\t', None, skip_take_lines)
    }

    pub fn get_stats(&self) -> &DelimitedLineTokenizerStats {
        &self.stats
    }

    fn skip_line_by_skiptake_sanitizer(&self, line_counter: usize, line: &String) -> bool {
        // If we have filters, we apply them and see if we need to skip this line.
        if let Some(ref skip_take_lines) = self.skip_take_lines_fns {
            skip_take_lines
                .iter()
                .map(|filter| filter.skip(Some(line_counter), Some(line))) // check line against every sanitizer
                .find(|res| *res) // if at least one yields true, we need to skip (this line)
                .unwrap_or(false)
        } else {
            // If we have no filters, well, then don't skip anything.
            false
        }
    }

    /// line_num is only used for error context
    fn tokenize(&mut self, line_num: usize, s: &str) -> Result<Vec<String>> {
        let mut state = State::Start;
        let mut data: Vec<CompactString> = Vec::with_capacity(self.num_tokens_exp);

        // A small FSM here...
        for c in s.chars() {
            state = match state {
                State::Start | State::Scan => match c {
                    _ if c == self.delim_char => {
                        // this means: empty field at start
                        data.push(CompactString::with_capacity(self.max_inline_str_size));
                        State::Scan
                    }
                    _ if Some(c) == self.encl_char => {
                        // enclosure symbol (start) found
                        data.push(CompactString::with_capacity(self.max_inline_str_size));
                        State::QuotedField
                    }
                    _ => {
                        // start of regular, un-enclosed field
                        let mut cs = CompactString::with_capacity(self.max_inline_str_size);
                        cs.push(c);

                        data.push(cs);
                        State::Field
                    }
                },
                State::Field => match c {
                    _ if c == self.delim_char => {
                        State::Scan // ready for next field
                    }
                    _ if Some(c) == self.encl_char => {
                        return Err(PattiCsvError::Tokenize(TokenizerError::IllegalEnclChar {
                            line: line_num,
                            token_num: data.len(),
                        }))
                    }
                    _ => {
                        data.last_mut().unwrap().push(c); // we only ever come from Start or Scan, so there is always a last element set!
                        State::Field
                    }
                },
                State::QuotedField => match c {
                    _ if Some(c) == self.encl_char => State::QuoteInQuotedField,
                    _ => {
                        data.last_mut().unwrap().push(c); // we only ever come from Start or Scan, or QuoteInQuotedField, so there is always a last element set!
                        State::QuotedField
                    }
                },
                State::QuoteInQuotedField => match c {
                    _ if c == self.delim_char => State::Scan, // enlosure closed, ready for next field
                    _ if Some(c) == self.encl_char => {
                        // enclosure character escaped successfully
                        data.last_mut().unwrap().push(c); // we only ever come here from QuotedField, so there is always a last element set!
                        State::QuotedField
                    }
                    _ => {
                        return Err(PattiCsvError::Tokenize(TokenizerError::UnescapedEnclChar {
                            line: line_num,
                            token_num: data.len(),
                        }))
                    }
                },
            }
        }

        // 1) A bit of cleanup. If we end in state Scan, this means, the last thing we read was a delimiter before it
        //    ended, thusly we must append an empty "" at the end, to represent the empty column at the end
        // 2) When we end on State:QuotedField, the field is not properly enclosed. For a quoted field to end properly,
        //    we'd need to end on State:QuoteInQuotedField instead.
        match state {
            State::Scan => {
                data.push(CompactString::new(""));
            }
            State::QuotedField => {
                return Err(PattiCsvError::Tokenize(TokenizerError::UnescapedEnclChar {
                    line: line_num,
                    token_num: data.len(),
                }))
            }
            _ => (),
        }

        // After the first real line, we adjust the (future) length of the vec we allocate
        if !self.num_tokens_exp_set {
            self.num_tokens_exp = data.len();
            self.num_tokens_exp_set = true;
        }

        Ok(data
            .iter()
            .map(|t| String::from(t.as_str()))
            .collect::<Vec<String>>())
    }
}

impl<'rd, R: Read> Iterator for DelimitedLineTokenizer<'rd, R> {
    type Item = Result<(Vec<String>, DelimitedLineTokenizerStats)>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = String::new();
        let mut skip_this_line = true;

        while skip_this_line {
            line = String::new();
            self.stats.curr_line_num += 1;
            let bytes_read = match self.buf_raw_data.read_line(&mut line) {
                Ok(num_bytes) => match num_bytes {
                    _ if num_bytes == 0_usize => return None, // returns "normal", i.e. end of "stream". ('return' always returns from a funtion!)
                    _ => Some(num_bytes),
                },
                Err(e) => {
                    let msg = format!("error reading line {}. {}", self.stats.curr_line_num, e);
                    return Some(Err(PattiCsvError::Generic { msg }));
                }
            };
            self.stats.bytes_read += bytes_read.unwrap(); // unwrap is OK here, we checked every other path
            skip_this_line = self.skip_line_by_skiptake_sanitizer(self.stats.curr_line_num, &line);
            if skip_this_line {
                self.stats.skipped_lines.push(self.stats.curr_line_num);
            }
        }
        self.stats.lines_parsed += 1;

        match self.tokenize(self.stats.curr_line_num, line.trim_end()) {
            Ok(v) => Some(Ok((v, self.stats.clone()))),
            Err(e) => Some(Err(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let mut test_data_cursor = std::io::Cursor::new("");
        let mut dlt_iter = DelimitedLineTokenizer::csv(&mut test_data_cursor, None).into_iter();
        let res = dlt_iter.next();

        assert_eq!(res, None);
    }

    fn test_it(inp: &str, exp: Vec<&str>) {
        let mut test_data_cursor = std::io::Cursor::new(inp);
        let mut dlt_iter = DelimitedLineTokenizer::csv(&mut test_data_cursor, None).into_iter();
        let res = dlt_iter.next().unwrap().unwrap().0;
        assert_eq!(res, exp);
    }

    #[test]
    fn simple_one_cols() {
        test_it("y̆es", vec!["y̆es"]);
    }

    #[test]
    fn simple_two_cols() {
        test_it("y̆es,bar", vec!["y̆es", "bar"]);
    }

    #[test]
    fn start_empty() {
        test_it(",y̆es,bar", vec!["", "y̆es", "bar"]);
    }

    #[test]
    fn middle_empty() {
        test_it("y̆es,,bar", vec!["y̆es", "", "bar"]);
    }

    #[test]
    fn end_empty() {
        test_it("y̆es,bar,", vec!["y̆es", "bar", ""]);
    }

    #[test]
    fn start_end_empty() {
        test_it(",y̆es,bar,", vec!["", "y̆es", "bar", ""]);
    }

    #[test]
    fn two_empty_cols() {
        test_it(",", vec!["", ""]);
    }

    #[test]
    fn three_empty_cols() {
        test_it(",,", vec!["", "", ""]);
    }

    #[test]
    fn single_col_quoted() {
        test_it("\"y̆,es\"", vec!["y̆,es"]);
    }

    #[test]
    fn start_quoted() {
        test_it("\"y̆,es\",bar", vec!["y̆,es", "bar"]);
    }

    #[test]
    fn middle_quoted() {
        test_it("foo,\"y̆,es\",bar", vec!["foo", "y̆,es", "bar"]);
    }

    #[test]
    fn end_quoted() {
        test_it("yes,\"y̆,es\"", vec!["yes", "y̆,es"]);
    }

    #[test]
    fn all_quoted() {
        test_it("\"foo\",\"y̆,es\",\"bar\"", vec!["foo", "y̆,es", "bar"]);
    }

    #[test]
    fn all_quoted_empty_start() {
        test_it(",\"foo\",\"y̆,es\",\"bar\"", vec!["", "foo", "y̆,es", "bar"]);
    }

    #[test]
    fn all_quoted_empty_end() {
        test_it("\"foo\",\"y̆,es\",\"bar\",", vec!["foo", "y̆,es", "bar", ""]);
    }

    #[test]
    fn all_quoted_empty_start_empty_end() {
        test_it(
            ",\"foo\",\"y̆,es\",\"bar\",",
            vec!["", "foo", "y̆,es", "bar", ""],
        );
    }

    #[test]
    fn empty_quoted_field() {
        test_it("\"\"", vec![""]);
    }

    #[test]
    fn one_quote_in_quoted_col() {
        test_it("\"\"\"\"", vec!["\""]);
    }

    #[test]
    fn two_quotes_in_quoted_col() {
        test_it("\"\"\"\"\"\"", vec!["\"\""]);
    }

    #[test]
    fn val_then_quote_in_quoted_col() {
        test_it("\"24 \"\"\"", vec!["24 \""]);
    }

    #[test]
    fn quote_then_val_in_quoted_col() {
        test_it("\"\"\" = zoll\"", vec!["\" = zoll"]);
    }

    #[test]
    fn two_quotes_then_value_then_two_quotes_in_quoted_col() {
        test_it("\"\"\"\"\"f,o,o\"\"\"\"\"", vec!["\"\"f,o,o\"\""]);
    }

    #[test]
    fn enclosing_with_enclosing_char_not_properly_escaped() {
        let mut test_data_cursor = std::io::Cursor::new("foo,\"bar\"\",baz");
        let mut dlt_iter = DelimitedLineTokenizer::csv(&mut test_data_cursor, None).into_iter();
        let res = dlt_iter.next().unwrap();
        assert_eq!(
            Err(PattiCsvError::Tokenize(TokenizerError::UnescapedEnclChar {
                line: 1,
                token_num: 2
            })),
            res
        );
    }

    #[test]
    fn enclosing_char_in_unenclosed_field() {
        let mut test_data_cursor = std::io::Cursor::new("f\"oo,bar");
        let mut dlt_iter = DelimitedLineTokenizer::csv(&mut test_data_cursor, None).into_iter();
        let res = dlt_iter.next().unwrap();
        assert_eq!(
            Err(PattiCsvError::Tokenize(TokenizerError::IllegalEnclChar {
                line: 1,
                token_num: 1
            })),
            res
        );
    }

    #[test]
    fn tab_separated_simple() {
        let mut test_data_cursor = std::io::Cursor::new("foo\tb\"a'r\tb|az");
        let mut dlt_iter = DelimitedLineTokenizer::tab(&mut test_data_cursor, None).into_iter();
        let res = dlt_iter.next().unwrap();
        assert_eq!(res.unwrap().0, vec!["foo", "b\"a'r", "b|az"]);
    }

    #[test]
    /// doesn't really work correctly, or does it?
    fn tab_separated_simple_enclosed() {
        let mut test_data_cursor = std::io::Cursor::new("foo\t\"b\tar\"\tbaz");
        let mut dlt_iter =
            DelimitedLineTokenizer::new(&mut test_data_cursor, '\t', Some('"'), None).into_iter();
        let res = dlt_iter.next().unwrap();
        assert_eq!(res.unwrap().0, vec!["foo", "b\tar", "baz"]);
    }

    #[test]
    fn pipe_separated_simple_enclosed() {
        let mut test_data_cursor = std::io::Cursor::new("foo|\"b|ar\"|baz");
        let mut dlt_iter =
            DelimitedLineTokenizer::new(&mut test_data_cursor, '|', Some('"'), None).into_iter();
        let res = dlt_iter.next().unwrap();
        assert_eq!(res.unwrap().0, vec!["foo", "b|ar", "baz"]);
    }

    #[test]
    fn pipe_separated_simple_enclosed2() {
        let mut test_data_cursor = std::io::Cursor::new("foo|'b|ar'|baz");
        let mut dlt_iter =
            DelimitedLineTokenizer::new(&mut test_data_cursor, '|', Some('\''), None).into_iter();
        let res = dlt_iter.next().unwrap();
        assert_eq!(res.unwrap().0, vec!["foo", "b|ar", "baz"]);
    }

    #[test]
    fn multiple_lines_test_simple() {
        let mut test_data_cursor = std::io::Cursor::new("a,b,c\n1,2,3");
        let mut dlt_iter = DelimitedLineTokenizer::csv(&mut test_data_cursor, None).into_iter();

        let (res, stats) = dlt_iter.next().unwrap().unwrap();
        assert_eq!(res, vec!["a", "b", "c"]);
        assert_eq!(stats.curr_line_num, 1);

        let (res, stats) = dlt_iter.next().unwrap().unwrap();
        assert_eq!(res, vec!["1", "2", "3"]);
        assert_eq!(stats.curr_line_num, 2);
    }
}
