// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"bytes"
	"errors"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"syscall/js"
)

type Redirect = string
type Mode = string
type CredentialsMode = string
type ResponseTainting = string
type ReferrerPolicy = string
type Referrer = string

const (
	REQUEST_REDIRECT_ERROR  = "error"
	REQUEST_REDIRECT_MANUAL = "manual"
	REQUEST_REDIRECT_FOLLOW = "follow"

	RESPONSE_TAINTING_BASIC  = "basic"
	RESPONSE_TAINTING_CORS   = "cors"
	RESPONSE_TAINTING_OPAQUE = "opaque"

	MODE_CORS        = "cors"
	MODE_SAME_ORIGIN = "same-origin"
	MODE_NO_CORS     = "no-cors"
	MODE_NAVIGATE    = "navigate"
	MODE_WEBSOCKET   = "websocket"

	CREDENTIALS_MODE_OMIT        = "omit"
	CREDENTIALS_MODE_SAME_ORIGIN = "same-origin"
	CREDENTIALS_MODE_INCLUDE     = "include"

	REFERRER_POLICY_NO_REFERRER                     = "no-referrer"
	REFERRER_POLICY_NO_REFERRER_WHEN_DOWNGRADE      = "no-referrer-when-downgrade"
	REFERRER_POLICY_ORIGIN                          = "origin"
	REFERRER_POLICY_ORIGIN_WHEN_CROSS_ORIGIN        = "origin-when-cross-origin"
	REFERRER_POLICY_SAME_ORIGIN                     = "same-origin"
	REFERRER_POLICY_STRICT_ORIGIN                   = "strict-origin"
	REFERRER_POLICY_STRICT_ORIGIN_WHEN_CROSS_ORIGIN = "strict-origin-when-cross-origin"
	REFERRER_POLICY_UNSAFE_URL                      = "unsafe-url"

	REFERRER_NO_REFERRER = "no-referrer"
	REFERRER_CLIENT      = "client"
)

type ParsedRequest struct {
	request *http.Request
	options *RequestOptions
}

type RequestOptions struct {
	redirect         Redirect
	mode             Mode
	credentialsMode  CredentialsMode
	referrerPolicy   ReferrerPolicy
	referrer         Referrer
	responseTainting ResponseTainting
	method           string
}

func (opts RequestOptions) String() string {
	return fmt.Sprintf(
		"{ redirect: %s, mode: %s, credentials: %s, referrerPolicy: %s, referrer: %s, responseTainting: %s, method: %s }",
		opts.redirect,
		opts.mode,
		opts.credentialsMode,
		opts.referrerPolicy,
		opts.referrer,
		opts.responseTainting,
		opts.method,
	)
}

// ParseJSRequest is a reverse of https://github.com/golang/go/blob/release-branch.go1.21/src/net/http/roundtrip_js.go#L91
// https://developer.mozilla.org/en-US/docs/Web/API/request
//
// Preflight requests: status unknown
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
	[⚠️] mode			- only "same-origin" is naively (and not fully) implemented
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

	body, err := parseBody(&request)
	if err != nil {
		return nil, err
	}
	redirect, err := parseRedirect(&request)
	if err != nil {
		return nil, err
	}

	mode, err := parseMode(&request)
	if err != nil {
		return nil, err
	}

	credentialsMode, err := parseCredentialsMode(&request)
	if err != nil {
		return nil, err
	}

	referrer, err := parseReferrer(&request)
	if err != nil {
		return nil, err
	}

	referrerPolicy, err := parseRefererPolicy(&request)
	if err != nil {
		return nil, err
	}

	// A request has an associated response tainting, which is "basic", "cors", or "opaque".
	// Unless stated otherwise, it is "basic".
	// Reference: https://fetch.spec.whatwg.org/#concept-request-response-tainting
	responseTainting := RESPONSE_TAINTING_BASIC

	options := RequestOptions{
		redirect:         redirect,
		mode:             mode,
		credentialsMode:  credentialsMode,
		referrer:         referrer,
		referrerPolicy:   referrerPolicy,
		responseTainting: responseTainting,
		method:           method,
	}

	jsHeaders := request.Get("headers")
	headers, err := parseHeaders(jsHeaders, options, method)
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
	Debug("using options: %s", options)

	return &ParsedRequest{
		request: req,
		options: &options,
	}, nil
}

