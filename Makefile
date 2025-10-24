build:
	MACOSX_DEPLOYMENT_TARGET=15.0 RUSTFLAGS='--print=native-static-libs -C strip=symbols' cargo build --release

build-headers:
	cargo run --features headers --bin generate-headers

build-go-example: build
	cp ./target/release/libmomento_protosocket_ffi.a ./examples/golang
	cp ./target/momento_protosocket_ffi.h ./examples/golang

format:
	cargo fmt

lint:
	cargo fmt -- --check && \
	cargo clippy --all-features -- -D warnings -W clippy::unwrap_used && \
	cargo clippy --tests -- -D warnings -W clippy::unwrap_used
