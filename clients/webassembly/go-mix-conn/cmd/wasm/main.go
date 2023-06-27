// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//go:build js && wasm

package main

import (
	"errors"
	"fmt"
	_ "net/http"
	"sync"
	"syscall/js"
	"time"
)

// ALL THE GLOBALS SHOULD GO HERE
var done chan struct{}
var activeRequests *ActiveRequests
var requestTimeout time.Duration = time.Second * 5
var maxRedirections int = 10

const (
	// methods exposed by go to rust
	goRustBridgeName = "__go_rs_bridge__"

	// methods exposed by rust to go
	rustGoBridgeName = "__rs_go_bridge__"
)

func createGoBridgeObject() js.Value {
	js.Global().Set(goRustBridgeName, js.Global().Get("Object").New())
	goBridgeRoot := js.Global().Get(goRustBridgeName)
	return goBridgeRoot
}

func init() {
	println("[go init]: go module init")

	done = make(chan struct{})
	activeRequests = &ActiveRequests{
		Mutex: sync.Mutex{},
		inner: make(map[RequestId]*ActiveRequest),
	}
	println("[go init]: go module init finished")
}

func main() {
	println("[go main]: go module loaded")

	goBridgeRoot := createGoBridgeObject()

	// user facing methods
	js.Global().Set("mixFetch", asyncFunc(mixFetch))
	js.Global().Set("setMixFetchRequestTimeout", js.FuncOf(changeRequestTimeout))

	// rust facing methods (don't expose them to the root)
	goBridgeRoot.Set("goWasmInjectServerData", js.FuncOf(injectServerData))
	goBridgeRoot.Set("goWasmCloseRemoteSocket", js.FuncOf(closeRemoteSocket))
	goBridgeRoot.Set("goWasmInjectConnError", js.FuncOf(injectConnError))
	<-done

	println("[go main]: go module finished")
}

func injectServerData(_ js.Value, args []js.Value) any {
	if len(args) != 2 {
		return errors.New(fmt.Sprintf("received invalid number of arguments. Got %d but expected 2", len(args)))
	}
	requestId, err := parseRequestId(args[0])
	if err != nil {
		return err
	}
	data, err := intoGoBytes(args[1])
	if err != nil {
		return err
	}

	return _injectServerData(requestId, data)
}

func injectConnError(_ js.Value, args []js.Value) any {
	if len(args) != 2 {
		return errors.New(fmt.Sprintf("received invalid number of arguments. Got %d but expected 2", len(args)))
	}

	requestId, err := parseRequestId(args[0])
	if err != nil {
		return err
	}

	if args[1].Type() != js.TypeString {
		return errors.New("provided error message is not a string")
	}
	errMsg := args[1].String()
	remoteErr := errors.New(errMsg)

	return _injectConnError(requestId, remoteErr)
}

func closeRemoteSocket(_ js.Value, args []js.Value) any {
	if len(args) != 1 {
		return errors.New(fmt.Sprintf("received invalid number of arguments. Got %d but expected 1", len(args)))
	}
	requestId, err := parseRequestId(args[0])
	if err != nil {
		return err
	}

	return _closeRemoteSocket(requestId)
}

func changeRequestTimeout(_ js.Value, args []js.Value) any {
	if len(args) != 1 {
		return errors.New(fmt.Sprintf("received invalid number of arguments. Got %d but expected 1", len(args)))
	}

	return errors.New("unimplemented")
}

func mixFetch(_ js.Value, args []js.Value) (any, error) {
	if !rsIsInitialised() {
		return nil, errors.New("mix fetch hasn't been initialised")
	}

	if len(args) == 0 {
		return nil, errors.New("no arguments passed for `mixfetch`")
	}

	requestConstructor := js.Global().Get("Request")

	jsRequest := js.Null()
	// that's bit weird. can't use the spread operator
	if len(args) == 1 {
		jsRequest = requestConstructor.New(args[0])
	}
	if len(args) == 2 {
		jsRequest = requestConstructor.New(args[0], args[1])
	}

	goRequest, err := parseJSRequest(jsRequest)
	if err != nil {
		return nil, err
	}

	return _mixFetch(goRequest)
}
