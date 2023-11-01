use cmake::Config;

const USE_DOUBLE: bool = false;

fn main() {

    let dst = Config::new("libccd")
                .define("BUILD_SHARED_LIBS", "OFF")
                .define("ENABLE_DOUBLE_PRECISION", if USE_DOUBLE {"ON"} else {"OFF"})
                .build();

    if USE_DOUBLE {
        println!("cargo:rustc-cfg=use_double=\"yes\"");
    } else {
        println!("cargo:rustc-cfg=use_double=\"no\"");
    }
    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=static=ccd");
}