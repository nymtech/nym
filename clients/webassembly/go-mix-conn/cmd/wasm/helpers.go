// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"bytes"
	"errors"
	"fmt"
	"io"
	"net/http"
	"strconv"
	"syscall/js"
)

type jsFn func(this js.Value, args []js.Value) (any, error)

var (
	jsErr     = js.Global().Get("Error")
	jsPromise = js.Global().Get("Promise")
)

type Redirect = string

const (
	REQUEST_REDIRECT_ERROR  = "error"
	REQUEST_REDIRECT_MANUAL = "manual"
	REQUEST_REDIRECT_FOLLOW = "follow"
)

type ParsedRequest struct {
	request  *http.Request
	redirect Redirect
}

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

func checkUnsupportedAttributes(request *js.Value) {
	cache := request.Get("cache")
	fmt.Printf("%+v", cache)
}

func parseHeaders(headers js.Value) (http.Header, error) {
	goHeaders := http.Header{}

	if headers.Type() != js.TypeObject {
		return nil, errors.New("the request headers is not an object")
	}
	headersIter := headers.Call("entries")

	for {
		next := headersIter.Call("next")
		done := next.Get("done").Bool()
		if done {
			return goHeaders, nil
		}
		keyValue := next.Get("value")
		if keyValue.Length() != 2 {
			return nil, errors.New("one of the headers has invalid length")
		}
		key := keyValue.Index(0)
		if key.Type() != js.TypeString {
			return nil, errors.New("one of the header keys is not a string")
		}

		value := keyValue.Index(1)
		if value.Type() != js.TypeString {
			return nil, errors.New("one of the header values is not a string")
		}
		ck := http.CanonicalHeaderKey(key.String())
		goHeaders[ck] = append(goHeaders[ck], value.String())
	}
}

func parseBody(request *js.Value) (io.Reader, error) {
	jsBody := request.Get("body")
	var bodyReader io.Reader
	if !jsBody.IsUndefined() && !jsBody.IsNull() {
		Debug("stream body - getReader")
		bodyReader = &streamReader{stream: jsBody.Call("getReader")}
	} else {
		Debug("unstremable body - fallback to ArrayBuffer")
		// Fall back to using ArrayBuffer
		// https://developer.mozilla.org/en-US/docs/Web/API/Body/arrayBuffer
		bodyReader = &arrayReader{arrayPromise: request.Call("arrayBuffer")}
	}

	bodyBytes, err := io.ReadAll(bodyReader)
	if err != nil {
		return nil, err
	}
	// TODO: that seems super awkward. we're constructing a reader only to fully consume it
	// and create it all over again so that we the recipient wouldn't complain about content-length
	// surely there must be a better way?
	return bytes.NewReader(bodyBytes), nil
}

func parseRedirect(request *js.Value) (string, error) {
	redirect := request.Get("redirect")
	if redirect.IsUndefined() || redirect.IsNull() {
		// if redirect is not specified, the default behaviour is 'follow'
		// Reference: https://developer.mozilla.org/en-US/docs/Web/API/WindowOrWorkerGlobalScope/fetch#Parameters
		return REQUEST_REDIRECT_FOLLOW, nil
	}

	if redirect.Type() != js.TypeString {
		return "", errors.New("the redirect field is not a string")
	}

	redirectString := redirect.String()
	if redirectString == REQUEST_REDIRECT_FOLLOW {
		return REQUEST_REDIRECT_FOLLOW, nil
	}
	if redirectString == REQUEST_REDIRECT_MANUAL {
		return REQUEST_REDIRECT_MANUAL, nil
	}
	if redirectString == REQUEST_REDIRECT_ERROR {
		return REQUEST_REDIRECT_ERROR, nil
	}

	return "", errors.New(fmt.Sprintf("%s is not a valid redirect", redirectString))
}

