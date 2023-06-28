// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"errors"
	"fmt"
	"strconv"
	"syscall/js"
)

type jsFn func(this js.Value, args []js.Value) (any, error)

var (
	jsErr     = js.Global().Get("Error")
	jsPromise = js.Global().Get("Promise")
	origin    = js.Global().Get("location").Get("origin").String()
)

// AsyncFunc converts a Go-JS function into a Promise
func asyncFunc(innerFunc jsFn) js.Func {
	return js.FuncOf(func(this js.Value, args []js.Value) any {
		handler := js.FuncOf(func(_ js.Value, promFn []js.Value) any {
			resolve, reject := promFn[0], promFn[1]

			go func() {
				defer func() {
					if r := recover(); r != nil {
						reject.Invoke(jsErr.New(fmt.Sprint("panic:", r)))
					}
				}()

				res, err := innerFunc(this, args)
				if err != nil {
					reject.Invoke(jsErr.New(err.Error()))
				} else {
					resolve.Invoke(res)
				}
			}()

			return nil
		})

		return jsPromise.New(handler)
	})
}

// https://stackoverflow.com/a/68427221
func await(awaitable js.Value) ([]js.Value, []js.Value) {
	then := make(chan []js.Value)
	defer close(then)
	thenFunc := js.FuncOf(func(this js.Value, args []js.Value) any {
		then <- args
		return nil
	})
	defer thenFunc.Release()

	catch := make(chan []js.Value)
	defer close(catch)
	catchFunc := js.FuncOf(func(this js.Value, args []js.Value) any {
		catch <- args
		return nil
	})
	defer catchFunc.Release()

	awaitable.Call("then", thenFunc).Call("catch", catchFunc)

	select {
	case result := <-then:
		return result, nil
	case err := <-catch:
		return nil, err
	}
}

func parseRequestId(raw js.Value) (uint64, error) {
	if raw.Type() != js.TypeString {
		return 0, errors.New("the received raw request id was not a string")
	}

	return strconv.ParseUint(raw.String(), 10, 64)
}

func intoGoBytes(raw js.Value) ([]byte, error) {
	if raw.Type() != js.TypeObject {
		return nil, errors.New("the received 'bytes' are not an object")
	}
	lenProp := raw.Get("length")
	if lenProp.Type() != js.TypeNumber {
		return nil, errors.New("the received 'bytes' object does not have a numerical 'length' property")
	}
	n := lenProp.Int()
	bytes := make([]byte, n)

	// TODO: somehow check that the object is an Uint8Array or Uint8ClampedArray
	copied := js.CopyBytesToGo(bytes, raw)
	if copied != n {
		// I don't see how this could ever be reached, thus panic
		panic("somehow copied fewer bytes from JavaScript into Go than what we specified as our buffer")
	}

	return bytes, nil
}

func intoJsBytes(raw []byte) js.Value {
	// "data" is a byte slice, so we need to convert it to a JS Uint8Array object
	arrayConstructor := js.Global().Get("Uint8Array")
	jsBytes := arrayConstructor.New(len(raw))
	js.CopyBytesToJS(jsBytes, raw)
	return jsBytes
}

func getStringProperty(obj *js.Value, name string) (string, error) {
	val := obj.Get(name)
	if val.Type() != js.TypeString {
		return "", errors.New(fmt.Sprintf("the property %s is not a string", name))
	}
	return val.String(), nil
}
