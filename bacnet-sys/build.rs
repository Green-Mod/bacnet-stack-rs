use std::env;
use std::path::PathBuf;

fn main() {
    let mut dir = cmake::Config::new("bacnet-stack")
        .define("BACNET_STACK_BUILD_APPS", "OFF")
        //.define("BAC_ROUTING", "OFF") // not sure what this implies
        .define("BACNET_BUILD_PIFACE_APP", "OFF")
        .define("BACNET_BUILD_PIFACE_APP", "OFF")
        .define("BACDL_BIP", "ON")
        .define("BACDL_ETHERNET", "OFF")
        .build();

    dir.push("build");
    // println!("cargo:warning={}", dir.display());

    println!("cargo:rustc-link-search=native={}", dir.display());
    println!("cargo:rustc-link-lib=static={}", "bacnet-stack"); // libbacnet-stack.a

    let bindings = bindgen::Builder::default()
        .clang_arg("-Ibacnet-stack/src")
        //.clang_arg("-I.")
        .header("wrapper.h")
        .derive_default(true)
        .derive_copy(true)
        .derive_debug(true)
        .derive_hash(true)
        .derive_partialeq(true)
        .derive_eq(true)
        .derive_ord(true)
        .derive_partialord(true)
        .derive_eq(true)
        .generate()
        .expect("unable to generate bindings");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out.join("bindings.rs"))
        .expect("couldn't write bindings");
}
