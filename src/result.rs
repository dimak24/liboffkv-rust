use std::{fmt,error};


pub enum OffkvError {
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


pub(crate) fn from_error_code(error_code: i64) -> Option<OffkvError> {
    match error_code {
        x if x == OffkvErrorCode::OFFKV_EEXIST as i64 => Some(OffkvError::EntryExists),
        x if x == OffkvErrorCode::OFFKV_ENOENT as i64 => Some(OffkvError::NoEntry),
        x if x == OffkvErrorCode::OFFKV_EADDR as i64 => Some(OffkvError::InvalidAddress),
        x if x == OffkvErrorCode::OFFKV_EKEY as i64 => Some(OffkvError::InvalidKey),
        x if x == OffkvErrorCode::OFFKV_ETXN as i64 => Some(OffkvError::TxnFailed(0)),
        _ => None,
    }
}

pub type Result<T> = std::result::Result<T, OffkvError>;
