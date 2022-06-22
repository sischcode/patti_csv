// use super::column::DataColumn;

// use crate::data::column::SplitValue;
// use crate::errors::{PattiCsvError, Result, SplitError};

// #[derive(Debug, Clone, PartialEq)]
// pub struct DataColumnOriented(pub Vec<DataColumn>);

// impl DataColumnOriented {
//     pub fn new() -> Self {
//         DataColumnOriented { 0: Vec::new() }
//     }

//     pub fn num_rows(&self) -> usize {
//         return if self.0.len() == 0 {
//             0
//         } else {
//             self.0.first().unwrap().data.len()
//         };
//     }

//     pub fn get_col(&self, idx: usize) -> Option<&DataColumn> {
//         self.0.iter().find(|&c| c.idx == idx)
//     }

//     pub fn get_col_mut(&mut self, idx: usize) -> Option<&mut DataColumn> {
//         self.0.iter_mut().find(|c| c.idx == idx)
//     }

//     pub fn add_col(&mut self, col: DataColumn) {
//         self.0.push(col);
//     }

//     pub fn del_col(&mut self, idx: usize) -> Result<DataColumn> {
//         let mut del = 0;
//         let mut found = false;
//         for col in self.0.iter().enumerate() {
//             if col.1.idx == idx {
//                 del = col.0;
//                 found = true;
//                 break;
//             }
//         }
//         if found {
//             return Ok(self.0.remove(del));
//         }
//         Err(PattiCsvError::Generic {
//             msg: format!("could not delete column with idx {}. (not found)", idx),
//         })
//     }

//     pub fn split_column_add<S>(
//         &mut self,
//         idx: usize,
//         splitter: &S,
//         mut dst_left: DataColumn,
//         mut dst_right: DataColumn,
//         delete_src_col: bool,
//     ) -> Result<()>
//     where
//         S: SplitValue,
//     {
//         let src_col = self
//             .get_col_mut(idx)
//             .ok_or(PattiCsvError::Split(SplitError::minim(format!(
//                 "Column with idx {} does not exist. Can't split.",
//                 idx,
//             ))))?;
//         src_col.split_by(splitter, &mut dst_left, &mut dst_right)?;
//         self.add_col(dst_left);
//         self.add_col(dst_right);

//         if delete_src_col {
//             self.del_col(idx).unwrap(); // we checked above
//         }
//         Ok(())
//     }
// }

// #[cfg(test)]
// mod tests {
//     use venum::venum::Value;

//     use super::*;
//     use crate::data::column::DataColumn;

//     #[test]
//     fn imf_add_col() {
//         let mut imf = DataColumnOriented::new();
//         imf.add_col(DataColumn::new(
//             Value::string_default(),
//             "column_1".to_string(),
//             0,
//         ));
//         assert_eq!(1, imf.0.len());
//     }

//     #[test]
//     fn imf_get_col() {
//         let mut imf = DataColumnOriented::new();
//         imf.add_col(DataColumn::new(
//             Value::string_default(),
//             "column_1".to_string(),
//             0,
//         ));
//         let col = imf.get_col(0);
//         assert!(col.is_some());
//     }

//     #[test]
//     fn imf_get_col_mut() {
//         let mut imf = DataColumnOriented::new();
//         imf.add_col(DataColumn::new(
//             Value::string_default(),
//             "column_1".to_string(),
//             0,
//         ));
//         let col = imf.get_col_mut(0);
//         assert!(col.is_some());
//     }

//     #[test]
//     fn imf_del_col_ok() {
//         let mut imf = DataColumnOriented::new();
//         imf.add_col(DataColumn::new(
//             Value::string_default(),
//             "column_1".to_string(),
//             0,
//         ));
//         let res = imf.del_col(0);
//         assert!(res.is_ok());
//     }

//     #[test]
//     fn imf_del_col_err() {
//         let mut imf = DataColumnOriented::new();
//         imf.add_col(DataColumn::new(
//             Value::string_default(),
//             "column_1".to_string(),
//             0,
//         ));
//         let res = imf.del_col(1);
//         assert!(res.is_err());
//     }

//     // #[test]
//     // fn imf_split_col_add_ok() {
//     //     let mut imf = CsvData::new();
//     //     let mut data_col = CsvColumn::new(CsvValue::string_default(), "column_1".to_string(), 0);

//     //     data_col.push_data(CsvValue::String(String::from("1.12 true")));
//     //     data_col.push_data(CsvValue::String(String::from("2.34 false")));

//     //     imf.add_col(data_col);

//     //     let dst_l = CsvColumn::new(CsvValue::float_default(), String::from("col#2"), 1);
//     //     let dst_r = CsvColumn::new(CsvValue::bool_default(), String::from("col#3"), 2);

//     //     let sep = ValueSeparatorCharSplitter {
//     //         sep_char: ' ',
//     //         split_none: true,
//     //     };

//     //     let res = imf.split_column_add(0, &sep, dst_l, dst_r, true);
//     //     assert!(res.is_ok());
//     //     assert!(imf.get_col(0).is_none());
//     //     assert!(imf.get_col(1).is_some());
//     //     assert!(imf.get_col(2).is_some());
//     //     assert!(imf.get_col(1).unwrap().data.get(0).unwrap() == &Some(CsvValue::Float(1.12)));
//     //     assert!(imf.get_col(1).unwrap().data.get(1).unwrap() == &Some(CsvValue::Float(2.34)));
//     //     assert!(imf.get_col(2).unwrap().data.get(0).unwrap() == &Some(CsvValue::Bool(true)));
//     //     assert!(imf.get_col(2).unwrap().data.get(1).unwrap() == &Some(CsvValue::Bool(false)));
//     // }
// }
