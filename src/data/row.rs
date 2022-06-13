use super::{value::Value, column::Column, data::CsvData};

pub type Row = Vec<Option<Value>>;

impl From<CsvData> for Row {
    fn from(mut csv: CsvData) -> Self {
        csv.columns.iter_mut().map(|c| -> Option<Value> {
            let foo = c.data.as_mut();
            let mut bar = foo.first().unwrap();
            std::mem::take(&mut bar)
        }).collect()
    }
}