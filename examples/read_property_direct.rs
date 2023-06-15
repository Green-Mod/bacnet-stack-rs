use bacnet_stack_rs::*;

static mut ERROR_DETECTED: bool = false;

const TARGET_IP_ADDRESS: &str = "192.168.200.101:47808";
const LOCAL_PORT: u16 = 47807;
const INSTANCE_NUMBER: u32 = 1232;
const DNET: u16 = 0 as u16;

const OBJECT_TYPE: BACNET_OBJECT_TYPE = BACnetObjectType_OBJECT_ANALOG_INPUT;
const OBJECT_INSTANCE: u32 = 0;

fn main() {
    unsafe {
        let mut dest: BACNET_ADDRESS = BACNET_ADDRESS {
            adr: [0; 7],
            mac_len: 0,
            len: 0,
            mac: [0; 7],
            net: 0,
        };

        let mut adr: BACNET_MAC_ADDRESS = BACNET_MAC_ADDRESS {
            adr: [0; 7],
            len: 0,
        };
        bacnet_address_mac_from_ascii(&mut adr, TARGET_IP_ADDRESS.as_ptr() as *const i8);

        let mut mac: BACNET_MAC_ADDRESS = BACNET_MAC_ADDRESS {
            adr: [0; 7],
            len: 0,
        };
        bacnet_address_mac_from_ascii(&mut mac, TARGET_IP_ADDRESS.as_ptr() as *const i8);

        bacnet_address_init(&mut dest, &mut mac, DNET, &mut adr);

        dest.adr = adr.adr;
        dest.len = adr.len;
        dest.net = DNET;

        // bip_get_broadcast_address(&mut dest);

        println!("{:?}", dest);

        // address_add(TARGET_INSTANCE_NUMBER, MAX_APDU, &mut dest);

        Device_Set_Object_Instance_Number(INSTANCE_NUMBER);
        println!("BACnet Device ID: {}", Device_Object_Instance_Number());

        Device_Init(&mut object_functions {
            Object_Type: BACNET_OBJECT_TYPE::BITS,
            Object_Init: None,
            Object_Count: None,
            Object_Index_To_Instance: None,
            Object_Valid_Instance: None,
            Object_Name: None,
            Object_Read_Property: None,
            Object_Write_Property: None,
            Object_RPM_List: None,
            Object_RR_Info: None,
            Object_Iterator: None,
            Object_Value_List: None,
            Object_COV: None,
            Object_COV_Clear: None,
            Object_Intrinsic_Reporting: None,
            Object_Add_List_Element: None,
            Object_Remove_List_Element: None,
        });

        /* we need to handle who-is
        to support dynamic device binding to us */
        apdu_set_unconfirmed_handler(
            BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_WHO_IS,
            Some(handler_who_is),
        );
        /* handle i-am to support binding to other devices */
        apdu_set_unconfirmed_handler(
            BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_I_AM,
            Some(handler_i_am_bind),
        );
        /* set the handler for all the services we don't implement
        It is required to send the proper reject message... */
        apdu_set_unrecognized_service_handler_handler(Some(handler_unrecognized_service));
        /* we must implement read property - it's required! */
        apdu_set_confirmed_handler(
            BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_READ_PROPERTY,
            Some(handler_read_property),
        );
        // apdu_set_confirmed_handler(
        //     BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_READ_PROPERTY,
        //     Some(my_read_property_handler),
        // );
        /* handle the data coming back from confirmed requests */
        apdu_set_confirmed_ack_handler(
            BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_READ_PROPERTY,
            Some(my_read_property_ack_handler),
        );
        /* handle any errors coming back */
        apdu_set_error_handler(
            BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_READ_PROPERTY,
            Some(my_error_handler),
        );
        apdu_set_abort_handler(Some(my_abort_handler));
        apdu_set_reject_handler(Some(my_reject_handler));

        bip_set_port(LOCAL_PORT);
        println!("Running on port {}", bip_get_port());
        address_init();
        dlenv_init();

        let mut rx_buf: [u8; MAX_MPDU as usize] = [0; MAX_MPDU as usize];

        let mut src: BACNET_ADDRESS = BACNET_ADDRESS {
            adr: [0; 7],
            mac_len: 0,
            len: 0,
            mac: [0; 7],
            net: 0,
        };

        Send_Read_Property_Request_Address(
            &mut dest,
            MAX_APDU as u16,
            OBJECT_TYPE,
            OBJECT_INSTANCE,
            BACNET_PROPERTY_ID_PROP_PRESENT_VALUE,
            BACNET_ARRAY_ALL,
        );

        loop {
            if ERROR_DETECTED {
                break;
            }

            let pdu_len = bip_receive(&mut src, &mut rx_buf[0], MAX_MPDU as u16, 1000);

            if pdu_len > 0 {
                println!("rx_buf: {:?}", rx_buf);
                npdu_handler(&mut src, &mut rx_buf[0], pdu_len);
            }
        }
    }
}

