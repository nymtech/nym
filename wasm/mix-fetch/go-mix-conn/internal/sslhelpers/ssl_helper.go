// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package sslhelpers

import (
	"crypto/tls"
)

func TlsConfig(serverName string) tls.Config {
	return tls.Config{
		ServerName: serverName,
		RootCAs:    rootCerts(),
	}
}
