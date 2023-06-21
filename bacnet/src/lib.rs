use anyhow::{anyhow, Result};
use bacnet_sys::{
    address_add, address_bind_request, address_remove_device, apdu_set_abort_handler,
    apdu_set_confirmed_ack_handler, apdu_set_confirmed_handler, apdu_set_error_handler,
    apdu_set_reject_handler, apdu_set_unconfirmed_handler,
    apdu_set_unrecognized_service_handler_handler, bacapp_decode_application_data,
    bacnet_address_same, bactext_abort_reason_name, bactext_application_tag_name,
    bactext_binary_present_value_name, bactext_engineering_unit_name, bactext_error_class_name,
    bactext_error_code_name, bactext_object_type_name, bactext_property_name, bip_receive,
    bitstring_bit, bitstring_bits_used, dlenv_init, handler_i_am_bind, handler_read_property,
    handler_unrecognized_service, handler_who_is, npdu_handler, property_list_special,
    rp_ack_decode_service_request, special_property_list_t, tsm_invoke_id_failed,
    tsm_invoke_id_free, BACnetObjectType_OBJECT_DEVICE, BACnetObjectType_OBJECT_PROPRIETARY_MIN,
    BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_READ_PROPERTY,
    BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_I_AM,
    BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_WHO_IS, Device_Init,
    Send_Read_Property_Request, BACNET_ADDRESS, BACNET_APPLICATION_DATA_VALUE, BACNET_ARRAY_ALL,
    BACNET_CONFIRMED_SERVICE_ACK_DATA, BACNET_ERROR_CLASS, BACNET_ERROR_CODE, BACNET_OBJECT_TYPE,
    BACNET_PROPERTY_ID, BACNET_PROPERTY_ID_PROP_OBJECT_LIST, BACNET_PROPERTY_ID_PROP_PRESENT_VALUE,
    BACNET_READ_PROPERTY_DATA, BACNET_STATUS_ERROR, MAX_APDU, MAX_ASHRAE_OBJECT_TYPE, MAX_MPDU,
};
pub use epics::Epics;
use lazy_static::lazy_static;
use log::{debug, error, info, log_enabled, warn};
use std::{
    cmp::min,
    collections::HashMap,
    ffi::CStr,
    net::Ipv4Addr,
    os::raw::c_char,
    sync::{Mutex, Once},
};
use thiserror::Error;
use value::BACnetValue;

mod epics;
pub mod value;
pub mod whohas;
pub mod whois;

static BACNET_STACK_INIT: Once = Once::new();

type RequestInvokeId = u8;
type DeviceId = u32;

// We need a global structure here for collecting "target addresses"
lazy_static! {
    /// Global tracking struct for target addresses. These are servers that we consider ourselves
    /// connected to and communicating with.
    static ref TARGET_ADDRESSES: Mutex<HashMap<DeviceId, TargetServer>> = Mutex::new(HashMap::new());
}

//// Epics property list
//lazy_static! {
//    static ref PROPERTY_LIST: Mutex<
//}
//
//struct PropertyList {
//    length: u32,
//    index: u32,
//    list: [130; i32],
//}

// Status of a request
enum RequestStatus {
    Ongoing,          // No reply has been received yet
    Done,             // Successfully completed
    Error(BACnetErr), // Request failed
}

#[derive(Debug, Error)]
pub enum BACnetErr {
    /// Request was rejected with the given reason code
    #[error("Rejected: code {code}")]
    Rejected { code: u8 }, // Rejected with the given reason code

    /// Request was aborted with the given reason code and text
    #[error("Aborted: {text} (code {code})")]
    Aborted { text: String, code: u8 },

    /// Request resulted in an error
    #[error("Error: class={class_text} ({class}) {text} ({code})")]
    Error {
        class_text: String,
        class: u32,
        text: String,
        code: u32,
    },

    /// Request is still ongoing
    #[error("Request is still ongoing")]
    RequestOngoing,

