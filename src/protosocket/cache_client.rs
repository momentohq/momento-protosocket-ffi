use std::ffi::CString;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use libc::c_char;
use libc::c_uchar;
use libc::c_ulonglong;
use momento::CredentialProvider;
use momento::ProtosocketCacheClient;
use tokio::sync::mpsc;
use zerocopy::byte_slice::IntoByteSlice;

use crate::protosocket::configuration::{
    ProtosocketClientConfiguration, ProtosocketCredentialProvider,
};
use crate::protosocket::inner::{from_get_result_to_inner_protosocket_result, from_set_result_to_inner_protosocket_result, handle_received, ProtosocketSetRequest};
use crate::protosocket::inner::InnerProtosocketResult;
use crate::protosocket::inner::ProcessingResult;
use crate::protosocket::inner::ProtosocketGetRequest;
use crate::protosocket::inner::ProtosocketRequestType;

use once_cell::sync::Lazy;
use tokio::runtime::{self, Runtime};

// seems best to ensure the tokio runtime and the protosocket client do not cross the FFI boundary

pub(crate) static RUNTIME: Lazy<Arc<Runtime>> = Lazy::new(|| {
    tokio_rustls::rustls::crypto::CryptoProvider::install_default(
        tokio_rustls::rustls::crypto::aws_lc_rs::default_provider(),
    )
    .expect("Error installing default crypto provider");

    Arc::new(
        runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to create runtime"),
    )
});

pub(crate) static RUNTIME_HANDLE: Lazy<tokio::runtime::Handle> =
    Lazy::new(|| tokio::runtime::Handle::current());

pub(crate) static mut PROTOSOCKET_CLIENT: *mut ProtosocketCacheClient = std::ptr::null_mut();
pub(crate) static mut REQUEST_SENDER: *mut mpsc::Sender<ProcessingResult> = std::ptr::null_mut();
pub(crate) static mut RESPONSE_SENDER: *mut mpsc::Sender<InnerProtosocketResult> =
    std::ptr::null_mut();

pub(crate) static OPERATION_COUNTER: AtomicU64 = AtomicU64::new(0);
pub(crate) static RESPONSES_ACCUMULATED: Lazy<DashMap<u64, InnerProtosocketResult>> =
    Lazy::new(DashMap::new);

// TODO: determine how large these channels should be (configurable or no?)
const REQUEST_CHANNEL_SIZE: usize = 1024;
const RESPONSE_CHANNEL_SIZE: usize = 1024;

pub struct ProtosocketCacheClientWrapper {
    pub client: *mut ProtosocketCacheClient,
}

#[unsafe(no_mangle)]
pub extern "C" fn init_protosocket_cache_client(
    item_default_ttl_millis: c_ulonglong,
    configuration: ProtosocketClientConfiguration,
    credential_provider: ProtosocketCredentialProvider,
) {
    let config = momento::protosocket::cache::Configuration::builder()
        .timeout(Duration::from_secs(configuration.timeout_millis as u64))
        .connection_count(configuration.connection_count as u32)
        .az_id(None)
        .build();

    let env_var_name = unsafe {
        std::ffi::CStr::from_ptr(credential_provider.env_var_name)
            .to_string_lossy()
            .into_owned()
    };
    let creds = CredentialProvider::from_env_var(env_var_name).expect("auth token should be valid");

    let client = RUNTIME.block_on(async {
        ProtosocketCacheClient::builder()
            .default_ttl(Duration::from_millis(item_default_ttl_millis))
            .configuration(config)
            .credential_provider(creds)
            .runtime(RUNTIME_HANDLE.clone())
            .build()
            .await
            .expect("failed to create client")
    });

    // Start background thread for processing requests
    let (requests_sender, mut requests_receiver) =
        mpsc::channel::<ProcessingResult>(REQUEST_CHANNEL_SIZE);
    RUNTIME_HANDLE.spawn(async move {
        while let Some(processing_result) = requests_receiver.recv().await {
            RUNTIME_HANDLE.spawn(async move {
                handle_received(processing_result).await;
            });
        }
    });

    // Start background thread for accumulating responses until FFI caller asks for them
    // since recv() is an async function
    let (responses_sender, mut responses_receiver) =
        mpsc::channel::<InnerProtosocketResult>(RESPONSE_CHANNEL_SIZE);
    RUNTIME_HANDLE.spawn(async move {
        while let Some(response) = responses_receiver.recv().await {
            let op_id = response.operation_id;
            (*RESPONSES_ACCUMULATED).insert(response.operation_id, response);
            println!(
                "[FFI INFO] inserted response for operation id: {:?}, remaining item count: {:?}",
                op_id,
                (*RESPONSES_ACCUMULATED).len()
            );
        }
    });

    unsafe {
        PROTOSOCKET_CLIENT = Box::into_raw(Box::new(client));
        REQUEST_SENDER = Box::into_raw(Box::new(requests_sender));
        RESPONSE_SENDER = Box::into_raw(Box::new(responses_sender));
    }
}

