use serde_hjson::{Map,Value};

pub fn from_slice(slice: &[u8]) -> serde_hjson::Result<Map<String, Value>> {
    serde_hjson::from_slice(slice)
}