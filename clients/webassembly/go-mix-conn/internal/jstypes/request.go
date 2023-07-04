// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package jstypes

type Redirect = string
type Mode = string
type CredentialsMode = string
type ResponseTainting = string
type ReferrerPolicy = string
type Referrer = string

const (
	REQUEST_REDIRECT_ERROR  = "error"
	REQUEST_REDIRECT_MANUAL = "manual"
	REQUEST_REDIRECT_FOLLOW = "follow"

	RESPONSE_TAINTING_BASIC              = "basic"
	RESPONSE_TAINTING_CORS               = "cors"
	RESPONSE_TAINTING_OPAQUE             = "opaque"
	RESPONSE_TAINTING_UNSAFE_IGNORE_CORS = "ignore-cors"

	MODE_CORS               = "cors"
	MODE_SAME_ORIGIN        = "same-origin"
	MODE_NO_CORS            = "no-cors"
	MODE_NAVIGATE           = "navigate"
	MODE_WEBSOCKET          = "websocket"
	MODE_UNSAFE_IGNORE_CORS = "unsafe-ignore-cors"

	CREDENTIALS_MODE_OMIT        = "omit"
	CREDENTIALS_MODE_SAME_ORIGIN = "same-origin"
	CREDENTIALS_MODE_INCLUDE     = "include"

	REFERRER_POLICY_NO_REFERRER                     = "no-referrer"
	REFERRER_POLICY_NO_REFERRER_WHEN_DOWNGRADE      = "no-referrer-when-downgrade"
	REFERRER_POLICY_ORIGIN                          = "origin"
	REFERRER_POLICY_ORIGIN_WHEN_CROSS_ORIGIN        = "origin-when-cross-origin"
	REFERRER_POLICY_SAME_ORIGIN                     = "same-origin"
	REFERRER_POLICY_STRICT_ORIGIN                   = "strict-origin"
	REFERRER_POLICY_STRICT_ORIGIN_WHEN_CROSS_ORIGIN = "strict-origin-when-cross-origin"
	REFERRER_POLICY_UNSAFE_URL                      = "unsafe-url"

	REFERRER_NO_REFERRER = "no-referrer"
	REFERRER_CLIENT      = "client"
)
