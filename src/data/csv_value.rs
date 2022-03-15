use crate::errors::{ConversionError, PattiCsvError, Result};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::convert::From;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum CsvValue {
    Null,
    String(String),
    Int8(i8),
    Int32(i32),
    Int64(i64),
    Int128(i128),
    Float32(f32),
    Float64(f64),
    Bool(bool),
    Decimal(Decimal),
    Date(String), //TODO
    NaiveDate(NaiveDate),
}

macro_rules! impl_from_type_for_value {
    ($enum_type:ident, $type:ty) => {
        impl From<$type> for CsvValue {
            fn from(item: $type) -> Self {
                CsvValue::$enum_type(item)
            }
        }
    };
}
impl_from_type_for_value!(String, String);
impl_from_type_for_value!(Int8, i8);
impl_from_type_for_value!(Int32, i32);
impl_from_type_for_value!(Int64, i64);
impl_from_type_for_value!(Int128, i128);
impl_from_type_for_value!(Float32, f32);
impl_from_type_for_value!(Float64, f64);
impl_from_type_for_value!(Bool, bool);
impl_from_type_for_value!(Decimal, Decimal);
impl_from_type_for_value!(NaiveDate, NaiveDate);

macro_rules! impl_from_value_for_result {
    ($enum_type:ident, $type:ty) => {
        impl From<CsvValue> for Result<$type> {
            fn from(item: CsvValue) -> Self {
                match item {
                    CsvValue::$enum_type(v) => Ok(v),
                    _ => Err(PattiCsvError::Conversion(
                        ConversionError::UnwrapToBaseTypeFailed {
                            src_value: format!("{:?}", item),
                            basic_type: stringify!($type),
                        },
                    )),
                }
            }
        }
    };
}
impl_from_value_for_result!(String, String);
impl_from_value_for_result!(Int8, i8);
impl_from_value_for_result!(Int32, i32);
impl_from_value_for_result!(Int64, i64);
impl_from_value_for_result!(Int128, i128);
impl_from_value_for_result!(Float32, f32);
impl_from_value_for_result!(Float64, f64);
impl_from_value_for_result!(Bool, bool);
impl_from_value_for_result!(Decimal, Decimal);
impl_from_value_for_result!(NaiveDate, NaiveDate);

macro_rules! from_type_string {
    ($fn_name:ident, $enum_type:ident, $type:ty) => {
        pub fn $fn_name(v: String) -> Result<CsvValue> {
            let temp = v.parse::<$type>().map_err(|_| {
                PattiCsvError::Conversion(ConversionError::ValueFromStringFailed {
                    src_value: v,
                    target_type: stringify!($enum_type),
                })
            })?;
            Ok(CsvValue::$enum_type(temp))
        }
    };
}

macro_rules! type_defaults {
    ($fn_name:ident, $enum_type:ident, $type:ty) => {
        pub fn $fn_name() -> CsvValue {
            CsvValue::$enum_type(<$type>::default())
        }
    };
}

impl CsvValue {
    pub fn decimal_default() -> Self {
        CsvValue::Decimal(0i8.into())
    }
    pub fn date_default() -> Self {
        CsvValue::Date(String::from(""))
    }
    pub fn naive_date_default() -> Self {
        CsvValue::NaiveDate(NaiveDate::from_ymd(0, 0, 0))
    }

    type_defaults!(string_default, String, String);
    type_defaults!(int8_default, Int8, i8);
    type_defaults!(int32_default, Int32, i32);
    type_defaults!(int64_default, Int64, i64);
    type_defaults!(int128_default, Int128, i128);
    type_defaults!(float32_default, Float32, f32);
    type_defaults!(float64_default, Float64, f64);
    type_defaults!(bool_default, Bool, bool);
    type_defaults!(decimal_default, Decimal, Decimal);

    from_type_string!(from_int8_string, Int8, i8);
    from_type_string!(from_int32_string, Int32, i32);
    from_type_string!(from_int64_string, Int64, i64);
    from_type_string!(from_int128_string, Int128, i128);
    from_type_string!(from_float32_string, Float32, f32);
    from_type_string!(from_float64_string, Float64, f64);
    from_type_string!(from_bool_string, Bool, bool);
    // TODO: from_decimal_string via Decimal::from_str("...")

