/// A Rust transliteration of the bacnet-stack whois app.
use std::time::Instant;

use bacnet_sys::*;

fn main() {
    println!(
        "bacnet stack v{}",
        String::from_utf8_lossy(BACNET_VERSION_TEXT)
    );

    let mut dest = BACNET_ADDRESS::default();
    let target_object_instance_min = -1i32;
    let target_object_instance_max = -1i32;

    let a = BACNET_BROADCAST_NETWORK;
    println!("BACNET_BROADCAST_NETWORK={}", a);

    unsafe {
        bip_get_broadcast_address(&mut dest as *mut _);
    }

    // Device_Set_Object_Instance_Number(BACNET_MAX_INSTANCE);
    unsafe {
        Device_Set_Object_Instance_Number(BACNET_MAX_INSTANCE);
    }

    // init_service_handlers()
    unsafe {
        Device_Init(std::ptr::null_mut());
        apdu_set_unrecognized_service_handler_handler(None);
        apdu_set_confirmed_handler(
            BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_READ_PROPERTY,
            Some(handler_read_property),
        );
        apdu_set_unconfirmed_handler(
            BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_I_AM,
            Some(my_i_am_handler),
        );

        //apdu_set_abort_handler(
    }
    //
    //
    unsafe {
        address_init();
    }
    //
    unsafe {
        dlenv_init();
    }

    let mut src = BACNET_ADDRESS::default();
    let mut rx_buf = [0u8; MAX_MPDU as usize];
    let timeout = 100; // ms
    unsafe {
        Send_WhoIs_To_Network(
            &mut dest as *mut _,
            target_object_instance_min,
            target_object_instance_max,
        );
    }
    let start = Instant::now();
    let mut i = 0;
    loop {
        let pdu_len = unsafe {
            bip_receive(
                &mut src as *mut _,
                &mut rx_buf as *mut _,
                MAX_MPDU as u16,
                timeout,
            )
        };
        if pdu_len > 0 {
            // process
            unsafe {
                npdu_handler(&mut src as *mut _, &mut rx_buf as *mut _, pdu_len);
            }
        }

        if start.elapsed().as_secs() > 3 {
            break;
        }
        i += 1;
    }
    println!("Looped {} times", i);

    // atexit(ethernet_cleanup());
    unsafe {
        bip_cleanup();
    }
}

#[no_mangle]
extern "C" fn my_i_am_handler(service_request: *mut u8, _: u16, src: *mut BACNET_ADDRESS) {
    let mut device_id = 0;
    let mut max_apdu = 0;
    let mut segmentation = 0;
    let mut vendor_id = 0;
    let mut mac_addr = [0u8; 6];

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
        println!("unable to decode I-Am request");
        return;
    }
    println!(
        "device_id = {} max_apdu = {} vendor_id = {}",
        device_id, max_apdu, vendor_id
    );
    let mac_len = unsafe { (*src).mac_len } as usize;
    mac_addr[..mac_len].copy_from_slice(unsafe { &(*src).mac[..mac_len] });
    println!("MAC = {:02X?}", mac_addr);
}
