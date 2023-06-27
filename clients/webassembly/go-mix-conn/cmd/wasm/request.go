// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"bytes"
	"errors"
	"fmt"
	"io"
	"net/http"
	"syscall/js"
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

	checkUnsupportedAttributes(&request)

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

func checkUnsupportedAttributes(request *js.Value) {
	cache := request.Get("cache")

	if !cache.IsUndefined() {
		Warn("'cache' attribute is set on the request - this is not supported by mixFetch")
	}

	// TODO: implement more of them
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