    /// Since we cannot really use the FromStr trait here...
    pub fn from_string_with_templ(v: String, templ_type: &CsvValue) -> Result<Option<CsvValue>> {
        if v == "".to_string() {
            return Ok(None);
        }
        match templ_type {
            CsvValue::String(_) => Ok(Some(CsvValue::String(v))),
            CsvValue::Int8(_) => {
                let temp = v.parse::<i8>().map_err(|_| {
                    PattiCsvError::Conversion(ConversionError::ValueFromStringFailed {
                        src_value: v.clone(),
                        target_type: "Int8",
                    })
                })?;
                Ok(Some(CsvValue::Int8(temp)))
            }
            CsvValue::Int64(_) => {
                let temp = v.parse::<i64>().map_err(|_| {
                    PattiCsvError::Conversion(ConversionError::ValueFromStringFailed {
                        src_value: v.clone(),
                        target_type: "Int64",
                    })
                })?;
                Ok(Some(CsvValue::Int64(temp)))
            }
            CsvValue::Int128(_) => {
                let temp = v.parse::<i128>().map_err(|_| {
                    PattiCsvError::Conversion(ConversionError::ValueFromStringFailed {
                        src_value: v.clone(),
                        target_type: "Int128",
                    })
                })?;
                Ok(Some(CsvValue::Int128(temp)))
            }
            CsvValue::Float64(_) => {
                let temp = v.parse::<f64>().map_err(|_| {
                    PattiCsvError::Conversion(ConversionError::ValueFromStringFailed {
                        src_value: v.clone(),
                        target_type: "Float64",
                    })
                })?;
                Ok(Some(CsvValue::Float64(temp)))
            }
            CsvValue::Bool(_) => {
                let temp = v.parse::<bool>().map_err(|_| {
                    PattiCsvError::Conversion(ConversionError::ValueFromStringFailed {
                        src_value: v.clone(),
                        target_type: "Bool",
                    })
                })?;
                Ok(Some(CsvValue::Bool(temp)))
            }
            CsvValue::Decimal(_) => {
                let temp = v.parse::<Decimal>().map_err(|_| {
                    PattiCsvError::Conversion(ConversionError::ValueFromStringFailed {
                        src_value: v.clone(),
                        target_type: "Decimal",
                    })
                })?;
                Ok(Some(CsvValue::Decimal(temp)))
            }
            //TODO
            _ => Ok(Some(CsvValue::String("".to_string()))),
        }
    }
}

