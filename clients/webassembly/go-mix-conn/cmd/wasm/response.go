// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"io"
	"net/http"
	"syscall/js"
)

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
