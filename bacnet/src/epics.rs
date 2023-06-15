use crate::value::BACnetValue;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Epics {
    pub device: HashMap<String, BACnetValue>,
    pub object_list: Vec<HashMap<String, BACnetValue>>,
}
