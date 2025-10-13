# Example usages of protosocket client FFI in other languages

## Golang

The cgo flags come from the FFI build output.

```shell
go run main.go
```

## pkg-config setup

Contains an template pkg-config file for linking the Momento protosocket FFI static library produced by this repo.
Easiest way is to put the `.pc`, `.a`, and `.h` files all in one directory, then set the `PKG_CONFIG_PATH` environment variable to point to that directory.

So far, tested with CGO and verified that doing the above steps and adding these headers will work:
```
#cgo pkg-config: --static momento_protosocket_ffi
#include <momento_protosocket_ffi.h>
```
