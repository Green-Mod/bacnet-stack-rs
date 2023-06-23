use crate::{cstr, value::BACnetValue, BACnetErr};
use bacnet_sys::{
    bacapp_decode_application_data, bactext_application_tag_name,
    bactext_binary_present_value_name, bactext_engineering_unit_name, bactext_object_type_name,
    bitstring_bit, bitstring_bits_used, bitstring_init, bitstring_set_bit,
    BACnetObjectType_OBJECT_PROPRIETARY_MIN, BACNET_APPLICATION_DATA_VALUE, BACNET_BIT_STRING,
    BACNET_CHARACTER_STRING, BACNET_OCTET_STRING, BACNET_READ_PROPERTY_DATA, BACNET_STATUS_ERROR,
    MAX_ASHRAE_OBJECT_TYPE,
};

pub fn decode_data(data: BACNET_READ_PROPERTY_DATA) -> Result<BACnetValue, BACnetErr> {
    let mut value = BACNET_APPLICATION_DATA_VALUE::default();
    let appdata = data.application_data;
    let appdata_len = data.application_data_len;

    let len = unsafe { bacapp_decode_application_data(appdata, appdata_len as u32, &mut value) };

    if len == BACNET_STATUS_ERROR {
        return Err(BACnetErr::DecodeFailed);
    }

    Ok(match value.tag as u32 {
        bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_NULL => BACnetValue::Null,
        bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_BOOLEAN => {
            BACnetValue::Bool(unsafe { value.type_.Boolean })
        }
        bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_SIGNED_INT => {
            BACnetValue::Int(unsafe { value.type_.Signed_Int })
        }
        bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_UNSIGNED_INT => {
            BACnetValue::Uint(unsafe { value.type_.Unsigned_Int })
        }
        bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_REAL => {
            BACnetValue::Real(unsafe { value.type_.Real })
        }
        bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_DOUBLE => {
            BACnetValue::Double(unsafe { value.type_.Double })
        }
        bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_CHARACTER_STRING => {
            // BACnet string has the following structure
            // size_t length, uint8_t encoding, char value[MAX_CHARACTER_STRING_BYTES]
            // For now just assume UTF-8 bytes, but we really should respect encodings...
            //
            // FIXME(tj): Look at value.type_.Character_String.encoding
            let s = cstr(unsafe {
                value.type_.Character_String.value[0..value.type_.Character_String.length].as_ptr()
            });
            BACnetValue::String(s)
        }
        bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_OCTET_STRING => {
            let v = unsafe { value.type_.Octet_String };
            BACnetValue::Bytes(v.value[0..v.length].to_vec())
        }
        bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_BIT_STRING => {
            let nbits = unsafe { bitstring_bits_used(&mut value.type_.Bit_String) };
            // info!("Number of bits: {}", nbits);

            let mut bits = vec![];
            for i in 0..nbits {
                let bit = unsafe { bitstring_bit(&mut value.type_.Bit_String, i) };
                bits.push(bit);
            }

            BACnetValue::BitString(bits)
        }
        bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_ENUMERATED => {
            // FIXME(tj): Find the string representation of the enum (if possible).
            // See bacapp.c:1200
            // See bactext.c:1266 - bactext_binary_present_value_name()
            // Try calling:
            //
            // int bacapp_snprintf_value(char *str, size_t str_len, BACNET_OBJECT_PROPERTY_VALUE *object_value)
            //
            // It should return the numbers of characters written so we can permute it to a String
            let enum_val = unsafe { value.type_.Enumerated };
            let s = match data.object_property {
                bacnet_sys::BACNET_PROPERTY_ID_PROP_UNITS => {
                    if enum_val < 256 {
                        Some(cstr(unsafe { bactext_engineering_unit_name(enum_val) }))
                    } else {
                        None
                    }
                }
                bacnet_sys::BACNET_PROPERTY_ID_PROP_OBJECT_TYPE => {
                    if enum_val < MAX_ASHRAE_OBJECT_TYPE {
                        Some(cstr(unsafe { bactext_object_type_name(enum_val) }))
                    } else {
                        None // Either "reserved" or "proprietary"
                    }
                }
                bacnet_sys::BACNET_PROPERTY_ID_PROP_PRESENT_VALUE
                | bacnet_sys::BACNET_PROPERTY_ID_PROP_RELINQUISH_DEFAULT => {
                    if data.object_type < BACnetObjectType_OBJECT_PROPRIETARY_MIN {
                        Some(cstr(unsafe { bactext_binary_present_value_name(enum_val) }))
                    } else {
                        None
                    }
                }
                _ => None,
            };

            //switch (property) {
            //    case PROP_PROPERTY_LIST:
            //        char_str = (char *)bactext_property_name_default(
            //            value->type.Enumerated, NULL);
            //        if (char_str) {
            //            ret_val = snprintf(str, str_len, "%s", char_str);
            //        } else {
            //            ret_val = snprintf(str, str_len, "%lu",
            //                (unsigned long)value->type.Enumerated);
            //        }
            //        break;
            //    case PROP_OBJECT_TYPE:
            //        if (value->type.Enumerated < MAX_ASHRAE_OBJECT_TYPE) {
            //            ret_val = snprintf(str, str_len, "%s",
            //                bactext_object_type_name(
            //                    value->type.Enumerated));
            //        } else if (value->type.Enumerated < 128) {
            //            ret_val = snprintf(str, str_len, "reserved %lu",
            //                (unsigned long)value->type.Enumerated);
            //        } else {
            //            ret_val = snprintf(str, str_len, "proprietary %lu",
            //                (unsigned long)value->type.Enumerated);
            //        }
            //        break;
            //    case PROP_EVENT_STATE:
            //        ret_val = snprintf(str, str_len, "%s",
            //            bactext_event_state_name(value->type.Enumerated));
            //        break;
            //    case PROP_UNITS:
            //        if (value->type.Enumerated < 256) {
            //            ret_val = snprintf(str, str_len, "%s",
            //                bactext_engineering_unit_name(
            //                    value->type.Enumerated));
            //        } else {
            //            ret_val = snprintf(str, str_len, "proprietary %lu",
            //                (unsigned long)value->type.Enumerated);
            //        }
            //        break;
            //    case PROP_POLARITY:
            //        ret_val = snprintf(str, str_len, "%s",
            //            bactext_binary_polarity_name(
            //                value->type.Enumerated));
            //        break;
            //    case PROP_PRESENT_VALUE:
            //    case PROP_RELINQUISH_DEFAULT:
            //        if (object_type < OBJECT_PROPRIETARY_MIN) {
            //            ret_val = snprintf(str, str_len, "%s",
            //                bactext_binary_present_value_name(
            //                    value->type.Enumerated));
            //        } else {
            //            ret_val = snprintf(str, str_len, "%lu",
            //                (unsigned long)value->type.Enumerated);
            //        }
            //        break;
            //    case PROP_RELIABILITY:
            //        ret_val = snprintf(str, str_len, "%s",
            //            bactext_reliability_name(value->type.Enumerated));
            //        break;
            //    case PROP_SYSTEM_STATUS:
            //        ret_val = snprintf(str, str_len, "%s",
            //            bactext_device_status_name(value->type.Enumerated));
            //        break;
            //    case PROP_SEGMENTATION_SUPPORTED:
            //        ret_val = snprintf(str, str_len, "%s",
            //            bactext_segmentation_name(value->type.Enumerated));
            //        break;
            //    case PROP_NODE_TYPE:
            //        ret_val = snprintf(str, str_len, "%s",
            //            bactext_node_type_name(value->type.Enumerated));
            //        break;
            //    default:
            //        ret_val = snprintf(str, str_len, "%lu",
            //            (unsigned long)value->type.Enumerated);
            //        break;
            //}

            BACnetValue::Enum(enum_val, s)
        }
        bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_OBJECT_ID => {
            // Store the object list, so we can interrogate each object

            let object_type = unsafe { value.type_.Object_Id.type_ };
            let object_instance = unsafe { value.type_.Object_Id.instance };
            BACnetValue::ObjectId {
                object_type,
                object_instance,
            }
        }
        bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_DATE => {
            let date = unsafe { value.type_.Date };
            BACnetValue::Date {
                year: date.year,
                month: date.month,
                day: date.day,
                weekday: date.wday,
            }
        }
        _ => {
            let tag_name = cstr(unsafe { bactext_application_tag_name(value.tag as u32) });
            return Err(BACnetErr::UnhandledTag {
                tag_name,
                tag: value.tag,
            });
        }
    })
}

