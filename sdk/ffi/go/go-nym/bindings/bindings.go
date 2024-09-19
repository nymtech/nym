package bindings

// #include <bindings.h>
// #cgo LDFLAGS: -L../../../../../target/release -lnym_go_ffi
import "C"

import (
	"bytes"
	"encoding/binary"
	"fmt"
	"io"
	"math"
	"unsafe"
)

type RustBuffer = C.RustBuffer

type RustBufferI interface {
	AsReader() *bytes.Reader
	Free()
	ToGoBytes() []byte
	Data() unsafe.Pointer
	Len() int
	Capacity() int
}

func RustBufferFromExternal(b RustBufferI) RustBuffer {
	return RustBuffer{
		capacity: C.int(b.Capacity()),
		len:      C.int(b.Len()),
		data:     (*C.uchar)(b.Data()),
	}
}

func (cb RustBuffer) Capacity() int {
	return int(cb.capacity)
}

func (cb RustBuffer) Len() int {
	return int(cb.len)
}

func (cb RustBuffer) Data() unsafe.Pointer {
	return unsafe.Pointer(cb.data)
}

func (cb RustBuffer) AsReader() *bytes.Reader {
	b := unsafe.Slice((*byte)(cb.data), C.int(cb.len))
	return bytes.NewReader(b)
}

func (cb RustBuffer) Free() {
	rustCall(func(status *C.RustCallStatus) bool {
		C.ffi_nym_go_ffi_rustbuffer_free(cb, status)
		return false
	})
}

func (cb RustBuffer) ToGoBytes() []byte {
	return C.GoBytes(unsafe.Pointer(cb.data), C.int(cb.len))
}

func stringToRustBuffer(str string) RustBuffer {
	return bytesToRustBuffer([]byte(str))
}

func bytesToRustBuffer(b []byte) RustBuffer {
	if len(b) == 0 {
		return RustBuffer{}
	}
	// We can pass the pointer along here, as it is pinned
	// for the duration of this call
	foreign := C.ForeignBytes{
		len:  C.int(len(b)),
		data: (*C.uchar)(unsafe.Pointer(&b[0])),
	}

	return rustCall(func(status *C.RustCallStatus) RustBuffer {
		return C.ffi_nym_go_ffi_rustbuffer_from_bytes(foreign, status)
	})
}

type BufLifter[GoType any] interface {
	Lift(value RustBufferI) GoType
}

type BufLowerer[GoType any] interface {
	Lower(value GoType) RustBuffer
}

type FfiConverter[GoType any, FfiType any] interface {
	Lift(value FfiType) GoType
	Lower(value GoType) FfiType
}

type BufReader[GoType any] interface {
	Read(reader io.Reader) GoType
}

type BufWriter[GoType any] interface {
	Write(writer io.Writer, value GoType)
}

type FfiRustBufConverter[GoType any, FfiType any] interface {
	FfiConverter[GoType, FfiType]
	BufReader[GoType]
}

func LowerIntoRustBuffer[GoType any](bufWriter BufWriter[GoType], value GoType) RustBuffer {
	// This might be not the most efficient way but it does not require knowing allocation size
	// beforehand
	var buffer bytes.Buffer
	bufWriter.Write(&buffer, value)

	bytes, err := io.ReadAll(&buffer)
	if err != nil {
		panic(fmt.Errorf("reading written data: %w", err))
	}
	return bytesToRustBuffer(bytes)
}

func LiftFromRustBuffer[GoType any](bufReader BufReader[GoType], rbuf RustBufferI) GoType {
	defer rbuf.Free()
	reader := rbuf.AsReader()
	item := bufReader.Read(reader)
	if reader.Len() > 0 {
		// TODO: Remove this
		leftover, _ := io.ReadAll(reader)
		panic(fmt.Errorf("Junk remaining in buffer after lifting: %s", string(leftover)))
	}
	return item
}

func rustCallWithError[U any](converter BufLifter[error], callback func(*C.RustCallStatus) U) (U, error) {
	var status C.RustCallStatus
	returnValue := callback(&status)
	err := checkCallStatus(converter, status)

	return returnValue, err
}