    /// No value was extracted
    #[error("No value was extracted")]
    NoValue,

    /// Invalid value was extracted
    #[error("Invalid value was extracted")]
    InvalidValue,

    /// Not connected to server
    #[error("Not connected to server with Device ID {device_id}")]
    NotConnected { device_id: u32 },

    /// TSM Timeout
    #[error("TSM Timeout")]
    TsmTimeout,

    /// APDU Timeout
    #[error("APDU Timeout")]
    ApduTimeout,

    /// Decoding failed
    #[error("Decoding failed")]
    DecodeFailed,

    /// Unhandled tag type
    #[error("Unhandled type tag {tag_name} ({tag:?})")]
    UnhandledTag { tag_name: String, tag: u8 },
}

// A structure for tracking
//
// FIXME(tj): This is a really poor hand-off mechanism. When making a request, we set the
// request_invoke_id so the response can be matched properly, then we set the decoded value inside
// an Option and read_prop() fishes it out. This means that read_prop() needs to acquire the mutex
// twice for each data extraction, which seems like a really poor design.
struct TargetServer {
    addr: BACNET_ADDRESS,
    request: Option<(RequestInvokeId, RequestStatus)>, // For tracking on-going an ongoing request
    value: Option<Result<BACnetValue, BACnetErr>>,     // TODO Build this into the 'request status'
}

// As I understand the BACnet stack, it works by acting as another BACnet device on the network.
//
// This means that there's not really a way
// to "connect" to a server, we call address_bind_request(device_id, ..) which adds the server (if
// possible) to the internal address cache. [sidenote: The MAX_ADDRESS_CACHE = 255, which I take to
// mean that we can connect to at most 255 devices].

#[derive(Debug, Clone)]
pub struct BACnetServer {
    pub device_id: u32,
    max_apdu: u32,
    addr: BACNET_ADDRESS,
}

pub type ObjectType = BACNET_OBJECT_TYPE;
pub type ObjectPropertyId = BACNET_PROPERTY_ID;

impl BACnetServer {
    pub fn builder() -> BACnetServerBuilder {
        BACnetServerBuilder::default()
    }

    pub fn connect(&mut self) -> Result<()> {
        BACNET_STACK_INIT.call_once(|| unsafe {
            init_service_handlers();
            dlenv_init();
        });
        // Add address
        unsafe {
            address_add(self.device_id, MAX_APDU, &mut self.addr);
        }
        let mut target_addr = BACNET_ADDRESS::default();
        // FIXME(tj): Wait until server is bound, or timeout
        let found =
            unsafe { address_bind_request(self.device_id, &mut self.max_apdu, &mut target_addr) };
        debug!("found = {}", found);
        if found {
            let mut lock = TARGET_ADDRESSES.lock().unwrap();
            lock.insert(
                self.device_id,
                TargetServer {
                    addr: target_addr,
                    request: None,
                    value: None,
                },
            );
            Ok(())
        } else {
            Err(anyhow!(
                "Failed to bind to the server with Device ID {}",
                self.device_id
            ))
        }
    }

    // Read_Property
    //
    // Only reads the present value (property 85)
    pub fn read_prop_present_value(
        &self,
        object_type: ObjectType,
        object_instance: u32,
    ) -> Result<BACnetValue, BACnetErr> {
        self.read_prop(
            object_type,
            object_instance,
            BACNET_PROPERTY_ID_PROP_PRESENT_VALUE,
        )
    }

    /// Read a property
    ///
    /// We call Send_Read_Property_Request, and wait for a result.
    pub fn read_prop(
        &self,
        object_type: ObjectType,
        object_instance: u32,
        property_id: ObjectPropertyId,
    ) -> Result<BACnetValue, BACnetErr> {
        self.read_prop_at(object_type, object_instance, property_id, BACNET_ARRAY_ALL)
    }

