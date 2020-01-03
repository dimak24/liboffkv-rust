extern crate libc;
use libc::{size_t,int64_t,c_int};
use std::os::raw::{c_char,c_void};


#[link(name = "liboffkv_c")]
extern "C" {
    pub(crate) fn offkv_open(
        url: *const c_char,
        prefix: *const c_char,
        error_code: *mut c_int
    ) -> *mut c_void;

    pub(crate) fn offkv_create(
        handle: *mut c_void,
        key: *const c_char,
        value: *const c_char,
        value_size: size_t,
        flags: c_int
    ) -> int64_t;

    pub(crate) fn offkv_erase(
        handle: *mut c_void,
        key: *const c_char,
        version: int64_t,
    ) -> c_int;

    pub(crate) fn offkv_set(
        handle: *mut c_void,
        key: *const c_char,
        value: *const c_char,
        value_size: size_t,
    ) -> int64_t;

    pub(crate) fn offkv_cas(
        handle: *mut c_void,
        key: *const c_char,
        value: *const c_char,
        value_size: size_t,
        version: int64_t,
    ) -> int64_t;

    pub(crate) fn offkv_get(
        handle: *mut c_void,
        key: *const c_char,
        watch_handle: *mut *mut c_void,
    ) -> offkv_GetResult;

    pub(crate) fn offkv_exists(
        handle: *mut c_void,
        key: *const c_char,
        watch_handle: *mut *mut c_void,
    ) -> int64_t;

    pub(crate) fn offkv_children(
        handle: *mut c_void,
        key: *const c_char,
        watch_handle: *mut *mut c_void,
    ) -> offkv_ChildrenResult;

    pub(crate) fn offkv_commit(
        handle: *mut c_void,
        checks: *const offkv_TxnCheck,
        nchecks: size_t,
        ops: *const offkv_TxnOp,
        nops: size_t,
        result: *mut offkv_TxnResult,
    ) -> c_int;

    pub(crate) fn offkv_watch(watch_handle: *mut c_void);
    pub(crate) fn offkv_watch_drop(watch_handle: *mut c_void);

    pub(crate) fn offkv_close(handle: *mut c_void);
}

#[repr(C)]
pub(crate) struct offkv_GetResult {
    value: *mut c_char,
    value_size: size_t,
    version: int64_t,
}

#[repr(C)]
pub(crate) struct offkv_ChildrenResult {
    keys: *mut *mut c_char,
    nkeys: size_t,
    error_code: c_int,
}

#[repr(C)]
pub(crate) struct offkv_TxnCheck {
    key: *const c_char,
    version: int64_t,
}

#[repr(C)]
pub(crate) struct offkv_TxnOp {
    op_kind: c_int,
    flags: c_int,
    key: *const c_char,
    value: *const c_char,
    value_size: size_t,
}

#[repr(C)]
pub(crate) struct offkv_TxnOpResult {
    op_kind: c_int,
    version: int64_t,
}

#[repr(C)]
pub(crate) struct offkv_TxnResult {
    results: *mut offkv_TxnOpResult,
    nresults: size_t,
    failed_op: size_t,
}


//flags
pub(crate) const OFFKV_LEASE: c_int = 1 << 0;

#[repr(C)]
pub(crate) enum OffkvErrorCode {
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

// transaction operations
#[repr(C)]
pub(crate) enum OffkvTxnOpCode {
    OFFKV_OP_CREATE,
    OFFKV_OP_SET,
    OFFKV_OP_ERASE,
}
