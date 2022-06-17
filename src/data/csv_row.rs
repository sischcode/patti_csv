use super::{csv_cell::CsvCell, csv_data_columns::CsvDataColumns, csv_value::CsvValue};

pub type CsvCellRow = Vec<CsvCell>;
pub type CsvValueRow = Vec<Option<CsvValue>>;
pub type CsvStringRow = Vec<Option<String>>;

impl From<CsvDataColumns> for CsvValueRow {
    fn from(mut csv: CsvDataColumns) -> Self {
        csv.columns
            .iter_mut()
            .map(|c| -> Option<CsvValue> { std::mem::take(c.data.first_mut()?) })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::data::{
        csv_column::CsvColumn, csv_data_columns::CsvDataColumns, csv_value::CsvValue,
    };

    use super::CsvValueRow;

    #[test]
    pub fn test_try_from_w_data() {
        let mut c = CsvDataColumns::new();

        let mut col1 = CsvColumn::new(CsvValue::string_default(), String::from("foo"), 0);
        col1.push(Some(CsvValue::String(String::from("meh"))));
        let mut col2 = CsvColumn::new(CsvValue::string_default(), String::from("bar"), 1);
        col2.push(Some(CsvValue::String(String::from("meh2"))));

        c.add_col(col1);
        c.add_col(col2);

        let r: CsvValueRow = c.into();
        println!("{:?}", r);
    }

    #[test]
    pub fn test_try_from_wo_data() {
        let mut c = CsvDataColumns::new();

        let col1 = CsvColumn::new(CsvValue::string_default(), String::from("foo"), 0);
        let col2 = CsvColumn::new(CsvValue::string_default(), String::from("bar"), 1);

        c.add_col(col1);
        c.add_col(col2);

        let r: CsvValueRow = c.into();
        println!("{:?}", r);
    }

    #[test]
    pub fn test_try_from_w_mixed_data_1() {
        let mut c = CsvDataColumns::new();

        // to be clear. The constructed case here can't (well, shouldn't) happen in our use case, since we
        // always parse a complete line. The real world case is more like in: test_try_from_w_mixed_data_2
        let col1 = CsvColumn::new(CsvValue::string_default(), String::from("foo"), 0);
        let col2 = CsvColumn::new(CsvValue::string_default(), String::from("bar"), 1);
        let mut col3 = CsvColumn::new(CsvValue::string_default(), String::from("baz"), 2);
        col3.push(Some(CsvValue::String(String::from("meh"))));

        c.add_col(col1);
        c.add_col(col2);
        c.add_col(col3);

        let r: CsvValueRow = c.into();
        println!("{:?}", r);
    }

    #[test]
    pub fn test_try_from_w_mixed_data_2() {
        let mut c = CsvDataColumns::new();

        let mut col1 = CsvColumn::new(CsvValue::string_default(), String::from("foo"), 0);
        col1.push(None);
        let mut col2 = CsvColumn::new(CsvValue::string_default(), String::from("bar"), 1);
        col2.push(None);
        let mut col3 = CsvColumn::new(CsvValue::string_default(), String::from("baz"), 2);
        col3.push(Some(CsvValue::String(String::from("meh"))));

        c.add_col(col1);
        c.add_col(col2);
        c.add_col(col3);

        let r: CsvValueRow = c.into();
        println!("{:?}", r);
    }
}
