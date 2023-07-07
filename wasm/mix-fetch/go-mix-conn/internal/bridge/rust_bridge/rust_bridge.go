// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// it's not a bridge per se, but it makes it easier to keep track of what rust endpoints we're calling from go

package rust_bridge

import (
	"go-mix-conn/internal/helpers"
	"go-mix-conn/internal/jstypes"
	"go-mix-conn/internal/log"
	"go-mix-conn/internal/types"
	"strconv"
	"syscall/js"
)

const (
	// methods exposed by rust to go
	rustGoBridgeName = "__rs_go_bridge__"
)

func rsBridge() js.Value {
	return js.Global().Get(rustGoBridgeName)
}

func RsSendClientData(requestId types.RequestId, data []byte) error {
	log.Debug("calling rust send_client_data")

	rawRequestId := strconv.FormatUint(requestId, 10)
	jsBytes := helpers.IntoJsBytes(data)

	sendPromise := rsBridge().Call("send_client_data", rawRequestId, jsBytes)
	_, err := jstypes.Await(sendPromise)
	if err != nil {
		panic("todo: extract error message")
	}
	return nil
}

func RsStartNewMixnetRequest(addr string) (types.RequestId, error) {
	log.Debug("calling rust start_new_mixnet_connection")

	requestPromise := rsBridge().Call("start_new_mixnet_connection", addr)
	rawRequestId, errRes := jstypes.Await(requestPromise)
	if errRes != nil {
		panic("todo: extract error message")
	}

	if len(rawRequestId) != 1 {
		panic("todo: handle error here")
	}

	requestId, err := helpers.ParseRequestId(rawRequestId[0])
	if err != nil {
		return 0, err
	}

	log.Debug("started request has id %d", requestId)

	return requestId, err
}

func RsIsInitialised() bool {
	log.Debug("calling rust mix_fetch_initialised")

	return rsBridge().Call("mix_fetch_initialised").Bool()
}

func RsFinishMixnetConnection(requestId types.RequestId) error {
	log.Debug("calling rust finish_mixnet_connection")

	rawRequestId := strconv.FormatUint(requestId, 10)

	finishPromise := rsBridge().Call("finish_mixnet_connection", rawRequestId)
	_, err := jstypes.Await(finishPromise)
	if err != nil {
		panic("todo: extract error message")
	}
	return nil
}
