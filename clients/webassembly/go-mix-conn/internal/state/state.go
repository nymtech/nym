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
		Mutex: sync.Mutex{},
		Inner: make(map[types.RequestId]*ActiveRequest),
	}
}

type CurrentActiveRequests struct {
	sync.Mutex
	Inner map[types.RequestId]*ActiveRequest
}

func (ar *CurrentActiveRequests) Exists(id types.RequestId) bool {
	log.Debug("checking if request %d exists", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.Inner[id]
	return exists
}

func (ar *CurrentActiveRequests) Insert(id types.RequestId, inj ConnectionInjector) {
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.Inner[id]
	if exists {
		panic("attempted to overwrite active connection")
	}
	ar.Inner[id] = &ActiveRequest{injector: inj}
}

func (ar *CurrentActiveRequests) Remove(id types.RequestId) {
	log.Debug("removing request %d", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.Inner[id]
	if !exists {
		panic("attempted to remove active connection that doesn't exist")
	}
	delete(ar.Inner, id)
}

func (ar *CurrentActiveRequests) InjectData(id types.RequestId, data []byte) {
	log.Debug("injecting data for %d", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.Inner[id]
	if !exists {
		panic("attempted to write to connection that doesn't exist")
	}
	ar.Inner[id].injector.ServerData <- data
}

func (ar *CurrentActiveRequests) CloseRemoteSocket(id types.RequestId) {
	log.Debug("closing remote socket for %d", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.Inner[id]
	if !exists {
		log.Warn("attempted to close remote socket of a connection that doesn't exist")
		return
	}
	close(ar.Inner[id].injector.RemoteDone)
}

func (ar *CurrentActiveRequests) SendError(id types.RequestId, err error) {
	log.Debug("injecting error for %d: %s", id, err)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.Inner[id]
	if !exists {
		panic("attempted to inject error data to connection that doesn't exist")
	}
	ar.Inner[id].injector.RemoteError <- err
}

type ActiveRequest struct {
	injector ConnectionInjector
}
