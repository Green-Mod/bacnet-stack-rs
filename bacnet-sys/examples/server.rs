use bacnet_sys::*;

const LOCAL_PORT: u16 = 47808;
const INSTANCE_NUMBER: u32 = 1231;

fn main() {
    unsafe {
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

        /* we need to handle who-is to support dynamic device binding */
        apdu_set_unconfirmed_handler(
            BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_WHO_IS,
            Some(handler_who_is),
        );
        apdu_set_unconfirmed_handler(
            BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_WHO_HAS,
            Some(handler_who_has),
        );

        /* set the handler for all the services we don't implement */
        /* It is required to send the proper reject message... */
        apdu_set_unrecognized_service_handler_handler(Some(handler_unrecognized_service));
        /* Set the handlers for any confirmed services that we support. */
        /* We must implement read property - it's required! */
        apdu_set_confirmed_handler(
            BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_READ_PROPERTY,
            Some(handler_read_property),
        );
        apdu_set_confirmed_handler(
            BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_READ_PROP_MULTIPLE,
            Some(handler_read_property_multiple),
        );
        apdu_set_confirmed_handler(
            BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_WRITE_PROPERTY,
            Some(handler_write_property),
        );
        apdu_set_confirmed_handler(
            BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_WRITE_PROP_MULTIPLE,
            Some(handler_write_property_multiple),
        );
        apdu_set_confirmed_handler(
            BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_READ_RANGE,
            Some(handler_read_range),
        );
        apdu_set_confirmed_handler(
            BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_REINITIALIZE_DEVICE,
            Some(handler_reinitialize_device),
        );
        apdu_set_unconfirmed_handler(
            BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_UTC_TIME_SYNCHRONIZATION,
            Some(handler_timesync_utc),
        );
        apdu_set_unconfirmed_handler(
            BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_TIME_SYNCHRONIZATION,
            Some(handler_timesync),
        );
        apdu_set_confirmed_handler(
            BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_SUBSCRIBE_COV,
            Some(handler_cov_subscribe),
        );
        apdu_set_unconfirmed_handler(
            BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_COV_NOTIFICATION,
            Some(handler_ucov_notification),
        );
        /* handle communication so we can shutup when asked */
        apdu_set_confirmed_handler(
            BACnet_Confirmed_Service_Choice_SERVICE_CONFIRMED_DEVICE_COMMUNICATION_CONTROL,
            Some(handler_device_communication_control),
        );
        /* handle the data coming back from private requests */
        apdu_set_unconfirmed_handler(
            BACnet_Unconfirmed_Service_Choice_SERVICE_UNCONFIRMED_PRIVATE_TRANSFER,
            Some(handler_unconfirmed_private_transfer),
        );

        bip_set_port(LOCAL_PORT);
        println!("Running on port {}", bip_get_port());
        address_init();
        dlenv_init();

        Send_I_Am(&mut Handler_Transmit_Buffer[0]);

        let mut rx_buf: [u8; MAX_MPDU as usize] = [0; MAX_MPDU as usize];

        let mut src: BACNET_ADDRESS = BACNET_ADDRESS {
            adr: [0; 7],
            mac_len: 0,
            len: 0,
            mac: [0; 7],
            net: 0,
        };

        loop {
            let pdu_len = bip_receive(&mut src, &mut rx_buf[0], MAX_MPDU as u16, 1000);

            if pdu_len > 0 {
                println!("Received {:?} bytes", rx_buf);
                // apdu_handler(&mut src, &mut rx_buf[0], pdu_len);
                npdu_handler(&mut src, &mut rx_buf[0], pdu_len);
            }

            handler_cov_task();
        }
    }
}
