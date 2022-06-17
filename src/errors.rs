use std::fmt::Display;
use thiserror::Error;

use crate::data::csv_value::CsvValue;

#[derive(Debug, PartialEq, Clone)]
pub enum PattiCsvError {
    Generic { msg: String },
    Conversion(ConversionError),
    Split(SplitError),
    Tokenize(TokenizerError),
    Sanitize(SanitizeError),
}

impl Display for PattiCsvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PattiCsvError::Generic { msg } => write!(f, "An error occurred: {:?}", msg),
            PattiCsvError::Conversion(ce) => ce.fmt(f),
            PattiCsvError::Tokenize(te) => te.fmt(f),
            PattiCsvError::Split(se) => se.fmt(f),
            PattiCsvError::Sanitize(se) => se.fmt(f),
        }
    }
}

impl From<std::io::Error> for PattiCsvError {
    fn from(e: std::io::Error) -> Self {
        PattiCsvError::Generic { msg: e.to_string() } // we need to find something better here
    }
}

pub type Result<T> = std::result::Result<T, PattiCsvError>;

#[derive(Error, Debug, PartialEq, Clone)]
pub enum ConversionError {
    #[error("Can't unwrap Value::{src_value:?} to basic type {basic_type:?}")]
    UnwrapToBaseTypeFailed {
        src_value: String,
        basic_type: &'static str,
    },
    #[error("Can't construct Value::{target_type:?} from string '{src_value:?}'")]
    ValueFromStringFailed {
        src_value: String,
        target_type: &'static str,
    },
}

#[derive(Error, Debug, PartialEq, Clone)]
#[error("error: {msg:?}; problem value: {src_val:?}; detail: {detail:?}")]
pub struct SplitError {
    msg: String,
    src_val: Option<CsvValue>,
    detail: Option<String>,
}
impl SplitError {
    pub fn minim(msg: String) -> Self {
        Self {
            msg,
            src_val: None,
            detail: None,
        }
    }
    pub fn from(msg: String, src_val: Option<CsvValue>, detail: Option<String>) -> Self {
        Self {
            msg,
            src_val,
            detail,
        }
    }
}

#[derive(Error, Debug, PartialEq, Clone)]
pub enum TokenizerError {
    #[error("Enclosure character used in regular, non-enclosed field. Line: {line:?}, token_num: {token_num:?}")]
    IllegalEnclChar { line: usize, token_num: usize },
    #[error("Enclosure character in enclosed field not properly escaped. Line: {line:?}, token_num: {token_num:?}")]
    UnescapedEnclChar { line: usize, token_num: usize },
}

#[derive(Error, Debug, PartialEq, Clone)]
#[error("line: {line:?}, column: {column:?}, from_token: {from_token:?}, msg: {msg:?}")]
pub struct SanitizeError {
    msg: String,
    line: Option<usize>,
    column: Option<usize>,
    from_token: String,
}
impl SanitizeError {
    pub fn minim(msg: String, from_token: String) -> Self {
        Self {
            msg,
            line: None,
            column: None,
            from_token,
        }
    }
    pub fn extend(
        se: SanitizeError,
        msg: Option<String>,
        line: Option<usize>,
        column: Option<usize>,
    ) -> Self {
        Self {
            msg: if let Some(m) = msg {
                let mut extended_msg = se.msg;
                extended_msg.push_str(m.as_str());
                extended_msg
            } else {
                se.msg
            },
            line: if let Some(l) = line { Some(l) } else { se.line },
            column: if let Some(c) = column {
                Some(c)
            } else {
                se.column
            },
            from_token: se.from_token,
        }
    }
}
