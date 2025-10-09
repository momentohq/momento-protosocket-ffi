# Example usages of protosocket client FFI in other languages

## Golang

There are two versions of the same Golang example: one for macos and one for ubuntu.
Both use the protosocket FFI library, but they each use different `cgo LDFLAGS` in the
C directives portion at the top of the file. Those flags come from the FFI build output.

```shell
go run main_macos.go

go run main_ubuntu.go
```