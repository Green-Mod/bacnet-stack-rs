use bacnet::whois::WhoIs;

fn main() {
    pretty_env_logger::init();
    let devices = WhoIs::new()
        .timeout(std::time::Duration::from_secs(1))
        .subnet(0)
        .execute()
        .unwrap();

    let ndevices = devices.len();
    println!("Device ID             MAC            SNET            SADR            APDU");
    println!("---------  ------------------------  ----  ------------------------  ----");
    for dev in devices {
        println!(
            "{:9}  {:02X?}  {:4}  {:02X?}  {:4}",
            dev.device_id, dev.mac_addr, dev.network_number, dev.addr, dev.max_apdu
        );
    }
    println!(
        "Total: {} device{}",
        ndevices,
        if ndevices == 1 { "" } else { "s" }
    );
}
