// use venum::venum::Value;

// use crate::errors::{PattiCsvError, Result, SplitError};

// // TODO: not sure if we should rename this, or make this a method on value, etc.
// pub trait SplitValue {
//     fn split(&self, src: &Option<Value>) -> Result<(Option<Value>, Option<Value>)>;
//     fn split_none(&self) -> bool;
// }

// #[derive(Debug, Clone, PartialEq, PartialOrd)]
// pub struct DataColumn {
//     pub type_info: Value, // We use the enum variants default value as our type info
//     pub name: String,     // the column header
//     pub idx: usize,       // columns are zero-indexed for now!
//     pub data: Vec<Option<Value>>,
// }

// impl DataColumn {
//     pub fn new(t_info: Value, name: String, idx: usize) -> Self {
//         DataColumn {
//             type_info: t_info,
//             name,
//             idx,
//             data: Vec::new(),
//         }
//     }

//     pub fn new_filled_with(
//         value: Option<Value>,
//         t_info: Value,
//         name: String,
//         idx: usize,
//         capacity: usize,
//     ) -> Self {
//         let mut data: Vec<Option<Value>> = Vec::with_capacity(capacity);
//         for _ in 0..capacity {
//             data.push(value.clone());
//         }

//         DataColumn {
//             type_info: t_info,
//             name,
//             idx,
//             data,
//         }
//     }

//     pub fn new_filled_with_value(value: Value, name: String, idx: usize, capacity: usize) -> Self {
//         let t_info = value.clone();
//         DataColumn::new_filled_with(Some(value), t_info, name, idx, capacity)
//     }

//     /// Appends data to the column.
//     pub fn push(&mut self, v: Option<Value>) {
//         self.data.push(v);
//     }

//     pub fn set_idx(&mut self, new_idx: usize) {
//         self.idx = new_idx;
//     }

//     pub fn split_by<S>(
//         &self,
//         splitter: &S,
//         dst_left: &mut DataColumn,
//         dst_right: &mut DataColumn,
//     ) -> Result<()>
//     where
//         S: SplitValue,
//     {
//         fn push_or_err(imf_val_opt: Option<Value>, dst: &mut DataColumn) -> Result<()> {
//             match imf_val_opt {
//                 None => {
//                     dst.data.push(None);
//                     return Ok(());
//                 }
//                 Some(ref imf_val) => {
//                     match imf_val {
//                         // we have a String variant as src type try converting it to the target type
//                         Value::String(s) => {
//                             let transf_val = Value::from_string_with_templ(s, &dst.type_info)?;
//                             dst.data.push(transf_val);
//                             Ok(())
//                         }
//                         // we have the same enum variant in src and dst, we can push, as is
//                         _ if std::mem::discriminant(imf_val)
//                             == std::mem::discriminant(&dst.type_info) =>
//                         {
//                             dst.data.push(imf_val_opt.clone());
//                             Ok(())
//                         }
//                         // We can do better, but we don't support arbitrary convertions for now...
//                         _ => Err(PattiCsvError::Split(SplitError::from(
//                             format!(
//                                 "type mismatch. {:?} cannot be put into left column of type {:?}",
//                                 imf_val, &dst.type_info
//                             ),
//                             imf_val_opt.clone(),
//                             None,
//                         ))),
//                     }
//                 }
//             }
//         }
//         for val in &self.data {
//             let (left, right) = splitter.split(val)?;
//             push_or_err(left, dst_left)?;
//             push_or_err(right, dst_right)?;
//         }
//         Ok(())
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_new_filled_with_value() {
//         let col =
//             DataColumn::new_filled_with_value(Value::Float64(1.12), String::from("col#1"), 0, 100);
//         assert!(col.data.len() == 100);
//         assert!(col.data.iter().all(|x| x == &Some(Value::Float64(1.12))));
//     }

//     #[test]
//     fn test_new_filled_with_and_value() {
//         let col = DataColumn::new_filled_with(
//             Some(Value::Float64(1.12)),
//             Value::float64_default(),
//             String::from("col#1"),
//             0,
//             100,
//         );
//         assert!(col.data.len() == 100);
//         assert!(col.data.iter().all(|x| x == &Some(Value::Float64(1.12))));
//     }

//     #[test]
//     fn test_new_filled_with_and_none() {
//         let col = DataColumn::new_filled_with(
//             None,
//             Value::float64_default(),
//             String::from("col#1"),
//             0,
//             100,
//         );
//         assert!(col.data.len() == 100);
//         assert!(col.data.iter().all(|x| x == &None));
//     }
// }
