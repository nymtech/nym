// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package mixfetch

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"errors"
	"fmt"
	"go-mix-conn/internal/bridge/rust_bridge"
	"go-mix-conn/internal/jstypes"
	"go-mix-conn/internal/jstypes/conv"
	"go-mix-conn/internal/log"
	"go-mix-conn/internal/state"
	"go-mix-conn/internal/types"
	"net"
	"net/http"
	"time"
)

func inRedirectionLoop(req *http.Request, via []*http.Request) bool {
	target := req.URL.String()

	for i := 0; i < len(via); i++ {
		if target == via[i].URL.String() {
			return true
		}
	}
	return false
}

func checkRedirect(ctx *types.RequestContext, opts *types.RequestOptions, req *http.Request, via []*http.Request) error {
	log.Debug("attempting to perform redirection to %s with our policy set to '%s'", req.URL.String(), opts.Redirect)

	if len(via) > state.MaxRedirections {
		return errors.New(fmt.Sprintf("Maximum (%d) redirects followed", state.MaxRedirections))
	}

	if inRedirectionLoop(req, via) {
		return errors.New("stuck in redirection loop")
	}

	redirectionChain := ""
	for i := 0; i < len(via); i++ {
		redirectionChain += fmt.Sprintf("%s -> ", via[i].URL.String())
	}
	redirectionChain += fmt.Sprintf("[%s]", req.URL.String())
	log.Debug("redirection chain: %s", redirectionChain)

	// Reference: https://fetch.spec.whatwg.org/#http-fetch
	// 4.3.6.2
	switch opts.Redirect {
	case jstypes.RequestRedirectError:
		return errors.New("encountered redirect")
	case jstypes.RequestRedirectManual:
		if opts.Mode == jstypes.ModeNavigate {
			return errors.New("unimplemented 'navigate' + 'manual' redirection")
		}
		ctx.OverwrittenResponseType = jstypes.ResponseTypeOpaqueRedirect
		return http.ErrUseLastResponse
	case jstypes.RequestRedirectFollow:
		log.Debug("will perform redirection")
		// this feels so nasty...
		ctx.WasRedirected = true
		return nil
	default:
		// if this was rust that had proper enums and match statements,
		// we could have guaranteed that at compile time...
		panic("unreachable")
	}
}

// 4.1.12
// Reference: https://fetch.spec.whatwg.org/#main-fetch
func mainFetchChecks(req *conv.ParsedRequest) error {
	// >>> START: NOT INCLUDED IN FETCH SPEC
	if req.Options.Mode == jstypes.ModeUnsafeIgnoreCors {
		// ignore all checks - everything should be accepted
		req.Options.ResponseTainting = jstypes.ResponseTaintingUnsafeIgnoreCors
		return nil
	}
	// >>> END: NOT INCLUDED IN FETCH SPEC

	// no preloading

	if (jstypes.IsSameOrigin(req.Request.URL) && req.Options.ResponseTainting == jstypes.ResponseTaintingBasic) ||
		req.Request.URL.Scheme == "data" || (req.Options.Mode == jstypes.ModeNavigate || req.Options.Mode == jstypes.ModeWebsocket) {
		log.Debug("setting response tainting to basic")
		req.Options.ResponseTainting = jstypes.ResponseTaintingBasic
		// TODO: scheme fetch here
		return nil
	}
	if req.Options.Mode == jstypes.ModeSameOrigin {
		return errors.New(fmt.Sprintf("MixFetch API cannot load %s. Request mode is \"%s\" but the URL's origin is not same as the request origin %v.", req.Request.URL.String(), jstypes.ModeSameOrigin, jstypes.Origin()))
	}
	if req.Options.Mode == jstypes.ModeNoCors {
		if req.Options.Redirect != jstypes.RequestRedirectFollow {
			return errors.New(fmt.Sprintf("MixFetch API cannot load %s. Request mode is \"%s\", but the redirect mode is not \"%s\".", req.Request.URL.String(), req.Options.Mode, jstypes.RequestRedirectFollow))
		}
		log.Debug("setting response tainting to opaque")
		req.Options.ResponseTainting = jstypes.ResponseTaintingOpaque
		// TODO: scheme fetch here
		return nil
	}
	if req.Request.URL.Scheme != "http" && req.Request.URL.Scheme != "https" {
		return errors.New(fmt.Sprintf("The requested url scheme (\"%s\" is not http(s)", req.Request.URL.Scheme))
	}

	// TODO: CORS-preflight flag
	// TODO: unsafe-request flag
	// (by default they're unset)
	corsPreflightFlag := false
	unsafeRequestFlag := false

	if corsPreflightFlag || (unsafeRequestFlag && (jstypes.IsCorsSafelistedMethod(req.Request.Method) || len(jstypes.CorsUnsafeRequestHeaderNames(req.Request.Header)) > 0)) {
		req.Options.ResponseTainting = jstypes.ResponseTaintingCors
		panic("unimplemented \"corsWithPreflightResponse\"")
	}

	req.Options.ResponseTainting = jstypes.ResponseTaintingCors
	log.Debug("setting response tainting to cors")
	// TODO: HTTP fetch here
	return nil
}