func checkCallStatus(converter BufLifter[error], status C.RustCallStatus) error {
	switch status.code {
	case 0:
		return nil
	case 1:
		return converter.Lift(status.errorBuf)
	case 2:
		// when the rust code sees a panic, it tries to construct a rustbuffer
		// with the message.  but if that code panics, then it just sends back
		// an empty buffer.
		if status.errorBuf.len > 0 {
			panic(fmt.Errorf("%s", FfiConverterStringINSTANCE.Lift(status.errorBuf)))
		} else {
			panic(fmt.Errorf("Rust panicked while handling Rust panic"))
		}
	default:
		return fmt.Errorf("unknown status code: %d", status.code)
	}
}

func checkCallStatusUnknown(status C.RustCallStatus) error {
	switch status.code {
	case 0:
		return nil
	case 1:
		panic(fmt.Errorf("function not returning an error returned an error"))
	case 2:
		// when the rust code sees a panic, it tries to construct a rustbuffer
		// with the message.  but if that code panics, then it just sends back
		// an empty buffer.
		if status.errorBuf.len > 0 {
			panic(fmt.Errorf("%s", FfiConverterStringINSTANCE.Lift(status.errorBuf)))
		} else {
			panic(fmt.Errorf("Rust panicked while handling Rust panic"))
		}
	default:
		return fmt.Errorf("unknown status code: %d", status.code)
	}
}

func rustCall[U any](callback func(*C.RustCallStatus) U) U {
	returnValue, err := rustCallWithError(nil, callback)
	if err != nil {
		panic(err)
	}
	return returnValue
}

