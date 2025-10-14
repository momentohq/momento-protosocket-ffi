
{{ ossHeader }}

# Momento Protosocket FFI Examples

To use the protosocket cache client in languages other than Rust, you must link the C static library produced by this FFI library and call the provided methods.

## Golang

The [golang folder](./golang/) contains an example of using the FFI library in a Go project.

To run the example, you will need a Momento API key, which you can generate in the [Momento Console](https://console.gomomento.com/).

```shell
export MOMENTO_API_KEY="your-api-key"

go run golang/main.go
```

You will also need to download the appropriate `.a` and `.h` files from the GitHub Releases of this repo and store them somewhere the Go project can access (two approaches are outlined below).

## Setup using locally available C static library

This is the easiest method. Simply put the `.a` and `.h` files in a local directory and update the cgo flags to point to them directly.

The other cgo flags featured here come from the FFI build output. You can remove the `cgo !darwin` or `cgo darwin` lines as needed though.

```go
package main

/*
#cgo LDFLAGS: /path/to/libmomento_protosocket_ffi.a -ldl -lm -lc
#cgo !darwin LDFLAGS: -lgcc_s -lutil -lrt -lpthread
#cgo darwin LDFLAGS: -framework Security -framework CoreFoundation -lc++ -liconv
#include "/path/to/momento_protosocket_ffi.h"
#include <string.h>
*/
import "C"

func main() {
  <your code here>
}
```

### Setup using pkg-config

This approach was outlined in [cgo docs](https://pkg.go.dev/cmd/cgo) and requires a bit more setup, but avoids the need to ship a golang project with the C static library.

The [pkg-config-setup folder](./pkg-config-setup/) contains an template pkg-config file for linking the C static library to the Go project. Simply make a copy of this file and update the `libdir` variable to point to the directory where you will put the `.pc`, `.a`, and `.h` files. Then set the `PKG_CONFIG_PATH` environment variable to point to that directory.

You can use the `pkg-config --list-package-names` or `pkg-config --libs --static momento-protosocket-ffi` commands to check if it worked.

The cgo flags in your go file should look like this:

```go
package main

/*
#cgo LDFLAGS: -ldl -lm -lc
#cgo !darwin LDFLAGS: -lgcc_s -lutil -lrt -lpthread
#cgo darwin LDFLAGS: -framework Security -framework CoreFoundation -lc++ -liconv
#cgo pkg-config: --static momento_protosocket_ffi
#include <momento_protosocket_ffi.h>
#include <string.h>
*/
import "C"

func main() {
  <your code here>
}
```

{{ ossFooter }}