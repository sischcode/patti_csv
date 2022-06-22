use venum::venum::Value;

use crate::{
    errors::{PattiCsvError, Result, SplitError},
    transform_enrich::split::SplitValue,
};

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct DataCell {
    pub type_info: Value, // We use the enum variants default value as our type info
    pub header: String,   // the column header
    pub idx: usize,       // columns are zero-indexed for now!
    pub data: Option<Value>, // Data
}

impl DataCell {
    pub fn new_without_data(type_info: Value, header: String, idx: usize) -> Self {
        Self {
            type_info,
            header,
            idx,
            data: None,
        }
    }

    pub fn new(type_info: Value, header: String, idx: usize, data: Option<Value>) -> Self {
        Self {
            type_info,
            header,
            idx,
            data,
        }
    }

    /// We need to do it this way, because we need the type info from the destinations beforehand.
    pub fn split_by<S>(
        &self,
        splitter: &S,
        dst_left: &mut DataCell,
        dst_right: &mut DataCell,
    ) -> Result<()>
    where
        S: SplitValue,
    {
        let (split_res_left, split_res_right) = splitter.split(&self.data)?;

        fn converse_to(val: &Value, type_info: &Value) -> Result<Option<Value>> {
            match val {
                // we have the same enum variant in src and dst, we can use/clone it as is
                _ if std::mem::discriminant(val) == std::mem::discriminant(type_info) => {
                    Ok(Some(val.clone()))
                }
                // we have a String variant as src type try converting it to the target type
                Value::String(s) => {
                    let transf_val = Value::from_string_with_templ(s, type_info)?;
                    Ok(transf_val)
                }
                // We can do better, but we don't support arbitrary convertions for now...
                _ => Err(PattiCsvError::Split(SplitError::from(
                    format!("type mismatch. {val:?} cannot be parsed/converted/put into destination of type {type_info:?}"),
                    Some(val.clone()),
                    None,
                ))),
            }
        }

        match (split_res_left, split_res_right) {
            (Some(ref data_left), Some(ref data_right)) => {
                dst_left.data = converse_to(data_left, &dst_left.type_info)?;
                dst_right.data = converse_to(data_right, &dst_right.type_info)?;
            }
            (Some(ref data_left), None) => {
                dst_left.data = converse_to(data_left, &dst_left.type_info)?
            }
            (None, Some(ref data_right)) => {
                dst_right.data = converse_to(data_right, &dst_right.type_info)?
            }
            (None, None) => {}
        }
        Ok(())
    }
}
