use crate::encoding::encode_data;
use anyhow::{anyhow, Result};
use bacnet_sys::{
    address_add, address_bind_request, address_init, address_remove_device, apdu_set_abort_handler,
    apdu_set_confirmed_ack_handler, apdu_set_confirmed_handler,
    apdu_set_confirmed_simple_ack_handler, apdu_set_error_handler, apdu_set_reject_handler,
    apdu_set_unconfirmed_handler, apdu_set_unrecognized_service_handler_handler,
    bacnet_address_same, bactext_abort_reason_name, bactext_error_class_name,
    bactext_error_code_name, bactext_property_name, bip_cleanup, bip_receive, dlenv_init,
    handler_read_property, handler_unrecognized_service, handler_who_is, npdu_handler,
    property_list_special, rp_ack_decode_service_request, special_property_list_t,
    tsm_invoke_id_failed, tsm_invoke_id_free, BACnetObjectType_OBJECT_DEVICE,
    BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_READ_PROPERTY,
    BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_WRITE_PROPERTY,
    BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_I_AM,
    BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_I_HAVE,
    BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_WHO_IS, Device_Init,
    Send_Read_Property_Request, Send_Write_Property_Request, BACNET_ADDRESS, BACNET_ARRAY_ALL,
    BACNET_CONFIRMED_SERVICE_ACK_DATA, BACNET_ERROR_CLASS, BACNET_ERROR_CODE, BACNET_OBJECT_TYPE,
    BACNET_PROPERTY_ID, BACNET_PROPERTY_ID_PROP_OBJECT_LIST, BACNET_PROPERTY_ID_PROP_PRESENT_VALUE,
    BACNET_READ_PROPERTY_DATA, MAX_APDU, MAX_MPDU,
};
use encoding::decode_data;
pub use epics::Epics;
use lazy_static::lazy_static;
use log::{debug, error, info, log_enabled, trace, warn};
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
use whohas::i_have_handler;
use whois::i_am_handler;

mod encoding;
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

    /// Encode failed
    #[error("Encode failed")]
    EncodeFailed,

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
            bip_cleanup();
            init_service_handlers();
            address_init();
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

    /// Reads a property
    ///
    /// Only reads the present value (property 85)
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

    /// Reads a property
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

    /// Reads a property at a specific index
    ///
    /// We call Send_Read_Property_Request, and wait for a result.
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

        trace!("read_prop_at() finished in {:?}", init.elapsed());
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
                        // If we get another error on a optional property, we'll just ignore it
                        _ => {}
                    }
                }
            }
        }

        Ok(ret)
    }

    /// Writes a property
    ///
    /// Only writes the present value (property 85)
    pub fn write_prop_present_value(
        &self,
        object_type: ObjectType,
        object_instance: u32,
        value: BACnetValue,
    ) -> Result<(), BACnetErr> {
        self.write_prop(
            object_type,
            object_instance,
            value,
            BACNET_PROPERTY_ID_PROP_PRESENT_VALUE,
        )
    }

    /// Writes a property
    ///
    /// We call Send_Write_Property_Request, and wait for a result.
    pub fn write_prop(
        &self,
        object_type: ObjectType,
        object_instance: u32,
        value: BACnetValue,
        property_id: ObjectPropertyId,
    ) -> Result<(), BACnetErr> {
        self.write_prop_at(
            object_type,
            object_instance,
            value,
            property_id,
            BACNET_ARRAY_ALL,
        )
    }

    /// Writes a property at a specific index
    ///
    /// We call Send_Write_Property_Request, and wait for a result.
    pub fn write_prop_at(
        &self,
        object_type: ObjectType,
        object_instance: u32,
        value: BACnetValue,
        property_id: ObjectPropertyId,
        index: u32,
    ) -> Result<(), BACnetErr> {
        let init = std::time::Instant::now();
        const TIMEOUT: u32 = 100;
        let request_invoke_id =
            if let Some(h) = TARGET_ADDRESSES.lock().unwrap().get_mut(&self.device_id) {
                let request_invoke_id = unsafe {
                    let mut object_value = encode_data(value)?;

                    Send_Write_Property_Request(
                        self.device_id,
                        object_type,
                        object_instance,
                        property_id,
                        &mut object_value,
                        0,
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
                RequestStatus::Done => Ok(()),
                RequestStatus::Ongoing => Err(BACnetErr::RequestOngoing),
                RequestStatus::Error(err) => Err(err),
            }
        };

        trace!("write_prop_at() finished in {:?}", init.elapsed());
        ret
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
        info!("disconnecting");
        unsafe { address_remove_device(self.device_id) };
    }
}

impl Drop for BACnetServer {
    fn drop(&mut self) {
        self.disconnect();
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

#[no_mangle]
extern "C" fn my_property_simple_ack_handler(src: *mut BACNET_ADDRESS, invoke_id: u8) {
    let mut lock = TARGET_ADDRESSES.lock().unwrap();
    if let Some(target) = find_matching_server(&mut lock, src, invoke_id) {
        target.request = Some((invoke_id, RequestStatus::Done));
    }
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

/// # Safety
///
/// We have to declare this function as unsafe but it's actually safe. The reason is that the
/// whole function is a callback from the C library and we have to declare it as unsafe.
pub unsafe fn init_service_handlers() {
    Device_Init(std::ptr::null_mut());
    apdu_set_unconfirmed_handler(
        BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_WHO_IS,
        Some(handler_who_is),
    );
    apdu_set_unconfirmed_handler(
        BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_I_AM,
        Some(i_am_handler),
    );
    apdu_set_unconfirmed_handler(
        BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_I_HAVE,
        Some(i_have_handler),
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
    apdu_set_confirmed_simple_ack_handler(
        BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_WRITE_PROPERTY,
        Some(my_property_simple_ack_handler),
    );

    apdu_set_error_handler(
        BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_READ_PROPERTY,
        Some(my_error_handler),
    );
    apdu_set_abort_handler(Some(my_abort_handler));
    apdu_set_reject_handler(Some(my_reject_handler));
}
