use compact_str::CompactString;
use std::{
    collections::VecDeque,
    io::{BufRead, BufReader, Read},
};

use super::errors::{PattiCsvError, Result, TokenizerError};
use super::skip_take_lines::SkipTakeLines;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelimitedLineTokenizerStats {
    pub curr_line_num: usize,       // needed for internal state while iterating
    pub num_lines_read: usize,      // needed for internal state while iterating
    pub num_lines_tokenized: usize, // needed for internal state while iterating
    pub skipped_lines: Vec<(usize, Option<String>)>,
    pub bytes_read: usize,
}

impl DelimitedLineTokenizerStats {
    pub fn new() -> Self {
        Self {
            curr_line_num: 0,
            num_lines_read: 0,
            num_lines_tokenized: 0,
            skipped_lines: Vec::with_capacity(5),
            bytes_read: 0,
        }
    }
    pub fn is_at_first_unskipped_line_to_parse(&self) -> bool {
        self.num_lines_tokenized == 1
    }
}

impl Default for DelimitedLineTokenizerStats {
    fn default() -> Self {
        Self::new()
    }
}

enum State {
    Start, // same as Scan, but we need the distinction, so that we can apply special treatment to scan at the end of tokenizing.
    Scan, // decide whether to go to Field or QuotedField, or just add an empty field, if we encounter the delimiter character
    Field, // regular, unenclosed field. We stay here until the field is finished
    QuotedField, // enclosed field start
    QuoteInQuotedField, // we need this to do proper escape checking of the enclosure character
}

#[derive(Debug)]
pub struct DelimitedLineTokenizer {
    max_inline_str_size: usize, // helper for compact string. This is the max that can get stack allocated. CompactString::with_capacity(0) does actually exactly this we well.
    save_skipped_lines: bool,
    pub delim_char: char,
    pub encl_char: Option<char>,
    pub skip_take_lines_fns: Option<Vec<Box<dyn SkipTakeLines + Send + Sync>>>, // needed here to skip lines while iterating
}

impl DelimitedLineTokenizer {
    pub fn new(
        delim: char,
        enclc: Option<char>,
        skip_take_lines_fns: Option<Vec<Box<dyn SkipTakeLines + Send + Sync>>>,
        save_skipped_lines: bool,
    ) -> Self {
        DelimitedLineTokenizer {
            max_inline_str_size: std::mem::size_of::<String>(),
            save_skipped_lines,
            delim_char: delim,
            encl_char: enclc,
            skip_take_lines_fns,
        }
    }

    pub fn csv(
        skip_take_lines_fns: Option<Vec<Box<dyn SkipTakeLines + Send + Sync>>>,
        save_skipped_lines: bool,
    ) -> Self {
        DelimitedLineTokenizer::new(',', Some('"'), skip_take_lines_fns, save_skipped_lines)
    }

    pub fn tsv(
        skip_take_lines_fns: Option<Vec<Box<dyn SkipTakeLines + Send + Sync>>>,
        save_skipped_lines: bool,
    ) -> Self {
        DelimitedLineTokenizer::new('\t', None, skip_take_lines_fns, save_skipped_lines)
    }

    pub fn tokenize_iter<'dlt, 'rd, R: Read>(
        &'dlt self,
        data: &'rd mut R,
    ) -> DelimitedLineTokenizerIter<'dlt, 'rd, R> {
        DelimitedLineTokenizerIter::new(self, data)
    }

    fn skip_line_by_skiptake_sanitizer(&self, line_counter: usize, line: &str) -> bool {
        // If we have filters, we apply them and see if we need to skip this line.
        if let Some(ref skip_take_lines) = self.skip_take_lines_fns {
            skip_take_lines
                .iter()
                .any(|filter| filter.skip(line_counter, line))
        } else {
            // If we have no filters, well, then don't skip anything.
            false
        }
    }

