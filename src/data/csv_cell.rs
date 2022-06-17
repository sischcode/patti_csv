use super::csv_value::CsvValue;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CsvCell {
    pub type_info: CsvValue, // We use the enum variants default value as our type info
    pub header: String,      // the column header
    pub idx: usize,          // columns are zero-indexed for now!
    pub data: Option<CsvValue>, // Data
}

impl CsvCell {
    pub fn new_empty(type_info: CsvValue, header: String, idx: usize) -> Self {
        Self {
            type_info,
            header,
            idx,
            data: None,
        }
    }

    pub fn new(type_info: CsvValue, header: String, idx: usize, data: Option<CsvValue>) -> Self {
        Self {
            type_info,
            header,
            idx,
            data,
        }
    }
}
