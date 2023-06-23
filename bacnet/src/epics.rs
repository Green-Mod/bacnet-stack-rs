use crate::value::BACnetValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Epics {
    pub device: HashMap<String, BACnetValue>,
    pub object_list: Vec<HashMap<String, BACnetValue>>,
}
