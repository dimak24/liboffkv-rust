use std::{fmt,error};
use libc::{c_int,c_char};
use std::ffi::CString;


#[repr(C)]
#[allow(non_camel_case_types)]
// copied from clib.h
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

extern "C" {
    fn offkv_error_descr(error_code: c_int) -> *const c_char;
}


/// Rsoffkv errors.
#[derive(Debug)]
pub enum OffkvError {
    /// returned if address given to `Client::new` is invalid
    /// (address must be of form `<service_name>://<host>:<port>`)
    InvalidAddress,

    /// returned if the key given to any function is invalid
    /// key must consist of ASCII characters and be of form `(/[^/].+)+`
    InvalidKey,

    /// returned when trying to perform an operation requiring key to exist on
    /// non-existing entry OR if trying to create new key when its direct parent
    /// doesn't exist
    NoEntry,

    /// returned when trying to perform an operation requiring key to not exist on
    /// existing entry
    EntryExists,

    /// returned when trying to create children for leased node
    NoChildrenForEphemeral,

    /// can be returned from any function if the connection with service was lost
    ConnectionLost,

    /// returned from commit if the transaction was failed (not all checks are satisfied
    /// or any operation failed)
    ///
    /// contains an index of failed operation
    TxnFailed(u32),

    /// can be returned from any function in case of some service specific errors
    ServiceError,

    /// can be returned from any function
    OutOfMemory,
}


pub(crate) fn from_error_code(error_code: i64) -> Option<OffkvError> {
    match error_code {
        x if x == OffkvErrorCode::OFFKV_EADDR as i64 => Some(OffkvError::InvalidAddress),
        x if x == OffkvErrorCode::OFFKV_EKEY as i64 => Some(OffkvError::InvalidKey),
        x if x == OffkvErrorCode::OFFKV_ENOENT as i64 => Some(OffkvError::NoEntry),
        x if x == OffkvErrorCode::OFFKV_EEXIST as i64 => Some(OffkvError::EntryExists),
        x if x == OffkvErrorCode::OFFKV_EEPHEM as i64 => Some(OffkvError::NoChildrenForEphemeral),
        x if x == OffkvErrorCode::OFFKV_ECONN as i64 => Some(OffkvError::ConnectionLost),
        x if x == OffkvErrorCode::OFFKV_ETXN as i64 => Some(OffkvError::TxnFailed(0)),
        x if x == OffkvErrorCode::OFFKV_ESRV as i64 => Some(OffkvError::ServiceError),
        x if x == OffkvErrorCode::OFFKV_ENOMEM as i64 => Some(OffkvError::OutOfMemory),
        _ => None,
    }
}

fn to_error_code(error: &OffkvError) -> c_int {
    (match *error {
        OffkvError::InvalidAddress => OffkvErrorCode::OFFKV_EADDR,
        OffkvError::InvalidKey => OffkvErrorCode::OFFKV_EKEY,
        OffkvError::NoEntry => OffkvErrorCode::OFFKV_ENOENT,
        OffkvError::EntryExists => OffkvErrorCode::OFFKV_EEXIST,
        OffkvError::NoChildrenForEphemeral => OffkvErrorCode::OFFKV_EEPHEM,
        OffkvError::ConnectionLost => OffkvErrorCode::OFFKV_ECONN,
        OffkvError::TxnFailed(_) => OffkvErrorCode::OFFKV_ETXN,
        OffkvError::ServiceError => OffkvErrorCode::OFFKV_ESRV,
        OffkvError::OutOfMemory => OffkvErrorCode::OFFKV_ENOMEM,
    }) as c_int
}


impl fmt::Display for OffkvError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let descr = unsafe {
            CString::from_raw(offkv_error_descr(to_error_code(&*self)) as *mut c_char)
        }.into_string().unwrap();

        match &*self {
            OffkvError::TxnFailed(index)
                => write!(f, "{} (failed operation index: {})", descr, index),
            _ => write!(f, "{}", descr),
        }
    }
}

impl error::Error for OffkvError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}
