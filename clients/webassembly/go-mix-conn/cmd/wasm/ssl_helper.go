// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"crypto/tls"
)

func tlsConfig(serverName string) tls.Config {
	return tls.Config{
		ServerName: serverName,
		RootCAs:    rootCerts(),
	}
}
