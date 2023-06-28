// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"strings"
)

const (
	headerOrigin = "Origin"

	headerRequestMethod         = "Access-Control-Request-Method"
	headerRequestHeaders        = "Access-Control-Request-Headers"
	headerRequestLocalNetwork   = "Access-Control-Request-Local-Network"
	headerRequestPrivateNetwork = "Access-Control-Request-Private-Network"

	headerAllowMethods        = "Access-Control-Allow-Methods"
	headerAllowHeaders        = "Access-Control-Allow-Headers"
	headerMageAge             = "Access-Control-Max-Age"
	headerAllowLocalNetwork   = "Access-Control-Allow-Local-Network"
	headerAllowPrivateNetwork = "Access-Control-Allow-Private-Network"

	// Indicates whether the response can be shared, via returning the literal value of the `Origin` request header (which can be `null`) or `*` in a response.
	headerAllowOrigin = "Access-Control-Allow-Origin"
	// Indicates whether the response can be shared when request’s credentials mode is "include".
	headerAllowCredentials = "Access-Control-Allow-Credentials"

	headerExposeHeaders = "Access-Control-Expose-Headers"

	wildcard = "*"
)

// see https://fetch.spec.whatwg.org/#forbidden-header-name
var discreteForbiddenHeaderNames = NewSet(
	"accept-charset",
	"accept-encoding",
	byteLowercase(headerRequestHeaders),
	byteLowercase(headerRequestMethod),
	// see https://wicg.github.io/local-network-access/#forbidden-header-names
	byteLowercase(headerRequestLocalNetwork),
	byteLowercase(headerRequestPrivateNetwork),
	"connection",
	"content-length",
	"cookie",
	"cookie2",
	"date",
	"dnt",
	"expect",
	"host",
	"keep-alive",
	byteLowercase(headerOrigin),
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
var disallowedRequestHeaderNames = NewSet(
	byteLowercase(wildcard),
	byteLowercase(headerAllowOrigin),
	byteLowercase(headerAllowCredentials),
	byteLowercase(headerAllowMethods),
	byteLowercase(headerAllowHeaders),
	byteLowercase(headerAllowLocalNetwork),
	byteLowercase(headerAllowPrivateNetwork),
	byteLowercase(headerMageAge),
	byteLowercase(headerExposeHeaders),
)

// almost always a mistake to expose the following as response headers
var disallowedResponseHeaderNames = NewSet(
	byteLowercase(wildcard),
	byteLowercase(headerOrigin),
	byteLowercase(headerRequestMethod),
	byteLowercase(headerRequestHeaders),
	byteLowercase(headerRequestLocalNetwork),
	byteLowercase(headerRequestPrivateNetwork),
)

// see https://fetch.spec.whatwg.org/#forbidden-response-header-name
var forbiddenResponseHeaderNames = NewSet(
	"set-cookie",
	"set-cookie2",
)

// see https://fetch.spec.whatwg.org/#cors-safelisted-response-header-name
var safelistedResponseHeaderNames = NewSet(
	"cache-control",
	"content-language",
	"content-length",
	"content-type",
	"expires",
	"last-modified",
	"pragma",
)

// see https://datatracker.ietf.org/doc/html/rfc7230#section-3.2
func isValidHeaderName(raw string) bool {
	return isToken(raw)
}

// see https://fetch.spec.whatwg.org/#forbidden-header-name
func isForbiddenRequestHeaderName(name string) bool {
	if discreteForbiddenHeaderNames.Contains(name) {
		return true
	}
	return strings.HasPrefix(name, "proxy-") ||
		strings.HasPrefix(name, "sec-")
}

// A CORS-unsafe request-header byte is a byte *byte* for which one of the following is true:
// - byte is less than 0x20 and is not 0x09 HT
// - byte is 0x22 ("), 0x28 (left parenthesis), 0x29 (right parenthesis), 0x3A (:), 0x3C (<), 0x3E (>), 0x3F (?), 0x40 (@), 0x5B ([), 0x5C (\), 0x5D (]), 0x7B ({), 0x7D (}), or 0x7F DEL.
// Reference: https://fetch.spec.whatwg.org/#cors-unsafe-request-header-byte
func containsCORSUnsafeRequestHeaderByte(b []byte) bool {
	for i := 0; i < len(b); i++ {
		v := b[i]
		if v < 0x20 && v != 0x09 {
			return true
		}
		if v == 0x22 || v == 0x28 || v == 0x29 || v == 0x3A || v == 0x3C || v == 0x3E || v == 0x3F || v == 0x40 || v == 0x5B || v == 0x5C || v == 0x5D || v == 0x7B || v == 0x7D || v == 0x7F {
			return true
		}
	}

	return false
}

// Reference: https://fetch.spec.whatwg.org/#cors-safelisted-request-header
func isCorsSafelistedRequestHeader(name string, value string) bool {
	// 1
	if len(value) > 128 {
		return false
	}

	// 2
	switch byteLowercase(name) {
	case "accept":
		if containsCORSUnsafeRequestHeaderByte([]byte(value)) {
			return false
		}
	case "accept-language", "content-language":
		valueBytes := []byte(value)
		for i := 0; i < len(valueBytes); i++ {
			v := valueBytes[i]
			if !(0x30 <= v && v <= 0x39) && !(0x41 <= v && v <= 0x5A) && !(0x61 <= v && v <= 0x7A) && v != 0x20 && v != 0x2A && v != 0x2C && v != 0x2D && v != 0x2E && v != 0x3B && v != 0x3D {
				return false
			}
		}
	case "content-type":
		// 2.1
		if containsCORSUnsafeRequestHeaderByte([]byte(value)) {
			return false
		}
		// 2.4
		if value != "application/x-www-form-urlencoded" && value != "multipart/form-data" && value != "text/plain" {
			return false
		}
	case "range":
		Error("unimplemented 'range' header")
		return false
	default:
		return false
	}

	// 3
	return true
}
