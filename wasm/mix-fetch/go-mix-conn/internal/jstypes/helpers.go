// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package jstypes

import (
	"go-mix-conn/internal/helpers"
	"go-mix-conn/internal/log"
	"net/http"
	"net/url"
	"strings"
)

// see https://datatracker.ietf.org/doc/html/rfc7230#section-3.2
func IsValidHeaderName(raw string) bool {
	return helpers.IsToken(raw)
}

// see https://fetch.spec.whatwg.org/#forbidden-header-name
func IsForbiddenRequestHeaderName(name string) bool {
	if DiscreteForbiddenHeaderNames.Contains(name) {
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
	switch helpers.ByteLowercase(name) {
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
		log.Error("unimplemented 'range' header")
		return false
	default:
		return false
	}

	// 3
	return true
}

// Reference: https://fetch.spec.whatwg.org/#cors-unsafe-request-header-names
func CorsUnsafeRequestHeaderNames(headers http.Header) []string {
	var unsafeNames []string
	var potentiallyUnsafeNames []string
	safelistValueSize := 0

	// 4
	for name, value := range headers {
		// TODO: is that actually correct?
		wasUnsafe := false
		for _, v := range value {
			// 4.1
			if !isCorsSafelistedRequestHeader(name, v) {
				unsafeNames = append(unsafeNames, name)
				wasUnsafe = true
				break
			}
		}
		if !wasUnsafe {
			potentiallyUnsafeNames = append(potentiallyUnsafeNames, name)
			safelistValueSize += len(value)
		}
	}

	// 5
	if safelistValueSize > 1024 {
		for _, name := range potentiallyUnsafeNames {
			unsafeNames = append(unsafeNames, name)
		}
	}

	return helpers.SortedByteLowercase(unsafeNames)
}

// Reference: https://fetch.spec.whatwg.org/#cors-safelisted-method
func IsCorsSafelistedMethod(method string) bool {
	if method == "GET" || method == "HEAD" || method == "POST" {
		return true
	} else {
		return false
	}
}

func IsSameOrigin(remoteUrl *url.URL) bool {
	originUrl := OriginUrl()

	// Roughly speaking, two URIs are part of the same JsOrigin (i.e., represent the same principal)
	// if they have the same scheme, host, and port.
	// Reference: https://www.rfc-editor.org/rfc/rfc6454.html#section-3.2
	if originUrl.Scheme != remoteUrl.Scheme || originUrl.Host != remoteUrl.Host || originUrl.Port() != remoteUrl.Port() {
		return false
	}
	return true
}