    fn tokenize_inner(
        &self,
        buf: &mut Vec<CompactString>,
        line_num: usize,
        s: &str,
    ) -> Result<VecDeque<String>> {
        let mut state = State::Start;

        // A small FSM here...
        for c in s.chars() {
            state = match state {
                State::Field => match c {
                    _ if c == self.delim_char => {
                        State::Scan // ready for next field
                    }
                    _ if Some(c) == self.encl_char => {
                        return Err(PattiCsvError::Tokenize(TokenizerError::IllegalEnclChar {
                            line: line_num,
                            token_num: buf.len(),
                        }))
                    }
                    _ => {
                        buf.last_mut().unwrap().push(c); // we know for sure, this is the last index and it exists!
                        State::Field
                    }
                },
                State::QuotedField => match c {
                    _ if Some(c) == self.encl_char => State::QuoteInQuotedField,
                    _ => {
                        buf.last_mut().unwrap().push(c); // we know for sure, this is the last index and it exists!
                        State::QuotedField
                    }
                },
                State::Scan | State::Start => match c {
                    _ if c == self.delim_char => {
                        // this means: empty field at start
                        buf.push(CompactString::with_capacity(self.max_inline_str_size));
                        State::Scan
                    }
                    _ if Some(c) == self.encl_char => {
                        // enclosure symbol (start) found
                        buf.push(CompactString::with_capacity(self.max_inline_str_size));
                        State::QuotedField
                    }
                    _ => {
                        // start of regular, un-enclosed field
                        let mut cs = CompactString::with_capacity(self.max_inline_str_size);
                        cs.push(c);
                        buf.push(cs);
                        State::Field
                    }
                },
                State::QuoteInQuotedField => match c {
                    _ if c == self.delim_char => State::Scan, // enlosure closed, ready for next field
                    _ if Some(c) == self.encl_char => {
                        // enclosure character escaped successfully
                        buf.last_mut().unwrap().push(c); // we know for sure, this is the last index and it exists!
                        State::QuotedField
                    }
                    _ => {
                        return Err(PattiCsvError::Tokenize(TokenizerError::UnescapedEnclChar {
                            line: line_num,
                            token_num: buf.len(),
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
                buf.push(CompactString::new(""));
            }
            State::QuotedField => {
                return Err(PattiCsvError::Tokenize(TokenizerError::UnescapedEnclChar {
                    line: line_num,
                    token_num: buf.len(),
                }))
            }
            _ => (),
        }

        let mut res: VecDeque<String> = VecDeque::with_capacity(buf.len());
        buf.iter()
            .for_each(|cs| res.push_back(String::from(cs.as_str())));

        Ok(res)
    }

    pub fn tokenize(&self, line_num: usize, s: &str) -> Result<VecDeque<String>> {
        let mut buf: Vec<CompactString> = Vec::with_capacity(10);
        self.tokenize_inner(&mut buf, line_num, s)
    }
}

pub struct DelimitedLineTokenizerIter<'dlt, 'rd, R: Read> {
    dlt: &'dlt DelimitedLineTokenizer,
    buf_raw_data: BufReader<&'rd mut R>,
    line_token_buf: Vec<CompactString>,
    stats: DelimitedLineTokenizerStats,
}

impl<'dlt, 'rd, R: Read> DelimitedLineTokenizerIter<'dlt, 'rd, R> {
    fn new(dlt: &'dlt DelimitedLineTokenizer, data: &'rd mut R) -> Self {
        Self {
            dlt,
            buf_raw_data: BufReader::new(data),
            stats: DelimitedLineTokenizerStats::default(),
            line_token_buf: Vec::with_capacity(10), // we default hard to 10 because, well, we gotta start somewhere
        }
    }

    pub fn get_stats(&self) -> &DelimitedLineTokenizerStats {
        &self.stats
    }
}

impl<'dlt, 'rd, R: Read> Iterator for DelimitedLineTokenizerIter<'dlt, 'rd, R> {
    type Item = Result<VecDeque<String>>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = String::new();
        let mut skip_this_line = true;

        while skip_this_line {
            line.clear();

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
            self.stats.num_lines_read += 1;
            self.stats.bytes_read += bytes_read.unwrap(); // unwrap is OK here, we checked every other path

            skip_this_line = self
                .dlt
                .skip_line_by_skiptake_sanitizer(self.stats.curr_line_num, &line);

            if skip_this_line {
                // additional info, only when configured
                self.stats.skipped_lines.push((
                    self.stats.curr_line_num,
                    if self.dlt.save_skipped_lines {
                        Some(line.clone())
                    } else {
                        None
                    },
                ));
            }
        }

        let tok_res = self.dlt.tokenize_inner(
            &mut self.line_token_buf,
            self.stats.curr_line_num,
            line.trim_end(),
        );
        if tok_res.is_ok() {
            self.stats.num_lines_tokenized += 1;
        }

        self.line_token_buf.clear();

        Some(tok_res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let mut test_data_cursor = std::io::Cursor::new("");

        let dlt = DelimitedLineTokenizer::csv(None, false);
        let mut dlt_iter = dlt.tokenize_iter(&mut test_data_cursor);
        let res = dlt_iter.next();

        assert_eq!(res, None);
    }

    fn test_it(inp: &str, exp: Vec<&str>) {
        let dlt = DelimitedLineTokenizer::csv(None, false);

        let mut test_data_cursor = std::io::Cursor::new(inp);
        let mut dlt_iter = dlt.tokenize_iter(&mut test_data_cursor);
        let res = dlt_iter.next().unwrap().unwrap();

        assert_eq!(res, exp);

        let mut test_data_cursor2 = std::io::Cursor::new(inp);
        let mut dlt_iter2 = dlt.tokenize_iter(&mut test_data_cursor2);
        let res2 = dlt_iter2.next().unwrap().unwrap();

        assert_eq!(res2, exp);
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

        let dlt = DelimitedLineTokenizer::csv(None, false);
        let mut dlt_iter = dlt.tokenize_iter(&mut test_data_cursor);
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

        let dlt = DelimitedLineTokenizer::csv(None, false);
        let mut dlt_iter = dlt.tokenize_iter(&mut test_data_cursor);
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

        let dlt = DelimitedLineTokenizer::tsv(None, false);
        let mut dlt_iter = dlt.tokenize_iter(&mut test_data_cursor);
        let res = dlt_iter.next().unwrap().unwrap();

        assert_eq!(res, vec!["foo", "b\"a'r", "b|az"]);
    }

    #[test]
    /// doesn't really work correctly, or does it?
    fn tab_separated_simple_enclosed() {
        let mut test_data_cursor = std::io::Cursor::new("foo\t\"b\tar\"\tbaz");

        let dlt = DelimitedLineTokenizer::new('\t', Some('"'), None, false);
        let mut dlt_iter = dlt.tokenize_iter(&mut test_data_cursor);

        let res = dlt_iter.next().unwrap().unwrap();
        assert_eq!(res, vec!["foo", "b\tar", "baz"]);
    }

    #[test]
    fn pipe_separated_simple_enclosed() {
        let mut test_data_cursor = std::io::Cursor::new("foo|\"b|ar\"|baz");

        let dlt = DelimitedLineTokenizer::new('|', Some('"'), None, false);
        let mut dlt_iter = dlt.tokenize_iter(&mut test_data_cursor);
        let res = dlt_iter.next().unwrap().unwrap();

        assert_eq!(res, vec!["foo", "b|ar", "baz"]);
    }

    #[test]
    fn pipe_separated_simple_enclosed2() {
        let mut test_data_cursor = std::io::Cursor::new("foo|'b|ar'|baz");

        let dlt = DelimitedLineTokenizer::new('|', Some('\''), None, false);
        let mut dlt_iter = dlt.tokenize_iter(&mut test_data_cursor);
        let res = dlt_iter.next().unwrap().unwrap();

        assert_eq!(res, vec!["foo", "b|ar", "baz"]);
    }

    #[test]
    fn multiple_lines_test_simple() {
        let mut test_data_cursor = std::io::Cursor::new("a,b,c\n1,2,3");

        let dlt = DelimitedLineTokenizer::csv(None, false);
        let mut dlt_iter = dlt.tokenize_iter(&mut test_data_cursor);

        let res = dlt_iter.next().unwrap().unwrap();
        assert_eq!(res, vec!["a", "b", "c"]);
        assert_eq!(dlt_iter.get_stats().curr_line_num, 1);
        assert_eq!(dlt_iter.get_stats().num_lines_tokenized, 1);
        assert_eq!(
            dlt_iter.get_stats().is_at_first_unskipped_line_to_parse(),
            true
        );

        let res = dlt_iter.next().unwrap().unwrap();
        assert_eq!(res, vec!["1", "2", "3"]);
        assert_eq!(dlt_iter.get_stats().num_lines_tokenized, 2);
        assert_eq!(dlt_iter.get_stats().curr_line_num, 2);

        println!("{:?}", &dlt_iter.get_stats())
    }
}
