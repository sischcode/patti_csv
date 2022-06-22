use regex::Regex;
use venum::venum::Value;

use crate::errors::{PattiCsvError, Result, SplitError};

pub trait SplitValue {
    fn split(&self, src: &Option<Value>) -> Result<(Option<Value>, Option<Value>)>;
}

pub trait SplitValueMulti {
    fn split(&self, src: &Option<Value>) -> Result<Vec<Option<Value>>>;
}

#[derive(Debug)]
pub struct ValueStringSeparatorCharSplitter {
    pub sep_char: char,
    pub split_none: bool,
}

impl SplitValue for ValueStringSeparatorCharSplitter {
    fn split(&self, src: &Option<Value>) -> Result<(Option<Value>, Option<Value>)> {
        if let Some(val) = src {
            match val {
                Value::String(s) => {
                    let splitted: Vec<&str> = s.split(self.sep_char).collect();
                    if splitted.len() != 2 {
                        return Err(PattiCsvError::Split(SplitError::from(
                            format!(
                                "expected 2 tokens as result of split, but got: {}",
                                splitted.len()
                            ),
                            src.clone(),
                            None,
                        )));
                    }
                    return Ok((
                        Some(Value::from(String::from(splitted[0]))),
                        Some(Value::from(String::from(splitted[1]))),
                    ));
                }
                _ => Err(PattiCsvError::Split(SplitError::minim(String::from(
                    "Not a Value::String. Can't split.",
                )))),
            }
        } else if src.is_none() && self.split_none {
            Ok((None, None))
        } else {
            Err(PattiCsvError::Split(SplitError::minim(String::from(
                "Value is None, but split_none is false",
            ))))
        }
    }
}

#[derive(Debug)]
pub struct ValueStringSeparatorCharSplitterMulti {
    pub sep_char: char,
    pub split_none: bool,
    pub split_none_into_num_clones: Option<usize>,
}

impl SplitValueMulti for ValueStringSeparatorCharSplitterMulti {
    fn split(&self, src: &Option<Value>) -> Result<Vec<Option<Value>>> {
        match src {
            None => match (&self.split_none, &self.split_none_into_num_clones) {
                (true, None) =>  Err(PattiCsvError::Split(SplitError::minim(String::from(
                    "Value is None, split_none is true, but split_none_into_num_clones is not set. Can't split into undefined number of targets!",
                )))),
                (false, _) => Err(PattiCsvError::Split(SplitError::minim(String::from(
                    "Value is None but split_none is false. Not allowed to split!",
                )))),
                (true, Some(num_targets)) => {
                    let mut v: Vec<Option<Value>> = Vec::with_capacity(*num_targets);
                    for _ in 1..=*num_targets {
                        v.push(None);
                    }
                    Ok(v)
                }                
            },
            Some(val) => {
                match val {
                    Value::String(s) => {
                        match s.is_empty() {
                            true => Err(PattiCsvError::Split(SplitError::minim(String::from(
                                "Source Value is empty. Can't split.", // TODO: None!?
                            )))),
                            false => {
                                let splitted: Vec<&str> = s.split(self.sep_char).collect(); // this will never return a length of 0, as it's implemented by rust!
                                match splitted.len() {
                                    1 => Err(PattiCsvError::Split(SplitError::minim(String::from(
                                        "Value is None, but split_none is false",
                                    )))),
                                    _ => Ok(splitted
                                        .into_iter()
                                        .map(|v| Some(Value::from(String::from(v))))
                                        .collect()),
                                }
                            }
                        }                        
                    },
                    _ => Err(PattiCsvError::Split(SplitError::minim(String::from(
                        "Not a Value::String. Can't split.",
                    )))),
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ValueStringRegexPairSplitter {
    pub re: Regex,
    pub split_none: bool,
}

impl ValueStringRegexPairSplitter {
    pub fn from(regex_pattern: String, split_none: bool) -> Result<Self> {
        let re = Regex::new(regex_pattern.as_str()).map_err(|e| {
            let mut err_msg = format!("{}", e);
            err_msg.push_str(" (RegexPairSplitter, ERROR_ON_REGEX_COMPILE)");
            PattiCsvError::Split(SplitError::minim(err_msg))
        })?;
        Ok(ValueStringRegexPairSplitter { re, split_none })
    }
}

impl SplitValue for ValueStringRegexPairSplitter {
    fn split(&self, src: &Option<Value>) -> Result<(Option<Value>, Option<Value>)> {
        if let Some(val) = src {
            match val {
                Value::String(s) => {
                    let caps =
                        self.re
                            .captures(&s)
                            .ok_or(PattiCsvError::Split(SplitError::from(
                                String::from("No captures, but we need exactly two."),
                                src.clone(),
                                Some(String::from(self.re.as_str())),
                            )))?;
                    if caps.len() == 3 {
                        let token_match_1 = caps.get(1).unwrap().as_str();
                        let token_match_2 = caps.get(2).unwrap().as_str();
                        Ok((
                            Some(Value::String(String::from(token_match_1))),
                            Some(Value::String(String::from(token_match_2))),
                        ))
                    } else {
                        Err(PattiCsvError::Split(SplitError::from(
                            String::from("Not enough/too much capture groups!"),
                            src.clone(),
                            Some(String::from(self.re.as_str())),
                        )))
                    }
                }
                _ => Err(PattiCsvError::Split(SplitError::minim(String::from(
                    "Not a Value::String. Can't split.",
                )))),
            }
        } else if src.is_none() && self.split_none {
            Ok((None, None))
        } else {
            Err(PattiCsvError::Split(SplitError::minim(String::from(
                "Value is None, but split_none is false",
            ))))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_seperator_char() {
        let sep = ValueStringSeparatorCharSplitter {
            sep_char: ' ',
            split_none: true,
        };
        let data = Some(Value::from("foo bar".to_string()));
        let split_res = sep.split(&data);
        assert!(split_res.is_ok());
        let split_vals = split_res.unwrap();
        assert_eq!(Some(Value::from("foo".to_string())), split_vals.0);
        assert_eq!(Some(Value::from("bar".to_string())), split_vals.1);
    }

    #[test]
    fn test_split_regex_pair() {
        let sep_res =
            ValueStringRegexPairSplitter::from("(\\d+\\.\\d+).*(\\d+\\.\\d+)".to_string(), true);
        assert!(sep_res.is_ok());
        let sep = sep_res.unwrap();

        let data = Some(Value::from("1.12 2.23".to_string()));
        let split_res = sep.split(&data);
        assert!(split_res.is_ok());
        let split_vals = split_res.unwrap();
        assert_eq!(Some(Value::from("1.12".to_string())), split_vals.0);
        assert_eq!(Some(Value::from("2.23".to_string())), split_vals.1);
    }

    #[test]
    fn test_split_regex_pair_illegal_regex() {
        let sep_res = ValueStringRegexPairSplitter::from("FWPUJWDJW/)!(!()?))".to_string(), true);
        assert!(sep_res.is_err());
    }
}
