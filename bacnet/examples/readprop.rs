extern crate bacnet;
extern crate structopt;

use bacnet::BACnetDevice;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "readprop")]
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
    object_type: bacnet_sys::BACNET_OBJECT_TYPE,
    #[structopt(short = "i", long, default_value = "22")]
    object_instance: u32,
    #[structopt(short = "p", long, default_value = "present-value", parse(try_from_str = parse_property))]
    property: u32,
    #[structopt(short = "I", long, default_value = "4294967295")]
    index: u32,

    #[structopt(short = "n", long, default_value = "1")]
    number_of_reads: usize,
}

fn parse_object_type(src: &str) -> Result<bacnet_sys::BACNET_OBJECT_TYPE, String> {
    if let Ok(t) = src.parse() {
        Ok(t)
    } else {
        let mut found_index = 0;
        if unsafe {
            bacnet_sys::bactext_object_type_strtol(
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

fn parse_property(src: &str) -> Result<bacnet_sys::BACNET_PROPERTY_ID, String> {
    if let Ok(t) = src.parse() {
        Ok(t)
    } else {
        let mut found_index = 0;
        if unsafe {
            bacnet_sys::bactext_property_strtol(
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
    let mut dev = BACnetDevice::builder()
        .device_id(opt.device_id)
        .ip(opt.ip)
        .dnet(opt.dnet)
        .dadr(opt.dadr)
        .port(opt.port)
        .build();

    println!("{:?}", dev);
    match dev.connect() {
        Ok(()) => {
            for _ in 0..opt.number_of_reads {
                let r = dev.read_prop_at(
                    opt.object_type,
                    opt.object_instance,
                    opt.property,
                    opt.index,
                );
                match r {
                    Ok(_) => println!("result {:?}", r),
                    Err(err) => eprintln!("failed to read property: {}", err),
                }
            }
        }
        Err(err) => {
            eprintln!("failed to connect to device... {}", err);
        }
    }
}
