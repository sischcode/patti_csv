use crate::errors::{PattiCsvError, Result, SanitizeError};
use regex::Regex;

pub trait TransformSanitizeToken {
    fn transitize(&self, input_token: String) -> Result<String>; // TODO: &str?
    fn get_info(&self) -> String {
        String::from("n/a")
    }
}

#[derive(Debug)]
pub struct ReplaceWith {
    pub from: String,
    pub to: String,
}
impl TransformSanitizeToken for ReplaceWith {
    fn transitize(&self, input_token: String) -> Result<String> {
        Ok(input_token.replace(self.from.as_str(), self.to.as_str()))
    }
    fn get_info(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Debug)]
pub struct Eradicate {
    pub eradicate: String,
}
impl TransformSanitizeToken for Eradicate {
    fn transitize(&self, input_token: String) -> Result<String> {
        Ok(input_token.replace(self.eradicate.as_str(), ""))
    }
    fn get_info(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Debug)]
pub struct ToLowercase;
impl TransformSanitizeToken for ToLowercase {
    fn transitize(&self, input_token: String) -> Result<String> {
        Ok(input_token.to_lowercase())
    }
    fn get_info(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Debug)]
pub struct ToUppercase;
impl TransformSanitizeToken for ToUppercase {
    fn transitize(&self, input_token: String) -> Result<String> {
        Ok(input_token.to_uppercase())
    }
    fn get_info(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Debug)]
pub struct TrimLeading;
impl TransformSanitizeToken for TrimLeading {
    fn transitize(&self, input_token: String) -> Result<String> {
        Ok(input_token.trim_start().into())
    }
    fn get_info(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Debug)]
pub struct TrimTrailing;
impl TransformSanitizeToken for TrimTrailing {
    fn transitize(&self, input_token: String) -> Result<String> {
        Ok(input_token.trim_end().into())
    }
    fn get_info(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Debug)]
pub struct TrimAll;
impl TransformSanitizeToken for TrimAll {
    fn transitize(&self, input_token: String) -> Result<String> {
        Ok(input_token.trim().into())
    }
    fn get_info(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Debug)]
pub struct RegexTake {
    pub regex: Regex,
}
impl RegexTake {
    pub fn new(regex_pattern: &str) -> Self {
        Self {
            regex: Regex::new(regex_pattern)
                .map_err(|e| {
                    PattiCsvError::Sanitize(SanitizeError::minim(
                        format!("{}", e),
                        "ERROR_ON_REGEX_COMPILE".into(),
                    ))
                })
                .unwrap(), // TODO
        }
    }
}
impl TransformSanitizeToken for RegexTake {
    fn transitize(&self, input_token: String) -> Result<String> {
        let caps = self
            .regex
            .captures(&input_token)
            .ok_or(PattiCsvError::Sanitize(SanitizeError::minim(
                "No captures, but we need exactly one.".into(),
                input_token.clone(),
            )))?;

        let token_match = caps
            .get(1)
            .ok_or(PattiCsvError::Sanitize(SanitizeError::minim(
                "No capture group#1.".into(),
                input_token.clone(),
            )))?;

        Ok(String::from(token_match.as_str()))
    }
    fn get_info(&self) -> String {
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
            RegexTake::new("(\\d+\\.\\d+).*".into()).transitize("10.00 (CHF)".into())
        );
    }

    #[test]
    fn test_regex_take_err() {
        assert_eq!(
            Err(PattiCsvError::Sanitize(SanitizeError::minim(
                "No captures, but we need exactly one.".into(),
                "1000 (CHF)".into(),
            ))),
            RegexTake::new("(\\d+\\.\\d+).*".into()).transitize("1000 (CHF)".into())
        );
    }

    #[test]
    fn test_regex_take_err2() {
        assert_eq!(
            Err(PattiCsvError::Sanitize(SanitizeError::minim(
                "No capture group#1.".into(),
                "1000 (CHF)".into(),
            ))),
            RegexTake::new("".into()).transitize("1000 (CHF)".into())
        );
    }

    #[test]
    fn test_replace_with_oneinstance() {
        assert_eq!(
            Ok("foobar".into()),
            ReplaceWith {
                from: "baz".into(),
                to: "bar".into()
            }
            .transitize("foobaz".into())
        );
    }

    #[test]
    fn test_replace_with_allinstances() {
        assert_eq!(
            Ok("barfoobar".into()),
            ReplaceWith {
                from: "baz".into(),
                to: "bar".into()
            }
            .transitize("bazfoobaz".into())
        );
    }

    #[test]
    fn test_eradicate_with_oneinstance() {
        assert_eq!(
            Ok("foo".into()),
            Eradicate {
                eradicate: "baz".into()
            }
            .transitize("foobaz".into())
        );
    }

    #[test]
    fn test_eradicate_with_allinstances() {
        assert_eq!(
            Ok("foo".into()),
            Eradicate {
                eradicate: "baz".into()
            }
            .transitize("bazfoobaz".into())
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
            ToUppercase {}.transitize("FoObAr".into())
        );
    }

    #[test]
    fn test_trim_leading() {
        assert_eq!(
            Ok("foobar".into()),
            TrimLeading {}.transitize("  foobar".into())
        );
    }

    #[test]
    fn test_trim_trailing() {
        assert_eq!(
            Ok("foobar".into()),
            TrimTrailing {}.transitize("foobar  ".into())
        );
    }

    #[test]
    fn test_trim() {
        assert_eq!(
            Ok("foobar".into()),
            TrimAll {}.transitize("  foobar  ".into())
        );
    }
}