func writeInt8(writer io.Writer, value int8) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeUint8(writer io.Writer, value uint8) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeInt16(writer io.Writer, value int16) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeUint16(writer io.Writer, value uint16) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeInt32(writer io.Writer, value int32) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeUint32(writer io.Writer, value uint32) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeInt64(writer io.Writer, value int64) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeUint64(writer io.Writer, value uint64) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeFloat32(writer io.Writer, value float32) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeFloat64(writer io.Writer, value float64) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func readInt8(reader io.Reader) int8 {
	var result int8
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readUint8(reader io.Reader) uint8 {
	var result uint8
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readInt16(reader io.Reader) int16 {
	var result int16
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readUint16(reader io.Reader) uint16 {
	var result uint16
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readInt32(reader io.Reader) int32 {
	var result int32
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readUint32(reader io.Reader) uint32 {
	var result uint32
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readInt64(reader io.Reader) int64 {
	var result int64
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readUint64(reader io.Reader) uint64 {
	var result uint64
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readFloat32(reader io.Reader) float32 {
	var result float32
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readFloat64(reader io.Reader) float64 {
	var result float64
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func init() {

	uniffiCheckChecksums()
}

func uniffiCheckChecksums() {
	// Get the bindings contract version from our ComponentInterface
	bindingsContractVersion := 24
	// Get the scaffolding contract version by calling the into the dylib
	scaffoldingContractVersion := rustCall(func(uniffiStatus *C.RustCallStatus) C.uint32_t {
		return C.ffi_nym_go_ffi_uniffi_contract_version(uniffiStatus)
	})
	if bindingsContractVersion != int(scaffoldingContractVersion) {
		// If this happens try cleaning and rebuilding your project
		panic("bindings: UniFFI contract version mismatch")
	}
	{
		checksum := rustCall(func(uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_nym_go_ffi_checksum_func_get_self_address(uniffiStatus)
		})
		if checksum != 51546 {
			// If this happens try cleaning and rebuilding your project
			panic("bindings: uniffi_nym_go_ffi_checksum_func_get_self_address: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_nym_go_ffi_checksum_func_init_ephemeral(uniffiStatus)
		})
		if checksum != 28391 {
			// If this happens try cleaning and rebuilding your project
			panic("bindings: uniffi_nym_go_ffi_checksum_func_init_ephemeral: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_nym_go_ffi_checksum_func_init_logging(uniffiStatus)
		})
		if checksum != 1547 {
			// If this happens try cleaning and rebuilding your project
			panic("bindings: uniffi_nym_go_ffi_checksum_func_init_logging: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_nym_go_ffi_checksum_func_listen_for_incoming(uniffiStatus)
		})
		if checksum != 52894 {
			// If this happens try cleaning and rebuilding your project
			panic("bindings: uniffi_nym_go_ffi_checksum_func_listen_for_incoming: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_nym_go_ffi_checksum_func_new_proxy_client(uniffiStatus)
		})
		if checksum != 14386 {
			// If this happens try cleaning and rebuilding your project
			panic("bindings: uniffi_nym_go_ffi_checksum_func_new_proxy_client: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_nym_go_ffi_checksum_func_reply(uniffiStatus)
		})
		if checksum != 50524 {
			// If this happens try cleaning and rebuilding your project
			panic("bindings: uniffi_nym_go_ffi_checksum_func_reply: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_nym_go_ffi_checksum_func_send_message(uniffiStatus)
		})
		if checksum != 33425 {
			// If this happens try cleaning and rebuilding your project
			panic("bindings: uniffi_nym_go_ffi_checksum_func_send_message: UniFFI API checksum mismatch")
		}
	}
}

type FfiConverterUint64 struct{}

var FfiConverterUint64INSTANCE = FfiConverterUint64{}

func (FfiConverterUint64) Lower(value uint64) C.uint64_t {
	return C.uint64_t(value)
}

func (FfiConverterUint64) Write(writer io.Writer, value uint64) {
	writeUint64(writer, value)
}

func (FfiConverterUint64) Lift(value C.uint64_t) uint64 {
	return uint64(value)
}

func (FfiConverterUint64) Read(reader io.Reader) uint64 {
	return readUint64(reader)
}

type FfiDestroyerUint64 struct{}

func (FfiDestroyerUint64) Destroy(_ uint64) {}

type FfiConverterString struct{}

var FfiConverterStringINSTANCE = FfiConverterString{}

func (FfiConverterString) Lift(rb RustBufferI) string {
	defer rb.Free()
	reader := rb.AsReader()
	b, err := io.ReadAll(reader)
	if err != nil {
		panic(fmt.Errorf("reading reader: %w", err))
	}
	return string(b)
}

func (FfiConverterString) Read(reader io.Reader) string {
	length := readInt32(reader)
	buffer := make([]byte, length)
	read_length, err := reader.Read(buffer)
	if err != nil {
		panic(err)
	}
	if read_length != int(length) {
		panic(fmt.Errorf("bad read length when reading string, expected %d, read %d", length, read_length))
	}
	return string(buffer)
}

func (FfiConverterString) Lower(value string) RustBuffer {
	return stringToRustBuffer(value)
}

func (FfiConverterString) Write(writer io.Writer, value string) {
	if len(value) > math.MaxInt32 {
		panic("String is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	write_length, err := io.WriteString(writer, value)
	if err != nil {
		panic(err)
	}
	if write_length != len(value) {
		panic(fmt.Errorf("bad write length when writing string, expected %d, written %d", len(value), write_length))
	}
}

type FfiDestroyerString struct{}

func (FfiDestroyerString) Destroy(_ string) {}

type FfiConverterBytes struct{}

var FfiConverterBytesINSTANCE = FfiConverterBytes{}

func (c FfiConverterBytes) Lower(value []byte) RustBuffer {
	return LowerIntoRustBuffer[[]byte](c, value)
}

func (c FfiConverterBytes) Write(writer io.Writer, value []byte) {
	if len(value) > math.MaxInt32 {
		panic("[]byte is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	write_length, err := writer.Write(value)
	if err != nil {
		panic(err)
	}
	if write_length != len(value) {
		panic(fmt.Errorf("bad write length when writing []byte, expected %d, written %d", len(value), write_length))
	}
}

func (c FfiConverterBytes) Lift(rb RustBufferI) []byte {
	return LiftFromRustBuffer[[]byte](c, rb)
}

func (c FfiConverterBytes) Read(reader io.Reader) []byte {
	length := readInt32(reader)
	buffer := make([]byte, length)
	read_length, err := reader.Read(buffer)
	if err != nil {
		panic(err)
	}
	if read_length != int(length) {
		panic(fmt.Errorf("bad read length when reading []byte, expected %d, read %d", length, read_length))
	}
	return buffer
}

type FfiDestroyerBytes struct{}

func (FfiDestroyerBytes) Destroy(_ []byte) {}

type IncomingMessage struct {
	Message string
	Sender  []byte
}

func (r *IncomingMessage) Destroy() {
	FfiDestroyerString{}.Destroy(r.Message)
	FfiDestroyerBytes{}.Destroy(r.Sender)
}

type FfiConverterTypeIncomingMessage struct{}

var FfiConverterTypeIncomingMessageINSTANCE = FfiConverterTypeIncomingMessage{}

func (c FfiConverterTypeIncomingMessage) Lift(rb RustBufferI) IncomingMessage {
	return LiftFromRustBuffer[IncomingMessage](c, rb)
}

func (c FfiConverterTypeIncomingMessage) Read(reader io.Reader) IncomingMessage {
	return IncomingMessage{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterBytesINSTANCE.Read(reader),
	}
}

func (c FfiConverterTypeIncomingMessage) Lower(value IncomingMessage) RustBuffer {
	return LowerIntoRustBuffer[IncomingMessage](c, value)
}

func (c FfiConverterTypeIncomingMessage) Write(writer io.Writer, value IncomingMessage) {
	FfiConverterStringINSTANCE.Write(writer, value.Message)
	FfiConverterBytesINSTANCE.Write(writer, value.Sender)
}

type FfiDestroyerTypeIncomingMessage struct{}

func (_ FfiDestroyerTypeIncomingMessage) Destroy(value IncomingMessage) {
	value.Destroy()
}

type GoWrapError struct {
	err error
}

func (err GoWrapError) Error() string {
	return fmt.Sprintf("GoWrapError: %s", err.err.Error())
}

func (err GoWrapError) Unwrap() error {
	return err.err
}

// Err* are used for checking error type with `errors.Is`
var ErrGoWrapErrorClientInitError = fmt.Errorf("GoWrapErrorClientInitError")
var ErrGoWrapErrorClientUninitialisedError = fmt.Errorf("GoWrapErrorClientUninitialisedError")
var ErrGoWrapErrorSelfAddrError = fmt.Errorf("GoWrapErrorSelfAddrError")
var ErrGoWrapErrorSendMsgError = fmt.Errorf("GoWrapErrorSendMsgError")
var ErrGoWrapErrorReplyError = fmt.Errorf("GoWrapErrorReplyError")
var ErrGoWrapErrorListenError = fmt.Errorf("GoWrapErrorListenError")
var ErrGoWrapErrorProxyInitError = fmt.Errorf("GoWrapErrorProxyInitError")
var ErrGoWrapErrorProxyUninitialisedError = fmt.Errorf("GoWrapErrorProxyUninitialisedError")

// Variant structs
type GoWrapErrorClientInitError struct {
	message string
}

func NewGoWrapErrorClientInitError() *GoWrapError {
	return &GoWrapError{
		err: &GoWrapErrorClientInitError{},
	}
}

func (err GoWrapErrorClientInitError) Error() string {
	return fmt.Sprintf("ClientInitError: %s", err.message)
}

func (self GoWrapErrorClientInitError) Is(target error) bool {
	return target == ErrGoWrapErrorClientInitError
}

type GoWrapErrorClientUninitialisedError struct {
	message string
}

func NewGoWrapErrorClientUninitialisedError() *GoWrapError {
	return &GoWrapError{
		err: &GoWrapErrorClientUninitialisedError{},
	}
}

func (err GoWrapErrorClientUninitialisedError) Error() string {
	return fmt.Sprintf("ClientUninitialisedError: %s", err.message)
}

func (self GoWrapErrorClientUninitialisedError) Is(target error) bool {
	return target == ErrGoWrapErrorClientUninitialisedError
}

type GoWrapErrorSelfAddrError struct {
	message string
}

func NewGoWrapErrorSelfAddrError() *GoWrapError {
	return &GoWrapError{
		err: &GoWrapErrorSelfAddrError{},
	}
}

func (err GoWrapErrorSelfAddrError) Error() string {
	return fmt.Sprintf("SelfAddrError: %s", err.message)
}

func (self GoWrapErrorSelfAddrError) Is(target error) bool {
	return target == ErrGoWrapErrorSelfAddrError
}

type GoWrapErrorSendMsgError struct {
	message string
}

func NewGoWrapErrorSendMsgError() *GoWrapError {
	return &GoWrapError{
		err: &GoWrapErrorSendMsgError{},
	}
}

func (err GoWrapErrorSendMsgError) Error() string {
	return fmt.Sprintf("SendMsgError: %s", err.message)
}

func (self GoWrapErrorSendMsgError) Is(target error) bool {
	return target == ErrGoWrapErrorSendMsgError
}

type GoWrapErrorReplyError struct {
	message string
}

func NewGoWrapErrorReplyError() *GoWrapError {
	return &GoWrapError{
		err: &GoWrapErrorReplyError{},
	}
}

func (err GoWrapErrorReplyError) Error() string {
	return fmt.Sprintf("ReplyError: %s", err.message)
}

func (self GoWrapErrorReplyError) Is(target error) bool {
	return target == ErrGoWrapErrorReplyError
}

type GoWrapErrorListenError struct {
	message string
}

func NewGoWrapErrorListenError() *GoWrapError {
	return &GoWrapError{
		err: &GoWrapErrorListenError{},
	}
}

func (err GoWrapErrorListenError) Error() string {
	return fmt.Sprintf("ListenError: %s", err.message)
}

func (self GoWrapErrorListenError) Is(target error) bool {
	return target == ErrGoWrapErrorListenError
}

type GoWrapErrorProxyInitError struct {
	message string
}

func NewGoWrapErrorProxyInitError() *GoWrapError {
	return &GoWrapError{
		err: &GoWrapErrorProxyInitError{},
	}
}

func (err GoWrapErrorProxyInitError) Error() string {
	return fmt.Sprintf("ProxyInitError: %s", err.message)
}

func (self GoWrapErrorProxyInitError) Is(target error) bool {
	return target == ErrGoWrapErrorProxyInitError
}

type GoWrapErrorProxyUninitialisedError struct {
	message string
}

func NewGoWrapErrorProxyUninitialisedError() *GoWrapError {
	return &GoWrapError{
		err: &GoWrapErrorProxyUninitialisedError{},
	}
}

func (err GoWrapErrorProxyUninitialisedError) Error() string {
	return fmt.Sprintf("ProxyUninitialisedError: %s", err.message)
}

func (self GoWrapErrorProxyUninitialisedError) Is(target error) bool {
	return target == ErrGoWrapErrorProxyUninitialisedError
}

type FfiConverterTypeGoWrapError struct{}

var FfiConverterTypeGoWrapErrorINSTANCE = FfiConverterTypeGoWrapError{}

func (c FfiConverterTypeGoWrapError) Lift(eb RustBufferI) error {
	return LiftFromRustBuffer[error](c, eb)
}

func (c FfiConverterTypeGoWrapError) Lower(value *GoWrapError) RustBuffer {
	return LowerIntoRustBuffer[*GoWrapError](c, value)
}

func (c FfiConverterTypeGoWrapError) Read(reader io.Reader) error {
	errorID := readUint32(reader)

	message := FfiConverterStringINSTANCE.Read(reader)
	switch errorID {
	case 1:
		return &GoWrapError{&GoWrapErrorClientInitError{message}}
	case 2:
		return &GoWrapError{&GoWrapErrorClientUninitialisedError{message}}
	case 3:
		return &GoWrapError{&GoWrapErrorSelfAddrError{message}}
	case 4:
		return &GoWrapError{&GoWrapErrorSendMsgError{message}}
	case 5:
		return &GoWrapError{&GoWrapErrorReplyError{message}}
	case 6:
		return &GoWrapError{&GoWrapErrorListenError{message}}
	case 7:
		return &GoWrapError{&GoWrapErrorProxyInitError{message}}
	case 8:
		return &GoWrapError{&GoWrapErrorProxyUninitialisedError{message}}
	default:
		panic(fmt.Sprintf("Unknown error code %d in FfiConverterTypeGoWrapError.Read()", errorID))
	}

}

func (c FfiConverterTypeGoWrapError) Write(writer io.Writer, value *GoWrapError) {
	switch variantValue := value.err.(type) {
	case *GoWrapErrorClientInitError:
		writeInt32(writer, 1)
	case *GoWrapErrorClientUninitialisedError:
		writeInt32(writer, 2)
	case *GoWrapErrorSelfAddrError:
		writeInt32(writer, 3)
	case *GoWrapErrorSendMsgError:
		writeInt32(writer, 4)
	case *GoWrapErrorReplyError:
		writeInt32(writer, 5)
	case *GoWrapErrorListenError:
		writeInt32(writer, 6)
	case *GoWrapErrorProxyInitError:
		writeInt32(writer, 7)
	case *GoWrapErrorProxyUninitialisedError:
		writeInt32(writer, 8)
	default:
		_ = variantValue
		panic(fmt.Sprintf("invalid error value `%v` in FfiConverterTypeGoWrapError.Write", value))
	}
}

type FfiConverterOptionalString struct{}

var FfiConverterOptionalStringINSTANCE = FfiConverterOptionalString{}

func (c FfiConverterOptionalString) Lift(rb RustBufferI) *string {
	return LiftFromRustBuffer[*string](c, rb)
}

func (_ FfiConverterOptionalString) Read(reader io.Reader) *string {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterStringINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalString) Lower(value *string) RustBuffer {
	return LowerIntoRustBuffer[*string](c, value)
}

func (_ FfiConverterOptionalString) Write(writer io.Writer, value *string) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterStringINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalString struct{}

func (_ FfiDestroyerOptionalString) Destroy(value *string) {
	if value != nil {
		FfiDestroyerString{}.Destroy(*value)
	}
}

func GetSelfAddress() (string, error) {
	_uniffiRV, _uniffiErr := rustCallWithError(FfiConverterTypeGoWrapError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return C.uniffi_nym_go_ffi_fn_func_get_self_address(_uniffiStatus)
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

func InitEphemeral() error {
	_, _uniffiErr := rustCallWithError(FfiConverterTypeGoWrapError{}, func(_uniffiStatus *C.RustCallStatus) bool {
		C.uniffi_nym_go_ffi_fn_func_init_ephemeral(_uniffiStatus)
		return false
	})
	return _uniffiErr
}

func InitLogging() {
	rustCall(func(_uniffiStatus *C.RustCallStatus) bool {
		C.uniffi_nym_go_ffi_fn_func_init_logging(_uniffiStatus)
		return false
	})
}

func ListenForIncoming() (IncomingMessage, error) {
	_uniffiRV, _uniffiErr := rustCallWithError(FfiConverterTypeGoWrapError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return C.uniffi_nym_go_ffi_fn_func_listen_for_incoming(_uniffiStatus)
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue IncomingMessage
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterTypeIncomingMessageINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

func NewProxyClient(serverAddress string, listenAddress string, listenPort string, closeTimeout uint64, env *string) error {
	_, _uniffiErr := rustCallWithError(FfiConverterTypeGoWrapError{}, func(_uniffiStatus *C.RustCallStatus) bool {
		C.uniffi_nym_go_ffi_fn_func_new_proxy_client(FfiConverterStringINSTANCE.Lower(serverAddress), FfiConverterStringINSTANCE.Lower(listenAddress), FfiConverterStringINSTANCE.Lower(listenPort), FfiConverterUint64INSTANCE.Lower(closeTimeout), FfiConverterOptionalStringINSTANCE.Lower(env), _uniffiStatus)
		return false
	})
	return _uniffiErr
}

func Reply(recipient []byte, message string) error {
	_, _uniffiErr := rustCallWithError(FfiConverterTypeGoWrapError{}, func(_uniffiStatus *C.RustCallStatus) bool {
		C.uniffi_nym_go_ffi_fn_func_reply(FfiConverterBytesINSTANCE.Lower(recipient), FfiConverterStringINSTANCE.Lower(message), _uniffiStatus)
		return false
	})
	return _uniffiErr
}

func SendMessage(recipient string, message string) error {
	_, _uniffiErr := rustCallWithError(FfiConverterTypeGoWrapError{}, func(_uniffiStatus *C.RustCallStatus) bool {
		C.uniffi_nym_go_ffi_fn_func_send_message(FfiConverterStringINSTANCE.Lower(recipient), FfiConverterStringINSTANCE.Lower(message), _uniffiStatus)
		return false
	})
	return _uniffiErr
}
