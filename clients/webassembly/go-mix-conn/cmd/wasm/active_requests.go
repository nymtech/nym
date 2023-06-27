// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"context"
	"errors"
	"fmt"
	"net"
	"net/http"
	"net/url"
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

func checkRedirect(redirect Redirect, req *http.Request, via []*http.Request) error {
	Debug("attempting to perform redirection to %s with our policy set to '%s'", req.URL.String(), redirect)

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

	switch redirect {
	case REQUEST_REDIRECT_MANUAL:
		Error("unimplemented '%s' redirect", redirect)
		return http.ErrUseLastResponse
	case REQUEST_REDIRECT_ERROR:
		return errors.New("encountered redirect")
	case REQUEST_REDIRECT_FOLLOW:
		Debug("will perform redirection")
		return nil
	default:
		// if this was rust that had proper enums and match statements,
		// we could have guaranteed that at compile time...
		panic("unreachable")
	}
}

func checkMode(mode Mode, addr string) error {
	originUrl, originErr := url.Parse(origin)
	if originErr != nil {
		return errors.New(fmt.Sprintf("could not obtain origin: %s", originErr))
	}
	remoteUrl, remoteErr := url.Parse(addr)
	if remoteErr != nil {
		return remoteErr
	}

	switch mode {
	case MODE_CORS:
		Warn("unimplemented %s mode", MODE_CORS)
	case MODE_SAME_ORIGIN:
		// Roughly speaking, two URIs are part of the same origin (i.e., represent the same principal)
		// if they have the same scheme, host, and port.
		// Reference: https://www.rfc-editor.org/rfc/rfc6454.html#section-3.2
		if originUrl.Scheme != remoteUrl.Scheme || originUrl.Host != remoteUrl.Host || originUrl.Port() != remoteUrl.Port() {
			return errors.New(fmt.Sprintf("Access to mixFetch at '%s' from origin '%s' has been blocked by CORS policy.", addr, origin))
		}

	case MODE_NO_CORS:
		Warn("unimplemented %s mode", MODE_NO_CORS)

	// those should have been rejected at parsing time
	case MODE_NAVIGATE, MODE_WEBSOCKET:
		panic("impossible request mode")

	default:
		// if this was rust that had proper enums and match statements,
		// we could have guaranteed that at compile time...
		panic("unreachable")
	}

	return nil
}

func dialContext(_ctx context.Context, opts RequestOptions, _network, addr string) (net.Conn, error) {
	Info("dialing plain connection to %s", addr)

	if err := checkMode(opts.mode, addr); err != nil {
		return nil, err
	}

	requestId, err := rsStartNewMixnetRequest(addr)
	if err != nil {
		return nil, err
	}

	conn, inj := NewFakeConnection(requestId, addr)
	activeRequests.insert(requestId, inj)

	return conn, nil
}

func dialTLSContext(_ctx context.Context, opts RequestOptions, _network, addr string) (net.Conn, error) {
	Info("dialing TLS connection to %s", addr)

	if err := checkMode(opts.mode, addr); err != nil {
		return nil, err
	}

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

func buildHttpClient(opts RequestOptions) *http.Client {
	return &http.Client{
		CheckRedirect: func(req *http.Request, via []*http.Request) error {
			return checkRedirect(opts.redirect, req, via)
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

func performRequest(req *ParsedRequest) (*http.Response, error) {
	reqClient := buildHttpClient(req.options)

	Info("Starting the request...")
	Debug("%v: %v", req.options, *req.request)
	return reqClient.Do(req.request)
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
		return intoJSResponse(res)
	case err := <-errCh:
		Warn("request failure: %v", err)
		return nil, err
	case <-time.After(requestTimeout):
		// TODO: cancel stuff here.... somehow...
		Warn("request has timed out")
		return nil, errors.New("request timeout")
	}
}
