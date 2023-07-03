// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// it's not a bridge per se, but it makes it easier to keep track of what rust endpoints we're calling from go

package main

import (
	"strconv"
	"syscall/js"
)

var (
	rsBridge = js.Global().Get(rustGoBridgeName)
)

func rsSendClientData(requestId RequestId, data []byte) error {
	Debug("calling rust send_client_data")

	rawRequestId := strconv.FormatUint(requestId, 10)
	jsBytes := intoJsBytes(data)

	sendPromise := rsBridge.Call("send_client_data", rawRequestId, jsBytes)
	_, err := await(sendPromise)
	if err != nil {
		panic("todo: extract error message")
	}
	return nil
}

func rsStartNewMixnetRequest(addr string) (RequestId, error) {
	Debug("calling rust start_new_mixnet_connection")

	requestPromise := rsBridge.Call("start_new_mixnet_connection", addr)
	rawRequestId, errRes := await(requestPromise)
	if errRes != nil {
		panic("todo: extract error message")
	}

	if len(rawRequestId) != 1 {
		panic("todo: handle error here")
	}

	requestId, err := parseRequestId(rawRequestId[0])
	if err != nil {
		return 0, err
	}
	if activeRequests.exists(requestId) {
		panic("todo: handle duplicate connection")
	}

	Debug("started request has id %d", requestId)

	return requestId, err
}

func rsIsInitialised() bool {
	return rsBridge.Call("mix_fetch_initialised").Bool()
}

func rsFinishMixnetConnection(requestId RequestId) error {
	Debug("calling rust finish_mixnet_connection")

	rawRequestId := strconv.FormatUint(requestId, 10)

	finishPromise := rsBridge.Call("finish_mixnet_connection", rawRequestId)
	_, err := await(finishPromise)
	if err != nil {
		panic("todo: extract error message")
	}
	return nil
}
