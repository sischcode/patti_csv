use crate::errors::{ConversionError, PattiCsvError, Result};
use chrono::NaiveDate;
use rust_decimal::{prelude::FromPrimitive, Decimal};
use std::convert::From;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Value {
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
        impl From<$type> for Value {
            fn from(item: $type) -> Self {
                Value::$enum_type(item)
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
        impl From<Value> for Result<$type> {
            fn from(item: Value) -> Self {
                match item {
                    Value::$enum_type(v) => Ok(v),
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
        pub fn $fn_name<T>(v: T) -> Result<Value>
        where
            T: Into<String>,
        {
            let v = v.into();
            let temp = v.parse::<$type>().map_err(|_| {
                PattiCsvError::Conversion(ConversionError::ValueFromStringFailed {
                    src_value: v,
                    target_type: stringify!($enum_type),
                })
            })?;
            Ok(Value::$enum_type(temp))
        }
    };
}

macro_rules! type_defaults {
    ($fn_name:ident, $enum_type:ident, $type:ty) => {
        pub fn $fn_name() -> Value {
            Value::$enum_type(<$type>::default())
        }
    };
}

macro_rules! is_type {
    ($fn_name:ident, $enum_type:ident) => {
        pub fn $fn_name(&self) -> bool {
            match self {
                Value::$enum_type(_) => true,
                _ => false,
            }
        }
    };
}

impl Value {
    pub fn date_default() -> Self {
        Value::Date(String::from(""))
    }
    pub fn naive_date_default() -> Self {
        Value::NaiveDate(NaiveDate::from_ymd(0, 0, 0))
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

    from_type_string!(int8_from_string, Int8, i8);
    from_type_string!(int32_from_string, Int32, i32);
    from_type_string!(int64_from_string, Int64, i64);
    from_type_string!(int128_from_string, Int128, i128);
    from_type_string!(float32_from_string, Float32, f32);
    from_type_string!(float64_from_string, Float64, f64);
    from_type_string!(bool_from_string, Bool, bool);

    pub fn decimal_from_string<T>(v: T) -> Result<Value>
    where
        T: Into<String>,
    {
        let v = v.into();
        let temp = Decimal::from_str_exact(&v).map_err(|_| {
            PattiCsvError::Conversion(ConversionError::ValueFromStringFailed {
                src_value: v,
                target_type: "Decimal",
            })
        })?;
        Ok(Value::Decimal(temp))
    }

    pub fn decimal_from_i8(v: i8) -> Value {
        Value::Decimal(Decimal::from_i16(v as i16).unwrap()) // I can't think of a case where an i8 cannot be represented by a decimal
    }

    pub fn decimal_from_i32(v: i32) -> Value {
        Value::Decimal(Decimal::from_i32(v).unwrap()) // I can't think of a case where an i32 cannot be represented by a decimal
    }

    pub fn decimal_from_i64(v: i64) -> Value {
        Value::Decimal(Decimal::from_i64(v).unwrap()) // I can't think of a case where an i64 cannot be represented by a decimal
    }

    pub fn decimal_from_i128(v: i128) -> Value {
        Value::Decimal(Decimal::from_i128(v).unwrap()) // I can't think of a case where an i128 cannot be represented by a decimal
    }

    pub fn decimal_from_f32(v: f32) -> Value {
        Value::Decimal(Decimal::from_f32(v).unwrap()) // I can't think of a case where a f32 cannot be represented by a decimal
    }

    pub fn decimal_from_f64(v: i64) -> Value {
        Value::Decimal(Decimal::from_i64(v).unwrap()) // I can't think of a case where a f64 cannot be represented by a decimal
    }

    is_type!(is_string, String);
    is_type!(is_int8, Int8);
    is_type!(is_int32, Int32);
    is_type!(is_int64, Int64);
    is_type!(is_int128, Int128);
    is_type!(is_float32, Float32);
    is_type!(is_float64, Float64);
    is_type!(is_bool, Bool);
    is_type!(is_decimal, Decimal);
    is_type!(is_date, Date);
    is_type!(is_naive_date, NaiveDate);

    pub fn get_default_of_self(&self) -> Value {
        match self {
            Value::String(_) => Value::string_default(),
            Value::Int8(_) => Value::int8_default(),
            Value::Int32(_) => Value::int32_default(),
            Value::Int64(_) => Value::int64_default(),
            Value::Int128(_) => Value::int128_default(),
            Value::Float32(_) => Value::float32_default(),
            Value::Float64(_) => Value::float64_default(),
            Value::Bool(_) => Value::bool_default(),
            Value::Decimal(_) => Value::decimal_default(),
            Value::Date(_) => Value::date_default(),
            Value::NaiveDate(_) => Value::naive_date_default(),
        }
    }

    /// NOTE: We decided agains Option<String> here as the type of the value since the intention is to create a typed version of a stringy-input we read from some CSV.
    ///       In that case, when a CSV column contains a "" as an entry, e.g. like this: `a,,c` or this `"a","","c"`, where the middle column would translate to empty / "",
    ///       we map it to a None internally, representing the absence of data.
    pub fn from_string_with_templ(value: String, templ_type: &Value) -> Result<Option<Value>> {
        if value == "".to_string() {
            return Ok(None);
        }
        match templ_type {
            Value::String(_) => Ok(Some(Value::String(value))), // even a string value of "" will be a real value, since it's not explicitly None (...i.e. coming from a "null")
            Value::Int8(_) => {
                let temp = value.parse::<i8>().map_err(|_| {
                    PattiCsvError::Conversion(ConversionError::ValueFromStringFailed {
                        src_value: value.clone(),
                        target_type: "Int8",
                    })
                })?;
                Ok(Some(Value::Int8(temp)))
            }
            Value::Int32(_) => {
                let temp = value.parse::<i32>().map_err(|_| {
                    PattiCsvError::Conversion(ConversionError::ValueFromStringFailed {
                        src_value: value.clone(),
                        target_type: "Int32",
                    })
                })?;
                Ok(Some(Value::Int32(temp)))
            }
            Value::Int64(_) => {
                let temp = value.parse::<i64>().map_err(|_| {
                    PattiCsvError::Conversion(ConversionError::ValueFromStringFailed {
                        src_value: value.clone(),
                        target_type: "Int64",
                    })
                })?;
                Ok(Some(Value::Int64(temp)))
            }
            Value::Int128(_) => {
                let temp = value.parse::<i128>().map_err(|_| {
                    PattiCsvError::Conversion(ConversionError::ValueFromStringFailed {
                        src_value: value.clone(),
                        target_type: "Int128",
                    })
                })?;
                Ok(Some(Value::Int128(temp)))
            }
            Value::Float32(_) => {
                let temp = value.parse::<f32>().map_err(|_| {
                    PattiCsvError::Conversion(ConversionError::ValueFromStringFailed {
                        src_value: value.clone(),
                        target_type: "Float32",
                    })
                })?;
                Ok(Some(Value::Float32(temp)))
            }
            Value::Float64(_) => {
                let temp = value.parse::<f64>().map_err(|_| {
                    PattiCsvError::Conversion(ConversionError::ValueFromStringFailed {
                        src_value: value.clone(),
                        target_type: "Float64",
                    })
                })?;
                Ok(Some(Value::Float64(temp)))
            }
            Value::Bool(_) => {
                let temp = value.parse::<bool>().map_err(|_| {
                    PattiCsvError::Conversion(ConversionError::ValueFromStringFailed {
                        src_value: value.clone(),
                        target_type: "Bool",
                    })
                })?;
                Ok(Some(Value::Bool(temp)))
            }
            Value::Decimal(_) => {
                let temp = value.parse::<Decimal>().map_err(|_| {
                    PattiCsvError::Conversion(ConversionError::ValueFromStringFailed {
                        src_value: value.clone(),
                        target_type: "Decimal",
                    })
                })?;
                Ok(Some(Value::Decimal(temp)))
            }
            //TODO
            _ => Ok(Some(Value::String("".to_string()))),
        }
    }
}

// TODO: not sure if we should rename this, or make this a method on value, etc.
pub trait SplitValue {
    fn split(&self, src: &Option<Value>) -> Result<(Option<Value>, Option<Value>)>;
    fn split_none(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_int8_from_string() {
        assert_eq!(Ok(Value::Int8(0)), Value::int8_from_string("0".to_string()));
    }

    #[test]
    pub fn test_int32_from_string() {
        assert_eq!(
            Ok(Value::Int32(0)),
            Value::int32_from_string("0".to_string())
        );
    }

    #[test]
    pub fn test_int64_from_string() {
        assert_eq!(
            Ok(Value::Int64(0)),
            Value::int64_from_string("0".to_string())
        );
    }

    #[test]
    pub fn test_int128_from_string() {
        assert_eq!(
            Ok(Value::Int128(0)),
            Value::int128_from_string("0".to_string())
        );
    }

    #[test]
    pub fn test_float32_from_string() {
        assert_eq!(
            Ok(Value::Float32(0.0)),
            Value::float32_from_string("0".to_string())
        );
    }

    #[test]
    pub fn test_float64_from_string() {
        assert_eq!(
            Ok(Value::Float64(0.0)),
            Value::float64_from_string("0".to_string())
        );
    }

    #[test]
    pub fn test_bool_from_string() {
        assert_eq!(
            Ok(Value::Bool(true)),
            Value::bool_from_string("true".to_string())
        );
    }

    #[test]
    pub fn test_decimal_from_string() {
        assert_eq!(
            Ok(Value::Decimal(Decimal::new(1123, 3))),
            Value::decimal_from_string("1.123".to_string())
        );
    }

    #[test]
    pub fn test_string_default() {
        assert_eq!(Value::String("".to_string()), Value::string_default());
    }

    #[test]
    pub fn test_int8_default() {
        assert_eq!(Value::Int8(0), Value::int8_default());
    }

    #[test]
    pub fn test_int32_default() {
        assert_eq!(Value::Int32(0), Value::int32_default());
    }

    #[test]
    pub fn test_int64_default() {
        assert_eq!(Value::Int64(0), Value::int64_default());
    }

    #[test]
    pub fn test_int128_default() {
        assert_eq!(Value::Int128(0), Value::int128_default());
    }

    #[test]
    pub fn test_float32_default() {
        assert_eq!(Value::Float32(0.0), Value::float32_default());
    }

    #[test]
    pub fn test_float64_default() {
        assert_eq!(Value::Float64(0.0), Value::float64_default());
    }

    #[test]
    pub fn test_bool_default() {
        assert_eq!(Value::Bool(false), Value::bool_default());
    }

    #[test]
    pub fn test_decimal_default() {
        assert_eq!(
            Value::Decimal(Decimal::new(00, 1)),
            Value::decimal_default()
        );
    }

    #[test]
    pub fn csv_string_to_bool_err() {
        let res: Result<bool> = Value::String("test_data".into()).into();
        let exp = Err(PattiCsvError::Conversion(
            ConversionError::UnwrapToBaseTypeFailed {
                src_value: "String(\"test_data\")".into(),
                basic_type: "bool",
            },
        ));
        assert_eq!(exp, res);
    }

    #[test]
    pub fn int8_from_string_and_templ_ok() {
        let test = Value::from_string_with_templ("10".to_string(), &Value::int8_default());
        assert_eq!(Ok(Some(Value::Int8(10))), test);
    }

    #[test]
    pub fn int8_from_string_and_templ_err() {
        let test = Value::from_string_with_templ("false".to_string(), &Value::int8_default());
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
}
