// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// it's not a bridge per se, but it makes it easier to keep track of what rust endpoints we're calling

package main

import (
	"strconv"
	"syscall/js"
)

func rsSendClientData(requestId RequestId, data []byte) error {
	Debug("calling rust send_client_data")

	rawRequestId := strconv.FormatUint(requestId, 10)
	jsBytes := intoJsBytes(data)

	sendPromise := js.Global().Call("send_client_data", rawRequestId, jsBytes)
	_, err := await(sendPromise)
	if err != nil {
		panic("todo: extract error message")
	}
	return nil
}

func rsStartNewMixnetRequest(addr string) (RequestId, error) {
	Debug("calling rust start_new_mixnet_connection")

	requestPromise := js.Global().Call("start_new_mixnet_connection", addr)
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
	return js.Global().Call("mix_fetch_initialised").Bool()
}
