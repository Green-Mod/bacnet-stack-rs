use anyhow::anyhow;
use std::convert::TryInto;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
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
    Date {
        year: u16,
        month: u8,
        day: u8,
        weekday: u8,
    },
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

impl From<String> for BACnetValue {
    fn from(raw: String) -> Self {
        if let Ok(value) = raw.parse::<bool>() {
            return BACnetValue::Bool(value);
        }

        if let Ok(value) = raw.parse::<u64>() {
            return BACnetValue::Uint(value);
        }

        if let Ok(value) = raw.parse::<i32>() {
            return BACnetValue::Int(value);
        }

        if let Ok(value) = raw.parse::<f32>() {
            return BACnetValue::Real(value);
        }

        if let Ok(value) = raw.parse::<f64>() {
            return BACnetValue::Double(value);
        }

        BACnetValue::String(raw)
    }
}
