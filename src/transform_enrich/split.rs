use venum::venum::Value;

use crate::errors::Result;

pub trait SplitValue {
    fn split(&self, src: &Option<Value>) -> Result<(Option<Value>, Option<Value>)>;
    fn split_none(&self) -> bool;
}
