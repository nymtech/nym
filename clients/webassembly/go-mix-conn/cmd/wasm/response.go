// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"io"
	"net/http"
	"net/url"
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

type InternalResponse struct {
	inner                 *http.Response
	responseTainting      ResponseTainting
	responseType          ResponseType
	corsExposedHeaderName []string
	urlList               []*url.URL
}

func NewInternalResponse(inner *http.Response, reqOpts *RequestOptions) InternalResponse {
	return InternalResponse{
		inner:            inner,
		responseTainting: reqOpts.responseTainting,
	}
}

// Reference: https://fetch.spec.whatwg.org/#null-body-status
func (IR *InternalResponse) isNullBodyStatus() bool {
	return IR.inner.StatusCode == 101 || IR.inner.StatusCode == 103 || IR.inner.StatusCode == 204 || IR.inner.StatusCode == 205 || IR.inner.StatusCode == 304
}

func (IR *InternalResponse) allHeaderNames() []string {
	keys := make([]string, 0, len(IR.inner.Header))

	for k, _ := range IR.inner.Header {
		keys = append(keys, k)
	}
	return keys
}

func (IR *InternalResponse) JSBody() (js.Value, error) {
	if IR.inner.Body == nil {
		return js.Undefined(), nil
	}

	defer func(Body io.ReadCloser) {
		err := Body.Close()
		if err != nil {
			Error("failed to close the response body: %s", err)
		}
	}(IR.inner.Body)

	// Read the response body
	// TODO: construct streamReader / arrayReader for better compat
	data, err := io.ReadAll(IR.inner.Body)
	if err != nil {
		return js.Undefined(), err
	}

	return intoJsBytes(data), nil
}

func (IR *InternalResponse) exposeHeadersNames() []string {
	allowed := IR.inner.Header.Values(headerExposeHeaders)

	allowedSet := NewSet(allowed...)
	var filtered []string
	for key, _ := range IR.inner.Header {
		if allowedSet.Contains(key) {
			filtered = append(filtered, key)
		}
	}

	return filtered
}

// Reference: https://fetch.spec.whatwg.org/#concept-filtered-response-cors
func (IR *InternalResponse) mutIntoBasicResponse() {
	IR.responseType = RESPONSE_TYPE_BASIC

	newHeaders := http.Header{}
	for key, values := range IR.inner.Header {
		for _, value := range values {
			if !forbiddenResponseHeaderNames.Contains(byteLowercase(key)) {
				ck := http.CanonicalHeaderKey(key)
				newHeaders[ck] = append(newHeaders[ck], value)
			}
		}
	}
	IR.inner.Header = newHeaders
}

// Reference: https://fetch.spec.whatwg.org/#concept-filtered-response-cors
func (IR *InternalResponse) mutIntoCORSResponse() {
	IR.responseType = RESPONSE_TYPE_CORS

	// TODO: figure out the proper usage of corsExposedHeaderName

	// https://github.com/mozilla/gecko-dev/blob/fabab5d10815c9d7210933379f0357b1cbc9aaaf/dom/fetch/InternalHeaders.cpp#L564
	newHeaders := http.Header{}
	//for _, name := range IR.corsExposedHeaderName {
	//	if safelistedResponseHeaderNames.Contains(byteLowercase(name)) {
	//		ck := http.CanonicalHeaderKey(name)
	//		newHeaders[ck] = IR.inner.Header.Values(ck)
	//	}
	//}

	for key, values := range IR.inner.Header {
		for _, value := range values {
			if safelistedResponseHeaderNames.Contains(byteLowercase(key)) {
				ck := http.CanonicalHeaderKey(key)
				newHeaders[ck] = append(newHeaders[ck], value)
			}
		}
	}

	IR.inner.Header = newHeaders
}

