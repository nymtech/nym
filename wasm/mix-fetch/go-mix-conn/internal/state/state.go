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

// CurrentActiveRequests tracks ongoing requests for thread-safe access.
// The AddressMapping uses unique keys (URL + random suffix) so that
// concurrent requests to the same URL each get their own entry.
type CurrentActiveRequests struct {
	sync.Mutex
	Requests       map[types.RequestId]*ActiveRequest
	AddressMapping map[string]types.RequestId // key is URL + random suffix
}

// GetId returns the request ID associated with the given mapping key.
func (ar *CurrentActiveRequests) GetId(mappingKey string) types.RequestId {
	log.Trace("getting id associated with mapping key %s", mappingKey)
	ar.Lock()
	defer ar.Unlock()
	return ar.AddressMapping[mappingKey]
}

func (ar *CurrentActiveRequests) Exists(id types.RequestId) bool {
	log.Trace("checking if request %d exists", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.Requests[id]
	return exists
}

// Insert adds a new active request to the tracking maps.
// The mappingKey should be a unique key (URL + random suffix) for this request.
func (ar *CurrentActiveRequests) Insert(id types.RequestId, mappingKey string, inj ConnectionInjector) {
	log.Trace("inserting request %d with mapping key %s", id, mappingKey)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.Requests[id]
	if exists {
		panic("attempted to overwrite active connection id")
	}
	_, exists = ar.AddressMapping[mappingKey]
	if exists {
		panic("attempted to overwrite active connection mapping key")
	}

	ar.Requests[id] = &ActiveRequest{injector: inj, mappingKey: mappingKey}
	ar.AddressMapping[mappingKey] = id
}

func (ar *CurrentActiveRequests) Remove(id types.RequestId) {
	log.Trace("removing request %d", id)
	ar.Lock()
	defer ar.Unlock()
	req, exists := ar.Requests[id]
	if !exists {
		panic("attempted to remove active connection id that doesn't exist")
	}
	_, exists = ar.AddressMapping[req.mappingKey]
	if !exists {
		panic("attempted to remove active connection mapping key that doesn't exist")
	}

	delete(ar.Requests, id)
	delete(ar.AddressMapping, req.mappingKey)
}

func (ar *CurrentActiveRequests) InjectData(id types.RequestId, data []byte) {
	log.Trace("injecting data for %d", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.Requests[id]
	if !exists {
		log.Error("attempted to inject data for connection %d that no longer exists — likely already cleaned up", id)
		return
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
		log.Error("attempted to inject error for connection %d that no longer exists — likely already cleaned up", id)
		return
	}
	ar.Requests[id].injector.RemoteError <- err
}

type ActiveRequest struct {
	injector   ConnectionInjector
	mappingKey string // Unique key for AddressMapping (URL + random suffix)
}
