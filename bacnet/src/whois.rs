//! A highlevel interface to bacnet-sys discovery (Who-Is) functionality
//!
//! Design is like a builder with different parameters and returns

// So the design of the BACnet stack is a little annoying in that we have to drive the subsystem
// forward, continually called bip_receive(). Each device that's discovered is processed by the
// my_i_am_handler, and we need to a global list of discovered devices.
//
// In effect, this library is not thread-safe, so we need to make sure that only one WhoIs client
// is running at a time.

use anyhow::{bail, Result};
use bacnet_sys::{
    address_init, apdu_set_confirmed_handler, apdu_set_unconfirmed_handler,
    apdu_set_unrecognized_service_handler_handler, bip_cleanup, bip_get_broadcast_address,
    bip_receive, dlenv_init, handler_read_property, iam_decode_service_request, npdu_handler,
    BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_READ_PROPERTY,
    BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_I_AM, Device_Init,
    Device_Set_Object_Instance_Number, Send_WhoIs_To_Network, BACNET_ADDRESS, BACNET_MAX_INSTANCE,
    MAX_MPDU,
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
    static ref DISCOVERED_DEVICES: Mutex<Vec<IAmDevice>> = Mutex::new(vec![]);
}

/// A BACnet device that responded with I-Am in response to the Who-Is we sent out.
pub struct IAmDevice {
    pub device_id: u32,
    pub max_apdu: u32,
    pub vendor_id: u16,
    pub mac_addr: [u8; 6],
    pub network_number: u16,
    pub addr: [u8; 6],
}

pub struct WhoIs {
    /// How long to wait until we stop listening for I-Am requests.
    timeout: Duration,

    /// Restrict whois query to the given subnet, default is `None` which means a global broadcast.
    subnet: Option<u16>,
}

// WhoIs::new().timeout(1000).execute()
impl WhoIs {
    pub fn new() -> WhoIs {
        WhoIs::default()
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

    pub fn execute(self) -> Result<Vec<IAmDevice>> {
        let WhoIs { timeout, subnet } = self;

        // create an object with a Drop impl that calls bip_cleanup
        whois(timeout, subnet);

        let devices = if let Ok(mut lock) = DISCOVERED_DEVICES.lock() {
            lock.drain(..).collect()
        } else {
            bail!("unable to lock DISCOVERED_DEVICES");
        };

        Ok(devices)
    }
}

impl Default for WhoIs {
    fn default() -> Self {
        WhoIs {
            timeout: Duration::from_secs(3),
            subnet: None,
        }
    }
}

#[no_mangle]
extern "C" fn i_am_handler(service_request: *mut u8, _service_len: u16, src: *mut BACNET_ADDRESS) {
    let mut device_id = 0;
    let mut max_apdu = 0;
    let mut segmentation = 0;
    let mut vendor_id = 0;

    let len = unsafe {
        iam_decode_service_request(
            service_request,
            &mut device_id,
            &mut max_apdu,
            &mut segmentation,
            &mut vendor_id,
        )
    };
    if len == -1 {
        error!("unable to decode I-Am request...");
        return;
    }
    debug!(
        "device_id = {} max_apdu = {} vendor_id = {}",
        device_id, max_apdu, vendor_id
    );
    let mac_len = unsafe { (*src).mac_len } as usize;
    let mut mac_addr = [0u8; 6];
    mac_addr[..mac_len].copy_from_slice(unsafe { &(*src).mac[..mac_len] });
    let network_number = unsafe { (*src).net };

    let mut addr = [0u8; 6];
    if network_number > 0 {
        let adr_len = unsafe { (*src).len } as usize;
        addr[..adr_len].copy_from_slice(unsafe { &(*src).adr[..adr_len] });
    }

    debug!("MAC = {:02X?}", mac_addr);
    if let Ok(mut lock) = DISCOVERED_DEVICES.lock() {
        lock.push(IAmDevice {
            device_id,
            max_apdu,
            vendor_id,
            mac_addr,
            network_number,
            addr,
        });
    }
}

// TODO(tj): Handle duplicates. A duplicate is pretty much a device ID we've already seen, from
// what I understand.
fn whois(timeout: Duration, subnet: Option<u16>) {
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
        println!("ok here");
        Device_Set_Object_Instance_Number(BACNET_MAX_INSTANCE);
        // service handlers
        Device_Init(std::ptr::null_mut());
        apdu_set_unrecognized_service_handler_handler(None);
        apdu_set_confirmed_handler(
            BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_READ_PROPERTY,
            Some(handler_read_property),
        );
        apdu_set_unconfirmed_handler(
            BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_I_AM,
            Some(i_am_handler),
        );

        // FIXME(tj): Set error handlers
        // apdu_set_abort_handler(MyAbortHandler);
        // apdu_set_reject_handler(MyRejectHandler);
        address_init();
        dlenv_init();
        println!("ok here too");
    }

    let mut src = BACNET_ADDRESS::default();
    let mut rx_buf = [0u8; MAX_MPDU as usize];
    let bip_timeout = 100; // ms
    unsafe {
        Send_WhoIs_To_Network(
            &mut dest as *mut _,
            target_object_instance_min,
            target_object_instance_max,
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
