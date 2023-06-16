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
    let mut target_object_type = 0;
    unsafe {
        bactext_object_type_strtol("analog-input".as_ptr() as *const _, &mut target_object_type)
    };
    let target_object_instance = BACNET_MAX_INSTANCE;

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

        /* we need to handle who-is
        to support dynamic device binding to us */
        apdu_set_unconfirmed_handler(
            BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_WHO_IS,
            Some(handler_who_is),
        );
        /* set the handler for all the services we don't implement
        It is required to send the proper reject message... */
        apdu_set_unrecognized_service_handler_handler(Some(handler_unrecognized_service));
        /* we must implement read property - it's required! */
        apdu_set_confirmed_handler(
            BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_READ_PROPERTY,
            Some(handler_read_property),
        );
        /* handle the reply (request) coming back */
        apdu_set_unconfirmed_handler(
            BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_I_HAVE,
            Some(handler_i_have),
        );
        /* handle any errors coming back */
        apdu_set_abort_handler(Some(my_abort_handler));
        apdu_set_reject_handler(Some(my_reject_handler));
    }

    unsafe {
        address_init();
    }
    unsafe {
        dlenv_init();
    }

    let mut src = BACNET_ADDRESS::default();
    let mut rx_buf = [0u8; MAX_MPDU as usize];
    let timeout = 100; // ms
    unsafe {
        Send_WhoHas_Object(
            target_object_instance_min,
            target_object_instance_max,
            target_object_type,
            target_object_instance,
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
extern "C" fn my_abort_handler(_: *mut BACNET_ADDRESS, invoke_id: u8, abort_reason: u8, _: bool) {
    println!(
        "aborted invoke_id = {} abort_reason = {}",
        invoke_id, abort_reason
    );
}

#[no_mangle]
extern "C" fn my_reject_handler(_: *mut BACNET_ADDRESS, invoke_id: u8, reject_reason: u8) {
    println!(
        "rejected invoke_id = {} reject_reason = {}",
        invoke_id, reject_reason
    );
}
