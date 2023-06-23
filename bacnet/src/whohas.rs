//! A highlevel interface to bacnet-sys prop discovery (Who-Has) functionality
//!
//! Design is like a builder with different parameters and returns

// So the design of the BACnet stack is a little annoying in that we have to drive the subsystem
// forward, continually called bip_receive(). Each device that's discovered is processed by the
// my_i_have_handler, and we need to a global list of discovered features.
//
// In effect, this library is not thread-safe, so we need to make sure that only one WhoHas client
// is running at a time.

use crate::{cstr, ObjectType};
use anyhow::{bail, Result};
use bacnet_sys::{
    address_init, apdu_set_confirmed_handler, apdu_set_unconfirmed_handler,
    apdu_set_unrecognized_service_handler_handler, bactext_object_type_name, bip_cleanup,
    bip_get_broadcast_address, bip_receive, characterstring_value, dlenv_init,
    handler_read_property, handler_unrecognized_service, handler_who_is,
    ihave_decode_service_request, npdu_handler, BACnetObjectType_OBJECT_DEVICE,
    BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_READ_PROPERTY,
    BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_I_HAVE,
    BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_WHO_IS, Device_Init,
    Device_Set_Object_Instance_Number, Send_WhoHas_Object, BACNET_ADDRESS, BACNET_I_HAVE_DATA,
    BACNET_MAX_INSTANCE, MAX_MPDU,
};
use lazy_static::lazy_static;
use log::{debug, error, trace};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

lazy_static! {
    /// A global list of discovered devices. The function my_i_am_handler() pushes discovered
    /// devices here.
    static ref DISCOVERED_DEVICES: Mutex<Vec<IHaveData>> = Mutex::new(vec![]);
}

pub struct ObjectId {
    pub object_type: ObjectType,
    pub object_instance: u32,
}

/// A BACnet device that responded with I-Am in response to the Who-Has we sent out.
pub struct IHaveData {
    pub device_id: ObjectId,
    pub object_id: ObjectId,
    pub object_name: String,
}

impl From<BACNET_I_HAVE_DATA> for IHaveData {
    fn from(data: BACNET_I_HAVE_DATA) -> Self {
        let device_id = ObjectId {
            object_type: data.device_id.type_,
            object_instance: data.device_id.instance,
        };
        let object_id = ObjectId {
            object_type: data.object_id.type_,
            object_instance: data.object_id.instance,
        };
        let object_name = unsafe { cstr(characterstring_value(&mut data.object_name.clone())) };
        IHaveData {
            device_id,
            object_id,
            object_name,
        }
    }
}

pub struct WhoHas {
    /// Object type to search for
    object_type: ObjectType,

    /// Object instance to search for
    object_instance: u32,

    /// How long to wait until we stop listening for I-Am requests.
    timeout: Duration,

    /// Restrict whohas query to the given subnet, default is `None` which means a global broadcast.
    subnet: Option<u16>,
}

// WhoHas::new().timeout(1000).execute()
impl WhoHas {
    pub fn new() -> WhoHas {
        WhoHas::default()
    }

    /// Set the object type to search for. Default: Device
    pub fn object_type(mut self, object_type: ObjectType) -> Self {
        self.object_type = object_type;
        self
    }

    /// Set the object instance to search for. Default: 0
    pub fn object_instance(mut self, object_instance: u32) -> Self {
        self.object_instance = object_instance;
        self
    }

    /// Set the amount of time to wait for I-Am requests to come in (in millis). Default: 3000
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn subnet<S>(mut self, subnet: S) -> Self
    where
        S: Into<Option<u16>>,
    {
        self.subnet = subnet.into();
        self
    }

    pub fn execute(self) -> Result<Vec<IHaveData>> {
        let WhoHas {
            object_type,
            object_instance,
            timeout,
            subnet,
        } = self;

        // create an object with a Drop impl that calls bip_cleanup
        whohas(object_type, object_instance, timeout, subnet);

        let devices = if let Ok(mut lock) = DISCOVERED_DEVICES.lock() {
            lock.drain(..).collect()
        } else {
            bail!("unable to lock DISCOVERED_DEVICES");
        };

        Ok(devices)
    }
}

impl Default for WhoHas {
    fn default() -> Self {
        WhoHas {
            object_type: BACnetObjectType_OBJECT_DEVICE,
            object_instance: 0,
            timeout: Duration::from_secs(3),
            subnet: None,
        }
    }
}

#[no_mangle]
extern "C" fn i_have_handler(service_request: *mut u8, service_len: u16, _: *mut BACNET_ADDRESS) {
    let mut data: BACNET_I_HAVE_DATA = BACNET_I_HAVE_DATA::default();

    let len =
        unsafe { ihave_decode_service_request(service_request, service_len as u32, &mut data) };
    if len == -1 {
        error!("unable to decode I-Have request...");
        return;
    }
    unsafe {
        debug!(
            "device_id = {} object_id = {} object_name = {}",
            cstr(bactext_object_type_name(data.device_id.type_)),
            cstr(bactext_object_type_name(data.object_id.type_)),
            cstr(characterstring_value(&mut data.object_name))
        );
    }

    if let Ok(mut lock) = DISCOVERED_DEVICES.lock() {
        lock.push(data.into());
    }
}

fn whohas(object_type: ObjectType, object_instance: u32, timeout: Duration, subnet: Option<u16>) {
    let mut dest = BACNET_ADDRESS::default();
    let target_object_instance_min = -1i32; // TODO(tj): parameterize?
    let target_object_instance_max = -1i32; // TODO(tj): parameterize?

    if let Some(subnet) = subnet {
        dest.net = subnet;
    } else {
        unsafe {
            bip_get_broadcast_address(&mut dest as *mut _);
        }
    }

    unsafe {
        Device_Set_Object_Instance_Number(BACNET_MAX_INSTANCE);
        // service handlers
        Device_Init(std::ptr::null_mut());
        apdu_set_unconfirmed_handler(
            BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_WHO_IS,
            Some(handler_who_is),
        );
        apdu_set_unrecognized_service_handler_handler(Some(handler_unrecognized_service));
        apdu_set_confirmed_handler(
            BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_READ_PROPERTY,
            Some(handler_read_property),
        );
        apdu_set_unconfirmed_handler(
            BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_I_HAVE,
            Some(i_have_handler),
        );

        // FIXME(tj): Set error handlers
        // apdu_set_abort_handler(MyAbortHandler);
        // apdu_set_reject_handler(MyRejectHandler);
        address_init();
        dlenv_init();
    }

    let mut src = BACNET_ADDRESS::default();
    let mut rx_buf = [0u8; MAX_MPDU as usize];
    let bip_timeout = 100; // ms
    unsafe {
        Send_WhoHas_Object(
            target_object_instance_min,
            target_object_instance_max,
            object_type,
            object_instance,
        );
    }
    let start = Instant::now();
    let mut i = 0;
    while start.elapsed() < timeout {
        let pdu_len = unsafe {
            bip_receive(
                &mut src as *mut _,
                &mut rx_buf as *mut _,
                MAX_MPDU as u16,
                bip_timeout,
            )
        };
        if pdu_len > 0 {
            unsafe {
                npdu_handler(&mut src as *mut _, &mut rx_buf as *mut _, pdu_len);
            }
        }

        i += 1;
    }
    trace!("Looped {} times", i);

    unsafe {
        bip_cleanup();
    }
}