extern "C" fn my_error_handler(
    _: *mut BACNET_ADDRESS,
    _: u8,
    error_class: BACNET_ERROR_CLASS,
    error_code: BACNET_ERROR_CODE,
) {
    unsafe {
        println!(
            "BACnet Error: {:?}: {:?}",
            bactext_error_class_name(error_class),
            bactext_error_code_name(error_code)
        );
        ERROR_DETECTED = true;
    }
}

extern "C" fn my_abort_handler(_: *mut BACNET_ADDRESS, _: u8, abort_reason: u8, _: bool) {
    unsafe {
        println!(
            "BACnet Abort: {:?}",
            bactext_abort_reason_name(abort_reason as u32),
        );
        ERROR_DETECTED = true;
    }
}

extern "C" fn my_reject_handler(_: *mut BACNET_ADDRESS, _: u8, reject_reason: u8) {
    unsafe {
        println!(
            "BACnet Reject: {:?}",
            bactext_reject_reason_name(reject_reason as u32),
        );
        ERROR_DETECTED = true;
    }
}

extern "C" fn my_read_property_ack_handler(
    service_request: *mut u8,
    service_len: u16,
    _: *mut BACNET_ADDRESS,
    _: *mut BACNET_CONFIRMED_SERVICE_ACK_DATA,
) {
    let mut data: BACNET_READ_PROPERTY_DATA = BACNET_READ_PROPERTY_DATA {
        application_data: &mut 0,
        application_data_len: 0,
        array_index: 0,
        error_class: 0,
        error_code: 0,
        object_instance: 0,
        object_property: 0,
        object_type: 0,
    };

    unsafe {
        println!("Received ReadProperty Ack!");
        let len = rp_ack_decode_service_request(service_request, service_len as i32, &mut data);
        if len < 0 {
            println!("<decode failed!>");
        } else {
            rp_ack_print_data(&mut data);
            println!("Data: {:?}", data);
            println!("Application data: {:?}", *data.application_data);
        }
    }
}

// extern "C" fn my_read_property_handler(
//     service_request: *mut u8,
//     service_len: u16,
//     src: *mut BACNET_ADDRESS,
//     service_data: *mut BACNET_CONFIRMED_SERVICE_DATA,
// ) {
//     let mut data: BACNET_READ_PROPERTY_DATA = BACNET_READ_PROPERTY_DATA {
//         application_data: &mut 0,
//         application_data_len: 0,
//         array_index: 0,
//         error_class: 0,
//         error_code: 0,
//         object_instance: 0,
//         object_property: 0,
//         object_type: 0,
//     };

//     unsafe {
//         handler_read_property(service_request, service_len, src, service_data);

//         println!("Received ReadProperty!");
//         let len = rp_decode_service_request(service_request, service_len as u32, &mut data);
//         if len < 0 {
//             println!("<decode failed!>");
//         } else {
//             rp_ack_print_data(&mut data);
//             println!("Data: {:?}", data);
//             println!("Application data: {:?}", *data.application_data);
//         }
//     }
// }
