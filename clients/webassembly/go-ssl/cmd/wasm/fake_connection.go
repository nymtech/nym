// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"bytes"
	"crypto/tls"
	"encoding/hex"
	"io"
	"net"
	"strconv"
	"sync/atomic"
	"syscall/js"
	"time"
)

// ConnectionInjector controls data that goes over corresponding FakeConnection
type ConnectionInjector struct {
	injectedServerData chan []byte
	closedRemote       *atomic.Bool
}

// FakeConnection is a type implementing net.Conn interface that allows us
// to inspect and control bytes that would normally go onto the wire
type FakeConnection struct {
	requestId RequestId
	injector  *ConnectionInjector

	incompleteReads chan []byte
}

// NewFakeConnection creates a new FakeConnection that implements net.Conn interface alongside
// handlers for injecting data into it
func NewFakeConnection(requestId RequestId) (FakeConnection, ConnectionInjector) {
	inj := ConnectionInjector{
		injectedServerData: make(chan []byte, 10),
		closedRemote:       &atomic.Bool{},
	}

	conn := FakeConnection{
		injector:        &inj,
		requestId:       requestId,
		incompleteReads: make(chan []byte, 1),
	}

	return conn, inj
}

// NewFakeTlsConn wraps a FakeConnection with all the TLS magic
// note: this returns a tls.Conn in the pre-handshake state
func NewFakeTlsConn(connectionId RequestId) (*tls.Conn, ConnectionInjector) {
	conn, inj := NewFakeConnection(connectionId)
	tlsConfig := tlsConfig()
	tlsConn := tls.Client(conn, &tlsConfig)
	return tlsConn, inj
}

func (conn FakeConnection) readAndBuffer(in []byte, out []byte) (int, error) {
	buf := bytes.NewReader(in)
	n, err := buf.Read(out)

	remaining := buf.Len()
	if remaining > 0 {
		leftover := make([]byte, remaining)
		_, _ = buf.Read(leftover)
		conn.incompleteReads <- leftover
	}

	encoded := hex.EncodeToString(out[:n])
	Debug("READING INJECTED >>> %s\n", encoded)
	return n, err
}

func (conn FakeConnection) Read(p []byte) (int, error) {
	select {
	// see if we have any leftover data from the previous read
	case incomplete := <-conn.incompleteReads:
		Debug("reading previously incomplete data")
		return conn.readAndBuffer(incomplete, p)
	default:
		select {
		// if we have any data: do read it
		case injectedData := <-conn.injector.injectedServerData:
			return conn.readAndBuffer(injectedData, p)
		default:
			// otherwise see if the socket is closed
			if conn.injector.closedRemote.Load() {
				return 0, io.EOF
			} else {
				Debug("waiting for data to read...")
				// wait for the data
				// TODO: what if we received information about closed socket here?
				data := <-conn.injector.injectedServerData
				if len(data) == 0 {
					return 0, io.EOF
				}
				return conn.readAndBuffer(data, p)
			}
		}
	}
}

func (conn FakeConnection) Write(p []byte) (int, error) {
	encoded := hex.EncodeToString(p)
	Debug("WRITING TO 'REMOTE' >>> %s\n", encoded)

	requestId := strconv.FormatUint(conn.requestId, 10)
	jsBytes := intoJsBytes(p)

	sendPromise := js.Global().Call("send_client_data", requestId, jsBytes)
	_, err := await(sendPromise)
	if err != nil {
		Error("failed to resolve sendPromise")
		return 0, nil
	}

	return len(p), nil
}
func (conn FakeConnection) Close() error {
	Warn("TODO: implement close FakeConnection")

	// TODO: call rust to send socks5 packet with close connection data

	return nil
}
func (conn FakeConnection) LocalAddr() net.Addr {
	Warn("TODO: implement LocalAddr FakeConnection")
	return nil
}
func (conn FakeConnection) RemoteAddr() net.Addr {
	Warn("TODO: implement RemoteAddr FakeConnection")
	return nil
}
func (conn FakeConnection) SetDeadline(t time.Time) error {
	Info("Setting deadline to %v\n", t)

	Warn("TODO: implement SetDeadline FakeConnection")
	return nil
}
func (conn FakeConnection) SetReadDeadline(t time.Time) error {
	Info("Setting read deadline to %v\n", t)

	Warn("TODO: implement SetReadDeadline FakeConnection")
	return nil
}
func (conn FakeConnection) SetWriteDeadline(t time.Time) error {
	Info("Setting write deadline to %v\n", t)

	Warn("TODO: implement SetWriteDeadline FakeConnection")
	return nil
}
