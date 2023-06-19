use bacnet_sys::*;
use std::{env, time::Instant};

fn main() {
    let mut src = BACNET_ADDRESS::default();
    let mut target_addr = BACNET_ADDRESS::default();

    let mut args: Vec<_> = env::args().collect();
    let progname = args.remove(0);

    if args.len() < 4 {
        println!(
            "usage: {} <device-instance> <object-type> <object-instance> <property>",
            progname
        );
        std::process::exit(0);
    }

    let device_instance: u32 = args[0].parse().unwrap();
    let object_type: BACNET_OBJECT_TYPE = if let Ok(t) = args[1].parse() {
        t
    } else {
        let mut found_index = 0;
        if unsafe {
            bactext_object_type_strtol(args[1].as_ptr() as *const _, &mut found_index as *mut _)
        } {
            found_index
        } else {
            panic!("Unable to parse '{}' as a known object-type", args[1]);
        }
    };
    let object_instance: u32 = args[2].parse().unwrap();
    let object_property: BACNET_PROPERTY_ID = if let Ok(t) = args[3].parse() {
        t
    } else {
        let mut found_index = 0;
        if unsafe {
            bactext_property_strtol(args[3].as_ptr() as *const _, &mut found_index as *mut _)
        } {
            found_index
        } else {
            panic!("Unable to parse '{}' as a known object-property", args[3]);
        }
    };

    println!(
        "device-instance = {} object-type = {} object-instance = {} property = {}",
        device_instance, object_type, object_instance, object_property
    );

    unsafe {
        address_init();
    }
    unsafe {
        Device_Set_Object_Instance_Number(BACNET_MAX_INSTANCE);
        init_service_handlers();
        dlenv_init();
    }

    // Try to bind with the device
    let mut max_apdu = 0;
    let mut found =
        unsafe { address_bind_request(device_instance, &mut max_apdu, &mut target_addr) };
    if !found {
        unsafe {
            Send_WhoIs(device_instance as i32, device_instance as i32);
        }
    }

    const TIMEOUT: u32 = 100;
    let mut rx_buf = [0u8; MAX_MPDU as usize];
    let start = Instant::now();
    let mut request_invoke_id = 0;
    let object_index = BACNET_ARRAY_ALL;
    loop {
        if !found {
            found =
                unsafe { address_bind_request(device_instance, &mut max_apdu, &mut target_addr) };
        }

        if found {
            if request_invoke_id == 0 {
                request_invoke_id = unsafe {
                    Send_Read_Property_Request(
                        device_instance,
                        object_type,
                        object_instance,
                        object_property,
                        object_index,
                    )
                }
            } else if unsafe { tsm_invoke_id_free(request_invoke_id) } {
                break;
            } else if unsafe { tsm_invoke_id_failed(request_invoke_id) } {
                // maybe this is how
                eprintln!("TSM timeout!");
                unsafe {
                    tsm_free_invoke_id(request_invoke_id);
                    break;
                }
            }
        } else if start.elapsed().as_secs() > 3 {
            eprintln!("APDU timeout!");
            break;
        }

        let pdu_len = unsafe {
            bip_receive(
                &mut src as *mut _,
                &mut rx_buf as *mut _,
                MAX_MPDU as u16,
                TIMEOUT,
            )
        };
        if pdu_len > 0 {
            unsafe {
                npdu_handler(&mut src as *mut _, &mut rx_buf as *mut _, pdu_len);
            }
        }
    }

    // At exit
    unsafe {
        bip_cleanup();
    }
}

#[no_mangle]
extern "C" fn my_readprop_ack_handler(
    service_request: *mut u8,
    service_len: u16,
    _: *mut BACNET_ADDRESS,
    _: *mut BACNET_CONFIRMED_SERVICE_ACK_DATA,
) {
    let mut data: BACNET_READ_PROPERTY_DATA = BACNET_READ_PROPERTY_DATA::default();

    let len = unsafe {
        rp_ack_decode_service_request(service_request, service_len.into(), &mut data as *mut _)
    };
    if len >= 0 {
        unsafe {
            rp_ack_print_data(&mut data);
        }
    } else {
        println!("<decode failed>");
    }
}

#[no_mangle]
extern "C" fn my_error_handler(
    _: *mut BACNET_ADDRESS,
    _: u8,
    error_class: BACNET_ERROR_CLASS,
    error_code: BACNET_ERROR_CODE,
) {
    let error_class_str =
        unsafe { std::ffi::CStr::from_ptr(bactext_error_class_name(error_class)) }
            .to_string_lossy()
            .into_owned();
    let error_code_str = unsafe { std::ffi::CStr::from_ptr(bactext_error_code_name(error_code)) }
        .to_string_lossy()
        .into_owned();
    println!(
        "BACnet error: error_class={} ({}) error_code={} ({})",
        error_class, error_class_str, error_code, error_code_str,
    );
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
