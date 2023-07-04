// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//go:build js && wasm

package main

import (
	"go-mix-conn/internal/bridge/go_bridge"
	"go-mix-conn/internal/state"
)

var done chan struct{}

func init() {
	println("[go init]: go module init")

	done = make(chan struct{})
	state.InitialiseGlobalState()

	println("[go init]: go module init finished")
}

func main() {
	println("[go main]: go module loaded")

	go_bridge.InitialiseGoBridge()
	<-done

	println("[go main]: go module finished")
}
