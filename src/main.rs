extern crate libc;
use libc::{size_t,int64_t,c_int};
use std::ffi::{CString,CStr};
use std::os::raw::{c_char,c_void};
use std::fmt;
use std::error;
use std::ptr;
use std::slice;
use std::thread;
use std::time;
use std::slice::from_raw_parts;


#[link(name="liboffkv_c")]
extern "C" {
    fn offkv_open(
        url: *const c_char,
        prefix: *const c_char,
        error_code: *mut c_int
    ) -> *mut c_void;

    fn offkv_create(
        handle: *mut c_void,
        key: *const c_char,
        value: *const c_char,
        value_size: size_t,
        flags: c_int
    ) -> int64_t;

    fn offkv_erase(
        handle: *mut c_void,
        key: *const c_char,
        version: int64_t,
    ) -> c_int;

    fn offkv_set(
        handle: *mut c_void,
        key: *const c_char,
        value: *const c_char,
        value_size: size_t,
    ) -> int64_t;

    fn offkv_cas(
        handle: *mut c_void,
        key: *const c_char,
        value: *const c_char,
        value_size: size_t,
        version: int64_t,
    ) -> int64_t;

    fn offkv_get(
        handle: *mut c_void,
        key: *const c_char,
        watch_handle: *mut *mut c_void,
    ) -> GetResult;

    fn offkv_exists(
        handle: *mut c_void,
        key: *const c_char,
        watch_handle: *mut *mut c_void,
    ) -> int64_t;

    fn offkv_children(
        handle: *mut c_void,
        key: *const c_char,
        watch_handle: *mut *mut c_void,
    ) -> ChildrenResult;

    fn offkv_watch(watch_handle: *mut c_void);
    fn offkv_watch_drop(watch_handle: *mut c_void);
}

#[repr(C)]
struct GetResult {
    value: *mut c_char,
    value_size: size_t,
    version: int64_t,
}

