// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package conv

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"go-mix-conn/internal/external"
	"go-mix-conn/internal/helpers"
	"go-mix-conn/internal/jstypes"
	"go-mix-conn/internal/log"
	"go-mix-conn/internal/types"
	"io"
	"net/http"
	"net/url"
	"syscall/js"
)

const (
	fieldRequestMethod         = "method"
	fieldRequestUrl            = "url"
	fieldRequestBody           = "body"
	fieldRequestRedirect       = "redirect"
	fieldRequestMode           = "mode"
	fieldRequestCredentials    = "credentials"
	fieldRequestReferrer       = "referrer"
	fieldRequestReferrerPolicy = "referrerPolicy"
	fieldRequestHeaders        = "headers"
	fieldRequestCache          = "cache"
	fieldRequestSignal         = "signal"
)

type ParsedRequest struct {
	Request *http.Request
	Options *types.RequestOptions
}

// ParseJSRequest is a reverse of https://github.com/golang/go/blob/release-branch.go1.21/src/net/http/roundtrip_js.go#L91
// https://developer.mozilla.org/en-US/docs/Web/API/request
//
// Preflight requests: status unknown
/*
	Request attributes and their implementation status:
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
	[✅] mode
	[✅️] redirect
	[❌] referrer
	[❌] referrerPolicy
	[✅] signal
	[✅] url
*/
func ParseJSRequest(request js.Value, unsafeCors bool) (*ParsedRequest, error) {
	// https://github.com/mozilla/gecko-dev/blob/d307d4d9f06dab6d16e963a4318e5e8ff4899141/dom/fetch/Fetch.cpp#L501
	// https://github.com/mozilla/gecko-dev/blob/d307d4d9f06dab6d16e963a4318e5e8ff4899141/dom/fetch/Request.cpp#L270

	method, err := helpers.GetStringProperty(&request, fieldRequestMethod)
	if err != nil {
		return nil, err
	}

	requestUrl, err := helpers.GetStringProperty(&request, fieldRequestUrl)
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

	mode, err := parseMode(&request, unsafeCors)
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

	ctx, cancel := context.WithCancel(context.Background())
	if err := dealWithSignal(&request, cancel); err != nil {
		return nil, err
	}

	// A Request has an associated response tainting, which is "basic", "cors", or "opaque".
	// Unless stated otherwise, it is "basic".
	// Reference: https://fetch.spec.whatwg.org/#concept-request-response-tainting
	responseTainting := jstypes.ResponseTaintingBasic

	options := types.RequestOptions{
		Redirect:         redirect,
		Mode:             mode,
		CredentialsMode:  credentialsMode,
		Referrer:         referrer,
		ReferrerPolicy:   referrerPolicy,
		ResponseTainting: responseTainting,
		Method:           method,
	}

	jsHeaders := request.Get(fieldRequestHeaders)
	headers, err := parseHeaders(jsHeaders, options, method)
	if err != nil {
		return nil, err
	}

	checkUnsupportedAttributes(&request)

	req, err := http.NewRequestWithContext(ctx, method, requestUrl, body)
	if err != nil {
		return nil, err
	}
	req.Header = headers

	log.Debug("constructed Request: %+v", req)
	log.Debug("using Options: %s", options)

	return &ParsedRequest{
		Request: req,
		Options: &options,
	}, nil
}

func checkUnsupportedAttributes(request *js.Value) {
	cache := request.Get(fieldRequestCache)

	if !cache.IsUndefined() {
		log.Warn("\"%s\"' attribute is set on the Request - this is not supported by mixFetch", fieldRequestCache)
	}

	// TODO: implement more of them
}

