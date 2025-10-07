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
pub enum OverrideType {
    None,
    Insecure,
    Unverified,
    Tls,
}

#[derive(PartialEq, Eq, Clone)]
#[repr(C)]
pub struct ProtosocketCredentialProvider {
    pub(crate) env_var_name: *const c_char,
    pub(crate) override_type: OverrideType,
    pub(crate) override_endpoint: *const c_char,
}

impl ProtosocketCredentialProvider {
    #[unsafe(no_mangle)]
    pub extern "C" fn new_protosocket_credential_provider(
        env_var_name: *const c_char,
        override_type: OverrideType,
        override_endpoint: *const c_char,
    ) -> ProtosocketCredentialProvider {
        ProtosocketCredentialProvider {
            env_var_name,
            override_type,
            override_endpoint,
        }
    }
}
