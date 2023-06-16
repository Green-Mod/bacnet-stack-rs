use bacnet_sys::*;
use std::ffi::CStr;

const INSTANCE_NUMBER: u32 = 1234;

fn main() {
    unsafe {
        println!(
            "bacnet stack v{}",
            String::from_utf8_lossy(BACNET_VERSION_TEXT)
        );

        let mut dest = BACNET_ADDRESS::default();

        let a = BACNET_BROADCAST_NETWORK;
        println!("BACNET_BROADCAST_NETWORK={}", a);

        bip_get_broadcast_address(&mut dest as *mut _);

        Device_Set_Object_Instance_Number(INSTANCE_NUMBER);
        println!("BACnet Device ID: {}", Device_Object_Instance_Number());

        Device_Init(&mut object_functions::default());

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
            BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_I_AM,
            Some(handler_i_am_add),
        );
        /* handle any errors coming back */
        apdu_set_abort_handler(Some(my_abort_handler));
        apdu_set_reject_handler(Some(my_reject_handler));

        address_init();
        dlenv_init();

        // Broadcasted I Am request only works for services on the same port
        Send_I_Am(&mut Handler_Transmit_Buffer[0]);

        let mut rx_buf: [u8; MAX_MPDU as usize] = [0; MAX_MPDU as usize];

        let mut src: BACNET_ADDRESS = BACNET_ADDRESS::default();

        loop {
            let pdu_len = bip_receive(&mut src, &mut rx_buf[0], MAX_MPDU as u16, 1000);

            if pdu_len > 0 {
                println!(
                    "Received PDU of {} bytes: {:?}",
                    pdu_len,
                    &rx_buf[..pdu_len as usize]
                );
                // apdu_handler(&mut src, &mut rx_buf[0], pdu_len);
                npdu_handler(&mut src, &mut rx_buf[0], pdu_len);
            }
        }
    }
}

#[no_mangle]
extern "C" fn my_abort_handler(_: *mut BACNET_ADDRESS, _: u8, abort_reason: u8, _: bool) {
    unsafe {
        println!(
            "BACnet Abort: {:?}",
            CStr::from_ptr(bactext_abort_reason_name(abort_reason as u32))
                .to_str()
                .unwrap(),
        );
    }
}

#[no_mangle]
extern "C" fn my_reject_handler(_: *mut BACNET_ADDRESS, _: u8, reject_reason: u8) {
    unsafe {
        println!(
            "BACnet Reject: {}",
            CStr::from_ptr(bactext_reject_reason_name(reject_reason as u32))
                .to_str()
                .unwrap(),
        );
    }
}