func parseHeaders(headers js.Value, reqOpts types.RequestOptions, method string) (http.Header, error) {
	goHeaders := http.Header{}

	if headers.Type() != js.TypeObject {
		return nil, errors.New("the Request headers is not an object")
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
	origin := jstypes.Origin()
	serializedOrigin := origin
	// Reference: https://fetch.spec.whatwg.org/#origin-header
	// TODO: 3.1.2: check response tainting
	// 3.1.3
	if method != "GET" && method != "HEAD" {
		// 3.1.3.1
		if reqOpts.Mode != jstypes.ModeCors {
			switch reqOpts.ReferrerPolicy {
			case jstypes.ReferrerNoReferrer:
				serializedOrigin = nil
			case jstypes.ReferrerPolicyNoReferrerWhenDowngrade, jstypes.ReferrerPolicyStrictOrigin, jstypes.ReferrerPolicyStrictOriginWhenCrossOrigin:
				panic("unimplemented Referrer policy")
			case jstypes.ReferrerPolicySameOrigin:
				panic("unimplemented Referrer policy")
			}
		}
		// 3.1.3.2
		if serializedOrigin != nil {
			goHeaders.Set(jstypes.HeaderOrigin, *serializedOrigin)
		}
	}

	return goHeaders, nil
}

func parseBody(request *js.Value) (io.Reader, error) {
	jsBody := request.Get(fieldRequestBody)
	var bodyReader io.Reader

	if jsBody.InstanceOf(js.Global().Get("ReadableStream")) && jsBody.Get("getReader").Type() == js.TypeFunction {
		// Check to see if getReader is a function
		log.Debug("stream body - getReader")
		bodyReader = external.NewStreamReader(jsBody.Call("getReader"))
	} else {
		log.Debug("unstreamable body - fallback to ArrayBuffer")
		bodyReader = external.NewArrayReader(request.Call("arrayBuffer"))
	}
	bodyBytes, err := io.ReadAll(bodyReader)
	if err != nil {
		return nil, err
	}
	// Leaving historical notes as reference points
	// TODO: that seems super awkward. we're constructing a reader only to fully consume it
	// and create it all over again so that we the recipient wouldn't complain about content-length
	// surely there must be a better way?

	return bytes.NewReader(bodyBytes), nil
}

func parseRedirect(request *js.Value) (string, error) {
	redirect := request.Get(fieldRequestRedirect)
	if redirect.IsUndefined() || redirect.IsNull() {
		// "A Request has an associated Redirect Mode, which is "follow", "error", or "manual". Unless stated otherwise, it is "follow"."
		// Reference: https://fetch.spec.whatwg.org/#concept-request
		return jstypes.RequestRedirectFollow, nil
	}

	if redirect.Type() != js.TypeString {
		return "", errors.New("the redirect field is not a string")
	}

	redirectString := redirect.String()
	switch redirect.String() {
	case jstypes.RequestRedirectManual:
		return jstypes.RequestRedirectManual, nil
	case jstypes.RequestRedirectError:
		return jstypes.RequestRedirectError, nil
	case jstypes.RequestRedirectFollow:
		return jstypes.RequestRedirectFollow, nil
	}

	return "", errors.New(fmt.Sprintf("%s is not a valid Redirect", redirectString))
}

func parseMode(request *js.Value, unsafeCors bool) (jstypes.Mode, error) {
	if unsafeCors {
		return jstypes.ModeUnsafeIgnoreCors, nil
	}

	mode := request.Get(fieldRequestMode)
	if mode.IsUndefined() || mode.IsNull() {
		// "Even though the default Request Mode is "no-cors", standards are highly discouraged from using it for new features. It is rather unsafe."
		// Reference: https://fetch.spec.whatwg.org/#concept-request-mode
		return jstypes.ModeNoCors, nil
	}

	if mode.Type() != js.TypeString {
		return "", errors.New("the mode field is not a string")
	}

	modeString := mode.String()
	switch modeString {
	case jstypes.ModeCors:
		return jstypes.ModeCors, nil
	case jstypes.ModeSameOrigin:
		return jstypes.ModeSameOrigin, nil
	case jstypes.ModeNoCors:
		return jstypes.ModeNoCors, nil
	case jstypes.ModeUnsafeIgnoreCors:
		return jstypes.ModeUnsafeIgnoreCors, nil
	case jstypes.ModeNavigate:
		return "", errors.New(fmt.Sprintf("%s Mode is not supported", jstypes.ModeNavigate))
	case jstypes.ModeWebsocket:
		return "", errors.New(fmt.Sprintf("%s Mode is not supported", jstypes.ModeWebsocket))
	}

	return "", errors.New(fmt.Sprintf("%s is not a valid Mode", modeString))
}

func parseCredentialsMode(request *js.Value) (jstypes.CredentialsMode, error) {
	credentialsMode := request.Get(fieldRequestCredentials)
	if credentialsMode.IsUndefined() || credentialsMode.IsNull() {
		// A Request has an associated credentials Mode, which is "omit", "same-origin", or "include". Unless stated otherwise, it is "same-origin".
		// Reference: https://fetch.spec.whatwg.org/#concept-request-mode
		return jstypes.CredentialsModeSameOrigin, nil
	}

	if credentialsMode.Type() != js.TypeString {
		return "", errors.New("the credentials field is not a string")
	}

	credentialsModeString := credentialsMode.String()
	switch credentialsModeString {
	case jstypes.CredentialsModeOmit:
		return jstypes.CredentialsModeOmit, nil
	case jstypes.CredentialsModeInclude:
		return jstypes.CredentialsModeInclude, nil
	case jstypes.CredentialsModeSameOrigin:
		return jstypes.CredentialsModeSameOrigin, nil
	}

	return "", errors.New(fmt.Sprintf("%s is not a valid credentials Mode", credentialsModeString))
}

func parseReferrer(request *js.Value) (jstypes.Referrer, error) {
	referrer := request.Get(fieldRequestReferrer)
	if referrer.IsUndefined() || referrer.IsNull() {
		// A Request has an associated Referrer, which is "no-Referrer", "client", or a URL. Unless stated otherwise it is "client".
		// Reference: https://fetch.spec.whatwg.org/#concept-request-referrer
		return jstypes.ReferrerClient, nil
	}

	if referrer.Type() != js.TypeString {
		return "", errors.New("the referrer field is not a string")
	}

	referrerString := referrer.String()
	if referrerString == jstypes.ReferrerNoReferrer {
		return jstypes.ReferrerNoReferrer, nil
	}

	if referrerString == jstypes.ReferrerClient {
		return jstypes.ReferrerClient, nil
	}

	_, err := url.Parse(referrerString)
	if err != nil {
		return "", errors.New(fmt.Sprintf("\"%s\" is not a valid URL Referrer: \"%s\"", referrerString, err))
	}
	return referrerString, nil
}

func parseRefererPolicy(request *js.Value) (jstypes.ReferrerPolicy, error) {
	referrerPolicy := request.Get(fieldRequestReferrerPolicy)
	if referrerPolicy.IsUndefined() || referrerPolicy.IsNull() {
		// A Request has an associated Referrer policy, which is a Referrer policy. Unless stated otherwise it is the empty string
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
	case jstypes.ReferrerPolicyNoReferrer:
		return jstypes.ReferrerPolicyNoReferrer, nil
	case jstypes.ReferrerPolicyNoReferrerWhenDowngrade:
		return jstypes.ReferrerPolicyNoReferrerWhenDowngrade, nil
	case jstypes.ReferrerPolicyOrigin:
		return jstypes.ReferrerPolicyOrigin, nil
	case jstypes.ReferrerPolicyOriginWhenCrossOrigin:
		return jstypes.ReferrerPolicyOriginWhenCrossOrigin, nil
	case jstypes.ReferrerPolicySameOrigin:
		return jstypes.ReferrerPolicySameOrigin, nil
	case jstypes.ReferrerPolicyStrictOrigin:
		return jstypes.ReferrerPolicyStrictOrigin, nil
	case jstypes.ReferrerPolicyStrictOriginWhenCrossOrigin:
		return jstypes.ReferrerPolicyStrictOriginWhenCrossOrigin, nil
	case jstypes.ReferrerPolicyUnsafeUrl:
		return jstypes.ReferrerPolicyUnsafeUrl, nil
	}

	return "", errors.New(fmt.Sprintf("%s is not a valid Referrer policy", referrerPolicyString))

}

func dealWithSignal(request *js.Value, cancel context.CancelFunc) error {
	abortSignal := request.Get(fieldRequestSignal)
	if abortSignal.IsUndefined() || abortSignal.IsNull() {
		log.Debug("no abort signal passed")
		return nil
	}
	if abortSignal.Type() != js.TypeObject {
		return errors.New("passed abort signal is not a valid object")
	}

	abortSignal.Call("addEventListener", "abort", js.FuncOf(func(this js.Value, args []js.Value) any {
		log.Debug("abort signal called")
		cancel()
		return nil
	}))

	return nil
}