// not actually sure if we need this yet, but perhaps better to have it than not
// to avoid memory leaks in Go?
#[unsafe(no_mangle)]
pub extern "C" fn destroy_protosocket_cache_client() {
    unsafe {
        drop(Box::from_raw(PROTOSOCKET_CLIENT));
    }
}

/// # Safety
///
/// This function should be called when the response is no longer needed.
#[no_mangle]
pub unsafe extern "C" fn free_response(result: *const ProtosocketResult) {
    if result.is_null() {
        return;
    }

    unsafe {
        if (*result).response_type.is_null()
            && (*result).error_message.is_null()
            && (*result).value.is_null()
        {
            return;
        }

        // Free the response_type string if it's not null
        if !(*result).response_type.is_null() {
            let _ = CString::from_raw((*result).response_type as *mut c_char);
        }

        // Free the error_message string if it's not null
        if !(*result).error_message.is_null() {
            let _ = CString::from_raw((*result).error_message as *mut c_char);
        }

        // Free the value data and the Bytes struct if value is not null
        if !(*result).value.is_null() {
            let bytes_ptr = (*result).value;
            // Free the leaked slice data
            if !(*bytes_ptr).data.is_null() {
                let _ = Box::from_raw(core::ptr::slice_from_raw_parts_mut(
                    (*bytes_ptr).data as *mut u8,
                    (*bytes_ptr).length,
                ));
            }
            // Free the Bytes struct itself
            let _ = Box::from_raw(bytes_ptr as *mut Bytes);
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct Bytes {
    pub data: *const c_uchar,
    pub length: usize,
}

#[derive(Debug)]
#[repr(C)]
pub struct ProtosocketResponse {
    pub awaiting: *mut AwaitingResult,
    pub completed: *mut ProtosocketResult,
}

// TODO: verify this is the format we want to go with
#[derive(Debug)]
#[repr(C)]
pub struct ProtosocketResult {
    pub response_type: *const c_char,
    pub value: *const Bytes,
    pub error_message: *const c_char,
}

#[derive(Debug)]
#[repr(C)]
pub struct AwaitingResult {
    pub operation_id: c_ulonglong,
}

#[derive(Debug)]
#[repr(C)]
pub enum ProtosocketResponseType {
    SetSuccess,
    GetHit,
    GetMiss,
    Error,
}

struct SendPtr(*mut std::ffi::c_void);
unsafe impl Send for SendPtr {}

pub type ProtosocketCallback = unsafe extern "C" fn(
    result: *mut ProtosocketResult,
    user_data: *mut std::ffi::c_void,
);

impl From<ProtosocketResponseType> for *const c_char {
    fn from(response_type: ProtosocketResponseType) -> Self {
        let response_type = match response_type {
            ProtosocketResponseType::SetSuccess => "SetSuccess",
            ProtosocketResponseType::GetHit => "GetHit",
            ProtosocketResponseType::GetMiss => "GetMiss",
            ProtosocketResponseType::Error => "Error",
        };
        let c_string = CString::new(response_type).expect("failed to convert to CString");
        c_string.into_raw()
    }
}

/// # Safety
///
/// This function expects C-allocated strings and bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn protosocket_cache_client_set(
    cache_name: *const c_char,
    key: *const Bytes,
    value: *const Bytes,
) -> ProtosocketResponse {
    let mut response = ProtosocketResponse {
        awaiting: std::ptr::null_mut(),
        completed: std::ptr::null_mut(),
    };

    if cache_name.is_null() {
        let completed = ProtosocketResult {
            response_type: ProtosocketResponseType::Error.into(),
            value: std::ptr::null(),
            error_message: CString::new("cache_name is null")
                .expect("failed to convert to CString")
                .into_raw(),
        };
        response.completed = Box::into_raw(Box::new(completed));
        return response;
    }
    if key.is_null() {
        let completed = ProtosocketResult {
            response_type: ProtosocketResponseType::Error.into(),
            value: std::ptr::null(),
            error_message: CString::new("key is null")
                .expect("failed to convert to CString")
                .into_raw(),
        };
        response.completed = Box::into_raw(Box::new(completed));
        return response;
    }
    if value.is_null() {
        let completed = ProtosocketResult {
            response_type: ProtosocketResponseType::Error.into(),
            value: std::ptr::null(),
            error_message: CString::new("key is null")
                .expect("failed to convert to CString")
                .into_raw(),
        };
        response.completed = Box::into_raw(Box::new(completed));
        return response;
    }

    unsafe {
        let awaiting_result = AwaitingResult {
            operation_id: OPERATION_COUNTER.fetch_add(1, Ordering::Relaxed),
        };
        let cache_name = std::ffi::CStr::from_ptr(cache_name)
            .to_string_lossy()
            .into_owned();
        let key = std::slice::from_raw_parts((*key).data, (*key).length);
        let value = std::slice::from_raw_parts((*value).data, (*value).length);

        let request = ProtosocketRequestType::Set(ProtosocketSetRequest {
            cache_name,
            key: key.into_byte_slice().to_vec(),
            value: value.into_byte_slice().to_vec(),
        });
        let processing_result = ProcessingResult {
            request,
            operation_id: awaiting_result.operation_id,
        };

        RUNTIME_HANDLE.clone().spawn(async move {
            let sender = REQUEST_SENDER.as_ref().expect("REQUEST_SENDER is null");
            sender
                .send(processing_result)
                .await
                .expect("failed to send message");
        });

        response.awaiting = Box::into_raw(Box::new(awaiting_result));
        response
    }
}

/// # Safety
///
/// This function expects C-allocated strings and bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn protosocket_cache_client_get(
    cache_name: *const c_char,
    key: *const Bytes,
) -> ProtosocketResponse {
    let mut response = ProtosocketResponse {
        awaiting: std::ptr::null_mut(),
        completed: std::ptr::null_mut(),
    };

    if cache_name.is_null() {
        let completed = ProtosocketResult {
            response_type: ProtosocketResponseType::Error.into(),
            value: std::ptr::null(),
            error_message: CString::new("cache_name is null")
                .expect("failed to convert to CString")
                .into_raw(),
        };
        response.completed = Box::into_raw(Box::new(completed));
        return response;
    }
    if key.is_null() {
        let completed = ProtosocketResult {
            response_type: ProtosocketResponseType::Error.into(),
            value: std::ptr::null(),
            error_message: CString::new("key is null")
                .expect("failed to convert to CString")
                .into_raw(),
        };
        response.completed = Box::into_raw(Box::new(completed));
        return response;
    }

    unsafe {
        let awaiting_result = AwaitingResult {
            operation_id: OPERATION_COUNTER.fetch_add(1, Ordering::Relaxed),
        };
        let cache_name = std::ffi::CStr::from_ptr(cache_name)
            .to_string_lossy()
            .into_owned();
        let key = std::slice::from_raw_parts((*key).data, (*key).length);

        // this is for passing to the async thread so it can do the async work
        let request = ProtosocketRequestType::Get(ProtosocketGetRequest {
            cache_name,
            key: key.into_byte_slice().to_vec(),
        });
        let processing_result = ProcessingResult {
            request,
            operation_id: awaiting_result.operation_id,
        };

        RUNTIME_HANDLE.clone().spawn(async move {
            let sender = REQUEST_SENDER.as_ref().expect("REQUEST_SENDER is null");
            sender
                .send(processing_result)
                .await
                .expect("failed to send message");
        });

        response.awaiting = Box::into_raw(Box::new(awaiting_result));
        response
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn protosocket_cache_client_poll_responses(
    operation_id: c_ulonglong,
) -> *mut ProtosocketResult {
    // check if the requested response is in the DashMap
    if let Some((_, response)) = (*RESPONSES_ACCUMULATED).remove(&operation_id) {
        println!(
            "[FFI INFO] found response for operation id: {:?}: {:?}",
            operation_id, response
        );
        return Box::into_raw(Box::new(response.into()));
    }
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn protosocket_cache_client_set_with_callback(
    cache_name: *const c_char,
    key: *const Bytes,
    value: *const Bytes,
    callback: ProtosocketCallback,
    user_data: *mut std::ffi::c_void,
) {
    if cache_name.is_null() {
        let error_result = ProtosocketResult {
            response_type: ProtosocketResponseType::Error.into(),
            value: std::ptr::null(),
            error_message: CString::new("cache_name is null")
                .expect("failed to convert to CString")
                .into_raw(),
        };
        callback(Box::into_raw(Box::new(error_result)), user_data);
        return;
    }
    if key.is_null() {
        let error_result = ProtosocketResult {
            response_type: ProtosocketResponseType::Error.into(),
            value: std::ptr::null(),
            error_message: CString::new("key is null")
                .expect("failed to convert to CString")
                .into_raw(),
        };
        callback(Box::into_raw(Box::new(error_result)), user_data);
        return;
    }
    if value.is_null() {
        let error_result = ProtosocketResult {
            response_type: ProtosocketResponseType::Error.into(),
            value: std::ptr::null(),
            error_message: CString::new("value is null")
                .expect("failed to convert to CString")
                .into_raw(),
        };
        callback(Box::into_raw(Box::new(error_result)), user_data);
        return;
    }

    unsafe {
        let cache_name = std::ffi::CStr::from_ptr(cache_name)
            .to_string_lossy()
            .into_owned();
        let key = std::slice::from_raw_parts((*key).data, (*key).length);
        let value = std::slice::from_raw_parts((*value).data, (*value).length);

        let user_data = SendPtr(user_data);

        RUNTIME_HANDLE.clone().spawn(async move {
            let client = PROTOSOCKET_CLIENT.as_ref().expect("PROTOSOCKET_CLIENT is null");
            let result = client.set(&cache_name, key, value).await;

            let inner_result = from_set_result_to_inner_protosocket_result(result, 0);
            let proto_result: ProtosocketResult = inner_result.into();

            callback(Box::into_raw(Box::new(proto_result)), user_data.0);
        });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn protosocket_cache_client_get_with_callback(
    cache_name: *const c_char,
    key: *const Bytes,
    callback: ProtosocketCallback,
    user_data: *mut std::ffi::c_void,
) {
    if cache_name.is_null() {
        let error_result = ProtosocketResult {
            response_type: ProtosocketResponseType::Error.into(),
            value: std::ptr::null(),
            error_message: CString::new("cache_name is null")
                .expect("failed to convert to CString")
                .into_raw(),
        };
        callback(Box::into_raw(Box::new(error_result)), user_data);
        return;
    }
    if key.is_null() {
        let error_result = ProtosocketResult {
            response_type: ProtosocketResponseType::Error.into(),
            value: std::ptr::null(),
            error_message: CString::new("key is null")
                .expect("failed to convert to CString")
                .into_raw(),
        };
        callback(Box::into_raw(Box::new(error_result)), user_data);
        return;
    }

    unsafe {
        let cache_name = std::ffi::CStr::from_ptr(cache_name)
            .to_string_lossy()
            .into_owned();
        let key = std::slice::from_raw_parts((*key).data, (*key).length);

        let user_data = SendPtr(user_data);

        RUNTIME_HANDLE.clone().spawn(async move {
            let client = PROTOSOCKET_CLIENT.as_ref().expect("PROTOSOCKET_CLIENT is null");
            let result = client.get(&cache_name, key).await;

            let inner_result = from_get_result_to_inner_protosocket_result(result, 0);
            let proto_result: ProtosocketResult = inner_result.into();

            callback(Box::into_raw(Box::new(proto_result)), user_data.0);
        });
    }
}
