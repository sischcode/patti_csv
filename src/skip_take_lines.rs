pub trait SkipTakeLines {
    fn skip(&self, line_num: Option<usize>, line_content: Option<&str>) -> bool;
}

#[derive(Debug)]
pub struct SkipLinesFromStart {
    pub skip_num_lines: usize,
}
impl SkipTakeLines for SkipLinesFromStart {
    fn skip(&self, line_num: Option<usize>, _line_content: Option<&str>) -> bool {
        match line_num {
            Some(ln) => ln <= self.skip_num_lines,
            None => false,
        }
    }
}

#[derive(Debug)]
pub struct SkipLinesFromEnd {
    pub skip_num_lines: usize,
    pub lines_total: usize,
}
impl SkipTakeLines for SkipLinesFromEnd {
    fn skip(&self, line_num: Option<usize>, _line_content: Option<&str>) -> bool {
        match line_num {
            Some(ln) => ln > self.lines_total - self.skip_num_lines,
            None => false,
        }
    }
}

#[derive(Debug)]
pub struct SkipLinesStartingWith {
    pub starts_with: String,
}
impl SkipTakeLines for SkipLinesStartingWith {
    fn skip(&self, _line_num: Option<usize>, line_content: Option<&str>) -> bool {
        match line_content {
            Some(c) => c.starts_with(&self.starts_with),
            None => false,
        }
    }
}

#[derive(Debug)]
pub struct TakeLinesStartingWith {
    pub starts_with: String,
}
impl SkipTakeLines for TakeLinesStartingWith {
    fn skip(&self, _line_num: Option<usize>, line_content: Option<&str>) -> bool {
        match line_content {
            Some(c) => !c.starts_with(&self.starts_with),
            None => false,
        }
    }
}

#[derive(Debug)]
pub struct SkipEmptyLines {}
impl SkipTakeLines for SkipEmptyLines {
    fn skip(&self, _line_num: Option<usize>, line_content: Option<&str>) -> bool {
        match line_content {
            Some(c) => c.eq("\n") || c.eq("\r\n"), // nothing there besides newline
            None => false,
        }
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
            "\"SOMEDATA   \",1,10.12,\"true\",eur\n",
            "\"SOMEDATA   \",2,10.12,\"true\",eur\n",
            "\"SOMEDATA   \",3,10.12,\"true\",eur\n",
        ]
    }

    #[test]
    fn skip_one_lines_from_start() {
        let check_line = SkipLinesFromStart { skip_num_lines: 1 };
        let to_skip = test_data_01()
            .iter()
            .enumerate()
            .map(|(i, &s)| check_line.skip(Some(i + 1), Some(s)))
            .collect::<Vec<bool>>();

        assert_eq![
            vec![true, false, false, false, false, false, false],
            to_skip
        ];
    }

    #[test]
    fn skip_one_lines_from_end() {
        let csv = test_data_01();
        let check_line = SkipLinesFromEnd {
            skip_num_lines: 1,
            lines_total: csv.len() as usize,
        };
        let to_skip = csv
            .iter()
            .enumerate()
            .map(|(i, &s)| check_line.skip(Some(i + 1), Some(s)))
            .collect::<Vec<bool>>();

        assert_eq![
            vec![false, false, false, false, false, false, true],
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
            .map(|(i, &s)| check_line.skip(Some(i + 1), Some(s)))
            .collect::<Vec<bool>>();

        assert_eq![
            vec![false, true, false, false, false, false, false],
            to_skip
        ];
    }

    #[test]
    fn take_lines_by_starts_with_hashbang() {
        let check_line = TakeLinesStartingWith {
            starts_with: "#".into(),
        };
        let to_skip = test_data_01()
            .iter()
            .enumerate()
            .map(|(i, &s)| check_line.skip(Some(i + 1), Some(s)))
            .collect::<Vec<bool>>();

        assert_eq![vec![true, false, true, true, true, true, true], to_skip];
    }

    #[test]
    fn skip_empty_rows() {
        let check_line = SkipEmptyLines {};
        let to_skip = test_data_01()
            .iter()
            .enumerate()
            .map(|(i, &s)| check_line.skip(Some(i + 1), Some(s)))
            .collect::<Vec<bool>>();

        assert_eq![
            vec![false, false, true, false, false, false, false],
            to_skip
        ];
    }
}
