use strum_macros::Display;
use thiserror::Error;

use venum::{errors::VenumError, venum::Value};

#[derive(Debug, PartialEq, Display, Clone)]
pub enum WrappedErrors {
    VenumError(VenumError),
}

#[derive(Debug, PartialEq, Display, Clone)]
pub enum PattiCsvError {
    Generic { msg: String },
    Wrapped(WrappedErrors),
    Split(SplitError),
    Tokenize(TokenizerError),
    Sanitize(SanitizeError),
}

impl From<std::io::Error> for PattiCsvError {
    fn from(e: std::io::Error) -> Self {
        PattiCsvError::Generic { msg: e.to_string() }
    }
}

impl From<VenumError> for PattiCsvError {
    fn from(ve: VenumError) -> Self {
        PattiCsvError::Wrapped(WrappedErrors::VenumError(ve))
    }
}

pub type Result<T> = std::result::Result<T, PattiCsvError>;

#[derive(Error, Debug, PartialEq, Clone)]
#[error("error: {msg:?}; problem value: {src_val:?}; detail: {detail:?}")]
pub struct SplitError {
    msg: String,
    src_val: Option<Value>,
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
    pub fn from(msg: String, src_val: Option<Value>, detail: Option<String>) -> Self {
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
