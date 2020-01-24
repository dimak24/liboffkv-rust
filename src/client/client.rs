use std::ffi::CString;
use std::os::raw::{c_char,c_void};
use std::{mem,ptr,slice};

use super::ffi::*;

use crate::txn::*;
use crate::result::*;


type Result<T> = std::result::Result<T, OffkvError>;


pub struct WatchHandle<'a> {
    _parent_client: &'a Client,
    _offkv_watch_handle: *mut c_void,
}

impl<'a> WatchHandle<'a> {
    /// Waits until some events occurred (depends on method `WatchHandle` is returned from)
    pub fn wait(self) {
        unsafe {
            offkv_watch(self._offkv_watch_handle);
        }
    }

    pub(crate) fn new(parent: &'a Client, ffi_watch_handle: *mut c_void) -> Self {
        Self{_parent_client: parent, _offkv_watch_handle: ffi_watch_handle}
    }
}

impl Drop for WatchHandle<'_> {
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
    /// Creates a new client given url, where the service is located, and
    /// prefix all keys should start with.
    ///
    /// # Arguments:
    ///
    /// * `url` - Address, where the service is located. Must be of form
    /// `<service_name>://<host>:<port>` where `<service_name>` is one of `{zk, consul, etcd}`.
    /// * `prefix` - An additional prefix, all used keys start with.
    ///
    /// # Example:
    ///
    /// ```
    /// use rsoffkv::client::Client;
    /// let zk_client = Client::new("zk://localhost:2181", "/test_prefix").unwrap();
    /// let consul_client = Client::new("consul://localhost:8500", "/test_prefix").unwrap();
    /// let etcd_client = Client::new("etcd://localhost:2379", "/test_prefix").unwrap();
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

        from_error_code(error_code as i64).map_or(Ok(Client{offkv_handle}), Err)
    }

    /// Creates new key. The parent key must exist.
    ///
    /// # Arguments:
    ///
    /// * `key` - key to create
    /// * `value` - initial value
    /// * `leased` - if `true` the key will be removed on client's disconnect
    ///
    /// # Returns:
    /// * inital version (`i64`)
    ///
    /// # Example:
    /// ```
    /// # use rsoffkv::client::Client;
    /// let client = Client::new("consul://localhost:8500", "/test_prefix").unwrap();
    /// let initial_version = client.create("/key", "value", false).unwrap();
    ///
    /// let (version, value, _) = client.get("/key", false).unwrap();
    /// assert_eq!(version, initial_version);
    /// assert_eq!(value, String::from("value"));
    ///
    /// # client.erase("/key", 0);
    /// ```
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

        from_error_code(result).map_or(Ok(result), Err)
    }

    /// Erases existing key.
    ///
    /// # Arguments:
    ///
    /// * `key` - key to erase
    /// * `version` - if not 0, erases the key _and all its descendants_
    /// iff its version equals to the given one,
    /// otherwise does it unconditionally
    ///
    /// # Returns:
    /// * ()
    ///
    /// # Example:
    /// ```
    /// # use rsoffkv::client::Client;
    /// use rsoffkv::result::OffkvError;
    /// let client = Client::new("consul://localhost:8500", "/test_prefix").unwrap();
    /// let initial_version = client.create("/key", "value", false).unwrap();
    ///
    /// // does nothing since versions differ
    /// client.erase("/key", initial_version + 1).unwrap();
    ///
    /// // should erase the key
    /// client.erase("/key", initial_version).unwrap();
    ///
    /// // next try to erase should panic with `NoEntry`
    /// if let OffkvError::NoEntry = client.erase("/key", 0).unwrap_err() {}
    /// else { assert!(false) }
    ///
    /// # client.erase("/key", 0);
    /// ```
    pub fn erase(&self, key: &str, version: i64) -> Result<()> {
        let result = unsafe {
            offkv_erase(
                self.offkv_handle,
                to_cstring(key).as_ptr(),
                version,
            )
        };

        from_error_code(result as i64).map_or(Ok(()), Err)
    }

    /// Assigns the value to the the key, creates it not exist (the parent key must exist).
    ///
    /// # Arguments:
    ///
    /// * `key` - key to assign value to
    /// * `value` - value to be assigned
    ///
    /// # Returns:
    ///
    /// * new version of the key
    ///
    /// # Example:
    /// ```
    /// # use rsoffkv::client::Client;
    /// # use rsoffkv::result::OffkvError;
    /// let client = Client::new("consul://localhost:8500", "/test_prefix").unwrap();
    /// let initial_version = client.set("/key", "value").unwrap();
    ///
    /// {
    ///     let (version, value, _) = client.get("/key", false).unwrap();
    ///     assert_eq!(version, initial_version);
    ///     assert_eq!(value, String::from("value"));
    /// }
    ///
    /// let new_version = client.set("/key", "new value").unwrap();
    ///
    /// {
    ///     let (version, value, _) = client.get("/key", false).unwrap();
    ///     assert_eq!(version, new_version);
    ///     assert_eq!(value, String::from("new value"));
    /// }
    ///
    /// # client.erase("/key", 0);
    /// ```
    pub fn set(&self, key: &str, value: &str) -> Result<i64> {
        let result = unsafe {
            offkv_set(
                self.offkv_handle,
                to_cstring(key).as_ptr(),
                value.as_ptr() as *const c_char,
                value.len(),
            )
        };

        from_error_code(result).map_or(Ok(result), Err)
    }

    /// Compare and set operation: if version is not 0, assigns value to key iff
    /// its current version equals to the given one, otherwise creates the key (its parent must exist).
    ///
    /// # Arguments:
    ///
    /// * `key` - the key CAS is to be performed on
    /// * `value` - value to be assigned
    /// * `version` - assumed current key's version
    ///
    /// # Returns:
    ///
    /// * new version of the key or 0 on failure (given version not equal to current)
    ///
    /// # Example:
    /// ```
    /// # use rsoffkv::client::Client;
    /// # use rsoffkv::result::OffkvError;
    /// let client = Client::new("consul://localhost:8500", "/test_prefix").unwrap();
    /// let initial_version = client.cas("/key", "value", 0).unwrap();
    ///
    /// {
    ///     let (version, value, _) = client.get("/key", false).unwrap();
    ///     assert_eq!(version, initial_version);
    ///     assert_eq!(value, String::from("value"));
    /// }
    ///
    /// // does nothing due to given version isn't equal to the current one
    /// let new_version = client.cas("/key", "new value", initial_version + 10).unwrap();
    /// assert_eq!(0, new_version);
    ///
    /// {
    ///     let (version, value, _) = client.get("/key", false).unwrap();
    ///     assert_eq!(version, initial_version);
    ///     assert_eq!(value, String::from("value"));
    /// }
    ///
    /// let new_version = client.cas("/key", "new value", initial_version).unwrap();
    ///
    /// {
    ///     let (version, value, _) = client.get("/key", false).unwrap();
    ///     assert_eq!(version, new_version);
    ///     assert_eq!(value, String::from("new value"));
    /// }
    ///
    /// # client.erase("/key", 0);
    /// ```
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

        from_error_code(result).map_or(Ok(result), Err)
    }

    /// Returns current version and assigned value.
    ///
    /// # Arguments:
    ///
    /// * `key` - a certain key
    /// * `watch` - if true, a `WatchHandle` is returned,
    /// it can wait for key deletion or its value change.
    ///
    /// # Returns:
    ///
    /// * current version of the key
    /// * current assigned value
    /// * (optional) `WatchHandle`
    ///
    /// # Example:
    /// ```
    /// # use rsoffkv::client::Client;
    /// # use rsoffkv::result::OffkvError;
    /// let client = Client::new("consul://localhost:8500", "/test_prefix").unwrap();
    /// client.create("/key", "value", false);
    ///
    /// use std::{thread,time};
    ///
    /// thread::spawn(|| {
    ///     let another_client = Client::new("consul://localhost:8500", "/test_prefix").unwrap();
    ///
    ///     let (_, value, watch_handle) = another_client.get("/key", true).unwrap();
    ///     assert_eq!(value, String::from("value"));
    ///
    ///     watch_handle.unwrap().wait();
    ///
    ///     let (_, value, _) = another_client.get("/key", false).unwrap();
    ///     assert_eq!(value, String::from("new value"));
    /// });
    ///
    /// thread::sleep(time::Duration::from_secs(5));
    /// client.set("/key", "new value");
    ///
    /// # client.erase("/key", 0);
    /// ```
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

        if let Some(error) = from_error_code(version) {
            Err(error)
        } else {
            // <str_value> now _owns_ the data <value> is pointing at
            // so on its destroy the data will be freed
            let str_value = unsafe {
                String::from_raw_parts(value as *mut u8, value_size, value_size)
            };

            let watch_handle = if !watch_handle.is_null() {
                Some(WatchHandle::new(&self, watch_handle))
            } else {
                None
            };

            return Ok((version, str_value, watch_handle));
        }
    }

    /// Checks if the key exists.
    ///
    /// # Arguments:
    ///
    /// * `key` - key to check for existence
    /// * `watch` - if true, creates `WatchHandle` that can wait for changes in the key's existence state
    ///
    /// # Returns:
    ///
    /// * current version if the key exists, 0 otherwise
    /// * (optional) `WatchHandle`
    ///
    /// # Example:
    /// ```
    /// # use rsoffkv::client::Client;
    /// # use rsoffkv::result::OffkvError;
    /// let client = Client::new("consul://localhost:8500", "/test_prefix").unwrap();
    /// client.create("/key", "value", false);
    ///
    /// use std::{thread,time};
    ///
    /// thread::spawn(|| {
    ///     let another_client = Client::new("consul://localhost:8500", "/test_prefix").unwrap();
    ///
    ///     let (version, watch_handle) = another_client.exists("/key", true).unwrap();
    ///     assert_ne!(0, version);
    ///
    ///     watch_handle.unwrap().wait();
    ///
    ///     let (version, _) = another_client.exists("/key", false).unwrap();
    ///     assert_eq!(0, version);
    /// });
    ///
    /// thread::sleep(time::Duration::from_secs(5));
    /// client.erase("/key", 0);
    /// ```
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

        if let Some(error) = from_error_code(result) {
            Err(error)
        } else {
            let watch_handle = if !watch_handle.is_null() {
                Some(WatchHandle::new(&self, watch_handle))
            } else { None };

            Ok((result, watch_handle))
        }
    }

    /// Returns a list of _direct_ children.
    ///
    /// # Arguments:
    ///
    /// * `key` - key whose children are to be found
    /// * `watch` - if true, creates `WatchHandle`
    /// that can wait for any changes among children of given key
    ///
    /// # Returns:
    /// * `Vec` of direct children
    /// * (optional) `WatchHandle`
    ///
    /// # Example:
    /// ```
    /// # use rsoffkv::client::Client;
    /// # use rsoffkv::result::OffkvError;
    /// let client = Client::new("consul://localhost:8500", "/test_prefix").unwrap();
    /// client.create("/key", "value", false);
    /// client.create("/key/child1", "value", false);
    /// client.create("/key/child2", "value", false);
    /// client.create("/key/child3", "value", false);
    /// client.create("/key/child1/not_child", "value", false);
    ///
    /// use std::collections::HashSet;
    /// use std::{thread, time};
    ///
    /// thread::spawn(|| {
    ///     let another_client = Client::new("consul://localhost:8500", "/test_prefix").unwrap();
    ///
    ///     let (children, watch_handle) = another_client.get_children("/key", true).unwrap();
    ///     {
    ///         let result: HashSet<_> = children.iter().cloned().collect();
    ///         let real: HashSet<_> =
    ///             ["/key/child1", "/key/child2", "/key/child3"]
    ///                 .iter()
    ///                 .map(|s| String::from(*s))
    ///                 .collect();
    ///
    ///         assert_eq!(result, real);
    ///     }
    ///
    ///     watch_handle.unwrap().wait();
    ///
    ///     let (children, _) = another_client.get_children("/key", false).unwrap();
    ///     {
    ///         let result: HashSet<_> = children.iter().cloned().collect();
    ///         let real: HashSet<_> =
    ///             ["/key/child1", "/key/child2"]
    ///                 .iter()
    ///                 .map(|s| String::from(*s))
    ///                 .collect();
    ///
    ///         assert_eq!(result, real);
    ///     }
    /// });
    ///
    /// thread::sleep(time::Duration::from_secs(5));
    /// client.erase("/key/child3", 0);
    ///
    /// # thread::sleep(time::Duration::from_secs(5));
    /// # client.erase("/key", 0);
    /// ```
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

        if let Some(error) = from_error_code(error_code as i64) {
            Err(error)
        } else {
            let mut vec = Vec::new();
            unsafe {
                let keys = slice::from_raw_parts(keys, nkeys);
                for key in keys {
                    vec.push(CString::from_raw(*key).into_string().unwrap());
                }
            }

            let watch_handle = if !watch_handle.is_null() {
                Some(WatchHandle::new(&self, watch_handle))
            } else {
                None
            };

            Ok((vec, watch_handle))
        }
    }

    /// Commits transaction. Transaction consists of two parts: firstly list
    /// some `TxnCheck`s -- checks that some keys have specified versions (or just exist);
    /// next list `TxnOp`s -- operations.
    ///
    /// `Transaction`, `TxnCheck` and `TxnOp` are available in `rsoffkv::txn` module
    ///
    /// # Arguments:
    ///
    /// * `transaction` - transaction to commit
    ///
    /// # Rertuns:
    ///
    /// * `Vec` of TxnOpResult - for each operation affecting versions
    /// (namely, `TxnOp::Set` and `TxnOp::Create`) returns a new key version
    ///
    /// # Example:
    /// ```
    /// # use rsoffkv::client::Client;
    /// # use rsoffkv::result::OffkvError;
    /// use rsoffkv::txn::{Transaction, TxnCheck, TxnOp};
    /// let client = Client::new("consul://localhost:8500", "/test_prefix").unwrap();
    /// let initial_version = client.create("/key", "value", false).unwrap();
    /// client.commit(Transaction{
    ///     checks: vec![
    ///         TxnCheck{key: "/key", version: initial_version},
    ///     ],
    ///     ops: vec![
    ///         TxnOp::Create{key: "/key/child", value: "value", leased: false},
    ///         TxnOp::Set{key: "/key", value: "new value"},
    ///     ],
    /// }).unwrap();
    ///
    /// assert_eq!(client.get("/key", false).unwrap().1, String::from("new value"));
    /// assert_eq!(client.get("/key/child", false).unwrap().1, String::from("value"));
    ///
    /// # client.erase("/key", 0);
    /// ```
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
