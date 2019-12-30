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
use syntax::util::map_in_place::MapInPlace;


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
    ) -> offkv_GetResult;

    fn offkv_exists(
        handle: *mut c_void,
        key: *const c_char,
        watch_handle: *mut *mut c_void,
    ) -> int64_t;

    fn offkv_children(
        handle: *mut c_void,
        key: *const c_char,
        watch_handle: *mut *mut c_void,
    ) -> offkv_ChildrenResult;

    fn offkv_commit(
        handle: *mut c_void,
        checks: *const Check,
        nchecks: size_t,
        ops: *const Operations,
        nops: size_t,
        result: *mut offkv_TxnResult,
    ) -> c_int;

    fn offkv_watch(watch_handle: *mut c_void);
    fn offkv_watch_drop(watch_handle: *mut c_void);
}

#[repr(C)]
struct offkv_GetResult {
    value: *mut c_char,
    value_size: size_t,
    version: int64_t,
}

#[repr(C)]
struct offkv_ChildrenResult {
    keys: *mut *mut c_char,
    nkeys: size_t,
    error_code: c_int,
}

#[repr(C)]
struct offkv_TxnCheck {
    key: *const c_char,
    version: int64_t,
}

#[repr(C)]
struct offkv_TxnOp {
    op_kind: c_int,
    flags: c_int,
    key: *const c_char,
    value: *const c_char,
    value_size: size_t,
}

#[repr(C)]
struct offkv_TxnOpResult {
    op_kind: c_int,
    version: int64_t,
}

#[repr(C)]
struct offkv_TxnResult {
    results: *mut offkv_TxnOpResult,
    nresults: size_t,
    failed_op: size_t,
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

struct TxnCheck<'a> {
    key: &'a str,
    version: i64,
}

enum TxnOp<'a> {
    Create { key: &'a str, value: &'a str, leased: bool },
    Set    { key: &'a str, value: &'a str},
    Erase  { key: &'a str },
}

struct Transaction<'a> {
    checks: Vec<TxnCheck<'a>>,
    ops: Vec<TxnOp<'a>>,
}

enum TxnOpResult {
    Create(i64),
    Set(i64),
}

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

        let offkv_GetResult{version, value, value_size} = unsafe {
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

        let offkv_ChildrenResult{keys, nkeys, error_code} = unsafe {
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

    fn commit(&self, transaction: Transaction) -> Result<Vec<TxnOpResult>> {
        let mut checks = Vec::new();
        for (key, version) in transaction.checks {
            checks.push(offkv_TxnCheck{
                key: CString::new(key)
                    .expect("Failed to create CString").as_ptr(),
                version,
            });
        }

        let mut ops = Vec::new();
        for op in transaction.ops {
            match op {
                TxnOp::Create{key, value, leased} => {
                    ops.push(offkv_TxnOp{
                        op_kind: OffkvTxnOpCode::OFFKV_OP_CREATE as c_int,
                        flags: match leased {
                            true => OFFKV_LEASE,
                            false => 0
                        },
                        key: CString::new(key)
                            .expect("Failed to create CString").as_ptr(),
                        value: CString::new(value)
                            .expect("Failed to create CString").as_ptr(),
                        value_size: value.len(),
                    });
                },
                TxnOp::Set{key, value} => {
                    ops.push(offkv_TxnOp{
                        op_kind: OffkvTxnOpCode::OFFKV_OP_SET as c_int,
                        flags: 0,
                        key: CString::new(key)
                            .expect("Failed to create CString").as_ptr(),
                        value: CString::new(value)
                            .expect("Failed to create CString").as_ptr(),
                        value_size: value.len(),
                    });
                },
                TxnOp::Erase{key} => {
                    ops.push(offkv_TxnOp{
                        op_kind: OffkvTxnOpCode::OFFKV_OP_ERASE as c_int,
                        flags: 0,
                        key: CString::new(key)
                            .expect("Failed to create CString").as_ptr(),
                        value: ptr::null(),
                        value_size: 0,
                    });
                },
            }
        }

        let mut results = ptr::null_mut();
        let mut nresults: size_t = 0;
        let mut failed_op: size_t = -1;

        let mut txn_result = offkv_TxnResult{
            results: results as *mut offkv_TxnOpResult,
            &mut nresults,
            &mut failed_op,
        };

        let error_code = unsafe {
            offkv_commit(
                self.offkv_handle,
                checks.as_ptr(),
                checks.len(),
                ops.as_ptr(),
                ops.len(),
                &mut txn_result,
            )
        };

        match from_error_code(error_code as i64) {
            Some(error) => Err(error),
            None => {
                if failed_op != 0 {
                    Err(OffkvError::TxnFailed(failed_op as u32))
                } else {
                    Ok(unsafe {
                        Vec::from_raw_parts(
                            results as *mut offkv_TxnOpResult,
                            nresults,
                            nresults)
                        }.expect("Failed to create vector from raw")
                        .iter()
                        .map(|offkv_TxnOpResult{op_kind, version}|
                            match op_kind {
                                x if x == OffkvTxnOpCode::OFFKV_OP_CREATE as i32
                                    => TxnOpResult::Create(version),
                                x if x == OffkvTxnOpCode::OFFKV_OP_SET as i32
                                    => TxnOpResult::Set(version),
                                _ => unreachable!(),
                            })
                        .collect())
                }
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

    let result = c.commit(Transaction{
            checks: vec![
                TxnCheck{key: "/key", version: 0},
                TxnCheck{key: "/key/ke1", version: 0},
            ],
            ops: vec![
                TxnOp::Create{key: "/new_key", value: "value", leased: true},
                TxnOp::Set{key: "/key/key1", value: "new_value"},
                TxnOp::Erase{key: "/key/key2"},
            ],
        }
    ).unwrap();

    c.erase("/key", 0).unwrap();
}
