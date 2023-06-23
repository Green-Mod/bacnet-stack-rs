extern crate bacnet;
extern crate structopt;

use bacnet::BACnetServer;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "epics")]
struct Opt {
    #[structopt(long, default_value = "0")]
    device_id: u32,
    #[structopt(long, default_value = "192.168.10.96")]
    ip: std::net::Ipv4Addr,
    #[structopt(long, default_value = "0")]
    dnet: u16,
    #[structopt(long, default_value = "0")]
    dadr: u8,
    #[structopt(long, default_value = "47808")]
    port: u16,
}

fn main() {
    pretty_env_logger::init();
    let opt = Opt::from_args();
    let mut server = BACnetServer::builder()
        .device_id(opt.device_id)
        .ip(opt.ip)
        .dnet(opt.dnet)
        .dadr(opt.dadr)
        .port(opt.port)
        .build();

    match server.connect() {
        Ok(()) => match server.epics() {
            Ok(epics) => {
                println!("Got epics {:#?}", serde_json::to_string(&epics));
            }
            Err(err) => eprintln!("failed to read property: {}", err),
        },
        Err(err) => {
            eprintln!("failed to connect to device... {}", err);
        }
    }
}
