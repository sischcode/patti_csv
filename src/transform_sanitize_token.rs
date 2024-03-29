use regex::Regex;
use std::fmt::Debug;

use crate::errors::{PattiCsvError, Result, SanitizeError};

pub trait TransformSanitizeToken: Debug {
    fn transitize(&self, input_token: &str) -> Result<String>;
    fn get_self_info(&self) -> String {
        String::from("n/a")
    }
}

#[derive(Debug)]
pub struct ReplaceWith {
    from: String,
    to: String,
}
impl ReplaceWith {
    pub fn new<T>(from: T, to: T) -> Self
    where
        T: Into<String> + Debug,
    {
        Self {
            from: from.into(),
            to: to.into(),
        }
    }
}
impl TransformSanitizeToken for ReplaceWith {
    fn transitize(&self, input_token: &str) -> Result<String> {
        Ok(input_token.replace(self.from.as_str(), self.to.as_str()))
    }
    fn get_self_info(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Debug)]
pub struct Eradicate {
    eradicate: String,
}
impl Eradicate {
    pub fn new<T>(eradicate: T) -> Self
    where
        T: Into<String> + Debug,
    {
        Self {
            eradicate: eradicate.into(),
        }
    }
}
impl TransformSanitizeToken for Eradicate {
    fn transitize(&self, input_token: &str) -> Result<String> {
        Ok(input_token.replace(self.eradicate.as_str(), ""))
    }
    fn get_self_info(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Debug)]
pub struct ToLowercase;
impl ToLowercase {
    pub fn new() -> Self {
        Self {}
    }
}
impl TransformSanitizeToken for ToLowercase {
    fn transitize(&self, input_token: &str) -> Result<String> {
        Ok(input_token.to_lowercase())
    }
    fn get_self_info(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Debug)]
pub struct ToUppercase;
impl ToUppercase {
    pub fn new() -> Self {
        Self {}
    }
}
impl TransformSanitizeToken for ToUppercase {
    fn transitize(&self, input_token: &str) -> Result<String> {
        Ok(input_token.to_uppercase())
    }
    fn get_self_info(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Debug)]
pub struct TrimLeading;
impl TrimLeading {
    pub fn new() -> Self {
        Self {}
    }
}
impl TransformSanitizeToken for TrimLeading {
    fn transitize(&self, input_token: &str) -> Result<String> {
        Ok(input_token.trim_start().into())
    }
    fn get_self_info(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Debug)]
pub struct TrimTrailing;
impl TrimTrailing {
    pub fn new() -> Self {
        Self {}
    }
}
impl TransformSanitizeToken for TrimTrailing {
    fn transitize(&self, input_token: &str) -> Result<String> {
        Ok(input_token.trim_end().into())
    }
    fn get_self_info(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Debug)]
pub struct TrimAll;
impl TrimAll {
    pub fn new() -> Self {
        Self {}
    }
}
impl TransformSanitizeToken for TrimAll {
    fn transitize(&self, input_token: &str) -> Result<String> {
        Ok(input_token.trim().into())
    }
    fn get_self_info(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Debug)]
pub struct RegexTake {
    regex: Regex,
}
impl RegexTake {
    pub fn new<T>(regex_pattern: T) -> Result<Self>
    where
        T: AsRef<str> + Debug,
    {
        let re = Regex::new(regex_pattern.as_ref()).map_err(|e| {
            PattiCsvError::Sanitize(SanitizeError::minim(
                format!("{}", e),
                "ERROR_ON_REGEX_COMPILE".into(),
            ))
        })?;
        Ok(Self { regex: re })
    }
}
impl TransformSanitizeToken for RegexTake {
    fn transitize(&self, input_token: &str) -> Result<String> {
        let caps = self.regex.captures(input_token).ok_or_else(|| {
            PattiCsvError::Sanitize(SanitizeError::minim(
                "No captures, but we need exactly one.".into(),
                input_token.to_string(),
            ))
        })?;

        let token_match = caps.get(1).ok_or_else(|| {
            PattiCsvError::Sanitize(SanitizeError::minim(
                "No capture group#1.".into(),
                input_token.to_string(),
            ))
        })?;

        Ok(String::from(token_match.as_str()))
    }
    fn get_self_info(&self) -> String {
        format!("{:?}", self)
    }
}

#[cfg(test)]
mod tests {
    use crate::transform_sanitize_token::*;

    #[test]
    fn test_regex_take() {
        assert_eq!(
            Ok("10.00".into()),
            RegexTake::new("(\\d+\\.\\d+).*")
                .unwrap()
                .transitize("10.00 (CHF)".into())
        );
    }

    #[test]
    fn test_regex_take_err() {
        assert_eq!(
            Err(PattiCsvError::Sanitize(SanitizeError::minim(
                "No captures, but we need exactly one.".into(),
                "1000 (CHF)".into(),
            ))),
            RegexTake::new("(\\d+\\.\\d+).*")
                .unwrap()
                .transitize("1000 (CHF)".into())
        );
    }

    #[test]
    fn test_regex_take_err2() {
        assert_eq!(
            Err(PattiCsvError::Sanitize(SanitizeError::minim(
                "No capture group#1.".into(),
                "1000 (CHF)".into(),
            ))),
            RegexTake::new("").unwrap().transitize("1000 (CHF)".into())
        );
    }

    #[test]
    fn test_replace_with_oneinstance() {
        assert_eq!(
            Ok("foobar".into()),
            ReplaceWith::new("baz", "bar").transitize("foobaz".into())
        );
    }

    #[test]
    fn test_replace_with_allinstances() {
        assert_eq!(
            Ok("barfoobar".into()),
            ReplaceWith::new("baz", "bar").transitize("bazfoobaz".into())
        );
    }

    #[test]
    fn test_eradicate_with_oneinstance() {
        assert_eq!(
            Ok("foo".into()),
            Eradicate::new("baz").transitize("foobaz".into())
        );
    }

    #[test]
    fn test_eradicate_with_allinstances() {
        assert_eq!(
            Ok("foo".into()),
            Eradicate::new("baz").transitize("bazfoobaz".into())
        );
    }

    #[test]
    fn test_to_lowercase() {
        assert_eq!(
            Ok("foobar".into()),
            ToLowercase {}.transitize("FoObAr".into())
        );
    }

    #[test]
    fn test_to_uppercase() {
        assert_eq!(
            Ok("FOOBAR".into()),
            ToUppercase::new().transitize("FoObAr".into())
        );
    }

    #[test]
    fn test_trim_leading() {
        assert_eq!(
            Ok("foobar".into()),
            TrimLeading::new().transitize("  foobar".into())
        );
    }

    #[test]
    fn test_trim_trailing() {
        assert_eq!(
            Ok("foobar".into()),
            TrimTrailing::new().transitize("foobar  ".into())
        );
    }

    #[test]
    fn test_trim() {
        assert_eq!(
            Ok("foobar".into()),
            TrimAll::new().transitize("  foobar  ".into())
        );
    }
}