func schemeFetch(req *conv.ParsedRequest) error {
	switch req.Request.URL.Scheme {
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

// dialContext is called by Go's HTTP transport with addr derived from the real
// request URL. mappingKey is only used for tracking in ActiveRequests — it is
// never sent over the wire.
func dialContext(_ctx context.Context, mappingKey string, _network, addr string) (net.Conn, error) {
	log.Debug("dialing plain connection to %s", addr)

	requestId, err := rust_bridge.RsStartNewMixnetRequest(addr)
	if err != nil {
		return nil, err
	}
	if state.ActiveRequests.Exists(requestId) {
		return nil, errors.New(fmt.Sprintf("somehow opened duplicate connection with id %d", requestId))
	}

	conn, inj := state.NewFakeConnection(requestId, addr)
	state.ActiveRequests.Insert(requestId, mappingKey, inj)

	return conn, nil
}

// dialTLSContext is called by Go's HTTP transport with addr derived from the
// real request URL. mappingKey is only used for tracking in ActiveRequests —
// it is never sent over the wire.
func dialTLSContext(_ctx context.Context, mappingKey string, _network, addr string) (net.Conn, error) {
	log.Debug("dialing TLS connection to %s", addr)

	requestId, err := rust_bridge.RsStartNewMixnetRequest(addr)
	if err != nil {
		return nil, err
	}
	if state.ActiveRequests.Exists(requestId) {
		return nil, errors.New(fmt.Sprintf("somehow opened duplicate connection with id %d", requestId))
	}

	conn, inj := state.NewFakeTlsConn(requestId, addr)
	state.ActiveRequests.Insert(requestId, mappingKey, inj)

	if err := conn.Handshake(); err != nil {
		return nil, err
	}

	return conn, nil
}

// buildHttpClient creates an HTTP client with custom dial functions that route
// connections through the mixnet. mappingKey is captured by the dial closures
// for request tracking only — the actual destination comes from addr, which
// Go's HTTP transport derives from the real request URL.
func buildHttpClient(reqCtx *types.RequestContext, opts *types.RequestOptions, mappingKey string) *http.Client {
	return &http.Client{
		CheckRedirect: func(req *http.Request, via []*http.Request) error {
			return checkRedirect(reqCtx, opts, req, via)
		},

		Transport: &http.Transport{
			DialContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
				return dialContext(ctx, mappingKey, network, addr)
			},
			DialTLSContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
				return dialTLSContext(ctx, mappingKey, network, addr)
			},

			//TLSClientConfig: &tlsConfig,
			DisableKeepAlives: true,
			// Allow multiple concurrent connections to the same host.
			// Previously set to 1.
			MaxIdleConns:        10,
			MaxIdleConnsPerHost: 10,
			MaxConnsPerHost:     10,
		},
		Timeout: state.RequestTimeout,
	}
}

func CloseRemoteSocket(requestId types.RequestId) any {
	state.ActiveRequests.CloseRemoteSocket(requestId)
	return nil
}

func InjectServerData(requestId types.RequestId, data []byte) any {
	state.ActiveRequests.InjectData(requestId, data)
	return nil
}

func InjectConnError(requestId types.RequestId, err error) any {
	state.ActiveRequests.SendError(requestId, err)
	return nil
}

func ChangeRequestTimeout(timeout time.Duration) any {
	log.Debug("changing request timeout to %v", timeout)
	state.RequestTimeout = timeout
	return nil
}

// Reference: https://fetch.spec.whatwg.org/#cors-check
func doCorsCheck(reqOpts *types.RequestOptions, resp *http.Response) error {
	// 4.9.1
	originHeader := resp.Header.Get(jstypes.HeaderAllowOrigin)
	// 4.9.2
	if originHeader == "" {
		return errors.New(fmt.Sprintf("\"%s\" header not present on remote", jstypes.HeaderAllowOrigin))
	}

	if reqOpts.CredentialsMode != jstypes.CredentialsModeInclude && originHeader == jstypes.Wildcard {
		// 4.9.3
		return nil
	}

	// 4.9.4
	// TODO: presumably this needs to better account for the wildcard?

	// if origin is null it means 4.9.2 would have failed anyway
	origin := jstypes.Origin()
	if origin == nil {
		// TODO: won't this essentially fail all node requests?
		return errors.New("the local origin is null")
	}

	// safety: it's fine to dereference the pointer here as we've just checked if it's null
	if *origin != originHeader {
		return errors.New(fmt.Sprintf("\"%v\" does not match the origin \"%s\" on \"%s\" remote header", jstypes.Origin(), originHeader, jstypes.HeaderAllowOrigin))
	}

	// 4.9.5
	if reqOpts.CredentialsMode != jstypes.CredentialsModeInclude {
		return nil
	}

	// 4.9.6
	credentials := resp.Header.Get(jstypes.HeaderAllowCredentials)
	// 4.9.7
	if credentials == "true" {
		return nil
	}

	// 4.9.8
	return errors.New("failed cors check")
}

