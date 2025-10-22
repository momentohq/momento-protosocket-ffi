use std::convert::TryInto;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use libc::c_ulonglong;
use momento::CredentialProvider;
use momento::ProtosocketCacheClient;

use crate::protosocket::configuration::{
    ProtosocketClientConfiguration, ProtosocketCredentialProvider,
};

use once_cell::sync::Lazy;
use safer_ffi::prelude::{char_p, repr_c};
use safer_ffi::{derive_ReprC, ffi_export};
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

pub(crate) static PROTOSOCKET_CLIENT: OnceLock<ProtosocketCacheClient> = OnceLock::new();

#[ffi_export]
pub fn init_protosocket_cache_client(
    item_default_ttl_millis: c_ulonglong,
    configuration: ProtosocketClientConfiguration,
    credential_provider: ProtosocketCredentialProvider,
) {
    let config = momento::protosocket::cache::Configuration::builder()
        .timeout(Duration::from_millis(configuration.timeout_millis as u64))
        .connection_count(configuration.connection_count as u32)
        .az_id(None)
        .build();

    let api_key = credential_provider.api_key.to_str();
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

    PROTOSOCKET_CLIENT
        .set(client)
        .expect("Client already initialized");
}

#[derive_ReprC]
#[repr(C)]
#[derive(Debug)]
pub struct Bytes {
    pub data: *const u8,
    pub length: usize,
}

// TODO: verify this is the format we want to go with
#[derive_ReprC]
#[repr(C)]
#[derive(Debug)]
pub struct ProtosocketResult {
    pub response_type: char_p::Box,
    pub value: Option<repr_c::Box<Bytes>>,
    pub error_message: Option<char_p::Box>,
}

#[ffi_export]
pub fn free_response(result: repr_c::Box<ProtosocketResult>) {
    drop(result);
}

#[derive_ReprC]
#[repr(u8)]
#[derive(Debug)]
pub enum ProtosocketResponseType {
    SetSuccess,
    GetHit,
    GetMiss,
    Error,
}

impl From<ProtosocketResponseType> for char_p::Box {
    fn from(response_type: ProtosocketResponseType) -> Self {
        let response_type_str = match response_type {
            ProtosocketResponseType::SetSuccess => "SetSuccess",
            ProtosocketResponseType::GetHit => "GetHit",
            ProtosocketResponseType::GetMiss => "GetMiss",
            ProtosocketResponseType::Error => "Error",
        };
        response_type_str
            .to_string()
            .try_into()
            .expect("ResponseType string contained null byte")
    }
}

struct SendPtr(*mut std::ffi::c_void);
unsafe impl Send for SendPtr {}

pub type ProtosocketCallback =
    unsafe extern "C" fn(result: repr_c::Box<ProtosocketResult>, user_data: *mut std::ffi::c_void);

#[ffi_export]
pub fn protosocket_cache_client_set(
    cache_name: char_p::Ref<'_>,
    key: &Bytes,
    value: &Bytes,
    callback: ProtosocketCallback,
    callback_data: *mut std::ffi::c_void,
) {
    let cache_name = cache_name.to_str().to_owned();

    // The caller must provide valid pointers and lengths
    // TODO: is it worth skipping the copy here and trusting the caller to keep this data alive
    // for the duration of the function call?
    let key = unsafe { std::slice::from_raw_parts(key.data, key.length) }.to_vec();
    let value = unsafe { std::slice::from_raw_parts(value.data, value.length) }.to_vec();

    let callback_data = SendPtr(callback_data);

    RUNTIME_HANDLE.clone().spawn(async move {
        let client = PROTOSOCKET_CLIENT.get().expect("Client not initialized");

        let result = client.set(&cache_name, key, value).await;
        let proto_result: ProtosocketResult = result.into();

        unsafe {
            callback(Box::new(proto_result).into(), callback_data.0);
        }
    });
}

#[ffi_export]
pub fn protosocket_cache_client_get(
    cache_name: char_p::Ref<'_>,
    key: &Bytes,
    callback: ProtosocketCallback,
    callback_data: *mut std::ffi::c_void,
) {
    let cache_name = cache_name.to_str().to_owned();
    // The caller must provide valid pointers and lengths
    // TODO: is it worth skipping the copy here and trusting the caller to keep this data alive
    // for the duration of the function call?
    let key = unsafe { std::slice::from_raw_parts(key.data, key.length) }.to_vec();

    let callback_data = SendPtr(callback_data);

    RUNTIME_HANDLE.clone().spawn(async move {
        let client = PROTOSOCKET_CLIENT.get().expect("Client not initialized");

        let result = client.get(&cache_name, key).await;
        let proto_result: ProtosocketResult = result.into();

        unsafe {
            callback(Box::new(proto_result).into(), callback_data.0);
        }
    });
}
