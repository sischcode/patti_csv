use venum::venum::Value;

use super::cell::DataCell;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct DataCellRow(pub Vec<DataCell>);

impl DataCellRow {
    pub fn new() -> Self {
        Self { 0: Vec::new() }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct DataRow(pub Vec<Option<Value>>);

impl From<DataCellRow> for DataRow {
    fn from(mut vcr: DataCellRow) -> Self {
        Self {
            0: vcr
                .0
                .iter_mut()
                .map(|v| std::mem::take(&mut v.data))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use venum::venum::Value;

    use crate::data::{cell::DataCell, row::DataCellRow};

    use super::DataRow;

    #[test]
    pub fn test_try_from_w_data() {
        let mut c = DataCellRow::new();

        let vc1 = DataCell::new(
            Value::string_default(),
            String::from("foo"),
            0,
            Some(Value::String(String::from("meh"))),
        );

        let vc2 = DataCell::new(
            Value::string_default(),
            String::from("bar"),
            1,
            Some(Value::String(String::from("meh2"))),
        );

        c.0.push(vc1);
        c.0.push(vc2);

        let r: DataRow = c.into();
        println!("{:?}", r);
    }

    #[test]
    pub fn test_try_from_wo_data() {
        let mut c = DataCellRow::new();
        let vc1 = DataCell::new_without_data(Value::string_default(), String::from("foo"), 0);
        let vc2 = DataCell::new_without_data(Value::string_default(), String::from("bar"), 1);
        c.0.push(vc1);
        c.0.push(vc2);

        let r: DataRow = c.into();
        println!("{:?}", r);
    }

    #[test]
    pub fn test_try_from_w_mixed_data_1() {
        // to be clear. The constructed case here can't (well, shouldn't) happen in our use case, since we
        // always parse a complete line. The real world case is more like in: test_try_from_w_mixed_data_2
        let mut c = DataCellRow::new();
        let vc1 = DataCell::new_without_data(Value::string_default(), String::from("foo"), 0);
        let vc2 = DataCell::new_without_data(Value::string_default(), String::from("bar"), 1);
        let vc3 = DataCell::new(
            Value::string_default(),
            String::from("col3"),
            3,
            Some(Value::String(String::from("baz"))),
        );
        c.0.push(vc1);
        c.0.push(vc2);
        c.0.push(vc3);

        let r: DataRow = c.into();
        println!("{:?}", r);
    }

    #[test]
    pub fn test_try_from_w_mixed_data_2() {
        let mut c = DataCellRow::new();
        let vc1 = DataCell::new(Value::string_default(), String::from("foo"), 0, None);
        let vc2 = DataCell::new(Value::string_default(), String::from("bar"), 1, None);
        let vc3 = DataCell::new(
            Value::string_default(),
            String::from("col3"),
            3,
            Some(Value::String(String::from("baz"))),
        );
        c.0.push(vc1);
        c.0.push(vc2);
        c.0.push(vc3);

        let r: DataRow = c.into();
        println!("{:?}", r);
    }
}