    pub fn read_prop_at(
        &self,
        object_type: ObjectType,
        object_instance: u32,
        property_id: ObjectPropertyId,
        index: u32,
    ) -> Result<BACnetValue, BACnetErr> {
        let init = std::time::Instant::now();
        const TIMEOUT: u32 = 100;
        let request_invoke_id =
            if let Some(h) = TARGET_ADDRESSES.lock().unwrap().get_mut(&self.device_id) {
                let request_invoke_id = unsafe {
                    Send_Read_Property_Request(
                        self.device_id,
                        object_type,
                        object_instance,
                        property_id,
                        index,
                    )
                };
                h.request = Some((request_invoke_id, RequestStatus::Ongoing));
                request_invoke_id
            } else {
                return Err(BACnetErr::NotConnected {
                    device_id: self.device_id,
                });
            };

        let mut src = BACNET_ADDRESS::default();
        let mut rx_buf = [0u8; MAX_MPDU as usize];
        let start = std::time::Instant::now();
        loop {
            // TODO(tj): Consider pulling the "driving forward the internal state machine" stuff
            // into an inner method here. We need it for EPICS as well.
            let pdu_len =
                unsafe { bip_receive(&mut src, &mut rx_buf as *mut _, MAX_MPDU as u16, TIMEOUT) };
            if pdu_len > 0 {
                unsafe { npdu_handler(&mut src, &mut rx_buf as *mut _, pdu_len) }
            }

            if unsafe { tsm_invoke_id_free(request_invoke_id) } {
                break;
            }
            if unsafe { tsm_invoke_id_failed(request_invoke_id) } {
                return Err(BACnetErr::TsmTimeout);
            }

            if start.elapsed().as_secs() > 3 {
                return Err(BACnetErr::ApduTimeout);
            }
        }

        let ret = {
            let mut lock = TARGET_ADDRESSES.lock().unwrap();

            let h = lock.get_mut(&self.device_id);
            if h.is_none() {
                return Err(BACnetErr::NotConnected {
                    device_id: self.device_id,
                });
            }
            let h = h.unwrap();

            let request_status = h.request.take();
            if request_status.is_none() {
                return Err(BACnetErr::NoValue);
            }
            let request_status = request_status.unwrap();

            match request_status.1 {
                RequestStatus::Done => h.value.take().unwrap_or(Err(BACnetErr::NoValue)),
                RequestStatus::Ongoing => Err(BACnetErr::RequestOngoing),
                RequestStatus::Error(err) => Err(err),
            }
        };

        debug!("read_prop() finished in {:?}", init.elapsed());
        ret
    }

