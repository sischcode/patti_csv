use super::{data::CsvData, value::Value};

pub type ValueRow = Vec<Option<Value>>;
pub type StringRow = Vec<Option<String>>;

impl From<CsvData> for ValueRow {
    fn from(mut csv: CsvData) -> Self {
        csv.columns
            .iter_mut()
            .map(|c| -> Option<Value> { std::mem::take(c.data.first_mut()?) })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::data::{column::Column, data::CsvData, value::Value};

    use super::ValueRow;

    #[test]
    pub fn test_try_from_w_data() {
        let mut c = CsvData::new();

        let mut col1 = Column::new(Value::string_default(), String::from("foo"), 0);
        col1.push(Some(Value::String(String::from("meh"))));
        let mut col2 = Column::new(Value::string_default(), String::from("bar"), 1);
        col2.push(Some(Value::String(String::from("meh2"))));

        c.add_col(col1);
        c.add_col(col2);

        let r: ValueRow = c.into();
        println!("{:?}", r);
    }

    #[test]
    pub fn test_try_from_wo_data() {
        let mut c = CsvData::new();

        let col1 = Column::new(Value::string_default(), String::from("foo"), 0);
        let col2 = Column::new(Value::string_default(), String::from("bar"), 1);

        c.add_col(col1);
        c.add_col(col2);

        let r: ValueRow = c.into();
        println!("{:?}", r);
    }

    #[test]
    pub fn test_try_from_w_mixed_data_1() {
        let mut c = CsvData::new();

        // to be clear. The constructed case here can't (well, shouldn't) happen in our use case, since we
        // always parse a complete line. The real world case is more like in: test_try_from_w_mixed_data_2
        let col1 = Column::new(Value::string_default(), String::from("foo"), 0);
        let col2 = Column::new(Value::string_default(), String::from("bar"), 1);
        let mut col3 = Column::new(Value::string_default(), String::from("baz"), 2);
        col3.push(Some(Value::String(String::from("meh"))));

        c.add_col(col1);
        c.add_col(col2);
        c.add_col(col3);

        let r: ValueRow = c.into();
        println!("{:?}", r);
    }

    #[test]
    pub fn test_try_from_w_mixed_data_2() {
        let mut c = CsvData::new();

        let mut col1 = Column::new(Value::string_default(), String::from("foo"), 0);
        col1.push(None);
        let mut col2 = Column::new(Value::string_default(), String::from("bar"), 1);
        col2.push(None);
        let mut col3 = Column::new(Value::string_default(), String::from("baz"), 2);
        col3.push(Some(Value::String(String::from("meh"))));

        c.add_col(col1);
        c.add_col(col2);
        c.add_col(col3);

        let r: ValueRow = c.into();
        println!("{:?}", r);
    }
}
