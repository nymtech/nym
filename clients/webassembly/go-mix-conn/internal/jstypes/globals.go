// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package jstypes

import (
	"fmt"
	"net/url"
	"syscall/js"
)

var (
	Error   = js.Global().Get("Error")
	Promise = js.Global().Get("Promise")
	Reflect = js.Global().Get("Reflect")
	Object  = js.Global().Get("Object")
	Origin  = js.Global().Get("location").Get("origin").String()
)

func OriginUrl() *url.URL {
	originUrl, originErr := url.Parse(Origin)
	if originErr != nil {
		panic(fmt.Sprintf("could not obtain origin: %s", originErr))
	}
	return originUrl
}
