use momento::{
    cache::{GetResponse, SetResponse},
    MomentoError,
};
use std::ffi::CString;

use crate::protosocket::cache_client::{Bytes, ProtosocketResponseType, ProtosocketResult};

impl From<Result<SetResponse, MomentoError>> for ProtosocketResult {
    fn from(value: Result<SetResponse, MomentoError>) -> Self {
        match value {
            Ok(_) => ProtosocketResult {
                response_type: ProtosocketResponseType::SetSuccess.into(),
                value: std::ptr::null(),
                error_message: std::ptr::null(),
            },
            Err(error) => ProtosocketResult {
                response_type: ProtosocketResponseType::Error.into(),
                value: std::ptr::null(),
                error_message: CString::new(error.to_string())
                    .unwrap_or_else(|_| unsafe {
                        CString::new("Error message contained null byte").unwrap_unchecked()
                    })
                    .into_raw(),
            },
        }
    }
}

impl From<Result<GetResponse, MomentoError>> for ProtosocketResult {
    fn from(value: Result<GetResponse, MomentoError>) -> Self {
        match value {
            Ok(GetResponse::Hit { value }) => {
                let value: Vec<u8> = value.into();
                let item_len = value.len();
                // Use Box::leak to prevent the Vec from being dropped
                // This memory will be freed when free_response is called
                let leaked_item = Box::leak(value.into_boxed_slice());
                let bytes = Bytes {
                    data: leaked_item.as_ptr(),
                    length: item_len,
                };
                ProtosocketResult {
                    response_type: ProtosocketResponseType::GetHit.into(),
                    value: Box::into_raw(Box::new(bytes)),
                    error_message: std::ptr::null(),
                }
            }
            Ok(GetResponse::Miss) => ProtosocketResult {
                response_type: ProtosocketResponseType::GetMiss.into(),
                value: std::ptr::null(),
                error_message: std::ptr::null(),
            },
            Err(error) => ProtosocketResult {
                response_type: ProtosocketResponseType::Error.into(),
                value: std::ptr::null(),
                error_message: CString::new(error.to_string())
                    .unwrap_or_else(|_| unsafe {
                        CString::new("Error message contained null byte").unwrap_unchecked()
                    })
                    .into_raw(),
            },
        }
    }
}
