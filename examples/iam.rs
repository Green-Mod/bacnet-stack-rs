use std::ffi::CStr;

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
static mut SHOULD_SEND_I_AM: bool = false;

const TARGET_IP_ADDRESS: &str = "192.168.200.101:47809";
const LOCAL_PORT: u16 = 47810;
const INSTANCE_NUMBER: u32 = 1234;
const DNET: u16 = 0 as u16;

fn main() {
    unsafe {
        address_init();
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
            Some(my_handler_who_is),
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
            BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_I_AM,
            Some(handler_i_am_add),
        );
        /* handle any errors coming back */
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

        Send_I_Am_To_Network(
            &mut dest,
            9,
            480,
            BACNET_SEGMENTATION_SEGMENTATION_NONE as i32,
            BACNET_VENDOR_ID as u16,
        );

        loop {
            let pdu_len = bip_receive(&mut src, &mut rx_buf[0], MAX_MPDU as u16, 1000);

            if pdu_len > 0 {
                // apdu_handler(&mut src, &mut rx_buf[0], pdu_len);
                npdu_handler(&mut src, &mut rx_buf[0], pdu_len);
            }

            if ERROR_DETECTED {
                break;
            }

            if SHOULD_SEND_I_AM {
                SHOULD_SEND_I_AM = false;
                Send_I_Am_To_Network(
                    &mut src,
                    9,
                    480,
                    BACNET_SEGMENTATION_SEGMENTATION_NONE as i32,
                    BACNET_VENDOR_ID as u16,
                );
            }
        }
    }
}

extern "C" fn my_handler_who_is(
    service_request: *mut u8,
    service_len: u16,
    src: *mut BACNET_ADDRESS,
) {
    unsafe {
        handler_who_is(service_request, service_len, src);

        SHOULD_SEND_I_AM = true;
    }
}

extern "C" fn my_abort_handler(src: *mut BACNET_ADDRESS, invoke_id: u8, abort_reason: u8, _: bool) {
    unsafe {
        if bacnet_address_same(&mut TARGET_ADDRESS, src) && (invoke_id == REQUEST_INVOKE_ID) {
            println!(
                "BACnet Abort: {:?}",
                CStr::from_ptr(bactext_abort_reason_name(abort_reason as u32))
                    .to_str()
                    .unwrap(),
            );
            ERROR_DETECTED = true;
        }
    }
}

extern "C" fn my_reject_handler(src: *mut BACNET_ADDRESS, invoke_id: u8, reject_reason: u8) {
    unsafe {
        if bacnet_address_same(&mut TARGET_ADDRESS, src) && (invoke_id == REQUEST_INVOKE_ID) {
            println!(
                "BACnet Reject: {}",
                CStr::from_ptr(bactext_reject_reason_name(reject_reason as u32))
                    .to_str()
                    .unwrap(),
            );
            ERROR_DETECTED = true;
        }
    }
}
