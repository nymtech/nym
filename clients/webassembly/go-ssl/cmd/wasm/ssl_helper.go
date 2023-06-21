// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"crypto/tls"
	"crypto/x509"
)

// func tlsConfig(sni string) tls.Config {
func tlsConfig() tls.Config {
	return tls.Config{
		VerifyPeerCertificate: func(rawCerts [][]byte, verifiedChains [][]*x509.Certificate) error {
			Error("TODO: implement VerifyPeerCertificate")
			return nil
		},
		// Set InsecureSkipVerify to skip the default validation we are
		// replacing. This will not disable VerifyConnection.
		InsecureSkipVerify: true,
		//VerifyConnection: func(cs tls.ConnectionState) error {
		//	Error("TODO: implement VerifyConnection")
		//	fmt.Printf("%+v\n", cs)
		//	return nil
		//},
		//ServerName: sni,
	}
}

func setupFakeTlsConn(connectionId ConnectionId) ManagedConnection {
	conn, inj := NewFakeConnection(connectionId)
	tlsConfig := tlsConfig()

	//tlsConn := tls.UClient(fakeConnection, &tlsConfig, tls.HelloGolang)
	tlsConn := tls.Client(conn, &tlsConfig)
	return NewTlsManagedConnection(tlsConn, inj)
}

func performSSLHandshake() {
	if !ensureManagedTls() {
		panic("no TLS connection established")
	}
	err := currentConnection.tlsConn.Handshake()
	if err != nil {
		panic(err)
	}
	Info("finished SSL handshake")
}

//func startSSLHandshake() error {
//	if currentConnection != nil {
//		Error("only a single SSL connection can be established at a time (for now)")
//		return fmt.Errorf("duplicate SSL handshake")
//	}
//
//	// TODO: sni vs actual endpoint
//	setupFakeTlsConn()
//
//	// TODO: or maybe do outside goroutine?
//	go func() {
//		Info("starting SSL handshake for %v\n")
//		performSSLHandshake()
//	}()
//
//	return nil
//}
