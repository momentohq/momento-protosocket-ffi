pub mod protosocket;

#[cfg(feature = "headers")]
pub fn generate_headers() -> ::std::io::Result<()> {
    ::safer_ffi::headers::builder()
        .to_file("target/momento_protosocket_ffi.h")?
        .generate()
}
