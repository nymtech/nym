// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//go:build js && wasm

package main

import (
	"context"
	"encoding/hex"
	"fmt"
	"io"
	"log"
	"net"
	"net/http"
	_ "net/http"
	"net/url"
	"syscall/js"
)

var done chan struct{}

func init() {
	println("[go init]: go module init")
	SetupLogging()
	done = make(chan struct{})
	println("[go init]: go module init finished")
}

func main() {
	println("[go main]: go module loaded")

	//js.Global().Set("debugGoWasmPrintConnection", js.FuncOf(printConnection))

	js.Global().Set("goWasmStartSSLHandshake", js.FuncOf(startSSLHandshakeJS))
	js.Global().Set("goWasmInjectServerData", js.FuncOf(injectServerData))
	js.Global().Set("goWasmTryReadClientData", js.FuncOf(tryReadClientData))

	js.Global().Set("goWasmHTTPTest", js.FuncOf(startConnection))
	js.Global().Set("startSSLHandshakeJS", js.FuncOf(startSSLHandshakeJS))

	//js.Global().Set("foomp", js.FuncOf(testStuff))

	<-done

	println("[go main]: go module finished")
}

func testStuff(this js.Value, args []js.Value) interface{} {
	//foomp := args[0].j
	println("about to call the arg")
	args[0].Invoke()
	println("called")
	return nil

}

//
//func printConnection(this js.Value, args []js.Value) interface{} {
//	//if currentSSLHelper == nil {
//	//	println("no connection")
//	//	return nil
//	//}
//	//
//	//println("ClientHelloBuilt: ", currentSSLHelper.tlsConn.ClientHelloBuilt)
//	//println("State")
//	//fmt.Printf("ServerHello: %+v\n", currentSSLHelper.tlsConn.HandshakeState.ServerHello)
//	//fmt.Printf("ClientHello: %+v\n", currentSSLHelper.tlsConn.HandshakeState.Hello)
//	//fmt.Printf("MasterSecret: %+v\n", currentSSLHelper.tlsConn.HandshakeState.MasterSecret)
//	//fmt.Printf("Session: %+v\n", currentSSLHelper.tlsConn.HandshakeState.Session)
//	//fmt.Printf("State12: %+v\n", currentSSLHelper.tlsConn.HandshakeState.State12)
//	//fmt.Printf("State13: %+v\n", currentSSLHelper.tlsConn.HandshakeState.State13)
//	//fmt.Printf("conn: %+v\n", currentSSLHelper.tlsConn.Conn)
//
//	return nil
//}

// will return ClientHello
func startSSLHandshakeJS(_ js.Value, args []js.Value) interface{} {
	if currentConnection != nil {
		println("we have already started the connection")
		return nil
	}
	sni := args[0].String()
	return startSSLHandshake(sni)
}

func injectServerData(_ js.Value, args []js.Value) interface{} {
	if currentConnection == nil {
		println("we haven't started any connection yet")
		return nil
	}

	hexData := args[0].String()
	decoded, err := hex.DecodeString(hexData)
	if err != nil {
		panic(err)
	}

	currentConnection.connectionInjector.injectedServerData <- decoded
	return nil
}

func tryReadClientData(_ js.Value, _ []js.Value) interface{} {
	if currentConnection == nil {
		println("we haven't started any connection yet")
		return nil
	}

	select {
	case data := <-currentConnection.connectionInjector.createdClientData:
		encoded := hex.EncodeToString(data)
		return encoded
	default:
		Info("there wasn't any data available to read")
		return nil
	}
}

// TODO: promisify, etc.
func startConnection(_ js.Value, args []js.Value) interface{} {
	//sni := args[0].String()
	endpoint := args[0].String()
	//tlsConfig := tlsConfig(sni)

	if currentConnection != nil {
		Error("only a single connection can be established at a time (for now)")
		return fmt.Errorf("duplicate connection")
	}

	//endpoint := "https://nymtech.net/.wellknown/wallet/validators.json"
	//endpoint := "http://localhost:12345"
	client := &http.Client{
		Transport: &http.Transport{
			Proxy: func(req *http.Request) (*url.URL, error) {

				println("proxy")
				return nil, nil
			},
			OnProxyConnectResponse: func(ctx context.Context, proxyURL *url.URL, connectReq *http.Request, connectRes *http.Response) error {
				println("OnProxyConnectResponse")
				return nil
			},
			DialContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
				println("DialContext")
				setupFakePlainConn()
				return currentConnection.plainConn, nil
			},

			DialTLSContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
				println("DialTLSContext")
				sni := "placeholder"
				setupFakeTlsConn(sni)
				performSSLHandshake()
				return currentConnection.tlsConn, nil

			},

			//TLSClientConfig: &tlsConfig,

			DisableKeepAlives: true,

			MaxIdleConns:        1,
			MaxIdleConnsPerHost: 1,
			MaxConnsPerHost:     1,
		},
	}

	go func() {
		println("starting a GET")
		req, err := http.NewRequest(http.MethodGet, endpoint, nil)
		if err != nil {
			panic(err)
		}
		res, err := client.Do(req)

		//res, err := client.Get(endpoint)
		foomp := fmt.Sprintf("%+v", res)
		b, err := io.ReadAll(res.Body)
		// b, err := ioutil.ReadAll(resp.Body)  Go.1.15 and earlier
		if err != nil {
			log.Fatalln(err)
		}

		js.Global().Call("onClientData", foomp)
		js.Global().Call("onClientData", string(b))

		fmt.Printf("res: %+v\n", res)
		fmt.Printf("%+v", err)
	}()

	return nil
}
