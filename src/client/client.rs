use std::ffi::CString;
use std::os::raw::{c_char,c_void};
use std::{mem,ptr,slice};

use super::ffi::*;

use crate::txn::*;
use crate::result::*;


type Result<T> = std::result::Result<T, OffkvError>;


pub struct WatchHandle {
    _offkv_watch_handle: *mut c_void,
}

impl WatchHandle {
    pub fn wait(&self) {
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


pub struct Client {
    offkv_handle: *mut c_void,
}


impl Client {
    /// Crates a new client given url, where the service is located and
    /// prefix all keys should start with.
    ///
    /// # Arguments:
    ///
    /// * `url` - Address, where the service is located. Must be of form
    /// "<service_name>://<host>:<port>" where <service_name> is one of {zk, consul, etcd}.
    ///
    /// # Example:
    ///
    /// ```
    /// use rsoffkv::client::Client;
    /// let client = Client::new("zk://localhost:2181", "/test_prefix").unwrap();
    /// ```
    pub fn new(url: &str, prefix: &str) -> Result<Self> {
        let mut error_code: i32 = 0;

        let offkv_handle: *mut c_void = unsafe {
            offkv_open(
                // create a null-terminated owned string
                // it will be live until the function returns so ptr will be valid
                to_cstring(url).as_ptr(),
                to_cstring(prefix).as_ptr(),
                &mut error_code,
            )
        };

        match from_error_code(error_code as i64) {
            Some(error) => Err(error),
            None => Ok(Client{offkv_handle}),
        }
    }

    pub fn create(&self, key: &str, value: &str, leased: bool) -> Result<i64> {
        let result = unsafe {
            offkv_create(
                self.offkv_handle,
                to_cstring(key).as_ptr(),
                // not null-terminated
                value.as_ptr() as *const c_char,
                value.len(),
                match leased {
                    true => OFFKV_LEASE,
                    false => 0,
                }
            )
        };

        match from_error_code(result) {
            Some(error) => Err(error),
            None => Ok(result),
        }
    }

    pub fn erase(&self, key: &str, version: i64) -> Result<()> {
        let result = unsafe {
            offkv_erase(
                self.offkv_handle,
                to_cstring(key).as_ptr(),
                version,
            )
        };

        match from_error_code(result as i64) {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    pub fn set(&self, key: &str, value: &str) -> Result<i64> {
        let result = unsafe {
            offkv_set(
                self.offkv_handle,
                to_cstring(key).as_ptr(),
                value.as_ptr() as *const c_char,
                value.len(),
            )
        };

        match from_error_code(result) {
            Some(error) => Err(error),
            None => Ok(result),
        }
    }

    pub fn cas(&self, key: &str, value: &str, version: i64) -> Result<i64> {
        let result = unsafe {
            offkv_cas(
                self.offkv_handle,
                to_cstring(key).as_ptr(),
                value.as_ptr() as *const c_char,
                value.len(),
                version,
            )
        };

        match from_error_code(result) {
            Some(error) => Err(error),
            None => Ok(result),
        }
    }

    pub fn get(&self, key: &str, watch: bool)
           -> Result<(i64, String, Option<WatchHandle>)> {

        let mut watch_handle: *mut c_void = match watch {
            true => ptr::NonNull::dangling().as_ptr(),
            false => ptr::null_mut(),
        };

        let offkv_GetResult{version, value, value_size} = unsafe {
            offkv_get(
                self.offkv_handle,
                to_cstring(key).as_ptr(),
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

    pub fn exists(&self, key: &str, watch: bool) -> Result<(i64, Option<WatchHandle>)> {
        let mut watch_handle: *mut c_void = match watch {
            true => ptr::NonNull::dangling().as_ptr(),
            false => ptr::null_mut(),
        };

        let result = unsafe {
            offkv_exists(
                self.offkv_handle,
                to_cstring(key).as_ptr(),
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

    pub fn get_children(&self, key: &str, watch: bool)
                    -> Result<(Vec<String>, Option<WatchHandle>)> {

        let mut watch_handle: *mut c_void = match watch {
            true => ptr::NonNull::dangling().as_ptr(),
            false => ptr::null_mut(),
        };

        let offkv_ChildrenResult{keys, nkeys, error_code} = unsafe {
            offkv_children(
                self.offkv_handle,
                to_cstring(key).as_ptr(),
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

    pub fn commit(&self, transaction: Transaction) -> Result<Vec<TxnOpResult>> {
        let mut checks = Vec::new();

        // firstly create null-terminated c-strings
        let cstrings_checks : Vec<CString> =
            transaction.checks
                .iter()
                .map(|TxnCheck{key, ..}| to_cstring(key))
                .collect();

        let cstrings_ops : Vec<CString> =
            transaction.ops
                .iter()
                .map(|op| match *op {
                    TxnOp::Create{key, ..} |
                    TxnOp::Set{key, ..} |
                    TxnOp::Erase{key}
                        => to_cstring(key)
                })
                .collect();


        // then pass pointers to them
        for (TxnCheck{version, ..}, key) in transaction.checks.iter().zip(cstrings_checks.iter()) {
            checks.push(offkv_TxnCheck{
                key: key.as_ptr(),
                version: *version,
            });
        }

        let mut ops = Vec::new();
        for (op, key) in transaction.ops.iter().zip(cstrings_ops.iter()) {
            match op {
                TxnOp::Create{value, leased, ..} => {
                    ops.push(offkv_TxnOp{
                        op_kind: OffkvTxnOpCode::OFFKV_OP_CREATE as i32,
                        flags: match *leased {
                            true => OFFKV_LEASE,
                            false => 0
                        },
                        key: key.as_ptr(),
                        value: value.as_ptr() as *const c_char,
                        value_size: value.len(),
                    });
                },
                TxnOp::Set{value, ..} => {
                    ops.push(offkv_TxnOp{
                        op_kind: OffkvTxnOpCode::OFFKV_OP_SET as i32,
                        key: key.as_ptr(),
                        value: value.as_ptr() as *const c_char,
                        value_size: value.len(),
                        // default
                        flags: 0,
                    });
                },
                TxnOp::Erase{..} => {
                    ops.push(offkv_TxnOp{
                        op_kind: OffkvTxnOpCode::OFFKV_OP_ERASE as i32,
                        key: key.as_ptr(),
                        // default
                        flags: 0,
                        value: ptr::null(),
                        value_size: 0,
                    });
                },
            }
        }

        let mut txn_result = mem::MaybeUninit::uninit();

        let error_code = unsafe {
            offkv_commit(
                self.offkv_handle,
                checks.as_ptr(),
                checks.len(),
                ops.as_ptr(),
                ops.len(),
                txn_result.as_mut_ptr(),
            )
        };

        let offkv_TxnResult{results, nresults, failed_op} =
            unsafe { txn_result.assume_init() };

        match from_error_code(error_code as i64) {
            Some(OffkvError::TxnFailed(_)) => Err(OffkvError::TxnFailed(failed_op as u32)),
            Some(error) => Err(error),
            None => Ok(unsafe { Vec::from_raw_parts(results, nresults, nresults) }
                .iter()
                .map(|offkv_TxnOpResult{op_kind, version}|
                    match *op_kind {
                        x if x == OffkvTxnOpCode::OFFKV_OP_CREATE as i32
                            => TxnOpResult::Create(*version),
                        x if x == OffkvTxnOpCode::OFFKV_OP_SET as i32
                            => TxnOpResult::Set(*version),
                        _ => unreachable!(),
                    })
                .collect())
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        unsafe {
            offkv_close(self.offkv_handle);
        }
    }
}
