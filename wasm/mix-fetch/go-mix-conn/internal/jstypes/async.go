// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package jstypes

import (
	"fmt"
	"syscall/js"
)

type jsFn func(this js.Value, args []js.Value) (any, error)

// AsyncFunc converts a Go-JS function into a Promise
func AsyncFunc(innerFunc jsFn) js.Func {
	return js.FuncOf(func(this js.Value, args []js.Value) any {
		handler := js.FuncOf(func(_ js.Value, promFn []js.Value) any {
			resolve, reject := promFn[0], promFn[1]

			go func() {
				defer func() {
					if r := recover(); r != nil {
						reject.Invoke(Error.New(fmt.Sprint("panic:", r)))
					}
				}()

				res, err := innerFunc(this, args)
				if err != nil {
					reject.Invoke(Error.New(err.Error()))
				} else {
					resolve.Invoke(res)
				}
			}()

			return nil
		})

		return Promise.New(handler)
	})
}

// https://stackoverflow.com/a/68427221
func Await(awaitable js.Value) ([]js.Value, []js.Value) {
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
