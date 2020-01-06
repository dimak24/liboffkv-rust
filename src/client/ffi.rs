use libc::{size_t,c_int};
use std::os::raw::{c_char,c_void};
use std::ffi::CString;


extern "C" {
    pub(super) fn offkv_open(
        url: *const c_char,
        prefix: *const c_char,
        error_code: *mut c_int
    ) -> *mut c_void;

    pub(super) fn offkv_create(
        handle: *mut c_void,
        key: *const c_char,
        value: *const c_char,
        value_size: size_t,
        flags: c_int
    ) -> i64;

    pub(super) fn offkv_erase(
        handle: *mut c_void,
        key: *const c_char,
        version: i64,
    ) -> c_int;

    pub(super) fn offkv_set(
        handle: *mut c_void,
        key: *const c_char,
        value: *const c_char,
        value_size: size_t,
    ) -> i64;

    pub(super) fn offkv_cas(
        handle: *mut c_void,
        key: *const c_char,
        value: *const c_char,
        value_size: size_t,
        version: i64,
    ) -> i64;

    pub(super) fn offkv_get(
        handle: *mut c_void,
        key: *const c_char,
        watch_handle: *mut *mut c_void,
    ) -> offkv_GetResult;

    pub(super) fn offkv_exists(
        handle: *mut c_void,
        key: *const c_char,
        watch_handle: *mut *mut c_void,
    ) -> i64;

    pub(super) fn offkv_children(
        handle: *mut c_void,
        key: *const c_char,
        watch_handle: *mut *mut c_void,
    ) -> offkv_ChildrenResult;

    pub(super) fn offkv_commit(
        handle: *mut c_void,
        checks: *const offkv_TxnCheck,
        nchecks: size_t,
        ops: *const offkv_TxnOp,
        nops: size_t,
        result: *mut offkv_TxnResult,
    ) -> c_int;

    pub(super) fn offkv_watch(watch_handle: *mut c_void);
    pub(super) fn offkv_watch_drop(watch_handle: *mut c_void);

    pub(super) fn offkv_close(handle: *mut c_void);
}

#[repr(C)]
pub(super) struct offkv_GetResult {
    pub(super) value: *mut c_char,
    pub(super) value_size: size_t,
    pub(super) version: i64,
}

#[repr(C)]
pub(super) struct offkv_ChildrenResult {
    pub(super) keys: *mut *mut c_char,
    pub(super) nkeys: size_t,
    pub(super) error_code: c_int,
}

#[repr(C)]
pub(super) struct offkv_TxnCheck {
    pub(super) key: *const c_char,
    pub(super) version: i64,
}

#[repr(C)]
pub(super) struct offkv_TxnOp {
    pub(super) op_kind: c_int,
    pub(super) flags: c_int,
    pub(super) key: *const c_char,
    pub(super) value: *const c_char,
    pub(super) value_size: size_t,
}

#[repr(C)]
pub(super) struct offkv_TxnOpResult {
    pub(super) op_kind: c_int,
    pub(super) version: i64,
}

#[repr(C)]
pub(super) struct offkv_TxnResult {
    pub(super) results: *mut offkv_TxnOpResult,
    pub(super) nresults: size_t,
    pub(super) failed_op: size_t,
}


#[allow(non_camel_case_types)]
// copied from clib.h
pub(super) const OFFKV_LEASE: c_int = 1 << 0;


// transaction operations
#[repr(C)]
#[allow(non_camel_case_types)]
// copied from clib.h
pub(super) enum OffkvTxnOpCode {
    OFFKV_OP_CREATE,
    OFFKV_OP_SET,
    OFFKV_OP_ERASE,
}


pub(super) fn to_cstring(s: &str) -> CString {
    CString::new(s).expect("Failed to create CString")
}
