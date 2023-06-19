use bacnet::whohas::WhoHas;
use bacnet_sys::{bactext_object_type_name, BACnetObjectType_OBJECT_ANALOG_INPUT};
use std::ffi::{c_char, CStr};

fn cstr(ptr: *const c_char) -> String {
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

fn main() {
    pretty_env_logger::init();
    let i_have_data = WhoHas::new()
        .object_type(BACnetObjectType_OBJECT_ANALOG_INPUT)
        .object_instance(0)
        .timeout(std::time::Duration::from_secs(1))
        .subnet(0)
        .execute()
        .unwrap();

    let ndata = i_have_data.len();
    println!("Device ID         OBJECT_ID                OBJECT_NAME       ");
    println!("---------  ------------------------  ------------------------");
    for data in i_have_data {
        unsafe {
            println!(
                "{:9}  {}  {}",
                cstr(bactext_object_type_name(data.device_id.object_type)),
                cstr(bactext_object_type_name(data.object_id.object_type)),
                data.object_name,
            );
        }
    }
    println!("Total: {} data", ndata);
}