pub trait SplitValue {
    fn split(&self, src: &Option<CsvValue>) -> Result<(Option<CsvValue>, Option<CsvValue>)>;
    fn split_none(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::FromStr;

    #[test]
    pub fn from_str_int8() {
        assert_eq!(
            CsvValue::Int8(1),
            CsvValue::from_int8_string("1".to_string()).unwrap()
        );
    }

    #[test]
    pub fn string_default() {
        assert_eq!(CsvValue::String("".to_string()), CsvValue::string_default());
    }

    #[test]
    pub fn string_to_csv_string() {
        assert_eq!(
            CsvValue::String("test".to_string()),
            "test".to_string().into()
        );
    }

    #[test]
    pub fn i8_to_csv_tinyint() {
        assert_eq!(CsvValue::Int8(32), 32i8.into());
    }

    #[test]
    pub fn i64_to_csv_int() {
        assert_eq!(CsvValue::Int64(32000), 32000i64.into());
    }

    #[test]
    pub fn i128_to_csv_bigint() {
        assert_eq!(CsvValue::Int128(32000000000), 32000000000i128.into());
    }

    #[test]
    pub fn float_to_csv_float() {
        assert_eq!(CsvValue::Float64(3200.123), 3200.123.into());
    }

    #[test]
    pub fn bool_to_csv_bool() {
        assert_eq!(CsvValue::Bool(true), true.into());
    }

    #[test]
    pub fn decimal_to_csv_decimal() {
        assert_eq!(
            CsvValue::Decimal(Decimal::from_str("1.123").unwrap()),
            Decimal::from_str("1.123").unwrap().into()
        );
    }

    #[test]
    pub fn csv_string_to_string_ok() {
        assert_eq!(
            Ok("test_data".to_string()),
            CsvValue::String("test_data".into()).into()
        );
    }

    #[test]
    pub fn csv_string_to_bool_err() {
        let res: Result<bool> = CsvValue::String("test_data".into()).into();
        let exp = Err(PattiCsvError::Conversion(
            ConversionError::UnwrapToBaseTypeFailed {
                src_value: "String(\"test_data\")".into(),
                basic_type: "bool",
            },
        ));
        assert_eq!(exp, res);
    }

    #[test]
    pub fn csv_tinyint_to_i8_ok() {
        assert_eq!(Ok(32i8), CsvValue::Int8(32).into());
    }

    #[test]
    pub fn csv_tinyint_to_bool_err() {
        let res: Result<bool> = CsvValue::Int8(32).into();
        let exp = Err(PattiCsvError::Conversion(
            ConversionError::UnwrapToBaseTypeFailed {
                src_value: "Int8(32)".into(),
                basic_type: "bool",
            },
        ));
        assert_eq!(exp, res);
    }

    #[test]
    pub fn csv_int_to_i64_ok() {
        assert_eq!(Ok(32i64), CsvValue::Int64(32).into());
    }

    #[test]
    pub fn csv_int_to_bool_err() {
        let res: Result<bool> = CsvValue::Int64(32000).into();
        let exp = Err(PattiCsvError::Conversion(
            ConversionError::UnwrapToBaseTypeFailed {
                src_value: "Int64(32000)".into(),
                basic_type: "bool",
            },
        ));
        assert_eq!(exp, res);
    }

    #[test]
    pub fn csv_bigint_to_i128_ok() {
        assert_eq!(Ok(3200000000i128), CsvValue::Int128(3200000000).into());
    }

    #[test]
    pub fn csv_bigint_to_bool_err() {
        let res: Result<bool> = CsvValue::Int128(3200000000).into();
        let exp = Err(PattiCsvError::Conversion(
            ConversionError::UnwrapToBaseTypeFailed {
                src_value: "Int128(3200000000)".into(),
                basic_type: "bool",
            },
        ));
        assert_eq!(exp, res);
    }

    #[test]
    pub fn csv_decimal_to_decimal_ok() {
        assert_eq!(
            Ok(Decimal::from(10i8)),
            CsvValue::Decimal(Decimal::from(10i8)).into()
        );
    }

    #[test]
    pub fn csv_decimal_to_bool_err() {
        let res: Result<bool> = CsvValue::Decimal(Decimal::from(32i8)).into();
        let exp = Err(PattiCsvError::Conversion(
            ConversionError::UnwrapToBaseTypeFailed {
                src_value: "Decimal(32)".into(),
                basic_type: "bool",
            },
        ));
        assert_eq!(exp, res);
    }

    #[test]
    pub fn csv_tinyint_from_string_and_templ_ok() {
        let test = CsvValue::from_string_with_templ("10".to_string(), &CsvValue::int8_default());
        assert_eq!(Ok(Some(CsvValue::Int8(10))), test);
    }

    #[test]
    pub fn csv_tinyint_from_string_and_templ_err() {
        let test = CsvValue::from_string_with_templ("false".to_string(), &CsvValue::int8_default());
        assert_eq!(
            Err(PattiCsvError::Conversion(
                ConversionError::ValueFromStringFailed {
                    src_value: "false".to_string(),
                    target_type: "Int8"
                }
            )),
            test
        );
    }

    #[test]
    pub fn csv_int_from_string_and_templ_ok() {
        let test = CsvValue::from_string_with_templ("10".to_string(), &CsvValue::int64_default());
        assert_eq!(Ok(Some(CsvValue::Int64(10))), test);
    }

    #[test]
    pub fn csv_int_from_string_and_templ_err() {
        let test =
            CsvValue::from_string_with_templ("false".to_string(), &CsvValue::int64_default());
        assert_eq!(
            Err(PattiCsvError::Conversion(
                ConversionError::ValueFromStringFailed {
                    src_value: "false".to_string(),
                    target_type: "Int64"
                }
            )),
            test
        );
    }

    #[test]
    pub fn csv_bigint_from_string_and_templ_ok() {
        let test = CsvValue::from_string_with_templ("10".to_string(), &CsvValue::int128_default());
        assert_eq!(Ok(Some(CsvValue::Int128(10))), test);
    }

    #[test]
    pub fn csv_bigint_from_string_and_templ_err() {
        let test =
            CsvValue::from_string_with_templ("false".to_string(), &CsvValue::int128_default());
        assert_eq!(
            Err(PattiCsvError::Conversion(
                ConversionError::ValueFromStringFailed {
                    src_value: "false".to_string(),
                    target_type: "Int128"
                }
            )),
            test
        );
    }

    #[test]
    pub fn csv_string_from_string_and_templ_ok() {
        let test = CsvValue::from_string_with_templ("10".to_string(), &CsvValue::string_default());
        assert_eq!(Ok(Some(CsvValue::String("10".to_string()))), test);
    }

    #[test]
    pub fn csv_float_from_string_and_templ_ok() {
        let test =
            CsvValue::from_string_with_templ("12.34".to_string(), &CsvValue::float64_default());
        assert_eq!(Ok(Some(CsvValue::Float64(12.34))), test);
    }

    #[test]
    pub fn csv_float_from_string_and_templ_err() {
        let test =
            CsvValue::from_string_with_templ("false".to_string(), &CsvValue::float64_default());
        assert_eq!(
            Err(PattiCsvError::Conversion(
                ConversionError::ValueFromStringFailed {
                    src_value: "false".to_string(),
                    target_type: "Float64"
                }
            )),
            test
        );
    }

    #[test]
    pub fn csv_bool_from_string_and_templ_ok() {
        let test = CsvValue::from_string_with_templ("true".to_string(), &CsvValue::bool_default());
        assert_eq!(Ok(Some(CsvValue::Bool(true))), test);
    }

    #[test]
    pub fn csv_bool_from_string_and_templ_err() {
        let test = CsvValue::from_string_with_templ("meh".to_string(), &CsvValue::bool_default());
        assert_eq!(
            Err(PattiCsvError::Conversion(
                ConversionError::ValueFromStringFailed {
                    src_value: "meh".to_string(),
                    target_type: "Bool"
                }
            )),
            test
        );
    }

    #[test]
    pub fn csv_decimal_from_string_and_templ_ok() {
        let test =
            CsvValue::from_string_with_templ("10.12456".to_string(), &CsvValue::decimal_default());
        assert_eq!(
            Ok(Some(CsvValue::Decimal(
                Decimal::from_str("10.12456").unwrap()
            ))),
            test
        );
    }
}
