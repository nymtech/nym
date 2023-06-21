// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"context"
	"errors"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/url"
	"strconv"
	"sync"
	"syscall/js"
)

type ConnectionId = uint64

type ActiveRequests struct {
	sync.Mutex
	inner map[ConnectionId]*ManagedConnection
}

func (ar *ActiveRequests) exists(id ConnectionId) bool {
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	return exists
}

func (ar *ActiveRequests) insert(id ConnectionId, conn *ManagedConnection) {
	ar.Lock()
	defer ar.Unlock()
	ar.inner[id] = conn
}

func (ar *ActiveRequests) remove(id ConnectionId) {
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	if !exists {
		panic("attempted to remove active connection that doesn't exist")
	}
	delete(ar.inner, id)
}

func (ar *ActiveRequests) injectData(id ConnectionId, data []byte) {
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	if !exists {
		panic("attempted to write to connection that doesn't exist")
	}
	ar.inner[id].connectionInjector.injectedServerData <- data
}

func (ar *ActiveRequests) closeRemoteSocket(id ConnectionId) {
	println("closing remote socket")
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	if !exists {
		panic("attempted to close remote socket of a connection that doesn't exist")
	}
	ar.inner[id].connectionInjector.closedRemote.Store(true)
}

func buildHttpClient(connectionId ConnectionId) *http.Client {
	if _, exists := activeRequests.inner[connectionId]; exists {
		panic("duplicate connection detected")
	}

	return &http.Client{
		Transport: &http.Transport{
			DialContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
				println("DialContext")

				if activeRequests.exists(connectionId) {
					return nil, errors.New("duplicate plain connection detected")
				}

				conn := setupFakePlainConn(connectionId)
				activeRequests.insert(connectionId, &conn)

				return conn.plainConn, nil
			},

			DialTLSContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
				println("DialTLSContext")

				if activeRequests.exists(connectionId) {
					return nil, errors.New("duplicate SSL connection detected")
				}

				conn := setupFakeTlsConn(connectionId)
				activeRequests.insert(connectionId, &conn)

				if err := conn.tlsConn.Handshake(); err != nil {
					return nil, err
				}

				performSSLHandshake()
				return currentConnection.tlsConn, nil
			},

			//TLSClientConfig: &tlsConfig,

			DisableKeepAlives:   true,
			MaxIdleConns:        1,
			MaxIdleConnsPerHost: 1,
			MaxConnsPerHost:     1,
		},
	}
}

func closeRemoteSocket(_ js.Value, args []js.Value) any {
	rawConnectionId := args[0].String()
	connectionId, err := strconv.ParseUint(rawConnectionId, 10, 64)
	if err != nil {
		panic(err)
	}

	activeRequests.closeRemoteSocket(connectionId)
	return nil
}

func makeRequest(connectionId ConnectionId, rawEndpoint string) (*http.Response, error) {
	// we just want to parse the url to make sure its valid
	_, err := url.Parse(rawEndpoint)
	if err != nil {
		return nil, err
	}

	// build the request
	Info("Building request for %s", rawEndpoint)
	// TODO: deal with other http methods later
	req, err := http.NewRequest(http.MethodGet, rawEndpoint, nil)
	if err != nil {
		println("can't build: ")
		println(err)
		return nil, err
	}
	reqClient := buildHttpClient(connectionId)

	Info("Starting the request...")
	return reqClient.Do(req)
}

func mixFetch(_ js.Value, args []js.Value) (any, error) {
	println("makeRequestPromise: start")
	// TODO: arg checks
	rawConnectionId := args[0].String()
	endpoint := args[1].String()

	connectionId, err := strconv.ParseUint(rawConnectionId, 10, 64)
	if err != nil {
		panic(err)
	}

	fmt.Printf("GO: got %d and %s\n", connectionId, endpoint)

	resp, err := makeRequest(connectionId, endpoint)
	println("finished go request")
	if err != nil {
		println("but it errored out")
		println(err)
		return nil, err
	}
	fmt.Printf("go response: %+v\n", resp)

	defer func(Body io.ReadCloser) {
		println("defer")
		err := Body.Close()
		if err != nil {
			// TODO: unimplemented
			panic(err)
		}
	}(resp.Body)

	// Read the response body
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		println("failed to read all")
		return nil, err
	}
	println("read content")

	// "data" is a byte slice, so we need to convert it to a JS Uint8Array object
	arrayConstructor := js.Global().Get("Uint8Array")
	dataJS := arrayConstructor.New(len(data))
	js.CopyBytesToJS(dataJS, data)
	// Create a Response object and pass the data
	responseConstructor := js.Global().Get("Response")
	response := responseConstructor.New(dataJS)

	// TODO: insert headers, status codes, etc.

	println("returning response from go")
	return response, nil
}

func makeRequestPromise(_ js.Value, args []js.Value) any {
	println("makeRequestPromise: start")
	// TODO: arg checks
	connectionId := uint64(args[0].Int())
	endpoint := args[1].String()

	handler := js.FuncOf(func(this js.Value, args []js.Value) any {
		println("makeRequestPromise: inside promise handler")

		resolve := args[0]
		reject := args[1]

		go func() {
			println("makeRequestPromise: inside goroutine")

			resp, err := makeRequest(connectionId, endpoint)
			if err != nil {
				// Handle errors: reject the Promise if we have an error
				errorConstructor := js.Global().Get("Error")
				errorObject := errorConstructor.New(err.Error())
				reject.Invoke(errorObject)
				return
			}

			defer func(Body io.ReadCloser) {
				err := Body.Close()
				if err != nil {
					// TODO: unimplemented
					panic(err)
				}
			}(resp.Body)

			// Read the response body
			data, err := io.ReadAll(resp.Body)
			if err != nil {
				// Handle errors here too
				errorConstructor := js.Global().Get("Error")
				errorObject := errorConstructor.New(err.Error())
				reject.Invoke(errorObject)
				return
			}

			// "data" is a byte slice, so we need to convert it to a JS Uint8Array object
			arrayConstructor := js.Global().Get("Uint8Array")
			dataJS := arrayConstructor.New(len(data))
			js.CopyBytesToJS(dataJS, data)

			// Create a Response object and pass the data
			responseConstructor := js.Global().Get("Response")
			response := responseConstructor.New(dataJS)

			// Resolve the Promise
			resolve.Invoke(response)
		}()

		println("makeRequestPromise: inside promise handler - out")

		// The handler of a Promise doesn't return any value
		return nil
	})

	println("makeRequestPromise: creating promise")
	// Create and return the Promise object
	promiseConstructor := js.Global().Get("Promise")
	return promiseConstructor.New(handler)
}
