use bacnet_stack_rs::*;

static mut REQUEST_INVOKE_ID: u8 = 0;
static mut TARGET_ADDRESS: BACNET_ADDRESS = BACNET_ADDRESS {
    net: 0,
    adr: [0; 7],
    len: 0,
    mac_len: 0,
    mac: [0; 7],
};
static mut ERROR_DETECTED: bool = false;

const TARGET_IP_ADDRESS: &str = "192.168.200.101:47808";
const LOCAL_PORT: u16 = 47807;
const INSTANCE_NUMBER: u32 = 1232;
const DNET: u16 = BACNET_BROADCAST_NETWORK as u16;

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

        address_add(1000, MAX_APDU, &mut dest);

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

        let mut max_apdu: u32 = 0;

        let mut found = address_bind_request(1000, &mut max_apdu, &mut TARGET_ADDRESS);
        if !found {
            Send_WhoIs(1000, 1000);
        }

        let mut rx_buf: [u8; MAX_MPDU as usize] = [0; MAX_MPDU as usize];

        let mut src: BACNET_ADDRESS = BACNET_ADDRESS {
            adr: [0; 7],
            mac_len: 0,
            len: 0,
            mac: [0; 7],
            net: 0,
        };

        loop {
            if ERROR_DETECTED {
                break;
            }

            if !found {
                found = address_bind_request(1000, &mut max_apdu, &mut TARGET_ADDRESS);
            } else {
                if REQUEST_INVOKE_ID == 0 {
                    REQUEST_INVOKE_ID = Send_Read_Property_Request(
                        1000,
                        BACnetObjectType_OBJECT_ANALOG_OUTPUT,
                        101,
                        BACNET_PROPERTY_ID_PROP_PRESENT_VALUE,
                        BACNET_ARRAY_ALL,
                    );
                } else if tsm_invoke_id_free(REQUEST_INVOKE_ID) {
                    break;
                } else if tsm_invoke_id_failed(REQUEST_INVOKE_ID) {
                    println!("Error: TSM Timeout!");
                    tsm_free_invoke_id(REQUEST_INVOKE_ID);
                    ERROR_DETECTED = true;
                    break;
                }
            }

            let pdu_len = bip_receive(&mut src, &mut rx_buf[0], MAX_MPDU as u16, 1000);
            println!("pdu_len: {}", pdu_len);

            if pdu_len > 0 {
                println!("rx_buf: {:?}", rx_buf);
                npdu_handler(&mut src, &mut rx_buf[0], pdu_len);
            }
        }
    }
}

extern "C" fn my_error_handler(
    src: *mut BACNET_ADDRESS,
    invoke_id: u8,
    error_class: BACNET_ERROR_CLASS,
    error_code: BACNET_ERROR_CODE,
) {
    unsafe {
        if bacnet_address_same(&mut TARGET_ADDRESS, src) && (invoke_id == REQUEST_INVOKE_ID) {
            println!(
                "BACnet Error: {:?}: {:?}",
                bactext_error_class_name(error_class),
                bactext_error_code_name(error_code)
            );
            ERROR_DETECTED = true;
        }
    }
}

extern "C" fn my_abort_handler(src: *mut BACNET_ADDRESS, invoke_id: u8, abort_reason: u8, _: bool) {
    unsafe {
        if bacnet_address_same(&mut TARGET_ADDRESS, src) && (invoke_id == REQUEST_INVOKE_ID) {
            println!(
                "BACnet Abort: {:?}",
                bactext_abort_reason_name(abort_reason as u32),
            );
            ERROR_DETECTED = true;
        }
    }
}

extern "C" fn my_reject_handler(src: *mut BACNET_ADDRESS, invoke_id: u8, reject_reason: u8) {
    unsafe {
        if bacnet_address_same(&mut TARGET_ADDRESS, src) && (invoke_id == REQUEST_INVOKE_ID) {
            println!(
                "BACnet Reject: {:?}",
                bactext_reject_reason_name(reject_reason as u32),
            );
            ERROR_DETECTED = true;
        }
    }
}

/** Handler for a ReadProperty ACK.
 * @ingroup DSRP
 * Doesn't actually do anything, except, for debugging, to
 * print out the ACK data of a matching request.
 *
 * @param service_request [in] The contents of the service request.
 * @param service_len [in] The length of the service_request.
 * @param src [in] BACNET_ADDRESS of the source of the message
 * @param service_data [in] The BACNET_CONFIRMED_SERVICE_DATA information
 *                          decoded from the APDU header of this message.
 */
extern "C" fn my_read_property_ack_handler(
    service_request: *mut u8,
    service_len: u16,
    src: *mut BACNET_ADDRESS,
    service_data: *mut BACNET_CONFIRMED_SERVICE_ACK_DATA,
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
        if bacnet_address_same(&mut TARGET_ADDRESS, src)
            && ((*service_data).invoke_id == REQUEST_INVOKE_ID)
        {
            println!("Received ReadProperty Ack!");
            let len = rp_ack_decode_service_request(service_request, service_len as i32, &mut data);
            if len < 0 {
                println!("<decode failed!>");
            } else {
                rp_ack_print_data(&mut data);
            }
        }
    }
}
