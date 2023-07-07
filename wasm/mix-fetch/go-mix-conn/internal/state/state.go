// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package state

import (
	"go-mix-conn/internal/log"
	"go-mix-conn/internal/types"
	"sync"
	"time"
)

// ALL THE GLOBALS SHOULD GO HERE
var ActiveRequests *CurrentActiveRequests
var RequestTimeout time.Duration = time.Second * 5

// 4.4.7
var MaxRedirections int = 20

func InitialiseGlobalState() {
	ActiveRequests = &CurrentActiveRequests{
		Mutex:          sync.Mutex{},
		Requests:       make(map[types.RequestId]*ActiveRequest),
		AddressMapping: make(map[string]types.RequestId),
	}
}

type CurrentActiveRequests struct {
	sync.Mutex
	Requests       map[types.RequestId]*ActiveRequest
	AddressMapping map[string]types.RequestId
}

func (ar *CurrentActiveRequests) GetId(canonicalAddr string) types.RequestId {
	log.Trace("getting id associated with request for %s", canonicalAddr)
	ar.Lock()
	defer ar.Unlock()
	return ar.AddressMapping[canonicalAddr]
}

func (ar *CurrentActiveRequests) Exists(id types.RequestId) bool {
	log.Trace("checking if request %d exists", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.Requests[id]
	return exists
}

func (ar *CurrentActiveRequests) ExistsCanonical(canonicalAddr string) bool {
	return ar.GetId(canonicalAddr) != 0
}

func (ar *CurrentActiveRequests) Insert(id types.RequestId, canonicalAddr string, inj ConnectionInjector) {
	log.Trace("inserting request %d for %s", id, canonicalAddr)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.Requests[id]
	if exists {
		panic("attempted to overwrite active connection id")
	}
	_, exists = ar.AddressMapping[canonicalAddr]
	if exists {
		panic("attempted to overwrite active connection canonicalAddr")
	}

	ar.Requests[id] = &ActiveRequest{injector: inj, canonicalAddr: canonicalAddr}
	ar.AddressMapping[canonicalAddr] = id
}

func (ar *CurrentActiveRequests) Remove(id types.RequestId) {
	log.Trace("removing request %d", id)
	ar.Lock()
	defer ar.Unlock()
	req, exists := ar.Requests[id]
	if !exists {
		panic("attempted to remove active connection id that doesn't exist")
	}
	_, exists = ar.AddressMapping[req.canonicalAddr]
	if !exists {
		panic("attempted to remove active connection canonicalAddr that doesn't exist")
	}

	delete(ar.Requests, id)
	delete(ar.AddressMapping, req.canonicalAddr)
}

func (ar *CurrentActiveRequests) InjectData(id types.RequestId, data []byte) {
	log.Trace("injecting data for %d", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.Requests[id]
	if !exists {
		panic("attempted to write to connection that doesn't exist")
	}
	ar.Requests[id].injector.ServerData <- data
}

func (ar *CurrentActiveRequests) CloseRemoteSocket(id types.RequestId) {
	log.Trace("closing remote socket for %d", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.Requests[id]
	if !exists {
		log.Warn("attempted to close remote socket of a connection that doesn't exist")
		return
	}
	close(ar.Requests[id].injector.RemoteDone)
}

func (ar *CurrentActiveRequests) SendError(id types.RequestId, err error) {
	log.Trace("injecting error for %d: %s", id, err)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.Requests[id]
	if !exists {
		panic("attempted to inject error data to connection that doesn't exist")
	}
	ar.Requests[id].injector.RemoteError <- err
}

type ActiveRequest struct {
	injector      ConnectionInjector
	canonicalAddr string
}