func checkUnsupportedAttributes(request *js.Value) {
	cache := request.Get("cache")

	if !cache.IsUndefined() {
		Warn("'cache' attribute is set on the request - this is not supported by mixFetch")
	}

	// TODO: implement more of them
}

func parseHeaders(headers js.Value, reqOpts RequestOptions, method string) (http.Header, error) {
	goHeaders := http.Header{}

	if headers.Type() != js.TypeObject {
		return nil, errors.New("the request headers is not an object")
	}
	headersIter := headers.Call("entries")

	for {
		next := headersIter.Call("next")
		done := next.Get("done").Bool()
		if done {
			break
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

	// add additional headers

	// 3.1.1
	serializedOrigin := &origin
	// Reference: https://fetch.spec.whatwg.org/#origin-header
	// TODO: 3.1.2: check response tainting
	// 3.1.3
	if method != "GET" && method != "HEAD" {
		// 3.1.3.1
		if reqOpts.mode != MODE_CORS {
			switch reqOpts.referrerPolicy {
			case REFERRER_NO_REFERRER:
				serializedOrigin = nil
			case REFERRER_POLICY_NO_REFERRER_WHEN_DOWNGRADE, REFERRER_POLICY_STRICT_ORIGIN, REFERRER_POLICY_STRICT_ORIGIN_WHEN_CROSS_ORIGIN:
				panic("unimplemented referrer policy")
			case REFERRER_POLICY_SAME_ORIGIN:
				panic("unimplemented referrer policy")
			}
		}
		// 3.1.3.2
		if serializedOrigin != nil {
			goHeaders.Set(headerOrigin, *serializedOrigin)
		}
	}

	return goHeaders, nil
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
		// "A request has an associated redirect mode, which is "follow", "error", or "manual". Unless stated otherwise, it is "follow"."
		// Reference: https://fetch.spec.whatwg.org/#concept-request
		return REQUEST_REDIRECT_FOLLOW, nil
	}

	if redirect.Type() != js.TypeString {
		return "", errors.New("the redirect field is not a string")
	}

	redirectString := redirect.String()
	switch redirect.String() {
	case REQUEST_REDIRECT_MANUAL:
		return REQUEST_REDIRECT_MANUAL, nil
	case REQUEST_REDIRECT_ERROR:
		return REQUEST_REDIRECT_ERROR, nil
	case REQUEST_REDIRECT_FOLLOW:
		return REQUEST_REDIRECT_FOLLOW, nil
	}

	return "", errors.New(fmt.Sprintf("%s is not a valid redirect", redirectString))
}

func parseMode(request *js.Value) (Mode, error) {
	mode := request.Get("mode")
	if mode.IsUndefined() || mode.IsNull() {
		// "Even though the default request mode is "no-cors", standards are highly discouraged from using it for new features. It is rather unsafe."
		// Reference: https://fetch.spec.whatwg.org/#concept-request-mode
		return MODE_NO_CORS, nil
	}

	if mode.Type() != js.TypeString {
		return "", errors.New("the mode field is not a string")
	}

	modeString := mode.String()
	switch modeString {
	case MODE_CORS:
		return MODE_CORS, nil
	case MODE_SAME_ORIGIN:
		return MODE_SAME_ORIGIN, nil
	case MODE_NO_CORS:
		return MODE_NO_CORS, nil
	case MODE_NAVIGATE:
		return "", errors.New(fmt.Sprintf("%s mode is not supported", MODE_NAVIGATE))
	case MODE_WEBSOCKET:
		return "", errors.New(fmt.Sprintf("%s mode is not supported", MODE_WEBSOCKET))
	}

	return "", errors.New(fmt.Sprintf("%s is not a valid mode", modeString))
}

func parseCredentialsMode(request *js.Value) (CredentialsMode, error) {
	credentialsMode := request.Get("credentials")
	if credentialsMode.IsUndefined() || credentialsMode.IsNull() {
		// A request has an associated credentials mode, which is "omit", "same-origin", or "include". Unless stated otherwise, it is "same-origin".
		// Reference: https://fetch.spec.whatwg.org/#concept-request-mode
		return CREDENTIALS_MODE_SAME_ORIGIN, nil
	}

	if credentialsMode.Type() != js.TypeString {
		return "", errors.New("the credentials field is not a string")
	}

	credentialsModeString := credentialsMode.String()
	switch credentialsModeString {
	case CREDENTIALS_MODE_OMIT:
		return CREDENTIALS_MODE_OMIT, nil
	case CREDENTIALS_MODE_INCLUDE:
		return CREDENTIALS_MODE_INCLUDE, nil
	case CREDENTIALS_MODE_SAME_ORIGIN:
		return CREDENTIALS_MODE_SAME_ORIGIN, nil
	}

	return "", errors.New(fmt.Sprintf("%s is not a valid credentials mode", credentialsModeString))
}

func parseReferrer(request *js.Value) (Referrer, error) {
	referrer := request.Get("referrer")
	if referrer.IsUndefined() || referrer.IsNull() {
		// A request has an associated referrer, which is "no-referrer", "client", or a URL. Unless stated otherwise it is "client".
		// Reference: https://fetch.spec.whatwg.org/#concept-request-referrer
		return REFERRER_CLIENT, nil
	}

	if referrer.Type() != js.TypeString {
		return "", errors.New("the referrer field is not a string")
	}

	referrerString := referrer.String()
	if referrerString == REFERRER_NO_REFERRER {
		return REFERRER_NO_REFERRER, nil
	}

	if referrerString == REFERRER_CLIENT {
		return REFERRER_CLIENT, nil
	}

	_, err := url.Parse(referrerString)
	if err != nil {
		return "", errors.New(fmt.Sprintf("\"%s\" is not a valid URL referrer: \"%s\"", referrerString, err))
	}
	return referrerString, nil
}

func parseRefererPolicy(request *js.Value) (ReferrerPolicy, error) {
	referrerPolicy := request.Get("referrerPolicy")
	if referrerPolicy.IsUndefined() || referrerPolicy.IsNull() {
		// A request has an associated referrer policy, which is a referrer policy. Unless stated otherwise it is the empty string
		// Reference: https://fetch.spec.whatwg.org/#concept-request-referrer-policy
		return "", nil
	}

	if referrerPolicy.Type() != js.TypeString {
		return "", errors.New("the referrerPolicy field is not a string")
	}

	referrerPolicyString := referrerPolicy.String()
	switch referrerPolicyString {
	case "":
		return "", nil
	case REFERRER_POLICY_NO_REFERRER:
		return REFERRER_POLICY_NO_REFERRER, nil
	case REFERRER_POLICY_NO_REFERRER_WHEN_DOWNGRADE:
		return REFERRER_POLICY_NO_REFERRER_WHEN_DOWNGRADE, nil
	case REFERRER_POLICY_ORIGIN:
		return REFERRER_POLICY_ORIGIN, nil
	case REFERRER_POLICY_ORIGIN_WHEN_CROSS_ORIGIN:
		return REFERRER_POLICY_ORIGIN_WHEN_CROSS_ORIGIN, nil
	case REFERRER_POLICY_SAME_ORIGIN:
		return REFERRER_POLICY_SAME_ORIGIN, nil
	case REFERRER_POLICY_STRICT_ORIGIN:
		return REFERRER_POLICY_STRICT_ORIGIN, nil
	case REFERRER_POLICY_STRICT_ORIGIN_WHEN_CROSS_ORIGIN:
		return REFERRER_POLICY_STRICT_ORIGIN_WHEN_CROSS_ORIGIN, nil
	case REFERRER_POLICY_UNSAFE_URL:
		return REFERRER_POLICY_UNSAFE_URL, nil
	}

	return "", errors.New(fmt.Sprintf("%s is not a valid referrer policy", referrerPolicyString))

}

// Reference: https://fetch.spec.whatwg.org/#cors-safelisted-method
func isCorsSafelistedMethod(method string) bool {
	if method == "GET" || method == "HEAD" || method == "POST" {
		return true
	} else {
		return false
	}
}
