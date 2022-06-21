use venum::venum::Value;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct ValueCell {
    pub type_info: Value, // We use the enum variants default value as our type info
    pub header: String,   // the column header
    pub idx: usize,       // columns are zero-indexed for now!
    pub data: Option<Value>, // Data
}

impl ValueCell {
    pub fn new_empty(type_info: Value, header: String, idx: usize) -> Self {
        Self {
            type_info,
            header,
            idx,
            data: None,
        }
    }

    pub fn new(type_info: Value, header: String, idx: usize, data: Option<Value>) -> Self {
        Self {
            type_info,
            header,
            idx,
            data,
        }
    }
}