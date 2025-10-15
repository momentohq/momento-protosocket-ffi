use std::fmt;
use libc::c_char;

#[derive(PartialEq, Eq, Clone, Debug)]
#[repr(C)]
pub struct ProtosocketClientConfiguration {
    pub(crate) timeout_millis: usize,
    pub(crate) connection_count: usize,
}

impl ProtosocketClientConfiguration {
    #[unsafe(no_mangle)]
    pub extern "C" fn new_protosocket_client_configuration(
        timeout_millis: usize,
        connection_count: usize,
    ) -> ProtosocketClientConfiguration {
        ProtosocketClientConfiguration {
            timeout_millis,
            connection_count,
        }
    }
}

#[derive(PartialEq, Eq, Clone)]
#[repr(C)]
pub struct ProtosocketCredentialProvider {
    pub(crate) api_key: *const c_char,
}

impl fmt::Debug for ProtosocketCredentialProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProtosocketCredentialProvider")
            .field("api_key", &"<redacted>")
            .finish()
    }
}

impl ProtosocketCredentialProvider {
    #[unsafe(no_mangle)]
    pub extern "C" fn new_protosocket_credential_provider(
        api_key: *const c_char,
    ) -> ProtosocketCredentialProvider {
        ProtosocketCredentialProvider { api_key }
    }
}
