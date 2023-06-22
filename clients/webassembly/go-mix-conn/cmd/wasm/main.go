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
		inner: make(map[RequestId]*ConnectionInjector),
	}
	println("[go init]: go module init finished")
}

func main() {
	println("[go main]: go module loaded")

	js.Global().Set("goWasmInjectServerData", js.FuncOf(injectServerData))
	js.Global().Set("goWasmCloseRemoteSocket", js.FuncOf(closeRemoteSocket))
	js.Global().Set("goWasmMixFetch", asyncFunc(mixFetch))

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

// TODO: change signature of that to allow the proper js.Request with RequestInit, etc.
func mixFetch(_ js.Value, args []js.Value) (any, error) {
	if len(args) != 2 {
		return nil, errors.New(fmt.Sprintf("received invalid number of arguments. Got %d but expected 2", len(args)))
	}

	requestId, err := parseRequestId(args[0])
	if err != nil {
		return nil, err
	}
	if args[1].Type() != js.TypeObject {
		return nil, errors.New("the received raw request was not an object")
	}

	request, err := parseRequest( args[1])
	if err != nil {
		return nil, err
	}

	return _mixFetch(requestId, request)
}
