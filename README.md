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
* use cargo
```bash
cargo build
```
## Docs
```bash
cargo doc --open
```
