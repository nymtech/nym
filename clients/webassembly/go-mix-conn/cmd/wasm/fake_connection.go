// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"bytes"
	"crypto/tls"
	"io"
	"net"
	"sync/atomic"
	"time"
)

// InjectedData is, well, the injected server data that came from the mixnet into this connection
type InjectedData struct {
	serverData   <-chan []byte
	remoteClosed *atomic.Bool
	redirected   *atomic.Bool
}

// ConnectionInjector controls data that goes over corresponding FakeConnection
type ConnectionInjector struct {
	serverData   chan<- []byte
	remoteClosed *atomic.Bool
	redirected   *atomic.Bool
}

// FakeConnection is a type implementing net.Conn interface that allows us
// to inspect and control bytes that would normally go onto the wire
type FakeConnection struct {
	requestId     RequestId
	remoteAddress string
	data          *InjectedData

	pendingReads chan []byte
}

// NewFakeConnection creates a new FakeConnection that implements net.Conn interface alongside
// handlers for injecting data into it
func NewFakeConnection(requestId RequestId, remoteAddress string) (FakeConnection, ConnectionInjector) {
	serverData := make(chan []byte, 10)
	remoteClosed := &atomic.Bool{}
	redirected := &atomic.Bool{}

	inj := ConnectionInjector{
		serverData:   serverData,
		remoteClosed: remoteClosed,
		redirected:   redirected,
	}

	conn := FakeConnection{
		data: &InjectedData{
			serverData:   serverData,
			remoteClosed: remoteClosed,
			redirected:   redirected,
		},
		requestId:     requestId,
		remoteAddress: remoteAddress,
		pendingReads:  make(chan []byte, 1),
	}

	return conn, inj
}

// NewFakeTlsConn wraps a FakeConnection with all the TLS magic
// note: this returns a tls.Conn in the pre-handshake state
func NewFakeTlsConn(connectionId RequestId, remoteAddress string) (*tls.Conn, ConnectionInjector) {
	conn, inj := NewFakeConnection(connectionId, remoteAddress)
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
		conn.pendingReads <- leftover
	}

	Debug("READING INJECTED %d bytes <<< \n", n)
	return n, err
}

// TODO: so many EOF edge cases here...
func (conn FakeConnection) Read(p []byte) (int, error) {
	if conn.data.redirected.Load() {
		Error("attempted to read more data from the socket after we got redirected!")
		return 0, io.ErrClosedPipe
	}

	select {
	// see if we have any leftover data from the previous read
	case incomplete := <-conn.pendingReads:
		Debug("reading previously incomplete data")
		return conn.readAndBuffer(incomplete, p)
	default:
		select {
		// if we have any data: do read it
		case injectedData := <-conn.data.serverData:
			Info("server data")
			return conn.readAndBuffer(injectedData, p)
		default:
			// otherwise see if the socket is closed
			if conn.data.remoteClosed.Load() {
				return 0, io.EOF
			} else {
				Debug("waiting for data to read...")
				// wait for the data
				// TODO: what if we received information about closed socket here?
				data := <-conn.data.serverData
				if len(data) == 0 {
					return 0, io.EOF
				}
				return conn.readAndBuffer(data, p)
			}
		}
	}
}

func (conn FakeConnection) Write(p []byte) (int, error) {
	if conn.data.redirected.Load() {
		Error("attempted to write more data to the socket after we got redirected!")
		return 0, io.ErrClosedPipe
	}

	Debug("WRITING %d bytes TO 'REMOTE' >>> \n", len(p))

	err := rsSendClientData(conn.requestId, p)
	if err != nil {
		return 0, err
	} else {
		return len(p), nil
	}

	//requestId := strconv.FormatUint(conn.requestId, 10)
	//jsBytes := intoJsBytes(p)
	//
	//sendPromise := js.Global().Call("send_client_data", requestId, jsBytes)
	//_, err := await(sendPromise)
	//if err != nil {
	//	Error("failed to resolve sendPromise")
	//	return 0, nil
	//}
	//
	//return len(p), nil
}

func (conn FakeConnection) Close() error {
	Warn("TODO: implement close FakeConnection")
	activeRequests.remove(conn.requestId)

	if !conn.data.remoteClosed.Load() {
		// TODO: call rust to send socks5 packet with close connection data
		Error("unimplemented: send socks5 closing packet")
		// TODO: should we actually send socks5 packet here? what if we have redirections?
		// we should only send close packet when we have our response
	}

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