// performRequest executes the HTTP request. mappingKey is threaded through to
// buildHttpClient for request tracking only. The actual HTTP request is made
// with req.Request (the original, unmodified URL) at reqClient.Do(req.Request).
func performRequest(req *conv.ParsedRequest, mappingKey string) (*conv.ResponseWrapper, error) {
	err := mainFetchChecks(req)
	if err != nil {
		return nil, err
	}

	reqCtx := &types.RequestContext{}

	// mappingKey is only captured by the dial closures for ActiveRequests tracking.
	reqClient := buildHttpClient(reqCtx, req.Options, mappingKey)

	if req.Options.ReferrerPolicy == "" {
		// 4.1.8
		// Reference: https://fetch.spec.whatwg.org/#main-fetch
		// TODO: implement
		log.Warn("unimplemented: could not obtain referrer policy from the policy container")
	}

	if req.Options.Referrer != jstypes.ReferrerNoReferrer {
		// 4.1.9
		// Reference: https://fetch.spec.whatwg.org/#main-fetch
		// TODO: implement
		log.Warn("unimplemented: could not determine request's referrer")
	}

	// TODO: this is such a nasty assumption, but assume we're doing a 4.3 HTTP fetch here

	log.Info("Starting the request...")
	log.Debug("%v: %v", req.Options, *req.Request)
	// TODO: CORS preflight...

	// This is where the actual HTTP request is made. req.Request contains the
	// original, unmodified URL — mappingKey is NOT used here.
	resp, err := reqClient.Do(req.Request)
	if err != nil {
		return nil, err
	}

	// 4.3.4.4
	if req.Options.ResponseTainting == jstypes.ResponseTaintingCors {
		err = doCorsCheck(req.Options, resp)
		if err != nil {
			return nil, err
		}
	}
	// TODO: policy checks, etc...

	wrapper := conv.NewResponseWrapper(resp, reqCtx)

	return &wrapper, err
}

func onErrCleanup(mappingKey string) {
	// TODO: cancel stuff here.... somehow...

	id := state.ActiveRequests.GetId(mappingKey)
	// TODO: can we guarantee that rust is not holding any references to that id (that we don't know on this side)?
	if id == 0 {
		// if id doesn't exist it [probably] means the error was thrown before the request was properly created
		log.Debug("there doesn't seem to exist a request associated with mapping key %s", mappingKey)
		return
	}
	state.ActiveRequests.Remove(id)
	err := rust_bridge.RsFinishMixnetConnection(id)
	if err != nil {
		// TODO: can we do anything else in here?
		log.Error("failed to properly abort the request: %s", err)
	}
}

// generateMappingKey creates a unique key for the address mapping by appending
// random bytes to the URL. This allows concurrent requests to the same URL.
func generateMappingKey(rawURL string) string {
	b := make([]byte, 8)
	_, _ = rand.Read(b)
	return rawURL + "#" + hex.EncodeToString(b)
}

// MixFetch performs an HTTP request over the Mixnet.
//
// Two separate values flow through this function:
//   - request.Request: the actual HTTP request (with the real URL). This is
//     what gets sent over the wire via reqClient.Do(). It is never modified.
//   - mappingKey: a unique internal key (URL + random suffix) used only for
//     tracking the request in ActiveRequests.AddressMapping. It is never
//     sent over the wire. This allows concurrent requests to the same URL.
func MixFetch(request *conv.ParsedRequest) (any, error) {
	log.Info("_mixFetch: start")

	// Generate a unique mapping key for internal request tracking.
	mappingKey := generateMappingKey(request.Request.URL.String())
	for state.ActiveRequests.GetId(mappingKey) != 0 {
		mappingKey = generateMappingKey(request.Request.URL.String())
	}

	resCh := make(chan *conv.ResponseWrapper)
	errCh := make(chan error)
	go func(resCh chan *conv.ResponseWrapper, errCh chan error) {
		resp, err := performRequest(request, mappingKey)
		if err != nil {
			errCh <- err
		} else {
			resCh <- resp
		}
	}(resCh, errCh)

	select {
	case res := <-resCh:
		log.Debug("finished performing the request")
		log.Trace("response: %+v", *res)
		return conv.IntoJSResponse(res, request.Options)
	case err := <-errCh:
		log.Warn("request failure: %s", err)
		onErrCleanup(mappingKey)
		return nil, err
	}
}
