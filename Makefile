build:
	cargo fmt
	MACOSX_DEPLOYMENT_TARGET=15.0 RUSTFLAGS='--print=native-static-libs -C strip=symbols' cargo build --release

build-go-example: build
	cp ./target/release/libmomento_protosocket_ffi.a ./examples/golang
	cp ./target/momento-protosocket-ffi.h ./examples/golang
