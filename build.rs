use cmake::Config;

fn main() {
    let dst = Config::new("libccd")
                .define("BUILD_SHARED_LIBS", "OFF")
                .define("ENABLE_DOUBLE_PRECISION", "ON")
                .build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=static=ccd");
}