    /// Read all required properties for a given object-type and object-instance
    ///
    /// The BACnet stack internally has a list of required properties for a given object-type, and
    /// this function will simply walk over every single one and call `read_prop()` on it.
    pub fn read_properties(
        &self,
        object_type: BACNET_OBJECT_TYPE,
        object_instance: u32,
    ) -> Result<HashMap<ObjectPropertyId, BACnetValue>, BACnetErr> {
        let mut special_property_list = special_property_list_t::default();

        // Fetch all the properties that are known to be required here.
        unsafe {
            property_list_special(object_type, &mut special_property_list);
        }

        let len = min(special_property_list.Required.count, 130);
        let mut ret = HashMap::with_capacity(len as usize);
        for i in 0..len {
            let prop = unsafe { *special_property_list.Required.pList.offset(i as isize) } as u32;

            if log_enabled!(log::Level::Debug) {
                let prop_name = cstr(unsafe { bactext_property_name(prop) });
                debug!("Required property {} ({})", prop_name, prop);
            }
            if prop == BACNET_PROPERTY_ID_PROP_OBJECT_LIST {
                // This particular property we will not try to read in one go, instead we'll resort
                // to reading it an item at a time.
                continue;
            }
            match self.read_prop(object_type, object_instance, prop) {
                Ok(v) => {
                    debug!("OK. Got value {:?}", v);
                    ret.insert(prop, v);
                }
                Err(bacnet_err) => {
                    match bacnet_err {
                        // If bacnet_err is unknown property, just debug it and move on
                        BACnetErr::Error {
                            class: 2, code: 32, ..
                        } => {
                            debug!("{}", bacnet_err);
                        }
                        // If we get a timeout, we'll just return the error
                        BACnetErr::TsmTimeout | BACnetErr::ApduTimeout => {
                            return Err(bacnet_err);
                        }
                        _ => {
                            warn!("{}", bacnet_err);
                        }
                    }
                }
            }
        }

        // Look at optional properties
        let optlen = min(special_property_list.Optional.count, 130 - len);
        for i in 0..optlen {
            let prop = unsafe { *special_property_list.Optional.pList.offset(i as isize) } as u32;

            if log_enabled!(log::Level::Debug) {
                let prop_name = cstr(unsafe { bactext_property_name(prop) });
                debug!("Optional property {} ({})", prop_name, prop);
            }
            match self.read_prop(object_type, object_instance, prop) {
                Ok(v) => {
                    debug!("OK. Got value {:?}", v);
                    ret.insert(prop, v);
                }
                Err(bacnet_err) => {
                    match bacnet_err {
                        BACnetErr::Aborted { code, .. } if code == 4 => {
                            // code == 4 is "segmentation not supported". This is an array
                            let len: Result<u64, _> = self
                                .read_prop_at(object_type, object_instance, prop, 0)
                                .and_then(|x| x.try_into().map_err(|_| BACnetErr::InvalidValue));

                            if let Ok(len) = len {
                                let mut ary = Vec::with_capacity(len as usize);
                                for i in 0..len {
                                    if let Ok(val) = self.read_prop_at(
                                        object_type,
                                        object_instance,
                                        prop,
                                        i as u32 + 1,
                                    ) {
                                        ary.push(val);
                                    }
                                }
                                ret.insert(prop, BACnetValue::Array(ary));
                            }
                        }
                        BACnetErr::TsmTimeout | BACnetErr::ApduTimeout => {
                            // If we get a timeout, we'll just return the error
                            return Err(bacnet_err);
                        }
                        _ => {
                            warn!("{}", bacnet_err);
                        }
                    }
                }
            }
        }

        Ok(ret)
    }

    /// Scan the server for all available properties and produce an `Epics` object
    pub fn epics(&self) -> Result<Epics, BACnetErr> {
        let device_props = self.read_properties(BACnetObjectType_OBJECT_DEVICE, self.device_id)?;

        // Read the object-list
        let len: u64 = self
            .read_prop_at(
                BACnetObjectType_OBJECT_DEVICE,
                self.device_id,
                BACNET_PROPERTY_ID_PROP_OBJECT_LIST,
                0,
            )?
            .try_into()
            .map_err(|_| BACnetErr::InvalidValue)?;

        let mut object_ids = Vec::with_capacity(len as usize);
        for i in 2..len + 1 {
            match self.read_prop_at(
                BACnetObjectType_OBJECT_DEVICE,
                self.device_id,
                BACNET_PROPERTY_ID_PROP_OBJECT_LIST,
                i as u32,
            )? {
                BACnetValue::ObjectId {
                    object_type,
                    object_instance,
                } => {
                    object_ids.push((object_type, object_instance));
                }
                v => error!("Unexpected type when reading object-list {:?}", v),
            }
        }

        debug!("{:#?}", device_props);
        debug!("object-list has {} elements", len);
        debug!("{:#?}", object_ids);

        let mut objects = Vec::with_capacity(len as usize);
        for (object_type, object_instance) in object_ids {
            let object_props = self.read_properties(object_type, object_instance)?;
            objects.push(object_props);
        }
        debug!("Objects:\n{:#?}", objects);

        // Populate
        let device = device_props
            .into_iter()
            .map(|(id, val)| (cstr(unsafe { bactext_property_name(id) }), val))
            .collect::<HashMap<_, _>>();

        let object_list = objects
            .into_iter()
            .map(|obj| {
                obj.into_iter()
                    .map(|(id, val)| (cstr(unsafe { bactext_property_name(id) }), val))
                    .collect::<HashMap<_, _>>()
            })
            .collect::<Vec<_>>();

        Ok(Epics {
            device,
            object_list,
        })
    }

