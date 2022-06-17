use crate::errors::{PattiCsvError, Result, SplitError};

use super::csv_value::{CsvValue, SplitCsvValue};

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CsvColumn {
    pub type_info: CsvValue, // We use the enum variants default value as our type info
    pub name: String,        // the column header
    pub idx: usize,          // columns are zero-indexed for now!
    pub data: Vec<Option<CsvValue>>,
}

impl CsvColumn {
    pub fn new(t_info: CsvValue, name: String, idx: usize) -> Self {
        CsvColumn {
            type_info: t_info,
            name,
            idx,
            data: Vec::new(),
        }
    }

    pub fn new_filled_with(
        value: Option<CsvValue>,
        t_info: CsvValue,
        name: String,
        idx: usize,
        capacity: usize,
    ) -> Self {
        let mut data: Vec<Option<CsvValue>> = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            data.push(value.clone());
        }

        CsvColumn {
            type_info: t_info,
            name,
            idx,
            data,
        }
    }

    pub fn new_filled_with_value(
        value: CsvValue,
        name: String,
        idx: usize,
        capacity: usize,
    ) -> Self {
        let t_info = value.clone();
        CsvColumn::new_filled_with(Some(value), t_info, name, idx, capacity)
    }

    /// Appends data to the column.
    pub fn push(&mut self, v: Option<CsvValue>) {
        self.data.push(v);
    }

    pub fn set_idx(&mut self, new_idx: usize) {
        self.idx = new_idx;
    }

    pub fn split_by<S>(
        &self,
        splitter: &S,
        dst_left: &mut CsvColumn,
        dst_right: &mut CsvColumn,
    ) -> Result<()>
    where
        S: SplitCsvValue,
    {
        fn push_or_err(imf_val_opt: Option<CsvValue>, dst: &mut CsvColumn) -> Result<()> {
            match imf_val_opt {
                None => {
                    dst.data.push(None);
                    return Ok(());
                }
                Some(ref imf_val) => {
                    match imf_val {
                        // we have a String variant as src type try converting it to the target type
                        CsvValue::String(s) => {
                            let transf_val =
                                CsvValue::from_string_with_templ(s.clone(), &dst.type_info)?;
                            dst.data.push(transf_val);
                            Ok(())
                        }
                        // we have the same enum variant in src and dst, we can push, as is
                        _ if std::mem::discriminant(imf_val)
                            == std::mem::discriminant(&dst.type_info) =>
                        {
                            dst.data.push(imf_val_opt.clone());
                            Ok(())
                        }
                        // We can do better, but we don't support arbitrary convertions for now...
                        _ => Err(PattiCsvError::Split(SplitError::from(
                            format!(
                                "type mismatch. {:?} cannot be put into left column of type {:?}",
                                imf_val, &dst.type_info
                            ),
                            imf_val_opt.clone(),
                            None,
                        ))),
                    }
                }
            }
        }
        for val in &self.data {
            let (left, right) = splitter.split(val)?;
            push_or_err(left, dst_left)?;
            push_or_err(right, dst_right)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_filled_with_value() {
        let col = CsvColumn::new_filled_with_value(
            CsvValue::Float64(1.12),
            String::from("col#1"),
            0,
            100,
        );
        assert!(col.data.len() == 100);
        assert!(col.data.iter().all(|x| x == &Some(CsvValue::Float64(1.12))));
    }

    #[test]
    fn test_new_filled_with_and_value() {
        let col = CsvColumn::new_filled_with(
            Some(CsvValue::Float64(1.12)),
            CsvValue::float64_default(),
            String::from("col#1"),
            0,
            100,
        );
        assert!(col.data.len() == 100);
        assert!(col.data.iter().all(|x| x == &Some(CsvValue::Float64(1.12))));
    }

    #[test]
    fn test_new_filled_with_and_none() {
        let col = CsvColumn::new_filled_with(
            None,
            CsvValue::float64_default(),
            String::from("col#1"),
            0,
            100,
        );
        assert!(col.data.len() == 100);
        assert!(col.data.iter().all(|x| x == &None));
    }
}
