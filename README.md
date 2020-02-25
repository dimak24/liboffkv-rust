[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Build Status](https://travis-ci.org/offscale/rsoffkv.svg?branch=master)](https://travis-ci.org/offscale/rsoffkv)

# rsoffkv

#### This library is designed to provide a uniform interface for 3 different distributed KV-storages: etcd, Zookeeper, Consul.

In our implementation, keys form a ZK-like hierarchy.
Each key has a version that is i64 number greater than 0.

Rsoffkv is a wrapper around _our_ C++ library [liboffkv](https://github.com/offscale/liboffkv).


## Build
* install dependencies (we recommend using vcpkg):
```bash
vcpkg install ppconsul etcdcpp zkpp
```
* build with cargo
```bash
cargo build
```
* run tests with
```bash
cargo tets
```


## Example
```rust
use rsoffkv::client::Client;
use rsoffkv::result::OffkvError;

use rsoffkv::txn::{Transaction, TxnCheck, TxnOp};

use std::{thread,time};

fn main() {
    // firstly specify service (zk | consul | etcd) and its address
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
        Ok(_ as Vec<TxnOpResult>) => println!("Success!"),
        // on failure an index of the first failed operation is returned
        Err(OffkvError::TxnFailed(failed_op)) => println!("Failed at {}", failed_op),
    };
}
```
