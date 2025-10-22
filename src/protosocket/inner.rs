use momento::{
    cache::{GetResponse, SetResponse},
    MomentoError,
};
use std::convert::TryInto;

use crate::protosocket::cache_client::{Bytes, ProtosocketResponseType, ProtosocketResult};

impl From<Result<SetResponse, MomentoError>> for ProtosocketResult {
    fn from(value: Result<SetResponse, MomentoError>) -> Self {
        match value {
            Ok(_) => ProtosocketResult {
                response_type: ProtosocketResponseType::SetSuccess.into(),
                value: None,
                error_message: None,
            },
            Err(error) => ProtosocketResult {
                response_type: ProtosocketResponseType::Error.into(),
                value: None,
                error_message: Some(
                    error
                        .to_string()
                        .try_into()
                        .expect("Error message contains null byte"),
                ),
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
                    value: Some(Box::new(bytes).into()),
                    error_message: None,
                }
            }
            Ok(GetResponse::Miss) => ProtosocketResult {
                response_type: ProtosocketResponseType::GetMiss.into(),
                value: None,
                error_message: None,
            },
            Err(error) => ProtosocketResult {
                response_type: ProtosocketResponseType::Error.into(),
                value: None,
                error_message: Some(
                    error
                        .to_string()
                        .try_into()
                        .expect("Error message contains null byte"),
                ),
            },
        }
    }
}
