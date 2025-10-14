package main

/*
#cgo LDFLAGS: ./libmomento_protosocket_ffi.a -ldl -lm -lc
#cgo !darwin LDFLAGS: -lgcc_s -lutil -lrt -lpthread
#cgo darwin LDFLAGS: -framework Security -framework CoreFoundation -lc++ -liconv
#include "./momento_protosocket_ffi.h"
#include <string.h>

extern void setCallback(ProtosocketResult* result, void* user_data);
extern void getCallback(ProtosocketResult* result, void* user_data);
*/
import "C"
import (
	"fmt"
	"os"
	"sync"
	"sync/atomic"
	"unsafe"
)

type SetResponse struct {
	Success bool
	Error   string
}

type GetResponse struct {
	Hit   bool
	Value []byte
	Error string
}

var (
	setContexts sync.Map
	getContexts sync.Map
	nextID      uint64
)

func main() {
	// Create FFI-compatible protosocket configuration
	timeoutMillis := C.ulong(15_000)
	connectionCount := C.ulong(1)
	config := C.new_protosocket_client_configuration(timeoutMillis, connectionCount)

	if os.Getenv("MOMENTO_API_KEY") == "" {
		fmt.Printf("[ERROR] MOMENTO_API_KEY is not set\n")
		os.Exit(1)
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

	makeSetCallWithCallback(cacheName, key, value)
	makeGetCallWithCallback(cacheName, key)

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

//export setCallback
func setCallback(result *C.ProtosocketResult, userData unsafe.Pointer) {
	id := uint64(uintptr(userData))

	chInterface, ok := setContexts.LoadAndDelete(id)
	if !ok {
		C.free_response(result)
		return
	}

	ch := chInterface.(chan SetResponse)
	responseType := C.GoString(result.response_type)

	var response SetResponse
	if responseType == "SetSuccess" {
		response.Success = true
	} else if responseType == "Error" {
		response.Error = C.GoString(result.error_message)
	}

	ch <- response
	C.free_response(result)
}

//export getCallback
func getCallback(result *C.ProtosocketResult, userData unsafe.Pointer) {
	id := uint64(uintptr(userData))

	chInterface, ok := getContexts.LoadAndDelete(id)
	if !ok {
		C.free_response(result)
		return
	}

	ch := chInterface.(chan GetResponse)
	responseType := C.GoString(result.response_type)

	var response GetResponse
	if responseType == "GetHit" {
		response.Hit = true
		response.Value = convertCBytesToGoBytes(result.value)
	} else if responseType == "GetMiss" {
		response.Hit = false
	} else if responseType == "Error" {
		response.Error = C.GoString(result.error_message)
	}

	ch <- response
	C.free_response(result)
}

func makeSetCallWithCallback(cacheName string, key string, value string) {
	cacheNameC := C.CString(cacheName)
	defer C.free(unsafe.Pointer(cacheNameC))

	keyC := convertGoStringToCBytes(key)
	defer C.free(unsafe.Pointer(keyC.data))

	valueC := convertGoStringToCBytes(value)
	defer C.free(unsafe.Pointer(valueC.data))

	responseCh := make(chan SetResponse, 1)

	id := atomic.AddUint64(&nextID, 1)

	setContexts.Store(id, responseCh)

	C.protosocket_cache_client_set_with_callback(
		cacheNameC,
		keyC,
		valueC,
		C.ProtosocketCallback(C.setCallback),
		unsafe.Pointer(uintptr(id)),
	)

	response := <-responseCh

	if response.Success {
		fmt.Printf("[INFO] set success\n")
	} else {
		fmt.Printf("[ERROR] set error: %v\n", response.Error)
	}
}

func makeGetCallWithCallback(cacheName string, key string) {
	cacheNameC := C.CString(cacheName)
	defer C.free(unsafe.Pointer(cacheNameC))
	keyC := convertGoStringToCBytes(key)
	defer C.free(unsafe.Pointer(keyC.data))

	responseCh := make(chan GetResponse, 1)

	id := atomic.AddUint64(&nextID, 1)

	getContexts.Store(id, responseCh)

	C.protosocket_cache_client_get_with_callback(
		cacheNameC,
		keyC,
		C.ProtosocketCallback(C.getCallback),
		unsafe.Pointer(uintptr(id)),
	)

	response := <-responseCh

	if response.Hit {
		fmt.Printf("[INFO] get hit | raw value: %v | string value: %s\n", response.Value, string(response.Value))
	} else if response.Error != "" {
		fmt.Printf("[ERROR] get error: %v\n", response.Error)
	} else {
		fmt.Printf("[INFO] get miss\n")
	}
}
