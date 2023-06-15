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

struct AddressEntry {
    flags: u8,
    device_id: u32,
    max_apdu: u32,
    address: BACNET_ADDRESS,
}

static mut ADDRESS_TABLE: Vec<AddressEntry> = Vec::new();

const BAC_ADDRESS_MULT: u8 = 1;
const TARGET_IP_ADDRESS: &str = "192.168.200.101:47810";
const LOCAL_PORT: u16 = 47809;
const INSTANCE_NUMBER: u32 = 1233;
const DNET: u16 = 0 as u16;

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

        apdu_set_unrecognized_service_handler_handler(Some(handler_unrecognized_service));
        /* we must implement read property - it's required! */
        apdu_set_confirmed_handler(
            BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_READ_PROPERTY,
            Some(handler_read_property),
        );
        /* handle the reply (request) coming back */
        apdu_set_unconfirmed_handler(
            BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_I_AM,
            Some(my_i_am_handler),
        );
        /* handle any errors coming back */
        apdu_set_abort_handler(Some(my_abort_handler));
        apdu_set_reject_handler(Some(my_reject_handler));

        bip_set_port(LOCAL_PORT);
        println!("Running on port {}", bip_get_port());
        address_init();
        dlenv_init();

        // Broadcasted Who Is request only works for services on the same port
        // Send_WhoIs(1, BACNET_MAX_INSTANCE as i32);

        Send_WhoIs_To_Network(&mut dest, 1, BACNET_MAX_INSTANCE as i32);

        // This should broadcast but doesn't seem to work
        // Send_WhoIs(1, BACNET_MAX_INSTANCE as i32);

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

            let pdu_len = bip_receive(&mut src, &mut rx_buf[0], MAX_MPDU as u16, 1000);

            if pdu_len > 0 {
                // apdu_handler(&mut src, &mut rx_buf[0], pdu_len);
                npdu_handler(&mut src, &mut rx_buf[0], pdu_len);
            }
        }

        for address in ADDRESS_TABLE.iter() {
            if address.flags & BAC_ADDRESS_MULT > 0 {
                println!(";");
            } else {
                println!(" ");
            }
            println!(
                "{} {:?} {} {}",
                address.device_id, address.address.mac, address.address.net, address.max_apdu
            );
        }
    }
}

fn address_table_add(device_id: u32, max_apdu: u32, src: *mut BACNET_ADDRESS) {
    let mut flags: u8 = 0;

    unsafe {
        for address in ADDRESS_TABLE.iter_mut() {
            if address.device_id == device_id {
                if bacnet_address_same(&mut address.address, src) {
                    return;
                }
                flags |= BAC_ADDRESS_MULT;
                address.flags |= BAC_ADDRESS_MULT;
            }
        }

        ADDRESS_TABLE.push(AddressEntry {
            flags,
            device_id,
            max_apdu,
            address: *src,
        });
    }

    return;
}

extern "C" fn my_i_am_handler(service_request: *mut u8, _: u16, src: *mut BACNET_ADDRESS) {
    unsafe {
        let mut device_id: u32 = 0;
        let mut max_apdu = 0;
        let mut segmentation = 0;
        let mut vendor_id: u16 = 0;

        print!("Received I-Am Request");
        let len = iam_decode_service_request(
            service_request,
            &mut device_id,
            &mut max_apdu,
            &mut segmentation,
            &mut vendor_id,
        );
        if len != -1 {
            print!(" from {}, MAC = ", device_id);
            if ((*src).mac_len == 6) && ((*src).len == 0) {
                print!(
                    "{}.{}.{}.{} {}{}\n",
                    (*src).mac[0],
                    (*src).mac[1],
                    (*src).mac[2],
                    (*src).mac[3],
                    (*src).mac[4],
                    (*src).mac[5]
                );
            } else {
                for i in 0..(*src).mac_len {
                    print!("{}", (*src).mac[i as usize]);
                    if i < ((*src).mac_len - 1) {
                        print!(":");
                    }
                }
                println!();
            }
            address_table_add(device_id, max_apdu, src);
        } else {
            println!(", but unable to decode it.\n");
        }
    }

    return;
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
