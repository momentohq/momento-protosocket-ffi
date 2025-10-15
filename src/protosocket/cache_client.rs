use std::ffi::CString;
use std::sync::Arc;
use std::time::Duration;

use libc::c_char;
use libc::c_uchar;
use libc::c_ulonglong;
use momento::CredentialProvider;
use momento::ProtosocketCacheClient;

use crate::protosocket::configuration::{
    ProtosocketClientConfiguration, ProtosocketCredentialProvider,
};

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

    let api_key = unsafe {
        std::ffi::CStr::from_ptr(credential_provider.api_key)
            .to_string_lossy()
            .into_owned()
    };
    let creds = CredentialProvider::from_string(api_key).expect("auth token should be valid");

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

    unsafe {
        PROTOSOCKET_CLIENT = Box::into_raw(Box::new(client));
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

    // Free the overall struct
    let _ = Box::from_raw(result as *mut ProtosocketResult);
}

#[derive(Debug)]
#[repr(C)]
pub struct Bytes {
    pub data: *const c_uchar,
    pub length: usize,
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
pub enum ProtosocketResponseType {
    SetSuccess,
    GetHit,
    GetMiss,
    Error,
}

struct SendPtr(*mut std::ffi::c_void);
unsafe impl Send for SendPtr {}

pub type ProtosocketCallback =
    unsafe extern "C" fn(result: *mut ProtosocketResult, user_data: *mut std::ffi::c_void);

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
/// * `cache_name` must be a valid, non-null pointer to a null-terminated C string
/// * `key` must be a valid, non-null pointer to a `Bytes` struct
/// * `value` must be a valid, non-null pointer to a `Bytes` struct
/// * `callback` must be a valid function pointer
/// * `callback_data` must be a valid pointer that remains valid until the callback is invoked
/// * The caller is responsible for freeing the `ProtosocketResult` passed to the callback
///   by calling `free_response()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn protosocket_cache_client_set(
    cache_name: *const c_char,
    key: *const Bytes,
    value: *const Bytes,
    callback: ProtosocketCallback,
    callback_data: *mut std::ffi::c_void,
) {
    if cache_name.is_null() {
        let error_result = ProtosocketResult {
            response_type: ProtosocketResponseType::Error.into(),
            value: std::ptr::null(),
            error_message: CString::new("cache_name is null")
                .expect("failed to convert to CString")
                .into_raw(),
        };
        callback(Box::into_raw(Box::new(error_result)), callback_data);
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
        callback(Box::into_raw(Box::new(error_result)), callback_data);
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
        callback(Box::into_raw(Box::new(error_result)), callback_data);
        return;
    }

    unsafe {
        let cache_name = std::ffi::CStr::from_ptr(cache_name)
            .to_string_lossy()
            .into_owned();
        let key = std::slice::from_raw_parts((*key).data, (*key).length);
        let value = std::slice::from_raw_parts((*value).data, (*value).length);

        let callback_data = SendPtr(callback_data);

        RUNTIME_HANDLE.clone().spawn(async move {
            let client = PROTOSOCKET_CLIENT
                .as_ref()
                .expect("PROTOSOCKET_CLIENT is null");
            let result = client.set(&cache_name, key, value).await;

            let proto_result: ProtosocketResult = result.into();

            callback(Box::into_raw(Box::new(proto_result)), callback_data.0);
        });
    }
}

/// # Safety
///
/// * `cache_name` must be a valid, non-null pointer to a null-terminated C string
/// * `cache_name` must remain valid for the duration of this function call
/// * `key` must be a valid, non-null pointer to a `Bytes` struct
/// * `callback` must be a valid function pointer
/// * `callback_data` must be a valid pointer that remains valid until the callback is invoked
/// * The caller is responsible for freeing the `ProtosocketResult` passed to the callback
///   by calling `free_response()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn protosocket_cache_client_get(
    cache_name: *const c_char,
    key: *const Bytes,
    callback: ProtosocketCallback,
    callback_data: *mut std::ffi::c_void,
) {
    if cache_name.is_null() {
        let error_result = ProtosocketResult {
            response_type: ProtosocketResponseType::Error.into(),
            value: std::ptr::null(),
            error_message: CString::new("cache_name is null")
                .expect("failed to convert to CString")
                .into_raw(),
        };
        callback(Box::into_raw(Box::new(error_result)), callback_data);
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
        callback(Box::into_raw(Box::new(error_result)), callback_data);
        return;
    }

    unsafe {
        let cache_name = std::ffi::CStr::from_ptr(cache_name)
            .to_string_lossy()
            .into_owned();
        let key = std::slice::from_raw_parts((*key).data, (*key).length);

        let callback_data = SendPtr(callback_data);

        RUNTIME_HANDLE.clone().spawn(async move {
            let client = PROTOSOCKET_CLIENT
                .as_ref()
                .expect("PROTOSOCKET_CLIENT is null");
            let result = client.get(&cache_name, key).await;

            let proto_result: ProtosocketResult = result.into();

            callback(Box::into_raw(Box::new(proto_result)), callback_data.0);
        });
    }
}
