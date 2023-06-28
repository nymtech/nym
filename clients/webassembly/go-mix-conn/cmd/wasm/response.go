// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"io"
	"net/http"
	"syscall/js"
)

type ResponseType = string

const (
	RESPONSE_TYPE_BASIC           = "basic"
	RESPONSE_TYPE_CORS            = "cors"
	RESPONSE_TYPE_DEFAULT         = "default"
	RESPONSE_TYPE_ERROR           = "error"
	RESPONSE_TYPE_OPAQUE          = "opaque"
	RESPONSE_TYPE_OPAQUE_REDIRECT = "opaqueredirect"
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

func checkCorsHeaders(headers *http.Header) {
	// in "no-cors"
	// you can: var simpleMethods = ["GET", "HEAD", "POST"];
	// you can't var otherMethods = ["DELETE", "OPTIONS", "PUT"];
	// https://fetch.spec.whatwg.org/#cors-safelisted-request-header

	//// Indicates whether the response can be shared, via returning the literal value of the `Origin` request header (which can be `null`) or `*` in a response.
	//allowOrigin := headers.Get("Access-Control-Allow-Origin")
	//
	//// Indicates whether the response can be shared when request’s credentials mode is "include".
	//allowCredentials := headers.Get("Access-Control-Allow-Credentials")
	//
	//// Indicates which methods are supported by the response’s URL for the purposes of the CORS protocol.
	//allowMethods := headers.Get("Access-Control-Allow-Methods")
	//
	//// Indicates which headers are supported by the response’s URL for the purposes of the CORS protocol.
	//allowHeaders := headers.Get("Access-Control-Allow-Headers")
	//
	//// Indicates the number of seconds (5 by default) the information provided by the `Access-Control-Allow-Methods` and `Access-Control-Allow-Headers` headers can be cached.
	//maxAge := headers.Get("Access-Control-Max-Age")
	//if maxAge != "" {
	//	Warn("\"Access-Control-Max-Age\" header is present on the remote, however its handling is currently unimplemented!")
	//}
	//
	//// Indicates which headers can be exposed as part of the response by listing their names.
	//exposeHeaders := headers.Get("Access-Control-Expose-Headers")
	//if exposeHeaders != "" {
	//	Warn("\"Access-Control-Expose-Headers\" header is present on the remote, however its handling is currently unimplemented!")
	//}
}

//func checkResponseTainting(reqOpts RequestOptions) ResponseTainting {
//	// Unless stated otherwise, it is "basic".
//	// Reference: https://fetch.spec.whatwg.org/#concept-request-response-tainting
//	responseTainting := REQUEST_RESPONSE_TAINTING_BASIC
//	if reqOpts.mode == MODE_NAVIGATE || reqOpts.mode == MODE_WEBSOCKET {
//		responseTainting = REQUEST_RESPONSE_TAINTING_BASIC
//	}
//}