pub fn encode_data(value: BACnetValue) -> Result<BACNET_APPLICATION_DATA_VALUE, BACnetErr> {
    let mut data = BACNET_APPLICATION_DATA_VALUE {
        context_specific: false,
        ..Default::default()
    };

    match value {
        BACnetValue::Null => {
            data.tag = bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_NULL as u8;
        }
        BACnetValue::Bool(b) => {
            data.tag = bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_BOOLEAN as u8;
            data.type_.Boolean = b;
        }
        BACnetValue::Int(i) => {
            data.tag = bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_SIGNED_INT as u8;
            data.type_.Signed_Int = i;
        }
        BACnetValue::Uint(u) => {
            data.tag = bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_UNSIGNED_INT as u8;
            data.type_.Unsigned_Int = u;
        }
        BACnetValue::Real(f) => {
            data.tag = bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_REAL as u8;
            data.type_.Real = f;
        }
        BACnetValue::Double(f) => {
            data.tag = bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_DOUBLE as u8;
            data.type_.Double = f;
        }
        BACnetValue::Bytes(s) => {
            data.tag = bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_OCTET_STRING as u8;
            data.type_.Octet_String = BACNET_OCTET_STRING::default();
            data.type_.Octet_String.length = s.len();
            data.type_.Octet_String.value = [0; 1470];
            unsafe {
                data.type_.Octet_String.value[..s.len()].copy_from_slice(&s);
            }
        }
        BACnetValue::String(s) => {
            data.tag =
                bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_CHARACTER_STRING as u8;
            data.type_.Character_String = BACNET_CHARACTER_STRING::default();
            data.type_.Character_String.length = s.len();
            data.type_.Character_String.value = [0; 1470];
            unsafe {
                data.type_.Character_String.value[..s.len()].copy_from_slice(
                    s.as_bytes()
                        .iter()
                        .map(|&c| c as i8)
                        .collect::<Vec<i8>>()
                        .as_slice(),
                );
            }
        }
        BACnetValue::BitString(s) => {
            data.tag = bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_BIT_STRING as u8;
            data.type_.Bit_String = BACNET_BIT_STRING::default();
            unsafe { bitstring_init(&mut data.type_.Bit_String) };
            for (bit_number, value) in s.iter().enumerate() {
                unsafe {
                    bitstring_set_bit(&mut data.type_.Bit_String, bit_number as u8, *value);
                }
            }
        }
        BACnetValue::Enum(e, _) => {
            data.tag = bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_ENUMERATED as u8;
            data.type_.Enumerated = e;
        }
        BACnetValue::ObjectId {
            object_type,
            object_instance,
        } => {
            data.tag = bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_OBJECT_ID as u8;
            data.type_.Object_Id.type_ = object_type;
            data.type_.Object_Id.instance = object_instance;
        }
        BACnetValue::Date {
            year,
            month,
            day,
            weekday,
        } => {
            data.tag = bacnet_sys::BACNET_APPLICATION_TAG_BACNET_APPLICATION_TAG_DATE as u8;
            data.type_.Date.year = year;
            data.type_.Date.month = month;
            data.type_.Date.day = day;
            data.type_.Date.wday = weekday;
        }
        BACnetValue::Array(_) => return Err(BACnetErr::EncodeFailed),
    }

    Ok(data)
}
