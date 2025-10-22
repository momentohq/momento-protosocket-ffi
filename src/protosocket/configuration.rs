use safer_ffi::__::char_p;
use safer_ffi::{derive_ReprC, ffi_export};
use std::convert::TryInto;
use std::fmt;

#[derive_ReprC]
#[repr(C)]
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ProtosocketClientConfiguration {
    pub(crate) timeout_millis: usize,
    pub(crate) connection_count: usize,
}

#[ffi_export]
pub fn new_protosocket_client_configuration(
    timeout_millis: usize,
    connection_count: usize,
) -> ProtosocketClientConfiguration {
    ProtosocketClientConfiguration {
        timeout_millis,
        connection_count,
    }
}

#[derive_ReprC]
#[repr(C)]
#[derive(Clone)]
pub struct ProtosocketCredentialProvider {
    pub(crate) api_key: char_p::Box,
}

#[ffi_export]
pub fn new_protosocket_credential_provider(
    api_key: char_p::Ref<'_>,
) -> ProtosocketCredentialProvider {
    ProtosocketCredentialProvider {
        api_key: api_key
            .to_str()
            .to_string()
            .try_into()
            .expect("API Key should not contain null bytes"),
    }
}

impl fmt::Debug for ProtosocketCredentialProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProtosocketCredentialProvider")
            .field("api_key", &"<redacted>")
            .finish()
    }
}
