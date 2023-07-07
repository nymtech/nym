// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package mixfetch

import (
	"golang.org/x/net/idna"
	"net"
	"net/url"
	"unicode"
)

var portMap = map[string]string{
	"http":   "80",
	"https":  "443",
	"socks5": "1080",
}

// canonicalAddr returns url.Host but always with a ":port" suffix.
// source: https://github.com/golang/go/blob/release-branch.go1.20/src/net/http/transport.go#L2747
func canonicalAddr(url *url.URL) string {
	addr := url.Hostname()
	if v, err := idnaASCII(addr); err == nil {
		addr = v
	}
	port := url.Port()
	if port == "" {
		port = portMap[url.Scheme]
	}
	return net.JoinHostPort(addr, port)
}

// source: https://github.com/golang/go/blob/release-branch.go1.20/src/net/http/request.go#L724
func idnaASCII(v string) (string, error) {
	// TODO: Consider removing this check after verifying performance is okay.
	// Right now punycode verification, length checks, context checks, and the
	// permissible character tests are all omitted. It also prevents the ToASCII
	// call from salvaging an invalid IDN, when possible. As a result it may be
	// possible to have two IDNs that appear identical to the user where the
	// ASCII-only version causes an error downstream whereas the non-ASCII
	// version does not.
	// Note that for correct ASCII IDNs ToASCII will only do considerably more
	// work, but it will not cause an allocation.
	if isAscii(v) {
		return v, nil
	}
	return idna.Lookup.ToASCII(v)
}

// isAscii returns whether s is ASCII.
// source: https://github.com/golang/go/blob/release-branch.go1.20/src/net/http/internal/ascii/print.go#L46
func isAscii(s string) bool {
	for i := 0; i < len(s); i++ {
		if s[i] > unicode.MaxASCII {
			return false
		}
	}
	return true
}
