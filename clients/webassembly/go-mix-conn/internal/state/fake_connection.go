// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package state

import (
	"bytes"
	"crypto/tls"
	"go-mix-conn/internal/bridge/rust_bridge"
	"go-mix-conn/internal/log"
	"go-mix-conn/internal/sslhelpers"
	"go-mix-conn/internal/types"
	"io"
	"net"
	"time"
)

// InjectedData is, well, the injected server data that came from the mixnet into this connection
type InjectedData struct {
	ServerData   <-chan []byte
	RemoteClosed <-chan bool
	RemoteError  <-chan error
}

// ConnectionInjector controls data that goes over corresponding FakeConnection
type ConnectionInjector struct {
	ServerData   chan<- []byte
	RemoteClosed chan<- bool
	RemoteError  chan<- error
}

// FakeConnection is a type implementing net.Conn interface that allows us
// to inspect and control bytes that would normally go onto the wire
type FakeConnection struct {
	requestId     types.RequestId
	remoteAddress string
	data          *InjectedData

	pendingReads chan []byte
}

// NewFakeConnection creates a new FakeConnection that implements net.Conn interface alongside
// handlers for injecting data into it
func NewFakeConnection(requestId types.RequestId, remoteAddress string) (FakeConnection, ConnectionInjector) {
	serverData := make(chan []byte, 10)
	remoteClosed := make(chan bool, 1)
	remoteError := make(chan error, 1)

	inj := ConnectionInjector{
		ServerData:   serverData,
		RemoteClosed: remoteClosed,
		RemoteError:  remoteError,
	}

	conn := FakeConnection{
		data: &InjectedData{
			ServerData:   serverData,
			RemoteClosed: remoteClosed,
			RemoteError:  remoteError,
		},
		requestId:     requestId,
		remoteAddress: remoteAddress,
		pendingReads:  make(chan []byte, 1),
	}

	return conn, inj
}

// NewFakeTlsConn wraps a FakeConnection with all the TLS magic
// note: this returns a tls.Conn in the pre-handshake state
func NewFakeTlsConn(connectionId types.RequestId, remoteAddress string) (*tls.Conn, ConnectionInjector) {
	host, _, err := net.SplitHostPort(remoteAddress)
	if err != nil {
		panic("todo")
	}
	conn, inj := NewFakeConnection(connectionId, remoteAddress)
	tlsConfig := sslhelpers.TlsConfig(host)
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

	log.Debug("READING INJECTED %d bytes <<< \n", n)
	return n, err
}

// TODO: so many EOF edge cases here...
func (conn FakeConnection) Read(p []byte) (int, error) {
	// TODO: is there really no better way for priority chan reads?
	select {
	// see if we have any leftover data from the previous read
	case incomplete := <-conn.pendingReads:
		log.Debug("reading previously incomplete data")
		return conn.readAndBuffer(incomplete, p)
	default:
		// reason for this extra select:
		// if we have BOTH server data and closing information - you HAVE TO use up the data first
		select {
		// if we have any data: do read it
		case injectedData := <-conn.data.ServerData:
			log.Info("server data")
			return conn.readAndBuffer(injectedData, p)
		default:
			// we wait for either some data, closing info or an error
			select {
			case data := <-conn.data.ServerData:
				if len(data) == 0 {
					return 0, io.EOF
				}
				return conn.readAndBuffer(data, p)
			case <-conn.data.RemoteClosed:
				return 0, io.EOF
			case err := <-conn.data.RemoteError:
				return 0, err
			}
		}
	}
}

func (conn FakeConnection) Write(p []byte) (int, error) {
	log.Debug("WRITING %d bytes TO 'REMOTE' >>> \n", len(p))

	err := rust_bridge.RsSendClientData(conn.requestId, p)
	if err != nil {
		return 0, err
	} else {
		return len(p), nil
	}
}

func (conn FakeConnection) Close() error {
	log.Debug("closing FakeConnection")
	ActiveRequests.Remove(conn.requestId)

	// TODO: if we already received information about remote being closed,
	// do we have to send a socks5 closing packet?
	return rust_bridge.RsFinishMixnetConnection(conn.requestId)
}

func (conn FakeConnection) LocalAddr() net.Addr {
	log.Warn("TODO: implement LocalAddr FakeConnection")
	return nil
}

func (conn FakeConnection) RemoteAddr() net.Addr {
	log.Warn("TODO: implement RemoteAddr FakeConnection")
	return nil
}

func (conn FakeConnection) SetDeadline(t time.Time) error {
	log.Info("Setting deadline to %v\n", t)

	log.Warn("TODO: implement SetDeadline FakeConnection")
	return nil
}

func (conn FakeConnection) SetReadDeadline(t time.Time) error {
	log.Info("Setting read deadline to %v\n", t)

	log.Warn("TODO: implement SetReadDeadline FakeConnection")
	return nil
}

func (conn FakeConnection) SetWriteDeadline(t time.Time) error {
	log.Info("Setting write deadline to %v\n", t)

	log.Warn("TODO: implement SetWriteDeadline FakeConnection")
	return nil
}