    pub fn disconnect(&self) {
        unsafe {
            address_remove_device(self.device_id);
        }
    }
}

impl Drop for BACnetServer {
    fn drop(&mut self) {
        info!("disconnecting");
        unsafe { address_remove_device(self.device_id) };
    }
}

#[derive(Debug)]
pub struct BACnetServerBuilder {
    ip: Ipv4Addr,
    dnet: u16,
    dadr: u8,
    port: u16,
    device_id: u32,
}

impl Default for BACnetServerBuilder {
    fn default() -> Self {
        Self {
            ip: Ipv4Addr::LOCALHOST,
            dnet: 0,
            dadr: 0,
            port: 0xBAC0,
            device_id: 0,
        }
    }
}

impl BACnetServerBuilder {
    pub fn ip(mut self, ip: Ipv4Addr) -> Self {
        self.ip = ip;
        self
    }

    pub fn dnet(mut self, dnet: u16) -> Self {
        self.dnet = dnet;
        self
    }

    pub fn dadr(mut self, dadr: u8) -> Self {
        self.dadr = dadr;
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn device_id(mut self, device_id: u32) -> Self {
        self.device_id = device_id;
        self
    }

    pub fn build(self) -> BACnetServer {
        let BACnetServerBuilder {
            ip,
            dnet,
            dadr,
            port,
            device_id,
        } = self;
        let mut addr = BACNET_ADDRESS::default();
        addr.mac[..4].copy_from_slice(&ip.octets());
        addr.mac[4] = (port >> 8) as u8;
        addr.mac[5] = (port & 0xff) as u8;
        addr.mac_len = 6;
        addr.net = dnet;
        addr.adr[0] = dadr;
        addr.len = 1;

        BACnetServer {
            device_id,
            max_apdu: 0,
            addr,
        }
    }
}

#[no_mangle]
extern "C" fn my_readprop_ack_handler(
    service_request: *mut u8,
    service_len: u16,
    src: *mut BACNET_ADDRESS,
    service_data: *mut BACNET_CONFIRMED_SERVICE_ACK_DATA,
) {
    let mut data: BACNET_READ_PROPERTY_DATA = BACNET_READ_PROPERTY_DATA::default();

    let invoke_id = unsafe { (*service_data).invoke_id };
    let mut lock = TARGET_ADDRESSES.lock().unwrap();
    if let Some(target) = find_matching_server(&mut lock, src, invoke_id) {
        // Decode the data
        let len = unsafe {
            rp_ack_decode_service_request(service_request, service_len.into(), &mut data as *mut _)
        };
        if len >= 0 {
            // XXX Consider moving data decoding out. We should probably just stick to getting
            // the raw data, putting it somewhere and let someone else decode it.
            let decoded = decode_data(data);
            target.value = Some(decoded);
        } else {
            error!("<decode failed>");
            target.value = Some(Err(BACnetErr::DecodeFailed));
        }
        target.request = Some((invoke_id, RequestStatus::Done));
    }
}

fn decode_data(data: BACNET_READ_PROPERTY_DATA) -> Result<BACnetValue, BACnetErr> {
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
        _ => {
            let tag_name = cstr(unsafe { bactext_application_tag_name(value.tag as u32) });
            return Err(BACnetErr::UnhandledTag {
                tag_name,
                tag: value.tag,
            });
        }
    })
}

#[no_mangle]
extern "C" fn my_readpropmultiple_ack_handler(
    _: u16,
    _: *mut BACNET_ADDRESS,
    _: *mut BACNET_CONFIRMED_SERVICE_ACK_DATA,
) {
    // TODO
    unimplemented!();
}

