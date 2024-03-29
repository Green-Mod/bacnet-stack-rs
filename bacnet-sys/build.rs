use std::{collections::HashSet, env, path::PathBuf};

#[derive(Debug)]
struct IgnoreMacros(HashSet<String>);

impl bindgen::callbacks::ParseCallbacks for IgnoreMacros {
    fn will_parse_macro(&self, name: &str) -> bindgen::callbacks::MacroParsingBehavior {
        if self.0.contains(name) {
            bindgen::callbacks::MacroParsingBehavior::Ignore
        } else {
            bindgen::callbacks::MacroParsingBehavior::Default
        }
    }
}

fn main() {
    let mut dir = cmake::Config::new("bacnet-stack")
        .define("BACNET_STACK_BUILD_APPS", "OFF")
        .define("BAC_ROUTING", "OFF") // not sure what this implies
        .define("BACNET_BUILD_PIFACE_APP", "OFF")
        .define("BACAPP_PRINT_ENABLED", "ON")
        .define("BACDL_BIP", "ON")
        .define("BACDL_ETHERNET", "OFF")
        .build();

    dir.push("build");
    // println!("cargo:warning={}", dir.display());

    println!("cargo:rustc-link-search=native={}", dir.display());
    println!("cargo:rustc-link-lib=static=bacnet-stack"); // libbacnet-stack.a

    let ignored_macros = IgnoreMacros(
        vec![
            "FP_INFINITE".into(),
            "FP_NAN".into(),
            "FP_NORMAL".into(),
            "FP_SUBNORMAL".into(),
            "FP_ZERO".into(),
            "IPPORT_RESERVED".into(),
            "M_E".into(),
            "M_LOG2E".into(),
            "M_LOG10E".into(),
            "M_LN2".into(),
            "M_LN10".into(),
            "M_PI".into(),
            "M_PI_2".into(),
            "M_PI_4".into(),
            "M_1_PI".into(),
            "M_2_PI".into(),
            "M_2_SQRTPI".into(),
            "M_SQRT2".into(),
            "M_SQRT1_2".into(),
        ]
        .into_iter()
        .collect(),
    );

    let bindings = bindgen::Builder::default()
        .clang_arg("-Ibacnet-stack/src")
        //.clang_arg("-I.")
        .header("wrapper.h")
        .parse_callbacks(Box::new(ignored_macros))
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
