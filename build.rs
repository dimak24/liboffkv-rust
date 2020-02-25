extern crate cmake;

use std::path::PathBuf;


fn main() {
    let mut toolchain = PathBuf::from(std::env::var("VCPKG_ROOT").unwrap());
    toolchain.push("scripts");
    toolchain.push("buildsystems");
    toolchain.push("vcpkg.cmake");

    let mut dst = cmake::Config::new("liboffkv")
        .define("BUILD_TESTS", "OFF")
        .define("ENABLE_ZK", "ON")
        .define("ENABLE_CONSUL", "ON")
        .define("ENABLE_ETCD", "ON")
        .define("BUILD_CLIB", "ON")
        .define("CMAKE_TOOLCHAIN_FILE", toolchain.as_os_str())
        .build();

    dst.push("build");

    println!("cargo:rustc-link-search=all={}", dst.display());
    println!("cargo:rustc-link-lib=dylib=liboffkv_c");
}
