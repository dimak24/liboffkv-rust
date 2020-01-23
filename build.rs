extern crate cmake;

fn main() {
    // TODO
    let mut toolchain = std::env::home_dir().unwrap();
    toolchain.push("vcpkg/scripts/buildsystems/vcpkg.cmake");

    // TODO: add option to build without installing liboffkv from submodule
    let mut dst = cmake::Config::new("liboffkv")
        .define("BUILD_TESTS", "OFF")
        .define("ENABLE_ZK", "OFF")
        .define("ENABLE_CONSUL", "ON")
        .define("ENABLE_ETCD", "OFF")
        .define("BUILD_CLIB", "ON")
        .define("CMAKE_TOOLCHAIN_FILE", toolchain.as_os_str())
        .build_target("")
        .build();

    dst.push("build");

    println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-link-lib=dylib=liboffkv_c");
}
