// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//go:build js && wasm

package main

import (
	_ "net/http"
	"strconv"
	"sync"
	"syscall/js"
	"time"
)

// ALL THE GLOBALS SHOULD GO HERE
var done chan struct{}
var activeRequests *ActiveRequests

func init() {
	println("[go init]: go module init")
	SetupLogging()

	done = make(chan struct{})
	activeRequests = &ActiveRequests{
		Mutex: sync.Mutex{},
		inner: make(map[ConnectionId]*ManagedConnection),
	}
	println("[go init]: go module init finished")
}

func main() {
	println("[go main]: go module loaded")

	//js.Global().Set("debugGoWasmPrintConnection", js.FuncOf(printConnection))

	//js.Global().Set("goWasmStartSSLHandshake", js.FuncOf(startSSLHandshakeJS))
	//js.Global().Set("goWasmTryReadClientData", js.FuncOf(tryReadClientData))
	//
	//js.Global().Set("goWasmHTTPTest", js.FuncOf(startConnection))
	//js.Global().Set("startSSLHandshakeJS", js.FuncOf(startSSLHandshakeJS))
	//
	//js.Global().Set("goMakeFakeResponse", js.FuncOf(fakeResponse))
	//js.Global().Set("foompWithCallback", js.FuncOf(foompWithCallback))
	//js.Global().Set("foomp", js.FuncOf(testStuff))

	js.Global().Set("goWasmCallRust", js.FuncOf(callRust))

	js.Global().Set("goWasmInjectServerData", js.FuncOf(injectServerData))
	js.Global().Set("goWasmCloseRemoteSocket", js.FuncOf(closeRemoteSocket))
	js.Global().Set("goWasmMixFetch", asyncFunc(mixFetch))

	<-done

	println("[go main]: go module finished")
}

func callRust(this js.Value, args []js.Value) any {
	goBytes := []byte{1, 2, 3, 4}

	arr := js.Global().Get("Uint8Array").New(len(goBytes))
	js.CopyBytesToJS(arr, goBytes)

	connId := strconv.FormatUint(uint64(1234), 10)

	js.Global().Call("send_client_data", connId, arr)

	//bindgen := js.Global().Get("wasm_bindgen");
	//aa := bindgen.Get("send_client_data")
	//aa.Invoke("foo1", "foo2")
	return nil
}

func testStuff(this js.Value, args []js.Value) any {
	//foomp := args[0].j
	println("about to call the arg")
	args[0].Invoke()
	println("called")
	return nil

}

func foompWithCallback(_ js.Value, args []js.Value) any {
	go func() {
		println("sleeping...")
		time.Sleep(1 * time.Second)
		println("done sleeping")
		args[0].Invoke("foomp")
	}()
	return nil
}

//
//func printConnection(this js.Value, args []js.Value) any {
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
func startSSLHandshakeJS(_ js.Value, args []js.Value) any {
	//if currentConnection != nil {
	//	println("we have already started the connection")
	//	return nil
	//}
	//sni := args[0].String()
	//return startSSLHandshake(sni)
	return nil
}

func injectServerData(_ js.Value, args []js.Value) any {
	if activeRequests == nil {
		println("we haven't started any connection yet")
		return nil
	}

	rawRequestId := args[0].String()
	jsBytes := args[1]

	requestId, err := strconv.ParseUint(rawRequestId, 10, 64)
	if err != nil {
		panic(err)
	}

	data := make([]byte, jsBytes.Get("length").Int())
	n := js.CopyBytesToGo(data, jsBytes)
	if n != len(data) {
		panic("todo")
	}

	activeRequests.injectData(requestId, data)
	return nil

	//hexData := args[0].String()
	//decoded, err := hex.DecodeString(hexData)
	//if err != nil {
	//	panic(err)
	//}
	//
	//currentConnection.connectionInjector.injectedServerData <- decoded
	//return nil
}

