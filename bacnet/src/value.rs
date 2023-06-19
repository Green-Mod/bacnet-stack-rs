use anyhow::anyhow;
use std::convert::TryInto;

#[derive(Debug)]
pub enum BACnetValue {
    Null, // Yes!
    Bool(bool),
    Uint(u64),
    Int(i32),
    Real(f32),
    Double(f64),
    String(String), // BACNET_CHARACTER_STRING
    Bytes(Vec<u8>), // BACNET_OCTET_STRING
    BitString(Vec<bool>),
    Enum(u32, Option<String>), // Enumerated values also have string representations...
    // A reference to an object, used during interrogation of the device (object-list)
    ObjectId {
        object_type: u32,
        object_instance: u32,
    },
    Array(Vec<BACnetValue>),
}

impl TryInto<String> for BACnetValue {
    type Error = anyhow::Error;
    fn try_into(self) -> Result<String, Self::Error> {
        Ok(match self {
            BACnetValue::String(s) => s,
            BACnetValue::Enum(_, Some(s)) => s,
            BACnetValue::Enum(i, None) => format!("{}", i),
            _ => return Err(anyhow!("Cannot turn '{:?}' into a string", self)),
        })
    }
}

impl TryInto<u64> for BACnetValue {
    type Error = anyhow::Error;
    fn try_into(self) -> Result<u64, Self::Error> {
        Ok(match self {
            BACnetValue::Uint(u) => u,
            _ => return Err(anyhow!("Cannot turn '{:?}' into a u64", self)),
        })
    }
}
