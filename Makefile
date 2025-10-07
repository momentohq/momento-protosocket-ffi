build:
	cargo fmt
	RUSTFLAGS='--print=native-static-libs -C strip=symbols' cargo build --release

build-go-example: build
	cp ./target/release/libmomento_protosocket_ffi.a ./examples/golang
	cp ./target/momento-protosocket-ffi.h ./examples/golang
