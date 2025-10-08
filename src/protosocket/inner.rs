use libc::c_ulonglong;
use momento::{
    cache::{GetResponse, SetResponse},
    MomentoError,
};

use crate::protosocket::cache_client::{
    Bytes, ProtosocketResponseType, ProtosocketResult, PROTOSOCKET_CLIENT, RESPONSE_SENDER,
};

pub enum ProtosocketRequestType {
    Set(ProtosocketSetRequest),
    Get(ProtosocketGetRequest),
}

pub struct ProtosocketSetRequest {
    pub cache_name: Box<String>,
    pub key: Box<Vec<u8>>,
    pub value: Box<Vec<u8>>,
}

pub struct ProtosocketGetRequest {
    pub cache_name: Box<String>,
    pub key: Box<Vec<u8>>,
}

#[repr(C)]
pub struct ProcessingResult {
    pub request: ProtosocketRequestType,
    pub operation_id: c_ulonglong,
}

#[derive(Debug)]
pub struct InnerProtosocketResult {
    pub response_type: ProtosocketResponseType,
    pub value: Vec<u8>,
    pub error_message: String,
    pub operation_id: u64,
}

impl From<InnerProtosocketResult> for ProtosocketResult {
    fn from(result: InnerProtosocketResult) -> Self {
        match result.response_type {
            ProtosocketResponseType::SetSuccess => ProtosocketResult {
                response_type: ProtosocketResponseType::SetSuccess.into(),
                value: std::ptr::null(),
                error_message: std::ptr::null(),
            },
            ProtosocketResponseType::GetHit => {
                let item: Vec<u8> = result.value.into();
                let item_len = item.len();
                // Use Box::leak to prevent the Vec from being dropped
                // This memory will be freed when free_response is called
                let leaked_item = Box::leak(item.into_boxed_slice());
                let bytes = Bytes {
                    data: leaked_item.as_ptr(),
                    length: item_len,
                };
                println!(
                    "[FFI INFO] created bytes: {:?} | data: {:?}",
                    bytes,
                    unsafe { std::slice::from_raw_parts(bytes.data, bytes.length) }
                );
                ProtosocketResult {
                    response_type: ProtosocketResponseType::GetHit.into(),
                    value: Box::into_raw(Box::new(bytes)),
                    error_message: std::ptr::null(),
                }
            }
            ProtosocketResponseType::GetMiss => ProtosocketResult {
                response_type: ProtosocketResponseType::GetMiss.into(),
                value: std::ptr::null(),
                error_message: std::ptr::null(),
            },
            ProtosocketResponseType::Error => ProtosocketResult {
                response_type: ProtosocketResponseType::Error.into(),
                value: std::ptr::null(),
                error_message: std::ptr::null(),
            },
        }
    }
}

fn from_set_result_to_inner_protosocket_result(
    result: Result<SetResponse, MomentoError>,
    operation_id: u64,
) -> InnerProtosocketResult {
    match result {
        Ok(_) => InnerProtosocketResult {
            response_type: ProtosocketResponseType::SetSuccess,
            value: vec![],
            error_message: "".to_string(),
            operation_id,
        },
        Err(error) => InnerProtosocketResult {
            response_type: ProtosocketResponseType::Error,
            value: vec![],
            error_message: error.to_string(),
            operation_id,
        },
    }
}

fn from_get_result_to_inner_protosocket_result(
    result: Result<GetResponse, MomentoError>,
    operation_id: u64,
) -> InnerProtosocketResult {
    match result {
        Ok(GetResponse::Hit { value }) => InnerProtosocketResult {
            response_type: ProtosocketResponseType::GetHit,
            value: value.into(),
            error_message: "".to_string(),
            operation_id,
        },
        Ok(GetResponse::Miss) => InnerProtosocketResult {
            response_type: ProtosocketResponseType::GetMiss,
            value: vec![],
            error_message: "".to_string(),
            operation_id,
        },
        Err(error) => InnerProtosocketResult {
            response_type: ProtosocketResponseType::Error,
            value: vec![],
            error_message: error.to_string(),
            operation_id,
        },
    }
}

pub(crate) async fn handle_received(processing_result: ProcessingResult) {
    match processing_result.request {
        ProtosocketRequestType::Set(set_request) => {
            println!(
                "[FFI INFO] Received a Set request: cache_name: {:?}, key: {:?}, value: {:?}",
                set_request.cache_name, set_request.key, set_request.value
            );
            let result = unsafe {
                (*PROTOSOCKET_CLIENT)
                    .set(
                        *set_request.cache_name,
                        *set_request.key,
                        *set_request.value,
                    )
                    .await
            };
            println!("\n[FFI INFO] Set result: {:?}", result);
            unsafe {
                (*RESPONSE_SENDER)
                    .send(from_set_result_to_inner_protosocket_result(
                        result,
                        processing_result.operation_id,
                    ))
                    .await
                    .expect("failed to send message");
            }
        }
        ProtosocketRequestType::Get(get_request) => {
            println!(
                "[FFI INFO] Received a Get request: cache_name: {:?}, key: {:?}",
                get_request.cache_name, get_request.key
            );
            let result = unsafe {
                (*PROTOSOCKET_CLIENT)
                    .get(*get_request.cache_name, *get_request.key)
                    .await
            };
            println!("\n[FFI INFO] Get result: {:?}", result);
            unsafe {
                (*RESPONSE_SENDER)
                    .send(from_get_result_to_inner_protosocket_result(
                        result,
                        processing_result.operation_id,
                    ))
                    .await
                    .expect("failed to send message");
            }
        }
    }
}
