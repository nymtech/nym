// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"fmt"
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

func proxyHandlerGet(proxied map[string]any) js.Func {
	return js.FuncOf(func(_ js.Value, args []js.Value) any {
		// Look at redirecting method's this:
		// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Proxy#no_private_property_forwarding
		target := args[0]
		prop := args[1].String()
		receiver := args[2]

		value := target.Get(prop)
		if value.Type() == js.TypeFunction {
			Debug("%s is a function", prop)
			return js.FuncOf(func(this js.Value, args []js.Value) any {
				if this.Equal(receiver) {
					return jsReflect.Call("apply", value, target, intoAnySlice(args))
				} else {
					return jsReflect.Call("apply", value, this, intoAnySlice(args))
				}
			})
		}

		// we're only proxing "normal" props, not function calls
		proxy, ok := proxied[prop]
		if ok {
			Debug("using proxy value for field \"%s\" (changing \"%v\" -> \"%v\")", prop, value, proxy)

			return proxy
		}

		return value
	})
}

func responseProxyHandler(proxied map[string]any) js.Value {
	handler := jsObject.New()
	handler.Set("get", proxyHandlerGet(proxied))
	return handler
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

	responseOptions := jsObject.New()
	responseOptions.Set("status", IR.inner.StatusCode)
	responseOptions.Set("statusText", http.StatusText(IR.inner.StatusCode))
	responseOptions.Set("headers", headers)

	proxied := make(map[string]any)

	if IR.responseType != "" {
		proxied["type"] = IR.responseType
	}

	// can't call the constructor properly if the value is outside the "legal" range (i.e. [200, 599])
	// (even though the fetch spec requires something different...)
	if IR.inner.StatusCode == 0 {
		responseOptions.Set("status", 418)
		proxied["status"] = IR.inner.StatusCode
	}

	if len(IR.urlList) > 0 {
		// "The value of the url property will be the final URL obtained after any redirects."
		// source: https://developer.mozilla.org/en-US/docs/Web/API/Response/url
		last := IR.urlList[len(IR.urlList)-1]
		proxied["url"] = last.String()
	}

	responseConstructor := js.Global().Get("Response")
	response := responseConstructor.New(jsBody, responseOptions)

	for k, v := range proxied {
		response.Set(fmt.Sprintf("_%s", k), v)
	}

	proxyConstructor := js.Global().Get("Proxy")
	proxy := proxyConstructor.New(response, responseProxyHandler(proxied))

	return proxy, nil
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
	[✅] type		    - has to be proxied
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
