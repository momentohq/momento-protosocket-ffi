package main

// Note: using LDFLAGS suggested by make build output

/*
#cgo LDFLAGS: ./libmomento_protosocket_ffi.a -ldl -framework Security -framework CoreFoundation -lc++ -liconv -lc -lm
#include "./momento-protosocket-ffi.h"
#include <string.h>
*/
import "C"
import (
	"fmt"
	"os"
	"time"
	"unsafe"
)

func main() {
	// Create FFI-compatible protosocket configuration
	timeoutMillis := C.ulong(15_000)
	connectionCount := C.ulong(1)
	config := C.new_protosocket_client_configuration(timeoutMillis, connectionCount)

	if os.Getenv("MOMENTO_API_KEY") == "" {
		fmt.Printf("[ERROR] MOMENTO_API_KEY is not set\n")
		return
	}

	// Create FFI-compatible credential provider
	envVarName := C.CString("MOMENTO_API_KEY")
	creds := C.new_protosocket_credential_provider(envVarName)

	// Create the tokio runtime and the protosocket client under the hood
	defaultTtlMillis := C.ulonglong(60 * 1000)
	C.init_protosocket_cache_client(defaultTtlMillis, config, creds)

	cacheName := "test"
	key := "test"
	value := "test"

	makeSetCall(cacheName, key, value)
	makeGetCall(cacheName, key)

	C.destroy_protosocket_cache_client()
}

func convertGoStringToCBytes(string string) *C.Bytes {
	bytes := []byte(string)
	return convertGoBytesToCBytes(bytes)
}

func convertGoBytesToCBytes(bytes []byte) *C.Bytes {
	c_bytes := C.malloc(C.size_t(len(bytes)))
	C.memcpy(c_bytes, unsafe.Pointer(&bytes[0]), C.size_t(len(bytes)))
	return &C.Bytes{
		data:   (*C.uchar)(c_bytes),
		length: C.ulong(len(bytes)),
	}
}

func convertCBytesToGoBytes(c_bytes *C.Bytes) []byte {
	return C.GoBytes(unsafe.Pointer(c_bytes.data), C.int(c_bytes.length))
}

func makeSetCall(cacheName string, key string, value string) {
	cacheNameC := C.CString(cacheName)
	keyC := convertGoStringToCBytes(key)
	valueC := convertGoStringToCBytes(value)

	// Kick off the set operation (runs asynchronously on the tokio runtime) and
	// receive the initial response with pointers to error or response object
	initialResponse := C.protosocket_cache_client_set(cacheNameC, keyC, valueC)

	// If it's already completed, we probably received an error
	if initialResponse.completed != nil {
		fmt.Printf("[ERROR] initialResponse.completed is not nil: %v %v\n", initialResponse.completed.response_type, initialResponse.completed.error_message)
		return
	}

	// If both completed and awaiting pointers are nil, something went wrong
	if initialResponse.awaiting == nil {
		fmt.Printf("[ERROR] initialResponse.awaiting is unexpectedly nil\n")
		return
	}

	// Otherwise we should be awaiting a response object and FFI
	// should have provided us with an operation id to poll for.
	op_id := initialResponse.awaiting.operation_id
	fmt.Printf("[INFO] operation id: %v\n", op_id)

	// Poll until we get a response
	var response *C.ProtosocketResult
	for {
		response = C.protosocket_cache_client_poll_responses(op_id)
		if response != nil {
			break
		}
		// Poll as frequently as desired
		time.Sleep(10 * time.Microsecond)
	}

	// Once the response is received, parse response type to determine success or error
	responseType := C.GoString(response.response_type)
	if responseType == "SetSuccess" {
		fmt.Printf("[INFO] set success\n")
	} else if responseType == "Error" {
		fmt.Printf("[ERROR] set error: %v\n", C.GoString(response.error_message))
	}

	// Free the C objects that were allocated
	C.free_response(response)
	C.free(unsafe.Pointer(cacheNameC))
	C.free(unsafe.Pointer(keyC.data))
	C.free(unsafe.Pointer(valueC.data))
}

func makeGetCall(cacheName string, key string) {
	cacheNameC := C.CString(cacheName)
	keyC := convertGoStringToCBytes(key)

	// Kick off the get operation (runs asynchronously on the tokio runtime) and
	// receive the initial response with pointers to error or response object
	initialResponse := C.protosocket_cache_client_get(cacheNameC, keyC)

	// If it's already completed, we probably received an error
	if initialResponse.completed != nil {
		fmt.Printf("[ERROR] initialResponse.completed is not nil: %v %v\n", initialResponse.completed.response_type, initialResponse.completed.error_message)
		return
	}

	// If both completed and awaiting pointers are nil, something went wrong
	if initialResponse.awaiting == nil {
		fmt.Printf("[ERROR] initialResponse.awaiting is unexpectedly nil\n")
		return
	}

	// Otherwise we should be awaiting a response object and FFI
	// should have provided us with an operation id to poll for.
	op_id := initialResponse.awaiting.operation_id
	fmt.Printf("[INFO] operation id: %v\n", op_id)

	// Poll until we get a response
	var response *C.ProtosocketResult
	for {
		response = C.protosocket_cache_client_poll_responses(op_id)
		if response != nil {
			break
		}
		// Poll as frequently as desired
		time.Sleep(10 * time.Microsecond)
	}

	// Once the response is received, parse response type to determine success or error
	responseType := C.GoString(response.response_type)
	if responseType == "GetHit" {
		getHitValue := convertCBytesToGoBytes(response.value)
		fmt.Printf("[INFO] get hit | raw value: %v | string value: %s\n", getHitValue, string(getHitValue))
	} else if responseType == "GetMiss" {
		fmt.Printf("[INFO] get miss\n")
	} else if responseType == "Error" {
		fmt.Printf("[ERROR] get error: %v\n", C.GoString(response.error_message))
	}

	C.free_response(response)
	C.free(unsafe.Pointer(cacheNameC))
	C.free(unsafe.Pointer(keyC.data))
}
