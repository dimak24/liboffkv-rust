rsoffkv
=======
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Travis CI](http://badges.herokuapp.com/travis/offscale/rsoffkv?branch=master&label=OSX&env=BADGE=osx&style=flat-square)](https://travis-ci.org/offscale/rsoffkv)
[![Travis CI](http://badges.herokuapp.com/travis/offscale/rsoffkv?branch=master&label=Linux&env=BADGE=linux&style=flat-square)](https://travis-ci.org/offscale/rsoffkv)
[![codecov](https://codecov.io/gh/offscale/rsoffkv/graphs/badge.svg)](https://codecov.io/gh/offscale/rsoffkv)
[![API](https://docs.rs/rsoffkv/badge.svg)](https://docs.rs/rsoffkv)
[![Crate](https://img.shields.io/crates/v/rsoffkv.svg)](https://crates.io/crates/rsoffkvv)

#### This library is designed to provide a uniform interface for 3 different distributed KV-storages: etcd, Zookeeper, Consul.

Rsoffkv is a wrapper around _our_ C++ library [liboffkv](https://github.com/offscale/liboffkv).
Design details can be found in the liboffkv repository.

## Build

- install [vcpkg](https://github.com/microsoft/vcpkg) and set `VCPKG_ROOT`
- install dependencies (you can build build rsoffkv only for some of the supported KV-storages;
in such a case feel free to change the value of `ENABLE_` in the build script)
```sh
vcpkg install ppconsul offscale-libetcd-cpp zkpp
```
- build with cargo
```sh
cargo build
```
- (optional) run documentation tests
```sh
cargo test
```

## Example
```rust
use rsoffkv::client::Client;
use rsoffkv::result::OffkvError;

use rsoffkv::txn::{Transaction, TxnCheck, TxnOp, TxnOpResult};

use std::{thread,time};

fn main() {
    // firstly specify service {zk | consul | etcd} and its address
    // you can also specify a prefix all keys will start with
    let client = Client::new("consul://localhost:8500", "/prefix").unwrap();

    // Each method returns std::Result
    match client.create("/key", "value", false) {
        Ok(initial_version) =>
            println!("Key \"/prefix/key\" successfully created. Initial version: {}",
                     initial_version),
        Err(OffkvError::EntryExists) =>
            println!("Error: key \"/prefix/key\" already exists!"),
    };

    // WATCH EXAMPLE
    let (result, watch_handle) = client.exists("/key", true).unwrap();

    thread::spawn(|| {
        let another_client = Client::new("consul://localhost:8500", "/prefix", false).unwrap();
        thread::sleep(time::Duration::from_secs(5));
        another_client.erase("/key", 0).unwrap();
    });

    // now the key exists
    assert!(result);

    // wait for changes
    watch_handle.wait();

    // if the waiting was completed, the existence state must be different
    let (result, _) = client.exists("/key", false).unwrap();
    assert!(!result);

    // TRANSACTION EXAMPLE
    match client.commit(
        // firstly list your checks
        checks: vec![
            TxnCheck{key: "/key", version: initial_version},
        ],
        // then operations
        ops: vec![
            TxnOp::Create{key: "/key/child", value: "value", leased: false},
            TxnOp::Set{key: "/key", value: "new value"},
        ],
    ) {
        // on success a vector with changed version is returned
        Ok(_) => println!("Success!"),
        // on failure an index of the first failed operation is returned
        Err(OffkvError::TxnFailed(failed_op)) => println!("Failed at {}", failed_op),
    };
}
```