// ParseJSRequest is a reverse of https://github.com/golang/go/blob/release-branch.go1.21/src/net/http/roundtrip_js.go#L91
// https://developer.mozilla.org/en-US/docs/Web/API/request
/*
	request attributes and their implementation status:
	[✅] - supported
	[⚠️] - partially supported (some features might be missing)
	[❌] - unsupported
	[❗] - not applicable / will not support

	[⚠️] body			- the current streaming is a bit awkward
	[❌] bodyUsed
	[❗️] cache
	[❌] credentials
	[❌] destination
	[⚠️] headers		- not all headers are properly respected
	[❌] integrity
	[✅] method
	[❌] mode
	[⚠️] redirect		- "manual" is not implemented
	[❌] referrer
	[❌] referrerPolicy
	[❌] signal
	[✅] url
*/
func parseJSRequest(request js.Value) (*ParsedRequest, error) {
	// https://github.com/mozilla/gecko-dev/blob/d307d4d9f06dab6d16e963a4318e5e8ff4899141/dom/fetch/Fetch.cpp#L501
	// https://github.com/mozilla/gecko-dev/blob/d307d4d9f06dab6d16e963a4318e5e8ff4899141/dom/fetch/Request.cpp#L270

	method, err := getStringProperty(&request, "method")
	if err != nil {
		return nil, err
	}

	requestUrl, err := getStringProperty(&request, "url")
	if err != nil {
		return nil, err
	}

	jsHeaders := request.Get("headers")
	headers, err := parseHeaders(jsHeaders)
	if err != nil {
		return nil, err
	}

	body, err := parseBody(&request)
	if err != nil {
		return nil, err
	}
	redirect, err := parseRedirect(&request)
	if err != nil {
		return nil, err
	}

	req, err := http.NewRequest(method, requestUrl, body)
	if err != nil {
		return nil, err
	}
	req.Header = headers

	Debug("constructed request: %+v", req)

	return &ParsedRequest{
		request:  req,
		redirect: redirect,
	}, nil
}

// IntoJSResponse is a reverse of https://github.com/golang/go/blob/release-branch.go1.21/src/net/http/roundtrip_js.go#L91
// https://developer.mozilla.org/en-US/docs/Web/API/response
/*
	request attributes and their implementation status:
	[✅] - supported
	[⚠️] - partially supported (some features might be missing)
	[❌] - unsupported
	[❗] - not applicable / will not support

	[⚠️] body			- response streaming via ReadableStream is unsupported (TODO: look into https://github.com/golang/go/blob/release-branch.go1.21/src/net/http/roundtrip_js.go#L152-L195 to implement it)
	[✅] bodyUsed
	[✅] headers
	[✅] ok 			- seems to be handled automagically (presumably via `status`)
	[❌] redirected
	[✅] status
	[✅] statusText
	[❌] type
	[❌] url
*/
func intoJSResponse(resp *http.Response) (js.Value, error) {
	defer func(Body io.ReadCloser) {
		err := Body.Close()
		if err != nil {
			Error("failed to close the response body: %s", err)
		}
	}(resp.Body)

	// Read the response body
	// TODO: construct streamReader / arrayReader for better compat
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return js.Null(), err
	}

	jsBytes := intoJsBytes(data)

	// Create a Response object and pass the data
	// inspired by https://github.com/golang/go/blob/release-branch.go1.21/src/net/http/roundtrip_js.go#L91
	headers := js.Global().Get("Headers").New()
	for key, values := range resp.Header {
		for _, value := range values {
			headers.Call("append", key, value)
		}
	}

	responseOptions := js.Global().Get("Object").New()
	responseOptions.Set("status", resp.StatusCode)
	responseOptions.Set("statusText", http.StatusText(resp.StatusCode))
	responseOptions.Set("headers", headers)

	responseConstructor := js.Global().Get("Response")
	response := responseConstructor.New(jsBytes, responseOptions)

	return response, nil
}