//
//func tryReadClientData(_ js.Value, _ []js.Value) any {
//	if currentConnection == nil {
//		println("we haven't started any connection yet")
//		return nil
//	}
//
//	select {
//	case data := <-currentConnection.connectionInjector.createdClientData:
//		encoded := hex.EncodeToString(data)
//		return encoded
//	default:
//		Info("there wasn't any data available to read")
//		return nil
//	}
//}

func fakeResponse(_ js.Value, _ []js.Value) any {
	arrayConstructor := js.Global().Get("Uint8Array")
	data := []byte{'f', 'o', 'o', 'm', 'p'}
	dataJS := arrayConstructor.New(len(data))
	js.CopyBytesToJS(dataJS, data)
	responseConstructor := js.Global().Get("Response")
	response := responseConstructor.New(dataJS)
	return response
}

func promisify(job func() (any, error)) js.Value {
	handler := js.FuncOf(func(this js.Value, args []js.Value) any {
		resolve := args[0]
		reject := args[1]

		go func() {
			res, err := job()
			if err != nil {
				// Handle errors: reject the Promise if we have an error
				errorConstructor := js.Global().Get("Error")
				errorObject := errorConstructor.New(err.Error())
				reject.Invoke(errorObject)
				return
			} else {
				//// "data" is a byte slice, so we need to convert it to a JS Uint8Array object
				//arrayConstructor := js.Global().Get("Uint8Array")
				//dataJS := arrayConstructor.New(len(data))
				//js.CopyBytesToJS(dataJS, data)
				//
				//// Create a Response object and pass the data
				//responseConstructor := js.Global().Get("Response")
				//response := responseConstructor.New(dataJS)
				//
				//// Resolve the Promise
				//resolve.Invoke(response)
				resolve.Invoke(res)
				return
			}
		}()
		// The handler of a Promise doesn't return any value
		return nil
	})

	// Create and return the Promise object
	promiseConstructor := js.Global().Get("Promise")
	return promiseConstructor.New(handler)
}

//
//// TODO: promisify, etc.
//func startConnection(_ js.Value, args []js.Value) any {
//	//sni := args[0].String()
//	endpoint := args[0].String()
//	//tlsConfig := tlsConfig(sni)
//
//	if currentConnection != nil {
//		Error("only a single connection can be established at a time (for now)")
//		return fmt.Errorf("duplicate connection")
//	}
//
//	//endpoint := "https://nymtech.net/.wellknown/wallet/validators.json"
//	//endpoint := "http://localhost:12345"
//	client := &http.Client{
//		Transport: &http.Transport{
//			Proxy: func(req *http.Request) (*url.URL, error) {
//
//				println("proxy")
//				return nil, nil
//			},
//			OnProxyConnectResponse: func(ctx context.Context, proxyURL *url.URL, connectReq *http.Request, connectRes *http.Response) error {
//				println("OnProxyConnectResponse")
//				return nil
//			},
//			DialContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
//				println("DialContext")
//				setupFakePlainConn()
//				return currentConnection.plainConn, nil
//			},
//
//			DialTLSContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
//				println("DialTLSContext")
//				setupFakeTlsConn()
//				performSSLHandshake()
//				return currentConnection.tlsConn, nil
//			},
//
//			//TLSClientConfig: &tlsConfig,
//
//			DisableKeepAlives: true,
//
//			MaxIdleConns:        1,
//			MaxIdleConnsPerHost: 1,
//			MaxConnsPerHost:     1,
//		},
//	}
//
//	go func() {
//		println("starting a GET")
//		req, err := http.NewRequest(http.MethodGet, endpoint, nil)
//		if err != nil {
//			panic(err)
//		}
//		res, err := client.Do(req)
//
//		//res, err := client.Get(endpoint)
//		foomp := fmt.Sprintf("%+v", res)
//		b, err := io.ReadAll(res.Body)
//		// b, err := ioutil.ReadAll(resp.Body)  Go.1.15 and earlier
//		if err != nil {
//			log.Fatalln(err)
//		}
//
//		js.Global().Call("onClientData", foomp)
//		js.Global().Call("onClientData", string(b))
//
//		fmt.Printf("res: %+v\n", res)
//		fmt.Printf("%+v", err)
//	}()
//
//	return nil
//}