// Reference: https://fetch.spec.whatwg.org/#concept-filtered-response-opaque
func (IR *InternalResponse) mutIntoOpaqueResponse() {
	IR.responseType = RESPONSE_TYPE_OPAQUE

	// TODO: set URL list to « »
	IR.inner.StatusCode = 0
	IR.inner.Status = ""
	IR.inner.Header = http.Header{}
	IR.inner.Body = nil
}

// Reference: https://fetch.spec.whatwg.org/#concept-filtered-response-cors
func (IR *InternalResponse) mutIntoOpaqueRedirectResponse() {
	IR.responseType = RESPONSE_TYPE_OPAQUE_REDIRECT

	IR.inner.StatusCode = 0
	IR.inner.Status = ""
	IR.inner.Header = http.Header{}
	IR.inner.Body = nil
}

func (IR *InternalResponse) intoJsResponse() (js.Value, error) {
	jsBody, err := IR.JSBody()
	if err != nil {
		return js.Undefined(), err
	}

	// Create a Response object and pass the data
	// inspired by https://github.com/golang/go/blob/release-branch.go1.21/src/net/http/roundtrip_js.go#L91
	headers := js.Global().Get("Headers").New()
	for key, values := range IR.inner.Header {
		for _, value := range values {
			headers.Call("append", key, value)
		}
	}

	responseOptions := js.Global().Get("Object").New()
	responseOptions.Set("status", IR.inner.StatusCode)
	responseOptions.Set("statusText", http.StatusText(IR.inner.StatusCode))
	responseOptions.Set("headers", headers)

	responseConstructor := js.Global().Get("Response")
	response := responseConstructor.New(jsBody, responseOptions)
	if IR.responseType != "" {
		// TODO: ideally we'd overwrite the `type` field, but it's readonly : (
		response.Set("actualType", IR.responseType)
	}

	return response, nil
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
	[❗/⚠️] type		- can't overwrite the "type" field. the proper value is set in a `actualType` instead.
	[❌] url
*/
func intoJSResponse(resp *http.Response, opts *RequestOptions) (js.Value, error) {
	// TODO: check if response is a filtered response
	isFilteredResponse := false

	internalResponse := NewInternalResponse(resp, opts)

	// 4.1.14
	if !isFilteredResponse {
		// 4.1.14.1
		if opts.responseTainting == RESPONSE_TAINTING_CORS {
			//  4.1.14.1.1
			headerNames := internalResponse.exposeHeadersNames()
			if opts.credentialsMode != CREDENTIALS_MODE_INCLUDE && contains(headerNames, wildcard) {
				//  4.1.14.1.2
				internalResponse.corsExposedHeaderName = unique(internalResponse.allHeaderNames())
			} else if len(headerNames) > 0 {
				//  4.1.14.1.3
				internalResponse.corsExposedHeaderName = headerNames
			}
		}
		switch opts.responseTainting {
		case RESPONSE_TAINTING_BASIC:
			internalResponse.mutIntoBasicResponse()
		case RESPONSE_TAINTING_CORS:
			internalResponse.mutIntoCORSResponse()
		case RESPONSE_TAINTING_OPAQUE:
			internalResponse.mutIntoOpaqueResponse()
		default:
			panic("unreachable")
		}
	}

	if len(internalResponse.urlList) == 0 {
		internalResponse.urlList = []*url.URL{internalResponse.inner.Request.URL}
	}
	// TODO: 4.1.17 - redirect-tainted origin
	// TODO: 4.1.18 - timing allow flag

	// TODO: 4.1.19 - mixed content check
	// TODO: 4.1.19 - Content Security Policy check
	// TODO: 4.1.19 - MIME type check
	// TODO: 4.1.19 - nosniff check

	// TODO: 4.1.20
	//rangeRequestedFlag := false
	//if internalResponse.responseType == RESPONSE_TYPE_OPAQUE && internalResponse.inner.Status == 206

	// 4.1.21
	if (opts.method == "HEAD" || opts.method == "CONNECT") || internalResponse.isNullBodyStatus() {
		internalResponse.inner.Body = nil
	}

	return internalResponse.intoJsResponse()
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
