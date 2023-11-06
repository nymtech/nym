// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//go:build js && wasm

package main

import (
	"go-mix-conn/internal/bridge/go_bridge"
	"go-mix-conn/internal/state"
	"syscall/js"
)

var done chan struct{}

func init() {
	println("[go init]: go module init")
	
	q := js.Global().Get("location")
	if q.IsUndefined() {
	  println("location undefined")
	} else {
	  println("location ok")
	}
	a := js.Global().Get("Error")
	if a.IsUndefined() {
	  println("Error undefined")
	} else {
	  println("Error ok")
	}
	b := js.Global().Get("Promise") 
	if b.IsUndefined() {
	  println("Promise undefined")
	} else {
	  println("Promise ok")
	}
	c := js.Global().Get("Reflect")
	if c.IsUndefined() {
	  println("Reflect undefined")
	} else {
	  println("Reflect ok")
	}
	d := js.Global().Get("Object")
	if d.IsUndefined() {
	println("Object undefined")
	} else {
	  println("Object ok")
	}
	e := js.Global().Get("Response")
	if e.IsUndefined() {
	println("Response undefined")
	} else {
	  println("Response ok")
	}
	f := js.Global().Get("Request")
	if f.IsUndefined() {
	println("Request undefined")
	} else {
	  println("Request ok")
	}
	g := js.Global().Get("Proxy")
	if g.IsUndefined() {
	println("Proxy undefined")
	} else {
	  println("Proxy ok")
	}
	h := js.Global().Get("Headers")
	if h.IsUndefined() {
	println("Headers undefined")
	} else {
	  println("Headers ok")
	}
	i := js.Global().Get("Uint8Array")
	if i.IsUndefined() {
	  println("Uint8Array undefined")
	} else {
	  println("Uint8Array ok")
	}

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
