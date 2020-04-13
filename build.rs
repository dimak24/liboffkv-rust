extern crate cmake;

use std::path::PathBuf;
use std::process::Command;


fn main() {
    if cfg!(docs) { return; }

    let mut toolchain = PathBuf::from(
        std::env::var_os("VCPKG_ROOT")
            .expect("Please set VCPKG_ROOT before build"));

    toolchain.push("scripts");
    toolchain.push("buildsystems");
    toolchain.push("vcpkg.cmake");

    Command::new("git").arg("submodule")
        .arg("sync").arg("--recursive").status().unwrap();
    Command::new("git").arg("submodule")
        .arg("foreach")
        .arg("git").arg("reset").arg("--hard").status().unwrap();
    Command::new("git").arg("submodule")
        .arg("update").arg("--init").arg("--recursive").arg("--checkout")
        .status().unwrap();

    let mut dst = cmake::Config::new("liboffkv")
        .define("ENABLE_ZK", "ON")
        .define("ENABLE_CONSUL", "ON")
        .define("ENABLE_ETCD", "ON")
        .define("BUILD_CLIB", "ON")
        .define("BUILD_TESTS", "OFF")
        .define("CMAKE_TOOLCHAIN_FILE", toolchain.as_os_str())
        .build();

    dst.push("build");

    println!("cargo:rustc-link-search=all={}", dst.display());
    println!("cargo:rustc-link-lib=dylib=liboffkv_c");
}
