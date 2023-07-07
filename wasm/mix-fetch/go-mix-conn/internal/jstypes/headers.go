// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package jstypes

import (
	"go-mix-conn/internal/external"
	"go-mix-conn/internal/helpers"
)

const (
	HeaderOrigin = "Origin"

	HeaderRequestMethod         = "Access-Control-Request-Method"
	HeaderRequestHeaders        = "Access-Control-Request-Headers"
	HeaderRequestLocalNetwork   = "Access-Control-Request-Local-Network"
	HeaderRequestPrivateNetwork = "Access-Control-Request-Private-Network"

	HeaderAllowMethods        = "Access-Control-Allow-Methods"
	HeaderAllowHeaders        = "Access-Control-Allow-Headers"
	HeaderMageAge             = "Access-Control-Max-Age"
	HeaderAllowLocalNetwork   = "Access-Control-Allow-Local-Network"
	HeaderAllowPrivateNetwork = "Access-Control-Allow-Private-Network"

	// Indicates whether the response can be shared, via returning the literal value of the `Origin` request header (which can be `null`) or `*` in a response.
	HeaderAllowOrigin = "Access-Control-Allow-Origin"
	// Indicates whether the response can be shared when request’s credentials mode is "include".
	HeaderAllowCredentials = "Access-Control-Allow-Credentials"

	HeaderExposeHeaders = "Access-Control-Expose-Headers"

	Wildcard = "*"
)

// see https://fetch.spec.whatwg.org/#forbidden-header-name
var DiscreteForbiddenHeaderNames = external.NewSet(
	"accept-charset",
	"accept-encoding",
	helpers.ByteLowercase(HeaderRequestHeaders),
	helpers.ByteLowercase(HeaderRequestMethod),
	// see https://wicg.github.io/local-network-access/#forbidden-header-names
	helpers.ByteLowercase(HeaderRequestLocalNetwork),
	helpers.ByteLowercase(HeaderRequestPrivateNetwork),
	"connection",
	"content-length",
	"cookie",
	"cookie2",
	"date",
	"dnt",
	"expect",
	"host",
	"keep-alive",
	helpers.ByteLowercase(HeaderOrigin),
	"referer",
	"set-cookie",
	"te",
	"trailer",
	"transfer-encoding",
	"upgrade",
	"via",
)

// almost always a mistake to allow the following as request headers
// as a result of misunderstanding of the CORS protocol.
var DisallowedRequestHeaderNames = external.NewSet(
	helpers.ByteLowercase(Wildcard),
	helpers.ByteLowercase(HeaderAllowOrigin),
	helpers.ByteLowercase(HeaderAllowCredentials),
	helpers.ByteLowercase(HeaderAllowMethods),
	helpers.ByteLowercase(HeaderAllowHeaders),
	helpers.ByteLowercase(HeaderAllowLocalNetwork),
	helpers.ByteLowercase(HeaderAllowPrivateNetwork),
	helpers.ByteLowercase(HeaderMageAge),
	helpers.ByteLowercase(HeaderExposeHeaders),
)

// almost always a mistake to expose the following as response headers
var DisallowedResponseHeaderNames = external.NewSet(
	helpers.ByteLowercase(Wildcard),
	helpers.ByteLowercase(HeaderOrigin),
	helpers.ByteLowercase(HeaderRequestMethod),
	helpers.ByteLowercase(HeaderRequestHeaders),
	helpers.ByteLowercase(HeaderRequestLocalNetwork),
	helpers.ByteLowercase(HeaderRequestPrivateNetwork),
)

// see https://fetch.spec.whatwg.org/#forbidden-response-header-name
var ForbiddenResponseHeaderNames = external.NewSet(
	"set-cookie",
	"set-cookie2",
)

// see https://fetch.spec.whatwg.org/#cors-safelisted-response-header-name
var SafelistedResponseHeaderNames = external.NewSet(
	"cache-control",
	"content-language",
	"content-length",
	"content-type",
	"expires",
	"last-modified",
	"pragma",
)
