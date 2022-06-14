use crate::errors::PattiCsvError;

use super::{data::CsvData, value::Value};

pub type Row = Vec<Option<Value>>;

impl TryFrom<CsvData> for Row {
    type Error = PattiCsvError;

    fn try_from(csv: CsvData) -> Result<Self, Self::Error> {
        csv.columns
            .iter_mut()
            .map(|c| -> Option<Value> { std::mem::take(c.data.first_mut()) })
            .collect()
    }
}
