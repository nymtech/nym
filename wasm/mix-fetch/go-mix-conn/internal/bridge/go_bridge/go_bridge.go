// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package go_bridge

import (
	"errors"
	"fmt"
	"go-mix-conn/internal/bridge/rust_bridge"
	"go-mix-conn/internal/helpers"
	"go-mix-conn/internal/jstypes"
	"go-mix-conn/internal/jstypes/conv"
	"go-mix-conn/internal/log"
	"go-mix-conn/internal/mixfetch"
	"syscall/js"
	"time"
)

const (
	// methods exposed by go to rust
	goRustBridgeName = "__go_rs_bridge__"
)

func createGoBridgeObject() js.Value {
	js.Global().Set(goRustBridgeName, jstypes.Object.New())
	goBridgeRoot := js.Global().Get(goRustBridgeName)
	return goBridgeRoot
}

func InitialiseGoBridge() {
	goBridgeRoot := createGoBridgeObject()

	// user facing methods
	js.Global().Set("mixFetch", jstypes.AsyncFunc(mixFetch))
	js.Global().Set("goWasmSetLogging", js.FuncOf(setLoggingLevel))

	// rust facing methods (don't expose them to the root)
	goBridgeRoot.Set("goWasmSetMixFetchRequestTimeout", js.FuncOf(changeRequestTimeout))
	goBridgeRoot.Set("goWasmInjectServerData", js.FuncOf(injectServerData))
	goBridgeRoot.Set("goWasmCloseRemoteSocket", js.FuncOf(closeRemoteSocket))
	goBridgeRoot.Set("goWasmInjectConnError", js.FuncOf(injectConnError))
}

func setLoggingLevel(_ js.Value, args []js.Value) any {
	if len(args) == 0 {
		return errors.New("no arguments passed for `setLoggingLevel`")
	}

	if args[0].Type() != js.TypeString {
		return errors.New("the provided logging level is not a string")
	}
	log.SetLoggingLevel(args[0].String())
	return nil
}

func mixFetch(_ js.Value, args []js.Value) (any, error) {
	if !rust_bridge.RsIsInitialised() {
		return nil, errors.New("mix fetch is not available")
	}

	if len(args) == 0 {
		return nil, errors.New("no arguments passed for `mixfetch`")
	}

	requestConstructor := jstypes.Request

	jsRequest := js.Null()
	unsafeCors := false
	// that's bit weird. can't use the spread operator
	if len(args) == 1 {
		jsRequest = requestConstructor.New(args[0])
	}
	if len(args) == 2 {
		if !args[1].IsUndefined() && !args[1].IsNull() {
			// check for 'MODE_UNSAFE_IGNORE_CORS'
			if args[1].Get("mode").String() == jstypes.ModeUnsafeIgnoreCors {
				unsafeCors = true
				// we need to delete that prop as technically it holds an invalid value to construct `Request`
				args[1].Delete("mode")
			}
		}

		jsRequest = requestConstructor.New(args[0], args[1])
	}

	goRequest, err := conv.ParseJSRequest(jsRequest, unsafeCors)
	if err != nil {
		return nil, err
	}

	return mixfetch.MixFetch(goRequest)
}

func injectServerData(_ js.Value, args []js.Value) any {
	if len(args) != 2 {
		return errors.New(fmt.Sprintf("received invalid number of arguments. Got %d but expected 2", len(args)))
	}
	requestId, err := helpers.ParseRequestId(args[0])
	if err != nil {
		return err
	}
	data, err := helpers.IntoGoBytes(args[1])
	if err != nil {
		return err
	}

	return mixfetch.InjectServerData(requestId, data)
}

func injectConnError(_ js.Value, args []js.Value) any {
	if len(args) != 2 {
		return errors.New(fmt.Sprintf("received invalid number of arguments. Got %d but expected 2", len(args)))
	}

	requestId, err := helpers.ParseRequestId(args[0])
	if err != nil {
		return err
	}

	if args[1].Type() != js.TypeString {
		return errors.New("provided error message is not a string")
	}
	errMsg := args[1].String()
	remoteErr := errors.New(errMsg)

	return mixfetch.InjectConnError(requestId, remoteErr)
}

func closeRemoteSocket(_ js.Value, args []js.Value) any {
	if len(args) != 1 {
		return errors.New(fmt.Sprintf("received invalid number of arguments. Got %d but expected 1", len(args)))
	}
	requestId, err := helpers.ParseRequestId(args[0])
	if err != nil {
		return err
	}

	return mixfetch.CloseRemoteSocket(requestId)
}

func changeRequestTimeout(_ js.Value, args []js.Value) any {
	if len(args) != 1 {
		return errors.New(fmt.Sprintf("received invalid number of arguments. Got %d but expected 1", len(args)))
	}

	if args[0].Type() != js.TypeNumber {
		return errors.New("the provided timeout is not a number")
	}
	timeoutMs := args[0].Int()
	timeout := time.Millisecond * time.Duration(timeoutMs)

	return mixfetch.ChangeRequestTimeout(timeout)
}