#[no_mangle]
extern "C" fn my_error_handler(
    src: *mut BACNET_ADDRESS,
    invoke_id: u8,
    error_class: BACNET_ERROR_CLASS,
    error_code: BACNET_ERROR_CODE,
) {
    let mut lock = TARGET_ADDRESSES.lock().unwrap();
    if let Some(target) = find_matching_server(&mut lock, src, invoke_id) {
        let error_class_str = cstr(unsafe { bactext_error_class_name(error_class) });
        let error_code_str = cstr(unsafe { bactext_error_code_name(error_code) });
        debug!(
            "BACnet error: error_class={} ({}) error_code={} ({})",
            error_class, error_class_str, error_code, error_code_str,
        );
        let err = BACnetErr::Error {
            class_text: error_class_str,
            class: error_class,
            text: error_code_str,
            code: error_code,
        };
        target.request = Some((invoke_id, RequestStatus::Error(err)));
    }
}

#[no_mangle]
extern "C" fn my_abort_handler(
    src: *mut BACNET_ADDRESS,
    invoke_id: u8,
    abort_reason: u8,
    server: bool,
) {
    let _ = server;
    let _ = src;
    let mut lock = TARGET_ADDRESSES.lock().unwrap();
    if let Some(target) = find_matching_server(&mut lock, src, invoke_id) {
        let abort_text = cstr(unsafe { bactext_abort_reason_name(abort_reason as u32) });
        debug!(
            "aborted invoke_id = {} abort_reason = {} ({})",
            invoke_id, abort_text, abort_reason
        );
        let err_abort = BACnetErr::Aborted {
            text: abort_text,
            code: abort_reason,
        };
        target.request = Some((invoke_id, RequestStatus::Error(err_abort)));
    }
}

#[no_mangle]
extern "C" fn my_reject_handler(src: *mut BACNET_ADDRESS, invoke_id: u8, reject_reason: u8) {
    let _ = src;

    let mut lock = TARGET_ADDRESSES.lock().unwrap();
    if let Some(target) = find_matching_server(&mut lock, src, invoke_id) {
        target.request = Some((
            invoke_id,
            RequestStatus::Error(BACnetErr::Rejected {
                code: reject_reason,
            }),
        ));
    }
}

fn cstr(ptr: *const c_char) -> String {
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

// Holding the lock on the global map of servers, find a server that matches `src` and the given
// RequestInvokeId.
//
// This function _should_ return something.
fn find_matching_server<'a>(
    guard: &'a mut std::sync::MutexGuard<'_, HashMap<u32, TargetServer>>,
    src: *mut BACNET_ADDRESS,
    invoke_id: RequestInvokeId,
) -> Option<&'a mut TargetServer> {
    for target in guard.values_mut() {
        let is_addr_match = unsafe { bacnet_address_same(&mut target.addr, src) };
        if let Some((request_invoke_id, _)) = &target.request {
            let is_request_invoke_id = invoke_id == *request_invoke_id;
            if is_addr_match && is_request_invoke_id {
                return Some(target);
            }
        }
    }
    error!("Server wasn't matched! {:?}", src);
    None
}

unsafe fn init_service_handlers() {
    Device_Init(std::ptr::null_mut());
    apdu_set_unconfirmed_handler(
        BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_WHO_IS,
        Some(handler_who_is),
    );
    apdu_set_unconfirmed_handler(
        BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_I_AM,
        Some(handler_i_am_bind),
    );
    apdu_set_unrecognized_service_handler_handler(Some(handler_unrecognized_service));
    apdu_set_confirmed_handler(
        BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_READ_PROPERTY,
        Some(handler_read_property),
    );
    apdu_set_confirmed_ack_handler(
        BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_READ_PROPERTY,
        Some(my_readprop_ack_handler),
    );

    apdu_set_error_handler(
        BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_READ_PROPERTY,
        Some(my_error_handler),
    );
    apdu_set_abort_handler(Some(my_abort_handler));
    apdu_set_reject_handler(Some(my_reject_handler));
}
