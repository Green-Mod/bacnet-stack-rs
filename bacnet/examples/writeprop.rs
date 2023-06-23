extern crate bacnet;
extern crate structopt;

use bacnet::{value::BACnetValue, BACnetServer};
use bacnet_sys::{
    bactext_object_type_strtol, bactext_property_strtol, BACNET_OBJECT_TYPE, BACNET_PROPERTY_ID,
};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "writeprop")]
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

    #[structopt(short = "t", long, default_value = "analog-value", parse(try_from_str = parse_object_type))]
    object_type: BACNET_OBJECT_TYPE,
    #[structopt(short = "i", long, default_value = "22")]
    object_instance: u32,
    #[structopt(short = "v", long, default_value = "1", parse(from_str = parse_object_value))]
    object_value: BACnetValue,
    #[structopt(short = "p", long, default_value = "present-value", parse(try_from_str = parse_property))]
    property: u32,
    #[structopt(short = "I", long, default_value = "4294967295")]
    index: u32,
}

fn parse_object_type(src: &str) -> Result<BACNET_OBJECT_TYPE, String> {
    if let Ok(t) = src.parse() {
        Ok(t)
    } else {
        let mut found_index = 0;
        if unsafe {
            bactext_object_type_strtol(
                src.as_ptr() as *const ::std::os::raw::c_char,
                &mut found_index,
            )
        } {
            Ok(found_index)
        } else {
            Err(format!("Couldn't parse input '{}' as object-type", src))
        }
    }
}

fn parse_object_value(src: &str) -> BACnetValue {
    BACnetValue::from(src.to_string())
}

fn parse_property(src: &str) -> Result<BACNET_PROPERTY_ID, String> {
    if let Ok(t) = src.parse() {
        Ok(t)
    } else {
        let mut found_index = 0;
        if unsafe {
            bactext_property_strtol(
                src.as_ptr() as *const ::std::os::raw::c_char,
                &mut found_index,
            )
        } {
            Ok(found_index)
        } else {
            Err(format!("Couldn't parse input '{}' as property", src))
        }
    }
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
        Ok(()) => {
            let object_value = if let BACnetValue::Bool(v) = opt.object_value {
                BACnetValue::Enum(
                    if v { 1 } else { 0 },
                    Some((if v { "active" } else { "inactive" }).to_string()),
                )
            } else {
                opt.object_value
            };

            let r = server.write_prop_at(
                opt.object_type,
                opt.object_instance,
                object_value,
                opt.property,
                opt.index,
            );
            match r {
                Ok(_) => println!("result {:?}", r),
                Err(err) => eprintln!("failed to write property: {}", err),
            }
        }
        Err(err) => {
            eprintln!("failed to connect to device... {}", err);
        }
    }
}
