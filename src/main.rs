extern crate libc;
use std::ffi::{CString,CStr};
use std::os::raw::c_char;
use std::fmt;
use std::error;

#[repr(C)]
struct S {
    array: *const *const c_char,
    n: i32,
}

extern "C" {
    fn f(a: i32) -> S;
}

fn to_cstring(s: &str) -> *const c_char {
    CString::new(s).expect("Failed to create Cstring").as_ptr()
}

#[derive(Debug)]
enum OffkvError {
    EntryExists,
    NoEntry,
    TxnFailed(u32),
}

impl fmt::Display for OffkvError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self {
            OffkvError::EntryExists => write!(f, "Entry exists"),
            OffkvError::NoEntry => write!(f, "No entry"),
            OffkvError::TxnFailed(index) => write!(f, "Transaction failed. Failed op index: {}", index),
        }
    }
}

impl error::Error for OffkvError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

#[derive(Debug)]
struct WatchHandle;

impl WatchHandle {
    fn wait(&self) {

    }
}

impl Drop for WatchHandle {
    fn drop(&mut self) {

    }
}

struct TxnCheck {
    key: String,
    version: i64,
}

enum TxnOp {
    Create { key: String, value: String, leased: bool },
    Set { key: String, value: String },
    Erase { key: String },
}

type Transaction = (Vec<TxnCheck>, Vec<TxnOp>);

trait Client : Drop {
    fn new(address: &str, prefix: &str) -> Self;

    fn create(key: &str, value: &str, leased: bool) -> Result<i64, OffkvError>;
    fn exists(key: &str, watch: bool) -> Result<(i64, Option<WatchHandle>), OffkvError>;
    fn get(key: &str, watch: bool) -> Result<(i64, String, Option<WatchHandle>), OffkvError>;
    fn erase(key: &str, version: i64) -> Result<(), OffkvError>;
    fn get_children(key: &str, watch: bool) -> Result<Vec<String>, OffkvError>;
    fn set(key: &str, value: &str) -> Result<i64, OffkvError>;
    fn cas(key: &str, value: &str, version: i64) -> Result<i64, OffkvError>;
    fn commit(txn: &Transaction) -> Result<Vec<i64>, OffkvError>;
}

struct ConsulClient {
    prefix: String,
    address: String,
}

struct ETCDClient;
struct ZookeeperClient;

impl Drop for ConsulClient {
    fn drop(&mut self) {

    }
}

impl Client for ConsulClient {
    fn new(address: &str, prefix: &str) -> Self {
        ConsulClient{
            prefix: String::from(prefix),
            address: String::from(address),
        }
    }
}



fn main() {
    let x = vec![CString::new("KEK").unwrap().as_ptr(), CString::new("LOLOLO").unwrap().as_ptr()];
    let x = unsafe { f(32) };
    unsafe {
        let y = std::slice::from_raw_parts(x.array, 2);
        println!("{:?}", CStr::from_ptr(y[0]).to_str().unwrap());
    }
}
