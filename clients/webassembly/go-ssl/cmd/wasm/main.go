// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"bytes"
	"context"
	"crypto/x509"
	"encoding/hex"
	"fmt"
	//gotls "crypto/tls"
	tls "github.com/refraction-networking/utls"
	"io"
	"net"
	"syscall/js"
	"time"
	_ "unsafe"
)

var done chan struct{}

func init() {
	println("[go init]: go module init")
	done = make(chan struct{})
	println("[go init]: go module init finished")
}

func main() {
	println("[go main]: go module loaded")

	js.Global().Set("wasmPrintConnection", js.FuncOf(printConnection))
	js.Global().Set("goWasmStartSSLHandshake", js.FuncOf(startSSLHandshake))
	js.Global().Set("goWasmInjectServerData", js.FuncOf(injectServerData))
	js.Global().Set("goWasmTryReadClientData", js.FuncOf(tryReadClientData))

	<-done

	println("[go main]: go module finished")
}

var helper *SSLHelper

type SSLHelper struct {
	tlsConn *tls.UConn

	injectedServerData chan []byte
	createdClientData  chan []byte

	clientHello []byte
	serverHello []byte
}

func NewSSLHelper() SSLHelper {
	return SSLHelper{
		tlsConn:            nil,
		injectedServerData: make(chan []byte, 10),
		createdClientData:  make(chan []byte, 10),
		clientHello:        nil,
		serverHello:        nil,
	}
}

type fakeConnection struct {
	injectedServerData chan []byte
	createdClientData  chan []byte
	incompleteReads    chan []byte
}

func (conn fakeConnection) Read(p []byte) (int, error) {
	select {
	case incomplete := <-conn.incompleteReads:
		println("reading previously incomplete data")
		buf := bytes.NewReader(incomplete)
		n, err := buf.Read(p)

		remaining := buf.Len()
		if remaining > 0 {
			leftover := make([]byte, remaining)
			_, _ = buf.Read(leftover)
			conn.incompleteReads <- leftover
		}

		encoded := hex.EncodeToString(p[:n])
		fmt.Printf("READ >>> %v\n", encoded)
		return n, err
	default:
		println("waiting for data to read...")

		context.Background()

		data := <-conn.injectedServerData
		if len(data) == 0 {
			return 0, io.ErrClosedPipe
		}

		buf := bytes.NewReader(data)
		n, err := buf.Read(p)

		remaining := buf.Len()
		if remaining > 0 {
			leftover := make([]byte, remaining)
			_, _ = buf.Read(leftover)
			conn.incompleteReads <- leftover
		}

		encoded := hex.EncodeToString(p[:n])
		fmt.Printf("READ >>> %v\n", encoded)
		return n, err
	}
}

func (conn fakeConnection) Write(p []byte) (int, error) {
	encoded := hex.EncodeToString(p)
	fmt.Printf("WRITE >>> %v\n", encoded)

	conn.createdClientData <- p
	return len(p), nil
}
func (conn fakeConnection) Close() error {
	println("close fakeConnection")
	return nil
}
func (conn fakeConnection) LocalAddr() net.Addr {
	println("LocalAddr fakeConnection")
	return nil
}
func (conn fakeConnection) RemoteAddr() net.Addr {
	println("RemoteAddr fakeConnection")
	return nil
}
func (conn fakeConnection) SetDeadline(t time.Time) error {
	println("SetDeadline fakeConnection")
	return nil
}
func (conn fakeConnection) SetReadDeadline(t time.Time) error {
	println("SetReadDeadline fakeConnection")
	return nil
}
func (conn fakeConnection) SetWriteDeadline(t time.Time) error {
	println("SetWriteDeadline fakeConnection")
	return nil
}

func printConnection(this js.Value, args []js.Value) interface{} {
	if helper == nil {
		println("no connection")
		return nil
	}

	println("ClientHelloBuilt: ", helper.tlsConn.ClientHelloBuilt)
	println("State")
	fmt.Printf("ServerHello: %+v\n", helper.tlsConn.HandshakeState.ServerHello)
	fmt.Printf("ClientHello: %+v\n", helper.tlsConn.HandshakeState.Hello)
	fmt.Printf("MasterSecret: %+v\n", helper.tlsConn.HandshakeState.MasterSecret)
	fmt.Printf("Session: %+v\n", helper.tlsConn.HandshakeState.Session)
	fmt.Printf("State12: %+v\n", helper.tlsConn.HandshakeState.State12)
	fmt.Printf("State13: %+v\n", helper.tlsConn.HandshakeState.State13)
	fmt.Printf("conn: %+v\n", helper.tlsConn.Conn)

	return nil
}

func setupFakeTlsConn() *SSLHelper {
	helper := NewSSLHelper()
	fakeConnection := fakeConnection{
		injectedServerData: helper.injectedServerData,
		createdClientData:  helper.createdClientData,
		incompleteReads:    make(chan []byte, 1),
	}

	tlsConfig := tls.Config{
		VerifyPeerCertificate: func(rawCerts [][]byte, verifiedChains [][]*x509.Certificate) error {
			println("TODO: verifying certs")
			return nil
		},
		// Set InsecureSkipVerify to skip the default validation we are
		// replacing. This will not disable VerifyConnection.
		InsecureSkipVerify: true,
		VerifyConnection: func(cs tls.ConnectionState) error {
			println("TODO: verifying conn")
			fmt.Printf("%+v\n", cs)
			return nil
		},
		// TODO: get this from arguments (presumably?)
		ServerName: "www.nymtech.net",
	}

	tlsConn := tls.UClient(fakeConnection, &tlsConfig, tls.HelloGolang)
	helper.tlsConn = tlsConn
	return &helper
}

// will return ClientHello
func startSSLHandshake(this js.Value, args []js.Value) interface{} {
	if helper != nil {
		println("we have already started the connection")
		return hex.EncodeToString(helper.clientHello)
	}
	helper = setupFakeTlsConn()

	go func() {
		println("starting TLS handshake in separate goroutine (how does that even work in wasm?)")
		err := helper.tlsConn.Handshake()
		println("handshake done")
		printConnection(js.Undefined(), []js.Value{})
		if err != nil {
			println("but there was an error")
			panic(err)
		}

	}()

	clientHello := <-helper.createdClientData
	helper.clientHello = clientHello
	return hex.EncodeToString(clientHello)
}

func injectServerData(this js.Value, args []js.Value) interface{} {
	if helper == nil {
		println("we haven't started any connection yet")
		return nil
	}

	value := args[0].String()
	decoded, err := hex.DecodeString(value)
	if err != nil {
		panic(err)
	}

	helper.injectedServerData <- decoded
	return nil
}

func tryReadClientData(this js.Value, args []js.Value) interface{} {
	if helper == nil {
		println("we haven't started any connection yet")
		return nil
	}

	select {
	case data := <-helper.createdClientData:
		println("data was available!")
		encoded := hex.EncodeToString(data)
		return encoded
	default:
		fmt.Println("No value ready, moving on.")
		return nil
	}
}
