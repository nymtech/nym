// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"context"
	"errors"
	"fmt"
	"net"
	"net/http"
	"sync"
	"time"
)

type RequestId = uint64

type ActiveRequests struct {
	sync.Mutex
	inner map[RequestId]*ActiveRequest
}

func (ar *ActiveRequests) exists(id RequestId) bool {
	Debug("checking if request %d exists", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	return exists
}

func (ar *ActiveRequests) insert(id RequestId, inj ConnectionInjector) {
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	if exists {
		panic("attempted to overwrite active connection")
	}
	ar.inner[id] = &ActiveRequest{injector: inj}
}

func (ar *ActiveRequests) remove(id RequestId) {
	Debug("removing request %d", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	if !exists {
		panic("attempted to remove active connection that doesn't exist")
	}
	delete(ar.inner, id)
}

func (ar *ActiveRequests) injectData(id RequestId, data []byte) {
	Debug("injecting data for %d", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	if !exists {
		panic("attempted to write to connection that doesn't exist")
	}
	ar.inner[id].injector.serverData <- data
}

func (ar *ActiveRequests) closeRemoteSocket(id RequestId) {
	Debug("closing remote socket for %d", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	if !exists {
		Warn("attempted to close remote socket of a connection that doesn't exist")
		return
	}
	ar.inner[id].injector.remoteClosed <- true
}

func (ar *ActiveRequests) sendError(id RequestId, err error) {
	Debug("injecting error for %d: %s", id, err)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	if !exists {
		panic("attempted to inject error data to connection that doesn't exist")
	}
	ar.inner[id].injector.remoteError <- err
}

type ActiveRequest struct {
	injector ConnectionInjector
}

func inRedirectionLoop(req *http.Request, via []*http.Request) bool {
	target := req.URL.String()

	for i := 0; i < len(via); i++ {
		if target == via[i].URL.String() {
			return true
		}
	}
	return false
}

func checkRedirect(opts *RequestOptions, req *http.Request, via []*http.Request) error {
	Debug("attempting to perform redirection to %s with our policy set to '%s'", req.URL.String(), opts.redirect)

	if len(via) > maxRedirections {
		return errors.New(fmt.Sprintf("Maximum (%d) redirects followed", maxRedirections))
	}

	if inRedirectionLoop(req, via) {
		return errors.New("stuck in redirection loop")
	}

	redirectionChain := ""
	for i := 0; i < len(via); i++ {
		redirectionChain += fmt.Sprintf("%s -> ", via[i].URL.String())
	}
	redirectionChain += fmt.Sprintf("[%s]", req.URL.String())
	Debug("redirection chain: %s", redirectionChain)

	// Reference: https://fetch.spec.whatwg.org/#http-fetch
	// 4.3.6.2
	switch opts.redirect {
	case REQUEST_REDIRECT_ERROR:
		return errors.New("encountered redirect")
	case REQUEST_REDIRECT_MANUAL:
		if opts.mode == MODE_NAVIGATE {
			return errors.New("unimplemented 'navigate' + 'manual' redirection")
		}
		Error("unimplemented '%s' redirect", opts.redirect)
		// TODO: somehow set response to opaque-redirect filtered response
		return http.ErrUseLastResponse
	case REQUEST_REDIRECT_FOLLOW:
		Debug("will perform redirection")
		return nil
	default:
		// if this was rust that had proper enums and match statements,
		// we could have guaranteed that at compile time...
		panic("unreachable")
	}
}

// 4.1.12
// Reference: https://fetch.spec.whatwg.org/#main-fetch
func mainFetchChecks(req *ParsedRequest) error {
	// no preloading

	if (isSameOrigin(req.request.URL) && req.options.responseTainting == RESPONSE_TAINTING_BASIC) ||
		req.request.URL.Scheme == "data" || (req.options.mode == MODE_NAVIGATE || req.options.mode == MODE_WEBSOCKET) {
		Debug("setting response tainting to basic")
		req.options.responseTainting = RESPONSE_TAINTING_BASIC
		// TODO: scheme fetch here
		return nil
	}
	if req.options.mode == MODE_SAME_ORIGIN {
		return errors.New(fmt.Sprintf("MixFetch API cannot load %s. Request mode is \"%s\" but the URL's origin is not same as the request origin %s.", req.request.URL.String(), MODE_SAME_ORIGIN, origin))
	}
	if req.options.mode == MODE_NO_CORS {
		if req.options.redirect != REQUEST_REDIRECT_FOLLOW {
			return errors.New(fmt.Sprintf("MixFetch API could not perform request with mode \"%s\" and redirect \"%s\"", req.options.mode, req.options.redirect))
		}
		Debug("setting response tainting to opaque")
		req.options.responseTainting = RESPONSE_TAINTING_OPAQUE
		// TODO: scheme fetch here
		return nil
	}
	if req.request.URL.Scheme != "http" && req.request.URL.Scheme != "https" {
		return errors.New(fmt.Sprintf("The requested url scheme (\"%s\" is not http(s)", req.request.URL.Scheme))
	}

	// TODO: CORS-preflight flag
	// TODO: unsafe-request flag
	// (by default they're unset)
	corsPreflightFlag := false
	unsafeRequestFlag := false

	if corsPreflightFlag || (unsafeRequestFlag && (isCorsSafelistedMethod(req.request.Method) || len(corsUnsafeRequestHeaderNames(req.request.Header)) > 0)) {
		req.options.responseTainting = RESPONSE_TAINTING_CORS
		panic("unimplemented \"corsWithPreflightResponse\"")
	}

	req.options.responseTainting = RESPONSE_TAINTING_CORS
	Debug("setting response tainting to cors")
	// TODO: HTTP fetch here
	return nil
}

func schemeFetch(req *ParsedRequest) error {
	switch req.request.URL.Scheme {
	case "about":
		return errors.New("unsupported 'about' scheme")
	case "blob":
		return errors.New("unsupported 'blob' scheme")
	case "data":
		return errors.New("unsupported 'data' scheme")
	case "file":
		return errors.New("unsupported 'file' scheme")
	case "http", "https":
		// TODO: HTTP fetch here
		return nil
	default:
		return errors.New("unknown url scheme")
	}
}

func dialContext(_ctx context.Context, opts *RequestOptions, _network, addr string) (net.Conn, error) {
	Info("dialing plain connection to %s", addr)

	//if err := checkMode(opts.mode, addr); err != nil {
	//	return nil, err
	//}

	requestId, err := rsStartNewMixnetRequest(addr)
	if err != nil {
		return nil, err
	}

	conn, inj := NewFakeConnection(requestId, addr)
	activeRequests.insert(requestId, inj)

	return conn, nil
}

func dialTLSContext(_ctx context.Context, opts *RequestOptions, _network, addr string) (net.Conn, error) {
	Info("dialing TLS connection to %s", addr)

	//if err := checkMode(opts.mode, addr); err != nil {
	//	return nil, err
	//}

	requestId, err := rsStartNewMixnetRequest(addr)
	if err != nil {
		return nil, err
	}

	conn, inj := NewFakeTlsConn(requestId, addr)
	activeRequests.insert(requestId, inj)

	if err := conn.Handshake(); err != nil {
		return nil, err
	}

	return conn, nil
}

func buildHttpClient(opts *RequestOptions) *http.Client {
	return &http.Client{
		CheckRedirect: func(req *http.Request, via []*http.Request) error {
			return checkRedirect(opts, req, via)
		},

		Transport: &http.Transport{
			DialContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
				return dialContext(ctx, opts, network, addr)
			},
			DialTLSContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
				return dialTLSContext(ctx, opts, network, addr)
			},

			//TLSClientConfig: &tlsConfig,
			DisableKeepAlives:   true,
			MaxIdleConns:        1,
			MaxIdleConnsPerHost: 1,
			MaxConnsPerHost:     1,
		},
	}
}

func _closeRemoteSocket(requestId RequestId) any {
	activeRequests.closeRemoteSocket(requestId)
	return nil
}

func _injectServerData(requestId RequestId, data []byte) any {
	activeRequests.injectData(requestId, data)
	return nil
}

func _injectConnError(requestId RequestId, err error) any {
	activeRequests.sendError(requestId, err)
	return nil
}

func _changeRequestTimeout(timeout time.Duration) any {
	Debug("changing request timeout to %v", timeout)
	requestTimeout = timeout
	return nil
}

// Reference: https://fetch.spec.whatwg.org/#cors-check
func doCorsCheck(reqOpts *RequestOptions, resp *http.Response) error {
	// 4.9.1
	originHeader := resp.Header.Get(headerAllowOrigin)
	// 4.9.2
	if originHeader == "" {
		return errors.New(fmt.Sprintf("\"%s\" header not present on remote", headerAllowOrigin))
	}

	if reqOpts.credentialsMode != CREDENTIALS_MODE_INCLUDE && originHeader == wildcard {
		// 4.9.3
		return nil
	}

	// 4.9.4
	// TODO: presumably this needs to better account for the wildcard?
	if origin != originHeader {
		return errors.New(fmt.Sprintf("\"%s\" does not match the origin \"%s\" on \"%s\" remote header", origin, originHeader, headerAllowOrigin))
	}

	// 4.9.5
	if reqOpts.credentialsMode != CREDENTIALS_MODE_INCLUDE {
		return nil
	}

	// 4.9.6
	credentials := resp.Header.Get(headerAllowCredentials)
	// 4.9.7
	if credentials == "true" {
		return nil
	}

	// 4.9.8
	return errors.New("failed cors check")
}

func performRequest(req *ParsedRequest) (*http.Response, error) {
	err := mainFetchChecks(req)
	if err != nil {
		return nil, err
	}

	reqClient := buildHttpClient(req.options)

	if req.options.referrerPolicy == "" {
		// 4.1.8
		// Reference: https://fetch.spec.whatwg.org/#main-fetch
		// TODO: implement
		Warn("unimplemented: could not obtain referrer policy from the policy container")
	}

	if req.options.referrer != REFERRER_NO_REFERRER {
		// 4.1.9
		// Reference: https://fetch.spec.whatwg.org/#main-fetch
		// TODO: implement
		Warn("unimplemented: could not determine request's referrer")
	}

	// TODO: this is such a nasty assumption, but assume we're doing a 4.3 HTTP fetch here

	Info("Starting the request...")
	Debug("%v: %v", req.options, *req.request)
	// TODO: CORS preflight...

	resp, err := reqClient.Do(req.request)
	if err != nil {
		return nil, err
	}

	// 4.3.4.4
	if req.options.responseTainting == RESPONSE_TAINTING_CORS {
		err = doCorsCheck(req.options, resp)
		if err != nil {
			return nil, err
		}
	}
	// TODO: policy checks, etc...

	return resp, err
}

func _mixFetch(request *ParsedRequest) (any, error) {
	Info("_mixFetch: start")

	resCh := make(chan *http.Response)
	errCh := make(chan error)
	go func(resCh chan *http.Response, errCh chan error) {
		resp, err := performRequest(request)
		if err != nil {
			errCh <- err
		} else {
			resCh <- resp
		}
	}(resCh, errCh)

	select {
	case res := <-resCh:
		Info("finished performing the request")
		Debug("response: %v", *res)
		return intoJSResponse(res, request.options)
	case err := <-errCh:
		Warn("request failure: %v", err)
		return nil, err
	case <-time.After(requestTimeout):
		// TODO: cancel stuff here.... somehow...
		Warn("request has timed out")
		return nil, errors.New("request timeout")
	}
}
