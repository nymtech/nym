// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"context"
	"errors"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/url"
	"sync"
	"syscall/js"
)

type RequestId = uint64

type ActiveRequests struct {
	sync.Mutex
	inner map[RequestId]*ConnectionInjector
}

func (ar *ActiveRequests) exists(id RequestId) bool {
	Debug("checking if request %d exists", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	return exists
}

func (ar *ActiveRequests) insert(id RequestId, conn *ConnectionInjector) {
	Debug("inserting request %d", id)
	ar.Lock()
	defer ar.Unlock()
	ar.inner[id] = conn
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
	ar.inner[id].injectedServerData <- data
}

func (ar *ActiveRequests) closeRemoteSocket(id RequestId) {
	Debug("closing remote socket for %d", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	if !exists {
		panic("attempted to close remote socket of a connection that doesn't exist")
	}
	ar.inner[id].closedRemote.Store(true)
}

func buildHttpClient(requestId RequestId) *http.Client {
	if _, exists := activeRequests.inner[requestId]; exists {
		panic("duplicate connection detected")
	}

	return &http.Client{
		Transport: &http.Transport{
			DialContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
				Info("entered DialContext")

				if activeRequests.exists(requestId) {
					return nil, errors.New("duplicate plain connection detected")
				}

				conn, inj := NewFakeConnection(requestId)
				activeRequests.insert(requestId, &inj)

				return conn, nil
			},

			DialTLSContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
				Info("entered DialTLSContext")

				if activeRequests.exists(requestId) {
					return nil, errors.New("duplicate SSL connection detected")
				}

				conn, inj := NewFakeTlsConn(requestId)
				activeRequests.insert(requestId, &inj)

				if err := conn.Handshake(); err != nil {
					return nil, err
				}

				return conn, nil
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

func performRequest(requestId RequestId, rawEndpoint string) (*http.Response, error) {
	// we just want to parse the url to make sure its valid
	_, err := url.Parse(rawEndpoint)
	if err != nil {
		return nil, err
	}

	// build the request
	Info("Building request for %s", rawEndpoint)
	// TODO: deal with other http methods later
	req, err := http.NewRequest(http.MethodGet, rawEndpoint, nil)
	if err != nil {
		return nil, err
	}
	reqClient := buildHttpClient(requestId)

	Info("Starting the request...")
	return reqClient.Do(req)
}

func _mixFetch(requestId RequestId, endpoint string) (any, error) {
	Info("_mixFetch: start")

	fmt.Printf("GO: got %d and %s\n", requestId, endpoint)

	resp, err := performRequest(requestId, endpoint)
	if err != nil {
		return nil, err
	}
	Info("finished performing the request")

	defer func(Body io.ReadCloser) {
		err := Body.Close()
		if err != nil {
			Error("failed to close the response body: %s", err)
		}
	}(resp.Body)

	// Read the response body
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}

	jsBytes := intoJsBytes(data)

	// Create a Response object and pass the data
	responseConstructor := js.Global().Get("Response")
	response := responseConstructor.New(jsBytes)

	// TODO: insert headers, status codes, etc.

	return response, nil
}
