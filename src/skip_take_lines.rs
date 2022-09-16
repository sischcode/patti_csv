use regex::Regex;
use std::fmt::Debug;

use crate::errors::{PattiCsvError, Result};

pub trait SkipTakeLines: Debug {
    fn skip(&self, line_num: usize, line_content: &str) -> bool;
    fn get_self_info(&self) -> String;
}

#[derive(Debug)]
pub struct SkipLinesFromStart {
    skip_num_lines: usize,
}
impl SkipLinesFromStart {
    pub fn new(skip_num_lines: usize) -> Self {
        Self { skip_num_lines }
    }
}
impl SkipTakeLines for SkipLinesFromStart {
    fn skip(&self, line_num: usize, _line_content: &str) -> bool {
        line_num <= self.skip_num_lines
    }
    fn get_self_info(&self) -> String {
        format!("{self:?}")
    }
}

#[derive(Debug)]
pub struct SkipLinesStartingWith {
    starts_with: String,
}
impl SkipLinesStartingWith {
    pub fn new<T>(starts_with: T) -> Self
    where
        T: Into<String> + Debug,
    {
        Self {
            starts_with: starts_with.into(),
        }
    }
}
impl SkipTakeLines for SkipLinesStartingWith {
    fn skip(&self, _line_num: usize, line_content: &str) -> bool {
        line_content.starts_with(&self.starts_with)
    }
    fn get_self_info(&self) -> String {
        format!("{self:?}")
    }
}

#[derive(Debug)]
pub struct SkipLinesByRegex {
    regex: Regex,
}
impl SkipLinesByRegex {
    pub fn new<T>(regex_pattern: T) -> Result<Self>
    where
        T: AsRef<str> + Debug,
    {
        let re = Regex::new(regex_pattern.as_ref()).map_err(|e| {
            PattiCsvError::ConfigError {msg: format!("[ERROR_ON_REGEX_COMPILE] Cannot create SkipLinesByRegex by given regex str={}. Error: {}", regex_pattern.as_ref(), e)}
        })?;
        Ok(Self { regex: re })
    }
}
impl SkipTakeLines for SkipLinesByRegex {
    fn skip(&self, _line_num: usize, line_content: &str) -> bool {
        self.regex.is_match(line_content)
    }

    fn get_self_info(&self) -> String {
        format!("{self:?}")
    }
}

#[derive(Debug)]
pub struct SkipEmptyLines {}
impl SkipEmptyLines {
    pub fn new() -> Self {
        Self {}
    }
}
impl SkipTakeLines for SkipEmptyLines {
    fn skip(&self, _line_num: usize, line_content: &str) -> bool {
        line_content.eq("\n") || line_content.eq("\r\n") // nothing there besides newline
    }
    fn get_self_info(&self) -> String {
        format!("{self:?}")
    }
}

#[cfg(test)]
mod tests {
    use crate::skip_take_lines::*;

    fn test_data_01() -> Vec<&'static str> {
        vec![
            "Some Bullshit\n",
            "# bullshit\n",
            "\n",
            "column1,column2,column3,column4,column5\n",
            r###""SOMEDATA   ",1,10.12,"true",eur\n"###,
            r###""SOMEDATA   ",2,10.12,"true",eur\n"###,
            r###""SOMEDATA   ",3,10.12,"true",eur\n"###,
            r###""","","","Totals:",5"###,
        ]
    }

    #[test]
    fn skip_one_lines_from_start() {
        let check_line = SkipLinesFromStart { skip_num_lines: 1 };
        let to_skip = test_data_01()
            .iter()
            .enumerate()
            .map(|(i, &s)| check_line.skip(i + 1, s))
            .collect::<Vec<bool>>();

        assert_eq![
            vec![true, false, false, false, false, false, false, false],
            to_skip
        ];
    }

    #[test]
    fn skip_lines_by_starts_with_hashbang() {
        let check_line = SkipLinesStartingWith {
            starts_with: "#".into(),
        };

        let to_skip = test_data_01()
            .iter()
            .enumerate()
            .map(|(i, &s)| check_line.skip(i + 1, s))
            .collect::<Vec<bool>>();

        assert_eq![
            vec![false, true, false, false, false, false, false, false],
            to_skip
        ];
    }

    #[test]
    fn skip_lines_by_regex_empty_column_with_total() {
        let check_line = SkipLinesByRegex::new(r###"^"","","","Totals:",.*"###).unwrap();

        let to_skip = test_data_01()
            .iter()
            .enumerate()
            .map(|(i, &s)| check_line.skip(i + 1, s))
            .collect::<Vec<bool>>();

        assert_eq![
            vec![false, false, false, false, false, false, false, true],
            to_skip
        ];
    }

    #[test]
    fn skip_empty_rows() {
        let check_line = SkipEmptyLines {};
        let to_skip = test_data_01()
            .iter()
            .enumerate()
            .map(|(i, &s)| check_line.skip(i + 1, s))
            .collect::<Vec<bool>>();

        assert_eq![
            vec![false, false, true, false, false, false, false, false],
            to_skip
        ];
    }
}