#[repr(C)]
struct ChildrenResult {
    keys: *mut *mut c_char,
    nkeys: size_t,
    error_code: c_int,
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

type Result<T> = std::result::Result<T, OffkvError>;

#[derive(Debug)]
struct WatchHandle {
    _offkv_watch_handle: *mut c_void,
}

impl WatchHandle {
    fn wait(&self) {
        unsafe {
            offkv_watch(self._offkv_watch_handle);
        }
    }
}

impl Drop for WatchHandle {
    fn drop(&mut self) {
        unsafe {
            offkv_watch_drop(self._offkv_watch_handle);
        }
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

    // fn commit(txn: &Transaction) -> Result<Vec<i64>, OffkvError>;
}


impl Client {
    fn new(url: &str, prefix: &str) -> Result<Self> {
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

    fn create(&self, key: &str, value: &str, leased: bool) -> Result<i64> {
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

    fn erase(&self, key: &str, version: i64) -> Result<()> {
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

    fn set(&self, key: &str, value: &str) -> Result<i64> {
        let result = unsafe {
            offkv_set(
                self.offkv_handle,
                CString::new(key)
                    .expect("Failed to create CString").as_ptr(),
                CString::new(value)
                    .expect("Failed to create CString").as_ptr(),
                value.len(),
            )
        };

        match from_error_code(result) {
            Some(error) => Err(error),
            None => Ok(result),
        }
    }

    fn cas(&self, key: &str, value: &str, version: i64) -> Result<i64> {
        let result = unsafe {
            offkv_cas(
                self.offkv_handle,
                CString::new(key)
                    .expect("Failed to create CString").as_ptr(),
                CString::new(value)
                    .expect("Failed to create CString").as_ptr(),
                value.len(),
                version,
            )
        };

        match from_error_code(result) {
            Some(error) => Err(error),
            None => Ok(result),
        }
    }

    fn get(&self, key: &str, watch: bool)
        -> Result<(i64, String, Option<WatchHandle>)> {

        let mut watch_handle: *mut c_void = match watch {
            true => ptr::NonNull::dangling().as_ptr(),
            false => ptr::null_mut(),
        };

        let GetResult{version, value, value_size} = unsafe {
            offkv_get(
                self.offkv_handle,
                CString::new(key)
                    .expect("Failed to create CString").as_ptr(),
                &mut watch_handle,
            )
        };

        match from_error_code(version) {
            Some(error) => Err(error),
            None => {
                // <str_value> now _owns_ the data <value> is pointing at
                // so on its destroy the data will be freed
                let str_value = unsafe {
                    String::from_raw_parts(value as *mut u8, value_size, value_size)
                };

                let watch_handle = if !watch_handle.is_null() {
                    Some(WatchHandle{ _offkv_watch_handle: watch_handle})
                } else {
                    None
                };

                return Ok((version, str_value, watch_handle));
            }
        }
    }

    fn exists(&self, key: &str, watch: bool) -> Result<(i64, Option<WatchHandle>)> {
        let mut watch_handle: *mut c_void = match watch {
            true => ptr::NonNull::dangling().as_ptr(),
            false => ptr::null_mut(),
        };

        let result = unsafe {
            offkv_exists(
                self.offkv_handle,
                CString::new(key)
                    .expect("Failed to create CString").as_ptr(),
                &mut watch_handle,
            )
        };

        match from_error_code(result) {
            Some(error) => Err(error),
            None => {
                let watch_handle = if !watch_handle.is_null() {
                    Some(WatchHandle{ _offkv_watch_handle: watch_handle})
                } else {
                    None
                };

                Ok((result, watch_handle))
            }
        }
    }

    fn get_children(&self, key: &str, watch: bool)
        -> Result<(Vec<String>, Option<WatchHandle>)> {

        let mut watch_handle: *mut c_void = match watch {
            true => ptr::NonNull::dangling().as_ptr(),
            false => ptr::null_mut(),
        };

        let ChildrenResult{keys, nkeys, error_code} = unsafe {
            offkv_children(
                self.offkv_handle,
                CString::new(key)
                    .expect("Failed to create CString").as_ptr(),
                &mut watch_handle,
            )
        };

        match from_error_code(error_code as i64) {
            Some(error) => Err(error),
            None => {
                let mut vec = Vec::new();
                unsafe {
                    let keys = slice::from_raw_parts(keys, nkeys);
                    for key in keys {
                        vec.push(CString::from_raw(*key).into_string().unwrap());
                    }
                }

                let watch_handle = if !watch_handle.is_null() {
                    Some(WatchHandle{ _offkv_watch_handle: watch_handle})
                } else {
                    None
                };

                Ok((vec, watch_handle))
            }
        }
    }
}

fn main() {
    let c = Client::new("zk://localhost:2181", "/rust_test_key").unwrap();
    c.erase("/key", 0);
    c.create("/key", "value", false).unwrap();
    let v = c.set("/key", "new_value").unwrap();
    assert!(c.cas("/key", "kek", v).unwrap() != 0);

    thread::spawn(|| {
        let new_c = Client::new("zk://localhost:2181", "/rust_test_key").unwrap();
        let (_, value, watch_handle) =
            new_c.get("/key", true).unwrap();
        assert_eq!(value, String::from("kek"));

        watch_handle.unwrap().wait();
        let (_, value, _) =
            new_c.get("/key", false).unwrap();
        assert_eq!(value, String::from("lol"));
    });

    std::thread::sleep(time::Duration::from_secs(5));
    c.set("/key", "lol").unwrap();

    c.create("/key/key1", "value", true);
    c.create("/key/key2", "value", true);

    assert!(c.exists("/key/key1", false).unwrap().0 != 0);

    let (children, _) = c.get_children("/key", false).unwrap();
    for child in children {
        println!("{:?}", child);
    }

    c.erase("/key", 0).unwrap();
}
