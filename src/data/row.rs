use venum::venum::Value;

use super::cell::ValueCell;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct ValueCellRow {
    pub data: Vec<ValueCell>,
}

impl ValueCellRow {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct ValueRow {
    data: Vec<Option<Value>>,
}

impl From<ValueCellRow> for ValueRow {
    fn from(mut vcr: ValueCellRow) -> Self {
        Self {
            data: vcr
                .data
                .iter_mut()
                .map(|v| std::mem::take(&mut v.data))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use venum::venum::Value;

    use crate::data::{cell::ValueCell, row::ValueCellRow};

    use super::ValueRow;

    #[test]
    pub fn test_try_from_w_data() {
        let mut c = ValueCellRow::new();

        let vc1 = ValueCell::new(
            Value::string_default(),
            String::from("foo"),
            0,
            Some(Value::String(String::from("meh"))),
        );

        let vc2 = ValueCell::new(
            Value::string_default(),
            String::from("bar"),
            1,
            Some(Value::String(String::from("meh2"))),
        );

        c.data.push(vc1);
        c.data.push(vc2);

        let r: ValueRow = c.into();
        println!("{:?}", r);
    }

    #[test]
    pub fn test_try_from_wo_data() {
        let mut c = ValueCellRow::new();
        let vc1 = ValueCell::new_empty(Value::string_default(), String::from("foo"), 0);
        let vc2 = ValueCell::new_empty(Value::string_default(), String::from("bar"), 1);
        c.data.push(vc1);
        c.data.push(vc2);

        let r: ValueRow = c.into();
        println!("{:?}", r);
    }

    #[test]
    pub fn test_try_from_w_mixed_data_1() {
        // to be clear. The constructed case here can't (well, shouldn't) happen in our use case, since we
        // always parse a complete line. The real world case is more like in: test_try_from_w_mixed_data_2
        let mut c = ValueCellRow::new();
        let vc1 = ValueCell::new_empty(Value::string_default(), String::from("foo"), 0);
        let vc2 = ValueCell::new_empty(Value::string_default(), String::from("bar"), 1);
        let vc3 = ValueCell::new(
            Value::string_default(),
            String::from("col3"),
            3,
            Some(Value::String(String::from("baz"))),
        );
        c.data.push(vc1);
        c.data.push(vc2);
        c.data.push(vc3);

        let r: ValueRow = c.into();
        println!("{:?}", r);
    }

    #[test]
    pub fn test_try_from_w_mixed_data_2() {
        let mut c = ValueCellRow::new();
        let vc1 = ValueCell::new(Value::string_default(), String::from("foo"), 0, None);
        let vc2 = ValueCell::new(Value::string_default(), String::from("bar"), 1, None);
        let vc3 = ValueCell::new(
            Value::string_default(),
            String::from("col3"),
            3,
            Some(Value::String(String::from("baz"))),
        );
        c.data.push(vc1);
        c.data.push(vc2);
        c.data.push(vc3);

        let r: ValueRow = c.into();
        println!("{:?}", r);
    }
}
