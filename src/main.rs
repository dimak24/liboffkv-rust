extern crate libc;
use libc::{size_t,int64_t};
use std::ffi::{CString,CStr};
use std::os::raw::{c_char,c_int,c_void};
use std::fmt;
use std::error;


#[link(name="offkv_c")]
extern "C" {
    fn offkv_open(
        url: *const c_char,
        prefix: *const c_char,
        error_code: *mut c_int) -> *mut c_void;

    fn offkv_create(
        handle: *mut c_void,
        key: *const c_char,
        value: *const c_char,
        value_size: size_t,
        flags: c_int) -> int64_t;

    fn offkv_erase(
        handle: *mut c_void,
        key: *const c_char,
        version: int64_t,
    ) -> c_int;
}

//flags
const OFFKV_LEASE: i32 = 1 << 0;

#[repr(C)]
enum OffkvErrorCode {
    OFFKV_EADDR  = -1,
    OFFKV_EKEY   = -2,
    OFFKV_ENOENT = -3,
    OFFKV_EEXIST = -4,
    OFFKV_EEPHEM = -5,
    OFFKV_ECONN  = -6,
    OFFKV_ETXN   = -7,
    OFFKV_ESRV   = -8,
    OFFKV_ENOMEM = -9,
}

fn from_error_code(error_code: i64) -> Option<OffkvError> {
    match error_code {
        x if x == OffkvErrorCode::OFFKV_EEXIST as i64 => Some(OffkvError::EntryExists),
        x if x == OffkvErrorCode::OFFKV_ENOENT as i64 => Some(OffkvError::NoEntry),
        x if x == OffkvErrorCode::OFFKV_EADDR as i64 => Some(OffkvError::InvalidAddress),
        x if x == OffkvErrorCode::OFFKV_EKEY as i64 => Some(OffkvError::InvalidKey),
        _ => None,
    }
}

// transaction operations
#[repr(C)]
enum OffkvTxnOpCode {
    OFFKV_OP_CREATE,
    OFFKV_OP_SET,
    OFFKV_OP_ERASE,
}

fn to_cstring(s: &str) -> *const c_char {
    CString::new(s).expect("Failed to create CString").as_ptr()
}

#[derive(Debug)]
enum OffkvError {
    InvalidAddress,
    InvalidKey,
    EntryExists,
    NoEntry,
    TxnFailed(u32),
}

impl fmt::Display for OffkvError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self {
            OffkvError::EntryExists => write!(f, "entry exists"),
            OffkvError::NoEntry => write!(f, "no entry"),
            OffkvError::TxnFailed(index)
                => write!(f, "transaction failed (failed operation index: {})", index),
            OffkvError::InvalidAddress => write!(f, "invalid address"),
            OffkvError::InvalidKey => write!(f, "invalid key"),
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

struct Client {
    offkv_handle: *mut c_void,

    // fn exists(key: &str, watch: bool) -> Result<(i64, Option<WatchHandle>), OffkvError>;
    // fn get(key: &str, watch: bool) -> Result<(i64, String, Option<WatchHandle>), OffkvError>;
    // fn get_children(key: &str, watch: bool) -> Result<Vec<String>, OffkvError>;
    // fn set(key: &str, value: &str) -> Result<i64, OffkvError>;
    // fn cas(key: &str, value: &str, version: i64) -> Result<i64, OffkvError>;
    // fn commit(txn: &Transaction) -> Result<Vec<i64>, OffkvError>;
}


impl Client {
    fn new(url: &str, prefix: &str) -> Result<Self, OffkvError> {
        let mut error_code: i32 = 0;

        let mut offkv_handle: *mut c_void = unsafe {
            offkv_open(
                CString::new(url)
                    .expect("Failed to create CString").as_ptr(),
                CString::new(prefix)
                    .expect("Failed to create CString").as_ptr(),
                &mut error_code,
            )
        };

        match from_error_code(error_code as i64) {
            Some(error) => Err(error),
            None => Ok(Client{offkv_handle}),
        }
    }

    fn create(&self, key: &str, value: &str, leased: bool) -> Result<i64, OffkvError> {
        let result = unsafe {
            offkv_create(
                self.offkv_handle,
                CString::new(key)
                    .expect("Failed to create CString").as_ptr(),
                CString::new(value)
                    .expect("Failed to create CString").as_ptr(),
                value.len(),
                match leased {
                    true => OFFKV_LEASE as c_int,
                    false => 0,
                }
            )
        };

        match from_error_code(result) {
            Some(error) => Err(error),
            None => Ok(result),
        }
    }

    fn erase(&self, key: &str, version: i64) -> Result<(), OffkvError> {
        let result = unsafe {
            offkv_erase(
                self.offkv_handle,
                CString::new(key)
                    .expect("Failed to create CString").as_ptr(),
                version,
            )
        };

        match from_error_code(result as i64) {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

fn main() {
    let c = Client::new("KEK", "LOL").unwrap();
    c.create("key", "value", false).unwrap();
    c.erase("key", 0).unwrap();
}
